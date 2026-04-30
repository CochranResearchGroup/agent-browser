//! Durable service-mode contracts.
//!
//! These types describe the browser service state model before the service API
//! and MCP surfaces are wired to runtime behavior. Keep them serializable and
//! conservative so future clients can depend on stable field names.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME: &str = "missing_service_name";
pub const SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME: &str = "missing_agent_name";
pub const SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME: &str = "missing_task_name";
pub const SERVICE_JOB_NAMING_WARNING_VALUES: [&str; 3] = [
    SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
];
pub const SERVICE_INCIDENT_STATE_VALUES: [&str; 3] = ["active", "recovered", "service"];
pub const SERVICE_INCIDENT_SEVERITY_VALUES: [&str; 4] = ["info", "warning", "error", "critical"];
pub const SERVICE_INCIDENT_ESCALATION_VALUES: [&str; 6] = [
    "none",
    "browser_degraded",
    "browser_recovery",
    "job_attention",
    "service_triage",
    "os_degraded_possible",
];
pub const SERVICE_BROWSER_HEALTH_VALUES: [&str; 10] = [
    "not_started",
    "launching",
    "ready",
    "degraded",
    "unreachable",
    "process_exited",
    "cdp_disconnected",
    "reconnecting",
    "closing",
    "faulted",
];
pub const SERVICE_BROWSER_HOST_VALUES: [&str; 6] = [
    "local_headless",
    "local_headed",
    "docker_headed",
    "remote_headed",
    "cloud_provider",
    "attached_existing",
];
pub const SERVICE_PROFILE_ALLOCATION_VALUES: [&str; 5] = [
    "shared_service",
    "per_service",
    "per_site",
    "per_identity",
    "caller_supplied",
];
pub const SERVICE_PROFILE_KEYRING_VALUES: [&str; 4] = [
    "basic_password_store",
    "real_os_keychain",
    "managed_vault",
    "manual_login_profile",
];
pub const SERVICE_LEASE_STATE_VALUES: [&str; 5] = [
    "shared",
    "exclusive",
    "human_takeover",
    "released",
    "expired",
];
pub const SERVICE_SESSION_CLEANUP_VALUES: [&str; 4] =
    ["detach", "close_tabs", "close_browser", "release_only"];
pub const SERVICE_TAB_LIFECYCLE_VALUES: [&str; 7] = [
    "unknown", "opening", "loading", "ready", "closing", "closed", "crashed",
];
pub const SERVICE_VIEW_STREAM_PROVIDER_VALUES: [&str; 5] = [
    "cdp_screencast",
    "chrome_tab_webrtc",
    "virtual_display_webrtc",
    "novnc",
    "external_url",
];
pub const SERVICE_CONTROL_INPUT_PROVIDER_VALUES: [&str; 4] = [
    "cdp_input",
    "webrtc_input",
    "vnc_input",
    "manual_attached_desktop",
];
pub const SERVICE_INTERACTION_MODE_VALUES: [&str; 5] = [
    "cdp_direct",
    "dom_action",
    "browser_input",
    "human_like_input",
    "manual",
];
pub const SERVICE_CHALLENGE_POLICY_VALUES: [&str; 5] = [
    "avoid_first",
    "manual_only",
    "provider_allowed",
    "provider_preferred",
    "deny",
];
pub const SERVICE_PROVIDER_KIND_VALUES: [&str; 8] = [
    "browser_credentials",
    "password_manager",
    "totp",
    "sms",
    "email",
    "manual_approval",
    "intelligence",
    "captcha",
];
pub const SERVICE_PROVIDER_CAPABILITY_VALUES: [&str; 8] = [
    "password_fill",
    "passkey",
    "totp_code",
    "sms_code",
    "email_code",
    "visual_reasoning",
    "captcha_solve",
    "human_approval",
];
pub const SERVICE_CHALLENGE_KIND_VALUES: [&str; 6] = [
    "unknown",
    "captcha",
    "two_factor",
    "passkey",
    "suspicious_login",
    "blocked_flow",
];
pub const SERVICE_CHALLENGE_STATE_VALUES: [&str; 6] = [
    "detected",
    "waiting_for_provider",
    "waiting_for_human",
    "resolved",
    "failed",
    "denied",
];
pub const SERVICE_EVENT_KIND_VALUES: [&str; 9] = [
    "reconciliation",
    "browser_launch_recorded",
    "browser_health_changed",
    "browser_recovery_started",
    "browser_recovery_override",
    "tab_lifecycle_changed",
    "reconciliation_error",
    "incident_acknowledged",
    "incident_resolved",
];
pub const SERVICE_TRACE_ACTIVITY_SOURCE_VALUES: [&str; 3] = ["event", "job", "metadata"];
pub const SERVICE_TRACE_ACTIVITY_KIND_VALUES: [&str; 12] = [
    "reconciliation",
    "browser_launch_recorded",
    "browser_health_changed",
    "browser_recovery_started",
    "browser_recovery_override",
    "tab_lifecycle_changed",
    "reconciliation_error",
    "incident_acknowledged",
    "incident_resolved",
    "service_job_timeout",
    "service_job_cancelled",
    "service_job",
];
pub const SERVICE_JOB_STATE_VALUES: [&str; 6] = [
    "queued",
    "running",
    "succeeded",
    "failed",
    "cancelled",
    "timed_out",
];
pub const SERVICE_JOB_PRIORITY_VALUES: [&str; 3] = ["low", "normal", "lifecycle"];

#[cfg(test)]
fn assert_record_fields(
    record_name: &str,
    value: &serde_json::Value,
    required_fields: &[&str],
    snake_case_fields: &[&str],
) {
    for field in required_fields {
        assert!(
            value.get(field).is_some(),
            "missing {record_name} field {field}"
        );
    }
    for snake_case_field in snake_case_fields {
        assert!(
            value.get(snake_case_field).is_none(),
            "unexpected snake_case {record_name} field {snake_case_field}"
        );
    }
}

#[cfg(test)]
pub fn service_job_naming_warning_values() -> Vec<String> {
    SERVICE_JOB_NAMING_WARNING_VALUES
        .iter()
        .map(|value| value.to_string())
        .collect()
}

#[cfg(test)]
pub fn assert_service_job_naming_warning_contract(value: &serde_json::Value) {
    assert_eq!(
        value["namingWarnings"],
        serde_json::json!(SERVICE_JOB_NAMING_WARNING_VALUES.to_vec())
    );
    assert_eq!(value["hasNamingWarning"], true);
    assert!(value.get("naming_warnings").is_none());
    assert!(value.get("has_naming_warning").is_none());
}

#[cfg(test)]
pub fn assert_service_incident_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "incident",
        value,
        &[
            "id",
            "browserId",
            "label",
            "state",
            "severity",
            "escalation",
            "recommendedAction",
            "acknowledgedAt",
            "acknowledgedBy",
            "acknowledgementNote",
            "resolvedAt",
            "resolvedBy",
            "resolutionNote",
            "latestTimestamp",
            "latestMessage",
            "latestKind",
            "currentHealth",
            "eventIds",
            "jobIds",
        ],
        &[
            "browser_id",
            "recommended_action",
            "acknowledged_at",
            "acknowledged_by",
            "acknowledgement_note",
            "resolved_at",
            "resolved_by",
            "resolution_note",
            "latest_timestamp",
            "latest_message",
            "latest_kind",
            "current_health",
            "event_ids",
            "job_ids",
        ],
    );
    assert!(SERVICE_INCIDENT_STATE_VALUES.contains(&value["state"].as_str().unwrap()));
    assert!(SERVICE_INCIDENT_SEVERITY_VALUES.contains(&value["severity"].as_str().unwrap()));
    assert!(SERVICE_INCIDENT_ESCALATION_VALUES.contains(&value["escalation"].as_str().unwrap()));
    if let Some(current_health) = value["currentHealth"].as_str() {
        assert!(SERVICE_BROWSER_HEALTH_VALUES.contains(&current_health));
    }
    assert!(value["eventIds"].is_array());
    assert!(value["jobIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_event_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "event",
        value,
        &[
            "id",
            "timestamp",
            "kind",
            "message",
            "browserId",
            "profileId",
            "sessionId",
            "serviceName",
            "agentName",
            "taskName",
            "previousHealth",
            "currentHealth",
            "details",
        ],
        &[
            "browser_id",
            "profile_id",
            "session_id",
            "service_name",
            "agent_name",
            "task_name",
            "previous_health",
            "current_health",
        ],
    );
    assert!(SERVICE_EVENT_KIND_VALUES.contains(&value["kind"].as_str().unwrap()));
    if let Some(previous_health) = value["previousHealth"].as_str() {
        assert!(SERVICE_BROWSER_HEALTH_VALUES.contains(&previous_health));
    }
    if let Some(current_health) = value["currentHealth"].as_str() {
        assert!(SERVICE_BROWSER_HEALTH_VALUES.contains(&current_health));
    }
}

#[cfg(test)]
pub fn assert_service_profile_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "profile",
        value,
        &[
            "id",
            "name",
            "userDataDir",
            "sitePolicyIds",
            "defaultBrowserHost",
            "allocation",
            "keyring",
            "sharedServiceIds",
            "credentialProviderIds",
            "manualLoginPreferred",
            "persistent",
            "tags",
        ],
        &[
            "user_data_dir",
            "site_policy_ids",
            "default_browser_host",
            "shared_service_ids",
            "credential_provider_ids",
            "manual_login_preferred",
        ],
    );
    if let Some(host) = value["defaultBrowserHost"].as_str() {
        assert!(SERVICE_BROWSER_HOST_VALUES.contains(&host));
    }
    assert!(SERVICE_PROFILE_ALLOCATION_VALUES.contains(&value["allocation"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_KEYRING_VALUES.contains(&value["keyring"].as_str().unwrap()));
    assert!(value["sitePolicyIds"].is_array());
    assert!(value["sharedServiceIds"].is_array());
    assert!(value["credentialProviderIds"].is_array());
    assert!(value["tags"].is_array());
}

#[cfg(test)]
pub fn assert_service_browser_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "browser",
        value,
        &[
            "id",
            "profileId",
            "host",
            "health",
            "pid",
            "cdpEndpoint",
            "viewStreams",
            "activeSessionIds",
            "lastError",
            "lastHealthObservation",
        ],
        &[
            "profile_id",
            "cdp_endpoint",
            "view_streams",
            "active_session_ids",
            "last_error",
            "last_health_observation",
        ],
    );
    assert!(SERVICE_BROWSER_HOST_VALUES.contains(&value["host"].as_str().unwrap()));
    assert!(SERVICE_BROWSER_HEALTH_VALUES.contains(&value["health"].as_str().unwrap()));
    assert!(value["viewStreams"].is_array());
    assert!(value["activeSessionIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_session_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "session",
        value,
        &[
            "id",
            "serviceName",
            "agentName",
            "taskName",
            "owner",
            "lease",
            "profileId",
            "cleanup",
            "browserIds",
            "tabIds",
            "createdAt",
            "expiresAt",
        ],
        &[
            "service_name",
            "agent_name",
            "task_name",
            "profile_id",
            "browser_ids",
            "tab_ids",
            "created_at",
            "expires_at",
        ],
    );
    assert!(SERVICE_LEASE_STATE_VALUES.contains(&value["lease"].as_str().unwrap()));
    assert!(SERVICE_SESSION_CLEANUP_VALUES.contains(&value["cleanup"].as_str().unwrap()));
    assert!(value["browserIds"].is_array());
    assert!(value["tabIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_tab_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "tab",
        value,
        &[
            "id",
            "browserId",
            "targetId",
            "sessionId",
            "lifecycle",
            "url",
            "title",
            "ownerSessionId",
            "latestSnapshotId",
            "latestScreenshotId",
            "challengeId",
        ],
        &[
            "browser_id",
            "target_id",
            "session_id",
            "owner_session_id",
            "latest_snapshot_id",
            "latest_screenshot_id",
            "challenge_id",
        ],
    );
    assert!(SERVICE_TAB_LIFECYCLE_VALUES.contains(&value["lifecycle"].as_str().unwrap()));
}

#[cfg(test)]
pub fn assert_service_site_policy_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "site policy",
        value,
        &[
            "id",
            "originPattern",
            "browserHost",
            "viewStream",
            "controlInput",
            "interactionMode",
            "rateLimit",
            "manualLoginPreferred",
            "profileRequired",
            "authProviders",
            "challengePolicy",
            "allowedChallengeProviders",
            "notes",
        ],
        &[
            "origin_pattern",
            "browser_host",
            "view_stream",
            "control_input",
            "interaction_mode",
            "rate_limit",
            "manual_login_preferred",
            "profile_required",
            "auth_providers",
            "challenge_policy",
            "allowed_challenge_providers",
        ],
    );
    if let Some(host) = value["browserHost"].as_str() {
        assert!(SERVICE_BROWSER_HOST_VALUES.contains(&host));
    }
    if let Some(provider) = value["viewStream"].as_str() {
        assert!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.contains(&provider));
    }
    if let Some(provider) = value["controlInput"].as_str() {
        assert!(SERVICE_CONTROL_INPUT_PROVIDER_VALUES.contains(&provider));
    }
    assert!(SERVICE_INTERACTION_MODE_VALUES.contains(&value["interactionMode"].as_str().unwrap()));
    assert!(SERVICE_CHALLENGE_POLICY_VALUES.contains(&value["challengePolicy"].as_str().unwrap()));
    assert!(value["rateLimit"].is_object());
    assert!(value["authProviders"].is_array());
    assert!(value["allowedChallengeProviders"].is_array());
}

