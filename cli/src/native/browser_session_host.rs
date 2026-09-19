//! Durable host for the ordinary Browser Session Manager path.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_browser_service_model::{
    BrowserDesktopRoute, BrowserDisposableProfilePolicy, BrowserOpenReservation,
    BrowserProfileCatalog, BrowserSessionEffects, BrowserSessionManager,
    BrowserSessionManagerConfig, BrowserSessionState, CloseBrowserSessionResult,
    CloseBrowserTabResult, OpenBrowserSession, OpenBrowserSessionResult, ReapBrowserSessionsResult,
    RemoteViewHandoff, RouteKeeperAuthority, RouteKeeperPhase, ServiceState, SessionEndReason,
};
use sha2::{Digest, Sha256};

use super::browser_session_runtime::{
    BrowserManagerRuntime, BrowserManagerRuntimeConfig, BrowserSessionEffectAdapter,
    ManagedBrowserCommandEffects, ReservedBrowserRecovery, ReservedBrowserRecoveryEffects,
};
use super::browser_session_store::{
    BrowserManagerHandoffRegistry, BrowserProfileCatalogLoad, BrowserRuntimeOperation,
    BrowserRuntimeOperationState, BrowserRuntimeSqliteStore, BrowserSessionJsonStore,
};
use super::presentation_inventory::StaticRouteInventory;

const DEFAULT_DISPOSABLE_POLICY_ID: &str = "default";

pub(crate) type DefaultBrowserSessionHost = BrowserSessionHost<
    BrowserRuntimeSqliteStore,
    BrowserSessionEffectAdapter<BrowserManagerRuntime>,
>;

pub(crate) fn load_default_browser_session_host() -> Result<DefaultBrowserSessionHost, String> {
    let legacy_state_path = super::service_store::default_service_state_path()?;
    let store = BrowserRuntimeSqliteStore::default_sqlite()?;
    let runtime_config = store.load_runtime_config()?;
    let disposable_root = legacy_state_path
        .parent()
        .ok_or_else(|| "browser_session_service_directory_missing".to_string())?
        .join("disposable-profiles");
    let display = std::env::var("AGENT_BROWSER_SESSION_DISPLAY").ok();
    let remote_desktop_routes = if display.is_some() {
        Vec::new()
    } else {
        route_keeper_desktop_routes(&store.load_route_keeper_authority()?)?
    };
    let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
        headless: display.is_none(),
        executable_path: std::env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok(),
        display,
        remote_headed: false,
    })?;
    BrowserSessionHost::load(
        store,
        BrowserSessionEffectAdapter::new(runtime),
        &legacy_state_path,
        BrowserSessionHostConfig {
            session_idle_timeout_ms: runtime_config.session_idle_timeout_ms,
            remote_desktop_routes,
            default_disposable_policy: Some(BrowserDisposableProfilePolicy {
                id: DEFAULT_DISPOSABLE_POLICY_ID.to_string(),
                user_data_root: disposable_root.to_string_lossy().into_owned(),
                cleanup_delay_ms: runtime_config.disposable_inactivity_ms,
            }),
        },
    )
}

pub(crate) fn load_current_remote_desktop_routes() -> Result<Vec<BrowserDesktopRoute>, String> {
    let store = BrowserRuntimeSqliteStore::default_sqlite()?;
    route_keeper_desktop_routes(&store.load_route_keeper_authority()?)
}

fn route_keeper_desktop_routes(
    authority: &RouteKeeperAuthority,
) -> Result<Vec<BrowserDesktopRoute>, String> {
    authority.projection()?;
    Ok(authority
        .records
        .values()
        .filter_map(|record| {
            if record.phase != RouteKeeperPhase::Ready {
                return None;
            }
            let ready = record.protocol_ready.as_ref()?;
            Some(BrowserDesktopRoute {
                id: record.slot_id.clone(),
                display_name: ready.display_name.clone(),
                healthy: true,
            })
        })
        .collect())
}

pub(crate) fn browser_session_navigation_requires_handoff(
    command: &serde_json::Value,
    _runtime_environment: Option<&str>,
) -> Result<bool, String> {
    if command
        .get("internalPresentationBootstrap")
        .is_some_and(|value| value != &serde_json::Value::Bool(false))
    {
        return Err("internal_presentation_bootstrap_removed".to_string());
    }
    Ok(command.get("action").and_then(serde_json::Value::as_str)
        == Some("browser_session_navigate"))
}

pub(crate) trait BrowserSessionPersistence {
    fn load_session_state(&self) -> Result<BrowserSessionState, String>;
    fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String>;
    fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String>;
    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String>;

    fn load_manager_handoffs(&self) -> Result<BTreeMap<String, RemoteViewHandoff>, String> {
        Ok(BTreeMap::new())
    }

    fn reserve_operation(
        &mut self,
        _operation_id: &str,
        _owner_key: &str,
        _request: serde_json::Value,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        Ok(None)
    }

    fn find_operation(
        &self,
        _operation_id: &str,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        Ok(None)
    }

    fn record_operation_observation(
        &mut self,
        _operation_id: &str,
        _generation: u64,
        _observation: serde_json::Value,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        Ok(None)
    }

    fn commit_browser_open(
        &mut self,
        _operation_id: &str,
        _generation: u64,
        _expected_base_state: &BrowserSessionState,
        _state: &BrowserSessionState,
        _handoff: &RemoteViewHandoff,
        _result: serde_json::Value,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        Ok(None)
    }
}

impl BrowserSessionPersistence for BrowserSessionJsonStore {
    fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        BrowserSessionJsonStore::load_session_state(self)
    }

    fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String> {
        BrowserSessionJsonStore::save_session_state(self, state)
    }

    fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        BrowserSessionJsonStore::load_or_import_profile_catalog(self, legacy_service_state_path)
    }

    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String> {
        BrowserSessionJsonStore::save_profile_catalog(self, catalog)
    }
}

