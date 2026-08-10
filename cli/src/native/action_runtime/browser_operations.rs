#![allow(unused_imports)]
use super::common::*;
use super::runtime::{
    is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
    relaunch_and_restore_page, service_browser_id, validate_service_tab_handle_for_current_session,
    validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
    HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
    AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
    AUTH_LOGIN_WAIT_UNTIL,
};
use super::service_workflows::truncate_utf8;
pub(crate) fn persist_service_owned_navigate_tab(
    cmd: &Value,
    session_id: &str,
    mgr: &BrowserManager,
    data: &Value,
) -> Result<(), String> {
    if optional_command_string(cmd, "serviceName").is_none()
        && optional_command_string(cmd, "agentName").is_none()
        && optional_command_string(cmd, "taskName").is_none()
    {
        return Ok(());
    }
    let Ok(target_id) = mgr.active_target_id() else {
        return Ok(());
    };
    let url = data
        .get("url")
        .and_then(Value::as_str)
        .or_else(|| mgr.active_page_url());
    let title = data.get("title").and_then(Value::as_str);
    let tab_id = format!("target:{target_id}");
    let profile_id = mgr
        .runtime_profile_name()
        .map(|profile| Value::String(profile.to_string()))
        .unwrap_or(Value::Null);
    let service_tab_handle = json!(
        { "browserId" : service_browser_id(session_id), "sessionName" : session_id,
        "tabId" : tab_id, "targetId" : target_id, "url" : url, "title" : title,
        "profileId" : profile_id.clone(), "profileOrigin" : "agent_browser_owned",
        "leaseId" : session_id, "leaseState" : "shared", "cleanupPolicy" : "detach",
        "leaseHeartbeatExpected" : true, "ownerSessionId" : session_id, "jobId" :
        Value::Null, "traceFilter" : { "browserId" : service_browser_id(session_id),
        "profileId" : profile_id, "sessionId" : session_id, "serviceName" :
        optional_command_string(cmd, "serviceName"), "agentName" :
        optional_command_string(cmd, "agentName"), "taskName" :
        optional_command_string(cmd, "taskName"), }, "valid" : true, "staleReason" :
        Value::Null, }
    );
    persist_service_owned_tab_new(
        cmd,
        session_id,
        Some(target_id),
        url,
        title,
        &service_tab_handle,
    )
}
pub(crate) fn add_manual_login_hint_warning(cmd: &Value, data: &mut Value) {
    let Some(service) = cmd
        .get("manualLoginPreferredService")
        .and_then(|v| v.as_str())
    else {
        return;
    };
    if let Some(obj) = data.as_object_mut() {
        obj.insert(
            "_warning".to_string(),
            json!(
                format!("Service hint: '{}' prefers manual login. If sign-in is blocked or required, use `agent-browser runtime login <url>` for detached sign-in, or `agent-browser runtime login <url> --attachable` followed by `agent-browser runtime attach` to bind automation to the live manual browser.",
                service)
            ),
        );
    }
}
pub(crate) fn take_response_warning(data: &mut Value) -> Option<String> {
    data.as_object_mut()
        .and_then(|obj| obj.remove("_warning"))
        .and_then(|v| v.as_str().map(str::to_string))
}
pub(crate) async fn handle_url(state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            let url = wb.get_url().await?;
            return Ok(json!({ "url" : url }));
        }
    }
    let desired_url = state
        .browser
        .as_ref()
        .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
    let first_result = {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        mgr.get_url().await
    };
    let url = match first_result {
        Ok(url) => url,
        Err(err) if is_stale_page_session_error(&err) => {
            let recovered = {
                let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                recover_browser_command_channel(mgr, &err).await
            };
            if recovered.is_err() {
                relaunch_and_restore_page(state, desired_url).await?;
            }
            state
                .browser
                .as_mut()
                .ok_or("Browser not launched")?
                .get_url()
                .await?
        }
        Err(err) => return Err(err),
    };
    Ok(json!({ "url" : url }))
}
pub(crate) fn handle_cdp_url(state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    Ok(json!({ "cdpUrl" : mgr.get_cdp_url() }))
}
pub(crate) async fn handle_inspect(state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    if let Some(server) = state.inspect_server.take() {
        server.shutdown();
    }
    let target_id = mgr.active_target_id()?.to_string();
    let chrome_hp = mgr.chrome_host_port().to_string();
    let proxy_handle = mgr.client.inspect_handle();
    let server = InspectServer::start(proxy_handle, target_id, chrome_hp).await?;
    let url = format!("http://127.0.0.1:{}", server.port());
    open_url_in_browser(&url);
    state.inspect_server = Some(server);
    Ok(json!({ "opened" : true, "url" : url }))
}
pub(crate) fn open_url_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/c", "start", "", url])
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let result: Result<std::process::Child, std::io::Error> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unsupported platform",
    ));
    if let Err(e) = result {
        let _ = writeln!(std::io::stderr(), "[inspect] Failed to open browser: {}", e);
    }
}
pub(crate) async fn handle_title(state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            let title = wb.get_title().await?;
            return Ok(json!({ "title" : title }));
        }
    }
    let desired_url = state
        .browser
        .as_ref()
        .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
    let first_result = {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        mgr.get_title().await
    };
    let title = match first_result {
        Ok(title) => title,
        Err(err) if is_stale_page_session_error(&err) => {
            let recovered = {
                let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                recover_browser_command_channel(mgr, &err).await
            };
            if recovered.is_err() {
                relaunch_and_restore_page(state, desired_url).await?;
            }
            state
                .browser
                .as_mut()
                .ok_or("Browser not launched")?
                .get_title()
                .await?
        }
        Err(err) => return Err(err),
    };
    Ok(json!({ "title" : title }))
}
pub(crate) async fn handle_content(state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            let html = wb.get_content().await?;
            let url = wb.get_url().await.unwrap_or_default();
            return Ok(json!({ "html" : html, "origin" : url }));
        }
    }
    let desired_url = state
        .browser
        .as_ref()
        .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
    let first_result = {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        mgr.get_content().await
    };
    let html = match first_result {
        Ok(html) => html,
        Err(err) if is_stale_page_session_error(&err) => {
            let recovered = {
                let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                recover_browser_command_channel(mgr, &err).await
            };
            if recovered.is_err() {
                relaunch_and_restore_page(state, desired_url).await?;
            }
            state
                .browser
                .as_mut()
                .ok_or("Browser not launched")?
                .get_content()
                .await?
        }
        Err(err) => return Err(err),
    };
    let url = state
        .browser
        .as_mut()
        .ok_or("Browser not launched")?
        .get_url()
        .await
        .unwrap_or_default();
    Ok(json!({ "html" : html, "origin" : url }))
}
pub(crate) fn command_evaluation_timeout_ms(cmd: &Value) -> Option<u64> {
    cmd.get("jobTimeoutMs")
        .and_then(Value::as_u64)
        .filter(|timeout_ms| *timeout_ms > 0)
}
pub(crate) async fn handle_evaluate(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    if cmd.get("serviceTabHandle").is_some() {
        return handle_bounded_service_evaluate(cmd, state).await;
    }
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            let script = cmd
                .get("script")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'script' parameter")?;
            let result = wb.evaluate(script).await?;
            let url = wb.get_url().await.unwrap_or_default();
            return Ok(json!({ "result" : result, "origin" : url }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let script = cmd
        .get("script")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'script' parameter")?;
    let result = if let Some(timeout_ms) = command_evaluation_timeout_ms(cmd) {
        mgr.evaluate_with_timeout(script, timeout_ms).await?
    } else {
        mgr.evaluate(script, None).await?
    };
    let url = mgr.active_page_url().unwrap_or_default().to_string();
    Ok(json!({ "result" : result, "origin" : url }))
}
pub(crate) async fn handle_bounded_service_evaluate(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "evaluate requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    if cmd.get("returnByValue").and_then(Value::as_bool) == Some(false) {
        return Err("evaluate requires returnByValue=true so results can be capped".to_string());
    }
    let script = cmd
        .get("script")
        .or_else(|| cmd.get("expression"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "evaluate requires script or expression".to_string())?;
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .ok_or_else(|| "evaluate requires positive timeoutMs".to_string())?;
    let max_return_bytes = cmd
        .get("maxReturnBytes")
        .and_then(Value::as_u64)
        .ok_or_else(|| "evaluate requires positive maxReturnBytes".to_string())?;
    let mgr = state.browser.as_mut().ok_or_else(|| {
        "Cannot evaluate: target browser session is not running; request a service tab first"
            .to_string()
    })?;
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "evaluate requires serviceTabHandle.targetId".to_string())?;
    if mgr.active_target_id().ok() != Some(target_id) {
        let _ = mgr.tab_switch_target_id(target_id).await?;
    }
    let started_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let evaluate_outcome = tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms),
        mgr.evaluate_with_timeout(script, timeout_ms),
    )
    .await;
    let url = mgr.active_page_url().unwrap_or_default().to_string();
    let title = mgr.active_page_title().unwrap_or_default().to_string();
    let result = match evaluate_outcome {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            return Ok(json!(
                { "ok" : false, "action" : "evaluate", "errorKind" : "exception",
                "error" : error, "timeoutMs" : timeout_ms, "maxReturnBytes" :
                max_return_bytes, "url" : url, "title" : title, "targetId" :
                target_id, "tabId" : handle.get("tabId").cloned()
                .unwrap_or(Value::Null), "profileId" : handle.get("profileId")
                .cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
                .get("serviceTabHandle").cloned().unwrap_or(Value::Null),
                "evaluatedAt" : started_at, }
            ));
        }
        Err(_) => {
            return Ok(json!(
                { "ok" : false, "action" : "evaluate", "errorKind" : "timeout",
                "error" : format!("evaluate timed out after {timeout_ms}ms"),
                "timeoutMs" : timeout_ms, "maxReturnBytes" : max_return_bytes, "url"
                : url, "title" : title, "targetId" : target_id, "tabId" : handle
                .get("tabId").cloned().unwrap_or(Value::Null), "profileId" : handle
                .get("profileId").cloned().unwrap_or(Value::Null), "serviceTabHandle"
                : cmd.get("serviceTabHandle").cloned().unwrap_or(Value::Null),
                "evaluatedAt" : started_at, }
            ));
        }
    };
    let serialized = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
    let serialized_len = serialized.len() as u64;
    let truncated = serialized_len > max_return_bytes;
    let returned = if truncated {
        Value::String(truncate_utf8(&serialized, max_return_bytes as usize))
    } else {
        result
    };
    Ok(json!(
        { "ok" : true, "action" : "evaluate", "result" : returned, "resultTruncated"
        : truncated, "resultBytes" : serialized_len, "maxReturnBytes" :
        max_return_bytes, "timeoutMs" : timeout_ms, "returnByValue" : true, "url" :
        url, "title" : title, "targetId" : target_id, "tabId" : handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "profileId" : handle.get("profileId")
        .cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "evaluatedAt" :
        started_at, }
    ))
}
pub(crate) async fn handle_screenshot(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let cancellation = state.current_cancellation.clone();
    let annotate = cmd
        .get("annotate")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            if annotate {
                return Err(
                    "Annotated screenshots are not yet implemented on the WebDriver backend"
                        .to_string(),
                );
            }
            let base64_data = cancellable(wb.screenshot(), cancellation.clone()).await?;
            let path = cmd.get("path").and_then(|v| v.as_str());
            if let Some(p) = path {
                let bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &base64_data,
                )
                .map_err(|e| format!("Base64 decode error: {}", e))?;
                std::fs::write(p, bytes)
                    .map_err(|e| format!("Failed to write screenshot: {}", e))?;
                return Ok(json!({ "path" : p }));
            }
            let tmp = format!(
                "/tmp/screenshot-{}.png",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
            let bytes =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_data)
                    .map_err(|e| format!("Base64 decode error: {}", e))?;
            std::fs::write(&tmp, bytes)
                .map_err(|e| format!("Failed to write screenshot: {}", e))?;
            return Ok(json!({ "path" : tmp }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let format = cmd
        .get("format")
        .or_else(|| cmd.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("png")
        .to_string();
    let options = ScreenshotOptions {
        selector: cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .map(String::from),
        path: cmd.get("path").and_then(|v| v.as_str()).map(String::from),
        full_page: cmd
            .get("fullPage")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        format,
        quality: cmd
            .get("quality")
            .and_then(|v| v.as_i64())
            .map(|q| q as i32),
        annotate,
        output_dir: cmd
            .get("screenshotDir")
            .and_then(|v| v.as_str())
            .map(String::from),
    };
    if annotate {
        state.ref_map.clear();
        let _ = cancellable(
            snapshot::take_snapshot(
                &mgr.client,
                &session_id,
                &SnapshotOptions {
                    interactive: true,
                    ..SnapshotOptions::default()
                },
                &mut state.ref_map,
                state.active_frame_id.as_deref(),
                &state.iframe_sessions,
            ),
            cancellation.clone(),
        )
        .await?;
    }
    let result = match cancellable(
        screenshot::take_screenshot(
            &mgr.client,
            &session_id,
            &state.ref_map,
            &options,
            &state.iframe_sessions,
        ),
        cancellation.clone(),
    )
    .await
    {
        Ok(result) => result,
        Err(err)
            if err.contains("CDP response channel closed")
                || err.contains("Trying to work with closed connection")
                || err.contains("Session with given id not found")
                || err.contains("No session with given id") =>
        {
            let desired_url = state
                .browser
                .as_ref()
                .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            if recover_browser_command_channel(mgr, &err).await.is_err() {
                relaunch_and_restore_page(state, desired_url).await?;
            }
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            let fresh_session_id = mgr.active_session_id()?.to_string();
            cancellable(
                screenshot::take_screenshot(
                    &mgr.client,
                    &fresh_session_id,
                    &state.ref_map,
                    &options,
                    &state.iframe_sessions,
                ),
                cancellation,
            )
            .await?
        }
        Err(err) => return Err(err),
    };
    let mut response = json!({ "path" : result.path });
    if !result.annotations.is_empty() {
        response["annotations"] = serde_json::to_value(&result.annotations)
            .map_err(|e| format!("Failed to serialize annotations: {}", e))?;
    }
    Ok(response)
}
pub(crate) async fn handle_click(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let capture_clipboard_write = cmd
        .get("captureClipboardWrite")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !capture_clipboard_write {
        return handle_click_action(cmd, state).await;
    }
    let (client, session_id) = {
        let mgr = state
            .browser
            .as_ref()
            .ok_or("--capture-clipboard-write requires the native Chrome browser backend")?;
        (mgr.client.clone(), mgr.active_session_id()?.to_string())
    };
    let action = handle_click_action(cmd, state);
    let action_timeout = cmd
        .get("jobTimeoutMs")
        .and_then(|value| value.as_u64())
        .map(|timeout_ms| {
            tokio::time::Duration::from_millis(timeout_ms.saturating_sub(1000).max(1))
        })
        .unwrap_or(super::super::clipboard::DEFAULT_WRITE_CAPTURE_ACTION_TIMEOUT);
    let (action_result, capture) = super::super::clipboard::capture_write_during(
        &client,
        &session_id,
        super::super::clipboard::DEFAULT_WRITE_CAPTURE_LIMIT,
        action_timeout,
        action,
    )
    .await?;
    match action_result {
        Ok(mut response) => {
            response["clipboardCapture"] = json!(
                { "supported" : capture.supported, "invoked" : capture.invoked, "text" :
                capture.text, "truncated" : capture.truncated, "originalLength" : capture
                .original_length, "restored" : capture.restored, "reason" : capture
                .reason, }
            );
            Ok(response)
        }
        Err(error) => Err(format!(
            "{error}; clipboardCaptureRestored={}",
            capture.restored
        )),
    }
}
pub(crate) async fn handle_click_action(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            wb.click(selector).await?;
            return Ok(json!({ "clicked" : selector }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let client = mgr.client.clone();
    let session_id = mgr.active_session_id()?.to_string();
    let new_tab = cmd.get("newTab").and_then(|v| v.as_bool()).unwrap_or(false);
    if new_tab {
        use super::super::element::resolve_element_object_id;
        let (object_id, effective_session_id) = resolve_element_object_id(
            &client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        let call_params = json!(
            { "objectId" : object_id, "functionDeclaration" :
            "function() { var h = this.getAttribute('href'); if (!h) return null; try { return new URL(h, document.baseURI).toString(); } catch(e) { return null; } }",
            "returnByValue" : true }
        );
        let call_result = client
            .send_command(
                "Runtime.callFunctionOn",
                Some(call_params),
                Some(&effective_session_id),
            )
            .await?;
        let href = call_result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                format!(
                    "Element '{}' does not have an href attribute. --new-tab only works on links.",
                    selector
                )
            })?
            .to_string();
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        state.ref_map.clear();
        mgr.tab_new(Some(&href)).await?;
        return Ok(json!({ "clicked" : selector, "newTab" : true, "url" : href }));
    }
    let button = cmd.get("button").and_then(|v| v.as_str()).unwrap_or("left");
    let click_count = cmd.get("clickCount").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    if button == "left" && click_count == 1 {
        if let Some(ref_id) = super::super::element::parse_ref(selector) {
            if let Some(entry) = state.ref_map.get(&ref_id) {
                if entry.role == "link" {
                    let nth = entry.nth.unwrap_or(0);
                    let link_lookup: Value = client
                        .send_command_typed(
                            "Runtime.evaluate",
                            &super::super::cdp::types::EvaluateParams {
                                expression: format!(
                                    r#"(function() {{
                                        const targetName = {name};
                                        const targetIndex = {nth};
                                        const links = Array.from(document.querySelectorAll('a[href]'))
                                            .filter((el) => {{
                                                const rect = el.getBoundingClientRect();
                                                const style = window.getComputedStyle(el);
                                                if (rect.width <= 0 || rect.height <= 0) return false;
                                                if (style.display === 'none' || style.visibility === 'hidden' || Number.parseFloat(style.opacity || '1') === 0) return false;
                                                const label = (el.getAttribute('aria-label') || el.getAttribute('title') || el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
                                                return label === targetName;
                                            }});
                                        const el = links[targetIndex];
                                        return el ? el.href : null;
                                    }})()"#,
                                    name = serde_json::to_string(& entry.name).unwrap_or_else(|
                                    _ | "\"\"".to_string()), nth = nth,
                                ),
                                return_by_value: Some(true),
                                await_promise: Some(true),
                            },
                            Some(&session_id),
                        )
                        .await
                        .ok()
                        .and_then(|r: super::super::cdp::types::EvaluateResult| {
                            r.result.value
                        })
                        .unwrap_or(Value::Null);
                    if let Some(href) = link_lookup.as_str() {
                        if let Some(mgr) = state.browser.as_mut() {
                            mgr.set_active_page_url(href);
                        }
                        let _ = client
                            .send_command_typed::<_, super::super::cdp::types::EvaluateResult>(
                                "Runtime.evaluate",
                                &super::super::cdp::types::EvaluateParams {
                                    expression: format!(
                                        "window.location.assign({});",
                                        serde_json::to_string(href)
                                            .unwrap_or_else(|_| "\"\"".to_string())
                                    ),
                                    return_by_value: Some(true),
                                    await_promise: Some(false),
                                },
                                Some(&session_id),
                            )
                            .await;
                        return Ok(json!(
                            { "clicked" : selector, "url" : href, "fallbackNavigation" :
                            true }
                        ));
                    }
                }
            }
        }
        use super::super::element::resolve_element_object_id;
        let (object_id, effective_session_id) = resolve_element_object_id(
            &client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        let call_result = client
            .send_command(
                "Runtime.callFunctionOn",
                Some(json!(
                    { "objectId" : object_id, "functionDeclaration" :
                    r#"function() {
                        const el = this.closest?.('a[href]') || this;
                        if (!el || !el.href) return null;
                        const href = String(el.href);
                        const target = el.getAttribute('target') || '';
                        if (target && target !== '_self') return null;
                        if (href.startsWith('javascript:')) return null;
                        return href;
                    }"#,
                    "returnByValue" : true }
                )),
                Some(&effective_session_id),
            )
            .await?;
        if let Some(href) = call_result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
        {
            interaction::focus(
                &client,
                &session_id,
                &state.ref_map,
                selector,
                &state.iframe_sessions,
            )
            .await?;
            let press_client = client.clone();
            let press_session_id = session_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;
                let _ = interaction::press_key(&press_client, &press_session_id, "Enter").await;
            });
            return Ok(json!(
                { "clicked" : selector, "url" : href, "deferredActivation" : true }
            ));
        }
    }
    interaction::click(
        &client,
        &session_id,
        &state.ref_map,
        selector,
        button,
        click_count,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "clicked" : selector }))
}
pub(crate) async fn handle_dblclick(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::dblclick(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "clicked" : selector }))
}
pub(crate) async fn handle_fill(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let value = cmd
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'value' parameter")?;
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            wb.fill(selector, value).await?;
            return Ok(json!({ "filled" : selector }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    interaction::fill(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        value,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "filled" : selector }))
}
pub(crate) async fn handle_type(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let text = cmd
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'text' parameter")?;
    let clear = cmd.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
    let delay = cmd.get("delay").and_then(|v| v.as_u64());
    interaction::type_text(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        text,
        clear,
        delay,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "typed" : text }))
}
pub(crate) async fn handle_press(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let key = cmd
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'key' parameter")?;
    let (actual_key, modifiers) = parse_key_chord(key);
    interaction::press_key_with_modifiers(&mgr.client, &session_id, &actual_key, modifiers).await?;
    Ok(json!({ "pressed" : key }))
}
/// Parse a key chord string like "Control+a" or "Control+Shift+Enter" into
/// the actual key name and an optional CDP modifier bitmask.
///
/// CDP modifier values: 1 = Alt, 2 = Control, 4 = Meta (Cmd), 8 = Shift.
pub(crate) fn parse_key_chord(input: &str) -> (String, Option<i32>) {
    let parts: Vec<&str> = input.split('+').collect();
    if parts.len() < 2 {
        return (input.to_string(), None);
    }
    let mut modifiers = 0i32;
    let mut key_parts: Vec<&str> = Vec::new();
    for part in &parts {
        match part.to_lowercase().as_str() {
            "alt" => modifiers |= 1,
            "control" | "ctrl" => modifiers |= 2,
            "meta" | "cmd" | "command" => modifiers |= 4,
            "shift" => modifiers |= 8,
            _ => key_parts.push(part),
        }
    }
    if modifiers == 0 {
        return (input.to_string(), None);
    }
    let actual_key = if key_parts.is_empty() {
        input.to_string()
    } else {
        key_parts.join("+")
    };
    (actual_key, Some(modifiers))
}
pub(crate) async fn handle_hover(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::hover(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "hovered" : selector }))
}
pub(crate) async fn handle_scroll(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd.get("selector").and_then(|v| v.as_str());
    let (mut dx, mut dy) = (
        cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
        cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
    );
    if let Some(direction) = cmd.get("direction").and_then(|v| v.as_str()) {
        let amount = cmd.get("amount").and_then(|v| v.as_f64()).unwrap_or(300.0);
        match direction {
            "up" => dy = -amount,
            "down" => dy = amount,
            "left" => dx = -amount,
            "right" => dx = amount,
            _ => {}
        }
    }
    interaction::scroll(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        dx,
        dy,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "scrolled" : true }))
}
pub(crate) async fn handle_select(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let values: Vec<String> = match cmd.get("values") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => cmd
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
    };
    interaction::select_option(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &values,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "selected" : values }))
}
pub(crate) async fn handle_check(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::check(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "checked" : selector }))
}
pub(crate) async fn handle_uncheck(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::uncheck(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "unchecked" : selector }))
}
pub(crate) async fn handle_wait(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let timeout_ms = state.timeout_ms(cmd);
    if let Some(text) = cmd.get("text").and_then(|v| v.as_str()) {
        wait_for_text(&mgr.client, &session_id, text, timeout_ms).await?;
        return Ok(json!({ "waited" : "text", "text" : text }));
    }
    if let Some(selector) = cmd.get("selector").and_then(|v| v.as_str()) {
        let state_str = cmd
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("visible");
        wait_for_selector(&mgr.client, &session_id, selector, state_str, timeout_ms).await?;
        return Ok(json!({ "waited" : "selector", "selector" : selector }));
    }
    if let Some(url_pattern) = cmd.get("url").and_then(|v| v.as_str()) {
        wait_for_url(&mgr.client, &session_id, url_pattern, timeout_ms).await?;
        return Ok(json!({ "waited" : "url", "url" : url_pattern }));
    }
    if let Some(fn_str) = cmd.get("function").and_then(|v| v.as_str()) {
        wait_for_function(&mgr.client, &session_id, fn_str, timeout_ms).await?;
        return Ok(json!({ "waited" : "function" }));
    }
    if let Some(load_state) = cmd.get("loadState").and_then(|v| v.as_str()) {
        let wait_until = WaitUntil::from_str(load_state);
        mgr.wait_for_lifecycle_external(wait_until, &session_id)
            .await?;
        return Ok(json!({ "waited" : "load", "state" : load_state }));
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(timeout_ms)).await;
    Ok(json!({ "waited" : "timeout", "ms" : timeout_ms }))
}
pub(crate) async fn handle_gettext(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let text = super::super::element::get_element_text(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    let url = mgr.get_url().await.unwrap_or_default();
    Ok(json!({ "text" : text, "origin" : url }))
}
pub(crate) async fn handle_getattribute(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let attribute = cmd
        .get("attribute")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'attribute' parameter")?;
    let value = super::super::element::get_element_attribute(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        attribute,
        &state.iframe_sessions,
    )
    .await?;
    let url = mgr.get_url().await.unwrap_or_default();
    Ok(json!({ "value" : value, "origin" : url }))
}
pub(crate) async fn handle_isvisible(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let visible = super::super::element::is_element_visible(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    let url = mgr.get_url().await.unwrap_or_default();
    Ok(json!({ "visible" : visible, "origin" : url }))
}
pub(crate) async fn handle_isenabled(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let enabled = super::super::element::is_element_enabled(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    let url = mgr.get_url().await.unwrap_or_default();
    Ok(json!({ "enabled" : enabled, "origin" : url }))
}
pub(crate) async fn handle_ischecked(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let checked = super::super::element::is_element_checked(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    let url = mgr.get_url().await.unwrap_or_default();
    Ok(json!({ "checked" : checked, "origin" : url }))
}
pub(crate) async fn handle_back(state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            wb.back().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let url = wb.get_url().await.unwrap_or_default();
            state.ref_map.clear();
            return Ok(json!({ "url" : url }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    mgr.evaluate("history.back()", None).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let url = mgr.get_url().await.unwrap_or_default();
    state.ref_map.clear();
    Ok(json!({ "url" : url }))
}
pub(crate) async fn handle_forward(state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            wb.forward().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let url = wb.get_url().await.unwrap_or_default();
            state.ref_map.clear();
            return Ok(json!({ "url" : url }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    mgr.evaluate("history.forward()", None).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let url = mgr.get_url().await.unwrap_or_default();
    state.ref_map.clear();
    Ok(json!({ "url" : url }))
}
pub(crate) async fn handle_reload(state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            wb.reload().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            let url = wb.get_url().await.unwrap_or_default();
            state.ref_map.clear();
            return Ok(json!({ "url" : url }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    mgr.client
        .send_command_no_params("Page.reload", Some(&session_id))
        .await?;
    let mut rx = mgr.client.subscribe();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event.method == "Page.loadEventFired"
                        && event.session_id.as_deref() == Some(&session_id)
                    {
                        return;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(_) => break,
            }
        }
    })
    .await;
    let url = mgr.get_url().await.unwrap_or_default();
    state.ref_map.clear();
    Ok(json!({ "url" : url }))
}
pub(crate) async fn wait_for_selector(
    client: &super::super::cdp::client::CdpClient,
    session_id: &str,
    selector: &str,
    state: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let check_fn = match state {
        "attached" => {
            format!(
                "!!document.querySelector({})",
                serde_json::to_string(selector).unwrap_or_default()
            )
        }
        "detached" => {
            format!(
                "!document.querySelector({})",
                serde_json::to_string(selector).unwrap_or_default()
            )
        }
        "hidden" => {
            format!(
                r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return true;
                const s = window.getComputedStyle(el);
                return s.display === 'none' || s.visibility === 'hidden' || parseFloat(s.opacity) === 0;
            }})()"#,
                sel = serde_json::to_string(selector).unwrap_or_default()
            )
        }
        _ => {
            format!(
                r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return false;
                const r = el.getBoundingClientRect();
                const s = window.getComputedStyle(el);
                return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none';
            }})()"#,
                sel = serde_json::to_string(selector).unwrap_or_default()
            )
        }
    };
    poll_until_true(client, session_id, &check_fn, timeout_ms).await
}
pub(crate) async fn wait_for_url(
    client: &super::super::cdp::client::CdpClient,
    session_id: &str,
    pattern: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let check_fn = format!(
        "location.href.includes({})",
        serde_json::to_string(pattern).unwrap_or_default()
    );
    poll_until_true(client, session_id, &check_fn, timeout_ms).await
}
pub(crate) async fn wait_for_text(
    client: &super::super::cdp::client::CdpClient,
    session_id: &str,
    text: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let check_fn = format!(
        "(document.body.innerText || '').includes({})",
        serde_json::to_string(text).unwrap_or_default()
    );
    poll_until_true(client, session_id, &check_fn, timeout_ms).await
}
pub(crate) async fn wait_for_function(
    client: &super::super::cdp::client::CdpClient,
    session_id: &str,
    fn_str: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let check_fn = format!("!!({})", fn_str);
    poll_until_true(client, session_id, &check_fn, timeout_ms).await
}
pub(crate) async fn poll_until_true(
    client: &super::super::cdp::client::CdpClient,
    session_id: &str,
    expression: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    loop {
        let result: super::super::cdp::types::EvaluateResult = client
            .send_command_typed(
                "Runtime.evaluate",
                &super::super::cdp::types::EvaluateParams {
                    expression: expression.to_string(),
                    return_by_value: Some(true),
                    await_promise: Some(true),
                },
                Some(session_id),
            )
            .await?;
        if result
            .result
            .value
            .as_ref()
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Wait timed out after {}ms", timeout_ms));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
pub(crate) async fn handle_cookies_get(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    if let Some(ref wb) = state.webdriver_backend {
        if state.browser.is_none() {
            let cookies_list = wb.get_cookies().await?;
            return Ok(json!({ "cookies" : cookies_list }));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let urls = cmd.get("urls").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });
    let cookies_list = cookies::get_cookies(&mgr.client, &session_id, urls).await?;
    Ok(json!({ "cookies" : cookies_list }))
}
pub(crate) async fn handle_cookies_set(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let url = mgr.get_url().await.ok();
    let cookie_values = if let Some(arr) = cmd.get("cookies").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        let mut cookie = serde_json::Map::new();
        for key in &[
            "name", "value", "domain", "path", "expires", "httpOnly", "secure", "sameSite", "url",
        ] {
            if let Some(v) = cmd.get(*key) {
                if !v.is_null() {
                    cookie.insert(key.to_string(), v.clone());
                }
            }
        }
        vec![Value::Object(cookie)]
    };
    cookies::set_cookies(&mgr.client, &session_id, cookie_values, url.as_deref()).await?;
    Ok(json!({ "set" : true }))
}
pub(crate) async fn handle_cookies_clear(state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    cookies::clear_cookies(&mgr.client, &session_id).await?;
    Ok(json!({ "cleared" : true }))
}
pub(crate) async fn handle_storage_get(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let storage_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("local");
    let key = cmd.get("key").and_then(|v| v.as_str());
    storage::storage_get(&mgr.client, &session_id, storage_type, key).await
}
pub(crate) async fn handle_storage_set(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let storage_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("local");
    let key = cmd
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'key' parameter")?;
    let value = cmd
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'value' parameter")?;
    storage::storage_set(&mgr.client, &session_id, storage_type, key, value).await?;
    let current_url = mgr.get_url().await.unwrap_or_default();
    if let Ok(parsed) = url::Url::parse(&current_url) {
        let origin = parsed.origin().ascii_serialization();
        if origin != "null" {
            let tracked = state
                .tracked_origin_storage
                .entry(origin.clone())
                .or_insert_with(|| state::OriginStorage {
                    origin,
                    local_storage: Vec::new(),
                    session_storage: Vec::new(),
                });
            let entries = if storage_type == "session" {
                &mut tracked.session_storage
            } else {
                &mut tracked.local_storage
            };
            if let Some(existing) = entries.iter_mut().find(|entry| entry.name == key) {
                existing.value = value.to_string();
            } else {
                entries.push(state::StorageEntry {
                    name: key.to_string(),
                    value: value.to_string(),
                });
            }
        }
    }
    Ok(json!({ "set" : true }))
}
pub(crate) async fn handle_storage_clear(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let storage_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("local");
    storage::storage_clear(&mgr.client, &session_id, storage_type).await?;
    Ok(json!({ "cleared" : true }))
}
pub(crate) async fn handle_setcontent(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let html = cmd
        .get("html")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'html' parameter")?;
    network::set_content(&mgr.client, &session_id, html).await?;
    Ok(json!({ "set" : true }))
}
pub(crate) async fn handle_headers(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let headers_value = cmd.get("headers").ok_or("Missing 'headers' parameter")?;
    let headers: HashMap<String, String> = headers_value
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        })
        .unwrap_or_default();
    network::set_extra_headers(&mgr.client, &session_id, &headers).await?;
    Ok(json!({ "set" : true }))
}
pub(crate) async fn handle_offline(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let offline = cmd.get("offline").and_then(|v| v.as_bool()).unwrap_or(true);
    network::set_offline(&mgr.client, &session_id, offline).await?;
    Ok(json!({ "offline" : offline }))
}
pub(crate) async fn handle_console(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let clear = cmd.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
    if clear {
        state.event_tracker.clear_console();
        Ok(json!({ "cleared" : true }))
    } else {
        let result = state.event_tracker.get_console_json();
        Ok(result)
    }
}
pub(crate) async fn handle_errors(state: &DaemonState) -> Result<Value, String> {
    Ok(state.event_tracker.get_errors_json())
}
pub(crate) async fn handle_state_save(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let path = cmd.get("path").and_then(|v| v.as_str());
    let tracked_origins = state
        .tracked_origin_storage
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let saved_path = state::save_state(
        &mgr.client,
        &session_id,
        path,
        state.session_name.as_deref(),
        &state.session_id,
        mgr.visited_origins(),
        &tracked_origins,
    )
    .await?;
    Ok(json!({ "saved" : true, "path" : saved_path }))
}
pub(crate) async fn handle_state_load(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let path = cmd
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' parameter")?;
    state::load_state(&mgr.client, &session_id, path).await?;
    Ok(json!({ "loaded" : true, "path" : path }))
}
pub(crate) async fn handle_diff_snapshot(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let compact = cmd
        .get("compact")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_depth = cmd
        .get("maxDepth")
        .and_then(|v| v.as_u64())
        .map(|d| d as usize);
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .map(String::from);
    let options = SnapshotOptions {
        compact,
        depth: max_depth,
        selector,
        ..SnapshotOptions::default()
    };
    let current = snapshot::take_snapshot(
        &mgr.client,
        &session_id,
        &options,
        &mut state.ref_map,
        state.active_frame_id.as_deref(),
        &state.iframe_sessions,
    )
    .await?;
    let baseline = cmd.get("baseline").and_then(|v| v.as_str());
    let baseline_text = match baseline {
        Some(b) if std::path::Path::new(b).exists() => {
            std::fs::read_to_string(b).map_err(|e| format!("Failed to read baseline: {}", e))?
        }
        Some(b) => b.to_string(),
        None => String::new(),
    };
    let result = diff::diff_snapshots(&baseline_text, &current);
    Ok(json!(
        { "diff" : result.diff, "additions" : result.additions, "removals" : result
        .removals, "unchanged" : result.unchanged, "changed" : result.changed, }
    ))
}
pub(crate) async fn handle_diff_url(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let url1 = cmd
        .get("url1")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url1' parameter")?;
    let url2 = cmd
        .get("url2")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url2' parameter")?;
    let wait_until = cmd
        .get("waitUntil")
        .and_then(|v| v.as_str())
        .map(WaitUntil::from_str)
        .unwrap_or(WaitUntil::Load);
    mgr.navigate(url1, wait_until).await?;
    let session_id = mgr.active_session_id()?.to_string();
    let options = SnapshotOptions::default();
    let snap1 = snapshot::take_snapshot(
        &mgr.client,
        &session_id,
        &options,
        &mut state.ref_map,
        None,
        &state.iframe_sessions,
    )
    .await?;
    mgr.navigate(url2, wait_until).await?;
    state.ref_map.clear();
    let snap2 = snapshot::take_snapshot(
        &mgr.client,
        &session_id,
        &options,
        &mut state.ref_map,
        None,
        &state.iframe_sessions,
    )
    .await?;
    let result = diff::diff_text(&snap1, &snap2);
    Ok(json!(
        { "diff" : result, "url1" : url1, "url2" : url2, "snapshot1" : snap1,
        "snapshot2" : snap2, }
    ))
}
pub(crate) async fn handle_credentials_set(cmd: &Value) -> Result<Value, String> {
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name'")?;
    let username = cmd
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'username'")?;
    let password = cmd
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'password'")?;
    let url = cmd.get("url").and_then(|v| v.as_str());
    auth::credentials_set(name, username, password, url)
}
pub(crate) async fn handle_credentials_get(cmd: &Value) -> Result<Value, String> {
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name'")?;
    auth::credentials_get(name)
}
pub(crate) async fn handle_credentials_delete(cmd: &Value) -> Result<Value, String> {
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name'")?;
    auth::credentials_delete(name)
}
pub(crate) async fn handle_credentials_list() -> Result<Value, String> {
    auth::credentials_list()
}
pub(crate) async fn handle_auth_show(cmd: &Value) -> Result<Value, String> {
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name'")?;
    auth::auth_show(name)
}
pub(crate) async fn handle_mouse(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let event_type = cmd
        .get("eventType")
        .and_then(|v| v.as_str())
        .unwrap_or("mouseMoved");
    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let button = cmd.get("button").and_then(|v| v.as_str()).unwrap_or("none");
    let click_count = cmd.get("clickCount").and_then(|v| v.as_i64()).unwrap_or(0);
    mgr.client
        .send_command(
            "Input.dispatchMouseEvent",
            Some(json!(
                { "type" : event_type, "x" : x, "y" : y, "button" : button,
                "clickCount" : click_count, }
            )),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "dispatched" : event_type }))
}
pub(crate) async fn handle_keyboard(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    match cmd.get("subaction").and_then(|v| v.as_str()) {
        Some("type") => {
            let text = cmd
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'text' parameter")?;
            interaction::type_text_into_active_context(&mgr.client, &session_id, text, None)
                .await?;
            return Ok(json!({ "typed" : text }));
        }
        Some("insertText") => {
            let text = cmd
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'text' parameter")?;
            mgr.client
                .send_command(
                    "Input.insertText",
                    Some(json!({ "text" : text })),
                    Some(&session_id),
                )
                .await?;
            return Ok(json!({ "inserted" : true }));
        }
        _ => {}
    }
    let event_type = cmd
        .get("eventType")
        .and_then(|v| v.as_str())
        .unwrap_or("keyDown");
    let key = cmd.get("key").and_then(|v| v.as_str());
    let code = cmd.get("code").and_then(|v| v.as_str());
    let text = cmd.get("text").and_then(|v| v.as_str());
    let mut params = json!({ "type" : event_type });
    if let Some(k) = key {
        params["key"] = Value::String(k.to_string());
    }
    if let Some(c) = code {
        params["code"] = Value::String(c.to_string());
    }
    if let Some(t) = text {
        params["text"] = Value::String(t.to_string());
    }
    mgr.client
        .send_command("Input.dispatchKeyEvent", Some(params), Some(&session_id))
        .await?;
    Ok(json!({ "dispatched" : event_type }))
}
pub(crate) async fn handle_tab_list(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let verbose = cmd
        .get("verbose")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let tabs = mgr.tab_list(verbose);
    Ok(json!({ "tabs" : tabs }))
}
pub(crate) fn handle_browser_pid(state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    Ok(json!({ "pid" : mgr.browser_pid().or(state.attached_browser_pid) }))
}
pub(crate) async fn handle_tab_new(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let url = cmd.get("url").and_then(|v| v.as_str());
    state.ref_map.clear();
    state.iframe_sessions.clear();
    state.active_frame_id = None;
    let mut result = mgr.tab_new(url).await?;
    if let Some(object) = result.as_object_mut() {
        object.insert(
            "browserId".to_string(),
            json!(service_browser_id(&state.session_id)),
        );
        object.insert("sessionId".to_string(), json!(state.session_id.clone()));
        let tab_id = object
            .get("targetId")
            .and_then(|value| value.as_str())
            .map(|target_id| format!("target:{target_id}"))
            .or_else(|| {
                object
                    .get("index")
                    .and_then(|value| value.as_u64())
                    .map(|index| format!("tab-index:{index}"))
            })
            .unwrap_or_else(|| format!("session:{}:active-tab", state.session_id));
        let target_id = object.get("targetId").cloned().unwrap_or(Value::Null);
        let current_url = object.get("url").cloned().unwrap_or(Value::Null);
        let title = object.get("title").cloned().unwrap_or(Value::Null);
        if let Some(runtime_profile) = mgr.runtime_profile_name() {
            object.insert("runtimeProfile".to_string(), json!(runtime_profile));
            object.insert("profileId".to_string(), json!(runtime_profile));
        }
        let profile_id = object.get("profileId").cloned().unwrap_or(Value::Null);
        object.insert(
            "sharedAcquisition".to_string(),
            tab_new_shared_acquisition_evidence(cmd, &state.session_id, profile_id.clone()),
        );
        let service_tab_handle = json!(
            { "browserId" : service_browser_id(& state.session_id), "sessionName" : state
            .session_id.clone(), "tabId" : tab_id, "targetId" : target_id, "url" :
            current_url, "title" : title, "profileId" : profile_id.clone(),
            "profileOrigin" : "agent_browser_owned", "leaseId" : state.session_id
            .clone(), "leaseState" : "shared", "cleanupPolicy" : "detach",
            "leaseHeartbeatExpected" : true, "ownerSessionId" : state.session_id.clone(),
            "jobId" : Value::Null, "traceFilter" : { "browserId" : service_browser_id(&
            state.session_id), "profileId" : profile_id.clone(), "sessionId" : state
            .session_id.clone(), "serviceName" : optional_command_string(cmd,
            "serviceName"), "agentName" : optional_command_string(cmd, "agentName"),
            "taskName" : optional_command_string(cmd, "taskName"), }, "valid" : true,
            "staleReason" : Value::Null, }
        );
        persist_service_owned_tab_new(
            cmd,
            &state.session_id,
            object.get("targetId").and_then(Value::as_str),
            object.get("url").and_then(Value::as_str),
            object.get("title").and_then(Value::as_str),
            &service_tab_handle,
        )?;
        object.insert("serviceTabHandle".to_string(), service_tab_handle);
    }
    Ok(result)
}
pub(crate) fn persist_service_owned_tab_new(
    cmd: &Value,
    session_id: &str,
    target_id: Option<&str>,
    url: Option<&str>,
    title: Option<&str>,
    service_tab_handle: &Value,
) -> Result<(), String> {
    let Some(target_id) = target_id else {
        return Ok(());
    };
    let handle: ServiceTabHandle = serde_json::from_value(service_tab_handle.clone())
        .map_err(|err| format!("Invalid service tab handle: {}", err))?;
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
                url: url.map(str::to_string),
                title: title.filter(|value| !value.is_empty()).map(str::to_string),
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
            id: format!(
                "service-tab-new-{}-{}",
                tab_id.replace(':', "-"),
                observed_at
            ),
            timestamp: observed_at.clone(),
            kind: ServiceEventKind::TabLifecycleChanged,
            message: format!("Service tab '{}' opened.", tab_id),
            browser_id: Some(browser_id),
            profile_id: handle.profile_id.clone(),
            session_id: Some(session_id.to_string()),
            service_name,
            agent_name,
            task_name,
            details: Some(json!(
                { "action" : "tab_new", "targetId" : target_id, "tabId" :
                tab_id, "url" : url, }
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
pub(crate) fn tab_new_shared_acquisition_evidence(
    cmd: &Value,
    session_id: &str,
    profile_id: Value,
) -> Value {
    let requested_browser_id = optional_command_string(cmd, "browserId");
    let requested_session_name = optional_command_string(cmd, "sessionName");
    let routed_browser_id = service_browser_id(session_id);
    let reused_browser = requested_browser_id
        .as_deref()
        .map(|browser_id| browser_id == routed_browser_id)
        .unwrap_or(false)
        || requested_session_name
            .as_deref()
            .map(|session_name| session_name == session_id)
            .unwrap_or(false);
    let route_hint_source = match (
        requested_browser_id.as_ref(),
        requested_session_name.as_ref(),
    ) {
        (Some(_), Some(_)) => "request.browserId_sessionName",
        (Some(_), None) => "request.browserId",
        (None, Some(_)) => "request.sessionName",
        (None, None) => "none",
    };
    let route_hint_fields: &[&str] = if route_hint_source == "none" {
        &[]
    } else {
        &["browserId", "sessionName"]
    };
    shared_profile_acquisition_result(SharedProfileAcquisitionResultInput {
        state: None,
        mode: "tab_new",
        action: "opened_new_tab",
        recommended_action: Some(if reused_browser {
            "reuse_existing_browser"
        } else {
            "open_shared_profile_tab"
        }),
        browser_reused: reused_browser,
        tab_opened: true,
        browser_id: &routed_browser_id,
        session_name: session_id,
        profile_id: Some(&profile_id),
        requested_profile: profile_id.as_str(),
        planned_profile: profile_id.as_str(),
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
        tab_acquisition_decision: None,
    })
}
pub(crate) async fn handle_tab_switch(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let index = cmd
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or("Missing 'index' parameter")? as usize;
    state.ref_map.clear();
    state.iframe_sessions.clear();
    state.active_frame_id = None;
    let result = mgr.tab_switch(index).await?;
    if let Some(ref server) = state.stream_server {
        if let Ok(dims) = mgr
            .evaluate(
                "JSON.stringify([window.innerWidth,window.innerHeight])",
                None,
            )
            .await
        {
            if let Some(s) = dims.get("result").and_then(|v| v.as_str()) {
                if let Ok(arr) = serde_json::from_str::<Vec<u32>>(s) {
                    if arr.len() == 2 && arr[0] > 0 && arr[1] > 0 {
                        server.set_viewport(arr[0], arr[1]).await;
                    }
                }
            }
        }
    }
    Ok(result)
}
pub(crate) async fn handle_tab_close(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let index = cmd
        .get("index")
        .and_then(|v| v.as_u64())
        .map(|i| i as usize);
    state.ref_map.clear();
    state.iframe_sessions.clear();
    state.active_frame_id = None;
    mgr.tab_close(index).await
}
pub(crate) async fn handle_tab_handle_refresh(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "tab_handle_refresh requires serviceTabHandle".to_string())?;
    let repair_policy =
        optional_command_string(cmd, "repairPolicy").unwrap_or_else(|| "reject_only".to_string());
    if !matches!(
        repair_policy.as_str(),
        "reject_only" | "reuse_compatible" | "open_if_missing" | "replace_duplicates"
    ) {
        return Err(
            "tab_handle_refresh repairPolicy must be reject_only, reuse_compatible, open_if_missing, or replace_duplicates"
                .to_string(),
        );
    }
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let browser_id = handle
        .get("browserId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| service_browser_id(&state.session_id));
    let target_id = handle.get("targetId").and_then(Value::as_str);
    let requested_url = optional_command_string(cmd, "url")
        .or_else(|| optional_command_string(cmd, "desiredUrl"))
        .or_else(|| {
            handle
                .get("url")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        });
    let desired_origin = requested_url.as_deref().and_then(origin_for_url);
    let mut candidates = retained_tab_handle_candidates(handle, requested_url.as_deref());
    let old_handle_valid =
        validate_service_tab_handle_for_current_session(handle, &state.session_id)
            .map(|_| true)
            .unwrap_or(false);
    let mgr = state.browser.as_mut().ok_or_else(|| {
        "Cannot refresh service tab handle: routed browser session is not running".to_string()
    })?;
    for page in mgr.pages_list() {
        let classification = classify_live_page_candidate(
            &page.target_id,
            page.url.as_str(),
            target_id,
            desired_origin.as_deref(),
        );
        candidates.push(json!(
            { "source" : "live_browser", "classification" : classification,
            "targetId" : page.target_id, "url" : page.url, "title" : page.title,
            }
        ));
    }
    if let Some(target_id) = target_id {
        if old_handle_valid || repair_policy != "reject_only" {
            if let Ok(mut switched) = mgr.tab_switch_target_id(target_id).await {
                let url = mgr.get_url().await.unwrap_or_default();
                let title = mgr.get_title().await.unwrap_or_default();
                switched["refreshDecision"] = json!("exact_handle_still_valid");
                let refreshed_handle = refreshed_service_tab_handle(
                    handle,
                    &state.session_id,
                    target_id,
                    url.as_str(),
                    title.as_str(),
                );
                persist_tab_handle_refresh_event(
                    cmd,
                    &browser_id,
                    refreshed_handle.get("profileId").and_then(Value::as_str),
                    "exact_handle_still_valid",
                    &observed_at,
                    &candidates,
                )?;
                let duplicate_target_cleanup = if repair_policy == "replace_duplicates" {
                    close_compatible_duplicate_targets(
                        mgr,
                        target_id,
                        Some(target_id),
                        desired_origin.as_deref(),
                    )
                    .await
                } else {
                    no_duplicate_target_cleanup()
                };
                return Ok(json!(
                    { "ok" : true, "action" : "tab_handle_refresh", "refreshed" :
                    true, "decision" : "exact_handle_still_valid", "repairPolicy" :
                    repair_policy, "observedAt" : observed_at, "browserId" :
                    browser_id, "targetId" : target_id, "url" : url, "title" : title,
                    "tabSwitch" : switched, "serviceTabHandle" : refreshed_handle,
                    "duplicateTargetCleanup" : duplicate_target_cleanup, "candidates"
                    : candidates, }
                ));
            }
        }
    }
    if repair_policy == "reject_only" {
        persist_tab_handle_refresh_event(
            cmd,
            &browser_id,
            handle.get("profileId").and_then(Value::as_str),
            "rejected_stale_or_missing_target",
            &observed_at,
            &candidates,
        )?;
        return Ok(json!(
            { "ok" : false, "action" : "tab_handle_refresh", "refreshed" : false,
            "decision" : "rejected_stale_or_missing_target", "repairPolicy" :
            repair_policy, "observedAt" : observed_at, "browserId" : browser_id,
            "staleReason" : handle.get("staleReason").cloned()
            .unwrap_or(Value::Null), "serviceTabHandle" : cmd.get("serviceTabHandle")
            .cloned().unwrap_or(Value::Null), "candidates" : candidates, }
        ));
    }
    if repair_policy == "reuse_compatible"
        || repair_policy == "open_if_missing"
        || repair_policy == "replace_duplicates"
    {
        if let Some(page) = mgr.pages_list().into_iter().find(|page| {
            classify_live_page_candidate(
                &page.target_id,
                page.url.as_str(),
                target_id,
                desired_origin.as_deref(),
            )
            .starts_with("compatible_")
        }) {
            let mut switched = mgr.tab_switch_target_id(&page.target_id).await?;
            let url = mgr.get_url().await.unwrap_or_default();
            let title = mgr.get_title().await.unwrap_or_default();
            switched["refreshDecision"] = json!("reused_compatible_target");
            let refreshed_handle = service_tab_handle_from_parts(
                handle,
                &state.session_id,
                &page.target_id,
                url.as_str(),
                title.as_str(),
            );
            persist_tab_handle_refresh_event(
                cmd,
                &browser_id,
                refreshed_handle.get("profileId").and_then(Value::as_str),
                "reused_compatible_target",
                &observed_at,
                &candidates,
            )?;
            let duplicate_target_cleanup = if repair_policy == "replace_duplicates" {
                close_compatible_duplicate_targets(
                    mgr,
                    &page.target_id,
                    target_id,
                    desired_origin.as_deref(),
                )
                .await
            } else {
                no_duplicate_target_cleanup()
            };
            return Ok(json!(
                { "ok" : true, "action" : "tab_handle_refresh", "refreshed" : true,
                "decision" : "reused_compatible_target", "repairPolicy" :
                repair_policy, "observedAt" : observed_at, "browserId" : browser_id,
                "targetId" : page.target_id, "url" : url, "title" : title,
                "tabSwitch" : switched, "serviceTabHandle" : refreshed_handle,
                "duplicateTargetCleanup" : duplicate_target_cleanup, "candidates" :
                candidates, }
            ));
        }
    }
    if repair_policy == "open_if_missing" || repair_policy == "replace_duplicates" {
        let open_url = requested_url.as_deref().unwrap_or("about:blank");
        let mut opened = mgr.tab_new(Some(open_url)).await?;
        let new_target_id = opened
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or_else(|| "tab_handle_refresh opened a tab without targetId".to_string())?
            .to_string();
        let url = mgr.get_url().await.unwrap_or_else(|_| open_url.to_string());
        let title = mgr.get_title().await.unwrap_or_default();
        opened["refreshDecision"] = json!("opened_replacement_target");
        let refreshed_handle =
            service_tab_handle_from_parts(handle, &state.session_id, &new_target_id, &url, &title);
        persist_tab_handle_refresh_event(
            cmd,
            &browser_id,
            refreshed_handle.get("profileId").and_then(Value::as_str),
            "opened_replacement_target",
            &observed_at,
            &candidates,
        )?;
        let duplicate_target_cleanup = if repair_policy == "replace_duplicates" {
            close_compatible_duplicate_targets(
                mgr,
                &new_target_id,
                target_id,
                desired_origin.as_deref(),
            )
            .await
        } else {
            no_duplicate_target_cleanup()
        };
        return Ok(json!(
            { "ok" : true, "action" : "tab_handle_refresh", "refreshed" : true,
            "decision" : "opened_replacement_target", "repairPolicy" : repair_policy,
            "observedAt" : observed_at, "browserId" : browser_id, "targetId" :
            new_target_id, "url" : url, "title" : title, "tabNew" : opened,
            "serviceTabHandle" : refreshed_handle, "duplicateTargetCleanup" :
            duplicate_target_cleanup, "candidates" : candidates, }
        ));
    }
    persist_tab_handle_refresh_event(
        cmd,
        &browser_id,
        handle.get("profileId").and_then(Value::as_str),
        "no_compatible_target",
        &observed_at,
        &candidates,
    )?;
    Ok(json!(
        { "ok" : false, "action" : "tab_handle_refresh", "refreshed" : false,
        "decision" : "no_compatible_target", "repairPolicy" : repair_policy,
        "observedAt" : observed_at, "browserId" : browser_id, "serviceTabHandle" :
        cmd.get("serviceTabHandle").cloned().unwrap_or(Value::Null), "candidates" :
        candidates, }
    ))
}
pub(crate) async fn handle_tab_handle_release(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "tab_handle_release requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_route_for_current_session(handle, &state.session_id)?;
    let physical_tab_close =
        release_physical_tab_for_handle(handle, state, cmd.get("closePhysicalTab")).await;
    let released_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let repository = LockedServiceStateRepository::default_json()?;
    repository.mutate(|service_state| {
        release_service_tab_handle_record(
            service_state,
            handle,
            &state.session_id,
            &released_at,
            &physical_tab_close,
        )
    })
}
pub(crate) async fn release_physical_tab_for_handle(
    handle: &Map<String, Value>,
    state: &mut DaemonState,
    close_physical_tab: Option<&Value>,
) -> Value {
    let close_requested = close_physical_tab.and_then(Value::as_bool).unwrap_or(true);
    if !close_requested {
        return json!(
            { "attempted" : false, "closed" : false, "skippedReason" :
            "request_disabled_physical_close", "error" : Value::Null, "result" :
            Value::Null, }
        );
    }
    if handle.get("cleanupPolicy").and_then(Value::as_str) == Some("release_only") {
        return json!(
            { "attempted" : false, "closed" : false, "skippedReason" :
            "cleanup_policy_release_only", "error" : Value::Null, "result" : Value::Null,
            }
        );
    }
    let Some(target_id) = handle.get("targetId").and_then(Value::as_str) else {
        return json!(
            { "attempted" : false, "closed" : false, "skippedReason" :
            "missing_target_id", "error" : Value::Null, "result" : Value::Null, }
        );
    };
    let Some(mgr) = state.browser.as_mut() else {
        return json!(
            { "attempted" : false, "closed" : false, "skippedReason" : "no_live_browser",
            "error" : Value::Null, "result" : Value::Null, }
        );
    };
    match mgr.tab_close_target_id(target_id).await {
        Ok(result) => {
            json!(
                { "attempted" : true, "closed" : true, "skippedReason" : Value::Null,
                "error" : Value::Null, "result" : result, }
            )
        }
        Err(error) => {
            let skipped_reason = if error.contains("Cannot close the last tab") {
                "last_tab_preserved"
            } else if error.contains("was not found in the attached tab list") {
                "target_not_attached"
            } else {
                "physical_close_failed"
            };
            json!(
                { "attempted" : true, "closed" : false, "skippedReason" : skipped_reason,
                "error" : error, "result" : Value::Null, }
            )
        }
    }
}
pub(crate) fn release_service_tab_handle_record(
    service_state: &mut ServiceState,
    handle: &Map<String, Value>,
    routed_session_id: &str,
    released_at: &str,
    physical_tab_close: &Value,
) -> Result<Value, String> {
    let tab_id = handle
        .get("tabId")
        .and_then(Value::as_str)
        .ok_or_else(|| "serviceTabHandle.tabId is required".to_string())?;
    let browser_id = handle
        .get("browserId")
        .and_then(Value::as_str)
        .ok_or_else(|| "serviceTabHandle.browserId is required".to_string())?;
    let session_name = handle
        .get("sessionName")
        .and_then(Value::as_str)
        .or_else(|| handle.get("ownerSessionId").and_then(Value::as_str))
        .unwrap_or(routed_session_id);
    let target_id = handle.get("targetId").cloned().unwrap_or(Value::Null);
    let cleanup_policy = handle.get("cleanupPolicy").cloned().unwrap_or(Value::Null);
    let before_lifecycle = service_state
        .tabs
        .get(tab_id)
        .map(|tab| serde_json::to_value(tab.lifecycle).unwrap_or_else(|_| json!("unknown")));
    let mut tab_released = false;
    let mut tab_missing = false;
    match service_state.tabs.get_mut(tab_id) {
        Some(tab) => {
            if tab.browser_id != browser_id {
                return Err(
                    format!(
                        "service tab handle browserId {browser_id} does not match retained tab {} browserId {}",
                        tab.id, tab.browser_id
                    ),
                );
            }
            tab.lifecycle = TabLifecycle::Closed;
            tab.service_tab_handle = None;
            tab_released = true;
        }
        None => {
            tab_missing = true;
        }
    }
    if let Some(session) = service_state.sessions.get_mut(session_name) {
        session.last_lease_observed_at = Some(released_at.to_string());
    }
    service_state.events.push(ServiceEvent {
        id: format!(
            "tab-handle-release-{}-{}",
            tab_id.replace(':', "-"),
            released_at
        ),
        timestamp: released_at.to_string(),
        kind: ServiceEventKind::TabLifecycleChanged,
        message: format!("Service tab handle '{}' released.", tab_id),
        browser_id: Some(browser_id.to_string()),
        profile_id: handle
            .get("profileId")
            .and_then(Value::as_str)
            .map(str::to_string),
        session_id: Some(session_name.to_string()),
        service_name: optional_command_string_from_handle_or_trace(handle, "serviceName"),
        agent_name: optional_command_string_from_handle_or_trace(handle, "agentName"),
        task_name: optional_command_string_from_handle_or_trace(handle, "taskName"),
        details: Some(json!(
            { "action" : "tab_handle_release", "tabId" : tab_id, "targetId" :
            target_id, "cleanupPolicy" : cleanup_policy, "physicalTabClose" :
            physical_tab_close, "browserProcessPreserved" : true,
            "sessionRoutePreserved" : true, "tabMissing" : tab_missing, }
        )),
        ..ServiceEvent::default()
    });
    if service_state.events.len() > 100 {
        let excess = service_state.events.len() - 100;
        service_state.events.drain(0..excess);
    }
    service_state.refresh_service_tab_handles();
    let released_handle = service_state
        .tabs
        .get(tab_id)
        .and_then(|tab| tab.service_tab_handle.clone());
    Ok(json!(
        { "ok" : true, "action" : "tab_handle_release", "released" : true,
        "tabReleased" : tab_released, "tabMissing" : tab_missing,
        "browserProcessPreserved" : true, "sessionRoutePreserved" : true,
        "closeBrowserOnRelease" : false, "physicalTabClose" : physical_tab_close,
        "physicalTabCloseAttempted" : physical_tab_close.get("attempted").cloned()
        .unwrap_or(Value::Bool(false)), "physicalTabClosed" : physical_tab_close
        .get("closed").cloned().unwrap_or(Value::Bool(false)),
        "physicalTabCloseSkippedReason" : physical_tab_close.get("skippedReason")
        .cloned().unwrap_or(Value::Null), "browserId" : browser_id, "sessionName" :
        session_name, "tabId" : tab_id, "targetId" : target_id, "cleanupPolicy" :
        cleanup_policy, "beforeLifecycle" : before_lifecycle.unwrap_or(Value::Null),
        "afterLifecycle" : if tab_released { json!("closed") } else { Value::Null },
        "serviceTabHandle" : released_handle, "releasedAt" : released_at, }
    ))
}
pub(crate) fn optional_command_string_from_handle_or_trace(
    handle: &Map<String, Value>,
    key: &str,
) -> Option<String> {
    handle
        .get(key)
        .and_then(Value::as_str)
        .or_else(|| {
            handle
                .get("traceFilter")
                .and_then(Value::as_object)
                .and_then(|trace_filter| trace_filter.get(key))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
}
pub(crate) fn service_tab_handle_from_parts(
    previous: &Map<String, Value>,
    session_id: &str,
    target_id: &str,
    url: &str,
    title: &str,
) -> Value {
    let tab_id = format!("target:{target_id}");
    let profile_id = previous.get("profileId").cloned().unwrap_or(Value::Null);
    json!(
        { "browserId" : service_browser_id(session_id), "sessionName" : session_id,
        "tabId" : tab_id, "targetId" : target_id, "url" : url, "title" : title,
        "profileId" : profile_id.clone(), "profileOrigin" : previous.get("profileOrigin")
        .cloned().unwrap_or_else(|| json!("agent_browser_owned")), "leaseId" : previous
        .get("leaseId").cloned().unwrap_or_else(|| json!(session_id)), "leaseState" :
        previous.get("leaseState").cloned().unwrap_or_else(|| json!("shared")),
        "cleanupPolicy" : previous.get("cleanupPolicy").cloned().unwrap_or_else(||
        json!("detach")), "leaseHeartbeatExpected" : previous
        .get("leaseHeartbeatExpected").and_then(Value::as_bool).unwrap_or(true),
        "ownerSessionId" : previous.get("ownerSessionId").cloned().unwrap_or_else(||
        json!(session_id)), "jobId" : previous.get("jobId").cloned()
        .unwrap_or(Value::Null), "traceFilter" : { "browserId" :
        service_browser_id(session_id), "profileId" : profile_id, "sessionId" :
        session_id, }, "valid" : true, "staleReason" : Value::Null, }
    )
}
pub(crate) fn refreshed_service_tab_handle(
    previous: &Map<String, Value>,
    session_id: &str,
    target_id: &str,
    url: &str,
    title: &str,
) -> Value {
    let mut refreshed = service_tab_handle_from_parts(previous, session_id, target_id, url, title);
    if let Some(tab_id) = previous.get("tabId") {
        refreshed["tabId"] = tab_id.clone();
    }
    refreshed
}
pub(crate) fn retained_tab_handle_candidates(
    handle: &Map<String, Value>,
    desired_url: Option<&str>,
) -> Vec<Value> {
    let mut service_state = LockedServiceStateRepository::default_json()
        .and_then(|repository| repository.load_snapshot())
        .unwrap_or_default();
    service_state.refresh_service_tab_handles();
    let handle_tab_id = handle.get("tabId").and_then(Value::as_str);
    let handle_target_id = handle.get("targetId").and_then(Value::as_str);
    let desired_origin = desired_url
        .or_else(|| handle.get("url").and_then(Value::as_str))
        .and_then(origin_for_url);
    service_state
        .tabs
        .values()
        .map(|tab| {
            let browser = service_state.browsers.get(&tab.browser_id);
            json!(
                { "source" : "service_state", "classification" :
                classify_retained_tab_candidate(tab, browser, handle_tab_id,
                handle_target_id, desired_origin.as_deref()), "tabId" : tab.id,
                "browserId" : tab.browser_id, "targetId" : tab.target_id, "url" : tab
                .url, "title" : tab.title, "lifecycle" : tab.lifecycle, "browserHealth" :
                browser.map(| browser | browser.health), "serviceTabHandle" : tab
                .service_tab_handle, }
            )
        })
        .collect()
}
pub(crate) fn classify_retained_tab_candidate(
    tab: &BrowserTab,
    browser: Option<&BrowserProcess>,
    handle_tab_id: Option<&str>,
    handle_target_id: Option<&str>,
    desired_origin: Option<&str>,
) -> &'static str {
    if Some(tab.id.as_str()) == handle_tab_id {
        if tab.lifecycle == TabLifecycle::Closed {
            return "closed_tab";
        }
        if browser.is_none_or(|browser| browser.health != ServiceBrowserHealth::Ready) {
            return "dead_browser";
        }
        return "exact_handle";
    }
    if tab.target_id.as_deref().is_some() && tab.target_id.as_deref() == handle_target_id {
        return "matching_target";
    }
    if tab.lifecycle == TabLifecycle::Closed {
        return "closed_tab";
    }
    if browser.is_none_or(|browser| browser.health != ServiceBrowserHealth::Ready) {
        return "dead_browser";
    }
    if let Some(url) = tab.url.as_deref() {
        if is_blank_url(url) {
            return "compatible_blank_tab";
        }
        if desired_origin.is_some() && origin_for_url(url).as_deref() == desired_origin {
            return "compatible_same_origin_tab";
        }
    }
    "incompatible_tab"
}
pub(crate) fn classify_live_page_candidate(
    page_target_id: &str,
    page_url: &str,
    handle_target_id: Option<&str>,
    desired_origin: Option<&str>,
) -> &'static str {
    if Some(page_target_id) == handle_target_id {
        return "matching_target";
    }
    if is_blank_url(page_url) {
        return "compatible_blank_tab";
    }
    if desired_origin.is_some() && origin_for_url(page_url).as_deref() == desired_origin {
        return "compatible_same_origin_tab";
    }
    "incompatible_tab"
}
pub(crate) fn compatible_duplicate_live_pages(
    pages: &[PageInfo],
    selected_target_id: &str,
    handle_target_id: Option<&str>,
    desired_origin: Option<&str>,
) -> Vec<Value> {
    pages
        .iter()
        .filter_map(|page| {
            if page.target_id == selected_target_id {
                return None;
            }
            let classification = classify_live_page_candidate(
                &page.target_id,
                page.url.as_str(),
                handle_target_id,
                desired_origin,
            );
            if !classification.starts_with("compatible_") {
                return None;
            }
            Some(json!(
                { "targetId" : page.target_id, "url" : page.url, "title" : page
                .title, "classification" : classification, }
            ))
        })
        .collect()
}
pub(crate) fn no_duplicate_target_cleanup() -> Value {
    json!(
        { "policy" : "preserve", "attempted" : false, "closedCount" : 0, "closedTargets"
        : [], "failedTargets" : [], }
    )
}
pub(crate) async fn close_compatible_duplicate_targets(
    mgr: &mut BrowserManager,
    selected_target_id: &str,
    handle_target_id: Option<&str>,
    desired_origin: Option<&str>,
) -> Value {
    let duplicates = compatible_duplicate_live_pages(
        &mgr.pages_list(),
        selected_target_id,
        handle_target_id,
        desired_origin,
    );
    if duplicates.is_empty() {
        return json!(
            { "policy" : "replace_duplicates", "attempted" : true, "closedCount" : 0,
            "closedTargets" : [], "failedTargets" : [], }
        );
    }
    let mut closed_targets = Vec::new();
    let mut failed_targets = Vec::new();
    for duplicate in duplicates {
        let target_id = duplicate
            .get("targetId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if target_id.is_empty() {
            continue;
        }
        match mgr.tab_close_target_id(&target_id).await {
            Ok(result) => closed_targets.push(json!(
                { "targetId" : target_id, "url" : duplicate.get("url")
                .cloned().unwrap_or(Value::Null), "title" : duplicate
                .get("title").cloned().unwrap_or(Value::Null),
                "classification" : duplicate.get("classification").cloned()
                .unwrap_or(Value::Null), "result" : result, }
            )),
            Err(error) => failed_targets.push(json!(
                { "targetId" : target_id, "url" : duplicate.get("url")
                .cloned().unwrap_or(Value::Null), "title" : duplicate
                .get("title").cloned().unwrap_or(Value::Null),
                "classification" : duplicate.get("classification").cloned()
                .unwrap_or(Value::Null), "error" : error, }
            )),
        }
    }
    let _ = mgr.tab_switch_target_id(selected_target_id).await;
    json!(
        { "policy" : "replace_duplicates", "attempted" : true, "closedCount" :
        closed_targets.len(), "closedTargets" : closed_targets, "failedTargets" :
        failed_targets, }
    )
}
pub(crate) fn is_blank_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.is_empty() || trimmed == "about:blank"
}
pub(crate) fn origin_for_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if let Some(rest) = trimmed.strip_prefix("https://") {
        return rest
            .split('/')
            .next()
            .filter(|host| !host.is_empty())
            .map(|host| format!("https://{}", host.to_ascii_lowercase()));
    }
    if let Some(rest) = trimmed.strip_prefix("http://") {
        return rest
            .split('/')
            .next()
            .filter(|host| !host.is_empty())
            .map(|host| format!("http://{}", host.to_ascii_lowercase()));
    }
    None
}
pub(crate) fn persist_tab_handle_refresh_event(
    cmd: &Value,
    browser_id: &str,
    profile_id: Option<&str>,
    decision: &str,
    observed_at: &str,
    candidates: &[Value],
) -> Result<(), String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let event_id = format!("tab-handle-refresh-{}-{}", browser_id, observed_at);
    let service_name = optional_command_string(cmd, "serviceName");
    let agent_name = optional_command_string(cmd, "agentName");
    let task_name = optional_command_string(cmd, "taskName");
    repository.mutate(|state| {
        state.events.push(ServiceEvent {
            id: event_id.clone(),
            timestamp: observed_at.to_string(),
            kind: ServiceEventKind::TabLifecycleChanged,
            message: format!("Service tab handle refresh {decision}."),
            browser_id: Some(browser_id.to_string()),
            profile_id: profile_id.map(ToString::to_string),
            session_id: optional_command_string(cmd, "sessionName"),
            service_name,
            agent_name,
            task_name,
            details: Some(json!(
                { "action" : "tab_handle_refresh", "decision" : decision,
                "repairPolicy" : cmd.get("repairPolicy").cloned()
                .unwrap_or_else(|| json!("reject_only")), "targetId" : cmd
                .get("targetId").cloned().unwrap_or(Value::Null),
                "candidateCount" : candidates.len(), "candidates" :
                candidates, }
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
pub(crate) async fn handle_view_focus(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    state.ref_map.clear();
    state.iframe_sessions.clear();
    state.active_frame_id = None;
    let mut tab_switched = None;
    let fallback_index = cmd
        .get("index")
        .and_then(|v| v.as_u64())
        .map(|i| i as usize);
    if let Some(target_id) = cmd
        .get("targetId")
        .or_else(|| cmd.get("target_id"))
        .and_then(|v| v.as_str())
        .filter(|value| !value.trim().is_empty())
    {
        let target_id = target_id.trim();
        if mgr.active_target_id().ok() == Some(target_id) {
            tab_switched = Some(json!({ "targetId" : target_id, "state" : "already_active", }));
        } else {
            match mgr.tab_switch_target_id(target_id).await {
                Ok(value) => tab_switched = Some(value),
                Err(target_err) => {
                    if let Some(index) = fallback_index {
                        let mut fallback = mgr.tab_switch(index).await?;
                        fallback["fallbackFromTargetId"] = json!(target_id);
                        fallback["fallbackReason"] = json!(target_err);
                        tab_switched = Some(fallback);
                    } else {
                        return Err(target_err);
                    }
                }
            }
        }
    } else if let Some(index) = fallback_index {
        tab_switched = Some(mgr.tab_switch(index).await?);
    }
    let maximize = cmd
        .get("maximize")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let mut result = if cmd
        .get("nativeFocusOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        mgr.focus_native_window_for_view_only(maximize)
    } else if cmd
        .get("allowBringToFrontFailure")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        mgr.focus_for_view_allowing_bring_to_front_failure(maximize)
            .await?
    } else {
        mgr.focus_for_view(maximize).await?
    };
    if let Some(tab_switched) = tab_switched {
        result["tabSwitch"] = tab_switched;
    }
    Ok(result)
}
pub(crate) async fn handle_view_takeover(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let browser_id = optional_command_string(cmd, "browserId")
        .unwrap_or_else(|| service_browser_id(&state.session_id));
    let session_name =
        optional_command_string(cmd, "sessionName").unwrap_or_else(|| state.session_id.clone());
    let stream_id = optional_command_string(cmd, "streamId");
    let provider = optional_command_string(cmd, "provider");
    let open_mode =
        optional_command_string(cmd, "openMode").unwrap_or_else(|| "iframe".to_string());
    let reason =
        optional_command_string(cmd, "reason").unwrap_or_else(|| "operator_request".to_string());
    let target_id = optional_command_string(cmd, "targetId");
    let tab_index = cmd.get("index").and_then(Value::as_u64);
    let provider_mode = match provider.as_deref() {
        Some("rdp_gateway" | "rdp-gateway") => "provider_single_view",
        Some(_) => "provider_multi_view",
        None => "provider_unknown",
    };
    let requested_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let viewer_lease_id = format!(
        "viewer:{}:{}:{}",
        browser_id,
        stream_id.as_deref().unwrap_or("default"),
        requested_at
    );
    let service_event_id = persist_view_takeover_requested_event(
        &browser_id,
        &session_name,
        stream_id.as_deref(),
        provider.as_deref(),
        &open_mode,
        &reason,
        target_id.as_deref(),
        tab_index,
        &viewer_lease_id,
        &requested_at,
        cmd,
    )?;
    Ok(json!(
        { "status" : "accepted", "takeoverStatus" : "accepted", "takeoverRequested" :
        true, "reconnectRequested" : true, "browserId" : browser_id, "sessionName" :
        session_name, "streamId" : stream_id, "provider" : provider, "openMode" :
        open_mode, "reason" : reason, "targetId" : target_id, "index" : tab_index,
        "providerMode" : provider_mode, "viewerLeaseId" : viewer_lease_id,
        "lastViewerEvent" : "takeover_requested", "serviceEventId" :
        service_event_id, "browserProcessPreserved" : true, "requestedAt" :
        requested_at, }
    ))
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn persist_view_takeover_requested_event(
    browser_id: &str,
    session_name: &str,
    stream_id: Option<&str>,
    provider: Option<&str>,
    open_mode: &str,
    reason: &str,
    target_id: Option<&str>,
    tab_index: Option<u64>,
    viewer_lease_id: &str,
    requested_at: &str,
    cmd: &Value,
) -> Result<String, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let event_id = format!("viewer-takeover-{}-{}", browser_id, requested_at);
    let service_name = optional_command_string(cmd, "serviceName");
    let agent_name = optional_command_string(cmd, "agentName");
    let task_name = optional_command_string(cmd, "taskName");
    repository.mutate(|state| {
        let profile_id = state
            .browsers
            .get(browser_id)
            .and_then(|browser| browser.profile_id.clone());
        let event = ServiceEvent {
            id: event_id.clone(),
            timestamp: requested_at.to_string(),
            kind: ServiceEventKind::ViewerTakeoverRequested,
            message: format!(
                "Viewer takeover requested for {} via {}.",
                browser_id,
                provider.unwrap_or("unknown_provider")
            ),
            browser_id: Some(browser_id.to_string()),
            profile_id,
            session_id: Some(session_name.to_string()),
            service_name,
            agent_name,
            task_name,
            details: Some(json!(
                { "streamId" : stream_id, "provider" : provider, "openMode" :
                open_mode, "reason" : reason, "targetId" : target_id, "index" :
                tab_index, "viewerLeaseId" : viewer_lease_id, "lastViewerEvent" :
                "takeover_requested", "takeoverStatus" : "accepted", }
            )),
            ..ServiceEvent::default()
        };
        state.events.push(event);
        if state.events.len() > 100 {
            let excess = state.events.len() - 100;
            state.events.drain(0..excess);
        }
        Ok(())
    })?;
    Ok(event_id)
}
pub(crate) async fn handle_set_media(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let media = cmd.get("media").and_then(|v| v.as_str());
    let mut feat_list: Vec<(String, String)> = Vec::new();
    if let Some(scheme) = cmd.get("colorScheme").and_then(|v| v.as_str()) {
        feat_list.push(("prefers-color-scheme".to_string(), scheme.to_string()));
    }
    if let Some(motion) = cmd.get("reducedMotion").and_then(|v| v.as_str()) {
        feat_list.push(("prefers-reduced-motion".to_string(), motion.to_string()));
    }
    if let Some(obj) = cmd.get("features").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            feat_list.push((k.clone(), v.as_str().unwrap_or("").to_string()));
        }
    }
    let features = if feat_list.is_empty() {
        None
    } else {
        Some(feat_list)
    };
    mgr.set_emulated_media(media, features).await?;
    Ok(json!({ "set" : true }))
}
pub(crate) async fn handle_download(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let path_str = cmd
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' parameter")?;
    let raw_dest = if std::path::Path::new(path_str).is_absolute() {
        PathBuf::from(path_str)
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?
            .join(path_str)
    };
    let download_dir = raw_dest
        .parent()
        .ok_or("Invalid download path: no parent directory")?
        .to_path_buf();
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create download directory: {}", e))?;
    let download_dir = download_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve download directory: {}", e))?;
    let dest = download_dir.join(
        raw_dest
            .file_name()
            .ok_or("Invalid download path: no filename")?,
    );
    let download_dir_str = download_dir
        .to_str()
        .ok_or("Download directory path is not valid UTF-8")?;
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    mgr.set_download_behavior(download_dir_str).await?;
    let mut rx = mgr.client.subscribe();
    interaction::click(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        "left",
        1,
        &state.iframe_sessions,
    )
    .await?;
    const DOWNLOAD_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + DOWNLOAD_TIMEOUT;
    let mut downloaded_guid: Option<String> = None;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("Timeout waiting for download to complete".to_string());
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                let is_page_session = event.session_id.as_deref() == Some(&session_id);
                let is_download_event = |method: &str, browser_method: &str, page_method: &str| {
                    method == browser_method || (method == page_method && is_page_session)
                };
                if is_download_event(
                    &event.method,
                    "Browser.downloadWillBegin",
                    "Page.downloadWillBegin",
                ) {
                    if let Some(guid) = event.params.get("guid").and_then(|v| v.as_str()) {
                        downloaded_guid = Some(guid.to_string());
                    }
                }
                if is_download_event(
                    &event.method,
                    "Browser.downloadProgress",
                    "Page.downloadProgress",
                ) {
                    match event.params.get("state").and_then(|v| v.as_str()) {
                        Some("completed") => break,
                        Some("canceled") => {
                            return Err("Download was canceled".to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => return Err("Event stream closed".to_string()),
            Err(_) => return Err("Timeout waiting for download to complete".to_string()),
        }
    }
    if let Some(guid) = downloaded_guid {
        let guid_path = download_dir.join(&guid);
        for _ in 0..10 {
            if guid_path.exists() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        if guid_path.exists() {
            std::fs::rename(&guid_path, &dest)
                .map_err(|e| format!("Failed to rename downloaded file: {}", e))?;
        } else {
            if !dest.exists() {
                return Err(format!(
                    "Downloaded file not found at expected path (GUID: {})",
                    guid
                ));
            }
        }
    } else {
        if !dest.exists() {
            return Err(
                "Download completed but could not determine the downloaded file name".to_string(),
            );
        }
    }
    let dest_str = dest.to_string_lossy().to_string();
    Ok(json!({ "path" : dest_str }))
}
pub(crate) async fn handle_trace_start(state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    native_tracing::trace_start(&mgr.client, &session_id, &mut state.tracing_state).await
}
pub(crate) async fn handle_trace_stop(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let path = cmd.get("path").and_then(|v| v.as_str());
    native_tracing::trace_stop(&mgr.client, &session_id, &mut state.tracing_state, path).await
}
pub(crate) async fn handle_profiler_start(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let categories = cmd.get("categories").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });
    native_tracing::profiler_start(
        &mgr.client,
        &session_id,
        &mut state.tracing_state,
        categories,
    )
    .await
}
pub(crate) async fn handle_profiler_stop(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let path = cmd.get("path").and_then(|v| v.as_str());
    native_tracing::profiler_stop(&mgr.client, &session_id, &mut state.tracing_state, path).await
}
pub(crate) async fn handle_recording_start(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let path = cmd
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' parameter")?;
    let recording_url = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let (client, recording_session_id) = {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        let active_session_id = mgr.active_session_id()?.to_string();
        let current_url = mgr
            .get_url()
            .await
            .unwrap_or_else(|_| "about:blank".to_string());
        if recording_url.is_none_or(|u| u == current_url) {
            (mgr.client.clone(), active_session_id)
        } else {
            let nav_url = recording_url.unwrap_or("about:blank").to_string();
            let cookies_result = mgr
                .client
                .send_command_no_params("Network.getAllCookies", Some(&active_session_id))
                .await
                .ok();
            let ctx_result = mgr
                .client
                .send_command_no_params("Target.createBrowserContext", None)
                .await?;
            let context_id = ctx_result
                .get("browserContextId")
                .and_then(|v| v.as_str())
                .ok_or("Failed to get browserContextId")?
                .to_string();
            let create_result: CreateTargetResult = mgr
                .client
                .send_command_typed(
                    "Target.createTarget",
                    &json!({ "url" : "about:blank", "browserContextId" : context_id }),
                    None,
                )
                .await?;
            let attach_result: AttachToTargetResult = mgr
                .client
                .send_command_typed(
                    "Target.attachToTarget",
                    &AttachToTargetParams {
                        target_id: create_result.target_id.clone(),
                        flatten: true,
                    },
                    None,
                )
                .await?;
            let new_session_id = attach_result.session_id.clone();
            mgr.enable_domains_pub(&new_session_id).await?;
            if let Some(ref dl_path) = mgr.download_path {
                let _ = mgr
                    .client
                    .send_command(
                        "Browser.setDownloadBehavior",
                        Some(json!(
                            { "behavior" : "allow", "downloadPath" : dl_path,
                            "browserContextId" : context_id, "eventsEnabled" : true }
                        )),
                        None,
                    )
                    .await;
            }
            if let Some(ref cr) = cookies_result {
                if let Some(cookie_arr) = cr.get("cookies").and_then(|v| v.as_array()) {
                    if !cookie_arr.is_empty() {
                        let _ = mgr
                            .client
                            .send_command(
                                "Network.setCookies",
                                Some(json!({ "cookies" : cookie_arr })),
                                Some(&new_session_id),
                            )
                            .await;
                    }
                }
            }
            if mgr.ignore_https_errors {
                let _ = mgr
                    .client
                    .send_command(
                        "Security.setIgnoreCertificateErrors",
                        Some(json!({ "ignore" : true })),
                        Some(&new_session_id),
                    )
                    .await;
            }
            mgr.add_page(super::super::browser::PageInfo {
                target_id: create_result.target_id,
                session_id: new_session_id.clone(),
                url: nav_url.clone(),
                title: String::new(),
                target_type: "page".to_string(),
            });
            if nav_url != "about:blank" {
                let _ = mgr
                    .client
                    .send_command(
                        "Page.navigate",
                        Some(json!({ "url" : nav_url })),
                        Some(&new_session_id),
                    )
                    .await;
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
            (mgr.client.clone(), new_session_id)
        }
    };
    let result = recording::recording_start(&mut state.recording_state, path)?;
    state
        .start_recording_task(client, recording_session_id)
        .await?;
    if let Some(ref server) = state.stream_server {
        server.set_recording(true, &state.engine).await;
    }
    Ok(result)
}
pub(crate) async fn handle_recording_stop(state: &mut DaemonState) -> Result<Value, String> {
    state.stop_recording_task().await?;
    let result = recording::recording_stop(&mut state.recording_state);
    if let Some(ref server) = state.stream_server {
        server.set_recording(false, &state.engine).await;
    }
    result
}
pub(crate) async fn handle_recording_restart(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let path = cmd
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' parameter")?;
    let _ = state.stop_recording_task().await;
    let result = recording::recording_restart(&mut state.recording_state, path)?;
    if let Some(ref browser) = state.browser {
        let session_id = browser.active_session_id()?.to_string();
        state
            .start_recording_task(browser.client.clone(), session_id)
            .await?;
    }
    Ok(result)
}
pub(crate) async fn handle_pdf(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let params = json!(
        { "printBackground" : cmd.get("printBackground").and_then(| v | v.as_bool())
        .unwrap_or(true), "landscape" : cmd.get("landscape").and_then(| v | v.as_bool())
        .unwrap_or(false), "preferCSSPageSize" : cmd.get("preferCSSPageSize").and_then(|
        v | v.as_bool()).unwrap_or(false), }
    );
    let result = mgr
        .client
        .send_command("Page.printToPDF", Some(params), Some(&session_id))
        .await?;
    let data = result
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or("No PDF data returned")?;
    let path = cmd.get("path").and_then(|v| v.as_str());
    let save_path = match path {
        Some(p) => p.to_string(),
        None => {
            let dir = dirs::home_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(".agent-browser")
                .join("tmp")
                .join("pdfs");
            let _ = std::fs::create_dir_all(&dir);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            dir.join(format!("page-{}.pdf", timestamp))
                .to_string_lossy()
                .to_string()
        }
    };
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
        .map_err(|e| format!("Failed to decode PDF: {}", e))?;
    std::fs::write(&save_path, &bytes).map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(json!({ "path" : save_path }))
}
pub(crate) async fn handle_focus(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::focus(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "focused" : selector }))
}
pub(crate) async fn handle_clear(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::clear(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "cleared" : selector }))
}
pub(crate) async fn handle_selectall(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::select_all(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "selected" : selector }))
}
pub(crate) async fn handle_scrollintoview(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::scroll_into_view(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "scrolled" : selector }))
}
pub(crate) async fn handle_dispatch(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let event_type = cmd
        .get("event")
        .or_else(|| cmd.get("eventType"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'event' parameter")?;
    let event_init = cmd.get("eventInit");
    interaction::dispatch_event(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        event_type,
        event_init,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "dispatched" : event_type, "selector" : selector }))
}
pub(crate) async fn handle_highlight(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    interaction::highlight(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "highlighted" : selector }))
}
pub(crate) async fn handle_tap(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let selector = cmd.get("selector").and_then(|v| v.as_str());
    if let Some(ref appium) = state.appium {
        if state.browser.is_none() {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(200.0);
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(200.0);
            appium.tap(x, y).await?;
            return Ok(json!({ "tapped" : true, "x" : x, "y" : y }));
        }
    }
    let sel = selector.ok_or("Missing 'selector' parameter")?;
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    interaction::tap_touch(
        &mgr.client,
        &session_id,
        &state.ref_map,
        sel,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "tapped" : sel }))
}
pub(crate) async fn handle_boundingbox(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let bbox = super::super::element::get_element_bounding_box(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(bbox)
}
pub(crate) async fn handle_innertext(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let text = super::super::element::get_element_inner_text(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "text" : text }))
}
pub(crate) async fn handle_innerhtml(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let html = super::super::element::get_element_inner_html(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "html" : html }))
}
pub(crate) async fn handle_inputvalue(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let value = super::super::element::get_element_input_value(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "value" : value }))
}
pub(crate) async fn handle_setvalue(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let value = cmd
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'value' parameter")?;
    super::super::element::set_element_value(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        value,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "set" : selector, "value" : value }))
}
pub(crate) async fn handle_count(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let count =
        super::super::element::get_element_count(&mgr.client, &session_id, selector).await?;
    Ok(json!({ "count" : count, "selector" : selector }))
}
pub(crate) async fn handle_styles(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let properties = cmd.get("properties").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });
    let styles = super::super::element::get_element_styles(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        properties,
        &state.iframe_sessions,
    )
    .await?;
    Ok(json!({ "styles" : styles }))
}
pub(crate) async fn handle_bringtofront(state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    mgr.bring_to_front().await?;
    Ok(json!({ "broughtToFront" : true }))
}
pub(crate) async fn handle_timezone(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let timezone = cmd
        .get("timezoneId")
        .or_else(|| cmd.get("timezone"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'timezoneId' parameter")?;
    mgr.set_timezone(timezone).await?;
    Ok(json!({ "timezoneId" : timezone }))
}
pub(crate) async fn handle_locale(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let locale = cmd
        .get("locale")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'locale' parameter")?;
    mgr.set_locale(locale).await?;
    Ok(json!({ "locale" : locale }))
}
pub(crate) async fn handle_geolocation(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let latitude = cmd
        .get("latitude")
        .and_then(|v| v.as_f64())
        .ok_or("Missing 'latitude' parameter")?;
    let longitude = cmd
        .get("longitude")
        .and_then(|v| v.as_f64())
        .ok_or("Missing 'longitude' parameter")?;
    let accuracy = cmd.get("accuracy").and_then(|v| v.as_f64());
    mgr.set_geolocation(latitude, longitude, accuracy).await?;
    Ok(json!({ "latitude" : latitude, "longitude" : longitude }))
}
pub(crate) async fn handle_permissions(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let permissions: Vec<String> = cmd
        .get("permissions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    mgr.grant_permissions(&permissions).await?;
    Ok(json!({ "granted" : permissions }))
}
pub(crate) async fn handle_dialog(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let response = cmd.get("response").and_then(|v| v.as_str());
    if response == Some("status") {
        return Ok(match &state.pending_dialog {
            Some(dialog) => {
                let mut obj = json!(
                    { "hasDialog" : true, "type" : dialog.dialog_type, "message" :
                    dialog.message, }
                );
                if let Some(ref prompt) = dialog.default_prompt {
                    obj["defaultPrompt"] = json!(prompt);
                }
                obj
            }
            None => json!({ "hasDialog" : false }),
        });
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let accept = response
        .map(|r| r == "accept")
        .or_else(|| cmd.get("accept").and_then(|v| v.as_bool()))
        .unwrap_or(true);
    let prompt_text = cmd.get("promptText").and_then(|v| v.as_str());
    mgr.handle_dialog(accept, prompt_text).await?;
    state.pending_dialog = None;
    Ok(json!({ "handled" : true, "accepted" : accept }))
}
pub(crate) async fn handle_upload(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let files: Vec<String> = cmd
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .or_else(|| {
            cmd.get("file")
                .and_then(|v| v.as_str())
                .map(|s| vec![s.to_string()])
        })
        .unwrap_or_default();
    let session_id = mgr.active_session_id()?.to_string();
    let (object_id, effective_session_id) = super::super::element::resolve_element_object_id(
        &mgr.client,
        &session_id,
        &state.ref_map,
        selector,
        &state.iframe_sessions,
    )
    .await?;
    mgr.client
        .send_command(
            "DOM.setFileInputFiles",
            Some(json!({ "files" : files, "objectId" : object_id, })),
            Some(&effective_session_id),
        )
        .await?;
    Ok(json!({ "uploaded" : files.len(), "selector" : selector }))
}
pub(crate) async fn handle_addscript(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let content = cmd
        .get("content")
        .or_else(|| cmd.get("source"))
        .or_else(|| cmd.get("script"))
        .and_then(|v| v.as_str());
    let url = cmd.get("url").and_then(|v| v.as_str());
    if content.is_none() && url.is_none() {
        return Err("At least one of 'content' or 'url' is required".to_string());
    }
    if let Some(src_url) = url {
        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const s = document.createElement('script');
                s.src = {};
                s.onload = () => resolve(true);
                s.onerror = () => reject(new Error('Failed to load script'));
                document.head.appendChild(s);
            }})"#,
            serde_json::to_string(src_url).unwrap_or_default()
        );
        mgr.evaluate(&js, None).await?;
    } else if let Some(source) = content {
        let js = format!(
            r#"(() => {{
                const s = document.createElement('script');
                s.textContent = {};
                document.head.appendChild(s);
            }})()"#,
            serde_json::to_string(source).unwrap_or_default()
        );
        mgr.evaluate(&js, None).await?;
    }
    Ok(json!({ "added" : true }))
}
pub(crate) async fn handle_addinitscript(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let source = cmd
        .get("script")
        .or_else(|| cmd.get("source"))
        .or_else(|| cmd.get("content"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'script' parameter")?;
    let identifier = mgr.add_script_to_evaluate(source).await?;
    Ok(json!({ "added" : true, "identifier" : identifier }))
}
pub(crate) async fn handle_addstyle(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let content = cmd
        .get("content")
        .or_else(|| cmd.get("css"))
        .and_then(|v| v.as_str());
    let url = cmd.get("url").and_then(|v| v.as_str());
    if content.is_none() && url.is_none() {
        return Err("At least one of 'content' or 'url' is required".to_string());
    }
    if let Some(href) = url {
        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const link = document.createElement('link');
                link.rel = 'stylesheet';
                link.href = {};
                link.onload = () => resolve(true);
                link.onerror = () => reject(new Error('Failed to load stylesheet'));
                document.head.appendChild(link);
            }})"#,
            serde_json::to_string(href).unwrap_or_default()
        );
        mgr.evaluate(&js, None).await?;
    } else if let Some(css) = content {
        let js = format!(
            r#"(() => {{
                const style = document.createElement('style');
                style.textContent = {};
                document.head.appendChild(style);
            }})()"#,
            serde_json::to_string(css).unwrap_or_default()
        );
        mgr.evaluate(&js, None).await?;
    }
    Ok(json!({ "added" : true }))
}
pub(crate) async fn handle_clipboard(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let action = cmd
        .get("subAction")
        .or_else(|| cmd.get("operation"))
        .and_then(|v| v.as_str())
        .unwrap_or("read");
    let session_id = mgr.active_session_id()?.to_string();
    let modifier: i32 = if cfg!(target_os = "macos") { 4 } else { 2 };
    match action {
        "write" => {
            let text = cmd
                .get("text")
                .or_else(|| cmd.get("value"))
                .and_then(|v| v.as_str())
                .ok_or("Missing 'text' parameter")?;
            let js = format!(
                "navigator.clipboard.writeText({})",
                serde_json::to_string(text).unwrap_or_default()
            );
            mgr.evaluate(&js, None).await?;
            Ok(json!({ "written" : text }))
        }
        "copy" => {
            interaction::press_key_with_modifiers(&mgr.client, &session_id, "c", Some(modifier))
                .await?;
            Ok(json!({ "copied" : true }))
        }
        "paste" => {
            interaction::press_key_with_modifiers(&mgr.client, &session_id, "v", Some(modifier))
                .await?;
            Ok(json!({ "pasted" : true }))
        }
        _ => {
            match super::super::clipboard::read_text(
                &mgr.client,
                &session_id,
                super::super::clipboard::DEFAULT_READ_TIMEOUT,
            )
            .await
            {
                Ok(outcome) => Ok(json!(
                    { "text" : outcome.text, "empty" : outcome.empty,
                    "clipboardOutcome" : if outcome.empty { "success_empty" }
                    else { "success_text" }, }
                )),
                Err(error) => Err(format!(
                    "Clipboard read failed: {}; diagnostic={}",
                    error.message(),
                    error.diagnostic()
                )),
            }
        }
    }
}
pub(crate) async fn handle_wheel(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let delta_x = cmd.get("deltaX").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let delta_y = cmd.get("deltaY").and_then(|v| v.as_f64()).unwrap_or(0.0);
    mgr.client
        .send_command(
            "Input.dispatchMouseEvent",
            Some(json!(
                { "type" : "mouseWheel", "x" : x, "y" : y, "deltaX" : delta_x,
                "deltaY" : delta_y, }
            )),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "scrolled" : true, "deltaX" : delta_x, "deltaY" : delta_y }))
}
pub(crate) async fn handle_device(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let name = cmd
        .get("name")
        .or_else(|| cmd.get("device"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name' parameter")?;
    let (width, height, scale, mobile, ua) = match name.to_lowercase().as_str() {
        "iphone 15" | "iphone15" => {
            (
                393,
                852,
                3.0,
                true,
                "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
            )
        }
        "iphone 16" | "iphone16" => {
            (
                393,
                852,
                3.0,
                true,
                "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
            )
        }
        "iphone 16 pro" | "iphone16pro" => {
            (
                402,
                874,
                3.0,
                true,
                "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
            )
        }
        "iphone 17" | "iphone17" => {
            (
                402,
                874,
                3.0,
                true,
                "Mozilla/5.0 (iPhone; CPU iPhone OS 19_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/19.0 Mobile/15E148 Safari/604.1",
            )
        }
        "ipad" | "ipad air" => {
            (
                820,
                1180,
                2.0,
                true,
                "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/604.1",
            )
        }
        "ipad pro" => {
            (
                1024,
                1366,
                2.0,
                true,
                "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/604.1",
            )
        }
        "pixel 9" | "pixel9" => {
            (
                412,
                923,
                2.625,
                true,
                "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36",
            )
        }
        "galaxy s25" | "galaxys25" => {
            (
                360,
                800,
                3.0,
                true,
                "Mozilla/5.0 (Linux; Android 15; SM-S931B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36",
            )
        }
        "iphone 12" | "iphone12" => {
            (
                390,
                844,
                3.0,
                true,
                "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0 Mobile/15E148 Safari/604.1",
            )
        }
        "iphone 14" | "iphone14" => {
            (
                390,
                844,
                3.0,
                true,
                "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
            )
        }
        "pixel 5" | "pixel5" => {
            (
                393,
                851,
                2.75,
                true,
                "Mozilla/5.0 (Linux; Android 11; Pixel 5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.91 Mobile Safari/537.36",
            )
        }
        "pixel 7" | "pixel7" => {
            (
                412,
                915,
                2.625,
                true,
                "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36",
            )
        }
        "galaxy s21" | "galaxys21" => {
            (
                360,
                800,
                3.0,
                true,
                "Mozilla/5.0 (Linux; Android 11; SM-G991B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.91 Mobile Safari/537.36",
            )
        }
        _ => {
            return Err(
                format!(
                    "Unknown device: {}. Supported: iPhone 15, iPhone 16, iPhone 16 Pro, iPhone 17, iPad, iPad Pro, Pixel 9, Galaxy S25",
                    name
                ),
            );
        }
    };
    mgr.set_viewport(width, height, scale, mobile).await?;
    mgr.set_user_agent(ua).await?;
    if let Some(ref server) = state.stream_server {
        server.set_viewport(width as u32, height as u32).await;
    }
    Ok(json!(
        { "device" : name, "width" : width, "height" : height, "deviceScaleFactor" :
        scale, "mobile" : mobile, }
    ))
}
pub(crate) fn stream_file_path(session_id: &str) -> PathBuf {
    get_socket_dir().join(format!("{}.stream", session_id))
}
pub(crate) fn write_stream_file(session_id: &str, port: u16) -> Result<(), String> {
    let path = stream_file_path(session_id);
    fs::write(&path, port.to_string()).map_err(|e| {
        format!(
            "Failed to write stream metadata '{}': {}",
            path.display(),
            e
        )
    })
}
pub(crate) fn remove_stream_file(session_id: &str) -> Result<(), String> {
    let path = stream_file_path(session_id);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "Failed to remove stream metadata '{}': {}",
            path.display(),
            err
        )),
    }
}
pub(crate) fn engine_file_path(session_id: &str) -> PathBuf {
    get_socket_dir().join(format!("{}.engine", session_id))
}
pub(crate) fn write_engine_file(session_id: &str, engine: &str) {
    let _ = fs::write(engine_file_path(session_id), engine);
}
pub(crate) fn remove_engine_file(session_id: &str) {
    let _ = fs::remove_file(engine_file_path(session_id));
}
pub(crate) fn provider_file_path(session_id: &str) -> PathBuf {
    get_socket_dir().join(format!("{}.provider", session_id))
}
pub(crate) fn write_provider_file(session_id: &str, provider: &str) {
    let _ = fs::write(provider_file_path(session_id), provider);
}
pub(crate) fn remove_provider_file(session_id: &str) {
    let _ = fs::remove_file(provider_file_path(session_id));
}
pub(crate) fn extensions_file_path(session_id: &str) -> PathBuf {
    get_socket_dir().join(format!("{}.extensions", session_id))
}
pub(crate) fn write_extensions_file(session_id: &str) {
    if let Ok(val) = env::var("AGENT_BROWSER_EXTENSIONS") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            let _ = fs::write(extensions_file_path(session_id), trimmed);
            return;
        }
    }
    let _ = fs::remove_file(extensions_file_path(session_id));
}
pub(crate) fn remove_extensions_file(session_id: &str) {
    let _ = fs::remove_file(extensions_file_path(session_id));
}
pub(crate) async fn current_stream_status(state: &DaemonState) -> Value {
    debug_assert_eq!(
        state.stream_server.is_some(),
        state.stream_client.is_some(),
        "stream server and stream client slot should be set together"
    );
    let connected = match state.browser.as_ref() {
        Some(mgr) => mgr.is_connection_alive().await,
        None => false,
    };
    let runtime_screencasting = match state.stream_server.as_ref() {
        Some(server) => server.is_screencasting().await,
        None => false,
    };
    json!(
        { "enabled" : state.stream_server.is_some(), "port" : state.stream_server
        .as_ref().map(| server | Value::from(server.port())).unwrap_or(Value::Null),
        "connected" : connected, "screencasting" : connected && (state.screencasting ||
        runtime_screencasting), }
    )
}
pub(crate) async fn handle_stream_enable(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    if state.stream_server.is_some() {
        return Err("Streaming is already enabled for this session".to_string());
    }
    let requested_port = match cmd.get("port").and_then(|value| value.as_u64()) {
        Some(raw) => u16::try_from(raw)
            .map_err(|_| format!("Invalid stream port '{}': expected 0-65535", raw))?,
        None => 0,
    };
    let (server, client_slot) =
        StreamServer::start_without_client(requested_port, state.session_id.clone(), false).await?;
    let port = server.port();
    if let Err(err) = write_stream_file(&state.session_id, port) {
        server.shutdown().await;
        return Err(err);
    }
    state.stream_client = Some(client_slot);
    state.stream_server = Some(Arc::new(server));
    state.request_tracking = true;
    if state.screencasting {
        if let Some(ref server) = state.stream_server {
            server.set_screencasting(true).await;
        }
    }
    state.update_stream_client().await;
    Ok(current_stream_status(state).await)
}
pub(crate) async fn handle_stream_disable(state: &mut DaemonState) -> Result<Value, String> {
    let Some(server) = state.stream_server.clone() else {
        return Err("Streaming is not enabled for this session".to_string());
    };
    server.shutdown().await;
    state.stream_server = None;
    state.stream_client = None;
    remove_stream_file(&state.session_id)?;
    remove_engine_file(&state.session_id);
    remove_provider_file(&state.session_id);
    Ok(json!({ "disabled" : true }))
}
pub(crate) async fn handle_stream_status(state: &DaemonState) -> Result<Value, String> {
    Ok(current_stream_status(state).await)
}
pub(crate) async fn handle_service_status(cmd: &Value) -> Result<Value, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let projector = super::super::service_status_projection::ServiceStatusProjector::local();
    handle_service_status_with_dependencies(
        cmd,
        super::super::service_status_projection::ServiceStatusProjectionDependencies::new(
            &repository,
            &super::super::service_status_projection::ReconcileServiceStatusAuthority,
            &super::super::service_status_projection::ReconciledBrowserSessionAuthority,
            &projector,
        ),
    )
    .await
}
pub(crate) async fn handle_service_status_with_dependencies<
    Repository,
    Preparer,
    BrowserAuthority,
