//! Effect-free admission for candidate-bound desktop intents.
//!
//! The contract validates physical-pixel geometry and current controller
//! authority before any caller may construct or emit an input event. It owns
//! no challenge vocabulary, route claim, capture, input, persistence, or
//! runtime operation.

use crate::{ControllerAuthority, DesktopBinding, PixelBounds, PixelPoint, COORDINATE_SPACE};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_PLANNED_STEPS: u8 = 16;
const MAX_PLANNED_EVENTS: u8 = 64;

/// One opaque candidate with geometry in physical desktop pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopCandidateGeometry {
    pub candidate_id: String,
    pub bounds: PixelBounds,
    pub center: PixelPoint,
}

/// A fresh, digest-bound candidate set observed on one desktop surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopCandidateObservation {
    pub binding: DesktopBinding,
    pub evidence_digest: String,
    pub frame_digest: String,
    pub context_digest: String,
    pub geometry_digest: String,
    pub candidate_set_digest: String,
    pub candidates: Vec<DesktopCandidateGeometry>,
    pub captured_at_ms: u64,
    pub expires_at_ms: u64,
    pub observation_digest: String,
}

/// A bounded request to admit selected candidates from one exact observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopCandidateIntent {
    pub source_intent_digest: String,
    pub observation_digest: String,
    pub evidence_digest: String,
    pub frame_digest: String,
    pub context_digest: String,
    pub geometry_digest: String,
    pub candidate_set_digest: String,
    pub selected_candidate_ids: Vec<String>,
    pub controller_lease_id: String,
    pub controller_viewer_id: String,
    pub provider_capability_digest: String,
    pub planned_steps: u8,
    pub planned_pointer_events: u8,
    pub planned_key_events: u8,
    pub expires_at_ms: u64,
    pub intent_digest: String,
}

/// The deterministic result of admission. Constructing it emits no input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopCandidateEffectPermit {
    pub permit_digest: String,
    pub source_intent_digest: String,
    pub evidence_digest: String,
    pub candidate_set_digest: String,
    pub selected_candidates: Vec<DesktopCandidateGeometry>,
    pub binding: DesktopBinding,
    pub controller_authority_digest: String,
    pub provider_capability_digest: String,
    pub planned_steps: u8,
    pub planned_pointer_events: u8,
    pub planned_key_events: u8,
    pub expires_at_ms: u64,
}

impl DesktopCandidateEffectPermit {
    /// Reports the contract's effect-free construction invariant.
    pub fn emitted_effects(&self) -> bool {
        false
    }
}

/// Typed fail-closed outcomes from candidate-intent admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopCandidateAdmissionError {
    InvalidObservation,
    InvalidIntent,
    CandidateMismatch,
    AuthorityMismatch,
    Stale,
}

/// Validates one candidate intent against exact observation and authority state.
pub fn admit_desktop_candidate_intent(
    intent: &DesktopCandidateIntent,
    observation: &DesktopCandidateObservation,
    authority: &ControllerAuthority,
    now_ms: u64,
) -> Result<DesktopCandidateEffectPermit, DesktopCandidateAdmissionError> {
    if !valid_observation(observation) {
        return Err(DesktopCandidateAdmissionError::InvalidObservation);
    }
    if !valid_intent(intent) {
        return Err(DesktopCandidateAdmissionError::InvalidIntent);
    }
    if now_ms < observation.captured_at_ms
        || now_ms >= observation.expires_at_ms
        || intent.expires_at_ms != observation.expires_at_ms
    {
        return Err(DesktopCandidateAdmissionError::Stale);
    }
    if intent.observation_digest != observation.observation_digest
        || intent.evidence_digest != observation.evidence_digest
        || intent.frame_digest != observation.frame_digest
        || intent.context_digest != observation.context_digest
        || intent.geometry_digest != observation.geometry_digest
        || intent.candidate_set_digest != observation.candidate_set_digest
    {
        return Err(DesktopCandidateAdmissionError::CandidateMismatch);
    }

    let candidate_positions = observation
        .candidates
        .iter()
        .enumerate()
        .map(|(position, candidate)| (candidate.candidate_id.as_str(), position))
        .collect::<BTreeMap<_, _>>();
    let mut last_position = None;
    for candidate_id in &intent.selected_candidate_ids {
        let position = candidate_positions
            .get(candidate_id.as_str())
            .copied()
            .ok_or(DesktopCandidateAdmissionError::CandidateMismatch)?;
        if last_position.is_some_and(|last| position <= last) {
            return Err(DesktopCandidateAdmissionError::CandidateMismatch);
        }
        last_position = Some(position);
    }
    let selected_candidates = intent
        .selected_candidate_ids
        .iter()
        .map(|candidate_id| {
            observation.candidates[candidate_positions[candidate_id.as_str()]].clone()
        })
        .collect::<Vec<_>>();

    let controller_authority_digest =
        validate_authority(&observation.binding, intent, authority, now_ms)?;
    let mut permit = DesktopCandidateEffectPermit {
        permit_digest: String::new(),
        source_intent_digest: intent.source_intent_digest.clone(),
        evidence_digest: intent.evidence_digest.clone(),
        candidate_set_digest: intent.candidate_set_digest.clone(),
        selected_candidates,
        binding: observation.binding.clone(),
        controller_authority_digest,
        provider_capability_digest: intent.provider_capability_digest.clone(),
        planned_steps: intent.planned_steps,
        planned_pointer_events: intent.planned_pointer_events,
        planned_key_events: intent.planned_key_events,
        expires_at_ms: intent.expires_at_ms.min(authority.lease_expires_at_ms),
    };
    permit.permit_digest = desktop_candidate_effect_permit_digest(&permit);
    Ok(permit)
}

