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
    pub(crate) async fn handle_set_media(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
            { "device" : name, "width" : width, "height" : height,
            "deviceScaleFactor" : scale, "mobile" : mobile, }
        ))
    }
    pub(crate) async fn handle_viewport(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let width = cmd.get("width").and_then(|v| v.as_i64()).unwrap_or(1280) as i32;
        let height = cmd.get("height").and_then(|v| v.as_i64()).unwrap_or(720) as i32;
        let scale = cmd
            .get("deviceScaleFactor")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let mobile = cmd.get("mobile").and_then(|v| v.as_bool()).unwrap_or(false);
        mgr.set_viewport(width, height, scale, mobile).await?;
        if let Some(ref server) = state.stream_server {
            server.set_viewport(width as u32, height as u32).await;
        }
        Ok(json!(
            { "width" : width, "height" : height, "deviceScaleFactor" : scale, "mobile" :
            mobile }
        ))
    }
    pub(crate) async fn handle_user_agent(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let ua = cmd
            .get("userAgent")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'userAgent' parameter")?;
        mgr.set_user_agent(ua).await?;
        Ok(json!({ "userAgent" : ua }))
    }
}
pub(crate) use action_commands::*;
