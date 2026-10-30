//! Provider-free ordinary host/manager/consumer composition with real SQLite.
use super::browser_session_host::{BrowserSessionHost, BrowserSessionHostConfig};
use super::browser_session_store::BrowserSessionSqliteStore;
use agent_browser_service_model::*;
use serde_json::Value;
use std::{cell::Cell, path::PathBuf, rc::Rc};

#[derive(Clone, Copy)]
enum Mode {
    Success,
    LiveProfileReuse,
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
        Self::with_root(root, true)
    }
    fn with_root(root: PathBuf, initialize: bool) -> Self {
        let fixture = Self { root };
        let store = fixture.store(initialize);
        let mut catalog = BrowserProfileCatalog::default();
        for id in ["profile-a", "profile-b"] {
            catalog.profiles.insert(
                id.into(),
                BrowserProfileCatalogEntry {
                    manual_login_url: None,
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
        let effects = RemoteViewSessionEffects::new(
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
        )?;
        if self.root.join("automatic-pool").exists() {
            effects.with_pool(RemoteViewSessionPool {
                name: "main".into(),
                desired_desktops: 1,
            })
        } else {
            Ok(effects)
        }
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
        // Production initialization seals its migration archive. This fixture
        // owns the disposable directory and makes only its archive removable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for entry in std::fs::read_dir(&self.root).unwrap() {
                let entry = entry.unwrap();
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("migration-archive-")
                    && entry.file_type().unwrap().is_dir()
                {
                    std::fs::set_permissions(entry.path(), std::fs::Permissions::from_mode(0o700))
                        .unwrap();
                }
            }
        }
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
            RemoteViewApplicationRequest::Inventory {} => Ok(wire()["inventory"].clone()),
            RemoteViewApplicationRequest::Windows { .. } => {
                let mut windows = wire()["windows"].clone();
                windows["windows"] = serde_json::json!([]);
                Ok(windows)
            }
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
        let expected_environment =
            match std::fs::read(self.root.join("native-composition-launch-environment.json")) {
                Ok(bytes) => serde_json::from_slice::<Value>(&bytes).unwrap(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    wire()["launchEnvironment"]["environment"].clone()
                }
                Err(error) => panic!("synthetic launch environment unavailable: {error}"),
            };
        assert_eq!(
            serde_json::to_value(environment).unwrap(),
            expected_environment
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
        if self
            .root
            .join("native-composition-launch-environment.json")
            .exists()
        {
            std::fs::write(
                self.root.join("native-composition-browser-pid"),
                (42 + self.calls.get()).to_string(),
            )
            .unwrap();
        }
        if matches!(self.mode, Mode::UnknownProcess) {
            return Err(RemoteViewBrowserProcessError::OutcomeUnknown);
        }
        if matches!(self.mode, Mode::CompetingPublication) {
            competing_write(&self.root);
        }
        if profile.manual_login_url.is_some() {
            std::fs::write(
                self.root.join("manual-launch-profile.json"),
                serde_json::to_vec(profile).unwrap(),
            )
            .unwrap();
        }
        let live_endpoint = self.root.join("live-cdp-endpoint.json");
        let live: Option<Value> = live_endpoint
            .exists()
            .then(|| serde_json::from_slice(&std::fs::read(live_endpoint).unwrap()).unwrap());
        let pid = live.as_ref().map_or(42 + self.calls.get(), |value| {
            value["pid"].as_u64().unwrap() as u32
        });
        Ok(BrowserLaunch {
            // Scripted current launch evidence, with an explicit legacy case.
            process_identity: (!std::fs::read_to_string(self.root.join("legacy-launch-no-kernel"))
                .ok()
                .is_some_and(|legacy| legacy.trim() == profile.id))
            .then(|| RecordedProcessIdentity {
                pid,
                start_token: format!("synthetic-original:{pid}"),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: None,
            }),
            browser_id: format!("browser:{}", profile.id),
            pid,
            cdp_endpoint: if profile.manual_login_url.is_some() {
                String::new()
            } else {
                live.as_ref().map_or_else(
                    || "http://127.0.0.1:9222".into(),
                    |value| value["endpoint"].as_str().unwrap().into(),
                )
            },
            desktop: Some(RemoteViewFixedDesktop {
                desktop_id: intent.assignment.desktop_id.clone(),
                generation: intent.assignment.generation,
                friendly_route_label: String::new(),
            }),
        })
    }
}
impl Process {
    fn begin_tab_acquisition(&self) -> Result<(), String> {
        let path = self.root.join("tab-acquisition-attempts");
        let attempts =
            std::fs::read_to_string(&path).map_or(0, |value| value.parse::<u32>().unwrap());
        std::fs::write(path, (attempts + 1).to_string()).unwrap();
        if matches!(self.mode, Mode::UnknownTab) {
            return Err("synthetic_tab_reply_lost".into());
        }
        Ok(())
    }
}
struct TestRetirementGuard;
impl BrowserIdleRetirementGuard for TestRetirementGuard {}

impl BrowserSessionEffects for Process {
    fn select_profile_browser_executable(
        &mut self,
        profile_id: String,
        path: String,
    ) -> Result<(), String> {
        std::fs::write(
            self.root.join("selected-profile-executable.json"),
            serde_json::json!({"profileId":profile_id,"path":path}).to_string(),
        )
        .map_err(|_| "fixture_profile_pin_unavailable".to_string())
    }

    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        if !self.root.join("check-retirement-read-exclusion").exists() {
            return Ok(Some(Box::new(TestRetirementGuard)));
        }
        let repository = super::service_store::LockedServiceStateRepository::default_json()?;
        let guard = repository.hold_snapshot_for_effect()?;
        for browser in browsers {
            if !super::service_resources::permits_managed_browser_idle_retirement(
                &guard.state,
                browser,
                &chrono::Utc::now().to_rfc3339(),
            )? {
                return Ok(None);
            }
        }
        Ok(Some(Box::new(guard)))
    }
    fn reconcile_profile_browser(
        &mut self,
        _profile: &BrowserProfileCatalogEntry,
        browser: Option<&ManagedBrowserInstance>,
        tabs: &[ManagedBrowserTab],
        _navigation: &[BrowserNavigationRecord],
    ) -> Result<Option<BrowserRecovery>, String> {
        if !matches!(self.mode, Mode::LiveProfileReuse) {
            return Ok(None);
        }
        Ok(Some(BrowserRecovery {
            launch: BrowserLaunch {
                process_identity: None,
                browser_id: browser
                    .map(|browser| browser.id.clone())
                    .unwrap_or_else(|| "shared-browser".into()),
                pid: 91,
                cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/synthetic".into(),
                desktop: None,
            },
            target_ids: tabs
                .iter()
                .map(|tab| (tab.id.clone(), tab.target_id.clone()))
                .collect(),
        }))
    }
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
    fn apply_idle_retirement_effect(
        &mut self,
        scope: &BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        if self.root.join("replacement-kernel-observed").exists() {
            assert!(scope.process_identity.is_some());
            return Ok(()); // Scripted positive proof that the original ended.
        }
        match tab {
            Some(tab) => self.close_tab(&scope.browser, tab),
            None => self.close_browser(&scope.browser),
        }
    }
    fn observe_idle_retirement_effect(
        &mut self,
        scope: &BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<bool, String> {
        let count_path = self.root.join("activity-observation-count");
        if count_path.exists() {
            let count = std::fs::read_to_string(&count_path)
                .unwrap()
                .parse::<u32>()
                .unwrap();
            std::fs::write(count_path, (count + 1).to_string()).unwrap();
            if self
                .root
                .join("activity-count-readback-unavailable")
                .exists()
            {
                return Err("synthetic_activity_observation_unavailable".into());
            }
        }
        if self.root.join("activity-original-readback").exists() {
            assert!(
                tab.is_none(),
                "activity reconciliation observes the original root only"
            );
            assert_eq!(
                scope.process_identity.as_ref().unwrap().start_token,
                format!("synthetic-original:{}", scope.browser.pid)
            );
            if self.root.join("activity-readback-unavailable").exists() {
                return Err("synthetic_original_activity_readback_unavailable".into());
            }
        }
        Ok(self.root.join("idle-process-absent").exists())
    }
    fn close_browser(&mut self, _browser: &ManagedBrowserInstance) -> Result<(), String> {
        if self.root.join("count-retirement-closes").exists() {
            let path = self.root.join("retirement-effect-count");
            let count =
                std::fs::read_to_string(&path).map_or(0, |count| count.parse::<u32>().unwrap());
            std::fs::write(path, (count + 1).to_string()).unwrap();
        }
        if self.root.join("fail-retirement-close").exists() {
            return Err("synthetic_close_outcome_unknown".into());
        }
        if self.root.join("require-retirement-custody").exists() {
            let count_path = self.root.join("retirement-close-count");
            let count = std::fs::read_to_string(&count_path)
                .map_or(0, |count| count.parse::<u32>().unwrap());
            std::fs::write(count_path, (count + 1).to_string()).unwrap();
            let mut store = open_store(&self.root, false);
            let records = store.idle_retirement_records().unwrap();
            let record = records
                .iter()
                .find(|record| {
                    record
                        .intent
                        .scopes
                        .iter()
                        .any(|scope| scope.browser.id == _browser.id)
                })
                .unwrap();
            let encoded = serde_json::to_value(record).unwrap();
            assert_eq!(
                encoded["effects"].as_array().unwrap().last().unwrap()["settled"],
                false
            );
            assert!(
                self.root.join("native-retirement-fence").exists(),
                "close ran without native admission exclusion"
            );
        }
        if matches!(self.mode, Mode::IdleRecovery) {
            std::fs::write(self.root.join("idle-process-absent"), "absent").unwrap();
        }
        if self.root.join("lose-retirement-close-reply").exists() {
            std::fs::remove_file(self.root.join("lose-retirement-close-reply")).unwrap();
            return Err("synthetic_close_reply_lost".into());
        }
        Ok(())
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.begin_tab_acquisition()?;
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
        self.begin_tab_acquisition()?;
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
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        if std::fs::read_to_string(self.root.join("fail-task-close-target"))
            .ok()
            .as_deref()
            == Some(&tab.target_id)
        {
            let path = self.root.join("failed-task-close-count");
            let count = std::fs::read_to_string(&path).map_or(0, |n| n.parse::<u32>().unwrap());
            std::fs::write(path, (count + 1).to_string()).unwrap();
            return Err("synthetic_task_close_outcome_unknown".into());
        }
        Ok(())
    }
    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        _tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        if std::fs::read_to_string(self.root.join("command-activity-unknown-profile"))
            .is_ok_and(|profile| profile == browser.profile_id)
            && url == "https://synthetic.example/unknown"
        {
            let count_path = self.root.join("unknown-navigation-count");
            let count = std::fs::read_to_string(&count_path)
                .map_or(0, |value| value.parse::<u32>().unwrap());
            std::fs::write(count_path, (count + 1).to_string()).unwrap();
            return Err("synthetic_navigation_outcome_unknown".into());
        }
        let race = self.root.join("command-retirement-race.json");
        if race.exists() {
            use super::browser_session_runtime::{BrowserManagerRuntime, BrowserRuntimeDriver};
            let packet: Value = serde_json::from_slice(&std::fs::read(race).unwrap()).unwrap();
            let mut runtime = BrowserManagerRuntime::start(Default::default()).unwrap();
            let guard = runtime.acquire_idle_retirement_guard(std::slice::from_ref(browser))?;
            if let Some(_guard) = guard {
                let intent: BrowserIdleRetirementIntent =
                    serde_json::from_value(packet["intent"].clone()).unwrap();
                let expected: BrowserSessionState =
                    serde_json::from_value(packet["expected"].clone()).unwrap();
                match open_store(&self.root, false).claim_idle_retirement(&intent, &expected) {
                    Ok(BrowserIdleRetirementAdmission::New) => {}
                    Ok(_) => panic!("fresh competing retirement identity must not replay"),
                    Err(error) => assert_eq!(
                        format!("{error:?}"),
                        "ActivityActive",
                        "unrelated setup rejection is not admission exclusion"
                    ),
                }
            }
            std::fs::write(self.root.join("racing-navigation-effect"), "once").unwrap();
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
    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation: &str,
        session: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        Ok(BrowserProfileCatalogEntry {
            manual_login_url: None,
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
        if self.root.join("fail-profile-delete").exists() {
            return Err("synthetic_profile_delete_failed".into());
        }
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
        assert!(matches!(
            command["action"].as_str(),
            Some("get_url" | "url")
        ));
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
fn consumer_exact_task_route_survives_duplicate_names_and_rejects_missing_identity() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let first = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let second = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-b", 101))
        .unwrap();
    assert_ne!(first.session_id, second.session_id);
    assert!(host
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"url", "activityAtMs":102
            })
        )
        .unwrap_err()
        .starts_with("browser_session_name_ambiguous:"));
    for task in [&first, &second] {
        let result = host
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "action":"url", "sessionId":task.session_id, "activityAtMs":102
                }),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result["targetId"], format!("target:{}", task.browser_id));
        let added = host
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "id":format!("add-{}", task.session_id), "action":"tab_new",
                    "sessionId":task.session_id, "activityAtMs":103
                }),
            )
            .unwrap()
            .unwrap();
        assert_eq!(added["browserSession"]["sessionId"], task.session_id);
        assert_eq!(added["browserSession"]["browserId"], task.browser_id);
    }
    let before = calls.get();
    assert!(host.execute_managed_command("Alice", &serde_json::json!({
        "action":"url", "sessionId":first.session_id, "profileId":"profile-b", "activityAtMs":103
    })).is_err(), "conflicting profile intent must not select the peer");
    assert!(host
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"url", "sessionId":"missing-task", "activityAtMs":103
            })
        )
        .is_err());
    assert_eq!(
        calls.get(),
        before,
        "an unknown exact ID must not launch a fallback browser"
    );
    assert!(
        host.execute_managed_command(
            "Bob",
            &serde_json::json!({
                "action":"url", "sessionId":first.session_id, "activityAtMs":104
            })
        )
        .is_err(),
        "an exact ID must not override conflicting name intent"
    );
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

