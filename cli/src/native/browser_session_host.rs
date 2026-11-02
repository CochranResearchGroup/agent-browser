//! Durable host for the ordinary Browser Session Manager path.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_browser_service_model::{
    BrowserDisposableProfilePolicy, BrowserLaunchCustodyStore, BrowserLaunchIntent,
    BrowserProfileCatalog, BrowserSessionEffects, BrowserSessionManager,
    BrowserSessionManagerConfig, BrowserSessionState, CloseBrowserSessionResult,
    CloseBrowserTabResult, OpenBrowserSession, OpenBrowserSessionResult, ReapBrowserSessionsResult,
    RemoteViewDesktopCandidate, SessionEndReason,
};

use super::browser_session_runtime::{
    BrowserManagerRuntime, BrowserManagerRuntimeConfig, BrowserSessionEffectAdapter,
    ManagedBrowserCommandEffects,
};
#[cfg(test)]
use super::browser_session_store::BrowserSessionJsonStore;
use super::browser_session_store::{BrowserProfileCatalogLoad, BrowserSessionSqliteStore};

#[path = "browser_build_open.rs"]
mod browser_build_open;
#[path = "browser_build_selection.rs"]
mod browser_build_selection;
#[path = "browser_mode_host.rs"]
pub(crate) mod browser_mode_host;
#[path = "browser_native_host.rs"]
mod browser_native_host;
#[path = "browser_operation_lookup.rs"]
mod browser_operation_lookup;

#[path = "profile_duplicate_host.rs"]
mod profile_duplicate_host;

const DEFAULT_SESSION_IDLE_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_DISPOSABLE_CLEANUP_DELAY_MS: u64 = 300_000;
const DEFAULT_DISPOSABLE_POLICY_ID: &str = "default";

pub(crate) type DefaultBrowserSessionHost = BrowserSessionHost<
    BrowserSessionSqliteStore,
    super::browser_session_remote_view::RuntimeSessionEffects,
>;

pub(crate) type SharedBrowserSessionHost =
    std::sync::Arc<std::sync::Mutex<Option<DefaultBrowserSessionHost>>>;

/// Validate pending-human configuration before any owner or browser effect.
/// Zero selects no automatic deadline; ordinary agent work has no human hold.
fn pending_human_retention(command: &serde_json::Value) -> Result<Option<u64>, String> {
    let human_handoff = match command.get("humanHandoff") {
        None => false,
        Some(value) => value.as_bool().ok_or("remote_view_human_handoff_invalid")?,
    };
    if !human_handoff {
        return Ok(None);
    }
    let ttl = match command.get("pendingHumanRetentionMs") {
        Some(value) => value
            .as_u64()
            .ok_or("remote_view_pending_human_retention_invalid")?,
        None => configured_u64("AGENT_BROWSER_PENDING_HUMAN_RETENTION_MS", 0)?,
    };
    if ttl != 0 {
        command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms)
            .checked_add(ttl)
            .ok_or("remote_view_pending_human_retention_invalid")?;
    }
    Ok(Some(ttl))
}

/// One route definition shared by sockets, scheduler admission and execution.
pub(crate) fn ordinary_managed_action(action: &str) -> bool {
    !super::actions::action_skips_browser_launch(action)
        && !matches!(action, "tab_switch" | "window_new")
}

pub(crate) fn managed_command_session_name(lane: &str, command: &serde_json::Value) -> String {
    optional_string(command, "sessionName")
        .unwrap_or(lane)
        .to_string()
}

fn matching_ordinary_session(
    state: &BrowserSessionState,
    name: &str,
    command: &serde_json::Value,
) -> bool {
    let profile = optional_string(command, "profileId")
        .or_else(|| optional_string(command, "runtimeProfile"));
    state.sessions.values().any(|session| {
        session.name == name && profile.is_none_or(|profile| session.profile_id == profile)
    })
}

/// Read retained session routing without constructing a runtime or launching.
pub(crate) fn retained_managed_session_selected(name: &str, command: &serde_json::Value) -> bool {
    BrowserSessionSqliteStore::default_sqlite()
        .and_then(|store| store.load_session_state())
        .is_ok_and(|state| matching_ordinary_session(&state, name, command))
}

/// Metadata-only selection. No host construction, provider read or browser launch.
pub(crate) async fn shared_managed_request_selected(
    shared: SharedBrowserSessionHost,
    name: String,
    command: serde_json::Value,
) -> Result<bool, String> {
    let action = command
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if !ordinary_managed_action(action) {
        return Ok(false);
    }
    tokio::task::spawn_blocking(move || {
        let host = shared
            .lock()
            .map_err(|_| "browser_session_host_lock_poisoned")?;
        if let Some(host) = host.as_ref() {
            return Ok(host.effects.admits_ordinary_sessions()
                || matching_ordinary_session(host.state(), &name, &command));
        }
        if super::browser_session_remote_view::remote_view_settings_present() {
            return Ok(true);
        }
        Ok(matching_ordinary_session(
            &BrowserSessionSqliteStore::default_sqlite()?.load_session_state()?,
            &name,
            &command,
        ))
    })
    .await
    .map_err(|_| "browser_session_host_join_failed")?
}

/// Observe an executable through the existing cross-namespace process adapter.
/// Retained browsers can outlive the runtime host that launched them.
fn shared_browser_executable(pid: u32) -> Result<Option<PathBuf>, String> {
    match crate::process_identity::observe_process(pid) {
        crate::process_identity::ProcessObservation::Missing => Ok(None),
        crate::process_identity::ProcessObservation::Observed(observation) => observation
            .executable_path
            .map(PathBuf::from)
            .map(Some)
            .ok_or_else(|| format!("browser_build_process_identity_unavailable:pid={pid}")),
        crate::process_identity::ProcessObservation::Failed { reason } => Err(format!(
            "browser_build_process_identity_unavailable:pid={pid}:{reason}"
        )),
    }
}

/// Select the existing-tab operator workflow, never profile acquisition or
/// navigation. This is dispatch shape only; the owner still verifies the signed
/// focus, current target binding and Native readiness before any effect.
pub(crate) fn existing_native_task_handoff(command: &serde_json::Value) -> bool {
    command["action"] == "remote_view_open"
        && command["operatorFocus"] == true
        && command["dryRun"] != true
        && command.get("url").is_none()
        && command["browserId"]
            .as_str()
            .is_some_and(|id| id.starts_with("browser:"))
        && ["sessionName", "targetId"].iter().all(|field| {
            command
                .get(field)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
        })
}

/// Open a client-selected persistent profile through the application pool.
/// Explicit paths are registered once; a changed ID/path binding fails closed.
pub(crate) async fn execute_shared_remote_view_open(
    shared: SharedBrowserSessionHost,
    name: String,
    mut command: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    command["humanHandoff"] = serde_json::json!(true);
    pending_human_retention(&command)?;
    tokio::task::spawn_blocking(move || {
        let mut shared = shared
            .lock()
            .map_err(|_| "browser_session_host_lock_poisoned")?;
        if shared.is_none() {
            *shared = Some(load_default_browser_session_host()?);
        }
        let host = shared.as_mut().ok_or("browser_session_host_missing")?;
        host.refresh_before_ordinary_command()?;
        super::browser_host_control::check_state(&host.state, &name, &command)?;
        super::browser_host_reconnect::check_state(&host.state, &name, &command)?;
        super::browser_host_policy::prepare_command(&host.state, &name, &mut command)?;
        super::browser_host_policy::check_state(&host.state, &name, &command)?;
        if command.get("url").is_none() && command.get("browserId").is_some() && command.get("targetId").is_some() {
            return host.prepare_existing_task_handoff(&name, &command).map(Some);
        }
        validate_handoff_ttl(&command, current_unix_ms(), &agent_browser_service_model::BrowserRetentionPolicy::default())?;
        if command["browserBuild"] == "cdp_free_headed" {
            command["manualLogin"] = serde_json::json!(true);
        }
        let selected_build = optional_string(&command, "browserBuild");
        let selected_executable = match selected_build {
            None => None,
            Some("stock_chrome" | "cdp_free_headed") => {
                let launcher = super::cdp::chrome::find_chrome()
                    .ok_or("browser_build_executable_unavailable:stock_chrome")?
                    .canonicalize()
                    .map_err(|_| "browser_build_executable_unavailable:stock_chrome")?;
                // Linux Google Chrome discovery can return its shell wrapper.
                // Launch the sibling browser binary so the process proof binds
                // the executable itself rather than the wrapper script.
                let binary = launcher.parent().map(|parent| parent.join("chrome"));
                Some(
                    if launcher
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("google-chrome"))
                        && binary.as_ref().is_some_and(|path| path.is_file())
                    {
                        binary.ok_or("browser_build_executable_unavailable:stock_chrome")?
                    } else {
                        launcher
                    },
                )
            }
            Some("stealthcdp_chromium") => {
                let flags = crate::flags::parse_flags(&["agent-browser".into()]);
                let manifest = flags
                    .browser_build_manifest_status
                    .get("stealthcdp_chromium")
                    .filter(|status| status["ready"] == true)
                    .ok_or("browser_build_manifest_unavailable:stealthcdp_chromium")?;
                Some(PathBuf::from(required_string(manifest, "executablePath")?))
            }
            Some(_) => return Err("browser_build_selector_unsupported".into()),
        };
        if let Some(path) = &selected_executable {
            let expected = path
                .canonicalize()
                .map_err(|_| "browser_build_executable_unavailable")?;
            let profile = optional_string(&command, "runtimeProfile")
                .or_else(|| optional_string(&command, "profileId"));
            for browser in host
                .state
                .browsers
                .values()
                .filter(|browser| profile == Some(browser.profile_id.as_str()))
            {
                let Some(actual) = shared_browser_executable(browser.pid)? else {
                    // Idle cleanup can leave retained logical ownership.
                    // Recovery below checks absence and launches the selected build.
                    continue;
                };
                if actual != expected {
                    return Err("browser_build_reuse_conflict".into());
                }
            }
            if let Some(profile) = profile {
                host.state.profile_executables.insert(profile.into(), expected.to_string_lossy().into_owned());
                host.commit_state()?;
                host.effects.select_profile_browser_executable(
                    profile.into(), expected.to_string_lossy().into_owned(),
                )?;
            } else {
                host.effects
                    .select_browser_executable(expected.to_string_lossy().into_owned())?;
            }
        }

        // Restore retained runtime ownership before navigation uses its target.
        // The manager handles live reuse and positively absent idle-cleaned owners.
        if let Some(profile) = optional_string(&command, "runtimeProfile") {
            let retained: Vec<String> = host.state.browsers.values()
                .filter(|browser| browser.profile_id == profile)
                .map(|browser| browser.id.clone())
                .collect();
            for browser_id in retained {
                host.require_explicit_hosted_recovery(&browser_id)?;
                host.manager()?.recover_retained_browser(&browser_id)?;
                host.commit_state()?;
            }
        }
        let mut response = host.execute_managed_command(&name, &command)?;
        if let Some(response) = response
            .as_mut()
            .filter(|response| response["success"] == true)
        {
            let identity = response["browserSession"].clone();
            if let Some(build) = selected_build {
                let browser = host
                    .state
                    .browsers
                    .get(required_string(&identity, "browserId")?)
                    .ok_or("browser_build_process_identity_missing")?;
                let actual = shared_browser_executable(browser.pid)?
                    .ok_or("browser_build_process_identity_unavailable")?;
                if selected_executable
                    .as_ref()
                    .and_then(|path| path.canonicalize().ok())
                    .as_ref()
                    != Some(&actual)
                {
                    return Err("browser_build_executable_mismatch".into());
                }
                let profile_id = browser.profile_id.clone();
                host.state.profile_executables.insert(profile_id, actual.to_string_lossy().into_owned());
                host.commit_state()?;
                response["browserBuildProof"] = serde_json::json!({
                    "state":"matched", "requestedBrowserBuild":build,
                    "selectedBrowserBuild":build, "actualExecutablePath":actual,
                });
            }

            let handoff_id = required_string(&identity, "handoffId")?;
            if host.effects.admits_ordinary_sessions() {
                let presentation = host.handle_command_result(&serde_json::json!({
                    "action":"service_remote_view_handoff_resolve",
                    "handoffId":handoff_id,
                    "nativeDesktop":true,
                }))?;
                response["presentationState"] = serde_json::json!("native_desktop");
                response["operatorVisible"] = presentation["operatorVisible"].clone();
                return Ok(Some(response.clone()));
            }
            let now_ms = current_unix_ms();
            let presentation = host.resolve_tab_handoff_view(handoff_id, now_ms).and_then(|view| {
                host.qualify_tab_handoff_presentation(handoff_id, &view, now_ms)?;
                Ok(view)
            });
            match presentation {
                Ok(view) => {
                    response["presentationState"] = serde_json::json!(view.presentation_state);
                    response["operatorVisible"] = serde_json::json!({"state":"ready","proofScope":"addressed_tab_owned_window_and_installed_transport"});
                }
                Err(error) => response["presentationError"] = serde_json::json!(error),
            }
        }
        Ok(response)
    })
    .await
    .map_err(|_| "browser_session_host_join_failed")?
}

pub(crate) async fn execute_shared_managed_command(
    shared: SharedBrowserSessionHost,
    name: String,
    command: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if command["action"] == "navigate"
        && optional_string(&command, "browserBuild").is_some()
        && super::browser_session_remote_view::remote_view_settings_present()
    {
        return execute_shared_remote_view_open(shared, name, command).await;
    }
    tokio::task::spawn_blocking(move || {
        let mut host = shared
            .lock()
            .map_err(|_| "browser_session_host_lock_poisoned")?;
        if host.is_none() {
            if !super::browser_session_remote_view::remote_view_settings_present()
                && !matching_ordinary_session(
                    &BrowserSessionSqliteStore::default_sqlite()?.load_session_state()?,
                    &name,
                    &command,
                )
            {
                return Ok(None);
            }
            *host = Some(load_default_browser_session_host()?);
        }
        let host = host.as_mut().ok_or("browser_session_host_missing")?;
        host.execute_managed_command(&name, &command)
    })
    .await
    .map_err(|_| "browser_session_host_join_failed")?
}

/// Explicit managed closure never falls back to a different daemon or launches
/// a replacement. Authenticate before constructing the retained runtime owner.
pub(crate) async fn close_shared_operator_tab(
    shared: SharedBrowserSessionHost,
    command: serde_json::Value,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
        let mut host = shared
            .lock()
            .map_err(|_| "browser_session_host_lock_poisoned")?;
        let snapshot = LockedServiceStateRepository::default_json()?.load_snapshot()?;
        super::stream::verify_operator_tab_close(&snapshot, &command)?;
        if host.is_none() {
            *host = Some(load_default_browser_session_host()?);
        }
        host.as_mut()
            .ok_or("browser_session_host_missing")?
            .close_operator_tab(&command)
    })
    .await
    .map_err(|_| "browser_session_host_join_failed")?
}

/// Both socket dispatch and queued service resolution borrow the same owner.
pub(crate) async fn resolve_shared_handoff_command(
    shared: SharedBrowserSessionHost,
    command: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let Some(id) = optional_string(&command, "handoffId").map(str::to_string) else {
        return Ok(None);
    };
    tokio::task::spawn_blocking(move || {
        let mut host = shared
            .lock()
            .map_err(|_| "browser_session_host_lock_poisoned")?;
        if host.is_none() {
            if !BrowserSessionSqliteStore::default_sqlite()?
                .load_session_state()?
                .remote_view_tab_handoffs
                .contains_key(&id)
            {
                return Ok(None);
            }
            *host = Some(load_default_browser_session_host()?);
        }
        let host = host.as_mut().ok_or("browser_session_host_missing")?;
        if !host.state().remote_view_tab_handoffs.contains_key(&id) {
            return Ok(None);
        }
        // Native Remote View owns operator authentication and desktop access.
        // Ordinary client resolution must not issue a separate viewer grant.
        let mut command = command;
        if host.effects.admits_ordinary_sessions() {
            command["nativeDesktop"] = serde_json::json!(true);
        }
        host.handle_command_result(&command).map(Some)
    })
    .await
    .map_err(|_| "browser_session_host_join_failed")?
}

/// Reaping loads retained physical ownership after restart, without admitting
/// launches or creating an empty session host in an unused environment.
pub(crate) fn reap_shared_browser_sessions(
    shared: SharedBrowserSessionHost,
) -> Result<ReapBrowserSessionsResult, String> {
    let mut host = shared
        .lock()
        .map_err(|_| "browser_session_host_lock_poisoned")?;
    reap_browser_session_host(&mut host, || {
        let state = BrowserSessionSqliteStore::default_sqlite()?.load_session_state()?;
        if state.browsers.is_empty() && state.disposable_profiles.is_empty() {
            return Ok(None);
        }
        load_default_browser_session_host().map(Some)
    })
}

fn reap_browser_session_host<P: BrowserSessionPersistence, E: BrowserSessionEffects>(
    host: &mut Option<BrowserSessionHost<P, E>>,
    load: impl FnOnce() -> Result<Option<BrowserSessionHost<P, E>>, String>,
) -> Result<ReapBrowserSessionsResult, String> {
    if host.is_none() {
        *host = load()?;
    }
    match host.as_mut() {
        Some(host) => host.reap_current(),
        None => Ok(ReapBrowserSessionsResult::default()),
    }
}

pub(crate) fn load_default_browser_session_host() -> Result<DefaultBrowserSessionHost, String> {
    let legacy_state_path = super::service_store::default_service_state_path()?;
    let store = BrowserSessionSqliteStore::default_sqlite()?;
    let session_idle_timeout_ms = configured_u64(
        "AGENT_BROWSER_SESSION_IDLE_TIMEOUT_MS",
        DEFAULT_SESSION_IDLE_TIMEOUT_MS,
    )?;
    let cleanup_delay_ms = configured_u64(
        "AGENT_BROWSER_DISPOSABLE_CLEANUP_DELAY_MS",
        DEFAULT_DISPOSABLE_CLEANUP_DELAY_MS,
    )?;
    let disposable_root = match std::env::var_os("AGENT_BROWSER_DISPOSABLE_PROFILE_ROOT") {
        Some(value) => PathBuf::from(value),
        None => legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?
            .join("disposable-profiles"),
    };
    if !disposable_root.is_absolute() {
        return Err("browser_disposable_root_not_absolute".to_string());
    }
    let display = std::env::var("AGENT_BROWSER_SESSION_DISPLAY").ok();
    // Remote View desktop candidates must arrive through its public consumer
    // contract. Never reconstruct them from Agent Browser's legacy route pool.
    let remote_view = super::browser_session_remote_view::configured_remote_view()?;
    let remote_view_desktops = Vec::new();
    let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
        headless: display.is_none(),
        executable_path: std::env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok(),
        display,
        remote_headed: false,
        maximum_browser_processes: None,
        remote_view_desktop_contexts: Vec::new(),
    })?;
    let effects = super::browser_session_remote_view::runtime_effects(
        BrowserSessionEffectAdapter::new(runtime),
        remote_view,
    )?;
    let mut host = BrowserSessionHost::load(
        store,
        effects,
        &legacy_state_path,
        BrowserSessionHostConfig {
            session_idle_timeout_ms,
            remote_view_desktops,
            default_disposable_policy: Some(BrowserDisposableProfilePolicy {
                id: DEFAULT_DISPOSABLE_POLICY_ID.to_string(),
                user_data_root: disposable_root.to_string_lossy().into_owned(),
                cleanup_delay_ms,
                maximum_retained_profiles: 20,
                maximum_total_bytes: 10 * 1024 * 1024 * 1024,
            }),
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
        },
    )?;
    let mut policy = host.state.retention_policy.clone();
    if std::env::var_os("AGENT_BROWSER_DURABLE_HANDOFF_TTL_MS").is_some() {
        let ttl = configured_u64("AGENT_BROWSER_DURABLE_HANDOFF_TTL_MS", 0)?;
        policy.durable_handoff_ttl_ms = (ttl != 0).then_some(ttl);
    }
    if std::env::var_os("AGENT_BROWSER_DISPOSABLE_HANDOFF_TTL_MS").is_some() {
        let ttl = configured_u64("AGENT_BROWSER_DISPOSABLE_HANDOFF_TTL_MS", 0)?;
        policy.disposable_handoff_ttl_ms = (ttl != 0).then_some(ttl);
    }
    if std::env::var_os("AGENT_BROWSER_DISPOSABLE_INACTIVITY_MS").is_some() {
        policy.disposable_inactivity_ms =
            configured_u64("AGENT_BROWSER_DISPOSABLE_INACTIVITY_MS", 86_400_000)?;
    }
    policy.validate()?;
    if host.state.retention_policy != policy {
        host.state.retention_policy = policy;
        host.commit_state()?;
    }
    Ok(host)
}

