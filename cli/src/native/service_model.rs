//! Durable service-mode contracts.
//!
//! These types describe the browser service state model before the service API
//! and MCP surfaces are wired to runtime behavior. Keep them serializable and
//! conservative so future clients can depend on stable field names.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
pub const SERVICE_INCIDENT_ESCALATION_VALUES: [&str; 7] = [
    "none",
    "browser_degraded",
    "browser_recovery",
    "job_attention",
    "monitor_attention",
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

/// In-memory provenance for service entities after persisted state, config, and
/// shipped defaults are layered. This is intentionally not serialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEntitySource {
    PersistedState,
    Config,
    Builtin,
    RuntimeObserved,
}

impl ServiceEntitySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PersistedState => "persisted_state",
            Self::Config => "config",
            Self::Builtin => "builtin",
            Self::RuntimeObserved => "runtime_observed",
        }
    }

    pub fn overrideable(self) -> bool {
        self == Self::Builtin
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceEntitySources {
    pub profiles: BTreeMap<String, ServiceEntitySource>,
    pub site_policies: BTreeMap<String, ServiceEntitySource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitePolicySourceRecord {
    pub id: String,
    pub source: String,
    pub overrideable: bool,
    pub precedence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSourceRecord {
    pub id: String,
    pub source: String,
    pub overrideable: bool,
    pub precedence: Vec<String>,
}
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
pub const SERVICE_PROFILE_SELECTION_REASON_VALUES: [&str; 4] = [
    "explicit_profile",
    "authenticated_target",
    "target_match",
    "service_allow_list",
];
pub const SERVICE_PROFILE_LEASE_DISPOSITION_VALUES: [&str; 3] =
    ["new_browser", "reused_browser", "active_lease_conflict"];
pub const SERVICE_PROFILE_READINESS_VALUES: [&str; 6] = [
    "unknown",
    "needs_manual_seeding",
    "seeded_unknown_freshness",
    "fresh",
    "stale",
    "blocked_by_attached_devtools",
];
pub const SERVICE_PROFILE_SEEDING_MODE_VALUES: [&str; 3] =
    ["not_required", "detached_headed_no_cdp", "attachable_ok"];
pub const SERVICE_PROFILE_SEEDING_HANDOFF_STATE_VALUES: [&str; 10] = [
    "not_required",
    "needs_manual_seeding",
    "seeding_launched_detached",
    "seeding_waiting_for_close",
    "completion_declared_waiting_for_close",
    "seeding_closed_unverified",
    "verification_pending",
    "fresh",
    "failed",
    "abandoned",
];
pub const SERVICE_TAB_LIFECYCLE_VALUES: [&str; 7] = [
    "unknown", "opening", "loading", "ready", "closing", "closed", "crashed",
];
pub const SERVICE_MONITOR_STATE_VALUES: [&str; 3] = ["active", "paused", "faulted"];
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
pub const SERVICE_EVENT_KIND_VALUES: [&str; 11] = [
    "reconciliation",
    "browser_launch_recorded",
    "browser_health_changed",
    "browser_recovery_started",
    "browser_recovery_override",
    "tab_lifecycle_changed",
    "profile_lease_wait_started",
    "profile_lease_wait_ended",
    "reconciliation_error",
    "incident_acknowledged",
    "incident_resolved",
];
pub const SERVICE_TRACE_ACTIVITY_SOURCE_VALUES: [&str; 3] = ["event", "job", "metadata"];
pub const SERVICE_TRACE_ACTIVITY_KIND_VALUES: [&str; 14] = [
    "reconciliation",
    "browser_launch_recorded",
    "browser_health_changed",
    "browser_recovery_started",
    "browser_recovery_override",
    "tab_lifecycle_changed",
    "profile_lease_wait_started",
    "profile_lease_wait_ended",
    "reconciliation_error",
    "incident_acknowledged",
    "incident_resolved",
    "service_job_timeout",
    "service_job_cancelled",
    "service_job",
];
pub const SERVICE_JOB_STATE_VALUES: [&str; 7] = [
    "queued",
    "waiting_profile_lease",
    "running",
    "succeeded",
    "failed",
    "cancelled",
    "timed_out",
];
pub const SERVICE_JOB_PRIORITY_VALUES: [&str; 3] = ["low", "normal", "lifecycle"];
pub const SERVICE_JOB_CONTROL_PLANE_MODE_VALUES: [&str; 3] = ["cdp", "cdp_free", "service"];

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
            "monitorId",
            "monitorTarget",
            "monitorResult",
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
            "monitor_id",
            "monitor_target",
            "monitor_result",
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
            "targetServiceIds",
            "authenticatedServiceIds",
            "defaultBrowserHost",
            "allocation",
            "keyring",
            "sharedServiceIds",
            "credentialProviderIds",
            "manualLoginPreferred",
            "targetReadiness",
            "persistent",
            "tags",
        ],
        &[
            "user_data_dir",
            "site_policy_ids",
            "target_service_ids",
            "authenticated_service_ids",
            "default_browser_host",
            "shared_service_ids",
            "credential_provider_ids",
            "manual_login_preferred",
            "target_readiness",
        ],
    );
    if let Some(host) = value["defaultBrowserHost"].as_str() {
        assert!(SERVICE_BROWSER_HOST_VALUES.contains(&host));
    }
    assert!(SERVICE_PROFILE_ALLOCATION_VALUES.contains(&value["allocation"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_KEYRING_VALUES.contains(&value["keyring"].as_str().unwrap()));
    assert!(value["sitePolicyIds"].is_array());
    assert!(value["targetServiceIds"].is_array());
    assert!(value["authenticatedServiceIds"].is_array());
    assert!(value["sharedServiceIds"].is_array());
    assert!(value["credentialProviderIds"].is_array());
    assert!(value["targetReadiness"].is_array());
    for readiness in value["targetReadiness"].as_array().unwrap() {
        assert_service_profile_readiness_contract(readiness);
    }
    assert!(value["tags"].is_array());
}

#[cfg(test)]
pub fn assert_service_profile_readiness_contract(value: &serde_json::Value) {
    assert_record_fields(
        "profile target readiness",
        value,
        &[
            "targetServiceId",
            "loginId",
            "state",
            "manualSeedingRequired",
            "evidence",
            "recommendedAction",
            "seedingMode",
            "cdpAttachmentAllowedDuringSeeding",
            "preferredKeyring",
            "setupScopes",
            "lastVerifiedAt",
            "freshnessExpiresAt",
        ],
        &[
            "target_service_id",
            "login_id",
            "manual_seeding_required",
            "recommended_action",
            "seeding_mode",
            "cdp_attachment_allowed_during_seeding",
            "preferred_keyring",
            "setup_scopes",
            "last_verified_at",
            "freshness_expires_at",
        ],
    );
    assert!(SERVICE_PROFILE_READINESS_VALUES.contains(&value["state"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_SEEDING_MODE_VALUES.contains(&value["seedingMode"].as_str().unwrap()));
    assert!(value["cdpAttachmentAllowedDuringSeeding"].is_boolean());
    if let Some(keyring) = value["preferredKeyring"].as_str() {
        assert!(SERVICE_PROFILE_KEYRING_VALUES.contains(&keyring));
    } else {
        assert!(value["preferredKeyring"].is_null());
    }
    assert!(value["setupScopes"].is_array());
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
            "profileSelectionReason",
            "profileLeaseDisposition",
            "profileLeaseConflictSessionIds",
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
            "profile_selection_reason",
            "profile_lease_disposition",
            "profile_lease_conflict_session_ids",
            "browser_ids",
            "tab_ids",
            "created_at",
            "expires_at",
        ],
    );
    assert!(SERVICE_LEASE_STATE_VALUES.contains(&value["lease"].as_str().unwrap()));
    if let Some(reason) = value["profileSelectionReason"].as_str() {
        assert!(SERVICE_PROFILE_SELECTION_REASON_VALUES.contains(&reason));
    }
    if let Some(disposition) = value["profileLeaseDisposition"].as_str() {
        assert!(SERVICE_PROFILE_LEASE_DISPOSITION_VALUES.contains(&disposition));
    }
    assert!(value["profileLeaseConflictSessionIds"].is_array());
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
pub fn assert_service_monitor_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "monitor",
        value,
        &[
            "id",
            "name",
            "target",
            "intervalMs",
            "state",
            "lastCheckedAt",
            "lastSucceededAt",
            "lastFailedAt",
            "lastResult",
            "consecutiveFailures",
        ],
        &[
            "interval_ms",
            "last_checked_at",
            "last_succeeded_at",
            "last_failed_at",
            "last_result",
            "consecutive_failures",
        ],
    );
    assert!(value["target"].is_object());
    assert!(SERVICE_MONITOR_STATE_VALUES.contains(&value["state"].as_str().unwrap()));
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
            "requiresCdpFree",
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
            "requires_cdp_free",
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
            "profileLeaseWaits",
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
    let profile_lease_waits = &value["profileLeaseWaits"];
    assert_record_fields(
        "trace profile lease waits",
        profile_lease_waits,
        &["count", "activeCount", "completedCount", "waits"],
        &["active_count", "completed_count"],
    );
    let waits = profile_lease_waits["waits"].as_array().unwrap();
    assert_eq!(
        profile_lease_waits["count"].as_u64().unwrap(),
        waits.len() as u64
    );
    assert!(profile_lease_waits["activeCount"].is_u64());
    assert!(profile_lease_waits["completedCount"].is_u64());
    for wait in waits {
        assert_record_fields(
            "trace profile lease wait",
            wait,
            &[
                "jobId",
                "profileId",
                "outcome",
                "startedAt",
                "endedAt",
                "waitedMs",
                "retryAfterMs",
                "conflictSessionIds",
                "serviceName",
                "agentName",
                "taskName",
            ],
            &[
                "job_id",
                "profile_id",
                "started_at",
                "ended_at",
                "waited_ms",
                "retry_after_ms",
                "conflict_session_ids",
                "service_name",
                "agent_name",
                "task_name",
            ],
        );
        assert!(wait["conflictSessionIds"].is_array());
    }
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
                "targetIdentityCount",
                "targetServiceIds",
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
                "target_identity_count",
                "target_service_ids",
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
        assert!(context["targetIdentityCount"].is_u64());
        assert!(context["targetServiceIds"].is_array());
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
                "remediesOnly",
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
        assert!(filters["remediesOnly"].is_boolean());
    }
    if let Some(incident) = value.get("incident") {
        assert_service_incident_record_contract(incident);
    }
    if let Some(summary) = value.get("summary") {
        assert_record_fields("incidents summary", summary, &["groupCount", "groups"], &[]);
        let groups = summary["groups"].as_array().unwrap();
        assert_eq!(
            summary["groupCount"].as_u64().unwrap(),
            groups.len() as u64,
            "incidents summary groupCount does not match groups length"
        );
        for group in groups {
            assert_record_fields(
                "incidents summary group",
                group,
                &[
                    "escalation",
                    "severity",
                    "state",
                    "count",
                    "latestTimestamp",
                    "recommendedAction",
                    "incidentIds",
                    "browserIds",
                    "monitorIds",
                    "remedyApplyCommand",
                ],
                &[],
            );
            assert!(group["incidentIds"].is_array());
            assert!(group["browserIds"].is_array());
            assert!(group["monitorIds"].is_array());
            assert!(
                group["remedyApplyCommand"].is_string() || group["remedyApplyCommand"].is_null()
            );
        }
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
                "controlPlaneMode",
                "lifecycleOnly",
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
                "control_plane_mode",
                "lifecycle_only",
                "submitted_at",
                "started_at",
                "completed_at",
                "timeout_ms",
            ],
        );
        assert!(SERVICE_JOB_STATE_VALUES.contains(&job["state"].as_str().unwrap()));
        assert!(SERVICE_JOB_PRIORITY_VALUES.contains(&job["priority"].as_str().unwrap()));
        assert!(SERVICE_JOB_CONTROL_PLANE_MODE_VALUES
            .contains(&job["controlPlaneMode"].as_str().unwrap()));
        assert!(job["namingWarnings"].is_array());
        assert!(job["hasNamingWarning"].is_boolean());
        assert!(job["lifecycleOnly"].is_boolean());
    }
    if let Some(job) = value.get("job") {
        assert!(
            jobs.iter().any(|item| item["id"] == job["id"]),
            "jobs response detail job is not present in jobs array"
        );
    }
}

