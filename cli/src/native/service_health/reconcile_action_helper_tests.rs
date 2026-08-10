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

#[cfg(unix)]
#[test]
fn test_authoritative_route_pool_skips_conflicting_pending_entry_without_allocation_proof() {
    let mut state = ServiceState {
        route_pool: BTreeMap::from([(
            "guacamole-rdp-a".to_string(),
            RoutePoolEntry {
                id: "guacamole-rdp-a".to_string(),
                provider: ViewStreamProvider::RdpGateway,
                route_id: "guacamole:4".to_string(),
                state: "pending".to_string(),
                current_route_allocation_id: None,
                ..RoutePoolEntry::default()
            },
        )]),
        ..ServiceState::default()
    };
    let authoritative = json!(
        [{ "id" : "guacamole-rdp-a", "provider" : "rdp_gateway", "routeId" :
        "guacamole:1", "target" : { "displayName" : ":11" } }]
    );
    let result = refresh_authoritative_route_pool(&mut state, Some(&authoritative)).unwrap();
    assert_eq!(
        result["skippedActiveConflictEntryIds"][0],
        "guacamole-rdp-a"
    );
    assert_eq!(state.route_pool["guacamole-rdp-a"].route_id, "guacamole:4");
    assert_eq!(state.route_pool["guacamole-rdp-a"].state, "pending");
}
#[test]
fn test_reconciled_service_state_in_repository_preserves_current_fields() {
    let home = unique_socket_dir("service-reconcile-repository-home");
    fs::create_dir_all(&home).unwrap();
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    let before = ServiceState {
        browsers: BTreeMap::from([(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("work-before".to_string()),
                health: ServiceBrowserHealth::Ready,
                active_session_ids: vec!["session-1".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        ..ServiceState::default()
    };
    let mut persisted_current = before.clone();
    persisted_current
        .browsers
        .get_mut("browser-1")
        .unwrap()
        .profile_id = Some("work-current".to_string());
    store.save(&persisted_current).unwrap();
    let mut reconciled = before.clone();
    reconciled.browsers.insert(
        "browser-1".to_string(),
        BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("work-before".to_string()),
            health: ServiceBrowserHealth::Unreachable,
            last_error: Some("CDP endpoint is unreachable".to_string()),
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        },
    );
    persist_reconciled_service_state_in_repository(&repository, &before, &reconciled).unwrap();
    let persisted = store.load().unwrap();
    let browser = &persisted.browsers["browser-1"];
    assert_eq!(browser.profile_id.as_deref(), Some("work-current"));
    assert_eq!(browser.health, ServiceBrowserHealth::Unreachable);
    assert_eq!(
        browser.last_error.as_deref(),
        Some("CDP endpoint is unreachable")
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_stale_browser_process_record_preserves_identity_and_marks_error() {
    let previous = BrowserProcess {
        id: "browser-mcp-live".to_string(),
        profile_id: Some("profile-work".to_string()),
        host: ServiceBrowserHost::LocalHeaded,
        health: ServiceBrowserHealth::Ready,
        pid: Some(1234),
        cdp_endpoint: Some("ws://127.0.0.1:9222/devtools/browser/old".to_string()),
        view_streams: vec![ViewStream {
            id: "cdp-screencast".to_string(),
            provider: ViewStreamProvider::CdpScreencast,
            control_input: Some(ControlInputProvider::CdpInput),
            url: Some("http://127.0.0.1:44841/".to_string()),
            frame_url: Some("http://127.0.0.1:44841/".to_string()),
            external_url: Some("http://127.0.0.1:44841/".to_string()),
            read_only: false,
            ..ViewStream::default()
        }],
        active_session_ids: vec!["mcp-live".to_string()],
        ..BrowserProcess::default()
    };
    let stale = stale_browser_process_record(
        "browser-mcp-live",
        "mcp-live",
        Some(&previous),
        Some(1234),
        Some("ws://127.0.0.1:9222/devtools/browser/old".to_string()),
        ServiceBrowserHealth::ProcessExited,
        "Active browser PID 1234 exited before command dispatch".to_string(),
    );
    assert_eq!(stale.id, "browser-mcp-live");
    assert_eq!(stale.profile_id.as_deref(), Some("profile-work"));
    assert_eq!(stale.host, ServiceBrowserHost::LocalHeaded);
    assert_eq!(stale.health, ServiceBrowserHealth::ProcessExited);
    assert_eq!(stale.pid, Some(1234));
    assert_eq!(
        stale.cdp_endpoint.as_deref(),
        Some("ws://127.0.0.1:9222/devtools/browser/old")
    );
    assert_eq!(stale.active_session_ids, vec!["mcp-live".to_string()]);
    assert_eq!(
        stale.last_error.as_deref(),
        Some("Active browser PID 1234 exited before command dispatch")
    );
    assert_eq!(stale.view_streams.len(), 1);
    assert_eq!(stale.view_streams[0].control_input, None);
    assert_eq!(stale.view_streams[0].url, None);
    assert!(stale.view_streams[0].read_only);
    assert_eq!(
        stale.view_streams[0].readiness.as_ref().unwrap()["reason"],
        "browser_not_ready"
    );
}
