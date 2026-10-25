//! Durable owner-directed mode conversion. Native Remote View owns all input.
use super::super::browser_mode_operation::{
    Choice, CloseOutcome, Mode, ModeRuntime, Operation, OperationStore, OriginalStatus, Phase,
    Replacement, Request,
};
use super::browser_build_selection::{
    executable_digest, inspect_executable, profile_data_version, verify_build, ExecutableEvidence,
};
use super::*;
use agent_browser_service_model::{
    BrowserLaunchCustodyStore, BrowserProfileCatalogEntry, BrowserRecovery, ManagedBrowserInstance,
    ManagedBrowserSession, ManagedBrowserTab, RecordedProcessIdentity, RemoteViewTabHandoffTarget,
};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Serialize, Deserialize)]
struct Binding {
    profile: BrowserProfileCatalogEntry,
    browser: ManagedBrowserInstance,
    session: ManagedBrowserSession,
    tab: ManagedBrowserTab,
    original: RecordedProcessIdentity,
    original_mode: Mode,
    #[serde(default)]
    selected_build: Option<ExecutableEvidence>,
    #[serde(default)]
    previous_build: Option<ExecutableEvidence>,
    #[serde(default)]
    profile_data_version: Option<String>,
}
impl Binding {
    fn target(&self) -> RemoteViewTabHandoffTarget {
        RemoteViewTabHandoffTarget {
            browser: self.browser.clone(),
            session: self.session.clone(),
            tab: self.tab.clone(),
        }
    }
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    pub(crate) fn mode_convert(&mut self, command: &Value) -> Result<Value, String> {
        let id = required_string(command, "operationId")?;
        if id.len() > 128 {
            return Err("mode_operation_id_invalid".into());
        }
        let task_id = required_string(command, "sessionId")?;
        let session = self
            .state
            .sessions
            .get(task_id)
            .cloned()
            .ok_or("mode_operation_task_missing")?;
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or("mode_operation_browser_missing")?;
        let path = self
            .persistence
            .runtime_database_path()?
            .with_extension("mode.sqlite3");
        let connection =
            rusqlite::Connection::open(&path).map_err(|_| "mode_operation_store_unavailable")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(|_| "mode_operation_permissions_failed")?;
        }
        connection.execute_batch("CREATE TABLE IF NOT EXISTS browser_mode_owner_bindings(id TEXT PRIMARY KEY,binding TEXT NOT NULL)").map_err(|_|"mode_operation_store_unavailable")?;
        let retained: Option<String> = connection
            .query_row(
                "SELECT binding FROM browser_mode_owner_bindings WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "mode_operation_binding_unavailable")?;
        let binding: Binding = if let Some(value) = retained {
            serde_json::from_str(&value).map_err(|_| "mode_operation_binding_invalid")?
        } else {
            let mut store = BrowserSessionSqliteStore::open_read_only(
                &self.persistence.runtime_database_path()?,
            )?;
            let original =
                if let Some(attached) = self.state.app_browser_attachments.get(&browser.id) {
                    attached.request.process.clone()
                } else {
                    store
                        .published_launch_process_identity(&browser)
                        .map_err(|_| "mode_operation_original_identity_unavailable")?
                        .ok_or("mode_operation_original_identity_missing:refresh task")?
                };
            if !original_matches(&original)? {
                return Err(
                    "mode_operation_target_changed:refresh task before choosing conversion".into(),
                );
            }
            let tab = session
                .current_tab_id
                .as_ref()
                .and_then(|id| self.state.tabs.get(id))
                .cloned()
                .ok_or("mode_operation_tab_missing")?;
            let profile = self
                .catalog
                .profiles
                .get(&session.profile_id)
                .or_else(|| {
                    self.state
                        .disposable_profiles
                        .get(&session.profile_id)
                        .map(|row| &row.profile)
                })
                .cloned()
                .ok_or("mode_operation_profile_missing")?;
            // Recovery restores the verified original process posture, not the
            // profile's next-launch intent (which can differ for app browsers).
            let observed = super::super::browser_session_store::mode_inspection::inspect(
                &browser,
                Some(&original),
                if self.state.app_browser_attachments.contains_key(&browser.id) {
                    "app"
                } else {
                    "managed"
                },
                crate::process_identity::observe_process,
            );
            let original_mode = match observed["mode"].as_str() {
                Some("cdp") => Mode::Cdp,
                Some("cdp_free") => Mode::CdpFree,
                _ => return Err("mode_operation_original_posture_unverified:refresh the current browser before conversion".into()),
            };
            let selected_build = self.requested_profile_build(command, &profile.id)?;
            let mut previous_build = selected_build
                .as_ref()
                .and(original.executable_path.as_ref())
                .map(|path| inspect_executable(Path::new(path)));
            #[cfg(target_os = "linux")]
            if let Some(previous) = previous_build.as_mut() {
                if executable_digest(Path::new(&format!("/proc/{}/exe", original.pid))).ok()
                    != previous.sha256
                {
                    previous.available = false;
                    previous.sha256 = None;
                }
            }
            let data_version = profile_data_version(&profile);
            let binding = Binding {
                profile,
                browser: browser.clone(),
                session: session.clone(),
                tab,
                original,
                original_mode,
                selected_build,
                previous_build,
                profile_data_version: data_version,
            };
            connection
                .execute(
                    "INSERT INTO browser_mode_owner_bindings VALUES(?1,?2)",
                    rusqlite::params![
                        id,
                        serde_json::to_string(&binding)
                            .map_err(|_| "mode_operation_binding_encode_failed")?
                    ],
                )
                .map_err(|_| "mode_operation_binding_conflict")?;
            binding
        };
        if session.browser_id != binding.browser.id || session.profile_id != binding.profile.id {
            return Err("mode_operation_task_binding_changed".into());
        }
        if optional_string(command, "choice") != Some("select_build") {
            if let Some(path) = optional_string(command, "executablePath") {
                let canonical = Path::new(path).canonicalize().ok();
                if binding.selected_build.as_ref().is_none_or(|retained| {
                    canonical
                        .as_ref()
                        .is_some_and(|path| path != Path::new(&retained.path))
                        || optional_string(command, "expectedSha256") != retained.sha256.as_deref()
                }) {
                    return Err(
                        "mode_operation_id_conflict:continue the retained build selection".into(),
                    );
                }
            }
        }
        if optional_string(command, "choice") != Some("select_build")
            && optional_string(command, "buildId").is_some_and(|id| {
                binding
                    .selected_build
                    .as_ref()
                    .is_none_or(|build| build.id != id)
            })
        {
            return Err("mode_operation_id_conflict:continue the retained build selection".into());
        }
        let requested_mode = match optional_string(command, "mode") {
            Some("cdp") => Mode::Cdp,
            Some("cdp_free") => Mode::CdpFree,
            _ => return Err("mode_operation_mode_required:cdp or cdp_free".into()),
        };
        let mut operations = OperationStore::open(&path)?;
        let mut state = if let Some(existing) = operations.read(id)? {
            if existing.request.browser_id != browser.id
                || existing.request.requested_mode != requested_mode
            {
                return Err("mode_operation_id_conflict".into());
            }
            existing
        } else {
            let tasks = self
                .state
                .sessions
                .values()
                .filter(|task| task.browser_id == browser.id)
                .map(|task| task.id.clone())
                .collect();
            operations.request(Request {
                operation_id: id.into(),
                browser_id: browser.id.clone(),
                profile_id: binding.profile.id.clone(),
                original_process: serde_json::to_string(&binding.original)
                    .map_err(|_| "mode_operation_original_encode_failed")?,
                executable: binding
                    .selected_build
                    .as_ref()
                    .map(|build| build.path.clone())
                    .or_else(|| binding.original.executable_path.clone())
                    .ok_or("mode_operation_executable_unverified")?,
                executable_sha256: binding
                    .selected_build
                    .as_ref()
                    .and_then(|build| build.sha256.clone()),
                task_ids: tasks,
                requested_mode,
            })?
        };
        let choice = optional_string(command, "choice").unwrap_or("prepare");
        let mut failure: Option<String> = None;
        if !matches!(state.phase, Phase::Completed | Phase::Cancelled) {
            if self
                .state
                .profile_operation_ids
                .get(&browser.profile_id)
                .is_some_and(|owner| owner != id)
            {
                return Err(
                    "mode_operation_profile_busy:inspect or cancel current owner operation".into(),
                );
            }
            if !self
                .state
                .profile_operation_ids
                .contains_key(&browser.profile_id)
            {
                self.state
                    .profile_operation_ids
                    .insert(browser.profile_id.clone(), id.into());
                self.commit_state()?;
            }
            if choice != "prepare" {
                if self
                    .state
                    .sessions
                    .values()
                    .filter(|task| task.browser_id == browser.id)
                    .map(|task| task.id.clone())
                    .collect::<Vec<_>>()
                    != state.request.task_ids
                {
                    return Err(
                        "mode_operation_affected_tasks_changed:cancel and review a fresh operation"
                            .into(),
                    );
                }
                let confirmed = command["ownerConfirmed"] == true;
                if (binding.selected_build.is_some() || state.effective_executable_sha256.is_some())
                    && matches!(choice, "relaunch" | "retry_graceful_close" | "force_close")
                    && !confirmed
                {
                    return Err("browser_build_owner_confirmation_required:profile migration and login loss are possible; duplicate-and-test, choose another build or cancel".into());
                }
                if !confirmed {
                    super::super::browser_host_control::check_state(
                        &self.state,
                        &session.name,
                        command,
                    )?;
                }
                let selected = match choice {
                    "select_build" => {
                        let build = self
                            .requested_profile_build(command, &binding.profile.id)?
                            .ok_or("browser_build_selection_required")?;
                        Choice::SelectExecutable {
                            executable: build.path,
                            sha256: build.sha256.ok_or("browser_build_unavailable")?,
                            confirmed,
                        }
                    }
                    "relaunch" => Choice::Relaunch { confirmed },
                    "pause_and_relaunch" => Choice::Pause,
                    "reconcile" => Choice::Resume,
                    "restore_previous" => Choice::RestorePrevious { confirmed },
                    "retry_graceful_close" => Choice::RetryGracefulClose,
                    "force_close" => Choice::ForceClose { confirmed },
                    "focus" => Choice::ReturnToBrowser,
                    "cancel" => Choice::Cancel,
                    _ => return Err("mode_operation_choice_invalid".into()),
                };
                let mut owner = Owner {
                    host: self,
                    binding: &binding,
                    recovery: None,
                };
                match operations.advance(id, selected, &mut owner) {
                    Ok(next) => state = next,
                    Err(error) => {
                        state = operations
                            .read(id)?
                            .ok_or("mode_operation_readback_required")?;
                        failure = Some(
                            error
                                .split(':')
                                .next()
                                .unwrap_or("mode_operation_unknown")
                                .to_string(),
                        );
                    }
                }
            }
        }
        if matches!(state.phase, Phase::Completed | Phase::Cancelled)
            && self
                .state
                .profile_operation_ids
                .get(&browser.profile_id)
                .map(String::as_str)
                == Some(id)
        {
            self.state.profile_operation_ids.remove(&browser.profile_id);
            self.commit_state()?;
        }
        let mut view = public_operation_view(&state, task_id, failure.as_deref());
        decorate_build_view(&mut view, &binding);
        Ok(view)
    }
}

