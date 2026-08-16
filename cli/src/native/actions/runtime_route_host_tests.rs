#![allow(unused_imports)]
use super::*;
use crate::native::action_runtime::cancellation::{cancellable, cancellation_error};
use crate::native::action_runtime::runtime::*;
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
use crate::native::service_access::*;
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
use crate::native::service_resources::handle_service_access_plan;
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

#[test]
fn test_service_profile_lease_gate_allows_service_control_commands() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("profile-lease-service-actions-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            profiles: BTreeMap::from([(
                "acs-profile".to_string(),
                BrowserProfile {
                    id: "acs-profile".to_string(),
                    shared_service_ids: vec!["JournalDownloader".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "active-session".to_string(),
                BrowserSession {
                    id: "active-session".to_string(),
                    profile_id: Some("acs-profile".to_string()),
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    for action in [
        "runtime_handoff_prepare",
        "runtime_handoff_resume",
        "runtime_handoff_abort",
        "runtime_handoff_rollback",
        "runtime_handoff_finalize",
        "service_status",
        "service_reconcile",
        "service_job_cancel",
        "service_browser_retry",
        "service_profile_upsert",
        "service_profile_freshness_update",
        "service_profile_delete",
        "service_session_upsert",
        "service_session_delete",
        "service_site_policy_upsert",
        "service_site_policy_delete",
        "service_monitor_upsert",
        "service_monitor_delete",
        "service_monitors_run_due",
        "service_provider_upsert",
        "service_provider_delete",
        "service_browser_capability_registry_upsert",
        "service_access_plan",
        "service_browser_capability_preflight",
        "service_incident_acknowledge",
        "service_incident_resolve",
        "service_incident_activity",
        "service_trace",
        "service_profiles",
        "service_profile_seeding_handoff",
        "service_sessions",
        "service_browsers",
        "service_tabs",
        "service_monitors",
        "service_site_policies",
        "service_providers",
        "service_challenges",
        "service_jobs",
        "service_incidents",
        "service_events",
    ] {
        assert!(
            action_skips_browser_launch(action),
            "{action} must remain no-launch"
        );
        let decision = service_profile_lease_gate(
            &json!(
                { "action" : action, "serviceName" : "JournalDownloader",
                "targetServiceId" : "acs", "profileLeasePolicy" : "reject" }
            ),
            "new-session",
            Some(0),
        )
        .expect("lease gate should evaluate");
        assert!(
            matches!(decision, ServiceProfileLeaseGate::Ready),
            "{action} should not be blocked by profile lease gates"
        );
    }
    let browser_decision = service_profile_lease_gate(
        &json!(
            { "action" : "tab_list", "serviceName" : "JournalDownloader",
            "targetServiceId" : "acs", "profileLeasePolicy" : "reject" }
        ),
        "new-session",
        Some(0),
    )
    .expect("lease gate should evaluate");
    assert!(matches!(
        browser_decision,
        ServiceProfileLeaseGate::Reject { .. }
    ));
    let _ = fs::remove_dir_all(&home);
}
