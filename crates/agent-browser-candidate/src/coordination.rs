use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{validate_nonempty, CandidateError};

pub const COORDINATION_LEDGER_SCHEMA_VERSION: &str =
    "agent-browser.candidate-coordination-ledger.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoordinatedOperation {
    pub operation_id: String,
    pub candidate_id: String,
    pub artifact_id: String,
    pub fencing_generation: u64,
    pub submitted_by_request_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationAction {
    StartOrJoin,
    Queue,
    CancelActive { operation_id: String },
    DiscardQueued { operation_id: String },
    Supersede { operation_id: String },
    ActivateQueued { operation_id: String },
    CompleteActive { operation_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoordinationRequest {
    pub request_id: String,
    pub candidate_id: String,
    pub artifact_id: String,
    pub expected_revision: u64,
    pub expected_fencing_generation: u64,
    pub action: CoordinationAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationOutcome {
    Started,
    JoinedExisting,
    Contended,
    Queued,
    Cancelled,
    Discarded,
    Superseded,
    ActivatedQueued,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPinReason {
    Active,
    Queued,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoordinationReceipt {
    pub request_id: String,
    pub outcome: CoordinationOutcome,
    pub operation_id: Option<String>,
    pub active_operation_id: Option<String>,
    pub revision: u64,
    pub fencing_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppliedCoordinationRequest {
    request: CoordinationRequest,
    receipt: CoordinationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoordinationLedger {
    pub schema_version: String,
    pub environment_id: String,
    pub revision: u64,
    pub fencing_generation: u64,
    active: Option<CoordinatedOperation>,
    queue: Vec<CoordinatedOperation>,
    applied_requests: BTreeMap<String, AppliedCoordinationRequest>,
    receipts: Vec<CoordinationReceipt>,
}

impl CoordinationLedger {
    pub fn new(environment_id: impl Into<String>) -> Self {
        Self {
            schema_version: COORDINATION_LEDGER_SCHEMA_VERSION.to_string(),
            environment_id: environment_id.into(),
            revision: 0,
            fencing_generation: 0,
            active: None,
            queue: Vec::new(),
            applied_requests: BTreeMap::new(),
            receipts: Vec::new(),
        }
    }

    pub fn active(&self) -> Option<&CoordinatedOperation> {
        self.active.as_ref()
    }

    pub fn queue(&self) -> &[CoordinatedOperation] {
        &self.queue
    }

    pub fn receipts(&self) -> &[CoordinationReceipt] {
        &self.receipts
    }

    pub fn artifact_pins(&self) -> BTreeMap<String, Vec<ArtifactPinReason>> {
        let mut pins = BTreeMap::<String, Vec<ArtifactPinReason>>::new();
        if let Some(active) = &self.active {
            pins.entry(active.artifact_id.clone())
                .or_default()
                .push(ArtifactPinReason::Active);
        }
        for queued in &self.queue {
            let reasons = pins.entry(queued.artifact_id.clone()).or_default();
            if !reasons.contains(&ArtifactPinReason::Queued) {
                reasons.push(ArtifactPinReason::Queued);
            }
        }
        for reasons in pins.values_mut() {
            reasons.sort();
        }
        pins
    }

    /// Validate a deserialized ledger before an adapter trusts its fencing or
    /// revision counters.
    pub fn validate(&self) -> Result<(), CandidateError> {
        if self.schema_version != COORDINATION_LEDGER_SCHEMA_VERSION {
            return Err(CandidateError::new(
                "unsupported_coordination_ledger_schema",
                "coordination ledger schema is not supported",
            ));
        }
        validate_nonempty("environment_id", &self.environment_id)?;
        if self.revision != self.receipts.len() as u64
            || self.applied_requests.len() != self.receipts.len()
        {
            return Err(CandidateError::new(
                "coordination_revision_inconsistent",
                "revision, applied request, and receipt counts must agree",
            ));
        }

        let mut prior_fence = 0;
        for (index, receipt) in self.receipts.iter().enumerate() {
            if receipt.revision != index as u64 + 1
                || receipt.fencing_generation < prior_fence
                || receipt.fencing_generation > self.fencing_generation
            {
                return Err(CandidateError::new(
                    "coordination_receipt_sequence_inconsistent",
                    "receipt revisions and fencing generations must be monotonic",
                ));
            }
            prior_fence = receipt.fencing_generation;
            let applied = self
                .applied_requests
                .get(&receipt.request_id)
                .filter(|applied| applied.receipt == *receipt)
                .ok_or_else(|| {
                    CandidateError::new(
                        "coordination_receipt_request_mismatch",
                        "every receipt must match its exact applied request",
                    )
                })?;
            validate_request(&applied.request)?;
            if applied.request.request_id != receipt.request_id {
                return Err(CandidateError::new(
                    "coordination_receipt_request_mismatch",
                    "applied request key and receipt request ID must agree",
                ));
            }
        }
        if prior_fence != self.fencing_generation {
            return Err(CandidateError::new(
                "coordination_fence_inconsistent",
                "latest receipt must bind the current fencing generation",
            ));
        }

        let mut operation_ids = BTreeSet::new();
        if let Some(active) = &self.active {
            validate_operation(active)?;
            if active.fencing_generation != self.fencing_generation {
                return Err(CandidateError::new(
                    "coordination_active_fence_inconsistent",
                    "active operation must bind the current fencing generation",
                ));
            }
            operation_ids.insert(active.operation_id.as_str());
        }
        for queued in &self.queue {
            validate_operation(queued)?;
            if queued.fencing_generation > self.fencing_generation
                || !operation_ids.insert(queued.operation_id.as_str())
            {
                return Err(CandidateError::new(
                    "coordination_queue_inconsistent",
                    "queued operation IDs must be unique and cannot use a future fence",
                ));
            }
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        request: CoordinationRequest,
    ) -> Result<CoordinationReceipt, CandidateError> {
        validate_request(&request)?;
        if let Some(applied) = self.applied_requests.get(&request.request_id) {
            if applied.request == request {
                return Ok(applied.receipt.clone());
            }
            return Err(CandidateError::new(
                "request_id_conflict",
                "coordination request ID was already used with different content",
            ));
        }
        if request.expected_fencing_generation != self.fencing_generation {
            return Err(CandidateError::new(
                "stale_fencing_generation",
                "coordination request fencing generation is stale",
            ));
        }
        if request.expected_revision != self.revision {
            return Err(CandidateError::new(
                "stale_revision",
                "coordination request revision is stale",
            ));
        }

        let mut next = self.clone();
        let receipt = next.apply_fresh(request)?;
        *self = next;
        Ok(receipt)
    }

    fn apply_fresh(
        &mut self,
        request: CoordinationRequest,
    ) -> Result<CoordinationReceipt, CandidateError> {
        let (outcome, operation_id) = match &request.action {
            CoordinationAction::StartOrJoin => self.start_or_join(&request)?,
            CoordinationAction::Queue => self.queue_or_join(&request),
            CoordinationAction::CancelActive { operation_id } => {
                self.cancel_active(&request, operation_id)?
            }
            CoordinationAction::DiscardQueued { operation_id } => {
                self.discard_queued(&request, operation_id)?
            }
            CoordinationAction::Supersede { operation_id } => {
                self.supersede(&request, operation_id)?
            }
            CoordinationAction::ActivateQueued { operation_id } => {
                self.activate_queued(&request, operation_id)?
            }
            CoordinationAction::CompleteActive { operation_id } => {
                self.complete_active(&request, operation_id)?
            }
        };
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| CandidateError::new("revision_exhausted", "revision overflow"))?;
        let receipt = CoordinationReceipt {
            request_id: request.request_id.clone(),
            outcome,
            operation_id,
            active_operation_id: self
                .active
                .as_ref()
                .map(|operation| operation.operation_id.clone()),
            revision: self.revision,
            fencing_generation: self.fencing_generation,
        };
        self.applied_requests.insert(
            request.request_id.clone(),
            AppliedCoordinationRequest {
                request,
                receipt: receipt.clone(),
            },
        );
        self.receipts.push(receipt.clone());
        Ok(receipt)
    }

    fn start_or_join(
        &mut self,
        request: &CoordinationRequest,
    ) -> Result<(CoordinationOutcome, Option<String>), CandidateError> {
        if let Some(active) = &self.active {
            if active.candidate_id == request.candidate_id {
                return Ok((
                    CoordinationOutcome::JoinedExisting,
                    Some(active.operation_id.clone()),
                ));
            }
            return Ok((
                CoordinationOutcome::Contended,
                Some(active.operation_id.clone()),
            ));
        }
        self.advance_fence()?;
        let operation = operation_from_request(request, self.fencing_generation);
        let operation_id = operation.operation_id.clone();
        self.active = Some(operation);
        Ok((CoordinationOutcome::Started, Some(operation_id)))
    }

    fn queue_or_join(
        &mut self,
        request: &CoordinationRequest,
    ) -> (CoordinationOutcome, Option<String>) {
        if let Some(active) = &self.active {
            if active.candidate_id == request.candidate_id {
                return (
                    CoordinationOutcome::JoinedExisting,
                    Some(active.operation_id.clone()),
                );
            }
        }
        if let Some(queued) = self
            .queue
            .iter()
            .find(|queued| queued.candidate_id == request.candidate_id)
        {
            return (
                CoordinationOutcome::JoinedExisting,
                Some(queued.operation_id.clone()),
            );
        }
        let operation = operation_from_request(request, self.fencing_generation);
        let operation_id = operation.operation_id.clone();
        self.queue.push(operation);
        (CoordinationOutcome::Queued, Some(operation_id))
    }

    fn cancel_active(
        &mut self,
        request: &CoordinationRequest,
        operation_id: &str,
    ) -> Result<(CoordinationOutcome, Option<String>), CandidateError> {
        let active = self.require_active_match(request, operation_id)?;
        let cancelled_id = active.operation_id.clone();
        self.active = None;
        self.advance_fence()?;
        Ok((CoordinationOutcome::Cancelled, Some(cancelled_id)))
    }

    fn discard_queued(
        &mut self,
        request: &CoordinationRequest,
        operation_id: &str,
    ) -> Result<(CoordinationOutcome, Option<String>), CandidateError> {
        let index = self
            .queue
            .iter()
            .position(|queued| {
                queued.operation_id == operation_id
                    && queued.candidate_id == request.candidate_id
                    && queued.artifact_id == request.artifact_id
            })
            .ok_or_else(|| {
                CandidateError::new(
                    "queued_operation_not_found",
                    "exact queued operation was not found",
                )
            })?;
        let discarded = self.queue.remove(index);
        Ok((CoordinationOutcome::Discarded, Some(discarded.operation_id)))
    }

    fn supersede(
        &mut self,
        request: &CoordinationRequest,
        operation_id: &str,
    ) -> Result<(CoordinationOutcome, Option<String>), CandidateError> {
        self.require_active_operation_id(operation_id)?;
        self.advance_fence()?;
        let replacement = operation_from_request(request, self.fencing_generation);
        let replacement_id = replacement.operation_id.clone();
        self.active = Some(replacement);
        Ok((CoordinationOutcome::Superseded, Some(replacement_id)))
    }

    fn activate_queued(
        &mut self,
        request: &CoordinationRequest,
        operation_id: &str,
    ) -> Result<(CoordinationOutcome, Option<String>), CandidateError> {
        if self.active.is_some() {
            return Err(CandidateError::new(
                "active_operation_present",
                "a queued operation cannot activate while another operation is active",
            ));
        }
        let head = self.queue.first().ok_or_else(|| {
            CandidateError::new("queue_empty", "no queued operation is available")
        })?;
        if head.operation_id != operation_id
            || head.candidate_id != request.candidate_id
            || head.artifact_id != request.artifact_id
        {
            return Err(CandidateError::new(
                "queue_head_mismatch",
                "only the exact FIFO queue head may activate",
            ));
        }
        self.advance_fence()?;
        let mut activated = self.queue.remove(0);
        activated.fencing_generation = self.fencing_generation;
        let activated_id = activated.operation_id.clone();
        self.active = Some(activated);
        Ok((CoordinationOutcome::ActivatedQueued, Some(activated_id)))
    }

    fn complete_active(
        &mut self,
        request: &CoordinationRequest,
        operation_id: &str,
    ) -> Result<(CoordinationOutcome, Option<String>), CandidateError> {
        let active = self.require_active_match(request, operation_id)?;
        let completed_id = active.operation_id.clone();
        self.active = None;
        self.advance_fence()?;
        Ok((CoordinationOutcome::Completed, Some(completed_id)))
    }

    fn require_active_match(
        &self,
        request: &CoordinationRequest,
        operation_id: &str,
    ) -> Result<&CoordinatedOperation, CandidateError> {
        let active = self.require_active_operation_id(operation_id)?;
        if active.candidate_id != request.candidate_id || active.artifact_id != request.artifact_id
        {
            return Err(CandidateError::new(
                "active_operation_identity_mismatch",
                "candidate or artifact does not match the active operation",
            ));
        }
        Ok(active)
    }

    fn require_active_operation_id(
        &self,
        operation_id: &str,
    ) -> Result<&CoordinatedOperation, CandidateError> {
        self.active
            .as_ref()
            .filter(|active| active.operation_id == operation_id)
            .ok_or_else(|| {
                CandidateError::new(
                    "active_operation_not_found",
                    "exact active operation was not found",
                )
            })
    }

    fn advance_fence(&mut self) -> Result<(), CandidateError> {
        self.fencing_generation = self.fencing_generation.checked_add(1).ok_or_else(|| {
            CandidateError::new(
                "fencing_generation_exhausted",
                "fencing generation cannot advance beyond u64::MAX",
            )
        })?;
        Ok(())
    }
}

fn operation_from_request(
    request: &CoordinationRequest,
    fencing_generation: u64,
) -> CoordinatedOperation {
    CoordinatedOperation {
        operation_id: format!("operation-{}", request.request_id),
        candidate_id: request.candidate_id.clone(),
        artifact_id: request.artifact_id.clone(),
        fencing_generation,
        submitted_by_request_id: request.request_id.clone(),
    }
}

fn validate_request(request: &CoordinationRequest) -> Result<(), CandidateError> {
    validate_nonempty("request_id", &request.request_id)?;
    validate_nonempty("candidate_id", &request.candidate_id)?;
    validate_nonempty("artifact_id", &request.artifact_id)?;
    Ok(())
}

fn validate_operation(operation: &CoordinatedOperation) -> Result<(), CandidateError> {
    validate_nonempty("operation_id", &operation.operation_id)?;
    validate_nonempty("candidate_id", &operation.candidate_id)?;
    validate_nonempty("artifact_id", &operation.artifact_id)?;
    validate_nonempty(
        "submitted_by_request_id",
        &operation.submitted_by_request_id,
    )
}
