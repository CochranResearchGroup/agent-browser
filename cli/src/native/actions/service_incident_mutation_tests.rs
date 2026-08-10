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
use crate::native::service_incidents::*;
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
async fn test_service_incident_acknowledge_persists_metadata() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-incident-ack-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            events: vec![crate::native::service_model::ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: crate::native::service_model::ServiceEventKind::BrowserHealthChanged,
                message: "Browser browser-1 health changed from Ready to ProcessExited".to_string(),
                browser_id: Some("browser-1".to_string()),
                previous_health: Some(crate::native::service_model::BrowserHealth::Ready),
                current_health: Some(crate::native::service_model::BrowserHealth::ProcessExited),
                ..crate::native::service_model::ServiceEvent::default()
            }],
            browsers: std::collections::BTreeMap::from([(
                "browser-1".to_string(),
                crate::native::service_model::BrowserProcess {
                    id: "browser-1".to_string(),
                    health: crate::native::service_model::BrowserHealth::ProcessExited,
                    ..crate::native::service_model::BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_incident_acknowledge", "id" :
            "svc-incidents-ack-1", "incidentId" : "browser-1", "by" : "operator",
            "note" : "triaged" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_incident_acknowledge_response_contract(&result["data"]);
    assert_eq!(result["data"]["incident"]["acknowledgedBy"], "operator");
    assert_eq!(result["data"]["incident"]["acknowledgementNote"], "triaged");
    assert_eq!(
        result["data"]["incident"]["eventIds"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.incidents[0].acknowledged_by.as_deref(),
        Some("operator")
    );
    assert_eq!(
        persisted.incidents[0].acknowledgement_note.as_deref(),
        Some("triaged")
    );
    let event = persisted.events.last().unwrap();
    assert_eq!(
        event.kind,
        crate::native::service_model::ServiceEventKind::IncidentAcknowledged
    );
    assert_eq!(event.browser_id.as_deref(), Some("browser-1"));
    assert_eq!(event.details.as_ref().unwrap()["incidentId"], "browser-1");
    assert_eq!(event.details.as_ref().unwrap()["actor"], "operator");
    assert_eq!(event.details.as_ref().unwrap()["note"], "triaged");
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_acknowledge_service_incident_in_repository_persists_metadata() {
    let home = unique_socket_dir("service-incident-repository-home");
    fs::create_dir_all(&home).unwrap();
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    store
        .save(&ServiceState {
            events: vec![ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser browser-1 health changed from Ready to ProcessExited".to_string(),
                browser_id: Some("browser-1".to_string()),
                previous_health: Some(ServiceBrowserHealth::Ready),
                current_health: Some(ServiceBrowserHealth::ProcessExited),
                ..ServiceEvent::default()
            }],
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: ServiceBrowserHealth::ProcessExited,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let incident = crate::native::service_incidents::acknowledge_service_incident_in_repository(
        &repository,
        "browser-1",
        "2026-04-22T01:00:00Z",
        "operator",
        Some("triaged"),
    )
    .unwrap();
    assert_eq!(incident.id, "browser-1");
    assert_eq!(incident.acknowledged_by.as_deref(), Some("operator"));
    assert_eq!(incident.acknowledgement_note.as_deref(), Some("triaged"));
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.incidents[0].acknowledged_by.as_deref(),
        Some("operator")
    );
    assert_eq!(
        persisted.incidents[0].acknowledgement_note.as_deref(),
        Some("triaged")
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_incident_resolve_persists_metadata() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-incident-resolve-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            events: vec![crate::native::service_model::ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: crate::native::service_model::ServiceEventKind::BrowserHealthChanged,
                message: "Browser browser-1 health changed from Ready to ProcessExited".to_string(),
                browser_id: Some("browser-1".to_string()),
                previous_health: Some(crate::native::service_model::BrowserHealth::Ready),
                current_health: Some(crate::native::service_model::BrowserHealth::ProcessExited),
                ..crate::native::service_model::ServiceEvent::default()
            }],
            incidents: vec![crate::native::service_model::ServiceIncident {
                id: "browser-1".to_string(),
                acknowledged_at: Some("2026-04-22T00:00:00Z".to_string()),
                acknowledged_by: Some("operator".to_string()),
                ..crate::native::service_model::ServiceIncident::default()
            }],
            browsers: std::collections::BTreeMap::from([(
                "browser-1".to_string(),
                crate::native::service_model::BrowserProcess {
                    id: "browser-1".to_string(),
                    health: crate::native::service_model::BrowserHealth::ProcessExited,
                    ..crate::native::service_model::BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_incident_resolve", "id" :
            "svc-incidents-resolve-1", "incidentId" : "browser-1", "by" : "operator",
            "note" : "recovered" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_incident_resolve_response_contract(&result["data"]);
    assert_eq!(result["data"]["incident"]["resolvedBy"], "operator");
    assert_eq!(result["data"]["incident"]["resolutionNote"], "recovered");
    assert_eq!(result["data"]["incident"]["state"], "recovered");
    assert_eq!(
        result["data"]["incident"]["currentHealth"],
        serde_json::Value::Null
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.incidents[0].resolved_by.as_deref(),
        Some("operator")
    );
    assert_eq!(
        persisted.incidents[0].resolution_note.as_deref(),
        Some("recovered")
    );
    assert_eq!(
        persisted.incidents[0].state,
        crate::native::service_model::ServiceIncidentState::Recovered
    );
    assert_eq!(persisted.incidents[0].current_health, None);
    let event = persisted.events.last().unwrap();
    assert_eq!(
        event.kind,
        crate::native::service_model::ServiceEventKind::IncidentResolved
    );
    assert_eq!(event.browser_id.as_deref(), Some("browser-1"));
    assert_eq!(event.details.as_ref().unwrap()["incidentId"], "browser-1");
    assert_eq!(event.details.as_ref().unwrap()["actor"], "operator");
    assert_eq!(event.details.as_ref().unwrap()["note"], "recovered");
    let _ = fs::remove_dir_all(&home);
}
