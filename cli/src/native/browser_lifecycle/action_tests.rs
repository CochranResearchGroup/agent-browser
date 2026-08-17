#![allow(unused_imports)]
use super::*;
use crate::native::action_runtime::cancellation::{cancellable, cancellation_error};
use crate::native::action_runtime::runtime::{
    active_browser_profile_mismatch_message, should_retry_transient_chrome_predevtools_launch_error,
};
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
fn test_remote_view_open_reacquires_active_intent_target_when_retained_id_expired() {
    let pages = vec![
        PageInfo {
            target_id: "compatible-first".to_string(),
            session_id: "session-first".to_string(),
            url: "https://accounts.example.com/sign-in/first".to_string(),
            title: "First".to_string(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "compatible-active".to_string(),
            session_id: "session-active".to_string(),
            url: "https://accounts.example.com/sign-in/active".to_string(),
            title: "Active".to_string(),
            target_type: "page".to_string(),
        },
        PageInfo {
            target_id: "wrong-intent".to_string(),
            session_id: "session-wrong".to_string(),
            url: "https://other.example.com/".to_string(),
            title: "Other".to_string(),
            target_type: "page".to_string(),
        },
    ];

    let target = remote_view_open_reacquired_live_target(
        &pages,
        Some("expired-target"),
        Some("compatible-active"),
        Some("https://accounts.example.com/"),
    )
    .unwrap();

    assert_eq!(target.target_id, "compatible-active");
}

#[test]
fn test_remote_view_open_reacquisition_rejects_same_origin_wrong_path() {
    let pages = vec![PageInfo {
        target_id: "wrong-article".to_string(),
        session_id: "session-wrong".to_string(),
        url: "https://example.com/other".to_string(),
        title: "Other".to_string(),
        target_type: "page".to_string(),
    }];

    assert!(remote_view_open_reacquired_live_target(
        &pages,
        Some("expired-target"),
        Some("wrong-article"),
        Some("https://example.com/article"),
    )
    .is_none());
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
