#![allow(unused_imports)]
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
use super::service_diagnostics::{handle_service_diagnostics, truncate_utf8};
use super::service_probe::probe_recipe_fingerprint;
use crate::native::interaction;
use crate::native::state;
use serde_json::{json, Map, Value};
use std::time::{Duration, Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) async fn handle_service_ui_action(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "ui_action requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let ui_action = cmd
        .get("uiAction")
        .and_then(Value::as_object)
        .ok_or_else(|| "ui_action requires uiAction object".to_string())?;
    let steps = ui_action
        .get("steps")
        .and_then(Value::as_array)
        .filter(|steps| !steps.is_empty())
        .ok_or_else(|| "ui_action requires uiAction.steps array".to_string())?;
    let max_actions = ui_action
        .get("maxActions")
        .and_then(Value::as_u64)
        .unwrap_or(10)
        .clamp(1, 20) as usize;
    if steps.len() > max_actions {
        return Err(format!(
            "ui_action step count {} exceeds maxActions {}",
            steps.len(),
            max_actions
        ));
    }
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .or_else(|| ui_action.get("timeoutMs").and_then(Value::as_u64))
        .ok_or_else(|| "ui_action requires positive timeoutMs".to_string())?;
    let max_text_bytes = cmd
        .get("maxTextBytes")
        .and_then(Value::as_u64)
        .or_else(|| ui_action.get("maxTextBytes").and_then(Value::as_u64))
        .unwrap_or(1024)
        .clamp(1, 16 * 1024);
    if timeout_ms == 0 {
        return Err("ui_action requires positive timeoutMs".to_string());
    }
    for step in steps {
        validate_service_ui_step(step)?;
    }
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "ui_action requires serviceTabHandle.targetId".to_string())?;
    {
        let mgr = state
            .browser
            .as_mut()
            .ok_or_else(|| {
                "Cannot run ui_action: target browser session is not running; request a service tab first"
                    .to_string()
            })?;
        if mgr.active_target_id().ok() != Some(target_id) {
            let _ = mgr.tab_switch_target_id(target_id).await?;
        }
    }
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let before = service_ui_current_page(state).await;
    let mut results = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        match run_service_ui_step(cmd, state, step, index, timeout_ms, max_text_bytes).await {
            Ok(result) => results.push(result),
            Err(error) => {
                let failure_page = service_ui_current_page(state).await;
                let diagnostics = if ui_action
                    .get("includeDiagnosticsOnFailure")
                    .or_else(|| cmd.get("captureEvidenceOnFailure"))
                    .and_then(Value::as_bool)
                    == Some(true)
                {
                    handle_service_diagnostics(cmd, state).await.unwrap_or_else(
                        |diagnostic_error| json!({ "ok" : false, "error" : diagnostic_error, }),
                    )
                } else {
                    Value::Null
                };
                results.push(json!(
                    { "index" : index, "type" : step.get("type")
                    .and_then(Value::as_str).unwrap_or("unknown"), "ok" : false,
                    "error" : error, "page" : failure_page, }
                ));
                return Ok(json!(
                    { "ok" : false, "action" : "ui_action", "observedAt" :
                    observed_at, "failedStepIndex" : index, "targetId" : target_id,
                    "tabId" : handle.get("tabId").cloned().unwrap_or(Value::Null),
                    "profileId" : handle.get("profileId").cloned()
                    .unwrap_or(Value::Null), "serviceTabHandle" : cmd
                    .get("serviceTabHandle").cloned().unwrap_or(Value::Null),
                    "traceFilter" : handle.get("traceFilter").cloned()
                    .unwrap_or(Value::Null), "uiAction" :
                    service_ui_action_summary(ui_action, steps.len(), max_actions,
                    timeout_ms, max_text_bytes), "before" : before, "after" :
                    failure_page, "diagnostics" : diagnostics, "steps" : results,
                    "caller" : service_ui_caller(cmd), }
                ));
            }
        }
    }
    let after = service_ui_current_page(state).await;
    Ok(json!(
        { "ok" : true, "action" : "ui_action", "observedAt" : observed_at, "targetId"
        : target_id, "tabId" : handle.get("tabId").cloned().unwrap_or(Value::Null),
        "profileId" : handle.get("profileId").cloned().unwrap_or(Value::Null),
        "serviceTabHandle" : cmd.get("serviceTabHandle").cloned()
        .unwrap_or(Value::Null), "traceFilter" : handle.get("traceFilter").cloned()
        .unwrap_or(Value::Null), "uiAction" : service_ui_action_summary(ui_action,
        steps.len(), max_actions, timeout_ms, max_text_bytes), "before" : before,
        "after" : after, "steps" : results, "caller" : service_ui_caller(cmd), }
    ))
}
pub(crate) fn service_ui_action_summary(
    ui_action: &Map<String, Value>,
    step_count: usize,
    max_actions: usize,
    timeout_ms: u64,
    max_text_bytes: u64,
) -> Value {
    json!(
        { "stepCount" : step_count, "maxActions" : max_actions, "timeoutMs" : timeout_ms,
        "maxTextBytes" : max_text_bytes, "recipeId" : ui_action.get("recipeId").cloned()
        .unwrap_or(Value::Null), "recipeFingerprint" :
        probe_recipe_fingerprint(ui_action), }
    )
}
pub(crate) fn validate_service_ui_step(step: &Value) -> Result<(), String> {
    let step = step
        .as_object()
        .ok_or_else(|| "ui_action step must be an object".to_string())?;
    let step_type = step
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "ui_action step requires type".to_string())?;
    match step_type {
        "find" | "click" | "fill" | "type" | "select" | "wait" | "focus" | "clear" | "dialog"
        | "menu_select" => {}
        _ => return Err(format!("unsupported ui_action step type: {step_type}")),
    }
    if matches!(
        step_type,
        "find" | "click" | "fill" | "type" | "select" | "focus" | "clear" | "menu_select"
    ) && step
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(format!("ui_action {step_type} step requires selector"));
    }
    if step_type == "fill" && step.get("value").and_then(Value::as_str).is_none() {
        return Err("ui_action fill step requires value".to_string());
    }
    if step_type == "type" && step.get("text").and_then(Value::as_str).is_none() {
        return Err("ui_action type step requires text".to_string());
    }
    if step_type == "select" && step.get("value").or_else(|| step.get("values")).is_none() {
        return Err("ui_action select step requires value or values".to_string());
    }
    if step_type == "wait"
        && ["selector", "text", "url", "function", "loadState"]
            .iter()
            .all(|field| step.get(*field).is_none())
    {
        return Err(
            "ui_action wait step requires selector, text, url, function, or loadState".to_string(),
        );
    }
    if step_type == "menu_select"
        && step
            .get("optionSelector")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err("ui_action menu_select step requires optionSelector".to_string());
    }
    if step_type == "dialog" {
        let response = step
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("status");
        if !matches!(response, "status" | "accept" | "dismiss") {
            return Err("ui_action dialog response must be status, accept, or dismiss".to_string());
        }
    }
    Ok(())
}
pub(crate) async fn run_service_ui_step(
    top_cmd: &Value,
    state: &mut DaemonState,
    step: &Value,
    index: usize,
    timeout_ms: u64,
    max_text_bytes: u64,
) -> Result<Value, String> {
    let step_type = step
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "ui_action step requires type".to_string())?;
    let started_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let mut cmd = step.clone();
    if let Some(object) = cmd.as_object_mut() {
        object.insert("timeoutMs".to_string(), json!(timeout_ms));
        for key in ["serviceName", "agentName", "taskName"] {
            if let Some(value) = top_cmd.get(key) {
                object.insert(key.to_string(), value.clone());
            }
        }
    }
    let result = match step_type {
        "find" => run_service_ui_find_step(state, step, timeout_ms, max_text_bytes).await?,
        "click" => handle_click(&cmd, state).await?,
        "fill" => handle_fill(&cmd, state).await?,
        "type" => handle_type(&cmd, state).await?,
        "select" => handle_select(&cmd, state).await?,
        "wait" => handle_wait(&cmd, state).await?,
        "focus" => handle_focus(&cmd, state).await?,
        "clear" => handle_clear(&cmd, state).await?,
        "dialog" => run_service_ui_dialog_step(&cmd, state).await?,
        "menu_select" => {
            handle_click(&cmd, state).await?;
            let mut option = cmd.clone();
            if let Some(object) = option.as_object_mut() {
                object.insert(
                    "selector".to_string(),
                    step.get("optionSelector").cloned().unwrap_or(Value::Null),
                );
            }
            handle_click(&option, state).await?
        }
        _ => return Err(format!("unsupported ui_action step type: {step_type}")),
    };
    let completed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let page = service_ui_current_page(state).await;
    Ok(json!(
        { "index" : index, "type" : step_type, "id" : step.get("id").cloned()
        .unwrap_or(Value::Null), "ok" : true, "startedAt" : started_at, "completedAt"
        : completed_at, "selector" : step.get("selector").cloned()
        .unwrap_or(Value::Null), "result" : result, "page" : page, }
    ))
}
pub(crate) async fn run_service_ui_find_step(
    state: &mut DaemonState,
    step: &Value,
    timeout_ms: u64,
    max_text_bytes: u64,
) -> Result<Value, String> {
    let selector = step
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| "ui_action find step requires selector".to_string())?;
    let max_candidates = step
        .get("maxCandidates")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 50);
    let selector_json =
        serde_json::to_string(selector).map_err(|err| format!("Invalid selector: {err}"))?;
    let expression = format!(
        r#"(() => {{
const nodes = Array.from(document.querySelectorAll({selector_json})).slice(0, {max_candidates});
return nodes.map((node) => {{
  const style = window.getComputedStyle(node);
  const rect = node.getBoundingClientRect();
  const visible = style.display !== 'none' && style.visibility !== 'hidden' && Number.parseFloat(style.opacity || '1') !== 0 && rect.width > 0 && rect.height > 0;
  const text = String(node.innerText || node.textContent || '').replace(/\s+/g, ' ').trim();
  return {{
    tagName: node.tagName ? node.tagName.toLowerCase() : null,
    text,
    visible,
    disabled: Boolean(node.disabled),
    rect: {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }},
    role: node.getAttribute('role'),
    ariaLabel: node.getAttribute('aria-label'),
  }};
}});
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms),
        mgr.evaluate(&expression, None),
    )
    .await
    .map_err(|_| format!("ui_action find timed out after {timeout_ms}ms"))??;
    let mut value = result
        .pointer("/result/value")
        .cloned()
        .unwrap_or_else(|| result.clone());
    if let Some(items) = value.as_array_mut() {
        for item in items {
            let original = item
                .get("text")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            if let Some(text) = original {
                item["text"] = json!(truncate_utf8(&text, max_text_bytes as usize));
            }
        }
    }
    Ok(json!(
        { "selector" : selector, "maxCandidates" : max_candidates, "candidates" :
        value, }
    ))
}
pub(crate) async fn run_service_ui_dialog_step(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let response = cmd
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("status");
    if response == "status" {
        return handle_dialog(cmd, state).await;
    }
    let allowed = cmd
        .get("allowedDialogLabels")
        .or_else(|| cmd.get("allowedLabels"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .unwrap_or(1000)
        .clamp(100, 10_000);
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    while state.pending_dialog.is_none() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
    }
    let Some(dialog) = state.pending_dialog.as_ref() else {
        return Err("ui_action dialog step found no pending dialog".to_string());
    };
    if allowed.is_empty()
        || !allowed
            .iter()
            .any(|label| dialog.message.contains(label.as_str()))
    {
        return Err("ui_action dialog step requires an allowedDialogLabels match".to_string());
    }
    match tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms),
        handle_dialog(cmd, state),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err("ui_action dialog step timed out while handling dialog".to_string()),
    }
}
pub(crate) async fn service_ui_current_page(state: &mut DaemonState) -> Value {
    let Some(mgr) = state.browser.as_mut() else {
        return json!({ "url" : Value::Null, "title" : Value::Null, });
    };
    let url = mgr.get_url().await.unwrap_or_default();
    let title = mgr.get_title().await.unwrap_or_default();
    json!({ "url" : url, "title" : title, })
}
pub(crate) fn service_ui_caller(cmd: &Value) -> Value {
    json!(
        { "serviceName" : cmd.get("serviceName").cloned().unwrap_or(Value::Null),
        "agentName" : cmd.get("agentName").cloned().unwrap_or(Value::Null), "taskName" :
        cmd.get("taskName").cloned().unwrap_or(Value::Null), "jobId" : cmd.get("id")
        .cloned().unwrap_or(Value::Null), }
    )
}