/// Digests candidate order, identities, bounds, and centers.
pub fn desktop_candidate_set_digest(candidates: &[DesktopCandidateGeometry]) -> String {
    let mut parts = Vec::with_capacity(candidates.len() * 7);
    for candidate in candidates {
        parts.push(candidate.candidate_id.clone());
        parts.push(candidate.bounds.x.to_string());
        parts.push(candidate.bounds.y.to_string());
        parts.push(candidate.bounds.width.to_string());
        parts.push(candidate.bounds.height.to_string());
        parts.push(candidate.center.x.to_string());
        parts.push(candidate.center.y.to_string());
    }
    digest_owned_parts(&parts)
}

/// Digests every observation field except the digest field itself.
pub fn desktop_candidate_observation_digest(observation: &DesktopCandidateObservation) -> String {
    let width = observation.binding.width.to_string();
    let height = observation.binding.height.to_string();
    let scale_millis = observation.binding.scale_millis.to_string();
    let captured_at_ms = observation.captured_at_ms.to_string();
    let expires_at_ms = observation.expires_at_ms.to_string();
    digest_parts([
        observation.binding.browser_id.as_str(),
        observation.binding.session_name.as_str(),
        observation.binding.profile_id.as_deref().unwrap_or(""),
        observation.binding.display_allocation_id.as_str(),
        observation.binding.stream_id.as_str(),
        observation.binding.route_id.as_str(),
        width.as_str(),
        height.as_str(),
        scale_millis.as_str(),
        observation.binding.coordinate_space.as_str(),
        observation.binding.geometry_epoch.as_str(),
        observation.evidence_digest.as_str(),
        observation.frame_digest.as_str(),
        observation.context_digest.as_str(),
        observation.geometry_digest.as_str(),
        observation.candidate_set_digest.as_str(),
        captured_at_ms.as_str(),
        expires_at_ms.as_str(),
    ])
}

/// Digests every intent field except the digest field itself.
pub fn desktop_candidate_intent_digest(intent: &DesktopCandidateIntent) -> String {
    let selected_digest = digest_owned_parts(&intent.selected_candidate_ids);
    let planned_steps = intent.planned_steps.to_string();
    let planned_pointer_events = intent.planned_pointer_events.to_string();
    let planned_key_events = intent.planned_key_events.to_string();
    let expires_at_ms = intent.expires_at_ms.to_string();
    digest_parts([
        intent.source_intent_digest.as_str(),
        intent.observation_digest.as_str(),
        intent.evidence_digest.as_str(),
        intent.frame_digest.as_str(),
        intent.context_digest.as_str(),
        intent.geometry_digest.as_str(),
        intent.candidate_set_digest.as_str(),
        selected_digest.as_str(),
        intent.controller_lease_id.as_str(),
        intent.controller_viewer_id.as_str(),
        intent.provider_capability_digest.as_str(),
        planned_steps.as_str(),
        planned_pointer_events.as_str(),
        planned_key_events.as_str(),
        expires_at_ms.as_str(),
    ])
}

