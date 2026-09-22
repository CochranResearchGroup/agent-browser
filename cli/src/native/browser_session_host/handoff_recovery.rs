//! Durable intent for repeatable handoff focus, with fresh readiness on replay.

use agent_browser_service_model::{BrowserSessionEffects, RemoteViewHandoff, RouteKeeperAuthority};
use serde_json::{json, Value};

use super::{
    required_string, BrowserRuntimeOperationState, BrowserSessionHost, BrowserSessionPersistence,
};

fn handoff_operation_request(command: &Value, handoff: &RemoteViewHandoff) -> Value {
    json!({
        "schemaVersion": "agent-browser.browser-handoff-operation.v1",
        "command": command,
        "identity": {
            "handoffId": handoff.id,
            "profileId": handoff.profile_id,
            "browserId": handoff.browser_id,
            "sessionName": handoff.session_name,
            "sessionId": handoff.intent.get("sessionId"),
            "tabId": handoff.tab_id,
            "targetId": handoff.target_id,
            "handoffUrl": handoff.handoff_url,
            "routeId": handoff.last_route_id,
            "presentationSlotId": handoff.intent.get("presentationSlotId"),
        },
    })
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    /// Retain logical identity before focus. Replays always resolve against the
    /// current keeper and browser; an old committed receipt is not visibility.
    pub(crate) fn resolve_journaled_manager_handoff_with_keeper(
        &mut self,
        command: &Value,
        handoff: &RemoteViewHandoff,
        authority: &RouteKeeperAuthority,
        activity_at_ms: u64,
    ) -> Result<Value, String> {
        if command.get("action").and_then(Value::as_str)
            != Some("service_remote_view_handoff_resolve")
        {
            return Err("browser_runtime_handoff_action_invalid".to_string());
        }
        let operation_id = required_string(command, "id")?;
        let request = handoff_operation_request(command, handoff);
        let operation = self
            .persistence
            .reserve_operation(operation_id, "browser-runtime-handoff", request)?
            .ok_or_else(|| "browser_runtime_operation_persistence_unsupported".to_string())?;
        // Focus is idempotent. The resolver checks current session expiry,
        // exact target attribution, live browser and current keeper binding.
        let activation = super::handoff_control::HandoffControlActivation {
            operation_id,
            client_connection_id: operation_id,
        };
        let result =
            self.resolve_keeper_handoff(handoff, authority, activity_at_ms, Some(&activation))?;
        if operation.state != BrowserRuntimeOperationState::Committed {
            self.persistence
                .commit_operation(operation_id, operation.generation, result.clone())?
                .ok_or_else(|| "browser_runtime_operation_persistence_unsupported".to_string())?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use agent_browser_service_model::{OpenBrowserSession, RouteKeeperPhase};

    use super::super::tests::{ready_keeper_authority_for_handoff, FixtureRuntime, TempDirectory};
    use super::super::{
        route_keeper_desktop_routes, BrowserSessionEffectAdapter, BrowserSessionHostConfig,
    };
    use super::*;
    use crate::native::browser_session_store::{
        BrowserRuntimeSqliteStore, LegacyBrowserRuntimeSources,
    };

    type TestHost =
        BrowserSessionHost<BrowserRuntimeSqliteStore, BrowserSessionEffectAdapter<FixtureRuntime>>;

    struct Fixture {
        _directory: TempDirectory,
        database_path: PathBuf,
        legacy_path: PathBuf,
        handoff: RemoteViewHandoff,
        authority: RouteKeeperAuthority,
        command: Value,
        focuses: Arc<AtomicUsize>,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = TempDirectory::new();
            let legacy_path = directory.0.join("state.json");
            let database_path = directory.0.join("runtime.sqlite3");
            fs::write(
                &legacy_path,
                json!({"profiles":{"work":{
                    "id":"work","name":"Work","userDataDir":directory.0.join("work"),
                    "profileClass":"durable_named"
                }}})
                .to_string(),
            )
            .unwrap();
            BrowserRuntimeSqliteStore::migrate_from_legacy(
                &database_path,
                LegacyBrowserRuntimeSources {
                    session_state_path: &directory.0.join("sessions.json"),
                    profile_catalog_path: &directory.0.join("profiles.json"),
                    service_state_path: &legacy_path,
                },
            )
            .unwrap();
            let authority = ready_keeper_authority_for_handoff();
            let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
            let previous = store.load_route_keeper_authority().unwrap();
            store
                .compare_and_swap_route_keeper_authority(&previous, &authority)
                .unwrap();
            drop(store);
            let focuses = Arc::new(AtomicUsize::new(0));
            let mut host = load_host(&database_path, &legacy_path, &authority, focuses.clone());
            let opened = host
                .open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
                .unwrap();
            let tab = host.new_tab(&opened.session_id, 1_100).unwrap();
            let mut response = json!({"success":true,"data":{
                "sessionId":opened.session_id,"browserId":opened.browser_id,
                "tabId":tab.tab_id,"targetId":tab.target_id
            }});
            host.attach_keeper_manager_handoff(&mut response, &authority)
                .unwrap();
            let handoff = host
                .manager_handoff(response["data"]["handoffId"].as_str().unwrap())
                .unwrap()
                .clone();
            drop(host);
            Self {
                _directory: directory,
                database_path,
                legacy_path,
                command: json!({
                    "id":"handoff-operation",
                    "action":"service_remote_view_handoff_resolve",
                    "handoffId":handoff.id
                }),
                handoff,
                authority,
                focuses,
            }
        }

        fn host(&self) -> TestHost {
            load_host(
                &self.database_path,
                &self.legacy_path,
                &self.authority,
                self.focuses.clone(),
            )
        }
    }

    fn load_host(
        database_path: &Path,
        legacy_path: &Path,
        authority: &RouteKeeperAuthority,
        focuses: Arc<AtomicUsize>,
    ) -> TestHost {
        BrowserSessionHost::load(
            BrowserRuntimeSqliteStore::open(database_path).unwrap(),
            BrowserSessionEffectAdapter::new(FixtureRuntime {
                live: true,
                focuses: Some(focuses),
                ..FixtureRuntime::default()
            }),
            legacy_path,
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_desktop_routes: route_keeper_desktop_routes(authority).unwrap(),
                default_disposable_policy: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn prepared_restart_replays_exact_stable_handoff() {
        let fixture = Fixture::new();
        let mut host = fixture.host();
        host.persistence
            .reserve_operation(
                "handoff-operation",
                "browser-runtime-handoff",
                handoff_operation_request(&fixture.command, &fixture.handoff),
            )
            .unwrap();
        drop(host);

        let mut restarted = fixture.host();
        let first = restarted
            .resolve_journaled_manager_handoff_with_keeper(
                &fixture.command,
                &fixture.handoff,
                &fixture.authority,
                2_000,
            )
            .unwrap();
        let replay = restarted
            .resolve_journaled_manager_handoff_with_keeper(
                &fixture.command,
                &fixture.handoff,
                &fixture.authority,
                2_000,
            )
            .unwrap();
        assert_eq!(replay, first);
        assert_eq!(
            restarted
                .persistence
                .load_operation("handoff-operation")
                .unwrap()
                .state,
            BrowserRuntimeOperationState::Committed
        );
        assert_eq!(fixture.focuses.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn committed_journal_still_requires_current_keeper() {
        let fixture = Fixture::new();
        let mut host = fixture.host();
        host.resolve_journaled_manager_handoff_with_keeper(
            &fixture.command,
            &fixture.handoff,
            &fixture.authority,
            2_000,
        )
        .unwrap();
        let focused = fixture.focuses.load(Ordering::SeqCst);
        let mut unavailable = fixture.authority.clone();
        unavailable.records.get_mut("route-slot-01").unwrap().phase = RouteKeeperPhase::Degraded;
        assert_eq!(
            host.resolve_journaled_manager_handoff_with_keeper(
                &fixture.command,
                &fixture.handoff,
                &unavailable,
                2_100,
            ),
            Err("route_keeper_handoff_not_ready".to_string())
        );
        assert_eq!(fixture.focuses.load(Ordering::SeqCst), focused);
    }

    #[test]
    fn changed_identity_or_command_is_rejected_before_focus() {
        for changed_identity in [true, false] {
            let fixture = Fixture::new();
            let mut host = fixture.host();
            host.persistence
                .reserve_operation(
                    "handoff-operation",
                    "browser-runtime-handoff",
                    handoff_operation_request(&fixture.command, &fixture.handoff),
                )
                .unwrap();
            let mut command = fixture.command.clone();
            let mut handoff = fixture.handoff.clone();
            if changed_identity {
                handoff.target_id = Some("different-target".to_string());
            } else {
                command["handoffId"] = "different-handoff".into();
            }
            assert!(matches!(
                host.resolve_journaled_manager_handoff_with_keeper(
                    &command,
                    &handoff,
                    &fixture.authority,
                    2_000,
                ),
                Err(error) if error == "browser_runtime_operation_replay_mismatch:handoff-operation"
            ));
            assert_eq!(fixture.focuses.load(Ordering::SeqCst), 0);
        }
    }
    #[test]
    fn successor_activation_fences_old_focus_replay() {
        let fixture = Fixture::new();
        let mut host = fixture.host();
        let first = host
            .resolve_journaled_manager_handoff_with_keeper(
                &fixture.command,
                &fixture.handoff,
                &fixture.authority,
                2_000,
            )
            .unwrap();
        let mut next = fixture.command.clone();
        next["id"] = "second-viewer-activation".into();
        let second = host
            .resolve_journaled_manager_handoff_with_keeper(
                &next,
                &fixture.handoff,
                &fixture.authority,
                2_100,
            )
            .unwrap();
        assert!(
            second["desktopControl"]["epoch"].as_u64().unwrap()
                > first["desktopControl"]["epoch"].as_u64().unwrap()
        );
        let focuses = fixture.focuses.load(Ordering::SeqCst);
        assert!(host
            .resolve_journaled_manager_handoff_with_keeper(
                &fixture.command,
                &fixture.handoff,
                &fixture.authority,
                2_200,
            )
            .is_err());
        assert_eq!(fixture.focuses.load(Ordering::SeqCst), focuses);
        assert_eq!(second["desktopControl"]["state"], "focus_authorized");
        assert_eq!(second["handoffUrl"], first["handoffUrl"]);
    }
}
