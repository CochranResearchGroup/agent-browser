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
fn unique_socket_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "agent-browser-{label}-{}-{nanos}",
        std::process::id()
    ))
}

#[tokio::test]
async fn test_service_events_returns_limited_events() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_events", "id" : "svc-events-1", "limit" : 1, "serviceState"
        : { "events" : [{ "id" : "event-1", "timestamp" : "2026-04-22T00:00:00Z", "kind"
        : "reconciliation", "message" : "first" }, { "id" : "event-2", "timestamp" :
        "2026-04-22T00:01:00Z", "kind" : "browser_health_changed", "message" : "second",
        "browserId" : "browser-1" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_events_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 2);
    assert_eq!(result["data"]["total"], 2);
    assert_eq!(result["data"]["events"][0]["id"], "event-2");
    assert_service_event_record_contract(&result["data"]["events"][0]);
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_events_filters_by_kind_browser_and_since() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_events", "id" : "svc-events-2", "kind" :
        "browser_health_changed", "browserId" : "browser-1", "profileId" : "work",
        "sessionId" : "session-1", "serviceName" : "JournalDownloader", "agentName" :
        "codex", "taskName" : "probeACSwebsite", "since" : "2026-04-22T00:01:00Z",
        "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" : "too old",
        "browserId" : "browser-1", "profileId" : "work", "sessionId" : "session-1",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite" }, { "id" : "event-2", "timestamp" : "2026-04-22T00:01:00Z",
        "kind" : "browser_health_changed", "message" : "matching", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-3", "timestamp" : "2026-04-22T00:02:00Z", "kind" :
        "browser_health_changed", "message" : "different browser", "browserId" :
        "browser-2", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-4", "timestamp" : "2026-04-22T00:03:00Z", "kind" :
        "reconciliation", "message" : "different kind", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-5", "timestamp" : "2026-04-22T00:04:00Z", "kind" :
        "browser_health_changed", "message" : "different context", "browserId" :
        "browser-1", "profileId" : "other", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_events_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["total"], 5);
    assert_eq!(result["data"]["events"][0]["id"], "event-2");
    assert_service_event_record_contract(&result["data"]["events"][0]);
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_events_rejects_invalid_since() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_events", "id" : "svc-events-3", "since" :
        "not-a-timestamp", "serviceState" : { "events" : [] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], false);
    assert!(result["error"]
        .as_str()
        .unwrap_or_default()
        .contains("Invalid --since timestamp"));
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_view_takeover_records_service_event() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("view-takeover-event-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    state.session_id = "rdp-hardening-a".to_string();
    let cmd = json!(
        { "action" : "view_takeover", "serviceName" : "agent-browser-dashboard",
        "agentName" : "operator", "taskName" : "workspace-viewport-takeover", "browserId"
        : "session:rdp-hardening-a", "sessionName" : "rdp-hardening-a", "streamId" :
        "remote-headed-view", "provider" : "rdp_gateway", "openMode" : "external",
        "reason" : "operator_request", "targetId" : "target-1", "index" : 1 }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    let service_event_id = result["data"]["serviceEventId"].as_str().unwrap();
    let viewer_lease_id = result["data"]["viewerLeaseId"].as_str().unwrap();
    let repository = LockedServiceStateRepository::default_json().unwrap();
    let persisted = repository.load_snapshot().unwrap();
    let event = persisted
        .events
        .iter()
        .find(|event| event.id == service_event_id)
        .expect("view takeover event should be persisted");
    assert_eq!(result["data"]["status"], "accepted");
    assert_eq!(result["data"]["providerMode"], "provider_single_view");
    assert_eq!(event.kind, ServiceEventKind::ViewerTakeoverRequested);
    assert_eq!(event.browser_id.as_deref(), Some("session:rdp-hardening-a"));
    assert_eq!(event.session_id.as_deref(), Some("rdp-hardening-a"));
    assert_eq!(
        event.service_name.as_deref(),
        Some("agent-browser-dashboard")
    );
    assert_eq!(
        event.details.as_ref().unwrap()["viewerLeaseId"],
        viewer_lease_id
    );
    assert_eq!(
        event.details.as_ref().unwrap()["lastViewerEvent"],
        "takeover_requested"
    );
    let _ = fs::remove_dir_all(&home);
}
