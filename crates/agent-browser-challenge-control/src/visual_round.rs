use crate::DeliveryState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualProviderCapability {
    pub capability_id: String,
    pub capability_version: String,
    pub capability_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualRoundPolicy {
    pub policy_digest: String,
    pub profile_digest: String,
    pub provider_capability: VisualProviderCapability,
    pub max_rounds: u8,
    pub max_selections_per_round: u8,
    pub max_steps_per_round: u8,
    pub max_pointer_events_per_round: u8,
    pub max_key_events_per_round: u8,
    pub max_total_selections: u8,
    pub max_total_steps: u8,
    pub max_total_pointer_events: u8,
    pub max_total_key_events: u8,
    pub deadline_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRoundEvidence {
    pub task_id: String,
    pub attempt_id: String,
    pub profile_digest: String,
    pub policy_digest: String,
    pub round_index: u8,
    pub round_id: String,
    pub evidence_digest: String,
    pub frame_digest: String,
    pub context_digest: String,
    pub geometry_digest: String,
    pub candidate_set_digest: String,
    pub candidate_ids: Vec<String>,
    pub observed_at_ms: u64,
    pub expires_at_ms: u64,
    pub provider_capability: VisualProviderCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRoundSelection {
    pub round_id: String,
    pub evidence_digest: String,
    pub candidate_set_digest: String,
    pub selected_candidate_ids: Vec<String>,
    pub planned_steps: u8,
    pub planned_pointer_events: u8,
    pub planned_key_events: u8,
    pub provider_capability: VisualProviderCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRoundEffectReceipt {
    pub round_id: String,
    pub evidence_digest: String,
    pub intent_digest: String,
    pub receipt_ref: String,
    pub delivery: DeliveryState,
    pub steps: u8,
    pub pointer_events: u8,
    pub key_events: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualAfterStateOutcome {
    Passed,
    Denied,
    NextRound,
    Ambiguous,
    Unsupported,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRoundAfterState {
    pub round_id: String,
    pub evidence_digest: String,
    pub frame_digest: String,
    pub context_digest: String,
    pub geometry_digest: String,
    pub candidate_set_digest: String,
    pub observed_at_ms: u64,
    pub expires_at_ms: u64,
    pub outcome: VisualAfterStateOutcome,
    pub terminal_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRoundPhase {
    AwaitingEvidence,
    AwaitingSelection,
    IntentAuthorized,
    AwaitingAfterState,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRoundTerminalOutcome {
    Passed,
    Denied,
    InterventionRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRoundInterventionReason {
    Ambiguous,
    Unsupported,
    Inconclusive,
    PartialEffect,
    UncertainEffect,
    InvalidEvidence,
    StaleEvidence,
    EvidenceChanged,
    CandidateMismatch,
    CapabilityMismatch,
    RoundOrder,
    RoundBudgetExceeded,
    CumulativeBudgetExceeded,
    UnexpectedRound,
    EffectMismatch,
    ProviderRequestMismatch,
    ProviderResponseMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRoundSnapshot {
    pub task_id: String,
    pub attempt_id: String,
    pub phase: VisualRoundPhase,
    pub next_round_index: u8,
    pub completed_rounds: u8,
    pub total_selections: u8,
    pub total_steps: u8,
    pub total_pointer_events: u8,
    pub total_key_events: u8,
    pub completed_round_ids: Vec<String>,
    pub expected_next_frame_digest: Option<String>,
    pub expected_next_context_digest: Option<String>,
    pub expected_next_geometry_digest: Option<String>,
    pub expected_next_candidate_set_digest: Option<String>,
    pub active_evidence: Option<VisualRoundEvidence>,
    pub active_selection: Option<VisualRoundSelection>,
    pub active_intent_digest: Option<String>,
    pub last_effect_receipt_ref: Option<String>,
    pub terminal_outcome: Option<VisualRoundTerminalOutcome>,
    pub intervention: Option<VisualRoundInterventionReason>,
    pub terminal_receipt_ref: Option<String>,
}

impl VisualRoundSnapshot {
    pub fn new(
        task_id: impl Into<String>,
        attempt_id: impl Into<String>,
    ) -> Result<Self, VisualRoundError> {
        let task_id = task_id.into();
        let attempt_id = attempt_id.into();
        if task_id.trim().is_empty() || attempt_id.trim().is_empty() {
            return Err(VisualRoundError::InvalidSnapshot);
        }
        Ok(Self {
            task_id,
            attempt_id,
            phase: VisualRoundPhase::AwaitingEvidence,
            next_round_index: 1,
            completed_rounds: 0,
            total_selections: 0,
            total_steps: 0,
            total_pointer_events: 0,
            total_key_events: 0,
            completed_round_ids: Vec::new(),
            expected_next_frame_digest: None,
            expected_next_context_digest: None,
            expected_next_geometry_digest: None,
            expected_next_candidate_set_digest: None,
            active_evidence: None,
            active_selection: None,
            active_intent_digest: None,
            last_effect_receipt_ref: None,
            terminal_outcome: None,
            intervention: None,
            terminal_receipt_ref: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRoundIntent {
    pub intent_digest: String,
    pub task_id: String,
    pub attempt_id: String,
    pub round_index: u8,
    pub round_id: String,
    pub evidence_digest: String,
    pub candidate_set_digest: String,
    pub selected_candidate_ids: Vec<String>,
    pub planned_steps: u8,
    pub planned_pointer_events: u8,
    pub planned_key_events: u8,
    pub provider_capability: VisualProviderCapability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualRoundEvent {
    Observe {
        evidence: VisualRoundEvidence,
        now_ms: u64,
    },
    Select {
        selection: VisualRoundSelection,
        now_ms: u64,
    },
    EffectFinished(VisualRoundEffectReceipt),
    Classify {
        after_state: VisualRoundAfterState,
        now_ms: u64,
    },
    Replay {
        original_receipt_ref: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualRoundDecision {
    AwaitSelection {
        snapshot: VisualRoundSnapshot,
    },
    PermitIntent {
        snapshot: VisualRoundSnapshot,
        intent: Box<VisualRoundIntent>,
    },
    AwaitAfterState {
        snapshot: VisualRoundSnapshot,
    },
    Continue {
        snapshot: VisualRoundSnapshot,
    },
    Terminal {
        snapshot: VisualRoundSnapshot,
    },
    Replay {
        snapshot: VisualRoundSnapshot,
        original_receipt_ref: String,
        emitted_new_effects: bool,
    },
}

impl VisualRoundDecision {
    pub fn snapshot(&self) -> &VisualRoundSnapshot {
        match self {
            Self::AwaitSelection { snapshot }
            | Self::PermitIntent { snapshot, .. }
            | Self::AwaitAfterState { snapshot }
            | Self::Continue { snapshot }
            | Self::Terminal { snapshot }
            | Self::Replay { snapshot, .. } => snapshot,
        }
    }

    pub fn emitted_effects(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualRoundError {
    InvalidPolicy,
    InvalidSnapshot,
    InvalidTransition,
}

pub fn decide_visual_round(
    policy: &VisualRoundPolicy,
    current: &VisualRoundSnapshot,
    event: VisualRoundEvent,
) -> Result<VisualRoundDecision, VisualRoundError> {
    validate_policy(policy)?;
    validate_snapshot(policy, current)?;
    if let VisualRoundEvent::Replay {
        original_receipt_ref,
    } = event
    {
        if current.phase != VisualRoundPhase::Terminal
            || current.terminal_receipt_ref.as_deref() != Some(original_receipt_ref.as_str())
        {
            return Err(VisualRoundError::InvalidTransition);
        }
        return Ok(VisualRoundDecision::Replay {
            snapshot: current.clone(),
            original_receipt_ref,
            emitted_new_effects: false,
        });
    }

    match (current.phase, event) {
        (VisualRoundPhase::AwaitingEvidence, VisualRoundEvent::Observe { evidence, now_ms }) => {
            observe(policy, current, evidence, now_ms)
        }
        (VisualRoundPhase::AwaitingSelection, VisualRoundEvent::Select { selection, now_ms }) => {
            select(policy, current, selection, now_ms)
        }
        (VisualRoundPhase::IntentAuthorized, VisualRoundEvent::EffectFinished(receipt)) => {
            finish_effect(policy, current, receipt)
        }
        (
            VisualRoundPhase::AwaitingAfterState,
            VisualRoundEvent::Classify {
                after_state,
                now_ms,
            },
        ) => classify(policy, current, after_state, now_ms),
        _ => Err(VisualRoundError::InvalidTransition),
    }
}

pub fn visual_candidate_set_digest(candidate_ids: &[String]) -> String {
    digest_parts(candidate_ids.iter().map(String::as_str))
}

pub fn visual_round_evidence_digest(evidence: &VisualRoundEvidence) -> String {
    let round_index = evidence.round_index.to_string();
    let observed_at_ms = evidence.observed_at_ms.to_string();
    let expires_at_ms = evidence.expires_at_ms.to_string();
    digest_parts([
        evidence.task_id.as_str(),
        evidence.attempt_id.as_str(),
        evidence.profile_digest.as_str(),
        evidence.policy_digest.as_str(),
        round_index.as_str(),
        evidence.round_id.as_str(),
        evidence.frame_digest.as_str(),
        evidence.context_digest.as_str(),
        evidence.geometry_digest.as_str(),
        evidence.candidate_set_digest.as_str(),
        observed_at_ms.as_str(),
        expires_at_ms.as_str(),
        evidence.provider_capability.capability_id.as_str(),
        evidence.provider_capability.capability_version.as_str(),
        evidence.provider_capability.capability_digest.as_str(),
    ])
}

pub fn visual_round_intent_digest(
    evidence: &VisualRoundEvidence,
    selection: &VisualRoundSelection,
) -> String {
    let round_index = evidence.round_index.to_string();
    let planned_steps = selection.planned_steps.to_string();
    let planned_pointer_events = selection.planned_pointer_events.to_string();
    let planned_key_events = selection.planned_key_events.to_string();
    let selected_candidates_digest = visual_candidate_set_digest(&selection.selected_candidate_ids);
    digest_parts([
        evidence.task_id.as_str(),
        evidence.attempt_id.as_str(),
        round_index.as_str(),
        evidence.round_id.as_str(),
        evidence.evidence_digest.as_str(),
        evidence.candidate_set_digest.as_str(),
        selected_candidates_digest.as_str(),
        planned_steps.as_str(),
        planned_pointer_events.as_str(),
        planned_key_events.as_str(),
        selection.provider_capability.capability_id.as_str(),
        selection.provider_capability.capability_version.as_str(),
        selection.provider_capability.capability_digest.as_str(),
    ])
}

fn observe(
    policy: &VisualRoundPolicy,
    current: &VisualRoundSnapshot,
    evidence: VisualRoundEvidence,
    now_ms: u64,
) -> Result<VisualRoundDecision, VisualRoundError> {
    if current.next_round_index > policy.max_rounds {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::UnexpectedRound,
        ));
    }
    if evidence.task_id != current.task_id
        || evidence.attempt_id != current.attempt_id
        || evidence.policy_digest != policy.policy_digest
        || evidence.profile_digest != policy.profile_digest
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::InvalidEvidence,
        ));
    }
    if evidence.provider_capability != policy.provider_capability {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::CapabilityMismatch,
        ));
    }
    if evidence.round_index != current.next_round_index
        || current.completed_round_ids.contains(&evidence.round_id)
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::RoundOrder,
        ));
    }
    let expected_next_matches = match (
        current.expected_next_frame_digest.as_deref(),
        current.expected_next_context_digest.as_deref(),
        current.expected_next_geometry_digest.as_deref(),
        current.expected_next_candidate_set_digest.as_deref(),
    ) {
        (None, None, None, None) => current.next_round_index == 1,
        (Some(frame), Some(context), Some(geometry), Some(candidates)) => {
            evidence.frame_digest == frame
                && evidence.context_digest == context
                && evidence.geometry_digest == geometry
                && evidence.candidate_set_digest == candidates
        }
        _ => false,
    };
    if !expected_next_matches {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::EvidenceChanged,
        ));
    }
    if !valid_evidence(&evidence) || now_ms < evidence.observed_at_ms {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::InvalidEvidence,
        ));
    }
    if now_ms >= evidence.expires_at_ms || now_ms >= policy.deadline_at_ms {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::StaleEvidence,
        ));
    }
    let mut next = current.clone();
    next.phase = VisualRoundPhase::AwaitingSelection;
    next.expected_next_frame_digest = None;
    next.expected_next_context_digest = None;
    next.expected_next_geometry_digest = None;
    next.expected_next_candidate_set_digest = None;
    next.active_evidence = Some(evidence);
    Ok(VisualRoundDecision::AwaitSelection { snapshot: next })
}

fn select(
    policy: &VisualRoundPolicy,
    current: &VisualRoundSnapshot,
    selection: VisualRoundSelection,
    now_ms: u64,
) -> Result<VisualRoundDecision, VisualRoundError> {
    let Some(evidence) = current.active_evidence.as_ref() else {
        return Err(VisualRoundError::InvalidSnapshot);
    };
    if now_ms >= evidence.expires_at_ms || now_ms >= policy.deadline_at_ms {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::StaleEvidence,
        ));
    }
    if selection.provider_capability != policy.provider_capability {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::CapabilityMismatch,
        ));
    }
    if selection.round_id != evidence.round_id
        || selection.evidence_digest != evidence.evidence_digest
        || selection.candidate_set_digest != evidence.candidate_set_digest
        || !nonempty_unique(&selection.selected_candidate_ids)
        || selection
            .selected_candidate_ids
            .iter()
            .any(|candidate| !evidence.candidate_ids.contains(candidate))
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::CandidateMismatch,
        ));
    }
    let selection_count = selection.selected_candidate_ids.len();
    if selection_count > usize::from(policy.max_selections_per_round)
        || selection.planned_steps > policy.max_steps_per_round
        || selection.planned_pointer_events > policy.max_pointer_events_per_round
        || selection.planned_key_events > policy.max_key_events_per_round
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::RoundBudgetExceeded,
        ));
    }
    if usize::from(current.total_selections)
        .checked_add(selection_count)
        .is_none_or(|total| total > usize::from(policy.max_total_selections))
        || current
            .total_steps
            .checked_add(selection.planned_steps)
            .is_none_or(|total| total > policy.max_total_steps)
        || current
            .total_pointer_events
            .checked_add(selection.planned_pointer_events)
            .is_none_or(|total| total > policy.max_total_pointer_events)
        || current
            .total_key_events
            .checked_add(selection.planned_key_events)
            .is_none_or(|total| total > policy.max_total_key_events)
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::CumulativeBudgetExceeded,
        ));
    }

    let intent_digest = visual_round_intent_digest(evidence, &selection);
    let intent = VisualRoundIntent {
        intent_digest: intent_digest.clone(),
        task_id: current.task_id.clone(),
        attempt_id: current.attempt_id.clone(),
        round_index: evidence.round_index,
        round_id: evidence.round_id.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        candidate_set_digest: evidence.candidate_set_digest.clone(),
        selected_candidate_ids: selection.selected_candidate_ids.clone(),
        planned_steps: selection.planned_steps,
        planned_pointer_events: selection.planned_pointer_events,
        planned_key_events: selection.planned_key_events,
        provider_capability: selection.provider_capability.clone(),
    };
    let mut next = current.clone();
    next.phase = VisualRoundPhase::IntentAuthorized;
    next.active_selection = Some(selection);
    next.active_intent_digest = Some(intent_digest);
    Ok(VisualRoundDecision::PermitIntent {
        snapshot: next,
        intent: Box::new(intent),
    })
}

