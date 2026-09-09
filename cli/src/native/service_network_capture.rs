#![allow(unused_imports)]
use super::action_runtime::runtime::{
    service_browser_id, validate_service_tab_handle_for_daemon, DaemonState,
    RuntimeHandoffDescriptor, TrackedRequest,
};
use super::browser_navigation::handle_reload;
use super::interaction::{
    handle_clear, handle_click, handle_dialog, handle_fill, handle_focus, handle_select,
    handle_type, handle_wait,
};
use super::network::matches_status_filter;
use super::service_diagnostics::truncate_utf8;
use super::service_probe::probe_recipe_fingerprint;
use super::service_ui_action::{service_ui_caller, service_ui_current_page};
use crate::native::interaction;
use crate::native::state;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{Duration, Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::{broadcast, oneshot, RwLock};
pub(crate) async fn handle_service_network_capture(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "network_capture requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_daemon(handle, cmd, state)?;
    let capture = cmd
        .get("networkCapture")
        .and_then(Value::as_object)
        .ok_or_else(|| "network_capture requires networkCapture object".to_string())?;
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .or_else(|| capture.get("timeoutMs").and_then(Value::as_u64))
        .or_else(|| capture.get("maxDurationMs").and_then(Value::as_u64))
        .ok_or_else(|| "network_capture requires positive timeoutMs".to_string())?;
    if timeout_ms == 0 {
        return Err("network_capture requires positive timeoutMs".to_string());
    }
    let max_events = capture
        .get("maxEvents")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 100) as usize;
    let capture_bodies = capture
        .get("captureBodies")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_body_bytes = if capture_bodies {
        let max_body_bytes = capture
            .get("maxBodyBytes")
            .and_then(Value::as_u64)
            .or_else(|| cmd.get("maxBodyBytes").and_then(Value::as_u64))
            .ok_or_else(|| {
                "network_capture captureBodies requires positive maxBodyBytes".to_string()
            })?;
        if max_body_bytes == 0 {
            return Err("network_capture captureBodies requires positive maxBodyBytes".to_string());
        }
        max_body_bytes.min(1024 * 1024)
    } else {
        0
    };
    validate_service_network_capture_recipe(capture)?;
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "network_capture requires serviceTabHandle.targetId".to_string())?;
    let session_id = {
        let mgr = state
            .browser
            .as_mut()
            .ok_or_else(|| {
                "Cannot run network_capture: target browser session is not running; request a service tab first"
                    .to_string()
            })?;
        if mgr.active_target_id().ok() != Some(target_id) {
            let _ = mgr.tab_switch_target_id(target_id).await?;
        }
        mgr.active_session_id()?.to_string()
    };
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let before = service_ui_current_page(state).await;
    let mgr = state
        .browser
        .as_ref()
        .ok_or_else(|| "Browser not launched".to_string())?;
    mgr.client
        .send_command_no_params("Network.enable", Some(&session_id))
        .await?;
    let mut rx = mgr.client.subscribe();
    run_service_network_capture_trigger(cmd, state).await?;
    let mut request_metadata: HashMap<String, Value> = HashMap::new();
    let mut pending_body: HashMap<String, Value> = HashMap::new();
    let mut captured = Vec::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    loop {
        if captured.len() >= max_events && (!capture_bodies || pending_body.is_empty()) {
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            timed_out = true;
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                if event.session_id.as_deref() != Some(&session_id) {
                    continue;
                }
                match event.method.as_str() {
                    "Network.requestWillBeSent" => {
                        if let Some(request_id) =
                            event.params.get("requestId").and_then(Value::as_str)
                        {
                            let request = event.params.get("request").cloned().unwrap_or(json!({}));
                            request_metadata.insert(
                                request_id.to_string(),
                                json!(
                                    { "requestId" : request_id, "url" : request.get("url")
                                    .cloned().unwrap_or(Value::Null), "method" : request
                                    .get("method").cloned().unwrap_or(Value::Null),
                                    "resourceType" : event.params.get("type").cloned()
                                    .unwrap_or(Value::Null), "timestamp" : event.params
                                    .get("wallTime").cloned().unwrap_or(Value::Null),
                                    "requestHeaders" : request.get("headers").cloned()
                                    .unwrap_or_else(|| json!({})), }
                                ),
                            );
                        }
                    }
                    "Network.responseReceived" => {
                        if captured.len() + pending_body.len() >= max_events {
                            continue;
                        }
                        let Some(request_id) =
                            event.params.get("requestId").and_then(Value::as_str)
                        else {
                            continue;
                        };
                        let response = event.params.get("response").cloned().unwrap_or(json!({}));
                        let metadata = request_metadata
                            .get(request_id)
                            .cloned()
                            .unwrap_or_else(|| json!({ "requestId" : request_id }));
                        if !service_network_capture_matches(capture, &metadata, &response) {
                            continue;
                        }
                        let event_value = service_network_capture_event(
                            capture,
                            request_id,
                            &metadata,
                            &response,
                            false,
                            None,
                            max_body_bytes,
                        );
                        if capture_bodies {
                            pending_body.insert(request_id.to_string(), event_value);
                        } else {
                            captured.push(event_value);
                        }
                    }
                    "Network.loadingFinished" => {
                        let Some(request_id) =
                            event.params.get("requestId").and_then(Value::as_str)
                        else {
                            continue;
                        };
                        let Some(mut event_value) = pending_body.remove(request_id) else {
                            continue;
                        };
                        let body = service_network_capture_body(
                            state,
                            request_id,
                            &session_id,
                            max_body_bytes,
                        )
                        .await
                        .unwrap_or_else(|error| json!({ "captured" : false, "error" : error, }));
                        event_value["body"] = body;
                        captured.push(event_value);
                    }
                    "Network.loadingFailed" => {
                        if let Some(request_id) =
                            event.params.get("requestId").and_then(Value::as_str)
                        {
                            if let Some(mut event_value) = pending_body.remove(request_id) {
                                event_value["body"] = json!(
                                    { "captured" : false, "error" : event.params
                                    .get("errorText").cloned().unwrap_or_else(||
                                    json!("loading failed")), }
                                );
                                captured.push(event_value);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => {
                timed_out = true;
                break;
            }
            Err(_) => {
                timed_out = true;
                break;
            }
        }
    }
    let after = service_ui_current_page(state).await;
    Ok(json!(
        { "ok" : true, "action" : "network_capture", "observedAt" : observed_at,
        "timedOut" : timed_out, "targetId" : target_id, "tabId" : handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "profileId" : handle.get("profileId")
        .cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "traceFilter" :
        handle.get("traceFilter").cloned().unwrap_or(Value::Null), "networkCapture" :
        { "eventCount" : captured.len(), "pendingBodyCount" : pending_body.len(),
        "maxEvents" : max_events, "timeoutMs" : timeout_ms, "captureBodies" :
        capture_bodies, "maxBodyBytes" : if capture_bodies { json!(max_body_bytes) }
        else { Value::Null }, "metadataOnly" : ! capture_bodies, "recipeId" : capture
        .get("recipeId").cloned().unwrap_or(Value::Null), "recipeFingerprint" :
        probe_recipe_fingerprint(capture), }, "before" : before, "after" : after,
        "events" : captured, "caller" : service_ui_caller(cmd), }
    ))
}
pub(crate) fn validate_service_network_capture_recipe(
    capture: &Map<String, Value>,
) -> Result<(), String> {
    if let Some(patterns) = capture.get("urlPatterns") {
        let valid = patterns
            .as_array()
            .filter(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|item| item.as_str().is_some_and(|value| !value.is_empty()))
            })
            .is_some();
        if !valid {
            return Err("network_capture urlPatterns must be a nonempty string array".to_string());
        }
    }
    if let Some(methods) = capture.get("methods") {
        let valid = methods
            .as_array()
            .filter(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|item| item.as_str().is_some_and(|value| !value.is_empty()))
            })
            .is_some();
        if !valid {
            return Err("network_capture methods must be a nonempty string array".to_string());
        }
    }
    if let Some(resource_types) = capture.get("resourceTypes") {
        let valid = resource_types
            .as_array()
            .filter(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|item| item.as_str().is_some_and(|value| !value.is_empty()))
            })
            .is_some();
        if !valid {
            return Err(
                "network_capture resourceTypes must be a nonempty string array".to_string(),
            );
        }
    }
    if let Some(statuses) = capture.get("status") {
        let valid = statuses
            .as_array()
            .filter(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|item| item.as_str().is_some_and(|value| !value.is_empty()))
            })
            .is_some()
            || statuses
                .as_str()
                .is_some_and(|value| !value.trim().is_empty());
        if !valid {
            return Err("network_capture status must be a string or string array".to_string());
        }
    }
    if let Some(trigger) = capture.get("trigger") {
        let trigger = trigger
            .as_object()
            .ok_or_else(|| "network_capture trigger must be an object".to_string())?;
        let trigger_type = trigger
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "network_capture trigger requires type".to_string())?;
        if trigger_type != "reload" {
            return Err("network_capture trigger.type must be reload".to_string());
        }
    }
    Ok(())
}
pub(crate) async fn run_service_network_capture_trigger(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<(), String> {
    let Some(trigger) = cmd
        .get("networkCapture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("trigger"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };
    match trigger.get("type").and_then(Value::as_str).unwrap_or("") {
        "reload" => {
            handle_reload(state).await?;
            Ok(())
        }
        _ => Err("network_capture trigger.type must be reload".to_string()),
    }
}
pub(crate) fn service_network_capture_matches(
    capture: &Map<String, Value>,
    metadata: &Value,
    response: &Value,
) -> bool {
    let url = response
        .get("url")
        .or_else(|| metadata.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let method = metadata.get("method").and_then(Value::as_str).unwrap_or("");
    let resource_type = metadata
        .get("resourceType")
        .and_then(Value::as_str)
        .unwrap_or("");
    let status = response.get("status").and_then(Value::as_i64);
    if let Some(patterns) = capture.get("urlPatterns").and_then(Value::as_array) {
        if !patterns
            .iter()
            .filter_map(Value::as_str)
            .any(|pattern| url.contains(pattern))
        {
            return false;
        }
    }
    if let Some(methods) = capture.get("methods").and_then(Value::as_array) {
        if !methods
            .iter()
            .filter_map(Value::as_str)
            .any(|expected| method.eq_ignore_ascii_case(expected))
        {
            return false;
        }
    }
    if let Some(types) = capture.get("resourceTypes").and_then(Value::as_array) {
        if !types
            .iter()
            .filter_map(Value::as_str)
            .any(|expected| resource_type.eq_ignore_ascii_case(expected))
        {
            return false;
        }
    }
    if let Some(status_filter) = capture.get("status") {
        let Some(code) = status else {
            return false;
        };
        if let Some(filter) = status_filter.as_str() {
            if !matches_status_filter(Some(code), filter) {
                return false;
            }
        } else if let Some(filters) = status_filter.as_array() {
            if !filters
                .iter()
                .filter_map(Value::as_str)
                .any(|filter| matches_status_filter(Some(code), filter))
            {
                return false;
            }
        }
    }
    true
}
pub(crate) fn service_network_capture_event(
    capture: &Map<String, Value>,
    request_id: &str,
    metadata: &Value,
    response: &Value,
    body_captured: bool,
    body: Option<Value>,
    max_body_bytes: u64,
) -> Value {
    let allowed_headers = service_network_allowed_header_names(capture);
    let include_request_headers = capture
        .get("includeRequestHeaders")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let include_response_headers = capture
        .get("includeResponseHeaders")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut event = json!(
        { "requestId" : request_id, "url" : response.get("url").or_else(|| metadata
        .get("url")).cloned().unwrap_or(Value::Null), "method" : metadata.get("method")
        .cloned().unwrap_or(Value::Null), "resourceType" : metadata.get("resourceType")
        .cloned().unwrap_or(Value::Null), "status" : response.get("status").cloned()
        .unwrap_or(Value::Null), "statusText" : response.get("statusText").cloned()
        .unwrap_or(Value::Null), "mimeType" : response.get("mimeType").cloned()
        .unwrap_or(Value::Null), "encodedDataLength" : response.get("encodedDataLength")
        .cloned().unwrap_or(Value::Null), "headersRedacted" : true, "body" : body
        .unwrap_or_else(|| json!({ "captured" : body_captured, "maxBodyBytes" : if
        max_body_bytes > 0 { json!(max_body_bytes) } else { Value::Null }, })), }
    );
    if include_request_headers {
        event["requestHeaders"] = filter_headers(metadata.get("requestHeaders"), &allowed_headers);
    }
    if include_response_headers {
        event["responseHeaders"] = filter_headers(response.get("headers"), &allowed_headers);
    }
    event
}
pub(crate) async fn service_network_capture_body(
    state: &DaemonState,
    request_id: &str,
    session_id: &str,
    max_body_bytes: u64,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let body_result = mgr
        .client
        .send_command(
            "Network.getResponseBody",
            Some(json!({ "requestId" : request_id })),
            Some(session_id),
        )
        .await?;
    let base64_encoded = body_result
        .get("base64Encoded")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let body = body_result
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("");
    let bytes = body.len() as u64;
    let truncated = bytes > max_body_bytes;
    let returned = if truncated {
        truncate_utf8(body, max_body_bytes as usize)
    } else {
        body.to_string()
    };
    if base64_encoded {
        Ok(json!(
            { "captured" : true, "base64Encoded" : true, "bodyBase64" : returned,
            "bodyBytes" : bytes, "bodyTruncated" : truncated, "maxBodyBytes" :
            max_body_bytes, }
        ))
    } else {
        Ok(json!(
            { "captured" : true, "base64Encoded" : false, "body" : returned,
            "bodyBytes" : bytes, "bodyTruncated" : truncated, "maxBodyBytes" :
            max_body_bytes, }
        ))
    }
}
pub(crate) fn service_network_allowed_header_names(
    capture: &Map<String, Value>,
) -> HashSet<String> {
    capture
        .get("allowedHeaderNames")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default()
}
pub(crate) fn filter_headers(headers: Option<&Value>, allowed_headers: &HashSet<String>) -> Value {
    let Some(headers) = headers.and_then(Value::as_object) else {
        return json!({});
    };
    if allowed_headers.is_empty() {
        return json!({});
    }
    let mut filtered = Map::new();
    for (key, value) in headers {
        if allowed_headers.contains(&key.to_ascii_lowercase()) {
            filtered.insert(key.clone(), value.clone());
        }
    }
    Value::Object(filtered)
}
