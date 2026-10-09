//! Provider-free ordinary host/manager/consumer composition with real SQLite.
use super::browser_session_host::{BrowserSessionHost, BrowserSessionHostConfig};
use super::browser_session_store::BrowserSessionSqliteStore;
use agent_browser_service_model::*;
use serde_json::Value;
use std::{cell::Cell, path::PathBuf, rc::Rc};

#[derive(Clone, Copy)]
enum Mode {
    Success,
    IdleRecovery,
    UnknownProcess,
    UnknownTab,
    StaleBaseline,
    CompetingPublication,
}
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("p220-session-consumer-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let fixture = Self { root };
        let store = fixture.store(true);
        let mut catalog = BrowserProfileCatalog::default();
        for id in ["profile-a", "profile-b"] {
            catalog.profiles.insert(
                id.into(),
                BrowserProfileCatalogEntry {
                    id: id.into(),
                    name: id.into(),
                    user_data_dir: fixture.root.join(id).to_string_lossy().into_owned(),
                    kind: BrowserProfileKind::Named,
                },
            );
        }
        store.save_profile_catalog(&catalog).unwrap();
        fixture
    }
    fn store(&self, initialize: bool) -> BrowserSessionSqliteStore {
        open_store(&self.root, initialize)
    }
    fn effects(
        &self,
        mode: Mode,
        calls: Rc<Cell<u32>>,
        requests: Rc<Cell<u32>>,
    ) -> Result<Effects, String> {
        let f = wire();
        RemoteViewSessionEffects::new(
            Process {
                mode,
                root: self.root.clone(),
                calls,
            },
            RemoteViewApplicationAdapter::new("agent-browser".into(), Transport { requests })
                .unwrap(),
            self.store(false),
            vec![serde_json::from_value(f["assignment"].clone()).unwrap()],
            || uuid::Uuid::new_v4().to_string(),
        )
    }
    fn host(&self, mode: Mode, calls: Rc<Cell<u32>>, requests: Rc<Cell<u32>>) -> Host {
        let assignment: RemoteViewAssignmentRecord =
            serde_json::from_value(wire()["assignment"].clone()).unwrap();
        BrowserSessionHost::load(
            self.store(false),
            self.effects(mode, calls, requests).unwrap(),
            &self.root.join("unused-legacy.json"),
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                default_disposable_policy: None,
                remote_view_desktops: vec![RemoteViewDesktopCandidate {
                    ready: true,
                    desktop: RemoteViewFixedDesktop {
                        desktop_id: assignment.desktop_id,
                        generation: assignment.generation,
                        friendly_route_label: "descriptive-label".into(),
                    },
                }],
                exact_url_history_maximum_bytes: 1024,
            },
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn open_store(root: &std::path::Path, initialize: bool) -> BrowserSessionSqliteStore {
    BrowserSessionSqliteStore::launch_custody_fixture(
        rusqlite::Connection::open(root.join("runtime.sqlite3")).unwrap(),
        initialize,
    )
    .unwrap()
}
fn wire() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap()
}
type Effects = RemoteViewSessionEffects<Process, Transport, BrowserSessionSqliteStore>;
type Host = BrowserSessionHost<BrowserSessionSqliteStore, Effects>;
struct Transport {
    requests: Rc<Cell<u32>>,
}
impl RemoteViewApplicationTransport for Transport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        assert_eq!(envelope.application, "agent-browser");
        self.requests.set(self.requests.get() + 1);
        match envelope.request {
            RemoteViewApplicationRequest::ObserveAssignment { .. } => {
                Ok(wire()["assignmentObservation"].clone())
            }
            RemoteViewApplicationRequest::LaunchEnvironment { .. } => {
                Ok(wire()["launchEnvironment"].clone())
            }
            _ => panic!("unexpected effect in launch composition"),
        }
    }
}
struct Process {
    mode: Mode,
    root: PathBuf,
    calls: Rc<Cell<u32>>,
}
fn competing_write(root: &std::path::Path) {
    let mut store = open_store(root, false);
    let state = store.load_session_state().unwrap();
    let mut changed = state.clone();
    changed.next_session_sequence += 1;
    store
        .compare_and_save_session_state(&state, &changed)
        .unwrap();
}
impl RemoteViewBrowserProcessEffects for Process {
    fn preflight_launch(&mut self, _profile: &BrowserProfileCatalogEntry) -> Result<(), String> {
        if self.root.join("preflight-pressure").exists() {
            Err("browser_launch_resource_pressure:browser_process_capacity_exhausted".into())
        } else {
            Ok(())
        }
    }

