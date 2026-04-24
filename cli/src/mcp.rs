use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use crate::connection::{send_command, Response};
use crate::native::service_activity::service_incident_activity_response;
use crate::native::service_model::ServiceState;
use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

const BROWSERS_RESOURCE: &str = "agent-browser://browsers";
const EVENTS_RESOURCE: &str = "agent-browser://events";
const JOBS_RESOURCE: &str = "agent-browser://jobs";
const PROFILES_RESOURCE: &str = "agent-browser://profiles";
const PROVIDERS_RESOURCE: &str = "agent-browser://providers";
const SESSIONS_RESOURCE: &str = "agent-browser://sessions";
const SITE_POLICIES_RESOURCE: &str = "agent-browser://site-policies";
const TABS_RESOURCE: &str = "agent-browser://tabs";
const CHALLENGES_RESOURCE: &str = "agent-browser://challenges";
const INCIDENTS_RESOURCE: &str = "agent-browser://incidents";
const INCIDENT_ACTIVITY_PREFIX: &str = "agent-browser://incidents/";
const INCIDENT_ACTIVITY_SUFFIX: &str = "/activity";
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// Run the local MCP command surface.
///
/// `mcp serve` is a stdio JSON-RPC transport for MCP clients. The other
/// subcommands are shell inspection helpers over the same read-only resources.
pub fn run_mcp_command(args: &[String], json_output: bool, session: &str) -> i32 {
    if args.get(1).map(|value| value.as_str()) == Some("serve") {
        return match run_stdio_server(io::stdin().lock(), io::stdout().lock(), session) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{}", err);
                1
            }
        };
    }

    match mcp_command_response(args) {
        Ok(value) => {
            if json_output {
                println!("{}", serde_json::to_string(&value).unwrap_or_default());
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                );
            }
            0
        }
        Err(err) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "success": false,
                        "error": err,
                    }))
                    .unwrap_or_default()
                );
            } else {
                eprintln!("{}", err);
            }
            1
        }
    }
}

fn mcp_command_response(args: &[String]) -> Result<Value, String> {
    match args.get(1).map(|value| value.as_str()) {
        Some("resources") | Some("list") => Ok(json!({
            "success": true,
            "data": {
                "resources": service_mcp_resources(),
                "resourceTemplates": service_mcp_resource_templates(),
            },
        })),
        Some("read") => {
            let uri = args
                .get(2)
                .ok_or("Missing resource URI. Usage: agent-browser mcp read <uri>")?;
            if args.len() > 3 {
                return Err(format!("Unknown argument for mcp read: {}", args[3]));
            }
            Ok(json!({
                "success": true,
                "data": read_service_mcp_resource(uri)?,
            }))
        }
        Some(subcommand) => Err(format!(
            "Unknown mcp subcommand: {}. Valid options: serve, resources, read",
            subcommand
        )),
        None => Err(
            "Missing mcp subcommand. Usage: agent-browser mcp <serve|resources|read>".to_string(),
        ),
    }
}

fn service_mcp_resources() -> Vec<Value> {
    vec![
        json!({
            "uri": INCIDENTS_RESOURCE,
            "name": "Service incidents",
            "mimeType": "application/json",
            "description": "Grouped retained service incidents derived from service events and jobs"
        }),
        json!({
            "uri": PROFILES_RESOURCE,
            "name": "Service profiles",
            "mimeType": "application/json",
            "description": "Service-owned browser profile records sorted by profile id"
        }),
        json!({
            "uri": SESSIONS_RESOURCE,
            "name": "Service sessions",
            "mimeType": "application/json",
            "description": "Service-owned browser session lease records sorted by session id"
        }),
        json!({
            "uri": BROWSERS_RESOURCE,
            "name": "Service browsers",
            "mimeType": "application/json",
            "description": "Service-owned browser process records sorted by browser id"
        }),
        json!({
            "uri": TABS_RESOURCE,
            "name": "Service tabs",
            "mimeType": "application/json",
            "description": "Service-owned browser tab records sorted by tab id"
        }),
        json!({
            "uri": SITE_POLICIES_RESOURCE,
            "name": "Service site policies",
            "mimeType": "application/json",
            "description": "Service-owned site access policy records sorted by policy id"
        }),
        json!({
            "uri": PROVIDERS_RESOURCE,
            "name": "Service providers",
            "mimeType": "application/json",
            "description": "Service-owned integration provider records sorted by provider id"
        }),
        json!({
            "uri": CHALLENGES_RESOURCE,
            "name": "Service challenges",
            "mimeType": "application/json",
            "description": "Service-owned auth and challenge records sorted by challenge id"
        }),
        json!({
            "uri": JOBS_RESOURCE,
            "name": "Service jobs",
            "mimeType": "application/json",
            "description": "Retained service control-plane jobs sorted by submission time"
        }),
        json!({
            "uri": EVENTS_RESOURCE,
            "name": "Service events",
            "mimeType": "application/json",
            "description": "Retained service events in chronological order"
        }),
    ]
}