impl BrowserSessionPersistence for BrowserRuntimeSqliteStore {
    fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        BrowserRuntimeSqliteStore::load_session_state(self)
    }

    fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String> {
        BrowserRuntimeSqliteStore::save_session_state(self, state)
    }

    fn load_or_import_profile_catalog(
        &self,
        _legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        Ok(BrowserProfileCatalogLoad {
            catalog: BrowserRuntimeSqliteStore::load_profile_catalog(self)?,
            diagnostics: Vec::new(),
            imported_legacy_profiles: false,
        })
    }

    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String> {
        BrowserRuntimeSqliteStore::save_profile_catalog(self, catalog)
    }

    fn load_manager_handoffs(&self) -> Result<BTreeMap<String, RemoteViewHandoff>, String> {
        BrowserRuntimeSqliteStore::load_handoff_registry(self)
            .map(|registry: BrowserManagerHandoffRegistry| registry.handoffs)
    }

    fn reserve_operation(
        &mut self,
        operation_id: &str,
        owner_key: &str,
        request: serde_json::Value,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        BrowserRuntimeSqliteStore::reserve_operation(self, operation_id, owner_key, request)
            .map(Some)
    }

    fn find_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        BrowserRuntimeSqliteStore::find_operation(self, operation_id)
    }

    fn record_operation_observation(
        &mut self,
        operation_id: &str,
        generation: u64,
        observation: serde_json::Value,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        BrowserRuntimeSqliteStore::record_operation_observation(
            self,
            operation_id,
            generation,
            observation,
        )
        .map(Some)
    }

    fn commit_browser_open(
        &mut self,
        operation_id: &str,
        generation: u64,
        expected_base_state: &BrowserSessionState,
        state: &BrowserSessionState,
        handoff: &RemoteViewHandoff,
        result: serde_json::Value,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        BrowserRuntimeSqliteStore::commit_browser_open(
            self,
            operation_id,
            generation,
            expected_base_state,
            state,
            handoff,
            result,
        )
        .map(Some)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSessionHostConfig {
    pub(crate) session_idle_timeout_ms: u64,
    pub(crate) remote_desktop_routes: Vec<BrowserDesktopRoute>,
    pub(crate) default_disposable_policy: Option<BrowserDisposableProfilePolicy>,
}

pub(crate) struct BrowserSessionHost<P, E> {
    persistence: P,
    effects: E,
    catalog: BrowserProfileCatalog,
    state: BrowserSessionState,
    handoffs: BTreeMap<String, RemoteViewHandoff>,
    manager_config: BrowserSessionManagerConfig,
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(crate) fn load(
        persistence: P,
        effects: E,
        legacy_service_state_path: &Path,
        config: BrowserSessionHostConfig,
    ) -> Result<Self, String> {
        let state = persistence.load_session_state()?;
        let handoffs = persistence.load_manager_handoffs()?;
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
            state,
            handoffs,
            manager_config: BrowserSessionManagerConfig {
                session_idle_timeout_ms: config.session_idle_timeout_ms,
                remote_desktop_routes: config.remote_desktop_routes,
            },
        })
    }

    /// Replace route choices for future browser launches without changing any
    /// persisted browser desktop assignment or existing session custody.
    pub(crate) fn replace_remote_desktop_routes(&mut self, routes: Vec<BrowserDesktopRoute>) {
        self.manager_config.remote_desktop_routes = routes;
    }

    pub(crate) fn open(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        let result = self.manager().open(request)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn tab_for_navigation(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserTabAcquisition, String> {
        let result = self
            .manager()
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
            .manager()
            .record_navigation(session_id, url, visited_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn navigate(
        &mut self,
        session_id: &str,
        url: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserNavigationRecord, String> {
        let result = self.manager().navigate(session_id, url, activity_at_ms)?;
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
            .manager()
            .focus_browser(browser_id, target_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    /// Resolve one manager-owned opaque handoff while this host has exclusive
    /// custody of the manager state and its browser effects.
    pub(crate) fn resolve_manager_handoff(
        &mut self,
        handoff: &RemoteViewHandoff,
        service: &ServiceState,
        inventory: &StaticRouteInventory,
        activity_at_ms: u64,
    ) -> Result<serde_json::Value, String> {
        if !super::browser_session_handoff::is_manager_handoff(handoff) {
            return Err("browser_session_handoff_not_manager_owned".to_string());
        }
        if handoff.state != "ready" {
            return Err("browser_session_handoff_not_ready".to_string());
        }
        let session_id = handoff
            .intent
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "browser_session_handoff_session_id_missing".to_string())?;
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| "browser_session_handoff_session_ended".to_string())?;
        if session.expires_at_ms < activity_at_ms {
            return Err("browser_session_handoff_session_expired".to_string());
        }
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_handoff_browser_missing".to_string())?;
        let tab_id = handoff
            .tab_id
            .as_deref()
            .ok_or_else(|| "browser_session_handoff_tab_missing".to_string())?;
        let tab = self
            .state
            .tabs
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_handoff_tab_closed".to_string())?;
        if handoff.profile_id.as_deref() != Some(session.profile_id.as_str())
            || handoff.browser_id.as_deref() != Some(browser.id.as_str())
            || handoff.session_name.as_deref() != Some(session.name.as_str())
            || handoff.target_id.as_deref() != Some(tab.target_id.as_str())
            || tab.browser_id != browser.id
            || tab.session_id != session.id
        {
            return Err("browser_session_handoff_identity_changed".to_string());
        }
        let desktop = browser
            .desktop
            .as_ref()
            .ok_or_else(|| "browser_session_handoff_desktop_missing".to_string())?;
        let binding = super::browser_session_handoff::manager_route_binding(
            service,
            inventory,
            &browser.id,
            &desktop.route_id,
            &desktop.display_name,
        )?;
        let control_input = service
            .remote_view_routes
            .get(&binding.route_id)
            .and_then(|route| route.control_input)
            .ok_or_else(|| "browser_session_handoff_control_input_unavailable".to_string())?;
        if handoff
            .view_stream_provider
            .is_some_and(|provider| provider != binding.provider)
            || handoff
                .control_input
                .is_some_and(|provider| provider != control_input)
        {
            return Err("browser_session_handoff_presentation_identity_changed".to_string());
        }
        let handoff_url =
            super::remote_view_handoff::durable_remote_view_handoff_url(&binding, &handoff.id)
                .ok_or_else(|| "browser_session_handoff_public_operator_url_missing".to_string())?;
        if handoff.handoff_url.as_deref() != Some(handoff_url.as_str()) {
            return Err("browser_session_handoff_opaque_url_changed".to_string());
        }

        let focused = self.focus_browser(&browser.id, Some(&tab.target_id), activity_at_ms)?;
        if focused.tab_id.as_deref() != Some(tab.id.as_str())
            || focused.target_id.as_deref() != Some(tab.target_id.as_str())
        {
            return Err("browser_session_handoff_focus_identity_changed".to_string());
        }
        let presentation_generation = 1_u64;
        let presentation_receipt = serde_json::json!({
            "generation": presentation_generation,
            "logicalBrowserId": browser.id,
            "targetId": tab.target_id,
            "requiredStreamProvider": binding.provider,
            "observedStreamProvider": binding.provider,
            "state": "ready",
            "browserSessionManager": true,
        });
        Ok(serde_json::json!({
            "status": "ready",
            "resolved": true,
            "browserSessionManager": true,
            "handoffId": handoff.id,
            "handoffUrl": handoff_url,
            "browserId": browser.id,
            "sessionName": session.name,
            "tabId": tab.id,
            "targetId": tab.target_id,
            "viewStreamProvider": binding.provider,
            "requiredViewStreamProvider": binding.provider,
            "controlInput": control_input,
            "operatorVisible": { "state": "ready" },
            "presentationGeneration": presentation_generation,
            "presentationReceipt": presentation_receipt,
        }))
    }

    pub(crate) fn new_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserTabAcquisition, String> {
        let result = self.manager().new_tab(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn close_current_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<CloseBrowserTabResult, String> {
        let result = self
            .manager()
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
            .manager()
            .close_session(session_id, reason, ended_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn reap(&mut self, now_ms: u64) -> Result<ReapBrowserSessionsResult, String> {
        let result = self.manager().reap(now_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn state(&self) -> &BrowserSessionState {
        &self.state
    }

    pub(crate) fn manager_handoff(&self, handoff_id: &str) -> Option<&RemoteViewHandoff> {
        self.handoffs.get(handoff_id)
    }

    pub(crate) fn has_operation(&self, operation_id: &str) -> Result<bool, String> {
        self.persistence
            .find_operation(operation_id)
            .map(|operation| operation.is_some())
    }

    pub(crate) fn reap_current(&mut self) -> Result<ReapBrowserSessionsResult, String> {
        self.reap(current_unix_ms())
    }

    pub(crate) fn reconcile_liveness_current(&mut self) -> Result<Vec<String>, String> {
        let retired = self.manager().reconcile_liveness(current_unix_ms())?;
        if !retired.is_empty() {
            self.commit_state()?;
        }
        Ok(retired)
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

    /// Execute one fresh browser open through the SQLite operation journal.
    ///
    /// The operation reserves its logical session, browser, desktop, and
    /// handoff intent before browser effects. Browser and tab observations are
    /// checkpointed for restart recovery, while the ready manager state and
    /// logical handoff publish in one fenced SQLite transaction.
    pub(crate) fn handle_journaled_open_with_handoff(
        &mut self,
        command: &serde_json::Value,
        service: &ServiceState,
        inventory: &StaticRouteInventory,
    ) -> serde_json::Value
    where
        E: ReservedBrowserRecoveryEffects,
    {
        let id = command
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let baseline_state = self.state.clone();
        let result = self.journaled_open_with_handoff_result(command, service, inventory);
        match result {
            Ok(response) => response,
            Err(error) => {
                self.state = self
                    .persistence
                    .load_session_state()
                    .unwrap_or(baseline_state);
                serde_json::json!({ "id": id, "success": false, "error": error })
            }
        }
    }

    fn journaled_open_with_handoff_result(
        &mut self,
        command: &serde_json::Value,
        service: &ServiceState,
        inventory: &StaticRouteInventory,
    ) -> Result<serde_json::Value, String>
    where
        E: ReservedBrowserRecoveryEffects,
    {
        if command.get("action").and_then(serde_json::Value::as_str) != Some("browser_session_open")
        {
            return Err("browser_runtime_open_action_invalid".to_string());
        }
        let operation_id = required_string(command, "id")?;
        let session_name = required_string(command, "sessionName")?;
        let profile_id = required_string(command, "profileId")?;
        let activity_at_ms = command
            .get("activityAtMs")
            .or_else(|| {
                command
                    .get("params")
                    .and_then(|params| params.get("activityAtMs"))
            })
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        let owner_key = "browser-runtime-open";
        let existing_operation = self.persistence.find_operation(operation_id)?;
        let request = if let Some(existing) = &existing_operation {
            if existing.request.get("command") != Some(command) {
                return Err(format!(
                    "browser_runtime_operation_replay_mismatch:{operation_id}"
                ));
            }
            existing.request.clone()
        } else {
            let matching_session = self.state.sessions.values().find(|session| {
                session.name == session_name
                    && session.profile_id == profile_id
                    && activity_at_ms < session.expires_at_ms
            });
            let reusable_browser = matching_session
                .and_then(|session| self.state.browsers.get(&session.browser_id))
                .or_else(|| {
                    self.state
                        .browsers
                        .values()
                        .find(|browser| browser.profile_id == profile_id)
                });
            let live_display_names = self
                .state
                .browsers
                .values()
                .filter_map(|browser| {
                    browser
                        .desktop
                        .as_ref()
                        .map(|desktop| desktop.display_name.clone())
                })
                .collect::<Vec<_>>();
            let desktop_intent = if let Some(browser) = reusable_browser {
                browser
                    .desktop
                    .clone()
                    .ok_or_else(|| "browser_session_open_desktop_missing".to_string())?
            } else {
                agent_browser_service_model::select_least_crowded_browser_desktop(
                    &self.manager_config.remote_desktop_routes,
                    &live_display_names,
                )?
            };
            let session_id = matching_session
                .map(|session| session.id.clone())
                .unwrap_or_else(|| {
                    format!(
                        "session:{session_name}:{profile_id}:{}",
                        self.state.next_session_sequence.saturating_add(1)
                    )
                });
            let browser_id = reusable_browser
                .map(|browser| browser.id.clone())
                .unwrap_or_else(|| deterministic_manager_browser_id(operation_id, profile_id));
            let handoff_id = deterministic_manager_handoff_id(operation_id);
            serde_json::json!({
                "schemaVersion": "agent-browser.browser-open-operation.v1",
                "command": command,
                "baseSessionState": &self.state,
                "intent": {
                    "session": {"id": session_id, "name": session_name, "profileId": profile_id},
                    "browser": {"id": browser_id, "profileId": profile_id, "disposition": "reuse_or_launch"},
                    "slot": {
                        "routeId": desktop_intent.route_id,
                        "displayName": desktop_intent.display_name,
                        "liveBrowserCount": desktop_intent.live_browser_count,
                    },
                    "handoff": {"id": handoff_id, "state": "pending"},
                }
            })
        };
        let base_session_state: BrowserSessionState = serde_json::from_value(
            request
                .get("baseSessionState")
                .cloned()
                .ok_or_else(|| "browser_runtime_operation_base_state_missing".to_string())?,
        )
        .map_err(|error| format!("browser_runtime_operation_base_state_invalid:{error}"))?;
        if existing_operation
            .as_ref()
            .is_some_and(|operation| operation.state != BrowserRuntimeOperationState::Committed)
            && self.state != base_session_state
        {
            return Err("browser_runtime_operation_base_state_conflict".to_string());
        }
        let handoff_id = request["intent"]["handoff"]["id"]
            .as_str()
            .ok_or_else(|| "browser_runtime_operation_handoff_id_missing".to_string())?
            .to_string();
        let mut operation = self
            .persistence
            .reserve_operation(operation_id, owner_key, request)?
            .ok_or_else(|| "browser_runtime_operation_persistence_unsupported".to_string())?;
        let mut launch_authorized = existing_operation
            .as_ref()
            .is_none_or(|operation| operation.state == BrowserRuntimeOperationState::Prepared);

        loop {
            match operation.state {
                BrowserRuntimeOperationState::Committed => {
                    return observation_response(operation.result.as_ref().ok_or_else(|| {
                        "browser_runtime_operation_committed_result_missing".to_string()
                    })?);
                }
                BrowserRuntimeOperationState::Prepared => {
                    let response = serde_json::json!({
                        "id": command.get("id").cloned().unwrap_or_default(),
                        "success": true,
                        "data": {
                            "sessionId": operation.request["intent"]["session"]["id"],
                            "sessionName": session_name,
                            "profileId": profile_id,
                            "browserId": operation.request["intent"]["browser"]["id"],
                            "handoffId": handoff_id,
                        }
                    });
                    operation =
                        self.record_open_observation(&operation, "launch_started", response, None)?;
                }
                BrowserRuntimeOperationState::Observed => {
                    let observation = operation.result.as_ref().ok_or_else(|| {
                        "browser_runtime_operation_observation_missing".to_string()
                    })?;
                    self.state = serde_json::from_value(
                        observation.get("sessionState").cloned().ok_or_else(|| {
                            "browser_runtime_operation_session_state_missing".to_string()
                        })?,
                    )
                    .map_err(|error| {
                        format!("browser_runtime_operation_session_state_invalid:{error}")
                    })?;
                    let phase = observation
                        .get("phase")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            "browser_runtime_operation_observation_phase_missing".to_string()
                        })?;
                    let response = observation_response(observation)?;
                    match phase {
                        "launch_started" => {
                            let reservation = BrowserOpenReservation {
                                session_id: operation.request["intent"]["session"]["id"]
                                    .as_str()
                                    .ok_or_else(|| {
                                        "browser_runtime_operation_session_id_missing".to_string()
                                    })?
                                    .to_string(),
                                browser_id: operation.request["intent"]["browser"]["id"]
                                    .as_str()
                                    .ok_or_else(|| {
                                        "browser_runtime_operation_browser_id_missing".to_string()
                                    })?
                                    .to_string(),
                                desktop: serde_json::from_value(
                                    operation.request["intent"]["slot"].clone(),
                                )
                                .map_err(|error| {
                                    format!(
                                        "browser_runtime_operation_desktop_intent_invalid:{error}"
                                    )
                                })?,
                            };
                            let opened = if launch_authorized {
                                self.open_request_from_command_reserved(
                                    command,
                                    activity_at_ms,
                                    reservation,
                                )?
                            } else {
                                let profile =
                                    self.catalog.profiles.get(profile_id).cloned().ok_or_else(
                                        || format!("browser_profile_not_found:{profile_id}"),
                                    )?;
                                match self.effects.recover_browser_reserved(
                                    &profile,
                                    &reservation.desktop,
                                    &reservation.browser_id,
                                ) {
                                    Ok(ReservedBrowserRecovery::Recovered(launch)) => {
                                        match self.manager().open_reserved_observed(
                                            OpenBrowserSession::exact_profile(
                                                session_name,
                                                profile_id,
                                                activity_at_ms,
                                            ),
                                            reservation,
                                            launch,
                                        ) {
                                            Ok(opened) => opened,
                                            Err(error) => {
                                                self.retain_reserved_browser_cleanup(
                                                    &operation,
                                                    response,
                                                    &operation.request,
                                                    format!("recovered_launch_rejected:{error}"),
                                                )?;
                                                return Err("browser_runtime_open_reserved_browser_recovery_unproven".to_string());
                                            }
                                        }
                                    }
                                    Ok(ReservedBrowserRecovery::Unproven { reason }) => {
                                        self.retain_reserved_browser_cleanup(
                                            &operation,
                                            response,
                                            &operation.request,
                                            reason,
                                        )?;
                                        return Err("browser_runtime_open_reserved_browser_recovery_unproven".to_string());
                                    }
                                    Err(error) => {
                                        self.retain_reserved_browser_cleanup(
                                            &operation,
                                            response,
                                            &operation.request,
                                            format!("recovery_probe_failed:{error}"),
                                        )?;
                                        return Err("browser_runtime_open_reserved_browser_recovery_unproven".to_string());
                                    }
                                }
                            };
                            let mut response = response;
                            response["data"]["browserDisposition"] =
                                serde_json::json!(
                                    format!("{:?}", opened.disposition).to_lowercase()
                                );
                            response["data"]["sessionDisposition"] =
                                serde_json::json!(
                                    format!("{:?}", opened.session_disposition).to_lowercase()
                                );
                            operation = self.record_open_observation(
                                &operation,
                                "browser_opened",
                                response,
                                None,
                            )?;
                            launch_authorized = false;
                        }
                        "launch_cleanup_required" => {
                            return Err("browser_runtime_open_reserved_browser_recovery_unproven"
                                .to_string());
                        }
                        "browser_opened" => {
                            let now_ms = command
                                .get("activityAtMs")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or_else(current_unix_ms);
                            self.manager().reconcile_liveness(now_ms)?;
                            let session_id =
                                response["data"]["sessionId"].as_str().ok_or_else(|| {
                                    "browser_runtime_operation_session_id_missing".to_string()
                                })?;
                            let tab = self.manager().tab_for_navigation(session_id, now_ms)?;
                            let mut response = response;
                            response["data"]["tabId"] = serde_json::json!(tab.tab_id);
                            response["data"]["targetId"] = serde_json::json!(tab.target_id);
                            operation = self.record_open_observation(
                                &operation,
                                "tab_acquired",
                                response,
                                None,
                            )?;
                        }
                        "tab_acquired" => {
                            let prepared = super::browser_session_handoff::prepare_manager_handoff(
                                &response,
                                &self.state,
                                service,
                                inventory,
                                &self.handoffs,
                            )?;
                            let mut response = response;
                            if let (Some(data), Some(projection)) = (
                                response
                                    .get_mut("data")
                                    .and_then(serde_json::Value::as_object_mut),
                                prepared.projection.as_json().as_object(),
                            ) {
                                for (key, value) in projection {
                                    data.insert(key.clone(), value.clone());
                                }
                            }
                            operation = self.record_open_observation(
                                &operation,
                                "ready",
                                response,
                                Some(&prepared.handoff),
                            )?;
                        }
                        "ready" => {
                            let handoff: RemoteViewHandoff = serde_json::from_value(
                                observation.get("handoff").cloned().ok_or_else(|| {
                                    "browser_runtime_operation_handoff_missing".to_string()
                                })?,
                            )
                            .map_err(|error| {
                                format!("browser_runtime_operation_handoff_invalid:{error}")
                            })?;
                            let committed = self
                                .persistence
                                .commit_browser_open(
                                    &operation.operation_id,
                                    operation.generation,
                                    &base_session_state,
                                    &self.state,
                                    &handoff,
                                    observation.clone(),
                                )?
                                .ok_or_else(|| {
                                    "browser_runtime_operation_persistence_unsupported".to_string()
                                })?;
                            self.handoffs.insert(handoff.id.clone(), handoff);
                            operation = committed;
                        }
                        other => {
                            return Err(format!(
                                "browser_runtime_operation_observation_phase_invalid:{other}"
                            ));
                        }
                    }
                }
            }
        }
    }

    fn open_request_from_command_reserved(
        &mut self,
        command: &serde_json::Value,
        activity_at_ms: u64,
        reservation: BrowserOpenReservation,
    ) -> Result<OpenBrowserSessionResult, String> {
        let session_name = required_string(command, "sessionName")?;
        let profile_id = required_string(command, "profileId")?;
        self.manager().open_reserved(
            OpenBrowserSession::exact_profile(session_name, profile_id, activity_at_ms),
            reservation,
        )
    }

    fn record_open_observation(
        &mut self,
        operation: &BrowserRuntimeOperation,
        phase: &str,
        response: serde_json::Value,
        handoff: Option<&RemoteViewHandoff>,
    ) -> Result<BrowserRuntimeOperation, String> {
        let observation = serde_json::json!({
            "phase": phase,
            "sessionState": &self.state,
            "handoff": handoff,
            "response": response,
        });
        self.persistence
            .record_operation_observation(
                &operation.operation_id,
                operation.generation,
                observation,
            )?
            .ok_or_else(|| "browser_runtime_operation_persistence_unsupported".to_string())
    }

    fn record_open_cleanup_observation(
        &mut self,
        operation: &BrowserRuntimeOperation,
        response: serde_json::Value,
        obligation: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        let observation = serde_json::json!({
            "phase": "launch_cleanup_required",
            "sessionState": &self.state,
            "handoff": null,
            "cleanupObligation": obligation,
            "response": response,
        });
        self.persistence
            .record_operation_observation(
                &operation.operation_id,
                operation.generation,
                observation,
            )?
            .ok_or_else(|| "browser_runtime_operation_persistence_unsupported".to_string())
    }

    fn retain_reserved_browser_cleanup(
        &mut self,
        operation: &BrowserRuntimeOperation,
        response: serde_json::Value,
        request: &serde_json::Value,
        reason: String,
    ) -> Result<(), String> {
        let obligation = serde_json::json!({
            "kind": "reserved_browser_reconciliation",
            "state": "pending",
            "operationId": operation.operation_id,
            "generation": operation.generation,
            "browserId": request["intent"]["browser"]["id"],
            "profileId": request["intent"]["browser"]["profileId"],
            "routeId": request["intent"]["slot"]["routeId"],
            "displayName": request["intent"]["slot"]["displayName"],
            "reason": reason,
        });
        self.record_open_cleanup_observation(operation, response, obligation)?;
        Ok(())
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
                    "closedBrowserIds": reaped.closed_browser_ids,
                    "deletedDisposableProfileIds": reaped.deleted_disposable_profile_ids,
                }))
            }
            "browser_session_status" => serde_json::to_value(self.state())
                .map_err(|error| format!("browser_session_status_serialize_failed:{error}")),
            _ => Err(format!("browser_session_action_unsupported:{action}")),
        }
    }

    fn manager(&mut self) -> BrowserSessionManager<'_, E> {
        BrowserSessionManager::new(
            &mut self.state,
            &self.catalog,
            &mut self.effects,
            self.manager_config.clone(),
        )
    }

    fn commit_state(&self) -> Result<(), String> {
        self.persistence.save_session_state(&self.state)
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
    /// Open or reuse a managed session, then run a header-bearing navigation
    /// through the ordinary command executor bound to the manager-owned tab.
    pub(crate) fn handle_managed_navigation_command(
        &mut self,
        command: &serde_json::Value,
    ) -> serde_json::Value {
        let id = command
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result = (|| -> Result<serde_json::Value, String> {
            let now_ms = command
                .get("activityAtMs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_else(current_unix_ms);
            let opened = self.open_request_from_command(command, now_ms)?;
            let tab = self.tab_for_navigation(&opened.session_id, now_ms)?;
            let session = self
                .state
                .sessions
                .get(&opened.session_id)
                .cloned()
                .ok_or_else(|| format!("browser_session_not_found:{}", opened.session_id))?;
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
            let url = required_string(command, "url")?;
            let mut navigate_command = command.clone();
            navigate_command["action"] = serde_json::json!("navigate");
            let response = self.effects.execute_command(
                &browser,
                &tab,
                &session.id,
                &session.name,
                &navigate_command,
            )?;
            if response.get("success").and_then(serde_json::Value::as_bool) != Some(true) {
                return Err(response
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("browser_session_navigation_failed")
                    .to_string());
            }
            let navigation = self.record_navigation(&session.id, url, now_ms)?;
            Ok(serde_json::json!({
                "sessionId": session.id,
                "profileId": navigation.profile_id,
                "browserId": navigation.browser_id,
                "tabId": navigation.tab_id,
                "targetId": navigation.target_id,
                "url": navigation.url,
                "visitedAtMs": navigation.visited_at_ms,
            }))
        })();
        match result {
            Ok(data) => serde_json::json!({ "id": id, "success": true, "data": data }),
            Err(error) => serde_json::json!({ "id": id, "success": false, "error": error }),
        }
    }

    /// Route an ordinary browser command through an already active managed
    /// session. `Ok(None)` means this lane has no manager-owned session and the
    /// caller may continue through the legacy lane.
    pub(crate) fn execute_managed_command(
        &mut self,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let profile_id = optional_string(command, "profileId")
            .or_else(|| optional_string(command, "runtimeProfile"));
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
            [] => return Ok(None),
            [session_id] => session_id.clone(),
            _ => return Err(format!("browser_session_name_ambiguous:{session_name}")),
        };
        let now_ms = command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
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
        self.effects
            .execute_command(&browser, &tab, &session.id, &session.name, command)
            .map(Some)
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

fn deterministic_manager_handoff_id(operation_id: &str) -> String {
    let digest = Sha256::digest(operation_id.as_bytes());
    let suffix = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("manager-{suffix}")
}

fn deterministic_manager_browser_id(operation_id: &str, profile_id: &str) -> String {
    let digest = Sha256::digest(format!("{operation_id}\0{profile_id}").as_bytes());
    let suffix = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("browser:{profile_id}:operation:{suffix}")
}

fn observation_response(observation: &serde_json::Value) -> Result<serde_json::Value, String> {
    observation
        .get("response")
        .cloned()
        .ok_or_else(|| "browser_runtime_operation_response_missing".to_string())
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
        BrowserRuntimeDriver, BrowserSessionEffectAdapter, ReservedBrowserRecovery,
    };
    use crate::native::browser_session_store::{
        BrowserRuntimeSqliteStore, LegacyBrowserRuntimeSources,
    };
    use agent_browser_service_model::{
        BrowserLaunch, BrowserProfileCatalogEntry, BrowserTabAcquisition, ControlInputProvider,
        DisplayAllocation, ManagedBrowserInstance, ManagedBrowserTab, RemoteViewHandoff,
        RemoteViewRoute, RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog,
        RouteKeeperReconcileAction, RouteKeeperStartPriority, RoutePoolEntry, ServiceState,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn legacy_internal_presentation_bootstrap_is_removed() {
        let legacy = serde_json::json!({
            "action": "browser_session_navigate",
            "sessionName": "development-presentation-provider-v5-1",
            "profileId": "development-presentation-provider-v5-1",
            "internalPresentationBootstrap": true
        });
        assert_eq!(
            browser_session_navigation_requires_handoff(&legacy, Some("development")),
            Err("internal_presentation_bootstrap_removed".to_string())
        );
        assert_eq!(
            browser_session_navigation_requires_handoff(
                &serde_json::json!({"action": "browser_session_navigate"}),
                Some("development")
            ),
            Ok(true)
        );
    }

    #[test]
    fn desktop_route_projection_uses_only_ready_sqlite_keeper_receipts() {
        let mut authority = RouteKeeperAuthority::new(4).unwrap();
        authority
            .replace_connection_catalog(
                RouteKeeperConnectionCatalog::new([RouteKeeperConnectionBinding {
                    slot_id: "route-slot-01".to_string(),
                    connection_key: "route-01".to_string(),
                    connection_name: "Agent Browser Route 01".to_string(),
                    guacamole_connection_id: 1,
                }])
                .unwrap(),
            )
            .unwrap();
        let (slot_id, keeper_id, fence) = match authority.next_reconcile_action().unwrap() {
            RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                priority: RouteKeeperStartPriority::Minimum,
            } => (slot_id, keeper_id, fence),
            other => panic!("expected minimum start, got {other:?}"),
        };
        authority
            .record_protocol_ready(
                agent_browser_service_model::RouteKeeperProtocolReadyReceipt {
                    slot_id: slot_id.clone(),
                    keeper_id,
                    fence,
                    guacamole_connection_uuid: "guacamole-01".to_string(),
                    xrdp_session_id: "xrdp-01".to_string(),
                    display_name: ":10".to_string(),
                    observed_at: "2026-09-19T22:00:00Z".to_string(),
                },
            )
            .unwrap();

        assert_eq!(
            route_keeper_desktop_routes(&authority).unwrap(),
            vec![BrowserDesktopRoute {
                id: slot_id,
                display_name: ":10".to_string(),
                healthy: true,
            }]
        );
    }

    #[derive(Default)]
    struct FixtureRuntime {
        launches: usize,
        live: bool,
        next_tab: usize,
    }

    impl BrowserRuntimeDriver for FixtureRuntime {
        fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
            Ok(self.live)
        }

        fn launch_browser(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
        ) -> Result<BrowserLaunch, String> {
            self.launches += 1;
            Ok(BrowserLaunch {
                browser_id: format!("browser:{}:{}", profile.id, self.launches),
                pid: 4242,
                cdp_endpoint: "ws://127.0.0.1:9422/devtools/browser/test".to_string(),
                process_identity: None,
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
            self.next_tab += 1;
            Ok(BrowserTabAcquisition {
                tab_id: format!("tab-{}", self.next_tab),
                target_id: format!("target-{}", self.next_tab),
                source: agent_browser_service_model::BrowserTabSource::Bootstrap,
            })
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
            if command.get("id").and_then(serde_json::Value::as_str)
                == Some("navigate-with-headers")
                && command.get("headers") != Some(&serde_json::json!({"Remote-User": "operator"}))
            {
                return Err("fixture_navigation_headers_missing".to_string());
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

    struct JournalFixtureRuntime {
        database_path: PathBuf,
        launches: Arc<AtomicUsize>,
        reserved_launches: Arc<Mutex<BTreeMap<String, BrowserLaunch>>>,
        recovery_probes: Arc<AtomicUsize>,
        recover_reserved_browser: bool,
        inner: FixtureRuntime,
        fail_reserved_launch_once: bool,
        fail_initial_tab_once: bool,
    }

    impl BrowserRuntimeDriver for JournalFixtureRuntime {
        fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
            self.inner.browser_is_live(browser)
        }

        fn launch_browser(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
        ) -> Result<BrowserLaunch, String> {
            let store = BrowserRuntimeSqliteStore::open(&self.database_path)?;
            let operation = store.load_operation("journal-open-1")?;
            assert_eq!(operation.state, BrowserRuntimeOperationState::Observed);
            assert_eq!(
                operation.result.as_ref().unwrap()["phase"],
                "launch_started"
            );
            assert_eq!(operation.request["intent"]["session"]["name"], "alice");
            assert_eq!(
                operation.request["intent"]["session"]["id"],
                "session:alice:work:1"
            );
            assert_eq!(operation.request["intent"]["browser"]["profileId"], "work");
            assert_eq!(
                operation.request["intent"]["browser"]["id"],
                deterministic_manager_browser_id("journal-open-1", "work")
            );
            assert_eq!(operation.request["intent"]["slot"]["routeId"], "slot-a");
            assert_eq!(operation.request["intent"]["slot"]["displayName"], ":10");
            assert_eq!(operation.request["intent"]["handoff"]["state"], "pending");
            assert!(store.load_session_state()?.sessions.is_empty());
            self.launches.fetch_add(1, Ordering::SeqCst);
            self.inner.launch_browser(profile, desktop)
        }

        fn launch_browser_reserved(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
            browser_id: &str,
        ) -> Result<BrowserLaunch, String> {
            let mut launch = self.launch_browser(profile, desktop)?;
            launch.browser_id = browser_id.to_string();
            self.reserved_launches
                .lock()
                .map_err(|_| "journal_fixture_reserved_launches_poisoned".to_string())?
                .insert(browser_id.to_string(), launch.clone());
            if self.fail_reserved_launch_once {
                self.fail_reserved_launch_once = false;
                return Err("injected_uncertain_launch_outcome".to_string());
            }
            Ok(launch)
        }

        fn recover_browser_reserved(
            &mut self,
            _profile: &BrowserProfileCatalogEntry,
            _desktop: &agent_browser_service_model::BrowserDesktopAssignment,
            browser_id: &str,
        ) -> Result<ReservedBrowserRecovery, String> {
            self.recovery_probes.fetch_add(1, Ordering::SeqCst);
            if self.recover_reserved_browser {
                if let Some(launch) = self
                    .reserved_launches
                    .lock()
                    .map_err(|_| "journal_fixture_reserved_launches_poisoned".to_string())?
                    .get(browser_id)
                    .cloned()
                {
                    return Ok(ReservedBrowserRecovery::Recovered(launch));
                }
            }
            Ok(ReservedBrowserRecovery::Unproven {
                reason: "adoption_unproven".to_string(),
            })
        }

        fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
            self.inner.close_browser(browser)
        }

        fn acquire_initial_tab(
            &mut self,
            browser: &ManagedBrowserInstance,
            attributed_target_ids: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            if self.fail_initial_tab_once {
                self.fail_initial_tab_once = false;
                return Err("injected_initial_tab_interruption".to_string());
            }
            self.inner
                .acquire_initial_tab(browser, attributed_target_ids)
        }

        fn create_tab(
            &mut self,
            browser: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            self.inner.create_tab(browser)
        }

        fn close_tab(
            &mut self,
            browser: &ManagedBrowserInstance,
            tab: &ManagedBrowserTab,
        ) -> Result<(), String> {
            self.inner.close_tab(browser, tab)
        }

        fn navigate(
            &mut self,
            browser: &ManagedBrowserInstance,
            tab: &ManagedBrowserTab,
            url: &str,
        ) -> Result<(), String> {
            self.inner.navigate(browser, tab, url)
        }

        fn focus_browser(
            &mut self,
            browser: &ManagedBrowserInstance,
            tab: Option<&ManagedBrowserTab>,
        ) -> Result<(), String> {
            self.inner.focus_browser(browser, tab)
        }
    }

    fn ready_handoff_service_state() -> ServiceState {
        let mut service = ServiceState::default();
        service.route_pool.insert(
            "slot-a".to_string(),
            RoutePoolEntry {
                id: "slot-a".to_string(),
                route_id: "route-a".to_string(),
                target: serde_json::json!({
                    "displayName": ":10",
                    "displayAllocationId": "display-a"
                }),
                state: "available".to_string(),
                readiness: Some(serde_json::json!({"state": "ready"})),
                ..RoutePoolEntry::default()
            },
        );
        service.remote_view_routes.insert(
            "route-a".to_string(),
            RemoteViewRoute {
                id: "route-a".to_string(),
                display_allocation_id: Some("display-a".to_string()),
                route_descriptor: Some(serde_json::json!({
                    "publicOperatorUrl": "https://dashboard.example/operator"
                })),
                control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                state: "ready".to_string(),
                readiness: Some(serde_json::json!({"state": "ready"})),
                ..RemoteViewRoute::default()
            },
        );
        service.display_allocations.insert(
            "display-a".to_string(),
            DisplayAllocation {
                id: "display-a".to_string(),
                display_name: Some(":10".to_string()),
                owner_browser_id: Some(deterministic_manager_browser_id("journal-open-1", "work")),
                state: "ready".to_string(),
                route_ids: vec!["route-a".to_string()],
                readiness: Some(serde_json::json!({"state": "ready"})),
                ..DisplayAllocation::default()
            },
        );
        service
    }

    #[test]
    fn managed_navigation_executes_headers_on_the_manager_owned_tab() {
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
                }
            })
            .to_string(),
        )
        .unwrap();
        let store = BrowserSessionJsonStore::new(&directory.0);
        let effects = BrowserSessionEffectAdapter::new(FixtureRuntime::default());
        let mut host = BrowserSessionHost::load(
            store,
            effects,
            &legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: Vec::new(),
                default_disposable_policy: None,
            },
        )
        .unwrap();

        let response = host.handle_managed_navigation_command(&serde_json::json!({
            "id": "navigate-with-headers",
            "action": "browser_session_navigate",
            "sessionName": "alice",
            "profileId": "work",
            "url": "https://guacamole.example.test/guacamole/",
            "headers": {"Remote-User": "operator"},
            "activityAtMs": 1_000
        }));

        assert_eq!(response["success"], true);
        assert_eq!(response["data"]["profileId"], "work");
        assert_eq!(
            response["data"]["url"],
            "https://guacamole.example.test/guacamole/"
        );
        assert_eq!(host.state().sessions.len(), 1);
        assert_eq!(host.state().tabs.len(), 1);
        assert_eq!(host.state().navigation_history.len(), 1);
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
            remote_desktop_routes: Vec::new(),
            default_disposable_policy: None,
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
    }

    #[test]
    fn sqlite_authority_loads_and_persists_browser_session_host_state() {
        let directory = TempDirectory::new();
        let session_path = directory.0.join("browser-session-state.json");
        let catalog_path = directory.0.join("browser-profile-catalog.json");
        let legacy_path = directory.0.join("state.json");
        let database_path = directory.0.join("runtime.sqlite3");
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
                }
            })
            .to_string(),
        )
        .unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &legacy_path,
            },
        )
        .unwrap();

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let effects = BrowserSessionEffectAdapter::new(FixtureRuntime::default());
        let mut host = BrowserSessionHost::load(
            store,
            effects,
            &legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: Vec::new(),
                default_disposable_policy: None,
            },
        )
        .unwrap();
        let opened = host
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
            .unwrap();
        drop(host);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            reopened.load_session_state().unwrap().sessions[&opened.session_id].name,
            "alice"
        );
        assert!(reopened
            .load_profile_catalog()
            .unwrap()
            .profiles
            .contains_key("work"));
    }

    #[test]
    fn journaled_open_recovers_observed_browser_and_publishes_ready_handoff_atomically() {
        let directory = TempDirectory::new();
        let session_path = directory.0.join("browser-session-state.json");
        let catalog_path = directory.0.join("browser-profile-catalog.json");
        let legacy_path = directory.0.join("state.json");
        let database_path = directory.0.join("runtime.sqlite3");
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
                }
            })
            .to_string(),
        )
        .unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &legacy_path,
            },
        )
        .unwrap();
        let launches = Arc::new(AtomicUsize::new(0));
        let reserved_launches = Arc::new(Mutex::new(BTreeMap::new()));
        let recovery_probes = Arc::new(AtomicUsize::new(0));
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![
                BrowserDesktopRoute {
                    id: "slot-a".to_string(),
                    display_name: ":10".to_string(),
                    healthy: true,
                },
                BrowserDesktopRoute {
                    id: "slot-b".to_string(),
                    display_name: ":11".to_string(),
                    healthy: true,
                },
            ],
            default_disposable_policy: None,
        };
        let command = serde_json::json!({
            "id": "journal-open-1",
            "action": "browser_session_open",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 1_000
        });
        let service = ready_handoff_service_state();
        let inventory =
            StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap();

        let mut prepared_store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let base_session_state = prepared_store.load_session_state().unwrap();
        let prepared = prepared_store
            .reserve_operation(
                "journal-open-1",
                "browser-runtime-open",
                serde_json::json!({
                    "schemaVersion": "agent-browser.browser-open-operation.v1",
                    "command": &command,
                    "baseSessionState": &base_session_state,
                    "intent": {
                        "session": {"id": "session:alice:work:1", "name": "alice", "profileId": "work"},
                        "browser": {
                            "id": deterministic_manager_browser_id("journal-open-1", "work"),
                            "profileId": "work",
                            "disposition": "reuse_or_launch"
                        },
                        "slot": {"routeId": "slot-a", "displayName": ":10", "liveBrowserCount": 0},
                        "handoff": {
                            "id": deterministic_manager_handoff_id("journal-open-1"),
                            "state": "pending"
                        }
                    }
                }),
            )
            .unwrap();
        assert_eq!(prepared.state, BrowserRuntimeOperationState::Prepared);
        drop(prepared_store);

        let first = {
            let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
            let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
                database_path: database_path.clone(),
                launches: launches.clone(),
                reserved_launches: reserved_launches.clone(),
                recovery_probes: recovery_probes.clone(),
                recover_reserved_browser: false,
                inner: FixtureRuntime {
                    live: true,
                    ..FixtureRuntime::default()
                },
                fail_reserved_launch_once: false,
                fail_initial_tab_once: true,
            });
            let mut host =
                BrowserSessionHost::load(store, effects, &legacy_path, config.clone()).unwrap();
            host.handle_journaled_open_with_handoff(&command, &service, &inventory)
        };
        assert_eq!(first["success"], false);
        assert_eq!(first["error"], "injected_initial_tab_interruption");
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        let interrupted = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let persisted_base_state = interrupted.load_session_state().unwrap();
        assert!(persisted_base_state.sessions.is_empty());
        let pending = interrupted.load_operation("journal-open-1").unwrap();
        assert_eq!(pending.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(pending.result.as_ref().unwrap()["phase"], "browser_opened");
        drop(interrupted);

        let intervening_store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let mut intervening_state = persisted_base_state.clone();
        intervening_state.next_disposable_sequence = 9;
        intervening_store
            .save_session_state(&intervening_state)
            .unwrap();
        let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
            database_path: database_path.clone(),
            launches: launches.clone(),
            reserved_launches: reserved_launches.clone(),
            recovery_probes: recovery_probes.clone(),
            recover_reserved_browser: false,
            inner: FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            },
            fail_reserved_launch_once: false,
            fail_initial_tab_once: false,
        });
        let mut conflicted =
            BrowserSessionHost::load(intervening_store, effects, &legacy_path, config.clone())
                .unwrap();
        let conflict =
            conflicted.handle_journaled_open_with_handoff(&command, &service, &inventory);
        assert_eq!(conflict["success"], false);
        assert_eq!(
            conflict["error"],
            "browser_runtime_operation_base_state_conflict"
        );
        drop(conflicted);
        let restore_store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            restore_store.load_session_state().unwrap(),
            intervening_state
        );
        restore_store
            .save_session_state(&persisted_base_state)
            .unwrap();
        drop(restore_store);

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
            database_path: database_path.clone(),
            launches: launches.clone(),
            reserved_launches: reserved_launches.clone(),
            recovery_probes: recovery_probes.clone(),
            recover_reserved_browser: false,
            inner: FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            },
            fail_reserved_launch_once: false,
            fail_initial_tab_once: false,
        });
        let mut restarted = BrowserSessionHost::load(store, effects, &legacy_path, config).unwrap();
        let ready = restarted.handle_journaled_open_with_handoff(&command, &service, &inventory);

        assert_eq!(ready["success"], true);
        assert_eq!(ready["data"]["operatorVisible"]["state"], "ready");
        assert_eq!(
            ready["data"]["browserId"],
            deterministic_manager_browser_id("journal-open-1", "work")
        );
        assert_eq!(ready["data"]["tabId"], "tab-1");
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        let handoff_id = ready["data"]["handoffId"].as_str().unwrap();
        assert!(restarted.manager_handoff(handoff_id).is_some());

        let replay = restarted.handle_journaled_open_with_handoff(&command, &service, &inventory);
        assert_eq!(replay, ready);
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        drop(restarted);

        let published = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            published.load_operation("journal-open-1").unwrap().state,
            BrowserRuntimeOperationState::Committed
        );
        let state = published.load_session_state().unwrap();
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(
            published
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .get(handoff_id)
                .map(|handoff| handoff.state.as_str()),
            Some("ready")
        );
    }

    #[test]
    fn journaled_open_retains_one_cleanup_obligation_for_unproven_reserved_launch() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        let database_path = directory.0.join("runtime.sqlite3");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {"work": {
                    "id": "work",
                    "name": "Work",
                    "userDataDir": directory.0.join("work"),
                    "profileClass": "durable_named"
                }}
            })
            .to_string(),
        )
        .unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join("browser-session-state.json"),
                profile_catalog_path: &directory.0.join("browser-profile-catalog.json"),
                service_state_path: &legacy_path,
            },
        )
        .unwrap();
        let launches = Arc::new(AtomicUsize::new(0));
        let reserved_launches = Arc::new(Mutex::new(BTreeMap::new()));
        let recovery_probes = Arc::new(AtomicUsize::new(0));
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![BrowserDesktopRoute {
                id: "slot-a".to_string(),
                display_name: ":10".to_string(),
                healthy: true,
            }],
            default_disposable_policy: None,
        };
        let command = serde_json::json!({
            "id": "journal-open-1",
            "action": "browser_session_open",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 1_000
        });
        let service = ready_handoff_service_state();
        let inventory =
            StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap();

        let first = {
            let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
            let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
                database_path: database_path.clone(),
                launches: launches.clone(),
                reserved_launches: reserved_launches.clone(),
                recovery_probes: recovery_probes.clone(),
                recover_reserved_browser: false,
                inner: FixtureRuntime {
                    live: true,
                    ..FixtureRuntime::default()
                },
                fail_reserved_launch_once: true,
                fail_initial_tab_once: false,
            });
            let mut host =
                BrowserSessionHost::load(store, effects, &legacy_path, config.clone()).unwrap();
            host.handle_journaled_open_with_handoff(&command, &service, &inventory)
        };
        assert_eq!(first["success"], false);
        assert_eq!(first["error"], "injected_uncertain_launch_outcome");
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        let pending = BrowserRuntimeSqliteStore::open(&database_path)
            .unwrap()
            .load_operation("journal-open-1")
            .unwrap();
        assert_eq!(pending.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(pending.result.as_ref().unwrap()["phase"], "launch_started");

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
            database_path: database_path.clone(),
            launches: launches.clone(),
            reserved_launches: reserved_launches.clone(),
            recovery_probes: recovery_probes.clone(),
            recover_reserved_browser: false,
            inner: FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            },
            fail_reserved_launch_once: false,
            fail_initial_tab_once: false,
        });
        let mut restarted = BrowserSessionHost::load(store, effects, &legacy_path, config).unwrap();
        let retry = restarted.handle_journaled_open_with_handoff(&command, &service, &inventory);

        assert_eq!(retry["success"], false);
        assert_eq!(
            retry["error"],
            "browser_runtime_open_reserved_browser_recovery_unproven"
        );
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        assert_eq!(recovery_probes.load(Ordering::SeqCst), 1);
        let cleanup_store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let cleanup = cleanup_store.load_operation("journal-open-1").unwrap();
        assert_eq!(cleanup.state, BrowserRuntimeOperationState::Observed);
        let cleanup_result = cleanup.result.as_ref().unwrap();
        assert_eq!(cleanup_result["phase"], "launch_cleanup_required");
        assert_eq!(
            cleanup_result["cleanupObligation"]["kind"],
            "reserved_browser_reconciliation"
        );
        assert_eq!(cleanup_result["cleanupObligation"]["state"], "pending");
        assert_eq!(
            cleanup_result["cleanupObligation"]["browserId"],
            deterministic_manager_browser_id("journal-open-1", "work")
        );
        assert_eq!(cleanup_result["cleanupObligation"]["profileId"], "work");
        assert_eq!(cleanup_result["cleanupObligation"]["routeId"], "slot-a");
        assert_eq!(cleanup_result["cleanupObligation"]["displayName"], ":10");
        assert_eq!(
            cleanup_result["cleanupObligation"]["reason"],
            "adoption_unproven"
        );
        let state = cleanup_store.load_session_state().unwrap();
        assert!(state.sessions.is_empty());
        assert!(state.browsers.is_empty());
        assert!(state.tabs.is_empty());
        assert!(cleanup_store
            .load_handoff_registry()
            .unwrap()
            .handoffs
            .is_empty());
        drop(cleanup_store);

        let repeated = restarted.handle_journaled_open_with_handoff(&command, &service, &inventory);
        assert_eq!(repeated, retry);
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        assert_eq!(recovery_probes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn journaled_open_adopts_exact_reserved_browser_after_ambiguous_launch_restart() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        let database_path = directory.0.join("runtime.sqlite3");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {"work": {
                    "id": "work",
                    "name": "Work",
                    "userDataDir": directory.0.join("work"),
                    "profileClass": "durable_named"
                }}
            })
            .to_string(),
        )
        .unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join("browser-session-state.json"),
                profile_catalog_path: &directory.0.join("browser-profile-catalog.json"),
                service_state_path: &legacy_path,
            },
        )
        .unwrap();
        let launches = Arc::new(AtomicUsize::new(0));
        let reserved_launches = Arc::new(Mutex::new(BTreeMap::new()));
        let recovery_probes = Arc::new(AtomicUsize::new(0));
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![BrowserDesktopRoute {
                id: "slot-a".to_string(),
                display_name: ":10".to_string(),
                healthy: true,
            }],
            default_disposable_policy: None,
        };
        let command = serde_json::json!({
            "id": "journal-open-1",
            "action": "browser_session_open",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 1_000
        });
        let service = ready_handoff_service_state();
        let inventory =
            StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap();

        let first = {
            let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
            let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
                database_path: database_path.clone(),
                launches: launches.clone(),
                reserved_launches: reserved_launches.clone(),
                recovery_probes: recovery_probes.clone(),
                recover_reserved_browser: false,
                inner: FixtureRuntime {
                    live: true,
                    ..FixtureRuntime::default()
                },
                fail_reserved_launch_once: true,
                fail_initial_tab_once: false,
            });
            let mut host =
                BrowserSessionHost::load(store, effects, &legacy_path, config.clone()).unwrap();
            host.handle_journaled_open_with_handoff(&command, &service, &inventory)
        };
        assert_eq!(first["error"], "injected_uncertain_launch_outcome");
        assert_eq!(launches.load(Ordering::SeqCst), 1);

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let effects = BrowserSessionEffectAdapter::new(JournalFixtureRuntime {
            database_path: database_path.clone(),
            launches: launches.clone(),
            reserved_launches,
            recovery_probes: recovery_probes.clone(),
            recover_reserved_browser: true,
            inner: FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            },
            fail_reserved_launch_once: false,
            fail_initial_tab_once: false,
        });
        let mut restarted = BrowserSessionHost::load(store, effects, &legacy_path, config).unwrap();
        let ready = restarted.handle_journaled_open_with_handoff(&command, &service, &inventory);

        assert_eq!(ready["success"], true);
        assert_eq!(
            ready["data"]["browserId"],
            deterministic_manager_browser_id("journal-open-1", "work")
        );
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        assert_eq!(recovery_probes.load(Ordering::SeqCst), 1);
        let replay = restarted.handle_journaled_open_with_handoff(&command, &service, &inventory);
        assert_eq!(replay, ready);
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        assert_eq!(recovery_probes.load(Ordering::SeqCst), 1);
        drop(restarted);

        let published = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let operation = published.load_operation("journal-open-1").unwrap();
        assert_eq!(operation.state, BrowserRuntimeOperationState::Committed);
        assert_eq!(operation.result.as_ref().unwrap()["phase"], "ready");
        let state = published.load_session_state().unwrap();
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.browsers.len(), 1);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(published.load_handoff_registry().unwrap().handoffs.len(), 1);
    }

    #[test]
    fn hosted_reaper_deletes_only_the_recorded_disposable_directory() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(&legacy_path, r#"{"profiles":{}}"#).unwrap();
        let disposable_root = directory.0.join("disposable-profiles");
        let foreign_directory = disposable_root.join("foreign-directory");
        fs::create_dir_all(&foreign_directory).unwrap();
        let foreign_marker = foreign_directory.join("keep");
        fs::write(&foreign_marker, b"foreign").unwrap();

        let mut host = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime::default()),
            &legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: Vec::new(),
                default_disposable_policy: Some(BrowserDisposableProfilePolicy {
                    id: DEFAULT_DISPOSABLE_POLICY_ID.to_string(),
                    user_data_root: disposable_root.to_string_lossy().into_owned(),
                    cleanup_delay_ms: 0,
                }),
            },
        )
        .unwrap();
        let opened = host
            .open(OpenBrowserSession::disposable(
                "alice",
                DEFAULT_DISPOSABLE_POLICY_ID,
                1_000,
            ))
            .unwrap();
        let allocation = host
            .state()
            .disposable_profiles
            .get(&opened.profile_id)
            .cloned()
            .expect("hosted disposable allocation");
        let allocated_directory = PathBuf::from(&allocation.profile.user_data_dir);
        assert!(allocated_directory.is_dir());
        fs::write(allocated_directory.join("owned-marker"), b"owned").unwrap();

        host.close_session(&opened.session_id, SessionEndReason::ExplicitClose, 1_100)
            .unwrap();
        assert!(allocated_directory.is_dir());

        let reaped = host.reap(1_100).unwrap();

        assert_eq!(
            reaped.deleted_disposable_profile_ids,
            vec![opened.profile_id.clone()]
        );
        assert!(!allocated_directory.exists());
        assert_eq!(fs::read(&foreign_marker).unwrap(), b"foreign");
        assert!(host.state().disposable_profiles.is_empty());
        let persisted = BrowserSessionJsonStore::new(&directory.0)
            .load_session_state()
            .unwrap();
        assert!(persisted.disposable_profiles.is_empty());
    }

    #[test]
    fn hosted_multi_display_selection_persists_without_legacy_authority() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        let legacy_state = serde_json::json!({
            "profiles": {
                "work": {
                    "id": "work",
                    "name": "Work",
                    "userDataDir": directory.0.join("work"),
                    "profileClass": "durable_named"
                },
                "personal": {
                    "id": "personal",
                    "name": "Personal",
                    "userDataDir": directory.0.join("personal"),
                    "profileClass": "durable_named"
                }
            },
            "sessions": "contradictory legacy session state",
            "runtimeOwners": ["contradictory legacy owner state"],
            "displayAllocations": "contradictory legacy display state"
        });
        fs::write(
            &legacy_path,
            serde_json::to_vec_pretty(&legacy_state).unwrap(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: vec![
                BrowserDesktopRoute {
                    id: "slot-a".to_string(),
                    display_name: ":10".to_string(),
                    healthy: true,
                },
                BrowserDesktopRoute {
                    id: "slot-b".to_string(),
                    display_name: ":11".to_string(),
                    healthy: true,
                },
            ],
            default_disposable_policy: None,
        };
        let mut host = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime::default()),
            &legacy_path,
            config.clone(),
        )
        .unwrap();

        let alice = host
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
            .unwrap();
        let bob = host
            .open(OpenBrowserSession::exact_profile("bob", "personal", 1_100))
            .unwrap();
        let alice_desktop = host.state().browsers[&alice.browser_id]
            .desktop
            .as_ref()
            .expect("Alice desktop assignment");
        let bob_desktop = host.state().browsers[&bob.browser_id]
            .desktop
            .as_ref()
            .expect("Bob desktop assignment");
        assert_eq!(alice_desktop.route_id, "slot-a");
        assert_eq!(alice_desktop.display_name, ":10");
        assert_eq!(alice_desktop.live_browser_count, 0);
        assert_eq!(bob_desktop.route_id, "slot-b");
        assert_eq!(bob_desktop.display_name, ":11");
        assert_eq!(bob_desktop.live_browser_count, 0);
        drop(host);

        let restarted = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            config,
        )
        .unwrap();
        assert_eq!(
            restarted.state().browsers[&alice.browser_id]
                .desktop
                .as_ref()
                .map(|desktop| desktop.display_name.as_str()),
            Some(":10")
        );
        assert_eq!(
            restarted.state().browsers[&bob.browser_id]
                .desktop
                .as_ref()
                .map(|desktop| desktop.display_name.as_str()),
            Some(":11")
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&legacy_path).unwrap()).unwrap(),
            legacy_state
        );
    }

    #[test]
    fn route_choices_can_refresh_after_provider_bootstrap_without_rebinding_existing_browsers() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {
                    "viewer": {
                        "id": "viewer",
                        "name": "Viewer",
                        "userDataDir": directory.0.join("viewer"),
                        "profileClass": "durable_named"
                    },
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": directory.0.join("work"),
                        "profileClass": "durable_named"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let mut host = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime::default()),
            &legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: Vec::new(),
                default_disposable_policy: None,
            },
        )
        .unwrap();

        let viewer = host
            .open(OpenBrowserSession::exact_profile("viewer", "viewer", 1_000))
            .unwrap();
        assert!(host.state().browsers[&viewer.browser_id].desktop.is_none());

        host.replace_remote_desktop_routes(vec![BrowserDesktopRoute {
            id: "development-route-1".to_string(),
            display_name: ":13".to_string(),
            healthy: true,
        }]);
        let work = host
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_100))
            .unwrap();
        assert!(host.state().browsers[&viewer.browser_id].desktop.is_none());
        assert_eq!(
            host.state().browsers[&work.browser_id]
                .desktop
                .as_ref()
                .map(|desktop| desktop.display_name.as_str()),
            Some(":13")
        );
    }

    #[test]
    fn handoff_resolution_focuses_and_persists_the_exact_session_heartbeat() {
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
                }
            })
            .to_string(),
        )
        .unwrap();
        let store = BrowserSessionJsonStore::new(&directory.0);
        let effects = BrowserSessionEffectAdapter::new(FixtureRuntime {
            live: true,
            ..FixtureRuntime::default()
        });
        let mut host = BrowserSessionHost::load(
            store,
            effects,
            &legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: vec![BrowserDesktopRoute {
                    id: "slot-a".to_string(),
                    display_name: ":10".to_string(),
                    healthy: true,
                }],
                default_disposable_policy: None,
            },
        )
        .unwrap();
        let opened = host
            .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
            .unwrap();
        let tab = host.new_tab(&opened.session_id, 1_100).unwrap();
        let handoff = RemoteViewHandoff {
            id: "opaque-a".to_string(),
            state: "ready".to_string(),
            intent: serde_json::json!({
                "browserSessionManager": true,
                "sessionId": opened.session_id,
            }),
            handoff_url: Some("https://dashboard.example/remote-view/opaque-a".to_string()),
            profile_id: Some("work".to_string()),
            browser_id: Some(opened.browser_id.clone()),
            session_name: Some("alice".to_string()),
            tab_id: Some(tab.tab_id.clone()),
            target_id: Some(tab.target_id.clone()),
            ..RemoteViewHandoff::default()
        };
        let mut service = ServiceState::default();
        service.display_allocations.insert(
            "display-a".to_string(),
            DisplayAllocation {
                id: "display-a".to_string(),
                display_name: Some(":10".to_string()),
                owner_browser_id: Some(opened.browser_id.clone()),
                state: "ready".to_string(),
                route_ids: vec!["route-a".to_string()],
                readiness: Some(serde_json::json!({"state":"ready"})),
                ..DisplayAllocation::default()
            },
        );
        service.remote_view_routes.insert(
            "route-a".to_string(),
            RemoteViewRoute {
                id: "route-a".to_string(),
                display_allocation_id: Some("display-a".to_string()),
                browser_id: Some(opened.browser_id.clone()),
                route_descriptor: Some(serde_json::json!({
                    "publicOperatorUrl":"https://dashboard.example/operator"
                })),
                control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                state: "ready".to_string(),
                readiness: Some(serde_json::json!({"state":"ready"})),
                ..RemoteViewRoute::default()
            },
        );
        let inventory = super::super::presentation_inventory::StaticRouteInventory::from_json(
            r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#,
        )
        .unwrap();

        let resolved = host
            .resolve_manager_handoff(&handoff, &service, &inventory, 2_000)
            .unwrap();

        assert_eq!(resolved["status"], "ready");
        assert_eq!(resolved["resolved"], true);
        assert_eq!(resolved["handoffUrl"], handoff.handoff_url.unwrap());
        assert_eq!(resolved["browserId"], opened.browser_id);
        assert_eq!(resolved["tabId"], tab.tab_id);
        assert_eq!(resolved["targetId"], tab.target_id);
        assert_eq!(resolved["operatorVisible"]["state"], "ready");
        assert_eq!(resolved["presentationGeneration"], 1);
        assert_eq!(resolved["presentationReceipt"]["generation"], 1);
        assert_eq!(
            resolved["presentationReceipt"]["logicalBrowserId"],
            opened.browser_id
        );
        assert_eq!(resolved["presentationReceipt"]["targetId"], tab.target_id);
        assert_eq!(
            resolved["presentationReceipt"]["requiredStreamProvider"],
            resolved["viewStreamProvider"]
        );
        assert_eq!(
            resolved["presentationReceipt"]["observedStreamProvider"],
            resolved["viewStreamProvider"]
        );
        assert_eq!(resolved["presentationReceipt"]["state"], "ready");
        assert_eq!(
            resolved["presentationReceipt"]["browserSessionManager"],
            true
        );
        assert!(resolved.get("externalUrl").is_none());
        assert!(resolved.get("providerExternalUrl").is_none());

        let persisted = BrowserSessionJsonStore::new(&directory.0)
            .load_session_state()
            .unwrap();
        let persisted_session = persisted.sessions.get(&opened.session_id).unwrap();
        assert_eq!(persisted_session.last_activity_at_ms, 2_000);
        assert_eq!(persisted_session.expires_at_ms, 302_000);
        assert_eq!(
            persisted_session.current_tab_id.as_deref(),
            Some(tab.tab_id.as_str())
        );
    }

    #[test]
    fn status_liveness_reconciliation_retires_dead_persisted_browsers() {
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
                }
            })
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: Vec::new(),
            default_disposable_policy: None,
        };
        let opened = {
            let mut host = BrowserSessionHost::load(
                BrowserSessionJsonStore::new(&directory.0),
                BrowserSessionEffectAdapter::new(FixtureRuntime {
                    live: true,
                    ..FixtureRuntime::default()
                }),
                &legacy_path,
                config.clone(),
            )
            .unwrap();
            host.open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
                .unwrap()
        };
        let mut restarted = BrowserSessionHost::load(
            BrowserSessionJsonStore::new(&directory.0),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: false,
                ..FixtureRuntime::default()
            }),
            &legacy_path,
            config,
        )
        .unwrap();

        let retired = restarted.reconcile_liveness_current().unwrap();

        assert_eq!(retired, vec![opened.browser_id]);
        assert!(restarted.state().browsers.is_empty());
        assert!(restarted.state().sessions.is_empty());
        assert_eq!(restarted.state().session_history.len(), 1);
        assert_eq!(
            restarted.state().session_history[0].reason,
            SessionEndReason::BrowserUnresponsive
        );
    }
}
