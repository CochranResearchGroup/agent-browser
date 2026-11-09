//! Public cooperative control adapter. Status/view also support ordinary managed
//! tasks without an app-host attachment. Reconciliation additionally proves exact
//! original process custody; other control changes require that attachment. Return
//! observes the same retained target through ordinary dispatch before resuming.
use super::{required_string_argument, JsonRpcError};
use crate::native::{
    browser_host_control as control, browser_session_store::BrowserSessionSqliteStore,
};
use serde_json::{json, Value};

pub(super) fn tool() -> Value {
    json!({"name":"browser_control","title":"View, take over or return hosted browser control","description":"Viewing does not pause. Explicit takeover persists across restart and gates supported automation routes. Return observes the same retained browser before resuming; pending effects are never replayed. Direct CDP/OS controllers must cooperate. Override requires an attributable actor/reason and current epoch using existing caller authority.","inputSchema":{"type":"object","additionalProperties":false,"properties":{"taskId":{"type":"string"},"operation":{"type":"string","enum":["status","view","takeover","return","override","reconcile"]},"expectedEpoch":{"type":"integer","minimum":0},"actor":{"type":"string"},"reason":{"type":"string"},"pauseReads":{"type":"boolean"},"requestId":{"type":"string"}},"required":["taskId","operation"]}})
}
fn issue(error: impl Into<String>) -> JsonRpcError {
    JsonRpcError::invalid_params(&error.into())
}

pub(super) fn call(arguments: &Value, transport: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let operation = required_string_argument(arguments, "operation")?;
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let store = BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| issue("browser_control_task_missing"))?;
    if !state.host_bindings.contains_key(task_id)
        && !matches!(operation, "status" | "view" | "reconcile")
    {
        return Err(issue("browser_control_hosted_task_required"));
    }
    let browser_id = task.browser_id.clone();
    if !matches!(operation, "status" | "view") {
        let expected = arguments["expectedEpoch"]
            .as_u64()
            .ok_or_else(|| issue("browser_control_expected_epoch_required"))?;
        let actor = required_string_argument(arguments, "actor")?;
        let reason = required_string_argument(arguments, "reason")?;
        let pause = match arguments.get("pauseReads") {
            None => false,
            Some(value) => value
                .as_bool()
                .ok_or_else(|| issue("browser_control_pause_reads_invalid"))?,
        };
        let retained_target = task
            .current_tab_id
            .as_ref()
            .and_then(|id| state.tabs.get(id))
            .map(|tab| tab.target_id.clone());
        let reconcile_process = if operation == "reconcile" {
            let request_id = required_string_argument(arguments, "requestId")?;
            control::check_reconcile(&browser_id, expected, request_id).map_err(issue)?;
            Some(current_process(&store, &state, task)?)
        } else {
            None
        };
        let atomic_return = operation == "return"
            && retained_target.is_some()
            && control::eligible_return_observation(&browser_id, expected, actor, reason)
                .map_err(issue)?;
        let epoch = if operation == "reconcile" || atomic_return {
            expected
        } else {
            control::change(&browser_id, operation, expected, actor, reason, pause)
                .map_err(issue)?
        };
        if matches!(operation, "return" | "override" | "reconcile") {
            // Awaiting-observation forbids physical recovery in the host. A lost target
            // remains blocked, preserving the original browser/task rather than reopening.
            let mut observation = json!({"id":uuid::Uuid::new_v4().to_string(),"action":"url","hostControlFreshObservation":true,"sessionName":task.name,"sessionId":task_id,"targetId":retained_target});
            if operation == "reconcile" {
                observation["retainedTaskOnly"] = json!(true);
                observation["retainedTaskContext"] =
                    crate::native::browser_task_operations::retained_context(task_id)
                        .map_err(issue)?;
            }
            if atomic_return {
                observation["hostControlReturnObservation"] = control::attest_return_observation(json!({"browserId":browser_id,"taskId":task_id,"profileId":task.profile_id,"targetId":retained_target,"expectedEpoch":expected,"actor":actor,"reason":reason})).map_err(issue)?;
            }
            let observed = super::workflow::execute(observation, transport)?;
            if operation == "reconcile"
                && (observed["urlObserved"] == false
                    || observed["url"].as_str().is_none_or(|url| url.is_empty())
                    || observed.get("targetId").is_some_and(|id| {
                        Some(id.as_str().unwrap_or_default()) != retained_target.as_deref()
                    }))
            {
                return Err(issue("browser_control_fresh_target_observation_required"));
            }
            let fresh = BrowserSessionSqliteStore::open_read_only(&path)
                .map_err(issue)?
                .load_session_state()
                .map_err(issue)?;
            let same_task = fresh.sessions.get(task_id).is_some_and(|current| {
                current.id == task.id
                    && current.created_at_ms == task.created_at_ms
                    && current.browser_id == task.browser_id
                    && current.profile_id == task.profile_id
                    && current.current_tab_id == task.current_tab_id
                    && (operation != "reconcile" || current.name == task.name)
            });
            // Observation updates activity and peer indexes. Compare identity,
            // including desktop generation, rather than mutable publication data.
            let same_browser = if operation == "reconcile" {
                state
                    .browsers
                    .get(&browser_id)
                    .zip(fresh.browsers.get(&browser_id))
                    .is_some_and(|(before, after)| {
                        before.id == after.id
                            && before.profile_id == after.profile_id
                            && before.pid == after.pid
                            && before.cdp_endpoint == after.cdp_endpoint
                            && before
                                .desktop
                                .as_ref()
                                .map(|d| (&d.desktop_id, d.generation))
                                == after
                                    .desktop
                                    .as_ref()
                                    .map(|d| (&d.desktop_id, d.generation))
                    })
            } else {
                fresh.browsers.get(&browser_id) == state.browsers.get(&browser_id)
            };
            let same_tab = operation != "reconcile"
                || task
                    .current_tab_id
                    .as_ref()
                    .and_then(|id| state.tabs.get(id).zip(fresh.tabs.get(id)))
                    .is_some_and(|(before, after)| {
                        before.id == after.id
                            && before.browser_id == after.browser_id
                            && before.session_id == after.session_id
                            && before.target_id == after.target_id
                            && before.created_at_ms == after.created_at_ms
                    });
            if !same_task || !same_browser || !same_tab {
                return Err(issue("browser_control_observation_target_changed"));
            }
            if operation == "reconcile" {
                let fresh_store =
                    BrowserSessionSqliteStore::open_read_only(&path).map_err(issue)?;
                let fresh_task = fresh
                    .sessions
                    .get(task_id)
                    .ok_or_else(|| issue("browser_control_task_missing"))?;
                if Some(current_process(&fresh_store, &fresh, fresh_task)?) != reconcile_process {
                    return Err(issue("browser_control_observation_process_changed"));
                }
                control::reconcile(
                    &browser_id,
                    epoch,
                    required_string_argument(arguments, "requestId")?,
                    actor,
                    reason,
                )
                .map_err(issue)?;
            }
            if atomic_return {
                control::commit_observed_return(&browser_id, expected, actor, reason)
                    .map_err(issue)?;
            } else if operation != "reconcile" {
                control::observed(&browser_id, epoch).map_err(issue)?;
            }
        }
    }
    Ok(super::tool_response_from_payload(
        "browser_control",
        transport,
        json!({}),
        json!({"taskId":task_id,"browserId":browser_id,"control":control::metadata(&browser_id).map_err(issue)?}),
        false,
    ))
}

