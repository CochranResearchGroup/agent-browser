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
    pub(crate) async fn handle_geolocation(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
    pub(crate) async fn handle_permissions(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
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
}
pub(crate) use action_commands::*;
