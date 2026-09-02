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
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::interaction;
use crate::native::remote_view::{
    display_allocation_id_for_route_pool_entry, normalize_remote_view_open_intent,
    plan_remote_view_acquisition, readiness_state, route_binding_readiness,
    route_bound_display_content, route_display_content, visible_browser_window_proof,
    RemoteViewAcquisitionPlan, RemoteViewRouteBinding,
};
use crate::native::service_config::{
    delete_persisted_monitor, delete_persisted_profile, delete_persisted_provider,
    delete_persisted_session, delete_persisted_site_policy, reset_persisted_monitor_failures,
    update_persisted_monitor_state, update_persisted_profile_freshness,
    update_persisted_profile_seeding_handoff, upsert_persisted_browser_capability_registry_record,
    upsert_persisted_monitor, upsert_persisted_profile, upsert_persisted_provider,
    upsert_persisted_session, upsert_persisted_site_policy,
};
use crate::native::state;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) async fn handle_service_probe(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "probe requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_daemon(handle, cmd, state)?;
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
#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id, validate_service_tab_handle_for_daemon,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::browser::{
        should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo,
        ProcessExitObservation, WaitUntil,
    };
    use crate::native::interaction;
    use crate::native::remote_view::{
        display_allocation_id_for_route_pool_entry, normalize_remote_view_open_intent,
        plan_remote_view_acquisition, readiness_state, route_binding_readiness,
        route_bound_display_content, route_display_content, visible_browser_window_proof,
        RemoteViewAcquisitionPlan, RemoteViewRouteBinding,
    };
    use crate::native::service_config::{
        delete_persisted_monitor, delete_persisted_profile, delete_persisted_provider,
        delete_persisted_session, delete_persisted_site_policy, reset_persisted_monitor_failures,
        update_persisted_monitor_state, update_persisted_profile_freshness,
        update_persisted_profile_seeding_handoff,
        upsert_persisted_browser_capability_registry_record, upsert_persisted_monitor,
        upsert_persisted_profile, upsert_persisted_provider, upsert_persisted_session,
        upsert_persisted_site_policy,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use serde_json::{json, Map, Value};
    use sha2::{Digest, Sha256};
    use std::time::{Duration, Instant};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};
    pub(crate) async fn handle_bounded_service_evaluate(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let handle = cmd
            .get("serviceTabHandle")
            .and_then(Value::as_object)
            .ok_or_else(|| "evaluate requires serviceTabHandle".to_string())?;
        validate_service_tab_handle_for_daemon(handle, cmd, state)?;
        if cmd.get("returnByValue").and_then(Value::as_bool) == Some(false) {
            return Err(
                "evaluate requires returnByValue=true so results can be capped".to_string(),
            );
        }
        let script = cmd
            .get("script")
            .or_else(|| cmd.get("expression"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "evaluate requires script or expression".to_string())?;
        let timeout_ms = cmd
            .get("timeoutMs")
            .and_then(Value::as_u64)
            .ok_or_else(|| "evaluate requires positive timeoutMs".to_string())?;
        let max_return_bytes = cmd
            .get("maxReturnBytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| "evaluate requires positive maxReturnBytes".to_string())?;
        let mgr = state.browser.as_mut().ok_or_else(|| {
            "Cannot evaluate: target browser session is not running; request a service tab first"
                .to_string()
        })?;
        let target_id = handle
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or_else(|| "evaluate requires serviceTabHandle.targetId".to_string())?;
        if mgr.active_target_id().ok() != Some(target_id) {
            let _ = mgr.tab_switch_target_id(target_id).await?;
        }
        let started_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        let evaluate_outcome = tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout_ms),
            mgr.evaluate_with_timeout(script, timeout_ms),
        )
        .await;
        let url = mgr.active_page_url().unwrap_or_default().to_string();
        let title = mgr.active_page_title().unwrap_or_default().to_string();
        let result = match evaluate_outcome {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => {
                return Ok(json!(
                    { "ok" : false, "action" : "evaluate", "errorKind" : "exception",
                    "error" : error, "timeoutMs" : timeout_ms, "maxReturnBytes" :
                    max_return_bytes, "url" : url, "title" : title, "targetId" :
                    target_id, "tabId" : handle.get("tabId").cloned()
                    .unwrap_or(Value::Null), "profileId" : handle.get("profileId")
                    .cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
                    .get("serviceTabHandle").cloned().unwrap_or(Value::Null),
                    "evaluatedAt" : started_at, }
                ));
            }
            Err(_) => {
                return Ok(json!(
                    { "ok" : false, "action" : "evaluate", "errorKind" : "timeout",
                    "error" : format!("evaluate timed out after {timeout_ms}ms"),
                    "timeoutMs" : timeout_ms, "maxReturnBytes" : max_return_bytes,
                    "url" : url, "title" : title, "targetId" : target_id, "tabId" :
                    handle.get("tabId").cloned().unwrap_or(Value::Null), "profileId"
                    : handle.get("profileId").cloned().unwrap_or(Value::Null),
                    "serviceTabHandle" : cmd.get("serviceTabHandle").cloned()
                    .unwrap_or(Value::Null), "evaluatedAt" : started_at, }
                ));
            }
        };
        let serialized = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
        let serialized_len = serialized.len() as u64;
        let truncated = serialized_len > max_return_bytes;
        let returned = if truncated {
            Value::String(truncate_utf8(&serialized, max_return_bytes as usize))
        } else {
            result
        };
        Ok(json!(
            { "ok" : true, "action" : "evaluate", "result" : returned,
            "resultTruncated" : truncated, "resultBytes" : serialized_len,
            "maxReturnBytes" : max_return_bytes, "timeoutMs" : timeout_ms,
            "returnByValue" : true, "url" : url, "title" : title, "targetId" :
            target_id, "tabId" : handle.get("tabId").cloned().unwrap_or(Value::Null),
            "profileId" : handle.get("profileId").cloned().unwrap_or(Value::Null),
            "serviceTabHandle" : cmd.get("serviceTabHandle").cloned()
            .unwrap_or(Value::Null), "evaluatedAt" : started_at, }
        ))
    }
}
pub(crate) use action_commands::*;
