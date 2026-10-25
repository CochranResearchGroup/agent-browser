//! Owner profile operations reuse the managed owner transport without acquiring a task.
use super::{optional_object_argument, required_string_argument, JsonRpcError};
use serde_json::{json, Value};

pub(super) fn call(arguments: &Value, transport: &str) -> Result<Value, JsonRpcError> {
    let mut command = Value::Object(
        optional_object_argument(arguments, "params")?
            .cloned()
            .unwrap_or_default(),
    );
    if command["choice"] != "inspect" {
        required_string_argument(&command, "operationId")?;
    }
    if let Some(task_id) = arguments["taskId"].as_str() {
        let session = super::workflow::address_task_command(&mut command, task_id)
            .map_err(|error| JsonRpcError::invalid_params(&error))?;
        let source = command["profileId"]
            .as_str()
            .ok_or_else(|| JsonRpcError::invalid_params("profile_duplicate_task_profile_missing"))?
            .to_string();
        if command
            .get("sourceProfileId")
            .is_some_and(|value| value.as_str() != Some(source.as_str()))
        {
            return Err(JsonRpcError::invalid_params(
                "profile_duplicate_task_profile_conflict",
            ));
        }
        command["sourceProfileId"] = json!(source);
        command["sessionName"] = json!(session);
    }
    required_string_argument(&command, "sourceProfileId")?;
    command["action"] = json!("browser_session_profile_duplicate");
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
        return Err("Usage: agent-browser --json mcp profile-duplicate '<JSON request>'".into());
    }
    let request: Value =
        serde_json::from_str(&args[2]).map_err(|_| "profile_duplicate_request_invalid")?;
    let response = call(&json!({"params":request}), "default").map_err(|error| error.message)?;
    serde_json::from_str(
        response["content"][0]["text"]
            .as_str()
            .ok_or("profile_duplicate_response_invalid")?,
    )
    .map_err(|_| "profile_duplicate_response_invalid".into())
}
