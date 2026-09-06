//! Backend task ownership for a receive-only Guacamole primary connection.

use super::guacamole_primary_binding::PrimaryGuard;
use super::guacamole_primary_protocol::Protocol;
use futures_util::{SinkExt, StreamExt};
use std::future::Future;
#[cfg(test)]
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PrimaryStatus {
    Starting,
    Ready(String),
    Closed(&'static str),
}

pub(super) struct PrimaryTask {
    pub occurrence_id: String,
    status: watch::Receiver<PrimaryStatus>,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

struct TerminalObserver<F: FnOnce(&str, &'static str, u64)> {
    occurrence_id: String,
    started: Instant,
    callback: Option<F>,
}

impl<F: FnOnce(&str, &'static str, u64)> TerminalObserver<F> {
    fn record(&mut self, code: &'static str) {
        if let Some(callback) = self.callback.take() {
            callback(
                &self.occurrence_id,
                code,
                self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            );
        }
    }
}

impl<F: FnOnce(&str, &'static str, u64)> Drop for TerminalObserver<F> {
    fn drop(&mut self) {
        // Cancellation and unwinding must not silently bypass terminal custody.
        // This cannot run after SIGKILL or process abort.
        self.record("guacamole_primary_task_cancelled");
    }
}

impl PrimaryTask {
    pub fn spawn<S>(socket: WebSocketStream<S>, is_current: PrimaryGuard) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Self::connect(async move { Ok(socket) }, is_current)
    }

    pub fn connect<S, F>(connection: F, is_current: PrimaryGuard) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        F: Future<Output = Result<WebSocketStream<S>, &'static str>> + Send + 'static,
    {
        Self::connect_observed(connection, is_current, |_, _, _| {})
    }

    /// Preserve the terminal cause before publishing Closed to waiting callers.
    /// The observer receives only a static code and elapsed time, never payloads.
    pub fn connect_observed<S, F>(
        connection: F,
        is_current: PrimaryGuard,
        on_closed: impl FnOnce(&str, &'static str, u64) + Send + 'static,
    ) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        F: Future<Output = Result<WebSocketStream<S>, &'static str>> + Send + 'static,
    {
        let (status_tx, status) = watch::channel(PrimaryStatus::Starting);
        let (shutdown, shutdown_rx) = watch::channel(false);
        let occurrence_id = uuid::Uuid::new_v4().to_string();
        let mut observer = TerminalObserver {
            occurrence_id: occurrence_id.clone(),
            started: Instant::now(),
            callback: Some(on_closed),
        };
        let task = tokio::spawn(async move {
            let outcome = connect_and_run(connection, is_current, &status_tx, shutdown_rx).await;
            observer.record(outcome);
            let _ = status_tx.send(PrimaryStatus::Closed(outcome));
        });
        Self {
            occurrence_id,
            status,
            shutdown,
            task,
        }
    }

    pub fn status(&self) -> PrimaryStatus {
        let status = self.status.borrow().clone();
        if self.task.is_finished() && !matches!(status, PrimaryStatus::Closed(_)) {
            PrimaryStatus::Closed("guacamole_primary_task_closed")
        } else {
            status
        }
    }

    pub async fn ready(&self) -> Result<(), &'static str> {
        let mut receiver = self.status.clone();
        tokio::time::timeout(Duration::from_secs(16), async {
            loop {
                match self.status() {
                    PrimaryStatus::Ready(_) => return Ok(()),
                    PrimaryStatus::Closed(code) => return Err(code),
                    PrimaryStatus::Starting => {}
                }
                receiver
                    .changed()
                    .await
                    .map_err(|_| "guacamole_primary_task_closed")?;
            }
        })
        .await
        .map_err(|_| "guacamole_primary_start_timeout")?
    }

    pub async fn close(&mut self) {
        self.stop();
        if tokio::time::timeout(Duration::from_secs(2), &mut self.task)
            .await
            .is_err()
        {
            self.task.abort();
            let _ = (&mut self.task).await;
        }
    }

    pub fn stop(&self) {
        let _ = self.shutdown.send(true);
    }
}

impl Drop for PrimaryTask {
    fn drop(&mut self) {
        // Registry/backend teardown cannot leave a detached provider owner.
        let _ = self.shutdown.send(true);
        self.task.abort();
    }
}

async fn connect_and_run<S, F>(
    connection: F,
    is_current: PrimaryGuard,
    status: &watch::Sender<PrimaryStatus>,
    mut shutdown: watch::Receiver<bool>,
) -> &'static str
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: Future<Output = Result<WebSocketStream<S>, &'static str>>,
{
    if let Err(code) = is_current() {
        return code;
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut fence = tokio::time::interval(Duration::from_millis(250));
    tokio::pin!(connection);
    let socket = loop {
        tokio::select! {
            _ = shutdown.changed() => return "guacamole_primary_stopped",
            _ = fence.tick() => {
                if let Err(code) = is_current() { return code; }
                if Instant::now() >= deadline { return "guacamole_primary_start_timeout"; }
            }
            result = &mut connection => match result {
                Ok(socket) => break socket,
                Err(code) => return code,
            }
        }
    };
    run(socket, is_current, status, shutdown, deadline).await
}

async fn run<S>(
    mut socket: WebSocketStream<S>,
    is_current: PrimaryGuard,
    status: &watch::Sender<PrimaryStatus>,
    mut shutdown: watch::Receiver<bool>,
    start_deadline: Instant,
) -> &'static str
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut protocol = Protocol::default();
    let mut ready = false;
    let mut fence = tokio::time::interval(Duration::from_millis(250));
    let mut keepalive = tokio::time::interval(Duration::from_secs(5));
    let outcome = loop {
        tokio::select! {
            _ = shutdown.changed() => break "guacamole_primary_stopped",
            _ = fence.tick() => {
                if let Err(code) = is_current() { break code; }
                if !ready && Instant::now() >= start_deadline { break "guacamole_primary_start_timeout"; }
            }
            _ = keepalive.tick() => {
                if let Err(code) = is_current() { break code; }
                if send(&mut socket, Message::Text("3.nop;".into())).await.is_err() {
                    break "guacamole_primary_transport_closed";
                }
            }
            message = socket.next() => {
                if let Err(code) = is_current() { break code; }
                let observation = match message {
                    Some(Ok(Message::Text(text))) => match protocol.receive(&text) {
                        Ok(value) => value,
                        Err(code) => break code,
                    },
                    Some(Ok(Message::Ping(payload))) => {
                        if send(&mut socket, Message::Pong(payload)).await.is_err() {
                            break "guacamole_primary_transport_closed";
                        }
                        continue;
                    }
                    Some(Ok(Message::Pong(_))) => continue,
                    _ => break "guacamole_primary_transport_closed",
                };
                for reply in observation.replies {
                    if let Err(code) = is_current() { return code; }
                    if send(&mut socket, Message::Text(reply)).await.is_err() {
                        return "guacamole_primary_transport_closed";
                    }
                }
                if observation.frame_complete && !ready {
                    if let Some(id) = observation.primary_id {
                        ready = true;
                        let _ = status.send(PrimaryStatus::Ready(id));
                    }
                }
            }
        }
    };
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.close(None)).await;
    outcome
}

