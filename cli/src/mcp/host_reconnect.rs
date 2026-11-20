//! Public explicit hosted continuity, with metadata-only status and no implicit replay.
use super::{required_string_argument, JsonRpcError};
use crate::native::{
    browser_host_context, browser_host_reconnect, browser_session_store::BrowserSessionSqliteStore,
};
use serde_json::{json, Value};
fn issue(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError::invalid_params(&message.into())
}
pub(super) fn tool() -> Value {
    let open = super::workflow::tools().remove(0);
    json!({"name":"browser_reconnect","title":"Reconnect an exact retained browser task","description":"Status is compact metadata only. Ordinary tasks support status/reconnect through the current or a fresh facade, observing the same original target without launch; intentionally closed tasks remain closed. Disconnect preserves browser/task/control. Reconnect freshly observes without launch; changed desktop requires explicit rebind. Configure report_only (default) or reopen policy. Explicit reopen preserves logical task/profile/browser, never replays URLs or uncertain website effects. Borrowed reopen requires fresh trusted app attachment and preserves human control; managed reopen requires agent control. Unknown reopen remains pending and is never retried.","inputSchema":{"type":"object","additionalProperties":false,"properties":{
 "taskId":{"type":"string"},"operation":{"type":"string","enum":["status","disconnect","reconnect","rebind","configure","reopen"]},
 "requestId":{"type":"string","minLength":1,"maxLength":128},"expectedRevision":{"type":"integer","minimum":0},
 "lossPolicy":{"type":"string","enum":["report_only","reopen"]},
 "desktop":{"type":"object","additionalProperties":false,"properties":{"desktopId":{"type":"string"},"generation":{"type":"integer","minimum":1}},"required":["desktopId","generation"]},
 "attachment":open["inputSchema"]["properties"]["attachment"]},"required":["taskId","operation"]}})
}
pub(super) fn call(arguments: &Value, transport: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "taskId")?;
    let operation = required_string_argument(arguments, "operation")?;
    let path = BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?;
    let before = BrowserSessionSqliteStore::open_read_only(&path)
        .map_err(issue)?
        .load_session_state()
        .map_err(issue)?;
    let Some(task) = before.sessions.get(id) else {
        if !matches!(operation, "status" | "reconnect") {
            return Err(issue("browser_reconnect_task_missing: inspect exact-task status; do not create or reopen a replacement implicitly"));
        }
        let terminal = before
            .session_history
            .iter()
            .rev()
            .find(|task| task.id == id);
        if operation == "reconnect" {
            let code = if terminal.is_some() {
                "browser_reconnect_task_ended"
            } else {
                "browser_reconnect_task_metadata_unavailable"
            };
            let mut error = issue(format!("{code}: inspect exact task history through browser_reconnect status; do not recreate or replay original work"));
            error.data.as_mut().unwrap()["taskId"] = json!(id);
            error.data.as_mut().unwrap()["terminalReason"] =
                json!(terminal.map(|task| task.reason));
            error.data.as_mut().unwrap()["effectStatus"] = json!("no_effect");
            error.data.as_mut().unwrap()["recovery"] = json!({"tool":"browser_reconnect","arguments":{"taskId":id,"operation":"status"},"replayAllowed":false});
            return Err(error);
        }
        return Ok(super::tool_response_from_payload(
            "browser_reconnect",
            transport,
            json!({}),
            json!({"taskId":id,"status":if terminal.is_some() {"task_ended"}else{"task_metadata_unavailable"},
                "terminalReason":terminal.map(|task| task.reason),
                "effectStatus":"no_effect","replayAllowed":false,
                "nextAction":"inspect_task_history_and_original_operation; explicitly closed tasks require a new separately intended task, never automatic reopen"}),
            false,
        ));
    };
    if !before.host_bindings.contains_key(id) {
        if !matches!(operation, "status" | "reconnect") {
            return Err(issue("browser_reconnect_ordinary_task_operation_unsupported: use status or reconnect for this exact task; no replacement browser is created"));
        }
        let briefing = super::workflow::inspect_briefing(id).map_err(issue)?;
        if operation == "status" {
            return Ok(super::tool_response_from_payload(
                "browser_reconnect",
                transport,
                json!({}),
                json!({"taskId":id,"status":"task_retained","connectionScope":"current_facade_only",
                    "browserId":task.browser_id,"profileId":task.profile_id,"briefing":briefing,
                    "recovery":{"tool":"browser_reconnect","arguments":{"taskId":id,"operation":"reconnect"},
                        "freshFacadeCommand":{"executable":"agent-browser","args":["--json","mcp","host-reconnect",id,"reconnect"]},
                        "precondition":"use an initialized client or the fresh-facade command; this does not reset any other selected client", "replayAllowed":false}}),
                false,
            ));
        }
        // Reuse exact retained-context observation. This may initialize the
        // transport facade, but never allocates a task/browser or replays input.
        return super::workflow::continue_task(&json!({"taskId":id}), transport);
    }
    let reconnect = if operation == "status" {
        browser_host_reconnect::metadata(&before, id)
    } else {
        let mut command = arguments.clone();
        command["action"] = json!("browser_session_reconnect");
        command["sessionId"] = json!(id);
        command["sessionName"] = json!(task.name);
        command["id"] = json!(uuid::Uuid::new_v4().to_string());
        super::workflow::execute(command, transport)?
    };
    let fresh = BrowserSessionSqliteStore::open_read_only(&path)
        .map_err(issue)?
        .load_session_state()
        .map_err(issue)?;
    Ok(super::tool_response_from_payload(
        "browser_reconnect",
        transport,
        json!({}),
        json!({"taskId":id,"reconnect":reconnect,"host":browser_host_context::snapshot(&fresh,id).map_err(issue)?}),
        false,
    ))
}
pub(super) fn cli(args: &[String]) -> Result<Value, String> {
    if !(4..=5).contains(&args.len()) {
        return Err("Usage: agent-browser --json mcp host-reconnect <task-id> status|disconnect|reconnect|rebind|configure|reopen [options-json]".into());
    }
    let mut arguments = if let Some(raw) = args.get(4) {
        serde_json::from_str::<Value>(raw).map_err(|_| "browser_reconnect_options_invalid")?
    } else {
        json!({})
    };
    if !arguments.is_object() {
        return Err("browser_reconnect_options_invalid".into());
    }
    arguments["taskId"] = json!(args[2]);
    arguments["operation"] = json!(args[3]);
    let result = call(&arguments, "default").map_err(|e| {
        e.data
            .and_then(|v| v["message"].as_str().map(str::to_owned))
            .unwrap_or(e.message.to_owned())
    })?;
    serde_json::from_str(
        result["content"][0]["text"]
            .as_str()
            .ok_or("browser_reconnect_response_invalid")?,
    )
    .map_err(|_| "browser_reconnect_response_invalid".into())
}
