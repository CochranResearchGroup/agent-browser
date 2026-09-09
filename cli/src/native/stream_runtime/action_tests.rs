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
fn route_pool_error_diagnostic(result: &Value) -> Value {
    let error = result["error"].as_str().unwrap();
    let diagnostic = error
        .split_once("diagnostic=")
        .map(|(_, diagnostic)| diagnostic)
        .expect("route pool error should include diagnostic JSON");
    serde_json::from_str(diagnostic).expect("route pool diagnostic should be valid JSON")
}

#[tokio::test]
async fn test_stream_enable_disable_and_status_without_browser() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_SOCKET_DIR", "AGENT_BROWSER_SESSION"]);
    let socket_dir = unique_socket_dir("stream-runtime");
    fs::create_dir_all(&socket_dir).expect("socket dir should be created");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        socket_dir.to_str().expect("socket dir should be utf-8"),
    );
    guard.set("AGENT_BROWSER_SESSION", "stream-runtime-session");
    let mut state = DaemonState::new();
    let disabled_status = handle_stream_status(&state)
        .await
        .expect("status should work before enable");
    assert_eq!(disabled_status["enabled"], false);
    assert_eq!(disabled_status["port"], Value::Null);
    assert_eq!(disabled_status["connected"], false);
    assert_eq!(disabled_status["screencasting"], false);
    let enabled_status = handle_stream_enable(&json!({ "port" : 0 }), &mut state)
        .await
        .expect("stream enable should succeed");
    let port = enabled_status["port"]
        .as_u64()
        .expect("runtime stream should report a bound port");
    assert!(port > 0, "runtime stream should bind a non-zero port");
    assert_eq!(enabled_status["enabled"], true);
    assert_eq!(enabled_status["connected"], false);
    assert_eq!(enabled_status["screencasting"], false);
    let stream_path = socket_dir.join("stream-runtime-session.stream");
    let port_file = fs::read_to_string(&stream_path).expect("stream metadata file should exist");
    assert_eq!(port_file.trim(), port.to_string());
    let original_server = state.stream_server.clone().unwrap();
    for command in [json!({}), json!({ "port": 0 }), json!({ "port": port })] {
        let repeated = handle_stream_enable(&command, &mut state)
            .await
            .expect("ensuring the existing stream should succeed");
        assert_eq!(repeated, enabled_status);
        assert!(std::sync::Arc::ptr_eq(
            &original_server,
            state.stream_server.as_ref().unwrap()
        ));
        assert_eq!(fs::read_to_string(&stream_path).unwrap(), port_file);
    }
    let other_port = if port == 65535 { 65534 } else { port + 1 };
    let conflict = handle_stream_enable(&json!({ "port": other_port }), &mut state)
        .await
        .expect_err("a different explicit port must not silently reuse the listener");
    assert!(conflict.contains("already enabled"));
    assert!(handle_stream_enable(&json!({ "port": 65536 }), &mut state)
        .await
        .expect_err("invalid ports must still fail")
        .contains("Invalid stream port"));
    assert!(std::sync::Arc::ptr_eq(
        &original_server,
        state.stream_server.as_ref().unwrap()
    ));
    for invalid in [json!(-1), json!(1.5), json!("9223"), Value::Null] {
        assert!(
            handle_stream_enable(&json!({ "port": invalid }), &mut state)
                .await
                .expect_err("malformed explicit ports must not become implicit reuse")
                .contains("Invalid stream port")
        );
    }
    let status = handle_stream_status(&state)
        .await
        .expect("status should work after enable");
    assert_eq!(status["enabled"], true);
    assert_eq!(status["port"], port);
    let disabled = handle_stream_disable(&mut state)
        .await
        .expect("stream disable should succeed");
    assert_eq!(disabled["disabled"], true);
    assert!(
        !stream_path.exists(),
        "disabling runtime stream should remove the metadata file"
    );
    assert!(state.stream_server.is_none());
    assert!(state.stream_client.is_none());
    let final_status = handle_stream_status(&state)
        .await
        .expect("status should work after disable");
    assert_eq!(final_status["enabled"], false);
    assert_eq!(final_status["port"], Value::Null);
    let disable_err = handle_stream_disable(&mut state)
        .await
        .expect_err("duplicate disable should fail");
    assert!(disable_err.contains("not enabled"));
    let _ = fs::remove_dir_all(&socket_dir);
}
#[tokio::test]
async fn test_stream_disable_preserves_existing_screencast_state() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_SOCKET_DIR", "AGENT_BROWSER_SESSION"]);
    let socket_dir = unique_socket_dir("stream-preserve-screencast");
    fs::create_dir_all(&socket_dir).expect("socket dir should be created");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        socket_dir.to_str().expect("socket dir should be utf-8"),
    );
    guard.set(
        "AGENT_BROWSER_SESSION",
        "stream-preserve-screencast-session",
    );
    let mut state = DaemonState::new();
    handle_stream_enable(&json!({ "port" : 0 }), &mut state)
        .await
        .expect("stream enable should succeed");
    state.screencasting = true;
    let disabled = handle_stream_disable(&mut state)
        .await
        .expect("stream disable should succeed");
    assert_eq!(disabled["disabled"], true);
    assert!(
        state.screencasting,
        "stream disable should not clear an independently managed screencast state"
    );
    let _ = fs::remove_dir_all(&socket_dir);
}
#[tokio::test]
async fn test_stream_disable_clears_state_when_stream_file_removal_fails() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_SOCKET_DIR", "AGENT_BROWSER_SESSION"]);
    let socket_dir = unique_socket_dir("stream-disable-cleanup");
    fs::create_dir_all(&socket_dir).expect("socket dir should be created");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        socket_dir.to_str().expect("socket dir should be utf-8"),
    );
    guard.set("AGENT_BROWSER_SESSION", "stream-disable-cleanup-session");
    let mut state = DaemonState::new();
    handle_stream_enable(&json!({ "port" : 0 }), &mut state)
        .await
        .expect("stream enable should succeed");
    let stream_path = socket_dir.join("stream-disable-cleanup-session.stream");
    fs::remove_file(&stream_path).expect("stream metadata file should exist");
    fs::create_dir(&stream_path).expect("directory should force remove_stream_file failure");
    let err = handle_stream_disable(&mut state)
        .await
        .expect_err("stream disable should surface file removal failure");
    assert!(err.contains("Failed to remove stream metadata"));
    assert!(
        state.stream_server.is_none(),
        "stream disable should clear stream_server even when metadata cleanup fails"
    );
    assert!(
        state.stream_client.is_none(),
        "stream disable should clear stream_client even when metadata cleanup fails"
    );
    let _ = fs::remove_dir_all(&socket_dir);
}
#[tokio::test]
async fn test_stream_enable_port_conflict_returns_error() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_SOCKET_DIR", "AGENT_BROWSER_SESSION"]);
    let socket_dir = unique_socket_dir("stream-port-conflict");
    fs::create_dir_all(&socket_dir).expect("socket dir should be created");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        socket_dir.to_str().expect("socket dir should be utf-8"),
    );
    guard.set("AGENT_BROWSER_SESSION", "stream-port-conflict-session");
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("test should reserve an ephemeral port");
    let port = listener
        .local_addr()
        .expect("listener should have local addr")
        .port();
    let mut state = DaemonState::new();
    let err = handle_stream_enable(&json!({ "port" : port }), &mut state)
        .await
        .expect_err("conflicting port should fail");
    assert!(err.contains("Failed to bind stream server"));
    assert!(state.stream_server.is_none());
    assert!(state.stream_client.is_none());
    assert!(
        !socket_dir
            .join("stream-port-conflict-session.stream")
            .exists(),
        "failed enable should not leave stale metadata behind"
    );
    drop(listener);
    let _ = fs::remove_dir_all(&socket_dir);
}
