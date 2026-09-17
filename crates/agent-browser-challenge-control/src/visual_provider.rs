//! Provider-neutral visual reasoning protocol.
//!
//! This module binds prepared visual evidence to one candidate-only provider
//! request and adjudicates a typed response. It performs no I/O and grants no
//! browser, image-capture, coordinate, desktop-input, retry, or runtime
//! authority.

use crate::{
    visual_candidate_set_digest,
    visual_round::{digest_parts, nonempty_unique, valid_digest, valid_evidence, validate_policy},
    VisualProviderCapability, VisualRoundEvidence, VisualRoundInterventionReason,
    VisualRoundPolicy, VisualRoundSelection,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedVisualArtifact {
    pub artifact_id: String,
    pub artifact_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualProviderRequest {
    pub task_id: String,
    pub attempt_id: String,
    pub round_index: u8,
    pub round_id: String,
    pub policy_digest: String,
    pub profile_digest: String,
    pub evidence_digest: String,
    pub frame_digest: String,
    pub context_digest: String,
    pub geometry_digest: String,
    pub candidate_set_digest: String,
    pub candidate_ids: Vec<String>,
    pub prepared_artifact: PreparedVisualArtifact,
    pub provider_capability: VisualProviderCapability,
    pub requested_at_ms: u64,
    pub expires_at_ms: u64,
    pub request_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VisualProviderDisposition {
    Selected { selected_candidate_ids: Vec<String> },
    Ambiguous,
    Unsupported,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualProviderResponse {
    pub request_digest: String,
    pub evidence_digest: String,
    pub candidate_set_digest: String,
    pub provider_capability: VisualProviderCapability,
    pub disposition: VisualProviderDisposition,
    pub produced_at_ms: u64,
    pub expires_at_ms: u64,
    pub response_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualProviderDecision {
    Selection(VisualRoundSelection),
    Intervention(VisualRoundInterventionReason),
}

impl VisualProviderDecision {
    pub fn emitted_effects(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualProviderError {
    InvalidPolicy,
    InvalidEvidence,
    InvalidArtifact,
}

pub fn prepare_visual_provider_request(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    prepared_artifact: PreparedVisualArtifact,
    now_ms: u64,
) -> Result<VisualProviderRequest, VisualProviderError> {
    if !valid_policy_identity(policy) {
        return Err(VisualProviderError::InvalidPolicy);
    }
    if !valid_evidence_for_policy(policy, evidence)
        || now_ms < evidence.observed_at_ms
        || now_ms >= evidence.expires_at_ms
        || now_ms >= policy.deadline_at_ms
    {
        return Err(VisualProviderError::InvalidEvidence);
    }
    if prepared_artifact.artifact_id.trim().is_empty()
        || !valid_digest(&prepared_artifact.artifact_digest)
    {
        return Err(VisualProviderError::InvalidArtifact);
    }

    let mut request = VisualProviderRequest {
        task_id: evidence.task_id.clone(),
        attempt_id: evidence.attempt_id.clone(),
        round_index: evidence.round_index,
        round_id: evidence.round_id.clone(),
        policy_digest: evidence.policy_digest.clone(),
        profile_digest: evidence.profile_digest.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        frame_digest: evidence.frame_digest.clone(),
        context_digest: evidence.context_digest.clone(),
        geometry_digest: evidence.geometry_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        candidate_ids: evidence.candidate_ids.clone(),
        prepared_artifact,
        provider_capability: evidence.provider_capability.clone(),
        requested_at_ms: now_ms,
        expires_at_ms: evidence.expires_at_ms.min(policy.deadline_at_ms),
        request_digest: String::new(),
    };
    request.request_digest = visual_provider_request_digest(&request);
    Ok(request)
}

pub fn adjudicate_visual_provider_response(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    request: &VisualProviderRequest,
    response: &VisualProviderResponse,
    now_ms: u64,
) -> Result<VisualProviderDecision, VisualProviderError> {
    if !valid_policy_identity(policy) {
        return Err(VisualProviderError::InvalidPolicy);
    }
    if !valid_evidence_for_policy(policy, evidence) {
        return Err(VisualProviderError::InvalidEvidence);
    }
    if !request_matches(policy, evidence, request) {
        return Ok(VisualProviderDecision::Intervention(
            VisualRoundInterventionReason::ProviderRequestMismatch,
        ));
    }
    if now_ms < request.requested_at_ms
        || now_ms >= request.expires_at_ms
        || now_ms >= policy.deadline_at_ms
    {
        return Ok(VisualProviderDecision::Intervention(
            VisualRoundInterventionReason::StaleEvidence,
        ));
    }
    if response.request_digest != request.request_digest
        || response.evidence_digest != evidence.evidence_digest
        || response.candidate_set_digest != evidence.candidate_set_digest
    {
        return Ok(VisualProviderDecision::Intervention(
            VisualRoundInterventionReason::ProviderResponseMismatch,
        ));
    }
    if response.provider_capability != policy.provider_capability {
        return Ok(VisualProviderDecision::Intervention(
            VisualRoundInterventionReason::CapabilityMismatch,
        ));
    }
    if response.response_digest != visual_provider_response_digest(response) {
        return Ok(VisualProviderDecision::Intervention(
            VisualRoundInterventionReason::ProviderResponseMismatch,
        ));
    }
    if response.produced_at_ms < request.requested_at_ms
        || response.produced_at_ms > now_ms
        || response.produced_at_ms >= response.expires_at_ms
        || response.expires_at_ms > request.expires_at_ms
        || now_ms >= response.expires_at_ms
    {
        return Ok(VisualProviderDecision::Intervention(
            VisualRoundInterventionReason::StaleEvidence,
        ));
    }

    let decision = match &response.disposition {
        VisualProviderDisposition::Selected {
            selected_candidate_ids,
        } => {
            let Ok(selection_count) = u8::try_from(selected_candidate_ids.len()) else {
                return Ok(VisualProviderDecision::Intervention(
                    VisualRoundInterventionReason::RoundBudgetExceeded,
                ));
            };
            if !nonempty_unique(selected_candidate_ids)
                || selected_candidate_ids
                    .iter()
                    .any(|candidate| !evidence.candidate_ids.contains(candidate))
            {
                VisualProviderDecision::Intervention(
                    VisualRoundInterventionReason::CandidateMismatch,
                )
            } else if selection_count > policy.max_selections_per_round
                || selection_count > policy.max_pointer_events_per_round
            {
                VisualProviderDecision::Intervention(
                    VisualRoundInterventionReason::RoundBudgetExceeded,
                )
            } else {
                VisualProviderDecision::Selection(VisualRoundSelection {
                    round_id: evidence.round_id.clone(),
                    evidence_digest: evidence.evidence_digest.clone(),
                    candidate_set_digest: evidence.candidate_set_digest.clone(),
                    selected_candidate_ids: selected_candidate_ids.clone(),
                    planned_steps: 1,
                    planned_pointer_events: selection_count,
                    planned_key_events: 0,
                    provider_capability: response.provider_capability.clone(),
                })
            }
        }
        VisualProviderDisposition::Ambiguous => {
            VisualProviderDecision::Intervention(VisualRoundInterventionReason::Ambiguous)
        }
        VisualProviderDisposition::Unsupported => {
            VisualProviderDecision::Intervention(VisualRoundInterventionReason::Unsupported)
        }
        VisualProviderDisposition::Inconclusive => {
            VisualProviderDecision::Intervention(VisualRoundInterventionReason::Inconclusive)
        }
    };
    Ok(decision)
}

pub fn visual_provider_request_digest(request: &VisualProviderRequest) -> String {
    let round_index = request.round_index.to_string();
    let requested_at_ms = request.requested_at_ms.to_string();
    let expires_at_ms = request.expires_at_ms.to_string();
    let candidates_digest = visual_candidate_set_digest(&request.candidate_ids);
    digest_parts([
        request.task_id.as_str(),
        request.attempt_id.as_str(),
        round_index.as_str(),
        request.round_id.as_str(),
        request.policy_digest.as_str(),
        request.profile_digest.as_str(),
        request.evidence_digest.as_str(),
        request.frame_digest.as_str(),
        request.context_digest.as_str(),
        request.geometry_digest.as_str(),
        request.candidate_set_digest.as_str(),
        candidates_digest.as_str(),
        request.prepared_artifact.artifact_id.as_str(),
        request.prepared_artifact.artifact_digest.as_str(),
        request.provider_capability.capability_id.as_str(),
        request.provider_capability.capability_version.as_str(),
        request.provider_capability.capability_digest.as_str(),
        requested_at_ms.as_str(),
        expires_at_ms.as_str(),
    ])
}

pub fn visual_provider_response_digest(response: &VisualProviderResponse) -> String {
    let produced_at_ms = response.produced_at_ms.to_string();
    let expires_at_ms = response.expires_at_ms.to_string();
    let (disposition, selected_digest) = match &response.disposition {
        VisualProviderDisposition::Selected {
            selected_candidate_ids,
        } => (
            "selected",
            visual_candidate_set_digest(selected_candidate_ids),
        ),
        VisualProviderDisposition::Ambiguous => ("ambiguous", String::new()),
        VisualProviderDisposition::Unsupported => ("unsupported", String::new()),
        VisualProviderDisposition::Inconclusive => ("inconclusive", String::new()),
    };
    digest_parts([
        response.request_digest.as_str(),
        response.evidence_digest.as_str(),
        response.candidate_set_digest.as_str(),
        response.provider_capability.capability_id.as_str(),
        response.provider_capability.capability_version.as_str(),
        response.provider_capability.capability_digest.as_str(),
        disposition,
        selected_digest.as_str(),
        produced_at_ms.as_str(),
        expires_at_ms.as_str(),
    ])
}

fn request_matches(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    request: &VisualProviderRequest,
) -> bool {
    request.task_id == evidence.task_id
        && request.attempt_id == evidence.attempt_id
        && request.round_index == evidence.round_index
        && request.round_id == evidence.round_id
        && request.policy_digest == evidence.policy_digest
        && request.profile_digest == evidence.profile_digest
        && request.evidence_digest == evidence.evidence_digest
        && request.frame_digest == evidence.frame_digest
        && request.context_digest == evidence.context_digest
        && request.geometry_digest == evidence.geometry_digest
        && request.candidate_set_digest == evidence.candidate_set_digest
        && request.candidate_ids == evidence.candidate_ids
        && !request.prepared_artifact.artifact_id.trim().is_empty()
        && valid_digest(&request.prepared_artifact.artifact_digest)
        && request.provider_capability == policy.provider_capability
        && request.requested_at_ms >= evidence.observed_at_ms
        && request.expires_at_ms == evidence.expires_at_ms.min(policy.deadline_at_ms)
        && request.requested_at_ms < request.expires_at_ms
        && request.request_digest == visual_provider_request_digest(request)
}

fn valid_policy_identity(policy: &VisualRoundPolicy) -> bool {
    validate_policy(policy).is_ok()
}

fn valid_evidence_for_policy(policy: &VisualRoundPolicy, evidence: &VisualRoundEvidence) -> bool {
    valid_evidence(evidence)
        && evidence.round_index > 0
        && evidence.policy_digest == policy.policy_digest
        && evidence.profile_digest == policy.profile_digest
        && evidence.provider_capability == policy.provider_capability
}
