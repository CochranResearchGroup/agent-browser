//! Durable Service custody for provider-free challenge tasks.
//!
//! The public request accepts registered identities and an exact retained-tab
//! binding. It never accepts pixels, coordinates, selectors, credentials, or
//! arbitrary desktop events.

use super::service_model::{LeaseState, ServiceState, ServiceTabHandle};
use super::service_store::ServiceStateRepository;
use agent_browser_challenge_control::{
    challenge_profile, ChallengeConsumerAdmissionReceipt, ChallengeConsumerAdmissionRequest,
    ChallengeConsumerKind, ChallengeTaskFixture,
};
use agent_browser_service_model::{
    admit_challenge_consumer_from_receipt, project_service_challenge_task,
    ServiceChallengeTaskCancelInput, ServiceChallengeTaskResumeInput,
    ServiceChallengeTaskStartInput,
};
pub(crate) use agent_browser_service_model::{
    ServiceChallengeTaskRecord, ServiceChallengeTaskState, AUTHENTICATION_CHALLENGE_INTENT_ID,
    NAVIGATION_CHALLENGE_INTENT_ID,
};
use chrono::Utc;
use serde::Serialize;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ChallengeTaskStartIntent {
    service_name: String,
    agent_name: String,
    task_name: String,
    principal_id: String,
    challenge_profile_id: String,
    site_policy_digest: String,
    downstream_intent_id: String,
    fixture: ChallengeTaskFixture,
    idempotency_key: String,
    deadline_ms: u64,
    max_transitions: u8,
    supplied_handle: ServiceTabHandle,
}

fn required_string(command: &Value, field: &str) -> Result<String, String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("challenge_task_{field}_required"))
}

