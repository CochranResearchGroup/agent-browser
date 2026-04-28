use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use super::actions::{execute_command, DaemonState};
use super::cancellation::CancellationToken as RunningJobCancel;
use super::service_health::{
    apply_browser_health_observation, browser_health_observation_details,
    reconcile_persisted_service_state, reconcile_service_state,
    record_browser_health_changed_event,
};
use super::service_model::{
    BrowserHealth as ServiceBrowserHealth, BrowserHost as ServiceBrowserHost, BrowserProcess,
    ControlPlaneSnapshot, JobPriority, JobState, JobTarget, ServiceActor, ServiceJob, ServiceState,
};
use super::service_store::{
    mutate_default_service_state, JsonServiceStateStore, ServiceStateStore,
};

const DEFAULT_QUEUE_CAPACITY: usize = 256;
const MAX_SERVICE_JOBS: usize = 200;

#[derive(Clone)]
pub struct ControlPlaneHandle {
    tx: mpsc::Sender<WorkerMessage>,
    status: Arc<ControlPlaneStatus>,
    service_job_timeout_ms: Option<u64>,
    running_cancellations: Arc<Mutex<HashMap<String, RunningJobCancel>>>,
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
    pub job_id: String,
    pub action: String,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub command: Value,
    pub priority: ControlPriority,
    /// Optional worker-bound execution timeout. The worker records timed-out
    /// requests as service jobs with `timed_out` state.
    pub timeout_ms: Option<u64>,
    pub cancellation: RunningJobCancel,
    pub submitted_at_wall: String,
    pub response_tx: oneshot::Sender<Value>,
}

enum WorkerMessage {
    Request(Box<ControlRequest>),
    Shutdown(oneshot::Sender<()>),
}

pub struct ControlPlaneWorker;

impl ControlPlaneWorker {
    pub fn start(state: DaemonState) -> ControlPlaneHandle {
        Self::start_with_capacity_and_options(state, DEFAULT_QUEUE_CAPACITY, None, None)
    }

    pub fn start_with_service_reconcile_interval(
        state: DaemonState,
        service_reconcile_interval_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        Self::start_with_options(state, service_reconcile_interval_ms, None)
    }

    pub fn start_with_options(
        state: DaemonState,
        service_reconcile_interval_ms: Option<u64>,
        service_job_timeout_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        Self::start_with_capacity_and_options(
            state,
            DEFAULT_QUEUE_CAPACITY,
            service_reconcile_interval_ms,
            service_job_timeout_ms,
        )
    }

    fn start_with_capacity(state: DaemonState, capacity: usize) -> ControlPlaneHandle {
        Self::start_with_capacity_and_options(state, capacity, None, None)
    }

