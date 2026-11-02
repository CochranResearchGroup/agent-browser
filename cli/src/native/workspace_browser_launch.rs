//! Manual workspace opening reuses the durable tab-request owner. Unknown
//! outcomes are inspected by their original request; inspection has no effects.
use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    pub(super) fn workspace_browser_launch(&mut self, command: &Value) -> Result<Value, String> {
        let result = self.workspace_browser_launch_inner(command);
        if command["operation"] == "inspect" || result.is_ok() {
            return result;
        }
        let operation = required_string(command, "operationId")?;
        let operator = required_string(command, "operator")?;
        let subject = format!("{:x}", Sha256::digest(operator.as_bytes()));
        let key = format!("workspace:{}:{operation}", &subject[..16]);
        match self.persistence.load_session_state() {
            Ok(state) if !state.managed_tab_requests.contains_key(&key) => {
                result.map_err(|error| format!("workspace_launch_refused_no_effect:{error}"))
            }
            _ => result,
        }
    }

    fn workspace_browser_launch_inner(&mut self, command: &Value) -> Result<Value, String> {
        let operation = required_string(command, "operationId")?;
        uuid::Uuid::parse_str(operation).map_err(|_| "workspace_launch_id_invalid")?;
        let operator = required_string(command, "operator")?;
        if operator.len() > 256 {
            return Err("workspace_operator_invalid".into());
        }
        let subject = format!("{:x}", Sha256::digest(operator.as_bytes()));
        let key = format!("workspace:{}:{operation}", &subject[..16]);
        let session_name = format!("workspace-{}-{operation}", &subject[..16]);
        // Read the durable owner directly. Inspection never reaps, allocates,
        // launches or replays a failed page effect.
        let current = self.persistence.load_session_state()?;
        if command["operation"] == "inspect" {
            let mut next = current.clone();
            let result = inspect_allocation(&mut next, &key, operation)?;
            if next != current {
                self.persistence
                    .compare_and_save_session_state(&current, &next)?;
                self.state = next.clone();
                self.persisted_state = next;
            }
            return Ok(result);
        }
        let intent = command
            .get("intent")
            .ok_or("workspace_launch_intent_required")?;
        if intent["operationId"] != operation {
            return Err("workspace_launch_intent_id_changed".into());
        }
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(intent).map_err(|_| "workspace_launch_intent_invalid")?
            )
        );
        if let Some(row) = current.managed_tab_requests.get(&key) {
            if row.command_digest != fingerprint || row.session_name != session_name {
                return Err("workspace_launch_intent_changed".into());
            }
            return row.response.clone().ok_or_else(|| {
                "workspace_launch_pending_original: inspect the original request".into()
            });
        }
        let kind = required_string(intent, "profileMode")?;
        if !matches!(kind, "disposable" | "persistent" | "existing") {
            return Err("workspace_profile_kind_invalid".into());
        }
        let mode = required_string(intent, "mode")?;
        if !matches!(mode, "cdp" | "cdp_free") || intent["leaveOpen"] != true {
            return Err("workspace_launch_mode_invalid".into());
        }
        let url = required_string(intent, "url")?;
        let parsed = reqwest::Url::parse(url).map_err(|_| "workspace_launch_url_invalid")?;
        if !(matches!(parsed.scheme(), "http" | "https") || url == "about:blank")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("workspace_launch_url_invalid".into());
        }
        if !self.effects.admits_ordinary_sessions() {
            return Err("manual_login_requires_native_remote_desktop".into());
        }
        let selection = intent
            .get("buildSelection")
            .ok_or("workspace_build_required")?;
        let selected = browser_build_selection::requested_build(selection, &[])?
            .ok_or("workspace_build_required")?;
        if !matches!(selected.family.as_deref(), Some("chrome" | "chromium")) {
            return Err("workspace_browser_family_unsupported".into());
        }
        let profile_id = match kind {
            "existing" => Some(required_string(intent, "profileId")?.to_string()),
            "persistent" => {
                let name = required_string(intent, "profileName")?;
                if name.trim().is_empty() || name.len() > 120 {
                    return Err("workspace_profile_name_invalid".into());
                }
                Some(format!("manual-{operation}"))
            }
            _ => None,
        };
        let mut open = json!({"action":"browser_session_open","sessionName":session_name,"manualLogin":mode=="cdp_free","url":url,"leaveOpen":true,"originKind":"human","humanName":operator,"requestId":operation,"taskName":"Manual browser"});
        if let Some(id) = &profile_id {
            open["profileId"] = json!(id);
        }
        if kind == "existing" {
            if !self
                .catalog
                .profiles
                .contains_key(profile_id.as_deref().unwrap_or(""))
            {
                return Err("workspace_profile_unavailable".into());
            }
            for browser in self
                .state
                .browsers
                .values()
                .filter(|b| Some(&b.profile_id) == profile_id.as_ref())
            {
                if browser.cdp_endpoint.is_empty() || mode != "cdp" {
                    return Err("workspace_profile_in_use: return to its current task or choose another profile".into());
                }
            }
        }
        let existing_active = profile_id
            .as_ref()
            .is_some_and(|id| self.state.browsers.values().any(|b| b.profile_id == *id));
        if kind == "existing" {
            let id = profile_id
                .as_deref()
                .ok_or("workspace_profile_unavailable")?;
            let inspected =
                self.browser_build_select(&json!({"choice":"inspect","profileId":id}))?;
            if existing_active {
                if inspected["nextLaunchBuild"]["id"] != selected.id {
                    return Err("workspace_existing_build_conflict: return to the current task or choose another profile".into());
                }
                open["buildSelectionOperationId"] =
                    inspected["currentSelectionOperationId"].clone();
                if !open["buildSelectionOperationId"].is_string() {
                    return Err("workspace_existing_build_unverified".into());
                }
                self.verify_build_open_selection(&open, true)?;
            } else if inspected["nextLaunchBuild"]["id"] != selected.id
                && intent["confirmProfileBuildChange"] != true
            {
                return Err("workspace_build_confirmation_required".into());
            }
        }
        // Retain intent in the existing owner before directory, profile-build
        // configuration, launch or navigation effects. Uncertain work is read
        // back, never replayed as another task.
        super::super::browser_host_control::check_state(&self.state, "", &open)?;
        self.state.managed_tab_requests.insert(
            key.clone(),
            agent_browser_service_model::ManagedBrowserTabRequest {
                session_name: session_name.clone(),
                command_digest: fingerprint,
                profile_id: profile_id.clone(),
                response: None,
            },
        );
        self.commit_state()?;
        if kind == "persistent" {
            let id = profile_id
                .as_deref()
                .ok_or("workspace_profile_unavailable")?;
            let path = crate::runtime_profile::resolve_profile(None, Some(id))?.user_data_dir;
            std::fs::create_dir_all(&path)
                .map_err(|_| "workspace_profile_directory_unavailable")?;
            open["runtimeProfile"] = json!(id);
            open["profile"] = json!(path);
            self.register_explicit_profile(&open)?;
            let expected = self.catalog.clone();
            let mut next = expected.clone();
            next.profiles
                .get_mut(id)
                .ok_or("workspace_profile_unavailable")?
                .name = required_string(intent, "profileName")?.trim().to_string();
            self.persistence.publish_profile_catalog(&expected, &next)?;
            self.catalog = next;
            let profile = crate::native::service_config::register_runtime_profile(
                id,
                &path.to_string_lossy(),
            )?;
            let name = required_string(intent, "profileName")?.trim().to_string();
            crate::native::service_store::mutate_default_service_state(|state| {
                let row = state
                    .profiles
                    .get_mut(&profile.id)
                    .ok_or("workspace_profile_unavailable")?;
                row.name = name.clone();
                Ok(())
            })?;
        }
        if let Some(id) = &profile_id {
            if !existing_active {
                let mut build = selection.clone();
                build["choice"] = json!("select_for_next_launch");
                build["profileId"] = json!(id);
                build["operationId"] = json!(format!("workspace-{operation}"));
                build["mode"] = json!(mode);
                build["ownerConfirmed"] = json!(true);
                self.browser_build_select(&build)?;
                open["buildSelectionOperationId"] = json!(format!("workspace-{operation}"));
            }
        } else {
            open["workspaceDisposableBuild"] = selection.clone();
        }
        let opened = self.open_request_from_command(&open, current_unix_ms())?;
        self.tab_for_navigation(&opened.session_id, current_unix_ms())?;
        let tab_id = self
            .state
            .sessions
            .get(&opened.session_id)
            .and_then(|s| s.current_tab_id.clone())
            .ok_or("workspace_launch_tab_missing")?;
        let metadata_command = json!({"originKind":"human","humanName":operator,"requestId":operation,"taskName":"Manual browser"});
        if let Some(record) = self.state.tabs.get(&tab_id) {
            self.state.tab_metadata.insert(
                tab_id.clone(),
                capture_browser_task_metadata(&metadata_command, record),
            );
        }
        if mode == "cdp" && url != "about:blank" {
            let result=self.execute_managed_command(&session_name,&json!({"action":"navigate","url":url,"sessionId":opened.session_id,"profileId":opened.profile_id,"originKind":"human","humanName":operator}))?.ok_or("workspace_launch_navigation_missing")?;
            if result["success"] != true {
                return Err(
                    "workspace_launch_navigation_uncertain: inspect the original task".into(),
                );
            }
        }
        let result = json!({"schemaVersion":"agent-browser.workspace-browser-launch.v1","operationId":operation,"state":"ready","taskId":opened.session_id,"sessionName":session_name,"profileId":opened.profile_id,"browserId":opened.browser_id,"tabId":tab_id});
        self.state
            .managed_tab_requests
            .get_mut(&key)
            .ok_or("workspace_launch_request_missing")?
            .response = Some(result.clone());
        self.commit_state()?;
        Ok(result)
    }
}

