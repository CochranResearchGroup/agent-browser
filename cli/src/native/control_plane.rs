use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};

use super::actions::{execute_command, DaemonState};
use super::service_health::{reconcile_persisted_service_state, reconcile_service_state};
use super::service_model::{ControlPlaneSnapshot, ServiceState};
use super::service_store::{JsonServiceStateStore, ServiceStateStore};

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

    pub fn start_with_service_reconcile_interval(
        state: DaemonState,
        service_reconcile_interval_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        Self::start_with_options(state, DEFAULT_QUEUE_CAPACITY, service_reconcile_interval_ms)
    }

    fn start_with_capacity(state: DaemonState, capacity: usize) -> ControlPlaneHandle {
        Self::start_with_options(state, capacity, None)
    }

    fn start_with_options(
        state: DaemonState,
        capacity: usize,
        service_reconcile_interval_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        let (tx, rx) = mpsc::channel(capacity);
        let status = Arc::new(ControlPlaneStatus::new());
        tokio::spawn(run_worker(
            state,
            rx,
            status.clone(),
            service_reconcile_interval_ms,
        ));
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

    pub async fn service_status_response(&self, id: &str, service_state: Value) -> Value {
        let mut service_state = serde_json::from_value::<ServiceState>(service_state)
            .unwrap_or_else(|_| ServiceState::default());
        service_state.control_plane = Some(self.status_snapshot());
        reconcile_service_state(&mut service_state).await;
        persist_service_state_snapshot(&service_state);

        json!({
            "id": id,
            "success": true,
            "data": {
                "control_plane": self.status_payload(),
                "service_state": service_state,
            },
        })
    }

    fn status_snapshot(&self) -> ControlPlaneSnapshot {
        ControlPlaneSnapshot {
            worker_state: self.status.worker_state().as_str().to_string(),
            browser_health: self.status.browser_health().as_str().to_string(),
            queue_depth: self.status.queue_depth(),
            queue_capacity: self.tx.max_capacity(),
            updated_at: Some(current_timestamp()),
        }
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

fn persist_service_state_snapshot(state: &ServiceState) {
    let Ok(path) = JsonServiceStateStore::default_path() else {
        return;
    };
    let store = JsonServiceStateStore::new(path);
    let _ = store.save(state);
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
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
    service_reconcile_interval_ms: Option<u64>,
) {
    let mut drain_interval = tokio::time::interval(Duration::from_millis(100));
    drain_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut service_reconcile_interval = service_reconcile_interval_ms.map(|ms| {
        let mut interval = tokio::time::interval(Duration::from_millis(ms));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval
    });
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
            _ = async {
                match service_reconcile_interval.as_mut() {
                    Some(interval) => interval.tick().await,
                    None => std::future::pending::<tokio::time::Instant>().await,
                }
            }, if service_reconcile_interval.is_some() => {
                let _ = reconcile_persisted_service_state().await;
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
    use crate::test_utils::EnvGuard;

    fn temp_home(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agent-browser-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

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
    async fn service_status_response_combines_worker_and_service_state() {
        let home = temp_home("control-plane-service-status");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let response = handle
            .service_status_response(
                "test-service-status",
                json!({
                    "sitePolicies": {
                        "google": {
                            "id": "google",
                            "originPattern": "https://accounts.google.com"
                        }
                    }
                }),
            )
            .await;

        assert_eq!(
            response.get("id").and_then(|v| v.as_str()),
            Some("test-service-status")
        );
        assert_eq!(
            response
                .pointer("/data/control_plane/worker_state")
                .and_then(|v| v.as_str())
                .is_some(),
            true
        );
        assert_eq!(
            response
                .pointer("/data/control_plane/queue_capacity")
                .and_then(|v| v.as_u64()),
            Some(DEFAULT_QUEUE_CAPACITY as u64)
        );
        assert_eq!(
            response
                .pointer("/data/service_state/sitePolicies/google/id")
                .and_then(|v| v.as_str()),
            Some("google")
        );
        assert_eq!(
            response
                .pointer("/data/service_state/controlPlane/queueCapacity")
                .and_then(|v| v.as_u64()),
            Some(DEFAULT_QUEUE_CAPACITY as u64)
        );
        assert_eq!(
            response
                .pointer("/data/service_state/reconciliation/browserCount")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
        assert!(response
            .pointer("/data/service_state/reconciliation/lastReconciledAt")
            .and_then(|v| v.as_str())
            .is_some());

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        let persisted = store.load().unwrap();
        assert_eq!(
            persisted
                .control_plane
                .as_ref()
                .map(|snapshot| snapshot.queue_capacity),
            Some(DEFAULT_QUEUE_CAPACITY)
        );
        assert_eq!(
            persisted
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.browser_count),
            Some(0)
        );

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn background_service_reconcile_updates_persisted_browser_health() {
        let home = temp_home("control-plane-reconcile-loop");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                browsers: std::collections::BTreeMap::from([(
                    "browser-1".to_string(),
                    crate::native::service_model::BrowserProcess {
                        id: "browser-1".to_string(),
                        host: crate::native::service_model::BrowserHost::AttachedExisting,
                        health: crate::native::service_model::BrowserHealth::Ready,
                        cdp_endpoint: Some(
                            "ws://127.0.0.1:9/devtools/browser/unreachable".to_string(),
                        ),
                        active_session_ids: vec!["reconcile-loop".to_string()],
                        ..crate::native::service_model::BrowserProcess::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let handle =
            ControlPlaneWorker::start_with_service_reconcile_interval(DaemonState::new(), Some(25));
        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.shutdown().await;

        let persisted = store.load().unwrap();
        assert_eq!(
            persisted.browsers["browser-1"].health,
            crate::native::service_model::BrowserHealth::Unreachable
        );
        assert_eq!(
            persisted
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.browser_count),
            Some(1)
        );
        assert!(persisted
            .reconciliation
            .as_ref()
            .and_then(|snapshot| snapshot.last_reconciled_at.as_deref())
            .is_some());

        let _ = std::fs::remove_dir_all(&home);
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
