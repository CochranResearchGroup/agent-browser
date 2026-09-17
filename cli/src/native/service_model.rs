//! Durable service-mode contracts.
//!
//! These types describe the browser service state model before the service API
//! and MCP surfaces are wired to runtime behavior. Keep them serializable and
//! conservative so future clients can depend on stable field names.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const SERVICE_TRACE_ACTIVITY_SOURCE_VALUES: [&str; 3] = ["event", "job", "metadata"];
pub const SERVICE_TRACE_ACTIVITY_KIND_VALUES: [&str; 23] = [
    "reconciliation",
    "browser_launch_recorded",
    "browser_health_changed",
    "browser_recovery_started",
    "browser_recovery_override",
    "tab_lifecycle_changed",
    "profile_lease_wait_started",
    "profile_lease_wait_ended",
    "profile_lease_lifecycle_changed",
    "viewer_takeover_requested",
    "viewer_connected",
    "viewer_disconnected",
    "controller_requested",
    "controller_granted",
    "controller_denied",
    "route_released",
    "reconciliation_error",
    "incident_acknowledged",
    "incident_resolved",
    "job_terminal",
    "service_job_timeout",
    "service_job_cancelled",
    "service_job",
];
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
            "provenance",
            "terminalOutcome",
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
            "profileOrigin",
            "profileClass",
            "userDataDir",
            "sitePolicyIds",
            "targetServiceIds",
            "authenticatedServiceIds",
            "accountIds",
            "defaultBrowserHost",
            "browserBuild",
            "allocation",
            "keyring",
            "sharedServiceIds",
            "credentialProviderIds",
            "manualLoginPreferred",
            "targetReadiness",
            "registration",
            "browserCompatibilityEvidence",
            "persistent",
            "tags",
        ],
        &[
            "profile_origin",
            "profile_class",
            "user_data_dir",
            "site_policy_ids",
            "target_service_ids",
            "authenticated_service_ids",
            "account_ids",
            "default_browser_host",
            "browser_build",
            "shared_service_ids",
            "credential_provider_ids",
            "manual_login_preferred",
            "target_readiness",
            "browser_compatibility_evidence",
        ],
    );
    assert!(matches!(
        value["profileOrigin"].as_str(),
        Some("agent_browser_owned" | "external_byop" | "external_observed")
    ));
    assert!(SERVICE_PROFILE_CLASS_VALUES.contains(&value["profileClass"].as_str().unwrap()));
    if let Some(host) = value["defaultBrowserHost"].as_str() {
        assert!(SERVICE_BROWSER_HOST_VALUES.contains(&host));
    }
    if let Some(build) = value["browserBuild"].as_str() {
        assert!(SERVICE_BROWSER_BUILD_VALUES.contains(&build));
    }
    assert!(SERVICE_PROFILE_ALLOCATION_VALUES.contains(&value["allocation"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_KEYRING_VALUES.contains(&value["keyring"].as_str().unwrap()));
    assert!(value["sitePolicyIds"].is_array());
    assert!(value["targetServiceIds"].is_array());
    assert!(value["authenticatedServiceIds"].is_array());
    assert!(value["accountIds"].is_array());
    assert!(value["sharedServiceIds"].is_array());
    assert!(value["credentialProviderIds"].is_array());
    assert!(value["targetReadiness"].is_array());
    assert!(
        value["registration"].is_null() || value["registration"].is_object(),
        "profile registration should be null or object"
    );
    assert!(value["browserCompatibilityEvidence"].is_array());
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
            "displayIsolation",
            "displayName",
            "displayAllocationId",
            "pid",
            "cdpEndpoint",
            "viewStreams",
            "activeSessionIds",
            "lastError",
            "lastHealthObservation",
        ],
        &[
            "profile_id",
            "display_isolation",
            "display_name",
            "display_allocation_id",
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
    for stream in value["viewStreams"].as_array().unwrap() {
        assert_service_view_stream_record_contract(stream);
    }
    if let Some(attachability) = value.get("attachability") {
        assert!(
            attachability.is_object() || attachability.is_null(),
            "browser attachability must be object or null"
        );
    }
    assert!(value["activeSessionIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_view_stream_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "view stream",
        value,
        &["id", "provider", "controlInput", "url", "readOnly"],
        &["control_input", "read_only"],
    );
    assert!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.contains(&value["provider"].as_str().unwrap()));
    if let Some(control_input) = value["controlInput"].as_str() {
        assert!(SERVICE_CONTROL_INPUT_PROVIDER_VALUES.contains(&control_input));
    }
    assert!(value["readOnly"].is_boolean());
    if value.get("viewerLeaseIds").is_some() {
        assert!(value["viewerLeaseIds"].is_array());
    }
    if let Some(attachability) = value.get("attachability") {
        assert!(
            attachability.is_object() || attachability.is_null(),
            "view stream attachability must be object or null"
        );
    }
}

#[cfg(test)]
pub fn assert_service_display_allocation_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "display allocation",
        value,
        &[
            "id",
            "displayName",
            "displayIsolation",
            "ownerBrowserId",
            "ownerSessionId",
            "profileId",
            "browserBuild",
            "host",
            "state",
            "pidHints",
            "routeIds",
            "createdAt",
            "updatedAt",
            "lastHealthCheckAt",
            "readiness",
        ],
        &[
            "display_name",
            "display_isolation",
            "owner_browser_id",
            "owner_session_id",
            "profile_id",
            "browser_build",
            "pid_hints",
            "route_ids",
            "created_at",
            "updated_at",
            "last_health_check_at",
        ],
    );
    if let Some(host) = value["host"].as_str() {
        assert!(SERVICE_BROWSER_HOST_VALUES.contains(&host));
    }
    assert!(value["routeIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_remote_view_route_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "remote view route",
        value,
        &[
            "id",
            "provider",
            "displayAllocationId",
            "browserId",
            "sessionId",
            "routeSource",
            "connectionId",
            "connectionName",
            "routeTemplate",
            "frameUrl",
            "externalUrl",
            "routeDescriptor",
            "readOnly",
            "controlInput",
            "providerMode",
            "state",
            "viewerLeaseIds",
            "controllerLeaseId",
            "lastProviderEvent",
            "readiness",
        ],
        &[
            "display_allocation_id",
            "browser_id",
            "session_id",
            "route_source",
            "connection_id",
            "connection_name",
            "route_template",
            "frame_url",
            "external_url",
            "route_descriptor",
            "read_only",
            "control_input",
            "provider_mode",
            "viewer_lease_ids",
            "controller_lease_id",
            "last_provider_event",
        ],
    );
    assert!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.contains(&value["provider"].as_str().unwrap()));
    if let Some(control_input) = value["controlInput"].as_str() {
        assert!(SERVICE_CONTROL_INPUT_PROVIDER_VALUES.contains(&control_input));
    }
    assert!(value["readOnly"].is_boolean());
    assert!(value["viewerLeaseIds"].is_array());
}

#[cfg(test)]
pub fn assert_service_route_pool_entry_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "route pool entry",
        value,
        &[
            "id",
            "provider",
            "routeId",
            "connectionId",
            "connectionName",
            "frameUrl",
            "externalUrl",
            "routeDescriptor",
            "target",
            "providerMode",
            "state",
            "currentRouteAllocationId",
            "readiness",
        ],
        &[
            "route_id",
            "connection_id",
            "connection_name",
            "frame_url",
            "external_url",
            "route_descriptor",
            "provider_mode",
            "current_route_allocation_id",
        ],
    );
    assert!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.contains(&value["provider"].as_str().unwrap()));
    assert!(value["target"].is_object());
}

#[cfg(test)]
pub fn assert_service_viewer_lease_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "viewer lease",
        value,
        &[
            "id",
            "routeId",
            "browserId",
            "viewerId",
            "viewerName",
            "viewerRole",
            "openMode",
            "state",
            "lastViewerEvent",
            "expiresAt",
            "createdAt",
            "updatedAt",
            "lastHeartbeatAt",
            "serviceEventId",
        ],
        &[
            "route_id",
            "browser_id",
            "viewer_id",
            "viewer_name",
            "viewer_role",
            "open_mode",
            "last_viewer_event",
            "expires_at",
            "created_at",
            "updated_at",
            "last_heartbeat_at",
            "service_event_id",
        ],
    );
}

