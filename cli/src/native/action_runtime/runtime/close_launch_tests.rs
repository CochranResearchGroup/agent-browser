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
#[cfg(target_os = "linux")]
use crate::native::service_lease_authority::{ProtectedBrowserOwner, ProtectedBrowserOwnerLease};
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
fn test_close_behavior_for_attached_browser_defaults_to_detach_for_external_attach() {
    assert_eq!(
        close_behavior_for_attached_browser(false, false),
        CloseBehavior::Detach
    );
    assert_eq!(
        close_behavior_for_attached_browser(false, true),
        CloseBehavior::Detach
    );
}
#[test]
fn test_close_behavior_for_attached_browser_closes_managed_runtime_by_default() {
    assert_eq!(
        close_behavior_for_attached_browser(true, false),
        CloseBehavior::CloseBrowser
    );
}
#[test]
fn test_close_behavior_for_attached_browser_respects_leave_open_override() {
    assert_eq!(
        close_behavior_for_attached_browser(true, true),
        CloseBehavior::Detach
    );
}
#[test]
fn test_close_behavior_for_launched_browser_detaches_only_for_named_runtime_profiles() {
    assert_eq!(
        close_behavior_for_launched_browser(Some("google-login"), true),
        CloseBehavior::Detach
    );
    assert_eq!(
        close_behavior_for_launched_browser(Some("google-login"), false),
        CloseBehavior::CloseBrowser
    );
    assert_eq!(
        close_behavior_for_launched_browser(None, true),
        CloseBehavior::CloseBrowser
    );
}

#[tokio::test]
async fn failed_owned_launch_persistence_runs_cleanup_exactly_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let cleanup_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = cleanup_calls.clone();
    let result =
        require_owned_launch_persistence(Err("registration rejected".to_string()), || async move {
            observed_calls.fetch_add(1, Ordering::SeqCst);
            Ok(BrowserShutdownOutcome {
                exact_process_exited: true,
                profile_lock_released: true,
                ..BrowserShutdownOutcome::default()
            })
        })
        .await;

    assert_eq!(cleanup_calls.load(Ordering::SeqCst), 1);
    let error = result.expect_err("failed persistence must fail the launch");
    assert!(error.contains("registration rejected"));
    assert!(error.contains("launched_browser_cleanup"));
}

