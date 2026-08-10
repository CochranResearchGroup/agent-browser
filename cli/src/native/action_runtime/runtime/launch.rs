#![allow(unused_imports)]
use super::capability::{
    close_behavior_for_attached_browser, close_behavior_for_launched_browser, service_browser_id,
};
use super::cdp_free_execute::{
    build_cdp_free_launch_plan, cdp_free_launch_response, launch_ios, launch_safari,
    validate_cdp_free_launch_plan,
};
use super::cdp_free_plan::{
    apply_launch_host_hints, apply_retained_remote_headed_launch_hints,
    apply_retained_remote_headed_metadata, manual_login_launch_from_command,
    optional_command_string, remote_headed_display_isolation, retained_remote_headed_launch_hint,
};
use super::daemon::{
    apply_service_browser_capability_selection, apply_service_profile_selection,
    keychain_password_from_env, launch_args_from_sources,
    launch_command_with_effective_service_defaults, launch_hash, launch_profile_from_sources,
    runtime_profile_from_env, runtime_profile_from_sources, use_real_keychain_from_env,
    CloseBehavior,
};
use super::profile_lease::apply_auto_launch_command_hints;
use super::recovery::{
    can_attach_managed_runtime_for_launch, managed_runtime_attach_target,
    retained_session_attach_target_for_auto_launch, runtime_profile_pid,
    shared_profile_attach_target_for_auto_launch, DaemonState, ManagedRuntimeAttachTarget,
    SharedProfileAttachTarget,
};
use super::remote_headed::{
    ensure_service_profile_lease_available, persist_current_browser_health,
    persist_service_browser_record,
};
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
use crate::native::network::resolve_fetch_paused;
use crate::native::network::{self, DomainFilter, EventTracker};
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::providers;
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
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceBrowserProcessIdentity, ServiceEntitySource,
    ServiceEvent, ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy,
    TabLifecycle, ViewStream, ViewStreamProvider, ViewerLease,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use crate::native::webdriver::ios;
