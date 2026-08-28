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
fn apply_launch_host_hints_preserves_reserved_route_private_display() {
    let command = json!(
        { "action" : "launch", "browserHost" : "remote_headed",
        "displayIsolation" : "private_virtual_display", "remoteHeadedDisplay" : ":94",
        "routeId" : "route-94", "displayAllocationId" : "display-94" }
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
    assert_eq!(options.display.as_deref(), Some(":94"));
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
        None,
    )
    .unwrap();
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
        None,
    )
    .unwrap();
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
        None,
    )
    .unwrap();
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
fn test_existing_session_inherits_exact_current_owner_profile_before_default() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("existing-owner-profile-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let profile_id = "odollo-fedex";
    let session_id = "odollo-fulfillment";
    let browser_id = "browser-odollo-fedex";
    let user_data_dir = home.join(profile_id);
    fs::create_dir_all(&user_data_dir).unwrap();
    let profile_digest =
        crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "owner-odollo-fedex".to_string(),
        profile_identity_digest: profile_digest,
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 4,
        browser_id: browser_id.to_string(),
        daemon_session_route: session_id.to_string(),
        process_instance_digest: "1".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "2".repeat(64),
        target_set_digest: "3".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    let state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.to_string(),
            BrowserProfile {
                id: profile_id.to_string(),
                user_data_dir: Some(user_data_dir.display().to_string()),
                ..BrowserProfile::default()
            },
        )]),
        sessions: BTreeMap::from([(
            session_id.to_string(),
            BrowserSession {
                id: session_id.to_string(),
                profile_id: Some(profile_id.to_string()),
                browser_ids: vec![browser_id.to_string()],
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        )]),
        browsers: BTreeMap::from([(
            browser_id.to_string(),
            BrowserProcess {
                id: browser_id.to_string(),
                profile_id: Some(profile_id.to_string()),
                active_session_ids: vec![session_id.to_string()],
                health: ServiceBrowserHealth::Ready,
                ..BrowserProcess::default()
            },
        )]),
        runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
            owner,
        ),
        ..ServiceState::default()
    };
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut options = LaunchOptions::default();

    let selected = apply_service_profile_selection(
        &mut options,
        &json!({
            "action": "launch",
            "serviceName": "OdolloFulfillment",
            "sessionName": "request-route-hint-does-not-override-daemon"
        }),
        Some(session_id),
    )
    .unwrap();

    assert_eq!(selected, Some(ProfileSelectionReason::ExistingOwner));
    assert_eq!(options.runtime_profile.as_deref(), Some(profile_id));
    assert_eq!(
        options.profile.as_deref(),
        Some(user_data_dir.to_str().unwrap())
    );
    let cdp_free_plan = build_cdp_free_launch_plan(
        &json!({ "action": "cdp_free_launch", "serviceName": "OdolloFulfillment" }),
        Some(session_id),
    )
    .unwrap();
    assert_eq!(
        cdp_free_plan.launch_options.runtime_profile.as_deref(),
        Some(profile_id)
    );
    assert_eq!(
        cdp_free_plan.metadata.profile_selection_reason,
        Some(ProfileSelectionReason::ExistingOwner)
    );
}

