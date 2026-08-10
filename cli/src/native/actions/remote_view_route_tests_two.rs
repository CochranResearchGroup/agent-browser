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
use crate::native::remote_view::route_pool_repair::ServiceRoutePoolRepairOptions;
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
use crate::native::service_retained_state::repair_route_pool_service_state;
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
fn route_pool_error_diagnostic(result: &Value) -> Value {
    let error = result["error"].as_str().unwrap();
    let diagnostic = error
        .split_once("diagnostic=")
        .map(|(_, diagnostic)| diagnostic)
        .expect("route pool error should include diagnostic JSON");
    serde_json::from_str(diagnostic).expect("route pool diagnostic should be valid JSON")
}

#[tokio::test]
async fn test_remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-open-inline-refresh-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            route_pool: BTreeMap::from([(
                "guacamole-rdp-a".to_string(),
                RoutePoolEntry {
                    id: "guacamole-rdp-a".to_string(),
                    route_id: "guacamole:1".to_string(),
                    frame_url: Some("http://127.0.0.1:8092/guacamole/#/client/old".to_string()),
                    target: json!(
                        { "hostname" : "host.docker.internal", "port" : "3389",
                        "targetIdentityKey" :
                        "host.docker.internal:3389:user:stale:bpp:24" }
                    ),
                    provider_mode: "simultaneous_view".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("guacamole:1".to_string()),
                    readiness: Some(json!({ "state" : "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "guacamole:1".to_string(),
                RemoteViewRoute {
                    id: "guacamole:1".to_string(),
                    display_allocation_id: Some("remote-view-display:guacamole-1".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:guacamole-1".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:guacamole-1".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    state: "ready".to_string(),
                    readiness: Some(json!(
                        { "state" : "released", "reason" :
                        "operator_requested_close" }
                    )),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    display_allocation_id: Some("remote-view-display:guacamole-1".to_string()),
                    display_name: Some(":10".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "default".to_string();
    let result = execute_command(
        &json!(
            { "action" : "remote_view_open", "routePoolEntryId" : "guacamole-rdp-a",
            "runtimeProfile" : "stealthcdp-default", "url" :
            "https://www.linkedin.com/", "displayAllocationId" :
            "remote-view-display:guacamole-1", "dryRun" : true, "routePoolEntry" : {
            "id" : "guacamole-rdp-a", "routeId" : "guacamole:3", "connectionId" :
            "3", "connectionName" : "Agent Browser RDP Route A", "frameUrl" :
            "http://127.0.0.1:8092/guacamole/#/client/new", "externalUrl" :
            "https://agent-browser.example/guacamole/#/client/new", "routeDescriptor"
            : { "dashboardEmbedUrl" : "http://127.0.0.1:8092/guacamole/#/client/new",
            "publicOperatorUrl" :
            "https://agent-browser.example/guacamole/#/client/new" }, "providerMode"
            : "simultaneous_view", "readiness" : { "state" : "ready" }, "target" : {
            "hostname" : "host.docker.internal", "port" : "3389", "colorDepth" :
            null, "targetIdentityKey" :
            "host.docker.internal:3389:user:current:bpp:default", "displayName" :
            ":11", "displayIsolation" : "shared_display" } } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["routeId"], "guacamole:3");
    assert_eq!(result["data"]["routeBinding"]["connectionId"], "3");
    assert_eq!(result["data"]["routeBinding"]["launchDisplayName"], ":11");
    assert_eq!(
        result["data"]["displayAllocationId"],
        "remote-view-display:11"
    );
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn test_service_route_pool_repair_dry_run_reports_stale_checkouts() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_route_pool_repair", "serviceState" : { "displayAllocations"
        : { "display-active" : { "id" : "display-active", "state" : "ready" },
        "display-orphaned" : { "id" : "display-orphaned", "state" : "ready" } },
        "browsers" : { "session:active" : { "id" : "session:active", "health" : "ready" }
        }, "remoteViewRoutes" : { "route-active" : { "id" : "route-active", "state" :
        "ready", "displayAllocationId" : "display-active", "browserId" : "session:active"
        }, "route-orphaned" : { "id" : "route-orphaned", "state" : "orphaned",
        "displayAllocationId" : "display-orphaned", "browserId" : "session:missing" } },
        "routePool" : { "pool-active" : { "id" : "pool-active", "routeId" :
        "route-active", "state" : "checked_out", "currentRouteAllocationId" :
        "route-active" }, "pool-missing" : { "id" : "pool-missing", "routeId" :
        "route-missing", "state" : "checked_out", "currentRouteAllocationId" :
        "route-missing" }, "pool-orphaned" : { "id" : "pool-orphaned", "routeId" :
        "route-orphaned", "state" : "checked_out", "currentRouteAllocationId" :
        "route-orphaned" }, "pool-available" : { "id" : "pool-available", "routeId" :
        "route-available", "state" : "available" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["dryRun"], true);
    assert_eq!(result["data"]["candidateCounts"]["staleCheckouts"], 2);
    assert_eq!(result["data"]["candidateCounts"]["staleRoutes"], 1);
    assert_eq!(
        result["data"]["candidateCounts"]["staleDisplayAllocations"],
        1
    );
    assert_eq!(
        result["data"]["candidates"]["staleCheckouts"],
        json!(["pool-missing", "pool-orphaned"])
    );
    assert_eq!(
        result["data"]["candidates"]["staleRoutes"],
        json!(["route-orphaned"])
    );
    assert_eq!(
        result["data"]["candidates"]["staleDisplayAllocations"],
        json!(["display-orphaned"])
    );
    assert_eq!(
        result["data"]["candidateReasons"]["staleCheckouts"]["pool-missing"]["reason"],
        "route_missing"
    );
    assert_eq!(
        result["data"]["candidateReasons"]["staleCheckouts"]["pool-orphaned"]["reason"],
        "route_not_active"
    );
    assert_eq!(
        result["data"]["skipped"]["activeCheckouts"],
        json!(["pool-active"])
    );
    assert_eq!(result["data"]["repairedCounts"]["staleCheckouts"], 0);
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_route_pool_repair_dry_run_reads_persisted_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("route-pool-repair-dry-run-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            route_pool: BTreeMap::from([(
                "pool-missing".to_string(),
                RoutePoolEntry {
                    id: "pool-missing".to_string(),
                    route_id: "route-missing".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-missing".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!({ "action" : "service_route_pool_repair", "apply" : false }),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["dryRun"], true);
    assert_eq!(result["data"]["candidateCounts"]["staleCheckouts"], 1);
    assert_eq!(
        result["data"]["candidates"]["staleCheckouts"],
        json!(["pool-missing"])
    );
    assert_eq!(
        store.load().unwrap().route_pool["pool-missing"].state,
        "checked_out"
    );
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn test_remote_view_route_checkout_selects_distinct_matching_pool_entries() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-pool-distinct-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            display_allocations: BTreeMap::from([
                (
                    "display-a".to_string(),
                    DisplayAllocation {
                        id: "display-a".to_string(),
                        display_name: Some(":91".to_string()),
                        display_isolation: "private_virtual_display".to_string(),
                        owner_browser_id: Some("session:rdp-a".to_string()),
                        owner_session_id: Some("rdp-a".to_string()),
                        state: "ready".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-b".to_string(),
                    DisplayAllocation {
                        id: "display-b".to_string(),
                        display_name: Some(":92".to_string()),
                        display_isolation: "private_virtual_display".to_string(),
                        owner_browser_id: Some("session:rdp-b".to_string()),
                        owner_session_id: Some("rdp-b".to_string()),
                        state: "ready".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
            ]),
            route_pool: BTreeMap::from([
                (
                    "pool-a".to_string(),
                    RoutePoolEntry {
                        id: "pool-a".to_string(),
                        route_id: "route-a".to_string(),
                        connection_id: Some("conn-a".to_string()),
                        frame_url: Some("https://guac.example/#/client/conn-a".to_string()),
                        target: json!({ "displayName" : ":91" }),
                        provider_mode: "single_controller".to_string(),
                        state: "available".to_string(),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-b".to_string(),
                    RoutePoolEntry {
                        id: "pool-b".to_string(),
                        route_id: "route-b".to_string(),
                        connection_id: Some("conn-b".to_string()),
                        frame_url: Some("https://guac.example/#/client/conn-b".to_string()),
                        target: json!({ "displayName" : ":92" }),
                        provider_mode: "single_controller".to_string(),
                        state: "available".to_string(),
                        ..RoutePoolEntry::default()
                    },
                ),
            ]),
            browsers: BTreeMap::from([
                (
                    "session:rdp-a".to_string(),
                    BrowserProcess {
                        id: "session:rdp-a".to_string(),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_allocation_id: Some("display-a".to_string()),
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "session:rdp-b".to_string(),
                    BrowserProcess {
                        id: "session:rdp-b".to_string(),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_allocation_id: Some("display-b".to_string()),
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let checkout_a = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(checkout_a["success"], true);
    assert_eq!(checkout_a["data"]["routeId"], "route-a");
    assert_eq!(checkout_a["data"]["routePoolEntryId"], "pool-a");
    let checkout_b = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "browserId" :
            "session:rdp-b", "sessionName" : "rdp-b" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(checkout_b["success"], true);
    assert_eq!(checkout_b["data"]["routeId"], "route-b");
    assert_eq!(checkout_b["data"]["routePoolEntryId"], "pool-b");
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.route_pool["pool-a"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-a")
    );
    assert_eq!(
        persisted.route_pool["pool-b"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-b")
    );
    assert_ne!(
        persisted.browsers["session:rdp-a"].view_streams[0]
            .route_id
            .as_deref(),
        persisted.browsers["session:rdp-b"].view_streams[0]
            .route_id
            .as_deref()
    );
    let _ = fs::remove_dir_all(&home);
}

#[tokio::test]
async fn test_viewer_lease_policy_rejects_single_viewer_and_controller_conflicts() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-viewer-lease-policy-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            remote_view_routes: BTreeMap::from([(
                "route-single".to_string(),
                RemoteViewRoute {
                    id: "route-single".to_string(),
                    provider_mode: "single_viewer".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("session:rdp-a".to_string()),
                    session_id: Some("rdp-a".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let first = execute_command(
        &json!(
            { "action" : "service_viewer_lease_request", "routeId" : "route-single",
            "viewerLeaseId" : "lease-a", "viewerId" : "viewer-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(first["success"], true);
    let second = execute_command(
        &json!(
            { "action" : "service_viewer_lease_request", "routeId" : "route-single",
            "viewerLeaseId" : "lease-b", "viewerId" : "viewer-b" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(second["success"], true);
    assert_eq!(second["data"]["status"], "viewer_denied");
    assert_eq!(second["data"]["reason"], "single_viewer_active");
    store
        .save(&ServiceState {
            remote_view_routes: BTreeMap::from([(
                "route-controller".to_string(),
                RemoteViewRoute {
                    id: "route-controller".to_string(),
                    provider_mode: "single_controller".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("session:rdp-b".to_string()),
                    session_id: Some("rdp-b".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let first_controller = execute_command(
        &json!(
            { "action" : "service_viewer_lease_request", "routeId" :
            "route-controller", "viewerLeaseId" : "lease-controller-a", "viewerId" :
            "viewer-a", "viewerRole" : "controller" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(first_controller["success"], true);
    let denied_controller = execute_command(
        &json!(
            { "action" : "service_viewer_lease_request", "routeId" :
            "route-controller", "viewerLeaseId" : "lease-controller-b", "viewerId" :
            "viewer-b", "viewerRole" : "controller" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(denied_controller["success"], true);
    assert_eq!(denied_controller["data"]["status"], "controller_denied");
    assert_eq!(denied_controller["data"]["reason"], "controller_active");
    let takeover = execute_command(
        &json!(
            { "action" : "service_controller_lease_takeover", "routeId" :
            "route-controller", "viewerLeaseId" : "lease-controller-b", "viewerId" :
            "viewer-b" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(takeover["success"], true);
    assert_eq!(takeover["data"]["controllerLeaseId"], "lease-controller-b");
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.remote_view_routes["route-controller"]
            .controller_lease_id
            .as_deref(),
        Some("lease-controller-b")
    );
    assert!(persisted.viewer_leases.contains_key("lease-controller-a"));
    assert!(persisted
        .events
        .iter()
        .any(|event| event.kind == ServiceEventKind::ControllerDenied));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_checkout_rejects_pool_target_mismatch_and_contention() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-pool-reject-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            display_allocations: BTreeMap::from([
                (
                    "display-a".to_string(),
                    DisplayAllocation {
                        id: "display-a".to_string(),
                        display_name: Some(":91".to_string()),
                        display_isolation: "private_virtual_display".to_string(),
                        state: "ready".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-b".to_string(),
                    DisplayAllocation {
                        id: "display-b".to_string(),
                        display_name: Some(":92".to_string()),
                        display_isolation: "private_virtual_display".to_string(),
                        state: "ready".to_string(),
                        ..DisplayAllocation::default()
                    },
                ),
            ]),
            route_pool: BTreeMap::from([
                (
                    "pool-a".to_string(),
                    RoutePoolEntry {
                        id: "pool-a".to_string(),
                        route_id: "route-a".to_string(),
                        target: json!({ "displayName" : ":91" }),
                        state: "available".to_string(),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-busy".to_string(),
                    RoutePoolEntry {
                        id: "pool-busy".to_string(),
                        route_id: "route-busy".to_string(),
                        frame_url: Some("https://guac.example/#/client/route-busy".to_string()),
                        external_url: Some("https://guac.example/#/client/route-busy".to_string()),
                        target: json!({ "displayName" : ":92" }),
                        state: "available".to_string(),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-failed".to_string(),
                    RoutePoolEntry {
                        id: "pool-failed".to_string(),
                        route_id: "route-failed".to_string(),
                        target: json!({ "displayName" : ":92" }),
                        state: "available".to_string(),
                        readiness: Some(json!(
                            { "components" : [{ "component" : "rdp_backend", "status" :
                            "failed", "evidence" : "target display did not answer" }] }
                        )),
                        ..RoutePoolEntry::default()
                    },
                ),
            ]),
            remote_view_routes: BTreeMap::from([(
                "route-busy".to_string(),
                RemoteViewRoute {
                    id: "route-busy".to_string(),
                    display_allocation_id: Some("display-a".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:rdp-b".to_string(),
                BrowserProcess {
                    id: "session:rdp-b".to_string(),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    display_allocation_id: Some("display-b".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let mismatch = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "display-b", "routePoolEntryId" : "pool-a", "browserId" :
            "session:rdp-b", "sessionName" : "rdp-b" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(mismatch["success"], false);
    assert!(mismatch["error"]
        .as_str()
        .unwrap()
        .contains("route_pool_target_mismatch"));
    let contention = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "display-b", "routeId" : "route-busy", "browserId" : "session:rdp-b",
            "sessionName" : "rdp-b" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(contention["success"], false);
    assert!(contention["error"]
        .as_str()
        .unwrap()
        .contains("route_pool_contention"));
    let not_ready = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "display-b", "routePoolEntryId" : "pool-failed", "browserId" :
            "session:rdp-b", "sessionName" : "rdp-b" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(not_ready["success"], false);
    assert!(not_ready["error"]
        .as_str()
        .unwrap()
        .contains("route_pool_not_ready"));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_checkout_reuses_checked_out_same_owner() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-pool-reuse-owner-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            display_allocations: BTreeMap::from([(
                "remote-view-display:12".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:12".to_string(),
                    display_name: Some(":12".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:repeat".to_string()),
                    owner_session_id: Some("repeat".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-b".to_string(),
                RoutePoolEntry {
                    id: "pool-b".to_string(),
                    route_id: "route-b".to_string(),
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-b".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-b".to_string()),
                    route_descriptor: Some(json!(
                        { "dashboardEmbedUrl" :
                        "https://dashboard.example/guacamole/#/client/route-b",
                        "publicOperatorUrl" :
                        "https://guac.example/#/client/route-b", }
                    )),
                    target: json!({ "displayName" : ":12" }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-b".to_string()),
                    readiness: Some(json!({ "state" : "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-b".to_string(),
                RemoteViewRoute {
                    id: "route-b".to_string(),
                    display_allocation_id: Some("remote-view-display:12".to_string()),
                    browser_id: Some("session:repeat".to_string()),
                    session_id: Some("repeat".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:repeat".to_string(),
                BrowserProcess {
                    id: "session:repeat".to_string(),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    display_allocation_id: Some("remote-view-display:12".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let repeat = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "remote-view-display:12", "routeId" : "route-b", "browserId" :
            "session:repeat", "sessionName" : "repeat" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(repeat["success"], true, "repeat checkout failed: {repeat}");
    assert_eq!(repeat["data"]["routeId"], "route-b");
    assert_eq!(repeat["data"]["routePoolEntryId"], "pool-b");
    let wrong_owner = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "remote-view-display:12", "routeId" : "route-b", "browserId" :
            "session:other", "sessionName" : "other" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(wrong_owner["success"], false);
    assert!(wrong_owner["error"]
        .as_str()
        .unwrap()
        .contains("display_allocation_owner_mismatch"));
    assert!(wrong_owner["error"]
        .as_str()
        .unwrap()
        .contains("availableRoutePoolEntries"));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_checkout_reports_route_pool_unavailable() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-pool-unavailable-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            display_allocations: BTreeMap::from([(
                "display-a".to_string(),
                DisplayAllocation {
                    id: "display-a".to_string(),
                    display_name: Some(":91".to_string()),
                    display_isolation: "private_virtual_display".to_string(),
                    owner_browser_id: Some("session:rdp-a".to_string()),
                    owner_session_id: Some("rdp-a".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    target: json!({ "displayName" : ":91" }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:rdp-a".to_string(),
                BrowserProcess {
                    id: "session:rdp-a".to_string(),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    display_allocation_id: Some("display-a".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], false);
    let error = result["error"].as_str().unwrap();
    assert!(
        error.contains("route_pool_exhausted"),
        "unexpected error: {error}"
    );
    let diagnostic = route_pool_error_diagnostic(&result);
    assert_eq!(diagnostic["requested"]["displayAllocationId"], "display-a");
    assert_eq!(diagnostic["requested"]["displayName"], ":91");
    assert_eq!(
        diagnostic["requested"]["displayIsolation"],
        "private_virtual_display"
    );
    assert_eq!(diagnostic["requested"]["ownerBrowserId"], "session:rdp-a");
    assert_eq!(diagnostic["requested"]["ownerSessionId"], "rdp-a");
    assert_eq!(diagnostic["requested"]["provider"], "rdp_gateway");
    assert_eq!(diagnostic["matchingRoutePoolEntries"][0]["id"], "pool-a");
    assert_eq!(
        diagnostic["matchingRoutePoolEntries"][0]["state"],
        "checked_out"
    );
    assert_eq!(
        diagnostic["matchingRoutePoolEntries"][0]["currentRouteAllocationId"],
        "route-a"
    );
    assert_eq!(diagnostic["availableRoutePoolEntries"], json!([]));
    assert_eq!(diagnostic["availableDisplayAllocationIds"][0], "display-a");
    assert_eq!(
        diagnostic["recommendedCommands"][0],
        "agent-browser service route-pool repair --dry-run"
    );
    assert_eq!(diagnostic["displayAllocations"][0]["id"], "display-a");
    let _ = fs::remove_dir_all(&home);
}
