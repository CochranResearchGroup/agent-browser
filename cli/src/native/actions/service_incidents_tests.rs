#![allow(unused_imports)]
use super::*;
use crate::native::action_runtime::cancellation::{cancellable, cancellation_error};
use crate::native::action_runtime::DaemonState;
use crate::native::auth;
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::cancellation::CancellationToken;
use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
use crate::native::cookies;
use crate::native::network::{self, DomainFilter, EventTracker};
use crate::native::policy::{ActionPolicy, ConfirmActions, PolicyResult};
use crate::native::providers;
use crate::native::remote_view::open::*;
use crate::native::remote_view::{
    display_allocation_id_for_route_pool_entry, normalize_remote_view_open_intent,
    plan_remote_view_acquisition, readiness_state, route_binding_readiness,
    route_bound_display_content, route_display_content, visible_browser_window_proof,
    RemoteViewAcquisitionPlan, RemoteViewRouteBinding,
};
use crate::native::remote_view_handoff::{
    apply_retained_remote_view_route, begin_route_bound_handoff_failure_recovery,
    begin_route_bound_handoff_plan_acquisition, complete_route_bound_handoff_failure_cleanup,
    complete_route_bound_handoff_open, planned_route_bound_handoff_response,
    remote_view_handoff_resolution_command, remote_view_handoff_was_explicitly_closed,
    route_bound_handoff_checkout_command_with_visible_window_proof,
    route_bound_handoff_checkout_failure, route_bound_handoff_failure_cleanup_task_result,
    route_bound_handoff_focus_command, route_bound_handoff_focus_failure,
    route_bound_handoff_immediate_failure, route_bound_handoff_launch_failure_cleanup,
    route_bound_handoff_operator_visible,
    route_bound_handoff_operator_visible_failure_if_not_ready, route_bound_handoff_plan,
    route_bound_handoff_post_checkout_proof, route_bound_handoff_pre_launch_failure_cleanup,
    route_bound_handoff_reused_browser_launch_result, route_bound_handoff_tab_open_failure,
    route_bound_handoff_target_url_readiness, route_bound_handoff_visible_window_proof_failure,
    shared_profile_acquisition_result, CompleteRouteBoundHandoffOpenInput,
    RouteBoundHandoffFailureCleanupInput, RouteBoundHandoffFailureCleanupSummary,
    RouteBoundHandoffFailureCleanupTask, RouteBoundHandoffFailureRecoveryInput,
    RouteBoundHandoffImmediateFailureInput, RouteBoundHandoffPlan,
    RouteBoundHandoffPlannedResponseInput, RouteBoundHandoffPostCheckoutProofInput,
    SharedProfileAcquisitionResultInput,
};
use crate::native::service_diagnostics::*;
use crate::native::service_file_transfer::*;
use crate::native::service_health::{
    close_health_from_outcome, recovery_policy_for_next_attempt, stale_browser_process_record,
};
use crate::native::service_health::{
    persist_browser_recovery_started_in_repository, persist_closed_browser_health_in_repository,
    persist_current_browser_stale_health_in_repository,
    persist_reconciled_service_state_in_repository, persist_service_browser_record_in_repository,
    reconcile_service_state, retry_degraded_service_browser_in_state,
    retry_persisted_service_browser_in_repository, retry_service_browser_in_state,
    BrowserRecoveryPersistence, BrowserRecoveryPolicyConfig, BrowserRecoveryPolicySource,
    BrowserRecoveryPolicyValueSource, BrowserRecoveryReasonKind,
};
use crate::native::service_lifecycle::{
    profile_lease_telemetry, select_service_profile_for_request, service_profile_id,
    ProfileSelectionRequest, ServiceLaunchMetadata,
};
use crate::native::service_model::{
    assert_service_browser_capability_registry_upsert_response_contract,
    assert_service_browser_retry_response_contract, assert_service_collection_response_contract,
    assert_service_event_record_contract, assert_service_events_response_contract,
    assert_service_incident_acknowledge_response_contract,
    assert_service_incident_activity_response_contract, assert_service_incident_record_contract,
    assert_service_incident_resolve_response_contract, assert_service_incidents_response_contract,
    assert_service_job_cancel_response_contract, assert_service_job_naming_warning_contract,
    assert_service_jobs_response_contract, assert_service_monitor_delete_response_contract,
    assert_service_monitor_state_response_contract,
    assert_service_monitor_triage_response_contract,
    assert_service_monitor_upsert_response_contract,
    assert_service_profile_delete_response_contract,
    assert_service_profile_upsert_response_contract,
    assert_service_provider_delete_response_contract,
    assert_service_provider_upsert_response_contract, assert_service_reconcile_response_contract,
    assert_service_remedies_apply_response_contract,
    assert_service_session_delete_response_contract,
    assert_service_session_upsert_response_contract,
    assert_service_site_policy_delete_response_contract,
    assert_service_site_policy_upsert_response_contract, assert_service_status_response_contract,
    assert_service_trace_activity_record_contract, assert_service_trace_response_contract,
    assert_service_trace_summary_record_contract, service_job_naming_warning_values,
    BrowserCapabilityRegistry, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    DisplayAllocation, ProfileSeedingHandoffState, RemoteViewRoute, RoutePoolEntry, ViewStream,
    ViewerLease,
};
use crate::native::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserHealth as ServiceBrowserHealth, BrowserHost as ServiceBrowserHost, ControlInputProvider,
    JobState as ServiceJobState, MonitorState, ProfileClass, ProfileKeyringPolicy,
    ProfileLeaseDisposition, ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease,
    RemoteViewHandoff, ServiceEntitySource, ServiceEvent, ServiceEventKind, ServiceState,
    ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStreamProvider,
};
use crate::native::service_model::{JobState, ServiceJob};
use crate::native::service_model::{LeaseState, ProfileAllocationPolicy};
use crate::native::service_network_capture::*;
use crate::native::service_probe::*;
use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::service_ui_action::*;
use crate::native::state;
use crate::test_utils::EnvGuard;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_service_incidents_returns_limited_incidents() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-1", "limit" : 1,
        "serviceState" : { "incidents" : [{ "id" : "browser-1", "browserId" :
        "browser-1", "label" : "browser-1", "state" : "active", "latestTimestamp" :
        "2026-04-22T00:00:00Z", "latestMessage" : "Browser crashed", "latestKind" :
        "browser_health_changed" }, { "id" : "service", "label" : "Service incidents",
        "state" : "service", "latestTimestamp" : "2026-04-22T00:01:00Z", "latestMessage"
        : "Reconciliation failed", "latestKind" : "reconciliation_error" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 2);
    assert_eq!(result["data"]["total"], 2);
    assert_eq!(result["data"]["incidents"][0]["id"], "service");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_remedies_only_groups_operator_ladder() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-remedies", "summary" :
        true, "remediesOnly" : true, "serviceState" : { "incidents" : [{ "id" :
        "browser-degraded", "browserId" : "browser-degraded", "label" :
        "browser-degraded", "state" : "active", "severity" : "warning", "escalation" :
        "browser_degraded", "recommendedAction" : "Inspect browser health.",
        "currentHealth" : "degraded", "latestTimestamp" : "2026-05-01T00:00:00Z",
        "latestMessage" : "Polite close failed", "latestKind" : "browser_health_changed"
        }, { "id" : "browser-faulted", "browserId" : "browser-faulted", "label" :
        "browser-faulted", "state" : "active", "severity" : "critical", "escalation" :
        "os_degraded_possible", "recommendedAction" : "Inspect the host OS.",
        "currentHealth" : "faulted", "latestTimestamp" : "2026-05-01T00:01:00Z",
        "latestMessage" : "Force kill failed", "latestKind" : "browser_health_changed" },
        { "id" : "browser-recovery", "browserId" : "browser-recovery", "label" :
        "browser-recovery", "state" : "active", "severity" : "error", "escalation" :
        "browser_recovery", "recommendedAction" : "Review recovery trace.",
        "currentHealth" : "process_exited", "latestTimestamp" : "2026-05-01T00:02:00Z",
        "latestMessage" : "Browser exited", "latestKind" : "browser_health_changed" }, {
        "id" : "monitor-login", "monitorId" : "google-login-freshness", "label" :
        "Monitor Google login freshness", "state" : "active", "severity" : "warning",
        "escalation" : "monitor_attention", "recommendedAction" :
        "Inspect the failed monitor target.", "latestTimestamp" : "2026-05-01T00:03:00Z",
        "latestMessage" : "Login freshness failed", "latestKind" :
        "service_monitor_failed" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 3);
    assert_eq!(result["data"]["matched"], 3);
    assert_eq!(result["data"]["filters"]["remediesOnly"], true);
    let summary_groups = result["data"]["summary"]["groups"].as_array().unwrap();
    assert_eq!(summary_groups.len(), 3);
    assert!(summary_groups
        .iter()
        .any(|group| group["escalation"] == "browser_degraded"));
    assert!(summary_groups
        .iter()
        .any(|group| group["escalation"] == "os_degraded_possible"));
    let monitor_group = summary_groups
        .iter()
        .find(|group| group["escalation"] == "monitor_attention")
        .expect("monitor attention group should be included in remedies");
    assert_eq!(
        monitor_group["monitorIds"],
        json!(["google-login-freshness"])
    );
    assert_eq!(
        monitor_group["monitorResetCommands"],
        json!(["agent-browser service monitors reset google-login-freshness"])
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_filter_by_state_kind_browser_and_since() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-2", "state" :
        "recovered", "kind" : "browser_health_changed", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "since" : "2026-04-22T00:01:00Z", "serviceState" : { "events" : [{ "id" :
        "event-old", "timestamp" : "2026-04-22T00:00:00Z", "kind" :
        "browser_health_changed", "message" : "Too old", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-match", "timestamp" : "2026-04-22T00:01:00Z", "kind" :
        "browser_health_changed", "message" : "Matching incident", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-wrong-context", "timestamp" : "2026-04-22T00:04:00Z", "kind" :
        "browser_health_changed", "message" : "Wrong context", "browserId" : "browser-1",
        "profileId" : "other", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }],
        "incidents" : [{ "id" : "browser-1-old", "browserId" : "browser-1", "label" :
        "browser-1", "state" : "recovered", "latestTimestamp" : "2026-04-22T00:00:00Z",
        "latestMessage" : "Too old", "latestKind" : "browser_health_changed", "eventIds"
        : ["event-old"] }, { "id" : "browser-1-match", "browserId" : "browser-1", "label"
        : "browser-1", "state" : "recovered", "latestTimestamp" : "2026-04-22T00:01:00Z",
        "latestMessage" : "Matching incident", "latestKind" : "browser_health_changed",
        "eventIds" : ["event-match"] }, { "id" : "browser-2", "browserId" : "browser-2",
        "label" : "browser-2", "state" : "recovered", "latestTimestamp" :
        "2026-04-22T00:02:00Z", "latestMessage" : "Wrong browser", "latestKind" :
        "browser_health_changed", "eventIds" : ["event-match"] }, { "id" : "service",
        "label" : "Service incidents", "state" : "service", "latestTimestamp" :
        "2026-04-22T00:03:00Z", "latestMessage" : "Wrong state", "latestKind" :
        "reconciliation_error", "eventIds" : ["event-match"] }, { "id" :
        "browser-1-wrong-context", "browserId" : "browser-1", "label" : "browser-1",
        "state" : "recovered", "latestTimestamp" : "2026-04-22T00:04:00Z",
        "latestMessage" : "Wrong context", "latestKind" : "browser_health_changed",
        "eventIds" : ["event-wrong-context"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["total"], 5);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-1-match");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_filter_by_handling_state() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-2b", "handlingState" :
        "acknowledged", "serviceState" : { "incidents" : [{ "id" : "incident-unack",
        "label" : "Service incidents", "state" : "service", "latestTimestamp" :
        "2026-04-22T00:00:00Z", "latestMessage" : "Needs attention", "latestKind" :
        "reconciliation_error" }, { "id" : "incident-ack", "label" : "Service incidents",
        "state" : "service", "acknowledgedAt" : "2026-04-22T00:01:00Z", "acknowledgedBy"
        : "operator", "latestTimestamp" : "2026-04-22T00:01:00Z", "latestMessage" :
        "Triaged", "latestKind" : "reconciliation_error" }, { "id" : "incident-resolved",
        "label" : "Service incidents", "state" : "service", "acknowledgedAt" :
        "2026-04-22T00:02:00Z", "resolvedAt" : "2026-04-22T00:03:00Z", "resolvedBy" :
        "operator", "latestTimestamp" : "2026-04-22T00:03:00Z", "latestMessage" :
        "Handled", "latestKind" : "reconciliation_error" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["incidents"][0]["id"], "incident-ack");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_filter_by_severity_and_escalation() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-severity", "severity" :
        "critical", "escalation" : "os_degraded_possible", "serviceState" : { "incidents"
        : [{ "id" : "browser-degraded", "browserId" : "browser-degraded", "label" :
        "browser-degraded", "state" : "active", "severity" : "warning", "escalation" :
        "browser_degraded", "recommendedAction" : "Inspect browser health.",
        "latestTimestamp" : "2026-04-27T00:00:00Z", "latestMessage" :
        "Polite close failed", "latestKind" : "browser_health_changed", "currentHealth" :
        "degraded" }, { "id" : "browser-faulted", "browserId" : "browser-faulted",
        "label" : "browser-faulted", "state" : "active", "severity" : "critical",
        "escalation" : "os_degraded_possible", "recommendedAction" :
        "Inspect the host OS.", "latestTimestamp" : "2026-04-27T00:01:00Z",
        "latestMessage" : "Force kill failed", "latestKind" : "browser_health_changed",
        "currentHealth" : "faulted" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-faulted");
    assert_eq!(result["data"]["incidents"][0]["severity"], "critical");
    assert_eq!(
        result["data"]["incidents"][0]["escalation"],
        "os_degraded_possible"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_summary_groups_by_operator_remedy() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-summary", "summary" :
        true, "serviceState" : { "incidents" : [{ "id" : "browser-degraded", "browserId"
        : "browser-degraded", "label" : "browser-degraded", "state" : "active",
        "severity" : "warning", "escalation" : "browser_degraded", "recommendedAction" :
        "Inspect browser health.", "latestTimestamp" : "2026-04-27T00:00:00Z",
        "latestMessage" : "Polite close failed", "latestKind" : "browser_health_changed",
        "currentHealth" : "degraded" }, { "id" : "browser-faulted", "browserId" :
        "browser-faulted", "label" : "browser-faulted", "state" : "active", "severity" :
        "critical", "escalation" : "os_degraded_possible", "recommendedAction" :
        "Inspect the host OS.", "latestTimestamp" : "2026-04-27T00:01:00Z",
        "latestMessage" : "Force kill failed", "latestKind" : "browser_health_changed",
        "currentHealth" : "faulted" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["summary"]["groupCount"], 2);
    let groups = result["data"]["summary"]["groups"].as_array().unwrap();
    assert!(groups.iter().any(|group| {
        group["escalation"] == "browser_degraded"
            && group["severity"] == "warning"
            && group["count"] == 1
            && group["recommendedAction"] == "Inspect browser health."
    }));
    assert!(groups.iter().any(|group| {
        group["escalation"] == "os_degraded_possible"
            && group["severity"] == "critical"
            && group["count"] == 1
            && group["recommendedAction"] == "Inspect the host OS."
    }));
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_returns_incident_by_id_with_related_records() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-3", "incidentId" :
        "browser-1", "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser recovered", "browserId" : "browser-1" }], "jobs" : { "job-1" : { "id" :
        "job-1", "action" : "navigate", "state" : "cancelled", "submittedAt" :
        "2026-04-22T00:01:00Z" } }, "incidents" : [{ "id" : "browser-1", "browserId" :
        "browser-1", "label" : "browser-1", "state" : "active", "latestTimestamp" :
        "2026-04-22T00:01:00Z", "latestMessage" : "navigate was cancelled", "latestKind"
        : "service_job_cancelled", "eventIds" : ["event-1"], "jobIds" : ["job-1"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["incident"]["id"], "browser-1");
    assert_eq!(result["data"]["events"][0]["id"], "event-1");
    assert_eq!(result["data"]["jobs"][0]["id"], "job-1");
    assert_eq!(result["data"]["count"], 1);
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incident_activity_returns_normalized_timeline() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incident_activity", "id" : "svc-activity-1", "incidentId" :
        "browser-1", "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser browser-1 health changed from Ready to ProcessExited", "browserId" :
        "browser-1" }, { "id" : "event-2", "timestamp" : "2026-04-22T00:02:00Z", "kind" :
        "incident_acknowledged", "message" : "Incident browser-1 acknowledged",
        "browserId" : "browser-1", "details" : { "incidentId" : "browser-1", "actor" :
        "operator", "action" : "acknowledged", "note" : "triaged" } }], "jobs" : {
        "job-1" : { "id" : "job-1", "action" : "navigate", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "target" : { "browser" : "browser-1" }, "state" : "timed_out", "submittedAt" :
        "2026-04-22T00:01:00Z", "error" : "Timed out after 30000 ms" } }, "browsers" : {
        "browser-1" : { "id" : "browser-1", "profileId" : "work", "activeSessionIds" :
        ["session-1"] } }, "sessions" : { "session-1" : { "id" : "session-1",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "profileId" : "work", "browserIds" : ["browser-1"],
        "browserCapabilityLaunch" : { "applied" : true, "reason" :
        "validated_binding_applied", "browserBuild" : "stealthcdp_chromium", "bindingId"
        : "binding-1", "hostId" : "local", "executableId" : "stealth-current",
        "capabilityId" : "stealth-cdp", "executablePath" :
        "/opt/chromium-stealthcdp/chrome" } } }, "incidents" : [{ "id" : "browser-1",
        "browserId" : "browser-1", "label" : "browser-1", "state" : "active",
        "acknowledgedAt" : "2026-04-22T00:02:00Z", "acknowledgedBy" : "operator",
        "resolvedAt" : "2026-04-22T00:03:00Z", "resolvedBy" : "operator",
        "resolutionNote" : "handled", "latestTimestamp" : "2026-04-22T00:03:00Z",
        "latestMessage" : "Handled", "latestKind" : "service_job_timeout", "eventIds" :
        ["event-1", "event-2"], "jobIds" : ["job-1"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["incident"]["id"], "browser-1");
    assert_service_incident_activity_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 4);
    assert_eq!(
        result["data"]["activity"][0]["kind"],
        "browser_health_changed"
    );
    assert_eq!(result["data"]["activity"][1]["kind"], "service_job_timeout");
    assert_eq!(result["data"]["activity"][1]["browserId"], "browser-1");
    assert_eq!(result["data"]["activity"][1]["profileId"], "work");
    assert_eq!(result["data"]["activity"][1]["sessionId"], "session-1");
    assert_eq!(
        result["data"]["activity"][1]["serviceName"],
        "JournalDownloader"
    );
    assert_eq!(result["data"]["activity"][1]["agentName"], "codex");
    assert_eq!(result["data"]["activity"][1]["taskName"], "probeACSwebsite");
    assert_eq!(
        result["data"]["activity"][2]["kind"],
        "incident_acknowledged"
    );
    assert_eq!(result["data"]["activity"][2]["source"], "event");
    assert_eq!(result["data"]["activity"][3]["kind"], "incident_resolved");
    assert_eq!(result["data"]["activity"][3]["source"], "metadata");
    assert!(state.browser.is_none());
}
