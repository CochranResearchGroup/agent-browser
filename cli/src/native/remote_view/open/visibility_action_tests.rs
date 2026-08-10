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
