#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::browser::{
        should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo,
        ProcessExitObservation, WaitUntil,
    };
    use crate::native::browser_wait::{wait_for_function, wait_for_url};
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use serde_json::{json, Map, Value};
    use std::time::{Duration, Instant};
    pub(crate) async fn handle_waitforurl(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
    pub(crate) async fn handle_frame(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
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
}
pub(crate) use action_commands::*;
