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
    use crate::native::interaction;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use serde_json::{json, Map, Value};
    use std::time::{Duration, Instant};
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
    pub(crate) async fn handle_evalhandle(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
                        { "type" : "mouseMoved", "x" : cx, "y" : cy, "button" :
                        "left", "buttons" : 1 }
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
                    { "type" : "mouseReleased", "x" : tx, "y" : ty, "button" :
                    "left", "buttons" : 0, "clickCount" : 1 }
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
    pub(crate) async fn handle_multiselect(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
}
pub(crate) use action_commands::*;