#[cfg(test)]
pub fn assert_service_provider_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "provider",
        value,
        &[
            "id",
            "kind",
            "displayName",
            "enabled",
            "configRef",
            "capabilities",
        ],
        &["display_name", "config_ref"],
    );
    assert!(SERVICE_PROVIDER_KIND_VALUES.contains(&value["kind"].as_str().unwrap()));
    for capability in value["capabilities"].as_array().unwrap() {
        assert!(SERVICE_PROVIDER_CAPABILITY_VALUES.contains(&capability.as_str().unwrap()));
    }
}

#[cfg(test)]
pub fn assert_service_challenge_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "challenge",
        value,
        &[
            "id",
            "tabId",
            "kind",
            "state",
            "detectedAt",
            "providerId",
            "policyDecision",
            "humanApproved",
            "result",
        ],
        &[
            "tab_id",
            "detected_at",
            "provider_id",
            "policy_decision",
            "human_approved",
        ],
    );
    assert!(SERVICE_CHALLENGE_KIND_VALUES.contains(&value["kind"].as_str().unwrap()));
    assert!(SERVICE_CHALLENGE_STATE_VALUES.contains(&value["state"].as_str().unwrap()));
}

#[cfg(test)]
pub fn assert_service_trace_summary_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "trace summary",
        value,
        &[
            "contextCount",
            "hasTraceContext",
            "namingWarningCount",
            "contexts",
        ],
        &["context_count", "has_trace_context", "naming_warning_count"],
    );
    let contexts = value["contexts"].as_array().unwrap();
    assert_eq!(
        value["contextCount"].as_u64().unwrap(),
        contexts.len() as u64
    );
    assert!(value["hasTraceContext"].is_boolean());
    assert!(value["namingWarningCount"].is_u64());
    for context in contexts {
        assert_record_fields(
            "trace summary context",
            context,
            &[
                "serviceName",
                "agentName",
                "taskName",
                "browserId",
                "profileId",
                "sessionId",
                "namingWarnings",
                "hasNamingWarning",
                "eventCount",
                "jobCount",
                "incidentCount",
                "activityCount",
                "latestTimestamp",
            ],
            &[
                "service_name",
                "agent_name",
                "task_name",
                "browser_id",
                "profile_id",
                "session_id",
                "naming_warnings",
                "has_naming_warning",
                "event_count",
                "job_count",
                "incident_count",
                "activity_count",
                "latest_timestamp",
            ],
        );
        for warning in context["namingWarnings"].as_array().unwrap() {
            assert!(
                SERVICE_JOB_NAMING_WARNING_VALUES.contains(&warning.as_str().unwrap()),
                "unexpected trace context naming warning {warning:?}"
            );
        }
        assert!(context["hasNamingWarning"].is_boolean());
        assert!(context["eventCount"].is_u64());
        assert!(context["jobCount"].is_u64());
        assert!(context["incidentCount"].is_u64());
        assert!(context["activityCount"].is_u64());
    }
}

#[cfg(test)]
pub fn assert_service_trace_activity_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "trace activity",
        value,
        &[
            "id",
            "source",
            "timestamp",
            "kind",
            "title",
            "message",
            "browserId",
        ],
        &[
            "event_id",
            "job_id",
            "browser_id",
            "profile_id",
            "session_id",
            "service_name",
            "agent_name",
            "task_name",
            "job_state",
            "job_action",
        ],
    );
    assert!(SERVICE_TRACE_ACTIVITY_SOURCE_VALUES.contains(&value["source"].as_str().unwrap()));
    assert!(SERVICE_TRACE_ACTIVITY_KIND_VALUES.contains(&value["kind"].as_str().unwrap()));
    if let Some(job_state) = value.get("jobState").and_then(|state| state.as_str()) {
        assert!(SERVICE_JOB_STATE_VALUES.contains(&job_state));
    }
}

#[cfg(test)]
pub fn assert_service_trace_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "trace response",
        value,
        &[
            "filters",
            "events",
            "jobs",
            "incidents",
            "activity",
            "summary",
            "counts",
            "matched",
            "total",
        ],
        &[],
    );
    assert_record_fields(
        "trace filters",
        &value["filters"],
        &[
            "browserId",
            "profileId",
            "sessionId",
            "serviceName",
            "agentName",
            "taskName",
            "since",
            "limit",
        ],
        &[
            "browser_id",
            "profile_id",
            "session_id",
            "service_name",
            "agent_name",
            "task_name",
        ],
    );
    for field in ["events", "jobs", "incidents", "activity"] {
        assert!(value[field].is_array(), "trace {field} is not an array");
        assert!(
            value["counts"][field].is_u64(),
            "trace counts.{field} is not an integer"
        );
        assert_eq!(
            value["counts"][field].as_u64().unwrap(),
            value[field].as_array().unwrap().len() as u64,
            "trace counts.{field} does not match returned array length"
        );
        assert!(
            value["matched"][field].is_u64(),
            "trace matched.{field} is not an integer"
        );
    }
    for field in ["events", "jobs", "incidents"] {
        assert!(
            value["total"][field].is_u64(),
            "trace total.{field} is not an integer"
        );
    }
    assert!(value["filters"]["limit"].is_u64());
    assert_service_trace_summary_record_contract(&value["summary"]);
}

#[cfg(test)]
pub fn assert_service_incidents_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "incidents response",
        value,
        &["incidents", "count", "matched", "total"],
        &[],
    );
    let incidents = value["incidents"].as_array().unwrap();
    assert_eq!(
        value["count"].as_u64().unwrap(),
        incidents.len() as u64,
        "incidents response count does not match incidents length"
    );
    assert!(value["matched"].is_u64());
    assert!(value["total"].is_u64());
    for incident in incidents {
        assert_service_incident_record_contract(incident);
    }
    if let Some(filters) = value.get("filters") {
        assert_record_fields(
            "incidents filters",
            filters,
            &[
                "state",
                "severity",
                "escalation",
                "handlingState",
                "kind",
                "browserId",
                "profileId",
                "sessionId",
                "serviceName",
                "agentName",
                "taskName",
                "since",
                "limit",
            ],
            &[
                "handling_state",
                "browser_id",
                "profile_id",
                "session_id",
                "service_name",
                "agent_name",
                "task_name",
            ],
        );
        assert!(filters["limit"].is_u64());
    }
    if let Some(incident) = value.get("incident") {
        assert_service_incident_record_contract(incident);
    }
    if let Some(events) = value.get("events").and_then(|events| events.as_array()) {
        for event in events {
            assert_service_event_record_contract(event);
        }
    }
    assert!(
        value
            .get("jobs")
            .is_none_or(|jobs| jobs.as_array().is_some()),
        "incidents response jobs is not an array"
    );
}

#[cfg(test)]
pub fn assert_service_events_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "events response",
        value,
        &["events", "count", "matched", "total"],
        &[],
    );
    let events = value["events"].as_array().unwrap();
    assert_eq!(
        value["count"].as_u64().unwrap(),
        events.len() as u64,
        "events response count does not match events length"
    );
    assert!(value["matched"].is_u64());
    assert!(value["total"].is_u64());
    for event in events {
        assert_service_event_record_contract(event);
    }
}

#[cfg(test)]
pub fn assert_service_jobs_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "jobs response",
        value,
        &["jobs", "count", "matched", "total"],
        &[],
    );
    let jobs = value["jobs"].as_array().unwrap();
    assert_eq!(
        value["count"].as_u64().unwrap(),
        jobs.len() as u64,
        "jobs response count does not match jobs length"
    );
    assert!(value["matched"].is_u64());
    assert!(value["total"].is_u64());
    for job in jobs {
        assert_record_fields(
            "job",
            job,
            &[
                "id",
                "action",
                "serviceName",
                "agentName",
                "taskName",
                "namingWarnings",
                "hasNamingWarning",
                "target",
                "owner",
                "state",
                "priority",
                "submittedAt",
                "startedAt",
                "completedAt",
                "timeoutMs",
                "result",
                "error",
            ],
            &[
                "service_name",
                "agent_name",
                "task_name",
                "naming_warnings",
                "has_naming_warning",
                "submitted_at",
                "started_at",
                "completed_at",
                "timeout_ms",
            ],
        );
        assert!(SERVICE_JOB_STATE_VALUES.contains(&job["state"].as_str().unwrap()));
        assert!(SERVICE_JOB_PRIORITY_VALUES.contains(&job["priority"].as_str().unwrap()));
        assert!(job["namingWarnings"].is_array());
        assert!(job["hasNamingWarning"].is_boolean());
    }
    if let Some(job) = value.get("job") {
        assert!(
            jobs.iter().any(|item| item["id"] == job["id"]),
            "jobs response detail job is not present in jobs array"
        );
    }
}

#[cfg(test)]
pub fn assert_service_site_policy_upsert_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "site policy upsert response",
        value,
        &["id", "sitePolicy", "upserted"],
        &["site_policy"],
    );
    assert!(value["id"].is_string());
    assert_eq!(value["upserted"], true);
    assert_service_site_policy_record_contract(&value["sitePolicy"]);
}

#[cfg(test)]
pub fn assert_service_site_policy_delete_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "site policy delete response",
        value,
        &["id", "deleted", "sitePolicy"],
        &["site_policy"],
    );
    assert!(value["id"].is_string());
    assert!(value["deleted"].is_boolean());
    if value["sitePolicy"].is_object() {
        assert_service_site_policy_record_contract(&value["sitePolicy"]);
    } else {
        assert!(value["sitePolicy"].is_null());
    }
}

