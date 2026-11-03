//! Native-authenticated mode adapter; the browser owner retains all effects.
use crate::native::browser_session_store::BrowserSessionSqliteStore;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModeRequest {
    slot: u32,
    generation: u64,
    choice: String,
    operation_id: Option<String>,
    mode: Option<String>,
    owner_confirmed: Option<bool>,
}
impl ModeRequest {
    fn valid(&self) -> bool {
        self.slot > 0
            && self.generation > 0
            && if self.choice == "inspect" {
                self.operation_id.is_none() && self.mode.is_none() && self.owner_confirmed.is_none()
            } else {
                matches!(
                    self.choice.as_str(),
                    "prepare"
                        | "reconcile"
                        | "relaunch"
                        | "pause_and_relaunch"
                        | "retry_graceful_close"
                        | "force_close"
                        | "focus"
                        | "restore_previous"
                        | "cancel"
                ) && self.operation_id.as_ref().is_some_and(|id| {
                    !id.trim().is_empty() && id.len() <= 128 && !id.chars().any(char::is_control)
                }) && self
                    .mode
                    .as_deref()
                    .is_some_and(|mode| matches!(mode, "cdp" | "cdp_free"))
            }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle(
    stream: &mut TcpStream,
    initial: &[u8],
    n: usize,
    method: &str,
    path: &str,
    query: Option<&str>,
    headers: &[(String, String)],
) {
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok());
    let authenticated = length.filter(|length| *length <= 4096).and_then(|length| {
        stream.peer_addr().ok().and_then(|peer| {
            super::remote_view_presentation::native_callback_operator(peer.ip(), headers, length)
                .ok()
        })
    });
    let Some(_operator) = authenticated else {
        super::dashboard::write_json_error(
            stream,
            "403 Forbidden",
            "remote_view_native_callback_denied",
        )
        .await;
        return;
    };
    let id = path
        .strip_prefix("/api/native-remote-view/")
        .and_then(|value| value.strip_suffix("/mode"));
    if method != "POST"
        || query.is_some()
        || id.is_none_or(|id| {
            id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
    {
        super::dashboard::write_json_error(
            stream,
            "400 Bad Request",
            "remote_view_native_callback_invalid",
        )
        .await;
        return;
    }
    let body = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        super::dashboard::read_post_body(stream, initial, n),
    )
    .await;
    let request = body
        .ok()
        .and_then(|body| serde_json::from_str::<ModeRequest>(&body).ok());
    if request.as_ref().is_none_or(|request| !request.valid()) {
        super::dashboard::write_json_error(
            stream,
            "400 Bad Request",
            "remote_view_native_mode_request_invalid",
        )
        .await;
        return;
    }
    match mode_reply(id.unwrap(), request.unwrap()).await {
        Ok(reply) => super::dashboard::write_json_value(stream, "200 OK", reply).await,
        Err(reason) => {
            let response = super::remote_view_presentation::failure_response(&reason, false);
            let _ = stream.write_all(response.as_bytes()).await;
        }
    }
}

/// Join the retained logical target and its current owned route before relaying.
/// There is no daemon acquisition, browser launch, target replay or fallback lane.
async fn mode_reply(id: &str, request: ModeRequest) -> Result<Value, String> {
    let handoff = id.to_owned();
    let slot = request.slot;
    let inspect = request.choice == "inspect";
    let (target, mode) = tokio::task::spawn_blocking(move || {
        let store = BrowserSessionSqliteStore::open_read_only(
            &BrowserSessionSqliteStore::default_sqlite_path()?,
        )?;
        let state = store.load_session_state()?;
        let link = state
            .remote_view_tab_handoffs
            .get(&handoff)
            .ok_or("remote_view_tab_handoff_missing")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "remote_view_clock_invalid")?
            .as_millis() as u64;
        link.check_link_at(now)?;
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&state, &handoff)?;
        let digest =
            agent_browser_service_model::remote_view_native_target_digest(&handoff, &target)?;
        let route =
            crate::native::browser_session_remote_view::resolve_handoff_desktop_route(&handoff)?;
        if route != format!("/{slot}") {
            return Err("remote_view_native_mode_desktop_changed".to_string());
        }
        let current = store.load_session_state()?;
        let current_target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&current, &handoff)?;
        current
            .remote_view_tab_handoffs
            .get(&handoff)
            .ok_or("remote_view_tab_handoff_missing")?
            .check_link_at(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| "remote_view_clock_invalid")?
                    .as_millis() as u64,
            )?;
        if agent_browser_service_model::remote_view_native_target_digest(&handoff, &current_target)?
            != digest
        {
            return Err("remote_view_native_mode_target_changed".into());
        }
        let mode = if inspect {
            store.inspect_task_mode(&target.session.id)?["mode"].clone()
        } else {
            Value::Null
        };
        Ok::<_, String>((target, mode))
    })
    .await
    .map_err(|_| "remote_view_native_mode_read_unavailable")??;
    let mut command = json!({"action":"browser_session_mode_convert", "sessionId":target.session.id,
        "sessionName":target.session.name,"sourceProfileId":target.session.profile_id,"browserId":target.browser.id,"choice":request.choice});
    if !inspect {
        command["operationId"] = json!(request.operation_id);
        command["mode"] = json!(request.mode);
        command["ownerConfirmed"] = json!(request.owner_confirmed.unwrap_or(false));
    }
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        super::http::relay_command_to_daemon(&target.session.name, &command.to_string()),
    )
    .await
    .map_err(|_| "remote_view_native_mode_result_unknown")??;
    if response.len() > 128 * 1024 {
        return Err("remote_view_native_mode_reply_invalid".into());
    }
    let reply: Value =
        serde_json::from_str(&response).map_err(|_| "remote_view_native_mode_reply_invalid")?;
    if reply["success"] == false {
        let code = reply["error"]
            .as_str()
            .unwrap_or("remote_view_native_mode_refused")
            .split(':')
            .next()
            .unwrap_or("remote_view_native_mode_refused");
        let code = if code.len() <= 128
            && (code.starts_with("mode_operation_") || code.starts_with("browser_operation_"))
            && code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            code
        } else {
            "remote_view_native_mode_refused"
        };
        return Ok(json!({"success":false,"error":code}));
    }
    if reply["success"] != true {
        return Err("remote_view_native_mode_reply_invalid".into());
    }
    if inspect {
        if reply["data"]["schemaVersion"] != "agent-browser.browser-operations.v1" {
            return Err("remote_view_native_mode_reply_invalid".into());
        }
        let operations = reply["data"]["operations"]
            .as_array()
            .ok_or("remote_view_native_mode_reply_invalid")?;
        if operations.iter().any(|row| {
            row["schemaVersion"] != "agent-browser.mode-operation.v1"
                || !row["operationId"].is_string()
        }) {
            return Err("remote_view_native_mode_reply_invalid".into());
        }
        let active: Vec<_> = operations
            .iter()
            .filter(|row| !matches!(row["phase"].as_str(), Some("completed" | "cancelled")))
            .collect();
        return Ok(
            json!({"success":true,"data":{"schemaVersion":"agent-browser.handoff-mode.v1","mode":mode,"operations":active}}),
        );
    }
    if reply["data"]["schemaVersion"] == "agent-browser.mode-operation.v1"
        && reply["data"]["operationId"] == command["operationId"]
        || reply["data"]["schemaVersion"] == "agent-browser.browser-operations.v1"
            && reply["data"]["state"] == "busy"
    {
        Ok(reply)
    } else {
        Err("remote_view_native_mode_reply_invalid".into())
    }
}
