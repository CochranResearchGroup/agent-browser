#![allow(unused_imports)]
use super::action_runtime::common::*;
use super::action_runtime::runtime::{
    service_browser_id, validate_service_tab_handle_for_current_session, DaemonState,
    RuntimeHandoffDescriptor, TrackedRequest,
};
use super::browser_navigation::handle_reload;
use super::interaction::{
    handle_clear, handle_click, handle_dialog, handle_fill, handle_focus, handle_select,
    handle_type, handle_wait,
};
use super::network::matches_status_filter;
pub(crate) async fn handle_service_diagnostics(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "diagnostics requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let target_id = handle.get("targetId").and_then(Value::as_str);
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let max_console_entries = bounded_usize(cmd, "maxConsoleEntries", 10, 50);
    let max_error_entries = bounded_usize(cmd, "maxErrorEntries", 10, 50);
    let max_request_entries = bounded_usize(cmd, "maxRequestEntries", 10, 50);
    let include_screenshot = cmd
        .get("includeScreenshot")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (url, title, active_target_id, active_session_id, screenshot) =
        if let Some(mgr) = state.browser.as_mut() {
            if let Some(target_id) = target_id {
                if mgr.active_target_id().ok() != Some(target_id) {
                    let _ = mgr.tab_switch_target_id(target_id).await?;
                }
            }
            let active_target_id = mgr.active_target_id().ok().map(ToString::to_string);
            let active_session_id = mgr.active_session_id().ok().map(ToString::to_string);
            let url = mgr.get_url().await.unwrap_or_default();
            let title = mgr.get_title().await.unwrap_or_default();
            let screenshot = if include_screenshot {
                if let Some(session_id) = active_session_id.as_deref() {
                    let options = ScreenshotOptions {
                        selector: None,
                        path: None,
                        full_page: false,
                        format: "png".to_string(),
                        quality: None,
                        annotate: false,
                        output_dir: cmd
                            .get("screenshotDir")
                            .and_then(Value::as_str)
                            .map(String::from),
                    };
                    match screenshot::take_screenshot(
                        &mgr.client,
                        session_id,
                        &state.ref_map,
                        &options,
                        &state.iframe_sessions,
                    )
                    .await
                    {
                        Ok(result) => Some(json!({ "captured" : true, "path" : result.path, })),
                        Err(error) => Some(json!({ "captured" : false, "error" : error, })),
                    }
                } else {
                    None
                }
            } else {
                None
            };
            (url, title, active_target_id, active_session_id, screenshot)
        } else {
            (String::new(), String::new(), None, None, None)
        };
    let browser_id = handle
        .get("browserId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| service_browser_id(&state.session_id));
    let tab_id = handle
        .get("tabId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let profile_id = handle
        .get("profileId")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let mut service_state = LockedServiceStateRepository::default_json()
        .and_then(|repository| repository.load_snapshot())
        .ok();
    if let Some(service_state) = service_state.as_mut() {
        service_state.refresh_derived_views();
    }
    let browser_record = service_state
        .as_ref()
        .and_then(|service_state| service_state.browsers.get(&browser_id))
        .cloned();
    let tab_record = service_state
        .as_ref()
        .and_then(|service_state| service_state.tabs.get(&tab_id))
        .cloned();
    let session_name = handle
        .get("sessionName")
        .and_then(Value::as_str)
        .or(active_session_id.as_deref())
        .unwrap_or(&state.session_id)
        .to_string();
    let session_record = service_state
        .as_ref()
        .and_then(|service_state| service_state.sessions.get(&session_name))
        .cloned();
    let profile_record = profile_id.as_deref().and_then(|profile_id| {
        service_state
            .as_ref()
            .and_then(|service_state| service_state.profiles.get(profile_id))
            .cloned()
    });
    let routes = service_state
        .as_ref()
        .map(|service_state| {
            service_state
                .remote_view_routes
                .values()
                .filter(|route| {
                    route.browser_id.as_deref() == Some(browser_id.as_str())
                        || route.session_id.as_deref() == Some(session_name.as_str())
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let console = cap_array_items(
        state.event_tracker.get_console_json(),
        "messages",
        max_console_entries,
    );
    let errors = cap_array_items(
        state.event_tracker.get_errors_json(),
        "errors",
        max_error_entries,
    );
    let requests = recent_request_summaries(&state.tracked_requests, max_request_entries);
    Ok(json!(
        { "ok" : true, "action" : "diagnostics", "observedAt" : observed_at,
        "compact" : true, "browserId" : browser_id, "sessionName" : session_name,
        "tabId" : tab_id, "targetId" : target_id.or(active_target_id.as_deref()),
        "activeSessionId" : active_session_id, "profileId" : profile_id,
        "profileOrigin" : handle.get("profileOrigin").cloned()
        .unwrap_or(Value::Null), "url" : if url.is_empty() { handle.get("url")
        .cloned().unwrap_or(Value::Null) } else { json!(url) }, "title" : if title
        .is_empty() { handle.get("title").cloned().unwrap_or(Value::Null) } else {
        json!(title) }, "serviceTabHandle" : cmd.get("serviceTabHandle").cloned()
        .unwrap_or(Value::Null), "traceFilter" : handle.get("traceFilter").cloned()
        .unwrap_or(Value::Null), "browser" : browser_record.as_ref().map(| browser |
        json!({ "id" : browser.id, "profileId" : browser.profile_id, "host" : browser
        .host, "health" : browser.health, "displayIsolation" : browser
        .display_isolation, "displayName" : browser.display_name,
        "displayAllocationId" : browser.display_allocation_id, "pid" : browser.pid,
        "activeSessionIds" : browser.active_session_ids, "viewStreams" : browser
        .view_streams, "lastError" : browser.last_error, "lastHealthObservation" :
        browser.last_health_observation, })), "session" : session_record.as_ref()
        .map(| session | json!({ "id" : session.id, "serviceName" : session
        .service_name, "agentName" : session.agent_name, "taskName" : session
        .task_name, "lease" : session.lease, "cleanup" : session.cleanup, "profileId"
        : session.profile_id, "browserIds" : session.browser_ids, "tabIds" : session
        .tab_ids, })), "tab" : tab_record.as_ref().map(| tab | json!({ "id" : tab.id,
        "browserId" : tab.browser_id, "targetId" : tab.target_id, "lifecycle" : tab
        .lifecycle, "url" : tab.url, "title" : tab.title, "ownerSessionId" : tab
        .owner_session_id, "latestSnapshotId" : tab.latest_snapshot_id,
        "latestScreenshotId" : tab.latest_screenshot_id, "challengeId" : tab
        .challenge_id, })), "profile" : profile_record.as_ref().map(| profile |
        json!({ "id" : profile.id, "name" : profile.name, "profileOrigin" : profile
        .profile_origin, "targetServiceIds" : profile.target_service_ids,
        "authenticatedServiceIds" : profile.authenticated_service_ids, "accountIds" :
        profile.account_ids, "browserBuild" : profile.browser_build, "allocation" :
        profile.allocation, "targetReadiness" : profile.target_readiness,
        "registration" : profile.registration, "browserCompatibilityEvidence" :
        profile.browser_compatibility_evidence, })), "remoteViewRoutes" : routes,
        "snapshotSummary" : { "refCount" : state.ref_map.entries_sorted().len(),
        "hasActiveFrame" : state.active_frame_id.is_some(), "latestSnapshotId" :
        tab_record.as_ref().and_then(| tab | tab.latest_snapshot_id.clone()), },
        "screenshot" : screenshot.unwrap_or_else(|| json!({ "captured" : false,
        "reason" : if include_screenshot { "unavailable" } else { "not_requested" },
        })), "console" : console, "errors" : errors, "requests" : { "count" : state
        .tracked_requests.len(), "returned" : requests.len(), "items" : requests, },
        "caller" : { "serviceName" : cmd.get("serviceName").cloned()
        .unwrap_or(Value::Null), "agentName" : cmd.get("agentName").cloned()
        .unwrap_or(Value::Null), "taskName" : cmd.get("taskName").cloned()
        .unwrap_or(Value::Null), "jobId" : cmd.get("id").cloned()
        .unwrap_or(Value::Null), }, }
    ))
}
pub(crate) fn bounded_usize(
    cmd: &Value,
    key: &str,
    default_value: usize,
    max_value: usize,
) -> usize {
    cmd.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
        .min(max_value)
}
pub(crate) fn cap_array_items(mut value: Value, key: &str, limit: usize) -> Value {
    let total = value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) {
        if items.len() > limit {
            let keep_from = items.len().saturating_sub(limit);
            *items = items.split_off(keep_from);
        }
    }
    let returned = value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if let Some(obj) = value.as_object_mut() {
        obj.insert("count".to_string(), json!(total));
        obj.insert("returned".to_string(), json!(returned));
        obj.insert("truncated".to_string(), json!(total > limit));
    }
    value
}
pub(crate) fn recent_request_summaries(requests: &[TrackedRequest], limit: usize) -> Vec<Value> {
    let keep_from = requests.len().saturating_sub(limit);
    requests
        .iter()
        .skip(keep_from)
        .map(|request| {
            json!(
                { "requestId" : request.request_id, "url" : request.url, "method" :
                request.method, "status" : request.status, "resourceType" : request
                .resource_type, "mimeType" : request.mime_type, }
            )
        })
        .collect()
}
pub(crate) fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}
