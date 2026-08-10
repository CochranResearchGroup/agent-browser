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
use crate::native::service_health::refresh_authoritative_route_pool;
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
async fn test_service_reconcile_records_then_compacts_unreachable_browser_health() {
    let home = unique_socket_dir("service-reconcile-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_reconcile", "id" : "svc-reconcile-1", "serviceState" : {
        "browsers" : { "browser-1" : { "id" : "browser-1", "host" : "attached_existing",
        "health" : "ready", "cdpEndpoint" :
        "ws://127.0.0.1:9/devtools/browser/unreachable", "activeSessionIds" :
        ["reconcile-session"] } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["reconciled"], true);
    assert_eq!(result["data"]["browserCount"], 0);
    assert_eq!(result["data"]["changedBrowsers"], 1);
    assert!(result["data"]["service_state"]["browsers"]
        .as_object()
        .is_none_or(|browsers| !browsers.contains_key("browser-1")));
    assert_eq!(
        result["data"]["service_state"]["reconciliation"]["changedBrowsers"],
        1
    );
    assert_eq!(
        result["data"]["service_state"]["events"][0]["kind"],
        "browser_health_changed"
    );
    assert_eq!(
        result["data"]["service_state"]["events"][1]["kind"],
        "reconciliation"
    );
    assert!(state.browser.is_none());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    let persisted = store.load().unwrap();
    assert!(!persisted.browsers.contains_key("browser-1"));
    assert_eq!(
        persisted
            .reconciliation
            .as_ref()
            .map(|snapshot| snapshot.changed_browsers),
        Some(1)
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserHealthChanged
            && event.browser_id.as_deref() == Some("browser-1")
    }));
    assert!(persisted
        .events
        .iter()
        .any(|event| event.kind == ServiceEventKind::Reconciliation));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_reconcile_reports_remote_view_repair_summary() {
    let home = unique_socket_dir("service-reconcile-remote-view-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    let mut service_state = ServiceState::default();
    service_state.browsers.insert(
        "browser-1".to_string(),
        BrowserProcess {
            id: "browser-1".to_string(),
            health: ServiceBrowserHealth::ProcessExited,
            ..BrowserProcess::default()
        },
    );
    service_state.display_allocations.insert(
        "display-1".to_string(),
        DisplayAllocation {
            id: "display-1".to_string(),
            state: "ready".to_string(),
            owner_browser_id: Some("browser-1".to_string()),
            ..DisplayAllocation::default()
        },
    );
    service_state.remote_view_routes.insert(
        "route-1".to_string(),
        RemoteViewRoute {
            id: "route-1".to_string(),
            state: "ready".to_string(),
            browser_id: Some("browser-1".to_string()),
            display_allocation_id: Some("display-1".to_string()),
            controller_lease_id: Some("viewer-1".to_string()),
            viewer_lease_ids: vec!["viewer-1".to_string()],
            ..RemoteViewRoute::default()
        },
    );
    service_state.viewer_leases.insert(
        "viewer-1".to_string(),
        ViewerLease {
            id: "viewer-1".to_string(),
            state: "observing".to_string(),
            route_id: Some("route-1".to_string()),
            browser_id: Some("browser-1".to_string()),
            ..ViewerLease::default()
        },
    );
    let result = execute_command(
        &json!(
            { "action" : "service_reconcile", "id" : "svc-reconcile-remote-view-1",
            "serviceState" : service_state, }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(
        result["data"]["remoteViewRepair"]["orphanedDisplayAllocations"],
        1
    );
    assert_eq!(result["data"]["remoteViewRepair"]["orphanedRoutes"], 1);
    assert_eq!(
        result["data"]["remoteViewRepair"]["releasedViewerLeases"],
        1
    );
    assert_eq!(
        result["data"]["remoteViewRepair"]["clearedControllerLeases"],
        1
    );
    assert_eq!(result["data"]["remoteViewRepair"]["repaired"], 2);
    assert_eq!(result["data"]["remoteViewRepair"]["released"], 1);
    assert_eq!(result["data"]["remoteViewRepair"]["skippedUnsafe"], 0);
    assert_eq!(
        result["data"]["service_state"]["events"][0]["details"]["remoteView"],
        result["data"]["remoteViewRepair"]
    );
    let _ = fs::remove_dir_all(&home);
}
#[cfg(unix)]
#[tokio::test]
async fn test_service_reconcile_refreshes_stale_available_route_pool_definition() {
    let home = unique_socket_dir("service-reconcile-authoritative-route-pool-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_reconcile", "id" :
        "svc-reconcile-authoritative-route-pool-1", "serviceState" : { "routePool" : {
        "guacamole-rdp-a" : { "id" : "guacamole-rdp-a", "provider" : "rdp_gateway",
        "routeId" : "guacamole:4", "connectionId" : "4", "connectionName" :
        "Agent Browser RDP Route A", "frameUrl" :
        "http://127.0.0.1:8092/guacamole/#/client/legacy", "externalUrl" :
        "https://example.test/guacamole/#/client/legacy", "target" : { "displayName" :
        ":10", "routeUser" : "agent-browser-rdp-a" }, "providerMode" :
        "simultaneous_view", "state" : "available", "currentRouteAllocationId" : null } }
        }, "authoritativeRoutePool" : [{ "id" : "guacamole-rdp-a", "provider" :
        "rdp_gateway", "routeId" : "guacamole:1", "connectionId" : "1", "connectionName"
        : "Agent Browser RDP Route A", "frameUrl" :
        "http://127.0.0.1:8092/guacamole/#/client/current", "externalUrl" :
        "https://example.test/guacamole/#/client/current", "target" : { "displayName" :
        ":11", "routeUser" : "agent-browser-rdp-a" }, "providerMode" :
        "simultaneous_view", "state" : "available", "currentRouteAllocationId" : null,
        "readiness" : { "state" : "ready" } }] }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(
        result["data"]["routePoolRefresh"]["updatedEntryIds"][0],
        "guacamole-rdp-a"
    );
    assert_eq!(
        result["data"]["service_state"]["routePool"]["guacamole-rdp-a"]["routeId"],
        "guacamole:1"
    );
    assert_eq!(
        result["data"]["service_state"]["routePool"]["guacamole-rdp-a"]["connectionId"],
        "1"
    );
    assert_eq!(
        result["data"]["service_state"]["routePool"]["guacamole-rdp-a"]["target"]["displayName"],
        ":11"
    );
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    let persisted = store.load().unwrap();
    let route = &persisted.route_pool["guacamole-rdp-a"];
    assert_eq!(route.route_id, "guacamole:1");
    assert_eq!(route.connection_id.as_deref(), Some("1"));
    assert_eq!(route.target["displayName"], ":11");
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_reconcile_does_not_replace_active_conflicting_route_pool_definition() {
    let home = unique_socket_dir("service-reconcile-active-route-pool-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_reconcile", "id" : "svc-reconcile-active-route-pool-1",
        "serviceState" : { "routePool" : { "guacamole-rdp-a" : { "id" :
        "guacamole-rdp-a", "provider" : "rdp_gateway", "routeId" : "guacamole:4",
        "target" : { "displayName" : ":10" }, "state" : "checked_out",
        "currentRouteAllocationId" : "guacamole:4" } }, "remoteViewRoutes" : {
        "guacamole:4" : { "id" : "guacamole:4", "provider" : "rdp_gateway", "state" :
        "ready" } } }, "authoritativeRoutePool" : [{ "id" : "guacamole-rdp-a", "provider"
        : "rdp_gateway", "routeId" : "guacamole:1", "target" : { "displayName" : ":11" },
        "readiness" : { "state" : "ready" } }] }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(
        result["data"]["routePoolRefresh"]["skippedActiveConflictEntryIds"][0],
        "guacamole-rdp-a"
    );
    assert_eq!(
        result["data"]["service_state"]["routePool"]["guacamole-rdp-a"]["routeId"],
        "guacamole:4"
    );
    assert_eq!(
        result["data"]["service_state"]["routePool"]["guacamole-rdp-a"]["currentRouteAllocationId"],
        "guacamole:4"
    );
    let _ = fs::remove_dir_all(&home);
}
