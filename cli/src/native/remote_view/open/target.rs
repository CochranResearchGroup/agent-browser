#![allow(unused_imports)]
use super::deadline::{RouteBoundOpenSupervisor, RouteBoundRuntimeIssue};
use super::route_pool::RouteParkingPlan;
use super::runtime::{
    route_bound_runtime_issue, NavigateTargetRequest, OpenTargetCommand, OpenTargetRequest,
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

pub(crate) fn remote_view_open_reacquired_live_target(
    pages: &[PageInfo],
    preferred_target_id: Option<&str>,
    active_target_id: Option<&str>,
    expected_url: Option<&str>,
) -> Option<PageInfo> {
    let expected_url = expected_url?;
    let compatible = |page: &&PageInfo| {
        !is_blank_url(page.url.as_str())
            && route_bound_handoff_target_url_readiness(Some(expected_url), Some(&page.url))
                == "ready"
    };
    preferred_target_id
        .and_then(|target_id| {
            pages
                .iter()
                .filter(&compatible)
                .find(|page| page.target_id == target_id)
        })
        .or_else(|| {
            active_target_id.and_then(|target_id| {
                pages
                    .iter()
                    .filter(&compatible)
                    .find(|page| page.target_id == target_id)
            })
        })
        .or_else(|| pages.iter().find(compatible))
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
    let reacquire_only =
        cmd.get("durableResolutionMode").and_then(Value::as_str) == Some("reacquire_only");
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
    let selected = if reacquire_only {
        remote_view_open_reacquired_live_target(
            &observation.pages,
            cmd.get("preferredTargetId").and_then(Value::as_str),
            observation.active_target_id.as_deref(),
            expected_url,
        )
    } else {
        remote_view_open_reusable_live_target(
            &observation.pages,
            cmd.get("preferredTargetId").and_then(Value::as_str),
            desired_origin.as_deref(),
        )
    }
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
                .into_value()
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
    } else if reacquire_only {
        return Err(route_bound_runtime_issue(
            "validate_retained_browser",
            "durable_handoff_target_unavailable: the exact retained target disappeared during presentation reacquisition".to_string(),
            Some(cmd),
        ));
    } else {
        let mut opened = supervisor
            .forward(
                "open_target",
                runtime.open_target(OpenTargetRequest {
                    command: OpenTargetCommand::from_compatibility(
                        remote_view_open_tab_creation_command(cmd),
                    )
                    .map_err(|message| {
                        route_bound_runtime_issue("open_target", message, Some(cmd))
                    })?,
                }),
            )
            .await?
            .into_value();
        opened["tabAcquisitionDecision"] = json!("opened_new_target");
        opened["reusedExistingTarget"] = Value::Bool(false);
        opened
    };
    if !reacquire_only {
        route_bound_open_wait_for_target(cmd, runtime, supervisor, &mut tab).await;
    }
    tab["duplicateTargetCleanup"] = no_duplicate_target_cleanup();
    if !reacquire_only {
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
        "leaseId" : session_id, "leaseState" : "shared", "cleanupPolicy" : "close_tabs",
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
        tab["targetSwitch"] = switched.into_value();
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
                    result.into_value(), }
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