#[cfg(test)]
pub fn assert_service_provider_upsert_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "provider upsert response",
        value,
        &["id", "provider", "upserted"],
        &[],
    );
    assert!(value["id"].is_string());
    assert_eq!(value["upserted"], true);
    assert_service_provider_record_contract(&value["provider"]);
}

#[cfg(test)]
pub fn assert_service_provider_delete_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "provider delete response",
        value,
        &["id", "deleted", "provider"],
        &[],
    );
    assert!(value["id"].is_string());
    assert!(value["deleted"].is_boolean());
    if value["provider"].is_object() {
        assert_service_provider_record_contract(&value["provider"]);
    } else {
        assert!(value["provider"].is_null());
    }
}

#[cfg(test)]
pub fn assert_service_incident_activity_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "incident activity response",
        value,
        &["incident", "activity", "count"],
        &[],
    );
    assert_service_incident_record_contract(&value["incident"]);
    let activity = value["activity"].as_array().unwrap();
    assert_eq!(value["count"].as_u64().unwrap(), activity.len() as u64);
    for item in activity {
        assert_service_trace_activity_record_contract(item);
    }
}

/// Top-level snapshot of the browser service control plane.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceState {
    pub control_plane: Option<ControlPlaneSnapshot>,
    pub reconciliation: Option<ServiceReconciliationSnapshot>,
    pub events: Vec<ServiceEvent>,
    pub incidents: Vec<ServiceIncident>,
    pub profiles: BTreeMap<String, BrowserProfile>,
    pub browsers: BTreeMap<String, BrowserProcess>,
    pub sessions: BTreeMap<String, BrowserSession>,
    pub tabs: BTreeMap<String, BrowserTab>,
    pub jobs: BTreeMap<String, ServiceJob>,
    pub monitors: BTreeMap<String, SiteMonitor>,
    pub site_policies: BTreeMap<String, SitePolicy>,
    pub providers: BTreeMap<String, ServiceProvider>,
    pub challenges: BTreeMap<String, Challenge>,
}

impl ServiceState {
    pub fn overlay_configured_entities(&mut self, configured: ServiceState) {
        self.profiles.extend(configured.profiles);
        self.sessions.extend(configured.sessions);
        self.site_policies.extend(configured.site_policies);
        self.providers.extend(configured.providers);
    }

    /// Refresh bounded derived collections before persistence or API exposure.
    pub fn refresh_derived_views(&mut self) {
        let preserved_metadata = self
            .incidents
            .iter()
            .map(|incident| {
                (
                    incident.id.clone(),
                    (
                        incident.acknowledged_at.clone(),
                        incident.acknowledged_by.clone(),
                        incident.acknowledgement_note.clone(),
                        incident.resolved_at.clone(),
                        incident.resolved_by.clone(),
                        incident.resolution_note.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.incidents = derive_service_incidents(self)
            .into_iter()
            .map(|mut incident| {
                if let Some((
                    acknowledged_at,
                    acknowledged_by,
                    acknowledgement_note,
                    resolved_at,
                    resolved_by,
                    resolution_note,
                )) = preserved_metadata.get(&incident.id)
                {
                    incident.acknowledged_at = acknowledged_at.clone();
                    incident.acknowledged_by = acknowledged_by.clone();
                    incident.acknowledgement_note = acknowledgement_note.clone();
                    incident.resolved_at = resolved_at.clone();
                    incident.resolved_by = resolved_by.clone();
                    incident.resolution_note = resolution_note.clone();
                }
                incident
            })
            .collect();
    }
}

/// Bounded service event log entry for operator auditability.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceEvent {
    pub id: String,
    pub timestamp: String,
    pub kind: ServiceEventKind,
    pub message: String,
    pub browser_id: Option<String>,
    pub profile_id: Option<String>,
    pub session_id: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub previous_health: Option<BrowserHealth>,
    pub current_health: Option<BrowserHealth>,
    pub details: Option<serde_json::Value>,
}

/// Grouped service incident derived from event and job history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceIncident {
    pub id: String,
    pub browser_id: Option<String>,
    pub label: String,
    pub state: ServiceIncidentState,
    pub severity: ServiceIncidentSeverity,
    pub escalation: ServiceIncidentEscalation,
    pub recommended_action: String,
    pub acknowledged_at: Option<String>,
    pub acknowledged_by: Option<String>,
    pub acknowledgement_note: Option<String>,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub resolution_note: Option<String>,
    pub latest_timestamp: String,
    pub latest_message: String,
    pub latest_kind: String,
    pub current_health: Option<BrowserHealth>,
    pub event_ids: Vec<String>,
    pub job_ids: Vec<String>,
}

/// Operator-facing summary state for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentState {
    #[default]
    Active,
    Recovered,
    Service,
}

/// Operator-facing severity for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentSeverity {
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

/// Operator escalation bucket for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentEscalation {
    #[default]
    None,
    BrowserDegraded,
    BrowserRecovery,
    JobAttention,
    ServiceTriage,
    OsDegradedPossible,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEventKind {
    #[default]
    Reconciliation,
    BrowserLaunchRecorded,
    BrowserHealthChanged,
    BrowserRecoveryStarted,
    BrowserRecoveryOverride,
    TabLifecycleChanged,
    ReconciliationError,
    IncidentAcknowledged,
    IncidentResolved,
}

fn derive_service_incidents(state: &ServiceState) -> Vec<ServiceIncident> {
    let mut grouped = BTreeMap::<String, ServiceIncident>::new();

    for event in state
        .events
        .iter()
        .filter(|event| service_event_is_incident(event))
    {
        let browser_id = event.browser_id.clone();
        let key = service_event_incident_id(event)
            .or(browser_id.clone())
            .unwrap_or_else(|| "service".to_string());
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                label: browser_id
                    .clone()
                    .unwrap_or_else(|| "Service incidents".to_string()),
                state: classify_incident_state(
                    browser_id.is_some(),
                    event.current_health,
                    event.kind,
                ),
                latest_timestamp: event.timestamp.clone(),
                latest_message: event.message.clone(),
                latest_kind: service_event_kind_name(event.kind).to_string(),
                current_health: event.current_health,
                ..ServiceIncident::default()
            });

        if !service_event_is_handling(event.kind)
            && incident_is_newer(&event.timestamp, &incident.latest_timestamp)
        {
            incident.latest_timestamp = event.timestamp.clone();
            incident.latest_message = event.message.clone();
            incident.latest_kind = service_event_kind_name(event.kind).to_string();
            incident.current_health = event.current_health.or(incident.current_health);
            incident.state = classify_incident_state(
                incident.browser_id.is_some(),
                incident.current_health,
                event.kind,
            );
        }

        incident.event_ids.push(event.id.clone());
    }

    for job in state
        .jobs
        .values()
        .filter(|job| service_job_is_incident(job))
    {
        let browser_id = service_job_browser_id(job, state);
        let key = browser_id.clone().unwrap_or_else(|| "service".to_string());
        let timestamp = service_job_incident_timestamp(job);
        let message = service_job_incident_message(job);
        let kind = service_job_incident_kind(job);
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                label: browser_id
                    .clone()
                    .unwrap_or_else(|| "Service incidents".to_string()),
                state: classify_job_incident_state(browser_id.is_some()),
                latest_timestamp: timestamp.to_string(),
                latest_message: message.clone(),
                latest_kind: kind.to_string(),
                current_health: state.browsers.get(&key).map(|browser| browser.health),
                ..ServiceIncident::default()
            });

        if incident_is_newer(&timestamp, &incident.latest_timestamp) {
            incident.latest_timestamp = timestamp.to_string();
            incident.latest_message = message;
            incident.latest_kind = kind.to_string();
            if let Some(browser_id) = incident.browser_id.as_ref() {
                incident.current_health =
                    state.browsers.get(browser_id).map(|browser| browser.health);
            }
            incident.state = classify_job_incident_state(incident.browser_id.is_some());
        }

        incident.job_ids.push(job.id.clone());
    }

    let event_timestamps: BTreeMap<&str, &str> = state
        .events
        .iter()
        .map(|event| (event.id.as_str(), event.timestamp.as_str()))
        .collect();
    let job_timestamps: BTreeMap<&str, &str> = state
        .jobs
        .values()
        .map(|job| (job.id.as_str(), service_job_incident_timestamp(job)))
        .collect();

    let mut incidents = grouped.into_values().collect::<Vec<_>>();
    for incident in &mut incidents {
        incident.event_ids.sort_by(|left, right| {
            job_or_event_timestamp(&event_timestamps, left)
                .cmp(&job_or_event_timestamp(&event_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        incident.job_ids.sort_by(|left, right| {
            job_or_event_timestamp(&job_timestamps, left)
                .cmp(&job_or_event_timestamp(&job_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        if incident.label.is_empty() {
            incident.label = incident
                .browser_id
                .clone()
                .unwrap_or_else(|| "Service incidents".to_string());
        }
        if incident.current_health.is_none() {
            incident.current_health = incident
                .browser_id
                .as_ref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .map(|browser| browser.health);
        }
        if let Some(browser_id) = incident.browser_id.as_ref() {
            if browser_health_is_bad(incident.current_health) {
                incident.state = ServiceIncidentState::Active;
            } else if incident.latest_kind == "browser_health_changed"
                && state
                    .events
                    .iter()
                    .filter(|event| incident.event_ids.contains(&event.id))
                    .any(|event| {
                        event.kind == ServiceEventKind::BrowserHealthChanged
                            && browser_health_is_recovery(
                                event.previous_health,
                                event.current_health,
                            )
                            && event.browser_id.as_deref() == Some(browser_id)
                    })
            {
                incident.state = ServiceIncidentState::Recovered;
            } else {
                incident.state = ServiceIncidentState::Active;
            }
        } else {
            incident.state = ServiceIncidentState::Service;
        }
        let (severity, escalation, recommended_action) = classify_incident_escalation(incident);
        incident.severity = severity;
        incident.escalation = escalation;
        incident.recommended_action = recommended_action.to_string();
    }
    incidents.sort_by(|left, right| {
        left.latest_timestamp
            .cmp(&right.latest_timestamp)
            .reverse()
            .then_with(|| left.id.cmp(&right.id))
    });
    incidents
}

fn classify_incident_escalation(
    incident: &ServiceIncident,
) -> (
    ServiceIncidentSeverity,
    ServiceIncidentEscalation,
    &'static str,
) {
    if incident.state == ServiceIncidentState::Recovered {
        return (
            ServiceIncidentSeverity::Info,
            ServiceIncidentEscalation::None,
            "No operator action required.",
        );
    }

    match incident.current_health {
        Some(BrowserHealth::Faulted) => (
            ServiceIncidentSeverity::Critical,
            ServiceIncidentEscalation::OsDegradedPossible,
            "Inspect the host OS and process table before retrying browser automation.",
        ),
        Some(BrowserHealth::Degraded) => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::BrowserDegraded,
            "Inspect browser health and retry or relaunch the affected browser if needed.",
        ),
        Some(BrowserHealth::ProcessExited)
        | Some(BrowserHealth::CdpDisconnected)
        | Some(BrowserHealth::Unreachable) => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::BrowserRecovery,
            "Review recovery trace and retry or relaunch the affected browser.",
        ),
        _ if incident.latest_kind == "service_job_timeout" => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::JobAttention,
            "Inspect the timed-out service job and retry only after checking browser state.",
        ),
        _ if incident.latest_kind == "service_job_cancelled" => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::JobAttention,
            "Confirm the cancellation was intentional before resubmitting the task.",
        ),
        _ if incident.state == ServiceIncidentState::Service => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect service logs, reconciliation state, and recent jobs.",
        ),
        _ => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect the incident activity timeline.",
        ),
    }
}

fn job_or_event_timestamp<'a>(timestamps: &'a BTreeMap<&str, &'a str>, id: &str) -> &'a str {
    timestamps.get(id).copied().unwrap_or("")
}

fn incident_is_newer(candidate: &str, current: &str) -> bool {
    candidate >= current
}

fn service_event_is_incident(event: &ServiceEvent) -> bool {
    match event.kind {
        ServiceEventKind::ReconciliationError => true,
        ServiceEventKind::IncidentAcknowledged
        | ServiceEventKind::IncidentResolved
        | ServiceEventKind::BrowserRecoveryOverride => true,
        ServiceEventKind::BrowserHealthChanged => {
            browser_health_is_bad(event.current_health)
                || browser_health_is_recovery(event.previous_health, event.current_health)
        }
        ServiceEventKind::Reconciliation
        | ServiceEventKind::BrowserLaunchRecorded
        | ServiceEventKind::BrowserRecoveryStarted
        | ServiceEventKind::TabLifecycleChanged => false,
    }
}

fn service_event_is_handling(kind: ServiceEventKind) -> bool {
    matches!(
        kind,
        ServiceEventKind::IncidentAcknowledged
            | ServiceEventKind::IncidentResolved
            | ServiceEventKind::BrowserRecoveryOverride
    )
}

fn service_event_incident_id(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("incidentId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn browser_health_is_bad(value: Option<BrowserHealth>) -> bool {
    matches!(
        value,
        Some(BrowserHealth::Degraded)
            | Some(BrowserHealth::ProcessExited)
            | Some(BrowserHealth::CdpDisconnected)
            | Some(BrowserHealth::Unreachable)
            | Some(BrowserHealth::Faulted)
    )
}

fn browser_health_is_recovery(
    previous: Option<BrowserHealth>,
    current: Option<BrowserHealth>,
) -> bool {
    browser_health_is_bad(previous) && current == Some(BrowserHealth::Ready)
}

fn service_job_is_incident(job: &ServiceJob) -> bool {
    matches!(job.state, JobState::Cancelled | JobState::TimedOut)
}

fn service_job_incident_kind(job: &ServiceJob) -> &'static str {
    match job.state {
        JobState::TimedOut => "service_job_timeout",
        JobState::Cancelled => "service_job_cancelled",
        _ => "service_job_incident",
    }
}

fn service_job_incident_message(job: &ServiceJob) -> String {
    if job.state == JobState::TimedOut {
        format!("{} timed out", empty_to_service_label(&job.action))
    } else {
        format!("{} was cancelled", empty_to_service_label(&job.action))
    }
}

fn empty_to_service_label(value: &str) -> &str {
    if value.is_empty() {
        "Service job"
    } else {
        value
    }
}

fn service_job_incident_timestamp(job: &ServiceJob) -> &str {
    job.completed_at
        .as_deref()
        .or(job.started_at.as_deref())
        .or(job.submitted_at.as_deref())
        .unwrap_or("")
}

fn service_job_browser_id(job: &ServiceJob, state: &ServiceState) -> Option<String> {
    match &job.target {
        JobTarget::Browser(browser_id) => Some(browser_id.clone()),
        JobTarget::Tab(tab_id) => state.tabs.get(tab_id).map(|tab| tab.browser_id.clone()),
        JobTarget::Service
        | JobTarget::Profile(_)
        | JobTarget::Monitor(_)
        | JobTarget::Challenge(_) => None,
    }
}

fn service_event_kind_name(kind: ServiceEventKind) -> &'static str {
    match kind {
        ServiceEventKind::Reconciliation => "reconciliation",
        ServiceEventKind::BrowserLaunchRecorded => "browser_launch_recorded",
        ServiceEventKind::BrowserHealthChanged => "browser_health_changed",
        ServiceEventKind::BrowserRecoveryStarted => "browser_recovery_started",
        ServiceEventKind::BrowserRecoveryOverride => "browser_recovery_override",
        ServiceEventKind::TabLifecycleChanged => "tab_lifecycle_changed",
        ServiceEventKind::ReconciliationError => "reconciliation_error",
        ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
        ServiceEventKind::IncidentResolved => "incident_resolved",
    }
}