use crate::native::webdriver::safari;
use crate::runtime_profile::{
    clear_runtime_state, looks_like_path, read_devtools_port, read_runtime_state,
    runtime_profile_user_data_dir,
};
use serde_json::{json, Map, Value};
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) fn shared_profile_auto_launch_acquisition_evidence(
    command: &Value,
    session_id: &str,
    target: &SharedProfileAttachTarget,
) -> Value {
    let requested_browser_id = optional_command_string(command, "browserId");
    let requested_session_name = optional_command_string(command, "sessionName");
    let owner_session_name = target
        .owner_session_ids
        .iter()
        .find(|session_id| !session_id.trim().is_empty())
        .map(String::as_str)
        .unwrap_or(session_id);
    let action = command
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("navigate");
    let route_hint_source = if requested_browser_id.is_some() || requested_session_name.is_some() {
        "request.browserId_sessionName"
    } else {
        "shared_profile_auto_launch"
    };
    let route_hint_fields: &[&str] = &["browserId", "sessionName"];
    let profile_id = Value::String(target.runtime_profile.clone());
    let requested_profile = optional_command_string(command, "runtimeProfile")
        .or_else(|| optional_command_string(command, "profile"))
        .unwrap_or_else(|| target.runtime_profile.clone());
    shared_profile_acquisition_result(SharedProfileAcquisitionResultInput {
        state: Some("opened"),
        mode: action,
        action: "opened_shared_profile_tab",
        recommended_action: Some("reuse_existing_browser"),
        browser_reused: true,
        tab_opened: true,
        browser_id: &target.browser_id,
        session_name: owner_session_name,
        profile_id: Some(&profile_id),
        requested_profile: Some(requested_profile.as_str()),
        planned_profile: Some(target.runtime_profile.as_str()),
        requested_browser_id: requested_browser_id.as_deref(),
        requested_session_name: requested_session_name.as_deref(),
        route_hint_source,
        route_hint_fields,
        route_bound: false,
        route_id: None,
        display_allocation_id: None,
        route_pool_entry_id: None,
        provider: None,
        provider_mode: None,
        tab_acquisition_decision: Some("opened_shared_profile_tab"),
    })
}
pub(crate) async fn attach_managed_runtime_browser(
    state: &mut DaemonState,
    target: &ManagedRuntimeAttachTarget,
    leave_open: bool,
    metadata: ServiceLaunchMetadata,
) -> Result<(), String> {
    state.reset_input_state();
    state.attached_runtime_profile = Some(target.runtime_profile.clone());
    state.attached_browser_pid = Some(target.browser_pid);
    state.close_behavior = close_behavior_for_attached_browser(true, leave_open);
    state.browser = Some(BrowserManager::connect_cdp(&target.cdp_port.to_string()).await?);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_current_browser_health(
        state,
        ServiceBrowserHost::AttachedExisting,
        ServiceBrowserHealth::Ready,
        Some(metadata),
    );
    Ok(())
}
pub(crate) async fn attach_shared_profile_browser_for_auto_launch(
    state: &mut DaemonState,
    target: &SharedProfileAttachTarget,
    command: &Value,
    leave_open: bool,
    metadata: ServiceLaunchMetadata,
) -> Result<(), String> {
    state.reset_input_state();
    state.attached_runtime_profile = Some(target.runtime_profile.clone());
    state.attached_browser_pid = target.browser_pid;
    state.close_behavior = close_behavior_for_attached_browser(true, leave_open);
    let mut mgr = BrowserManager::connect_cdp(&target.cdp_endpoint).await?;
    mgr.tab_new(None).await.map_err(|err| {
        format!(
            "shared_profile_tab_acquisition_failed: browserId={} profileId={} owners={:?}: {}",
            target.browser_id, target.runtime_profile, target.owner_session_ids, err
        )
    })?;
    state.pending_shared_profile_acquisition = Some(
        shared_profile_auto_launch_acquisition_evidence(command, &state.session_id, target),
    );
    state.browser = Some(mgr);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_current_browser_health(
        state,
        ServiceBrowserHost::AttachedExisting,
        ServiceBrowserHealth::Ready,
        Some(metadata),
    );
    Ok(())
}
pub(crate) async fn attach_retained_service_session_browser_for_auto_launch(
    state: &mut DaemonState,
    target: &SharedProfileAttachTarget,
) -> Result<(), String> {
    state.reset_input_state();
    state.attached_runtime_profile = Some(target.runtime_profile.clone());
    state.attached_browser_pid = target.browser_pid;
    state.close_behavior = CloseBehavior::Detach;
    state.browser = Some(BrowserManager::connect_cdp(&target.cdp_endpoint).await?);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    Ok(())
}
pub(crate) fn env_u64_or_default(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}
pub(crate) fn browser_recovery_policy_config_from_env() -> BrowserRecoveryPolicyConfig {
    let defaults = BrowserRecoveryPolicyConfig::default();
    BrowserRecoveryPolicyConfig {
        retry_budget: env_u64_or_default(
            "AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET",
            defaults.retry_budget,
        ),
        base_backoff_ms: env_u64_or_default(
            "AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS",
            defaults.base_backoff_ms,
        ),
        max_backoff_ms: env_u64_or_default(
            "AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS",
            defaults.max_backoff_ms,
        ),
        source: BrowserRecoveryPolicySource {
            retry_budget: browser_recovery_policy_source_from_env(
                "AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET",
                "AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET_SOURCE",
            ),
            base_backoff_ms: browser_recovery_policy_source_from_env(
                "AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS",
                "AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS_SOURCE",
            ),
            max_backoff_ms: browser_recovery_policy_source_from_env(
                "AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS",
                "AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS_SOURCE",
            ),
        },
    }
}
pub(crate) fn browser_recovery_policy_source_from_env(
    value_name: &str,
    source_name: &str,
) -> BrowserRecoveryPolicyValueSource {
    env::var(source_name)
        .ok()
        .map(|value| BrowserRecoveryPolicyValueSource::from_str(&value))
        .unwrap_or_else(|| {
            if env::var(value_name).is_ok() {
                BrowserRecoveryPolicyValueSource::Env
            } else {
                BrowserRecoveryPolicyValueSource::Default
            }
        })
}
pub(crate) async fn terminate_runtime_browser(
    runtime_profile: Option<String>,
    pid: u32,
) -> BrowserShutdownOutcome {
    tokio::task::spawn_blocking(move || {
        let mut outcome = BrowserShutdownOutcome::default();
        let recorded = match runtime_browser_termination_identity(runtime_profile.as_deref(), pid) {
            Ok(Some(recorded)) => recorded,
            Ok(None) => return outcome,
            Err(error) => {
                outcome.errors.push(error);
                return outcome;
            }
        };
        let process = match crate::process_identity::VerifiedProcessTermination::open(&recorded) {
            Ok(Some(process)) => process,
            Ok(None) => return outcome,
            Err(error) => {
                outcome.errors.push(error);
                return outcome;
            }
        };
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            match process.is_running() {
                Ok(true) => {}
                Ok(false) => return outcome,
                Err(error) => {
                    outcome.errors.push(error);
                    return outcome;
                }
            }
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            outcome.polite_close_attempted = true;
            match process.signal(crate::process_identity::VerifiedProcessSignal::Terminate) {
                Ok(true) => {}
                Ok(false) => {
                    outcome.polite_close_succeeded = true;
                    return outcome;
                }
                Err(error) => {
                    outcome.errors.push(error);
                    outcome.polite_close_failed = true;
                    return outcome;
                }
            }
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                match process.is_running() {
                    Ok(true) => {}
                    Ok(false) => {
                        outcome.polite_close_succeeded = true;
                        return outcome;
                    }
                    Err(error) => {
                        outcome.errors.push(error);
                        return outcome;
                    }
                }
            }
            outcome.polite_close_failed = true;
            outcome.force_kill_attempted = true;
            match process.signal(crate::process_identity::VerifiedProcessSignal::Kill) {
                Ok(true) => {}
                Ok(false) => {
                    outcome.force_kill_succeeded = true;
                    return outcome;
                }
                Err(error) => {
                    outcome.errors.push(error);
                    outcome.force_kill_failed = true;
                    return outcome;
                }
            }
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                match process.is_running() {
                    Ok(true) => {}
                    Ok(false) => {
                        outcome.force_kill_succeeded = true;
                        return outcome;
                    }
                    Err(error) => {
                        outcome.errors.push(error);
                        return outcome;
                    }
                }
            }
            outcome.errors.push(format!(
                "Runtime browser PID {} survived force kill; OS may be degraded",
                pid
            ));
            outcome.force_kill_failed = true;
        }
        #[cfg(windows)]
        {
            outcome.force_kill_attempted = true;
            match process.signal(crate::process_identity::VerifiedProcessSignal::Kill) {
                Ok(_) => outcome.force_kill_succeeded = true,
                Err(error) => outcome.errors.push(error),
            }
            if outcome.force_kill_attempted && !outcome.force_kill_succeeded {
                outcome.force_kill_failed = true;
            }
        }
        outcome
    })
    .await
    .unwrap_or_else(|err| BrowserShutdownOutcome {
        force_kill_attempted: true,
        force_kill_failed: true,
        errors: vec![format!(
            "Failed to join runtime browser termination task: {}",
            err
        )],
        ..BrowserShutdownOutcome::default()
    })
}

