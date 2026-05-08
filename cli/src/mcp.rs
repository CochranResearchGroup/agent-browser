use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use crate::connection::{send_command, Response};
use crate::native::service_access::{
    parse_service_access_plan_query, service_access_plan_for_state,
};
use crate::native::service_activity::service_incident_activity_response;
use crate::native::service_contracts::{
    service_contracts_metadata, SERVICE_ACCESS_PLAN_MCP_RESOURCE, SERVICE_CONTRACTS_RESOURCE,
    SERVICE_REQUEST_ACTIONS,
};
use crate::native::service_incidents::{
    service_incident_summary, service_incidents_response, ServiceIncidentFilters,
};
use crate::native::service_model::{
    service_profile_allocations, service_profile_sources, service_site_policy_sources, ServiceState,
};
use crate::native::service_store::load_default_service_state_snapshot;
use crate::native::service_trace::{service_trace_response, ServiceTraceFilters};

const BROWSERS_RESOURCE: &str = "agent-browser://browsers";
const EVENTS_RESOURCE: &str = "agent-browser://events";
const JOBS_RESOURCE: &str = "agent-browser://jobs";
const PROFILES_RESOURCE: &str = "agent-browser://profiles";
const PROVIDERS_RESOURCE: &str = "agent-browser://providers";
const SESSIONS_RESOURCE: &str = "agent-browser://sessions";
const SITE_POLICIES_RESOURCE: &str = "agent-browser://site-policies";
const TABS_RESOURCE: &str = "agent-browser://tabs";
const MONITORS_RESOURCE: &str = "agent-browser://monitors";
const CHALLENGES_RESOURCE: &str = "agent-browser://challenges";
const INCIDENTS_RESOURCE: &str = "agent-browser://incidents";
const INCIDENT_ACTIVITY_PREFIX: &str = "agent-browser://incidents/";
const INCIDENT_ACTIVITY_SUFFIX: &str = "/activity";
const ACCESS_PLAN_TEMPLATE: &str = "agent-browser://access-plan{?serviceName,agentName,taskName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,sitePolicyId,challengeId,readinessProfileId}";
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const BROWSER_COMMAND_ALLOWED_ACTIONS: &[&str] = SERVICE_REQUEST_ACTIONS;

/// Run the local MCP command surface.
///
/// `mcp serve` is a stdio JSON-RPC transport for MCP clients. The other
/// subcommands are shell inspection helpers over the same read-only resources.
pub fn run_mcp_command(
    args: &[String],
    json_output: bool,
    session: &str,
    configured_service_state: &ServiceState,
) -> i32 {
    if args.get(1).map(|value| value.as_str()) == Some("serve") {
        return match run_stdio_server(
            io::stdin().lock(),
            io::stdout().lock(),
            session,
            configured_service_state.clone(),
        ) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{}", err);
                1
            }
        };
    }

    match mcp_command_response_with_config(args, configured_service_state) {
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

#[cfg(test)]
fn mcp_command_response(args: &[String]) -> Result<Value, String> {
    mcp_command_response_with_config(args, &ServiceState::default())
}

fn mcp_command_response_with_config(
    args: &[String],
    configured_service_state: &ServiceState,
) -> Result<Value, String> {
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
                "data": read_service_mcp_resource(uri, configured_service_state)?,
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
            "uri": SERVICE_CONTRACTS_RESOURCE,
            "name": "Service contracts",
            "mimeType": "application/json",
            "description": "Runtime compatibility metadata for service HTTP and MCP contracts"
        }),
        json!({
            "uri": SERVICE_ACCESS_PLAN_MCP_RESOURCE,
            "name": "Service access plan",
            "mimeType": "application/json",
            "description": "No-launch service-owned access recommendation combining profile readiness, site policy, providers, challenges, and target identity"
        }),
        json!({
            "uri": INCIDENTS_RESOURCE,
            "name": "Service incidents",
            "mimeType": "application/json",
            "description": "Grouped retained service incidents with severity and escalation derived from service events and jobs"
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
            "uri": MONITORS_RESOURCE,
            "name": "Service monitors",
            "mimeType": "application/json",
            "description": "Service-owned recurring heartbeat and freshness monitor records sorted by monitor id"
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
    vec![
        json!({
            "uriTemplate": ACCESS_PLAN_TEMPLATE,
            "name": "Service access plan",
            "mimeType": "application/json",
            "description": "No-launch service-owned access recommendation for one service, site, login, policy, or challenge selector"
        }),
        json!({
            "uriTemplate": "agent-browser://incidents/{incident_id}/activity",
            "name": "Service incident activity",
            "mimeType": "application/json",
            "description": "Canonical service-owned chronological activity timeline for one incident"
        }),
    ]
}

fn read_service_mcp_resource(
    uri: &str,
    configured_service_state: &ServiceState,
) -> Result<Value, String> {
    let mut state = load_default_service_state_snapshot()?;
    state.overlay_configured_entities(configured_service_state.clone());
    read_service_mcp_resource_from_state(uri, &state)
}

fn read_service_mcp_resource_from_state(uri: &str, state: &ServiceState) -> Result<Value, String> {
    let mut state = state.clone();
    state.refresh_profile_readiness();
    let contents = match uri {
        SERVICE_CONTRACTS_RESOURCE => service_contracts_metadata(),
        SERVICE_ACCESS_PLAN_MCP_RESOURCE => {
            service_access_plan_for_state(&state, Default::default())
        }
        INCIDENTS_RESOURCE => json!({
            "incidents": state.incidents,
            "count": state.incidents.len(),
        }),
        PROFILES_RESOURCE => {
            let profile_allocations = service_profile_allocations(&state);
            let profile_sources = service_profile_sources(&state);
            let profiles = state.profiles.values().cloned().collect::<Vec<_>>();
            json!({
                "profiles": profiles,
                "profileSources": profile_sources,
                "profileAllocations": profile_allocations,
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
        MONITORS_RESOURCE => {
            let monitors = state.monitors.values().cloned().collect::<Vec<_>>();
            json!({
                "monitors": monitors,
                "count": monitors.len(),
            })
        }
        SITE_POLICIES_RESOURCE => {
            let site_policies = state.site_policies.values().cloned().collect::<Vec<_>>();
            let site_policy_sources = service_site_policy_sources(&state);
            json!({
                "sitePolicies": site_policies,
                "sitePolicySources": site_policy_sources,
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
                service_incident_activity_response(&state, incident_id)?
            } else if let Some(query) = access_plan_resource_query(uri) {
                let request = parse_service_access_plan_query(query)?;
                service_access_plan_for_state(&state, request)
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

fn read_service_mcp_resource_contents_with_config(
    uri: &str,
    configured_service_state: &ServiceState,
) -> Result<Value, String> {
    let resource = read_service_mcp_resource(uri, configured_service_state)?;
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

fn run_stdio_server<R, W>(
    reader: R,
    mut writer: W,
    session: &str,
    configured_service_state: ServiceState,
) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.map_err(|err| format!("Failed to read MCP stdin: {}", err))?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) =
            handle_jsonrpc_line_with_config(&line, session, &configured_service_state)
        {
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

#[cfg(test)]
fn handle_jsonrpc_line(line: &str, session: &str) -> Option<Value> {
    handle_jsonrpc_line_with_config(line, session, &ServiceState::default())
}

fn handle_jsonrpc_line_with_config(
    line: &str,
    session: &str,
    configured_service_state: &ServiceState,
) -> Option<Value> {
    match serde_json::from_str::<Value>(line) {
        Ok(message) => handle_jsonrpc_message(&message, session, configured_service_state),
        Err(err) => Some(jsonrpc_error(
            Value::Null,
            -32700,
            "Parse error",
            Some(json!({ "message": err.to_string() })),
        )),
    }
}

fn handle_jsonrpc_message(
    message: &Value,
    session: &str,
    configured_service_state: &ServiceState,
) -> Option<Value> {
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

    match handle_jsonrpc_request(
        method,
        object.get("params"),
        session,
        configured_service_state,
    ) {
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
    configured_service_state: &ServiceState,
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
            read_service_mcp_resource_contents_with_config(uri, configured_service_state)
                .map_err(|err| resource_read_error(uri, err))
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
    let tools = vec![
        json!({
            "name": "service_request",
            "title": "Queue service browser request",
            "description": "Queue one intent-based browser request through the service control plane. Use serviceName, agentName, taskName, siteId or loginId, action, and params so agent-browser can select the managed profile and dispatch through the browser queue.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": BROWSER_COMMAND_ALLOWED_ACTIONS,
                        "description": "Underlying browser-control action to queue."
                    },
                    "params": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "Action parameters. These are copied into the queued daemon command after id/action are reserved."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued service request."
                    },
                    "profileLeasePolicy": {
                        "type": "string",
                        "enum": ["reject", "wait"],
                        "description": "How service-scoped launches handle active exclusive leases on the selected profile. reject fails before browser start; wait polls until the lease releases or profileLeaseWaitTimeoutMs elapses."
                    },
                    "profileLeaseWaitTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum time to wait for profileLeasePolicy=wait before failing the request."
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
                    },
                    "targetServiceId": {
                        "type": "string",
                        "description": "Target site or identity provider for profile selection."
                    },
                    "targetService": {
                        "type": "string",
                        "description": "Alias for targetServiceId."
                    },
                    "targetServiceIds": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Target sites or identity providers for profile selection."
                    },
                    "targetServices": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Alias for targetServiceIds."
                    },
                    "siteId": {
                        "type": "string",
                        "description": "Site identifier alias for targetServiceId."
                    },
                    "siteIds": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Site identifier aliases for targetServiceIds."
                    },
                    "loginId": {
                        "type": "string",
                        "description": "Login identity alias for targetServiceId."
                    },
                    "loginIds": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Login identity aliases for targetServiceIds."
                    }
                },
                "required": ["action"]
            }
        }),
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
            "name": "service_browser_retry",
            "title": "Retry faulted service browser",
            "description": "Enable one new recovery attempt for a faulted service browser. This is an explicit operator override and should include serviceName, agentName, and taskName when available.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "browserId": {
                        "type": "string",
                        "description": "Service browser id to make retryable."
                    },
                    "by": {
                        "type": "string",
                        "description": "Operator or automation identity approving the retry."
                    },
                    "note": {
                        "type": "string",
                        "description": "Optional operator note explaining why retry is safe."
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
                "required": ["browserId"]
            }
        }),
        json!({
            "name": "service_incidents",
            "title": "Read service incidents",
            "description": "Read grouped retained service incidents with the same filters as the HTTP and CLI service incidents surfaces, including severity and escalation.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "incidentId": {
                        "type": "string",
                        "description": "Read one incident by id and include related retained events and jobs."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum incident records to return. Defaults to 20."
                    },
                    "summary": {
                        "type": "boolean",
                        "description": "When true, include summary groups by escalation, severity, and state with recommended next actions."
                    },
                    "remediesOnly": {
                        "type": "boolean",
                        "description": "When true, return only active browser_degraded and os_degraded_possible remedy groups for operator triage."
                    },
                    "state": {
                        "type": "string",
                        "enum": ["active", "recovered", "service"],
                        "description": "Filter by incident lifecycle state."
                    },
                    "severity": {
                        "type": "string",
                        "enum": ["info", "warning", "error", "critical"],
                        "description": "Filter by service-derived incident severity."
                    },
                    "escalation": {
                        "type": "string",
                        "enum": ["none", "browser_degraded", "browser_recovery", "job_attention", "service_triage", "os_degraded_possible"],
                        "description": "Filter by service-derived escalation bucket."
                    },
                    "handlingState": {
                        "type": "string",
                        "enum": ["unacknowledged", "acknowledged", "resolved"],
                        "description": "Filter by operator handling state."
                    },
                    "kind": {
                        "type": "string",
                        "enum": ["browser_health_changed", "reconciliation_error", "service_job_timeout", "service_job_cancelled"],
                        "description": "Filter by latest incident kind."
                    },
                    "browserId": {
                        "type": "string",
                        "description": "Filter by browser id."
                    },
                    "profileId": {
                        "type": "string",
                        "description": "Filter incidents by related profile id."
                    },
                    "sessionId": {
                        "type": "string",
                        "description": "Filter incidents by related session id."
                    },
                    "serviceName": {
                        "type": "string",
                        "description": "Filter incidents by related service name."
                    },
                    "agentName": {
                        "type": "string",
                        "description": "Filter incidents by related agent name."
                    },
                    "taskName": {
                        "type": "string",
                        "description": "Filter incidents by related task name."
                    },
                    "since": {
                        "type": "string",
                        "description": "RFC 3339 timestamp. Only incidents with latestTimestamp at or after this time are returned."
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "service_profile_upsert",
            "title": "Upsert service profile",
            "description": "Persist one service profile record into service state. The id argument is authoritative and must match profile.id when the nested object includes an id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Profile id, for example journal-downloader."
                    },
                    "profile": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "Full profile object using ServiceState camelCase field names."
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
                "required": ["id", "profile"]
            }
        }),
        json!({
            "name": "service_profile_delete",
            "title": "Delete service profile",
            "description": "Delete one persisted service profile record by id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Profile id to delete."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_profile_freshness_update",
            "title": "Update service profile freshness",
            "description": "Merge bounded-probe target readiness evidence into an existing service profile through the queued service config path.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Profile id to update."
                    },
                    "loginId": {
                        "type": "string",
                        "description": "Login identity whose freshness was probed."
                    },
                    "siteId": {
                        "type": "string",
                        "description": "Site identity whose freshness was probed."
                    },
                    "targetServiceId": {
                        "type": "string",
                        "description": "Target service or identity provider whose freshness was probed."
                    },
                    "targetServiceIds": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Additional target identities to update."
                    },
                    "readinessState": {
                        "type": "string",
                        "enum": ["unknown", "needs_manual_seeding", "seeded_unknown_freshness", "fresh", "stale", "blocked_by_attached_devtools"],
                        "description": "New readiness state for the target identities. Defaults to fresh."
                    },
                    "readinessEvidence": {
                        "type": "string",
                        "description": "Evidence code from the bounded probe."
                    },
                    "readinessRecommendedAction": {
                        "type": "string",
                        "description": "Optional recommended action override."
                    },
                    "lastVerifiedAt": {
                        "type": "string",
                        "description": "Optional RFC 3339 probe timestamp. Defaults to server time."
                    },
                    "freshnessExpiresAt": {
                        "type": "string",
                        "description": "Optional RFC 3339 freshness expiry timestamp."
                    },
                    "updateAuthenticatedServiceIds": {
                        "type": "boolean",
                        "description": "Whether to add fresh targets to authenticatedServiceIds and remove stale or blocked targets. Defaults to true."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_session_upsert",
            "title": "Upsert service session",
            "description": "Persist one service session record into service state. The id argument is authoritative and must match session.id when the nested object includes an id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Session id, for example journal-run."
                    },
                    "session": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "Full session object using ServiceState camelCase field names."
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
                "required": ["id", "session"]
            }
        }),
        json!({
            "name": "service_session_delete",
            "title": "Delete service session",
            "description": "Delete one persisted service session record by id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Session id to delete."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_site_policy_upsert",
            "title": "Upsert service site policy",
            "description": "Persist one service site-policy record into service state. The id argument is authoritative and must match sitePolicy.id when the nested object includes an id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Site policy id, for example google."
                    },
                    "sitePolicy": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "Full site policy object using ServiceState camelCase field names."
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
                "required": ["id", "sitePolicy"]
            }
        }),
        json!({
            "name": "service_site_policy_delete",
            "title": "Delete service site policy",
            "description": "Delete one persisted service site-policy record by id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Site policy id to delete."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_monitor_upsert",
            "title": "Upsert service monitor",
            "description": "Persist one service monitor record into service state. The id argument is authoritative and must match monitor.id when the nested object includes an id. This stores monitor definitions only; probe scheduling is a later service loop.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Monitor id, for example google-login-freshness."
                    },
                    "monitor": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "Full monitor object using ServiceState camelCase field names."
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
                "required": ["id", "monitor"]
            }
        }),
        json!({
            "name": "service_monitor_delete",
            "title": "Delete service monitor",
            "description": "Delete one persisted service monitor record by id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Monitor id to delete."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_monitors_run_due",
            "title": "Run due service monitors",
            "description": "Run due active service monitors immediately through the service worker.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
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
                }
            }
        }),
        json!({
            "name": "service_monitor_pause",
            "title": "Pause service monitor",
            "description": "Pause one noisy service monitor while preserving retained health history.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Monitor id to pause."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_monitor_resume",
            "title": "Resume service monitor",
            "description": "Resume one paused or faulted service monitor while preserving retained health history.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Monitor id to resume."
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
                "required": ["id"]
            }
        }),
        json!({
            "name": "service_provider_upsert",
            "title": "Upsert service provider",
            "description": "Persist one service provider record into service state. The id argument is authoritative and must match provider.id when the nested object includes an id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Provider id, for example manual."
                    },
                    "provider": {
                        "type": "object",
                        "additionalProperties": true,
                        "description": "Full provider object using ServiceState camelCase field names."
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
                "required": ["id", "provider"]
            }
        }),
        json!({
            "name": "service_provider_delete",
            "title": "Delete service provider",
            "description": "Delete one persisted service provider record by id.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Provider id to delete."
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
                "required": ["id"]
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
        browser_read_tool_schema(BROWSER_GET_URL_TOOL),
        browser_read_tool_schema(BROWSER_GET_TITLE_TOOL),
        json!({
            "name": "browser_tabs",
            "title": "List browser tabs",
            "description": "Queue a read of the active browser session tabs. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "verbose": {
                        "type": "boolean",
                        "description": "Include targetId and sessionId for each tab."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued tabs read job."
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
            "name": "browser_screenshot",
            "title": "Take browser screenshot",
            "description": "Queue a screenshot against the active browser session and return the saved image path. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Optional CSS selector or cached ref to scope the screenshot."
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional output path. If omitted, agent-browser saves to the configured screenshot directory."
                    },
                    "fullPage": {
                        "type": "boolean",
                        "description": "Capture the full scrollable page."
                    },
                    "annotate": {
                        "type": "boolean",
                        "description": "Overlay numbered labels for interactive elements and return annotation metadata."
                    },
                    "format": {
                        "type": "string",
                        "enum": ["png", "jpeg"],
                        "description": "Screenshot format. Defaults to png."
                    },
                    "quality": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "description": "JPEG quality from 0 to 100. Only applies when format is jpeg."
                    },
                    "screenshotDir": {
                        "type": "string",
                        "description": "Optional output directory when path is omitted."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued screenshot job."
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
            "name": "browser_click",
            "title": "Click browser element",
            "description": "Queue a click against a selector or cached ref in the active browser session. This mutates page state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref such as @e1."
                    },
                    "newTab": {
                        "type": "boolean",
                        "description": "Open a link target in a new tab instead of clicking in-place."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued click job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_fill",
            "title": "Fill browser field",
            "description": "Queue a fill against a selector or cached ref in the active browser session. This mutates page state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref such as @e1."
                    },
                    "value": {
                        "type": "string",
                        "description": "Required text value to fill into the target field."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued fill job."
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
                "required": ["selector", "value"]
            }
        }),
        json!({
            "name": "browser_wait",
            "title": "Wait for browser state",
            "description": "Queue a bounded wait against the active browser session. Choose one of selector, text, url, function, loadState, or provide timeoutMs alone for a fixed wait. Include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Optional CSS selector or cached ref to wait for."
                    },
                    "state": {
                        "type": "string",
                        "enum": ["visible", "hidden", "attached", "detached"],
                        "description": "Selector wait state. Defaults to visible."
                    },
                    "text": {
                        "type": "string",
                        "description": "Optional page text substring to wait for."
                    },
                    "url": {
                        "type": "string",
                        "description": "Optional URL glob pattern to wait for."
                    },
                    "function": {
                        "type": "string",
                        "description": "Optional JavaScript expression to wait until truthy."
                    },
                    "loadState": {
                        "type": "string",
                        "enum": ["load", "domcontentloaded", "networkidle", "none"],
                        "description": "Optional load state to wait for."
                    },
                    "timeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Condition timeout in milliseconds, or fixed wait duration when no condition is provided."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued wait job."
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
            "name": "browser_type",
            "title": "Type into browser field",
            "description": "Queue typed text against a selector or cached ref in the active browser session. This mutates page state using keyboard-style input, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref such as @e1."
                    },
                    "text": {
                        "type": "string",
                        "description": "Required text to type into the target field."
                    },
                    "clear": {
                        "type": "boolean",
                        "description": "Clear the field before typing. Defaults to false."
                    },
                    "delayMs": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Optional delay in milliseconds between key events."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued type job."
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
                "required": ["selector", "text"]
            }
        }),
        json!({
            "name": "browser_press",
            "title": "Press browser key",
            "description": "Queue a key press in the active browser session. Supports key names and chords such as Enter, Tab, Escape, Control+a, or Shift+Enter. This mutates page state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "Required key name or modifier chord, for example Enter, Tab, Escape, Control+a, or Shift+Enter."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued press job."
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
                "required": ["key"]
            }
        }),
        json!({
            "name": "browser_hover",
            "title": "Hover browser element",
            "description": "Queue a hover against a selector or cached ref in the active browser session. This can reveal menus or other hover-triggered controls, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref such as @e1."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued hover job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_select",
            "title": "Select browser option",
            "description": "Queue selection of one or more values in a select control in the active browser session. This mutates page state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the select control."
                    },
                    "values": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "description": "Required option value or values to select."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued select job."
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
                "required": ["selector", "values"]
            }
        }),
        browser_element_read_tool_schema(
            "browser_get_text",
            "Read browser element text",
            "Queue reading an element's visible text in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            false,
        ),
        browser_element_read_tool_schema(
            "browser_get_value",
            "Read browser field value",
            "Queue reading an input, textarea, or select value in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            false,
        ),
        browser_element_read_tool_schema(
            "browser_get_attribute",
            "Read browser element attribute",
            "Queue reading an element attribute in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            true,
        ),
        browser_element_read_tool_schema(
            "browser_get_html",
            "Read browser element HTML",
            "Queue reading an element's inner HTML in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            false,
        ),
        browser_get_styles_tool_schema(),
        browser_element_read_tool_schema(
            "browser_count",
            "Count browser elements",
            "Queue counting elements matching a CSS selector in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            false,
        ),
        browser_element_read_tool_schema(
            "browser_get_box",
            "Read browser element box",
            "Queue reading an element bounding box in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            false,
        ),
        browser_element_state_tool_schema(
            "browser_is_visible",
            "Read browser visibility state",
            "Queue reading whether an element is visible in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            "Required CSS selector or cached ref for the element to inspect.",
            "Optional worker-bound timeout for this queued visibility read job.",
        ),
        browser_element_state_tool_schema(
            "browser_is_enabled",
            "Read browser enabled state",
            "Queue reading whether an element is enabled in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            "Required CSS selector or cached ref for the element to inspect.",
            "Optional worker-bound timeout for this queued enabled-state read job.",
        ),
        json!({
            "name": "browser_check",
            "title": "Check browser control",
            "description": "Queue checking a checkbox or radio control in the active browser session. This mutates page state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the checkbox or radio control."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued check job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_is_checked",
            "title": "Read browser checked state",
            "description": "Queue reading whether a checkbox, radio, or ARIA checked control is checked in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the checkbox, radio, or ARIA checked control."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued checked-state read job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_uncheck",
            "title": "Uncheck browser control",
            "description": "Queue unchecking a checkbox control in the active browser session. This mutates page state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the checkbox control."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued uncheck job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_scroll",
            "title": "Scroll browser page",
            "description": "Queue page or container scrolling in the active browser session. Use direction plus amount, or explicit deltaX and deltaY. This mutates viewport state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Optional CSS selector or cached ref for a scrollable container. Omit to scroll the page."
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right"],
                        "description": "Optional scroll direction. Defaults to down when no deltaX or deltaY is supplied."
                    },
                    "amount": {
                        "type": "number",
                        "minimum": 0,
                        "description": "Optional pixels to scroll with direction. Defaults to 300."
                    },
                    "deltaX": {
                        "type": "number",
                        "description": "Optional horizontal scroll delta in pixels. Use instead of direction."
                    },
                    "deltaY": {
                        "type": "number",
                        "description": "Optional vertical scroll delta in pixels. Use instead of direction."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued scroll job."
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
            "name": "browser_scroll_into_view",
            "title": "Scroll browser element into view",
            "description": "Queue scrolling an element into view in the active browser session. This mutates viewport state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the element to scroll into view."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued scroll-into-view job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_focus",
            "title": "Focus browser element",
            "description": "Queue focusing an element in the active browser session. This prepares keyboard-driven interaction, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the element to focus."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued focus job."
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
                "required": ["selector"]
            }
        }),
        json!({
            "name": "browser_clear",
            "title": "Clear browser field",
            "description": "Queue clearing an input or editable field in the active browser session. This mutates field state, so include serviceName, agentName, and taskName when available for traceability.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "Required CSS selector or cached ref for the field to clear."
                    },
                    "jobTimeoutMs": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional worker-bound timeout for this queued clear job."
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
                "required": ["selector"]
            }
        }),
        browser_navigate_tool_schema(),
        browser_requests_tool_schema(),
        browser_request_detail_tool_schema(),
        browser_headers_tool_schema(),
        browser_offline_tool_schema(),
        browser_cookies_get_tool_schema(),
        browser_cookies_set_tool_schema(),
        browser_cookies_clear_tool_schema(),
        browser_storage_get_tool_schema(),
        browser_storage_set_tool_schema(),
        browser_storage_clear_tool_schema(),
        browser_user_agent_tool_schema(),
        browser_viewport_tool_schema(),
        browser_geolocation_tool_schema(),
        browser_permissions_tool_schema(),
        browser_timezone_tool_schema(),
        browser_locale_tool_schema(),
        browser_media_tool_schema(),
        browser_dialog_tool_schema(),
        browser_upload_tool_schema(),
        browser_download_tool_schema(),
        browser_wait_for_download_tool_schema(),
        browser_har_start_tool_schema(),
        browser_har_stop_tool_schema(),
        browser_route_tool_schema(),
        browser_unroute_tool_schema(),
        browser_console_tool_schema(),
        browser_errors_tool_schema(),
        browser_pdf_tool_schema(),
        browser_response_body_tool_schema(),
        browser_clipboard_tool_schema(),
        browser_simple_action_tool_schema(
            "browser_back",
            "Go back in browser history",
            "Queue a browser history back operation against the active session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "Optional worker-bound timeout for this queued back-navigation job.",
        ),
        browser_simple_action_tool_schema(
            "browser_forward",
            "Go forward in browser history",
            "Queue a browser history forward operation against the active session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "Optional worker-bound timeout for this queued forward-navigation job.",
        ),
        browser_simple_action_tool_schema(
            "browser_reload",
            "Reload browser page",
            "Queue a page reload against the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
            "Optional worker-bound timeout for this queued reload job.",
        ),
        browser_tab_new_tool_schema(),
        browser_tab_switch_tool_schema(),
        browser_tab_close_tool_schema(),
        browser_set_content_tool_schema(),
        browser_command_tool_schema(),
        json!({
            "name": "service_trace",
            "title": "Read service trace",
            "description": "Read related service events, jobs, incidents, and normalized activity from persisted service state in one response. Use serviceName, agentName, and taskName to debug one service/task trace without shelling out.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum records to return per trace section. Defaults to 20."
                    },
                    "browserId": {
                        "type": "string",
                        "description": "Filter trace records by browser id."
                    },
                    "profileId": {
                        "type": "string",
                        "description": "Filter trace records by profile id."
                    },
                    "sessionId": {
                        "type": "string",
                        "description": "Filter trace records by session id."
                    },
                    "serviceName": {
                        "type": "string",
                        "description": "Filter trace records by service name, for example JournalDownloader."
                    },
                    "agentName": {
                        "type": "string",
                        "description": "Filter trace records by agent name."
                    },
                    "taskName": {
                        "type": "string",
                        "description": "Filter trace records by task name, for example probeACSwebsite."
                    },
                    "since": {
                        "type": "string",
                        "description": "RFC 3339 timestamp. Only records at or after this time are returned."
                    }
                },
                "required": []
            }
        }),
    ];
    tools
        .into_iter()
        .map(with_browser_target_profile_hint_properties)
        .collect()
}