fn classify_incident_state(
    has_browser: bool,
    current_health: Option<BrowserHealth>,
    kind: ServiceEventKind,
) -> ServiceIncidentState {
    if !has_browser {
        return ServiceIncidentState::Service;
    }
    if kind == ServiceEventKind::BrowserHealthChanged && !browser_health_is_bad(current_health) {
        return ServiceIncidentState::Recovered;
    }
    ServiceIncidentState::Active
}

fn classify_job_incident_state(has_browser: bool) -> ServiceIncidentState {
    if has_browser {
        ServiceIncidentState::Active
    } else {
        ServiceIncidentState::Service
    }
}

/// Latest persisted service reconciliation result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceReconciliationSnapshot {
    pub last_reconciled_at: Option<String>,
    pub last_error: Option<String>,
    pub browser_count: usize,
    pub changed_browsers: usize,
}

/// Latest persisted control-plane status snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlPlaneSnapshot {
    pub worker_state: String,
    pub browser_health: String,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub service_job_timeout_ms: Option<u64>,
    pub updated_at: Option<String>,
}

/// Durable profile identity and launch policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProfile {
    pub id: String,
    pub name: String,
    pub user_data_dir: Option<String>,
    pub site_policy_ids: Vec<String>,
    pub default_browser_host: Option<BrowserHost>,
    pub allocation: ProfileAllocationPolicy,
    pub keyring: ProfileKeyringPolicy,
    pub shared_service_ids: Vec<String>,
    pub credential_provider_ids: Vec<String>,
    pub manual_login_preferred: bool,
    pub persistent: bool,
    pub tags: Vec<String>,
}

/// A supervised or attached browser process known to the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProcess {
    pub id: String,
    pub profile_id: Option<String>,
    pub host: BrowserHost,
    pub health: BrowserHealth,
    pub pid: Option<u32>,
    pub cdp_endpoint: Option<String>,
    pub view_streams: Vec<ViewStream>,
    pub active_session_ids: Vec<String>,
    pub last_error: Option<String>,
    pub last_health_observation: Option<BrowserHealthObservation>,
}

impl Default for BrowserProcess {
    fn default() -> Self {
        Self {
            id: String::new(),
            profile_id: None,
            host: BrowserHost::LocalHeaded,
            health: BrowserHealth::NotStarted,
            pid: None,
            cdp_endpoint: None,
            view_streams: Vec::new(),
            active_session_ids: Vec::new(),
            last_error: None,
            last_health_observation: None,
        }
    }
}

/// Latest service-owned browser health evidence retained on the browser record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserHealthObservation {
    pub observed_at: String,
    pub health: BrowserHealth,
    pub reason_kind: Option<String>,
    pub failure_class: Option<String>,
    pub process_exit_cause: Option<String>,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}

impl Default for BrowserHealthObservation {
    fn default() -> Self {
        Self {
            observed_at: String::new(),
            health: BrowserHealth::NotStarted,
            reason_kind: None,
            failure_class: None,
            process_exit_cause: None,
            message: None,
            details: None,
        }
    }
}

/// Logical lease for an agent, human, system task, or API client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserSession {
    pub id: String,
    /// Calling service label supplied by MCP, CLI, HTTP, or API clients.
    pub service_name: Option<String>,
    /// Calling agent label supplied by MCP, CLI, HTTP, or API clients.
    pub agent_name: Option<String>,
    /// Calling task label supplied by MCP, CLI, HTTP, or API clients.
    pub task_name: Option<String>,
    pub owner: ServiceActor,
    pub lease: LeaseState,
    pub profile_id: Option<String>,
    pub cleanup: SessionCleanupPolicy,
    pub browser_ids: Vec<String>,
    pub tab_ids: Vec<String>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self {
            id: String::new(),
            service_name: None,
            agent_name: None,
            task_name: None,
            owner: ServiceActor::System,
            lease: LeaseState::Shared,
            profile_id: None,
            cleanup: SessionCleanupPolicy::Detach,
            browser_ids: Vec::new(),
            tab_ids: Vec::new(),
            created_at: None,
            expires_at: None,
        }
    }
}

/// Current service view of a CDP target or browser tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserTab {
    pub id: String,
    pub browser_id: String,
    pub target_id: Option<String>,
    pub session_id: Option<String>,
    pub lifecycle: TabLifecycle,
    pub url: Option<String>,
    pub title: Option<String>,
    pub owner_session_id: Option<String>,
    pub latest_snapshot_id: Option<String>,
    pub latest_screenshot_id: Option<String>,
    pub challenge_id: Option<String>,
}

impl Default for BrowserTab {
    fn default() -> Self {
        Self {
            id: String::new(),
            browser_id: String::new(),
            target_id: None,
            session_id: None,
            lifecycle: TabLifecycle::Unknown,
            url: None,
            title: None,
            owner_session_id: None,
            latest_snapshot_id: None,
            latest_screenshot_id: None,
            challenge_id: None,
        }
    }
}

/// Queued or completed service work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceJob {
    pub id: String,
    pub action: String,
    /// Service-level caller label supplied by MCP, CLI, HTTP, or API clients.
    pub service_name: Option<String>,
    /// Agent-level caller label supplied by MCP, CLI, HTTP, or API clients.
    pub agent_name: Option<String>,
    /// Task-level caller label supplied by MCP, CLI, HTTP, or API clients.
    pub task_name: Option<String>,
    /// Non-blocking policy warnings for missing caller labels.
    ///
    /// Current warning values are the `SERVICE_JOB_NAMING_WARNING_VALUES`
    /// constants.
    pub naming_warnings: Vec<String>,
    /// True when `naming_warnings` is non-empty.
    pub has_naming_warning: bool,
    pub target: JobTarget,
    pub owner: ServiceActor,
    pub state: JobState,
    pub priority: JobPriority,
    pub submitted_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub timeout_ms: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl Default for ServiceJob {
    fn default() -> Self {
        Self {
            id: String::new(),
            action: String::new(),
            service_name: None,
            agent_name: None,
            task_name: None,
            naming_warnings: Vec::new(),
            has_naming_warning: false,
            target: JobTarget::Service,
            owner: ServiceActor::System,
            state: JobState::Queued,
            priority: JobPriority::Normal,
            submitted_at: None,
            started_at: None,
            completed_at: None,
            timeout_ms: None,
            result: None,
            error: None,
        }
    }
}

/// Site or tab heartbeat managed by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SiteMonitor {
    pub id: String,
    pub name: String,
    pub target: MonitorTarget,
    pub interval_ms: u64,
    pub state: MonitorState,
    pub last_checked_at: Option<String>,
    pub last_result: Option<String>,
}

impl Default for SiteMonitor {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            target: MonitorTarget::Url(String::new()),
            interval_ms: 60_000,
            state: MonitorState::Paused,
            last_checked_at: None,
            last_result: None,
        }
    }
}

/// Per-site access reliability and interaction policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SitePolicy {
    pub id: String,
    pub origin_pattern: String,
    pub browser_host: Option<BrowserHost>,
    pub view_stream: Option<ViewStreamProvider>,
    pub control_input: Option<ControlInputProvider>,
    pub interaction_mode: InteractionMode,
    pub rate_limit: RateLimitPolicy,
    pub manual_login_preferred: bool,
    pub profile_required: bool,
    pub auth_providers: Vec<String>,
    pub challenge_policy: ChallengePolicy,
    pub allowed_challenge_providers: Vec<String>,
    pub notes: Option<String>,
}

