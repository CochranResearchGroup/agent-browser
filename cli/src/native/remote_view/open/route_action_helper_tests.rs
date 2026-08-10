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
async fn test_remote_view_open_cleanup_reports_new_browser_close_on_failure() {
    let mut state = DaemonState::new();
    let supervisor = RouteBoundOpenSupervisor::system(None, None);
    let mut runtime = DaemonRouteBoundOpenRuntime::new(&mut state);
    let cleanup = remote_view_open_cleanup_after_failure(
        &mut runtime,
        &supervisor,
        &RouteBoundHandoffFailureCleanupTask::CloseNewBrowser {
            command: json!({ "action" : "close" }),
        },
        None,
    )
    .await;
    let summary: Value = serde_json::from_str(
        &crate::native::remote_view_handoff::route_bound_handoff_cleanup_summary(&cleanup, None),
    )
    .unwrap();
    assert_eq!(summary["state"], "closed_new_browser");
    assert!(summary["leaseRollback"].is_null());
    assert_eq!(cleanup["result"]["closed"], true);
}
#[test]
fn test_remote_view_open_acquisition_lease_rollback_restores_route_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-open-lease-rollback-home");
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
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!(
                        { "displayName" : ":41", "displayIsolation" :
                        "shared_display" }
                    ),
                    state: "available".to_string(),
                    readiness: Some(json!({ "state" : "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let repository = LockedServiceStateRepository::default_json().unwrap();
    let snapshot = repository.load_snapshot().unwrap();
    let intent = normalize_remote_view_open_intent(&json!(
        { "action" : "remote_view_open", "routePoolEntryId" : "pool-a", "dryRun"
        : true }
    ))
    .unwrap();
    let acquisition_plan =
        plan_remote_view_acquisition(&snapshot, &intent, None, "session:lease-a", "lease-a")
            .unwrap();
    let observed_at = service_remote_view_timestamp();
    let lease = begin_route_bound_handoff_plan_acquisition(
        &repository,
        None,
        &acquisition_plan,
        "session:lease-a",
        "lease-a",
        &observed_at,
    )
    .unwrap();
    let pending = repository.load_snapshot().unwrap();
    assert_eq!(pending.route_pool["pool-a"].state, "pending");
    assert_eq!(
        pending.display_allocations["remote-view-display:41"].state,
        "pending"
    );
    assert_eq!(pending.remote_view_routes["route-a"].state, "pending");
    let rollback = crate::native::remote_view_handoff::rollback_route_bound_handoff_acquisition(
        &repository,
        &lease.id,
        "proof_failed",
        "forced proof failure",
        &json!({ "state" : "closed_new_browser" }),
        "2026-07-06T12:00:00Z",
    )
    .unwrap();
    assert_eq!(rollback["state"], "rolled_back");
    let restored = repository.load_snapshot().unwrap();
    assert_eq!(restored.route_pool["pool-a"].state, "available");
    assert!(!restored
        .display_allocations
        .contains_key("remote-view-display:41"));
    assert!(!restored.remote_view_routes.contains_key("route-a"));
    assert_eq!(
        restored.remote_view_acquisition_leases[&lease.id].state,
        "failed"
    );
    assert_eq!(
        restored.remote_view_acquisition_leases[&lease.id].phase,
        "rollback_complete"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_repair_route_pool_service_state_apply_rolls_back_stale_pending_acquisition() {
    let mut service_state = ServiceState {
        display_allocations: BTreeMap::from([(
            "display-pending".to_string(),
            DisplayAllocation {
                id: "display-pending".to_string(),
                state: "pending".to_string(),
                ..DisplayAllocation::default()
            },
        )]),
        remote_view_routes: BTreeMap::from([(
            "route-pending".to_string(),
            RemoteViewRoute {
                id: "route-pending".to_string(),
                state: "pending".to_string(),
                display_allocation_id: Some("display-pending".to_string()),
                browser_id: Some("session:pending".to_string()),
                ..RemoteViewRoute::default()
            },
        )]),
        route_pool: BTreeMap::from([(
            "pool-pending".to_string(),
            RoutePoolEntry {
                id: "pool-pending".to_string(),
                route_id: "route-pending".to_string(),
                state: "pending".to_string(),
                current_route_allocation_id: Some("route-pending".to_string()),
                ..RoutePoolEntry::default()
            },
        )]),
        remote_view_acquisition_leases: BTreeMap::from([(
            "lease-pending".to_string(),
            RemoteViewAcquisitionLease {
                id: "lease-pending".to_string(),
                browser_id: "session:pending".to_string(),
                session_id: "pending".to_string(),
                route_id: "route-pending".to_string(),
                display_allocation_id: "display-pending".to_string(),
                route_pool_entry_id: Some("pool-pending".to_string()),
                state: "pending".to_string(),
                phase: "reserved".to_string(),
                ..RemoteViewAcquisitionLease::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = repair_route_pool_service_state(
        &mut service_state,
        ServiceRoutePoolRepairOptions {
            apply: true,
            stale_checkouts: true,
            stale_pending_acquisitions: true,
        },
        "2026-06-23T14:00:00Z",
    );
    assert_eq!(result["repaired"], true);
    assert_eq!(result["candidateCounts"]["stalePendingAcquisitions"], 1);
    assert_eq!(result["repairedCounts"]["stalePendingAcquisitions"], 1);
    assert!(!service_state.route_pool.contains_key("pool-pending"));
    assert!(!service_state
        .remote_view_routes
        .contains_key("route-pending"));
    assert!(!service_state
        .display_allocations
        .contains_key("display-pending"));
    let lease = &service_state.remote_view_acquisition_leases["lease-pending"];
    assert_eq!(lease.state, "failed");
    assert_eq!(lease.phase, "rollback_complete");
    assert!(lease
        .failure_reason
        .as_deref()
        .unwrap_or_default()
        .contains("pending_acquisition_without_ready_browser"));
    assert_eq!(
        lease.cleanup.as_ref().unwrap()["cleanup"]["state"],
        "stale_pending_acquisition_repaired"
    );
}
#[test]
fn test_repair_route_pool_service_state_characterizes_completed_lease_pending_drift() {
    let mut service_state = ServiceState {
        display_allocations: BTreeMap::from([(
            "remote-view-display:13".to_string(),
            DisplayAllocation {
                id: "remote-view-display:13".to_string(),
                display_name: Some(":13".to_string()),
                display_isolation: "shared_display".to_string(),
                owner_browser_id: Some("session:default".to_string()),
                owner_session_id: Some("default".to_string()),
                state: "pending".to_string(),
                route_ids: vec!["guacamole:3".to_string()],
                readiness: Some(json!(
                    { "state" : "pending", "component" :
                    "remote_view_open_acquisition", "leaseId" :
                    "lease-completed", }
                )),
                ..DisplayAllocation::default()
            },
        )]),
        browsers: BTreeMap::from([(
            "session:default".to_string(),
            BrowserProcess {
                id: "session:default".to_string(),
                health: ServiceBrowserHealth::Ready,
                display_allocation_id: Some("remote-view-display:13".to_string()),
                display_name: Some(":13".to_string()),
                ..BrowserProcess::default()
            },
        )]),
        remote_view_routes: BTreeMap::from([(
            "guacamole:3".to_string(),
            RemoteViewRoute {
                id: "guacamole:3".to_string(),
                state: "orphaned".to_string(),
                display_allocation_id: Some("remote-view-display:13".to_string()),
                browser_id: Some("session:default".to_string()),
                session_id: Some("default".to_string()),
                last_provider_event: Some("display_allocation_unavailable".to_string()),
                readiness: Some(json!(
                    { "state" : "orphaned", "component" : "display_allocation",
                    "reason" : "display_allocation_unavailable", "entityId" :
                    "remote-view-display:13", "entityState" : "pending", }
                )),
                ..RemoteViewRoute::default()
            },
        )]),
        route_pool: BTreeMap::from([(
            "guacamole-rdp-a".to_string(),
            RoutePoolEntry {
                id: "guacamole-rdp-a".to_string(),
                route_id: "guacamole:3".to_string(),
                state: "pending".to_string(),
                current_route_allocation_id: Some("guacamole:3".to_string()),
                readiness: Some(json!(
                    { "state" : "pending", "component" :
                    "remote_view_open_acquisition", "leaseId" :
                    "lease-completed", }
                )),
                ..RoutePoolEntry::default()
            },
        )]),
        remote_view_acquisition_leases: BTreeMap::from([(
            "lease-completed".to_string(),
            RemoteViewAcquisitionLease {
                id: "lease-completed".to_string(),
                browser_id: "session:default".to_string(),
                session_id: "default".to_string(),
                route_id: "guacamole:3".to_string(),
                display_allocation_id: "remote-view-display:13".to_string(),
                route_pool_entry_id: Some("guacamole-rdp-a".to_string()),
                state: "completed".to_string(),
                phase: "checked_out".to_string(),
                completed_at: Some("2026-06-24T20:45:38Z".to_string()),
                ..RemoteViewAcquisitionLease::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = repair_route_pool_service_state(
        &mut service_state,
        ServiceRoutePoolRepairOptions {
            apply: false,
            stale_checkouts: true,
            stale_pending_acquisitions: true,
        },
        "2026-06-24T20:46:03Z",
    );
    assert_eq!(result["candidateCounts"]["stalePendingAcquisitions"], 0);
    assert_eq!(result["candidateCounts"]["staleCheckouts"], 0);
    assert_eq!(result["skipped"]["activeCheckouts"], json!([]));
    assert_eq!(
        service_state.route_pool["guacamole-rdp-a"]
            .readiness
            .as_ref()
            .unwrap()["state"],
        "pending"
    );
    assert_eq!(
        service_state.display_allocations["remote-view-display:13"].state,
        "pending"
    );
    assert_eq!(
        service_state.remote_view_routes["guacamole:3"].state,
        "orphaned"
    );
}
#[test]
fn test_repair_route_pool_service_state_apply_resets_stale_checkout_only() {
    let mut service_state = ServiceState {
        display_allocations: BTreeMap::from([
            (
                "display-active".to_string(),
                DisplayAllocation {
                    id: "display-active".to_string(),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-orphaned".to_string(),
                DisplayAllocation {
                    id: "display-orphaned".to_string(),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
        ]),
        browsers: BTreeMap::from([(
            "session:active".to_string(),
            BrowserProcess {
                id: "session:active".to_string(),
                health: ServiceBrowserHealth::Ready,
                ..BrowserProcess::default()
            },
        )]),
        remote_view_routes: BTreeMap::from([
            (
                "route-active".to_string(),
                RemoteViewRoute {
                    id: "route-active".to_string(),
                    state: "ready".to_string(),
                    display_allocation_id: Some("display-active".to_string()),
                    browser_id: Some("session:active".to_string()),
                    ..RemoteViewRoute::default()
                },
            ),
            (
                "route-orphaned".to_string(),
                RemoteViewRoute {
                    id: "route-orphaned".to_string(),
                    state: "orphaned".to_string(),
                    display_allocation_id: Some("display-orphaned".to_string()),
                    browser_id: Some("session:missing".to_string()),
                    ..RemoteViewRoute::default()
                },
            ),
        ]),
        route_pool: BTreeMap::from([
            (
                "pool-active".to_string(),
                RoutePoolEntry {
                    id: "pool-active".to_string(),
                    route_id: "route-active".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-active".to_string()),
                    ..RoutePoolEntry::default()
                },
            ),
            (
                "pool-orphaned".to_string(),
                RoutePoolEntry {
                    id: "pool-orphaned".to_string(),
                    route_id: "route-orphaned".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-orphaned".to_string()),
                    ..RoutePoolEntry::default()
                },
            ),
            (
                "pool-missing".to_string(),
                RoutePoolEntry {
                    id: "pool-missing".to_string(),
                    route_id: "route-missing".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-missing".to_string()),
                    ..RoutePoolEntry::default()
                },
            ),
        ]),
        ..ServiceState::default()
    };
    let result = repair_route_pool_service_state(
        &mut service_state,
        ServiceRoutePoolRepairOptions {
            apply: true,
            stale_checkouts: true,
            stale_pending_acquisitions: true,
        },
        "2026-05-28T12:00:00Z",
    );
    assert_eq!(result["repaired"], true);
    assert_eq!(result["repairedCounts"]["staleCheckouts"], 2);
    assert_eq!(result["repairedCounts"]["staleRoutes"], 1);
    assert_eq!(result["repairedCounts"]["staleDisplayAllocations"], 1);
    assert_eq!(service_state.route_pool["pool-missing"].state, "available");
    assert_eq!(
        service_state.route_pool["pool-missing"].current_route_allocation_id,
        None
    );
    assert_eq!(service_state.route_pool["pool-orphaned"].state, "available");
    assert_eq!(
        service_state.route_pool["pool-orphaned"].current_route_allocation_id,
        None
    );
    assert_eq!(
        service_state.remote_view_routes["route-orphaned"].state,
        "released"
    );
    assert_eq!(
        service_state.display_allocations["display-orphaned"].state,
        "released"
    );
    assert_eq!(
        service_state.route_pool["pool-missing"]
            .readiness
            .as_ref()
            .unwrap()["reason"],
        "stale_route_pool_checkout_repaired"
    );
    assert_eq!(service_state.route_pool["pool-active"].state, "checked_out");
    assert_eq!(
        service_state.route_pool["pool-active"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-active")
    );
}

#[test]
fn test_remote_view_open_persist_request_route_pool_preserves_active_checkout() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("remote-view-route-pool-preserve-active-home");
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
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!(
                        { "state" : "ready", "component" :
                        "route_bound_finalization" }
                    )),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let repository = LockedServiceStateRepository::new(store.clone());
    remote_view_open_persist_request_route_pool(
        &repository,
        &[
            RoutePoolEntry {
                id: "pool-a".to_string(),
                route_id: "route-a".to_string(),
                state: "available".to_string(),
                current_route_allocation_id: None,
                readiness: Some(json!({ "state" : "ready" })),
                ..RoutePoolEntry::default()
            },
            RoutePoolEntry {
                id: "pool-b".to_string(),
                route_id: "route-b".to_string(),
                state: "available".to_string(),
                current_route_allocation_id: None,
                readiness: Some(json!({ "state" : "ready" })),
                ..RoutePoolEntry::default()
            },
        ],
    )
    .unwrap();
    let persisted = store.load().unwrap();
    assert_eq!(persisted.route_pool["pool-a"].state, "checked_out");
    assert_eq!(
        persisted.route_pool["pool-a"]
            .current_route_allocation_id
            .as_deref(),
        Some("route-a")
    );
    assert_eq!(persisted.route_pool["pool-b"].state, "available");
    assert_eq!(
        persisted.route_pool["pool-b"].current_route_allocation_id,
        None
    );
    let _ = fs::remove_dir_all(&home);
}
