use agent_browser_challenge_control::{
    visual_candidate_set_digest, visual_provider_response_digest, visual_round_evidence_digest,
    VisualProviderCapability, VisualProviderDecision, VisualProviderDisposition,
    VisualProviderResponse, VisualRoundEvidence, VisualRoundExecutionPlan, VisualRoundPolicy,
};
use agent_browser_challenge_visual_adapter::{
    invoke_visual_provider, visual_artifact_envelope_digest, visual_payload_digest,
    visual_provider_invocation_digest, PreparedVisualPayload, VisualAdapterError,
    VisualArtifactEnvelope, VisualArtifactRetention, VisualInvocationPolicy,
    VisualProviderInvocation, VisualProviderTransport, VisualTransportFailure,
    VisualTransportResponse,
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

fn round_policy() -> VisualRoundPolicy {
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

fn invocation_policy() -> VisualInvocationPolicy {
    VisualInvocationPolicy {
        max_payload_bytes: 64,
        max_request_bytes: 4_096,
        max_response_bytes: 4_096,
        allowed_media_types: vec!["image/png".to_string()],
        max_width: 64,
        max_height: 64,
    }
}

fn execution_plan() -> VisualRoundExecutionPlan {
    VisualRoundExecutionPlan {
        planned_steps: 1,
        planned_pointer_events: 4,
        planned_key_events: 0,
    }
}

fn payload(evidence: &VisualRoundEvidence) -> PreparedVisualPayload {
    let bytes = vec![0x89, 0x50, 0x4e, 0x47];
    let mut envelope = VisualArtifactEnvelope {
        artifact_id: "artifact:synthetic:1".to_string(),
        byte_digest: visual_payload_digest(&bytes),
        byte_length: bytes.len() as u64,
        media_type: "image/png".to_string(),
        width: 2,
        height: 2,
        evidence_digest: evidence.evidence_digest.clone(),
        frame_digest: evidence.frame_digest.clone(),
        context_digest: evidence.context_digest.clone(),
        geometry_digest: evidence.geometry_digest.clone(),
        redaction_policy_digest: digest('d'),
        redaction_receipt_digest: digest('e'),
        provider_capability: capability(),
        prepared_at_ms: 1_050,
        expires_at_ms: evidence.expires_at_ms,
        retention: VisualArtifactRetention::EphemeralProcessLocal,
        envelope_digest: String::new(),
    };
    envelope.envelope_digest = visual_artifact_envelope_digest(&envelope);
    PreparedVisualPayload { envelope, bytes }
}

#[derive(Default)]
struct RecordingTransport {
    calls: usize,
    invocation_digest: Option<String>,
    invocation: Option<VisualProviderInvocation>,
    disposition: Option<VisualProviderDisposition>,
    response_bytes: Option<Vec<u8>>,
    failure: Option<VisualTransportFailure>,
}

impl VisualProviderTransport for RecordingTransport {
    fn invoke(
        &mut self,
        invocation: &VisualProviderInvocation,
    ) -> Result<VisualTransportResponse, VisualTransportFailure> {
        self.calls += 1;
        self.invocation_digest = Some(invocation.invocation_digest.clone());
        self.invocation = Some(invocation.clone());
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        if let Some(response_bytes) = &self.response_bytes {
            return Ok(VisualTransportResponse {
                response_bytes: response_bytes.clone(),
                received_at_ms: 1_300,
            });
        }
        let mut response = VisualProviderResponse {
            request_digest: invocation.request.request_digest.clone(),
            evidence_digest: invocation.request.evidence_digest.clone(),
            candidate_set_digest: invocation.request.candidate_set_digest.clone(),
            provider_capability: invocation.request.provider_capability.clone(),
            disposition: self.disposition.clone().unwrap_or_else(|| {
                VisualProviderDisposition::Selected {
                    selected_candidate_ids: vec!["candidate:1".to_string()],
                }
            }),
            produced_at_ms: invocation.request.requested_at_ms,
            expires_at_ms: invocation.request.expires_at_ms,
            response_digest: String::new(),
        };
        response.response_digest = visual_provider_response_digest(&response);
        Ok(VisualTransportResponse {
            response_bytes: serde_json::to_vec(&response).unwrap(),
            received_at_ms: 1_300,
        })
    }
}

#[test]
fn valid_synthetic_payload_invokes_once_and_returns_bound_selection() {
    let policy = round_policy();
    let evidence = evidence();
    let payload = payload(&evidence);
    let mut transport = RecordingTransport::default();

    let decision = invoke_visual_provider(
        &invocation_policy(),
        &policy,
        &evidence,
        payload,
        execution_plan(),
        &mut transport,
        1_100,
    )
    .unwrap();

    assert_eq!(transport.calls, 1);
    assert!(transport.invocation_digest.is_some());
    let VisualProviderDecision::Selection(selection) = decision else {
        panic!("expected a bound candidate selection");
    };
    assert_eq!(selection.selected_candidate_ids, vec!["candidate:1"]);
    assert!(!VisualProviderDecision::Selection(selection).emitted_effects());
}

#[test]
fn ambiguous_invocation_policy_stops_before_transport() {
    let round_policy = round_policy();
    let evidence = evidence();
    let payload = payload(&evidence);
    let mut invocation_policy = invocation_policy();
    invocation_policy
        .allowed_media_types
        .push("image/png".to_string());
    let mut transport = RecordingTransport::default();

    let error = invoke_visual_provider(
        &invocation_policy,
        &round_policy,
        &evidence,
        payload,
        execution_plan(),
        &mut transport,
        1_100,
    )
    .unwrap_err();

    assert_eq!(error, VisualAdapterError::InvalidInvocationPolicy);
    assert_eq!(transport.calls, 0);
}

#[test]
fn artifact_expiry_must_match_the_exact_request_boundary() {
    let mut round_policy = round_policy();
    round_policy.deadline_at_ms = 1_500;
    let evidence = evidence();
    let payload = payload(&evidence);
    let mut transport = RecordingTransport::default();

    let error = invoke_visual_provider(
        &invocation_policy(),
        &round_policy,
        &evidence,
        payload,
        execution_plan(),
        &mut transport,
        1_100,
    )
    .unwrap_err();

    assert_eq!(error, VisualAdapterError::InvalidArtifact);
    assert_eq!(transport.calls, 0);
}

fn refresh_envelope(payload: &mut PreparedVisualPayload) {
    payload.envelope.envelope_digest = visual_artifact_envelope_digest(&payload.envelope);
}

fn assert_invalid_payload(
    round_policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    payload: PreparedVisualPayload,
) {
    let mut transport = RecordingTransport::default();
    assert_eq!(
        invoke_visual_provider(
            &invocation_policy(),
            round_policy,
            evidence,
            payload,
            execution_plan(),
            &mut transport,
            1_100,
        ),
        Err(VisualAdapterError::InvalidArtifact)
    );
    assert_eq!(transport.calls, 0);
}

#[test]
fn invalid_artifact_matrix_never_reaches_transport() {
    let policy = round_policy();
    let evidence = evidence();

    let mut cases = Vec::new();

    let mut changed = payload(&evidence);
    changed.envelope.evidence_digest = digest('4');
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.frame_digest = digest('5');
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.context_digest = digest('6');
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.geometry_digest = digest('7');
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.bytes.push(0xff);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.byte_length += 1;
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.bytes.clear();
    changed.envelope.byte_length = 0;
    changed.envelope.byte_digest = visual_payload_digest(&changed.bytes);
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.media_type = "image/gif".to_string();
    refresh_envelope(&mut changed);
    cases.push(changed);

    for (width, height) in [(0, 2), (65, 2), (2, 0), (2, 65)] {
        let mut changed = payload(&evidence);
        changed.envelope.width = width;
        changed.envelope.height = height;
        refresh_envelope(&mut changed);
        cases.push(changed);
    }

    let mut changed = payload(&evidence);
    changed.envelope.redaction_policy_digest.clear();
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.redaction_receipt_digest = "not-a-digest".to_string();
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.provider_capability.capability_digest = digest('8');
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.prepared_at_ms = 999;
    refresh_envelope(&mut changed);
    cases.push(changed);

    let mut changed = payload(&evidence);
    changed.envelope.expires_at_ms = 1_100;
    refresh_envelope(&mut changed);
    cases.push(changed);

    for invalid in cases {
        assert_invalid_payload(&policy, &evidence, invalid);
    }

    let serialized = serde_json::to_value(&payload(&evidence).envelope).unwrap();
    let mut retained = serialized.clone();
    retained["retention"] = serde_json::json!("durable");
    assert!(serde_json::from_value::<VisualArtifactEnvelope>(retained).is_err());
    let mut unknown = serialized;
    unknown["pixels"] = serde_json::json!("private-bytes");
    assert!(serde_json::from_value::<VisualArtifactEnvelope>(unknown).is_err());
}

#[test]
fn payload_request_and_response_byte_ceilings_preserve_call_cardinality() {
    let round_policy = round_policy();
    let evidence = evidence();

    let mut payload_policy = invocation_policy();
    payload_policy.max_payload_bytes = 3;
    let mut transport = RecordingTransport::default();
    assert_eq!(
        invoke_visual_provider(
            &payload_policy,
            &round_policy,
            &evidence,
            payload(&evidence),
            execution_plan(),
            &mut transport,
            1_100,
        ),
        Err(VisualAdapterError::InvalidArtifact)
    );
    assert_eq!(transport.calls, 0);

    let mut request_policy = invocation_policy();
    request_policy.max_request_bytes = 1;
    let mut transport = RecordingTransport::default();
    assert_eq!(
        invoke_visual_provider(
            &request_policy,
            &round_policy,
            &evidence,
            payload(&evidence),
            execution_plan(),
            &mut transport,
            1_100,
        ),
        Err(VisualAdapterError::RequestTooLarge)
    );
    assert_eq!(transport.calls, 0);

    let mut response_policy = invocation_policy();
    response_policy.max_response_bytes = 1;
    let mut transport = RecordingTransport::default();
    assert_eq!(
        invoke_visual_provider(
            &response_policy,
            &round_policy,
            &evidence,
            payload(&evidence),
            execution_plan(),
            &mut transport,
            1_100,
        ),
        Err(VisualAdapterError::ResponseTooLarge)
    );
    assert_eq!(transport.calls, 1);
}

#[test]
fn transport_and_malformed_output_fail_after_one_call_without_retry() {
    let round_policy = round_policy();
    let evidence = evidence();

    let mut unavailable = RecordingTransport {
        failure: Some(VisualTransportFailure::Unavailable),
        ..Default::default()
    };
    assert_eq!(
        invoke_visual_provider(
            &invocation_policy(),
            &round_policy,
            &evidence,
            payload(&evidence),
            execution_plan(),
            &mut unavailable,
            1_100,
        ),
        Err(VisualAdapterError::Transport(
            VisualTransportFailure::Unavailable
        ))
    );
    assert_eq!(unavailable.calls, 1);

    let mut malformed = RecordingTransport {
        response_bytes: Some(b"not-json-or-provider-output".to_vec()),
        ..Default::default()
    };
    assert_eq!(
        invoke_visual_provider(
            &invocation_policy(),
            &round_policy,
            &evidence,
            payload(&evidence),
            execution_plan(),
            &mut malformed,
            1_100,
        ),
        Err(VisualAdapterError::MalformedResponse)
    );
    assert_eq!(malformed.calls, 1);
}

struct SmugglingTransport {
    calls: usize,
}

impl VisualProviderTransport for SmugglingTransport {
    fn invoke(
        &mut self,
        invocation: &VisualProviderInvocation,
    ) -> Result<VisualTransportResponse, VisualTransportFailure> {
        self.calls += 1;
        let mut response = VisualProviderResponse {
            request_digest: invocation.request.request_digest.clone(),
            evidence_digest: invocation.request.evidence_digest.clone(),
            candidate_set_digest: invocation.request.candidate_set_digest.clone(),
            provider_capability: invocation.request.provider_capability.clone(),
            disposition: VisualProviderDisposition::Inconclusive,
            produced_at_ms: invocation.request.requested_at_ms,
            expires_at_ms: invocation.request.expires_at_ms,
            response_digest: String::new(),
        };
        response.response_digest = visual_provider_response_digest(&response);
        let mut serialized = serde_json::to_value(response).unwrap();
        serialized["retry"] = serde_json::json!(true);
        Ok(VisualTransportResponse {
            response_bytes: serde_json::to_vec(&serialized).unwrap(),
            received_at_ms: 1_300,
        })
    }
}

#[test]
fn effect_smuggling_is_rejected_after_one_call() {
    let round_policy = round_policy();
    let evidence = evidence();
    let mut transport = SmugglingTransport { calls: 0 };
    assert_eq!(
        invoke_visual_provider(
            &invocation_policy(),
            &round_policy,
            &evidence,
            payload(&evidence),
            execution_plan(),
            &mut transport,
            1_100,
        ),
        Err(VisualAdapterError::MalformedResponse)
    );
    assert_eq!(transport.calls, 1);
}

#[test]
fn typed_abstentions_cross_the_adapter_without_retry() {
    let round_policy = round_policy();
    let evidence = evidence();
    for (disposition, expected) in [
        (
            VisualProviderDisposition::Ambiguous,
            agent_browser_challenge_control::VisualRoundInterventionReason::Ambiguous,
        ),
        (
            VisualProviderDisposition::Unsupported,
            agent_browser_challenge_control::VisualRoundInterventionReason::Unsupported,
        ),
        (
            VisualProviderDisposition::Inconclusive,
            agent_browser_challenge_control::VisualRoundInterventionReason::Inconclusive,
        ),
    ] {
        let mut transport = RecordingTransport {
            disposition: Some(disposition),
            ..Default::default()
        };
        assert_eq!(
            invoke_visual_provider(
                &invocation_policy(),
                &round_policy,
                &evidence,
                payload(&evidence),
                execution_plan(),
                &mut transport,
                1_100,
            )
            .unwrap(),
            VisualProviderDecision::Intervention(expected)
        );
        assert_eq!(transport.calls, 1);
    }
}

#[test]
fn artifact_and_invocation_digests_are_deterministic_and_diagnostics_redact_bytes() {
    let round_policy = round_policy();
    let evidence = evidence();
    let first_payload = payload(&evidence);
    assert_eq!(
        first_payload.envelope.envelope_digest,
        visual_artifact_envelope_digest(&first_payload.envelope)
    );
    let mut changed_payload = first_payload.clone();
    changed_payload.envelope.width = 3;
    assert_ne!(
        first_payload.envelope.envelope_digest,
        visual_artifact_envelope_digest(&changed_payload.envelope)
    );

    let mut first_transport = RecordingTransport::default();
    invoke_visual_provider(
        &invocation_policy(),
        &round_policy,
        &evidence,
        first_payload.clone(),
        execution_plan(),
        &mut first_transport,
        1_100,
    )
    .unwrap();
    let first = first_transport.invocation.unwrap();
    assert_eq!(
        first.invocation_digest,
        visual_provider_invocation_digest(&first)
    );

    let mut second_transport = RecordingTransport::default();
    invoke_visual_provider(
        &invocation_policy(),
        &round_policy,
        &evidence,
        first_payload,
        execution_plan(),
        &mut second_transport,
        1_100,
    )
    .unwrap();
    assert_eq!(first, second_transport.invocation.unwrap());

    let mut secret_payload = payload(&evidence);
    secret_payload.bytes = b"SECRET_PIXEL_BYTES".to_vec();
    secret_payload.envelope.byte_length = secret_payload.bytes.len() as u64;
    secret_payload.envelope.byte_digest = visual_payload_digest(&secret_payload.bytes);
    refresh_envelope(&mut secret_payload);
    let debug = format!("{secret_payload:?}");
    assert!(!debug.contains("SECRET_PIXEL_BYTES"));
    assert!(debug.contains("<redacted>"));
    assert!(!format!("{:?}", VisualAdapterError::MalformedResponse).contains("SECRET_PIXEL_BYTES"));
    let provider_output = VisualTransportResponse {
        response_bytes: b"SECRET_PROVIDER_OUTPUT".to_vec(),
        received_at_ms: 1_300,
    };
    let debug = format!("{provider_output:?}");
    assert!(!debug.contains("SECRET_PROVIDER_OUTPUT"));
    assert!(debug.contains("<redacted>"));
}

struct DelayedTransport {
    calls: usize,
}

impl VisualProviderTransport for DelayedTransport {
    fn invoke(
        &mut self,
        invocation: &VisualProviderInvocation,
    ) -> Result<VisualTransportResponse, VisualTransportFailure> {
        self.calls += 1;
        let mut response = VisualProviderResponse {
            request_digest: invocation.request.request_digest.clone(),
            evidence_digest: invocation.request.evidence_digest.clone(),
            candidate_set_digest: invocation.request.candidate_set_digest.clone(),
            provider_capability: invocation.request.provider_capability.clone(),
            disposition: VisualProviderDisposition::Selected {
                selected_candidate_ids: vec!["candidate:1".to_string()],
            },
            produced_at_ms: 1_200,
            expires_at_ms: 1_900,
            response_digest: String::new(),
        };
        response.response_digest = visual_provider_response_digest(&response);
        Ok(VisualTransportResponse {
            response_bytes: serde_json::to_vec(&response).unwrap(),
            received_at_ms: 1_300,
        })
    }
}

#[test]
fn delayed_provider_response_uses_transport_receipt_time() {
    let round_policy = round_policy();
    let evidence = evidence();
    let mut transport = DelayedTransport { calls: 0 };
    let decision = invoke_visual_provider(
        &invocation_policy(),
        &round_policy,
        &evidence,
        payload(&evidence),
        execution_plan(),
        &mut transport,
        1_100,
    )
    .unwrap();

    assert!(matches!(decision, VisualProviderDecision::Selection(_)));
    assert_eq!(transport.calls, 1);
}
