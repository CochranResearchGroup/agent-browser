//! Profile-scoped build inspection/configuration uses the existing owner transport.
use super::{optional_object_argument, required_string_argument, JsonRpcError};
use serde_json::{json, Value};

pub(super) fn call(arguments: &Value, transport: &str) -> Result<Value, JsonRpcError> {
    let mut command = Value::Object(
        optional_object_argument(arguments, "params")?
            .cloned()
            .unwrap_or_default(),
    );
    let choice = command["choice"].as_str().unwrap_or("inspect");
    if choice != "inspect" {
        required_string_argument(&command, "operationId")?;
    }
    if choice == "open_selected" {
        for field in ["serviceName", "agentName", "taskName"] {
            required_string_argument(&command, field)?;
        }
        command["originKind"] = json!("agent");
    }
    let provided_profile = command.get("profileId").cloned();
    if let Some(task_id) = arguments["taskId"].as_str() {
        let session = super::workflow::address_task_command(&mut command, task_id)
            .map_err(|error| JsonRpcError::invalid_params(&error))?;
        if provided_profile
            .as_ref()
            .is_some_and(|profile| profile != &command["profileId"])
        {
            return Err(JsonRpcError::invalid_params(
                "browser_build_task_profile_conflict",
            ));
        }
        command["sessionName"] = json!(session);
    }
    required_string_argument(&command, "profileId")?;
    command["action"] = json!("browser_session_build_select");
    command["id"] = json!(uuid::Uuid::new_v4().to_string());
    let data = super::workflow::execute(command, transport)?;
    Ok(super::tool_response_from_payload(
        "browser_command",
        transport,
        json!({}),
        data,
        false,
    ))
}

pub(super) fn cli(args: &[String]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("Usage: agent-browser --json mcp browser-build '<JSON request>'".into());
    }
    let mut request: Value =
        serde_json::from_str(&args[2]).map_err(|_| "browser_build_request_invalid")?;
    let object = request
        .as_object_mut()
        .ok_or("browser_build_request_invalid")?;
    let task = object.remove("taskId");
    let mut arguments = json!({"params":request});
    if let Some(task) = task {
        arguments["taskId"] = task;
    }
    let response = call(&arguments, "default").map_err(|error| error.message)?;
    serde_json::from_str(
        response["content"][0]["text"]
            .as_str()
            .ok_or("browser_build_response_invalid")?,
    )
    .map_err(|_| "browser_build_response_invalid".into())
}
