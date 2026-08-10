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
    use crate::native::browser_lifecycle::persist_service_owned_tab_new;
    use crate::native::service_diagnostics::truncate_utf8;
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
}
pub(crate) use action_commands::*;
