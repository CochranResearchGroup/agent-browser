use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use crate::native::service_activity::service_incident_activity_response;
use crate::native::service_model::ServiceState;
use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

const EVENTS_RESOURCE: &str = "agent-browser://events";
const JOBS_RESOURCE: &str = "agent-browser://jobs";
const INCIDENTS_RESOURCE: &str = "agent-browser://incidents";
const INCIDENT_ACTIVITY_PREFIX: &str = "agent-browser://incidents/";
const INCIDENT_ACTIVITY_SUFFIX: &str = "/activity";
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// Run the local MCP command surface.
///
/// `mcp serve` is a stdio JSON-RPC transport for MCP clients. The other
/// subcommands are shell inspection helpers over the same read-only resources.
pub fn run_mcp_command(args: &[String], json_output: bool) -> i32 {
    if args.get(1).map(|value| value.as_str()) == Some("serve") {
        return match run_stdio_server(io::stdin().lock(), io::stdout().lock()) {
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

fn run_stdio_server<R, W>(reader: R, mut writer: W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.map_err(|err| format!("Failed to read MCP stdin: {}", err))?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle_jsonrpc_line(&line) {
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

fn handle_jsonrpc_line(line: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(line) {
        Ok(message) => handle_jsonrpc_message(&message),
        Err(err) => Some(jsonrpc_error(
            Value::Null,
            -32700,
            "Parse error",
            Some(json!({ "message": err.to_string() })),
        )),
    }
}

fn handle_jsonrpc_message(message: &Value) -> Option<Value> {
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

    match handle_jsonrpc_request(method, object.get("params")) {
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

fn handle_jsonrpc_request(method: &str, params: Option<&Value>) -> Result<Value, JsonRpcError> {
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
        "tools/list" => Ok(json!({ "tools": [] })),
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
        },
        "serverInfo": {
            "name": "agent-browser",
            "title": "Agent Browser",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": "Read-only agent-browser service resources. Browser control tools are intentionally not exposed yet.",
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
        assert_eq!(response["data"]["resources"][1]["uri"], JOBS_RESOURCE);
        assert_eq!(response["data"]["resources"][2]["uri"], EVENTS_RESOURCE);
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
        )
        .unwrap();

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert!(response["result"]["capabilities"]["resources"].is_object());
    }

    #[test]
    fn resources_list_returns_jsonrpc_resources() {
        let response =
            handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":"r1","method":"resources/list"}"#)
                .unwrap();

        assert_eq!(response["id"], "r1");
        assert_eq!(
            response["result"]["resources"][0]["uri"],
            INCIDENTS_RESOURCE
        );
        assert_eq!(response["result"]["resources"][1]["uri"], JOBS_RESOURCE);
        assert_eq!(response["result"]["resources"][2]["uri"], EVENTS_RESOURCE);
    }

    #[test]
    fn resource_templates_list_returns_jsonrpc_templates() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"t1","method":"resources/templates/list"}"#,
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
        assert!(
            handle_jsonrpc_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let response = handle_jsonrpc_line("{bad json").unwrap();

        assert_eq!(response["id"], Value::Null);
        assert_eq!(response["error"]["code"], -32700);
    }

    #[test]
    fn missing_resource_uri_returns_invalid_params() {
        let response =
            handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":2,"method":"resources/read"}"#).unwrap();

        assert_eq!(response["id"], 2);
        assert_eq!(response["error"]["code"], -32602);
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

        run_stdio_server(input.as_bytes(), &mut output).unwrap();
        let lines = String::from_utf8(output).unwrap();
        let responses = lines.lines().collect::<Vec<_>>();

        assert_eq!(responses.len(), 2);
        assert!(responses[0].contains(r#""method""#) == false);
        assert!(responses[1].contains("agent-browser://incidents"));
        assert!(responses[1].contains("agent-browser://jobs"));
        assert!(responses[1].contains("agent-browser://events"));
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
