//! Small public workflow schemas, separate from explicitly discovered operations.
use serde_json::{json, Value};

pub(super) fn tools() -> Vec<Value> {
    let mut tools = super::workflow::tools();
    tools.push(super::host_control::tool());
    tools.push(super::host_reconnect::tool());
    tools.push(super::host_policy::tool());
    tools.push(super::host_policy::artifact_tool());
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
                    "params":{"type":"object"},"policyOverrideId":{"type":"string"},"requestId":{"type":"string"}},"required":["action"],"anyOf":[{"required":["taskId"]},{"properties":{"action":{"enum":["profile_duplicate","browser_build"]}},"required":["params"]}]}}),
        json!({"name":"browser_discover","title":"Discover advanced operations",
            "description":"Read supported browser, diagnostic or administrative operation metadata on demand. Select an operation for its argument schema and CLI/API guidance.",
            "inputSchema":{"type":"object","additionalProperties":false,
                "properties":{"category":{"enum":["browser","diagnostics","administration","playbooks"]},
                    "operation":{"type":"string"}},"required":["category"]}}),
    ]);
    for tool in &mut tools {
        if matches!(
            tool["name"].as_str(),
            Some(
                "browser_navigate"
                    | "browser_click"
                    | "browser_fill"
                    | "browser_snapshot"
                    | "browser_screenshot"
                    | "browser_close"
                    | "browser_tab"
            )
        ) {
            tool["inputSchema"]["properties"]["policyOverrideId"] = json!({"type":"string"});
            tool["inputSchema"]["properties"]["requestId"] = json!({"type":"string"});
        }
    }
    tools
}

/// Metadata is loaded only for the requested category or operation. Diagnostic
/// and administrative effects use the independently supported CLI/API surface.
pub(super) fn discover(arguments: &Value, session: &str) -> Result<Value, super::JsonRpcError> {
    let category = super::required_string_argument(arguments, "category")?;
    if category == "playbooks" {
        return super::playbooks::discover(arguments, session);
    }
    let operation = super::optional_string_argument(arguments, "operation")?;
    let names = operation_names(category)?;
    let payload = if let Some(operation) = operation {
        if !names.iter().any(|name| name == operation) {
            return Err(super::JsonRpcError::invalid_params(
                "Operation is not available in this category",
            ));
        }
        let schema = operation_schema(category, operation)?;
        json!({"category":category,"operation":operation,"inputSchema":schema,
            "execution":if category == "browser" {json!({"tool":"browser_command","action":operation,
                "arguments":if operation=="browser_build" {"Supply params.profileId or taskId. inspect needs no operationId; open_selected continues a confirmed selection and requires serviceName, agentName and taskName."} else if operation == "profile_duplicate" {"Supply params.sourceProfileId, or taskId to bind the source profile. Supply params.operationId except for choice inspect, which lists matching operations."} else {"Place these page arguments in params and supply taskId."}})}
                else {service_execution(operation)}})
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

const DIAGNOSTIC_OPERATIONS: &[&str] = &[
    "service_access_plan",
    "service_status",
    "service_resources",
    "service_incidents",
    "service_trace",
    "service_browser_capability_preflight",
    "service_remote_view_route_preflight",
];

fn operation_names(category: &str) -> Result<Vec<String>, super::JsonRpcError> {
    let mut names: Vec<String> = match category {
        "browser" => super::BROWSER_COMMAND_ALLOWED_ACTIONS
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        "diagnostics" => DIAGNOSTIC_OPERATIONS
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        "administration" => super::service_mcp_tools()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .filter(|name| {
                name.starts_with("service_")
                    && *name != "service_request"
                    && !DIAGNOSTIC_OPERATIONS.contains(name)
            })
            .map(str::to_string)
            .collect(),
        _ => {
            return Err(super::JsonRpcError::invalid_params(
                "Unknown discovery category",
            ))
        }
    };
    names.sort();
    names.dedup();
    Ok(names)
}

pub(super) fn browser_argument_schema(action: &str) -> Result<Value, super::JsonRpcError> {
    operation_schema("browser", action)
}

pub(super) fn browser_adapter_name(action: &str) -> String {
    operation_tool_name("browser", action)
}

fn operation_tool_name(category: &str, operation: &str) -> String {
    match operation {
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
        "count" => "browser_count".to_string(),
        "boundingbox" => "browser_get_box".to_string(),
        "scrollintoview" => "browser_scroll_into_view".to_string(),
        name if category == "browser" => format!("browser_{name}"),
        name => name.to_string(),
    }
}

fn operation_schema(category: &str, operation: &str) -> Result<Value, super::JsonRpcError> {
    let tool_name = operation_tool_name(category, operation);
    let mut schema = super::service_mcp_tools()
        .into_iter()
        .find(|tool| tool["name"] == tool_name)
        .map(|tool| tool["inputSchema"].clone())
        .or_else(|| match operation {
            "task_briefing" | "mode" | "back" | "forward" | "reload" | "service_resources" => {
                Some(json!({"type":"object",
                "additionalProperties":false,"properties":{},"required":[]}))
            }
            "browser_build"=>Some(json!({"type":"object","additionalProperties":false,"properties":{
                "profileId":{"type":"string","minLength":1},"operationId":{"type":"string","minLength":1},"choice":{"enum":["inspect","select_for_next_launch","open_selected"]},"buildId":{"type":"string"},"executablePath":{"type":"string"},"expectedSha256":{"type":"string"},"mode":{"enum":["cdp","cdp_free"]},"ownerConfirmed":{"type":"boolean"},"serviceName":{"type":"string"},"agentName":{"type":"string"},"taskName":{"type":"string"}
            }})),
            "mode_convert" => Some(json!({"type":"object","additionalProperties":false,"properties":{
                "operationId":{"type":"string","minLength":1},"mode":{"enum":["cdp","cdp_free"]},
                "choice":{"enum":["inspect","prepare","relaunch","pause_and_relaunch","reconcile","retry_graceful_close","force_close","focus","restore_previous","select_build","cancel"]},"ownerConfirmed":{"type":"boolean"},"buildId":{"type":"string"},"executablePath":{"type":"string"},"expectedSha256":{"type":"string"}
            },"allOf":[{"if":{"properties":{"choice":{"const":"inspect"}},"required":["choice"]},"then":{},"else":{"required":["operationId","mode"]}}]})),
            "profile_duplicate" => Some(json!({"type":"object","additionalProperties":false,"properties":{
                "operationId":{"type":"string","minLength":1},"sourceProfileId":{"type":"string","minLength":1},"destinationName":{"type":"string","minLength":1},
                "choice":{"enum":["inspect","prepare","reconcile","retry_launch","finish_without_opening","open_source","focus_source","close_and_duplicate","cancel"]},"ownerConfirmed":{"type":"boolean"},"launch":{"type":"boolean"},"buildId":{"type":"string"},"executablePath":{"type":"string"},"expectedSha256":{"type":"string"},"mode":{"enum":["cdp","cdp_free"]}
            },"allOf":[{"if":{"properties":{"choice":{"const":"inspect"}},"required":["choice"]},"then":{},"else":{"required":["operationId"]}}]})),
            "native_capture" => Some(json!({"type":"object","additionalProperties":false,"properties":{"region":{"type":"object","properties":{"x":{"type":"integer"},"y":{"type":"integer"},"width":{"type":"integer","minimum":1},"height":{"type":"integer","minimum":1}},"required":["x","y","width","height"],"additionalProperties":false}},"required":[]})),
            "native_input" => Some(json!({"type":"object","additionalProperties":false,"properties":{"actions":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"object"}},"controlEpoch":{"type":"integer","minimum":0}},"required":["actions"]})),
            "evaluate" => Some(json!({"type":"object","additionalProperties":false,
                "properties":{"script":{"type":"string"}},"required":["script"]})),
            _ => None,
        })
        .ok_or_else(|| {
            super::JsonRpcError::invalid_params(
                "Argument metadata is unavailable for this operation; use agent-browser --help",
            )
        })?;
    // The build adapter declares its own narrow profile/open context. Applying
    // the generic page-only projection would remove fields that it requires.
    if category == "browser" && operation != "browser_build" {
        if let Some(properties) = schema["properties"].as_object_mut() {
            properties.retain(|name, _| schema_property_is_page_argument(name));
        }
        if let Some(required) = schema["required"].as_array_mut() {
            required
                .retain(|name| schema_property_is_page_argument(name.as_str().unwrap_or_default()));
        }
    }
    Ok(schema)
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
            | "targetServiceId"
            | "targetServiceIds"
            | "siteId"
            | "siteIds"
            | "loginId"
            | "loginIds"
            | "accountId"
            | "accountIds"
            | "readinessProfileId"
            | "sitePolicyId"
            | "browserBuild"
            | "browserHost"
            | "viewStreamProvider"
            | "controlInputProvider"
            | "displayIsolation"
    )
}

