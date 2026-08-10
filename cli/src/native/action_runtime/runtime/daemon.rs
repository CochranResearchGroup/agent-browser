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
use crate::native::cdp::types::{
    AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
    DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
    TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
};
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
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
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
    pub(crate) engine: String,
    pub(crate) host: ServiceBrowserHost,
    pub(crate) close_browser_on_close: bool,
    #[serde(default)]
    pub(crate) active_target_id: Option<String>,
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
    if let Some(ref mut mgr) = state.browser {
        let _ = mgr.close().await;
    }
    state.browser = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
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
pub(crate) fn apply_service_profile_selection(
    options: &mut LaunchOptions,
    cmd: &Value,
) -> Option<ProfileSelectionReason> {
    if options.profile.is_some() {
        return None;
    }
    let service_owned_launch = cmd.get("action").and_then(Value::as_str) == Some("launch")
        && optional_command_string(cmd, "serviceName").is_some();
    let explicit_profile_id = service_owned_launch.then(|| {
        optional_command_or_params_string(cmd, "runtimeProfile")
            .or_else(|| optional_command_or_params_string(cmd, "profileId"))
    });
    if let Some(profile_id) = explicit_profile_id.flatten() {
        let repository = LockedServiceStateRepository::default_json().ok()?;
        let service_state = repository.load_snapshot().ok()?;
        let profile = service_state.profiles.get(&profile_id)?;
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
        return Some(ProfileSelectionReason::ExplicitProfile);
    }
    if options.runtime_profile.is_some() {
        return None;
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
        return None;
    }
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let selection = select_service_profile_for_request(&service_state, &request)?;
    let profile = service_state.profiles.get(&selection.profile_id)?;
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
    Some(selection.reason)
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