fn decorate_build_view(view: &mut Value, binding: &Binding) {
    if let Some(build) = &binding.selected_build {
        view["compatibility"] = super::browser_build_selection::compatibility(
            binding.profile_data_version.as_deref(),
            binding.original.browser_family.as_deref(),
            build,
        );
        view["requestedBuild"] = build.public();
        view["selectedBuild"] = if view["effectiveBuild"].is_object() {
            view["effectiveBuild"].clone()
        } else {
            build.public()
        };
        view["previousBuild"] = binding
            .previous_build
            .as_ref()
            .map(ExecutableEvidence::public)
            .unwrap_or(Value::Null);
        view["profileDataVersion"] = json!(binding.profile_data_version);
        view["compatibilityEvidenceSource"] = json!("prepare_snapshot");
        view["credentialStorage"] = json!("unverified");
        view["authenticationContinuity"] = json!("unverified");
        view["profileDataReadable"] = json!("unverified");
        view["migrationWarning"]=json!("The selected build may migrate profile data or lose login. Selecting the previous executable does not restore profile bytes. Verify login independently; do not change credential encryption.");
        if let Some(actions) = view["actions"].as_array_mut() {
            for action in actions {
                if action["action"] == "relaunch" {
                    action["requiresConfirmation"] = json!(true);
                }
            }
        }
    }
}

