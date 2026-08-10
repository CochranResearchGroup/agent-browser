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
    use crate::native::service_probe::handle_bounded_service_evaluate;
    pub(crate) fn command_evaluation_timeout_ms(cmd: &Value) -> Option<u64> {
        cmd.get("jobTimeoutMs")
            .and_then(Value::as_u64)
            .filter(|timeout_ms| *timeout_ms > 0)
    }
    pub(crate) async fn handle_evaluate(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
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
}
pub(crate) use action_commands::*;
