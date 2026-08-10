#![allow(unused_imports)]
use super::super::browser_operations::{
    add_manual_login_hint_warning, har_cdp_protocol_to_http_version, har_extract_headers,
    persist_service_owned_navigate_tab, resolve_fetch_paused, stream_file_path, write_engine_file,
    write_extensions_file, write_provider_file,
};
use super::super::common::*;
use super::super::service_workflows::{runtime_handoff_path, write_runtime_handoff};
use super::capability::service_browser_id;
use super::cdp_free_plan::{
    apply_launch_host_hints, apply_retained_remote_headed_launch_hints, optional_command_string,
    RetainedRemoteHeadedLaunchHint,
};
use super::daemon::{
    apply_service_browser_capability_selection, apply_service_profile_selection,
    launch_command_with_effective_service_defaults, launch_profile_from_sources,
    runtime_profile_from_sources, use_real_keychain_from_env, BrowserCapabilityLaunchResolution,
    ProfileLeasePolicy, ServiceProfileLeaseGate, DEFAULT_PROFILE_LEASE_WAIT_TIMEOUT_MS,
    PROFILE_LEASE_WAIT_POLL_MS,
};
use super::recovery::DaemonState;
pub(crate) fn service_profile_lease_gate(
    command: &Value,
    session_id: &str,
    waited_ms: Option<u64>,
) -> Result<ServiceProfileLeaseGate, String> {
    let Some(metadata) = service_profile_lease_metadata_for_command(command) else {
        return Ok(ServiceProfileLeaseGate::Ready);
    };
    let Some(profile_id) = metadata.profile_id.as_deref() else {
        return Ok(ServiceProfileLeaseGate::Ready);
    };
    let reusable_browser_ids = service_profile_live_reusable_browser_ids(session_id, profile_id);
    if !reusable_browser_ids.is_empty()
        && !allow_duplicate_profile_lane_from_command(command)
        && command.get("browserId").is_none()
        && command.get("sessionName").is_none()
    {
        return Ok(ServiceProfileLeaseGate::Reject {
            error: service_duplicate_profile_lane_error(
                &metadata,
                profile_id,
                &reusable_browser_ids,
            ),
        });
    }
    let policy = profile_lease_policy_from_command(command)?;
    let wait_timeout_ms = profile_lease_wait_timeout_ms_from_command(command)?;
    let conflict_session_ids =
        service_profile_lease_conflict_session_ids(&metadata, session_id, profile_id);
    if conflict_session_ids.is_empty() {
        return Ok(ServiceProfileLeaseGate::Ready);
    }
    if policy == ProfileLeasePolicy::Reject {
        return Ok(ServiceProfileLeaseGate::Reject {
            error: service_profile_lease_conflict_error(
                &metadata,
                profile_id,
                &conflict_session_ids,
                None,
            ),
        });
    }
    if waited_ms.unwrap_or_default() >= wait_timeout_ms {
        return Ok(ServiceProfileLeaseGate::Reject {
            error: service_profile_lease_conflict_error(
                &metadata,
                profile_id,
                &conflict_session_ids,
                Some(wait_timeout_ms),
            ),
        });
    }
    Ok(ServiceProfileLeaseGate::Wait {
        retry_after_ms: PROFILE_LEASE_WAIT_POLL_MS,
        profile_id: profile_id.to_string(),
        conflict_session_ids,
    })
}
pub(crate) fn service_profile_lease_metadata_for_command(
    command: &Value,
) -> Option<ServiceLaunchMetadata> {
    if command
        .get("action")
        .and_then(|value| value.as_str())
        .is_some_and(|action| {
            action.starts_with("service_")
                || matches!(action, "runtime_handoff_prepare" | "runtime_handoff_resume")
        })
    {
        return None;
    }
    let mut launch_options = LaunchOptions {
        profile: launch_profile_from_sources(command, true),
        runtime_profile: runtime_profile_from_sources(command, true),
        expected_browser_family: command
            .get("runtimeProfileBrowserFamily")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        use_real_keychain: use_real_keychain_from_env(),
        ..LaunchOptions::default()
    };
    let selection_reason = apply_service_profile_selection(&mut launch_options, command);
    let metadata = ServiceLaunchMetadata::from_launch_options(
        &launch_options,
        Some(command),
        selection_reason,
    );
    (metadata.service_name.is_some() && metadata.profile_id.is_some()).then_some(metadata)
}
pub(crate) fn apply_explicit_launch_identity_from_command(
    options: &mut LaunchOptions,
    command: &Value,
) {
    if let Some(profile) = optional_command_string(command, "profile") {
        options.profile = Some(profile);
    }
    if let Some(runtime_profile) = optional_command_string(command, "runtimeProfile") {
        options.runtime_profile = Some(runtime_profile);
    } else if let Some(profile_id) = optional_command_string(command, "profileId") {
        options.runtime_profile = Some(profile_id);
    }
    if let Some(browser_family) = optional_command_string(command, "runtimeProfileBrowserFamily") {
        options.expected_browser_family = Some(browser_family);
    }
}
pub(crate) fn apply_auto_launch_command_hints(
    options: &mut LaunchOptions,
    command: &Value,
    retained_remote_headed: Option<&RetainedRemoteHeadedLaunchHint>,
) -> (
    ServiceBrowserHost,
    Option<ProfileSelectionReason>,
    BrowserCapabilityLaunchResolution,
    Value,
) {
    let effective_command = launch_command_with_effective_service_defaults(command, options);
    apply_explicit_launch_identity_from_command(options, &effective_command);
    apply_retained_remote_headed_launch_hints(options, retained_remote_headed);
    let service_host = apply_launch_host_hints(options, &effective_command);
    let selection_reason = apply_service_profile_selection(options, &effective_command);
    let browser_capability_launch =
        apply_service_browser_capability_selection(options, &effective_command);
    (
        service_host,
        selection_reason,
        browser_capability_launch,
        effective_command,
    )
}
pub(crate) fn service_profile_lease_conflict_session_ids(
    metadata: &ServiceLaunchMetadata,
    session_id: &str,
    profile_id: &str,
) -> Vec<String> {
    let repository = match LockedServiceStateRepository::default_json() {
        Ok(repository) => repository,
        Err(_) => return Vec::new(),
    };
    let service_state = match repository.load_snapshot() {
        Ok(service_state) => service_state,
        Err(_) => return Vec::new(),
    };
    service_profile_lease_conflict_session_ids_in_state(
        &service_state,
        metadata,
        session_id,
        profile_id,
    )
}
pub(crate) fn service_profile_live_reusable_browser_ids(
    session_id: &str,
    profile_id: &str,
) -> Vec<String> {
    let repository = match LockedServiceStateRepository::default_json() {
        Ok(repository) => repository,
        Err(_) => return Vec::new(),
    };
    let service_state = match repository.load_snapshot() {
        Ok(service_state) => service_state,
        Err(_) => return Vec::new(),
    };
    let mut browser_ids = service_state
        .browsers
        .iter()
        .filter(|(browser_id, browser)| {
            browser.profile_id.as_deref() == Some(profile_id)
                && service_browser_health_counts_as_live(browser.health)
                && browser_id.as_str() != service_browser_id(session_id)
                && !browser
                    .active_session_ids
                    .iter()
                    .any(|active_session_id| active_session_id == session_id)
        })
        .map(|(browser_id, _)| browser_id.clone())
        .collect::<Vec<_>>();
    browser_ids.sort();
    browser_ids.dedup();
    browser_ids
}
pub(crate) fn service_browser_health_counts_as_live(health: ServiceBrowserHealth) -> bool {
    !matches!(
        health,
        ServiceBrowserHealth::NotStarted
            | ServiceBrowserHealth::ProcessExited
            | ServiceBrowserHealth::Closing
            | ServiceBrowserHealth::Faulted
    )
}
pub(crate) fn service_profile_lease_conflict_session_ids_in_state(
    service_state: &ServiceState,
    _metadata: &ServiceLaunchMetadata,
    session_id: &str,
    profile_id: &str,
) -> Vec<String> {
    let lease_telemetry = profile_lease_telemetry(service_state, session_id, profile_id);
    if lease_telemetry.disposition != ProfileLeaseDisposition::ActiveLeaseConflict {
        return Vec::new();
    }
    lease_telemetry.conflict_session_ids
}
pub(crate) fn service_profile_lease_conflict_error(
    metadata: &ServiceLaunchMetadata,
    profile_id: &str,
    conflict_session_ids: &[String],
    wait_timeout_ms: Option<u64>,
) -> String {
    let service_label = metadata
        .service_name
        .as_deref()
        .unwrap_or("unknown service");
    let wait_detail = wait_timeout_ms
        .map(|timeout| format!(" after waiting {} ms", timeout))
        .unwrap_or_default();
    format!(
        "Service profile lease conflict for profile '{}': service '{}' cannot launch{} while exclusive session(s) {} already hold the profile. Retry after those sessions release the profile, set profileLeasePolicy to wait, or request a different profile.",
        profile_id, service_label, wait_detail, conflict_session_ids.join(", ")
    )
}
pub(crate) fn service_duplicate_profile_lane_error(
    metadata: &ServiceLaunchMetadata,
    profile_id: &str,
    browser_ids: &[String],
) -> String {
    let service_label = metadata
        .service_name
        .as_deref()
        .unwrap_or("unknown service");
    format!(
        "Duplicate service profile lane blocked for profile '{}': service '{}' selected a profile already backed by live browser(s) {}. Reuse the access-plan browserId/sessionName route hints, wait for the profile lane, request a different profile, or set allowDuplicateProfileLane=true for reviewed isolation or throwaway browser behavior.",
        profile_id, service_label, browser_ids.join(", ")
    )
}
pub(crate) fn allow_duplicate_profile_lane_from_command(command: &Value) -> bool {
    command
        .get("allowDuplicateProfileLane")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
pub(crate) fn active_browser_profile_mismatch(
    command: &Value,
    state: &DaemonState,
) -> Option<String> {
    let browser = state.browser.as_ref()?;
    active_browser_profile_mismatch_message(
        optional_command_string(command, "runtimeProfile").as_deref(),
        optional_command_string(command, "profile").as_deref(),
        browser.runtime_profile_name(),
        browser.browser_user_data_dir(),
        &state.session_id,
    )
}
pub(crate) fn active_browser_profile_mismatch_message(
    requested_runtime_profile: Option<&str>,
    requested_profile: Option<&str>,
    active_runtime_profile: Option<&str>,
    active_user_data_dir: Option<&Path>,
    session_id: &str,
) -> Option<String> {
    let requested_runtime_profile = requested_runtime_profile
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let requested_profile = requested_profile
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if requested_runtime_profile.is_none() && requested_profile.is_none() {
        return None;
    }
    if let (Some(requested), Some(active)) = (requested_runtime_profile, active_runtime_profile) {
        if requested == active {
            return None;
        }
    }
    if let (Some(requested), Some(active)) = (requested_profile, active_user_data_dir) {
        if pathish_eq(requested, active) {
            return None;
        }
    }
    Some(
        format!(
            "Service request selected profile mismatch: request runtimeProfile={} profile={} but active session '{}' is using runtimeProfile={} profile={}. Refusing to run against the wrong authenticated profile; request the access-plan route hints, close or route away from the current browser, or use a matching retained browser.",
            requested_runtime_profile.unwrap_or("none"), requested_profile
            .unwrap_or("none"), session_id, active_runtime_profile.unwrap_or("none"),
            active_user_data_dir.map(| path | path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        ),
    )
}
pub(crate) fn pathish_eq(left: &str, right: &Path) -> bool {
    let left = Path::new(left);
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
pub(crate) fn profile_lease_policy_from_command(
    command: &Value,
) -> Result<ProfileLeasePolicy, String> {
    match optional_command_string(command, "profileLeasePolicy").as_deref() {
        None | Some("reject") => Ok(ProfileLeasePolicy::Reject),
        Some("wait") => Ok(ProfileLeasePolicy::Wait),
        Some(value) => Err(format!(
            "profileLeasePolicy must be 'reject' or 'wait', got '{}'",
            value
        )),
    }
}
pub(crate) fn profile_lease_wait_timeout_ms_from_command(command: &Value) -> Result<u64, String> {
    match command.get("profileLeaseWaitTimeoutMs") {
        None => Ok(DEFAULT_PROFILE_LEASE_WAIT_TIMEOUT_MS),
        Some(value) => value
            .as_u64()
            .filter(|timeout| *timeout > 0)
            .ok_or_else(|| "profileLeaseWaitTimeoutMs must be a positive integer".to_string()),
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserStaleState {
    pub(crate) needs_launch: bool,
    pub(crate) health: Option<ServiceBrowserHealth>,
    pub(crate) recovery_reason_kind: Option<BrowserRecoveryReasonKind>,
    pub(crate) message: Option<String>,
    pub(crate) event_details: Option<Value>,
}
pub(crate) async fn detect_browser_stale_state(state: &mut DaemonState) -> BrowserStaleState {
    let Some(ref mut mgr) = state.browser else {
        return BrowserStaleState {
            needs_launch: true,
            health: None,
            recovery_reason_kind: None,
            message: None,
            event_details: None,
        };
    };
    if let Some(exit) = mgr.poll_process_exit() {
        let message = format!(
            "Active browser PID {} exited before command dispatch",
            exit.pid
        );
        return BrowserStaleState {
            needs_launch: true,
            health: Some(ServiceBrowserHealth::ProcessExited),
            recovery_reason_kind: Some(BrowserRecoveryReasonKind::ProcessExited),
            message: Some(message),
            event_details: Some(process_exit_observation_details(&exit)),
        };
    }
    if !mgr.is_connection_alive().await {
        let mut details = json!(
            { "cdpProbe" : "Browser.getVersion", "cdpEndpoint" : mgr.get_cdp_url(), }
        );
        if let Some(path) = mgr.browser_stderr_log_path() {
            details["browserStderrLogPath"] = json!(path.to_string_lossy().to_string());
        }
        return BrowserStaleState {
            needs_launch: true,
            health: Some(ServiceBrowserHealth::CdpDisconnected),
            recovery_reason_kind: Some(BrowserRecoveryReasonKind::CdpDisconnected),
            message: Some(format!(
                "Active browser CDP connection is not responding: {}",
                mgr.get_cdp_url()
            )),
            event_details: Some(details),
        };
    }
    BrowserStaleState {
        needs_launch: false,
        health: None,
        recovery_reason_kind: None,
        message: None,
        event_details: None,
    }
}
pub(crate) fn process_exit_observation_details(exit: &ProcessExitObservation) -> Value {
    let mut details = json!(
        { "processExitDetection" : "local_child_try_wait", "processExitPid" : exit.pid, }
    );
    if let Some(code) = exit.exit_code {
        details["processExitCode"] = json!(code);
    }
    #[cfg(unix)]
    if let Some(signal) = exit.signal {
        details["processExitSignal"] = json!(signal);
    }
    if let Some(error) = exit.poll_error.as_deref() {
        details["processExitPollError"] = json!(error);
    }
    if let Some(path) = exit.stderr_log_path.as_deref() {
        details["browserStderrLogPath"] = json!(path.to_string_lossy().to_string());
    }
    details
}
pub(crate) fn persist_current_browser_stale_health(
    state: &DaemonState,
    health: ServiceBrowserHealth,
    reason_kind: BrowserRecoveryReasonKind,
    last_error: String,
    event_details: Option<Value>,
) -> BrowserRecoveryPersistence {
    let Some(mgr) = state.browser.as_ref() else {
        return BrowserRecoveryPersistence::NotRecorded;
    };
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        return persist_current_browser_stale_health_in_repository(
            &repository,
            &state.session_id,
            mgr.browser_pid(),
            Some(mgr.get_cdp_url().to_string()),
            state.browser_recovery_policy_config,
            health,
            reason_kind,
            last_error,
            event_details,
        );
    }
    BrowserRecoveryPersistence::NotRecorded
}
