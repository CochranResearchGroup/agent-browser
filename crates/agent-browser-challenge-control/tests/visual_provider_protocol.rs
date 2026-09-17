use agent_browser_challenge_control::{
    adjudicate_visual_provider_response, decide_visual_round, prepare_visual_provider_request,
    visual_candidate_set_digest, visual_provider_response_digest, visual_round_evidence_digest,
    PreparedVisualArtifact, VisualProviderCapability, VisualProviderDecision,
    VisualProviderDisposition, VisualProviderResponse, VisualRoundDecision, VisualRoundEvent,
    VisualRoundEvidence, VisualRoundExecutionPlan, VisualRoundInterventionReason,
    VisualRoundPolicy, VisualRoundSnapshot,
};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn capability() -> VisualProviderCapability {
    VisualProviderCapability {
        capability_id: "fake-visual-reasoning".to_string(),
        capability_version: "v1".to_string(),
        capability_digest: digest('c'),
    }
}

fn policy() -> VisualRoundPolicy {
    VisualRoundPolicy {
        policy_digest: digest('a'),
        profile_digest: digest('b'),
        provider_capability: capability(),
        max_rounds: 2,
        max_selections_per_round: 2,
        max_steps_per_round: 1,
        max_pointer_events_per_round: 4,
        max_key_events_per_round: 0,
        max_total_selections: 3,
        max_total_steps: 2,
        max_total_pointer_events: 8,
        max_total_key_events: 0,
        deadline_at_ms: 10_000,
    }
}

fn evidence() -> VisualRoundEvidence {
    let candidate_ids = vec!["candidate:1".to_string(), "candidate:2".to_string()];
    let mut evidence = VisualRoundEvidence {
        task_id: "task:visual".to_string(),
        attempt_id: "attempt:1".to_string(),
        profile_digest: digest('b'),
        policy_digest: digest('a'),
        round_index: 1,
        round_id: "round:1".to_string(),
        evidence_digest: String::new(),
        frame_digest: digest('1'),
        context_digest: digest('2'),
        geometry_digest: digest('3'),
        candidate_set_digest: visual_candidate_set_digest(&candidate_ids),
        candidate_ids,
        observed_at_ms: 1_000,
        expires_at_ms: 2_000,
        provider_capability: capability(),
    };
    evidence.evidence_digest = visual_round_evidence_digest(&evidence);
    evidence
}

fn prepared_artifact() -> PreparedVisualArtifact {
    PreparedVisualArtifact {
        artifact_id: "artifact:prepared:1".to_string(),
        artifact_digest: digest('d'),
    }
}

fn execution_plan() -> VisualRoundExecutionPlan {
    VisualRoundExecutionPlan {
        planned_steps: 1,
        planned_pointer_events: 4,
        planned_key_events: 0,
    }
}

fn prepare_request(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
) -> agent_browser_challenge_control::VisualProviderRequest {
    prepare_visual_provider_request(
        policy,
        evidence,
        prepared_artifact(),
        execution_plan(),
        1_100,
    )
    .unwrap()
}

fn selected_response(
    request_digest: &str,
    evidence: &VisualRoundEvidence,
    selected_candidate_ids: Vec<String>,
) -> VisualProviderResponse {
    let mut response = VisualProviderResponse {
        request_digest: request_digest.to_string(),
        evidence_digest: evidence.evidence_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        provider_capability: capability(),
        disposition: VisualProviderDisposition::Selected {
            selected_candidate_ids,
        },
        produced_at_ms: 1_200,
        expires_at_ms: 1_900,
        response_digest: String::new(),
    };
    response.response_digest = visual_provider_response_digest(&response);
    response
}

fn assert_intervention(decision: VisualProviderDecision, expected: VisualRoundInterventionReason) {
    assert_eq!(decision, VisualProviderDecision::Intervention(expected));
    assert!(!decision.emitted_effects());
}

