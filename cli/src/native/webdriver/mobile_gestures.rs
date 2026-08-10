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
    pub(crate) async fn handle_swipe(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
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
                    { "swiped" : true, "from" : [start_x, start_y], "to" : [end_x,
                    end_y] }
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
                        { "type" : "touchStart", "touchPoints" : [{ "x" : cx, "y" :
                        cy }] }
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
        Ok(json!(
            { "swiped" : true, "from" : [start_x, start_y], "to" : [end_x, end_y] }
        ))
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
}
pub(crate) use action_commands::*;
