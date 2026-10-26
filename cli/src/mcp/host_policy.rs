//! Public optional task rules and authorized local artifact handles.
use super::{required_string_argument, JsonRpcError};
use crate::native::{
    browser_host_policy as policy, browser_session_store::BrowserSessionSqliteStore,
};
use serde_json::{json, Value};

fn issue(error: impl Into<String>) -> JsonRpcError {
    JsonRpcError::invalid_params(&error.into())
}
pub(super) fn tool() -> Value {
    json!({"name":"browser_policy","title":"Configure optional app/task rules","description":"App defaults and explicit per-task replacement/clear. Unrestricted operations need no additional grant. Scoped operator override is attributable and bound to one task/action/request and current effective policy; it never bypasses human takeover or peer custody.","inputSchema":{"type":"object","additionalProperties":false,"properties":{
        "taskId":{"type":"string"},"operation":{"type":"string","enum":["status","set_app","set_task","clear_app","clear_task","override"]},"expectedRevision":{"type":"integer","minimum":0},"actor":{"type":"string"},"reason":{"type":"string"},
        "policy":{"type":"object","additionalProperties":false,"properties":{"version":{"const":1},"deniedActions":{"type":"array","items":{"type":"string"},"maxItems":128},"artifactDestinations":{"type":"object","additionalProperties":{"type":"string"}}},"required":["version"]},
        "overrideAction":{"type":"string"},"requestId":{"type":"string"}},"required":["taskId","operation"]}})
}
pub(super) fn artifact_tool() -> Value {
    json!({"name":"browser_artifact","title":"Capture or download to an app-authorized handle","description":"Write a screenshot or triggered download to one new file below the task's configured artifact handle. Returns local metadata only; does not publish/share the file. Existing files and symlinked destinations are refused.","inputSchema":{"type":"object","additionalProperties":false,"properties":{
        "taskId":{"type":"string"},"kind":{"type":"string","enum":["screenshot","download"]},"handle":{"type":"string"},"name":{"type":"string"},"selector":{"type":"string"},"fullPage":{"type":"boolean"},"requestId":{"type":"string"},"policyOverrideId":{"type":"string"}},"required":["taskId","kind","handle","name"]}})
}
pub(super) fn call(arguments: &Value, transport: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let name = super::workflow::task_session(task_id)?;
    let mut command = arguments.clone();
    command["id"] = json!(uuid::Uuid::new_v4().to_string());
    command["action"] = json!("browser_session_policy");
    command["sessionName"] = json!(name);
    let result = super::workflow::execute(command, transport)?;
    Ok(super::tool_response_from_payload(
        "browser_policy",
        transport,
        json!({}),
        result,
        false,
    ))
}
pub(super) fn artifact(arguments: &Value, transport: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let kind = required_string_argument(arguments, "kind")?;
    if !matches!(kind, "screenshot" | "download") {
        return Err(issue("browser_artifact_kind_invalid"));
    }
    let store = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path().map_err(issue)?,
    )
    .map_err(issue)?;
    let state = store.load_session_state().map_err(issue)?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| issue("browser_artifact_task_missing"))?;
    if state
        .task_artifact_references
        .get(task_id)
        .is_some_and(|rows| rows.len() >= 256)
    {
        return Err(issue("browser_artifact_reference_capacity_reached"));
    }
    let id = arguments["requestId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut command = json!({"id":id,"action":kind,"sessionId":task_id,"sessionName":task.name,
        "artifactHandle":required_string_argument(arguments,"handle")?,"artifactName":required_string_argument(arguments,"name")?});
    if kind == "download" {
        command["selector"] = json!(required_string_argument(arguments, "selector")?);
    } else if let Some(selector) = arguments.get("selector") {
        command["selector"] = selector.clone();
    }
    if let Some(full_page) = arguments.get("fullPage") {
        command["fullPage"] = full_page.clone();
    }
    if let Some(override_id) = arguments.get("policyOverrideId") {
        command["policyOverrideId"] = override_id.clone();
    }
    policy::prepare_command(&state, &task.name, &mut command).map_err(issue)?;
    let path = command["path"].clone();
    let result = super::workflow::execute(command, transport)?;
    // Publish only a successful operation's opaque reference into the existing
    // owner aggregate. Never persist private paths or page/download contents.
    let current = store.load_session_state().map_err(issue)?;
    let mut next = current.clone();
    let reference = serde_json::from_value(json!({"artifactId":id,"kind":kind,
        "handle":arguments["handle"],"observedAtMs":std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).map_err(|_| issue("browser_artifact_clock_invalid"))?.as_millis() as u64}))
        .map_err(|_| issue("browser_artifact_reference_invalid"))?;
    next.retain_completed_task_artifact(&state, task_id, reference)
        .map_err(|error| issue(format!("{error}:{id}:do not repeat capture")))?;
    // Write through a writable owner connection; the earlier inspection remains
    // read-only. Optimistic fencing prevents loss of concurrent task updates.
    let mut writer = BrowserSessionSqliteStore::default_sqlite().map_err(issue)?;
    writer
        .compare_and_save_session_state(&current, &next)
        .map_err(|_| {
            issue(format!(
                "browser_artifact_completed_reference_pending:{id}:do not repeat capture"
            ))
        })?;
    Ok(super::tool_response_from_payload(
        "browser_artifact",
        transport,
        json!({}),
        json!({"taskId":task_id,"kind":kind,"handle":arguments["handle"],"name":arguments["name"],"artifactId":id,"path":path,"publication":"not_requested","result":result}),
        false,
    ))
}
pub(super) fn cli(args: &[String]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("Usage: agent-browser --json mcp host-policy '<JSON request>'".into());
    }
    let arguments: Value =
        serde_json::from_str(&args[2]).map_err(|_| "browser_policy_request_invalid")?;
    let response = call(&arguments, "default").map_err(|error| {
        error
            .data
            .and_then(|data| data["message"].as_str().map(str::to_owned))
            .unwrap_or_else(|| error.message.into())
    })?;
    serde_json::from_str(
        response["content"][0]["text"]
            .as_str()
            .ok_or("browser_policy_response_invalid")?,
    )
    .map_err(|_| "browser_policy_response_invalid".into())
}