fn with_browser_target_profile_hint_properties(mut tool: Value) -> Value {
    let is_browser_tool = tool
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.starts_with("browser_"));
    if !is_browser_tool {
        return tool;
    }
    let Some(properties) = tool
        .get_mut("inputSchema")
        .and_then(|schema| schema.get_mut("properties"))
        .and_then(Value::as_object_mut)
    else {
        return tool;
    };
    if !properties.contains_key("serviceName") {
        return tool;
    }

    properties.insert(
        "profileLeasePolicy".to_string(),
        json!({
            "type": "string",
            "enum": ["reject", "wait"],
            "description": "How service-scoped launches handle active exclusive leases on the selected profile."
        }),
    );
    properties.insert(
        "profileLeaseWaitTimeoutMs".to_string(),
        json!({
            "type": "integer",
            "minimum": 1,
            "description": "Maximum time to wait for profileLeasePolicy=wait before failing the request."
        }),
    );
    properties.insert(
        "targetServiceId".to_string(),
        json!({
            "type": "string",
            "description": "Target site or identity provider for profile selection, for example google, microsoft, or acs."
        }),
    );
    properties.insert(
        "targetService".to_string(),
        json!({
            "type": "string",
            "description": "Alias for targetServiceId."
        }),
    );
    properties.insert(
        "targetServiceIds".to_string(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Target sites or identity providers for profile selection."
        }),
    );
    properties.insert(
        "targetServices".to_string(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Alias for targetServiceIds."
        }),
    );
    properties.insert(
        "siteId".to_string(),
        json!({
            "type": "string",
            "description": "Site identifier alias for targetServiceId, for example google, microsoft, or acs."
        }),
    );
    properties.insert(
        "loginId".to_string(),
        json!({
            "type": "string",
            "description": "Login identity alias for targetServiceId when the caller thinks in credential or SSO scope."
        }),
    );
    properties.insert(
        "siteIds".to_string(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Site identifier aliases for targetServiceIds."
        }),
    );
    properties.insert(
        "loginIds".to_string(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Login identity aliases for targetServiceIds."
        }),
    );
    tool
}

#[derive(Clone, Copy)]
struct BrowserReadToolSpec {
    tool_name: &'static str,
    title: &'static str,
    action: &'static str,
    state_name: &'static str,
    id_prefix: &'static str,
}

const BROWSER_GET_URL_TOOL: BrowserReadToolSpec = BrowserReadToolSpec {
    tool_name: "browser_get_url",
    title: "Get browser URL",
    action: "url",
    state_name: "URL",
    id_prefix: "mcp-browser-get-url",
};

const BROWSER_GET_TITLE_TOOL: BrowserReadToolSpec = BrowserReadToolSpec {
    tool_name: "browser_get_title",
    title: "Get browser title",
    action: "title",
    state_name: "title",
    id_prefix: "mcp-browser-get-title",
};

fn browser_navigate_tool_schema() -> Value {
    json!({
        "name": "browser_navigate",
        "title": "Navigate browser",
        "description": "Queue navigation in the active browser session. Use this typed tool instead of browser_command for ordinary page navigation. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Required URL to navigate the active browser session to."
                },
                "waitUntil": {
                    "type": "string",
                    "enum": ["load", "domcontentloaded", "networkidle", "none"],
                    "description": "Optional navigation wait condition. Defaults to load."
                },
                "headers": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Optional origin-scoped extra HTTP headers for this navigation."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued navigation job."
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
            "required": ["url"]
        }
    })
}

fn browser_requests_tool_schema() -> Value {
    json!({
        "name": "browser_requests",
        "title": "Inspect browser network requests",
        "description": "Queue network request inspection for the active browser session. The first call enables request tracking; later calls return tracked requests. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "clear": {
                    "type": "boolean",
                    "description": "Clear retained in-memory request tracking for the active browser session."
                },
                "filter": {
                    "type": "string",
                    "description": "Optional substring filter applied to request URLs."
                },
                "type": {
                    "type": "string",
                    "description": "Optional comma-separated resource type filter, for example document,xhr,fetch."
                },
                "method": {
                    "type": "string",
                    "description": "Optional HTTP method filter, for example GET or POST."
                },
                "status": {
                    "type": "string",
                    "description": "Optional status filter, for example 2xx, 404, or 200-299."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued request-inspection job."
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
    })
}

fn browser_request_detail_tool_schema() -> Value {
    json!({
        "name": "browser_request_detail",
        "title": "Inspect one browser network request",
        "description": "Queue detailed inspection for one tracked browser network request. Call browser_requests first to enable tracking and discover requestId values. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "requestId": {
                    "type": "string",
                    "description": "Required tracked request id returned by browser_requests."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued request-detail job."
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
            "required": ["requestId"]
        }
    })
}

fn browser_headers_tool_schema() -> Value {
    json!({
        "name": "browser_headers",
        "title": "Set browser extra HTTP headers",
        "description": "Queue setting extra HTTP headers for the active browser session. Use this for session-shaping and site-specific policy before navigation. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "headers": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Required map of HTTP header names to string values."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued headers job."
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
            "required": ["headers"]
        }
    })
}

fn browser_offline_tool_schema() -> Value {
    json!({
        "name": "browser_offline",
        "title": "Set browser offline mode",
        "description": "Queue changing network offline emulation for the active browser session. Pass offline false to restore connectivity after a test. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "offline": {
                    "type": "boolean",
                    "description": "Whether to emulate offline network state. Defaults to true when omitted."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued offline-mode job."
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
    })
}

fn browser_cookies_get_tool_schema() -> Value {
    json!({
        "name": "browser_cookies_get",
        "title": "Get browser cookies",
        "description": "Queue reading browser cookies from the active browser session. Optionally pass urls to scope cookies by URL. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional non-empty list of URLs whose cookies should be returned."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued cookie read job."
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
    })
}

fn browser_cookies_set_tool_schema() -> Value {
    json!({
        "name": "browser_cookies_set",
        "title": "Set browser cookies",
        "description": "Queue setting one or more cookies in the active browser session. Pass cookies for bulk set, or pass name and value for a single cookie. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "cookies": {
                    "type": "array",
                    "items": { "type": "object" },
                    "description": "Optional non-empty list of cookie objects accepted by the daemon."
                },
                "name": {
                    "type": "string",
                    "description": "Cookie name when setting one cookie."
                },
                "value": {
                    "type": "string",
                    "description": "Cookie value when setting one cookie."
                },
                "url": {
                    "type": "string",
                    "description": "Optional cookie URL."
                },
                "domain": {
                    "type": "string",
                    "description": "Optional cookie domain."
                },
                "path": {
                    "type": "string",
                    "description": "Optional cookie path."
                },
                "expires": {
                    "type": "number",
                    "description": "Optional cookie expiration timestamp."
                },
                "httpOnly": {
                    "type": "boolean",
                    "description": "Whether the cookie is HTTP-only."
                },
                "secure": {
                    "type": "boolean",
                    "description": "Whether the cookie is secure."
                },
                "sameSite": {
                    "type": "string",
                    "description": "Optional SameSite policy."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued cookie set job."
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
    })
}

fn browser_cookies_clear_tool_schema() -> Value {
    json!({
        "name": "browser_cookies_clear",
        "title": "Clear browser cookies",
        "description": "Queue clearing browser cookies in the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued cookie clear job."
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
    })
}

fn browser_storage_get_tool_schema() -> Value {
    json!({
        "name": "browser_storage_get",
        "title": "Get browser storage",
        "description": "Queue reading localStorage or sessionStorage from the active browser session. Defaults to local storage when type is omitted. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "type": {
                    "type": "string",
                    "enum": ["local", "session"],
                    "description": "Storage bucket to read. Defaults to local."
                },
                "key": {
                    "type": "string",
                    "description": "Optional storage key. When omitted, all entries are returned."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued storage read job."
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
    })
}

fn browser_storage_set_tool_schema() -> Value {
    json!({
        "name": "browser_storage_set",
        "title": "Set browser storage",
        "description": "Queue setting a localStorage or sessionStorage entry in the active browser session. Defaults to local storage when type is omitted. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "type": {
                    "type": "string",
                    "enum": ["local", "session"],
                    "description": "Storage bucket to write. Defaults to local."
                },
                "key": {
                    "type": "string",
                    "description": "Required storage key."
                },
                "value": {
                    "type": "string",
                    "description": "Required storage value."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued storage set job."
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
            "required": ["key", "value"]
        }
    })
}

fn browser_storage_clear_tool_schema() -> Value {
    json!({
        "name": "browser_storage_clear",
        "title": "Clear browser storage",
        "description": "Queue clearing localStorage or sessionStorage in the active browser session. Defaults to local storage when type is omitted. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "type": {
                    "type": "string",
                    "enum": ["local", "session"],
                    "description": "Storage bucket to clear. Defaults to local."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued storage clear job."
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
    })
}

fn browser_user_agent_tool_schema() -> Value {
    json!({
        "name": "browser_user_agent",
        "title": "Set browser user agent",
        "description": "Queue setting the active browser session user agent. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "userAgent": {
                    "type": "string",
                    "description": "Required user agent string."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued user-agent job."
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
            "required": ["userAgent"]
        }
    })
}

fn browser_viewport_tool_schema() -> Value {
    json!({
        "name": "browser_viewport",
        "title": "Set browser viewport",
        "description": "Queue changing the active browser session viewport and stream viewport metadata. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "width": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Required viewport width in CSS pixels."
                },
                "height": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Required viewport height in CSS pixels."
                },
                "deviceScaleFactor": {
                    "type": "number",
                    "exclusiveMinimum": 0,
                    "description": "Optional device scale factor. Defaults to 1."
                },
                "mobile": {
                    "type": "boolean",
                    "description": "Whether to emulate a mobile viewport. Defaults to false."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued viewport job."
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
            "required": ["width", "height"]
        }
    })
}

fn browser_geolocation_tool_schema() -> Value {
    json!({
        "name": "browser_geolocation",
        "title": "Set browser geolocation",
        "description": "Queue setting geolocation emulation for the active browser session. Grant geolocation permission separately when the site requires it. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "latitude": {
                    "type": "number",
                    "description": "Required latitude."
                },
                "longitude": {
                    "type": "number",
                    "description": "Required longitude."
                },
                "accuracy": {
                    "type": "number",
                    "minimum": 0,
                    "description": "Optional accuracy in meters."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued geolocation job."
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
            "required": ["latitude", "longitude"]
        }
    })
}

fn browser_permissions_tool_schema() -> Value {
    json!({
        "name": "browser_permissions",
        "title": "Grant browser permissions",
        "description": "Queue granting browser permissions for the active browser session, for example geolocation. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "permissions": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "description": "Required non-empty list of permission names to grant."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued permissions job."
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
            "required": ["permissions"]
        }
    })
}

fn browser_timezone_tool_schema() -> Value {
    json!({
        "name": "browser_timezone",
        "title": "Set browser timezone",
        "description": "Queue timezone emulation for the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "timezoneId": {
                    "type": "string",
                    "description": "Required IANA timezone id, for example America/Chicago."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued timezone job."
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
            "required": ["timezoneId"]
        }
    })
}

fn browser_locale_tool_schema() -> Value {
    json!({
        "name": "browser_locale",
        "title": "Set browser locale",
        "description": "Queue locale emulation for the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "locale": {
                    "type": "string",
                    "description": "Required locale tag, for example en-US."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued locale job."
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
            "required": ["locale"]
        }
    })
}

fn browser_media_tool_schema() -> Value {
    json!({
        "name": "browser_media",
        "title": "Set browser media emulation",
        "description": "Queue CSS media and media-feature emulation for the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "media": {
                    "type": "string",
                    "description": "Optional emulated media type, for example screen, print, or no-override."
                },
                "colorScheme": {
                    "type": "string",
                    "description": "Optional prefers-color-scheme value, for example light or dark."
                },
                "reducedMotion": {
                    "type": "string",
                    "description": "Optional prefers-reduced-motion value, for example reduce or no-preference."
                },
                "features": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Optional map of media feature names to emulated string values."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued media-emulation job."
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
    })
}

fn browser_dialog_tool_schema() -> Value {
    json!({
        "name": "browser_dialog",
        "title": "Handle browser dialog",
        "description": "Queue dialog status or response handling for the active browser session. Use status before deciding whether to accept or dismiss. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "response": {
                    "type": "string",
                    "enum": ["status", "accept", "dismiss"],
                    "description": "Required dialog action."
                },
                "promptText": {
                    "type": "string",
                    "description": "Optional prompt text when accepting a prompt dialog."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued dialog job."
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
            "required": ["response"]
        }
    })
}

fn browser_upload_tool_schema() -> Value {
    json!({
        "name": "browser_upload",
        "title": "Upload files",
        "description": "Queue setting files on an input element in the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "Required CSS selector or cached ref for the file input."
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "description": "Required non-empty list of file paths to upload."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued upload job."
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
            "required": ["selector", "files"]
        }
    })
}

fn browser_download_tool_schema() -> Value {
    json!({
        "name": "browser_download",
        "title": "Download by clicking",
        "description": "Queue clicking an element that triggers a browser download and save it to the requested path. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "Required CSS selector or cached ref for the element that triggers the download."
                },
                "path": {
                    "type": "string",
                    "description": "Required destination path for the downloaded file."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued download job."
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
            "required": ["selector", "path"]
        }
    })
}

fn browser_wait_for_download_tool_schema() -> Value {
    json!({
        "name": "browser_wait_for_download",
        "title": "Wait for browser download",
        "description": "Queue waiting for a browser download event or a destination path change. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Optional expected download path to watch for changes."
                },
                "timeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional daemon-side download wait timeout."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued download-wait job."
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
    })
}

fn browser_har_start_tool_schema() -> Value {
    json!({
        "name": "browser_har_start",
        "title": "Start HAR capture",
        "description": "Queue starting HAR capture for the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued HAR start job."
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
    })
}

fn browser_har_stop_tool_schema() -> Value {
    json!({
        "name": "browser_har_stop",
        "title": "Stop HAR capture",
        "description": "Queue stopping HAR capture and writing the HAR file. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Optional destination path for the HAR file."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued HAR stop job."
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
    })
}

fn browser_route_tool_schema() -> Value {
    json!({
        "name": "browser_route",
        "title": "Route browser requests",
        "description": "Queue adding a request route for the active browser session. Use abort to block matches or response to fulfill matches. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Required URL pattern to intercept, for example **/api/*."
                },
                "abort": {
                    "type": "boolean",
                    "description": "Whether matching requests should be aborted."
                },
                "response": {
                    "type": "object",
                    "additionalProperties": true,
                    "description": "Optional response object with status, body, contentType, and headers fields."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued route job."
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
            "required": ["url"]
        }
    })
}

fn browser_unroute_tool_schema() -> Value {
    json!({
        "name": "browser_unroute",
        "title": "Remove browser request routes",
        "description": "Queue removing one request route pattern, or all routes when url is omitted. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Optional URL pattern to remove. Omit to remove all routes."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued unroute job."
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
    })
}

fn browser_console_tool_schema() -> Value {
    json!({
        "name": "browser_console",
        "title": "Read browser console",
        "description": "Queue reading or clearing retained browser console messages. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "clear": {
                    "type": "boolean",
                    "description": "Whether to clear retained console messages instead of reading them."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued console job."
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
    })
}

fn browser_errors_tool_schema() -> Value {
    json!({
        "name": "browser_errors",
        "title": "Read browser errors",
        "description": "Queue reading retained browser page errors. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued errors job."
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
    })
}

fn browser_pdf_tool_schema() -> Value {
    json!({
        "name": "browser_pdf",
        "title": "Save browser PDF",
        "description": "Queue printing the active browser page to PDF. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Optional destination path. When omitted the daemon writes to its temporary PDF directory."
                },
                "printBackground": {
                    "type": "boolean",
                    "description": "Whether to print background graphics. Defaults to true."
                },
                "landscape": {
                    "type": "boolean",
                    "description": "Whether to print in landscape orientation. Defaults to false."
                },
                "preferCSSPageSize": {
                    "type": "boolean",
                    "description": "Whether to prefer CSS page size. Defaults to false."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued PDF job."
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
    })
}

fn browser_response_body_tool_schema() -> Value {
    json!({
        "name": "browser_response_body",
        "title": "Read browser response body",
        "description": "Queue waiting for a network response whose URL contains the requested substring, then read its response body. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Required URL substring to match."
                },
                "timeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional daemon-side wait timeout."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued response-body job."
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
            "required": ["url"]
        }
    })
}

fn browser_clipboard_tool_schema() -> Value {
    json!({
        "name": "browser_clipboard",
        "title": "Use browser clipboard",
        "description": "Queue browser clipboard read, write, copy, or paste for the active session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "copy", "paste"],
                    "description": "Clipboard operation. Defaults to read."
                },
                "text": {
                    "type": "string",
                    "description": "Required text for write operations."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued clipboard job."
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
    })
}

fn browser_simple_action_tool_schema(
    name: &str,
    title: &str,
    description: &str,
    timeout_description: &str,
) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": timeout_description
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
    })
}

fn browser_tab_new_tool_schema() -> Value {
    json!({
        "name": "browser_tab_new",
        "title": "Open browser tab",
        "description": "Queue opening a new tab in the active browser session, optionally navigating it to a URL. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Optional URL to load in the new tab."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued new-tab job."
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
    })
}

fn browser_tab_switch_tool_schema() -> Value {
    json!({
        "name": "browser_tab_switch",
        "title": "Switch browser tab",
        "description": "Queue switching the active browser session to a tab by zero-based index. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "index": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Required zero-based tab index."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued tab-switch job."
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
            "required": ["index"]
        }
    })
}

fn browser_tab_close_tool_schema() -> Value {
    json!({
        "name": "browser_tab_close",
        "title": "Close browser tab",
        "description": "Queue closing the current tab or a tab by zero-based index in the active browser session. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "index": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional zero-based tab index. Omit to close the current tab."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued tab-close job."
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
    })
}

fn browser_set_content_tool_schema() -> Value {
    json!({
        "name": "browser_set_content",
        "title": "Set browser page content",
        "description": "Queue replacing the active page document with supplied HTML. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "html": {
                    "type": "string",
                    "description": "Required HTML document or fragment to install in the active page."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued set-content job."
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
            "required": ["html"]
        }
    })
}

fn browser_command_tool_schema() -> Value {
    json!({
        "name": "browser_command",
        "title": "Queue browser control command",
        "description": "Queue any supported browser-control daemon action against the active session. Use typed browser_* tools for common interactions; use this parity tool for advanced HTTP-equivalent controls that do not yet have a typed schema. Include serviceName, agentName, and taskName when available so the retained service job is traceable.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "action": {
                    "type": "string",
                    "enum": BROWSER_COMMAND_ALLOWED_ACTIONS,
                    "description": "Underlying browser-control action to queue."
                },
                "params": {
                    "type": "object",
                    "additionalProperties": true,
                    "description": "Action parameters. These are copied into the queued daemon command after id/action are reserved. For first-command browser launch selection, include targetServiceId, targetService, targetServiceIds, or targetServices here."
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional worker-bound timeout for this queued browser command."
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
            "required": ["action"]
        }
    })
}

fn browser_read_tool_schema(spec: BrowserReadToolSpec) -> Value {
    json!({
        "name": spec.tool_name,
        "title": spec.title,
        "description": format!("Queue a read of the active browser session {}. Include serviceName, agentName, and taskName when available so the retained service job is traceable.", spec.state_name),
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": format!("Optional worker-bound timeout for this queued {} read job.", spec.state_name)
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
    })
}