#[cfg(test)]
pub fn assert_service_job_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "job",
        value,
        &[
            "id",
            "action",
            "serviceName",
            "agentName",
            "taskName",
            "targetServiceId",
            "siteId",
            "loginId",
            "targetServiceIds",
            "namingWarnings",
            "hasNamingWarning",
            "controlPlaneMode",
            "lifecycleOnly",
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
            "target_service_id",
            "site_id",
            "login_id",
            "target_service_ids",
            "naming_warnings",
            "has_naming_warning",
            "control_plane_mode",
            "lifecycle_only",
            "submitted_at",
            "started_at",
            "completed_at",
            "timeout_ms",
        ],
    );
    assert!(SERVICE_JOB_STATE_VALUES.contains(&value["state"].as_str().unwrap()));
    assert!(SERVICE_JOB_PRIORITY_VALUES.contains(&value["priority"].as_str().unwrap()));
    assert!(SERVICE_JOB_CONTROL_PLANE_MODE_VALUES
        .contains(&value["controlPlaneMode"].as_str().unwrap()));
    assert!(value["namingWarnings"].is_array());
    assert!(value["hasNamingWarning"].is_boolean());
    assert!(value["lifecycleOnly"].is_boolean());
}

#[cfg(test)]
pub fn assert_service_profile_upsert_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "profile upsert response",
        value,
        &["id", "profile", "upserted"],
        &[],
    );
    assert!(value["id"].is_string());
    assert_eq!(value["upserted"], true);
    assert_service_profile_record_contract(&value["profile"]);
}

#[cfg(test)]
pub fn assert_service_profile_delete_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "profile delete response",
        value,
        &["id", "deleted", "profile"],
        &[],
    );
    assert!(value["id"].is_string());
    assert!(value["deleted"].is_boolean());
    if value["profile"].is_object() {
        assert_service_profile_record_contract(&value["profile"]);
    } else {
        assert!(value["profile"].is_null());
    }
}

#[cfg(test)]
pub fn assert_service_session_upsert_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "session upsert response",
        value,
        &["id", "session", "upserted"],
        &[],
    );
    assert!(value["id"].is_string());
    assert_eq!(value["upserted"], true);
    assert_service_session_record_contract(&value["session"]);
}

#[cfg(test)]
pub fn assert_service_session_delete_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "session delete response",
        value,
        &["id", "deleted", "session"],
        &[],
    );
    assert!(value["id"].is_string());
    assert!(value["deleted"].is_boolean());
    if value["session"].is_object() {
        assert_service_session_record_contract(&value["session"]);
    } else {
        assert!(value["session"].is_null());
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
pub fn assert_service_monitor_upsert_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "monitor upsert response",
        value,
        &["id", "monitor", "upserted"],
        &[],
    );
    assert!(value["id"].is_string());
    assert_eq!(value["upserted"], true);
    assert_service_monitor_record_contract(&value["monitor"]);
}

#[cfg(test)]
pub fn assert_service_monitor_delete_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "monitor delete response",
        value,
        &["id", "deleted", "monitor"],
        &[],
    );
    assert!(value["id"].is_string());
    assert!(value["deleted"].is_boolean());
    if value["monitor"].is_object() {
        assert_service_monitor_record_contract(&value["monitor"]);
    } else {
        assert!(value["monitor"].is_null());
    }
}

#[cfg(test)]
pub fn assert_service_monitor_state_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "monitor state response",
        value,
        &["id", "monitor", "state", "updated"],
        &[],
    );
    assert!(value["id"].is_string());
    assert!(SERVICE_MONITOR_STATE_VALUES.contains(&value["state"].as_str().unwrap()));
    assert_eq!(value["updated"], true);
    assert_service_monitor_record_contract(&value["monitor"]);
}

#[cfg(test)]
pub fn assert_service_monitor_triage_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "monitor triage response",
        value,
        &[
            "id",
            "monitor",
            "state",
            "updated",
            "resetFailures",
            "acknowledged",
            "incident",
        ],
        &[],
    );
    assert_service_monitor_state_response_contract(value);
    assert_eq!(value["resetFailures"], true);
    assert!(value["acknowledged"].is_boolean());
    if value["incident"].is_object() {
        assert_service_incident_record_contract(&value["incident"]);
    } else {
        assert!(value["incident"].is_null());
    }
}

#[cfg(test)]
pub fn assert_service_remedies_apply_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "service remedies apply response",
        value,
        &[
            "applied",
            "escalation",
            "count",
            "monitorIds",
            "monitorResults",
            "browserIds",
            "browserResults",
        ],
        &[],
    );
    assert_eq!(value["applied"], true);
    assert!(matches!(
        value["escalation"].as_str(),
        Some("browser_degraded" | "monitor_attention" | "os_degraded_possible")
    ));
    assert!(value["count"].is_u64());
    assert!(value["monitorIds"].is_array());
    for result in value["monitorResults"].as_array().unwrap() {
        assert_service_monitor_triage_response_contract(result);
    }
    assert!(value["browserIds"].is_array());
    for result in value["browserResults"].as_array().unwrap() {
        assert_record_fields(
            "service remedies apply browser result",
            result,
            &["id", "retryEnabled", "browser", "incident"],
            &[],
        );
        assert_eq!(result["retryEnabled"], true);
        assert_service_browser_record_contract(&result["browser"]);
        if result["incident"].is_object() {
            assert_service_incident_record_contract(&result["incident"]);
        } else {
            assert!(result["incident"].is_null());
        }
    }
}

#[cfg(test)]
pub fn assert_service_monitor_run_due_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "monitor run-due response",
        value,
        &["checked", "succeeded", "failed", "monitorIds"],
        &[],
    );
    assert!(value["checked"].is_u64());
    assert!(value["succeeded"].is_u64());
    assert!(value["failed"].is_u64());
    let monitor_ids = value["monitorIds"].as_array().unwrap();
    assert!(monitor_ids.iter().all(|id| id.is_string()));
}

#[cfg(test)]
pub fn assert_service_job_cancel_response_contract(value: &serde_json::Value) {
    assert_record_fields("job cancel response", value, &["cancelled", "job"], &[]);
    assert!(value["cancelled"].is_boolean());
    assert_service_job_record_contract(&value["job"]);
}

#[cfg(test)]
pub fn assert_service_browser_retry_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "browser retry response",
        value,
        &["retryEnabled", "browser", "incident"],
        &["retry_enabled"],
    );
    assert!(value["retryEnabled"].is_boolean());
    assert_service_browser_record_contract(&value["browser"]);
    if value["incident"].is_object() {
        assert_service_incident_record_contract(&value["incident"]);
    } else {
        assert!(value["incident"].is_null());
    }
}

#[cfg(test)]
pub fn assert_service_incident_acknowledge_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "incident acknowledge response",
        value,
        &["acknowledged", "incident"],
        &[],
    );
    assert!(value["acknowledged"].is_boolean());
    assert_service_incident_record_contract(&value["incident"]);
}

#[cfg(test)]
pub fn assert_service_incident_resolve_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "incident resolve response",
        value,
        &["resolved", "incident"],
        &[],
    );
    assert!(value["resolved"].is_boolean());
    assert_service_incident_record_contract(&value["incident"]);
}

#[cfg(test)]
pub fn assert_service_reconcile_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "service reconcile response",
        value,
        &[
            "reconciled",
            "browserCount",
            "changedBrowsers",
            "service_state",
        ],
        &["browser_count", "changed_browsers"],
    );
    assert!(value["reconciled"].is_boolean());
    assert!(value["browserCount"].is_u64());
    assert!(value["changedBrowsers"].is_u64());
    assert!(value["service_state"].is_object());
}

#[cfg(test)]
pub fn assert_service_status_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "service status response",
        value,
        &["service_state", "profileAllocations"],
        &["serviceState"],
    );
    assert!(value["service_state"].is_object());
    for allocation in value["profileAllocations"].as_array().unwrap() {
        assert_service_profile_allocation_contract(allocation);
    }
    if let Some(control_plane) = value.get("control_plane") {
        assert_record_fields(
            "service status control plane",
            control_plane,
            &[
                "waiting_profile_lease_job_count",
                "service_monitor_interval_ms",
            ],
            &["waitingProfileLeaseJobCount", "serviceMonitorIntervalMs"],
        );
        assert!(control_plane["waiting_profile_lease_job_count"].is_u64());
    }
}