#[test]
fn consumer_leave_open_close_preserves_exact_browser_and_task_for_continuation() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    host.execute_managed_command(
        "Alice",
        &serde_json::json!({"action":"get_url", "activityAtMs":101}),
    )
    .unwrap()
    .unwrap();
    let browser = host.state().browsers[&opened.browser_id].clone();
    let session = host.state().sessions[&opened.session_id].clone();
    let response = host.handle_command(&serde_json::json!({
        "action":"browser_session_close", "sessionId":opened.session_id,
        "leaveOpen":true, "activityAtMs":102
    }));
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["disposition"], "browser_preserved");
    assert_eq!(host.state().browsers[&opened.browser_id], browser);
    assert_eq!(host.state().sessions[&opened.session_id], session);
    drop(host);
    let mut restarted = fixture.host(Mode::Success, calls.clone(), requests);
    let reaped = restarted.reap(400_000).unwrap();
    assert!(reaped.closed_browser_ids.is_empty());
    assert!(reaped.expired_session_ids.is_empty());
    let continued = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({"action":"get_url", "activityAtMs":400_001}),
        )
        .unwrap()
        .unwrap();
    assert_eq!(continued["browserSession"]["browserId"], opened.browser_id);
    assert_eq!(continued["browserSession"]["sessionId"], opened.session_id);
    assert_eq!(calls.get(), 1);
    let closed = restarted.handle_command(&serde_json::json!({
        "action":"browser_session_close", "sessionId":opened.session_id,
        "leaveOpen":false, "activityAtMs":400_002
    }));
    assert_eq!(closed["success"], true);
    assert_eq!(closed["data"]["disposition"], "browser_closed");
    assert!(restarted.state().sessions.is_empty());
    assert!(restarted.state().browsers.is_empty());
}

