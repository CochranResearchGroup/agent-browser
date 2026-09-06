#![allow(unused_imports)]
use super::capability::{
    browser_build_label, browser_capability_service_state, executable_path_is_operator_supplied,
    select_browser_capability_launch_binding,
};
use super::cdp_free_plan::{
    browser_host_from_command, launch_options_service_profile_id,
    optional_command_or_params_string, optional_command_string, remote_headed_display_isolation,
    remote_headed_display_isolation_from_command,
};
use super::launch::auto_launch;
use super::recovery::DaemonState;
use super::remote_headed::{parse_control_input_provider, parse_view_stream_provider};
use crate::native::auth;
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
use crate::native::network::resolve_fetch_paused;
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::service_access::{service_access_plan_for_state, ServiceAccessPlanRequest};
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
    RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStream,
    ViewStreamProvider, ViewerLease,
};
use crate::native::service_profile_access_policy::{
    effective_profile_permissions, ProfileAccessMode, ProfileIdentityAssurance, ProfilePermission,
    ServiceProfileAccessPolicy,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use agent_browser_cdp::types::{
    AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
    DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
    TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
/// Wait strategy used by `auth_login` when navigating to the login page.
///
/// We intentionally use `Load` (instead of `NetworkIdle`) because many modern
/// apps keep background requests active indefinitely (polling, analytics,
/// websockets), which can prevent network-idle from ever resolving.
///
/// After navigation completes, `auth_login` explicitly waits for form selectors
/// to appear before filling/clicking.
pub(crate) const AUTH_LOGIN_WAIT_UNTIL: WaitUntil = WaitUntil::Load;
pub(crate) const DEFAULT_PROFILE_LEASE_WAIT_TIMEOUT_MS: u64 = 30_000;
pub(crate) const PROFILE_LEASE_WAIT_POLL_MS: u64 = 250;
pub(crate) const REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS: &str = "2s";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileLeasePolicy {
    Reject,
    Wait,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServiceProfileLeaseGate {
    Ready,
    Reject {
        reason: super::profile_lease::ServiceProfileLeaseBlockReason,
        error: String,
    },
    Wait {
        retry_after_ms: u64,
        profile_id: String,
        conflict_session_ids: Vec<String>,
    },
}
/// Poll interval used while waiting for auth form selectors to appear.
pub(crate) const AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS: u64 = 100;
/// Time spent trying targeted username selectors before broad text-input
/// fallback selectors are allowed.
pub(crate) const AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS: u64 = 5_000;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CloseBehavior {
    #[default]
    CloseBrowser,
    Detach,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeHandoffDescriptor {
    pub(crate) schema_version: u8,
    pub(crate) session_name: String,
    pub(crate) cdp_url: String,
    pub(crate) browser_pid: Option<u32>,
    pub(crate) runtime_profile: Option<String>,
    #[serde(default)]
    pub(crate) process_identity: Option<crate::process_identity::RecordedProcessIdentity>,
    pub(crate) engine: String,
    pub(crate) host: ServiceBrowserHost,
    pub(crate) close_browser_on_close: bool,
    #[serde(default)]
    pub(crate) active_target_id: Option<String>,
    /// Present for the generation-fenced two-phase protocol. Schema version 1
    /// descriptors remain readable only as verified orphan-adoption evidence.
    #[serde(default)]
    pub(crate) owner_transfer: Option<crate::runtime_owner_transfer::OwnerTransferProposal>,
    pub(crate) prepared_at: String,
}
pub(crate) fn debug_session_events_enabled() -> bool {
    env::var("AGENT_BROWSER_DEBUG_SESSIONS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
pub(crate) fn is_stale_page_session_error(err: &str) -> bool {
    err.contains("CDP response channel closed")
        || err.contains("Trying to work with closed connection")
        || err.contains("Session with given id not found")
        || err.contains("No session with given id")
}
pub(crate) async fn recover_browser_command_channel(
    mgr: &mut BrowserManager,
    err: &str,
) -> Result<(), String> {
    if err.contains("Trying to work with closed connection")
        || err.contains("CDP response channel closed")
    {
        mgr.reconnect_client().await
    } else {
        mgr.refresh_active_page_session().await.map(|_| ())
    }
}
pub(crate) async fn relaunch_and_restore_page(
    state: &mut DaemonState,
    desired_url: Option<String>,
) -> Result<(), String> {
    if state.browser.is_some() {
        state.close_behavior = CloseBehavior::CloseBrowser;
        super::navigation::handle_close(state).await?;
    }
    auto_launch(state, &json!({})).await?;
    if let Some(url) = desired_url.as_deref() {
        if !url.is_empty() && url != "about:blank" {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            let _ = mgr.navigate(url, WaitUntil::Load).await?;
        }
    }
    Ok(())
}
pub(crate) struct PendingConfirmation {
    pub action: String,
    pub cmd: Value,
}
/// Captured request/response metadata used to export HAR 1.2 files.
pub(crate) struct HarEntry {
    pub request_id: String,
    /// Seconds since Unix epoch (CDP `wallTime`), with sub-second precision.
    pub wall_time: f64,
    pub method: String,
    pub url: String,
    pub request_headers: Vec<(String, String)>,
    pub post_data: Option<String>,
    pub request_body_size: i64,
    pub resource_type: String,
    pub status: Option<i64>,
    pub status_text: String,
    /// Normalised from CDP `response.protocol` (e.g. `"h2"` → `"HTTP/2.0"`).
    pub http_version: String,
    pub response_headers: Vec<(String, String)>,
    pub mime_type: String,
    pub redirect_url: String,
    /// Updated by `Network.loadingFinished` for final accuracy.
    pub response_body_size: i64,
    /// Raw CDP `ResourceTiming` object from `Network.responseReceived`.
    pub cdp_timing: Option<Value>,
    /// Monotonic timestamp (seconds) from `Network.loadingFinished`; used to
    /// compute the `receive` timing phase.
    pub loading_finished_timestamp: Option<f64>,
}
pub(crate) struct RouteEntry {
    pub url_pattern: String,
    pub response: Option<RouteResponse>,
    pub abort: bool,
}
pub(crate) struct RouteResponse {
    pub status: Option<u16>,
    pub body: Option<String>,
    pub content_type: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}
#[derive(Clone, serde::Serialize)]
pub(crate) struct TrackedRequest {
    pub url: String,
    pub method: String,
    pub headers: Value,
    pub timestamp: u64,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "postData", skip_serializing_if = "Option::is_none")]
    pub post_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i64>,
    #[serde(rename = "responseHeaders", skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<Value>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
pub(crate) struct FetchPausedRequest {
    pub request_id: String,
    pub url: String,
    pub resource_type: String,
    pub session_id: String,
    /// Original request headers from the Fetch.requestPaused event, needed
    /// because Fetch.continueRequest replaces (not merges) headers.
    pub request_headers: Option<serde_json::Map<String, Value>>,
}
pub(crate) enum BackendType {
    Cdp,
    WebDriver,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct PendingDialog {
    pub dialog_type: String,
    pub message: String,
    pub url: String,
    pub default_prompt: Option<String>,
}
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MouseState {
    pub x: f64,
    pub y: f64,
    pub buttons: i32,
}
#[derive(Default)]
pub(crate) struct DrainedEvents {
    pub(crate) pending_acks: Vec<i64>,
    pub(crate) new_targets: Vec<TargetCreatedEvent>,
    pub(crate) changed_targets: Vec<TargetInfoChangedEvent>,
    pub(crate) destroyed_targets: Vec<String>,
    /// Page/webview targets can be re-attached with a new CDP session during
    /// navigation or process swaps. Track the fresh session by target_id.
    pub(crate) attached_page_sessions: Vec<(String, String)>,
    /// Cross-origin iframe (frame_id, session_id) pairs from Target.attachedToTarget.
    pub(crate) attached_iframe_sessions: Vec<(String, String)>,
    /// Page/webview session IDs from Target.detachedFromTarget.
    pub(crate) detached_page_sessions: Vec<String>,
    /// Session IDs from Target.detachedFromTarget.
    pub(crate) detached_iframe_sessions: Vec<String>,
    /// Renderer crashes are kept distinct from ordinary target destruction and
    /// session replacement so lifecycle projection cannot infer crashes from
    /// detach heuristics.
    pub(crate) renderer_crashes: Vec<crate::native::service_renderer_crash::RendererCrashSignal>,
}
/// Compute a hash of the [`LaunchOptions`] fields that require a browser
/// relaunch when changed (baked into the Chrome process at startup).
///
/// Fields NOT hashed (adjustable at runtime via CDP without relaunch):
/// ignore_https_errors, color_scheme, download_path, storage_state
pub(crate) fn launch_hash(opts: &LaunchOptions) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    opts.headless.hash(&mut h);
    opts.extensions.hash(&mut h);
    opts.profile.hash(&mut h);
    opts.executable_path.hash(&mut h);
    opts.args.hash(&mut h);
    opts.proxy.hash(&mut h);
    opts.proxy_bypass.hash(&mut h);
    opts.proxy_username.hash(&mut h);
    opts.proxy_password.hash(&mut h);
    opts.user_agent.hash(&mut h);
    opts.allow_file_access.hash(&mut h);
    opts.runtime_profile.hash(&mut h);
    opts.expected_browser_family.hash(&mut h);
    opts.use_real_keychain.hash(&mut h);
    opts.keychain_password.hash(&mut h);
    opts.manual_login.hash(&mut h);
    opts.display.hash(&mut h);
    opts.remote_headed.hash(&mut h);
    opts.remote_headed_display_isolation.hash(&mut h);
    h.finish()
}
pub(crate) fn parse_env_bool(name: &str) -> bool {
    env::var(name)
        .is_ok_and(|v| !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no" | ""))
}
pub(crate) fn keychain_password_from_env() -> Option<String> {
    env::var("AGENT_BROWSER_KEYCHAIN_PASSWORD").ok()
}
pub(crate) fn use_real_keychain_from_env() -> bool {
    parse_env_bool("AGENT_BROWSER_USE_REAL_KEYCHAIN") || keychain_password_from_env().is_some()
}
pub(crate) fn runtime_profile_from_env() -> Option<String> {
    env::var("AGENT_BROWSER_RUNTIME_PROFILE").ok()
}
pub(crate) fn runtime_profile_from_sources(
    cmd: &Value,
    include_env_profile: bool,
) -> Option<String> {
    cmd.get("runtimeProfile")
        .and_then(|v| v.as_str())
        .or_else(|| cmd.get("profileId").and_then(|v| v.as_str()))
        .map(str::to_string)
        .or_else(|| include_env_profile.then(runtime_profile_from_env).flatten())
}
pub(crate) fn launch_profile_from_sources(
    cmd: &Value,
    include_env_profile: bool,
) -> Option<String> {
    let command_profile = cmd
        .get("profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if command_profile.is_some() {
        return command_profile;
    }
    include_env_profile
        .then(|| env::var("AGENT_BROWSER_PROFILE").ok())
        .flatten()
}
pub(crate) fn launch_args_from_sources(cmd: &Value) -> Vec<String> {
    if let Some(args) = cmd.get("args").and_then(|v| v.as_array()) {
        return args
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    env::var("AGENT_BROWSER_ARGS")
        .map(|v| {
            v.split([',', '\n'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
pub(crate) fn target_service_ids_from_command(cmd: &Value) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "targetServiceId",
        "targetService",
        "siteId",
        "loginId",
        "target_service_id",
        "site_id",
        "login_id",
    ] {
        if let Some(value) = cmd.get(key).and_then(|value| value.as_str()) {
            merge_target_service_id(&mut values, value);
        }
    }
    for key in [
        "targetServiceIds",
        "targetServices",
        "siteIds",
        "loginIds",
        "target_service_ids",
        "site_ids",
        "login_ids",
    ] {
        if let Some(raw_values) = cmd.get(key).and_then(|value| value.as_array()) {
            for value in raw_values.iter().filter_map(|value| value.as_str()) {
                merge_target_service_id(&mut values, value);
            }
        }
    }
    values
}
pub(crate) fn merge_target_service_id(values: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() && !values.iter().any(|existing| existing == trimmed) {
        values.push(trimmed.to_string());
    }
}
pub(crate) fn account_ids_from_command(cmd: &Value) -> Vec<String> {
    let mut values = Vec::new();
    for key in ["accountId", "account", "account_id", "account-id"] {
        if let Some(value) = cmd.get(key).and_then(|value| value.as_str()) {
            merge_target_service_id(&mut values, value);
        }
    }
    for key in ["accountIds", "accounts", "account_ids", "account-ids"] {
        if let Some(raw_values) = cmd.get(key).and_then(|value| value.as_array()) {
            for value in raw_values.iter().filter_map(|value| value.as_str()) {
                merge_target_service_id(&mut values, value);
            }
        }
    }
    values
}
pub(crate) fn target_url_from_command(cmd: &Value) -> Option<String> {
    optional_command_string(cmd, "url").or_else(|| {
        cmd.get("params")
            .and_then(|params| optional_command_string(params, "url"))
    })
}
pub(crate) fn browser_build_from_command(cmd: &Value) -> Option<BrowserBuild> {
    for key in ["browserBuild", "browser_build", "browser-build"] {
        if let Some(value) = cmd.get(key).and_then(|value| value.as_str()) {
            if let Some(browser_build) = BrowserBuild::parse_label(value) {
                return Some(browser_build);
            }
        }
    }
    cmd.get("params").and_then(browser_build_from_command)
}
pub(crate) fn launch_command_with_effective_service_defaults(
    command: &Value,
    options: &LaunchOptions,
) -> Value {
    let Ok(service_state) = browser_capability_service_state(command) else {
        return command.clone();
    };
    let request = ServiceAccessPlanRequest {
        service_name: optional_command_string(command, "serviceName"),
        agent_name: optional_command_string(command, "agentName"),
        task_name: optional_command_string(command, "taskName"),
        client_subject_id: optional_command_string(command, "clientSubjectId"),
        identity_assurance: optional_command_string(command, "identityAssurance"),
        session_name: optional_command_string(command, "sessionName"),
        target_service_ids: target_service_ids_from_command(command),
        account_ids: account_ids_from_command(command),
        target_url: target_url_from_command(command),
        site_policy_id: optional_command_string(command, "sitePolicyId"),
        challenge_id: optional_command_string(command, "challengeId"),
        readiness_profile_id: optional_command_string(command, "readinessProfileId"),
        runtime_profile: runtime_profile_from_sources(command, false),
        browser_build: browser_build_from_command(command),
        browser_build_explicit: command
            .get("browserBuild")
            .and_then(Value::as_str)
            .is_some(),
        browser_host: browser_host_from_command(command),
        view_stream_provider: optional_command_string(command, "viewStreamProvider")
            .or_else(|| optional_command_string(command, "viewStream"))
            .or_else(|| {
                command.get("params").and_then(|params| {
                    optional_command_string(params, "viewStreamProvider")
                        .or_else(|| optional_command_string(params, "viewStream"))
                })
            })
            .and_then(|value| parse_view_stream_provider(&value)),
        control_input_provider: optional_command_string(command, "controlInputProvider")
            .or_else(|| optional_command_string(command, "controlInput"))
            .or_else(|| {
                command.get("params").and_then(|params| {
                    optional_command_string(params, "controlInputProvider")
                        .or_else(|| optional_command_string(params, "controlInput"))
                })
            })
            .and_then(|value| parse_control_input_provider(&value)),
        display_isolation: remote_headed_display_isolation_from_command(command),
    };
    let plan = service_access_plan_for_state(&service_state, request);
    let Some(planned_request) = plan.pointer("/decision/serviceRequest/request") else {
        return command.clone();
    };
    apply_planned_launch_defaults(command, &plan, planned_request, options)
}
pub(crate) fn apply_planned_launch_defaults(
    command: &Value,
    plan: &Value,
    planned_request: &Value,
    options: &LaunchOptions,
) -> Value {
    let mut object = command.as_object().cloned().unwrap_or_default();
    insert_planned_string_if_missing(&mut object, command, planned_request, "browserBuild");
    if options.runtime_profile.is_none()
        && options.profile.is_none()
        && command.get("profile").is_none()
        && command.get("runtimeProfile").is_none()
        && command.get("profileId").is_none()
    {
        insert_planned_string_if_missing(&mut object, command, planned_request, "runtimeProfile");
    }
    if options.profile.is_none()
        && options.runtime_profile.is_none()
        && command.get("profile").is_none()
        && command.get("profileId").is_none()
        && command.get("runtimeProfile").is_none()
    {
        insert_planned_string_if_missing(&mut object, command, planned_request, "profile");
    }
    if command.get("profileLeasePolicy").is_none() {
        insert_planned_string_if_missing(
            &mut object,
            command,
            planned_request,
            "profileLeasePolicy",
        );
    }
    if command.get("cdpAttachmentAllowed").is_none() {
        if let Some(value) = planned_request.get("cdpAttachmentAllowed") {
            object.insert("cdpAttachmentAllowed".to_string(), value.clone());
        }
    }
    let planned_params = planned_request.get("params").and_then(Value::as_object);
    if let Some(planned_params) = planned_params {
        let mut params = command
            .get("params")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let posture_source = plan
            .pointer("/decision/launchPosture/source")
            .and_then(Value::as_str);
        if posture_source != Some("service_default") {
            insert_planned_param_if_missing(
                &mut object,
                &mut params,
                command,
                planned_params,
                "browserHost",
            );
        }
        insert_planned_param_if_missing(
            &mut object,
            &mut params,
            command,
            planned_params,
            "viewStreamProvider",
        );
        insert_planned_param_if_missing(
            &mut object,
            &mut params,
            command,
            planned_params,
            "controlInputProvider",
        );
        insert_planned_param_if_missing(
            &mut object,
            &mut params,
            command,
            planned_params,
            "displayIsolation",
        );
        if command.get("headlessExplicit").and_then(Value::as_bool) != Some(true)
            && command
                .get("params")
                .and_then(|params| params.get("headlessExplicit"))
                .and_then(Value::as_bool)
                != Some(true)
            && command.get("headless").is_none()
            && !params.contains_key("headless")
        {
            if let Some(value) = planned_params.get("headless") {
                object.insert("headless".to_string(), value.clone());
            }
        }
        if !params.is_empty() {
            object.insert("params".to_string(), Value::Object(params));
        }
    }
    Value::Object(object)
}
pub(crate) fn insert_planned_string_if_missing(
    object: &mut Map<String, Value>,
    command: &Value,
    planned_request: &Value,
    key: &str,
) {
    if command.get(key).is_some() {
        return;
    }
    if let Some(value) = planned_request.get(key).and_then(Value::as_str) {
        object.insert(key.to_string(), json!(value));
    }
}
pub(crate) fn insert_planned_param_if_missing(
    object: &mut Map<String, Value>,
    params: &mut Map<String, Value>,
    command: &Value,
    planned_params: &Map<String, Value>,
    key: &str,
) {
    if command.get(key).is_some()
        || command
            .get("params")
            .and_then(|params| params.get(key))
            .is_some()
    {
        return;
    }
    if let Some(value) = planned_params.get(key) {
        object.insert(key.to_string(), value.clone());
        params.insert(key.to_string(), value.clone());
    }
}
fn resolved_service_profile_identity_path(
    profile_hint: Option<&str>,
    profile_id: &str,
) -> Result<std::path::PathBuf, String> {
    match profile_hint {
        Some(profile_hint) => Ok(crate::runtime_profile::resolve_profile(
            Some(profile_hint),
            Some(profile_id),
        )?
        .user_data_dir),
        None => crate::runtime_profile::runtime_profile_user_data_dir(profile_id),
    }
}

pub(crate) fn apply_service_profile_selection(
    options: &mut LaunchOptions,
    cmd: &Value,
    effective_session: Option<&str>,
) -> Result<Option<ProfileSelectionReason>, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let service_state = repository.load_snapshot()?;
    if let Some(selection) =
        apply_existing_session_profile_selection(options, cmd, effective_session, &service_state)?
    {
        return Ok(Some(selection));
    }
    if options.profile.is_some() {
        return Ok(None);
    }
    let service_owned_launch = cmd.get("action").and_then(Value::as_str) == Some("launch")
        && optional_command_string(cmd, "serviceName").is_some();
    let explicit_profile_id = service_owned_launch.then(|| {
        optional_command_or_params_string(cmd, "runtimeProfile")
            .or_else(|| optional_command_or_params_string(cmd, "profileId"))
    });
    if let Some(profile_id) = explicit_profile_id.flatten() {
        let Some(profile) = service_state.profiles.get(&profile_id) else {
            return Ok(None);
        };
        options.runtime_profile = Some(profile_id);
        if let Some(user_data_dir) = profile
            .user_data_dir
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            options.profile = Some(user_data_dir.to_string());
        }
        if profile.browser_build == Some(BrowserBuild::StockChrome)
            && cmd.get("executablePath").is_none()
        {
            options.executable_path = None;
        }
        return Ok(Some(ProfileSelectionReason::ExplicitProfile));
    }
    if options.runtime_profile.is_some() {
        return Ok(None);
    }
    let request = ProfileSelectionRequest {
        service_name: optional_command_string(cmd, "serviceName"),
        target_service_ids: target_service_ids_from_command(cmd),
        account_ids: account_ids_from_command(cmd),
        target_url: target_url_from_command(cmd),
        browser_build: browser_build_from_command(cmd),
    };
    if request.service_name.is_none()
        && request.target_service_ids.is_empty()
        && request.account_ids.is_empty()
        && request.target_url.is_none()
        && request.browser_build.is_none()
    {
        return Ok(None);
    }
    let Some(selection) = select_service_profile_for_request(&service_state, &request) else {
        return Ok(None);
    };
    let Some(profile) = service_state.profiles.get(&selection.profile_id) else {
        return Ok(None);
    };
    options.runtime_profile = Some(selection.profile_id.clone());
    if let Some(user_data_dir) = profile
        .user_data_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        options.profile = Some(user_data_dir.to_string());
    }
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && cmd.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(Some(selection.reason))
}

/// Service custom-profile IDs identify a directory record; they are not named
/// runtime profiles. Preserve that distinction when resolving an existing owner
/// so repeated commands keep the original launch identity and profile path.
fn retained_profile_launch_identity(
    profile_id: &str,
    profile: &BrowserProfile,
) -> (Option<String>, Option<String>) {
    (
        (!profile_id.starts_with("custom:")).then(|| profile_id.to_string()),
        profile.user_data_dir.clone(),
    )
}

pub(crate) fn apply_existing_session_profile_selection(
    options: &mut LaunchOptions,
    command: &Value,
    effective_session: Option<&str>,
    state: &ServiceState,
) -> Result<Option<ProfileSelectionReason>, String> {
    let requested_session =
        optional_command_or_params_string(command, "sessionName").or_else(|| {
            optional_command_or_params_string(command, "browserId")
                .and_then(|browser_id| browser_id.strip_prefix("session:").map(str::to_string))
        });
    // A launch command reaches an already resolved daemon lane, so its request
    // route hint must not replace that effective session. Other service actions
    // can explicitly target a browser session while sharing a runtime host.
    let session_id = if command.get("action").and_then(Value::as_str) == Some("launch") {
        effective_session.map(str::to_string).or(requested_session)
    } else {
        requested_session.or_else(|| effective_session.map(str::to_string))
    };
    let Some(session_id) = session_id else {
        return Ok(None);
    };
    let binding = state
        .runtime_owner_registry
        .binding_for_session(&session_id)
        .map_err(|_| "existing_session_profile_identity_ambiguous".to_string())?;
    let retained_observation = state
        .sessions
        .get(&session_id)
        .is_some_and(|session| session.profile_id.is_some() || !session.browser_ids.is_empty())
        || state.browsers.values().any(|browser| {
            browser
                .active_session_ids
                .iter()
                .any(|id| id == &session_id)
        });
    let Some(binding) = binding else {
        if apply_registered_session_profile_continuity(options, command, &session_id, state)? {
            return Ok(Some(ProfileSelectionReason::ExistingOwner));
        }
        if apply_shared_local_session_profile_continuity(options, command, &session_id, state)? {
            return Ok(Some(ProfileSelectionReason::ExistingOwner));
        }
        if apply_authenticated_access_plan_profile_selection(options, command, &session_id, state)?
        {
            return Ok(Some(ProfileSelectionReason::ExplicitProfile));
        }
        if std::env::var_os("AGENT_BROWSER_DEBUG").is_some() {
            eprintln!(
                "[profile-selection] session={} binding=missing retained_observation={} persisted_session={} active_browser_session={}",
                session_id,
                retained_observation,
                state.sessions.contains_key(&session_id),
                state.browsers.values().any(|browser| browser
                    .active_session_ids
                    .iter()
                    .any(|active_session| active_session == &session_id)),
            );
        }
        return if retained_observation {
            Err("existing_session_profile_identity_unproven".to_string())
        } else {
            Ok(None)
        };
    };
    if exact_terminal_owner_allows_explicit_profile_relaunch(
        options,
        command,
        &session_id,
        state,
        &binding,
    )? {
        return Ok(None);
    }
    if apply_authenticated_orphaned_owner_recourse(options, command, &session_id, state, &binding)?
    {
        return Ok(Some(ProfileSelectionReason::ExistingOwner));
    }
    if !binding.effect_capable {
        if apply_registered_session_profile_continuity(options, command, &session_id, state)? {
            return Ok(Some(ProfileSelectionReason::ExistingOwner));
        }
        if apply_shared_local_session_profile_continuity(options, command, &session_id, state)? {
            return Ok(Some(ProfileSelectionReason::ExistingOwner));
        }
        return Err("existing_session_profile_identity_unproven".to_string());
    }
    let session = state
        .sessions
        .get(&session_id)
        .ok_or_else(|| "existing_session_profile_identity_unproven".to_string())?;
    let profile_id = session
        .profile_id
        .as_deref()
        .ok_or_else(|| "existing_session_profile_identity_unproven".to_string())?;
    let browser = state
        .browsers
        .get(&binding.claim.logical_browser_id)
        .ok_or_else(|| "existing_session_profile_identity_unproven".to_string())?;
    if !session
        .browser_ids
        .iter()
        .any(|browser_id| browser_id == &binding.claim.logical_browser_id)
        || !browser
            .active_session_ids
            .iter()
            .any(|active_session| active_session == &session_id)
        || browser.profile_id.as_deref() != Some(profile_id)
    {
        return Err("existing_session_profile_identity_inconsistent".to_string());
    }
    let profile = state
        .profiles
        .get(profile_id)
        .ok_or_else(|| "existing_session_profile_identity_unproven".to_string())?;
    let user_data_dir =
        resolved_service_profile_identity_path(profile.user_data_dir.as_deref(), profile_id)?;
    let profile_digest = crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)?;
    if profile_digest != binding.claim.profile_identity_digest {
        return Err("existing_session_profile_identity_inconsistent".to_string());
    }
    if let Some(principal_binding) = state
        .runtime_owner_registry
        .principal_bindings
        .get(&binding.claim.profile_identity_digest)
    {
        // Superseding a browser does not promote its predecessor's capability.
        // A fresh shared-local subject has independent policy authority, so an
        // older binding cannot veto its exact current-owner profile selection.
        // Registered callers still require their guarded continuity/rejoin path.
        let independent_shared_local_use = principal_binding.owner_generation
            < binding.claim.owner_generation
            && command
                .get("servicePrincipalProvenance")
                .and_then(Value::as_str)
                != Some("registered_capability")
            && shared_local_profile_use_allowed(profile, profile_id, command);
        if principal_binding.profile_id != profile_id
            || (!state
                .runtime_owner_registry
                .principal_binding_is_current(Some(principal_binding))
                && !independent_shared_local_use)
        {
            return Err("existing_session_profile_identity_inconsistent".to_string());
        }
    }
    let exact_command_profile_overrides_inherited_default = command.get("runtimeProfile").is_none()
        && command.get("profileId").is_none()
        && optional_command_or_params_string(command, "profile")
            .map(|requested_profile| {
                resolved_service_profile_identity_path(Some(&requested_profile), profile_id)
                    .and_then(|requested_path| {
                        crate::runtime_profile::canonical_profile_identity_digest(&requested_path)
                    })
                    .is_ok_and(|requested_digest| requested_digest == profile_digest)
            })
            .unwrap_or(false);
    let retained_focus_uses_current_owner_profile = command.get("action").and_then(Value::as_str)
        == Some("view_focus")
        && command.get("runtimeProfile").is_none()
        && command.get("profileId").is_none()
        && optional_command_or_params_string(command, "profile").is_none();
    if options
        .runtime_profile
        .as_deref()
        .is_some_and(|requested| requested != profile_id)
        && !exact_command_profile_overrides_inherited_default
        && !retained_focus_uses_current_owner_profile
    {
        return Err("explicit_profile_conflicts_with_current_owner".to_string());
    }
    if let Some(requested_path) = options.profile.as_deref() {
        let requested_path =
            resolved_service_profile_identity_path(Some(requested_path), profile_id)?;
        let requested_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&requested_path)?;
        if requested_digest != binding.claim.profile_identity_digest {
            return Err("explicit_profile_conflicts_with_current_owner".to_string());
        }
    }
    (options.runtime_profile, options.profile) =
        retained_profile_launch_identity(profile_id, profile);
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && command.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(Some(ProfileSelectionReason::ExistingOwner))
}

/// Admit the exact prelaunch session created for an authenticated principal's
/// first browser lane.
///
/// Service request adapters strip caller-authored authority fields and attach
/// the capability identity only after authenticating the raw bearer secret.
/// The daemon revalidates that internal identity against current Service State,
/// requires the deterministic cold route, and refuses any competing owner,
/// session, or live browser evidence before selecting the profile.
pub(crate) fn apply_authenticated_access_plan_profile_selection(
    options: &mut LaunchOptions,
    command: &Value,
    session_id: &str,
    state: &ServiceState,
) -> Result<bool, String> {
    if !matches!(
        command.get("action").and_then(Value::as_str),
        Some("tab_new" | "remote_view_open" | "launch")
    ) || command
        .get("servicePrincipalProvenance")
        .and_then(Value::as_str)
        != Some("registered_capability")
    {
        return Ok(false);
    }
    let Some(route_authorization) = command
        .get("serviceProfileRouteAuthorization")
        .and_then(Value::as_object)
    else {
        return Ok(false);
    };
    if route_authorization
        .get("schemaVersion")
        .and_then(Value::as_str)
        != Some("agent-browser.profile-launch-route-authorization.v1")
        || route_authorization
            .get("sessionName")
            .and_then(Value::as_str)
            != Some(session_id)
    {
        return Ok(false);
    }
    let Some(route_kind) = route_authorization.get("kind").and_then(Value::as_str) else {
        return Ok(false);
    };
    let Some(principal_id) = command
        .get("servicePrincipalId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(capability_id) = command
        .get("serviceProfileCapabilityId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(capability_revision) = command
        .get("serviceProfileCapabilityRevision")
        .and_then(Value::as_u64)
    else {
        return Ok(false);
    };
    let Some(profile_id) = optional_command_or_params_string(command, "runtimeProfile")
        .or_else(|| optional_command_or_params_string(command, "profileId"))
    else {
        return Ok(false);
    };
    if route_authorization.get("profileId").and_then(Value::as_str) != Some(profile_id.as_str())
        || route_authorization
            .get("principalId")
            .and_then(Value::as_str)
            != Some(principal_id)
        || route_authorization
            .get("capabilityId")
            .and_then(Value::as_str)
            != Some(capability_id)
        || route_authorization
            .get("capabilityRevision")
            .and_then(Value::as_u64)
            != Some(capability_revision)
    {
        return Ok(false);
    }
    let authority = crate::native::service_principal::AuthenticatedServicePrincipal {
        principal_id: principal_id.to_string(),
        profile_id: profile_id.clone(),
        capability_id: capability_id.to_string(),
        capability_revision,
        provenance:
            crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
    };
    if !crate::native::service_principal::authenticated_authority_is_current(
        &state.service_principals,
        &authority,
    ) {
        return Ok(false);
    }
    let Some(profile) = state.profiles.get(&profile_id) else {
        return Ok(false);
    };
    // Access planning deliberately allocates a deterministic route before the
    // first session record exists. If a prelaunch record is already present it
    // must be the exact empty principal/profile placeholder; absence is the
    // normal launch-new state, not an identity failure.
    if state.sessions.get(session_id).is_some_and(|session| {
        session.profile_id.as_deref() != Some(profile_id.as_str())
            || session.principal_id.as_deref() != Some(principal_id)
            || session.principal_provenance != Some(authority.provenance)
            || !session.browser_ids.is_empty()
            || !session.tab_ids.is_empty()
            || !matches!(session.lease, LeaseState::Exclusive)
    }) {
        return Ok(false);
    }
    let user_data_dir =
        resolved_service_profile_identity_path(profile.user_data_dir.as_deref(), &profile_id)?;
    let profile_digest = crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)?;
    let owner = state.runtime_owner_registry.owner(&profile_digest);
    let exact_observed_owner = |owner: &crate::runtime_owner_transfer::ProfileOwner| {
        route_authorization
            .get("runtimeOwnerRegistryRevision")
            .and_then(Value::as_u64)
            == Some(state.runtime_owner_registry.revision)
            && route_authorization.get("ownerId").and_then(Value::as_str)
                == Some(owner.owner_id.as_str())
            && route_authorization
                .get("ownerGeneration")
                .and_then(Value::as_u64)
                == Some(owner.owner_generation)
    };
    let route_authorized = match route_kind {
        "authenticated_cold" => match owner {
            Some(owner) => exact_observed_owner(owner),
            None => {
                route_authorization
                    .get("runtimeOwnerRegistryRevision")
                    .and_then(Value::as_u64)
                    == Some(state.runtime_owner_registry.revision)
                    && route_authorization.get("ownerId").is_some_and(Value::is_null)
                    && route_authorization
                        .get("ownerGeneration")
                        .is_some_and(Value::is_null)
            }
        },
        "terminal_replacement" => owner.is_some_and(|owner| {
            exact_observed_owner(owner)
                && owner.daemon_session_route == session_id
                && state
                    .runtime_owner_registry
                    .lifecycle_records
                    .get(&owner.browser_id)
                    .is_some_and(|lifecycle| {
                        lifecycle.owner_generation == owner.owner_generation
                            && lifecycle.profile_identity_digest == profile_digest
                            && lifecycle.lifecycle_state
                                == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal
                            && lifecycle.cleanup_obligation_state
                                == crate::runtime_owner_transfer::CleanupObligationState::Satisfied
                            && lifecycle
                                .terminal_evidence
                                .iter()
                                .any(|evidence| evidence == "exact_process_exited")
                    })
                && state
                    .browsers
                    .get(&owner.browser_id)
                    .is_none_or(|browser| browser.pid.is_none())
        }),
        _ => false,
    };
    if !route_authorized {
        return Ok(false);
    }
    if options
        .runtime_profile
        .as_deref()
        .is_some_and(|requested| requested != profile_id)
    {
        return Err("explicit_profile_conflicts_with_authenticated_cold_route".to_string());
    }
    if let Some(requested_path) = options.profile.as_deref() {
        let requested_path =
            resolved_service_profile_identity_path(Some(requested_path), &profile_id)?;
        let requested_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&requested_path)?;
        if requested_digest != profile_digest {
            return Err("explicit_profile_conflicts_with_authenticated_cold_route".to_string());
        }
    }
    options.runtime_profile = Some(profile_id);
    options.profile = profile.user_data_dir.clone();
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && command.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(true)
}

fn exact_terminal_owner_allows_explicit_profile_relaunch(
    options: &mut LaunchOptions,
    command: &Value,
    session_id: &str,
    state: &ServiceState,
    binding: &crate::runtime_owner_transfer::RuntimeOwnerBinding,
) -> Result<bool, String> {
    if !matches!(
        command.get("action").and_then(Value::as_str),
        Some("cdp_free_launch" | "remote_view_open" | "launch" | "navigate")
    ) {
        return Ok(false);
    }
    let command_profile_id = optional_command_or_params_string(command, "runtimeProfile")
        .or_else(|| optional_command_or_params_string(command, "profileId"));
    let command_profile = optional_command_or_params_string(command, "profile");
    // Path-backed CLI profiles have no runtime-profile name. Resolve the
    // retained record by canonical directory, then keep the same exact owner
    // and terminal cleanup gates used for named profiles. Ambiguous records
    // must not choose an arbitrary profile policy.
    let requested_path = command_profile
        .as_deref()
        .or(options.profile.as_deref())
        .filter(|profile| crate::runtime_profile::looks_like_path(profile));
    let path_profile_id = if command_profile_id.is_none() {
        if let Some(path) = requested_path {
            let digest = crate::runtime_profile::canonical_profile_identity_digest(
                &crate::runtime_profile::resolve_profile(Some(path), None)?.user_data_dir,
            )?;
            let mut matches = state.profiles.iter().filter_map(|(id, profile)| {
                let path =
                    resolved_service_profile_identity_path(profile.user_data_dir.as_deref(), id)
                        .ok()?;
                (crate::runtime_profile::canonical_profile_identity_digest(&path).ok()? == digest)
                    .then_some(id.as_str())
            });
            let matched = matches.next();
            if matches.next().is_some() {
                return Ok(false);
            }
            matched
        } else {
            None
        }
    } else {
        None
    };
    let Some(profile_id) = command_profile_id
        .as_deref()
        .or_else(|| {
            command_profile
                .as_deref()
                .filter(|profile| !crate::runtime_profile::looks_like_path(profile))
        })
        .or(path_profile_id)
        .or(options.runtime_profile.as_deref())
        .or_else(|| {
            options
                .profile
                .as_deref()
                .filter(|profile| !crate::runtime_profile::looks_like_path(profile))
        })
    else {
        return Ok(false);
    };
    if command_profile_id
        .as_deref()
        .is_some_and(|command_profile_id| command_profile_id != profile_id)
    {
        return Ok(false);
    }
    let Some(profile) = state.profiles.get(profile_id) else {
        return Ok(false);
    };
    let user_data_dir =
        resolved_service_profile_identity_path(profile.user_data_dir.as_deref(), profile_id)?;
    let profile_digest = crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)?;
    if profile_digest != binding.claim.profile_identity_digest
        || binding.claim.daemon_session_route != session_id
    {
        return Ok(false);
    }
    if let Some(requested_path) = options.profile.as_deref().or(command_profile.as_deref()) {
        let requested_path =
            resolved_service_profile_identity_path(Some(requested_path), profile_id)?;
        if crate::runtime_profile::canonical_profile_identity_digest(&requested_path)?
            != profile_digest
        {
            return Ok(false);
        }
    }
    let Some(owner) = state.runtime_owner_registry.owner(&profile_digest) else {
        return Ok(false);
    };
    let Some(lifecycle) = state
        .runtime_owner_registry
        .lifecycle_records
        .get(&binding.claim.logical_browser_id)
    else {
        return Ok(false);
    };
    let process_absence_proven = lifecycle.terminal_evidence.iter().any(|evidence| {
        evidence == "exact_process_exited"
            || evidence.starts_with("service_reconcile_process_group_absent:")
    });
    let profile_lock_release_proven = lifecycle.terminal_evidence.iter().any(|evidence| {
        evidence == "profile_lock_released"
            || evidence == "service_reconcile_profile_lock_absent"
            || evidence.starts_with("service_reconcile_profile_lock_stale_pid_absent:")
    });
    let exact_terminal_owner = owner.owner_generation == binding.claim.owner_generation
        && owner.browser_id == binding.claim.logical_browser_id
        && owner.daemon_session_route == session_id
        && owner.pending_transfer.is_none()
        && lifecycle.logical_browser_id == binding.claim.logical_browser_id
        && lifecycle.profile_identity_digest == profile_digest
        && lifecycle.owner_generation == binding.claim.owner_generation
        && lifecycle.lifecycle_state
            == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal
        && lifecycle.cleanup_obligation_state
            == crate::runtime_owner_transfer::CleanupObligationState::Satisfied
        && process_absence_proven
        && profile_lock_release_proven;
    let logical_browser_id = binding.claim.logical_browser_id.as_str();
    let inert_handle = |handle: &crate::native::service_model::ServiceTabHandle| {
        !handle.valid
            && handle.lease_state == Some(LeaseState::Released)
            && handle.browser_id == logical_browser_id
            && handle.session_name.as_deref() == Some(session_id)
            && handle.owner_session_id.as_deref() == Some(session_id)
            && handle.profile_id.as_deref() == Some(profile_id)
            && handle.stale_reason.is_some()
    };
    let session_projection_inert = match state.sessions.get(session_id) {
        None => true,
        Some(session) => {
            session.lease == LeaseState::Released
                && session.profile_id.as_deref() == Some(profile_id)
                && session.browser_ids.len() <= 1
                && session
                    .browser_ids
                    .iter()
                    .all(|browser_id| browser_id == logical_browser_id)
                && session.tab_ids.is_empty()
        }
    };
    let prepared_remote_display_matches = |display_allocation_id: &str| {
        matches!(
            command.get("action").and_then(Value::as_str),
            Some("remote_view_open" | "launch")
        )
            && state
                .display_allocations
                .get(display_allocation_id)
                .is_some_and(|allocation| {
                    allocation.state == "ready"
                        && allocation.owner_browser_id.as_deref() == Some(logical_browser_id)
                        && allocation.owner_session_id.as_deref() == Some(session_id)
                        && allocation.profile_id.as_deref() == Some(profile_id)
                        && allocation.route_ids.iter().any(|route_id| {
                            state.remote_view_acquisition_leases.values().any(|lease| {
                                crate::native::remote_view::pending_remote_view_acquisition_lease_matches_owner(
                                    lease,
                                    logical_browser_id,
                                    session_id,
                                    route_id,
                                    display_allocation_id,
                                    None,
                                )
                            })
                        })
                })
    };
    let browser_projection_inert = state.browsers.iter().all(|(browser_id, browser)| {
        if browser_id == logical_browser_id {
            browser.profile_id.as_deref() == Some(profile_id)
                && browser.pid.is_none()
                && browser.cdp_endpoint.is_none()
                && browser.active_session_ids.is_empty()
                // Remote-view acquisition reserves the replacement display before
                // profile selection. That exact pending lease is preparation for
                // this relaunch, not evidence that the terminal browser is live.
                && browser
                    .display_allocation_id
                    .as_deref()
                    .is_none_or(&prepared_remote_display_matches)
                && browser.tab_handles.iter().all(&inert_handle)
        } else {
            browser.profile_id.as_deref() != Some(profile_id)
                && !browser
                    .active_session_ids
                    .iter()
                    .any(|active_session| active_session == session_id)
        }
    });
    let tab_projection_inert = state.tabs.values().all(|tab| {
        let related = tab.browser_id == logical_browser_id
            || tab.owner_session_id.as_deref() == Some(session_id)
            || tab.service_tab_handle.as_ref().is_some_and(|handle| {
                handle.browser_id == logical_browser_id
                    || handle.session_name.as_deref() == Some(session_id)
            });
        !related
            || (tab.lifecycle == crate::native::service_model::TabLifecycle::Closed
                && tab.service_tab_handle.as_ref().is_some_and(&inert_handle))
    });
    let owner_projection_inert =
        session_projection_inert && browser_projection_inert && tab_projection_inert;
    let principal_projection_absent = !state
        .runtime_owner_registry
        .principal_bindings
        .contains_key(&profile_digest);
    if !(exact_terminal_owner && owner_projection_inert && principal_projection_absent) {
        return Ok(false);
    }
    (options.runtime_profile, options.profile) =
        retained_profile_launch_identity(profile_id, profile);
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && command.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(true)
}

fn apply_authenticated_orphaned_owner_recourse(
    options: &mut LaunchOptions,
    command: &Value,
    session_id: &str,
    state: &ServiceState,
    binding: &crate::runtime_owner_transfer::RuntimeOwnerBinding,
) -> Result<bool, String> {
    if !matches!(
        command.get("action").and_then(Value::as_str),
        Some("tab_new" | "remote_view_open" | "launch")
    ) || command
        .get("servicePrincipalProvenance")
        .and_then(Value::as_str)
        != Some("registered_capability")
    {
        return Ok(false);
    }
    let Some(principal_id) = command
        .get("servicePrincipalId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(profile_id) = optional_command_or_params_string(command, "runtimeProfile")
        .or_else(|| optional_command_or_params_string(command, "profileId"))
    else {
        return Ok(false);
    };
    let Some(profile) = state.profiles.get(&profile_id) else {
        return Ok(false);
    };
    let principal_active = state
        .service_principals
        .principals
        .get(principal_id)
        .is_some_and(|principal| {
            principal.state
                == crate::native::service_principal::ServicePrincipalState::Active
                && principal.provenance
                    == crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability
        });
    let capability = state
        .service_principals
        .profile_capabilities
        .values()
        .find(|capability| {
            capability.principal_id == principal_id
                && capability.profile_id == profile_id
                && capability.state
                    == crate::native::service_principal::ServiceProfileCapabilityState::Active
        });
    if !principal_active || capability.is_none() {
        return Ok(false);
    }
    let user_data_dir =
        resolved_service_profile_identity_path(profile.user_data_dir.as_deref(), &profile_id)?;
    let profile_digest = crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)?;
    let principal_binding = state
        .runtime_owner_registry
        .principal_bindings
        .get(&profile_digest);
    let owner_projection_absent = !state.sessions.contains_key(session_id)
        && !state
            .browsers
            .contains_key(&binding.claim.logical_browser_id)
        && !state.browsers.values().any(|browser| {
            browser
                .active_session_ids
                .iter()
                .any(|active_session| active_session == session_id)
        });
    // A normal owned shutdown releases the session and clears the browser's
    // reverse session link before retained diagnostics are pruned. That exact
    // inert projection is recoverable by the registered principal that owns
    // the same profile capability. PID, CDP, active-session, and valid-tab
    // evidence still fail closed, as do any foreign profile references.
    let owner_projection_inert = state
        .sessions
        .get(session_id)
        .zip(state.browsers.get(&binding.claim.logical_browser_id))
        .is_some_and(|(session, browser)| {
            session.profile_id.as_deref() == Some(profile_id.as_str())
                && session.browser_ids.len() == 1
                && session.browser_ids.first().map(String::as_str)
                    == Some(binding.claim.logical_browser_id.as_str())
                && matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                && session.tab_ids.is_empty()
                && session
                    .principal_id
                    .as_deref()
                    .is_none_or(|retained_principal| retained_principal == principal_id)
                && browser.profile_id.as_deref() == Some(profile_id.as_str())
                && browser.pid.is_none()
                && browser.cdp_endpoint.is_none()
                && browser.active_session_ids.is_empty()
                && browser.tab_handles.iter().all(|handle| !handle.valid)
        });
    let owner_projection_session_only = !state
        .browsers
        .contains_key(&binding.claim.logical_browser_id)
        && state.sessions.get(session_id).is_some_and(|session| {
            let tab_refs_are_prelaunch =
                session
                    .tab_ids
                    .iter()
                    .all(|tab_id| match state.tabs.get(tab_id) {
                        Some(tab) => {
                            let handle_is_prelaunch =
                                tab.service_tab_handle.as_ref().is_none_or(|handle| {
                                    handle.browser_id == binding.claim.logical_browser_id
                                        && handle.session_name.as_deref() == Some(session_id)
                                        && handle.tab_id == tab.id
                                        && handle.target_id.is_none()
                                        && handle.profile_id.as_deref() == Some(profile_id.as_str())
                                        && handle.owner_session_id.as_deref() == Some(session_id)
                                        && !handle.valid
                                        && handle.stale_reason.as_deref() == Some("browser_missing")
                                });
                            tab.browser_id == binding.claim.logical_browser_id
                                && tab.owner_session_id.as_deref() == Some(session_id)
                                && tab
                                    .principal_id
                                    .as_deref()
                                    .is_none_or(|retained_principal| {
                                        retained_principal == principal_id
                                    })
                                && matches!(
                                    tab.lifecycle,
                                    TabLifecycle::Unknown | TabLifecycle::Opening
                                )
                                && tab.target_id.is_none()
                                && handle_is_prelaunch
                        }
                        None => {
                            command.get("action").and_then(Value::as_str) == Some("tab_new")
                                && session.tab_ids.len() == 1
                        }
                    });
            session.profile_id.as_deref() == Some(profile_id.as_str())
                && (session.browser_ids.is_empty()
                    || (session.browser_ids.len() == 1
                        && session.browser_ids.first().map(String::as_str)
                            == Some(binding.claim.logical_browser_id.as_str())))
                && tab_refs_are_prelaunch
                && session
                    .principal_id
                    .as_deref()
                    .is_none_or(|retained_principal| retained_principal == principal_id)
                && !state.browsers.values().any(|browser| {
                    browser
                        .active_session_ids
                        .iter()
                        .any(|active_session| active_session == session_id)
                })
        });
    let competing_live_profile_projection = state.browsers.values().any(|browser| {
        browser.id != binding.claim.logical_browser_id
            && browser.profile_id.as_deref() == Some(profile_id.as_str())
            && (browser.pid.is_some()
                || browser.cdp_endpoint.is_some()
                || !browser.active_session_ids.is_empty()
                || browser.tab_handles.iter().any(|handle| handle.valid))
    });
    let profile_identity_mismatch = binding.claim.profile_identity_digest != profile_digest;
    let daemon_route_mismatch = binding.claim.daemon_session_route != session_id;
    let principal_binding_mismatch = principal_binding.is_some_and(|principal_binding| {
        principal_binding.principal_id != principal_id
            || principal_binding.profile_id != profile_id
            || principal_binding.owner_generation != binding.claim.owner_generation
    });
    if (!owner_projection_absent && !owner_projection_inert && !owner_projection_session_only)
        || competing_live_profile_projection
        || profile_identity_mismatch
        || daemon_route_mismatch
        || principal_binding_mismatch
    {
        return Ok(false);
    }
    options.runtime_profile = Some(profile_id);
    options.profile = profile.user_data_dir.clone();
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && command.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(true)
}

fn apply_registered_session_profile_continuity(
    options: &mut LaunchOptions,
    command: &Value,
    session_id: &str,
    state: &ServiceState,
) -> Result<bool, String> {
    // A browser exit can temporarily remove effect-capable runtime ownership
    // before replay creates the next owner generation. Only a current registered
    // capability-derived session work lease may carry profile identity across
    // that gap. Retained labels and legacy observations never qualify.
    let Some(session) = state.sessions.get(session_id) else {
        return Ok(false);
    };
    let now = crate::native::service_trace::service_commands::service_now_timestamp();
    let Some(profile_id) = session.profile_id.as_deref() else {
        return Ok(false);
    };
    let Some(authority) = crate::native::service_principal::authenticated_session_work_authority(
        state, session_id, &now,
    ) else {
        return Ok(false);
    };
    let Some(profile) = state.profiles.get(profile_id) else {
        return Ok(false);
    };
    let user_data_dir = profile
        .user_data_dir
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or(crate::runtime_profile::runtime_profile_user_data_dir(
            profile_id,
        )?);
    let profile_digest = crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)?;
    let Some(principal_binding) = state
        .runtime_owner_registry
        .principal_bindings
        .get(&profile_digest)
    else {
        return Ok(false);
    };
    if principal_binding.principal_id != authority.principal_id
        || principal_binding.profile_id != profile_id
        || principal_binding.capability_id != authority.capability_id
        || principal_binding.provenance != authority.provenance
    {
        return Ok(false);
    }
    if options
        .runtime_profile
        .as_deref()
        .is_some_and(|requested| requested != profile_id)
    {
        return Err("explicit_profile_conflicts_with_registered_work_lease".to_string());
    }
    if let Some(requested_path) = options.profile.as_deref() {
        let requested_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(requested_path),
        )?;
        if requested_digest != profile_digest {
            return Err("explicit_profile_conflicts_with_registered_work_lease".to_string());
        }
    }
    (options.runtime_profile, options.profile) =
        retained_profile_launch_identity(profile_id, profile);
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && command.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(true)
}

/// Evaluate only the caller's current shared-local policy permission. This
/// neither validates a browser identity nor grants capability continuity.
fn shared_local_profile_use_allowed(
    profile: &BrowserProfile,
    profile_id: &str,
    command: &Value,
) -> bool {
    let Some(subject_id) = command
        .get("clientSubjectId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let assurance = match command.get("identityAssurance").and_then(Value::as_str) {
        Some("self-declared") => ProfileIdentityAssurance::SelfDeclared,
        Some("authenticated-ingress") => ProfileIdentityAssurance::AuthenticatedIngress,
        Some("registered-capability") => ProfileIdentityAssurance::RegisteredCapability,
        Some("operator") => ProfileIdentityAssurance::Operator,
        _ => return false,
    };
    let policy = profile
        .access_policy
        .clone()
        .unwrap_or_else(|| ServiceProfileAccessPolicy::shared_local_default(profile_id));
    policy.mode == ProfileAccessMode::SharedLocal
        && effective_profile_permissions(&policy, Some(subject_id), assurance)
            .contains(&ProfilePermission::ProfileUse)
}

fn apply_shared_local_session_profile_continuity(
    options: &mut LaunchOptions,
    command: &Value,
    session_id: &str,
    state: &ServiceState,
) -> Result<bool, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Ok(false);
    };
    let Some(profile_id) = session.profile_id.as_deref() else {
        return Ok(false);
    };
    let Some(profile) = state.profiles.get(profile_id) else {
        return Ok(false);
    };
    if !shared_local_profile_use_allowed(profile, profile_id, command) {
        return Ok(false);
    }
    if session.browser_ids.iter().any(|browser_id| {
        state
            .browsers
            .get(browser_id)
            .is_some_and(|browser| browser.profile_id.as_deref() != Some(profile_id))
    }) {
        return Err("existing_session_profile_identity_inconsistent".to_string());
    }
    if options
        .runtime_profile
        .as_deref()
        .is_some_and(|requested| requested != profile_id)
    {
        return Err("explicit_profile_conflicts_with_shared_local_session".to_string());
    }
    let user_data_dir = profile
        .user_data_dir
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or(crate::runtime_profile::runtime_profile_user_data_dir(
            profile_id,
        )?);
    if let Some(requested_path) = options.profile.as_deref() {
        let requested_digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(requested_path),
        )?;
        let profile_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&user_data_dir)?;
        if requested_digest != profile_digest {
            return Err("explicit_profile_conflicts_with_shared_local_session".to_string());
        }
    }
    (options.runtime_profile, options.profile) =
        retained_profile_launch_identity(profile_id, profile);
    if profile.browser_build == Some(BrowserBuild::StockChrome)
        && command.get("executablePath").is_none()
    {
        options.executable_path = None;
    }
    Ok(true)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserCapabilityLaunchSelection {
    pub(crate) binding_id: String,
    pub(crate) host_id: String,
    pub(crate) executable_id: String,
    pub(crate) capability_id: Option<String>,
    pub(crate) executable_path: String,
    pub(crate) profile_compatibility_ids: Vec<String>,
    pub(crate) validation_evidence_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserCapabilityLaunchResolution {
    pub(crate) applied: bool,
    pub(crate) reason: &'static str,
    pub(crate) browser_build: Option<BrowserBuild>,
    pub(crate) profile_id: Option<String>,
    pub(crate) selection: Option<BrowserCapabilityLaunchSelection>,
}
impl BrowserCapabilityLaunchResolution {
    pub(crate) fn skipped(
        reason: &'static str,
        browser_build: Option<BrowserBuild>,
        profile_id: Option<String>,
    ) -> Self {
        Self {
            applied: false,
            reason,
            browser_build,
            profile_id,
            selection: None,
        }
    }
    pub(crate) fn applied(
        browser_build: BrowserBuild,
        profile_id: Option<String>,
        selection: BrowserCapabilityLaunchSelection,
    ) -> Self {
        Self {
            applied: true,
            reason: "validated_binding_applied",
            browser_build: Some(browser_build),
            profile_id,
            selection: Some(selection),
        }
    }
    pub(crate) fn to_value(&self) -> Value {
        let mut value = json!(
            { "applied" : self.applied, "reason" : self.reason, "browserBuild" : self
            .browser_build.map(browser_build_label), "profileId" : self.profile_id, }
        );
        if let Some(selection) = self.selection.as_ref() {
            value["bindingId"] = json!(selection.binding_id);
            value["hostId"] = json!(selection.host_id);
            value["executableId"] = json!(selection.executable_id);
            value["capabilityId"] = json!(selection.capability_id);
            value["executablePath"] = json!(selection.executable_path);
            value["profileCompatibilityIds"] = json!(selection.profile_compatibility_ids);
            value["validationEvidenceIds"] = json!(selection.validation_evidence_ids);
        }
        value
    }
}
pub(crate) fn apply_service_browser_capability_selection(
    options: &mut LaunchOptions,
    cmd: &Value,
) -> BrowserCapabilityLaunchResolution {
    let profile_id = launch_options_service_profile_id(options);
    if executable_path_is_operator_supplied(options.executable_path.as_deref(), cmd) {
        return BrowserCapabilityLaunchResolution::skipped(
            "explicit_executable_path",
            browser_build_from_command(cmd),
            profile_id,
        );
    }
    let Some(browser_build) = browser_build_from_command(cmd) else {
        return BrowserCapabilityLaunchResolution::skipped(
            "missing_browser_build",
            None,
            profile_id,
        );
    };
    let Ok(service_state) = browser_capability_service_state(cmd) else {
        return BrowserCapabilityLaunchResolution::skipped(
            "service_state_unavailable",
            Some(browser_build),
            profile_id,
        );
    };
    let cdp_free = options.manual_login && !options.attachable && !options.remote_headed;
    let selection = match select_browser_capability_launch_binding(
        &service_state,
        cmd,
        browser_build,
        profile_id.as_deref(),
        options.headless,
        cdp_free,
    ) {
        Ok(selection) => selection,
        Err(reason) => {
            return BrowserCapabilityLaunchResolution::skipped(
                reason,
                Some(browser_build),
                profile_id,
            );
        }
    };
    options.executable_path = Some(selection.executable_path.clone());
    BrowserCapabilityLaunchResolution::applied(browser_build, profile_id, selection)
}