fn service_mcp_resource_templates() -> Vec<Value> {
    vec![json!({
        "uriTemplate": "agent-browser://incidents/{incident_id}/activity",
        "name": "Service incident activity",
        "mimeType": "application/json",
        "description": "Canonical service-owned chronological activity timeline for one incident"
    })]
}

fn read_service_mcp_resource(uri: &str) -> Result<Value, String> {
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path()?);
    let state = store.load()?;
    read_service_mcp_resource_from_state(uri, &state)
}

fn read_service_mcp_resource_from_state(uri: &str, state: &ServiceState) -> Result<Value, String> {
    let contents = match uri {
        INCIDENTS_RESOURCE => json!({
            "incidents": state.incidents,
            "count": state.incidents.len(),
        }),
        PROFILES_RESOURCE => {
            let profiles = state.profiles.values().cloned().collect::<Vec<_>>();
            json!({
                "profiles": profiles,
                "count": profiles.len(),
            })
        }
        SESSIONS_RESOURCE => {
            let sessions = state.sessions.values().cloned().collect::<Vec<_>>();
            json!({
                "sessions": sessions,
                "count": sessions.len(),
            })
        }
        BROWSERS_RESOURCE => {
            let browsers = state.browsers.values().cloned().collect::<Vec<_>>();
            json!({
                "browsers": browsers,
                "count": browsers.len(),
            })
        }
        TABS_RESOURCE => {
            let tabs = state.tabs.values().cloned().collect::<Vec<_>>();
            json!({
                "tabs": tabs,
                "count": tabs.len(),
            })
        }
        SITE_POLICIES_RESOURCE => {
            let site_policies = state.site_policies.values().cloned().collect::<Vec<_>>();
            json!({
                "sitePolicies": site_policies,
                "count": site_policies.len(),
            })
        }
        PROVIDERS_RESOURCE => {
            let providers = state.providers.values().cloned().collect::<Vec<_>>();
            json!({
                "providers": providers,
                "count": providers.len(),
            })
        }
        CHALLENGES_RESOURCE => {
            let challenges = state.challenges.values().cloned().collect::<Vec<_>>();
            json!({
                "challenges": challenges,
                "count": challenges.len(),
            })
        }
        JOBS_RESOURCE => {
            let mut jobs = state.jobs.values().cloned().collect::<Vec<_>>();
            jobs.sort_by(|left, right| {
                let left_time = left.submitted_at.as_deref().unwrap_or_default();
                let right_time = right.submitted_at.as_deref().unwrap_or_default();
                left_time
                    .cmp(right_time)
                    .then_with(|| left.id.cmp(&right.id))
            });
            json!({
                "jobs": jobs,
                "count": jobs.len(),
            })
        }
        EVENTS_RESOURCE => json!({
            "events": state.events,
            "count": state.events.len(),
        }),
        _ => {
            if let Some(incident_id) = incident_activity_resource_id(uri) {
                service_incident_activity_response(state, incident_id)?
            } else {
                return Err(format!("Unknown MCP resource URI: {}", uri));
            }
        }
    };

    Ok(json!({
        "uri": uri,
        "mimeType": "application/json",
        "contents": contents,
    }))
}

fn read_service_mcp_resource_contents(uri: &str) -> Result<Value, String> {
    let resource = read_service_mcp_resource(uri)?;
    let contents = resource
        .get("contents")
        .cloned()
        .ok_or_else(|| format!("MCP resource has no contents: {}", uri))?;
    let text = serde_json::to_string_pretty(&contents)
        .map_err(|err| format!("Failed to serialize MCP resource {}: {}", uri, err))?;
    Ok(json!({
        "contents": [
            {
                "uri": uri,
                "mimeType": "application/json",
                "text": text,
            }
        ],
    }))
}

fn run_stdio_server<R, W>(reader: R, mut writer: W, session: &str) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.map_err(|err| format!("Failed to read MCP stdin: {}", err))?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle_jsonrpc_line(&line, session) {
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&response)
                    .map_err(|err| format!("Failed to serialize MCP response: {}", err))?
            )
            .map_err(|err| format!("Failed to write MCP stdout: {}", err))?;
            writer
                .flush()
                .map_err(|err| format!("Failed to flush MCP stdout: {}", err))?;
        }
    }
    Ok(())
}

fn handle_jsonrpc_line(line: &str, session: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(line) {
        Ok(message) => handle_jsonrpc_message(&message, session),
        Err(err) => Some(jsonrpc_error(
            Value::Null,
            -32700,
            "Parse error",
            Some(json!({ "message": err.to_string() })),
        )),
    }
}

