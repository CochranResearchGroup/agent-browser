use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

use super::types::{CdpCommand, CdpEvent, CdpMessage};

type PendingMap = Arc<StdMutex<HashMap<u64, oneshot::Sender<CdpMessage>>>>;

fn lock_pending(
    pending: &PendingMap,
) -> std::sync::MutexGuard<'_, HashMap<u64, oneshot::Sender<CdpMessage>>> {
    pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct PendingCommandRegistration {
    id: u64,
    pending: PendingMap,
    active: bool,
}

impl PendingCommandRegistration {
    fn insert(id: u64, sender: oneshot::Sender<CdpMessage>, pending: PendingMap) -> Self {
        lock_pending(&pending).insert(id, sender);
        Self {
            id,
            pending,
            active: true,
        }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for PendingCommandRegistration {
    fn drop(&mut self) {
        if self.active {
            lock_pending(&self.pending).remove(&self.id);
        }
    }
}

const DEFAULT_COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Stable command-lifecycle failures for callers that need more than the
/// compatibility string returned by `send_command`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdpCommandError {
    Serialization {
        method: String,
        message: String,
    },
    Transport {
        method: String,
        message: String,
    },
    ResponseChannelClosed {
        method: String,
    },
    Timeout {
        method: String,
        timeout: std::time::Duration,
    },
    Protocol {
        method: String,
        message: String,
    },
}

impl std::fmt::Display for CdpCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization { message, .. } => {
                write!(formatter, "Failed to serialize CDP command: {message}")
            }
            Self::Transport { message, .. } => {
                write!(formatter, "Failed to send CDP command: {message}")
            }
            Self::ResponseChannelClosed { .. } => {
                write!(formatter, "CDP response channel closed")
            }
            Self::Timeout { method, .. } => {
                write!(formatter, "CDP command timed out: {method}")
            }
            Self::Protocol { method, message } => {
                write!(formatter, "CDP error ({method}): {message}")
            }
        }
    }
}

impl std::error::Error for CdpCommandError {}

/// Interval between WebSocket ping frames sent to keep the connection alive
/// through intermediate proxies (reverse proxies, load balancers, service meshes).
const WS_KEEPALIVE_INTERVAL_SECS: u64 = 30;

/// Raw incoming CDP message (text) broadcast to all subscribers.
/// Used by the inspect proxy to forward responses and events to DevTools.
#[derive(Debug, Clone)]
pub struct RawCdpMessage {
    pub text: String,
    pub session_id: Option<String>,
}

pub struct CdpClient {
    ws_tx: Arc<
        Mutex<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
        >,
    >,
    next_id: AtomicU64,
    pending: PendingMap,
    event_tx: broadcast::Sender<CdpEvent>,
    raw_tx: broadcast::Sender<RawCdpMessage>,
    _reader_handle: tokio::task::JoinHandle<()>,
    _keepalive_handle: tokio::task::JoinHandle<()>,
}

impl CdpClient {
    pub async fn connect(url: &str) -> Result<Self, String> {
        Self::connect_with_headers(url, None).await
    }

