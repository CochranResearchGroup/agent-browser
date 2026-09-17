use agent_browser_challenge_control::{
    admit_challenge_consumer, execute_provider_free_task, ChallengeConsumerAdmission,
    ChallengeConsumerAdmissionError, ChallengeConsumerAdmissionRequest, ChallengeConsumerEvidence,
    ChallengeConsumerKind, ChallengeTaskFixture, ChallengeTaskRequest, TURNSTILE_PROFILE,
};

fn evidence(fixture: ChallengeTaskFixture, intent: &str) -> ChallengeConsumerEvidence {
    let receipt = execute_provider_free_task(ChallengeTaskRequest {
        task_id: format!("challenge-task-{intent}"),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "a".repeat(64),
        downstream_intent_id: intent.to_string(),
        fixture,
        max_transitions: 8,
    })
    .unwrap();
    ChallengeConsumerEvidence::from(&receipt)
}

fn request(consumer: ChallengeConsumerKind, intent: &str) -> ChallengeConsumerAdmissionRequest {
    ChallengeConsumerAdmissionRequest {
        consumer,
        consumer_operation_id: format!("consumer-operation-{intent}"),
        expected_site_policy_digest: "a".repeat(64),
        expected_downstream_intent_id: intent.to_string(),
    }
}

#[test]
fn authentication_and_navigation_share_one_admission_contract() {
    let authentication = admit_challenge_consumer(
        &evidence(
            ChallengeTaskFixture::PassAfterAcknowledgedResolution,
            "authentication-run-start",
        ),
        request(
            ChallengeConsumerKind::Authentication,
            "authentication-run-start",
        ),
    )
    .unwrap();
    let navigation = admit_challenge_consumer(
        &evidence(
            ChallengeTaskFixture::ChallengeNotPresent,
            "navigation-dispatch",
        ),
        request(ChallengeConsumerKind::Navigation, "navigation-dispatch"),
    )
    .unwrap();

    assert_eq!(
        authentication.consumer_admission,
        ChallengeConsumerAdmission::Admitted
    );
    assert_eq!(
        navigation.consumer_admission,
        ChallengeConsumerAdmission::Admitted
    );
    assert_eq!(
        authentication.consumer,
        ChallengeConsumerKind::Authentication
    );
    assert_eq!(navigation.consumer, ChallengeConsumerKind::Navigation);
}

#[test]
fn admission_preserves_cooldown_and_effect_evidence_without_rejecting_a_verified_pass() {
    let mut completed = evidence(
        ChallengeTaskFixture::PassAfterAcknowledgedResolution,
        "authentication-run-start",
    );
    completed.emitted_effects = true;

    let receipt = admit_challenge_consumer(
        &completed,
        request(
            ChallengeConsumerKind::Authentication,
            "authentication-run-start",
        ),
    )
    .unwrap();

    assert!(receipt.challenge_emitted_effects);
    assert_eq!(receipt.challenge_cooldown, completed.cooldown);
}

#[test]
fn mismatched_policy_or_intent_fails_before_consumer_admission() {
    let completed = evidence(
        ChallengeTaskFixture::ChallengeNotPresent,
        "navigation-dispatch",
    );
    let mut policy_mismatch = request(ChallengeConsumerKind::Navigation, "navigation-dispatch");
    policy_mismatch.expected_site_policy_digest = "b".repeat(64);
    assert_eq!(
        admit_challenge_consumer(&completed, policy_mismatch),
        Err(ChallengeConsumerAdmissionError::SitePolicyMismatch)
    );

    assert_eq!(
        admit_challenge_consumer(
            &completed,
            request(ChallengeConsumerKind::Navigation, "other-intent"),
        ),
        Err(ChallengeConsumerAdmissionError::DownstreamIntentMismatch)
    );
}

#[test]
fn denied_or_intervention_receipts_withhold_both_consumers() {
    for fixture in [
        ChallengeTaskFixture::RejectedResolution,
        ChallengeTaskFixture::AmbiguousObservation,
    ] {
        let completed = evidence(fixture, "navigation-dispatch");
        assert_eq!(
            admit_challenge_consumer(
                &completed,
                request(ChallengeConsumerKind::Navigation, "navigation-dispatch"),
            ),
            Err(ChallengeConsumerAdmissionError::ChallengeWithheld)
        );
    }
}

#[test]
fn inconsistent_terminal_or_pass_receipts_fail_closed() {
    let mut wrong_terminal = evidence(
        ChallengeTaskFixture::ChallengeNotPresent,
        "navigation-dispatch",
    );
    wrong_terminal.phases.pop();
    assert_eq!(
        admit_challenge_consumer(
            &wrong_terminal,
            request(ChallengeConsumerKind::Navigation, "navigation-dispatch"),
        ),
        Err(ChallengeConsumerAdmissionError::InvalidReceipt)
    );

    let mut unverified_pass = evidence(
        ChallengeTaskFixture::PassAfterAcknowledgedResolution,
        "authentication-run-start",
    );
    unverified_pass.verification = None;
    assert_eq!(
        admit_challenge_consumer(
            &unverified_pass,
            request(
                ChallengeConsumerKind::Authentication,
                "authentication-run-start",
            ),
        ),
        Err(ChallengeConsumerAdmissionError::InvalidReceipt)
    );
}

#[test]
fn exact_replay_is_deterministic_and_emits_no_consumer_effect() {
    let completed = evidence(
        ChallengeTaskFixture::ChallengeNotPresent,
        "navigation-dispatch",
    );
    let first = admit_challenge_consumer(
        &completed,
        request(ChallengeConsumerKind::Navigation, "navigation-dispatch"),
    )
    .unwrap();
    let replay = admit_challenge_consumer(
        &completed,
        request(ChallengeConsumerKind::Navigation, "navigation-dispatch"),
    )
    .unwrap();

    assert_eq!(first, replay);
}
