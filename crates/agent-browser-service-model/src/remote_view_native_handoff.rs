//! Private provider observations for ordinary native handoffs, without grants.
//! These records qualify provider context; the browser owner must separately
//! join the still-current handoff, tab and target before consuming retention.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    RemoteViewApplicationAdapter, RemoteViewApplicationAdapterError, RemoteViewApplicationRequest,
    RemoteViewApplicationResponseError, RemoteViewApplicationTarget,
    RemoteViewApplicationTransport, RemoteViewAssignmentRecord,
};

/// Qualified producer source integrated through Remote View PR420.
pub const REMOTE_VIEW_NATIVE_HANDOFF_SOURCE_CHECKPOINT: &str =
    "701b4ab2b1aa5dd4627f1edc1a4bb78af6fdcd5f";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewNativeObservationClock {
    pub boot_id: String,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewNativeUseContext {
    pub correlation_id: String,
    pub operator: String,
    pub viewer_connection_id: String,
    pub desktop_id: String,
    pub lifecycle_generation: u64,
    pub viewing_generation: u64,
    pub target_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewNativeUseEvidence {
    pub context: RemoteViewNativeUseContext,
    pub frame_sync: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewNativeHandoffUse {
    pub evidence: RemoteViewNativeUseEvidence,
    pub recorded_at_ms: u64,
}

/// Provider-issued continuation identity. Deserialization alone is not proof;
/// only a fenced private adapter read qualifies it for the browser owner.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewNativeHandoffObservation {
    pub correlation_id: String,
    pub request_digest: String,
    pub application: String,
    pub target: RemoteViewApplicationTarget,
    pub target_digest: String,
    pub clock: RemoteViewNativeObservationClock,
    pub lifetime_ms: u64,
    pub used: Option<RemoteViewNativeHandoffUse>,
}

fn identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl RemoteViewNativeHandoffObservation {
    fn validate(&self) -> Result<(), RemoteViewApplicationResponseError> {
        if !digest(&self.correlation_id)
            || !digest(&self.request_digest)
            || !identity(&self.application)
            || !identity(&self.target_digest)
            || !identity(&self.clock.boot_id)
            || !(1..=300_000).contains(&self.lifetime_ms)
        {
            return Err(RemoteViewApplicationResponseError::InvalidShape);
        }
        if let Some(used) = &self.used {
            let context = &used.evidence.context;
            if used.recorded_at_ms == 0
                || !identity(&context.operator)
                || !identity(&context.viewer_connection_id)
                || context.correlation_id != self.correlation_id
                || context.target_digest != self.target_digest
                || context.desktop_id != self.target.viewing_desktop_id
                || context.lifecycle_generation != self.target.lifecycle_generation
                || context.viewing_generation != self.target.viewing_generation
            {
                return Err(RemoteViewApplicationResponseError::InvalidTarget);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RegistrationResponse {
    schema_version: u32,
    observation: RemoteViewNativeHandoffObservation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UseResponse {
    schema_version: u32,
    state: String,
    observation: RemoteViewNativeHandoffObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewNativeHandoffUseState {
    Pending,
    Expired,
    Used,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteViewNativeHandoffUseRead {
    pub state: RemoteViewNativeHandoffUseState,
    pub observation: RemoteViewNativeHandoffObservation,
}

impl<T: RemoteViewApplicationTransport> RemoteViewApplicationAdapter<T> {
    /// Register exact continuation context against a currently live assignment.
    /// Lost replies are returned without automatic POST retry. Replaying the
    /// same key is provider-idempotent and never extends its observation clock.
    pub fn register_native_handoff_observation(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
        target_digest: String,
        idempotency_key: String,
        lifetime_ms: u64,
    ) -> Result<RemoteViewNativeHandoffObservation, RemoteViewApplicationAdapterError> {
        if !identity(&target_digest)
            || !identity(&idempotency_key)
            || !(1..=300_000).contains(&lifetime_ms)
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let before = self.observe_assignment(assignment)?;
        let mut request_hash = Sha256::new();
        request_hash.update((self.application.len() as u64).to_be_bytes());
        request_hash.update(self.application.as_bytes());
        request_hash.update(idempotency_key.as_bytes());
        let expected_request = format!("{:x}", request_hash.finalize());
        let response: RegistrationResponse = serde_json::from_value(self.request(
            RemoteViewApplicationRequest::RegisterNativeHandoffObservation {
                assignment_id: assignment.assignment_id.clone(),
                expected_generation: before.target.lifecycle_generation,
                expected_viewing_generation: before.target.viewing_generation,
                target_digest: target_digest.clone(),
                idempotency_key,
                lifetime_ms,
            },
        )?)
        .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        let observation = response.observation;
        observation.validate()?;
        if response.schema_version != 1
            || observation.application != self.application
            || observation.target != before.target
            || observation.target_digest != target_digest
            || observation.request_digest != expected_request
            || observation.lifetime_ms != lifetime_ms
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        if self.observe_assignment(assignment)?.target != before.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        Ok(observation)
    }

    /// Read provider-qualified use, preserving the original registration and
    /// both generation fences. No caller-supplied successful-use field is sent.
    /// Pending or expired observations carry no use and cannot consume a hold.
    pub fn observe_native_handoff_use(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
        registered: &RemoteViewNativeHandoffObservation,
    ) -> Result<RemoteViewNativeHandoffObservation, RemoteViewApplicationAdapterError> {
        self.observe_native_handoff_use_state(assignment, registered)
            .map(|read| read.observation)
    }

    /// Keep the provider's boot-qualified pending/expired distinction. Local
    /// wall time cannot renew or decide a remote observation's deadline.
    pub fn observe_native_handoff_use_state(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
        registered: &RemoteViewNativeHandoffObservation,
    ) -> Result<RemoteViewNativeHandoffUseRead, RemoteViewApplicationAdapterError> {
        registered.validate()?;
        if registered.application != self.application {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let before = self.observe_assignment(assignment)?;
        if before.target != registered.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        let response = match self.request(RemoteViewApplicationRequest::ObserveNativeHandoffUse {
            correlation_id: registered.correlation_id.clone(),
            target_digest: registered.target_digest.clone(),
        }) {
            // Producer expiry may remove the short context entirely. Only this
            // exact private response allows fresh preparation. It cannot prove
            // human use, clear a hold, or extend the durable link lifetime.
            Err(RemoteViewApplicationAdapterError::Transport(
                crate::RemoteViewApplicationTransportError::NativeObservationMissing,
            )) => {
                if self.observe_assignment(assignment)?.target != before.target {
                    return Err(RemoteViewApplicationResponseError::StaleTarget.into());
                }
                return Ok(RemoteViewNativeHandoffUseRead {
                    state: RemoteViewNativeHandoffUseState::Expired,
                    observation: registered.clone(),
                });
            }
            Err(error) => return Err(error),
            Ok(value) => serde_json::from_value::<UseResponse>(value)
                .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?,
        };
        let observation = response.observation;
        observation.validate()?;
        let mut original = observation.clone();
        original.used = registered.used.clone();
        if response.schema_version != 1
            || original != *registered
            || !matches!(response.state.as_str(), "pending" | "expired" | "used")
            || (response.state == "used") != observation.used.is_some()
            || registered
                .used
                .as_ref()
                .is_some_and(|used| observation.used.as_ref() != Some(used))
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        if self.observe_assignment(assignment)?.target != before.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        let state = match response.state.as_str() {
            "used" => RemoteViewNativeHandoffUseState::Used,
            "expired" => RemoteViewNativeHandoffUseState::Expired,
            _ => RemoteViewNativeHandoffUseState::Pending,
        };
        Ok(RemoteViewNativeHandoffUseRead { state, observation })
    }
}