>(
    cmd: &Value,
    dependencies: super::super::service_status_projection::ServiceStatusProjectionDependencies<
        '_,
        Repository,
        Preparer,
        BrowserAuthority,
    >,
) -> Result<Value, String>
where
    Repository: ServiceStateRepository,
    Preparer: super::super::service_status_projection::ServiceStatusAuthorityPreparer,
    BrowserAuthority:
        super::super::service_status_projection::ServiceStatusBrowserAuthorityProvider,
{
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let before = service_state.clone();
    let waiting_profile_lease_job_count = service_state
        .jobs
        .values()
        .filter(|job| job.state == ServiceJobState::WaitingProfileLease)
        .count();
    if let Some(control_plane) = service_state.control_plane.as_mut() {
        control_plane.waiting_profile_lease_job_count = waiting_profile_lease_job_count;
    } else {
        service_state.control_plane = Some(super::super::service_model::ControlPlaneSnapshot {
            worker_state: "Ready".to_string(),
            browser_health: "NotStarted".to_string(),
            waiting_profile_lease_job_count,
            ..super::super::service_model::ControlPlaneSnapshot::default()
        });
    }
    dependencies.preparer.prepare(&mut service_state).await;
    persist_reconciled_service_state_in_repository(
        dependencies.repository,
        &before,
        &service_state,
    )?;
    let browser_session_authority = dependencies.browser_authority.snapshot(&service_state);
    let control_plane = service_state
        .control_plane
        .as_ref()
        .expect("service status always creates a control-plane snapshot");
    let control_plane =
        super::super::service_status_projection::StatusControlPlaneAuthority::try_from(
            control_plane,
        )
        .map_err(|error| error.to_string())?;
    let launch_config =
        super::super::service_status_projection::launch_configuration_from_status_command(cmd);
    let full_tab_history = cmd
        .get("fullTabHistory")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response =
        super::super::service_status_projection::project_status_with_launch_configuration(
            dependencies.projector,
            service_state,
            control_plane,
            browser_session_authority,
            launch_config,
            full_tab_history,
        )
        .await
        .map_err(|error| error.to_string())?;
    serde_json::to_value(response).map_err(|error| error.to_string())
}
pub(crate) async fn handle_waitforurl(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let url_pattern = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' parameter")?;
    let timeout_ms = state.timeout_ms(cmd);
    wait_for_url(&mgr.client, &session_id, url_pattern, timeout_ms).await?;
    let url = mgr.get_url().await.unwrap_or_default();
    Ok(json!({ "url" : url }))
}
pub(crate) async fn handle_waitforloadstate(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let load_state = cmd.get("state").and_then(|v| v.as_str()).unwrap_or("load");
    let timeout_ms = state.timeout_ms(cmd);
    let wait_until = WaitUntil::from_str(load_state);
    let _ = tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms),
        mgr.wait_for_lifecycle_external(wait_until, &session_id),
    )
    .await
    .map_err(|_| format!("Timeout waiting for load state: {}", load_state))?;
    Ok(json!({ "state" : load_state }))
}
pub(crate) async fn handle_waitforfunction(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let expression = cmd
        .get("expression")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'expression' parameter")?;
    let timeout_ms = state.timeout_ms(cmd);
    wait_for_function(&mgr.client, &session_id, expression, timeout_ms).await?;
    let result: super::super::cdp::types::EvaluateResult = mgr
        .client
        .send_command_typed(
            "Runtime.evaluate",
            &super::super::cdp::types::EvaluateParams {
                expression: format!("({})", expression),
                return_by_value: Some(true),
                await_promise: Some(true),
            },
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "result" : result.result.value.unwrap_or(Value::Null) }))
}
pub(crate) async fn handle_frame(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd.get("selector").and_then(|v| v.as_str());
    let name = cmd.get("name").and_then(|v| v.as_str());
    let url = cmd.get("url").and_then(|v| v.as_str());
    if selector.is_none() && name.is_none() && url.is_none() {
        return Err("At least one of 'selector', 'name', or 'url' is required".to_string());
    }
    let tree_result = mgr
        .client
        .send_command_no_params("Page.getFrameTree", Some(&session_id))
        .await?;
    fn find_frame(tree: &Value, name: Option<&str>, url: Option<&str>) -> Option<String> {
        let frame = tree.get("frame")?;
        let frame_name = frame.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let frame_url = frame.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let frame_id = frame.get("id").and_then(|v| v.as_str())?;
        if let Some(n) = name {
            if frame_name == n {
                return Some(frame_id.to_string());
            }
        }
        if let Some(u) = url {
            if frame_url.contains(u) {
                return Some(frame_id.to_string());
            }
        }
        if let Some(children) = tree.get("childFrames").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(id) = find_frame(child, name, url) {
                    return Some(id);
                }
            }
        }
        None
    }
    let frame_tree = &tree_result["frameTree"];
    if let Some(sel) = selector {
        if let Some(ref_id) = super::super::element::parse_ref(sel) {
            let entry = state
                .ref_map
                .get(&ref_id)
                .ok_or_else(|| format!("Unknown ref: {}", ref_id))?;
            let backend_node_id = entry
                .backend_node_id
                .ok_or_else(|| format!("Ref {} has no backend node id", ref_id))?;
            let describe: Value = mgr
                .client
                .send_command(
                    "DOM.describeNode",
                    Some(json!({ "backendNodeId" : backend_node_id, "depth" : 1 })),
                    Some(&session_id),
                )
                .await?;
            let node_name = describe
                .get("node")
                .and_then(|n| n.get("nodeName"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if node_name != "IFRAME" && node_name != "FRAME" {
                return Err("Ref does not point to an iframe element".to_string());
            }
            let frame_id = describe
                .get("node")
                .and_then(|n| n.get("contentDocument"))
                .and_then(|cd| cd.get("frameId"))
                .and_then(|v| v.as_str())
                .or_else(|| {
                    describe
                        .get("node")
                        .and_then(|n| n.get("frameId"))
                        .and_then(|v| v.as_str())
                })
                .ok_or("Could not resolve frame ID for iframe element")?;
            let label = describe
                .get("node")
                .and_then(|n| n.get("attributes"))
                .and_then(|a| a.as_array())
                .and_then(|attrs| {
                    attrs
                        .iter()
                        .enumerate()
                        .find(|(_, v)| v.as_str() == Some("name"))
                        .and_then(|(i, _)| attrs.get(i + 1))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or(&ref_id);
            state.active_frame_id = Some(frame_id.to_string());
            return Ok(json!({ "frame" : label }));
        }
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({});
                if (!el) return null;
                if (el.tagName === 'IFRAME' || el.tagName === 'FRAME') {{
                    return el.name || el.id || el.src || null;
                }}
                return null;
            }})()"#,
            serde_json::to_string(sel).unwrap_or_default()
        );
        let result = mgr.evaluate(&js, None).await?;
        let frame_name = result.as_str().ok_or("Could not find frame for selector")?;
        if let Some(frame_id) = find_frame(frame_tree, Some(frame_name), None) {
            state.active_frame_id = Some(frame_id);
            return Ok(json!({ "frame" : frame_name }));
        }
    }
    if let Some(frame_id) = find_frame(frame_tree, name, url) {
        let label = name.or(url).unwrap_or("frame");
        state.active_frame_id = Some(frame_id);
        return Ok(json!({ "frame" : label }));
    }
    Err("Frame not found".to_string())
}
pub(crate) async fn handle_mainframe(state: &mut DaemonState) -> Result<Value, String> {
    state.active_frame_id = None;
    Ok(json!({ "frame" : "main" }))
}
pub(crate) async fn execute_subaction(
    cmd: &Value,
    state: &mut DaemonState,
    selector: &str,
) -> Result<Value, String> {
    let subaction = cmd
        .get("subaction")
        .and_then(|v| v.as_str())
        .unwrap_or("click");
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    match subaction {
        "click" => {
            interaction::click(
                &mgr.client,
                &session_id,
                &state.ref_map,
                selector,
                "left",
                1,
                &state.iframe_sessions,
            )
            .await?;
            Ok(json!({ "clicked" : selector }))
        }
        "fill" => {
            let value = cmd
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'value' for fill subaction")?;
            interaction::fill(
                &mgr.client,
                &session_id,
                &state.ref_map,
                selector,
                value,
                &state.iframe_sessions,
            )
            .await?;
            Ok(json!({ "filled" : selector }))
        }
        "check" => {
            interaction::check(
                &mgr.client,
                &session_id,
                &state.ref_map,
                selector,
                &state.iframe_sessions,
            )
            .await?;
            Ok(json!({ "checked" : selector }))
        }
        "hover" => {
            interaction::hover(
                &mgr.client,
                &session_id,
                &state.ref_map,
                selector,
                &state.iframe_sessions,
            )
            .await?;
            Ok(json!({ "hovered" : selector }))
        }
        "text" => {
            let text = super::super::element::get_element_text(
                &mgr.client,
                &session_id,
                &state.ref_map,
                selector,
                &state.iframe_sessions,
            )
            .await?;
            Ok(json!({ "text" : text }))
        }
        _ => Err(format!("Unknown subaction: {}", subaction)),
    }
}
pub(crate) fn build_role_selector(role: &str, name: Option<&str>, exact: bool) -> String {
    match name {
        Some(n) => {
            let exact_str = if exact { ", exact: true" } else { "" };
            format!("getByRole('{}', {{ name: '{}'{} }})", role, n, exact_str)
        }
        None => format!("getByRole('{}')", role),
    }
}
pub(crate) async fn handle_getbyrole(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let (client, session_id) = {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        (mgr.client.clone(), mgr.active_session_id()?.to_string())
    };
    let role = cmd
        .get("role")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'role' parameter")?;
    let name = cmd.get("name").and_then(|v| v.as_str());
    let exact = cmd.get("exact").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut frame_ids = state.iframe_sessions.keys().cloned().collect::<Vec<_>>();
    frame_ids.sort();
    let mut located = None;
    for frame_id in
        std::iter::once(None).chain(frame_ids.iter().map(|frame_id| Some(frame_id.as_str())))
    {
        let (_, effective_session_id) = super::super::element::resolve_ax_session(
            frame_id,
            &session_id,
            &state.iframe_sessions,
        );
        let _ = client
            .send_command_no_params("Accessibility.enable", Some(effective_session_id))
            .await;
        if let Ok(backend_node_id) = super::super::element::find_backend_node_by_role_name(
            &client,
            &session_id,
            super::super::element::RoleNameQuery {
                role,
                name,
                exact,
                nth: None,
            },
            frame_id,
            &state.iframe_sessions,
        )
        .await
        {
            located = Some((backend_node_id, frame_id.map(str::to_string)));
            break;
        }
    }
    let Some((backend_node_id, frame_id)) = located else {
        let desc = build_role_selector(role, name, exact);
        return Err(format!("No element found: {}", desc));
    };
    let ref_id = format!("e{}", state.ref_map.next_ref_num());
    state.ref_map.add_with_frame(
        ref_id.clone(),
        Some(backend_node_id),
        role,
        name.unwrap_or(""),
        None,
        frame_id.as_deref(),
    );
    let selector = format!("@{ref_id}");
    let result = execute_subaction(cmd, state, &selector).await;
    state.ref_map.remove(&ref_id);
    result
}
pub(crate) async fn handle_semantic_locator(
    cmd: &Value,
    state: &mut DaemonState,
    strategy: &str,
    param_name: &str,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let value = cmd
        .get(param_name)
        .and_then(|v| v.as_str())
        .ok_or(format!("Missing '{}' parameter", param_name))?;
    let exact = cmd.get("exact").and_then(|v| v.as_bool()).unwrap_or(false);
    let match_fn = if exact {
        format!(
            "el.textContent.trim() === {}",
            serde_json::to_string(value).unwrap_or_default()
        )
    } else {
        format!(
            "el.textContent.includes({})",
            serde_json::to_string(value).unwrap_or_default()
        )
    };
    let query = match strategy {
        "label" => {
            format!(
                r#"(() => {{
                const label = Array.from(document.querySelectorAll('label')).find(el => {match_fn});
                if (!label) return false;
                const forId = label.getAttribute('for');
                const target = forId ? document.getElementById(forId) : label.querySelector('input,select,textarea');
                if (target) {{ target.setAttribute('data-agent-browser-located', 'true'); return true; }}
                return false;
            }})()"#,
                match_fn = match_fn,
            )
        }
        "placeholder" => {
            format!(
                r#"(() => {{
                const el = document.querySelector('input[placeholder={val}], textarea[placeholder={val}]');
                if (el) {{ el.setAttribute('data-agent-browser-located', 'true'); return true; }}
                return false;
            }})()"#,
                val = serde_json::to_string(value).unwrap_or_default(),
            )
        }
        "alttext" => {
            format!(
                r#"(() => {{
                const el = document.querySelector('img[alt={val}], [alt={val}]');
                if (el) {{ el.setAttribute('data-agent-browser-located', 'true'); return true; }}
                return false;
            }})()"#,
                val = serde_json::to_string(value).unwrap_or_default(),
            )
        }
        "title" => {
            format!(
                r#"(() => {{
                const el = document.querySelector('[title={val}]');
                if (el) {{ el.setAttribute('data-agent-browser-located', 'true'); return true; }}
                return false;
            }})()"#,
                val = serde_json::to_string(value).unwrap_or_default(),
            )
        }
        "testid" => {
            format!(
                r#"(() => {{
                const el = document.querySelector('[data-testid={val}]');
                if (el) {{ el.setAttribute('data-agent-browser-located', 'true'); return true; }}
                return false;
            }})()"#,
                val = serde_json::to_string(value).unwrap_or_default(),
            )
        }
        _ => {
            format!(
                r#"(() => {{
                    const all = document.querySelectorAll('*');
                    for (const el of all) {{
                        if (el.children.length === 0 && {match_fn}) {{
                            el.setAttribute('data-agent-browser-located', 'true');
                            return true;
                        }}
                    }}
                    return false;
                }})()"#,
                match_fn = match_fn,
            )
        }
    };
    let result: super::super::cdp::types::EvaluateResult = mgr
        .client
        .send_command_typed(
            "Runtime.evaluate",
            &super::super::cdp::types::EvaluateParams {
                expression: query,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&session_id),
        )
        .await?;
    if !result
        .result
        .value
        .as_ref()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err(format!("No element found by {} '{}'", strategy, value));
    }
    let selector = "[data-agent-browser-located='true']";
    let action_result = execute_subaction(cmd, state, selector).await;
    if let Some(ref browser) = state.browser {
        let _ = browser
            .evaluate(
                "document.querySelector('[data-agent-browser-located]')?.removeAttribute('data-agent-browser-located')",
                None,
            )
            .await;
    }
    action_result
}
pub(crate) async fn handle_getbytext(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    handle_semantic_locator(cmd, state, "text", "text").await
}
pub(crate) async fn handle_getbylabel(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    handle_semantic_locator(cmd, state, "label", "label").await
}
pub(crate) async fn handle_getbyplaceholder(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    handle_semantic_locator(cmd, state, "placeholder", "placeholder").await
}
pub(crate) async fn handle_getbyalttext(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    handle_semantic_locator(cmd, state, "alttext", "text").await
}
pub(crate) async fn handle_getbytitle(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    handle_semantic_locator(cmd, state, "title", "text").await
}
pub(crate) async fn handle_getbytestid(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    handle_semantic_locator(cmd, state, "testid", "testId").await
}
pub(crate) async fn handle_nth(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let index = cmd
        .get("index")
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'index' parameter")?;
    let js = format!(
        r#"(() => {{
            const els = document.querySelectorAll({sel});
            const idx = {idx} < 0 ? els.length + {idx} : {idx};
            if (idx < 0 || idx >= els.length) return false;
            els[idx].setAttribute('data-agent-browser-located', 'true');
            return true;
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_default(),
        idx = index,
    );
    let result: super::super::cdp::types::EvaluateResult = mgr
        .client
        .send_command_typed(
            "Runtime.evaluate",
            &super::super::cdp::types::EvaluateParams {
                expression: js,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&session_id),
        )
        .await?;
    if !result
        .result
        .value
        .as_ref()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err(format!(
            "No element at index {} for selector '{}'",
            index, selector
        ));
    }
    let located = "[data-agent-browser-located='true']";
    let action_result = execute_subaction(cmd, state, located).await;
    if let Some(ref browser) = state.browser {
        let _ = browser
            .evaluate(
                "document.querySelector('[data-agent-browser-located]')?.removeAttribute('data-agent-browser-located')",
                None,
            )
            .await;
    }
    action_result
}
pub(crate) async fn handle_find(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let js = format!(
        r#"(() => {{
            const els = document.querySelectorAll({});
            return Array.from(els).map((el, i) => ({{
                index: i,
                tagName: el.tagName.toLowerCase(),
                text: el.textContent?.trim().substring(0, 100) || '',
                visible: el.offsetWidth > 0 && el.offsetHeight > 0,
            }}));
        }})()"#,
        serde_json::to_string(selector).unwrap_or_default()
    );
    let result = mgr.evaluate(&js, None).await?;
    Ok(json!({ "elements" : result, "selector" : selector }))
}
pub(crate) async fn handle_evalhandle(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let script = cmd
        .get("script")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'script' parameter")?;
    let result: super::super::cdp::types::EvaluateResult = mgr
        .client
        .send_command_typed(
            "Runtime.evaluate",
            &super::super::cdp::types::EvaluateParams {
                expression: script.to_string(),
                return_by_value: Some(false),
                await_promise: Some(true),
            },
            Some(&session_id),
        )
        .await?;
    let handle = result.result.object_id.unwrap_or_default();
    Ok(json!({ "handle" : handle }))
}
pub(crate) async fn handle_drag(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let source = cmd
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'source' parameter")?;
    let target = cmd
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'target' parameter")?;
    let (sx, sy, source_session_id) = super::super::element::resolve_element_center(
        &mgr.client,
        &session_id,
        &state.ref_map,
        source,
        &state.iframe_sessions,
    )
    .await?;
    let (tx, ty, target_session_id) = super::super::element::resolve_element_center(
        &mgr.client,
        &session_id,
        &state.ref_map,
        target,
        &state.iframe_sessions,
    )
    .await?;
    mgr.client
        .send_command(
            "Input.dispatchMouseEvent",
            Some(json!({ "type" : "mouseMoved", "x" : sx, "y" : sy })),
            Some(&source_session_id),
        )
        .await?;
    mgr.client
        .send_command(
            "Input.dispatchMouseEvent",
            Some(json!(
                { "type" : "mousePressed", "x" : sx, "y" : sy, "button" : "left",
                "buttons" : 1, "clickCount" : 1 }
            )),
            Some(&source_session_id),
        )
        .await?;
    let steps = 10;
    for i in 1..=steps {
        let cx = sx + (tx - sx) * (i as f64) / (steps as f64);
        let cy = sy + (ty - sy) * (i as f64) / (steps as f64);
        mgr.client
            .send_command(
                "Input.dispatchMouseEvent",
                Some(json!(
                    { "type" : "mouseMoved", "x" : cx, "y" : cy, "button" : "left",
                    "buttons" : 1 }
                )),
                Some(&target_session_id),
            )
            .await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    mgr.client
        .send_command(
            "Input.dispatchMouseEvent",
            Some(json!(
                { "type" : "mouseReleased", "x" : tx, "y" : ty, "button" : "left",
                "buttons" : 0, "clickCount" : 1 }
            )),
            Some(&target_session_id),
        )
        .await?;
    Ok(json!({ "dragged" : true, "source" : source, "target" : target }))
}
pub(crate) async fn handle_expose(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name' parameter")?;
    mgr.client
        .send_command(
            "Runtime.addBinding",
            Some(json!({ "name" : name })),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "exposed" : name }))
}
pub(crate) async fn handle_pause(_state: &DaemonState) -> Result<Value, String> {
    Ok(json!(
        { "paused" : true, "note" :
        "Use DevTools to inspect. The daemon remains running." }
    ))
}
pub(crate) async fn handle_multiselect(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let selector = cmd
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    let values: Vec<String> = cmd
        .get("values")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let values_json = serde_json::to_string(&values).unwrap_or("[]".to_string());
    let js = format!(
        r#"(() => {{
            const select = document.querySelector({sel});
            if (!select) throw new Error('Select element not found');
            const vals = {vals};
            for (const opt of select.options) {{
                opt.selected = vals.includes(opt.value);
            }}
            select.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return Array.from(select.selectedOptions).map(o => o.value);
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_default(),
        vals = values_json,
    );
    let result = mgr.evaluate(&js, None).await?;
    Ok(json!({ "selected" : result }))
}
pub(crate) async fn handle_responsebody(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let url_pattern = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' parameter")?;
    let timeout_ms = state.timeout_ms(cmd);
    let mut rx = mgr.client.subscribe();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "Timeout waiting for response matching '{}'",
                url_pattern
            ));
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                if event.method == "Network.responseReceived"
                    && event.session_id.as_deref() == Some(&session_id)
                {
                    if let Some(resp_url) = event
                        .params
                        .get("response")
                        .and_then(|r| r.get("url"))
                        .and_then(|u| u.as_str())
                    {
                        if resp_url.contains(url_pattern) {
                            let request_id = event
                                .params
                                .get("requestId")
                                .and_then(|v| v.as_str())
                                .ok_or("No requestId in response event")?;
                            let status = event
                                .params
                                .get("response")
                                .and_then(|r| r.get("status"))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                            let headers = event
                                .params
                                .get("response")
                                .and_then(|r| r.get("headers"))
                                .cloned()
                                .unwrap_or(json!({}));
                            let body_result = mgr
                                .client
                                .send_command(
                                    "Network.getResponseBody",
                                    Some(json!({ "requestId" : request_id })),
                                    Some(&session_id),
                                )
                                .await?;
                            let body = body_result
                                .get("body")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            return Ok(json!(
                                { "body" : body, "status" : status, "headers" : headers }
                            ));
                        }
                    }
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => return Err("Event stream closed".to_string()),
            Err(_) => {
                return Err(format!(
                    "Timeout waiting for response matching '{}'",
                    url_pattern
                ));
            }
        }
    }
}
pub(crate) async fn handle_waitfordownload(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let timeout_ms = state.timeout_ms(cmd);
    let expected_path = cmd.get("path").and_then(|v| v.as_str()).map(String::from);
    let initial_file_state = expected_path.as_ref().and_then(|path| {
        std::fs::metadata(path).ok().map(|meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|dur| dur.as_nanos());
            (meta.len(), modified)
        })
    });
    let mut rx = mgr.client.subscribe();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("Timeout waiting for download".to_string());
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                let is_page_session = event.session_id.as_deref() == Some(&session_id);
                let is_progress = event.method == "Browser.downloadProgress"
                    || (event.method == "Page.downloadProgress" && is_page_session);
                if is_progress
                    && event.params.get("state").and_then(|v| v.as_str()) == Some("completed")
                {
                    let path = expected_path.as_deref().unwrap_or("download");
                    return Ok(json!({ "path" : path }));
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => return Err("Event stream closed".to_string()),
            Err(_) => return Err("Timeout waiting for download".to_string()),
        }
        if let Some(ref path) = expected_path {
            if let Ok(meta) = std::fs::metadata(path) {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dur| dur.as_nanos());
                let current_state = (meta.len(), modified);
                if initial_file_state.as_ref() != Some(&current_state) {
                    return Ok(json!({ "path" : path }));
                }
            }
        }
    }
}
pub(crate) async fn handle_window_new(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    let url = cmd
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or("about:blank");
    let same_profile = cmd
        .get("sameProfile")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let create_params = if same_profile {
        json!({ "url" : url, "newWindow" : true })
    } else {
        let context_result = mgr
            .client
            .send_command_no_params("Target.createBrowserContext", None)
            .await?;
        let context_id = context_result
            .get("browserContextId")
            .and_then(|v| v.as_str())
            .ok_or("Failed to create browser context")?
            .to_string();
        json!({ "url" : url, "browserContextId" : context_id, "newWindow" : true })
    };
    let create_result: super::super::cdp::types::CreateTargetResult = mgr
        .client
        .send_command_typed("Target.createTarget", &create_params, None)
        .await?;
    let attach: super::super::cdp::types::AttachToTargetResult = mgr
        .client
        .send_command_typed(
            "Target.attachToTarget",
            &super::super::cdp::types::AttachToTargetParams {
                target_id: create_result.target_id.clone(),
                flatten: true,
            },
            None,
        )
        .await?;
    mgr.add_page(super::super::browser::PageInfo {
        target_id: create_result.target_id.clone(),
        session_id: attach.session_id,
        url: url.to_string(),
        title: String::new(),
        target_type: "page".to_string(),
    });
    if let Some(viewport) = cmd.get("viewport") {
        let width = viewport
            .get("width")
            .and_then(|v| v.as_i64())
            .unwrap_or(1280) as i32;
        let height = viewport
            .get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(720) as i32;
        mgr.set_viewport(width, height, 1.0, false).await?;
        if let Some(ref server) = state.stream_server {
            server.set_viewport(width as u32, height as u32).await;
        }
    }
    let total = mgr.page_count();
    let index = total - 1;
    state.ref_map.clear();
    Ok(json!(
        { "index" : index, "total" : total, "url" : url, "targetId" : create_result
        .target_id, "sameProfile" : same_profile, }
    ))
}
pub(crate) async fn handle_diff_screenshot(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let baseline_path = cmd
        .get("baseline")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'baseline' parameter")?;
    let threshold = cmd.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.1);
    let options = ScreenshotOptions {
        selector: cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .map(String::from),
        path: None,
        full_page: cmd
            .get("fullPage")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        format: "png".to_string(),
        quality: None,
        annotate: false,
        output_dir: None,
    };
    let result = screenshot::take_screenshot(
        &mgr.client,
        &session_id,
        &state.ref_map,
        &options,
        &state.iframe_sessions,
    )
    .await?;
    let current_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.base64)
            .map_err(|e| format!("Failed to decode screenshot: {}", e))?;
    let baseline_bytes =
        std::fs::read(baseline_path).map_err(|e| format!("Failed to read baseline: {}", e))?;
    let result = diff::diff_screenshot(&baseline_bytes, &current_bytes, threshold)?;
    let output_path = cmd.get("output").and_then(|v| v.as_str());
    if let (Some(out_path), Some(ref diff_data)) = (output_path, &result.diff_image) {
        std::fs::write(out_path, diff_data)
            .map_err(|e| format!("Failed to write diff image: {}", e))?;
    }
    Ok(json!(
        { "match" : result.matched, "mismatchPercentage" : result
        .mismatch_percentage, "totalPixels" : result.total_pixels, "differentPixels"
        : result.different_pixels, "diffPath" : output_path, "dimensionMismatch" :
        result.dimension_mismatch, }
    ))
}
pub(crate) async fn handle_video_start(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let path = cmd
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' parameter")?;
    if state.recording_state.active {
        return Err("A recording is already in progress".to_string());
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    recording::recording_start(&mut state.recording_state, path)?;
    state
        .start_recording_task(mgr.client.clone(), session_id)
        .await?;
    Ok(json!(
        { "started" : true, "note" :
        "Video recording started. Use video_stop to save the recording." }
    ))
}
pub(crate) async fn handle_video_stop(state: &mut DaemonState) -> Result<Value, String> {
    if !state.recording_state.active {
        return Ok(json!(
            { "stopped" : false, "note" :
            "No video recording was started. Use recording_stop if you used recording_start."
            }
        ));
    }
    state.stop_recording_task().await?;
    recording::recording_stop(&mut state.recording_state)
}
/// Begin capturing network traffic for a later HAR export.
pub(crate) async fn handle_har_start(state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    mgr.client
        .send_command_no_params("Network.enable", Some(&session_id))
        .await?;
    for iframe_sid in state.iframe_sessions.values() {
        let _ = mgr
            .client
            .send_command_no_params("Network.enable", Some(iframe_sid.as_str()))
            .await;
    }
    state.har_recording = true;
    state.har_entries.clear();
    Ok(json!({ "started" : true }))
}
/// Stop HAR recording and write the captured requests to disk.
pub(crate) async fn handle_har_stop(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let path = har_output_path(cmd.get("path").and_then(|v| v.as_str()));
    state.har_recording = false;
    let entries: Vec<Value> = state.har_entries.drain(..).map(har_entry_to_json).collect();
    let request_count = entries.len();
    let browser = har_browser_metadata(state).await;
    let mut log = json!(
        { "version" : "1.2", "creator" : { "name" : "agent-browser", "version" :
        env!("CARGO_PKG_VERSION") }, "entries" : entries }
    );
    if let Some(browser) = browser {
        log["browser"] = browser;
    }
    let har = json!({ "log" : log });
    let har_str = serde_json::to_string_pretty(&har)
        .map_err(|e| format!("Failed to serialize HAR: {}", e))?;
    std::fs::write(&path, har_str).map_err(|e| format!("Failed to write HAR: {}", e))?;
    Ok(json!({ "path" : path, "requestCount" : request_count }))
}
/// Convert a `HarEntry` (collected from CDP events) into a HAR 1.2 entry object.
pub(crate) fn har_entry_to_json(e: HarEntry) -> Value {
    let started_date_time = har_wall_time_to_rfc3339(e.wall_time);
    let request_cookies = e
        .request_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("cookie"))
        .map(|(_, v)| har_parse_request_cookies(v))
        .unwrap_or_default();
    let query_string = har_parse_query_string(&e.url);
    let req_headers: Vec<Value> = e
        .request_headers
        .iter()
        .map(|(k, v)| json!({ "name" : k, "value" : v }))
        .collect();
    let resp_cookies: Vec<Value> = e
        .response_headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| {
            let name_value = v.split(';').next().unwrap_or("");
            let (name, value) = name_value.split_once('=').unwrap_or((name_value, ""));
            json!({ "name" : name.trim(), "value" : value.trim() })
        })
        .collect();
    let resp_headers: Vec<Value> = e
        .response_headers
        .iter()
        .map(|(k, v)| json!({ "name" : k, "value" : v }))
        .collect();
    let (timings, total_time) =
        har_compute_timings(e.cdp_timing.as_ref(), e.loading_finished_timestamp);
    let mime_type = if e.mime_type.is_empty() {
        "application/octet-stream".to_string()
    } else {
        e.mime_type
    };
    let post_content_type = e
        .request_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("text/plain")
        .to_string();
    let mut request = json!(
        { "method" : e.method, "url" : e.url, "httpVersion" : e.http_version, "cookies" :
        request_cookies, "headers" : req_headers, "queryString" : query_string,
        "headersSize" : - 1, "bodySize" : e.request_body_size, }
    );
    if let Some(body) = e.post_data {
        request["postData"] = json!({ "mimeType" : post_content_type, "text" : body });
    }
    json!(
        { "startedDateTime" : started_date_time, "time" : total_time, "request" :
        request, "response" : { "status" : e.status.unwrap_or(0), "statusText" : e
        .status_text, "httpVersion" : e.http_version, "cookies" : resp_cookies, "headers"
        : resp_headers, "content" : { "size" : e.response_body_size, "mimeType" :
        mime_type, }, "redirectURL" : e.redirect_url, "headersSize" : - 1, "bodySize" : e
        .response_body_size, }, "cache" : {}, "timings" : timings, "_resourceType" : e
        .resource_type, }
    )
}
/// Convert a CDP headers object (`{ "Name": "value", ... }`) into a flat
/// `Vec<(name, value)>` preserving insertion order.
pub(crate) fn har_extract_headers(headers_val: Option<&Value>) -> Vec<(String, String)> {
    headers_val
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        })
        .unwrap_or_default()
}
/// Map a CDP `response.protocol` value to an HTTP-version string as required
/// by the HAR spec (e.g. `"h2"` → `"HTTP/2.0"`).
pub(crate) fn har_cdp_protocol_to_http_version(protocol: &str) -> String {
    match protocol.to_ascii_lowercase().as_str() {
        "h2" => "HTTP/2.0".to_string(),
        "h3" => "HTTP/3.0".to_string(),
        "http/1.0" => "HTTP/1.0".to_string(),
        _ => "HTTP/1.1".to_string(),
    }
}
/// Parse query-string parameters from a URL into a HAR `queryString` array.
pub(crate) fn har_parse_query_string(url_str: &str) -> Vec<Value> {
    url::Url::parse(url_str)
        .map(|u| {
            u.query_pairs()
                .map(|(k, v)| json!({ "name" : k.as_ref(), "value" : v.as_ref() }))
                .collect()
        })
        .unwrap_or_default()
}
/// Parse a `Cookie: name1=val1; name2=val2` header value into HAR cookie objects.
pub(crate) fn har_parse_request_cookies(cookie_header: &str) -> Vec<Value> {
    cookie_header
        .split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some(json!({ "name" : name.trim(), "value" : value.trim() }))
        })
        .collect()
}
/// Compute HAR `timings` and total `time` (ms) from a CDP `ResourceTiming`
/// object and the optional `Network.loadingFinished` monotonic timestamp.
///
/// CDP timing values are milliseconds relative to `requestTime` (seconds since
/// browser start). A value of `-1` means the phase did not occur.
pub(crate) fn har_compute_timings(
    cdp_timing: Option<&Value>,
    loading_finished_ts: Option<f64>,
) -> (Value, f64) {
    let Some(t) = cdp_timing else {
        return (json!({ "send" : 0, "wait" : 0, "receive" : 0 }), 0.0);
    };
    let get = |key: &str| t.get(key).and_then(|v| v.as_f64()).unwrap_or(-1.0);
    let request_time = get("requestTime");
    let dns_start = get("dnsStart");
    let dns_end = get("dnsEnd");
    let connect_start = get("connectStart");
    let connect_end = get("connectEnd");
    let ssl_start = get("sslStart");
    let ssl_end = get("sslEnd");
    let send_start = get("sendStart");
    let send_end = get("sendEnd");
    let recv_headers_start = get("receiveHeadersStart");
    let recv_headers_end = get("receiveHeadersEnd");
    let dns = if dns_start >= 0.0 && dns_end >= 0.0 {
        dns_end - dns_start
    } else {
        -1.0
    };
    let connect = if connect_start >= 0.0 && connect_end >= 0.0 {
        connect_end - connect_start
    } else {
        -1.0
    };
    let ssl = if ssl_start >= 0.0 && ssl_end >= 0.0 {
        ssl_end - ssl_start
    } else {
        -1.0
    };
    let send = (send_end - send_start).max(0.0);
    let wait_end = if recv_headers_start >= 0.0 {
        recv_headers_start
    } else {
        recv_headers_end
    };
    let wait = if send_end >= 0.0 && wait_end >= send_end {
        wait_end - send_end
    } else {
        0.0
    };
    let receive = loading_finished_ts
        .filter(|_| request_time >= 0.0 && recv_headers_end >= 0.0)
        .map(|lf_ts| {
            let recv_start_abs = request_time + recv_headers_end / 1000.0;
            ((lf_ts - recv_start_abs) * 1000.0).max(0.0)
        })
        .unwrap_or(0.0);
    let blocked = if dns_start > 0.0 {
        dns_start
    } else if connect_start > 0.0 {
        connect_start
    } else if send_start > 0.0 {
        send_start
    } else {
        -1.0
    };
    let total: f64 = [
        if blocked > 0.0 { blocked } else { 0.0 },
        if dns >= 0.0 { dns } else { 0.0 },
        if connect >= 0.0 { connect } else { 0.0 },
        send,
        wait,
        receive,
    ]
    .iter()
    .sum();
    let mut timings = json!({ "send" : send, "wait" : wait, "receive" : receive });
    if blocked > 0.0 {
        timings["blocked"] = json!(blocked);
    }
    if dns >= 0.0 {
        timings["dns"] = json!(dns);
    }
    if connect >= 0.0 {
        timings["connect"] = json!(connect);
    }
    if ssl >= 0.0 {
        timings["ssl"] = json!(ssl);
    }
    (timings, total)
}
/// Format a Unix epoch timestamp (seconds, fractional) as RFC 3339 using the
/// `time` crate, e.g. `"2024-03-17T10:30:00.456Z"`.
pub(crate) fn har_wall_time_to_rfc3339(wall_time: f64) -> String {
    if wall_time > 0.0 {
        let nanos = (wall_time * 1_000_000_000.0).round() as i128;
        if let Ok(dt) = OffsetDateTime::from_unix_timestamp_nanos(nanos) {
            if let Ok(s) = dt.format(&Rfc3339) {
                return s;
            }
        }
    }
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
pub(crate) fn har_output_path(explicit_path: Option<&str>) -> String {
    match explicit_path {
        Some(path) => path.to_string(),
        None => {
            let dir = get_har_dir();
            let _ = std::fs::create_dir_all(&dir);
            dir.join(format!("har-{}.har", unix_timestamp_millis()))
                .to_string_lossy()
                .to_string()
        }
    }
}
pub(crate) fn get_har_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".agent-browser").join("tmp").join("har")
    } else {
        std::env::temp_dir().join("agent-browser").join("har")
    }
}
pub(crate) fn unix_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
pub(crate) async fn har_browser_metadata(state: &DaemonState) -> Option<Value> {
    let mgr = state.browser.as_ref()?;
    if !mgr.is_connection_alive().await {
        return None;
    }
    let version = mgr
        .client
        .send_command_no_params("Browser.getVersion", None)
        .await
        .ok()?;
    browser_metadata_from_version(&version)
}
pub(crate) fn browser_metadata_from_version(version: &Value) -> Option<Value> {
    let product = version.get("product").and_then(|v| v.as_str())?;
    let (name, browser_version) = product.split_once('/').unwrap_or((product, ""));
    Some(json!({ "name" : name, "version" : browser_version, }))
}
pub(crate) async fn resolve_fetch_paused(
    client: &CdpClient,
    domain_filter: Option<&DomainFilter>,
    routes: &[RouteEntry],
    origin_headers: &HashMap<String, HashMap<String, String>>,
    paused: &FetchPausedRequest,
) {
    let session_id = &paused.session_id;
    if let Some(filter) = domain_filter {
        if let Ok(parsed) = url::Url::parse(&paused.url) {
            let scheme = parsed.scheme();
            if scheme != "http" && scheme != "https" {
                if paused.resource_type.eq_ignore_ascii_case("document") {
                    let _ = client
                        .send_command(
                            "Fetch.failRequest",
                            Some(json!(
                                { "requestId" : paused.request_id, "errorReason" :
                                "BlockedByClient" }
                            )),
                            Some(session_id),
                        )
                        .await;
                } else {
                    let _ = client
                        .send_command(
                            "Fetch.continueRequest",
                            Some(json!({ "requestId" : paused.request_id })),
                            Some(session_id),
                        )
                        .await;
                }
                return;
            }
            if let Some(hostname) = parsed.host_str() {
                if !filter.is_allowed(hostname) {
                    if paused.resource_type.eq_ignore_ascii_case("document") {
                        let error_body = format!(
                            "<html><body><h1>Blocked</h1><p>Navigation to {} is not allowed by domain filter.</p></body></html>",
                            hostname
                        );
                        let encoded = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            error_body.as_bytes(),
                        );
                        let _ = client
                            .send_command(
                                "Fetch.fulfillRequest",
                                Some(json!(
                                    { "requestId" : paused.request_id, "responseCode" : 403,
                                    "responseHeaders" : [{ "name" : "Content-Type", "value" :
                                    "text/html" },], "body" : encoded, }
                                )),
                                Some(session_id),
                            )
                            .await;
                    } else {
                        let _ = client
                            .send_command(
                                "Fetch.failRequest",
                                Some(json!(
                                    { "requestId" : paused.request_id, "errorReason" :
                                    "BlockedByClient" }
                                )),
                                Some(session_id),
                            )
                            .await;
                    }
                    return;
                }
            }
        }
    }
    for route in routes {
        let matches = if route.url_pattern == "*" {
            true
        } else if route.url_pattern.contains('*') {
            let parts: Vec<&str> = route.url_pattern.split('*').collect();
            if parts.len() == 2 {
                paused.url.starts_with(parts[0]) && paused.url.ends_with(parts[1])
            } else {
                paused.url.contains(&route.url_pattern)
            }
        } else {
            paused.url.contains(&route.url_pattern)
        };
        if matches {
            if route.abort {
                let _ = client
                    .send_command(
                        "Fetch.failRequest",
                        Some(json!(
                            { "requestId" : paused.request_id, "errorReason" : "Failed"
                            }
                        )),
                        Some(session_id),
                    )
                    .await;
                return;
            }
            if let Some(ref resp) = route.response {
                let status = resp.status.unwrap_or(200);
                let body_str = resp.body.as_deref().unwrap_or("");
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    body_str.as_bytes(),
                );
                let mut headers = vec![];
                if let Some(ct) = &resp.content_type {
                    headers.push(json!({ "name" : "Content-Type", "value" : ct }));
                }
                if let Some(h) = &resp.headers {
                    for (k, v) in h {
                        headers.push(json!({ "name" : k, "value" : v }));
                    }
                }
                let _ = client
                    .send_command(
                        "Fetch.fulfillRequest",
                        Some(json!(
                            { "requestId" : paused.request_id, "responseCode" : status,
                            "responseHeaders" : headers, "body" : encoded, }
                        )),
                        Some(session_id),
                    )
                    .await;
                return;
            }
        }
    }
    let extra = url::Url::parse(&paused.url)
        .ok()
        .map(|u| u.origin().ascii_serialization())
        .and_then(|o| origin_headers.get(&o));
    if let Some(extra_headers) = extra {
        let mut combined: Vec<Value> = Vec::new();
        if let Some(ref orig) = paused.request_headers {
            for (k, v) in orig {
                if !extra_headers.keys().any(|ek| ek.eq_ignore_ascii_case(k)) {
                    if let Some(s) = v.as_str() {
                        combined.push(json!({ "name" : k, "value" : s }));
                    }
                }
            }
        }
        for (k, v) in extra_headers {
            combined.push(json!({ "name" : k, "value" : v }));
        }
        let _ = client
            .send_command(
                "Fetch.continueRequest",
                Some(json!({ "requestId" : paused.request_id, "headers" : combined })),
                Some(session_id),
            )
            .await;
    } else {
        let _ = client
            .send_command(
                "Fetch.continueRequest",
                Some(json!({ "requestId" : paused.request_id })),
                Some(session_id),
            )
            .await;
    }
}
/// Build the Fetch.enable patterns list from current routes, domain filter,
/// and origin headers state.  When domain filtering or origin-scoped headers
/// are active a wildcard pattern is included so all requests are intercepted.
pub(crate) async fn build_fetch_patterns(state: &DaemonState) -> Vec<Value> {
    let routes = state.routes.read().await;
    let mut patterns: Vec<Value> = routes
        .iter()
        .map(|r| json!({ "urlPattern" : r.url_pattern }))
        .collect();
    let has_domain_filter = state.domain_filter.read().await.is_some();
    let has_origin_headers = !state.origin_headers.read().await.is_empty();
    let has_proxy_creds = state.proxy_credentials.read().await.is_some();
    if (has_domain_filter || has_origin_headers || has_proxy_creds)
        && !patterns.iter().any(|p| p["urlPattern"] == "*")
    {
        patterns.push(json!({ "urlPattern" : "*" }));
    }
    patterns
}
/// Build the full Fetch.enable params object, including `handleAuthRequests`
/// when proxy credentials are configured.
pub(crate) async fn build_fetch_enable_params(state: &DaemonState, patterns: Vec<Value>) -> Value {
    let has_proxy_creds = state.proxy_credentials.read().await.is_some();
    if has_proxy_creds {
        json!({ "patterns" : patterns, "handleAuthRequests" : true })
    } else {
        json!({ "patterns" : patterns })
    }
}
pub(crate) async fn handle_route(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let url_pattern = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' parameter")?
        .to_string();
    let abort = cmd.get("abort").and_then(|v| v.as_bool()).unwrap_or(false);
    let response = cmd.get("response").and_then(|v| {
        if v.is_null() {
            return None;
        }
        Some(RouteResponse {
            status: v.get("status").and_then(|s| s.as_u64()).map(|s| s as u16),
            body: v.get("body").and_then(|s| s.as_str()).map(String::from),
            content_type: v
                .get("contentType")
                .and_then(|s| s.as_str())
                .map(String::from),
            headers: v.get("headers").and_then(|h| {
                h.as_object().map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
            }),
        })
    });
    {
        let mut routes = state.routes.write().await;
        routes.push(RouteEntry {
            url_pattern: url_pattern.clone(),
            response,
            abort,
        });
    }
    let patterns = build_fetch_patterns(state).await;
    let params = build_fetch_enable_params(state, patterns).await;
    mgr.client
        .send_command("Fetch.enable", Some(params), Some(&session_id))
        .await?;
    Ok(json!({ "routed" : url_pattern }))
}
pub(crate) async fn handle_unroute(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let url = cmd.get("url").and_then(|v| v.as_str());
    {
        let mut routes = state.routes.write().await;
        match url {
            Some(pattern) => {
                routes.retain(|r| r.url_pattern != pattern);
            }
            None => {
                routes.clear();
            }
        }
    }
    let patterns = build_fetch_patterns(state).await;
    if patterns.is_empty() {
        mgr.client
            .send_command("Fetch.disable", None, Some(&session_id))
            .await?;
    } else {
        let params = build_fetch_enable_params(state, patterns).await;
        mgr.client
            .send_command("Fetch.enable", Some(params), Some(&session_id))
            .await?;
    }
    let label = url.unwrap_or("all");
    Ok(json!({ "unrouted" : label }))
}
pub(crate) fn matches_status_filter(status: Option<i64>, filter: &str) -> bool {
    let Some(code) = status else { return false };
    let f = filter.to_lowercase();
    if let Ok(exact) = f.parse::<i64>() {
        return code == exact;
    }
    if f.len() == 3 && f.ends_with("xx") {
        if let Ok(prefix) = f[..1].parse::<i64>() {
            return code / 100 == prefix;
        }
    }
    if let Some((lo, hi)) = f.split_once('-') {
        if let (Ok(lo), Ok(hi)) = (lo.parse::<i64>(), hi.parse::<i64>()) {
            return code >= lo && code <= hi;
        }
    }
    false
}
pub(crate) async fn handle_requests(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    if cmd.get("clear").and_then(|v| v.as_bool()).unwrap_or(false) {
        state.tracked_requests.clear();
        return Ok(json!({ "cleared" : true }));
    }
    if !state.request_tracking {
        state.request_tracking = true;
        if let Some(ref mgr) = state.browser {
            if let Ok(session_id) = mgr.active_session_id() {
                let _ = mgr
                    .client
                    .send_command_no_params("Network.enable", Some(session_id))
                    .await;
            }
        }
    }
    let filter = cmd.get("filter").and_then(|v| v.as_str());
    let type_filter = cmd.get("type").and_then(|v| v.as_str());
    let method_filter = cmd.get("method").and_then(|v| v.as_str());
    let status_filter = cmd.get("status").and_then(|v| v.as_str());
    let type_list: Vec<String> = type_filter
        .map(|t| t.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default();
    let requests: Vec<&TrackedRequest> = state
        .tracked_requests
        .iter()
        .filter(|r| {
            if let Some(f) = filter {
                if !r.url.contains(f) {
                    return false;
                }
            }
            if !type_list.is_empty() && !type_list.contains(&r.resource_type.to_lowercase()) {
                return false;
            }
            if let Some(m) = method_filter {
                if !r.method.eq_ignore_ascii_case(m) {
                    return false;
                }
            }
            if let Some(s) = status_filter {
                if !matches_status_filter(r.status, s) {
                    return false;
                }
            }
            true
        })
        .collect();
    Ok(json!({ "requests" : requests }))
}
pub(crate) async fn handle_request_detail(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let request_id = cmd
        .get("requestId")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'requestId' parameter")?;
    let entry = state
        .tracked_requests
        .iter()
        .find(|r| r.request_id == request_id)
        .ok_or("Request not found")?;
    let mut result = serde_json::to_value(entry).unwrap_or(json!({}));
    if let Some(ref mgr) = state.browser {
        if let Ok(session_id) = mgr.active_session_id() {
            if let Ok(body_result) = mgr
                .client
                .send_command(
                    "Network.getResponseBody",
                    Some(json!({ "requestId" : request_id })),
                    Some(session_id),
                )
                .await
            {
                let base64_encoded = body_result
                    .get("base64Encoded")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let body = body_result
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if base64_encoded {
                    result["responseBody"] = json!(format!("[base64, {} chars]", body.len()));
                } else {
                    result["responseBody"] = json!(body);
                }
            }
        }
    }
    Ok(result)
}
pub(crate) async fn handle_http_credentials(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let username = cmd
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'username' parameter")?;
    let password = cmd
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'password' parameter")?;
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        format!("{}:{}", username, password),
    );
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
    network::set_extra_headers(&mgr.client, &session_id, &headers).await?;
    Ok(json!({ "set" : true }))
}
/// Wait for any selector in `selectors` to appear and return the first match.
///
/// This is used by `auth_login` auto-detection so SPA login forms can render
/// after initial navigation without requiring global network-idle.
pub(crate) async fn wait_for_any_selector(
    client: &super::super::cdp::client::CdpClient,
    session_id: &str,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<String, String> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    loop {
        for selector in selectors {
            let expression = format!(
                r#"(() => {{
                    const el = document.querySelector({sel});
                    if (!el) return false;

                    const r = el.getBoundingClientRect();
                    const s = window.getComputedStyle(el);
                    const opacity = parseFloat(s.opacity || '1');
                    const isVisible =
                        r.width > 0 &&
                        r.height > 0 &&
                        s.visibility !== 'hidden' &&
                        s.display !== 'none' &&
                        (!Number.isFinite(opacity) || opacity > 0);

                    if (!isVisible) return false;
                    if (el.matches(':disabled')) return false;

                    if (el instanceof HTMLInputElement && el.type === 'hidden') return false;
                    if ((el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) && el.readOnly) return false;

                    return true;
                }})()"#,
                sel = serde_json::to_string(selector).unwrap_or_default()
            );
            let result: super::super::cdp::types::EvaluateResult = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &super::super::cdp::types::EvaluateParams {
                        expression,
                        return_by_value: Some(true),
                        await_promise: Some(true),
                    },
                    Some(session_id),
                )
                .await?;
            if result
                .result
                .value
                .as_ref()
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return Ok((*selector).to_string());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Wait timed out after {}ms", timeout_ms));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(
            AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        ))
        .await;
    }
}
pub(crate) async fn handle_auth_save(cmd: &Value) -> Result<Value, String> {
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name'")?;
    let url = cmd
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url'")?;
    let username = cmd
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'username'")?;
    let password = cmd
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'password'")?;
    let username_selector = cmd.get("usernameSelector").and_then(|v| v.as_str());
    let password_selector = cmd.get("passwordSelector").and_then(|v| v.as_str());
    let submit_selector = cmd.get("submitSelector").and_then(|v| v.as_str());
    auth::auth_save(
        name,
        url,
        username,
        password,
        username_selector,
        password_selector,
        submit_selector,
    )
}
pub(crate) async fn handle_auth_login(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let name = cmd
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name'")?;
    let cred = auth::credentials_get_full(name)?;
    if cred.url.is_empty() {
        return Err("Credential has no URL".to_string());
    }
    let url = cred.url;
    let username = cred.username;
    let password = cred.password;
    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
    mgr.navigate(&url, AUTH_LOGIN_WAIT_UNTIL).await?;
    let session_id = mgr.active_session_id()?.to_string();
    let auth_timeout_ms = mgr.default_timeout_ms();
    let preferred_user_selectors = [
        "input[type=email]",
        "input[name=email]",
        "input[id=email]",
        "input[autocomplete=email]",
        "input[autocomplete=username]",
        "input[name=username]",
        "input[name*=email i]",
        "input[name*=user i]",
        "input[id*=email i]",
        "input[id*=user i]",
        "input[type=text][name*=email i]",
        "input[type=text][name*=user i]",
        "input[type=text][id*=email i]",
        "input[type=text][id*=user i]",
        "input[type=text][autocomplete=email]",
        "input[type=text][autocomplete=username]",
    ];
    let fallback_user_selectors = ["input[type=text]", "input:not([type])"];
    let auto_submit_selectors = [
        "button[type=submit]",
        "input[type=submit]",
        "button:not([type])",
    ];
    let username_sel = cmd
        .get("usernameSelector")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or(cred.username_selector);
    let password_sel = cmd
        .get("passwordSelector")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or(cred.password_selector);
    let submit_sel = cmd
        .get("submitSelector")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or(cred.submit_selector);
    let user_sel = if let Some(s) = username_sel {
        wait_for_selector(&mgr.client, &session_id, &s, "visible", auth_timeout_ms)
            .await
            .map_err(|_| format!("Timed out waiting for username selector '{}'", s))?;
        s
    } else {
        let preferred_window_ms = auth_timeout_ms.min(AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS);
        let fallback_window_ms = auth_timeout_ms.saturating_sub(preferred_window_ms);
        match wait_for_any_selector(
            &mgr.client,
            &session_id,
            &preferred_user_selectors,
            preferred_window_ms,
        )
        .await
        {
            Ok(selector) => selector,
            Err(_) => {
                if fallback_window_ms == 0 {
                    return Err(format!(
                        "Timed out waiting for username field (preferred selectors for {}ms: {})",
                        preferred_window_ms,
                        preferred_user_selectors.join(", ")
                    ));
                }
                wait_for_any_selector(
                        &mgr.client,
                        &session_id,
                        &fallback_user_selectors,
                        fallback_window_ms,
                    )
                    .await
                    .map_err(|_| {
                        format!(
                            "Timed out waiting for username field (preferred selectors for {}ms: {}; fallback selectors for {}ms: {})",
                            preferred_window_ms, preferred_user_selectors.join(", "),
                            fallback_window_ms, fallback_user_selectors.join(", ")
                        )
                    })?
            }
        }
    };
    interaction::fill(
        &mgr.client,
        &session_id,
        &state.ref_map,
        &user_sel,
        &username,
        &state.iframe_sessions,
    )
    .await?;
    let pass_sel = password_sel.unwrap_or_else(|| "input[type=password]".to_string());
    wait_for_selector(
        &mgr.client,
        &session_id,
        &pass_sel,
        "visible",
        auth_timeout_ms,
    )
    .await
    .map_err(|_| format!("Timed out waiting for password selector '{}'", pass_sel))?;
    interaction::fill(
        &mgr.client,
        &session_id,
        &state.ref_map,
        &pass_sel,
        &password,
        &state.iframe_sessions,
    )
    .await?;
    let sub_sel = if let Some(s) = submit_sel {
        wait_for_selector(&mgr.client, &session_id, &s, "visible", auth_timeout_ms)
            .await
            .map_err(|_| format!("Timed out waiting for submit selector '{}'", s))?;
        s
    } else {
        wait_for_any_selector(
            &mgr.client,
            &session_id,
            &auto_submit_selectors,
            auth_timeout_ms,
        )
        .await
        .map_err(|_| {
            format!(
                "Timed out waiting for submit button (tried selectors: {})",
                auto_submit_selectors.join(", ")
            )
        })?
    };
    interaction::click(
        &mgr.client,
        &session_id,
        &state.ref_map,
        &sub_sel,
        "left",
        1,
        &state.iframe_sessions,
    )
    .await?;
    let mut rx = mgr.client.subscribe();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
    let mut navigated = false;
    loop {
        let result = tokio::time::timeout_at(deadline, rx.recv()).await;
        match result {
            Ok(Ok(event)) => {
                if event.session_id.as_deref() == Some(&session_id) {
                    match event.method.as_str() {
                        "Page.frameNavigated" | "Page.loadEventFired" => {
                            navigated = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }
    if !navigated {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    Ok(json!({ "loggedIn" : true, "name" : name }))
}
pub(crate) async fn handle_swipe(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    if let Some(ref appium) = state.appium {
        if state.browser.is_none() {
            let start_x = cmd.get("startX").and_then(|v| v.as_f64()).unwrap_or(200.0);
            let start_y = cmd.get("startY").and_then(|v| v.as_f64()).unwrap_or(400.0);
            let end_x = cmd.get("endX").and_then(|v| v.as_f64()).unwrap_or(200.0);
            let end_y = cmd.get("endY").and_then(|v| v.as_f64()).unwrap_or(100.0);
            if let Some(direction) = cmd.get("direction").and_then(|v| v.as_str()) {
                let distance = cmd
                    .get("distance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(300.0);
                let (dx, dy) = match direction {
                    "up" => (0.0, -distance),
                    "down" => (0.0, distance),
                    "left" => (-distance, 0.0),
                    "right" => (distance, 0.0),
                    _ => (0.0, -distance),
                };
                let actual_end_x = start_x + dx;
                let actual_end_y = start_y + dy;
                let duration = cmd.get("duration").and_then(|v| v.as_u64()).unwrap_or(800);
                appium
                    .swipe(start_x, start_y, actual_end_x, actual_end_y, duration)
                    .await?;
                return Ok(json!({ "swiped" : direction }));
            }
            let duration = cmd.get("duration").and_then(|v| v.as_u64()).unwrap_or(800);
            appium
                .swipe(start_x, start_y, end_x, end_y, duration)
                .await?;
            return Ok(json!(
                { "swiped" : true, "from" : [start_x, start_y], "to" : [end_x, end_y]
                }
            ));
        }
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let start_x = cmd.get("startX").and_then(|v| v.as_f64()).unwrap_or(200.0);
    let start_y = cmd.get("startY").and_then(|v| v.as_f64()).unwrap_or(400.0);
    let end_x = cmd.get("endX").and_then(|v| v.as_f64()).unwrap_or(200.0);
    let end_y = cmd.get("endY").and_then(|v| v.as_f64()).unwrap_or(100.0);
    if let Some(direction) = cmd.get("direction").and_then(|v| v.as_str()) {
        let distance = cmd
            .get("distance")
            .and_then(|v| v.as_f64())
            .unwrap_or(300.0);
        let (dx, dy) = match direction {
            "up" => (0.0, -distance),
            "down" => (0.0, distance),
            "left" => (-distance, 0.0),
            "right" => (distance, 0.0),
            _ => (0.0, -distance),
        };
        let cx = start_x;
        let cy = start_y;
        mgr.client
            .send_command(
                "Input.dispatchTouchEvent",
                Some(json!(
                    { "type" : "touchStart", "touchPoints" : [{ "x" : cx, "y" : cy }]
                    }
                )),
                Some(&session_id),
            )
            .await?;
        let steps = 10;
        for i in 1..=steps {
            let x = cx + dx * (i as f64) / (steps as f64);
            let y = cy + dy * (i as f64) / (steps as f64);
            mgr.client
                .send_command(
                    "Input.dispatchTouchEvent",
                    Some(json!(
                        { "type" : "touchMove", "touchPoints" : [{ "x" : x, "y" : y
                        }] }
                    )),
                    Some(&session_id),
                )
                .await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
        }
        mgr.client
            .send_command(
                "Input.dispatchTouchEvent",
                Some(json!({ "type" : "touchEnd", "touchPoints" : [] })),
                Some(&session_id),
            )
            .await?;
        return Ok(json!({ "swiped" : direction }));
    }
    mgr.client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(json!(
                { "type" : "touchStart", "touchPoints" : [{ "x" : start_x, "y" :
                start_y }] }
            )),
            Some(&session_id),
        )
        .await?;
    let steps = 10;
    for i in 1..=steps {
        let x = start_x + (end_x - start_x) * (i as f64) / (steps as f64);
        let y = start_y + (end_y - start_y) * (i as f64) / (steps as f64);
        mgr.client
            .send_command(
                "Input.dispatchTouchEvent",
                Some(json!(
                    { "type" : "touchMove", "touchPoints" : [{ "x" : x, "y" : y }] }
                )),
                Some(&session_id),
            )
            .await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
    }
    mgr.client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(json!({ "type" : "touchEnd", "touchPoints" : [] })),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "swiped" : true, "from" : [start_x, start_y], "to" : [end_x, end_y] }))
}
pub(crate) async fn handle_device_list() -> Result<Value, String> {
    #[cfg(target_os = "macos")]
    {
        use super::webdriver::ios;
        let devices = ios::list_all_devices()?;
        Ok(ios::to_device_json(&devices))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("device_list is only available on macOS with Xcode".to_string())
    }
}
pub(crate) fn mouse_button_mask(button: &str) -> i32 {
    match button {
        "left" => 1,
        "right" => 2,
        "middle" => 4,
        "back" => 8,
        "forward" => 16,
        _ => 0,
    }
}
pub(crate) fn primary_button_from_mask(buttons: i32) -> &'static str {
    if buttons & 1 != 0 {
        "left"
    } else if buttons & 2 != 0 {
        "right"
    } else if buttons & 4 != 0 {
        "middle"
    } else if buttons & 8 != 0 {
        "back"
    } else if buttons & 16 != 0 {
        "forward"
    } else {
        "none"
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_mouse_event_params(
    mouse_state: &mut MouseState,
    event_type: &str,
    x: Option<f64>,
    y: Option<f64>,
    button: Option<&str>,
    buttons: Option<i32>,
    click_count: Option<i32>,
    delta_x: Option<f64>,
    delta_y: Option<f64>,
    modifiers: Option<i32>,
) -> DispatchMouseEventParams {
    let x = x.unwrap_or(mouse_state.x);
    let y = y.unwrap_or(mouse_state.y);
    mouse_state.x = x;
    mouse_state.y = y;
    let mut next_buttons = buttons.unwrap_or(mouse_state.buttons);
    if buttons.is_none() {
        match event_type {
            "mousePressed" => {
                next_buttons |= mouse_button_mask(button.unwrap_or("left"));
            }
            "mouseReleased" => {
                next_buttons &= !mouse_button_mask(button.unwrap_or("left"));
            }
            _ => {}
        }
    }
    mouse_state.buttons = next_buttons;
    DispatchMouseEventParams {
        event_type: event_type.to_string(),
        x,
        y,
        button: Some(
            button
                .unwrap_or(primary_button_from_mask(next_buttons))
                .to_string(),
        ),
        buttons: Some(next_buttons),
        click_count,
        delta_x,
        delta_y,
        modifiers,
    }
}
pub(crate) async fn handle_input_mouse(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let event_type = cmd
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("mouseMoved");
    let params = build_mouse_event_params(
        &mut state.mouse_state,
        event_type,
        cmd.get("x").and_then(|v| v.as_f64()),
        cmd.get("y").and_then(|v| v.as_f64()),
        cmd.get("button").and_then(|v| v.as_str()),
        cmd.get("buttons")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        cmd.get("clickCount")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        cmd.get("deltaX").and_then(|v| v.as_f64()),
        cmd.get("deltaY").and_then(|v| v.as_f64()),
        cmd.get("modifiers")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    );
    mgr.client
        .send_command_typed::<_, Value>("Input.dispatchMouseEvent", &params, Some(&session_id))
        .await?;
    Ok(json!({ "dispatched" : event_type }))
}
pub(crate) async fn handle_input_keyboard(
    cmd: &Value,
    state: &DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let event_type = cmd
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("keyDown");
    let mut params = json!({ "type" : event_type });
    for key in &["key", "code", "text"] {
        if let Some(v) = cmd.get(*key) {
            params[*key] = v.clone();
        }
    }
    mgr.client
        .send_command("Input.dispatchKeyEvent", Some(params), Some(&session_id))
        .await?;
    Ok(json!({ "dispatched" : event_type }))
}
pub(crate) async fn handle_input_touch(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let event_type = cmd
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("touchStart");
    mgr.client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(json!(
                { "type" : event_type, "touchPoints" : cmd.get("touchPoints")
                .unwrap_or(& json!([])), }
            )),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "dispatched" : event_type }))
}
pub(crate) async fn handle_keydown(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let key = cmd
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'key' parameter")?;
    mgr.client
        .send_command(
            "Input.dispatchKeyEvent",
            Some(json!({ "type" : "keyDown", "key" : key })),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "keydown" : key }))
}
pub(crate) async fn handle_keyup(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let key = cmd
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'key' parameter")?;
    mgr.client
        .send_command(
            "Input.dispatchKeyEvent",
            Some(json!({ "type" : "keyUp", "key" : key })),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "keyup" : key }))
}
pub(crate) async fn handle_inserttext(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let text = cmd
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'text' parameter")?;
    mgr.client
        .send_command(
            "Input.insertText",
            Some(json!({ "text" : text })),
            Some(&session_id),
        )
        .await?;
    Ok(json!({ "inserted" : true }))
}
pub(crate) async fn handle_mousemove(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let params = build_mouse_event_params(
        &mut state.mouse_state,
        "mouseMoved",
        Some(x),
        Some(y),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    mgr.client
        .send_command_typed::<_, Value>("Input.dispatchMouseEvent", &params, Some(&session_id))
        .await?;
    Ok(json!({ "moved" : true }))
}
pub(crate) async fn handle_mousedown(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let button = cmd.get("button").and_then(|v| v.as_str()).unwrap_or("left");
    let params = build_mouse_event_params(
        &mut state.mouse_state,
        "mousePressed",
        None,
        None,
        Some(button),
        None,
        Some(1),
        None,
        None,
        None,
    );
    mgr.client
        .send_command_typed::<_, Value>("Input.dispatchMouseEvent", &params, Some(&session_id))
        .await?;
    Ok(json!({ "pressed" : true }))
}
pub(crate) async fn handle_mouseup(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let button = cmd.get("button").and_then(|v| v.as_str()).unwrap_or("left");
    let params = build_mouse_event_params(
        &mut state.mouse_state,
        "mouseReleased",
        None,
        None,
        Some(button),
        None,
        Some(1),
        None,
        None,
        None,
    );
    mgr.client
        .send_command_typed::<_, Value>("Input.dispatchMouseEvent", &params, Some(&session_id))
        .await?;
    Ok(json!({ "released" : true }))
}

pub(crate) struct ConfirmationExecution {
    action: String,
    command: Value,
    policy: Option<ActionPolicy>,
    confirm_actions: Option<ConfirmActions>,
}

impl ConfirmationExecution {
    pub(crate) fn command(&self) -> &Value {
        &self.command
    }

    pub(crate) fn complete(self, state: &mut DaemonState, result: Value) -> Value {
        state.policy = self.policy;
        state.confirm_actions = self.confirm_actions;
        json!({ "confirmed": true, "action": self.action, "result": result })
    }
}

pub(crate) fn begin_confirmation(state: &mut DaemonState) -> Result<ConfirmationExecution, String> {
    let pending = state
        .pending_confirmation
        .take()
        .ok_or("No pending confirmation")?;
    Ok(ConfirmationExecution {
        action: pending.action,
        command: pending.cmd,
        policy: state.policy.take(),
        confirm_actions: state.confirm_actions.take(),
    })
}

pub(crate) async fn handle_deny(_cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let pending = state
        .pending_confirmation
        .take()
        .ok_or("No pending confirmation")?;
    Ok(json!({ "denied": true, "action": pending.action }))
}