#[cfg(test)]
pub fn assert_service_remote_view_acquisition_lease_record_contract(value: &serde_json::Value) {
    assert_record_fields(
        "remote view acquisition lease",
        value,
        &[
            "id",
            "browserId",
            "sessionId",
            "routeId",
            "displayAllocationId",
            "routePoolEntryId",
            "state",
            "phase",
            "createdAt",
            "updatedAt",
            "completedAt",
            "failedAt",
            "failureReason",
            "cleanup",
            "previousRoutePoolEntry",
            "previousDisplayAllocation",
            "previousRemoteViewRoute",
            "previousBrowserDisplayAllocationId",
        ],
        &[
            "browser_id",
            "session_id",
            "route_id",
            "display_allocation_id",
            "route_pool_entry_id",
            "created_at",
            "updated_at",
            "completed_at",
            "failed_at",
            "failure_reason",
            "previous_route_pool_entry",
            "previous_display_allocation",
            "previous_remote_view_route",
            "previous_browser_display_allocation_id",
        ],
    );
    if let Some(entry) = value["previousRoutePoolEntry"].as_object() {
        assert_service_route_pool_entry_record_contract(&Value::Object(entry.clone()));
    }
    if let Some(allocation) = value["previousDisplayAllocation"].as_object() {
        assert_service_display_allocation_record_contract(&Value::Object(allocation.clone()));
    }
    if let Some(route) = value["previousRemoteViewRoute"].as_object() {
        assert_service_remote_view_route_record_contract(&Value::Object(route.clone()));
    }
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
            "browserCapabilityLaunch",
            "cleanup",
            "browserIds",
            "tabIds",
            "createdAt",
            "lastLeaseObservedAt",
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
            "browser_capability_launch",
            "browser_ids",
            "tab_ids",
            "created_at",
            "last_lease_observed_at",
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
    assert!(
        value["browserCapabilityLaunch"].is_object() || value["browserCapabilityLaunch"].is_null()
    );
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
            "browserBuild",
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
            "browser_build",
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
    if let Some(build) = value["browserBuild"].as_str() {
        assert!(SERVICE_BROWSER_BUILD_VALUES.contains(&build));
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
            "browserCapabilityLaunches",
            "displayAllocations",
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
    let browser_capability_launches = &value["browserCapabilityLaunches"];
    assert_record_fields(
        "trace browser capability launches",
        browser_capability_launches,
        &["count", "appliedCount", "skippedCount", "launches"],
        &["applied_count", "skipped_count"],
    );
    let launches = browser_capability_launches["launches"].as_array().unwrap();
    assert_eq!(
        browser_capability_launches["count"].as_u64().unwrap(),
        launches.len() as u64
    );
    assert!(browser_capability_launches["appliedCount"].is_u64());
    assert!(browser_capability_launches["skippedCount"].is_u64());
    for launch in launches {
        assert_record_fields(
            "trace browser capability launch",
            launch,
            &[
                "source",
                "timestamp",
                "serviceName",
                "agentName",
                "taskName",
                "browserId",
                "profileId",
                "sessionId",
                "applied",
                "reason",
                "browserBuild",
                "bindingId",
                "hostId",
                "executableId",
                "capabilityId",
                "executablePath",
            ],
            &[
                "service_name",
                "agent_name",
                "task_name",
                "browser_id",
                "profile_id",
                "session_id",
                "browser_build",
                "binding_id",
                "host_id",
                "executable_id",
                "capability_id",
                "executable_path",
            ],
        );
        assert!(matches!(
            launch["source"].as_str().unwrap(),
            "event" | "session"
        ));
        assert!(launch["applied"].is_boolean());
    }
    let display_allocations = &value["displayAllocations"];
    assert_record_fields(
        "trace display allocations",
        display_allocations,
        &[
            "count",
            "recordedCount",
            "unrecordedCount",
            "privateVirtualDisplayCount",
            "sharedDisplayCount",
            "ambientDisplayCount",
            "allocations",
            "unrecordedJobIds",
        ],
        &[
            "recorded_count",
            "unrecorded_count",
            "private_virtual_display_count",
            "shared_display_count",
            "ambient_display_count",
            "unrecorded_job_ids",
        ],
    );
    assert!(display_allocations["count"].is_u64());
    assert!(display_allocations["recordedCount"].is_u64());
    assert!(display_allocations["unrecordedCount"].is_u64());
    assert!(display_allocations["privateVirtualDisplayCount"].is_u64());
    assert!(display_allocations["sharedDisplayCount"].is_u64());
    assert!(display_allocations["ambientDisplayCount"].is_u64());
    assert!(display_allocations["unrecordedJobIds"].is_array());
    for allocation in display_allocations["allocations"].as_array().unwrap() {
        assert_record_fields(
            "trace display allocation",
            allocation,
            &["displayIsolation", "label", "count", "jobIds"],
            &["display_isolation", "job_ids"],
        );
        assert!(allocation["displayIsolation"].is_string());
        assert!(allocation["label"].is_string());
        assert!(allocation["count"].is_u64());
        assert!(allocation["jobIds"].is_array());
    }
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
                "controlPlaneModes",
                "displayAllocations",
                "unrecordedDisplayAllocationJobCount",
                "lifecycleOnlyJobCount",
                "attention",
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
                "control_plane_modes",
                "display_allocations",
                "unrecorded_display_allocation_job_count",
                "lifecycle_only_job_count",
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
        assert!(context["controlPlaneModes"].is_array());
        assert!(context["displayAllocations"].is_array());
        assert!(context["unrecordedDisplayAllocationJobCount"].is_u64());
        assert!(context["lifecycleOnlyJobCount"].is_u64());
        assert_record_fields(
            "trace summary context attention",
            &context["attention"],
            &[
                "required",
                "owner",
                "severity",
                "reason",
                "message",
                "suggestedActions",
                "presentation",
            ],
            &["suggested_actions"],
        );
        assert!(context["attention"]["required"].is_boolean());
        assert!(matches!(
            context["attention"]["owner"].as_str().unwrap(),
            "none" | "operator" | "service"
        ));
        assert!(matches!(
            context["attention"]["severity"].as_str().unwrap(),
            "info" | "warning"
        ));
        assert!(matches!(
            context["attention"]["reason"].as_str().unwrap(),
            "none" | "incidents_present" | "missing_caller_label"
        ));
        assert!(context["attention"]["message"].is_string());
        assert!(context["attention"]["suggestedActions"].is_array());
        assert_eq!(context["attention"]["presentation"], "client_decides");
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
                "provenance",
                "terminalOutcome",
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
pub fn assert_service_browser_capability_registry_upsert_response_contract(
    value: &serde_json::Value,
) {
    assert_record_fields(
        "browser capability registry upsert response",
        value,
        &[
            "id",
            "collection",
            "record",
            "browserCapabilityRegistry",
            "counts",
            "upserted",
            "advisory",
            "routingApplied",
        ],
        &[],
    );
    assert!(value["id"].is_string());
    assert!(value["collection"].is_string());
    assert!(value["record"].is_object());
    assert!(value["browserCapabilityRegistry"].is_object());
    assert!(value["counts"].is_object());
    assert_eq!(value["upserted"], true);
    assert_eq!(value["advisory"], true);
    assert_eq!(value["routingApplied"], false);
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
            "expiredSessionLeases",
            "expiredSessionLeaseCount",
            "remoteViewRepair",
            "service_state",
        ],
        &["browser_count", "changed_browsers"],
    );
    assert!(value["reconciled"].is_boolean());
    assert!(value["browserCount"].is_u64());
    assert!(value["changedBrowsers"].is_u64());
    assert!(value["expiredSessionLeases"].is_array());
    assert!(value["expiredSessionLeaseCount"].is_u64());
    assert!(value["remoteViewRepair"].is_object());
    assert!(value["remoteViewRepair"]["unavailableRoutePoolEntries"].is_u64());
    assert!(value["remoteViewRepair"]["restoredRoutePoolEntries"].is_u64());
    assert!(value["remoteViewRepair"]["orphanedDisplayAllocations"].is_u64());
    assert!(value["remoteViewRepair"]["orphanedRoutes"].is_u64());
    assert!(value["remoteViewRepair"]["releasedViewerLeases"].is_u64());
    assert!(value["remoteViewRepair"]["expiredViewerLeases"].is_u64());
    assert!(value["remoteViewRepair"]["clearedControllerLeases"].is_u64());
    assert!(value["remoteViewRepair"]["repaired"].is_u64());
    assert!(value["remoteViewRepair"]["released"].is_u64());
    assert!(value["remoteViewRepair"]["skippedUnsafe"].is_u64());
    assert!(value["service_state"].is_object());
}

#[cfg(test)]
pub fn assert_service_status_response_contract(value: &serde_json::Value) {
    assert_record_fields(
        "service status response",
        value,
        &[
            "control_plane",
            "service_state",
            "profileAllocations",
            "manualBrowsers",
            "browserSessionAuthority",
            "statusProjection",
            "serviceStateLockDiagnostics",
        ],
        &["serviceState"],
    );
    assert!(value["service_state"].is_object());
    assert!(value["control_plane"].is_object());
    assert!(value["manualBrowsers"].is_array());
    assert_eq!(value["browserSessionAuthority"]["schemaVersion"], 1);
    assert!(matches!(
        value["browserSessionAuthority"]["availability"].as_str(),
        Some("available" | "partial" | "unknown")
    ));
    let authority_summary = &value["browserSessionAuthority"]["summary"];
    for field in [
        "modeledBrowserCount",
        "viableBrowserCount",
        "attentionBrowserCount",
        "nonViableBrowserCount",
        "unknownBrowserCount",
    ] {
        assert!(authority_summary[field].is_u64(), "{field}");
    }
    assert_eq!(value["statusProjection"]["schemaVersion"], 1);
    assert_eq!(
        value["statusProjection"]["authority"]["source"],
        "reconciled_service_state"
    );
    assert!(value["statusProjection"]["authority"]["projectedAt"].is_string());
    assert!(matches!(
        value["statusProjection"]["observations"]["state"].as_str(),
        Some("complete" | "partial" | "unavailable")
    ));
    assert_eq!(
        value["serviceStateLockDiagnostics"]["schemaVersion"],
        "agent-browser.service-state-lock-diagnostics.v1"
    );
    assert!(value["serviceStateLockDiagnostics"]["active"].is_array());
    assert!(value["serviceStateLockDiagnostics"]["recent"].is_array());
    if let Some(launch_config) = value.get("launchConfig") {
        assert_record_fields(
            "service status launch config",
            launch_config,
            &[
                "defaultBrowserBuild",
                "stealthCdpChromiumRequired",
                "stealthCdpChromiumReady",
                "executablePath",
                "executablePathSource",
                "executablePathExists",
                "browserBuildManifests",
                "profileSmoke",
                "warnings",
            ],
            &["default_browser_build"],
        );
        assert!(launch_config["stealthCdpChromiumRequired"].is_boolean());
        assert!(launch_config["stealthCdpChromiumReady"].is_boolean());
        assert!(launch_config["profileSmoke"].is_object());
        assert!(launch_config["profileSmoke"]["available"].is_boolean());
        assert!(launch_config["profileSmoke"]["command"].is_string());
        assert!(launch_config["profileSmoke"]["reason"].is_string());
        assert!(launch_config["warnings"].is_array());
    }
    for allocation in value["profileAllocations"].as_array().unwrap() {
        assert_service_profile_allocation_contract(allocation);
    }
    if let Some(retained_display_allocations) = value.get("retainedDisplayAllocations") {
        assert_record_fields(
            "service status retained display allocations",
            retained_display_allocations,
            &[
                "count",
                "applySafeCount",
                "retainedCount",
                "classCounts",
                "applySafeIds",
                "retainedIds",
                "candidateReasons",
                "explanation",
                "cleanupCommand",
            ],
            &["apply_safe_count", "candidate_reasons"],
        );
        assert!(retained_display_allocations["count"].is_u64());
        assert!(retained_display_allocations["applySafeCount"].is_u64());
        assert!(retained_display_allocations["retainedCount"].is_u64());
        assert!(retained_display_allocations["classCounts"].is_object());
        assert!(retained_display_allocations["applySafeIds"].is_array());
        assert!(retained_display_allocations["retainedIds"].is_array());
        assert!(retained_display_allocations["candidateReasons"].is_object());
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
            "profileOrigin",
            "profileClass",
            "allocation",
            "keyring",
            "browserBuild",
            "targetServiceIds",
            "authenticatedServiceIds",
            "accountIds",
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
            "browserSummaries",
            "tabIds",
        ],
        &[
            "profile_id",
            "profile_name",
            "profile_origin",
            "profile_class",
            "browser_build",
            "target_service_ids",
            "authenticated_service_ids",
            "account_ids",
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
            "browser_summaries",
            "tab_ids",
        ],
    );
    assert!(SERVICE_PROFILE_CLASS_VALUES.contains(&value["profileClass"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_ALLOCATION_VALUES.contains(&value["allocation"].as_str().unwrap()));
    assert!(SERVICE_PROFILE_KEYRING_VALUES.contains(&value["keyring"].as_str().unwrap()));
    if let Some(build) = value["browserBuild"].as_str() {
        assert!(SERVICE_BROWSER_BUILD_VALUES.contains(&build));
    }
    assert!(value["targetServiceIds"].is_array());
    assert!(value["authenticatedServiceIds"].is_array());
    assert!(value["accountIds"].is_array());
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
    assert!(value["browserSummaries"].is_array());
    for browser in value["browserSummaries"].as_array().unwrap() {
        assert_record_fields(
            "profile allocation browser summary",
            browser,
            &[
                "browserId",
                "host",
                "health",
                "pid",
                "hasCdpEndpoint",
                "activeSessionIds",
            ],
            &["browser_id", "has_cdp_endpoint", "active_session_ids"],
        );
        assert!(SERVICE_BROWSER_HOST_VALUES.contains(&browser["host"].as_str().unwrap()));
        assert!(SERVICE_BROWSER_HEALTH_VALUES.contains(&browser["health"].as_str().unwrap()));
        assert!(browser["pid"].is_u64() || browser["pid"].is_null());
        assert!(browser["hasCdpEndpoint"].is_boolean());
        assert!(browser["activeSessionIds"].is_array());
    }
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

pub use agent_browser_service_model::{
    browser_profile_compatibility_matches, builtin_site_policies, builtin_site_policy,
    default_profile_seeding_url, service_profile_sources, service_site_policy_sources,
    BrowserCapabilityRegistry, ServiceState,
};

/// Backend-owned allocation summary for one profile.
///
/// This is derived from service state at read time so API, MCP, CLI, and UI
/// consumers share the same profile/session coordination model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceProfileAllocation {
    pub profile_id: String,
    pub profile_name: String,
    pub profile_origin: ProfileOrigin,
    pub profile_class: ProfileClass,
    pub allocation: ProfileAllocationPolicy,
    pub keyring: ProfileKeyringPolicy,
    pub browser_build: Option<BrowserBuild>,
    pub target_service_ids: Vec<String>,
    pub authenticated_service_ids: Vec<String>,
    pub account_ids: Vec<String>,
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
    pub browser_summaries: Vec<ServiceProfileAllocationBrowserSummary>,
    pub tab_ids: Vec<String>,
}

/// Compact browser state attached to a profile allocation.
///
/// This lets service clients answer which browser currently hosts a profile
/// without joining the raw browser collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceProfileAllocationBrowserSummary {
    pub browser_id: String,
    pub host: BrowserHost,
    pub health: BrowserHealth,
    pub pid: Option<u32>,
    pub has_cdp_endpoint: bool,
    pub active_session_ids: Vec<String>,
}

