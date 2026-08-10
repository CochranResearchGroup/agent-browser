#![allow(unused_imports)]
use super::*;
use crate::native::action_runtime::cancellation::{cancellable, cancellation_error};
use crate::native::actions::*;
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
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStream,
    ViewStreamProvider, ViewerLease,
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
use std::collections::BTreeMap;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
#[test]
fn evaluate_uses_per_command_worker_deadline() {
    let command = json!(
        { "action" : "evaluate", "script" : "document.body.textContent", "jobTimeoutMs" :
        3_000, }
    );
    assert_eq!(command_evaluation_timeout_ms(&command), Some(3_000));
    assert_eq!(
        command_evaluation_timeout_ms(&json!({ "action" : "evaluate" })),
        None
    );
}
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
async fn dependent_batch_executes_ordered_steps_under_one_command() {
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "dependent-1", "action" : "dependent_batch", "bail" : true,
            "commands" : [{ "id" : "step-1", "action" : "__test_sleep", "ms" : 2 }, {
            "id" : "step-2", "action" : "__test_sleep", "ms" : 1 }] }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["completed"], 2);
    assert_eq!(result["data"]["hadError"], false);
    assert_eq!(result["data"]["results"][0]["result"]["sleptMs"], 2);
    assert_eq!(result["data"]["results"][1]["result"]["sleptMs"], 1);
    assert!(result["data"]["results"][0]["timings"]["actionExecutionMs"].is_u64());
}
#[tokio::test]
async fn dependent_batch_bails_before_a_step_after_rejected_lifecycle_action() {
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "dependent-bail-1", "action" : "dependent_batch", "bail" : true,
            "commands" : [{ "id" : "step-1", "action" : "close" }, { "id" : "step-2",
            "action" : "__test_sleep", "ms" : 1 }] }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["completed"], 1);
    assert_eq!(result["data"]["hadError"], true);
    assert_eq!(result["data"]["results"][0]["action"], "close");
    assert!(result["data"]["results"][0]["error"]
        .as_str()
        .unwrap()
        .contains("cannot run"));
}
#[test]
fn test_remote_view_visible_window_proof_retryable_states_are_transient_only() {
    assert!(remote_view_visible_window_proof_retryable_state(
        "display_probe_unavailable"
    ));
    assert!(remote_view_visible_window_proof_retryable_state(
        "empty_display"
    ));
    assert!(remote_view_visible_window_proof_retryable_state(
        "non_browser_windows"
    ));
    assert!(!remote_view_visible_window_proof_retryable_state(
        "terminal_only"
    ));
    assert!(!remote_view_visible_window_proof_retryable_state(
        "terminal_topmost"
    ));
    assert!(!remote_view_visible_window_proof_retryable_state(
        "browser_window_visible"
    ));
}
#[test]
fn test_route_bound_handoff_operator_visible_reports_ready_proof() {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("https://dashboard.example/guac/#/client/route-a".to_string()),
        external_url: Some("https://guac.example/#/client/route-a".to_string()),
        route_descriptor: None,
        readiness: None,
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com"),
    );
    assert_eq!(operator_visible["state"], "ready");
    assert_eq!(operator_visible["browserId"], "session:rdp-a");
    assert_eq!(operator_visible["sessionName"], "rdp-a");
    assert_eq!(operator_visible["routeId"], "route-a");
    assert_eq!(operator_visible["routePoolEntryId"], "pool-a");
    assert_eq!(operator_visible["displayAllocationId"], "display-a");
    assert_eq!(operator_visible["displayName"], ":11");
    assert_eq!(
        operator_visible["proof"]["displayContent"]["state"],
        "browser_window_visible"
    );
    assert_eq!(operator_visible["target"]["state"], "ready");
    assert_eq!(operator_visible["target"]["targetId"], "target-1");
    assert_eq!(
        operator_visible["target"]["expectedUrl"],
        "https://www.facebook.com"
    );
    assert_eq!(operator_visible["target"]["urlReadiness"], "ready");
    assert_eq!(
        operator_visible["target"]["url"],
        "https://www.facebook.com/"
    );
    assert_eq!(
        operator_visible["target"]["profileId"],
        "last30days-facebook"
    );
    assert_eq!(operator_visible["components"]["route"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["display"]["contentState"],
        "browser_window_visible"
    );
    assert_eq!(operator_visible["components"]["tab"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["tab"]["urlReadiness"],
        "ready"
    );
    assert_eq!(
        operator_visible["components"]["browser"]["profileId"],
        "last30days-facebook"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["state"],
        "ready"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["hasRouteUrl"],
        true
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["state"],
        "ready"
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["required"],
        false
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_requires_public_operator_access_check() {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("http://127.0.0.1:8092/guacamole/#/client/route-a".to_string()),
        external_url: Some("https://agent-browser.example/guacamole/#/client/route-a".to_string()),
        route_descriptor: Some(json!(
            { "localEmbedUrl" : "http://127.0.0.1:8092/guacamole/#/client/route-a",
            "dashboardEmbedUrl" :
            "https://dashboard.example/guacamole/#/client/route-a",
            "publicOperatorUrl" :
            "https://agent-browser.example/guacamole/#/client/route-a" }
        )),
        readiness: None,
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "public_operator_not_checked");
    assert_eq!(operator_visible["proof"]["state"], "ready");
    assert_eq!(operator_visible["target"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["guacamole"]["state"],
        "ready"
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["state"],
        "public_operator_not_checked"
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["required"],
        true
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["publicOperatorUrl"],
        "https://agent-browser.example/guacamole/#/client/route-a"
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_blocks_failed_public_operator_access() {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("http://127.0.0.1:8092/guacamole/#/client/route-a".to_string()),
        external_url: Some("https://agent-browser.example/guacamole/#/client/route-a".to_string()),
        route_descriptor: Some(json!(
            { "localEmbedUrl" : "http://127.0.0.1:8092/guacamole/#/client/route-a",
            "dashboardEmbedUrl" :
            "https://dashboard.example/guacamole/#/client/route-a",
            "publicOperatorUrl" :
            "https://agent-browser.example/guacamole/#/client/route-a" }
        )),
        readiness: Some(json!(
            { "state" : "ready", "operatorAccess" : { "state" : "proxy_failed",
            "httpStatus" : 502, "reason" : "public ingress returned 502" } }
        )),
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "public_operator_unavailable");
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["state"],
        "public_operator_unavailable"
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["readinessState"],
        "proxy_failed"
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["httpStatus"],
        502
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_blocks_malformed_guacamole_client_token() {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("http://127.0.0.1:8092/guacamole/#/client/NABjAHBvc3RncmVzcW".to_string()),
        external_url: None,
        route_descriptor: None,
        readiness: Some(json!(
            { "state" : "ready", "operatorAccess" : { "state" : "ready", "httpStatus"
            : 200 } }
        )),
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "invalid_operator_route");
    assert_eq!(
        operator_visible["components"]["route"]["state"],
        "invalid_operator_route"
    );
    assert_eq!(
        operator_visible["components"]["operatorAccess"]["state"],
        "ready"
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_ignores_stale_acquisition_pending_readiness() {
    let route_binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "guacamole:4".to_string(),
        route_pool_entry_id: Some("guacamole-rdp-b".to_string()),
        display_allocation_id: "remote-view-display:14".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":14".to_string()),
        launch_display_name: Some(":14".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-b".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "simultaneous_view".to_string(),
        connection_id: Some("4".to_string()),
        connection_name: Some("Agent Browser RDP Route B".to_string()),
        frame_url: Some(
            "http://127.0.0.1:8092/guacamole/#/client/NABjAHBvc3RncmVzcWw=".to_string(),
        ),
        external_url: Some(
            "https://agent-browser.example/guacamole/#/client/NABjAHBvc3RncmVzcWw=".to_string(),
        ),
        route_descriptor: None,
        readiness: Some(json!(
            { "state" : "pending", "component" : "remote_view_open_acquisition",
            "leaseId" : "remote-view-open:default:guacamole-4:stale" }
        )),
    };
    let acquisition_plan = RemoteViewAcquisitionPlan {
        mode: "strict_operator_open".to_string(),
        reuse_policy: "test".to_string(),
        tab_policy: "open_new".to_string(),
        requested_profile: None,
        requested_browser_build: None,
        requested_browser_host: "remote_headed".to_string(),
        requested_view_stream_provider: ViewStreamProvider::RdpGateway,
        requested_control_input: "manual_attached_desktop".to_string(),
        requested_display_isolation: Some("shared_display".to_string()),
        requested_route_pool_entry_id: Some("guacamole-rdp-b".to_string()),
        requested_route_id: Some("guacamole:4".to_string()),
        selected_route_pool_entry_id: Some("guacamole-rdp-b".to_string()),
        selected_route_id: "guacamole:4".to_string(),
        display_allocation_id: "remote-view-display:14".to_string(),
        display_name: Some(":14".to_string()),
        route_binding,
        decisions: Vec::new(),
        blockers: Vec::new(),
        proof_required: Vec::new(),
        cleanup_on_failure: Vec::new(),
        suggested_commands: Vec::new(),
    };
    let handoff_plan =
        route_bound_handoff_plan(&json!({}), &acquisition_plan, "session:rdp", "rdp");
    let binding = handoff_plan.route_binding;
    let proof = json!(
        { "state" : "ready", "displayName" : ":14", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-facebook", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:default",
        "default",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "ready");
    assert_eq!(
        operator_visible["components"]["route"]["readinessState"],
        "ready"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["readinessState"],
        "ready"
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_distinguishes_wrong_tab_from_visible_browser() {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("https://dashboard.example/guac/#/client/route-a".to_string()),
        external_url: Some("https://guac.example/#/client/route-a".to_string()),
        route_descriptor: None,
        readiness: None,
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.linkedin.com/", "title" :
        "LinkedIn", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "wrong_tab");
    assert_eq!(operator_visible["proof"]["state"], "ready");
    assert_eq!(
        operator_visible["proof"]["displayContent"]["state"],
        "browser_window_visible"
    );
    assert_eq!(operator_visible["target"]["state"], "wrong_tab");
    assert_eq!(operator_visible["target"]["urlReadiness"], "wrong_tab");
    assert_eq!(
        operator_visible["target"]["expectedUrl"],
        "https://www.facebook.com/"
    );
    assert_eq!(operator_visible["components"]["display"]["state"], "ready");
    assert_eq!(operator_visible["components"]["tab"]["state"], "wrong_tab");
    assert_eq!(
        operator_visible["components"]["guacamole"]["state"],
        "ready"
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_distinguishes_guacamole_unavailable_from_visible_browser(
) {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: None,
        external_url: None,
        route_descriptor: None,
        readiness: Some(json!({ "state" : "failed", "reason" : "local_embed_not_ready" })),
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "guacamole_route_unavailable");
    assert_eq!(operator_visible["proof"]["state"], "ready");
    assert_eq!(operator_visible["target"]["state"], "ready");
    assert_eq!(operator_visible["components"]["display"]["state"], "ready");
    assert_eq!(operator_visible["components"]["tab"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["guacamole"]["state"],
        "guacamole_route_unavailable"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["readinessState"],
        "failed"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["reason"],
        "local_embed_not_ready"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["hasRouteUrl"],
        false
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_distinguishes_cdp_target_unavailable_from_visible_browser(
) {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("https://dashboard.example/guac/#/client/route-a".to_string()),
        external_url: Some("https://guac.example/#/client/route-a".to_string()),
        route_descriptor: None,
        readiness: None,
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "url" : "https://www.facebook.com/", "title" : "Facebook", "profileId" :
        "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "cdp_target_unavailable");
    assert_eq!(operator_visible["proof"]["state"], "ready");
    assert_eq!(
        operator_visible["target"]["state"],
        "cdp_target_unavailable"
    );
    assert_eq!(operator_visible["target"]["targetId"], Value::Null);
    assert_eq!(operator_visible["target"]["urlReadiness"], "ready");
    assert_eq!(operator_visible["components"]["display"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["tab"]["state"],
        "cdp_target_unavailable"
    );
    assert_eq!(
        operator_visible["components"]["guacamole"]["state"],
        "ready"
    );
}
#[test]
fn test_route_bound_handoff_operator_visible_distinguishes_stale_route_record_from_visible_browser()
{
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("checked_out".to_string()),
        current_route_allocation_id: Some("route-missing".to_string()),
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("https://dashboard.example/guac/#/client/route-a".to_string()),
        external_url: Some("https://guac.example/#/client/route-a".to_string()),
        route_descriptor: None,
        readiness: Some(json!({ "state" : "ready", "reason" : "retained_checkout_metadata" })),
    };
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "last30days-facebook" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "stale_route_record");
    assert_eq!(operator_visible["proof"]["state"], "ready");
    assert_eq!(operator_visible["target"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["route"]["state"],
        "stale_route_record"
    );
    assert_eq!(
        operator_visible["components"]["route"]["routePoolEntryState"],
        "checked_out"
    );
    assert_eq!(
        operator_visible["components"]["route"]["currentRouteAllocationId"],
        "route-missing"
    );
    assert_eq!(operator_visible["components"]["display"]["state"], "ready");
    assert_eq!(operator_visible["components"]["tab"]["state"], "ready");
    assert_eq!(
        operator_visible["components"]["guacamole"]["state"],
        "ready"
    );
}
#[test]
fn test_remote_view_open_final_route_binding_preserves_post_checkout_stale_route() {
    let binding = crate::native::remote_view::RemoteViewRouteBinding {
        route_id: "route-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: None,
        display_name: Some(":11".to_string()),
        launch_display_name: Some(":11".to_string()),
        display_isolation: "shared_display".to_string(),
        route_user: Some("agent-browser-rdp-a".to_string()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some("conn-a".to_string()),
        connection_name: Some("Route A".to_string()),
        frame_url: Some("https://dashboard.example/guac/#/client/route-a".to_string()),
        external_url: Some("https://guac.example/#/client/route-a".to_string()),
        route_descriptor: None,
        readiness: Some(json!({ "state" : "ready", "reason" : "pre_checkout_ready" })),
    };
    let checkout = json!(
        { "routeBinding" : binding, "remoteViewRoute" : { "id" : "route-a",
        "displayAllocationId" : "display-a", "frameUrl" :
        "https://dashboard.example/guac/#/client/route-a", "externalUrl" :
        "https://guac.example/#/client/route-a", "readiness" : { "state" : "ready",
        "component" : "remote_view_open_visible_window" } }, "routePoolEntry" : { "id" :
        "pool-a", "state" : "checked_out", "currentRouteAllocationId" : "route-old",
        "readiness" : { "state" : "ready", "component" :
        "remote_view_open_visible_window" } } }
    );
    let final_binding = crate::native::remote_view_handoff::final_route_bound_handoff_route_binding(
        &binding, &checkout,
    );
    let proof = json!(
        { "state" : "ready", "displayName" : ":11", "displayContent" : { "state" :
        "browser_window_visible" } }
    );
    let tab = json!(
        { "targetId" : "target-1", "url" : "https://www.facebook.com/", "title" :
        "Facebook", "profileId" : "managed-one-time" }
    );
    let operator_visible = route_bound_handoff_operator_visible(
        &final_binding,
        "session:rdp-a",
        "rdp-a",
        Some(&proof),
        Some(&tab),
        Some("https://www.facebook.com/"),
    );
    assert_eq!(operator_visible["state"], "stale_route_record");
    assert_eq!(
        operator_visible["components"]["route"]["currentRouteAllocationId"],
        "route-old"
    );
}
#[test]
fn test_remote_view_open_warns_on_arbitrary_one_time_runtime_profile() {
    let intent = crate::native::remote_view::RemoteViewOpenIntent {
        url: Some("https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string()),
        runtime_profile: Some("tx-sos-temp-stock-b".to_string()),
        profile: None,
        browser_id: None,
        session_name: None,
        service_name: Some("sosdirect".to_string()),
        agent_name: Some("codex".to_string()),
        task_name: Some("temporary-login-payment".to_string()),
        browser_build: Some("stock_chrome".to_string()),
        browser_host: "remote_headed".to_string(),
        view_stream_provider: ViewStreamProvider::RdpGateway,
        control_input: "manual_attached_desktop".to_string(),
        route_pool_entry_id: None,
        route_id: None,
        display_allocation_id: None,
        remote_headed_display: None,
        display_isolation: Some("private_virtual_display".to_string()),
        manual_login_launch: false,
        dry_run: false,
    };
    let warning = remote_view_open_one_time_profile_warning(&intent, &ServiceState::default());
    assert_eq!(warning["state"], "warning");
    assert_eq!(
        warning["code"],
        "arbitrary_runtime_profile_for_one_time_handoff"
    );
    assert_eq!(warning["requestedRuntimeProfile"], "tx-sos-temp-stock-b");
    assert_eq!(warning["profileClass"], "operator_supplied");
    assert_eq!(warning["recommendedProfileClass"], "managed_one_time");
    assert!(warning["recommendedProfileId"]
        .as_str()
        .unwrap()
        .starts_with("managed-one-time-"));
}
#[test]
fn test_remote_view_open_does_not_warn_on_known_profile() {
    let intent = crate::native::remote_view::RemoteViewOpenIntent {
        url: Some("https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string()),
        runtime_profile: Some("sosdirect".to_string()),
        profile: None,
        browser_id: None,
        session_name: None,
        service_name: Some("sosdirect".to_string()),
        agent_name: Some("codex".to_string()),
        task_name: Some("temporary-login-payment".to_string()),
        browser_build: Some("stock_chrome".to_string()),
        browser_host: "remote_headed".to_string(),
        view_stream_provider: ViewStreamProvider::RdpGateway,
        control_input: "manual_attached_desktop".to_string(),
        route_pool_entry_id: None,
        route_id: None,
        display_allocation_id: None,
        remote_headed_display: None,
        display_isolation: Some("private_virtual_display".to_string()),
        manual_login_launch: false,
        dry_run: false,
    };
    let service_state = ServiceState {
        profiles: BTreeMap::from([(
            "sosdirect".to_string(),
            BrowserProfile {
                id: "sosdirect".to_string(),
                name: "SOSDirect".to_string(),
                ..BrowserProfile::default()
            },
        )]),
        ..ServiceState::default()
    };
    let warning = remote_view_open_one_time_profile_warning(&intent, &service_state);
    assert!(warning.is_null());
}
#[test]
fn test_remote_view_open_plans_managed_one_time_profile_without_arbitrary_profile() {
    let repository = LockedServiceStateRepository::default_json().expect("repository");
    let mut service_state = ServiceState::default();
    let mut intent = crate::native::remote_view::RemoteViewOpenIntent {
        url: Some("https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string()),
        runtime_profile: None,
        profile: None,
        browser_id: None,
        session_name: None,
        service_name: Some("sosdirect".to_string()),
        agent_name: Some("codex".to_string()),
        task_name: Some("temporary-login-payment".to_string()),
        browser_build: Some("stock_chrome".to_string()),
        browser_host: "remote_headed".to_string(),
        view_stream_provider: ViewStreamProvider::RdpGateway,
        control_input: "manual_attached_desktop".to_string(),
        route_pool_entry_id: None,
        route_id: None,
        display_allocation_id: None,
        remote_headed_display: None,
        display_isolation: Some("private_virtual_display".to_string()),
        manual_login_launch: false,
        dry_run: true,
    };
    let managed = remote_view_open_ensure_managed_one_time_profile(
        &repository,
        &mut service_state,
        &mut intent,
        true,
    )
    .expect("managed one-time profile");
    let profile_id = managed["profileId"].as_str().expect("profile id");
    let effective_cmd = remote_view_open_command_with_effective_intent(
        &json!({ "action" : "remote_view_open", "url" : intent.url, }),
        &intent,
    );
    let warning = remote_view_open_one_time_profile_warning(&intent, &service_state);
    assert_eq!(managed["state"], "planned");
    assert_eq!(managed["profileClass"], "managed_one_time");
    assert!(profile_id.starts_with("managed-one-time-"));
    assert_eq!(intent.runtime_profile.as_deref(), Some(profile_id));
    assert_eq!(effective_cmd["runtimeProfile"], profile_id);
    assert!(service_state.profiles.contains_key(profile_id));
    assert_eq!(
        service_state.profiles[profile_id].profile_class,
        ProfileClass::ManagedOneTime
    );
    assert!(warning.is_null());
}
#[test]
fn test_remote_view_open_reuses_existing_managed_one_time_profile() {
    let repository = LockedServiceStateRepository::default_json().expect("repository");
    let mut intent = crate::native::remote_view::RemoteViewOpenIntent {
        url: Some("https://direct.sos.state.tx.us/acct/acct-templogin.asp".to_string()),
        runtime_profile: None,
        profile: None,
        browser_id: None,
        session_name: None,
        service_name: Some("sosdirect".to_string()),
        agent_name: Some("codex".to_string()),
        task_name: Some("temporary-login-payment".to_string()),
        browser_build: Some("stock_chrome".to_string()),
        browser_host: "remote_headed".to_string(),
        view_stream_provider: ViewStreamProvider::RdpGateway,
        control_input: "manual_attached_desktop".to_string(),
        route_pool_entry_id: None,
        route_id: None,
        display_allocation_id: None,
        remote_headed_display: None,
        display_isolation: Some("private_virtual_display".to_string()),
        manual_login_launch: false,
        dry_run: true,
    };
    let profile_id = remote_view_open_managed_one_time_profile_id(&intent);
    let mut service_state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.clone(),
            BrowserProfile {
                id: profile_id.clone(),
                name: "Managed one-time temporary-login-payment".to_string(),
                profile_class: ProfileClass::ManagedOneTime,
                persistent: false,
                ..BrowserProfile::default()
            },
        )]),
        ..ServiceState::default()
    };
    let managed = remote_view_open_ensure_managed_one_time_profile(
        &repository,
        &mut service_state,
        &mut intent,
        true,
    )
    .expect("managed one-time profile");
    assert_eq!(managed["state"], "reused");
    assert_eq!(managed["profileId"], profile_id);
    assert_eq!(intent.runtime_profile.as_deref(), Some(profile_id.as_str()));
    assert_eq!(service_state.profiles.len(), 1);
}
#[test]
fn test_tab_handle_refresh_classifies_retained_candidates() {
    let ready_browser = BrowserProcess {
        id: "browser-ready".to_string(),
        health: ServiceBrowserHealth::Ready,
        ..BrowserProcess::default()
    };
    let dead_browser = BrowserProcess {
        id: "browser-dead".to_string(),
        health: ServiceBrowserHealth::ProcessExited,
        ..BrowserProcess::default()
    };
    let exact_tab = BrowserTab {
        id: "target:old-target".to_string(),
        browser_id: "browser-ready".to_string(),
        target_id: Some("old-target".to_string()),
        lifecycle: TabLifecycle::Ready,
        url: Some("https://example.com/old".to_string()),
        ..BrowserTab::default()
    };
    let closed_tab = BrowserTab {
        lifecycle: TabLifecycle::Closed,
        ..exact_tab.clone()
    };
    let same_origin_tab = BrowserTab {
        id: "target:new-target".to_string(),
        browser_id: "browser-ready".to_string(),
        target_id: Some("new-target".to_string()),
        lifecycle: TabLifecycle::Ready,
        url: Some("https://example.com/recover".to_string()),
        ..BrowserTab::default()
    };
    let blank_tab = BrowserTab {
        id: "target:blank-target".to_string(),
        browser_id: "browser-ready".to_string(),
        target_id: Some("blank-target".to_string()),
        lifecycle: TabLifecycle::Ready,
        url: Some("about:blank".to_string()),
        ..BrowserTab::default()
    };
    let incompatible_tab = BrowserTab {
        id: "target:other-target".to_string(),
        browser_id: "browser-ready".to_string(),
        target_id: Some("other-target".to_string()),
        lifecycle: TabLifecycle::Ready,
        url: Some("https://other.example/recover".to_string()),
        ..BrowserTab::default()
    };
    assert_eq!(
        classify_retained_tab_candidate(
            &exact_tab,
            Some(&ready_browser),
            Some("target:old-target"),
            Some("old-target"),
            Some("https://example.com")
        ),
        "exact_handle"
    );
    assert_eq!(
        classify_retained_tab_candidate(
            &closed_tab,
            Some(&ready_browser),
            Some("target:old-target"),
            Some("old-target"),
            Some("https://example.com")
        ),
        "closed_tab"
    );
    assert_eq!(
        classify_retained_tab_candidate(
            &same_origin_tab,
            Some(&ready_browser),
            Some("target:old-target"),
            Some("old-target"),
            Some("https://example.com")
        ),
        "compatible_same_origin_tab"
    );
    assert_eq!(
        classify_retained_tab_candidate(
            &blank_tab,
            Some(&ready_browser),
            Some("target:old-target"),
            Some("old-target"),
            Some("https://example.com")
        ),
        "compatible_blank_tab"
    );
    assert_eq!(
        classify_retained_tab_candidate(
            &incompatible_tab,
            Some(&ready_browser),
            Some("target:old-target"),
            Some("old-target"),
            Some("https://example.com")
        ),
        "incompatible_tab"
    );
    assert_eq!(
        classify_retained_tab_candidate(
            &same_origin_tab,
            Some(&dead_browser),
            Some("target:old-target"),
            Some("old-target"),
            Some("https://example.com")
        ),
        "dead_browser"
    );
}
#[test]
fn test_tab_handle_refresh_handle_builder_preserves_trace_context() {
    let previous = serde_json::Map::from_iter([
        ("browserId".to_string(), json!("session:old")),
        ("sessionName".to_string(), json!("old")),
        ("tabId".to_string(), json!("target:old-target")),
        ("targetId".to_string(), json!("old-target")),
        ("profileId".to_string(), json!("profile-1")),
        ("profileOrigin".to_string(), json!("agent_browser_owned")),
        ("leaseId".to_string(), json!("lease-1")),
        ("leaseState".to_string(), json!("shared")),
        ("valid".to_string(), json!(false)),
        ("staleReason".to_string(), json!("tab_closed")),
    ]);
    let refreshed = refreshed_service_tab_handle(
        &previous,
        "service-session",
        "new-target",
        "https://example.com/recover",
        "Recovered",
    );
    assert_eq!(refreshed["browserId"], "session:service-session");
    assert_eq!(refreshed["sessionName"], "service-session");
    assert_eq!(refreshed["tabId"], "target:old-target");
    assert_eq!(refreshed["targetId"], "new-target");
    assert_eq!(refreshed["profileId"], "profile-1");
    assert_eq!(refreshed["valid"], true);
    assert_eq!(refreshed["staleReason"], Value::Null);
    assert_eq!(
        refreshed["traceFilter"]["browserId"],
        "session:service-session"
    );
    assert_eq!(refreshed["traceFilter"]["profileId"], "profile-1");
    assert_eq!(refreshed["traceFilter"]["sessionId"], "service-session");
}
#[test]
fn test_tab_new_shared_acquisition_evidence_reports_reused_route_hints() {
    let command = json!(
        { "action" : "tab_new", "browserId" : "session:runtime-session", "sessionName" :
        "runtime-session", "runtimeProfile" : "auracall-profile" }
    );
    let evidence =
        tab_new_shared_acquisition_evidence(&command, "runtime-session", json!("auracall-profile"));
    assert_eq!(evidence["policy"], "shared_browser_tabs");
    assert_eq!(evidence["mode"], "tab_new");
    assert_eq!(evidence["action"], "opened_new_tab");
    assert_eq!(evidence["browserReused"], true);
    assert_eq!(evidence["tabOpened"], true);
    assert_eq!(evidence["waitedForProfileLease"], false);
    assert_eq!(evidence["rejectedDuplicateProcess"], false);
    assert_eq!(evidence["duplicateProcessAllowed"], false);
    assert_eq!(
        evidence["duplicateProcessPolicy"],
        "reject_duplicate_process"
    );
    assert_eq!(evidence["browserId"], "session:runtime-session");
    assert_eq!(evidence["sessionName"], "runtime-session");
    assert_eq!(evidence["profileId"], "auracall-profile");
    assert_eq!(evidence["plannedProfile"], "auracall-profile");
    assert_eq!(evidence["requestedBrowserId"], "session:runtime-session");
    assert_eq!(evidence["requestedSessionName"], "runtime-session");
    assert_eq!(
        evidence["routeHintFields"],
        json!(["browserId", "sessionName"])
    );
    assert_eq!(evidence["routeHintSource"], "request.browserId_sessionName");
}
#[test]
fn test_tab_new_shared_acquisition_evidence_reports_direct_tab() {
    let command = json!({ "action" : "tab_new", "runtimeProfile" : "scratch-profile" });
    let evidence =
        tab_new_shared_acquisition_evidence(&command, "scratch-session", json!("scratch-profile"));
    assert_eq!(evidence["policy"], "shared_browser_tabs");
    assert_eq!(evidence["mode"], "tab_new");
    assert_eq!(evidence["browserReused"], false);
    assert_eq!(evidence["tabOpened"], true);
    assert_eq!(evidence["browserId"], "session:scratch-session");
    assert_eq!(evidence["sessionName"], "scratch-session");
    assert_eq!(evidence["requestedBrowserId"], Value::Null);
    assert_eq!(evidence["requestedSessionName"], Value::Null);
    assert_eq!(evidence["routeHintFields"], json!([]));
    assert_eq!(evidence["routeHintSource"], "none");
}
#[test]
fn test_active_browser_profile_mismatch_rejects_wrong_runtime_profile() {
    let message = active_browser_profile_mismatch_message(
        Some("auracall-chatgpt-wsl-chrome-2-consult"),
        Some("/home/me/.auracall/browser-profiles/chatgpt-consult"),
        Some("default"),
        Some(Path::new(
            "/home/me/.agent-browser/runtime-profiles/default/user-data",
        )),
        "default",
    )
    .expect("mismatched selected profile should fail closed");
    assert!(message.contains("selected profile mismatch"));
    assert!(message.contains("auracall-chatgpt-wsl-chrome-2-consult"));
    assert!(message.contains("runtimeProfile=default"));
}
#[test]
fn test_active_browser_profile_mismatch_allows_matching_runtime_profile() {
    assert!(active_browser_profile_mismatch_message(
        Some("auracall-chatgpt-wsl-chrome-2-consult"),
        Some("/home/me/.auracall/browser-profiles/chatgpt-consult"),
        Some("auracall-chatgpt-wsl-chrome-2-consult"),
        Some(Path::new("/different/path")),
        "auracall-chatgpt-wsl-chrome-2-consult",
    )
    .is_none());
}
#[test]
fn test_active_browser_profile_mismatch_allows_matching_profile_path() {
    assert!(active_browser_profile_mismatch_message(
        Some("auracall-chatgpt-wsl-chrome-2-consult"),
        Some("/tmp/agent-browser-profile-match"),
        None,
        Some(Path::new("/tmp/agent-browser-profile-match")),
        "profile-path-session",
    )
    .is_none());
}
#[test]
fn test_transient_wsl_predevtools_launch_error_is_retryable_only_for_chrome() {
    let error = "Chrome exited early (exit code: 1) without exposing DevTools\nChrome stderr:\n  <3>WSL (123 - ) ERROR: UtilAcceptVsock:271: accept4 failed 110";
    assert!(should_retry_transient_chrome_predevtools_launch_error(
        Some("chrome"),
        error
    ));
    assert!(should_retry_transient_chrome_predevtools_launch_error(
        None, error
    ));
    assert!(!should_retry_transient_chrome_predevtools_launch_error(
        Some("lightpanda"),
        error
    ));
    assert!(!should_retry_transient_chrome_predevtools_launch_error(
        Some("chrome"),
        "Chrome exited early without exposing DevTools"
    ));
}
#[test]
fn test_tab_handle_release_closes_only_selected_tab_record() {
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([(
            "session:shared-session".to_string(),
            BrowserProcess {
                id: "session:shared-session".to_string(),
                profile_id: Some("shared-profile".to_string()),
                health: ServiceBrowserHealth::Ready,
                active_session_ids: vec!["shared-session".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "shared-session".to_string(),
            BrowserSession {
                id: "shared-session".to_string(),
                service_name: Some("AuraCall".to_string()),
                agent_name: Some("auracall-agent".to_string()),
                task_name: Some("shared-tab".to_string()),
                lease: LeaseState::Exclusive,
                profile_id: Some("shared-profile".to_string()),
                cleanup: SessionCleanupPolicy::Detach,
                browser_ids: vec!["session:shared-session".to_string()],
                tab_ids: vec!["target:tab-a".to_string(), "target:tab-b".to_string()],
                ..BrowserSession::default()
            },
        )]),
        tabs: BTreeMap::from([
            (
                "target:tab-a".to_string(),
                BrowserTab {
                    id: "target:tab-a".to_string(),
                    browser_id: "session:shared-session".to_string(),
                    target_id: Some("tab-a".to_string()),
                    session_id: Some("shared-session".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    owner_session_id: Some("shared-session".to_string()),
                    ..BrowserTab::default()
                },
            ),
            (
                "target:tab-b".to_string(),
                BrowserTab {
                    id: "target:tab-b".to_string(),
                    browser_id: "session:shared-session".to_string(),
                    target_id: Some("tab-b".to_string()),
                    session_id: Some("shared-session".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    owner_session_id: Some("shared-session".to_string()),
                    ..BrowserTab::default()
                },
            ),
        ]),
        ..ServiceState::default()
    };
    service_state.refresh_service_tab_handles();
    let handle_value = serde_json::to_value(
        service_state.tabs["target:tab-a"]
            .service_tab_handle
            .clone()
            .expect("tab handle should exist"),
    )
    .expect("handle should serialize");
    let handle = handle_value
        .as_object()
        .expect("handle should be an object");
    let result = release_service_tab_handle_record(
        &mut service_state,
        handle,
        "shared-session",
        "2026-06-19T22:45:00Z",
        &json!(
            { "attempted" : false, "closed" : false, "skippedReason" :
            "no_live_browser", "error" : Value::Null, "result" : Value::Null, }
        ),
    )
    .expect("release should succeed");
    assert_eq!(result["action"], "tab_handle_release");
    assert_eq!(result["tabReleased"], true);
    assert_eq!(result["browserProcessPreserved"], true);
    assert_eq!(result["sessionRoutePreserved"], true);
    assert_eq!(result["closeBrowserOnRelease"], false);
    assert_eq!(result["physicalTabCloseAttempted"], false);
    assert_eq!(result["physicalTabClosed"], false);
    assert_eq!(result["physicalTabCloseSkippedReason"], "no_live_browser");
    assert_eq!(
        service_state.tabs["target:tab-a"].lifecycle,
        TabLifecycle::Closed
    );
    assert_eq!(
        service_state.tabs["target:tab-b"].lifecycle,
        TabLifecycle::Ready
    );
    assert!(service_state
        .browsers
        .contains_key("session:shared-session"));
    assert_eq!(
        service_state.browsers["session:shared-session"].active_session_ids,
        vec!["shared-session".to_string()]
    );
    assert_eq!(
        service_state.sessions["shared-session"].lease,
        LeaseState::Exclusive
    );
    assert_eq!(
        service_state.sessions["shared-session"].tab_ids,
        vec!["target:tab-a".to_string(), "target:tab-b".to_string()]
    );
    assert_eq!(result["serviceTabHandle"]["staleReason"], "tab_closed");
    assert_eq!(
        service_state.tabs["target:tab-a"]
            .service_tab_handle
            .as_ref()
            .and_then(|handle| handle.stale_reason.as_deref()),
        Some("tab_closed")
    );
}
#[test]
fn test_tab_handle_refresh_classifies_live_pages_by_origin() {
    assert_eq!(
        classify_live_page_candidate(
            "old-target",
            "https://example.com/old",
            Some("old-target"),
            Some("https://example.com")
        ),
        "matching_target"
    );
    assert_eq!(
        classify_live_page_candidate(
            "blank-target",
            "about:blank",
            Some("old-target"),
            Some("https://example.com")
        ),
        "compatible_blank_tab"
    );
    assert_eq!(
        classify_live_page_candidate(
            "new-target",
            "https://example.com/recover",
            Some("old-target"),
            Some("https://example.com")
        ),
        "compatible_same_origin_tab"
    );
    assert_eq!(
        classify_live_page_candidate(
            "other-target",
            "https://other.example/recover",
            Some("old-target"),
            Some("https://example.com")
        ),
        "incompatible_tab"
    );
}
#[test]
fn test_tab_handle_refresh_selects_compatible_duplicate_live_pages() {
    let pages = vec![
        PageInfo {
            target_id: "selected-target".to_string(),
            session_id: "session-selected".to_string(),
            url: "https://example.com/current".to_string(),
            title: "Selected".to_string(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "same-origin-target".to_string(),
            session_id: "session-same-origin".to_string(),
            url: "https://example.com/duplicate".to_string(),
            title: "Duplicate".to_string(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "blank-target".to_string(),
            session_id: "session-blank".to_string(),
            url: "about:blank".to_string(),
            title: String::new(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "other-target".to_string(),
            session_id: "session-other".to_string(),
            url: "https://other.example/current".to_string(),
            title: "Other".to_string(),
            target_type: "page".to_string(),
        },
    ];
    let duplicates = compatible_duplicate_live_pages(
        &pages,
        "selected-target",
        Some("stale-target"),
        Some("https://example.com"),
    );
    assert_eq!(duplicates.len(), 2);
    assert_eq!(duplicates[0]["targetId"], "same-origin-target");
    assert_eq!(
        duplicates[0]["classification"],
        "compatible_same_origin_tab"
    );
    assert_eq!(duplicates[1]["targetId"], "blank-target");
    assert_eq!(duplicates[1]["classification"], "compatible_blank_tab");
}
#[test]
fn test_remote_view_open_reusable_live_target_prefers_same_origin_non_blank_page() {
    let pages = vec![
        PageInfo {
            target_id: "blank-target".to_string(),
            session_id: "session-blank".to_string(),
            url: "about:blank".to_string(),
            title: String::new(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "same-origin-target".to_string(),
            session_id: "session-same-origin".to_string(),
            url: "https://example.com/current".to_string(),
            title: "Current".to_string(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "other-target".to_string(),
            session_id: "session-other".to_string(),
            url: "https://other.example/current".to_string(),
            title: "Other".to_string(),
            target_type: "page".to_string(),
        },
    ];
    let target = remote_view_open_reusable_live_target(
        &pages,
        Some("same-origin-target"),
        Some("https://example.com"),
    )
    .unwrap();
    assert_eq!(target.target_id, "same-origin-target");
}
#[test]
fn test_remote_view_open_reusable_live_target_prefers_handoff_target() {
    let pages = vec![
        PageInfo {
            target_id: "same-origin-first".to_string(),
            session_id: "session-first".to_string(),
            url: "https://example.com/first".to_string(),
            title: "First".to_string(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "handoff-target".to_string(),
            session_id: "session-handoff".to_string(),
            url: "https://example.com/article".to_string(),
            title: "Article".to_string(),
            target_type: "page".to_string(),
        },
    ];
    let target = remote_view_open_reusable_live_target(
        &pages,
        Some("handoff-target"),
        Some("https://example.com"),
    )
    .unwrap();
    assert_eq!(target.target_id, "handoff-target");
}
#[test]
fn test_remote_view_open_creates_blank_target_before_destination_navigation() {
    let command = json!(
        { "action" : "remote_view_open", "url" : "https://www.linkedin.com/feed/",
        "runtimeProfile" : "last30days-facebook", "serviceName" : "last30days",
        "jobTimeoutMs" : 90_000, }
    );
    let initial = remote_view_open_tab_creation_command(&command);
    assert_eq!(initial["url"], "about:blank");
    assert_eq!(initial["runtimeProfile"], "last30days-facebook");
    assert_eq!(initial["serviceName"], "last30days");
    assert_eq!(initial["jobTimeoutMs"], 90_000);
    assert_eq!(command["url"], "https://www.linkedin.com/feed/");
}
#[test]
fn test_remote_view_open_reuses_only_exact_active_target_metadata() {
    let pages = vec![PageInfo {
        target_id: "target-feed".to_string(),
        session_id: "page-session".to_string(),
        url: "https://www.linkedin.com/feed/".to_string(),
        title: "Feed | LinkedIn".to_string(),
        target_type: "page".to_string(),
    }];
    let readback =
        remote_view_open_active_target_readback(Some("target-feed"), &pages, "target-feed")
            .unwrap();
    assert_eq!(readback["state"], "already_active");
    assert_eq!(readback["url"], "https://www.linkedin.com/feed/");
    assert!(
        remote_view_open_active_target_readback(Some("other-target"), &pages, "target-feed")
            .is_none()
    );
}
#[test]
fn test_remote_view_open_reusable_live_target_rejects_blank_only_pages() {
    let pages = vec![PageInfo {
        target_id: "blank-target".to_string(),
        session_id: "session-blank".to_string(),
        url: "about:blank".to_string(),
        title: String::new(),
        target_type: "page".to_string(),
    }];
    assert!(
        remote_view_open_reusable_live_target(&pages, None, Some("https://example.com")).is_none()
    );
}
#[test]
fn test_remote_view_open_retained_tab_candidate_requires_ready_same_origin_tab() {
    let service_state = ServiceState {
        tabs: BTreeMap::from([
            (
                "target:selected-target".to_string(),
                BrowserTab {
                    id: "target:selected-target".to_string(),
                    browser_id: "browser-a".to_string(),
                    target_id: Some("selected-target".to_string()),
                    owner_session_id: Some("session-a".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    url: Some("https://example.com/current".to_string()),
                    title: Some("Current".to_string()),
                    ..BrowserTab::default()
                },
            ),
            (
                "target:closed-target".to_string(),
                BrowserTab {
                    id: "target:closed-target".to_string(),
                    browser_id: "browser-a".to_string(),
                    target_id: Some("closed-target".to_string()),
                    owner_session_id: Some("session-a".to_string()),
                    lifecycle: TabLifecycle::Closed,
                    url: Some("https://example.com/closed".to_string()),
                    ..BrowserTab::default()
                },
            ),
        ]),
        ..ServiceState::default()
    };
    let tab = remote_view_open_retained_tab_candidate(
        &service_state,
        "browser-a",
        "session-a",
        Some("https://example.com"),
    )
    .expect("retained tab");
    assert_eq!(tab.target_id.as_deref(), Some("selected-target"));
    assert!(remote_view_open_retained_tab_candidate(
        &service_state,
        "browser-a",
        "session-a",
        Some("https://other.example"),
    )
    .is_none());
}
#[test]
fn service_browser_host_for_launch_honors_nested_remote_headed_param() {
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed", "headless"
        : false } }
    );
    assert_eq!(
        service_browser_host_for_launch(&command, false),
        ServiceBrowserHost::RemoteHeaded
    );
}
#[test]
fn apply_launch_host_hints_defaults_remote_headed_to_private_display() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_HEADED_DISPLAY"]);
    guard.set("AGENT_BROWSER_REMOTE_HEADED_DISPLAY", ":93");
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed" } }
    );
    let mut options = LaunchOptions::default();
    let host = apply_launch_host_hints(&mut options, &command);
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(!options.headless);
    assert!(options.remote_headed);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("private_virtual_display")
    );
    assert_eq!(options.display, None);
}
#[test]
fn apply_launch_host_hints_preserves_explicit_private_over_configured_remote_display() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_HEADED_DISPLAY"]);
    guard.set("AGENT_BROWSER_REMOTE_HEADED_DISPLAY", ":93");
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed" } }
    );
    let mut options = LaunchOptions {
        remote_headed_display_isolation: Some("private_virtual_display".to_string()),
        ..LaunchOptions::default()
    };
    let host = apply_launch_host_hints(&mut options, &command);
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(!options.headless);
    assert!(options.remote_headed);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("private_virtual_display")
    );
    assert_eq!(options.display, None);
}
#[test]
fn apply_launch_host_hints_allows_private_remote_headed_display() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_HEADED_DISPLAY"]);
    guard.set("AGENT_BROWSER_REMOTE_HEADED_DISPLAY", ":93");
    let command = json!(
        { "action" : "tab_new", "displayIsolation" : "private_virtual_display", "params"
        : { "browserHost" : "remote_headed", "display" : ":94" } }
    );
    let mut options = LaunchOptions::default();
    let host = apply_launch_host_hints(&mut options, &command);
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(!options.headless);
    assert!(options.remote_headed);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("private_virtual_display")
    );
    assert_eq!(options.display, None);
}
#[test]
fn manual_login_launch_accepts_params_only_for_headed_launches() {
    let command = json!({ "params" : { "manualLoginLaunch" : true } });
    assert!(manual_login_launch_from_command(&command, false).unwrap());
    assert!(manual_login_launch_from_command(&command, true)
        .unwrap_err()
        .contains("manual_login_launch_requires_headed"));
}
#[test]
fn apply_launch_host_hints_allows_shared_remote_headed_display() {
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed",
        "displayIsolation" : "shared_display", "remoteHeadedDisplay" : ":95" } }
    );
    let mut options = LaunchOptions::default();
    let host = apply_launch_host_hints(&mut options, &command);
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(options.remote_headed);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("shared_display")
    );
    assert_eq!(options.display.as_deref(), Some(":95"));
}
#[test]
fn apply_launch_host_hints_allows_ambient_remote_headed_display() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_HEADED_DISPLAY"]);
    guard.set("AGENT_BROWSER_REMOTE_HEADED_DISPLAY", ":93");
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed",
        "displayIsolation" : "ambient_display" } }
    );
    let mut options = LaunchOptions::default();
    let host = apply_launch_host_hints(&mut options, &command);
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(options.remote_headed);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("ambient_display")
    );
    assert_eq!(options.display, None);
}
#[test]
fn remote_headed_view_stream_defaults_to_cdp_screencast() {
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed" } }
    );
    let streams = remote_headed_view_streams_from_command(&command);
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].provider, ViewStreamProvider::CdpScreencast);
    assert_eq!(
        streams[0].control_input,
        Some(ControlInputProvider::CdpInput)
    );
    assert_eq!(streams[0].id, "remote-headed-view");
}
#[test]
fn remote_headed_view_stream_accepts_nested_provider_url_and_control_input() {
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed",
        "viewStreamProvider" : "rdp_gateway", "controlInputProvider" :
        "manual_attached_desktop", "viewStreamUrl" : "http://127.0.0.1:8080/rdp/session"
        } }
    );
    let streams = remote_headed_view_streams_from_command(&command);
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].provider, ViewStreamProvider::RdpGateway);
    assert_eq!(
        streams[0].control_input,
        Some(ControlInputProvider::ManualAttachedDesktop)
    );
    assert_eq!(
        streams[0].url.as_deref(),
        Some("http://127.0.0.1:8080/rdp/session")
    );
}
#[test]
fn remote_headed_view_stream_accepts_service_owned_route_metadata() {
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed",
        "viewStreamProvider" : "rdp_gateway", "controlInputProvider" :
        "manual_attached_desktop", "viewStreamUrl" :
        "https://agent-browser.example/guacamole/", "frameUrl" :
        "https://agent-browser.example/guacamole/#/client/browser-a", "externalUrl" :
        "https://agent-browser.example/guacamole/#/client/browser-a", "routeId" :
        "route-browser-a", "guacamoleConnectionId" : "browser-a",
        "guacamoleConnectionName" : "Browser A" } }
    );
    let streams = remote_headed_view_streams_from_command(&command);
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].provider, ViewStreamProvider::RdpGateway);
    assert_eq!(
        streams[0].url.as_deref(),
        Some("https://agent-browser.example/guacamole/")
    );
    assert_eq!(
        streams[0].frame_url.as_deref(),
        Some("https://agent-browser.example/guacamole/#/client/browser-a")
    );
    assert_eq!(
        streams[0].external_url.as_deref(),
        Some("https://agent-browser.example/guacamole/#/client/browser-a")
    );
    assert_eq!(streams[0].route_id.as_deref(), Some("route-browser-a"));
    assert_eq!(streams[0].connection_id.as_deref(), Some("browser-a"));
    assert_eq!(streams[0].connection_name.as_deref(), Some("Browser A"));
    assert_eq!(streams[0].route_source.as_deref(), Some("service_request"));
}
#[test]
fn remote_headed_view_stream_does_not_invent_guacamole_route_from_root_url() {
    let _guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_URL"]);
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed",
        "viewStreamProvider" : "rdp_gateway", "viewStreamUrl" :
        "https://agent-browser.example/guacamole/" } }
    );
    let streams = remote_headed_view_streams_from_command(&command);
    assert_eq!(streams.len(), 1);
    assert_eq!(
        streams[0].url.as_deref(),
        Some("https://agent-browser.example/guacamole/")
    );
    assert!(streams[0].frame_url.is_none());
    assert!(streams[0].external_url.is_none());
    assert!(streams[0].connection_id.is_none());
}
#[test]
fn remote_headed_view_stream_derives_route_identity_from_guacamole_client_url() {
    let _guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_URL"]);
    let command = json!(
        { "action" : "tab_new", "params" : { "browserHost" : "remote_headed",
        "viewStreamProvider" : "rdp_gateway", "viewStreamUrl" :
        "https://agent-browser.example/guacamole/#/client/browser-a" } }
    );
    let streams = remote_headed_view_streams_from_command(&command);
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].connection_id.as_deref(), Some("browser-a"));
    assert_eq!(streams[0].route_id.as_deref(), Some("guacamole:browser-a"));
    assert_eq!(streams[0].route_source.as_deref(), Some("service_request"));
}
#[test]
fn test_target_service_ids_from_command_accepts_singular_and_arrays() {
    let command = json!(
        { "targetServiceId" : "google", "targetServices" : ["acs", " google ", "", 7],
        "siteId" : "nih", "loginIds" : ["orcid", "acs"], "target_service_ids" :
        ["microsoft"], "login_id" : "era" }
    );
    assert_eq!(
        target_service_ids_from_command(&command),
        vec![
            "google".to_string(),
            "nih".to_string(),
            "era".to_string(),
            "acs".to_string(),
            "orcid".to_string(),
            "microsoft".to_string()
        ]
    );
}
#[test]
fn test_apply_service_profile_selection_prefers_authenticated_target() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("profile-selection-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let mut service_state = ServiceState::default();
    service_state.profiles.insert(
        "journal-default".to_string(),
        BrowserProfile {
            id: "journal-default".to_string(),
            name: "Journal default".to_string(),
            user_data_dir: Some(home.join("journal-default").display().to_string()),
            target_service_ids: vec![
                "acs".to_string(),
                "google".to_string(),
                "microsoft".to_string(),
                "orcid".to_string(),
                "nih".to_string(),
                "pubmed".to_string(),
                "crossref".to_string(),
                "scopus".to_string(),
                "wos".to_string(),
                "canvas".to_string(),
                "github".to_string(),
                "gmail".to_string(),
                "outlook".to_string(),
            ],
            shared_service_ids: vec!["JournalDownloader".to_string()],
            persistent: true,
            ..BrowserProfile::default()
        },
    );
    service_state.profiles.insert(
        "journal-auth".to_string(),
        BrowserProfile {
            id: "journal-auth".to_string(),
            name: "Journal authenticated".to_string(),
            user_data_dir: Some(home.join("journal-auth").display().to_string()),
            target_service_ids: vec!["acs".to_string()],
            authenticated_service_ids: vec!["acs".to_string()],
            shared_service_ids: vec!["JournalDownloader".to_string()],
            persistent: true,
            ..BrowserProfile::default()
        },
    );
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&service_state)
        .expect("service state should be persisted");
    let mut options = LaunchOptions::default();
    let selected = apply_service_profile_selection(
        &mut options,
        &json!(
            { "serviceName" : "JournalDownloader", "targetServiceId" : "acs",
            "targetServices" : ["google", "microsoft", "orcid", "nih", "pubmed",
            "crossref", "scopus", "wos", "canvas", "github", "gmail", "outlook"] }
        ),
    );
    assert_eq!(selected, Some(ProfileSelectionReason::AuthenticatedTarget));
    assert_eq!(options.runtime_profile.as_deref(), Some("journal-auth"));
    let expected_profile = home.join("journal-auth").display().to_string();
    assert_eq!(options.profile.as_deref(), Some(expected_profile.as_str()));
}
#[test]
fn test_apply_service_profile_selection_preserves_explicit_profile() {
    let mut options = LaunchOptions {
        profile: Some("/tmp/explicit-profile".to_string()),
        ..LaunchOptions::default()
    };
    let selected = apply_service_profile_selection(
        &mut options,
        &json!({ "serviceName" : "JournalDownloader", "targetServiceId" : "acs" }),
    );
    assert!(selected.is_none());
    assert_eq!(options.profile.as_deref(), Some("/tmp/explicit-profile"));
    assert!(options.runtime_profile.is_none());
}
#[test]
fn test_apply_service_profile_selection_resolves_explicit_runtime_profile_directory() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("explicit-runtime-profile-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let user_data_dir = home.join("paired-google-messages-profile");
    let mut service_state = ServiceState::default();
    service_state.profiles.insert(
        "google-messages-main".to_string(),
        BrowserProfile {
            id: "google-messages-main".to_string(),
            name: "Google Messages main".to_string(),
            user_data_dir: Some(user_data_dir.display().to_string()),
            browser_build: Some(BrowserBuild::StockChrome),
            persistent: true,
            ..BrowserProfile::default()
        },
    );
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&service_state)
        .expect("service state should be persisted");
    let mut options = LaunchOptions {
        runtime_profile: Some("google-messages-main".to_string()),
        executable_path: Some("/tmp/environment-browser".to_string()),
        ..LaunchOptions::default()
    };
    let selected = apply_service_profile_selection(
        &mut options,
        &json!(
            { "action" : "launch", "serviceName" : "im-receipts", "runtimeProfile" :
            "google-messages-main", "browserBuild" : "stock_chrome" }
        ),
    );
    assert_eq!(selected, Some(ProfileSelectionReason::ExplicitProfile));
    assert_eq!(
        options.profile.as_deref(),
        Some(user_data_dir.to_str().expect("path should be utf-8"))
    );
    assert_eq!(
        options.runtime_profile.as_deref(),
        Some("google-messages-main")
    );
    assert!(options.executable_path.is_none());
}
#[test]
fn test_apply_auto_launch_command_hints_honors_planned_identity_and_capability() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_EXECUTABLE_PATH"]);
    let home = unique_socket_dir("auto-launch-command-hints");
    fs::create_dir_all(&home).expect("test home should be created");
    let executable = home.join("chrome");
    let user_data_dir = home.join("stealth-profile");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    guard.remove("AGENT_BROWSER_EXECUTABLE_PATH");
    let command = json!(
        { "action" : "tab_new", "serviceName" : "CanaryRunner", "targetServiceId" :
        "canary-site", "browserBuild" : "stealthcdp_chromium", "runtimeProfile" :
        "stealth-profile", "profile" : user_data_dir.display().to_string(), "params" : {
        "browserHost" : "remote_headed", "displayIsolation" : "private_virtual_display",
        "viewStreamProvider" : "rdp_gateway", "controlInputProvider" :
        "manual_attached_desktop", "viewStreamUrl" :
        "http://agent-browser.localhost/guacamole/" }, "serviceState" : {
        "browserCapabilityRegistry" : { "browserHosts" : [{ "id" : "linux-local",
        "hostKind" : "local", "reachable" : true, "lifecycleOwner" : "agent_browser" }],
        "browserExecutables" : [{ "id" : "stealth-current", "hostId" : "linux-local",
        "buildLabel" : "stealthcdp_chromium", "executablePath" : executable.display()
        .to_string() }], "browserCapabilities" : [{ "id" : "stealth-capability", "hostId"
        : "linux-local", "executableId" : "stealth-current", "cdpSupported" : true,
        "headedSupported" : true, "headlessSupported" : true }], "profileCompatibility" :
        [{ "id" : "stealth-profile-compatible", "profileId" : "stealth-profile", "hostId"
        : "linux-local", "executableId" : "stealth-current", "compatible" : true,
        "requiresOperatorOverride" : false }], "browserPreferenceBindings" : [{ "id" :
        "canary-stealth-default", "scope" : "site", "targetServiceIds" : ["canary-site"],
        "preferredHostId" : "linux-local", "preferredExecutableId" : "stealth-current",
        "preferredCapabilityId" : "stealth-capability", "browserBuild" :
        "stealthcdp_chromium", "priority" : 50 }], "validationEvidence" : [{ "id" :
        "stealth-launch-smoke", "hostId" : "linux-local", "executableId" :
        "stealth-current", "capabilityId" : "stealth-capability", "kind" : "launch",
        "state" : "passed" }] } } }
    );
    let mut options = LaunchOptions::default();
    let (host, selection_reason, browser_capability_launch, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    let metadata = ServiceLaunchMetadata::from_launch_options(
        &options,
        Some(&effective_command),
        selection_reason,
    );
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(!options.headless);
    assert!(options.remote_headed);
    assert_eq!(options.runtime_profile.as_deref(), Some("stealth-profile"));
    assert_eq!(
        options.profile.as_deref(),
        Some(user_data_dir.to_str().expect("path should be utf-8"))
    );
    assert_eq!(
        options.executable_path.as_deref(),
        Some(executable.to_str().expect("path should be utf-8"))
    );
    assert!(browser_capability_launch.applied);
    assert_eq!(
        browser_capability_launch.to_value()["bindingId"],
        "canary-stealth-default"
    );
    assert_eq!(metadata.profile_id.as_deref(), Some("stealth-profile"));
    assert_eq!(metadata.view_streams.len(), 1);
    assert_eq!(
        metadata.view_streams[0].provider,
        ViewStreamProvider::RdpGateway
    );
    assert_eq!(
        metadata.view_streams[0].control_input,
        Some(ControlInputProvider::ManualAttachedDesktop)
    );
    assert_eq!(
        metadata.view_streams[0].url.as_deref(),
        Some("http://agent-browser.localhost/guacamole/")
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_apply_auto_launch_command_hints_preserves_explicit_runtime_profile() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_EXECUTABLE_PATH"]);
    guard.remove("AGENT_BROWSER_EXECUTABLE_PATH");
    let home = unique_socket_dir("auto-launch-explicit-runtime-profile");
    fs::create_dir_all(&home).expect("test home should be created");
    let executable = home.join("chrome");
    let user_data_dir = home.join("stealthcdp-default");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    let service_state = json!(
        { "defaultBrowserBuild" : "stealthcdp_chromium", "profiles" : {
        "stealthcdp-default" : { "id" : "stealthcdp-default", "name" : "Stealth default",
        "userDataDir" : user_data_dir.display().to_string(), "defaultBrowserHost" :
        "remote_headed", "browserBuild" : "stealthcdp_chromium", "persistent" : true } },
        "browserCapabilityRegistry" : { "browserHosts" : [{ "id" : "linux-local",
        "hostKind" : "local", "reachable" : true, "lifecycleOwner" : "agent_browser" }],
        "browserExecutables" : [{ "id" : "stealth-current", "hostId" : "linux-local",
        "buildLabel" : "stealthcdp_chromium", "executablePath" : executable.display()
        .to_string() }], "browserCapabilities" : [{ "id" : "stealth-capability", "hostId"
        : "linux-local", "executableId" : "stealth-current", "cdpSupported" : true,
        "headedSupported" : true, "headlessSupported" : true }], "profileCompatibility" :
        [{ "id" : "stealth-profile-compatible", "profileId" : "stealthcdp-default",
        "hostId" : "linux-local", "executableId" : "stealth-current", "compatible" : true
        }], "browserPreferenceBindings" : [{ "id" : "global-stealth-default", "scope" :
        "global", "preferredHostId" : "linux-local", "preferredExecutableId" :
        "stealth-current", "preferredCapabilityId" : "stealth-capability", "browserBuild"
        : "stealthcdp_chromium", "priority" : 50 }], "validationEvidence" : [{ "id" :
        "stealth-launch-smoke", "hostId" : "linux-local", "executableId" :
        "stealth-current", "capabilityId" : "stealth-capability", "kind" : "launch",
        "state" : "passed" }] } }
    );
    let command = json!(
        { "action" : "tab_new", "url" : "https://example.com/", "runtimeProfile" :
        "switch-b-profile", "serviceState" : service_state }
    );
    let mut options = LaunchOptions::default();
    let (_host, selection_reason, _browser_capability_launch, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    assert!(selection_reason.is_none());
    assert_eq!(options.runtime_profile.as_deref(), Some("switch-b-profile"));
    assert!(options.profile.is_none());
    assert_eq!(
        effective_command
            .get("runtimeProfile")
            .and_then(Value::as_str),
        Some("switch-b-profile")
    );
    assert!(effective_command.get("profile").is_none());
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_open_preserves_runtime_profile_when_default_profile_is_locked_shape() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_EXECUTABLE_PATH"]);
    guard.remove("AGENT_BROWSER_EXECUTABLE_PATH");
    let home = unique_socket_dir("open-preserves-runtime-profile");
    fs::create_dir_all(&home).expect("test home should be created");
    let executable = home.join("chrome");
    let default_user_data_dir = home.join("stealthcdp-default");
    let requested_user_data_dir = home.join("last30days-facebook");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    let service_state = json!(
        { "defaultBrowserBuild" : "stealthcdp_chromium", "profiles" : {
        "stealthcdp-default" : { "id" : "stealthcdp-default", "name" : "Stealth default",
        "userDataDir" : default_user_data_dir.display().to_string(), "defaultBrowserHost"
        : "remote_headed", "browserBuild" : "stealthcdp_chromium", "persistent" : true },
        "last30days-facebook" : { "id" : "last30days-facebook", "name" :
        "Last 30 Days Facebook", "userDataDir" : requested_user_data_dir.display()
        .to_string(), "defaultBrowserHost" : "remote_headed", "browserBuild" :
        "stealthcdp_chromium", "persistent" : true } }, "browserCapabilityRegistry" : {
        "browserHosts" : [{ "id" : "linux-local", "hostKind" : "local", "reachable" :
        true, "lifecycleOwner" : "agent_browser" }], "browserExecutables" : [{ "id" :
        "stealth-current", "hostId" : "linux-local", "buildLabel" :
        "stealthcdp_chromium", "executablePath" : executable.display().to_string() }],
        "browserCapabilities" : [{ "id" : "stealth-capability", "hostId" : "linux-local",
        "executableId" : "stealth-current", "cdpSupported" : true, "headedSupported" :
        true, "headlessSupported" : true }], "profileCompatibility" : [{ "id" :
        "stealth-default-compatible", "profileId" : "stealthcdp-default", "hostId" :
        "linux-local", "executableId" : "stealth-current", "compatible" : true }, { "id"
        : "last30days-compatible", "profileId" : "last30days-facebook", "hostId" :
        "linux-local", "executableId" : "stealth-current", "compatible" : true }],
        "browserPreferenceBindings" : [{ "id" : "global-stealth-default", "scope" :
        "global", "preferredHostId" : "linux-local", "preferredExecutableId" :
        "stealth-current", "preferredCapabilityId" : "stealth-capability", "browserBuild"
        : "stealthcdp_chromium", "priority" : 50 }], "validationEvidence" : [{ "id" :
        "stealth-launch-smoke", "hostId" : "linux-local", "executableId" :
        "stealth-current", "capabilityId" : "stealth-capability", "kind" : "launch",
        "state" : "passed" }] }, "browsers" : { "session:detected-profile-mirror-38305-2"
        : { "id" : "session:detected-profile-mirror-38305-2", "profileId" :
        "stealthcdp-default", "host" : "remote_headed", "health" : "live",
        "activeSessionIds" : ["detected-profile-mirror-38305-2"] } } }
    );
    let command = json!(
        { "action" : "navigate", "url" : "https://x.com/home", "runtimeProfile" :
        "last30days-facebook", "browserBuild" : "stealthcdp_chromium", "browserHost" :
        "remote_headed", "serviceState" : service_state }
    );
    let mut options = LaunchOptions::default();
    let (host, selection_reason, _browser_capability_launch, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(selection_reason.is_none());
    assert_eq!(
        options.runtime_profile.as_deref(),
        Some("last30days-facebook")
    );
    assert!(options.profile.is_none());
    assert_eq!(
        effective_command
            .get("runtimeProfile")
            .and_then(Value::as_str),
        Some("last30days-facebook")
    );
    assert!(effective_command.get("profile").is_none());
    assert_ne!(
        effective_command.get("profile").and_then(Value::as_str),
        Some(
            default_user_data_dir
                .to_str()
                .expect("path should be utf-8")
        )
    );
    assert_eq!(effective_command["browserBuild"], "stealthcdp_chromium");
    assert_eq!(effective_command["browserHost"], "remote_headed");
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_apply_auto_launch_command_hints_preserves_explicit_profile_id() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_EXECUTABLE_PATH"]);
    guard.remove("AGENT_BROWSER_EXECUTABLE_PATH");
    let home = unique_socket_dir("auto-launch-explicit-profile-id");
    fs::create_dir_all(&home).expect("test home should be created");
    let executable = home.join("chrome");
    let user_data_dir = home.join("stealthcdp-default");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    let service_state = json!(
        { "defaultBrowserBuild" : "stealthcdp_chromium", "profiles" : {
        "stealthcdp-default" : { "id" : "stealthcdp-default", "name" : "Stealth default",
        "userDataDir" : user_data_dir.display().to_string(), "defaultBrowserHost" :
        "remote_headed", "browserBuild" : "stealthcdp_chromium", "persistent" : true } },
        "browserCapabilityRegistry" : { "browserHosts" : [{ "id" : "linux-local",
        "hostKind" : "local", "reachable" : true, "lifecycleOwner" : "agent_browser" }],
        "browserExecutables" : [{ "id" : "stealth-current", "hostId" : "linux-local",
        "buildLabel" : "stealthcdp_chromium", "executablePath" : executable.display()
        .to_string() }], "browserCapabilities" : [{ "id" : "stealth-capability", "hostId"
        : "linux-local", "executableId" : "stealth-current", "cdpSupported" : true,
        "headedSupported" : true, "headlessSupported" : true }], "profileCompatibility" :
        [{ "id" : "stealth-profile-compatible", "profileId" : "stealthcdp-default",
        "hostId" : "linux-local", "executableId" : "stealth-current", "compatible" : true
        }], "browserPreferenceBindings" : [{ "id" : "global-stealth-default", "scope" :
        "global", "preferredHostId" : "linux-local", "preferredExecutableId" :
        "stealth-current", "preferredCapabilityId" : "stealth-capability", "browserBuild"
        : "stealthcdp_chromium", "priority" : 50 }], "validationEvidence" : [{ "id" :
        "stealth-launch-smoke", "hostId" : "linux-local", "executableId" :
        "stealth-current", "capabilityId" : "stealth-capability", "kind" : "launch",
        "state" : "passed" }] } }
    );
    let command = json!(
        { "action" : "tab_new", "url" : "https://example.com/", "profileId" :
        "switch-c-profile", "serviceState" : service_state }
    );
    let mut options = LaunchOptions::default();
    let (_host, selection_reason, _browser_capability_launch, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    assert!(selection_reason.is_none());
    assert_eq!(options.runtime_profile.as_deref(), Some("switch-c-profile"));
    assert!(options.profile.is_none());
    assert_eq!(
        effective_command.get("profileId").and_then(Value::as_str),
        Some("switch-c-profile")
    );
    assert!(effective_command.get("runtimeProfile").is_none());
    assert!(effective_command.get("profile").is_none());
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_apply_auto_launch_command_hints_uses_effective_service_default() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_EXECUTABLE_PATH"]);
    guard.remove("AGENT_BROWSER_EXECUTABLE_PATH");
    let home = unique_socket_dir("auto-launch-effective-default");
    fs::create_dir_all(&home).expect("test home should be created");
    let executable = home.join("chrome");
    let user_data_dir = home.join("stealthcdp-default");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    let service_state = json!(
        { "defaultBrowserBuild" : "stealthcdp_chromium", "profiles" : {
        "stealthcdp-default" : { "id" : "stealthcdp-default", "name" : "Stealth default",
        "userDataDir" : user_data_dir.display().to_string(), "defaultBrowserHost" :
        "remote_headed", "browserBuild" : "stealthcdp_chromium", "persistent" : true } },
        "browserCapabilityRegistry" : { "browserHosts" : [{ "id" : "linux-local",
        "hostKind" : "local", "reachable" : true, "lifecycleOwner" : "agent_browser" }],
        "browserExecutables" : [{ "id" : "stealth-current", "hostId" : "linux-local",
        "buildLabel" : "stealthcdp_chromium", "executablePath" : executable.display()
        .to_string() }], "browserCapabilities" : [{ "id" : "stealth-capability", "hostId"
        : "linux-local", "executableId" : "stealth-current", "cdpSupported" : true,
        "headedSupported" : true, "headlessSupported" : true }], "profileCompatibility" :
        [{ "id" : "stealth-profile-compatible", "profileId" : "stealthcdp-default",
        "hostId" : "linux-local", "executableId" : "stealth-current", "compatible" : true
        }], "browserPreferenceBindings" : [{ "id" : "global-stealth-default", "scope" :
        "global", "preferredHostId" : "linux-local", "preferredExecutableId" :
        "stealth-current", "preferredCapabilityId" : "stealth-capability", "browserBuild"
        : "stealthcdp_chromium", "priority" : 50 }], "validationEvidence" : [{ "id" :
        "stealth-launch-smoke", "hostId" : "linux-local", "executableId" :
        "stealth-current", "capabilityId" : "stealth-capability", "kind" : "launch",
        "state" : "passed" }] } }
    );
    let command = json!(
        { "action" : "tab_new", "url" : "https://example.com/", "serviceState" :
        service_state }
    );
    let mut options = LaunchOptions::default();
    let (host, selection_reason, browser_capability_launch, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    let metadata = ServiceLaunchMetadata::from_launch_options(
        &options,
        Some(&effective_command),
        selection_reason,
    );
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(!options.headless);
    assert!(options.remote_headed);
    assert_eq!(
        options.runtime_profile.as_deref(),
        Some("stealthcdp-default")
    );
    assert_eq!(
        options.profile.as_deref(),
        Some(user_data_dir.to_str().expect("path should be utf-8"))
    );
    assert_eq!(
        options.executable_path.as_deref(),
        Some(executable.to_str().expect("path should be utf-8"))
    );
    assert!(browser_capability_launch.applied);
    assert_eq!(effective_command["browserBuild"], "stealthcdp_chromium");
    assert_eq!(effective_command["browserHost"], "remote_headed");
    assert_eq!(effective_command["viewStreamProvider"], "rdp_gateway");
    assert_eq!(
        effective_command["controlInputProvider"],
        "manual_attached_desktop"
    );
    assert_eq!(
        effective_command["displayIsolation"],
        "private_virtual_display"
    );
    assert_eq!(metadata.profile_id.as_deref(), Some("stealthcdp-default"));
    assert_eq!(metadata.view_streams.len(), 1);
    assert_eq!(
        metadata.view_streams[0].provider,
        ViewStreamProvider::RdpGateway
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_manifest_executable_path_does_not_block_capability_selection() {
    let command = json!(
        { "action" : "launch", "executablePath" : "/opt/chromium-stealth/chrome",
        "executablePathSource" : "manifest" }
    );
    assert!(!executable_path_is_operator_supplied(
        Some("/opt/chromium-stealth/chrome"),
        &command
    ));
    assert!(executable_path_is_operator_supplied(
        Some("/opt/chromium-stealth/chrome"),
        &json!({ "action" : "launch", "executablePath" : "/opt/chromium-stealth/chrome",
        "executablePathSource" : "config" })
    ));
}
#[test]
fn test_apply_auto_launch_command_hints_preserves_retained_remote_headed_surface() {
    let retained = RetainedRemoteHeadedLaunchHint {
        view_streams: vec![ViewStream {
            id: "remote-headed-view".to_string(),
            provider: ViewStreamProvider::RdpGateway,
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            url: Some("/guacamole/#/client/MQBjAHBvc3RncmVzcWw=".to_string()),
            frame_url: Some("/guacamole/#/client/MQBjAHBvc3RncmVzcWw=".to_string()),
            external_url: Some("/guacamole/#/client/MQBjAHBvc3RncmVzcWw=".to_string()),
            route_descriptor: None,
            route_id: None,
            display_allocation_id: None,
            connection_id: Some("MQBjAHBvc3RncmVzcWw=".to_string()),
            connection_name: None,
            route_source: Some("test_fixture".to_string()),
            provider_mode: None,
            viewer_lease_ids: Vec::new(),
            controller_lease_id: None,
            read_only: false,
            readiness: None,
            remote_readiness: None,
            attachability: None,
        }],
        display_isolation: Some("shared_display".to_string()),
        display_name: Some(":10".to_string()),
    };
    let command = json!(
        { "action" : "launch", "headless" : true, "runtimeProfile" : "stealthcdp-default"
        }
    );
    let mut options = LaunchOptions::default();
    assert!(!command_has_explicit_launch_surface(&command));
    assert!(command_has_explicit_launch_surface(
        &json!({ "action" : "launch", "headless" :
        true, "headlessExplicit" : true })
    ));
    let (host, selection_reason, _, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, Some(&retained));
    let mut metadata = ServiceLaunchMetadata::from_launch_options(
        &options,
        Some(&effective_command),
        selection_reason,
    );
    apply_retained_remote_headed_metadata(&mut metadata, Some(&retained));
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert!(!options.headless);
    assert!(options.remote_headed);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("shared_display")
    );
    assert_eq!(options.display.as_deref(), Some(":10"));
    assert_eq!(metadata.view_streams, retained.view_streams);
    assert_eq!(
        metadata.display_isolation.as_deref(),
        Some("shared_display")
    );
    assert_eq!(metadata.display_name.as_deref(), Some(":10"));
}
#[test]
fn test_explicit_local_headless_launch_surface_overrides_retained_remote_hint() {
    let retained = RetainedRemoteHeadedLaunchHint {
        view_streams: vec![ViewStream {
            id: "remote-headed-view".to_string(),
            provider: ViewStreamProvider::RdpGateway,
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            url: None,
            frame_url: None,
            external_url: None,
            route_descriptor: None,
            route_id: None,
            display_allocation_id: None,
            connection_id: None,
            connection_name: None,
            route_source: None,
            provider_mode: None,
            viewer_lease_ids: Vec::new(),
            controller_lease_id: None,
            read_only: false,
            readiness: None,
            remote_readiness: None,
            attachability: None,
        }],
        display_isolation: Some("shared_display".to_string()),
        display_name: Some(":10".to_string()),
    };
    let command = json!(
        { "action" : "launch", "browserHost" : "local_headless", "headless" : true,
        "headlessExplicit" : true }
    );
    let mut options = LaunchOptions::default();
    let (host, _, _, _) = apply_auto_launch_command_hints(&mut options, &command, Some(&retained));
    assert_eq!(host, ServiceBrowserHost::LocalHeadless);
    assert!(options.headless);
    assert!(!options.remote_headed);
    assert!(options.remote_headed_display_isolation.is_none());
}
#[test]
fn test_private_remote_headed_metadata_waits_for_launched_display_name() {
    let guard = EnvGuard::new(&["DISPLAY"]);
    guard.set("DISPLAY", ":0");
    let command = json!(
        { "action" : "navigate", "browserHost" : "remote_headed", "displayIsolation" :
        "private_virtual_display", "headless" : false }
    );
    let mut options = LaunchOptions::default();
    let (host, selection_reason, _, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    let metadata = ServiceLaunchMetadata::from_launch_options(
        &options,
        Some(&effective_command),
        selection_reason,
    );
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert_eq!(
        metadata.display_isolation.as_deref(),
        Some("private_virtual_display")
    );
    assert_eq!(metadata.display_name, None);
}
#[test]
fn test_remote_headed_defaults_to_private_display_when_display_is_inherited() {
    let guard = EnvGuard::new(&["DISPLAY"]);
    guard.set("DISPLAY", ":0");
    let command = json!(
        { "action" : "navigate", "browserHost" : "remote_headed", "headless" : false }
    );
    let mut options = LaunchOptions::default();
    let (host, selection_reason, _, effective_command) =
        apply_auto_launch_command_hints(&mut options, &command, None);
    let metadata = ServiceLaunchMetadata::from_launch_options(
        &options,
        Some(&effective_command),
        selection_reason,
    );
    assert_eq!(host, ServiceBrowserHost::RemoteHeaded);
    assert_eq!(
        options.remote_headed_display_isolation.as_deref(),
        Some("private_virtual_display")
    );
    assert_eq!(
        metadata.display_isolation.as_deref(),
        Some("private_virtual_display")
    );
    assert_eq!(metadata.display_name, None);
}
#[test]
fn test_browser_capability_preference_guide_builds_copyable_command() {
    let service_state = ServiceState {
        browser_capability_registry: BrowserCapabilityRegistry {
            browser_executables: vec![
                json!({ "id" : "windows-chrome-stable", "hostId" : "windows-desktop-1",
                "buildLabel" : "stock_chrome", "executablePath" :
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", "source" :
                "system", "fresh" : true }),
            ],
            browser_capabilities: vec![json!({ "id" : "windows-chrome-capability", "hostId" :
                "windows-desktop-1", "executableId" : "windows-chrome-stable" })],
            browser_preference_bindings: vec![
                json!({ "id" : "existing-chrome-binding", "preferredExecutableId" :
                "windows-chrome-stable" }),
            ],
            ..BrowserCapabilityRegistry::default()
        },
        ..ServiceState::default()
    };
    let guide = browser_capability_preference_guide(
        &service_state,
        &json!(
            { "browserBuild" : "stock_chrome", "targetServiceId" :
            "only-works-on-chrome", "accountId" : "my user", "reason" :
            "site requires stock chrome" }
        ),
    );
    assert_eq!(guide["copyable"], true);
    assert_eq!(guide["counts"]["matchingExecutables"], 1);
    assert_eq!(
        guide["suggestions"][0]["executableId"],
        "windows-chrome-stable"
    );
    assert_eq!(
        guide["suggestions"][0]["existingBindingIds"],
        json!(["existing-chrome-binding"])
    );
    assert_eq!(
        guide["suggestions"] [0] ["command"],
        "agent-browser service browser-capability prefer --browser-build stock_chrome --preferred-executable-id windows-chrome-stable --preferred-host-id windows-desktop-1 --preferred-capability-id windows-chrome-capability --target-service-id only-works-on-chrome --account-id 'my user' --reason 'site requires stock chrome'"
    );
}
#[test]
fn test_apply_service_browser_capability_selection_sets_validated_executable() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_EXECUTABLE_PATH"]);
    let home = unique_socket_dir("browser-capability-launch-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let executable = home.join("chrome");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![
                    json!({ "id" : "linux-local", "hostKind" : "local", "reachable" :
                        true, "lifecycleOwner" : "agent_browser" }),
                ],
                browser_executables: vec![
                    json!({ "id" : "stealth-current", "hostId" : "linux-local",
                        "buildLabel" : "stealthcdp_chromium", "executablePath" :
                        executable.display().to_string() }),
                ],
                browser_capabilities: vec![
                    json!({ "id" : "stealth-capability", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "cdpSupported" : true,
                        "cdpFreeLaunchSupported" : false, "headedSupported" : true,
                        "headlessSupported" : true }),
                ],
                profile_compatibility: vec![
                    json!({ "id" : "stealth-profile-compatible", "profileId" :
                        "stealth-profile", "hostId" : "linux-local", "executableId" :
                        "stealth-current", "compatible" : true,
                        "requiresOperatorOverride" : false }),
                ],
                browser_preference_bindings: vec![
                    json!({ "id" : "canary-stealth-default", "scope" : "site",
                        "targetServiceIds" : ["canary-site"], "preferredHostId" :
                        "linux-local", "preferredExecutableId" : "stealth-current",
                        "preferredCapabilityId" : "stealth-capability", "browserBuild" :
                        "stealthcdp_chromium", "priority" : 50 }),
                ],
                validation_evidence: vec![
                    json!({ "id" : "stealth-launch-smoke", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "capabilityId" :
                        "stealth-capability", "kind" : "launch", "state" : "passed" }),
                ],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let mut options = LaunchOptions {
        runtime_profile: Some("stealth-profile".to_string()),
        manual_login: true,
        remote_headed: true,
        ..LaunchOptions::default()
    };
    let resolution = apply_service_browser_capability_selection(
        &mut options,
        &json!(
            { "serviceName" : "CanaryRunner", "targetServiceId" : "canary-site",
            "browserBuild" : "stealthcdp_chromium" }
        ),
    );
    assert!(resolution.applied);
    assert_eq!(resolution.reason, "validated_binding_applied");
    let selection = resolution
        .selection
        .as_ref()
        .expect("validated local binding should be selected");
    assert_eq!(selection.binding_id, "canary-stealth-default");
    assert_eq!(selection.executable_id, "stealth-current");
    assert_eq!(resolution.to_value()["applied"], true);
    assert_eq!(resolution.to_value()["bindingId"], "canary-stealth-default");
    assert_eq!(
        resolution.to_value()["profileCompatibilityIds"],
        json!(["stealth-profile-compatible"])
    );
    assert_eq!(
        resolution.to_value()["validationEvidenceIds"],
        json!(["stealth-launch-smoke"])
    );
    assert_eq!(
        options.executable_path.as_deref(),
        Some(executable.to_str().expect("path should be utf-8"))
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_browser_capability_preflight_reports_validated_binding_without_launch() {
    let guard = EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_EXECUTABLE_PATH",
        "AGENT_BROWSER_EXECUTABLE_PATH_SOURCE",
    ]);
    let home = unique_socket_dir("browser-capability-preflight-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let executable = home.join("chrome");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    let manifest_default = home.join("manifest-default-chrome");
    fs::write(&manifest_default, "#!/bin/sh\n")
        .expect("manifest default executable should be written");
    guard.set(
        "AGENT_BROWSER_EXECUTABLE_PATH",
        manifest_default
            .to_str()
            .expect("manifest default path should be utf-8"),
    );
    guard.set("AGENT_BROWSER_EXECUTABLE_PATH_SOURCE", "manifest");
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![
                    json!({ "id" : "linux-local", "hostKind" : "local", "reachable" :
                        true, "lifecycleOwner" : "agent_browser" }),
                ],
                browser_executables: vec![
                    json!({ "id" : "stealth-current", "hostId" : "linux-local",
                        "buildLabel" : "stealthcdp_chromium", "executablePath" :
                        executable.display().to_string() }),
                ],
                browser_capabilities: vec![
                    json!({ "id" : "stealth-capability", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "cdpSupported" : true,
                        "headedSupported" : true, "headlessSupported" : true }),
                ],
                profile_compatibility: vec![
                    json!({ "id" : "stealth-profile-compatible", "profileId" :
                        "stealth-profile", "hostId" : "linux-local", "executableId" :
                        "stealth-current", "compatible" : true,
                        "requiresOperatorOverride" : false }),
                ],
                browser_preference_bindings: vec![
                    json!({ "id" : "canary-stealth-default", "scope" : "site",
                        "targetServiceIds" : ["canary-site"], "preferredHostId" :
                        "linux-local", "preferredExecutableId" : "stealth-current",
                        "preferredCapabilityId" : "stealth-capability", "browserBuild" :
                        "stealthcdp_chromium", "priority" : 50 }),
                ],
                validation_evidence: vec![
                    json!({ "id" : "stealth-launch-smoke", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "capabilityId" :
                        "stealth-capability", "kind" : "launch", "state" : "passed" }),
                ],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let response = handle_service_browser_capability_preflight(&json!(
        { "targetServiceId" : "canary-site", "runtimeProfile" :
        "stealth-profile", "browserBuild" : "stealthcdp_chromium", "headless" :
        false }
    ))
    .await
    .expect("preflight should evaluate");
    assert_eq!(response["preflight"], true);
    assert_eq!(response["wouldLaunch"], false);
    assert_eq!(response["wouldApplyExecutable"], true);
    assert_eq!(
        response["browserCapabilityLaunch"]["reason"],
        "validated_binding_applied"
    );
    assert_eq!(
        response["selectedExecutablePath"],
        executable.to_str().expect("path should be utf-8")
    );
    assert_eq!(
        response["browserCapabilityLaunch"]["profileCompatibilityIds"],
        json!(["stealth-profile-compatible"])
    );
    assert_eq!(
        response["browserCapabilityLaunch"]["validationEvidenceIds"],
        json!(["stealth-launch-smoke"])
    );
    assert_eq!(response["request"]["profileId"], "stealth-profile");
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_browser_preference_binding_requires_all_identity_filters_for_launch() {
    let binding = json!(
        { "id" : "only-works-on-chrome-myuser-primary", "scope" : "account",
        "targetServiceIds" : ["only-works-on-chrome"], "accountIds" : ["myuser"],
        "browserBuild" : "stock_chrome" }
    );
    assert!(!preference_binding_matches_launch_command(
        &binding,
        &json!({
        "targetServiceId" : "only-works-on-chrome", "browserBuild" : "stock_chrome" }),
        Some("stock_chrome")
    ));
    assert!(!preference_binding_matches_launch_command(
        &binding,
        &json!({ "accountId" :
        "myuser", "browserBuild" : "stock_chrome" }),
        Some("stock_chrome")
    ));
    assert!(preference_binding_matches_launch_command(
        &binding,
        &json!({ "targetServiceId"
        : "only-works-on-chrome", "accountId" : "myuser", "browserBuild" : "stock_chrome"
        }),
        Some("stock_chrome")
    ));
    assert!(preference_binding_matches_launch_command(
        &json!({ "id" :
        "default-new-identities-use-stealthcdp", "scope" : "global", "browserBuild" :
        "stealthcdp_chromium" }),
        &json!({ "targetServiceId" : "any-site",
        "browserBuild" : "stealthcdp_chromium" }),
        Some("stealthcdp_chromium")
    ));
}
#[tokio::test]
async fn test_service_access_plan_reports_browser_build_summary_without_launch() {
    let response = handle_service_access_plan(&json!(
        { "serviceName" : "CanvaCLI", "agentName" : "codex", "taskName" :
        "openCanvaWorkspace", "loginId" : "canary-site", "browserBuild" :
        "stealthcdp_chromium", "browserHost" : "remote_headed",
        "viewStreamProvider" : "rdp_gateway", "controlInputProvider" :
        "manual_attached_desktop", "displayIsolation" : "private_virtual_display"
        }
    ))
    .await
    .expect("access plan should evaluate");
    assert_eq!(response["query"]["serviceName"], "CanvaCLI");
    assert_eq!(response["query"]["browserHost"], "remote_headed");
    assert_eq!(response["query"]["viewStreamProvider"], "rdp_gateway");
    assert_eq!(
        response["query"]["controlInputProvider"],
        "manual_attached_desktop"
    );
    assert_eq!(
        response["query"]["displayIsolation"],
        "private_virtual_display"
    );
    assert!(response["decision"].is_object());
    assert_eq!(response["decision"]["launchPosture"]["source"], "request");
    assert_eq!(
        response["decision"]["profileReuse"]["recommendedAction"],
        "register_or_select_profile"
    );
    assert_eq!(
        response["browserBuildSelectionSummary"]["browserBuild"],
        "stealthcdp_chromium"
    );
    assert!(response["browserBuildSelectionSummary"]["compact"]
        .as_str()
        .expect("compact summary should be present")
        .contains("build=stealthcdp_chromium"));
}
#[test]
fn test_apply_service_browser_capability_selection_requires_compatibility() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_EXECUTABLE_PATH"]);
    let home = unique_socket_dir("browser-capability-incompatible-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let executable = home.join("chrome");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![
                    json!({ "id" : "linux-local", "hostKind" : "local", "reachable" :
                        true, "lifecycleOwner" : "agent_browser" }),
                ],
                browser_executables: vec![
                    json!({ "id" : "stealth-current", "hostId" : "linux-local",
                        "buildLabel" : "stealthcdp_chromium", "executablePath" :
                        executable.display().to_string() }),
                ],
                browser_capabilities: vec![
                    json!({ "id" : "stealth-capability", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "cdpSupported" : true,
                        "headedSupported" : true, "headlessSupported" : true }),
                ],
                profile_compatibility: vec![
                    json!({ "id" : "chrome-profile-incompatible", "profileId" :
                        "chrome-profile", "hostId" : "linux-local", "executableId" :
                        "stealth-current", "compatible" : false,
                        "requiresOperatorOverride" : true }),
                ],
                browser_preference_bindings: vec![
                    json!({ "id" : "canary-stealth-default", "scope" : "site",
                        "targetServiceIds" : ["canary-site"], "preferredHostId" :
                        "linux-local", "preferredExecutableId" : "stealth-current",
                        "preferredCapabilityId" : "stealth-capability", "browserBuild" :
                        "stealthcdp_chromium", "priority" : 50 }),
                ],
                validation_evidence: vec![
                    json!({ "id" : "stealth-launch-smoke", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "capabilityId" :
                        "stealth-capability", "kind" : "launch", "state" : "passed" }),
                ],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let mut options = LaunchOptions {
        runtime_profile: Some("chrome-profile".to_string()),
        ..LaunchOptions::default()
    };
    let resolution = apply_service_browser_capability_selection(
        &mut options,
        &json!(
            { "targetServiceId" : "canary-site", "browserBuild" : "stealthcdp_chromium" }
        ),
    );
    assert!(!resolution.applied);
    assert_eq!(
        resolution.reason,
        "profile_compatibility_missing_or_blocked"
    );
    assert_eq!(resolution.to_value()["applied"], false);
    assert_eq!(
        resolution.to_value()["reason"],
        "profile_compatibility_missing_or_blocked"
    );
    assert!(options.executable_path.is_none());
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_apply_service_browser_capability_selection_rejects_mixed_validation() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_EXECUTABLE_PATH"]);
    let home = unique_socket_dir("browser-capability-mixed-validation-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let executable = home.join("chrome");
    fs::write(&executable, "#!/bin/sh\n").expect("test executable should be written");
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            browser_capability_registry: BrowserCapabilityRegistry {
                browser_hosts: vec![
                    json!({ "id" : "linux-local", "hostKind" : "local", "reachable" :
                        true, "lifecycleOwner" : "agent_browser" }),
                ],
                browser_executables: vec![
                    json!({ "id" : "stealth-current", "hostId" : "linux-local",
                        "buildLabel" : "stealthcdp_chromium", "executablePath" :
                        executable.display().to_string() }),
                ],
                browser_capabilities: vec![
                    json!({ "id" : "stealth-capability", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "cdpSupported" : true,
                        "headedSupported" : true, "headlessSupported" : true }),
                ],
                profile_compatibility: vec![
                    json!({ "id" : "stealth-profile-compatible", "profileId" :
                        "stealth-profile", "hostId" : "linux-local", "executableId" :
                        "stealth-current", "compatible" : true,
                        "requiresOperatorOverride" : false }),
                ],
                browser_preference_bindings: vec![
                    json!({ "id" : "canary-stealth-default", "scope" : "site",
                        "targetServiceIds" : ["canary-site"], "preferredHostId" :
                        "linux-local", "preferredExecutableId" : "stealth-current",
                        "preferredCapabilityId" : "stealth-capability", "browserBuild" :
                        "stealthcdp_chromium", "priority" : 50 }),
                ],
                validation_evidence: vec![
                    json!({ "id" : "stealth-launch-smoke", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "capabilityId" :
                        "stealth-capability", "kind" : "launch", "state" : "passed" }),
                    json!({ "id" : "stealth-launch-stale", "hostId" : "linux-local",
                        "executableId" : "stealth-current", "capabilityId" :
                        "stealth-capability", "kind" : "launch", "state" : "stale" }),
                ],
                ..BrowserCapabilityRegistry::default()
            },
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let mut options = LaunchOptions {
        runtime_profile: Some("stealth-profile".to_string()),
        ..LaunchOptions::default()
    };
    let resolution = apply_service_browser_capability_selection(
        &mut options,
        &json!(
            { "targetServiceId" : "canary-site", "browserBuild" : "stealthcdp_chromium" }
        ),
    );
    assert!(!resolution.applied);
    assert_eq!(
        resolution.reason,
        "validation_evidence_missing_or_not_passed"
    );
    assert!(options.executable_path.is_none());
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_cdp_free_launch_plan_is_no_devtools_headed_lifecycle_only() {
    let cmd = json!(
        { "serviceName" : "CanvaCLI", "agentName" : "canva-cli-agent", "taskName" :
        "openCanvaWorkspace", "targetServiceId" : "canva", "runtimeProfile" :
        "canva-default", "url" : "https://www.canva.com/", "args" :
        ["--window-size=960,720"], "requiresCdpFree" : true, "cdpAttachmentAllowed" :
        false }
    );
    let plan = build_cdp_free_launch_plan(&cmd).expect("plan should parse without launching");
    assert!(!plan.launch_options.headless);
    assert!(!plan.launch_options.attachable);
    assert!(plan.launch_options.manual_login);
    assert_eq!(
        plan.launch_options.runtime_profile.as_deref(),
        Some("canva-default")
    );
    assert_eq!(plan.url.as_deref(), Some("https://www.canva.com/"));
    assert_eq!(
        plan.launch_options.args,
        vec![
            "--window-size=960,720".to_string(),
            "https://www.canva.com/".to_string()
        ]
    );
    assert_eq!(plan.metadata.profile_id.as_deref(), Some("canva-default"));
    assert_eq!(plan.metadata.service_name.as_deref(), Some("CanvaCLI"));
    assert_eq!(plan.metadata.agent_name.as_deref(), Some("canva-cli-agent"));
    assert_eq!(
        plan.metadata.task_name.as_deref(),
        Some("openCanvaWorkspace")
    );
    assert!(plan.metadata.persistent_profile);
}
#[test]
fn test_cdp_free_launch_response_reports_unsupported_cdp_operations() {
    let mut state = DaemonState::new();
    state.session_id = "cdp-free-session".to_string();
    let launch_options = LaunchOptions {
        runtime_profile: Some("canva-default".to_string()),
        headless: false,
        manual_login: true,
        attachable: false,
        ..LaunchOptions::default()
    };
    let launch = ManualChromeLaunch {
        pid: 4242,
        user_data_dir: PathBuf::from("/tmp/canva-default"),
        runtime_profile: Some("canva-default".to_string()),
        devtools_port: None,
    };
    let response = cdp_free_launch_response(
        &state,
        &launch_options,
        &launch,
        Some("https://www.canva.com/".to_string()),
    );
    assert_eq!(response["launched"], true);
    assert_eq!(response["cdpFree"], true);
    assert_eq!(response["cdpAttachmentAllowed"], false);
    assert_eq!(response["browserId"], "session:cdp-free-session");
    assert_eq!(response["browserPid"], 4242);
    assert_eq!(response["profileId"], "canva-default");
    assert_eq!(response["runtimeProfile"], "canva-default");
    assert_eq!(response["url"], "https://www.canva.com/");
    assert_eq!(response["supportedOperations"][0], "process_lifecycle");
    assert!(response["unsupportedOperations"]
        .as_array()
        .expect("unsupported operations should be an array")
        .iter()
        .any(|operation| operation == "cdp_commands"));
    assert!(response["unsupportedCommands"]
        .as_array()
        .expect("unsupported commands should be an array")
        .iter()
        .any(|command| command == "snapshot"));
    assert!(response["unsupportedCommands"]
        .as_array()
        .expect("unsupported commands should be an array")
        .iter()
        .any(|command| command == "click"));
    assert!(launch.devtools_port.is_none());
}
#[test]
fn test_cdp_free_launch_plan_rejects_dash_prefixed_url() {
    let result = build_cdp_free_launch_plan(
        &json!({ "action" : "cdp_free_launch", "url" : "--remote-debugging-port=9222" }),
    );
    let err = match result {
        Ok(_) => panic!("dash-prefixed url should be rejected"),
        Err(err) => err,
    };
    assert!(err.contains("url must not start"));
}
#[test]
fn test_service_profile_lease_guard_rejects_conflicting_service_launch() {
    let mut service_state = ServiceState::default();
    service_state.sessions.insert(
        "active-session".to_string(),
        BrowserSession {
            id: "active-session".to_string(),
            profile_id: Some("acs-profile".to_string()),
            lease: LeaseState::Exclusive,
            ..BrowserSession::default()
        },
    );
    let metadata = ServiceLaunchMetadata {
        profile_id: Some("acs-profile".to_string()),
        service_name: Some("JournalDownloader".to_string()),
        ..ServiceLaunchMetadata::default()
    };
    let conflict_session_ids = service_profile_lease_conflict_session_ids_in_state(
        &service_state,
        &metadata,
        "new-session",
        "acs-profile",
    )
    .join(", ");
    assert_eq!(conflict_session_ids, "active-session");
}
#[test]
fn test_service_profile_lease_guard_allows_same_session_reuse() {
    let service_state = ServiceState {
        sessions: BTreeMap::from([(
            "active-session".to_string(),
            BrowserSession {
                id: "active-session".to_string(),
                profile_id: Some("acs-profile".to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        )]),
        browsers: BTreeMap::from([(
            "session:active-session".to_string(),
            BrowserProcess {
                id: "session:active-session".to_string(),
                profile_id: Some("acs-profile".to_string()),
                active_session_ids: vec!["active-session".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        ..ServiceState::default()
    };
    let metadata = ServiceLaunchMetadata {
        profile_id: Some("acs-profile".to_string()),
        service_name: Some("JournalDownloader".to_string()),
        ..ServiceLaunchMetadata::default()
    };
    let conflict_session_ids = service_profile_lease_conflict_session_ids_in_state(
        &service_state,
        &metadata,
        "active-session",
        "acs-profile",
    );
    assert!(conflict_session_ids.is_empty());
}
#[test]
fn test_cdp_screencast_view_stream_ready_for_non_remote_cdp_browser() {
    let stream = cdp_screencast_view_stream(
        "stream-session",
        ServiceBrowserHost::LocalHeadless,
        ServiceBrowserHealth::Ready,
        Some("http://127.0.0.1:9222"),
        Some(44841),
    )
    .expect("local CDP browser should advertise a CDP stream");
    assert_eq!(stream.id, "cdp-screencast");
    assert_eq!(stream.provider, ViewStreamProvider::CdpScreencast);
    assert_eq!(stream.control_input, Some(ControlInputProvider::CdpInput));
    assert_eq!(stream.url.as_deref(), Some("http://127.0.0.1:44841/"));
    assert_eq!(stream.frame_url.as_deref(), Some("http://127.0.0.1:44841/"));
    assert!(!stream.read_only);
    assert_eq!(
        stream.readiness.as_ref().unwrap()["reason"],
        "stream_server_ready"
    );
}
#[test]
fn test_cdp_screencast_view_stream_reports_unavailable_without_stream_server() {
    let stream = cdp_screencast_view_stream(
        "stream-session",
        ServiceBrowserHost::AttachedExisting,
        ServiceBrowserHealth::Ready,
        Some("http://127.0.0.1:9222"),
        None,
    )
    .expect("attached CDP browser should retain unavailable stream readiness");
    assert!(stream.url.is_none());
    assert!(stream.frame_url.is_none());
    assert_eq!(stream.control_input, None);
    assert!(stream.read_only);
    assert_eq!(
        stream.readiness.as_ref().unwrap()["reason"],
        "missing_stream_server"
    );
}
#[test]
fn test_cdp_screencast_view_stream_leaves_remote_headed_contract_unchanged() {
    assert!(cdp_screencast_view_stream(
        "remote-session",
        ServiceBrowserHost::RemoteHeaded,
        ServiceBrowserHealth::Ready,
        Some("http://127.0.0.1:9222"),
        Some(44841),
    )
    .is_none());
}
#[test]
fn test_profile_lease_policy_rejects_invalid_value() {
    let err =
        profile_lease_policy_from_command(&json!({ "profileLeasePolicy" : "maybe" })).unwrap_err();
    assert!(err.contains("profileLeasePolicy must be"));
}
#[test]
fn test_profile_lease_wait_timeout_requires_positive_integer() {
    let err =
        profile_lease_wait_timeout_ms_from_command(&json!({ "profileLeaseWaitTimeoutMs" : 0 }))
            .unwrap_err();
    assert!(err.contains("profileLeaseWaitTimeoutMs must be a positive integer"));
}
#[test]
fn test_service_profile_lease_gate_wait_policy_reports_wait() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("profile-lease-wait-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
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
    let decision = service_profile_lease_gate(
        &json!(
            { "serviceName" : "JournalDownloader", "runtimeProfile" : "acs-profile",
            "profileLeasePolicy" : "wait", "profileLeaseWaitTimeoutMs" : 2_000 }
        ),
        "new-session",
        Some(0),
    )
    .expect("lease gate should evaluate");
    match decision {
        ServiceProfileLeaseGate::Wait {
            retry_after_ms,
            profile_id,
            conflict_session_ids,
        } => {
            assert_eq!(retry_after_ms, PROFILE_LEASE_WAIT_POLL_MS);
            assert_eq!(profile_id, "acs-profile");
            assert_eq!(conflict_session_ids, vec!["active-session".to_string()]);
        }
        other => panic!("expected wait decision, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_service_profile_lease_gate_blocks_duplicate_live_profile_lane() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("profile-lane-duplicate-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                "browser-existing".to_string(),
                BrowserProcess {
                    id: "browser-existing".to_string(),
                    profile_id: Some("acs-profile".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["existing-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let decision = service_profile_lease_gate(
        &json!(
            { "serviceName" : "JournalDownloader", "runtimeProfile" : "acs-profile",
            "profileLeasePolicy" : "wait", "profileLeaseWaitTimeoutMs" : 2_000 }
        ),
        "new-session",
        Some(0),
    )
    .expect("lane gate should evaluate");
    match decision {
        ServiceProfileLeaseGate::Reject { error } => {
            assert!(error.contains("Duplicate service profile lane blocked"));
            assert!(error.contains("browser-existing"));
            assert!(error.contains("allowDuplicateProfileLane=true"));
        }
        other => panic!("expected duplicate lane rejection, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_shared_profile_attach_target_selects_compatible_retained_browser() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("shared-profile-attach-target-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                "browser-existing".to_string(),
                BrowserProcess {
                    id: "browser-existing".to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    host: ServiceBrowserHost::RemoteHeaded,
                    health: ServiceBrowserHealth::Ready,
                    display_isolation: Some("private_virtual_display".to_string()),
                    pid: Some(42),
                    cdp_endpoint: Some("http://127.0.0.1:9222".to_string()),
                    active_session_ids: vec!["facebook-operator".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let metadata = ServiceLaunchMetadata {
        profile_id: Some("last30days-facebook".to_string()),
        ..ServiceLaunchMetadata::default()
    };
    let target = shared_profile_attach_target_for_auto_launch(
        &metadata,
        &json!(
            { "action" : "navigate", "url" : "https://x.com/home", "runtimeProfile" :
            "last30days-facebook", "browserHost" : "remote_headed",
            "displayIsolation" : "private_virtual_display" }
        ),
        "x-login-check",
    )
    .expect("compatible retained browser should be selected");
    assert_eq!(target.browser_id, "browser-existing");
    assert_eq!(target.runtime_profile, "last30days-facebook");
    assert_eq!(target.cdp_endpoint, "http://127.0.0.1:9222");
    assert_eq!(target.browser_pid, Some(42));
    assert_eq!(
        target.owner_session_ids,
        vec!["facebook-operator".to_string()]
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_retained_session_attach_target_reconnects_registered_tab_list_client() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("retained-session-attach-target-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([
                (
                    "session:auracall-corel".to_string(),
                    BrowserProcess {
                        id: "session:auracall-corel".to_string(),
                        profile_id: Some("default".to_string()),
                        health: ServiceBrowserHealth::Ready,
                        cdp_endpoint: Some(
                            "ws://127.0.0.1:45015/devtools/browser/default".to_string(),
                        ),
                        active_session_ids: vec!["auracall-corel".to_string()],
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "session:last30days-facebook".to_string(),
                    BrowserProcess {
                        id: "session:last30days-facebook".to_string(),
                        profile_id: Some("last30days-facebook".to_string()),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_isolation: Some("shared_display".to_string()),
                        pid: Some(42),
                        cdp_endpoint: Some(
                            "ws://127.0.0.1:36753/devtools/browser/social".to_string(),
                        ),
                        active_session_ids: vec!["last30days-facebook".to_string()],
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let target = retained_session_attach_target_for_auto_launch(
        &json!({ "action" : "tab_list" }),
        "last30days-facebook",
    )
    .expect("registered session should reconnect to its retained browser");
    assert_eq!(target.browser_id, "session:last30days-facebook");
    assert_eq!(target.runtime_profile, "last30days-facebook");
    assert_eq!(
        target.cdp_endpoint,
        "ws://127.0.0.1:36753/devtools/browser/social"
    );
    assert_eq!(target.browser_pid, Some(42));
    assert_eq!(
        target.owner_session_ids,
        vec!["last30days-facebook".to_string()]
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_retained_session_attach_target_does_not_cross_session_ownership() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("retained-session-cross-owner-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                "session:last30days-facebook".to_string(),
                BrowserProcess {
                    id: "session:last30days-facebook".to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    cdp_endpoint: Some("ws://127.0.0.1:36753/devtools/browser/social".to_string()),
                    active_session_ids: vec!["last30days-facebook".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    assert!(retained_session_attach_target_for_auto_launch(
        &json!({ "action" : "tab_list",
        "browserId" : "session:last30days-facebook", "sessionName" :
        "last30days-facebook" }),
        "unrelated-client",
    )
    .is_none());
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_launch_options_service_profile_id_treats_named_profile_as_runtime_profile() {
    let named = LaunchOptions {
        profile: Some("stealthcdp-default".to_string()),
        ..LaunchOptions::default()
    };
    assert_eq!(
        launch_options_service_profile_id(&named).as_deref(),
        Some("stealthcdp-default")
    );
    let path = LaunchOptions {
        profile: Some("/tmp/agent-browser-smoke-profile".to_string()),
        ..LaunchOptions::default()
    };
    let profile_id =
        launch_options_service_profile_id(&path).expect("path profile should have identity");
    assert!(profile_id.starts_with("custom:"));
}
#[test]
fn test_shared_profile_attach_target_reuses_current_session_owner() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("shared-profile-current-owner-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([
                (
                    "browser-other".to_string(),
                    BrowserProcess {
                        id: "browser-other".to_string(),
                        profile_id: Some("custom:route-viewer-a".to_string()),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_isolation: Some("private_virtual_display".to_string()),
                        pid: Some(41),
                        cdp_endpoint: Some("http://127.0.0.1:9221".to_string()),
                        active_session_ids: vec!["other-route-viewer".to_string()],
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "session:rdp-guac-route-a-viewer".to_string(),
                    BrowserProcess {
                        id: "session:rdp-guac-route-a-viewer".to_string(),
                        profile_id: Some("custom:route-viewer-a".to_string()),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_isolation: Some("private_virtual_display".to_string()),
                        pid: Some(42),
                        cdp_endpoint: Some("http://127.0.0.1:9222".to_string()),
                        active_session_ids: vec!["rdp-guac-route-a-viewer".to_string()],
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let metadata = ServiceLaunchMetadata {
        profile_id: Some("custom:route-viewer-a".to_string()),
        ..ServiceLaunchMetadata::default()
    };
    let target = shared_profile_attach_target_for_auto_launch(
        &metadata,
        &json!(
            { "action" : "open", "url" : "http://127.0.0.1:8092/guacamole/",
            "profile" : home.join("guacamole-route-viewers/a").display().to_string(),
            "browserHost" : "remote_headed", "displayIsolation" :
            "private_virtual_display" }
        ),
        "rdp-guac-route-a-viewer",
    )
    .expect("current session owner should be selected");
    assert_eq!(target.browser_id, "session:rdp-guac-route-a-viewer");
    assert_eq!(target.runtime_profile, "custom:route-viewer-a");
    assert_eq!(target.cdp_endpoint, "http://127.0.0.1:9222");
    assert_eq!(target.browser_pid, Some(42));
    assert_eq!(
        target.owner_session_ids,
        vec!["rdp-guac-route-a-viewer".to_string()]
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_shared_profile_auto_launch_acquisition_reports_plain_open_owner() {
    let command = json!(
        { "action" : "navigate", "url" : "https://x.com/home", "runtimeProfile" :
        "last30days-facebook", "browserHost" : "remote_headed", "displayIsolation" :
        "private_virtual_display" }
    );
    let target = SharedProfileAttachTarget {
        browser_id: "browser-existing".to_string(),
        runtime_profile: "last30days-facebook".to_string(),
        cdp_endpoint: "http://127.0.0.1:9222".to_string(),
        browser_pid: Some(42),
        owner_session_ids: vec!["facebook-operator".to_string()],
    };
    let evidence =
        shared_profile_auto_launch_acquisition_evidence(&command, "x-login-check", &target);
    assert_eq!(evidence["policy"], "shared_browser_tabs");
    assert_eq!(evidence["mode"], "navigate");
    assert_eq!(evidence["action"], "opened_shared_profile_tab");
    assert_eq!(evidence["recommendedAction"], "reuse_existing_browser");
    assert_eq!(evidence["browserReused"], true);
    assert_eq!(evidence["tabOpened"], true);
    assert_eq!(
        evidence["duplicateProcessPolicy"],
        "reject_duplicate_process"
    );
    assert_eq!(evidence["browserId"], "browser-existing");
    assert_eq!(evidence["sessionName"], "facebook-operator");
    assert_eq!(evidence["profileId"], "last30days-facebook");
    assert_eq!(evidence["requestedProfile"], "last30days-facebook");
    assert_eq!(evidence["plannedProfile"], "last30days-facebook");
    assert_eq!(evidence["requiresRouteHints"], true);
    assert_eq!(
        evidence["routeHintFields"],
        json!(["browserId", "sessionName"])
    );
    assert_eq!(evidence["routeHintSource"], "shared_profile_auto_launch");
    assert_eq!(
        evidence["tabAcquisitionDecision"],
        "opened_shared_profile_tab"
    );
}
#[test]
fn test_shared_profile_attach_target_ignores_incompatible_retained_browser() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("shared-profile-incompatible-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                "browser-existing".to_string(),
                BrowserProcess {
                    id: "browser-existing".to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    host: ServiceBrowserHost::LocalHeadless,
                    health: ServiceBrowserHealth::Ready,
                    cdp_endpoint: Some("http://127.0.0.1:9222".to_string()),
                    active_session_ids: vec!["facebook-operator".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let metadata = ServiceLaunchMetadata {
        profile_id: Some("last30days-facebook".to_string()),
        ..ServiceLaunchMetadata::default()
    };
    let target = shared_profile_attach_target_for_auto_launch(
        &metadata,
        &json!(
            { "action" : "navigate", "url" : "https://x.com/home", "runtimeProfile" :
            "last30days-facebook", "browserHost" : "remote_headed" }
        ),
        "x-login-check",
    );
    assert!(target.is_none());
    let decision = service_profile_lease_gate(
        &json!(
            { "serviceName" : "Last30Days", "runtimeProfile" : "last30days-facebook",
            "browserHost" : "remote_headed", "profileLeasePolicy" : "wait",
            "profileLeaseWaitTimeoutMs" : 2_000 }
        ),
        "x-login-check",
        Some(0),
    )
    .expect("lane gate should evaluate");
    assert!(matches!(decision, ServiceProfileLeaseGate::Reject { .. }));
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_service_profile_lease_gate_allows_duplicate_lane_route_hints() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("profile-lane-route-hint-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                "browser-existing".to_string(),
                BrowserProcess {
                    id: "browser-existing".to_string(),
                    profile_id: Some("acs-profile".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["existing-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let decision = service_profile_lease_gate(
        &json!(
            { "serviceName" : "JournalDownloader", "runtimeProfile" : "acs-profile",
            "browserId" : "browser-existing", "sessionName" : "existing-session",
            "profileLeasePolicy" : "wait", "profileLeaseWaitTimeoutMs" : 2_000 }
        ),
        "new-session",
        Some(0),
    )
    .expect("lane gate should evaluate");
    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_service_profile_lease_gate_allows_duplicate_lane_override() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("profile-lane-override-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                "browser-existing".to_string(),
                BrowserProcess {
                    id: "browser-existing".to_string(),
                    profile_id: Some("acs-profile".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["existing-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let decision = service_profile_lease_gate(
        &json!(
            { "serviceName" : "JournalDownloader", "runtimeProfile" : "acs-profile",
            "allowDuplicateProfileLane" : true, "profileLeasePolicy" : "wait",
            "profileLeaseWaitTimeoutMs" : 2_000 }
        ),
        "new-session",
        Some(0),
    )
    .expect("lane gate should evaluate");
    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    let _ = fs::remove_dir_all(&home);
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
#[test]
fn test_runtime_handoff_descriptor_accepts_legacy_schema_v1_without_active_target() {
    let descriptor: RuntimeHandoffDescriptor = serde_json::from_value(json!(
        { "schemaVersion" : 1, "sessionName" : "legacy-session", "cdpUrl" :
        "ws://127.0.0.1:9222/devtools/browser/example", "browserPid" : 42,
        "runtimeProfile" : "legacy-profile", "engine" : "chrome", "host" :
        "attached_existing", "closeBrowserOnClose" : false, "preparedAt" :
        "2026-08-08T12:00:00Z" }
    ))
    .expect("schema-v1 handoff descriptor should remain readable");
    assert_eq!(descriptor.active_target_id, None);
}
#[test]
fn test_managed_runtime_attach_target_uses_runtime_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("managed-runtime-attach-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let runtime_profile = "managed-attach-test";
    let user_data_dir = home.join("managed-user-data");
    fs::create_dir_all(&user_data_dir).expect("user data dir should be created");
    crate::runtime_profile::write_runtime_state(&crate::runtime_profile::RuntimeState {
        runtime_profile: runtime_profile.to_string(),
        user_data_dir: user_data_dir.display().to_string(),
        browser_pid: std::process::id(),
        headed: true,
        launch_mode: "automation".to_string(),
        devtools_port: Some(9333),
        ws_url: Some("ws://127.0.0.1:9333/devtools/browser/test".to_string()),
        launch_record: None,
    })
    .expect("runtime state should be written");
    let target = managed_runtime_attach_target(Some(runtime_profile))
        .expect("live runtime state should produce attach target");
    assert_eq!(target.runtime_profile, runtime_profile);
    assert_eq!(target.browser_pid, std::process::id());
    assert_eq!(target.cdp_port, 9333);
}
#[test]
fn test_managed_runtime_attach_target_reads_devtools_active_port() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("managed-runtime-devtools-file-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let runtime_profile = "managed-devtools-file-test";
    let user_data_dir = home.join("managed-user-data");
    fs::create_dir_all(&user_data_dir).expect("user data dir should be created");
    fs::write(
        user_data_dir.join("DevToolsActivePort"),
        "9444\n/devtools/browser/test",
    )
    .expect("DevToolsActivePort should be written");
    crate::runtime_profile::write_runtime_state(&crate::runtime_profile::RuntimeState {
        runtime_profile: runtime_profile.to_string(),
        user_data_dir: user_data_dir.display().to_string(),
        browser_pid: std::process::id(),
        headed: true,
        launch_mode: "automation".to_string(),
        devtools_port: None,
        ws_url: None,
        launch_record: None,
    })
    .expect("runtime state should be written");
    let target = managed_runtime_attach_target(Some(runtime_profile))
        .expect("DevToolsActivePort should produce attach target");
    assert_eq!(target.runtime_profile, runtime_profile);
    assert_eq!(target.browser_pid, std::process::id());
    assert_eq!(target.cdp_port, 9444);
}
#[test]
fn test_managed_runtime_attach_is_only_for_compatible_headless_launches() {
    let headless = LaunchOptions::default();
    assert!(can_attach_managed_runtime_for_launch(&headless));
    let headed = LaunchOptions {
        headless: false,
        ..LaunchOptions::default()
    };
    assert!(!can_attach_managed_runtime_for_launch(&headed));
    let remote_headed = LaunchOptions {
        headless: false,
        remote_headed: true,
        remote_headed_display_isolation: Some("shared_display".to_string()),
        ..LaunchOptions::default()
    };
    assert!(!can_attach_managed_runtime_for_launch(&remote_headed));
}
#[tokio::test]
async fn test_cancellable_returns_cancelled_error_before_future_completes() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = cancellable(
        async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            Ok::<_, String>(json!({ "completed" : true }))
        },
        Some(cancellation),
    )
    .await;
    assert_eq!(result, Err(cancellation_error()));
}

#[tokio::test]
async fn test_confirm_executes_once_and_restores_confirmation_gate() {
    let mut state = DaemonState::new();
    state.confirm_actions = Some(ConfirmActions {
        categories: HashSet::from(["close".to_string()]),
    });

    let pending = execute_command(&json!({ "id": "close-1", "action": "close" }), &mut state).await;
    assert_eq!(pending["data"]["confirmation_required"], true);
    assert!(state.pending_confirmation.is_some());

    let confirmed = execute_command(
        &json!({ "id": "confirm-1", "action": "confirm" }),
        &mut state,
    )
    .await;
    assert_eq!(confirmed["success"], true);
    assert_eq!(confirmed["data"]["confirmed"], true);
    assert_eq!(confirmed["data"]["action"], "close");
    assert_eq!(confirmed["data"]["result"]["success"], true);
    assert!(state.pending_confirmation.is_none());
    assert!(state
        .confirm_actions
        .as_ref()
        .is_some_and(|actions| actions.requires_confirmation("close")));
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
    let duplicate_err = handle_stream_enable(&json!({}), &mut state)
        .await
        .expect_err("duplicate enable should fail");
    assert!(duplicate_err.contains("already enabled"));
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
async fn test_daemon_state_new() {
    let guard = EnvGuard::new(&[
        "AGENT_BROWSER_ALLOWED_DOMAINS",
        "AGENT_BROWSER_SESSION_NAME",
        "AGENT_BROWSER_SESSION",
    ]);
    guard.remove("AGENT_BROWSER_ALLOWED_DOMAINS");
    guard.remove("AGENT_BROWSER_SESSION_NAME");
    guard.remove("AGENT_BROWSER_SESSION");
    let state = DaemonState::new();
    assert!(state.browser.is_none());
    assert!(state.domain_filter.read().await.is_none());
    assert_eq!(state.session_id, "default");
    assert!(!state.tracing_state.active);
    assert!(!state.recording_state.active);
    assert_eq!(state.mouse_state.x, 0.0);
    assert_eq!(state.mouse_state.y, 0.0);
    assert_eq!(state.mouse_state.buttons, 0);
}
#[test]
fn test_mouse_event_params_preserve_position_and_buttons() {
    let mut mouse_state = MouseState::default();
    let move_params = build_mouse_event_params(
        &mut mouse_state,
        "mouseMoved",
        Some(120.0),
        Some(240.0),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(move_params.x, 120.0);
    assert_eq!(move_params.y, 240.0);
    assert_eq!(move_params.buttons, Some(0));
    let down_params = build_mouse_event_params(
        &mut mouse_state,
        "mousePressed",
        None,
        None,
        Some("left"),
        None,
        Some(1),
        None,
        None,
        None,
    );
    assert_eq!(down_params.x, 120.0);
    assert_eq!(down_params.y, 240.0);
    assert_eq!(down_params.button.as_deref(), Some("left"));
    assert_eq!(down_params.buttons, Some(1));
    assert_eq!(mouse_state.buttons, 1);
    let drag_move_params = build_mouse_event_params(
        &mut mouse_state,
        "mouseMoved",
        Some(150.0),
        Some(260.0),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(drag_move_params.buttons, Some(1));
    assert_eq!(drag_move_params.button.as_deref(), Some("left"));
    assert_eq!(mouse_state.x, 150.0);
    assert_eq!(mouse_state.y, 260.0);
    let up_params = build_mouse_event_params(
        &mut mouse_state,
        "mouseReleased",
        None,
        None,
        Some("left"),
        None,
        Some(1),
        None,
        None,
        None,
    );
    assert_eq!(up_params.x, 150.0);
    assert_eq!(up_params.y, 260.0);
    assert_eq!(up_params.buttons, Some(0));
    assert_eq!(mouse_state.buttons, 0);
}
#[test]
fn test_reset_input_state_clears_mouse_state() {
    let mut state = DaemonState::new();
    state.mouse_state.x = 12.0;
    state.mouse_state.y = 34.0;
    state.mouse_state.buttons = 1;
    state.reset_input_state();
    assert_eq!(state.mouse_state.x, 0.0);
    assert_eq!(state.mouse_state.y, 0.0);
    assert_eq!(state.mouse_state.buttons, 0);
}
#[test]
fn test_launch_options_from_env_defaults() {
    let _guard = EnvGuard::new(&[
        "AGENT_BROWSER_HEADED",
        "AGENT_BROWSER_USE_REAL_KEYCHAIN",
        "AGENT_BROWSER_KEYCHAIN_PASSWORD",
    ]);
    let opts = launch_options_from_env();
    assert!(opts.headless);
    assert!(opts.args.is_empty());
    assert!(!opts.allow_file_access);
    assert!(!opts.use_real_keychain);
    assert!(opts.keychain_password.is_none());
}
#[test]
fn test_launch_options_from_env_headed_flag() {
    let _guard = EnvGuard::new(&[
        "AGENT_BROWSER_HEADED",
        "AGENT_BROWSER_USE_REAL_KEYCHAIN",
        "AGENT_BROWSER_KEYCHAIN_PASSWORD",
    ]);
    _guard.set("AGENT_BROWSER_HEADED", "1");
    let opts = launch_options_from_env();
    assert!(
        !opts.headless,
        "AGENT_BROWSER_HEADED=1 should set headless=false"
    );
}
#[test]
fn test_launch_options_from_env_keychain_password_enables_real_keychain() {
    let _guard = EnvGuard::new(&[
        "AGENT_BROWSER_USE_REAL_KEYCHAIN",
        "AGENT_BROWSER_KEYCHAIN_PASSWORD",
    ]);
    _guard.set("AGENT_BROWSER_KEYCHAIN_PASSWORD", "secret");
    let opts = launch_options_from_env();
    assert!(opts.use_real_keychain);
    assert_eq!(opts.keychain_password.as_deref(), Some("secret"));
}
#[test]
fn test_launch_options_from_env_real_keychain_flag_without_password() {
    let _guard = EnvGuard::new(&[
        "AGENT_BROWSER_USE_REAL_KEYCHAIN",
        "AGENT_BROWSER_KEYCHAIN_PASSWORD",
    ]);
    _guard.set("AGENT_BROWSER_USE_REAL_KEYCHAIN", "1");
    let opts = launch_options_from_env();
    assert!(opts.use_real_keychain);
    assert!(opts.keychain_password.is_none());
}
#[test]
fn test_har_entry_to_json_enriches_request_and_response() {
    let entry = HarEntry {
        request_id: "req-1".to_string(),
        wall_time: 1773576000.0,
        method: "POST".to_string(),
        url: "https://example.com/api?foo=bar&baz=qux".to_string(),
        request_headers: vec![
            ("Accept".to_string(), "application/json".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Cookie".to_string(), "session=abc; theme=dark".to_string()),
        ],
        post_data: Some(r#"{"x":1}"#.to_string()),
        request_body_size: 7,
        resource_type: "XHR".to_string(),
        status: Some(201),
        status_text: "Created".to_string(),
        http_version: "HTTP/2.0".to_string(),
        response_headers: vec![
            ("content-type".to_string(), "application/json".to_string()),
            (
                "location".to_string(),
                "https://example.com/api/1".to_string(),
            ),
            (
                "set-cookie".to_string(),
                "token=xyz; Path=/; HttpOnly".to_string(),
            ),
        ],
        mime_type: "application/json".to_string(),
        redirect_url: "https://example.com/api/1".to_string(),
        response_body_size: 42,
        cdp_timing: None,
        loading_finished_timestamp: None,
    };
    let har = har_entry_to_json(entry);
    assert_eq!(har["startedDateTime"], "2026-03-15T12:00:00Z");
    assert_eq!(har["request"]["method"], "POST");
    assert_eq!(har["request"]["httpVersion"], "HTTP/2.0");
    assert_eq!(har["request"]["queryString"][0]["name"], "foo");
    assert_eq!(har["request"]["queryString"][0]["value"], "bar");
    assert_eq!(har["request"]["bodySize"], 7);
    assert_eq!(har["request"]["postData"]["mimeType"], "application/json");
    assert_eq!(har["request"]["postData"]["text"], r#"{"x":1}"#);
    assert_eq!(har["request"]["cookies"][0]["name"], "session");
    assert_eq!(har["request"]["cookies"][0]["value"], "abc");
    assert_eq!(har["request"]["cookies"][1]["name"], "theme");
    assert_eq!(har["request"]["cookies"][1]["value"], "dark");
    assert_eq!(har["response"]["status"], 201);
    assert_eq!(har["response"]["statusText"], "Created");
    assert_eq!(har["response"]["content"]["mimeType"], "application/json");
    assert_eq!(har["response"]["content"]["size"], 42);
    assert_eq!(har["response"]["redirectURL"], "https://example.com/api/1");
    assert_eq!(har["response"]["cookies"][0]["name"], "token");
    assert_eq!(har["response"]["cookies"][0]["value"], "xyz");
    assert_eq!(har["_resourceType"], "XHR");
}
#[test]
fn test_har_wall_time_to_rfc3339_epoch() {
    let result = har_wall_time_to_rfc3339(1773576000.0);
    assert!(result.starts_with("2026-03-15T12:00:00"));
}
#[test]
fn test_har_wall_time_to_rfc3339_fractional_seconds() {
    let result = har_wall_time_to_rfc3339(1773576000.456);
    assert!(result.contains(".456") || result.contains("456"));
}
#[test]
fn test_har_cdp_protocol_to_http_version() {
    assert_eq!(har_cdp_protocol_to_http_version("h2"), "HTTP/2.0");
    assert_eq!(har_cdp_protocol_to_http_version("h3"), "HTTP/3.0");
    assert_eq!(har_cdp_protocol_to_http_version("http/1.0"), "HTTP/1.0");
    assert_eq!(har_cdp_protocol_to_http_version("http/1.1"), "HTTP/1.1");
    assert_eq!(har_cdp_protocol_to_http_version("unknown"), "HTTP/1.1");
}
#[test]
fn test_har_parse_request_cookies() {
    let cookies = har_parse_request_cookies("session=abc; theme=dark; empty=");
    assert_eq!(cookies.len(), 3);
    assert_eq!(cookies[0]["name"], "session");
    assert_eq!(cookies[0]["value"], "abc");
    assert_eq!(cookies[1]["name"], "theme");
    assert_eq!(cookies[1]["value"], "dark");
    assert_eq!(cookies[2]["name"], "empty");
    assert_eq!(cookies[2]["value"], "");
}
#[test]
fn test_har_set_cookie_strips_attributes_before_equal_split() {
    let entry = HarEntry {
        request_id: "r".to_string(),
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
        response_headers: vec![(
            "set-cookie".to_string(),
            "token=abc; Path=/; HttpOnly".to_string(),
        )],
        mime_type: "text/html".to_string(),
        redirect_url: String::new(),
        response_body_size: 0,
        cdp_timing: None,
        loading_finished_timestamp: None,
    };
    let har = har_entry_to_json(entry);
    assert_eq!(har["response"]["cookies"][0]["name"], "token");
    assert_eq!(har["response"]["cookies"][0]["value"], "abc");
}
#[test]
fn test_har_compute_timings_no_cdp_timing() {
    let (timings, total) = har_compute_timings(None, None);
    assert_eq!(timings["send"], 0);
    assert_eq!(timings["wait"], 0);
    assert_eq!(timings["receive"], 0);
    assert_eq!(total, 0.0);
}
#[test]
fn test_har_compute_timings_with_cdp_timing() {
    let cdp = json!(
        { "requestTime" : 1000.0, "dnsStart" : 0.0, "dnsEnd" : 5.0, "connectStart" : 5.0,
        "connectEnd" : 15.0, "sslStart" : 8.0, "sslEnd" : 15.0, "sendStart" : 15.0,
        "sendEnd" : 16.0, "receiveHeadersStart" : 16.0, "receiveHeadersEnd" : 50.0, }
    );
    let (timings, total) = har_compute_timings(Some(&cdp), Some(1000.1));
    assert_eq!(timings["dns"], 5.0);
    assert_eq!(timings["connect"], 10.0);
    assert_eq!(timings["ssl"], 7.0);
    assert_eq!(timings["send"], 1.0);
    assert!(total > 0.0);
}
#[tokio::test]
async fn test_handle_har_stop_without_path_uses_default_location() {
    let _guard = EnvGuard::new(&["HOME"]);
    let mut state = DaemonState::new();
    state.har_recording = true;
    state.har_entries.push(HarEntry {
        request_id: "req-2".to_string(),
        wall_time: 1773576000.0,
        method: "GET".to_string(),
        url: "https://example.com/".to_string(),
        request_headers: vec![("Accept".to_string(), "text/html".to_string())],
        post_data: None,
        request_body_size: 0,
        resource_type: "Document".to_string(),
        status: Some(200),
        status_text: "OK".to_string(),
        http_version: "HTTP/2.0".to_string(),
        response_headers: vec![("content-type".to_string(), "text/html".to_string())],
        mime_type: "text/html".to_string(),
        redirect_url: String::new(),
        response_body_size: 128,
        cdp_timing: None,
        loading_finished_timestamp: None,
    });
    let result = handle_har_stop(&json!({ "action" : "har_stop" }), &mut state)
        .await
        .unwrap();
    let path = result["path"].as_str().unwrap();
    assert!(path.ends_with(".har"));
    assert!(std::path::Path::new(path).starts_with(get_har_dir()));
    assert_eq!(result["requestCount"], 1);
    assert!(!state.har_recording);
    assert!(state.har_entries.is_empty());
    let har: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(har["log"]["version"], "1.2");
    assert_eq!(har["log"]["creator"]["name"], "agent-browser");
    assert!(har["log"].get("browser").is_none());
    assert_eq!(har["log"]["entries"][0]["response"]["content"]["size"], 128);
    let _ = fs::remove_file(path);
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
#[test]
fn test_browser_metadata_from_version_parses_product() {
    let metadata =
        browser_metadata_from_version(&json!({ "product" : "HeadlessChrome/123.0.6312.0" }))
            .unwrap();
    assert_eq!(metadata["name"], "HeadlessChrome");
    assert_eq!(metadata["version"], "123.0.6312.0");
}
#[test]
fn test_default_timeout_ms_from_env() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_DEFAULT_TIMEOUT"]);
    guard.set("AGENT_BROWSER_DEFAULT_TIMEOUT", "3000");
    let state = DaemonState::new();
    assert_eq!(state.default_timeout_ms, 3000);
}
#[test]
fn test_default_timeout_ms_fallback() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_DEFAULT_TIMEOUT"]);
    guard.remove("AGENT_BROWSER_DEFAULT_TIMEOUT");
    let state = DaemonState::new();
    assert_eq!(state.default_timeout_ms, 30_000);
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
#[tokio::test]
async fn test_state_list_via_actions() {
    let mut state = DaemonState::new();
    let cmd = json!({ "action" : "state_list", "id" : "s1" });
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert!(result["data"]["files"].is_array());
}
#[tokio::test]
async fn test_service_status_via_actions_does_not_launch_browser() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_status", "id" : "svc1", "serviceState" : { "controlPlane" :
        { "workerState" : "ready", "browserHealth" : "NotStarted", "queueDepth" : 2,
        "queueCapacity" : 64 }, "sitePolicies" : { "google" : { "id" : "google",
        "originPattern" : "https://accounts.google.com" } }, "jobs" : { "lease-wait" : {
        "id" : "lease-wait", "action" : "navigate", "state" : "waiting_profile_lease",
        "result" : { "profileId" : "work", "conflictSessionIds" : ["holder"] } },
        "queued" : { "id" : "queued", "action" : "click", "state" : "queued" } },
        "profiles" : { "work" : { "id" : "work", "name" : "Work" } }, "sessions" : {
        "holder" : { "id" : "holder", "profileId" : "work", "lease" : "exclusive" } },
        "displayAllocations" : { "display-orphan" : { "id" : "display-orphan", "state" :
        "released", "routeIds" : [] } } }, "launchConfig" : { "defaultBrowserBuild" :
        "stealthcdp_chromium", "stealthCdpChromiumRequired" : true,
        "stealthCdpChromiumReady" : false, "executablePath" : null,
        "executablePathSource" : null, "executablePathExists" : null,
        "browserBuildManifests" : {}, "profileSmoke" : { "available" : false, "command" :
        "pnpm test:wsl-windows-chromium-profile-live", "reason" :
        "stealthcdp_executable_missing", "isWsl" : true, "executableOnWindowsMount" :
        false, "description" :
        "Launches Windows chromium-stealthcdp from WSL with an isolated daemon socket and Windows-mounted profile, then verifies profile writes and Chrome stderr path hygiene."
        }, "warnings" : [{ "code" : "stealthcdp_executable_missing", "severity" :
        "warning", "message" : "missing" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_status_response_contract(&result["data"]);
    assert_eq!(result["data"]["closedTabProjection"]["mode"], "bounded");
    assert_eq!(
        result["data"]["closedTabProjection"]["diagnosticAvailable"],
        true
    );
    let mut full_cmd = cmd.clone();
    full_cmd["fullTabHistory"] = json!(true);
    let full_result = execute_command(&full_cmd, &mut state).await;
    assert_eq!(full_result["data"]["closedTabProjection"]["mode"], "full");
    assert_eq!(
        result["data"]["launchConfig"]["warnings"][0]["code"],
        "stealthcdp_executable_missing"
    );
    assert_eq!(
        result["data"]["service_state"]["sitePolicies"]["google"]["id"],
        "google"
    );
    assert_eq!(
        result["data"]["service_state"]["controlPlane"]["waitingProfileLeaseJobCount"],
        1
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["recommendedAction"],
        "release_holder_or_redirect_waiting_jobs"
    );
    assert_eq!(result["data"]["retainedDisplayAllocations"]["count"], 1);
    assert_eq!(
        result["data"]["retainedDisplayAllocations"]["applySafeCount"],
        1
    );
    assert_eq!(
        result["data"]["retainedDisplayAllocations"]["classCounts"]["safe-orphan-display"],
        1
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_status_repairs_stale_guacamole_view_url() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_URL"]);
    guard.set(
        "AGENT_BROWSER_REMOTE_VIEW_URL",
        "/guacamole/#/client/MQBjAHBvc3RncmVzcWw=",
    );
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_status", "id" : "svc-view-url", "serviceState" : {
        "browsers" : { "session:odollo-carrier-ups" : { "id" :
        "session:odollo-carrier-ups", "host" : "remote_headed", "health" : "ready",
        "viewStreams" : [{ "id" : "remote-headed-view", "provider" : "rdp_gateway",
        "controlInput" : "manual_attached_desktop", "url" :
        "https://agent-browser.example/guacamole/", "readOnly" : false }] },
        "browser-cdp" : { "id" : "browser-cdp", "host" : "remote_headed", "health" :
        "ready", "viewStreams" : [{ "id" : "cdp", "provider" : "cdp_screencast", "url" :
        "https://agent-browser.example/guacamole/", "readOnly" : false }] } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(
        result["data"]["service_state"]["browsers"]["session:odollo-carrier-ups"]["viewStreams"][0]
            ["frameUrl"],
        "/guacamole/#/client/MQBjAHBvc3RncmVzcWw="
    );
    assert_eq!(
        result["data"]["service_state"]["browsers"]["browser-cdp"]["viewStreams"][0]["url"],
        "https://agent-browser.example/guacamole/"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_status_leaves_guacamole_root_without_route() {
    let _guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_URL"]);
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_status", "id" : "svc-view-url-fallback", "serviceState" : {
        "browsers" : { "session:odollo-carrier-ups" : { "id" :
        "session:odollo-carrier-ups", "host" : "remote_headed", "health" : "ready",
        "viewStreams" : [{ "id" : "remote-headed-view", "provider" : "rdp_gateway",
        "controlInput" : "manual_attached_desktop", "url" :
        "https://agent-browser.example/guacamole/", "readOnly" : false }] } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(
        result["data"]["service_state"]["browsers"]["session:odollo-carrier-ups"]["viewStreams"][0]
            ["url"],
        "https://agent-browser.example/guacamole/"
    );
    assert!(
        result["data"]["service_state"]["browsers"]["session:odollo-carrier-ups"]["viewStreams"][0]
            ["frameUrl"]
            .is_null()
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_status_legacy_launch_default_does_not_accept_present_malformed_value() {
    let mut state = DaemonState::new();
    let legacy = execute_command(
        &json!({ "action" : "service_status", "id" : "svc-legacy-launch-default" }),
        &mut state,
    )
    .await;
    assert_eq!(legacy["success"], true);
    assert_eq!(
        legacy["data"]["launchConfig"]["defaultBrowserBuild"],
        Value::Null
    );
    assert_eq!(
        legacy["data"]["launchConfig"]["stealthCdpChromiumReady"],
        true
    );
    let malformed = execute_command(
        &json!(
            { "action" : "service_status", "id" : "svc-malformed-launch-config",
            "launchConfig" : {} }
        ),
        &mut state,
    )
    .await;
    assert_eq!(malformed["success"], false);
    assert!(malformed["error"]
        .as_str()
        .unwrap()
        .contains("invalid launchConfig"));
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_browsers_via_actions_returns_last_health_observation() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_browsers", "id" : "svc-browsers-1", "serviceState" : {
        "browsers" : { "browser-1" : { "id" : "browser-1", "health" : "degraded",
        "lastHealthObservation" : { "observedAt" : "2026-04-25T00:00:00Z", "failureClass"
        : "browser_shutdown_degraded", "processExitCause" : "operator_requested_close" }
        } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(&result["data"], "browsers", "browsers response");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["browsers"][0]["id"], "browser-1");
    assert_eq!(
        result["data"]["browsers"][0]["lastHealthObservation"]["failureClass"],
        "browser_shutdown_degraded"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_profiles_via_actions_returns_profile_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_profiles", "id" : "svc-profiles-1", "serviceState" : {
        "profiles" : { "work" : { "id" : "work", "name" : "Work", "profileOrigin" :
        "external_byop", "allocation" : "per_service", "keyring" :
        "basic_password_store", "targetServiceIds" : ["google"],
        "authenticatedServiceIds" : [], "sharedServiceIds" : ["JournalDownloader"] } },
        "sitePolicies" : { "google" : { "id" : "google", "originPattern" :
        "https://accounts.google.com", "manualLoginPreferred" : true } }, "sessions" : {
        "holder" : { "id" : "holder", "serviceName" : "JournalDownloader", "agentName" :
        "codex", "taskName" : "probeACSwebsite", "profileId" : "work", "lease" :
        "exclusive", "browserIds" : ["browser-1"], "tabIds" : ["tab-1"] } }, "browsers" :
        { "browser-1" : { "id" : "browser-1", "profileId" : "work", "activeSessionIds" :
        ["holder"] } }, "jobs" : { "wait" : { "id" : "wait", "action" : "navigate",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "state" : "waiting_profile_lease", "result" : {
        "waitingProfileLease" : true, "profileId" : "work", "conflictSessionIds" :
        ["holder"], "retryAfterMs" : 250 } } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(&result["data"], "profiles", "profiles response");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["profiles"][0]["id"], "work");
    assert_eq!(result["data"]["profiles"][0]["name"], "Work");
    assert_eq!(
        result["data"]["profiles"][0]["profileOrigin"],
        "external_byop"
    );
    assert_eq!(result["data"]["profileSources"][0]["id"], "work");
    assert_eq!(
        result["data"]["profileSources"][0]["source"],
        "persisted_state"
    );
    assert_eq!(
        result["data"]["profiles"][0]["targetReadiness"][0]["state"],
        "needs_manual_seeding"
    );
    assert_eq!(
        result["data"]["profiles"][0]["targetReadiness"][0]["recommendedAction"],
        "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
    );
    assert_eq!(result["data"]["profileAllocations"][0]["profileId"], "work");
    assert_eq!(
        result["data"]["profileAllocations"][0]["profileOrigin"],
        "external_byop"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["targetReadiness"][0]["state"],
        "needs_manual_seeding"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["leaseState"],
        "conflicted"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["recommendedAction"],
        "release_holder_or_redirect_waiting_jobs"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["holderSessionIds"][0],
        "holder"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["waitingJobIds"][0],
        "wait"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["conflictSessionIds"][0],
        "holder"
    );
    assert_eq!(
        result["data"]["profileAllocations"][0]["serviceNames"][0],
        "JournalDownloader"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_profile_lookup_via_actions_is_ranked_and_no_launch() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_profile_lookup", "id" : "svc-profile-lookup-1",
        "targetServiceId" : "x", "serviceState" : { "profiles" : { "social" : { "id" :
        "social", "name" : "Social", "targetServiceIds" : ["x"],
        "authenticatedServiceIds" : ["x"], "sharedServiceIds" : ["last30days"],
        "persistent" : true } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["status"], "matched");
    assert_eq!(result["data"]["rankedProfiles"][0]["profileId"], "social");
    assert_eq!(
        result["data"]["rankedProfiles"][0]["reason"],
        "authenticated_target"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_profile_seeding_handoff_via_actions_returns_command() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_profile_seeding_handoff", "id" : "svc-profile-seeding-1",
        "profileId" : "work", "targetServiceId" : "google", "serviceState" : { "profiles"
        : { "work" : { "id" : "work", "name" : "Work", "targetServiceIds" : ["google"] }
        }, "sitePolicies" : { "google" : { "id" : "google", "originPattern" :
        "https://accounts.google.com", "manualLoginPreferred" : true } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["profileId"], "work");
    assert_eq!(result["data"]["seedingMode"], "detached_headed_no_cdp");
    assert_eq!(
        result["data"]["command"],
        "agent-browser --runtime-profile work runtime login https://accounts.google.com"
    );
    assert_eq!(
        result["data"]["operatorIntervention"]["severity"],
        "action_required"
    );
    assert_eq!(
        result["data"]["operatorIntervention"]["desktopPopupPolicy"],
        "optional_policy_controlled"
    );
    assert_eq!(
        result["data"]["operatorIntervention"]["defaultChannels"],
        json!(["api", "mcp", "dashboard"])
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_sessions_via_actions_returns_session_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_sessions", "id" : "svc-sessions-1", "serviceState" : {
        "sessions" : { "session-1" : { "id" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "profileId" : "work", "browserIds" : ["browser-1"] } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(&result["data"], "sessions", "sessions response");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["sessions"][0]["id"], "session-1");
    assert_eq!(
        result["data"]["sessions"][0]["serviceName"],
        "JournalDownloader"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_tabs_via_actions_returns_tab_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_tabs", "id" : "svc-tabs-1", "serviceState" : { "tabs" : {
        "tab-1" : { "id" : "tab-1", "browserId" : "browser-1", "sessionId" :
        "cdp-session-1", "ownerSessionId" : "runtime-session", "lifecycle" : "ready",
        "targetId" : "target-1", "title" : "Example", "url" : "https://example.com/" } }
        } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(&result["data"], "tabs", "tabs response");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["tabs"][0]["id"], "tab-1");
    assert_eq!(result["data"]["tabs"][0]["lifecycle"], "ready");
    assert_eq!(result["data"]["tabs"][0]["browserId"], "browser-1");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_monitors_via_actions_returns_monitor_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_monitors", "id" : "svc-monitors-1", "serviceState" : {
        "monitors" : { "login-freshness" : { "id" : "login-freshness", "name" :
        "Login freshness", "target" : { "site_policy" : "google" }, "intervalMs" : 60000,
        "state" : "paused", "lastCheckedAt" : null, "lastSucceededAt" : null,
        "lastFailedAt" : null, "lastResult" : null, "consecutiveFailures" : 0 } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(&result["data"], "monitors", "monitors response");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["monitors"][0]["id"], "login-freshness");
    assert_eq!(result["data"]["monitors"][0]["state"], "paused");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_monitors_via_actions_filters_and_summarizes_failures() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_monitors", "id" : "svc-monitors-filtered", "monitorState" :
        "faulted", "failedOnly" : true, "summary" : true, "serviceState" : { "monitors" :
        { "healthy" : { "id" : "healthy", "name" : "Healthy", "target" : { "site_policy"
        : "google" }, "intervalMs" : 60000, "state" : "active", "lastCheckedAt" :
        "2026-05-07T00:00:00Z", "lastSucceededAt" : "2026-05-07T00:00:00Z",
        "lastFailedAt" : null, "lastResult" : "site_policy_available",
        "consecutiveFailures" : 0 }, "login-freshness" : { "id" : "login-freshness",
        "name" : "Login freshness", "target" : { "site_policy" : "google" }, "intervalMs"
        : 60000, "state" : "faulted", "lastCheckedAt" : "2026-05-07T00:01:00Z",
        "lastSucceededAt" : null, "lastFailedAt" : "2026-05-07T00:01:00Z", "lastResult" :
        "site_policy_missing", "consecutiveFailures" : 2 } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["total"], 2);
    assert_eq!(result["data"]["monitors"][0]["id"], "login-freshness");
    assert_eq!(result["data"]["summary"]["faulted"], 1);
    assert_eq!(result["data"]["summary"]["failing"], 1);
    assert_eq!(result["data"]["summary"]["repeatedFailures"], 1);
}
#[tokio::test]
async fn test_service_prune_retained_dry_run_reports_candidates_without_launch() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_prune_retained", "id" : "svc-prune-retained-1",
        "serviceState" : { "browsers" : { "browser-old" : { "id" : "browser-old",
        "health" : "not_started", "host" : "local_headed" }, "browser-live" : { "id" :
        "browser-live", "health" : "ready", "host" : "local_headed", "pid" : 123 } },
        "tabs" : { "tab-closed" : { "id" : "tab-closed", "browserId" : "browser-old",
        "lifecycle" : "closed" }, "tab-ready" : { "id" : "tab-ready", "browserId" :
        "browser-live", "lifecycle" : "ready" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["dryRun"], true);
    assert_eq!(result["data"]["candidateCounts"]["closedTabs"], 1);
    assert_eq!(result["data"]["candidateCounts"]["browsers"], 1);
    assert_eq!(result["data"]["removed"]["closedTabs"], 0);
}
#[tokio::test]
async fn test_service_repair_retained_dry_run_reports_legacy_missing_age() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_repair_retained", "id" : "svc-repair-retained-1",
        "serviceState" : { "browsers" : { "session:legacy-session" : { "id" :
        "session:legacy-session", "health" : "not_started", "activeSessionIds" :
        ["legacy-session"] }, "session:tabbed-session" : { "id" :
        "session:tabbed-session", "health" : "not_started", "activeSessionIds" :
        ["tabbed-session"] } }, "sessions" : { "legacy-session" : { "id" :
        "legacy-session", "lease" : "shared", "browserIds" : ["session:legacy-session"]
        }, "fresh-session" : { "id" : "fresh-session", "lease" : "shared", "browserIds" :
        ["session:fresh-session"], "lastLeaseObservedAt" : "2026-05-17T00:00:00Z" },
        "tabbed-session" : { "id" : "tabbed-session", "lease" : "shared", "browserIds" :
        ["session:tabbed-session"], "tabIds" : ["tab-1"] } }, "tabs" : { "tab-1" : { "id"
        : "tab-1", "browserId" : "session:tabbed-session", "lifecycle" : "ready" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["dryRun"], true);
    assert_eq!(
        result["data"]["candidateCounts"]["missingLeaseObservedAt"],
        1
    );
    assert_eq!(
        result["data"]["candidates"]["missingLeaseObservedAt"][0],
        "legacy-session"
    );
    assert_eq!(
        result["data"]["repairedCounts"]["missingLeaseObservedAt"],
        0
    );
    assert!(state.browser.is_none());
}
#[test]
fn test_repair_retained_service_state_apply_stamps_observation_time() {
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([(
            "session:legacy-session".to_string(),
            BrowserProcess {
                id: "session:legacy-session".to_string(),
                health: ServiceBrowserHealth::NotStarted,
                active_session_ids: vec!["legacy-session".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "legacy-session".to_string(),
            BrowserSession {
                id: "legacy-session".to_string(),
                lease: LeaseState::Shared,
                browser_ids: vec!["session:legacy-session".to_string()],
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = repair_retained_service_state(
        &mut service_state,
        ServiceRetentionRepairOptions {
            apply: true,
            missing_lease_observed_at: true,
        },
        "2026-05-17T12:00:00Z",
    );
    assert_eq!(result["repaired"], true);
    assert_eq!(result["repairedCounts"]["missingLeaseObservedAt"], 1);
    assert_eq!(
        service_state.sessions["legacy-session"].last_lease_observed_at,
        Some("2026-05-17T12:00:00Z".to_string())
    );
}
#[test]
fn test_prune_retained_service_state_apply_removes_session_references() {
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([
            (
                "browser-old".to_string(),
                BrowserProcess {
                    id: "browser-old".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    ..BrowserProcess::default()
                },
            ),
            (
                "browser-exited".to_string(),
                BrowserProcess {
                    id: "browser-exited".to_string(),
                    health: ServiceBrowserHealth::ProcessExited,
                    pid: Some(99),
                    cdp_endpoint: Some("http://127.0.0.1:9999".to_string()),
                    ..BrowserProcess::default()
                },
            ),
        ]),
        tabs: BTreeMap::from([(
            "tab-closed".to_string(),
            BrowserTab {
                id: "tab-closed".to_string(),
                browser_id: "browser-old".to_string(),
                lifecycle: TabLifecycle::Closed,
                ..BrowserTab::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-old".to_string(), "browser-exited".to_string()],
                tab_ids: vec!["tab-closed".to_string()],
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: false,
            released_sessions: false,
            abandoned_sessions: false,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(result["pruned"], true);
    assert_eq!(result["removed"]["closedTabs"], 1);
    assert_eq!(result["removed"]["browsers"], 1);
    assert!(!service_state.browsers.contains_key("browser-old"));
    assert!(service_state.browsers.contains_key("browser-exited"));
    assert!(!service_state.tabs.contains_key("tab-closed"));
    assert_eq!(
        service_state.sessions["session-1"].browser_ids,
        vec!["browser-exited"]
    );
    assert!(service_state.sessions["session-1"].tab_ids.is_empty());
}
#[test]
fn test_prune_retained_service_state_removes_released_inert_session() {
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([(
            "session:released-session".to_string(),
            BrowserProcess {
                id: "session:released-session".to_string(),
                health: ServiceBrowserHealth::NotStarted,
                active_session_ids: vec!["released-session".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "released-session".to_string(),
            BrowserSession {
                id: "released-session".to_string(),
                lease: LeaseState::Released,
                browser_ids: vec!["session:released-session".to_string()],
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: false,
            released_sessions: true,
            abandoned_sessions: false,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(result["removed"]["sessions"], 1);
    assert_eq!(result["removed"]["browsers"], 1);
    assert!(service_state.sessions.is_empty());
    assert!(service_state.browsers.is_empty());
}
#[test]
fn test_prune_retained_service_state_removes_released_session_with_retained_view_stream() {
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([(
            "session:released-session".to_string(),
            BrowserProcess {
                id: "session:released-session".to_string(),
                health: ServiceBrowserHealth::NotStarted,
                active_session_ids: vec!["released-session".to_string()],
                view_streams: vec![ViewStream {
                    id: "stale-left-rail-stream".to_string(),
                    ..ViewStream::default()
                }],
                ..BrowserProcess::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "released-session".to_string(),
            BrowserSession {
                id: "released-session".to_string(),
                lease: LeaseState::Released,
                browser_ids: vec!["session:released-session".to_string()],
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: false,
            released_sessions: true,
            abandoned_sessions: false,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(result["removed"]["sessions"], 1);
    assert_eq!(result["removed"]["browsers"], 1);
    assert_eq!(
        result["policy"]["releasedSessionPruneRemovesRetainedViewStreams"],
        true
    );
    assert!(service_state.sessions.is_empty());
    assert!(service_state.browsers.is_empty());
}
#[test]
fn test_prune_retained_service_state_removes_historical_browser_placeholders() {
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([
            (
                "session:missing-session".to_string(),
                BrowserProcess {
                    id: "session:missing-session".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    active_session_ids: vec!["missing-session".to_string()],
                    profile_id: Some("default".to_string()),
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:released-faulted".to_string(),
                BrowserProcess {
                    id: "session:released-faulted".to_string(),
                    health: ServiceBrowserHealth::Faulted,
                    active_session_ids: vec!["released-faulted".to_string()],
                    view_streams: vec![ViewStream {
                        id: "stale-stream".to_string(),
                        ..ViewStream::default()
                    }],
                    last_error: Some("Force kill failed; OS may be degraded.".to_string()),
                    ..BrowserProcess::default()
                },
            ),
        ]),
        sessions: BTreeMap::from([(
            "released-faulted".to_string(),
            BrowserSession {
                id: "released-faulted".to_string(),
                lease: LeaseState::Released,
                browser_ids: vec!["session:released-faulted".to_string()],
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: true,
            released_sessions: true,
            abandoned_sessions: false,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(result["removed"]["browsers"], 2);
    assert_eq!(result["removed"]["sessions"], 1);
    assert!(service_state.browsers.is_empty());
    assert!(service_state.sessions.is_empty());
}
#[test]
fn test_prune_retained_service_state_abandoned_sessions_require_age() {
    let old_session_time = (chrono::Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
    let fresh_session_time = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([
            (
                "session:old-session".to_string(),
                BrowserProcess {
                    id: "session:old-session".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    active_session_ids: vec!["old-session".to_string()],
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:fresh-session".to_string(),
                BrowserProcess {
                    id: "session:fresh-session".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    active_session_ids: vec!["fresh-session".to_string()],
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:fresh-session-2".to_string(),
                BrowserProcess {
                    id: "session:fresh-session-2".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    active_session_ids: vec!["fresh-session-2".to_string()],
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:unknown-session".to_string(),
                BrowserProcess {
                    id: "session:unknown-session".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    active_session_ids: vec!["unknown-session".to_string()],
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:unknown-session-2".to_string(),
                BrowserProcess {
                    id: "session:unknown-session-2".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    active_session_ids: vec!["unknown-session-2".to_string()],
                    ..BrowserProcess::default()
                },
            ),
        ]),
        sessions: BTreeMap::from([
            (
                "old-session".to_string(),
                BrowserSession {
                    id: "old-session".to_string(),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["session:old-session".to_string()],
                    created_at: Some(fresh_session_time.clone()),
                    last_lease_observed_at: Some(old_session_time),
                    ..BrowserSession::default()
                },
            ),
            (
                "fresh-session".to_string(),
                BrowserSession {
                    id: "fresh-session".to_string(),
                    lease: LeaseState::Shared,
                    browser_ids: vec!["session:fresh-session".to_string()],
                    created_at: Some("2000-01-01T00:00:00Z".to_string()),
                    last_lease_observed_at: Some(fresh_session_time.clone()),
                    ..BrowserSession::default()
                },
            ),
            (
                "fresh-session-2".to_string(),
                BrowserSession {
                    id: "fresh-session-2".to_string(),
                    lease: LeaseState::Shared,
                    browser_ids: vec!["session:fresh-session-2".to_string()],
                    last_lease_observed_at: Some(fresh_session_time.clone()),
                    ..BrowserSession::default()
                },
            ),
            (
                "unknown-session".to_string(),
                BrowserSession {
                    id: "unknown-session".to_string(),
                    lease: LeaseState::Shared,
                    browser_ids: vec!["session:unknown-session".to_string()],
                    ..BrowserSession::default()
                },
            ),
            (
                "unknown-session-2".to_string(),
                BrowserSession {
                    id: "unknown-session-2".to_string(),
                    lease: LeaseState::Shared,
                    browser_ids: vec!["session:unknown-session-2".to_string()],
                    ..BrowserSession::default()
                },
            ),
        ]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: false,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: false,
            released_sessions: false,
            abandoned_sessions: true,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 60,
        },
    );
    assert_eq!(result["candidateCounts"]["sessions"], 1);
    assert_eq!(result["candidates"]["sessions"][0], "old-session");
    assert_eq!(
        result["skipped"]["abandonedSessionsTooFresh"][0],
        "fresh-session"
    );
    assert_eq!(
        result["skipped"]["abandonedSessionsMissingAgeTimestamp"][0],
        "unknown-session"
    );
    assert_eq!(result["skippedCounts"]["abandonedSessionsTooFresh"], 2);
    assert_eq!(
        result["skippedCounts"]["abandonedSessionsMissingAgeTimestamp"],
        2
    );
    assert_eq!(
        result["skippedSummary"]["abandonedSessionsTooFresh"]["groups"][0]["group"],
        "fresh-session"
    );
    assert_eq!(
        result["skippedSummary"]["abandonedSessionsTooFresh"]["groups"][0]["count"],
        2
    );
    assert_eq!(
        result["skippedSummary"]["abandonedSessionsMissingAgeTimestamp"]["groups"][0]["group"],
        "unknown-session"
    );
    assert_eq!(
        result["skippedSummary"]["abandonedSessionsMissingAgeTimestamp"]["groups"][0]["count"],
        2
    );
    assert_eq!(
        result["policy"]["abandonedSessionsRequireAgeTimestamp"],
        true
    );
    assert_eq!(
        result["policy"]["abandonedSessionAgeSource"],
        "lastLeaseObservedAtOrCreatedAt"
    );
    assert_eq!(result["policy"]["abandonedSessionMinAgeMinutes"], 60);
    assert!(service_state.sessions.contains_key("fresh-session"));
    assert!(service_state.sessions.contains_key("unknown-session"));
}
#[test]
fn test_prune_retained_service_state_removes_old_failed_session_browser() {
    let old_session_time = (chrono::Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([
            (
                "session:old-process-exited".to_string(),
                BrowserProcess {
                    id: "session:old-process-exited".to_string(),
                    health: ServiceBrowserHealth::ProcessExited,
                    pid: Some(99),
                    cdp_endpoint: Some("ws://127.0.0.1:9999/devtools/browser/stale".to_string()),
                    active_session_ids: vec!["old-process-exited".to_string()],
                    view_streams: vec![ViewStream {
                        id: "stale-stream".to_string(),
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:old-unreachable".to_string(),
                BrowserProcess {
                    id: "session:old-unreachable".to_string(),
                    health: ServiceBrowserHealth::Unreachable,
                    cdp_endpoint: Some(
                        "ws://127.0.0.1:9998/devtools/browser/unreachable".to_string(),
                    ),
                    active_session_ids: vec!["old-unreachable".to_string()],
                    view_streams: vec![ViewStream {
                        id: "stale-cdp-stream".to_string(),
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            ),
            (
                "session:fresh-process-exited".to_string(),
                BrowserProcess {
                    id: "session:fresh-process-exited".to_string(),
                    health: ServiceBrowserHealth::ProcessExited,
                    active_session_ids: vec!["fresh-process-exited".to_string()],
                    ..BrowserProcess::default()
                },
            ),
        ]),
        sessions: BTreeMap::from([
            (
                "old-process-exited".to_string(),
                BrowserSession {
                    id: "old-process-exited".to_string(),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["session:old-process-exited".to_string()],
                    last_lease_observed_at: Some(old_session_time.clone()),
                    ..BrowserSession::default()
                },
            ),
            (
                "old-unreachable".to_string(),
                BrowserSession {
                    id: "old-unreachable".to_string(),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["session:old-unreachable".to_string()],
                    last_lease_observed_at: Some(old_session_time),
                    ..BrowserSession::default()
                },
            ),
            (
                "fresh-process-exited".to_string(),
                BrowserSession {
                    id: "fresh-process-exited".to_string(),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["session:fresh-process-exited".to_string()],
                    last_lease_observed_at: Some(
                        (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339(),
                    ),
                    ..BrowserSession::default()
                },
            ),
        ]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: true,
            released_sessions: false,
            abandoned_sessions: true,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 60,
        },
    );
    assert_eq!(result["removed"]["sessions"], 2);
    assert_eq!(result["removed"]["browsers"], 2);
    assert_eq!(result["skippedCounts"]["abandonedSessionsTooFresh"], 1);
    assert!(!service_state.sessions.contains_key("old-process-exited"));
    assert!(!service_state.sessions.contains_key("old-unreachable"));
    assert!(!service_state
        .browsers
        .contains_key("session:old-process-exited"));
    assert!(!service_state
        .browsers
        .contains_key("session:old-unreachable"));
    assert!(service_state.sessions.contains_key("fresh-process-exited"));
    assert!(service_state
        .browsers
        .contains_key("session:fresh-process-exited"));
}
#[test]
fn test_prune_retained_service_state_keeps_failed_session_without_explicit_flag() {
    let old_session_time = (chrono::Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([(
            "session:old-process-exited".to_string(),
            BrowserProcess {
                id: "session:old-process-exited".to_string(),
                health: ServiceBrowserHealth::ProcessExited,
                active_session_ids: vec!["old-process-exited".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "old-process-exited".to_string(),
            BrowserSession {
                id: "old-process-exited".to_string(),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["session:old-process-exited".to_string()],
                last_lease_observed_at: Some(old_session_time),
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: true,
            not_started_browsers: true,
            process_exited_browsers: false,
            released_sessions: false,
            abandoned_sessions: true,
            orphaned_profiles: false,
            display_allocations: false,
            abandoned_session_min_age_minutes: 60,
        },
    );
    assert_eq!(result["removed"]["sessions"], 0);
    assert_eq!(result["removed"]["browsers"], 0);
    assert!(service_state.sessions.contains_key("old-process-exited"));
    assert!(service_state
        .browsers
        .contains_key("session:old-process-exited"));
}
#[test]
fn test_prune_retained_service_state_removes_orphaned_custom_profiles() {
    let mut service_state = ServiceState {
        profiles: BTreeMap::from([
            (
                "custom:orphan".to_string(),
                BrowserProfile {
                    id: "custom:orphan".to_string(),
                    name: "/tmp/agent-browser-orphan-profile".to_string(),
                    user_data_dir: Some("/tmp/agent-browser-orphan-profile".to_string()),
                    ..BrowserProfile::default()
                },
            ),
            (
                "custom:referenced".to_string(),
                BrowserProfile {
                    id: "custom:referenced".to_string(),
                    name: "/tmp/agent-browser-referenced-profile".to_string(),
                    user_data_dir: Some("/tmp/agent-browser-referenced-profile".to_string()),
                    ..BrowserProfile::default()
                },
            ),
            (
                "managed-one-time-orphan".to_string(),
                BrowserProfile {
                    id: "managed-one-time-orphan".to_string(),
                    name: "Managed one-time login".to_string(),
                    profile_class: ProfileClass::ManagedOneTime,
                    user_data_dir: Some("/tmp/agent-browser-managed-one-time-orphan".to_string()),
                    shared_service_ids: vec!["login-service".to_string()],
                    persistent: false,
                    ..BrowserProfile::default()
                },
            ),
            (
                "durable-orphan".to_string(),
                BrowserProfile {
                    id: "durable-orphan".to_string(),
                    name: "Durable orphan".to_string(),
                    profile_class: ProfileClass::DurableNamed,
                    user_data_dir: Some("/tmp/agent-browser-durable-profile".to_string()),
                    persistent: false,
                    ..BrowserProfile::default()
                },
            ),
            (
                "custom:byop".to_string(),
                BrowserProfile {
                    id: "custom:byop".to_string(),
                    name: "/tmp/agent-browser-byop-profile".to_string(),
                    profile_origin: ProfileOrigin::ExternalByop,
                    user_data_dir: Some("/tmp/agent-browser-byop-profile".to_string()),
                    ..BrowserProfile::default()
                },
            ),
            (
                "custom:observed".to_string(),
                BrowserProfile {
                    id: "custom:observed".to_string(),
                    name: "/tmp/agent-browser-observed-profile".to_string(),
                    profile_origin: ProfileOrigin::ExternalObserved,
                    user_data_dir: Some("/tmp/agent-browser-observed-profile".to_string()),
                    ..BrowserProfile::default()
                },
            ),
            (
                "default".to_string(),
                BrowserProfile {
                    id: "default".to_string(),
                    name: "default".to_string(),
                    user_data_dir: Some("/tmp/agent-browser-default-profile".to_string()),
                    ..BrowserProfile::default()
                },
            ),
        ]),
        sessions: BTreeMap::from([(
            "referenced-session".to_string(),
            BrowserSession {
                id: "referenced-session".to_string(),
                profile_id: Some("custom:referenced".to_string()),
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let result = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: false,
            not_started_browsers: false,
            process_exited_browsers: false,
            released_sessions: false,
            abandoned_sessions: false,
            orphaned_profiles: true,
            display_allocations: false,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(result["candidateCounts"]["orphanedProfiles"], 2);
    assert_eq!(result["candidates"]["orphanedProfiles"][0], "custom:orphan");
    assert_eq!(
        result["candidates"]["orphanedProfiles"][1],
        "managed-one-time-orphan"
    );
    assert_eq!(
        result["candidateReasons"]["orphanedProfiles"]["managed-one-time-orphan"]["reason"],
        "managed_one_time_unreferenced"
    );
    assert_eq!(result["removed"]["orphanedProfiles"], 2);
    assert!(!service_state.profiles.contains_key("custom:orphan"));
    assert!(!service_state
        .profiles
        .contains_key("managed-one-time-orphan"));
    assert!(service_state.profiles.contains_key("durable-orphan"));
    assert!(service_state.profiles.contains_key("custom:referenced"));
    assert!(service_state.profiles.contains_key("custom:byop"));
    assert!(service_state.profiles.contains_key("custom:observed"));
    assert!(service_state.profiles.contains_key("default"));
}
#[test]
fn test_prune_retained_service_state_classifies_display_allocations() {
    let mut service_state = ServiceState {
        display_allocations: BTreeMap::from([
            (
                "display-live".to_string(),
                DisplayAllocation {
                    id: "display-live".to_string(),
                    owner_browser_id: Some("browser-live".to_string()),
                    route_ids: vec!["route-live".to_string()],
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-stale-route".to_string(),
                DisplayAllocation {
                    id: "display-stale-route".to_string(),
                    route_ids: vec!["route-stale".to_string()],
                    state: "released".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-orphan".to_string(),
                DisplayAllocation {
                    id: "display-orphan".to_string(),
                    state: "released".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-unknown".to_string(),
                DisplayAllocation {
                    id: "display-unknown".to_string(),
                    route_ids: vec!["route-missing".to_string()],
                    state: "released".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-historical".to_string(),
                DisplayAllocation {
                    id: "display-historical".to_string(),
                    owner_browser_id: Some("browser-missing".to_string()),
                    state: "released".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-diagnostic".to_string(),
                DisplayAllocation {
                    id: "display-diagnostic".to_string(),
                    owner_browser_id: Some("browser-diagnostic".to_string()),
                    state: "released".to_string(),
                    readiness: Some(json!({ "state" : "failed", "reason" : "provider_error" })),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-pool-live".to_string(),
                DisplayAllocation {
                    id: "display-pool-live".to_string(),
                    route_ids: vec!["route-pool".to_string()],
                    state: "released".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
            (
                "display-incident".to_string(),
                DisplayAllocation {
                    id: "display-incident".to_string(),
                    owner_browser_id: Some("browser-incident".to_string()),
                    state: "released".to_string(),
                    ..DisplayAllocation::default()
                },
            ),
        ]),
        remote_view_routes: BTreeMap::from([
            (
                "route-live".to_string(),
                RemoteViewRoute {
                    id: "route-live".to_string(),
                    display_allocation_id: Some("display-live".to_string()),
                    browser_id: Some("browser-live".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            ),
            (
                "route-stale".to_string(),
                RemoteViewRoute {
                    id: "route-stale".to_string(),
                    display_allocation_id: Some("display-stale-route".to_string()),
                    state: "released".to_string(),
                    ..RemoteViewRoute::default()
                },
            ),
            (
                "route-pool".to_string(),
                RemoteViewRoute {
                    id: "route-pool".to_string(),
                    display_allocation_id: Some("display-pool-live".to_string()),
                    state: "released".to_string(),
                    ..RemoteViewRoute::default()
                },
            ),
        ]),
        route_pool: BTreeMap::from([(
            "pool-1".to_string(),
            RoutePoolEntry {
                id: "pool-1".to_string(),
                state: "checked_out".to_string(),
                current_route_allocation_id: Some("route-pool".to_string()),
                ..RoutePoolEntry::default()
            },
        )]),
        browsers: BTreeMap::from([
            (
                "browser-live".to_string(),
                BrowserProcess {
                    id: "browser-live".to_string(),
                    health: ServiceBrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            ),
            (
                "browser-diagnostic".to_string(),
                BrowserProcess {
                    id: "browser-diagnostic".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    ..BrowserProcess::default()
                },
            ),
            (
                "browser-incident".to_string(),
                BrowserProcess {
                    id: "browser-incident".to_string(),
                    health: ServiceBrowserHealth::NotStarted,
                    ..BrowserProcess::default()
                },
            ),
        ]),
        incidents: vec![crate::native::service_model::ServiceIncident {
            id: "incident-display".to_string(),
            browser_id: Some("browser-incident".to_string()),
            monitor_id: None,
            monitor_target: None,
            monitor_result: None,
            label: "browser incident".to_string(),
            state: crate::native::service_model::ServiceIncidentState::Active,
            severity: crate::native::service_model::ServiceIncidentSeverity::Warning,
            escalation: crate::native::service_model::ServiceIncidentEscalation::BrowserRecovery,
            recommended_action: "review retained browser evidence".to_string(),
            acknowledged_at: None,
            acknowledged_by: None,
            acknowledgement_note: None,
            resolved_at: None,
            resolved_by: None,
            resolution_note: None,
            latest_timestamp: "2026-06-28T00:00:00Z".to_string(),
            latest_message: "browser incident".to_string(),
            latest_kind: "browser_health_changed".to_string(),
            current_health: Some(ServiceBrowserHealth::Faulted),
            event_ids: Vec::new(),
            job_ids: Vec::new(),
        }],
        ..ServiceState::default()
    };
    let dry_run = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: false,
            closed_tabs: false,
            not_started_browsers: false,
            process_exited_browsers: false,
            released_sessions: false,
            abandoned_sessions: false,
            orphaned_profiles: false,
            display_allocations: true,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(dry_run["candidateCounts"]["displayAllocations"], 3);
    assert_eq!(
        dry_run["candidateClassCounts"]["displayAllocations"]["live"],
        2
    );
    assert_eq!(
        dry_run["candidateClassCounts"]["displayAllocations"]["diagnostic-retained"],
        2
    );
    assert_eq!(
        dry_run["candidateClassCounts"]["displayAllocations"]["unknown"],
        1
    );
    assert_eq!(
        dry_run["candidateReasons"]["displayAllocations"]["display-stale-route"]["class"],
        "stale-route-reference"
    );
    assert_eq!(
        dry_run["candidateReasons"]["displayAllocations"]["display-orphan"]["class"],
        "safe-orphan-display"
    );
    assert_eq!(
        dry_run["candidateReasons"]["displayAllocations"]["display-historical"]["class"],
        "historical-placeholder"
    );
    assert_eq!(
        dry_run["candidateReasons"]["displayAllocations"]["display-pool-live"]["applySafe"],
        false
    );
    assert_eq!(dry_run["removed"]["displayAllocations"], 0);
    assert!(service_state
        .display_allocations
        .contains_key("display-stale-route"));
    let applied = prune_retained_service_state(
        &mut service_state,
        ServiceRetentionPruneOptions {
            apply: true,
            closed_tabs: false,
            not_started_browsers: false,
            process_exited_browsers: false,
            released_sessions: false,
            abandoned_sessions: false,
            orphaned_profiles: false,
            display_allocations: true,
            abandoned_session_min_age_minutes: 1440,
        },
    );
    assert_eq!(applied["removed"]["displayAllocations"], 3);
    assert!(!service_state
        .display_allocations
        .contains_key("display-stale-route"));
    assert!(!service_state
        .display_allocations
        .contains_key("display-orphan"));
    assert!(!service_state
        .display_allocations
        .contains_key("display-historical"));
    assert!(service_state
        .display_allocations
        .contains_key("display-live"));
    assert!(service_state
        .display_allocations
        .contains_key("display-diagnostic"));
    assert!(service_state
        .display_allocations
        .contains_key("display-unknown"));
    assert!(service_state
        .display_allocations
        .contains_key("display-pool-live"));
    assert!(service_state
        .display_allocations
        .contains_key("display-incident"));
}
#[tokio::test]
async fn test_service_site_policies_via_actions_returns_policy_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_site_policies", "id" : "svc-site-policies-1",
        "serviceState" : { "sitePolicies" : { "google" : { "id" : "google",
        "originPattern" : "https://accounts.google.com", "interactionMode" :
        "human_like_input", "manualLoginPreferred" : true, "profileRequired" : true,
        "challengePolicy" : "avoid_first" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(
        &result["data"],
        "sitePolicies",
        "site policies response",
    );
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["sitePolicies"][0]["id"], "google");
    assert_eq!(
        result["data"]["sitePolicies"][0]["originPattern"],
        "https://accounts.google.com"
    );
    assert_eq!(result["data"]["sitePolicySources"][0]["id"], "google");
    assert_eq!(
        result["data"]["sitePolicySources"][0]["source"],
        "persisted_state"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_providers_via_actions_returns_provider_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_providers", "id" : "svc-providers-1", "serviceState" : {
        "providers" : { "manual" : { "id" : "manual", "kind" : "manual_approval",
        "displayName" : "Dashboard approval", "enabled" : true, "capabilities" :
        ["human_approval"] } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(&result["data"], "providers", "providers response");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["providers"][0]["id"], "manual");
    assert_eq!(
        result["data"]["providers"][0]["displayName"],
        "Dashboard approval"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_challenges_via_actions_returns_challenge_collection() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_challenges", "id" : "svc-challenges-1", "serviceState" : {
        "challenges" : { "challenge-1" : { "id" : "challenge-1", "tabId" : "tab-1",
        "kind" : "captcha", "state" : "waiting_for_provider", "providerId" :
        "captcha-api", "policyDecision" : "provider_allowed" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_collection_response_contract(
        &result["data"],
        "challenges",
        "challenges response",
    );
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["challenges"][0]["id"], "challenge-1");
    assert_eq!(result["data"]["challenges"][0]["kind"], "captcha");
    assert!(state.browser.is_none());
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
        persisted.remote_view_routes["route-a"]
            .display_allocation_id
            .as_deref(),
        Some("display-a")
    );
    assert_eq!(persisted.remote_view_routes["route-a"].state, "ready");
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
    assert!(restored
        .display_allocations
        .get("remote-view-display:41")
        .is_none());
    assert!(restored.remote_view_routes.get("route-a").is_none());
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
    assert!(service_state.route_pool.get("pool-pending").is_none());
    assert!(service_state
        .remote_view_routes
        .get("route-pending")
        .is_none());
    assert!(service_state
        .display_allocations
        .get("display-pending")
        .is_none());
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
#[tokio::test]
async fn test_service_incidents_returns_limited_incidents() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-1", "limit" : 1,
        "serviceState" : { "incidents" : [{ "id" : "browser-1", "browserId" :
        "browser-1", "label" : "browser-1", "state" : "active", "latestTimestamp" :
        "2026-04-22T00:00:00Z", "latestMessage" : "Browser crashed", "latestKind" :
        "browser_health_changed" }, { "id" : "service", "label" : "Service incidents",
        "state" : "service", "latestTimestamp" : "2026-04-22T00:01:00Z", "latestMessage"
        : "Reconciliation failed", "latestKind" : "reconciliation_error" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 2);
    assert_eq!(result["data"]["total"], 2);
    assert_eq!(result["data"]["incidents"][0]["id"], "service");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_remedies_only_groups_operator_ladder() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-remedies", "summary" :
        true, "remediesOnly" : true, "serviceState" : { "incidents" : [{ "id" :
        "browser-degraded", "browserId" : "browser-degraded", "label" :
        "browser-degraded", "state" : "active", "severity" : "warning", "escalation" :
        "browser_degraded", "recommendedAction" : "Inspect browser health.",
        "currentHealth" : "degraded", "latestTimestamp" : "2026-05-01T00:00:00Z",
        "latestMessage" : "Polite close failed", "latestKind" : "browser_health_changed"
        }, { "id" : "browser-faulted", "browserId" : "browser-faulted", "label" :
        "browser-faulted", "state" : "active", "severity" : "critical", "escalation" :
        "os_degraded_possible", "recommendedAction" : "Inspect the host OS.",
        "currentHealth" : "faulted", "latestTimestamp" : "2026-05-01T00:01:00Z",
        "latestMessage" : "Force kill failed", "latestKind" : "browser_health_changed" },
        { "id" : "browser-recovery", "browserId" : "browser-recovery", "label" :
        "browser-recovery", "state" : "active", "severity" : "error", "escalation" :
        "browser_recovery", "recommendedAction" : "Review recovery trace.",
        "currentHealth" : "process_exited", "latestTimestamp" : "2026-05-01T00:02:00Z",
        "latestMessage" : "Browser exited", "latestKind" : "browser_health_changed" }, {
        "id" : "monitor-login", "monitorId" : "google-login-freshness", "label" :
        "Monitor Google login freshness", "state" : "active", "severity" : "warning",
        "escalation" : "monitor_attention", "recommendedAction" :
        "Inspect the failed monitor target.", "latestTimestamp" : "2026-05-01T00:03:00Z",
        "latestMessage" : "Login freshness failed", "latestKind" :
        "service_monitor_failed" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 3);
    assert_eq!(result["data"]["matched"], 3);
    assert_eq!(result["data"]["filters"]["remediesOnly"], true);
    let summary_groups = result["data"]["summary"]["groups"].as_array().unwrap();
    assert_eq!(summary_groups.len(), 3);
    assert!(summary_groups
        .iter()
        .any(|group| group["escalation"] == "browser_degraded"));
    assert!(summary_groups
        .iter()
        .any(|group| group["escalation"] == "os_degraded_possible"));
    let monitor_group = summary_groups
        .iter()
        .find(|group| group["escalation"] == "monitor_attention")
        .expect("monitor attention group should be included in remedies");
    assert_eq!(
        monitor_group["monitorIds"],
        json!(["google-login-freshness"])
    );
    assert_eq!(
        monitor_group["monitorResetCommands"],
        json!(["agent-browser service monitors reset google-login-freshness"])
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_filter_by_state_kind_browser_and_since() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-2", "state" :
        "recovered", "kind" : "browser_health_changed", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "since" : "2026-04-22T00:01:00Z", "serviceState" : { "events" : [{ "id" :
        "event-old", "timestamp" : "2026-04-22T00:00:00Z", "kind" :
        "browser_health_changed", "message" : "Too old", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-match", "timestamp" : "2026-04-22T00:01:00Z", "kind" :
        "browser_health_changed", "message" : "Matching incident", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }, {
        "id" : "event-wrong-context", "timestamp" : "2026-04-22T00:04:00Z", "kind" :
        "browser_health_changed", "message" : "Wrong context", "browserId" : "browser-1",
        "profileId" : "other", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite" }],
        "incidents" : [{ "id" : "browser-1-old", "browserId" : "browser-1", "label" :
        "browser-1", "state" : "recovered", "latestTimestamp" : "2026-04-22T00:00:00Z",
        "latestMessage" : "Too old", "latestKind" : "browser_health_changed", "eventIds"
        : ["event-old"] }, { "id" : "browser-1-match", "browserId" : "browser-1", "label"
        : "browser-1", "state" : "recovered", "latestTimestamp" : "2026-04-22T00:01:00Z",
        "latestMessage" : "Matching incident", "latestKind" : "browser_health_changed",
        "eventIds" : ["event-match"] }, { "id" : "browser-2", "browserId" : "browser-2",
        "label" : "browser-2", "state" : "recovered", "latestTimestamp" :
        "2026-04-22T00:02:00Z", "latestMessage" : "Wrong browser", "latestKind" :
        "browser_health_changed", "eventIds" : ["event-match"] }, { "id" : "service",
        "label" : "Service incidents", "state" : "service", "latestTimestamp" :
        "2026-04-22T00:03:00Z", "latestMessage" : "Wrong state", "latestKind" :
        "reconciliation_error", "eventIds" : ["event-match"] }, { "id" :
        "browser-1-wrong-context", "browserId" : "browser-1", "label" : "browser-1",
        "state" : "recovered", "latestTimestamp" : "2026-04-22T00:04:00Z",
        "latestMessage" : "Wrong context", "latestKind" : "browser_health_changed",
        "eventIds" : ["event-wrong-context"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["total"], 5);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-1-match");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_filter_by_handling_state() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-2b", "handlingState" :
        "acknowledged", "serviceState" : { "incidents" : [{ "id" : "incident-unack",
        "label" : "Service incidents", "state" : "service", "latestTimestamp" :
        "2026-04-22T00:00:00Z", "latestMessage" : "Needs attention", "latestKind" :
        "reconciliation_error" }, { "id" : "incident-ack", "label" : "Service incidents",
        "state" : "service", "acknowledgedAt" : "2026-04-22T00:01:00Z", "acknowledgedBy"
        : "operator", "latestTimestamp" : "2026-04-22T00:01:00Z", "latestMessage" :
        "Triaged", "latestKind" : "reconciliation_error" }, { "id" : "incident-resolved",
        "label" : "Service incidents", "state" : "service", "acknowledgedAt" :
        "2026-04-22T00:02:00Z", "resolvedAt" : "2026-04-22T00:03:00Z", "resolvedBy" :
        "operator", "latestTimestamp" : "2026-04-22T00:03:00Z", "latestMessage" :
        "Handled", "latestKind" : "reconciliation_error" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["incidents"][0]["id"], "incident-ack");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_filter_by_severity_and_escalation() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-severity", "severity" :
        "critical", "escalation" : "os_degraded_possible", "serviceState" : { "incidents"
        : [{ "id" : "browser-degraded", "browserId" : "browser-degraded", "label" :
        "browser-degraded", "state" : "active", "severity" : "warning", "escalation" :
        "browser_degraded", "recommendedAction" : "Inspect browser health.",
        "latestTimestamp" : "2026-04-27T00:00:00Z", "latestMessage" :
        "Polite close failed", "latestKind" : "browser_health_changed", "currentHealth" :
        "degraded" }, { "id" : "browser-faulted", "browserId" : "browser-faulted",
        "label" : "browser-faulted", "state" : "active", "severity" : "critical",
        "escalation" : "os_degraded_possible", "recommendedAction" :
        "Inspect the host OS.", "latestTimestamp" : "2026-04-27T00:01:00Z",
        "latestMessage" : "Force kill failed", "latestKind" : "browser_health_changed",
        "currentHealth" : "faulted" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-faulted");
    assert_eq!(result["data"]["incidents"][0]["severity"], "critical");
    assert_eq!(
        result["data"]["incidents"][0]["escalation"],
        "os_degraded_possible"
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_summary_groups_by_operator_remedy() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-summary", "summary" :
        true, "serviceState" : { "incidents" : [{ "id" : "browser-degraded", "browserId"
        : "browser-degraded", "label" : "browser-degraded", "state" : "active",
        "severity" : "warning", "escalation" : "browser_degraded", "recommendedAction" :
        "Inspect browser health.", "latestTimestamp" : "2026-04-27T00:00:00Z",
        "latestMessage" : "Polite close failed", "latestKind" : "browser_health_changed",
        "currentHealth" : "degraded" }, { "id" : "browser-faulted", "browserId" :
        "browser-faulted", "label" : "browser-faulted", "state" : "active", "severity" :
        "critical", "escalation" : "os_degraded_possible", "recommendedAction" :
        "Inspect the host OS.", "latestTimestamp" : "2026-04-27T00:01:00Z",
        "latestMessage" : "Force kill failed", "latestKind" : "browser_health_changed",
        "currentHealth" : "faulted" }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["summary"]["groupCount"], 2);
    let groups = result["data"]["summary"]["groups"].as_array().unwrap();
    assert!(groups.iter().any(|group| {
        group["escalation"] == "browser_degraded"
            && group["severity"] == "warning"
            && group["count"] == 1
            && group["recommendedAction"] == "Inspect browser health."
    }));
    assert!(groups.iter().any(|group| {
        group["escalation"] == "os_degraded_possible"
            && group["severity"] == "critical"
            && group["count"] == 1
            && group["recommendedAction"] == "Inspect the host OS."
    }));
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incidents_returns_incident_by_id_with_related_records() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incidents", "id" : "svc-incidents-3", "incidentId" :
        "browser-1", "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser recovered", "browserId" : "browser-1" }], "jobs" : { "job-1" : { "id" :
        "job-1", "action" : "navigate", "state" : "cancelled", "submittedAt" :
        "2026-04-22T00:01:00Z" } }, "incidents" : [{ "id" : "browser-1", "browserId" :
        "browser-1", "label" : "browser-1", "state" : "active", "latestTimestamp" :
        "2026-04-22T00:01:00Z", "latestMessage" : "navigate was cancelled", "latestKind"
        : "service_job_cancelled", "eventIds" : ["event-1"], "jobIds" : ["job-1"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_incidents_response_contract(&result["data"]);
    assert_eq!(result["data"]["incident"]["id"], "browser-1");
    assert_eq!(result["data"]["events"][0]["id"], "event-1");
    assert_eq!(result["data"]["jobs"][0]["id"], "job-1");
    assert_eq!(result["data"]["count"], 1);
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_incident_activity_returns_normalized_timeline() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_incident_activity", "id" : "svc-activity-1", "incidentId" :
        "browser-1", "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser browser-1 health changed from Ready to ProcessExited", "browserId" :
        "browser-1" }, { "id" : "event-2", "timestamp" : "2026-04-22T00:02:00Z", "kind" :
        "incident_acknowledged", "message" : "Incident browser-1 acknowledged",
        "browserId" : "browser-1", "details" : { "incidentId" : "browser-1", "actor" :
        "operator", "action" : "acknowledged", "note" : "triaged" } }], "jobs" : {
        "job-1" : { "id" : "job-1", "action" : "navigate", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "target" : { "browser" : "browser-1" }, "state" : "timed_out", "submittedAt" :
        "2026-04-22T00:01:00Z", "error" : "Timed out after 30000 ms" } }, "browsers" : {
        "browser-1" : { "id" : "browser-1", "profileId" : "work", "activeSessionIds" :
        ["session-1"] } }, "sessions" : { "session-1" : { "id" : "session-1",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "profileId" : "work", "browserIds" : ["browser-1"],
        "browserCapabilityLaunch" : { "applied" : true, "reason" :
        "validated_binding_applied", "browserBuild" : "stealthcdp_chromium", "bindingId"
        : "binding-1", "hostId" : "local", "executableId" : "stealth-current",
        "capabilityId" : "stealth-cdp", "executablePath" :
        "/opt/chromium-stealthcdp/chrome" } } }, "incidents" : [{ "id" : "browser-1",
        "browserId" : "browser-1", "label" : "browser-1", "state" : "active",
        "acknowledgedAt" : "2026-04-22T00:02:00Z", "acknowledgedBy" : "operator",
        "resolvedAt" : "2026-04-22T00:03:00Z", "resolvedBy" : "operator",
        "resolutionNote" : "handled", "latestTimestamp" : "2026-04-22T00:03:00Z",
        "latestMessage" : "Handled", "latestKind" : "service_job_timeout", "eventIds" :
        ["event-1", "event-2"], "jobIds" : ["job-1"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["incident"]["id"], "browser-1");
    assert_service_incident_activity_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 4);
    assert_eq!(
        result["data"]["activity"][0]["kind"],
        "browser_health_changed"
    );
    assert_eq!(result["data"]["activity"][1]["kind"], "service_job_timeout");
    assert_eq!(result["data"]["activity"][1]["browserId"], "browser-1");
    assert_eq!(result["data"]["activity"][1]["profileId"], "work");
    assert_eq!(result["data"]["activity"][1]["sessionId"], "session-1");
    assert_eq!(
        result["data"]["activity"][1]["serviceName"],
        "JournalDownloader"
    );
    assert_eq!(result["data"]["activity"][1]["agentName"], "codex");
    assert_eq!(result["data"]["activity"][1]["taskName"], "probeACSwebsite");
    assert_eq!(
        result["data"]["activity"][2]["kind"],
        "incident_acknowledged"
    );
    assert_eq!(result["data"]["activity"][2]["source"], "event");
    assert_eq!(result["data"]["activity"][3]["kind"], "incident_resolved");
    assert_eq!(result["data"]["activity"][3]["source"], "metadata");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_trace_returns_related_records_and_activity() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_trace", "id" : "svc-trace-1", "serviceName" :
        "JournalDownloader", "taskName" : "probeACSwebsite", "profileId" : "work",
        "serviceState" : { "events" : [{ "id" : "event-1", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser failed", "browserId" : "browser-1", "profileId" : "work", "sessionId" :
        "session-1", "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "probeACSwebsite" }, { "id" : "event-wait-started", "timestamp" :
        "2026-04-22T00:00:30Z", "kind" : "profile_lease_wait_started", "message" :
        "Service job job-1 started waiting for profile lease work", "profileId" : "work",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "details" : { "jobId" : "job-1", "outcome" : "started",
        "profileId" : "work", "conflictSessionIds" : ["active-session"], "retryAfterMs" :
        50 } }, { "id" : "event-wait-ended", "timestamp" : "2026-04-22T00:01:30Z", "kind"
        : "profile_lease_wait_ended", "message" :
        "Service job job-1 ended profile lease wait for work with outcome ready",
        "profileId" : "work", "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "probeACSwebsite", "details" : { "jobId" : "job-1", "outcome" :
        "ready", "profileId" : "work", "conflictSessionIds" : ["active-session"],
        "retryAfterMs" : 50, "waitedMs" : 60000 } }, { "id" : "event-2", "timestamp" :
        "2026-04-22T00:01:00Z", "kind" : "browser_health_changed", "message" :
        "Wrong task", "browserId" : "browser-1", "profileId" : "work", "sessionId" :
        "session-1", "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "otherTask" }], "jobs" : { "job-1" : { "id" : "job-1", "action" :
        "navigate", "state" : "timed_out", "target" : { "browser" : "browser-1" },
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "siteId" : "acs", "targetServiceIds" : ["acs", "google"],
        "displayIsolation" : "private_virtual_display", "namingWarnings" :
        service_job_naming_warning_values(), "hasNamingWarning" : true, "submittedAt" :
        "2026-04-22T00:02:00Z", "error" : "Timed out" }, "job-2" : { "id" : "job-2",
        "action" : "navigate", "state" : "timed_out", "target" : { "profile" : "other" },
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "displayIsolation" : "shared_display", "submittedAt" :
        "2026-04-22T00:03:00Z", "error" : "Wrong profile" } }, "browsers" : { "browser-1"
        : { "id" : "browser-1", "profileId" : "work", "activeSessionIds" : ["session-1"]
        } }, "sessions" : { "session-1" : { "id" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "profileId" : "work", "browserIds" : ["browser-1"], "browserCapabilityLaunch" : {
        "applied" : true, "reason" : "validated_binding_applied", "browserBuild" :
        "stealthcdp_chromium", "bindingId" : "binding-1", "hostId" : "local",
        "executableId" : "stealth-current", "capabilityId" : "stealth-cdp",
        "executablePath" : "/opt/chromium-stealthcdp/chrome" } } }, "incidents" : [{ "id"
        : "browser-1", "browserId" : "browser-1", "label" : "browser-1", "state" :
        "active", "severity" : "error", "escalation" : "browser_recovery",
        "recommendedAction" :
        "Review recovery trace and retry or relaunch the affected browser.",
        "latestTimestamp" : "2026-04-22T00:02:00Z", "latestMessage" : "Timed out",
        "latestKind" : "service_job_timeout", "currentHealth" : "process_exited",
        "eventIds" : ["event-1"], "jobIds" : ["job-1"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["counts"]["events"], 3);
    assert_eq!(result["data"]["counts"]["jobs"], 1);
    assert_eq!(result["data"]["counts"]["incidents"], 1);
    assert_eq!(result["data"]["counts"]["activity"], 2);
    assert_eq!(result["data"]["events"][0]["id"], "event-1");
    assert_service_event_record_contract(&result["data"]["events"][0]);
    assert_eq!(result["data"]["jobs"][0]["id"], "job-1");
    assert_service_job_naming_warning_contract(&result["data"]["jobs"][0]);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-1");
    assert_service_incident_record_contract(&result["data"]["incidents"][0]);
    assert_service_trace_response_contract(&result["data"]);
    assert_eq!(result["data"]["activity"][1]["jobId"], "job-1");
    assert_service_trace_activity_record_contract(&result["data"]["activity"][1]);
    assert_service_trace_summary_record_contract(&result["data"]["summary"]);
    assert_eq!(result["data"]["summary"]["contextCount"], 3);
    assert_eq!(result["data"]["summary"]["hasTraceContext"], true);
    assert_eq!(result["data"]["summary"]["namingWarningCount"], 1);
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["count"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["appliedCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["launches"][0]["browserBuild"],
        "stealthcdp_chromium"
    );
    assert_eq!(
        result["data"]["summary"]["browserCapabilityLaunches"]["launches"][0]["reason"],
        "validated_binding_applied"
    );
    assert_eq!(result["data"]["summary"]["displayAllocations"]["count"], 1);
    assert_eq!(
        result["data"]["summary"]["displayAllocations"]["recordedCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["displayAllocations"]["privateVirtualDisplayCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["displayAllocations"]["allocations"][0]["displayIsolation"],
        "private_virtual_display"
    );
    assert_eq!(result["data"]["summary"]["profileLeaseWaits"]["count"], 1);
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["completedCount"],
        1
    );
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["waits"][0]["jobId"],
        "job-1"
    );
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["waits"][0]["outcome"],
        "ready"
    );
    assert_eq!(
        result["data"]["summary"]["profileLeaseWaits"]["waits"][0]["waitedMs"],
        60000
    );
    let owned_context = result["data"]["summary"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| {
            context["serviceName"] == "JournalDownloader"
                && context["agentName"] == "codex"
                && context["taskName"] == "probeACSwebsite"
                && context["browserId"] == "browser-1"
                && context["profileId"] == "work"
                && context["sessionId"] == "session-1"
        })
        .expect("trace summary should include the owned service context");
    assert_eq!(owned_context["eventCount"], 1);
    assert_eq!(owned_context["jobCount"], 1);
    assert_eq!(owned_context["activityCount"], 2);
    assert_eq!(owned_context["targetIdentityCount"], 2);
    assert_eq!(owned_context["targetServiceIds"], json!(["acs", "google"]));
    assert_eq!(
        owned_context["displayAllocations"],
        json!(["private_virtual_display"])
    );
    assert_eq!(
        owned_context["unrecordedDisplayAllocationJobCount"],
        json!(0)
    );
    assert_eq!(owned_context["hasNamingWarning"], false);
    assert_eq!(owned_context["namingWarnings"].as_array().unwrap().len(), 0);
    assert_eq!(owned_context["attention"]["required"], false);
    assert_eq!(owned_context["attention"]["reason"], "none");
    assert_eq!(owned_context["latestTimestamp"], "2026-04-22T00:02:00Z");
    let wait_context = result["data"]["summary"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| {
            context["serviceName"] == "JournalDownloader"
                && context["agentName"] == "codex"
                && context["taskName"] == "probeACSwebsite"
                && context["profileId"] == "work"
                && context["browserId"].is_null()
                && context["sessionId"].is_null()
        })
        .expect("trace summary should include a profile lease wait context");
    assert_eq!(wait_context["eventCount"], 2);
    assert_eq!(wait_context["hasNamingWarning"], false);
    let incident_context = result["data"]["summary"]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|context| context["incidentCount"] == 1)
        .expect("trace summary should include incident-only context");
    assert_eq!(incident_context["hasNamingWarning"], true);
    assert_eq!(incident_context["attention"]["required"], true);
    assert_eq!(incident_context["attention"]["owner"], "operator");
    assert_eq!(incident_context["attention"]["reason"], "incidents_present");
    assert_eq!(
        incident_context["namingWarnings"],
        json!([
            "missing_service_name",
            "missing_agent_name",
            "missing_task_name"
        ])
    );
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_trace_returns_browser_recovery_sequence() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_trace", "id" : "svc-trace-recovery-1", "browserId" :
        "browser-1", "serviceName" : "JournalDownloader", "taskName" : "probeACSwebsite",
        "serviceState" : { "events" : [{ "id" : "event-stale", "timestamp" :
        "2026-04-22T00:00:00Z", "kind" : "browser_health_changed", "message" :
        "Browser browser-1 health changed from Ready to ProcessExited", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "previousHealth" : "ready", "currentHealth" : "process_exited", "details" : {
        "currentReasonKind" : "process_exited" } }, { "id" : "event-recovery",
        "timestamp" : "2026-04-22T00:00:01Z", "kind" : "browser_recovery_started",
        "message" : "Browser browser-1 recovery started", "browserId" : "browser-1",
        "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "currentHealth" : "process_exited", "details" : { "reasonKind" :
        "process_exited", "reason" :
        "Active browser PID 1234 exited before command dispatch" } }, { "id" :
        "event-ready", "timestamp" : "2026-04-22T00:00:02Z", "kind" :
        "browser_health_changed", "message" :
        "Browser browser-1 health changed from ProcessExited to Ready", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "previousHealth" : "process_exited", "currentHealth" : "ready" }], "incidents" :
        [{ "id" : "browser-1", "browserId" : "browser-1", "label" : "browser-1", "state"
        : "recovered", "latestTimestamp" : "2026-04-22T00:00:02Z", "latestMessage" :
        "Browser browser-1 health changed from ProcessExited to Ready", "latestKind" :
        "browser_health_changed", "eventIds" : ["event-stale", "event-ready"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["counts"]["events"], 3);
    assert_eq!(
        result["data"]["events"][0]["kind"],
        "browser_health_changed"
    );
    assert_eq!(
        result["data"]["events"][0]["currentHealth"],
        "process_exited"
    );
    assert_eq!(
        result["data"]["events"][0]["details"]["currentReasonKind"],
        "process_exited"
    );
    assert_eq!(
        result["data"]["events"][1]["kind"],
        "browser_recovery_started"
    );
    assert_eq!(
        result["data"]["events"][1]["details"]["reason"],
        "Active browser PID 1234 exited before command dispatch"
    );
    assert_eq!(
        result["data"]["events"][1]["details"]["reasonKind"],
        "process_exited"
    );
    assert_eq!(
        result["data"]["events"][2]["kind"],
        "browser_health_changed"
    );
    assert_eq!(result["data"]["events"][2]["currentHealth"], "ready");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_trace_filters_browser_recovery_override_events() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_trace", "id" : "svc-trace-recovery-override-1",
        "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
        "probeACSwebsite", "serviceState" : { "events" : [{ "id" : "event-override",
        "timestamp" : "2026-04-22T00:00:03Z", "kind" : "browser_recovery_override",
        "message" : "Browser browser-1 recovery retry enabled by operator", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "previousHealth" : "faulted", "currentHealth" : "process_exited", "details" : {
        "actor" : "operator", "action" : "retry_enabled" } }, { "id" :
        "event-other-task", "timestamp" : "2026-04-22T00:00:04Z", "kind" :
        "browser_recovery_override", "message" :
        "Browser browser-1 recovery retry enabled by operator", "browserId" :
        "browser-1", "profileId" : "work", "sessionId" : "session-1", "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "otherTask",
        "previousHealth" : "faulted", "currentHealth" : "process_exited", "details" : {
        "actor" : "operator", "action" : "retry_enabled" } }], "incidents" : [{ "id" :
        "browser-1", "browserId" : "browser-1", "label" : "browser-1", "state" :
        "active", "latestTimestamp" : "2026-04-22T00:00:03Z", "latestMessage" :
        "Browser browser-1 recovery retry enabled by operator", "latestKind" :
        "browser_recovery_override", "eventIds" : ["event-override"] }] } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["counts"]["events"], 1);
    assert_eq!(result["data"]["events"][0]["id"], "event-override");
    assert_eq!(
        result["data"]["events"][0]["kind"],
        "browser_recovery_override"
    );
    assert_eq!(
        result["data"]["events"][0]["details"]["action"],
        "retry_enabled"
    );
    assert_eq!(result["data"]["counts"]["incidents"], 1);
    assert_eq!(result["data"]["incidents"][0]["id"], "browser-1");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_config_actions_mutate_persisted_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-config-actions-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    let mut state = DaemonState::new();
    let upsert_profile = execute_command(
        &json!(
            { "action" : "service_profile_upsert", "id" : "svc-profile-upsert-1",
            "profileId" : "journal-downloader", "profile" : { "name" :
            "Journal Downloader", "allocation" : "per_service", "keyring" :
            "basic_password_store", "persistent" : true, "sharedServiceIds" :
            ["JournalDownloader"] } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_profile["success"], true);
    assert_service_profile_upsert_response_contract(&upsert_profile["data"]);
    assert_eq!(
        upsert_profile["data"]["profile"]["id"],
        "journal-downloader"
    );
    let freshness = execute_command(
        &json!(
            { "action" : "service_profile_freshness_update", "id" :
            "svc-profile-freshness-1", "profileId" : "journal-downloader",
            "freshness" : { "loginId" : "google", "readinessState" : "fresh",
            "readinessEvidence" : "auth_probe_cookie_present", "lastVerifiedAt" :
            "2026-05-06T12:00:00Z", "freshnessExpiresAt" : "2026-05-06T13:00:00Z" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(freshness["success"], true);
    assert_service_profile_upsert_response_contract(&freshness["data"]);
    assert_eq!(
        freshness["data"]["profile"]["targetReadiness"][0]["state"],
        "fresh"
    );
    assert_eq!(
        freshness["data"]["profile"]["authenticatedServiceIds"][0],
        "google"
    );
    let handoff = execute_command(
        &json!(
            { "action" : "service_profile_seeding_handoff_update", "id" :
            "svc-profile-seeding-handoff-1", "profileId" : "journal-downloader",
            "handoff" : { "targetServiceId" : "google", "state" :
            "seeding_launched_detached", "pid" : 1234, "startedAt" :
            "2026-05-10T12:00:00Z", "expiresAt" : "2026-05-10T12:30:00Z", "actor" :
            "operator", "note" : "manual seeding started" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(handoff["success"], true);
    assert_eq!(
        handoff["data"]["handoff"]["id"],
        "journal-downloader:google"
    );
    assert_eq!(
        handoff["data"]["seedingHandoff"]["operatorIntervention"]["state"],
        "seeding_launched_detached"
    );
    assert_eq!(
        store.load().unwrap().profile_seeding_handoffs["journal-downloader:google"].state,
        ProfileSeedingHandoffState::SeedingLaunchedDetached
    );
    let upsert_session = execute_command(
        &json!(
            { "action" : "service_session_upsert", "id" : "svc-session-upsert-1",
            "sessionId" : "journal-run", "session" : { "serviceName" :
            "JournalDownloader", "agentName" : "codex", "taskName" :
            "probeACSwebsite", "profileId" : "journal-downloader", "lease" :
            "exclusive", "cleanup" : "close_browser" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_session["success"], true);
    assert_service_session_upsert_response_contract(&upsert_session["data"]);
    assert_eq!(upsert_session["data"]["session"]["id"], "journal-run");
    let upsert_policy = execute_command(
        &json!(
            { "action" : "service_site_policy_upsert", "id" : "svc-policy-upsert-1",
            "sitePolicyId" : "google", "sitePolicy" : { "originPattern" :
            "https://accounts.google.com", "interactionMode" : "human_like_input" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_policy["success"], true);
    assert_service_site_policy_upsert_response_contract(&upsert_policy["data"]);
    assert_eq!(upsert_policy["data"]["sitePolicy"]["id"], "google");
    let upsert_provider = execute_command(
        &json!(
            { "action" : "service_provider_upsert", "id" : "svc-provider-upsert-1",
            "providerId" : "manual", "provider" : { "kind" : "manual_approval",
            "displayName" : "Dashboard approval", "capabilities" : ["human_approval"]
            } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_provider["success"], true);
    assert_service_provider_upsert_response_contract(&upsert_provider["data"]);
    assert_eq!(upsert_provider["data"]["provider"]["id"], "manual");
    let upsert_browser_capability = execute_command(
        &json!(
            { "action" : "service_browser_capability_registry_upsert", "id" :
            "svc-browser-capability-upsert-1", "collection" : "browserHosts",
            "recordId" : "local-linux", "record" : { "name" : "Local Linux host",
            "serviceName" : "JournalDownloader" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_browser_capability["success"], true);
    assert_service_browser_capability_registry_upsert_response_contract(
        &upsert_browser_capability["data"],
    );
    assert_eq!(
        upsert_browser_capability["data"]["record"]["id"],
        "local-linux"
    );
    assert_eq!(
        upsert_browser_capability["data"]["counts"]["browserHosts"],
        1
    );
    let upsert_monitor = execute_command(
        &json!(
            { "action" : "service_monitor_upsert", "id" : "svc-monitor-upsert-1",
            "monitorId" : "google-login-freshness", "monitor" : { "name" :
            "Google login freshness", "target" : { "site_policy" : "google" },
            "intervalMs" : 60000, "state" : "paused" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_monitor["success"], true);
    assert_service_monitor_upsert_response_contract(&upsert_monitor["data"]);
    assert_eq!(
        upsert_monitor["data"]["monitor"]["id"],
        "google-login-freshness"
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.profiles["journal-downloader"].shared_service_ids,
        vec!["JournalDownloader".to_string()]
    );
    assert_eq!(
        persisted.sessions["journal-run"].service_name.as_deref(),
        Some("JournalDownloader")
    );
    assert_eq!(
        persisted.site_policies["google"].origin_pattern,
        "https://accounts.google.com"
    );
    assert_eq!(
        persisted.providers["manual"].display_name,
        "Dashboard approval"
    );
    assert_eq!(
        persisted.browser_capability_registry.browser_hosts[0]["id"],
        "local-linux"
    );
    assert_eq!(
        persisted.monitors["google-login-freshness"].name,
        "Google login freshness"
    );
    let resume_monitor = execute_command(
        &json!(
            { "action" : "service_monitor_resume", "id" : "svc-monitor-resume-1",
            "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(resume_monitor["success"], true);
    assert_service_monitor_state_response_contract(&resume_monitor["data"]);
    assert_eq!(resume_monitor["data"]["state"], "active");
    assert_eq!(
        store.load().unwrap().monitors["google-login-freshness"].state,
        MonitorState::Active
    );
    let pause_monitor = execute_command(
        &json!(
            { "action" : "service_monitor_pause", "id" : "svc-monitor-pause-1",
            "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(pause_monitor["success"], true);
    assert_service_monitor_state_response_contract(&pause_monitor["data"]);
    assert_eq!(pause_monitor["data"]["state"], "paused");
    let reset_monitor = execute_command(
        &json!(
            { "action" : "service_monitor_reset_failures", "id" :
            "svc-monitor-reset-1", "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(reset_monitor["success"], true);
    assert_service_monitor_state_response_contract(&reset_monitor["data"]);
    assert_eq!(reset_monitor["data"]["resetFailures"], true);
    assert_eq!(reset_monitor["data"]["monitor"]["consecutiveFailures"], 0);
    let mut persisted = store.load().unwrap();
    let monitor = persisted
        .monitors
        .get_mut("google-login-freshness")
        .expect("monitor should exist");
    monitor.state = MonitorState::Faulted;
    monitor.consecutive_failures = 2;
    monitor.last_result = Some("site_policy_missing".to_string());
    persisted.events.push(ServiceEvent {
        id: "event-google-login-freshness-failed".to_string(),
        timestamp: "2026-04-22T00:00:00Z".to_string(),
        kind: ServiceEventKind::ReconciliationError,
        message: "Service monitor google-login-freshness failed".to_string(),
        details: Some(json!(
            { "incidentId" : "monitor:google-login-freshness", "monitorId" :
            "google-login-freshness", "monitorResult" : "site_policy_missing",
            "monitorTarget" : { "site_policy" : "google" } }
        )),
        ..ServiceEvent::default()
    });
    persisted.refresh_derived_views();
    store.save(&persisted).unwrap();
    let triage_monitor = execute_command(
        &json!(
            { "action" : "service_monitor_triage", "id" : "svc-monitor-triage-1",
            "monitorId" : "google-login-freshness", "by" : "operator", "note" :
            "reviewed" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(triage_monitor["success"], true);
    assert_service_monitor_triage_response_contract(&triage_monitor["data"]);
    assert_eq!(triage_monitor["data"]["acknowledged"], true);
    assert_eq!(
        triage_monitor["data"]["incident"]["monitorId"],
        "google-login-freshness"
    );
    assert_eq!(
        triage_monitor["data"]["incident"]["acknowledgedBy"],
        "operator"
    );
    assert_eq!(triage_monitor["data"]["monitor"]["consecutiveFailures"], 0);
    let mut persisted = store.load().unwrap();
    let monitor = persisted
        .monitors
        .get_mut("google-login-freshness")
        .expect("monitor should exist");
    monitor.state = MonitorState::Faulted;
    monitor.consecutive_failures = 3;
    monitor.last_result = Some("site_policy_missing".to_string());
    persisted.events.push(ServiceEvent {
        id: "event-google-login-freshness-failed-again".to_string(),
        timestamp: "2026-04-22T00:05:00Z".to_string(),
        kind: ServiceEventKind::ReconciliationError,
        message: "Service monitor google-login-freshness failed again".to_string(),
        details: Some(json!(
            { "incidentId" : "monitor:google-login-freshness", "monitorId" :
            "google-login-freshness", "monitorResult" : "site_policy_missing",
            "monitorTarget" : { "site_policy" : "google" } }
        )),
        ..ServiceEvent::default()
    });
    persisted.refresh_derived_views();
    store.save(&persisted).unwrap();
    let apply_remedies = execute_command(
        &json!(
            { "action" : "service_remedies_apply", "id" : "svc-remedies-apply-1",
            "escalation" : "monitor_attention", "by" : "operator", "note" :
            "reviewed group" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(apply_remedies["success"], true);
    assert_service_remedies_apply_response_contract(&apply_remedies["data"]);
    assert_eq!(apply_remedies["data"]["count"], 1);
    assert_eq!(
        apply_remedies["data"]["monitorIds"],
        json!(["google-login-freshness"])
    );
    let delete_session = execute_command(
        &json!(
            { "action" : "service_session_delete", "id" : "svc-session-delete-1",
            "sessionId" : "journal-run" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_session["success"], true);
    assert_service_session_delete_response_contract(&delete_session["data"]);
    assert_eq!(delete_session["data"]["deleted"], true);
    assert!(!store.load().unwrap().sessions.contains_key("journal-run"));
    let delete_profile = execute_command(
        &json!(
            { "action" : "service_profile_delete", "id" : "svc-profile-delete-1",
            "profileId" : "journal-downloader" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_profile["success"], true);
    assert_service_profile_delete_response_contract(&delete_profile["data"]);
    assert_eq!(delete_profile["data"]["deleted"], true);
    assert!(!store
        .load()
        .unwrap()
        .profiles
        .contains_key("journal-downloader"));
    let delete_provider = execute_command(
        &json!(
            { "action" : "service_provider_delete", "id" : "svc-provider-delete-1",
            "providerId" : "manual" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_provider["success"], true);
    assert_service_provider_delete_response_contract(&delete_provider["data"]);
    assert_eq!(delete_provider["data"]["deleted"], true);
    assert!(!store.load().unwrap().providers.contains_key("manual"));
    let delete_monitor = execute_command(
        &json!(
            { "action" : "service_monitor_delete", "id" : "svc-monitor-delete-1",
            "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_monitor["success"], true);
    assert_service_monitor_delete_response_contract(&delete_monitor["data"]);
    assert_eq!(delete_monitor["data"]["deleted"], true);
    assert!(!store
        .load()
        .unwrap()
        .monitors
        .contains_key("google-login-freshness"));
    let delete_policy = execute_command(
        &json!(
            { "action" : "service_site_policy_delete", "id" : "svc-policy-delete-1",
            "sitePolicyId" : "google" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_policy["success"], true);
    assert_service_site_policy_delete_response_contract(&delete_policy["data"]);
    assert_eq!(delete_policy["data"]["deleted"], true);
    let loaded_after_policy_delete = store.load().unwrap();
    assert_eq!(
        loaded_after_policy_delete.site_policy_source("google"),
        Some(crate::native::service_model::ServiceEntitySource::Builtin)
    );
    assert_eq!(
        loaded_after_policy_delete.site_policies["google"].origin_pattern,
        "https://accounts.google.com"
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_job_cancel_response_matches_contract() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-job-cancel-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            jobs: BTreeMap::from([(
                "job-queued".to_string(),
                ServiceJob {
                    id: "job-queued".to_string(),
                    action: "navigate".to_string(),
                    state: JobState::Queued,
                    submitted_at: Some("2026-04-22T00:00:00Z".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_job_cancel", "id" : "svc-job-cancel-1", "jobId" :
            "job-queued", "reason" : "stale" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_job_cancel_response_contract(&result["data"]);
    assert_eq!(result["data"]["cancelled"], true);
    assert_eq!(result["data"]["job"]["state"], "cancelled");
    assert_eq!(result["data"]["job"]["error"], "stale");
    assert_eq!(
        store.load().unwrap().jobs["job-queued"].state,
        JobState::Cancelled
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_reconcile_response_matches_contract() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-reconcile-response-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_reconcile", "id" : "svc-reconcile-response-1",
            "serviceState" : ServiceState::default(), }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_reconcile_response_contract(&result["data"]);
    assert_eq!(result["data"]["reconciled"], true);
    let _ = fs::remove_dir_all(&home);
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
#[tokio::test]
async fn test_service_jobs_returns_limited_jobs() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_jobs", "id" : "svc-jobs-1", "limit" : 1, "serviceState" : {
        "jobs" : { "job-1" : { "id" : "job-1", "action" : "navigate", "state" :
        "succeeded", "submittedAt" : "2026-04-22T00:00:00Z" }, "job-2" : { "id" :
        "job-2", "action" : "click", "state" : "failed", "submittedAt" :
        "2026-04-22T00:01:00Z" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 2);
    assert_eq!(result["data"]["total"], 2);
    assert_eq!(result["data"]["jobs"][0]["id"], "job-2");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_jobs_filters_by_state_action_and_since() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_jobs", "id" : "svc-jobs-2", "state" :
        "waiting_profile_lease", "jobAction" : "navigate", "profileId" : "work",
        "sessionId" : "session-1", "serviceName" : "JournalDownloader", "agentName" :
        "codex", "taskName" : "probeACSwebsite", "since" : "2026-04-22T00:01:00Z",
        "serviceState" : { "sessions" : { "session-1" : { "id" : "session-1", "profileId"
        : "work", "browserIds" : ["browser-1"] } }, "browsers" : { "browser-1" : { "id" :
        "browser-1", "profileId" : "work", "activeSessionIds" : ["session-1"] } }, "jobs"
        : { "job-1" : { "id" : "job-1", "action" : "navigate", "state" :
        "waiting_profile_lease", "target" : { "browser" : "browser-1" }, "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "submittedAt" : "2026-04-22T00:00:00Z" }, "job-2" : { "id" : "job-2", "action" :
        "navigate", "state" : "waiting_profile_lease", "target" : { "browser" :
        "browser-1" }, "serviceName" : "JournalDownloader", "agentName" : "codex",
        "taskName" : "probeACSwebsite", "submittedAt" : "2026-04-22T00:01:00Z" }, "job-3"
        : { "id" : "job-3", "action" : "click", "state" : "failed", "target" : {
        "browser" : "browser-1" }, "serviceName" : "JournalDownloader", "agentName" :
        "codex", "taskName" : "probeACSwebsite", "submittedAt" : "2026-04-22T00:02:00Z"
        }, "job-4" : { "id" : "job-4", "action" : "navigate", "state" : "succeeded",
        "target" : { "browser" : "browser-1" }, "serviceName" : "JournalDownloader",
        "agentName" : "codex", "taskName" : "probeACSwebsite", "submittedAt" :
        "2026-04-22T00:03:00Z" }, "job-5" : { "id" : "job-5", "action" : "navigate",
        "state" : "failed", "target" : { "profile" : "other" }, "serviceName" :
        "JournalDownloader", "agentName" : "codex", "taskName" : "probeACSwebsite",
        "submittedAt" : "2026-04-22T00:04:00Z" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["matched"], 1);
    assert_eq!(result["data"]["total"], 5);
    assert_eq!(result["data"]["jobs"][0]["id"], "job-2");
    assert!(state.browser.is_none());
}
#[tokio::test]
async fn test_service_jobs_returns_job_by_id() {
    let mut state = DaemonState::new();
    let cmd = json!(
        { "action" : "service_jobs", "id" : "svc-jobs-3", "jobId" : "job-2",
        "serviceState" : { "jobs" : { "job-1" : { "id" : "job-1", "action" : "navigate",
        "state" : "succeeded", "submittedAt" : "2026-04-22T00:00:00Z" }, "job-2" : { "id"
        : "job-2", "action" : "click", "state" : "failed", "submittedAt" :
        "2026-04-22T00:01:00Z", "error" : "selector missing" } } } }
    );
    let result = execute_command(&cmd, &mut state).await;
    assert_eq!(result["success"], true);
    assert_service_jobs_response_contract(&result["data"]);
    assert_eq!(result["data"]["job"]["id"], "job-2");
    assert_eq!(result["data"]["jobs"][0]["id"], "job-2");
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["total"], 2);
    assert!(state.browser.is_none());
}
#[test]
fn test_remote_view_display_access_grant_timeout_is_typed() {
    let timeout_error =
        remote_view_display_access_grant_error("guacamole:1", ":11", 124, "hung xhost");
    assert!(timeout_error
        .starts_with("display_access_grant_timeout: route 'guacamole:1' display ':11'"));
    assert!(timeout_error.contains("helper exceeded"));
    assert!(timeout_error.contains("hung xhost"));
    let failed_error =
        remote_view_display_access_grant_error("guacamole:1", ":11", 1, "sudo failed");
    assert!(
        failed_error.starts_with("display_access_grant_failed: route 'guacamole:1' display ':11'")
    );
    assert!(failed_error.contains("helper exited with 1"));
    assert!(failed_error.contains("sudo failed"));
}
#[test]
fn test_remote_view_helper_status_contract_accepts_current_capabilities() {
    let report = json!(
        { "success" : true, "parsed" : { "schemaVersion" : 1, "helperVersion" :
        "2026-06-23.p44-route-desktop-v2", "routeDesktopSession" : { "ready" : true,
        "terminalStartupDetected" : false }, "displayAccess" : {
        "supportsFilesystemX11Socket" : true, "supportsAbstractX11Socket" : true,
        "boundedXhostTimeoutSeconds" : 2 } } }
    );
    assert!(remote_view_helper_status_contract_ready(&report));
}
#[test]
fn test_remote_view_helper_status_contract_rejects_missing_abstract_socket_support() {
    let report = json!(
        { "success" : true, "parsed" : { "schemaVersion" : 1, "helperVersion" :
        "2026-06-23.p44-route-desktop-v2", "routeDesktopSession" : { "ready" : true,
        "terminalStartupDetected" : false }, "displayAccess" : {
        "supportsFilesystemX11Socket" : true, "supportsAbstractX11Socket" : false,
        "boundedXhostTimeoutSeconds" : 2 } } }
    );
    assert!(!remote_view_helper_status_contract_ready(&report));
}
#[test]
fn test_persist_service_browser_record_round_trips() {
    let home = unique_socket_dir("service-browser-record-home");
    fs::create_dir_all(&home).unwrap();
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    persist_service_browser_record_in_repository(
        &repository,
        "persist-session",
        ServiceBrowserHost::LocalHeadless,
        ServiceBrowserHealth::Ready,
        Some(1234),
        Some("http://127.0.0.1:9222".to_string()),
        None,
        Some(ServiceLaunchMetadata {
            profile_id: Some("work".to_string()),
            profile_name: Some("Work".to_string()),
            user_data_dir: Some("/tmp/agent-browser-work".to_string()),
            persistent_profile: true,
            keyring: ProfileKeyringPolicy::RealOsKeychain,
            service_name: Some("JournalDownloader".to_string()),
            agent_name: Some("codex".to_string()),
            task_name: Some("probe-acs-website".to_string()),
            cleanup: SessionCleanupPolicy::Detach,
            profile_selection_reason: Some(ProfileSelectionReason::ExplicitProfile),
            browser_stderr_log_path: None,
            browser_capability_launch: Some(json!(
                { "applied" : true, "bindingId" : "test-binding", "reason" :
                "validated_binding_applied" }
            )),
            view_streams: Vec::new(),
            display_isolation: Some("shared_display".to_string()),
            display_name: Some(":93".to_string()),
            ..ServiceLaunchMetadata::default()
        }),
    )
    .unwrap();
    let state = store.load().unwrap();
    let browser = &state.browsers["session:persist-session"];
    assert_eq!(browser.host, ServiceBrowserHost::LocalHeadless);
    assert_eq!(browser.health, ServiceBrowserHealth::Ready);
    assert_eq!(browser.pid, Some(1234));
    assert_eq!(browser.display_isolation.as_deref(), Some("shared_display"));
    assert_eq!(browser.display_name.as_deref(), Some(":93"));
    assert_eq!(
        browser.cdp_endpoint.as_deref(),
        Some("http://127.0.0.1:9222")
    );
    assert_eq!(browser.profile_id.as_deref(), Some("work"));
    let profile = &state.profiles["work"];
    assert_eq!(profile.name, "Work");
    assert_eq!(
        profile.user_data_dir.as_deref(),
        Some("/tmp/agent-browser-work")
    );
    assert_eq!(profile.allocation, ProfileAllocationPolicy::PerService);
    assert_eq!(profile.keyring, ProfileKeyringPolicy::RealOsKeychain);
    assert!(profile.persistent);
    assert!(profile.manual_login_preferred);
    assert_eq!(profile.shared_service_ids, vec!["JournalDownloader"]);
    let session = &state.sessions["persist-session"];
    assert_eq!(session.service_name.as_deref(), Some("JournalDownloader"));
    assert_eq!(session.agent_name.as_deref(), Some("codex"));
    assert_eq!(session.task_name.as_deref(), Some("probe-acs-website"));
    assert_eq!(session.profile_id.as_deref(), Some("work"));
    assert_eq!(session.lease, LeaseState::Exclusive);
    assert_eq!(session.cleanup, SessionCleanupPolicy::Detach);
    assert_eq!(session.browser_ids, vec!["session:persist-session"]);
    assert_eq!(
        session.browser_capability_launch.as_ref().unwrap()["bindingId"],
        "test-binding"
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_persist_service_browser_record_clears_stale_view_streams_when_metadata_is_empty() {
    let home = unique_socket_dir("service-browser-record-clears-streams");
    fs::create_dir_all(&home).unwrap();
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    let view_stream = ViewStream {
        id: "remote-headed-view".to_string(),
        provider: ViewStreamProvider::RdpGateway,
        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
        url: Some("/guacamole/#/client/MQBjAHBvc3RncmVzcWw=".to_string()),
        frame_url: Some("/guacamole/#/client/MQBjAHBvc3RncmVzcWw=".to_string()),
        external_url: Some("/guacamole/#/client/MQBjAHBvc3RncmVzcWw=".to_string()),
        route_descriptor: None,
        route_id: None,
        display_allocation_id: None,
        connection_id: Some("MQBjAHBvc3RncmVzcWw=".to_string()),
        connection_name: None,
        route_source: Some("test_fixture".to_string()),
        provider_mode: None,
        viewer_lease_ids: Vec::new(),
        controller_lease_id: None,
        read_only: false,
        readiness: None,
        remote_readiness: None,
        attachability: None,
    };
    persist_service_browser_record_in_repository(
        &repository,
        "persist-session",
        ServiceBrowserHost::RemoteHeaded,
        ServiceBrowserHealth::Ready,
        Some(1234),
        Some("http://127.0.0.1:9222".to_string()),
        None,
        Some(ServiceLaunchMetadata {
            profile_id: Some("work".to_string()),
            view_streams: vec![view_stream],
            display_isolation: Some("shared_display".to_string()),
            display_name: Some(":10".to_string()),
            ..ServiceLaunchMetadata::default()
        }),
    )
    .unwrap();
    persist_service_browser_record_in_repository(
        &repository,
        "persist-session",
        ServiceBrowserHost::LocalHeadless,
        ServiceBrowserHealth::Ready,
        Some(5678),
        Some("http://127.0.0.1:9333".to_string()),
        None,
        Some(ServiceLaunchMetadata {
            profile_id: Some("work".to_string()),
            view_streams: Vec::new(),
            display_isolation: None,
            display_name: None,
            ..ServiceLaunchMetadata::default()
        }),
    )
    .unwrap();
    let state = store.load().unwrap();
    let browser = &state.browsers["session:persist-session"];
    assert_eq!(browser.host, ServiceBrowserHost::LocalHeadless);
    assert_eq!(browser.pid, Some(5678));
    assert!(browser.view_streams.is_empty());
    assert!(browser.display_isolation.is_none());
    assert!(browser.display_name.is_none());
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_recovery_policy_counts_attempts_since_ready() {
    let browser_id = "session:budget-session";
    let state = ServiceState {
        events: vec![
            ServiceEvent {
                id: "ready".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                browser_id: Some(browser_id.to_string()),
                current_health: Some(ServiceBrowserHealth::Ready),
                ..ServiceEvent::default()
            },
            ServiceEvent {
                id: "recovery-1".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
            ServiceEvent {
                id: "recovery-2".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
        ],
        ..ServiceState::default()
    };
    let policy = recovery_policy_for_next_attempt(
        &state,
        browser_id,
        BrowserRecoveryPolicyConfig::default(),
    );
    assert_eq!(policy.attempt, 3);
    assert_eq!(
        policy.retry_budget,
        BrowserRecoveryPolicyConfig::default().retry_budget
    );
    assert!(!policy.retry_budget_exceeded);
    assert_eq!(policy.next_retry_delay_ms, 4_000);
}
#[test]
fn test_recovery_policy_blocks_after_budget() {
    let browser_id = "session:budget-session";
    let state = ServiceState {
        events: vec![
            ServiceEvent {
                id: "recovery-1".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
            ServiceEvent {
                id: "recovery-2".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
            ServiceEvent {
                id: "recovery-3".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
        ],
        ..ServiceState::default()
    };
    let policy = recovery_policy_for_next_attempt(
        &state,
        browser_id,
        BrowserRecoveryPolicyConfig::default(),
    );
    assert_eq!(policy.attempt, 4);
    assert!(policy.retry_budget_exceeded);
    assert_eq!(policy.next_retry_delay_ms, 8_000);
}
#[test]
fn test_recovery_policy_uses_configured_budget_and_backoff() {
    let browser_id = "session:budget-session";
    let policy = BrowserRecoveryPolicyConfig {
        retry_budget: 2,
        base_backoff_ms: 250,
        max_backoff_ms: 1_000,
        source: BrowserRecoveryPolicySource::default(),
    };
    let state = ServiceState {
        events: vec![
            ServiceEvent {
                id: "recovery-1".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
            ServiceEvent {
                id: "recovery-2".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
        ],
        ..ServiceState::default()
    };
    let configured_policy = recovery_policy_for_next_attempt(&state, browser_id, policy);
    assert_eq!(configured_policy.attempt, 3);
    assert_eq!(configured_policy.retry_budget, 2);
    assert!(configured_policy.retry_budget_exceeded);
    assert_eq!(configured_policy.next_retry_delay_ms, 1_000);
}
#[test]
fn test_recovery_policy_resets_after_operator_override() {
    let browser_id = "session:budget-session";
    let state = ServiceState {
        events: vec![
            ServiceEvent {
                id: "recovery-1".to_string(),
                kind: ServiceEventKind::BrowserRecoveryStarted,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
            ServiceEvent {
                id: "override-1".to_string(),
                kind: ServiceEventKind::BrowserRecoveryOverride,
                browser_id: Some(browser_id.to_string()),
                ..ServiceEvent::default()
            },
        ],
        ..ServiceState::default()
    };
    let policy = recovery_policy_for_next_attempt(
        &state,
        browser_id,
        BrowserRecoveryPolicyConfig::default(),
    );
    assert_eq!(policy.attempt, 1);
    assert!(!policy.retry_budget_exceeded);
}
#[tokio::test]
async fn test_service_browser_retry_marks_faulted_browser_retryable() {
    let home = unique_socket_dir("service-browser-retry-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let browser_id = "session:retry-session";
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("work".to_string()),
                    health: ServiceBrowserHealth::Faulted,
                    active_session_ids: vec!["retry-session".to_string()],
                    last_error: Some("Browser recovery retry budget exceeded".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            events: vec![ServiceEvent {
                id: "event-faulted".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser faulted".to_string(),
                browser_id: Some(browser_id.to_string()),
                current_health: Some(ServiceBrowserHealth::Faulted),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        })
        .unwrap();
    let mut daemon_state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "retry-1", "action" : "service_browser_retry", "browserId" :
            browser_id, "by" : "operator", "note" : "manual retry approved",
            "serviceName" : "JournalDownloader", "agentName" : "codex", "taskName" :
            "probeACSwebsite" }
        ),
        &mut daemon_state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_browser_retry_response_contract(&result["data"]);
    assert_eq!(result["data"]["retryEnabled"], true);
    assert_eq!(result["data"]["browser"]["health"], "process_exited");
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.browsers[browser_id].health,
        ServiceBrowserHealth::ProcessExited
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserRecoveryOverride
            && event.browser_id.as_deref() == Some(browser_id)
            && event.service_name.as_deref() == Some("JournalDownloader")
            && event.agent_name.as_deref() == Some("codex")
            && event.task_name.as_deref() == Some("probeACSwebsite")
            && event
                .details
                .as_ref()
                .and_then(|details| details.get("actor"))
                == Some(&json!("operator"))
    }));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_remedies_apply_retries_os_degraded_browsers() {
    let home = unique_socket_dir("service-remedies-apply-os-degraded-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let browser_id = "session:os-degraded-session";
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("work".to_string()),
                    health: ServiceBrowserHealth::Faulted,
                    active_session_ids: vec!["os-degraded-session".to_string()],
                    last_error: Some("Runtime browser PID survived force kill".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            events: vec![ServiceEvent {
                id: "event-force-kill-failed".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Force kill failed".to_string(),
                browser_id: Some(browser_id.to_string()),
                current_health: Some(ServiceBrowserHealth::Faulted),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        })
        .unwrap();
    let mut daemon_state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "remedy-os-degraded-1", "action" : "service_remedies_apply",
            "escalation" : "os_degraded_possible", "by" : "operator", "note" :
            "host inspected", "serviceName" : "JournalDownloader", "agentName" :
            "codex", "taskName" : "probeACSwebsite" }
        ),
        &mut daemon_state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_remedies_apply_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["browserIds"], json!([browser_id]));
    assert_eq!(
        result["data"]["browserResults"][0]["browser"]["health"],
        "process_exited"
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.browsers[browser_id].health,
        ServiceBrowserHealth::ProcessExited
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserRecoveryOverride
            && event.browser_id.as_deref() == Some(browser_id)
            && event.service_name.as_deref() == Some("JournalDownloader")
            && event.agent_name.as_deref() == Some("codex")
            && event.task_name.as_deref() == Some("probeACSwebsite")
    }));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_remedies_apply_retries_degraded_browsers() {
    let home = unique_socket_dir("service-remedies-apply-browser-degraded-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let browser_id = "session:degraded-session";
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("work".to_string()),
                    health: ServiceBrowserHealth::Degraded,
                    active_session_ids: vec!["degraded-session".to_string()],
                    last_error: Some(
                        "Polite browser close failed; force kill was required".to_string(),
                    ),
                    ..BrowserProcess::default()
                },
            )]),
            events: vec![ServiceEvent {
                id: "event-polite-close-failed".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Polite close failed; force kill succeeded".to_string(),
                browser_id: Some(browser_id.to_string()),
                current_health: Some(ServiceBrowserHealth::Degraded),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        })
        .unwrap();
    let mut daemon_state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "remedy-browser-degraded-1", "action" :
            "service_remedies_apply", "escalation" : "browser_degraded", "by" :
            "operator", "note" : "force kill succeeded", "serviceName" :
            "JournalDownloader", "agentName" : "codex", "taskName" :
            "probeACSwebsite" }
        ),
        &mut daemon_state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_remedies_apply_response_contract(&result["data"]);
    assert_eq!(result["data"]["count"], 1);
    assert_eq!(result["data"]["browserIds"], json!([browser_id]));
    assert_eq!(
        result["data"]["browserResults"][0]["browser"]["health"],
        "process_exited"
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.browsers[browser_id].health,
        ServiceBrowserHealth::ProcessExited
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserRecoveryOverride
            && event.browser_id.as_deref() == Some(browser_id)
            && event.previous_health == Some(ServiceBrowserHealth::Degraded)
            && event.current_health == Some(ServiceBrowserHealth::ProcessExited)
            && event.service_name.as_deref() == Some("JournalDownloader")
            && event.agent_name.as_deref() == Some("codex")
            && event.task_name.as_deref() == Some("probeACSwebsite")
    }));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_browser_repair_retries_degraded_browser() {
    let home = unique_socket_dir("service-browser-repair-degraded-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let browser_id = "session:degraded-session";
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    health: ServiceBrowserHealth::Degraded,
                    active_session_ids: vec!["degraded-session".to_string()],
                    last_error: Some(
                        "Polite browser close failed; force kill was required".to_string(),
                    ),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut daemon_state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "service-browser-repair-1", "action" : "service_browser_repair",
            "browserId" : browser_id, "by" : "operator", "note" :
            "operator reviewed shutdown outcome", "serviceName" :
            "JournalDownloader", "agentName" : "codex", "taskName" :
            "probeACSwebsite" }
        ),
        &mut daemon_state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["repaired"], true);
    assert_eq!(result["data"]["browser"]["health"], "process_exited");
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.browsers[browser_id].health,
        ServiceBrowserHealth::ProcessExited
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserRecoveryOverride
            && event.browser_id.as_deref() == Some(browser_id)
            && event.service_name.as_deref() == Some("JournalDownloader")
            && event.agent_name.as_deref() == Some("codex")
            && event.task_name.as_deref() == Some("probeACSwebsite")
    }));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_browser_repair_rejects_ready_browser() {
    let home = unique_socket_dir("service-browser-repair-ready-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    let browser_id = "session:ready-session";
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["ready-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut daemon_state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "id" : "service-browser-repair-ready", "action" :
            "service_browser_repair", "browserId" : browser_id }
        ),
        &mut daemon_state,
    )
    .await;
    assert_eq!(result["success"], false);
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("is not degraded or faulted"));
    assert_eq!(
        store.load().unwrap().browsers[browser_id].health,
        ServiceBrowserHealth::Ready
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_browser_close_rejects_non_active_browser_without_launch() {
    let mut daemon_state = DaemonState::new();
    daemon_state.session_id = "active-session".to_string();
    let result = execute_command(
        &json!(
            { "id" : "service-browser-close-wrong", "action" :
            "service_browser_close", "browserId" : "session:other-session" }
        ),
        &mut daemon_state,
    )
    .await;
    assert_eq!(result["success"], false);
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("can only close the active service browser"));
    assert!(daemon_state.browser.is_none());
}
#[test]
fn test_retry_service_browser_in_repository_marks_faulted_browser_retryable() {
    let home = unique_socket_dir("service-browser-retry-repository-home");
    fs::create_dir_all(&home).unwrap();
    let browser_id = "session:retry-repository-session";
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("work".to_string()),
                    health: ServiceBrowserHealth::Faulted,
                    active_session_ids: vec!["retry-repository-session".to_string()],
                    last_error: Some("Browser recovery retry budget exceeded".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            events: vec![ServiceEvent {
                id: "event-faulted".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser faulted".to_string(),
                browser_id: Some(browser_id.to_string()),
                current_health: Some(ServiceBrowserHealth::Faulted),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        })
        .unwrap();
    let (retryable, incident) = retry_persisted_service_browser_in_repository(
        &repository,
        browser_id,
        "2026-04-22T01:00:00Z",
        "operator",
        Some("manual retry approved"),
        Some("JournalDownloader"),
        Some("codex"),
        Some("probeACSwebsite"),
    )
    .unwrap();
    assert_eq!(retryable.health, ServiceBrowserHealth::ProcessExited);
    assert_eq!(
        incident.as_ref().map(|incident| incident.id.as_str()),
        Some(browser_id)
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.browsers[browser_id].health,
        ServiceBrowserHealth::ProcessExited
    );
    assert!(persisted.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserRecoveryOverride
            && event.browser_id.as_deref() == Some(browser_id)
            && event.service_name.as_deref() == Some("JournalDownloader")
            && event.agent_name.as_deref() == Some("codex")
            && event.task_name.as_deref() == Some("probeACSwebsite")
            && event
                .details
                .as_ref()
                .and_then(|details| details.get("note"))
                == Some(&json!("manual retry approved"))
    }));
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_persisted_recovery_rehydrates_removed_terminal_browser_from_event_history() {
    let home = unique_socket_dir("service-recovery-history-home");
    fs::create_dir_all(&home).unwrap();
    let browser_id = "session:history-session";
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    store
        .save(&ServiceState {
            events: vec![ServiceEvent {
                id: "terminal-health".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                browser_id: Some(browser_id.to_string()),
                session_id: Some("history-session".to_string()),
                service_name: Some("HistoryService".to_string()),
                agent_name: Some("history-agent".to_string()),
                task_name: Some("recoverHistory".to_string()),
                previous_health: Some(ServiceBrowserHealth::Ready),
                current_health: Some(ServiceBrowserHealth::ProcessExited),
                details: Some(json!({ "currentError" : "Browser process 4242 exited",
                    "processExitPid" : 4242, })),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        })
        .unwrap();
    let result = persist_browser_recovery_started_in_repository(
        &repository,
        "history-session",
        BrowserRecoveryPolicyConfig::default(),
        "Browser relaunch requested from persisted unhealthy state",
    );
    assert_eq!(result, BrowserRecoveryPersistence::Recorded);
    let state = store.load().unwrap();
    assert_eq!(
        state.browsers[browser_id].health,
        ServiceBrowserHealth::ProcessExited
    );
    assert_eq!(state.browsers[browser_id].pid, Some(4242));
    let recovery = state
        .events
        .iter()
        .find(|event| event.kind == ServiceEventKind::BrowserRecoveryStarted)
        .unwrap();
    assert_eq!(recovery.browser_id.as_deref(), Some(browser_id));
    assert_eq!(recovery.service_name.as_deref(), Some("HistoryService"));
    assert_eq!(recovery.agent_name.as_deref(), Some("history-agent"));
    assert_eq!(recovery.task_name.as_deref(), Some("recoverHistory"));
    assert_eq!(
        recovery
            .details
            .as_ref()
            .and_then(|details| details.get("reasonKind"))
            .and_then(|reason| reason.as_str()),
        Some("process_exited")
    );
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_persisted_recovery_blocks_and_marks_browser_faulted_after_budget() {
    let home = unique_socket_dir("service-recovery-budget-home");
    fs::create_dir_all(&home).unwrap();
    let browser_id = "session:budget-session";
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    health: ServiceBrowserHealth::ProcessExited,
                    last_error: Some("Recorded browser PID 1234 is no longer running".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            events: (1..=BrowserRecoveryPolicyConfig::default().retry_budget)
                .map(|attempt| ServiceEvent {
                    id: format!("recovery-{attempt}"),
                    kind: ServiceEventKind::BrowserRecoveryStarted,
                    browser_id: Some(browser_id.to_string()),
                    ..ServiceEvent::default()
                })
                .collect(),
            ..ServiceState::default()
        })
        .unwrap();
    let result = persist_browser_recovery_started_in_repository(
        &repository,
        "budget-session",
        BrowserRecoveryPolicyConfig::default(),
        "Browser relaunch requested from persisted unhealthy state",
    );
    assert!(matches!(result, BrowserRecoveryPersistence::Blocked(_)));
    let state = store.load().unwrap();
    assert_eq!(
        state.browsers[browser_id].health,
        ServiceBrowserHealth::Faulted
    );
    assert!(state.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserHealthChanged
            && event.browser_id.as_deref() == Some(browser_id)
            && event.current_health == Some(ServiceBrowserHealth::Faulted)
    }));
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_current_stale_health_in_repository_records_recovery_started() {
    let home = unique_socket_dir("service-current-stale-health-repository");
    fs::create_dir_all(&home).unwrap();
    let browser_id = "session:stale-session";
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some("work".to_string()),
                    host: ServiceBrowserHost::LocalHeaded,
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["stale-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let result = persist_current_browser_stale_health_in_repository(
        &repository,
        "stale-session",
        Some(1234),
        Some("ws://127.0.0.1:9222/devtools/browser/stale".to_string()),
        BrowserRecoveryPolicyConfig::default(),
        ServiceBrowserHealth::CdpDisconnected,
        BrowserRecoveryReasonKind::CdpDisconnected,
        "CDP response channel closed".to_string(),
        Some(json!({ "failureClass" : "cdp_disconnected" })),
    );
    assert_eq!(result, BrowserRecoveryPersistence::Recorded);
    let state = store.load().unwrap();
    let browser = &state.browsers[browser_id];
    assert_eq!(browser.health, ServiceBrowserHealth::CdpDisconnected);
    assert_eq!(browser.pid, Some(1234));
    assert_eq!(
        browser.cdp_endpoint.as_deref(),
        Some("ws://127.0.0.1:9222/devtools/browser/stale")
    );
    assert_eq!(browser.profile_id.as_deref(), Some("work"));
    assert!(state.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserHealthChanged
            && event.browser_id.as_deref() == Some(browser_id)
    }));
    assert!(state.events.iter().any(|event| {
        event.kind == ServiceEventKind::BrowserRecoveryStarted
            && event.browser_id.as_deref() == Some(browser_id)
    }));
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_close_does_not_persist_not_started_browser_placeholder() {
    let home = unique_socket_dir("service-browser-close-home");
    fs::create_dir_all(&home).unwrap();
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_SESSION"]);
    guard.set("HOME", home.to_str().unwrap());
    guard.set("AGENT_BROWSER_SESSION", "close-session");
    let mut state = DaemonState::new();
    let result =
        execute_command(&json!({ "action" : "close", "id" : "close-1" }), &mut state).await;
    assert_eq!(result["success"], true);
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    let persisted = store.load().unwrap();
    assert!(!persisted.browsers.contains_key("session:close-session"));
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_close_health_event_marks_operator_requested_close() {
    let home = unique_socket_dir("service-browser-close-reason-home");
    fs::create_dir_all(&home).unwrap();
    let browser_id = "session:close-reason-session";
    let store = JsonServiceStateStore::new(home.join("state.json"));
    let repository = LockedServiceStateRepository::new(store.clone());
    store
        .save(&ServiceState {
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["close-reason-session".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "close-reason-session".to_string(),
                BrowserSession {
                    id: "close-reason-session".to_string(),
                    profile_id: Some("work".to_string()),
                    lease: LeaseState::Exclusive,
                    profile_lease_conflict_session_ids: vec!["other-session".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    persist_closed_browser_health_in_repository(
        &repository,
        "close-reason-session",
        Some(&BrowserShutdownOutcome {
            polite_close_attempted: true,
            polite_close_succeeded: true,
            ..BrowserShutdownOutcome::default()
        }),
    )
    .unwrap();
    let persisted = store.load().unwrap();
    let event = persisted
        .events
        .iter()
        .find(|event| {
            event.kind == ServiceEventKind::BrowserHealthChanged
                && event.browser_id.as_deref() == Some(browser_id)
        })
        .expect("close should record a browser health event");
    assert_eq!(event.current_health, Some(ServiceBrowserHealth::NotStarted));
    assert_eq!(
        event
            .details
            .as_ref()
            .and_then(|details| details.get("shutdownReasonKind"))
            .and_then(|reason| reason.as_str()),
        Some("operator_requested_close")
    );
    assert_eq!(
        event
            .details
            .as_ref()
            .and_then(|details| details.get("processExitCause"))
            .and_then(|cause| cause.as_str()),
        Some("operator_requested_close")
    );
    assert_eq!(
        event
            .details
            .as_ref()
            .and_then(|details| details.get("shutdownRequested"))
            .and_then(|requested| requested.as_bool()),
        Some(true)
    );
    assert_eq!(
        event
            .details
            .as_ref()
            .and_then(|details| details.get("politeCloseSucceeded"))
            .and_then(|succeeded| succeeded.as_bool()),
        Some(true)
    );
    assert!(!persisted.browsers.contains_key(browser_id));
    assert!(!persisted.sessions.contains_key("close-reason-session"));
    let _ = fs::remove_dir_all(&home);
}
#[test]
fn test_process_exit_observation_details_include_exit_evidence() {
    let observation = ProcessExitObservation {
        pid: 1234,
        exit_code: Some(137),
        #[cfg(unix)]
        signal: Some(9),
        poll_error: None,
        stderr_log_path: Some(PathBuf::from(
            "/home/user/.agent-browser/tmp/chrome-launches/chrome-1234.stderr.log",
        )),
    };
    let details = process_exit_observation_details(&observation);
    assert_eq!(details["processExitDetection"], "local_child_try_wait");
    assert_eq!(details["processExitPid"], 1234);
    assert_eq!(details["processExitCode"], 137);
    #[cfg(unix)]
    assert_eq!(details["processExitSignal"], 9);
    assert_eq!(
        details["browserStderrLogPath"],
        "/home/user/.agent-browser/tmp/chrome-launches/chrome-1234.stderr.log"
    );
}
#[test]
fn test_close_health_marks_polite_close_failure_degraded() {
    let outcome = BrowserShutdownOutcome {
        polite_close_attempted: true,
        polite_close_succeeded: false,
        polite_close_failed: true,
        force_kill_attempted: true,
        force_kill_succeeded: true,
        errors: vec!["CDP connection closed".to_string()],
        ..BrowserShutdownOutcome::default()
    };
    let (health, last_error) = close_health_from_outcome(Some(&outcome));
    assert_eq!(health, ServiceBrowserHealth::Degraded);
    assert!(last_error
        .as_deref()
        .unwrap_or_default()
        .contains("Polite browser close failed"));
}
#[test]
fn test_close_health_marks_force_kill_failure_faulted() {
    let outcome = BrowserShutdownOutcome {
        polite_close_attempted: true,
        polite_close_succeeded: false,
        polite_close_failed: true,
        force_kill_attempted: true,
        force_kill_succeeded: false,
        force_kill_failed: true,
        errors: vec!["permission denied".to_string()],
        ..BrowserShutdownOutcome::default()
    };
    let (health, last_error) = close_health_from_outcome(Some(&outcome));
    assert_eq!(health, ServiceBrowserHealth::Faulted);
    assert!(last_error
        .as_deref()
        .unwrap_or_default()
        .contains("OS may be degraded"));
}
#[tokio::test]
async fn test_build_fetch_patterns_empty_state() {
    let state = DaemonState::new();
    let patterns = build_fetch_patterns(&state).await;
    assert!(
        patterns.is_empty(),
        "No routes/filters/headers → no patterns"
    );
}
#[tokio::test]
async fn test_build_fetch_patterns_with_routes() {
    let state = DaemonState::new();
    {
        let mut routes = state.routes.write().await;
        routes.push(super::RouteEntry {
            url_pattern: "https://example.com/*".to_string(),
            response: None,
            abort: true,
        });
    }
    let patterns = build_fetch_patterns(&state).await;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0]["urlPattern"], "https://example.com/*");
}
#[tokio::test]
async fn test_build_fetch_patterns_adds_wildcard_for_domain_filter() {
    let state = DaemonState::new();
    {
        let mut df = state.domain_filter.write().await;
        *df = Some(super::super::network::DomainFilter::new("example.com"));
    }
    let patterns = build_fetch_patterns(&state).await;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0]["urlPattern"], "*");
}
#[tokio::test]
async fn test_build_fetch_patterns_adds_wildcard_for_origin_headers() {
    let state = DaemonState::new();
    {
        let mut oh = state.origin_headers.write().await;
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer xxx".to_string());
        oh.insert("http://example.com".to_string(), headers);
    }
    let patterns = build_fetch_patterns(&state).await;
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0]["urlPattern"], "*");
}
#[tokio::test]
async fn test_build_fetch_patterns_no_duplicate_wildcard() {
    let state = DaemonState::new();
    {
        let mut routes = state.routes.write().await;
        routes.push(super::RouteEntry {
            url_pattern: "*".to_string(),
            response: None,
            abort: false,
        });
    }
    {
        let mut df = state.domain_filter.write().await;
        *df = Some(super::super::network::DomainFilter::new("example.com"));
    }
    let patterns = build_fetch_patterns(&state).await;
    assert_eq!(
        patterns.len(),
        1,
        "Should not add a second wildcard when routes already contain one"
    );
}
#[test]
fn test_auth_login_waits_for_load_event() {
    use super::super::browser::WaitUntil;
    assert_eq!(
        super::AUTH_LOGIN_WAIT_UNTIL,
        WaitUntil::Load,
        "auth_login should navigate with Load and then wait for form \
             selectors explicitly"
    );
}
#[test]
fn test_parse_key_chord_plain_key() {
    let (key, mods) = parse_key_chord("a");
    assert_eq!(key, "a");
    assert_eq!(mods, None);
}
#[test]
fn test_parse_key_chord_enter() {
    let (key, mods) = parse_key_chord("Enter");
    assert_eq!(key, "Enter");
    assert_eq!(mods, None);
}
#[test]
fn test_parse_key_chord_control_a() {
    let (key, mods) = parse_key_chord("Control+a");
    assert_eq!(key, "a");
    assert_eq!(mods, Some(2));
}
#[test]
fn test_parse_key_chord_ctrl_alias() {
    let (key, mods) = parse_key_chord("Ctrl+c");
    assert_eq!(key, "c");
    assert_eq!(mods, Some(2));
}
#[test]
fn test_parse_key_chord_shift_enter() {
    let (key, mods) = parse_key_chord("Shift+Enter");
    assert_eq!(key, "Enter");
    assert_eq!(mods, Some(8));
}
#[test]
fn test_parse_key_chord_control_shift_a() {
    let (key, mods) = parse_key_chord("Control+Shift+a");
    assert_eq!(key, "a");
    assert_eq!(mods, Some(2 | 8));
}
#[test]
fn test_parse_key_chord_meta_a() {
    let (key, mods) = parse_key_chord("Meta+a");
    assert_eq!(key, "a");
    assert_eq!(mods, Some(4));
}
#[test]
fn test_parse_key_chord_alt_tab() {
    let (key, mods) = parse_key_chord("Alt+Tab");
    assert_eq!(key, "Tab");
    assert_eq!(mods, Some(1));
}
#[test]
fn test_parse_key_chord_plus_key() {
    let (key, mods) = parse_key_chord("+");
    assert_eq!(key, "+");
    assert_eq!(mods, None);
}
#[tokio::test]
async fn test_auto_dialog_enabled_by_default() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_NO_AUTO_DIALOG"]);
    std::env::remove_var("AGENT_BROWSER_NO_AUTO_DIALOG");
    let state = DaemonState::new();
    assert!(state.auto_dialog, "auto_dialog should be true by default");
    drop(guard);
}
#[tokio::test]
async fn test_auto_dialog_disabled_by_env() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_NO_AUTO_DIALOG"]);
    guard.set("AGENT_BROWSER_NO_AUTO_DIALOG", "1");
    let state = DaemonState::new();
    assert!(
        !state.auto_dialog,
        "auto_dialog should be false when AGENT_BROWSER_NO_AUTO_DIALOG=1"
    );
    drop(guard);
}
#[tokio::test]
async fn test_auto_dialog_disabled_by_env_true() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_NO_AUTO_DIALOG"]);
    guard.set("AGENT_BROWSER_NO_AUTO_DIALOG", "true");
    let state = DaemonState::new();
    assert!(
        !state.auto_dialog,
        "auto_dialog should be false when AGENT_BROWSER_NO_AUTO_DIALOG=true"
    );
    drop(guard);
}
#[tokio::test]
async fn test_auto_dialog_not_disabled_by_random_value() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_NO_AUTO_DIALOG"]);
    guard.set("AGENT_BROWSER_NO_AUTO_DIALOG", "no");
    let state = DaemonState::new();
    assert!(
        state.auto_dialog,
        "auto_dialog should remain true for non-truthy env values"
    );
    drop(guard);
}
#[test]
fn test_pending_dialog_not_set_for_auto_handled_alert() {
    let auto_dialog = true;
    for dialog_type in &["alert", "beforeunload"] {
        let auto_handled = auto_dialog && matches!(*dialog_type, "beforeunload" | "alert");
        assert!(
            auto_handled,
            "{dialog_type} should be auto-handled when auto_dialog is true"
        );
    }
}
#[test]
fn test_pending_dialog_set_for_confirm_prompt() {
    let auto_dialog = true;
    for dialog_type in &["confirm", "prompt"] {
        let auto_handled = auto_dialog && matches!(*dialog_type, "beforeunload" | "alert");
        assert!(!auto_handled, "{dialog_type} should NOT be auto-handled");
    }
}
#[test]
fn test_close_behavior_for_attached_browser_defaults_to_detach_for_external_attach() {
    assert_eq!(
        close_behavior_for_attached_browser(false, false),
        CloseBehavior::Detach
    );
    assert_eq!(
        close_behavior_for_attached_browser(false, true),
        CloseBehavior::Detach
    );
}
#[test]
fn test_close_behavior_for_attached_browser_closes_managed_runtime_by_default() {
    assert_eq!(
        close_behavior_for_attached_browser(true, false),
        CloseBehavior::CloseBrowser
    );
}
#[test]
fn test_close_behavior_for_attached_browser_respects_leave_open_override() {
    assert_eq!(
        close_behavior_for_attached_browser(true, true),
        CloseBehavior::Detach
    );
}
#[test]
fn test_close_behavior_for_launched_browser_detaches_only_for_named_runtime_profiles() {
    assert_eq!(
        close_behavior_for_launched_browser(Some("google-login"), true),
        CloseBehavior::Detach
    );
    assert_eq!(
        close_behavior_for_launched_browser(Some("google-login"), false),
        CloseBehavior::CloseBrowser
    );
    assert_eq!(
        close_behavior_for_launched_browser(None, true),
        CloseBehavior::CloseBrowser
    );
}
#[test]
fn test_launch_profile_from_sources_prefers_command_then_env() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_PROFILE"]);
    guard.set("AGENT_BROWSER_PROFILE", "/tmp/env-profile");
    assert_eq!(
        launch_profile_from_sources(&json!({}), true).as_deref(),
        Some("/tmp/env-profile")
    );
    assert_eq!(
        launch_profile_from_sources(&json!({ "profile" : "/tmp/cmd-profile" }), true).as_deref(),
        Some("/tmp/cmd-profile")
    );
    assert_eq!(
        launch_profile_from_sources(&json!({}), false).as_deref(),
        None
    );
    guard.remove("AGENT_BROWSER_PROFILE");
    assert_eq!(launch_profile_from_sources(&json!({}), true), None);
}
#[test]
fn test_launch_args_from_sources_prefers_command_then_env() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_ARGS"]);
    guard.set("AGENT_BROWSER_ARGS", "--no-sandbox,--disable-gpu\n--foo");
    assert_eq!(
        launch_args_from_sources(&json!({})),
        vec![
            "--no-sandbox".to_string(),
            "--disable-gpu".to_string(),
            "--foo".to_string()
        ]
    );
    assert_eq!(
        launch_args_from_sources(&json!({ "args" : ["--command-arg"] })),
        vec!["--command-arg".to_string()]
    );
    guard.remove("AGENT_BROWSER_ARGS");
    assert!(launch_args_from_sources(&json!({})).is_empty());
}
