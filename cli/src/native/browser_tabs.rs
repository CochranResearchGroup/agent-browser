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
    use crate::native::browser_lifecycle::{
        persist_service_owned_tab_new, tab_new_shared_acquisition_evidence,
    };
    use crate::native::cdp::types::{
        AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
        DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
        TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use serde_json::{json, Map, Value};
    pub(crate) async fn handle_tab_list(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let verbose = cmd
            .get("verbose")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tabs = mgr.tab_list(verbose);
        Ok(json!({ "tabs" : tabs }))
    }
    pub(crate) fn handle_browser_pid(state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        Ok(json!({ "pid" : mgr.browser_pid().or(state.attached_browser_pid) }))
    }
    pub(crate) async fn handle_tab_new(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        let url = cmd.get("url").and_then(|v| v.as_str());
        state.ref_map.clear();
        state.iframe_sessions.clear();
        state.active_frame_id = None;
        let mut result = mgr.tab_new(url).await?;
        if let Some(object) = result.as_object_mut() {
            object.insert(
                "browserId".to_string(),
                json!(service_browser_id(&state.session_id)),
            );
            object.insert("sessionId".to_string(), json!(state.session_id.clone()));
            let tab_id = object
                .get("targetId")
                .and_then(|value| value.as_str())
                .map(|target_id| format!("target:{target_id}"))
                .or_else(|| {
                    object
                        .get("index")
                        .and_then(|value| value.as_u64())
                        .map(|index| format!("tab-index:{index}"))
                })
                .unwrap_or_else(|| format!("session:{}:active-tab", state.session_id));
            let target_id = object.get("targetId").cloned().unwrap_or(Value::Null);
            let current_url = object.get("url").cloned().unwrap_or(Value::Null);
            let title = object.get("title").cloned().unwrap_or(Value::Null);
            if let Some(runtime_profile) = mgr.runtime_profile_name() {
                object.insert("runtimeProfile".to_string(), json!(runtime_profile));
                object.insert("profileId".to_string(), json!(runtime_profile));
            }
            let profile_id = object.get("profileId").cloned().unwrap_or(Value::Null);
            object.insert(
                "sharedAcquisition".to_string(),
                tab_new_shared_acquisition_evidence(cmd, &state.session_id, profile_id.clone()),
            );
            let service_tab_handle = json!(
                { "browserId" : service_browser_id(& state.session_id), "sessionName" :
                state.session_id.clone(), "tabId" : tab_id, "targetId" : target_id, "url"
                : current_url, "title" : title, "profileId" : profile_id.clone(),
                "profileOrigin" : "agent_browser_owned", "leaseId" : state.session_id
                .clone(), "leaseState" : "shared", "cleanupPolicy" : "detach",
                "leaseHeartbeatExpected" : true, "ownerSessionId" : state.session_id
                .clone(), "jobId" : Value::Null, "traceFilter" : { "browserId" :
                service_browser_id(& state.session_id), "profileId" : profile_id.clone(),
                "sessionId" : state.session_id.clone(), "serviceName" :
                optional_command_string(cmd, "serviceName"), "agentName" :
                optional_command_string(cmd, "agentName"), "taskName" :
                optional_command_string(cmd, "taskName"), }, "valid" : true,
                "staleReason" : Value::Null, }
            );
            persist_service_owned_tab_new(
                cmd,
                &state.session_id,
                object.get("targetId").and_then(Value::as_str),
                object.get("url").and_then(Value::as_str),
                object.get("title").and_then(Value::as_str),
                &service_tab_handle,
            )?;
            object.insert("serviceTabHandle".to_string(), service_tab_handle);
        }
        Ok(result)
    }
    pub(crate) async fn handle_window_new(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        let url = cmd
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("about:blank");
        let same_profile = cmd
            .get("sameProfile")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let create_params = if same_profile {
            json!({ "url" : url, "newWindow" : true })
        } else {
            let context_result = mgr
                .client
                .send_command_no_params("Target.createBrowserContext", None)
                .await?;
            let context_id = context_result
                .get("browserContextId")
                .and_then(|v| v.as_str())
                .ok_or("Failed to create browser context")?
                .to_string();
            json!({ "url" : url, "browserContextId" : context_id, "newWindow" : true })
        };
        let create_result: super::super::cdp::types::CreateTargetResult = mgr
            .client
            .send_command_typed("Target.createTarget", &create_params, None)
            .await?;
        let attach: super::super::cdp::types::AttachToTargetResult = mgr
            .client
            .send_command_typed(
                "Target.attachToTarget",
                &super::super::cdp::types::AttachToTargetParams {
                    target_id: create_result.target_id.clone(),
                    flatten: true,
                },
                None,
            )
            .await?;
        mgr.add_page(super::super::browser::PageInfo {
            target_id: create_result.target_id.clone(),
            session_id: attach.session_id,
            url: url.to_string(),
            title: String::new(),
            target_type: "page".to_string(),
        });
        if let Some(viewport) = cmd.get("viewport") {
            let width = viewport
                .get("width")
                .and_then(|v| v.as_i64())
                .unwrap_or(1280) as i32;
            let height = viewport
                .get("height")
                .and_then(|v| v.as_i64())
                .unwrap_or(720) as i32;
            mgr.set_viewport(width, height, 1.0, false).await?;
            if let Some(ref server) = state.stream_server {
                server.set_viewport(width as u32, height as u32).await;
            }
        }
        let total = mgr.page_count();
        let index = total - 1;
        state.ref_map.clear();
        Ok(json!(
            { "index" : index, "total" : total, "url" : url, "targetId" :
            create_result.target_id, "sameProfile" : same_profile, }
        ))
    }
}
pub(crate) use action_commands::*;
