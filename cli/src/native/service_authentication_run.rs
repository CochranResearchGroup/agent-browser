//! Durable public custody for response-only authentication runs.
//!
//! This module accepts only opaque references and exact retained-tab identity.
//! It intentionally does not accept selectors, usernames, passwords, one-time
//! codes, message bodies, verification URLs, or generic UI recipes.

use super::authentication_run::{AuthenticationRun, AuthenticationRunBinding};
use super::service_model::{LeaseState, ServiceState, ServiceTabHandle};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use super::service_trace::service_now_timestamp;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) const SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION: &str =
    "agent-browser.service-authentication-run.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ServiceAuthenticationRunRecord {
    pub(crate) schema_version: String,
    pub(crate) request_sha256: String,
    pub(crate) idempotency_key_sha256: String,
    pub(crate) created_at: String,
    pub(crate) deadline_at: String,
    pub(crate) run: AuthenticationRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthenticationRunStartIntent {
    service_name: String,
    agent_name: String,
    task_name: String,
    principal_id: String,
    target_service_id: String,
    account_ref: String,
    profile_id: String,
    browser_id: String,
    session_name: String,
    site_recipe_id: String,
    policy_digest: String,
    idempotency_key: String,
    deadline_ms: u64,
    max_transitions: u32,
    supplied_handle: ServiceTabHandle,
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("authentication_run_hash_failed:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn required_string(command: &Value, field: &str) -> Result<String, String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("authentication_run_{field}_required"))
}

fn caller_principal(command: &Value) -> Result<String, String> {
    command
        .get("clientSubjectId")
        .or_else(|| command.get("callerId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "authentication_run_caller_principal_required".to_string())
}

fn parse_start_intent(command: &Value) -> Result<AuthenticationRunStartIntent, String> {
    for forbidden in [
        "username",
        "password",
        "otp",
        "code",
        "messageBody",
        "verificationUrl",
        "selector",
        "uiAction",
        "expression",
        "script",
    ] {
        if command.get(forbidden).is_some() {
            return Err(format!("authentication_run_forbidden_field:{forbidden}"));
        }
    }
    let policy_digest = required_string(command, "policyDigest")?;
    if policy_digest.len() != 64 || !policy_digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("authentication_run_policy_digest_invalid".to_string());
    }
    let deadline_ms = command
        .get("deadlineMs")
        .and_then(Value::as_u64)
        .filter(|value| (1_000..=600_000).contains(value))
        .ok_or_else(|| "authentication_run_deadline_invalid".to_string())?;
    let max_transitions = command
        .get("maxTransitions")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| (1..=64).contains(value))
        .ok_or_else(|| "authentication_run_transition_budget_invalid".to_string())?;
    let supplied_handle: ServiceTabHandle = serde_json::from_value(
        command
            .get("serviceTabHandle")
            .cloned()
            .ok_or_else(|| "authentication_run_service_tab_handle_required".to_string())?,
    )
    .map_err(|error| format!("authentication_run_service_tab_handle_invalid:{error}"))?;
    Ok(AuthenticationRunStartIntent {
        service_name: required_string(command, "serviceName")?,
        agent_name: required_string(command, "agentName")?,
        task_name: required_string(command, "taskName")?,
        principal_id: caller_principal(command)?,
        target_service_id: required_string(command, "targetServiceId")?,
        account_ref: required_string(command, "accountRef")?,
        profile_id: required_string(command, "profileId")?,
        browser_id: required_string(command, "browserId")?,
        session_name: required_string(command, "sessionName")?,
        site_recipe_id: required_string(command, "siteRecipeId")?,
        policy_digest: policy_digest.to_ascii_lowercase(),
        idempotency_key: required_string(command, "idempotencyKey")?,
        deadline_ms,
        max_transitions,
        supplied_handle,
    })
}

fn exact_handle_binding(
    intent: &AuthenticationRunStartIntent,
    current: &ServiceTabHandle,
) -> Result<AuthenticationRunBinding, String> {
    let supplied = &intent.supplied_handle;
    let checks = [
        (supplied.valid, "supplied_valid"),
        (current.valid, "current_valid"),
        (
            supplied.browser_id == intent.browser_id,
            "requested_browser_id",
        ),
        (
            supplied.session_name.as_deref() == Some(intent.session_name.as_str()),
            "requested_session_name",
        ),
        (
            supplied.profile_id.as_deref() == Some(intent.profile_id.as_str()),
            "requested_profile_id",
        ),
        (supplied.browser_id == current.browser_id, "browser_id"),
        (
            supplied.session_name == current.session_name,
            "session_name",
        ),
        (supplied.tab_id == current.tab_id, "tab_id"),
        (supplied.target_id == current.target_id, "target_id"),
        (supplied.profile_id == current.profile_id, "profile_id"),
        (supplied.lease_id == current.lease_id, "lease_id"),
        (
            supplied.owner_session_id == current.owner_session_id,
            "owner_session_id",
        ),
        (
            current.lease_state == Some(LeaseState::Exclusive),
            "exclusive_lease",
        ),
        (
            current.trace_filter.service_name.as_deref() == Some(&intent.service_name),
            "service_name",
        ),
        (
            current.trace_filter.agent_name.as_deref() == Some(&intent.agent_name),
            "agent_name",
        ),
        (
            current.trace_filter.task_name.as_deref() == Some(&intent.task_name),
            "task_name",
        ),
        (
            current
                .profile_access
                .as_ref()
                .and_then(|access| access.subject_id.as_deref())
                == Some(&intent.principal_id),
            "principal_id",
        ),
    ];
    if let Some((_, axis)) = checks.into_iter().find(|(matched, _)| !matched) {
        return Err(format!(
            "authentication_run_service_tab_handle_mismatch:{axis}"
        ));
    }
    Ok(AuthenticationRunBinding {
        service_id: intent.service_name.clone(),
        agent_id: intent.agent_name.clone(),
        task_id: intent.task_name.clone(),
        principal_id: intent.principal_id.clone(),
        target_service_id: intent.target_service_id.clone(),
        target_account_ref: intent.account_ref.clone(),
        profile_id: current
            .profile_id
            .clone()
            .ok_or_else(|| "authentication_run_profile_missing".to_string())?,
        browser_id: current.browser_id.clone(),
        session_name: current
            .session_name
            .clone()
            .ok_or_else(|| "authentication_run_session_missing".to_string())?,
        login_tab_id: current.tab_id.clone(),
        site_recipe_id: intent.site_recipe_id.clone(),
        policy_digest: intent.policy_digest.clone(),
    })
}

fn start_run_in_state(
    state: &mut ServiceState,
    intent: AuthenticationRunStartIntent,
    created_at: &str,
) -> Result<(ServiceAuthenticationRunRecord, bool), String> {
    let current = state
        .service_tab_handle(&intent.supplied_handle.tab_id)
        .ok_or_else(|| "authentication_run_service_tab_handle_missing".to_string())?;
    let binding = exact_handle_binding(&intent, &current)?;
    let idempotency_key_sha256 = canonical_sha256(&intent.idempotency_key)?;
    let request_sha256 = canonical_sha256(&(
        &binding,
        &idempotency_key_sha256,
        intent.deadline_ms,
        intent.max_transitions,
    ))?;
    if let Some(existing) = state
        .authentication_runs
        .values()
        .find(|record| record.idempotency_key_sha256 == idempotency_key_sha256)
    {
        if existing.request_sha256 != request_sha256 {
            return Err("authentication_run_idempotency_conflict".to_string());
        }
        return Ok((existing.clone(), true));
    }
    let run_id = format!("authrun-{}", &request_sha256[..24]);
    let created = DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| "authentication_run_created_at_invalid".to_string())?
        .with_timezone(&Utc);
    let deadline_delta = i64::try_from(intent.deadline_ms)
        .map_err(|_| "authentication_run_deadline_invalid".to_string())?;
    let deadline_at = (created + Duration::milliseconds(deadline_delta)).to_rfc3339();
    let record = ServiceAuthenticationRunRecord {
        schema_version: SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION.to_string(),
        request_sha256,
        idempotency_key_sha256,
        created_at: created.to_rfc3339(),
        deadline_at,
        run: AuthenticationRun::new(run_id.clone(), binding, intent.max_transitions)
            .map_err(|error| format!("authentication_run_invalid:{error:?}"))?,
    };
    state.authentication_runs.insert(run_id, record.clone());
    Ok((record, false))
}

