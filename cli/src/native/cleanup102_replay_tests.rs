// Included at the end of browser_session_pool_tests.rs. These are diagnostic
// fixtures, not a claim that the captured caller's in-memory snapshot is known.
// Raw input is opt-in, private, hash-pinned, and never embedded in source.

fn cleanup102_environment(root: &std::path::Path) -> crate::test_utils::EnvGuard<'static> {
    let environment = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_CONFIG",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
    ]);
    environment.set("HOME", root.to_str().unwrap());
    environment.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    environment.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    environment.remove("AGENT_BROWSER_CONFIG");
    for (name, directory) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        environment.set(name, root.join(directory).to_str().unwrap());
    }
    std::fs::create_dir_all(root.join(".agent-browser/service")).unwrap();
    environment
}

fn cleanup102_assert_scripted_effects(fixture: &Fixture, provider: &Provider) {
    assert!(
        provider.native_bridge.is_none(),
        "cleanup102_setup_network_bridge_forbidden"
    );
    for marker in [
        "copy-process-fixture",
        "real-native-context-adoption",
        "command-retirement-race.json",
    ] {
        assert!(
            !fixture.root.join(marker).exists(),
            "cleanup102_setup_real_effect_marker_forbidden"
        );
    }
}

#[test]
fn cleanup102_fresh_public_manual_close_control() {
    let fixture = Fixture::new();
    let _environment = cleanup102_environment(&fixture.root);
    let provider = Rc::new(RefCell::new(Provider::default()));
    cleanup102_assert_scripted_effects(&fixture, &provider.borrow());
    let launches = Rc::new(Cell::new(0));
    std::fs::write(fixture.root.join("count-retirement-closes"), "count").unwrap();
    let mut consumer = host(&fixture, provider.clone(), launches.clone());
    let opened = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_open", "sessionName":"cleanup102-fresh",
        "profileId":"profile-a", "manualLogin":true,
        "url":"https://synthetic.example/login", "activityAtMs":100,
        "serviceName":"fixture", "agentName":"fixture", "taskName":"Close control"
    }));
    assert_eq!(
        opened["success"], true,
        "cleanup102_setup_fresh_open_failed"
    );
    let task = opened["data"]["sessionId"].as_str().unwrap();
    let browser = opened["data"]["browserId"].as_str().unwrap();
    let closed = consumer.handle_command(&serde_json::json!({
        "id":"cleanup102-fresh-close", "action":"browser_session_close",
        "sessionId":task, "leaveOpen":false, "scope":"task", "activityAtMs":102
    }));
    assert_eq!(
        closed["success"], true,
        "cleanup102_fresh_public_close_failed"
    );
    assert_eq!(closed["data"]["closed"], true);
    assert_eq!(closed["data"]["disposition"], "browser_closed");
    let stored = fixture.store(false).load_session_state().unwrap();
    assert!(!stored.sessions.contains_key(task));
    assert!(!stored.browsers.contains_key(browser));
    assert_eq!(launches.get(), 1, "only one scripted launch");
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
        "1"
    );
    cleanup102_assert_scripted_effects(&fixture, &provider.borrow());
}

// This conservative audit does not rebase private paths, fabricate owner
// inventory, choose an activity ID, or replace the captured expected snapshot.
// A refusal is a setup/attribution gap, never the target publication-conflict RED.
fn cleanup102_snapshot_preflight(
    fixture: &Fixture,
    task: &str,
) -> Result<RemoteViewAssignmentRecord, &'static str> {
    let mut store = fixture.store(false);
    let state = store
        .load_session_state()
        .map_err(|_| "cleanup102_setup_state_unreadable")?;
    let catalog = store
        .load_profile_catalog()
        .map_err(|_| "cleanup102_setup_catalog_unreadable")?
        .catalog;
    if !state.host_bindings.is_empty()
        || !state.app_browser_attachments.is_empty()
        || !state.shared_browser_ids.is_empty()
    {
        return Err("cleanup102_setup_external_owner_context_unqualified");
    }
    for profile in catalog.profiles.values() {
        if !std::path::Path::new(&profile.user_data_dir).starts_with(&fixture.root) {
            return Err("cleanup102_setup_external_profile_paths_unqualified");
        }
    }
    for allocation in state.disposable_profiles.values() {
        if !std::path::Path::new(&allocation.profile.user_data_dir).starts_with(&fixture.root)
            || !std::path::Path::new(&allocation.user_data_root).starts_with(&fixture.root)
        {
            return Err("cleanup102_setup_external_disposable_paths_unqualified");
        }
    }
    if !state.profile_executables.is_empty() {
        return Err("cleanup102_setup_captured_executable_mapping_unqualified");
    }
    let session = state
        .sessions
        .get(task)
        .ok_or("cleanup102_setup_target_task_missing")?;
    if !state.browsers.contains_key(&session.browser_id) {
        return Err("cleanup102_setup_target_browser_missing");
    }
    let scripted: RemoteViewAssignmentRecord = serde_json::from_value(wire()["assignment"].clone())
        .map_err(|_| "cleanup102_setup_scripted_owner_invalid")?;
    for browser in state.browsers.values() {
        let published = store
            .published_launch_assignment(browser)
            .map_err(|_| "cleanup102_setup_captured_owner_unqualified")?;
        if published != scripted {
            return Err("cleanup102_setup_scripted_owner_inventory_mismatch");
        }
    }
    Ok(scripted)
}

