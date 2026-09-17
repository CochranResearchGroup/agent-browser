//! Provider-free visual artifact custody and one-shot provider invocation.
//!
//! This crate owns only process-local payload validation and an injected
//! transport seam. It performs no capture, network, filesystem, credential,
//! browser, desktop-input, retry, or runtime operation.

use agent_browser_challenge_control::{
    adjudicate_visual_provider_response, prepare_visual_provider_request, PreparedVisualArtifact,
    VisualProviderCapability, VisualProviderDecision, VisualProviderError, VisualProviderRequest,
    VisualProviderResponse, VisualRoundEvidence, VisualRoundExecutionPlan, VisualRoundPolicy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualInvocationPolicy {
    pub max_payload_bytes: u64,
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub allowed_media_types: Vec<String>,
    pub max_width: u32,
    pub max_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualArtifactRetention {
    EphemeralProcessLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualArtifactEnvelope {
    pub artifact_id: String,
    pub byte_digest: String,
    pub byte_length: u64,
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub evidence_digest: String,
    pub frame_digest: String,
    pub context_digest: String,
    pub geometry_digest: String,
    pub redaction_policy_digest: String,
    pub redaction_receipt_digest: String,
    pub provider_capability: VisualProviderCapability,
    pub prepared_at_ms: u64,
    pub expires_at_ms: u64,
    pub retention: VisualArtifactRetention,
    pub envelope_digest: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PreparedVisualPayload {
    pub envelope: VisualArtifactEnvelope,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for PreparedVisualPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedVisualPayload")
            .field("envelope", &self.envelope)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct VisualProviderInvocation {
    pub request: VisualProviderRequest,
    pub payload: PreparedVisualPayload,
    pub invocation_digest: String,
}

impl fmt::Debug for VisualProviderInvocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VisualProviderInvocation")
            .field("request", &self.request)
            .field("payload", &self.payload)
            .field("invocation_digest", &self.invocation_digest)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualTransportFailure {
    Unavailable,
    Timeout,
    Rejected,
}

pub trait VisualProviderTransport {
    fn invoke(
        &mut self,
        invocation: &VisualProviderInvocation,
    ) -> Result<VisualTransportResponse, VisualTransportFailure>;
}

#[derive(Clone, PartialEq, Eq)]
pub struct VisualTransportResponse {
    pub response_bytes: Vec<u8>,
    pub received_at_ms: u64,
}

impl fmt::Debug for VisualTransportResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VisualTransportResponse")
            .field("response_bytes", &"<redacted>")
            .field("received_at_ms", &self.received_at_ms)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualAdapterError {
    InvalidInvocationPolicy,
    InvalidArtifact,
    RequestTooLarge,
    ResponseTooLarge,
    Transport(VisualTransportFailure),
    MalformedResponse,
    Protocol(VisualProviderError),
}

pub fn invoke_visual_provider<T: VisualProviderTransport>(
    invocation_policy: &VisualInvocationPolicy,
    round_policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    payload: PreparedVisualPayload,
    execution_plan: VisualRoundExecutionPlan,
    transport: &mut T,
    now_ms: u64,
) -> Result<VisualProviderDecision, VisualAdapterError> {
    if !valid_invocation_policy(invocation_policy) {
        return Err(VisualAdapterError::InvalidInvocationPolicy);
    }
    if !valid_payload(invocation_policy, round_policy, evidence, &payload, now_ms) {
        return Err(VisualAdapterError::InvalidArtifact);
    }

    let request = prepare_visual_provider_request(
        round_policy,
        evidence,
        PreparedVisualArtifact {
            artifact_id: payload.envelope.artifact_id.clone(),
            artifact_digest: payload.envelope.envelope_digest.clone(),
        },
        execution_plan,
        now_ms,
    )
    .map_err(VisualAdapterError::Protocol)?;
    let request_bytes =
        serde_json::to_vec(&request).map_err(|_| VisualAdapterError::MalformedResponse)?;
    if request_bytes.len() as u64 > invocation_policy.max_request_bytes {
        return Err(VisualAdapterError::RequestTooLarge);
    }

    let mut invocation = VisualProviderInvocation {
        request,
        payload,
        invocation_digest: String::new(),
    };
    invocation.invocation_digest = visual_provider_invocation_digest(&invocation);

    let transport_response = transport
        .invoke(&invocation)
        .map_err(VisualAdapterError::Transport)?;
    if transport_response.response_bytes.len() as u64 > invocation_policy.max_response_bytes {
        return Err(VisualAdapterError::ResponseTooLarge);
    }
    let response: VisualProviderResponse =
        serde_json::from_slice(&transport_response.response_bytes)
            .map_err(|_| VisualAdapterError::MalformedResponse)?;
    adjudicate_visual_provider_response(
        round_policy,
        evidence,
        &invocation.request,
        &response,
        transport_response.received_at_ms,
    )
    .map_err(VisualAdapterError::Protocol)
}

pub fn visual_payload_digest(bytes: &[u8]) -> String {
    hex_digest(bytes)
}

pub fn visual_artifact_envelope_digest(envelope: &VisualArtifactEnvelope) -> String {
    let byte_length = envelope.byte_length.to_string();
    let width = envelope.width.to_string();
    let height = envelope.height.to_string();
    let prepared_at_ms = envelope.prepared_at_ms.to_string();
    let expires_at_ms = envelope.expires_at_ms.to_string();
    digest_parts([
        envelope.artifact_id.as_str(),
        envelope.byte_digest.as_str(),
        byte_length.as_str(),
        envelope.media_type.as_str(),
        width.as_str(),
        height.as_str(),
        envelope.evidence_digest.as_str(),
        envelope.frame_digest.as_str(),
        envelope.context_digest.as_str(),
        envelope.geometry_digest.as_str(),
        envelope.redaction_policy_digest.as_str(),
        envelope.redaction_receipt_digest.as_str(),
        envelope.provider_capability.capability_id.as_str(),
        envelope.provider_capability.capability_version.as_str(),
        envelope.provider_capability.capability_digest.as_str(),
        prepared_at_ms.as_str(),
        expires_at_ms.as_str(),
        "ephemeral_process_local",
    ])
}

pub fn visual_provider_invocation_digest(invocation: &VisualProviderInvocation) -> String {
    digest_parts([
        invocation.request.request_digest.as_str(),
        invocation.payload.envelope.envelope_digest.as_str(),
        invocation.payload.envelope.byte_digest.as_str(),
    ])
}

fn valid_invocation_policy(policy: &VisualInvocationPolicy) -> bool {
    policy.max_payload_bytes > 0
        && policy.max_request_bytes > 0
        && policy.max_response_bytes > 0
        && policy.max_width > 0
        && policy.max_height > 0
        && !policy.allowed_media_types.is_empty()
        && policy
            .allowed_media_types
            .iter()
            .all(|media_type| !media_type.trim().is_empty())
        && policy
            .allowed_media_types
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            == policy.allowed_media_types.len()
}

fn valid_payload(
    policy: &VisualInvocationPolicy,
    round_policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    payload: &PreparedVisualPayload,
    now_ms: u64,
) -> bool {
    let envelope = &payload.envelope;
    !payload.bytes.is_empty()
        && payload.bytes.len() as u64 <= policy.max_payload_bytes
        && envelope.byte_length == payload.bytes.len() as u64
        && envelope.byte_digest == visual_payload_digest(&payload.bytes)
        && !envelope.artifact_id.trim().is_empty()
        && envelope.envelope_digest == visual_artifact_envelope_digest(envelope)
        && policy.allowed_media_types.contains(&envelope.media_type)
        && envelope.width > 0
        && envelope.width <= policy.max_width
        && envelope.height > 0
        && envelope.height <= policy.max_height
        && envelope.evidence_digest == evidence.evidence_digest
        && envelope.frame_digest == evidence.frame_digest
        && envelope.context_digest == evidence.context_digest
        && envelope.geometry_digest == evidence.geometry_digest
        && valid_digest(&envelope.redaction_policy_digest)
        && valid_digest(&envelope.redaction_receipt_digest)
        && envelope.provider_capability == evidence.provider_capability
        && envelope.provider_capability == round_policy.provider_capability
        && envelope.prepared_at_ms >= evidence.observed_at_ms
        && envelope.prepared_at_ms <= now_ms
        && envelope.expires_at_ms == evidence.expires_at_ms.min(round_policy.deadline_at_ms)
        && now_ms < envelope.expires_at_ms
        && envelope.retention == VisualArtifactRetention::EphemeralProcessLocal
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_parts<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format_digest(hasher.finalize().as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    format_digest(Sha256::digest(bytes).as_slice())
}

fn format_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
