#![allow(unused_imports)]
use super::deadline::{RouteBoundOpenSupervisor, RouteBoundRuntimeIssue};
use super::route_pool::RouteParkingPlan;
use super::runtime::{
    route_bound_runtime_issue, NavigateTargetRequest, OpenTargetRequest,
    RouteBoundBrowserObservation, RouteBoundOpenRuntime, SwitchTargetRequest,
};
use super::shared::*;
pub(crate) fn remote_view_open_reusable_live_target(
    pages: &[PageInfo],
    preferred_target_id: Option<&str>,
    desired_origin: Option<&str>,
) -> Option<PageInfo> {
    if let Some(preferred_target_id) = preferred_target_id {
        if let Some(page) = pages
            .iter()
            .find(|page| page.target_id == preferred_target_id && !is_blank_url(&page.url))
        {
            return Some(page.clone());
        }
    }
    let desired_origin = desired_origin?;
    pages
        .iter()
        .find(|page| {
            !is_blank_url(page.url.as_str())
                && origin_for_url(page.url.as_str()).as_deref() == Some(desired_origin)
        })
        .cloned()
}
pub(crate) fn remote_view_open_retained_tab_candidate(
    service_state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    desired_origin: Option<&str>,
) -> Option<BrowserTab> {
    let desired_origin = desired_origin?;
    service_state
        .tabs
        .values()
        .filter(|tab| tab.browser_id == browser_id)
        .filter(|tab| tab.owner_session_id.as_deref() == Some(session_id))
        .filter(|tab| tab.lifecycle == TabLifecycle::Ready)
        .filter(|tab| {
            tab.target_id
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        })
        .find(|tab| {
            tab.url
                .as_deref()
                .filter(|url| !is_blank_url(url))
                .and_then(origin_for_url)
                .as_deref()
                == Some(desired_origin)
        })
        .cloned()
}
pub(crate) fn remote_view_open_tab_creation_command(cmd: &Value) -> Value {
    let mut initial = cmd.clone();
    initial["url"] = json!("about:blank");
    initial
}
pub(crate) fn remote_view_open_active_target_readback(
    active_target_id: Option<&str>,
    pages: &[PageInfo],
    target_id: &str,
) -> Option<Value> {
    if active_target_id != Some(target_id) {
        return None;
    }
    let page = pages.iter().find(|page| page.target_id == target_id)?;
    Some(json!(
        { "targetId" : page.target_id, "state" : "already_active", "url" : page.url,
        "title" : page.title, }
    ))
}
pub(crate) async fn route_bound_open_acquire_target<R: RouteBoundOpenRuntime>(
    cmd: &Value,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    service_state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    prefer_active_existing_target: bool,
) -> Result<Value, RouteBoundRuntimeIssue> {
    let expected_url = cmd.get("url").and_then(Value::as_str);
    let desired_origin = expected_url.and_then(origin_for_url);
    let observation = supervisor
        .forward("refresh_targets", runtime.refresh_targets())
        .await?;
    let retained_target_id = remote_view_open_retained_tab_candidate(
        service_state,
        browser_id,
        session_id,
        desired_origin.as_deref(),
    )
    .and_then(|tab| tab.target_id.clone());
    let selected = remote_view_open_reusable_live_target(
        &observation.pages,
        cmd.get("preferredTargetId").and_then(Value::as_str),
        desired_origin.as_deref(),
    )
    .or_else(|| {
        retained_target_id.as_deref().and_then(|target_id| {
            observation
                .pages
                .iter()
                .find(|page| page.target_id == target_id)
                .cloned()
        })
    })
    .or_else(|| {
        prefer_active_existing_target.then(|| {
            observation
                .active_target_id
                .as_deref()
                .and_then(|target_id| {
                    observation
                        .pages
                        .iter()
                        .find(|page| page.target_id == target_id)
                        .cloned()
                })
        })?
    });
    let mut tab = if let Some(page) = selected {
        let switch = if observation.active_target_id.as_deref() == Some(page.target_id.as_str()) {
            json!(
                { "targetId" : page.target_id, "state" : "already_active", "url" : page
                .url, "title" : page.title, }
            )
        } else {
            supervisor
                .forward(
                    "switch_target",
                    runtime.switch_target(SwitchTargetRequest {
                        target_id: page.target_id.clone(),
                    }),
                )
                .await?
        };
        let selected_target_id = switch
            .get("targetId")
            .and_then(Value::as_str)
            .unwrap_or(&page.target_id)
            .to_string();
        let decision = if retained_target_id.as_deref() == Some(selected_target_id.as_str()) {
            "reused_retained_service_tab"
        } else if prefer_active_existing_target
            && observation.active_target_id.as_deref() == Some(selected_target_id.as_str())
        {
            "reused_active_target_for_route_reattach"
        } else {
            "reused_compatible_target"
        };
        route_bound_open_reused_target_result(
            cmd,
            &observation,
            browser_id,
            session_id,
            &selected_target_id,
            switch,
            decision,
        )
        .map_err(|message| route_bound_runtime_issue("observe_browser", message, Some(cmd)))?
    } else {
        let mut opened = supervisor
            .forward(
                "open_target",
                runtime.open_target(OpenTargetRequest {
                    command: remote_view_open_tab_creation_command(cmd),
                }),
            )
            .await?;
        opened["tabAcquisitionDecision"] = json!("opened_new_target");
        opened["reusedExistingTarget"] = Value::Bool(false);
        opened
    };
    route_bound_open_wait_for_target(cmd, runtime, supervisor, &mut tab).await;
    tab["duplicateTargetCleanup"] = no_duplicate_target_cleanup();
    if let Some(service_tab_handle) = tab.get("serviceTabHandle").cloned() {
        persist_service_owned_tab_new(
            cmd,
            session_id,
            tab.get("targetId").and_then(Value::as_str),
            tab.get("url").and_then(Value::as_str),
            tab.get("title").and_then(Value::as_str),
            &service_tab_handle,
        )
        .map_err(|message| route_bound_runtime_issue("open_target", message, Some(cmd)))?;
    }
    Ok(tab)
}
pub(crate) fn route_bound_open_reused_target_result(
    cmd: &Value,
    observation: &RouteBoundBrowserObservation,
    browser_id: &str,
    session_id: &str,
    target_id: &str,
    switch: Value,
    decision: &str,
) -> Result<Value, String> {
    let page = observation
        .pages
        .iter()
        .find(|page| page.target_id == target_id);
    let url = switch
        .get("url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| page.map(|page| page.url.clone()))
        .or_else(|| observation.active_url.clone())
        .unwrap_or_default();
    let title = switch
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| page.map(|page| page.title.clone()))
        .or_else(|| observation.active_title.clone())
        .unwrap_or_default();
    let profile_id = observation.runtime_profile.clone().unwrap_or_default();
    let service_tab_handle = json!(
        { "browserId" : browser_id, "sessionName" : session_id, "tabId" :
        format!("target:{target_id}"), "targetId" : target_id, "url" : url, "title" :
        title, "profileId" : profile_id, "profileOrigin" : "agent_browser_owned",
        "leaseId" : session_id, "leaseState" : "shared", "cleanupPolicy" : "detach",
        "leaseHeartbeatExpected" : true, "ownerSessionId" : session_id, "jobId" :
        Value::Null, "traceFilter" : { "browserId" : browser_id, "profileId" :
        profile_id, "sessionId" : session_id, "serviceName" :
        optional_command_string(cmd, "serviceName"), "agentName" :
        optional_command_string(cmd, "agentName"), "taskName" :
        optional_command_string(cmd, "taskName"), }, "valid" : true, "staleReason" :
        Value::Null, }
    );
    persist_service_owned_tab_new(
        cmd,
        session_id,
        Some(target_id),
        Some(&url),
        Some(&title),
        &service_tab_handle,
    )?;
    Ok(json!(
        { "targetId" : target_id, "url" : url, "title" : title, "browserId" :
        browser_id, "sessionId" : session_id, "profileId" : profile_id,
        "serviceTabHandle" : service_tab_handle, "reusedExistingTarget" : true,
        "tabAcquisitionDecision" : decision, "targetReadiness" :
        route_bound_handoff_target_url_readiness(cmd.get("url")
        .and_then(Value::as_str), Some(& url),), "tabSwitch" : switch, }
    ))
}
pub(crate) async fn route_bound_open_wait_for_target<R: RouteBoundOpenRuntime>(
    cmd: &Value,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    tab: &mut Value,
) {
    let Some(expected_url) = cmd.get("url").and_then(Value::as_str) else {
        return;
    };
    let Some(target_id) = tab
        .get("targetId")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    if let Ok(switched) = supervisor
        .forward(
            "switch_target",
            runtime.switch_target(SwitchTargetRequest {
                target_id: target_id.clone(),
            }),
        )
        .await
    {
        tab["targetSwitch"] = switched;
    }
    let observed_url = tab
        .pointer("/targetSwitch/url")
        .and_then(Value::as_str)
        .or_else(|| tab.get("url").and_then(Value::as_str));
    if route_bound_handoff_target_url_readiness(Some(expected_url), observed_url) != "ready" {
        match supervisor
            .forward(
                "navigate_target",
                runtime.navigate_target(NavigateTargetRequest {
                    url: expected_url.to_string(),
                }),
            )
            .await
        {
            Ok(result) => {
                tab["targetNavigation"] = json!(
                    { "state" : "requested", "requestedUrl" : expected_url, "result" :
                    result, }
                );
            }
            Err(error) => {
                tab["targetNavigation"] = json!(
                    { "state" : "failed", "requestedUrl" : expected_url, "error" : error
                    .compatibility_message(), }
                );
            }
        }
    }
    for attempt in 0..20 {
        let Ok(observation) = supervisor
            .forward("refresh_targets", runtime.refresh_targets())
            .await
        else {
            return;
        };
        let page = observation
            .pages
            .iter()
            .find(|page| page.target_id == target_id);
        let url = page
            .map(|page| page.url.as_str())
            .or(observation.active_url.as_deref());
        let title = page
            .map(|page| page.title.as_str())
            .or(observation.active_title.as_deref());
        if let Some(url) = url {
            tab["url"] = json!(url);
        }
        if let Some(title) = title {
            tab["title"] = json!(title);
        }
        tab["urlReadbackAttempts"] = json!(attempt + 1);
        let readiness = route_bound_handoff_target_url_readiness(Some(expected_url), url);
        tab["targetReadiness"] = json!(readiness);
        if readiness == "ready" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
pub(crate) async fn remote_view_open_acquire_tab(
    cmd: &Value,
    state: &mut DaemonState,
    service_state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    prefer_active_existing_target: bool,
) -> Result<Value, String> {
    let requested_url = cmd.get("url").and_then(Value::as_str);
    let desired_origin = requested_url.and_then(origin_for_url);
    if desired_origin.is_some() {
        if let Some(mgr) = state.browser.as_mut() {
            let active_url = mgr.get_url().await.ok();
            let active_title = mgr.get_title().await.ok();
            if active_url.as_deref().and_then(origin_for_url).as_deref()
                == desired_origin.as_deref()
            {
                mgr.set_active_page_metadata(active_url.as_deref(), active_title.as_deref());
            }
        }
    }
    let reusable_target = {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        remote_view_open_reusable_live_target(
            &mgr.pages_list(),
            cmd.get("preferredTargetId").and_then(Value::as_str),
            desired_origin.as_deref(),
        )
    };
    if let Some(page) = reusable_target {
        state.ref_map.clear();
        state.iframe_sessions.clear();
        state.active_frame_id = None;
        let session_id = state.session_id.clone();
        let browser_id = service_browser_id(&session_id);
        let mut result = {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            let already_active = mgr.active_target_id().ok() == Some(page.target_id.as_str());
            let mut switched = if already_active {
                json!(
                    { "targetId" : page.target_id.clone(), "state" : "already_active",
                    "url" : page.url.clone(), "title" : page.title.clone(), }
                )
            } else {
                mgr.tab_switch_target_id(&page.target_id).await?
            };
            let url = if already_active {
                page.url
            } else {
                mgr.get_url().await.unwrap_or(page.url)
            };
            let title = if already_active {
                page.title
            } else {
                mgr.get_title().await.unwrap_or(page.title)
            };
            switched["refreshDecision"] = json!("reused_compatible_target");
            let mut result = json!(
                { "targetId" : switched.get("targetId").and_then(Value::as_str)
                .unwrap_or_default(), "url" : url, "title" : title, "browserId" :
                browser_id, "sessionId" : session_id, "reusedExistingTarget" : true,
                "tabAcquisitionDecision" : "reused_compatible_target", "tabSwitch" :
                switched, }
            );
            if let Some(object) = result.as_object_mut() {
                if let Some(runtime_profile) = mgr.runtime_profile_name() {
                    object.insert("runtimeProfile".to_string(), json!(runtime_profile));
                    object.insert("profileId".to_string(), json!(runtime_profile));
                }
                let profile_id = object.get("profileId").cloned().unwrap_or(Value::Null);
                object.insert(
                    "sharedAcquisition".to_string(),
                    tab_new_shared_acquisition_evidence(cmd, &state.session_id, profile_id.clone()),
                );
                let tab_id = object
                    .get("targetId")
                    .and_then(Value::as_str)
                    .map(|target_id| format!("target:{target_id}"))
                    .unwrap_or_else(|| format!("session:{}:active-tab", state.session_id));
                let service_tab_handle = json!(
                    { "browserId" : service_browser_id(& state.session_id), "sessionName"
                    : state.session_id.clone(), "tabId" : tab_id, "targetId" : object
                    .get("targetId").cloned().unwrap_or(Value::Null), "url" : object
                    .get("url").cloned().unwrap_or(Value::Null), "title" : object
                    .get("title").cloned().unwrap_or(Value::Null), "profileId" :
                    profile_id.clone(), "profileOrigin" : "agent_browser_owned",
                    "leaseId" : state.session_id.clone(), "leaseState" : "shared",
                    "cleanupPolicy" : "detach", "leaseHeartbeatExpected" : true,
                    "ownerSessionId" : state.session_id.clone(), "jobId" : Value::Null,
                    "traceFilter" : { "browserId" : service_browser_id(& state
                    .session_id), "profileId" : profile_id.clone(), "sessionId" : state
                    .session_id.clone(), "serviceName" : optional_command_string(cmd,
                    "serviceName"), "agentName" : optional_command_string(cmd,
                    "agentName"), "taskName" : optional_command_string(cmd, "taskName"),
                    }, "valid" : true, "staleReason" : Value::Null, }
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
            result
        };
        remote_view_open_wait_for_target_url(cmd, state, &mut result).await;
        if let Some(service_tab_handle) = result.get("serviceTabHandle").cloned() {
            persist_service_owned_tab_new(
                cmd,
                &state.session_id,
                result.get("targetId").and_then(Value::as_str),
                result.get("url").and_then(Value::as_str),
                result.get("title").and_then(Value::as_str),
                &service_tab_handle,
            )?;
        }
        let selected_target_id = result
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or_else(|| "remote_view_open reused a tab without targetId".to_string())?
            .to_string();
        if let Some(mgr) = state.browser.as_mut() {
            let duplicate_target_cleanup = close_compatible_duplicate_targets(
                mgr,
                &selected_target_id,
                None,
                desired_origin.as_deref(),
            )
            .await;
            result["duplicateTargetCleanup"] = duplicate_target_cleanup;
        }
        return Ok(result);
    }
    if let Some(tab) = remote_view_open_retained_tab_candidate(
        service_state,
        browser_id,
        session_id,
        desired_origin.as_deref(),
    ) {
        if let Some(target_id) = tab.target_id.as_deref() {
            if state
                .browser
                .as_ref()
                .and_then(|mgr| mgr.active_target_id().ok())
                == Some(target_id)
            {
                if let Some(mgr) = state.browser.as_mut() {
                    mgr.set_active_page_metadata(tab.url.as_deref(), tab.title.as_deref());
                }
                let profile_id = tab
                    .service_tab_handle
                    .as_ref()
                    .and_then(|handle| handle.profile_id.clone())
                    .unwrap_or_default();
                let mut result = json!(
                    { "targetId" : target_id, "url" : tab.url.clone()
                    .unwrap_or_default(), "title" : tab.title.clone()
                    .unwrap_or_default(), "browserId" : browser_id, "sessionId" :
                    session_id, "profileId" : profile_id, "reusedExistingTarget" : true,
                    "tabAcquisitionDecision" :
                    "reused_retained_service_tab_active_target", "targetReadiness" :
                    route_bound_handoff_target_url_readiness(cmd.get("url")
                    .and_then(Value::as_str), tab.url.as_deref()),
                    "duplicateTargetCleanup" : no_duplicate_target_cleanup(), }
                );
                if let Some(handle) = tab.service_tab_handle.as_ref() {
                    result["serviceTabHandle"] =
                        serde_json::to_value(handle).unwrap_or(Value::Null);
                }
                return Ok(result);
            }
        }
    }
    if prefer_active_existing_target {
        if let Some(mgr) = state.browser.as_mut() {
            if let Ok(target_id) = mgr.active_target_id().map(str::to_string) {
                let requested_url = requested_url.unwrap_or("about:blank");
                let title = mgr.get_title().await.unwrap_or_default();
                mgr.set_page_metadata_for_target(&target_id, Some(requested_url), Some(&title));
                let profile_id = mgr.runtime_profile_name().unwrap_or_default().to_string();
                let service_tab_handle = json!(
                    { "browserId" : browser_id, "sessionName" : session_id, "tabId" :
                    format!("target:{target_id}"), "targetId" : target_id, "url" :
                    requested_url, "title" : title, "profileId" : profile_id,
                    "profileOrigin" : "agent_browser_owned", "leaseId" : session_id,
                    "leaseState" : "shared", "cleanupPolicy" : "detach",
                    "leaseHeartbeatExpected" : true, "ownerSessionId" : session_id,
                    "jobId" : Value::Null, "traceFilter" : { "browserId" : browser_id,
                    "profileId" : profile_id.clone(), "sessionId" : session_id,
                    "serviceName" : optional_command_string(cmd, "serviceName"),
                    "agentName" : optional_command_string(cmd, "agentName"), "taskName" :
                    optional_command_string(cmd, "taskName"), }, "valid" : true,
                    "staleReason" : Value::Null, }
                );
                persist_service_owned_tab_new(
                    cmd,
                    session_id,
                    Some(&target_id),
                    Some(requested_url),
                    Some(&title),
                    &service_tab_handle,
                )?;
                return Ok(json!(
                    { "targetId" : target_id, "url" : requested_url, "title" : title,
                    "browserId" : browser_id, "sessionId" : session_id, "profileId" :
                    profile_id.clone(), "serviceTabHandle" : service_tab_handle,
                    "reusedExistingTarget" : true, "tabAcquisitionDecision" :
                    "reused_active_target_for_route_reattach", "targetReadiness" :
                    route_bound_handoff_target_url_readiness(cmd.get("url")
                    .and_then(Value::as_str), Some(requested_url)),
                    "duplicateTargetCleanup" : no_duplicate_target_cleanup(), }
                ));
            }
        }
    }
    let initial_tab_command = remote_view_open_tab_creation_command(cmd);
    let mut opened = handle_tab_new(&initial_tab_command, state).await?;
    remote_view_open_wait_for_target_url(cmd, state, &mut opened).await;
    if let Some(service_tab_handle) = opened.get("serviceTabHandle").cloned() {
        persist_service_owned_tab_new(
            cmd,
            &state.session_id,
            opened.get("targetId").and_then(Value::as_str),
            opened.get("url").and_then(Value::as_str),
            opened.get("title").and_then(Value::as_str),
            &service_tab_handle,
        )?;
    }
    if let Some(target_id) = opened
        .get("targetId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
    {
        if let Some(mgr) = state.browser.as_mut() {
            let duplicate_target_cleanup = close_compatible_duplicate_targets(
                mgr,
                &target_id,
                None,
                desired_origin.as_deref(),
            )
            .await;
            opened["duplicateTargetCleanup"] = duplicate_target_cleanup;
        }
    }
    opened["tabAcquisitionDecision"] = json!("opened_new_target");
    Ok(opened)
}
pub(crate) async fn remote_view_open_wait_for_target_url(
    cmd: &Value,
    state: &mut DaemonState,
    tab: &mut Value,
) {
    let Some(expected_url) = cmd.get("url").and_then(Value::as_str).map(str::to_string) else {
        return;
    };
    let mut target_id = tab
        .get("targetId")
        .and_then(Value::as_str)
        .map(str::to_string);
    {
        let Some(mgr) = state.browser.as_mut() else {
            return;
        };
        let mut switched_once = false;
        if let Some(target_id) = target_id.as_deref() {
            let active_readback = remote_view_open_active_target_readback(
                mgr.active_target_id().ok(),
                &mgr.pages_list(),
                target_id,
            );
            match if let Some(readback) = active_readback {
                Ok(readback)
            } else {
                mgr.tab_switch_target_id(target_id).await
            } {
                Ok(switched) => {
                    switched_once = true;
                    tab["targetSwitch"] = switched;
                }
                Err(err) => {
                    tab["targetSwitch"] = json!({ "state" : "failed", "error" : err, });
                }
            }
        }
        let switched_url = tab
            .pointer("/targetSwitch/url")
            .and_then(Value::as_str)
            .map(str::to_string);
        if switched_once
            && route_bound_handoff_target_url_readiness(
                Some(&expected_url),
                switched_url.as_deref(),
            ) != "ready"
        {
            match mgr.navigate(&expected_url, WaitUntil::None).await {
                Ok(navigation) => {
                    tab["targetNavigation"] = json!(
                        { "state" : "requested", "requestedUrl" : expected_url.clone(),
                        "result" : navigation, }
                    );
                }
                Err(err) => {
                    tab["targetNavigation"] = json!(
                        { "state" : "failed", "requestedUrl" : expected_url.clone(),
                        "error" : err, }
                    );
                    if let Some(target_id) = target_id.as_deref() {
                        mgr.set_page_metadata_for_target(target_id, Some(&expected_url), None);
                    }
                }
            }
        }
    }
    let desired_origin = origin_for_url(&expected_url);
    for attempt in 0..20 {
        state.drain_cdp_events_background().await;
        let Some(mgr) = state.browser.as_mut() else {
            return;
        };
        let selected_switched = if let Some(target_id) = target_id.as_deref() {
            let active_readback = remote_view_open_active_target_readback(
                mgr.active_target_id().ok(),
                &mgr.pages_list(),
                target_id,
            );
            if active_readback.is_some() {
                active_readback
            } else {
                mgr.tab_switch_target_id(target_id).await.ok()
            }
        } else {
            None
        };
        let pages = mgr.pages_list();
        let mut selected_target_id = target_id.clone();
        let mut switched = selected_switched;
        let mut target_page = target_id.as_deref().and_then(|target_id| {
            pages
                .iter()
                .find(|page| page.target_id == target_id)
                .cloned()
        });
        let selected_url = switched
            .as_ref()
            .and_then(|value| value.get("url"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| target_page.as_ref().map(|page| page.url.clone()));
        if route_bound_handoff_target_url_readiness(Some(&expected_url), selected_url.as_deref())
            != "ready"
        {
            if let Some(compatible_page) =
                remote_view_open_reusable_live_target(&pages, None, desired_origin.as_deref())
            {
                if target_id.as_deref() != Some(compatible_page.target_id.as_str()) {
                    if let Ok(compatible_switched) =
                        mgr.tab_switch_target_id(&compatible_page.target_id).await
                    {
                        tab["targetReselection"] = json!(
                            { "state" : "reselected_compatible_target",
                            "previousTargetId" : target_id, "targetId" : compatible_page
                            .target_id, "url" : compatible_page.url, "title" :
                            compatible_page.title, }
                        );
                        target_id = compatible_switched
                            .get("targetId")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .or_else(|| {
                                tab.pointer("/targetReselection/targetId")
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                            });
                        selected_target_id = target_id.clone();
                        switched = Some(compatible_switched);
                    }
                }
            }
        }
        target_page = selected_target_id.as_deref().and_then(|target_id| {
            mgr.pages_list()
                .into_iter()
                .find(|page| page.target_id == target_id)
        });
        let mut url = switched
            .as_ref()
            .and_then(|value| value.get("url"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if url.is_none() {
            url = mgr.get_url().await.ok();
        }
        if url.is_none() {
            url = target_page.as_ref().map(|page| page.url.clone());
        }
        let mut title = switched
            .as_ref()
            .and_then(|value| value.get("title"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if title.is_none() {
            title = mgr.get_title().await.ok();
        }
        if title.is_none() {
            title = target_page.as_ref().map(|page| page.title.clone());
        }
        if let Some(url) = url.as_deref() {
            tab["url"] = json!(url);
            if let Some(service_tab_handle) = tab
                .get_mut("serviceTabHandle")
                .and_then(Value::as_object_mut)
            {
                service_tab_handle.insert("url".to_string(), json!(url));
            }
        }
        if let Some(title) = title.as_deref() {
            tab["title"] = json!(title);
            if let Some(service_tab_handle) = tab
                .get_mut("serviceTabHandle")
                .and_then(Value::as_object_mut)
            {
                service_tab_handle.insert("title".to_string(), json!(title));
            }
        }
        if let Some(target_id) = selected_target_id.as_deref() {
            tab["targetId"] = json!(target_id);
            if let Some(service_tab_handle) = tab
                .get_mut("serviceTabHandle")
                .and_then(Value::as_object_mut)
            {
                service_tab_handle.insert("targetId".to_string(), json!(target_id));
                service_tab_handle
                    .insert("tabId".to_string(), json!(format!("target:{target_id}")));
            }
            mgr.set_page_metadata_for_target(target_id, url.as_deref(), title.as_deref());
        }
        mgr.set_active_page_metadata(url.as_deref(), title.as_deref());
        tab["urlReadbackAttempts"] = json!(attempt + 1);
        tab["targetReadiness"] = json!(route_bound_handoff_target_url_readiness(
            Some(&expected_url),
            url.as_deref()
        ));
        if route_bound_handoff_target_url_readiness(Some(&expected_url), url.as_deref()) == "ready"
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
pub(crate) fn retained_readiness_component<'a>(
    readiness: Option<&'a Value>,
    component_names: &[&str],
) -> Option<&'a Value> {
    readiness
        .and_then(|readiness| {
            readiness
                .get("components")
                .or_else(|| readiness.pointer("/readiness/components"))
        })
        .and_then(Value::as_array)
        .and_then(|components| {
            components.iter().find(|component| {
                component
                    .get("component")
                    .and_then(Value::as_str)
                    .is_some_and(|name| {
                        component_names.iter().any(|expected| {
                            name == *expected
                                || name
                                    .strip_prefix(*expected)
                                    .is_some_and(|rest| rest.starts_with(':'))
                        })
                    })
            })
        })
}
