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
use crate::native::service_retained_state::{
    prune_retained_service_state, repair_retained_service_state, ServiceRetentionPruneOptions,
    ServiceRetentionRepairOptions,
};
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
fn p117_red_profile_record_prune_does_not_reclaim_the_profile_directory() {
    let profile_root = env::temp_dir().join(format!(
        "agent-browser-p117-profile-prune-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&profile_root).unwrap();
    fs::write(profile_root.join("retained-evidence"), b"fixture").unwrap();
    let mut service_state = ServiceState {
        profiles: BTreeMap::from([(
            "managed-one-time-p117".to_string(),
            BrowserProfile {
                id: "managed-one-time-p117".to_string(),
                name: "P117 managed one-time fixture".to_string(),
                profile_class: ProfileClass::ManagedOneTime,
                user_data_dir: Some(profile_root.display().to_string()),
                persistent: false,
                ..BrowserProfile::default()
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

    assert_eq!(result["removed"]["orphanedProfiles"], 1);
    assert!(!service_state.profiles.contains_key("managed-one-time-p117"));
    assert!(profile_root.join("retained-evidence").is_file());

    fs::remove_dir_all(profile_root).unwrap();
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
