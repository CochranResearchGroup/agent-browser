use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};

use super::service_model::{
    JobTarget, ServiceEvent, ServiceIncident, ServiceIncidentEscalation, ServiceIncidentSeverity,
    ServiceIncidentState, ServiceJob, ServiceState,
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ServiceIncidentFilters<'a> {
    pub limit: usize,
    pub incident_id: Option<&'a str>,
    pub state: Option<&'a str>,
    pub severity: Option<&'a str>,
    pub escalation: Option<&'a str>,
    pub handling_state: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub browser_id: Option<&'a str>,
    pub profile_id: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub service_name: Option<&'a str>,
    pub agent_name: Option<&'a str>,
    pub task_name: Option<&'a str>,
    pub since: Option<&'a str>,
}

/// Build the canonical filtered service incident response for CLI, HTTP, and MCP.
pub(crate) fn service_incidents_response(
    service_state: &ServiceState,
    filters: ServiceIncidentFilters<'_>,
) -> Result<Value, String> {
    let limit = if filters.limit == 0 {
        20
    } else {
        filters.limit
    };
    let since = filters
        .since
        .map(parse_service_incident_timestamp)
        .transpose()?;
    let total = service_state.incidents.len();

    if let Some(incident_id) = filters.incident_id {
        let incident = service_state
            .incidents
            .iter()
            .find(|incident| incident.id == incident_id)
            .cloned()
            .ok_or_else(|| format!("Service incident not found: {}", incident_id))?;
        let events = related_incident_events(service_state, &incident);
        let jobs = related_incident_jobs(service_state, &incident);
        return Ok(json!({
            "incident": incident,
            "incidents": [incident],
            "events": events,
            "jobs": jobs,
            "count": 1,
            "matched": 1,
            "total": total,
        }));
    }

    let mut incidents = service_state
        .incidents
        .iter()
        .filter(|incident| {
            filters
                .state
                .is_none_or(|expected| service_incident_state_name(incident.state) == expected)
                && filters.severity.is_none_or(|expected| {
                    service_incident_severity_name(incident.severity) == expected
                })
                && filters.escalation.is_none_or(|expected| {
                    service_incident_escalation_name(incident.escalation) == expected
                })
                && filters.handling_state.is_none_or(|expected| {
                    service_incident_handling_state_name(incident) == expected
                })
                && filters
                    .kind
                    .is_none_or(|expected| incident.latest_kind == expected)
                && filters
                    .browser_id
                    .is_none_or(|expected| incident.browser_id.as_deref() == Some(expected))
                && service_incident_matches_trace_filters(incident, service_state, &filters)
                && since.is_none_or(|minimum| service_incident_at_or_after(incident, minimum))
        })
        .cloned()
        .collect::<Vec<_>>();
    let matched = incidents.len();
    let start = matched.saturating_sub(limit);
    incidents = incidents[start..].to_vec();

    Ok(json!({
        "filters": {
            "state": filters.state,
            "severity": filters.severity,
            "escalation": filters.escalation,
            "handlingState": filters.handling_state,
            "kind": filters.kind,
            "browserId": filters.browser_id,
            "profileId": filters.profile_id,
            "sessionId": filters.session_id,
            "serviceName": filters.service_name,
            "agentName": filters.agent_name,
            "taskName": filters.task_name,
            "since": filters.since,
            "limit": limit,
        },
        "incidents": incidents,
        "count": incidents.len(),
        "matched": matched,
        "total": total,
    }))
}

pub(crate) fn service_incident_state_name(state: ServiceIncidentState) -> &'static str {
    match state {
        ServiceIncidentState::Active => "active",
        ServiceIncidentState::Recovered => "recovered",
        ServiceIncidentState::Service => "service",
    }
}

pub(crate) fn service_incident_severity_name(severity: ServiceIncidentSeverity) -> &'static str {
    match severity {
        ServiceIncidentSeverity::Info => "info",
        ServiceIncidentSeverity::Warning => "warning",
        ServiceIncidentSeverity::Error => "error",
        ServiceIncidentSeverity::Critical => "critical",
    }
}

pub(crate) fn service_incident_escalation_name(
    escalation: ServiceIncidentEscalation,
) -> &'static str {
    match escalation {
        ServiceIncidentEscalation::None => "none",
        ServiceIncidentEscalation::BrowserDegraded => "browser_degraded",
        ServiceIncidentEscalation::BrowserRecovery => "browser_recovery",
        ServiceIncidentEscalation::JobAttention => "job_attention",
        ServiceIncidentEscalation::ServiceTriage => "service_triage",
        ServiceIncidentEscalation::OsDegradedPossible => "os_degraded_possible",
    }
}

fn parse_service_incident_timestamp(raw: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(raw)
        .map_err(|err| format!("Invalid --since timestamp '{}': {}", raw, err))
}

fn related_incident_events(
    service_state: &ServiceState,
    incident: &ServiceIncident,
) -> Vec<ServiceEvent> {
    incident
        .event_ids
        .iter()
        .filter_map(|event_id| {
            service_state
                .events
                .iter()
                .find(|event| &event.id == event_id)
                .cloned()
        })
        .collect()
}

fn related_incident_jobs(
    service_state: &ServiceState,
    incident: &ServiceIncident,
) -> Vec<ServiceJob> {
    incident
        .job_ids
        .iter()
        .filter_map(|job_id| service_state.jobs.get(job_id).cloned())
        .collect()
}

fn service_incident_handling_state_name(incident: &ServiceIncident) -> &'static str {
    if incident.resolved_at.is_some() {
        "resolved"
    } else if incident.acknowledged_at.is_some() {
        "acknowledged"
    } else {
        "unacknowledged"
    }
}

fn service_incident_matches_trace_filters(
    incident: &ServiceIncident,
    service_state: &ServiceState,
    filters: &ServiceIncidentFilters<'_>,
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

fn service_event_matches_trace_filters(
    event: &ServiceEvent,
    filters: &ServiceIncidentFilters<'_>,
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
    filters: &ServiceIncidentFilters<'_>,
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

fn service_incident_at_or_after(
    incident: &ServiceIncident,
    minimum: DateTime<FixedOffset>,
) -> bool {
    DateTime::parse_from_rfc3339(&incident.latest_timestamp)
        .map(|timestamp| timestamp >= minimum)
        .unwrap_or(false)
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
