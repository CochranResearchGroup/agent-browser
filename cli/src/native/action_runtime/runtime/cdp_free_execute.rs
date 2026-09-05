#![allow(unused_imports)]
use super::capability::service_browser_id;
use super::cdp_free_plan::{
    apply_launch_host_hints, optional_command_string, remote_headed_display_isolation,
    CdpFreeLaunchPlan,
};
use super::daemon::{
    apply_service_browser_capability_selection, apply_service_profile_selection,
    keychain_password_from_env, launch_profile_from_sources, runtime_profile_from_sources,
    use_real_keychain_from_env, BackendType,
};
use super::recovery::DaemonState;
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
use crate::native::network::resolve_fetch_paused;
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::service_lifecycle::{
    profile_lease_telemetry, select_service_profile_for_request, service_profile_id,
    ProfileSelectionRequest, ServiceLaunchMetadata,
};
use crate::native::service_profile_access_policy::{
    evaluate_profile_child_access, ProfileChildAccess, ProfileChildAccessRequest,
    ProfileIdentityAssurance, ProfilePermission, ServiceProfileAccessPolicy,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use crate::native::webdriver::appium::AppiumManager;
use crate::native::webdriver::backend::{
    BrowserBackend, WebDriverBackend, WEBDRIVER_UNSUPPORTED_ACTIONS,
};
use crate::native::webdriver::ios;
use crate::native::webdriver::safari;
use serde_json::{json, Map, Value};
use std::env;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) fn build_cdp_free_launch_plan(
    cmd: &Value,
    effective_session: Option<&str>,
) -> Result<CdpFreeLaunchPlan, String> {
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
    // CDP-free is a control-plane posture, not a request to abandon an
    // explicitly allocated remote-headed display or its operator view stream.
    let service_host = apply_launch_host_hints(&mut launch_options, cmd);
    let selection_reason =
        apply_service_profile_selection(&mut launch_options, cmd, effective_session)?;
    let browser_capability_launch =
        apply_service_browser_capability_selection(&mut launch_options, cmd);
    let mut metadata =
        ServiceLaunchMetadata::from_launch_options(&launch_options, Some(cmd), selection_reason);
    metadata.browser_capability_launch = Some(browser_capability_launch.to_value());
    Ok(CdpFreeLaunchPlan {
        launch_options,
        metadata,
        service_host,
        url,
    })
}
pub(crate) fn validate_cdp_free_launch_plan(plan: &CdpFreeLaunchPlan) -> Result<(), String> {
    super::super::super::browser::validate_launch_options(
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
    validate_cdp_attach_request(cmd, state)?;
    let browser_id = service_tab_handle_browser_id(state);
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
    validate_service_tab_handle_for_daemon(handle, cmd, state)?;
    let detached_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    Ok(json!(
        { "detached" : true, "controlPlaneMode" : "cdp", "detachKind" :
        "service_tab_handle", "browserId" : service_tab_handle_browser_id(state),
        "sessionName" : state.session_id.clone(), "tabId" : handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "targetId" : handle.get("targetId")
        .cloned().unwrap_or(Value::Null), "profileId" : handle.get("profileId")
        .cloned().unwrap_or(Value::Null), "browserProcessPreserved" : true,
        "closeBrowserOnDetach" : false, "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "detachedAt" :
        detached_at, }
    ))
}
pub(crate) fn validate_cdp_attach_request(cmd: &Value, state: &DaemonState) -> Result<(), String> {
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
    validate_service_tab_handle_for_daemon(handle, cmd, state)?;
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
    validate_service_tab_handle_route(handle, session_id, None)
}
pub(crate) fn validate_service_tab_handle_route_for_current_session(
    handle: &Map<String, Value>,
    session_id: &str,
) -> Result<(), String> {
    validate_service_tab_handle_route(handle, session_id, None)
}
pub(crate) fn service_tab_handle_browser_id(state: &DaemonState) -> String {
    state
        .runtime_owner_binding
        .as_ref()
        .map(|binding| binding.claim.logical_browser_id.trim())
        .filter(|browser_id| !browser_id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| service_browser_id(&state.session_id))
}
pub(crate) fn validate_service_tab_handle_for_daemon(
    handle: &Map<String, Value>,
    cmd: &Value,
    state: &DaemonState,
) -> Result<Option<ProfileChildAccess>, String> {
    let action = cmd
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let stale_error = (handle.get("valid").and_then(Value::as_bool) != Some(true)).then(|| {
        let stale_reason = handle
            .get("staleReason")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        format!("service tab handle is stale: {stale_reason}")
    });
    let browser_id = service_tab_handle_browser_id(state);
    validate_service_tab_handle_route(handle, &state.session_id, Some(&browser_id))?;
    let access = authorize_profile_child_access(handle, cmd)?;
    if action != "tab_handle_refresh" {
        if let Some(error) = stale_error {
            return Err(error);
        }
    }
    Ok(access)
}

fn authorize_profile_child_access(
    handle: &Map<String, Value>,
    cmd: &Value,
) -> Result<Option<ProfileChildAccess>, String> {
    let Some(tab_id) = handle.get("tabId").and_then(Value::as_str) else {
        return Err("serviceTabHandle.tabId is required".to_string());
    };
    let Some(connection_instance_id) = cmd
        .get("connectionInstanceId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        if handle
            .get("profileAccess")
            .is_some_and(|value| !value.is_null())
        {
            return Err("profile child access requires a service-generated connection".to_string());
        }
        return Ok(None);
    };
    let authenticated_subject = cmd
        .get("servicePrincipalId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let subject_id = authenticated_subject.or_else(|| {
        cmd.get("clientSubjectId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
    });
    let assurance = if authenticated_subject.is_some() {
        ProfileIdentityAssurance::RegisteredCapability
    } else if subject_id.is_some() {
        ProfileIdentityAssurance::SelfDeclared
    } else {
        ProfileIdentityAssurance::Unknown
    };
    let action = cmd
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let permission = match action {
        "diagnostics" | "probe" | "network_capture" => ProfilePermission::TabObserve,
        "tab_handle_release" => ProfilePermission::TabCloseOwn,
        _ => ProfilePermission::TabControlOwn,
    };
    let repository = LockedServiceStateRepository::default_json()?;
    repository.mutate(|service_state| {
        authorize_profile_child_access_in_state(
            service_state,
            handle,
            tab_id,
            subject_id,
            assurance,
            connection_instance_id,
            permission,
        )
    })
}

fn authorize_profile_child_access_in_state(
    service_state: &mut crate::native::service_model::ServiceState,
    handle: &Map<String, Value>,
    tab_id: &str,
    subject_id: Option<&str>,
    assurance: ProfileIdentityAssurance,
    connection_instance_id: &str,
    permission: ProfilePermission,
) -> Result<Option<ProfileChildAccess>, String> {
    let Some(tab) = service_state.tabs.get(tab_id) else {
        if handle
            .get("profileAccess")
            .is_some_and(|value| !value.is_null())
        {
            return Err("profile child access record is missing".to_string());
        }
        return Ok(None);
    };
    let Some(child) = tab.profile_access.clone() else {
        if handle
            .get("profileAccess")
            .is_some_and(|value| !value.is_null())
        {
            return Err("profile child access record is missing".to_string());
        }
        return Ok(None);
    };
    // One-shot HTTP and MCP requests receive a fresh service-generated
    // connection. A disconnected child may therefore reconnect as part of
    // its next authorized operation. An active child remains exclusive to
    // its current connection and cannot be stolen by matching labels.
    let reconnect = child.connection_state
        == crate::native::service_profile_access_policy::ProfileConnectionState::Disconnected;
    let profile_id = tab
        .owner_session_id
        .as_deref()
        .or(tab.session_id.as_deref())
        .and_then(|session_id| service_state.sessions.get(session_id))
        .and_then(|session| session.profile_id.as_deref())
        .or_else(|| {
            service_state
                .browsers
                .get(&tab.browser_id)
                .and_then(|browser| browser.profile_id.as_deref())
        })
        .unwrap_or("unselected");
    let policy = service_state
        .profiles
        .get(profile_id)
        .and_then(|profile| profile.access_policy.clone())
        .unwrap_or_else(|| ServiceProfileAccessPolicy::shared_local_default(profile_id));
    let result = evaluate_profile_child_access(ProfileChildAccessRequest {
        child: &child,
        current_policy: &policy,
        subject_id,
        assurance,
        connection_instance_id,
        permission,
        reconnect,
    });
    if !result.allowed {
        // Keep the exact reason: Service recourse classifies these pre-effect
        // authority denials without changing this ownership decision.
        return Err(format!("profile child access denied: {}", result.reason));
    }
    if result.reconnected {
        if let Some(tab) = service_state.tabs.get_mut(tab_id) {
            tab.profile_access = Some(result.child.clone());
        }
        service_state.refresh_service_tab_handles();
    }
    Ok(Some(result.child))
}
pub(crate) fn validate_service_tab_handle_route_for_daemon(
    handle: &Map<String, Value>,
    state: &DaemonState,
) -> Result<(), String> {
    let browser_id = service_tab_handle_browser_id(state);
    validate_service_tab_handle_route(handle, &state.session_id, Some(&browser_id))
}
fn validate_service_tab_handle_route(
    handle: &Map<String, Value>,
    session_id: &str,
    authorized_browser_id: Option<&str>,
) -> Result<(), String> {
    let browser_id = handle
        .get("browserId")
        .and_then(Value::as_str)
        .ok_or_else(|| "serviceTabHandle.browserId is required".to_string())?;
    let expected_browser_id = service_browser_id(session_id);
    if browser_id != expected_browser_id
        && browser_id != format!("session:{session_id}")
        && authorized_browser_id != Some(browser_id)
    {
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
            super::super::super::webdriver::client::WebDriverClient::new_with_session(4723, sid);
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
    let mut client = super::super::super::webdriver::client::WebDriverClient::new(actual_port);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, BrowserTab, ServiceState,
    };
    use crate::native::service_profile_access_policy::{
        ProfileConnectionState, PROFILE_CHILD_ACCESS_SCHEMA_V1,
    };
    use crate::runtime_owner_transfer::{OwnerAuthorityClaim, RuntimeOwnerBinding};
    use std::collections::BTreeMap;

    fn retained_owner_state() -> DaemonState {
        let mut state = DaemonState::new();
        state.session_id = "handoff-owner-route".to_string();
        state.runtime_owner_binding =
            Some(RuntimeOwnerBinding::effect_capable(OwnerAuthorityClaim {
                owner_id: "owner-1".to_string(),
                profile_identity_digest: "profile-digest".to_string(),
                owner_generation: 19,
                logical_browser_id: "session:durable-browser".to_string(),
                daemon_session_route: "handoff-owner-route".to_string(),
                process_instance_digest: "process-digest".to_string(),
            }));
        state
    }

    #[test]
    fn retained_owner_authorizes_durable_service_tab_handle_identity() {
        let state = retained_owner_state();
        let handle = json!({
            "browserId": "session:durable-browser",
            "sessionName": "handoff-owner-route",
            "tabId": "target:tab-1",
            "targetId": "tab-1",
            "valid": true,
        });

        assert_eq!(
            service_tab_handle_browser_id(&state),
            "session:durable-browser"
        );
        validate_service_tab_handle_for_daemon(
            handle.as_object().expect("handle object"),
            &json!({"action": "cdp_attach"}),
            &state,
        )
        .expect("durable owner browser id should be authorized");
    }

    #[test]
    fn retained_owner_rejects_unrelated_service_tab_handle_identity() {
        let state = retained_owner_state();
        let handle = json!({
            "browserId": "session:unrelated-browser",
            "sessionName": "handoff-owner-route",
            "tabId": "target:tab-1",
            "targetId": "tab-1",
            "valid": true,
        });

        let error = validate_service_tab_handle_for_daemon(
            handle.as_object().expect("handle object"),
            &json!({"action": "cdp_attach"}),
            &state,
        )
        .expect_err("unrelated browser id must fail closed");
        assert!(error.contains("does not match routed session"));
    }

    #[test]
    fn attributed_tab_access_enforces_connection_subject_and_reconnect() {
        let profile_id = "research-gov";
        let browser_id = "session:shared-browser";
        let session_id = "shared-session";
        let tab_id = "target:fieldwork-tab";
        let child = ProfileChildAccess {
            schema_version: PROFILE_CHILD_ACCESS_SCHEMA_V1.to_string(),
            parent_policy_revision: 1,
            access_decision_id: "decision:fieldwork".to_string(),
            subject_id: Some("client:fieldwork".to_string()),
            identity_assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: Some("connection:owner".to_string()),
            connection_state: ProfileConnectionState::Active,
            permissions: vec![
                ProfilePermission::TabObserve,
                ProfilePermission::TabControlOwn,
                ProfilePermission::TabCloseOwn,
            ],
        };
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                profile_id.to_string(),
                BrowserProfile {
                    id: profile_id.to_string(),
                    access_policy: Some(ServiceProfileAccessPolicy::shared_local_default(
                        profile_id,
                    )),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                browser_id.to_string(),
                BrowserProcess {
                    id: browser_id.to_string(),
                    profile_id: Some(profile_id.to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                session_id.to_string(),
                BrowserSession {
                    id: session_id.to_string(),
                    profile_id: Some(profile_id.to_string()),
                    browser_ids: vec![browser_id.to_string()],
                    tab_ids: vec![tab_id.to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([(
                tab_id.to_string(),
                BrowserTab {
                    id: tab_id.to_string(),
                    browser_id: browser_id.to_string(),
                    owner_session_id: Some(session_id.to_string()),
                    profile_access: Some(child),
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        };
        let handle = json!({
            "tabId": tab_id,
            "profileAccess": {"subjectId": "client:fieldwork"}
        });
        let handle = handle.as_object().expect("handle object");

        let active_owner_error = authorize_profile_child_access_in_state(
            &mut state,
            handle,
            tab_id,
            Some("client:fieldwork"),
            ProfileIdentityAssurance::SelfDeclared,
            "connection:other",
            ProfilePermission::TabControlOwn,
        )
        .expect_err("matching labels cannot steal an active child");
        assert!(active_owner_error.contains("owner_connection_still_active"));

        assert_eq!(
            state.mark_profile_connection_disconnected("connection:owner"),
            1
        );
        let reconnected = authorize_profile_child_access_in_state(
            &mut state,
            handle,
            tab_id,
            Some("client:fieldwork"),
            ProfileIdentityAssurance::SelfDeclared,
            "connection:other",
            ProfilePermission::TabControlOwn,
        )
        .expect("stable subject should reconnect a disconnected child")
        .expect("attributed child should be returned");
        assert_eq!(
            reconnected.connection_instance_id.as_deref(),
            Some("connection:other")
        );

        authorize_profile_child_access_in_state(
            &mut state,
            handle,
            tab_id,
            Some("client:fieldwork"),
            ProfileIdentityAssurance::SelfDeclared,
            "connection:other",
            ProfilePermission::TabCloseOwn,
        )
        .expect("the owner connection may close its own tab");
        let before_denial = serde_json::to_value(&state).unwrap();
        let wrong_subject_error = authorize_profile_child_access_in_state(
            &mut state,
            handle,
            tab_id,
            Some("client:other"),
            ProfileIdentityAssurance::SelfDeclared,
            "connection:other",
            ProfilePermission::TabObserve,
        )
        .expect_err("a different subject cannot use the child");
        assert!(wrong_subject_error.contains("subject_mismatch"));
        assert_eq!(serde_json::to_value(&state).unwrap(), before_denial);
        let failure =
            crate::native::service_failure::classify_service_failure(&wrong_subject_error);
        assert_eq!(failure.code, "profile_child_subject_mismatch");
        assert_eq!(
            failure.effect_state,
            crate::native::service_failure::ServiceEffectState::NoEffect
        );
        assert_eq!(failure.recommended_action, "use_own_service_tab_handle");
    }
}
