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
    optional_command_string, remote_headed_display_isolation, CdpFreeLaunchPlan,
};
use super::daemon::{
    apply_service_browser_capability_selection, apply_service_profile_selection,
    keychain_password_from_env, launch_profile_from_sources, runtime_profile_from_sources,
    use_real_keychain_from_env, BackendType,
};
use super::recovery::DaemonState;
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