#[cfg(test)]
pub fn assert_service_profile_allocation_contract(value: &serde_json::Value) {
    assert_record_fields(
        "profile allocation",
        value,
        &[
            "profileId",
            "profileName",
            "allocation",
            "keyring",
            "targetServiceIds",
            "authenticatedServiceIds",
            "targetReadiness",
            "sharedServiceIds",
            "holderSessionIds",
            "holderCount",
            "exclusiveHolderSessionIds",
            "waitingJobIds",
            "waitingJobCount",
            "conflictSessionIds",
            "leaseState",
            "recommendedAction",
            "serviceNames",
            "agentNames",
            "taskNames",
            "browserIds",
            "tabIds",
        ],
        &[
            "profile_id",
            "profile_name",
            "target_service_ids",
            "authenticated_service_ids",
            "target_readiness",
            "shared_service_ids",
            "holder_session_ids",
            "holder_count",
            "exclusive_holder_session_ids",
            "waiting_job_ids",
            "waiting_job_count",
            "conflict_session_ids",
            "lease_state",
            "recommended_action",
            "service_names",
            "agent_names",
            "task_names",
            "browser_ids",
            "tab_ids",
        ],
    );
    assert!(SERVICE_PROFILE_ALLOCATION_VALUES.contains(&value["allocation"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_KEYRING_VALUES.contains(&value["keyring"].as_str().unwrap()));
    assert!(value["targetServiceIds"].is_array());
    assert!(value["authenticatedServiceIds"].is_array());
    assert!(value["targetReadiness"].is_array());
    for readiness in value["targetReadiness"].as_array().unwrap() {
        assert_service_profile_readiness_contract(readiness);
    }
    assert!(value["sharedServiceIds"].is_array());
    assert!(value["holderSessionIds"].is_array());
    assert!(value["holderCount"].is_u64());
    assert!(value["exclusiveHolderSessionIds"].is_array());
    assert!(value["waitingJobIds"].is_array());
    assert!(value["waitingJobCount"].is_u64());
    assert!(value["conflictSessionIds"].is_array());
    assert!(value["leaseState"].is_string());
    assert!(value["recommendedAction"].is_string());
    assert!(value["serviceNames"].is_array());
    assert!(value["agentNames"].is_array());
    assert!(value["taskNames"].is_array());
    assert!(value["browserIds"].is_array());
    assert!(value["tabIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_collection_response_contract(
    value: &serde_json::Value,
    field: &str,
    label: &str,
) {
    assert_record_fields(label, value, &[field, "count"], &[]);
    if field == "profiles" {
        assert!(value["profileSources"].is_array());
        assert!(value["profileAllocations"].is_array());
    }
    if field == "sitePolicies" {
        assert!(value["sitePolicySources"].is_array());
    }
    let records = value[field].as_array().unwrap_or_else(|| {
        panic!("{label} missing {field} array");
    });
    assert_eq!(
        value["count"].as_u64().unwrap(),
        records.len() as u64,
        "{label} count does not match {field} length"
    );
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
    pub profile_seeding_handoffs: BTreeMap<String, ProfileSeedingHandoffRecord>,
    #[serde(skip)]
    pub entity_sources: ServiceEntitySources,
}

impl ServiceState {
    pub fn mark_persisted_entity_sources(&mut self) {
        for id in self.profiles.keys() {
            self.entity_sources
                .profiles
                .entry(id.clone())
                .or_insert(ServiceEntitySource::PersistedState);
        }
        for id in self.site_policies.keys() {
            self.entity_sources
                .site_policies
                .entry(id.clone())
                .or_insert(ServiceEntitySource::PersistedState);
        }
    }

    pub fn mark_config_entity_sources(&mut self) {
        for id in self.profiles.keys() {
            self.entity_sources
                .profiles
                .insert(id.clone(), ServiceEntitySource::Config);
        }
        for id in self.site_policies.keys() {
            self.entity_sources
                .site_policies
                .insert(id.clone(), ServiceEntitySource::Config);
        }
    }

    pub fn profile_source(&self, id: &str) -> Option<ServiceEntitySource> {
        self.entity_sources.profiles.get(id).copied()
    }

    pub fn mark_runtime_observed_profile_source(&mut self, id: &str) {
        self.entity_sources
            .profiles
            .entry(id.to_string())
            .or_insert(ServiceEntitySource::RuntimeObserved);
    }

    pub fn site_policy_source(&self, id: &str) -> Option<ServiceEntitySource> {
        self.entity_sources.site_policies.get(id).copied()
    }

    pub fn remove_builtin_entity_defaults_for_persistence(&mut self) {
        let builtin_ids = self
            .entity_sources
            .site_policies
            .iter()
            .filter(|(_id, source)| **source == ServiceEntitySource::Builtin)
            .map(|(id, _source)| id.clone())
            .collect::<Vec<_>>();
        for id in builtin_ids {
            self.site_policies.remove(&id);
            self.entity_sources.site_policies.remove(&id);
        }
    }

    pub fn overlay_configured_entities(&mut self, configured: ServiceState) {
        let mut configured = configured;
        configured.mark_config_entity_sources();
        for (id, profile) in configured.profiles {
            self.profiles.insert(id.clone(), profile);
            self.entity_sources
                .profiles
                .insert(id, ServiceEntitySource::Config);
        }
        self.sessions.extend(configured.sessions);
        for (id, monitor) in configured.monitors {
            let mut monitor = monitor;
            if let Some(existing) = self.monitors.get(&id) {
                if monitor.last_checked_at.is_none() {
                    monitor.last_checked_at = existing.last_checked_at.clone();
                }
                if monitor.last_succeeded_at.is_none() {
                    monitor.last_succeeded_at = existing.last_succeeded_at.clone();
                }
                if monitor.last_failed_at.is_none() {
                    monitor.last_failed_at = existing.last_failed_at.clone();
                }
                if monitor.last_result.is_none() {
                    monitor.last_result = existing.last_result.clone();
                }
                if monitor.consecutive_failures == 0 && existing.consecutive_failures > 0 {
                    monitor.consecutive_failures = existing.consecutive_failures;
                }
                if monitor.state == MonitorState::Active && existing.state == MonitorState::Faulted
                {
                    monitor.state = MonitorState::Faulted;
                }
            }
            self.monitors.insert(id, monitor);
        }
        for (id, policy) in configured.site_policies {
            self.site_policies.insert(id.clone(), policy);
            self.entity_sources
                .site_policies
                .insert(id, ServiceEntitySource::Config);
        }
        self.providers.extend(configured.providers);
    }

    /// Add shipped site-policy defaults without overriding local policy.
    pub fn apply_builtin_site_policies(&mut self) {
        for policy in builtin_site_policies() {
            let id = policy.id.clone();
            if self.site_policies.contains_key(&id) {
                continue;
            }
            self.site_policies.insert(id.clone(), policy);
            self.entity_sources
                .site_policies
                .insert(id, ServiceEntitySource::Builtin);
        }
    }

    /// Refresh profile target-readiness rows from retained service policy.
    pub fn refresh_profile_readiness(&mut self) {
        self.apply_builtin_site_policies();
        let site_policies = self.site_policies.clone();
        for profile in self.profiles.values_mut() {
            profile.target_readiness = derive_profile_target_readiness(profile, &site_policies);
        }
    }

    /// Refresh bounded derived collections before persistence or API exposure.
    pub fn refresh_derived_views(&mut self) {
        self.refresh_profile_readiness();
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

/// Backend-owned allocation summary for one profile.
///
/// This is derived from service state at read time so API, MCP, CLI, and UI
/// consumers share the same profile/session coordination model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceProfileAllocation {
    pub profile_id: String,
    pub profile_name: String,
    pub allocation: ProfileAllocationPolicy,
    pub keyring: ProfileKeyringPolicy,
    pub target_service_ids: Vec<String>,
    pub authenticated_service_ids: Vec<String>,
    pub target_readiness: Vec<ProfileTargetReadiness>,
    pub shared_service_ids: Vec<String>,
    pub holder_session_ids: Vec<String>,
    pub holder_count: usize,
    pub exclusive_holder_session_ids: Vec<String>,
    pub waiting_job_ids: Vec<String>,
    pub waiting_job_count: usize,
    pub conflict_session_ids: Vec<String>,
    pub lease_state: String,
    pub recommended_action: String,
    pub service_names: Vec<String>,
    pub agent_names: Vec<String>,
    pub task_names: Vec<String>,
    pub browser_ids: Vec<String>,
    pub tab_ids: Vec<String>,
}

/// Return the service-owned profile allocation view sorted by profile id.
pub fn service_profile_allocations(service_state: &ServiceState) -> Vec<ServiceProfileAllocation> {
    let mut profile_ids = service_state
        .profiles
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    for session in service_state.sessions.values() {
        if let Some(profile_id) = session.profile_id.as_deref().filter(|id| !id.is_empty()) {
            profile_ids.insert(profile_id.to_string());
        }
    }
    for browser in service_state.browsers.values() {
        if let Some(profile_id) = browser.profile_id.as_deref().filter(|id| !id.is_empty()) {
            profile_ids.insert(profile_id.to_string());
        }
    }
    for job in service_state.jobs.values() {
        if job.state == JobState::WaitingProfileLease {
            if let Some(profile_id) = waiting_profile_lease_profile_id(job) {
                profile_ids.insert(profile_id.to_string());
            }
        }
    }

    profile_ids
        .into_iter()
        .map(|profile_id| service_profile_allocation(service_state, &profile_id))
        .collect()
}

fn service_profile_allocation(
    service_state: &ServiceState,
    profile_id: &str,
) -> ServiceProfileAllocation {
    let profile = service_state.profiles.get(profile_id);
    let mut holder_session_ids = BTreeSet::new();
    let mut exclusive_holder_session_ids = BTreeSet::new();
    let mut waiting_job_ids = BTreeSet::new();
    let mut conflict_session_ids = BTreeSet::new();
    let mut service_names = BTreeSet::new();
    let mut agent_names = BTreeSet::new();
    let mut task_names = BTreeSet::new();
    let mut browser_ids = BTreeSet::new();
    let mut tab_ids = BTreeSet::new();

    for session in service_state.sessions.values() {
        if session.profile_id.as_deref() != Some(profile_id) || is_inactive_lease(session.lease) {
            continue;
        }
        holder_session_ids.insert(session.id.clone());
        if matches!(
            session.lease,
            LeaseState::Exclusive | LeaseState::HumanTakeover
        ) {
            exclusive_holder_session_ids.insert(session.id.clone());
        }
        insert_non_empty(&mut service_names, session.service_name.as_deref());
        insert_non_empty(&mut agent_names, session.agent_name.as_deref());
        insert_non_empty(&mut task_names, session.task_name.as_deref());
        for browser_id in &session.browser_ids {
            insert_non_empty(&mut browser_ids, Some(browser_id));
        }
        for tab_id in &session.tab_ids {
            insert_non_empty(&mut tab_ids, Some(tab_id));
        }
        for conflict_session_id in &session.profile_lease_conflict_session_ids {
            insert_non_empty(&mut conflict_session_ids, Some(conflict_session_id));
        }
    }

    for browser in service_state.browsers.values() {
        if browser.profile_id.as_deref() == Some(profile_id) {
            insert_non_empty(&mut browser_ids, Some(browser.id.as_str()));
            for session_id in &browser.active_session_ids {
                insert_non_empty(&mut holder_session_ids, Some(session_id));
            }
        }
    }

    for tab in service_state.tabs.values() {
        let references_profile = tab
            .session_id
            .as_ref()
            .and_then(|session_id| service_state.sessions.get(session_id))
            .and_then(|session| session.profile_id.as_deref())
            == Some(profile_id)
            || tab
                .owner_session_id
                .as_ref()
                .and_then(|session_id| service_state.sessions.get(session_id))
                .and_then(|session| session.profile_id.as_deref())
                == Some(profile_id);
        if references_profile {
            insert_non_empty(&mut tab_ids, Some(tab.id.as_str()));
        }
    }

    for job in service_state.jobs.values() {
        if job.state != JobState::WaitingProfileLease
            || waiting_profile_lease_profile_id(job) != Some(profile_id)
        {
            continue;
        }
        insert_non_empty(&mut waiting_job_ids, Some(job.id.as_str()));
        insert_non_empty(&mut service_names, job.service_name.as_deref());
        insert_non_empty(&mut agent_names, job.agent_name.as_deref());
        insert_non_empty(&mut task_names, job.task_name.as_deref());
        for conflict_session_id in waiting_profile_lease_conflict_session_ids(job) {
            insert_non_empty(&mut conflict_session_ids, Some(conflict_session_id));
        }
    }

    let holder_session_ids = holder_session_ids.into_iter().collect::<Vec<_>>();
    let exclusive_holder_session_ids = exclusive_holder_session_ids.into_iter().collect::<Vec<_>>();
    let waiting_job_ids = waiting_job_ids.into_iter().collect::<Vec<_>>();
    let lease_state = profile_allocation_lease_state(
        !holder_session_ids.is_empty(),
        !exclusive_holder_session_ids.is_empty(),
        !waiting_job_ids.is_empty(),
    );
    let recommended_action = profile_allocation_recommended_action(lease_state);

    ServiceProfileAllocation {
        profile_id: profile_id.to_string(),
        profile_name: profile
            .map(|profile| profile.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| profile_id.to_string()),
        allocation: profile
            .map(|profile| profile.allocation)
            .unwrap_or_default(),
        keyring: profile.map(|profile| profile.keyring).unwrap_or_default(),
        target_service_ids: profile
            .map(|profile| sorted_strings(profile.target_service_ids.iter()))
            .unwrap_or_default(),
        authenticated_service_ids: profile
            .map(|profile| sorted_strings(profile.authenticated_service_ids.iter()))
            .unwrap_or_default(),
        target_readiness: profile
            .map(|profile| profile.target_readiness.clone())
            .unwrap_or_default(),
        shared_service_ids: profile
            .map(|profile| sorted_strings(profile.shared_service_ids.iter()))
            .unwrap_or_default(),
        holder_count: holder_session_ids.len(),
        waiting_job_count: waiting_job_ids.len(),
        holder_session_ids,
        exclusive_holder_session_ids,
        waiting_job_ids,
        conflict_session_ids: conflict_session_ids.into_iter().collect(),
        lease_state: lease_state.to_string(),
        recommended_action: recommended_action.to_string(),
        service_names: service_names.into_iter().collect(),
        agent_names: agent_names.into_iter().collect(),
        task_names: task_names.into_iter().collect(),
        browser_ids: browser_ids.into_iter().collect(),
        tab_ids: tab_ids.into_iter().collect(),
    }
}

pub fn service_profile_sources(service_state: &ServiceState) -> Vec<ProfileSourceRecord> {
    service_state
        .profiles
        .keys()
        .map(|id| {
            let source = service_state
                .profile_source(id)
                .unwrap_or(ServiceEntitySource::PersistedState);
            ProfileSourceRecord {
                id: id.clone(),
                source: source.as_str().to_string(),
                overrideable: source.overrideable(),
                precedence: vec![
                    "config".to_string(),
                    "runtime_observed".to_string(),
                    "persisted_state".to_string(),
                ],
            }
        })
        .collect()
}

fn derive_profile_target_readiness(
    profile: &BrowserProfile,
    site_policies: &BTreeMap<String, SitePolicy>,
) -> Vec<ProfileTargetReadiness> {
    let explicit_readiness = profile
        .target_readiness
        .iter()
        .filter(|row| !row.target_service_id.is_empty())
        .filter(|row| row_has_explicit_freshness_evidence(row))
        .map(|row| (row.target_service_id.clone(), row.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut target_service_ids = profile
        .target_service_ids
        .iter()
        .chain(profile.authenticated_service_ids.iter())
        .chain(explicit_readiness.keys())
        .filter(|target| !target.is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();

    for site_policy_id in &profile.site_policy_ids {
        if let Some(policy) = site_policies.get(site_policy_id) {
            if !policy.id.is_empty() {
                target_service_ids.insert(policy.id.clone());
            }
        }
    }

    target_service_ids
        .into_iter()
        .map(|target_service_id| {
            let derived =
                derive_target_readiness_for_profile(profile, site_policies, &target_service_id);
            if let Some(explicit) = explicit_readiness.get(&target_service_id) {
                normalize_explicit_target_readiness(explicit, derived)
            } else {
                derived
            }
        })
        .collect()
}

fn row_has_explicit_freshness_evidence(row: &ProfileTargetReadiness) -> bool {
    matches!(
        row.state,
        ProfileReadinessState::Fresh
            | ProfileReadinessState::Stale
            | ProfileReadinessState::BlockedByAttachedDevtools
    ) || row.last_verified_at.is_some()
        || row.freshness_expires_at.is_some()
}

fn normalize_explicit_target_readiness(
    explicit: &ProfileTargetReadiness,
    derived: ProfileTargetReadiness,
) -> ProfileTargetReadiness {
    let recommended_action = if explicit.recommended_action.is_empty() {
        match explicit.state {
            ProfileReadinessState::Fresh => "use_profile",
            ProfileReadinessState::Stale => "probe_target_auth_or_reseed_if_needed",
            ProfileReadinessState::BlockedByAttachedDevtools => {
                "close_attached_devtools_then_verify_profile"
            }
            ProfileReadinessState::NeedsManualSeeding => {
                "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
            }
            ProfileReadinessState::SeededUnknownFreshness => {
                "probe_target_auth_or_reuse_if_acceptable"
            }
            ProfileReadinessState::Unknown => "verify_or_seed_profile_before_authenticated_work",
        }
        .to_string()
    } else {
        explicit.recommended_action.clone()
    };
    let seeding_mode = if explicit.manual_seeding_required
        || matches!(explicit.state, ProfileReadinessState::NeedsManualSeeding)
    {
        ProfileSeedingMode::DetachedHeadedNoCdp
    } else {
        explicit.seeding_mode
    };
    let preferred_keyring = explicit
        .preferred_keyring
        .or(derived.preferred_keyring)
        .or_else(|| {
            matches!(seeding_mode, ProfileSeedingMode::DetachedHeadedNoCdp)
                .then_some(ProfileKeyringPolicy::BasicPasswordStore)
        });
    let setup_scopes = if explicit.setup_scopes.is_empty() {
        derived.setup_scopes
    } else {
        explicit.setup_scopes.clone()
    };

    ProfileTargetReadiness {
        target_service_id: explicit.target_service_id.clone(),
        login_id: explicit.login_id.clone().or(derived.login_id),
        state: explicit.state,
        manual_seeding_required: matches!(
            explicit.state,
            ProfileReadinessState::NeedsManualSeeding
        ) || (explicit.manual_seeding_required
            && !matches!(explicit.state, ProfileReadinessState::Fresh)),
        evidence: if explicit.evidence.is_empty() {
            derived.evidence
        } else {
            explicit.evidence.clone()
        },
        recommended_action,
        seeding_mode,
        cdp_attachment_allowed_during_seeding: matches!(
            seeding_mode,
            ProfileSeedingMode::AttachableOk
        ),
        preferred_keyring,
        setup_scopes,
        last_verified_at: explicit
            .last_verified_at
            .clone()
            .or(derived.last_verified_at),
        freshness_expires_at: explicit
            .freshness_expires_at
            .clone()
            .or(derived.freshness_expires_at),
    }
}

pub(crate) fn builtin_site_policy(id: &str) -> Option<SitePolicy> {
    builtin_site_policies()
        .into_iter()
        .find(|policy| policy.id == id)
}

pub fn service_site_policy_sources(service_state: &ServiceState) -> Vec<SitePolicySourceRecord> {
    service_state
        .site_policies
        .keys()
        .map(|id| {
            let source = service_state
                .site_policy_source(id)
                .unwrap_or(ServiceEntitySource::PersistedState);
            SitePolicySourceRecord {
                id: id.clone(),
                source: source.as_str().to_string(),
                overrideable: source.overrideable(),
                precedence: vec![
                    "config".to_string(),
                    "persisted_state".to_string(),
                    "builtin".to_string(),
                ],
            }
        })
        .collect()
}

pub fn service_profile_seeding_handoff(
    service_state: &ServiceState,
    profile_id: &str,
    target_service_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    let profile = service_state
        .profiles
        .get(profile_id)
        .ok_or_else(|| format!("Profile seeding handoff not found: {profile_id}"))?;
    let readiness = profile
        .target_readiness
        .iter()
        .find(|row| {
            target_service_id
                .map(|target| row.target_service_id == target)
                .unwrap_or(row.manual_seeding_required)
        })
        .or_else(|| profile.target_readiness.first())
        .ok_or_else(|| format!("Profile has no target readiness rows: {profile_id}"))?;
    let target = readiness.target_service_id.as_str();
    let lifecycle = service_state
        .profile_seeding_handoffs
        .get(&profile_seeding_handoff_id(profile_id, target))
        .cloned()
        .unwrap_or_else(|| ProfileSeedingHandoffRecord {
            id: profile_seeding_handoff_id(profile_id, target),
            profile_id: profile_id.to_string(),
            target_service_id: target.to_string(),
            state: if readiness.manual_seeding_required {
                ProfileSeedingHandoffState::NeedsManualSeeding
            } else {
                ProfileSeedingHandoffState::NotRequired
            },
            ..ProfileSeedingHandoffRecord::default()
        });
    let policy = service_state.site_policies.get(target);
    let url = policy
        .map(|policy| policy.origin_pattern.as_str())
        .filter(|origin| origin.starts_with("http://") || origin.starts_with("https://"))
        .unwrap_or_else(|| default_seeding_url(target));
    let command = format!("agent-browser --runtime-profile {profile_id} runtime login {url}");
    let mut warnings = Vec::new();
    if readiness.seeding_mode == ProfileSeedingMode::DetachedHeadedNoCdp {
        warnings.push(
            "Do not add --attachable or any remote debugging/CDP flag during first seeding."
                .to_string(),
        );
    }
    if readiness.preferred_keyring != Some(ProfileKeyringPolicy::BasicPasswordStore) {
        warnings.push("Consider basic_password_store for managed profiles so OS keyring modals do not block unattended workflows.".to_string());
    }
    let intervention_severity = lifecycle.state.intervention_severity();
    let intervention_title = if lifecycle.state == ProfileSeedingHandoffState::NotRequired {
        format!("Profile {profile_id} does not require seeding for {target}")
    } else {
        format!("Seed profile {profile_id} for {target}")
    };
    let intervention_message = lifecycle.state.intervention_message();
    let blocks_profile_lease = lifecycle.state.blocks_profile_lease();

    Ok(serde_json::json!({
        "profileId": profile_id,
        "profileName": profile.name.clone(),
        "targetServiceId": readiness.target_service_id.clone(),
        "loginId": readiness.login_id.clone(),
        "manualSeedingRequired": readiness.manual_seeding_required,
        "seedingMode": readiness.seeding_mode,
        "cdpAttachmentAllowedDuringSeeding": readiness.cdp_attachment_allowed_during_seeding,
        "preferredKeyring": readiness.preferred_keyring,
        "setupScopes": readiness.setup_scopes.clone(),
        "recommendedAction": readiness.recommended_action.clone(),
        "url": url,
        "command": command,
        "lifecycle": lifecycle,
        "operatorSteps": [
            "Run the command exactly as shown.",
            "Complete sign-in and any requested sync, passkey, or browser plugin setup in the headed browser.",
            "Close Chrome after seeding is complete.",
            "Request future tabs through service-owned agent-browser automation so CDP attaches only after seeding."
        ],
        "operatorIntervention": {
            "state": lifecycle.state,
            "severity": intervention_severity,
            "title": intervention_title,
            "message": intervention_message,
            "ownedBy": "agent-browser",
            "defaultChannels": ["api", "mcp", "dashboard"],
            "optionalChannels": ["desktop", "webhook", "agent"],
            "desktopPopupPolicy": "optional_policy_controlled",
            "blocksProfileLease": blocks_profile_lease,
            "completionSignals": [
                "seeding_browser_closed",
                "operator_or_agent_declared_complete",
                "post_seeding_probe_records_freshness"
            ],
            "actions": [
                {
                    "id": "run_detached_seeding_command",
                    "label": "Run detached seeding command",
                    "kind": "operator_command",
                    "safety": "safe",
                    "command": command,
                    "description": "Launch headed Chrome without CDP or DevTools so first sign-in and setup can complete."
                },
                {
                    "id": "close_seeded_browser",
                    "label": "Close seeding browser when finished",
                    "kind": "operator_instruction",
                    "safety": "safe",
                    "description": "Close Chrome after sign-in, sync, passkey, and plugin setup are complete so agent-browser can later attach."
                },
                {
                    "id": "retry_access_plan_after_close",
                    "label": "Retry the access plan after close",
                    "kind": "service_request",
                    "safety": "safe",
                    "description": "Ask agent-browser for the same access plan again after the seeding browser closes."
                },
                {
                    "id": "force_close_seeded_browser",
                    "label": "Force close only after operator approval",
                    "kind": "operator_remedy",
                    "safety": "danger",
                    "description": "Force close can lose setup progress or corrupt profile state; reserve it for abandoned seeding browsers."
                }
            ]
        },
        "warnings": warnings,
    }))
}

fn default_seeding_url(target_service_id: &str) -> &'static str {
    if target_is_google_signin(target_service_id) {
        "https://accounts.google.com"
    } else {
        "about:blank"
    }
}

fn builtin_site_policies() -> Vec<SitePolicy> {
    vec![
        SitePolicy {
            id: "canva".to_string(),
            origin_pattern: "https://www.canva.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            requires_cdp_free: true,
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(700),
                jitter_ms: Some(600),
                cooldown_ms: Some(3_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            manual_login_preferred: true,
            profile_required: true,
            challenge_policy: ChallengePolicy::ManualOnly,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: Some(
                "Canva can reject sessions when a DevTools port is attached; prefer headed Chrome without CDP."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "google".to_string(),
            origin_pattern: "https://accounts.google.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(500),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            manual_login_preferred: true,
            profile_required: true,
            challenge_policy: ChallengePolicy::ManualOnly,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: Some(
                "Google first sign-in should use detached headed Chrome before attachable automation."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "gmail".to_string(),
            origin_pattern: "https://mail.google.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(500),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            manual_login_preferred: true,
            profile_required: true,
            challenge_policy: ChallengePolicy::ManualOnly,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: Some(
                "Gmail inherits Google sign-in seeding and should prefer a persistent headed profile."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "microsoft".to_string(),
            origin_pattern: "https://login.microsoftonline.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(450),
                jitter_ms: Some(300),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(2),
            },
            manual_login_preferred: true,
            profile_required: true,
            challenge_policy: ChallengePolicy::ProviderAllowed,
            allowed_challenge_providers: vec![
                "manual".to_string(),
                "totp".to_string(),
                "sms".to_string(),
                "email".to_string(),
            ],
            notes: Some(
                "Microsoft sign-in should prefer persistent headed profiles with conservative pacing."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
    ]
}

fn derive_target_readiness_for_profile(
    profile: &BrowserProfile,
    site_policies: &BTreeMap<String, SitePolicy>,
    target_service_id: &str,
) -> ProfileTargetReadiness {
    let authenticated = profile
        .authenticated_service_ids
        .iter()
        .any(|id| id == target_service_id);
    let manual_seeding_required = !authenticated
        && target_requires_detached_manual_seeding(profile, site_policies, target_service_id);
    let (state, evidence, recommended_action, seeding_mode, preferred_keyring, setup_scopes) =
        if authenticated {
            (
                ProfileReadinessState::SeededUnknownFreshness,
                "profile_authenticated_service_hint".to_string(),
                "probe_target_auth_or_reuse_if_acceptable".to_string(),
                ProfileSeedingMode::NotRequired,
                None,
                Vec::new(),
            )
        } else if manual_seeding_required {
            (
                ProfileReadinessState::NeedsManualSeeding,
                "manual_seed_required_without_authenticated_hint".to_string(),
                "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
                    .to_string(),
                ProfileSeedingMode::DetachedHeadedNoCdp,
                Some(ProfileKeyringPolicy::BasicPasswordStore),
                profile_seeding_setup_scopes(target_service_id),
            )
        } else {
            (
                ProfileReadinessState::Unknown,
                "no_authenticated_service_hint".to_string(),
                "verify_or_seed_profile_before_authenticated_work".to_string(),
                ProfileSeedingMode::AttachableOk,
                Some(profile.keyring),
                vec!["signin".to_string()],
            )
        };

    ProfileTargetReadiness {
        target_service_id: target_service_id.to_string(),
        login_id: None,
        state,
        manual_seeding_required,
        evidence,
        recommended_action,
        seeding_mode,
        cdp_attachment_allowed_during_seeding: matches!(
            seeding_mode,
            ProfileSeedingMode::AttachableOk
        ),
        preferred_keyring,
        setup_scopes,
        last_verified_at: None,
        freshness_expires_at: None,
    }
}

fn target_requires_detached_manual_seeding(
    profile: &BrowserProfile,
    site_policies: &BTreeMap<String, SitePolicy>,
    target_service_id: &str,
) -> bool {
    profile.manual_login_preferred
        || target_is_google_signin(target_service_id)
        || site_policies
            .get(target_service_id)
            .map(|policy| policy.manual_login_preferred)
            .unwrap_or(false)
}

fn target_is_google_signin(target_service_id: &str) -> bool {
    let normalized = target_service_id.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "google" | "gmail" | "google-login" | "google_signin" | "google-signin"
    )
}

fn profile_seeding_setup_scopes(target_service_id: &str) -> Vec<String> {
    if target_is_google_signin(target_service_id) {
        vec![
            "signin".to_string(),
            "chrome_sync".to_string(),
            "passkeys".to_string(),
            "browser_plugins".to_string(),
        ]
    } else {
        vec!["signin".to_string()]
    }
}

fn is_inactive_lease(lease: LeaseState) -> bool {
    matches!(lease, LeaseState::Released | LeaseState::Expired)
}

fn waiting_profile_lease_profile_id(job: &ServiceJob) -> Option<&str> {
    job.result
        .as_ref()
        .and_then(|result| result.get("profileId"))
        .and_then(|profile_id| profile_id.as_str())
        .filter(|profile_id| !profile_id.is_empty())
}

fn waiting_profile_lease_conflict_session_ids(job: &ServiceJob) -> impl Iterator<Item = &str> {
    job.result
        .as_ref()
        .and_then(|result| result.get("conflictSessionIds"))
        .and_then(|conflicts| conflicts.as_array())
        .into_iter()
        .flatten()
        .filter_map(|conflict| conflict.as_str())
        .filter(|conflict| !conflict.is_empty())
}

fn profile_allocation_lease_state(
    has_holders: bool,
    has_exclusive_holders: bool,
    has_waiting_jobs: bool,
) -> &'static str {
    match (has_holders, has_exclusive_holders, has_waiting_jobs) {
        (_, true, true) => "conflicted",
        (_, _, true) => "waiting",
        (_, true, false) => "exclusive",
        (true, false, false) => "shared",
        (false, false, false) => "available",
    }
}

fn profile_allocation_recommended_action(lease_state: &str) -> &'static str {
    match lease_state {
        "conflicted" => "release_holder_or_redirect_waiting_jobs",
        "waiting" => "inspect_waiting_jobs",
        "exclusive" => "reuse_holder_or_release_profile",
        "shared" => "shared_profile_in_use",
        _ => "available",
    }
}

fn insert_non_empty(values: &mut BTreeSet<String>, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        values.insert(value.to_string());
    }
}

fn sorted_strings<'a>(values: impl Iterator<Item = &'a String>) -> Vec<String> {
    values
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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
    pub monitor_id: Option<String>,
    pub monitor_target: Option<serde_json::Value>,
    pub monitor_result: Option<String>,
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
    MonitorAttention,
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
    ProfileLeaseWaitStarted,
    ProfileLeaseWaitEnded,
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
        let monitor_id = service_event_monitor_id(event);
        let key = service_event_incident_id(event)
            .or(browser_id.clone())
            .unwrap_or_else(|| "service".to_string());
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                monitor_id: monitor_id.clone(),
                monitor_target: service_event_monitor_target(event),
                monitor_result: service_event_monitor_result(event),
                label: incident_label(state, browser_id.as_deref(), monitor_id.as_deref()),
                state: classify_incident_state(
                    browser_id.is_some(),
                    monitor_id.is_some(),
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
            incident.monitor_id = monitor_id.clone().or(incident.monitor_id.clone());
            incident.monitor_target =
                service_event_monitor_target(event).or_else(|| incident.monitor_target.clone());
            incident.monitor_result =
                service_event_monitor_result(event).or_else(|| incident.monitor_result.clone());
            incident.state = classify_incident_state(
                incident.browser_id.is_some(),
                incident.monitor_id.is_some(),
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

        if incident_is_newer(timestamp, &incident.latest_timestamp) {
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
                .cmp(job_or_event_timestamp(&event_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        incident.job_ids.sort_by(|left, right| {
            job_or_event_timestamp(&job_timestamps, left)
                .cmp(job_or_event_timestamp(&job_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        if incident.label.is_empty() {
            incident.label = incident_label(
                state,
                incident.browser_id.as_deref(),
                incident.monitor_id.as_deref(),
            );
        }
        if incident.current_health.is_none() {
            incident.current_health = incident
                .browser_id
                .as_ref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .map(|browser| browser.health);
        }
        if incident.monitor_id.is_some() {
            incident.state = ServiceIncidentState::Active;
        } else if let Some(browser_id) = incident.browser_id.as_ref() {
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
        _ if incident.monitor_id.is_some() => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::MonitorAttention,
            "Inspect the failed monitor target and last result; fix the target, refresh login state, pause the monitor, or reset reviewed failures before rerunning.",
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
        | ServiceEventKind::TabLifecycleChanged
        | ServiceEventKind::ProfileLeaseWaitStarted
        | ServiceEventKind::ProfileLeaseWaitEnded => false,
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

fn service_event_monitor_id(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("monitorId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn service_event_monitor_target(event: &ServiceEvent) -> Option<serde_json::Value> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("target"))
        .cloned()
}

fn service_event_monitor_result(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("result"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn incident_label(
    state: &ServiceState,
    browser_id: Option<&str>,
    monitor_id: Option<&str>,
) -> String {
    if let Some(browser_id) = browser_id {
        return browser_id.to_string();
    }
    if let Some(monitor_id) = monitor_id {
        return state
            .monitors
            .get(monitor_id)
            .map(|monitor| {
                if monitor.name.trim().is_empty() {
                    format!("Monitor {}", monitor_id)
                } else {
                    format!("Monitor {}", monitor.name)
                }
            })
            .unwrap_or_else(|| format!("Monitor {}", monitor_id));
    }
    "Service incidents".to_string()
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
        ServiceEventKind::ProfileLeaseWaitStarted => "profile_lease_wait_started",
        ServiceEventKind::ProfileLeaseWaitEnded => "profile_lease_wait_ended",
        ServiceEventKind::ReconciliationError => "reconciliation_error",
        ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
        ServiceEventKind::IncidentResolved => "incident_resolved",
    }
}

fn classify_incident_state(
    has_browser: bool,
    has_monitor: bool,
    current_health: Option<BrowserHealth>,
    kind: ServiceEventKind,
) -> ServiceIncidentState {
    if has_monitor {
        return ServiceIncidentState::Active;
    }
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
    /// Number of retained jobs currently delayed by profile lease contention.
    pub waiting_profile_lease_job_count: usize,
    pub service_job_timeout_ms: Option<u64>,
    pub service_monitor_interval_ms: Option<u64>,
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
    /// Target sites or identity providers this profile is intended to satisfy.
    ///
    /// Examples include google, microsoft, acs, and publisher-specific login
    /// systems. These are not caller service names; they describe stored
    /// credential or login-state scope.
    pub target_service_ids: Vec<String>,
    /// Target services currently believed to have usable authenticated state.
    ///
    /// This is advisory until active auth probes can refresh it.
    pub authenticated_service_ids: Vec<String>,
    pub default_browser_host: Option<BrowserHost>,
    pub allocation: ProfileAllocationPolicy,
    pub keyring: ProfileKeyringPolicy,
    pub shared_service_ids: Vec<String>,
    pub credential_provider_ids: Vec<String>,
    pub manual_login_preferred: bool,
    /// No-launch readiness rows for target services or login identities.
    ///
    /// These rows are derived from retained profile and site-policy state. They
    /// do not prove live authentication until a future probe records freshness
    /// evidence.
    pub target_readiness: Vec<ProfileTargetReadiness>,
    pub persistent: bool,
    pub tags: Vec<String>,
}

/// No-launch service view of whether a profile can satisfy a target identity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileTargetReadiness {
    pub target_service_id: String,
    pub login_id: Option<String>,
    pub state: ProfileReadinessState,
    pub manual_seeding_required: bool,
    pub evidence: String,
    pub recommended_action: String,
    pub seeding_mode: ProfileSeedingMode,
    pub cdp_attachment_allowed_during_seeding: bool,
    pub preferred_keyring: Option<ProfileKeyringPolicy>,
    pub setup_scopes: Vec<String>,
    pub last_verified_at: Option<String>,
    pub freshness_expires_at: Option<String>,
}

/// Persisted lifecycle for a CDP-free profile seeding handoff.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileSeedingHandoffRecord {
    pub id: String,
    pub profile_id: String,
    pub target_service_id: String,
    pub state: ProfileSeedingHandoffState,
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub expires_at: Option<String>,
    pub last_prompted_at: Option<String>,
    pub declared_complete_at: Option<String>,
    pub closed_at: Option<String>,
    pub updated_at: Option<String>,
    pub actor: Option<String>,
    pub note: Option<String>,
}

/// Lifecycle state for CDP-free profile seeding handoffs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSeedingHandoffState {
    NotRequired,
    #[default]
    NeedsManualSeeding,
    SeedingLaunchedDetached,
    SeedingWaitingForClose,
    CompletionDeclaredWaitingForClose,
    SeedingClosedUnverified,
    VerificationPending,
    Fresh,
    Failed,
    Abandoned,
}

impl ProfileSeedingHandoffState {
    pub fn intervention_severity(self) -> &'static str {
        match self {
            Self::NotRequired | Self::Fresh => "info",
            Self::SeedingLaunchedDetached
            | Self::SeedingClosedUnverified
            | Self::VerificationPending => "attention",
            Self::NeedsManualSeeding
            | Self::SeedingWaitingForClose
            | Self::CompletionDeclaredWaitingForClose => "action_required",
            Self::Failed | Self::Abandoned => "danger",
        }
    }

    pub fn intervention_message(self) -> &'static str {
        match self {
            Self::NotRequired => "No CDP-free profile seeding action is required for this target.",
            Self::NeedsManualSeeding => {
                "Launch the detached headed browser, complete setup, close Chrome, then let agent-browser verify freshness after CDP is allowed again."
            }
            Self::SeedingLaunchedDetached => {
                "The detached seeding browser has been launched. Complete setup in Chrome, then close that browser."
            }
            Self::SeedingWaitingForClose => {
                "The seeding browser is still open. Finish setup, close Chrome, extend the handoff, or abandon it."
            }
            Self::CompletionDeclaredWaitingForClose => {
                "Completion was declared, but Chrome still appears open. Close the seeding browser before attachable automation resumes."
            }
            Self::SeedingClosedUnverified => {
                "The seeding browser is closed, but authentication freshness has not been verified."
            }
            Self::VerificationPending => {
                "The profile is ready for a bounded post-seeding auth probe."
            }
            Self::Fresh => "The profile has fresh authenticated evidence for this target.",
            Self::Failed => "The seeding handoff failed. Review the operator note and retry or abandon.",
            Self::Abandoned => "The seeding handoff was abandoned. Start a new handoff before authenticated work.",
        }
    }

    pub fn blocks_profile_lease(self) -> bool {
        matches!(
            self,
            Self::NeedsManualSeeding
                | Self::SeedingLaunchedDetached
                | Self::SeedingWaitingForClose
                | Self::CompletionDeclaredWaitingForClose
        )
    }
}

pub fn profile_seeding_handoff_id(profile_id: &str, target_service_id: &str) -> String {
    format!("{profile_id}:{target_service_id}")
}

/// Browser launch posture required while a target profile is being seeded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSeedingMode {
    #[default]
    NotRequired,
    DetachedHeadedNoCdp,
    AttachableOk,
}

/// Profile readiness state for one target service or login identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileReadinessState {
    #[default]
    Unknown,
    NeedsManualSeeding,
    SeededUnknownFreshness,
    Fresh,
    Stale,
    BlockedByAttachedDevtools,
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
    /// Why this session's profile was selected, when agent-browser can infer it.
    pub profile_selection_reason: Option<ProfileSelectionReason>,
    /// Whether the selected profile is new, reused, or already leased elsewhere.
    pub profile_lease_disposition: Option<ProfileLeaseDisposition>,
    /// Other sessions currently holding an exclusive lease on the same profile.
    pub profile_lease_conflict_session_ids: Vec<String>,
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
            profile_selection_reason: None,
            profile_lease_disposition: None,
            profile_lease_conflict_session_ids: Vec::new(),
            cleanup: SessionCleanupPolicy::Detach,
            browser_ids: Vec::new(),
            tab_ids: Vec::new(),
            created_at: None,
            expires_at: None,
        }
    }
}

/// Explanation for a service session's chosen profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSelectionReason {
    /// Caller explicitly supplied a profile or runtime profile override.
    ExplicitProfile,
    /// Selected profile has authenticated state for a requested target service.
    AuthenticatedTarget,
    /// Selected profile targets a requested site or identity provider.
    TargetMatch,
    /// Selected profile was chosen by caller service allow-list fallback.
    ServiceAllowList,
}

/// Current lease relationship between a session and its selected profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileLeaseDisposition {
    /// No retained browser or exclusive session was already using the profile.
    NewBrowser,
    /// This session already had a retained browser for the selected profile.
    ReusedBrowser,
    /// Another exclusive session already references the selected profile.
    ActiveLeaseConflict,
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
    /// Target service or identity-provider hint supplied by the caller.
    pub target_service_id: Option<String>,
    /// Site hint supplied by the caller. This is treated as a target-service alias.
    pub site_id: Option<String>,
    /// Login identity hint supplied by the caller. This is treated as a target-service alias.
    pub login_id: Option<String>,
    /// Normalized target-service, site, and login identity hints used for profile selection.
    pub target_service_ids: Vec<String>,
    /// Non-blocking policy warnings for missing caller labels.
    ///
    /// Current warning values are the `SERVICE_JOB_NAMING_WARNING_VALUES`
    /// constants.
    pub naming_warnings: Vec<String>,
    /// True when `naming_warnings` is non-empty.
    pub has_naming_warning: bool,
    /// Control-plane mode required for this job. CDP-free jobs launch or manage
    /// browsers without attaching a DevTools endpoint.
    pub control_plane_mode: JobControlPlaneMode,
    /// True when this job manages process or profile lifecycle rather than a
    /// CDP-backed tab interaction.
    pub lifecycle_only: bool,
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
            target_service_id: None,
            site_id: None,
            login_id: None,
            target_service_ids: Vec::new(),
            naming_warnings: Vec::new(),
            has_naming_warning: false,
            control_plane_mode: JobControlPlaneMode::Cdp,
            lifecycle_only: false,
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
    pub last_succeeded_at: Option<String>,
    pub last_failed_at: Option<String>,
    pub last_result: Option<String>,
    pub consecutive_failures: u64,
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
            last_succeeded_at: None,
            last_failed_at: None,
            last_result: None,
            consecutive_failures: 0,
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
    /// True when this site should launch without a DevTools/CDP attachment.
    pub requires_cdp_free: bool,
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
            requires_cdp_free: false,
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

impl ServiceActor {
    /// Infer the most specific service actor available from caller labels.
    pub fn from_caller_context(service_name: Option<&str>, agent_name: Option<&str>) -> Self {
        if let Some(agent_name) = non_empty_label(agent_name) {
            return ServiceActor::Agent(agent_name.to_string());
        }
        if let Some(service_name) = non_empty_label(service_name) {
            return ServiceActor::ApiClient(service_name.to_string());
        }
        ServiceActor::System
    }

    pub fn is_system(&self) -> bool {
        matches!(self, ServiceActor::System)
    }
}

fn non_empty_label(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
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
    WaitingProfileLease,
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

/// Control-plane attachment posture for a service job.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobControlPlaneMode {
    #[default]
    Cdp,
    CdpFree,
    Service,
}

/// Monitor target variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorTarget {
    Url(String),
    Tab(String),
    SitePolicy(String),
    /// Checks retained no-launch target readiness for a login/service identity.
    ProfileReadiness(String),
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
            requires_cdp_free: true,
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
        assert_eq!(value["requiresCdpFree"], true);
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
        assert_eq!(
            schema["properties"]["targetServiceIds"]["description"],
            "Normalized target-service, site, and login identity hints used for profile selection."
        );
        assert_eq!(
            schema["properties"]["controlPlaneMode"]["enum"],
            json!(SERVICE_JOB_CONTROL_PLANE_MODE_VALUES.to_vec())
        );
        for field in [
            "id",
            "action",
            "serviceName",
            "agentName",
            "taskName",
            "namingWarnings",
            "hasNamingWarning",
            "controlPlaneMode",
            "lifecycleOnly",
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
        let monitor_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-monitor-record.v1.schema.json"
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
            session_schema["properties"]["profileSelectionReason"]["oneOf"][0]["enum"],
            json!(SERVICE_PROFILE_SELECTION_REASON_VALUES.to_vec())
        );
        assert_eq!(
            session_schema["properties"]["profileLeaseDisposition"]["oneOf"][0]["enum"],
            json!(SERVICE_PROFILE_LEASE_DISPOSITION_VALUES.to_vec())
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
            monitor_schema["properties"]["state"]["enum"],
            json!(SERVICE_MONITOR_STATE_VALUES.to_vec())
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
            &[
                "id",
                "name",
                "defaultBrowserHost",
                "allocation",
                "keyring",
                "targetReadiness",
            ],
        );
        assert_eq!(
            profile_schema["$defs"]["profileTargetReadiness"]["properties"]["state"]["enum"],
            json!(SERVICE_PROFILE_READINESS_VALUES.to_vec())
        );
        assert_schema_required_fields(&browser_schema, &["id", "profileId", "host", "health"]);
        assert_schema_required_fields(
            &session_schema,
            &[
                "id",
                "serviceName",
                "lease",
                "profileId",
                "profileSelectionReason",
                "profileLeaseDisposition",
                "profileLeaseConflictSessionIds",
                "cleanup",
            ],
        );
        assert_schema_required_fields(&tab_schema, &["id", "browserId", "sessionId", "lifecycle"]);
        assert_schema_required_fields(
            &monitor_schema,
            &[
                "id",
                "name",
                "target",
                "intervalMs",
                "state",
                "lastCheckedAt",
                "lastSucceededAt",
                "lastFailedAt",
                "lastResult",
                "consecutiveFailures",
            ],
        );
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
            profile_selection_reason: Some(ProfileSelectionReason::AuthenticatedTarget),
            profile_lease_disposition: Some(ProfileLeaseDisposition::NewBrowser),
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
        let monitor = SiteMonitor {
            id: "monitor-1".to_string(),
            name: "ACS heartbeat".to_string(),
            target: MonitorTarget::Url("https://example.com/health".to_string()),
            interval_ms: 60_000,
            state: MonitorState::Active,
            last_checked_at: Some("2026-05-07T00:00:00Z".to_string()),
            last_succeeded_at: Some("2026-05-07T00:00:00Z".to_string()),
            last_failed_at: None,
            last_result: Some("ok".to_string()),
            consecutive_failures: 0,
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
        assert_service_monitor_record_contract(&serde_json::to_value(monitor).unwrap());
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
        assert_schema_required_fields(
            &response_schema["properties"]["summary"]["properties"]["groups"]["items"],
            &[
                "escalation",
                "severity",
                "state",
                "count",
                "latestTimestamp",
                "recommendedAction",
                "incidentIds",
                "browserIds",
                "monitorIds",
                "remedyApplyCommand",
            ],
        );

        let incident = json!({
            "id": "browser-1",
            "browserId": "browser-1",
            "monitorId": null,
            "monitorTarget": null,
            "monitorResult": null,
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
                "remediesOnly": true,
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
            "targetServiceId": null,
            "siteId": null,
            "loginId": null,
            "targetServiceIds": [],
            "namingWarnings": [],
            "hasNamingWarning": false,
            "controlPlaneMode": "cdp",
            "lifecycleOnly": false,
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
        let profile_upsert_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-profile-upsert-response.v1.schema.json"
        ))
        .unwrap();
        let profile_delete_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-profile-delete-response.v1.schema.json"
        ))
        .unwrap();
        let session_upsert_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-session-upsert-response.v1.schema.json"
        ))
        .unwrap();
        let session_delete_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-session-delete-response.v1.schema.json"
        ))
        .unwrap();
        let site_policy_upsert_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-site-policy-upsert-response.v1.schema.json"
        ))
        .unwrap();
        let site_policy_delete_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-site-policy-delete-response.v1.schema.json"
        ))
        .unwrap();
        let monitor_upsert_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-monitor-upsert-response.v1.schema.json"
        ))
        .unwrap();
        let monitor_delete_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-monitor-delete-response.v1.schema.json"
        ))
        .unwrap();
        let monitor_state_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-monitor-state-response.v1.schema.json"
        ))
        .unwrap();
        let monitor_run_due_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-monitor-run-due-response.v1.schema.json"
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

        assert_schema_required_fields(&profile_upsert_schema, &["id", "profile", "upserted"]);
        assert_schema_required_fields(&profile_delete_schema, &["id", "deleted", "profile"]);
        assert_schema_required_fields(&session_upsert_schema, &["id", "session", "upserted"]);
        assert_schema_required_fields(&session_delete_schema, &["id", "deleted", "session"]);
        assert_schema_required_fields(
            &site_policy_upsert_schema,
            &["id", "sitePolicy", "upserted"],
        );
        assert_schema_required_fields(&site_policy_delete_schema, &["id", "deleted", "sitePolicy"]);
        assert_schema_required_fields(&monitor_upsert_schema, &["id", "monitor", "upserted"]);
        assert_schema_required_fields(&monitor_delete_schema, &["id", "deleted", "monitor"]);
        assert_schema_required_fields(
            &monitor_state_schema,
            &["id", "monitor", "state", "updated"],
        );
        assert_schema_required_fields(
            &monitor_run_due_schema,
            &["checked", "succeeded", "failed", "monitorIds"],
        );
        assert_schema_required_fields(&provider_upsert_schema, &["id", "provider", "upserted"]);
        assert_schema_required_fields(&provider_delete_schema, &["id", "deleted", "provider"]);

        let profile = json!({
            "id": "journal-downloader",
            "name": "Journal Downloader",
            "userDataDir": null,
            "sitePolicyIds": [],
            "targetServiceIds": ["acs"],
            "authenticatedServiceIds": [],
            "defaultBrowserHost": null,
            "allocation": "per_service",
            "keyring": "basic_password_store",
            "sharedServiceIds": [],
            "credentialProviderIds": [],
            "manualLoginPreferred": false,
            "targetReadiness": [],
            "persistent": true,
            "tags": [],
        });
        let session = json!({
            "id": "journal-run",
            "serviceName": "JournalDownloader",
            "agentName": "codex",
            "taskName": "probeACSwebsite",
            "owner": "system",
            "lease": "exclusive",
            "profileId": "journal-downloader",
            "profileSelectionReason": "authenticated_target",
            "profileLeaseDisposition": "new_browser",
            "profileLeaseConflictSessionIds": [],
            "cleanup": "close_browser",
            "browserIds": [],
            "tabIds": [],
            "createdAt": null,
            "expiresAt": null,
        });
        let site_policy = json!({
            "id": "google",
            "originPattern": "https://accounts.google.com",
            "browserHost": null,
            "viewStream": null,
            "controlInput": null,
            "requiresCdpFree": false,
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
        let monitor = json!({
            "id": "google-login-freshness",
            "name": "Google login freshness",
            "target": {"site_policy": "google"},
            "intervalMs": 60000,
            "state": "paused",
            "lastCheckedAt": null,
            "lastSucceededAt": null,
            "lastFailedAt": null,
            "lastResult": null,
            "consecutiveFailures": 0,
        });

        assert_service_profile_upsert_response_contract(&json!({
            "id": "journal-downloader",
            "profile": profile.clone(),
            "upserted": true,
        }));
        assert_service_profile_delete_response_contract(&json!({
            "id": "journal-downloader",
            "deleted": true,
            "profile": profile,
        }));
        assert_service_session_upsert_response_contract(&json!({
            "id": "journal-run",
            "session": session.clone(),
            "upserted": true,
        }));
        assert_service_session_delete_response_contract(&json!({
            "id": "journal-run",
            "deleted": true,
            "session": session,
        }));
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
        assert_service_monitor_upsert_response_contract(&json!({
            "id": "google-login-freshness",
            "monitor": monitor.clone(),
            "upserted": true,
        }));
        assert_service_monitor_delete_response_contract(&json!({
            "id": "google-login-freshness",
            "deleted": true,
            "monitor": monitor.clone(),
        }));
        assert_service_monitor_state_response_contract(&json!({
            "id": "google-login-freshness",
            "monitor": monitor.clone(),
            "state": "paused",
            "updated": true,
        }));
        assert_service_monitor_run_due_response_contract(&json!({
            "checked": 1,
            "succeeded": 0,
            "failed": 1,
            "monitorIds": ["google-login-freshness"],
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
    fn service_operator_mutation_response_contracts_match_wire_shape() {
        let job_cancel_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-job-cancel-response.v1.schema.json"
        ))
        .unwrap();
        let browser_retry_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-browser-retry-response.v1.schema.json"
        ))
        .unwrap();
        let incident_acknowledge_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-incident-acknowledge-response.v1.schema.json"
        ))
        .unwrap();
        let incident_resolve_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-incident-resolve-response.v1.schema.json"
        ))
        .unwrap();
        let monitor_triage_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-monitor-triage-response.v1.schema.json"
        ))
        .unwrap();
        let remedies_apply_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-remedies-apply-response.v1.schema.json"
        ))
        .unwrap();

        assert_schema_required_fields(&job_cancel_schema, &["cancelled", "job"]);
        assert_schema_required_fields(
            &browser_retry_schema,
            &["retryEnabled", "browser", "incident"],
        );
        assert_schema_required_fields(&incident_acknowledge_schema, &["acknowledged", "incident"]);
        assert_schema_required_fields(&incident_resolve_schema, &["resolved", "incident"]);
        assert_schema_required_fields(
            &monitor_triage_schema,
            &[
                "id",
                "monitor",
                "state",
                "updated",
                "resetFailures",
                "acknowledged",
                "incident",
            ],
        );
        assert_schema_required_fields(
            &remedies_apply_schema,
            &[
                "applied",
                "escalation",
                "count",
                "monitorIds",
                "monitorResults",
                "browserIds",
                "browserResults",
            ],
        );

        let job = json!({
            "id": "job-queued",
            "action": "navigate",
            "serviceName": "JournalDownloader",
            "agentName": "codex",
            "taskName": "probeACSwebsite",
            "targetServiceId": null,
            "siteId": null,
            "loginId": null,
            "targetServiceIds": [],
            "namingWarnings": [],
            "hasNamingWarning": false,
            "target": "browser",
            "owner": "agent",
            "state": "cancelled",
            "priority": "normal",
            "submittedAt": "2026-04-22T00:00:00Z",
            "startedAt": null,
            "completedAt": "2026-04-22T00:00:01Z",
            "timeoutMs": null,
            "result": { "success": false, "cancelled": true },
            "error": "stale",
        });
        let browser = json!({
            "id": "session:retry-session",
            "profileId": "work",
            "host": "local_headless",
            "pid": null,
            "cdpEndpoint": null,
            "viewStreams": [],
            "health": "process_exited",
            "lastError": "Browser retry requested by operator",
            "activeSessionIds": ["retry-session"],
            "lastHealthObservation": null,
        });
        let incident = json!({
            "id": "session:retry-session",
            "browserId": "session:retry-session",
            "monitorId": null,
            "monitorTarget": null,
            "monitorResult": null,
            "label": "Browser session:retry-session",
            "state": "active",
            "severity": "error",
            "escalation": "browser_recovery",
            "recommendedAction": "Inspect browser health and retry or relaunch the affected browser if needed.",
            "acknowledgedAt": null,
            "acknowledgedBy": null,
            "acknowledgementNote": null,
            "resolvedAt": null,
            "resolvedBy": null,
            "resolutionNote": null,
            "latestTimestamp": "2026-04-22T00:00:00Z",
            "latestMessage": "Browser faulted",
            "latestKind": "browser_health_changed",
            "currentHealth": "faulted",
            "eventIds": ["event-faulted"],
            "jobIds": [],
        });

        assert_service_job_cancel_response_contract(&json!({
            "cancelled": true,
            "job": job,
        }));
        assert_service_browser_retry_response_contract(&json!({
            "retryEnabled": true,
            "browser": browser,
            "incident": incident.clone(),
        }));
        assert_service_incident_acknowledge_response_contract(&json!({
            "acknowledged": true,
            "incident": incident.clone(),
        }));
        assert_service_incident_resolve_response_contract(&json!({
            "resolved": true,
            "incident": incident,
        }));
        assert_service_monitor_triage_response_contract(&json!({
            "id": "google-login-freshness",
            "monitor": {
                "id": "google-login-freshness",
                "name": "Google login freshness",
                "target": {"site_policy": "google"},
                "intervalMs": 60000,
                "state": "active",
                "lastCheckedAt": null,
                "lastSucceededAt": null,
                "lastFailedAt": null,
                "lastResult": null,
                "consecutiveFailures": 0,
            },
            "state": "active",
            "updated": true,
            "resetFailures": true,
            "acknowledged": true,
            "incident": {
                "id": "monitor:google-login-freshness",
                "browserId": null,
                "monitorId": "google-login-freshness",
                "monitorTarget": {"site_policy": "google"},
                "monitorResult": "site_policy_missing",
                "label": "Monitor google-login-freshness",
                "state": "active",
                "severity": "warning",
                "escalation": "monitor_attention",
                "recommendedAction": "Inspect the failed monitor target and last result; fix the target, refresh login state, pause the monitor, or reset reviewed failures before rerunning.",
                "acknowledgedAt": "2026-04-22T00:00:01Z",
                "acknowledgedBy": "operator",
                "acknowledgementNote": "reviewed",
                "resolvedAt": null,
                "resolvedBy": null,
                "resolutionNote": null,
                "latestTimestamp": "2026-04-22T00:00:00Z",
                "latestMessage": "Monitor failed",
                "latestKind": "reconciliation_error",
                "currentHealth": null,
                "eventIds": ["event-monitor-failed"],
                "jobIds": [],
            },
        }));
        assert_service_remedies_apply_response_contract(&json!({
            "applied": true,
            "escalation": "monitor_attention",
            "count": 1,
            "monitorIds": ["google-login-freshness"],
            "monitorResults": [{
                "id": "google-login-freshness",
                "monitor": {
                    "id": "google-login-freshness",
                    "name": "Google login freshness",
                    "target": {"site_policy": "google"},
                    "intervalMs": 60000,
                    "state": "active",
                    "lastCheckedAt": null,
                    "lastSucceededAt": null,
                    "lastFailedAt": null,
                    "lastResult": null,
                    "consecutiveFailures": 0,
                },
                "state": "active",
                "updated": true,
                "resetFailures": true,
                "acknowledged": true,
                "incident": {
                    "id": "monitor:google-login-freshness",
                    "browserId": null,
                    "monitorId": "google-login-freshness",
                    "monitorTarget": {"site_policy": "google"},
                    "monitorResult": "site_policy_missing",
                    "label": "Monitor google-login-freshness",
                    "state": "active",
                    "severity": "warning",
                    "escalation": "monitor_attention",
                    "recommendedAction": "Inspect the failed monitor target and last result; fix the target, refresh login state, pause the monitor, or reset reviewed failures before rerunning.",
                    "acknowledgedAt": "2026-04-22T00:00:01Z",
                    "acknowledgedBy": "operator",
                    "acknowledgementNote": "reviewed",
                    "resolvedAt": null,
                    "resolvedBy": null,
                    "resolutionNote": null,
                    "latestTimestamp": "2026-04-22T00:00:00Z",
                    "latestMessage": "Monitor failed",
                    "latestKind": "reconciliation_error",
                    "currentHealth": null,
                    "eventIds": ["event-monitor-failed"],
                    "jobIds": [],
                },
            }],
            "browserIds": [],
            "browserResults": [],
        }));
        assert_service_remedies_apply_response_contract(&json!({
            "applied": true,
            "escalation": "os_degraded_possible",
            "count": 1,
            "monitorIds": [],
            "monitorResults": [],
            "browserIds": ["browser-1"],
            "browserResults": [{
                "id": "browser-1",
                "retryEnabled": true,
                "browser": {
                    "id": "browser-1",
                    "profileId": "work",
                    "host": "attached_existing",
                    "health": "process_exited",
                    "pid": null,
                    "cdpEndpoint": null,
                    "viewStreams": [],
                    "activeSessionIds": ["session-1"],
                    "lastError": "Browser retry requested by operator",
                    "lastHealthObservation": null,
                },
                "incident": null,
            }],
        }));
    }

    #[test]
    fn service_reconcile_response_contract_matches_wire_shape() {
        let response_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-reconcile-response.v1.schema.json"
        ))
        .unwrap();

        assert_schema_required_fields(
            &response_schema,
            &[
                "reconciled",
                "browserCount",
                "changedBrowsers",
                "service_state",
            ],
        );

        assert_service_reconcile_response_contract(&json!({
            "reconciled": true,
            "browserCount": 1,
            "changedBrowsers": 1,
            "service_state": {
                "profiles": {},
                "browsers": {},
                "sessions": {},
                "tabs": {},
                "sitePolicies": {},
                "providers": {},
                "challenges": {},
                "events": [],
                "jobs": {},
                "incidents": [],
                "reconciliation": null,
            },
        }));
    }

    #[test]
    fn service_status_and_collection_response_contracts_match_wire_shape() {
        let status_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-status-response.v1.schema.json"
        ))
        .unwrap();
        let collection_schemas = [
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-profiles-response.v1.schema.json"
                ))
                .unwrap(),
                "profiles",
                "profiles response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-sessions-response.v1.schema.json"
                ))
                .unwrap(),
                "sessions",
                "sessions response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-browsers-response.v1.schema.json"
                ))
                .unwrap(),
                "browsers",
                "browsers response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-tabs-response.v1.schema.json"
                ))
                .unwrap(),
                "tabs",
                "tabs response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-monitors-response.v1.schema.json"
                ))
                .unwrap(),
                "monitors",
                "monitors response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-site-policies-response.v1.schema.json"
                ))
                .unwrap(),
                "sitePolicies",
                "site policies response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-providers-response.v1.schema.json"
                ))
                .unwrap(),
                "providers",
                "providers response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-challenges-response.v1.schema.json"
                ))
                .unwrap(),
                "challenges",
                "challenges response",
            ),
        ];

        assert_schema_required_fields(&status_schema, &["service_state", "profileAllocations"]);
        assert!(status_schema["properties"]["control_plane"]["properties"]
            .get("waiting_profile_lease_job_count")
            .is_some());
        assert!(status_schema["properties"]["control_plane"]["properties"]
            .get("service_monitor_interval_ms")
            .is_some());
        assert_service_status_response_contract(&json!({
            "control_plane": {
                "waiting_profile_lease_job_count": 0,
                "service_monitor_interval_ms": 60000
            },
            "service_state": {},
            "profileAllocations": [],
        }));

        for (schema, field, label) in collection_schemas {
            if field == "profiles" {
                assert_schema_required_fields(
                    &schema,
                    &[field, "profileSources", "profileAllocations", "count"],
                );
            } else if field == "sitePolicies" {
                assert_schema_required_fields(&schema, &[field, "sitePolicySources", "count"]);
            } else {
                assert_schema_required_fields(&schema, &[field, "count"]);
            }
            let response = if field == "profiles" {
                json!({
                    field: [],
                    "profileSources": [],
                    "profileAllocations": [],
                    "count": 0,
                })
            } else if field == "sitePolicies" {
                json!({
                    field: [],
                    "sitePolicySources": [],
                    "count": 0,
                })
            } else {
                json!({
                    field: [],
                    "count": 0,
                })
            };
            assert_service_collection_response_contract(&response, field, label);
        }
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
            summary_schema["properties"]["contexts"]["items"]["properties"]["controlPlaneModes"]
                ["items"]["enum"],
            json!(SERVICE_JOB_CONTROL_PLANE_MODE_VALUES.to_vec())
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
                "profileLeaseWaits",
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
            "profileLeaseWaits": {
                "count": 0,
                "activeCount": 0,
                "completedCount": 0,
                "waits": [],
            },
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
                "targetIdentityCount": 1,
                "targetServiceIds": ["acs"],
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
            "monitorId": null,
            "monitorTarget": null,
            "monitorResult": null,
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
                waiting_profile_lease_job_count: 1,
                service_job_timeout_ms: Some(5000),
                service_monitor_interval_ms: Some(60000),
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
                    target_service_ids: vec!["google".to_string(), "acs".to_string()],
                    authenticated_service_ids: vec!["google".to_string()],
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
    fn builtin_site_policies_apply_without_overriding_local_policy() {
        let mut state = ServiceState {
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "local-google".to_string(),
                    browser_host: Some(BrowserHost::RemoteHeaded),
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.apply_builtin_site_policies();

        assert_eq!(state.site_policies["google"].origin_pattern, "local-google");
        assert_eq!(
            state.site_policies["google"].browser_host,
            Some(BrowserHost::RemoteHeaded)
        );
        assert_eq!(
            state.site_policies["microsoft"].origin_pattern,
            "https://login.microsoftonline.com"
        );
        assert_eq!(
            state.site_policies["canva"].origin_pattern,
            "https://www.canva.com"
        );
        assert!(state.site_policies["canva"].requires_cdp_free);
        assert_eq!(
            state.site_policies["microsoft"].interaction_mode,
            InteractionMode::HumanLikeInput
        );
        assert_eq!(
            state.site_policies["gmail"].challenge_policy,
            ChallengePolicy::ManualOnly
        );
    }

    #[test]
    fn refresh_profile_readiness_marks_google_profiles_for_manual_seeding() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([
                (
                    "google-new".to_string(),
                    BrowserProfile {
                        id: "google-new".to_string(),
                        name: "Google New".to_string(),
                        target_service_ids: vec!["google".to_string()],
                        ..BrowserProfile::default()
                    },
                ),
                (
                    "google-seeded".to_string(),
                    BrowserProfile {
                        id: "google-seeded".to_string(),
                        name: "Google Seeded".to_string(),
                        target_service_ids: vec!["google".to_string()],
                        authenticated_service_ids: vec!["google".to_string()],
                        ..BrowserProfile::default()
                    },
                ),
            ]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    manual_login_preferred: true,
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_profile_readiness();

        let new_google = &state.profiles["google-new"].target_readiness[0];
        assert_eq!(new_google.target_service_id, "google");
        assert_eq!(new_google.state, ProfileReadinessState::NeedsManualSeeding);
        assert!(new_google.manual_seeding_required);
        assert_eq!(
            new_google.recommended_action,
            "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
        );
        assert_eq!(
            new_google.seeding_mode,
            ProfileSeedingMode::DetachedHeadedNoCdp
        );
        assert!(!new_google.cdp_attachment_allowed_during_seeding);
        assert_eq!(
            new_google.preferred_keyring,
            Some(ProfileKeyringPolicy::BasicPasswordStore)
        );
        assert_eq!(
            new_google.setup_scopes,
            vec!["signin", "chrome_sync", "passkeys", "browser_plugins"]
        );

        let seeded_google = &state.profiles["google-seeded"].target_readiness[0];
        assert_eq!(
            seeded_google.state,
            ProfileReadinessState::SeededUnknownFreshness
        );
        assert!(!seeded_google.manual_seeding_required);
        assert_eq!(seeded_google.seeding_mode, ProfileSeedingMode::NotRequired);
        assert!(seeded_google.setup_scopes.is_empty());
        assert_eq!(
            seeded_google.recommended_action,
            "probe_target_auth_or_reuse_if_acceptable"
        );
    }

    #[test]
    fn service_profile_seeding_handoff_returns_detached_runtime_command() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "google-new".to_string(),
                BrowserProfile {
                    id: "google-new".to_string(),
                    name: "Google New".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "https://accounts.google.com".to_string(),
                    manual_login_preferred: true,
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };
        state.refresh_profile_readiness();

        let handoff =
            service_profile_seeding_handoff(&state, "google-new", Some("google")).unwrap();

        assert_eq!(handoff["profileId"], "google-new");
        assert_eq!(handoff["targetServiceId"], "google");
        assert_eq!(handoff["seedingMode"], "detached_headed_no_cdp");
        assert_eq!(handoff["cdpAttachmentAllowedDuringSeeding"], false);
        assert_eq!(handoff["preferredKeyring"], "basic_password_store");
        assert_eq!(
            handoff["command"],
            "agent-browser --runtime-profile google-new runtime login https://accounts.google.com"
        );
        assert_eq!(
            handoff["operatorIntervention"]["severity"],
            "action_required"
        );
        assert_eq!(
            handoff["operatorIntervention"]["desktopPopupPolicy"],
            "optional_policy_controlled"
        );
        assert_eq!(handoff["operatorIntervention"]["blocksProfileLease"], true);
        assert_eq!(
            handoff["operatorIntervention"]["defaultChannels"],
            serde_json::json!(["api", "mcp", "dashboard"])
        );
        assert_eq!(handoff["lifecycle"]["state"], "needs_manual_seeding");
        assert_eq!(handoff["lifecycle"]["id"], "google-new:google");
        assert!(handoff["operatorIntervention"]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["id"] == "force_close_seeded_browser"
                && action["safety"] == "danger"));
        assert!(handoff["warnings"][0]
            .as_str()
            .unwrap()
            .contains("--attachable"));
    }

    #[test]
    fn service_profile_seeding_handoff_uses_persisted_lifecycle_state() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "google-new".to_string(),
                BrowserProfile {
                    id: "google-new".to_string(),
                    name: "Google New".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            profile_seeding_handoffs: BTreeMap::from([(
                "google-new:google".to_string(),
                ProfileSeedingHandoffRecord {
                    id: "google-new:google".to_string(),
                    profile_id: "google-new".to_string(),
                    target_service_id: "google".to_string(),
                    state: ProfileSeedingHandoffState::SeedingClosedUnverified,
                    pid: Some(1234),
                    closed_at: Some("2026-05-10T12:00:00Z".to_string()),
                    ..ProfileSeedingHandoffRecord::default()
                },
            )]),
            ..ServiceState::default()
        };
        state.refresh_profile_readiness();

        let handoff =
            service_profile_seeding_handoff(&state, "google-new", Some("google")).unwrap();

        assert_eq!(
            handoff["operatorIntervention"]["state"],
            "seeding_closed_unverified"
        );
        assert_eq!(handoff["operatorIntervention"]["severity"], "attention");
        assert_eq!(handoff["operatorIntervention"]["blocksProfileLease"], false);
        assert_eq!(handoff["lifecycle"]["pid"], 1234);
        assert_eq!(handoff["lifecycle"]["closedAt"], "2026-05-10T12:00:00Z");
    }

    #[test]
    fn refresh_profile_readiness_preserves_explicit_freshness_evidence() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "google-fresh".to_string(),
                BrowserProfile {
                    id: "google-fresh".to_string(),
                    name: "Google Fresh".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    authenticated_service_ids: vec!["google".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "google".to_string(),
                        state: ProfileReadinessState::Fresh,
                        evidence: "auth_probe_cookie_present".to_string(),
                        recommended_action: "use_profile".to_string(),
                        last_verified_at: Some("2026-05-06T12:00:00Z".to_string()),
                        freshness_expires_at: Some("2026-05-06T13:00:00Z".to_string()),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_profile_readiness();

        let readiness = &state.profiles["google-fresh"].target_readiness[0];
        assert_eq!(readiness.target_service_id, "google");
        assert_eq!(readiness.state, ProfileReadinessState::Fresh);
        assert!(!readiness.manual_seeding_required);
        assert_eq!(readiness.evidence, "auth_probe_cookie_present");
        assert_eq!(readiness.recommended_action, "use_profile");
        assert_eq!(
            readiness.last_verified_at.as_deref(),
            Some("2026-05-06T12:00:00Z")
        );
        assert_eq!(
            readiness.freshness_expires_at.as_deref(),
            Some("2026-05-06T13:00:00Z")
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