fn runtime_browser_termination_identity(
    runtime_profile: Option<&str>,
    pid: u32,
) -> Result<Option<crate::process_identity::RecordedProcessIdentity>, String> {
    let assessment = crate::runtime_profile::runtime_process_assessment(runtime_profile, pid);
    if assessment.ownership == crate::process_identity::RuntimeProcessOwnership::Missing {
        return Ok(None);
    }
    if !assessment.authorizes_adoption() {
        return Err(format!(
            "Refusing to signal PID {} because runtime browser ownership is not proven ({})",
            pid, assessment.reason
        ));
    }
    let runtime_profile = runtime_profile.ok_or_else(|| {
        format!(
            "Refusing to signal PID {} without an authoritative runtime profile identity",
            pid
        )
    })?;
    let state = crate::runtime_profile::read_runtime_state(runtime_profile)?
        .ok_or_else(|| format!("Refusing to signal PID {} without runtime state", pid))?;
    if state.browser_pid != pid {
        return Err(format!(
            "Refusing to signal PID {} because runtime state records PID {}",
            pid, state.browser_pid
        ));
    }
    state
        .process_identity
        .ok_or_else(|| {
            format!(
            "Refusing to signal PID {} because legacy runtime state has no exact process identity",
            pid
        )
        })
        .map(Some)
}
impl Drop for DaemonState {
    fn drop(&mut self) {
        if let Some(task) = self.fetch_handler_task.take() {
            task.abort();
        }
        if let Some(task) = self.dialog_handler_task.take() {
            task.abort();
        }
    }
}
/// Connect to a running Chrome via auto-discovery and open a fresh tab so
/// subsequent navigations don't hijack the user's existing tabs.
pub(crate) async fn connect_auto_with_fresh_tab() -> Result<BrowserManager, String> {
    let mut mgr = BrowserManager::connect_auto().await?;
    mgr.tab_new(None).await?;
    let session_id = mgr.active_session_id()?.to_string();
    let _ = mgr
        .client
        .send_command("Page.bringToFront", None, Some(&session_id))
        .await;
    Ok(mgr)
}
pub(crate) async fn focus_remote_headed_launch_for_view(
    mgr: &BrowserManager,
    options: &LaunchOptions,
) -> Option<Value> {
    if !options.remote_headed || options.headless {
        return None;
    }
    let mut last_error = None;
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
        match mgr.focus_for_view(true).await {
            Ok(result) => return Some(result),
            Err(err) => last_error = Some(err),
        }
    }
    Some(json!(
        { "broughtToFront" : false, "maximizeRequested" : true, "maximized" : false,
        "maximizeError" : last_error.unwrap_or_else(||
        "Remote-headed view focus failed".to_string()), }
    ))
}
pub(crate) fn should_retry_transient_chrome_predevtools_launch_error(
    engine: Option<&str>,
    error: &str,
) -> bool {
    if engine.unwrap_or("chrome") != "chrome" {
        return false;
    }
    error.contains("Chrome exited early")
        && error.contains("without exposing DevTools")
        && error.contains("UtilAcceptVsock")
        && error.contains("accept4 failed 110")
}
pub(crate) async fn launch_browser_with_transient_retry(
    options: LaunchOptions,
    engine: Option<&str>,
) -> Result<BrowserManager, String> {
    match BrowserManager::launch(options.clone(), engine).await {
        Ok(mgr) => Ok(mgr),
        Err(first_error)
            if should_retry_transient_chrome_predevtools_launch_error(engine, &first_error) =>
        {
            tokio::time::sleep(std::time::Duration::from_millis(750)).await;
            BrowserManager::launch(options, engine)
                .await
                .map_err(|second_error| {
                    format!(
                        "{second_error}\nRetried once after transient WSL pre-DevTools Chrome launch failure: {first_error}"
                    )
                })
        }
        Err(error) => Err(error),
    }
}
pub(crate) async fn auto_launch(state: &mut DaemonState, command: &Value) -> Result<(), String> {
    state.pending_shared_profile_acquisition = None;
    let mut options = launch_options_from_env();
    let leave_open = env::var("AGENT_BROWSER_LEAVE_OPEN")
        .is_ok_and(|v| !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no" | ""));
    let runtime_attach_managed = env::var("AGENT_BROWSER_RUNTIME_ATTACH_MANAGED")
        .is_ok_and(|v| !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no" | ""));
    if let Some(ref server) = state.stream_server {
        options.viewport_size = Some(server.viewport().await);
    }
    let engine = env::var("AGENT_BROWSER_ENGINE").ok();
    if let Some(target) = retained_session_attach_target_for_auto_launch(command, &state.session_id)
    {
        attach_retained_service_session_browser_for_auto_launch(state, &target).await?;
        return Ok(());
    }
    let retained_remote_headed = retained_remote_headed_launch_hint(&state.session_id, command);
    let (service_host, selection_reason, browser_capability_launch, effective_command) =
        apply_auto_launch_command_hints(&mut options, command, retained_remote_headed.as_ref());
    let mut metadata = ServiceLaunchMetadata::from_launch_options(
        &options,
        Some(&effective_command),
        selection_reason,
    );
    apply_retained_remote_headed_metadata(&mut metadata, retained_remote_headed.as_ref());
    metadata.browser_capability_launch = Some(browser_capability_launch.to_value());
    if let Some(target) = shared_profile_attach_target_for_auto_launch(
        &metadata,
        &effective_command,
        &state.session_id,
    ) {
        attach_shared_profile_browser_for_auto_launch(
            state,
            &target,
            &effective_command,
            leave_open,
            metadata,
        )
        .await?;
        return Ok(());
    }
    ensure_service_profile_lease_available(&metadata, &state.session_id, &effective_command)
        .await?;
    let has_proxy_auth = options.proxy_username.is_some();
    if has_proxy_auth {
        let mut creds = state.proxy_credentials.write().await;
        *creds = Some((
            options.proxy_username.clone().unwrap_or_default(),
            options.proxy_password.clone().unwrap_or_default(),
        ));
    }
    state.engine = engine.as_deref().unwrap_or("chrome").to_string();
    write_engine_file(&state.session_id, &state.engine);
    write_extensions_file(&state.session_id);
    if let Ok(cdp) = env::var("AGENT_BROWSER_CDP") {
        let mgr = BrowserManager::connect_cdp(&cdp).await?;
        state.reset_input_state();
        state.attached_runtime_profile = if runtime_attach_managed {
            options.runtime_profile.clone()
        } else {
            None
        };
        state.attached_browser_pid = if runtime_attach_managed {
            runtime_profile_pid(options.runtime_profile.as_deref())
        } else {
            None
        };
        state.close_behavior =
            close_behavior_for_attached_browser(runtime_attach_managed, leave_open);
        state.browser = Some(mgr);
        state.subscribe_to_browser_events();
        state.start_fetch_handler();
        state.start_dialog_handler();
        state.update_stream_client().await;
        persist_current_browser_health(
            state,
            ServiceBrowserHost::AttachedExisting,
            ServiceBrowserHealth::Ready,
            Some(metadata),
        );
        try_auto_restore_state(state).await;
        return Ok(());
    }
    if env::var("AGENT_BROWSER_AUTO_CONNECT").is_ok() {
        state.reset_input_state();
        state.attached_runtime_profile = None;
        state.attached_browser_pid = None;
        state.close_behavior = CloseBehavior::Detach;
        state.browser = Some(connect_auto_with_fresh_tab().await?);
        state.subscribe_to_browser_events();
        state.start_fetch_handler();
        state.start_dialog_handler();
        state.update_stream_client().await;
        persist_current_browser_health(
            state,
            ServiceBrowserHost::AttachedExisting,
            ServiceBrowserHealth::Ready,
            None,
        );
        try_auto_restore_state(state).await;
        return Ok(());
    }
    if let Ok(provider) = env::var("AGENT_BROWSER_PROVIDER") {
        let p = provider.to_lowercase();
        if !p.is_empty() && p != "ios" && p != "safari" {
            let conn = providers::connect_provider(&p).await?;
            let ws_headers = if p == "agentcore" {
                providers::take_agentcore_ws_headers()
            } else {
                None
            };
            let connect_result = if conn.direct_page {
                BrowserManager::connect_cdp_direct(&conn.ws_url).await
            } else if ws_headers.is_some() {
                BrowserManager::connect_cdp_with_headers(&conn.ws_url, ws_headers).await
            } else {
                BrowserManager::connect_cdp(&conn.ws_url).await
            };
            match connect_result {
                Ok(mgr) => {
                    state.reset_input_state();
                    state.attached_runtime_profile = None;
                    state.attached_browser_pid = None;
                    state.close_behavior = CloseBehavior::CloseBrowser;
                    state.browser = Some(mgr);
                    state.subscribe_to_browser_events();
                    state.start_fetch_handler();
                    state.start_dialog_handler();
                    state.update_stream_client().await;
                    write_provider_file(&state.session_id, &p);
                    persist_current_browser_health(
                        state,
                        ServiceBrowserHost::CloudProvider,
                        ServiceBrowserHealth::Ready,
                        None,
                    );
                    try_auto_restore_state(state).await;
                    return Ok(());
                }
                Err(e) => {
                    if let Some(ref ps) = conn.session {
                        providers::close_provider_session(ps).await;
                    }
                    return Err(format!("Provider '{}' connection failed: {}", p, e));
                }
            }
        }
    }
    let hash = launch_hash(&options);
    if engine.as_deref().unwrap_or("chrome") == "chrome"
        && can_attach_managed_runtime_for_launch(&options)
    {
        if let Some(target) = managed_runtime_attach_target(options.runtime_profile.as_deref()) {
            attach_managed_runtime_browser(state, &target, leave_open, metadata).await?;
            state.launch_hash = Some(hash);
            return Ok(());
        }
    }
    let remote_focus_options = options.clone();
    let mgr = launch_browser_with_transient_retry(options, engine.as_deref()).await?;
    let _ = focus_remote_headed_launch_for_view(&mgr, &remote_focus_options).await;
    state.reset_input_state();
    state.attached_runtime_profile = None;
    state.attached_browser_pid = None;
    state.close_behavior =
        close_behavior_for_launched_browser(mgr.runtime_profile_name(), leave_open);
    state.browser = Some(mgr);
    state.launch_hash = Some(hash);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_current_browser_health(
        state,
        service_host,
        ServiceBrowserHealth::Ready,
        Some(metadata),
    );
    if has_proxy_auth {
        if let Some(ref mgr) = state.browser {
            if let Ok(session_id) = mgr.active_session_id() {
                let _ = network::install_domain_filter_fetch(&mgr.client, session_id, true).await;
            }
        }
    }
    try_auto_restore_state(state).await;
    Ok(())
}
pub(crate) fn launch_options_from_env() -> LaunchOptions {
    let headed = env::var("AGENT_BROWSER_HEADED")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    let extensions: Option<Vec<String>> = env::var("AGENT_BROWSER_EXTENSIONS").ok().map(|v| {
        v.split([',', '\n'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });
    LaunchOptions {
        headless: !headed,
        executable_path: env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok(),
        proxy: env::var("AGENT_BROWSER_PROXY").ok(),
        proxy_bypass: env::var("AGENT_BROWSER_PROXY_BYPASS").ok(),
        proxy_username: env::var("AGENT_BROWSER_PROXY_USERNAME").ok(),
        proxy_password: env::var("AGENT_BROWSER_PROXY_PASSWORD").ok(),
        profile: env::var("AGENT_BROWSER_PROFILE").ok(),
        runtime_profile: runtime_profile_from_env(),
        expected_browser_family: None,
        allow_file_access: env::var("AGENT_BROWSER_ALLOW_FILE_ACCESS")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false),
        args: env::var("AGENT_BROWSER_ARGS")
            .map(|v| {
                v.split([',', '\n'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
        extensions,
        storage_state: env::var("AGENT_BROWSER_STATE").ok(),
        user_agent: env::var("AGENT_BROWSER_USER_AGENT").ok(),
        ignore_https_errors: env::var("AGENT_BROWSER_IGNORE_HTTPS_ERRORS")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false),
        color_scheme: env::var("AGENT_BROWSER_COLOR_SCHEME").ok(),
        download_path: env::var("AGENT_BROWSER_DOWNLOAD_PATH").ok(),
        viewport_size: None,
        use_real_keychain: use_real_keychain_from_env(),
        keychain_password: keychain_password_from_env(),
        manual_login: false,
        attachable: false,
        display: None,
        remote_headed: false,
        remote_headed_display_isolation: None,
    }
}
pub(crate) async fn try_auto_restore_state(state: &mut DaemonState) {
    let session_name = match state.session_name.as_deref() {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => return,
    };
    if let Some(path) = state::find_auto_state_file(&session_name) {
        if let Some(ref mgr) = state.browser {
            if let Ok(session_id) = mgr.active_session_id() {
                let _ = state::load_state(&mgr.client, session_id, &path).await;
            }
        }
    }
}
pub(crate) async fn handle_launch(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let headless = cmd
        .get("headless")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let cdp_url = cmd.get("cdpUrl").and_then(|v| v.as_str());
    let cdp_port = cmd.get("cdpPort").and_then(|v| v.as_u64());
    let auto_connect = cmd
        .get("autoConnect")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_cdp = cdp_url.is_some() || cdp_port.is_some();
    let leave_open = cmd
        .get("leaveOpen")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let runtime_attach_managed = cmd
        .get("runtimeAttachManaged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let manual_login_launch = manual_login_launch_from_command(cmd, headless)?;
    let viewport_size = cmd.get("viewport").and_then(|viewport| {
        let width = viewport.get("width").and_then(|v| v.as_u64())?;
        let height = viewport.get("height").and_then(|v| v.as_u64())?;
        Some((width as u32, height as u32))
    });
    let extensions: Option<Vec<String>> =
        cmd.get("extensions").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
    let storage_state = cmd.get("storageState").and_then(|v| v.as_str());
    let mut launch_options = LaunchOptions {
        headless,
        executable_path: cmd
            .get("executablePath")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok()),
        proxy: cmd.get("proxy").and_then(|v| {
            v.as_str().map(|s| s.to_string()).or_else(|| {
                v.get("server")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
        }),
        proxy_bypass: cmd
            .get("proxy")
            .and_then(|v| v.get("bypass"))
            .and_then(|v| v.as_str())
            .map(String::from),
        proxy_username: cmd
            .get("proxy")
            .and_then(|v| v.get("username"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| env::var("AGENT_BROWSER_PROXY_USERNAME").ok()),
        proxy_password: cmd
            .get("proxy")
            .and_then(|v| v.get("password"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| env::var("AGENT_BROWSER_PROXY_PASSWORD").ok()),
        profile: launch_profile_from_sources(cmd, !(runtime_attach_managed && has_cdp)),
        runtime_profile: runtime_profile_from_sources(cmd, true),
        expected_browser_family: cmd
            .get("runtimeProfileBrowserFamily")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        allow_file_access: cmd
            .get("allowFileAccess")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        args: launch_args_from_sources(cmd),
        extensions,
        storage_state: storage_state.map(String::from),
        user_agent: cmd
            .get("userAgent")
            .and_then(|v| v.as_str())
            .map(String::from),
        ignore_https_errors: cmd
            .get("ignoreHTTPSErrors")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        color_scheme: cmd
            .get("colorScheme")
            .and_then(|v| v.as_str())
            .map(String::from),
        download_path: cmd
            .get("downloadPath")
            .and_then(|v| v.as_str())
            .map(String::from),
        viewport_size,
        use_real_keychain: use_real_keychain_from_env(),
        keychain_password: keychain_password_from_env(),
        manual_login: manual_login_launch,
        attachable: false,
        display: None,
        remote_headed: false,
        remote_headed_display_isolation: None,
    };
    let effective_cmd = launch_command_with_effective_service_defaults(cmd, &launch_options);
    let retained_remote_headed = retained_remote_headed_launch_hint(&state.session_id, cmd);
    apply_retained_remote_headed_launch_hints(&mut launch_options, retained_remote_headed.as_ref());
    let service_host = apply_launch_host_hints(&mut launch_options, &effective_cmd);
    let selection_reason = apply_service_profile_selection(&mut launch_options, &effective_cmd);
    let browser_capability_launch =
        apply_service_browser_capability_selection(&mut launch_options, &effective_cmd);
    let mut metadata = ServiceLaunchMetadata::from_launch_options(
        &launch_options,
        Some(&effective_cmd),
        selection_reason,
    );
    apply_retained_remote_headed_metadata(&mut metadata, retained_remote_headed.as_ref());
    metadata.browser_capability_launch = Some(browser_capability_launch.to_value());
    ensure_service_profile_lease_available(&metadata, &state.session_id, &effective_cmd).await?;
    let new_hash = launch_hash(&launch_options);
    super::super::super::browser::validate_launch_options(
        launch_options.extensions.as_deref(),
        has_cdp,
        launch_options.profile.as_deref(),
        storage_state,
        launch_options.allow_file_access,
        launch_options.executable_path.as_deref(),
    )?;
    let needs_relaunch = if let Some(ref mut mgr) = state.browser {
        let is_external = cdp_url.is_some() || cdp_port.is_some() || auto_connect;
        let was_external = mgr.is_cdp_connection();
        let already_owns_managed_runtime = runtime_attach_managed
            && is_external
            && launch_options
                .runtime_profile
                .as_deref()
                .is_some_and(|runtime| {
                    mgr.runtime_profile_name() == Some(runtime)
                        && runtime_profile_pid(Some(runtime))
                            .is_none_or(|pid| mgr.browser_pid() == Some(pid))
                });
        if already_owns_managed_runtime {
            false
        } else {
            let hash_changed = !is_external && state.launch_hash != Some(new_hash);
            is_external != was_external
                || hash_changed
                || mgr.has_process_exited()
                || !mgr.is_connection_alive().await
        }
    } else {
        true
    };
    if needs_relaunch {
        if let Some(ref mut b) = state.browser {
            b.close().await?;
            state.browser = None;
            state.launch_hash = None;
            state.attached_runtime_profile = None;
            state.attached_browser_pid = None;
            state.close_behavior = CloseBehavior::CloseBrowser;
            state.screencasting = false;
            state.reset_input_state();
            state.update_stream_client().await;
        }
    } else {
        persist_current_browser_health(
            state,
            service_host,
            ServiceBrowserHealth::Ready,
            Some(metadata),
        );
        return Ok(json!({ "launched" : true, "reused" : true }));
    }
    state.ref_map.clear();
    if let Some(url) = cdp_url {
        state.reset_input_state();
        state.attached_runtime_profile = if runtime_attach_managed {
            launch_options.runtime_profile.clone()
        } else {
            None
        };
        state.attached_browser_pid = if runtime_attach_managed {
            runtime_profile_pid(launch_options.runtime_profile.as_deref())
        } else {
            None
        };
        state.close_behavior =
            close_behavior_for_attached_browser(runtime_attach_managed, leave_open);
        state.browser = Some(BrowserManager::connect_cdp(url).await?);
        state.subscribe_to_browser_events();
        state.start_fetch_handler();
        state.start_dialog_handler();
        state.update_stream_client().await;
        persist_current_browser_health(
            state,
            service_host,
            ServiceBrowserHealth::Ready,
            Some(metadata),
        );
        return Ok(json!({ "launched" : true }));
    }
    if let Some(port) = cdp_port {
        state.reset_input_state();
        state.attached_runtime_profile = if runtime_attach_managed {
            launch_options.runtime_profile.clone()
        } else {
            None
        };
        state.attached_browser_pid = if runtime_attach_managed {
            runtime_profile_pid(launch_options.runtime_profile.as_deref())
        } else {
            None
        };
        state.close_behavior =
            close_behavior_for_attached_browser(runtime_attach_managed, leave_open);
        state.browser = Some(BrowserManager::connect_cdp(&port.to_string()).await?);
        state.subscribe_to_browser_events();
        state.start_fetch_handler();
        state.start_dialog_handler();
        state.update_stream_client().await;
        persist_current_browser_health(
            state,
            service_host,
            ServiceBrowserHealth::Ready,
            Some(metadata),
        );
        return Ok(json!({ "launched" : true }));
    }
    if auto_connect {
        state.reset_input_state();
        state.attached_runtime_profile = None;
        state.attached_browser_pid = None;
        state.close_behavior = CloseBehavior::Detach;
        state.browser = Some(connect_auto_with_fresh_tab().await?);
        state.subscribe_to_browser_events();
        state.start_fetch_handler();
        state.start_dialog_handler();
        state.update_stream_client().await;
        persist_current_browser_health(state, service_host, ServiceBrowserHealth::Ready, None);
        return Ok(json!({ "launched" : true }));
    }
    if let Some(provider) = cmd.get("provider").and_then(|v| v.as_str()) {
        match provider.to_lowercase().as_str() {
            "ios" => {
                return launch_ios(cmd, state).await;
            }
            "safari" => {
                return launch_safari(cmd, state).await;
            }
            _ => {
                let conn = providers::connect_provider(provider).await?;
                let ws_headers = if provider.eq_ignore_ascii_case("agentcore") {
                    providers::take_agentcore_ws_headers()
                } else {
                    None
                };
                let connect_result = if conn.direct_page {
                    BrowserManager::connect_cdp_direct(&conn.ws_url).await
                } else if ws_headers.is_some() {
                    BrowserManager::connect_cdp_with_headers(&conn.ws_url, ws_headers).await
                } else {
                    BrowserManager::connect_cdp(&conn.ws_url).await
                };
                match connect_result {
                    Ok(mgr) => {
                        state.reset_input_state();
                        state.attached_runtime_profile = None;
                        state.attached_browser_pid = None;
                        state.close_behavior = CloseBehavior::CloseBrowser;
                        state.browser = Some(mgr);
                        state.subscribe_to_browser_events();
                        state.start_fetch_handler();
                        state.start_dialog_handler();
                        state.update_stream_client().await;
                        write_provider_file(&state.session_id, provider);
                        persist_current_browser_health(
                            state,
                            service_host,
                            ServiceBrowserHealth::Ready,
                            None,
                        );
                        if let Some(info) = providers::get_agentcore_info() {
                            return Ok(json!(
                                { "launched" : true, "provider" : provider,
                                "agentCoreSessionId" : info.session_id,
                                "agentCoreLiveViewUrl" : info.live_view_url }
                            ));
                        }
                        return Ok(json!({ "launched" : true, "provider" : provider }));
                    }
                    Err(e) => {
                        if let Some(ref ps) = conn.session {
                            providers::close_provider_session(ps).await;
                        }
                        return Err(e);
                    }
                }
            }
        }
    }
    let engine = cmd
        .get("engine")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| env::var("AGENT_BROWSER_ENGINE").ok());
    let has_proxy_auth = launch_options.proxy_username.is_some();
    if has_proxy_auth {
        let mut creds = state.proxy_credentials.write().await;
        *creds = Some((
            launch_options.proxy_username.clone().unwrap_or_default(),
            launch_options.proxy_password.clone().unwrap_or_default(),
        ));
    }
    if let Some(ref domains) = cmd
        .get("allowedDomains")
        .and_then(|v| v.as_str())
        .map(String::from)
    {
        let mut df = state.domain_filter.write().await;
        *df = Some(DomainFilter::new(domains));
    }
    state.engine = engine.as_deref().unwrap_or("chrome").to_string();
    write_engine_file(&state.session_id, &state.engine);
    write_extensions_file(&state.session_id);
    if engine.as_deref().unwrap_or("chrome") == "chrome"
        && can_attach_managed_runtime_for_launch(&launch_options)
    {
        if let Some(target) =
            managed_runtime_attach_target(launch_options.runtime_profile.as_deref())
        {
            attach_managed_runtime_browser(state, &target, leave_open, metadata).await?;
            state.launch_hash = Some(new_hash);
            return Ok(json!(
                { "launched" : true, "attachedToExistingBrowser" : true,
                "runtimeProfile" : target.runtime_profile, "browserPid" : target
                .browser_pid, "cdpPort" : target.cdp_port, }
            ));
        }
    }
    let remote_focus_options = launch_options.clone();
    state.reset_input_state();
    state.attached_runtime_profile = None;
    state.attached_browser_pid = None;
    let launched_browser =
        launch_browser_with_transient_retry(launch_options, engine.as_deref()).await?;
    let remote_view_focus =
        focus_remote_headed_launch_for_view(&launched_browser, &remote_focus_options).await;
    state.browser = Some(launched_browser);
    state.close_behavior = close_behavior_for_launched_browser(
        state
            .browser
            .as_ref()
            .and_then(|mgr| mgr.runtime_profile_name()),
        leave_open,
    );
    state.launch_hash = Some(new_hash);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_current_browser_health(
        state,
        service_host,
        ServiceBrowserHealth::Ready,
        Some(metadata),
    );
    {
        let df = state.domain_filter.read().await;
        let has_domain_filter = df.is_some();
        if has_domain_filter || has_proxy_auth {
            if let Some(ref mgr) = state.browser {
                if let Ok(session_id) = mgr.active_session_id() {
                    if let Some(ref filter) = *df {
                        let _ = network::install_domain_filter(
                            &mgr.client,
                            session_id,
                            &filter.allowed_domains,
                            has_proxy_auth,
                        )
                        .await;
                        network::sanitize_existing_pages(&mgr.client, &mgr.pages_list(), filter)
                            .await;
                    } else {
                        let _ = network::install_domain_filter_fetch(
                            &mgr.client,
                            session_id,
                            has_proxy_auth,
                        )
                        .await;
                    }
                }
            }
        }
    }
    let mut response = json!({ "launched" : true });
    if let Some(remote_view_focus) = remote_view_focus {
        response["viewFocus"] = remote_view_focus;
    }
    Ok(response)
}
pub(crate) async fn handle_cdp_free_launch(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let plan = build_cdp_free_launch_plan(cmd)?;
    ensure_service_profile_lease_available(&plan.metadata, &state.session_id, cmd).await?;
    validate_cdp_free_launch_plan(&plan)?;
    let launch = launch_chrome_detached(&plan.launch_options)?;
    let process_identity = crate::process_identity::capture_process_identity(
        launch.pid,
        plan.launch_options
            .executable_path
            .as_deref()
            .map(Path::new),
        plan.launch_options.expected_browser_family.as_deref(),
    )
    .map(|process_identity| ServiceBrowserProcessIdentity {
        process_identity,
        user_data_dir: Some(launch.user_data_dir.to_string_lossy().into_owned()),
        runtime_profile: launch.runtime_profile.clone(),
    });
    persist_service_browser_record(
        &state.session_id,
        ServiceBrowserHost::LocalHeaded,
        ServiceBrowserHealth::Ready,
        Some(launch.pid),
        None,
        None,
        Some(plan.metadata),
        process_identity,
    );
    Ok(cdp_free_launch_response(
        state,
        &plan.launch_options,
        &launch,
        plan.url,
    ))
}
pub(crate) async fn handle_external_byop_adopt(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let profile_id = optional_command_string(cmd, "runtimeProfile")
        .or_else(|| optional_command_string(cmd, "profileId"))
        .ok_or_else(|| {
            "external_byop_adopt requires runtimeProfile or profileId for a registered external_byop profile"
                .to_string()
        })?;
    let cdp_url = optional_command_string(cmd, "cdpUrl");
    let cdp_port = cmd.get("cdpPort").and_then(Value::as_u64);
    if cdp_url.is_some() == cdp_port.is_some() {
        return Err("external_byop_adopt requires exactly one of cdpUrl or cdpPort".to_string());
    }
    let repository = LockedServiceStateRepository::default_json()?;
    let service_state = repository.load_snapshot()?;
    let profile = service_state.profiles.get(&profile_id).ok_or_else(|| {
        format!(
            "external_byop_adopt profile '{}' is not registered",
            profile_id
        )
    })?;
    if profile.profile_origin != ProfileOrigin::ExternalByop {
        return Err(format!(
            "external_byop_adopt requires profileOrigin external_byop; profile '{}' is {:?}",
            profile_id, profile.profile_origin
        ));
    }
    if let Some(mgr) = state.browser.as_mut() {
        if mgr.is_connection_alive().await {
            return Err(
                "external_byop_adopt requires an idle service session; route the request to a new sessionName or close the current browser first"
                    .to_string(),
            );
        }
    }
    state.browser = None;
    state.launch_hash = None;
    state.reset_input_state();
    state.attached_runtime_profile = None;
    state.attached_browser_pid = None;
    state.close_behavior = CloseBehavior::Detach;
    state.screencasting = false;
    let mgr = if let Some(url) = cdp_url.as_deref() {
        BrowserManager::connect_cdp(url).await?
    } else {
        BrowserManager::connect_cdp(&cdp_port.unwrap().to_string()).await?
    };
    state.browser = Some(mgr);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    let metadata = ServiceLaunchMetadata {
        profile_id: Some(profile_id.clone()),
        profile_name: Some(profile.name.clone()),
        user_data_dir: profile.user_data_dir.clone(),
        persistent_profile: true,
        keyring: profile.keyring,
        service_name: optional_command_string(cmd, "serviceName").or_else(|| {
            profile
                .registration
                .as_ref()
                .and_then(|registration| registration.service_name.clone())
        }),
        agent_name: optional_command_string(cmd, "agentName"),
        task_name: optional_command_string(cmd, "taskName"),
        cleanup: SessionCleanupPolicy::Detach,
        profile_selection_reason: Some(ProfileSelectionReason::ExplicitProfile),
        browser_stderr_log_path: None,
        browser_capability_launch: None,
        view_streams: Vec::new(),
        display_isolation: None,
        display_name: None,
    };
    persist_current_browser_health(
        state,
        ServiceBrowserHost::AttachedExisting,
        ServiceBrowserHealth::Ready,
        Some(metadata),
    );
    let open_url = optional_command_string(cmd, "url").unwrap_or_else(|| "about:blank".to_string());
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let opened = mgr.tab_new(Some(open_url.as_str())).await?;
    let target_id = opened
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "external_byop_adopt opened a tab without targetId".to_string())?
        .to_string();
    let url = mgr.get_url().await.unwrap_or(open_url);
    let title = mgr.get_title().await.unwrap_or_default();
    let service_tab_handle =
        external_byop_service_tab_handle(&state.session_id, &target_id, &url, &title, &profile_id);
    persist_external_byop_adopted_tab(
        cmd,
        &state.session_id,
        &profile_id,
        &target_id,
        &url,
        &title,
        &service_tab_handle,
    )?;
    Ok(json!(
        { "ok" : true, "action" : "external_byop_adopt", "adopted" : true,
        "browserId" : service_browser_id(& state.session_id), "sessionName" : state
        .session_id, "profileId" : profile_id, "profileOrigin" : "external_byop",
        "browserHost" : ServiceBrowserHost::AttachedExisting, "targetId" : target_id,
        "url" : url, "title" : title, "tabNew" : opened, "serviceTabHandle" :
        service_tab_handle, }
    ))
}
pub(crate) fn external_byop_service_tab_handle(
    session_id: &str,
    target_id: &str,
    url: &str,
    title: &str,
    profile_id: &str,
) -> Value {
    let browser_id = service_browser_id(session_id);
    let tab_id = format!("target:{target_id}");
    json!(
        { "browserId" : browser_id, "sessionName" : session_id, "tabId" : tab_id,
        "targetId" : target_id, "url" : url, "title" : title, "profileId" : profile_id,
        "profileOrigin" : "external_byop", "leaseId" : session_id, "leaseState" :
        "shared", "cleanupPolicy" : "detach", "leaseHeartbeatExpected" : true,
        "ownerSessionId" : session_id, "jobId" : Value::Null, "traceFilter" : {
        "browserId" : service_browser_id(session_id), "profileId" : profile_id,
        "sessionId" : session_id, }, "valid" : true, "staleReason" : Value::Null, }
    )
}
pub(crate) fn persist_external_byop_adopted_tab(
    cmd: &Value,
    session_id: &str,
    profile_id: &str,
    target_id: &str,
    url: &str,
    title: &str,
    service_tab_handle: &Value,
) -> Result<(), String> {
    let handle: ServiceTabHandle = serde_json::from_value(service_tab_handle.clone())
        .map_err(|err| format!("Invalid adopted service tab handle: {}", err))?;
    let repository = LockedServiceStateRepository::default_json()?;
    let browser_id = service_browser_id(session_id);
    let tab_id = format!("target:{target_id}");
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let service_name = optional_command_string(cmd, "serviceName");
    let agent_name = optional_command_string(cmd, "agentName");
    let task_name = optional_command_string(cmd, "taskName");
    repository.mutate(|state| {
        state.tabs.insert(
            tab_id.clone(),
            BrowserTab {
                id: tab_id.clone(),
                browser_id: browser_id.clone(),
                target_id: Some(target_id.to_string()),
                session_id: Some(session_id.to_string()),
                lifecycle: TabLifecycle::Ready,
                url: Some(url.to_string()),
                title: (!title.is_empty()).then(|| title.to_string()),
                owner_session_id: Some(session_id.to_string()),
                service_tab_handle: Some(handle.clone()),
                ..BrowserTab::default()
            },
        );
        if let Some(session) = state.sessions.get_mut(session_id) {
            if !session.tab_ids.contains(&tab_id) {
                session.tab_ids.push(tab_id.clone());
            }
        }
        if let Some(browser) = state.browsers.get_mut(&browser_id) {
            if !browser.active_session_ids.iter().any(|id| id == session_id) {
                browser.active_session_ids.push(session_id.to_string());
            }
        }
        state.events.push(ServiceEvent {
            id: format!("external-byop-adopt-{}-{}", session_id, observed_at),
            timestamp: observed_at.clone(),
            kind: ServiceEventKind::TabLifecycleChanged,
            message: format!("External BYOP browser adopted for profile {}.", profile_id),
            browser_id: Some(browser_id.clone()),
            profile_id: Some(profile_id.to_string()),
            session_id: Some(session_id.to_string()),
            service_name,
            agent_name,
            task_name,
            details: Some(json!(
                { "action" : "external_byop_adopt", "targetId" : target_id,
                "tabId" : tab_id, "url" : url, }
            )),
            ..ServiceEvent::default()
        });
        if state.events.len() > 100 {
            let excess = state.events.len() - 100;
            state.events.drain(0..excess);
        }
        Ok(())
    })
}
