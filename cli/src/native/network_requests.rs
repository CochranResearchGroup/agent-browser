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
    use crate::native::network::matches_status_filter;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use serde_json::{json, Map, Value};
    pub(crate) async fn handle_requests(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
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
}
pub(crate) use action_commands::*;
