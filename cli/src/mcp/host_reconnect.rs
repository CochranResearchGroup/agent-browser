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
    json!({"name":"browser_reconnect","title":"Reconnect or explicitly rebind a hosted browser task","description":"Status is compact metadata only. Disconnect preserves browser/task/control. Reconnect freshly observes without launch; changed desktop requires explicit rebind. Configure report_only (default) or reopen policy. Explicit reopen preserves logical task/profile/browser, never replays URLs or uncertain website effects. Borrowed reopen requires fresh trusted app attachment and preserves human control; managed reopen requires agent control. Unknown reopen remains pending and is never retried.","inputSchema":{"type":"object","additionalProperties":false,"properties":{
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
    let task = before
        .sessions
        .get(id)
        .ok_or_else(|| issue("browser_reconnect_task_missing"))?;
    if !before.host_bindings.contains_key(id) {
        return Err(issue("browser_reconnect_hosted_task_required"));
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
