use agent_browser_candidate::{
    advise, apply_transition, ActiveOperation, AdvisoryInput, AdvisoryOutcome, CandidateRelation,
    IntegrityPreconditions, ObservedState, OperationRecord, OperationState, OperatorAction,
    TransitionRequest, TransitionResult,
};

fn active(candidate_id: &str, state: OperationState) -> ActiveOperation {
    ActiveOperation {
        operation_id: "operation-1".to_string(),
        candidate_id: candidate_id.to_string(),
        state,
        revision: 3,
        fencing_generation: 7,
    }
}

#[test]
fn equivalent_active_build_is_joined_instead_of_duplicated() {
    let result = advise(AdvisoryInput {
        observed_state: ObservedState::BuildActive,
        requested_candidate_id: "candidate-a".to_string(),
        candidate_relation: CandidateRelation::EquivalentInput,
        active_operation: Some(active("candidate-a", OperationState::Building)),
        integrity: IntegrityPreconditions::satisfied(),
    });

    assert_eq!(result.outcome, AdvisoryOutcome::JoinedExisting);
    assert_eq!(result.recommendation, OperatorAction::Observe);
    assert!(result.alternatives.contains(&OperatorAction::Cancel));
    assert!(result
        .consequences
        .iter()
        .any(|value| value == "no_new_build"));
}

#[test]
fn competing_candidate_reports_choices_without_claiming_permission_denied() {
    let result = advise(AdvisoryInput {
        observed_state: ObservedState::InstallActive,
        requested_candidate_id: "candidate-b".to_string(),
        candidate_relation: CandidateRelation::CompetingCandidate,
        active_operation: Some(active("candidate-a", OperationState::Installing)),
        integrity: IntegrityPreconditions::satisfied(),
    });

    assert_eq!(result.outcome, AdvisoryOutcome::Queued);
    assert_eq!(result.recommendation, OperatorAction::Wait);
    assert_eq!(
        result.alternatives,
        vec![
            OperatorAction::Queue,
            OperatorAction::Cancel,
            OperatorAction::Discard,
            OperatorAction::Supersede,
        ]
    );
    assert!(!result
        .reason_codes
        .iter()
        .any(|code| code == "permission_denied"));
}

#[test]
fn failed_integrity_precondition_routes_to_recovery_without_an_effect() {
    let result = advise(AdvisoryInput {
        observed_state: ObservedState::InstallActive,
        requested_candidate_id: "candidate-a".to_string(),
        candidate_relation: CandidateRelation::SameCandidate,
        active_operation: Some(active("candidate-a", OperationState::Installing)),
        integrity: IntegrityPreconditions {
            manifest_valid: true,
            artifact_digest_valid: true,
            revision_current: true,
            fencing_generation_current: false,
        },
    });

    assert_eq!(result.outcome, AdvisoryOutcome::IntegrityPreconditionFailed);
    assert_eq!(result.recommendation, OperatorAction::Recover);
    assert_eq!(result.alternatives, vec![OperatorAction::Inspect]);
    assert_eq!(
        result.integrity_preconditions,
        vec!["fencing_generation_current"]
    );
}

#[test]
fn transitions_are_idempotent_and_fence_stale_writers() {
    let mut operation = OperationRecord::new("operation-1", "candidate-a", 7);
    let request = TransitionRequest {
        request_id: "request-1".to_string(),
        expected_revision: 0,
        expected_fencing_generation: 7,
        next_state: OperationState::Building,
    };

    let applied = apply_transition(&mut operation, request.clone()).expect("first transition");
    assert_eq!(applied, TransitionResult::Applied);
    assert_eq!(operation.revision, 1);

    let replay = apply_transition(&mut operation, request).expect("idempotent replay");
    assert_eq!(replay, TransitionResult::AlreadyApplied);
    assert_eq!(operation.revision, 1);

    let stale = apply_transition(
        &mut operation,
        TransitionRequest {
            request_id: "request-2".to_string(),
            expected_revision: 1,
            expected_fencing_generation: 6,
            next_state: OperationState::Cancelled,
        },
    )
    .expect_err("stale fencing generation must fail");
    assert_eq!(stale.code(), "stale_fencing_generation");
    assert_eq!(operation.state, OperationState::Building);
}
