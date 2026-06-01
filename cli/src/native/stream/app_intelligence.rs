use super::app_intelligence_schema::{validate_packet, CONTEXTUAL_CHAT_PROVIDER_ID};
use super::app_intelligence_supervisor::{
    inspect_with_supervisor, resolve_codex_bin, InspectionFailure, InspectionInput,
};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::process::Command;

pub(crate) const APP_INTELLIGENCE_INSPECT_HTTP_ROUTE: &str =
    "/api/app-intelligence/inspect-workspace";
pub(crate) const APP_INTELLIGENCE_STATUS_HTTP_ROUTE: &str = "/api/app-intelligence/status";

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

#[cfg(test)]
mod tests {
    use super::super::app_intelligence_schema::{
        CODEX_WORKSPACE_OBSERVATION_VERSION, SELECTED_WORKSPACE_CHAT_PACKET_VERSION,
    };
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
}
