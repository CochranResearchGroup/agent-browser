//! Typed native admission exclusion. These observations are not close authority;
//! runtime adapters own durable operation custody and exact application effects.
use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const REMOTE_VIEW_IDLE_RETIREMENT_SOURCE_CHECKPOINT: &str =
    "22775a23ea1271e60838b4d95b9992e7533a1e5d";

/// Private provider fence retained with the application operation. Its native
/// slot is never an operator locator, and the fence grants no viewer access.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewIdleRetirementFence {
    schema_version: u32,
    installation_id: String,
    config_hash: String,
    slot: u32,
    generation: u64,
    operation_id: String,
    revision: u64,
}

fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl RemoteViewIdleRetirementFence {
    fn validate(&self) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1
            || self.slot == 0
            || self.generation == 0
            || self.revision == 0
            || !(16..=64).contains(&self.installation_id.len())
            || !self
                .installation_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
            || !digest(&self.config_hash)
            || !digest(&self.operation_id)
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewIdleRetirementState {
    pub revision: u64,
    pub fence: Option<RemoteViewIdleRetirementFence>,
    pub completed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewIdleRetirementRead {
    pub schema_version: u32,
    pub target: RemoteViewApplicationTarget,
    pub readiness_scope: RemoteViewApplicationReadinessScope,
    pub retirement: RemoteViewIdleRetirementState,
}

/// Immutable provider inputs to retain in application custody before prepare.
/// This intent does not authorize any local browser or target close.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewIdleRetirementIntent {
    pub schema_version: u32,
    pub assignment: RemoteViewAssignmentRecord,
    pub target: RemoteViewApplicationTarget,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub expected_retirement: RemoteViewDesktopViewingRetirement,
}

impl RemoteViewIdleRetirementIntent {
    pub fn new(
        assignment: &RemoteViewAssignmentRecord,
        idle: &RemoteViewIdleAssignmentObservation,
        admission: &RemoteViewIdleRetirementRead,
        idempotency_key: &str,
    ) -> Result<Self, RemoteViewApplicationResponseError> {
        idle.release_target(assignment)?;
        admission.validate(assignment, &idle.target)?;
        if idle.viewer_active || !admission.retirement.completed {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        let intent = Self {
            schema_version: 1,
            assignment: assignment.clone(),
            target: idle.target.clone(),
            idempotency_key: idempotency_key.into(),
            expected_revision: admission.retirement.revision,
            expected_retirement: idle.retirement.clone(),
        };
        intent.validate()?;
        Ok(intent)
    }

    pub fn validate(&self) -> Result<(), RemoteViewApplicationResponseError> {
        self.target.validate_assignment(&self.assignment)?;
        let retirement = &self.expected_retirement;
        if self.schema_version != 1
            || self.expected_revision.checked_add(1).is_none()
            || self.idempotency_key.is_empty()
            || self.idempotency_key.len() > 128
            || !self
                .idempotency_key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || retirement.schema_version != 1
            || retirement.desktop_id != self.target.desktop_id
            || retirement.generation != self.target.lifecycle_generation
            || !retirement.sessions.is_empty()
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        RemoteViewApplicationReleaseTarget {
            assignment: self.assignment.clone(),
            route_ids: retirement.routes.clone(),
            viewer_session_ids: vec![],
        }
        .validate()
    }

    /// Frozen producer recipe: the registration-scoped key, exact public
    /// target and retirement snapshot, serialized in their published order.
    fn operation_digest(&self) -> Result<String, RemoteViewApplicationResponseError> {
        self.validate()?;
        let key = format!(
            "consumer-{:x}",
            Sha256::digest(
                format!("{}:{}", self.target.registration_id, self.idempotency_key).as_bytes()
            )
        );
        let bytes = serde_json::to_vec(&(key, &self.target, &self.expected_retirement))
            .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewIdleRetirementReceiptState {
    Prepared,
    Completed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewIdleRetirementReceipt {
    pub schema_version: u32,
    pub target: RemoteViewApplicationTarget,
    pub readiness_scope: RemoteViewApplicationReadinessScope,
    pub state: RemoteViewIdleRetirementReceiptState,
    pub fence: RemoteViewIdleRetirementFence,
}

impl RemoteViewIdleRetirementReceipt {
    pub fn validate(
        &self,
        intent: &RemoteViewIdleRetirementIntent,
        state: RemoteViewIdleRetirementReceiptState,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        intent.validate()?;
        self.fence.validate()?;
        if self.schema_version != 1 || self.state != state {
            return Err(RemoteViewApplicationResponseError::InvalidShape);
        }
        if self.readiness_scope != RemoteViewApplicationReadinessScope::NativeAdmission {
            return Err(RemoteViewApplicationResponseError::NativeAdmissionRequired);
        }
        if self.target != intent.target
            || self.fence.generation != intent.target.viewing_generation
            || Some(self.fence.revision) != intent.expected_revision.checked_add(1)
            || self.fence.operation_id != intent.operation_digest()?
        {
            return Err(RemoteViewApplicationResponseError::StaleTarget);
        }
        Ok(())
    }
}

impl RemoteViewIdleRetirementRead {
    fn validate(
        &self,
        assignment: &RemoteViewAssignmentRecord,
        expected: &RemoteViewApplicationTarget,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1 {
            return Err(RemoteViewApplicationResponseError::InvalidShape);
        }
        self.target.validate_assignment(assignment)?;
        if self.target != *expected {
            return Err(RemoteViewApplicationResponseError::StaleTarget);
        }
        if self.readiness_scope != RemoteViewApplicationReadinessScope::NativeAdmission {
            return Err(RemoteViewApplicationResponseError::NativeAdmissionRequired);
        }
        match &self.retirement.fence {
            Some(fence) => {
                fence.validate()?;
                if self.retirement.revision != fence.revision {
                    return Err(RemoteViewApplicationResponseError::InvalidTarget);
                }
                if !self.retirement.completed && fence.generation != expected.viewing_generation {
                    return Err(RemoteViewApplicationResponseError::StaleTarget);
                }
            }
            None if self.retirement.revision != 0 || !self.retirement.completed => {
                return Err(RemoteViewApplicationResponseError::InvalidTarget);
            }
            None => (),
        }
        Ok(())
    }
}

impl<T: RemoteViewApplicationTransport> RemoteViewApplicationAdapter<T> {
    /// Reconcile only the retained operation through native readback. Absence
    /// does not settle an unknown prepare. A known prepared fence additionally
    /// pins the installation, configuration and slot; no mutation is issued.
    pub fn reconcile_idle_retirement(
        &mut self,
        intent: &RemoteViewIdleRetirementIntent,
        prepared: Option<&RemoteViewIdleRetirementReceipt>,
    ) -> Result<RemoteViewIdleRetirementReceipt, RemoteViewApplicationAdapterError> {
        intent.validate()?;
        if let Some(prepared) = prepared {
            prepared.validate(intent, RemoteViewIdleRetirementReceiptState::Prepared)?;
        }
        let observed = self.read_idle_retirement(&intent.assignment)?;
        let fence = observed
            .retirement
            .fence
            .ok_or(RemoteViewApplicationAdapterError::MutationReadbackRequired)?;
        let state = if observed.retirement.completed {
            RemoteViewIdleRetirementReceiptState::Completed
        } else {
            RemoteViewIdleRetirementReceiptState::Prepared
        };
        let receipt = RemoteViewIdleRetirementReceipt {
            schema_version: observed.schema_version,
            target: observed.target,
            readiness_scope: observed.readiness_scope,
            state,
            fence,
        };
        receipt.validate(intent, state)?;
        if prepared.is_some_and(|prepared| prepared.fence != receipt.fence) {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        Ok(receipt)
    }

    /// Acknowledge only the runtime owner's conclusive, durably recorded
    /// settlement of all application effects. This boolean is that owner's
    /// assertion, not provider proof of a browser close. Unsettled effects and
    /// unknown transport outcomes never clear a fence or trigger a retry.
    pub fn complete_idle_retirement(
        &mut self,
        intent: &RemoteViewIdleRetirementIntent,
        prepared: &RemoteViewIdleRetirementReceipt,
        effects_settled: bool,
    ) -> Result<RemoteViewIdleRetirementReceipt, RemoteViewApplicationAdapterError> {
        if !effects_settled {
            return Err(RemoteViewApplicationAdapterError::MutationReadbackRequired);
        }
        prepared.validate(intent, RemoteViewIdleRetirementReceiptState::Prepared)?;
        if self.observe_assignment(&intent.assignment)?.target != intent.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        let response = self.request(RemoteViewApplicationRequest::CompleteIdleRetirement {
            assignment_id: intent.assignment.assignment_id.clone(),
            expected_generation: intent.target.lifecycle_generation,
            expected_viewing_generation: intent.target.viewing_generation,
            idempotency_key: intent.idempotency_key.clone(),
            expected_retirement: intent.expected_retirement.clone(),
            fence: prepared.fence.clone(),
            effects_settled: true,
        })?;
        let completed: RemoteViewIdleRetirementReceipt = serde_json::from_value(response)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        completed.validate(intent, RemoteViewIdleRetirementReceiptState::Completed)?;
        if completed.fence != prepared.fence {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        if self.observe_assignment(&intent.assignment)?.target != intent.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        Ok(completed)
    }

    /// Prepare only after the runtime owner has durably admitted this exact
    /// intent. Invalid or unknown results require its original readback; this
    /// low-level adapter neither retries nor clears an ambiguous native fence.
    pub fn prepare_idle_retirement(
        &mut self,
        intent: &RemoteViewIdleRetirementIntent,
    ) -> Result<RemoteViewIdleRetirementReceipt, RemoteViewApplicationAdapterError> {
        intent.validate()?;
        if self.observe_assignment(&intent.assignment)?.target != intent.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        let response = self.request(RemoteViewApplicationRequest::PrepareIdleRetirement {
            assignment_id: intent.assignment.assignment_id.clone(),
            expected_generation: intent.target.lifecycle_generation,
            expected_viewing_generation: intent.target.viewing_generation,
            idempotency_key: intent.idempotency_key.clone(),
            expected_revision: intent.expected_revision,
            expected_retirement: intent.expected_retirement.clone(),
        })?;
        let receipt: RemoteViewIdleRetirementReceipt = serde_json::from_value(response)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        receipt.validate(intent, RemoteViewIdleRetirementReceiptState::Prepared)?;
        if self.observe_assignment(&intent.assignment)?.target != intent.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        Ok(receipt)
    }

    /// Read through fresh exact-target observations. A native fence is not a
    /// browser-close result, and an unknown transport result never retries.
    pub fn read_idle_retirement(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<RemoteViewIdleRetirementRead, RemoteViewApplicationAdapterError> {
        let before = self.observe_assignment(assignment)?;
        let response = self.request(RemoteViewApplicationRequest::ReadIdleRetirement {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: assignment.generation,
            expected_viewing_generation: before.target.viewing_generation,
        })?;
        let read: RemoteViewIdleRetirementRead = serde_json::from_value(response)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        read.validate(assignment, &before.target)?;
        if self.observe_assignment(assignment)?.target != before.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        Ok(read)
    }
}