fn finish_effect(
    _policy: &VisualRoundPolicy,
    current: &VisualRoundSnapshot,
    receipt: VisualRoundEffectReceipt,
) -> Result<VisualRoundDecision, VisualRoundError> {
    let (Some(evidence), Some(selection), Some(intent_digest)) = (
        current.active_evidence.as_ref(),
        current.active_selection.as_ref(),
        current.active_intent_digest.as_ref(),
    ) else {
        return Err(VisualRoundError::InvalidSnapshot);
    };
    if receipt.round_id != evidence.round_id
        || receipt.evidence_digest != evidence.evidence_digest
        || receipt.intent_digest != *intent_digest
        || receipt.receipt_ref.trim().is_empty()
        || receipt.steps != selection.planned_steps
        || receipt.pointer_events != selection.planned_pointer_events
        || receipt.key_events != selection.planned_key_events
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::EffectMismatch,
        ));
    }

    let mut next = current.clone();
    let selection_count = u8::try_from(selection.selected_candidate_ids.len())
        .map_err(|_| VisualRoundError::InvalidTransition)?;
    next.total_selections = next
        .total_selections
        .checked_add(selection_count)
        .ok_or(VisualRoundError::InvalidTransition)?;
    next.total_steps = next
        .total_steps
        .checked_add(receipt.steps)
        .ok_or(VisualRoundError::InvalidTransition)?;
    next.total_pointer_events = next
        .total_pointer_events
        .checked_add(receipt.pointer_events)
        .ok_or(VisualRoundError::InvalidTransition)?;
    next.total_key_events = next
        .total_key_events
        .checked_add(receipt.key_events)
        .ok_or(VisualRoundError::InvalidTransition)?;
    next.last_effect_receipt_ref = Some(receipt.receipt_ref.clone());
    match receipt.delivery {
        DeliveryState::Acknowledged => {
            next.phase = VisualRoundPhase::AwaitingAfterState;
            Ok(VisualRoundDecision::AwaitAfterState { snapshot: next })
        }
        DeliveryState::Partial => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::InterventionRequired,
            Some(VisualRoundInterventionReason::PartialEffect),
            Some(receipt.receipt_ref),
        )),
        DeliveryState::Uncertain => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::InterventionRequired,
            Some(VisualRoundInterventionReason::UncertainEffect),
            Some(receipt.receipt_ref),
        )),
        DeliveryState::Rejected => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::Denied,
            None,
            Some(receipt.receipt_ref),
        )),
    }
}

