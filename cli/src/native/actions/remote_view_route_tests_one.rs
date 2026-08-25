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
use crate::native::presentation_capacity::{
    PresentationCapacityAuthority, PresentationCapacityConfig, PresentationSlot,
    PresentationSlotState,
};
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
async fn test_remote_view_route_and_lease_actions_mutate_service_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-lease-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            display_allocations: BTreeMap::from([(
                "display-a".to_string(),
                DisplayAllocation {
                    id: "display-a".to_string(),
                    display_name: Some(":21".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    route_descriptor: Some(json!(
                        { "localEmbedUrl" :
                        "http://127.0.0.1:8092/guacamole/#/client/route-a",
                        "dashboardEmbedUrl" :
                        "https://dashboard.example/guacamole/#/client/route-a",
                        "publicOperatorUrl" :
                        "https://guac.example/#/client/route-a", "healthUrl" :
                        "http://127.0.0.1:8092/guacamole/#/client/route-a",
                        "externalUrl" : "https://guac.example/#/client/route-a" }
                    )),
                    target: json!(
                        { "displayName" : ":21", "displayIsolation" :
                        "shared_display", "routeUser" : "agent-browser-rdp-a",
                        "displayAccess" : { "state" : "ready" } }
                    ),
                    provider_mode: "single_controller".to_string(),
                    state: "available".to_string(),
                    readiness: Some(json!(
                        { "status" : "ready", "components" : [{ "component" :
                        "guacamole_web", "status" : "ready", "evidence" :
                        "retained guacamole web readiness", "observedAt" :
                        "2026-06-23T00:00:00Z", "nextAction" : "none" }, {
                        "component" : "guacamole_login", "status" : "ready",
                        "evidence" : "retained guacamole login readiness",
                        "observedAt" : "2026-06-23T00:00:00Z", "nextAction" : "none"
                        }, { "component" : "guacamole_connection_permissions",
                        "status" : "ready", "evidence" :
                        "retained connection permissions readiness", "observedAt" :
                        "2026-06-23T00:00:00Z", "nextAction" : "none" }, {
                        "component" : "rdp_backend_tcp:route-a", "status" : "ready",
                        "evidence" : "retained RDP backend readiness", "observedAt"
                        : "2026-06-23T00:00:00Z", "nextAction" : "none" }] }
                    )),
                    ..RoutePoolEntry::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:rdp-a".to_string(),
                BrowserProcess {
                    id: "session:rdp-a".to_string(),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-a".to_string();
    let preflight = execute_command(
        &json!(
            { "action" : "service_remote_view_route_preflight", "displayAllocationId"
            : "display-a", "routePoolEntryId" : "pool-a", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(preflight["success"], true);
    assert_eq!(preflight["data"]["status"], "preflight_ready");
    assert_eq!(preflight["data"]["routeBinding"]["routeId"], "route-a");
    assert_eq!(preflight["data"]["routeBinding"]["displayName"], ":21");
    assert_eq!(preflight["data"]["fastPreflight"]["noLaunch"], true);
    assert_eq!(
        preflight["data"]["fastPreflight"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["component"] == "guacamole_route_url")
            .unwrap()["status"],
        "ready"
    );
    assert_eq!(
        preflight["data"]["fastPreflight"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["component"] == "guacamole_login")
            .unwrap()["freshness"]["observedAt"],
        "2026-06-23T00:00:00Z"
    );
    assert!(preflight["data"]["fastPreflight"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .any(|component| component["component"] == "privileged_helper_status"));
    assert!(preflight["data"]["fastPreflight"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .any(|component| component["component"] == "display_access"));
    assert_eq!(
        store.load().unwrap().route_pool["pool-a"].state,
        "available"
    );
    let checkout = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "display-a", "routePoolEntryId" : "pool-a", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a", "streamId" :
            "remote-headed-view" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(checkout["success"], true);
    assert_eq!(checkout["data"]["routeId"], "route-a");
    assert_eq!(checkout["data"]["displayAllocationId"], "display-a");
    assert_eq!(checkout["data"]["routeBinding"]["displayName"], ":21");
    assert_eq!(checkout["data"]["routeBinding"]["launchDisplayName"], ":21");
    assert_eq!(
        checkout["data"]["routeBinding"]["displayIsolation"],
        "shared_display"
    );
    assert_eq!(
        checkout["data"]["routeBinding"]["routeUser"],
        "agent-browser-rdp-a"
    );
    assert_eq!(
        checkout["data"]["acquisitionPlan"]["mode"],
        "strict_operator_open"
    );
    assert_eq!(
        checkout["data"]["acquisitionPlan"]["selectedRoutePoolEntryId"],
        "pool-a"
    );
    assert_eq!(checkout["data"]["routePoolEntry"]["state"], "checked_out");
    assert_eq!(
        checkout["data"]["routePoolEntry"]["readiness"]["state"],
        "ready"
    );
    assert_eq!(
        checkout["data"]["remoteViewRoute"]["lastProviderEvent"],
        "route_checked_out"
    );
    let viewer = execute_command(
        &json!(
            { "action" : "service_viewer_lease_request", "routeId" : "route-a",
            "viewerId" : "viewer-a", "viewerName" : "Operator A", "openMode" : "tile"
            }
        ),
        &mut state,
    )
    .await;
    assert_eq!(viewer["success"], true);
    let viewer_lease_id = viewer["data"]["viewerLeaseId"]
        .as_str()
        .unwrap()
        .to_string();
    let heartbeat = execute_command(
        &json!(
            { "action" : "service_viewer_lease_heartbeat", "viewerLeaseId" :
            viewer_lease_id, "expiresAt" : "2026-05-28T04:00:00Z" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(heartbeat["success"], true);
    assert_eq!(heartbeat["data"]["status"], "viewer_heartbeat");
    let controller = execute_command(
        &json!(
            { "action" : "service_controller_lease_takeover", "routeId" : "route-a",
            "viewerLeaseId" : viewer_lease_id, "viewerId" : "viewer-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(controller["success"], true);
    assert_eq!(controller["data"]["controllerLeaseId"], viewer_lease_id);
    let release_viewer = execute_command(
        &json!(
            { "action" : "service_viewer_lease_release", "viewerLeaseId" :
            viewer_lease_id }
        ),
        &mut state,
    )
    .await;
    assert_eq!(release_viewer["success"], true);
    let release_route = execute_command(
        &json!(
            { "action" : "service_remote_view_route_release", "routeId" : "route-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(release_route["success"], true);
    assert_eq!(release_route["data"]["status"], "released");
    assert_eq!(release_route["data"]["routeId"], "route-a");
    assert!(release_route["data"]["releasedViewerLeaseIds"].is_array());
    assert_eq!(
        release_route["data"]["remoteViewRoute"]["lastProviderEvent"],
        "route_released"
    );
    let persisted = store.load().unwrap();
    let route = persisted.remote_view_routes.get("route-a").unwrap();
    assert_eq!(route.state, "released");
    assert_eq!(
        route
            .route_descriptor
            .as_ref()
            .unwrap()
            .get("publicOperatorUrl")
            .and_then(Value::as_str),
        Some("https://guac.example/#/client/route-a")
    );
    assert_eq!(route.controller_lease_id, None);
    assert_eq!(
        persisted.route_pool["pool-a"].current_route_allocation_id,
        None
    );
    assert_eq!(persisted.route_pool["pool-a"].state, "available");
    assert_eq!(
        persisted.route_pool["pool-a"]
            .readiness
            .as_ref()
            .and_then(|readiness| readiness.get("state"))
            .and_then(Value::as_str),
        Some("ready")
    );
    assert_eq!(
        persisted.viewer_leases[&viewer_lease_id].state,
        "disconnected"
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::ViewerConnected
            && event.details.as_ref().unwrap()["viewerLeaseId"] == viewer_lease_id
    }));
    assert!(persisted
        .events
        .iter()
        .any(|event| event.kind == ServiceEventKind::ControllerRequested));
    assert!(persisted
        .events
        .iter()
        .any(|event| event.kind == ServiceEventKind::ControllerGranted));
    assert!(persisted
        .events
        .iter()
        .any(|event| event.kind == ServiceEventKind::ViewerDisconnected));
    assert!(persisted
        .events
        .iter()
        .any(|event| event.kind == ServiceEventKind::RouteReleased));
    assert_eq!(
        persisted.browsers["session:rdp-a"].view_streams[0]
            .route_id
            .as_deref(),
        Some("route-a")
    );
    assert_eq!(
        persisted.browsers["session:rdp-a"].view_streams[0]
            .route_descriptor
            .as_ref()
            .unwrap()
            .get("dashboardEmbedUrl")
            .and_then(Value::as_str),
        Some("https://dashboard.example/guacamole/#/client/route-a")
    );
    assert!(state.browser.is_none());
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_checkout_and_release_clear_stale_acquisition_pending_readiness() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-stale-pending-route-readiness-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            display_allocations: BTreeMap::from([(
                "remote-view-display:13".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:13".to_string(),
                    display_name: Some(":13".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:default".to_string()),
                    owner_session_id: Some("default".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "guacamole-rdp-a".to_string(),
                RoutePoolEntry {
                    id: "guacamole-rdp-a".to_string(),
                    route_id: "guacamole:3".to_string(),
                    frame_url: Some(
                        "https://agent-browser.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some(
                        "https://agent-browser.example/guacamole/#/client/route-a".to_string(),
                    ),
                    target: json!(
                        { "displayName" : ":13", "displayIsolation" :
                        "shared_display" }
                    ),
                    provider_mode: "simultaneous_view".to_string(),
                    state: "available".to_string(),
                    current_route_allocation_id: None,
                    readiness: Some(json!(
                        { "state" : "pending", "component" :
                        "remote_view_open_acquisition", "leaseId" :
                        "remote-view-open:default:guacamole-3:stale" }
                    )),
                    ..RoutePoolEntry::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    display_allocation_id: Some("remote-view-display:13".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "default".to_string();
    let checkout = execute_command(
        &json!(
            { "action" : "service_remote_view_route_checkout", "displayAllocationId"
            : "remote-view-display:13", "routePoolEntryId" : "guacamole-rdp-a",
            "browserId" : "session:default", "sessionName" : "default", "streamId" :
            "remote-headed-view" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(
        checkout["success"], true,
        "checkout should tolerate stale acquisition-pending readiness: {checkout}"
    );
    assert_eq!(checkout["data"]["routePoolEntry"]["state"], "checked_out");
    assert_eq!(
        checkout["data"]["routePoolEntry"]["readiness"]["state"],
        "ready"
    );
    let mut snapshot = store.load().unwrap();
    snapshot
        .route_pool
        .get_mut("guacamole-rdp-a")
        .unwrap()
        .readiness = Some(json!(
        { "state" : "pending", "component" : "remote_view_open_acquisition",
        "leaseId" : "remote-view-open:default:guacamole-3:stale-after-checkout" }
    ));
    store.save(&snapshot).unwrap();
    let release = execute_command(
        &json!(
            { "action" : "service_remote_view_route_release", "routeId" :
            "guacamole:3" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(release["success"], true);
    let persisted = store.load().unwrap();
    let entry = persisted.route_pool.get("guacamole-rdp-a").unwrap();
    assert_eq!(entry.state, "available");
    assert_eq!(entry.current_route_allocation_id, None);
    assert_eq!(
        entry
            .readiness
            .as_ref()
            .and_then(|readiness| readiness.get("state"))
            .and_then(Value::as_str),
        Some("ready")
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_browser_reattach_reuses_retained_browser_without_duplicate_row() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-browser-reattach-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            presentation_capacity: Some(
                PresentationCapacityAuthority::new(
                    PresentationCapacityConfig {
                        warm_minimum: 1,
                        hard_maximum: 1,
                        human_priority_reserve: 0,
                        recovery_reserve: 0,
                        max_queue_depth: 8,
                    },
                    vec![{
                        let mut slot = PresentationSlot::warm_idle("slot:pool-a")
                            .with_binding("route-a", "display-a");
                        slot.state = PresentationSlotState::Active;
                        slot.browser_id = Some("session:rdp-a".to_string());
                        slot
                    }],
                )
                .unwrap(),
            ),
            display_allocations: BTreeMap::from([(
                "display-a".to_string(),
                DisplayAllocation {
                    id: "display-a".to_string(),
                    display_name: Some(":21".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:rdp-a".to_string()),
                    owner_session_id: Some("rdp-a".to_string()),
                    state: "ready".to_string(),
                    route_ids: vec!["route-a".to_string()],
                    ..DisplayAllocation::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!(
                        { "displayAllocationId" : "display-a", "displayName" :
                        ":21", "displayIsolation" : "shared_display",
                        "displayAccess" : { "state" : "ready" } }
                    ),
                    provider_mode: "simultaneous_view".to_string(),
                    state: "pending".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!(
                        { "state" : "pending", "reason" : "stale_pending_route", }
                    )),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("display-stale".to_string()),
                    browser_id: Some("session:rdp-a".to_string()),
                    session_id: Some("rdp-a".to_string()),
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    provider_mode: "simultaneous_view".to_string(),
                    state: "orphaned".to_string(),
                    last_provider_event: Some("display_allocation_unavailable".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:rdp-a".to_string(),
                BrowserProcess {
                    id: "session:rdp-a".to_string(),
                    host: ServiceBrowserHost::AttachedExisting,
                    health: ServiceBrowserHealth::Ready,
                    display_allocation_id: Some("display-a".to_string()),
                    active_session_ids: vec!["rdp-a".to_string()],
                    view_streams: vec![ViewStream {
                        id: "remote-headed-view".to_string(),
                        provider: ViewStreamProvider::RdpGateway,
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-a".to_string()),
                        route_id: Some("route-a".to_string()),
                        display_allocation_id: Some("display-a".to_string()),
                        provider_mode: Some("simultaneous_view".to_string()),
                        remote_readiness: Some(json!({ "state" : "orphaned",
                                "displayContent" : { "state" : "browser_window_visible",
                                "displayName" : ":21" } })),
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-a".to_string();
    let result = execute_command(
        &json!(
            { "action" : "service_remote_view_browser_reattach", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a", "streamId" :
            "remote-headed-view" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["data"]["status"], "reattached");
    assert_eq!(result["data"]["routeId"], "route-a");
    assert_eq!(result["data"]["recoveryAdmission"]["status"], "granted");
    assert_eq!(result["data"]["recoveryAdmission"]["slotId"], "slot:pool-a");
    assert_eq!(result["data"]["recoveryRelease"]["status"], "released");
    assert_eq!(
        result["data"]["checkout"]["attachability"]["state"],
        "attached_ready"
    );
    let persisted = store.load().unwrap();
    assert_eq!(persisted.browsers.len(), 1);
    assert!(persisted.browsers.contains_key("session:rdp-a"));
    let browser = persisted.browsers.get("session:rdp-a").unwrap();
    assert_eq!(browser.view_streams.len(), 1);
    assert_eq!(browser.view_streams[0].route_id.as_deref(), Some("route-a"));
    assert_eq!(persisted.route_pool["pool-a"].state, "checked_out");
    assert_eq!(
        persisted.display_allocations["display-a"].host,
        Some(ServiceBrowserHost::RemoteHeaded)
    );
    assert_eq!(
        persisted.remote_view_routes["route-a"]
            .display_allocation_id
            .as_deref(),
        Some("display-a")
    );
    assert_eq!(persisted.remote_view_routes["route-a"].state, "ready");
    let slot = &persisted.presentation_capacity.as_ref().unwrap().slots[0];
    assert_eq!(slot.state, PresentationSlotState::Active);
    assert_eq!(slot.browser_id.as_deref(), Some("session:rdp-a"));
    assert_eq!(slot.lease_request_id, None);
    assert_eq!(slot.lease_priority, None);
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_switch_releases_previous_route_and_checks_out_new_route() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-switch-home");
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
                        display_name: Some(":21".to_string()),
                        display_isolation: "shared_display".to_string(),
                        owner_browser_id: Some("session:rdp-a".to_string()),
                        owner_session_id: Some("rdp-a".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-a".to_string()],
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-b".to_string(),
                    DisplayAllocation {
                        id: "display-b".to_string(),
                        display_name: Some(":22".to_string()),
                        display_isolation: "shared_display".to_string(),
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
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-a".to_string()),
                        target: json!(
                            { "displayAllocationId" : "display-a", "displayName" :
                            ":21", "displayIsolation" : "shared_display",
                            "displayAccess" : { "state" : "ready" } }
                        ),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("route-a".to_string()),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-b".to_string(),
                    RoutePoolEntry {
                        id: "pool-b".to_string(),
                        route_id: "route-stale".to_string(),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-stale".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-stale".to_string()),
                        target: json!(
                            { "displayAllocationId" : "display-stale", "displayName" :
                            ":99", "displayIsolation" : "shared_display",
                            "displayAccess" : { "state" : "ready" } }
                        ),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "available".to_string(),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RoutePoolEntry::default()
                    },
                ),
            ]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("display-a".to_string()),
                    browser_id: Some("session:rdp-a".to_string()),
                    session_id: Some("rdp-a".to_string()),
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    provider_mode: "simultaneous_view".to_string(),
                    state: "ready".to_string(),
                    readiness: Some(json!({ "state" : "ready" })),
                    ..RemoteViewRoute::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:rdp-a".to_string(),
                BrowserProcess {
                    id: "session:rdp-a".to_string(),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    display_allocation_id: Some("display-a".to_string()),
                    active_session_ids: vec!["rdp-a".to_string()],
                    view_streams: vec![ViewStream {
                        id: "remote-headed-view".to_string(),
                        provider: ViewStreamProvider::RdpGateway,
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-a".to_string()),
                        route_id: Some("route-a".to_string()),
                        display_allocation_id: Some("display-a".to_string()),
                        provider_mode: Some("simultaneous_view".to_string()),
                        remote_readiness: Some(json!({ "state" : "ready",
                                "displayContent" : { "state" : "browser_window_visible",
                                "displayName" : ":21" } })),
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-a".to_string();
    let result = execute_command(
        &json!(
            { "action" : "service_remote_view_route_switch", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a", "streamId" :
            "remote-headed-view", "routePoolEntryId" : "pool-b", "routeId" :
            "route-b", "remoteViewRouteId" : "route-b", "displayAllocationId" :
            "display-b", "frameUrl" :
            "https://dashboard.example/guacamole/#/client/route-b", "externalUrl" :
            "https://guac.example/#/client/route-b", "routePoolEntry" : { "id" :
            "pool-b", "provider" : "rdp_gateway", "routeId" : "route-b", "frameUrl" :
            "https://dashboard.example/guacamole/#/client/route-b", "externalUrl" :
            "https://guac.example/#/client/route-b", "target" : {
            "displayAllocationId" : "display-b", "displayName" : ":22",
            "displayIsolation" : "shared_display", "displayAccess" : { "state" :
            "ready" } }, "providerMode" : "simultaneous_view", "state" : "available",
            "readiness" : { "state" : "ready" } } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["data"]["status"], "route_switched");
    assert_eq!(result["data"]["previousRouteId"], "route-a");
    assert_eq!(result["data"]["newRouteId"], "route-b");
    assert_eq!(result["data"]["routeSwitchRelease"]["status"], "released");
    assert_eq!(
        result["data"]["checkout"]["attachability"]["state"],
        "attached_ready"
    );
    let persisted = store.load().unwrap();
    assert_eq!(persisted.browsers.len(), 1);
    let browser = persisted.browsers.get("session:rdp-a").unwrap();
    assert_eq!(browser.display_allocation_id.as_deref(), Some("display-b"));
    assert_eq!(browser.view_streams[0].route_id.as_deref(), Some("route-b"));
    assert_eq!(
        browser.view_streams[0].display_allocation_id.as_deref(),
        Some("display-b")
    );
    assert_eq!(persisted.route_pool["pool-a"].state, "available");
    assert_eq!(
        persisted.route_pool["pool-a"].current_route_allocation_id,
        None
    );
    assert_eq!(persisted.display_allocations["display-a"].state, "released");
    assert_eq!(persisted.route_pool["pool-b"].state, "checked_out");
    assert_eq!(
        persisted.route_pool["pool-b"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-b")
    );
    assert_eq!(persisted.remote_view_routes["route-a"].state, "released");
    assert_eq!(persisted.remote_view_routes["route-b"].state, "ready");
    assert_eq!(
        persisted.remote_view_routes["route-b"]
            .display_allocation_id
            .as_deref(),
        Some("display-b")
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_switch_reuses_route_released_by_previous_switch() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-switch-reuse-home");
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
                        display_name: Some(":21".to_string()),
                        display_isolation: "shared_display".to_string(),
                        owner_browser_id: Some("session:rdp-a".to_string()),
                        owner_session_id: Some("rdp-a".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-a".to_string()],
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-b".to_string(),
                    DisplayAllocation {
                        id: "display-b".to_string(),
                        display_name: Some(":22".to_string()),
                        display_isolation: "shared_display".to_string(),
                        owner_browser_id: Some("session:rdp-b".to_string()),
                        owner_session_id: Some("rdp-b".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-b".to_string()],
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
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-a".to_string()),
                        target: json!(
                            { "displayAllocationId" : "display-a", "displayName" :
                            ":21", "displayIsolation" : "shared_display",
                            "displayAccess" : { "state" : "ready" } }
                        ),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("route-a".to_string()),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-b".to_string(),
                    RoutePoolEntry {
                        id: "pool-b".to_string(),
                        route_id: "route-b".to_string(),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-b".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-b".to_string()),
                        target: json!(
                            { "displayAllocationId" : "display-b", "displayName" :
                            ":22", "displayIsolation" : "shared_display",
                            "displayAccess" : { "state" : "ready" } }
                        ),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("route-b".to_string()),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RoutePoolEntry::default()
                    },
                ),
            ]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-a".to_string(),
                    RemoteViewRoute {
                        id: "route-a".to_string(),
                        display_allocation_id: Some("display-a".to_string()),
                        browser_id: Some("session:rdp-a".to_string()),
                        session_id: Some("rdp-a".to_string()),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-a".to_string()),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "ready".to_string(),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-b".to_string(),
                    RemoteViewRoute {
                        id: "route-b".to_string(),
                        display_allocation_id: Some("display-b".to_string()),
                        browser_id: Some("session:rdp-b".to_string()),
                        session_id: Some("rdp-b".to_string()),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-b".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-b".to_string()),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "ready".to_string(),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RemoteViewRoute::default()
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
                        active_session_ids: vec!["rdp-a".to_string()],
                        view_streams: vec![ViewStream {
                            id: "remote-headed-view".to_string(),
                            provider: ViewStreamProvider::RdpGateway,
                            route_id: Some("route-a".to_string()),
                            display_allocation_id: Some("display-a".to_string()),
                            provider_mode: Some("simultaneous_view".to_string()),
                            remote_readiness: Some(json!({ "state" :
                                "ready", "displayContent" : { "state" :
                                "browser_window_visible", "displayName" : ":21" } })),
                            ..ViewStream::default()
                        }],
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
                        active_session_ids: vec!["rdp-b".to_string()],
                        view_streams: vec![ViewStream {
                            id: "remote-headed-view".to_string(),
                            provider: ViewStreamProvider::RdpGateway,
                            route_id: Some("route-b".to_string()),
                            display_allocation_id: Some("display-b".to_string()),
                            provider_mode: Some("simultaneous_view".to_string()),
                            remote_readiness: Some(json!({ "state" :
                                "ready", "displayContent" : { "state" :
                                "browser_window_visible", "displayName" : ":22" } })),
                            ..ViewStream::default()
                        }],
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-b".to_string();
    let switch_b_to_a = execute_command(
        &json!(
            { "action" : "service_remote_view_route_switch", "browserId" :
            "session:rdp-b", "sessionName" : "rdp-b", "routePoolEntryId" : "pool-a",
            "routeId" : "route-a", "remoteViewRouteId" : "route-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(switch_b_to_a["success"], true, "{switch_b_to_a}");
    assert_eq!(
        switch_b_to_a["data"]["routeSwitchParking"]["browserId"],
        "session:rdp-a"
    );
    let after_first_switch = store.load().unwrap();
    assert_eq!(
        after_first_switch.display_allocations["display-b"].state,
        "released"
    );
    assert_eq!(after_first_switch.route_pool["pool-b"].state, "available");
    state.session_id = "rdp-a".to_string();
    let switch_a_to_available = execute_command(
        &json!(
            { "action" : "service_remote_view_route_switch", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(
        switch_a_to_available["success"], true,
        "{switch_a_to_available}"
    );
    assert_eq!(switch_a_to_available["data"]["routeId"], "route-b");
    assert_eq!(
        switch_a_to_available["data"]["checkout"]["attachability"]["state"],
        "attached_ready"
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.remote_view_routes["route-a"]
            .browser_id
            .as_deref(),
        Some("session:rdp-b")
    );
    assert_eq!(
        persisted.remote_view_routes["route-b"]
            .browser_id
            .as_deref(),
        Some("session:rdp-a")
    );
    assert_eq!(
        persisted.display_allocations["display-b"]
            .owner_browser_id
            .as_deref(),
        Some("session:rdp-a")
    );
    assert_eq!(persisted.route_pool["pool-a"].state, "checked_out");
    assert_eq!(persisted.route_pool["pool-b"].state, "checked_out");
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_route_switch_parks_occupied_route_when_no_route_available() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-switch-parking-home");
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
                        display_name: Some(":21".to_string()),
                        display_isolation: "shared_display".to_string(),
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
                        display_name: Some(":22".to_string()),
                        display_isolation: "shared_display".to_string(),
                        owner_browser_id: Some("session:rdp-b".to_string()),
                        owner_session_id: Some("rdp-b".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-b".to_string()],
                        ..DisplayAllocation::default()
                    },
                ),
                (
                    "display-c".to_string(),
                    DisplayAllocation {
                        id: "display-c".to_string(),
                        display_name: Some(":23".to_string()),
                        display_isolation: "shared_display".to_string(),
                        owner_browser_id: Some("session:rdp-c".to_string()),
                        owner_session_id: Some("rdp-c".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-c".to_string()],
                        ..DisplayAllocation::default()
                    },
                ),
            ]),
            route_pool: BTreeMap::from([
                (
                    "pool-b".to_string(),
                    RoutePoolEntry {
                        id: "pool-b".to_string(),
                        route_id: "route-b".to_string(),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-b".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-b".to_string()),
                        target: json!(
                            { "displayAllocationId" : "display-b", "displayName" :
                            ":22", "displayIsolation" : "shared_display",
                            "displayAccess" : { "state" : "ready" } }
                        ),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("route-b".to_string()),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RoutePoolEntry::default()
                    },
                ),
                (
                    "pool-c".to_string(),
                    RoutePoolEntry {
                        id: "pool-c".to_string(),
                        route_id: "route-c".to_string(),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-c".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-c".to_string()),
                        target: json!(
                            { "displayAllocationId" : "display-c", "displayName" :
                            ":23", "displayIsolation" : "shared_display",
                            "displayAccess" : { "state" : "ready" } }
                        ),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("route-c".to_string()),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RoutePoolEntry::default()
                    },
                ),
            ]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-b".to_string(),
                    RemoteViewRoute {
                        id: "route-b".to_string(),
                        display_allocation_id: Some("display-b".to_string()),
                        browser_id: Some("session:rdp-b".to_string()),
                        session_id: Some("rdp-b".to_string()),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-b".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-b".to_string()),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "ready".to_string(),
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RemoteViewRoute::default()
                    },
                ),
                (
                    "route-c".to_string(),
                    RemoteViewRoute {
                        id: "route-c".to_string(),
                        display_allocation_id: Some("display-c".to_string()),
                        browser_id: Some("session:rdp-c".to_string()),
                        session_id: Some("rdp-c".to_string()),
                        frame_url: Some(
                            "https://dashboard.example/guacamole/#/client/route-c".to_string(),
                        ),
                        external_url: Some("https://guac.example/#/client/route-c".to_string()),
                        provider_mode: "simultaneous_view".to_string(),
                        state: "ready".to_string(),
                        viewer_lease_ids: vec!["viewer-c".to_string()],
                        readiness: Some(json!({ "state" : "ready" })),
                        ..RemoteViewRoute::default()
                    },
                ),
            ]),
            viewer_leases: BTreeMap::from([(
                "viewer-c".to_string(),
                ViewerLease {
                    id: "viewer-c".to_string(),
                    state: "observing".to_string(),
                    route_id: Some("route-c".to_string()),
                    browser_id: Some("session:rdp-c".to_string()),
                    last_heartbeat_at: Some("2026-07-05T00:01:00Z".to_string()),
                    ..ViewerLease::default()
                },
            )]),
            browsers: BTreeMap::from([
                (
                    "session:rdp-a".to_string(),
                    BrowserProcess {
                        id: "session:rdp-a".to_string(),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_allocation_id: Some("display-a".to_string()),
                        active_session_ids: vec!["rdp-a".to_string()],
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
                        active_session_ids: vec!["rdp-b".to_string()],
                        view_streams: vec![ViewStream {
                            id: "remote-headed-view".to_string(),
                            provider: ViewStreamProvider::RdpGateway,
                            route_id: Some("route-b".to_string()),
                            display_allocation_id: Some("display-b".to_string()),
                            provider_mode: Some("simultaneous_view".to_string()),
                            remote_readiness: Some(json!({ "state" :
                                "ready", "displayContent" : { "state" :
                                "browser_window_visible", "displayName" : ":22" } })),
                            ..ViewStream::default()
                        }],
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "session:rdp-c".to_string(),
                    BrowserProcess {
                        id: "session:rdp-c".to_string(),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_allocation_id: Some("display-c".to_string()),
                        active_session_ids: vec!["rdp-c".to_string()],
                        view_streams: vec![ViewStream {
                            id: "remote-headed-view".to_string(),
                            provider: ViewStreamProvider::RdpGateway,
                            route_id: Some("route-c".to_string()),
                            display_allocation_id: Some("display-c".to_string()),
                            provider_mode: Some("simultaneous_view".to_string()),
                            viewer_lease_ids: vec!["viewer-c".to_string()],
                            remote_readiness: Some(json!({ "state" :
                                "ready", "displayContent" : { "state" :
                                "browser_window_visible", "displayName" : ":23" } })),
                            ..ViewStream::default()
                        }],
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-a".to_string();
    let result = execute_command(
        &json!(
            { "action" : "service_remote_view_route_switch", "browserId" :
            "session:rdp-a", "sessionName" : "rdp-a" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["data"]["status"], "route_switched");
    assert_eq!(result["data"]["routeId"], "route-b");
    assert_eq!(result["data"]["routePoolEntryId"], "pool-b");
    assert_eq!(
        result["data"]["routeSwitchParking"]["browserId"],
        "session:rdp-b"
    );
    assert_eq!(
        result["data"]["routeSwitchParking"]["release"]["status"],
        "released"
    );
    assert_eq!(
        result["data"]["checkout"]["attachability"]["state"],
        "attached_ready"
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.remote_view_routes["route-b"]
            .browser_id
            .as_deref(),
        Some("session:rdp-a")
    );
    assert_eq!(
        persisted.route_pool["pool-b"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-b")
    );
    assert_eq!(
        persisted.browsers["session:rdp-a"].view_streams[0]
            .route_id
            .as_deref(),
        Some("route-b")
    );
    assert_eq!(
        persisted.browsers["session:rdp-b"]
            .attachability
            .as_ref()
            .unwrap()["state"],
        "reattachable_stale_route"
    );
    assert_eq!(
        persisted.remote_view_routes["route-c"]
            .browser_id
            .as_deref(),
        Some("session:rdp-c")
    );
    assert_eq!(
        persisted.route_pool["pool-c"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-c")
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-open-dry-run-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    route_descriptor: Some(json!(
                        { "localEmbedUrl" :
                        "http://127.0.0.1:8092/guacamole/#/client/route-a",
                        "dashboardEmbedUrl" :
                        "https://dashboard.example/guacamole/#/client/route-a",
                        "publicOperatorUrl" :
                        "https://guac.example/#/client/route-a", "healthUrl" :
                        "http://127.0.0.1:8092/guacamole/#/client/route-a",
                        "externalUrl" : "https://guac.example/#/client/route-a" }
                    )),
                    target: json!(
                        { "displayName" : ":31", "displayIsolation" :
                        "shared_display", "routeUser" : "agent-browser-rdp-a",
                        "displayAccess" : { "state" : "ready" } }
                    ),
                    provider_mode: "single_controller".to_string(),
                    state: "available".to_string(),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-open-a".to_string();
    let result = execute_command(
        &json!(
            { "action" : "remote_view_open", "routePoolEntryId" : "pool-a",
            "provider" : "rdp_gateway", "runtimeProfile" : "stealthcdp-default",
            "url" : "https://www.linkedin.com/", "dryRun" : true }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["status"], "planned");
    assert_eq!(result["data"]["dryRun"], true);
    assert_eq!(result["data"]["routeId"], "route-a");
    assert_eq!(result["data"]["operatorVisible"]["state"], "not_checked");
    assert_eq!(
        result["data"]["operatorVisible"]["browserId"],
        "session:rdp-open-a"
    );
    assert_eq!(
        result["data"]["operatorVisible"]["sessionName"],
        "rdp-open-a"
    );
    assert_eq!(result["data"]["operatorVisible"]["displayName"], ":31");
    assert_eq!(result["data"]["routeBoundHandoff"]["state"], "planned");
    assert_eq!(
        result["data"]["routeBoundHandoff"]["profile"]["id"],
        "stealthcdp-default"
    );
    assert_eq!(
        result["data"]["routeBoundHandoff"]["browser"]["browserId"],
        "session:rdp-open-a"
    );
    assert_eq!(
        result["data"]["routeBoundHandoff"]["route"]["routeId"],
        "route-a"
    );
    assert_eq!(
        result["data"]["routeBoundHandoff"]["display"]["displayAllocationId"],
        "remote-view-display:31"
    );
    assert_eq!(
        result["data"]["routeBoundHandoff"]["operatorVisible"]["state"],
        "not_checked"
    );
    assert_eq!(
        result["data"]["displayAllocationId"],
        "remote-view-display:31"
    );
    assert_eq!(
        result["data"]["acquisitionPlan"]["mode"],
        "strict_operator_open"
    );
    assert_eq!(
        result["data"]["acquisitionPlan"]["selectedRoutePoolEntryId"],
        "pool-a"
    );
    assert_eq!(
        result["data"]["acquisitionPlan"]["selectedRouteId"],
        "route-a"
    );
    assert!(result["data"]["acquisitionPlan"]["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|decision| decision["step"] == "route_pool_entry"
            && decision["reason"] == "available_or_explicit_route_pool_entry"));
    assert_eq!(result["data"]["routeBinding"]["launchDisplayName"], ":31");
    assert_eq!(
        result["data"]["launchCommand"]["browserHost"],
        "remote_headed"
    );
    assert_eq!(
        result["data"]["launchCommand"]["remoteHeadedDisplay"],
        ":31"
    );
    assert_eq!(
        result["data"]["launchCommand"]["displayIsolation"],
        "shared_display"
    );
    assert!(result["data"]["launchCommand"]["provider"].is_null());
    assert_eq!(
        result["data"]["launchCommand"]["viewStreamProvider"],
        "rdp_gateway"
    );
    assert_eq!(
        result["data"]["launchCommand"]["routeDescriptor"]["dashboardEmbedUrl"],
        "https://dashboard.example/guacamole/#/client/route-a"
    );
    assert_eq!(
        result["data"]["tabCommand"]["url"],
        "https://www.linkedin.com/"
    );
    assert_eq!(
        result["data"]["checkoutCommand"]["displayAllocationId"],
        "remote-view-display:31"
    );
    assert!(state.browser.is_none());
    assert_eq!(
        store.load().unwrap().route_pool["pool-a"].state,
        "available"
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_open_dry_run_accepts_inline_route_pool_entry_and_display() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-open-inline-route-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store.save(&ServiceState::default()).unwrap();
    let mut state = DaemonState::new();
    state.session_id = "rdp-open-inline".to_string();
    let result = execute_command(
        &json!(
            { "action" : "remote_view_open", "runtimeProfile" : "stealthcdp-default",
            "url" : "https://www.linkedin.com/", "remoteHeadedDisplay" : ":10",
            "displayIsolation" : "shared_display", "dryRun" : true, "routePoolEntry"
            : { "id" : "guacamole-rdp-a", "routeId" : "guacamole:1", "connectionId" :
            "1", "connectionName" : "Agent Browser RDP Existing User Route A",
            "frameUrl" : "http://127.0.0.1:8092/guacamole/#/client/route-a",
            "externalUrl" :
            "https://agent-browser.example/guacamole/#/client/route-a",
            "routeDescriptor" : { "dashboardEmbedUrl" :
            "http://127.0.0.1:8092/guacamole/#/client/route-a", "publicOperatorUrl" :
            "https://agent-browser.example/guacamole/#/client/route-a" },
            "providerMode" : "simultaneous_view", "readiness" : { "state" : "ready"
            }, "target" : { "hostname" : "host.docker.internal", "port" : "3389" } }
            }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["status"], "planned");
    assert_eq!(result["data"]["routePoolEntryId"], "guacamole-rdp-a");
    assert_eq!(result["data"]["operatorVisible"]["state"], "not_checked");
    assert_eq!(result["data"]["operatorVisible"]["routeId"], "guacamole:1");
    assert_eq!(result["data"]["operatorVisible"]["displayName"], ":10");
    assert_eq!(result["data"]["routeBinding"]["launchDisplayName"], ":10");
    assert_eq!(
        result["data"]["routeBinding"]["displayIsolation"],
        "shared_display"
    );
    assert_eq!(
        result["data"]["launchCommand"]["remoteHeadedDisplay"],
        ":10"
    );
    assert!(store.load().unwrap().route_pool.is_empty());
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_remote_view_open_dry_run_reuses_checked_out_same_route() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-open-reuse-route-home");
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
                    frame_url: Some("http://127.0.0.1:8092/guacamole/#/client/route-a".to_string()),
                    external_url: Some(
                        "https://agent-browser.example/guacamole/#/client/route-a".to_string(),
                    ),
                    route_descriptor: Some(json!(
                        { "dashboardEmbedUrl" :
                        "http://127.0.0.1:8092/guacamole/#/client/route-a",
                        "publicOperatorUrl" :
                        "https://agent-browser.example/guacamole/#/client/route-a" }
                    )),
                    target: json!(
                        { "displayName" : ":10", "displayIsolation" :
                        "shared_display" }
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
                    browser_id: Some("session:default".to_string()),
                    session_id: Some("default".to_string()),
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
                    owner_browser_id: Some("session:default".to_string()),
                    owner_session_id: Some("default".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    display_allocation_id: Some("remote-view-display:guacamole-1".to_string()),
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
            "https://www.linkedin.com/", "dryRun" : true }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true, "{}", result);
    assert_eq!(result["data"]["routeId"], "guacamole:1");
    assert_eq!(result["data"]["routeBinding"]["launchDisplayName"], ":10");
    assert_eq!(
        result["data"]["displayAllocationId"],
        "remote-view-display:guacamole-1"
    );
    assert_eq!(
        result["data"]["acquisitionPlan"]["selectedRoutePoolEntryId"],
        "guacamole-rdp-a"
    );
    assert!(result["data"]["acquisitionPlan"]["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|decision| decision["step"] == "route_pool_entry"
            && decision["reason"] == "same_owner_checked_out_route"));
    let _ = fs::remove_dir_all(&home);
}
