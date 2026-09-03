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
use crate::native::service_lifecycle::upsert_service_profile_and_session;
use crate::native::service_profile_access_policy::ServiceProfileAccessPolicy;
use crate::native::service_profile_acquisition::authenticated_cold_session_name;

#[test]
fn exact_close_skips_launch_only_profile_lease_selection() {
    let command = json!({
        "action": "close",
        "serviceName": "development-presentation-provider",
        "runtimeProfile": "development-presentation-provider-v5-1"
    });
    let metadata = service_profile_lease_metadata_for_command(
        &command,
        Some("development-presentation-provider-v5-1"),
    )
    .unwrap();
    assert!(metadata.is_none());
}

#[test]
fn named_profile_alias_matches_active_runtime_profile() {
    let active_path = Path::new("/tmp/agent-browser-work-profile/user-data");
    assert!(active_browser_profile_mismatch_message(
        None,
        Some("work-profile"),
        Some("work-profile"),
        Some(active_path),
        "work-session",
    )
    .is_none());
    assert!(active_browser_profile_mismatch_message(
        None,
        Some("different-profile"),
        Some("work-profile"),
        Some(active_path),
        "work-session",
    )
    .is_some());
}
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
    let profile_hint = profile_id;
    let user_data_dir =
        crate::runtime_profile::resolve_profile(Some(profile_hint), Some(profile_id))
            .unwrap()
            .user_data_dir;
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
                user_data_dir: Some(profile_hint.to_string()),
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
    assert_eq!(options.profile.as_deref(), Some(profile_hint));
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

    let mut inherited_default_options = LaunchOptions {
        runtime_profile: Some("development-default".to_string()),
        profile: Some(profile_id.to_string()),
        ..LaunchOptions::default()
    };
    let inherited_default_selection = apply_service_profile_selection(
        &mut inherited_default_options,
        &json!({
            "action": "navigate",
            "profile": profile_id,
            "serviceName": "OdolloFulfillment"
        }),
        Some(session_id),
    )
    .unwrap();
    assert_eq!(
        inherited_default_selection,
        Some(ProfileSelectionReason::ExistingOwner)
    );
    assert_eq!(
        inherited_default_options.runtime_profile.as_deref(),
        Some(profile_id)
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
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/dev/fixtures/profile-recovery/plan-0137-odollo-contractor-portal.v1.json"
    ))
    .unwrap();
    let session_id = fixture["sessionId"].as_str().unwrap();
    let browser_id = fixture["browserId"].as_str().unwrap();
    let profile_id = fixture["profileId"].as_str().unwrap();
    let service_name = fixture["serviceName"].as_str().unwrap();
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("existing-owner-unproven-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&ServiceState {
            sessions: BTreeMap::from([(
                session_id.to_string(),
                BrowserSession {
                    id: session_id.to_string(),
                    profile_id: Some(profile_id.to_string()),
                    browser_ids: vec![browser_id.to_string()],
                    ..BrowserSession::default()
                },
            )]),
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some(profile_id.to_string()),
                    active_session_ids: vec![session_id.to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut options = LaunchOptions::default();

    let error = apply_service_profile_selection(
        &mut options,
        &json!({ "action": "launch", "serviceName": service_name }),
        Some(session_id),
    )
    .unwrap_err();

    assert_eq!(error, fixture["currentFailure"]);
    assert_eq!(
        fixture["expectedRecoveryClass"],
        "reconcile_exact_principal_profile_identity"
    );
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
    let profile_hint = profile_id;
    let user_data_dir =
        crate::runtime_profile::resolve_profile(Some(profile_hint), Some(profile_id))
            .unwrap()
            .user_data_dir;
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
                user_data_dir: Some(profile_hint.to_string()),
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
    assert_eq!(options.profile.as_deref(), Some(profile_hint));

    state.sessions.insert(
        session_id.to_string(),
        BrowserSession {
            id: session_id.to_string(),
            service_name: Some("odollo-fulfillment".to_string()),
            principal_id: Some(principal_id.to_string()),
            profile_id: Some(profile_id.to_string()),
            browser_ids: vec![format!("session:{session_id}")],
            tab_ids: vec!["prelaunch-tab".to_string()],
            lease: LeaseState::Exclusive,
            ..BrowserSession::default()
        },
    );
    state.tabs.insert(
        "prelaunch-tab".to_string(),
        BrowserTab {
            id: "prelaunch-tab".to_string(),
            browser_id: format!("session:{session_id}"),
            lifecycle: TabLifecycle::Opening,
            owner_session_id: Some(session_id.to_string()),
            principal_id: Some(principal_id.to_string()),
            ..BrowserTab::default()
        },
    );
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut prelaunch_options = LaunchOptions::default();
    let prelaunch_selection = apply_service_profile_selection(
        &mut prelaunch_options,
        &json!({
            "action": "tab_new",
            "profileId": profile_id,
            "serviceName": "odollo-fulfillment",
            "servicePrincipalId": principal_id,
            "servicePrincipalProvenance": "registered_capability",
        }),
        Some(session_id),
    )
    .unwrap();
    assert_eq!(
        prelaunch_selection,
        Some(ProfileSelectionReason::ExistingOwner)
    );
    assert_eq!(
        prelaunch_options.runtime_profile.as_deref(),
        Some(profile_id)
    );

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
fn explicit_browser_session_precedes_shared_runtime_host_transport() {
    let state = ServiceState {
        sessions: BTreeMap::from([(
            "runtime-host".to_string(),
            BrowserSession {
                id: "runtime-host".to_string(),
                profile_id: Some("unrelated-runtime-host-profile".to_string()),
                ..BrowserSession::default()
            },
        )]),
        ..ServiceState::default()
    };
    let mut options = LaunchOptions::default();

    let selection =
        crate::native::action_runtime::runtime::daemon::apply_existing_session_profile_selection(
            &mut options,
            &json!({
                "action": "remote_view_open",
                "runtimeProfile": "requested-profile",
                "sessionName": "requested-browser-session",
            }),
            Some("runtime-host"),
            &state,
        )
        .unwrap();

    assert_eq!(selection, None);
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
fn exact_terminal_owner_without_live_projection_allows_explicit_profile_relaunch() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("terminal-owner-explicit-profile-relaunch-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let profile_id = "development-presentation-provider-v5-1";
    let session_id = profile_id;
    let browser_id = format!("session:{session_id}");
    let user_data_dir = crate::runtime_profile::resolve_profile(Some(profile_id), Some(profile_id))
        .unwrap()
        .user_data_dir;
    fs::create_dir_all(&user_data_dir).unwrap();
    let profile_identity_digest =
        crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir).unwrap();
    let owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "terminal-provider-owner".to_string(),
        profile_identity_digest: profile_identity_digest.clone(),
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
    let mut runtime_owner_registry =
        crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(owner);
    runtime_owner_registry.lifecycle_records.insert(
        browser_id.clone(),
        crate::runtime_owner_transfer::RuntimeLifecycleRecord {
            logical_browser_id: browser_id,
            profile_identity_digest,
            owner_generation: 5,
            lifecycle_state: crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal,
            cleanup_obligation_state:
                crate::runtime_owner_transfer::CleanupObligationState::Satisfied,
            terminal_evidence: vec![
                "service_reconcile_process_group_absent:62232".to_string(),
                "service_reconcile_profile_lock_stale_pid_absent:62232".to_string(),
            ],
            ..crate::runtime_owner_transfer::RuntimeLifecycleRecord::default()
        },
    );
    let mut state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.to_string(),
            BrowserProfile {
                id: profile_id.to_string(),
                user_data_dir: Some(profile_id.to_string()),
                ..BrowserProfile::default()
            },
        )]),
        sessions: BTreeMap::from([(
            session_id.to_string(),
            BrowserSession {
                id: session_id.to_string(),
                lease: LeaseState::Released,
                profile_id: Some(profile_id.to_string()),
                ..BrowserSession::default()
            },
        )]),
        runtime_owner_registry,
        ..ServiceState::default()
    };
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();

    let command = json!({
        "action": "navigate",
        "profile": profile_id,
        "serviceName": "development-presentation-provider",
    });
    let mut options = LaunchOptions {
        runtime_profile: Some("development-default".to_string()),
        ..LaunchOptions::default()
    };
    let selection =
        apply_service_profile_selection(&mut options, &command, Some(session_id)).unwrap();

    assert_eq!(selection, None);
    assert_eq!(options.runtime_profile.as_deref(), Some(profile_id));
    assert_eq!(options.profile.as_deref(), Some(profile_id));

    let cdp_free_command = json!({
        "action": "cdp_free_launch",
        "profile": profile_id,
        "serviceName": "development-presentation-provider",
    });
    let mut cdp_free_options = LaunchOptions {
        runtime_profile: Some("development-default".to_string()),
        ..LaunchOptions::default()
    };
    let cdp_free_selection =
        apply_service_profile_selection(&mut cdp_free_options, &cdp_free_command, Some(session_id))
            .unwrap();
    assert_eq!(cdp_free_selection, None);
    assert_eq!(
        cdp_free_options.runtime_profile.as_deref(),
        Some(profile_id)
    );
    assert_eq!(cdp_free_options.profile.as_deref(), Some(profile_id));

    let closed_tab_id = "target:closed-provider-route";
    let closed_handle = ServiceTabHandle {
        browser_id: format!("session:{session_id}"),
        session_name: Some(session_id.to_string()),
        tab_id: closed_tab_id.to_string(),
        profile_id: Some(profile_id.to_string()),
        lease_state: Some(LeaseState::Released),
        owner_session_id: Some(session_id.to_string()),
        valid: false,
        stale_reason: Some("tab_closed".to_string()),
        ..ServiceTabHandle::default()
    };
    state.sessions.get_mut(session_id).unwrap().browser_ids = vec![format!("session:{session_id}")];
    state.browsers.insert(
        format!("session:{session_id}"),
        BrowserProcess {
            id: format!("session:{session_id}"),
            profile_id: Some(profile_id.to_string()),
            tab_handles: vec![closed_handle.clone()],
            ..BrowserProcess::default()
        },
    );
    state.tabs.insert(
        closed_tab_id.to_string(),
        BrowserTab {
            id: closed_tab_id.to_string(),
            browser_id: format!("session:{session_id}"),
            lifecycle: TabLifecycle::Closed,
            owner_session_id: Some(session_id.to_string()),
            service_tab_handle: Some(closed_handle),
            ..BrowserTab::default()
        },
    );
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut closed_projection_options = LaunchOptions {
        runtime_profile: Some("development-default".to_string()),
        ..LaunchOptions::default()
    };
    let closed_projection_selection =
        apply_service_profile_selection(&mut closed_projection_options, &command, Some(session_id))
            .unwrap();
    assert_eq!(closed_projection_selection, None);
    assert_eq!(
        closed_projection_options.runtime_profile.as_deref(),
        Some(profile_id)
    );

    state.sessions.get_mut(session_id).unwrap().lease = LeaseState::Exclusive;
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut active_lease_options = LaunchOptions {
        profile: Some(profile_id.to_string()),
        ..LaunchOptions::default()
    };
    assert_eq!(
        apply_service_profile_selection(&mut active_lease_options, &command, Some(session_id))
            .unwrap_err(),
        "existing_session_profile_identity_inconsistent"
    );
    state.sessions.get_mut(session_id).unwrap().lease = LeaseState::Released;

    state
        .runtime_owner_registry
        .lifecycle_records
        .get_mut(&format!("session:{session_id}"))
        .unwrap()
        .terminal_evidence = vec!["service_reconcile_process_group_absent:62232".to_string()];
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut missing_lock_options = LaunchOptions {
        profile: Some(profile_id.to_string()),
        ..LaunchOptions::default()
    };
    assert_eq!(
        apply_service_profile_selection(&mut missing_lock_options, &command, Some(session_id))
            .unwrap_err(),
        "existing_session_profile_identity_inconsistent"
    );

    state
        .runtime_owner_registry
        .lifecycle_records
        .get_mut(&format!("session:{session_id}"))
        .unwrap()
        .terminal_evidence = vec![
        "service_reconcile_process_group_absent:62232".to_string(),
        "service_reconcile_profile_lock_stale_pid_absent:62232".to_string(),
    ];
    state.browsers.insert(
        format!("session:{session_id}"),
        BrowserProcess {
            id: format!("session:{session_id}"),
            profile_id: Some(profile_id.to_string()),
            pid: Some(62232),
            ..BrowserProcess::default()
        },
    );
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .unwrap();
    let mut live_projection_options = LaunchOptions {
        profile: Some(profile_id.to_string()),
        ..LaunchOptions::default()
    };
    assert_eq!(
        apply_service_profile_selection(&mut live_projection_options, &command, Some(session_id))
            .unwrap_err(),
        "existing_session_profile_identity_inconsistent"
    );
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

