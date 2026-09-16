use agent_browser_candidate::{
    validate_lock_trace, LivenessStatus, LivenessTracker, LockClass, LockEvent, LockEventKind,
    ProgressEvidence, RecoveryChoice,
};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

#[test]
fn renewal_requires_new_durable_progress_and_stalls_are_actionable() {
    let initial = ProgressEvidence {
        sequence: 1,
        phase: "candidate_build".to_string(),
        observed_at_unix_seconds: 100,
        process_evidence_sha256: digest('a'),
    };
    let mut tracker = LivenessTracker::new("operation-1", initial, 30).expect("tracker");
    assert!(matches!(
        tracker.status(120),
        LivenessStatus::Healthy {
            deadline_unix_seconds: 130,
            ..
        }
    ));

    let no_progress = tracker
        .record_progress(
            ProgressEvidence {
                sequence: 2,
                phase: "candidate_build".to_string(),
                observed_at_unix_seconds: 125,
                process_evidence_sha256: digest('a'),
            },
            30,
        )
        .expect_err("unchanged evidence must not renew custody");
    assert_eq!(no_progress.code(), "durable_progress_missing");

    tracker
        .record_progress(
            ProgressEvidence {
                sequence: 2,
                phase: "artifact_seal".to_string(),
                observed_at_unix_seconds: 125,
                process_evidence_sha256: digest('b'),
            },
            30,
        )
        .expect("real progress");
    assert!(matches!(
        tracker.status(154),
        LivenessStatus::Healthy {
            deadline_unix_seconds: 155,
            ..
        }
    ));

    let LivenessStatus::Stalled {
        last_phase,
        last_progress_at_unix_seconds,
        recovery_choices,
        ..
    } = tracker.status(156)
    else {
        panic!("operation should be stalled")
    };
    assert_eq!(last_phase, "artifact_seal");
    assert_eq!(last_progress_at_unix_seconds, 125);
    assert_eq!(
        recovery_choices,
        vec![
            RecoveryChoice::Wait,
            RecoveryChoice::Cancel,
            RecoveryChoice::Recover,
            RecoveryChoice::Supersede,
        ]
    );
}

#[test]
fn lock_order_and_hold_budget_are_enforced() {
    let valid = vec![
        LockEvent::acquire(0, LockClass::Coordination),
        LockEvent::acquire(2, LockClass::InstallTransaction),
        LockEvent::acquire(4, LockClass::RuntimeMutation),
        LockEvent::acquire(6, LockClass::ServiceState),
        LockEvent::release(8, LockClass::ServiceState),
        LockEvent::release(10, LockClass::RuntimeMutation),
        LockEvent::release(12, LockClass::InstallTransaction),
        LockEvent::release(14, LockClass::Coordination),
    ];
    let summary = validate_lock_trace(&valid, 20).expect("valid lock trace");
    assert_eq!(summary.maximum_hold_millis, 14);
    assert_eq!(summary.acquisition_count, 4);

    let wrong_order = vec![
        LockEvent::acquire(0, LockClass::RuntimeMutation),
        LockEvent::acquire(1, LockClass::InstallTransaction),
    ];
    let error = validate_lock_trace(&wrong_order, 20).expect_err("lock inversion");
    assert_eq!(error.code(), "lock_order_violation");

    let overlong = vec![
        LockEvent {
            monotonic_millis: 0,
            kind: LockEventKind::Acquire(LockClass::Coordination),
        },
        LockEvent {
            monotonic_millis: 21,
            kind: LockEventKind::Release(LockClass::Coordination),
        },
    ];
    let error = validate_lock_trace(&overlong, 20).expect_err("overlong lock");
    assert_eq!(error.code(), "physical_lock_hold_exceeded");
}
