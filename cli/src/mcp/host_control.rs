//! Public cooperative control adapter. Status/view are metadata only; return
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
    if !state.host_bindings.contains_key(task_id) {
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
        let epoch = if operation == "reconcile" {
            expected
        } else {
            control::change(&browser_id, operation, expected, actor, reason, pause)
                .map_err(issue)?
        };
        if matches!(operation, "return" | "override" | "reconcile") {
            // Awaiting-observation forbids physical recovery in the host. A lost target
            // remains blocked, preserving the original browser/task rather than reopening.
            super::workflow::execute(
                json!({"id":uuid::Uuid::new_v4().to_string(),"action":"url","sessionName":task.name,"sessionId":task_id,"targetId":task.current_tab_id.as_ref().and_then(|id|state.tabs.get(id)).map(|tab|&tab.target_id)}),
                transport,
            )?;
            let fresh = BrowserSessionSqliteStore::open_read_only(&path)
                .map_err(issue)?
                .load_session_state()
                .map_err(issue)?;
            if !fresh.sessions.get(task_id).is_some_and(|current| {
                current.browser_id == task.browser_id
                    && current.profile_id == task.profile_id
                    && current.current_tab_id == task.current_tab_id
            }) || fresh.browsers.get(&browser_id) != state.browsers.get(&browser_id)
            {
                return Err(issue("browser_control_observation_target_changed"));
            }
            if operation == "reconcile" {
                control::reconcile(
                    &browser_id,
                    epoch,
                    required_string_argument(arguments, "requestId")?,
                    actor,
                    reason,
                )
                .map_err(issue)?;
            }
            control::observed(&browser_id, epoch).map_err(issue)?;
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
    call(&arguments, "default").map_err(|error| {
        error
            .data
            .and_then(|data| data["message"].as_str().map(str::to_owned))
            .unwrap_or_else(|| error.message.to_owned())
    })
}