fn browser_element_read_tool_schema(
    tool_name: &str,
    title: &str,
    description: &str,
    requires_attribute: bool,
) -> Value {
    let mut properties = json!({
        "selector": {
            "type": "string",
            "description": "Required CSS selector or cached ref for the element to read."
        },
        "jobTimeoutMs": {
            "type": "integer",
            "minimum": 1,
            "description": "Optional worker-bound timeout for this queued element read job."
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
    });
    let required = if requires_attribute {
        properties["attribute"] = json!({
            "type": "string",
            "description": "Required attribute name to read from the target element."
        });
        json!(["selector", "attribute"])
    } else {
        json!(["selector"])
    };

    json!({
        "name": tool_name,
        "title": title,
        "description": description,
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": properties,
            "required": required
        }
    })
}

fn browser_get_styles_tool_schema() -> Value {
    let mut schema = browser_element_read_tool_schema(
        "browser_get_styles",
        "Read browser element styles",
        "Queue reading computed styles for an element in the active browser session. Include serviceName, agentName, and taskName when available for traceability.",
        false,
    );
    schema["inputSchema"]["properties"]["properties"] = json!({
        "type": "array",
        "items": { "type": "string" },
        "minItems": 1,
        "description": "Optional CSS property names to read. Omit to return all computed styles."
    });
    schema
}

fn browser_element_state_tool_schema(
    tool_name: &str,
    title: &str,
    description: &str,
    selector_description: &str,
    timeout_description: &str,
) -> Value {
    json!({
        "name": tool_name,
        "title": title,
        "description": description,
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "selector": {
                    "type": "string",
                    "description": selector_description
                },
                "jobTimeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": timeout_description
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
            "required": ["selector"]
        }
    })
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
        "service_browser_retry" => call_service_browser_retry(arguments, session),
        "service_incidents" => call_service_incidents(arguments, session),
        "service_trace" => call_service_trace(arguments, session),
        "service_profile_upsert" => call_service_profile_upsert(arguments, session),
        "service_profile_freshness_update" => {
            call_service_profile_freshness_update(arguments, session)
        }
        "service_profile_delete" => call_service_profile_delete(arguments, session),
        "service_session_upsert" => call_service_session_upsert(arguments, session),
        "service_session_delete" => call_service_session_delete(arguments, session),
        "service_site_policy_upsert" => call_service_site_policy_upsert(arguments, session),
        "service_site_policy_delete" => call_service_site_policy_delete(arguments, session),
        "service_monitor_upsert" => call_service_monitor_upsert(arguments, session),
        "service_monitor_delete" => call_service_monitor_delete(arguments, session),
        "service_monitors_run_due" => call_service_monitors_run_due(arguments, session),
        "service_monitor_pause" => {
            call_service_monitor_state(arguments, session, "service_monitor_pause")
        }
        "service_monitor_resume" => {
            call_service_monitor_state(arguments, session, "service_monitor_resume")
        }
        "service_provider_upsert" => call_service_provider_upsert(arguments, session),
        "service_provider_delete" => call_service_provider_delete(arguments, session),
        "service_request" => call_service_request(arguments, session),
        "browser_command" => call_browser_command(arguments, session),
        "browser_navigate" => call_browser_navigate(arguments, session),
        "browser_requests" => call_browser_requests(arguments, session),
        "browser_request_detail" => call_browser_request_detail(arguments, session),
        "browser_headers" => call_browser_headers(arguments, session),
        "browser_offline" => call_browser_offline(arguments, session),
        "browser_cookies_get" => call_browser_cookies_get(arguments, session),
        "browser_cookies_set" => call_browser_cookies_set(arguments, session),
        "browser_cookies_clear" => call_browser_cookies_clear(arguments, session),
        "browser_storage_get" => call_browser_storage_get(arguments, session),
        "browser_storage_set" => call_browser_storage_set(arguments, session),
        "browser_storage_clear" => call_browser_storage_clear(arguments, session),
        "browser_user_agent" => call_browser_user_agent(arguments, session),
        "browser_viewport" => call_browser_viewport(arguments, session),
        "browser_geolocation" => call_browser_geolocation(arguments, session),
        "browser_permissions" => call_browser_permissions(arguments, session),
        "browser_timezone" => call_browser_timezone(arguments, session),
        "browser_locale" => call_browser_locale(arguments, session),
        "browser_media" => call_browser_media(arguments, session),
        "browser_dialog" => call_browser_dialog(arguments, session),
        "browser_upload" => call_browser_upload(arguments, session),
        "browser_download" => call_browser_download(arguments, session),
        "browser_wait_for_download" => call_browser_wait_for_download(arguments, session),
        "browser_har_start" => call_browser_har_start(arguments, session),
        "browser_har_stop" => call_browser_har_stop(arguments, session),
        "browser_route" => call_browser_route(arguments, session),
        "browser_unroute" => call_browser_unroute(arguments, session),
        "browser_console" => call_browser_console(arguments, session),
        "browser_errors" => call_browser_errors(arguments, session),
        "browser_pdf" => call_browser_pdf(arguments, session),
        "browser_response_body" => call_browser_response_body(arguments, session),
        "browser_clipboard" => call_browser_clipboard(arguments, session),
        "browser_back" => call_browser_simple_action(
            arguments,
            session,
            "browser_back",
            "mcp-browser-back",
            "back",
        ),
        "browser_forward" => call_browser_simple_action(
            arguments,
            session,
            "browser_forward",
            "mcp-browser-forward",
            "forward",
        ),
        "browser_reload" => call_browser_simple_action(
            arguments,
            session,
            "browser_reload",
            "mcp-browser-reload",
            "reload",
        ),
        "browser_tab_new" => call_browser_tab_new(arguments, session),
        "browser_tab_switch" => call_browser_tab_switch(arguments, session),
        "browser_tab_close" => call_browser_tab_close(arguments, session),
        "browser_set_content" => call_browser_set_content(arguments, session),
        "browser_snapshot" => call_browser_snapshot(arguments, session),
        "browser_get_url" => call_browser_read_tool(arguments, session, BROWSER_GET_URL_TOOL),
        "browser_get_title" => call_browser_read_tool(arguments, session, BROWSER_GET_TITLE_TOOL),
        "browser_tabs" => call_browser_tabs(arguments, session),
        "browser_screenshot" => call_browser_screenshot(arguments, session),
        "browser_click" => call_browser_click(arguments, session),
        "browser_fill" => call_browser_fill(arguments, session),
        "browser_wait" => call_browser_wait(arguments, session),
        "browser_type" => call_browser_type(arguments, session),
        "browser_press" => call_browser_press(arguments, session),
        "browser_hover" => call_browser_hover(arguments, session),
        "browser_select" => call_browser_select(arguments, session),
        "browser_get_text" => {
            call_browser_element_read_tool(arguments, session, "browser_get_text", "gettext", None)
        }
        "browser_get_value" => call_browser_element_read_tool(
            arguments,
            session,
            "browser_get_value",
            "inputvalue",
            None,
        ),
        "browser_get_attribute" => call_browser_element_read_tool(
            arguments,
            session,
            "browser_get_attribute",
            "getattribute",
            Some("attribute"),
        ),
        "browser_get_html" => call_browser_element_read_tool(
            arguments,
            session,
            "browser_get_html",
            "innerhtml",
            None,
        ),
        "browser_get_styles" => call_browser_get_styles(arguments, session),
        "browser_count" => {
            call_browser_element_read_tool(arguments, session, "browser_count", "count", None)
        }
        "browser_get_box" => call_browser_element_read_tool(
            arguments,
            session,
            "browser_get_box",
            "boundingbox",
            None,
        ),
        "browser_is_visible" => {
            call_browser_element_state_tool(arguments, session, "browser_is_visible", "isvisible")
        }
        "browser_is_enabled" => {
            call_browser_element_state_tool(arguments, session, "browser_is_enabled", "isenabled")
        }
        "browser_check" => call_browser_check(arguments, session),
        "browser_is_checked" => call_browser_is_checked(arguments, session),
        "browser_uncheck" => call_browser_uncheck(arguments, session),
        "browser_scroll" => call_browser_scroll(arguments, session),
        "browser_scroll_into_view" => call_browser_scroll_into_view(arguments, session),
        "browser_focus" => call_browser_focus(arguments, session),
        "browser_clear" => call_browser_clear(arguments, session),
        _ => Err(JsonRpcError {
            code: -32602,
            message: "Invalid params",
            data: Some(json!({ "message": format!("Unknown MCP tool: {}", name) })),
        }),
    }
}

fn call_service_trace(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let limit = optional_positive_u64_argument(arguments, "limit")?
        .map(|value| value as usize)
        .unwrap_or(20);
    let browser_id = optional_string_argument(arguments, "browserId")?;
    let profile_id = optional_string_argument(arguments, "profileId")?;
    let session_id = optional_string_argument(arguments, "sessionId")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let since = optional_string_argument(arguments, "since")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let state = load_default_service_state_snapshot().map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({ "message": err, "tool": "service_trace" })),
    })?;
    let data = service_trace_response(
        &state,
        ServiceTraceFilters {
            limit,
            browser_id,
            profile_id,
            session_id,
            service_name,
            agent_name,
            task_name,
            since,
        },
    )
    .map_err(|err| JsonRpcError {
        code: -32602,
        message: "Invalid params",
        data: Some(json!({ "message": err, "tool": "service_trace" })),
    })?;

    Ok(tool_response_from_payload(
        "service_trace",
        session,
        trace,
        data,
        false,
    ))
}

fn call_service_incidents(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let limit = optional_positive_u64_argument(arguments, "limit")?
        .map(|value| value as usize)
        .unwrap_or(20);
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let state = load_default_service_state_snapshot().map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({ "message": err, "tool": "service_incidents" })),
    })?;
    let remedies_only = optional_bool_argument(arguments, "remediesOnly")?.unwrap_or(false);
    let summarize = optional_bool_argument(arguments, "summary")?.unwrap_or(false) || remedies_only;
    let incident_state =
        optional_enum_string_argument(arguments, "state", &["active", "recovered", "service"])?
            .or(if remedies_only { Some("active") } else { None });
    let mut data = service_incidents_response(
        &state,
        ServiceIncidentFilters {
            limit,
            incident_id: optional_string_argument(arguments, "incidentId")?,
            state: incident_state,
            severity: optional_enum_string_argument(
                arguments,
                "severity",
                &["info", "warning", "error", "critical"],
            )?,
            escalation: optional_enum_string_argument(
                arguments,
                "escalation",
                &[
                    "none",
                    "browser_degraded",
                    "browser_recovery",
                    "job_attention",
                    "service_triage",
                    "os_degraded_possible",
                ],
            )?,
            handling_state: optional_enum_string_argument(
                arguments,
                "handlingState",
                &["unacknowledged", "acknowledged", "resolved"],
            )?,
            kind: optional_enum_string_argument(
                arguments,
                "kind",
                &[
                    "browser_health_changed",
                    "reconciliation_error",
                    "service_job_timeout",
                    "service_job_cancelled",
                ],
            )?,
            browser_id: optional_string_argument(arguments, "browserId")?,
            profile_id: optional_string_argument(arguments, "profileId")?,
            session_id: optional_string_argument(arguments, "sessionId")?,
            service_name,
            agent_name,
            task_name,
            since: optional_string_argument(arguments, "since")?,
            remedies_only,
        },
    )
    .map_err(|err| JsonRpcError {
        code: -32602,
        message: "Invalid params",
        data: Some(json!({ "message": err, "tool": "service_incidents" })),
    })?;
    if summarize {
        let incidents = data
            .get("incidents")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        data["summary"] = service_incident_summary(&incidents);
    }

    Ok(tool_response_from_payload(
        "service_incidents",
        session,
        trace,
        data,
        false,
    ))
}

fn call_service_site_policy_upsert(
    arguments: &Value,
    session: &str,
) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let site_policy = required_object_argument(arguments, "sitePolicy")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command =
        service_site_policy_upsert_command(id, &site_policy, service_name, agent_name, task_name);

    send_queued_tool_command("service_site_policy_upsert", session, trace, command)
}

fn call_service_site_policy_delete(
    arguments: &Value,
    session: &str,
) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_site_policy_delete_command(id, service_name, agent_name, task_name);

    send_queued_tool_command("service_site_policy_delete", session, trace, command)
}

fn call_service_monitor_upsert(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let monitor = required_object_argument(arguments, "monitor")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_monitor_upsert_command(id, &monitor, service_name, agent_name, task_name);

    send_queued_tool_command("service_monitor_upsert", session, trace, command)
}

fn call_service_monitor_delete(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_monitor_delete_command(id, service_name, agent_name, task_name);

    send_queued_tool_command("service_monitor_delete", session, trace, command)
}

fn call_service_monitors_run_due(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_monitors_run_due_command(service_name, agent_name, task_name);

    send_queued_tool_command("service_monitors_run_due", session, trace, command)
}

fn call_service_monitor_state(
    arguments: &Value,
    session: &str,
    action: &str,
) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_monitor_state_command(id, action, service_name, agent_name, task_name);

    send_queued_tool_command(action, session, trace, command)
}

fn call_service_profile_upsert(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let profile = required_object_argument(arguments, "profile")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_profile_upsert_command(id, &profile, service_name, agent_name, task_name);

    send_queued_tool_command("service_profile_upsert", session, trace, command)
}

fn call_service_profile_freshness_update(
    arguments: &Value,
    session: &str,
) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let freshness = service_profile_freshness_arguments(arguments);
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command =
        service_profile_freshness_command(id, &freshness, service_name, agent_name, task_name);

    send_queued_tool_command("service_profile_freshness_update", session, trace, command)
}

fn call_service_profile_delete(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_profile_delete_command(id, service_name, agent_name, task_name);

    send_queued_tool_command("service_profile_delete", session, trace, command)
}

fn call_service_session_upsert(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_session = required_object_argument(arguments, "session")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command =
        service_session_upsert_command(id, &service_session, service_name, agent_name, task_name);

    send_queued_tool_command("service_session_upsert", session, trace, command)
}

fn call_service_session_delete(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_session_delete_command(id, service_name, agent_name, task_name);

    send_queued_tool_command("service_session_delete", session, trace, command)
}

fn call_service_provider_upsert(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let provider = required_object_argument(arguments, "provider")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command =
        service_provider_upsert_command(id, &provider, service_name, agent_name, task_name);

    send_queued_tool_command("service_provider_upsert", session, trace, command)
}

