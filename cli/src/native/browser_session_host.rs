//! Durable host for the ordinary Browser Session Manager path.

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

const DEFAULT_SESSION_IDLE_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_DISPOSABLE_CLEANUP_DELAY_MS: u64 = 300_000;
const DEFAULT_DISPOSABLE_POLICY_ID: &str = "default";

pub(crate) type DefaultBrowserSessionHost = BrowserSessionHost<
    BrowserSessionSqliteStore,
    super::browser_session_remote_view::RuntimeSessionEffects,
>;

pub(crate) type SharedBrowserSessionHost =
    std::sync::Arc<std::sync::Mutex<Option<DefaultBrowserSessionHost>>>;

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
            if command["action"] == "tab_new" && !host.effects.admits_ordinary_sessions() {
                return Ok(false);
            }
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

/// Open a client-selected persistent profile through the application pool.
/// Explicit paths are registered once; a changed ID/path binding fails closed.
pub(crate) async fn execute_shared_remote_view_open(
    shared: SharedBrowserSessionHost,
    name: String,
    command: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    tokio::task::spawn_blocking(move || {
        let mut shared = shared
            .lock()
            .map_err(|_| "browser_session_host_lock_poisoned")?;
        if shared.is_none() {
            *shared = Some(load_default_browser_session_host()?);
        }
        let host = shared.as_mut().ok_or("browser_session_host_missing")?;
        let selected_build = optional_string(&command, "browserBuild");
        let selected_executable = match selected_build {
            None => None,
            Some("stock_chrome") => {
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
            let profile = optional_string(&command, "runtimeProfile");
            for browser in host
                .state
                .browsers
                .values()
                .filter(|browser| profile == Some(browser.profile_id.as_str()))
            {
                let actual = std::fs::read_link(format!("/proc/{}/exe", browser.pid))
                    .map_err(|_| "browser_build_reuse_identity_unavailable")?;
                if actual != expected {
                    return Err("browser_build_reuse_conflict".into());
                }
            }
            host.effects
                .select_browser_executable(expected.to_string_lossy().into_owned())?;
        }

        if let (Some(id), Some(path)) = (
            optional_string(&command, "runtimeProfile"),
            optional_string(&command, "profile"),
        ) {
            let path = Path::new(path);
            if !path.is_absolute() || !path.is_dir() {
                return Err("browser_session_explicit_profile_path_invalid".into());
            }
            let path = path
                .canonicalize()
                .map_err(|_| "browser_session_explicit_profile_path_invalid")?;
            let path = path.to_string_lossy().into_owned();
            if let Some(existing) = host.catalog.profiles.get(id) {
                if existing.user_data_dir != path {
                    return Err("browser_session_profile_selector_conflict".into());
                }
            } else {
                if host
                    .catalog
                    .profiles
                    .values()
                    .any(|profile| profile.user_data_dir == path)
                {
                    return Err("browser_session_profile_selector_conflict".into());
                }
                host.catalog.profiles.insert(
                    id.into(),
                    agent_browser_service_model::BrowserProfileCatalogEntry {
                        id: id.into(),
                        name: id.into(),
                        user_data_dir: path,
                        kind: agent_browser_service_model::BrowserProfileKind::Named,
                    },
                );
                host.persistence.save_profile_catalog(&host.catalog)?;
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
                let actual = std::fs::read_link(format!("/proc/{}/exe", browser.pid))
                    .map_err(|_| "browser_build_process_identity_unavailable")?;
                if selected_executable
                    .as_ref()
                    .and_then(|path| path.canonicalize().ok())
                    .as_ref()
                    != Some(&actual)
                {
                    return Err("browser_build_executable_mismatch".into());
                }
                response["browserBuildProof"] = serde_json::json!({
                    "state":"matched", "requestedBrowserBuild":build,
                    "selectedBrowserBuild":build, "actualExecutablePath":actual,
                });
            }

            let handoff_id = required_string(&identity, "handoffId")?;
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
        if command["action"] == "tab_new" && !host.effects.admits_ordinary_sessions() {
            return Ok(None);
        }
        host.execute_managed_command(&name, &command)
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
        host.handle_command_result(&command).map(Some)
    })
    .await
    .map_err(|_| "browser_session_host_join_failed")?
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
    BrowserSessionHost::load(
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
    )
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
}

impl BrowserSessionPersistence for BrowserSessionSqliteStore {
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
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(crate) fn load(
        persistence: P,
        effects: E,
        legacy_service_state_path: &Path,
        config: BrowserSessionHostConfig,
    ) -> Result<Self, String> {
        let state = persistence.load_session_state()?;
        let mut catalog_load =
            persistence.load_or_import_profile_catalog(legacy_service_state_path)?;
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
            persistence.save_profile_catalog(&catalog_load.catalog)?;
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
        })
    }

    pub(crate) fn open(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        let result = self.manager()?.open(request)?;
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
        self.compact_navigation_history()?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn navigate(
        &mut self,
        session_id: &str,
        url: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserNavigationRecord, String> {
        let result = self.manager()?.navigate(session_id, url, activity_at_ms)?;
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

    pub(crate) fn reap(&mut self, now_ms: u64) -> Result<ReapBrowserSessionsResult, String> {
        let result = self.manager()?.reap(now_ms)?;
        self.commit_state()?;
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
        self.reap(current_unix_ms())
    }

    pub(crate) fn handle_command(&mut self, command: &serde_json::Value) -> serde_json::Value {
        let id = command
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result = self.handle_command_result(command);
        match result {
            Ok(data) => serde_json::json!({ "id": id, "success": true, "data": data }),
            Err(error) => serde_json::json!({ "id": id, "success": false, "error": error }),
        }
    }

    fn handle_command_result(
        &mut self,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let action = command
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "browser_session_action_missing".to_string())?;
        let now_ms = command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        match action {
            "service_remote_view_handoff_resolve" => {
                let handoff_id = required_string(command, "handoffId")?;
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
            "browser_session_close" => {
                let session_id = self.session_id_from_command(command)?;
                let closed =
                    self.close_session(&session_id, SessionEndReason::ExplicitClose, now_ms)?;
                Ok(serde_json::json!({
                    "closed": true,
                    "sessionId": closed.session_id,
                    "browserId": closed.browser_id,
                    "disposition": session_close_disposition_name(closed.disposition),
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
                    "quotaEvictedSessionIds": reaped.quota_evicted_session_ids,
                    "closedBrowserIds": reaped.closed_browser_ids,
                    "deletedDisposableProfileIds": reaped.deleted_disposable_profile_ids,
                }))
            }
            "browser_session_status" => serde_json::to_value(self.state())
                .map_err(|error| format!("browser_session_status_serialize_failed:{error}")),
            _ => Err(format!("browser_session_action_unsupported:{action}")),
        }
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

    fn compact_navigation_history(&mut self) -> Result<(), String> {
        self.state
            .compact_navigation_history(self.exact_url_history_maximum_bytes)?;
        Ok(())
    }

    fn commit_state(&mut self) -> Result<(), String> {
        let intent = self.effects.pending_launch_intent();
        self.persistence.publish_session_state(
            &self.persisted_state,
            &self.state,
            intent.as_ref(),
        )?;
        self.persisted_state = self.state.clone();
        self.effects.acknowledge_launch_publication();
        Ok(())
    }

    fn open_request_from_command(
        &mut self,
        command: &serde_json::Value,
        activity_at_ms: u64,
    ) -> Result<OpenBrowserSessionResult, String> {
        let session_name = required_string(command, "sessionName")?;
        let request = if let Some(profile_id) = optional_string(command, "profileId") {
            OpenBrowserSession::exact_profile(session_name, profile_id, activity_at_ms)
        } else {
            OpenBrowserSession::disposable(
                session_name,
                optional_string(command, "disposablePolicyId")
                    .unwrap_or(DEFAULT_DISPOSABLE_POLICY_ID),
                activity_at_ms,
            )
        };
        self.open(request)
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
    /// Admit ordinary requests through the configured Remote View owner, or
    /// route to an existing managed session. Only unconfigured, unowned lanes
    /// return None for legacy routing.
    pub(crate) fn execute_managed_command(
        &mut self,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let automatic = self.effects.admits_ordinary_sessions();
        if automatic && command["action"] == "tab_new" {
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
                    && profile_id.is_none_or(|profile_id| session.profile_id == profile_id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        let session_id = match matching_ids.as_slice() {
            [] if automatic => {
                if self.state.managed_tab_requests.values().any(|request| {
                    request.response.is_none()
                        && (request.session_name == session_name
                            || profile_id
                                .is_some_and(|id| request.profile_id.as_deref() == Some(id)))
                }) {
                    return Err("browser_session_tab_creation_readback_required".into());
                }
                let request = match profile_id {
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
            [] => return Ok(None),
            [session_id] => session_id.clone(),
            _ => return Err(format!("browser_session_name_ambiguous:{session_name}")),
        };
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
        // Retain the logical target before returning an identity to a client.
        // Provider route issuance and the operator URL are separate joins.
        let handoff = if automatic {
            let id = self
                .state
                .remote_view_tab_handoffs
                .values()
                .find(|record| record.session_id == session.id && record.tab_id == tab.id)
                .map(|record| record.id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let record = agent_browser_service_model::retain_remote_view_tab_handoff(
                &mut self.state,
                &id,
                &session.id,
                &tab.id,
            )?;
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
        if command.get("action").and_then(serde_json::Value::as_str) == Some("navigate")
            && response.get("success").and_then(serde_json::Value::as_bool) == Some(true)
        {
            if let Some(url) = optional_string(command, "url") {
                self.record_navigation(&session.id, url, now_ms)?;
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
                    identity["handoffUrl"] =
                        serde_json::json!(format!("/remote-view/{}", handoff.id));
                }
            }
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
            let handoff = response
                .pointer("/browserSession/handoffId")
                .and_then(serde_json::Value::as_str)
                .ok_or("browser_session_tab_request_invalid")?;
            agent_browser_service_model::resolve_remote_view_tab_handoff(&self.state, handoff)
                .map_err(|_| "browser_session_tab_request_target_closed")?;
            let mut response = response;
            response["id"] = serde_json::json!(response_id);
            return Ok(response);
        }
        let profile = self.ordinary_profile_selector(command)?;
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
                    && profile.as_ref().is_none_or(|id| &session.profile_id == id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        let existing = match matches.as_slice() {
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
        self.state
            .managed_tab_requests
            .get_mut(id)
            .ok_or("browser_session_tab_request_missing")?
            .response = Some(response.clone());
        self.commit_state()?;
        Ok(response)
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
        live: bool,
        next_tab: usize,
        pending_intent: Option<BrowserLaunchIntent>,
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
            Ok(self.live)
        }

        fn launch_browser(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::RemoteViewFixedDesktop>,
        ) -> Result<BrowserLaunch, String> {
            self.launches += 1;
            Ok(BrowserLaunch {
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
            _browser: &ManagedBrowserInstance,
            _attributed_target_ids: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
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
        assert!(peer
            .compare_and_save_session_state(&initial, &host.state)
            .is_err());
        assert_eq!(peer.load_session_state().unwrap(), initial);
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
        assert_eq!(response["success"], true);
        assert_eq!(response["data"]["sessionId"], first.session_id);
        assert_eq!(response["data"]["disposition"], "browser_closed");
        assert!(restarted.state().sessions.is_empty());

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
