use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::CandidateError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Planned,
    Building,
    Sealed,
    Installing,
    Accepted,
    Cancelled,
    Discarded,
    RecoveryRequired,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRecord {
    pub operation_id: String,
    pub candidate_id: String,
    pub state: OperationState,
    pub revision: u64,
    pub fencing_generation: u64,
    applied_requests: BTreeMap<String, AppliedTransition>,
}

impl OperationRecord {
    pub fn new(
        operation_id: impl Into<String>,
        candidate_id: impl Into<String>,
        fencing_generation: u64,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            candidate_id: candidate_id.into(),
            state: OperationState::Planned,
            revision: 0,
            fencing_generation,
            applied_requests: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionRequest {
    pub request_id: String,
    pub expected_revision: u64,
    pub expected_fencing_generation: u64,
    pub next_state: OperationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionResult {
    Applied,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppliedTransition {
    expected_revision: u64,
    expected_fencing_generation: u64,
    next_state: OperationState,
}

pub fn apply_transition(
    operation: &mut OperationRecord,
    request: TransitionRequest,
) -> Result<TransitionResult, CandidateError> {
    if request.request_id.trim().is_empty() {
        return Err(CandidateError::new(
            "empty_request_id",
            "transition request ID must not be empty",
        ));
    }
    let applied = AppliedTransition {
        expected_revision: request.expected_revision,
        expected_fencing_generation: request.expected_fencing_generation,
        next_state: request.next_state,
    };
    if let Some(previous) = operation.applied_requests.get(&request.request_id) {
        if previous == &applied {
            return Ok(TransitionResult::AlreadyApplied);
        }
        return Err(CandidateError::new(
            "request_id_conflict",
            "request ID was already used for a different transition",
        ));
    }
    if request.expected_fencing_generation != operation.fencing_generation {
        return Err(CandidateError::new(
            "stale_fencing_generation",
            "transition fencing generation does not match the operation",
        ));
    }
    if request.expected_revision != operation.revision {
        return Err(CandidateError::new(
            "stale_revision",
            "transition revision does not match the operation",
        ));
    }
    if !transition_allowed(operation.state, request.next_state) {
        return Err(CandidateError::new(
            "invalid_state_transition",
            format!(
                "transition from {:?} to {:?} is not supported",
                operation.state, request.next_state
            ),
        ));
    }

    operation.state = request.next_state;
    operation.revision += 1;
    operation
        .applied_requests
        .insert(request.request_id, applied);
    Ok(TransitionResult::Applied)
}

fn transition_allowed(current: OperationState, next: OperationState) -> bool {
    matches!(
        (current, next),
        (OperationState::Planned, OperationState::Building)
            | (OperationState::Planned, OperationState::Cancelled)
            | (OperationState::Building, OperationState::Sealed)
            | (OperationState::Building, OperationState::Cancelled)
            | (OperationState::Building, OperationState::Failed)
            | (OperationState::Sealed, OperationState::Installing)
            | (OperationState::Sealed, OperationState::Discarded)
            | (OperationState::Installing, OperationState::Accepted)
            | (OperationState::Installing, OperationState::RecoveryRequired)
            | (OperationState::RecoveryRequired, OperationState::Installing)
            | (OperationState::RecoveryRequired, OperationState::RolledBack)
    )
}
