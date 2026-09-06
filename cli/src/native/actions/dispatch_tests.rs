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

#[tokio::test]
async fn configured_desktop_provider_gates_precede_confirmation_and_dispatch_effects() {
    let mut state = DaemonState::new();
    state.confirm_actions = Some(ConfirmActions {
        categories: HashSet::from(["desktop_prompt_observe".to_string()]),
    });

    let response = execute_command(
        &json!({
            "id": "prompt-observe-1",
            "action": "desktop_prompt_observe",
            "browserId": "browser-rdp-1",
            "promptProfileId": "p110-external-prompt-v1",
            "serviceName": "DesktopPromptObserver",
            "agentName": "fixture-agent",
            "taskName": "observe"
        }),
        &mut state,
    )
    .await;

    assert_eq!(response["success"], false);
    assert!(response["error"]
        .as_str()
        .is_some_and(|error| error.starts_with("desktop_prompt_provider_unavailable:")));
    assert!(state.pending_confirmation.is_none());

    let mut state = DaemonState::new();
    state.confirm_actions = Some(ConfirmActions {
        categories: HashSet::from(["desktop_interact".to_string()]),
    });

    let response = execute_command(
        &json!({
            "id": "desktop-interact-1",
            "action": "desktop_interact",
            "browserId": "browser-rdp-1",
            "controllerLeaseId": "lease-1",
            "operationId": "operation-stress-1",
            "recipe": { "recipeId": "p110-foundation-stress-v1" },
            "serviceName": "DesktopInteractor",
            "agentName": "fixture-agent",
            "taskName": "stress"
        }),
        &mut state,
    )
    .await;

    assert_eq!(response["success"], false);
    assert!(response["error"].as_str().is_some_and(|error| {
        error.starts_with("desktop_input_provider_")
            && error.ends_with(": controlled desktop input admission failed")
    }));
    assert!(state.pending_confirmation.is_none());
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

#[test]
fn manual_seeding_lifecycle_denies_cdp_action_before_auto_launch() {
    let state = DaemonState::new();
    let browser_id = crate::native::action_runtime::runtime::service_browser_id(&state.session_id);
    let path = JsonServiceStateStore::default_path().unwrap();
    JsonServiceStateStore::new(path)
        .save(&ServiceState {
            browsers: std::collections::BTreeMap::from([(
                browser_id.clone(),
                BrowserProcess {
                    id: browser_id,
                    profile_id: Some("manual-cdp-block".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            profile_seeding_handoffs: std::collections::BTreeMap::from([(
                "manual-cdp-block:google".to_string(),
                crate::native::service_model::ProfileSeedingHandoffRecord {
                    id: "manual-cdp-block:google".to_string(),
                    profile_id: "manual-cdp-block".to_string(),
                    target_service_id: "google".to_string(),
                    state: ProfileSeedingHandoffState::SeedingWaitingForClose,
                    pid: Some(4242),
                    ..crate::native::service_model::ProfileSeedingHandoffRecord::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();

    let blocker = active_manual_seeding_cdp_blocker(&json!({"action": "snapshot"}), &state)
        .expect("manual seeding must block CDP actions");
    assert!(blocker.starts_with("manual_seeding_cdp_action_denied:"));
    assert!(blocker.contains("manual-cdp-block"));
}

// Drives the same dispatcher as an original-handle request after host restart.
// The websocket is a protocol fixture, not a Chrome process or launch substitute.
#[tokio::test]
async fn retained_handle_evaluate_reconnects_without_acquiring_a_tab() {
    run_retained_handle_recovery_fixture(true).await;
}

#[tokio::test]
async fn retained_handle_missing_target_never_creates_or_selects_a_peer() {
    run_retained_handle_recovery_fixture(false).await;
}

async fn run_retained_handle_recovery_fixture(target_present: bool) {
    use crate::native::runtime_lifecycle::{ManagedLaneRegistration, RuntimeLifecycleAuthority};
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    let guard = EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    let home = std::env::temp_dir().join(format!(
        "p159-retained-handle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.as_path().to_str().unwrap());
    // Opt into this exact disposable HOME instead of the process-shared test store.
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set(
        "AGENT_BROWSER_HOME",
        home.as_path().join(".agent-browser").to_str().unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "ws://{}/devtools/browser/retained",
        listener.local_addr().unwrap()
    );
    let methods = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let observed = methods.clone();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut websocket = tokio_tungstenite::accept_async(socket).await.unwrap();
        while let Some(Ok(Message::Text(text))) = websocket.next().await {
            let command: Value = serde_json::from_str(&text).unwrap();
            let method = command["method"].as_str().unwrap();
            observed.lock().unwrap().push(method.to_string());
            let result = match method {
                "Target.getTargets" => json!({"targetInfos": [{
                    "targetId": if target_present { "retained-target" } else { "peer-target" }, "type": "page", "title": "fixture",
                    "url": "about:blank", "attached": false
                }]}),
                "Target.attachToTarget" => json!({"sessionId": "retained-cdp-session"}),
                "Runtime.evaluate" => json!({"result": {"type": "number", "value": 2}}),
                _ => json!({}),
            };
            websocket
                .send(Message::Text(
                    json!({"id": command["id"], "result": result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
    });
    let repository = LockedServiceStateRepository::default_json().unwrap();
    let identity =
        crate::process_identity::capture_process_identity(std::process::id(), None, None).unwrap();
    RuntimeLifecycleAuthority::new(&repository)
        .register_managed_lane(ManagedLaneRegistration {
            logical_browser_id: "session:retained-owner".to_string(),
            profile_root: home.as_path().join("profile"),
            daemon_session_route: "retained-owner".to_string(),
            process_group_id: None,
            process_identity: identity,
            browser_family: "chrome".to_string(),
            cdp_endpoint: endpoint.clone(),
            target_ids: vec!["retained-target".to_string()],
        })
        .unwrap();
    repository
        .mutate(|service| {
            service.browsers.insert(
                "session:retained-owner".to_string(),
                BrowserProcess {
                    id: "session:retained-owner".to_string(),
                    profile_id: Some("retained-profile".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    pid: Some(std::process::id()),
                    cdp_endpoint: Some(endpoint.clone()),
                    active_session_ids: vec!["retained-owner".to_string()],
                    ..BrowserProcess::default()
                },
            );
            Ok(())
        })
        .unwrap();
    // An exact-looking handle cannot authorize attachment to a changed endpoint.
    repository
        .mutate(|service| {
            service
                .browsers
                .get_mut("session:retained-owner")
                .unwrap()
                .cdp_endpoint = Some("ws://127.0.0.1:1/foreign".to_string());
            Ok(())
        })
        .unwrap();
    let mut state = DaemonState::new();
    state.session_id = "retained-owner".to_string();
    let command = json!({
        "id": "retained-evaluate", "action": "evaluate", "script": "1+1",
        "timeoutMs": 1000, "maxReturnBytes": 256,
        "serviceTabHandle": {"valid": true, "browserId": "session:retained-owner",
            "sessionName": "retained-owner", "profileId": "retained-profile",
            "targetId": "retained-target", "tabId": "target:retained-target"}
    });
    // Missing owner evidence must stop the original-handle path before CDP.
    let owner_registry = repository.load_snapshot().unwrap().runtime_owner_registry;
    repository
        .mutate(|service| {
            service.runtime_owner_registry = Default::default();
            Ok(())
        })
        .unwrap();
    let missing_owner = execute_command(&command, &mut state).await;
    let failure = crate::native::service_failure::classify_service_failure(
        missing_owner["error"].as_str().unwrap(),
    );
    assert_eq!(
        failure.code, "service_tab_recovery_owner_missing",
        "{missing_owner}"
    );
    assert_eq!(
        failure.effect_state,
        crate::native::service_failure::ServiceEffectState::NoEffect
    );
    assert!(failure.executable_next_action.is_some());
    assert!(state.browser.is_none());
    assert!(methods.lock().unwrap().is_empty());
    repository
        .mutate(|service| {
            service.runtime_owner_registry = owner_registry.clone();
            service
                .browsers
                .get_mut("session:retained-owner")
                .unwrap()
                .pid = None;
            Ok(())
        })
        .unwrap();
    let missing_process = execute_command(&command, &mut state).await;
    let failure = crate::native::service_failure::classify_service_failure(
        missing_process["error"].as_str().unwrap(),
    );
    assert_eq!(
        failure.code, "service_tab_recovery_process_unproven",
        "{missing_process}"
    );
    assert_eq!(
        failure.effect_state,
        crate::native::service_failure::ServiceEffectState::NoEffect
    );
    assert!(failure.executable_next_action.is_some());
    assert!(state.browser.is_none());
    assert!(methods.lock().unwrap().is_empty());
    repository
        .mutate(|service| {
            service
                .browsers
                .get_mut("session:retained-owner")
                .unwrap()
                .pid = Some(std::process::id());
            Ok(())
        })
        .unwrap();
    let rejected = execute_command(&command, &mut state).await;
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .starts_with("service_tab_recovery_identity_mismatch:"),
        "{rejected}"
    );
    let failure = crate::native::service_failure::classify_service_failure(
        rejected["error"].as_str().unwrap(),
    );
    assert_eq!(failure.code, "service_tab_recovery_identity_mismatch");
    assert_eq!(
        failure.effect_state,
        crate::native::service_failure::ServiceEffectState::NoEffect
    );
    assert!(failure.executable_next_action.is_some());
    assert!(state.browser.is_none());
    assert!(methods.lock().unwrap().is_empty());
    repository
        .mutate(|service| {
            service
                .browsers
                .get_mut("session:retained-owner")
                .unwrap()
                .cdp_endpoint = Some(endpoint.clone());
            Ok(())
        })
        .unwrap();
    let mut unauthorized = command.clone();
    unauthorized["serviceTabHandle"]["profileAccess"] = json!({"subjectId": "foreign"});
    let rejected = execute_command(&unauthorized, &mut state).await;
    assert_eq!(
        rejected["error"],
        "profile child access requires a service-generated connection"
    );
    assert!(state.browser.is_none());
    assert!(methods.lock().unwrap().is_empty());
    let response = execute_command(
        &json!({
            "id": "retained-evaluate", "action": "evaluate", "script": "1+1",
            "timeoutMs": 1000, "maxReturnBytes": 256,
            "serviceTabHandle": {"valid": true, "browserId": "session:retained-owner",
                "sessionName": "retained-owner", "profileId": "retained-profile",
                "targetId": "retained-target", "tabId": "target:retained-target"}
        }),
        &mut state,
    )
    .await;
    peer.abort();
    if !target_present {
        assert_eq!(response["success"], false, "{response}");
        let failure = crate::native::service_failure::classify_service_failure(
            response["error"].as_str().unwrap(),
        );
        assert_eq!(failure.code, "service_tab_recovery_target_missing");
        assert_eq!(
            failure.effect_state,
            crate::native::service_failure::ServiceEffectState::NoEffect
        );
        assert!(failure.executable_next_action.is_some());
        assert!(state.browser.is_none());
        assert_eq!(*methods.lock().unwrap(), vec!["Target.getTargets"]);
        std::fs::remove_dir_all(home).unwrap();
        return;
    }
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["result"], 2, "{response}");
    let commands = methods.lock().unwrap();
    assert!(commands
        .iter()
        .any(|method| method == "Target.attachToTarget"));
    assert!(!commands.iter().any(|method| matches!(
        method.as_str(),
        "Target.createTarget" | "Browser.close" | "Target.closeTarget"
    )));
    assert!(!state
        .browser
        .as_ref()
        .unwrap()
        .owns_launched_browser_process());
    drop(commands);
    drop(state);
    std::fs::remove_dir_all(home).unwrap();
}