#[test]
fn consumer_leave_open_launch_preference_survives_repeat_close_and_preserves_peers() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests);
    let opened = host.handle_command(&serde_json::json!({
        "action":"browser_session_open", "sessionName":"Alice",
        "profileId":"profile-a", "leaveOpen":true, "activityAtMs":100
    }));
    assert_eq!(opened["success"], true);
    let session_id = opened["data"]["sessionId"].as_str().unwrap().to_string();
    let browser_id = opened["data"]["browserId"].as_str().unwrap().to_string();
    let bob = host
        .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 101))
        .unwrap();
    assert_eq!(bob.browser_id, browser_id);
    for now in [102, 103] {
        let preserved = host.handle_command(&serde_json::json!({
            "action":"browser_session_close", "sessionId":session_id, "activityAtMs":now
        }));
        assert_eq!(preserved["success"], true);
        assert_eq!(preserved["data"]["disposition"], "browser_preserved");
        assert_eq!(preserved["data"]["taskRetained"], true);
        assert_eq!(host.state().sessions.len(), 2);
    }
    let closed = host.handle_command(&serde_json::json!({
        "action":"browser_session_close", "sessionId":session_id,
        "leaveOpen":false, "activityAtMs":104
    }));
    assert_eq!(closed["success"], true);
    assert_eq!(closed["data"]["taskRetained"], false);
    assert!(!host.state().leave_open_session_ids.contains(&session_id));
    assert!(!host.state().sessions.contains_key(&session_id));
    assert!(host.state().sessions.contains_key(&bob.session_id));
    assert!(host.state().browsers.contains_key(&browser_id));
    host.close_session(&bob.session_id, SessionEndReason::ExplicitClose, 105)
        .unwrap();
    assert!(host.state().leave_open_session_ids.is_empty());
    assert!(host.state().browsers.is_empty());
    assert_eq!(calls.get(), 1);
}