fn configured_u64(name: &str, default: u64) -> Result<u64, String> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|error| format!("browser_session_config_invalid:{name}:{error}")),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(format!("browser_session_config_invalid:{name}:{error}")),
    }
}

pub(crate) trait BrowserSessionPersistence {
    fn runtime_database_path(&self) -> Result<PathBuf, String> {
        Err("browser_runtime_database_path_unavailable".into())
    }

    /// A new borrowed profile and its lifecycle fence must become visible in
    /// the same transaction. Legacy stores refuse rather than leak admission.
    fn publish_app_attachment(
        &mut self,
        _expected: &BrowserSessionState,
        _next: &BrowserSessionState,
        _expected_catalog: &BrowserProfileCatalog,
        _next_catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        Err("browser_attachment_atomic_publication_unsupported".into())
    }
    /// Publish observed launch custody and the aggregate in one transaction.
    /// Persistence implementations without that facility fail closed.
    fn publish_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        intent: Option<&BrowserLaunchIntent>,
    ) -> Result<(), String> {
        if intent.is_some() {
            return Err("browser_session_launch_publication_unsupported".to_string());
        }
        self.compare_and_save_session_state(expected, state)
    }
    /// Join retirement custody to session publication; legacy stores refuse
    /// this operation instead of publishing a partially settled close.
    fn publish_session_effects(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        intent: Option<&BrowserLaunchIntent>,
        retirement_ids: &[String],
        activity_ids: &[String],
    ) -> Result<(), String> {
        if !retirement_ids.is_empty() || !activity_ids.is_empty() {
            return Err("browser_session_idle_publication_unsupported".into());
        }
        self.publish_session_state(expected, state, intent)
    }
    fn load_session_state(&self) -> Result<BrowserSessionState, String>;
    fn compare_and_save_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), String>;
    fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String>;
    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String>;
    fn load_owner_snapshot(&self) -> Result<(BrowserSessionState, BrowserProfileCatalog), String> {
        Ok((
            self.load_session_state()?,
            self.load_or_import_profile_catalog(Path::new(""))?.catalog,
        ))
    }
    fn publish_profile_catalog(
        &self,
        expected: &BrowserProfileCatalog,
        next: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        if self.load_or_import_profile_catalog(Path::new(""))?.catalog != *expected {
            return Err("browser_profile_catalog_publication_conflict".into());
        }
        self.save_profile_catalog(next)
    }
}

impl BrowserSessionPersistence for BrowserSessionSqliteStore {
    fn runtime_database_path(&self) -> Result<PathBuf, String> {
        self.database_path()
    }

    fn publish_app_attachment(
        &mut self,
        expected: &BrowserSessionState,
        next: &BrowserSessionState,
        expected_catalog: &BrowserProfileCatalog,
        next_catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        self.compare_and_save_app_attachment(expected, next, expected_catalog, next_catalog)
    }
    fn load_owner_snapshot(&self) -> Result<(BrowserSessionState, BrowserProfileCatalog), String> {
        self.load_workspace_snapshot()
    }
    fn publish_profile_catalog(
        &self,
        expected: &BrowserProfileCatalog,
        next: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        self.compare_and_save_profile_catalog(expected, next)
    }

    fn publish_session_effects(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        intent: Option<&BrowserLaunchIntent>,
        retirement_ids: &[String],
        activity_ids: &[String],
    ) -> Result<(), String> {
        if retirement_ids.is_empty() && activity_ids.is_empty() {
            self.publish_session_state(expected, state, intent)
        } else {
            self.publish_session_retirements(expected, state, retirement_ids, intent, activity_ids)
                .map_err(|_| "browser_session_idle_publication_conflict".into())
        }
    }
    fn publish_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        intent: Option<&BrowserLaunchIntent>,
    ) -> Result<(), String> {
        if let Some(intent) = intent {
            self.publish_launch_intent(intent, expected, state)
                .map_err(|_| "browser_session_launch_publication_conflict".to_string())
        } else {
            self.compare_and_save_session_state(expected, state)
        }
    }
    fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        BrowserSessionSqliteStore::load_session_state(self)
    }

    fn compare_and_save_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), String> {
        BrowserSessionSqliteStore::compare_and_save_session_state(self, expected, state)
    }

    fn load_or_import_profile_catalog(
        &self,
        _legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        BrowserSessionSqliteStore::load_profile_catalog(self)
    }

    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String> {
        BrowserSessionSqliteStore::save_profile_catalog(self, catalog)
    }
}

