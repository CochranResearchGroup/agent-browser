use super::*;

#[cfg(target_os = "linux")]
#[test]
fn retained_human_handoff_recovers_after_ordinary_browser_exit() {
    ordinary_handoff_fixture(false);
}

#[cfg(target_os = "linux")]
#[test]
fn retained_human_handoff_reconstructs_previously_unmapped_replacement() {
    ordinary_handoff_fixture(true);
}

#[cfg(target_os = "linux")]
fn ordinary_handoff_fixture(reconstruct_legacy: bool) {
    let sandbox = Fixture::new();
    let service = sandbox.root.join(".agent-browser/service");
    std::fs::create_dir_all(&service).unwrap();
    let fixture = Fixture::with_root(service, true);
    std::fs::create_dir_all(fixture.root.join("profile-a")).unwrap();
    let env = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    env.set("HOME", sandbox.root.to_str().unwrap());
    env.set("AGENT_BROWSER_HOME", sandbox.root.to_str().unwrap());
    env.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    let _children = CopyProcessFixture::new(&fixture.root);
    // The retained production browser is CDP-backed. The common child fixture
    // defaults to CDP-free manual mode, which has a separate recovery contract.
    std::fs::write(
        fixture.root.join("copy-process-cdp-endpoint"),
        "http://127.0.0.1:9222",
    )
    .unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut original = host(&fixture, provider.clone(), calls.clone());
    let opened = original.handle_command(&serde_json::json!({
        "action":"browser_session_open", "sessionName":"Alice", "profileId":"profile-a",
        "url":"https://synthetic.example/retained-tab", "activityAtMs":100,
        "serviceName":"fixture", "agentName":"fixture", "taskName":"Recover ordinary browser exit"
    }));
    assert_eq!(opened["success"], true, "{opened}");
    let task = opened["data"]["sessionId"].as_str().unwrap().to_owned();
    original.tab_for_navigation(&task, 101).unwrap();
    original
        .record_navigation(&task, "https://synthetic.example/old-tab", 100)
        .unwrap();
    original
        .record_navigation(&task, "https://synthetic.example/last-tab", 101)
        .unwrap();
    let mut before = original.state().clone();
    let browser_id = before.sessions[&task].browser_id.clone();
    let pid = before.browsers[&browser_id].pid;
    let tab = before.sessions[&task].current_tab_id.clone().unwrap();
    let link =
        retain_remote_view_tab_handoff_at(&mut before, "ordinary-exit-handoff", &task, &tab, 101)
            .unwrap();
    drop(original);
    fixture.store(false).save_session_state(&before).unwrap();
    let target = resolve_remote_view_tab_handoff(&before, &link.id).unwrap();
    let digest = remote_view_native_target_digest(&link.id, &target).unwrap();
    crate::native::browser_host_control::begin_handoff(&browser_id, &link.id, &digest).unwrap();
    let control = crate::native::browser_host_control::read(&browser_id).unwrap();
    let control_path = crate::native::browser_host_control::path(&browser_id).unwrap();
    let original_control_bytes = std::fs::read(&control_path).unwrap();
    COPY_PROCESS_CHILDREN.with(|children| {
        let mut children = children.borrow_mut();
        let child = children.get_mut(&(fixture.root.clone(), pid)).unwrap();
        child.stdin.take();
        child.wait().unwrap();
    });
    assert!(matches!(
        crate::process_identity::observe_process(pid),
        crate::process_identity::ProcessObservation::Missing
    ));
    assert!(
        fixture_retirement_records(&fixture.root).is_empty(),
        "ordinary process exit must not require a fictional idle retirement"
    );
    std::fs::write(
        fixture.root.join("idle-process-absent"),
        "verified kernel absence",
    )
    .unwrap();
    let mut recovered = host_mode(&fixture, provider, calls.clone(), Mode::IdleRecovery);
    let result = recovered.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":link.id,
        "nativeDesktop":true, "activityAtMs":102
    }));
    assert_eq!(result["success"], true,
        "a retained handoff must recover its saved profile/tab after ordinary browser exit: {result}");
    assert_eq!(result["data"]["operatorVisible"]["state"], "ready");
    assert_eq!(result["data"]["browserId"], browser_id);
    assert_eq!(result["data"]["tabId"], tab);
    assert_eq!(recovered.state().sessions[&task].profile_id, "profile-a");
    let after = crate::native::browser_host_control::read(&browser_id).unwrap();
    assert!(after.human && after.pause_reads);
    assert_eq!(after.epoch, control.epoch);
    assert_eq!(calls.get(), 2, "one original and one replacement launch");
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("copy-process-last-navigation")).unwrap(),
        "https://synthetic.example/last-tab",
        "restore the last known tab URL"
    );
    let database = rusqlite::Connection::open(fixture.root.join("runtime.sqlite3")).unwrap();
    let proof = crate::native::browser_launch_custody_store::retained_handoff_recovery_proof;
    assert_eq!(
        proof(&database, recovered.state(), &link.id, &digest).unwrap()["kind"],
        "ordinaryProcessExit"
    );
    let mut changed = recovered.state().clone();
    changed.tabs.get_mut(&tab).unwrap().target_id = "unrelated-target".into();
    assert_eq!(
        proof(&database, &changed, &link.id, &digest).unwrap_err(),
        "browser_handoff_recovery_mapping_unrecorded"
    );
    assert_eq!(
        database
            .execute(
                "DELETE FROM state_documents WHERE kind LIKE 'ordinary_handoff_recovery_binding:%'",
                []
            )
            .unwrap(),
        1
    );
    assert_eq!(
        proof(&database, recovered.state(), &link.id, &digest).unwrap_err(),
        "browser_handoff_recovery_mapping_unrecorded",
        "never invent a missing historical mapping"
    );
    if reconstruct_legacy {
        // Synthetic old-runtime history: replacement publication exists, its
        // mapping is absent, and the retained human invitation still names the
        // original physical target. Never perform this fixture setup live.
        std::fs::write(control_path, original_control_bytes).unwrap();
        let result = recovered.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":link.id,
            "nativeDesktop":true, "activityAtMs":103
        }));
        assert_eq!(result["success"], true,
            "the same link must reconstruct from saved profile/URL when an old runtime omitted its mapping: {result}");
        assert_eq!(result["data"]["operatorVisible"]["state"], "ready");
        assert_ne!(
            result["data"]["tabId"], tab,
            "new reconstruction has a truthful new target, not a fabricated historical mapping"
        );
        assert_eq!(result["data"]["browserId"], browser_id);
        assert_eq!(recovered.state().sessions[&task].profile_id, "profile-a");
        let current = crate::native::browser_host_control::read(&browser_id).unwrap();
        assert!(current.human && current.pause_reads);
        assert!(current.epoch > control.epoch);
        assert_eq!(
            calls.get(),
            2,
            "reconstruction reuses the verified replacement browser"
        );
    }
}