    fn prove_recovery_absence(
        &mut self,
        _browser: &ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
    ) -> Result<(), RemoteViewBrowserProcessError> {
        if self.root.join("idle-process-absent").exists()
            && !self.root.join("recovery-absence-unknown").exists()
        {
            Ok(())
        } else {
            Err(RemoteViewBrowserProcessError::Rejected)
        }
    }

    fn recover_launch(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        self.prove_recovery_absence(browser, profile)?;
        let mut launch = self.launch(profile, intent, environment, observation)?;
        launch.browser_id = browser.id.clone();
        std::fs::remove_file(self.root.join("idle-process-absent")).unwrap();
        std::fs::write(
            self.root.join("recovered-target-generation"),
            self.calls.get().to_string(),
        )
        .unwrap();
        Ok(launch)
    }

    fn launch(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        let environment = environment
            .into_environment(observation, &intent.assignment)
            .unwrap();
        assert_eq!(
            serde_json::to_value(environment).unwrap(),
            wire()["launchEnvironment"]["environment"]
        );
        let mut peer = open_store(&self.root, false);
        let records = peer.unpublished_launch_records().unwrap();
        assert_eq!(
            records
                .iter()
                .filter(|record| record.intent.profile_id == profile.id)
                .count(),
            1
        );
        assert!(records.iter().any(|record| record.intent == *intent));
        self.calls.set(self.calls.get() + 1);
        if matches!(self.mode, Mode::UnknownProcess) {
            return Err(RemoteViewBrowserProcessError::OutcomeUnknown);
        }
        if matches!(self.mode, Mode::CompetingPublication) {
            competing_write(&self.root);
        }
        let live_endpoint = self.root.join("live-cdp-endpoint.json");
        let live: Option<Value> = live_endpoint
            .exists()
            .then(|| serde_json::from_slice(&std::fs::read(live_endpoint).unwrap()).unwrap());
        Ok(BrowserLaunch {
            browser_id: format!("browser:{}", profile.id),
            pid: live.as_ref().map_or(42 + self.calls.get(), |value| {
                value["pid"].as_u64().unwrap() as u32
            }),
            cdp_endpoint: live.as_ref().map_or_else(
                || "http://127.0.0.1:9222".into(),
                |value| value["endpoint"].as_str().unwrap().into(),
            ),
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation,
                friendly_route_label: String::new(),
            }),
        })
    }
}
impl BrowserSessionEffects for Process {
    fn begin_operation(&mut self, _expected: &BrowserSessionState) -> Result<(), String> {
        if matches!(self.mode, Mode::StaleBaseline) {
            competing_write(&self.root);
        }
        Ok(())
    }
    fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
        Ok(!self.root.join("idle-process-absent").exists())
    }
    fn launch_browser(
        &mut self,
        _profile: &BrowserProfileCatalogEntry,
        _desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        panic!("ordinary launch must use consumer custody")
    }
    fn close_browser(&mut self, _browser: &ManagedBrowserInstance) -> Result<(), String> {
        if matches!(self.mode, Mode::IdleRecovery) {
            std::fs::write(self.root.join("idle-process-absent"), "absent").unwrap();
        }
        Ok(())
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        let suffix = if self.root.join("recovered-target-generation").exists() {
            format!(":recovered-{}", self.calls.get())
        } else if attributed.is_empty() {
            String::new()
        } else {
            format!(":{}", uuid::Uuid::new_v4())
        };
        Ok(BrowserTabAcquisition {
            tab_id: format!("tab:{}{suffix}", browser.id),
            target_id: format!("target:{}{suffix}", browser.id),
            source: BrowserTabSource::SessionInitial,
        })
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        if matches!(self.mode, Mode::UnknownTab) {
            return Err("synthetic_tab_reply_lost".into());
        }
        let suffix = uuid::Uuid::new_v4();
        Ok(BrowserTabAcquisition {
            tab_id: format!("tab:{}:{suffix}", browser.id),
            target_id: format!("target:{}:{suffix}", browser.id),
            source: BrowserTabSource::ExplicitNew,
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
    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation: &str,
        session: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        Ok(BrowserProfileCatalogEntry {
            id: allocation.into(),
            name: session.into(),
            user_data_dir: std::path::Path::new(&policy.user_data_root)
                .join(allocation.replace(':', "-"))
                .to_string_lossy()
                .into_owned(),
            kind: BrowserProfileKind::Disposable,
        })
    }
    fn delete_disposable_profile(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<(), String> {
        std::fs::write(
            self.root.join("deleted-disposable-profile"),
            &allocation.profile.id,
        )
        .unwrap();
        Ok(())
    }
}

impl ManagedBrowserCommandEffects for Process {
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &Value,
    ) -> Result<Value, String> {
        assert!(tab.target_id.starts_with(&format!("target:{}", browser.id)));
        assert!(!session_id.is_empty());
        assert!(matches!(session_name, "Alice" | "Bob"));
        if command["action"] == "navigate" {
            return Ok(
                serde_json::json!({"id": command["id"], "success": command["url"] != "https://synthetic.example/failure", "data": {"url": command["url"], "headers": command["headers"], "waitUntil": command["waitUntil"]}}),
            );
        }
        assert_eq!(command["action"], "get_url");
        Ok(serde_json::json!({"id":command["id"], "success":true,
            "data":{"targetId":tab.target_id, "url":"https://synthetic.example/"},
            "targetId": tab.target_id, "url": "https://synthetic.example/"}))
    }
}

#[test]
fn consumer_preflight_failure_preserves_reason_and_allows_same_host_retry() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let pressure = fixture.root.join("preflight-pressure");
    std::fs::write(&pressure, "synthetic pressure").unwrap();
    let error = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap_err();
    assert_eq!(
        error,
        "browser_launch_resource_pressure:browser_process_capacity_exhausted"
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(requests.get(), 0);
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
    std::fs::remove_file(pressure).unwrap();
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 101))
        .unwrap();
    assert_eq!(opened.profile_id, "profile-a");
    assert_eq!(calls.get(), 1);
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn consumer_host_managed_dispatch_uses_current_tab_after_publication() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let result = host
        .execute_managed_command(
            "Alice",
            &serde_json::json!({"action": "get_url", "activityAtMs": 101}),
        )
        .unwrap()
        .unwrap();
    assert_eq!(result["targetId"], format!("target:{}", opened.browser_id));
    assert_eq!(calls.get(), 1);
    assert_eq!(requests.get(), 4);
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn consumer_host_open_reuse_and_colocation_publish_real_custody() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let alice = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let bob = host
        .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 101))
        .unwrap();
    assert_eq!(alice.browser_id, bob.browser_id);
    assert_eq!(calls.get(), 1);
    assert_eq!(requests.get(), 4);
    let second = host
        .open(OpenBrowserSession::exact_profile("Other", "profile-b", 102))
        .unwrap();
    assert_ne!(second.browser_id, alice.browser_id);
    assert_eq!(host.state().browsers.len(), 2);
    let desktops: Vec<_> = host
        .state()
        .browsers
        .values()
        .map(|browser| browser.desktop.clone())
        .collect();
    assert_eq!(desktops[0], desktops[1]);
    assert_eq!(calls.get(), 2);
    assert_eq!(requests.get(), 8);
    let mut peer = fixture.store(false);
    assert_eq!(peer.load_session_state().unwrap(), *host.state());
    assert!(peer.unpublished_launch_records().unwrap().is_empty());
    drop(host);
    let mut restarted = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let reused = restarted
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 103))
        .unwrap();
    assert_eq!(reused.session_id, alice.session_id);
    assert_eq!(calls.get(), 2);
    assert_eq!(requests.get(), 8);
}

