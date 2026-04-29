use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};

use super::service_activity::service_incident_activity_items;
use super::service_model::{JobTarget, ServiceEvent, ServiceIncident, ServiceJob, ServiceState};

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
    let summary = service_trace_summary(service_state, &events, &jobs, &incidents, &activity);

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
    latest_timestamp: Option<String>,
}

/// Compact owner/context rollup for dashboards, MCP agents, and API clients.
fn service_trace_summary(
    service_state: &ServiceState,
    events: &[ServiceEvent],
    jobs: &[ServiceJob],
    incidents: &[ServiceIncident],
    activity: &[Value],
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
            json!({
                "serviceName": key.service_name,
                "agentName": key.agent_name,
                "taskName": key.task_name,
                "browserId": key.browser_id,
                "profileId": key.profile_id,
                "sessionId": key.session_id,
                "eventCount": summary.event_count,
                "jobCount": summary.job_count,
                "incidentCount": summary.incident_count,
                "activityCount": summary.activity_count,
                "latestTimestamp": summary.latest_timestamp,
            })
        })
        .collect::<Vec<_>>();

    let context_count = contexts.len();
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
        "contexts": contexts,
    })
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
