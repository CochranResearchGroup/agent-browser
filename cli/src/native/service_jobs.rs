//! Repository-backed service job operations.

use serde_json::json;

use super::service_failure::attach_service_failure_recourse;
use super::service_model::{JobState, ServiceEvent, ServiceEventKind, ServiceJob, ServiceState};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use super::service_terminal_outcome::{
    ServiceTerminalOutcome, ServiceTerminalPhase, ServiceTerminalState,
};

pub const MAX_SERVICE_JOBS: usize = 200;
/// Every persisted running job needs a bounded recovery horizon. Callers may
/// choose a larger explicit timeout for long work; legacy records without one
/// use this conservative fallback during reconciliation.
pub const DEFAULT_SERVICE_JOB_TIMEOUT_MS: u64 = 15 * 60 * 1_000;

pub fn mutate_persisted_service_jobs(mutator: impl FnOnce(&mut ServiceState)) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ = mutate_service_jobs_in_repository(&repository, mutator);
    }
}

pub fn mutate_service_jobs_in_repository(
    repository: &impl ServiceStateRepository,
    mutator: impl FnOnce(&mut ServiceState),
) -> Result<(), String> {
    repository.mutate(|state| {
        mutator(state);
        prune_service_jobs(state);
        Ok(())
    })
}

/// Convert a running record whose persisted deadline has elapsed into a
/// terminal, effect-uncertain timeout. This repairs state left behind when a
/// worker exits or loses its terminal persistence race. It does not retry or
/// compensate the original operation.
pub fn reconcile_stale_running_service_jobs(state: &mut ServiceState, now: &str) -> Vec<String> {
    let Ok(parsed_now) = chrono::DateTime::parse_from_rfc3339(now) else {
        return Vec::new();
    };
    let mut reconciled = Vec::new();
    let mut terminal_events = Vec::new();
    for job in state
        .jobs
        .values_mut()
        .filter(|job| job.state == JobState::Running)
    {
        let Some(started_at) = job.started_at.as_deref().or(job.submitted_at.as_deref()) else {
            continue;
        };
        let Ok(started_at) = chrono::DateTime::parse_from_rfc3339(started_at) else {
            continue;
        };
        let timeout_ms = job
            .timeout_ms
            .filter(|timeout_ms| *timeout_ms > 0)
            .unwrap_or(DEFAULT_SERVICE_JOB_TIMEOUT_MS);
        let elapsed_ms = parsed_now
            .signed_duration_since(started_at)
            .num_milliseconds();
        if elapsed_ms < 0 || i128::from(elapsed_ms) < i128::from(timeout_ms) {
            continue;
        }
        let error = format!(
            "Service job exceeded its persisted {}ms deadline and was reconciled",
            timeout_ms
        );
        let outcome = terminal_outcome_for_job(
            job,
            &error,
            ServiceTerminalState::TimedOut,
            ServiceTerminalPhase::Finalize,
            now,
        );
        job.state = JobState::TimedOut;
        job.completed_at = Some(now.to_string());
        job.error = Some(error.clone());
        job.result = Some(json!({
            "success": false,
            "timedOut": true,
            "reconciled": true,
            "effectUncertain": true,
            "timeoutMs": timeout_ms,
        }));
        job.failure = outcome.failure.clone();
        job.terminal_outcome = Some(outcome.clone());
        terminal_events.push(terminal_event_for_job(job, outcome, &error));
        reconciled.push(job.id.clone());
    }
    state.events.extend(terminal_events);
    if state.events.len() > 100 {
        let excess = state.events.len() - 100;
        state.events.drain(0..excess);
    }
    reconciled
}

pub fn cancel_persisted_service_job(
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    LockedServiceStateRepository::default_json()
        .and_then(|repository| cancel_service_job_in_repository(&repository, job_id, reason))
        .map_err(cancel_persisted_service_job_response_error)
}

pub fn cancel_service_job_in_repository(
    repository: &impl ServiceStateRepository,
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    repository.mutate(|state| cancel_service_job_in_state(state, job_id, reason))
}

fn cancel_service_job_in_state(
    state: &mut super::service_model::ServiceState,
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    let job = state
        .jobs
        .get_mut(job_id)
        .ok_or_else(|| format!("Service job not found: {}", job_id))?;

    match job.state {
        JobState::Queued | JobState::WaitingProfileLease => {
            let completed_at = current_timestamp();
            let error = reason
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Cancelled by operator")
                .to_string();
            let outcome = terminal_outcome_for_job(
                job,
                &error,
                ServiceTerminalState::Cancelled,
                ServiceTerminalPhase::Dispatch,
                &completed_at,
            );
            job.state = JobState::Cancelled;
            job.completed_at = Some(completed_at);
            job.error = Some(error.clone());
            job.result = Some(json!({ "success": false, "cancelled": true }));
            job.failure = outcome.failure.clone();
            job.terminal_outcome = Some(outcome.clone());
            let result = job.clone();
            let event = terminal_event_for_job(job, outcome, &error);
            state.events.push(event);
            if state.events.len() > 100 {
                let excess = state.events.len() - 100;
                state.events.drain(0..excess);
            }
            Ok(result)
        }
        JobState::Cancelled => Ok(job.clone()),
        JobState::Running => Err(format!(
            "Service job {} is already running and cannot be cancelled safely",
            job_id
        )),
        JobState::Succeeded | JobState::Failed | JobState::TimedOut => Err(format!(
            "Service job {} is already terminal with state {}",
            job_id,
            job_state_name(job.state)
        )),
    }
}