fn classify(
    policy: &VisualRoundPolicy,
    current: &VisualRoundSnapshot,
    after_state: VisualRoundAfterState,
    now_ms: u64,
) -> Result<VisualRoundDecision, VisualRoundError> {
    let Some(evidence) = current.active_evidence.as_ref() else {
        return Err(VisualRoundError::InvalidSnapshot);
    };
    if after_state.round_id != evidence.round_id
        || after_state.evidence_digest != evidence.evidence_digest
        || !valid_digest(&after_state.frame_digest)
        || !valid_digest(&after_state.context_digest)
        || !valid_digest(&after_state.geometry_digest)
        || !valid_digest(&after_state.candidate_set_digest)
        || after_state.observed_at_ms <= evidence.observed_at_ms
        || now_ms < after_state.observed_at_ms
    {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::InvalidEvidence,
        ));
    }
    if now_ms >= after_state.expires_at_ms || now_ms >= policy.deadline_at_ms {
        return Ok(intervention(
            current,
            VisualRoundInterventionReason::StaleEvidence,
        ));
    }
    let mut next = current.clone();
    next.completed_rounds = next.completed_rounds.saturating_add(1);
    next.completed_round_ids.push(evidence.round_id.clone());
    match after_state.outcome {
        VisualAfterStateOutcome::NextRound => {
            if next.completed_rounds >= policy.max_rounds {
                return Ok(intervention(
                    &next,
                    VisualRoundInterventionReason::UnexpectedRound,
                ));
            }
            next.next_round_index = next.next_round_index.saturating_add(1);
            next.phase = VisualRoundPhase::AwaitingEvidence;
            next.expected_next_frame_digest = Some(after_state.frame_digest);
            next.expected_next_context_digest = Some(after_state.context_digest);
            next.expected_next_geometry_digest = Some(after_state.geometry_digest);
            next.expected_next_candidate_set_digest = Some(after_state.candidate_set_digest);
            next.active_evidence = None;
            next.active_selection = None;
            next.active_intent_digest = None;
            Ok(VisualRoundDecision::Continue { snapshot: next })
        }
        VisualAfterStateOutcome::Passed => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::Passed,
            None,
            terminal_receipt(&after_state)?,
        )),
        VisualAfterStateOutcome::Denied => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::Denied,
            None,
            terminal_receipt(&after_state)?,
        )),
        VisualAfterStateOutcome::Ambiguous => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::InterventionRequired,
            Some(VisualRoundInterventionReason::Ambiguous),
            terminal_receipt(&after_state)?,
        )),
        VisualAfterStateOutcome::Unsupported => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::InterventionRequired,
            Some(VisualRoundInterventionReason::Unsupported),
            terminal_receipt(&after_state)?,
        )),
        VisualAfterStateOutcome::Inconclusive => Ok(terminal(
            next,
            VisualRoundTerminalOutcome::InterventionRequired,
            Some(VisualRoundInterventionReason::Inconclusive),
            terminal_receipt(&after_state)?,
        )),
    }
}

