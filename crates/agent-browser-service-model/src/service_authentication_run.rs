//! Durable Service authentication envelope and deterministic decisions.
//!
//! The caller supplies validated Service bindings, current tab identity,
//! challenge admission, and observation times. Persistence, clocks, browser
//! input, credentials, providers, and effect execution remain adapter concerns.

use crate::ServiceTabHandle;
use agent_browser_authentication_control::{
    AuthenticationActionContext, AuthenticationActionFailure, AuthenticationActionKind,
    AuthenticationActionReceipt, AuthenticationRun, AuthenticationRunBinding,
    AuthenticationRunError, ResponseOnlyAuthenticationAction, ResponseOnlySiteLoginAction,
    SiteLoginActionContext, SiteLoginActionReceipt,
};
use agent_browser_challenge_control::{ChallengeConsumerAdmissionReceipt, ChallengeConsumerKind};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION: &str =
    "agent-browser.service-authentication-run.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceAuthenticationRunRecord {
    pub schema_version: String,
    pub request_sha256: String,
    pub idempotency_key_sha256: String,
    pub created_at: String,
    pub deadline_at: String,
    pub service_tab_handle: ServiceTabHandle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_consumer_admission: Option<ChallengeConsumerAdmissionReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_effect: Option<PendingAuthenticationEffect>,
    pub run: AuthenticationRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingAuthenticationEffect {
    pub operation_id: String,
    pub action: AuthenticationActionKind,
    pub state_instance_id: String,
    pub reserved_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAuthenticationRunStartInput {
    pub binding: AuthenticationRunBinding,
    pub idempotency_key: String,
    pub challenge_task_id: Option<String>,
    pub site_policy_id: Option<String>,
    pub deadline_ms: u64,
    pub max_transitions: u32,
    pub current_service_tab_handle: ServiceTabHandle,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedServiceAuthenticationRunStart {
    input: ServiceAuthenticationRunStartInput,
    pub request_sha256: String,
    pub idempotency_key_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAuthenticationRunStartDecision {
    Replayed(Box<ServiceAuthenticationRunRecord>),
    Create(Box<PreparedServiceAuthenticationRunStart>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAuthenticationRunCompletion {
    Completed,
    TransitionFailed(AuthenticationRunError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAuthenticationRunError {
    HashFailed(String),
    IdempotencyConflict,
    ChallengeAdmissionIncomplete,
    CreatedAtInvalid,
    DeadlineInvalid,
    ObservedAtInvalid,
    AuthenticationRunInvalid(AuthenticationRunError),
    DeadlineExpired,
    EffectOutcomeUnknown,
    OperationReplay,
    PendingEffectMissing,
    PendingEffectMismatch,
    CancelFailed(AuthenticationRunError),
}

impl ServiceAuthenticationRunError {
    pub fn cli_message(&self) -> String {
        match self {
            Self::HashFailed(error) => format!("authentication_run_hash_failed:{error}"),
            Self::IdempotencyConflict => "authentication_run_idempotency_conflict".to_string(),
            Self::ChallengeAdmissionIncomplete => {
                "authentication_run_challenge_admission_incomplete".to_string()
            }
            Self::CreatedAtInvalid => "authentication_run_created_at_invalid".to_string(),
            Self::DeadlineInvalid => "authentication_run_deadline_invalid".to_string(),
            Self::ObservedAtInvalid => "authentication_run_observed_at_invalid".to_string(),
            Self::AuthenticationRunInvalid(error) => {
                format!("authentication_run_invalid:{error:?}")
            }
            Self::DeadlineExpired => "authentication_run_deadline_expired".to_string(),
            Self::EffectOutcomeUnknown => "authentication_run_effect_outcome_unknown".to_string(),
            Self::OperationReplay => "authentication_run_operation_replay".to_string(),
            Self::PendingEffectMissing => "authentication_run_pending_effect_missing".to_string(),
            Self::PendingEffectMismatch => "authentication_run_pending_effect_mismatch".to_string(),
            Self::CancelFailed(error) => {
                format!("authentication_run_cancel_failed:{error:?}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAuthenticationRunProjection {
    pub schema_version: String,
    pub run_id: String,
    pub state: agent_browser_authentication_control::AuthenticationRunState,
    pub created_at: String,
    pub deadline_at: String,
    pub request_sha256: String,
    pub account_ref_sha256: Option<String>,
    pub organization_ref_sha256: Option<String>,
    pub challenge_provider_id: String,
    pub challenge_provider_tenant_ref_sha256: Option<String>,
    pub challenge_provider_account_ref_sha256: Option<String>,
    pub target_service_id: String,
    pub profile_id: String,
    pub browser_id: String,
    pub session_name: String,
    pub tab_id: String,
    pub transition_count: u32,
    pub action_receipt_count: usize,
    pub observation_receipt_count: usize,
    pub challenge_consumer_admission: Option<ChallengeConsumerAdmissionReceipt>,
    pub effect_pending: bool,
    pub replayed: bool,
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, ServiceAuthenticationRunError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ServiceAuthenticationRunError::HashFailed(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Decide replay or new creation before the CLI performs challenge admission.
/// This preserves the existing idempotency and error ordering without moving
/// challenge authority into the model.
pub fn prepare_service_authentication_run_start(
    existing: &BTreeMap<String, ServiceAuthenticationRunRecord>,
    input: ServiceAuthenticationRunStartInput,
) -> Result<ServiceAuthenticationRunStartDecision, ServiceAuthenticationRunError> {
    let idempotency_key_sha256 = canonical_sha256(&input.idempotency_key)?;
    let request_sha256 = canonical_sha256(&(
        &input.binding,
        &idempotency_key_sha256,
        &input.challenge_task_id,
        &input.site_policy_id,
        input.deadline_ms,
        input.max_transitions,
    ))?;
    if let Some(existing) = existing
        .values()
        .find(|record| record.idempotency_key_sha256 == idempotency_key_sha256)
    {
        if existing.request_sha256 != request_sha256 {
            return Err(ServiceAuthenticationRunError::IdempotencyConflict);
        }
        return Ok(ServiceAuthenticationRunStartDecision::Replayed(Box::new(
            existing.clone(),
        )));
    }
    Ok(ServiceAuthenticationRunStartDecision::Create(Box::new(
        PreparedServiceAuthenticationRunStart {
            input,
            request_sha256,
            idempotency_key_sha256,
        },
    )))
}

pub fn complete_service_authentication_run_start(
    prepared: PreparedServiceAuthenticationRunStart,
    challenge_consumer_admission: Option<ChallengeConsumerAdmissionReceipt>,
) -> Result<ServiceAuthenticationRunRecord, ServiceAuthenticationRunError> {
    let challenge_binding_complete = match (
        prepared.input.challenge_task_id.as_deref(),
        prepared.input.site_policy_id.as_deref(),
        challenge_consumer_admission.as_ref(),
    ) {
        (None, None, None) => true,
        (Some(task_id), Some(_), Some(admission)) => {
            admission.challenge_task_id == task_id
                && admission.consumer == ChallengeConsumerKind::Authentication
                && admission.consumer_operation_id == prepared.idempotency_key_sha256
        }
        _ => false,
    };
    if !challenge_binding_complete {
        return Err(ServiceAuthenticationRunError::ChallengeAdmissionIncomplete);
    }

    let created = DateTime::parse_from_rfc3339(&prepared.input.created_at)
        .map_err(|_| ServiceAuthenticationRunError::CreatedAtInvalid)?
        .with_timezone(&Utc);
    let deadline_delta = i64::try_from(prepared.input.deadline_ms)
        .map_err(|_| ServiceAuthenticationRunError::DeadlineInvalid)?;
    let deadline_at = (created + Duration::milliseconds(deadline_delta)).to_rfc3339();
    let run_id = format!("authrun-{}", &prepared.request_sha256[..24]);
    let run = AuthenticationRun::new(
        run_id,
        prepared.input.binding,
        prepared.input.max_transitions,
    )
    .map_err(ServiceAuthenticationRunError::AuthenticationRunInvalid)?;
    Ok(ServiceAuthenticationRunRecord {
        schema_version: SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION.to_string(),
        request_sha256: prepared.request_sha256,
        idempotency_key_sha256: prepared.idempotency_key_sha256,
        created_at: created.to_rfc3339(),
        deadline_at,
        service_tab_handle: prepared.input.current_service_tab_handle,
        challenge_consumer_admission,
        pending_effect: None,
        run,
    })
}

pub fn project_service_authentication_run(
    record: &ServiceAuthenticationRunRecord,
    replayed: bool,
) -> ServiceAuthenticationRunProjection {
    ServiceAuthenticationRunProjection {
        schema_version: record.schema_version.clone(),
        run_id: record.run.run_id.clone(),
        state: record.run.state,
        created_at: record.created_at.clone(),
        deadline_at: record.deadline_at.clone(),
        request_sha256: record.request_sha256.clone(),
        account_ref_sha256: canonical_sha256(&record.run.binding.target_account_ref).ok(),
        organization_ref_sha256: canonical_sha256(&record.run.binding.target_organization_ref).ok(),
        challenge_provider_id: record.run.binding.challenge_provider_id.clone(),
        challenge_provider_tenant_ref_sha256: canonical_sha256(
            &record.run.binding.challenge_provider_tenant_ref,
        )
        .ok(),
        challenge_provider_account_ref_sha256: canonical_sha256(
            &record.run.binding.challenge_provider_account_ref,
        )
        .ok(),
        target_service_id: record.run.binding.target_service_id.clone(),
        profile_id: record.run.binding.profile_id.clone(),
        browser_id: record.run.binding.browser_id.clone(),
        session_name: record.run.binding.session_name.clone(),
        tab_id: record.run.binding.login_tab_id.clone(),
        transition_count: record.run.transition_count,
        action_receipt_count: record.run.action_receipts.len()
            + record.run.site_action_receipts.len(),
        observation_receipt_count: record.run.site_observation_receipts.len(),
        challenge_consumer_admission: record.challenge_consumer_admission.clone(),
        effect_pending: record.pending_effect.is_some(),
        replayed,
    }
}

pub fn require_live_authentication_run(
    record: &ServiceAuthenticationRunRecord,
    observed_at: &str,
) -> Result<(), ServiceAuthenticationRunError> {
    let deadline = DateTime::parse_from_rfc3339(&record.deadline_at)
        .map_err(|_| ServiceAuthenticationRunError::DeadlineInvalid)?
        .with_timezone(&Utc);
    let observed = DateTime::parse_from_rfc3339(observed_at)
        .map_err(|_| ServiceAuthenticationRunError::ObservedAtInvalid)?
        .with_timezone(&Utc);
    if observed > deadline {
        return Err(ServiceAuthenticationRunError::DeadlineExpired);
    }
    if record.pending_effect.is_some() {
        return Err(ServiceAuthenticationRunError::EffectOutcomeUnknown);
    }
    Ok(())
}

pub fn reserve_authentication_effect(
    record: &mut ServiceAuthenticationRunRecord,
    operation_id: &str,
    action: AuthenticationActionKind,
    state_instance_id: &str,
    reserved_at: &str,
    observed_at: &str,
) -> Result<(), ServiceAuthenticationRunError> {
    require_live_authentication_run(record, observed_at)?;
    if record.run.used_operation_ids.contains(operation_id) {
        return Err(ServiceAuthenticationRunError::OperationReplay);
    }
    record.pending_effect = Some(PendingAuthenticationEffect {
        operation_id: operation_id.to_string(),
        action,
        state_instance_id: state_instance_id.to_string(),
        reserved_at: reserved_at.to_string(),
    });
    Ok(())
}

fn take_matching_pending_effect(
    record: &mut ServiceAuthenticationRunRecord,
    operation_id: &str,
    expected_action: AuthenticationActionKind,
) -> Result<(), ServiceAuthenticationRunError> {
    let pending = record
        .pending_effect
        .as_ref()
        .ok_or(ServiceAuthenticationRunError::PendingEffectMissing)?;
    if pending.operation_id != operation_id || pending.action != expected_action {
        return Err(ServiceAuthenticationRunError::PendingEffectMismatch);
    }
    record.pending_effect = None;
    Ok(())
}

#[derive(Clone)]
struct OneShotSiteAction(Result<SiteLoginActionReceipt, AuthenticationActionFailure>);

impl ResponseOnlySiteLoginAction for OneShotSiteAction {
    fn execute(
        &mut self,
        _context: &SiteLoginActionContext<'_>,
    ) -> Result<SiteLoginActionReceipt, AuthenticationActionFailure> {
        self.0.clone()
    }
}

#[derive(Clone)]
struct OneShotAuthenticationAction(
    Result<AuthenticationActionReceipt, AuthenticationActionFailure>,
);

impl ResponseOnlyAuthenticationAction for OneShotAuthenticationAction {
    fn execute(
        &mut self,
        _context: &AuthenticationActionContext<'_>,
    ) -> Result<AuthenticationActionReceipt, AuthenticationActionFailure> {
        self.0.clone()
    }
}

pub fn complete_site_authentication_action(
    record: &mut ServiceAuthenticationRunRecord,
    operation_id: &str,
    expected_action: AuthenticationActionKind,
    outcome: Result<SiteLoginActionReceipt, AuthenticationActionFailure>,
) -> Result<ServiceAuthenticationRunCompletion, ServiceAuthenticationRunError> {
    take_matching_pending_effect(record, operation_id, expected_action)?;
    let mut adapter = OneShotSiteAction(outcome);
    Ok(
        match record
            .run
            .submit_account_identifier(operation_id, &mut adapter)
        {
            Ok(_) => ServiceAuthenticationRunCompletion::Completed,
            Err(error) => ServiceAuthenticationRunCompletion::TransitionFailed(error),
        },
    )
}

pub fn complete_credential_delivery_action(
    record: &mut ServiceAuthenticationRunRecord,
    operation_id: &str,
    challenge_id: &str,
    outcome: Result<AuthenticationActionReceipt, AuthenticationActionFailure>,
) -> Result<ServiceAuthenticationRunCompletion, ServiceAuthenticationRunError> {
    take_matching_pending_effect(
        record,
        operation_id,
        AuthenticationActionKind::SubmitNativeStoredCredentials,
    )?;
    let mut adapter = OneShotAuthenticationAction(outcome);
    Ok(
        match record.run.submit_credentials_and_trigger_delivery(
            operation_id,
            challenge_id,
            &mut adapter,
        ) {
            Ok(_) => ServiceAuthenticationRunCompletion::Completed,
            Err(error) => ServiceAuthenticationRunCompletion::TransitionFailed(error),
        },
    )
}

pub fn complete_challenge_authentication_action(
    record: &mut ServiceAuthenticationRunRecord,
    operation_id: &str,
    challenge_id: &str,
    outcome: Result<AuthenticationActionReceipt, AuthenticationActionFailure>,
) -> Result<ServiceAuthenticationRunCompletion, ServiceAuthenticationRunError> {
    take_matching_pending_effect(record, operation_id, AuthenticationActionKind::SubmitSmsOtp)?;
    let mut adapter = OneShotAuthenticationAction(outcome);
    Ok(
        match record
            .run
            .consume_challenge(operation_id, challenge_id, 1, &mut adapter)
        {
            Ok(_) => ServiceAuthenticationRunCompletion::Completed,
            Err(error) => ServiceAuthenticationRunCompletion::TransitionFailed(error),
        },
    )
}

pub fn cancel_authentication_run(
    record: &mut ServiceAuthenticationRunRecord,
    operation_id: &str,
) -> Result<(), ServiceAuthenticationRunError> {
    record
        .run
        .cancel(operation_id)
        .map_err(ServiceAuthenticationRunError::CancelFailed)
}

pub fn authentication_run_map_is_empty(
    value: &BTreeMap<String, ServiceAuthenticationRunRecord>,
) -> bool {
    value.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_authentication_control::{
        AuthenticationRunState, SiteLoginObservationReceipt, SiteLoginState,
    };
    use agent_browser_challenge_control::{
        ChallengeConsumerAdmission, ChallengeTaskAdmission, ChallengeTaskCooldown,
        ChallengeTaskOutcome,
    };

    fn binding() -> AuthenticationRunBinding {
        AuthenticationRunBinding {
            service_id: "service-1".to_string(),
            agent_id: "agent-1".to_string(),
            task_id: "task-1".to_string(),
            principal_id: "principal-1".to_string(),
            target_service_id: "target-1".to_string(),
            target_account_ref: "opaque-account-canary".to_string(),
            target_organization_ref: "opaque-organization-canary".to_string(),
            challenge_provider_id: "provider-1".to_string(),
            challenge_provider_tenant_ref: "opaque-tenant-canary".to_string(),
            challenge_provider_account_ref: "opaque-provider-account-canary".to_string(),
            profile_id: "profile-1".to_string(),
            browser_id: "browser-1".to_string(),
            session_name: "session-1".to_string(),
            login_tab_id: "tab-1".to_string(),
            site_recipe_id: "recipe-1".to_string(),
            policy_digest: "a".repeat(64),
        }
    }

    fn start_input(idempotency_key: &str) -> ServiceAuthenticationRunStartInput {
        ServiceAuthenticationRunStartInput {
            binding: binding(),
            idempotency_key: idempotency_key.to_string(),
            challenge_task_id: None,
            site_policy_id: None,
            deadline_ms: 120_000,
            max_transitions: 16,
            current_service_tab_handle: ServiceTabHandle {
                browser_id: "browser-1".to_string(),
                tab_id: "tab-1".to_string(),
                valid: true,
                ..ServiceTabHandle::default()
            },
            created_at: "2026-09-17T12:00:00Z".to_string(),
        }
    }

    fn create_record(idempotency_key: &str) -> ServiceAuthenticationRunRecord {
        let decision = prepare_service_authentication_run_start(
            &BTreeMap::new(),
            start_input(idempotency_key),
        )
        .unwrap();
        let ServiceAuthenticationRunStartDecision::Create(prepared) = decision else {
            panic!("new input unexpectedly replayed");
        };
        complete_service_authentication_run_start(*prepared, None).unwrap()
    }

    #[test]
    fn start_round_trips_and_projection_redacts_opaque_inputs() {
        let record = create_record("raw-idempotency-canary");
        let encoded = serde_json::to_value(&record).unwrap();
        let decoded: ServiceAuthenticationRunRecord = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, record);
        let projection =
            serde_json::to_string(&project_service_authentication_run(&record, false)).unwrap();
        for secret in [
            "raw-idempotency-canary",
            "opaque-account-canary",
            "opaque-organization-canary",
            "opaque-tenant-canary",
            "opaque-provider-account-canary",
        ] {
            assert!(!projection.contains(secret));
        }
        assert!(authentication_run_map_is_empty(&BTreeMap::new()));
    }

    #[test]
    fn start_replays_exact_request_and_rejects_changed_request() {
        let record = create_record("stable-key");
        let records = BTreeMap::from([(record.run.run_id.clone(), record.clone())]);
        assert_eq!(
            prepare_service_authentication_run_start(&records, start_input("stable-key")).unwrap(),
            ServiceAuthenticationRunStartDecision::Replayed(Box::new(record))
        );
        let mut changed = start_input("stable-key");
        changed.deadline_ms += 1;
        assert_eq!(
            prepare_service_authentication_run_start(&records, changed).unwrap_err(),
            ServiceAuthenticationRunError::IdempotencyConflict
        );
    }

    #[test]
    fn typed_challenge_admission_is_required_and_bound_to_the_prepared_start() {
        let mut input = start_input("challenge-key");
        input.challenge_task_id = Some("challenge-1".to_string());
        input.site_policy_id = Some("site-policy-1".to_string());
        let decision = prepare_service_authentication_run_start(&BTreeMap::new(), input).unwrap();
        let ServiceAuthenticationRunStartDecision::Create(prepared) = decision else {
            panic!("new input unexpectedly replayed");
        };
        assert_eq!(
            complete_service_authentication_run_start(*prepared.clone(), None).unwrap_err(),
            ServiceAuthenticationRunError::ChallengeAdmissionIncomplete
        );
        let admission = ChallengeConsumerAdmissionReceipt {
            schema_version: "agent-browser.challenge-consumer-admission.v1".to_string(),
            challenge_task_id: "challenge-1".to_string(),
            consumer: ChallengeConsumerKind::Authentication,
            consumer_operation_id: prepared.idempotency_key_sha256.clone(),
            site_policy_digest: "b".repeat(64),
            downstream_intent_id: "authentication-run-start".to_string(),
            challenge_outcome: ChallengeTaskOutcome::Passed,
            challenge_admission: ChallengeTaskAdmission::Admitted,
            challenge_cooldown: ChallengeTaskCooldown::NotRequired,
            challenge_emitted_effects: false,
            consumer_admission: ChallengeConsumerAdmission::Admitted,
        };
        let record =
            complete_service_authentication_run_start(*prepared, Some(admission.clone())).unwrap();
        assert_eq!(record.challenge_consumer_admission, Some(admission));
    }

    #[test]
    fn liveness_reservation_replay_and_pending_fences_are_deterministic() {
        let mut record = create_record("reservation-key");
        require_live_authentication_run(&record, "2026-09-17T12:01:00Z").unwrap();
        assert_eq!(
            require_live_authentication_run(&record, "2026-09-17T12:03:00Z").unwrap_err(),
            ServiceAuthenticationRunError::DeadlineExpired
        );
        reserve_authentication_effect(
            &mut record,
            "operation-1",
            AuthenticationActionKind::SubmitAccountIdentifier,
            "state-1",
            "2026-09-17T12:01:00Z",
            "2026-09-17T12:01:01Z",
        )
        .unwrap();
        assert!(project_service_authentication_run(&record, false).effect_pending);
        assert_eq!(
            reserve_authentication_effect(
                &mut record,
                "operation-2",
                AuthenticationActionKind::SubmitAccountIdentifier,
                "state-1",
                "2026-09-17T12:01:02Z",
                "2026-09-17T12:01:03Z",
            )
            .unwrap_err(),
            ServiceAuthenticationRunError::EffectOutcomeUnknown
        );
    }

    fn identifier_ready_record(idempotency_key: &str) -> ServiceAuthenticationRunRecord {
        let mut record = create_record(idempotency_key);
        record
            .run
            .observe_site_login_state(
                "observe-1",
                SiteLoginObservationReceipt {
                    observer_id: "observer-1".to_string(),
                    target_service_id: "target-1".to_string(),
                    target_account_ref: "opaque-account-canary".to_string(),
                    profile_id: "profile-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    session_name: "session-1".to_string(),
                    tab_id: "tab-1".to_string(),
                    origin_verified: true,
                    page_state_fresh: true,
                    site_state: SiteLoginState::IdentifierForm,
                    state_instance_id: "state-1".to_string(),
                    exact_account_authenticated: false,
                    browser_chrome_semantics_used: false,
                },
            )
            .unwrap();
        record
    }

    fn identifier_receipt() -> SiteLoginActionReceipt {
        SiteLoginActionReceipt {
            provider_id: "provider-1".to_string(),
            effect_id: "effect-1".to_string(),
            action: AuthenticationActionKind::SubmitAccountIdentifier,
            state_instance_id: "state-1".to_string(),
            response_only_material_consumption: true,
            secret_material_exposed: false,
            browser_chrome_semantics_used: false,
            persistence_verified: false,
        }
    }

    #[test]
    fn completion_consumes_pending_fence_and_preserves_success_or_failure_state() {
        for (suffix, outcome, expected_state, expected_completion) in [
            (
                "success",
                Ok(identifier_receipt()),
                AuthenticationRunState::Ready,
                ServiceAuthenticationRunCompletion::Completed,
            ),
            (
                "failure",
                Err(AuthenticationActionFailure::EffectUnproven),
                AuthenticationRunState::Blocked,
                ServiceAuthenticationRunCompletion::TransitionFailed(
                    AuthenticationRunError::ActionFailed,
                ),
            ),
        ] {
            let mut record = identifier_ready_record(suffix);
            reserve_authentication_effect(
                &mut record,
                "submit-1",
                AuthenticationActionKind::SubmitAccountIdentifier,
                "state-1",
                "2026-09-17T12:01:00Z",
                "2026-09-17T12:01:01Z",
            )
            .unwrap();
            let completion = complete_site_authentication_action(
                &mut record,
                "submit-1",
                AuthenticationActionKind::SubmitAccountIdentifier,
                outcome,
            )
            .unwrap();
            assert_eq!(completion, expected_completion);
            assert_eq!(record.run.state, expected_state);
            assert!(!project_service_authentication_run(&record, false).effect_pending);
        }
    }

    #[test]
    fn cancellation_uses_the_canonical_authentication_transition() {
        let mut record = create_record("cancel-key");
        cancel_authentication_run(&mut record, "cancel-1").unwrap();
        assert_eq!(record.run.state, AuthenticationRunState::Cancelled);
        assert_eq!(
            cancel_authentication_run(&mut record, "cancel-2").unwrap_err(),
            ServiceAuthenticationRunError::CancelFailed(AuthenticationRunError::UnexpectedState)
        );
    }
}
