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
            && if matches!(self.choice.as_str(), "inspect" | "inspect_control") {
                self.operation_id.is_none() && self.mode.is_none() && self.owner_confirmed.is_none()
            } else if self.choice == "return_handoff" {
                self.operation_id
                    .as_ref()
                    .is_some_and(|id| !id.is_empty() && id.len() <= 128)
                    && self.mode.is_none()
                    && self.owner_confirmed.is_none()
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
    let inspect = matches!(
        request.choice.as_str(),
        "inspect" | "inspect_control" | "return_handoff"
    );
    let generation = request.generation;
    let returning = (request.choice == "return_handoff")
        .then(|| request.operation_id.clone())
        .flatten();
    let (target, mode, control_target) = tokio::task::spawn_blocking(move || {
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
        let original = store.original_browser_identity(&current, &target.browser);
        let mode = if inspect {
            store.inspect_task_mode(&target.session.id)?["mode"].clone()
        } else {
            Value::Null
        };
        // Input authority remains Remote View-owned. Export only fresh, opaque
        // evidence from this existing authenticated read-only owner callback.
        let after = store.load_session_state()?;
        let after_target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&after, &handoff)?;
        let after_link = after
            .remote_view_tab_handoffs
            .get(&handoff)
            .ok_or("remote_view_tab_handoff_missing")?;
        after_link.check_link_at(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "remote_view_clock_invalid")?
                .as_millis() as u64,
        )?;
        if agent_browser_service_model::remote_view_native_target_digest(&handoff, &after_target)?
            != digest
            || after_link.native_observation != link.native_observation
        {
            return Err("remote_view_native_mode_target_changed".into());
        }
        let current_original = store.original_browser_identity(&after, &after_target.browser);
        let binding = after_link.native_observation.as_ref().filter(|binding| {
            binding.target_digest == digest
                && binding.observation.as_ref().is_some_and(|observation| {
                    observation.target_digest == digest
                        && observation.target == binding.provider_target
                })
        });
        let mut control_target = if inspect {
            native_control_target(
                mode.as_str().unwrap_or("unverified"),
                generation,
                &digest,
                original.as_ref(),
                current_original.as_ref(),
                binding.map(|b| &b.provider_target),
            )
        } else {
            None
        };
        if let Some(proof) = control_target.as_mut() {
            use crate::native::browser_host_control as control;
            if let Some(request) = &returning {
                control::return_handoff(&target.browser.id, &handoff, &digest, request)?;
            }
            proof["handoffIntent"] = json!(control::handoff_intent(
                &target.browser.id,
                &handoff,
                &digest
            )?);
        } else if returning.is_some() {
            return Err("browser_handoff_target_unconfirmed".into());
        }
        Ok::<_, String>((target, mode, control_target))
    })
    .await
    .map_err(|_| "remote_view_native_mode_read_unavailable")??;
    // The snapshot above retains the provider assignment and fresh process checks.
    // Control input does not consume operation history or wait on its daemon queue.
    finish_current_inspection(
        &request.choice,
        mode.clone(),
        control_target.clone(),
        async {
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
                // The owner relay may have waited. Do not export input target evidence
                // unless the same original process and binding remain current afterward.
                let control_target = if let Some(expected) = control_target {
                    let handoff = id.to_owned();
                    tokio::task::spawn_blocking(move || {
                        let store = BrowserSessionSqliteStore::open_read_only(
                            &BrowserSessionSqliteStore::default_sqlite_path()?,
                        )?;
                        let state = store.load_session_state()?;
                        let target =
                            agent_browser_service_model::resolve_remote_view_tab_handoff(&state, &handoff)?;
                        let digest = agent_browser_service_model::remote_view_native_target_digest(
                            &handoff, &target,
                        )?;
                        let link = state
                            .remote_view_tab_handoffs
                            .get(&handoff)
                            .ok_or("remote_view_tab_handoff_missing")?;
                        link.check_link_at(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_err(|_| "remote_view_clock_invalid")?
                                .as_millis() as u64,
                        )?;
                        let original = store.original_browser_identity(&state, &target.browser);
                        let mode = store.inspect_task_mode(&target.session.id)?;
                        let current = store.load_session_state()?;
                        let current_target = agent_browser_service_model::resolve_remote_view_tab_handoff(
                            &current, &handoff,
                        )?;
                        if agent_browser_service_model::remote_view_native_target_digest(
                            &handoff,
                            &current_target,
                        )? != digest
                            || current
                                .remote_view_tab_handoffs
                                .get(&handoff)
                                .and_then(|l| l.native_observation.as_ref())
                                != link.native_observation.as_ref()
                        {
                            return Ok::<_, String>(None);
                        }
                        let current_original =
                            store.original_browser_identity(&current, &current_target.browser);
                        let binding = link
                            .native_observation
                            .as_ref()
                            .filter(|b| b.target_digest == digest);
                        let mut observed = native_control_target(
                            mode["mode"].as_str().unwrap_or("unverified"),
                            generation,
                            &digest,
                            original.as_ref(),
                            current_original.as_ref(),
                            binding.map(|b| &b.provider_target),
                        );
                        if let Some(proof) = observed.as_mut() {
                            proof["handoffIntent"] = json!(crate::native::browser_host_control::handoff_intent(
                                &current_target.browser.id, &handoff, &digest
                            )?);
                        }
                        Ok(observed.filter(|observed| observed == &expected))
                    })
                    .await
                    .map_err(|_| "remote_view_native_mode_read_unavailable")??
                } else {
                    None
                };
                return Ok(
                    json!({"success":true,"data":{"schemaVersion":"agent-browser.handoff-mode.v1","mode":mode,"operations":active,"nativeControlTarget":control_target}}),
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
        },
    )
    .await
}