fn intervention(
    current: &VisualRoundSnapshot,
    reason: VisualRoundInterventionReason,
) -> VisualRoundDecision {
    terminal(
        current.clone(),
        VisualRoundTerminalOutcome::InterventionRequired,
        Some(reason),
        None,
    )
}

fn terminal(
    mut snapshot: VisualRoundSnapshot,
    outcome: VisualRoundTerminalOutcome,
    intervention: Option<VisualRoundInterventionReason>,
    receipt_ref: Option<String>,
) -> VisualRoundDecision {
    snapshot.phase = VisualRoundPhase::Terminal;
    snapshot.terminal_outcome = Some(outcome);
    snapshot.intervention = intervention;
    snapshot.terminal_receipt_ref = receipt_ref;
    VisualRoundDecision::Terminal { snapshot }
}

fn terminal_receipt(
    after_state: &VisualRoundAfterState,
) -> Result<Option<String>, VisualRoundError> {
    after_state
        .terminal_receipt_ref
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(Some)
        .ok_or(VisualRoundError::InvalidTransition)
}

pub(crate) fn validate_policy(policy: &VisualRoundPolicy) -> Result<(), VisualRoundError> {
    if !valid_digest(&policy.policy_digest)
        || !valid_digest(&policy.profile_digest)
        || !valid_capability(&policy.provider_capability)
        || policy.max_rounds == 0
        || policy.max_selections_per_round == 0
        || policy.max_steps_per_round == 0
        || policy.max_total_selections == 0
        || policy.max_total_steps == 0
        || policy.deadline_at_ms == 0
    {
        return Err(VisualRoundError::InvalidPolicy);
    }
    Ok(())
}

