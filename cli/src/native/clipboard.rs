use std::time::Duration;
use std::{future::Future, result::Result as StdResult};

use serde::Deserialize;
use serde_json::json;

use super::cdp::client::{CdpClient, CdpCommandError};
use super::cdp::types::EvaluateResult;

const CDP_RESPONSE_GRACE: Duration = Duration::from_millis(250);
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(3);
pub const DEFAULT_WRITE_CAPTURE_LIMIT: usize = 4096;
pub const DEFAULT_WRITE_CAPTURE_ACTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of an explicitly requested, bounded `Clipboard.writeText` capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardWriteCaptureOutcome {
    pub supported: bool,
    pub invoked: bool,
    pub text: Option<String>,
    pub truncated: bool,
    pub original_length: usize,
    pub restored: bool,
    pub reason: Option<String>,
}

/// Installed page-scoped capture that must be finalized to restore the
/// original `Clipboard.prototype.writeText` descriptor.
pub struct ClipboardWriteCapture<'a> {
    client: &'a CdpClient,
    session_id: &'a str,
    token: String,
    supported: bool,
}

impl<'a> ClipboardWriteCapture<'a> {
    pub async fn begin(client: &'a CdpClient, session_id: &'a str) -> Result<Self, String> {
        let token = format!("__agentBrowserClipboardCapture_{}", uuid::Uuid::new_v4());
        let token_json = serde_json::to_string(&token).map_err(|error| error.to_string())?;
        let expression = format!(
            r#"(() => {{
                const token = {token_json};
                const clipboard = globalThis.navigator?.clipboard;
                const prototype = globalThis.Clipboard?.prototype ?? (clipboard ? Object.getPrototypeOf(clipboard) : null);
                if (!clipboard || !prototype) {{
                    globalThis[token] = {{ supported: false, reason: "clipboard_backend_unavailable" }};
                    return {{ supported: false }};
                }}
                // Capture only calls to Clipboard.prototype.writeText.
                const descriptor = Object.getOwnPropertyDescriptor(prototype, "writeText");
                if (!descriptor || typeof descriptor.value !== "function" || descriptor.configurable === false) {{
                    globalThis[token] = {{ supported: false, reason: "write_text_not_patchable" }};
                    return {{ supported: false }};
                }}
                const state = {{ supported: true, prototype, descriptor, calls: [] }};
                globalThis[token] = state;
                Object.defineProperty(prototype, "writeText", {{
                    ...descriptor,
                    value: function(text) {{
                        state.calls.push(String(text));
                        return Reflect.apply(descriptor.value, this, arguments);
                    }}
                }});
                return {{ supported: true }};
            }})()"#
        );
        let response = evaluate_capture_script(client, session_id, expression).await?;
        let supported = response
            .get("supported")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        Ok(Self {
            client,
            session_id,
            token,
            supported,
        })
    }

    pub async fn finish(self, max_chars: usize) -> Result<ClipboardWriteCaptureOutcome, String> {
        let token_json = serde_json::to_string(&self.token).map_err(|error| error.to_string())?;
        let expression = format!(
            r#"(() => {{
                const token = {token_json};
                const state = globalThis[token];
                if (!state) {{
                    return {{ supported: false, invoked: false, text: null, truncated: false, originalLength: 0, restored: false, reason: "capture_state_missing" }};
                }}
                let restored = !state.supported;
                if (state.supported) {{
                    try {{
                        Object.defineProperty(state.prototype, "writeText", state.descriptor);
                        restored = true;
                    }} catch (_) {{
                        restored = false;
                    }}
                }}
                delete globalThis[token];
                const invoked = Boolean(state.supported && state.calls.length);
                const text = invoked ? state.calls[state.calls.length - 1] : null;
                const characters = text === null ? [] : Array.from(text);
                const bounded = text === null ? null : characters.slice(0, {max_chars}).join("");
                return {{
                    supported: Boolean(state.supported),
                    invoked,
                    text: bounded,
                    truncated: characters.length > {max_chars},
                    originalLength: characters.length,
                    restored,
                    reason: state.reason ?? null
                }};
            }})()"#
        );
        let value = evaluate_capture_script(self.client, self.session_id, expression).await?;
        let outcome: ClipboardWriteCaptureWire = serde_json::from_value(value)
            .map_err(|error| format!("Invalid clipboard capture response: {error}"))?;
        if self.supported && !outcome.restored {
            return Err(
                "Clipboard write capture could not restore Clipboard.prototype.writeText"
                    .to_string(),
            );
        }
        Ok(outcome.into())
    }
}

