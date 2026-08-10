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
async fn test_service_trace_returns_related_records_and_activity() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_trace", "id" : "svc-trace-1", "serviceName" :
        "JournalDownloader", "taskName" : "probeACSwebsite", "profileId" : "work",
        "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser failed", "browserId" : "browser-1", "profileId" : "work", "sessionId" :
        "session-1", "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "probeACSwebsite" }, { "id" : "event-wait-started", "timestamp" :
        "2026-04-22T00:00:30Z", "kind" : "profile_lease_wait_started", "message" :
        "Service job job-1 started waiting for profile lease work", "profileId" : "work",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "details" : { "jobId" : "job-1", "outcome" : "started",
        "profileId" : "work", "conflictSessionIds" : ["active-session"], "retryAfterMs" :
        50 } }, { "id" : "event-wait-ended", "timestamp" : "2026-04-22T00:01:30Z", "kind"
        : "profile_lease_wait_ended", "message" :
        "Service job job-1 ended profile lease wait for work with outcome ready",
        "profileId" : "work", "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "probeACSwebsite", "details" : { "jobId" : "job-1", "outcome" :
        "ready", "profileId" : "work", "conflictSessionIds" : ["active-session"],
        "retryAfterMs" : 50, "waitedMs" : 60000 } }, { "id" : "event-2", "timestamp" :
        "2026-04-22T00:01:00Z", "kind" : "browser_health_changed", "message" :
        "Wrong task", "browserId" : "browser-1", "profileId" : "work", "sessionId" :
        "session-1", "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "otherTask" }], "jobs" : { "job-1" : { "id" : "job-1", "action" :
        "navigate", "state" : "timed_out", "target" : { "browser" : "browser-1" },
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "siteId" : "acs", "targetServiceIds" : ["acs", "google"],
        "displayIsolation" : "private_virtual_display", "namingWarnings" :
        service_job_naming_warning_values(), "hasNamingWarning" : true, "submittedAt" :
        "2026-04-22T00:02:00Z", "error" : "Timed out" }, "job-2" : { "id" : "job-2",
        "action" : "navigate", "state" : "timed_out", "target" : { "profile" : "other" },
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "displayIsolation" : "shared_display", "submittedAt" :
        "2026-04-22T00:03:00Z", "error" : "Wrong profile" } }, "browsers" : { "browser-1"
        : { "id" : "browser-1", "profileId" : "work", "activeSessionIds" : ["session-1"]
        } }, "sessions" : { "session-1" : { "id" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "profileId" : "work", "browserIds" : ["browser-1"], "browserCapabilityLaunch" : {
        "applied" : true, "reason" : "validated_binding_applied", "browserBuild" :
        "stealthcdp_chromium", "bindingId" : "binding-1", "hostId" : "local",
        "executableId" : "stealth-current", "capabilityId" : "stealth-cdp",
        "executablePath" : "/opt/chromium-stealthcdp/chrome" } } }, "incidents" : [{ "id"
        : "browser-1", "browserId" : "browser-1", "label" : "browser-1", "state" :
        "active", "severity" : "error", "escalation" : "browser_recovery",
        "recommendedAction" :
        "Review recovery trace and retry or relaunch the affected browser.",
        "latestTimestamp" : "2026-04-22T00:02:00Z", "latestMessage" : "Timed out",
        "latestKind" : "service_job_timeout", "currentHealth" : "process_exited",
        "eventIds" : ["event-1"], "jobIds" : ["job-1"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["counts"]["events"], 3);
    assert_eq!(result["data"]["counts"]["jobs"], 1);
    assert_eq!(result["data"]["counts"]["incidents"], 1);
    assert_eq!(result["data"]["counts"]["activity"], 2);
    assert_eq!(result["data"]["events"][0]["id"], "event-1");
    assert_service_event_record_contract(&result["data"]["events"][0]);
    assert_eq!(result["data"]["jobs"][0]["id"], "job-1");
    assert_service_job_naming_warning_contract(&result["data"]["jobs"][0]);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-1");
    assert_service_incident_record_contract(&result["data"]["incidents"][0]);
    assert_service_trace_response_contract(&result["data"]);
    assert_eq!(result["data"]["activity"][1]["jobId"], "job-1");
    assert_service_trace_activity_record_contract(&result["data"]["activity"][1]);
    assert_service_trace_summary_record_contract(&result["data"]["summary"]);
    assert_eq!(result["data"]["summary"]["contextCount"], 3);
    assert_eq!(result["data"]["summary"]["hasTraceContext"], true);
    assert_eq!(result["data"]["summary"]["namingWarningCount"], 1);
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["count"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["appliedCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["launches"][0]["browserBuild"],
        "stealthcdp_chromium"
    );
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["launches"][0]["reason"],
        "validated_binding_applied"
    );
    assert_eq!(result["data"]["summary"]["displayAllocations"]["count"], 1);
    assert_eq!(
        result["data"]["summary"]["displayAllocations"]["recordedCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["displayAllocations"]["privateVirtualDisplayCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["displayAllocations"]["allocations"][0]["displayIsolation"],
        "private_virtual_display"
    );
    assert_eq!(result["data"]["summary"]["profileLeaseWaits"]["count"], 1);
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["completedCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["waits"][0]["jobId"],
        "job-1"
    );
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["waits"][0]["outcome"],
        "ready"
    );
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["waits"][0]["waitedMs"],
        60000
    );
    let owned_context = result["data"]["summary"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| {
            context["serviceName"] == "JournalDownloader"
                && context["agentName"] == "codex"
                && context["taskName"] == "probeACSwebsite"
                && context["browserId"] == "browser-1"
                && context["profileId"] == "work"
                && context["sessionId"] == "session-1"
        })
        .expect("trace summary should include the owned service context");
    assert_eq!(owned_context["eventCount"], 1);
    assert_eq!(owned_context["jobCount"], 1);
    assert_eq!(owned_context["activityCount"], 2);
    assert_eq!(owned_context["targetIdentityCount"], 2);
    assert_eq!(owned_context["targetServiceIds"], json!(["acs", "google"]));
    assert_eq!(
        owned_context["displayAllocations"],
        json!(["private_virtual_display"])
    );
    assert_eq!(
        owned_context["unrecordedDisplayAllocationJobCount"],
        json!(0)
    );
    assert_eq!(owned_context["hasNamingWarning"], false);
    assert_eq!(owned_context["namingWarnings"].as_array().unwrap().len(), 0);
    assert_eq!(owned_context["attention"]["required"], false);
    assert_eq!(owned_context["attention"]["reason"], "none");
    assert_eq!(owned_context["latestTimestamp"], "2026-04-22T00:02:00Z");
    let wait_context = result["data"]["summary"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| {
            context["serviceName"] == "JournalDownloader"
                && context["agentName"] == "codex"
                && context["taskName"] == "probeACSwebsite"
                && context["profileId"] == "work"
                && context["browserId"].is_null()
                && context["sessionId"].is_null()
        })
        .expect("trace summary should include a profile lease wait context");
    assert_eq!(wait_context["eventCount"], 2);
    assert_eq!(wait_context["hasNamingWarning"], false);
    let incident_context = result["data"]["summary"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| context["incidentCount"] == 1)
        .expect("trace summary should include incident-only context");
    assert_eq!(incident_context["hasNamingWarning"], true);
    assert_eq!(incident_context["attention"]["required"], true);
    assert_eq!(incident_context["attention"]["owner"], "operator");
    assert_eq!(incident_context["attention"]["reason"], "incidents_present");
    assert_eq!(
        incident_context["namingWarnings"],
        json!([
            "missing_service_name",
            "missing_agent_name",
            "missing_task_name"
        ])
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_trace_returns_browser_recovery_sequence() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_trace", "id" : "svc-trace-recovery-1", "browserId" :
        "browser-1", "serviceName" : "JournalDownloader", "taskName" : "probeACSwebsite",
        "serviceState" : { "events" : [{ "id" : "event-stale", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser browser-1 health changed from Ready to ProcessExited", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "previousHealth" : "ready", "currentHealth" : "process_exited", "details" : {
        "currentReasonKind" : "process_exited" } }, { "id" : "event-recovery",
        "timestamp" : "2026-04-22T00:00:01Z", "kind" : "browser_recovery_started",
        "message" : "Browser browser-1 recovery started", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "currentHealth" : "process_exited", "details" : { "reasonKind" :
        "process_exited", "reason" :
        "Active browser PID 1234 exited before command dispatch" } }, { "id" :
        "event-ready", "timestamp" : "2026-04-22T00:00:02Z", "kind" :
        "browser_health_changed", "message" :
        "Browser browser-1 health changed from ProcessExited to Ready", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "previousHealth" : "process_exited", "currentHealth" : "ready" }], "incidents" :
        [{ "id" : "browser-1", "browserId" : "browser-1", "label" : "browser-1", "state"
        : "recovered", "latestTimestamp" : "2026-04-22T00:00:02Z", "latestMessage" :
        "Browser browser-1 health changed from ProcessExited to Ready", "latestKind" :
        "browser_health_changed", "eventIds" : ["event-stale", "event-ready"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["counts"]["events"], 3);
    assert_eq!(
        result["data"]["events"][0]["kind"],
        "browser_health_changed"
    );
    assert_eq!(
        result["data"]["events"][0]["currentHealth"],
        "process_exited"
    );
    assert_eq!(
        result["data"]["events"][0]["details"]["currentReasonKind"],
        "process_exited"
    );
    assert_eq!(
        result["data"]["events"][1]["kind"],
        "browser_recovery_started"
    );
    assert_eq!(
        result["data"]["events"][1]["details"]["reason"],
        "Active browser PID 1234 exited before command dispatch"
    );
    assert_eq!(
        result["data"]["events"][1]["details"]["reasonKind"],
        "process_exited"
    );
    assert_eq!(
        result["data"]["events"][2]["kind"],
        "browser_health_changed"
    );
    assert_eq!(result["data"]["events"][2]["currentHealth"], "ready");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_trace_filters_browser_recovery_override_events() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_trace", "id" : "svc-trace-recovery-override-1",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "serviceState" : { "events" : [{ "id" : "event-override",
        "timestamp" : "2026-04-22T00:00:03Z", "kind" : "browser_recovery_override",
        "message" : "Browser browser-1 recovery retry enabled by operator", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "previousHealth" : "faulted", "currentHealth" : "process_exited", "details" : {
        "actor" : "operator", "action" : "retry_enabled" } }, { "id" :
        "event-other-task", "timestamp" : "2026-04-22T00:00:04Z", "kind" :
        "browser_recovery_override", "message" :
        "Browser browser-1 recovery retry enabled by operator", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "otherTask",
        "previousHealth" : "faulted", "currentHealth" : "process_exited", "details" : {
        "actor" : "operator", "action" : "retry_enabled" } }], "incidents" : [{ "id" :
        "browser-1", "browserId" : "browser-1", "label" : "browser-1", "state" :
        "active", "latestTimestamp" : "2026-04-22T00:00:03Z", "latestMessage" :
        "Browser browser-1 recovery retry enabled by operator", "latestKind" :
        "browser_recovery_override", "eventIds" : ["event-override"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["counts"]["events"], 1);
    assert_eq!(result["data"]["events"][0]["id"], "event-override");
    assert_eq!(
        result["data"]["events"][0]["kind"],
        "browser_recovery_override"
    );
    assert_eq!(
        result["data"]["events"][0]["details"]["action"],
        "retry_enabled"
    );
    assert_eq!(result["data"]["counts"]["incidents"], 1);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-1");
    assert!(state.browser.is_none());
}