fn handle_jsonrpc_message(message: &Value, session: &str) -> Option<Value> {
    let Some(object) = message.as_object() else {
        return Some(jsonrpc_error(
            Value::Null,
            -32600,
            "Invalid Request",
            Some(json!({ "message": "JSON-RPC message must be an object" })),
        ));
    };

    let id = object.get("id").cloned();
    let Some(method) = object.get("method").and_then(|value| value.as_str()) else {
        if id.is_some() {
            return Some(jsonrpc_error(
                id.unwrap_or(Value::Null),
                -32600,
                "Invalid Request",
                Some(json!({ "message": "JSON-RPC request is missing method" })),
            ));
        }
        return None;
    };

    let Some(id) = id else {
        return handle_jsonrpc_notification(method);
    };

    match handle_jsonrpc_request(method, object.get("params"), session) {
        Ok(result) => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        })),
        Err(error) => Some(jsonrpc_error(id, error.code, error.message, error.data)),
    }
}

fn handle_jsonrpc_notification(method: &str) -> Option<Value> {
    match method {
        "notifications/initialized" | "notifications/cancelled" => None,
        _ => None,
    }
}

fn handle_jsonrpc_request(
    method: &str,
    params: Option<&Value>,
    session: &str,
) -> Result<Value, JsonRpcError> {
    match method {
        "initialize" => Ok(initialize_result(params)),
        "ping" => Ok(json!({})),
        "resources/list" => Ok(json!({
            "resources": service_mcp_resources(),
        })),
        "resources/templates/list" => Ok(json!({
            "resourceTemplates": service_mcp_resource_templates(),
        })),
        "resources/read" => {
            let uri = params
                .and_then(|value| value.get("uri"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    JsonRpcError::invalid_params("resources/read requires params.uri")
                })?;
            read_service_mcp_resource_contents(uri).map_err(|err| resource_read_error(uri, err))
        }
        "tools/list" => Ok(json!({ "tools": service_mcp_tools() })),
        "tools/call" => call_service_mcp_tool(params, session),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        _ => Err(JsonRpcError::method_not_found(method)),
    }
}

fn initialize_result(params: Option<&Value>) -> Value {
    let requested_protocol = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(|value| value.as_str());
    let protocol_version = if requested_protocol == Some(MCP_PROTOCOL_VERSION) {
        requested_protocol.unwrap()
    } else {
        MCP_PROTOCOL_VERSION
    };

    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "resources": {},
            "tools": {},
        },
        "serverInfo": {
            "name": "agent-browser",
            "title": "Agent Browser",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": "agent-browser service resources plus queued browser-control tools. Include serviceName, agentName, and taskName on tools/call whenever possible.",
    })
}

fn service_mcp_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "service_job_cancel",
            "title": "Cancel service job",
            "description": "Cancel a queued service job or request cancellation for a running service job. Include serviceName, agentName, and taskName when available to make multi-agent traces debuggable.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "jobId": {
                        "type": "string",
                        "description": "Service job id to cancel."
                    },
                    "reason": {
                        "type": "string",
                        "description": "Human-readable cancellation reason."
                    },
                    "serviceName": {
                        "type": "string",
                        "description": "Calling service name, for example JournalDownloader."
                    },
                    "agentName": {
                        "type": "string",
                        "description": "Calling agent name."
                    },
                    "taskName": {
                        "type": "string",
                        "description": "Calling task name, for example probeACSwebsite."
                    }
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "browser_snapshot",
            "title": "Take browser snapshot",
            "description": "Queue a browser accessibility snapshot against the active session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Optional CSS selector to scope the snapshot."
                    },
                    "interactive": {
                        "type": "boolean",
                        "description": "Return only interactive elements and assign refs."
                    },
                    "compact": {
                        "type": "boolean",
                        "description": "Remove empty structural elements from the snapshot."
                    },
                    "maxDepth": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Optional maximum accessibility-tree depth."
                    },
                    "urls": {
                        "type": "boolean",
                        "description": "Include href URLs for links when available."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued snapshot job."
                    },
                    "serviceName": {
                        "type": "string",
                        "description": "Calling service name, for example JournalDownloader."
                    },
                    "agentName": {
                        "type": "string",
                        "description": "Calling agent name."
                    },
                    "taskName": {
                        "type": "string",
                        "description": "Calling task name, for example probeACSwebsite."
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "browser_get_url",
            "title": "Get browser URL",
            "description": "Queue a read of the active browser session URL. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued URL read job."
                    },
                    "serviceName": {
                        "type": "string",
                        "description": "Calling service name, for example JournalDownloader."
                    },
                    "agentName": {
                        "type": "string",
                        "description": "Calling agent name."
                    },
                    "taskName": {
                        "type": "string",
                        "description": "Calling task name, for example probeACSwebsite."
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "browser_get_title",
            "title": "Get browser title",
            "description": "Queue a read of the active browser session title. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued title read job."
                    },
                    "serviceName": {
                        "type": "string",
                        "description": "Calling service name, for example JournalDownloader."
                    },
                    "agentName": {
                        "type": "string",
                        "description": "Calling agent name."
                    },
                    "taskName": {
                        "type": "string",
                        "description": "Calling task name, for example probeACSwebsite."
                    }
                },
                "required": []
            }
        }),
    ]
}