/// Runs one action between capture installation and restoration. The action
/// result is preserved so cleanup still runs when the action itself fails.
pub async fn capture_write_during<F, T>(
    client: &CdpClient,
    session_id: &str,
    max_chars: usize,
    action_timeout: Duration,
    action: F,
) -> Result<(StdResult<T, String>, ClipboardWriteCaptureOutcome), String>
where
    F: Future<Output = StdResult<T, String>>,
{
    let capture = ClipboardWriteCapture::begin(client, session_id).await?;
    let action_result = match tokio::time::timeout(action_timeout, action).await {
        Ok(result) => result,
        Err(_) => Err(format!(
            "Click did not complete within the {} ms clipboard capture deadline",
            action_timeout.as_millis()
        )),
    };
    let outcome = capture.finish(max_chars).await?;
    Ok((action_result, outcome))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClipboardWriteCaptureWire {
    supported: bool,
    invoked: bool,
    text: Option<String>,
    truncated: bool,
    original_length: usize,
    restored: bool,
    reason: Option<String>,
}

impl From<ClipboardWriteCaptureWire> for ClipboardWriteCaptureOutcome {
    fn from(value: ClipboardWriteCaptureWire) -> Self {
        Self {
            supported: value.supported,
            invoked: value.invoked,
            text: value.text,
            truncated: value.truncated,
            original_length: value.original_length,
            restored: value.restored,
            reason: value.reason,
        }
    }
}

async fn evaluate_capture_script(
    client: &CdpClient,
    session_id: &str,
    expression: String,
) -> Result<serde_json::Value, String> {
    let response: EvaluateResult = client
        .send_command_typed(
            "Runtime.evaluate",
            &super::cdp::types::EvaluateParams {
                expression,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;
    if let Some(details) = response.exception_details {
        return Err(format!(
            "Clipboard capture evaluation failed: {}",
            details.text
        ));
    }
    response
        .result
        .value
        .ok_or_else(|| "Clipboard capture evaluation returned no value".to_string())
}

/// Successful browser clipboard text read. Empty text remains successful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardReadOutcome {
    pub text: String,
    pub empty: bool,
}

/// Stable failure classification exposed in clipboard diagnostic JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardFailureCode {
    PermissionDenied,
    UnresolvedPromise,
    BackendUnavailable,
    CdpFailure,
    RecoveryFailure,
}

impl ClipboardFailureCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PermissionDenied => "permission_denied",
            Self::UnresolvedPromise => "unresolved_promise",
            Self::BackendUnavailable => "backend_unavailable",
            Self::CdpFailure => "cdp_failure",
            Self::RecoveryFailure => "recovery_failure",
        }
    }
}

/// Target-health result after an unresolved clipboard promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardRecovery {
    NotNeeded,
    SameTargetReady,
    ReplaceTabRequired,
}

impl ClipboardRecovery {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotNeeded => "not_needed",
            Self::SameTargetReady => "same_target_ready",
            Self::ReplaceTabRequired => "replace_tab_required",
        }
    }
}

#[derive(Debug)]
pub struct ClipboardReadError {
    code: ClipboardFailureCode,
    recovery: ClipboardRecovery,
    message: String,
}

impl ClipboardReadError {
    pub fn code(&self) -> ClipboardFailureCode {
        self.code
    }

