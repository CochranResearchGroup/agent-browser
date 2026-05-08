use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};

use super::service_config::reset_monitor_failures;
use super::service_model::{
    JobTarget, ServiceEvent, ServiceEventKind, ServiceIncident, ServiceIncidentEscalation,
    ServiceIncidentSeverity, ServiceIncidentState, ServiceJob, ServiceState, SiteMonitor,
};
use super::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};

const MAX_SERVICE_EVENTS: usize = 100;

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
    pub remedies_only: bool,
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
                && (!filters.remedies_only
                    || matches!(
                        incident.escalation,
                        ServiceIncidentEscalation::BrowserDegraded
                            | ServiceIncidentEscalation::MonitorAttention
                            | ServiceIncidentEscalation::OsDegradedPossible
                    ))
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
            "remediesOnly": filters.remedies_only,
            "limit": limit,
        },
        "incidents": incidents,
        "count": incidents.len(),
        "matched": matched,
        "total": total,
    }))
}

pub(crate) fn service_incident_summary(incidents: &[Value]) -> Value {
    let mut groups = std::collections::BTreeMap::<(String, String, String), Vec<&Value>>::new();
    for incident in incidents {
        let escalation = incident
            .get("escalation")
            .and_then(|value| value.as_str())
            .unwrap_or("none")
            .to_string();
        let severity = incident
            .get("severity")
            .and_then(|value| value.as_str())
            .unwrap_or("info")
            .to_string();
        let state = incident
            .get("state")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        groups
            .entry((escalation, severity, state))
            .or_default()
            .push(incident);
    }

    let groups = groups
        .into_iter()
        .map(|((escalation, severity, state), incidents)| {
            let recommended_action = incidents
                .iter()
                .filter_map(|incident| {
                    incident
                        .get("recommendedAction")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                })
                .next()
                .unwrap_or("Inspect incident details.");
            let ids = incidents
                .iter()
                .filter_map(|incident| incident.get("id").and_then(|value| value.as_str()))
                .collect::<Vec<_>>();
            let mut monitor_ids = incidents
                .iter()
                .filter_map(|incident| {
                    incident
                        .get("monitorId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                })
                .collect::<Vec<_>>();
            monitor_ids.sort();
            monitor_ids.dedup();
            let monitor_reset_commands = monitor_ids
                .iter()
                .map(|monitor_id| format!("agent-browser service monitors reset {monitor_id}"))
                .collect::<Vec<_>>();
            let newest = incidents
                .iter()
                .filter_map(|incident| {
                    incident
                        .get("latestTimestamp")
                        .and_then(|value| value.as_str())
                })
                .max()
                .unwrap_or("unknown-time");
            json!({
                "escalation": escalation,
                "severity": severity,
                "state": state,
                "count": incidents.len(),
                "latestTimestamp": newest,
                "recommendedAction": recommended_action,
                "incidentIds": ids,
                "monitorIds": monitor_ids,
                "monitorResetCommands": monitor_reset_commands,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "groupCount": groups.len(),
        "groups": groups,
    })
}

pub(crate) fn acknowledge_persisted_service_incident(
    incident_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> Result<ServiceIncident, String> {
    let repository = default_incident_repository()?;
    acknowledge_service_incident_in_repository(&repository, incident_id, timestamp, actor, note)
}

pub(crate) fn acknowledge_service_incident_in_repository(
    repository: &impl ServiceStateRepository,
    incident_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> Result<ServiceIncident, String> {
    let note = note.map(str::to_string);
    mutate_persisted_service_incident_in_repository(
        repository,
        incident_id,
        |state, incident_index| {
            let incident = &mut state.incidents[incident_index];
            if incident.acknowledged_at.is_none() {
                incident.acknowledged_at = Some(timestamp.to_string());
            }
            incident.acknowledged_by = Some(actor.to_string());
            incident.acknowledgement_note = note.clone();
            let incident = incident.clone();
            push_service_event_bounded(
                state,
                service_incident_handling_event(
                    &incident,
                    ServiceEventKind::IncidentAcknowledged,
                    timestamp,
                    actor,
                    note.as_deref(),
                ),
            );
        },
    )
}

pub(crate) fn triage_persisted_service_monitor(
    monitor_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> Result<(SiteMonitor, Option<ServiceIncident>), String> {
    let repository = default_incident_repository()?;
    triage_service_monitor_in_repository(&repository, monitor_id, timestamp, actor, note)
}

pub(crate) fn triage_service_monitor_in_repository(
    repository: &impl ServiceStateRepository,
    monitor_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> Result<(SiteMonitor, Option<ServiceIncident>), String> {
    let note = note.map(str::to_string);
    repository
        .mutate(|state| {
            state.refresh_derived_views();
            let incident_index = state
                .incidents
                .iter()
                .enumerate()
                .filter(|(_, incident)| incident.monitor_id.as_deref() == Some(monitor_id))
                .max_by(|(_, left), (_, right)| left.latest_timestamp.cmp(&right.latest_timestamp))
                .map(|(index, _)| index);

            let incident_id = if let Some(incident_index) = incident_index {
                let incident = &mut state.incidents[incident_index];
                if incident.acknowledged_at.is_none() {
                    incident.acknowledged_at = Some(timestamp.to_string());
                }
                incident.acknowledged_by = Some(actor.to_string());
                incident.acknowledgement_note = note.clone();
                let incident = incident.clone();
                let incident_id = incident.id.clone();
                push_service_event_bounded(
                    state,
                    service_incident_handling_event(
                        &incident,
                        ServiceEventKind::IncidentAcknowledged,
                        timestamp,
                        actor,
                        note.as_deref(),
                    ),
                );
                Some(incident_id)
            } else {
                None
            };

            let monitor = reset_monitor_failures(state, monitor_id)?;
            state.refresh_derived_views();
            let incident = incident_id.and_then(|incident_id| {
                state
                    .incidents
                    .iter()
                    .find(|incident| incident.id == incident_id)
                    .cloned()
            });
            Ok((monitor, incident))
        })
        .map_err(|err| {
            if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
                format!("Unable to load service state: {}", err)
            } else {
                err
            }
        })
}

pub(crate) fn resolve_persisted_service_incident(
    incident_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> Result<ServiceIncident, String> {
    let repository = default_incident_repository()?;
    resolve_service_incident_in_repository(&repository, incident_id, timestamp, actor, note)
}

pub(crate) fn resolve_service_incident_in_repository(
    repository: &impl ServiceStateRepository,
    incident_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> Result<ServiceIncident, String> {
    let note = note.map(str::to_string);
    mutate_persisted_service_incident_in_repository(
        repository,
        incident_id,
        |state, incident_index| {
            let incident = &mut state.incidents[incident_index];
            if incident.acknowledged_at.is_none() {
                incident.acknowledged_at = Some(timestamp.to_string());
            }
            if incident.acknowledged_by.is_none() {
                incident.acknowledged_by = Some(actor.to_string());
            }
            incident.resolved_at = Some(timestamp.to_string());
            incident.resolved_by = Some(actor.to_string());
            incident.resolution_note = note.clone();
            let incident = incident.clone();
            push_service_event_bounded(
                state,
                service_incident_handling_event(
                    &incident,
                    ServiceEventKind::IncidentResolved,
                    timestamp,
                    actor,
                    note.as_deref(),
                ),
            );
        },
    )
}

pub(crate) fn mutate_persisted_service_incident_in_repository(
    repository: &impl ServiceStateRepository,
    incident_id: &str,
    mutator: impl FnOnce(&mut ServiceState, usize),
) -> Result<ServiceIncident, String> {
    repository
        .mutate(|state| {
            state.refresh_derived_views();
            let incident_index = state
                .incidents
                .iter()
                .position(|incident| incident.id == incident_id)
                .ok_or_else(|| format!("Service incident not found: {}", incident_id))?;
            mutator(state, incident_index);
            state.refresh_derived_views();
            state
                .incidents
                .iter()
                .find(|incident| incident.id == incident_id)
                .cloned()
                .ok_or_else(|| format!("Service incident not found: {}", incident_id))
        })
        .map_err(|err| {
            if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
                format!("Unable to load service state: {}", err)
            } else {
                err
            }
        })
}

fn default_incident_repository(
) -> Result<LockedServiceStateRepository<JsonServiceStateStore>, String> {
    LockedServiceStateRepository::default_json().map_err(|err| {
        if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
            format!("Unable to load service state: {}", err)
        } else {
            err
        }
    })
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
        ServiceIncidentEscalation::MonitorAttention => "monitor_attention",
        ServiceIncidentEscalation::ServiceTriage => "service_triage",
        ServiceIncidentEscalation::OsDegradedPossible => "os_degraded_possible",
    }
}

fn service_incident_handling_event(
    incident: &ServiceIncident,
    kind: ServiceEventKind,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
) -> ServiceEvent {
    let action = match kind {
        ServiceEventKind::IncidentResolved => "resolved",
        _ => "acknowledged",
    };
    let mut details = json!({
        "incidentId": incident.id,
        "actor": actor,
        "action": action,
    });
    if let Some(note) = note {
        details["note"] = json!(note);
    }
    ServiceEvent {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        timestamp: timestamp.to_string(),
        kind,
        message: format!("Incident {} {}", incident.label, action),
        browser_id: incident.browser_id.clone(),
        details: Some(details),
        ..ServiceEvent::default()
    }
}

fn push_service_event_bounded(state: &mut ServiceState, event: ServiceEvent) {
    state.events.push(event);
    if state.events.len() > MAX_SERVICE_EVENTS {
        let excess = state.events.len() - MAX_SERVICE_EVENTS;
        state.events.drain(0..excess);
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
