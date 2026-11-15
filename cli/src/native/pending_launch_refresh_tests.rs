//! An observed but unqualified launch must not poison a different profile.
use super::*;

#[test]
fn unpublished_launch_without_state_delta_does_not_block_other_profile() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut original = host_mode(&fixture, provider.clone(), calls.clone(), Mode::IdleRecovery);
    let opened = original.execute_managed_command("Alice", &serde_json::json!({
        "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
    let session_id = opened["browserSession"]["sessionId"].as_str().unwrap();
    let tab_id = opened["browserSession"]["tabId"].as_str().unwrap();
    let before = original.state().clone();
    let browser = before.browsers[browser_id].clone();
    provider.borrow_mut().occupied_desktops.insert(browser.desktop.as_ref().unwrap().desktop_id.clone());
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    let profile = fixture.store(false).load_profile_catalog().unwrap().catalog.profiles["profile-a"].clone();
    let mut effects = RemoteViewSessionEffects::new(
        Process { mode: Mode::IdleRecovery, root: fixture.root.clone(), calls: calls.clone() },
        adapter(&fixture, provider), fixture.store(false), Vec::new(),
        || uuid::Uuid::new_v4().to_string(),
    ).unwrap().with_pool(RemoteViewSessionPool { name:"main".into(), desired_desktops:1 }).unwrap();
    effects.begin_operation(&before).unwrap();
    let recovery = effects.recover_browser(&browser, &profile, &[before.tabs[tab_id].clone()], &[]).unwrap();
    let pending = fixture.store(false).unpublished_launch_records().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].observed_pid, Some(recovery.launch.pid));
    let launches = calls.get();
    // The native qualification boundary has not admitted the replacement into
    // logical state. Keep that original journal, rather than publishing it.
    let mut consumer = BrowserSessionHost::load(
        fixture.store(false), effects, &fixture.root.join("unused.json"),
        BrowserSessionHostConfig { session_idle_timeout_ms:300_000, remote_view_desktops:Vec::new(),
            default_disposable_policy:None, exact_url_history_maximum_bytes:1024 },
    ).unwrap();
    // An independent writer changes only bookkeeping. This is the branch that
    // previously committed the unqualified launch before Bob could proceed.
    let mut peer = before.clone();
    peer.next_session_sequence += 1;
    fixture.store(false).compare_and_save_session_state(&before, &peer).unwrap();
    let reply = consumer.execute_managed_command("Bob", &serde_json::json!({
        "action":"get_url", "runtimeProfile":"profile-b", "activityAtMs":101
    })).expect("an unqualified Alice launch must not veto Bob").unwrap();
    assert_eq!(reply["success"], true, "{reply}");
    let bob = reply["browserSession"]["sessionId"].as_str().unwrap();
    let persisted = fixture.store(false).load_session_state().unwrap();
    assert_eq!(persisted.sessions[bob].profile_id, "profile-b");
    assert_eq!(persisted.sessions[session_id], before.sessions[session_id]);
    assert_eq!(persisted.browsers[browser_id], before.browsers[browser_id]);
    assert_eq!(calls.get(), launches + 1, "only Bob may launch; never replay Alice");
    assert_eq!(fixture.store(false).unpublished_launch_records().unwrap(), pending,
        "Alice remains unqualified with the exact original journal");
}
