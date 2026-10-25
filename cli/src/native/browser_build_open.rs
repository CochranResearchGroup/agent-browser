//! A selected build enters the existing managed-open pipeline. The durable
//! configuration receipt binds profile, executable bytes and independent mode.
use super::browser_build_selection::{executable_digest, verify_build};
use super::*;
use agent_browser_service_model::BrowserLaunchCustodyStore;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(crate) fn verify_build_open_selection(
        &self,
        command: &Value,
        require_loaded: bool,
    ) -> Result<(), String> {
        let Some(value) = command.get("buildSelectionOperationId") else {
            return Ok(());
        };
        let id = value
            .as_str()
            .filter(|id| !id.is_empty() && id.len() <= 128)
            .ok_or("browser_build_selection_id_invalid")?;
        let profile_id = required_string(command, "profileId")?;
        let database = self.persistence.runtime_database_path()?;
        let mut store = BrowserSessionSqliteStore::open_read_only(&database)?;
        let receipt = store
            .profile_build_selection_by_id(id, profile_id)?
            .ok_or("browser_build_selection_missing:inspect and confirm a current selection")?;
        let (state, catalog) = self.persistence.load_owner_snapshot()?;
        let profile = catalog
            .profiles
            .get(profile_id)
            .or_else(|| {
                state
                    .disposable_profiles
                    .get(profile_id)
                    .map(|row| &row.profile)
            })
            .ok_or("browser_build_profile_missing")?;
        if state.profile_operation_ids.contains_key(profile_id) {
            return Err(
                "browser_build_profile_busy:reconcile the current profile operation".into(),
            );
        }
        if receipt["profileDirectoryExisted"] == true && !Path::new(&profile.user_data_dir).is_dir()
        {
            return Err("browser_build_profile_directory_unavailable:restore the existing data or choose a new profile".into());
        }
        let path = state
            .profile_executables
            .get(profile_id)
            .ok_or("browser_build_selection_superseded:inspect current profile configuration")?;
        let expected = receipt["selectedBuild"]["sha256"]
            .as_str()
            .ok_or("browser_build_receipt_invalid")?;
        let build_id = format!(
            "build:{:x}",
            Sha256::digest(format!("{path}\0{expected}").as_bytes())
        );
        let mode = receipt["requestedMode"]
            .as_str()
            .ok_or("browser_build_receipt_invalid")?;
        if receipt["selectedBuild"]["id"] != build_id
            || profile.manual_login_url.is_some() != (mode == "cdp_free")
        {
            return Err("browser_build_selection_superseded:inspect and confirm the current profile configuration".into());
        }
        if command
            .get("manualLogin")
            .is_some_and(|manual| manual.as_bool() != Some(mode == "cdp_free"))
        {
            return Err(
                "browser_build_selection_mode_conflict:confirm a new independent mode selection"
                    .into(),
            );
        }
        verify_build(path, Some(expected))?;
        let mut live = false;
        for browser in state
            .browsers
            .values()
            .filter(|browser| browser.profile_id == profile_id)
        {
            let identity = if let Some(attached) = state.app_browser_attachments.get(&browser.id) {
                Some(attached.request.process.clone())
            } else {
                store
                    .published_launch_process_identity(browser)
                    .map_err(|_| "browser_build_process_identity_unavailable")?
            };
            let identity = identity
                .ok_or("browser_build_process_identity_unavailable:inspect the existing browser")?;
            match crate::process_identity::observe_process(identity.pid) {
                crate::process_identity::ProcessObservation::Missing if !require_loaded=>continue,
                crate::process_identity::ProcessObservation::Observed(observed) if observed.start_token.as_deref()==Some(&identity.start_token)&&observed.executable_path==identity.executable_path=>{},
                _=>return Err("browser_build_process_unverified:reconcile the existing browser without another launch".into())
            }
            if identity.executable_path.as_deref() != Some(path.as_str()) {
                return Err("browser_build_existing_browser_mismatch:use mode conversion for the existing task".into());
            }
            #[cfg(target_os = "linux")]
            if executable_digest(Path::new(&format!("/proc/{}/exe", identity.pid)))?.as_str()
                != expected
            {
                return Err("browser_build_loaded_bytes_mismatch:inspect the existing browser or use mode conversion".into());
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Err(
                    "browser_build_loaded_bytes_unverified:platform qualification required".into(),
                );
            }
            let proof = super::super::browser_session_store::mode_inspection::inspect(
                browser,
                Some(&identity),
                "managed",
                crate::process_identity::observe_process,
            );
            if proof["mode"] != mode {
                return Err(
                    "browser_build_loaded_mode_unverified:inspect the existing browser".into(),
                );
            }
            live = true;
        }
        if require_loaded && !live {
            return Err(
                "browser_build_launch_pending:inspect or resume the same opening request".into(),
            );
        }
        Ok(())
    }
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    /// Explicit adapter, with a stable task name and no website-effect replay.
    pub(super) fn open_selected_build(&mut self, command: &Value) -> Result<Value, String> {
        let operation_id = required_string(command, "operationId")?;
        let profile_id = required_string(command, "profileId")?;
        let mut open = command.clone();
        open["action"] = json!("browser_session_open");
        open["buildSelectionOperationId"] = json!(operation_id);
        open["sessionName"] = json!(format!(
            "build-{:x}",
            Sha256::digest(format!("{profile_id}\0{operation_id}").as_bytes())
        ));
        open["url"] = json!("about:blank");
        if let Some(mode) = command.get("mode") {
            open["manualLogin"] = match mode.as_str() {
                Some("cdp") => json!(false),
                Some("cdp_free") => json!(true),
                _ => return Err("browser_build_mode_invalid".into()),
            };
        }
        self.verify_build_open_selection(&open, false)?;
        let (state, catalog) = self.persistence.load_owner_snapshot()?;
        self.persisted_state = state.clone();
        self.state = state;
        self.catalog = catalog;
        let opened = self.open_request_from_command(&open, current_unix_ms())?;
        let tab_id = self
            .tab_for_navigation(&opened.session_id, current_unix_ms())?
            .tab_id;
        self.verify_build_open_selection(&open, true)?;
        let target = agent_browser_service_model::RemoteViewTabHandoffTarget {
            browser: self
                .state
                .browsers
                .get(&opened.browser_id)
                .cloned()
                .ok_or("browser_build_browser_missing")?,
            session: self
                .state
                .sessions
                .get(&opened.session_id)
                .cloned()
                .ok_or("browser_build_task_missing")?,
            tab: self
                .state
                .tabs
                .get(&tab_id)
                .cloned()
                .ok_or("browser_build_tab_missing")?,
        };
        let ready = self.native_browser_command(&target, &json!({"action":"native_ready"}));
        let native_ready = ready.as_ref().is_ok_and(|value| value["ready"] == true);
        let failure = ready.err().map(|error| {
            error
                .split(':')
                .next()
                .unwrap_or("browser_build_native_pending")
                .to_owned()
        });
        let receipt =
            BrowserSessionSqliteStore::open_read_only(&self.persistence.runtime_database_path()?)?
                .profile_build_selection_by_id(operation_id, profile_id)?
                .ok_or("browser_build_selection_missing")?;
        Ok(
            json!({"schemaVersion":"agent-browser.build-selection.v1","operationId":operation_id,"profileId":profile_id,
            "phase":if native_ready {"opened"}else{"open_pending"},"taskId":opened.session_id,"sessionName":opened.session_name,"browserId":opened.browser_id,
            "selectedBuild":receipt["selectedBuild"],"requestedMode":receipt["requestedMode"],"observedMode":receipt["requestedMode"],"loadedBytesVerified":true,
            "nativeReady":native_ready,"failure":failure,"authenticationContinuity":"unverified","profileDataReadable":"unverified",
            "actions":[{"action":"inspect","requiresConfirmation":false},{"action":"open_selected","requiresConfirmation":false}]}),
        )
    }
}