#[test]
fn test_existing_session_rejects_explicit_profile_conflict() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("existing-owner-conflict-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let owned_path = home.join("owned-profile");
    fs::create_dir_all(&owned_path).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "owner-books".to_string(),
        profile_identity_digest: crate::runtime_profile::canonical_profile_identity_digest(
            &owned_path,
        )
        .unwrap(),
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 2,
        browser_id: "browser-books".to_string(),
        daemon_session_route: "books-receipts".to_string(),
        process_instance_digest: "4".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "5".repeat(64),
        target_set_digest: "6".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            profiles: BTreeMap::from([(
                "books-bank".to_string(),
                BrowserProfile {
                    id: "books-bank".to_string(),
                    user_data_dir: Some(owned_path.display().to_string()),
                    ..BrowserProfile::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "books-receipts".to_string(),
                BrowserSession {
                    id: "books-receipts".to_string(),
                    profile_id: Some("books-bank".to_string()),
                    browser_ids: vec!["browser-books".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-books".to_string(),
                BrowserProcess {
                    id: "browser-books".to_string(),
                    profile_id: Some("books-bank".to_string()),
                    active_session_ids: vec!["books-receipts".to_string()],
                    health: ServiceBrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                owner,
            ),
            ..ServiceState::default()
        })
        .unwrap();
    let mut options = LaunchOptions {
        runtime_profile: Some("default".to_string()),
        ..LaunchOptions::default()
    };

    let error = apply_service_profile_selection(
        &mut options,
        &json!({ "action": "launch", "serviceName": "BooksReceipts" }),
        Some("books-receipts"),
    )
    .unwrap_err();

    assert_eq!(error, "explicit_profile_conflicts_with_current_owner");
}

#[test]
fn test_existing_session_rejects_retained_identity_without_current_owner() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("existing-owner-unproven-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            sessions: BTreeMap::from([(
                "odollo-fulfillment".to_string(),
                BrowserSession {
                    id: "odollo-fulfillment".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    browser_ids: vec!["browser-odollo-fedex".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-odollo-fedex".to_string(),
                BrowserProcess {
                    id: "browser-odollo-fedex".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    active_session_ids: vec!["odollo-fulfillment".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut options = LaunchOptions::default();

    let error = apply_service_profile_selection(
        &mut options,
        &json!({ "action": "launch", "serviceName": "OdolloFulfillment" }),
        Some("odollo-fulfillment"),
    )
    .unwrap_err();

    assert_eq!(error, "existing_session_profile_identity_unproven");
    assert!(options.runtime_profile.is_none());
    assert!(options.profile.is_none());
}

#[test]
fn authenticated_principal_recovers_exact_orphaned_owner_without_foreign_bypass() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("authenticated-orphan-owner-recourse-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let profile_id = "odollo-fedex";
    let session_id = "principal-profile-odollo-fedex";
    let principal_id = "odollo-fulfillment";
    let user_data_dir = home.join(profile_id);
    fs::create_dir_all(&user_data_dir).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "orphaned-owner-odollo-fedex".to_string(),
        profile_identity_digest: crate::runtime_profile::canonical_profile_identity_digest(
            &user_data_dir,
        )
        .unwrap(),
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 4,
        browser_id: format!("session:{session_id}"),
        daemon_session_route: session_id.to_string(),
        process_instance_digest: "1".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "2".repeat(64),
        target_set_digest: "3".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    let mut state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.to_string(),
            BrowserProfile {
                id: profile_id.to_string(),
                user_data_dir: Some(user_data_dir.display().to_string()),
                ..BrowserProfile::default()
            },
        )]),
        runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
            owner,
        ),
        ..ServiceState::default()
    };
    crate::native::service_principal::register_profile_capability(
        &mut state.service_principals,
        crate::native::service_principal::ServicePrincipalRegistrationRequest {
            principal_id: principal_id.to_string(),
            display_name: Some("Odollo fulfillment".to_string()),
            profile_id: profile_id.to_string(),
            registered_at: Some("2026-08-28T00:00:00Z".to_string()),
            registered_by: Some("operator".to_string()),
        },
        "synthetic-odollo-profile-capability-more-than-thirty-two-characters",
    )
    .unwrap();
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();

    let command = json!({
        "action": "remote_view_open",
        "runtimeProfile": profile_id,
        "serviceName": "odollo-fulfillment",
        "servicePrincipalId": principal_id,
        "servicePrincipalProvenance": "registered_capability",
    });
    let mut options = LaunchOptions::default();
    let selection =
        apply_service_profile_selection(&mut options, &command, Some(session_id)).unwrap();

    assert_eq!(selection, Some(ProfileSelectionReason::ExistingOwner));
    assert_eq!(options.runtime_profile.as_deref(), Some(profile_id));
    assert_eq!(options.profile.as_deref(), user_data_dir.to_str());

    let mut foreign_options = LaunchOptions::default();
    let foreign_error = apply_service_profile_selection(
        &mut foreign_options,
        &json!({
            "action": "remote_view_open",
            "runtimeProfile": profile_id,
            "serviceName": "foreign-service",
            "servicePrincipalId": "foreign-principal",
            "servicePrincipalProvenance": "registered_capability",
        }),
        Some(session_id),
    )
    .unwrap_err();
    assert_eq!(foreign_error, "existing_session_profile_identity_unproven");
}

#[test]
fn authenticated_principal_recovers_exact_released_terminal_projection() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("authenticated-terminal-owner-recourse-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let profile_id = "odollo-fedex";
    let session_id = "principal-profile-odollo-fedex";
    let browser_id = format!("session:{session_id}");
    let principal_id = "odollo-fulfillment";
    let user_data_dir = home.join(profile_id);
    fs::create_dir_all(&user_data_dir).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "released-owner-odollo-fedex".to_string(),
        profile_identity_digest: crate::runtime_profile::canonical_profile_identity_digest(
            &user_data_dir,
        )
        .unwrap(),
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 5,
        browser_id: browser_id.clone(),
        daemon_session_route: session_id.to_string(),
        process_instance_digest: "1".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "2".repeat(64),
        target_set_digest: "3".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    let mut state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.to_string(),
            BrowserProfile {
                id: profile_id.to_string(),
                user_data_dir: Some(user_data_dir.display().to_string()),
                ..BrowserProfile::default()
            },
        )]),
        sessions: BTreeMap::from([(
            session_id.to_string(),
            BrowserSession {
                id: session_id.to_string(),
                lease: LeaseState::Released,
                profile_id: Some(profile_id.to_string()),
                browser_ids: vec![browser_id.clone()],
                ..BrowserSession::default()
            },
        )]),
        browsers: BTreeMap::from([(
            browser_id.clone(),
            BrowserProcess {
                id: browser_id,
                profile_id: Some(profile_id.to_string()),
                health: ServiceBrowserHealth::Degraded,
                ..BrowserProcess::default()
            },
        )]),
        runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
            owner,
        ),
        ..ServiceState::default()
    };
    crate::native::service_principal::register_profile_capability(
        &mut state.service_principals,
        crate::native::service_principal::ServicePrincipalRegistrationRequest {
            principal_id: principal_id.to_string(),
            display_name: Some("Odollo fulfillment".to_string()),
            profile_id: profile_id.to_string(),
            registered_at: Some("2026-08-28T00:00:00Z".to_string()),
            registered_by: Some("operator".to_string()),
        },
        "synthetic-odollo-profile-capability-more-than-thirty-two-characters",
    )
    .unwrap();
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();

    let command = json!({
        "action": "remote_view_open",
        "runtimeProfile": profile_id,
        "serviceName": "odollo-fulfillment",
        "servicePrincipalId": principal_id,
        "servicePrincipalProvenance": "registered_capability",
    });
    let mut options = LaunchOptions::default();
    let selection =
        apply_service_profile_selection(&mut options, &command, Some(session_id)).unwrap();

    assert_eq!(selection, Some(ProfileSelectionReason::ExistingOwner));
    assert_eq!(options.runtime_profile.as_deref(), Some(profile_id));
    assert_eq!(options.profile.as_deref(), user_data_dir.to_str());

    state
        .browsers
        .get_mut(&format!("session:{session_id}"))
        .unwrap()
        .pid = Some(42);
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut live_options = LaunchOptions::default();
    let live_error =
        apply_service_profile_selection(&mut live_options, &command, Some(session_id)).unwrap_err();
    assert_eq!(live_error, "existing_session_profile_identity_inconsistent");
}