async fn send<S>(socket: &mut WebSocketStream<S>, message: Message) -> Result<(), ()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    tokio::time::timeout(Duration::from_secs(1), socket.send(message))
        .await
        .map_err(|_| ())?
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio_tungstenite::tungstenite::protocol::Role;

    #[tokio::test]
    async fn viewer_waiter_departure_preserves_owner_until_binding_invalidation() {
        let (client, server) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server, Role::Server, None).await;
        let current = Arc::new(AtomicBool::new(true));
        let guard = current.clone();
        let (connected_tx, connected_rx) = tokio::sync::oneshot::channel();
        let (terminal_tx, terminal_rx) = tokio::sync::oneshot::channel();
        let mut owner = PrimaryTask::connect_observed(
            async move { connected_rx.await.map_err(|_| "fixture_connection_closed") },
            Arc::new(move || {
                if guard.load(Ordering::SeqCst) {
                    Ok(())
                } else {
                    Err("guacamole_primary_state_lock_timeout")
                }
            }),
            move |occurrence_id, code, elapsed_ms| {
                terminal_tx
                    .send((occurrence_id.to_owned(), code, elapsed_ms))
                    .unwrap();
            },
        );
        // Cancel an actual pending viewer wait before the first provider frame.
        // Connection ownership must remain with the backend task.
        {
            let waiter = owner.ready();
            tokio::pin!(waiter);
            assert!(matches!(
                futures_util::poll!(&mut waiter),
                std::task::Poll::Pending
            ));
        }
        connected_tx.send(client).unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000001;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        owner.ready().await.unwrap();
        // A second request can observe the same owner; returning/dropping that
        // request's future must not close the connection owned by the registry.
        owner.ready().await.unwrap();
        assert_eq!(
            owner.status(),
            PrimaryStatus::Ready("00000000-0000-4000-8000-000000000001".into())
        );
        // A quiet display must still receive periodic client keepalives after
        // every startup waiter has returned. A single initial nop is insufficient.
        tokio::time::timeout(Duration::from_secs(7), async {
            let mut keepalives = 0;
            while keepalives < 2 {
                match server.next().await.unwrap().unwrap() {
                    Message::Text(text) if text == "3.nop;" => keepalives += 1,
                    Message::Text(text) => assert_eq!(text, "4.sync,1.1;"),
                    other => panic!("unexpected primary message: {other:?}"),
                }
            }
        })
        .await
        .expect("primary stopped sending keepalives after startup");
        current.store(false, Ordering::SeqCst);
        tokio::time::timeout(Duration::from_secs(2), async {
            while !matches!(owner.status(), PrimaryStatus::Closed(_)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            owner.status(),
            PrimaryStatus::Closed("guacamole_primary_state_lock_timeout")
        );
        // Closed is observable only after the terminal sink has accepted custody.
        let (occurrence_id, code, elapsed_ms) = terminal_rx.await.unwrap();
        assert_eq!(occurrence_id, owner.occurrence_id);
        assert_eq!(code, "guacamole_primary_state_lock_timeout");
        assert!(elapsed_ms >= 5_000);
        owner.close().await;
    }

    #[tokio::test]
    async fn aborted_owner_cannot_report_stale_readiness() {
        let (client, server) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server, Role::Server, None).await;
        let (terminal_tx, terminal_rx) = tokio::sync::oneshot::channel();
        let owner = PrimaryTask::connect_observed(
            async move { Ok(client) },
            Arc::new(|| Ok(())),
            move |_, code, _| {
                terminal_tx.send(code).unwrap();
            },
        );
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000001;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        owner.ready().await.unwrap();
        owner.task.abort();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !owner.task.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(owner.ready().await, Err("guacamole_primary_task_closed"));
        assert_eq!(
            terminal_rx.await.unwrap(),
            "guacamole_primary_task_cancelled"
        );
    }

    #[tokio::test]
    async fn an_invalid_binding_cannot_acknowledge_provider_frames() {
        let (client, server) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server, Role::Server, None).await;
        let mut owner = PrimaryTask::spawn(
            client,
            Arc::new(|| Err("guacamole_primary_binding_changed")),
        );
        server
            .send(Message::Text("4.sync,1.1;".into()))
            .await
            .unwrap();
        assert_eq!(
            owner.ready().await,
            Err("guacamole_primary_binding_changed")
        );
        assert!(matches!(
            server.next().await,
            None | Some(Err(_)) | Some(Ok(Message::Close(_)))
        ));
        owner.close().await;
    }
}