fn caller_principal(command: &Value) -> Result<String, String> {
    command
        .get("clientSubjectId")
        .or_else(|| command.get("callerId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "challenge_task_caller_principal_required".to_string())
}

fn parse_start_intent(command: &Value) -> Result<ChallengeTaskStartIntent, String> {
    for forbidden in [
        "username",
        "password",
        "otp",
        "code",
        "selector",
        "pixels",
        "coordinates",
        "providerControls",
        "retryCount",
    ] {
        if command.get(forbidden).is_some() {
            return Err(format!("challenge_task_forbidden_field:{forbidden}"));
        }
    }
    let challenge_profile_id = required_string(command, "challengeProfileId")?;
    if challenge_profile(&challenge_profile_id).is_none() {
        return Err("challenge_task_profile_not_registered".to_string());
    }
    let fixture: ChallengeTaskFixture = serde_json::from_value(
        command
            .get("fixtureScenarioId")
            .cloned()
            .ok_or_else(|| "challenge_task_fixtureScenarioId_required".to_string())?,
    )
    .map_err(|_| "challenge_task_fixture_not_registered".to_string())?;
    let site_policy_digest = required_string(command, "sitePolicyDigest")?;
    if site_policy_digest.len() != 64
        || !site_policy_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("challenge_task_site_policy_digest_invalid".to_string());
    }
    let deadline_ms = command
        .get("deadlineMs")
        .and_then(Value::as_u64)
        .filter(|value| (1_000..=600_000).contains(value))
        .ok_or_else(|| "challenge_task_deadline_invalid".to_string())?;
    let max_transitions = command
        .get("maxTransitions")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| (1..=64).contains(value))
        .ok_or_else(|| "challenge_task_transition_budget_invalid".to_string())?;
    let supplied_handle: ServiceTabHandle = serde_json::from_value(
        command
            .get("serviceTabHandle")
            .cloned()
            .ok_or_else(|| "challenge_task_service_tab_handle_required".to_string())?,
    )
    .map_err(|_| "challenge_task_service_tab_handle_invalid".to_string())?;
    Ok(ChallengeTaskStartIntent {
        service_name: required_string(command, "serviceName")?,
        agent_name: required_string(command, "agentName")?,
        task_name: required_string(command, "taskName")?,
        principal_id: caller_principal(command)?,
        challenge_profile_id,
        site_policy_digest,
        downstream_intent_id: required_string(command, "downstreamIntentId")?,
        fixture,
        idempotency_key: required_string(command, "idempotencyKey")?,
        deadline_ms,
        max_transitions,
        supplied_handle,
    })
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, String> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| format!("challenge_task_hash_failed:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn exact_current_handle(
    state: &ServiceState,
    intent: &ChallengeTaskStartIntent,
) -> Result<ServiceTabHandle, String> {
    let current = state
        .service_tab_handle(&intent.supplied_handle.tab_id)
        .ok_or_else(|| "challenge_task_service_tab_handle_missing".to_string())?;
    let supplied = &intent.supplied_handle;
    let controlled_lease = matches!(
        current.lease_state,
        Some(LeaseState::Shared | LeaseState::Exclusive)
    );
    let principal_matches = current
        .profile_access
        .as_ref()
        .and_then(|access| access.subject_id.as_deref())
        == Some(intent.principal_id.as_str());
    let trace = &current.trace_filter;
    let matches = supplied.valid
        && current.valid
        && controlled_lease
        && principal_matches
        && supplied.browser_id == current.browser_id
        && supplied.session_name == current.session_name
        && supplied.tab_id == current.tab_id
        && supplied.target_id == current.target_id
        && supplied.profile_id == current.profile_id
        && trace.service_name.as_deref() == Some(intent.service_name.as_str())
        && trace.agent_name.as_deref() == Some(intent.agent_name.as_str())
        && trace.task_name.as_deref() == Some(intent.task_name.as_str());
    if matches {
        Ok(current)
    } else {
        Err("challenge_task_service_tab_handle_mismatch".to_string())
    }
}

fn challenge_task_start_input(
    state: &ServiceState,
    intent: ChallengeTaskStartIntent,
    created_at: &str,
) -> Result<ServiceChallengeTaskStartInput, String> {
    let current = exact_current_handle(state, &intent)?;
    Ok(ServiceChallengeTaskStartInput {
        service_name: intent.service_name,
        agent_name: intent.agent_name,
        task_name: intent.task_name,
        principal_id: intent.principal_id,
        challenge_profile_id: intent.challenge_profile_id,
        site_policy_digest: intent.site_policy_digest,
        downstream_intent_id: intent.downstream_intent_id,
        fixture: intent.fixture,
        idempotency_key: intent.idempotency_key,
        deadline_ms: intent.deadline_ms,
        max_transitions: intent.max_transitions,
        current_service_tab_handle: current,
        created_at: created_at.to_string(),
    })
}

fn require_current_record_handle(
    state: &ServiceState,
    record: &ServiceChallengeTaskRecord,
) -> Result<(), String> {
    let current = state
        .service_tab_handle(&record.service_tab_handle.tab_id)
        .ok_or_else(|| "challenge_task_service_tab_handle_missing".to_string())?;
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
    if supplied.valid
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
    {
        Ok(())
    } else {
        Err("challenge_task_service_tab_handle_mismatch".to_string())
    }
}

/// Hash the current canonical site policy for a challenge consumer binding.
pub(crate) fn effective_site_policy_digest(
    state: &ServiceState,
    site_policy_id: &str,
) -> Result<String, String> {
    let policy = state
        .site_policies
        .get(site_policy_id)
        .ok_or_else(|| "challenge_consumer_site_policy_missing".to_string())?;
    if policy.id != site_policy_id {
        return Err("challenge_consumer_site_policy_identity_mismatch".to_string());
    }
    canonical_sha256(policy)
}

#[allow(clippy::too_many_arguments)]
/// Admit one downstream consumer only after the durable task, principal,
/// current tab, effective policy, and registered intent all still agree.
///
/// This adapter is read-only. It returns a typed admission receipt but does
/// not execute authentication, navigation, browser input, or provider work.
pub(crate) fn admit_challenge_consumer(
    state: &ServiceState,
    challenge_task_id: &str,
    principal_id: &str,
    supplied_handle: &ServiceTabHandle,
    site_policy_id: &str,
    downstream_intent_id: &str,
    consumer_operation_id: &str,
    consumer: ChallengeConsumerKind,
) -> Result<ChallengeConsumerAdmissionReceipt, String> {
    let record = state
        .service_challenge_task(challenge_task_id)
        .ok_or_else(|| "challenge_consumer_task_not_found".to_string())?;
    if record.principal_id != principal_id {
        return Err("challenge_task_principal_mismatch".to_string());
    }
    if record.state != ServiceChallengeTaskState::Completed {
        return Err("challenge_consumer_task_not_completed".to_string());
    }
    require_current_record_handle(state, record)?;
    if supplied_handle != &record.service_tab_handle {
        return Err("challenge_consumer_service_tab_handle_mismatch".to_string());
    }
    let site_policy_digest = effective_site_policy_digest(state, site_policy_id)?;
    admit_challenge_consumer_from_receipt(
        record.receipt.as_ref(),
        ChallengeConsumerAdmissionRequest {
            consumer,
            consumer_operation_id: consumer_operation_id.to_string(),
            expected_site_policy_digest: site_policy_digest,
            expected_downstream_intent_id: downstream_intent_id.to_string(),
        },
    )
    .map_err(|error| error.cli_message())
}

fn task_projection(record: &ServiceChallengeTaskRecord, replayed: bool) -> Value {
    serde_json::to_value(project_service_challenge_task(record, replayed))
        .expect("Service challenge projection must serialize")
}

/// Start a durable, provider-free challenge task through Service State.
///
/// The command is resolved without launching or controlling a browser. The
/// supplied tab handle must match the currently retained Service tab and its
/// authenticated principal exactly.
pub(crate) fn handle_service_challenge_task(command: &Value) -> Result<Value, String> {
    let action = command
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "challenge_task_action_required".to_string())?;
    let repository = super::service_store::LockedServiceStateRepository::default_json()?;
    match action {
        "service_challenge_task_start" => {
            let intent = parse_start_intent(command)?;
            let created_at = Utc::now().to_rfc3339();
            let (record, replayed) = repository.mutate(|state| {
                let input = challenge_task_start_input(state, intent.clone(), &created_at)?;
                state
                    .start_service_challenge_task(input)
                    .map_err(|error| error.cli_message())
            })?;
            Ok(task_projection(&record, replayed))
        }
        "service_challenge_task_status" => {
            let task_id = required_string(command, "challengeTaskId")?;
            let principal_id = caller_principal(command)?;
            let state = repository.load_snapshot()?;
            let record = state
                .status_service_challenge_task(&task_id, &principal_id)
                .map_err(|error| error.cli_message())?;
            Ok(task_projection(&record, false))
        }
        "service_challenge_task_resume" => {
            let task_id = required_string(command, "challengeTaskId")?;
            let principal_id = caller_principal(command)?;
            let operation_id = required_string(command, "operationId")?;
            let resumed_at = Utc::now().to_rfc3339();
            let (record, replayed) = repository.mutate(|state| {
                state
                    .resume_service_challenge_task(ServiceChallengeTaskResumeInput {
                        task_id: task_id.clone(),
                        principal_id: principal_id.clone(),
                        operation_id: operation_id.clone(),
                        resumed_at: resumed_at.clone(),
                    })
                    .map_err(|error| error.cli_message())
            })?;
            Ok(task_projection(&record, replayed))
        }
        "service_challenge_task_cancel" => {
            let task_id = required_string(command, "challengeTaskId")?;
            let principal_id = caller_principal(command)?;
            let operation_id = required_string(command, "operationId")?;
            let cancelled_at = Utc::now().to_rfc3339();
            let (record, replayed) = repository.mutate(|state| {
                state
                    .cancel_service_challenge_task(ServiceChallengeTaskCancelInput {
                        task_id: task_id.clone(),
                        principal_id: principal_id.clone(),
                        operation_id: operation_id.clone(),
                        cancelled_at: cancelled_at.clone(),
                    })
                    .map_err(|error| error.cli_message())
            })?;
            Ok(task_projection(&record, replayed))
        }
        _ => Err(format!("challenge_task_action_unsupported:{action}")),
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn complete_provider_free_test_task(
    state: &mut ServiceState,
    service_name: &str,
    agent_name: &str,
    task_name: &str,
    principal_id: &str,
    site_policy_id: &str,
    downstream_intent_id: &str,
    fixture_scenario_id: &str,
) -> Result<String, String> {
    let supplied_handle = state
        .tabs
        .keys()
        .find_map(|tab_id| state.service_tab_handle(tab_id))
        .ok_or_else(|| "challenge_consumer_test_handle_missing".to_string())?;
    let command = json!({
        "action": "service_challenge_task_start",
        "serviceName": service_name,
        "agentName": agent_name,
        "taskName": task_name,
        "clientSubjectId": principal_id,
        "challengeProfileId": "turnstile-checkbox-p169-v1",
        "sitePolicyDigest": effective_site_policy_digest(state, site_policy_id)?,
        "downstreamIntentId": downstream_intent_id,
        "fixtureScenarioId": fixture_scenario_id,
        "idempotencyKey": format!("test-{downstream_intent_id}"),
        "deadlineMs": 120000,
        "maxTransitions": 8,
        "serviceTabHandle": supplied_handle,
    });
    let input =
        challenge_task_start_input(state, parse_start_intent(&command)?, "2026-09-16T00:00:00Z")?;
    let (record, _) = state
        .start_service_challenge_task(input)
        .map_err(|error| error.cli_message())?;
    let (completed, _) = state
        .resume_service_challenge_task(ServiceChallengeTaskResumeInput {
            task_id: record.task_id,
            principal_id: principal_id.to_string(),
            operation_id: "complete-provider-free-test-task".to_string(),
            resumed_at: "2026-09-16T00:01:00Z".to_string(),
        })
        .map_err(|error| error.cli_message())?;
    Ok(completed.task_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserProcess, BrowserSession, BrowserTab, LeaseState, ProfileOrigin,
        ServiceState, ServiceTabHandle, ServiceTabHandleTraceFilter, TabLifecycle,
    };
    use crate::native::service_profile_access_policy::{
        ProfileChildAccess, ProfileConnectionState, ProfileIdentityAssurance,
    };
    use serde_json::{json, Value};

    fn service_state() -> ServiceState {
        let profile_access = ProfileChildAccess {
            schema_version: "agent-browser.profile-child-access.v1".to_string(),
            parent_policy_revision: 1,
            access_decision_id: "decision-1".to_string(),
            subject_id: Some("principal-1".to_string()),
            identity_assurance: ProfileIdentityAssurance::AuthenticatedIngress,
            connection_instance_id: Some("connection-1".to_string()),
            connection_state: ProfileConnectionState::Active,
            permissions: Vec::new(),
        };
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
                service_name: Some("consumer-service".to_string()),
                agent_name: Some("challenge-worker".to_string()),
                task_name: Some("challenge-aware-task".to_string()),
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
                profile_access: Some(profile_access),
                ..BrowserTab::default()
            },
        );
        state.site_policies.insert(
            "example".to_string(),
            crate::native::service_model::SitePolicy {
                id: "example".to_string(),
                name: "Example".to_string(),
                origin_pattern: "https://example.test".to_string(),
                ..crate::native::service_model::SitePolicy::default()
            },
        );
        state
    }

    fn start_command(state: &ServiceState) -> Value {
        let current: ServiceTabHandle = state.service_tab_handle("tab-1").unwrap();
        assert_eq!(current.profile_origin, ProfileOrigin::AgentBrowserOwned);
        assert_eq!(
            current.trace_filter,
            ServiceTabHandleTraceFilter {
                browser_id: Some("browser-1".to_string()),
                profile_id: Some("profile-1".to_string()),
                session_id: Some("session-1".to_string()),
                service_name: Some("consumer-service".to_string()),
                agent_name: Some("challenge-worker".to_string()),
                task_name: Some("challenge-aware-task".to_string()),
            }
        );
        json!({
            "action": "service_challenge_task_start",
            "serviceName": "consumer-service",
            "agentName": "challenge-worker",
            "taskName": "challenge-aware-task",
            "clientSubjectId": "principal-1",
            "challengeProfileId": "turnstile-checkbox-p169-v1",
            "sitePolicyDigest": "a".repeat(64),
            "downstreamIntentId": "authenticate-account",
            "fixtureScenarioId": "pass_after_acknowledged_resolution",
            "idempotencyKey": "challenge-task-idempotency-1",
            "deadlineMs": 120000,
            "maxTransitions": 8,
            "serviceTabHandle": current,
        })
    }

    #[test]
    fn start_is_exact_handle_bound_idempotent_and_secret_free() {
        let mut state = service_state();
        let command = start_command(&state);
        let intent = parse_start_intent(&command).unwrap();
        let input = challenge_task_start_input(&state, intent, "2026-09-15T12:00:00Z").unwrap();
        let (first, replayed) = state.start_service_challenge_task(input).unwrap();
        assert!(!replayed);

        let replay_intent = parse_start_intent(&command).unwrap();
        let replay_input =
            challenge_task_start_input(&state, replay_intent, "2026-09-15T12:01:00Z").unwrap();
        let (second, replayed) = state.start_service_challenge_task(replay_input).unwrap();
        assert!(replayed);
        assert_eq!(first, second);
        assert_eq!(state.challenge_tasks.len(), 1);

        let mut conflicting = command.clone();
        conflicting["downstreamIntentId"] = json!("different-intent");
        let conflicting_input = challenge_task_start_input(
            &state,
            parse_start_intent(&conflicting).unwrap(),
            "2026-09-15T12:01:00Z",
        )
        .unwrap();
        assert_eq!(
            state
                .start_service_challenge_task(conflicting_input)
                .unwrap_err()
                .cli_message(),
            "challenge_task_idempotency_conflict",
        );

        let projection = task_projection(&first, false).to_string();
        assert!(!projection.contains("challenge-task-idempotency-1"));
        let encoded = serde_json::to_value(&state).unwrap();
        let decoded: ServiceState = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.challenge_tasks, state.challenge_tasks);
    }

    #[test]
    fn status_resume_and_cancel_enforce_ownership_and_terminal_replay() {
        let mut state = service_state();
        let command = start_command(&state);
        let input = challenge_task_start_input(
            &state,
            parse_start_intent(&command).unwrap(),
            "2026-09-15T12:00:00Z",
        )
        .unwrap();
        let (record, _) = state.start_service_challenge_task(input).unwrap();

        assert_eq!(
            state
                .status_service_challenge_task(&record.task_id, "different-principal")
                .unwrap_err()
                .cli_message(),
            "challenge_task_principal_mismatch",
        );
        let (completed, replayed) = state
            .resume_service_challenge_task(ServiceChallengeTaskResumeInput {
                task_id: record.task_id.clone(),
                principal_id: "principal-1".to_string(),
                operation_id: "resume-operation-1".to_string(),
                resumed_at: "2026-09-15T12:01:00Z".to_string(),
            })
            .unwrap();
        assert!(!replayed);
        assert_eq!(completed.state, ServiceChallengeTaskState::Completed);
        assert_eq!(completed.receipt.as_ref().unwrap()["admission"], "admitted");
        assert_eq!(completed.receipt.as_ref().unwrap()["emittedEffects"], false);
        assert!(!serde_json::to_string(&state)
            .unwrap()
            .contains("resume-operation-1"));
        assert_eq!(
            agent_browser_service_model::challenge_task_summary(&state.challenge_tasks)
                .terminal_count,
            1
        );
        assert_eq!(
            agent_browser_service_model::challenge_task_summary(&state.challenge_tasks)
                .cooldown_count,
            1
        );

        let (replayed_record, replayed) = state
            .resume_service_challenge_task(ServiceChallengeTaskResumeInput {
                task_id: record.task_id.clone(),
                principal_id: "principal-1".to_string(),
                operation_id: "resume-operation-1".to_string(),
                resumed_at: "2026-09-15T12:02:00Z".to_string(),
            })
            .unwrap();
        assert!(replayed);

        let mut expired_state = service_state();
        let expired_command = start_command(&expired_state);
        let expired_input = challenge_task_start_input(
            &expired_state,
            parse_start_intent(&expired_command).unwrap(),
            "2026-09-15T12:00:00Z",
        )
        .unwrap();
        let (expired, _) = expired_state
            .start_service_challenge_task(expired_input)
            .unwrap();
        assert_eq!(
            expired_state
                .resume_service_challenge_task(ServiceChallengeTaskResumeInput {
                    task_id: expired.task_id,
                    principal_id: "principal-1".to_string(),
                    operation_id: "expired-operation-1".to_string(),
                    resumed_at: "2026-09-15T12:03:00Z".to_string(),
                })
                .unwrap_err()
                .cli_message(),
            "challenge_task_deadline_exceeded",
        );
        assert_eq!(replayed_record, completed);

        let mut cancel_state = service_state();
        let cancel_command = start_command(&cancel_state);
        let cancel_input = challenge_task_start_input(
            &cancel_state,
            parse_start_intent(&cancel_command).unwrap(),
            "2026-09-15T12:00:00Z",
        )
        .unwrap();
        let (cancel_record, _) = cancel_state
            .start_service_challenge_task(cancel_input)
            .unwrap();
        let (cancelled, replayed) = cancel_state
            .cancel_service_challenge_task(ServiceChallengeTaskCancelInput {
                task_id: cancel_record.task_id.clone(),
                principal_id: "principal-1".to_string(),
                operation_id: "cancel-operation-1".to_string(),
                cancelled_at: "2026-09-15T12:01:00Z".to_string(),
            })
            .unwrap();
        assert!(!replayed);
        assert_eq!(cancelled.state, ServiceChallengeTaskState::Cancelled);
        let (_, replayed) = cancel_state
            .cancel_service_challenge_task(ServiceChallengeTaskCancelInput {
                task_id: cancel_record.task_id,
                principal_id: "principal-1".to_string(),
                operation_id: "cancel-operation-1".to_string(),
                cancelled_at: "2026-09-15T12:02:00Z".to_string(),
            })
            .unwrap();
        assert!(replayed);
    }

    fn completed_consumer_task(
        state: &mut ServiceState,
        fixture: &str,
        downstream_intent_id: &str,
    ) -> ServiceChallengeTaskRecord {
        let mut command = start_command(state);
        command["sitePolicyDigest"] =
            json!(effective_site_policy_digest(state, "example").unwrap());
        command["downstreamIntentId"] = json!(downstream_intent_id);
        command["fixtureScenarioId"] = json!(fixture);
        let input = challenge_task_start_input(
            state,
            parse_start_intent(&command).unwrap(),
            "2026-09-15T12:00:00Z",
        )
        .unwrap();
        let (record, _) = state.start_service_challenge_task(input).unwrap();
        state
            .resume_service_challenge_task(ServiceChallengeTaskResumeInput {
                task_id: record.task_id,
                principal_id: "principal-1".to_string(),
                operation_id: "complete-consumer-task".to_string(),
                resumed_at: "2026-09-15T12:01:00Z".to_string(),
            })
            .unwrap()
            .0
    }

    #[test]
    fn shared_consumer_adapter_binds_policy_principal_intent_and_current_tab() {
        let mut state = service_state();
        let completed = completed_consumer_task(
            &mut state,
            "pass_after_acknowledged_resolution",
            "authentication-run-start",
        );
        let handle = state.service_tab_handle("tab-1").unwrap();

        let authentication = admit_challenge_consumer(
            &state,
            &completed.task_id,
            "principal-1",
            &handle,
            "example",
            "authentication-run-start",
            "auth-operation-1",
            ChallengeConsumerKind::Authentication,
        )
        .unwrap();
        assert_eq!(
            authentication.consumer,
            ChallengeConsumerKind::Authentication
        );

        assert_eq!(
            admit_challenge_consumer(
                &state,
                &completed.task_id,
                "principal-2",
                &handle,
                "example",
                "authentication-run-start",
                "auth-operation-2",
                ChallengeConsumerKind::Authentication,
            )
            .unwrap_err(),
            "challenge_task_principal_mismatch"
        );

        let mut stale_handle = handle.clone();
        stale_handle.valid = false;
        assert_eq!(
            admit_challenge_consumer(
                &state,
                &completed.task_id,
                "principal-1",
                &stale_handle,
                "example",
                "authentication-run-start",
                "auth-operation-3",
                ChallengeConsumerKind::Authentication,
            )
            .unwrap_err(),
            "challenge_consumer_service_tab_handle_mismatch"
        );

        let mut changed_policy = state.clone();
        changed_policy
            .site_policies
            .get_mut("example")
            .unwrap()
            .name = "Changed Example".to_string();
        assert_eq!(
            admit_challenge_consumer(
                &changed_policy,
                &completed.task_id,
                "principal-1",
                &handle,
                "example",
                "authentication-run-start",
                "auth-operation-4",
                ChallengeConsumerKind::Authentication,
            )
            .unwrap_err(),
            "challenge_consumer_site_policy_mismatch"
        );
    }

    #[test]
    fn shared_consumer_adapter_withholds_before_navigation_dispatch() {
        let mut state = service_state();
        let completed =
            completed_consumer_task(&mut state, "rejected_resolution", "navigation-dispatch");
        let handle = state.service_tab_handle("tab-1").unwrap();
        assert_eq!(
            admit_challenge_consumer(
                &state,
                &completed.task_id,
                "principal-1",
                &handle,
                "example",
                "navigation-dispatch",
                "navigation-operation-1",
                ChallengeConsumerKind::Navigation,
            )
            .unwrap_err(),
            "challenge_consumer_admission_withheld"
        );
    }
}