#[test]
fn valid_fake_provider_selection_crosses_the_round_seam_without_emitting_effects() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    let response = selected_response(
        &request.request_digest,
        &evidence,
        vec!["candidate:1".to_string()],
    );

    let decision =
        adjudicate_visual_provider_response(&policy, &evidence, &request, &response, 1_300)
            .unwrap();
    let VisualProviderDecision::Selection(selection) = decision else {
        panic!("expected a candidate-only selection");
    };
    assert_eq!(selection.planned_steps, 1);
    assert_eq!(selection.planned_pointer_events, 4);
    assert_eq!(selection.planned_key_events, 0);

    let initial = VisualRoundSnapshot::new("task:visual", "attempt:1").unwrap();
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence,
            now_ms: 1_100,
        },
    )
    .unwrap();
    let permitted = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection,
            now_ms: 1_300,
        },
    )
    .unwrap();

    assert!(matches!(
        permitted,
        VisualRoundDecision::PermitIntent { .. }
    ));
    assert!(!permitted.emitted_effects());
}

#[test]
fn request_digest_is_deterministic_and_binds_candidate_order_and_artifact() {
    let policy = policy();
    let evidence = evidence();
    let first = prepare_request(&policy, &evidence);
    let second = prepare_request(&policy, &evidence);
    assert_eq!(first, second);

    let mut reordered = first.clone();
    reordered.candidate_ids.reverse();
    assert_ne!(
        first.request_digest,
        agent_browser_challenge_control::visual_provider_request_digest(&reordered)
    );

    let mut changed_artifact = first.clone();
    changed_artifact.prepared_artifact.artifact_digest = digest('e');
    assert_ne!(
        first.request_digest,
        agent_browser_challenge_control::visual_provider_request_digest(&changed_artifact)
    );

    let serialized = serde_json::to_value(&first).unwrap();
    for (field, replacement) in [
        ("taskId", serde_json::json!("task:changed")),
        ("attemptId", serde_json::json!("attempt:changed")),
        ("roundIndex", serde_json::json!(2)),
        ("roundId", serde_json::json!("round:changed")),
        ("policyDigest", serde_json::json!(digest('1'))),
        ("profileDigest", serde_json::json!(digest('2'))),
        ("evidenceDigest", serde_json::json!(digest('3'))),
        ("frameDigest", serde_json::json!(digest('4'))),
        ("contextDigest", serde_json::json!(digest('5'))),
        ("geometryDigest", serde_json::json!(digest('6'))),
        ("candidateSetDigest", serde_json::json!(digest('7'))),
        (
            "candidateIds",
            serde_json::json!(["candidate:2", "candidate:1"]),
        ),
        (
            "preparedArtifact",
            serde_json::json!({
                "artifactId": "artifact:changed",
                "artifactDigest": digest('8')
            }),
        ),
        (
            "executionPlan",
            serde_json::json!({
                "plannedSteps": 1,
                "plannedPointerEvents": 3,
                "plannedKeyEvents": 0
            }),
        ),
        (
            "providerCapability",
            serde_json::json!({
                "capabilityId": "fake-visual-reasoning-changed",
                "capabilityVersion": "v2",
                "capabilityDigest": digest('9')
            }),
        ),
        ("requestedAtMs", serde_json::json!(1_101)),
        ("expiresAtMs", serde_json::json!(1_999)),
    ] {
        let mut changed = serialized.clone();
        changed
            .as_object_mut()
            .unwrap()
            .insert(field.to_string(), replacement);
        let changed: agent_browser_challenge_control::VisualProviderRequest =
            serde_json::from_value(changed).unwrap();
        assert_ne!(
            first.request_digest,
            agent_browser_challenge_control::visual_provider_request_digest(&changed),
            "request digest did not bind {field}"
        );
    }
}

