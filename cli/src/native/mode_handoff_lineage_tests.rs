use super::*;
use crate::native::browser_host_control;
use agent_browser_service_model::{
    BrowserLaunch, BrowserLaunchCustodyRecord, BrowserLaunchIntent, BrowserSessionState,
};

struct Fixture {
    root: std::path::PathBuf,
    state: BrowserSessionState,
    browser: String,
    handoff: String,
    previous_digest: String,
    operation: Operation,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(path) = browser_host_control::path(&self.browser) {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("mode-handoff-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let legacy = root.join("state.json");
        std::fs::write(&legacy, r#"{"profiles":{}}"#).unwrap();
        let mut store = BrowserSessionSqliteStore::open_or_migrate(&root, &legacy)
            .unwrap()
            .0;
        let mut before = BrowserSessionState::default();
        let browser = format!("browser-{}", uuid::Uuid::new_v4());
        let handoff = "retained-human".to_string();
        let f: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let assignment: agent_browser_service_model::RemoteViewAssignmentRecord =
            serde_json::from_value(f["assignment"].clone()).unwrap();
        before.browsers.insert(browser.clone(),serde_json::from_value(json!({"id":browser,"profileId":"profile","pid":u32::MAX,"cdpEndpoint":"ws://original","activeSessionIds":["task"],"desktop":{"desktop_id":assignment.desktop_id,"generation":assignment.generation,"friendly_route_label":"Synthetic"}})).unwrap());
        before.sessions.insert("task".into(),serde_json::from_value(json!({"id":"task","name":"mode-owner","profileId":"profile","browserId":browser,"createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":u64::MAX,"currentTabId":"tab"})).unwrap());
        before.tabs.insert("tab".into(),serde_json::from_value(json!({"id":"tab","targetId":"old-target","browserId":browser,"sessionId":"task","createdAtMs":1,"lastActivityAtMs":1})).unwrap());
        agent_browser_service_model::retain_remote_view_tab_handoff(
            &mut before,
            &handoff,
            "task",
            "tab",
        )
        .unwrap();
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&before, &handoff)
                .unwrap();
        let previous_digest =
            agent_browser_service_model::remote_view_native_target_digest(&handoff, &target)
                .unwrap();
        let original = RecordedProcessIdentity {
            pid: u32::MAX,
            start_token: "linux:old:1".into(),
            executable_path: Some("/fixture/chrome".into()),
            browser_family: Some("chrome".into()),
        };
        let binding = Binding {
            profile: BrowserProfileCatalogEntry {
                id: "profile".into(),
                name: "Synthetic".into(),
                user_data_dir: root.to_string_lossy().into_owned(),
                kind: agent_browser_service_model::BrowserProfileKind::Named,
                manual_login_url: None,
            },
            browser: target.browser,
            session: target.session,
            tab: target.tab,
            original: original.clone(),
            original_mode: Mode::Cdp,
            selected_build: None,
            previous_build: None,
            profile_data_version: None,
            handoff_targets: Default::default(),
        };
        let identity =
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .unwrap();
        let mut state = before;
        let current = state.browsers.get_mut(&browser).unwrap();
        current.pid = identity.pid;
        current.cdp_endpoint = String::new();
        state.tabs.get_mut("tab").unwrap().target_id = "new-native-target".into();
        store.save_session_state(&state).unwrap();
        let intent = BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: "profile".into(),
            assignment,
        };
        let mut record = BrowserLaunchCustodyRecord::pending(intent).unwrap();
        record
            .observe(&BrowserLaunch {
                browser_id: browser.clone(),
                pid: identity.pid,
                cdp_endpoint: String::new(),
                desktop: state.browsers[&browser].desktop.clone(),
                process_identity: Some(identity.clone()),
            })
            .unwrap();
        record.confirm_publication(&state).unwrap();
        // Qualified process observations use the runtime's custody schema.
        store
            .remote_view_mutation_connection()
            .pragma_update(None, "user_version", 2)
            .unwrap();
        let ledger = json!({"records":{record.intent.intent_id.clone():record}});
        store.remote_view_mutation_connection().execute("INSERT INTO state_documents(kind,schema_version,json)VALUES('browser_launch_custody','agent-browser.browser-launch-custody.v1',?1)",[ledger.to_string()]).unwrap();
        assert_eq!(
            crate::native::browser_launch_custody_store::published_process_identity(
                store.remote_view_mutation_connection(),
                &state.browsers[&browser],
            )
            .unwrap(),
            Some(identity.clone()),
            "fixture must reach actual published process custody"
        );
        let operation = Operation {
            request: Request {
                operation_id: "mode-operation".into(),
                browser_id: browser.clone(),
                profile_id: "profile".into(),
                original_process: serde_json::to_string(&original).unwrap(),
                executable: "/fixture/chrome".into(),
                executable_sha256: None,
                task_ids: vec!["task".into()],
                requested_mode: Mode::CdpFree,
            },
            phase: Phase::Completed,
            replacement: Some(Replacement {
                process_identity: serde_json::to_string(&identity).unwrap(),
                mode: Mode::CdpFree,
                debugging: false,
                native_observation: true,
                native_input: false,
            }),
            close_method: None,
            effective_mode: None,
            effective_executable: None,
            effective_executable_sha256: None,
            failure: None,
            launch_attempts: 1,
            restoring_previous: false,
        };
        let connection = rusqlite::Connection::open(root.join("runtime.mode.sqlite3")).unwrap();
        connection.execute_batch("CREATE TABLE browser_mode_owner_bindings(id TEXT PRIMARY KEY,binding TEXT NOT NULL);CREATE TABLE browser_mode_operations(id TEXT PRIMARY KEY,state TEXT NOT NULL)").unwrap();
        connection
            .execute(
                "INSERT INTO browser_mode_owner_bindings VALUES(?1,?2)",
                ["mode-operation", &serde_json::to_string(&binding).unwrap()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO browser_mode_operations VALUES(?1,?2)",
                [
                    "mode-operation",
                    &serde_json::to_string(&operation).unwrap(),
                ],
            )
            .unwrap();
        let control_path = browser_host_control::path(&browser).unwrap();
        std::fs::create_dir_all(control_path.parent().unwrap()).unwrap();
        browser_host_control::begin_handoff(&browser, &handoff, &previous_digest).unwrap();
        Self {
            root,
            state,
            browser,
            handoff,
            previous_digest,
            operation,
        }
    }
    fn database(&self) -> std::path::PathBuf {
        self.root.join("runtime.sqlite3")
    }
    fn write_operation(&self, operation: &Operation) {
        rusqlite::Connection::open(self.root.join("runtime.mode.sqlite3"))
            .unwrap()
            .execute(
                "UPDATE browser_mode_operations SET state=?1",
                [serde_json::to_string(operation).unwrap()],
            )
            .unwrap();
    }
}
#[test]
fn completed_mode_handoff_rebinds_exact_invitation_and_returns_on_same_link() {
    let f = Fixture::new();
    let before = browser_host_control::read(&f.browser).unwrap();
    browser_host_control::reconcile_recovered_handoff(&f.database(), &f.state, &f.handoff, 2)
        .unwrap();
    let target =
        agent_browser_service_model::resolve_remote_view_tab_handoff(&f.state, &f.handoff).unwrap();
    let digest =
        agent_browser_service_model::remote_view_native_target_digest(&f.handoff, &target).unwrap();
    assert_ne!(digest, f.previous_digest);
    assert_eq!(
        browser_host_control::handoff_intent(&f.browser, &f.handoff, &digest).unwrap(),
        "interactive"
    );
    let current = browser_host_control::read(&f.browser).unwrap();
    assert_eq!(current.epoch, before.epoch);
    assert!(current.human && current.pause_reads);
    browser_host_control::return_handoff(&f.browser, &f.handoff, &digest, "original-native-return")
        .unwrap();
    browser_host_control::return_handoff(&f.browser, &f.handoff, &digest, "original-native-return")
        .unwrap();
    assert!(!browser_host_control::read(&f.browser).unwrap().human);
}
#[test]
fn completed_mode_handoff_refuses_unsettled_or_different_mode_lineage() {
    for case in [
        "phase",
        "task",
        "profile",
        "browser",
        "replacement",
        "original",
        "posture",
    ] {
        let f = Fixture::new();
        let mut operation = f.operation.clone();
        match case {
            "phase" => operation.phase = Phase::ReplacementPending,
            "task" => operation.request.task_ids = vec!["other".into()],
            "profile" => operation.request.profile_id = "other".into(),
            "browser" => operation.request.browser_id = "other".into(),
            "replacement" => {
                operation.replacement.as_mut().unwrap().process_identity =
                    operation.request.original_process.clone()
            }
            "original" => operation.request.original_process = "{}".into(),
            "posture" => operation.replacement.as_mut().unwrap().debugging = true,
            _ => unreachable!(),
        };
        f.write_operation(&operation);
        let before = std::fs::read(browser_host_control::path(&f.browser).unwrap()).unwrap();
        assert!(
            browser_host_control::reconcile_recovered_handoff(
                &f.database(),
                &f.state,
                &f.handoff,
                2
            )
            .is_err(),
            "{case}"
        );
        assert_eq!(
            std::fs::read(browser_host_control::path(&f.browser).unwrap()).unwrap(),
            before,
            "{case}"
        );
    }
}

#[test]
fn completed_mode_handoff_keeps_a_returned_invitation_passive() {
    let f = Fixture::new();
    browser_host_control::return_handoff(
        &f.browser,
        &f.handoff,
        &f.previous_digest,
        "before-mode-return",
    )
    .unwrap();
    let before = browser_host_control::read(&f.browser).unwrap();
    browser_host_control::reconcile_recovered_handoff(&f.database(), &f.state, &f.handoff, 2)
        .unwrap();
    let target =
        agent_browser_service_model::resolve_remote_view_tab_handoff(&f.state, &f.handoff).unwrap();
    let digest =
        agent_browser_service_model::remote_view_native_target_digest(&f.handoff, &target).unwrap();
    assert_eq!(
        browser_host_control::handoff_intent(&f.browser, &f.handoff, &digest).unwrap(),
        "view"
    );
    let current = browser_host_control::read(&f.browser).unwrap();
    assert_eq!(current.epoch, before.epoch);
    assert!(!current.human && !current.pause_reads);
    browser_host_control::return_handoff(&f.browser, &f.handoff, &digest, "later-native-return")
        .unwrap();
    assert_eq!(
        browser_host_control::read(&f.browser).unwrap().epoch,
        before.epoch
    );
}

#[test]
fn pending_mode_handoff_exposes_only_exact_missing_original_without_mutation() {
    for case in [
        "exact",
        "other_owner",
        "missing_owner",
        "shared",
        "task",
        "phase",
        "launched",
        "replacement",
        "forced",
        "changed",
        "unfrozen",
        "alive",
        "ambiguous",
    ] {
        let mut f = Fixture::new();
        let modes = rusqlite::Connection::open(f.root.join("runtime.mode.sqlite3")).unwrap();
        let raw: String = modes
            .query_row("SELECT binding FROM browser_mode_owner_bindings", [], |r| {
                r.get(0)
            })
            .unwrap();
        let mut binding: Binding = serde_json::from_str(&raw).unwrap();
        f.state
            .browsers
            .insert(f.browser.clone(), binding.browser.clone());
        f.state
            .tabs
            .insert(binding.tab.id.clone(), binding.tab.clone());
        f.state
            .profile_operation_ids
            .insert("profile".into(), "mode-operation".into());
        if case == "alive" {
            let identity =
                crate::process_identity::capture_process_identity(std::process::id(), None, None)
                    .unwrap();
            binding.original = identity.clone();
            binding.browser.pid = identity.pid;
            f.state
                .browsers
                .insert(f.browser.clone(), binding.browser.clone());
        }
        binding.handoff_targets.insert(
            f.handoff.clone(),
            FrozenHandoffTarget {
                browser: binding.browser.clone(),
                session: binding.session.clone(),
                tab: binding.tab.clone(),
            },
        );
        let mut operation = f.operation.clone();
        operation.phase = Phase::ClosePrompt;
        operation.launch_attempts = 0;
        operation.replacement = None;
        operation.close_method = Some(crate::native::browser_mode_operation::CloseMethod::Graceful);
        operation.request.original_process = serde_json::to_string(&binding.original).unwrap();
        match case {
            "other_owner" => {
                f.state
                    .profile_operation_ids
                    .insert("profile".into(), "other".into());
            }
            "missing_owner" => {
                f.state.profile_operation_ids.remove("profile");
            }
            "shared" => {
                f.state.shared_browser_ids.insert(f.browser.clone());
            }
            "task" => operation.request.task_ids.push("other".into()),
            "phase" => operation.phase = Phase::Prepared,
            "launched" => operation.launch_attempts = 1,
            "replacement" => operation.replacement = f.operation.replacement.clone(),
            "forced" => {
                operation.close_method =
                    Some(crate::native::browser_mode_operation::CloseMethod::Forced)
            }
            "changed" => f.state.tabs.get_mut("tab").unwrap().target_id = "other".into(),
            "unfrozen" => binding.handoff_targets.clear(),
            _ => {}
        }
        modes
            .execute(
                "UPDATE browser_mode_owner_bindings SET binding=?1",
                [serde_json::to_string(&binding).unwrap()],
            )
            .unwrap();
        f.write_operation(&operation);
        if case == "ambiguous" {
            operation.request.operation_id = "second".into();
            modes
                .execute(
                    "INSERT INTO browser_mode_operations VALUES('second',?1)",
                    [serde_json::to_string(&operation).unwrap()],
                )
                .unwrap();
        }
        drop(modes);
        let before = std::fs::read(f.root.join("runtime.mode.sqlite3")).unwrap();
        let control = std::fs::read(browser_host_control::path(&f.browser).unwrap()).unwrap();
        let state = f.state.clone();
        let connection = rusqlite::Connection::open(f.database()).unwrap();
        let proof = pending_mode_handoff_proof(&connection, &f.state, &f.handoff).unwrap();
        assert_eq!(
            proof,
            if case == "exact" {
                Some("mode-operation".into())
            } else {
                None
            },
            "{case}"
        );
        assert_eq!(f.state, state, "{case}");
        assert_eq!(
            std::fs::read(f.root.join("runtime.mode.sqlite3")).unwrap(),
            before,
            "{case}"
        );
        assert_eq!(
            std::fs::read(browser_host_control::path(&f.browser).unwrap()).unwrap(),
            control,
            "{case}"
        );
    }
}
