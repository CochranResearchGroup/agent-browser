use agent_browser_service_model::{
    decide_browser_recovery, record_browser_recovery_failure,
    record_browser_recovery_observed_live, record_browser_recovery_success,
    BrowserRecoveryAdmissionPolicy, BrowserRecoveryDecision, BrowserRecoveryDemand,
    BrowserRecoveryPhase, OldBrowserUsability,
};

fn policy() -> BrowserRecoveryAdmissionPolicy {
    BrowserRecoveryAdmissionPolicy {
        maximum_attempts: 3,
        base_backoff_ms: 10,
        maximum_backoff_ms: 15,
        deadline_ms: 100,
    }
}

#[test]
fn recovery_requires_exact_demand_and_proven_old_browser_failure() {
    assert_eq!(
        decide_browser_recovery(
            None,
            "browser",
            BrowserRecoveryDemand::Dormant,
            OldBrowserUsability::ProvenUnusable,
            1_000,
            policy(),
        )
        .unwrap(),
        BrowserRecoveryDecision::WaitForExactClientResume
    );
    assert_eq!(
        decide_browser_recovery(
            None,
            "browser",
            BrowserRecoveryDemand::ExactClientResume,
            OldBrowserUsability::Unknown,
            1_000,
            policy(),
        )
        .unwrap(),
        BrowserRecoveryDecision::AwaitOldBrowserProof
    );
}

#[test]
fn recovery_generation_fences_observation_failure_and_success() {
    let first = match decide_browser_recovery(
        None,
        "browser",
        BrowserRecoveryDemand::ExactClientResume,
        OldBrowserUsability::ProvenUnusable,
        1_000,
        policy(),
    )
    .unwrap()
    {
        BrowserRecoveryDecision::AdmitReplacement { state } => state,
        other => panic!("expected admission, got {other:?}"),
    };
    assert_eq!(first.generation, 1);
    assert!(record_browser_recovery_observed_live(&first, 2).is_err());

    let retry = record_browser_recovery_failure(&first, 1, 1_005, policy()).unwrap();
    assert_eq!(retry.phase, BrowserRecoveryPhase::RetryWait);
    assert_eq!(retry.next_eligible_at_ms, 1_015);
    assert_eq!(
        decide_browser_recovery(
            Some(&retry),
            "browser",
            BrowserRecoveryDemand::ExactClientResume,
            OldBrowserUsability::ProvenUnusable,
            1_014,
            policy(),
        )
        .unwrap(),
        BrowserRecoveryDecision::WaitForRetry { retry_at_ms: 1_015 }
    );

    let second = match decide_browser_recovery(
        Some(&retry),
        "browser",
        BrowserRecoveryDemand::ExactClientResume,
        OldBrowserUsability::ProvenUnusable,
        1_015,
        policy(),
    )
    .unwrap()
    {
        BrowserRecoveryDecision::AdmitReplacement { state } => state,
        other => panic!("expected retry admission, got {other:?}"),
    };
    assert_eq!(second.generation, 2);
    let observed = record_browser_recovery_observed_live(&second, 2).unwrap();
    assert!(record_browser_recovery_success(&observed, 1, 1_020).is_err());
    let recovered = record_browser_recovery_success(&observed, 2, 1_020).unwrap();
    assert_eq!(recovered.phase, BrowserRecoveryPhase::Recovered);
    assert_eq!(recovered.attempts, 0);
}