    fn start_with_capacity_and_options(
        state: DaemonState,
        capacity: usize,
        service_reconcile_interval_ms: Option<u64>,
        service_job_timeout_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        let (tx, rx) = mpsc::channel(capacity);
        let status = Arc::new(ControlPlaneStatus::new());
        let running_cancellations = Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn(run_worker(
            state,
            rx,
            status.clone(),
            service_reconcile_interval_ms,
            service_job_timeout_ms,
            running_cancellations.clone(),
        ));
        ControlPlaneHandle {
            tx,
            status,
            service_job_timeout_ms,
            running_cancellations,
        }
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
            service_job_timeout_ms: self.service_job_timeout_ms,
            updated_at: Some(current_timestamp()),
        }
    }

    fn status_payload(&self) -> Value {
        json!({
            "worker_state": self.status.worker_state().as_str(),
            "browser_health": self.status.browser_health().as_str(),
            "queue_depth": self.status.queue_depth(),
            "queue_capacity": self.tx.max_capacity(),
            "service_job_timeout_ms": self.service_job_timeout_ms,
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
        let job_id = if id.is_empty() {
            format!("job-{}", uuid::Uuid::new_v4())
        } else {
            id.clone()
        };
        let timeout_ms = command
            .get("jobTimeoutMs")
            .and_then(|v| v.as_u64())
            .filter(|ms| *ms > 0)
            .or(self.service_job_timeout_ms);
        let service_name = optional_command_string(&command, "serviceName");
        let agent_name = optional_command_string(&command, "agentName");
        let task_name = optional_command_string(&command, "taskName");
        let request = ControlRequest {
            id: id.clone(),
            job_id,
            action: action.clone(),
            service_name,
            agent_name,
            task_name,
            command,
            priority: ControlPriority::Normal,
            timeout_ms,
            cancellation: RunningJobCancel::new(),
            submitted_at_wall: current_timestamp(),
            response_tx,
        };

        self.status.queue_depth.fetch_add(1, Ordering::Relaxed);
        persist_service_job_queued(&request);
        match self.tx.try_send(WorkerMessage::Request(Box::new(request))) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(WorkerMessage::Request(request))) => {
                let request = *request;
                self.status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                persist_service_job_failed_to_enqueue(&request, "Control queue is full");
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
            Err(mpsc::error::TrySendError::Closed(WorkerMessage::Request(request))) => {
                let request = *request;
                self.status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                persist_service_job_failed_to_enqueue(&request, "Control plane worker is stopped");
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
            Err(mpsc::error::TrySendError::Full(WorkerMessage::Shutdown(_)))
            | Err(mpsc::error::TrySendError::Closed(WorkerMessage::Shutdown(_))) => {
                self.status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                return json!({
                    "id": id,
                    "success": false,
                    "error": "Control plane worker rejected an internal shutdown message",
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

    pub fn cancel_job_response(&self, id: &str, job_id: &str, reason: Option<&str>) -> Value {
        if let Some(cancel) = self
            .running_cancellations
            .lock()
            .ok()
            .and_then(|running| running.get(job_id).cloned())
        {
            cancel.cancel();
            return json!({
                "id": id,
                "success": true,
                "data": {
                    "cancelled": false,
                    "cancellationRequested": true,
                    "jobId": job_id,
                },
            });
        }

        match cancel_persisted_service_job(job_id, reason) {
            Ok(job) => json!({
                "id": id,
                "success": true,
                "data": {
                    "cancelled": true,
                    "job": job,
                },
            }),
            Err(err) => json!({
                "id": id,
                "success": false,
                "error": err,
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

fn service_browser_id(session_id: &str) -> String {
    format!("session:{}", session_id)
}

fn persist_process_exited_browser_health(state: &DaemonState) {
    let Ok(path) = JsonServiceStateStore::default_path() else {
        return;
    };
    let store = JsonServiceStateStore::new(path);
    let mut service_state = store.load().unwrap_or_else(|_| ServiceState::default());
    let id = service_browser_id(&state.session_id);
    let previous = service_state.browsers.get(&id).cloned();
    let host = previous
        .as_ref()
        .map(|browser| browser.host)
        .unwrap_or(ServiceBrowserHost::LocalHeaded);
    let (pid, cdp_endpoint) = state
        .browser
        .as_ref()
        .map(|mgr| (mgr.browser_pid(), Some(mgr.get_cdp_url().to_string())))
        .unwrap_or((None, None));
    let last_error = pid.map(|pid| format!("Browser process {} exited", pid));

    let mut browser = BrowserProcess {
        id: id.clone(),
        profile_id: previous
            .as_ref()
            .and_then(|browser| browser.profile_id.clone()),
        host,
        health: ServiceBrowserHealth::ProcessExited,
        pid,
        cdp_endpoint,
        view_streams: previous
            .as_ref()
            .map(|browser| browser.view_streams.clone())
            .unwrap_or_default(),
        active_session_ids: vec![state.session_id.clone()],
        last_error,
        last_health_observation: None,
    };
    let observation_details = browser_health_observation_details(&browser, None);
    apply_browser_health_observation(&mut browser, Some(&observation_details));
    record_browser_health_changed_event(&mut service_state, &id, previous.as_ref(), &browser);
    service_state.browsers.insert(id, browser);
    let _ = store.save(&service_state);
}

/// Persist a bounded audit record for each control-plane request.
fn persist_service_job(job: ServiceJob) {
    mutate_service_jobs(|state| {
        state.jobs.insert(job.id.clone(), job);
    });
}

fn mutate_service_jobs(mutator: impl FnOnce(&mut ServiceState)) {
    let _ = mutate_default_service_state(|state| {
        mutator(state);
        prune_service_jobs(state);
        Ok(())
    });
}

fn persist_service_job_queued(request: &ControlRequest) {
    persist_service_job(ServiceJob {
        id: service_job_id(request),
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::Queued,
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        timeout_ms: request.timeout_ms,
        ..ServiceJob::default()
    });
}

fn persist_service_job_running(request: &ControlRequest) {
    persist_service_job(ServiceJob {
        id: service_job_id(request),
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::Running,
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        started_at: Some(current_timestamp()),
        timeout_ms: request.timeout_ms,
        ..ServiceJob::default()
    });
}

fn persist_service_job_finished(request: &ControlRequest, response: &Value) {
    let job_id = service_job_id(request);
    let started_at = load_service_job(&job_id)
        .and_then(|job| job.started_at)
        .unwrap_or_else(current_timestamp);
    let success = response
        .get("success")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let error = response
        .get("error")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: if success {
            JobState::Succeeded
        } else {
            JobState::Failed
        },
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        started_at: Some(started_at),
        completed_at: Some(current_timestamp()),
        timeout_ms: request.timeout_ms,
        result: Some(json!({ "success": success })),
        error,
    });
}

fn persist_service_job_timed_out(request: &ControlRequest) {
    let job_id = service_job_id(request);
    let started_at = load_service_job(&job_id)
        .and_then(|job| job.started_at)
        .unwrap_or_else(current_timestamp);
    let timeout_ms = request.timeout_ms.unwrap_or_default();
    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::TimedOut,
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        started_at: Some(started_at),
        completed_at: Some(current_timestamp()),
        timeout_ms: request.timeout_ms,
        result: Some(json!({ "success": false, "timedOut": true, "timeoutMs": timeout_ms })),
        error: Some(format!("Service job timed out after {}ms", timeout_ms)),
    });
}

fn persist_service_job_cancelled(request: &ControlRequest, reason: &str) {
    let job_id = service_job_id(request);
    let started_at = load_service_job(&job_id)
        .and_then(|job| job.started_at)
        .unwrap_or_else(current_timestamp);
    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::Cancelled,
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        started_at: Some(started_at),
        completed_at: Some(current_timestamp()),
        timeout_ms: request.timeout_ms,
        result: Some(json!({ "success": false, "cancelled": true })),
        error: Some(reason.to_string()),
    });
}

fn persist_service_job_failed_to_enqueue(request: &ControlRequest, error: &str) {
    let job_id = service_job_id(request);
    let submitted_at = load_service_job(&job_id)
        .and_then(|job| job.submitted_at)
        .unwrap_or_else(current_timestamp);
    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::Failed,
        priority: JobPriority::Normal,
        submitted_at: Some(submitted_at),
        completed_at: Some(current_timestamp()),
        result: Some(json!({ "success": false })),
        error: Some(error.to_string()),
        ..ServiceJob::default()
    });
}

pub fn cancel_persisted_service_job(
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    let Ok(path) = JsonServiceStateStore::default_path() else {
        return Err("Unable to resolve service state path".to_string());
    };
    let store = JsonServiceStateStore::new(path);
    let mut state = store
        .load()
        .map_err(|err| format!("Unable to load service state: {}", err))?;
    let job = state
        .jobs
        .get_mut(job_id)
        .ok_or_else(|| format!("Service job not found: {}", job_id))?;

    match job.state {
        JobState::Queued => {
            job.state = JobState::Cancelled;
            job.completed_at = Some(current_timestamp());
            job.error = Some(
                reason
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("Cancelled by operator")
                    .to_string(),
            );
            job.result = Some(json!({ "success": false, "cancelled": true }));
            let job = job.clone();
            store.save(&state)?;
            Ok(job)
        }
        JobState::Cancelled => Ok(job.clone()),
        JobState::Running => Err(format!(
            "Service job {} is already running and cannot be cancelled safely",
            job_id
        )),
        JobState::Succeeded | JobState::Failed | JobState::TimedOut => Err(format!(
            "Service job {} is already terminal with state {}",
            job_id,
            job_state_name(job.state)
        )),
    }
}

fn service_job_cancelled(job_id: &str) -> bool {
    load_service_job(job_id).is_some_and(|job| job.state == JobState::Cancelled)
}

fn load_service_job(id: &str) -> Option<ServiceJob> {
    let path = JsonServiceStateStore::default_path().ok()?;
    let store = JsonServiceStateStore::new(path);
    store.load().ok()?.jobs.remove(id)
}

fn job_state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::TimedOut => "timed_out",
    }
}

fn service_job_id(request: &ControlRequest) -> String {
    request.job_id.clone()
}

fn service_job_priority(priority: ControlPriority) -> JobPriority {
    match priority {
        ControlPriority::Normal => JobPriority::Normal,
        ControlPriority::Lifecycle => JobPriority::Lifecycle,
    }
}

fn optional_command_string(command: &Value, name: &str) -> Option<String> {
    command
        .get(name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn prune_service_jobs(state: &mut ServiceState) {
    if state.jobs.len() <= MAX_SERVICE_JOBS {
        return;
    }
    let mut jobs = state
        .jobs
        .values()
        .map(|job| (job.submitted_at.clone().unwrap_or_default(), job.id.clone()))
        .collect::<Vec<_>>();
    jobs.sort();
    let excess = state.jobs.len() - MAX_SERVICE_JOBS;
    for (_, id) in jobs.into_iter().take(excess) {
        state.jobs.remove(&id);
    }
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
    service_job_timeout_ms: Option<u64>,
    running_cancellations: Arc<Mutex<HashMap<String, RunningJobCancel>>>,
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
                        let request = *request;
                        status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                        if service_job_cancelled(&request.job_id) {
                            let _ = request.response_tx.send(json!({
                                "id": request.id,
                                "success": false,
                                "error": "Service job was cancelled before dispatch",
                            }));
                            continue;
                        }
                        status.set_state(WorkerState::Busy);
                        persist_service_job_running(&request);
                        if let Ok(mut running) = running_cancellations.lock() {
                            running.insert(request.job_id.clone(), request.cancellation.clone());
                        }
                        refresh_browser_health(&mut state, &status).await;
                        let timeout_ms = request.timeout_ms.or(service_job_timeout_ms);
                        let previous_cancellation = state
                            .current_cancellation
                            .replace(request.cancellation.clone());
                        let response = match timeout_ms {
                            Some(ms) if ms > 0 => {
                                tokio::select! {
                                    response = execute_command(&request.command, &mut state) => response,
                                    _ = request.cancellation.cancelled() => {
                                        persist_service_job_cancelled(&request, "Service job was cancelled while running");
                                        cleanup_exited_browser(&mut state).await;
                                        status.set_browser_health(BrowserHealth::NotStarted);
                                        json!({
                                            "id": request.id.clone(),
                                            "success": false,
                                            "error": "Service job was cancelled while running",
                                            "data": {
                                                "cancelled": true,
                                            },
                                        })
                                    }
                                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                                        persist_service_job_timed_out(&request);
                                        cleanup_exited_browser(&mut state).await;
                                        status.set_browser_health(BrowserHealth::NotStarted);
                                        json!({
                                            "id": request.id.clone(),
                                            "success": false,
                                            "error": format!("Service job timed out after {}ms", ms),
                                            "data": {
                                                "timedOut": true,
                                                "timeoutMs": ms,
                                            },
                                        })
                                    }
                                }
                            }
                            _ => {
                                tokio::select! {
                                    response = execute_command(&request.command, &mut state) => response,
                                    _ = request.cancellation.cancelled() => {
                                        persist_service_job_cancelled(&request, "Service job was cancelled while running");
                                        cleanup_exited_browser(&mut state).await;
                                        status.set_browser_health(BrowserHealth::NotStarted);
                                        json!({
                                            "id": request.id.clone(),
                                            "success": false,
                                            "error": "Service job was cancelled while running",
                                            "data": {
                                                "cancelled": true,
                                            },
                                        })
                                    }
                                }
                            }
                        };
                        state.current_cancellation = previous_cancellation;
                        if let Ok(mut running) = running_cancellations.lock() {
                            running.remove(&request.job_id);
                        }
                        let timed_out = response
                            .pointer("/data/timedOut")
                            .and_then(|value| value.as_bool())
                            == Some(true);
                        let cancelled = response
                            .pointer("/data/cancelled")
                            .and_then(|value| value.as_bool())
                            == Some(true);
                        if cancelled {
                            persist_service_job_cancelled(&request, "Service job was cancelled while running");
                            cleanup_exited_browser(&mut state).await;
                            status.set_browser_health(BrowserHealth::NotStarted);
                        } else {
                            refresh_browser_health(&mut state, &status).await;
                        }
                        if !timed_out && !cancelled {
                            persist_service_job_finished(&request, &response);
                        }
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
    if state.browser.is_some() {
        persist_process_exited_browser_health(state);
    }
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
        let home = temp_home("control-plane-submit");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
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

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        let persisted = store.load().unwrap();
        let job = &persisted.jobs["test-1"];
        assert_eq!(job.action, "state_list");
        assert_eq!(job.state, JobState::Succeeded);
        assert!(job.submitted_at.is_some());
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_some());
        assert_eq!(job.result.as_ref().unwrap()["success"], true);

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn status_response_reports_worker_state() {
        let home = temp_home("control-plane-status");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
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
        let _ = std::fs::remove_dir_all(&home);
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
        assert_eq!(
            response
                .pointer("/data/service_state/events/0/kind")
                .and_then(|v| v.as_str()),
            Some("reconciliation")
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
        assert_eq!(persisted.events.len(), 1);

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn cancel_persisted_service_job_marks_queued_job_cancelled() {
        let home = temp_home("control-plane-cancel-queued");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                jobs: std::collections::BTreeMap::from([(
                    "job-queued".to_string(),
                    ServiceJob {
                        id: "job-queued".to_string(),
                        action: "navigate".to_string(),
                        state: JobState::Queued,
                        submitted_at: Some("2026-04-22T00:00:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let job = cancel_persisted_service_job("job-queued", Some("stale")).unwrap();

        assert_eq!(job.state, JobState::Cancelled);
        assert_eq!(job.error.as_deref(), Some("stale"));
        assert_eq!(job.result.as_ref().unwrap()["cancelled"], true);
        assert!(job.completed_at.is_some());

        let persisted = store.load().unwrap();
        assert_eq!(persisted.jobs["job-queued"].state, JobState::Cancelled);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn cancel_persisted_service_job_rejects_running_job() {
        let home = temp_home("control-plane-cancel-running");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                jobs: std::collections::BTreeMap::from([(
                    "job-running".to_string(),
                    ServiceJob {
                        id: "job-running".to_string(),
                        action: "navigate".to_string(),
                        state: JobState::Running,
                        submitted_at: Some("2026-04-22T00:00:00Z".to_string()),
                        started_at: Some("2026-04-22T00:00:01Z".to_string()),
                        ..ServiceJob::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let err = cancel_persisted_service_job("job-running", Some("stale")).unwrap_err();

        assert!(err.contains("already running"));
        let persisted = store.load().unwrap();
        assert_eq!(persisted.jobs["job-running"].state, JobState::Running);
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
        assert!(persisted.events.iter().any(|event| {
            event.kind == crate::native::service_model::ServiceEventKind::BrowserHealthChanged
                && event.browser_id.as_deref() == Some("browser-1")
        }));
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
        let home = temp_home("control-plane-parallel");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
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
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn parallel_service_config_mutations_are_serialized() {
        let home = temp_home("control-plane-config-parallel");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let mut tasks = Vec::new();

        for idx in 0..48 {
            let handle = handle.clone();
            tasks.push(tokio::spawn(async move {
                handle
                    .submit(json!({
                        "id": format!("test-provider-upsert-{idx}"),
                        "action": "service_provider_upsert",
                        "providerId": format!("provider-{idx}"),
                        "provider": {
                            "kind": "manual_approval",
                            "displayName": format!("Provider {idx}"),
                            "capabilities": ["human_approval"]
                        },
                        "serviceName": "ConfigMutationSmoke",
                        "agentName": "unit-test",
                        "taskName": "parallelConfigMutation"
                    }))
                    .await
            }));
        }

        for task in tasks {
            let response = task.await.expect("config submit task should complete");
            assert_eq!(
                response.get("success").and_then(|value| value.as_bool()),
                Some(true)
            );
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(handle.queue_depth(), 0);
        handle.shutdown().await;

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        let persisted = store.load().unwrap();
        for idx in 0..48 {
            let id = format!("provider-{idx}");
            assert_eq!(
                persisted.providers[&id].display_name,
                format!("Provider {idx}")
            );
            assert_eq!(
                persisted.jobs[&format!("test-provider-upsert-{idx}")].state,
                JobState::Succeeded
            );
        }

        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn service_job_timeout_marks_running_job_timed_out() {
        let home = temp_home("control-plane-job-timeout");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start_with_options(DaemonState::new(), None, Some(10));
        let response = handle
            .submit(json!({
                "id": "test-timeout",
                "action": "__test_sleep",
                "serviceName": "JournalDownloader",
                "agentName": "article-probe-agent",
                "taskName": "probeACSwebsite",
                "ms": 100,
            }))
            .await;

        assert_eq!(
            response.get("success").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            response.pointer("/data/timedOut").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            response.pointer("/data/timeoutMs").and_then(|v| v.as_u64()),
            Some(10)
        );

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        let persisted = store.load().unwrap();
        let job = &persisted.jobs["test-timeout"];
        assert_eq!(job.state, JobState::TimedOut);
        assert_eq!(job.timeout_ms, Some(10));
        assert_eq!(job.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(job.agent_name.as_deref(), Some("article-probe-agent"));
        assert_eq!(job.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(job.result.as_ref().unwrap()["timedOut"], true);

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn service_job_cancel_requests_running_job_cancellation() {
        let home = temp_home("control-plane-running-cancel");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let submit_handle = {
            let handle = handle.clone();
            tokio::spawn(async move {
                handle
                    .submit(json!({
                        "id": "test-running-cancel",
                        "action": "__test_sleep",
                        "ms": 5000,
                    }))
                    .await
            })
        };
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());

        for _ in 0..50 {
            if store
                .load()
                .unwrap()
                .jobs
                .get("test-running-cancel")
                .is_some_and(|job| job.state == JobState::Running)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let cancel_response =
            handle.cancel_job_response("cancel-running", "test-running-cancel", None);

        assert_eq!(
            cancel_response
                .pointer("/data/cancellationRequested")
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let response = submit_handle.await.expect("submit task should complete");

        assert_eq!(
            response.get("success").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            response
                .pointer("/data/cancelled")
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        let persisted = store.load().unwrap();
        let job = &persisted.jobs["test-running-cancel"];
        assert_eq!(job.state, JobState::Cancelled);
        assert_eq!(
            job.error.as_deref(),
            Some("Service job was cancelled while running")
        );

        let follow_up = handle
            .submit(json!({
                "id": "test-after-running-cancel",
                "action": "state_list",
            }))
            .await;

        assert_eq!(
            follow_up.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "worker should accept follow-up work after running cancellation: {}",
            follow_up
        );
        let persisted = store.load().unwrap();
        assert_eq!(
            persisted.jobs["test-after-running-cancel"].state,
            JobState::Succeeded
        );

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn full_queue_returns_structured_error() {
        let home = temp_home("control-plane-full-queue");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
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
                "serviceName": "JournalDownloader",
                "agentName": "article-probe-agent",
                "taskName": "probeACSwebsite",
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

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        let persisted = store.load().unwrap();
        let job = &persisted.jobs["test-full"];
        assert_eq!(job.state, JobState::Failed);
        assert_eq!(job.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(job.agent_name.as_deref(), Some("article-probe-agent"));
        assert_eq!(job.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(job.error.as_deref(), Some("Control queue is full"));

        drop(_permit);
        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }
}