#[tokio::test]
async fn successful_owned_launch_persistence_skips_cleanup() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let cleanup_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = cleanup_calls.clone();
    require_owned_launch_persistence(Ok(()), || async move {
        observed_calls.fetch_add(1, Ordering::SeqCst);
        Ok(BrowserShutdownOutcome::default())
    })
    .await
    .expect("successful persistence must retain the browser");

    assert_eq!(cleanup_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn rejected_protected_launch_completion_cleans_up_and_terminalizes_uncertainty() {
    use std::sync::{Arc, Mutex};

    let observations = Arc::new(Mutex::new(Vec::new()));
    let completion_observations = observations.clone();
    let cleanup_observations = observations.clone();
    let uncertainty_observations = observations.clone();

    let result = require_protected_launch_completion(
        || {
            completion_observations
                .lock()
                .unwrap()
                .push("complete".to_string());
            Err::<String, _>("owner commit rejected".to_string())
        },
        || async move {
            cleanup_observations
                .lock()
                .unwrap()
                .push("cleanup".to_string());
            Ok(BrowserShutdownOutcome {
                exact_process_exited: true,
                profile_lock_released: true,
                ..BrowserShutdownOutcome::default()
            })
        },
        |evidence_digest| {
            assert!(evidence_digest.starts_with("sha256:"));
            uncertainty_observations
                .lock()
                .unwrap()
                .push("uncertain".to_string());
            Ok(())
        },
    )
    .await;

    assert_eq!(
        *observations.lock().unwrap(),
        vec!["complete", "cleanup", "uncertain"]
    );
    let error = result.expect_err("rejected completion must fail the protected launch");
    assert!(error.contains("owner commit rejected"));
    assert!(error.contains("exact_process_exited"));
    assert!(error.contains("profile_lock_released"));
    assert!(error.contains("uncertainty=recorded"));
}

#[cfg(target_os = "linux")]
#[test]
fn confirmed_protected_close_reconciles_exact_owner_and_clears_custody() {
    use crate::native::service_lease_authority::{
        ProtectedBrowserOwner, ProtectedBrowserOwnerLease,
    };

    let mut state = DaemonState::new();
    state.session_id = "session:protected-close".to_string();
    state.protected_browser_owner = Some(ProtectedBrowserOwnerLease {
        raw_capability: "capability-secret".to_string(),
        profile_id: "last30days-facebook".to_string(),
        owner: ProtectedBrowserOwner {
            authority_receipt_id: "effect-receipt:protected-close".to_string(),
            owner_id: "owner:protected-close".to_string(),
            owner_generation: 7,
            logical_browser_id: "browser:session:protected-close".to_string(),
            daemon_session_route: state.session_id.clone(),
            process_instance_digest: format!("sha256:{}", "1".repeat(64)),
            process_pid: 42007,
            revision: 11,
        },
    });
    let shutdown = BrowserShutdownOutcome {
        exact_process_exited: true,
        profile_lock_released: true,
        ..BrowserShutdownOutcome::default()
    };

    crate::native::action_runtime::runtime::navigation::reconcile_closed_protected_browser_owner_with(
        &mut state,
        &shutdown,
        |request| {
            assert_eq!(request.raw_capability, "capability-secret");
            assert_eq!(request.profile_id, "last30days-facebook");
            assert_eq!(request.expected_owner_id, "owner:protected-close");
            assert_eq!(request.expected_owner_generation, 7);
            assert!(request
                .idempotency_key
                .starts_with("protected-owner-close-reconcile:"));
            Ok(())
        },
    )
    .expect("confirmed close should reconcile the exact protected owner");

    assert!(state.protected_browser_owner.is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn protected_owner_custody_survives_projection_failure() {
    let mut state = DaemonState::new();
    let lease = ProtectedBrowserOwnerLease {
        raw_capability: "profile-capability".to_string(),
        profile_id: "profile-a".to_string(),
        owner: ProtectedBrowserOwner {
            authority_receipt_id: "effect-receipt:projection-failure".to_string(),
            owner_id: "owner-a".to_string(),
            owner_generation: 7,
            logical_browser_id: "browser-a".to_string(),
            daemon_session_route: "protected-projection-failure".to_string(),
            process_instance_digest: "a".repeat(64),
            process_pid: 4242,
            revision: 11,
        },
    };

    let error = crate::native::action_runtime::runtime::launch::retain_protected_browser_owner_before_projection(
        &mut state,
        lease,
        |_state, _owner| Err("projection_failed".to_string()),
    )
    .unwrap_err();

    assert_eq!(error, "projection_failed");
    let retained = state
        .protected_browser_owner
        .as_ref()
        .expect("committed owner custody must survive a derived projection failure");
    assert_eq!(retained.owner.owner_id, "owner-a");
    assert_eq!(retained.owner.owner_generation, 7);
}

#[cfg(target_os = "linux")]
#[test]
fn protected_launch_hints_use_exact_command_identity_without_session_reconciliation() {
    let mut options = LaunchOptions::default();
    let command = json!({
        "action": "tab_new",
        "profileId": "last30days-facebook",
        "profile": "/srv/agent-browser/profiles/last30days-facebook",
        "sessionName": "principal-profile-protected",
        "serviceName": "Last30days",
    });

    let (_, selection_reason, _, effective_command) =
        crate::native::action_runtime::runtime::profile_lease::apply_protected_auto_launch_command_hints(
            &mut options,
            &command,
        )
        .expect("protected hints should accept exact profile identity");

    assert_eq!(
        options.profile.as_deref(),
        Some("/srv/agent-browser/profiles/last30days-facebook")
    );
    assert_eq!(
        options.runtime_profile.as_deref(),
        Some("last30days-facebook")
    );
    assert_eq!(
        selection_reason,
        Some(ProfileSelectionReason::ExplicitProfile)
    );
    assert_eq!(effective_command["profileId"], "last30days-facebook");
}

#[cfg(target_os = "linux")]
#[test]
fn protected_launch_start_failure_is_uncertain_and_never_retryable() {
    let mut observed_digest = None;
    let error = protected_launch_start_failure("devtools endpoint unavailable", |digest| {
        observed_digest = Some(digest.to_string());
        Ok(())
    });

    let digest = observed_digest.expect("start failure must terminalize uncertainty");
    assert!(digest.starts_with("sha256:"));
    assert!(error.contains("devtools endpoint unavailable"));
    assert!(error.contains("uncertainty=recorded"));
    assert!(error.contains("automatic_retry=forbidden"));
}
#[test]
fn managed_close_terminal_evidence_requires_exit_and_profile_unlock() {
    let incomplete = BrowserShutdownOutcome {
        exact_process_exited: true,
        ..BrowserShutdownOutcome::default()
    };
    assert!(
        crate::native::action_runtime::runtime::navigation::browser_terminal_evidence(&incomplete)
            .is_none()
    );

    let complete = BrowserShutdownOutcome {
        exact_process_exited: true,
        profile_lock_released: true,
        ..BrowserShutdownOutcome::default()
    };
    assert_eq!(
        crate::native::action_runtime::runtime::navigation::browser_terminal_evidence(&complete),
        Some(vec![
            "exact_process_exited".to_string(),
            "profile_lock_released".to_string(),
        ])
    );

    let crashed = BrowserShutdownOutcome {
        polite_close_failed: true,
        exact_process_exited: true,
        profile_lock_released: true,
        errors: vec!["CDP channel already closed".to_string()],
        ..BrowserShutdownOutcome::default()
    };
    assert!(
        crate::native::action_runtime::runtime::navigation::browser_terminal_evidence(&crashed)
            .is_some()
    );
}
#[test]
fn test_launch_profile_from_sources_prefers_command_then_env() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_PROFILE"]);
    guard.set("AGENT_BROWSER_PROFILE", "/tmp/env-profile");
    assert_eq!(
        launch_profile_from_sources(&json!({}), true).as_deref(),
        Some("/tmp/env-profile")
    );
    assert_eq!(
        launch_profile_from_sources(&json!({ "profile" : "/tmp/cmd-profile" }), true).as_deref(),
        Some("/tmp/cmd-profile")
    );
    assert_eq!(
        launch_profile_from_sources(&json!({}), false).as_deref(),
        None
    );
    guard.remove("AGENT_BROWSER_PROFILE");
    assert_eq!(launch_profile_from_sources(&json!({}), true), None);
}
#[test]
fn test_launch_args_from_sources_prefers_command_then_env() {
    let guard = EnvGuard::new(&["AGENT_BROWSER_ARGS"]);
    guard.set("AGENT_BROWSER_ARGS", "--no-sandbox,--disable-gpu\n--foo");
    assert_eq!(
        launch_args_from_sources(&json!({})),
        vec![
            "--no-sandbox".to_string(),
            "--disable-gpu".to_string(),
            "--foo".to_string()
        ]
    );
    assert_eq!(
        launch_args_from_sources(&json!({ "args" : ["--command-arg"] })),
        vec!["--command-arg".to_string()]
    );
    guard.remove("AGENT_BROWSER_ARGS");
    assert!(launch_args_from_sources(&json!({})).is_empty());
}