pub(crate) fn cancel_profile_eviction_jobs_in_state(
    state: &mut super::service_model::ServiceState,
    profile_id: &str,
    tab_id: &str,
    evicted_subject_id: Option<&str>,
) -> Result<Vec<String>, String> {
    let job_ids = state
        .jobs
        .values()
        .filter(|job| matches!(job.state, JobState::Queued | JobState::WaitingProfileLease))
        .filter(|job| {
            job.provenance.tab_id.as_deref() == Some(tab_id)
                || (job.provenance.profile_id.as_deref() == Some(profile_id)
                    && evicted_subject_id.is_some()
                    && job.provenance.client_subject_id.as_deref() == evicted_subject_id)
        })
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();
    for job_id in &job_ids {
        cancel_service_job_in_state(
            state,
            job_id,
            Some("Cancelled by authorized profile eviction"),
        )?;
    }
    Ok(job_ids)
}

fn terminal_outcome_for_job(
    job: &ServiceJob,
    error: &str,
    state: ServiceTerminalState,
    phase: ServiceTerminalPhase,
    completed_at: &str,
) -> ServiceTerminalOutcome {
    let mut response = json!({
        "id": job.id,
        "success": false,
        "error": error,
    });
    attach_service_failure_recourse(&mut response);
    ServiceTerminalOutcome::from_response(
        &job.provenance,
        &response,
        state,
        phase,
        completed_at.to_string(),
    )
}

fn terminal_event_for_job(
    job: &ServiceJob,
    outcome: ServiceTerminalOutcome,
    message: &str,
) -> ServiceEvent {
    ServiceEvent {
        id: format!("service-job-terminal-{}", uuid::Uuid::new_v4()),
        timestamp: outcome.completed_at.clone(),
        kind: ServiceEventKind::JobTerminal,
        message: message.to_string(),
        browser_id: job.provenance.browser_id.clone(),
        profile_id: job.provenance.profile_id.clone(),
        session_id: job.provenance.session_id.clone(),
        service_name: job.service_name.clone(),
        agent_name: job.agent_name.clone(),
        task_name: job.task_name.clone(),
        provenance: Some(job.provenance.clone()),
        terminal_outcome: Some(outcome),
        details: Some(json!({ "jobId": job.id, "action": job.action })),
        ..ServiceEvent::default()
    }
}

pub fn load_service_job_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Option<ServiceJob> {
    repository.load_snapshot().ok()?.jobs.remove(id)
}

pub fn cancel_persisted_service_job_response_error(err: String) -> String {
    if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
        format!("Unable to load service state: {}", err)
    } else {
        err
    }
}

fn prune_service_jobs(state: &mut ServiceState) {
    if state.jobs.len() <= MAX_SERVICE_JOBS {
        return;
    }
    let mut jobs = state
        .jobs
        .values()
        .map(|job| (job.submitted_at.clone().unwrap_or_default(), job.id.clone()))
        .collect::<Vec<_>>();
    jobs.sort();
    let excess = state.jobs.len() - MAX_SERVICE_JOBS;
    for (_, id) in jobs.into_iter().take(excess) {
        state.jobs.remove(&id);
    }
}