/// Digests every permit field except the digest field itself.
pub fn desktop_candidate_effect_permit_digest(permit: &DesktopCandidateEffectPermit) -> String {
    let selected_digest = desktop_candidate_set_digest(&permit.selected_candidates);
    let width = permit.binding.width.to_string();
    let height = permit.binding.height.to_string();
    let scale_millis = permit.binding.scale_millis.to_string();
    let planned_steps = permit.planned_steps.to_string();
    let planned_pointer_events = permit.planned_pointer_events.to_string();
    let planned_key_events = permit.planned_key_events.to_string();
    let expires_at_ms = permit.expires_at_ms.to_string();
    digest_parts([
        permit.source_intent_digest.as_str(),
        permit.evidence_digest.as_str(),
        permit.candidate_set_digest.as_str(),
        selected_digest.as_str(),
        permit.binding.browser_id.as_str(),
        permit.binding.session_name.as_str(),
        permit.binding.profile_id.as_deref().unwrap_or(""),
        permit.binding.display_allocation_id.as_str(),
        permit.binding.stream_id.as_str(),
        permit.binding.route_id.as_str(),
        width.as_str(),
        height.as_str(),
        scale_millis.as_str(),
        permit.binding.coordinate_space.as_str(),
        permit.binding.geometry_epoch.as_str(),
        permit.controller_authority_digest.as_str(),
        permit.provider_capability_digest.as_str(),
        planned_steps.as_str(),
        planned_pointer_events.as_str(),
        planned_key_events.as_str(),
        expires_at_ms.as_str(),
    ])
}

fn valid_observation(observation: &DesktopCandidateObservation) -> bool {
    let binding = &observation.binding;
    valid_binding(binding)
        && valid_digest(&observation.evidence_digest)
        && valid_digest(&observation.frame_digest)
        && valid_digest(&observation.context_digest)
        && valid_digest(&observation.geometry_digest)
        && !observation.candidates.is_empty()
        && unique_nonempty(
            observation
                .candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str()),
        )
        && observation
            .candidates
            .iter()
            .all(|candidate| valid_candidate(binding, candidate))
        && observation.candidate_set_digest == desktop_candidate_set_digest(&observation.candidates)
        && observation.captured_at_ms < observation.expires_at_ms
        && observation.observation_digest == desktop_candidate_observation_digest(observation)
}

fn valid_binding(binding: &DesktopBinding) -> bool {
    !binding.browser_id.trim().is_empty()
        && !binding.session_name.trim().is_empty()
        && binding
            .profile_id
            .as_deref()
            .is_none_or(|profile_id| !profile_id.trim().is_empty())
        && !binding.display_allocation_id.trim().is_empty()
        && !binding.stream_id.trim().is_empty()
        && !binding.route_id.trim().is_empty()
        && binding.width > 0
        && binding.height > 0
        && binding.scale_millis > 0
        && binding.coordinate_space == COORDINATE_SPACE
        && !binding.geometry_epoch.trim().is_empty()
}

fn valid_intent(intent: &DesktopCandidateIntent) -> bool {
    let planned_events = intent
        .planned_pointer_events
        .checked_add(intent.planned_key_events);
    valid_digest(&intent.source_intent_digest)
        && valid_digest(&intent.observation_digest)
        && valid_digest(&intent.evidence_digest)
        && valid_digest(&intent.frame_digest)
        && valid_digest(&intent.context_digest)
        && valid_digest(&intent.geometry_digest)
        && valid_digest(&intent.candidate_set_digest)
        && unique_nonempty(intent.selected_candidate_ids.iter().map(String::as_str))
        && !intent.controller_lease_id.trim().is_empty()
        && !intent.controller_viewer_id.trim().is_empty()
        && valid_digest(&intent.provider_capability_digest)
        && intent.planned_steps > 0
        && intent.planned_steps <= MAX_PLANNED_STEPS
        && usize::from(intent.planned_steps) == intent.selected_candidate_ids.len()
        && intent.planned_pointer_events <= MAX_PLANNED_EVENTS
        && intent.planned_key_events <= MAX_PLANNED_EVENTS
        && planned_events
            .is_some_and(|count| count >= intent.planned_steps && count <= MAX_PLANNED_EVENTS)
        && intent.expires_at_ms > 0
        && intent.intent_digest == desktop_candidate_intent_digest(intent)
}

fn valid_candidate(binding: &DesktopBinding, candidate: &DesktopCandidateGeometry) -> bool {
    if candidate.bounds.width == 0
        || candidate.bounds.height == 0
        || candidate.bounds.x < 0
        || candidate.bounds.y < 0
    {
        return false;
    }
    let right = candidate
        .bounds
        .x
        .checked_add(i64::from(candidate.bounds.width));
    let bottom = candidate
        .bounds
        .y
        .checked_add(i64::from(candidate.bounds.height));
    right.is_some_and(|right| right <= i64::from(binding.width))
        && bottom.is_some_and(|bottom| bottom <= i64::from(binding.height))
        && candidate.bounds.contains(candidate.center)
}

