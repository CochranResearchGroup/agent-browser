//! Durable recovery for manager-owned header-bearing navigation.
//!
//! A navigation is distinct from an open: after a renderer has accepted a
//! command, repeating it can repeat a non-idempotent request.  The journal
//! therefore records the target and an `issued` fence before executing the
//! command. A later process may finish only when it can observe that exact
//! target at the exact requested URL. URL equality is an observational
//! recovery proof, not proof that the original headers were delivered, so a
//! mismatched or unobservable issued operation is never retried.

use agent_browser_service_model::{
    BrowserSessionEffects, ManagedBrowserInstance, ManagedBrowserTab, RouteKeeperAuthority,
};
use serde_json::{json, Value};

use super::{
    current_unix_ms, deterministic_manager_handoff_id, optional_string, required_string,
    BrowserRuntimeOperation, BrowserRuntimeOperationState, BrowserSessionHost,
    BrowserSessionPersistence, ManagedBrowserCommandEffects, ManagerHandoffAuthority,
    ReservedBrowserRecoveryEffects,
};

const NAVIGATION_OWNER_KEY: &str = "browser-runtime-navigation";
const NAVIGATION_OPERATION_SCHEMA: &str = "agent-browser.browser-navigation-operation.v1";

/// The one observation that can prove an issued navigation completed.
///
/// The runtime implementation must inspect the live CDP target, rather than
/// returning the intended URL from the command executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NavigationTargetObservation {
    pub(crate) target_id: String,
    pub(crate) url: String,
}

