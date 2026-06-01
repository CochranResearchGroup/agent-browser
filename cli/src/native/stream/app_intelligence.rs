use super::app_intelligence_schema::{validate_packet, CONTEXTUAL_CHAT_PROVIDER_ID};
use super::app_intelligence_supervisor::{
    inspect_with_supervisor, operator_guidance_with_supervisor, resolve_codex_bin,
    InspectionFailure, InspectionInput, OperatorGuidanceInput,
};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub(crate) const APP_INTELLIGENCE_INSPECT_HTTP_ROUTE: &str =
    "/api/app-intelligence/inspect-workspace";
pub(crate) const APP_INTELLIGENCE_STATUS_HTTP_ROUTE: &str = "/api/app-intelligence/status";
pub(crate) const APP_INTELLIGENCE_OPERATOR_STATUS_HTTP_ROUTE: &str =
    "/api/app-intelligence/operator/status";
pub(crate) const APP_INTELLIGENCE_OPERATOR_TURN_HTTP_ROUTE: &str =
    "/api/app-intelligence/operator/turn";
pub(crate) const APP_INTELLIGENCE_OPERATOR_CONFIRM_HTTP_ROUTE: &str =
    "/api/app-intelligence/operator/confirm";

#[derive(Debug, Clone)]
pub(crate) struct OperatorIdentity {
    pub username: String,
    pub display_name: String,
    pub role: String,
}

pub(crate) fn app_intelligence_status_json() -> Value {
    let readiness = codex_app_server_readiness();
    json!({
        "success": true,
        "data": {
            "providers": [
                {
                    "id": CONTEXTUAL_CHAT_PROVIDER_ID,
                    "label": "Codex app server",
                    "transport": "stdio-jsonl",
                    "mode": "read-only-inspection",
                    "ready": readiness.ready,
                    "reason": readiness.reason,
                }
            ],
            "defaultProvider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "exposedProviderCount": 1,
        }
    })
}

pub(crate) fn operator_status_json(identity: &OperatorIdentity) -> Value {
    json!({
        "success": true,
        "data": {
            "mode": "superuser-operator",
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "ready": true,
            "authenticatedUser": {
                "username": identity.username,
                "displayName": identity.display_name,
                "role": identity.role,
            },
            "toolGroups": operator_tool_manifest(),
            "destructiveActionsRequireConfirmation": true,
        }
    })
}