#[test]
fn request_and_response_mutations_fail_before_selection() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    let response = selected_response(
        &request.request_digest,
        &evidence,
        vec!["candidate:1".to_string()],
    );

    let mut changed_request = request.clone();
    changed_request.frame_digest = digest('f');
    assert_intervention(
        adjudicate_visual_provider_response(&policy, &evidence, &changed_request, &response, 1_300)
            .unwrap(),
        VisualRoundInterventionReason::ProviderRequestMismatch,
    );

    for changed_response in [
        response.clone(),
        response.clone(),
        response.clone(),
        response.clone(),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, mut response)| {
        match index {
            0 => response.request_digest = digest('1'),
            1 => response.evidence_digest = digest('2'),
            2 => response.candidate_set_digest = digest('3'),
            3 => response.response_digest = digest('4'),
            _ => unreachable!(),
        }
        response
    }) {
        assert_intervention(
            adjudicate_visual_provider_response(
                &policy,
                &evidence,
                &request,
                &changed_response,
                1_300,
            )
            .unwrap(),
            VisualRoundInterventionReason::ProviderResponseMismatch,
        );
    }

    let mut changed_capability = response;
    changed_capability.provider_capability.capability_digest = digest('9');
    changed_capability.response_digest = visual_provider_response_digest(&changed_capability);
    assert_intervention(
        adjudicate_visual_provider_response(
            &policy,
            &evidence,
            &request,
            &changed_capability,
            1_300,
        )
        .unwrap(),
        VisualRoundInterventionReason::CapabilityMismatch,
    );
}

#[test]
fn stale_request_or_response_fails_before_selection() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    let valid_response = selected_response(
        &request.request_digest,
        &evidence,
        vec!["candidate:1".to_string()],
    );
    assert_intervention(
        adjudicate_visual_provider_response(&policy, &evidence, &request, &valid_response, 2_000)
            .unwrap(),
        VisualRoundInterventionReason::StaleEvidence,
    );

    for (mut response, now_ms) in [
        (
            {
                let mut response = valid_response.clone();
                response.expires_at_ms = 1_250;
                response
            },
            1_300,
        ),
        (
            {
                let mut response = valid_response.clone();
                response.produced_at_ms = request.requested_at_ms - 1;
                response
            },
            1_300,
        ),
        (
            {
                let mut response = valid_response.clone();
                response.produced_at_ms = 1_301;
                response
            },
            1_300,
        ),
        (
            {
                let mut response = valid_response.clone();
                response.produced_at_ms = response.expires_at_ms;
                response
            },
            1_300,
        ),
        (
            {
                let mut response = valid_response.clone();
                response.expires_at_ms = request.expires_at_ms + 1;
                response
            },
            1_300,
        ),
    ] {
        response.response_digest = visual_provider_response_digest(&response);
        assert_intervention(
            adjudicate_visual_provider_response(&policy, &evidence, &request, &response, now_ms)
                .unwrap(),
            VisualRoundInterventionReason::StaleEvidence,
        );
    }
}

#[test]
fn invalid_candidate_sets_and_budget_excess_fail_before_intent() {
    let evidence = evidence();
    for (selected, expected) in [
        (
            Vec::<String>::new(),
            VisualRoundInterventionReason::CandidateMismatch,
        ),
        (
            vec!["candidate:1".to_string(), "candidate:1".to_string()],
            VisualRoundInterventionReason::CandidateMismatch,
        ),
        (
            vec!["candidate:outside".to_string()],
            VisualRoundInterventionReason::CandidateMismatch,
        ),
    ] {
        let policy = policy();
        let request = prepare_request(&policy, &evidence);
        let response = selected_response(&request.request_digest, &evidence, selected);
        assert_intervention(
            adjudicate_visual_provider_response(&policy, &evidence, &request, &response, 1_300)
                .unwrap(),
            expected,
        );
    }

    let mut constrained = policy();
    constrained.max_selections_per_round = 1;
    let request = prepare_request(&constrained, &evidence);
    let response = selected_response(
        &request.request_digest,
        &evidence,
        vec!["candidate:1".to_string(), "candidate:2".to_string()],
    );
    assert_intervention(
        adjudicate_visual_provider_response(&constrained, &evidence, &request, &response, 1_300)
            .unwrap(),
        VisualRoundInterventionReason::RoundBudgetExceeded,
    );
}

#[test]
fn execution_budget_is_bound_before_provider_adjudication() {
    let policy = policy();
    let evidence = evidence();
    let mut over_budget = execution_plan();
    over_budget.planned_pointer_events = policy.max_pointer_events_per_round + 1;
    assert_eq!(
        prepare_visual_provider_request(
            &policy,
            &evidence,
            prepared_artifact(),
            over_budget,
            1_100,
        ),
        Err(agent_browser_challenge_control::VisualProviderError::InvalidExecutionPlan)
    );
}

