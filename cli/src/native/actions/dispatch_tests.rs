#![allow(unused_imports)]
use super::*;
use crate::native::action_runtime::cancellation::{cancellable, cancellation_error};
use crate::native::action_runtime::runtime::{launch_options_from_env, HarEntry, MouseState};
use crate::native::action_runtime::DaemonState;
use crate::native::auth;
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::browser_input::build_mouse_event_params;
use crate::native::cancellation::CancellationToken;
use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
use crate::native::cookies;
use crate::native::network::{self, DomainFilter, EventTracker};
use crate::native::network_archive::{
    browser_metadata_from_version, get_har_dir, har_cdp_protocol_to_http_version,
    har_compute_timings, har_entry_to_json, har_parse_request_cookies, har_wall_time_to_rfc3339,
    unix_timestamp_millis,
};
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
#[test]
fn test_success_response_structure() {
    let resp = success_response("cmd-1", json!({ "url" : "https://example.com" }));
    assert_eq!(resp["id"], "cmd-1");
    assert_eq!(resp["success"], true);
    assert!(resp["data"].is_object());
    assert_eq!(resp["data"]["url"], "https://example.com");
}
#[test]
fn test_take_response_warning_removes_internal_warning_field() {
    let mut data = json!(
        { "url" : "https://accounts.google.com/", "_warning" : "manual login preferred" }
    );
    assert_eq!(
        take_response_warning(&mut data),
        Some("manual login preferred".to_string())
    );
    assert!(data.get("_warning").is_none());
    assert_eq!(data["url"], "https://accounts.google.com/");
}
#[test]
fn test_error_response_structure() {
    let resp = error_response("cmd-2", "Something went wrong");
    assert_eq!(resp["id"], "cmd-2");
    assert_eq!(resp["success"], false);
    assert_eq!(resp["error"], "Something went wrong");
}

#[tokio::test]
async fn test_execute_har_stop_skips_browser_auto_launch() {
    let path = std::env::temp_dir().join(format!(
        "agent-browser-har-stop-{}.har",
        unix_timestamp_millis()
    ));
    let mut state = DaemonState::new();
    state.har_entries.push(HarEntry {
        request_id: "req-3".to_string(),
        wall_time: 1773576000.0,
        method: "GET".to_string(),
        url: "https://example.com/".to_string(),
        request_headers: vec![],
        post_data: None,
        request_body_size: 0,
        resource_type: "Document".to_string(),
        status: Some(200),
        status_text: "OK".to_string(),
        http_version: "HTTP/1.1".to_string(),
        response_headers: vec![],
        mime_type: "text/html".to_string(),
        redirect_url: String::new(),
        response_body_size: 64,
        cdp_timing: None,
        loading_finished_timestamp: None,
    });
    let result = execute_command(
        &json!(
            { "action" : "har_stop", "id" : "har-stop-1", "path" : path
            .to_string_lossy().to_string() }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true, "{}", result);
    assert_eq!(result["data"]["requestCount"], 1);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn test_execute_unknown_command() {
    let mut state = DaemonState::new();
    let cmd = json!({ "action" : "unknown_action_xyz", "id" : "test-1" });
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], false);
    let error_msg = result["error"].as_str().unwrap();
    assert!(
        error_msg.contains("Not yet implemented") || error_msg.contains("Auto-launch failed"),
        "Unexpected error: {}",
        error_msg
    );
}
#[tokio::test]
async fn test_execute_empty_action() {
    let mut state = DaemonState::new();
    let cmd = json!({ "id" : "test-2" });
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], false);
}
#[tokio::test]
async fn test_execute_close_without_browser() {
    let mut state = DaemonState::new();
    let cmd = json!({ "action" : "close", "id" : "test-3" });
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true, "{}", result);
    assert_eq!(result["data"]["closed"], true);
}
#[tokio::test]
async fn test_navigate_without_browser() {
    let mut state = DaemonState::new();
    {
        let mut df = state.domain_filter.write().await;
        *df = Some(DomainFilter::new("example.com"));
    }
    let cmd = json!(
        { "action" : "navigate", "url" : "https://blocked.com", "id" : "test-4" }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], false);
}
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_credentials_roundtrip_via_actions() {
    let env_guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_ENCRYPTION_KEY"]);
    let _lock = crate::native::auth::AUTH_TEST_MUTEX.lock().unwrap();
    let home = std::env::temp_dir().join(format!(
        "agent-browser-credentials-action-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    env_guard.set("HOME", home.to_str().unwrap());
    env_guard.set("AGENT_BROWSER_ENCRYPTION_KEY", &"a".repeat(64));
    let mut state = DaemonState::new();
    let set_cmd = json!(
        { "action" : "credentials_set", "name" : "test-cred-action", "username" : "user",
        "password" : "pass", "id" : "c1" }
    );
    let result = execute_command(&set_cmd, &mut state).await;
    assert_eq!(result["success"], true);
    let get_cmd = json!(
        { "action" : "credentials_get", "name" : "test-cred-action", "id" : "c2" }
    );
    let result = execute_command(&get_cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["username"], "user");
    let list_cmd = json!({ "action" : "credentials_list", "id" : "c3" });
    let result = execute_command(&list_cmd, &mut state).await;
    assert_eq!(result["success"], true);
    let del_cmd = json!(
        { "action" : "credentials_delete", "name" : "test-cred-action", "id" : "c4" }
    );
    let result = execute_command(&del_cmd, &mut state).await;
    assert_eq!(result["success"], true);
}