impl Default for SitePolicy {
    fn default() -> Self {
        Self {
            id: String::new(),
            origin_pattern: String::new(),
            browser_host: None,
            view_stream: None,
            control_input: None,
            interaction_mode: InteractionMode::CdpDirect,
            rate_limit: RateLimitPolicy::default(),
            manual_login_preferred: false,
            profile_required: false,
            auth_providers: Vec::new(),
            challenge_policy: ChallengePolicy::AvoidFirst,
            allowed_challenge_providers: Vec::new(),
            notes: None,
        }
    }
}

/// Pacing and concurrency limits for a site policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimitPolicy {
    pub min_action_delay_ms: Option<u64>,
    pub jitter_ms: Option<u64>,
    pub cooldown_ms: Option<u64>,
    pub max_parallel_sessions: Option<u32>,
    pub retry_budget: Option<u32>,
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self {
            min_action_delay_ms: Some(0),
            jitter_ms: Some(0),
            cooldown_ms: None,
            max_parallel_sessions: None,
            retry_budget: None,
        }
    }
}

/// External or built-in integration available to service workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceProvider {
    pub id: String,
    pub kind: ProviderKind,
    pub display_name: String,
    pub enabled: bool,
    pub config_ref: Option<String>,
    pub capabilities: Vec<ProviderCapability>,
}

impl Default for ServiceProvider {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: ProviderKind::ManualApproval,
            display_name: String::new(),
            enabled: true,
            config_ref: None,
            capabilities: Vec::new(),
        }
    }
}

/// Detected auth, 2FA, captcha, passkey, or blocked-flow challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Challenge {
    pub id: String,
    pub tab_id: Option<String>,
    pub kind: ChallengeKind,
    pub state: ChallengeState,
    pub detected_at: Option<String>,
    pub provider_id: Option<String>,
    pub policy_decision: Option<String>,
    pub human_approved: bool,
    pub result: Option<String>,
}

impl Default for Challenge {
    fn default() -> Self {
        Self {
            id: String::new(),
            tab_id: None,
            kind: ChallengeKind::Unknown,
            state: ChallengeState::Detected,
            detected_at: None,
            provider_id: None,
            policy_decision: None,
            human_approved: false,
            result: None,
        }
    }
}

/// Browser host execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHost {
    LocalHeadless,
    LocalHeaded,
    DockerHeaded,
    RemoteHeaded,
    CloudProvider,
    AttachedExisting,
}

/// Browser process health as seen by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHealth {
    NotStarted,
    Launching,
    Ready,
    Degraded,
    Unreachable,
    ProcessExited,
    CdpDisconnected,
    Reconnecting,
    Closing,
    Faulted,
}

/// Dashboard viewing mechanism for a browser or tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ViewStream {
    pub id: String,
    pub provider: ViewStreamProvider,
    pub url: Option<String>,
    pub read_only: bool,
}

impl Default for ViewStream {
    fn default() -> Self {
        Self {
            id: String::new(),
            provider: ViewStreamProvider::CdpScreencast,
            url: None,
            read_only: true,
        }
    }
}

/// Supported live-view transport families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewStreamProvider {
    CdpScreencast,
    ChromeTabWebrtc,
    VirtualDisplayWebrtc,
    Novnc,
    ExternalUrl,
}

/// Supported remote-input transport families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlInputProvider {
    CdpInput,
    WebrtcInput,
    VncInput,
    ManualAttachedDesktop,
}

/// Policy-selected action backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    CdpDirect,
    DomAction,
    BrowserInput,
    HumanLikeInput,
    Manual,
}

/// Challenge resolution posture for a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengePolicy {
    AvoidFirst,
    ManualOnly,
    ProviderAllowed,
    ProviderPreferred,
    Deny,
}

/// How the service may allocate or share this profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAllocationPolicy {
    #[default]
    SharedService,
    PerService,
    PerSite,
    PerIdentity,
    CallerSupplied,
}

/// Browser credential-store posture for launches using this profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKeyringPolicy {
    #[default]
    BasicPasswordStore,
    RealOsKeychain,
    ManagedVault,
    ManualLoginProfile,
}

/// Service-side actor category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceActor {
    Agent(String),
    Human(String),
    ApiClient(String),
    System,
}

/// Lease semantics for a session or tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Shared,
    Exclusive,
    HumanTakeover,
    Released,
    Expired,
}

/// What the service should do when a session lease ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCleanupPolicy {
    #[default]
    Detach,
    CloseTabs,
    CloseBrowser,
    ReleaseOnly,
}

/// Current tab lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabLifecycle {
    Unknown,
    Opening,
    Loading,
    Ready,
    Closing,
    Closed,
    Crashed,
}

/// Queue target for a service job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobTarget {
    Service,
    Browser(String),
    Tab(String),
    Profile(String),
    Monitor(String),
    Challenge(String),
}

/// Queue lifecycle for a service job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

/// Job dispatch priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    Low,
    Normal,
    Lifecycle,
}

/// Monitor target variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorTarget {
    Url(String),
    Tab(String),
    SitePolicy(String),
}

/// Monitor execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorState {
    Active,
    Paused,
    Faulted,
}

/// Provider family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    BrowserCredentials,
    PasswordManager,
    Totp,
    Sms,
    Email,
    ManualApproval,
    Intelligence,
    Captcha,
}

/// Capability advertised by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    PasswordFill,
    Passkey,
    TotpCode,
    SmsCode,
    EmailCode,
    VisualReasoning,
    CaptchaSolve,
    HumanApproval,
}

/// Challenge category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeKind {
    Unknown,
    Captcha,
    TwoFactor,
    Passkey,
    SuspiciousLogin,
    BlockedFlow,
}