#[cfg(test)]
impl BrowserSessionPersistence for super::browser_session_store::BrowserSessionJsonStore {
    fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        self.load_session_state()
    }

    fn compare_and_save_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), String> {
        if self.load_session_state()? != *expected {
            return Err("browser_session_publication_conflict".into());
        }
        self.save_session_state(state)
    }

    fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        self.load_or_import_profile_catalog(legacy_service_state_path)
    }

    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String> {
        self.save_profile_catalog(catalog)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSessionHostConfig {
    pub(crate) session_idle_timeout_ms: u64,
    pub(crate) remote_view_desktops: Vec<RemoteViewDesktopCandidate>,
    pub(crate) default_disposable_policy: Option<BrowserDisposableProfilePolicy>,
    pub(crate) exact_url_history_maximum_bytes: u64,
}

pub(crate) struct BrowserSessionHost<P, E> {
    persistence: P,
    effects: E,
    catalog: BrowserProfileCatalog,
    state: BrowserSessionState,
    persisted_state: BrowserSessionState,
    manager_config: BrowserSessionManagerConfig,
    exact_url_history_maximum_bytes: u64,
    native_use_cursor: usize,
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(crate) fn load(
        mut persistence: P,
        effects: E,
        legacy_service_state_path: &Path,
        config: BrowserSessionHostConfig,
    ) -> Result<Self, String> {
        let previous = persistence.load_session_state()?;
        let mut state = previous.clone();
        state.retention_policy.validate()?;
        for link in state.remote_view_tab_handoffs.values_mut() {
            // Migration adds creation metadata without imposing a new lifetime
            // on an existing link or renewing its recorded finite expiry.
            link.initialize_profile_retention(current_unix_ms(), None)?;
        }
        if state != previous {
            persistence.compare_and_save_session_state(&previous, &state)?;
        }
        let mut catalog_load =
            persistence.load_or_import_profile_catalog(legacy_service_state_path)?;
        let original_catalog = catalog_load.catalog.clone();
        let mut catalog_changed = false;
        if let Some(policy) = config.default_disposable_policy {
            if !catalog_load
                .catalog
                .disposable_policies
                .contains_key(&policy.id)
            {
                catalog_load
                    .catalog
                    .disposable_policies
                    .insert(policy.id.clone(), policy);
                catalog_changed = true;
            }
        }
        if catalog_changed {
            persistence.publish_profile_catalog(&original_catalog, &catalog_load.catalog)?;
        }
        Ok(Self {
            persistence,
            effects,
            catalog: catalog_load.catalog,
            persisted_state: state.clone(),
            state,
            manager_config: BrowserSessionManagerConfig {
                session_idle_timeout_ms: config.session_idle_timeout_ms,
                remote_view_desktops: config.remote_view_desktops,
            },
            exact_url_history_maximum_bytes: config.exact_url_history_maximum_bytes,
            native_use_cursor: 0,
        })
    }

    pub(crate) fn open(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        self.open_with_host_binding(request, None)
    }

    fn open_with_host_binding(
        &mut self,
        request: OpenBrowserSession,
        hosted: Option<agent_browser_service_model::BrowserHostBinding>,
    ) -> Result<OpenBrowserSessionResult, String> {
        self.open_with_host_binding_policy(request, hosted, None)
    }

    fn open_with_host_binding_policy(
        &mut self,
        request: OpenBrowserSession,
        hosted: Option<agent_browser_service_model::BrowserHostBinding>,
        task_policy: Option<&serde_json::Value>,
    ) -> Result<OpenBrowserSessionResult, String> {
        let existing_task = self
            .state
            .sessions
            .values()
            .any(|session| session.name == request.session_name);
        let existing_profile = match &request.profile_intent {
            agent_browser_service_model::BrowserProfileIntent::Exact { profile_id } => self
                .state
                .browsers
                .values()
                .any(|browser| browser.profile_id == *profile_id),
            _ => false,
        };
        if !existing_task
            && !existing_profile
            && super::browser_launch_admission::observe_browser_launch_admission(
                Path::new("."),
                None,
            )
            .is_memory_pressure_only()
        {
            // Reuse only the owner reaper's established idle/expiry eligibility.
            // Existing task/profile continuation is never reclaimed here.
            self.reap(request.activity_at_ms)?;
        }
        let result = match self.manager()?.open(request.clone()) {
            Err(error)
                if error.starts_with("browser_session_retirement_publication_required:")
                    && !self.effects.pending_idle_retirement_ids().is_empty() =>
            {
                let profile_id = error
                    .strip_prefix("browser_session_retirement_publication_required:")
                    .expect("matched publication boundary");
                let continuation = match &request.profile_intent {
                    agent_browser_service_model::BrowserProfileIntent::Disposable { .. }
                        if self.state.disposable_profiles.contains_key(profile_id) =>
                    {
                        let mut continuation = OpenBrowserSession::existing_disposable(
                            &request.session_name,
                            profile_id,
                            request.activity_at_ms,
                        );
                        continuation.placement = request.placement.clone();
                        continuation
                    }
                    agent_browser_service_model::BrowserProfileIntent::Exact {
                        profile_id: requested,
                    }
                    | agent_browser_service_model::BrowserProfileIntent::ExistingDisposable {
                        profile_id: requested,
                    } if requested == profile_id => request,
                    _ => return Err("browser_session_retirement_continuation_invalid".into()),
                };
                // This is a declared pre-launch phase boundary, not a retry of
                // any unknown effect. Completion failure stops before replacement.
                self.commit_state()?;
                self.manager()?.open(continuation)?
            }
            result => result?,
        };
        if let Some(binding) = hosted {
            capture_host_policy_defaults(&mut self.state, &binding, &result.session_id)?;
            let desktop = self
                .state
                .browsers
                .get(&result.browser_id)
                .and_then(|browser| browser.desktop.clone())
                .ok_or("browser_host_desktop_missing")?;
            self.state.host_bindings.insert(
                result.session_id.clone(),
                agent_browser_service_model::BrowserHostTaskBinding {
                    request: binding,
                    desktop,
                },
            );
        }
        if let Some(policy) = task_policy {
            capture_task_policy(
                &mut self.state,
                &result.session_id,
                &serde_json::json!({"taskPolicy":policy}),
            )?;
        }
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn tab_for_navigation(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserTabAcquisition, String> {
        let result = self
            .manager()?
            .tab_for_navigation(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn record_navigation(
        &mut self,
        session_id: &str,
        url: &str,
        visited_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserNavigationRecord, String> {
        let result = self
            .manager()?
            .record_navigation(session_id, url, visited_at_ms)?;
        self.retain_handoff_destination(&result.tab_id, url);
        self.compact_navigation_history()?;
        self.commit_state()?;
        Ok(result)
    }

    fn retain_handoff_destination(&mut self, tab_id: &str, url: &str) {
        for link in self
            .state
            .remote_view_tab_handoffs
            .values_mut()
            .filter(|link| link.tab_id == tab_id)
        {
            if let Some(intent) = &mut link.resume {
                intent.url = Some(url.to_owned());
            }
        }
    }

    pub(crate) fn navigate(
        &mut self,
        session_id: &str,
        url: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserNavigationRecord, String> {
        let result = self.manager()?.navigate(session_id, url, activity_at_ms)?;
        self.retain_handoff_destination(&result.tab_id, url);
        self.compact_navigation_history()?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn focus_browser(
        &mut self,
        browser_id: &str,
        target_id: Option<&str>,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::FocusBrowserResult, String> {
        let result = self
            .manager()?
            .focus_browser(browser_id, target_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn new_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserTabAcquisition, String> {
        let result = self.manager()?.new_tab(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn close_current_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<CloseBrowserTabResult, String> {
        let result = self
            .manager()?
            .close_current_tab(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    /// Resolve the named owner target rather than its mutable current tab.
    /// The physical runtime revalidates operator authority before sending close.
    pub(crate) fn close_operator_tab(
        &mut self,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String>
    where
        E: ManagedBrowserCommandEffects,
    {
        if command["action"] != "tab_close" || command["operatorClose"] != true {
            return Err("operator_tab_close_binding_changed".into());
        }
        let browser_id = required_string(command, "browserId")?;
        let target_id = required_string(command, "targetId")?;
        let name = required_string(command, "sessionName")?;
        let mut matches = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser_id && tab.target_id == target_id);
        let tab = matches
            .next()
            .ok_or("operator_tab_close_target_missing")?
            .clone();
        if matches.next().is_some() {
            return Err("operator_tab_close_target_ambiguous".into());
        }
        let session = self
            .state
            .sessions
            .get(&tab.session_id)
            .filter(|session| session.name == name && session.browser_id == browser_id)
            .ok_or("operator_tab_close_binding_changed")?
            .clone();
        let now_ms = current_unix_ms();
        let mut physical = serde_json::Value::Null;
        let closed = self.manager()?.close_exact_tab_with(
            &session.id,
            &tab.id,
            now_ms,
            |effects, browser, target| {
                let response = effects.execute_command(
                    browser,
                    target,
                    &session.id,
                    &session.name,
                    command,
                )?;
                if response["success"] != true {
                    return Err(response["error"]
                        .as_str()
                        .unwrap_or("tab_close_effect_uncertain")
                        .to_string());
                }
                if response["data"]["targetId"] != target.target_id
                    || response["data"]["physicalTabClosed"] != true
                    || response["data"]["unrelatedTargetsPreserved"] != true
                    || !response["data"]["closed"].is_u64()
                    || !response["data"]["activeIndex"].is_u64()
                {
                    return Err("tab_close_effect_uncertain: exact physical receipt missing".into());
                }
                physical = response["data"].clone();
                Ok(())
            },
        )?;
        self.commit_state()?;
        // Preserve the existing tab_close numeric index fields. Managed owner
        // identities augment that receipt rather than changing their types.
        physical["source"] = serde_json::json!("managed-owner");
        physical["closedTabId"] = serde_json::json!(closed.closed_tab_id);
        physical["currentTabId"] = serde_json::json!(closed.current_tab_id);
        physical["browserClosed"] =
            serde_json::json!(!self.state.browsers.contains_key(browser_id));
        Ok(physical)
    }

    pub(crate) fn close_session(
        &mut self,
        session_id: &str,
        reason: SessionEndReason,
        ended_at_ms: u64,
    ) -> Result<CloseBrowserSessionResult, String> {
        let result = self
            .manager()?
            .close_session(session_id, reason, ended_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    fn native_handoff_target(
        &self,
        id: &str,
        now_ms: u64,
    ) -> Result<
        (
            agent_browser_service_model::RemoteViewTabHandoffTarget,
            String,
        ),
        String,
    > {
        self.state
            .remote_view_tab_handoffs
            .get(id)
            .ok_or("remote_view_tab_handoff_missing")?
            .check_link_at(now_ms)?;
        let target = agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, id)?;
        let digest = agent_browser_service_model::remote_view_native_target_digest(id, &target)?;
        Ok((target, digest))
    }

    fn prepare_native_handoff_observation(
        &mut self,
        id: &str,
        now_ms: u64,
    ) -> Result<serde_json::Value, String> {
        use agent_browser_service_model::{
            RemoteViewNativeHandoffUseState, RemoteViewTabNativeObservation,
        };
        let (target, digest) = self.native_handoff_target(id, now_ms)?;
        self.effects.begin_operation(&self.persisted_state)?;
        let provider_target = self.effects.native_handoff_assignment_target(&target)?;
        if self.native_handoff_target(id, now_ms)?.1 != digest
            || self.persistence.load_session_state()? != self.persisted_state
        {
            return Err("remote_view_human_use_target_changed".into());
        }
        let previous = self
            .state
            .remote_view_tab_handoffs
            .get(id)
            .and_then(|link| link.native_observation.clone());
        let mut replace = previous.as_ref().is_none_or(|binding| {
            binding.target_digest != digest || binding.provider_target != provider_target
        });
        if let Some(binding) = previous.as_ref().filter(|binding| {
            binding.target_digest == digest && binding.provider_target == provider_target
        }) {
            if let Some(observation) = &binding.observation {
                self.effects.begin_operation(&self.persisted_state)?;
                let read = self
                    .effects
                    .observe_native_handoff_use(&target, observation)?;
                if self.native_handoff_target(id, now_ms)?.1 != digest
                    || self.persistence.load_session_state()? != self.persisted_state
                {
                    return Err("remote_view_human_use_target_changed".into());
                }
                replace = read.state == RemoteViewNativeHandoffUseState::Expired;
            }
        }
        if replace {
            self.state
                .remote_view_tab_handoffs
                .get_mut(id)
                .ok_or("remote_view_tab_handoff_missing")?
                .native_observation = Some(RemoteViewTabNativeObservation {
                target_digest: digest.clone(),
                provider_target: provider_target.clone(),
                idempotency_key: uuid::Uuid::new_v4().to_string(),
                observation: None,
            });
            self.commit_state()?;
        }
        let binding = self
            .state
            .remote_view_tab_handoffs
            .get(id)
            .and_then(|link| link.native_observation.clone())
            .ok_or("remote_view_native_registration_missing")?;
        let observation = match binding.observation {
            Some(observation) => observation,
            None => {
                self.effects.begin_operation(&self.persisted_state)?;
                let observation = self.effects.register_native_handoff_observation(
                    &target,
                    &binding.provider_target,
                    &digest,
                    &binding.idempotency_key,
                )?;
                if self.native_handoff_target(id, now_ms)?.1 != digest
                    || self.persistence.load_session_state()? != self.persisted_state
                {
                    return Err("remote_view_human_use_target_changed".into());
                }
                self.state
                    .remote_view_tab_handoffs
                    .get_mut(id)
                    .and_then(|link| link.native_observation.as_mut())
                    .ok_or("remote_view_native_registration_missing")?
                    .observation = Some(observation.clone());
                self.commit_state()?;
                observation
            }
        };
        Ok(
            serde_json::json!({"handoffId":id,"tabId":target.tab.id,"targetId":target.tab.target_id,"correlationId":observation.correlation_id,"status":"native_use_pending"}),
        )
    }

    /// Successful-use facts come only from the private provider transport. The
    /// host joins current state again before native focus and one-time mutation.
    fn consume_native_handoff_use(
        &mut self,
        id: &str,
        now_ms: u64,
    ) -> Result<serde_json::Value, String> {
        use agent_browser_service_model::RemoteViewNativeHandoffUseState;
        let (target, digest) = self.native_handoff_target(id, now_ms)?;
        let binding = self
            .state
            .remote_view_tab_handoffs
            .get(id)
            .and_then(|link| link.native_observation.clone())
            .ok_or("remote_view_native_registration_missing")?;
        if binding.target_digest != digest {
            return Err("remote_view_human_use_target_changed".into());
        }
        let observation = binding
            .observation
            .as_ref()
            .ok_or("remote_view_native_registration_missing")?;
        if observation.target_digest != digest || observation.target != binding.provider_target {
            return Err("remote_view_human_use_target_changed".into());
        }
        self.effects.begin_operation(&self.persisted_state)?;
        let read = self
            .effects
            .observe_native_handoff_use(&target, observation)?;
        if self.native_handoff_target(id, now_ms)?.1 != digest
            || self.persistence.load_session_state()? != self.persisted_state
        {
            return Err("remote_view_human_use_target_changed".into());
        }
        if read.state != RemoteViewNativeHandoffUseState::Used {
            return Ok(
                serde_json::json!({"handoffId":id,"tabId":target.tab.id,"status":if read.state == RemoteViewNativeHandoffUseState::Expired {"native_use_expired"} else {"native_use_pending"}}),
            );
        }
        let used_at_ms = read
            .observation
            .used
            .as_ref()
            .ok_or("remote_view_human_use_evidence_invalid")?
            .recorded_at_ms;
        if !self.state.pending_human_tabs.contains_key(&target.tab.id)
            && self
                .state
                .tab_metadata
                .get(&target.tab.id)
                .is_some_and(|metadata| metadata.first_human_use_at_ms.is_some())
        {
            return Ok(
                serde_json::json!({"handoffId":id,"tabId":target.tab.id,"status":"human_use_already_confirmed"}),
            );
        }
        let activity_at_ms = used_at_ms.max(target.tab.last_activity_at_ms);
        let session_activity_at_ms = target.session.last_activity_at_ms;
        let session_expires_at_ms = target.session.expires_at_ms;
        self.manager()?.focus_browser(
            &target.browser.id,
            Some(&target.tab.target_id),
            activity_at_ms,
        )?;
        if let Some(session) = self.state.sessions.get_mut(&target.session.id) {
            session.last_activity_at_ms = session.last_activity_at_ms.max(session_activity_at_ms);
            session.expires_at_ms = session.expires_at_ms.max(session_expires_at_ms);
        }
        self.state.pending_human_tabs.remove(&target.tab.id);
        self.state
            .tab_metadata
            .entry(target.tab.id.clone())
            .or_default()
            .first_human_use_at_ms
            .get_or_insert(used_at_ms);
        let metadata = self.state.tab_metadata.get(&target.tab.id).cloned();
        let resume = agent_browser_service_model::remote_view_handoff_resume_intent(
            &self.state,
            id,
            now_ms,
        )?;
        for link in self
            .state
            .remote_view_tab_handoffs
            .values_mut()
            .filter(|link| link.tab_id == target.tab.id && link.session_id == target.session.id)
        {
            link.resume
                .get_or_insert_with(|| resume.clone())
                .task_metadata = metadata.clone();
        }
        self.state
            .remote_view_tab_handoffs
            .get_mut(id)
            .and_then(|link| link.native_observation.as_mut())
            .ok_or("remote_view_native_registration_missing")?
            .observation = Some(read.observation);
        if let Err(error) = self.commit_state() {
            // A lost reply is successful only when the complete proposed
            // aggregate is already durable. Otherwise discard staged use so
            // Properties and a later retry retain the last published truth.
            if self
                .persistence
                .load_session_state()
                .is_ok_and(|state| state == self.state)
            {
                self.persisted_state = self.state.clone();
                // Exact publication readback includes the transaction's
                // activity release. Drop its stale process-local IDs too.
                self.effects.acknowledge_launch_publication();
            } else {
                self.state = self.persisted_state.clone();
                return Err(error);
            }
        }
        Ok(
            serde_json::json!({"handoffId":id,"tabId":target.tab.id,"status":"human_use_confirmed","usedAtMs":used_at_ms}),
        )
    }

    fn reap_phase(
        &mut self,
        now_ms: u64,
        borrowed_only: bool,
        protected: &BTreeSet<String>,
    ) -> Result<ReapBrowserSessionsResult, String> {
        let result = if borrowed_only {
            self.borrowed_retirement_manager()
                .and_then(|mut manager| manager.reap_borrowed_task_tabs(now_ms))
        } else {
            self.manager()
                .and_then(|mut manager| manager.reap_with_protected_sessions(now_ms, protected))
        };
        match result.and_then(|result| {
            self.commit_state()?;
            Ok(result)
        }) {
            Ok(result) => Ok(result),
            Err(reason) => {
                // Restore only this phase. Earlier qualified task outcomes are
                // already durable; physical-effect journals/fences are retained.
                self.state = self
                    .persistence
                    .load_session_state()
                    .unwrap_or_else(|_| self.persisted_state.clone());
                self.persisted_state = self.state.clone();
                Err(reason)
            }
        }
    }

    pub(crate) fn reap(&mut self, now_ms: u64) -> Result<ReapBrowserSessionsResult, String> {
        // Borrowed targets have no Native operation journal to recover. Publish
        // their qualified outcomes before unrelated Native/viewer/profile work.
        let mut result = self.reap_phase(now_ms, true, &BTreeSet::new())?;
        let deferred_sessions = self
            .state
            .sessions
            .values()
            .filter(|session| {
                result
                    .deferred_browser_errors
                    .contains_key(&session.browser_id)
            })
            .map(|session| session.id.clone())
            .collect();
        // One handoff per pass bounds private transport work under the host
        // mutex. Refresh pending context through the owner, so a perpetual
        // public link does not depend on dashboard navigation for registration.
        let candidates = self
            .state
            .remote_view_tab_handoffs
            .values()
            .filter(|link| {
                link.check_link_at(now_ms).is_ok()
                    && (self.state.pending_human_tabs.contains_key(&link.tab_id)
                        || link
                            .native_observation
                            .as_ref()
                            .is_some_and(|binding| binding.observation.is_some()))
                    && self.state.tabs.contains_key(&link.tab_id)
                    && (self.state.pending_human_tabs.contains_key(&link.tab_id)
                        || self
                            .state
                            .tab_metadata
                            .get(&link.tab_id)
                            .is_none_or(|metadata| metadata.first_human_use_at_ms.is_none()))
            })
            .map(|link| link.id.clone())
            .collect::<Vec<_>>();
        let mut observation_error = None;
        if !candidates.is_empty() {
            let id = &candidates[self.native_use_cursor % candidates.len()];
            self.native_use_cursor = self.native_use_cursor.wrapping_add(1);
            let id = id.clone();
            match self.prepare_native_handoff_observation(&id, now_ms) {
                Ok(_) => {
                    if let Err(reason) = self.consume_native_handoff_use(&id, now_ms) {
                        observation_error = Some(reason);
                    }
                }
                Err(reason) if reason == "remote_view_native_observation_unsupported" => {}
                Err(reason) => observation_error = Some(reason),
            }
        }
        let maintenance = self.reap_phase(now_ms, false, &deferred_sessions)?;
        result.expired_tab_ids.extend(maintenance.expired_tab_ids);
        result
            .expired_session_ids
            .extend(maintenance.expired_session_ids);
        result
            .quota_evicted_session_ids
            .extend(maintenance.quota_evicted_session_ids);
        result
            .closed_browser_ids
            .extend(maintenance.closed_browser_ids);
        result
            .deleted_disposable_profile_ids
            .extend(maintenance.deleted_disposable_profile_ids);
        result
            .deferred_browser_errors
            .extend(maintenance.deferred_browser_errors);
        // A provider observation failure must remain visible, but cannot
        // starve unrelated eligible expiry. Existing manager/publication fences
        // still reject conflicting state before any cleanup is published.
        if let Some(reason) = observation_error {
            return Err(reason);
        }
        Ok(result)
    }

    /// Resolve a logical tab's provider grant. Issuance is not pixel readiness.
    /// An unresolved request keeps its durable key across host restart.
    pub(crate) fn resolve_tab_handoff_view(
        &mut self,
        handoff_id: &str,
        now_ms: u64,
    ) -> Result<agent_browser_service_model::RemoteViewApplicationViewIssuance, String> {
        if self.persistence.load_session_state()? != self.persisted_state {
            return Err("browser_session_publication_conflict".into());
        }
        let retained =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff_id)?;
        self.require_explicit_hosted_recovery(&retained.browser.id)?;
        self.manager()?
            .recover_retained_browser(&retained.browser.id)?;
        self.commit_state()?;
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff_id)?;
        let record = self
            .state
            .remote_view_tab_handoffs
            .get_mut(handoff_id)
            .ok_or("remote_view_tab_handoff_missing")?;
        if record.view.is_none() {
            record.view = Some(agent_browser_service_model::RemoteViewTabView {
                idempotency_key: uuid::Uuid::new_v4().to_string(),
                issuance: None,
            });
            self.commit_state()?;
        }
        for attempt in 0..2 {
            let view = self
                .state
                .remote_view_tab_handoffs
                .get(handoff_id)
                .and_then(|record| record.view.clone())
                .ok_or("remote_view_tab_view_missing")?;
            self.effects.begin_operation(&self.persisted_state)?;
            let issuance = match self
                .effects
                .resolve_remote_view_tab_view(&target, &view, now_ms)
            {
                Ok(issuance) => issuance,
                Err(error) if error == "remote_view_tab_view_grant_terminal" && attempt == 0 => {
                    // A qualified retained expiry or durable provider terminal
                    // proof permits one replacement. Preserve issuance history
                    // and the durable handoff identity.
                    self.state
                        .remote_view_tab_handoffs
                        .get_mut(handoff_id)
                        .ok_or("remote_view_tab_handoff_missing")?
                        .view = Some(agent_browser_service_model::RemoteViewTabView {
                        idempotency_key: uuid::Uuid::new_v4().to_string(),
                        issuance: None,
                    });
                    self.commit_state()?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if view.issuance.as_ref() != Some(&issuance) {
                self.state
                    .remote_view_tab_handoffs
                    .get_mut(handoff_id)
                    .and_then(|record| record.view.as_mut())
                    .ok_or("remote_view_tab_view_missing")?
                    .issuance = Some(issuance.clone());
                self.commit_state()?;
            }
            return Ok(issuance);
        }
        Err("remote_view_tab_view_grant_terminal".into())
    }

    /// Join the addressed foreground tab to the provider's current transport.
    /// Ready means a qualified presentation route; viewer pixels/input require
    /// independent live acceptance after opening the authenticated handoff.
    fn qualify_tab_handoff_presentation(
        &mut self,
        handoff_id: &str,
        issuance: &agent_browser_service_model::RemoteViewApplicationViewIssuance,
        now_ms: u64,
    ) -> Result<(), String> {
        let configured = std::env::var("AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN")
            .map_err(|_| "remote_view_public_origin_missing")?;
        let origin = super::browser_session_remote_view::public_origin(&configured)?;
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff_id)?;
        self.focus_browser(&target.browser.id, Some(&target.tab.target_id), now_ms)?;
        let current =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff_id)?;
        self.effects.begin_operation(&self.persisted_state)?;
        let observation =
            self.effects
                .observe_remote_view_tab_view(&current, issuance, current_unix_ms())?;
        if observation.state
            != agent_browser_service_model::RemoteViewApplicationViewReadiness::Ready
        {
            return Err("remote_view_tab_transport_pending".into());
        }
        let observed_origin = observation
            .public_origin
            .as_deref()
            .ok_or("remote_view_public_origin_missing")?;
        if super::browser_session_remote_view::public_origin(observed_origin)? != origin {
            return Err("remote_view_public_origin_mismatch".into());
        }
        // Check the exact owned foreground window again after provider transport.
        self.focus_browser(
            &current.browser.id,
            Some(&current.tab.target_id),
            current_unix_ms(),
        )?;
        issuance
            .grant
            .validate_target(
                &observation.grant.request.target,
                &observation.grant.request.application,
                "remote_view",
                current_unix_ms(),
            )
            .map_err(|_| "remote_view_tab_view_issuance_invalid")?;
        Ok(())
    }

    pub(crate) fn state(&self) -> &BrowserSessionState {
        &self.state
    }

    pub(crate) fn reap_current(&mut self) -> Result<ReapBrowserSessionsResult, String> {
        self.refresh_before_ordinary_command()?;
        self.reap(current_unix_ms())
    }

    pub(crate) fn handle_command(&mut self, command: &serde_json::Value) -> serde_json::Value
    where
        E: ManagedBrowserCommandEffects,
    {
        let id = command
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let admission = if command["action"]
            .as_str()
            .is_some_and(|action| action.starts_with("browser_session_"))
            && command["action"] != "browser_session_profile_duplicate"
            && command["action"] != "browser_session_mode_convert"
            && command["action"] != "browser_session_build_select"
            && !(command["action"] == "browser_session_close" && command["scope"] == "browser")
        {
            super::browser_host_control::admit_command(None, None, command, false)
        } else {
            Ok(None)
        };
        let admission = match admission {
            Ok(value) => value,
            Err(error) => return serde_json::json!({"id":id,"success":false,"error":error}),
        };
        let mut result = self.handle_command_result(command);
        if let Some(admission) = admission {
            if let Err(error) = admission.finish(result.is_ok()) {
                result = Err(error);
            }
        }
        match result {
            Ok(data) => serde_json::json!({ "id": id, "success": true, "data": data }),
            Err(error) => serde_json::json!({ "id": id, "success": false, "error": error }),
        }
    }

    fn handle_command_result(
        &mut self,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String>
    where
        E: ManagedBrowserCommandEffects,
    {
        let action = command
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "browser_session_action_missing".to_string())?;
        if let Some(readback) = self.operation_request_readback(command)? {
            return Ok(readback);
        }
        if action == "browser_session_open" {
            self.verify_build_open_selection(command, false)?;
        }
        if action == "browser_session_build_select" {
            return self.browser_build_select(command);
        }
        if action == "browser_session_mode_convert" {
            self.refresh_before_ordinary_command()?;
            return self.mode_convert(command);
        }
        if action == "browser_session_profile_duplicate" {
            self.refresh_before_ordinary_command()?;
            if let Some(task_id) =
                optional_string(command, "sessionId").or_else(|| optional_string(command, "taskId"))
            {
                let task = self
                    .state
                    .sessions
                    .get(task_id)
                    .ok_or("profile_duplicate_task_missing")?;
                if optional_string(command, "sourceProfileId") != Some(task.profile_id.as_str())
                    || optional_string(command, "sessionName").is_some_and(|name| name != task.name)
                {
                    return Err("profile_duplicate_task_profile_conflict".into());
                }
            }
            // The owner operation presents its affected tasks and explicit
            // consequence choices before a close. It never changes human control.
            return self.profile_duplicate(command);
        }
        super::browser_host_control::check_state(&self.state, "", command)?;
        if action.starts_with("browser_session_") && action != "browser_session_status" {
            self.refresh_before_ordinary_command()?;
        }
        if action == "browser_session_policy" {
            let result = super::browser_host_policy::apply(&mut self.state, command)?;
            self.commit_state()?;
            return Ok(result);
        }
        if action == "browser_session_reconnect" {
            let persistence = &mut self.persistence;
            let persisted = &mut self.persisted_state;
            let result = super::browser_host_reconnect::execute(
                &mut self.state,
                &self.catalog,
                &mut self.effects,
                command,
                &mut |expected, next, effects| {
                    let intent = effects.pending_launch_intent();
                    let retirement = effects.pending_idle_retirement_ids();
                    persistence.publish_session_effects(
                        expected,
                        next,
                        intent.as_ref(),
                        &retirement,
                        &effects.pending_browser_activity_ids(),
                    )?;
                    *persisted = persistence.load_session_state()?;
                    effects.acknowledge_launch_publication();
                    effects.finish_idle_retirement_publication(&retirement)?;
                    Ok(())
                },
            );
            // Failed effects may already have published a pending intent. Adopt it
            // without clearing or replaying the retained uncertainty.
            self.state = self.persistence.load_session_state()?;
            self.persisted_state = self.state.clone();
            return result;
        }
        super::browser_host_reconnect::check_state(&self.state, "", command)?;
        super::browser_host_policy::check_state(&self.state, "", command)?;
        if command
            .get("leaveOpen")
            .is_some_and(|value| !value.is_boolean())
        {
            return Err("browser_session_leave_open_invalid".into());
        }
        let now_ms = command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        match action {
            "service_remote_view_handoff_resolve" => {
                let handoff_id = required_string(command, "handoffId")?;
                if self.persistence.load_session_state()? != self.persisted_state {
                    return Err("browser_session_publication_conflict".into());
                }
                if command["handoffOperation"] == "delete" {
                    // Revoke only the durable link. Tab lifetime and pending-human
                    // retention belong to the target, independent of this record.
                    self.state
                        .remote_view_tab_handoffs
                        .remove(handoff_id)
                        .ok_or("remote_view_tab_handoff_missing")?;
                    self.commit_state()?;
                    return Ok(serde_json::json!({"handoffId":handoff_id,"status":"deleted"}));
                }
                let link = self
                    .state
                    .remote_view_tab_handoffs
                    .get(handoff_id)
                    .ok_or("remote_view_tab_handoff_missing")?;
                if command["handoffOperation"] == "inspect" {
                    let tab_lifetime = self.state.tab_lifetimes.get(&link.tab_id).or_else(|| {
                        link.resume
                            .as_ref()
                            .and_then(|intent| intent.tab_lifetime.as_ref())
                    });
                    let task_metadata = self.state.tab_metadata.get(&link.tab_id).or_else(|| {
                        link.resume
                            .as_ref()
                            .and_then(|intent| intent.task_metadata.as_ref())
                    });
                    return Ok(serde_json::json!({"handoffId":handoff_id,
                        "handoffUrl":format!("/remote-view/{handoff_id}"),
                        "createdAtMs":link.created_at_ms,"expiresAtMs":link.expires_at_ms,
                        "tabId":link.tab_id,"tabLifetime":tab_lifetime,"taskMetadata":task_metadata,
                        "pendingHuman":self.state.pending_human_tabs.get(&link.tab_id).map(|hold| serde_json::json!({
                            "awaitingUse":true,"requestedAtMs":hold.requested_at_ms,"expiresAtMs":hold.expires_at_ms,
                            "retained":hold.expires_at_ms.is_none_or(|deadline| now_ms < deadline),
                        })),
                        "expired":link.check_link_at(now_ms).is_err(),
                        "status":if link.check_link_at(now_ms).is_err() {"expired"} else {"active"}}));
                }
                link.check_link_at(now_ms)?;
                if command["handoffOperation"] == "prepare_human" {
                    let ttl = command["pendingHumanRetentionMs"]
                        .as_u64()
                        .ok_or("remote_view_pending_human_retention_invalid")?;
                    let expires_at_ms = if ttl == 0 {
                        None
                    } else {
                        Some(
                            now_ms
                                .checked_add(ttl)
                                .ok_or("remote_view_pending_human_retention_invalid")?,
                        )
                    };
                    let target = agent_browser_service_model::resolve_remote_view_tab_handoff(
                        &self.state,
                        handoff_id,
                    )?;
                    let hold = agent_browser_service_model::PendingHumanTabRetention {
                        handoff_id: handoff_id.to_string(),
                        requested_at_ms: now_ms,
                        expires_at_ms,
                    };
                    self.state
                        .pending_human_tabs
                        .insert(target.tab.id.clone(), hold.clone());
                    // A new hold requires a new native observation. Historical
                    // qualified use must not release another handoff attempt.
                    for link in self
                        .state
                        .remote_view_tab_handoffs
                        .values_mut()
                        .filter(|link| {
                            link.tab_id == target.tab.id && link.session_id == target.session.id
                        })
                    {
                        link.native_observation = None;
                    }
                    self.commit_state()?;
                    return Ok(
                        serde_json::json!({"handoffId":handoff_id,"tabId":target.tab.id,
                        "status":"pending_human","pendingHuman":hold}),
                    );
                }
                if command["handoffOperation"] == "prepare_native_use" {
                    let target = agent_browser_service_model::resolve_remote_view_tab_handoff(
                        &self.state,
                        handoff_id,
                    )?;
                    if command["expectedTabId"] != target.tab.id
                        || command["expectedTargetId"] != target.tab.target_id
                    {
                        return Err("remote_view_human_use_target_changed".into());
                    }
                    return self.prepare_native_handoff_observation(handoff_id, now_ms);
                }
                if command["handoffOperation"] == "confirm_human_use"
                    || command["handoffOperation"] == "observe_native_use"
                {
                    if command.get("humanUseEvidence").is_some() {
                        return Err("remote_view_human_use_evidence_invalid".into());
                    }
                    return self.consume_native_handoff_use(handoff_id, now_ms);
                }
                if command["handoffOperation"] == "set_tab_lifetime" {
                    let ttl = command["tabInactivityMs"]
                        .as_u64()
                        .ok_or("browser_tab_lifetime_invalid")?;
                    let target = agent_browser_service_model::resolve_remote_view_tab_handoff(
                        &self.state,
                        handoff_id,
                    )?;
                    target
                        .tab
                        .last_activity_at_ms
                        .checked_add(ttl)
                        .ok_or("browser_tab_lifetime_invalid")?;
                    let pressure_inactivity_ms = match command.get("pressureInactivityMs") {
                        Some(value) => {
                            let duration = value
                                .as_u64()
                                .ok_or("browser_tab_pressure_lifetime_invalid")?;
                            target
                                .tab
                                .last_activity_at_ms
                                .checked_add(duration)
                                .ok_or("browser_tab_pressure_lifetime_invalid")?;
                            (duration != 0).then_some(duration)
                        }
                        None => self
                            .state
                            .tab_lifetimes
                            .get(&target.tab.id)
                            .and_then(|lifetime| lifetime.pressure_inactivity_ms),
                    };
                    let lifetime = agent_browser_service_model::BrowserTabLifetime {
                        inactivity_ms: (ttl != 0).then_some(ttl),
                        pressure_inactivity_ms,
                    };
                    let mut intent =
                        agent_browser_service_model::remote_view_handoff_resume_intent(
                            &self.state,
                            handoff_id,
                            now_ms,
                        )?;
                    intent.tab_lifetime = Some(lifetime.clone());
                    self.state
                        .tab_lifetimes
                        .insert(target.tab.id.clone(), lifetime.clone());
                    for link in self
                        .state
                        .remote_view_tab_handoffs
                        .values_mut()
                        .filter(|link| link.tab_id == target.tab.id)
                    {
                        link.resume = Some(intent.clone());
                    }
                    self.commit_state()?;
                    return Ok(serde_json::json!({"handoffId":handoff_id,
                        "tabId":target.tab.id,"status":"tab_lifetime_updated",
                        "tabLifetime":lifetime}));
                }
                if command["handoffOperation"] == "extend"
                    || command["handoffOperation"] == "set_link_lifetime"
                {
                    // Link lifetime management does not require a live target
                    // and must not recreate one as a side effect.
                    let ttl = command["ttlMs"]
                        .as_u64()
                        .ok_or("remote_view_handoff_ttl_invalid")?;
                    let link = self
                        .state
                        .remote_view_tab_handoffs
                        .get_mut(handoff_id)
                        .ok_or("remote_view_tab_handoff_missing")?;
                    let status = if command["handoffOperation"] == "extend" {
                        link.extend_link_retention(now_ms, ttl)?;
                        "link_extended"
                    } else {
                        link.set_link_lifetime(now_ms, ttl)?;
                        "link_lifetime_updated"
                    };
                    let response = serde_json::json!({"handoffId":handoff_id,
                        "handoffUrl":format!("/remote-view/{handoff_id}"),
                        "status":status,"createdAtMs":link.created_at_ms,
                        "expiresAtMs":link.expires_at_ms});
                    self.commit_state()?;
                    return Ok(response);
                }
                self.recreate_expired_handoff(handoff_id, now_ms)?;
                if command["nativeDesktop"] == true {
                    if self.persistence.load_session_state()? != self.persisted_state {
                        return Err("browser_session_publication_conflict".into());
                    }
                    let retained = agent_browser_service_model::resolve_remote_view_tab_handoff(
                        &self.state,
                        handoff_id,
                    )?;
                    self.recover_browser_for_command(&retained.browser.id, command)?;
                    let target = agent_browser_service_model::resolve_remote_view_tab_handoff(
                        &self.state,
                        handoff_id,
                    )?;
                    self.focus_browser(&target.browser.id, Some(&target.tab.target_id), now_ms)?;
                    return Ok(serde_json::json!({
                        "handoffId":handoff_id, "status":"opened",
                        "createdAtMs":self.state.remote_view_tab_handoffs[handoff_id].created_at_ms,
                        "expiresAtMs":self.state.remote_view_tab_handoffs[handoff_id].expires_at_ms,
                        "browserId":target.browser.id, "sessionName":target.session.name,
                        "tabId":target.tab.id, "targetId":target.tab.target_id,
                        "operatorVisible":{"state":"ready","proofScope":"owned_browser_window"},
                    }));
                }
                let view = self.resolve_tab_handoff_view(handoff_id, now_ms)?;
                let qualification =
                    self.qualify_tab_handoff_presentation(handoff_id, &view, now_ms);
                let operator_visible = match qualification {
                    Ok(()) => {
                        serde_json::json!({"state":"ready","proofScope":"addressed_tab_owned_window_and_installed_transport"})
                    }
                    Err(reason) => serde_json::json!({"state":"pending","reason":reason}),
                };
                let ready = operator_visible["state"] == "ready";
                let target = agent_browser_service_model::resolve_remote_view_tab_handoff(
                    &self.state,
                    handoff_id,
                )?;
                Ok(serde_json::json!({
                    "handoffId": handoff_id,
                    "status": if ready { "opened" } else { "converging" },
                    "presentationState": view.presentation_state,
                    "readinessScope": view.readiness_scope,
                    "browserId": target.browser.id,
                    "sessionId": target.session.id,
                    "sessionName": target.session.name,
                    "tabId": target.tab.id,
                    "targetId": target.tab.target_id,
                    "operatorVisible": operator_visible,
                }))
            }
            "browser_session_open" => {
                let opened = self.open_request_from_command(command, now_ms)?;
                Ok(serde_json::json!({
                    "sessionId": opened.session_id,
                    "sessionName": opened.session_name,
                    "profileId": opened.profile_id,
                    "browserId": opened.browser_id,
                    "browserDisposition": format!("{:?}", opened.disposition).to_lowercase(),
                    "sessionDisposition": format!("{:?}", opened.session_disposition).to_lowercase(),
                }))
            }
            "browser_session_retention" => {
                let fields = [
                    "operation",
                    "claimId",
                    "ownerId",
                    "sessionId",
                    "browserId",
                    "browserPid",
                    "ttlMs",
                    "generation",
                ];
                let value = serde_json::Value::Object(
                    fields
                        .into_iter()
                        .filter_map(|key| {
                            command
                                .get(key)
                                .map(|value| (key.to_string(), value.clone()))
                        })
                        .collect(),
                );
                let request = serde_json::from_value::<
                    agent_browser_service_model::BrowserSessionRetentionRequest,
                >(value)
                .map_err(|_| "browser_retention_request_invalid")?;
                // Retention uses owner time, never caller-supplied activity timestamps.
                let receipt = self.manager()?.retain_session(request, current_unix_ms())?;
                self.commit_state()?;
                serde_json::to_value(receipt)
                    .map_err(|_| "browser_retention_receipt_invalid".into())
            }
            "browser_session_close" => {
                let session_id = self.session_id_from_command(command)?;
                let scope = command["scope"].as_str().unwrap_or("task");
                if scope == "browser" {
                    let before = self.state.clone();
                    let mut admission = None;
                    let result = super::browser_host_policy::close_browser_scope(
                        &before,
                        &session_id,
                        command,
                        || {
                            // Only admitted effects enter the flight journal. Known
                            // scope/attribution refusals remain undispatched.
                            admission = super::browser_host_control::admit_command(
                                None, None, command, false,
                            )?;
                            let closed = self.close_session(
                                &session_id,
                                SessionEndReason::ExplicitClose,
                                now_ms,
                            )?;
                            Ok(
                                serde_json::json!({"closed":true,"sessionId":closed.session_id,"browserId":closed.browser_id,
                            "disposition":session_close_disposition_name(closed.disposition)}),
                            )
                        },
                    );
                    if let Some(admission) = admission {
                        admission.finish(result.is_ok())?;
                    }
                    return result;
                }
                if scope != "task" {
                    return Err("browser_lifecycle_scope_invalid".into());
                }
                let preserve = match command.get("leaveOpen") {
                    Some(value) => value
                        .as_bool()
                        .ok_or("browser_session_leave_open_invalid")?,
                    None => self.state.leave_open_session_ids.contains(&session_id),
                };
                let closed = if preserve {
                    let result = self.manager()?.preserve_session(&session_id)?;
                    self.commit_state()?;
                    result
                } else {
                    self.close_session(&session_id, SessionEndReason::ExplicitClose, now_ms)?
                };
                Ok(serde_json::json!({
                    "closed": true,
                    "sessionId": closed.session_id,
                    "browserId": closed.browser_id,
                    "disposition": session_close_disposition_name(closed.disposition),
                    "taskRetained": preserve,
                }))
            }
            "browser_session_navigate" => {
                let opened = if optional_string(command, "sessionId").is_none() {
                    Some(self.open_request_from_command(command, now_ms)?)
                } else {
                    None
                };
                let session_id = optional_string(command, "sessionId")
                    .map(str::to_string)
                    .or_else(|| opened.as_ref().map(|result| result.session_id.clone()))
                    .ok_or_else(|| "browser_session_field_missing:sessionId".to_string())?;
                let url = required_string(command, "url")?;
                let navigation = self.navigate(&session_id, url, now_ms)?;
                Ok(serde_json::json!({
                    "sessionId": session_id,
                    "profileId": navigation.profile_id,
                    "browserId": navigation.browser_id,
                    "tabId": navigation.tab_id,
                    "targetId": navigation.target_id,
                    "url": navigation.url,
                    "visitedAtMs": navigation.visited_at_ms,
                }))
            }
            "browser_session_tab_new" => {
                let session_id = self.session_id_from_command(command)?;
                let tab = self.new_tab(&session_id, now_ms)?;
                let navigation = optional_string(command, "url")
                    .map(|url| self.navigate(&session_id, url, now_ms))
                    .transpose()?;
                Ok(serde_json::json!({
                    "sessionId": session_id,
                    "tabId": tab.tab_id,
                    "targetId": tab.target_id,
                    "source": "explicit_new",
                    "url": navigation.map(|record| record.url),
                }))
            }
            "browser_session_tab_close" => {
                let session_id = self.session_id_from_command(command)?;
                // A task client can bind its earlier selection across commands.
                // Reject a changed current tab before any physical close effect.
                if command.get("expectedTabId").is_some() {
                    let expected = required_string(command, "expectedTabId")?;
                    let current = self
                        .state
                        .sessions
                        .get(&session_id)
                        .and_then(|session| session.current_tab_id.as_deref());
                    if current != Some(expected) {
                        return Err("browser_session_tab_close_selection_changed".into());
                    }
                }
                let closed = self.close_current_tab(&session_id, now_ms)?;
                Ok(serde_json::json!({
                    "sessionId": session_id,
                    "closed": true,
                    "closedTabId": closed.closed_tab_id,
                    "currentTabId": closed.current_tab_id,
                }))
            }
            "browser_session_focus" => {
                let browser_id = required_string(command, "browserId")?;
                let focused =
                    self.focus_browser(browser_id, optional_string(command, "targetId"), now_ms)?;
                Ok(serde_json::json!({
                    "browserId": focused.browser_id,
                    "tabId": focused.tab_id,
                    "targetId": focused.target_id,
                    "focused": true,
                    "maximized": true,
                }))
            }
            "browser_session_reap" => {
                let reaped = self.reap(now_ms)?;
                Ok(serde_json::json!({
                    "expiredSessionIds": reaped.expired_session_ids,
                    "expiredTabIds": reaped.expired_tab_ids,
                    "quotaEvictedSessionIds": reaped.quota_evicted_session_ids,
                    "closedBrowserIds": reaped.closed_browser_ids,
                    "deletedDisposableProfileIds": reaped.deleted_disposable_profile_ids,
                    "deferredBrowserErrors": reaped.deferred_browser_errors,
                }))
            }
            "browser_session_status" => {
                let mut status = serde_json::to_value(self.state())
                    .map_err(|error| format!("browser_session_status_serialize_failed:{error}"))?;
                // Fingerprints belong to private persistence, never raw status.
                if let Some(object) = status.as_object_mut() {
                    object.remove("taskOperations");
                    object.remove("adoptedBrowserProcessIdentities");
                }
                Ok(status)
            }
            _ => Err(format!("browser_session_action_unsupported:{action}")),
        }
    }

    /// Start a new ordinary command from current durable state. This is not an
    /// effect retry: keep the adapter and all of its original-operation journals.
    /// Unpublished local state or launch/retirement work cannot be discarded.
    fn refresh_before_ordinary_command(&mut self) -> Result<(), String> {
        let (current, catalog) = self.persistence.load_owner_snapshot()?;
        if current == self.persisted_state {
            self.catalog = catalog;
            return Ok(());
        }
        if self.state != self.persisted_state
            || self.effects.pending_launch_intent().is_some()
            || !self.effects.pending_idle_retirement_ids().is_empty()
        {
            return Err("browser_session_publication_conflict".into());
        }
        current.retention_policy.validate()?;
        self.catalog = catalog;
        self.state = current.clone();
        self.persisted_state = current;
        Ok(())
    }

    fn borrowed_retirement_manager(&mut self) -> Result<BrowserSessionManager<'_, E>, String> {
        if self.persistence.load_session_state()? != self.persisted_state {
            return Err("browser_session_publication_conflict".into());
        }
        self.effects
            .begin_borrowed_retirement_operation(&self.persisted_state)?;
        Ok(BrowserSessionManager::new(
            &mut self.state,
            &self.catalog,
            &mut self.effects,
            self.manager_config.clone(),
        ))
    }

    fn manager(&mut self) -> Result<BrowserSessionManager<'_, E>, String> {
        if self.persistence.load_session_state()? != self.persisted_state {
            return Err("browser_session_publication_conflict".into());
        }
        self.effects.begin_operation(&self.persisted_state)?;
        Ok(BrowserSessionManager::new(
            &mut self.state,
            &self.catalog,
            &mut self.effects,
            self.manager_config.clone(),
        ))
    }

    /// Host-selected loss policy never permits ordinary dispatch to relaunch.
    /// Explicit reconnect owns recovery and retains uncertain effects separately.
    fn require_explicit_hosted_recovery(&mut self, browser_id: &str) -> Result<(), String> {
        let hosted = self.state.host_bindings.keys().any(|id| {
            self.state
                .sessions
                .get(id)
                .is_some_and(|task| task.browser_id == browser_id)
        });
        if hosted {
            let browser = self
                .state
                .browsers
                .get(browser_id)
                .ok_or("browser_session_browser_missing")?;
            if !self.effects.browser_is_live(browser)? {
                return Err("browser_reconnect_explicit_required: observe loss and follow the app's report-only or explicit reopen policy".into());
            }
        }
        Ok(())
    }

    /// Attach or recover the exact recorded browser through its per-owner
    /// publication fence, independently of legacy installer metadata.
    fn recover_browser_for_command(
        &mut self,
        browser_id: &str,
        command: &serde_json::Value,
    ) -> Result<(), String> {
        // Use the same publication and operation fence as manager recovery.
        let _ = self.manager()?;
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .ok_or("browser_session_browser_missing")?
            .clone();
        if self.effects.browser_is_live(&browser)? {
            return Ok(());
        }
        if command["retainedTaskOnly"] == true {
            return Err("browser_task_transient_state_lost: observe current task and choose explicit recovery; unsaved page state is not reconstructed".into());
        }
        // Explicit return observation must never recover a different physical
        // browser, including while atomic return still retains human ownership.
        if (command["action"] == "url" && command["hostControlFreshObservation"] == true)
            || super::browser_host_control::read(browser_id)?.awaiting_observation
        {
            return Err("browser_control_fresh_observation_cannot_recover_browser".into());
        }
        self.require_explicit_hosted_recovery(browser_id)?;
        self.manager()?.recover_retained_browser(browser_id)?;
        self.commit_state()
    }

    fn compact_navigation_history(&mut self) -> Result<(), String> {
        self.state
            .compact_navigation_history(self.exact_url_history_maximum_bytes)?;
        Ok(())
    }

    fn commit_state(&mut self) -> Result<(), String> {
        self.state
            .host_policies
            .retain_active_tasks(&self.state.sessions.keys().cloned().collect());
        let intent = self.effects.pending_launch_intent();
        let retirement_ids = self.effects.pending_idle_retirement_ids();
        self.persistence.publish_session_effects(
            &self.persisted_state,
            &self.state,
            intent.as_ref(),
            &retirement_ids,
            &self.effects.pending_browser_activity_ids(),
        )?;
        // Publication may move completed history into the same durable archive.
        // Adopt its exact active projection so the next CAS sees the same state.
        self.state = self.persistence.load_session_state()?;
        self.persisted_state = self.state.clone();
        self.effects.acknowledge_launch_publication();
        self.effects
            .finish_idle_retirement_publication(&retirement_ids)?;
        if intent.is_some() {
            super::browser_session_remote_view::request_pool_maintenance();
        }
        Ok(())
    }

    /// Register an explicitly selected profile path using the same identity
    /// checks for ordinary commands and remote-view opens.
    fn register_explicit_profile(&mut self, command: &serde_json::Value) -> Result<(), String> {
        if let (Some(id), Some(path)) = (
            optional_string(command, "runtimeProfile"),
            optional_string(command, "profile"),
        ) {
            if let Some(selector) = command.get("profileId") {
                let selector = selector
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or("browser_session_profile_selector_invalid")?;
                if selector != id {
                    return Err("browser_session_profile_selector_conflict".into());
                }
            }
            let path = Path::new(path);
            if !path.is_absolute() || !path.is_dir() {
                return Err("browser_session_explicit_profile_path_invalid".into());
            }
            let path = path
                .canonicalize()
                .map_err(|_| "browser_session_explicit_profile_path_invalid")?;
            let path = path.to_string_lossy().into_owned();
            if let Some(existing) = self.catalog.profiles.get(id) {
                if existing.user_data_dir != path {
                    return Err("browser_session_profile_selector_conflict".into());
                }
            } else {
                if self
                    .catalog
                    .profiles
                    .values()
                    .any(|profile| profile.user_data_dir == path)
                {
                    return Err("browser_session_profile_selector_conflict".into());
                }
                let original_catalog = self.catalog.clone();
                self.catalog.profiles.insert(
                    id.into(),
                    agent_browser_service_model::BrowserProfileCatalogEntry {
                        manual_login_url: None,
                        id: id.into(),
                        name: id.into(),
                        user_data_dir: path,
                        kind: agent_browser_service_model::BrowserProfileKind::Named,
                    },
                );
                self.persistence
                    .publish_profile_catalog(&original_catalog, &self.catalog)?;
            }
        }
        Ok(())
    }

    /// Launch posture belongs to the profile's current process. Never change
    /// it underneath another task or silently attach a debugging client.
    fn configure_profile_login(&mut self, command: &serde_json::Value) -> Result<(), String> {
        let explicit_manual = match command.get("manualLogin") {
            Some(serde_json::Value::Bool(value)) => Some(*value),
            Some(_) => return Err("browser_login_mode_invalid".into()),
            None => None,
        };
        let id = optional_string(command, "profileId")
            .or_else(|| optional_string(command, "runtimeProfile"));
        let Some(id) = id else {
            if explicit_manual == Some(true) {
                if !self.effects.admits_ordinary_sessions() {
                    return Err("manual_login_requires_native_remote_desktop".into());
                }
                reqwest::Url::parse(required_string(command, "url")?)
                    .map_err(|_| "manual_login_url_invalid")?;
            }
            return Ok(());
        };
        let profile = self
            .catalog
            .profiles
            .get(id)
            .ok_or("browser_session_profile_selector_unknown")?;
        let manual = explicit_manual.unwrap_or(profile.manual_login_url.is_some());
        if manual && command.get("attachment").is_some() {
            return Err("manual_login_requires_owned_browser".into());
        }
        if manual && !self.effects.admits_ordinary_sessions() {
            return Err("manual_login_requires_native_remote_desktop".into());
        }
        let requested_url = optional_string(command, "url").or_else(|| {
            if explicit_manual.is_none() {
                profile.manual_login_url.as_deref()
            } else {
                None
            }
        });
        if manual {
            reqwest::Url::parse(requested_url.ok_or("manual_login_url_required")?)
                .map_err(|_| "manual_login_url_invalid")?;
        }
        let session_name = required_string(command, "sessionName")?;
        for browser in self
            .state
            .browsers
            .values()
            .filter(|browser| browser.profile_id == id)
        {
            if browser.cdp_endpoint.is_empty() != manual {
                return Err("browser_login_mode_conflict: close the current profile task before changing login mode".into());
            }
            if manual
                && browser.active_session_ids.iter().any(|sid| {
                    self.state
                        .sessions
                        .get(sid)
                        .is_some_and(|session| session.name != session_name)
                })
            {
                return Err("manual_login_profile_in_use: reuse the original task key or close that login task".into());
            }
        }
        let url = if manual {
            // Replays retain the initial requested URL, not a claim about the
            // human's current location. They must not navigate the login again.
            profile
                .manual_login_url
                .clone()
                .filter(|_| {
                    self.state
                        .browsers
                        .values()
                        .any(|browser| browser.profile_id == id)
                })
                .or_else(|| requested_url.map(str::to_owned))
        } else {
            None
        };
        if profile.manual_login_url == url {
            return Ok(());
        }
        let expected = self.catalog.clone();
        let mut next = expected.clone();
        next.profiles
            .get_mut(id)
            .ok_or("browser_session_profile_selector_unknown")?
            .manual_login_url = url;
        self.persistence.publish_profile_catalog(&expected, &next)?;
        self.catalog = next;
        Ok(())
    }

    fn open_request_from_command(
        &mut self,
        command: &serde_json::Value,
        activity_at_ms: u64,
    ) -> Result<OpenBrowserSessionResult, String> {
        self.verify_build_open_selection(command, false)?;
        if command.get("attachment").is_some()
            && command
                .get("manualLogin")
                .is_some_and(|value| value != false)
        {
            return Err("manual_login_requires_owned_browser".into());
        }
        let hosted = command
            .get("hostBinding")
            .map(|value| {
                serde_json::from_value::<agent_browser_service_model::BrowserHostBinding>(
                    value.clone(),
                )
                .map_err(|_| "browser_host_binding_invalid".to_string())
            })
            .transpose()?;
        if let Some(binding) = &hosted {
            super::browser_host_context::validate_provider(binding)?;
            let host = super::browser_host_context::configured_host(binding)?;
            super::browser_host_policy::check_open(
                &self.state,
                binding,
                host.policy_defaults.as_ref(),
                command,
            )?;
        } else if command.get("taskPolicy").is_some() {
            return Err("browser_policy_hosted_task_required".into());
        }
        if let Some(attachment) = command.get("attachment") {
            let binding = hosted
                .as_ref()
                .ok_or("browser_attachment_host_binding_required")?;
            let request: agent_browser_service_model::BrowserAttachmentRequest =
                serde_json::from_value(attachment.clone())
                    .map_err(|_| "browser_attachment_invalid")?;
            request.validate()?;
            let profile_id = required_string(command, "profileId")?;
            let canonical = Path::new(&request.profile_path)
                .canonicalize()
                .map_err(|_| "browser_attachment_profile_unavailable")?;
            if canonical.to_str() != Some(request.profile_path.as_str()) {
                return Err("browser_attachment_profile_path_not_canonical".into());
            }
            if self
                .catalog
                .profiles
                .values()
                .any(|row| row.user_data_dir == request.profile_path && row.id != profile_id)
            {
                return Err(
                    "browser_attachment_profile_conflict: reuse the registered profile name".into(),
                );
            }
            let mut profile = self
                .catalog
                .profiles
                .get(profile_id)
                .cloned()
                .unwrap_or_else(|| agent_browser_service_model::BrowserProfileCatalogEntry {
                    manual_login_url: None,
                    id: profile_id.into(),
                    name: profile_id.into(),
                    user_data_dir: request.profile_path.clone(),
                    kind: agent_browser_service_model::BrowserProfileKind::Named,
                });
            if profile.user_data_dir != request.profile_path {
                return Err(
                    "browser_attachment_profile_conflict: retain the existing profile path".into(),
                );
            }
            // A successfully observed CDP attachment is explicit automation.
            // Clear any prior, closed manual process's launch posture only in
            // the existing atomic attachment/catalog publication below.
            profile.manual_login_url = None;
            self.effects.begin_operation(&self.persisted_state)?;
            let result = agent_browser_service_model::attach_app_browser_task(
                &mut self.state,
                &mut self.effects,
                &profile,
                required_string(command, "sessionName")?,
                binding,
                &request,
                activity_at_ms,
                self.manager_config.session_idle_timeout_ms,
            )?;
            if let Some(tab) = self
                .state
                .sessions
                .get(&result.session_id)
                .and_then(|task| task.current_tab_id.as_ref())
                .and_then(|id| self.state.tabs.get(id))
                .cloned()
            {
                if !self.state.tab_metadata.contains_key(&tab.id) {
                    self.state
                        .tab_metadata
                        .insert(tab.id.clone(), capture_browser_task_metadata(command, &tab));
                }
            }
            if let Some(attached_binding) =
                self.state.host_bindings.get(&result.session_id).cloned()
            {
                capture_host_policy_defaults(
                    &mut self.state,
                    &attached_binding.request,
                    &result.session_id,
                )?;
            }
            capture_task_policy(&mut self.state, &result.session_id, command)?;
            let mut next_catalog = self.catalog.clone();
            next_catalog.profiles.insert(profile.id.clone(), profile);
            // Publication must not expose an app profile before its borrowed
            // lifecycle fence. A failed transaction leaves both unchanged.
            if let Err(error) = self.persistence.publish_app_attachment(
                &self.persisted_state,
                &self.state,
                &self.catalog,
                &next_catalog,
            ) {
                self.state = self.persisted_state.clone();
                return Err(error);
            }
            let (published, catalog) = self.persistence.load_owner_snapshot()?;
            self.state = published.clone();
            self.persisted_state = published;
            self.catalog = catalog;
            return Ok(result);
        }
        self.register_explicit_profile(command)?;
        self.configure_profile_login(command)?;
        let session_name = required_string(command, "sessionName")?;
        if let Some(binding) = &hosted {
            for session in self
                .state
                .sessions
                .values()
                .filter(|session| session.name == session_name)
            {
                if let Some(current) = self.state.host_bindings.get(&session.id) {
                    if current.request != *binding {
                        return Err("browser_host_binding_changed: explicitly rebind the app before changing an existing task".into());
                    }
                }
            }
        }
        let request = if let Some(profile_id) = optional_string(command, "profileId") {
            OpenBrowserSession::exact_profile(session_name, profile_id, activity_at_ms)
        } else {
            let request = OpenBrowserSession::disposable(
                session_name,
                optional_string(command, "disposablePolicyId")
                    .unwrap_or(DEFAULT_DISPOSABLE_POLICY_ID),
                activity_at_ms,
            );
            if command.get("manualLogin").and_then(serde_json::Value::as_bool) == Some(true) {
                request.with_manual_login_url(required_string(command, "url")?)
            } else {
                request
            }
        };
        let request = if let Some(binding) = &hosted {
            request.with_placement(binding.placement.clone())
        } else {
            request
        };
        let opened =
            self.open_with_host_binding_policy(request, hosted, command.get("taskPolicy"))?;
        self.apply_leave_open_choice(&opened.session_id, command)?;
        self.verify_build_open_selection(command, true)?;
        Ok(opened)
    }

    /// Retain managed task identity and physical occupancy rather than claiming
    /// that an externally consumed browser's profile became available.
    fn apply_leave_open_choice(
        &mut self,
        session_id: &str,
        command: &serde_json::Value,
    ) -> Result<(), String> {
        match command.get("leaveOpen") {
            Some(serde_json::Value::Bool(true)) => {
                self.manager()?.preserve_session(session_id)?;
                self.commit_state()?;
            }
            Some(serde_json::Value::Bool(false)) => {
                self.manager()?;
                self.state.leave_open_session_ids.remove(session_id);
                self.commit_state()?;
            }
            Some(_) => return Err("browser_session_leave_open_invalid".into()),
            None => {}
        }
        Ok(())
    }

    fn session_id_from_command(&self, command: &serde_json::Value) -> Result<String, String> {
        if let Some(session_id) = optional_string(command, "sessionId") {
            return Ok(session_id.to_string());
        }
        let session_name = required_string(command, "sessionName")?;
        let profile_id = optional_string(command, "profileId");
        let matching_ids: Vec<&str> = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.name == session_name
                    && profile_id.is_none_or(|profile_id| session.profile_id == profile_id)
            })
            .map(|session| session.id.as_str())
            .collect();
        match matching_ids.as_slice() {
            [session_id] => Ok((*session_id).to_string()),
            [] => Err(format!("browser_session_not_found_by_name:{session_name}")),
            _ => Err(format!("browser_session_name_ambiguous:{session_name}")),
        }
    }
}

impl<P, E> BrowserSessionHost<P, E>
where
    P: BrowserSessionPersistence,
    E: BrowserSessionEffects + ManagedBrowserCommandEffects,
{
    /// Recreate only an automatically expired addressed task. Freeze the request
    /// before effects and use the ordinary no-replay tab creation protocol.
    fn recreate_expired_handoff(&mut self, handoff_id: &str, now_ms: u64) -> Result<(), String> {
        if agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff_id)
            .is_ok()
        {
            return Ok(());
        }
        let intent = agent_browser_service_model::remote_view_handoff_resume_intent(
            &self.state,
            handoff_id,
            now_ms,
        )?;
        let mut link = self
            .state
            .remote_view_tab_handoffs
            .get(handoff_id)
            .ok_or("remote_view_tab_handoff_missing")?
            .clone();
        if link.recreation.is_none() {
            link.resume = Some(intent.clone());
            link.recreation = Some(
                agent_browser_service_model::RemoteViewTabHandoffRecreation {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    requested_at_ms: now_ms,
                },
            );
            self.state
                .remote_view_tab_handoffs
                .insert(handoff_id.to_owned(), link.clone());
            self.commit_state()?;
        }
        let pending = link
            .recreation
            .ok_or("remote_view_handoff_recreation_missing")?;
        let mut request = serde_json::json!({
            "id":pending.request_id, "tabRequestId":pending.request_id,
            "action":"tab_new", "activityAtMs":pending.requested_at_ms,
            "resumeHandoffId":handoff_id,
        });
        if self.catalog.profiles.contains_key(&intent.profile_id)
            || self
                .state
                .disposable_profiles
                .contains_key(&intent.profile_id)
        {
            request["profileId"] = serde_json::json!(intent.profile_id);
        } else if let Some(policy) = intent.disposable_policy_id {
            request["disposablePolicyId"] = serde_json::json!(policy);
        } else {
            return Err("remote_view_handoff_resume_profile_unavailable".into());
        }
        if let Some(url) = intent.url {
            request["url"] = serde_json::json!(url);
        }
        let response = self.execute_managed_tab_new(&intent.session_name, &request)?;
        if response["success"] != true {
            return Err("remote_view_handoff_recreation_navigation_failed".into());
        }
        Ok(())
    }

    /// Admit ordinary requests through the configured Remote View owner, or
    /// route to an existing managed session. Only unconfigured, unowned lanes
    /// return None for legacy routing. Retained idle browsers recover through
    /// the existing fenced owner before refreshing or executing a tab command.
    /// Explicit humanHandoff commands prepare a configured pending tab hold;
    /// AGENT_BROWSER_PENDING_HUMAN_RETENTION_MS supplies the default maximum.
    /// Prepare a durable native handoff for an existing task without a URL,
    /// navigation, new task admission, or replacement of original provenance.
    fn prepare_existing_task_handoff(
        &mut self,
        name: &str,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let now_ms = current_unix_ms();
        let mut focus = command.clone();
        focus["action"] = serde_json::json!("view_focus");
        focus["operatorFocus"] = serde_json::json!(true);
        use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
        super::stream::verify_operator_focus(
            &LockedServiceStateRepository::default_json()?.load_snapshot()?,
            &focus,
        )?;
        let browser_id = required_string(command, "browserId")?;
        let target_id = required_string(command, "targetId")?;
        let tab = self
            .state
            .tabs
            .values()
            .find(|tab| tab.browser_id == browser_id && tab.target_id == target_id)
            .ok_or("operator_focus_target_missing")?
            .clone();
        let session = self
            .state
            .sessions
            .get(&tab.session_id)
            .ok_or("operator_focus_session_missing")?
            .clone();
        if session.name != name {
            return Err("operator_focus_binding_changed".into());
        }
        validate_handoff_ttl(command, now_ms, &self.state.retention_policy)?;
        let ttl = pending_human_retention(command)?;
        let id = self
            .state
            .remote_view_tab_handoffs
            .values()
            .find(|link| {
                link.session_id == session.id
                    && link.tab_id == tab.id
                    && link.check_link_at(now_ms).is_ok()
            })
            .map(|link| link.id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut link = agent_browser_service_model::retain_remote_view_tab_handoff_at(
            &mut self.state,
            &id,
            &session.id,
            &tab.id,
            now_ms,
        )?;
        if let Some(ttl) = command
            .get("handoffTtlMs")
            .and_then(serde_json::Value::as_u64)
        {
            link.extend_link_retention(now_ms, ttl)?;
            self.state
                .remote_view_tab_handoffs
                .insert(id.clone(), link.clone());
        }
        self.commit_state()?;
        let unused = self
            .state
            .tab_metadata
            .get(&tab.id)
            .is_none_or(|metadata| metadata.first_human_use_at_ms.is_none());
        if unused && !self.state.pending_human_tabs.contains_key(&tab.id) {
            if let Some(ttl) = ttl {
                self.handle_command_result(&serde_json::json!({"action":"service_remote_view_handoff_resolve",
                    "handoffId":id,"handoffOperation":"prepare_human","pendingHumanRetentionMs":ttl,"activityAtMs":now_ms}))?;
            }
        }
        self.prepare_native_handoff_observation(&id, now_ms)?;
        let selected = self.execute_operator_focus(&focus)?;
        if selected["success"] != true {
            return Ok(selected);
        }
        Ok(serde_json::json!({"id":command["id"],"success":true,
            "data":{"handoffPrepared":true,"tabNavigation":false},
            "browserSession":{"browserId":browser_id,"profileId":session.profile_id,
                "sessionId":session.id,"sessionName":session.name,"tabId":tab.id,"targetId":tab.target_id,
                "handoffId":id,"handoffUrl":format!("/remote-view/{id}"),
                "handoffCreatedAtMs":link.created_at_ms,"handoffExpiresAtMs":link.expires_at_ms},
            "presentationState":"native_desktop","operatorVisible":{"state":"ready","proofScope":"owned_browser_window"}}))
    }

    /// Keep signed operator focus separate from agent child custody. The runtime
    /// rechecks the proof under the desktop effect fence before any tab switch.
    pub(crate) fn execute_operator_focus(
        &mut self,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if command["action"] != "view_focus" || command["operatorFocus"] != true {
            return Err("operator_focus_action_mismatch".into());
        }
        let browser_id = required_string(command, "browserId")?;
        let target_id = required_string(command, "targetId")?;
        let name = required_string(command, "sessionName")?;
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .ok_or("operator_focus_browser_missing")?
            .clone();
        let mut matching = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser_id && tab.target_id == target_id);
        let tab = matching
            .next()
            .ok_or("operator_focus_target_missing")?
            .clone();
        if matching.next().is_some() {
            return Err("operator_focus_target_ambiguous".into());
        }
        let session = self
            .state
            .sessions
            .get(&tab.session_id)
            .ok_or("operator_focus_session_missing")?
            .clone();
        if session.name != name
            || session.browser_id != browser.id
            || self.persistence.load_session_state()? != self.persisted_state
        {
            return Err("operator_focus_binding_changed".into());
        }
        self.effects.begin_operation(&self.persisted_state)?;
        // Restart keeps the owned process but drops its in-memory CDP manager.
        // Reattach by exact recorded identity without launching a replacement.
        if !self.effects.browser_is_live(&browser)? {
            return Err("operator_focus_browser_unavailable".into());
        }
        let response =
            self.effects
                .execute_command(&browser, &tab, &session.id, &session.name, command)?;
        if !self.effects.pending_browser_activity_ids().is_empty() {
            self.commit_state()?;
        }
        Ok(response)
    }

    pub(crate) fn execute_managed_command(
        &mut self,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        self.refresh_before_ordinary_command()?;
        let mut prepared = command.clone();
        super::browser_host_policy::prepare_command(&self.state, session_name, &mut prepared)?;
        let command = &prepared;
        let exact_task_id = self.exact_task_selector(session_name, command)?;
        super::browser_task_operations::check_expected(&self.state, command)?;
        for task in self
            .state
            .sessions
            .values()
            .filter(|task| exact_task_id.map_or(task.name == session_name, |id| task.id == id))
        {
            if let Some(operation) = self.state.profile_operation_ids.get(&task.profile_id) {
                return Err(format!("browser_profile_operation_pending:{operation}:inspect or cancel the owner operation before page actions"));
            }
        }

        super::browser_host_control::check_state(&self.state, session_name, command)?;
        super::browser_host_reconnect::check_state(&self.state, session_name, command)?;
        super::browser_host_policy::check_state(&self.state, session_name, command)?;
        if matches!(
            command["action"].as_str(),
            Some("native_input" | "native_capture")
        ) {
            return self.execute_native_task(session_name, command);
        }
        let pending_human_ttl = pending_human_retention(command)?;
        if command
            .get("leaveOpen")
            .is_some_and(|value| !value.is_boolean())
        {
            return Err("browser_session_leave_open_invalid".into());
        }
        validate_handoff_ttl(
            command,
            command
                .get("activityAtMs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_else(current_unix_ms),
            &self.state.retention_policy,
        )?;
        self.register_explicit_profile(command)?;
        let automatic = self.effects.admits_ordinary_sessions();
        // Existing managed tasks need the same owned, replayable tab operation
        // even when this host has no ordinary presentation provider configured.
        let existing_task = self
            .state
            .sessions
            .values()
            .any(|task| task.name == session_name);
        // Last-task closure must not turn a retained creation replay into a
        // fresh fallback launch. The existing validator owns identity conflicts,
        // uncertain outcomes and closed-target rejection.
        let retained_tab_request = command
            .get("tabRequestId")
            .or_else(|| command.get("id"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|id| self.state.managed_tab_requests.contains_key(id));
        if (automatic || existing_task || retained_tab_request) && command["action"] == "tab_new" {
            return self
                .execute_managed_tab_new(session_name, command)
                .map(Some);
        }
        if automatic
            && command.get("action").and_then(serde_json::Value::as_str) == Some("navigate")
        {
            required_string(command, "url")?;
        }
        let requested_profile = if automatic {
            self.ordinary_profile_selector(command)?
        } else {
            optional_string(command, "profileId")
                .or_else(|| optional_string(command, "runtimeProfile"))
                .map(str::to_string)
        };
        let profile_id = requested_profile.as_deref();
        let now_ms = command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        let matching_ids = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.name == session_name
                    && exact_task_id.is_none_or(|id| session.id == id)
                    && profile_id.is_none_or(|profile_id| session.profile_id == profile_id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        if matching_ids.is_empty()
            || (command.get("manualLogin").is_some() && command["action"] == "navigate")
        {
            let mut posture = command.clone();
            posture["sessionName"] = serde_json::json!(session_name);
            if let Some(profile_id) = profile_id {
                posture["profileId"] = serde_json::json!(profile_id);
            }
            self.configure_profile_login(&posture)?;
        }
        let session_id = match matching_ids.as_slice() {
            [] if automatic && exact_task_id.is_none() => {
                if self.state.managed_tab_requests.values().any(|request| {
                    request.response.is_none()
                        && (request.session_name == session_name
                            || profile_id
                                .is_some_and(|id| request.profile_id.as_deref() == Some(id)))
                }) {
                    return Err("browser_session_tab_creation_readback_required".into());
                }
                let request = match profile_id {
                    Some(profile_id) if self.state.disposable_profiles.contains_key(profile_id) => {
                        OpenBrowserSession::existing_disposable(session_name, profile_id, now_ms)
                    }
                    Some(profile_id) => {
                        OpenBrowserSession::exact_profile(session_name, profile_id, now_ms)
                    }
                    None => OpenBrowserSession::disposable(
                        session_name,
                        optional_string(command, "disposablePolicyId")
                            .unwrap_or(DEFAULT_DISPOSABLE_POLICY_ID),
                        now_ms,
                    ),
                };
                self.open(request)?.session_id
            }
            [] if exact_task_id.is_some() => {
                return Err("browser_session_exact_selector_conflict".into());
            }
            [] => return Ok(None),
            [session_id] => session_id.clone(),
            _ => return Err(format!("browser_session_name_ambiguous:{session_name}")),
        };
        let browser_id = self
            .state
            .sessions
            .get(&session_id)
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?
            .browser_id
            .clone();
        if self
            .state
            .browsers
            .get(&browser_id)
            .is_some_and(|browser| browser.cdp_endpoint.is_empty())
        {
            let allowed = command["observeChildTabs"] != true
                && match command["action"].as_str() {
                    Some("url") => true,
                    Some("navigate") => self
                        .catalog
                        .profiles
                        .get(&self.state.sessions[&session_id].profile_id)
                        .and_then(|profile| profile.manual_login_url.as_deref())
                        .is_some_and(|url| command["url"] == url),
                    _ => false,
                };
            if !allowed {
                // This is a known preflight refusal, not an attempted physical
                // command. Do not leave unknown activity that fences close.
                return Err("manual_login_automation_unavailable: use native_capture/native_input for visual control, or close and reopen this preserved profile for DOM automation".into());
            }
        }
        self.recover_browser_for_command(&browser_id, command)?;
        self.apply_leave_open_choice(&session_id, command)?;
        let tab = self.tab_for_navigation(&session_id, now_ms)?;
        let session = self
            .state
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let tab = self
            .state
            .tabs
            .get(&tab.tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        // Capture original context once; later polling and navigation do not
        // replace provenance with the inspecting client's descriptive labels.
        if !self.state.tab_metadata.contains_key(&tab.id) {
            let metadata = capture_browser_task_metadata(command, &tab);
            self.state.tab_metadata.insert(tab.id.clone(), metadata);
            self.commit_state()?;
        }
        // Retain the logical target before returning an identity to a client.
        // Provider route issuance and the operator URL are separate joins.
        let handoff = if automatic && command.get("resumeHandoffId").is_none() {
            let id = self
                .state
                .remote_view_tab_handoffs
                .values()
                .find(|record| {
                    record.session_id == session.id
                        && record.tab_id == tab.id
                        && record.check_link_at(now_ms).is_ok()
                })
                .map(|record| record.id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let mut record = agent_browser_service_model::retain_remote_view_tab_handoff_at(
                &mut self.state,
                &id,
                &session.id,
                &tab.id,
                now_ms,
            )?;
            let ttl = match command.get("handoffTtlMs") {
                Some(value) => Some(
                    value
                        .as_u64()
                        .filter(|ttl| *ttl > 0)
                        .ok_or("remote_view_handoff_ttl_invalid")?,
                ),
                None if self
                    .state
                    .disposable_profiles
                    .contains_key(&session.profile_id) =>
                {
                    self.state.retention_policy.disposable_handoff_ttl_ms
                }
                None => self.state.retention_policy.durable_handoff_ttl_ms,
            };
            record.initialize_profile_retention(now_ms, ttl)?;
            record.resume = Some(
                agent_browser_service_model::RemoteViewTabHandoffResumeIntent {
                    tab_lifetime: self.state.tab_lifetimes.get(&tab.id).cloned(),
                    task_metadata: self.state.tab_metadata.get(&tab.id).cloned(),
                    session_name: session.name.clone(),
                    profile_id: session.profile_id.clone(),
                    disposable_policy_id: self
                        .state
                        .disposable_profiles
                        .get(&session.profile_id)
                        .map(|allocation| allocation.policy_id.clone()),
                    url: record.resume.as_ref().and_then(|intent| intent.url.clone()),
                },
            );
            self.state
                .remote_view_tab_handoffs
                .insert(record.id.clone(), record.clone());
            if self.state != self.persisted_state {
                self.commit_state()?;
            }
            Some(record)
        } else {
            None
        };
        self.effects.begin_operation(&self.persisted_state)?;
        let mut response =
            self.effects
                .execute_command(&browser, &tab, &session.id, &session.name, command)?;
        if command["observeChildTabs"] == true {
            let observed = self.effects.execute_command(
                &browser,
                &tab,
                &session.id,
                &session.name,
                &serde_json::json!({"action":"__observe_owned_popup_targets"}),
            )?;
            let children = observed["children"]
                .as_array()
                .ok_or("browser_session_popup_observation_invalid")?
                .iter()
                .map(|child| {
                    Ok((
                        required_string(child, "targetId")?.to_owned(),
                        required_string(child, "openerId")?.to_owned(),
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            self.manager()?
                .observe_child_tabs(&session.id, &children, now_ms)?;
            if self.state != self.persisted_state {
                self.commit_state()?;
            }
        }
        if command.get("action").and_then(serde_json::Value::as_str) == Some("navigate")
            && response.get("success").and_then(serde_json::Value::as_bool) == Some(true)
        {
            if let Some(url) = optional_string(command, "url") {
                self.record_navigation(&session.id, url, now_ms)?;
            }
        }
        if response["success"] == true {
            if let Some(ttl) = pending_human_ttl {
                let id = &handoff
                    .as_ref()
                    .ok_or("remote_view_tab_handoff_missing")?
                    .id;
                // An uncertain registration retains its exact key and hold
                // until the owner reconciles it; replay must not discard intent.
                let registration_pending = self
                    .state
                    .pending_human_tabs
                    .get(&tab.id)
                    .is_some_and(|hold| hold.handoff_id == *id)
                    && self.state.remote_view_tab_handoffs[id]
                        .native_observation
                        .as_ref()
                        .is_some_and(|binding| binding.observation.is_none());
                if !registration_pending {
                    self.handle_command_result(&serde_json::json!({
                        "action":"service_remote_view_handoff_resolve", "handoffId":id,
                        "handoffOperation":"prepare_human", "pendingHumanRetentionMs":ttl,
                        "activityAtMs":now_ms,
                    }))?;
                }
                // Registration is continuity context, never evidence of use.
                // Prepare it while the authorized task owner produces the
                // handoff; the public locator remains a read-only projection.
                match self.prepare_native_handoff_observation(id, now_ms) {
                    Ok(_) => {}
                    Err(reason) if reason == "remote_view_native_observation_unsupported" => {}
                    Err(reason) => return Err(reason),
                }
            }
        }
        if let Some(object) = response.as_object_mut() {
            object.insert("browserSession".into(), serde_json::json!({
                "sessionId": session.id, "sessionName": session.name, "profileId": session.profile_id,
                "browserId": browser.id, "tabId": tab.id, "targetId": tab.target_id,
            }));
            if let Some(handoff) = handoff {
                if let Some(identity) = object.get_mut("browserSession") {
                    identity["handoffId"] = serde_json::json!(handoff.id);
                    identity["handoffCreatedAtMs"] = serde_json::json!(handoff.created_at_ms);
                    identity["handoffExpiresAtMs"] = serde_json::json!(handoff.expires_at_ms);
                    identity["handoffUrl"] =
                        serde_json::json!(format!("/remote-view/{}", handoff.id));
                }
            }
        }
        if !self.effects.pending_browser_activity_ids().is_empty() {
            self.commit_state()?;
        }
        Ok(Some(response))
    }

    /// Retain one request before session admission or CDP tab creation. Completed
    /// responses replay without navigation; unknown outcomes require readback.
    fn execute_managed_tab_new(
        &mut self,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        use sha2::{Digest, Sha256};
        let exact_task_id = self.exact_task_selector(session_name, command)?;
        let response_id = required_string(command, "id")?;
        let id = if command.get("tabRequestId").is_some() {
            required_string(command, "tabRequestId")?
        } else {
            response_id
        };
        if id.len() > 256 {
            return Err("browser_session_tab_request_id_invalid".into());
        }
        let mut identity = command.clone();
        for field in [
            "id",
            "jobId",
            "serviceJobId",
            "connectionInstanceId",
            "requestId",
            "callerId",
            "requestPrincipalSource",
            "dashboardDeploymentGeneration",
        ] {
            identity
                .as_object_mut()
                .ok_or("browser_session_command_invalid")?
                .remove(field);
        }
        let digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&identity).map_err(|_| "browser_session_command_invalid")?
            )
        );
        self.effects.begin_operation(&self.persisted_state)?;
        if let Some(retained) = self.state.managed_tab_requests.get(id) {
            if retained.session_name != session_name || retained.command_digest != digest {
                return Err("browser_session_tab_request_conflict".into());
            }
            let response = retained
                .response
                .clone()
                .ok_or("browser_session_tab_creation_readback_required")?;
            if let Some(handoff) = response
                .pointer("/browserSession/handoffId")
                .and_then(serde_json::Value::as_str)
            {
                self.state
                    .remote_view_tab_handoffs
                    .get(handoff)
                    .ok_or("remote_view_tab_handoff_missing")?
                    .check_link_at(
                        command
                            .get("activityAtMs")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or_else(current_unix_ms),
                    )?;
                agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff)
                    .map_err(|_| "browser_session_tab_request_target_closed")?;
            } else {
                // A headless task has no viewer handoff. Its retained native
                // session/tab identity still fences replay after scoped close.
                if self.effects.admits_ordinary_sessions() {
                    return Err("browser_session_tab_request_invalid".into());
                }
                let session_id = response
                    .pointer("/browserSession/sessionId")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("browser_session_tab_request_invalid")?;
                let tab_id = response
                    .pointer("/browserSession/tabId")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("browser_session_tab_request_invalid")?;
                let session = self
                    .state
                    .sessions
                    .get(session_id)
                    .filter(|session| session.name == session_name)
                    .ok_or("browser_session_tab_request_target_closed")?;
                self.state
                    .tabs
                    .get(tab_id)
                    .filter(|tab| {
                        tab.session_id == session.id && tab.browser_id == session.browser_id
                    })
                    .ok_or("browser_session_tab_request_target_closed")?;
            }
            let mut response = response;
            response["id"] = serde_json::json!(response_id);
            return Ok(response);
        }
        let profile = self.ordinary_profile_selector(command)?;
        if self
            .state
            .sessions
            .values()
            .filter(|session| session.name == session_name)
            .any(|session| {
                self.state
                    .browsers
                    .get(&session.browser_id)
                    .is_some_and(|browser| browser.cdp_endpoint.is_empty())
            })
        {
            return Err("manual_login_additional_tabs_unavailable".into());
        }
        if command.get("url").is_some() {
            required_string(command, "url")?;
        }
        if command.get("serviceTabHandle").is_some() {
            return Err("browser_session_tab_handle_join_unavailable".into());
        }
        let matches = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.name == session_name
                    && exact_task_id.is_none_or(|id| session.id == id)
                    && profile.as_ref().is_none_or(|id| &session.profile_id == id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        let existing = match matches.as_slice() {
            [] if exact_task_id.is_some() => {
                return Err("browser_session_exact_selector_conflict".into());
            }
            [] => None,
            [id] => Some(id.clone()),
            _ => return Err("browser_session_name_ambiguous".into()),
        };
        if let Some(browser_id) = optional_string(command, "browserId") {
            if existing
                .as_ref()
                .and_then(|id| self.state.sessions.get(id))
                .is_none_or(|session| session.browser_id != browser_id)
            {
                return Err("browser_session_browser_selector_conflict".into());
            }
        }
        if self.state.managed_tab_requests.values().any(|request| {
            request.response.is_none()
                && (request.session_name == session_name
                    || profile
                        .as_ref()
                        .is_some_and(|id| request.profile_id.as_ref() == Some(id)))
        }) {
            return Err("browser_session_tab_creation_readback_required".into());
        }
        self.state.managed_tab_requests.insert(
            id.to_string(),
            agent_browser_service_model::ManagedBrowserTabRequest {
                session_name: session_name.to_string(),
                profile_id: profile.clone().or_else(|| {
                    existing
                        .as_ref()
                        .and_then(|id| self.state.sessions.get(id))
                        .map(|session| session.profile_id.clone())
                }),
                command_digest: digest,
                response: None,
            },
        );
        self.commit_state()?;
        let now_ms = command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        let session_id = match existing {
            Some(session_id) => {
                self.new_tab(&session_id, now_ms)?;
                session_id
            }
            None => {
                let request = match profile {
                    Some(profile) if self.state.disposable_profiles.contains_key(&profile) => {
                        OpenBrowserSession::existing_disposable(session_name, profile, now_ms)
                    }
                    Some(profile) => {
                        OpenBrowserSession::exact_profile(session_name, profile, now_ms)
                    }
                    None => OpenBrowserSession::disposable(
                        session_name,
                        optional_string(command, "disposablePolicyId")
                            .unwrap_or(DEFAULT_DISPOSABLE_POLICY_ID),
                        now_ms,
                    ),
                };
                self.open(request)?.session_id
            }
        };
        // Execute against the selected tab, never another native tab_new.
        let mut addressed = command.clone();
        addressed["action"] = serde_json::json!(if command.get("url").is_some() {
            "navigate"
        } else {
            "get_url"
        });
        let mut response = self
            .execute_managed_command(session_name, &addressed)?
            .ok_or("browser_session_managed_route_changed")?;
        if let Some(data) = response
            .get_mut("data")
            .and_then(serde_json::Value::as_object_mut)
        {
            data.insert(
                "tabId".into(),
                response_identity_tab(&self.state, &session_id)?,
            );
        }
        if let Some(handoff_id) = optional_string(command, "resumeHandoffId") {
            if response["success"] == true {
                let mut link = self
                    .state
                    .remote_view_tab_handoffs
                    .get(handoff_id)
                    .ok_or("remote_view_tab_handoff_missing")?
                    .clone();
                if link
                    .recreation
                    .as_ref()
                    .is_none_or(|pending| pending.request_id != id)
                {
                    return Err("remote_view_handoff_recreation_conflict".into());
                }
                link.check_link_at(now_ms)?;
                link.session_id = response["browserSession"]["sessionId"]
                    .as_str()
                    .ok_or("remote_view_handoff_recreation_identity_missing")?
                    .to_owned();
                link.tab_id = response["browserSession"]["tabId"]
                    .as_str()
                    .ok_or("remote_view_handoff_recreation_identity_missing")?
                    .to_owned();
                let target = agent_browser_service_model::RemoteViewTabHandoffResumeIntent {
                    task_metadata: link
                        .resume
                        .as_ref()
                        .and_then(|intent| intent.task_metadata.clone()),
                    tab_lifetime: link
                        .resume
                        .as_ref()
                        .and_then(|intent| intent.tab_lifetime.clone()),
                    session_name: session_name.to_owned(),
                    profile_id: response["browserSession"]["profileId"]
                        .as_str()
                        .ok_or("remote_view_handoff_recreation_identity_missing")?
                        .to_owned(),
                    disposable_policy_id: link
                        .resume
                        .as_ref()
                        .and_then(|intent| intent.disposable_policy_id.clone()),
                    url: optional_string(command, "url").map(str::to_owned),
                };
                if let Some(lifetime) = &target.tab_lifetime {
                    self.state
                        .tab_lifetimes
                        .insert(link.tab_id.clone(), lifetime.clone());
                }
                if let Some(metadata) = &target.task_metadata {
                    self.state
                        .tab_metadata
                        .insert(link.tab_id.clone(), metadata.clone());
                }
                link.resume = Some(target);
                link.recreation = None;
                link.target_expired = false;
                link.view = None;
                response["browserSession"]["handoffId"] = serde_json::json!(link.id);
                response["browserSession"]["handoffUrl"] =
                    serde_json::json!(format!("/remote-view/{}", link.id));
                response["browserSession"]["handoffCreatedAtMs"] =
                    serde_json::json!(link.created_at_ms);
                response["browserSession"]["handoffExpiresAtMs"] =
                    serde_json::json!(link.expires_at_ms);
                self.state
                    .remote_view_tab_handoffs
                    .insert(handoff_id.to_owned(), link);
            }
        }
        self.state
            .managed_tab_requests
            .get_mut(id)
            .ok_or("browser_session_tab_request_missing")?
            .response = Some(response.clone());
        self.commit_state()?;
        Ok(response)
    }

    /// An explicit ID is an exact selector, never permission to create a fallback.
    fn exact_task_selector<'a>(
        &self,
        session_name: &str,
        command: &'a serde_json::Value,
    ) -> Result<Option<&'a str>, String> {
        if command.get("sessionId").is_none() {
            return Ok(None);
        }
        let id = required_string(command, "sessionId")?;
        let task = self
            .state
            .sessions
            .get(id)
            .ok_or_else(|| format!("browser_session_not_found:{id}"))?;
        if task.name != session_name {
            return Err("browser_session_exact_selector_conflict".into());
        }
        Ok(Some(id))
    }

    /// Exact catalog selection; a raw profile path must already belong to a
    /// catalog entry. Conflicting explicit selectors fail before admission.
    fn ordinary_profile_selector(
        &self,
        command: &serde_json::Value,
    ) -> Result<Option<String>, String> {
        let mut selected: Option<String> = None;
        for field in ["profileId", "runtimeProfile", "profile"] {
            let Some(raw) = command.get(field) else {
                continue;
            };
            let value = raw
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "browser_session_profile_selector_invalid".to_string())?;
            let matches = self
                .catalog
                .profiles
                .values()
                .chain(
                    self.state
                        .disposable_profiles
                        .values()
                        .map(|allocation| &allocation.profile),
                )
                .filter(|profile| {
                    profile.id == value
                        || (field == "profile"
                            && (profile.name == value || profile.user_data_dir == value))
                })
                .map(|profile| profile.id.clone())
                .collect::<Vec<_>>();
            let id = match matches.as_slice() {
                [id] => id.clone(),
                [] => return Err("browser_session_profile_selector_unknown".into()),
                _ => return Err("browser_session_profile_selector_ambiguous".into()),
            };
            if selected.as_ref().is_some_and(|current| *current != id) {
                return Err("browser_session_profile_selector_conflict".into());
            }
            selected = Some(id);
        }
        Ok(selected)
    }
}

fn capture_task_policy(
    state: &mut BrowserSessionState,
    task_id: &str,
    command: &serde_json::Value,
) -> Result<(), String> {
    if let Some(value) = command.get("taskPolicy") {
        let policy = serde_json::from_value(value.clone()).map_err(|_| "browser_policy_invalid")?;
        let binding = state
            .host_bindings
            .get(task_id)
            .ok_or("browser_policy_hosted_task_required")?;
        state.host_policies.update(
            state.host_policies.revision,
            &binding.request.host_id,
            task_id,
            "set_task",
            Some(policy),
            "configured_app",
            "Explicit task policy",
        )?;
    }
    Ok(())
}

/// Persist reviewed application defaults once; later explicit policy changes win.
fn capture_host_policy_defaults(
    state: &mut BrowserSessionState,
    binding: &agent_browser_service_model::BrowserHostBinding,
    task_id: &str,
) -> Result<(), String> {
    if state
        .host_policies
        .initialized_hosts
        .contains(&binding.host_id)
    {
        return Ok(());
    }
    state
        .host_policies
        .initialized_hosts
        .insert(binding.host_id.clone());
    if let Some(policy) = super::browser_host_context::configured_host(binding)?.policy_defaults {
        state.host_policies.update(
            state.host_policies.revision,
            &binding.host_id,
            task_id,
            "set_app",
            Some(policy),
            "configured_app",
            "Reviewed host policy defaults",
        )?;
    }
    Ok(())
}

fn session_close_disposition_name(
    disposition: agent_browser_service_model::SessionCloseDisposition,
) -> &'static str {
    match disposition {
        agent_browser_service_model::SessionCloseDisposition::BrowserPreserved => {
            "browser_preserved"
        }
        agent_browser_service_model::SessionCloseDisposition::BrowserClosed => "browser_closed",
    }
}

fn required_string<'a>(command: &'a serde_json::Value, field: &str) -> Result<&'a str, String> {
    optional_string(command, field).ok_or_else(|| format!("browser_session_field_missing:{field}"))
}

/// Keep supplied context explicitly descriptive; absent identity is unknown.
fn capture_browser_task_metadata(
    command: &serde_json::Value,
    tab: &agent_browser_service_model::ManagedBrowserTab,
) -> agent_browser_service_model::BrowserTaskMetadata {
    let value = |field| {
        optional_string(command, field)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    let mut metadata = agent_browser_service_model::BrowserTaskMetadata {
        created_at_ms: Some(tab.created_at_ms),
        service_name: value("serviceName"),
        agent_name: value("agentName"),
        human_name: value("humanName"),
        origin_kind: value("originKind"),
        task_name: value("taskName"),
        purpose: value("purpose"),
        request_id: value("requestId"),
        job_id: value("serviceJobId").or_else(|| value("jobId")),
        ..Default::default()
    };
    if [
        &metadata.service_name,
        &metadata.agent_name,
        &metadata.human_name,
        &metadata.origin_kind,
        &metadata.task_name,
        &metadata.purpose,
        &metadata.request_id,
        &metadata.job_id,
    ]
    .iter()
    .any(|field| field.is_some())
    {
        metadata.attribution = "supplied".into();
    }
    metadata
}

fn optional_string<'a>(command: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    command
        .get(field)
        .or_else(|| command.get("params").and_then(|params| params.get(field)))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_control_return_observation_cannot_recover_while_human_is_still_held() {
        use crate::test_utils::EnvGuard;
        let directory = TempDirectory::new();
        let guard = EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        ]);
        guard.set("HOME", directory.0.to_str().unwrap());
        guard.set(
            "AGENT_BROWSER_HOME",
            directory.0.join(".agent-browser").to_str().unwrap(),
        );
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        fs::create_dir_all(directory.0.join(".agent-browser/service")).unwrap();
        let legacy = directory.0.join("state.json");
        fs::write(&legacy, serde_json::json!({"profiles":{"work":{"id":"work","name":"Work","userDataDir":directory.0.join("work"),"profileClass":"durable_named"}}}).to_string()).unwrap();
        let launches = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut host = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                launch_attempts: launches.clone(),
                ..FixtureRuntime::default()
            }),
            &legacy,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_view_desktops: Vec::new(),
                default_disposable_policy: None,
                exact_url_history_maximum_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let opened = host
            .open(OpenBrowserSession::exact_profile(
                "return-held",
                "work",
                1_000,
            ))
            .unwrap();
        let browser = host.state.sessions[&opened.session_id].browser_id.clone();
        super::super::browser_host_control::change(
            &browser, "takeover", 0, "operator", "Assist", true,
        )
        .unwrap();
        assert!(
            !super::super::browser_host_control::read(&browser)
                .unwrap()
                .awaiting_observation
        );
        let before = host.state.clone();
        let launched_before = launches.get();
        let error = host
            .recover_browser_for_command(
                &browser,
                &serde_json::json!({"action":"url","hostControlFreshObservation":true}),
            )
            .unwrap_err();
        assert_eq!(
            error,
            "browser_control_fresh_observation_cannot_recover_browser"
        );
        assert_eq!(
            launches.get(),
            launched_before,
            "no replacement launch before observation"
        );
        assert_eq!(
            host.state, before,
            "retained task and browser identity stay intact"
        );
        let control = super::super::browser_host_control::read(&browser).unwrap();
        assert!(control.human);
        assert_eq!(control.epoch, 1);
    }

    use super::*;
    use crate::native::browser_session_runtime::{
        BrowserRuntimeDriver, BrowserSessionEffectAdapter,
    };
    use agent_browser_service_model::{
        BrowserLaunch, BrowserProfileCatalogEntry, BrowserTabAcquisition, ManagedBrowserInstance,
        ManagedBrowserTab,
    };
    use std::fs;
    use std::path::PathBuf;

    #[derive(Default)]
    struct FixtureRuntime {
        launches: usize,
        launch_attempts: std::rc::Rc<std::cell::Cell<u32>>,
        live: bool,
        attachment_required: bool,
        attached: bool,
        next_tab: usize,
        pending_intent: Option<BrowserLaunchIntent>,
        initial_tabs: bool,
        navigation_failure: bool,
        navigation_attempts: std::rc::Rc<std::cell::Cell<u32>>,
        acknowledgements: std::rc::Rc<std::cell::Cell<u32>>,
    }

    impl BrowserRuntimeDriver for FixtureRuntime {
        fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
            self.pending_intent.clone()
        }
        fn acknowledge_launch_publication(&mut self) {
            if self.pending_intent.take().is_some() {
                self.acknowledgements.set(self.acknowledgements.get() + 1);
            }
        }
        fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
            self.attached = self.live;
            Ok(self.live)
        }

        fn launch_browser(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::RemoteViewFixedDesktop>,
        ) -> Result<BrowserLaunch, String> {
            self.launches += 1;
            self.launch_attempts.set(self.launch_attempts.get() + 1);
            Ok(BrowserLaunch {
                process_identity: None,
                browser_id: format!("browser:{}:{}", profile.id, self.launches),
                pid: 4242,
                cdp_endpoint: "ws://127.0.0.1:9422/devtools/browser/test".to_string(),
                desktop: desktop.cloned(),
            })
        }

        fn close_browser(&mut self, _browser: &ManagedBrowserInstance) -> Result<(), String> {
            Ok(())
        }

        fn acquire_initial_tab(
            &mut self,
            browser: &ManagedBrowserInstance,
            _attributed_target_ids: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            if self.initial_tabs {
                let mut tab = self.create_tab(browser)?;
                tab.source = agent_browser_service_model::BrowserTabSource::SessionInitial;
                return Ok(tab);
            }
            Err("unused".to_string())
        }

        fn create_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            self.next_tab += 1;
            Ok(BrowserTabAcquisition {
                tab_id: format!("tab-{}", self.next_tab),
                target_id: format!("target-{}", self.next_tab),
                source: agent_browser_service_model::BrowserTabSource::ExplicitNew,
            })
        }

        fn close_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
        ) -> Result<(), String> {
            Ok(())
        }

        fn navigate(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
            _url: &str,
        ) -> Result<(), String> {
            self.navigation_attempts
                .set(self.navigation_attempts.get() + 1);
            if self.navigation_failure {
                return Err("synthetic_navigation_outcome_unknown".into());
            }
            Ok(())
        }

        fn focus_browser(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: Option<&ManagedBrowserTab>,
        ) -> Result<(), String> {
            Ok(())
        }

        fn execute_command(
            &mut self,
            browser: &ManagedBrowserInstance,
            tab: &ManagedBrowserTab,
            session_id: &str,
            session_name: &str,
            command: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if self.attachment_required && !self.attached {
                return Err("browser_session_runtime_browser_missing".into());
            }
            Ok(serde_json::json!({
                "id": command.get("id").cloned().unwrap_or_default(),
                "success": true,
                "data": {
                    "browserId": browser.id,
                    "tabId": tab.id,
                    "targetId": tab.target_id,
                    "sessionId": session_id,
                    "sessionName": session_name,
                    "action": command.get("action").cloned().unwrap_or_default(),
                }
            }))
        }
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-browser-session-host-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn host_publishes_observed_launch_and_aggregate_atomically_before_acknowledgement() {
        use agent_browser_service_model::{LaunchCustodyAdmission, RemoteViewFixedDesktop};
        let directory = TempDirectory::new();
        let database = directory.0.join("runtime.sqlite3");
        let open_store = |initialize| {
            BrowserSessionSqliteStore::launch_custody_fixture(
                rusqlite::Connection::open(&database).unwrap(),
                initialize,
            )
            .unwrap()
        };
        let mut store = open_store(true);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let intent = BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: "profile-a".into(),
            assignment: serde_json::from_value(fixture["assignment"].clone()).unwrap(),
        };
        let initial = store.load_session_state().unwrap();
        let mut unsupported = BrowserSessionJsonStore::new(&directory.0.join("unsupported"));
        assert_eq!(
            unsupported
                .publish_session_state(&initial, &initial, Some(&intent))
                .unwrap_err(),
            "browser_session_launch_publication_unsupported"
        );
        store.admit_launch_intent(&intent, &initial).unwrap();
        let launch = BrowserLaunch {
            process_identity: None,
            browser_id: "host-browser".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation,
                friendly_route_label: String::new(),
            }),
        };
        store.observe_launch_intent(&intent, &launch).unwrap();
        let acknowledgements = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut host = BrowserSessionHost {
            persistence: store,
            effects: BrowserSessionEffectAdapter::new(FixtureRuntime {
                pending_intent: Some(intent.clone()),
                acknowledgements: acknowledgements.clone(),
                ..FixtureRuntime::default()
            }),
            catalog: BrowserProfileCatalog::default(),
            state: initial.clone(),
            persisted_state: initial.clone(),
            manager_config: BrowserSessionManagerConfig::default(),
            exact_url_history_maximum_bytes: 1024,
            native_use_cursor: 0,
        };
        host.state.browsers.insert(
            launch.browser_id.clone(),
            ManagedBrowserInstance {
                id: launch.browser_id.clone(),
                profile_id: intent.profile_id.clone(),
                pid: launch.pid + 1,
                cdp_endpoint: launch.cdp_endpoint,
                desktop: launch.desktop,
                active_session_ids: vec![],
            },
        );
        assert_eq!(
            host.commit_state().unwrap_err(),
            "browser_session_launch_publication_conflict"
        );
        assert_eq!(acknowledgements.get(), 0);
        assert_eq!(host.persisted_state, initial);
        assert!(host.effects.pending_launch_intent().is_some());
        let mut peer = open_store(false);
        assert_eq!(peer.load_session_state().unwrap(), initial);
        host.state.browsers.get_mut(&launch.browser_id).unwrap().pid = launch.pid;
        // Ordinary observations do not publish or consume process-effect custody.
        peer.compare_and_save_session_state(&initial, &host.state)
            .unwrap();
        assert_eq!(peer.load_session_state().unwrap(), host.state);
        assert!(!peer.unpublished_launch_records().unwrap()[0].published);
        peer.compare_and_save_session_state(&host.state, &initial)
            .unwrap();
        let mut competing = initial.clone();
        competing.next_session_sequence += 1;
        peer.compare_and_save_session_state(&initial, &competing)
            .unwrap();
        assert!(host.commit_state().is_err());
        assert_eq!(acknowledgements.get(), 0);
        assert_eq!(peer.load_session_state().unwrap(), competing);
        peer.compare_and_save_session_state(&competing, &initial)
            .unwrap();
        host.commit_state().unwrap();
        assert_eq!(acknowledgements.get(), 1);
        assert!(host.effects.pending_launch_intent().is_none());
        assert_eq!(host.persisted_state, host.state);
        assert_eq!(peer.load_session_state().unwrap(), host.state);
        match peer.admit_launch_intent(&intent, &host.state).unwrap() {
            LaunchCustodyAdmission::Existing(record) => assert!(record.published),
            LaunchCustodyAdmission::New => panic!("publication must retain the original intent"),
        }
    }

    #[test]
    fn hosted_context_publishes_with_task_or_not_at_all() {
        let guard = crate::test_utils::EnvGuard::new(&["AGENT_BROWSER_HOSTS_FILE"]);
        let directory = TempDirectory::new();
        let registry = directory.0.join("hosts.json");
        fs::write(
            &registry,
            serde_json::json!({"version":1,"hosts":{"app":{
                "origin":"https://app.example.test","providerApplication":"agent-browser",
                "policyDefaults":{"version":1,"deniedActions":["evaluate"],"artifactDestinations":{}}
            }}})
            .to_string(),
        )
        .unwrap();
        guard.set("AGENT_BROWSER_HOSTS_FILE", registry.to_str().unwrap());
        let task_policy =
            serde_json::json!({"version":1,"deniedActions":["click"],"artifactDestinations":{}});
        struct Store {
            state: BrowserSessionState,
            catalog: BrowserProfileCatalog,
            fail: bool,
            attempts: Vec<BrowserSessionState>,
        }
        impl BrowserSessionPersistence for Store {
            fn load_session_state(&self) -> Result<BrowserSessionState, String> {
                Ok(self.state.clone())
            }
            fn compare_and_save_session_state(
                &mut self,
                expected: &BrowserSessionState,
                next: &BrowserSessionState,
            ) -> Result<(), String> {
                assert_eq!(expected, &self.state);
                // Every publication candidate must carry context with the new task.
                for session in next.sessions.values() {
                    assert!(
                        next.host_bindings.contains_key(&session.id),
                        "task publication omitted hosted context"
                    );
                    assert!(
                        next.host_policies.app_defaults["app"]
                            .denied_actions
                            .contains("evaluate"),
                        "task publication omitted configured app defaults"
                    );
                    assert!(
                        next.host_policies.task_overrides[&session.id]
                            .denied_actions
                            .contains("click"),
                        "task publication omitted explicit task policy"
                    );
                    let effective = next.host_policies.effective("app", &session.id).unwrap();
                    assert!(effective.denied_actions.contains("click"));
                    assert!(
                        !effective.denied_actions.contains("evaluate"),
                        "task policy must replace the default"
                    );
                }
                self.attempts.push(next.clone());
                if self.fail {
                    return Err("synthetic_publication_failure".into());
                }
                self.state = next.clone();
                Ok(())
            }
            fn load_or_import_profile_catalog(
                &self,
                _: &Path,
            ) -> Result<BrowserProfileCatalogLoad, String> {
                Ok(BrowserProfileCatalogLoad {
                    catalog: self.catalog.clone(),
                    diagnostics: vec![],
                    imported_legacy_profiles: false,
                })
            }
            fn save_profile_catalog(&self, _: &BrowserProfileCatalog) -> Result<(), String> {
                panic!("unexpected profile mutation")
            }
        }
        use agent_browser_service_model::{
            BrowserDesktopBinding, BrowserHostBinding, BrowserPlacement, BrowserProfileKind,
            RemoteViewFixedDesktop,
        };
        for fail in [false, true] {
            let desktop = RemoteViewFixedDesktop {
                desktop_id: "app-desktop".into(),
                generation: 1,
                friendly_route_label: "app".into(),
            };
            let profile = BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "work".into(),
                name: "work".into(),
                user_data_dir: "/synthetic/work".into(),
                kind: BrowserProfileKind::Named,
            };
            let catalog = BrowserProfileCatalog {
                profiles: std::collections::BTreeMap::from([(profile.id.clone(), profile)]),
                ..BrowserProfileCatalog::default()
            };
            let mut state = BrowserSessionState::default();
            state.browsers.insert(
                "browser".into(),
                ManagedBrowserInstance {
                    id: "browser".into(),
                    profile_id: "work".into(),
                    pid: 42,
                    cdp_endpoint: "http://synthetic".into(),
                    desktop: Some(desktop.clone()),
                    active_session_ids: vec![],
                },
            );
            let mut host = BrowserSessionHost {
                persistence: Store {
                    state: state.clone(),
                    catalog: catalog.clone(),
                    fail,
                    attempts: vec![],
                },
                effects: BrowserSessionEffectAdapter::new(FixtureRuntime {
                    live: true,
                    ..Default::default()
                }),
                catalog,
                state: state.clone(),
                persisted_state: state.clone(),
                manager_config: BrowserSessionManagerConfig {
                    session_idle_timeout_ms: 1000,
                    remote_view_desktops: vec![
                        agent_browser_service_model::RemoteViewDesktopCandidate {
                            desktop,
                            ready: true,
                        },
                    ],
                },
                exact_url_history_maximum_bytes: 1024,
                native_use_cursor: 0,
            };
            let binding = BrowserHostBinding {
                version: 1,
                host_id: "app".into(),
                participant_id: "participant".into(),
                session_id: "app-session".into(),
                placement: BrowserPlacement::Required {
                    desktop: BrowserDesktopBinding {
                        desktop_id: "app-desktop".into(),
                        generation: 1,
                    },
                },
            };
            let result = host.open_with_host_binding_policy(
                OpenBrowserSession::exact_profile("task", "work", 1)
                    .with_placement(binding.placement.clone()),
                Some(binding.clone()),
                Some(&task_policy),
            );
            assert_eq!(host.persistence.attempts.len(), 1);
            if fail {
                assert_eq!(result.unwrap_err(), "synthetic_publication_failure");
                assert_eq!(host.persistence.state, state);
            } else {
                let opened = result.unwrap();
                assert_eq!(
                    host.persistence.state.host_bindings[&opened.session_id].request,
                    binding
                );
                assert!(host
                    .persistence
                    .state
                    .sessions
                    .contains_key(&opened.session_id));
                assert!(host
                    .persistence
                    .state
                    .host_policies
                    .initialized_hosts
                    .contains("app"));
                assert!(
                    host.persistence.state.host_policies.task_overrides[&opened.session_id]
                        .denied_actions
                        .contains("click")
                );
            }
        }
    }

    #[test]
    fn startup_reaper_preserves_empty_and_failed_loads_without_a_host() {
        type FixtureHost = BrowserSessionHost<
            BrowserSessionJsonStore,
            BrowserSessionEffectAdapter<FixtureRuntime>,
        >;
        let mut host: Option<FixtureHost> = None;
        reap_browser_session_host(&mut host, || Ok(None)).unwrap();
        assert!(host.is_none());
        assert_eq!(
            reap_browser_session_host(&mut host, || Err("retained state unavailable".into()))
                .unwrap_err(),
            "retained state unavailable"
        );
        assert!(host.is_none());
    }

    #[test]
    fn startup_reaper_loads_retained_sessions_without_a_client_request() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(&legacy_path, serde_json::json!({"profiles":{"work":{"id":"work","name":"Work","userDataDir":directory.0.join("work"),"profileClass":"durable_named"}}}).to_string()).unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: vec![],
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 1024,
        };
        let mut initial = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime::default()),
            &legacy_path,
            config.clone(),
        )
        .unwrap();
        let opened = initial
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
            .unwrap();
        let tab = initial
            .manager()
            .unwrap()
            .new_tab(&opened.session_id, 1_000)
            .unwrap();
        agent_browser_service_model::retain_remote_view_tab_handoff(
            &mut initial.state,
            "startup-handoff",
            &opened.session_id,
            &tab.tab_id,
        )
        .unwrap();
        initial.commit_state().unwrap();
        drop(initial);
        let mut restarted = None;
        reap_browser_session_host(&mut restarted, || {
            BrowserSessionHost::load(
                BrowserSessionJsonStore::new(&directory.0),
                BrowserSessionEffectAdapter::new(FixtureRuntime::default()),
                &legacy_path,
                config,
            )
            .map(Some)
        })
        .unwrap();
        let host = restarted
            .as_ref()
            .expect("restart reaping must load retained ownership without a client request");
        assert!(host
            .state
            .idle_closed_browsers
            .contains_key(&opened.browser_id));
        assert!(
            agent_browser_service_model::resolve_remote_view_tab_handoff(
                &host.state,
                "startup-handoff"
            )
            .is_ok()
        );
        assert_eq!(
            BrowserSessionJsonStore::new(&directory.0)
                .load_session_state()
                .unwrap(),
            host.state
        );
    }

    #[test]
    fn first_named_task_command_after_restart_attaches_existing_browser() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({"profiles": {"work": {
                "id": "work", "name": "Work",
                "userDataDir": directory.0.join("work"),
                "profileClass": "durable_named"
            }}})
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
        };
        let (first, tab_id) = {
            let mut host = BrowserSessionHost::load(
                BrowserSessionJsonStore::new(&directory.0),
                BrowserSessionEffectAdapter::new(FixtureRuntime::default()),
                &legacy_path,
                config.clone(),
            )
            .unwrap();
            let opened = host
                .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
                .unwrap();
            let tab = host.handle_command(&serde_json::json!({
                "action": "browser_session_tab_new", "sessionName": "alice",
                "profileId": "work", "activityAtMs": 1_100
            }));
            assert_eq!(tab["success"], true);
            (opened, tab["data"]["tabId"].as_str().unwrap().to_string())
        };
        let mut restarted = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                attachment_required: true,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            config.clone(),
        )
        .unwrap();
        let result = restarted
            .execute_managed_command(
                "alice",
                &serde_json::json!({
                    "action": "title", "profileId": "work", "activityAtMs": 1_500
                }),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result["data"]["browserId"], first.browser_id);
        assert_eq!(result["data"]["sessionId"], first.session_id);
        assert_eq!(result["data"]["tabId"], tab_id);
        assert_eq!(restarted.state().browsers.len(), 1);
        assert_eq!(restarted.state().tabs.len(), 1);
        let mut focused = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                attachment_required: true,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            config.clone(),
        )
        .unwrap();
        let target = focused.state().tabs[&tab_id].target_id.clone();
        let before_focus = focused.state().clone();
        let selected = focused
            .execute_operator_focus(&serde_json::json!({
                "action":"view_focus", "operatorFocus":true,
                "browserId":first.browser_id, "targetId":target, "sessionName":"alice"
            }))
            .expect("operator focus must attach the exact live browser after restart");
        assert_eq!(selected["success"], true);
        assert_eq!(selected["data"]["browserId"], first.browser_id);
        assert_eq!(focused.state(), &before_focus);

        let mut unavailable = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: false,
                attachment_required: true,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            config,
        )
        .unwrap();
        assert_eq!(
            unavailable
                .execute_operator_focus(&serde_json::json!({
                    "action":"view_focus", "operatorFocus":true,
                    "browserId":first.browser_id, "targetId":target, "sessionName":"alice"
                }))
                .unwrap_err(),
            "operator_focus_browser_unavailable"
        );
        assert_eq!(unavailable.state(), &before_focus);
    }

    #[test]
    fn task_tab_close_rejects_a_changed_current_tab_before_effects() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({"profiles":{"work":{
            "id":"work","name":"Work","userDataDir":directory.0.join("work"),
            "profileClass":"durable_named"}}})
            .to_string(),
        )
        .unwrap();
        let mut host = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_view_desktops: Vec::new(),
                default_disposable_policy: None,
                exact_url_history_maximum_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let task = host
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
            .unwrap();
        let first = host.new_tab(&task.session_id, 1_100).unwrap();
        // A different client selects another owned tab after MCP selected first.
        let second = host.new_tab(&task.session_id, 1_200).unwrap();
        let before = host.state().clone();
        let stale = host.handle_command(&serde_json::json!({"id":"stale-close",
            "action":"browser_session_tab_close","sessionId":task.session_id,
            "expectedTabId":first.tab_id,"activityAtMs":1_300}));
        assert_eq!(
            stale["success"], false,
            "stale close must not close the current peer"
        );
        assert_eq!(
            stale["error"],
            "browser_session_tab_close_selection_changed"
        );
        assert_eq!(host.state(), &before);
        let exact = host.handle_command(&serde_json::json!({"id":"exact-close",
            "action":"browser_session_tab_close","sessionId":task.session_id,
            "expectedTabId":second.tab_id,"activityAtMs":1_400}));
        assert_eq!(exact["success"], true);
        assert_eq!(exact["data"]["closedTabId"], second.tab_id);
        assert!(host.state().tabs.contains_key(&first.tab_id));
        let legacy = host.handle_command(&serde_json::json!({"id":"legacy-close",
            "action":"browser_session_tab_close","sessionId":task.session_id,"activityAtMs":1_500}));
        assert_eq!(legacy["success"], true);
        assert_eq!(legacy["data"]["closedTabId"], first.tab_id);
    }

    #[test]
    fn headless_managed_tab_retry_replays_without_presentation_and_rejects_closed_target() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({"profiles":{"work":{
            "id":"work","name":"Work","userDataDir":directory.0.join("work"),
            "profileClass":"durable_named"}}})
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
        };
        let mut host = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            config,
        )
        .unwrap();
        let task = host
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
            .unwrap();
        let command = serde_json::json!({"id":"first","action":"tab_new",
            "tabRequestId":"stable-key","activityAtMs":1_100});
        let first = host
            .execute_managed_command("alice", &command)
            .unwrap()
            .unwrap();
        assert!(first["browserSession"]["handoffId"].is_null());
        let mut retry = command.clone();
        retry["id"] = "retry".into();
        let replay = host
            .execute_managed_command("alice", &retry)
            .unwrap()
            .unwrap();
        assert_eq!(first["data"]["tabId"], replay["data"]["tabId"]);
        assert_eq!(host.state.tabs.len(), 1);
        host.close_current_tab(&task.session_id, 1_200).unwrap();
        assert_eq!(
            host.execute_managed_command("alice", &retry).unwrap_err(),
            "browser_session_tab_request_target_closed"
        );
        assert!(host.state.tabs.is_empty());
    }

    #[test]
    fn ordinary_task_open_refreshes_peer_changes_without_losing_or_replaying_tasks() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({"profiles":{"work":{
                "id":"work", "name":"Work", "userDataDir":directory.0.join("work"),
                "profileClass":"durable_named"
            }}})
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
        };
        let mut stale = BrowserSessionHost::load(
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path)
                .unwrap()
                .0,
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                initial_tabs: true,
                next_tab: 10,
                ..Default::default()
            }),
            &legacy_path,
            config.clone(),
        )
        .unwrap();
        let mut peer = BrowserSessionHost::load(
            BrowserSessionSqliteStore::open(&directory.0).unwrap(),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                initial_tabs: true,
                ..Default::default()
            }),
            &legacy_path,
            config,
        )
        .unwrap();
        let bob = peer
            .open(OpenBrowserSession::exact_profile("bob", "work", 1_000))
            .unwrap();
        let bob_acquired = peer.tab_for_navigation(&bob.session_id, 1_100).unwrap();
        let bob_session = peer.state().sessions[&bob.session_id].clone();
        let bob_tab = peer.state().tabs[&bob_acquired.tab_id].clone();
        let command = serde_json::json!({"action":"browser_session_open",
            "sessionName":"alice", "profileId":"work", "activityAtMs":2_000});
        let opened = stale.handle_command(&command);
        assert_eq!(opened["success"], true, "{opened}");
        let alice_id = opened["data"]["sessionId"].as_str().unwrap();
        assert_eq!(stale.state().sessions[&bob.session_id], bob_session);
        assert_eq!(stale.state().tabs[&bob_acquired.tab_id], bob_tab);
        let replay = stale.handle_command(&command);
        assert_eq!(replay["data"]["sessionId"], alice_id);
        let navigated = stale
            .handle_command(&serde_json::json!({"action":"browser_session_navigate",
            "sessionId":alice_id, "url":"https://synthetic.example/alice", "activityAtMs":2_100}));
        assert_eq!(navigated["success"], true, "{navigated}");
        assert_eq!(stale.state().browsers.len(), 1);
        assert_eq!(stale.state().tabs.len(), 2);
        let closed = stale.handle_command(&serde_json::json!({"action":"browser_session_close",
            "sessionName":"alice", "profileId":"work", "activityAtMs":3_000}));
        assert_eq!(closed["success"], true, "{closed}");
        assert_eq!(stale.state().sessions[&bob.session_id], bob_session);
        assert_eq!(stale.state().tabs[&bob_acquired.tab_id], bob_tab);
        assert_eq!(stale.state().tabs.len(), 1);
    }

    #[test]
    fn ordinary_refresh_preserves_unpublished_navigation_after_peer_advances() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({"profiles":{"work":{
                "id":"work", "name":"Work", "userDataDir":directory.0.join("work"),
                "profileClass":"durable_named"
            }}})
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
        };
        let attempts = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut uncertain = BrowserSessionHost::load(
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path)
                .unwrap()
                .0,
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                initial_tabs: true,
                navigation_failure: true,
                navigation_attempts: attempts.clone(),
                ..Default::default()
            }),
            &legacy_path,
            config.clone(),
        )
        .unwrap();
        let failed =
            uncertain.handle_command(&serde_json::json!({"action":"browser_session_navigate",
            "sessionName":"alice", "profileId":"work", "url":"https://synthetic.example/unknown",
            "activityAtMs":1_000}));
        assert_eq!(failed["error"], "synthetic_navigation_outcome_unknown");
        let held = uncertain.state().clone();
        let mut peer = BrowserSessionHost::load(
            BrowserSessionSqliteStore::open(&directory.0).unwrap(),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                initial_tabs: true,
                ..Default::default()
            }),
            &legacy_path,
            config,
        )
        .unwrap();
        peer.open(OpenBrowserSession::exact_profile("bob", "work", 2_000))
            .unwrap();
        let durable = peer.state().clone();
        let rejected = uncertain
            .handle_command(&serde_json::json!({"action":"browser_session_open",
            "sessionName":"charlie", "profileId":"work", "activityAtMs":3_000}));
        assert_eq!(rejected["error"], "browser_session_publication_conflict");
        assert_eq!(uncertain.state(), &held);
        let status =
            uncertain.handle_command(&serde_json::json!({"action":"browser_session_status"}));
        assert_eq!(
            status["success"], true,
            "diagnostics must remain available: {status}"
        );
        assert_eq!(
            attempts.get(),
            1,
            "uncertain navigation must not be replayed"
        );
        assert_eq!(
            BrowserSessionSqliteStore::open(&directory.0)
                .unwrap()
                .load_session_state()
                .unwrap(),
            durable
        );
    }

    #[test]
    fn restart_loads_independent_state_and_reuses_healthy_session() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": directory.0.join("work"),
                        "profileClass": "durable_named"
                    }
                },
                "sessions": "ignored legacy contradiction"
            })
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
        };
        let first = {
            let store = BrowserSessionJsonStore::new(&directory.0);
            let effects = BrowserSessionEffectAdapter::new(FixtureRuntime::default());
            let mut host =
                BrowserSessionHost::load(store, effects, &legacy_path, config.clone()).unwrap();
            host.open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
                .unwrap()
        };

        let store = BrowserSessionJsonStore::new(&directory.0);
        let effects = BrowserSessionEffectAdapter::new(FixtureRuntime {
            launches: 0,
            live: true,
            next_tab: 0,
            ..FixtureRuntime::default()
        });
        let mut restarted = BrowserSessionHost::load(store, effects, &legacy_path, config).unwrap();
        let resumed = restarted
            .open(OpenBrowserSession::exact_profile("alice", "work", 2_000))
            .unwrap();

        assert_eq!(resumed.session_id, first.session_id);
        assert_eq!(resumed.browser_id, first.browser_id);
        assert_eq!(restarted.state().sessions.len(), 1);

        let new_tab = restarted.handle_command(&serde_json::json!({
            "id": "new-alice-tab",
            "action": "browser_session_tab_new",
            "sessionName": "alice",
            "profileId": "work",
            "url": "https://example.test/next",
            "activityAtMs": 2_500
        }));
        assert_eq!(new_tab["success"], true);
        assert_eq!(new_tab["data"]["source"], "explicit_new");
        assert_eq!(new_tab["data"]["url"], "https://example.test/next");
        assert_eq!(restarted.state().tabs.len(), 1);
        assert_eq!(restarted.state().navigation_history.len(), 1);
        restarted.exact_url_history_maximum_bytes = 2;
        restarted
            .record_navigation(
                &first.session_id,
                "https://example.test/compacted",
                1_758_758_400_000,
            )
            .unwrap();
        assert!(restarted.state().navigation_history.is_empty());
        assert_eq!(restarted.state().navigation_daily_summaries.len(), 2);
        assert_eq!(restarted.state().history_compaction_events.len(), 1);
        let persisted = BrowserSessionJsonStore::new(&directory.0)
            .load_session_state()
            .unwrap();
        assert_eq!(persisted, *restarted.state());

        let focus = restarted.handle_command(&serde_json::json!({
            "id": "focus-alice-browser",
            "action": "browser_session_focus",
            "browserId": resumed.browser_id,
            "targetId": "target-1",
            "activityAtMs": 2_600
        }));
        assert_eq!(focus["success"], true);
        assert_eq!(focus["data"]["focused"], true);
        assert_eq!(focus["data"]["maximized"], true);
        assert_eq!(focus["data"]["targetId"], "target-1");

        let snapshot = restarted
            .execute_managed_command(
                "alice",
                &serde_json::json!({
                    "id": "snapshot-alice",
                    "action": "snapshot",
                    "activityAtMs": 2_700
                }),
            )
            .unwrap()
            .expect("alice's active manager session should own the command");
        assert_eq!(snapshot["success"], true);
        assert_eq!(snapshot["data"]["browserId"], resumed.browser_id);
        assert_eq!(snapshot["data"]["tabId"], "tab-1");
        assert_eq!(snapshot["data"]["targetId"], "target-1");
        assert_eq!(snapshot["data"]["sessionId"], first.session_id);
        assert_eq!(snapshot["data"]["sessionName"], "alice");
        assert_eq!(snapshot["data"]["action"], "snapshot");
        assert_eq!(
            restarted
                .state()
                .sessions
                .get(&first.session_id)
                .unwrap()
                .last_activity_at_ms,
            2_700
        );
        assert!(restarted
            .execute_managed_command("bob", &serde_json::json!({ "action": "snapshot" }))
            .unwrap()
            .is_none());

        let close_tab = restarted.handle_command(&serde_json::json!({
            "id": "close-alice-tab",
            "action": "browser_session_tab_close",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 2_750
        }));
        assert_eq!(close_tab["success"], true);
        assert_eq!(close_tab["data"]["closedTabId"], "tab-1");
        assert!(restarted.state().tabs.is_empty());

        let response = restarted.handle_command(&serde_json::json!({
            "id": "close-alice",
            "action": "browser_session_close",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 3_000
        }));
        assert_eq!(response["success"], false);
        assert_eq!(response["error"], "browser_session_not_found_by_name:alice");
        assert!(restarted.state().sessions.is_empty());
        assert!(restarted.state().browsers.is_empty());

        // A different persistence owner advances state after this host loaded it.
        // Refuse the next manager operation before it can launch a browser.
        let peer = BrowserSessionJsonStore::new(&directory.0);
        let mut newer = peer.load_session_state().unwrap();
        newer.next_session_sequence += 1;
        peer.save_session_state(&newer).unwrap();
        let before = restarted.state().clone();
        assert_eq!(
            restarted
                .open(OpenBrowserSession::exact_profile("alice", "work", 4_000))
                .unwrap_err(),
            "browser_session_publication_conflict"
        );
        assert_eq!(*restarted.state(), before);
        assert_eq!(peer.load_session_state().unwrap(), newer);
    }
}

fn response_identity_tab(
    state: &BrowserSessionState,
    session_id: &str,
) -> Result<serde_json::Value, String> {
    let tab = state
        .sessions
        .get(session_id)
        .and_then(|session| session.current_tab_id.as_ref())
        .ok_or("browser_session_current_tab_missing")?;
    Ok(serde_json::json!(tab))
}

/// Reject malformed creation TTLs before profile registration or browser effects.
fn validate_handoff_ttl(
    command: &serde_json::Value,
    now_ms: u64,
    policy: &agent_browser_service_model::BrowserRetentionPolicy,
) -> Result<(), String> {
    policy.validate()?;
    for ttl in [
        policy.durable_handoff_ttl_ms,
        policy.disposable_handoff_ttl_ms,
    ]
    .into_iter()
    .flatten()
    {
        now_ms
            .checked_add(ttl)
            .ok_or("browser_retention_policy_invalid")?;
    }
    if let Some(value) = command.get("handoffTtlMs") {
        let ttl = value
            .as_u64()
            .filter(|ttl| *ttl > 0)
            .ok_or("remote_view_handoff_ttl_invalid")?;
        now_ms
            .checked_add(ttl)
            .ok_or("remote_view_handoff_ttl_invalid")?;
    }
    Ok(())
}