    pub async fn connect_with_headers(
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<Self, String> {
        let mut request = url
            .into_client_request()
            .map_err(|e| format!("Invalid WebSocket URL: {}", e))?;

        if let Some(hdrs) = headers {
            let req_headers = request.headers_mut();
            for (key, value) in hdrs {
                if let (Ok(name), Ok(val)) = (
                    key.parse::<tokio_tungstenite::tungstenite::http::header::HeaderName>(),
                    value.parse::<tokio_tungstenite::tungstenite::http::header::HeaderValue>(),
                ) {
                    req_headers.insert(name, val);
                }
            }
        }

        let ws_config = WebSocketConfig {
            max_message_size: None,
            max_frame_size: None,
            ..Default::default()
        };

        let (ws_stream, _) =
            tokio_tungstenite::connect_async_with_config(request, Some(ws_config), false)
                .await
                .map_err(|e| format!("CDP WebSocket connect failed: {}", e))?;

        enable_tcp_keepalive(ws_stream.get_ref());

        let (ws_tx, mut ws_rx) = ws_stream.split();
        let ws_tx = Arc::new(Mutex::new(ws_tx));

        let pending: PendingMap = Arc::new(StdMutex::new(HashMap::new()));
        let (event_tx, _) = broadcast::channel(4096);
        let (raw_tx, _) = broadcast::channel(4096);

        let pending_clone = pending.clone();
        let event_tx_clone = event_tx.clone();
        let raw_tx_clone = raw_tx.clone();

        // Notify used to stop the keepalive task when the reader loop exits.
        let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);

        let reader_handle = tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                // Accept both Text and Binary frames — remote CDP proxies
                // (e.g. Browserless) may send responses as Binary frames.
                let msg = match msg {
                    Ok(Message::Text(text)) => text,
                    Ok(Message::Binary(data)) => match String::from_utf8(data) {
                        Ok(text) => text,
                        Err(_) => continue,
                    },
                    Ok(Message::Close(frame)) => {
                        if std::env::var("AGENT_BROWSER_DEBUG").is_ok() {
                            let reason = frame
                                .as_ref()
                                .map(|f| format!("code={}, reason={}", f.code, f.reason))
                                .unwrap_or_else(|| "no frame".to_string());
                            let _ =
                                writeln!(std::io::stderr(), "[cdp] WebSocket Close: {}", reason);
                        }
                        break;
                    }
                    Ok(Message::Pong(_)) => continue,
                    Ok(_) => continue,
                    Err(e) => {
                        if std::env::var("AGENT_BROWSER_DEBUG").is_ok() {
                            let _ = writeln!(std::io::stderr(), "[cdp] WebSocket Error: {}", e);
                        }
                        break;
                    }
                };

                // Broadcast raw message for inspect proxy subscribers before typed parse,
                // so messages with negative IDs (used by the inspect proxy) are still delivered.
                if raw_tx_clone.receiver_count() > 0 {
                    let session_id = serde_json::from_str::<serde_json::Value>(&msg)
                        .ok()
                        .and_then(|v| v.get("sessionId")?.as_str().map(String::from));
                    let _ = raw_tx_clone.send(RawCdpMessage {
                        text: msg.clone(),
                        session_id,
                    });
                }

                let parsed: CdpMessage = match serde_json::from_str(&msg) {
                    Ok(m) => m,
                    // Expected for inspect proxy messages with negative IDs
                    // (CdpMessage.id is u64); handled via raw broadcast above.
                    Err(_) => continue,
                };

                if let Some(id) = parsed.id {
                    // Response to a command
                    let mut pending = lock_pending(&pending_clone);
                    if let Some(tx) = pending.remove(&id) {
                        let _ = tx.send(parsed);
                    }
                } else if let Some(ref method) = parsed.method {
                    // Event
                    let event = CdpEvent {
                        method: method.clone(),
                        params: parsed.params.clone().unwrap_or(Value::Null),
                        session_id: parsed.session_id.clone(),
                    };
                    let _ = event_tx_clone.send(event);
                }
            }

            // Reader loop exited (connection closed or error). Drop all pending
            // command senders so callers get an immediate channel-closed error
            // instead of waiting for the 30-second timeout.
            lock_pending(&pending_clone).clear();

