//! Durable Service challenge envelope and deterministic provider-free decisions.
//!
//! Callers validate request syntax, registered identifiers, and the current
//! Service tab join before calling this module. Persistence, clocks, browser
//! control, providers, and all runtime effects remain adapter concerns.

use crate::{LeaseState, ServiceTabHandle};
use agent_browser_challenge_control::{
    admit_challenge_consumer as decide_consumer_admission, challenge_profile,
    execute_provider_free_task, ChallengeConsumerAdmissionError, ChallengeConsumerAdmissionReceipt,
    ChallengeConsumerAdmissionRequest, ChallengeConsumerEvidence, ChallengeTaskError,
    ChallengeTaskFixture, ChallengeTaskRequest,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SERVICE_CHALLENGE_TASK_SCHEMA_VERSION: &str = "agent-browser.service-challenge-task.v1";
/// Registered downstream intent used when Authentication Run consumes a task.
pub const AUTHENTICATION_CHALLENGE_INTENT_ID: &str = "authentication-run-start";
/// Registered downstream intent used when navigation consumes a task.
pub const NAVIGATION_CHALLENGE_INTENT_ID: &str = "navigation-dispatch";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceChallengeTaskState {
    #[default]
    Ready,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceChallengeTaskSummary {
    pub total_count: usize,
    pub active_count: usize,
    pub terminal_count: usize,
    pub cooldown_count: usize,
    pub intervention_count: usize,
    pub pending_effect_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PendingChallengeTaskEffect {
    operation_id_sha256: String,
    reserved_at: String,
    effect_kind: ChallengeTaskEffectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum ChallengeTaskEffectKind {
    RegisteredResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceChallengeTaskRecord {
    pub schema_version: String,
    pub task_id: String,
    pub request_sha256: String,
    pub idempotency_key_sha256: String,
    pub created_at: String,
    pub deadline_at: String,
    pub service_name: String,
    pub agent_name: String,
    pub task_name: String,
    pub principal_id: String,
    pub challenge_profile_id: String,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
    pub fixture: ChallengeTaskFixture,
    pub max_transitions: u8,
    pub service_tab_handle: ServiceTabHandle,
    #[serde(default)]
    pub state: ServiceChallengeTaskState,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub used_operation_id_sha256s: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_effect: Option<PendingChallengeTaskEffect>,
    /// The durable envelope intentionally retains the Challenge Control receipt
    /// as opaque JSON. Authority decisions decode it into typed evidence first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceChallengeTaskStartInput {
    pub service_name: String,
    pub agent_name: String,
    pub task_name: String,
    pub principal_id: String,
    pub challenge_profile_id: String,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
    pub fixture: ChallengeTaskFixture,
    pub idempotency_key: String,
    pub deadline_ms: u64,
    pub max_transitions: u8,
    /// Already joined by the Service adapter; this model never looks up tabs.
    pub current_service_tab_handle: ServiceTabHandle,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedServiceChallengeTaskStart {
    input: ServiceChallengeTaskStartInput,
    pub request_sha256: String,
    pub idempotency_key_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceChallengeTaskStartDecision {
    Replayed(Box<ServiceChallengeTaskRecord>),
    Create(Box<PreparedServiceChallengeTaskStart>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceChallengeTaskResumeInput {
    pub task_id: String,
    pub principal_id: String,
    pub operation_id: String,
    pub resumed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedServiceChallengeTaskResume {
    record: ServiceChallengeTaskRecord,
    operation_id_sha256: String,
    resumed_at: String,
}

impl PreparedServiceChallengeTaskResume {
    /// The adapter uses this exact retained-tab key only after preparation has
    /// established replay, terminal-state, timestamp, and deadline ordering.
    pub fn service_tab_id(&self) -> &str {
        &self.record.service_tab_handle.tab_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceChallengeTaskResumeDecision {
    Replayed(Box<ServiceChallengeTaskRecord>),
    Execute(Box<PreparedServiceChallengeTaskResume>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceChallengeTaskCancelInput {
    pub task_id: String,
    pub principal_id: String,
    pub operation_id: String,
    pub cancelled_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceChallengeTaskCancelDecision {
    Replayed(Box<ServiceChallengeTaskRecord>),
    Cancelled(Box<ServiceChallengeTaskRecord>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceChallengeTaskError {
    HashFailed(String),
    IdempotencyConflict,
    CreatedAtInvalid,
    DeadlineInvalid,
    TaskNotFound,
    PrincipalMismatch,
    NotResumable,
    ResumedAtInvalid,
    DeadlineExceeded,
    ServiceTabHandleMismatch,
    ProfileNotRegistered,
    ExecutionInvalid,
    TransitionBudgetExceeded,
    ReceiptSerializationFailed(String),
    NotCancellable,
    CancelledAtInvalid,
    ConsumerReceiptMissing,
    ConsumerReceiptInvalid(String),
    ConsumerReceiptRejected,
    ConsumerRequestInvalid,
    ConsumerSitePolicyMismatch,
    ConsumerDownstreamIntentMismatch,
    ConsumerAdmissionWithheld,
}

impl ServiceChallengeTaskError {
    pub fn cli_message(&self) -> String {
        match self {
            Self::HashFailed(error) => format!("challenge_task_hash_failed:{error}"),
            Self::IdempotencyConflict => "challenge_task_idempotency_conflict".to_string(),
            Self::CreatedAtInvalid => "challenge_task_created_at_invalid".to_string(),
            Self::DeadlineInvalid => "challenge_task_deadline_invalid".to_string(),
            Self::TaskNotFound => "challenge_task_not_found".to_string(),
            Self::PrincipalMismatch => "challenge_task_principal_mismatch".to_string(),
            Self::NotResumable => "challenge_task_not_resumable".to_string(),
            Self::ResumedAtInvalid => "challenge_task_resumed_at_invalid".to_string(),
            Self::DeadlineExceeded => "challenge_task_deadline_exceeded".to_string(),
            Self::ServiceTabHandleMismatch => {
                "challenge_task_service_tab_handle_mismatch".to_string()
            }
            Self::ProfileNotRegistered => "challenge_task_profile_not_registered".to_string(),
            Self::ExecutionInvalid => "challenge_task_execution_invalid".to_string(),
            Self::TransitionBudgetExceeded => {
                "challenge_task_transition_budget_exceeded".to_string()
            }
            Self::ReceiptSerializationFailed(error) => {
                format!("challenge_task_receipt_serialization_failed:{error}")
            }
            Self::NotCancellable => "challenge_task_not_cancellable".to_string(),
            Self::CancelledAtInvalid => "challenge_task_cancelled_at_invalid".to_string(),
            Self::ConsumerReceiptMissing => "challenge_consumer_receipt_missing".to_string(),
            Self::ConsumerReceiptInvalid(error) => {
                format!("challenge_consumer_receipt_invalid:{error}")
            }
            Self::ConsumerReceiptRejected => "challenge_consumer_receipt_invalid".to_string(),
            Self::ConsumerRequestInvalid => "challenge_consumer_request_invalid".to_string(),
            Self::ConsumerSitePolicyMismatch => {
                "challenge_consumer_site_policy_mismatch".to_string()
            }
            Self::ConsumerDownstreamIntentMismatch => {
                "challenge_consumer_downstream_intent_mismatch".to_string()
            }
            Self::ConsumerAdmissionWithheld => "challenge_consumer_admission_withheld".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceChallengeTaskProjection {
    pub schema_version: String,
    pub challenge_task_id: String,
    pub state: ServiceChallengeTaskState,
    pub created_at: String,
    pub deadline_at: String,
    pub request_sha256: String,
    pub challenge_profile_id: String,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
    pub browser_id: String,
    pub session_name: Option<String>,
    pub tab_id: String,
    pub transition_count: usize,
    pub effect_pending: bool,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub receipt: Option<Value>,
    pub replayed: bool,
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, ServiceChallengeTaskError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ServiceChallengeTaskError::HashFailed(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn record_for_principal<'a>(
    records: &'a BTreeMap<String, ServiceChallengeTaskRecord>,
    task_id: &str,
    principal_id: &str,
) -> Result<&'a ServiceChallengeTaskRecord, ServiceChallengeTaskError> {
    let record = records
        .get(task_id)
        .ok_or(ServiceChallengeTaskError::TaskNotFound)?;
    if record.principal_id == principal_id {
        Ok(record)
    } else {
        Err(ServiceChallengeTaskError::PrincipalMismatch)
    }
}

pub fn prepare_service_challenge_task_start(
    existing: &BTreeMap<String, ServiceChallengeTaskRecord>,
    input: ServiceChallengeTaskStartInput,
) -> Result<ServiceChallengeTaskStartDecision, ServiceChallengeTaskError> {
    let idempotency_key_sha256 = canonical_sha256(&input.idempotency_key)?;
    let request_sha256 = canonical_sha256(&(
        &input.service_name,
        &input.agent_name,
        &input.task_name,
        &input.principal_id,
        &input.challenge_profile_id,
        &input.site_policy_digest,
        &input.downstream_intent_id,
        input.fixture,
        &idempotency_key_sha256,
        input.deadline_ms,
        input.max_transitions,
        &input.current_service_tab_handle,
    ))?;
    if let Some(record) = existing
        .values()
        .find(|record| record.idempotency_key_sha256 == idempotency_key_sha256)
    {
        if record.request_sha256 != request_sha256 {
            return Err(ServiceChallengeTaskError::IdempotencyConflict);
        }
        return Ok(ServiceChallengeTaskStartDecision::Replayed(Box::new(
            record.clone(),
        )));
    }
    Ok(ServiceChallengeTaskStartDecision::Create(Box::new(
        PreparedServiceChallengeTaskStart {
            input,
            request_sha256,
            idempotency_key_sha256,
        },
    )))
}

pub fn complete_service_challenge_task_start(
    prepared: PreparedServiceChallengeTaskStart,
) -> Result<ServiceChallengeTaskRecord, ServiceChallengeTaskError> {
    let created = DateTime::parse_from_rfc3339(&prepared.input.created_at)
        .map_err(|_| ServiceChallengeTaskError::CreatedAtInvalid)?
        .with_timezone(&Utc);
    let deadline_delta = i64::try_from(prepared.input.deadline_ms)
        .map_err(|_| ServiceChallengeTaskError::DeadlineInvalid)?;
    let task_id = format!("challenge-task-{}", &prepared.request_sha256[..24]);
    Ok(ServiceChallengeTaskRecord {
        schema_version: SERVICE_CHALLENGE_TASK_SCHEMA_VERSION.to_string(),
        task_id,
        request_sha256: prepared.request_sha256,
        idempotency_key_sha256: prepared.idempotency_key_sha256,
        created_at: created.to_rfc3339(),
        deadline_at: (created + Duration::milliseconds(deadline_delta)).to_rfc3339(),
        service_name: prepared.input.service_name,
        agent_name: prepared.input.agent_name,
        task_name: prepared.input.task_name,
        principal_id: prepared.input.principal_id,
        challenge_profile_id: prepared.input.challenge_profile_id,
        site_policy_digest: prepared.input.site_policy_digest,
        downstream_intent_id: prepared.input.downstream_intent_id,
        fixture: prepared.input.fixture,
        max_transitions: prepared.input.max_transitions,
        service_tab_handle: prepared.input.current_service_tab_handle,
        state: ServiceChallengeTaskState::Ready,
        used_operation_id_sha256s: BTreeSet::new(),
        completed_at: None,
        cancelled_at: None,
        pending_effect: None,
        receipt: None,
    })
}

pub fn service_challenge_task_status(
    records: &BTreeMap<String, ServiceChallengeTaskRecord>,
    task_id: &str,
    principal_id: &str,
) -> Result<ServiceChallengeTaskRecord, ServiceChallengeTaskError> {
    Ok(record_for_principal(records, task_id, principal_id)?.clone())
}

/// Preserve replay, terminal, timestamp, and deadline ordering before an
/// adapter performs the exact current-tab join.
pub fn prepare_service_challenge_task_resume(
    records: &BTreeMap<String, ServiceChallengeTaskRecord>,
    input: ServiceChallengeTaskResumeInput,
) -> Result<ServiceChallengeTaskResumeDecision, ServiceChallengeTaskError> {
    let snapshot = record_for_principal(records, &input.task_id, &input.principal_id)?.clone();
    let operation_id_sha256 = canonical_sha256(&input.operation_id)?;
    if snapshot
        .used_operation_id_sha256s
        .contains(&operation_id_sha256)
    {
        return Ok(ServiceChallengeTaskResumeDecision::Replayed(Box::new(
            snapshot,
        )));
    }
    if snapshot.state != ServiceChallengeTaskState::Ready {
        return Err(ServiceChallengeTaskError::NotResumable);
    }
    let resumed = DateTime::parse_from_rfc3339(&input.resumed_at)
        .map_err(|_| ServiceChallengeTaskError::ResumedAtInvalid)?;
    let deadline = DateTime::parse_from_rfc3339(&snapshot.deadline_at)
        .map_err(|_| ServiceChallengeTaskError::DeadlineInvalid)?;
    if resumed > deadline {
        return Err(ServiceChallengeTaskError::DeadlineExceeded);
    }
    Ok(ServiceChallengeTaskResumeDecision::Execute(Box::new(
        PreparedServiceChallengeTaskResume {
            record: snapshot,
            operation_id_sha256,
            resumed_at: resumed.to_rfc3339(),
        },
    )))
}

/// Complete a prepared provider-free transition after the adapter has joined
/// and supplied the current exact Service tab handle.
pub fn complete_service_challenge_task_resume(
    prepared: PreparedServiceChallengeTaskResume,
    current_service_tab_handle: ServiceTabHandle,
) -> Result<ServiceChallengeTaskRecord, ServiceChallengeTaskError> {
    if !exact_current_record_handle(&prepared.record, &current_service_tab_handle) {
        return Err(ServiceChallengeTaskError::ServiceTabHandleMismatch);
    }
    let profile = challenge_profile(&prepared.record.challenge_profile_id)
        .copied()
        .ok_or(ServiceChallengeTaskError::ProfileNotRegistered)?;
    let receipt = execute_provider_free_task(ChallengeTaskRequest {
        task_id: prepared.record.task_id.clone(),
        profile,
        site_policy_digest: prepared.record.site_policy_digest.clone(),
        downstream_intent_id: prepared.record.downstream_intent_id.clone(),
        fixture: prepared.record.fixture,
        max_transitions: prepared.record.max_transitions,
    })
    .map_err(|error| match error {
        ChallengeTaskError::InvalidRequest => ServiceChallengeTaskError::ExecutionInvalid,
        ChallengeTaskError::TransitionBudgetExceeded => {
            ServiceChallengeTaskError::TransitionBudgetExceeded
        }
    })?;
    let receipt = serde_json::to_value(receipt).map_err(|error| {
        ServiceChallengeTaskError::ReceiptSerializationFailed(error.to_string())
    })?;
    let mut record = prepared.record;
    record
        .used_operation_id_sha256s
        .insert(prepared.operation_id_sha256);
    record.state = ServiceChallengeTaskState::Completed;
    record.completed_at = Some(prepared.resumed_at);
    record.receipt = Some(receipt);
    Ok(record)
}

fn exact_current_record_handle(
    record: &ServiceChallengeTaskRecord,
    current: &ServiceTabHandle,
) -> bool {
    let supplied = &record.service_tab_handle;
    let trace = &current.trace_filter;
    let controlled_lease = matches!(
        current.lease_state,
        Some(LeaseState::Shared | LeaseState::Exclusive)
    );
    let principal_matches = current
        .profile_access
        .as_ref()
        .and_then(|access| access.subject_id.as_deref())
        == Some(record.principal_id.as_str());
    supplied.valid
        && current.valid
        && controlled_lease
        && principal_matches
        && supplied.browser_id == current.browser_id
        && supplied.session_name == current.session_name
        && supplied.tab_id == current.tab_id
        && supplied.target_id == current.target_id
        && supplied.profile_id == current.profile_id
        && trace.service_name.as_deref() == Some(record.service_name.as_str())
        && trace.agent_name.as_deref() == Some(record.agent_name.as_str())
        && trace.task_name.as_deref() == Some(record.task_name.as_str())
}

pub fn cancel_service_challenge_task(
    records: &BTreeMap<String, ServiceChallengeTaskRecord>,
    input: ServiceChallengeTaskCancelInput,
) -> Result<ServiceChallengeTaskCancelDecision, ServiceChallengeTaskError> {
    let mut record = record_for_principal(records, &input.task_id, &input.principal_id)?.clone();
    let operation_id_sha256 = canonical_sha256(&input.operation_id)?;
    if record
        .used_operation_id_sha256s
        .contains(&operation_id_sha256)
    {
        return Ok(ServiceChallengeTaskCancelDecision::Replayed(Box::new(
            record,
        )));
    }
    if record.state != ServiceChallengeTaskState::Ready {
        return Err(ServiceChallengeTaskError::NotCancellable);
    }
    let cancelled = DateTime::parse_from_rfc3339(&input.cancelled_at)
        .map_err(|_| ServiceChallengeTaskError::CancelledAtInvalid)?;
    record.used_operation_id_sha256s.insert(operation_id_sha256);
    record.state = ServiceChallengeTaskState::Cancelled;
    record.cancelled_at = Some(cancelled.to_rfc3339());
    Ok(ServiceChallengeTaskCancelDecision::Cancelled(Box::new(
        record,
    )))
}

pub fn project_service_challenge_task(
    record: &ServiceChallengeTaskRecord,
    replayed: bool,
) -> ServiceChallengeTaskProjection {
    ServiceChallengeTaskProjection {
        schema_version: record.schema_version.clone(),
        challenge_task_id: record.task_id.clone(),
        state: record.state,
        created_at: record.created_at.clone(),
        deadline_at: record.deadline_at.clone(),
        request_sha256: record.request_sha256.clone(),
        challenge_profile_id: record.challenge_profile_id.clone(),
        site_policy_digest: record.site_policy_digest.clone(),
        downstream_intent_id: record.downstream_intent_id.clone(),
        browser_id: record.service_tab_handle.browser_id.clone(),
        session_name: record.service_tab_handle.session_name.clone(),
        tab_id: record.service_tab_handle.tab_id.clone(),
        transition_count: record
            .receipt
            .as_ref()
            .and_then(|receipt| receipt.get("phases"))
            .and_then(Value::as_array)
            .map(|phases| phases.len().saturating_sub(1))
            .unwrap_or(0),
        effect_pending: record.pending_effect.is_some(),
        completed_at: record.completed_at.clone(),
        cancelled_at: record.cancelled_at.clone(),
        receipt: record.receipt.clone(),
        replayed,
    }
}

pub fn challenge_task_map_is_empty(value: &BTreeMap<String, ServiceChallengeTaskRecord>) -> bool {
    value.is_empty()
}

pub fn challenge_task_summary(
    records: &BTreeMap<String, ServiceChallengeTaskRecord>,
) -> ServiceChallengeTaskSummary {
    let values = records.values();
    ServiceChallengeTaskSummary {
        total_count: values.clone().count(),
        active_count: values
            .clone()
            .filter(|record| record.state == ServiceChallengeTaskState::Ready)
            .count(),
        terminal_count: values
            .clone()
            .filter(|record| record.state != ServiceChallengeTaskState::Ready)
            .count(),
        cooldown_count: values
            .clone()
            .filter(|record| {
                record.receipt.as_ref().is_some_and(|receipt| {
                    receipt.get("cooldown").and_then(Value::as_str) == Some("active")
                })
            })
            .count(),
        intervention_count: values
            .clone()
            .filter(|record| {
                record.receipt.as_ref().is_some_and(|receipt| {
                    receipt
                        .get("intervention")
                        .is_some_and(|value| !value.is_null())
                })
            })
            .count(),
        pending_effect_count: values
            .filter(|record| record.pending_effect.is_some())
            .count(),
    }
}

/// Decode the opaque durable receipt before delegating one typed admission
/// decision to Challenge Control. The adapter preserves task, principal, tab,
/// and policy ordering before invoking this narrow helper.
pub fn admit_challenge_consumer_from_receipt(
    receipt: Option<&Value>,
    request: ChallengeConsumerAdmissionRequest,
) -> Result<ChallengeConsumerAdmissionReceipt, ServiceChallengeTaskError> {
    let evidence: ChallengeConsumerEvidence = serde_json::from_value(
        receipt
            .cloned()
            .ok_or(ServiceChallengeTaskError::ConsumerReceiptMissing)?,
    )
    .map_err(|error| ServiceChallengeTaskError::ConsumerReceiptInvalid(error.to_string()))?;
    decide_consumer_admission(&evidence, request).map_err(|error| match error {
        ChallengeConsumerAdmissionError::InvalidRequest => {
            ServiceChallengeTaskError::ConsumerRequestInvalid
        }
        ChallengeConsumerAdmissionError::InvalidReceipt => {
            ServiceChallengeTaskError::ConsumerReceiptRejected
        }
        ChallengeConsumerAdmissionError::SitePolicyMismatch => {
            ServiceChallengeTaskError::ConsumerSitePolicyMismatch
        }
        ChallengeConsumerAdmissionError::DownstreamIntentMismatch => {
            ServiceChallengeTaskError::ConsumerDownstreamIntentMismatch
        }
        ChallengeConsumerAdmissionError::ChallengeWithheld => {
            ServiceChallengeTaskError::ConsumerAdmissionWithheld
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_challenge_control::{ChallengeConsumerKind, ChallengeTaskFixture};
    use serde_json::json;

    fn handle() -> ServiceTabHandle {
        ServiceTabHandle {
            browser_id: "browser-1".to_string(),
            session_name: Some("session-1".to_string()),
            tab_id: "tab-1".to_string(),
            target_id: Some("target-1".to_string()),
            profile_id: Some("profile-1".to_string()),
            lease_state: Some(LeaseState::Exclusive),
            profile_access: Some(crate::ProfileChildAccess {
                subject_id: Some("principal-1".to_string()),
                ..crate::ProfileChildAccess::default()
            }),
            trace_filter: crate::ServiceTabHandleTraceFilter {
                service_name: Some("consumer-service".to_string()),
                agent_name: Some("challenge-worker".to_string()),
                task_name: Some("challenge-aware-task".to_string()),
                ..crate::ServiceTabHandleTraceFilter::default()
            },
            valid: true,
            ..ServiceTabHandle::default()
        }
    }

    fn start_input(key: &str) -> ServiceChallengeTaskStartInput {
        ServiceChallengeTaskStartInput {
            service_name: "consumer-service".to_string(),
            agent_name: "challenge-worker".to_string(),
            task_name: "challenge-aware-task".to_string(),
            principal_id: "principal-1".to_string(),
            challenge_profile_id: "turnstile-checkbox-p169-v1".to_string(),
            site_policy_digest: "a".repeat(64),
            downstream_intent_id: "authenticate-account".to_string(),
            fixture: ChallengeTaskFixture::PassAfterAcknowledgedResolution,
            idempotency_key: key.to_string(),
            deadline_ms: 120_000,
            max_transitions: 8,
            current_service_tab_handle: handle(),
            created_at: "2026-09-16T00:00:00Z".to_string(),
        }
    }

    fn create_record(key: &str) -> ServiceChallengeTaskRecord {
        let ServiceChallengeTaskStartDecision::Create(prepared) =
            prepare_service_challenge_task_start(&BTreeMap::new(), start_input(key)).unwrap()
        else {
            panic!("new input unexpectedly replayed");
        };
        complete_service_challenge_task_start(*prepared).unwrap()
    }

    #[test]
    fn record_wire_defaults_opaque_receipt_and_rejects_unknown_fields() {
        let record = create_record("raw-idempotency-canary");
        let wire = serde_json::to_value(&record).unwrap();
        let decoded: ServiceChallengeTaskRecord = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(decoded, record);
        assert!(wire.get("receipt").is_none());
        assert!(wire.get("usedOperationIdSha256s").is_none());
        let mut unknown = wire;
        unknown["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ServiceChallengeTaskRecord>(unknown).is_err());
        let defaulted: ServiceChallengeTaskRecord = serde_json::from_value(json!({
            "schemaVersion": SERVICE_CHALLENGE_TASK_SCHEMA_VERSION,
            "taskId": "task-1", "requestSha256": "request", "idempotencyKeySha256": "key",
            "createdAt": "2026-09-16T00:00:00+00:00", "deadlineAt": "2026-09-16T00:01:00+00:00",
            "serviceName": "service", "agentName": "agent", "taskName": "task",
            "principalId": "principal", "challengeProfileId": "profile", "sitePolicyDigest": "a",
            "downstreamIntentId": "intent", "fixture": "pass_after_acknowledged_resolution",
            "maxTransitions": 1, "serviceTabHandle": handle()
        }))
        .unwrap();
        assert_eq!(defaulted.state, ServiceChallengeTaskState::Ready);
        assert!(defaulted.receipt.is_none());
    }

    #[test]
    fn stable_hash_id_start_replay_and_conflict_are_deterministic() {
        let record = create_record("stable-key");
        assert_eq!(
            record.task_id,
            format!("challenge-task-{}", &record.request_sha256[..24])
        );
        let records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        assert_eq!(
            prepare_service_challenge_task_start(&records, start_input("stable-key")).unwrap(),
            ServiceChallengeTaskStartDecision::Replayed(Box::new(record))
        );
        let mut changed = start_input("stable-key");
        changed.deadline_ms += 1;
        assert_eq!(
            prepare_service_challenge_task_start(&records, changed).unwrap_err(),
            ServiceChallengeTaskError::IdempotencyConflict
        );
    }

    #[test]
    fn resume_preserves_replay_then_terminal_timestamp_deadline_and_execution_order() {
        let record = create_record("resume-key");
        let records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        let input = ServiceChallengeTaskResumeInput {
            task_id: record.task_id.clone(),
            principal_id: "principal-1".to_string(),
            operation_id: "resume-1".to_string(),
            resumed_at: "2026-09-16T01:01:00+01:00".to_string(),
        };
        let ServiceChallengeTaskResumeDecision::Execute(prepared) =
            prepare_service_challenge_task_resume(&records, input).unwrap()
        else {
            panic!()
        };
        let completed = complete_service_challenge_task_resume(*prepared, handle()).unwrap();
        assert_eq!(completed.state, ServiceChallengeTaskState::Completed);
        assert_eq!(
            completed.completed_at.as_deref(),
            Some("2026-09-16T01:01:00+01:00")
        );
        let completed_records = BTreeMap::from([(completed.task_id.clone(), completed.clone())]);
        let replay = prepare_service_challenge_task_resume(
            &completed_records,
            ServiceChallengeTaskResumeInput {
                task_id: completed.task_id.clone(),
                principal_id: "principal-1".to_string(),
                operation_id: "resume-1".to_string(),
                resumed_at: "invalid".to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            replay,
            ServiceChallengeTaskResumeDecision::Replayed(Box::new(completed.clone()))
        );
        assert_eq!(
            prepare_service_challenge_task_resume(
                &completed_records,
                ServiceChallengeTaskResumeInput {
                    task_id: completed.task_id.clone(),
                    principal_id: "principal-1".to_string(),
                    operation_id: "resume-2".to_string(),
                    resumed_at: "invalid".to_string(),
                }
            )
            .unwrap_err(),
            ServiceChallengeTaskError::NotResumable
        );
        let ready_records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        assert_eq!(
            prepare_service_challenge_task_resume(
                &ready_records,
                ServiceChallengeTaskResumeInput {
                    task_id: record.task_id.clone(),
                    principal_id: "principal-1".to_string(),
                    operation_id: "late".to_string(),
                    resumed_at: "2026-09-16T00:03:00Z".to_string(),
                }
            )
            .unwrap_err(),
            ServiceChallengeTaskError::DeadlineExceeded
        );
    }

    #[test]
    fn resume_requires_the_adapter_joined_current_handle() {
        let record = create_record("join-key");
        let records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        let ServiceChallengeTaskResumeDecision::Execute(prepared) =
            prepare_service_challenge_task_resume(
                &records,
                ServiceChallengeTaskResumeInput {
                    task_id: record.task_id,
                    principal_id: "principal-1".to_string(),
                    operation_id: "resume".to_string(),
                    resumed_at: "2026-09-16T00:01:00Z".to_string(),
                },
            )
            .unwrap()
        else {
            panic!()
        };
        let mut wrong = handle();
        wrong.tab_id = "tab-2".to_string();
        assert_eq!(
            complete_service_challenge_task_resume(*prepared, wrong).unwrap_err(),
            ServiceChallengeTaskError::ServiceTabHandleMismatch
        );
    }

    #[test]
    fn resume_selected_handle_predicate_requires_lease_principal_and_trace() {
        let record = create_record("predicate-key");
        let records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        for current in [
            {
                let mut value = handle();
                value.lease_state = None;
                value
            },
            {
                let mut value = handle();
                value.profile_access.as_mut().unwrap().subject_id = Some("other".to_string());
                value
            },
            {
                let mut value = handle();
                value.trace_filter.task_name = Some("other".to_string());
                value
            },
        ] {
            let ServiceChallengeTaskResumeDecision::Execute(prepared) =
                prepare_service_challenge_task_resume(
                    &records,
                    ServiceChallengeTaskResumeInput {
                        task_id: record.task_id.clone(),
                        principal_id: "principal-1".to_string(),
                        operation_id: format!("predicate-{}", current.tab_id),
                        resumed_at: "2026-09-16T00:01:00Z".to_string(),
                    },
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(
                complete_service_challenge_task_resume(*prepared, current).unwrap_err(),
                ServiceChallengeTaskError::ServiceTabHandleMismatch
            );
        }
    }

    #[test]
    fn cancel_replays_before_terminal_and_timestamp_checks() {
        let record = create_record("cancel-key");
        let records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        let cancelled = cancel_service_challenge_task(
            &records,
            ServiceChallengeTaskCancelInput {
                task_id: record.task_id.clone(),
                principal_id: "principal-1".to_string(),
                operation_id: "cancel-1".to_string(),
                cancelled_at: "2026-09-16T01:01:00+01:00".to_string(),
            },
        )
        .unwrap();
        let ServiceChallengeTaskCancelDecision::Cancelled(cancelled) = cancelled else {
            panic!()
        };
        assert_eq!(
            cancelled.cancelled_at.as_deref(),
            Some("2026-09-16T01:01:00+01:00")
        );
        let cancelled_records = BTreeMap::from([(cancelled.task_id.clone(), (*cancelled).clone())]);
        assert!(matches!(
            cancel_service_challenge_task(
                &cancelled_records,
                ServiceChallengeTaskCancelInput {
                    task_id: cancelled.task_id.clone(),
                    principal_id: "principal-1".to_string(),
                    operation_id: "cancel-1".to_string(),
                    cancelled_at: "invalid".to_string(),
                }
            )
            .unwrap(),
            ServiceChallengeTaskCancelDecision::Replayed(_)
        ));
        assert_eq!(
            cancel_service_challenge_task(
                &cancelled_records,
                ServiceChallengeTaskCancelInput {
                    task_id: cancelled.task_id.clone(),
                    principal_id: "principal-1".to_string(),
                    operation_id: "cancel-2".to_string(),
                    cancelled_at: "invalid".to_string(),
                }
            )
            .unwrap_err(),
            ServiceChallengeTaskError::NotCancellable
        );
    }

    #[test]
    fn projection_redacts_sensitive_envelope_fields_and_summary_is_stable() {
        let mut record = create_record("raw-idempotency-canary");
        record.receipt = Some(
            json!({"phases": [{}, {}], "cooldown": "active", "intervention": {"kind": "human"}}),
        );
        let encoded =
            serde_json::to_string(&project_service_challenge_task(&record, false)).unwrap();
        for secret in [
            "raw-idempotency-canary",
            "consumer-service",
            "challenge-worker",
            "challenge-aware-task",
            "principal-1",
        ] {
            assert!(!encoded.contains(secret));
        }
        let records = BTreeMap::from([(record.task_id.clone(), record)]);
        assert!(!challenge_task_map_is_empty(&records));
        assert_eq!(
            challenge_task_summary(&records),
            ServiceChallengeTaskSummary {
                total_count: 1,
                active_count: 1,
                terminal_count: 0,
                cooldown_count: 1,
                intervention_count: 1,
                pending_effect_count: 0
            }
        );
        assert!(challenge_task_map_is_empty(&BTreeMap::new()));
    }

    #[test]
    fn malformed_consumer_evidence_fails_closed_before_admission() {
        let error = admit_challenge_consumer_from_receipt(
            Some(&json!({"not": "evidence"})),
            ChallengeConsumerAdmissionRequest {
                consumer: ChallengeConsumerKind::Authentication,
                consumer_operation_id: "operation-1".to_string(),
                expected_site_policy_digest: "a".repeat(64),
                expected_downstream_intent_id: "intent-1".to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ServiceChallengeTaskError::ConsumerReceiptInvalid(_)
        ));
        assert_eq!(
            admit_challenge_consumer_from_receipt(
                None,
                ChallengeConsumerAdmissionRequest {
                    consumer: ChallengeConsumerKind::Authentication,
                    consumer_operation_id: "operation-1".to_string(),
                    expected_site_policy_digest: "a".repeat(64),
                    expected_downstream_intent_id: "intent-1".to_string(),
                }
            )
            .unwrap_err(),
            ServiceChallengeTaskError::ConsumerReceiptMissing
        );
    }

    #[test]
    fn receipt_decode_and_challenge_control_invalid_receipt_keep_distinct_wire_messages() {
        assert!(admit_challenge_consumer_from_receipt(
            Some(&json!({"not": "evidence"})),
            ChallengeConsumerAdmissionRequest {
                consumer: ChallengeConsumerKind::Authentication,
                consumer_operation_id: "operation-1".to_string(),
                expected_site_policy_digest: "a".repeat(64),
                expected_downstream_intent_id: "intent-1".to_string(),
            }
        )
        .unwrap_err()
        .cli_message()
        .starts_with("challenge_consumer_receipt_invalid:"));
        let record = create_record("receipt-rejected");
        let records = BTreeMap::from([(record.task_id.clone(), record.clone())]);
        let ServiceChallengeTaskResumeDecision::Execute(prepared) =
            prepare_service_challenge_task_resume(
                &records,
                ServiceChallengeTaskResumeInput {
                    task_id: record.task_id,
                    principal_id: "principal-1".to_string(),
                    operation_id: "resume".to_string(),
                    resumed_at: "2026-09-16T00:01:00Z".to_string(),
                },
            )
            .unwrap()
        else {
            panic!()
        };
        let mut completed = complete_service_challenge_task_resume(*prepared, handle()).unwrap();
        completed.receipt.as_mut().unwrap()["schemaVersion"] = json!("not-a-challenge-receipt");
        assert_eq!(
            admit_challenge_consumer_from_receipt(
                completed.receipt.as_ref(),
                ChallengeConsumerAdmissionRequest {
                    consumer: ChallengeConsumerKind::Authentication,
                    consumer_operation_id: "operation-1".to_string(),
                    expected_site_policy_digest: "a".repeat(64),
                    expected_downstream_intent_id: "authenticate-account".to_string(),
                }
            )
            .unwrap_err()
            .cli_message(),
            "challenge_consumer_receipt_invalid"
        );
    }
}