fn call_service_mcp_tool(params: Option<&Value>, session: &str) -> Result<Value, JsonRpcError> {
    let name = params
        .and_then(|value| value.get("name"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| JsonRpcError::invalid_params("tools/call requires params.name"))?;
    let arguments = params
        .and_then(|value| value.get("arguments"))
        .unwrap_or(&Value::Null);

    match name {
        "service_job_cancel" => call_service_job_cancel(arguments, session),
        "browser_snapshot" => call_browser_snapshot(arguments, session),
        "browser_get_url" => call_browser_get_url(arguments, session),
        "browser_get_title" => call_browser_get_title(arguments, session),
        _ => Err(JsonRpcError {
            code: -32602,
            message: "Invalid params",
            data: Some(json!({ "message": format!("Unknown MCP tool: {}", name) })),
        }),
    }
}

fn call_service_job_cancel(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let job_id = arguments
        .get("jobId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| JsonRpcError::invalid_params("service_job_cancel requires jobId"))?;
    let reason = optional_string_argument(arguments, "reason")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);

    let command = service_job_cancel_command(job_id, reason, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "service_job_cancel",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "service_job_cancel",
        session,
        trace,
        response,
    ))
}

fn call_browser_snapshot(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = optional_string_argument(arguments, "selector")?;
    let interactive = optional_bool_argument(arguments, "interactive")?;
    let compact = optional_bool_argument(arguments, "compact")?;
    let max_depth = optional_u64_argument(arguments, "maxDepth")?;
    let urls = optional_bool_argument(arguments, "urls")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_snapshot_command(BrowserSnapshotCommandArgs {
        selector,
        interactive,
        compact,
        max_depth,
        urls,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    });

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_snapshot",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_snapshot",
        session,
        trace,
        response,
    ))
}

fn call_browser_get_url(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_get_url_command(job_timeout_ms, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_get_url",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_get_url",
        session,
        trace,
        response,
    ))
}

fn call_browser_get_title(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_get_title_command(job_timeout_ms, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_get_title",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_get_title",
        session,
        trace,
        response,
    ))
}

fn service_job_cancel_command(
    job_id: &str,
    reason: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-job-cancel-{}", uuid::Uuid::new_v4()),
        "action": "service_job_cancel",
        "jobId": job_id,
    });
    if let Some(reason) = reason {
        command["reason"] = json!(reason);
    }
    if let Some(service_name) = service_name {
        command["serviceName"] = json!(service_name);
    }
    if let Some(agent_name) = agent_name {
        command["agentName"] = json!(agent_name);
    }
    if let Some(task_name) = task_name {
        command["taskName"] = json!(task_name);
    }
    command
}

fn browser_get_url_command(
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-get-url-{}", uuid::Uuid::new_v4()),
        "action": "url",
    });
    if let Some(job_timeout_ms) = job_timeout_ms {
        command["jobTimeoutMs"] = json!(job_timeout_ms);
    }
    if let Some(service_name) = service_name {
        command["serviceName"] = json!(service_name);
    }
    if let Some(agent_name) = agent_name {
        command["agentName"] = json!(agent_name);
    }
    if let Some(task_name) = task_name {
        command["taskName"] = json!(task_name);
    }
    command
}

fn browser_get_title_command(
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-get-title-{}", uuid::Uuid::new_v4()),
        "action": "title",
    });
    if let Some(job_timeout_ms) = job_timeout_ms {
        command["jobTimeoutMs"] = json!(job_timeout_ms);
    }
    if let Some(service_name) = service_name {
        command["serviceName"] = json!(service_name);
    }
    if let Some(agent_name) = agent_name {
        command["agentName"] = json!(agent_name);
    }
    if let Some(task_name) = task_name {
        command["taskName"] = json!(task_name);
    }
    command
}