            // Stop the keepalive task — the connection is gone.
            let _ = cancel_tx.send(true);
        });

        // Spawn a keepalive task that sends WebSocket Ping frames at a regular
        // interval. This prevents intermediate proxies (Envoy, nginx, OpenResty,
        // cloud load balancers) from closing idle WebSocket connections. If the
        // send fails, the connection is dead and we stop pinging.
        let keepalive_tx = ws_tx.clone();
        let keepalive_handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(WS_KEEPALIVE_INTERVAL_SECS);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = cancel_rx.changed() => break,
                }
                let mut tx = keepalive_tx.lock().await;
                if tx.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            ws_tx,
            next_id: AtomicU64::new(1),
            pending,
            event_tx,
            raw_tx,
            _reader_handle: reader_handle,
            _keepalive_handle: keepalive_handle,
        })
    }

    pub async fn send_command(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        self.send_command_with_timeout(method, params, session_id, DEFAULT_COMMAND_TIMEOUT)
            .await
            .map_err(|error| error.to_string())
    }

    /// Sends one CDP command with a caller-selected transport deadline.
    ///
    /// The pending response registration is removed on response, timeout,
    /// transport failure, channel closure, or external future cancellation.
    pub async fn send_command_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
        timeout: std::time::Duration,
    ) -> Result<Value, CdpCommandError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let method_name = method.to_string();

        let cmd = CdpCommand {
            id,
            method: method_name.clone(),
            params,
            session_id: session_id.filter(|s| !s.is_empty()).map(|s| s.to_string()),
        };

        let json = serde_json::to_string(&cmd).map_err(|error| CdpCommandError::Serialization {
            method: method_name.clone(),
            message: error.to_string(),
        })?;

        let (tx, rx) = oneshot::channel();

        let mut registration = PendingCommandRegistration::insert(id, tx, self.pending.clone());

        {
            let mut ws_tx = self.ws_tx.lock().await;
            if let Err(error) = ws_tx.send(Message::Text(json)).await {
                return Err(CdpCommandError::Transport {
                    method: method_name,
                    message: error.to_string(),
                });
            }
        }

        let response = match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(resp)) => {
                registration.disarm();
                resp
            }
            Ok(Err(_)) => {
                return Err(CdpCommandError::ResponseChannelClosed {
                    method: method_name,
                })
            }
            Err(_) => {
                return Err(CdpCommandError::Timeout {
                    method: method_name,
                    timeout,
                });
            }
        };

        if let Some(error) = response.error {
            return Err(CdpCommandError::Protocol {
                method: method_name,
                message: error.to_string(),
            });
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    #[cfg(test)]
    async fn pending_command_count(&self) -> usize {
        lock_pending(&self.pending).len()
    }

    /// Subscribe to all raw incoming CDP messages (responses + events).
    /// Used by the inspect proxy to forward traffic to the DevTools frontend.
    pub fn subscribe_raw(&self) -> broadcast::Receiver<RawCdpMessage> {
        self.raw_tx.subscribe()
    }

    /// Create a lightweight handle for the inspect WebSocket proxy.
    /// Contains only what's needed to forward messages bidirectionally.
    pub fn inspect_handle(&self) -> InspectProxyHandle {
        InspectProxyHandle {
            ws_tx: self.ws_tx.clone(),
            raw_tx: self.raw_tx.clone(),
        }
    }

    pub async fn send_command_typed<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
        session_id: Option<&str>,
    ) -> Result<R, String> {
        let params_value = serde_json::to_value(params)
            .map_err(|e| format!("Failed to serialize params: {}", e))?;
        let result = self
            .send_command(method, Some(params_value), session_id)
            .await?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to deserialize CDP response for {}: {}", method, e))
    }

    pub async fn send_command_no_params(
        &self,
        method: &str,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        self.send_command(method, None, session_id).await
    }

    /// Send raw JSON through the WebSocket without tracking a response.
    /// Used by the inspect proxy to forward DevTools frontend messages.
    pub async fn send_raw(&self, json: String) -> Result<(), String> {
        let mut ws_tx = self.ws_tx.lock().await;
        ws_tx
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send raw CDP message: {}", e))
    }
}

type WsTx = Arc<
    Mutex<
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    >,
>;

/// Lightweight handle for the inspect WebSocket proxy, holding only
/// the cloneable parts of CdpClient needed for bidirectional message forwarding.
pub struct InspectProxyHandle {
    ws_tx: WsTx,
    raw_tx: broadcast::Sender<RawCdpMessage>,
}