#[test]
fn consumer_shared_profile_tabs_route_independently_and_close_preserves_peer() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests);
    let alice = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let bob = host
        .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 101))
        .unwrap();
    assert_eq!(alice.browser_id, bob.browser_id);
    assert_ne!(alice.session_id, bob.session_id);
    // Initial tabs are acquired lazily by the first addressed command.
    for name in ["Alice", "Bob"] {
        host.execute_managed_command(
            name,
            &serde_json::json!({"action":"get_url", "activityAtMs":102}),
        )
        .unwrap()
        .unwrap();
    }
    let alice_tab = host.state().sessions[&alice.session_id]
        .current_tab_id
        .clone()
        .unwrap();
    let bob_tab = host.state().sessions[&bob.session_id]
        .current_tab_id
        .clone()
        .unwrap();
    assert_ne!(alice_tab, bob_tab);
    let alice_target = host.state().tabs[&alice_tab].target_id.clone();
    let bob_target = host.state().tabs[&bob_tab].target_id.clone();
    assert_ne!(alice_target, bob_target);
    for (name, target) in [("Alice", &alice_target), ("Bob", &bob_target)] {
        let result = host
            .execute_managed_command(
                name,
                &serde_json::json!({"action":"get_url", "activityAtMs":102}),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result["targetId"], *target);
    }
    host.close_session(&alice.session_id, SessionEndReason::ExplicitClose, 103)
        .unwrap();
    assert!(!host.state().sessions.contains_key(&alice.session_id));
    assert!(host.state().browsers.contains_key(&bob.browser_id));
    let continued = host
        .execute_managed_command(
            "Bob",
            &serde_json::json!({"action":"get_url", "activityAtMs":104}),
        )
        .unwrap()
        .unwrap();
    assert_eq!(continued["targetId"], bob_target);
    host.close_session(&bob.session_id, SessionEndReason::ExplicitClose, 105)
        .unwrap();
    assert!(host.state().browsers.is_empty());
    assert_eq!(calls.get(), 1);
}

