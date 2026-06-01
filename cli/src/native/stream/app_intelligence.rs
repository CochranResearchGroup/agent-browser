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
    (
        "200 OK",
        json!({
            "success": true,
            "data": {
                "confirmationId": confirmation_id,
                "status": "recorded",
                "authenticatedUser": {
                    "username": identity.username,
                    "role": identity.role,
                },
                "execution": "not-implemented",
                "reason": "Confirmation recording is staged before mutating operator tools are enabled.",
            }
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
    let tool_calls = operator_read_tool_calls(&run_id, &created_at, prompt, &packet);
    let dashboard_actions = operator_dashboard_actions(&packet);
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
            "reason": "Read selected browser, session, tab, profile, process, and stream identity from the redacted packet",
            "tools": ["describe_selected_browser"]
        },
        {
            "id": "dom",
            "label": "DOM tools",
            "enabled": false,
            "reason": "Tool contract pending",
            "tools": ["snapshot", "query", "click", "type", "press", "scroll", "screenshot"]
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
            "enabled": false,
            "reason": "Service-mediated operator contracts pending",
            "tools": ["service_request", "launch_workspace", "repair_stream", "close_browser"]
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
}