fn service_execution(operation: &str) -> Value {
    let (cli, method, route) = match operation {
        "service_status" => ("agent-browser service status", "GET", "/api/service/status"),
        "service_resources" => (
            "agent-browser service resources",
            "GET",
            "/api/service/resources",
        ),
        "service_incidents" => (
            "agent-browser service incidents",
            "GET",
            "/api/service/incidents",
        ),
        "service_trace" => ("agent-browser service trace", "GET", "/api/service/trace"),
        "service_job_cancel" => (
            "agent-browser service cancel <job-id>",
            "POST",
            "/api/service/jobs/<job-id>/cancel",
        ),
        "service_browser_retry" => (
            "agent-browser service retry <browser-id>",
            "POST",
            "/api/service/browsers/<browser-id>/retry",
        ),
        "service_remedies_apply" => (
            "agent-browser service remedies apply",
            "POST",
            "/api/service/remedies/apply",
        ),
        _ => {
            let metadata = crate::native::service_contracts::service_contracts_metadata();
            let contract = metadata["contracts"]
                .as_object()
                .unwrap()
                .values()
                .find(|contract| {
                    contract["mcp"].as_object().is_some_and(|mcp| {
                        mcp.values().any(|value| {
                            value.as_str() == Some(operation)
                                || value.as_array().is_some_and(|names| {
                                    names.iter().any(|name| name.as_str() == Some(operation))
                                })
                        })
                    })
                });
            return json!({"cli":"agent-browser service --help",
                "api":contract.map(|contract| contract["http"].clone()),
                "client":contract.map(|contract| contract["client"].clone()),
                "contractsResource":"agent-browser://contracts", "metadataOnly":true,
                "guidance":"Use the supported CLI/API for this operation. This metadata is not a callable MCP tool."});
        }
    };
    json!({"cli":cli,"api":{"method":method,"route":route},
        "guidance":"Supply the selected arguments through the documented CLI/API. Discovery performs no effects."})
}