struct BrowserSnapshotCommandArgs<'a> {
    selector: Option<&'a str>,
    interactive: Option<bool>,
    compact: Option<bool>,
    max_depth: Option<u64>,
    urls: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_snapshot_command(args: BrowserSnapshotCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-snapshot-{}", uuid::Uuid::new_v4()),
        "action": "snapshot",
    });
    if let Some(selector) = args.selector {
        command["selector"] = json!(selector);
    }
    if let Some(interactive) = args.interactive {
        command["interactive"] = json!(interactive);
    }
    if let Some(compact) = args.compact {
        command["compact"] = json!(compact);
    }
    if let Some(max_depth) = args.max_depth {
        command["maxDepth"] = json!(max_depth);
    }
    if let Some(urls) = args.urls {
        command["urls"] = json!(urls);
    }
    if let Some(job_timeout_ms) = args.job_timeout_ms {
        command["jobTimeoutMs"] = json!(job_timeout_ms);
    }
    if let Some(service_name) = args.service_name {
        command["serviceName"] = json!(service_name);
    }
    if let Some(agent_name) = args.agent_name {
        command["agentName"] = json!(agent_name);
    }
    if let Some(task_name) = args.task_name {
        command["taskName"] = json!(task_name);
    }
    command
}

fn optional_string_argument<'a>(
    arguments: &'a Value,
    name: &str,
) -> Result<Option<&'a str>, JsonRpcError> {
    match arguments.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(Some)
            .ok_or_else(|| {
                JsonRpcError::invalid_params(&format!("{} must be a non-empty string", name))
            }),
        None => Ok(None),
    }
}

fn optional_bool_argument(arguments: &Value, name: &str) -> Result<Option<bool>, JsonRpcError> {
    match arguments.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} must be a boolean", name))),
        None => Ok(None),
    }
}

fn optional_u64_argument(arguments: &Value, name: &str) -> Result<Option<u64>, JsonRpcError> {
    match arguments.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            JsonRpcError::invalid_params(&format!("{} must be a non-negative integer", name))
        }),
        None => Ok(None),
    }
}

fn optional_positive_u64_argument(
    arguments: &Value,
    name: &str,
) -> Result<Option<u64>, JsonRpcError> {
    match optional_u64_argument(arguments, name)? {
        Some(0) => Err(JsonRpcError::invalid_params(&format!(
            "{} must be a positive integer",
            name
        ))),
        value => Ok(value),
    }
}

fn service_tool_trace(
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    json!({
        "serviceName": service_name,
        "agentName": agent_name,
        "taskName": task_name,
    })
}

fn tool_response_from_daemon(
    tool_name: &str,
    session: &str,
    trace: Value,
    response: Response,
) -> Value {
    let payload = json!({
        "tool": tool_name,
        "session": session,
        "trace": trace,
        "success": response.success,
        "data": response.data,
        "error": response.error,
    });
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&payload).unwrap_or_default(),
            }
        ],
        "isError": !response.success,
    })
}

fn resource_read_error(uri: &str, err: String) -> JsonRpcError {
    if err.contains("not found") || err.contains("Unknown MCP resource URI") {
        JsonRpcError {
            code: -32002,
            message: "Resource not found",
            data: Some(json!({ "uri": uri, "message": err })),
        }
    } else {
        JsonRpcError {
            code: -32603,
            message: "Internal error",
            data: Some(json!({ "uri": uri, "message": err })),
        }
    }
}

fn jsonrpc_error(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message,
    });
    if let Some(data) = data {
        error["data"] = data;
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error,
    })
}

struct JsonRpcError {
    code: i64,
    message: &'static str,
    data: Option<Value>,
}

impl JsonRpcError {
    fn invalid_params(message: &str) -> Self {
        Self {
            code: -32602,
            message: "Invalid params",
            data: Some(json!({ "message": message })),
        }
    }

    fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: "Method not found",
            data: Some(json!({ "method": method })),
        }
    }
}

