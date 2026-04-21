use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};

use super::actions::{execute_command, DaemonState};

const DEFAULT_QUEUE_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct ControlPlaneHandle {
    tx: mpsc::Sender<WorkerMessage>,
    status: Arc<ControlPlaneStatus>,
}

pub struct ControlPlaneStatus {
    state: AtomicUsize,
    browser_health: AtomicUsize,
    queue_depth: AtomicUsize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Starting,
    Ready,
    Busy,
    Draining,
    Closing,
    Stopped,
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserHealth {
    NotStarted,
    Launching,
    Ready,
    Unreachable,
    ProcessExited,
    CdpDisconnected,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlPriority {
    Normal,
    Lifecycle,
}

pub struct ControlRequest {
    pub id: String,
    pub action: String,
    pub command: Value,
    pub priority: ControlPriority,
    pub submitted_at: Instant,
    pub response_tx: oneshot::Sender<Value>,
}

enum WorkerMessage {
    Request(ControlRequest),
    Shutdown(oneshot::Sender<()>),
}

pub struct ControlPlaneWorker;

impl ControlPlaneWorker {
    pub fn start(state: DaemonState) -> ControlPlaneHandle {
        Self::start_with_capacity(state, DEFAULT_QUEUE_CAPACITY)
    }

    fn start_with_capacity(state: DaemonState, capacity: usize) -> ControlPlaneHandle {
        let (tx, rx) = mpsc::channel(capacity);
        let status = Arc::new(ControlPlaneStatus::new());
        tokio::spawn(run_worker(state, rx, status.clone()));
        ControlPlaneHandle { tx, status }
    }
}

impl ControlPlaneHandle {
    pub fn status_response(&self, id: &str) -> Value {
        json!({
            "id": id,
            "success": true,
            "data": self.status_payload(),
        })
    }

    fn status_payload(&self) -> Value {
        json!({
            "worker_state": self.status.worker_state().as_str(),
            "browser_health": self.status.browser_health().as_str(),
            "queue_depth": self.status.queue_depth(),
            "queue_capacity": self.tx.max_capacity(),
        })
    }

    pub async fn submit(&self, command: Value) -> Value {
        let id = command
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let action = command
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let (response_tx, response_rx) = oneshot::channel();
        let request = ControlRequest {
            id: id.clone(),
            action,
            command,
            priority: ControlPriority::Normal,
            submitted_at: Instant::now(),
            response_tx,
        };

        self.status.queue_depth.fetch_add(1, Ordering::Relaxed);
        match self.tx.try_send(WorkerMessage::Request(request)) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                return json!({
                    "id": id,
                    "success": false,
                    "error": "Control queue is full",
                    "data": {
                        "queue_depth": self.status.queue_depth(),
                        "worker_state": self.status.worker_state().as_str(),
                        "browser_health": self.status.browser_health().as_str(),
                    },
                });
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                return json!({
                    "id": id,
                    "success": false,
                    "error": "Control plane worker is stopped",
                    "data": {
                        "worker_state": self.status.worker_state().as_str(),
                        "browser_health": self.status.browser_health().as_str(),
                    },
                });
            }
        }

        match response_rx.await {
            Ok(response) => response,
            Err(_) => json!({
                "id": id,
                "success": false,
                "error": "Control plane worker stopped before responding",
                "data": {
                    "worker_state": self.status.worker_state().as_str(),
                    "browser_health": self.status.browser_health().as_str(),
                },
            }),
        }
    }

    pub async fn shutdown(&self) {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(WorkerMessage::Shutdown(tx)).await.is_ok() {
            let _ = rx.await;
        }
    }

    #[cfg(test)]
    fn queue_depth(&self) -> usize {
        self.status.queue_depth()
    }

    #[cfg(test)]
    fn browser_health(&self) -> BrowserHealth {
        self.status.browser_health()
    }
}

impl ControlPlaneStatus {
    fn new() -> Self {
        Self {
            state: AtomicUsize::new(WorkerState::Starting as usize),
            browser_health: AtomicUsize::new(BrowserHealth::NotStarted as usize),
            queue_depth: AtomicUsize::new(0),
        }
    }

    fn set_state(&self, state: WorkerState) {
        self.state.store(state as usize, Ordering::Relaxed);
    }

    fn worker_state(&self) -> WorkerState {
        match self.state.load(Ordering::Relaxed) {
            0 => WorkerState::Starting,
            1 => WorkerState::Ready,
            2 => WorkerState::Busy,
            3 => WorkerState::Draining,
            4 => WorkerState::Closing,
            5 => WorkerState::Stopped,
            _ => WorkerState::Faulted,
        }
    }

    fn set_browser_health(&self, health: BrowserHealth) {
        self.browser_health
            .store(health as usize, Ordering::Relaxed);
    }

    fn browser_health(&self) -> BrowserHealth {
        match self.browser_health.load(Ordering::Relaxed) {
            0 => BrowserHealth::NotStarted,
            1 => BrowserHealth::Launching,
            2 => BrowserHealth::Ready,
            3 => BrowserHealth::Unreachable,
            4 => BrowserHealth::ProcessExited,
            5 => BrowserHealth::CdpDisconnected,
            _ => BrowserHealth::Closing,
        }
    }

    fn queue_depth(&self) -> usize {
        self.queue_depth.load(Ordering::Relaxed)
    }
}

impl WorkerState {
    fn as_str(self) -> &'static str {
        match self {
            WorkerState::Starting => "Starting",
            WorkerState::Ready => "Ready",
            WorkerState::Busy => "Busy",
            WorkerState::Draining => "Draining",
            WorkerState::Closing => "Closing",
            WorkerState::Stopped => "Stopped",
            WorkerState::Faulted => "Faulted",
        }
    }
}