#[test]
#[ignore = "opt-in private snapshot; preflight refusal is not an exact-close reproduction"]
fn cleanup102_private_snapshot_public_close_replay() {
    use sha2::Digest;
    let input =
        std::env::var("AGENT_BROWSER_CLEANUP102_SNAPSHOT").expect("private snapshot path required");
    let digest = std::env::var("AGENT_BROWSER_CLEANUP102_SNAPSHOT_SHA256")
        .expect("snapshot SHA256 required");
    let task =
        std::env::var("AGENT_BROWSER_CLEANUP102_TASK_ID").expect("explicit captured task required");
    let at: u64 = std::env::var("AGENT_BROWSER_CLEANUP102_AT_MS")
        .expect("explicit replay clock required")
        .parse()
        .expect("replay clock must be an unsigned millisecond timestamp");
    let input = std::path::PathBuf::from(input);
    assert!(
        input.is_absolute(),
        "cleanup102_setup_absolute_snapshot_required"
    );
    assert!(
        std::fs::symlink_metadata(&input)
            .unwrap()
            .file_type()
            .is_file(),
        "cleanup102_setup_regular_snapshot_required"
    );
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = input.as_os_str().to_os_string();
        sidecar.push(suffix);
        assert!(
            !std::path::PathBuf::from(sidecar).exists(),
            "cleanup102_setup_standalone_snapshot_required"
        );
    }
    let bytes = std::fs::read(&input).unwrap();
    assert!(
        bytes.len() <= 256 * 1024 * 1024,
        "cleanup102_setup_snapshot_size_bound"
    );
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&bytes)),
        digest,
        "cleanup102_setup_snapshot_hash_mismatch"
    );
    let fixture = Fixture::new();
    let _environment = cleanup102_environment(&fixture.root);
    let copy = fixture.root.join("runtime.sqlite3");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Seal the owned directory and existing empty DB before private bytes
        // enter it; do not briefly create a world-readable captured snapshot.
        std::fs::set_permissions(&fixture.root, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    std::fs::write(&copy, bytes).unwrap();
    let assignment = cleanup102_snapshot_preflight(&fixture, &task)
        .expect("cleanup102_setup_refused_before_close; not an exact-close RED");
    let provider = Rc::new(RefCell::new(Provider {
        assignments: vec![assignment],
        ..Default::default()
    }));
    cleanup102_assert_scripted_effects(&fixture, &provider.borrow());
    let launches = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), launches.clone());
    let closed = consumer.handle_command(&serde_json::json!({
        "id":"cleanup102-captured-close", "action":"browser_session_close",
        "sessionId":task, "leaveOpen":false, "scope":"task", "activityAtMs":at
    }));
    cleanup102_assert_scripted_effects(&fixture, &provider.borrow());
    assert_eq!(
        launches.get(),
        0,
        "replay must not create a replacement browser"
    );
    if closed["error"].as_str() == Some("browser_session_idle_publication_conflict") {
        panic!("cleanup102_actual_public_close_RED: browser_session_idle_publication_conflict");
    }
    assert_eq!(
        closed["success"], true,
        "cleanup102_other_outcome; compare private trace before attribution"
    );
    assert_eq!(closed["data"]["closed"], true);
}