/// Joins public policy-filtered profile rows to the existing session owner.
/// Intended account labels remain separate from live authentication evidence.
pub(crate) fn catalog(library: Value) -> Result<Value, String> {
    let store = BrowserSessionSqliteStore::default_sqlite()?;
    let state = store.load_session_state()?;
    let catalog = store.load_profile_catalog()?.catalog;
    let mut profiles = Vec::new();
    for row in library["profiles"].as_array().into_iter().flatten() {
        let Some(id) = row["profile"]["id"].as_str() else {
            continue;
        };
        let Some(profile) = catalog.profiles.get(id) else {
            continue;
        };
        let tasks = state
            .sessions
            .values()
            .filter(|task| task.profile_id == id)
            .collect::<Vec<_>>();
        let browser = tasks
            .first()
            .and_then(|task| state.browsers.get(&task.browser_id));
        let mut labels = Vec::new();
        for field in ["accountLabels", "aliases", "origins", "loginIds"] {
            for label in row["profile"][field]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                labels.push(label.to_string());
            }
        }
        for association in row["profile"]["siteAssociations"]
            .as_array()
            .into_iter()
            .flatten()
        {
            for field in ["sitePolicyId", "account", "organization"] {
                if let Some(value) = association[field].as_str() {
                    labels.push(value.to_string());
                }
            }
        }
        labels.sort();
        labels.dedup();
        let mut item = json!({"id":id,"name":profile.name,"activeTaskCount":tasks.len(),"canShareTabs":browser.is_some_and(|b|!b.cdp_endpoint.is_empty()),"accountLabels":labels});
        if let Some(browser) = browser {
            item["mode"] = json!(if browser.cdp_endpoint.is_empty() {
                "cdp_free"
            } else {
                "cdp"
            });
            item["browserId"] = json!(browser.id);
        }
        if let Some(task) = tasks.first() {
            item["taskId"] = json!(task.id);
            item["sessionName"] = json!(task.name);
            item["tabId"] = json!(task.current_tab_id);
        }
        // The executable pin is next-launch configuration, not observed login
        // or proof of the bytes loaded by a currently running browser.
        if let Some(path) = state.profile_executables.get(id) {
            let build = browser_build_selection::inspect_executable(Path::new(path));
            item["buildId"] = json!(build.id);
            item["family"] = json!(build.family);
            item["version"] = json!(build.version);
        }
        profiles.push(item);
    }
    Ok(
        json!({"schemaVersion":"agent-browser.workspace-browser-catalog.v1","builds":browser_build_selection::workspace_builds()?,"profiles":profiles}),
    )
}

