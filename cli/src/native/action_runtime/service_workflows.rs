#![allow(unused_imports)]
use super::browser_operations::{
    handle_clear, handle_click, handle_dialog, handle_fill, handle_focus, handle_reload,
    handle_select, handle_type, handle_wait, matches_status_filter,
};
use super::common::*;
use super::runtime::{
    service_browser_id, validate_service_tab_handle_for_current_session, DaemonState,
    RuntimeHandoffDescriptor, TrackedRequest,
};
pub(crate) async fn handle_service_probe(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "probe requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let probe = cmd
        .get("probe")
        .and_then(Value::as_object)
        .ok_or_else(|| "probe requires probe object".to_string())?;
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .or_else(|| probe.get("timeoutMs").and_then(Value::as_u64))
        .ok_or_else(|| "probe requires positive timeoutMs".to_string())?;
    let max_return_bytes = cmd
        .get("maxReturnBytes")
        .and_then(Value::as_u64)
        .or_else(|| probe.get("maxReturnBytes").and_then(Value::as_u64))
        .ok_or_else(|| "probe requires positive maxReturnBytes".to_string())?;
    if timeout_ms == 0 || max_return_bytes == 0 {
        return Err("probe requires positive timeoutMs and maxReturnBytes".to_string());
    }
    let detectors = probe
        .get("detectors")
        .and_then(Value::as_array)
        .filter(|detectors| !detectors.is_empty())
        .ok_or_else(|| "probe requires at least one detector".to_string())?;
    let max_detectors = probe
        .get("maxDetectors")
        .and_then(Value::as_u64)
        .unwrap_or(10)
        .clamp(1, 25) as usize;
    if detectors.len() > max_detectors {
        return Err(format!(
            "probe detector count {} exceeds maxDetectors {}",
            detectors.len(),
            max_detectors
        ));
    }
    let mgr = state.browser.as_mut().ok_or_else(|| {
        "Cannot probe: target browser session is not running; request a service tab first"
            .to_string()
    })?;
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "probe requires serviceTabHandle.targetId".to_string())?;
    if mgr.active_target_id().ok() != Some(target_id) {
        let _ = mgr.tab_switch_target_id(target_id).await?;
    }
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let url = mgr.get_url().await.unwrap_or_default();
    let title = mgr.get_title().await.unwrap_or_default();
    let mut results = Vec::new();
    for detector in detectors {
        results.push(
            run_service_probe_detector(mgr, detector, timeout_ms, max_return_bytes)
                .await
                .unwrap_or_else(|error| json!({ "ok" : false, "error" : error, })),
        );
    }
    let identity = normalize_probe_identity(&results, probe.get("expectedIdentity"));
    let freshness = record_probe_freshness(cmd, probe, handle, &identity, &observed_at)?;
    Ok(json!(
        { "ok" : true, "action" : "probe", "observedAt" : observed_at, "url" : url,
        "title" : title, "targetId" : target_id, "tabId" : handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "profileId" : handle.get("profileId")
        .cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "traceFilter" :
        handle.get("traceFilter").cloned().unwrap_or(Value::Null), "probe" : {
        "detectorCount" : detectors.len(), "maxReturnBytes" : max_return_bytes,
        "timeoutMs" : timeout_ms, "recipeId" : probe.get("recipeId").cloned()
        .unwrap_or(Value::Null), "recipeFingerprint" :
        probe_recipe_fingerprint(probe), }, "identity" : identity, "detectors" :
        results, "freshness" : freshness, "caller" : { "serviceName" : cmd
        .get("serviceName").cloned().unwrap_or(Value::Null), "agentName" : cmd
        .get("agentName").cloned().unwrap_or(Value::Null), "taskName" : cmd
        .get("taskName").cloned().unwrap_or(Value::Null), "jobId" : cmd.get("id")
        .cloned().unwrap_or(Value::Null), }, }
    ))
}
pub(crate) async fn run_service_probe_detector(
    mgr: &mut BrowserManager,
    detector: &Value,
    timeout_ms: u64,
    max_return_bytes: u64,
) -> Result<Value, String> {
    let detector = detector
        .as_object()
        .ok_or_else(|| "probe detector must be an object".to_string())?;
    let detector_id = detector
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("detector");
    let detector_type = detector
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "probe detector requires type".to_string())?;
    match detector_type {
        "evaluate" => {
            let expression = detector
                .get("expression")
                .or_else(|| detector.get("script"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "evaluate probe detector requires expression".to_string())?;
            let result = run_probe_evaluate(mgr, expression, timeout_ms, max_return_bytes).await?;
            Ok(json!(
                { "id" : detector_id, "type" : "evaluate", "ok" : true, "result" :
                result.value, "resultTruncated" : result.truncated, "resultBytes" :
                result.bytes, "maxReturnBytes" : max_return_bytes, }
            ))
        }
        "url_title" => {
            let url = mgr.get_url().await.unwrap_or_default();
            let title = mgr.get_title().await.unwrap_or_default();
            Ok(json!(
                { "id" : detector_id, "type" : "url_title", "ok" : true, "url" : url,
                "title" : title, }
            ))
        }
        "selector_text" => {
            let selector = detector
                .get("selector")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "selector_text probe detector requires selector".to_string())?;
            let max_text_bytes = detector
                .get("maxTextBytes")
                .and_then(Value::as_u64)
                .unwrap_or(max_return_bytes)
                .min(max_return_bytes);
            let selector_json = serde_json::to_string(selector)
                .map_err(|err| format!("Invalid selector: {err}"))?;
            let expression = format!(
                r#"(() => {{
const node = document.querySelector({selector_json});
if (!node) return {{ matched: false, text: null, visible: false }};
const style = window.getComputedStyle(node);
const rect = node.getBoundingClientRect();
const visible = style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0' && rect.width > 0 && rect.height > 0;
const text = String(node.innerText || node.textContent || '').replace(/\s+/g, ' ').trim();
return {{ matched: true, text, visible, tagName: node.tagName ? node.tagName.toLowerCase() : null }};
}})()"#
            );
            let result = run_probe_evaluate(mgr, &expression, timeout_ms, max_text_bytes).await?;
            Ok(json!(
                { "id" : detector_id, "type" : "selector_text", "ok" : true,
                "selector" : selector, "result" : result.value, "resultTruncated" :
                result.truncated, "resultBytes" : result.bytes, "maxReturnBytes" :
                max_text_bytes, }
            ))
        }
        "client_evidence" => Ok(json!(
            { "id" : detector_id, "type" : "client_evidence", "ok" : true,
            "evidence" : detector.get("evidence").cloned()
            .unwrap_or(Value::Null), }
        )),
        _ => Err(format!("unsupported probe detector type: {detector_type}")),
    }
}
pub(crate) struct ProbeEvalResult {
    pub(crate) value: Value,
    pub(crate) truncated: bool,
    pub(crate) bytes: u64,
}
pub(crate) async fn run_probe_evaluate(
    mgr: &BrowserManager,
    expression: &str,
    timeout_ms: u64,
    max_return_bytes: u64,
) -> Result<ProbeEvalResult, String> {
    let outcome = tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms),
        mgr.evaluate(expression, None),
    )
    .await;
    let result = match outcome {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return Err(error),
        Err(_) => return Err(format!("probe detector timed out after {timeout_ms}ms")),
    };
    let value = result
        .pointer("/result/value")
        .cloned()
        .unwrap_or_else(|| result.clone());
    let serialized = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
    let bytes = serialized.len() as u64;
    let truncated = bytes > max_return_bytes;
    let returned = if truncated {
        Value::String(truncate_utf8(&serialized, max_return_bytes as usize))
    } else {
        value
    };
    Ok(ProbeEvalResult {
        value: returned,
        truncated,
        bytes,
    })
}
pub(crate) fn normalize_probe_identity(
    results: &[Value],
    expected_identity: Option<&Value>,
) -> Value {
    let mut detected_identity = None;
    let mut detected_account_id = None;
    let mut confidence = None;
    let mut source = None;
    for result in results {
        let mut found_identity = false;
        let candidates = [
            result.pointer("/result/detectedIdentity"),
            result.pointer("/result/identity"),
            result.pointer("/result/email"),
            result.pointer("/result/accountId"),
            result.pointer("/evidence/detectedIdentity"),
            result.pointer("/evidence/accountId"),
        ];
        for candidate in candidates.into_iter().flatten() {
            if let Some(value) = candidate.as_str().filter(|value| !value.trim().is_empty()) {
                detected_identity.get_or_insert_with(|| value.trim().to_string());
                detected_account_id.get_or_insert_with(|| value.trim().to_string());
                found_identity = true;
                break;
            }
        }
        if let Some(value) = result
            .pointer("/result/confidence")
            .or_else(|| result.pointer("/evidence/confidence"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            confidence.get_or_insert_with(|| value.trim().to_string());
        }
        if found_identity {
            source.get_or_insert_with(|| {
                result
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("probe")
                    .to_string()
            });
        }
    }
    json!(
        { "detectedIdentity" : detected_identity, "detectedAccountId" :
        detected_account_id, "expectedIdentity" : expected_identity.cloned()
        .unwrap_or(Value::Null), "confidence" : confidence.unwrap_or_else(|| "unknown"
        .to_string()), "source" : source.unwrap_or_else(|| "probe".to_string()), }
    )
}
pub(crate) fn record_probe_freshness(
    cmd: &Value,
    probe: &Map<String, Value>,
    handle: &Map<String, Value>,
    identity: &Value,
    observed_at: &str,
) -> Result<Value, String> {
    let Some(record_freshness) = probe.get("recordFreshness") else {
        return Ok(Value::Null);
    };
    let record = record_freshness
        .as_object()
        .ok_or_else(|| "probe recordFreshness must be an object".to_string())?;
    let target_service_id = record
        .get("targetServiceId")
        .or_else(|| cmd.get("targetServiceId"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "probe recordFreshness requires targetServiceId".to_string())?;
    let account_id = record
        .get("accountId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "probe recordFreshness requires accountId".to_string())?;
    let profile_id = record
        .get("profileId")
        .or_else(|| handle.get("profileId"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "probe recordFreshness requires profileId".to_string())?;
    let readiness_state = record
        .get("readinessState")
        .and_then(Value::as_str)
        .unwrap_or("fresh");
    let evidence = json!(
        { "kind" : "service_probe", "observedAt" : observed_at, "targetServiceId" :
        target_service_id, "accountId" : account_id, "detectedIdentity" : identity
        .get("detectedIdentity").cloned().unwrap_or(Value::Null), "detectedAccountId" :
        identity.get("detectedAccountId").cloned().unwrap_or(Value::Null), "confidence" :
        identity.get("confidence").cloned().unwrap_or(Value::Null), "source" : identity
        .get("source").cloned().unwrap_or(Value::Null), "recipeId" : probe
        .get("recipeId").cloned().unwrap_or(Value::Null), }
    );
    let body = json!(
        { "targetServiceId" : target_service_id, "accountId" : account_id,
        "readinessState" : readiness_state, "readinessEvidence" : serde_json::to_string(&
        evidence).unwrap_or_else(| _ | "service_probe".to_string()),
        "readinessRecommendedAction" : record.get("readinessRecommendedAction")
        .and_then(Value::as_str)
        .unwrap_or("profile_freshness_verified_by_service_probe"), "lastVerifiedAt" :
        observed_at, "freshnessExpiresAt" : record.get("freshnessExpiresAt").cloned()
        .unwrap_or(Value::Null), "updateAuthenticatedServiceIds" : record
        .get("updateAuthenticatedServiceIds").and_then(Value::as_bool).unwrap_or(true), }
    );
    let profile = update_persisted_profile_freshness(profile_id, body)?;
    Ok(json!(
        { "recorded" : true, "profileId" : profile_id, "targetServiceId" :
        target_service_id, "accountId" : account_id, "profile" : profile, }
    ))
}
pub(crate) fn probe_recipe_fingerprint(probe: &Map<String, Value>) -> String {
    let raw = serde_json::to_string(probe).unwrap_or_else(|_| "{}".to_string());
    let digest = Sha256::digest(raw.as_bytes());
    format!("{digest:x}").chars().take(16).collect()
}
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
pub(crate) async fn handle_service_network_capture(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "network_capture requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
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
pub(crate) async fn handle_service_file_transfer(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "file_transfer requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let transfer = cmd
        .get("fileTransfer")
        .and_then(Value::as_object)
        .ok_or_else(|| "file_transfer requires fileTransfer object".to_string())?;
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .or_else(|| transfer.get("timeoutMs").and_then(Value::as_u64))
        .ok_or_else(|| "file_transfer requires positive timeoutMs".to_string())?;
    if timeout_ms == 0 {
        return Err("file_transfer requires positive timeoutMs".to_string());
    }
    if transfer.get("upload").is_none() && transfer.get("download").is_none() {
        return Err("file_transfer requires upload or download recipe".to_string());
    }
    validate_service_file_transfer_recipe(transfer)?;
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer requires serviceTabHandle.targetId".to_string())?;
    {
        let mgr = state
            .browser
            .as_mut()
            .ok_or_else(|| {
                "Cannot run file_transfer: target browser session is not running; request a service tab first"
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
    let mut upload_result = Value::Null;
    let mut download_result = Value::Null;
    if let Some(upload) = transfer.get("upload").and_then(Value::as_object) {
        match run_service_file_upload(upload, state).await {
            Ok(result) => upload_result = result,
            Err(error) => {
                return service_file_transfer_failure(
                    cmd,
                    state,
                    ServiceFileTransferFailure {
                        handle,
                        transfer,
                        target_id,
                        observed_at: &observed_at,
                        before,
                        phase: "upload",
                        error,
                    },
                )
                .await;
            }
        }
    }
    if let Some(download) = transfer.get("download").and_then(Value::as_object) {
        match run_service_download_capture(download, state, timeout_ms).await {
            Ok(result) => download_result = result,
            Err(error) => {
                return service_file_transfer_failure(
                    cmd,
                    state,
                    ServiceFileTransferFailure {
                        handle,
                        transfer,
                        target_id,
                        observed_at: &observed_at,
                        before,
                        phase: "download",
                        error,
                    },
                )
                .await;
            }
        }
    }
    let after = service_ui_current_page(state).await;
    Ok(json!(
        { "ok" : true, "action" : "file_transfer", "observedAt" : observed_at,
        "targetId" : target_id, "tabId" : handle.get("tabId").cloned()
        .unwrap_or(Value::Null), "profileId" : handle.get("profileId").cloned()
        .unwrap_or(Value::Null), "serviceTabHandle" : cmd.get("serviceTabHandle")
        .cloned().unwrap_or(Value::Null), "traceFilter" : handle.get("traceFilter")
        .cloned().unwrap_or(Value::Null), "fileTransfer" :
        service_file_transfer_summary(transfer, timeout_ms), "before" : before,
        "after" : after, "upload" : upload_result, "download" : download_result,
        "caller" : service_ui_caller(cmd), }
    ))
}
pub(crate) struct ServiceFileTransferFailure<'a> {
    pub(crate) handle: &'a Map<String, Value>,
    pub(crate) transfer: &'a Map<String, Value>,
    pub(crate) target_id: &'a str,
    pub(crate) observed_at: &'a str,
    pub(crate) before: Value,
    pub(crate) phase: &'a str,
    pub(crate) error: String,
}
pub(crate) async fn service_file_transfer_failure(
    cmd: &Value,
    state: &mut DaemonState,
    failure: ServiceFileTransferFailure<'_>,
) -> Result<Value, String> {
    let after = service_ui_current_page(state).await;
    let diagnostics = if failure
        .transfer
        .get("includeDiagnosticsOnFailure")
        .or_else(|| cmd.get("captureEvidenceOnFailure"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        handle_service_diagnostics(cmd, state)
            .await
            .unwrap_or_else(|diagnostic_error| json!({ "ok" : false, "error" : diagnostic_error, }))
    } else {
        Value::Null
    };
    Ok(json!(
        { "ok" : false, "action" : "file_transfer", "observedAt" : failure
        .observed_at, "failedPhase" : failure.phase, "error" : failure.error,
        "targetId" : failure.target_id, "tabId" : failure.handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "profileId" : failure.handle
        .get("profileId").cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "traceFilter" :
        failure.handle.get("traceFilter").cloned().unwrap_or(Value::Null),
        "fileTransfer" : service_file_transfer_summary(failure.transfer, cmd
        .get("timeoutMs").and_then(Value::as_u64).or_else(|| failure.transfer
        .get("timeoutMs").and_then(Value::as_u64)).unwrap_or(0),), "before" : failure
        .before, "after" : after, "diagnostics" : diagnostics, "caller" :
        service_ui_caller(cmd), }
    ))
}
pub(crate) fn service_file_transfer_summary(
    transfer: &Map<String, Value>,
    timeout_ms: u64,
) -> Value {
    json!(
        { "hasUpload" : transfer.get("upload").is_some(), "hasDownload" : transfer
        .get("download").is_some(), "timeoutMs" : timeout_ms, "recipeId" : transfer
        .get("recipeId").cloned().unwrap_or(Value::Null), "recipeFingerprint" :
        probe_recipe_fingerprint(transfer), }
    )
}
pub(crate) fn validate_service_file_transfer_recipe(
    transfer: &Map<String, Value>,
) -> Result<(), String> {
    if let Some(upload) = transfer.get("upload") {
        let upload = upload
            .as_object()
            .ok_or_else(|| "file_transfer upload must be an object".to_string())?;
        validate_service_file_upload_recipe(upload)?;
    }
    if let Some(download) = transfer.get("download") {
        let download = download
            .as_object()
            .ok_or_else(|| "file_transfer download must be an object".to_string())?;
        validate_service_download_recipe(download)?;
    }
    Ok(())
}
pub(crate) fn validate_service_file_upload_recipe(
    upload: &Map<String, Value>,
) -> Result<(), String> {
    if upload
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
        && upload
            .get("labelText")
            .or_else(|| upload.get("label"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return Err("file_transfer upload requires selector or labelText".to_string());
    }
    let files = upload
        .get("files")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "file_transfer upload requires files array".to_string())?;
    if !files
        .iter()
        .all(|file| file.as_str().is_some_and(|value| !value.trim().is_empty()))
    {
        return Err("file_transfer upload files must be nonempty strings".to_string());
    }
    let max_files = upload
        .get("maxFiles")
        .and_then(Value::as_u64)
        .ok_or_else(|| "file_transfer upload requires positive maxFiles".to_string())?;
    if max_files == 0 {
        return Err("file_transfer upload requires positive maxFiles".to_string());
    }
    if files.len() as u64 > max_files {
        return Err(format!(
            "file_transfer upload file count {} exceeds maxFiles {}",
            files.len(),
            max_files
        ));
    }
    validate_nonempty_string_array(
        upload.get("allowedPaths"),
        "file_transfer upload allowedPaths",
    )
}
pub(crate) fn validate_service_download_recipe(
    download: &Map<String, Value>,
) -> Result<(), String> {
    if download
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err("file_transfer download requires selector".to_string());
    }
    if download
        .get("directory")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err("file_transfer download requires directory".to_string());
    }
    validate_nonempty_string_array(
        download.get("allowedDirectories"),
        "file_transfer download allowedDirectories",
    )?;
    if let Some(max_bytes) = download.get("maxBytes").and_then(Value::as_u64) {
        if max_bytes == 0 {
            return Err("file_transfer download maxBytes must be positive".to_string());
        }
    }
    Ok(())
}
pub(crate) fn validate_nonempty_string_array(
    value: Option<&Value>,
    label: &str,
) -> Result<(), String> {
    let valid = value
        .and_then(Value::as_array)
        .filter(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|text| !text.trim().is_empty()))
        })
        .is_some();
    if !valid {
        return Err(format!("{label} must be a nonempty string array"));
    }
    Ok(())
}
pub(crate) async fn run_service_file_upload(
    upload: &Map<String, Value>,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let selector = resolve_service_file_input_selector(upload, state).await?;
    let allowed_paths = service_canonical_allowed_paths(upload.get("allowedPaths"))?;
    let files = upload
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "file_transfer upload requires files array".to_string())?;
    let mut resolved_files = Vec::new();
    let mut file_items = Vec::new();
    for file in files {
        let raw = file
            .as_str()
            .ok_or_else(|| "file_transfer upload files must be strings".to_string())?;
        let path = service_existing_path(raw)?;
        service_require_allowed_path(&path, &allowed_paths, "upload file")?;
        let metadata = fs::metadata(&path)
            .map_err(|err| format!("Failed to read upload file metadata: {err}"))?;
        resolved_files.push(path.to_string_lossy().to_string());
        file_items.push(json!(
            { "name" : path.file_name().and_then(| value | value.to_str())
            .unwrap_or(""), "path" : path.to_string_lossy().to_string(), "size" :
            metadata.len(), }
        ));
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    mgr.upload_files(
        &selector,
        &resolved_files,
        &state.ref_map,
        &state.iframe_sessions,
    )
    .await?;
    let selected = if upload
        .get("verifySelectedNames")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        service_file_input_selected_names(&selector, state).await?
    } else {
        Value::Null
    };
    Ok(json!(
        { "ok" : true, "selector" : selector, "uploaded" : resolved_files.len(),
        "files" : file_items, "selectedFileNames" : selected, }
    ))
}
pub(crate) async fn resolve_service_file_input_selector(
    upload: &Map<String, Value>,
    state: &mut DaemonState,
) -> Result<String, String> {
    if let Some(selector) = upload
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(selector.to_string());
    }
    let label_text = upload
        .get("labelText")
        .or_else(|| upload.get("label"))
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer upload requires selector or labelText".to_string())?;
    let label_json = serde_json::to_string(label_text).unwrap_or_else(|_| "\"\"".to_string());
    let expression = format!(
        r#"(() => {{
const expected = String({label_json}).trim().toLowerCase();
for (const input of Array.from(document.querySelectorAll('input[type="file"]'))) {{
  const labels = Array.from(input.labels || []);
  const text = labels.map((label) => String(label.innerText || label.textContent || '')).join(' ').replace(/\s+/g, ' ').trim().toLowerCase();
  if (text.includes(expected)) {{
    const token = input.getAttribute('data-agent-browser-file-input-id') || `file-input-${{Date.now()}}-${{Math.random().toString(16).slice(2)}}`;
    input.setAttribute('data-agent-browser-file-input-id', token);
    return `[data-agent-browser-file-input-id="${{token}}"]`;
  }}
}}
return null;
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = mgr.evaluate(&expression, None).await?;
    result
        .pointer("/result/value")
        .or_else(|| result.pointer("/result/result/value"))
        .or_else(|| result.get("value"))
        .or(Some(&result))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "file_transfer upload could not resolve labelText to file input: {}",
                result
            )
        })
}
pub(crate) async fn service_file_input_selected_names(
    selector: &str,
    state: &DaemonState,
) -> Result<Value, String> {
    let selector_json = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string());
    let expression = format!(
        r#"(() => {{
const input = document.querySelector({selector_json});
if (!input || !input.files) return [];
return Array.from(input.files).map((file) => file.name);
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = mgr.evaluate(&expression, None).await?;
    Ok(result
        .pointer("/result/value")
        .or_else(|| result.pointer("/result/result/value"))
        .or_else(|| result.get("value"))
        .or(Some(&result))
        .cloned()
        .unwrap_or_else(|| json!([])))
}
pub(crate) async fn run_service_download_capture(
    download: &Map<String, Value>,
    state: &mut DaemonState,
    timeout_ms: u64,
) -> Result<Value, String> {
    let selector = download
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer download requires selector".to_string())?;
    let directory = download
        .get("directory")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer download requires directory".to_string())?;
    let download_dir = service_prepare_allowed_download_dir(directory, download)?;
    let download_dir_str = download_dir
        .to_str()
        .ok_or("Download directory path is not valid UTF-8")?;
    let max_bytes = download.get("maxBytes").and_then(Value::as_u64);
    if download
        .get("captureMode")
        .and_then(Value::as_str)
        .unwrap_or("fetch")
        != "browser"
    {
        return run_service_download_fetch_capture(download, state, &download_dir, max_bytes).await;
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms.min(1000)),
        mgr.set_download_behavior(download_dir_str),
    )
    .await
    .map_err(|_| "file_transfer set download behavior timed out".to_string())??;
    let mut rx = mgr.client.subscribe();
    tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms.min(1000)),
        interaction::click(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            "left",
            1,
            &state.iframe_sessions,
        ),
    )
    .await
    .map_err(|_| "file_transfer download click timed out".to_string())??;
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    let mut downloaded_guid: Option<String> = None;
    let mut source_url: Option<String> = None;
    let mut canceled_event = false;
    let mut suggested_filename = download
        .get("expectedFileName")
        .or_else(|| download.get("expectedFilename"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("Timeout waiting for file_transfer download".to_string());
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                let is_page_session = event.session_id.as_deref() == Some(&session_id);
                let is_download_event = |method: &str, browser_method: &str, page_method: &str| {
                    method == browser_method || (method == page_method && is_page_session)
                };
                if is_download_event(
                    &event.method,
                    "Browser.downloadWillBegin",
                    "Page.downloadWillBegin",
                ) {
                    if let Some(guid) = event.params.get("guid").and_then(Value::as_str) {
                        downloaded_guid = Some(guid.to_string());
                    }
                    if source_url.is_none() {
                        source_url = event
                            .params
                            .get("url")
                            .and_then(Value::as_str)
                            .map(ToString::to_string);
                    }
                    if suggested_filename.is_none() {
                        suggested_filename = event
                            .params
                            .get("suggestedFilename")
                            .and_then(Value::as_str)
                            .map(ToString::to_string);
                    }
                }
                if is_download_event(
                    &event.method,
                    "Browser.downloadProgress",
                    "Page.downloadProgress",
                ) {
                    match event.params.get("state").and_then(Value::as_str) {
                        Some("completed") => break,
                        Some("canceled") => {
                            canceled_event = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => return Err("Event stream closed".to_string()),
            Err(_) => {
                return Err("Timeout waiting for file_transfer download".to_string());
            }
        }
    }
    let file_name = suggested_filename
        .as_deref()
        .and_then(service_safe_file_name)
        .ok_or_else(|| "file_transfer download could not determine safe file name".to_string())?;
    let dest = download_dir.join(&file_name);
    if let Some(guid) = downloaded_guid.as_deref() {
        let guid_path = download_dir.join(guid);
        for _ in 0..10 {
            if guid_path.exists() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        if guid_path.exists() && guid_path != dest {
            fs::rename(&guid_path, &dest)
                .map_err(|err| format!("Failed to rename downloaded file: {err}"))?;
        }
    }
    if !dest.exists() && canceled_event {
        return Err("Download was canceled".to_string());
    }
    if !dest.exists() {
        return Err("Downloaded file not found at captured path".to_string());
    }
    let metadata = fs::metadata(&dest)
        .map_err(|err| format!("Failed to read downloaded file metadata: {err}"))?;
    if let Some(max_bytes) = max_bytes {
        if metadata.len() > max_bytes {
            return Err(format!(
                "Downloaded file size {} exceeds maxBytes {}",
                metadata.len(),
                max_bytes
            ));
        }
    }
    Ok(json!(
        { "ok" : true, "selector" : selector, "localPath" : dest.to_string_lossy()
        .to_string(), "fileName" : file_name, "size" : metadata.len(), "mimeType" :
        service_guess_mime_type(& dest), "sourceUrl" : source_url, "timedOut" :
        false, "canceledEvent" : canceled_event, "maxBytes" : max_bytes, }
    ))
}
pub(crate) fn service_canonical_allowed_paths(
    value: Option<&Value>,
) -> Result<Vec<PathBuf>, String> {
    let items = value
        .and_then(Value::as_array)
        .ok_or_else(|| "allowed paths must be an array".to_string())?;
    items
        .iter()
        .map(|item| {
            let path = item
                .as_str()
                .ok_or_else(|| "allowed paths must be strings".to_string())?;
            service_existing_path(path)
        })
        .collect()
}
pub(crate) fn service_existing_path(path: &str) -> Result<PathBuf, String> {
    let raw = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        env::current_dir()
            .map_err(|err| format!("Failed to get current directory: {err}"))?
            .join(path)
    };
    raw.canonicalize()
        .map_err(|err| format!("Failed to resolve path '{}': {err}", raw.display()))
}
pub(crate) fn service_require_allowed_path(
    path: &Path,
    allowed_paths: &[PathBuf],
    label: &str,
) -> Result<(), String> {
    if allowed_paths
        .iter()
        .any(|allowed| path == allowed || path.starts_with(allowed))
    {
        Ok(())
    } else {
        Err(format!("{label} is outside allowedPaths"))
    }
}
pub(crate) fn service_prepare_allowed_download_dir(
    directory: &str,
    download: &Map<String, Value>,
) -> Result<PathBuf, String> {
    let raw = if Path::new(directory).is_absolute() {
        PathBuf::from(directory)
    } else {
        env::current_dir()
            .map_err(|err| format!("Failed to get current directory: {err}"))?
            .join(directory)
    };
    fs::create_dir_all(&raw)
        .map_err(|err| format!("Failed to create download directory: {err}"))?;
    let canonical_dir = raw
        .canonicalize()
        .map_err(|err| format!("Failed to resolve download directory: {err}"))?;
    let allowed = service_canonical_allowed_paths(download.get("allowedDirectories"))?;
    if allowed.iter().any(|item| canonical_dir.starts_with(item)) {
        Ok(canonical_dir)
    } else {
        Err("download directory is outside allowedDirectories".to_string())
    }
}
pub(crate) fn service_safe_file_name(value: &str) -> Option<String> {
    let path = Path::new(value);
    let name = path.file_name()?.to_str()?.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        None
    } else {
        Some(name.to_string())
    }
}
pub(crate) async fn run_service_download_fetch_capture(
    download: &Map<String, Value>,
    state: &DaemonState,
    download_dir: &Path,
    max_bytes: Option<u64>,
) -> Result<Value, String> {
    let selector = download
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer download requires selector".to_string())?;
    let max_fetch_bytes = max_bytes.unwrap_or(10 * 1024 * 1024).min(10 * 1024 * 1024);
    let expected_file_name = download
        .get("expectedFileName")
        .or_else(|| download.get("expectedFilename"))
        .and_then(Value::as_str);
    let selector_json =
        serde_json::to_string(selector).map_err(|err| format!("Invalid selector: {err}"))?;
    let expected_json = serde_json::to_string(&expected_file_name)
        .map_err(|err| format!("Invalid file name: {err}"))?;
    let script = format!(
        r#"(async () => {{
const node = document.querySelector({selector_json});
if (!node) throw new Error('download selector not found');
const rawUrl = node.href || node.getAttribute('href');
if (!rawUrl) throw new Error('download selector has no href');
const url = new URL(rawUrl, window.location.href).toString();
const response = await fetch(url, {{ credentials: 'include' }});
const buffer = await response.arrayBuffer();
if (buffer.byteLength > {max_fetch_bytes}) throw new Error(`download exceeds maxBytes ${{buffer.byteLength}}`);
let binary = '';
const bytes = new Uint8Array(buffer);
const chunkSize = 0x8000;
for (let i = 0; i < bytes.length; i += chunkSize) {{
  binary += String.fromCharCode(...bytes.slice(i, i + chunkSize));
}}
const expected = {expected_json};
const attrName = node.getAttribute('download');
const pathName = new URL(response.url || url).pathname.split('/').filter(Boolean).pop();
return {{
  sourceUrl: response.url || url,
  status: response.status,
  ok: response.ok,
  fileName: expected || attrName || pathName || 'download',
  mimeType: response.headers.get('content-type'),
  size: buffer.byteLength,
  bodyBase64: btoa(binary),
}};
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = tokio::time::timeout(
        tokio::time::Duration::from_millis(
            download
                .get("fetchTimeoutMs")
                .and_then(Value::as_u64)
                .unwrap_or(10_000)
                .clamp(1, 60_000),
        ),
        mgr.evaluate(&script, None),
    )
    .await
    .map_err(|_| "file_transfer fetch download timed out".to_string())??;
    let payload = service_extract_evaluate_value(&result)
        .ok_or_else(|| format!("file_transfer fetch download returned no payload: {result}"))?;
    let file_name = payload
        .get("fileName")
        .and_then(Value::as_str)
        .and_then(service_safe_file_name)
        .ok_or_else(|| "file_transfer download could not determine safe file name".to_string())?;
    let body = payload
        .get("bodyBase64")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer fetch download returned no body".to_string())?;
    let bytes = BASE64_STANDARD
        .decode(body)
        .map_err(|err| format!("file_transfer fetch download body was invalid base64: {err}"))?;
    if let Some(max_bytes) = max_bytes {
        if bytes.len() as u64 > max_bytes {
            return Err(format!(
                "Downloaded file size {} exceeds maxBytes {}",
                bytes.len(),
                max_bytes
            ));
        }
    }
    let dest = download_dir.join(&file_name);
    fs::write(&dest, &bytes).map_err(|err| format!("Failed to write downloaded file: {err}"))?;
    Ok(json!(
        { "ok" : true, "selector" : selector, "captureMode" : "fetch", "localPath" :
        dest.to_string_lossy().to_string(), "fileName" : file_name, "size" : bytes
        .len(), "mimeType" : payload.get("mimeType").cloned().unwrap_or(Value::Null),
        "sourceUrl" : payload.get("sourceUrl").cloned().unwrap_or(Value::Null),
        "status" : payload.get("status").cloned().unwrap_or(Value::Null), "timedOut"
        : false, "maxBytes" : max_bytes, }
    ))
}
pub(crate) fn service_extract_evaluate_value(result: &Value) -> Option<&Value> {
    result
        .pointer("/result/value")
        .or_else(|| result.pointer("/result/result/value"))
        .or_else(|| result.get("value"))
        .or(Some(result))
}
pub(crate) fn service_guess_mime_type(path: &Path) -> Value {
    let mime = match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "txt" | "text" => "text/plain",
        "json" => "application/json",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "html" | "htm" => "text/html",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        _ => return Value::Null,
    };
    json!(mime)
}
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
pub(crate) fn runtime_handoff_path(session_name: &str) -> PathBuf {
    get_socket_dir().join(format!("{}.handoff.json", session_name))
}
pub(crate) fn write_runtime_handoff(
    descriptor: &RuntimeHandoffDescriptor,
) -> Result<PathBuf, String> {
    let path = runtime_handoff_path(&descriptor.session_name);
    let parent = path
        .parent()
        .ok_or_else(|| format!("Runtime handoff path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create runtime handoff directory {}: {}",
            parent.display(),
            error
        )
    })?;
    let staged = path.with_extension(format!("handoff.json.next-{}", std::process::id()));
    let payload = serde_json::to_vec_pretty(descriptor)
        .map_err(|error| format!("Failed to serialize runtime handoff: {}", error))?;
    fs::write(&staged, payload).map_err(|error| {
        format!(
            "Failed to stage runtime handoff {}: {}",
            staged.display(),
            error
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).map_err(|error| {
            format!(
                "Failed to secure runtime handoff {}: {}",
                staged.display(),
                error
            )
        })?;
    }
    if path.exists() {
        fs::remove_file(&path).map_err(|error| {
            format!(
                "Failed to replace runtime handoff {}: {}",
                path.display(),
                error
            )
        })?;
    }
    fs::rename(&staged, &path).map_err(|error| {
        format!(
            "Failed to publish runtime handoff {}: {}",
            path.display(),
            error
        )
    })?;
    Ok(path)
}