#[test]
fn typed_abstentions_preserve_their_intervention_reason() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    for (disposition, expected) in [
        (
            VisualProviderDisposition::Ambiguous,
            VisualRoundInterventionReason::Ambiguous,
        ),
        (
            VisualProviderDisposition::Unsupported,
            VisualRoundInterventionReason::Unsupported,
        ),
        (
            VisualProviderDisposition::Inconclusive,
            VisualRoundInterventionReason::Inconclusive,
        ),
    ] {
        let mut response = selected_response(&request.request_digest, &evidence, vec![]);
        response.disposition = disposition;
        response.response_digest = visual_provider_response_digest(&response);
        assert_intervention(
            adjudicate_visual_provider_response(&policy, &evidence, &request, &response, 1_300)
                .unwrap(),
            expected,
        );
    }
}

#[test]
fn serialized_effect_instructions_and_retry_fields_are_rejected() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    let response = selected_response(
        &request.request_digest,
        &evidence,
        vec!["candidate:1".to_string()],
    );
    let serialized = serde_json::to_value(response).unwrap();

    for (field, value) in [
        ("coordinate", serde_json::json!({"x": 10, "y": 20})),
        ("eventSequence", serde_json::json!(["pointer_down"])),
        ("retry", serde_json::json!(true)),
        ("instructions", serde_json::json!("click at 10,20")),
    ] {
        let mut smuggled = serialized.clone();
        smuggled
            .as_object_mut()
            .unwrap()
            .insert(field.to_string(), value);
        assert!(serde_json::from_value::<VisualProviderResponse>(smuggled).is_err());
    }

    let mut nested = serialized;
    nested["disposition"]["coordinate"] = serde_json::json!({"x": 10, "y": 20});
    assert!(serde_json::from_value::<VisualProviderResponse>(nested).is_err());
}

#[test]
fn serialized_requests_reject_effect_authority_and_nested_unknown_fields() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    let serialized = serde_json::to_value(request).unwrap();

    for (field, value) in [
        ("coordinate", serde_json::json!({"x": 10, "y": 20})),
        ("eventSequence", serde_json::json!(["pointer_down"])),
        ("retry", serde_json::json!(true)),
        ("instructions", serde_json::json!("click at 10,20")),
    ] {
        let mut smuggled = serialized.clone();
        smuggled
            .as_object_mut()
            .unwrap()
            .insert(field.to_string(), value);
        assert!(
            serde_json::from_value::<agent_browser_challenge_control::VisualProviderRequest>(
                smuggled
            )
            .is_err()
        );
    }

    let mut artifact_smuggling = serialized.clone();
    artifact_smuggling["preparedArtifact"]["bytes"] = serde_json::json!("opaque-image-data");
    assert!(
        serde_json::from_value::<agent_browser_challenge_control::VisualProviderRequest>(
            artifact_smuggling
        )
        .is_err()
    );

    let mut plan_smuggling = serialized;
    plan_smuggling["executionPlan"]["repeatCount"] = serde_json::json!(2);
    assert!(
        serde_json::from_value::<agent_browser_challenge_control::VisualProviderRequest>(
            plan_smuggling
        )
        .is_err()
    );
}

#[test]
fn replaying_identical_provider_input_is_deterministic_and_effect_free() {
    let policy = policy();
    let evidence = evidence();
    let request = prepare_request(&policy, &evidence);
    let response = selected_response(
        &request.request_digest,
        &evidence,
        vec!["candidate:1".to_string()],
    );
    let first = adjudicate_visual_provider_response(&policy, &evidence, &request, &response, 1_300)
        .unwrap();
    let replay =
        adjudicate_visual_provider_response(&policy, &evidence, &request, &response, 1_300)
            .unwrap();
    assert_eq!(first, replay);
    assert!(!first.emitted_effects());
    assert!(!replay.emitted_effects());
}