#[test]
fn consumer_leave_open_close_refreshes_peer_state_without_losing_retention() {
    let fixture = Fixture::new();
    let mut host = fixture.host(Mode::Success, Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let opened = host
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    competing_write(&fixture.root);
    let peer_sequence = fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .next_session_sequence;
    let response = host.handle_command(&serde_json::json!({
        "action":"browser_session_close", "sessionId":opened.session_id,
        "leaveOpen":true, "activityAtMs":101
    }));
    assert_eq!(response["success"], true, "{response}");
    assert!(host
        .state()
        .leave_open_session_ids
        .contains(&opened.session_id));
    assert!(host.state().browsers.contains_key(&opened.browser_id));
    let persisted = fixture.store(false).load_session_state().unwrap();
    assert_eq!(persisted.next_session_sequence, peer_sequence);
    assert!(persisted
        .leave_open_session_ids
        .contains(&opened.session_id));
}

fn seed_upgrade_admission_drain(root: &std::path::Path, malformed: bool) {
    let path = root.join(".agent-browser/runtime-adoption/admission-drain.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let body = if malformed {
        "invalid-json".to_string()
    } else {
        serde_json::json!({
            "schemaVersion": "agent-browser.runtime-adoption.v1",
            "transactionId": "upgrade-fixture", "candidateGenerationId": "candidate-fixture",
            "transactionRevision": 7, "recordedAt": "2026-10-11T06:30:00Z"
        })
        .to_string()
    };
    std::fs::write(path, body).unwrap();
}

#[test]
fn consumer_retired_upgrade_drain_allows_explicit_open_and_publication() {
    for malformed in [false, true] {
        let fixture = Fixture::new();
        let guard = crate::test_utils::EnvGuard::new(&["HOME"]);
        guard.set("HOME", fixture.root.to_str().unwrap());
        let calls = Rc::new(Cell::new(0));
        let requests = Rc::new(Cell::new(0));
        let mut host = fixture.host(Mode::Success, calls.clone(), requests);
        let path = fixture.root.join("new-profile");
        std::fs::create_dir(&path).unwrap();
        seed_upgrade_admission_drain(&fixture.root, malformed);
        let result = host.handle_command(&serde_json::json!({
            "action": "browser_session_navigate", "sessionName": "Alice",
            "profileId": "new-profile", "runtimeProfile": "new-profile", "profile": path,
            "url": "https://synthetic.example/", "activityAtMs": 100
        }));
        assert_eq!(result["success"], true, "{result}");
        assert_eq!(calls.get(), 1);
        assert!(fixture
            .store(false)
            .load_profile_catalog()
            .unwrap()
            .catalog
            .profiles
            .contains_key("new-profile"));
        assert!(fixture
            .store(false)
            .unpublished_launch_records()
            .unwrap()
            .is_empty());
    }
}

#[test]
fn consumer_retired_upgrade_drain_allows_implicit_open_and_owned_tab() {
    for malformed in [false, true] {
        for already_open in [false, true] {
            let fixture = Fixture::new();
            std::fs::write(fixture.root.join("automatic-pool"), "enabled").unwrap();
            let guard = crate::test_utils::EnvGuard::new(&["HOME"]);
            guard.set("HOME", fixture.root.to_str().unwrap());
            let calls = Rc::new(Cell::new(0));
            let requests = Rc::new(Cell::new(0));
            let mut host = fixture.host(Mode::Success, calls.clone(), requests);
            if already_open {
                host.open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
                    .unwrap();
            }
            seed_upgrade_admission_drain(&fixture.root, malformed);
            let result = host
                .execute_managed_command(
                    "Alice",
                    &serde_json::json!({
                        "action":"url", "profileId":"profile-a", "activityAtMs":101
                    }),
                )
                .unwrap()
                .unwrap();
            assert_eq!(result["success"], true, "{result}");
            assert_eq!(calls.get(), 1);
            assert_eq!(host.state().tabs.len(), 1);
            assert!(fixture
                .store(false)
                .unpublished_launch_records()
                .unwrap()
                .is_empty());
        }
    }
}

#[test]
fn consumer_retired_upgrade_drain_allows_live_navigation_and_physical_recovery() {
    for malformed in [false, true] {
        let fixture = Fixture::new();
        std::fs::write(fixture.root.join("automatic-pool"), "enabled").unwrap();
        let guard = crate::test_utils::EnvGuard::new(&["HOME"]);
        guard.set("HOME", fixture.root.to_str().unwrap());
        let calls = Rc::new(Cell::new(0));
        let requests = Rc::new(Cell::new(0));
        let mut host = fixture.host(Mode::IdleRecovery, calls.clone(), requests);
        let command =
            serde_json::json!({"action":"url", "profileId":"profile-a", "activityAtMs":100});
        let first = host
            .execute_managed_command("Alice", &command)
            .unwrap()
            .unwrap();
        seed_upgrade_admission_drain(&fixture.root, malformed);
        let live = host
            .execute_managed_command("Alice", &command)
            .unwrap()
            .unwrap();
        assert_eq!(live["browserSession"], first["browserSession"]);
        let navigate = host
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "action":"navigate", "profileId":"profile-a", "url":"https://synthetic.example/"
                }),
            )
            .unwrap()
            .unwrap();
        assert_eq!(navigate["success"], true, "{navigate}");
        assert_eq!(calls.get(), 1);
        std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
        let recovered = host
            .execute_managed_command("Alice", &command)
            .unwrap()
            .unwrap();
        assert_eq!(recovered["success"], true, "{recovered}");
        assert_eq!(calls.get(), 2);
        assert_eq!(
            recovered["browserSession"]["sessionId"],
            first["browserSession"]["sessionId"]
        );
    }
}