fn validate_authority(
    binding: &DesktopBinding,
    intent: &DesktopCandidateIntent,
    authority: &ControllerAuthority,
    now_ms: u64,
) -> Result<String, DesktopCandidateAdmissionError> {
    let provider = authority.route_machine_input.as_deref();
    if authority.browser_id != binding.browser_id
        || authority.display_allocation_id != binding.display_allocation_id
        || authority.stream_id != binding.stream_id
        || authority.route_id != binding.route_id
        || authority.route_controller_lease_id != intent.controller_lease_id
        || authority.stream_controller_lease_id != intent.controller_lease_id
        || authority.lease_id != intent.controller_lease_id
        || authority.lease_record_id != intent.controller_lease_id
        || authority.lease_route_id != binding.route_id
        || authority.lease_browser_id != binding.browser_id
        || authority.lease_viewer_id != intent.controller_viewer_id
        || authority.lease_role != "controller"
        || authority.lease_state != "controlling"
        || authority.lease_updated_at.trim().is_empty()
        || !authority.route_contains_lease
        || !authority.stream_contains_lease
        || !authority.route_writable
        || !authority.stream_writable
        || provider.is_none()
        || provider.is_some_and(|provider| provider.trim().is_empty())
        || provider == Some("manual_attached_desktop")
        || provider != authority.stream_machine_input.as_deref()
        || authority.controller_epoch == 0
        || authority.route_controller_epoch != authority.controller_epoch
        || authority.stream_controller_epoch != authority.controller_epoch
        || authority.lease_expires_at_ms <= now_ms
    {
        return Err(DesktopCandidateAdmissionError::AuthorityMismatch);
    }
    Ok(desktop_controller_authority_digest(authority))
}

fn desktop_controller_authority_digest(authority: &ControllerAuthority) -> String {
    let lease_expires_at_ms = authority.lease_expires_at_ms.to_string();
    let controller_epoch = authority.controller_epoch.to_string();
    let route_controller_epoch = authority.route_controller_epoch.to_string();
    let stream_controller_epoch = authority.stream_controller_epoch.to_string();
    digest_parts([
        authority.browser_id.as_str(),
        authority.display_allocation_id.as_str(),
        authority.stream_id.as_str(),
        authority.route_id.as_str(),
        authority.route_controller_lease_id.as_str(),
        authority.stream_controller_lease_id.as_str(),
        authority.lease_id.as_str(),
        authority.lease_record_id.as_str(),
        authority.lease_route_id.as_str(),
        authority.lease_browser_id.as_str(),
        authority.lease_viewer_id.as_str(),
        authority.lease_role.as_str(),
        authority.lease_state.as_str(),
        authority.lease_updated_at.as_str(),
        lease_expires_at_ms.as_str(),
        controller_epoch.as_str(),
        route_controller_epoch.as_str(),
        stream_controller_epoch.as_str(),
        if authority.route_contains_lease {
            "1"
        } else {
            "0"
        },
        if authority.stream_contains_lease {
            "1"
        } else {
            "0"
        },
        if authority.route_writable { "1" } else { "0" },
        if authority.stream_writable { "1" } else { "0" },
        authority.route_machine_input.as_deref().unwrap_or(""),
        authority.stream_machine_input.as_deref().unwrap_or(""),
    ])
}

fn unique_nonempty<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let values = values.into_iter().collect::<Vec<_>>();
    !values.is_empty()
        && values.iter().all(|value| !value.trim().is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Validates the canonical structure of a permit crossing into a pure next stage.
pub fn validate_desktop_candidate_effect_permit(
    permit: &DesktopCandidateEffectPermit,
) -> Result<(), DesktopCandidateAdmissionError> {
    let selection_ids = permit
        .selected_candidates
        .iter()
        .map(|candidate| candidate.candidate_id.as_str());
    let planned_events = permit
        .planned_pointer_events
        .checked_add(permit.planned_key_events);
    if !valid_digest(&permit.source_intent_digest)
        || !valid_digest(&permit.evidence_digest)
        || !valid_digest(&permit.candidate_set_digest)
        || !valid_binding(&permit.binding)
        || permit.selected_candidates.is_empty()
        || !unique_nonempty(selection_ids)
        || permit
            .selected_candidates
            .iter()
            .any(|candidate| !valid_candidate(&permit.binding, candidate))
        || !valid_digest(&permit.controller_authority_digest)
        || !valid_digest(&permit.provider_capability_digest)
        || permit.planned_steps == 0
        || permit.planned_steps > MAX_PLANNED_STEPS
        || usize::from(permit.planned_steps) != permit.selected_candidates.len()
        || permit.planned_pointer_events > MAX_PLANNED_EVENTS
        || permit.planned_key_events > MAX_PLANNED_EVENTS
        || planned_events
            .is_none_or(|count| count < permit.planned_steps || count > MAX_PLANNED_EVENTS)
        || permit.expires_at_ms == 0
        || permit.permit_digest != desktop_candidate_effect_permit_digest(permit)
    {
        return Err(DesktopCandidateAdmissionError::InvalidIntent);
    }
    Ok(())
}

fn digest_owned_parts(parts: &[String]) -> String {
    digest_parts(parts.iter().map(String::as_str))
}

fn digest_parts<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