fn incident_activity_resource_id(uri: &str) -> Option<&str> {
    uri.strip_prefix(INCIDENT_ACTIVITY_PREFIX)
        .and_then(|rest| rest.strip_suffix(INCIDENT_ACTIVITY_SUFFIX))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_resources_lists_read_only_service_resources() {
        let response = mcp_command_response(&["mcp".to_string(), "resources".to_string()]).unwrap();

        assert_eq!(response["success"], true);
        assert_eq!(response["data"]["resources"][0]["uri"], INCIDENTS_RESOURCE);
        assert_eq!(response["data"]["resources"][1]["uri"], PROFILES_RESOURCE);
        assert_eq!(response["data"]["resources"][2]["uri"], SESSIONS_RESOURCE);
        assert_eq!(response["data"]["resources"][3]["uri"], BROWSERS_RESOURCE);
        assert_eq!(response["data"]["resources"][4]["uri"], TABS_RESOURCE);
        assert_eq!(
            response["data"]["resources"][5]["uri"],
            SITE_POLICIES_RESOURCE
        );
        assert_eq!(response["data"]["resources"][6]["uri"], PROVIDERS_RESOURCE);
        assert_eq!(response["data"]["resources"][7]["uri"], CHALLENGES_RESOURCE);
        assert_eq!(response["data"]["resources"][8]["uri"], JOBS_RESOURCE);
        assert_eq!(response["data"]["resources"][9]["uri"], EVENTS_RESOURCE);
        assert_eq!(
            response["data"]["resourceTemplates"][0]["uriTemplate"],
            "agent-browser://incidents/{incident_id}/activity"
        );
    }

    #[test]
    fn incident_activity_resource_id_maps_uri() {
        assert_eq!(
            incident_activity_resource_id("agent-browser://incidents/browser-1/activity"),
            Some("browser-1")
        );
        assert_eq!(
            incident_activity_resource_id("agent-browser://incidents//activity"),
            None
        );
        assert_eq!(
            incident_activity_resource_id("agent-browser://incidents/browser-1/events"),
            None
        );
    }

    #[test]
    fn initialize_returns_read_only_resource_capability() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert!(response["result"]["capabilities"]["resources"].is_object());
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn resources_list_returns_jsonrpc_resources() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"r1","method":"resources/list"}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], "r1");
        assert_eq!(
            response["result"]["resources"][0]["uri"],
            INCIDENTS_RESOURCE
        );
        assert_eq!(response["result"]["resources"][1]["uri"], PROFILES_RESOURCE);
        assert_eq!(response["result"]["resources"][2]["uri"], SESSIONS_RESOURCE);
        assert_eq!(response["result"]["resources"][3]["uri"], BROWSERS_RESOURCE);
        assert_eq!(response["result"]["resources"][4]["uri"], TABS_RESOURCE);
        assert_eq!(
            response["result"]["resources"][5]["uri"],
            SITE_POLICIES_RESOURCE
        );
        assert_eq!(
            response["result"]["resources"][6]["uri"],
            PROVIDERS_RESOURCE
        );
        assert_eq!(
            response["result"]["resources"][7]["uri"],
            CHALLENGES_RESOURCE
        );
        assert_eq!(response["result"]["resources"][8]["uri"], JOBS_RESOURCE);
        assert_eq!(response["result"]["resources"][9]["uri"], EVENTS_RESOURCE);
    }

    #[test]
    fn resource_templates_list_returns_jsonrpc_templates() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"t1","method":"resources/templates/list"}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], "t1");
        assert_eq!(
            response["result"]["resourceTemplates"][0]["uriTemplate"],
            "agent-browser://incidents/{incident_id}/activity"
        );
    }

    #[test]
    fn notifications_do_not_return_responses() {
        assert!(handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "default",
        )
        .is_none());
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let response = handle_jsonrpc_line("{bad json", "default").unwrap();

        assert_eq!(response["id"], Value::Null);
        assert_eq!(response["error"]["code"], -32700);
    }

    #[test]
    fn missing_resource_uri_returns_invalid_params() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"resources/read"}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 2);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn tools_list_returns_service_job_cancel() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"tools","method":"tools/list"}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], "tools");
        assert_eq!(response["result"]["tools"][0]["name"], "service_job_cancel");
        assert_eq!(
            response["result"]["tools"][0]["inputSchema"]["required"][0],
            "jobId"
        );
        assert!(
            response["result"]["tools"][0]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert!(
            response["result"]["tools"][0]["inputSchema"]["properties"]["agentName"].is_object()
        );
        assert!(
            response["result"]["tools"][0]["inputSchema"]["properties"]["taskName"].is_object()
        );
        assert_eq!(response["result"]["tools"][1]["name"], "browser_snapshot");
        assert!(
            response["result"]["tools"][1]["inputSchema"]["properties"]["interactive"].is_object()
        );
        assert!(
            response["result"]["tools"][1]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][2]["name"], "browser_get_url");
        assert!(
            response["result"]["tools"][2]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert!(
            response["result"]["tools"][2]["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
        );
        assert_eq!(response["result"]["tools"][3]["name"], "browser_get_title");
        assert!(
            response["result"]["tools"][3]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert!(
            response["result"]["tools"][3]["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
        );
    }

    #[test]
    fn tools_call_requires_tool_name() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call"}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 3);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn service_job_cancel_requires_job_id_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"service_job_cancel","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 4);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_snapshot_rejects_invalid_argument_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"browser_snapshot","arguments":{"interactive":"true"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 5);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_get_url_rejects_invalid_timeout_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"browser_get_url","arguments":{"jobTimeoutMs":0}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 6);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_get_title_rejects_invalid_timeout_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"browser_get_title","arguments":{"jobTimeoutMs":0}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 7);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn service_tool_trace_preserves_names() {
        let trace = service_tool_trace(
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(trace["serviceName"], "JournalDownloader");
        assert_eq!(trace["agentName"], "agent-a");
        assert_eq!(trace["taskName"], "probeACSwebsite");
    }

    #[test]
    fn service_job_cancel_command_forwards_trace_fields() {
        let command = service_job_cancel_command(
            "job-1",
            Some("stale"),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "service_job_cancel");
        assert_eq!(command["jobId"], "job-1");
        assert_eq!(command["reason"], "stale");
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_snapshot_command_forwards_options_and_trace_fields() {
        let command = browser_snapshot_command(BrowserSnapshotCommandArgs {
            selector: Some("#main"),
            interactive: Some(true),
            compact: Some(true),
            max_depth: Some(3),
            urls: Some(true),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(command["action"], "snapshot");
        assert_eq!(command["selector"], "#main");
        assert_eq!(command["interactive"], true);
        assert_eq!(command["compact"], true);
        assert_eq!(command["maxDepth"], 3);
        assert_eq!(command["urls"], true);
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_get_url_command_forwards_timeout_and_trace_fields() {
        let command = browser_get_url_command(
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "url");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_get_title_command_forwards_timeout_and_trace_fields() {
        let command = browser_get_title_command(
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "title");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn tool_response_includes_trace_and_error_flag() {
        let response = Response {
            success: false,
            data: None,
            error: Some("Service job not found: job-1".to_string()),
            warning: None,
        };
        let tool_response = tool_response_from_daemon(
            "service_job_cancel",
            "default",
            service_tool_trace(Some("svc"), Some("agent"), Some("task")),
            response,
        );
        let text = tool_response["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).unwrap();

        assert_eq!(tool_response["isError"], true);
        assert_eq!(payload["tool"], "service_job_cancel");
        assert_eq!(payload["session"], "default");
        assert_eq!(payload["trace"]["serviceName"], "svc");
        assert_eq!(payload["error"], "Service job not found: job-1");
    }

    #[test]
    fn stdio_server_processes_newline_delimited_messages() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"resources/list"}"#,
            "\n"
        );
        let mut output = Vec::new();

        run_stdio_server(input.as_bytes(), &mut output, "default").unwrap();
        let lines = String::from_utf8(output).unwrap();
        let responses = lines.lines().collect::<Vec<_>>();

        assert_eq!(responses.len(), 2);
        assert!(responses[0].contains(r#""method""#) == false);
        assert!(responses[1].contains("agent-browser://incidents"));
        assert!(responses[1].contains("agent-browser://profiles"));
        assert!(responses[1].contains("agent-browser://sessions"));
        assert!(responses[1].contains("agent-browser://browsers"));
        assert!(responses[1].contains("agent-browser://tabs"));
        assert!(responses[1].contains("agent-browser://site-policies"));
        assert!(responses[1].contains("agent-browser://providers"));
        assert!(responses[1].contains("agent-browser://challenges"));
        assert!(responses[1].contains("agent-browser://jobs"));
        assert!(responses[1].contains("agent-browser://events"));
    }

    #[test]
    fn read_profiles_resource_returns_profiles_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{BrowserHost, BrowserProfile};

        let state = ServiceState {
            profiles: BTreeMap::from([
                (
                    "profile-b".to_string(),
                    BrowserProfile {
                        id: "profile-b".to_string(),
                        name: "Profile B".to_string(),
                        default_browser_host: Some(BrowserHost::LocalHeaded),
                        ..BrowserProfile::default()
                    },
                ),
                (
                    "profile-a".to_string(),
                    BrowserProfile {
                        id: "profile-a".to_string(),
                        name: "Profile A".to_string(),
                        default_browser_host: Some(BrowserHost::LocalHeadless),
                        ..BrowserProfile::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(PROFILES_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["profiles"][0]["id"], "profile-a");
        assert_eq!(resource["contents"]["profiles"][1]["id"], "profile-b");
    }

    #[test]
    fn read_sessions_resource_returns_sessions_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{BrowserSession, LeaseState};

        let state = ServiceState {
            sessions: BTreeMap::from([
                (
                    "session-b".to_string(),
                    BrowserSession {
                        id: "session-b".to_string(),
                        lease: LeaseState::Exclusive,
                        ..BrowserSession::default()
                    },
                ),
                (
                    "session-a".to_string(),
                    BrowserSession {
                        id: "session-a".to_string(),
                        lease: LeaseState::Shared,
                        ..BrowserSession::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(SESSIONS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["sessions"][0]["id"], "session-a");
        assert_eq!(resource["contents"]["sessions"][1]["id"], "session-b");
    }

    #[test]
    fn read_browsers_resource_returns_browsers_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{BrowserHealth, BrowserProcess};

        let state = ServiceState {
            browsers: BTreeMap::from([
                (
                    "browser-b".to_string(),
                    BrowserProcess {
                        id: "browser-b".to_string(),
                        health: BrowserHealth::Ready,
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "browser-a".to_string(),
                    BrowserProcess {
                        id: "browser-a".to_string(),
                        health: BrowserHealth::NotStarted,
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(BROWSERS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["browsers"][0]["id"], "browser-a");
        assert_eq!(resource["contents"]["browsers"][1]["id"], "browser-b");
    }

    #[test]
    fn read_tabs_resource_returns_tabs_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{BrowserTab, TabLifecycle};

        let state = ServiceState {
            tabs: BTreeMap::from([
                (
                    "tab-b".to_string(),
                    BrowserTab {
                        id: "tab-b".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-a".to_string(),
                    BrowserTab {
                        id: "tab-a".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Loading,
                        ..BrowserTab::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(TABS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["tabs"][0]["id"], "tab-a");
        assert_eq!(resource["contents"]["tabs"][1]["id"], "tab-b");
    }

    #[test]
    fn read_site_policies_resource_returns_policies_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::SitePolicy;

        let state = ServiceState {
            site_policies: BTreeMap::from([
                (
                    "microsoft".to_string(),
                    SitePolicy {
                        id: "microsoft".to_string(),
                        origin_pattern: "https://login.microsoftonline.com".to_string(),
                        ..SitePolicy::default()
                    },
                ),
                (
                    "google".to_string(),
                    SitePolicy {
                        id: "google".to_string(),
                        origin_pattern: "https://accounts.google.com".to_string(),
                        ..SitePolicy::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource =
            read_service_mcp_resource_from_state(SITE_POLICIES_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["sitePolicies"][0]["id"], "google");
        assert_eq!(resource["contents"]["sitePolicies"][1]["id"], "microsoft");
    }

    #[test]
    fn read_providers_resource_returns_providers_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{ProviderKind, ServiceProvider};

        let state = ServiceState {
            providers: BTreeMap::from([
                (
                    "sms".to_string(),
                    ServiceProvider {
                        id: "sms".to_string(),
                        kind: ProviderKind::Sms,
                        display_name: "SMS".to_string(),
                        ..ServiceProvider::default()
                    },
                ),
                (
                    "manual".to_string(),
                    ServiceProvider {
                        id: "manual".to_string(),
                        kind: ProviderKind::ManualApproval,
                        display_name: "Manual approval".to_string(),
                        ..ServiceProvider::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(PROVIDERS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["providers"][0]["id"], "manual");
        assert_eq!(resource["contents"]["providers"][1]["id"], "sms");
    }

    #[test]
    fn read_challenges_resource_returns_challenges_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{Challenge, ChallengeKind, ChallengeState};

        let state = ServiceState {
            challenges: BTreeMap::from([
                (
                    "challenge-b".to_string(),
                    Challenge {
                        id: "challenge-b".to_string(),
                        kind: ChallengeKind::TwoFactor,
                        state: ChallengeState::WaitingForProvider,
                        ..Challenge::default()
                    },
                ),
                (
                    "challenge-a".to_string(),
                    Challenge {
                        id: "challenge-a".to_string(),
                        kind: ChallengeKind::Captcha,
                        state: ChallengeState::Detected,
                        ..Challenge::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(CHALLENGES_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["challenges"][0]["id"], "challenge-a");
        assert_eq!(resource["contents"]["challenges"][1]["id"], "challenge-b");
    }

    #[test]
    fn read_jobs_resource_returns_jobs_sorted_by_submission_time() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{JobState, JobTarget, ServiceJob};

        let state = ServiceState {
            jobs: BTreeMap::from([
                (
                    "job-b".to_string(),
                    ServiceJob {
                        id: "job-b".to_string(),
                        action: "snapshot".to_string(),
                        target: JobTarget::Service,
                        state: JobState::Succeeded,
                        submitted_at: Some("2026-04-22T00:02:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                ),
                (
                    "job-a".to_string(),
                    ServiceJob {
                        id: "job-a".to_string(),
                        action: "navigate".to_string(),
                        target: JobTarget::Service,
                        state: JobState::Queued,
                        submitted_at: Some("2026-04-22T00:01:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(JOBS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["jobs"][0]["id"], "job-a");
        assert_eq!(resource["contents"]["jobs"][1]["id"], "job-b");
    }

    #[test]
    fn read_events_resource_returns_retained_events() {
        use crate::native::service_model::{ServiceEvent, ServiceEventKind};

        let state = ServiceState {
            events: vec![ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::Reconciliation,
                message: "Reconciled service state".to_string(),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(EVENTS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 1);
        assert_eq!(resource["contents"]["events"][0]["id"], "event-1");
    }
}