#[test]
fn test_registered_work_lease_preserves_profile_selection_after_owner_exit() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("registered-session-owner-exit-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let profile_id = "odollo-fedex";
    let session_id = "odollo-fulfillment";
    let browser_id = "browser-odollo-fedex";
    let principal_id = "principal:odollo-fulfillment";
    let user_data_dir = home.join(profile_id);
    fs::create_dir_all(&user_data_dir).unwrap();
    let profile_digest =
        crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "owner-odollo-fedex".to_string(),
        profile_identity_digest: profile_digest.clone(),
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 4,
        browser_id: browser_id.to_string(),
        daemon_session_route: session_id.to_string(),
        process_instance_digest: "1".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "2".repeat(64),
        target_set_digest: "3".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    let mut state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.to_string(),
            BrowserProfile {
                id: profile_id.to_string(),
                user_data_dir: Some(user_data_dir.display().to_string()),
                ..BrowserProfile::default()
            },
        )]),
        sessions: BTreeMap::from([(
            session_id.to_string(),
            BrowserSession {
                id: session_id.to_string(),
                profile_id: Some(profile_id.to_string()),
                browser_ids: vec![browser_id.to_string()],
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        )]),
        browsers: BTreeMap::from([(
            browser_id.to_string(),
            BrowserProcess {
                id: browser_id.to_string(),
                profile_id: Some(profile_id.to_string()),
                active_session_ids: vec![session_id.to_string()],
                health: ServiceBrowserHealth::ProcessExited,
                ..BrowserProcess::default()
            },
        )]),
        runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
            owner,
        ),
        ..ServiceState::default()
    };
    let capability = "route-host-registered-capability-with-more-than-thirty-two-characters";
    let registered = crate::native::service_principal::register_profile_capability(
        &mut state.service_principals,
        crate::native::service_principal::ServicePrincipalRegistrationRequest {
            principal_id: principal_id.to_string(),
            display_name: Some("Odollo fulfillment".to_string()),
            profile_id: profile_id.to_string(),
            registered_at: Some("2026-08-27T17:00:00Z".to_string()),
            registered_by: Some("test".to_string()),
        },
        capability,
    )
    .unwrap();
    state
        .runtime_owner_registry
        .bind_principal_authority(
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                principal_id: principal_id.to_string(),
                profile_id: profile_id.to_string(),
                profile_identity_digest: profile_digest.clone(),
                capability_id: registered.capability.capability_id,
                provenance:
                    crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 4,
            },
        )
        .unwrap();
    let authority = crate::native::service_principal::authenticate_profile_capability(
        &state.service_principals,
        capability,
        Some(profile_id),
    )
    .unwrap();
    crate::native::service_principal::bind_session_work_lease(
        &mut state,
        session_id,
        &authority,
        "2099-08-27T18:00:00Z".to_string(),
    )
    .unwrap();
    state
        .runtime_owner_registry
        .owners
        .get_mut(&profile_digest)
        .unwrap()
        .state = crate::runtime_owner_transfer::ProfileOwnerState::Orphaned;
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut options = LaunchOptions::default();

    let selected = apply_service_profile_selection(
        &mut options,
        &json!({ "action": "get_url", "sessionName": session_id }),
        Some(session_id),
    )
    .unwrap();

    assert_eq!(selected, Some(ProfileSelectionReason::ExistingOwner));
    assert_eq!(options.runtime_profile.as_deref(), Some(profile_id));
    assert_eq!(
        options.profile.as_deref(),
        Some(user_data_dir.to_str().unwrap())
    );
}

