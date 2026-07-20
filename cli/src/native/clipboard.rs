use std::time::Duration;

use serde_json::json;

use super::cdp::client::{CdpClient, CdpCommandError};
use super::cdp::types::EvaluateResult;

const CDP_RESPONSE_GRACE: Duration = Duration::from_millis(250);
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(3);

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

    use super::{read_text, ClipboardFailureCode, ClipboardRecovery};
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