async fn finish_current_inspection(
    choice: &str,
    mode: Value,
    control_target: Option<Value>,
    continuation: impl std::future::Future<Output = Result<Value, String>>,
) -> Result<Value, String> {
    if matches!(choice, "inspect_control" | "return_handoff") {
        return Ok(json!({"success":true,"data":{
            "schemaVersion":"agent-browser.handoff-mode.v1",
            "mode":mode,"operations":[],"nativeControlTarget":control_target
        }}));
    }
    continuation.await
}

/// No caller-provided locator becomes input authority. Remote View must join
/// this private evidence to its current authenticated page and Native owner.
fn native_control_target(
    mode: &str,
    generation: u64,
    digest: &str,
    original: Option<&agent_browser_service_model::RecordedProcessIdentity>,
    current: Option<&agent_browser_service_model::RecordedProcessIdentity>,
    target: Option<&agent_browser_service_model::RemoteViewApplicationTarget>,
) -> Option<Value> {
    use sha2::{Digest, Sha256};
    let original = original?;
    let target = target?;
    if !matches!(mode, "cdp" | "cdp_free")
        || generation != target.viewing_generation
        || generation == 0
        || current != Some(original)
        || original.pid == 0
        || original.start_token.is_empty()
        || original.executable_path.is_none()
        || digest.len() != 64
        || !digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let bytes = serde_json::to_vec(original).ok()?;
    Some(
        json!({"schemaVersion":"agent-browser.native-control-target.v1",
        "targetDigest":digest,"processInstanceDigest":format!("{:x}",Sha256::digest(bytes)),
        "target":target}),
    )
}

#[cfg(test)]
mod control_target_tests {
    use super::*;

    #[test]
    fn native_control_target_requires_exact_process_and_current_binding() {
        let process = agent_browser_service_model::RecordedProcessIdentity {
            pid: 41,
            start_token: "boot:100".into(),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: Some("chrome".into()),
        };
        let target = agent_browser_service_model::RemoteViewApplicationTarget {
            assignment_id: "assignment".into(),
            registration_id: "registration".into(),
            desktop_id: "11111111-1111-4111-8111-111111111111".into(),
            lifecycle_generation: 1,
            viewing_desktop_id: "22222222-2222-4222-8222-222222222222".into(),
            viewing_generation: 4,
        };
        let proof = native_control_target(
            "cdp_free",
            4,
            &"a".repeat(64),
            Some(&process),
            Some(&process),
            Some(&target),
        )
        .unwrap();
        assert_eq!(
            proof["schemaVersion"],
            "agent-browser.native-control-target.v1"
        );
        assert_eq!(proof["target"]["viewingGeneration"], 4);
        assert_eq!(proof["processInstanceDigest"].as_str().unwrap().len(), 64);
        assert!(native_control_target(
            "unverified",
            4,
            &"a".repeat(64),
            Some(&process),
            Some(&process),
            Some(&target)
        )
        .is_none());
        assert!(native_control_target(
            "cdp",
            5,
            &"a".repeat(64),
            Some(&process),
            Some(&process),
            Some(&target)
        )
        .is_none());
        let mut changed = process.clone();
        changed.start_token = "boot:101".into();
        assert!(native_control_target(
            "cdp",
            4,
            &"a".repeat(64),
            Some(&process),
            Some(&changed),
            Some(&target)
        )
        .is_none());
        assert!(native_control_target(
            "cdp",
            4,
            &"a".repeat(64),
            None,
            Some(&process),
            Some(&target)
        )
        .is_none());
        assert!(native_control_target(
            "cdp",
            4,
            &"a".repeat(64),
            Some(&process),
            Some(&process),
            None
        )
        .is_none());
    }
    #[tokio::test]
    async fn current_control_inspection_does_not_wait_for_operation_history() {
        use std::{cell::Cell, time::Duration};
        let called = Cell::new(false);
        let blocked_history = async {
            called.set(true);
            std::future::pending::<Result<Value, String>>().await
        };
        let target = json!({"schemaVersion":"agent-browser.native-control-target.v1"});
        let reply = tokio::time::timeout(
            Duration::from_millis(50),
            finish_current_inspection(
                "inspect_control",
                json!("cdp"),
                Some(target.clone()),
                blocked_history,
            ),
        )
        .await
        .expect("fresh control proof must not wait for operation history")
        .unwrap();
        assert!(!called.get());
        assert_eq!(reply["data"]["nativeControlTarget"], target);
        assert_eq!(reply["data"]["operations"], json!([]));
        assert_eq!(reply["data"]["mode"], "cdp");

        let normal = finish_current_inspection("inspect", json!("cdp"), None, async {
            Ok(json!({"normalHistory":true}))
        })
        .await
        .unwrap();
        assert_eq!(normal["normalHistory"], true);
        let absent =
            finish_current_inspection("inspect_control", json!("unverified"), None, async {
                panic!("control inspection cannot relay a mode operation")
            })
            .await
            .unwrap();
        assert!(absent["data"]["nativeControlTarget"].is_null());
    }

    #[test]
    fn control_inspection_accepts_no_effect_fields() {
        for choice in ["inspect", "inspect_control"] {
            let valid: ModeRequest =
                serde_json::from_value(json!({"slot":1,"generation":1,"choice":choice})).unwrap();
            assert!(valid.valid());
            for field in ["operationId", "mode", "ownerConfirmed"] {
                let mut value = json!({"slot":1,"generation":1,"choice":choice});
                value[field] = if field == "ownerConfirmed" {
                    json!(true)
                } else {
                    json!("unexpected")
                };
                assert!(!serde_json::from_value::<ModeRequest>(value)
                    .unwrap()
                    .valid());
            }
        }
    }
}