impl Default for ServiceProfileAllocationBrowserSummary {
    fn default() -> Self {
        Self {
            browser_id: String::new(),
            host: BrowserHost::LocalHeaded,
            health: BrowserHealth::NotStarted,
            pid: None,
            has_cdp_endpoint: false,
            active_session_ids: Vec::new(),
        }
    }
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
    let mut browser_summaries = BTreeMap::new();
    let mut tab_ids = BTreeSet::new();

    for session in service_state.sessions.values() {
        if session.profile_id.as_deref() != Some(profile_id) || session.lease.is_inactive() {
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
            browser_summaries.insert(
                browser.id.clone(),
                ServiceProfileAllocationBrowserSummary {
                    browser_id: browser.id.clone(),
                    host: browser.host,
                    health: browser.health,
                    pid: browser.pid,
                    has_cdp_endpoint: browser
                        .cdp_endpoint
                        .as_deref()
                        .map(|endpoint| !endpoint.is_empty())
                        .unwrap_or(false),
                    active_session_ids: sorted_strings(browser.active_session_ids.iter()),
                },
            );
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
        profile_origin: profile
            .map(|profile| profile.profile_origin)
            .unwrap_or_default(),
        profile_class: profile
            .map(|profile| profile.profile_class)
            .unwrap_or_default(),
        allocation: profile
            .map(|profile| profile.allocation)
            .unwrap_or_default(),
        keyring: profile.map(|profile| profile.keyring).unwrap_or_default(),
        browser_build: profile.and_then(|profile| profile.browser_build),
        target_service_ids: profile
            .map(|profile| sorted_strings(profile.target_service_ids.iter()))
            .unwrap_or_default(),
        authenticated_service_ids: profile
            .map(|profile| sorted_strings(profile.authenticated_service_ids.iter()))
            .unwrap_or_default(),
        account_ids: profile
            .map(|profile| sorted_strings(profile.account_ids.iter()))
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
        browser_summaries: browser_summaries.into_values().collect(),
        tab_ids: tab_ids.into_iter().collect(),
    }
}

pub(crate) fn service_site_policy_id_for_url(
    service_state: &ServiceState,
    raw_url: &str,
) -> Option<String> {
    let builtin_policies = builtin_site_policies();
    service_state
        .site_policies
        .values()
        .chain(builtin_policies.iter())
        .find(|policy| !policy.id.is_empty() && url_matches_policy_pattern(raw_url, policy))
        .map(|policy| policy.id.clone())
}

fn url_matches_policy_pattern(raw_url: &str, policy: &SitePolicy) -> bool {
    let Ok(url) = url::Url::parse(raw_url) else {
        return false;
    };
    let Ok(pattern) = url::Url::parse(&policy.origin_pattern) else {
        return false;
    };
    if url.scheme() != pattern.scheme() || url.host_str() != pattern.host_str() {
        return false;
    }
    match (url.port_or_known_default(), pattern.port_or_known_default()) {
        (left, right) if left == right => {}
        _ => return false,
    }
    let pattern_path = pattern.path();
    if pattern_path == "/" {
        return true;
    }
    let pattern_path = pattern_path.trim_end_matches('/');
    url.path() == pattern_path
        || url
            .path()
            .strip_prefix(pattern_path)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn url_origin(raw_url: &str) -> Option<String> {
    let parsed = url::Url::parse(raw_url).ok()?;
    let scheme = parsed.scheme();
    let host = parsed.host_str()?;
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Some(format!("{scheme}://{host}{port}"))
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
        .unwrap_or_else(|| default_profile_seeding_url(target));
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

pub use agent_browser_service_model::{
    interaction_decision, profile_seeding_handoff_id, provider_decision, BrowserBuild,
    BrowserHealth, BrowserHealthObservation, BrowserHost, BrowserProcess, BrowserProfile,
    BrowserRecordAuthoritySource, BrowserRecordLifecycleClassification, BrowserRecordProvenance,
    BrowserRecordSource, BrowserSession, BrowserTab, Challenge, ChallengePolicy, ChallengeState,
    ControlInputProvider, ControlPlaneSnapshot, DisplayAllocation,
    DurableHandoffPresentationReceipt, JobControlPlaneMode, JobPriority, JobState, JobTarget,
    LeaseState, MonitorState, MonitorTarget, ProfileAllocationPolicy, ProfileClass,
    ProfileKeyringPolicy, ProfileLeaseDisposition, ProfileOrigin, ProfileReadinessState,
    ProfileSeedingHandoffRecord, ProfileSeedingHandoffState, ProfileSeedingMode,
    ProfileSelectionReason, ProfileTargetReadiness, ProtectedBrowserOwnerObservation,
    RemoteViewAcquisitionLease, RemoteViewHandoff, RemoteViewRoute,
    RetainedDisplayAllocationCandidate, RoutePoolEntry, ServiceActor,
    ServiceBrowserProcessIdentity, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceIncident, ServiceIncidentEscalation, ServiceIncidentSeverity, ServiceIncidentState,
    ServiceJob, ServiceProvider, ServiceReconciliationSnapshot, ServiceTabHandle,
    SessionCleanupPolicy, SiteMonitor, SitePolicy, TabLifecycle, ViewStream, ViewStreamProvider,
    ViewerLease, SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME, SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
};
#[cfg(test)]
pub use agent_browser_service_model::{
    ChallengeKind, InteractionMode, ProfileConnectionState, ProviderCapability, ProviderKind,
    RateLimitPolicy, ServiceTabHandleTraceFilter, SERVICE_EVENT_KIND_VALUES,
    SERVICE_INCIDENT_ESCALATION_VALUES, SERVICE_INCIDENT_SEVERITY_VALUES,
    SERVICE_INCIDENT_STATE_VALUES, SERVICE_JOB_CONTROL_PLANE_MODE_VALUES,
    SERVICE_JOB_NAMING_WARNING_VALUES, SERVICE_JOB_PRIORITY_VALUES, SERVICE_JOB_STATE_VALUES,
    SERVICE_MONITOR_STATE_VALUES,
};
#[cfg(test)]
use agent_browser_service_model::{
    ProfileChildAccess, SERVICE_BROWSER_BUILD_VALUES, SERVICE_BROWSER_HEALTH_VALUES,
    SERVICE_BROWSER_HOST_VALUES, SERVICE_CHALLENGE_KIND_VALUES, SERVICE_CHALLENGE_POLICY_VALUES,
    SERVICE_CHALLENGE_STATE_VALUES, SERVICE_CONTROL_INPUT_PROVIDER_VALUES,
    SERVICE_INTERACTION_MODE_VALUES, SERVICE_LEASE_STATE_VALUES, SERVICE_PROFILE_ALLOCATION_VALUES,
    SERVICE_PROFILE_CLASS_VALUES, SERVICE_PROFILE_KEYRING_VALUES,
    SERVICE_PROFILE_LEASE_DISPOSITION_VALUES, SERVICE_PROFILE_READINESS_VALUES,
    SERVICE_PROFILE_SEEDING_MODE_VALUES, SERVICE_PROFILE_SELECTION_REASON_VALUES,
    SERVICE_PROVIDER_CAPABILITY_VALUES, SERVICE_PROVIDER_KIND_VALUES,
    SERVICE_SESSION_CLEANUP_VALUES, SERVICE_TAB_LIFECYCLE_VALUES,
    SERVICE_VIEW_STREAM_PROVIDER_VALUES,
};

pub fn retained_display_allocation_candidates(
    state: &ServiceState,
    current_boot_epoch: Option<&str>,
) -> Vec<RetainedDisplayAllocationCandidate> {
    let mut candidates = state
        .display_allocations
        .values()
        .map(|allocation| {
            classify_retained_display_allocation(state, allocation, current_boot_epoch)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    candidates
}

pub fn retained_display_allocation_summary(
    state: &ServiceState,
    current_boot_epoch: Option<&str>,
) -> Value {
    let candidates = retained_display_allocation_candidates(state, current_boot_epoch);
    let mut class_counts = BTreeMap::new();
    let mut apply_safe_ids = Vec::new();
    let mut retained_ids = Vec::new();
    let mut candidate_reasons = serde_json::Map::new();
    for candidate in &candidates {
        *class_counts.entry(candidate.class_name).or_insert(0usize) += 1;
        if candidate.apply_safe {
            apply_safe_ids.push(candidate.id.clone());
        } else {
            retained_ids.push(candidate.id.clone());
        }
        candidate_reasons.insert(candidate.id.clone(), candidate.to_json());
    }

    json!({
        "count": candidates.len(),
        "applySafeCount": apply_safe_ids.len(),
        "retainedCount": retained_ids.len(),
        "classCounts": class_counts,
        "applySafeIds": apply_safe_ids,
        "retainedIds": retained_ids,
        "candidateReasons": candidate_reasons,
        "explanation": "Retained display allocations are historical service-state records, not live control rows unless classified as live.",
        "cleanupCommand": "agent-browser service prune-retained --display-allocations --dry-run",
    })
}

fn classify_retained_display_allocation(
    state: &ServiceState,
    allocation: &DisplayAllocation,
    current_boot_epoch: Option<&str>,
) -> RetainedDisplayAllocationCandidate {
    if boot_epoch_is_prior(allocation.boot_epoch.as_deref(), current_boot_epoch) {
        return RetainedDisplayAllocationCandidate {
            id: allocation.id.clone(),
            class_name: "prior-boot-observation",
            reason: "allocation_boot_epoch_is_not_current",
            apply_safe: false,
            linked_route_ids: allocation.route_ids.clone(),
            linked_browser_ids: allocation.owner_browser_id.iter().cloned().collect(),
            linked_session_ids: allocation.owner_session_id.iter().cloned().collect(),
            linked_incident_ids: Vec::new(),
            linked_route_pool_entry_ids: Vec::new(),
        };
    }
    let mut linked_route_ids = allocation
        .route_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for (route_id, route) in &state.remote_view_routes {
        if route.display_allocation_id.as_deref() == Some(allocation.id.as_str()) {
            linked_route_ids.insert(route_id.clone());
        }
    }
    let linked_route_ids = linked_route_ids.into_iter().collect::<Vec<_>>();

    let mut linked_browser_ids = BTreeSet::new();
    if let Some(browser_id) = allocation.owner_browser_id.as_deref() {
        linked_browser_ids.insert(browser_id.to_string());
    }
    for route_id in &linked_route_ids {
        if let Some(browser_id) = state
            .remote_view_routes
            .get(route_id)
            .and_then(|route| route.browser_id.as_deref())
        {
            linked_browser_ids.insert(browser_id.to_string());
        }
    }

    let mut linked_session_ids = BTreeSet::new();
    if let Some(session_id) = allocation.owner_session_id.as_deref() {
        linked_session_ids.insert(session_id.to_string());
    }
    for route_id in &linked_route_ids {
        if let Some(session_id) = state
            .remote_view_routes
            .get(route_id)
            .and_then(|route| route.session_id.as_deref())
        {
            linked_session_ids.insert(session_id.to_string());
        }
    }

    let linked_route_id_set = linked_route_ids.iter().cloned().collect::<BTreeSet<_>>();
    let linked_route_pool_entry_ids = state
        .route_pool
        .values()
        .filter(|entry| {
            entry
                .current_route_allocation_id
                .as_ref()
                .is_some_and(|route_id| linked_route_id_set.contains(route_id))
        })
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();

    let linked_incident_ids = state
        .incidents
        .iter()
        .filter(|incident| {
            incident.state == ServiceIncidentState::Active
                && incident
                    .browser_id
                    .as_ref()
                    .is_some_and(|browser_id| linked_browser_ids.contains(browser_id))
        })
        .map(|incident| incident.id.clone())
        .collect::<Vec<_>>();

    let has_active_route_pool_checkout = state.route_pool.values().any(|entry| {
        entry
            .current_route_allocation_id
            .as_ref()
            .is_some_and(|route_id| linked_route_id_set.contains(route_id))
            && matches!(entry.state.as_str(), "checked_out" | "pending")
    });
    let has_active_route = linked_route_ids.iter().any(|route_id| {
        state
            .remote_view_routes
            .get(route_id)
            .map(|route| {
                matches!(
                    route.state.as_str(),
                    "ready" | "allocating" | "pending" | "reconnecting"
                )
            })
            .unwrap_or(false)
    });
    let has_missing_route_ref = linked_route_ids
        .iter()
        .any(|route_id| !state.remote_view_routes.contains_key(route_id));
    let has_live_browser = linked_browser_ids.iter().any(|browser_id| {
        state
            .browsers
            .get(browser_id)
            .map(|browser| {
                matches!(
                    browser.health,
                    BrowserHealth::Launching
                        | BrowserHealth::Ready
                        | BrowserHealth::Degraded
                        | BrowserHealth::Reconnecting
                        | BrowserHealth::Closing
                )
            })
            .unwrap_or(false)
    });
    let has_live_session = linked_session_ids.iter().any(|session_id| {
        state
            .sessions
            .get(session_id)
            .map(|session| !matches!(session.lease, LeaseState::Released | LeaseState::Expired))
            .unwrap_or(false)
    });
    let has_diagnostic_evidence = matches!(allocation.state.as_str(), "failed" | "unavailable")
        || allocation.readiness.is_some()
        || !linked_incident_ids.is_empty();

    let (class_name, reason, apply_safe) = if has_active_route_pool_checkout {
        (
            "live",
            "route_pool_entry_currently_references_allocation_route",
            false,
        )
    } else if has_active_route || has_live_browser || has_live_session {
        (
            "live",
            "allocation_has_active_route_browser_or_session",
            false,
        )
    } else if has_diagnostic_evidence {
        (
            "diagnostic-retained",
            "allocation_has_diagnostic_readiness_or_active_incident_evidence",
            false,
        )
    } else if has_missing_route_ref {
        ("unknown", "allocation_references_missing_route", false)
    } else if !linked_route_ids.is_empty() {
        (
            "stale-route-reference",
            "allocation_links_only_inactive_or_released_routes",
            true,
        )
    } else if allocation.owner_browser_id.is_none() && allocation.owner_session_id.is_none() {
        (
            "safe-orphan-display",
            "allocation_has_no_route_browser_or_session_references",
            true,
        )
    } else {
        (
            "historical-placeholder",
            "allocation_links_only_missing_inactive_owner_placeholders",
            true,
        )
    };

    RetainedDisplayAllocationCandidate {
        id: allocation.id.clone(),
        class_name,
        reason,
        apply_safe,
        linked_route_ids,
        linked_browser_ids: linked_browser_ids.into_iter().collect(),
        linked_session_ids: linked_session_ids.into_iter().collect(),
        linked_incident_ids,
        linked_route_pool_entry_ids,
    }
}

fn boot_epoch_is_prior(
    recorded_boot_epoch: Option<&str>,
    current_boot_epoch: Option<&str>,
) -> bool {
    matches!(
        (recorded_boot_epoch, current_boot_epoch),
        (Some(recorded), Some(current)) if recorded != current
    )
}

/// Change the primary controller once, advance its ABA fencing epoch, and
/// project that exact authority into every stream bound to the route.
pub(crate) fn advance_route_controller_authority(
    state: &mut ServiceState,
    route_id: &str,
    controller_lease_id: Option<String>,
) -> Result<u64, String> {
    let route = state
        .remote_view_routes
        .get_mut(route_id)
        .ok_or_else(|| format!("remote view route '{route_id}' not found"))?;
    let epoch = route.advance_controller(controller_lease_id);
    let route = route.clone();
    for browser in state.browsers.values_mut() {
        for stream in &mut browser.view_streams {
            if stream.route_id.as_deref() == Some(route_id) {
                stream.project_controller(&route);
            }
        }
    }
    Ok(epoch)
}

/// Check the route/stream portion of a previously observed controller fence.
/// Lease role, actor, expiry, and desktop binding remain interaction-layer
/// predicates; this helper prevents former-controller and ABA acceptance.
pub(crate) fn controller_authority_fence_matches(
    state: &ServiceState,
    route_id: &str,
    stream_id: &str,
    controller_lease_id: &str,
    controller_epoch: u64,
) -> bool {
    let Some(route) = state.remote_view_routes.get(route_id) else {
        return false;
    };
    if route.controller_lease_id.as_deref() != Some(controller_lease_id)
        || route.controller_epoch != controller_epoch
    {
        return false;
    }
    state.browsers.values().any(|browser| {
        browser.view_streams.iter().any(|stream| {
            stream.id == stream_id
                && stream.route_id.as_deref() == Some(route_id)
                && stream.controller_lease_id.as_deref() == Some(controller_lease_id)
                && stream.controller_epoch == controller_epoch
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn controller_epoch_defaults_for_legacy_records_and_fences_same_id_regrant() {
        let mut route: RemoteViewRoute = serde_json::from_value(json!({
            "id": "route-a",
            "controllerLeaseId": "controller-a"
        }))
        .unwrap();
        let stream: ViewStream = serde_json::from_value(json!({
            "id": "stream-a",
            "controllerLeaseId": "controller-a"
        }))
        .unwrap();

        assert_eq!(route.controller_epoch, 0);
        assert_eq!(stream.controller_epoch, 0);
        assert_eq!(
            route.advance_controller(Some("controller-a".to_string())),
            1
        );
        assert_eq!(route.advance_controller(None), 2);
        assert_eq!(
            route.advance_controller(Some("controller-a".to_string())),
            3
        );
    }

    #[test]
    fn controller_fence_rejects_former_controller_and_same_id_aba() {
        let route_id = "route-a".to_string();
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-a".to_string(),
                BrowserProcess {
                    id: "browser-a".to_string(),
                    view_streams: vec![ViewStream {
                        id: "stream-a".to_string(),
                        route_id: Some(route_id.clone()),
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                route_id.clone(),
                RemoteViewRoute {
                    id: route_id.clone(),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        };

        let first_epoch = advance_route_controller_authority(
            &mut state,
            &route_id,
            Some("controller-a".to_string()),
        )
        .unwrap();
        assert!(controller_authority_fence_matches(
            &state,
            &route_id,
            "stream-a",
            "controller-a",
            first_epoch,
        ));

        let takeover_epoch = advance_route_controller_authority(
            &mut state,
            &route_id,
            Some("controller-b".to_string()),
        )
        .unwrap();
        assert!(!controller_authority_fence_matches(
            &state,
            &route_id,
            "stream-a",
            "controller-a",
            first_epoch,
        ));
        assert!(controller_authority_fence_matches(
            &state,
            &route_id,
            "stream-a",
            "controller-b",
            takeover_epoch,
        ));

        let aba_epoch = advance_route_controller_authority(
            &mut state,
            &route_id,
            Some("controller-a".to_string()),
        )
        .unwrap();
        assert_ne!(aba_epoch, first_epoch);
        assert!(!controller_authority_fence_matches(
            &state,
            &route_id,
            "stream-a",
            "controller-a",
            first_epoch,
        ));
    }

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
            browser_build: Some(BrowserBuild::StealthcdpChromium),
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
            ..SitePolicy::default()
        };

        let value = serde_json::to_value(policy).unwrap();
        assert_eq!(value["browserHost"], "docker_headed");
        assert_eq!(value["browserBuild"], "stealthcdp_chromium");
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
        assert_eq!(
            schema["properties"]["displayIsolation"]["enum"],
            json!([
                "private_virtual_display",
                "shared_display",
                "ambient_display",
                serde_json::Value::Null
            ])
        );
        for field in [
            "requestedDisplayAllocationId",
            "displayAllocationId",
            "requestedRemoteViewRouteId",
            "remoteViewRouteId",
            "routePoolEntryId",
            "viewerLeaseId",
            "controllerLeaseId",
        ] {
            assert_eq!(
                schema["properties"][field]["type"],
                json!(["string", "null"])
            );
        }
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
            display_isolation: Some("private_virtual_display".to_string()),
            requested_display_allocation_id: Some("display-requested".to_string()),
            display_allocation_id: Some("display-1".to_string()),
            requested_remote_view_route_id: Some("route-requested".to_string()),
            remote_view_route_id: Some("route-1".to_string()),
            route_pool_entry_id: Some("pool-1".to_string()),
            viewer_lease_id: Some("viewer-1".to_string()),
            controller_lease_id: Some("controller-1".to_string()),
            naming_warnings: service_job_naming_warning_values(),
            has_naming_warning: true,
            ..ServiceJob::default()
        };
        let value = serde_json::to_value(job).unwrap();

        assert_service_job_naming_warning_contract(&value);
        assert_eq!(value["displayIsolation"], "private_virtual_display");
        assert_eq!(value["requestedDisplayAllocationId"], "display-requested");
        assert_eq!(value["displayAllocationId"], "display-1");
        assert_eq!(value["requestedRemoteViewRouteId"], "route-requested");
        assert_eq!(value["remoteViewRouteId"], "route-1");
        assert_eq!(value["routePoolEntryId"], "pool-1");
        assert_eq!(value["viewerLeaseId"], "viewer-1");
        assert_eq!(value["controllerLeaseId"], "controller-1");
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
            provenance: None,
            terminal_outcome: None,
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
        let display_allocation_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-display-allocation-record.v1.schema.json"
        ))
        .unwrap();
        let remote_view_route_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-remote-view-route-record.v1.schema.json"
        ))
        .unwrap();
        let route_pool_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-route-pool-entry-record.v1.schema.json"
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
            browser_schema["properties"]["viewStreams"]["items"]["properties"]["provider"]["enum"],
            json!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.to_vec())
        );
        assert_eq!(
            browser_schema["properties"]["viewStreams"]["items"]["properties"]["controlInput"]
                ["oneOf"][0]["enum"],
            json!(SERVICE_CONTROL_INPUT_PROVIDER_VALUES.to_vec())
        );
        assert_eq!(
            display_allocation_schema["properties"]["host"]["type"],
            json!(["string", "null"])
        );
        assert_eq!(
            remote_view_route_schema["properties"]["provider"]["enum"],
            json!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.to_vec())
        );
        assert_eq!(
            route_pool_schema["properties"]["provider"]["enum"],
            json!(SERVICE_VIEW_STREAM_PROVIDER_VALUES.to_vec())
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
            site_policy_schema["properties"]["browserBuild"]["oneOf"][0]["enum"],
            json!(SERVICE_BROWSER_BUILD_VALUES.to_vec())
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
                "accountIds",
                "defaultBrowserHost",
                "browserBuild",
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
                "browserCapabilityLaunch",
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
            browser_build: Some(BrowserBuild::StockChrome),
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
                "provenance": null,
                "terminalOutcome": null,
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
                "provenance": null,
                "terminalOutcome": null,
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
            "provenance": {
                "schemaVersion": "agent-browser.service-request-provenance.v1",
                "requestId": "request-1",
                "jobId": "job-1",
                "traceId": null,
                "causedByRequestId": null,
                "clientSubjectId": null,
                "identityAssurance": "unknown",
                "connectionInstanceId": null,
                "runtimeEnvironmentId": null,
                "runtimeLaneId": null,
                "profileId": null,
                "profileResourceKey": null,
                "browserId": null,
                "sessionId": null,
                "tabId": null,
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
                "action": "navigate",
                "policyRevision": null,
                "accessDecisionId": null
            },
            "terminalOutcome": null,
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
            "profileOrigin": "agent_browser_owned",
            "profileClass": "durable_named",
            "userDataDir": null,
            "sitePolicyIds": [],
            "targetServiceIds": ["acs"],
            "authenticatedServiceIds": [],
            "accountIds": [],
            "defaultBrowserHost": null,
            "browserBuild": null,
            "allocation": "per_service",
            "keyring": "basic_password_store",
            "sharedServiceIds": [],
            "credentialProviderIds": [],
            "manualLoginPreferred": false,
            "targetReadiness": [],
            "registration": null,
            "browserCompatibilityEvidence": [],
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
            "browserCapabilityLaunch": null,
            "cleanup": "close_browser",
            "browserIds": [],
            "tabIds": [],
            "createdAt": null,
            "lastLeaseObservedAt": null,
            "expiresAt": null,
        });
        let site_policy = json!({
            "id": "google",
            "originPattern": "https://accounts.google.com",
            "browserHost": null,
            "browserBuild": null,
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
            "controlPlaneMode": "cdp",
            "lifecycleOnly": false,
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
            "displayIsolation": null,
            "displayName": null,
            "displayAllocationId": null,
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
                    "displayIsolation": null,
                    "displayName": null,
                    "displayAllocationId": null,
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
                "expiredSessionLeases",
                "expiredSessionLeaseCount",
                "remoteViewRepair",
                "service_state",
            ],
        );

        assert_service_reconcile_response_contract(&json!({
            "reconciled": true,
            "browserCount": 1,
            "changedBrowsers": 1,
            "expiredSessionLeases": [],
            "expiredSessionLeaseCount": 0,
            "remoteViewRepair": {
                "unavailableRoutePoolEntries": 0,
                "restoredRoutePoolEntries": 0,
                "orphanedDisplayAllocations": 0,
                "orphanedRoutes": 0,
                "releasedViewerLeases": 0,
                "expiredViewerLeases": 0,
                "clearedControllerLeases": 0,
                "repaired": 0,
                "released": 0,
                "skippedUnsafe": 0,
            },
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
                    "../../../docs/dev/contracts/service-display-allocations-response.v1.schema.json"
                ))
                .unwrap(),
                "displayAllocations",
                "display allocations response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-remote-view-routes-response.v1.schema.json"
                ))
                .unwrap(),
                "remoteViewRoutes",
                "remote view routes response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-route-pool-response.v1.schema.json"
                ))
                .unwrap(),
                "routePool",
                "route pool response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-viewer-leases-response.v1.schema.json"
                ))
                .unwrap(),
                "viewerLeases",
                "viewer leases response",
            ),
            (
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../docs/dev/contracts/service-profile-leases-response.v1.schema.json"
                ))
                .unwrap(),
                "profileLeases",
                "profile leases response",
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
        assert!(status_schema["properties"]
            .get("retainedDisplayAllocations")
            .is_some());
        assert_service_status_response_contract(&json!({
            "control_plane": {
                "worker_state": "ready",
                "browser_health": "ready",
                "queue_depth": 0,
                "queue_capacity": 64,
                "waiting_profile_lease_job_count": 0,
                "service_job_timeout_ms": null,
                "service_monitor_interval_ms": 60000
            },
            "service_state": {},
            "profileAllocations": [],
            "manualBrowsers": [],
            "browserSessionAuthority": {
                "schemaVersion": 1,
                "availability": "unknown",
                "summary": {
                    "modeledBrowserCount": 0,
                    "viableBrowserCount": 0,
                    "attentionBrowserCount": 0,
                    "nonViableBrowserCount": 0,
                    "unknownBrowserCount": 0
                },
                "resourcePressure": {},
                "browserVerdicts": []
            },
            "statusProjection": {
                "schemaVersion": 1,
                "authority": {
                    "source": "reconciled_service_state",
                    "projectedAt": "2026-08-09T21:00:05.000Z"
                },
                "observations": {
                    "state": "unavailable"
                }
            },
            "serviceStateLockDiagnostics": {
                "schemaVersion": "agent-browser.service-state-lock-diagnostics.v1",
                "recentCapacity": 32,
                "active": [],
                "recent": [],
                "counters": {
                    "processAcquisitions": 0,
                    "fileAcquisitions": 0,
                    "processTimeouts": 0,
                    "fileTimeouts": 0,
                    "processPoisonRecoveries": 0
                }
            },
        }));

        for (schema, field, label) in collection_schemas {
            if field == "profiles" {
                assert_schema_required_fields(
                    &schema,
                    &[field, "profileSources", "profileAllocations", "count"],
                );
            } else if field == "sitePolicies" {
                assert_schema_required_fields(&schema, &[field, "sitePolicySources", "count"]);
            } else if field == "profileLeases" {
                assert_schema_required_fields(&schema, &[field, "count", "observedAt", "doctor"]);
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
            } else if field == "profileLeases" {
                json!({
                    field: [],
                    "count": 0,
                    "observedAt": "2026-08-27T12:00:00Z",
                    "doctor": {},
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
                "browserCapabilityLaunches",
                "displayAllocations",
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
            "browserCapabilityLaunches": {
                "count": 0,
                "appliedCount": 0,
                "skippedCount": 0,
                "launches": [],
            },
            "displayAllocations": {
                "count": 1,
                "recordedCount": 1,
                "unrecordedCount": 0,
                "privateVirtualDisplayCount": 1,
                "sharedDisplayCount": 0,
                "ambientDisplayCount": 0,
                "allocations": [{
                    "displayIsolation": "private_virtual_display",
                    "label": "private display",
                    "count": 1,
                    "jobIds": ["job-1"],
                }],
                "unrecordedJobIds": [],
            },
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
                "controlPlaneModes": ["cdp"],
                "displayAllocations": ["private_virtual_display"],
                "unrecordedDisplayAllocationJobCount": 0,
                "lifecycleOnlyJobCount": 0,
                "attention": {
                    "required": false,
                    "owner": "none",
                    "severity": "info",
                    "reason": "none",
                    "message": "No trace-context intervention is required.",
                    "suggestedActions": [],
                    "presentation": "client_decides",
                },
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
    fn service_state_round_trips_durable_remote_view_handoffs() {
        let handoff = RemoteViewHandoff {
            id: "job-handoff-a".to_string(),
            state: "ready".to_string(),
            intent: json!({
                "url": "https://example.com/article",
                "profile": "shared-social",
                "viewStreamProvider": "rdp_gateway",
                "controlInput": "manual_attached_desktop"
            }),
            handoff_url: Some("https://view.example/remote-view/job-handoff-a".to_string()),
            desired_url: Some("https://example.com/article".to_string()),
            profile_id: Some("shared-social".to_string()),
            browser_id: Some("browser-a".to_string()),
            session_name: Some("session-a".to_string()),
            tab_id: Some("tab-a".to_string()),
            target_id: Some("target-a".to_string()),
            view_stream_provider: Some(ViewStreamProvider::RdpGateway),
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            last_route_id: Some("route-a".to_string()),
            last_route_pool_entry_id: Some("pool-a".to_string()),
            last_display_allocation_id: Some("display-a".to_string()),
            created_at: Some("2026-08-07T12:00:00Z".to_string()),
            updated_at: Some("2026-08-07T12:01:00Z".to_string()),
            last_resolved_at: Some("2026-08-07T12:01:00Z".to_string()),
            last_resolution: Some(json!({"status": "ready"})),
            presentation_receipt: None,
        };
        let state = ServiceState {
            remote_view_handoffs: BTreeMap::from([(handoff.id.clone(), handoff)]),
            ..ServiceState::default()
        };

        let encoded = serde_json::to_value(&state).expect("service state should serialize");
        let decoded: ServiceState =
            serde_json::from_value(encoded.clone()).expect("service state should deserialize");

        assert_eq!(decoded, state);
        assert_eq!(
            encoded["remoteViewHandoffs"]["job-handoff-a"]["viewStreamProvider"],
            "rdp_gateway"
        );
        assert_eq!(
            encoded["remoteViewHandoffs"]["job-handoff-a"]["handoffUrl"],
            "https://view.example/remote-view/job-handoff-a"
        );
    }

    #[test]
    fn service_state_round_trips_profile_recovery_and_reset_receipts() {
        use super::super::service_profile_acquisition::{
            ProfileAcquisitionState, ProfileResetReceipt, ProfileResetScope, RecoveryReceipt,
            PROFILE_RECOVERY_RECEIPT_SCHEMA_V1, PROFILE_RESET_RECEIPT_SCHEMA_V1,
        };

        let recovery_receipt = RecoveryReceipt {
            schema_version: PROFILE_RECOVERY_RECEIPT_SCHEMA_V1.to_string(),
            recovery_id: "recovery-1".to_string(),
            plan_id: "recovery-plan-1".to_string(),
            principal_id: "principal-1".to_string(),
            profile_id: "profile-1".to_string(),
            producer_build_identity: None,
            terminal_result: "applied".to_string(),
            precondition_comparison: "matched".to_string(),
            attempted_operation_ids: vec!["operation-1".to_string()],
            compensation_result: "not_required".to_string(),
            final_state_revision: 7,
            acquisition_retry_state: ProfileAcquisitionState::Acquired,
            browser_id: "browser-1".to_string(),
            daemon_session_route: "session-1".to_string(),
        };
        let reset_receipt = ProfileResetReceipt {
            schema_version: PROFILE_RESET_RECEIPT_SCHEMA_V1.to_string(),
            reset_id: "reset-1".to_string(),
            plan_id: "reset-plan-1".to_string(),
            principal_id: "principal-1".to_string(),
            profile_id: "profile-1".to_string(),
            producer_build_identity: json!({"version": "test-build"}),
            scope: ProfileResetScope::Authentication,
            target_service_id: None,
            terminal_result: "applied".to_string(),
            applied_at: "2026-09-17T12:00:00Z".to_string(),
            final_state_revision: 8,
            browser_cookies_erased: false,
            seeding_handoff: None,
        };
        let state = ServiceState {
            profile_recovery_receipts: BTreeMap::from([(
                recovery_receipt.recovery_id.clone(),
                recovery_receipt,
            )]),
            profile_reset_receipts: BTreeMap::from([(
                reset_receipt.reset_id.clone(),
                reset_receipt,
            )]),
            ..ServiceState::default()
        };

        let encoded = serde_json::to_value(&state).expect("service state should serialize");
        let decoded: ServiceState =
            serde_json::from_value(encoded.clone()).expect("service state should deserialize");

        assert_eq!(decoded, state);
        assert_eq!(
            encoded["profileRecoveryReceipts"]["recovery-1"]["acquisitionRetryState"],
            "acquired"
        );
        assert!(encoded["profileRecoveryReceipts"]["recovery-1"]
            .get("producerBuildIdentity")
            .is_none());
        assert_eq!(
            encoded["profileResetReceipts"]["reset-1"]["scope"],
            "authentication"
        );
        assert!(encoded["profileResetReceipts"]["reset-1"]
            .get("targetServiceId")
            .is_none());
        assert!(encoded["profileResetReceipts"]["reset-1"]
            .get("seedingHandoff")
            .is_none());
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
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    display_name: Some(":41".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    owner_session_id: Some("session-1".to_string()),
                    state: "ready".to_string(),
                    route_ids: vec!["route-1".to_string()],
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    display_allocation_id: Some("display-1".to_string()),
                    browser_id: Some("browser-1".to_string()),
                    session_id: Some("session-1".to_string()),
                    state: "ready".to_string(),
                    provider_mode: "simultaneous_view".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-1".to_string(),
                RoutePoolEntry {
                    id: "pool-1".to_string(),
                    route_id: "route-1".to_string(),
                    target: json!({ "displayName": ":41" }),
                    state: "available".to_string(),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_acquisition_leases: BTreeMap::from([(
                "remote-view-open-session-1-route-1".to_string(),
                RemoteViewAcquisitionLease {
                    id: "remote-view-open-session-1-route-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    session_id: "session-1".to_string(),
                    route_id: "route-1".to_string(),
                    display_allocation_id: "display-1".to_string(),
                    route_pool_entry_id: Some("pool-1".to_string()),
                    state: "failed".to_string(),
                    phase: "rollback_complete".to_string(),
                    failure_reason: Some("proof_failed: forced proof failure".to_string()),
                    cleanup: Some(json!({
                        "state": "rolled_back",
                        "phase": "proof_failed"
                    })),
                    previous_route_pool_entry: Some(RoutePoolEntry {
                        id: "pool-1".to_string(),
                        route_id: "route-1".to_string(),
                        target: json!({ "displayName": ":41" }),
                        state: "available".to_string(),
                        readiness: Some(json!({ "state": "ready" })),
                        ..RoutePoolEntry::default()
                    }),
                    previous_display_allocation: Some(DisplayAllocation {
                        id: "display-1".to_string(),
                        display_name: Some(":41".to_string()),
                        display_isolation: "shared_display".to_string(),
                        owner_browser_id: Some("browser-1".to_string()),
                        owner_session_id: Some("session-1".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-1".to_string()],
                        ..DisplayAllocation::default()
                    }),
                    previous_remote_view_route: Some(RemoteViewRoute {
                        id: "route-1".to_string(),
                        display_allocation_id: Some("display-1".to_string()),
                        browser_id: Some("browser-1".to_string()),
                        session_id: Some("session-1".to_string()),
                        state: "ready".to_string(),
                        provider_mode: "simultaneous_view".to_string(),
                        ..RemoteViewRoute::default()
                    }),
                    previous_browser_display_allocation_id: Some("display-1".to_string()),
                    ..RemoteViewAcquisitionLease::default()
                },
            )]),
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![json!({
                    "id": "local-linux",
                    "name": "Local Linux desktop",
                    "hostKind": "local",
                    "operatingSystem": "linux",
                    "displaySupport": "x11",
                    "remoteViewSupport": false,
                    "reachable": true,
                    "lifecycleOwner": "agent_browser",
                    "tags": ["default"]
                })],
                browser_executables: vec![json!({
                    "id": "stealthcdp-current",
                    "hostId": "local-linux",
                    "browserFamily": "chromium",
                    "vendor": "chromium",
                    "channel": "custom",
                    "buildLabel": "stealthcdp_chromium",
                    "executablePath": "/opt/agent-browser/chromium-stealthcdp/chrome",
                    "source": "manifest",
                    "version": "136.0.0.0",
                    "tags": ["preferred"]
                })],
                browser_capabilities: vec![json!({
                    "id": "stealthcdp-current:local-linux",
                    "hostId": "local-linux",
                    "executableId": "stealthcdp-current",
                    "cdpSupported": true,
                    "cdpFreeLaunchSupported": true,
                    "extensionsSupported": true,
                    "passkeysSupported": true,
                    "headedSupported": true,
                    "headlessSupported": true,
                    "streamingSupported": false,
                    "profileLockBehavior": "exclusive_process",
                    "keyringBehavior": "basic_password_store_preferred",
                    "knownLimits": []
                })],
                generated_at: Some("2026-05-13T00:00:00Z".to_string()),
                ..BrowserCapabilityRegistry::default()
            },
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
        assert_eq!(
            decoded.remote_view_acquisition_leases["remote-view-open-session-1-route-1"].state,
            "failed"
        );
        let encoded_value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert!(encoded_value["remoteViewAcquisitionLeases"].is_object());
        assert!(encoded_value
            .get("remote_view_acquisition_leases")
            .is_none());
        assert_service_remote_view_acquisition_lease_record_contract(
            &encoded_value["remoteViewAcquisitionLeases"]["remote-view-open-session-1-route-1"],
        );
        assert_eq!(
            decoded.browser_capability_registry.browser_hosts[0]["hostKind"],
            "local"
        );
        assert_eq!(
            decoded.browser_capability_registry.browser_executables[0]["buildLabel"],
            "stealthcdp_chromium"
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
    fn refresh_derived_views_groups_remote_view_route_failures() {
        let mut state = ServiceState {
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    state: "orphaned".to_string(),
                    updated_at: Some("2026-05-28T00:00:00Z".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-display-missing".to_string(),
                    RemoteViewRoute {
                        id: "route-display-missing".to_string(),
                        display_allocation_id: Some("display-missing".to_string()),
                        state: "orphaned".to_string(),
                        readiness: Some(json!({
                            "reason": "display_allocation_missing",
                            "state": "orphaned"
                        })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-auth".to_string(),
                    RemoteViewRoute {
                        id: "route-auth".to_string(),
                        display_allocation_id: Some("display-1".to_string()),
                        state: "degraded".to_string(),
                        readiness: Some(json!({
                            "component": "provider_auth",
                            "status": "failed",
                            "evidence": "Guacamole login rejected the dashboard session"
                        })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-frame".to_string(),
                    RemoteViewRoute {
                        id: "route-frame".to_string(),
                        display_allocation_id: Some("display-1".to_string()),
                        state: "degraded".to_string(),
                        readiness: Some(json!({
                            "component": "iframe",
                            "status": "blocked",
                            "evidence": "x-frame-options denied"
                        })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-released".to_string(),
                    RemoteViewRoute {
                        id: "route-released".to_string(),
                        display_allocation_id: Some("display-1".to_string()),
                        state: "released".to_string(),
                        readiness: Some(json!({
                            "reason": "stale_route_pool_checkout_repaired",
                            "state": "released"
                        })),
                        ..RemoteViewRoute::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        let kinds = state
            .incidents
            .iter()
            .map(|incident| incident.latest_kind.as_str())
            .collect::<BTreeSet<_>>();
        assert!(kinds.contains("remote_view_display_missing"));
        assert!(kinds.contains("remote_view_provider_auth_failed"));
        assert!(kinds.contains("remote_view_iframe_blocked"));
        assert!(!state
            .incidents
            .iter()
            .any(|incident| incident.id == "remote-view-route:route-released"));
        for incident in &state.incidents {
            assert_eq!(incident.state, ServiceIncidentState::Service);
            assert_eq!(incident.severity, ServiceIncidentSeverity::Error);
            assert_eq!(
                incident.escalation,
                ServiceIncidentEscalation::ServiceTriage
            );
            assert!(incident
                .recommended_action
                .contains("remote-view route readiness"));
        }
    }

    #[test]
    fn refresh_derived_views_distinguishes_route_bound_finalization_states() {
        let mut state = ServiceState {
            display_allocations: BTreeMap::from([
                (
                    "display-finalized".to_string(),
                    DisplayAllocation {
                        id: "display-finalized".to_string(),
                        state: "ready".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-pending-browser-missing".to_string(),
                    DisplayAllocation {
                        id: "display-pending-browser-missing".to_string(),
                        state: "pending".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-pending-completed".to_string(),
                    DisplayAllocation {
                        id: "display-pending-completed".to_string(),
                        state: "pending".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-rolled-back".to_string(),
                    DisplayAllocation {
                        id: "display-rolled-back".to_string(),
                        state: "released".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-released".to_string(),
                    DisplayAllocation {
                        id: "display-released".to_string(),
                        state: "released".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-completed-released".to_string(),
                    DisplayAllocation {
                        id: "display-completed-released".to_string(),
                        state: "released".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
            ]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-finalized".to_string(),
                    RemoteViewRoute {
                        id: "route-finalized".to_string(),
                        display_allocation_id: Some("display-finalized".to_string()),
                        state: "ready".to_string(),
                        readiness: Some(json!({ "state": "ready" })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-pending-browser-missing".to_string(),
                    RemoteViewRoute {
                        id: "route-pending-browser-missing".to_string(),
                        display_allocation_id: Some("display-pending-browser-missing".to_string()),
                        state: "pending".to_string(),
                        readiness: Some(json!({
                            "state": "pending",
                            "component": "remote_view_open_acquisition"
                        })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-completed-display-pending".to_string(),
                    RemoteViewRoute {
                        id: "route-completed-display-pending".to_string(),
                        display_allocation_id: Some("display-pending-completed".to_string()),
                        state: "orphaned".to_string(),
                        readiness: Some(json!({
                            "state": "orphaned",
                            "reason": "display_allocation_unavailable"
                        })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-missing-display".to_string(),
                    RemoteViewRoute {
                        id: "route-missing-display".to_string(),
                        display_allocation_id: Some("display-missing".to_string()),
                        state: "orphaned".to_string(),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-rolled-back".to_string(),
                    RemoteViewRoute {
                        id: "route-rolled-back".to_string(),
                        display_allocation_id: Some("display-rolled-back".to_string()),
                        state: "released".to_string(),
                        readiness: Some(json!({ "state": "released" })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-released".to_string(),
                    RemoteViewRoute {
                        id: "route-released".to_string(),
                        display_allocation_id: Some("display-released".to_string()),
                        state: "released".to_string(),
                        readiness: Some(json!({ "state": "released" })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-completed-released".to_string(),
                    RemoteViewRoute {
                        id: "route-completed-released".to_string(),
                        display_allocation_id: Some("display-completed-released".to_string()),
                        state: "released".to_string(),
                        readiness: Some(json!({ "state": "released" })),
                        ..RemoteViewRoute::default()
                    },
                ),
            ]),
            route_pool: BTreeMap::from([
                (
                    "pool-finalized".to_string(),
                    RoutePoolEntry {
                        id: "pool-finalized".to_string(),
                        route_id: "route-finalized".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("route-finalized".to_string()),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-completed-display-pending".to_string(),
                    RoutePoolEntry {
                        id: "pool-completed-display-pending".to_string(),
                        route_id: "route-completed-display-pending".to_string(),
                        state: "pending".to_string(),
                        current_route_allocation_id: Some(
                            "route-completed-display-pending".to_string(),
                        ),
                        readiness: Some(json!({
                            "state": "pending",
                            "component": "remote_view_open_acquisition"
                        })),
                        ..RoutePoolEntry::default()
                    },
                ),
            ]),
            remote_view_acquisition_leases: BTreeMap::from([
                (
                    "lease-finalized".to_string(),
                    RemoteViewAcquisitionLease {
                        id: "lease-finalized".to_string(),
                        route_id: "route-finalized".to_string(),
                        display_allocation_id: "display-finalized".to_string(),
                        route_pool_entry_id: Some("pool-finalized".to_string()),
                        state: "completed".to_string(),
                        phase: "checked_out".to_string(),
                        ..RemoteViewAcquisitionLease::default()
                    },
                ),
                (
                    "lease-pending-browser-missing".to_string(),
                    RemoteViewAcquisitionLease {
                        id: "lease-pending-browser-missing".to_string(),
                        route_id: "route-pending-browser-missing".to_string(),
                        display_allocation_id: "display-pending-browser-missing".to_string(),
                        state: "pending".to_string(),
                        phase: "reserved".to_string(),
                        ..RemoteViewAcquisitionLease::default()
                    },
                ),
                (
                    "lease-completed-display-pending".to_string(),
                    RemoteViewAcquisitionLease {
                        id: "lease-completed-display-pending".to_string(),
                        route_id: "route-completed-display-pending".to_string(),
                        display_allocation_id: "display-pending-completed".to_string(),
                        route_pool_entry_id: Some("pool-completed-display-pending".to_string()),
                        state: "completed".to_string(),
                        phase: "checked_out".to_string(),
                        ..RemoteViewAcquisitionLease::default()
                    },
                ),
                (
                    "lease-rolled-back".to_string(),
                    RemoteViewAcquisitionLease {
                        id: "lease-rolled-back".to_string(),
                        route_id: "route-rolled-back".to_string(),
                        display_allocation_id: "display-rolled-back".to_string(),
                        state: "failed".to_string(),
                        phase: "rollback_complete".to_string(),
                        ..RemoteViewAcquisitionLease::default()
                    },
                ),
                (
                    "lease-completed-released".to_string(),
                    RemoteViewAcquisitionLease {
                        id: "lease-completed-released".to_string(),
                        route_id: "route-completed-released".to_string(),
                        display_allocation_id: "display-completed-released".to_string(),
                        state: "completed".to_string(),
                        phase: "checked_out".to_string(),
                        ..RemoteViewAcquisitionLease::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        assert!(!state
            .incidents
            .iter()
            .any(|incident| incident.id == "remote-view-route:route-finalized"));
        assert!(!state
            .incidents
            .iter()
            .any(|incident| incident.id == "remote-view-route:route-rolled-back"));
        assert!(!state
            .incidents
            .iter()
            .any(|incident| incident.id == "remote-view-route:route-released"));
        assert!(!state
            .incidents
            .iter()
            .any(|incident| incident.id == "remote-view-route:route-completed-released"));

        let pending = state
            .incidents
            .iter()
            .find(|incident| incident.id == "remote-view-route:route-pending-browser-missing")
            .unwrap();
        assert_eq!(pending.latest_kind, "remote_view_route_unreachable");

        let missing = state
            .incidents
            .iter()
            .find(|incident| incident.id == "remote-view-route:route-missing-display")
            .unwrap();
        assert_eq!(missing.latest_kind, "remote_view_display_missing");

        let finalization_route = state
            .incidents
            .iter()
            .find(|incident| incident.id == "remote-view-route:route-completed-display-pending")
            .unwrap();
        assert_eq!(
            finalization_route.latest_kind,
            "remote_view_finalization_incomplete"
        );

        let finalization_pool = state
            .incidents
            .iter()
            .find(|incident| incident.id == "remote-view-route-pool:pool-completed-display-pending")
            .unwrap();
        assert_eq!(
            finalization_pool.latest_kind,
            "remote_view_finalization_incomplete"
        );
    }

    #[test]
    fn refresh_derived_views_groups_route_pool_exhaustion_and_unreachable_entries() {
        let mut state = ServiceState {
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    display_isolation: "private_virtual_display".to_string(),
                    display_name: Some(":91".to_string()),
                    owner_browser_id: Some("browser-1".to_string()),
                    state: "ready".to_string(),
                    updated_at: Some("2026-05-28T00:01:00Z".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-busy".to_string(),
                RoutePoolEntry {
                    id: "pool-busy".to_string(),
                    route_id: "route-busy".to_string(),
                    target: json!({ "displayName": ":91" }),
                    state: "degraded".to_string(),
                    readiness: Some(json!({
                        "component": "rdp_backend",
                        "status": "failed",
                        "evidence": "target unreachable"
                    })),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        let route_pool = state
            .incidents
            .iter()
            .find(|incident| incident.id == "remote-view-route-pool:pool-busy")
            .unwrap();
        assert_eq!(route_pool.latest_kind, "remote_view_route_unreachable");
        let exhausted = state
            .incidents
            .iter()
            .find(|incident| incident.id == "remote-view-route-pool-exhausted:display-1")
            .unwrap();
        assert_eq!(exhausted.latest_kind, "remote_view_route_pool_exhausted");
        assert_eq!(exhausted.browser_id.as_deref(), Some("browser-1"));
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
        assert_eq!(state.incidents[0].state, ServiceIncidentState::Recovered);
        assert_eq!(state.incidents[0].current_health, None);
        assert_eq!(state.incidents[0].severity, ServiceIncidentSeverity::Info);
        assert_eq!(
            state.incidents[0].escalation,
            ServiceIncidentEscalation::None
        );
        assert_eq!(
            state.incidents[0].recommended_action,
            "No operator action required."
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
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![json!({
                    "id": "configured-host",
                    "name": "Configured browser host"
                })],
                ..BrowserCapabilityRegistry::default()
            },
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
        assert_eq!(
            persisted.browser_capability_registry.browser_hosts[0]["id"],
            "configured-host"
        );
    }

    #[test]
    fn configured_profile_preserves_persisted_freshness_evidence() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "Persisted BILL".to_string(),
                    allocation: ProfileAllocationPolicy::PerService,
                    target_service_ids: vec!["bill".to_string()],
                    authenticated_service_ids: vec!["bill".to_string()],
                    account_ids: vec!["soylei".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "bill".to_string(),
                        state: ProfileReadinessState::Fresh,
                        evidence: "authenticated_bill_home".to_string(),
                        recommended_action: "use_profile".to_string(),
                        last_verified_at: Some("2026-09-09T16:40:40Z".to_string()),
                        ..ProfileTargetReadiness::default()
                    }],
                    shared_service_ids: vec![
                        "BooksReceipts".to_string(),
                        "books-receipts".to_string(),
                    ],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let configured = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "Configured BILL".to_string(),
                    allocation: ProfileAllocationPolicy::SharedService,
                    target_service_ids: vec!["bill".to_string()],
                    shared_service_ids: vec![
                        "BooksReceipts".to_string(),
                        "books-receipts".to_string(),
                    ],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.overlay_configured_entities(configured);

        let profile = &state.profiles["bill-soylei"];
        assert_eq!(profile.name, "Configured BILL");
        assert_eq!(profile.allocation, ProfileAllocationPolicy::SharedService);
        assert_eq!(profile.account_ids, vec!["soylei"]);
        assert_eq!(profile.authenticated_service_ids, vec!["bill"]);
        assert_eq!(profile.target_readiness.len(), 1);
        assert_eq!(
            profile.target_readiness[0].evidence,
            "authenticated_bill_home"
        );
        assert_eq!(
            state.profile_source("bill-soylei"),
            Some(ServiceEntitySource::Config)
        );
    }

    #[test]
    fn configured_provider_inventory_preserves_checked_out_runtime_ownership() {
        let mut persisted = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    display_name: Some(":12".to_string()),
                    owner_browser_id: Some("browser-1".to_string()),
                    owner_session_id: Some("scene-1".to_string()),
                    route_ids: vec!["route-1".to_string()],
                    readiness: Some(json!({"state": "ready", "source": "checkout"})),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    session_id: Some("scene-1".to_string()),
                    route_source: "pool".to_string(),
                    state: "ready".to_string(),
                    last_provider_event: Some("route_checked_out".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "slot-1".to_string(),
                RoutePoolEntry {
                    id: "slot-1".to_string(),
                    route_id: "route-1".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-1".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        };
        let configured = ServiceState {
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    display_name: Some(":12".to_string()),
                    route_ids: vec!["route-1".to_string()],
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    route_source: "provider_inventory".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "slot-1".to_string(),
                RoutePoolEntry {
                    id: "slot-1".to_string(),
                    route_id: "route-1".to_string(),
                    state: "available".to_string(),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        };

        persisted.overlay_configured_entities(configured.clone());

        let display = &persisted.display_allocations["display-1"];
        assert_eq!(display.owner_browser_id.as_deref(), Some("browser-1"));
        assert_eq!(display.owner_session_id.as_deref(), Some("scene-1"));
        assert_eq!(
            persisted.remote_view_routes["route-1"]
                .session_id
                .as_deref(),
            Some("scene-1")
        );
        assert_eq!(persisted.route_pool["slot-1"].state, "checked_out");
        assert_eq!(
            persisted.route_pool["slot-1"]
                .current_route_allocation_id
                .as_deref(),
            Some("route-1")
        );

        persisted.browsers.remove("browser-1");
        persisted
            .remote_view_routes
            .get_mut("route-1")
            .unwrap()
            .state = "orphaned".to_string();
        persisted.overlay_configured_entities(configured);

        assert_eq!(persisted.remote_view_routes["route-1"].browser_id, None);
        assert_eq!(
            persisted.display_allocations["display-1"].owner_browser_id,
            None
        );
        assert_eq!(persisted.route_pool["slot-1"].state, "available");
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
        assert_eq!(state.site_policies["google"].browser_build, None);
        let builtin_google = builtin_site_policy("google").unwrap();
        assert_eq!(
            builtin_google.browser_build,
            Some(BrowserBuild::StealthcdpChromium)
        );
        assert!(!builtin_google.manual_login_preferred);
        assert_eq!(
            state.site_policies["gmail"].browser_build,
            Some(BrowserBuild::StealthcdpChromium)
        );
        assert!(!state.site_policies["gmail"].manual_login_preferred);
        assert_eq!(
            state.site_policies["google_sheets"].browser_build,
            Some(BrowserBuild::StealthcdpChromium)
        );
        assert_eq!(
            state.site_policies["google_sheets"].browser_host,
            Some(BrowserHost::RemoteHeaded)
        );
        assert_eq!(
            service_site_policy_id_for_url(
                &state,
                "https://docs.google.com/spreadsheets/d/example/edit"
            ),
            Some("google_sheets".to_string())
        );
        assert_ne!(
            service_site_policy_id_for_url(&state, "https://docs.google.com/document/d/example"),
            Some("google_sheets".to_string())
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
    fn refresh_profile_readiness_allows_stealth_cdp_first_login() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "google-stealth".to_string(),
                BrowserProfile {
                    id: "google-stealth".to_string(),
                    name: "Google Stealth".to_string(),
                    target_service_ids: vec!["google".to_string()],
                    browser_build: Some(BrowserBuild::StealthcdpChromium),
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    manual_login_preferred: true,
                    ..SitePolicy::default()
                },
            )]),
            default_browser_build: Some(BrowserBuild::StealthcdpChromium),
            ..ServiceState::default()
        };

        state.refresh_profile_readiness();

        let readiness = &state.profiles["google-stealth"].target_readiness[0];
        assert_eq!(readiness.state, ProfileReadinessState::Unknown);
        assert!(!readiness.manual_seeding_required);
        assert_eq!(readiness.seeding_mode, ProfileSeedingMode::AttachableOk);
        assert!(readiness.cdp_attachment_allowed_during_seeding);
        assert_eq!(
            readiness.recommended_action,
            "verify_or_seed_profile_before_authenticated_work"
        );
    }

    #[test]
    fn refresh_profile_readiness_preserves_required_cdp_free_first_login() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "strict-login".to_string(),
                BrowserProfile {
                    id: "strict-login".to_string(),
                    name: "Strict Login".to_string(),
                    target_service_ids: vec!["strict-login".to_string()],
                    browser_build: Some(BrowserBuild::StealthcdpChromium),
                    ..BrowserProfile::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "strict-login".to_string(),
                SitePolicy {
                    id: "strict-login".to_string(),
                    manual_login_preferred: true,
                    requires_cdp_free: true,
                    ..SitePolicy::default()
                },
            )]),
            default_browser_build: Some(BrowserBuild::StealthcdpChromium),
            ..ServiceState::default()
        };

        state.refresh_profile_readiness();

        let readiness = &state.profiles["strict-login"].target_readiness[0];
        assert_eq!(readiness.state, ProfileReadinessState::NeedsManualSeeding);
        assert!(readiness.manual_seeding_required);
        assert_eq!(
            readiness.seeding_mode,
            ProfileSeedingMode::DetachedHeadedNoCdp
        );
        assert!(!readiness.cdp_attachment_allowed_during_seeding);
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
    fn refresh_service_tab_handles_derives_valid_and_stale_handles() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "profile-1".to_string(),
                BrowserProfile {
                    id: "profile-1".to_string(),
                    name: "Profile 1".to_string(),
                    profile_origin: ProfileOrigin::ExternalByop,
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("profile-1".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-1".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    service_name: Some("service-a".to_string()),
                    agent_name: Some("agent-a".to_string()),
                    task_name: Some("task-a".to_string()),
                    profile_id: Some("profile-1".to_string()),
                    lease: LeaseState::Exclusive,
                    cleanup: SessionCleanupPolicy::CloseTabs,
                    browser_ids: vec!["browser-1".to_string()],
                    tab_ids: vec!["tab-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([
                (
                    "tab-1".to_string(),
                    BrowserTab {
                        id: "tab-1".to_string(),
                        browser_id: "browser-1".to_string(),
                        target_id: Some("target-1".to_string()),
                        lifecycle: TabLifecycle::Ready,
                        url: Some("https://example.com".to_string()),
                        title: Some("Example".to_string()),
                        owner_session_id: Some("session-1".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-closed".to_string(),
                    BrowserTab {
                        id: "tab-closed".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Closed,
                        owner_session_id: Some("session-1".to_string()),
                        ..BrowserTab::default()
                    },
                ),
            ]),
            jobs: BTreeMap::from([(
                "job-tab".to_string(),
                ServiceJob {
                    id: "job-tab".to_string(),
                    target: JobTarget::Tab("tab-1".to_string()),
                    completed_at: Some("2026-06-13T12:00:00Z".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_service_tab_handles();

        let handle = state.tabs["tab-1"].service_tab_handle.as_ref().unwrap();
        assert_eq!(handle.browser_id, "browser-1");
        assert_eq!(handle.session_name.as_deref(), Some("session-1"));
        assert_eq!(handle.tab_id, "tab-1");
        assert_eq!(handle.target_id.as_deref(), Some("target-1"));
        assert_eq!(handle.profile_id.as_deref(), Some("profile-1"));
        assert_eq!(handle.profile_origin, ProfileOrigin::ExternalByop);
        assert_eq!(handle.lease_state, Some(LeaseState::Exclusive));
        assert_eq!(handle.cleanup_policy, Some(SessionCleanupPolicy::CloseTabs));
        assert_eq!(handle.job_id.as_deref(), Some("job-tab"));
        assert_eq!(
            handle.trace_filter.service_name.as_deref(),
            Some("service-a")
        );
        assert_eq!(handle.trace_filter.agent_name.as_deref(), Some("agent-a"));
        assert_eq!(handle.trace_filter.task_name.as_deref(), Some("task-a"));
        assert!(handle.valid);
        assert_eq!(handle.stale_reason, None);
        assert_eq!(
            state.browsers["browser-1"].tab_handles[0],
            state.tabs["tab-1"].service_tab_handle.clone().unwrap()
        );

        let stale = state.tabs["tab-closed"]
            .service_tab_handle
            .as_ref()
            .unwrap();
        assert!(!stale.valid);
        assert_eq!(stale.stale_reason.as_deref(), Some("tab_closed"));
    }

    #[test]
    fn shared_profile_coordination_derives_incidents_for_conflicts_and_abandoned_tabs() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("shared-profile".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["shared-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "shared-session".to_string(),
                BrowserSession {
                    id: "shared-session".to_string(),
                    profile_id: Some("shared-profile".to_string()),
                    lease: LeaseState::Shared,
                    browser_ids: vec!["browser-1".to_string()],
                    tab_ids: vec![
                        "tab-closed".to_string(),
                        "tab-missing".to_string(),
                        "tab-ready".to_string(),
                    ],
                    profile_lease_conflict_session_ids: vec!["holder-session".to_string()],
                    last_lease_observed_at: Some("2026-06-19T22:45:00Z".to_string()),
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([
                (
                    "tab-closed".to_string(),
                    BrowserTab {
                        id: "tab-closed".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Closed,
                        owner_session_id: Some("shared-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-ready".to_string(),
                    BrowserTab {
                        id: "tab-ready".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("shared-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        let incidents = state
            .incidents
            .iter()
            .map(|incident| (incident.id.as_str(), incident))
            .collect::<BTreeMap<_, _>>();
        let conflict = incidents
            .get("profile-lease-conflict:shared-session")
            .unwrap();
        assert_eq!(conflict.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(conflict.latest_kind, "profile_lease_conflict");
        assert_eq!(conflict.state, ServiceIncidentState::Active);
        assert_eq!(conflict.severity, ServiceIncidentSeverity::Warning);
        assert_eq!(
            conflict.escalation,
            ServiceIncidentEscalation::ServiceTriage
        );
        assert!(conflict.latest_message.contains("holder-session"));
        assert!(conflict
            .recommended_action
            .contains("retained profile holder"));
        assert_service_incident_record_contract(&serde_json::to_value(conflict).unwrap());

        let closed_tab = incidents
            .get("shared-tab-abandoned:shared-session:tab-closed")
            .unwrap();
        assert_eq!(closed_tab.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(closed_tab.latest_kind, "shared_tab_abandoned");
        assert_eq!(closed_tab.severity, ServiceIncidentSeverity::Warning);
        assert!(closed_tab.latest_message.contains("closed"));
        assert_service_incident_record_contract(&serde_json::to_value(closed_tab).unwrap());

        let missing_tab = incidents
            .get("shared-tab-abandoned:shared-session:tab-missing")
            .unwrap();
        assert_eq!(missing_tab.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(missing_tab.latest_kind, "shared_tab_abandoned");
        assert!(missing_tab.latest_message.contains("missing"));

        assert!(!incidents.contains_key("shared-tab-abandoned:shared-session:tab-ready"));
    }

    #[test]
    fn expire_stale_session_leases_preserves_browser_and_marks_handles_stale() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("shared-profile".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec![
                        "expired-session".to_string(),
                        "fresh-session".to_string(),
                        "invalid-session".to_string(),
                        "released-session".to_string(),
                    ],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([
                (
                    "expired-session".to_string(),
                    BrowserSession {
                        id: "expired-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Shared,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-expired".to_string()],
                        expires_at: Some("2026-06-19T22:44:59Z".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "fresh-session".to_string(),
                    BrowserSession {
                        id: "fresh-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Shared,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-fresh".to_string()],
                        expires_at: Some("2026-06-19T22:45:01Z".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "invalid-session".to_string(),
                    BrowserSession {
                        id: "invalid-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Exclusive,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-invalid".to_string()],
                        expires_at: Some("not-a-timestamp".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "released-session".to_string(),
                    BrowserSession {
                        id: "released-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Released,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-released".to_string()],
                        expires_at: Some("2026-06-19T22:44:00Z".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "orphaned-session".to_string(),
                    BrowserSession {
                        id: "orphaned-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Exclusive,
                        browser_ids: vec!["missing-browser".to_string()],
                        ..BrowserSession::default()
                    },
                ),
            ]),
            tabs: BTreeMap::from([
                (
                    "tab-expired".to_string(),
                    BrowserTab {
                        id: "tab-expired".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("expired-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-fresh".to_string(),
                    BrowserTab {
                        id: "tab-fresh".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("fresh-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-invalid".to_string(),
                    BrowserTab {
                        id: "tab-invalid".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("invalid-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-released".to_string(),
                    BrowserTab {
                        id: "tab-released".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("released-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let expired =
            state.expire_stale_session_leases("2026-06-19T22:45:00Z", Some("test-boot-epoch"));

        assert_eq!(
            expired,
            vec![
                "expired-session".to_string(),
                "orphaned-session".to_string()
            ]
        );
        assert_eq!(state.sessions["expired-session"].lease, LeaseState::Expired);
        assert_eq!(
            state.sessions["expired-session"].boot_epoch.as_deref(),
            Some("test-boot-epoch")
        );
        assert_eq!(
            state.sessions["orphaned-session"].lease,
            LeaseState::Expired
        );
        assert_eq!(
            state.sessions["expired-session"]
                .last_lease_observed_at
                .as_deref(),
            Some("2026-06-19T22:45:00Z")
        );
        assert_eq!(state.sessions["fresh-session"].lease, LeaseState::Shared);
        assert_eq!(
            state.sessions["invalid-session"].lease,
            LeaseState::Exclusive
        );
        assert_eq!(
            state.sessions["released-session"].lease,
            LeaseState::Released
        );
        assert_eq!(
            state.browsers["browser-1"].active_session_ids,
            vec![
                "fresh-session".to_string(),
                "invalid-session".to_string(),
                "released-session".to_string()
            ]
        );
        assert_eq!(state.browsers["browser-1"].health, BrowserHealth::Ready);
        assert_eq!(
            state.tabs["tab-expired"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .stale_reason
                .as_deref(),
            Some("lease_expired")
        );
        assert!(
            state.tabs["tab-fresh"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .valid
        );
        assert!(
            state.tabs["tab-invalid"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .valid
        );
        assert_eq!(
            state.tabs["tab-released"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .stale_reason
                .as_deref(),
            Some("lease_released")
        );
    }

    #[test]
    fn closing_one_connection_marks_only_its_profile_children_disconnected() {
        let child = |connection: &str| ProfileChildAccess {
            subject_id: Some("client:fieldwork".to_string()),
            connection_instance_id: Some(connection.to_string()),
            ..ProfileChildAccess::default()
        };
        let mut state = ServiceState {
            tabs: BTreeMap::from([
                (
                    "tab-owned".to_string(),
                    BrowserTab {
                        id: "tab-owned".to_string(),
                        browser_id: "browser-shared".to_string(),
                        profile_access: Some(child("connection-closing")),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-independent".to_string(),
                    BrowserTab {
                        id: "tab-independent".to_string(),
                        browser_id: "browser-shared".to_string(),
                        profile_access: Some(child("connection-surviving")),
                        ..BrowserTab::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        assert_eq!(
            state.mark_profile_connection_disconnected("connection-closing"),
            1
        );
        assert_eq!(
            state.tabs["tab-owned"]
                .profile_access
                .as_ref()
                .unwrap()
                .connection_state,
            ProfileConnectionState::Disconnected
        );
        assert_eq!(
            state.tabs["tab-independent"]
                .profile_access
                .as_ref()
                .unwrap()
                .connection_state,
            ProfileConnectionState::Active
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

    #[test]
    fn boot_epoch_prior_classification_is_pure_and_fail_closed() {
        assert!(boot_epoch_is_prior(Some("boot:old"), Some("boot:new")));
        assert!(!boot_epoch_is_prior(Some("boot:same"), Some("boot:same")));
        assert!(!boot_epoch_is_prior(None, Some("boot:current")));
        assert!(!boot_epoch_is_prior(Some("boot:recorded"), None));
    }
}