fn validate_snapshot(
    policy: &VisualRoundPolicy,
    snapshot: &VisualRoundSnapshot,
) -> Result<(), VisualRoundError> {
    if snapshot.task_id.trim().is_empty()
        || snapshot.attempt_id.trim().is_empty()
        || snapshot.next_round_index == 0
        || snapshot.completed_rounds as usize != snapshot.completed_round_ids.len()
        || !all_unique(&snapshot.completed_round_ids)
        || snapshot.completed_rounds > policy.max_rounds
        || snapshot.total_selections > policy.max_total_selections
        || snapshot.total_steps > policy.max_total_steps
        || snapshot.total_pointer_events > policy.max_total_pointer_events
        || snapshot.total_key_events > policy.max_total_key_events
    {
        return Err(VisualRoundError::InvalidSnapshot);
    }
    if snapshot
        .last_effect_receipt_ref
        .as_deref()
        .is_some_and(str::is_empty)
    {
        return Err(VisualRoundError::InvalidSnapshot);
    }
    let expected_next_shape = (
        snapshot.expected_next_frame_digest.is_some(),
        snapshot.expected_next_context_digest.is_some(),
        snapshot.expected_next_geometry_digest.is_some(),
        snapshot.expected_next_candidate_set_digest.is_some(),
    );
    if !matches!(
        expected_next_shape,
        (false, false, false, false) | (true, true, true, true)
    ) {
        return Err(VisualRoundError::InvalidSnapshot);
    }
    let active_shape = (
        snapshot.active_evidence.as_ref(),
        snapshot.active_selection.as_ref(),
        snapshot.active_intent_digest.as_ref(),
    );
    let phase_valid = match snapshot.phase {
        VisualRoundPhase::AwaitingEvidence => {
            matches!(active_shape, (None, None, None))
                && snapshot.next_round_index == snapshot.completed_rounds.saturating_add(1)
                && if snapshot.next_round_index == 1 {
                    expected_next_shape == (false, false, false, false)
                } else {
                    expected_next_shape == (true, true, true, true)
                }
        }
        VisualRoundPhase::AwaitingSelection => {
            matches!(active_shape, (Some(_), None, None))
                && expected_next_shape == (false, false, false, false)
        }
        VisualRoundPhase::IntentAuthorized => {
            matches!(active_shape, (Some(_), Some(_), Some(_)))
                && expected_next_shape == (false, false, false, false)
        }
        VisualRoundPhase::AwaitingAfterState => {
            matches!(active_shape, (Some(_), Some(_), Some(_)))
                && expected_next_shape == (false, false, false, false)
                && snapshot
                    .last_effect_receipt_ref
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())
        }
        VisualRoundPhase::Terminal => snapshot.terminal_outcome.is_some(),
    };
    if !phase_valid {
        return Err(VisualRoundError::InvalidSnapshot);
    }
    if snapshot.phase != VisualRoundPhase::Terminal
        && (snapshot.terminal_outcome.is_some()
            || snapshot.intervention.is_some()
            || snapshot.terminal_receipt_ref.is_some())
    {
        return Err(VisualRoundError::InvalidSnapshot);
    }
    if snapshot.phase == VisualRoundPhase::Terminal {
        match snapshot.terminal_outcome {
            Some(VisualRoundTerminalOutcome::InterventionRequired) => {
                if snapshot.intervention.is_none() {
                    return Err(VisualRoundError::InvalidSnapshot);
                }
            }
            Some(VisualRoundTerminalOutcome::Passed | VisualRoundTerminalOutcome::Denied) => {
                if snapshot.intervention.is_some()
                    || snapshot
                        .terminal_receipt_ref
                        .as_deref()
                        .is_none_or(str::is_empty)
                {
                    return Err(VisualRoundError::InvalidSnapshot);
                }
            }
            None => return Err(VisualRoundError::InvalidSnapshot),
        }
    }
    if let Some(evidence) = snapshot.active_evidence.as_ref() {
        if !valid_evidence(evidence)
            || evidence.task_id != snapshot.task_id
            || evidence.attempt_id != snapshot.attempt_id
            || evidence.round_index != snapshot.next_round_index
            || evidence.profile_digest != policy.profile_digest
            || evidence.policy_digest != policy.policy_digest
            || evidence.provider_capability != policy.provider_capability
        {
            return Err(VisualRoundError::InvalidSnapshot);
        }
    }
    if let (Some(evidence), Some(selection), Some(intent_digest)) = active_shape {
        if !valid_selection(policy, evidence, selection)
            || visual_round_intent_digest(evidence, selection) != *intent_digest
        {
            return Err(VisualRoundError::InvalidSnapshot);
        }
    }
    Ok(())
}