pub(super) fn decorate_retained_build(
    connection: &rusqlite::Connection,
    state: &Operation,
    view: &mut Value,
) -> Result<(), String> {
    let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='browser_mode_owner_bindings')",[],|row|row.get(0)).map_err(|_|"browser_build_readback_unavailable")?;
    if !exists {
        return Ok(());
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT binding FROM browser_mode_owner_bindings WHERE id=?1",
            [&state.request.operation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "browser_build_readback_unavailable")?;
    if let Some(raw) = raw {
        let binding: Binding =
            serde_json::from_str(&raw).map_err(|_| "browser_build_readback_invalid")?;
        if binding.profile.id != state.request.profile_id
            || binding.browser.id != state.request.browser_id
        {
            return Err("browser_build_readback_binding_changed".into());
        }
        decorate_build_view(view, &binding);
    }
    Ok(())
}

/// Redacted owner readback shared by explicit actions and read-only lookup.
pub(super) fn public_operation_view(
    state: &Operation,
    task_id: &str,
    failure: Option<&str>,
) -> Value {
    let phase = phase_name(&state.phase);
    let actions = match state.phase {
        Phase::Prepared => vec![
            (
                "relaunch",
                state.request.task_ids.len() > 1
                    || state.request.executable_sha256.is_some()
                    || state.effective_executable_sha256.is_some(),
            ),
            ("pause_and_relaunch", false),
            ("select_build", true),
            ("cancel", false),
        ],
        Phase::Paused => vec![
            (
                "relaunch",
                state.request.task_ids.len() > 1
                    || state.request.executable_sha256.is_some()
                    || state.effective_executable_sha256.is_some(),
            ),
            ("select_build", true),
            ("cancel", false),
        ],
        Phase::Closing => vec![
            ("focus", false),
            ("retry_graceful_close", false),
            ("force_close", true),
            ("reconcile", false),
        ],
        Phase::ClosePrompt => vec![
            ("cancel", false),
            ("focus", false),
            ("retry_graceful_close", false),
            ("force_close", true),
            ("reconcile", false),
        ],
        Phase::OriginalExited => vec![
            ("reconcile", false),
            ("select_build", true),
            ("restore_previous", true),
        ],
        Phase::LaunchDispatched | Phase::ReplacementPending => {
            vec![("reconcile", false), ("restore_previous", true)]
        }
        Phase::LaunchAbsent => vec![
            ("relaunch", true),
            ("select_build", true),
            ("restore_previous", true),
            ("reconcile", false),
        ],
        _ => vec![],
    };
    json!({"schemaVersion":"agent-browser.mode-operation.v1","operationId":state.request.operation_id,"phase":phase,"failure":failure.or(state.failure.as_deref()),"launchAttempts":state.launch_attempts,"effectiveBuild":state.effective_executable.as_ref().map(|path|json!({"name":Path::new(path).file_name().map(|name|name.to_string_lossy()),"sha256":state.effective_executable_sha256,"version":null,"versionEvidence":"unverified"})),"recoveryChoice":if state.restoring_previous {Some("restore_previous")}else{None},"effectiveMode":state.effective_mode.as_ref().map(|mode|if *mode==Mode::Cdp{"cdp"}else{"cdp_free"}),"browserId":state.request.browser_id,"profileId":state.request.profile_id,"taskId":task_id,"affectedTaskIds":state.request.task_ids,"requestedMode":if state.request.requested_mode==Mode::Cdp{"cdp"}else{"cdp_free"},"observedMode":if state.phase==Phase::Completed {Some(if state.effective_mode.as_ref().unwrap_or(&state.request.requested_mode)==&Mode::Cdp{"cdp"}else{"cdp_free"})}else{None},"nativeContextScope":"browser","restoration":"logical tasks and profile retained; transient pages are not replayed; previous executable selection does not restore migrated profile bytes","warning":"Relaunch replaces the whole browser. Unsaved forms, downloads and page state can be lost. Native controls address the current browser window; URLs are unverified until observed. Force close may lose buffered data.","closeMethod":state.close_method,"actions":actions.into_iter().map(|(action,confirmation)|json!({"action":action,"requiresConfirmation":confirmation})).collect::<Vec<_>>()})
}
fn phase_name(phase: &Phase) -> &'static str {
    match phase {
        Phase::Prepared => "prepared",
        Phase::Paused => "paused",
        Phase::Closing => "closing",
        Phase::ClosePrompt => "close_prompt",
        Phase::OriginalExited => "original_exited",
        Phase::LaunchDispatched => "launch_dispatched",
        Phase::LaunchAbsent => "launch_absent",
        Phase::ReplacementPending => "replacement_pending",
        Phase::Completed => "completed",
        Phase::Cancelled => "cancelled",
    }
}
fn original_matches(original: &RecordedProcessIdentity) -> Result<bool, String> {
    match crate::process_identity::observe_process(original.pid) {
        crate::process_identity::ProcessObservation::Missing => Ok(false),
        crate::process_identity::ProcessObservation::Observed(current) => Ok(current
            .start_token
            .as_deref()
            == Some(&original.start_token)
            && current.executable_path == original.executable_path),
        _ => Err("mode_operation_original_readback_required".into()),
    }
}
struct Owner<'a, P, E> {
    host: &'a mut BrowserSessionHost<P, E>,
    binding: &'a Binding,
    recovery: Option<BrowserRecovery>,
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    Owner<'_, P, E>
{
    fn profile(&self, request: &Request) -> BrowserProfileCatalogEntry {
        let mut profile = self.binding.profile.clone();
        profile.manual_login_url = if request.requested_mode == Mode::CdpFree {
            Some("about:blank".into())
        } else {
            None
        };
        profile
    }
    fn qualify(
        &mut self,
        request: &Request,
        recovery: &BrowserRecovery,
    ) -> Result<Replacement, String> {
        let original = recovery
            .launch
            .process_identity
            .as_ref()
            .ok_or("mode_operation_replacement_identity_missing")?;
        if let Some(expected) = request.executable_sha256.as_deref() {
            if original.executable_path.as_deref() != Some(request.executable.as_str()) {
                return Err("browser_build_loaded_path_mismatch:inspect the launched process or restore previous executable".into());
            }
            #[cfg(target_os = "linux")]
            if executable_digest(Path::new(&format!("/proc/{}/exe", original.pid)))?.as_str()
                != expected
            {
                return Err(
                    "browser_build_loaded_bytes_mismatch:reconcile the existing launch".into(),
                );
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = expected;
                return Err(
                    "browser_build_loaded_bytes_unverified:platform qualification required".into(),
                );
            }
        }
        let mut browser = self.binding.browser.clone();
        browser.pid = recovery.launch.pid;
        browser.cdp_endpoint = recovery.launch.cdp_endpoint.clone();
        browser.desktop = recovery.launch.desktop.clone();
        let proof = super::super::browser_session_store::mode_inspection::inspect(
            &browser,
            Some(original),
            "managed",
            crate::process_identity::observe_process,
        );
        if proof["mode"]
            != if request.requested_mode == Mode::Cdp {
                "cdp"
            } else {
                "cdp_free"
            }
        {
            return Err("mode_operation_replacement_posture_unverified".into());
        }
        // Keep the observed launch intent until atomic replacement publication.
        let target = self
            .host
            .effects
            .native_handoff_assignment_target(&self.binding.target())?;
        let mut ready = None;
        for _ in 0..30 {
            match self.host.native_process_command(
                original,
                &target,
                &json!({"action":"native_ready"}),
            ) {
                Ok(observed) => {
                    ready = Some(observed);
                    break;
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        let ready =
            ready.ok_or("mode_operation_native_window_pending:reconcile without relaunch")?;
        if ready["window"]["active"] != true {
            self.host.native_process_command(
                original,
                &target,
                &json!({"action":"native_focus"}),
            )?;
        }
        let x = ready["window"]["x"]
            .as_i64()
            .ok_or("mode_operation_native_geometry_unverified")?
            + 10;
        let y = ready["window"]["y"]
            .as_i64()
            .ok_or("mode_operation_native_geometry_unverified")?
            + 10;
        // Validate native delivery on the new browser's empty chrome, not a
        // website action. Selective-control unavailability is never called ready.
        let delivery = self.host.native_process_command(
            original,
            &target,
            &json!({"action":"native_input","actions":[{"kind":"move","x":x,"y":y}]}),
        )?;
        if delivery["delivery"] != "dispatched" {
            return Err(
                "mode_operation_native_input_unavailable:reconcile or restore configuration".into(),
            );
        }

        Ok(Replacement {
            process_identity: serde_json::to_string(original)
                .map_err(|_| "mode_operation_replacement_encode_failed")?,
            mode: request.requested_mode.clone(),
            debugging: request.requested_mode == Mode::Cdp,
            native_observation: true,
            native_input: true,
        })
    }
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    ModeRuntime for Owner<'_, P, E>
{
    fn validate_executable(&mut self, request: &Request) -> Result<(), String> {
        verify_build(&request.executable, request.executable_sha256.as_deref())
    }
    fn original_status(&mut self, _: &Request) -> Result<OriginalStatus, String> {
        let _ = self.host.effects.browser_is_live(&self.binding.browser);
        Ok(if original_matches(&self.binding.original)? {
            OriginalStatus::Live
        } else {
            OriginalStatus::Exited
        })
    }
    fn graceful_close(&mut self, request: &Request) -> Result<CloseOutcome, String> {
        let reply = self
            .host
            .native_browser_command(&self.binding.target(), &json!({"action":"native_close"}))?;
        if reply["delivery"] != "dispatched" {
            return Ok(CloseOutcome::Uncertain);
        }
        for _ in 0..30 {
            if matches!(self.original_status(request)?, OriginalStatus::Exited) {
                return Ok(CloseOutcome::Exited);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(CloseOutcome::Prompt)
    }
    fn cancel_graceful_close(&mut self, request: &Request) -> Result<bool, String> {
        if !matches!(self.original_status(request)?, OriginalStatus::Live) {
            return Ok(false);
        }
        let reply = self.host.native_browser_command(
            &self.binding.target(),
            &json!({"action":"native_input","actions":[{"kind":"key","chord":"Escape"}]}),
        )?;
        if reply["delivery"] != "dispatched" {
            return Err("mode_operation_cancel_unproven:inspect original browser".into());
        }
        // Read current exact-process Native window evidence after Escape. A lost
        // input response or exited original never releases the profile fence.
        self.host
            .native_browser_command(&self.binding.target(), &json!({"action":"native_ready"}))?;
        Ok(matches!(
            self.original_status(request)?,
            OriginalStatus::Live
        ))
    }
    fn force_close(&mut self, request: &Request) -> Result<CloseOutcome, String> {
        #[cfg(target_os = "linux")]
        {
            if !original_matches(&self.binding.original)? {
                return Ok(CloseOutcome::Exited);
            }
            use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
            let raw =
                unsafe { libc::syscall(libc::SYS_pidfd_open, self.binding.original.pid, 0) } as i32;
            if raw < 0 {
                return Err(
                    "mode_operation_force_unavailable:check OS permission and return to browser"
                        .into(),
                );
            }
            let fd = unsafe { OwnedFd::from_raw_fd(raw) };
            let result = if original_matches(&self.binding.original)? {
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        fd.as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0,
                    )
                }
            } else {
                0
            };
            if result < 0 {
                return Err("mode_operation_force_failed:check OS permission".into());
            }
            for _ in 0..30 {
                if matches!(self.original_status(request)?, OriginalStatus::Exited) {
                    return Ok(CloseOutcome::Exited);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Ok(CloseOutcome::Uncertain)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            Err("mode_operation_force_platform_unavailable:return to browser".into())
        }
    }
    fn focus(&mut self, _: &Request) -> Result<(), String> {
        self.host
            .native_browser_command(&self.binding.target(), &json!({"action":"native_focus"}))
            .map(|_| ())
    }
    fn launch(&mut self, request: &Request) -> Result<Replacement, String> {
        verify_build(&request.executable, request.executable_sha256.as_deref())?;
        let profile = self.profile(request);
        if let Some(current) = self.host.catalog.profiles.get(&profile.id) {
            let expected = self.host.catalog.clone();
            let mut next = expected.clone();
            let _ = current;
            next.profiles.insert(profile.id.clone(), profile.clone());
            self.host
                .persistence
                .publish_profile_catalog(&expected, &next)?;
            self.host.catalog = next;
        } else if let Some(row) = self.host.state.disposable_profiles.get_mut(&profile.id) {
            row.profile = profile.clone();
        }
        self.host
            .state
            .profile_executables
            .insert(profile.id.clone(), request.executable.clone());
        self.host
            .state
            .app_browser_attachments
            .remove(&self.binding.browser.id);
        self.host.commit_state()?;
        self.host
            .effects
            .begin_operation(&self.host.persisted_state)?;
        self.host
            .effects
            .select_profile_browser_executable(profile.id.clone(), request.executable.clone())?;
        let tabs = self
            .host
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == self.binding.browser.id)
            .cloned()
            .collect::<Vec<_>>();
        let recovery =
            self.host
                .effects
                .recover_browser(&self.binding.browser, &profile, &tabs, &[])?;
        let proof = self.qualify(request, &recovery).map_err(|error| {
            if error.starts_with("mode_operation_replacement_") {
                error
            } else {
                format!("mode_operation_native_unavailable:{error}")
            }
        })?;
        self.recovery = Some(recovery);
        Ok(proof)
    }
    fn reconcile_replacement(&mut self, request: &Request) -> Result<Option<Replacement>, String> {
        let profile = self.profile(request);
        self.host
            .effects
            .begin_operation(&self.host.persisted_state)?;
        let tabs = self
            .host
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == self.binding.browser.id)
            .cloned()
            .collect::<Vec<_>>();
        let Some(recovery) = self.host.effects.reconcile_profile_browser(
            &profile,
            Some(&self.binding.browser),
            &tabs,
            &[],
        )?
        else {
            return Ok(None);
        };
        let proof = self.qualify(request, &recovery).map_err(|error| {
            if error.starts_with("mode_operation_replacement_") {
                error
            } else {
                format!("mode_operation_native_unavailable:{error}")
            }
        })?;
        self.recovery = Some(recovery);
        Ok(Some(proof))
    }
    fn replacement_absent(&mut self, request: &Request) -> Result<bool, String> {
        let profile = self.profile(request);
        super::super::browser_session_runtime::prove_profile_launch_absence(&profile)?;
        let path = self.host.persistence.runtime_database_path()?;
        BrowserSessionSqliteStore::open(path.parent().ok_or("mode_operation_store_unavailable")?)?
            .settle_unpublished_profile_launch_absence(
                &profile,
                &self.host.persisted_state,
                Some(&self.binding.browser),
            )
            .map_err(|_| "mode_operation_launch_custody_readback_required")?;
        // The exact absent launch is durably settled before dropping its local
        // observation. A subsequent state commit must not publish that dead
        // replacement as if it were the selected recovery outcome.
        self.host
            .effects
            .begin_operation(&self.host.persisted_state)?;
        Ok(true)
    }
    fn previous_executable(&self, _: &Request) -> Option<String> {
        self.binding.original.executable_path.clone()
    }
    fn previous_executable_sha256(&self, _: &Request) -> Option<String> {
        self.binding
            .previous_build
            .as_ref()
            .and_then(|build| build.sha256.clone())
    }
    fn restore_previous(&mut self, request: &Request) -> Result<Mode, String> {
        if self.binding.selected_build.is_some() {
            let previous=self.binding.previous_build.as_ref().filter(|build|build.available).ok_or("browser_build_previous_unavailable:choose another build; profile bytes have not been restored")?;
            verify_build(&previous.path, previous.sha256.as_deref())?;
        }
        let profile = self.profile(request);
        self.host
            .effects
            .begin_operation(&self.host.persisted_state)?;
        let tabs = self
            .host
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == self.binding.browser.id)
            .cloned()
            .collect::<Vec<_>>();
        // Reconcile custody without requiring Native readiness. An exact launched
        // process may need closure precisely because readiness is unavailable.
        if let Some(recovery) = self.host.effects.reconcile_profile_browser(
            &profile,
            Some(&self.binding.browser),
            &tabs,
            &[],
        )? {
            let identity = recovery
                .launch
                .process_identity
                .as_ref()
                .ok_or("mode_operation_replacement_identity_missing")?;
            if identity == &self.binding.original || !original_matches(identity)? {
                return Err("mode_operation_replacement_changed".into());
            }
            let mut current = self.binding.browser.clone();
            current.pid = recovery.launch.pid;
            current.cdp_endpoint = recovery.launch.cdp_endpoint;
            current.desktop = recovery.launch.desktop;
            self.host.effects.close_browser(&current)?;
            if original_matches(identity)? {
                return Err("mode_operation_replacement_exit_unproven".into());
            }
        }
        if !self.replacement_absent(request)? {
            return Err("mode_operation_launch_absence_unproven".into());
        }
        self.recovery = None;
        Ok(self.binding.original_mode.clone())
    }
    fn replacement_is_published(
        &mut self,
        request: &Request,
        replacement: &Replacement,
    ) -> Result<bool, String> {
        if self.host.persistence.load_session_state()? != self.host.persisted_state {
            return Err("mode_operation_publication_conflict".into());
        }
        let current = self
            .host
            .state
            .browsers
            .get(&request.browser_id)
            .ok_or("mode_operation_browser_missing")?;
        let expected: RecordedProcessIdentity = serde_json::from_str(&replacement.process_identity)
            .map_err(|_| "mode_operation_replacement_identity_invalid")?;
        let published = BrowserSessionSqliteStore::open_read_only(
            &self.host.persistence.runtime_database_path()?,
        )?
        .published_launch_process_identity(current)
        .map_err(|_| "mode_operation_publication_readback_required")?;
        if published.as_ref() == Some(&expected) {
            if current.profile_id != request.profile_id
                || current.pid != expected.pid
                || !original_matches(&expected)?
                || self.host.persistence.load_session_state()? != self.host.persisted_state
            {
                return Err("mode_operation_replacement_changed:inspect current target".into());
            }
            return Ok(true);
        }
        if current != &self.binding.browser {
            return Err("mode_operation_replacement_changed:inspect current target".into());
        }
        Ok(false)
    }
    fn publish(&mut self, request: &Request, replacement: &Replacement) -> Result<(), String> {
        if let Some(current) = self
            .host
            .state
            .browsers
            .get(&request.browser_id)
            .filter(|browser| browser.pid != self.binding.browser.pid)
        {
            let original: RecordedProcessIdentity =
                serde_json::from_str(&replacement.process_identity)
                    .map_err(|_| "mode_operation_replacement_identity_invalid")?;
            let published = BrowserSessionSqliteStore::open_read_only(
                &self.host.persistence.runtime_database_path()?,
            )?
            .published_launch_process_identity(current)
            .map_err(|_| "mode_operation_publication_readback_required")?;
            if published.as_ref() != Some(&original) || !original_matches(&original)? {
                return Err("mode_operation_replacement_changed:inspect current target".into());
            }
            return Ok(());
        }
        if self.recovery.is_none() {
            let Some(proof) = self.reconcile_replacement(request)? else {
                return Err("mode_operation_replacement_readback_required".into());
            };
            if &proof != replacement {
                return Err("mode_operation_replacement_changed".into());
            }
        }
        let recovery = self
            .recovery
            .take()
            .ok_or("mode_operation_replacement_readback_required")?;
        if self.host.persistence.load_session_state()? != self.host.persisted_state {
            return Err("mode_operation_publication_conflict".into());
        }
        BrowserSessionManager::new(
            &mut self.host.state,
            &self.host.catalog,
            &mut self.host.effects,
            self.host.manager_config.clone(),
        )
        .publish_owner_replacement(&request.browser_id, &request.operation_id, recovery)?;
        self.host.commit_state()
    }
}

/// Only a private owner journal and its published profile fence authorize manual
/// recovery. A public command flag cannot turn an ordinary reopen into conversion.
pub(crate) fn recovery_authorized(
    profile: &BrowserProfileCatalogEntry,
    browser: &ManagedBrowserInstance,
) -> bool {
    let Ok(database) = BrowserSessionSqliteStore::default_sqlite_path() else {
        return false;
    };
    let Ok(store) = BrowserSessionSqliteStore::open_read_only(&database) else {
        return false;
    };
    let Ok(state) = store.load_session_state() else {
        return false;
    };
    let Some(id) = state.profile_operation_ids.get(&profile.id) else {
        return false;
    };
    let Ok(connection) = rusqlite::Connection::open_with_flags(
        database.with_extension("mode.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) else {
        return false;
    };
    let Ok(value) = connection.query_row(
        "SELECT state FROM browser_mode_operations WHERE id=?1",
        [id],
        |row| row.get::<_, String>(0),
    ) else {
        return false;
    };
    serde_json::from_str::<Operation>(&value).is_ok_and(|operation| {
        matches!(
            operation.phase,
            Phase::LaunchDispatched | Phase::LaunchAbsent | Phase::ReplacementPending
        ) && operation.request.browser_id == browser.id
            && operation.request.profile_id == profile.id
            && operation
                .effective_mode
                .as_ref()
                .unwrap_or(&operation.request.requested_mode)
                == &Mode::CdpFree
    })
}
