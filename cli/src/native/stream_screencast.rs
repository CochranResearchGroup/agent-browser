#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
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
    pub(crate) async fn handle_screencast_start(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        if state.screencasting {
            return Err("Screencast already active".to_string());
        }
        let (default_w, default_h) = if let Some(ref server) = state.stream_server {
            server.viewport().await
        } else {
            (1280, 720)
        };
        let format = cmd.get("format").and_then(|v| v.as_str()).unwrap_or("jpeg");
        let quality = cmd.get("quality").and_then(|v| v.as_i64()).unwrap_or(80) as i32;
        let max_width = cmd
            .get("maxWidth")
            .and_then(|v| v.as_i64())
            .unwrap_or(default_w as i64) as i32;
        let max_height = cmd
            .get("maxHeight")
            .and_then(|v| v.as_i64())
            .unwrap_or(default_h as i64) as i32;
        stream::start_screencast(
            &mgr.client,
            &session_id,
            format,
            quality,
            max_width,
            max_height,
        )
        .await?;
        state.screencasting = true;
        if let Some(ref server) = state.stream_server {
            server.set_screencasting(true).await;
            server
                .broadcast_status(
                    true,
                    true,
                    max_width as u32,
                    max_height as u32,
                    &state.engine,
                )
                .await;
        }
        Ok(json!({ "started" : true }))
    }
    pub(crate) async fn handle_screencast_stop(state: &mut DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?;
        if !state.screencasting {
            return Err("No screencast active".to_string());
        }
        stream::stop_screencast(&mgr.client, session_id).await?;
        state.screencasting = false;
        if let Some(ref server) = state.stream_server {
            server.set_screencasting(false).await;
            let (vw, vh) = server.viewport().await;
            server
                .broadcast_status(true, false, vw, vh, &state.engine)
                .await;
        }
        Ok(json!({ "stopped" : true }))
    }
}
pub(crate) use service_commands::*;
