use agent_browser_service_model::{
    freeze_task_retirement, retire_lost_task, BrowserSessionState, LostTaskRetirementObservation,
    LostTaskRetirementRequest, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
    RecordedProcessIdentity,
};

fn fixture() -> (BrowserSessionState, LostTaskRetirementObservation) {
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        "original-browser".into(),
        ManagedBrowserInstance {
            id: "original-browser".into(),
            profile_id: "profile".into(),
            pid: 4100,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: None,
            active_session_ids: vec!["original-task".into()],
        },
    );
    state.sessions.insert(
        "original-task".into(),
        ManagedBrowserSession {
            id: "original-task".into(),
            name: "synthetic-task".into(),
            profile_id: "profile".into(),
            browser_id: "original-browser".into(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
            expires_at_ms: 9999,
            current_tab_id: Some("original-tab".into()),
        },
    );
    state.tabs.insert(
        "original-tab".into(),
        ManagedBrowserTab {
            id: "original-tab".into(),
            target_id: "original-target".into(),
            browser_id: "original-browser".into(),
            session_id: "original-task".into(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
        },
    );
    let observed = LostTaskRetirementObservation {
        original_process: RecordedProcessIdentity {
            pid: 4100,
            start_token: "linux:00000000-0000-0000-0000-000000000001:17".into(),
            executable_path: Some("/synthetic/chromium".into()),
            browser_family: None,
        },
        current_boot_id: "00000000-0000-0000-0000-000000000002".into(),
        original_pid_absent: true,
        profile_unoccupied: true,
        control_epoch: 3,
    };
    (state, observed)
}

#[test]
fn lost_task_retirement_preserves_history_and_replays_the_receipt_after_task_removal() {
    let (mut state, observed) = fixture();
    let reference =
        freeze_task_retirement(&state, "original-task", &observed.original_process, 3).unwrap();
    let request = LostTaskRetirementRequest {
        task_id: "original-task".into(),
        operation_id: "auth-retire-synthetic".into(),
        expected_binding_ref: reference,
    };
    let receipt = retire_lost_task(&mut state, &request, &observed, 100).unwrap();
    assert_eq!(
        receipt
            .evidence_references
            .get("retirementResult")
            .map(String::as_str),
        Some("retired_lost")
    );
    assert_eq!(receipt.task_id, "original-task");
    assert_eq!(receipt.browser_id, "original-browser");
    assert!(!state.sessions.contains_key("original-task"));
    assert!(!state.browsers.contains_key("original-browser"));
    assert_eq!(state.session_history.len(), 1);
    assert_eq!(state.tab_history.len(), 1);
    let unchanged = state.clone();
    assert_eq!(
        retire_lost_task(&mut state, &request, &observed, 101).unwrap(),
        receipt
    );
    assert_eq!(state, unchanged);
}