#[test]
fn test_existing_session_rejects_contradictory_browser_profile() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("existing-owner-inconsistent-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let owned_path = home.join("owned-profile");
    fs::create_dir_all(&owned_path).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "owner-odollo".to_string(),
        profile_identity_digest: crate::runtime_profile::canonical_profile_identity_digest(
            &owned_path,
        )
        .unwrap(),
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 7,
        browser_id: "browser-odollo".to_string(),
        daemon_session_route: "odollo-fulfillment".to_string(),
        process_instance_digest: "7".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "8".repeat(64),
        target_set_digest: "9".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    user_data_dir: Some(owned_path.display().to_string()),
                    ..BrowserProfile::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "odollo-fulfillment".to_string(),
                BrowserSession {
                    id: "odollo-fulfillment".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    browser_ids: vec!["browser-odollo".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-odollo".to_string(),
                BrowserProcess {
                    id: "browser-odollo".to_string(),
                    profile_id: Some("foreign-profile".to_string()),
                    active_session_ids: vec!["odollo-fulfillment".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                owner,
            ),
            ..ServiceState::default()
        })
        .unwrap();
    let mut options = LaunchOptions::default();

    let error = apply_service_profile_selection(
        &mut options,
        &json!({ "action": "launch", "serviceName": "OdolloFulfillment" }),
        Some("odollo-fulfillment"),
    )
    .unwrap_err();

    assert_eq!(error, "existing_session_profile_identity_inconsistent");
}

#[test]
fn test_new_unbound_session_retains_default_profile_fallback() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("new-unbound-default-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState::default())
        .unwrap();
    let mut options = LaunchOptions::default();

    let selection = apply_service_profile_selection(
        &mut options,
        &json!({ "action": "launch" }),
        Some("genuinely-new-session"),
    )
    .unwrap();

    assert!(selection.is_none());
    assert!(options.runtime_profile.is_none());
    let resolved = crate::runtime_profile::resolve_profile(None, None).unwrap();
    assert_eq!(resolved.runtime_profile.as_deref(), Some("default"));
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
            controller_epoch: 0,
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
        apply_auto_launch_command_hints(&mut options, &command, Some(&retained), "test-session")
            .unwrap();
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
            controller_epoch: 0,
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
    let (host, _, _, _) =
        apply_auto_launch_command_hints(&mut options, &command, Some(&retained), "test-session")
            .unwrap();
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
        apply_auto_launch_command_hints(&mut options, &command, None, "test-session").unwrap();
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
    let plan = build_cdp_free_launch_plan(&cmd, None).expect("plan should parse without launching");
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
fn test_cdp_free_launch_plan_preserves_remote_headed_route_without_devtools() {
    let cmd = json!(
        { "action" : "cdp_free_launch", "serviceName" : "im-receipts", "agentName" :
        "im-receipts-service", "taskName" : "google-messages-manual-seeding",
        "targetServiceId" : "google-messages", "runtimeProfile" :
        "im-receipts-google-messages-stock-v4", "url" :
        "https://messages.google.com/web/", "browserHost" : "remote_headed",
        "requiresCdpFree" : true, "cdpAttachmentAllowed" : false, "params" : {
        "browserHost" : "remote_headed", "displayIsolation" : "shared_display",
        "remoteHeadedDisplay" : ":10", "viewStreamProvider" : "rdp_gateway",
        "controlInputProvider" : "manual_attached_desktop", "routeId" :
        "guacamole:1", "displayAllocationId" : "remote-view-display:10", "frameUrl" :
        "http://127.0.0.1:8092/guacamole/#/client/opaque", "externalUrl" :
        "https://agent-browser.example/guacamole/#/client/opaque", "connectionId" :
        "1", "connectionName" : "Agent Browser RDP Route A" } }
    );
    let plan = build_cdp_free_launch_plan(&cmd, None).expect("plan should parse without launching");
    assert_eq!(plan.service_host, ServiceBrowserHost::RemoteHeaded);
    assert!(!plan.launch_options.headless);
    assert!(plan.launch_options.remote_headed);
    assert!(!plan.launch_options.attachable);
    assert!(plan.launch_options.manual_login);
    assert_eq!(plan.launch_options.display.as_deref(), Some(":10"));
    assert_eq!(
        plan.launch_options
            .remote_headed_display_isolation
            .as_deref(),
        Some("shared_display")
    );
    assert_eq!(plan.metadata.display_name.as_deref(), Some(":10"));
    let stream = plan
        .metadata
        .view_streams
        .first()
        .expect("remote headed launch should retain one RDP stream");
    assert_eq!(stream.provider, ViewStreamProvider::RdpGateway);
    assert_eq!(stream.route_id.as_deref(), Some("guacamole:1"));
    assert_eq!(
        stream.display_allocation_id.as_deref(),
        Some("remote-view-display:10")
    );
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
        None,
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
fn test_retained_session_attach_target_rejects_cross_profile_browser_link() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("retained-session-cross-profile-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            sessions: BTreeMap::from([(
                "last30days-facebook".to_string(),
                BrowserSession {
                    id: "last30days-facebook".to_string(),
                    profile_id: Some("last30days-facebook".to_string()),
                    browser_ids: vec!["session:last30days-facebook".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    profile_id: Some("default".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    cdp_endpoint: Some("ws://127.0.0.1:37137/devtools/browser/default".to_string()),
                    active_session_ids: vec!["last30days-facebook".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");

    assert!(retained_session_attach_target_for_auto_launch(
        &json!({ "action" : "tab_list", "sessionName" : "last30days-facebook" }),
        "last30days-facebook",
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

fn retained_profile_route_state() -> ServiceState {
    ServiceState {
        profiles: BTreeMap::from([(
            "managed-one-time-route".to_string(),
            BrowserProfile {
                id: "managed-one-time-route".to_string(),
                name: "Managed one-time route".to_string(),
                user_data_dir: Some("/tmp/agent-browser-managed-one-time-route".to_string()),
                ..BrowserProfile::default()
            },
        )]),
        browsers: BTreeMap::from([(
            "session:carrier-evidence".to_string(),
            BrowserProcess {
                id: "session:carrier-evidence".to_string(),
                profile_id: Some("managed-one-time-route".to_string()),
                health: ServiceBrowserHealth::Ready,
                pid: Some(4242),
                cdp_endpoint: Some("http://127.0.0.1:39111/devtools/browser/current".to_string()),
                active_session_ids: vec!["carrier-evidence".to_string()],
                ..BrowserProcess::default()
            },
        )]),
        sessions: BTreeMap::from([(
            "carrier-evidence".to_string(),
            BrowserSession {
                id: "carrier-evidence".to_string(),
                profile_id: Some("managed-one-time-route".to_string()),
                browser_ids: vec!["session:carrier-evidence".to_string()],
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    }
}

#[test]
fn test_retained_profile_route_accepts_exact_current_owner() {
    let state = retained_profile_route_state();
    let command = json!({
        "action": "tab_new",
        "runtimeProfile": "managed-one-time-route",
        "profile": "/tmp/agent-browser-managed-one-time-route",
        "browserId": "session:carrier-evidence",
        "sessionName": "carrier-evidence"
    });
    assert!(retained_route_matches_selected_profile(
        &command,
        "carrier-evidence",
        Some(4242),
        "ws://127.0.0.1:39111/devtools/browser/current",
        &state,
    ));
}

#[test]
fn test_retained_profile_route_rejects_unproven_owner() {
    let state = retained_profile_route_state();
    let command = json!({
        "action": "tab_new",
        "runtimeProfile": "managed-one-time-route",
        "profile": "/tmp/agent-browser-managed-one-time-route",
        "browserId": "session:carrier-evidence",
        "sessionName": "carrier-evidence"
    });
    for (field, value) in [
        ("browserId", "session:other-browser"),
        ("sessionName", "other-session"),
        ("runtimeProfile", "other-profile"),
        ("profile", "/tmp/other-profile"),
    ] {
        let mut mismatched = command.clone();
        mismatched[field] = json!(value);
        assert!(!retained_route_matches_selected_profile(
            &mismatched,
            "carrier-evidence",
            Some(4242),
            "ws://127.0.0.1:39111/devtools/browser/current",
            &state,
        ));
    }
    assert!(!retained_route_matches_selected_profile(
        &command,
        "carrier-evidence",
        Some(9999),
        "ws://127.0.0.1:49999/devtools/browser/wrong",
        &state,
    ));
}

#[test]
fn test_service_profile_lease_gate_defers_to_attributed_tab_handle() {
    let command = json!({
        "action": "cdp_attach",
        "serviceName": "Odollo",
        "runtimeProfile": "default",
        "serviceTabHandle": {
            "browserId": "browser-existing",
            "tabId": "tab:carrier-evidence",
            "targetId": "target-carrier-evidence"
        }
    });
    assert!(
        service_profile_lease_metadata_for_command(&command, Some("test-session"))
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        service_profile_lease_gate(&command, "existing-session", Some(0))
            .expect("attributed tab handle should bypass launch-profile leasing"),
        ServiceProfileLeaseGate::Ready
    ));
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
    assert_eq!(descriptor.process_identity, None);
}

#[cfg(unix)]
#[test]
fn test_no_runtime_profile_handoff_identity_matches_at_resume_boundary() {
    let root = unique_socket_dir("no-runtime-handoff-identity");
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("handoff-chrome");
    fs::copy("/bin/sleep", &executable).unwrap();
    let mut child = std::process::Command::new(&executable)
        .arg("30")
        .spawn()
        .unwrap();
    let process_identity = crate::process_identity::capture_process_identity(
        child.id(),
        Some(&executable),
        Some("chrome"),
    )
    .unwrap();
    let descriptor = RuntimeHandoffDescriptor {
        schema_version: 1,
        session_name: "no-runtime-profile".to_string(),
        cdp_url: "ws://127.0.0.1:9222/devtools/browser/example".to_string(),
        browser_pid: Some(child.id()),
        runtime_profile: None,
        process_identity: Some(process_identity),
        engine: "chrome".to_string(),
        host: ServiceBrowserHost::AttachedExisting,
        close_browser_on_close: false,
        active_target_id: None,
        owner_transfer: None,
        prepared_at: "2026-08-10T12:00:00Z".to_string(),
    };

    assert!(runtime_handoff_process_assessment(&descriptor, child.id()).authorizes_adoption());

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn test_no_runtime_profile_handoff_identity_mismatch_is_rejected_before_resume() {
    let root = unique_socket_dir("no-runtime-handoff-mismatch");
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("handoff-chrome");
    fs::copy("/bin/sleep", &executable).unwrap();
    let mut child = std::process::Command::new(&executable)
        .arg("30")
        .spawn()
        .unwrap();
    let mut process_identity = crate::process_identity::capture_process_identity(
        child.id(),
        Some(&executable),
        Some("chrome"),
    )
    .unwrap();
    process_identity.start_token.push_str(":reused");
    let descriptor = RuntimeHandoffDescriptor {
        schema_version: 1,
        session_name: "no-runtime-profile".to_string(),
        cdp_url: "ws://127.0.0.1:9222/devtools/browser/example".to_string(),
        browser_pid: Some(child.id()),
        runtime_profile: None,
        process_identity: Some(process_identity),
        engine: "chrome".to_string(),
        host: ServiceBrowserHost::AttachedExisting,
        close_browser_on_close: false,
        active_target_id: None,
        owner_transfer: None,
        prepared_at: "2026-08-10T12:00:00Z".to_string(),
    };

    assert_eq!(
        runtime_handoff_process_assessment(&descriptor, child.id()).ownership,
        crate::process_identity::RuntimeProcessOwnership::ReusedUnrelated
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(root);
}
#[cfg(unix)]
#[test]
fn test_managed_runtime_attach_target_uses_runtime_state() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("managed-runtime-attach-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let runtime_profile = "managed-attach-test";
    let user_data_dir = home.join("managed-user-data");
    fs::create_dir_all(&user_data_dir).expect("user data dir should be created");
    let executable = home.join("managed-chrome");
    fs::copy("/bin/sleep", &executable).expect("browser-looking fixture should be copied");
    let mut child = std::process::Command::new(&executable)
        .arg("30")
        .spawn()
        .expect("browser-looking fixture should start");
    crate::runtime_profile::write_runtime_state(&crate::runtime_profile::RuntimeState {
        runtime_profile: runtime_profile.to_string(),
        user_data_dir: user_data_dir.display().to_string(),
        browser_pid: child.id(),
        process_identity: crate::process_identity::capture_process_identity(
            child.id(),
            Some(&executable),
            Some("chrome"),
        ),
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
    assert_eq!(target.browser_pid, child.id());
    assert_eq!(target.cdp_port, 9333);
    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(&home);
}
#[cfg(unix)]
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
    let executable = home.join("managed-chrome");
    fs::copy("/bin/sleep", &executable).expect("browser-looking fixture should be copied");
    let mut child = std::process::Command::new(&executable)
        .arg("30")
        .spawn()
        .expect("browser-looking fixture should start");
    crate::runtime_profile::write_runtime_state(&crate::runtime_profile::RuntimeState {
        runtime_profile: runtime_profile.to_string(),
        user_data_dir: user_data_dir.display().to_string(),
        browser_pid: child.id(),
        process_identity: crate::process_identity::capture_process_identity(
            child.id(),
            Some(&executable),
            Some("chrome"),
        ),
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
    assert_eq!(target.browser_pid, child.id());
    assert_eq!(target.cdp_port, 9444);
    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(&home);
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