#[tokio::test]
async fn canonical_profile_claim_fences_the_prelaunch_effect() {
    use crate::native::service_lease_authority::{
        acquire_lease_claim_with_receipt_in_repository, issue_lease_effect_authorization_for_state,
        AcquireLeaseClaimRequest, LeaseClaimMode, LeaseEffectIntent, LeaseResourceKey,
    };
    use crate::native::service_principal::{
        register_profile_capability, ServicePrincipalRegistrationRequest,
    };

    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("canonical-profile-effect-fence");
    fs::create_dir_all(&home).expect("test home should exist");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let repository = LockedServiceStateRepository::default_json().unwrap();
    let profile_path = home.join("profiles/acs-profile");
    let registered = repository
        .mutate(|state| {
            state.profiles.insert(
                "acs-profile".to_string(),
                BrowserProfile {
                    id: "acs-profile".to_string(),
                    user_data_dir: Some(profile_path.to_string_lossy().into_owned()),
                    ..BrowserProfile::default()
                },
            );
            register_profile_capability(
                &mut state.service_principals,
                ServicePrincipalRegistrationRequest {
                    principal_id: "principal:journal".to_string(),
                    display_name: Some("Journal".to_string()),
                    profile_id: "acs-profile".to_string(),
                    registered_at: Some(chrono::Utc::now().to_rfc3339()),
                    registered_by: Some("canonical-effect-test".to_string()),
                },
                "journal-canonical-effect-proof-capability-with-sufficient-length",
            )
            .map_err(|error| format!("service_principal_{}", error.code.as_str()))
        })
        .unwrap();
    let now = chrono::Utc::now();
    let acquired = acquire_lease_claim_with_receipt_in_repository(
        &repository,
        AcquireLeaseClaimRequest {
            resource: LeaseResourceKey::profile("acs-profile"),
            parent_claim_id: None,
            principal_id: "principal:journal".to_string(),
            capability_id: registered.capability.capability_id.clone(),
            capability_revision: registered.capability.revision,
            mode: LeaseClaimMode::Ephemeral,
            expected_claim_revision: 0,
            idempotency_key: "journal-acquire-1".to_string(),
            now: now.to_rfc3339(),
            expires_at: (now + chrono::Duration::minutes(5)).to_rfc3339(),
            transition_deadline: None,
            recovery_controller_id: None,
            boot_epoch: crate::process_identity::current_boot_epoch(),
            owner_generation: None,
        },
    )
    .unwrap();
    let authorization = issue_lease_effect_authorization_for_state(
        &repository.load_snapshot().unwrap(),
        acquired.claim.as_ref().unwrap(),
        &LeaseEffectIntent {
            action_class: "browser_launch".to_string(),
            audience: "journal-session".to_string(),
            operation_idempotency_key: "journal-launch-1".to_string(),
            executor_identity_digest: None,
            issued_at: now.to_rfc3339(),
            authorization_expires_at: (now + chrono::Duration::minutes(2)).to_rfc3339(),
        },
        b"journal-canonical-effect-proof-capability-with-sufficient-length",
    )
    .unwrap();
    let metadata = ServiceLaunchMetadata {
        profile_id: Some("acs-profile".to_string()),
        service_name: Some("JournalDownloader".to_string()),
        ..ServiceLaunchMetadata::default()
    };
    let command = json!({
        "action": "tab_new",
        "profileId": "acs-profile",
        "leaseEffectOperationId": "journal-launch-1",
        "leaseEffectAuthorization": authorization,
    });

    ensure_service_profile_lease_available(&metadata, "journal-session", &command)
        .await
        .unwrap();

    let wrong_audience =
        ensure_service_profile_lease_available(&metadata, "foreign-session", &command)
            .await
            .unwrap_err();
    assert_eq!(wrong_audience, "lease_authority_effect_scope_mismatch");

    let mut missing_operation = command.clone();
    missing_operation
        .as_object_mut()
        .unwrap()
        .remove("leaseEffectOperationId");
    let missing_operation_error =
        ensure_service_profile_lease_available(&metadata, "journal-session", &missing_operation)
            .await
            .unwrap_err();
    assert_eq!(
        missing_operation_error,
        "lease_authority_effect_operation_id_missing"
    );

    let mut stale_command = command.clone();
    stale_command["leaseEffectAuthorization"]["fencingToken"] = json!(999);
    let error =
        ensure_service_profile_lease_available(&metadata, "journal-session", &stale_command)
            .await
            .unwrap_err();
    assert_eq!(error, "lease_authority_stale_claim");

    let mut unsupported_command = command.clone();
    unsupported_command["leaseEffectAuthorization"]["schemaVersion"] =
        json!("agent-browser.lease-effect-authorization.v0");
    let error =
        ensure_service_profile_lease_available(&metadata, "journal-session", &unsupported_command)
            .await
            .unwrap_err();
    assert_eq!(error, "lease_authority_unsupported_schema");

    let profile_identity_digest = crate::runtime_profile::canonical_profile_identity_digest(
        &crate::runtime_profile::resolve_profile(
            Some(profile_path.to_str().unwrap()),
            Some("acs-profile"),
        )
        .unwrap()
        .user_data_dir,
    )
    .unwrap();
    repository
        .mutate(|state| {
            state.runtime_owner_registry =
                crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                    crate::runtime_owner_transfer::ProfileOwner {
                        owner_id: "owner:appeared-after-claim".to_string(),
                        profile_identity_digest,
                        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                        owner_generation: 1,
                        browser_id: "browser:foreign".to_string(),
                        daemon_session_route: "foreign-route".to_string(),
                        process_instance_digest: "a".repeat(64),
                        browser_family: "chrome".to_string(),
                        cdp_endpoint_identity_digest: "b".repeat(64),
                        target_set_digest: "c".repeat(64),
                        pending_transfer: None,
                        last_transition: None,
                    },
                );
            Ok(())
        })
        .unwrap();
    let error = ensure_service_profile_lease_available(&metadata, "journal-session", &command)
        .await
        .unwrap_err();
    assert_eq!(error, "lease_authority_owner_generation_stale");
    let _ = fs::remove_dir_all(&home);
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
        ServiceProfileLeaseGate::Reject { error, .. } => {
            assert!(error.contains("Duplicate service profile lane blocked"));
            assert!(error.contains("browser-existing"));
            assert!(error.contains("allowDuplicateProfileLane=true"));
        }
        other => panic!("expected duplicate lane rejection, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn service_profile_lease_fail_open_rewrites_duplicate_lane_to_isolated_profile() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_PROFILE_LEASE_MODE"]);
    let home = unique_socket_dir("profile-lease-fail-open-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    guard.set("AGENT_BROWSER_PROFILE_LEASE_MODE", "fail_open_ephemeral");
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
    let mut command = json!({
        "action": "tab_new",
        "serviceName": "JournalDownloader",
        "runtimeProfile": "acs-profile",
        "profileId": "acs-profile",
        "profile": "/private/authenticated/acs-profile",
        "profileLeasePolicy": "wait",
        "profileLeaseWaitTimeoutMs": 2_000
    });

    let decision = service_profile_lease_admission(&mut command, "new-session", Some(0))
        .expect("fail-open lease gate should evaluate");

    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    let fallback_profile = command["runtimeProfile"]
        .as_str()
        .expect("fallback should select a runtime profile");
    assert!(fallback_profile.starts_with("lease-fail-open-"));
    assert_ne!(fallback_profile, "acs-profile");
    assert!(command.get("profile").is_none());
    assert!(command.get("profileId").is_none());
    assert_eq!(command["profileLeaseFailOpen"]["applied"], true);
    assert_eq!(
        command["profileLeaseFailOpen"]["originalProfileId"],
        "acs-profile"
    );
    assert_eq!(
        command["profileLeaseFailOpen"]["reason"],
        "duplicate_live_profile_lane"
    );
    let fallback_metadata =
        service_profile_lease_metadata_for_command(&command, Some("new-session"))
            .expect("fallback metadata should resolve")
            .expect("fallback should carry launch metadata");
    assert!(!fallback_metadata.persistent_profile);
    let mut fallback_state = ServiceState::default();
    upsert_service_profile_and_session(
        &mut fallback_state,
        "new-session",
        fallback_metadata.profile_id.clone(),
        &fallback_metadata,
    );
    let retained_fallback = fallback_state
        .profiles
        .get(fallback_profile)
        .expect("fallback profile should be retained for lifecycle cleanup");
    assert_eq!(
        retained_fallback.profile_class,
        ProfileClass::ManagedOneTime
    );
    assert!(!retained_fallback.persistent);
    let mut repeated_command = json!({
        "action": "tab_new",
        "serviceName": "JournalDownloader",
        "runtimeProfile": "acs-profile",
        "profile": "/private/authenticated/acs-profile",
        "profileLeasePolicy": "wait"
    });
    let repeated_decision =
        service_profile_lease_admission(&mut repeated_command, "new-session", Some(0))
            .expect("repeated fail-open admission should evaluate");
    assert!(matches!(repeated_decision, ServiceProfileLeaseGate::Ready));
    assert_eq!(repeated_command["runtimeProfile"], fallback_profile);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn service_profile_lease_fail_open_rewrites_exclusive_conflict_without_waiting() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_PROFILE_LEASE_MODE"]);
    let home = unique_socket_dir("profile-lease-fail-open-exclusive-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    guard.set("AGENT_BROWSER_PROFILE_LEASE_MODE", "fail_open_ephemeral");
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
    let mut command = json!({
        "action": "tab_new",
        "serviceName": "JournalDownloader",
        "runtimeProfile": "acs-profile",
        "profileLeasePolicy": "wait",
        "profileLeaseWaitTimeoutMs": 2_000
    });

    let decision = service_profile_lease_admission(&mut command, "new-session", Some(0))
        .expect("fail-open lease admission should evaluate");

    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    assert_eq!(
        command["profileLeaseFailOpen"]["reason"],
        "exclusive_profile_lease"
    );
    assert_eq!(command["profileLeasePolicy"], "reject");
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn service_profile_lease_unsafe_claim_any_preserves_requested_profile_and_admits_conflict() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_PROFILE_LEASE_MODE"]);
    let home = unique_socket_dir("profile-lease-unsafe-claim-any-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    guard.set("AGENT_BROWSER_PROFILE_LEASE_MODE", "unsafe_claim_any");
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            sessions: BTreeMap::from([(
                "foreign-session".to_string(),
                BrowserSession {
                    id: "foreign-session".to_string(),
                    profile_id: Some("acs-profile".to_string()),
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let mut command = json!({
        "action": "tab_new",
        "serviceName": "EmergencyClient",
        "runtimeProfile": "acs-profile",
        "sessionName": "claiming-session",
        "profileLeasePolicy": "reject"
    });

    let decision = service_profile_lease_admission(&mut command, "claiming-session", Some(0))
        .expect("unsafe claim-any admission should evaluate");

    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    assert_eq!(command["runtimeProfile"], "acs-profile");
    assert_eq!(command["sessionName"], "claiming-session");
    assert_eq!(command["profileLeaseUnsafeClaim"]["applied"], true);
    assert_eq!(
        command["profileLeaseUnsafeClaim"]["mode"],
        "unsafe_claim_any"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn service_profile_lease_fail_open_leaves_conflict_free_request_unchanged() {
    let guard = EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_PROFILE_LEASE_MODE",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    let home = unique_socket_dir("profile-lease-fail-open-clear-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    guard.set("AGENT_BROWSER_PROFILE_LEASE_MODE", "fail_open_ephemeral");
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    let mut command = json!({
        "action": "tab_new",
        "serviceName": "JournalDownloader",
        "runtimeProfile": "acs-profile",
        "profileLeasePolicy": "wait",
        "profileLeaseWaitTimeoutMs": 2_000
    });
    let original = command.clone();

    let decision = service_profile_lease_admission(&mut command, "new-session", Some(0))
        .expect("conflict-free lease admission should evaluate");

    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    assert_eq!(command, original);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn service_profile_lease_admission_never_rewrites_canonical_authorization() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_PROFILE_LEASE_MODE"]);
    let home = unique_socket_dir("profile-lease-canonical-admission-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    guard.set("AGENT_BROWSER_PROFILE_LEASE_MODE", "fail_open_ephemeral");
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
    let mut command = json!({
        "action": "tab_new",
        "serviceName": "JournalDownloader",
        "runtimeProfile": "acs-profile",
        "profileLeasePolicy": "wait",
        "leaseEffectAuthorization": {
            "schemaVersion": "agent-browser.lease-effect-authorization.v5",
            "claimId": "claim-a"
        }
    });
    let original = command.clone();

    let decision = service_profile_lease_admission(&mut command, "new-session", Some(0))
        .expect("canonical lease admission should bypass the legacy scheduler gate");

    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));
    assert_eq!(command, original);
    assert!(command.get("profileLeaseFailOpen").is_none());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn service_profile_lease_admission_rejects_duplicate_lane_when_emergency_mode_is_disabled() {
    let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_PROFILE_LEASE_MODE"]);
    let home = unique_socket_dir("profile-lease-fail-open-disabled-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    guard.remove("AGENT_BROWSER_PROFILE_LEASE_MODE");
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
    let mut command = json!({
        "action": "tab_new",
        "serviceName": "JournalDownloader",
        "runtimeProfile": "acs-profile"
    });

    let decision = service_profile_lease_admission(&mut command, "new-session", Some(0))
        .expect("normal lease admission should evaluate");

    assert!(matches!(decision, ServiceProfileLeaseGate::Reject { .. }));
    assert_eq!(command["runtimeProfile"], "acs-profile");
    assert!(command.get("profileLeaseFailOpen").is_none());
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

#[tokio::test]
async fn canonical_effect_fence_precedes_retained_session_attach() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("canonical-fence-before-retained-attach");
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
                    cdp_endpoint: Some("ws://127.0.0.1:1/devtools/browser/unreachable".to_string()),
                    active_session_ids: vec!["last30days-facebook".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        })
        .expect("service state should be persisted");
    let mut state = DaemonState::new();
    state.session_id = "last30days-facebook".to_string();
    let command = json!({
        "action": "tab_list",
        "profileId": "last30days-facebook",
        "leaseEffectOperationId": "launch:stale",
        "leaseEffectAuthorization": {
            "schemaVersion": "agent-browser.lease-effect-authorization.v0",
            "signingKeyId": "lease-signing-key:stale",
            "signingKeyEpoch": 1,
            "resource": { "kind": "profile", "id": "last30days-facebook" },
            "claimId": "claim:stale",
            "principalId": "principal:last30days",
            "capabilityId": "capability:last30days",
            "capabilityRevision": 1,
            "claimRevision": 1,
            "fencingToken": 1,
            "ownerGeneration": null,
            "actionClass": "browser_launch",
            "audience": "last30days-facebook",
            "operationIdempotencyKey": "launch:stale",
            "issuedAt": "2026-08-31T12:00:00Z",
            "authorizationExpiresAt": "2026-08-31T12:02:00Z",
            "proof": "00"
        }
    });

    let error = auto_launch(&mut state, &command).await.unwrap_err();

    assert_eq!(error, "lease_authority_unsupported_schema");
    assert!(state.browser.is_none());
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
fn shared_local_session_continuity_does_not_require_runtime_owner_proof() {
    let profile_id = "ephemeral-debug-profile";
    let session_id = "ephemeral-debug-session";
    let browser_id = "session:ephemeral-debug-session";
    let state = ServiceState {
        profiles: BTreeMap::from([(
            profile_id.to_string(),
            BrowserProfile {
                id: profile_id.to_string(),
                name: "Ephemeral debug profile".to_string(),
                access_policy: Some(ServiceProfileAccessPolicy::shared_local_default(profile_id)),
                ..BrowserProfile::default()
            },
        )]),
        sessions: BTreeMap::from([(
            session_id.to_string(),
            BrowserSession {
                id: session_id.to_string(),
                profile_id: Some(profile_id.to_string()),
                browser_ids: vec![browser_id.to_string()],
                ..BrowserSession::default()
            },
        )]),
        browsers: BTreeMap::from([(
            browser_id.to_string(),
            BrowserProcess {
                id: browser_id.to_string(),
                profile_id: Some(profile_id.to_string()),
                health: ServiceBrowserHealth::ProcessExited,
                active_session_ids: vec![session_id.to_string()],
                ..BrowserProcess::default()
            },
        )]),
        ..ServiceState::default()
    };
    let mut options = LaunchOptions::default();
    let reason = apply_existing_session_profile_selection(
        &mut options,
        &json!({
            "action": "navigate",
            "clientSubjectId": "client:debugger",
            "identityAssurance": "self-declared"
        }),
        Some(session_id),
        &state,
    )
    .expect("shared-local continuity should remain usable without a strict owner binding");

    assert_eq!(reason, Some(ProfileSelectionReason::ExistingOwner));
    assert_eq!(options.runtime_profile.as_deref(), Some(profile_id));
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
fn authenticated_cold_access_plan_route_without_preexisting_session_passes_profile_lease_gate() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("authenticated-cold-profile-lease-gate-home");
    fs::create_dir_all(&home).expect("test home should be created");
    guard.set("HOME", home.to_str().expect("test home should be utf-8"));
    let profile_id = "last30days-facebook";
    let principal_id = "last30days-social";
    let raw_capability = "synthetic-last30days-capability-more-than-thirty-two-characters";
    let user_data_dir = home.join("last30days-facebook-user-data");
    fs::create_dir_all(&user_data_dir).expect("profile directory should be created");
    let profile = BrowserProfile {
        id: profile_id.to_string(),
        user_data_dir: Some(user_data_dir.display().to_string()),
        ..BrowserProfile::default()
    };
    let mut state = ServiceState {
        profiles: BTreeMap::from([(profile_id.to_string(), profile.clone())]),
        ..ServiceState::default()
    };
    crate::native::service_principal::register_profile_capability(
        &mut state.service_principals,
        crate::native::service_principal::ServicePrincipalRegistrationRequest {
            principal_id: principal_id.to_string(),
            display_name: Some("Last30days social".to_string()),
            profile_id: profile_id.to_string(),
            registered_at: Some("2026-08-30T12:00:00Z".to_string()),
            registered_by: Some("operator".to_string()),
        },
        raw_capability,
    )
    .expect("profile capability should register");
    let authority = crate::native::service_principal::authenticate_profile_capability(
        &state.service_principals,
        raw_capability,
        Some(profile_id),
    )
    .expect("profile capability should authenticate");
    let session_id = authenticated_cold_session_name(&authority, &profile)
        .expect("authenticated profile should have a deterministic cold route");
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .expect("service state should be persisted");

    let mut command = json!({
        "action": "tab_new",
        "serviceName": "Last30days",
        "runtimeProfile": profile_id,
        "profile": user_data_dir.display().to_string(),
        "sessionName": session_id,
        "servicePrincipalId": authority.principal_id,
        "servicePrincipalProvenance": authority.provenance.as_str(),
        "serviceProfileCapabilityId": authority.capability_id,
        "serviceProfileCapabilityRevision": authority.capability_revision,
        "serviceProfileRouteAuthorization": {
            "schemaVersion": "agent-browser.profile-launch-route-authorization.v1",
            "kind": "authenticated_cold",
            "sessionName": session_id,
            "profileId": profile_id,
            "principalId": authority.principal_id,
            "capabilityId": authority.capability_id,
            "capabilityRevision": authority.capability_revision,
            "runtimeOwnerRegistryRevision": 0,
            "ownerId": null,
            "ownerGeneration": null
        },
        "profileLeasePolicy": "wait"
    });
    let mut launch_options = LaunchOptions::default();
    assert!(apply_authenticated_access_plan_profile_selection(
        &mut launch_options,
        &command,
        &session_id,
        &state,
    )
    .expect("the authenticated route should be evaluated"));

    let decision = service_profile_lease_gate(&command, &session_id, Some(0))
        .expect("the authenticated access-plan route should be executable");

    assert!(matches!(decision, ServiceProfileLeaseGate::Ready));

    let profile_identity_digest =
        crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir).unwrap();
    let retained_owner = crate::runtime_owner_transfer::ProfileOwner {
        owner_id: "owner-stale-transfer-generation-57".to_string(),
        profile_identity_digest: profile_identity_digest.clone(),
        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
        owner_generation: 57,
        browser_id: "session:last30days-social-direct-20260830-c13".to_string(),
        daemon_session_route: "last30days-social-direct-20260830-c13".to_string(),
        process_instance_digest: "a".repeat(64),
        browser_family: "chrome".to_string(),
        cdp_endpoint_identity_digest: "b".repeat(64),
        target_set_digest: "c".repeat(64),
        pending_transfer: None,
        last_transition: None,
    };
    state.runtime_owner_registry =
        crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(retained_owner.clone());
    state.runtime_owner_registry.lifecycle_records.insert(
        retained_owner.browser_id.clone(),
        crate::runtime_owner_transfer::RuntimeLifecycleRecord {
            logical_browser_id: retained_owner.browser_id.clone(),
            boot_epoch: None,
            profile_identity_digest,
            owner_generation: retained_owner.owner_generation,
            lifecycle_state: crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal,
            cleanup_obligation_state:
                crate::runtime_owner_transfer::CleanupObligationState::Satisfied,
            process_group_id: None,
            package_launch_identity_digest: None,
            terminal_evidence: vec![
                "exact_process_exited".to_string(),
                "profile_lock_released".to_string(),
            ],
        },
    );
    state.runtime_owner_registry.principal_bindings.insert(
        retained_owner.profile_identity_digest.clone(),
        crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
            principal_id: authority.principal_id.clone(),
            profile_id: profile_id.to_string(),
            profile_identity_digest: retained_owner.profile_identity_digest.clone(),
            capability_id: authority.capability_id.clone(),
            provenance: authority.provenance,
            owner_generation: retained_owner.owner_generation - 1,
        },
    );
    command["serviceProfileRouteAuthorization"]["runtimeOwnerRegistryRevision"] =
        json!(state.runtime_owner_registry.revision);
    command["serviceProfileRouteAuthorization"]["ownerId"] = json!(retained_owner.owner_id);
    command["serviceProfileRouteAuthorization"]["ownerGeneration"] =
        json!(retained_owner.owner_generation);
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .expect("stale projection fixture should be persisted");

    let access_plan = service_access_plan_for_state_with_principal(
        &state,
        ServiceAccessPlanRequest {
            runtime_profile: Some(profile_id.to_string()),
            target_service_ids: vec!["social".to_string()],
            ..ServiceAccessPlanRequest::default()
        },
        Some(&authority),
    );
    let mut terminal_replacement_command =
        access_plan["decision"]["serviceRequest"]["request"].clone();
    let terminal_replacement_session = terminal_replacement_command["sessionName"]
        .as_str()
        .expect("terminal replacement should expose a launch session")
        .to_string();
    assert_ne!(
        terminal_replacement_session,
        retained_owner.daemon_session_route
    );
    apply_shared_profile_route_hints_for_service_request_with_principal(
        &state,
        &mut terminal_replacement_command,
        Some(&authority),
    )
    .expect("the copied terminal replacement request should remain admissible");
    terminal_replacement_command["servicePrincipalId"] = json!(authority.principal_id);
    terminal_replacement_command["servicePrincipalProvenance"] =
        json!(authority.provenance.as_str());
    terminal_replacement_command["serviceProfileCapabilityId"] = json!(authority.capability_id);
    terminal_replacement_command["serviceProfileCapabilityRevision"] =
        json!(authority.capability_revision);
    let mut terminal_replacement_options = LaunchOptions::default();
    assert!(apply_authenticated_access_plan_profile_selection(
        &mut terminal_replacement_options,
        &terminal_replacement_command,
        &terminal_replacement_session,
        &state,
    )
    .expect("the executor should admit the exact fresh terminal replacement lane"));
    assert!(matches!(
        service_profile_lease_gate(
            &terminal_replacement_command,
            &terminal_replacement_session,
            Some(0),
        )
        .unwrap(),
        ServiceProfileLeaseGate::Ready
    ));

    state.sessions.insert(
        "historical-exclusive-session".to_string(),
        BrowserSession {
            id: "historical-exclusive-session".to_string(),
            profile_id: Some(profile_id.to_string()),
            lease: LeaseState::Exclusive,
            ..BrowserSession::default()
        },
    );
    JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap())
        .save(&state)
        .expect("historical projection fixture should be persisted");

    let mut stale_projection_options = LaunchOptions::default();
    assert!(apply_authenticated_access_plan_profile_selection(
        &mut stale_projection_options,
        &command,
        &session_id,
        &state,
    )
    .expect("exact stale projection route should be evaluated"));
    assert!(matches!(
        service_profile_lease_gate(&command, &session_id, Some(0)).unwrap(),
        ServiceProfileLeaseGate::Ready
    ));
    let mut tampered_command = command.clone();
    tampered_command["serviceProfileRouteAuthorization"]["ownerGeneration"] = json!(58);
    assert!(matches!(
        service_profile_lease_gate(&tampered_command, &session_id, Some(0)).unwrap(),
        ServiceProfileLeaseGate::Wait { .. }
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