impl InspectProxyHandle {
    pub async fn send_raw(&self, json: String) -> Result<(), String> {
        let mut ws_tx = self.ws_tx.lock().await;
        ws_tx
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send raw CDP message: {}", e))
    }

    pub fn subscribe_raw(&self) -> broadcast::Receiver<RawCdpMessage> {
        self.raw_tx.subscribe()
    }
}

/// Enable TCP SO_KEEPALIVE on the underlying socket of a WebSocket connection.
/// This is best-effort: failures are silently ignored since the WebSocket-level
/// Ping keepalive provides the primary connection liveness mechanism.
fn enable_tcp_keepalive(stream: &tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>) {
    let tcp_stream = match stream {
        tokio_tungstenite::MaybeTlsStream::Plain(s) => s,
        tokio_tungstenite::MaybeTlsStream::Rustls(s) => s.get_ref().0,
        _ => return,
    };

    // SockRef borrows the fd without taking ownership.
    let sock = socket2::SockRef::from(tcp_stream);
    let keepalive = socket2::TcpKeepalive::new().with_time(std::time::Duration::from_secs(30));

    // with_interval sets TCP_KEEPINTVL — the time between probes after the
    // first keepalive probe goes unanswered. Available on most platforms
    // (Linux, macOS, Windows, FreeBSD, etc.) but not OpenBSD or Haiku.
    #[cfg(not(any(target_os = "openbsd", target_os = "haiku")))]
    let keepalive = keepalive.with_interval(std::time::Duration::from_secs(10));

    let _ = sock.set_tcp_keepalive(&keepalive);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    use super::{CdpClient, CdpCommandError};

    #[tokio::test]
    async fn short_deadline_times_out_without_blocking_the_next_command() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

            let first = websocket.next().await.unwrap().unwrap();
            let first: serde_json::Value = serde_json::from_str(first.to_text().unwrap()).unwrap();
            assert_eq!(first["method"], "Runtime.evaluate");

            let second = websocket.next().await.unwrap().unwrap();
            let second: serde_json::Value =
                serde_json::from_str(second.to_text().unwrap()).unwrap();
            websocket
                .send(Message::Text(
                    json!({
                        "id": second["id"],
                        "result": { "ready": true }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });

        let client = CdpClient::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        let error = client
            .send_command_with_timeout(
                "Runtime.evaluate",
                Some(json!({ "expression": "new Promise(() => {})" })),
                Some("session-1"),
                Duration::from_millis(25),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            CdpCommandError::Timeout {
                ref method,
                timeout
            } if method == "Runtime.evaluate" && timeout == Duration::from_millis(25)
        ));

        let result = client
            .send_command_with_timeout(
                "Runtime.evaluate",
                Some(json!({ "expression": "true" })),
                Some("session-1"),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        assert_eq!(result, json!({ "ready": true }));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn externally_cancelled_command_removes_its_pending_registration() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (received_tx, received_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let command = websocket.next().await.unwrap().unwrap();
            let command: serde_json::Value =
                serde_json::from_str(command.to_text().unwrap()).unwrap();
            received_tx.send(command["id"].as_u64().unwrap()).unwrap();
            futures_util::future::pending::<()>().await;
        });

        let client = Arc::new(
            CdpClient::connect(&format!("ws://{address}"))
                .await
                .unwrap(),
        );
        let command_client = client.clone();
        let command = tokio::spawn(async move {
            command_client
                .send_command_with_timeout(
                    "Runtime.evaluate",
                    Some(json!({ "expression": "new Promise(() => {})" })),
                    Some("session-1"),
                    Duration::from_secs(60),
                )
                .await
        });

        received_rx.await.unwrap();
        assert_eq!(client.pending_command_count().await, 1);
        command.abort();
        let _ = command.await;

        tokio::task::yield_now().await;
        assert_eq!(client.pending_command_count().await, 0);

        server.abort();
    }
}