fn job_state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::WaitingProfileLease => "waiting_profile_lease",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::TimedOut => "timed_out",
    }
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn reconciliation_terminalizes_only_overdue_running_jobs() {
        let mut state = ServiceState {
            jobs: BTreeMap::from([
                (
                    "overdue".to_string(),
                    ServiceJob {
                        id: "overdue".to_string(),
                        action: "tab_new".to_string(),
                        state: JobState::Running,
                        started_at: Some("2026-08-30T12:00:00Z".to_string()),
                        timeout_ms: Some(1_000),
                        ..ServiceJob::default()
                    },
                ),
                (
                    "active".to_string(),
                    ServiceJob {
                        id: "active".to_string(),
                        action: "navigate".to_string(),
                        state: JobState::Running,
                        started_at: Some("2026-08-30T12:00:01.500Z".to_string()),
                        timeout_ms: Some(1_000),
                        ..ServiceJob::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let reconciled = reconcile_stale_running_service_jobs(&mut state, "2026-08-30T12:00:02Z");

        assert_eq!(reconciled, vec!["overdue"]);
        assert_eq!(state.jobs["overdue"].state, JobState::TimedOut);
        assert_eq!(
            state.jobs["overdue"].result.as_ref().unwrap()["effectUncertain"],
            true
        );
        let outcome = state.jobs["overdue"].terminal_outcome.as_ref().unwrap();
        assert_eq!(outcome.state, ServiceTerminalState::TimedOut);
        assert_eq!(outcome.phase, ServiceTerminalPhase::Finalize);
        assert_eq!(
            outcome.failure.as_ref().unwrap().code,
            "service_job_timed_out"
        );
        let event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::JobTerminal)
            .unwrap();
        assert_eq!(event.terminal_outcome.as_ref(), Some(outcome));
        assert_eq!(
            event.provenance.as_ref(),
            Some(&state.jobs["overdue"].provenance)
        );
        assert_eq!(state.jobs["active"].state, JobState::Running);
    }

    #[test]
    fn reconciliation_bounds_legacy_running_jobs_without_timeout_metadata() {
        let mut state = ServiceState {
            jobs: BTreeMap::from([(
                "legacy".to_string(),
                ServiceJob {
                    id: "legacy".to_string(),
                    state: JobState::Running,
                    started_at: Some("2026-08-30T12:00:00Z".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };

        let reconciled = reconcile_stale_running_service_jobs(&mut state, "2026-08-30T12:15:00Z");

        assert_eq!(reconciled, vec!["legacy"]);
        assert_eq!(state.jobs["legacy"].state, JobState::TimedOut);
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::service_jobs::cancel_persisted_service_job;
    use crate::native::service_model::{
        retained_display_allocation_candidates, service_profile_allocations,
        service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
        BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
        BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession,
        BrowserTab, ControlInputProvider, DisplayAllocation, JobState as ServiceJobState,
        LeaseState, MonitorState, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
        ProfileLeaseDisposition, ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease,
        RemoteViewHandoff, RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent,
        ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle,
        ViewStream, ViewStreamProvider, ViewerLease,
    };
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    use crate::native::service_trace::service_commands::{
        parse_service_event_timestamp, service_job_at_or_after, service_job_matches_trace_filters,
        service_job_state_name,
    };
    use crate::native::state;
    use serde_json::{json, Map, Value};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};
    pub(crate) async fn handle_service_job_cancel(cmd: &Value) -> Result<Value, String> {
        let job_id = cmd
            .get("jobId")
            .and_then(|value| value.as_str())
            .ok_or("Missing jobId")?;
        let reason = cmd.get("reason").and_then(|value| value.as_str());
        let job = cancel_persisted_service_job(job_id, reason)?;
        Ok(json!({ "cancelled" : true, "job" : job, }))
    }
    pub(crate) async fn handle_service_jobs(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let limit = cmd
            .get("limit")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(20);
        let state = cmd.get("state").and_then(|value| value.as_str());
        let action = cmd.get("jobAction").and_then(|value| value.as_str());
        let profile_id = cmd.get("profileId").and_then(|value| value.as_str());
        let session_id = cmd.get("sessionId").and_then(|value| value.as_str());
        let service_name = cmd.get("serviceName").and_then(|value| value.as_str());
        let agent_name = cmd.get("agentName").and_then(|value| value.as_str());
        let task_name = cmd.get("taskName").and_then(|value| value.as_str());
        let since = cmd
            .get("since")
            .and_then(|value| value.as_str())
            .map(parse_service_event_timestamp)
            .transpose()?;
        let total = service_state.jobs.len();
        if let Some(job_id) = cmd.get("jobId").and_then(|value| value.as_str()) {
            let job = service_state
                .jobs
                .get(job_id)
                .cloned()
                .ok_or_else(|| format!("Service job not found: {}", job_id))?;
            return Ok(json!(
                { "job" : job, "jobs" : [job], "count" : 1, "matched" : 1, "total" :
                total, }
            ));
        }
        let mut jobs = service_state.jobs.values().cloned().collect::<Vec<_>>();
        jobs.sort_by(|left, right| {
            let left_time = left.submitted_at.as_deref().unwrap_or_default();
            let right_time = right.submitted_at.as_deref().unwrap_or_default();
            left_time
                .cmp(right_time)
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut jobs = jobs
            .into_iter()
            .filter(|job| {
                state.is_none_or(|expected| service_job_state_name(job.state) == expected)
                    && action.is_none_or(|expected| job.action == expected)
                    && service_job_matches_trace_filters(
                        job,
                        &service_state,
                        profile_id,
                        session_id,
                        service_name,
                        agent_name,
                        task_name,
                    )
                    && since.is_none_or(|minimum| service_job_at_or_after(job, minimum))
            })
            .collect::<Vec<_>>();
        let matched = jobs.len();
        let start = matched.saturating_sub(limit);
        jobs = jobs[start..].to_vec();
        Ok(json!(
            { "jobs" : jobs, "count" : jobs.len(), "matched" : matched, "total" :
            total, }
        ))
    }
}
pub(crate) use service_commands::*;
