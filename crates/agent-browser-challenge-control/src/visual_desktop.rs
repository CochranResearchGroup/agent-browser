//! Effect-free composition of authorized visual intents and desktop admission.
//!
//! This module maps already-authorized challenge state into the shared desktop
//! contract. It performs no capture, transport, route claim, input, storage,
//! browser, or runtime operation.

use crate::{
    validate_visual_round_intent, VisualRoundError, VisualRoundIntent, VisualRoundPolicy,
    VisualRoundSnapshot,
};
use agent_browser_desktop_services::{
    admit_desktop_candidate_intent, desktop_candidate_intent_digest, ControllerAuthority,
    DesktopCandidateAdmissionError, DesktopCandidateEffectPermit, DesktopCandidateIntent,
    DesktopCandidateObservation,
};

/// Exact controller identities carried into desktop authority admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualDesktopControllerBinding {
    pub controller_lease_id: String,
    pub controller_viewer_id: String,
}

/// Typed fail-closed outcomes from visual-to-desktop composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualDesktopAdmissionError {
    VisualIntent(VisualRoundError),
    StaleVisualIntent,
    ObservationMismatch,
    Desktop(DesktopCandidateAdmissionError),
}

/// Maps one current visual intent into one effect-free desktop permit.
pub fn admit_visual_desktop_candidate_intent(
    policy: &VisualRoundPolicy,
    snapshot: &VisualRoundSnapshot,
    visual_intent: &VisualRoundIntent,
    observation: &DesktopCandidateObservation,
    authority: &ControllerAuthority,
    controller: &VisualDesktopControllerBinding,
    now_ms: u64,
) -> Result<DesktopCandidateEffectPermit, VisualDesktopAdmissionError> {
    validate_visual_round_intent(policy, snapshot, visual_intent)
        .map_err(VisualDesktopAdmissionError::VisualIntent)?;
    let evidence = snapshot
        .active_evidence
        .as_ref()
        .ok_or(VisualDesktopAdmissionError::ObservationMismatch)?;
    let observation_candidate_ids = observation
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.as_str())
        .collect::<Vec<_>>();
    let effective_expires_at_ms = evidence.expires_at_ms.min(policy.deadline_at_ms);
    if now_ms < evidence.observed_at_ms || now_ms >= effective_expires_at_ms {
        return Err(VisualDesktopAdmissionError::StaleVisualIntent);
    }
    if evidence
        .candidate_ids
        .iter()
        .map(String::as_str)
        .ne(observation_candidate_ids)
        || evidence.evidence_digest != observation.evidence_digest
        || evidence.frame_digest != observation.frame_digest
        || evidence.context_digest != observation.context_digest
        || evidence.geometry_digest != observation.geometry_digest
        || evidence.observed_at_ms != observation.captured_at_ms
        || effective_expires_at_ms != observation.expires_at_ms
    {
        return Err(VisualDesktopAdmissionError::ObservationMismatch);
    }

    let mut desktop_intent = DesktopCandidateIntent {
        source_intent_digest: visual_intent.intent_digest.clone(),
        observation_digest: observation.observation_digest.clone(),
        evidence_digest: visual_intent.evidence_digest.clone(),
        frame_digest: evidence.frame_digest.clone(),
        context_digest: evidence.context_digest.clone(),
        geometry_digest: evidence.geometry_digest.clone(),
        candidate_set_digest: observation.candidate_set_digest.clone(),
        selected_candidate_ids: visual_intent.selected_candidate_ids.clone(),
        controller_lease_id: controller.controller_lease_id.clone(),
        controller_viewer_id: controller.controller_viewer_id.clone(),
        provider_capability_digest: visual_intent.provider_capability.capability_digest.clone(),
        planned_steps: visual_intent.planned_steps,
        planned_pointer_events: visual_intent.planned_pointer_events,
        planned_key_events: visual_intent.planned_key_events,
        expires_at_ms: observation.expires_at_ms,
        intent_digest: String::new(),
    };
    desktop_intent.intent_digest = desktop_candidate_intent_digest(&desktop_intent);
    admit_desktop_candidate_intent(&desktop_intent, observation, authority, now_ms)
        .map_err(VisualDesktopAdmissionError::Desktop)
}
