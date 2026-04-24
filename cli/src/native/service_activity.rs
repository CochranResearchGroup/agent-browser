use serde_json::{json, Value};

use super::service_model::{
    JobState, ServiceEvent, ServiceEventKind, ServiceIncident, ServiceJob, ServiceState,
};

pub(crate) fn service_incident_activity_response(
    service_state: &ServiceState,
    incident_id: &str,
) -> Result<Value, String> {
    let incident = service_state
        .incidents
        .iter()
        .find(|incident| incident.id == incident_id)
        .cloned()
        .ok_or_else(|| format!("Service incident not found: {}", incident_id))?;
    let activity = service_incident_activity(service_state, &incident);

    Ok(json!({
        "incident": incident,
        "activity": activity,
        "count": activity.len(),
    }))
}

fn service_incident_activity(
    service_state: &ServiceState,
    incident: &ServiceIncident,
) -> Vec<Value> {
    let mut activity = Vec::new();

    for event_id in &incident.event_ids {
        if let Some(event) = service_state
            .events
            .iter()
            .find(|event| &event.id == event_id)
        {
            activity.push(service_event_activity(event));
        }
    }

    for job_id in &incident.job_ids {
        if let Some(job) = service_state.jobs.get(job_id) {
            activity.push(service_job_activity(job));
        }
    }

    if !activity.iter().any(|item| {
        item.get("kind")
            .and_then(|kind| kind.as_str())
            .is_some_and(|kind| kind == "incident_acknowledged")
    }) {
        if let Some(item) = service_incident_metadata_activity(incident, "acknowledged") {
            activity.push(item);
        }
    }

    if !activity.iter().any(|item| {
        item.get("kind")
            .and_then(|kind| kind.as_str())
            .is_some_and(|kind| kind == "incident_resolved")
    }) {
        if let Some(item) = service_incident_metadata_activity(incident, "resolved") {
            activity.push(item);
        }
    }

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
    activity
}

fn service_event_activity(event: &ServiceEvent) -> Value {
    let kind = service_event_kind_name(event.kind);
    json!({
        "id": event.id,
        "source": "event",
        "eventId": event.id,
        "timestamp": event.timestamp,
        "kind": kind,
        "title": service_activity_title(kind),
        "message": event.message,
        "browserId": event.browser_id,
        "details": event.details,
    })
}

fn service_job_activity(job: &ServiceJob) -> Value {
    let kind = match job.state {
        JobState::TimedOut => "service_job_timeout",
        JobState::Cancelled => "service_job_cancelled",
        _ => "service_job",
    };
    let timestamp = job
        .completed_at
        .as_deref()
        .or(job.started_at.as_deref())
        .or(job.submitted_at.as_deref())
        .unwrap_or("unknown-time");
    let message = job.error.as_deref().unwrap_or("Service job changed state");
    json!({
        "id": job.id,
        "source": "job",
        "jobId": job.id,
        "timestamp": timestamp,
        "kind": kind,
        "title": service_activity_title(kind),
        "message": message,
        "jobState": service_job_state_name(job.state),
        "jobAction": job.action,
        "target": job.target,
    })
}

fn service_incident_metadata_activity(incident: &ServiceIncident, action: &str) -> Option<Value> {
    let (timestamp, actor, note, kind, title) = match action {
        "acknowledged" => (
            incident.acknowledged_at.as_deref()?,
            incident.acknowledged_by.as_deref().unwrap_or("operator"),
            incident.acknowledgement_note.as_deref(),
            "incident_acknowledged",
            "Incident acknowledged",
        ),
        "resolved" => (
            incident.resolved_at.as_deref()?,
            incident.resolved_by.as_deref().unwrap_or("operator"),
            incident.resolution_note.as_deref(),
            "incident_resolved",
            "Incident resolved",
        ),
        _ => return None,
    };
    let mut details = json!({
        "incidentId": incident.id,
        "actor": actor,
        "action": action,
    });
    if let Some(note) = note {
        details["note"] = json!(note);
    }
    Some(json!({
        "id": format!("{}-{}", incident.id, action),
        "source": "metadata",
        "timestamp": timestamp,
        "kind": kind,
        "title": title,
        "message": format!("Incident {} {}", incident.label, action),
        "browserId": incident.browser_id,
        "details": details,
    }))
}

fn service_activity_title(kind: &str) -> &'static str {
    match kind {
        "browser_health_changed" => "Browser health changed",
        "tab_lifecycle_changed" => "Tab lifecycle changed",
        "reconciliation_error" => "Reconciliation error",
        "incident_acknowledged" => "Incident acknowledged",
        "incident_resolved" => "Incident resolved",
        "service_job_timeout" => "Service job timed out",
        "service_job_cancelled" => "Service job cancelled",
        "service_job" => "Service job updated",
        _ => "Service activity",
    }
}

fn service_event_kind_name(kind: ServiceEventKind) -> &'static str {
    match kind {
        ServiceEventKind::Reconciliation => "reconciliation",
        ServiceEventKind::BrowserHealthChanged => "browser_health_changed",
        ServiceEventKind::TabLifecycleChanged => "tab_lifecycle_changed",
        ServiceEventKind::ReconciliationError => "reconciliation_error",
        ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
        ServiceEventKind::IncidentResolved => "incident_resolved",
    }
}

fn service_job_state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::TimedOut => "timed_out",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::native::service_model::{JobTarget, ServiceIncidentState};

    #[test]
    fn service_incident_activity_response_combines_events_jobs_and_metadata() {
        let state = ServiceState {
            events: vec![ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser failed".to_string(),
                browser_id: Some("browser-1".to_string()),
                ..ServiceEvent::default()
            }],
            jobs: BTreeMap::from([(
                "job-1".to_string(),
                ServiceJob {
                    id: "job-1".to_string(),
                    action: "navigate".to_string(),
                    target: JobTarget::Browser("browser-1".to_string()),
                    state: JobState::TimedOut,
                    submitted_at: Some("2026-04-22T00:01:00Z".to_string()),
                    error: Some("Timed out".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            incidents: vec![ServiceIncident {
                id: "browser-1".to_string(),
                browser_id: Some("browser-1".to_string()),
                label: "browser-1".to_string(),
                state: ServiceIncidentState::Active,
                acknowledged_at: Some("2026-04-22T00:02:00Z".to_string()),
                acknowledged_by: Some("operator".to_string()),
                latest_timestamp: "2026-04-22T00:02:00Z".to_string(),
                latest_kind: "service_job_timeout".to_string(),
                event_ids: vec!["event-1".to_string()],
                job_ids: vec!["job-1".to_string()],
                ..ServiceIncident::default()
            }],
            ..ServiceState::default()
        };

        let response = service_incident_activity_response(&state, "browser-1").unwrap();

        assert_eq!(response["count"], 3);
        assert_eq!(response["activity"][0]["source"], "event");
        assert_eq!(response["activity"][1]["source"], "job");
        assert_eq!(response["activity"][2]["source"], "metadata");
    }
}
