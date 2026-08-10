#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::common::*;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
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
    pub(crate) async fn handle_input_touch(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
    pub(crate) async fn handle_inserttext(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
    pub(crate) async fn handle_mouseup(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
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
}
pub(crate) use action_commands::*;