/// Require the existing owner journal and a current exact process instance;
/// a PID, profile name or caller attribution alone cannot grant reconciliation.
fn current_process(
    store: &BrowserSessionSqliteStore,
    state: &agent_browser_service_model::BrowserSessionState,
    task: &agent_browser_service_model::ManagedBrowserSession,
) -> Result<agent_browser_service_model::RecordedProcessIdentity, JsonRpcError> {
    let browser = state
        .browsers
        .get(&task.browser_id)
        .filter(|browser| browser.profile_id == task.profile_id)
        .ok_or_else(|| issue("browser_control_original_process_required"))?;
    let tab = task
        .current_tab_id
        .as_ref()
        .and_then(|id| state.tabs.get(id))
        .filter(|tab| tab.session_id == task.id && tab.browser_id == browser.id)
        .ok_or_else(|| issue("browser_control_original_target_required"))?;
    if tab.target_id.is_empty() {
        return Err(issue("browser_control_original_target_required"));
    }
    let original = store
        .original_browser_identity(state, browser)
        .ok_or_else(|| issue("browser_control_original_process_required"))?;
    let assessment = crate::process_identity::assess_process_ownership(
        Some(&original),
        crate::process_identity::observe_process(browser.pid),
        crate::process_identity::LegacyProfileProof::Unproven,
    );
    if !assessment.authorizes_adoption() {
        return Err(issue("browser_control_original_process_changed"));
    }
    Ok(original)
}

pub(super) fn cli(args: &[String]) -> Result<Value, String> {
    if !(4..=8).contains(&args.len()) {
        return Err("Usage: agent-browser --json mcp host-control <task-id> status|view|takeover|return|override|reconcile [epoch actor reason request-id]".into());
    }
    let mut arguments = json!({"taskId":args[2],"operation":args[3]});
    if args.len() > 4 {
        arguments["expectedEpoch"] = json!(args[4]
            .parse::<u64>()
            .map_err(|_| "browser_control_epoch_invalid")?);
    }
    if args.len() > 5 {
        arguments["actor"] = json!(args[5]);
    }
    if args.len() > 6 {
        arguments["reason"] = json!(args[6]);
    }
    if args.len() > 7 {
        arguments["requestId"] = json!(args[7]);
    }
    let response = call(&arguments, "default").map_err(|error| {
        error
            .data
            .and_then(|data| data["message"].as_str().map(str::to_owned))
            .unwrap_or_else(|| error.message.to_owned())
    })?;
    serde_json::from_str(
        response["content"][0]["text"]
            .as_str()
            .ok_or("browser_control_response_invalid")?,
    )
    .map_err(|_| "browser_control_response_invalid".into())
}

#[cfg(all(test, target_os = "linux"))]
#[path = "host_control_tests.rs"]
mod tests;
