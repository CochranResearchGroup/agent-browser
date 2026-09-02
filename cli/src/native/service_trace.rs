use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};

use super::service_activity::service_incident_activity_items;
use super::service_model::{
    JobTarget, ServiceEvent, ServiceEventKind, ServiceIncident, ServiceJob, ServiceState,
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ServiceTraceFilters<'a> {
    pub limit: usize,
    pub browser_id: Option<&'a str>,
    pub profile_id: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub service_name: Option<&'a str>,
    pub agent_name: Option<&'a str>,
    pub task_name: Option<&'a str>,
    pub since: Option<&'a str>,
}

/// Join retained service records into one trace response for MCP, CLI, and HTTP.
pub(crate) fn service_trace_response(
    service_state: &ServiceState,
    filters: ServiceTraceFilters<'_>,
) -> Result<Value, String> {
    let limit = if filters.limit == 0 {
        20
    } else {
        filters.limit
    };
    let since = filters
        .since
        .map(parse_service_trace_timestamp)
        .transpose()?;

    let total_events = service_state.events.len();
    let mut events = service_state
        .events
        .iter()
        .filter(|event| {
            filters
                .browser_id
                .is_none_or(|expected| event.browser_id.as_deref() == Some(expected))
                && service_event_matches_trace_filters(event, &filters)
                && since.is_none_or(|minimum| service_event_at_or_after(event, minimum))
        })
        .cloned()
        .collect::<Vec<_>>();
    let matched_events = events.len();
    events = tail_limit(events, limit);

    let total_jobs = service_state.jobs.len();
    let mut jobs = service_state.jobs.values().cloned().collect::<Vec<_>>();
    jobs.sort_by(|left, right| {
        let left_time = left.submitted_at.as_deref().unwrap_or_default();
        let right_time = right.submitted_at.as_deref().unwrap_or_default();
        left_time
            .cmp(right_time)
            .then_with(|| left.id.cmp(&right.id))
    });
    let jobs =
        jobs.into_iter()
            .filter(|job| {
                filters.browser_id.is_none_or(|expected| {
                    service_job_browser_id(job, service_state) == Some(expected)
                }) && service_job_matches_trace_filters(job, service_state, &filters)
                    && since.is_none_or(|minimum| service_job_at_or_after(job, minimum))
            })
            .collect::<Vec<_>>();
    let matched_jobs = jobs.len();
    let jobs = tail_limit(jobs, limit);

    let total_incidents = service_state.incidents.len();
    let incidents = service_state
        .incidents
        .iter()
        .filter(|incident| {
            filters
                .browser_id
                .is_none_or(|expected| incident.browser_id.as_deref() == Some(expected))
                && service_incident_matches_trace_filters(incident, service_state, &filters)
                && since.is_none_or(|minimum| service_incident_at_or_after(incident, minimum))
        })
        .cloned()
        .collect::<Vec<_>>();
    let matched_incidents = incidents.len();
    let incidents = tail_limit(incidents, limit);

    let mut activity = incidents
        .iter()
        .flat_map(|incident| service_incident_activity_items(service_state, incident))
        .filter(|item| {
            since.is_none_or(|minimum| {
                item.get("timestamp")
                    .and_then(|value| value.as_str())
                    .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
                    .is_some_and(|timestamp| timestamp >= minimum)
            })
        })
        .collect::<Vec<_>>();
    activity.sort_by(|left, right| {
        let left_timestamp = left
            .get("timestamp")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let right_timestamp = right
            .get("timestamp")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let left_id = left
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let right_id = right
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        left_timestamp
            .cmp(right_timestamp)
            .then_with(|| left_id.cmp(right_id))
    });
    activity.dedup_by(|left, right| left.get("id") == right.get("id"));
    let matched_activity = activity.len();
    activity = tail_limit(activity, limit);
    let event_count = events.len();
    let job_count = jobs.len();
    let incident_count = incidents.len();
    let activity_count = activity.len();
    let summary = service_trace_summary(
        service_state,
        &events,
        &jobs,
        &incidents,
        &activity,
        &filters,
        since,
    );

    Ok(json!({
        "filters": {
            "browserId": filters.browser_id,
            "profileId": filters.profile_id,
            "sessionId": filters.session_id,
            "serviceName": filters.service_name,
            "agentName": filters.agent_name,
            "taskName": filters.task_name,
            "since": filters.since,
            "limit": limit,
        },
        "events": events,
        "jobs": jobs,
        "incidents": incidents,
        "activity": activity,
        "summary": summary,
        "counts": {
            "events": event_count,
            "jobs": job_count,
            "incidents": incident_count,
            "activity": activity_count,
        },
        "matched": {
            "events": matched_events,
            "jobs": matched_jobs,
            "incidents": matched_incidents,
            "activity": matched_activity,
        },
        "total": {
            "events": total_events,
            "jobs": total_jobs,
            "incidents": total_incidents,
        },
    }))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct TraceContextKey {
    service_name: Option<String>,
    agent_name: Option<String>,
    task_name: Option<String>,
    browser_id: Option<String>,
    profile_id: Option<String>,
    session_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TraceContextSummary {
    event_count: usize,
    job_count: usize,
    incident_count: usize,
    activity_count: usize,
    target_service_ids: Vec<String>,
    control_plane_modes: Vec<String>,
    display_allocations: Vec<String>,
    unrecorded_display_allocation_job_count: usize,
    lifecycle_only_job_count: usize,
    latest_timestamp: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ProfileLeaseWaitSummary {
    job_id: String,
    profile_id: Option<String>,
    outcome: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    waited_ms: Option<u64>,
    retry_after_ms: Option<u64>,
    conflict_session_ids: Vec<String>,
    service_name: Option<String>,
    agent_name: Option<String>,
    task_name: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct BrowserCapabilityLaunchContext<'a> {
    source: &'a str,
    timestamp: Option<&'a str>,
    browser_id: Option<&'a str>,
    profile_id: Option<&'a str>,
    session_id: Option<&'a str>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

/// Compact owner/context rollup for dashboards, MCP agents, and API clients.
fn service_trace_summary(
    service_state: &ServiceState,
    events: &[ServiceEvent],
    jobs: &[ServiceJob],
    incidents: &[ServiceIncident],
    activity: &[Value],
    filters: &ServiceTraceFilters<'_>,
    since: Option<DateTime<FixedOffset>>,
) -> Value {
    let mut contexts = BTreeMap::<TraceContextKey, TraceContextSummary>::new();

    for event in events {
        let key = TraceContextKey {
            service_name: event.service_name.clone(),
            agent_name: event.agent_name.clone(),
            task_name: event.task_name.clone(),
            browser_id: event.browser_id.clone(),
            profile_id: event.profile_id.clone(),
            session_id: event.session_id.clone(),
        };
        let summary = contexts.entry(key).or_default();
        summary.event_count += 1;
        update_latest_timestamp(
            &mut summary.latest_timestamp,
            Some(event.timestamp.as_str()),
        );
    }

    for job in jobs {
        let key = TraceContextKey {
            service_name: job.service_name.clone(),
            agent_name: job.agent_name.clone(),
            task_name: job.task_name.clone(),
            browser_id: service_job_browser_id(job, service_state).map(str::to_string),
            profile_id: service_job_profile_id(job, service_state).map(str::to_string),
            session_id: service_job_session_id(job, service_state).map(str::to_string),
        };
        let summary = contexts.entry(key).or_default();
        summary.job_count += 1;
        merge_job_target_service_ids(&mut summary.target_service_ids, job);
        merge_control_plane_mode(&mut summary.control_plane_modes, job);
        if let Some(display_isolation) = job.display_isolation.as_deref() {
            merge_string_value(&mut summary.display_allocations, display_isolation);
        } else {
            summary.unrecorded_display_allocation_job_count += 1;
        }
        if job.lifecycle_only {
            summary.lifecycle_only_job_count += 1;
        }
        update_latest_timestamp(
            &mut summary.latest_timestamp,
            service_job_latest_timestamp(job),
        );
    }

    for incident in incidents {
        let key = TraceContextKey {
            browser_id: incident.browser_id.clone(),
            ..TraceContextKey::default()
        };
        let summary = contexts.entry(key).or_default();
        summary.incident_count += 1;
        update_latest_timestamp(
            &mut summary.latest_timestamp,
            Some(incident.latest_timestamp.as_str()),
        );
    }

    for item in activity {
        let key = TraceContextKey {
            service_name: item
                .get("serviceName")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            agent_name: item
                .get("agentName")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            task_name: item
                .get("taskName")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            browser_id: item
                .get("browserId")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            profile_id: item
                .get("profileId")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            session_id: item
                .get("sessionId")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        };
        let summary = contexts.entry(key).or_default();
        summary.activity_count += 1;
        update_latest_timestamp(
            &mut summary.latest_timestamp,
            item.get("timestamp").and_then(|value| value.as_str()),
        );
    }

    let contexts = contexts
        .into_iter()
        .map(|(key, summary)| {
            let naming_warnings = trace_context_naming_warnings(&key);
            let attention =
                trace_context_attention(summary.incident_count, naming_warnings.as_slice());
            json!({
                "serviceName": key.service_name,
                "agentName": key.agent_name,
                "taskName": key.task_name,
                "browserId": key.browser_id,
                "profileId": key.profile_id,
                "sessionId": key.session_id,
                "namingWarnings": naming_warnings,
                "hasNamingWarning": !naming_warnings.is_empty(),
                "eventCount": summary.event_count,
                "jobCount": summary.job_count,
                "incidentCount": summary.incident_count,
                "activityCount": summary.activity_count,
                "targetIdentityCount": summary.target_service_ids.len(),
                "targetServiceIds": summary.target_service_ids,
                "controlPlaneModes": summary.control_plane_modes,
                "displayAllocations": summary.display_allocations,
                "unrecordedDisplayAllocationJobCount": summary.unrecorded_display_allocation_job_count,
                "lifecycleOnlyJobCount": summary.lifecycle_only_job_count,
                "attention": attention,
                "latestTimestamp": summary.latest_timestamp,
            })
        })
        .collect::<Vec<_>>();

    let context_count = contexts.len();
    let naming_warning_count = contexts
        .iter()
        .filter(|context| {
            context
                .get("hasNamingWarning")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .count();
    let has_trace_context = contexts.iter().any(|context| {
        context
            .get("serviceName")
            .is_some_and(|value| !value.is_null())
            || context
                .get("agentName")
                .is_some_and(|value| !value.is_null())
            || context
                .get("taskName")
                .is_some_and(|value| !value.is_null())
    });

    json!({
        "contextCount": context_count,
        "hasTraceContext": has_trace_context,
        "namingWarningCount": naming_warning_count,
        "browserCapabilityLaunches": service_trace_browser_capability_launch_summary(
            service_state,
            events,
            filters,
            since,
        ),
        "displayAllocations": service_trace_display_allocation_summary(jobs),
        "profileLeaseWaits": service_trace_profile_lease_wait_summary(events),
        "contexts": contexts,
    })
}

fn service_trace_display_allocation_summary(jobs: &[ServiceJob]) -> Value {
    let mut allocations = BTreeMap::<String, Vec<String>>::new();
    let mut unrecorded_job_ids = Vec::<String>::new();

    for job in jobs {
        if let Some(display_isolation) = job.display_isolation.as_deref() {
            allocations
                .entry(display_isolation.to_string())
                .or_default()
                .push(job.id.clone());
        } else {
            unrecorded_job_ids.push(job.id.clone());
        }
    }

    let allocation_rows = allocations
        .into_iter()
        .map(|(display_isolation, job_ids)| {
            json!({
                "displayIsolation": display_isolation,
                "label": service_trace_display_allocation_label(display_isolation.as_str()),
                "count": job_ids.len(),
                "jobIds": job_ids,
            })
        })
        .collect::<Vec<_>>();
    let recorded_count = allocation_rows
        .iter()
        .map(|row| {
            row.get("count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        })
        .sum::<u64>() as usize;
    let unrecorded_count = unrecorded_job_ids.len();

    json!({
        "count": jobs.len(),
        "recordedCount": recorded_count,
        "unrecordedCount": unrecorded_count,
        "privateVirtualDisplayCount": service_trace_display_allocation_count(&allocation_rows, "private_virtual_display"),
        "sharedDisplayCount": service_trace_display_allocation_count(&allocation_rows, "shared_display"),
        "ambientDisplayCount": service_trace_display_allocation_count(&allocation_rows, "ambient_display"),
        "allocations": allocation_rows,
        "unrecordedJobIds": unrecorded_job_ids,
    })
}

fn service_trace_display_allocation_count(rows: &[Value], display_isolation: &str) -> u64 {
    rows.iter()
        .find(|row| {
            row.get("displayIsolation").and_then(|value| value.as_str()) == Some(display_isolation)
        })
        .and_then(|row| row.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn service_trace_display_allocation_label(display_isolation: &str) -> &'static str {
    match display_isolation {
        "private_virtual_display" => "private display",
        "shared_display" => "shared display",
        "ambient_display" => "ambient display",
        _ => "unknown display",
    }
}

fn service_trace_browser_capability_launch_summary(
    service_state: &ServiceState,
    events: &[ServiceEvent],
    filters: &ServiceTraceFilters<'_>,
    since: Option<DateTime<FixedOffset>>,
) -> Value {
    let mut launches = BTreeMap::<String, Value>::new();

    for event in events {
        let Some(details) = event.details.as_ref() else {
            continue;
        };
        let Some(diagnostic) = details.get("browserCapabilityLaunch") else {
            continue;
        };
        let session_id = event.session_id.as_deref().or_else(|| {
            details
                .get("currentSessionIds")
                .and_then(|value| value.as_array())
                .and_then(|values| values.iter().find_map(|value| value.as_str()))
        });
        let browser_id = event.browser_id.as_deref();
        let key = browser_capability_launch_key(session_id, browser_id, Some(event.id.as_str()));
        launches.insert(
            key,
            compact_browser_capability_launch(
                diagnostic,
                BrowserCapabilityLaunchContext {
                    source: "event",
                    timestamp: Some(event.timestamp.as_str()),
                    browser_id,
                    profile_id: event.profile_id.as_deref(),
                    session_id,
                    service_name: event.service_name.as_deref(),
                    agent_name: event.agent_name.as_deref(),
                    task_name: event.task_name.as_deref(),
                },
            ),
        );
    }

    for session in service_state.sessions.values() {
        if !session_matches_browser_capability_trace_filters(session, filters, since) {
            continue;
        }
        let Some(diagnostic) = session.browser_capability_launch.as_ref() else {
            continue;
        };
        let browser_id = session.browser_ids.first().map(String::as_str);
        let key = browser_capability_launch_key(Some(session.id.as_str()), browser_id, None);
        launches.entry(key).or_insert_with(|| {
            compact_browser_capability_launch(
                diagnostic,
                BrowserCapabilityLaunchContext {
                    source: "session",
                    timestamp: session
                        .last_lease_observed_at
                        .as_deref()
                        .or(session.created_at.as_deref()),
                    browser_id,
                    profile_id: session.profile_id.as_deref(),
                    session_id: Some(session.id.as_str()),
                    service_name: session.service_name.as_deref(),
                    agent_name: session.agent_name.as_deref(),
                    task_name: session.task_name.as_deref(),
                },
            )
        });
    }

    let launches = launches.into_values().collect::<Vec<_>>();
    let applied_count = launches
        .iter()
        .filter(|launch| {
            launch
                .get("applied")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .count();
    let skipped_count = launches.len().saturating_sub(applied_count);
    json!({
        "count": launches.len(),
        "appliedCount": applied_count,
        "skippedCount": skipped_count,
        "launches": launches,
    })
}

fn browser_capability_launch_key(
    session_id: Option<&str>,
    browser_id: Option<&str>,
    event_id: Option<&str>,
) -> String {
    if session_id.is_some() || browser_id.is_some() {
        return format!("{}:{}", session_id.unwrap_or(""), browser_id.unwrap_or(""));
    }
    format!("event:{}", event_id.unwrap_or(""))
}

fn compact_browser_capability_launch(
    diagnostic: &Value,
    context: BrowserCapabilityLaunchContext<'_>,
) -> Value {
    json!({
        "source": context.source,
        "timestamp": context.timestamp,
        "serviceName": context.service_name,
        "agentName": context.agent_name,
        "taskName": context.task_name,
        "browserId": context.browser_id,
        "profileId": context.profile_id,
        "sessionId": context.session_id,
        "applied": diagnostic.get("applied").and_then(|value| value.as_bool()).unwrap_or(false),
        "reason": diagnostic.get("reason").and_then(|value| value.as_str()),
        "browserBuild": diagnostic.get("browserBuild").and_then(|value| value.as_str()),
        "bindingId": diagnostic.get("bindingId").and_then(|value| value.as_str()),
        "hostId": diagnostic.get("hostId").and_then(|value| value.as_str()),
        "executableId": diagnostic.get("executableId").and_then(|value| value.as_str()),
        "capabilityId": diagnostic.get("capabilityId").and_then(|value| value.as_str()),
        "executablePath": diagnostic.get("executablePath").and_then(|value| value.as_str()),
        "profileCompatibilityIds": diagnostic.get("profileCompatibilityIds").and_then(|value| value.as_array()).cloned().unwrap_or_default(),
        "validationEvidenceIds": diagnostic.get("validationEvidenceIds").and_then(|value| value.as_array()).cloned().unwrap_or_default(),
    })
}

fn session_matches_browser_capability_trace_filters(
    session: &super::service_model::BrowserSession,
    filters: &ServiceTraceFilters<'_>,
    since: Option<DateTime<FixedOffset>>,
) -> bool {
    filters.browser_id.is_none_or(|expected| {
        session
            .browser_ids
            .iter()
            .any(|browser_id| browser_id == expected)
    }) && filters
        .profile_id
        .is_none_or(|expected| session.profile_id.as_deref() == Some(expected))
        && filters
            .session_id
            .is_none_or(|expected| session.id == expected)
        && filters
            .service_name
            .is_none_or(|expected| session.service_name.as_deref() == Some(expected))
        && filters
            .agent_name
            .is_none_or(|expected| session.agent_name.as_deref() == Some(expected))
        && filters
            .task_name
            .is_none_or(|expected| session.task_name.as_deref() == Some(expected))
        && since.is_none_or(|minimum| {
            session
                .created_at
                .as_deref()
                .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
                .is_some_and(|timestamp| timestamp >= minimum)
        })
}

fn merge_job_target_service_ids(target_service_ids: &mut Vec<String>, job: &ServiceJob) {
    merge_optional_target_service_id(target_service_ids, job.target_service_id.as_deref());
    merge_optional_target_service_id(target_service_ids, job.site_id.as_deref());
    merge_optional_target_service_id(target_service_ids, job.login_id.as_deref());
    for target_service_id in &job.target_service_ids {
        merge_optional_target_service_id(target_service_ids, Some(target_service_id.as_str()));
    }
}

fn merge_optional_target_service_id(target_service_ids: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !target_service_ids.iter().any(|existing| existing == value) {
        target_service_ids.push(value.to_string());
    }
}

fn merge_control_plane_mode(control_plane_modes: &mut Vec<String>, job: &ServiceJob) {
    let Ok(value) = serde_json::to_value(job.control_plane_mode) else {
        return;
    };
    let Some(mode) = value.as_str().filter(|mode| !mode.is_empty()) else {
        return;
    };
    if !control_plane_modes.iter().any(|existing| existing == mode) {
        control_plane_modes.push(mode.to_string());
    }
}

fn merge_string_value(values: &mut Vec<String>, value: &str) {
    if value.is_empty() || values.iter().any(|existing| existing == value) {
        return;
    }
    values.push(value.to_string());
}

fn trace_context_attention(incident_count: usize, naming_warnings: &[&str]) -> Value {
    if incident_count > 0 {
        return json!({
            "required": true,
            "owner": "operator",
            "severity": "warning",
            "reason": "incidents_present",
            "message": "Trace context has retained incidents; inspect related incidents and activity before reusing this browser context.",
            "suggestedActions": [
                "inspect_incidents",
                "review_trace_activity",
                "apply_remedy_if_available"
            ],
            "presentation": "client_decides",
        });
    }

    if !naming_warnings.is_empty() {
        return json!({
            "required": true,
            "owner": "service",
            "severity": "info",
            "reason": "missing_caller_label",
            "message": "Trace context is missing service, agent, or task labels; future requests should include caller labels for deterministic multi-agent debugging.",
            "suggestedActions": [
                "include_service_name",
                "include_agent_name",
                "include_task_name"
            ],
            "presentation": "client_decides",
        });
    }

    json!({
        "required": false,
        "owner": "none",
        "severity": "info",
        "reason": "none",
        "message": "No trace-context intervention is required.",
        "suggestedActions": [],
        "presentation": "client_decides",
    })
}

fn service_trace_profile_lease_wait_summary(events: &[ServiceEvent]) -> Value {
    let mut waits = BTreeMap::<String, ProfileLeaseWaitSummary>::new();
    let mut wait_events = events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                ServiceEventKind::ProfileLeaseWaitStarted | ServiceEventKind::ProfileLeaseWaitEnded
            )
        })
        .collect::<Vec<_>>();
    wait_events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    for event in wait_events {
        let details = event.details.as_ref();
        let job_id = details
            .and_then(|details| details.get("jobId"))
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(&event.id)
            .to_string();
        let wait = waits
            .entry(job_id.clone())
            .or_insert_with(|| ProfileLeaseWaitSummary {
                job_id,
                ..ProfileLeaseWaitSummary::default()
            });
        wait.profile_id = event
            .profile_id
            .clone()
            .or_else(|| {
                details
                    .and_then(|details| details.get("profileId"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .or_else(|| wait.profile_id.clone());
        wait.service_name = wait.service_name.clone().or(event.service_name.clone());
        wait.agent_name = wait.agent_name.clone().or(event.agent_name.clone());
        wait.task_name = wait.task_name.clone().or(event.task_name.clone());
        wait.retry_after_ms = details
            .and_then(|details| details.get("retryAfterMs"))
            .and_then(|value| value.as_u64())
            .or(wait.retry_after_ms);
        let conflict_session_ids = details
            .and_then(|details| details.get("conflictSessionIds"))
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !conflict_session_ids.is_empty() {
            wait.conflict_session_ids = conflict_session_ids;
        }

        match event.kind {
            ServiceEventKind::ProfileLeaseWaitStarted => {
                wait.started_at = Some(event.timestamp.clone());
                wait.outcome = details
                    .and_then(|details| details.get("outcome"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| Some("started".to_string()));
            }
            ServiceEventKind::ProfileLeaseWaitEnded => {
                wait.ended_at = Some(event.timestamp.clone());
                wait.waited_ms = details
                    .and_then(|details| details.get("waitedMs"))
                    .and_then(|value| value.as_u64());
                wait.outcome = details
                    .and_then(|details| details.get("outcome"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| Some("ended".to_string()));
            }
            _ => {}
        }
    }

    let waits = waits
        .into_values()
        .map(|wait| {
            json!({
                "jobId": wait.job_id,
                "profileId": wait.profile_id,
                "outcome": wait.outcome.unwrap_or_else(|| "unknown".to_string()),
                "startedAt": wait.started_at,
                "endedAt": wait.ended_at,
                "waitedMs": wait.waited_ms,
                "retryAfterMs": wait.retry_after_ms,
                "conflictSessionIds": wait.conflict_session_ids,
                "serviceName": wait.service_name,
                "agentName": wait.agent_name,
                "taskName": wait.task_name,
            })
        })
        .collect::<Vec<_>>();
    let active_count = waits
        .iter()
        .filter(|wait| wait.get("endedAt").is_none_or(|value| value.is_null()))
        .count();
    let completed_count = waits.len().saturating_sub(active_count);
    json!({
        "count": waits.len(),
        "activeCount": active_count,
        "completedCount": completed_count,
        "waits": waits,
    })
}

fn trace_context_naming_warnings(key: &TraceContextKey) -> Vec<&'static str> {
    [
        key.service_name.is_none().then_some("missing_service_name"),
        key.agent_name.is_none().then_some("missing_agent_name"),
        key.task_name.is_none().then_some("missing_task_name"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn update_latest_timestamp(latest: &mut Option<String>, timestamp: Option<&str>) {
    let Some(timestamp) = timestamp.filter(|value| !value.is_empty()) else {
        return;
    };
    if latest.as_deref().is_none_or(|current| timestamp > current) {
        *latest = Some(timestamp.to_string());
    }
}

fn service_job_latest_timestamp(job: &ServiceJob) -> Option<&str> {
    job.completed_at
        .as_deref()
        .or(job.started_at.as_deref())
        .or(job.submitted_at.as_deref())
}

fn parse_service_trace_timestamp(raw: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(raw)
        .map_err(|err| format!("Invalid --since timestamp '{}': {}", raw, err))
}

fn tail_limit<T: Clone>(items: Vec<T>, limit: usize) -> Vec<T> {
    let start = items.len().saturating_sub(limit);
    items[start..].to_vec()
}

fn service_event_matches_trace_filters(
    event: &ServiceEvent,
    filters: &ServiceTraceFilters<'_>,
) -> bool {
    filters
        .profile_id
        .is_none_or(|expected| event.profile_id.as_deref() == Some(expected))
        && filters
            .session_id
            .is_none_or(|expected| event.session_id.as_deref() == Some(expected))
        && filters
            .service_name
            .is_none_or(|expected| event.service_name.as_deref() == Some(expected))
        && filters
            .agent_name
            .is_none_or(|expected| event.agent_name.as_deref() == Some(expected))
        && filters
            .task_name
            .is_none_or(|expected| event.task_name.as_deref() == Some(expected))
}

fn service_job_matches_trace_filters(
    job: &ServiceJob,
    service_state: &ServiceState,
    filters: &ServiceTraceFilters<'_>,
) -> bool {
    filters
        .profile_id
        .is_none_or(|expected| service_job_profile_id(job, service_state) == Some(expected))
        && filters
            .session_id
            .is_none_or(|expected| service_job_session_id(job, service_state) == Some(expected))
        && filters
            .service_name
            .is_none_or(|expected| job.service_name.as_deref() == Some(expected))
        && filters
            .agent_name
            .is_none_or(|expected| job.agent_name.as_deref() == Some(expected))
        && filters
            .task_name
            .is_none_or(|expected| job.task_name.as_deref() == Some(expected))
}

fn service_incident_matches_trace_filters(
    incident: &ServiceIncident,
    service_state: &ServiceState,
    filters: &ServiceTraceFilters<'_>,
) -> bool {
    if filters.profile_id.is_none()
        && filters.session_id.is_none()
        && filters.service_name.is_none()
        && filters.agent_name.is_none()
        && filters.task_name.is_none()
    {
        return true;
    }

    incident.event_ids.iter().any(|event_id| {
        service_state
            .events
            .iter()
            .find(|event| &event.id == event_id)
            .is_some_and(|event| service_event_matches_trace_filters(event, filters))
    }) || incident.job_ids.iter().any(|job_id| {
        service_state
            .jobs
            .get(job_id)
            .is_some_and(|job| service_job_matches_trace_filters(job, service_state, filters))
    })
}

fn service_job_at_or_after(job: &ServiceJob, minimum: DateTime<FixedOffset>) -> bool {
    job.submitted_at
        .as_deref()
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .is_some_and(|timestamp| timestamp >= minimum)
}

fn service_event_at_or_after(event: &ServiceEvent, minimum: DateTime<FixedOffset>) -> bool {
    DateTime::parse_from_rfc3339(&event.timestamp)
        .map(|timestamp| timestamp >= minimum)
        .unwrap_or(false)
}

fn service_incident_at_or_after(
    incident: &ServiceIncident,
    minimum: DateTime<FixedOffset>,
) -> bool {
    DateTime::parse_from_rfc3339(&incident.latest_timestamp)
        .map(|timestamp| timestamp >= minimum)
        .unwrap_or(false)
}

fn service_job_browser_id<'a>(
    job: &'a ServiceJob,
    service_state: &'a ServiceState,
) -> Option<&'a str> {
    match &job.target {
        JobTarget::Browser(browser_id) => Some(browser_id.as_str()),
        JobTarget::Tab(tab_id) => service_state
            .tabs
            .get(tab_id)
            .map(|tab| tab.browser_id.as_str()),
        JobTarget::Service
        | JobTarget::Profile(_)
        | JobTarget::Monitor(_)
        | JobTarget::Challenge(_) => None,
    }
}

fn service_job_profile_id<'a>(
    job: &'a ServiceJob,
    service_state: &'a ServiceState,
) -> Option<&'a str> {
    match &job.target {
        JobTarget::Profile(profile_id) => Some(profile_id.as_str()),
        JobTarget::Browser(browser_id) => service_state
            .browsers
            .get(browser_id)
            .and_then(|browser| browser.profile_id.as_deref()),
        JobTarget::Tab(tab_id) => service_state.tabs.get(tab_id).and_then(|tab| {
            tab.owner_session_id
                .as_deref()
                .and_then(|session_id| service_state.sessions.get(session_id))
                .and_then(|session| session.profile_id.as_deref())
                .or_else(|| {
                    service_state
                        .browsers
                        .get(&tab.browser_id)
                        .and_then(|browser| browser.profile_id.as_deref())
                })
        }),
        JobTarget::Service | JobTarget::Monitor(_) | JobTarget::Challenge(_) => None,
    }
}

fn service_job_session_id<'a>(
    job: &'a ServiceJob,
    service_state: &'a ServiceState,
) -> Option<&'a str> {
    match &job.target {
        JobTarget::Browser(browser_id) => service_state
            .browsers
            .get(browser_id)
            .and_then(|browser| browser.active_session_ids.first().map(String::as_str))
            .or_else(|| session_id_for_browser(service_state, browser_id)),
        JobTarget::Tab(tab_id) => service_state
            .tabs
            .get(tab_id)
            .and_then(|tab| tab.owner_session_id.as_deref()),
        JobTarget::Service
        | JobTarget::Profile(_)
        | JobTarget::Monitor(_)
        | JobTarget::Challenge(_) => None,
    }
}

fn session_id_for_browser<'a>(
    service_state: &'a ServiceState,
    browser_id: &str,
) -> Option<&'a str> {
    service_state
        .sessions
        .iter()
        .find_map(|(session_id, session)| {
            session
                .browser_ids
                .iter()
                .any(|id| id == browser_id)
                .then_some(session_id.as_str())
        })
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
    use crate::native::service_trace::{service_trace_response, ServiceTraceFilters};
    use crate::native::state;
    use chrono::{DateTime, FixedOffset};
    use serde_json::{json, Map, Value};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    pub(crate) async fn handle_service_trace(cmd: &Value) -> Result<Value, String> {
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
        let browser_id = cmd.get("browserId").and_then(|value| value.as_str());
        let profile_id = cmd.get("profileId").and_then(|value| value.as_str());
        let session_id = cmd.get("sessionId").and_then(|value| value.as_str());
        let service_name = cmd.get("serviceName").and_then(|value| value.as_str());
        let agent_name = cmd.get("agentName").and_then(|value| value.as_str());
        let task_name = cmd.get("taskName").and_then(|value| value.as_str());
        let since = cmd.get("since").and_then(|value| value.as_str());
        service_trace_response(
            &service_state,
            ServiceTraceFilters {
                limit,
                browser_id,
                profile_id,
                session_id,
                service_name,
                agent_name,
                task_name,
                since,
            },
        )
    }
    pub(crate) fn service_event_kind_name(kind: ServiceEventKind) -> &'static str {
        match kind {
            ServiceEventKind::Reconciliation => "reconciliation",
            ServiceEventKind::BrowserLaunchRecorded => "browser_launch_recorded",
            ServiceEventKind::BrowserHealthChanged => "browser_health_changed",
            ServiceEventKind::BrowserRecoveryStarted => "browser_recovery_started",
            ServiceEventKind::BrowserRecoveryOverride => "browser_recovery_override",
            ServiceEventKind::TabLifecycleChanged => "tab_lifecycle_changed",
            ServiceEventKind::ProfileLeaseWaitStarted => "profile_lease_wait_started",
            ServiceEventKind::ProfileLeaseWaitEnded => "profile_lease_wait_ended",
            ServiceEventKind::ProfileLeaseLifecycleChanged => "profile_lease_lifecycle_changed",
            ServiceEventKind::ViewerTakeoverRequested => "viewer_takeover_requested",
            ServiceEventKind::ViewerConnected => "viewer_connected",
            ServiceEventKind::ViewerDisconnected => "viewer_disconnected",
            ServiceEventKind::ControllerRequested => "controller_requested",
            ServiceEventKind::ControllerGranted => "controller_granted",
            ServiceEventKind::ControllerDenied => "controller_denied",
            ServiceEventKind::RouteReleased => "route_released",
            ServiceEventKind::ReconciliationError => "reconciliation_error",
            ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
            ServiceEventKind::IncidentResolved => "incident_resolved",
            ServiceEventKind::JobTerminal => "job_terminal",
        }
    }
    pub(crate) fn service_job_state_name(state: ServiceJobState) -> &'static str {
        match state {
            ServiceJobState::Queued => "queued",
            ServiceJobState::WaitingProfileLease => "waiting_profile_lease",
            ServiceJobState::Running => "running",
            ServiceJobState::Succeeded => "succeeded",
            ServiceJobState::Failed => "failed",
            ServiceJobState::Cancelled => "cancelled",
            ServiceJobState::TimedOut => "timed_out",
        }
    }
    pub(crate) fn service_incident_state_name(
        state: super::super::service_model::ServiceIncidentState,
    ) -> &'static str {
        match state {
            super::super::service_model::ServiceIncidentState::Active => "active",
            super::super::service_model::ServiceIncidentState::Recovered => "recovered",
            super::super::service_model::ServiceIncidentState::Service => "service",
        }
    }
    pub(crate) fn service_incident_severity_name(
        severity: super::super::service_model::ServiceIncidentSeverity,
    ) -> &'static str {
        match severity {
            super::super::service_model::ServiceIncidentSeverity::Info => "info",
            super::super::service_model::ServiceIncidentSeverity::Warning => "warning",
            super::super::service_model::ServiceIncidentSeverity::Error => "error",
            super::super::service_model::ServiceIncidentSeverity::Critical => "critical",
        }
    }
    pub(crate) fn service_incident_escalation_name(
        escalation: super::super::service_model::ServiceIncidentEscalation,
    ) -> &'static str {
        match escalation {
            super::super::service_model::ServiceIncidentEscalation::None => "none",
            super::super::service_model::ServiceIncidentEscalation::BrowserDegraded => {
                "browser_degraded"
            }
            super::super::service_model::ServiceIncidentEscalation::BrowserRecovery => {
                "browser_recovery"
            }
            super::super::service_model::ServiceIncidentEscalation::JobAttention => "job_attention",
            super::super::service_model::ServiceIncidentEscalation::MonitorAttention => {
                "monitor_attention"
            }
            super::super::service_model::ServiceIncidentEscalation::ServiceTriage => {
                "service_triage"
            }
            super::super::service_model::ServiceIncidentEscalation::OsDegradedPossible => {
                "os_degraded_possible"
            }
        }
    }
    pub(crate) fn service_incident_handling_state_name(
        incident: &super::super::service_model::ServiceIncident,
    ) -> &'static str {
        if incident.resolved_at.is_some() {
            "resolved"
        } else if incident.acknowledged_at.is_some() {
            "acknowledged"
        } else {
            "unacknowledged"
        }
    }
    pub(crate) fn parse_service_event_timestamp(
        raw: &str,
    ) -> Result<DateTime<FixedOffset>, String> {
        DateTime::parse_from_rfc3339(raw)
            .map_err(|err| format!("Invalid --since timestamp '{}': {}", raw, err))
    }
    pub(crate) fn service_job_at_or_after(
        job: &super::super::service_model::ServiceJob,
        minimum: DateTime<FixedOffset>,
    ) -> bool {
        job.submitted_at
            .as_deref()
            .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
            .is_some_and(|timestamp| timestamp >= minimum)
    }
    pub(crate) fn service_event_at_or_after(
        event: &ServiceEvent,
        minimum: DateTime<FixedOffset>,
    ) -> bool {
        DateTime::parse_from_rfc3339(&event.timestamp)
            .map(|timestamp| timestamp >= minimum)
            .unwrap_or(false)
    }
    pub(crate) fn service_incident_matches_trace_filters(
        incident: &super::super::service_model::ServiceIncident,
        service_state: &ServiceState,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        service_name: Option<&str>,
        agent_name: Option<&str>,
        task_name: Option<&str>,
    ) -> bool {
        if profile_id.is_none()
            && session_id.is_none()
            && service_name.is_none()
            && agent_name.is_none()
            && task_name.is_none()
        {
            return true;
        }
        incident.event_ids.iter().any(|event_id| {
            service_state
                .events
                .iter()
                .find(|event| &event.id == event_id)
                .is_some_and(|event| {
                    service_event_matches_trace_filters(
                        event,
                        profile_id,
                        session_id,
                        service_name,
                        agent_name,
                        task_name,
                    )
                })
        }) || incident.job_ids.iter().any(|job_id| {
            service_state.jobs.get(job_id).is_some_and(|job| {
                service_job_matches_trace_filters(
                    job,
                    service_state,
                    profile_id,
                    session_id,
                    service_name,
                    agent_name,
                    task_name,
                )
            })
        })
    }
    pub(crate) fn service_event_matches_trace_filters(
        event: &ServiceEvent,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        service_name: Option<&str>,
        agent_name: Option<&str>,
        task_name: Option<&str>,
    ) -> bool {
        profile_id.is_none_or(|expected| event.profile_id.as_deref() == Some(expected))
            && session_id.is_none_or(|expected| event.session_id.as_deref() == Some(expected))
            && service_name.is_none_or(|expected| event.service_name.as_deref() == Some(expected))
            && agent_name.is_none_or(|expected| event.agent_name.as_deref() == Some(expected))
            && task_name.is_none_or(|expected| event.task_name.as_deref() == Some(expected))
    }
    pub(crate) fn service_job_matches_trace_filters(
        job: &super::super::service_model::ServiceJob,
        service_state: &ServiceState,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        service_name: Option<&str>,
        agent_name: Option<&str>,
        task_name: Option<&str>,
    ) -> bool {
        profile_id
            .is_none_or(|expected| service_job_profile_id(job, service_state) == Some(expected))
            && session_id
                .is_none_or(|expected| service_job_session_id(job, service_state) == Some(expected))
            && service_name.is_none_or(|expected| job.service_name.as_deref() == Some(expected))
            && agent_name.is_none_or(|expected| job.agent_name.as_deref() == Some(expected))
            && task_name.is_none_or(|expected| job.task_name.as_deref() == Some(expected))
    }
    pub(crate) fn service_job_profile_id<'a>(
        job: &'a super::super::service_model::ServiceJob,
        service_state: &'a ServiceState,
    ) -> Option<&'a str> {
        match &job.target {
            super::super::service_model::JobTarget::Profile(profile_id) => {
                Some(profile_id.as_str())
            }
            super::super::service_model::JobTarget::Browser(browser_id) => service_state
                .browsers
                .get(browser_id)
                .and_then(|browser| browser.profile_id.as_deref()),
            super::super::service_model::JobTarget::Tab(tab_id) => {
                service_state.tabs.get(tab_id).and_then(|tab| {
                    tab.owner_session_id
                        .as_deref()
                        .and_then(|session_id| service_state.sessions.get(session_id))
                        .and_then(|session| session.profile_id.as_deref())
                        .or_else(|| {
                            service_state
                                .browsers
                                .get(&tab.browser_id)
                                .and_then(|browser| browser.profile_id.as_deref())
                        })
                })
            }
            super::super::service_model::JobTarget::Service
            | super::super::service_model::JobTarget::Monitor(_)
            | super::super::service_model::JobTarget::Challenge(_) => None,
        }
    }
    pub(crate) fn service_job_session_id<'a>(
        job: &'a super::super::service_model::ServiceJob,
        service_state: &'a ServiceState,
    ) -> Option<&'a str> {
        match &job.target {
            super::super::service_model::JobTarget::Browser(browser_id) => service_state
                .browsers
                .get(browser_id)
                .and_then(|browser| browser.active_session_ids.first().map(String::as_str))
                .or_else(|| session_id_for_browser(service_state, browser_id)),
            super::super::service_model::JobTarget::Tab(tab_id) => service_state
                .tabs
                .get(tab_id)
                .and_then(|tab| tab.owner_session_id.as_deref()),
            super::super::service_model::JobTarget::Service
            | super::super::service_model::JobTarget::Profile(_)
            | super::super::service_model::JobTarget::Monitor(_)
            | super::super::service_model::JobTarget::Challenge(_) => None,
        }
    }
    pub(crate) fn session_id_for_browser<'a>(
        service_state: &'a ServiceState,
        browser_id: &str,
    ) -> Option<&'a str> {
        service_state
            .sessions
            .iter()
            .find_map(|(session_id, session)| {
                session
                    .browser_ids
                    .iter()
                    .any(|id| id == browser_id)
                    .then_some(session_id.as_str())
            })
    }
    pub(crate) fn service_incident_at_or_after(
        incident: &super::super::service_model::ServiceIncident,
        minimum: DateTime<FixedOffset>,
    ) -> bool {
        DateTime::parse_from_rfc3339(&incident.latest_timestamp)
            .map(|timestamp| timestamp >= minimum)
            .unwrap_or(false)
    }
    pub(crate) fn service_now_timestamp() -> String {
        chrono::Utc::now().to_rfc3339()
    }
}
pub(crate) use service_commands::*;
