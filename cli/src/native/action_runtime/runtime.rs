#![allow(unused_imports)]
use super::browser_operations::{
    add_manual_login_hint_warning, har_cdp_protocol_to_http_version, har_extract_headers,
    persist_service_owned_navigate_tab, resolve_fetch_paused, stream_file_path, write_engine_file,
    write_extensions_file, write_provider_file,
};
use super::common::*;
use super::service_workflows::{runtime_handoff_path, write_runtime_handoff};
use crate::native::actions::cancellable;
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
pub(crate) fn browser_capability_service_state(cmd: &Value) -> Result<ServiceState, String> {
    if let Some(service_state) = cmd.get("serviceState") {
        return serde_json::from_value::<ServiceState>(service_state.clone())
            .map_err(|err| format!("Invalid serviceState: {}", err));
    }
    LockedServiceStateRepository::default_json()
        .and_then(|repository| repository.load_snapshot())
        .map_err(|err| err.to_string())
}
pub(crate) fn executable_path_is_operator_supplied(
    executable_path: Option<&str>,
    cmd: &Value,
) -> bool {
    if cmd.get("executablePath").is_some() {
        return !matches!(
            optional_command_string(cmd, "executablePathSource").as_deref(),
            Some("manifest")
        );
    }
    let Some(executable_path) = executable_path else {
        return false;
    };
    let Ok(env_executable_path) = env::var("AGENT_BROWSER_EXECUTABLE_PATH") else {
        return true;
    };
    if env_executable_path != executable_path {
        return true;
    }
    !matches!(
        env::var("AGENT_BROWSER_EXECUTABLE_PATH_SOURCE").as_deref(),
        Ok("manifest")
    )
}
pub(crate) fn select_browser_capability_launch_binding(
    service_state: &ServiceState,
    cmd: &Value,
    browser_build: BrowserBuild,
    profile_id: Option<&str>,
    headless: bool,
    cdp_free: bool,
) -> Result<BrowserCapabilityLaunchSelection, &'static str> {
    let registry = &service_state.browser_capability_registry;
    let browser_build_label = browser_build_label(browser_build);
    let binding = registry
        .browser_preference_bindings
        .iter()
        .filter(|binding| {
            preference_binding_matches_launch_command(binding, cmd, Some(browser_build_label))
        })
        .max_by(|left, right| {
            preference_binding_rank(left, cmd).cmp(&preference_binding_rank(right, cmd))
        })
        .ok_or("no_matching_preference_binding")?;
    let binding_id = registry_string_field(binding, "id").ok_or("binding_missing_id")?;
    let executable_id = registry_string_field(binding, "preferredExecutableId")
        .ok_or("binding_missing_executable_id")?;
    let executable = registry
        .browser_executables
        .iter()
        .find(|candidate| {
            registry_string_field(candidate, "id").as_deref() == Some(executable_id.as_str())
                && registry_string_field(candidate, "buildLabel").as_deref()
                    == Some(browser_build_label)
        })
        .ok_or("executable_not_found")?;
    let executable_path =
        registry_string_field(executable, "executablePath").ok_or("executable_path_missing")?;
    if !PathBuf::from(&executable_path).is_file() {
        return Err("executable_path_not_found");
    }
    let host_id = registry_string_field(binding, "preferredHostId")
        .or_else(|| registry_string_field(executable, "hostId"))
        .ok_or("host_id_missing")?;
    let host = registry
        .browser_hosts
        .iter()
        .find(|candidate| {
            registry_string_field(candidate, "id").as_deref() == Some(host_id.as_str())
        })
        .ok_or("host_not_found")?;
    if registry_string_field(host, "hostKind").as_deref() != Some("local")
        || host.get("reachable").and_then(Value::as_bool) != Some(true)
        || registry_string_field(host, "lifecycleOwner").as_deref() != Some("agent_browser")
    {
        return Err("host_not_local_reachable_agent_browser_owned");
    }
    let capability_id = registry_string_field(binding, "preferredCapabilityId");
    let capability = capability_id
        .as_ref()
        .and_then(|id| {
            registry.browser_capabilities.iter().find(|candidate| {
                registry_string_field(candidate, "id").as_deref() == Some(id.as_str())
            })
        })
        .or_else(|| {
            registry.browser_capabilities.iter().find(|candidate| {
                registry_string_field(candidate, "executableId").as_deref()
                    == Some(executable_id.as_str())
                    && registry_string_field(candidate, "hostId").as_deref()
                        == Some(host_id.as_str())
            })
        })
        .ok_or("capability_not_found")?;
    let capability_id = capability_id.or_else(|| registry_string_field(capability, "id"));
    if registry_string_field(capability, "executableId").as_deref() != Some(executable_id.as_str())
    {
        return Err("capability_executable_mismatch");
    }
    if headless {
        if capability.get("headlessSupported").and_then(Value::as_bool) != Some(true) {
            return Err("headless_not_supported");
        }
    } else if capability.get("headedSupported").and_then(Value::as_bool) != Some(true) {
        return Err("headed_not_supported");
    }
    if cdp_free {
        if capability
            .get("cdpFreeLaunchSupported")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err("cdp_free_launch_not_supported");
        }
    } else if capability.get("cdpSupported").and_then(Value::as_bool) != Some(true) {
        return Err("cdp_not_supported");
    }
    let profile_compatibility =
        profile_compatibility_gate(service_state, profile_id, &host_id, &executable_id);
    if !profile_compatibility.allowed {
        return Err("profile_compatibility_missing_or_blocked");
    }
    let validation = validation_gate(
        service_state,
        &host_id,
        &executable_id,
        capability_id.as_deref(),
        cdp_free,
    );
    if !validation.allowed {
        return Err("validation_evidence_missing_or_not_passed");
    }
    Ok(BrowserCapabilityLaunchSelection {
        binding_id,
        host_id,
        executable_id,
        capability_id,
        executable_path,
        profile_compatibility_ids: profile_compatibility.allowed_ids,
        validation_evidence_ids: validation.passed_ids,
    })
}
pub(crate) fn browser_build_label(browser_build: BrowserBuild) -> &'static str {
    match browser_build {
        BrowserBuild::StockChrome => "stock_chrome",
        BrowserBuild::StealthcdpChromium => "stealthcdp_chromium",
        BrowserBuild::CdpFreeHeaded => "cdp_free_headed",
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileCompatibilityGate {
    pub(crate) allowed: bool,
    pub(crate) allowed_ids: Vec<String>,
}
pub(crate) fn profile_compatibility_gate(
    service_state: &ServiceState,
    profile_id: Option<&str>,
    host_id: &str,
    executable_id: &str,
) -> ProfileCompatibilityGate {
    let Some(profile_id) = profile_id else {
        return ProfileCompatibilityGate {
            allowed: true,
            allowed_ids: Vec::new(),
        };
    };
    let matching_rows = service_state
        .browser_capability_registry
        .profile_compatibility
        .iter()
        .filter(|compatibility| {
            registry_string_field(compatibility, "profileId").as_deref() == Some(profile_id)
                && registry_string_field(compatibility, "hostId").as_deref() == Some(host_id)
                && registry_string_field(compatibility, "executableId").as_deref()
                    == Some(executable_id)
        })
        .collect::<Vec<_>>();
    let blocked = matching_rows.iter().any(|compatibility| {
        compatibility.get("compatible").and_then(Value::as_bool) != Some(true)
            || compatibility
                .get("requiresOperatorOverride")
                .and_then(Value::as_bool)
                == Some(true)
    });
    let allowed_ids = matching_rows
        .iter()
        .filter(|compatibility| {
            compatibility.get("compatible").and_then(Value::as_bool) == Some(true)
                && compatibility
                    .get("requiresOperatorOverride")
                    .and_then(Value::as_bool)
                    != Some(true)
        })
        .filter_map(|compatibility| registry_string_field(compatibility, "id"))
        .collect::<Vec<_>>();
    ProfileCompatibilityGate {
        allowed: !allowed_ids.is_empty() && !blocked,
        allowed_ids,
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ValidationGate {
    pub(crate) allowed: bool,
    pub(crate) passed_ids: Vec<String>,
}
pub(crate) fn validation_gate(
    service_state: &ServiceState,
    host_id: &str,
    executable_id: &str,
    capability_id: Option<&str>,
    cdp_free: bool,
) -> ValidationGate {
    let matching_rows = service_state
        .browser_capability_registry
        .validation_evidence
        .iter()
        .filter(|evidence| {
            registry_string_field(evidence, "hostId").as_deref() == Some(host_id)
                && registry_string_field(evidence, "executableId").as_deref() == Some(executable_id)
                && capability_id.is_none_or(|capability_id| {
                    registry_string_field(evidence, "capabilityId").as_deref()
                        == Some(capability_id)
                })
                && validation_kind_matches_launch(evidence, cdp_free)
        })
        .collect::<Vec<_>>();
    let blocked = matching_rows.iter().any(|evidence| {
        matches!(
            evidence.get("state").and_then(Value::as_str),
            Some("failed") | Some("stale")
        )
    });
    let passed_ids = matching_rows
        .iter()
        .filter(|evidence| evidence.get("state").and_then(Value::as_str) == Some("passed"))
        .filter_map(|evidence| registry_string_field(evidence, "id"))
        .collect::<Vec<_>>();
    ValidationGate {
        allowed: !passed_ids.is_empty() && !blocked,
        passed_ids,
    }
}
pub(crate) fn validation_kind_matches_launch(evidence: &Value, cdp_free: bool) -> bool {
    matches!(
        evidence.get("kind").and_then(Value::as_str), Some("launch") |
        Some("site_reliability") | Some("cdp_attach") if ! cdp_free
    ) || cdp_free
        && matches!(
            evidence.get("kind").and_then(Value::as_str),
            Some("launch") | Some("cdp_free_launch") | Some("site_reliability")
        )
}
pub(crate) fn preference_binding_matches_launch_command(
    binding: &Value,
    cmd: &Value,
    browser_build_label: Option<&str>,
) -> bool {
    let browser_build_matches = browser_build_label.is_none_or(|label| {
        registry_string_field(binding, "browserBuild")
            .as_deref()
            .is_none_or(|build| build == label)
    });
    let target_service_ids = target_service_ids_from_command(cmd);
    let account_ids = account_ids_from_command(cmd);
    let service_name = optional_command_string(cmd, "serviceName");
    let task_name = optional_command_string(cmd, "taskName");
    let has_filters = registry_array_field_has_items(binding, "targetServiceIds")
        || registry_array_field_has_items(binding, "accountIds")
        || registry_array_field_has_items(binding, "serviceNames")
        || registry_array_field_has_items(binding, "taskNames");
    let identity_matches = registry_string_field(binding, "scope").as_deref() == Some("global")
        && !has_filters
        || has_filters
            && registry_binding_filter_matches(binding, "targetServiceIds", &target_service_ids)
            && registry_binding_filter_matches(binding, "accountIds", &account_ids)
            && registry_binding_optional_filter_matches(
                binding,
                "serviceNames",
                service_name.as_deref(),
            )
            && registry_binding_optional_filter_matches(binding, "taskNames", task_name.as_deref());
    browser_build_matches && identity_matches
}
pub(crate) fn preference_binding_rank(binding: &Value, cmd: &Value) -> (i64, i64, String) {
    let priority = binding
        .get("priority")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let specificity = i64::from(registry_array_field_intersects(
        binding,
        "accountIds",
        &account_ids_from_command(cmd),
    )) * 16
        + i64::from(registry_array_field_intersects(
            binding,
            "targetServiceIds",
            &target_service_ids_from_command(cmd),
        )) * 8
        + i64::from(
            optional_command_string(cmd, "serviceName").is_some_and(|service_name| {
                registry_array_field_contains(binding, "serviceNames", &service_name)
            }),
        ) * 4
        + i64::from(
            optional_command_string(cmd, "taskName").is_some_and(|task_name| {
                registry_array_field_contains(binding, "taskNames", &task_name)
            }),
        ) * 2
        + i64::from(registry_string_field(binding, "scope").as_deref() != Some("global"));
    let id = registry_string_field(binding, "id").unwrap_or_default();
    (priority, specificity, id)
}
pub(crate) fn registry_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
pub(crate) fn registry_array_field_contains(value: &Value, field: &str, expected: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .any(|item| item == expected)
        })
}
pub(crate) fn registry_array_field_has_items(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str().is_some_and(|item| !item.is_empty()))
        })
}
pub(crate) fn registry_array_field_intersects(
    value: &Value,
    field: &str,
    expected: &[String],
) -> bool {
    expected
        .iter()
        .any(|item| registry_array_field_contains(value, field, item))
}
pub(crate) fn registry_binding_filter_matches(
    value: &Value,
    field: &str,
    expected: &[String],
) -> bool {
    !registry_array_field_has_items(value, field)
        || registry_array_field_intersects(value, field, expected)
}
pub(crate) fn registry_binding_optional_filter_matches(
    value: &Value,
    field: &str,
    expected: Option<&str>,
) -> bool {
    !registry_array_field_has_items(value, field)
        || expected.is_some_and(|expected| registry_array_field_contains(value, field, expected))
}
pub(crate) fn close_behavior_for_attached_browser(
    runtime_attach_managed: bool,
    leave_open: bool,
) -> CloseBehavior {
    if runtime_attach_managed && !leave_open {
        CloseBehavior::CloseBrowser
    } else {
        CloseBehavior::Detach
    }
}
pub(crate) fn close_behavior_for_launched_browser(
    runtime_profile_name: Option<&str>,
    leave_open: bool,
) -> CloseBehavior {
    if leave_open && runtime_profile_name.is_some() {
        CloseBehavior::Detach
    } else {
        CloseBehavior::CloseBrowser
    }
}
pub(crate) fn service_browser_id(session_id: &str) -> String {
    format!("session:{}", session_id)
}
pub(crate) struct CdpFreeLaunchPlan {
    pub(crate) launch_options: LaunchOptions,
    pub(crate) metadata: ServiceLaunchMetadata,
    pub(crate) url: Option<String>,
}
impl ServiceLaunchMetadata {
    /// Captures the service-model metadata that can be inferred from a launch
    /// command before Chrome starts, so every launch path writes the same
    /// browser/profile/session relationships into persisted service state.
    pub(crate) fn from_launch_options(
        options: &LaunchOptions,
        command: Option<&Value>,
        selection_reason: Option<ProfileSelectionReason>,
    ) -> Self {
        let profile_id = launch_options_service_profile_id(options);
        let user_data_dir = options.profile.clone().or_else(|| {
            options.runtime_profile.as_ref().and_then(|name| {
                runtime_profile_user_data_dir(name)
                    .ok()
                    .map(|path| path.to_string_lossy().to_string())
            })
        });
        Self {
            profile_name: options.runtime_profile.clone().or(options.profile.clone()),
            user_data_dir,
            persistent_profile: profile_id.is_some(),
            keyring: if options.use_real_keychain {
                ProfileKeyringPolicy::RealOsKeychain
            } else {
                ProfileKeyringPolicy::BasicPasswordStore
            },
            profile_id: profile_id.clone(),
            service_name: command.and_then(|cmd| optional_command_string(cmd, "serviceName")),
            agent_name: command.and_then(|cmd| optional_command_string(cmd, "agentName")),
            task_name: command.and_then(|cmd| optional_command_string(cmd, "taskName")),
            cleanup: if command
                .and_then(|cmd| cmd.get("leaveOpen"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                SessionCleanupPolicy::Detach
            } else {
                SessionCleanupPolicy::CloseBrowser
            },
            profile_selection_reason: selection_reason.or_else(|| {
                profile_id
                    .is_some()
                    .then_some(ProfileSelectionReason::ExplicitProfile)
            }),
            browser_stderr_log_path: None,
            browser_capability_launch: None,
            view_streams: command
                .map(remote_headed_view_streams_from_command)
                .unwrap_or_default(),
            display_isolation: remote_headed_display_isolation(options),
            display_name: match remote_headed_display_isolation(options).as_deref() {
                Some("private_virtual_display") => None,
                _ => options.display.clone().or_else(|| {
                    (options.remote_headed && !options.headless)
                        .then(|| env::var("DISPLAY").ok())
                        .flatten()
                }),
            },
        }
    }
}
pub(crate) fn launch_options_service_profile_id(options: &LaunchOptions) -> Option<String> {
    if options
        .runtime_profile
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return options.runtime_profile.clone();
    }
    let profile = options.profile.as_deref()?.trim();
    if profile.is_empty() {
        return None;
    }
    if looks_like_path(profile) {
        return service_profile_id(Some(profile), None);
    }
    Some(profile.to_string())
}
pub(crate) fn remote_headed_display_isolation(options: &LaunchOptions) -> Option<String> {
    if !options.remote_headed || options.headless {
        return None;
    }
    if let Some(value) = options.remote_headed_display_isolation.as_ref() {
        return Some(value.clone());
    }
    if options.display.is_some() {
        return Some("shared_display".to_string());
    }
    Some("private_virtual_display".to_string())
}
pub(crate) fn optional_command_string(command: &Value, name: &str) -> Option<String> {
    command
        .get(name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
pub(crate) fn optional_command_or_params_string(command: &Value, name: &str) -> Option<String> {
    optional_command_string(command, name).or_else(|| {
        command
            .get("params")
            .and_then(|params| optional_command_string(params, name))
    })
}
pub(crate) fn optional_command_or_params_bool(command: &Value, name: &str) -> Option<bool> {
    command_or_params_value(command, name).and_then(Value::as_bool)
}
pub(crate) fn manual_login_launch_from_command(
    command: &Value,
    headless: bool,
) -> Result<bool, String> {
    let enabled = optional_command_or_params_bool(command, "manualLoginLaunch").unwrap_or(false);
    if enabled && headless {
        return Err(
            "manual_login_launch_requires_headed: set headless=false for the minimal manual-login-safe Chrome launch posture"
                .to_string(),
        );
    }
    Ok(enabled)
}
pub(crate) fn command_or_params_value<'a>(command: &'a Value, name: &str) -> Option<&'a Value> {
    command
        .get(name)
        .or_else(|| command.get("params").and_then(|params| params.get(name)))
}
pub(crate) fn browser_host_from_command(command: &Value) -> Option<ServiceBrowserHost> {
    for name in ["browserHost", "browser_host", "browser-host"] {
        if let Some(host) = command.get(name).and_then(Value::as_str) {
            return parse_service_browser_host(host);
        }
    }
    command.get("params").and_then(browser_host_from_command)
}
pub(crate) fn parse_service_browser_host(value: &str) -> Option<ServiceBrowserHost> {
    match value.trim() {
        "local_headless" | "local-headless" => Some(ServiceBrowserHost::LocalHeadless),
        "local_headed" | "local-headed" => Some(ServiceBrowserHost::LocalHeaded),
        "docker_headed" | "docker-headed" => Some(ServiceBrowserHost::DockerHeaded),
        "remote_headed" | "remote-headed" => Some(ServiceBrowserHost::RemoteHeaded),
        "cloud_provider" | "cloud-provider" => Some(ServiceBrowserHost::CloudProvider),
        "attached_existing" | "attached-existing" => Some(ServiceBrowserHost::AttachedExisting),
        _ => None,
    }
}
#[derive(Debug, Clone)]
pub(crate) struct RetainedRemoteHeadedLaunchHint {
    pub(crate) view_streams: Vec<ViewStream>,
    pub(crate) display_isolation: Option<String>,
    pub(crate) display_name: Option<String>,
}
pub(crate) fn command_has_explicit_launch_surface(command: &Value) -> bool {
    browser_host_from_command(command).is_some()
        || command.get("provider").is_some()
        || command.get("cdpUrl").is_some()
        || command.get("cdpPort").is_some()
        || command
            .get("autoConnect")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        || command
            .get("headlessExplicit")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        || command
            .get("params")
            .and_then(|params| params.get("headlessExplicit"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}
pub(crate) fn retained_remote_headed_launch_hint(
    session_id: &str,
    command: &Value,
) -> Option<RetainedRemoteHeadedLaunchHint> {
    if command_has_explicit_launch_surface(command) {
        return None;
    }
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let browser = service_state
        .browsers
        .get(&format!("session:{}", session_id))?;
    if browser.host != ServiceBrowserHost::RemoteHeaded || browser.view_streams.is_empty() {
        return None;
    }
    Some(RetainedRemoteHeadedLaunchHint {
        view_streams: browser.view_streams.clone(),
        display_isolation: browser.display_isolation.clone(),
        display_name: browser.display_name.clone(),
    })
}
pub(crate) fn apply_retained_remote_headed_launch_hints(
    options: &mut LaunchOptions,
    retained: Option<&RetainedRemoteHeadedLaunchHint>,
) {
    let Some(retained) = retained else {
        return;
    };
    options.headless = false;
    options.remote_headed = true;
    if options.remote_headed_display_isolation.is_none() {
        options.remote_headed_display_isolation = retained
            .display_isolation
            .as_deref()
            .and_then(normalize_remote_headed_display_isolation)
            .or_else(|| {
                retained
                    .display_name
                    .as_ref()
                    .map(|_| "shared_display".to_string())
            });
    }
    if options.display.is_none()
        && !matches!(
            options.remote_headed_display_isolation.as_deref(),
            Some("private_virtual_display" | "ambient_display")
        )
    {
        options.display = retained.display_name.clone();
    }
}
pub(crate) fn apply_retained_remote_headed_metadata(
    metadata: &mut ServiceLaunchMetadata,
    retained: Option<&RetainedRemoteHeadedLaunchHint>,
) {
    let Some(retained) = retained else {
        return;
    };
    if metadata.view_streams.is_empty() {
        metadata.view_streams = retained.view_streams.clone();
    }
    if metadata.display_isolation.is_none() {
        metadata.display_isolation = retained.display_isolation.clone();
    }
    if metadata.display_name.is_none() {
        metadata.display_name = retained.display_name.clone();
    }
}
pub(crate) fn apply_launch_host_hints(
    options: &mut LaunchOptions,
    command: &Value,
) -> ServiceBrowserHost {
    let host = browser_host_from_command(command).unwrap_or_else(|| {
        if command.get("provider").is_some() {
            ServiceBrowserHost::CloudProvider
        } else if command.get("cdpUrl").is_some()
            || command.get("cdpPort").is_some()
            || command
                .get("autoConnect")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        {
            ServiceBrowserHost::AttachedExisting
        } else if options.remote_headed {
            ServiceBrowserHost::RemoteHeaded
        } else if options.headless {
            ServiceBrowserHost::LocalHeadless
        } else {
            ServiceBrowserHost::LocalHeaded
        }
    });
    if matches!(
        host,
        ServiceBrowserHost::LocalHeaded
            | ServiceBrowserHost::RemoteHeaded
            | ServiceBrowserHost::DockerHeaded
    ) {
        options.headless = false;
    }
    if host == ServiceBrowserHost::LocalHeadless {
        options.headless = true;
        options.remote_headed = false;
    }
    if host == ServiceBrowserHost::RemoteHeaded {
        options.remote_headed = true;
        let explicit_remote_display = remote_headed_display_from_command_only(command);
        if let Some(display_isolation) = remote_headed_display_isolation_from_command(command) {
            options.remote_headed_display_isolation = Some(display_isolation);
        } else if options.remote_headed_display_isolation.is_none()
            && explicit_remote_display.is_some()
        {
            options.remote_headed_display_isolation = Some("shared_display".to_string());
        } else if options.remote_headed_display_isolation.is_none() {
            options.remote_headed_display_isolation = Some("private_virtual_display".to_string());
        }
        options.display = match options.remote_headed_display_isolation.as_deref() {
            Some("private_virtual_display") | Some("ambient_display") => None,
            _ => remote_headed_display_from_command(command).or_else(|| options.display.clone()),
        };
    } else {
        options.remote_headed = false;
        options.remote_headed_display_isolation = None;
    }
    host
}
pub(crate) fn remote_headed_display_isolation_from_command(command: &Value) -> Option<String> {
    optional_command_string(command, "displayIsolation")
        .or_else(|| optional_command_string(command, "displayAllocation"))
        .or_else(|| optional_command_string(command, "displayAllocationPolicy"))
        .or_else(|| {
            command
                .get("params")
                .and_then(remote_headed_display_isolation_from_command)
        })
        .and_then(|value| normalize_remote_headed_display_isolation(&value))
}
pub(crate) fn normalize_remote_headed_display_isolation(value: &str) -> Option<String> {
    match value.trim() {
        "private_virtual_display" | "private-virtual-display" | "private" => {
            Some("private_virtual_display".to_string())
        }
        "shared_display" | "shared-display" | "shared" => Some("shared_display".to_string()),
        "ambient_display" | "ambient-display" | "ambient" => Some("ambient_display".to_string()),
        _ => None,
    }
}
pub(crate) fn remote_headed_display_from_command(command: &Value) -> Option<String> {
    remote_headed_display_from_command_only(command)
        .or_else(|| env::var("AGENT_BROWSER_REMOTE_HEADED_DISPLAY").ok())
}
pub(crate) fn remote_headed_display_from_command_only(command: &Value) -> Option<String> {
    optional_command_string(command, "remoteHeadedDisplay")
        .or_else(|| optional_command_string(command, "display"))
        .or_else(|| {
            command
                .get("params")
                .and_then(remote_headed_display_from_command_only)
        })
}
pub(crate) fn remote_headed_view_streams_from_command(command: &Value) -> Vec<ViewStream> {
    if browser_host_from_command(command) != Some(ServiceBrowserHost::RemoteHeaded) {
        return Vec::new();
    }
    let provider = optional_command_string(command, "viewStream")
        .or_else(|| optional_command_string(command, "viewStreamProvider"))
        .or_else(|| {
            command.get("params").and_then(|params| {
                optional_command_string(params, "viewStream")
                    .or_else(|| optional_command_string(params, "viewStreamProvider"))
            })
        })
        .or_else(|| env::var("AGENT_BROWSER_REMOTE_VIEW_PROVIDER").ok())
        .and_then(|provider| parse_view_stream_provider(&provider))
        .unwrap_or(ViewStreamProvider::CdpScreencast);
    let url = view_stream_command_string(
        command,
        &["remoteViewUrl", "viewStreamUrl"],
        "AGENT_BROWSER_REMOTE_VIEW_URL",
    );
    let frame_url = view_stream_command_string(
        command,
        &["frameUrl", "viewStreamFrameUrl", "remoteViewFrameUrl"],
        "AGENT_BROWSER_REMOTE_VIEW_FRAME_URL",
    )
    .or_else(|| guacamole_client_url(url.as_deref()));
    let external_url = view_stream_command_string(
        command,
        &[
            "externalUrl",
            "viewStreamExternalUrl",
            "remoteViewExternalUrl",
        ],
        "AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL",
    )
    .or_else(|| guacamole_client_url(url.as_deref()))
    .or_else(|| frame_url.clone());
    let explicit_route_id = view_stream_command_string(
        command,
        &["routeId", "viewStreamRouteId", "guacamoleRouteId"],
        "AGENT_BROWSER_REMOTE_VIEW_ROUTE_ID",
    );
    let connection_id = view_stream_command_string(
        command,
        &["connectionId", "guacamoleConnectionId"],
        "AGENT_BROWSER_GUACAMOLE_CONNECTION_ID",
    )
    .or_else(|| {
        frame_url
            .as_deref()
            .or(external_url.as_deref())
            .or(url.as_deref())
            .and_then(guacamole_connection_id_from_url)
    });
    let route_id = explicit_route_id.or_else(|| {
        connection_id
            .as_ref()
            .map(|value| format!("guacamole:{}", value))
    });
    let connection_name = view_stream_command_string(
        command,
        &["connectionName", "guacamoleConnectionName"],
        "AGENT_BROWSER_GUACAMOLE_CONNECTION_NAME",
    );
    let route_descriptor = command
        .get("routeDescriptor")
        .cloned()
        .or_else(|| command.get("route_descriptor").cloned())
        .or_else(|| {
            command
                .get("params")
                .and_then(|params| params.get("routeDescriptor").cloned())
        });
    let display_allocation_id = optional_command_string(command, "displayAllocationId")
        .or_else(|| optional_command_string(command, "requestedDisplayAllocationId"))
        .or_else(|| {
            command.get("params").and_then(|params| {
                optional_command_string(params, "displayAllocationId")
                    .or_else(|| optional_command_string(params, "requestedDisplayAllocationId"))
            })
        });
    let provider_mode = optional_command_string(command, "providerMode").or_else(|| {
        command
            .get("params")
            .and_then(|params| optional_command_string(params, "providerMode"))
    });
    let route_source = if route_id.is_some()
        || connection_id.is_some()
        || frame_url.is_some()
        || external_url.is_some()
    {
        Some("service_request".to_string())
    } else {
        None
    };
    let url = url
        .or_else(|| frame_url.clone())
        .or_else(|| external_url.clone());
    let control_input = optional_command_string(command, "controlInput")
        .or_else(|| optional_command_string(command, "controlInputProvider"))
        .or_else(|| {
            command.get("params").and_then(|params| {
                optional_command_string(params, "controlInput")
                    .or_else(|| optional_command_string(params, "controlInputProvider"))
            })
        })
        .or_else(|| env::var("AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER").ok())
        .and_then(|provider| parse_control_input_provider(&provider))
        .or_else(|| default_control_input_provider(provider));
    vec![ViewStream {
        id: "remote-headed-view".to_string(),
        provider,
        control_input,
        url,
        frame_url,
        external_url,
        route_descriptor,
        route_id,
        display_allocation_id,
        connection_id,
        connection_name,
        route_source,
        provider_mode,
        viewer_lease_ids: Vec::new(),
        controller_lease_id: None,
        read_only: false,
        readiness: None,
        remote_readiness: None,
        attachability: None,
    }]
}
pub(crate) fn view_stream_command_string(
    command: &Value,
    keys: &[&str],
    env_key: &str,
) -> Option<String> {
    for key in keys {
        if let Some(value) = optional_command_string(command, key) {
            return Some(value);
        }
    }
    if let Some(params) = command.get("params") {
        for key in keys {
            if let Some(value) = optional_command_string(params, key) {
                return Some(value);
            }
        }
    }
    env::var(env_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
pub(crate) fn guacamole_client_url(root_url: Option<&str>) -> Option<String> {
    if let Ok(configured_url) = env::var("AGENT_BROWSER_REMOTE_VIEW_URL") {
        let configured_url = configured_url.trim();
        if !configured_url.is_empty() && configured_url.contains("#/client/") {
            return Some(configured_url.to_string());
        }
    }
    let root_url = root_url.map(str::trim).filter(|url| !url.is_empty())?;
    if root_url.contains("#/client/") {
        return Some(root_url.to_string());
    }
    None
}
pub(crate) fn guacamole_connection_id_from_url(url: &str) -> Option<String> {
    let (_, route) = url.split_once("#/client/")?;
    let connection_id = route
        .split(['?', '&', '#', '/'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(connection_id.to_string())
}
pub(crate) fn parse_view_stream_provider(value: &str) -> Option<ViewStreamProvider> {
    match value.trim() {
        "cdp_screencast" | "cdp-screencast" => Some(ViewStreamProvider::CdpScreencast),
        "chrome_tab_webrtc" | "chrome-tab-webrtc" => Some(ViewStreamProvider::ChromeTabWebrtc),
        "virtual_display_webrtc" | "virtual-display-webrtc" => {
            Some(ViewStreamProvider::VirtualDisplayWebrtc)
        }
        "novnc" => Some(ViewStreamProvider::Novnc),
        "rdp_gateway" | "rdp-gateway" | "rdp" => Some(ViewStreamProvider::RdpGateway),
        "external_url" | "external-url" => Some(ViewStreamProvider::ExternalUrl),
        _ => None,
    }
}
pub(crate) fn parse_control_input_provider(value: &str) -> Option<ControlInputProvider> {
    match value.trim() {
        "cdp_input" | "cdp-input" | "cdp" => Some(ControlInputProvider::CdpInput),
        "webrtc_input" | "webrtc-input" | "webrtc" => Some(ControlInputProvider::WebrtcInput),
        "vnc_input" | "vnc-input" | "vnc" => Some(ControlInputProvider::VncInput),
        "manual_attached_desktop"
        | "manual-attached-desktop"
        | "manual_desktop"
        | "manual-desktop"
        | "manual" => Some(ControlInputProvider::ManualAttachedDesktop),
        _ => None,
    }
}
pub(crate) fn default_control_input_provider(
    provider: ViewStreamProvider,
) -> Option<ControlInputProvider> {
    let input = match provider {
        ViewStreamProvider::CdpScreencast => ControlInputProvider::CdpInput,
        ViewStreamProvider::ChromeTabWebrtc | ViewStreamProvider::VirtualDisplayWebrtc => {
            ControlInputProvider::WebrtcInput
        }
        ViewStreamProvider::Novnc => ControlInputProvider::VncInput,
        ViewStreamProvider::RdpGateway | ViewStreamProvider::ExternalUrl => {
            ControlInputProvider::ManualAttachedDesktop
        }
    };
    Some(input)
}
pub(crate) fn service_browser_host_for_launch(cmd: &Value, headless: bool) -> ServiceBrowserHost {
    if let Some(host) = browser_host_from_command(cmd) {
        return host;
    }
    if cmd.get("provider").is_some() {
        ServiceBrowserHost::CloudProvider
    } else if cmd.get("cdpUrl").is_some()
        || cmd.get("cdpPort").is_some()
        || cmd
            .get("autoConnect")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        ServiceBrowserHost::AttachedExisting
    } else if headless {
        ServiceBrowserHost::LocalHeadless
    } else {
        ServiceBrowserHost::LocalHeaded
    }
}
pub(crate) fn persist_service_browser_record(
    session_id: &str,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    pid: Option<u32>,
    cdp_endpoint: Option<String>,
    last_error: Option<String>,
    metadata: Option<ServiceLaunchMetadata>,
) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ = persist_service_browser_record_in_repository(
            &repository,
            session_id,
            host,
            health,
            pid,
            cdp_endpoint,
            last_error,
            metadata,
        );
    }
}
pub(crate) fn cdp_stream_supported_host(host: ServiceBrowserHost) -> bool {
    matches!(
        host,
        ServiceBrowserHost::LocalHeadless
            | ServiceBrowserHost::LocalHeaded
            | ServiceBrowserHost::AttachedExisting
    )
}
pub(crate) fn cdp_screencast_url(port: u16) -> String {
    format!("http://127.0.0.1:{}/", port)
}
pub(crate) fn cdp_screencast_readiness(
    state: &str,
    reason: &str,
    session_id: &str,
    port: Option<u16>,
    cdp_endpoint: Option<&str>,
) -> Value {
    let mut readiness = json!(
        { "state" : state, "reason" : reason, "sessionName" : session_id, "browserId" :
        service_browser_id(session_id), }
    );
    if let Some(port) = port {
        readiness["streamPort"] = json!(port);
    }
    if let Some(endpoint) = cdp_endpoint {
        readiness["cdpEndpoint"] = json!(endpoint);
    }
    readiness
}
pub(crate) fn cdp_screencast_view_stream(
    session_id: &str,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    cdp_endpoint: Option<&str>,
    stream_port: Option<u16>,
) -> Option<ViewStream> {
    if !cdp_stream_supported_host(host) {
        return None;
    }
    let (ready, reason) = if health != ServiceBrowserHealth::Ready {
        (false, "browser_not_ready")
    } else if cdp_endpoint
        .map(str::trim)
        .is_none_or(|endpoint| endpoint.is_empty())
    {
        (false, "missing_cdp_endpoint")
    } else if stream_port.is_none() {
        (false, "missing_stream_server")
    } else {
        (true, "stream_server_ready")
    };
    let url = if ready {
        stream_port.map(cdp_screencast_url)
    } else {
        None
    };
    Some(ViewStream {
        id: "cdp-screencast".to_string(),
        provider: ViewStreamProvider::CdpScreencast,
        control_input: ready.then_some(ControlInputProvider::CdpInput),
        url: url.clone(),
        frame_url: url.clone(),
        external_url: url,
        route_descriptor: None,
        route_id: None,
        display_allocation_id: None,
        connection_id: None,
        connection_name: None,
        route_source: Some("daemon_stream_server".to_string()),
        provider_mode: Some("simultaneous_view".to_string()),
        viewer_lease_ids: Vec::new(),
        controller_lease_id: None,
        read_only: !ready,
        readiness: Some(cdp_screencast_readiness(
            if ready { "ready" } else { "unavailable" },
            reason,
            session_id,
            stream_port,
            cdp_endpoint,
        )),
        remote_readiness: None,
        attachability: None,
    })
}
pub(crate) fn upsert_cdp_screencast_view_stream(
    metadata: &mut ServiceLaunchMetadata,
    session_id: &str,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    cdp_endpoint: Option<&str>,
    stream_port: Option<u16>,
) {
    let Some(cdp_stream) =
        cdp_screencast_view_stream(session_id, host, health, cdp_endpoint, stream_port)
    else {
        return;
    };
    if let Some(existing) = metadata
        .view_streams
        .iter_mut()
        .find(|stream| stream.provider == ViewStreamProvider::CdpScreencast)
    {
        *existing = cdp_stream;
    } else {
        metadata.view_streams.push(cdp_stream);
    }
}
pub(crate) fn service_browser_session_id(browser: &BrowserProcess) -> Option<String> {
    browser
        .active_session_ids
        .iter()
        .find(|session_id| !session_id.trim().is_empty())
        .cloned()
        .or_else(|| {
            browser
                .id
                .strip_prefix("session:")
                .filter(|session_id| !session_id.trim().is_empty())
                .map(ToOwned::to_owned)
        })
}
pub(crate) fn read_stream_port_for_session(session_id: &str) -> Option<u16> {
    fs::read_to_string(stream_file_path(session_id))
        .ok()
        .and_then(|contents| contents.trim().parse::<u16>().ok())
}
pub(crate) fn upsert_browser_cdp_screencast_view_stream(browser: &mut BrowserProcess) {
    let Some(session_id) = service_browser_session_id(browser) else {
        return;
    };
    let stream_port = read_stream_port_for_session(&session_id);
    let Some(cdp_stream) = cdp_screencast_view_stream(
        &session_id,
        browser.host,
        browser.health,
        browser.cdp_endpoint.as_deref(),
        stream_port,
    ) else {
        return;
    };
    if let Some(existing) = browser
        .view_streams
        .iter_mut()
        .find(|stream| stream.provider == ViewStreamProvider::CdpScreencast)
    {
        *existing = cdp_stream;
    } else {
        browser.view_streams.push(cdp_stream);
    }
}
pub(crate) fn refresh_cdp_screencast_view_streams(service_state: &mut ServiceState) {
    for browser in service_state.browsers.values_mut() {
        upsert_browser_cdp_screencast_view_stream(browser);
    }
}
pub(crate) fn persist_current_browser_health(
    state: &DaemonState,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    metadata: Option<ServiceLaunchMetadata>,
) {
    let preserves_existing_metadata = metadata.is_none();
    let (pid, cdp_endpoint, browser_stderr_log_path) = state
        .browser
        .as_ref()
        .map(|mgr| {
            (
                mgr.browser_pid().or(state.attached_browser_pid),
                Some(mgr.get_cdp_url().to_string()),
                mgr.browser_stderr_log_path()
                    .map(|path| path.to_string_lossy().to_string()),
            )
        })
        .unwrap_or((None, None, None));
    let metadata = metadata.map(|mut metadata| {
        metadata.browser_stderr_log_path = browser_stderr_log_path;
        if metadata.display_name.is_none() {
            metadata.display_name = state
                .browser
                .as_ref()
                .and_then(|mgr| mgr.browser_display_name().map(str::to_string));
        }
        upsert_cdp_screencast_view_stream(
            &mut metadata,
            &state.session_id,
            host,
            health,
            cdp_endpoint.as_deref(),
            state.stream_server.as_ref().map(|server| server.port()),
        );
        metadata
    });
    persist_service_browser_record(
        &state.session_id,
        host,
        health,
        pid,
        cdp_endpoint,
        None,
        metadata,
    );
    if preserves_existing_metadata {
        if let Ok(repository) = LockedServiceStateRepository::default_json() {
            let _ = repository.mutate(|service_state| {
                refresh_cdp_screencast_view_streams(service_state);
                Ok(())
            });
        }
    }
}
/// Enforces service-owned profile leases before Chrome starts.
///
/// The control-plane scheduler handles bounded `wait` policy by requeueing the
/// request so the worker can run other jobs. This launch-path guard remains as
/// a deterministic fallback for direct execution and rejects unresolved waits.
/// The same retained session may still reuse its browser, and non-service
/// launches keep the existing direct-control behavior.
pub(crate) async fn ensure_service_profile_lease_available(
    _metadata: &ServiceLaunchMetadata,
    session_id: &str,
    command: &Value,
) -> Result<(), String> {
    let wait_timeout_ms = profile_lease_wait_timeout_ms_from_command(command)?;
    match service_profile_lease_gate(command, session_id, Some(wait_timeout_ms))? {
        ServiceProfileLeaseGate::Ready => Ok(()),
        ServiceProfileLeaseGate::Reject { error } => Err(error),
        ServiceProfileLeaseGate::Wait { .. } => Err(
            "Service profile lease wait must be handled by the control-plane scheduler".to_string(),
        ),
    }
}
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
pub(crate) fn persist_browser_recovery_started_from_persisted_state(
    state: &DaemonState,
    reason: &str,
) -> BrowserRecoveryPersistence {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        return persist_browser_recovery_started_in_repository(
            &repository,
            &state.session_id,
            state.browser_recovery_policy_config,
            reason,
        );
    }
    BrowserRecoveryPersistence::NotRecorded
}
pub(crate) fn persist_closed_browser_health(
    state: &DaemonState,
    outcome: Option<&BrowserShutdownOutcome>,
) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ =
            persist_closed_browser_health_in_repository(&repository, &state.session_id, outcome);
    }
}
pub(crate) struct DaemonState {
    pub browser: Option<BrowserManager>,
    pub appium: Option<AppiumManager>,
    pub safari_driver: Option<safari::SafariDriverProcess>,
    pub webdriver_backend: Option<super::super::webdriver::backend::WebDriverBackend>,
    pub backend_type: BackendType,
    pub ref_map: RefMap,
    pub domain_filter: Arc<RwLock<Option<DomainFilter>>>,
    pub event_tracker: EventTracker,
    pub session_name: Option<String>,
    pub session_id: String,
    pub tracing_state: TracingState,
    pub recording_state: RecordingState,
    pub(crate) event_rx: Option<broadcast::Receiver<CdpEvent>>,
    pub screencasting: bool,
    pub policy: Option<ActionPolicy>,
    pub pending_confirmation: Option<PendingConfirmation>,
    pub har_recording: bool,
    pub har_entries: Vec<HarEntry>,
    pub confirm_actions: Option<ConfirmActions>,
    pub inspect_server: Option<InspectServer>,
    pub routes: Arc<RwLock<Vec<RouteEntry>>>,
    pub tracked_requests: Vec<TrackedRequest>,
    pub request_tracking: bool,
    pub active_frame_id: Option<String>,
    /// Cross-origin iframe frame_id → dedicated CDP session_id.
    /// Populated by Target.attachedToTarget events from Target.setAutoAttach.
    pub iframe_sessions: HashMap<String, String>,
    /// Origin-scoped extra HTTP headers set via `--headers` on navigate.
    /// Key is the origin (scheme + host + port), value is the headers map.
    /// Wrapped in Arc<RwLock<>> so the background Fetch handler can read it.
    pub origin_headers: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    /// Proxy authentication credentials (username, password) for handling
    /// Fetch.authRequired events from authenticated proxies.
    pub proxy_credentials: Arc<RwLock<Option<(String, String)>>>,
    /// Background task that processes Fetch.requestPaused events in real-time,
    /// handling domain filtering, route interception, and origin-scoped headers
    /// without deadlocking navigation/evaluate.
    pub(crate) fetch_handler_task: Option<tokio::task::JoinHandle<()>>,
    /// Background task that auto-accepts `alert` and `beforeunload` dialogs
    /// so they never block the agent.
    pub(crate) dialog_handler_task: Option<tokio::task::JoinHandle<()>>,
    pub mouse_state: MouseState,
    /// Tracks the currently open JavaScript dialog (alert/confirm/prompt), if any.
    pub pending_dialog: Option<PendingDialog>,
    /// When true, automatically dismiss `beforeunload` dialogs and accept `alert`
    /// dialogs so they never block the agent.  Enabled by default.
    pub auto_dialog: bool,
    /// Shared slot for stream server to receive CDP client when browser launches.
    pub stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
    /// Stream server instance kept alive so the broadcast channel remains open.
    pub stream_server: Option<Arc<StreamServer>>,
    /// Hash of launch options used for the current browser, for relaunch detection.
    pub(crate) launch_hash: Option<u64>,
    /// Runtime profile for a browser attached through CDP that this daemon owns logically.
    pub(crate) attached_runtime_profile: Option<String>,
    /// Process ID for an attached runtime-profile browser, used for explicit close.
    pub(crate) attached_browser_pid: Option<u32>,
    /// Whether closing this daemon session should shut down the browser or detach.
    pub(crate) close_behavior: CloseBehavior,
    /// Browser engine name (e.g. "chrome", "lightpanda") for observability.
    pub engine: String,
    /// Default timeout for wait operations, from AGENT_BROWSER_DEFAULT_TIMEOUT env var.
    pub default_timeout_ms: u64,
    /// Retry budget and backoff used when a stale browser is relaunched.
    pub browser_recovery_policy_config: BrowserRecoveryPolicyConfig,
    /// Cancellation token for the currently running service job, if this
    /// command is executing inside the service control-plane worker.
    pub current_cancellation: Option<CancellationToken>,
    /// Launch-time shared-profile acquisition evidence to attach to the next
    /// command response that consumes the auto-launched tab.
    pub(crate) pending_shared_profile_acquisition: Option<Value>,
    /// Storage mutations made through agent-browser storage commands, keyed by origin.
    /// This preserves cross-origin storage for state saves even after navigation.
    pub(crate) tracked_origin_storage: HashMap<String, state::OriginStorage>,
}
impl DaemonState {
    pub fn new() -> Self {
        Self {
            browser: None,
            appium: None,
            safari_driver: None,
            webdriver_backend: None,
            backend_type: BackendType::Cdp,
            ref_map: RefMap::new(),
            domain_filter: Arc::new(RwLock::new(
                env::var("AGENT_BROWSER_ALLOWED_DOMAINS")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(|s| DomainFilter::new(&s)),
            )),
            event_tracker: EventTracker::new(),
            session_name: env::var("AGENT_BROWSER_SESSION_NAME").ok(),
            session_id: env::var("AGENT_BROWSER_SESSION").unwrap_or_else(|_| "default".to_string()),
            tracing_state: TracingState::new(),
            recording_state: RecordingState::new(),
            event_rx: None,
            screencasting: false,
            policy: ActionPolicy::load_if_exists(),
            pending_confirmation: None,
            har_recording: false,
            har_entries: Vec::new(),
            confirm_actions: ConfirmActions::from_env(),
            inspect_server: None,
            routes: Arc::new(RwLock::new(Vec::new())),
            tracked_requests: Vec::new(),
            request_tracking: false,
            active_frame_id: None,
            iframe_sessions: HashMap::new(),
            origin_headers: Arc::new(RwLock::new(HashMap::new())),
            proxy_credentials: Arc::new(RwLock::new(None)),
            fetch_handler_task: None,
            dialog_handler_task: None,
            mouse_state: MouseState::default(),
            pending_dialog: None,
            auto_dialog: !matches!(
                env::var("AGENT_BROWSER_NO_AUTO_DIALOG").as_deref(),
                Ok("1" | "true" | "yes")
            ),
            stream_client: None,
            stream_server: None,
            launch_hash: None,
            attached_runtime_profile: None,
            attached_browser_pid: None,
            close_behavior: CloseBehavior::CloseBrowser,
            engine: env::var("AGENT_BROWSER_ENGINE").unwrap_or_else(|_| "chrome".to_string()),
            default_timeout_ms: env::var("AGENT_BROWSER_DEFAULT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30_000),
            browser_recovery_policy_config: browser_recovery_policy_config_from_env(),
            current_cancellation: None,
            pending_shared_profile_acquisition: None,
            tracked_origin_storage: HashMap::new(),
        }
    }
    /// Extract the timeout from a command JSON, falling back to the
    /// configured `default_timeout_ms` (from `AGENT_BROWSER_DEFAULT_TIMEOUT`).
    /// All wait-family handlers should use this instead of reading the
    /// timeout field and providing their own fallback.
    pub(crate) fn timeout_ms(&self, cmd: &Value) -> u64 {
        cmd.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout_ms)
    }
    pub(crate) fn reset_input_state(&mut self) {
        self.mouse_state = MouseState::default();
    }
    /// Create state with an optional stream client slot and server instance
    /// (for daemon startup with stream server).
    pub fn new_with_stream(
        stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
        stream_server: Option<Arc<StreamServer>>,
    ) -> Self {
        let mut s = Self::new();
        if stream_server.is_some() {
            s.request_tracking = true;
        }
        s.stream_client = stream_client;
        s.stream_server = stream_server;
        s
    }
    pub(crate) fn subscribe_to_browser_events(&mut self) {
        if let Some(ref browser) = self.browser {
            self.event_rx = Some(browser.client.subscribe());
        }
    }
    /// Start the background task that processes Fetch.requestPaused and
    /// Fetch.authRequired events in real-time (domain filtering, route
    /// interception, origin-scoped headers, proxy authentication).
    /// Must be called after the browser is set and events are subscribed.
    pub(crate) fn start_fetch_handler(&mut self) {
        if let Some(task) = self.fetch_handler_task.take() {
            task.abort();
        }
        let Some(ref browser) = self.browser else {
            return;
        };
        let client = browser.client.clone();
        let mut rx = browser.client.subscribe();
        let domain_filter = self.domain_filter.clone();
        let routes = self.routes.clone();
        let origin_headers = self.origin_headers.clone();
        let proxy_credentials = self.proxy_credentials.clone();
        self.fetch_handler_task = Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) if event.method == "Fetch.authRequired" => {
                        let request_id = event
                            .params
                            .get("requestId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let sid = event.session_id.clone().unwrap_or_default();
                        let creds = proxy_credentials.read().await;
                        if let Some((ref user, ref pass)) = *creds {
                            let _ = client
                                .send_command(
                                    "Fetch.continueWithAuth",
                                    Some(json!(
                                        { "requestId" : request_id, "authChallengeResponse" : {
                                        "response" : "ProvideCredentials", "username" : user,
                                        "password" : pass, } }
                                    )),
                                    Some(&sid),
                                )
                                .await;
                        } else {
                            let _ = client
                                .send_command(
                                    "Fetch.continueWithAuth",
                                    Some(json!(
                                        { "requestId" : request_id, "authChallengeResponse" : {
                                        "response" : "CancelAuth", } }
                                    )),
                                    Some(&sid),
                                )
                                .await;
                        }
                    }
                    Ok(event) if event.method == "Fetch.requestPaused" => {
                        let request_id = event
                            .params
                            .get("requestId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let request_url = event
                            .params
                            .get("request")
                            .and_then(|r| r.get("url"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let resource_type = event
                            .params
                            .get("resourceType")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let request_headers = event
                            .params
                            .get("request")
                            .and_then(|r| r.get("headers"))
                            .and_then(|h| h.as_object())
                            .cloned();
                        let sid = event.session_id.clone().unwrap_or_default();
                        let paused = FetchPausedRequest {
                            request_id,
                            url: request_url,
                            resource_type,
                            session_id: sid,
                            request_headers,
                        };
                        let df = domain_filter.read().await;
                        let rt = routes.read().await;
                        let oh = origin_headers.read().await;
                        resolve_fetch_paused(&client, df.as_ref(), &rt, &oh, &paused).await;
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }));
    }
    /// Start the background task that auto-accepts `alert` and `beforeunload`
    /// dialogs so they never block the agent. `confirm` and `prompt` dialogs
    /// are left for the agent to handle explicitly.
    pub(crate) fn start_dialog_handler(&mut self) {
        if let Some(task) = self.dialog_handler_task.take() {
            task.abort();
        }
        if !self.auto_dialog {
            return;
        }
        let Some(ref browser) = self.browser else {
            return;
        };
        let client = browser.client.clone();
        let mut rx = browser.client.subscribe();
        self.dialog_handler_task = Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) if event.method == "Page.javascriptDialogOpening" => {
                        let dialog_type = event
                            .params
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if matches!(dialog_type, "beforeunload" | "alert") {
                            let message = event
                                .params
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            eprintln!("[auto-dismiss] {} dialog: {}", dialog_type, message);
                            let sid = event.session_id.clone().unwrap_or_default();
                            if let Err(e) = client
                                .send_command(
                                    "Page.handleJavaScriptDialog",
                                    Some(json!({ "accept" : true })),
                                    Some(&sid),
                                )
                                .await
                            {
                                eprintln!(
                                    "[auto-dismiss] failed to dismiss {} dialog: {}",
                                    dialog_type, e
                                );
                            }
                        }
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }));
    }
    /// Update the stream server's CDP client slot when browser is set or cleared.
    pub async fn update_stream_client(&self) {
        if let Some(ref slot) = self.stream_client {
            let mut guard = slot.write().await;
            *guard = self.browser.as_ref().map(|m| Arc::clone(&m.client));
        }
        if let Some(ref server) = self.stream_server {
            let session_id = self
                .browser
                .as_ref()
                .and_then(|m| m.active_session_id().ok().map(|s| s.to_string()));
            server.set_cdp_session_id(session_id).await;
            let connected = self.browser.is_some();
            let sc = server.is_screencasting().await;
            let (vw, vh) = server.viewport().await;
            server
                .broadcast_status(connected, sc, vw, vh, &self.engine)
                .await;
            if let Some(ref mgr) = self.browser {
                server.broadcast_tabs(&mgr.tab_list(false)).await;
            } else {
                server.broadcast_tabs(&[]).await;
            }
            server.notify_client_changed();
        }
    }
    pub(crate) async fn try_recover_browser_connection(&mut self) -> Result<bool, String> {
        let Some(browser) = self.browser.as_mut() else {
            return Ok(false);
        };
        if browser.has_process_exited() || browser.is_connection_alive().await {
            return Ok(false);
        }
        browser.reconnect_client().await?;
        self.subscribe_to_browser_events();
        self.start_fetch_handler();
        self.start_dialog_handler();
        self.update_stream_client().await;
        Ok(true)
    }
    /// Spawn a background task that polls screenshots and pipes them to ffmpeg.
    pub(crate) async fn start_recording_task(
        &mut self,
        client: Arc<CdpClient>,
        session_id: String,
    ) -> Result<(), String> {
        let shared_count = Arc::new(AtomicU64::new(0));
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let handle = recording::spawn_recording_task(
            client,
            session_id,
            self.recording_state.output_path.clone(),
            shared_count.clone(),
            cancel_rx,
        );
        self.recording_state.capture_task = Some(handle);
        self.recording_state.shared_frame_count = Some(shared_count);
        self.recording_state.cancel_tx = Some(cancel_tx);
        Ok(())
    }
    pub(crate) async fn stop_recording_task(&mut self) -> Result<(), String> {
        recording::stop_recording_task(&mut self.recording_state).await
    }
    pub async fn drain_cdp_events_background(&mut self) {
        let drained = self.drain_cdp_events();
        self.apply_drained_events(drained).await;
    }
    pub(crate) async fn apply_drained_events(&mut self, drained: DrainedEvents) {
        if debug_session_events_enabled() {
            if let Some(ref mgr) = self.browser {
                eprintln!(
                    "[agent-browser][sessions] before active={} pages={:?} attached_page={:?} detached_page={:?} changed_targets={} destroyed_targets={:?}",
                    mgr.active_session_id().unwrap_or("<none>"), mgr.pages_list().iter()
                    .map(| p | format!("{} {} {}", p.target_id, p.session_id, p.url))
                    .collect::< Vec < _ >> (), drained.attached_page_sessions, drained
                    .detached_page_sessions, drained.changed_targets.len(), drained
                    .destroyed_targets
                );
            }
        }
        if !drained.pending_acks.is_empty() {
            if let Some(ref browser) = self.browser {
                if let Ok(session_id) = browser.active_session_id() {
                    for ack_sid in drained.pending_acks {
                        let _ = stream::ack_screencast_frame(&browser.client, session_id, ack_sid)
                            .await;
                    }
                }
            }
        }
        for target_id in &drained.destroyed_targets {
            if let Some(ref mut mgr) = self.browser {
                mgr.remove_page_by_target_id(target_id);
            }
        }
        for (target_id, page_sid) in &drained.attached_page_sessions {
            if let Some(ref mut mgr) = self.browser {
                let should_update =
                    mgr.page_session_for_target(target_id)
                        .is_some_and(|current_sid| {
                            drained
                                .detached_page_sessions
                                .iter()
                                .any(|detached_sid| detached_sid == current_sid)
                        });
                if should_update && mgr.update_page_session(target_id, page_sid) {
                    let _ = mgr.enable_domains_pub(page_sid).await;
                }
            }
        }
        for (frame_id, iframe_sid) in &drained.attached_iframe_sessions {
            self.iframe_sessions
                .insert(frame_id.clone(), iframe_sid.clone());
            if let Some(ref mgr) = self.browser {
                let _ = mgr
                    .client
                    .send_command_no_params(
                        "Runtime.runIfWaitingForDebugger",
                        Some(iframe_sid.as_str()),
                    )
                    .await;
                let _ = mgr
                    .client
                    .send_command_no_params("DOM.enable", Some(iframe_sid.as_str()))
                    .await;
                let _ = mgr
                    .client
                    .send_command_no_params("Accessibility.enable", Some(iframe_sid.as_str()))
                    .await;
                if self.har_recording || self.request_tracking {
                    let _ = mgr
                        .client
                        .send_command_no_params("Network.enable", Some(iframe_sid.as_str()))
                        .await;
                }
            }
        }
        for sid in &drained.detached_iframe_sessions {
            self.iframe_sessions.retain(|_, v| v != sid);
        }
        for te in &drained.new_targets {
            if let Some(ref mut mgr) = self.browser {
                let attach_result: Result<AttachToTargetResult, String> = mgr
                    .client
                    .send_command_typed(
                        "Target.attachToTarget",
                        &AttachToTargetParams {
                            target_id: te.target_info.target_id.clone(),
                            flatten: true,
                        },
                        None,
                    )
                    .await;
                if let Ok(attach) = attach_result {
                    let _ = mgr.enable_domains_pub(&attach.session_id).await;
                    let df = self.domain_filter.read().await;
                    if let Some(ref filter) = *df {
                        let has_proxy_creds = self.proxy_credentials.read().await.is_some();
                        let _ = network::install_domain_filter(
                            &mgr.client,
                            &attach.session_id,
                            &filter.allowed_domains,
                            has_proxy_creds,
                        )
                        .await;
                    }
                    mgr.add_page_with_activation(
                        super::super::browser::PageInfo {
                            target_id: te.target_info.target_id.clone(),
                            session_id: attach.session_id,
                            url: te.target_info.url.clone(),
                            title: te.target_info.title.clone(),
                            target_type: te.target_info.target_type.clone(),
                        },
                        false,
                    );
                }
            }
        }
        for te in &drained.changed_targets {
            if let Some(ref mut mgr) = self.browser {
                mgr.update_page_target_info(&te.target_info);
            }
        }
        if debug_session_events_enabled() {
            if let Some(ref mgr) = self.browser {
                eprintln!(
                    "[agent-browser][sessions] after active={} pages={:?}",
                    mgr.active_session_id().unwrap_or("<none>"),
                    mgr.pages_list()
                        .iter()
                        .map(|p| format!("{} {} {}", p.target_id, p.session_id, p.url))
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    pub(crate) fn drain_cdp_events(&mut self) -> DrainedEvents {
        let rx = match self.event_rx.as_mut() {
            Some(rx) => rx,
            None => return DrainedEvents::default(),
        };
        let mut pending_acks: Vec<i64> = Vec::new();
        let mut new_targets: Vec<TargetCreatedEvent> = Vec::new();
        let mut new_target_ids: HashSet<String> = HashSet::new();
        let mut changed_targets: Vec<TargetInfoChangedEvent> = Vec::new();
        let mut destroyed_targets: Vec<String> = Vec::new();
        let mut attached_page_sessions: Vec<(String, String)> = Vec::new();
        let mut attached_iframe_sessions: Vec<(String, String)> = Vec::new();
        let mut detached_page_sessions: Vec<String> = Vec::new();
        let mut detached_iframe_sessions: Vec<String> = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    match event.method.as_str() {
                        "Target.targetCreated" => {
                            if let Ok(te) =
                                serde_json::from_value::<TargetCreatedEvent>(event.params.clone())
                            {
                                if should_track_target(&te.target_info) {
                                    let already_tracked = self
                                        .browser
                                        .as_ref()
                                        .is_none_or(|b| b.has_target(&te.target_info.target_id));
                                    if !already_tracked {
                                        new_target_ids.insert(te.target_info.target_id.clone());
                                        new_targets.push(te);
                                    }
                                }
                            }
                            continue;
                        }
                        "Target.targetInfoChanged" => {
                            if let Ok(te) = serde_json::from_value::<TargetInfoChangedEvent>(
                                event.params.clone(),
                            ) {
                                if should_track_target(&te.target_info) {
                                    let already_tracked = self
                                        .browser
                                        .as_ref()
                                        .is_some_and(|b| b.has_target(&te.target_info.target_id));
                                    if already_tracked
                                        || new_target_ids.contains(&te.target_info.target_id)
                                    {
                                        changed_targets.push(te);
                                    } else {
                                        new_target_ids.insert(te.target_info.target_id.clone());
                                        new_targets.push(TargetCreatedEvent {
                                            target_info: te.target_info,
                                        });
                                    }
                                }
                            }
                            continue;
                        }
                        "Target.targetDestroyed" => {
                            if let Ok(te) =
                                serde_json::from_value::<TargetDestroyedEvent>(event.params.clone())
                            {
                                destroyed_targets.push(te.target_id);
                            }
                            continue;
                        }
                        "Target.attachedToTarget" => {
                            if let (Some(sid), Some(target_info)) = (
                                event.params.get("sessionId").and_then(|v| v.as_str()),
                                event.params.get("targetInfo"),
                            ) {
                                let target_type = target_info
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if target_type == "iframe" {
                                    if let Some(target_id) =
                                        target_info.get("targetId").and_then(|v| v.as_str())
                                    {
                                        attached_iframe_sessions
                                            .push((target_id.to_string(), sid.to_string()));
                                    }
                                } else if matches!(target_type, "page" | "webview") {
                                    if let Some(target_id) =
                                        target_info.get("targetId").and_then(|v| v.as_str())
                                    {
                                        attached_page_sessions
                                            .push((target_id.to_string(), sid.to_string()));
                                    }
                                }
                            }
                            continue;
                        }
                        "Target.detachedFromTarget" => {
                            if let Some(sid) =
                                event.params.get("sessionId").and_then(|v| v.as_str())
                            {
                                let is_page_session = self.browser.as_ref().is_some_and(|b| {
                                    b.pages_list().iter().any(|p| p.session_id == sid)
                                });
                                if is_page_session {
                                    detached_page_sessions.push(sid.to_string());
                                } else {
                                    detached_iframe_sessions.push(sid.to_string());
                                }
                            }
                            continue;
                        }
                        _ => {}
                    }
                    let session_matches = if let Some(ref browser) = self.browser {
                        event.session_id.as_deref() == browser.active_session_id().ok()
                    } else {
                        false
                    };
                    let iframe_network_event = !session_matches
                        && (self.har_recording || self.request_tracking)
                        && event.method.starts_with("Network.")
                        && event
                            .session_id
                            .as_ref()
                            .is_some_and(|sid| self.iframe_sessions.values().any(|v| v == sid));
                    if !session_matches && !iframe_network_event {
                        continue;
                    }
                    match event.method.as_str() {
                        "Runtime.consoleAPICalled" => {
                            let level = event
                                .params
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("log");
                            let raw_args: Vec<Value> = event
                                .params
                                .get("args")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let text = network::format_console_args(&raw_args);
                            if let Some(ref server) = self.stream_server {
                                server.broadcast_console(level, &text, &raw_args);
                            }
                            self.event_tracker.add_console(level, &text, raw_args);
                        }
                        "Runtime.exceptionThrown" => {
                            if let Ok(ex_event) =
                                serde_json::from_value::<ExceptionThrownEvent>(event.params.clone())
                            {
                                let details = &ex_event.exception_details;
                                let text = details
                                    .exception
                                    .as_ref()
                                    .and_then(|e| e.description.as_deref())
                                    .unwrap_or(&details.text);
                                self.event_tracker.add_error(
                                    text,
                                    None,
                                    details.line_number,
                                    details.column_number,
                                );
                                if let Some(ref server) = self.stream_server {
                                    server.broadcast_page_error(
                                        text,
                                        details.line_number,
                                        details.column_number,
                                    );
                                }
                            }
                        }
                        "Network.requestWillBeSent"
                            if self.har_recording || self.request_tracking =>
                        {
                            if let Some(request) = event.params.get("request") {
                                let method = request
                                    .get("method")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("GET")
                                    .to_string();
                                let url = request
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let request_id = event
                                    .params
                                    .get("requestId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if self.har_recording {
                                    let wall_time = event
                                        .params
                                        .get("wallTime")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    let request_headers =
                                        har_extract_headers(request.get("headers"));
                                    let post_data = request
                                        .get("postData")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                    let request_body_size =
                                        post_data.as_ref().map(|s| s.len() as i64).unwrap_or(0);
                                    let resource_type = event
                                        .params
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Other")
                                        .to_string();
                                    self.har_entries.push(HarEntry {
                                        request_id: request_id.clone(),
                                        wall_time,
                                        method: method.clone(),
                                        url: url.clone(),
                                        request_headers,
                                        post_data,
                                        request_body_size,
                                        resource_type,
                                        status: None,
                                        status_text: String::new(),
                                        http_version: "HTTP/1.1".to_string(),
                                        response_headers: Vec::new(),
                                        mime_type: String::new(),
                                        redirect_url: String::new(),
                                        response_body_size: -1,
                                        cdp_timing: None,
                                        loading_finished_timestamp: None,
                                    });
                                }
                                if self.request_tracking {
                                    let headers =
                                        request.get("headers").cloned().unwrap_or(json!({}));
                                    let resource_type = event
                                        .params
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Other")
                                        .to_string();
                                    let timestamp = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_millis() as u64)
                                        .unwrap_or(0);
                                    self.tracked_requests.push(TrackedRequest {
                                        url,
                                        method,
                                        headers,
                                        timestamp,
                                        resource_type,
                                        request_id,
                                        post_data: request
                                            .get("postData")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                        status: None,
                                        response_headers: None,
                                        mime_type: None,
                                    });
                                }
                            }
                        }
                        "Network.responseReceived"
                            if self.har_recording || self.request_tracking =>
                        {
                            if let Some(response) = event.params.get("response") {
                                let request_id = event
                                    .params
                                    .get("requestId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let status = response.get("status").and_then(|v| v.as_i64());
                                let status_text = response
                                    .get("statusText")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let mime_type = response
                                    .get("mimeType")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let http_version = response
                                    .get("protocol")
                                    .and_then(|v| v.as_str())
                                    .map(har_cdp_protocol_to_http_version)
                                    .unwrap_or_else(|| "HTTP/1.1".to_string());
                                let response_headers = har_extract_headers(response.get("headers"));
                                let redirect_url = response_headers
                                    .iter()
                                    .find(|(k, _)| k.eq_ignore_ascii_case("location"))
                                    .map(|(_, v)| v.clone())
                                    .unwrap_or_default();
                                let encoded_data_length = response
                                    .get("encodedDataLength")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(-1);
                                let cdp_timing = response.get("timing").cloned();
                                if self.har_recording {
                                    if let Some(entry) = self
                                        .har_entries
                                        .iter_mut()
                                        .rev()
                                        .find(|e| e.request_id == request_id)
                                    {
                                        entry.status = status;
                                        entry.status_text = status_text;
                                        entry.mime_type = mime_type;
                                        entry.http_version = http_version;
                                        entry.response_headers = response_headers;
                                        entry.redirect_url = redirect_url;
                                        entry.response_body_size = encoded_data_length;
                                        entry.cdp_timing = cdp_timing;
                                    }
                                }
                                if self.request_tracking {
                                    let resp_headers = response.get("headers").cloned();
                                    let resp_mime = response
                                        .get("mimeType")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                    if let Some(entry) = self
                                        .tracked_requests
                                        .iter_mut()
                                        .rev()
                                        .find(|e| e.request_id == request_id)
                                    {
                                        entry.status = status;
                                        entry.mime_type = resp_mime;
                                        entry.response_headers = resp_headers;
                                    }
                                }
                            }
                        }
                        "Network.loadingFinished" if self.har_recording => {
                            let request_id = event
                                .params
                                .get("requestId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let timestamp = event.params.get("timestamp").and_then(|v| v.as_f64());
                            let encoded_data_length = event
                                .params
                                .get("encodedDataLength")
                                .and_then(|v| v.as_i64());
                            if let Some(entry) = self
                                .har_entries
                                .iter_mut()
                                .rev()
                                .find(|e| e.request_id == request_id)
                            {
                                if let Some(ts) = timestamp {
                                    entry.loading_finished_timestamp = Some(ts);
                                }
                                if let Some(len) = encoded_data_length {
                                    entry.response_body_size = len;
                                }
                            }
                        }
                        "Network.loadingFailed" if self.har_recording => {
                            let request_id = event
                                .params
                                .get("requestId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let timestamp = event.params.get("timestamp").and_then(|v| v.as_f64());
                            let error_text = event
                                .params
                                .get("errorText")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Failed");
                            if let Some(entry) = self
                                .har_entries
                                .iter_mut()
                                .rev()
                                .find(|e| e.request_id == request_id)
                            {
                                if entry.status.is_none() {
                                    entry.status = Some(0);
                                    entry.status_text = error_text.to_string();
                                }
                                if let Some(ts) = timestamp {
                                    entry.loading_finished_timestamp = Some(ts);
                                }
                            }
                        }
                        "Page.screencastFrame" if self.stream_server.is_none() => {
                            if let Some(sid) =
                                event.params.get("sessionId").and_then(|v| v.as_i64())
                            {
                                pending_acks.push(sid);
                            }
                        }
                        "Page.javascriptDialogOpening" => {
                            if let Ok(dialog_event) =
                                serde_json::from_value::<JavascriptDialogOpeningEvent>(
                                    event.params.clone(),
                                )
                            {
                                let auto_handled = self.auto_dialog
                                    && matches!(
                                        dialog_event.dialog_type.as_str(),
                                        "beforeunload" | "alert"
                                    );
                                if !auto_handled {
                                    self.pending_dialog = Some(PendingDialog {
                                        dialog_type: dialog_event.dialog_type,
                                        message: dialog_event.message,
                                        url: dialog_event.url,
                                        default_prompt: dialog_event.default_prompt,
                                    });
                                }
                            }
                        }
                        "Page.javascriptDialogClosed" => {
                            self.pending_dialog = None;
                        }
                        _ => {}
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    eprintln!(
                        "[agent-browser] Warning: CDP event buffer overflowed, {} events dropped. Network requests may be missing from HAR output.",
                        n
                    );
                    continue;
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    self.event_rx = None;
                    break;
                }
            }
        }
        DrainedEvents {
            pending_acks,
            new_targets,
            changed_targets,
            destroyed_targets,
            attached_page_sessions,
            attached_iframe_sessions,
            detached_page_sessions,
            detached_iframe_sessions,
        }
    }
}
pub(crate) fn runtime_profile_pid(runtime_profile: Option<&str>) -> Option<u32> {
    runtime_profile
        .and_then(|name| read_runtime_state(name).ok().flatten())
        .map(|state| state.browser_pid)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedRuntimeAttachTarget {
    pub(crate) runtime_profile: String,
    pub(crate) browser_pid: u32,
    pub(crate) cdp_port: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedProfileAttachTarget {
    pub(crate) browser_id: String,
    pub(crate) runtime_profile: String,
    pub(crate) cdp_endpoint: String,
    pub(crate) browser_pid: Option<u32>,
    pub(crate) owner_session_ids: Vec<String>,
}
pub(crate) fn managed_runtime_attach_target(
    runtime_profile: Option<&str>,
) -> Option<ManagedRuntimeAttachTarget> {
    let runtime_profile = runtime_profile?;
    let state = read_runtime_state(runtime_profile).ok().flatten()?;
    if !pid_is_running(state.browser_pid) {
        return None;
    }
    let cdp_port = state
        .devtools_port
        .or_else(|| read_devtools_port(std::path::Path::new(&state.user_data_dir)))?;
    Some(ManagedRuntimeAttachTarget {
        runtime_profile: runtime_profile.to_string(),
        browser_pid: state.browser_pid,
        cdp_port,
    })
}
pub(crate) fn can_attach_managed_runtime_for_launch(options: &LaunchOptions) -> bool {
    options.headless && !options.remote_headed
}
pub(crate) fn shared_profile_attach_target_for_auto_launch(
    metadata: &ServiceLaunchMetadata,
    command: &Value,
    session_id: &str,
) -> Option<SharedProfileAttachTarget> {
    let action = command.get("action").and_then(Value::as_str)?;
    if !matches!(action, "open" | "navigate" | "tab_new") {
        return None;
    }
    if command.get("browserId").is_some() || command.get("sessionName").is_some() {
        return None;
    }
    if allow_duplicate_profile_lane_from_command(command) {
        return None;
    }
    let profile_id = metadata.profile_id.as_deref()?;
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let requested_host = browser_host_from_command(command);
    let requested_display_isolation = remote_headed_display_isolation_from_command(command);
    let current_browser_id = service_browser_id(session_id);
    let mut candidates = service_state
        .browsers
        .values()
        .filter(|browser| browser.profile_id.as_deref() == Some(profile_id))
        .filter(|browser| service_browser_health_counts_as_live(browser.health))
        .filter(|browser| {
            requested_host.is_none_or(|host| {
                host == browser.host || host == ServiceBrowserHost::AttachedExisting
            })
        })
        .filter(|browser| {
            requested_display_isolation
                .as_deref()
                .is_none_or(|display_isolation| {
                    browser
                        .display_isolation
                        .as_deref()
                        .is_none_or(|owner_display_isolation| {
                            owner_display_isolation == display_isolation
                        })
                })
        })
        .filter_map(|browser| {
            browser
                .cdp_endpoint
                .as_deref()
                .map(str::trim)
                .filter(|endpoint| !endpoint.is_empty())
                .map(|endpoint| (browser, endpoint.to_string()))
        })
        .collect::<Vec<(&BrowserProcess, String)>>();
    candidates.sort_by(|left, right| {
        let left_current = left.0.id == current_browser_id;
        let right_current = right.0.id == current_browser_id;
        right_current
            .cmp(&left_current)
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let (browser, cdp_endpoint) = candidates.into_iter().next()?;
    Some(SharedProfileAttachTarget {
        browser_id: browser.id.clone(),
        runtime_profile: profile_id.to_string(),
        cdp_endpoint,
        browser_pid: browser.pid,
        owner_session_ids: browser.active_session_ids.clone(),
    })
}
pub(crate) fn retained_session_attach_target_for_auto_launch(
    command: &Value,
    session_id: &str,
) -> Option<SharedProfileAttachTarget> {
    let action = command.get("action").and_then(Value::as_str)?;
    if matches!(
        action,
        "launch" | "cdp_free_launch" | "open" | "navigate" | "tab_new"
    ) {
        return None;
    }
    if optional_command_string(command, "sessionName")
        .is_some_and(|requested| requested != session_id)
    {
        return None;
    }
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let requested_browser_id = optional_command_string(command, "browserId");
    let current_browser_id = service_browser_id(session_id);
    let mut candidates = service_state
        .browsers
        .values()
        .filter(|browser| service_browser_health_counts_as_live(browser.health))
        .filter(|browser| {
            browser.id == current_browser_id
                || browser
                    .active_session_ids
                    .iter()
                    .any(|owner_session_id| owner_session_id == session_id)
        })
        .filter(|browser| {
            requested_browser_id
                .as_deref()
                .is_none_or(|requested| requested == browser.id)
        })
        .filter_map(|browser| {
            let runtime_profile = browser.profile_id.as_deref()?.trim();
            let cdp_endpoint = browser.cdp_endpoint.as_deref()?.trim();
            if runtime_profile.is_empty() || cdp_endpoint.is_empty() {
                return None;
            }
            Some((
                browser,
                runtime_profile.to_string(),
                cdp_endpoint.to_string(),
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_current = left.0.id == current_browser_id;
        let right_current = right.0.id == current_browser_id;
        right_current
            .cmp(&left_current)
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let (browser, runtime_profile, cdp_endpoint) = candidates.into_iter().next()?;
    Some(SharedProfileAttachTarget {
        browser_id: browser.id.clone(),
        runtime_profile,
        cdp_endpoint,
        browser_pid: browser.pid,
        owner_session_ids: browser.active_session_ids.clone(),
    })
}
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
pub(crate) async fn terminate_runtime_browser(pid: u32) -> BrowserShutdownOutcome {
    tokio::task::spawn_blocking(move || {
        let mut outcome = BrowserShutdownOutcome::default();
        #[cfg(unix)]
        {
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !pid_is_running(pid) {
                    return outcome;
                }
            }
            outcome.polite_close_attempted = true;
            let term_rc = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            if term_rc != 0 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::ESRCH) {
                    outcome.polite_close_succeeded = true;
                    return outcome;
                }
                outcome.errors.push(format!(
                    "Failed to politely terminate runtime browser PID {}: {}",
                    pid, err
                ));
                outcome.polite_close_failed = true;
            }
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !pid_is_running(pid) {
                    outcome.polite_close_succeeded = true;
                    return outcome;
                }
            }
            outcome.polite_close_failed = true;
            outcome.force_kill_attempted = true;
            let kill_rc = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            if kill_rc != 0 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::ESRCH) {
                    outcome.force_kill_succeeded = true;
                    return outcome;
                }
                outcome.errors.push(format!(
                    "Failed to force kill runtime browser PID {}: {}",
                    pid, err
                ));
                outcome.force_kill_failed = true;
                return outcome;
            }
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !pid_is_running(pid) {
                    outcome.force_kill_succeeded = true;
                    return outcome;
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
            let status = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            match status {
                Ok(status) if status.success() => outcome.force_kill_succeeded = true,
                Ok(status) => outcome.errors.push(format!(
                    "taskkill failed for runtime browser PID {} with status {}",
                    pid, status
                )),
                Err(err) => outcome.errors.push(format!(
                    "Failed to start taskkill for runtime browser PID {}: {}",
                    pid, err
                )),
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
    super::super::browser::validate_launch_options(
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
    persist_service_browser_record(
        &state.session_id,
        ServiceBrowserHost::LocalHeaded,
        ServiceBrowserHealth::Ready,
        Some(launch.pid),
        None,
        None,
        Some(plan.metadata),
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
pub(crate) fn build_cdp_free_launch_plan(cmd: &Value) -> Result<CdpFreeLaunchPlan, String> {
    let url = optional_command_string(cmd, "url");
    if url.as_deref().is_some_and(|value| value.starts_with('-')) {
        return Err("cdp_free_launch url must not start with '-'".to_string());
    }
    let extensions: Option<Vec<String>> = cmd
        .get("extensions")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        });
    let mut args = cmd
        .get("args")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(url) = url.as_ref() {
        args.push(url.clone());
    }
    let mut launch_options = LaunchOptions {
        headless: false,
        executable_path: cmd
            .get("executablePath")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok()),
        proxy: cmd.get("proxy").and_then(|value| {
            value.as_str().map(str::to_string).or_else(|| {
                value
                    .get("server")
                    .and_then(|server| server.as_str())
                    .map(str::to_string)
            })
        }),
        proxy_bypass: cmd
            .get("proxy")
            .and_then(|value| value.get("bypass"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        proxy_username: cmd
            .get("proxy")
            .and_then(|value| value.get("username"))
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| env::var("AGENT_BROWSER_PROXY_USERNAME").ok()),
        proxy_password: cmd
            .get("proxy")
            .and_then(|value| value.get("password"))
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| env::var("AGENT_BROWSER_PROXY_PASSWORD").ok()),
        profile: launch_profile_from_sources(cmd, true),
        runtime_profile: runtime_profile_from_sources(cmd, true),
        expected_browser_family: cmd
            .get("runtimeProfileBrowserFamily")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        allow_file_access: cmd
            .get("allowFileAccess")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        args,
        extensions,
        storage_state: None,
        user_agent: cmd
            .get("userAgent")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        ignore_https_errors: cmd
            .get("ignoreHTTPSErrors")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        color_scheme: cmd
            .get("colorScheme")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        download_path: cmd
            .get("downloadPath")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        viewport_size: None,
        use_real_keychain: use_real_keychain_from_env(),
        keychain_password: keychain_password_from_env(),
        manual_login: true,
        attachable: false,
        display: None,
        remote_headed: false,
        remote_headed_display_isolation: None,
    };
    let selection_reason = apply_service_profile_selection(&mut launch_options, cmd);
    let browser_capability_launch =
        apply_service_browser_capability_selection(&mut launch_options, cmd);
    let mut metadata =
        ServiceLaunchMetadata::from_launch_options(&launch_options, Some(cmd), selection_reason);
    metadata.browser_capability_launch = Some(browser_capability_launch.to_value());
    Ok(CdpFreeLaunchPlan {
        launch_options,
        metadata,
        url,
    })
}
pub(crate) fn validate_cdp_free_launch_plan(plan: &CdpFreeLaunchPlan) -> Result<(), String> {
    super::super::browser::validate_launch_options(
        plan.launch_options.extensions.as_deref(),
        false,
        plan.launch_options.profile.as_deref(),
        None,
        plan.launch_options.allow_file_access,
        plan.launch_options.executable_path.as_deref(),
    )
}
pub(crate) fn cdp_free_launch_response(
    state: &DaemonState,
    launch_options: &LaunchOptions,
    launch: &ManualChromeLaunch,
    url: Option<String>,
) -> Value {
    const UNSUPPORTED_CDP_FREE_COMMANDS: &[&str] = &[
        "navigate",
        "back",
        "forward",
        "reload",
        "tab_new",
        "tab_switch",
        "tab_close",
        "view_focus",
        "tab_list",
        "url",
        "title",
        "viewport",
        "user_agent",
        "emulatemedia",
        "timezone",
        "locale",
        "geolocation",
        "permissions",
        "cookies_get",
        "cookies_set",
        "cookies_clear",
        "storage_get",
        "storage_set",
        "storage_clear",
        "console",
        "errors",
        "setcontent",
        "headers",
        "offline",
        "dialog",
        "clipboard",
        "upload",
        "download",
        "waitfordownload",
        "pdf",
        "responsebody",
        "har_start",
        "har_stop",
        "route",
        "unroute",
        "requests",
        "request_detail",
        "snapshot",
        "screenshot",
        "click",
        "fill",
        "wait",
        "type",
        "press",
        "hover",
        "select",
        "gettext",
        "inputvalue",
        "isvisible",
        "getattribute",
        "innerhtml",
        "styles",
        "count",
        "boundingbox",
        "isenabled",
        "ischecked",
        "check",
        "uncheck",
        "scroll",
        "scrollintoview",
        "focus",
        "clear",
    ];
    json!(
        { "launched" : true, "cdpFree" : true, "cdpAttachmentAllowed" : false,
        "browserId" : service_browser_id(& state.session_id), "browserPid" : launch.pid,
        "profileId" : service_profile_id(launch_options.profile.as_deref(),
        launch_options.runtime_profile.as_deref(),), "runtimeProfile" : launch
        .runtime_profile, "userDataDir" : launch.user_data_dir, "url" : url,
        "supportedOperations" : ["process_lifecycle", "profile_lease", "service_state",],
        "unsupportedOperations" : ["cdp_commands", "snapshot", "screenshot",
        "dom_interaction",], "unsupportedCommands" : UNSUPPORTED_CDP_FREE_COMMANDS, }
    )
}
pub(crate) async fn handle_cdp_attach(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    validate_cdp_attach_request(cmd, &state.session_id)?;
    let mgr = state.browser.as_mut().ok_or_else(|| {
        "Cannot attach CDP: target browser session is not running; request a service tab first"
            .to_string()
    })?;
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "cdp_attach requires serviceTabHandle".to_string())?;
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .or_else(|| cmd.get("targetId").and_then(Value::as_str))
        .ok_or_else(|| "cdp_attach requires serviceTabHandle.targetId".to_string())?;
    if mgr.active_target_id().ok() != Some(target_id) {
        let _ = mgr.tab_switch_target_id(target_id).await?;
    }
    let page_session_id = mgr.active_session_id()?.to_string();
    let attached_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let browser_id = service_browser_id(&state.session_id);
    let profile_id = handle.get("profileId").cloned().unwrap_or(Value::Null);
    let tab_id = handle
        .get("tabId")
        .cloned()
        .unwrap_or_else(|| json!(format!("target:{target_id}")));
    Ok(json!(
        { "attached" : true, "controlPlaneMode" : "cdp", "attachKind" :
        "service_tab_handle", "browserId" : browser_id, "sessionName" : state
        .session_id.clone(), "tabId" : tab_id, "targetId" : target_id,
        "pageSessionId" : page_session_id, "profileId" : profile_id.clone(),
        "profileOrigin" : handle.get("profileOrigin").cloned()
        .unwrap_or(Value::Null), "leaseId" : handle.get("leaseId").cloned()
        .unwrap_or(Value::Null), "leaseState" : handle.get("leaseState").cloned()
        .unwrap_or(Value::Null), "cleanupPolicy" : handle.get("cleanupPolicy")
        .cloned().unwrap_or(Value::Null), "browserWebSocketUrl" : mgr.get_cdp_url(),
        "cdpAttachmentAllowed" : true, "detachAction" : "cdp_detach",
        "detachRequired" : true, "closeBrowserOnDetach" : false,
        "browserProcessPreserved" : true, "traceFilter" : { "browserId" : browser_id,
        "profileId" : profile_id, "sessionId" : state.session_id.clone(), },
        "serviceTabHandle" : cmd.get("serviceTabHandle").cloned()
        .unwrap_or(Value::Null), "attachedAt" : attached_at, }
    ))
}
pub(crate) async fn handle_cdp_detach(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "cdp_detach requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let detached_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    Ok(json!(
        { "detached" : true, "controlPlaneMode" : "cdp", "detachKind" :
        "service_tab_handle", "browserId" : service_browser_id(& state.session_id),
        "sessionName" : state.session_id.clone(), "tabId" : handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "targetId" : handle.get("targetId")
        .cloned().unwrap_or(Value::Null), "profileId" : handle.get("profileId")
        .cloned().unwrap_or(Value::Null), "browserProcessPreserved" : true,
        "closeBrowserOnDetach" : false, "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "detachedAt" :
        detached_at, }
    ))
}
pub(crate) fn validate_cdp_attach_request(cmd: &Value, session_id: &str) -> Result<(), String> {
    if cmd.get("requiresCdpFree").and_then(Value::as_bool) == Some(true) {
        return Err(
            "cdp_attach is blocked because the selected policy requires CDP-free browser operation"
                .to_string(),
        );
    }
    if cmd.get("cdpAttachmentAllowed").and_then(Value::as_bool) != Some(true) {
        return Err(
            "cdp_attach requires cdpAttachmentAllowed=true from the access-plan decision"
                .to_string(),
        );
    }
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "cdp_attach requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, session_id)?;
    if handle.get("targetId").and_then(Value::as_str).is_none()
        && cmd.get("targetId").and_then(Value::as_str).is_none()
    {
        return Err("cdp_attach requires a CDP target id on the service tab handle".to_string());
    }
    Ok(())
}
pub(crate) fn validate_service_tab_handle_for_current_session(
    handle: &Map<String, Value>,
    session_id: &str,
) -> Result<(), String> {
    if handle.get("valid").and_then(Value::as_bool) != Some(true) {
        let stale_reason = handle
            .get("staleReason")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Err(format!("service tab handle is stale: {stale_reason}"));
    }
    validate_service_tab_handle_route_for_current_session(handle, session_id)
}
pub(crate) fn validate_service_tab_handle_route_for_current_session(
    handle: &Map<String, Value>,
    session_id: &str,
) -> Result<(), String> {
    let browser_id = handle
        .get("browserId")
        .and_then(Value::as_str)
        .ok_or_else(|| "serviceTabHandle.browserId is required".to_string())?;
    let expected_browser_id = service_browser_id(session_id);
    if browser_id != expected_browser_id && browser_id != format!("session:{session_id}") {
        return Err(format!(
            "service tab handle browserId {browser_id} does not match routed session {session_id}"
        ));
    }
    if let Some(handle_session_name) = handle.get("sessionName").and_then(Value::as_str) {
        if handle_session_name != session_id {
            return Err(
                format!(
                    "service tab handle sessionName {handle_session_name} does not match routed session {session_id}"
                ),
            );
        }
    }
    if handle.get("tabId").and_then(Value::as_str).is_none() {
        return Err("serviceTabHandle.tabId is required".to_string());
    }
    Ok(())
}
pub(crate) async fn launch_ios(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let device_name = cmd.get("deviceName").and_then(|v| v.as_str());
    let device_udid = cmd.get("udid").and_then(|v| v.as_str());
    let platform_version = cmd.get("platformVersion").and_then(|v| v.as_str());
    let device = ios::select_device(device_name, device_udid)?;
    if !device.is_real && device.state != "Booted" {
        ios::boot_simulator(&device.udid)?;
    }
    let mut appium = AppiumManager::connect_or_launch(Some(&device.udid)).await?;
    appium
        .create_ios_session(Some(&device.name), platform_version)
        .await?;
    if let Some(sid) = appium.client.session_id_pub().map(String::from) {
        let wd_client =
            super::super::webdriver::client::WebDriverClient::new_with_session(4723, sid);
        state.webdriver_backend = Some(WebDriverBackend::new(wd_client));
    }
    state.appium = Some(appium);
    state.backend_type = BackendType::WebDriver;
    state.engine = "safari".to_string();
    write_engine_file(&state.session_id, &state.engine);
    write_provider_file(&state.session_id, "ios");
    write_extensions_file(&state.session_id);
    state.reset_input_state();
    Ok(json!(
        { "launched" : true, "provider" : "ios", "device" : device.name, "udid" :
        device.udid, "backend" : "webdriver", }
    ))
}
pub(crate) async fn launch_safari(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let port: u16 = cmd
        .get("port")
        .and_then(|v| v.as_u64())
        .map(|p| p as u16)
        .unwrap_or(0);
    let driver_port = if port > 0 { port } else { 0 };
    let actual_port = if driver_port > 0 {
        driver_port
    } else {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to find free port: {}", e))?;
        listener
            .local_addr()
            .map_err(|e| format!("Failed to get local address: {}", e))?
            .port()
    };
    let driver = safari::launch_safaridriver(actual_port)?;
    let mut client = super::super::webdriver::client::WebDriverClient::new(actual_port);
    client
        .create_session(serde_json::json!({ "browserName" : "safari", }))
        .await?;
    state.safari_driver = Some(driver);
    state.webdriver_backend = Some(WebDriverBackend::new(client));
    state.backend_type = BackendType::WebDriver;
    state.engine = "safari".to_string();
    write_engine_file(&state.session_id, &state.engine);
    write_provider_file(&state.session_id, "safari");
    write_extensions_file(&state.session_id);
    state.reset_input_state();
    Ok(json!(
        { "launched" : true, "provider" : "safari", "port" : actual_port, "backend" :
        "webdriver", }
    ))
}
pub(crate) async fn handle_navigate(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let url = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' parameter")?;
    {
        let df = state.domain_filter.read().await;
        if let Some(ref filter) = *df {
            filter.check_url(url)?;
        }
    }
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            state.ref_map.clear();
            cancellable(wb.navigate(url), cancellation.clone()).await?;
            let new_url = cancellable(wb.get_url(), cancellation.clone())
                .await
                .unwrap_or_else(|_| url.to_string());
            let title = cancellable(wb.get_title(), cancellation.clone())
                .await
                .unwrap_or_default();
            let mut data = json!({ "url" : new_url, "title" : title });
            add_manual_login_hint_warning(cmd, &mut data);
            return Ok(data);
        }
    }
    let pending_shared_profile_acquisition = state.pending_shared_profile_acquisition.take();
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let wait_until = cmd
        .get("waitUntil")
        .and_then(|v| v.as_str())
        .map(WaitUntil::from_str)
        .unwrap_or(WaitUntil::Load);
    let scoped_headers = cmd
        .get("headers")
        .and_then(|v| v.as_object())
        .filter(|m| !m.is_empty());
    if let Some(headers_map) = scoped_headers {
        if let Some(origin) = url::Url::parse(url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
        {
            let headers: HashMap<String, String> = headers_map
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
            let first_origin_header = {
                let mut map = state.origin_headers.write().await;
                let first = map.is_empty();
                map.insert(origin, headers);
                first
            };
            if first_origin_header {
                let session_id = mgr.active_session_id()?.to_string();
                let has_proxy_creds = state.proxy_credentials.read().await.is_some();
                let mut params = json!({ "patterns" : [{ "urlPattern" : "*" }] });
                if has_proxy_creds {
                    params["handleAuthRequests"] = json!(true);
                }
                cancellable(
                    mgr.client
                        .send_command("Fetch.enable", Some(params), Some(&session_id)),
                    cancellation.clone(),
                )
                .await?;
            }
        }
    }
    state.ref_map.clear();
    state.iframe_sessions.clear();
    state.active_frame_id = None;
    let mut data = cancellable(mgr.navigate(url, wait_until), cancellation).await?;
    if let (Some(object), Some(shared_acquisition)) = (
        data.as_object_mut(),
        pending_shared_profile_acquisition.as_ref(),
    ) {
        object.insert("sharedAcquisition".to_string(), shared_acquisition.clone());
    }
    add_manual_login_hint_warning(cmd, &mut data);
    persist_service_owned_navigate_tab(cmd, &state.session_id, mgr, &data)?;
    Ok(data)
}
pub(crate) fn read_runtime_handoff(session_name: &str) -> Result<RuntimeHandoffDescriptor, String> {
    let path = runtime_handoff_path(session_name);
    let payload = fs::read(&path).map_err(|error| {
        format!(
            "No prepared runtime handoff is available for session '{}': {}",
            session_name, error
        )
    })?;
    serde_json::from_slice(&payload).map_err(|error| {
        format!(
            "Runtime handoff for session '{}' is invalid: {}",
            session_name, error
        )
    })
}
pub(crate) fn current_service_browser_host(session_name: &str) -> ServiceBrowserHost {
    LockedServiceStateRepository::default_json()
        .ok()
        .and_then(|repository| repository.load_snapshot().ok())
        .and_then(|service_state| {
            service_state
                .browsers
                .get(&service_browser_id(session_name))
                .map(|browser| browser.host)
        })
        .unwrap_or(ServiceBrowserHost::AttachedExisting)
}
pub(crate) async fn handle_runtime_handoff_prepare(
    state: &mut DaemonState,
) -> Result<Value, String> {
    let Some(manager) = state.browser.as_mut() else {
        let path = runtime_handoff_path(&state.session_id);
        let _ = fs::remove_file(path);
        return Ok(json!(
            { "prepared" : false, "browserPresent" : false, "sessionName" : state
            .session_id, }
        ));
    };
    if !manager.is_connection_alive().await {
        return Err(format!(
            "Cannot prepare runtime handoff for session '{}': browser CDP connection is not alive",
            state.session_id
        ));
    }
    let descriptor = RuntimeHandoffDescriptor {
        schema_version: 1,
        session_name: state.session_id.clone(),
        cdp_url: manager.get_cdp_url().to_string(),
        browser_pid: manager.browser_pid().or(state.attached_browser_pid),
        runtime_profile: manager
            .runtime_profile_name()
            .map(str::to_string)
            .or_else(|| state.attached_runtime_profile.clone()),
        engine: state.engine.clone(),
        host: current_service_browser_host(&state.session_id),
        close_browser_on_close: state.close_behavior == CloseBehavior::CloseBrowser,
        active_target_id: manager.active_target_id().ok().map(str::to_string),
        prepared_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string()),
    };
    let path = write_runtime_handoff(&descriptor)?;
    manager.relinquish_browser_for_handoff();
    state.browser = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    Ok(json!(
        { "prepared" : true, "browserPresent" : true, "sessionName" : descriptor
        .session_name, "browserPid" : descriptor.browser_pid, "cdpUrl" : descriptor
        .cdp_url, "runtimeProfile" : descriptor.runtime_profile, "handoffPath" :
        path, }
    ))
}
pub(crate) async fn handle_runtime_handoff_resume(
    state: &mut DaemonState,
) -> Result<Value, String> {
    if state.browser.is_some() {
        return Err(format!(
            "Cannot resume runtime handoff for session '{}': daemon already has a browser",
            state.session_id
        ));
    }
    let descriptor = read_runtime_handoff(&state.session_id)?;
    if descriptor.schema_version != 1 || descriptor.session_name != state.session_id {
        return Err(format!(
            "Runtime handoff identity mismatch for session '{}'",
            state.session_id
        ));
    }
    if descriptor
        .browser_pid
        .is_some_and(|browser_pid| !pid_is_running(browser_pid))
    {
        return Err(format!(
            "Runtime handoff browser PID is no longer running for session '{}'",
            state.session_id
        ));
    }
    let manager = BrowserManager::connect_cdp_for_handoff(
        &descriptor.cdp_url,
        descriptor.active_target_id.as_deref(),
    )
    .await?;
    state.reset_input_state();
    state.attached_runtime_profile = descriptor.runtime_profile.clone();
    state.attached_browser_pid = descriptor.browser_pid;
    state.close_behavior = if descriptor.close_browser_on_close {
        CloseBehavior::CloseBrowser
    } else {
        CloseBehavior::Detach
    };
    state.engine = descriptor.engine.clone();
    write_engine_file(&state.session_id, &state.engine);
    state.browser = Some(manager);
    state.subscribe_to_browser_events();
    state.start_fetch_handler();
    state.start_dialog_handler();
    state.update_stream_client().await;
    persist_current_browser_health(state, descriptor.host, ServiceBrowserHealth::Ready, None);
    let retry_record_removed = fs::remove_file(runtime_handoff_path(&state.session_id)).is_ok();
    Ok(json!(
        { "resumed" : true, "sessionName" : descriptor.session_name, "browserPid" :
        descriptor.browser_pid, "cdpUrl" : descriptor.cdp_url, "runtimeProfile" :
        descriptor.runtime_profile, "activeTargetId" : state.browser.as_ref()
        .and_then(| browser | browser.active_target_id().ok()), "retryRecordRemoved"
        : retry_record_removed, "targetsReattached" : state.browser.as_ref()
        .map(BrowserManager::page_count).unwrap_or(0), }
    ))
}
pub(crate) async fn handle_close(state: &mut DaemonState) -> Result<Value, String> {
    let attached_runtime_profile = state.attached_runtime_profile.take();
    let attached_browser_pid = state.attached_browser_pid.take();
    let close_behavior = std::mem::take(&mut state.close_behavior);
    let mut shutdown_outcome = BrowserShutdownOutcome::default();
    if let Some(ref mgr) = state.browser {
        if let Some(ref session_name) = state.session_name {
            if let Ok(session_id) = mgr.active_session_id() {
                let tracked_origins = state
                    .tracked_origin_storage
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                let _ = state::save_state(
                    &mgr.client,
                    session_id,
                    None,
                    Some(session_name.as_str()),
                    &state.session_id,
                    mgr.visited_origins(),
                    &tracked_origins,
                )
                .await;
            }
        }
    }
    if let Some(ref mut mgr) = state.browser {
        let runtime_profile = mgr.runtime_profile_name().map(str::to_string);
        if (attached_runtime_profile.is_some() || attached_browser_pid.is_some())
            && close_behavior == CloseBehavior::CloseBrowser
        {
            let _ = mgr
                .client
                .send_command_no_params("Browser.close", None)
                .await;
        }
        if close_behavior == CloseBehavior::Detach && runtime_profile.is_some() {
            mgr.detach_runtime_browser()?;
        } else {
            let outcome = mgr.close_with_outcome().await?;
            shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
            shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
            shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
            shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
            shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
            shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
            shutdown_outcome.errors.extend(outcome.errors);
            if let Some(runtime_profile) = runtime_profile {
                let _ = clear_runtime_state(&runtime_profile);
            }
        }
    }
    if let Some(runtime_profile) = attached_runtime_profile {
        if close_behavior == CloseBehavior::CloseBrowser {
            let pid = attached_browser_pid.or_else(|| runtime_profile_pid(Some(&runtime_profile)));
            if let Some(pid) = pid {
                let outcome = terminate_runtime_browser(pid).await;
                shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
                shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
                shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
                shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
                shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
                shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
                shutdown_outcome.errors.extend(outcome.errors);
            }
            let _ = clear_runtime_state(&runtime_profile);
        }
    } else if close_behavior == CloseBehavior::CloseBrowser {
        if let Some(pid) = attached_browser_pid {
            let outcome = terminate_runtime_browser(pid).await;
            shutdown_outcome.polite_close_attempted |= outcome.polite_close_attempted;
            shutdown_outcome.polite_close_succeeded |= outcome.polite_close_succeeded;
            shutdown_outcome.polite_close_failed |= outcome.polite_close_failed;
            shutdown_outcome.force_kill_attempted |= outcome.force_kill_attempted;
            shutdown_outcome.force_kill_succeeded |= outcome.force_kill_succeeded;
            shutdown_outcome.force_kill_failed |= outcome.force_kill_failed;
            shutdown_outcome.errors.extend(outcome.errors);
        }
    }
    state.browser = None;
    state.launch_hash = None;
    state.screencasting = false;
    state.reset_input_state();
    state.update_stream_client().await;
    persist_closed_browser_health(state, Some(&shutdown_outcome));
    if let Some(task) = state.fetch_handler_task.take() {
        task.abort();
    }
    {
        let mut map = state.origin_headers.write().await;
        map.clear();
    }
    if let Some(ref mut wb) = state.webdriver_backend {
        let _ = wb.close().await;
    }
    state.webdriver_backend = None;
    if let Some(ref mut appium) = state.appium {
        let _ = appium.close().await;
    }
    state.appium = None;
    if let Some(ref mut driver) = state.safari_driver {
        driver.kill();
    }
    state.safari_driver = None;
    state.backend_type = BackendType::Cdp;
    if let Some(server) = state.inspect_server.take() {
        server.shutdown();
    }
    state.ref_map.clear();
    Ok(json!({ "closed" : true }))
}
pub(crate) async fn handle_snapshot(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let options = SnapshotOptions {
        selector: cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .map(String::from),
        interactive: cmd
            .get("interactive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        compact: cmd
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        depth: cmd
            .get("maxDepth")
            .and_then(|v| v.as_u64())
            .map(|d| d as usize),
        urls: cmd.get("urls").and_then(|v| v.as_bool()).unwrap_or(false),
    };
    state.ref_map.clear();
    let tree = cancellable(
        snapshot::take_snapshot(
            &mgr.client,
            &session_id,
            &options,
            &mut state.ref_map,
            state.active_frame_id.as_deref(),
            &state.iframe_sessions,
        ),
        cancellation.clone(),
    )
    .await?;
    let url = cancellable(mgr.get_url(), cancellation)
        .await
        .unwrap_or_default();
    let refs: serde_json::Map<String, Value> = state
        .ref_map
        .entries_sorted()
        .into_iter()
        .map(|(ref_id, entry)| {
            let mut obj = serde_json::Map::new();
            obj.insert("role".into(), Value::String(entry.role));
            obj.insert("name".into(), Value::String(entry.name));
            (ref_id, Value::Object(obj))
        })
        .collect();
    Ok(json!({ "snapshot" : tree, "origin" : url, "refs" : refs }))
}
