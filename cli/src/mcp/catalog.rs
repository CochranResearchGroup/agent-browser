//! Small public workflow schemas, separate from explicitly discovered operations.
use serde_json::{json, Value};

pub(super) fn tools() -> Vec<Value> {
    let mut tools = super::workflow::tools();
    for mut tool in super::service_mcp_tools() {
        let description = match tool["name"].as_str() {
            Some("browser_navigate") => "Navigate this task's current tab.",
            Some("browser_snapshot") => "Read this task's page and element references.",
            Some("browser_click") => "Click an element in this task's current tab.",
            Some("browser_fill") => "Fill an input in this task's current tab.",
            Some("browser_screenshot") => "Capture this task's current page.",
            _ => continue,
        };
        tool["description"] = json!(description);
        let properties = tool["inputSchema"]["properties"].as_object_mut().unwrap();
        properties.retain(|name, _| {
            matches!(
                name.as_str(),
                "taskId"
                    | "url"
                    | "selector"
                    | "value"
                    | "path"
                    | "fullPage"
                    | "interactive"
                    | "compact"
                    | "depth"
                    | "maxDepth"
                    | "timeoutMs"
                    | "button"
                    | "clickCount"
                    | "force"
                    | "quality"
                    | "format"
            )
        });
        let allowed: Vec<String> = properties.keys().cloned().collect();
        let required = tool["inputSchema"]["required"].as_array_mut().unwrap();
        required.retain(|name| {
            name.as_str()
                .is_some_and(|name| allowed.iter().any(|key| key == name))
        });
        if !required.iter().any(|name| name == "taskId") {
            required.push(json!("taskId"));
        }
        tools.push(tool);
    }
    tools.extend([
        json!({"name":"browser_tab","title":"Manage task tabs",
            "description":"List, open, select or close only this task's tabs. Reuse tabKey when opening to avoid duplicate tabs.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"},
                    "operation":{"enum":["list","new","select","close"]},
                    "tabId":{"type":"string"},"tabKey":{"type":"string"},"url":{"type":"string"}},
                "required":["taskId","operation"]}}),
        json!({"name":"browser_command","title":"Run an advanced browser command",
            "description":"Run a supported advanced operation in this task. Use browser_discover for operation names and arguments. Retired authority operations are unavailable.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"taskId":{"type":"string"},"action":{"type":"string"},
                    "params":{"type":"object"}},"required":["taskId","action"]}}),
        json!({"name":"browser_discover","title":"Discover advanced operations",
            "description":"Read supported browser, diagnostic or administrative operation metadata on demand. Select an operation for its argument schema and CLI/API guidance.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"category":{"enum":["browser","diagnostics","administration"]},
                    "operation":{"type":"string"}},"required":["category"]}}),
    ]);
    tools
}

/// Metadata is loaded only for the requested category or operation. Diagnostic
/// and administrative effects use the independently supported CLI/API surface.
pub(super) fn discover(arguments: &Value, session: &str) -> Result<Value, super::JsonRpcError> {
    let category = super::required_string_argument(arguments, "category")?;
    let operation = super::optional_string_argument(arguments, "operation")?;
    let names: Vec<&str> = match category {
        "browser" => super::BROWSER_COMMAND_ALLOWED_ACTIONS.to_vec(),
        "diagnostics" => vec![
            "service_status",
            "service_resources",
            "service_incidents",
            "service_trace",
        ],
        "administration" => vec![
            "service_job_cancel",
            "service_browser_retry",
            "service_remedies_apply",
        ],
        _ => {
            return Err(super::JsonRpcError::invalid_params(
                "Unknown discovery category",
            ))
        }
    };
    let payload = if let Some(operation) = operation {
        if !names.contains(&operation) {
            return Err(super::JsonRpcError::invalid_params(
                "Operation is not available in this category",
            ));
        }
        let tool_name = match operation {
            "gettext" => "browser_get_text".to_string(),
            "inputvalue" => "browser_get_value".to_string(),
            "url" => "browser_get_url".to_string(),
            "title" => "browser_get_title".to_string(),
            "emulatemedia" => "browser_media".to_string(),
            "setcontent" => "browser_set_content".to_string(),
            "waitfordownload" => "browser_wait_for_download".to_string(),
            "responsebody" => "browser_response_body".to_string(),
            "isvisible" => "browser_is_visible".to_string(),
            "isenabled" => "browser_is_enabled".to_string(),
            "ischecked" => "browser_is_checked".to_string(),
            "getattribute" => "browser_get_attribute".to_string(),
            "innerhtml" => "browser_get_html".to_string(),
            "styles" => "browser_get_styles".to_string(),
            "count" => "browser_get_count".to_string(),
            "boundingbox" => "browser_get_bounding_box".to_string(),
            "scrollintoview" => "browser_scroll_into_view".to_string(),
            name if category == "browser" => format!("browser_{name}"),
            name => name.to_string(),
        };
        let mut schema = super::service_mcp_tools()
            .into_iter()
            .find(|tool| tool["name"] == tool_name)
            .map(|tool| tool["inputSchema"].clone())
            .ok_or_else(|| {
                super::JsonRpcError::invalid_params(
                    "Argument metadata is unavailable for this operation; use agent-browser --help",
                )
            })?;
        if category == "browser" {
            if let Some(properties) = schema["properties"].as_object_mut() {
                properties.retain(|name, _| {
                    !matches!(
                        name.as_str(),
                        "serviceName"
                            | "agentName"
                            | "taskName"
                            | "jobTimeoutMs"
                            | "taskId"
                            | "profileCapability"
                            | "serviceTabHandle"
                            | "profileId"
                            | "runtimeProfile"
                            | "profile"
                            | "browserId"
                            | "sessionId"
                            | "sessionName"
                            | "routeId"
                            | "profileLeaseId"
                            | "profileLeaseGeneration"
                            | "tabHandle"
                    )
                });
            }
            if let Some(required) = schema["required"].as_array_mut() {
                required.retain(|name| {
                    schema_property_is_page_argument(name.as_str().unwrap_or_default())
                });
            }
        }
        json!({"category":category,"operation":operation,"inputSchema":schema,
            "execution":if category == "browser" {json!({"tool":"browser_command","action":operation,
                "arguments":"Place these page arguments in params and supply taskId."})}
                else {json!({"cli":"agent-browser service --help","api":"/api/service/request",
                    "guidance":"Use the documented CLI/API operation; diagnostic discovery does not execute effects."})}})
    } else {
        json!({"category":category,"operations":names,"next":"Request one operation for its argument schema."})
    };
    Ok(super::tool_response_from_payload(
        "browser_discover",
        session,
        json!({}),
        payload,
        false,
    ))
}

fn schema_property_is_page_argument(name: &str) -> bool {
    !matches!(
        name,
        "serviceName"
            | "agentName"
            | "taskName"
            | "jobTimeoutMs"
            | "taskId"
            | "profileCapability"
            | "serviceTabHandle"
            | "profileId"
            | "runtimeProfile"
            | "profile"
            | "browserId"
            | "sessionId"
            | "sessionName"
            | "routeId"
            | "profileLeaseId"
            | "profileLeaseGeneration"
            | "tabHandle"
    )
}