/// Challenge lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeState {
    Detected,
    WaitingForProvider,
    WaitingForHuman,
    Resolved,
    Failed,
    Denied,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_schema_required_fields(schema: &serde_json::Value, fields: &[&str]) {
        for field in fields {
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|required| required == field),
                "schema missing required field {field}"
            );
        }
    }

    #[test]
    fn site_policy_serializes_stable_wire_names() {
        let policy = SitePolicy {
            id: "google".to_string(),
            origin_pattern: "https://accounts.google.com".to_string(),
            browser_host: Some(BrowserHost::DockerHeaded),
            view_stream: Some(ViewStreamProvider::VirtualDisplayWebrtc),
            control_input: Some(ControlInputProvider::WebrtcInput),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(250),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(2),
            },
            manual_login_preferred: true,
            profile_required: true,
            auth_providers: vec!["gog".to_string(), "imcli".to_string()],
            challenge_policy: ChallengePolicy::AvoidFirst,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: None,
        };

        let value = serde_json::to_value(policy).unwrap();
        assert_eq!(value["browserHost"], "docker_headed");
        assert_eq!(value["viewStream"], "virtual_display_webrtc");
        assert_eq!(value["controlInput"], "webrtc_input");
        assert_eq!(value["interactionMode"], "human_like_input");
        assert_eq!(value["rateLimit"]["minActionDelayMs"], 250);
        assert_eq!(value["challengePolicy"], "avoid_first");
    }

    #[test]
    fn service_job_record_contract_matches_wire_shape() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-job-record.v1.schema.json"
        ))
        .unwrap();
        let warning_values = SERVICE_JOB_NAMING_WARNING_VALUES.to_vec();

        assert_eq!(
            schema["properties"]["namingWarnings"]["items"]["enum"],
            json!(warning_values)
        );
        assert_eq!(
            schema["properties"]["hasNamingWarning"]["description"],
            "True when namingWarnings is non-empty."
        );
        for field in [
            "id",
            "action",
            "serviceName",
            "agentName",
            "taskName",
            "namingWarnings",
            "hasNamingWarning",
        ] {
            assert!(schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field));
        }

        let job = ServiceJob {
            id: "job-1".to_string(),
            action: "navigate".to_string(),
            naming_warnings: service_job_naming_warning_values(),
            has_naming_warning: true,
            ..ServiceJob::default()
        };
        let value = serde_json::to_value(job).unwrap();

        assert_service_job_naming_warning_contract(&value);
    }

    #[test]
    fn service_incident_record_contract_matches_wire_shape() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-incident-record.v1.schema.json"
        ))
        .unwrap();

        assert_eq!(
            schema["properties"]["state"]["enum"],
            json!(SERVICE_INCIDENT_STATE_VALUES.to_vec())
        );
        assert_eq!(
            schema["properties"]["severity"]["enum"],
            json!(SERVICE_INCIDENT_SEVERITY_VALUES.to_vec())
        );
        assert_eq!(
            schema["properties"]["escalation"]["enum"],
            json!(SERVICE_INCIDENT_ESCALATION_VALUES.to_vec())
        );
        assert_eq!(
            schema["properties"]["currentHealth"]["oneOf"][0]["enum"],
            json!(SERVICE_BROWSER_HEALTH_VALUES.to_vec())
        );
        for field in [
            "id",
            "browserId",
            "label",
            "state",
            "severity",
            "escalation",
            "recommendedAction",
            "latestTimestamp",
            "latestMessage",
            "latestKind",
            "currentHealth",
            "eventIds",
            "jobIds",
        ] {
            assert!(schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field));
        }

        let incident = ServiceIncident {
            id: "browser-1".to_string(),
            browser_id: Some("browser-1".to_string()),
            label: "browser-1".to_string(),
            state: ServiceIncidentState::Active,
            severity: ServiceIncidentSeverity::Error,
            escalation: ServiceIncidentEscalation::BrowserRecovery,
            recommended_action: "Review recovery trace and retry or relaunch the affected browser."
                .to_string(),
            latest_timestamp: "2026-04-22T00:01:00Z".to_string(),
            latest_message: "Browser crashed".to_string(),
            latest_kind: "browser_health_changed".to_string(),
            current_health: Some(BrowserHealth::ProcessExited),
            event_ids: vec!["event-1".to_string()],
            job_ids: vec!["job-1".to_string()],
            ..ServiceIncident::default()
        };
        let value = serde_json::to_value(incident).unwrap();

        assert_service_incident_record_contract(&value);
        assert_eq!(value["browserId"], "browser-1");
        assert_eq!(value["severity"], "error");
        assert_eq!(value["escalation"], "browser_recovery");
        assert_eq!(value["currentHealth"], "process_exited");
    }

    #[test]
    fn service_event_record_contract_matches_wire_shape() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-event-record.v1.schema.json"
        ))
        .unwrap();

        assert_eq!(
            schema["properties"]["kind"]["enum"],
            json!(SERVICE_EVENT_KIND_VALUES.to_vec())
        );
        assert_eq!(
            schema["properties"]["previousHealth"]["oneOf"][0]["enum"],
            json!(SERVICE_BROWSER_HEALTH_VALUES.to_vec())
        );
        assert_eq!(
            schema["properties"]["currentHealth"]["oneOf"][0]["enum"],
            json!(SERVICE_BROWSER_HEALTH_VALUES.to_vec())
        );
        for field in [
            "id",
            "timestamp",
            "kind",
            "message",
            "browserId",
            "profileId",
            "sessionId",
            "serviceName",
            "agentName",
            "taskName",
            "previousHealth",
            "currentHealth",
            "details",
        ] {
            assert!(schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field));
        }

        let event = ServiceEvent {
            id: "event-1".to_string(),
            timestamp: "2026-04-22T00:01:00Z".to_string(),
            kind: ServiceEventKind::BrowserHealthChanged,
            message: "Browser crashed".to_string(),
            browser_id: Some("browser-1".to_string()),
            profile_id: Some("work".to_string()),
            session_id: Some("session-1".to_string()),
            service_name: Some("JournalDownloader".to_string()),
            agent_name: Some("codex".to_string()),
            task_name: Some("probeACSwebsite".to_string()),
            previous_health: Some(BrowserHealth::Ready),
            current_health: Some(BrowserHealth::ProcessExited),
            details: Some(json!({"reasonKind": "process_exited"})),
        };
        let value = serde_json::to_value(event).unwrap();

        assert_service_event_record_contract(&value);
        assert_eq!(value["kind"], "browser_health_changed");
        assert_eq!(value["previousHealth"], "ready");
        assert_eq!(value["currentHealth"], "process_exited");
    }

    #[test]
    fn service_collection_record_contracts_match_wire_shape() {
        let profile_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-profile-record.v1.schema.json"
        ))
        .unwrap();
        let browser_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-browser-record.v1.schema.json"
        ))
        .unwrap();
        let session_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-session-record.v1.schema.json"
        ))
        .unwrap();
        let tab_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-tab-record.v1.schema.json"
        ))
        .unwrap();
        let site_policy_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-site-policy-record.v1.schema.json"
        ))
        .unwrap();
        let provider_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-provider-record.v1.schema.json"
        ))
        .unwrap();
        let challenge_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-challenge-record.v1.schema.json"
        ))
        .unwrap();

        assert_eq!(
            profile_schema["properties"]["defaultBrowserHost"]["oneOf"][0]["enum"],
            json!(SERVICE_BROWSER_HOST_VALUES.to_vec())
        );
        assert_eq!(
            profile_schema["properties"]["allocation"]["enum"],
            json!(SERVICE_PROFILE_ALLOCATION_VALUES.to_vec())
        );
        assert_eq!(
            profile_schema["properties"]["keyring"]["enum"],
            json!(SERVICE_PROFILE_KEYRING_VALUES.to_vec())
        );
        assert_eq!(
            browser_schema["properties"]["host"]["enum"],
            json!(SERVICE_BROWSER_HOST_VALUES.to_vec())
        );
        assert_eq!(
            browser_schema["properties"]["health"]["enum"],
            json!(SERVICE_BROWSER_HEALTH_VALUES.to_vec())
        );
        assert_eq!(
            session_schema["properties"]["lease"]["enum"],
            json!(SERVICE_LEASE_STATE_VALUES.to_vec())
        );
        assert_eq!(
            session_schema["properties"]["cleanup"]["enum"],
            json!(SERVICE_SESSION_CLEANUP_VALUES.to_vec())
        );
        assert_eq!(
            tab_schema["properties"]["lifecycle"]["enum"],
            json!(SERVICE_TAB_LIFECYCLE_VALUES.to_vec())
        );
        assert_eq!(
            site_policy_schema["properties"]["browserHost"]["oneOf"][0]["enum"],
            json!(SERVICE_BROWSER_HOST_VALUES.to_vec())
        );
        assert_eq!(
            site_policy_schema["properties"]["viewStream"]["oneOf"][0]["enum"],
            json!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.to_vec())
        );
        assert_eq!(
            site_policy_schema["properties"]["controlInput"]["oneOf"][0]["enum"],
            json!(SERVICE_CONTROL_INPUT_PROVIDER_VALUES.to_vec())
        );
        assert_eq!(
            site_policy_schema["properties"]["interactionMode"]["enum"],
            json!(SERVICE_INTERACTION_MODE_VALUES.to_vec())
        );
        assert_eq!(
            site_policy_schema["properties"]["challengePolicy"]["enum"],
            json!(SERVICE_CHALLENGE_POLICY_VALUES.to_vec())
        );
        assert_eq!(
            provider_schema["properties"]["kind"]["enum"],
            json!(SERVICE_PROVIDER_KIND_VALUES.to_vec())
        );
        assert_eq!(
            provider_schema["properties"]["capabilities"]["items"]["enum"],
            json!(SERVICE_PROVIDER_CAPABILITY_VALUES.to_vec())
        );
        assert_eq!(
            challenge_schema["properties"]["kind"]["enum"],
            json!(SERVICE_CHALLENGE_KIND_VALUES.to_vec())
        );
        assert_eq!(
            challenge_schema["properties"]["state"]["enum"],
            json!(SERVICE_CHALLENGE_STATE_VALUES.to_vec())
        );

        assert_schema_required_fields(
            &profile_schema,
            &["id", "name", "defaultBrowserHost", "allocation", "keyring"],
        );
        assert_schema_required_fields(&browser_schema, &["id", "profileId", "host", "health"]);
        assert_schema_required_fields(
            &session_schema,
            &["id", "serviceName", "lease", "profileId", "cleanup"],
        );
        assert_schema_required_fields(&tab_schema, &["id", "browserId", "sessionId", "lifecycle"]);
        assert_schema_required_fields(
            &site_policy_schema,
            &["id", "originPattern", "interactionMode", "challengePolicy"],
        );
        assert_schema_required_fields(&provider_schema, &["id", "kind", "displayName"]);
        assert_schema_required_fields(&challenge_schema, &["id", "tabId", "kind", "state"]);

        let profile = BrowserProfile {
            id: "profile-1".to_string(),
            name: "Work profile".to_string(),
            default_browser_host: Some(BrowserHost::LocalHeaded),
            allocation: ProfileAllocationPolicy::PerService,
            keyring: ProfileKeyringPolicy::BasicPasswordStore,
            manual_login_preferred: true,
            persistent: true,
            ..BrowserProfile::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("profile-1".to_string()),
            host: BrowserHost::LocalHeaded,
            health: BrowserHealth::Ready,
            pid: Some(1234),
            ..BrowserProcess::default()
        };
        let session = BrowserSession {
            id: "session-1".to_string(),
            service_name: Some("JournalDownloader".to_string()),
            agent_name: Some("codex".to_string()),
            task_name: Some("probeACSwebsite".to_string()),
            lease: LeaseState::Exclusive,
            profile_id: Some("profile-1".to_string()),
            cleanup: SessionCleanupPolicy::CloseTabs,
            ..BrowserSession::default()
        };
        let tab = BrowserTab {
            id: "tab-1".to_string(),
            browser_id: "browser-1".to_string(),
            target_id: Some("target-1".to_string()),
            session_id: Some("session-1".to_string()),
            lifecycle: TabLifecycle::Ready,
            ..BrowserTab::default()
        };
        let site_policy = SitePolicy {
            id: "google".to_string(),
            origin_pattern: "https://accounts.google.com".to_string(),
            browser_host: Some(BrowserHost::DockerHeaded),
            view_stream: Some(ViewStreamProvider::ChromeTabWebrtc),
            control_input: Some(ControlInputProvider::WebrtcInput),
            interaction_mode: InteractionMode::HumanLikeInput,
            challenge_policy: ChallengePolicy::ProviderAllowed,
            ..SitePolicy::default()
        };
        let provider = ServiceProvider {
            id: "sms".to_string(),
            kind: ProviderKind::Sms,
            display_name: "SMS".to_string(),
            capabilities: vec![ProviderCapability::SmsCode],
            ..ServiceProvider::default()
        };
        let challenge = Challenge {
            id: "challenge-1".to_string(),
            tab_id: Some("tab-1".to_string()),
            kind: ChallengeKind::TwoFactor,
            state: ChallengeState::WaitingForProvider,
            provider_id: Some("sms".to_string()),
            ..Challenge::default()
        };

        assert_service_profile_record_contract(&serde_json::to_value(profile).unwrap());
        assert_service_browser_record_contract(&serde_json::to_value(browser).unwrap());
        assert_service_session_record_contract(&serde_json::to_value(session).unwrap());
        assert_service_tab_record_contract(&serde_json::to_value(tab).unwrap());
        assert_service_site_policy_record_contract(&serde_json::to_value(site_policy).unwrap());
        assert_service_provider_record_contract(&serde_json::to_value(provider).unwrap());
        assert_service_challenge_record_contract(&serde_json::to_value(challenge).unwrap());
    }

    #[test]
    fn service_incidents_response_contract_matches_wire_shape() {
        let response_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-incidents-response.v1.schema.json"
        ))
        .unwrap();

        assert_schema_required_fields(
            &response_schema,
            &["incidents", "count", "matched", "total"],
        );
        assert_schema_required_fields(
            &response_schema["properties"]["filters"],
            &[
                "state",
                "severity",
                "escalation",
                "handlingState",
                "kind",
                "browserId",
                "profileId",
                "sessionId",
                "serviceName",
                "agentName",
                "taskName",
                "since",
                "limit",
            ],
        );

        let incident = json!({
            "id": "browser-1",
            "browserId": "browser-1",
            "label": "browser-1",
            "state": "active",
            "severity": "critical",
            "escalation": "os_degraded_possible",
            "recommendedAction": "Inspect the host OS.",
            "acknowledgedAt": null,
            "acknowledgedBy": null,
            "acknowledgementNote": null,
            "resolvedAt": null,
            "resolvedBy": null,
            "resolutionNote": null,
            "latestTimestamp": "2026-04-27T00:01:00Z",
            "latestMessage": "Force kill failed",
            "latestKind": "browser_health_changed",
            "currentHealth": "faulted",
            "eventIds": ["event-1"],
            "jobIds": ["job-1"],
        });
        let list_response = json!({
            "filters": {
                "state": null,
                "severity": "critical",
                "escalation": "os_degraded_possible",
                "handlingState": null,
                "kind": null,
                "browserId": "browser-1",
                "profileId": "work",
                "sessionId": "session-1",
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "since": null,
                "limit": 20,
            },
            "incidents": [incident.clone()],
            "count": 1,
            "matched": 1,
            "total": 1,
        });
        let detail_response = json!({
            "incident": incident.clone(),
            "incidents": [incident],
            "events": [{
                "id": "event-1",
                "timestamp": "2026-04-27T00:01:00Z",
                "kind": "browser_health_changed",
                "message": "Force kill failed",
                "browserId": "browser-1",
                "profileId": "work",
                "sessionId": "session-1",
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "previousHealth": "degraded",
                "currentHealth": "faulted",
                "details": null,
            }],
            "jobs": [],
            "count": 1,
            "matched": 1,
            "total": 1,
        });

        assert_service_incidents_response_contract(&list_response);
        assert_service_incidents_response_contract(&detail_response);
    }

    #[test]
    fn service_events_response_contract_matches_wire_shape() {
        let response_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-events-response.v1.schema.json"
        ))
        .unwrap();

        assert_schema_required_fields(&response_schema, &["events", "count", "matched", "total"]);

        let response = json!({
            "events": [{
                "id": "event-1",
                "timestamp": "2026-04-27T00:01:00Z",
                "kind": "browser_health_changed",
                "message": "Browser browser-1 health changed from degraded to faulted",
                "browserId": "browser-1",
                "profileId": "work",
                "sessionId": "session-1",
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "previousHealth": "degraded",
                "currentHealth": "faulted",
                "details": null,
            }],
            "count": 1,
            "matched": 1,
            "total": 2,
        });

        assert_service_events_response_contract(&response);
    }

    #[test]
    fn service_jobs_response_contract_matches_wire_shape() {
        let response_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-jobs-response.v1.schema.json"
        ))
        .unwrap();

        assert_schema_required_fields(&response_schema, &["jobs", "count", "matched", "total"]);

        let job = json!({
            "id": "job-1",
            "action": "navigate",
            "serviceName": "JournalDownloader",
            "agentName": "codex",
            "taskName": "probeACSwebsite",
            "namingWarnings": [],
            "hasNamingWarning": false,
            "target": {"browser": "browser-1"},
            "owner": null,
            "state": "failed",
            "priority": "normal",
            "submittedAt": "2026-04-27T00:01:00Z",
            "startedAt": "2026-04-27T00:01:01Z",
            "completedAt": "2026-04-27T00:01:02Z",
            "timeoutMs": 5000,
            "result": null,
            "error": "selector missing",
        });
        let list_response = json!({
            "jobs": [job.clone()],
            "count": 1,
            "matched": 1,
            "total": 2,
        });
        let detail_response = json!({
            "job": job.clone(),
            "jobs": [job],
            "count": 1,
            "matched": 1,
            "total": 2,
        });

        assert_service_jobs_response_contract(&list_response);
        assert_service_jobs_response_contract(&detail_response);
    }

    #[test]
    fn service_config_mutation_response_contracts_match_wire_shape() {
        let site_policy_upsert_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-site-policy-upsert-response.v1.schema.json"
        ))
        .unwrap();
        let site_policy_delete_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-site-policy-delete-response.v1.schema.json"
        ))
        .unwrap();
        let provider_upsert_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-provider-upsert-response.v1.schema.json"
        ))
        .unwrap();
        let provider_delete_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-provider-delete-response.v1.schema.json"
        ))
        .unwrap();

        assert_schema_required_fields(
            &site_policy_upsert_schema,
            &["id", "sitePolicy", "upserted"],
        );
        assert_schema_required_fields(&site_policy_delete_schema, &["id", "deleted", "sitePolicy"]);
        assert_schema_required_fields(&provider_upsert_schema, &["id", "provider", "upserted"]);
        assert_schema_required_fields(&provider_delete_schema, &["id", "deleted", "provider"]);

        let site_policy = json!({
            "id": "google",
            "originPattern": "https://accounts.google.com",
            "browserHost": null,
            "viewStream": null,
            "controlInput": null,
            "interactionMode": "human_like_input",
            "rateLimit": {},
            "manualLoginPreferred": true,
            "profileRequired": true,
            "authProviders": [],
            "challengePolicy": "avoid_first",
            "allowedChallengeProviders": [],
            "notes": null,
        });
        let provider = json!({
            "id": "manual",
            "kind": "manual_approval",
            "displayName": "Dashboard approval",
            "enabled": true,
            "configRef": null,
            "capabilities": ["human_approval"],
        });

        assert_service_site_policy_upsert_response_contract(&json!({
            "id": "google",
            "sitePolicy": site_policy.clone(),
            "upserted": true,
        }));
        assert_service_site_policy_delete_response_contract(&json!({
            "id": "google",
            "deleted": true,
            "sitePolicy": site_policy,
        }));
        assert_service_provider_upsert_response_contract(&json!({
            "id": "manual",
            "provider": provider.clone(),
            "upserted": true,
        }));
        assert_service_provider_delete_response_contract(&json!({
            "id": "manual",
            "deleted": true,
            "provider": provider,
        }));
    }

    #[test]
    fn service_trace_aggregate_contracts_match_wire_shape() {
        let incident_activity_response_schema: serde_json::Value =
            serde_json::from_str(include_str!(
                "../../../docs/dev/contracts/service-incident-activity-response.v1.schema.json"
            ))
            .unwrap();
        let response_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-trace-response.v1.schema.json"
        ))
        .unwrap();
        let summary_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-trace-summary-record.v1.schema.json"
        ))
        .unwrap();
        let activity_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-trace-activity-record.v1.schema.json"
        ))
        .unwrap();

        assert_eq!(
            summary_schema["properties"]["contexts"]["items"]["properties"]["namingWarnings"]
                ["items"]["enum"],
            json!(SERVICE_JOB_NAMING_WARNING_VALUES.to_vec())
        );
        assert_eq!(
            activity_schema["properties"]["source"]["enum"],
            json!(SERVICE_TRACE_ACTIVITY_SOURCE_VALUES.to_vec())
        );
        assert_eq!(
            activity_schema["properties"]["kind"]["enum"],
            json!(SERVICE_TRACE_ACTIVITY_KIND_VALUES.to_vec())
        );
        assert_eq!(
            activity_schema["properties"]["jobState"]["enum"],
            json!(SERVICE_JOB_STATE_VALUES.to_vec())
        );
        assert_schema_required_fields(
            &incident_activity_response_schema,
            &["incident", "activity", "count"],
        );
        assert_schema_required_fields(
            &response_schema,
            &[
                "filters",
                "events",
                "jobs",
                "incidents",
                "activity",
                "summary",
                "counts",
                "matched",
                "total",
            ],
        );
        assert_schema_required_fields(
            &response_schema["properties"]["filters"],
            &[
                "browserId",
                "profileId",
                "sessionId",
                "serviceName",
                "agentName",
                "taskName",
                "since",
                "limit",
            ],
        );
        assert_schema_required_fields(
            &response_schema["properties"]["counts"],
            &["events", "jobs", "incidents", "activity"],
        );
        assert_schema_required_fields(
            &response_schema["properties"]["matched"],
            &["events", "jobs", "incidents", "activity"],
        );
        assert_schema_required_fields(
            &response_schema["properties"]["total"],
            &["events", "jobs", "incidents"],
        );
        assert_schema_required_fields(
            &summary_schema,
            &[
                "contextCount",
                "hasTraceContext",
                "namingWarningCount",
                "contexts",
            ],
        );
        assert_schema_required_fields(
            &activity_schema,
            &["id", "source", "timestamp", "kind", "title", "message"],
        );

        let summary = json!({
            "contextCount": 1,
            "hasTraceContext": true,
            "namingWarningCount": 0,
            "contexts": [{
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "browserId": "browser-1",
                "profileId": "work",
                "sessionId": "session-1",
                "namingWarnings": [],
                "hasNamingWarning": false,
                "eventCount": 1,
                "jobCount": 1,
                "incidentCount": 0,
                "activityCount": 1,
                "latestTimestamp": "2026-04-22T00:01:00Z",
            }],
        });
        let activity = json!({
            "id": "job-1",
            "source": "job",
            "jobId": "job-1",
            "timestamp": "2026-04-22T00:01:00Z",
            "kind": "service_job_timeout",
            "title": "Service job timed out",
            "message": "Timed out",
            "jobState": "timed_out",
            "jobAction": "navigate",
            "target": {"browser": "browser-1"},
            "browserId": "browser-1",
            "profileId": "work",
            "sessionId": "session-1",
            "serviceName": "JournalDownloader",
            "agentName": "codex",
            "taskName": "probeACSwebsite",
        });
        let incident = json!({
            "id": "browser-1",
            "browserId": "browser-1",
            "label": "browser-1",
            "state": "active",
            "severity": "error",
            "escalation": "browser_recovery",
            "recommendedAction": "Review recovery trace and retry or relaunch the affected browser.",
            "acknowledgedAt": null,
            "acknowledgedBy": null,
            "acknowledgementNote": null,
            "resolvedAt": null,
            "resolvedBy": null,
            "resolutionNote": null,
            "latestTimestamp": "2026-04-22T00:01:00Z",
            "latestMessage": "Timed out",
            "latestKind": "service_job_timeout",
            "currentHealth": "process_exited",
            "eventIds": [],
            "jobIds": ["job-1"],
        });

        assert_service_trace_summary_record_contract(&summary);
        assert_service_trace_activity_record_contract(&activity);
        let incident_activity_response = json!({
            "incident": incident,
            "activity": [activity.clone()],
            "count": 1,
        });
        assert_service_incident_activity_response_contract(&incident_activity_response);
        let response = json!({
            "filters": {
                "browserId": "browser-1",
                "profileId": "work",
                "sessionId": "session-1",
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "since": null,
                "limit": 20,
            },
            "events": [],
            "jobs": [],
            "incidents": [],
            "activity": [activity],
            "summary": summary,
            "counts": {
                "events": 0,
                "jobs": 0,
                "incidents": 0,
                "activity": 1,
            },
            "matched": {
                "events": 0,
                "jobs": 0,
                "incidents": 0,
                "activity": 1,
            },
            "total": {
                "events": 0,
                "jobs": 0,
                "incidents": 0,
            },
        });
        assert_service_trace_response_contract(&response);
    }

    #[test]
    fn service_state_round_trips_nested_entities() {
        let state = ServiceState {
            control_plane: Some(ControlPlaneSnapshot {
                worker_state: "Ready".to_string(),
                browser_health: "Ready".to_string(),
                queue_depth: 0,
                queue_capacity: 256,
                service_job_timeout_ms: Some(5000),
                updated_at: Some("2026-04-22T00:00:00Z".to_string()),
            }),
            reconciliation: Some(ServiceReconciliationSnapshot {
                last_reconciled_at: Some("2026-04-22T00:01:00Z".to_string()),
                last_error: None,
                browser_count: 1,
                changed_browsers: 0,
            }),
            events: vec![ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:01:00Z".to_string(),
                kind: ServiceEventKind::Reconciliation,
                message: "Reconciled 1 browser records, 0 changed".to_string(),
                browser_id: Some("browser-1".to_string()),
                profile_id: Some("work".to_string()),
                session_id: Some("session-1".to_string()),
                service_name: Some("JournalDownloader".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("probeACSwebsite".to_string()),
                details: Some(json!({"browserCount": 1, "changedBrowsers": 0})),
                ..ServiceEvent::default()
            }],
            profiles: BTreeMap::from([(
                "work".to_string(),
                BrowserProfile {
                    id: "work".to_string(),
                    name: "Work".to_string(),
                    allocation: ProfileAllocationPolicy::PerService,
                    keyring: ProfileKeyringPolicy::ManualLoginProfile,
                    shared_service_ids: vec!["JournalDownloader".to_string()],
                    credential_provider_ids: vec!["keepassxc".to_string()],
                    manual_login_preferred: true,
                    persistent: true,
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("work".to_string()),
                    host: BrowserHost::LocalHeaded,
                    health: BrowserHealth::Ready,
                    pid: Some(42),
                    cdp_endpoint: Some("http://127.0.0.1:9222".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    service_name: Some("JournalDownloader".to_string()),
                    agent_name: Some("article-probe-agent".to_string()),
                    task_name: Some("probeACSwebsite".to_string()),
                    owner: ServiceActor::Agent("codex".to_string()),
                    lease: LeaseState::Exclusive,
                    profile_id: Some("work".to_string()),
                    cleanup: SessionCleanupPolicy::CloseTabs,
                    browser_ids: vec!["browser-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    lifecycle: TabLifecycle::Ready,
                    url: Some("https://example.com".to_string()),
                    ..BrowserTab::default()
                },
            )]),
            jobs: BTreeMap::from([(
                "job-1".to_string(),
                ServiceJob {
                    id: "job-1".to_string(),
                    action: "navigate".to_string(),
                    service_name: Some("JournalDownloader".to_string()),
                    agent_name: Some("article-probe-agent".to_string()),
                    task_name: Some("probeACSwebsite".to_string()),
                    target: JobTarget::Tab("tab-1".to_string()),
                    result: Some(json!({"ok": true})),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };

        let encoded = serde_json::to_string(&state).unwrap();
        let decoded: ServiceState = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, state);
        assert_eq!(
            decoded
                .control_plane
                .as_ref()
                .map(|snapshot| snapshot.worker_state.as_str()),
            Some("Ready")
        );
        assert_eq!(
            decoded
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.browser_count),
            Some(1)
        );
        assert_eq!(decoded.events.len(), 1);
        assert_eq!(decoded.events[0].kind, ServiceEventKind::Reconciliation);
        assert_eq!(decoded.events[0].profile_id.as_deref(), Some("work"));
        assert_eq!(decoded.events[0].session_id.as_deref(), Some("session-1"));
        assert_eq!(
            decoded.events[0].service_name.as_deref(),
            Some("JournalDownloader")
        );
        assert_eq!(decoded.events[0].agent_name.as_deref(), Some("codex"));
        assert_eq!(
            decoded.events[0].task_name.as_deref(),
            Some("probeACSwebsite")
        );
        assert_eq!(
            decoded.profiles["work"].allocation,
            ProfileAllocationPolicy::PerService
        );
        assert_eq!(
            decoded.profiles["work"].keyring,
            ProfileKeyringPolicy::ManualLoginProfile
        );
        assert_eq!(decoded.browsers["browser-1"].health, BrowserHealth::Ready);
        assert_eq!(
            decoded.sessions["session-1"].owner,
            ServiceActor::Agent("codex".to_string())
        );
        assert_eq!(
            decoded.sessions["session-1"].service_name.as_deref(),
            Some("JournalDownloader")
        );
        assert_eq!(
            decoded.sessions["session-1"].cleanup,
            SessionCleanupPolicy::CloseTabs
        );
        assert_eq!(
            decoded.jobs["job-1"].service_name.as_deref(),
            Some("JournalDownloader")
        );
        assert_eq!(
            decoded.jobs["job-1"].agent_name.as_deref(),
            Some("article-probe-agent")
        );
        assert_eq!(
            decoded.jobs["job-1"].task_name.as_deref(),
            Some("probeACSwebsite")
        );
    }

    #[test]
    fn refresh_derived_views_groups_incidents_by_browser() {
        let mut state = ServiceState {
            events: vec![
                ServiceEvent {
                    id: "event-crash".to_string(),
                    timestamp: "2026-04-22T00:02:00Z".to_string(),
                    kind: ServiceEventKind::BrowserHealthChanged,
                    message: "Browser browser-1 health changed from Ready to ProcessExited"
                        .to_string(),
                    browser_id: Some("browser-1".to_string()),
                    previous_health: Some(BrowserHealth::Ready),
                    current_health: Some(BrowserHealth::ProcessExited),
                    ..ServiceEvent::default()
                },
                ServiceEvent {
                    id: "event-recover".to_string(),
                    timestamp: "2026-04-22T00:03:00Z".to_string(),
                    kind: ServiceEventKind::BrowserHealthChanged,
                    message: "Browser browser-1 health changed from ProcessExited to Ready"
                        .to_string(),
                    browser_id: Some("browser-1".to_string()),
                    previous_health: Some(BrowserHealth::ProcessExited),
                    current_health: Some(BrowserHealth::Ready),
                    ..ServiceEvent::default()
                },
                ServiceEvent {
                    id: "event-reconcile-error".to_string(),
                    timestamp: "2026-04-22T00:04:00Z".to_string(),
                    kind: ServiceEventKind::ReconciliationError,
                    message: "Failed to reconcile service state".to_string(),
                    ..ServiceEvent::default()
                },
            ],
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    ..BrowserTab::default()
                },
            )]),
            jobs: BTreeMap::from([
                (
                    "job-cancelled".to_string(),
                    ServiceJob {
                        id: "job-cancelled".to_string(),
                        action: "navigate".to_string(),
                        target: JobTarget::Tab("tab-1".to_string()),
                        state: JobState::Cancelled,
                        completed_at: Some("2026-04-22T00:05:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                ),
                (
                    "job-timeout".to_string(),
                    ServiceJob {
                        id: "job-timeout".to_string(),
                        action: "snapshot".to_string(),
                        target: JobTarget::Service,
                        state: JobState::TimedOut,
                        completed_at: Some("2026-04-22T00:06:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        assert_eq!(state.incidents.len(), 2);
        assert_eq!(state.incidents[0].id, "service");
        assert_eq!(state.incidents[0].state, ServiceIncidentState::Service);
        assert_eq!(state.incidents[0].severity, ServiceIncidentSeverity::Error);
        assert_eq!(
            state.incidents[0].escalation,
            ServiceIncidentEscalation::JobAttention
        );
        assert_eq!(state.incidents[0].latest_kind, "service_job_timeout");
        assert_eq!(state.incidents[0].event_ids, vec!["event-reconcile-error"]);
        assert_eq!(state.incidents[0].job_ids, vec!["job-timeout"]);
        assert_eq!(state.incidents[1].id, "browser-1");
        assert_eq!(state.incidents[1].browser_id.as_deref(), Some("browser-1"));
        assert_eq!(state.incidents[1].state, ServiceIncidentState::Active);
        assert_eq!(
            state.incidents[1].severity,
            ServiceIncidentSeverity::Warning
        );
        assert_eq!(
            state.incidents[1].escalation,
            ServiceIncidentEscalation::JobAttention
        );
        assert_eq!(state.incidents[1].latest_kind, "service_job_cancelled");
        assert_eq!(
            state.incidents[1].event_ids,
            vec!["event-recover", "event-crash"]
        );
        assert_eq!(state.incidents[1].job_ids, vec!["job-cancelled"]);
        assert_eq!(
            state.incidents[1].current_health,
            Some(BrowserHealth::Ready)
        );
    }

    #[test]
    fn refresh_derived_views_classifies_shutdown_remedy_severity() {
        let mut state = ServiceState {
            events: vec![
                ServiceEvent {
                    id: "event-degraded".to_string(),
                    timestamp: "2026-04-27T00:00:00Z".to_string(),
                    kind: ServiceEventKind::BrowserHealthChanged,
                    message: "Browser browser-1 health changed from Ready to Degraded".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    previous_health: Some(BrowserHealth::Ready),
                    current_health: Some(BrowserHealth::Degraded),
                    ..ServiceEvent::default()
                },
                ServiceEvent {
                    id: "event-faulted".to_string(),
                    timestamp: "2026-04-27T00:01:00Z".to_string(),
                    kind: ServiceEventKind::BrowserHealthChanged,
                    message: "Browser browser-2 health changed from Degraded to Faulted"
                        .to_string(),
                    browser_id: Some("browser-2".to_string()),
                    previous_health: Some(BrowserHealth::Degraded),
                    current_health: Some(BrowserHealth::Faulted),
                    ..ServiceEvent::default()
                },
            ],
            browsers: BTreeMap::from([
                (
                    "browser-1".to_string(),
                    BrowserProcess {
                        id: "browser-1".to_string(),
                        health: BrowserHealth::Degraded,
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "browser-2".to_string(),
                    BrowserProcess {
                        id: "browser-2".to_string(),
                        health: BrowserHealth::Faulted,
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        let degraded = state
            .incidents
            .iter()
            .find(|incident| incident.id == "browser-1")
            .unwrap();
        assert_eq!(degraded.severity, ServiceIncidentSeverity::Warning);
        assert_eq!(
            degraded.escalation,
            ServiceIncidentEscalation::BrowserDegraded
        );
        assert!(degraded.recommended_action.contains("browser health"));

        let faulted = state
            .incidents
            .iter()
            .find(|incident| incident.id == "browser-2")
            .unwrap();
        assert_eq!(faulted.severity, ServiceIncidentSeverity::Critical);
        assert_eq!(
            faulted.escalation,
            ServiceIncidentEscalation::OsDegradedPossible
        );
        assert!(faulted.recommended_action.contains("host OS"));
    }

    #[test]
    fn refresh_derived_views_preserves_incident_operator_metadata() {
        let mut state = ServiceState {
            incidents: vec![ServiceIncident {
                id: "browser-1".to_string(),
                acknowledged_at: Some("2026-04-22T00:09:00Z".to_string()),
                acknowledged_by: Some("operator".to_string()),
                acknowledgement_note: Some("Investigating".to_string()),
                resolved_at: Some("2026-04-22T00:10:00Z".to_string()),
                resolved_by: Some("operator".to_string()),
                resolution_note: Some("Recovered".to_string()),
                ..ServiceIncident::default()
            }],
            events: vec![ServiceEvent {
                id: "event-crash".to_string(),
                timestamp: "2026-04-22T00:02:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser browser-1 health changed from Ready to ProcessExited".to_string(),
                browser_id: Some("browser-1".to_string()),
                previous_health: Some(BrowserHealth::Ready),
                current_health: Some(BrowserHealth::ProcessExited),
                ..ServiceEvent::default()
            }],
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::ProcessExited,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        assert_eq!(state.incidents.len(), 1);
        assert_eq!(
            state.incidents[0].acknowledged_at.as_deref(),
            Some("2026-04-22T00:09:00Z")
        );
        assert_eq!(state.incidents[0].resolved_by.as_deref(), Some("operator"));
        assert_eq!(
            state.incidents[0].resolution_note.as_deref(),
            Some("Recovered")
        );
    }

    #[test]
    fn configured_entities_overlay_persisted_state() {
        let mut persisted = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "persisted".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };
        let configured = ServiceState {
            profiles: BTreeMap::from([(
                "work".to_string(),
                BrowserProfile {
                    id: "work".to_string(),
                    name: "Configured Work".to_string(),
                    allocation: ProfileAllocationPolicy::PerService,
                    keyring: ProfileKeyringPolicy::BasicPasswordStore,
                    shared_service_ids: vec!["JournalDownloader".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "service-session".to_string(),
                BrowserSession {
                    id: "service-session".to_string(),
                    service_name: Some("JournalDownloader".to_string()),
                    profile_id: Some("work".to_string()),
                    lease: LeaseState::Exclusive,
                    cleanup: SessionCleanupPolicy::CloseTabs,
                    ..BrowserSession::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "configured".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            providers: BTreeMap::from([(
                "manual".to_string(),
                ServiceProvider {
                    id: "manual".to_string(),
                    display_name: "Dashboard approval".to_string(),
                    ..ServiceProvider::default()
                },
            )]),
            ..ServiceState::default()
        };

        persisted.overlay_configured_entities(configured);

        assert!(persisted.browsers.contains_key("browser-1"));
        assert_eq!(persisted.profiles["work"].name, "Configured Work");
        assert_eq!(
            persisted.sessions["service-session"].profile_id.as_deref(),
            Some("work")
        );
        assert_eq!(
            persisted.site_policies["google"].origin_pattern,
            "configured"
        );
        assert_eq!(
            persisted.providers["manual"].display_name,
            "Dashboard approval"
        );
    }

    #[test]
    fn provider_and_challenge_model_capabilities() {
        let provider = ServiceProvider {
            id: "manual".to_string(),
            kind: ProviderKind::ManualApproval,
            display_name: "Dashboard approval".to_string(),
            capabilities: vec![ProviderCapability::HumanApproval],
            ..ServiceProvider::default()
        };
        let challenge = Challenge {
            id: "challenge-1".to_string(),
            kind: ChallengeKind::TwoFactor,
            state: ChallengeState::WaitingForHuman,
            provider_id: Some(provider.id.clone()),
            policy_decision: Some("manual_only".to_string()),
            ..Challenge::default()
        };

        let provider_value = serde_json::to_value(provider).unwrap();
        let challenge_value = serde_json::to_value(challenge).unwrap();

        assert_eq!(provider_value["kind"], "manual_approval");
        assert_eq!(provider_value["capabilities"][0], "human_approval");
        assert_eq!(challenge_value["kind"], "two_factor");
        assert_eq!(challenge_value["state"], "waiting_for_human");
    }
}