fn caller_owns_run(command: &Value, record: &ServiceAuthenticationRunRecord) -> Result<(), String> {
    let binding = &record.run.binding;
    let matches = required_string(command, "serviceName")? == binding.service_id
        && required_string(command, "agentName")? == binding.agent_id
        && required_string(command, "taskName")? == binding.task_id
        && caller_principal(command)? == binding.principal_id;
    if matches {
        Ok(())
    } else {
        Err("authentication_run_caller_mismatch".to_string())
    }
}

fn run_projection(record: &ServiceAuthenticationRunRecord, replayed: bool) -> Value {
    json!({
        "schemaVersion": record.schema_version,
        "runId": record.run.run_id,
        "state": record.run.state,
        "createdAt": record.created_at,
        "deadlineAt": record.deadline_at,
        "requestSha256": record.request_sha256,
        "accountRefSha256": canonical_sha256(&record.run.binding.target_account_ref).ok(),
        "targetServiceId": record.run.binding.target_service_id,
        "profileId": record.run.binding.profile_id,
        "browserId": record.run.binding.browser_id,
        "sessionName": record.run.binding.session_name,
        "tabId": record.run.binding.login_tab_id,
        "transitionCount": record.run.transition_count,
        "actionReceiptCount": record.run.action_receipts.len() + record.run.site_action_receipts.len(),
        "observationReceiptCount": record.run.site_observation_receipts.len(),
        "replayed": replayed,
    })
}

