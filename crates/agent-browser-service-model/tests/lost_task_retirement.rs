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

#[test]
fn invalid_current_boot_is_not_lost_runtime_proof() {
    let (mut state, mut observed) = fixture();
    let reference =
        freeze_task_retirement(&state, "original-task", &observed.original_process, 3).unwrap();
    let request = LostTaskRetirementRequest {
        task_id: "original-task".into(),
        operation_id: "auth-retire-invalid-boot".into(),
        expected_binding_ref: reference,
    };
    observed.current_boot_id = "unqualified-kernel-observation".into();
    let original = state.clone();
    assert!(retire_lost_task(&mut state, &request, &observed, 100).is_err());
    assert_eq!(state, original);
}

#[test]
fn lost_task_retirement_refuses_changed_or_unqualified_ownership_without_mutation() {
    use agent_browser_service_model::{BrowserTaskOperation, BrowserTaskOperationState};
    for case in [
        "same-boot",
        "pid-reused",
        "profile-busy",
        "wrong-ref",
        "epoch-changed",
        "new-process",
        "new-target",
        "peer-browser",
        "profile-operation",
        "unknown-input",
    ] {
        let (mut state, mut observed) = fixture();
        let reference =
            freeze_task_retirement(&state, "original-task", &observed.original_process, 3).unwrap();
        let mut request = LostTaskRetirementRequest {
            task_id: "original-task".into(),
            operation_id: "auth-retire-negative".into(),
            expected_binding_ref: reference,
        };
        match case {
            "same-boot" => observed.current_boot_id = "00000000-0000-0000-0000-000000000001".into(),
            "pid-reused" => observed.original_pid_absent = false,
            "profile-busy" => observed.profile_unoccupied = false,
            "wrong-ref" => request.expected_binding_ref = "0".repeat(64),
            "epoch-changed" => observed.control_epoch = 4,
            "new-process" => {
                observed.original_process.start_token =
                    "linux:00000000-0000-0000-0000-000000000001:18".into()
            }
            "new-target" => {
                state.tabs.get_mut("original-tab").unwrap().target_id = "replacement-target".into()
            }
            "peer-browser" => {
                let mut peer = state.browsers["original-browser"].clone();
                peer.id = "peer".into();
                state.browsers.insert("peer".into(), peer);
            }
            "profile-operation" => {
                state
                    .profile_operation_ids
                    .insert("profile".into(), "in-flight".into());
            }
            "unknown-input" => {
                state
                    .task_operations
                    .entry("original-task".into())
                    .or_default()
                    .insert(
                        "input".into(),
                        BrowserTaskOperation {
                            request_id: "input".into(),
                            task_id: "original-task".into(),
                            profile_id: "profile".into(),
                            browser_id: "original-browser".into(),
                            tab_id: "original-tab".into(),
                            target_id: "original-target".into(),
                            process_pid: 4100,
                            process_identity: Some(observed.original_process.clone()),
                            control_epoch: 3,
                            action: "fill".into(),
                            fingerprint: "synthetic-original-input".into(),
                            state: BrowserTaskOperationState::Uncertain,
                            created_at_ms: 2,
                            completed_at_ms: None,
                            evidence_references: Default::default(),
                        },
                    );
            }
            _ => unreachable!(),
        }
        let before = state.clone();
        assert!(
            retire_lost_task(&mut state, &request, &observed, 100).is_err(),
            "{case}"
        );
        assert_eq!(state, before, "{case} must retain custody");
    }
}

#[test]
fn durable_retirement_replay_cannot_mutate_a_new_generation_or_accept_an_operation_collision() {
    let (mut state, observed) = fixture();
    let request = LostTaskRetirementRequest {
        task_id: "original-task".into(),
        operation_id: "auth-retire-once".into(),
        expected_binding_ref: freeze_task_retirement(
            &state,
            "original-task",
            &observed.original_process,
            3,
        )
        .unwrap(),
    };
    let receipt = retire_lost_task(&mut state, &request, &observed, 100).unwrap();
    let serialized = serde_json::to_vec(&state).unwrap();
    let mut restarted: BrowserSessionState = serde_json::from_slice(&serialized).unwrap();
    let (replacement, _) = fixture();
    restarted.sessions = replacement.sessions;
    restarted.browsers = replacement.browsers;
    restarted.tabs = replacement.tabs;
    restarted.browsers.get_mut("original-browser").unwrap().pid = 4200;
    let before = restarted.clone();
    assert_eq!(
        retire_lost_task(&mut restarted, &request, &observed, 101).unwrap(),
        receipt
    );
    assert_eq!(restarted, before, "old receipt must not act on replacement");
    let conflicting = LostTaskRetirementRequest {
        expected_binding_ref: "0".repeat(64),
        ..request
    };
    assert!(retire_lost_task(&mut restarted, &conflicting, &observed, 102).is_err());
    assert_eq!(restarted, before);
}