impl BrowserHealth {
    fn as_str(self) -> &'static str {
        match self {
            BrowserHealth::NotStarted => "NotStarted",
            BrowserHealth::Launching => "Launching",
            BrowserHealth::Ready => "Ready",
            BrowserHealth::Unreachable => "Unreachable",
            BrowserHealth::ProcessExited => "ProcessExited",
            BrowserHealth::CdpDisconnected => "CdpDisconnected",
            BrowserHealth::Closing => "Closing",
        }
    }
}

async fn run_worker(
    mut state: DaemonState,
    mut rx: mpsc::Receiver<WorkerMessage>,
    status: Arc<ControlPlaneStatus>,
) {
    let mut drain_interval = tokio::time::interval(Duration::from_millis(100));
    drain_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    status.set_state(WorkerState::Ready);

    loop {
        tokio::select! {
            maybe_message = rx.recv() => {
                let Some(message) = maybe_message else {
                    break;
                };

                match message {
                    WorkerMessage::Request(request) => {
                        status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                        status.set_state(WorkerState::Busy);
                        refresh_browser_health(&mut state, &status).await;
                        let response = execute_command(&request.command, &mut state).await;
                        refresh_browser_health(&mut state, &status).await;
                        let _ = request.response_tx.send(response);
                        status.set_state(WorkerState::Ready);
                    }
                    WorkerMessage::Shutdown(done_tx) => {
                        status.set_state(WorkerState::Closing);
                        close_browser(&mut state).await;
                        let _ = done_tx.send(());
                        break;
                    }
                }
            }
            _ = drain_interval.tick() => {
                if state.browser.is_some() {
                    status.set_state(WorkerState::Draining);
                    let browser_exited = state
                        .browser
                        .as_mut()
                        .is_some_and(|mgr| mgr.has_process_exited());
                    if browser_exited {
                        status.set_browser_health(BrowserHealth::ProcessExited);
                        cleanup_exited_browser(&mut state).await;
                    } else {
                        state.drain_cdp_events_background().await;
                        status.set_browser_health(BrowserHealth::Ready);
                    }
                    status.set_state(WorkerState::Ready);
                }
            }
        }
    }

    status.set_state(WorkerState::Stopped);
}

async fn close_browser(state: &mut DaemonState) {
    if let Some(ref mut mgr) = state.browser {
        let _ = mgr.close().await;
    }
}

async fn cleanup_exited_browser(state: &mut DaemonState) {
    if let Some(ref mut mgr) = state.browser {
        let _ = mgr.close().await;
    }
    state.browser = None;
    state.screencasting = false;
    state.update_stream_client().await;
}

async fn refresh_browser_health(state: &mut DaemonState, status: &ControlPlaneStatus) {
    let Some(ref mut mgr) = state.browser else {
        status.set_browser_health(BrowserHealth::NotStarted);
        return;
    };

    if mgr.has_process_exited() {
        status.set_browser_health(BrowserHealth::ProcessExited);
        cleanup_exited_browser(state).await;
        return;
    }

    if mgr.is_connection_alive().await {
        status.set_browser_health(BrowserHealth::Ready);
    } else {
        status.set_browser_health(BrowserHealth::CdpDisconnected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn submit_returns_command_response() {
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let response = handle
            .submit(json!({
                "id": "test-1",
                "action": "state_list",
            }))
            .await;

        assert_eq!(response.get("id").and_then(|v| v.as_str()), Some("test-1"));
        assert_eq!(
            response.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(handle.queue_depth(), 0);
        assert_eq!(handle.browser_health(), BrowserHealth::NotStarted);
        handle.shutdown().await;
    }

    #[tokio::test]
    async fn status_response_reports_worker_state() {
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let _ = handle
            .submit(json!({
                "id": "test-status-prime",
                "action": "state_list",
            }))
            .await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        let response = handle.status_response("test-status");

        assert_eq!(
            response.get("id").and_then(|v| v.as_str()),
            Some("test-status")
        );
        assert_eq!(
            response.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            response
                .pointer("/data/worker_state")
                .and_then(|v| v.as_str()),
            Some("Ready")
        );
        assert_eq!(
            response
                .pointer("/data/browser_health")
                .and_then(|v| v.as_str()),
            Some("NotStarted")
        );
        assert_eq!(
            response
                .pointer("/data/queue_depth")
                .and_then(|v| v.as_u64()),
            Some(0)
        );

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn parallel_submits_leave_queue_depth_at_zero() {
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let mut tasks = Vec::new();

        for idx in 0..64 {
            let handle = handle.clone();
            tasks.push(tokio::spawn(async move {
                handle
                    .submit(json!({
                        "id": format!("test-parallel-{idx}"),
                        "action": "state_list",
                    }))
                    .await
            }));
        }

        for task in tasks {
            let response = task.await.expect("submit task should complete");
            assert_eq!(
                response.get("success").and_then(|v| v.as_bool()),
                Some(true)
            );
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(handle.queue_depth(), 0);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn full_queue_returns_structured_error() {
        let handle = ControlPlaneWorker::start_with_capacity(DaemonState::new(), 1);
        let _permit = handle
            .tx
            .reserve()
            .await
            .expect("queue should accept reserve");

        let response = handle
            .submit(json!({
                "id": "test-full",
                "action": "state_list",
            }))
            .await;

        assert_eq!(
            response.get("id").and_then(|v| v.as_str()),
            Some("test-full")
        );
        assert_eq!(
            response.get("success").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            response.get("error").and_then(|v| v.as_str()),
            Some("Control queue is full")
        );
        assert_eq!(
            response
                .pointer("/data/browser_health")
                .and_then(|v| v.as_str()),
            Some("NotStarted")
        );

        drop(_permit);
        handle.shutdown().await;
    }
}