#[path = "browser_operation_lookup_tests.rs"]
mod operation_lookup_tests;

#[path = "browser_build_selection_tests.rs"]
mod build_selection_tests;

#[test]
fn consumer_retention_claim_uses_owner_time_and_survives_sqlite_reopen() {
    let fixture = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let requests = Rc::new(Cell::new(0));
    let mut host = fixture.host(Mode::Success, calls.clone(), requests.clone());
    let opened = host
        .open(OpenBrowserSession::exact_profile(
            "Auth wait",
            "profile-a",
            100,
        ))
        .unwrap();
    let browser = host.state().browsers[&opened.browser_id].clone();
    let mut command = serde_json::json!({"action":"browser_session_retention","operation":"acquire","claimId":"auth:one","ownerId":"auth-receipts","sessionId":opened.session_id,"browserId":browser.id,"browserPid":browser.pid,"ttlMs":90000,"activityAtMs":1});
    let response = host.handle_command(&command);
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["released"], false);
    assert!(response["data"]["expiresAtMs"].as_u64().unwrap() > 90001);
    assert!(host.state().leave_open_session_ids.is_empty());
    let generation = response["data"]["generation"].clone();
    drop(host);
    let mut restarted = fixture.host(Mode::Success, calls, requests);
    command["operation"] = serde_json::json!("renew");
    command["generation"] = generation.clone();
    let renewed = restarted.handle_command(&command);
    assert_eq!(renewed["success"], true, "{renewed}");
    assert_eq!(renewed["data"]["generation"], generation);
    assert_eq!(renewed["data"]["released"], false);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(restarted.reap(now).unwrap().closed_browser_ids.is_empty());
    command["operation"] = serde_json::json!("release");
    command["generation"] = generation;
    let response = restarted.handle_command(&command);
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["released"], true);
}
