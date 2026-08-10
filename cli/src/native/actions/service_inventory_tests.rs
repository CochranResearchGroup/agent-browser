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