#[test]
fn consumer_host_failed_launch_or_publication_retains_claim_without_restart_retry() {
    for mode in [Mode::UnknownProcess, Mode::CompetingPublication] {
        let fixture = Fixture::new();
        let calls = Rc::new(Cell::new(0));
        let requests = Rc::new(Cell::new(0));
        let mut host = fixture.host(mode, calls.clone(), requests.clone());
        assert!(host
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
            .is_err());
        assert!(host
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 101))
            .is_err());
        assert_eq!(calls.get(), 1);
        assert_eq!(requests.get(), 4);
        let mut peer = fixture.store(false);
        let records = peer.unpublished_launch_records().unwrap();
        assert_eq!(records.len(), 1);
        assert!(!records[0].published);
        assert_eq!(
            records[0].observed_pid.is_some(),
            matches!(mode, Mode::CompetingPublication)
        );
        assert!(peer.load_session_state().unwrap().browsers.is_empty());
        drop(host);
        let mut restarted = fixture.host(Mode::Success, calls.clone(), requests.clone());
        assert!(restarted
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 102))
            .is_err());
        assert_eq!(peer.unpublished_launch_records().unwrap(), records);
        assert_eq!(calls.get(), 1);
        assert_eq!(requests.get(), 4);
    }
}

#[test]
fn consumer_host_stale_baseline_rejects_before_provider_read_or_process() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::StaleBaseline, calls.clone(), requests.clone());
    assert!(host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .is_err());
    assert_eq!(calls.get(), 0);
    assert_eq!(requests.get(), 0);
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[path = "browser_session_pool_tests.rs"]
mod pool_tests;