/// Inspect the original launcher allocation without constructing a runtime host.
pub(crate) fn inspect(operator: &str, operation: &str) -> Result<Value, String> {
    uuid::Uuid::parse_str(operation).map_err(|_| "workspace_launch_id_invalid")?;
    let subject = format!("{:x}", Sha256::digest(operator.as_bytes()));
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let current = store.load_session_state()?;
    let mut next = current.clone();
    let key = format!("workspace:{}:{operation}", &subject[..16]);
    let result = inspect_allocation(&mut next, &key, operation)?;
    if next != current {
        store.compare_and_save_session_state(&current, &next)?;
    }
    Ok(result)
}

/// Reconcile published allocation identity only. A pending page action remains
/// unconfirmed and is never replayed, nor promoted to Native visibility proof.
fn inspect_allocation(
    state: &mut BrowserSessionState,
    key: &str,
    operation: &str,
) -> Result<Value, String> {
    let row = state.managed_tab_requests.get(key).ok_or(
        "workspace_launch_unknown: inspect the original request; no creation replay is permitted",
    )?;
    if let Some(response) = &row.response {
        return Ok(response.clone());
    }
    let tasks = state
        .sessions
        .values()
        .filter(|task| task.name == row.session_name)
        .collect::<Vec<_>>();
    let [task] = tasks.as_slice() else {
        return Err("workspace_launch_unknown: no unique original allocation is published".into());
    };
    let browser = state
        .browsers
        .get(&task.browser_id)
        .ok_or("workspace_launch_unknown")?;
    let tab = task
        .current_tab_id
        .as_ref()
        .and_then(|id| state.tabs.get(id))
        .ok_or("workspace_launch_unknown")?;
    if row
        .profile_id
        .as_ref()
        .is_some_and(|id| id != &task.profile_id)
        || browser.profile_id != task.profile_id
        || tab.session_id != task.id
        || tab.browser_id != browser.id
    {
        return Err("workspace_launch_original_identity_conflict".into());
    }
    let response = json!({"schemaVersion":"agent-browser.workspace-browser-launch.v1","operationId":operation,"state":"ready","taskId":task.id,"sessionName":task.name,"profileId":task.profile_id,"browserId":browser.id,"tabId":tab.id,"initialNavigation":"unconfirmed","warning":"The original browser allocation was found. Starting navigation is unconfirmed; continue in the existing task."});
    state
        .managed_tab_requests
        .get_mut(key)
        .ok_or("workspace_launch_unknown")?
        .response = Some(response.clone());
    Ok(response)
}