pub(crate) fn handle_service_authentication_run(command: &Value) -> Result<Value, String> {
    let action = command
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "authentication_run_action_required".to_string())?;
    let repository = LockedServiceStateRepository::default_json()?;
    match action {
        "service_authentication_run_start" => {
            let intent = parse_start_intent(command)?;
            let created_at = service_now_timestamp();
            let (record, replayed) =
                repository.mutate(|state| start_run_in_state(state, intent, &created_at))?;
            Ok(run_projection(&record, replayed))
        }
        "service_authentication_run_status" => {
            let run_id = required_string(command, "authenticationRunId")?;
            let state = repository.load_snapshot()?;
            let record = state
                .authentication_runs
                .get(&run_id)
                .ok_or_else(|| "authentication_run_not_found".to_string())?;
            caller_owns_run(command, record)?;
            Ok(run_projection(record, false))
        }
        "service_authentication_run_cancel" => {
            let run_id = required_string(command, "authenticationRunId")?;
            let operation_id = required_string(command, "operationId")?;
            let record = repository.mutate(|state| {
                let record = state
                    .authentication_runs
                    .get_mut(&run_id)
                    .ok_or_else(|| "authentication_run_not_found".to_string())?;
                caller_owns_run(command, record)?;
                record
                    .run
                    .cancel(&operation_id)
                    .map_err(|error| format!("authentication_run_cancel_failed:{error:?}"))?;
                Ok(record.clone())
            })?;
            Ok(run_projection(&record, false))
        }
        "service_authentication_run_resume" => {
            Err("authentication_run_sealed_provider_unavailable".to_string())
        }
        _ => Err(format!("authentication_run_action_unsupported:{action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserProcess, BrowserSession, BrowserTab, ProfileOrigin,
        ServiceTabHandleTraceFilter, TabLifecycle,
    };
    use crate::native::service_profile_access_policy::{
        ProfileChildAccess, ProfileConnectionState, ProfileIdentityAssurance,
    };

    fn handle() -> ServiceTabHandle {
        ServiceTabHandle {
            browser_id: "browser-1".to_string(),
            session_name: Some("session-1".to_string()),
            tab_id: "tab-1".to_string(),
            target_id: Some("target-1".to_string()),
            profile_id: Some("profile-1".to_string()),
            profile_origin: ProfileOrigin::AgentBrowserOwned,
            lease_id: Some("session-1".to_string()),
            lease_state: Some(LeaseState::Exclusive),
            owner_session_id: Some("session-1".to_string()),
            profile_access: Some(ProfileChildAccess {
                schema_version: "agent-browser.profile-child-access.v1".to_string(),
                parent_policy_revision: 1,
                access_decision_id: "decision-1".to_string(),
                subject_id: Some("principal-1".to_string()),
                identity_assurance: ProfileIdentityAssurance::AuthenticatedIngress,
                connection_instance_id: Some("connection-1".to_string()),
                connection_state: ProfileConnectionState::Active,
                permissions: Vec::new(),
            }),
            trace_filter: ServiceTabHandleTraceFilter {
                browser_id: Some("browser-1".to_string()),
                profile_id: Some("profile-1".to_string()),
                session_id: Some("session-1".to_string()),
                service_name: Some("books-receipts".to_string()),
                agent_name: Some("closeout-worker".to_string()),
                task_name: Some("bill-auth".to_string()),
            },
            valid: true,
            ..ServiceTabHandle::default()
        }
    }

    fn service_state() -> ServiceState {
        let handle = handle();
        let mut state = ServiceState::default();
        state.browsers.insert(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("profile-1".to_string()),
                health: BrowserHealth::Ready,
                active_session_ids: vec!["session-1".to_string()],
                ..BrowserProcess::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                service_name: Some("books-receipts".to_string()),
                agent_name: Some("closeout-worker".to_string()),
                task_name: Some("bill-auth".to_string()),
                profile_id: Some("profile-1".to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "tab-1".to_string(),
            BrowserTab {
                id: "tab-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("target-1".to_string()),
                session_id: Some("session-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                profile_access: handle.profile_access,
                ..BrowserTab::default()
            },
        );
        state
    }

    fn start_command(state: &ServiceState) -> Value {
        let current = state.service_tab_handle("tab-1").unwrap();
        json!({
            "action": "service_authentication_run_start",
            "serviceName": "books-receipts",
            "agentName": "closeout-worker",
            "taskName": "bill-auth",
            "clientSubjectId": "principal-1",
            "targetServiceId": "bill",
            "accountRef": "opaque-account-1",
            "profileId": "profile-1",
            "browserId": "browser-1",
            "sessionName": "session-1",
            "siteRecipeId": "bill-login-v1",
            "policyDigest": "a".repeat(64),
            "idempotencyKey": "auth-idempotency-1",
            "deadlineMs": 120000,
            "maxTransitions": 32,
            "serviceTabHandle": current,
        })
    }

    #[test]
    fn start_is_exact_handle_bound_idempotent_and_secret_free() {
        let mut state = service_state();
        let command = start_command(&state);
        let intent = parse_start_intent(&command).unwrap();
        let (first, replayed) =
            start_run_in_state(&mut state, intent, "2026-09-10T12:00:00Z").unwrap();
        assert!(!replayed);
        let replay_intent = parse_start_intent(&command).unwrap();
        let (second, replayed) =
            start_run_in_state(&mut state, replay_intent, "2026-09-10T12:01:00Z").unwrap();
        assert!(replayed);
        assert_eq!(first, second);
        assert_eq!(state.authentication_runs.len(), 1);
        let projection = run_projection(&first, false).to_string();
        assert!(!projection.contains("opaque-account-1"));
        assert!(!projection.contains("auth-idempotency-1"));

        let encoded = serde_json::to_value(&state).unwrap();
        let decoded: ServiceState = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.authentication_runs, state.authentication_runs);
    }

    #[test]
    fn idempotency_key_reuse_with_changed_request_is_rejected() {
        let mut state = service_state();
        let command = start_command(&state);
        start_run_in_state(
            &mut state,
            parse_start_intent(&command).unwrap(),
            "2026-09-10T12:00:00Z",
        )
        .unwrap();

        let mut changed = command;
        changed["deadlineMs"] = json!(121000);
        assert_eq!(
            start_run_in_state(
                &mut state,
                parse_start_intent(&changed).unwrap(),
                "2026-09-10T12:01:00Z",
            )
            .unwrap_err(),
            "authentication_run_idempotency_conflict"
        );
        assert_eq!(state.authentication_runs.len(), 1);
    }

    #[test]
    fn wrong_principal_or_stale_handle_fails_before_record_creation() {
        let mut state = service_state();
        let mut wrong_principal = start_command(&state);
        wrong_principal["clientSubjectId"] = json!("different-principal");
        assert_eq!(
            start_run_in_state(
                &mut state,
                parse_start_intent(&wrong_principal).unwrap(),
                "2026-09-10T12:00:00Z",
            )
            .unwrap_err(),
            "authentication_run_service_tab_handle_mismatch:principal_id"
        );
        let mut stale = start_command(&state);
        stale["serviceTabHandle"]["valid"] = json!(false);
        assert_eq!(
            start_run_in_state(
                &mut state,
                parse_start_intent(&stale).unwrap(),
                "2026-09-10T12:00:00Z",
            )
            .unwrap_err(),
            "authentication_run_service_tab_handle_mismatch:supplied_valid"
        );
        assert!(state.authentication_runs.is_empty());
    }

    #[test]
    fn forbidden_secret_and_generic_input_fields_are_rejected() {
        let state = service_state();
        for field in ["username", "password", "otp", "uiAction", "selector"] {
            let mut command = start_command(&state);
            command[field] = json!("synthetic-secret-canary");
            assert_eq!(
                parse_start_intent(&command).unwrap_err(),
                format!("authentication_run_forbidden_field:{field}")
            );
        }
    }
}

pub(crate) fn authentication_run_map_is_empty(
    value: &BTreeMap<String, ServiceAuthenticationRunRecord>,
) -> bool {
    value.is_empty()
}