pub(crate) fn operator_turn_response(
    body: &str,
    identity: &OperatorIdentity,
) -> (&'static str, Value) {
    match operator_turn_response_inner(body, identity) {
        Ok(value) => ("200 OK", value),
        Err((status, message)) => (
            status,
            json!({
                "success": false,
                "error": message,
                "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            }),
        ),
    }
}

pub(crate) fn operator_confirm_response(
    body: &str,
    identity: &OperatorIdentity,
) -> (&'static str, Value) {
    let parsed: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(err) => {
            return (
                "400 Bad Request",
                json!({"success": false, "error": format!("Invalid operator confirmation JSON: {err}")}),
            )
        }
    };
    let confirmation_id = parsed
        .get("confirmationId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if confirmation_id.is_empty() {
        return (
            "400 Bad Request",
            json!({"success": false, "error": "Missing confirmationId."}),
        );
    }
    let action = match parsed.get("action") {
        Some(value) if value.is_object() => value.clone(),
        _ => {
            return (
                "400 Bad Request",
                json!({"success": false, "error": "Missing confirmation action."}),
            )
        }
    };
    if action.get("kind").and_then(Value::as_str) != Some("operator_confirmation") {
        return (
            "400 Bad Request",
            json!({"success": false, "error": "Confirmation action must be an operator_confirmation action."}),
        );
    }
    if action.get("confirmationId").and_then(Value::as_str) != Some(confirmation_id) {
        return (
            "400 Bad Request",
            json!({"success": false, "error": "Confirmation id does not match action."}),
        );
    }
    let Some(request) = action.get("request").filter(|value| value.is_object()) else {
        return (
            "400 Bad Request",
            json!({"success": false, "error": "Confirmation action is missing a service request."}),
        );
    };
    let confirmed_at = Utc::now().to_rfc3339();
    let confirmed_action = json!({
        "id": format!("confirmed-{confirmation_id}"),
        "label": action.get("label").cloned().unwrap_or_else(|| json!("Apply confirmed action")),
        "kind": "service_request",
        "requiresConfirmation": false,
        "request": request,
        "reason": action.get("reason").cloned().unwrap_or_else(|| json!("Confirmed by superuser.")),
        "confirmation": {
            "confirmationId": confirmation_id,
            "confirmedAt": confirmed_at,
            "confirmedBy": identity.username,
        }
    });
    let record = json!({
        "version": "codex-operator-confirmation.v1",
        "confirmationId": confirmation_id,
        "status": "confirmed",
        "confirmedAt": confirmed_at,
        "authenticatedUser": {
            "username": identity.username,
            "displayName": identity.display_name,
            "role": identity.role,
        },
        "action": action,
        "confirmedAction": confirmed_action,
    });
    if let Err(message) = write_operator_confirmation_ledger(confirmation_id, &record) {
        return (
            "500 Internal Server Error",
            json!({"success": false, "error": format!("Failed to write operator confirmation: {message}")}),
        );
    }
    (
        "200 OK",
        json!({
            "success": true,
            "data": record
        }),
    )
}

pub(crate) fn inspect_workspace_response(body: &str) -> (&'static str, Value) {
    match inspect_workspace_response_inner(body) {
        Ok(value) => ("200 OK", value),
        Err((status, message)) => (
            status,
            json!({
                "success": false,
                "error": message,
                "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            }),
        ),
    }
}

fn inspect_workspace_response_inner(body: &str) -> Result<Value, (&'static str, String)> {
    let request: Value = serde_json::from_str(body).map_err(|err| {
        (
            "400 Bad Request",
            format!("Invalid app intelligence request JSON: {err}"),
        )
    })?;
    reject_mutating_request(&request)?;

    let provider = request
        .get("provider")
        .or_else(|| request.pointer("/packet/provider"))
        .and_then(Value::as_str)
        .unwrap_or(CONTEXTUAL_CHAT_PROVIDER_ID);
    if provider != CONTEXTUAL_CHAT_PROVIDER_ID {
        return Err((
            "400 Bad Request",
            "Contextual Chat currently exposes only the Codex app server provider.".to_string(),
        ));
    }

    let packet = request.get("packet").ok_or_else(|| {
        (
            "400 Bad Request",
            "Missing selected workspace chat packet.".to_string(),
        )
    })?;
    validate_packet(packet).map_err(|message| ("400 Bad Request", message))?;

    let run_id = format!("codex-inspect-{}", uuid::Uuid::new_v4());
    let created_at = Utc::now().to_rfc3339();
    let packet_hash = sha256_json(packet);
    let prompt = request
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let workspace_id = packet.pointer("/workspace/id").and_then(Value::as_str);
    let input = InspectionInput {
        run_id,
        created_at,
        prompt: prompt.to_string(),
        packet: packet.clone(),
        packet_hash,
        workspace_id: workspace_id.map(str::to_string),
    };
    match inspect_with_supervisor(input) {
        Ok(result) => Ok(json!({
            "success": true,
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "data": {
                "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
                "observation": result.observation,
                "ledger": dashboard_ledger(result.ledger),
            }
        })),
        Err(failure) => Ok(failure_response(failure)),
    }
}

fn operator_turn_response_inner(
    body: &str,
    identity: &OperatorIdentity,
) -> Result<Value, (&'static str, String)> {
    let request: Value = serde_json::from_str(body).map_err(|err| {
        (
            "400 Bad Request",
            format!("Invalid operator request JSON: {err}"),
        )
    })?;
    let prompt = request
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if prompt.is_empty() {
        return Err(("400 Bad Request", "Missing operator prompt.".to_string()));
    }
    if let Some(packet) = request.get("packet") {
        validate_packet(packet).map_err(|message| ("400 Bad Request", message))?;
    }
    let run_id = format!("codex-operator-{}", uuid::Uuid::new_v4());
    let created_at = Utc::now().to_rfc3339();
    let packet = request.get("packet").cloned().unwrap_or(Value::Null);
    let packet_hash = if packet.is_null() {
        None
    } else {
        Some(sha256_json(&packet))
    };
    let workspace_id = packet
        .pointer("/workspace/id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let tool_groups = operator_tool_manifest();
    let target = operator_target_summary(&packet);
    let tool_calls = combine_tool_calls(
        operator_read_tool_calls(&run_id, &created_at, prompt, &packet),
        operator_action_tool_calls(&run_id, &created_at, prompt, &packet),
    );
    let dashboard_actions = combine_dashboard_actions(
        operator_dashboard_actions(&packet),
        operator_executable_actions(prompt, &packet, identity),
    );
    let guidance_result = operator_guidance_with_supervisor(OperatorGuidanceInput {
        run_id: run_id.clone(),
        created_at: created_at.clone(),
        prompt: prompt.to_string(),
        packet: packet.clone(),
        packet_hash: packet_hash.clone(),
        workspace_id: workspace_id.clone(),
        username: identity.username.clone(),
        tool_manifest: tool_groups.clone(),
        read_tool_calls: tool_calls.clone(),
        dashboard_actions: dashboard_actions.clone(),
    });
    let (operator_guidance, operator_guidance_ledger, operator_guidance_failure) =
        match guidance_result {
            Ok(success) => (success.guidance, success.ledger, Value::Null),
            Err(failure) => (
                deterministic_operator_guidance_fallback(&packet, &tool_calls),
                failure.ledger.unwrap_or(Value::Null),
                json!({
                    "code": failure.code,
                    "message": failure.message,
                }),
            ),
        };
    let summary = operator_guidance
        .get("summary")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| operator_turn_summary(&packet, &tool_calls));
    let response = json!({
        "version": "codex-operator-turn.v1",
        "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
        "mode": "superuser-operator",
        "runId": run_id,
        "createdAt": created_at,
        "workspaceId": workspace_id,
        "contextPacketHash": packet_hash,
        "authenticatedUser": {
            "username": identity.username,
            "displayName": identity.display_name,
            "role": identity.role,
        },
        "target": target,
        "summary": summary,
        "operatorGuidance": operator_guidance,
        "operatorGuidanceFailure": operator_guidance_failure,
        "toolGroups": tool_groups,
        "proposedNextSteps": operator_next_steps(&packet),
        "dashboardActions": dashboard_actions,
        "requiresConfirmation": false,
        "toolCalls": tool_calls,
        "ledger": {
            "runId": run_id,
            "createdAt": created_at,
            "mode": "superuser-operator",
            "status": "read-tools-completed",
            "workspaceId": workspace_id,
            "contextPacketHash": packet_hash,
            "target": target,
            "toolCallCount": tool_calls.as_array().map(|items| items.len()).unwrap_or(0),
            "codex": operator_guidance_ledger,
            "authenticatedUser": {
                "username": identity.username,
                "role": identity.role,
            }
        }
    });
    write_operator_ledger(&run_id, &response).map_err(|message| {
        (
            "500 Internal Server Error",
            format!("Failed to write operator ledger: {message}"),
        )
    })?;
    Ok(json!({
        "success": true,
        "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
        "data": response,
    }))
}

fn reject_mutating_request(value: &Value) -> Result<(), (&'static str, String)> {
    for key in [
        "action",
        "command",
        "execute",
        "mutating",
        "serviceRequest",
        "service_request",
        "toolCall",
    ] {
        if value.get(key).is_some() {
            return Err((
                "400 Bad Request",
                format!(
                    "App Intelligence inspection is read-only and rejected mutating field `{key}`."
                ),
            ));
        }
    }
    Ok(())
}

struct Readiness {
    ready: bool,
    reason: String,
}

fn codex_app_server_readiness() -> Readiness {
    let codex_bin = resolve_codex_bin();
    match Command::new(&codex_bin)
        .args(["app-server", "--help"])
        .output()
    {
        Ok(output) if output.status.success() => Readiness {
            ready: true,
            reason: format!("codex app-server command is available: {codex_bin}"),
        },
        Ok(output) => Readiness {
            ready: false,
            reason: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(err) => Readiness {
            ready: false,
            reason: format!("codex app-server command is unavailable: {err}"),
        },
    }
}

fn sha256_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn dashboard_ledger(ledger: Value) -> Value {
    json!({
        "runId": ledger["runId"],
        "provider": ledger["provider"],
        "createdAt": ledger["createdAt"],
        "workspaceId": ledger["workspaceId"],
        "contextPacketHash": ledger["contextPacketHash"],
        "threadId": ledger.pointer("/codex/threadId").cloned().unwrap_or(Value::Null),
        "turnId": ledger.pointer("/codex/turnId").cloned().unwrap_or(Value::Null),
        "appServerTransport": ledger.pointer("/codex/transport").cloned().unwrap_or(Value::Null),
        "appServerCliVersion": ledger.pointer("/codex/cliVersion").cloned().unwrap_or(Value::Null),
        "eventLogPath": ledger.pointer("/artifacts/eventLogPath").cloned().unwrap_or(Value::Null),
        "normalizedEventLogPath": ledger.pointer("/artifacts/normalizedEventLogPath").cloned().unwrap_or(Value::Null),
        "observationPath": ledger.pointer("/artifacts/observationPath").cloned().unwrap_or(Value::Null),
        "runPath": ledger.pointer("/artifacts/runPath").cloned().unwrap_or(Value::Null),
    })
}

fn failure_response(failure: InspectionFailure) -> Value {
    let message = failure.message;
    json!({
        "success": false,
        "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
        "error": message.clone(),
        "data": {
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "failure": {
                "code": failure.code,
                "message": message,
            },
            "ledger": failure.ledger.map(dashboard_ledger).unwrap_or(Value::Null),
        }
    })
}

fn operator_tool_manifest() -> Value {
    json!([
        {
            "id": "dashboard",
            "label": "Dashboard tools",
            "enabled": true,
            "reason": "Read selected workspace and return superuser-applied dashboard selection actions",
            "tools": ["get_selected_workspace", "propose_view_selected_workspace"]
        },
        {
            "id": "browser",
            "label": "Browser tools",
            "enabled": true,
            "reason": "Read selected browser identity and prepare scoped navigation actions for the human superuser to apply",
            "tools": ["describe_selected_browser", "propose_launch_browser", "propose_focus", "propose_navigate", "propose_new_tab", "propose_wait", "propose_close_browser", "propose_repair_browser"]
        },
        {
            "id": "dom",
            "label": "DOM tools",
            "enabled": true,
            "reason": "Scoped DOM discovery and interaction proposals are available through service request contracts",
            "tools": ["propose_snapshot", "propose_query", "propose_click", "propose_type", "propose_press", "propose_scroll", "propose_screenshot"]
        },
        {
            "id": "debug",
            "label": "Debug evidence tools",
            "enabled": true,
            "reason": "Read stream and runtime readiness from selected workspace evidence",
            "tools": ["stream_readiness", "runtime_readiness"]
        },
        {
            "id": "service",
            "label": "Service tools",
            "enabled": true,
            "reason": "Superuser-applied service request actions are available for scoped browser operation and confirmation-gated service management",
            "tools": ["propose_clear_storage", "propose_clear_cookies", "service_request:navigate", "service_request:view_focus", "service_request:tab_new", "service_request:wait", "service_request:snapshot", "service_request:count", "service_request:click", "service_request:type", "service_request:press", "service_request:scroll", "service_request:storage_clear", "service_request:cookies_clear", "service_request:service_browser_close", "service_request:service_browser_repair", "service_request:service_prune_retained", "service_request:service_repair_retained"]
        }
    ])
}

fn operator_target_summary(packet: &Value) -> Value {
    if packet.is_null() {
        return json!({
            "workspaceId": Value::Null,
            "browserId": Value::Null,
            "sessionId": Value::Null,
            "tabId": Value::Null,
            "profileId": Value::Null,
            "jobId": Value::Null,
            "label": "No workspace selected",
            "state": "none",
            "url": Value::Null,
        });
    }
    json!({
        "workspaceId": packet.pointer("/workspace/id").cloned().unwrap_or(Value::Null),
        "browserId": packet.pointer("/selection/browserId").cloned().unwrap_or(Value::Null),
        "sessionId": packet.pointer("/selection/sessionId").cloned().unwrap_or(Value::Null),
        "tabId": packet.pointer("/selection/tabId").cloned().unwrap_or(Value::Null),
        "profileId": packet.pointer("/selection/profileId").cloned().unwrap_or(Value::Null),
        "jobId": packet.pointer("/selection/jobId").cloned().unwrap_or(Value::Null),
        "label": packet.pointer("/workspace/label").cloned().unwrap_or(Value::Null),
        "state": packet.pointer("/workspace/state").cloned().unwrap_or(Value::Null),
        "url": packet.pointer("/page/url").cloned().unwrap_or(Value::Null),
    })
}

fn operator_read_tool_calls(run_id: &str, created_at: &str, prompt: &str, packet: &Value) -> Value {
    let target = operator_target_summary(packet);
    let workspace = packet.get("workspace").cloned().unwrap_or(Value::Null);
    let runtime = packet.get("runtime").cloned().unwrap_or(Value::Null);
    let stream = packet.get("stream").cloned().unwrap_or(Value::Null);
    let page = packet.get("page").cloned().unwrap_or(Value::Null);
    let selection = packet.get("selection").cloned().unwrap_or(Value::Null);
    let evidence_ids: Vec<Value> = packet
        .get("evidence")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.get("included")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .map(|id| Value::String(id.to_string()))
                .collect()
        })
        .unwrap_or_default();
    json!([
        {
            "id": format!("{run_id}:get-selected-workspace"),
            "group": "dashboard",
            "tool": "get_selected_workspace",
            "status": "succeeded",
            "createdAt": created_at,
            "target": target,
            "input": {
                "promptHash": sha256_text(prompt),
                "source": "selected-workspace-chat-packet"
            },
            "output": {
                "selection": selection,
                "workspace": workspace,
                "page": page,
                "includedEvidenceIds": evidence_ids,
            },
            "summary": "Read the selected workspace identity from the redacted dashboard packet."
        },
        {
            "id": format!("{run_id}:describe-selected-browser"),
            "group": "browser",
            "tool": "describe_selected_browser",
            "status": "succeeded",
            "createdAt": created_at,
            "target": target,
            "input": {
                "source": "selected-workspace-chat-packet"
            },
            "output": {
                "runtime": runtime,
                "stream": stream,
                "page": page,
                "controllable": packet.pointer("/workspace/controllable").cloned().unwrap_or(Value::Bool(false)),
                "viewable": packet.pointer("/workspace/viewable").cloned().unwrap_or(Value::Bool(false)),
                "live": packet.pointer("/workspace/live").cloned().unwrap_or(Value::Bool(false)),
            },
            "summary": "Read selected browser runtime, stream, and page readiness from service-owned dashboard evidence."
        },
        {
            "id": format!("{run_id}:stream-readiness"),
            "group": "debug",
            "tool": "stream_readiness",
            "status": "succeeded",
            "createdAt": created_at,
            "target": target,
            "input": {
                "source": "selected-workspace-chat-packet"
            },
            "output": operator_stream_readiness(packet),
            "summary": "Evaluated embeddable stream, control input, CDP port, stream port, and recent frame evidence."
        }
    ])
}

fn operator_action_tool_calls(
    run_id: &str,
    created_at: &str,
    prompt: &str,
    packet: &Value,
) -> Value {
    let target = operator_target_summary(packet);
    let controllable = packet
        .pointer("/workspace/controllable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let session_id = packet
        .pointer("/selection/sessionId")
        .and_then(Value::as_str)
        .or_else(|| packet.pointer("/workspace/id").and_then(Value::as_str))
        .map(str::to_string);
    let mut calls = Vec::new();
    for action in operator_prompt_actions(prompt) {
        let target_ready = !action.requires_target || (controllable && session_id.is_some());
        let status = if target_ready { "proposed" } else { "blocked" };
        let summary = if status == "proposed" {
            format!(
                "Prepared a scoped service-mediated {} action for the selected browser. The human superuser must apply it from the dashboard.",
                action.contract_action
            )
        } else {
            format!(
                "{} intent was detected, but the selected workspace is not controllable or lacks a session target.",
                action.label
            )
        };
        calls.push(json!({
            "id": format!("{run_id}:{}", action.tool),
            "group": action.group,
            "tool": action.tool,
            "status": status,
            "createdAt": created_at,
            "target": target,
            "input": {
                "promptHash": sha256_text(prompt),
                "url": action.url,
                "selector": action.selector,
            },
            "output": {
                "url": action.url,
                "selector": action.selector,
                "controllable": controllable,
                "sessionId": session_id,
                "requiresConfirmation": action.requires_confirmation,
                "serviceContract": format!("service_request:{}", action.contract_action),
            },
            "summary": summary,
        }));
    }
    Value::Array(calls)
}

fn operator_stream_readiness(packet: &Value) -> Value {
    let embeddable = packet
        .pointer("/stream/embeddable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let controllable = packet
        .pointer("/stream/controllable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let live = packet
        .pointer("/workspace/live")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stream_port = packet
        .pointer("/runtime/streamPort")
        .cloned()
        .unwrap_or(Value::Null);
    let cdp_port = packet
        .pointer("/runtime/cdpPort")
        .cloned()
        .unwrap_or(Value::Null);
    let last_frame_at = packet
        .pointer("/runtime/lastFrameAt")
        .cloned()
        .unwrap_or(Value::Null);
    let ready = live && embeddable && !stream_port.is_null();
    json!({
        "ready": ready,
        "live": live,
        "embeddable": embeddable,
        "controllable": controllable,
        "provider": packet.pointer("/stream/provider").cloned().unwrap_or(Value::Null),
        "routeSummary": packet.pointer("/stream/routeSummary").cloned().unwrap_or(Value::Null),
        "controlInput": packet.pointer("/stream/controlInput").cloned().unwrap_or(Value::Null),
        "cdpPort": cdp_port,
        "streamPort": stream_port,
        "lastFrameAt": last_frame_at,
        "diagnosis": if ready {
            "Selected workspace reports a live embeddable stream."
        } else if !live {
            "Selected workspace is not reporting a live browser."
        } else if !embeddable {
            "Selected workspace does not report an embeddable stream."
        } else {
            "Selected workspace is missing a stream port."
        }
    })
}

fn operator_dashboard_actions(packet: &Value) -> Value {
    let workspace_id = packet.pointer("/workspace/id").and_then(Value::as_str);
    if workspace_id.is_none() {
        return json!([]);
    }
    json!([
        {
            "id": "view-selected-workspace",
            "label": "View selected workspace",
            "kind": "set_selected_workspace",
            "requiresConfirmation": false,
            "selection": {
                "workspaceId": packet.pointer("/workspace/id").cloned().unwrap_or(Value::Null),
                "browserId": packet.pointer("/selection/browserId").cloned().unwrap_or(Value::Null),
                "sessionId": packet.pointer("/selection/sessionId").cloned().unwrap_or(Value::Null),
                "tabId": packet.pointer("/selection/tabId").cloned().unwrap_or(Value::Null),
                "profileId": packet.pointer("/selection/profileId").cloned().unwrap_or(Value::Null),
                "jobId": packet.pointer("/selection/jobId").cloned().unwrap_or(Value::Null),
            },
            "reason": "This changes only the dashboard URL selection so the viewport and inspector follow the audited target."
        }
    ])
}

fn operator_executable_actions(prompt: &str, packet: &Value, identity: &OperatorIdentity) -> Value {
    let controllable = packet
        .pointer("/workspace/controllable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let session_id = packet
        .pointer("/selection/sessionId")
        .and_then(Value::as_str)
        .or_else(|| packet.pointer("/workspace/id").and_then(Value::as_str));
    let browser_id = packet
        .pointer("/selection/browserId")
        .cloned()
        .unwrap_or(Value::Null);
    let target_id = packet
        .pointer("/page/targetId")
        .cloned()
        .unwrap_or(Value::Null);
    let page_url = packet.pointer("/page/url").and_then(Value::as_str);
    let profile_id = packet
        .pointer("/selection/profileId")
        .and_then(Value::as_str);
    let mut actions = Vec::new();
    for action in operator_prompt_actions(prompt) {
        if action.requires_target && (!controllable || session_id.is_none()) {
            continue;
        }
        let mut params = json!({
            "browserId": browser_id,
            "targetId": target_id,
            "by": identity.username,
            "reason": "superuser_operator_request"
        });
        if let Some(session_id) = session_id {
            params["sessionName"] = json!(session_id);
        }
        if let Some(url) = action.url.as_deref() {
            params["url"] = json!(url);
        }
        if let Some(selector) = action.selector.as_deref() {
            params["selector"] = json!(selector);
        }
        if matches!(action.contract_action, "storage_clear" | "cookies_clear") {
            params["scope"] = if action.contract_action == "storage_clear" {
                json!("selected-tab-origin")
            } else {
                json!("selected-browser-profile")
            };
            if let Some(page_url) = page_url {
                params["currentUrl"] = json!(page_url);
                if let Some(origin) = origin_from_url(page_url) {
                    params["origin"] = json!(origin);
                }
            }
            if let Some(profile_id) = profile_id {
                params["profileId"] = json!(profile_id);
            }
        }
        for (key, value) in action.extra_params {
            params[key] = value;
        }
        let request = json!({
            "action": action.contract_action,
            "serviceName": "agent-browser-dashboard",
            "agentName": identity.username,
            "taskName": action.task_name,
            "params": params,
            "jobTimeoutMs": 30000
        });
        let mut request = request;
        if let Some(object) = request.as_object_mut() {
            for (key, value) in action.request_fields {
                object.insert(key.to_string(), value);
            }
        }
        if action.requires_confirmation {
            let confirmation_id = format!(
                "confirm-{}-{}",
                action.contract_action,
                uuid::Uuid::new_v4()
            );
            actions.push(json!({
                "id": action.action_id,
                "label": format!("Confirm {}", action.label),
                "kind": "operator_confirmation",
                "requiresConfirmation": true,
                "confirmationId": confirmation_id,
                "request": request,
                "risk": action.confirmation_risk,
                "target": operator_target_summary(packet),
                "promptHash": sha256_text(prompt),
                "reason": action.reason
            }));
            continue;
        }
        actions.push(json!({
            "id": action.action_id,
            "label": action.label,
            "kind": "service_request",
            "requiresConfirmation": action.requires_confirmation,
            "request": request,
            "reason": action.reason
        }));
    }
    Value::Array(actions)
}

fn combine_tool_calls(first: Value, second: Value) -> Value {
    let mut combined = Vec::new();
    if let Some(items) = first.as_array() {
        combined.extend(items.iter().cloned());
    }
    if let Some(items) = second.as_array() {
        combined.extend(items.iter().cloned());
    }
    Value::Array(combined)
}

fn combine_dashboard_actions(first: Value, second: Value) -> Value {
    let mut combined = Vec::new();
    if let Some(items) = first.as_array() {
        combined.extend(items.iter().cloned());
    }
    if let Some(items) = second.as_array() {
        combined.extend(items.iter().cloned());
    }
    Value::Array(combined)
}

fn operator_turn_summary(packet: &Value, tool_calls: &Value) -> String {
    if packet.is_null() {
        return "Operator mode is authenticated, but no workspace packet was supplied. Select a workspace before requesting browser operation.".to_string();
    }
    let label = packet
        .pointer("/workspace/label")
        .and_then(Value::as_str)
        .unwrap_or("selected workspace");
    let state = packet
        .pointer("/workspace/state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let stream = operator_stream_readiness(packet);
    let diagnosis = stream
        .get("diagnosis")
        .and_then(Value::as_str)
        .unwrap_or("Readiness diagnosis unavailable.");
    let count = tool_calls.as_array().map(|items| items.len()).unwrap_or(0);
    format!(
        "Ran {count} audited read tools for {label}: state={state}. {diagnosis} Mutation tools remain disabled until service and DOM contracts are wired."
    )
}

fn deterministic_operator_guidance_fallback(packet: &Value, tool_calls: &Value) -> Value {
    let label = packet
        .pointer("/workspace/label")
        .and_then(Value::as_str)
        .unwrap_or("selected workspace");
    let state = packet
        .pointer("/workspace/state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let stream = operator_stream_readiness(packet);
    let diagnosis = stream
        .get("diagnosis")
        .and_then(Value::as_str)
        .unwrap_or("Stream readiness diagnosis unavailable.");
    let count = tool_calls.as_array().map(|items| items.len()).unwrap_or(0);
    let controllable = packet
        .pointer("/workspace/controllable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "summary": format!("Agent Browser Operator reviewed {count} audited read tools for {label}: state={state}. {diagnosis}"),
        "targetAssessment": format!(
            "Target scope is workspace={}, browser={}, session={}, tab={}, profile={}.",
            packet.pointer("/workspace/id").and_then(Value::as_str).unwrap_or("null"),
            packet.pointer("/selection/browserId").and_then(Value::as_str).unwrap_or("null"),
            packet.pointer("/selection/sessionId").and_then(Value::as_str).unwrap_or("null"),
            packet.pointer("/selection/tabId").and_then(Value::as_str).unwrap_or("null"),
            packet.pointer("/selection/profileId").and_then(Value::as_str).unwrap_or("null"),
        ),
        "recommendedActions": [
            {
                "label": "Apply audited dashboard selection",
                "reason": "Align the viewport and inspector with the audited target before mutating browser or service state.",
                "toolGroup": "dashboard",
                "requiresConfirmation": false,
            },
            {
                "label": if controllable { "Enable scoped browser and DOM tools next" } else { "Resolve control path before mutation tools" },
                "reason": if controllable { "The selected workspace reports control input; the next slice should route navigation and DOM operations through audited service/CDP contracts." } else { "The selected workspace is not marked controllable, so mutation tools should remain disabled until the service reports a control path." },
                "toolGroup": if controllable { "browser" } else { "debug" },
                "requiresConfirmation": false,
            }
        ],
        "risks": [
            {
                "summary": "Codex operator guidance was unavailable, so this fallback uses host-side read tools only.",
                "severity": "warning",
            }
        ],
        "confirmationRequired": false,
        "confidence": if stream.get("ready").and_then(Value::as_bool).unwrap_or(false) { "medium" } else { "low" },
    })
}

fn operator_next_steps(packet: &Value) -> Value {
    if packet.is_null() {
        return json!([
            {
                "label": "Select a workspace",
                "reason": "Operator tools need an explicit selected workspace before they can identify browser, tab, profile, stream, and service targets."
            }
        ]);
    }
    let stream = operator_stream_readiness(packet);
    let ready = stream
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let controllable = packet
        .pointer("/workspace/controllable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut steps = Vec::new();
    steps.push(json!({
        "label": "View audited target",
        "reason": "Apply the dashboard selection action to align the viewport and inspector with the workspace the operator just inspected."
    }));
    if !ready {
        steps.push(json!({
            "label": "Inspect stream readiness",
            "reason": stream.get("diagnosis").and_then(Value::as_str).unwrap_or("Stream readiness needs more evidence.")
        }));
    }
    if controllable {
        steps.push(json!({
            "label": "Enable DOM and navigation tools",
            "reason": "The selected workspace reports control input; the next implementation slice should route click, type, navigate, and wait through audited service/CDP contracts."
        }));
    } else {
        steps.push(json!({
            "label": "Resolve control path",
            "reason": "The selected workspace is not marked controllable, so mutation tools should stay disabled until a service-owned control path exists."
        }));
    }
    Value::Array(steps)
}

fn sha256_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

struct OperatorPromptAction {
    action_id: &'static str,
    label: &'static str,
    group: &'static str,
    tool: &'static str,
    contract_action: &'static str,
    task_name: &'static str,
    reason: &'static str,
    confirmation_risk: &'static str,
    requires_confirmation: bool,
    requires_target: bool,
    url: Option<String>,
    selector: Option<String>,
    request_fields: Vec<(&'static str, Value)>,
    extra_params: Vec<(&'static str, Value)>,
}

fn operator_prompt_actions(prompt: &str) -> Vec<OperatorPromptAction> {
    let lower = prompt.to_lowercase();
    let mut actions = Vec::new();
    let launch_intent = lower.contains("new browser")
        || lower.contains("launch browser")
        || lower.contains("open browser")
        || lower.contains("new workspace")
        || lower.contains("new session");
    if launch_intent {
        let url = extract_navigation_url(prompt);
        actions.push(OperatorPromptAction {
            action_id: "launch-browser-workspace",
            label: "Launch browser workspace",
            group: "browser",
            tool: "propose_launch_browser",
            contract_action: "tab_new",
            task_name: "superuser-operator-launch-browser",
            reason: "Submits the existing tab_new service request contract with launch posture fields so the service can create or reuse a browser workspace.",
            confirmation_risk: "Launching a browser can allocate a profile lease and a private virtual display.",
            requires_confirmation: false,
            requires_target: false,
            url: url.clone(),
            selector: None,
            request_fields: vec![
                ("browserBuild", json!("stealthcdp_chromium")),
                ("displayIsolation", json!("private_virtual_display")),
                ("profileLeasePolicy", json!("wait")),
                ("profileLeaseWaitTimeoutMs", json!(30_000)),
            ],
            extra_params: vec![
                ("headless", json!(false)),
                ("browserBuild", json!("stealthcdp_chromium")),
                ("displayIsolation", json!("private_virtual_display")),
                ("viewStreamProvider", json!("cdp")),
                ("controlInputProvider", json!("cdp")),
            ],
        });
    }
    if !launch_intent {
        if let Some(url) = extract_navigation_url(prompt) {
            actions.push(OperatorPromptAction {
                action_id: "navigate-selected-browser",
                label: "Navigate selected browser",
                group: "browser",
                tool: "propose_navigate",
                contract_action: "navigate",
                task_name: "superuser-operator-navigate",
                reason: "Submits the existing service request navigate contract for the audited selected browser target.",
                confirmation_risk: "Navigation can change authenticated page state if the selected browser is signed in.",
                requires_confirmation: false,
                requires_target: true,
                url: Some(url),
                selector: None,
                request_fields: Vec::new(),
                extra_params: vec![("waitUntil", json!("load"))],
            });
        }
    }
    if lower.contains("focus")
        || lower.contains("bring to front")
        || lower.contains("view selected")
    {
        actions.push(OperatorPromptAction {
            action_id: "focus-selected-browser",
            label: "Focus selected browser",
            group: "browser",
            tool: "propose_focus",
            contract_action: "view_focus",
            task_name: "superuser-operator-focus",
            reason: "Submits the existing view_focus service request contract for the audited selected browser target.",
            confirmation_risk: "Focusing changes the visible service-owned browser window only.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: vec![("maximize", json!(true))],
        });
    }
    if lower.contains("new tab") || lower.contains("open tab") {
        actions.push(OperatorPromptAction {
            action_id: "new-tab-selected-browser",
            label: "Open new tab",
            group: "browser",
            tool: "propose_new_tab",
            contract_action: "tab_new",
            task_name: "superuser-operator-new-tab",
            reason: "Submits the existing tab_new service request contract for the audited selected browser target.",
            confirmation_risk: "Opening a new tab changes selected browser tab state.",
            requires_confirmation: false,
            requires_target: true,
            url: extract_navigation_url(prompt).or_else(|| Some("about:blank".to_string())),
            selector: None,
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("snapshot") || lower.contains("dom") || lower.contains("accessible tree") {
        actions.push(OperatorPromptAction {
            action_id: "snapshot-selected-browser",
            label: "Snapshot selected browser",
            group: "dom",
            tool: "propose_snapshot",
            contract_action: "snapshot",
            task_name: "superuser-operator-snapshot",
            reason: "Submits the existing snapshot service request contract for read-only DOM discovery on the audited selected browser target.",
            confirmation_risk: "DOM snapshots can expose visible page text but omit raw browser secrets by default.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: vec![("interactive", json!(true))],
        });
    }
    if lower.contains("query")
        || lower.contains("count ")
        || lower.contains("find element")
        || lower.contains("find selector")
    {
        actions.push(OperatorPromptAction {
            action_id: "query-selected-browser",
            label: "Query selected browser",
            group: "dom",
            tool: "propose_query",
            contract_action: "count",
            task_name: "superuser-operator-query",
            reason: "Submits the existing count service request contract for scoped selector discovery on the audited selected browser target.",
            confirmation_risk: "Selector count is read-only and returns aggregate DOM match count.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: extract_selector_after(prompt, "query")
                .or_else(|| extract_selector(prompt))
                .or_else(|| Some("body".to_string())),
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("click") || lower.contains("press button") {
        actions.push(OperatorPromptAction {
            action_id: "click-selected-browser",
            label: "Click selected browser",
            group: "dom",
            tool: "propose_click",
            contract_action: "click",
            task_name: "superuser-operator-click",
            reason: "Submits the existing click service request contract for the audited selected browser target and selector.",
            confirmation_risk: "Click can activate page controls in the selected browser.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: extract_selector_after(prompt, "click")
                .or_else(|| extract_selector(prompt))
                .or_else(|| Some("body".to_string())),
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("type ") || lower.contains("enter text") {
        actions.push(OperatorPromptAction {
            action_id: "type-selected-browser",
            label: "Type into selected browser",
            group: "dom",
            tool: "propose_type",
            contract_action: "type",
            task_name: "superuser-operator-type",
            reason: "Submits the existing type service request contract for the audited selected browser target and selector.",
            confirmation_risk: "Typing inserts the provided text into the selected browser.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: extract_selector_after(prompt, "into")
                .or_else(|| extract_selector_after(prompt, "in"))
                .or_else(|| extract_selector(prompt))
                .or_else(|| Some("body".to_string())),
            request_fields: Vec::new(),
            extra_params: vec![(
                "text",
                json!(extract_quoted_text(prompt).unwrap_or_default()),
            )],
        });
    }
    if lower.contains("press ") || lower.contains("hit enter") {
        actions.push(OperatorPromptAction {
            action_id: "press-selected-browser",
            label: "Press key in selected browser",
            group: "dom",
            tool: "propose_press",
            contract_action: "press",
            task_name: "superuser-operator-press",
            reason: "Submits the existing press service request contract for the audited selected browser target.",
            confirmation_risk: "Key press can submit forms or activate focused controls.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: extract_selector_after(prompt, "in").or_else(|| extract_selector_after(prompt, "on")),
            request_fields: Vec::new(),
            extra_params: vec![("key", json!(extract_key(prompt)))],
        });
    }
    if lower.contains("scroll") {
        actions.push(OperatorPromptAction {
            action_id: "scroll-selected-browser",
            label: "Scroll selected browser",
            group: "dom",
            tool: "propose_scroll",
            contract_action: "scroll",
            task_name: "superuser-operator-scroll",
            reason: "Submits the existing scroll service request contract for the audited selected browser target.",
            confirmation_risk: "Scroll changes viewport position only.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: extract_selector_after(prompt, "scroll").or_else(|| extract_selector(prompt)),
            request_fields: Vec::new(),
            extra_params: vec![
                ("direction", json!(extract_scroll_direction(prompt))),
                ("amount", json!(600)),
            ],
        });
    }
    if lower.contains("screenshot") {
        actions.push(OperatorPromptAction {
            action_id: "screenshot-selected-browser",
            label: "Screenshot selected browser",
            group: "dom",
            tool: "propose_screenshot",
            contract_action: "screenshot",
            task_name: "superuser-operator-screenshot",
            reason: "Prepares the existing screenshot service request contract, but confirmation execution must collect privacy intent before capture.",
            confirmation_risk: "Screenshots can capture private visible page content and require explicit superuser confirmation before execution.",
            requires_confirmation: true,
            requires_target: true,
            url: None,
            selector: extract_selector(prompt),
            request_fields: Vec::new(),
            extra_params: vec![("fullPage", json!(lower.contains("full page")))],
        });
    }
    if lower.contains("clear storage")
        || lower.contains("clear local storage")
        || lower.contains("clear localstorage")
        || lower.contains("clear session storage")
        || lower.contains("clear sessionstorage")
    {
        let storage_type = if lower.contains("session storage") || lower.contains("sessionstorage")
        {
            "session"
        } else {
            "local"
        };
        actions.push(OperatorPromptAction {
            action_id: "clear-selected-origin-storage",
            label: "Clear selected origin storage",
            group: "service",
            tool: "propose_clear_storage",
            contract_action: "storage_clear",
            task_name: "superuser-operator-clear-storage",
            reason: "Prepares the existing storage_clear contract for the selected browser tab origin and records the active URL, origin, profile, and storage type before execution.",
            confirmation_risk: "Clearing storage can sign the selected origin out, erase local application state, or remove pending form/session data.",
            requires_confirmation: true,
            requires_target: true,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: vec![("type", json!(storage_type))],
        });
    }
    if lower.contains("clear cookies") || lower.contains("clear cookie") {
        actions.push(OperatorPromptAction {
            action_id: "clear-selected-profile-cookies",
            label: "Clear selected profile cookies",
            group: "service",
            tool: "propose_clear_cookies",
            contract_action: "cookies_clear",
            task_name: "superuser-operator-clear-cookies",
            reason: "Prepares the existing cookies_clear contract for the selected browser profile and records the active URL, origin, and profile before execution.",
            confirmation_risk: "Clearing cookies affects the selected browser profile cookie jar and can sign sites out beyond the currently visible origin.",
            requires_confirmation: true,
            requires_target: true,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("wait") {
        actions.push(OperatorPromptAction {
            action_id: "wait-selected-browser",
            label: "Wait on selected browser",
            group: "browser",
            tool: "propose_wait",
            contract_action: "wait",
            task_name: "superuser-operator-wait",
            reason: "Submits the existing wait service request contract for the audited selected browser target.",
            confirmation_risk: "Wait observes page state without mutating it.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: extract_selector(prompt),
            request_fields: Vec::new(),
            extra_params: vec![("state", json!("visible")), ("timeout", json!(10_000))],
        });
    }
    if lower.contains("close browser")
        || lower.contains("close selected browser")
        || lower.contains("close the selected browser")
        || lower.contains("kill browser")
        || lower.contains("terminate browser")
    {
        actions.push(OperatorPromptAction {
            action_id: "close-selected-browser",
            label: "Close selected browser",
            group: "browser",
            tool: "propose_close_browser",
            contract_action: "service_browser_close",
            task_name: "superuser-operator-close-browser",
            reason: "Prepares the existing service_browser_close contract for the audited selected browser. Execution requires explicit superuser confirmation.",
            confirmation_risk: "Closing a browser terminates its live session and can interrupt active work, unsaved form state, and visible service-owned streams.",
            requires_confirmation: true,
            requires_target: true,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("repair browser")
        || lower.contains("reconnect browser")
        || lower.contains("repair stream")
        || lower.contains("retry browser")
    {
        actions.push(OperatorPromptAction {
            action_id: "repair-selected-browser",
            label: "Repair selected browser",
            group: "browser",
            tool: "propose_repair_browser",
            contract_action: "service_browser_repair",
            task_name: "superuser-operator-repair-browser",
            reason: "Submits the existing service_browser_repair contract for the audited selected browser to refresh service-owned stream and control readiness.",
            confirmation_risk: "Repair can restart or reconnect service-owned browser stream resources for the selected target.",
            requires_confirmation: false,
            requires_target: true,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("prune retained") || lower.contains("clear retained") {
        actions.push(OperatorPromptAction {
            action_id: "prune-retained-workspaces",
            label: "Prune retained workspaces",
            group: "service",
            tool: "propose_prune_retained",
            contract_action: "service_prune_retained",
            task_name: "superuser-operator-prune-retained",
            reason: "Prepares the existing service_prune_retained contract to remove retained workspace records. Execution requires explicit superuser confirmation.",
            confirmation_risk: "Pruning retained workspaces removes retained browser/workspace records and may discard useful forensic or recovery context.",
            requires_confirmation: true,
            requires_target: false,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    if lower.contains("repair retained") || lower.contains("recover retained") {
        actions.push(OperatorPromptAction {
            action_id: "repair-retained-workspaces",
            label: "Repair retained workspaces",
            group: "service",
            tool: "propose_repair_retained",
            contract_action: "service_repair_retained",
            task_name: "superuser-operator-repair-retained",
            reason: "Prepares the existing service_repair_retained contract for retained workspace recovery. Execution requires explicit superuser confirmation.",
            confirmation_risk: "Repairing retained workspaces can relink or restart retained service records and should be reviewed before execution.",
            requires_confirmation: true,
            requires_target: false,
            url: None,
            selector: None,
            request_fields: Vec::new(),
            extra_params: Vec::new(),
        });
    }
    actions
}

fn extract_navigation_url(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !["navigate", "open ", "go to", "visit"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return None;
    }
    prompt
        .split_whitespace()
        .map(|part| {
            part.trim_matches(|ch: char| {
                matches!(
                    ch,
                    '"' | '\''
                        | '`'
                        | ','
                        | ';'
                        | ')'
                        | '('
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '<'
                        | '>'
                        | '.'
                )
            })
        })
        .find(|part| {
            part.starts_with("https://") || part.starts_with("http://") || *part == "about:blank"
        })
        .map(str::to_string)
}

fn origin_from_url(value: &str) -> Option<String> {
    let parsed = url::Url::parse(value).ok()?;
    let origin = parsed.origin().ascii_serialization();
    (origin != "null").then_some(origin)
}

fn extract_selector(prompt: &str) -> Option<String> {
    prompt
        .split_whitespace()
        .map(|part| {
            part.trim_matches(|ch: char| {
                matches!(
                    ch,
                    '"' | '\'' | '`' | ',' | ';' | ')' | '(' | '{' | '}' | '<' | '>'
                )
            })
        })
        .find(|part| part.starts_with('#') || part.starts_with('.') || part.starts_with('['))
        .map(str::to_string)
}

fn extract_selector_after(prompt: &str, anchor: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    let anchor = anchor.to_lowercase();
    let index = lower.find(&anchor)?;
    extract_selector(&prompt[index + anchor.len()..])
}

fn extract_quoted_text(prompt: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let mut parts = prompt.split(quote);
        let _ = parts.next();
        if let Some(value) = parts.next() {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_key(prompt: &str) -> String {
    let lower = prompt.to_lowercase();
    if lower.contains("enter") || lower.contains("return") {
        return "Enter".to_string();
    }
    if lower.contains("escape") || lower.contains(" esc") {
        return "Escape".to_string();
    }
    if lower.contains("tab") {
        return "Tab".to_string();
    }
    prompt
        .split_whitespace()
        .skip_while(|part| part.to_lowercase() != "press")
        .nth(1)
        .map(|part| {
            part.trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | ',' | ';' | '.'))
                .to_string()
        })
        .filter(|part| !part.is_empty())
        .unwrap_or_else(|| "Enter".to_string())
}

fn extract_scroll_direction(prompt: &str) -> &'static str {
    let lower = prompt.to_lowercase();
    if lower.contains(" up") {
        "up"
    } else if lower.contains(" left") {
        "left"
    } else if lower.contains(" right") {
        "right"
    } else {
        "down"
    }
}

fn write_operator_ledger(run_id: &str, value: &Value) -> Result<(), String> {
    let root = app_intelligence_run_root()
        .ok_or_else(|| "Cannot resolve app intelligence run root.".to_string())?
        .join(run_id);
    fs::create_dir_all(&root)
        .map_err(|err| format!("Failed to create {}: {err}", root.display()))?;
    let path = root.join("operator-turn.json");
    let bytes = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&path, bytes).map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

fn write_operator_confirmation_ledger(confirmation_id: &str, value: &Value) -> Result<(), String> {
    let root = app_intelligence_run_root()
        .ok_or_else(|| "Cannot resolve app intelligence run root.".to_string())?
        .join("operator-confirmations");
    fs::create_dir_all(&root)
        .map_err(|err| format!("Failed to create {}: {err}", root.display()))?;
    let safe_id = confirmation_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let path = root.join(format!("{safe_id}.json"));
    let bytes = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&path, bytes).map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

fn app_intelligence_run_root() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    dirs::home_dir().map(|home| home.join(".agent-browser/app-intelligence/runs"))
}

#[cfg(test)]
mod tests {
    use super::super::app_intelligence_schema::{
        CODEX_WORKSPACE_OBSERVATION_VERSION, SELECTED_WORKSPACE_CHAT_PACKET_VERSION,
    };
    use super::super::app_intelligence_supervisor::APP_INTELLIGENCE_ENV_LOCK;
    use super::*;
    use std::{env, fs};

    fn fixture_packet() -> Value {
        json!({
            "version": SELECTED_WORKSPACE_CHAT_PACKET_VERSION,
            "createdAt": "2026-05-31T00:00:00Z",
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "selection": {
                "workspaceId": "browser:session:default",
                "browserId": "session:default",
                "sessionId": "default",
                "tabId": "target:abc",
                "profileId": "default",
                "jobId": null,
            },
            "workspace": {
                "id": "browser:session:default",
                "label": "default",
                "source": "attached_existing",
                "state": "active",
                "health": "ready",
                "live": true,
                "retained": false,
                "viewable": true,
                "controllable": true,
                "missingReason": null,
            },
            "runtime": {
                "pid": 123,
                "running": true,
                "rssBytes": 10,
                "cpuSeconds": 1,
                "cdpPort": 9222,
                "streamPort": 38395,
                "lastFrameAt": 1780240000,
            },
            "page": {
                "title": "Agent Browser",
                "url": "http://127.0.0.1:38409/app",
                "targetId": "abc",
                "lifecycle": "active",
                "active": true,
            },
            "stream": {
                "provider": "cdp_screencast",
                "routeSummary": "cdp screencast",
                "controlInput": "cdp input",
                "embeddable": true,
                "controllable": true,
            },
            "ownership": {
                "serviceName": null,
                "agentName": null,
                "taskName": null,
            },
            "evidence": [
                {
                    "id": "workspace.summary",
                    "source": "workspace",
                    "summary": "ready",
                    "facts": {},
                    "freshness": "fresh",
                    "included": true,
                },
                {
                    "id": "activity.unavailable",
                    "source": "activity",
                    "summary": "unavailable",
                    "facts": {},
                    "freshness": "unavailable",
                    "included": false,
                },
                {
                    "id": "console.unavailable",
                    "source": "console",
                    "summary": "unavailable",
                    "facts": {},
                    "freshness": "unavailable",
                    "included": false,
                },
                {
                    "id": "network.unavailable",
                    "source": "network",
                    "summary": "unavailable",
                    "facts": {},
                    "freshness": "unavailable",
                    "included": false,
                },
                {
                    "id": "storage.unavailable",
                    "source": "storage",
                    "summary": "unavailable",
                    "facts": {},
                    "freshness": "unavailable",
                    "included": false,
                },
                {
                    "id": "extensions.unavailable",
                    "source": "extensions",
                    "summary": "unavailable",
                    "facts": {},
                    "freshness": "unavailable",
                    "included": false,
                }
            ],
            "redaction": {
                "secretsOmitted": true,
                "screenshotsIncluded": false,
                "rawStorageIncluded": false,
                "rawHeadersIncluded": false,
            },
        })
    }

    #[test]
    fn rejects_non_codex_provider() {
        let request = json!({
            "provider": "openai",
            "packet": fixture_packet(),
        });
        let (status, body) = inspect_workspace_response(&request.to_string());
        assert_eq!(status, "400 Bad Request");
        assert_eq!(body["success"], false);
    }

    #[test]
    fn rejects_mutating_request_fields() {
        let request = json!({
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "action": "navigate",
            "packet": fixture_packet(),
        });
        let (status, body) = inspect_workspace_response(&request.to_string());
        assert_eq!(status, "400 Bad Request");
        assert_eq!(body["success"], false);
        assert!(body["error"].as_str().unwrap().contains("read-only"));
    }

    #[test]
    fn accepts_selected_workspace_packet() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-app-intelligence-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_MODE", "deterministic");
        let request = json!({
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "prompt": "inspect the selected browser",
            "packet": fixture_packet(),
        });
        let (status, body) = inspect_workspace_response(&request.to_string());
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_MODE");
        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        assert_eq!(body["provider"], CONTEXTUAL_CHAT_PROVIDER_ID);
        assert_eq!(
            body["data"]["observation"]["provider"],
            CONTEXTUAL_CHAT_PROVIDER_ID
        );
        assert_eq!(
            body["data"]["observation"]["version"],
            CODEX_WORKSPACE_OBSERVATION_VERSION
        );
        assert!(body["data"]["ledger"]["eventLogPath"].as_str().is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_status_exposes_superuser_tool_groups() {
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let body = operator_status_json(&identity);

        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["mode"], "superuser-operator");
        assert_eq!(body["data"]["authenticatedUser"]["role"], "superuser");
        assert!(body["data"]["toolGroups"].as_array().unwrap().len() >= 5);
    }

    #[test]
    fn operator_turn_writes_read_tool_ledger() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-intelligence-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let request = json!({
            "prompt": "Plan how to switch the viewed browser",
            "packet": fixture_packet(),
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["mode"], "superuser-operator");
        assert_eq!(body["data"]["authenticatedUser"]["username"], "admin");
        assert_eq!(body["data"]["toolCalls"].as_array().unwrap().len(), 3);
        assert_eq!(body["data"]["ledger"]["status"], "read-tools-completed");
        assert_eq!(
            body["data"]["operatorGuidance"]["recommendedActions"][0]["toolGroup"],
            "dashboard"
        );
        assert_eq!(
            body["data"]["ledger"]["codex"]["codex"]["transport"],
            "deterministic-test"
        );
        assert_eq!(
            body["data"]["dashboardActions"][0]["kind"],
            "set_selected_workspace"
        );
        let run_id = body["data"]["runId"].as_str().unwrap();
        assert!(root.join(run_id).join("operator-turn.json").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_turn_prepares_scoped_navigate_action() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-navigate-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let request = json!({
            "prompt": "Navigate the selected browser to https://example.com",
            "packet": fixture_packet(),
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["toolCalls"].as_array().unwrap().len(), 4);
        assert_eq!(body["data"]["toolCalls"][3]["tool"], "propose_navigate");
        assert_eq!(body["data"]["toolCalls"][3]["status"], "proposed");
        assert_eq!(
            body["data"]["dashboardActions"][1]["kind"],
            "service_request"
        );
        assert_eq!(
            body["data"]["dashboardActions"][1]["request"]["action"],
            "navigate"
        );
        assert_eq!(
            body["data"]["dashboardActions"][1]["request"]["params"]["url"],
            "https://example.com"
        );
        assert_eq!(
            body["data"]["dashboardActions"][1]["request"]["params"]["sessionName"],
            "default"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_turn_prepares_launch_browser_action_without_selected_target() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-launch-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let mut packet = fixture_packet();
        packet["workspace"]["controllable"] = json!(false);
        packet["selection"]["sessionId"] = Value::Null;
        let request = json!({
            "prompt": "Open a new browser to https://example.com",
            "packet": packet,
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        let tools = body["data"]["toolCalls"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|call| call.get("tool").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(tools.contains(&"propose_launch_browser"));
        assert!(!tools.contains(&"propose_navigate"));
        let launch_action = body["data"]["dashboardActions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("tab_new")
            })
            .unwrap();
        assert_eq!(launch_action["kind"], "service_request");
        assert_eq!(
            launch_action["request"]["browserBuild"],
            "stealthcdp_chromium"
        );
        assert_eq!(
            launch_action["request"]["displayIsolation"],
            "private_virtual_display"
        );
        assert_eq!(launch_action["request"]["profileLeasePolicy"], "wait");
        assert_eq!(
            launch_action["request"]["params"]["viewStreamProvider"],
            "cdp"
        );
        assert_eq!(
            launch_action["request"]["params"]["controlInputProvider"],
            "cdp"
        );
        assert_eq!(
            launch_action["request"]["params"]["url"],
            "https://example.com"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_turn_prepares_focus_wait_and_snapshot_actions() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-controls-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let request = json!({
            "prompt": "Focus the selected browser, snapshot the DOM, and wait for #ready",
            "packet": fixture_packet(),
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        let tools = body["data"]["toolCalls"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|call| call.get("tool").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(tools.contains(&"propose_focus"));
        assert!(tools.contains(&"propose_snapshot"));
        assert!(tools.contains(&"propose_wait"));
        let actions = body["data"]["dashboardActions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|action| {
                action
                    .pointer("/request/action")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        assert!(actions.contains(&"view_focus".to_string()));
        assert!(actions.contains(&"snapshot".to_string()));
        assert!(actions.contains(&"wait".to_string()));
        assert_eq!(
            body["data"]["dashboardActions"][3]["request"]["params"]["selector"],
            "#ready"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_turn_prepares_selector_dom_workflow_actions() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-dom-workflow-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let request = json!({
            "prompt": "Query #result, click #search, type \"Tempo\" into #search, press Enter, scroll down, and take a full page screenshot",
            "packet": fixture_packet(),
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        let tools = body["data"]["toolCalls"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|call| call.get("tool").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(tools.contains(&"propose_query"));
        assert!(tools.contains(&"propose_click"));
        assert!(tools.contains(&"propose_type"));
        assert!(tools.contains(&"propose_press"));
        assert!(tools.contains(&"propose_scroll"));
        assert!(tools.contains(&"propose_screenshot"));

        let dashboard_actions = body["data"]["dashboardActions"].as_array().unwrap();
        let service_actions = dashboard_actions
            .iter()
            .filter_map(|action| action.pointer("/request/action").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(service_actions.contains(&"count"));
        assert!(service_actions.contains(&"click"));
        assert!(service_actions.contains(&"type"));
        assert!(service_actions.contains(&"press"));
        assert!(service_actions.contains(&"scroll"));
        assert!(service_actions.contains(&"screenshot"));
        let screenshot_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("screenshot")
            })
            .unwrap();
        assert_eq!(screenshot_action["kind"], "operator_confirmation");
        assert_eq!(screenshot_action["requiresConfirmation"], true);
        assert!(screenshot_action["confirmationId"]
            .as_str()
            .unwrap()
            .starts_with("confirm-screenshot-"));

        let type_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("type")
            })
            .unwrap();
        assert_eq!(type_action["request"]["params"]["selector"], "#search");
        assert_eq!(type_action["request"]["params"]["text"], "Tempo");
        let press_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("press")
            })
            .unwrap();
        assert_eq!(press_action["request"]["params"]["key"], "Enter");
        let scroll_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("scroll")
            })
            .unwrap();
        assert_eq!(scroll_action["request"]["params"]["direction"], "down");
        let screenshot_call = body["data"]["toolCalls"]
            .as_array()
            .unwrap()
            .iter()
            .find(|call| call.get("tool").and_then(Value::as_str) == Some("propose_screenshot"))
            .unwrap();
        assert_eq!(
            screenshot_call["output"]["requiresConfirmation"], true,
            "screenshot capture should remain confirmation-gated"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_turn_prepares_service_management_confirmations() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-service-management-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let request = json!({
            "prompt": "Close the selected browser, repair browser stream, prune retained workspaces, and repair retained workspaces",
            "packet": fixture_packet(),
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        let tools = body["data"]["toolCalls"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|call| call.get("tool").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(tools.contains(&"propose_close_browser"));
        assert!(tools.contains(&"propose_repair_browser"));
        assert!(tools.contains(&"propose_prune_retained"));
        assert!(tools.contains(&"propose_repair_retained"));

        let dashboard_actions = body["data"]["dashboardActions"].as_array().unwrap();
        let service_actions = dashboard_actions
            .iter()
            .filter_map(|action| action.pointer("/request/action").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(service_actions.contains(&"service_browser_close"));
        assert!(service_actions.contains(&"service_browser_repair"));
        assert!(service_actions.contains(&"service_prune_retained"));
        assert!(service_actions.contains(&"service_repair_retained"));

        let close_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str)
                    == Some("service_browser_close")
            })
            .unwrap();
        assert_eq!(close_action["kind"], "operator_confirmation");
        assert_eq!(close_action["requiresConfirmation"], true);
        assert!(close_action["confirmationId"]
            .as_str()
            .unwrap()
            .starts_with("confirm-service_browser_close-"));
        assert_eq!(
            close_action["request"]["params"]["browserId"],
            "session:default"
        );

        let repair_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str)
                    == Some("service_browser_repair")
            })
            .unwrap();
        assert_eq!(repair_action["kind"], "service_request");
        assert_eq!(repair_action["requiresConfirmation"], false);

        for contract_action in ["service_prune_retained", "service_repair_retained"] {
            let action = dashboard_actions
                .iter()
                .find(|action| {
                    action.pointer("/request/action").and_then(Value::as_str)
                        == Some(contract_action)
                })
                .unwrap();
            assert_eq!(action["kind"], "operator_confirmation");
            assert_eq!(action["requiresConfirmation"], true);
            let confirmation_prefix = format!("confirm-{contract_action}-");
            assert!(action["confirmationId"]
                .as_str()
                .unwrap()
                .starts_with(confirmation_prefix.as_str()));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_turn_prepares_scoped_storage_and_cookie_confirmations() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-storage-cookies-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        env::set_var(
            "AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE",
            "deterministic",
        );
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let request = json!({
            "prompt": "Clear session storage and clear cookies for the selected browser",
            "packet": fixture_packet(),
        });
        let (status, body) = operator_turn_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_OPERATOR_MODE");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        let tools = body["data"]["toolCalls"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|call| call.get("tool").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(tools.contains(&"propose_clear_storage"));
        assert!(tools.contains(&"propose_clear_cookies"));

        let dashboard_actions = body["data"]["dashboardActions"].as_array().unwrap();
        let storage_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("storage_clear")
            })
            .unwrap();
        assert_eq!(storage_action["kind"], "operator_confirmation");
        assert_eq!(storage_action["requiresConfirmation"], true);
        assert_eq!(
            storage_action["request"]["params"]["scope"],
            "selected-tab-origin"
        );
        assert_eq!(storage_action["request"]["params"]["type"], "session");
        assert_eq!(
            storage_action["request"]["params"]["origin"],
            "http://127.0.0.1:38409"
        );
        assert_eq!(storage_action["request"]["params"]["profileId"], "default");

        let cookies_action = dashboard_actions
            .iter()
            .find(|action| {
                action.pointer("/request/action").and_then(Value::as_str) == Some("cookies_clear")
            })
            .unwrap();
        assert_eq!(cookies_action["kind"], "operator_confirmation");
        assert_eq!(cookies_action["requiresConfirmation"], true);
        assert_eq!(
            cookies_action["request"]["params"]["scope"],
            "selected-browser-profile"
        );
        assert_eq!(
            cookies_action["request"]["params"]["origin"],
            "http://127.0.0.1:38409"
        );
        assert_eq!(cookies_action["request"]["params"]["profileId"], "default");
        assert!(cookies_action["risk"]
            .as_str()
            .unwrap()
            .contains("beyond the currently visible origin"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn operator_confirm_records_and_returns_confirmed_service_action() {
        let _guard = APP_INTELLIGENCE_ENV_LOCK.lock().unwrap();
        let root = env::temp_dir().join(format!(
            "agent-browser-operator-confirm-{}",
            uuid::Uuid::new_v4()
        ));
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", &root);
        let identity = OperatorIdentity {
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let action = json!({
            "id": "screenshot-selected-browser",
            "label": "Confirm Screenshot selected browser",
            "kind": "operator_confirmation",
            "requiresConfirmation": true,
            "confirmationId": "confirm-screenshot-test",
            "request": {
                "action": "screenshot",
                "serviceName": "agent-browser-dashboard",
                "agentName": "admin",
                "taskName": "superuser-operator-screenshot",
                "params": {
                    "sessionName": "default",
                    "fullPage": true,
                    "by": "admin",
                    "reason": "superuser_operator_request"
                },
                "jobTimeoutMs": 30000
            },
            "risk": "Screenshots can capture private visible page content.",
            "reason": "Confirmed screenshot capture."
        });
        let request = json!({
            "confirmationId": "confirm-screenshot-test",
            "action": action,
        });
        let (status, body) = operator_confirm_response(&request.to_string(), &identity);
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");

        assert_eq!(status, "200 OK");
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["status"], "confirmed");
        assert_eq!(body["data"]["confirmedAction"]["kind"], "service_request");
        assert_eq!(
            body["data"]["confirmedAction"]["request"]["action"],
            "screenshot"
        );
        assert_eq!(
            body["data"]["confirmedAction"]["confirmation"]["confirmedBy"],
            "admin"
        );
        assert!(root
            .join("operator-confirmations")
            .join("confirm-screenshot-test.json")
            .exists());
        let _ = fs::remove_dir_all(root);
    }
}
