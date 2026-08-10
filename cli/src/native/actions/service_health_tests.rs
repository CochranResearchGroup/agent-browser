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
        force_kill_failed: false,
        errors: vec!["CDP connection closed".to_string()],
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
