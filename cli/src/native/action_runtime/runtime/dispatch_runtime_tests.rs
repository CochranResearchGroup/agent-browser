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
use crate::native::network_archive::handle_har_stop;
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

#[test]
fn transferred_owner_can_prepare_the_next_runtime_handoff() {
    use crate::runtime_owner_transfer::{
        OwnerAuthorityClaim, ProfileOwner, ProfileOwnerState, RuntimeOwnerBinding,
    };

    let owner = ProfileOwner {
        owner_id: "owner-transfer-issued".to_string(),
        profile_identity_digest: "profile-digest".to_string(),
        state: ProfileOwnerState::Ready,
        owner_generation: 2,
        browser_id: "session:logical-browser".to_string(),
        daemon_session_route: "handoff-candidate".to_string(),
        process_instance_digest: "process-digest".to_string(),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "cdp-digest".to_string(),
        target_set_digest: "targets-digest".to_string(),
        pending_transfer: None,
        last_transition: None,
    };
    let binding = RuntimeOwnerBinding::effect_capable(OwnerAuthorityClaim::from_owner(&owner));

    assert!(current_owner_matches_preparing_daemon(
        &owner,
        Some(&binding),
        "owner-route-derived",
        "session:handoff-candidate",
        "handoff-candidate",
        "process-digest",
        "chrome",
        "cdp-digest",
        "targets-digest",
    ));
}

#[test]
fn transferred_owner_requires_effect_capable_exact_binding() {
    use crate::runtime_owner_transfer::{
        OwnerAuthorityClaim, ProfileOwner, ProfileOwnerState, RuntimeOwnerBinding,
    };

    let owner = ProfileOwner {
        owner_id: "owner-transfer-issued".to_string(),
        profile_identity_digest: "profile-digest".to_string(),
        state: ProfileOwnerState::Ready,
        owner_generation: 2,
        browser_id: "session:logical-browser".to_string(),
        daemon_session_route: "handoff-candidate".to_string(),
        process_instance_digest: "process-digest".to_string(),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "cdp-digest".to_string(),
        target_set_digest: "targets-digest".to_string(),
        pending_transfer: None,
        last_transition: None,
    };
    let binding = RuntimeOwnerBinding::observation_only(OwnerAuthorityClaim::from_owner(&owner));

    assert!(!current_owner_matches_preparing_daemon(
        &owner,
        Some(&binding),
        "owner-route-derived",
        "session:handoff-candidate",
        "handoff-candidate",
        "process-digest",
        "chrome",
        "cdp-digest",
        "targets-digest",
    ));
}

#[test]
fn route_derived_legacy_owner_can_prepare_without_a_binding() {
    use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};

    let owner = ProfileOwner {
        owner_id: "owner-route-derived".to_string(),
        profile_identity_digest: "profile-digest".to_string(),
        state: ProfileOwnerState::Ready,
        owner_generation: 1,
        browser_id: "session:legacy-route".to_string(),
        daemon_session_route: "legacy-route".to_string(),
        process_instance_digest: "process-digest".to_string(),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "cdp-digest".to_string(),
        target_set_digest: "targets-digest".to_string(),
        pending_transfer: None,
        last_transition: None,
    };

    assert!(current_owner_matches_preparing_daemon(
        &owner,
        None,
        "owner-route-derived",
        "session:legacy-route",
        "legacy-route",
        "process-digest",
        "chrome",
        "cdp-digest",
        "targets-digest",
    ));
    assert!(!current_owner_matches_preparing_daemon(
        &owner,
        None,
        "owner-route-derived",
        "session:legacy-route",
        "legacy-route",
        "different-process-digest",
        "chrome",
        "cdp-digest",
        "targets-digest",
    ));
}

#[test]
fn orphan_adoption_follows_the_revoked_owner_logical_browser() {
    use crate::native::service_model::{BrowserSession, ServiceState};
    use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};

    let mut service_state = ServiceState::default();
    service_state.runtime_owner_registry.owners.insert(
        "profile-digest".to_string(),
        ProfileOwner {
            owner_id: "owner-revoked".to_string(),
            profile_identity_digest: "profile-digest".to_string(),
            state: ProfileOwnerState::Orphaned,
            owner_generation: 3,
            browser_id: "session:logical-browser".to_string(),
            daemon_session_route: "handoff-source".to_string(),
            process_instance_digest: "process-digest".to_string(),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "cdp-digest".to_string(),
            target_set_digest: "targets-digest".to_string(),
            pending_transfer: None,
            last_transition: None,
        },
    );

    assert_eq!(
        orphan_logical_browser_id(&service_state, "handoff-source").unwrap(),
        "session:logical-browser"
    );
    let mut mapped_state = service_state.clone();
    mapped_state
        .runtime_owner_registry
        .owners
        .get_mut("profile-digest")
        .unwrap()
        .daemon_session_route = "registry-alias".to_string();
    mapped_state.sessions.insert(
        "handoff-source".to_string(),
        BrowserSession {
            id: "handoff-source".to_string(),
            browser_ids: vec!["session:logical-browser".to_string()],
            ..BrowserSession::default()
        },
    );
    assert_eq!(
        orphan_logical_browser_id(&mapped_state, "handoff-source").unwrap(),
        "session:logical-browser"
    );
    assert_eq!(
        orphan_logical_browser_id(&ServiceState::default(), "legacy-source").unwrap(),
        "session:legacy-source"
    );
}