fn call_service_provider_delete(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let id = required_string_argument(arguments, "id")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = service_provider_delete_command(id, service_name, agent_name, task_name);

    send_queued_tool_command("service_provider_delete", session, trace, command)
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

fn call_browser_command(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let action = required_string_argument(arguments, "action")?;
    if !BROWSER_COMMAND_ALLOWED_ACTIONS.contains(&action) {
        return Err(JsonRpcError::invalid_params(&format!(
            "browser_command action '{}' is not supported",
            action
        )));
    }
    let params = optional_object_argument(arguments, "params")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_command_command(BrowserCommandArgs {
        action,
        params,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_command", session, trace, command)
}

fn service_request_command(arguments: &Value) -> Result<(Value, Value), JsonRpcError> {
    let action = required_string_argument(arguments, "action")?;
    if !BROWSER_COMMAND_ALLOWED_ACTIONS.contains(&action) {
        return Err(JsonRpcError::invalid_params(&format!(
            "service_request action '{}' is not supported",
            action
        )));
    }
    let params = optional_object_argument(arguments, "params")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let mut command = browser_command_command(BrowserCommandArgs {
        action,
        params,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });
    if let Some(id) = command.get("id").and_then(Value::as_str) {
        let new_id = id.replacen("mcp-browser-command-", "mcp-service-request-", 1);
        command["id"] = json!(new_id);
    }
    context.apply_target_profile_hints(&mut command);
    Ok((trace, command))
}

fn call_service_request(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let (trace, command) = service_request_command(arguments)?;
    send_queued_tool_command("service_request", session, trace, command)
}

fn call_browser_navigate(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let url = required_string_argument(arguments, "url")?;
    let wait_until = optional_wait_until_argument(arguments)?;
    let headers = optional_object_argument(arguments, "headers")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_navigate_command(BrowserNavigateCommandArgs {
        url,
        wait_until,
        headers,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_navigate", session, trace, command)
}

fn call_browser_requests(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let clear = optional_bool_argument(arguments, "clear")?;
    let filter = optional_string_argument(arguments, "filter")?;
    let resource_type = optional_string_argument(arguments, "type")?;
    let method = optional_string_argument(arguments, "method")?;
    let status = optional_string_argument(arguments, "status")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_requests_command(BrowserRequestsCommandArgs {
        clear,
        filter,
        resource_type,
        method,
        status,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_requests", session, trace, command)
}

fn call_browser_request_detail(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let request_id = required_string_argument(arguments, "requestId")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_request_detail_command(
        request_id,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_request_detail", session, trace, command)
}

fn call_browser_headers(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let headers = arguments
        .get("headers")
        .and_then(|value| value.as_object())
        .ok_or_else(|| JsonRpcError::invalid_params("headers must be an object"))?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_headers_command(
        headers,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_headers", session, trace, command)
}

fn call_browser_offline(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let offline = optional_bool_argument(arguments, "offline")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_offline_command(
        offline,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_offline", session, trace, command)
}

fn call_browser_cookies_get(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let urls = optional_string_array_argument(arguments, "urls")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_cookies_get_command(
        urls.as_deref(),
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_cookies_get", session, trace, command)
}

fn call_browser_cookies_set(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let cookies = optional_cookie_array_argument(arguments)?;
    let name = optional_string_argument(arguments, "name")?;
    let value = optional_string_argument(arguments, "value")?;
    if cookies.is_none() && (name.is_none() || value.is_none()) {
        return Err(JsonRpcError::invalid_params(
            "browser_cookies_set requires cookies or name and value",
        ));
    }
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_cookies_set_command(BrowserCookiesSetCommandArgs {
        cookies,
        name,
        value,
        url: optional_string_argument(arguments, "url")?,
        domain: optional_string_argument(arguments, "domain")?,
        path: optional_string_argument(arguments, "path")?,
        expires: arguments.get("expires").filter(|value| !value.is_null()),
        http_only: optional_bool_argument(arguments, "httpOnly")?,
        secure: optional_bool_argument(arguments, "secure")?,
        same_site: optional_string_argument(arguments, "sameSite")?,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_cookies_set", session, trace, command)
}

fn call_browser_cookies_clear(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_action_command(
        "mcp-browser-cookies-clear",
        "cookies_clear",
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_cookies_clear", session, trace, command)
}

fn call_browser_storage_get(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let storage_type = optional_storage_type_argument(arguments)?;
    let key = optional_string_argument(arguments, "key")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_storage_get_command(
        storage_type,
        key,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_storage_get", session, trace, command)
}

fn call_browser_storage_set(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let storage_type = optional_storage_type_argument(arguments)?;
    let key = required_string_argument(arguments, "key")?;
    let value = required_string_argument(arguments, "value")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_storage_set_command(BrowserStorageSetCommandArgs {
        storage_type,
        key,
        value,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_storage_set", session, trace, command)
}

fn call_browser_storage_clear(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let storage_type = optional_storage_type_argument(arguments)?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_storage_clear_command(
        storage_type,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_storage_clear", session, trace, command)
}

fn call_browser_user_agent(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let user_agent = required_string_argument(arguments, "userAgent")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_user_agent_command(
        user_agent,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_user_agent", session, trace, command)
}

fn call_browser_viewport(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let width = required_positive_u64_argument(arguments, "width")?;
    let height = required_positive_u64_argument(arguments, "height")?;
    let device_scale_factor = optional_positive_f64_argument(arguments, "deviceScaleFactor")?;
    let mobile = optional_bool_argument(arguments, "mobile")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_viewport_command(BrowserViewportCommandArgs {
        width,
        height,
        device_scale_factor,
        mobile,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_viewport", session, trace, command)
}

fn call_browser_geolocation(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let latitude = required_f64_argument(arguments, "latitude")?;
    let longitude = required_f64_argument(arguments, "longitude")?;
    let accuracy = optional_non_negative_f64_argument(arguments, "accuracy")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_geolocation_command(
        latitude,
        longitude,
        accuracy,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_geolocation", session, trace, command)
}

fn call_browser_permissions(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let permissions = required_string_array_argument(arguments, "permissions")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_permissions_command(
        &permissions,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_permissions", session, trace, command)
}

fn call_browser_timezone(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let timezone_id = required_string_argument(arguments, "timezoneId")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_timezone_command(
        timezone_id,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_timezone", session, trace, command)
}

fn call_browser_locale(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let locale = required_string_argument(arguments, "locale")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_locale_command(
        locale,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_locale", session, trace, command)
}

fn call_browser_media(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let media = optional_string_argument(arguments, "media")?;
    let color_scheme = optional_string_argument(arguments, "colorScheme")?;
    let reduced_motion = optional_string_argument(arguments, "reducedMotion")?;
    let features = optional_object_argument(arguments, "features")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_media_command(BrowserMediaCommandArgs {
        media,
        color_scheme,
        reduced_motion,
        features,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_media", session, trace, command)
}

fn call_browser_dialog(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let response = required_dialog_response_argument(arguments)?;
    let prompt_text = optional_string_argument(arguments, "promptText")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_dialog_command(
        response,
        prompt_text,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_dialog", session, trace, command)
}

fn call_browser_upload(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let files = required_string_array_argument(arguments, "files")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_upload_command(
        selector,
        &files,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_upload", session, trace, command)
}

fn call_browser_download(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let path = required_string_argument(arguments, "path")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_download_command(
        selector,
        path,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_download", session, trace, command)
}

fn call_browser_wait_for_download(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let path = optional_string_argument(arguments, "path")?;
    let timeout_ms = optional_positive_u64_argument(arguments, "timeoutMs")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_wait_for_download_command(
        path,
        timeout_ms,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_wait_for_download", session, trace, command)
}

fn call_browser_har_start(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_action_command(
        "mcp-browser-har-start",
        "har_start",
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_har_start", session, trace, command)
}

fn call_browser_har_stop(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let path = optional_string_argument(arguments, "path")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_har_stop_command(
        path,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_har_stop", session, trace, command)
}

fn call_browser_route(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let url = required_string_argument(arguments, "url")?;
    let abort = optional_bool_argument(arguments, "abort")?;
    let response = optional_object_argument(arguments, "response")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_route_command(BrowserRouteCommandArgs {
        url,
        abort,
        response,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_route", session, trace, command)
}

fn call_browser_unroute(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let url = optional_string_argument(arguments, "url")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_unroute_command(
        url,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_unroute", session, trace, command)
}

fn call_browser_console(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let clear = optional_bool_argument(arguments, "clear")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_console_command(
        clear,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_console", session, trace, command)
}

fn call_browser_errors(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_action_command(
        "mcp-browser-errors",
        "errors",
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_errors", session, trace, command)
}

fn call_browser_pdf(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let path = optional_string_argument(arguments, "path")?;
    let print_background = optional_bool_argument(arguments, "printBackground")?;
    let landscape = optional_bool_argument(arguments, "landscape")?;
    let prefer_css_page_size = optional_bool_argument(arguments, "preferCSSPageSize")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_pdf_command(BrowserPdfCommandArgs {
        path,
        print_background,
        landscape,
        prefer_css_page_size,
        job_timeout_ms: context.job_timeout_ms,
        service_name: context.service_name,
        agent_name: context.agent_name,
        task_name: context.task_name,
    });

    send_queued_tool_command("browser_pdf", session, trace, command)
}

fn call_browser_response_body(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let url = required_string_argument(arguments, "url")?;
    let timeout_ms = optional_positive_u64_argument(arguments, "timeoutMs")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_response_body_command(
        url,
        timeout_ms,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_response_body", session, trace, command)
}

fn call_browser_clipboard(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let operation = optional_clipboard_operation_argument(arguments)?;
    let text = optional_string_argument(arguments, "text")?;
    if operation == Some("write") && text.is_none() {
        return Err(JsonRpcError::invalid_params(
            "text is required when operation is write",
        ));
    }
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_clipboard_command(
        operation,
        text,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_clipboard", session, trace, command)
}

fn call_browser_simple_action(
    arguments: &Value,
    session: &str,
    tool_name: &str,
    id_prefix: &str,
    action: &str,
) -> Result<Value, JsonRpcError> {
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_action_command(
        id_prefix,
        action,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command(tool_name, session, trace, command)
}

fn call_browser_tab_new(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let url = optional_string_argument(arguments, "url")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_tab_new_command(
        url,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_tab_new", session, trace, command)
}

fn call_browser_tab_switch(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let index = required_u64_argument(arguments, "index")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_tab_switch_command(
        index,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_tab_switch", session, trace, command)
}

fn call_browser_tab_close(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let index = optional_u64_argument(arguments, "index")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_tab_close_command(
        index,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_tab_close", session, trace, command)
}

fn call_browser_set_content(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let html = required_string_argument(arguments, "html")?;
    let context = ServiceToolContext::from_arguments(arguments)?;
    let trace = context.trace();
    let command = browser_set_content_command(
        html,
        context.job_timeout_ms,
        context.service_name,
        context.agent_name,
        context.task_name,
    );

    send_queued_tool_command("browser_set_content", session, trace, command)
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

fn call_browser_read_tool(
    arguments: &Value,
    session: &str,
    spec: BrowserReadToolSpec,
) -> Result<Value, JsonRpcError> {
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_read_command(spec, job_timeout_ms, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": spec.tool_name,
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        spec.tool_name,
        session,
        trace,
        response,
    ))
}

fn call_browser_tabs(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let verbose = optional_bool_argument(arguments, "verbose")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command =
        browser_tabs_command(verbose, job_timeout_ms, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_tabs",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_tabs",
        session,
        trace,
        response,
    ))
}

fn call_browser_screenshot(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = optional_string_argument(arguments, "selector")?;
    let path = optional_string_argument(arguments, "path")?;
    let full_page = optional_bool_argument(arguments, "fullPage")?;
    let annotate = optional_bool_argument(arguments, "annotate")?;
    let format = optional_screenshot_format_argument(arguments)?;
    let quality = optional_bounded_u64_argument(arguments, "quality", 0, 100)?;
    let screenshot_dir = optional_string_argument(arguments, "screenshotDir")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_screenshot_command(BrowserScreenshotCommandArgs {
        selector,
        path,
        full_page,
        annotate,
        format,
        quality,
        screenshot_dir,
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
            "tool": "browser_screenshot",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_screenshot",
        session,
        trace,
        response,
    ))
}

fn call_browser_click(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let new_tab = optional_bool_argument(arguments, "newTab")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_click_command(
        selector,
        new_tab,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_click",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_click",
        session,
        trace,
        response,
    ))
}

fn call_browser_fill(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let value = required_string_argument(arguments, "value")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_fill_command(
        selector,
        value,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_fill",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_fill",
        session,
        trace,
        response,
    ))
}

fn call_browser_wait(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = optional_string_argument(arguments, "selector")?;
    let state = optional_selector_wait_state_argument(arguments)?;
    let text = optional_string_argument(arguments, "text")?;
    let url = optional_string_argument(arguments, "url")?;
    let function = optional_string_argument(arguments, "function")?;
    let load_state = optional_load_state_argument(arguments)?;
    let timeout_ms = optional_positive_u64_argument(arguments, "timeoutMs")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let args = BrowserWaitCommandArgs {
        selector,
        state,
        text,
        url,
        function,
        load_state,
        timeout_ms,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    };
    let command = browser_wait_command(args)?;

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_wait",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_wait",
        session,
        trace,
        response,
    ))
}

fn call_browser_type(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let text = required_string_argument(arguments, "text")?;
    let clear = optional_bool_argument(arguments, "clear")?;
    let delay_ms = optional_u64_argument(arguments, "delayMs")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_type_command(BrowserTypeCommandArgs {
        selector,
        text,
        clear,
        delay_ms,
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
            "tool": "browser_type",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_type",
        session,
        trace,
        response,
    ))
}

fn call_browser_press(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let key = required_string_argument(arguments, "key")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_press_command(key, job_timeout_ms, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_press",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_press",
        session,
        trace,
        response,
    ))
}

fn call_browser_hover(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_hover_command(
        selector,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_hover",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_hover",
        session,
        trace,
        response,
    ))
}

fn call_browser_select(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let values = required_string_array_argument(arguments, "values")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_select_command(
        selector,
        &values,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_select",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_select",
        session,
        trace,
        response,
    ))
}

fn call_browser_element_state_tool(
    arguments: &Value,
    session: &str,
    tool_name: &str,
    action: &str,
) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_element_state_command(
        action,
        selector,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": tool_name,
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        tool_name, session, trace, response,
    ))
}

fn call_browser_element_read_tool(
    arguments: &Value,
    session: &str,
    tool_name: &str,
    action: &str,
    attribute_key: Option<&str>,
) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let attribute = match attribute_key {
        Some(key) => Some(required_string_argument(arguments, key)?),
        None => None,
    };
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_element_read_command(BrowserElementReadCommandArgs {
        action,
        selector,
        attribute,
        properties: None,
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
            "tool": tool_name,
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        tool_name, session, trace, response,
    ))
}

fn call_browser_get_styles(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let properties = optional_string_array_argument(arguments, "properties")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_element_read_command(BrowserElementReadCommandArgs {
        action: "styles",
        selector,
        attribute: None,
        properties,
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
            "tool": "browser_get_styles",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_get_styles",
        session,
        trace,
        response,
    ))
}

fn call_browser_check(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    call_browser_checked_tool(arguments, session, "browser_check", "check")
}

fn call_browser_uncheck(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    call_browser_checked_tool(arguments, session, "browser_uncheck", "uncheck")
}

fn call_browser_is_checked(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    call_browser_checked_tool(arguments, session, "browser_is_checked", "ischecked")
}

fn call_browser_checked_tool(
    arguments: &Value,
    session: &str,
    tool_name: &str,
    action: &str,
) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_checked_command(
        action,
        selector,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": tool_name,
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        tool_name, session, trace, response,
    ))
}

fn call_browser_scroll(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = optional_string_argument(arguments, "selector")?;
    let direction = optional_scroll_direction_argument(arguments)?;
    let amount = optional_non_negative_f64_argument(arguments, "amount")?;
    let delta_x = optional_f64_argument(arguments, "deltaX")?;
    let delta_y = optional_f64_argument(arguments, "deltaY")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_scroll_command(BrowserScrollCommandArgs {
        selector,
        direction,
        amount,
        delta_x,
        delta_y,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    })?;

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_scroll",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_scroll",
        session,
        trace,
        response,
    ))
}

fn call_browser_scroll_into_view(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_scroll_into_view_command(
        selector,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "browser_scroll_into_view",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "browser_scroll_into_view",
        session,
        trace,
        response,
    ))
}

fn call_browser_focus(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    call_browser_field_tool(arguments, session, "browser_focus", "focus")
}

fn call_browser_clear(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    call_browser_field_tool(arguments, session, "browser_clear", "clear")
}

fn call_browser_field_tool(
    arguments: &Value,
    session: &str,
    tool_name: &str,
    action: &str,
) -> Result<Value, JsonRpcError> {
    let selector = required_string_argument(arguments, "selector")?;
    let job_timeout_ms = optional_positive_u64_argument(arguments, "jobTimeoutMs")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);
    let command = browser_field_command(
        action,
        selector,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": tool_name,
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        tool_name, session, trace, response,
    ))
}

fn call_service_browser_retry(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let browser_id = arguments
        .get("browserId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| JsonRpcError::invalid_params("service_browser_retry requires browserId"))?;
    let by = optional_string_argument(arguments, "by")?;
    let note = optional_string_argument(arguments, "note")?;
    let service_name = optional_string_argument(arguments, "serviceName")?;
    let agent_name = optional_string_argument(arguments, "agentName")?;
    let task_name = optional_string_argument(arguments, "taskName")?;
    let trace = service_tool_trace(service_name, agent_name, task_name);

    let command =
        service_browser_retry_command(browser_id, by, note, service_name, agent_name, task_name);

    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": "service_browser_retry",
            "trace": trace,
        })),
    })?;
    Ok(tool_response_from_daemon(
        "service_browser_retry",
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

fn service_site_policy_upsert_command(
    site_policy_id: &str,
    site_policy: &Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-site-policy-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_site_policy_upsert",
        "sitePolicyId": site_policy_id,
        "sitePolicy": site_policy,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_profile_upsert_command(
    profile_id: &str,
    profile: &Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-profile-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_profile_upsert",
        "profileId": profile_id,
        "profile": profile,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_profile_freshness_arguments(arguments: &Value) -> Value {
    let mut freshness = serde_json::Map::new();
    for key in [
        "loginId",
        "siteId",
        "targetServiceId",
        "targetServiceIds",
        "readinessState",
        "readinessEvidence",
        "readinessRecommendedAction",
        "lastVerifiedAt",
        "freshnessExpiresAt",
        "updateAuthenticatedServiceIds",
    ] {
        if let Some(value) = arguments.get(key) {
            freshness.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(freshness)
}

fn service_profile_freshness_command(
    profile_id: &str,
    freshness: &Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-profile-freshness-{}", uuid::Uuid::new_v4()),
        "action": "service_profile_freshness_update",
        "profileId": profile_id,
        "freshness": freshness,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_profile_delete_command(
    profile_id: &str,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-profile-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_profile_delete",
        "profileId": profile_id,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_session_upsert_command(
    session_id: &str,
    service_session: &Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-session-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_session_upsert",
        "sessionId": session_id,
        "session": service_session,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_session_delete_command(
    session_id: &str,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-session-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_session_delete",
        "sessionId": session_id,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_site_policy_delete_command(
    site_policy_id: &str,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-site-policy-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_site_policy_delete",
        "sitePolicyId": site_policy_id,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_monitor_upsert_command(
    monitor_id: &str,
    monitor: &Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-monitor-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_monitor_upsert",
        "monitorId": monitor_id,
        "monitor": monitor,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_monitor_delete_command(
    monitor_id: &str,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-monitor-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_monitor_delete",
        "monitorId": monitor_id,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_monitors_run_due_command(
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-monitors-run-due-{}", uuid::Uuid::new_v4()),
        "action": "service_monitors_run_due",
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_monitor_state_command(
    monitor_id: &str,
    action: &str,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-{action}-{}", uuid::Uuid::new_v4()),
        "action": action,
        "monitorId": monitor_id,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_provider_upsert_command(
    provider_id: &str,
    provider: &Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-provider-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_provider_upsert",
        "providerId": provider_id,
        "provider": provider,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn service_provider_delete_command(
    provider_id: &str,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-provider-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_provider_delete",
        "providerId": provider_id,
    });
    apply_service_trace_fields(&mut command, service_name, agent_name, task_name);
    command
}

fn apply_service_trace_fields(
    command: &mut Value,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) {
    if let Some(service_name) = service_name {
        command["serviceName"] = json!(service_name);
    }
    if let Some(agent_name) = agent_name {
        command["agentName"] = json!(agent_name);
    }
    if let Some(task_name) = task_name {
        command["taskName"] = json!(task_name);
    }
}

fn service_browser_retry_command(
    browser_id: &str,
    by: Option<&str>,
    note: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-service-browser-retry-{}", uuid::Uuid::new_v4()),
        "action": "service_browser_retry",
        "browserId": browser_id,
    });
    if let Some(by) = by {
        command["by"] = json!(by);
    }
    if let Some(note) = note {
        command["note"] = json!(note);
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

struct BrowserCommandArgs<'a> {
    action: &'a str,
    params: Option<&'a serde_json::Map<String, Value>>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserNavigateCommandArgs<'a> {
    url: &'a str,
    wait_until: Option<&'a str>,
    headers: Option<&'a serde_json::Map<String, Value>>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserRequestsCommandArgs<'a> {
    clear: Option<bool>,
    filter: Option<&'a str>,
    resource_type: Option<&'a str>,
    method: Option<&'a str>,
    status: Option<&'a str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserCookiesSetCommandArgs<'a> {
    cookies: Option<&'a [Value]>,
    name: Option<&'a str>,
    value: Option<&'a str>,
    url: Option<&'a str>,
    domain: Option<&'a str>,
    path: Option<&'a str>,
    expires: Option<&'a Value>,
    http_only: Option<bool>,
    secure: Option<bool>,
    same_site: Option<&'a str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserStorageSetCommandArgs<'a> {
    storage_type: Option<&'a str>,
    key: &'a str,
    value: &'a str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserViewportCommandArgs<'a> {
    width: u64,
    height: u64,
    device_scale_factor: Option<f64>,
    mobile: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserMediaCommandArgs<'a> {
    media: Option<&'a str>,
    color_scheme: Option<&'a str>,
    reduced_motion: Option<&'a str>,
    features: Option<&'a serde_json::Map<String, Value>>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserRouteCommandArgs<'a> {
    url: &'a str,
    abort: Option<bool>,
    response: Option<&'a serde_json::Map<String, Value>>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

struct BrowserPdfCommandArgs<'a> {
    path: Option<&'a str>,
    print_background: Option<bool>,
    landscape: Option<bool>,
    prefer_css_page_size: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_command_command(args: BrowserCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-command-{}-{}", args.action, uuid::Uuid::new_v4()),
        "action": args.action,
    });
    if let Some(params) = args.params {
        for (key, value) in params {
            if key != "id" && key != "action" {
                command[key] = value.clone();
            }
        }
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_navigate_command(args: BrowserNavigateCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-navigate-{}", uuid::Uuid::new_v4()),
        "action": "navigate",
        "url": args.url,
    });
    if let Some(wait_until) = args.wait_until {
        command["waitUntil"] = json!(wait_until);
    }
    if let Some(headers) = args.headers {
        command["headers"] = json!(headers);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_requests_command(args: BrowserRequestsCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-requests-{}", uuid::Uuid::new_v4()),
        "action": "requests",
    });
    if let Some(clear) = args.clear {
        command["clear"] = json!(clear);
    }
    if let Some(filter) = args.filter {
        command["filter"] = json!(filter);
    }
    if let Some(resource_type) = args.resource_type {
        command["type"] = json!(resource_type);
    }
    if let Some(method) = args.method {
        command["method"] = json!(method);
    }
    if let Some(status) = args.status {
        command["status"] = json!(status);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_request_detail_command(
    request_id: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-request-detail-{}", uuid::Uuid::new_v4()),
        "action": "request_detail",
        "requestId": request_id,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_headers_command(
    headers: &serde_json::Map<String, Value>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-headers-{}", uuid::Uuid::new_v4()),
        "action": "headers",
        "headers": headers,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_offline_command(
    offline: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-offline-{}", uuid::Uuid::new_v4()),
        "action": "offline",
    });
    if let Some(offline) = offline {
        command["offline"] = json!(offline);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_cookies_get_command(
    urls: Option<&[String]>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-cookies-get-{}", uuid::Uuid::new_v4()),
        "action": "cookies_get",
    });
    if let Some(urls) = urls {
        command["urls"] = json!(urls);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_cookies_set_command(args: BrowserCookiesSetCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-cookies-set-{}", uuid::Uuid::new_v4()),
        "action": "cookies_set",
    });
    if let Some(cookies) = args.cookies {
        command["cookies"] = json!(cookies);
    }
    if let Some(name) = args.name {
        command["name"] = json!(name);
    }
    if let Some(value) = args.value {
        command["value"] = json!(value);
    }
    if let Some(url) = args.url {
        command["url"] = json!(url);
    }
    if let Some(domain) = args.domain {
        command["domain"] = json!(domain);
    }
    if let Some(path) = args.path {
        command["path"] = json!(path);
    }
    if let Some(expires) = args.expires {
        command["expires"] = expires.clone();
    }
    if let Some(http_only) = args.http_only {
        command["httpOnly"] = json!(http_only);
    }
    if let Some(secure) = args.secure {
        command["secure"] = json!(secure);
    }
    if let Some(same_site) = args.same_site {
        command["sameSite"] = json!(same_site);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_storage_get_command(
    storage_type: Option<&str>,
    key: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-storage-get-{}", uuid::Uuid::new_v4()),
        "action": "storage_get",
    });
    if let Some(storage_type) = storage_type {
        command["type"] = json!(storage_type);
    }
    if let Some(key) = key {
        command["key"] = json!(key);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_storage_set_command(args: BrowserStorageSetCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-storage-set-{}", uuid::Uuid::new_v4()),
        "action": "storage_set",
        "key": args.key,
        "value": args.value,
    });
    if let Some(storage_type) = args.storage_type {
        command["type"] = json!(storage_type);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_storage_clear_command(
    storage_type: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-storage-clear",
        "storage_clear",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(storage_type) = storage_type {
        command["type"] = json!(storage_type);
    }
    command
}

fn browser_action_command(
    id_prefix: &str,
    action: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("{}-{}", id_prefix, uuid::Uuid::new_v4()),
        "action": action,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_tab_new_command(
    url: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-tab-new",
        "tab_new",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(url) = url {
        command["url"] = json!(url);
    }
    command
}

fn browser_tab_switch_command(
    index: u64,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-tab-switch",
        "tab_switch",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command["index"] = json!(index);
    command
}

fn browser_tab_close_command(
    index: Option<u64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-tab-close",
        "tab_close",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(index) = index {
        command["index"] = json!(index);
    }
    command
}

fn browser_set_content_command(
    html: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-set-content",
        "setcontent",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command["html"] = json!(html);
    command
}

fn browser_user_agent_command(
    user_agent: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-user-agent-{}", uuid::Uuid::new_v4()),
        "action": "user_agent",
        "userAgent": user_agent,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_viewport_command(args: BrowserViewportCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-viewport-{}", uuid::Uuid::new_v4()),
        "action": "viewport",
        "width": args.width,
        "height": args.height,
    });
    if let Some(device_scale_factor) = args.device_scale_factor {
        command["deviceScaleFactor"] = json!(device_scale_factor);
    }
    if let Some(mobile) = args.mobile {
        command["mobile"] = json!(mobile);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_geolocation_command(
    latitude: f64,
    longitude: f64,
    accuracy: Option<f64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-geolocation-{}", uuid::Uuid::new_v4()),
        "action": "geolocation",
        "latitude": latitude,
        "longitude": longitude,
    });
    if let Some(accuracy) = accuracy {
        command["accuracy"] = json!(accuracy);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_permissions_command(
    permissions: &[String],
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-permissions-{}", uuid::Uuid::new_v4()),
        "action": "permissions",
        "permissions": permissions,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_timezone_command(
    timezone_id: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-timezone-{}", uuid::Uuid::new_v4()),
        "action": "timezone",
        "timezoneId": timezone_id,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_locale_command(
    locale: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-locale-{}", uuid::Uuid::new_v4()),
        "action": "locale",
        "locale": locale,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_media_command(args: BrowserMediaCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-media-{}", uuid::Uuid::new_v4()),
        "action": "emulatemedia",
    });
    if let Some(media) = args.media {
        command["media"] = json!(media);
    }
    if let Some(color_scheme) = args.color_scheme {
        command["colorScheme"] = json!(color_scheme);
    }
    if let Some(reduced_motion) = args.reduced_motion {
        command["reducedMotion"] = json!(reduced_motion);
    }
    if let Some(features) = args.features {
        command["features"] = json!(features);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_dialog_command(
    response: &str,
    prompt_text: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-dialog-{}", uuid::Uuid::new_v4()),
        "action": "dialog",
        "response": response,
    });
    if let Some(prompt_text) = prompt_text {
        command["promptText"] = json!(prompt_text);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_upload_command(
    selector: &str,
    files: &[String],
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-upload-{}", uuid::Uuid::new_v4()),
        "action": "upload",
        "selector": selector,
        "files": files,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_download_command(
    selector: &str,
    path: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-download-{}", uuid::Uuid::new_v4()),
        "action": "download",
        "selector": selector,
        "path": path,
    });
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_wait_for_download_command(
    path: Option<&str>,
    timeout_ms: Option<u64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-wait-for-download-{}", uuid::Uuid::new_v4()),
        "action": "waitfordownload",
    });
    if let Some(path) = path {
        command["path"] = json!(path);
    }
    if let Some(timeout_ms) = timeout_ms {
        command["timeoutMs"] = json!(timeout_ms);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_har_stop_command(
    path: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-har-stop",
        "har_stop",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(path) = path {
        command["path"] = json!(path);
    }
    command
}

fn browser_route_command(args: BrowserRouteCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-route-{}", uuid::Uuid::new_v4()),
        "action": "route",
        "url": args.url,
    });
    if let Some(abort) = args.abort {
        command["abort"] = json!(abort);
    }
    if let Some(response) = args.response {
        command["response"] = json!(response);
    }
    apply_service_command_fields(
        &mut command,
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    command
}

fn browser_unroute_command(
    url: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-unroute",
        "unroute",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(url) = url {
        command["url"] = json!(url);
    }
    command
}

fn browser_console_command(
    clear: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-console",
        "console",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(clear) = clear {
        command["clear"] = json!(clear);
    }
    command
}

fn browser_pdf_command(args: BrowserPdfCommandArgs<'_>) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-pdf",
        "pdf",
        args.job_timeout_ms,
        args.service_name,
        args.agent_name,
        args.task_name,
    );
    if let Some(path) = args.path {
        command["path"] = json!(path);
    }
    if let Some(print_background) = args.print_background {
        command["printBackground"] = json!(print_background);
    }
    if let Some(landscape) = args.landscape {
        command["landscape"] = json!(landscape);
    }
    if let Some(prefer_css_page_size) = args.prefer_css_page_size {
        command["preferCSSPageSize"] = json!(prefer_css_page_size);
    }
    command
}

fn browser_response_body_command(
    url: &str,
    timeout_ms: Option<u64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-response-body-{}", uuid::Uuid::new_v4()),
        "action": "responsebody",
        "url": url,
    });
    if let Some(timeout_ms) = timeout_ms {
        command["timeoutMs"] = json!(timeout_ms);
    }
    apply_service_command_fields(
        &mut command,
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    command
}

fn browser_clipboard_command(
    operation: Option<&str>,
    text: Option<&str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = browser_action_command(
        "mcp-browser-clipboard",
        "clipboard",
        job_timeout_ms,
        service_name,
        agent_name,
        task_name,
    );
    if let Some(operation) = operation {
        command["operation"] = json!(operation);
    }
    if let Some(text) = text {
        command["text"] = json!(text);
    }
    command
}

fn browser_click_command(
    selector: &str,
    new_tab: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-click-{}", uuid::Uuid::new_v4()),
        "action": "click",
        "selector": selector,
    });
    if let Some(new_tab) = new_tab {
        command["newTab"] = json!(new_tab);
    }
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

fn browser_fill_command(
    selector: &str,
    value: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-fill-{}", uuid::Uuid::new_v4()),
        "action": "fill",
        "selector": selector,
        "value": value,
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

struct BrowserWaitCommandArgs<'a> {
    selector: Option<&'a str>,
    state: Option<&'a str>,
    text: Option<&'a str>,
    url: Option<&'a str>,
    function: Option<&'a str>,
    load_state: Option<&'a str>,
    timeout_ms: Option<u64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_wait_command(args: BrowserWaitCommandArgs<'_>) -> Result<Value, JsonRpcError> {
    let condition_count = [
        args.selector.is_some(),
        args.text.is_some(),
        args.url.is_some(),
        args.function.is_some(),
        args.load_state.is_some(),
    ]
    .into_iter()
    .filter(|has_condition| *has_condition)
    .count();

    if condition_count > 1 {
        return Err(JsonRpcError::invalid_params(
            "browser_wait accepts only one of selector, text, url, function, or loadState",
        ));
    }
    if args.state.is_some() && args.selector.is_none() {
        return Err(JsonRpcError::invalid_params(
            "state can only be used with selector",
        ));
    }
    if condition_count == 0 && args.timeout_ms.is_none() {
        return Err(JsonRpcError::invalid_params(
            "browser_wait requires a condition or timeoutMs",
        ));
    }

    let mut command = if let Some(url) = args.url {
        json!({
            "id": format!("mcp-browser-wait-{}", uuid::Uuid::new_v4()),
            "action": "waitforurl",
            "url": url,
        })
    } else if let Some(function) = args.function {
        json!({
            "id": format!("mcp-browser-wait-{}", uuid::Uuid::new_v4()),
            "action": "waitforfunction",
            "expression": function,
        })
    } else if let Some(load_state) = args.load_state {
        json!({
            "id": format!("mcp-browser-wait-{}", uuid::Uuid::new_v4()),
            "action": "waitforloadstate",
            "state": load_state,
        })
    } else {
        let mut command = json!({
            "id": format!("mcp-browser-wait-{}", uuid::Uuid::new_v4()),
            "action": "wait",
        });
        if let Some(selector) = args.selector {
            command["selector"] = json!(selector);
        }
        if let Some(state) = args.state {
            command["state"] = json!(state);
        }
        if let Some(text) = args.text {
            command["text"] = json!(text);
        }
        command
    };

    if let Some(timeout_ms) = args.timeout_ms {
        command["timeout"] = json!(timeout_ms);
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
    Ok(command)
}

struct BrowserTypeCommandArgs<'a> {
    selector: &'a str,
    text: &'a str,
    clear: Option<bool>,
    delay_ms: Option<u64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_type_command(args: BrowserTypeCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-type-{}", uuid::Uuid::new_v4()),
        "action": "type",
        "selector": args.selector,
        "text": args.text,
    });
    if let Some(clear) = args.clear {
        command["clear"] = json!(clear);
    }
    if let Some(delay_ms) = args.delay_ms {
        command["delay"] = json!(delay_ms);
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

fn browser_press_command(
    key: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-press-{}", uuid::Uuid::new_v4()),
        "action": "press",
        "key": key,
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

fn browser_hover_command(
    selector: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-hover-{}", uuid::Uuid::new_v4()),
        "action": "hover",
        "selector": selector,
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

fn browser_select_command(
    selector: &str,
    values: &[String],
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-select-{}", uuid::Uuid::new_v4()),
        "action": "select",
        "selector": selector,
        "values": values,
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

fn browser_checked_command(
    action: &str,
    selector: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-{}-{}", action, uuid::Uuid::new_v4()),
        "action": action,
        "selector": selector,
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

fn browser_element_state_command(
    action: &str,
    selector: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-{}-{}", action, uuid::Uuid::new_v4()),
        "action": action,
        "selector": selector,
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

struct BrowserElementReadCommandArgs<'a> {
    action: &'a str,
    selector: &'a str,
    attribute: Option<&'a str>,
    properties: Option<Vec<String>>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_element_read_command(args: BrowserElementReadCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-{}-{}", args.action, uuid::Uuid::new_v4()),
        "action": args.action,
        "selector": args.selector,
    });
    if let Some(attribute) = args.attribute {
        command["attribute"] = json!(attribute);
    }
    if let Some(properties) = args.properties {
        command["properties"] = json!(properties);
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

struct BrowserScrollCommandArgs<'a> {
    selector: Option<&'a str>,
    direction: Option<&'a str>,
    amount: Option<f64>,
    delta_x: Option<f64>,
    delta_y: Option<f64>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_scroll_command(args: BrowserScrollCommandArgs<'_>) -> Result<Value, JsonRpcError> {
    let uses_direction = args.direction.is_some() || args.amount.is_some();
    let uses_delta = args.delta_x.is_some() || args.delta_y.is_some();
    if uses_direction && uses_delta {
        return Err(JsonRpcError::invalid_params(
            "browser_scroll accepts direction/amount or deltaX/deltaY, not both",
        ));
    }

    let mut command = json!({
        "id": format!("mcp-browser-scroll-{}", uuid::Uuid::new_v4()),
        "action": "scroll",
    });
    if let Some(selector) = args.selector {
        command["selector"] = json!(selector);
    }
    if uses_delta {
        if let Some(delta_x) = args.delta_x {
            command["x"] = json!(delta_x);
        }
        if let Some(delta_y) = args.delta_y {
            command["y"] = json!(delta_y);
        }
    } else {
        command["direction"] = json!(args.direction.unwrap_or("down"));
        command["amount"] = json!(args.amount.unwrap_or(300.0));
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
    Ok(command)
}

fn browser_scroll_into_view_command(
    selector: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-scroll-into-view-{}", uuid::Uuid::new_v4()),
        "action": "scrollintoview",
        "selector": selector,
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

fn browser_field_command(
    action: &str,
    selector: &str,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-{}-{}", action, uuid::Uuid::new_v4()),
        "action": action,
        "selector": selector,
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

struct BrowserScreenshotCommandArgs<'a> {
    selector: Option<&'a str>,
    path: Option<&'a str>,
    full_page: Option<bool>,
    annotate: Option<bool>,
    format: Option<&'a str>,
    quality: Option<u64>,
    screenshot_dir: Option<&'a str>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
}

fn browser_screenshot_command(args: BrowserScreenshotCommandArgs<'_>) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-screenshot-{}", uuid::Uuid::new_v4()),
        "action": "screenshot",
    });
    if let Some(selector) = args.selector {
        command["selector"] = json!(selector);
    }
    if let Some(path) = args.path {
        command["path"] = json!(path);
    }
    if let Some(full_page) = args.full_page {
        command["fullPage"] = json!(full_page);
    }
    if let Some(annotate) = args.annotate {
        command["annotate"] = json!(annotate);
    }
    if let Some(format) = args.format {
        command["format"] = json!(format);
    }
    if let Some(quality) = args.quality {
        command["quality"] = json!(quality);
    }
    if let Some(screenshot_dir) = args.screenshot_dir {
        command["screenshotDir"] = json!(screenshot_dir);
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

fn browser_tabs_command(
    verbose: Option<bool>,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("mcp-browser-tabs-{}", uuid::Uuid::new_v4()),
        "action": "tab_list",
    });
    if let Some(verbose) = verbose {
        command["verbose"] = json!(verbose);
    }
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

fn browser_read_command(
    spec: BrowserReadToolSpec,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Value {
    let mut command = json!({
        "id": format!("{}-{}", spec.id_prefix, uuid::Uuid::new_v4()),
        "action": spec.action,
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

fn optional_enum_string_argument<'a>(
    arguments: &'a Value,
    name: &str,
    allowed: &[&str],
) -> Result<Option<&'a str>, JsonRpcError> {
    let value = optional_string_argument(arguments, name)?;
    if let Some(value) = value {
        if !allowed.contains(&value) {
            return Err(JsonRpcError::invalid_params(&format!(
                "{} must be one of {}",
                name,
                allowed.join(", ")
            )));
        }
    }
    Ok(value)
}

fn optional_profile_lease_policy_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    optional_enum_string_argument(arguments, "profileLeasePolicy", &["reject", "wait"])
}

fn required_string_argument<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, JsonRpcError> {
    optional_string_argument(arguments, name)?
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} is required", name)))
}

fn required_object_argument(arguments: &Value, name: &str) -> Result<Value, JsonRpcError> {
    let value = arguments
        .get(name)
        .cloned()
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} is required", name)))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(JsonRpcError::invalid_params(&format!(
            "{} must be a JSON object",
            name
        )))
    }
}

fn required_positive_u64_argument(arguments: &Value, name: &str) -> Result<u64, JsonRpcError> {
    optional_positive_u64_argument(arguments, name)?
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} is required", name)))
}

fn required_u64_argument(arguments: &Value, name: &str) -> Result<u64, JsonRpcError> {
    optional_u64_argument(arguments, name)?
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} is required", name)))
}

fn required_f64_argument(arguments: &Value, name: &str) -> Result<f64, JsonRpcError> {
    optional_f64_argument(arguments, name)?
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} is required", name)))
}

fn required_string_array_argument(
    arguments: &Value,
    name: &str,
) -> Result<Vec<String>, JsonRpcError> {
    let value = arguments
        .get(name)
        .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} is required", name)))?;
    let values = value.as_array().ok_or_else(|| {
        JsonRpcError::invalid_params(&format!("{} must be a non-empty array of strings", name))
    })?;
    if values.is_empty() {
        return Err(JsonRpcError::invalid_params(&format!(
            "{} must be a non-empty array of strings",
            name
        )));
    }

    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    JsonRpcError::invalid_params(&format!(
                        "{} must be a non-empty array of strings",
                        name
                    ))
                })
        })
        .collect()
}

fn optional_object_argument<'a>(
    arguments: &'a Value,
    name: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>, JsonRpcError> {
    match arguments.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_object()
            .map(Some)
            .ok_or_else(|| JsonRpcError::invalid_params(&format!("{} must be an object", name))),
        None => Ok(None),
    }
}

fn optional_string_array_argument(
    arguments: &Value,
    name: &str,
) -> Result<Option<Vec<String>>, JsonRpcError> {
    if arguments.get(name).is_none() {
        return Ok(None);
    }

    required_string_array_argument(arguments, name).map(Some)
}

fn optional_string_value_array_argument<'a>(
    arguments: &'a Value,
    name: &str,
) -> Result<Option<&'a [Value]>, JsonRpcError> {
    match arguments.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => {
            let values = value.as_array().ok_or_else(|| {
                JsonRpcError::invalid_params(&format!(
                    "{} must be a non-empty array of strings",
                    name
                ))
            })?;
            if values.is_empty()
                || values
                    .iter()
                    .any(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
            {
                return Err(JsonRpcError::invalid_params(&format!(
                    "{} must be a non-empty array of strings",
                    name
                )));
            }
            Ok(Some(values.as_slice()))
        }
        None => Ok(None),
    }
}

fn optional_cookie_array_argument(arguments: &Value) -> Result<Option<&[Value]>, JsonRpcError> {
    match arguments.get("cookies") {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => {
            let cookies = value.as_array().ok_or_else(|| {
                JsonRpcError::invalid_params("cookies must be a non-empty array of objects")
            })?;
            if cookies.is_empty() || cookies.iter().any(|cookie| !cookie.is_object()) {
                return Err(JsonRpcError::invalid_params(
                    "cookies must be a non-empty array of objects",
                ));
            }
            Ok(Some(cookies.as_slice()))
        }
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

fn optional_storage_type_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "type")? {
        Some("local") => Ok(Some("local")),
        Some("session") => Ok(Some("session")),
        Some(_) => Err(JsonRpcError::invalid_params(
            "type must be either local or session",
        )),
        None => Ok(None),
    }
}

fn optional_wait_until_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "waitUntil")? {
        Some(value) if matches!(value, "load" | "domcontentloaded" | "networkidle" | "none") => {
            Ok(Some(value))
        }
        Some(_) => Err(JsonRpcError::invalid_params(
            "waitUntil must be load, domcontentloaded, networkidle, or none",
        )),
        None => Ok(None),
    }
}

fn required_dialog_response_argument(arguments: &Value) -> Result<&str, JsonRpcError> {
    match required_string_argument(arguments, "response")? {
        "status" => Ok("status"),
        "accept" => Ok("accept"),
        "dismiss" => Ok("dismiss"),
        _ => Err(JsonRpcError::invalid_params(
            "response must be status, accept, or dismiss",
        )),
    }
}

fn optional_clipboard_operation_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "operation")? {
        Some(value) if matches!(value, "read" | "write" | "copy" | "paste") => Ok(Some(value)),
        Some(_) => Err(JsonRpcError::invalid_params(
            "operation must be read, write, copy, or paste",
        )),
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

fn optional_bounded_u64_argument(
    arguments: &Value,
    name: &str,
    min: u64,
    max: u64,
) -> Result<Option<u64>, JsonRpcError> {
    match optional_u64_argument(arguments, name)? {
        Some(value) if value < min || value > max => Err(JsonRpcError::invalid_params(&format!(
            "{} must be between {} and {}",
            name, min, max
        ))),
        value => Ok(value),
    }
}

fn optional_f64_argument(arguments: &Value, name: &str) -> Result<Option<f64>, JsonRpcError> {
    match arguments.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(Some)
            .ok_or_else(|| {
                JsonRpcError::invalid_params(&format!("{} must be a finite number", name))
            }),
        None => Ok(None),
    }
}

fn optional_non_negative_f64_argument(
    arguments: &Value,
    name: &str,
) -> Result<Option<f64>, JsonRpcError> {
    match optional_f64_argument(arguments, name)? {
        Some(value) if value < 0.0 => Err(JsonRpcError::invalid_params(&format!(
            "{} must be a non-negative number",
            name
        ))),
        value => Ok(value),
    }
}

fn optional_positive_f64_argument(
    arguments: &Value,
    name: &str,
) -> Result<Option<f64>, JsonRpcError> {
    match optional_f64_argument(arguments, name)? {
        Some(value) if value <= 0.0 => Err(JsonRpcError::invalid_params(&format!(
            "{} must be a positive number",
            name
        ))),
        value => Ok(value),
    }
}

fn optional_scroll_direction_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "direction")? {
        Some("up") => Ok(Some("up")),
        Some("down") => Ok(Some("down")),
        Some("left") => Ok(Some("left")),
        Some("right") => Ok(Some("right")),
        Some(_) => Err(JsonRpcError::invalid_params(
            "direction must be one of up, down, left, or right",
        )),
        None => Ok(None),
    }
}

fn optional_screenshot_format_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "format")? {
        Some("png") => Ok(Some("png")),
        Some("jpeg") => Ok(Some("jpeg")),
        Some(_) => Err(JsonRpcError::invalid_params(
            "format must be either png or jpeg",
        )),
        None => Ok(None),
    }
}

fn optional_selector_wait_state_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "state")? {
        Some("visible") => Ok(Some("visible")),
        Some("hidden") => Ok(Some("hidden")),
        Some("attached") => Ok(Some("attached")),
        Some("detached") => Ok(Some("detached")),
        Some(_) => Err(JsonRpcError::invalid_params(
            "state must be one of visible, hidden, attached, or detached",
        )),
        None => Ok(None),
    }
}

fn optional_load_state_argument(arguments: &Value) -> Result<Option<&str>, JsonRpcError> {
    match optional_string_argument(arguments, "loadState")? {
        Some("load") => Ok(Some("load")),
        Some("domcontentloaded") => Ok(Some("domcontentloaded")),
        Some("networkidle") => Ok(Some("networkidle")),
        Some("none") => Ok(Some("none")),
        Some(_) => Err(JsonRpcError::invalid_params(
            "loadState must be one of load, domcontentloaded, networkidle, or none",
        )),
        None => Ok(None),
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

#[derive(Clone, Copy)]
struct ServiceToolContext<'a> {
    job_timeout_ms: Option<u64>,
    profile_lease_policy: Option<&'a str>,
    profile_lease_wait_timeout_ms: Option<u64>,
    service_name: Option<&'a str>,
    agent_name: Option<&'a str>,
    task_name: Option<&'a str>,
    target_service_id: Option<&'a str>,
    target_service: Option<&'a str>,
    target_service_ids: Option<&'a [Value]>,
    target_services: Option<&'a [Value]>,
    site_id: Option<&'a str>,
    login_id: Option<&'a str>,
    site_ids: Option<&'a [Value]>,
    login_ids: Option<&'a [Value]>,
}

impl<'a> ServiceToolContext<'a> {
    fn from_arguments(arguments: &'a Value) -> Result<Self, JsonRpcError> {
        Ok(Self {
            job_timeout_ms: optional_positive_u64_argument(arguments, "jobTimeoutMs")?,
            profile_lease_policy: optional_profile_lease_policy_argument(arguments)?,
            profile_lease_wait_timeout_ms: optional_positive_u64_argument(
                arguments,
                "profileLeaseWaitTimeoutMs",
            )?,
            service_name: optional_string_argument(arguments, "serviceName")?,
            agent_name: optional_string_argument(arguments, "agentName")?,
            task_name: optional_string_argument(arguments, "taskName")?,
            target_service_id: optional_string_argument(arguments, "targetServiceId")?,
            target_service: optional_string_argument(arguments, "targetService")?,
            target_service_ids: optional_string_value_array_argument(
                arguments,
                "targetServiceIds",
            )?,
            target_services: optional_string_value_array_argument(arguments, "targetServices")?,
            site_id: optional_string_argument(arguments, "siteId")?,
            login_id: optional_string_argument(arguments, "loginId")?,
            site_ids: optional_string_value_array_argument(arguments, "siteIds")?,
            login_ids: optional_string_value_array_argument(arguments, "loginIds")?,
        })
    }

    fn trace(self) -> Value {
        let mut trace = service_tool_trace(self.service_name, self.agent_name, self.task_name);
        self.apply_target_profile_hints(&mut trace);
        trace
    }

    fn apply_target_profile_hints(self, command: &mut Value) {
        if let Some(profile_lease_policy) = self.profile_lease_policy {
            command["profileLeasePolicy"] = json!(profile_lease_policy);
        }
        if let Some(profile_lease_wait_timeout_ms) = self.profile_lease_wait_timeout_ms {
            command["profileLeaseWaitTimeoutMs"] = json!(profile_lease_wait_timeout_ms);
        }
        if let Some(target_service_id) = self.target_service_id {
            command["targetServiceId"] = json!(target_service_id);
        }
        if let Some(target_service) = self.target_service {
            command["targetService"] = json!(target_service);
        }
        if let Some(target_service_ids) = self.target_service_ids {
            command["targetServiceIds"] = json!(target_service_ids);
        }
        if let Some(target_services) = self.target_services {
            command["targetServices"] = json!(target_services);
        }
        if let Some(site_id) = self.site_id {
            command["siteId"] = json!(site_id);
        }
        if let Some(login_id) = self.login_id {
            command["loginId"] = json!(login_id);
        }
        if let Some(site_ids) = self.site_ids {
            command["siteIds"] = json!(site_ids);
        }
        if let Some(login_ids) = self.login_ids {
            command["loginIds"] = json!(login_ids);
        }
    }
}

fn apply_service_command_fields(
    command: &mut Value,
    job_timeout_ms: Option<u64>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) {
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
}

fn send_queued_tool_command(
    tool_name: &str,
    session: &str,
    trace: Value,
    mut command: Value,
) -> Result<Value, JsonRpcError> {
    if tool_name.starts_with("browser_") {
        copy_target_profile_hints(&trace, &mut command);
    }
    let response = send_command(command, session).map_err(|err| JsonRpcError {
        code: -32603,
        message: "Internal error",
        data: Some(json!({
            "message": err,
            "session": session,
            "tool": tool_name,
            "trace": trace.clone(),
        })),
    })?;
    Ok(tool_response_from_daemon(
        tool_name, session, trace, response,
    ))
}

fn copy_target_profile_hints(source: &Value, command: &mut Value) {
    for key in [
        "profileLeasePolicy",
        "profileLeaseWaitTimeoutMs",
        "targetServiceId",
        "targetService",
        "targetServiceIds",
        "targetServices",
        "siteId",
        "loginId",
        "siteIds",
        "loginIds",
    ] {
        if let Some(value) = source.get(key) {
            command[key] = value.clone();
        }
    }
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

fn tool_response_from_payload(
    tool_name: &str,
    session: &str,
    trace: Value,
    data: Value,
    is_error: bool,
) -> Value {
    let payload = json!({
        "tool": tool_name,
        "session": session,
        "trace": trace,
        "success": !is_error,
        "data": data,
        "error": Value::Null,
    });
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&payload).unwrap_or_default(),
            }
        ],
        "isError": is_error,
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

#[derive(Debug)]
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

fn access_plan_resource_query(uri: &str) -> Option<Vec<(String, String)>> {
    let query = uri
        .strip_prefix(SERVICE_ACCESS_PLAN_MCP_RESOURCE)
        .and_then(|rest| rest.strip_prefix('?'))?;
    Some(
        url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_resources_lists_read_only_service_resources() {
        let response = mcp_command_response(&["mcp".to_string(), "resources".to_string()]).unwrap();

        assert_eq!(response["success"], true);
        assert_eq!(
            response["data"]["resources"][0]["uri"],
            SERVICE_CONTRACTS_RESOURCE
        );
        assert_eq!(
            response["data"]["resources"][1]["uri"],
            SERVICE_ACCESS_PLAN_MCP_RESOURCE
        );
        assert_eq!(response["data"]["resources"][2]["uri"], INCIDENTS_RESOURCE);
        assert_eq!(response["data"]["resources"][3]["uri"], PROFILES_RESOURCE);
        assert_eq!(response["data"]["resources"][4]["uri"], SESSIONS_RESOURCE);
        assert_eq!(response["data"]["resources"][5]["uri"], BROWSERS_RESOURCE);
        assert_eq!(response["data"]["resources"][6]["uri"], TABS_RESOURCE);
        assert_eq!(response["data"]["resources"][7]["uri"], MONITORS_RESOURCE);
        assert_eq!(
            response["data"]["resources"][8]["uri"],
            SITE_POLICIES_RESOURCE
        );
        assert_eq!(response["data"]["resources"][9]["uri"], PROVIDERS_RESOURCE);
        assert_eq!(
            response["data"]["resources"][10]["uri"],
            CHALLENGES_RESOURCE
        );
        assert_eq!(response["data"]["resources"][11]["uri"], JOBS_RESOURCE);
        assert_eq!(response["data"]["resources"][12]["uri"], EVENTS_RESOURCE);
        assert_eq!(
            response["data"]["resourceTemplates"][0]["uriTemplate"],
            ACCESS_PLAN_TEMPLATE
        );
        assert_eq!(
            response["data"]["resourceTemplates"][1]["uriTemplate"],
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
            SERVICE_CONTRACTS_RESOURCE
        );
        assert_eq!(
            response["result"]["resources"][1]["uri"],
            SERVICE_ACCESS_PLAN_MCP_RESOURCE
        );
        assert_eq!(
            response["result"]["resources"][2]["uri"],
            INCIDENTS_RESOURCE
        );
        assert_eq!(response["result"]["resources"][3]["uri"], PROFILES_RESOURCE);
        assert_eq!(response["result"]["resources"][4]["uri"], SESSIONS_RESOURCE);
        assert_eq!(response["result"]["resources"][5]["uri"], BROWSERS_RESOURCE);
        assert_eq!(response["result"]["resources"][6]["uri"], TABS_RESOURCE);
        assert_eq!(response["result"]["resources"][7]["uri"], MONITORS_RESOURCE);
        assert_eq!(
            response["result"]["resources"][8]["uri"],
            SITE_POLICIES_RESOURCE
        );
        assert_eq!(
            response["result"]["resources"][9]["uri"],
            PROVIDERS_RESOURCE
        );
        assert_eq!(
            response["result"]["resources"][10]["uri"],
            CHALLENGES_RESOURCE
        );
        assert_eq!(response["result"]["resources"][11]["uri"], JOBS_RESOURCE);
        assert_eq!(response["result"]["resources"][12]["uri"], EVENTS_RESOURCE);
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
            ACCESS_PLAN_TEMPLATE
        );
        assert_eq!(
            response["result"]["resourceTemplates"][1]["uriTemplate"],
            "agent-browser://incidents/{incident_id}/activity"
        );
    }

    #[test]
    fn read_contracts_resource_returns_service_request_metadata() {
        let resource = read_service_mcp_resource_from_state(
            SERVICE_CONTRACTS_RESOURCE,
            &ServiceState::default(),
        )
        .unwrap();

        assert_eq!(resource["uri"], SERVICE_CONTRACTS_RESOURCE);
        assert_eq!(resource["contents"]["schemaVersion"], "v1");
        assert_eq!(
            resource["contents"]["contracts"]["serviceRequest"]["http"]["route"],
            "/api/service/request"
        );
        assert_eq!(
            resource["contents"]["contracts"]["serviceRequest"]["mcp"]["tool"],
            "service_request"
        );
        assert_eq!(
            resource["contents"]["contracts"]["serviceAccessPlanResponse"]["mcp"]["resource"],
            SERVICE_ACCESS_PLAN_MCP_RESOURCE
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
        let mut response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"tools","method":"tools/list"}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], "tools");
        let tools = response["result"]["tools"].as_array().unwrap();
        let service_job_cancel = tools
            .iter()
            .find(|tool| tool["name"] == "service_job_cancel")
            .expect("service_job_cancel schema should be listed");
        assert_eq!(service_job_cancel["inputSchema"]["required"][0], "jobId");
        assert!(service_job_cancel["inputSchema"]["properties"]["serviceName"].is_object());
        assert!(service_job_cancel["inputSchema"]["properties"]["agentName"].is_object());
        assert!(service_job_cancel["inputSchema"]["properties"]["taskName"].is_object());
        let service_browser_retry = tools
            .iter()
            .find(|tool| tool["name"] == "service_browser_retry")
            .expect("service_browser_retry schema should be listed");
        assert!(service_browser_retry["inputSchema"]["required"][0] == "browserId");
        assert!(service_browser_retry["inputSchema"]["properties"]["serviceName"].is_object());
        let service_incidents = tools
            .iter()
            .find(|tool| tool["name"] == "service_incidents")
            .expect("service_incidents schema should be listed");
        assert!(
            service_incidents["inputSchema"]["properties"]["severity"]["enum"]
                .as_array()
                .unwrap()
                .contains(&json!("critical"))
        );
        assert!(
            service_incidents["inputSchema"]["properties"]["escalation"]["enum"]
                .as_array()
                .unwrap()
                .contains(&json!("os_degraded_possible"))
        );
        assert!(service_incidents["inputSchema"]["properties"]["summary"].is_object());
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_profile_upsert"));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_profile_freshness_update"
                && tool["inputSchema"]["properties"]["readinessState"]["enum"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("stale"))));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_session_upsert"));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_site_policy_upsert"));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_monitor_upsert"
                && tool["inputSchema"]["properties"]["monitor"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_monitors_run_due"));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_monitor_pause"
                && tool["inputSchema"]["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("id"))));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_monitor_resume"));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "service_provider_upsert"));
        let browser_and_trace_tools: Vec<Value> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|tool| {
                let name = tool["name"].as_str().unwrap_or_default();
                name == "service_job_cancel"
                    || name.starts_with("browser_")
                    || name == "service_trace"
            })
            .cloned()
            .collect();
        response["result"]["tools"] = Value::Array(browser_and_trace_tools);
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
        assert_eq!(response["result"]["tools"][4]["name"], "browser_tabs");
        assert!(response["result"]["tools"][4]["inputSchema"]["properties"]["verbose"].is_object());
        assert!(
            response["result"]["tools"][4]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert!(
            response["result"]["tools"][4]["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
        );
        assert_eq!(response["result"]["tools"][5]["name"], "browser_screenshot");
        assert!(
            response["result"]["tools"][5]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(response["result"]["tools"][5]["inputSchema"]["properties"]["format"].is_object());
        assert!(
            response["result"]["tools"][5]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][6]["name"], "browser_click");
        assert_eq!(
            response["result"]["tools"][6]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(response["result"]["tools"][6]["inputSchema"]["properties"]["newTab"].is_object());
        assert!(
            response["result"]["tools"][6]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][7]["name"], "browser_fill");
        assert_eq!(
            response["result"]["tools"][7]["inputSchema"]["required"][0],
            "selector"
        );
        assert_eq!(
            response["result"]["tools"][7]["inputSchema"]["required"][1],
            "value"
        );
        assert!(response["result"]["tools"][7]["inputSchema"]["properties"]["value"].is_object());
        assert!(
            response["result"]["tools"][7]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][8]["name"], "browser_wait");
        assert!(
            response["result"]["tools"][8]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(response["result"]["tools"][8]["inputSchema"]["properties"]["text"].is_object());
        assert!(
            response["result"]["tools"][8]["inputSchema"]["properties"]["loadState"].is_object()
        );
        assert!(
            response["result"]["tools"][8]["inputSchema"]["properties"]["timeoutMs"].is_object()
        );
        assert!(
            response["result"]["tools"][8]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][9]["name"], "browser_type");
        assert_eq!(
            response["result"]["tools"][9]["inputSchema"]["required"][0],
            "selector"
        );
        assert_eq!(
            response["result"]["tools"][9]["inputSchema"]["required"][1],
            "text"
        );
        assert!(response["result"]["tools"][9]["inputSchema"]["properties"]["clear"].is_object());
        assert!(response["result"]["tools"][9]["inputSchema"]["properties"]["delayMs"].is_object());
        assert!(
            response["result"]["tools"][9]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][10]["name"], "browser_press");
        assert_eq!(
            response["result"]["tools"][10]["inputSchema"]["required"][0],
            "key"
        );
        assert!(response["result"]["tools"][10]["inputSchema"]["properties"]["key"].is_object());
        assert!(
            response["result"]["tools"][10]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][10]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][11]["name"], "browser_hover");
        assert_eq!(
            response["result"]["tools"][11]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][11]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][11]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][11]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][12]["name"], "browser_select");
        assert_eq!(
            response["result"]["tools"][12]["inputSchema"]["required"][0],
            "selector"
        );
        assert_eq!(
            response["result"]["tools"][12]["inputSchema"]["required"][1],
            "values"
        );
        assert!(response["result"]["tools"][12]["inputSchema"]["properties"]["values"].is_object());
        assert!(
            response["result"]["tools"][12]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][13]["name"], "browser_get_text");
        assert_eq!(
            response["result"]["tools"][13]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][13]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][13]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][13]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][14]["name"], "browser_get_value");
        assert_eq!(
            response["result"]["tools"][14]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][14]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][14]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][14]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(
            response["result"]["tools"][15]["name"],
            "browser_get_attribute"
        );
        assert_eq!(
            response["result"]["tools"][15]["inputSchema"]["required"][0],
            "selector"
        );
        assert_eq!(
            response["result"]["tools"][15]["inputSchema"]["required"][1],
            "attribute"
        );
        assert!(
            response["result"]["tools"][15]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][15]["inputSchema"]["properties"]["attribute"].is_object()
        );
        assert!(
            response["result"]["tools"][15]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert_eq!(response["result"]["tools"][16]["name"], "browser_get_html");
        assert_eq!(
            response["result"]["tools"][16]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][16]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][16]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert_eq!(
            response["result"]["tools"][17]["name"],
            "browser_get_styles"
        );
        assert_eq!(
            response["result"]["tools"][17]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][17]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][17]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][17]["inputSchema"]["properties"]["properties"].is_object()
        );
        assert_eq!(response["result"]["tools"][18]["name"], "browser_count");
        assert_eq!(
            response["result"]["tools"][18]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][18]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][18]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert_eq!(response["result"]["tools"][19]["name"], "browser_get_box");
        assert_eq!(
            response["result"]["tools"][19]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][19]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][19]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert_eq!(
            response["result"]["tools"][20]["name"],
            "browser_is_visible"
        );
        assert_eq!(
            response["result"]["tools"][20]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][20]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][20]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][20]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(
            response["result"]["tools"][21]["name"],
            "browser_is_enabled"
        );
        assert_eq!(
            response["result"]["tools"][21]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][21]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][21]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][21]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][22]["name"], "browser_check");
        assert_eq!(
            response["result"]["tools"][22]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][22]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][22]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(
            response["result"]["tools"][23]["name"],
            "browser_is_checked"
        );
        assert_eq!(
            response["result"]["tools"][23]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][23]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][23]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][23]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][24]["name"], "browser_uncheck");
        assert_eq!(
            response["result"]["tools"][24]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][24]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][24]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][25]["name"], "browser_scroll");
        assert!(
            response["result"]["tools"][25]["inputSchema"]["properties"]["direction"].is_object()
        );
        assert!(response["result"]["tools"][25]["inputSchema"]["properties"]["amount"].is_object());
        assert!(response["result"]["tools"][25]["inputSchema"]["properties"]["deltaX"].is_object());
        assert!(response["result"]["tools"][25]["inputSchema"]["properties"]["deltaY"].is_object());
        assert!(
            response["result"]["tools"][25]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][25]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(
            response["result"]["tools"][26]["name"],
            "browser_scroll_into_view"
        );
        assert_eq!(
            response["result"]["tools"][26]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][26]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][26]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][26]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][27]["name"], "browser_focus");
        assert_eq!(
            response["result"]["tools"][27]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][27]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][27]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][27]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert_eq!(response["result"]["tools"][28]["name"], "browser_clear");
        assert_eq!(
            response["result"]["tools"][28]["inputSchema"]["required"][0],
            "selector"
        );
        assert!(
            response["result"]["tools"][28]["inputSchema"]["properties"]["selector"].is_object()
        );
        assert!(
            response["result"]["tools"][28]["inputSchema"]["properties"]["jobTimeoutMs"]
                .is_object()
        );
        assert!(
            response["result"]["tools"][28]["inputSchema"]["properties"]["serviceName"].is_object()
        );
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_navigate"
                && tool["inputSchema"]["required"][0] == "url"
                && tool["inputSchema"]["properties"]["waitUntil"].is_object()
                && tool["inputSchema"]["properties"]["headers"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_requests"
                && tool["inputSchema"]["properties"]["filter"].is_object()
                && tool["inputSchema"]["properties"]["method"].is_object()
                && tool["inputSchema"]["properties"]["status"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_request_detail"
                && tool["inputSchema"]["required"][0] == "requestId"
                && tool["inputSchema"]["properties"]["requestId"].is_object()
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_headers"
                && tool["inputSchema"]["required"][0] == "headers"
                && tool["inputSchema"]["properties"]["headers"].is_object()
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_offline"
                && tool["inputSchema"]["properties"]["offline"].is_object()
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_cookies_get"
                && tool["inputSchema"]["properties"]["urls"].is_object()
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_cookies_set"
                && tool["inputSchema"]["properties"]["cookies"].is_object()
                && tool["inputSchema"]["properties"]["name"].is_object()
                && tool["inputSchema"]["properties"]["value"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_cookies_clear"
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_storage_get"
                && tool["inputSchema"]["properties"]["type"].is_object()
                && tool["inputSchema"]["properties"]["key"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_storage_set"
                && tool["inputSchema"]["required"][0] == "key"
                && tool["inputSchema"]["required"][1] == "value"
                && tool["inputSchema"]["properties"]["type"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_storage_clear"
                && tool["inputSchema"]["properties"]["type"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_user_agent"
                && tool["inputSchema"]["required"][0] == "userAgent"
                && tool["inputSchema"]["properties"]["userAgent"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_viewport"
                && tool["inputSchema"]["required"][0] == "width"
                && tool["inputSchema"]["required"][1] == "height"
                && tool["inputSchema"]["properties"]["deviceScaleFactor"].is_object()
                && tool["inputSchema"]["properties"]["mobile"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_geolocation"
                && tool["inputSchema"]["required"][0] == "latitude"
                && tool["inputSchema"]["required"][1] == "longitude"
                && tool["inputSchema"]["properties"]["accuracy"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_permissions"
                && tool["inputSchema"]["required"][0] == "permissions"
                && tool["inputSchema"]["properties"]["permissions"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_timezone"
                && tool["inputSchema"]["required"][0] == "timezoneId"
                && tool["inputSchema"]["properties"]["timezoneId"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_locale"
                && tool["inputSchema"]["required"][0] == "locale"
                && tool["inputSchema"]["properties"]["locale"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_media"
                && tool["inputSchema"]["properties"]["media"].is_object()
                && tool["inputSchema"]["properties"]["colorScheme"].is_object()
                && tool["inputSchema"]["properties"]["reducedMotion"].is_object()
                && tool["inputSchema"]["properties"]["features"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_dialog"
                && tool["inputSchema"]["required"][0] == "response"
                && tool["inputSchema"]["properties"]["promptText"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_upload"
                && tool["inputSchema"]["required"][0] == "selector"
                && tool["inputSchema"]["required"][1] == "files"
                && tool["inputSchema"]["properties"]["files"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_download"
                && tool["inputSchema"]["required"][0] == "selector"
                && tool["inputSchema"]["required"][1] == "path"
                && tool["inputSchema"]["properties"]["path"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_wait_for_download"
                && tool["inputSchema"]["properties"]["path"].is_object()
                && tool["inputSchema"]["properties"]["timeoutMs"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_har_start"
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_har_stop"
                && tool["inputSchema"]["properties"]["path"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_route"
                && tool["inputSchema"]["required"][0] == "url"
                && tool["inputSchema"]["properties"]["response"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_unroute"
                && tool["inputSchema"]["properties"]["url"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_console"
                && tool["inputSchema"]["properties"]["clear"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_errors"
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_pdf"
                && tool["inputSchema"]["properties"]["path"].is_object()
                && tool["inputSchema"]["properties"]["printBackground"].is_object()
                && tool["inputSchema"]["properties"]["landscape"].is_object()
                && tool["inputSchema"]["properties"]["preferCSSPageSize"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_response_body"
                && tool["inputSchema"]["required"][0] == "url"
                && tool["inputSchema"]["properties"]["timeoutMs"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_clipboard"
                && tool["inputSchema"]["properties"]["operation"].is_object()
                && tool["inputSchema"]["properties"]["text"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_back"
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_reload"
                && tool["inputSchema"]["properties"]["jobTimeoutMs"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_tab_new"
                && tool["inputSchema"]["properties"]["url"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_tab_switch"
                && tool["inputSchema"]["required"][0] == "index"
                && tool["inputSchema"]["properties"]["index"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_tab_close"
                && tool["inputSchema"]["properties"]["index"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_set_content"
                && tool["inputSchema"]["required"][0] == "html"
                && tool["inputSchema"]["properties"]["html"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        assert!(response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "browser_command"
                && tool["inputSchema"]["properties"]["action"].is_object()
                && tool["inputSchema"]["properties"]["params"].is_object()
                && tool["inputSchema"]["properties"]["serviceName"].is_object()));
        let trace_tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .last()
            .unwrap();
        assert_eq!(trace_tool["name"], "service_trace");
        assert!(trace_tool["inputSchema"]["properties"]["limit"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["browserId"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["profileId"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["sessionId"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["serviceName"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["agentName"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["taskName"].is_object());
        assert!(trace_tool["inputSchema"]["properties"]["since"].is_object());
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
    fn service_trace_rejects_invalid_limit_before_store_read() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"service_trace","arguments":{"limit":0,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 40);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn service_incidents_rejects_invalid_severity_before_store_read() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"service_incidents","arguments":{"severity":"panic","escalation":"os_degraded_possible"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 42);
        assert_eq!(response["error"]["code"], -32602);
        assert!(response["error"]["data"]["message"]
            .as_str()
            .unwrap()
            .contains("severity must be one of"));
    }

    #[test]
    fn service_incidents_rejects_invalid_summary_before_store_read() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"service_incidents","arguments":{"summary":"yes"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 43);
        assert_eq!(response["error"]["code"], -32602);
        assert!(response["error"]["data"]["message"]
            .as_str()
            .unwrap()
            .contains("summary must be a boolean"));
    }

    #[test]
    fn service_config_tools_validate_required_args_before_daemon_call() {
        let missing_profile = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"profile","method":"tools/call","params":{"name":"service_profile_upsert","arguments":{"id":"journal-downloader"}}}"#,
            "default",
        )
        .unwrap();
        assert_eq!(missing_profile["id"], "profile");
        assert_eq!(missing_profile["error"]["code"], -32602);

        let missing_session = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"session","method":"tools/call","params":{"name":"service_session_upsert","arguments":{"id":"journal-run"}}}"#,
            "default",
        )
        .unwrap();
        assert_eq!(missing_session["id"], "session");
        assert_eq!(missing_session["error"]["code"], -32602);

        let missing_policy = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"policy","method":"tools/call","params":{"name":"service_site_policy_upsert","arguments":{"id":"google"}}}"#,
            "default",
        )
        .unwrap();
        assert_eq!(missing_policy["id"], "policy");
        assert_eq!(missing_policy["error"]["code"], -32602);

        let missing_monitor = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"monitor","method":"tools/call","params":{"name":"service_monitor_upsert","arguments":{"id":"google-login-freshness"}}}"#,
            "default",
        )
        .unwrap();
        assert_eq!(missing_monitor["id"], "monitor");
        assert_eq!(missing_monitor["error"]["code"], -32602);

        let missing_provider = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"provider","method":"tools/call","params":{"name":"service_provider_upsert","arguments":{"id":"manual"}}}"#,
            "default",
        )
        .unwrap();
        assert_eq!(missing_provider["id"], "provider");
        assert_eq!(missing_provider["error"]["code"], -32602);
    }

    #[test]
    fn browser_command_rejects_unsupported_action_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"browser_command","arguments":{"action":"service_status","params":{}}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 41);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_navigate_requires_url_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"browser_navigate","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 42);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_navigate_rejects_invalid_wait_until_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"browser_navigate","arguments":{"url":"https://example.com","waitUntil":"paint","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 43);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_requests_rejects_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":44,"method":"tools/call","params":{"name":"browser_requests","arguments":{"clear":"true","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 44);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":45,"method":"tools/call","params":{"name":"browser_requests","arguments":{"jobTimeoutMs":0,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 45);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_request_detail_requires_request_id_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":46,"method":"tools/call","params":{"name":"browser_request_detail","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 46);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_headers_requires_headers_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":47,"method":"tools/call","params":{"name":"browser_headers","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 47);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":48,"method":"tools/call","params":{"name":"browser_headers","arguments":{"headers":"X-Test: ok","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 48);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_offline_rejects_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":49,"method":"tools/call","params":{"name":"browser_offline","arguments":{"offline":"true","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 49);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_cookies_tools_reject_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"browser_cookies_get","arguments":{"urls":"https://example.com","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 50);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"browser_cookies_set","arguments":{"name":"session","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 51);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":52,"method":"tools/call","params":{"name":"browser_cookies_set","arguments":{"cookies":["bad"],"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 52);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_storage_tools_reject_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":53,"method":"tools/call","params":{"name":"browser_storage_get","arguments":{"type":"indexeddb","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 53);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":54,"method":"tools/call","params":{"name":"browser_storage_set","arguments":{"key":"token","type":"local","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 54);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":55,"method":"tools/call","params":{"name":"browser_storage_clear","arguments":{"type":"cache","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 55);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_session_shaping_tools_reject_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":56,"method":"tools/call","params":{"name":"browser_user_agent","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 56);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":57,"method":"tools/call","params":{"name":"browser_viewport","arguments":{"width":0,"height":720,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 57);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":58,"method":"tools/call","params":{"name":"browser_viewport","arguments":{"width":1280,"height":720,"deviceScaleFactor":0,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 58);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":59,"method":"tools/call","params":{"name":"browser_geolocation","arguments":{"latitude":41.8781,"accuracy":-1,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 59);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":60,"method":"tools/call","params":{"name":"browser_permissions","arguments":{"permissions":[],"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 60);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":61,"method":"tools/call","params":{"name":"browser_timezone","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 61);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":62,"method":"tools/call","params":{"name":"browser_locale","arguments":{"locale":"","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 62);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":63,"method":"tools/call","params":{"name":"browser_media","arguments":{"features":["bad"],"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 63);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":64,"method":"tools/call","params":{"name":"browser_dialog","arguments":{"response":"close","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 64);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":65,"method":"tools/call","params":{"name":"browser_upload","arguments":{"selector":"#file","files":[],"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 65);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":66,"method":"tools/call","params":{"name":"browser_download","arguments":{"selector":"#download","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 66);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":67,"method":"tools/call","params":{"name":"browser_wait_for_download","arguments":{"timeoutMs":0,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 67);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":68,"method":"tools/call","params":{"name":"browser_route","arguments":{"response":[],"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 68);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":69,"method":"tools/call","params":{"name":"browser_console","arguments":{"clear":"true","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 69);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":70,"method":"tools/call","params":{"name":"browser_pdf","arguments":{"landscape":"false","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 70);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":71,"method":"tools/call","params":{"name":"browser_response_body","arguments":{"timeoutMs":0,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 71);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":72,"method":"tools/call","params":{"name":"browser_clipboard","arguments":{"operation":"write","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 72);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":73,"method":"tools/call","params":{"name":"browser_clipboard","arguments":{"operation":"cut","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 73);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_tab_and_content_tools_reject_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":74,"method":"tools/call","params":{"name":"browser_tab_new","arguments":{"url":42,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 74);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":75,"method":"tools/call","params":{"name":"browser_tab_switch","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 75);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":76,"method":"tools/call","params":{"name":"browser_tab_switch","arguments":{"index":-1,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 76);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":77,"method":"tools/call","params":{"name":"browser_tab_close","arguments":{"index":"0","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 77);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":78,"method":"tools/call","params":{"name":"browser_set_content","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 78);
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
    fn browser_tabs_rejects_invalid_argument_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"browser_tabs","arguments":{"verbose":"yes"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 8);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_screenshot_rejects_invalid_argument_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"browser_screenshot","arguments":{"format":"webp"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 9);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"browser_screenshot","arguments":{"quality":101}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 10);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_click_requires_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"browser_click","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 11);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_fill_requires_selector_and_value_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"browser_fill","arguments":{"value":"Ada Lovelace","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 12);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"browser_fill","arguments":{"selector":"#name","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 13);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_wait_rejects_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"browser_wait","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 14);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"browser_wait","arguments":{"selector":"#name","text":"Ada Lovelace","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 15);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"browser_wait","arguments":{"selector":"#name","state":"gone","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 16);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"browser_wait","arguments":{"state":"visible","timeoutMs":1000,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 17);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_type_requires_selector_and_text_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"browser_type","arguments":{"text":" Jr","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 18);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":19,"method":"tools/call","params":{"name":"browser_type","arguments":{"selector":"#name","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 19);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"browser_type","arguments":{"selector":"#name","text":" Jr","clear":"false","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 20);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_press_requires_key_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"browser_press","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 21);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_hover_requires_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"browser_hover","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 22);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_select_requires_selector_and_values_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"browser_select","arguments":{"values":["org-a"],"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 23);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{"name":"browser_select","arguments":{"selector":"#org","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 24);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":25,"method":"tools/call","params":{"name":"browser_select","arguments":{"selector":"#org","values":"org-a","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 25);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_element_read_tools_require_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":37,"method":"tools/call","params":{"name":"browser_get_text","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 37);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":38,"method":"tools/call","params":{"name":"browser_get_value","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 38);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":39,"method":"tools/call","params":{"name":"browser_get_attribute","arguments":{"selector":"#link","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 39);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"browser_get_html","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 42);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"browser_get_styles","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 43);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r##"{"jsonrpc":"2.0","id":44,"method":"tools/call","params":{"name":"browser_get_styles","arguments":{"selector":"#ready","properties":"display","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"##,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 44);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"browser_count","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 40);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"browser_get_box","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 41);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_element_state_tools_require_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":35,"method":"tools/call","params":{"name":"browser_is_visible","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 35);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":36,"method":"tools/call","params":{"name":"browser_is_enabled","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 36);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_check_and_uncheck_require_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":26,"method":"tools/call","params":{"name":"browser_check","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 26);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":27,"method":"tools/call","params":{"name":"browser_uncheck","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 27);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_is_checked_requires_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":34,"method":"tools/call","params":{"name":"browser_is_checked","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 34);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_scroll_rejects_invalid_arguments_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":28,"method":"tools/call","params":{"name":"browser_scroll","arguments":{"direction":"diagonal","serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 28);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":29,"method":"tools/call","params":{"name":"browser_scroll","arguments":{"amount":-1,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 29);
        assert_eq!(response["error"]["code"], -32602);

        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"browser_scroll","arguments":{"direction":"down","deltaY":100,"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 30);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_scroll_into_view_requires_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{"name":"browser_scroll_into_view","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 31);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_focus_requires_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"browser_focus","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 32);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn browser_clear_requires_selector_before_daemon_call() {
        let response = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"browser_clear","arguments":{"serviceName":"JournalDownloader","agentName":"agent-a","taskName":"probeACSwebsite"}}}"#,
            "default",
        )
        .unwrap();

        assert_eq!(response["id"], 33);
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
    fn browser_tool_schemas_include_target_profile_hints() {
        let tools = service_mcp_tools();
        let service_request = tools
            .iter()
            .find(|tool| tool["name"] == "service_request")
            .expect("service_request schema should be listed");
        let navigate = tools
            .iter()
            .find(|tool| tool["name"] == "browser_navigate")
            .expect("browser_navigate schema should be listed");
        let service_trace = tools
            .iter()
            .find(|tool| tool["name"] == "service_trace")
            .expect("service_trace schema should be listed");

        assert!(navigate["inputSchema"]["properties"]["targetServiceId"].is_object());
        assert!(navigate["inputSchema"]["properties"]["targetServiceIds"].is_object());
        assert!(navigate["inputSchema"]["properties"]["siteId"].is_object());
        assert!(navigate["inputSchema"]["properties"]["loginIds"].is_object());
        assert!(service_request["inputSchema"]["properties"]["siteId"].is_object());
        assert!(service_request["inputSchema"]["properties"]["loginIds"].is_object());
        assert!(service_trace["inputSchema"]["properties"]["targetServiceId"].is_null());
        assert!(service_trace["inputSchema"]["properties"]["siteId"].is_null());
    }

    #[test]
    fn service_request_schema_and_command_accept_contract_actions() {
        let tools = service_mcp_tools();
        let service_request = tools
            .iter()
            .find(|tool| tool["name"] == "service_request")
            .expect("service_request schema should be listed");

        assert_eq!(
            service_request["inputSchema"]["properties"]["action"]["enum"],
            json!(SERVICE_REQUEST_ACTIONS)
        );

        for action in SERVICE_REQUEST_ACTIONS {
            let (_, command) = service_request_command(&json!({
                "action": action,
                "serviceName": "JournalDownloader",
                "agentName": "agent-a",
                "taskName": "probeACSwebsite"
            }))
            .unwrap_or_else(|err| panic!("service_request should accept {action}: {err:?}"));

            assert_eq!(command["action"], *action);
            assert!(command["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("mcp-service-request-")));
        }
    }

    #[test]
    fn service_request_command_builds_intent_command() {
        let (trace, command) = service_request_command(&json!({
            "action": "navigate",
            "params": {
                "url": "https://example.com",
                "id": "ignored",
                "action": "ignored"
            },
            "serviceName": "JournalDownloader",
            "agentName": "agent-a",
            "taskName": "probeACSwebsite",
            "siteId": "acs",
            "loginIds": ["orcid"],
            "jobTimeoutMs": 1000,
            "profileLeasePolicy": "wait",
            "profileLeaseWaitTimeoutMs": 2500
        }))
        .unwrap();

        assert_eq!(trace["serviceName"], "JournalDownloader");
        assert_eq!(trace["siteId"], "acs");
        assert_eq!(command["action"], "navigate");
        assert!(command["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("mcp-service-request-navigate-")));
        assert_eq!(command["url"], "https://example.com");
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
        assert_eq!(command["siteId"], "acs");
        assert_eq!(command["loginIds"][0], "orcid");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["profileLeasePolicy"], "wait");
        assert_eq!(command["profileLeaseWaitTimeoutMs"], 2500);
    }

    #[test]
    fn service_profile_freshness_command_builds_mutation_command() {
        let freshness = service_profile_freshness_arguments(&json!({
            "id": "journal-google",
            "loginId": "google",
            "readinessState": "stale",
            "readinessEvidence": "auth_probe_cookie_missing",
            "serviceName": "JournalDownloader"
        }));
        let command = service_profile_freshness_command(
            "journal-google",
            &freshness,
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeGoogleLogin"),
        );

        assert_eq!(command["action"], "service_profile_freshness_update");
        assert_eq!(command["profileId"], "journal-google");
        assert_eq!(command["freshness"]["loginId"], "google");
        assert_eq!(command["freshness"]["readinessState"], "stale");
        assert!(command["freshness"].get("serviceName").is_none());
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeGoogleLogin");
    }

    #[test]
    fn service_tool_context_copies_target_profile_hints_to_browser_commands() {
        let arguments = json!({
            "serviceName": "JournalDownloader",
            "agentName": "agent-a",
            "taskName": "probeACSwebsite",
            "targetServiceId": "google",
            "targetServices": ["acs", "microsoft"],
            "siteId": "nih",
            "loginIds": ["orcid"],
            "profileLeasePolicy": "wait",
            "profileLeaseWaitTimeoutMs": 2500
        });
        let context = ServiceToolContext::from_arguments(&arguments).unwrap();
        let trace = context.trace();
        let mut command = json!({
            "action": "navigate",
            "url": "https://example.com"
        });

        copy_target_profile_hints(&trace, &mut command);

        assert_eq!(trace["targetServiceId"], "google");
        assert_eq!(trace["targetServices"][0], "acs");
        assert_eq!(trace["siteId"], "nih");
        assert_eq!(trace["loginIds"][0], "orcid");
        assert_eq!(trace["profileLeasePolicy"], "wait");
        assert_eq!(trace["profileLeaseWaitTimeoutMs"], 2500);
        assert_eq!(command["targetServiceId"], "google");
        assert_eq!(command["targetServices"][1], "microsoft");
        assert_eq!(command["siteId"], "nih");
        assert_eq!(command["loginIds"][0], "orcid");
        assert_eq!(command["profileLeasePolicy"], "wait");
        assert_eq!(command["profileLeaseWaitTimeoutMs"], 2500);
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
    fn service_browser_retry_command_forwards_operator_and_trace_fields() {
        let command = service_browser_retry_command(
            "browser-1",
            Some("operator"),
            Some("approved"),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "service_browser_retry");
        assert_eq!(command["browserId"], "browser-1");
        assert_eq!(command["by"], "operator");
        assert_eq!(command["note"], "approved");
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
    fn browser_command_command_forwards_params_timeout_and_trace_fields() {
        let params = json!({
            "url": "https://example.com",
            "waitUntil": "load",
            "action": "ignored",
            "id": "ignored",
        });
        let command = browser_command_command(BrowserCommandArgs {
            action: "navigate",
            params: params.as_object(),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(command["action"], "navigate");
        assert_ne!(command["id"], "ignored");
        assert_eq!(command["url"], "https://example.com");
        assert_eq!(command["waitUntil"], "load");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_navigate_command_forwards_options_and_trace_fields() {
        let headers = json!({
            "X-Agent-Browser-Smoke": "typed-navigate",
        });
        let command = browser_navigate_command(BrowserNavigateCommandArgs {
            url: "https://example.com",
            wait_until: Some("load"),
            headers: headers.as_object(),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(command["action"], "navigate");
        assert_eq!(command["url"], "https://example.com");
        assert_eq!(command["waitUntil"], "load");
        assert_eq!(
            command["headers"]["X-Agent-Browser-Smoke"],
            "typed-navigate"
        );
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_requests_command_forwards_filters_and_trace_fields() {
        let command = browser_requests_command(BrowserRequestsCommandArgs {
            clear: Some(false),
            filter: Some("/api"),
            resource_type: Some("fetch,xhr"),
            method: Some("GET"),
            status: Some("2xx"),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(command["action"], "requests");
        assert_eq!(command["clear"], false);
        assert_eq!(command["filter"], "/api");
        assert_eq!(command["type"], "fetch,xhr");
        assert_eq!(command["method"], "GET");
        assert_eq!(command["status"], "2xx");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_request_detail_command_forwards_request_id_and_trace_fields() {
        let command = browser_request_detail_command(
            "request-1",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "request_detail");
        assert_eq!(command["requestId"], "request-1");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_headers_command_forwards_headers_and_trace_fields() {
        let headers = json!({
            "X-Agent-Browser-Smoke": "typed-headers",
        });
        let command = browser_headers_command(
            headers.as_object().unwrap(),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "headers");
        assert_eq!(command["headers"]["X-Agent-Browser-Smoke"], "typed-headers");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_offline_command_forwards_state_and_trace_fields() {
        let command = browser_offline_command(
            Some(false),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "offline");
        assert_eq!(command["offline"], false);
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_cookies_commands_forward_options_and_trace_fields() {
        let urls = vec!["https://example.com".to_string()];
        let get_command = browser_cookies_get_command(
            Some(&urls),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(get_command["action"], "cookies_get");
        assert_eq!(get_command["urls"][0], "https://example.com");
        assert_eq!(get_command["jobTimeoutMs"], 1000);
        assert_eq!(get_command["serviceName"], "JournalDownloader");
        assert_eq!(get_command["agentName"], "agent-a");
        assert_eq!(get_command["taskName"], "probeACSwebsite");

        let expires = json!(1893456000.0);
        let set_command = browser_cookies_set_command(BrowserCookiesSetCommandArgs {
            cookies: None,
            name: Some("session"),
            value: Some("abc"),
            url: Some("https://example.com"),
            domain: None,
            path: Some("/"),
            expires: Some(&expires),
            http_only: Some(true),
            secure: Some(false),
            same_site: Some("Lax"),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(set_command["action"], "cookies_set");
        assert_eq!(set_command["name"], "session");
        assert_eq!(set_command["value"], "abc");
        assert_eq!(set_command["url"], "https://example.com");
        assert_eq!(set_command["path"], "/");
        assert_eq!(set_command["expires"], 1893456000.0);
        assert_eq!(set_command["httpOnly"], true);
        assert_eq!(set_command["secure"], false);
        assert_eq!(set_command["sameSite"], "Lax");
        assert_eq!(set_command["jobTimeoutMs"], 1000);
        assert_eq!(set_command["serviceName"], "JournalDownloader");
        assert_eq!(set_command["agentName"], "agent-a");
        assert_eq!(set_command["taskName"], "probeACSwebsite");

        let clear_command = browser_action_command(
            "mcp-browser-cookies-clear",
            "cookies_clear",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );
        assert_eq!(clear_command["action"], "cookies_clear");
        assert_eq!(clear_command["jobTimeoutMs"], 1000);
        assert_eq!(clear_command["serviceName"], "JournalDownloader");
        assert_eq!(clear_command["agentName"], "agent-a");
        assert_eq!(clear_command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_storage_commands_forward_options_and_trace_fields() {
        let get_command = browser_storage_get_command(
            Some("local"),
            Some("token"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(get_command["action"], "storage_get");
        assert_eq!(get_command["type"], "local");
        assert_eq!(get_command["key"], "token");
        assert_eq!(get_command["jobTimeoutMs"], 1000);
        assert_eq!(get_command["serviceName"], "JournalDownloader");
        assert_eq!(get_command["agentName"], "agent-a");
        assert_eq!(get_command["taskName"], "probeACSwebsite");

        let set_command = browser_storage_set_command(BrowserStorageSetCommandArgs {
            storage_type: Some("session"),
            key: "token",
            value: "abc",
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(set_command["action"], "storage_set");
        assert_eq!(set_command["type"], "session");
        assert_eq!(set_command["key"], "token");
        assert_eq!(set_command["value"], "abc");
        assert_eq!(set_command["jobTimeoutMs"], 1000);
        assert_eq!(set_command["serviceName"], "JournalDownloader");
        assert_eq!(set_command["agentName"], "agent-a");
        assert_eq!(set_command["taskName"], "probeACSwebsite");

        let clear_command = browser_storage_clear_command(
            Some("local"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );
        assert_eq!(clear_command["action"], "storage_clear");
        assert_eq!(clear_command["type"], "local");
        assert_eq!(clear_command["jobTimeoutMs"], 1000);
        assert_eq!(clear_command["serviceName"], "JournalDownloader");
        assert_eq!(clear_command["agentName"], "agent-a");
        assert_eq!(clear_command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_session_shaping_commands_forward_options_and_trace_fields() {
        let user_agent_command = browser_user_agent_command(
            "TestBot/1.0",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(user_agent_command["action"], "user_agent");
        assert_eq!(user_agent_command["userAgent"], "TestBot/1.0");
        assert_eq!(user_agent_command["jobTimeoutMs"], 1000);
        assert_eq!(user_agent_command["serviceName"], "JournalDownloader");
        assert_eq!(user_agent_command["agentName"], "agent-a");
        assert_eq!(user_agent_command["taskName"], "probeACSwebsite");

        let viewport_command = browser_viewport_command(BrowserViewportCommandArgs {
            width: 800,
            height: 600,
            device_scale_factor: Some(2.0),
            mobile: Some(true),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(viewport_command["action"], "viewport");
        assert_eq!(viewport_command["width"], 800);
        assert_eq!(viewport_command["height"], 600);
        assert_eq!(viewport_command["deviceScaleFactor"], 2.0);
        assert_eq!(viewport_command["mobile"], true);
        assert_eq!(viewport_command["jobTimeoutMs"], 1000);
        assert_eq!(viewport_command["serviceName"], "JournalDownloader");
        assert_eq!(viewport_command["agentName"], "agent-a");
        assert_eq!(viewport_command["taskName"], "probeACSwebsite");

        let geolocation_command = browser_geolocation_command(
            41.8781,
            -87.6298,
            Some(10.0),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(geolocation_command["action"], "geolocation");
        assert_eq!(geolocation_command["latitude"], 41.8781);
        assert_eq!(geolocation_command["longitude"], -87.6298);
        assert_eq!(geolocation_command["accuracy"], 10.0);
        assert_eq!(geolocation_command["jobTimeoutMs"], 1000);
        assert_eq!(geolocation_command["serviceName"], "JournalDownloader");
        assert_eq!(geolocation_command["agentName"], "agent-a");
        assert_eq!(geolocation_command["taskName"], "probeACSwebsite");

        let permissions = vec!["geolocation".to_string(), "notifications".to_string()];
        let permissions_command = browser_permissions_command(
            &permissions,
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(permissions_command["action"], "permissions");
        assert_eq!(permissions_command["permissions"][0], "geolocation");
        assert_eq!(permissions_command["permissions"][1], "notifications");
        assert_eq!(permissions_command["jobTimeoutMs"], 1000);
        assert_eq!(permissions_command["serviceName"], "JournalDownloader");
        assert_eq!(permissions_command["agentName"], "agent-a");
        assert_eq!(permissions_command["taskName"], "probeACSwebsite");

        let timezone_command = browser_timezone_command(
            "America/Chicago",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(timezone_command["action"], "timezone");
        assert_eq!(timezone_command["timezoneId"], "America/Chicago");
        assert_eq!(timezone_command["jobTimeoutMs"], 1000);
        assert_eq!(timezone_command["serviceName"], "JournalDownloader");
        assert_eq!(timezone_command["agentName"], "agent-a");
        assert_eq!(timezone_command["taskName"], "probeACSwebsite");

        let locale_command = browser_locale_command(
            "en-US",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(locale_command["action"], "locale");
        assert_eq!(locale_command["locale"], "en-US");
        assert_eq!(locale_command["jobTimeoutMs"], 1000);
        assert_eq!(locale_command["serviceName"], "JournalDownloader");
        assert_eq!(locale_command["agentName"], "agent-a");
        assert_eq!(locale_command["taskName"], "probeACSwebsite");

        let features =
            serde_json::Map::from_iter([("prefers-contrast".to_string(), json!("more"))]);
        let media_command = browser_media_command(BrowserMediaCommandArgs {
            media: Some("screen"),
            color_scheme: Some("dark"),
            reduced_motion: Some("reduce"),
            features: Some(&features),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(media_command["action"], "emulatemedia");
        assert_eq!(media_command["media"], "screen");
        assert_eq!(media_command["colorScheme"], "dark");
        assert_eq!(media_command["reducedMotion"], "reduce");
        assert_eq!(media_command["features"]["prefers-contrast"], "more");
        assert_eq!(media_command["jobTimeoutMs"], 1000);
        assert_eq!(media_command["serviceName"], "JournalDownloader");
        assert_eq!(media_command["agentName"], "agent-a");
        assert_eq!(media_command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_file_dialog_har_and_route_commands_forward_options_and_trace_fields() {
        let dialog_command = browser_dialog_command(
            "accept",
            Some("prompt value"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(dialog_command["action"], "dialog");
        assert_eq!(dialog_command["response"], "accept");
        assert_eq!(dialog_command["promptText"], "prompt value");
        assert_eq!(dialog_command["jobTimeoutMs"], 1000);
        assert_eq!(dialog_command["serviceName"], "JournalDownloader");
        assert_eq!(dialog_command["agentName"], "agent-a");
        assert_eq!(dialog_command["taskName"], "probeACSwebsite");

        let files = vec!["/tmp/one.txt".to_string(), "/tmp/two.txt".to_string()];
        let upload_command = browser_upload_command(
            "#file",
            &files,
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(upload_command["action"], "upload");
        assert_eq!(upload_command["selector"], "#file");
        assert_eq!(upload_command["files"][0], "/tmp/one.txt");
        assert_eq!(upload_command["files"][1], "/tmp/two.txt");
        assert_eq!(upload_command["jobTimeoutMs"], 1000);
        assert_eq!(upload_command["serviceName"], "JournalDownloader");
        assert_eq!(upload_command["agentName"], "agent-a");
        assert_eq!(upload_command["taskName"], "probeACSwebsite");

        let download_command = browser_download_command(
            "#download",
            "/tmp/download.txt",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(download_command["action"], "download");
        assert_eq!(download_command["selector"], "#download");
        assert_eq!(download_command["path"], "/tmp/download.txt");
        assert_eq!(download_command["jobTimeoutMs"], 1000);
        assert_eq!(download_command["serviceName"], "JournalDownloader");
        assert_eq!(download_command["agentName"], "agent-a");
        assert_eq!(download_command["taskName"], "probeACSwebsite");

        let wait_command = browser_wait_for_download_command(
            Some("/tmp/download.txt"),
            Some(5000),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(wait_command["action"], "waitfordownload");
        assert_eq!(wait_command["path"], "/tmp/download.txt");
        assert_eq!(wait_command["timeoutMs"], 5000);
        assert_eq!(wait_command["jobTimeoutMs"], 1000);
        assert_eq!(wait_command["serviceName"], "JournalDownloader");
        assert_eq!(wait_command["agentName"], "agent-a");
        assert_eq!(wait_command["taskName"], "probeACSwebsite");

        let har_stop_command = browser_har_stop_command(
            Some("/tmp/capture.har"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(har_stop_command["action"], "har_stop");
        assert_eq!(har_stop_command["path"], "/tmp/capture.har");
        assert_eq!(har_stop_command["jobTimeoutMs"], 1000);
        assert_eq!(har_stop_command["serviceName"], "JournalDownloader");
        assert_eq!(har_stop_command["agentName"], "agent-a");
        assert_eq!(har_stop_command["taskName"], "probeACSwebsite");

        let route_response = serde_json::Map::from_iter([
            ("status".to_string(), json!(200)),
            ("body".to_string(), json!("{}")),
            ("contentType".to_string(), json!("application/json")),
        ]);
        let route_command = browser_route_command(BrowserRouteCommandArgs {
            url: "**/api/*",
            abort: Some(false),
            response: Some(&route_response),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(route_command["action"], "route");
        assert_eq!(route_command["url"], "**/api/*");
        assert_eq!(route_command["abort"], false);
        assert_eq!(route_command["response"]["status"], 200);
        assert_eq!(route_command["response"]["body"], "{}");
        assert_eq!(route_command["jobTimeoutMs"], 1000);
        assert_eq!(route_command["serviceName"], "JournalDownloader");
        assert_eq!(route_command["agentName"], "agent-a");
        assert_eq!(route_command["taskName"], "probeACSwebsite");

        let unroute_command = browser_unroute_command(
            Some("**/api/*"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(unroute_command["action"], "unroute");
        assert_eq!(unroute_command["url"], "**/api/*");
        assert_eq!(unroute_command["jobTimeoutMs"], 1000);
        assert_eq!(unroute_command["serviceName"], "JournalDownloader");
        assert_eq!(unroute_command["agentName"], "agent-a");
        assert_eq!(unroute_command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_observability_artifact_commands_forward_options_and_trace_fields() {
        let console_command = browser_console_command(
            Some(true),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(console_command["action"], "console");
        assert_eq!(console_command["clear"], true);
        assert_eq!(console_command["jobTimeoutMs"], 1000);
        assert_eq!(console_command["serviceName"], "JournalDownloader");
        assert_eq!(console_command["agentName"], "agent-a");
        assert_eq!(console_command["taskName"], "probeACSwebsite");

        let pdf_command = browser_pdf_command(BrowserPdfCommandArgs {
            path: Some("/tmp/page.pdf"),
            print_background: Some(false),
            landscape: Some(true),
            prefer_css_page_size: Some(true),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(pdf_command["action"], "pdf");
        assert_eq!(pdf_command["path"], "/tmp/page.pdf");
        assert_eq!(pdf_command["printBackground"], false);
        assert_eq!(pdf_command["landscape"], true);
        assert_eq!(pdf_command["preferCSSPageSize"], true);
        assert_eq!(pdf_command["jobTimeoutMs"], 1000);
        assert_eq!(pdf_command["serviceName"], "JournalDownloader");
        assert_eq!(pdf_command["agentName"], "agent-a");
        assert_eq!(pdf_command["taskName"], "probeACSwebsite");

        let response_body_command = browser_response_body_command(
            "/api/data",
            Some(5000),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(response_body_command["action"], "responsebody");
        assert_eq!(response_body_command["url"], "/api/data");
        assert_eq!(response_body_command["timeoutMs"], 5000);
        assert_eq!(response_body_command["jobTimeoutMs"], 1000);
        assert_eq!(response_body_command["serviceName"], "JournalDownloader");
        assert_eq!(response_body_command["agentName"], "agent-a");
        assert_eq!(response_body_command["taskName"], "probeACSwebsite");

        let clipboard_command = browser_clipboard_command(
            Some("write"),
            Some("clip text"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(clipboard_command["action"], "clipboard");
        assert_eq!(clipboard_command["operation"], "write");
        assert_eq!(clipboard_command["text"], "clip text");
        assert_eq!(clipboard_command["jobTimeoutMs"], 1000);
        assert_eq!(clipboard_command["serviceName"], "JournalDownloader");
        assert_eq!(clipboard_command["agentName"], "agent-a");
        assert_eq!(clipboard_command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_navigation_tab_and_content_commands_forward_options_and_trace_fields() {
        let back_command = browser_action_command(
            "mcp-browser-back",
            "back",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(back_command["action"], "back");
        assert_eq!(back_command["jobTimeoutMs"], 1000);
        assert_eq!(back_command["serviceName"], "JournalDownloader");
        assert_eq!(back_command["agentName"], "agent-a");
        assert_eq!(back_command["taskName"], "probeACSwebsite");

        let tab_new_command = browser_tab_new_command(
            Some("https://example.com"),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(tab_new_command["action"], "tab_new");
        assert_eq!(tab_new_command["url"], "https://example.com");
        assert_eq!(tab_new_command["jobTimeoutMs"], 1000);
        assert_eq!(tab_new_command["serviceName"], "JournalDownloader");
        assert_eq!(tab_new_command["agentName"], "agent-a");
        assert_eq!(tab_new_command["taskName"], "probeACSwebsite");

        let tab_switch_command = browser_tab_switch_command(
            0,
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(tab_switch_command["action"], "tab_switch");
        assert_eq!(tab_switch_command["index"], 0);
        assert_eq!(tab_switch_command["jobTimeoutMs"], 1000);
        assert_eq!(tab_switch_command["serviceName"], "JournalDownloader");
        assert_eq!(tab_switch_command["agentName"], "agent-a");
        assert_eq!(tab_switch_command["taskName"], "probeACSwebsite");

        let tab_close_command = browser_tab_close_command(
            Some(1),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(tab_close_command["action"], "tab_close");
        assert_eq!(tab_close_command["index"], 1);
        assert_eq!(tab_close_command["jobTimeoutMs"], 1000);
        assert_eq!(tab_close_command["serviceName"], "JournalDownloader");
        assert_eq!(tab_close_command["agentName"], "agent-a");
        assert_eq!(tab_close_command["taskName"], "probeACSwebsite");

        let set_content_command = browser_set_content_command(
            "<main>ok</main>",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(set_content_command["action"], "setcontent");
        assert_eq!(set_content_command["html"], "<main>ok</main>");
        assert_eq!(set_content_command["jobTimeoutMs"], 1000);
        assert_eq!(set_content_command["serviceName"], "JournalDownloader");
        assert_eq!(set_content_command["agentName"], "agent-a");
        assert_eq!(set_content_command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_get_url_command_forwards_timeout_and_trace_fields() {
        let command = browser_read_command(
            BROWSER_GET_URL_TOOL,
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
        let command = browser_read_command(
            BROWSER_GET_TITLE_TOOL,
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
    fn browser_tabs_command_forwards_options_and_trace_fields() {
        let command = browser_tabs_command(
            Some(true),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "tab_list");
        assert_eq!(command["verbose"], true);
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_screenshot_command_forwards_options_and_trace_fields() {
        let command = browser_screenshot_command(BrowserScreenshotCommandArgs {
            selector: Some("#main"),
            path: Some("/tmp/page.jpeg"),
            full_page: Some(true),
            annotate: Some(false),
            format: Some("jpeg"),
            quality: Some(80),
            screenshot_dir: Some("/tmp/shots"),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(command["action"], "screenshot");
        assert_eq!(command["selector"], "#main");
        assert_eq!(command["path"], "/tmp/page.jpeg");
        assert_eq!(command["fullPage"], true);
        assert_eq!(command["annotate"], false);
        assert_eq!(command["format"], "jpeg");
        assert_eq!(command["quality"], 80);
        assert_eq!(command["screenshotDir"], "/tmp/shots");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_click_command_forwards_options_and_trace_fields() {
        let command = browser_click_command(
            "#ready",
            Some(true),
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "click");
        assert_eq!(command["selector"], "#ready");
        assert_eq!(command["newTab"], true);
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_fill_command_forwards_options_and_trace_fields() {
        let command = browser_fill_command(
            "#name",
            "Ada Lovelace",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "fill");
        assert_eq!(command["selector"], "#name");
        assert_eq!(command["value"], "Ada Lovelace");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_wait_command_forwards_selector_options_and_trace_fields() {
        let command = browser_wait_command(BrowserWaitCommandArgs {
            selector: Some("#ready"),
            state: Some("visible"),
            text: None,
            url: None,
            function: None,
            load_state: None,
            timeout_ms: Some(1000),
            job_timeout_ms: Some(1500),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        })
        .unwrap();

        assert_eq!(command["action"], "wait");
        assert_eq!(command["selector"], "#ready");
        assert_eq!(command["state"], "visible");
        assert_eq!(command["timeout"], 1000);
        assert_eq!(command["jobTimeoutMs"], 1500);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_wait_command_maps_url_function_load_and_timeout_modes() {
        let url_command = browser_wait_command(BrowserWaitCommandArgs {
            selector: None,
            state: None,
            text: None,
            url: Some("**/dashboard"),
            function: None,
            load_state: None,
            timeout_ms: Some(1000),
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        })
        .unwrap();
        assert_eq!(url_command["action"], "waitforurl");
        assert_eq!(url_command["url"], "**/dashboard");
        assert_eq!(url_command["timeout"], 1000);

        let function_command = browser_wait_command(BrowserWaitCommandArgs {
            selector: None,
            state: None,
            text: None,
            url: None,
            function: Some("window.ready === true"),
            load_state: None,
            timeout_ms: Some(1000),
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        })
        .unwrap();
        assert_eq!(function_command["action"], "waitforfunction");
        assert_eq!(function_command["expression"], "window.ready === true");

        let load_command = browser_wait_command(BrowserWaitCommandArgs {
            selector: None,
            state: None,
            text: None,
            url: None,
            function: None,
            load_state: Some("domcontentloaded"),
            timeout_ms: Some(1000),
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        })
        .unwrap();
        assert_eq!(load_command["action"], "waitforloadstate");
        assert_eq!(load_command["state"], "domcontentloaded");

        let timeout_command = browser_wait_command(BrowserWaitCommandArgs {
            selector: None,
            state: None,
            text: None,
            url: None,
            function: None,
            load_state: None,
            timeout_ms: Some(250),
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        })
        .unwrap();
        assert_eq!(timeout_command["action"], "wait");
        assert_eq!(timeout_command["timeout"], 250);
    }

    #[test]
    fn browser_type_command_forwards_options_and_trace_fields() {
        let command = browser_type_command(BrowserTypeCommandArgs {
            selector: "#name",
            text: " Jr",
            clear: Some(false),
            delay_ms: Some(25),
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(command["action"], "type");
        assert_eq!(command["selector"], "#name");
        assert_eq!(command["text"], " Jr");
        assert_eq!(command["clear"], false);
        assert_eq!(command["delay"], 25);
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_press_command_forwards_key_and_trace_fields() {
        let command = browser_press_command(
            "Control+a",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "press");
        assert_eq!(command["key"], "Control+a");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_hover_command_forwards_selector_and_trace_fields() {
        let command = browser_hover_command(
            "#menu",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "hover");
        assert_eq!(command["selector"], "#menu");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_select_command_forwards_values_and_trace_fields() {
        let command = browser_select_command(
            "#org",
            &["org-a".to_string(), "org-b".to_string()],
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "select");
        assert_eq!(command["selector"], "#org");
        assert_eq!(command["values"][0], "org-a");
        assert_eq!(command["values"][1], "org-b");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_element_read_command_forwards_action_selector_attribute_and_trace_fields() {
        let text_command = browser_element_read_command(BrowserElementReadCommandArgs {
            action: "gettext",
            selector: "#status",
            attribute: None,
            properties: None,
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        });

        assert_eq!(text_command["action"], "gettext");
        assert_eq!(text_command["selector"], "#status");
        assert_eq!(text_command["jobTimeoutMs"], 1000);
        assert_eq!(text_command["serviceName"], "JournalDownloader");
        assert_eq!(text_command["agentName"], "agent-a");
        assert_eq!(text_command["taskName"], "probeACSwebsite");

        let attribute_command = browser_element_read_command(BrowserElementReadCommandArgs {
            action: "getattribute",
            selector: "#link",
            attribute: Some("href"),
            properties: None,
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        });

        assert_eq!(attribute_command["action"], "getattribute");
        assert_eq!(attribute_command["selector"], "#link");
        assert_eq!(attribute_command["attribute"], "href");
        assert!(attribute_command.get("jobTimeoutMs").is_none());
        assert!(attribute_command.get("serviceName").is_none());

        let count_command = browser_element_read_command(BrowserElementReadCommandArgs {
            action: "count",
            selector: ".item",
            attribute: None,
            properties: None,
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        });

        assert_eq!(count_command["action"], "count");
        assert_eq!(count_command["selector"], ".item");

        let html_command = browser_element_read_command(BrowserElementReadCommandArgs {
            action: "innerhtml",
            selector: "#main",
            attribute: None,
            properties: None,
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        });

        assert_eq!(html_command["action"], "innerhtml");
        assert_eq!(html_command["selector"], "#main");

        let box_command = browser_element_read_command(BrowserElementReadCommandArgs {
            action: "boundingbox",
            selector: "#ready",
            attribute: None,
            properties: None,
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        });

        assert_eq!(box_command["action"], "boundingbox");
        assert_eq!(box_command["selector"], "#ready");

        let styles_command = browser_element_read_command(BrowserElementReadCommandArgs {
            action: "styles",
            selector: "#ready",
            attribute: None,
            properties: Some(vec!["display".to_string(), "color".to_string()]),
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        });

        assert_eq!(styles_command["action"], "styles");
        assert_eq!(styles_command["selector"], "#ready");
        assert_eq!(styles_command["properties"][0], "display");
        assert_eq!(styles_command["properties"][1], "color");
    }

    #[test]
    fn browser_checked_command_forwards_action_selector_and_trace_fields() {
        let check_command = browser_checked_command(
            "check",
            "#remember",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(check_command["action"], "check");
        assert_eq!(check_command["selector"], "#remember");
        assert_eq!(check_command["jobTimeoutMs"], 1000);
        assert_eq!(check_command["serviceName"], "JournalDownloader");
        assert_eq!(check_command["agentName"], "agent-a");
        assert_eq!(check_command["taskName"], "probeACSwebsite");

        let uncheck_command =
            browser_checked_command("uncheck", "#remember", None, None, None, None);

        assert_eq!(uncheck_command["action"], "uncheck");
        assert_eq!(uncheck_command["selector"], "#remember");
        assert!(uncheck_command.get("jobTimeoutMs").is_none());
        assert!(uncheck_command.get("serviceName").is_none());

        let read_command =
            browser_checked_command("ischecked", "#remember", None, None, None, None);

        assert_eq!(read_command["action"], "ischecked");
        assert_eq!(read_command["selector"], "#remember");
        assert!(read_command.get("jobTimeoutMs").is_none());
    }

    #[test]
    fn browser_element_state_command_forwards_action_selector_and_trace_fields() {
        let visible_command = browser_element_state_command(
            "isvisible",
            "#ready",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(visible_command["action"], "isvisible");
        assert_eq!(visible_command["selector"], "#ready");
        assert_eq!(visible_command["jobTimeoutMs"], 1000);
        assert_eq!(visible_command["serviceName"], "JournalDownloader");
        assert_eq!(visible_command["agentName"], "agent-a");
        assert_eq!(visible_command["taskName"], "probeACSwebsite");

        let enabled_command =
            browser_element_state_command("isenabled", "#name", None, None, None, None);

        assert_eq!(enabled_command["action"], "isenabled");
        assert_eq!(enabled_command["selector"], "#name");
        assert!(enabled_command.get("jobTimeoutMs").is_none());
        assert!(enabled_command.get("serviceName").is_none());
    }

    #[test]
    fn browser_scroll_command_forwards_direction_delta_and_trace_fields() {
        let direction_command = browser_scroll_command(BrowserScrollCommandArgs {
            selector: Some("#panel"),
            direction: Some("down"),
            amount: Some(500.0),
            delta_x: None,
            delta_y: None,
            job_timeout_ms: Some(1000),
            service_name: Some("JournalDownloader"),
            agent_name: Some("agent-a"),
            task_name: Some("probeACSwebsite"),
        })
        .unwrap();

        assert_eq!(direction_command["action"], "scroll");
        assert_eq!(direction_command["selector"], "#panel");
        assert_eq!(direction_command["direction"], "down");
        assert_eq!(direction_command["amount"], 500.0);
        assert_eq!(direction_command["jobTimeoutMs"], 1000);
        assert_eq!(direction_command["serviceName"], "JournalDownloader");
        assert_eq!(direction_command["agentName"], "agent-a");
        assert_eq!(direction_command["taskName"], "probeACSwebsite");

        let delta_command = browser_scroll_command(BrowserScrollCommandArgs {
            selector: None,
            direction: None,
            amount: None,
            delta_x: Some(12.5),
            delta_y: Some(250.0),
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        })
        .unwrap();

        assert_eq!(delta_command["action"], "scroll");
        assert_eq!(delta_command["x"], 12.5);
        assert_eq!(delta_command["y"], 250.0);
        assert!(delta_command.get("direction").is_none());
        assert!(delta_command.get("amount").is_none());

        let default_command = browser_scroll_command(BrowserScrollCommandArgs {
            selector: None,
            direction: None,
            amount: None,
            delta_x: None,
            delta_y: None,
            job_timeout_ms: None,
            service_name: None,
            agent_name: None,
            task_name: None,
        })
        .unwrap();

        assert_eq!(default_command["direction"], "down");
        assert_eq!(default_command["amount"], 300.0);
    }

    #[test]
    fn browser_scroll_into_view_command_forwards_selector_and_trace_fields() {
        let command = browser_scroll_into_view_command(
            "#footer",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "scrollintoview");
        assert_eq!(command["selector"], "#footer");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");
    }

    #[test]
    fn browser_field_command_forwards_action_selector_and_trace_fields() {
        let command = browser_field_command(
            "focus",
            "#name",
            Some(1000),
            Some("JournalDownloader"),
            Some("agent-a"),
            Some("probeACSwebsite"),
        );

        assert_eq!(command["action"], "focus");
        assert_eq!(command["selector"], "#name");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "agent-a");
        assert_eq!(command["taskName"], "probeACSwebsite");

        let clear_command = browser_field_command("clear", "#name", None, None, None, None);

        assert_eq!(clear_command["action"], "clear");
        assert_eq!(clear_command["selector"], "#name");
        assert!(clear_command.get("jobTimeoutMs").is_none());
        assert!(clear_command.get("serviceName").is_none());
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

        run_stdio_server(
            input.as_bytes(),
            &mut output,
            "default",
            ServiceState::default(),
        )
        .unwrap();
        let lines = String::from_utf8(output).unwrap();
        let responses = lines.lines().collect::<Vec<_>>();

        assert_eq!(responses.len(), 2);
        assert!(responses[0].contains(r#""method""#) == false);
        assert!(responses[1].contains("agent-browser://incidents"));
        assert!(responses[1].contains("agent-browser://profiles"));
        assert!(responses[1].contains("agent-browser://sessions"));
        assert!(responses[1].contains("agent-browser://browsers"));
        assert!(responses[1].contains("agent-browser://tabs"));
        assert!(responses[1].contains("agent-browser://monitors"));
        assert!(responses[1].contains("agent-browser://site-policies"));
        assert!(responses[1].contains("agent-browser://providers"));
        assert!(responses[1].contains("agent-browser://challenges"));
        assert!(responses[1].contains("agent-browser://jobs"));
        assert!(responses[1].contains("agent-browser://events"));
    }

    #[test]
    fn read_profiles_resource_returns_profiles_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_profile_allocation_contract, assert_service_profile_record_contract,
            BrowserHost, BrowserProfile, ProfileAllocationPolicy, ProfileKeyringPolicy,
        };

        let state = ServiceState {
            profiles: BTreeMap::from([
                (
                    "profile-b".to_string(),
                    BrowserProfile {
                        id: "profile-b".to_string(),
                        name: "Profile B".to_string(),
                        default_browser_host: Some(BrowserHost::LocalHeaded),
                        allocation: ProfileAllocationPolicy::PerService,
                        keyring: ProfileKeyringPolicy::BasicPasswordStore,
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
        assert_eq!(
            resource["contents"]["profiles"][1]["allocation"],
            "per_service"
        );
        assert_eq!(
            resource["contents"]["profiles"][1]["keyring"],
            "basic_password_store"
        );
        assert_service_profile_record_contract(&resource["contents"]["profiles"][1]);
        assert_eq!(resource["contents"]["profileSources"][0]["id"], "profile-a");
        assert_eq!(
            resource["contents"]["profileSources"][0]["source"],
            "persisted_state"
        );
        assert_eq!(
            resource["contents"]["profileAllocations"][0]["profileId"],
            "profile-a"
        );
        assert_eq!(
            resource["contents"]["profileAllocations"][1]["profileId"],
            "profile-b"
        );
        assert_service_profile_allocation_contract(&resource["contents"]["profileAllocations"][1]);
    }

    #[test]
    fn read_access_plan_resource_returns_service_owned_recommendation() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            BrowserProfile, ProfileReadinessState, ProfileTargetReadiness,
        };

        let state = ServiceState {
            profiles: BTreeMap::from([(
                "canva".to_string(),
                BrowserProfile {
                    id: "canva".to_string(),
                    name: "Canva".to_string(),
                    target_service_ids: vec!["canva".to_string()],
                    authenticated_service_ids: vec!["canva".to_string()],
                    shared_service_ids: vec!["CanvaCLI".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "canva".to_string(),
                        state: ProfileReadinessState::Fresh,
                        evidence: "authenticated_hint_present".to_string(),
                        recommended_action: "use_profile".to_string(),
                        ..ProfileTargetReadiness::default()
                    }],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(
            "agent-browser://access-plan?serviceName=CanvaCLI&agentName=codex&taskName=openCanvaWorkspace&loginId=canva",
            &state,
        )
        .unwrap();

        assert_eq!(
            resource["uri"],
            "agent-browser://access-plan?serviceName=CanvaCLI&agentName=codex&taskName=openCanvaWorkspace&loginId=canva"
        );
        assert_eq!(resource["contents"]["query"]["agentName"], "codex");
        assert_eq!(
            resource["contents"]["query"]["taskName"],
            "openCanvaWorkspace"
        );
        assert_eq!(resource["contents"]["query"]["hasNamingWarning"], false);
        assert_eq!(resource["contents"]["selectedProfile"]["id"], "canva");
        assert_eq!(
            resource["contents"]["selectedProfileMatch"]["reason"],
            "authenticated_target"
        );
        assert_eq!(
            resource["contents"]["decision"]["recommendedAction"],
            "use_selected_profile"
        );
    }

    #[test]
    fn read_sessions_resource_returns_sessions_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_session_record_contract, BrowserSession, LeaseState,
            SessionCleanupPolicy,
        };

        let state = ServiceState {
            sessions: BTreeMap::from([
                (
                    "session-b".to_string(),
                    BrowserSession {
                        id: "session-b".to_string(),
                        service_name: Some("JournalDownloader".to_string()),
                        lease: LeaseState::Exclusive,
                        profile_id: Some("profile-b".to_string()),
                        cleanup: SessionCleanupPolicy::CloseTabs,
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
        assert_eq!(
            resource["contents"]["sessions"][1]["serviceName"],
            "JournalDownloader"
        );
        assert_eq!(
            resource["contents"]["sessions"][1]["profileId"],
            "profile-b"
        );
        assert_eq!(resource["contents"]["sessions"][1]["cleanup"], "close_tabs");
        assert_service_session_record_contract(&resource["contents"]["sessions"][1]);
    }

    #[test]
    fn read_browsers_resource_returns_browsers_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_browser_record_contract, BrowserHealth, BrowserProcess,
        };

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
        assert_service_browser_record_contract(&resource["contents"]["browsers"][1]);
    }

    #[test]
    fn read_tabs_resource_returns_tabs_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_tab_record_contract, BrowserTab, TabLifecycle,
        };

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
        assert_service_tab_record_contract(&resource["contents"]["tabs"][1]);
    }

    #[test]
    fn read_monitors_resource_returns_monitors_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_monitor_record_contract, MonitorState, MonitorTarget, SiteMonitor,
        };

        let state = ServiceState {
            monitors: BTreeMap::from([
                (
                    "monitor-b".to_string(),
                    SiteMonitor {
                        id: "monitor-b".to_string(),
                        name: "Monitor B".to_string(),
                        target: MonitorTarget::SitePolicy("google".to_string()),
                        state: MonitorState::Paused,
                        ..SiteMonitor::default()
                    },
                ),
                (
                    "monitor-a".to_string(),
                    SiteMonitor {
                        id: "monitor-a".to_string(),
                        name: "Monitor A".to_string(),
                        target: MonitorTarget::Url("https://example.com/".to_string()),
                        state: MonitorState::Active,
                        ..SiteMonitor::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(MONITORS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 2);
        assert_eq!(resource["contents"]["monitors"][0]["id"], "monitor-a");
        assert_eq!(resource["contents"]["monitors"][1]["id"], "monitor-b");
        assert_eq!(
            resource["contents"]["monitors"][1]["target"]["site_policy"],
            "google"
        );
        assert_service_monitor_record_contract(&resource["contents"]["monitors"][1]);
    }

    #[test]
    fn read_site_policies_resource_returns_policies_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_site_policy_record_contract, SitePolicy,
        };

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

        assert_eq!(resource["contents"]["count"], 3);
        assert_eq!(resource["contents"]["sitePolicies"][0]["id"], "gmail");
        assert_eq!(resource["contents"]["sitePolicies"][1]["id"], "google");
        assert_eq!(resource["contents"]["sitePolicies"][2]["id"], "microsoft");
        assert_eq!(resource["contents"]["sitePolicySources"][0]["id"], "gmail");
        assert_eq!(
            resource["contents"]["sitePolicySources"][0]["source"],
            "builtin"
        );
        assert_eq!(
            resource["contents"]["sitePolicySources"][1]["source"],
            "persisted_state"
        );
        assert_eq!(
            resource["contents"]["sitePolicySources"][2]["source"],
            "persisted_state"
        );
        assert_service_site_policy_record_contract(&resource["contents"]["sitePolicies"][0]);
    }

    #[test]
    fn read_providers_resource_returns_providers_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_provider_record_contract, ProviderKind, ServiceProvider,
        };

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
        assert_service_provider_record_contract(&resource["contents"]["providers"][1]);
    }

    #[test]
    fn read_challenges_resource_returns_challenges_sorted_by_id() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_challenge_record_contract, Challenge, ChallengeKind, ChallengeState,
        };

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
        assert_service_challenge_record_contract(&resource["contents"]["challenges"][1]);
    }

    #[test]
    fn read_jobs_resource_returns_jobs_sorted_by_submission_time() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            assert_service_job_naming_warning_contract, service_job_naming_warning_values,
            JobState, JobTarget, ServiceJob,
        };

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
                        naming_warnings: service_job_naming_warning_values(),
                        has_naming_warning: true,
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
        assert_service_job_naming_warning_contract(&resource["contents"]["jobs"][0]);
    }

    #[test]
    fn read_incidents_resource_returns_record_contract_fields() {
        use crate::native::service_model::{
            assert_service_incident_record_contract, BrowserHealth, ServiceIncident,
            ServiceIncidentEscalation, ServiceIncidentSeverity, ServiceIncidentState,
        };

        let state = ServiceState {
            incidents: vec![ServiceIncident {
                id: "browser-1".to_string(),
                browser_id: Some("browser-1".to_string()),
                label: "browser-1".to_string(),
                state: ServiceIncidentState::Active,
                severity: ServiceIncidentSeverity::Error,
                escalation: ServiceIncidentEscalation::BrowserRecovery,
                recommended_action:
                    "Review recovery trace and retry or relaunch the affected browser.".to_string(),
                latest_timestamp: "2026-04-22T00:01:00Z".to_string(),
                latest_message: "Browser crashed".to_string(),
                latest_kind: "browser_health_changed".to_string(),
                current_health: Some(BrowserHealth::ProcessExited),
                event_ids: vec!["event-1".to_string()],
                job_ids: vec!["job-1".to_string()],
                ..ServiceIncident::default()
            }],
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(INCIDENTS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 1);
        assert_eq!(resource["contents"]["incidents"][0]["id"], "browser-1");
        assert_service_incident_record_contract(&resource["contents"]["incidents"][0]);
    }

    #[test]
    fn read_events_resource_returns_retained_events() {
        use crate::native::service_model::{
            assert_service_event_record_contract, BrowserHealth, ServiceEvent, ServiceEventKind,
        };

        let state = ServiceState {
            events: vec![ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:00:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser crashed".to_string(),
                browser_id: Some("browser-1".to_string()),
                profile_id: Some("work".to_string()),
                session_id: Some("session-1".to_string()),
                service_name: Some("JournalDownloader".to_string()),
                agent_name: Some("codex".to_string()),
                task_name: Some("probeACSwebsite".to_string()),
                previous_health: Some(BrowserHealth::Ready),
                current_health: Some(BrowserHealth::ProcessExited),
                details: Some(json!({"reasonKind": "process_exited"})),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        };

        let resource = read_service_mcp_resource_from_state(EVENTS_RESOURCE, &state).unwrap();

        assert_eq!(resource["contents"]["count"], 1);
        assert_eq!(resource["contents"]["events"][0]["id"], "event-1");
        assert_service_event_record_contract(&resource["contents"]["events"][0]);
    }
}