fn valid_selection(
    policy: &VisualRoundPolicy,
    evidence: &VisualRoundEvidence,
    selection: &VisualRoundSelection,
) -> bool {
    let Ok(selection_count) = u8::try_from(selection.selected_candidate_ids.len()) else {
        return false;
    };
    selection.round_id == evidence.round_id
        && selection.evidence_digest == evidence.evidence_digest
        && selection.candidate_set_digest == evidence.candidate_set_digest
        && selection.provider_capability == policy.provider_capability
        && nonempty_unique(&selection.selected_candidate_ids)
        && selection
            .selected_candidate_ids
            .iter()
            .all(|candidate| evidence.candidate_ids.contains(candidate))
        && selection_count <= policy.max_selections_per_round
        && selection.planned_steps <= policy.max_steps_per_round
        && selection.planned_pointer_events <= policy.max_pointer_events_per_round
        && selection.planned_key_events <= policy.max_key_events_per_round
}

pub(crate) fn valid_evidence(evidence: &VisualRoundEvidence) -> bool {
    !evidence.task_id.trim().is_empty()
        && !evidence.attempt_id.trim().is_empty()
        && !evidence.round_id.trim().is_empty()
        && valid_digest(&evidence.profile_digest)
        && valid_digest(&evidence.policy_digest)
        && valid_digest(&evidence.evidence_digest)
        && valid_digest(&evidence.frame_digest)
        && valid_digest(&evidence.context_digest)
        && valid_digest(&evidence.geometry_digest)
        && valid_digest(&evidence.candidate_set_digest)
        && nonempty_unique(&evidence.candidate_ids)
        && evidence.candidate_set_digest == visual_candidate_set_digest(&evidence.candidate_ids)
        && evidence.evidence_digest == visual_round_evidence_digest(evidence)
        && evidence.observed_at_ms < evidence.expires_at_ms
        && valid_capability(&evidence.provider_capability)
}

pub(crate) fn digest_parts<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn valid_capability(capability: &VisualProviderCapability) -> bool {
    !capability.capability_id.trim().is_empty()
        && !capability.capability_version.trim().is_empty()
        && valid_digest(&capability.capability_digest)
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn nonempty_unique(values: &[String]) -> bool {
    !values.is_empty()
        && values.iter().all(|value| !value.trim().is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn all_unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
