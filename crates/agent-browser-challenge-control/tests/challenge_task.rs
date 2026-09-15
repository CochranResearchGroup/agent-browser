use agent_browser_challenge_control::{
    execute_provider_free_task, ChallengeTaskAdmission, ChallengeTaskCooldown,
    ChallengeTaskDelivery, ChallengeTaskError, ChallengeTaskFixture, ChallengeTaskIntervention,
    ChallengeTaskOutcome, ChallengeTaskPhase, ChallengeTaskRequest, ChallengeTaskVerification,
    TURNSTILE_PROFILE,
};

#[test]
fn provider_free_pass_task_owns_the_complete_lifecycle() {
    let receipt = execute_provider_free_task(ChallengeTaskRequest {
        task_id: "challenge-task-1".to_string(),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "a".repeat(64),
        downstream_intent_id: "authenticate-account".to_string(),
        fixture: ChallengeTaskFixture::PassAfterAcknowledgedResolution,
        max_transitions: 8,
    })
    .unwrap();

    assert_eq!(
        receipt.phases,
        vec![
            ChallengeTaskPhase::Ready,
            ChallengeTaskPhase::Observing,
            ChallengeTaskPhase::Deciding,
            ChallengeTaskPhase::Resolving,
            ChallengeTaskPhase::Verifying,
            ChallengeTaskPhase::Admitted,
        ]
    );
    assert_eq!(receipt.outcome, ChallengeTaskOutcome::Passed);
    assert_eq!(receipt.delivery, Some(ChallengeTaskDelivery::Acknowledged));
    assert_eq!(
        receipt.verification,
        Some(ChallengeTaskVerification::Passed)
    );
    assert_eq!(receipt.admission, ChallengeTaskAdmission::Admitted);
    assert_eq!(receipt.attempts_started, 1);
    assert!(!receipt.emitted_effects);
}

#[test]
fn task_rejects_an_invalid_site_policy_digest() {
    let result = execute_provider_free_task(ChallengeTaskRequest {
        task_id: "challenge-task-invalid-policy".to_string(),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "not-a-sha256".to_string(),
        downstream_intent_id: "authenticate-account".to_string(),
        fixture: ChallengeTaskFixture::PassAfterAcknowledgedResolution,
        max_transitions: 8,
    });

    assert_eq!(result, Err(ChallengeTaskError::InvalidRequest));
}

#[test]
fn absent_challenge_admits_without_a_resolution_attempt() {
    let receipt = execute_provider_free_task(ChallengeTaskRequest {
        task_id: "challenge-task-not-present".to_string(),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "b".repeat(64),
        downstream_intent_id: "navigate-after-check".to_string(),
        fixture: ChallengeTaskFixture::ChallengeNotPresent,
        max_transitions: 4,
    })
    .unwrap();

    assert_eq!(
        receipt.phases,
        vec![
            ChallengeTaskPhase::Ready,
            ChallengeTaskPhase::Observing,
            ChallengeTaskPhase::Deciding,
            ChallengeTaskPhase::Admitted,
        ]
    );
    assert_eq!(receipt.outcome, ChallengeTaskOutcome::NotPresent);
    assert_eq!(receipt.delivery, None);
    assert_eq!(receipt.verification, None);
    assert_eq!(receipt.admission, ChallengeTaskAdmission::Admitted);
    assert_eq!(receipt.attempts_started, 0);
    assert!(!receipt.emitted_effects);
}

#[test]
fn ambiguous_observation_requires_intervention_without_an_attempt() {
    let receipt = execute_provider_free_task(ChallengeTaskRequest {
        task_id: "challenge-task-ambiguous".to_string(),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "c".repeat(64),
        downstream_intent_id: "authenticate-account".to_string(),
        fixture: ChallengeTaskFixture::AmbiguousObservation,
        max_transitions: 4,
    })
    .unwrap();

    assert_eq!(
        receipt.phases,
        vec![
            ChallengeTaskPhase::Ready,
            ChallengeTaskPhase::Observing,
            ChallengeTaskPhase::Deciding,
            ChallengeTaskPhase::InterventionRequired,
        ]
    );
    assert_eq!(receipt.outcome, ChallengeTaskOutcome::InterventionRequired);
    assert_eq!(
        receipt.intervention,
        Some(ChallengeTaskIntervention::Ambiguous)
    );
    assert_eq!(receipt.admission, ChallengeTaskAdmission::Withheld);
    assert_eq!(receipt.attempts_started, 0);
    assert!(!receipt.emitted_effects);
}

#[test]
fn rejected_resolution_withholds_admission_and_projects_cooldown() {
    let receipt = execute_provider_free_task(ChallengeTaskRequest {
        task_id: "challenge-task-denied".to_string(),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "d".repeat(64),
        downstream_intent_id: "navigate-after-check".to_string(),
        fixture: ChallengeTaskFixture::RejectedResolution,
        max_transitions: 8,
    })
    .unwrap();

    assert_eq!(
        receipt.phases,
        vec![
            ChallengeTaskPhase::Ready,
            ChallengeTaskPhase::Observing,
            ChallengeTaskPhase::Deciding,
            ChallengeTaskPhase::Resolving,
            ChallengeTaskPhase::CoolingDown,
            ChallengeTaskPhase::NotAdmitted,
        ]
    );
    assert_eq!(receipt.outcome, ChallengeTaskOutcome::Denied);
    assert_eq!(receipt.delivery, Some(ChallengeTaskDelivery::Rejected));
    assert_eq!(receipt.verification, None);
    assert_eq!(receipt.cooldown, ChallengeTaskCooldown::Active);
    assert_eq!(receipt.admission, ChallengeTaskAdmission::Withheld);
    assert_eq!(receipt.attempts_started, 1);
    assert!(!receipt.emitted_effects);
}

#[test]
fn task_fails_closed_when_the_transition_budget_is_too_small() {
    let result = execute_provider_free_task(ChallengeTaskRequest {
        task_id: "challenge-task-budget".to_string(),
        profile: TURNSTILE_PROFILE,
        site_policy_digest: "e".repeat(64),
        downstream_intent_id: "authenticate-account".to_string(),
        fixture: ChallengeTaskFixture::PassAfterAcknowledgedResolution,
        max_transitions: 4,
    });

    assert_eq!(result, Err(ChallengeTaskError::TransitionBudgetExceeded));
}
