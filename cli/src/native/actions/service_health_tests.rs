#![allow(unused_imports)]
use super::*;
use crate::native::action_runtime::cancellation::{cancellable, cancellation_error};
use crate::native::action_runtime::runtime::process_exit_observation_details;
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