/// Provider boundary for the post-crash navigation proof.
pub(crate) trait NavigationTargetObservationEffects {
    fn observe_navigation_target(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<NavigationTargetObservation, String>;
}

#[derive(Debug, Clone)]
struct NavigationTarget {
    session_id: String,
    session_name: String,
    profile_id: String,
    browser: ManagedBrowserInstance,
    tab: ManagedBrowserTab,
}

impl<P, E> BrowserSessionHost<P, E>
where
    P: BrowserSessionPersistence,
    E: BrowserSessionEffects
        + ManagedBrowserCommandEffects
        + ReservedBrowserRecoveryEffects
        + NavigationTargetObservationEffects,
{
    /// Execute a managed navigation through a durable operation journal.
    ///
    /// The caller supplies the current keeper authority because the nested
    /// bootstrap open and the completed navigation must publish a handoff
    /// against the same ready route proof.
    pub(crate) fn handle_journaled_navigation_with_keeper(
        &mut self,
        command: &Value,
        authority: &RouteKeeperAuthority,
    ) -> Value {
        let id = command.get("id").cloned().unwrap_or(Value::Null);
        let baseline_state = self.state.clone();
        match self.journaled_navigation_with_keeper_result(command, authority) {
            Ok(response) => response,
            Err(error) => {
                self.state = self
                    .persistence
                    .load_session_state()
                    .unwrap_or(baseline_state);
                json!({ "id": id, "success": false, "error": error })
            }
        }
    }

    fn journaled_navigation_with_keeper_result(
        &mut self,
        command: &Value,
        authority: &RouteKeeperAuthority,
    ) -> Result<Value, String> {
        if command.get("action").and_then(Value::as_str) != Some("browser_session_navigate") {
            return Err("browser_runtime_navigation_action_invalid".to_string());
        }
        self.preflight_keeper_navigation_command(command, authority)?;
        let operation_id = required_string(command, "id")?;
        let url = required_string(command, "url")?.to_string();
        let activity_at_ms = command
            .get("activityAtMs")
            .or_else(|| {
                command
                    .get("params")
                    .and_then(|params| params.get("activityAtMs"))
            })
            .and_then(Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        let existing_operation = self.persistence.find_operation(operation_id)?;
        let request = if let Some(existing) = &existing_operation {
            if existing.owner_key != NAVIGATION_OWNER_KEY
                || existing.request.get("command") != Some(command)
            {
                return Err(format!(
                    "browser_runtime_operation_replay_mismatch:{operation_id}"
                ));
            }
            existing.request.clone()
        } else {
            json!({
                "schemaVersion": NAVIGATION_OPERATION_SCHEMA,
                "command": command,
                "baseSessionState": &self.state,
                "intent": {
                    "url": url,
                    "bootstrapOperationId": format!("{operation_id}:bootstrap"),
                },
            })
        };
        let mut operation = self
            .persistence
            .reserve_operation(operation_id, NAVIGATION_OWNER_KEY, request)?
            .ok_or_else(|| "browser_runtime_operation_persistence_unsupported".to_string())?;
        let mut execute_authorized = false;

        loop {
            match operation.state {
                BrowserRuntimeOperationState::Committed => {
                    return operation.result.ok_or_else(|| {
                        "browser_runtime_operation_committed_result_missing".to_string()
                    });
                }
                BrowserRuntimeOperationState::Prepared => {
                    let target = match self.persisted_navigation_target(command, activity_at_ms)? {
                        Some(target) => target,
                        None => {
                            let bootstrap = self.navigation_bootstrap_command(
                                command,
                                operation.request["intent"]["bootstrapOperationId"]
                                    .as_str()
                                    .ok_or_else(|| {
                                        "browser_runtime_navigation_bootstrap_id_missing"
                                            .to_string()
                                    })?,
                            )?;
                            let response = self.journaled_open_with_handoff_result(
                                &bootstrap,
                                ManagerHandoffAuthority::Keeper(authority),
                            )?;
                            self.navigation_target_from_response(&response)?
                        }
                    };
                    operation = self.record_navigation_observation(
                        &operation,
                        "target_prepared",
                        &target,
                        None,
                    )?;
                }
                BrowserRuntimeOperationState::Observed => {
                    let observation = operation.result.as_ref().ok_or_else(|| {
                        "browser_runtime_operation_observation_missing".to_string()
                    })?;
                    self.require_navigation_state_current(observation)?;
                    let phase = observation
                        .get("phase")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            "browser_runtime_operation_observation_phase_missing".to_string()
                        })?;
                    match phase {
                        "target_prepared" => {
                            let target = self.navigation_target_from_observation(observation)?;
                            operation = self.record_navigation_observation(
                                &operation, "issued", &target, None,
                            )?;
                            execute_authorized = true;
                        }
                        "issued" => {
                            let target = self.navigation_target_from_observation(observation)?;
                            if execute_authorized {
                                if !self.effects.browser_is_live(&target.browser)? {
                                    return Err(
                                        "browser_runtime_navigation_target_not_live".to_string()
                                    );
                                }
                                let mut navigate_command = command.clone();
                                navigate_command["action"] = json!("navigate");
                                let response = self.effects.execute_command(
                                    &target.browser,
                                    &target.tab,
                                    &target.session_id,
                                    &target.session_name,
                                    &navigate_command,
                                )?;
                                if response.get("success").and_then(Value::as_bool) != Some(true) {
                                    return Err(response
                                        .get("error")
                                        .and_then(Value::as_str)
                                        .unwrap_or("browser_session_navigation_failed")
                                        .to_string());
                                }
                                operation = self.record_navigation_observation(
                                    &operation, "executed", &target, None,
                                )?;
                                execute_authorized = false;
                            } else {
                                let observed = self
                                    .effects
                                    .observe_navigation_target(&target.browser, &target.tab)
                                    .map_err(|_| {
                                        "browser_runtime_navigation_issued_observation_unavailable"
                                            .to_string()
                                    })?;
                                if observed.target_id != target.tab.target_id || observed.url != url
                                {
                                    return Err(
                                        "browser_runtime_navigation_issued_unproven".to_string()
                                    );
                                }
                                operation = self.record_navigation_observation(
                                    &operation, "executed", &target, None,
                                )?;
                            }
                        }
                        "executed" => {
                            let target = self.navigation_target_from_observation(observation)?;
                            // A persisted successful execution is not proof that its
                            // target still exists after restart. Redirects are valid,
                            // so requalify identity without requiring the original URL.
                            let observed = self
                                .effects
                                .observe_navigation_target(&target.browser, &target.tab)
                                .map_err(|_| {
                                    "browser_runtime_navigation_executed_target_unavailable"
                                        .to_string()
                                })?;
                            if observed.target_id != target.tab.target_id {
                                return Err(
                                    "browser_runtime_navigation_executed_target_unavailable"
                                        .to_string(),
                                );
                            }
                            let expected_base_state = self.state.clone();
                            let navigation = self.manager().record_navigation(
                                &target.session_id,
                                &url,
                                activity_at_ms,
                            )?;
                            let mut response = json!({
                                "id": command.get("id").cloned().unwrap_or(Value::Null),
                                "success": true,
                                "data": {
                                    "sessionId": navigation.session_id,
                                    "profileId": navigation.profile_id,
                                    "browserId": navigation.browser_id,
                                    "tabId": navigation.tab_id,
                                    "targetId": navigation.target_id,
                                    "url": navigation.url,
                                    "visitedAtMs": navigation.visited_at_ms,
                                    "handoffId": deterministic_manager_handoff_id(&operation.operation_id),
                                },
                            });
                            let prepared = super::super::browser_session_handoff::prepare_keeper_manager_handoff(
                                &response,
                                &self.state,
                                authority,
                                &self.handoffs,
                            )?;
                            if let (Some(data), Some(projection)) = (
                                response.get_mut("data").and_then(Value::as_object_mut),
                                prepared.projection.as_json().as_object(),
                            ) {
                                for (key, value) in projection {
                                    data.insert(key.clone(), value.clone());
                                }
                            }
                            let committed = self
                                .persistence
                                .commit_browser_navigation(
                                    &operation,
                                    &expected_base_state,
                                    &self.state,
                                    &prepared.handoff,
                                    response,
                                )?
                                .ok_or_else(|| {
                                    "browser_runtime_operation_persistence_unsupported".to_string()
                                })?;
                            self.handoffs
                                .insert(prepared.handoff.id.clone(), prepared.handoff);
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

    fn navigation_bootstrap_command(
        &self,
        command: &Value,
        operation_id: &str,
    ) -> Result<Value, String> {
        Ok(json!({
            "id": operation_id,
            "action": "browser_session_open",
            "sessionName": required_string(command, "sessionName")?,
            "profileId": required_string(command, "profileId")?,
            "activityAtMs": command.get("activityAtMs").cloned().unwrap_or(Value::Null),
        }))
    }

    fn persisted_navigation_target(
        &self,
        command: &Value,
        activity_at_ms: u64,
    ) -> Result<Option<NavigationTarget>, String> {
        let requested_session_id = optional_string(command, "sessionId").map(str::to_string);
        let session_id = if let Some(session_id) = requested_session_id {
            session_id
        } else {
            let session_name = required_string(command, "sessionName")?;
            let profile_id = required_string(command, "profileId")?;
            match self.state.sessions.values().find(|session| {
                session.name == session_name
                    && session.profile_id == profile_id
                    && activity_at_ms < session.expires_at_ms
            }) {
                Some(session) => session.id.clone(),
                None => return Ok(None),
            }
        };
        let session = self
            .state
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let Some(tab_id) = session.current_tab_id.as_deref() else {
            return Ok(None);
        };
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let tab = self
            .state
            .tabs
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        if tab.session_id != session.id || tab.browser_id != browser.id {
            return Err("browser_session_navigation_target_attribution_mismatch".to_string());
        }
        Ok(Some(NavigationTarget {
            session_id: session.id,
            session_name: session.name,
            profile_id: session.profile_id,
            browser,
            tab,
        }))
    }

    fn navigation_target_from_response(
        &self,
        response: &Value,
    ) -> Result<NavigationTarget, String> {
        let data = response
            .get("data")
            .ok_or_else(|| "browser_runtime_navigation_bootstrap_data_missing".to_string())?;
        self.navigation_target_from_ids(
            data.get("sessionId").and_then(Value::as_str),
            data.get("browserId").and_then(Value::as_str),
            data.get("tabId").and_then(Value::as_str),
            data.get("targetId").and_then(Value::as_str),
        )
    }

    fn navigation_target_from_observation(
        &self,
        observation: &Value,
    ) -> Result<NavigationTarget, String> {
        let target = observation
            .get("target")
            .ok_or_else(|| "browser_runtime_navigation_target_missing".to_string())?;
        self.navigation_target_from_ids(
            target.get("sessionId").and_then(Value::as_str),
            target.get("browserId").and_then(Value::as_str),
            target.get("tabId").and_then(Value::as_str),
            target.get("targetId").and_then(Value::as_str),
        )
    }

    fn navigation_target_from_ids(
        &self,
        session_id: Option<&str>,
        browser_id: Option<&str>,
        tab_id: Option<&str>,
        target_id: Option<&str>,
    ) -> Result<NavigationTarget, String> {
        let session_id = session_id
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "browser_runtime_navigation_session_id_missing".to_string())?;
        let browser_id = browser_id
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "browser_runtime_navigation_browser_id_missing".to_string())?;
        let tab_id = tab_id
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "browser_runtime_navigation_tab_id_missing".to_string())?;
        let target_id = target_id
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "browser_runtime_navigation_target_id_missing".to_string())?;
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let tab = self
            .state
            .tabs
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        if session.browser_id != browser.id
            || tab.session_id != session.id
            || tab.browser_id != browser.id
            || tab.target_id != target_id
        {
            return Err("browser_runtime_navigation_target_attribution_mismatch".to_string());
        }
        Ok(NavigationTarget {
            session_id: session.id,
            session_name: session.name,
            profile_id: session.profile_id,
            browser,
            tab,
        })
    }

    fn record_navigation_observation(
        &mut self,
        operation: &BrowserRuntimeOperation,
        phase: &str,
        target: &NavigationTarget,
        response: Option<Value>,
    ) -> Result<BrowserRuntimeOperation, String> {
        let observation = json!({
            "phase": phase,
            "sessionState": &self.state,
            "target": {
                "sessionId": target.session_id,
                "sessionName": target.session_name,
                "profileId": target.profile_id,
                "browserId": target.browser.id,
                "tabId": target.tab.id,
                "targetId": target.tab.target_id,
            },
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

    fn require_navigation_state_current(&self, observation: &Value) -> Result<(), String> {
        let expected: agent_browser_service_model::BrowserSessionState = serde_json::from_value(
            observation
                .get("sessionState")
                .cloned()
                .ok_or_else(|| "browser_runtime_operation_session_state_missing".to_string())?,
        )
        .map_err(|error| format!("browser_runtime_operation_session_state_invalid:{error}"))?;
        if self.state != expected {
            return Err("browser_runtime_navigation_state_conflict".to_string());
        }
        if self.persistence.load_session_state()? != expected {
            return Err("browser_runtime_navigation_state_conflict".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_host::tests::{
        ready_keeper_authority_for_handoff, TempDirectory,
    };
    use crate::native::browser_session_runtime::{
        BrowserRuntimeDriver, BrowserSessionEffectAdapter,
    };
    use crate::native::browser_session_store::{
        BrowserRuntimeSqliteStore, LegacyBrowserRuntimeSources,
    };
    use crate::native::presentation_request_admission::{
        now_ms, PresentationAdmission, PresentationAdmissionRequest,
    };
    use agent_browser_service_model::{
        BrowserLaunch, BrowserProfileCatalogEntry, BrowserTabAcquisition, BrowserTabSource,
        ManagedBrowserInstance, ManagedBrowserTab, PresentationRequestState,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    struct NavigationFixtureRuntime {
        executes: Arc<AtomicUsize>,
        target: Arc<Mutex<(String, String)>>,
        fail_after_effect: bool,
    }

    impl BrowserRuntimeDriver for NavigationFixtureRuntime {
        fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
            Ok(true)
        }

        fn launch_browser(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
        ) -> Result<BrowserLaunch, String> {
            Ok(BrowserLaunch {
                browser_id: format!("browser:{}:fixture", profile.id),
                pid: 4242,
                cdp_endpoint: "ws://fixture".to_string(),
                process_identity: None,
                desktop: desktop.cloned(),
            })
        }

        fn launch_browser_reserved(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
            browser_id: &str,
        ) -> Result<BrowserLaunch, String> {
            Ok(BrowserLaunch {
                browser_id: browser_id.to_string(),
                pid: 4242,
                cdp_endpoint: format!("ws://fixture/{}", profile.id),
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
            Ok(BrowserTabAcquisition {
                tab_id: "tab-fixture".to_string(),
                target_id: "target-fixture".to_string(),
                source: BrowserTabSource::Bootstrap,
            })
        }

        fn create_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            unreachable!("navigation fixture uses the bootstrap tab")
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
            unreachable!("header-bearing navigation uses execute_command")
        }

        fn focus_browser(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: Option<&ManagedBrowserTab>,
        ) -> Result<(), String> {
            Ok(())
        }

        fn observe_navigation_target(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
        ) -> Result<NavigationTargetObservation, String> {
            let (target_id, url) = self.target.lock().unwrap().clone();
            Ok(NavigationTargetObservation { target_id, url })
        }

        fn execute_command(
            &mut self,
            _browser: &ManagedBrowserInstance,
            tab: &ManagedBrowserTab,
            _session_id: &str,
            _session_name: &str,
            command: &Value,
        ) -> Result<Value, String> {
            self.executes.fetch_add(1, Ordering::SeqCst);
            *self.target.lock().unwrap() = (
                tab.target_id.clone(),
                required_string(command, "url")?.to_string(),
            );
            if self.fail_after_effect {
                return Err("injected_navigation_interruption".to_string());
            }
            Ok(json!({ "success": true }))
        }
    }

    fn navigation_fixture() -> (TempDirectory, PathBuf, PathBuf) {
        let directory = TempDirectory::new();
        let session_path = directory.0.join("browser-session-state.json");
        let catalog_path = directory.0.join("browser-profile-catalog.json");
        let legacy_path = directory.0.join("state.json");
        let database_path = directory.0.join("runtime.sqlite3");
        fs::write(
            &legacy_path,
            json!({
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
        (directory, legacy_path, database_path)
    }

    fn navigation_command(id: &str) -> Value {
        json!({
            "id": id,
            "action": "browser_session_navigate",
            "sessionName": "alice",
            "profileId": "work",
            "url": "https://example.test/expected",
            "headers": {"Remote-User": "operator"},
            "activityAtMs": 1_000,
        })
    }

    fn navigation_config(
        authority: &RouteKeeperAuthority,
    ) -> super::super::BrowserSessionHostConfig {
        super::super::BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: super::super::route_keeper_desktop_routes(authority).unwrap(),
            default_disposable_policy: None,
        }
    }

    #[test]
    fn journaled_navigation_bootstraps_then_recovers_issued_target_without_replaying_effect() {
        let (_directory, legacy_path, database_path) = navigation_fixture();
        let authority = ready_keeper_authority_for_handoff();
        let command = navigation_command("navigation-replay-1");
        let executes = Arc::new(AtomicUsize::new(0));
        let target = Arc::new(Mutex::new((
            "target-fixture".to_string(),
            "about:blank".to_string(),
        )));

        let request = PresentationAdmissionRequest::enqueue_at(
            database_path.clone(),
            &command,
            1,
            false,
            now_ms(),
        )
        .unwrap();
        let mut first_permit = match request.poll(Some(true)).unwrap() {
            Some(PresentationAdmission::Execute(permit)) => permit,
            _ => panic!("expected initial execute permit"),
        };
        first_permit.require_current().unwrap();
        let interrupted = {
            let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
            let effects = BrowserSessionEffectAdapter::new(NavigationFixtureRuntime {
                executes: executes.clone(),
                target: target.clone(),
                fail_after_effect: true,
            });
            let mut host = BrowserSessionHost::load(
                store,
                effects,
                &legacy_path,
                navigation_config(&authority),
            )
            .unwrap();
            host.handle_journaled_navigation_with_keeper(&command, &authority)
        };
        assert_eq!(interrupted["success"], false);
        assert_eq!(interrupted["error"], "injected_navigation_interruption");
        assert_eq!(executes.load(Ordering::SeqCst), 1);
        drop(first_permit);
        let pending = BrowserRuntimeSqliteStore::open(&database_path)
            .unwrap()
            .load_operation("navigation-replay-1")
            .unwrap();
        assert_eq!(pending.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(pending.result.unwrap()["phase"], "issued");
        let (bootstrap_handoff_id, bootstrap_handoff_url) =
            BrowserRuntimeSqliteStore::open(&database_path)
                .unwrap()
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .into_iter()
                .next()
                .map(|(id, handoff)| (id, handoff.handoff_url))
                .expect("bootstrap open must publish its keeper handoff");

        let recovered_request = PresentationAdmissionRequest::enqueue_at(
            database_path.clone(),
            &command,
            2,
            false,
            now_ms(),
        )
        .unwrap();
        let mut recovered_permit = match recovered_request.poll(Some(true)).unwrap() {
            Some(PresentationAdmission::Execute(permit)) => permit,
            _ => panic!("expected recovery execute permit"),
        };
        recovered_permit.require_current().unwrap();

        let recovered = {
            let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
            let effects = BrowserSessionEffectAdapter::new(NavigationFixtureRuntime {
                executes: executes.clone(),
                target: target.clone(),
                fail_after_effect: false,
            });
            let mut host = BrowserSessionHost::load(
                store,
                effects,
                &legacy_path,
                navigation_config(&authority),
            )
            .unwrap();
            let response = host.handle_journaled_navigation_with_keeper(&command, &authority);
            assert_eq!(response["success"], true);
            assert_eq!(host.state().navigation_history.len(), 1);
            let replay = host.handle_journaled_navigation_with_keeper(&command, &authority);
            assert_eq!(replay, response);
            assert_eq!(host.state().navigation_history.len(), 1);
            response
        };
        assert_eq!(recovered["data"]["url"], "https://example.test/expected");
        assert_eq!(executes.load(Ordering::SeqCst), 1);
        recovered_permit.finish(Ok(recovered.clone())).unwrap();

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let operation = store.load_operation("navigation-replay-1").unwrap();
        assert_eq!(operation.state, BrowserRuntimeOperationState::Committed);
        assert_eq!(
            store.load_session_state().unwrap().navigation_history.len(),
            1
        );
        assert_eq!(
            store.load_handoff_registry().unwrap().handoffs[&bootstrap_handoff_id].handoff_url,
            bootstrap_handoff_url
        );
        assert!(matches!(
            store.load_presentation_queue().unwrap().entries["browser:navigation-replay-1"].state,
            PresentationRequestState::Completed { .. }
        ));
    }

    #[test]
    fn journaled_navigation_retains_issued_obligation_when_live_target_url_mismatches() {
        let (_directory, legacy_path, database_path) = navigation_fixture();
        let authority = ready_keeper_authority_for_handoff();
        let command = navigation_command("navigation-mismatch-1");
        let executes = Arc::new(AtomicUsize::new(0));
        let target = Arc::new(Mutex::new((
            "target-fixture".to_string(),
            "https://example.test/other".to_string(),
        )));

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let effects = BrowserSessionEffectAdapter::new(NavigationFixtureRuntime {
            executes: executes.clone(),
            target: target.clone(),
            fail_after_effect: true,
        });
        let mut host =
            BrowserSessionHost::load(store, effects, &legacy_path, navigation_config(&authority))
                .unwrap();
        let first = host.handle_journaled_navigation_with_keeper(&command, &authority);
        assert_eq!(first["success"], false);
        assert_eq!(executes.load(Ordering::SeqCst), 1);

        *target.lock().unwrap() = (
            "target-fixture".to_string(),
            "https://example.test/other".to_string(),
        );
        let replay = host.handle_journaled_navigation_with_keeper(&command, &authority);
        assert_eq!(replay["success"], false);
        assert_eq!(
            replay["error"],
            "browser_runtime_navigation_issued_unproven"
        );
        assert_eq!(executes.load(Ordering::SeqCst), 1);
        let pending = host
            .persistence
            .load_operation("navigation-mismatch-1")
            .unwrap();
        assert_eq!(pending.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(pending.result.unwrap()["phase"], "issued");
        assert!(host.state().navigation_history.is_empty());
    }
    #[test]
    fn executed_navigation_requires_live_target_before_atomic_publication() {
        let (_directory, legacy_path, database_path) = navigation_fixture();
        let authority = ready_keeper_authority_for_handoff();
        let command = navigation_command("navigation-executed-1");
        let executes = Arc::new(AtomicUsize::new(0));
        let target = Arc::new(Mutex::new((
            "target-fixture".to_string(),
            "about:blank".to_string(),
        )));
        let mut host = BrowserSessionHost::load(
            BrowserRuntimeSqliteStore::open(&database_path).unwrap(),
            BrowserSessionEffectAdapter::new(NavigationFixtureRuntime {
                executes: executes.clone(),
                target: target.clone(),
                fail_after_effect: true,
            }),
            &legacy_path,
            navigation_config(&authority),
        )
        .unwrap();
        assert_eq!(
            host.handle_journaled_navigation_with_keeper(&command, &authority)["success"],
            false
        );
        let operation = host
            .persistence
            .load_operation("navigation-executed-1")
            .unwrap();
        let mut observation = operation.result.unwrap();
        observation["phase"] = json!("executed");
        host.persistence
            .record_operation_observation(
                "navigation-executed-1",
                operation.generation,
                observation,
            )
            .unwrap();
        drop(host);
        *target.lock().unwrap() = (
            "different-target".to_string(),
            "https://example.test/login".to_string(),
        );
        let mut restarted = BrowserSessionHost::load(
            BrowserRuntimeSqliteStore::open(&database_path).unwrap(),
            BrowserSessionEffectAdapter::new(NavigationFixtureRuntime {
                executes: executes.clone(),
                target: target.clone(),
                fail_after_effect: false,
            }),
            &legacy_path,
            navigation_config(&authority),
        )
        .unwrap();
        let failed = restarted.handle_journaled_navigation_with_keeper(&command, &authority);
        assert_eq!(
            failed["error"],
            "browser_runtime_navigation_executed_target_unavailable"
        );
        assert!(restarted
            .persistence
            .load_session_state()
            .unwrap()
            .navigation_history
            .is_empty());
        let retained = restarted
            .persistence
            .load_operation("navigation-executed-1")
            .unwrap();
        assert_eq!(retained.result.unwrap()["phase"], "executed");
        *target.lock().unwrap() = (
            "target-fixture".to_string(),
            "https://example.test/login".to_string(),
        );
        let recovered = restarted.handle_journaled_navigation_with_keeper(&command, &authority);
        assert_eq!(recovered["success"], true, "{recovered}");
        assert_eq!(
            restarted
                .persistence
                .load_session_state()
                .unwrap()
                .navigation_history
                .len(),
            1
        );
        assert_eq!(executes.load(Ordering::SeqCst), 1);
    }
}