    pub fn recovery(&self) -> ClipboardRecovery {
        self.recovery
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn diagnostic(&self) -> serde_json::Value {
        let recommended_action = match self.recovery {
            ClipboardRecovery::ReplaceTabRequired => "replace_affected_tab",
            ClipboardRecovery::SameTargetReady | ClipboardRecovery::NotNeeded => "none",
        };
        json!({
            "code": self.code.as_str(),
            "recovery": self.recovery.as_str(),
            "recommendedAction": recommended_action,
        })
    }

    fn new(
        code: ClipboardFailureCode,
        recovery: ClipboardRecovery,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            recovery,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ClipboardReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ClipboardReadError {}

/// Reads browser clipboard text with a renderer deadline, a longer CDP
/// transport deadline, and bounded target-health recovery.
pub async fn read_text(
    client: &CdpClient,
    session_id: &str,
    timeout: Duration,
) -> Result<ClipboardReadOutcome, ClipboardReadError> {
    let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
    let response = match client
        .send_command_with_timeout(
            "Runtime.evaluate",
            Some(json!({
                "expression": "navigator.clipboard.readText()",
                "returnByValue": true,
                "awaitPromise": true,
                "timeout": timeout_ms,
            })),
            Some(session_id),
            timeout.saturating_add(CDP_RESPONSE_GRACE),
        )
        .await
    {
        Ok(response) => response,
        Err(CdpCommandError::Timeout { .. }) => {
            let recovery = recover_after_transport_timeout(client, session_id).await;
            let code = if recovery == ClipboardRecovery::ReplaceTabRequired {
                ClipboardFailureCode::RecoveryFailure
            } else {
                ClipboardFailureCode::UnresolvedPromise
            };
            return Err(ClipboardReadError::new(
                code,
                recovery,
                "Clipboard read promise did not resolve before its deadline",
            ));
        }
        Err(error) if is_terminated_execution(&error) => {
            let message = error.to_string();
            let recovery = probe_target_health(client, session_id).await;
            let code = if recovery == ClipboardRecovery::ReplaceTabRequired {
                ClipboardFailureCode::RecoveryFailure
            } else {
                ClipboardFailureCode::UnresolvedPromise
            };
            return Err(ClipboardReadError::new(code, recovery, message));
        }
        Err(error) => {
            let message = error.to_string();
            return Err(ClipboardReadError::new(
                classify_failure(&message),
                ClipboardRecovery::NotNeeded,
                message,
            ));
        }
    };
    let response: EvaluateResult = serde_json::from_value(response).map_err(|error| {
        ClipboardReadError::new(
            ClipboardFailureCode::CdpFailure,
            ClipboardRecovery::NotNeeded,
            format!("Invalid clipboard evaluation response: {error}"),
        )
    })?;

    if let Some(details) = response.exception_details {
        let message = details
            .exception
            .as_ref()
            .and_then(|exception| exception.description.as_deref())
            .unwrap_or(&details.text)
            .to_string();
        return Err(ClipboardReadError::new(
            classify_failure(&message),
            ClipboardRecovery::NotNeeded,
            format!("Clipboard evaluation error: {message}"),
        ));
    }

    let text = response
        .result
        .value
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or_else(|| {
            ClipboardReadError::new(
                ClipboardFailureCode::CdpFailure,
                ClipboardRecovery::NotNeeded,
                "Clipboard read returned a non-text result",
            )
        })?;
    Ok(ClipboardReadOutcome {
        empty: text.is_empty(),
        text,
    })
}

fn classify_failure(message: &str) -> ClipboardFailureCode {
    let message = message.to_ascii_lowercase();
    if message.contains("permission")
        || message.contains("notallowederror")
        || message.contains("not allowed")
        || message.contains("denied")
    {
        ClipboardFailureCode::PermissionDenied
    } else if message.contains("clipboard is undefined")
        || message.contains("clipboard is not available")
        || message.contains("clipboard backend")
        || message.contains("secure context")
    {
        ClipboardFailureCode::BackendUnavailable
    } else {
        ClipboardFailureCode::CdpFailure
    }
}

fn is_terminated_execution(error: &CdpCommandError) -> bool {
    let CdpCommandError::Protocol { message, .. } = error else {
        return false;
    };
    let message = message.to_ascii_lowercase();
    message.contains("execution was terminated")
        || message.contains("execution terminated")
        || message.contains("execution timed out")
}

async fn recover_after_transport_timeout(
    client: &CdpClient,
    session_id: &str,
) -> ClipboardRecovery {
    let recovery_timeout = Duration::from_secs(1);
    if client
        .send_command_with_timeout(
            "Runtime.terminateExecution",
            Some(json!({})),
            Some(session_id),
            recovery_timeout,
        )
        .await
        .is_err()
    {
        return ClipboardRecovery::ReplaceTabRequired;
    }

    probe_target_health(client, session_id).await
}

async fn probe_target_health(client: &CdpClient, session_id: &str) -> ClipboardRecovery {
    let recovery_timeout = Duration::from_secs(1);
    let probe = client
        .send_command_with_timeout(
            "Runtime.evaluate",
            Some(json!({
                "expression": "true",
                "returnByValue": true,
                "awaitPromise": false,
            })),
            Some(session_id),
            recovery_timeout,
        )
        .await;
    let Ok(probe) = probe else {
        return ClipboardRecovery::ReplaceTabRequired;
    };
    let Ok(probe) = serde_json::from_value::<EvaluateResult>(probe) else {
        return ClipboardRecovery::ReplaceTabRequired;
    };
    if probe.exception_details.is_none() && probe.result.value == Some(json!(true)) {
        ClipboardRecovery::SameTargetReady
    } else {
        ClipboardRecovery::ReplaceTabRequired
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    use super::{
        capture_write_during, read_text, ClipboardFailureCode, ClipboardRecovery,
        ClipboardWriteCapture, DEFAULT_WRITE_CAPTURE_LIMIT,
    };
    use crate::native::cdp::client::CdpClient;

    #[tokio::test]
    async fn empty_clipboard_text_is_a_successful_read() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let command = websocket.next().await.unwrap().unwrap();
            let command: serde_json::Value =
                serde_json::from_str(command.to_text().unwrap()).unwrap();
            assert_eq!(command["method"], "Runtime.evaluate");
            assert_eq!(
                command["params"]["expression"],
                "navigator.clipboard.readText()"
            );
            assert_eq!(command["params"]["awaitPromise"], true);
            assert_eq!(command["params"]["timeout"], 25);

            websocket
                .send(Message::Text(
                    json!({
                        "id": command["id"],
                        "result": {
                            "result": {
                                "type": "string",
                                "value": ""
                            }
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let outcome = read_text(&client, "session-1", Duration::from_millis(25))
            .await
            .unwrap();
        assert_eq!(outcome.text, "");
        assert!(outcome.empty);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn write_capture_is_bounded_and_restores_the_original_descriptor() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let begin = websocket.next().await.unwrap().unwrap();
            let begin: serde_json::Value = serde_json::from_str(begin.to_text().unwrap()).unwrap();
            assert_eq!(begin["method"], "Runtime.evaluate");
            assert!(begin["params"]["expression"]
                .as_str()
                .unwrap()
                .contains("Clipboard.prototype.writeText"));
            websocket
                .send(Message::Text(
                    json!({
                        "id": begin["id"],
                        "result": { "result": { "type": "object", "value": {
                            "supported": true
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();

            let finish = websocket.next().await.unwrap().unwrap();
            let finish: serde_json::Value =
                serde_json::from_str(finish.to_text().unwrap()).unwrap();
            assert_eq!(finish["method"], "Runtime.evaluate");
            assert!(finish["params"]["expression"]
                .as_str()
                .unwrap()
                .contains("Object.defineProperty"));
            websocket
                .send(Message::Text(
                    json!({
                        "id": finish["id"],
                        "result": { "result": { "type": "object", "value": {
                            "supported": true,
                            "invoked": true,
                            "text": "bounded",
                            "truncated": true,
                            "originalLength": 5000,
                            "restored": true,
                            "reason": null
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let capture = ClipboardWriteCapture::begin(&client, "session-1")
            .await
            .unwrap();
        let outcome = capture.finish(DEFAULT_WRITE_CAPTURE_LIMIT).await.unwrap();
        assert!(outcome.supported);
        assert!(outcome.invoked);
        assert_eq!(outcome.text.as_deref(), Some("bounded"));
        assert!(outcome.truncated);
        assert_eq!(outcome.original_length, 5000);
        assert!(outcome.restored);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn write_capture_restores_after_the_wrapped_action_fails() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let begin = websocket.next().await.unwrap().unwrap();
            let begin: serde_json::Value = serde_json::from_str(begin.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": begin["id"],
                        "result": { "result": { "type": "object", "value": {
                            "supported": true
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();

            let finish = websocket.next().await.unwrap().unwrap();
            let finish: serde_json::Value =
                serde_json::from_str(finish.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": finish["id"],
                        "result": { "result": { "type": "object", "value": {
                            "supported": true,
                            "invoked": false,
                            "text": null,
                            "truncated": false,
                            "originalLength": 0,
                            "restored": true,
                            "reason": null
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let (action_result, outcome) = capture_write_during(
            &client,
            "session-1",
            DEFAULT_WRITE_CAPTURE_LIMIT,
            Duration::from_secs(1),
            async { Err::<(), String>("click failed".to_string()) },
        )
        .await
        .unwrap();
        assert_eq!(action_result.unwrap_err(), "click failed");
        assert!(outcome.restored);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn write_capture_restores_after_the_wrapped_action_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let begin = websocket.next().await.unwrap().unwrap();
            let begin: serde_json::Value = serde_json::from_str(begin.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": begin["id"],
                        "result": { "result": { "type": "object", "value": {
                            "supported": true
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();

            let finish = websocket.next().await.unwrap().unwrap();
            let finish: serde_json::Value =
                serde_json::from_str(finish.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": finish["id"],
                        "result": { "result": { "type": "object", "value": {
                            "supported": true,
                            "invoked": false,
                            "text": null,
                            "truncated": false,
                            "originalLength": 0,
                            "restored": true,
                            "reason": null
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let (action_result, outcome) = capture_write_during(
            &client,
            "session-1",
            DEFAULT_WRITE_CAPTURE_LIMIT,
            Duration::from_millis(10),
            std::future::pending::<Result<(), String>>(),
        )
        .await
        .unwrap();
        assert!(action_result.unwrap_err().contains("capture deadline"));
        assert!(outcome.restored);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn unresolved_read_terminates_execution_and_proves_same_target_health() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let read = websocket.next().await.unwrap().unwrap();
            let read: serde_json::Value = serde_json::from_str(read.to_text().unwrap()).unwrap();
            assert_eq!(read["method"], "Runtime.evaluate");

            let terminate = websocket.next().await.unwrap().unwrap();
            let terminate: serde_json::Value =
                serde_json::from_str(terminate.to_text().unwrap()).unwrap();
            assert_eq!(terminate["method"], "Runtime.terminateExecution");
            websocket
                .send(Message::Text(
                    json!({ "id": terminate["id"], "result": {} }).to_string(),
                ))
                .await
                .unwrap();

            let probe = websocket.next().await.unwrap().unwrap();
            let probe: serde_json::Value = serde_json::from_str(probe.to_text().unwrap()).unwrap();
            assert_eq!(probe["method"], "Runtime.evaluate");
            assert_eq!(probe["params"]["expression"], "true");
            websocket
                .send(Message::Text(
                    json!({
                        "id": probe["id"],
                        "result": {
                            "result": {
                                "type": "boolean",
                                "value": true
                            }
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let error = read_text(&client, "session-1", Duration::from_millis(25))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ClipboardFailureCode::UnresolvedPromise);
        assert_eq!(error.recovery(), ClipboardRecovery::SameTargetReady);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn failed_recovery_requires_replacement_tab_with_structured_diagnostic() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let _read = websocket.next().await.unwrap().unwrap();
            let terminate = websocket.next().await.unwrap().unwrap();
            let terminate: serde_json::Value =
                serde_json::from_str(terminate.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": terminate["id"],
                        "error": {
                            "code": -32000,
                            "message": "termination unavailable"
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let error = read_text(&client, "session-1", Duration::from_millis(25))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ClipboardFailureCode::RecoveryFailure);
        assert_eq!(error.recovery(), ClipboardRecovery::ReplaceTabRequired);
        assert_eq!(
            error.diagnostic(),
            json!({
                "code": "recovery_failure",
                "recovery": "replace_tab_required",
                "recommendedAction": "replace_affected_tab"
            })
        );

        server.await.unwrap();
    }

    #[tokio::test]
    async fn runtime_terminated_read_probes_health_without_arming_next_execution() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let read = websocket.next().await.unwrap().unwrap();
            let read: serde_json::Value = serde_json::from_str(read.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": read["id"],
                        "error": {
                            "code": -32000,
                            "message": "Execution was terminated"
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();

            let probe = websocket.next().await.unwrap().unwrap();
            let probe: serde_json::Value = serde_json::from_str(probe.to_text().unwrap()).unwrap();
            assert_eq!(probe["method"], "Runtime.evaluate");
            assert_eq!(probe["params"]["expression"], "true");
            websocket
                .send(Message::Text(
                    json!({
                        "id": probe["id"],
                        "result": {
                            "result": {
                                "type": "boolean",
                                "value": true
                            }
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let error = read_text(&client, "session-1", Duration::from_millis(25))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ClipboardFailureCode::UnresolvedPromise);
        assert_eq!(error.recovery(), ClipboardRecovery::SameTargetReady);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn permission_denial_has_a_stable_failure_code() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let read = websocket.next().await.unwrap().unwrap();
            let read: serde_json::Value = serde_json::from_str(read.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": read["id"],
                        "error": {
                            "code": -32000,
                            "message": "NotAllowedError: Read permission denied"
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let error = read_text(&client, "session-1", Duration::from_millis(25))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ClipboardFailureCode::PermissionDenied);
        assert_eq!(error.recovery(), ClipboardRecovery::NotNeeded);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn unavailable_clipboard_backend_has_a_stable_failure_code() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let read = websocket.next().await.unwrap().unwrap();
            let read: serde_json::Value = serde_json::from_str(read.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": read["id"],
                        "result": {
                            "result": { "type": "undefined" },
                            "exceptionDetails": {
                                "text": "Uncaught",
                                "exception": {
                                    "type": "object",
                                    "description": "TypeError: navigator.clipboard is undefined"
                                }
                            }
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let error = read_text(&client, "session-1", Duration::from_millis(25))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ClipboardFailureCode::BackendUnavailable);
        assert_eq!(error.recovery(), ClipboardRecovery::NotNeeded);

        server.await.unwrap();
    }
}
