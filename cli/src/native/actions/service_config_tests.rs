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

#[tokio::test]
async fn test_service_config_actions_mutate_persisted_state() {
    // Keep each dispatcher future off this long scenario's debug-mode stack.
    // The scenario still exercises the real dispatcher with the default test stack.
    async fn execute_config_command(cmd: &Value, state: &mut DaemonState) -> Value {
        Box::pin(super::execute_command(cmd, state)).await
    }

    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-config-actions-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    let mut state = DaemonState::new();
    let upsert_profile = execute_config_command(
        &json!(
            { "action" : "service_profile_upsert", "id" : "svc-profile-upsert-1",
            "profileId" : "journal-downloader", "profile" : { "name" :
            "Journal Downloader", "allocation" : "per_service", "keyring" :
            "basic_password_store", "persistent" : true, "sharedServiceIds" :
            ["JournalDownloader"] } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_profile["success"], true);
    assert_service_profile_upsert_response_contract(&upsert_profile["data"]);
    assert_eq!(
        upsert_profile["data"]["profile"]["id"],
        "journal-downloader"
    );
    let freshness = execute_config_command(
        &json!(
            { "action" : "service_profile_freshness_update", "id" :
            "svc-profile-freshness-1", "profileId" : "journal-downloader",
            "freshness" : { "loginId" : "google", "readinessState" : "fresh",
            "readinessEvidence" : "auth_probe_cookie_present", "lastVerifiedAt" :
            "2026-05-06T12:00:00Z", "freshnessExpiresAt" : "2026-05-06T13:00:00Z" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(freshness["success"], true);
    assert_service_profile_upsert_response_contract(&freshness["data"]);
    assert_eq!(
        freshness["data"]["profile"]["targetReadiness"][0]["state"],
        "fresh"
    );
    assert_eq!(
        freshness["data"]["profile"]["authenticatedServiceIds"][0],
        "google"
    );
    let handoff = execute_config_command(
        &json!(
            { "action" : "service_profile_seeding_handoff_update", "id" :
            "svc-profile-seeding-handoff-1", "profileId" : "journal-downloader",
            "handoff" : { "targetServiceId" : "google", "state" :
            "seeding_launched_detached", "pid" : 1234, "startedAt" :
            "2026-05-10T12:00:00Z", "expiresAt" : "2026-05-10T12:30:00Z", "actor" :
            "operator", "note" : "manual seeding started" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(handoff["success"], true);
    assert_eq!(
        handoff["data"]["handoff"]["id"],
        "journal-downloader:google"
    );
    assert_eq!(
        handoff["data"]["seedingHandoff"]["operatorIntervention"]["state"],
        "seeding_launched_detached"
    );
    assert_eq!(
        store.load().unwrap().profile_seeding_handoffs["journal-downloader:google"].state,
        ProfileSeedingHandoffState::SeedingLaunchedDetached
    );
    let upsert_session = execute_config_command(
        &json!(
            { "action" : "service_session_upsert", "id" : "svc-session-upsert-1",
            "sessionId" : "journal-run", "session" : { "serviceName" :
            "JournalDownloader", "agentName" : "codex", "taskName" :
            "probeACSwebsite", "profileId" : "journal-downloader", "lease" :
            "exclusive", "cleanup" : "close_browser" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_session["success"], true);
    assert_service_session_upsert_response_contract(&upsert_session["data"]);
    assert_eq!(upsert_session["data"]["session"]["id"], "journal-run");
    let upsert_policy = execute_config_command(
        &json!(
            { "action" : "service_site_policy_upsert", "id" : "svc-policy-upsert-1",
            "sitePolicyId" : "google", "sitePolicy" : { "originPattern" :
            "https://accounts.google.com", "interactionMode" : "human_like_input" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_policy["success"], true);
    assert_service_site_policy_upsert_response_contract(&upsert_policy["data"]);
    assert_eq!(upsert_policy["data"]["sitePolicy"]["id"], "google");
    let upsert_provider = execute_config_command(
        &json!(
            { "action" : "service_provider_upsert", "id" : "svc-provider-upsert-1",
            "providerId" : "manual", "provider" : { "kind" : "manual_approval",
            "displayName" : "Dashboard approval", "capabilities" : ["human_approval"]
            } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_provider["success"], true);
    assert_service_provider_upsert_response_contract(&upsert_provider["data"]);
    assert_eq!(upsert_provider["data"]["provider"]["id"], "manual");
    let upsert_browser_capability = execute_config_command(
        &json!(
            { "action" : "service_browser_capability_registry_upsert", "id" :
            "svc-browser-capability-upsert-1", "collection" : "browserHosts",
            "recordId" : "local-linux", "record" : { "name" : "Local Linux host",
            "serviceName" : "JournalDownloader" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_browser_capability["success"], true);
    assert_service_browser_capability_registry_upsert_response_contract(
        &upsert_browser_capability["data"],
    );
    assert_eq!(
        upsert_browser_capability["data"]["record"]["id"],
        "local-linux"
    );
    assert_eq!(
        upsert_browser_capability["data"]["counts"]["browserHosts"],
        1
    );
    let upsert_monitor = execute_config_command(
        &json!(
            { "action" : "service_monitor_upsert", "id" : "svc-monitor-upsert-1",
            "monitorId" : "google-login-freshness", "monitor" : { "name" :
            "Google login freshness", "target" : { "site_policy" : "google" },
            "intervalMs" : 60000, "state" : "paused" } }
        ),
        &mut state,
    )
    .await;
    assert_eq!(upsert_monitor["success"], true);
    assert_service_monitor_upsert_response_contract(&upsert_monitor["data"]);
    assert_eq!(
        upsert_monitor["data"]["monitor"]["id"],
        "google-login-freshness"
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.profiles["journal-downloader"].shared_service_ids,
        vec!["JournalDownloader".to_string()]
    );
    assert_eq!(
        persisted.sessions["journal-run"].service_name.as_deref(),
        Some("JournalDownloader")
    );
    assert_eq!(
        persisted.site_policies["google"].origin_pattern,
        "https://accounts.google.com"
    );
    assert_eq!(
        persisted.providers["manual"].display_name,
        "Dashboard approval"
    );
    assert_eq!(
        persisted.browser_capability_registry.browser_hosts[0]["id"],
        "local-linux"
    );
    assert_eq!(
        persisted.monitors["google-login-freshness"].name,
        "Google login freshness"
    );
    let resume_monitor = execute_config_command(
        &json!(
            { "action" : "service_monitor_resume", "id" : "svc-monitor-resume-1",
            "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(resume_monitor["success"], true);
    assert_service_monitor_state_response_contract(&resume_monitor["data"]);
    assert_eq!(resume_monitor["data"]["state"], "active");
    assert_eq!(
        store.load().unwrap().monitors["google-login-freshness"].state,
        MonitorState::Active
    );
    let pause_monitor = execute_config_command(
        &json!(
            { "action" : "service_monitor_pause", "id" : "svc-monitor-pause-1",
            "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(pause_monitor["success"], true);
    assert_service_monitor_state_response_contract(&pause_monitor["data"]);
    assert_eq!(pause_monitor["data"]["state"], "paused");
    let reset_monitor = execute_config_command(
        &json!(
            { "action" : "service_monitor_reset_failures", "id" :
            "svc-monitor-reset-1", "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(reset_monitor["success"], true);
    assert_service_monitor_state_response_contract(&reset_monitor["data"]);
    assert_eq!(reset_monitor["data"]["resetFailures"], true);
    assert_eq!(reset_monitor["data"]["monitor"]["consecutiveFailures"], 0);
    let mut persisted = store.load().unwrap();
    let monitor = persisted
        .monitors
        .get_mut("google-login-freshness")
        .expect("monitor should exist");
    monitor.state = MonitorState::Faulted;
    monitor.consecutive_failures = 2;
    monitor.last_result = Some("site_policy_missing".to_string());
    persisted.events.push(ServiceEvent {
        id: "event-google-login-freshness-failed".to_string(),
        timestamp: "2026-04-22T00:00:00Z".to_string(),
        kind: ServiceEventKind::ReconciliationError,
        message: "Service monitor google-login-freshness failed".to_string(),
        details: Some(json!(
            { "incidentId" : "monitor:google-login-freshness", "monitorId" :
            "google-login-freshness", "monitorResult" : "site_policy_missing",
            "monitorTarget" : { "site_policy" : "google" } }
        )),
        ..ServiceEvent::default()
    });
    persisted.refresh_derived_views();
    store.save(&persisted).unwrap();
    let triage_monitor = execute_config_command(
        &json!(
            { "action" : "service_monitor_triage", "id" : "svc-monitor-triage-1",
            "monitorId" : "google-login-freshness", "by" : "operator", "note" :
            "reviewed" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(triage_monitor["success"], true);
    assert_service_monitor_triage_response_contract(&triage_monitor["data"]);
    assert_eq!(triage_monitor["data"]["acknowledged"], true);
    assert_eq!(
        triage_monitor["data"]["incident"]["monitorId"],
        "google-login-freshness"
    );
    assert_eq!(
        triage_monitor["data"]["incident"]["acknowledgedBy"],
        "operator"
    );
    assert_eq!(triage_monitor["data"]["monitor"]["consecutiveFailures"], 0);
    let mut persisted = store.load().unwrap();
    let monitor = persisted
        .monitors
        .get_mut("google-login-freshness")
        .expect("monitor should exist");
    monitor.state = MonitorState::Faulted;
    monitor.consecutive_failures = 3;
    monitor.last_result = Some("site_policy_missing".to_string());
    persisted.events.push(ServiceEvent {
        id: "event-google-login-freshness-failed-again".to_string(),
        timestamp: "2026-04-22T00:05:00Z".to_string(),
        kind: ServiceEventKind::ReconciliationError,
        message: "Service monitor google-login-freshness failed again".to_string(),
        details: Some(json!(
            { "incidentId" : "monitor:google-login-freshness", "monitorId" :
            "google-login-freshness", "monitorResult" : "site_policy_missing",
            "monitorTarget" : { "site_policy" : "google" } }
        )),
        ..ServiceEvent::default()
    });
    persisted.refresh_derived_views();
    store.save(&persisted).unwrap();
    let apply_remedies = execute_config_command(
        &json!(
            { "action" : "service_remedies_apply", "id" : "svc-remedies-apply-1",
            "escalation" : "monitor_attention", "by" : "operator", "note" :
            "reviewed group" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(apply_remedies["success"], true);
    assert_service_remedies_apply_response_contract(&apply_remedies["data"]);
    assert_eq!(apply_remedies["data"]["count"], 1);
    assert_eq!(
        apply_remedies["data"]["monitorIds"],
        json!(["google-login-freshness"])
    );
    let delete_session = execute_config_command(
        &json!(
            { "action" : "service_session_delete", "id" : "svc-session-delete-1",
            "sessionId" : "journal-run" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_session["success"], true);
    assert_service_session_delete_response_contract(&delete_session["data"]);
    assert_eq!(delete_session["data"]["deleted"], true);
    assert!(!store.load().unwrap().sessions.contains_key("journal-run"));
    let delete_profile = execute_config_command(
        &json!(
            { "action" : "service_profile_delete", "id" : "svc-profile-delete-1",
            "profileId" : "journal-downloader" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_profile["success"], true);
    assert_service_profile_delete_response_contract(&delete_profile["data"]);
    assert_eq!(delete_profile["data"]["deleted"], true);
    assert!(!store
        .load()
        .unwrap()
        .profiles
        .contains_key("journal-downloader"));
    let delete_provider = execute_config_command(
        &json!(
            { "action" : "service_provider_delete", "id" : "svc-provider-delete-1",
            "providerId" : "manual" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_provider["success"], true);
    assert_service_provider_delete_response_contract(&delete_provider["data"]);
    assert_eq!(delete_provider["data"]["deleted"], true);
    assert!(!store.load().unwrap().providers.contains_key("manual"));
    let delete_monitor = execute_config_command(
        &json!(
            { "action" : "service_monitor_delete", "id" : "svc-monitor-delete-1",
            "monitorId" : "google-login-freshness" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_monitor["success"], true);
    assert_service_monitor_delete_response_contract(&delete_monitor["data"]);
    assert_eq!(delete_monitor["data"]["deleted"], true);
    assert!(!store
        .load()
        .unwrap()
        .monitors
        .contains_key("google-login-freshness"));
    let delete_policy = execute_config_command(
        &json!(
            { "action" : "service_site_policy_delete", "id" : "svc-policy-delete-1",
            "sitePolicyId" : "google" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(delete_policy["success"], true);
    assert_service_site_policy_delete_response_contract(&delete_policy["data"]);
    assert_eq!(delete_policy["data"]["deleted"], true);
    let loaded_after_policy_delete = store.load().unwrap();
    assert_eq!(
        loaded_after_policy_delete.site_policy_source("google"),
        Some(crate::native::service_model::ServiceEntitySource::Builtin)
    );
    assert_eq!(
        loaded_after_policy_delete.site_policies["google"].origin_pattern,
        "https://accounts.google.com"
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_job_cancel_response_matches_contract() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-job-cancel-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
    store
        .save(&ServiceState {
            jobs: BTreeMap::from([(
                "job-queued".to_string(),
                ServiceJob {
                    id: "job-queued".to_string(),
                    action: "navigate".to_string(),
                    state: JobState::Queued,
                    submitted_at: Some("2026-04-22T00:00:00Z".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_job_cancel", "id" : "svc-job-cancel-1", "jobId" :
            "job-queued", "reason" : "stale" }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_job_cancel_response_contract(&result["data"]);
    assert_eq!(result["data"]["cancelled"], true);
    assert_eq!(result["data"]["job"]["state"], "cancelled");
    assert_eq!(result["data"]["job"]["error"], "stale");
    assert_eq!(
        store.load().unwrap().jobs["job-queued"].state,
        JobState::Cancelled
    );
    let _ = fs::remove_dir_all(&home);
}
#[tokio::test]
async fn test_service_reconcile_response_matches_contract() {
    let guard = EnvGuard::new(&["HOME"]);
    let home = unique_socket_dir("service-reconcile-response-home");
    fs::create_dir_all(&home).unwrap();
    guard.set("HOME", home.to_str().unwrap());
    let mut state = DaemonState::new();
    let result = execute_command(
        &json!(
            { "action" : "service_reconcile", "id" : "svc-reconcile-response-1",
            "serviceState" : ServiceState::default(), }
        ),
        &mut state,
    )
    .await;
    assert_eq!(result["success"], true);
    assert_service_reconcile_response_contract(&result["data"]);
    assert_eq!(result["data"]["reconciled"], true);
    let _ = fs::remove_dir_all(&home);
}
