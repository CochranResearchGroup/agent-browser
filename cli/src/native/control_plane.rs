use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};

use super::action_runtime::runtime::{handle_close, CloseBehavior};
use super::action_runtime::{service_profile_lease_gate, DaemonState, ServiceProfileLeaseGate};
use super::actions::execute_command;
use super::cancellation::CancellationToken as RunningJobCancel;
use super::desktop_evidence_action::redact_desktop_evidence_stream_result;
use super::desktop_interaction::redact_desktop_interaction_stream_result;
use super::desktop_prompt_perception::redact_desktop_prompt_stream_result;
use super::service_health::{
    apply_browser_health_observation, browser_health_observation_details,
    persist_reconciled_service_state_in_repository, reconcile_persisted_service_state,
    record_browser_health_changed_event, remove_browser_operational_record,
};
use super::service_jobs::{
    cancel_persisted_service_job, load_service_job_in_repository, mutate_persisted_service_jobs,
};
use super::service_model::{
    BrowserHealth as ServiceBrowserHealth, BrowserHost as ServiceBrowserHost, BrowserProcess,
    ControlPlaneSnapshot, JobControlPlaneMode, JobPriority, JobState, JobTarget, ServiceActor,
    ServiceEvent, ServiceEventKind, ServiceJob, ServiceState,
    SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME, SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
};
use super::service_monitors::{
    persisted_due_monitor_work_pending, SERVICE_MONITORS_RUN_DUE_ACTION,
};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

const DEFAULT_QUEUE_CAPACITY: usize = 256;
const MAX_SERVICE_EVENTS: usize = 100;

#[derive(Clone)]
pub struct ControlPlaneHandle {
    tx: mpsc::Sender<WorkerMessage>,
    status: Arc<ControlPlaneStatus>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
    running_cancellations: Arc<Mutex<HashMap<String, RunningJobCancel>>>,
}

pub struct ControlPlaneStatus {
    state: AtomicUsize,
    browser_health: AtomicUsize,
    queue_depth: AtomicUsize,
}

struct WorkerRuntimeOptions {
    service_reconcile_interval_ms: Option<u64>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
    running_cancellations: Arc<Mutex<HashMap<String, RunningJobCancel>>>,
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
    pub naming_warnings: Vec<String>,
    pub command: Value,
    pub priority: ControlPriority,
    /// Optional worker-bound execution timeout. The worker records timed-out
    /// requests as service jobs with `timed_out` state.
    pub timeout_ms: Option<u64>,
    pub cancellation: RunningJobCancel,
    pub submitted_at_wall: String,
    pub submitted_at_mono: Instant,
    pub profile_lease_wait_started_at: Option<Instant>,
    pub profile_lease_wait_profile_id: Option<String>,
    pub profile_lease_wait_conflict_session_ids: Vec<String>,
    pub profile_lease_wait_retry_after_ms: Option<u64>,
    pub response_tx: oneshot::Sender<Value>,
}

enum WorkerMessage {
    Request(Box<ControlRequest>),
    Shutdown(oneshot::Sender<()>),
}

pub struct ControlPlaneWorker;

impl ControlPlaneWorker {
    pub fn start(state: DaemonState) -> ControlPlaneHandle {
        Self::start_with_capacity_and_options(state, DEFAULT_QUEUE_CAPACITY, None, None, None)
    }

    pub fn start_with_service_reconcile_interval(
        state: DaemonState,
        service_reconcile_interval_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        Self::start_with_options(state, service_reconcile_interval_ms, None, None)
    }

    pub fn start_with_options(
        state: DaemonState,
        service_reconcile_interval_ms: Option<u64>,
        service_job_timeout_ms: Option<u64>,
        service_monitor_interval_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        Self::start_with_capacity_and_options(
            state,
            DEFAULT_QUEUE_CAPACITY,
            service_reconcile_interval_ms,
            service_job_timeout_ms,
            service_monitor_interval_ms,
        )
    }

    fn start_with_capacity(state: DaemonState, capacity: usize) -> ControlPlaneHandle {
        Self::start_with_capacity_and_options(state, capacity, None, None, None)
    }

    fn start_with_capacity_and_options(
        state: DaemonState,
        capacity: usize,
        service_reconcile_interval_ms: Option<u64>,
        service_job_timeout_ms: Option<u64>,
        service_monitor_interval_ms: Option<u64>,
    ) -> ControlPlaneHandle {
        let (tx, rx) = mpsc::channel(capacity);
        let status = Arc::new(ControlPlaneStatus::new());
        let running_cancellations = Arc::new(Mutex::new(HashMap::new()));
        let runtime_options = WorkerRuntimeOptions {
            service_reconcile_interval_ms,
            service_job_timeout_ms,
            service_monitor_interval_ms,
            running_cancellations: running_cancellations.clone(),
        };
        tokio::spawn(run_worker(
            state,
            tx.clone(),
            rx,
            status.clone(),
            runtime_options,
        ));
        ControlPlaneHandle {
            tx,
            status,
            service_job_timeout_ms,
            service_monitor_interval_ms,
            running_cancellations,
        }
    }
}

impl ControlPlaneHandle {
    pub fn status_response(&self, id: &str) -> Value {
        json!({
            "id": id,
            "success": true,
            "data": self.status_payload(0),
        })
    }

    pub async fn service_status_response(
        &self,
        id: &str,
        service_state: Value,
        launch_config: Value,
        full_tab_history: bool,
    ) -> Value {
        let repository = match LockedServiceStateRepository::default_json() {
            Ok(repository) => repository,
            Err(error) => return json!({ "id": id, "success": false, "error": error }),
        };
        let projector = super::service_status_projection::ServiceStatusProjector::local();
        self.service_status_response_with_dependencies(
            id,
            service_state,
            launch_config,
            full_tab_history,
            super::service_status_projection::ServiceStatusProjectionDependencies::new(
                &repository,
                &super::service_status_projection::ReconcileServiceStatusAuthority,
                &super::service_status_projection::ReconciledBrowserSessionAuthority,
                &projector,
            ),
        )
        .await
    }

    pub(crate) async fn service_status_response_with_dependencies<
        Repository,
        Preparer,
        BrowserAuthority,
    >(
        &self,
        id: &str,
        service_state: Value,
        launch_config: Value,
        full_tab_history: bool,
        dependencies: super::service_status_projection::ServiceStatusProjectionDependencies<
            '_,
            Repository,
            Preparer,
            BrowserAuthority,
        >,
    ) -> Value
    where
        Repository: ServiceStateRepository,
        Preparer: super::service_status_projection::ServiceStatusAuthorityPreparer,
        BrowserAuthority: super::service_status_projection::ServiceStatusBrowserAuthorityProvider,
    {
        let Ok(mut service_state) = serde_json::from_value::<ServiceState>(service_state) else {
            return json!({
                "id": id,
                "success": false,
                "error": "Invalid serviceState",
            });
        };
        let before = service_state.clone();
        let waiting_profile_lease_job_count =
            service_state_waiting_profile_lease_job_count(&service_state);
        service_state.control_plane = Some(self.status_snapshot(waiting_profile_lease_job_count));
        dependencies.preparer.prepare(&mut service_state).await;
        if let Err(error) = persist_reconciled_service_state_in_repository(
            dependencies.repository,
            &before,
            &service_state,
        ) {
            return json!({ "id": id, "success": false, "error": error });
        }
        let browser_session_authority = dependencies.browser_authority.snapshot(&service_state);
        let control_plane = service_state
            .control_plane
            .as_ref()
            .expect("service status always creates a control-plane snapshot");
        let control_plane =
            match super::service_status_projection::StatusControlPlaneAuthority::try_from(
                control_plane,
            ) {
                Ok(control_plane) => control_plane,
                Err(error) => {
                    return json!({ "id": id, "success": false, "error": error.to_string() })
                }
            };
        let result = super::service_status_projection::project_status_with_launch_configuration(
            dependencies.projector,
            service_state,
            control_plane,
            browser_session_authority,
            launch_config,
            full_tab_history,
        )
        .await;
        service_status_result_envelope(id, result)
    }

    fn status_snapshot(&self, waiting_profile_lease_job_count: usize) -> ControlPlaneSnapshot {
        ControlPlaneSnapshot {
            worker_state: self.status.worker_state().as_str().to_string(),
            browser_health: self.status.browser_health().as_str().to_string(),
            queue_depth: self.status.queue_depth(),
            queue_capacity: self.tx.max_capacity(),
            waiting_profile_lease_job_count,
            service_job_timeout_ms: self.service_job_timeout_ms,
            service_monitor_interval_ms: self.service_monitor_interval_ms,
            updated_at: Some(current_timestamp()),
        }
    }

    fn status_payload(&self, waiting_profile_lease_job_count: usize) -> Value {
        json!({
            "worker_state": self.status.worker_state().as_str(),
            "browser_health": self.status.browser_health().as_str(),
            "queue_depth": self.status.queue_depth(),
            "queue_capacity": self.tx.max_capacity(),
            "waiting_profile_lease_job_count": waiting_profile_lease_job_count,
            "service_job_timeout_ms": self.service_job_timeout_ms,
            "service_monitor_interval_ms": self.service_monitor_interval_ms,
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
        let command = command_with_service_job_id(command, &job_id);
        let (command, timeout_ms) =
            command_with_effective_job_timeout(command, self.service_job_timeout_ms);
        let timeout_ms = timeout_ms.filter(|ms| *ms > 0);
        let service_name = optional_command_string(&command, "serviceName");
        let agent_name = optional_command_string(&command, "agentName");
        let task_name = optional_command_string(&command, "taskName");
        let naming_warnings = request_naming_warnings(
            service_name.as_deref(),
            agent_name.as_deref(),
            task_name.as_deref(),
        );
        let request = ControlRequest {
            id: id.clone(),
            job_id,
            action: action.clone(),
            service_name,
            agent_name,
            task_name,
            naming_warnings,
            command,
            priority: ControlPriority::Normal,
            timeout_ms,
            cancellation: RunningJobCancel::new(),
            submitted_at_wall: current_timestamp(),
            submitted_at_mono: Instant::now(),
            profile_lease_wait_started_at: None,
            profile_lease_wait_profile_id: None,
            profile_lease_wait_conflict_session_ids: Vec::new(),
            profile_lease_wait_retry_after_ms: None,
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

pub(crate) fn service_status_result_envelope(
    id: &str,
    result: Result<
        super::service_status_projection::ServiceStatusResponse,
        super::service_status_projection::ServiceStatusProjectionError,
    >,
) -> Value {
    match result {
        Ok(data) => json!({ "id": id, "success": true, "data": data }),
        Err(error) => json!({ "id": id, "success": false, "error": error.to_string() }),
    }
}

fn command_with_service_job_id(mut command: Value, job_id: &str) -> Value {
    if let Some(object) = command.as_object_mut() {
        object.insert(
            "serviceJobId".to_string(),
            Value::String(job_id.to_string()),
        );
    }
    command
}

fn command_with_effective_job_timeout(
    mut command: Value,
    default_timeout_ms: Option<u64>,
) -> (Value, Option<u64>) {
    let timeout_ms = command
        .get("jobTimeoutMs")
        .and_then(Value::as_u64)
        .filter(|timeout_ms| *timeout_ms > 0)
        .or(default_timeout_ms.filter(|timeout_ms| *timeout_ms > 0));
    if command.get("jobTimeoutMs").is_none() {
        if let (Some(timeout_ms), Some(command)) = (timeout_ms, command.as_object_mut()) {
            command.insert("jobTimeoutMs".to_string(), json!(timeout_ms));
        }
    }
    (command, timeout_ms)
}

fn route_bound_action_owns_completion(action: &str) -> bool {
    matches!(
        action,
        "remote_view_open" | "service_remote_view_handoff_resolve"
    )
}

enum CoordinatedExecution {
    Completed(Value),
    CancelledAfterCompensation(Value),
    CancelledAtBoundary,
    TimedOutAtDeadline { timeout_ms: u64 },
}

async fn await_coordinated_execution<F>(
    execution: F,
    cancellation: RunningJobCancel,
    timeout_ms: Option<u64>,
) -> CoordinatedExecution
where
    F: Future<Output = Value>,
{
    let mut execution = Box::pin(execution);
    match timeout_ms {
        Some(timeout_ms) if timeout_ms > 0 => {
            let total_deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
            tokio::select! {
                biased;
                response = &mut execution => CoordinatedExecution::Completed(response),
                _ = cancellation.cancelled() => {
                    cancellation.cancel();
                    match tokio::time::timeout_at(total_deadline, &mut execution).await {
                        Ok(response) => CoordinatedExecution::CancelledAfterCompensation(response),
                        Err(_) => CoordinatedExecution::TimedOutAtDeadline { timeout_ms },
                    }
                }
                _ = tokio::time::sleep_until(total_deadline) => {
                    // The coordinator owns its internal compensation reserve.
                    // Reaching the public total deadline cancels and drops the
                    // unfinished future. It is never detached or awaited past T.
                    cancellation.cancel();
                    CoordinatedExecution::TimedOutAtDeadline { timeout_ms }
                }
            }
        }
        _ => {
            tokio::select! {
                biased;
                response = &mut execution => CoordinatedExecution::Completed(response),
                _ = cancellation.cancelled() => {
                    cancellation.cancel();
                    CoordinatedExecution::CancelledAtBoundary
                }
            }
        }
    }
}

fn service_job_cancelled_response(request: &ControlRequest) -> Value {
    json!({
        "id": request.id.clone(),
        "success": false,
        "error": "Service job was cancelled while running",
        "data": { "cancelled": true },
    })
}

fn service_job_timed_out_response(request: &ControlRequest, timeout_ms: u64) -> Value {
    json!({
        "id": request.id.clone(),
        "success": false,
        "error": format!("Service job timed out after {}ms", timeout_ms),
        "data": {
            "timedOut": true,
            "timeoutMs": timeout_ms,
        },
    })
}

fn coordinated_execution_response(
    request: &ControlRequest,
    execution: CoordinatedExecution,
    timeout_ms: Option<u64>,
) -> Value {
    match execution {
        CoordinatedExecution::CancelledAfterCompensation(terminal_response) => {
            let _ = terminal_response;
            service_job_cancelled_response(request)
        }
        CoordinatedExecution::CancelledAtBoundary => service_job_cancelled_response(request),
        CoordinatedExecution::TimedOutAtDeadline { timeout_ms } => {
            service_job_timed_out_response(request, timeout_ms)
        }
        CoordinatedExecution::Completed(response) => {
            let error = response.get("error").and_then(Value::as_str).unwrap_or("");
            if error.contains("Service job was cancelled while running") {
                service_job_cancelled_response(request)
            } else if error.contains("Service job timed out during route-bound open") {
                service_job_timed_out_response(request, timeout_ms.unwrap_or_default())
            } else {
                response
            }
        }
    }
}

enum SchedulerLeaseDecision {
    Ready,
    Reject(String),
    Wait {
        retry_after_ms: u64,
        profile_id: String,
        conflict_session_ids: Vec<String>,
        first_wait: bool,
    },
}

fn scheduler_profile_lease_gate(
    request: &mut ControlRequest,
    session_id: &str,
) -> SchedulerLeaseDecision {
    let waited_ms = request
        .profile_lease_wait_started_at
        .map(|started_at| started_at.elapsed().as_millis() as u64);
    match service_profile_lease_gate(&request.command, session_id, waited_ms) {
        Ok(ServiceProfileLeaseGate::Ready) => SchedulerLeaseDecision::Ready,
        Ok(ServiceProfileLeaseGate::Reject { error }) => SchedulerLeaseDecision::Reject(error),
        Ok(ServiceProfileLeaseGate::Wait {
            retry_after_ms,
            profile_id,
            conflict_session_ids,
        }) => {
            let first_wait = request.profile_lease_wait_started_at.is_none();
            if first_wait {
                request.profile_lease_wait_started_at = Some(Instant::now());
            }
            request.profile_lease_wait_profile_id = Some(profile_id.clone());
            request.profile_lease_wait_conflict_session_ids = conflict_session_ids.clone();
            request.profile_lease_wait_retry_after_ms = Some(retry_after_ms);
            SchedulerLeaseDecision::Wait {
                retry_after_ms,
                profile_id,
                conflict_session_ids,
                first_wait,
            }
        }
        Err(error) => SchedulerLeaseDecision::Reject(error),
    }
}

fn service_browser_id(session_id: &str) -> String {
    format!("session:{}", session_id)
}

fn persist_process_exited_browser_health(state: &DaemonState) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ = persist_process_exited_browser_health_in_repository(&repository, state);
    }
}

fn persist_process_exited_browser_health_in_repository(
    repository: &impl ServiceStateRepository,
    state: &DaemonState,
) -> Result<(), String> {
    repository.mutate(|service_state| {
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
            boot_epoch: crate::process_identity::current_boot_epoch(),
            profile_id: previous
                .as_ref()
                .and_then(|browser| browser.profile_id.clone()),
            host,
            health: ServiceBrowserHealth::ProcessExited,
            display_isolation: previous
                .as_ref()
                .and_then(|browser| browser.display_isolation.clone()),
            display_name: previous
                .as_ref()
                .and_then(|browser| browser.display_name.clone()),
            display_allocation_id: previous
                .as_ref()
                .and_then(|browser| browser.display_allocation_id.clone()),
            pid,
            cdp_endpoint,
            view_streams: previous
                .as_ref()
                .map(|browser| browser.view_streams.clone())
                .unwrap_or_default(),
            active_session_ids: vec![state.session_id.clone()],
            tab_handles: previous
                .as_ref()
                .map(|browser| browser.tab_handles.clone())
                .unwrap_or_default(),
            last_error,
            last_health_observation: None,
            attachability: None,
        };
        let observation_details = browser_health_observation_details(&browser, None);
        apply_browser_health_observation(&mut browser, Some(&observation_details));
        record_browser_health_changed_event(service_state, &id, previous.as_ref(), &browser);
        if let Some(display_allocation_id) = browser.display_allocation_id.as_ref() {
            if let Some(allocation) = service_state
                .display_allocations
                .get_mut(display_allocation_id)
            {
                allocation.state = "orphaned".to_string();
                allocation.owner_browser_id = Some(id.clone());
                allocation.owner_session_id = Some(state.session_id.clone());
                allocation.updated_at = Some(current_timestamp());
                allocation.readiness = Some(json!({
                    "state": "orphaned",
                    "reason": "browser_process_exited"
                }));
            }
        }
        remove_browser_operational_record(service_state, &id, Some(&state.session_id));
        Ok(())
    })
}

fn service_state_waiting_profile_lease_job_count(service_state: &ServiceState) -> usize {
    service_state
        .jobs
        .values()
        .filter(|job| job.state == JobState::WaitingProfileLease)
        .count()
}

/// Persist a bounded audit record for each control-plane request.
fn persist_service_job(job: ServiceJob) {
    mutate_persisted_service_jobs(|state| {
        state.jobs.insert(job.id.clone(), job);
    });
}

fn persist_service_job_queued(request: &ControlRequest) {
    let allocation_refs = service_job_allocation_refs(request, None);
    persist_service_job(ServiceJob {
        id: service_job_id(request),
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::Queued,
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        timeout_ms: request.timeout_ms,
        ..ServiceJob::default()
    });
}

fn persist_service_job_waiting_profile_lease(
    request: &ControlRequest,
    retry_after_ms: u64,
    profile_id: &str,
    conflict_session_ids: &[String],
) {
    let allocation_refs = service_job_allocation_refs(request, None);
    persist_service_job(ServiceJob {
        id: service_job_id(request),
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
        target: JobTarget::Service,
        owner: ServiceActor::System,
        state: JobState::WaitingProfileLease,
        priority: service_job_priority(request.priority),
        submitted_at: Some(request.submitted_at_wall.clone()),
        timeout_ms: request.timeout_ms,
        result: Some(json!({
            "waitingProfileLease": true,
            "profileId": profile_id,
            "conflictSessionIds": conflict_session_ids,
            "retryAfterMs": retry_after_ms,
        })),
        ..ServiceJob::default()
    });
}

fn record_profile_lease_wait_started_event(
    request: &ControlRequest,
    profile_id: &str,
    conflict_session_ids: &[String],
    retry_after_ms: u64,
) {
    record_profile_lease_wait_event(
        request,
        ProfileLeaseWaitEvent {
            kind: ServiceEventKind::ProfileLeaseWaitStarted,
            outcome: "started",
            profile_id: Some(profile_id),
            conflict_session_ids,
            retry_after_ms: Some(retry_after_ms),
            waited_ms: None,
            error: None,
        },
    );
}

fn record_profile_lease_wait_ended_event(
    request: &ControlRequest,
    outcome: &str,
    error: Option<&str>,
) {
    let waited_ms = request
        .profile_lease_wait_started_at
        .map(|started_at| started_at.elapsed().as_millis() as u64);
    record_profile_lease_wait_event(
        request,
        ProfileLeaseWaitEvent {
            kind: ServiceEventKind::ProfileLeaseWaitEnded,
            outcome,
            profile_id: request.profile_lease_wait_profile_id.as_deref(),
            conflict_session_ids: &request.profile_lease_wait_conflict_session_ids,
            retry_after_ms: request.profile_lease_wait_retry_after_ms,
            waited_ms,
            error,
        },
    );
}

struct ProfileLeaseWaitEvent<'a> {
    kind: ServiceEventKind,
    outcome: &'a str,
    profile_id: Option<&'a str>,
    conflict_session_ids: &'a [String],
    retry_after_ms: Option<u64>,
    waited_ms: Option<u64>,
    error: Option<&'a str>,
}

fn record_profile_lease_wait_event(request: &ControlRequest, event: ProfileLeaseWaitEvent<'_>) {
    mutate_persisted_service_jobs(|state| {
        let mut details = json!({
            "jobId": service_job_id(request),
            "action": request.action,
            "outcome": event.outcome,
            "profileId": event.profile_id,
            "conflictSessionIds": event.conflict_session_ids,
            "retryAfterMs": event.retry_after_ms,
            "waitedMs": event.waited_ms,
        });
        if let Some(error) = event.error {
            details["error"] = json!(error);
        }
        state.events.push(ServiceEvent {
            id: format!("event-{}", uuid::Uuid::new_v4()),
            timestamp: current_timestamp(),
            kind: event.kind,
            message: profile_lease_wait_event_message(request, event.outcome, event.profile_id),
            profile_id: event.profile_id.map(str::to_string),
            session_id: None,
            service_name: request.service_name.clone(),
            agent_name: request.agent_name.clone(),
            task_name: request.task_name.clone(),
            details: Some(details),
            ..ServiceEvent::default()
        });
        if state.events.len() > MAX_SERVICE_EVENTS {
            let excess = state.events.len() - MAX_SERVICE_EVENTS;
            state.events.drain(0..excess);
        }
    });
}

fn profile_lease_wait_event_message(
    request: &ControlRequest,
    outcome: &str,
    profile_id: Option<&str>,
) -> String {
    let profile = profile_id.unwrap_or("unknown profile");
    match outcome {
        "started" => format!(
            "Service job {} started waiting for profile lease {}",
            service_job_id(request),
            profile
        ),
        _ => format!(
            "Service job {} ended profile lease wait for {} with outcome {}",
            service_job_id(request),
            profile,
            outcome
        ),
    }
}

fn persist_service_job_running(request: &ControlRequest) {
    let allocation_refs = service_job_allocation_refs(request, None);
    persist_service_job(ServiceJob {
        id: service_job_id(request),
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
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
    let allocation_refs = service_job_allocation_refs(request, Some(response));

    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
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
        result: Some(service_job_persisted_result(request, response)),
        error,
    });
}

fn service_job_persisted_result(request: &ControlRequest, response: &Value) -> Value {
    let success = response
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if request.action == "desktop_interact" {
        return json!({
            "success": success,
            "data": redact_desktop_interaction_stream_result(
                response.get("data").unwrap_or(&Value::Null),
            ),
        });
    }
    if request.action == "desktop_evidence_observe" {
        return json!({
            "success": success,
            "data": redact_desktop_evidence_stream_result(
                response.get("data").unwrap_or(&Value::Null),
            ),
        });
    }
    if request.action == "desktop_prompt_observe" {
        return json!({
            "success": success,
            "data": redact_desktop_prompt_stream_result(
                response.get("data").unwrap_or(&Value::Null),
            ),
        });
    }
    json!({ "success": success })
}

fn persist_service_job_timed_out(request: &ControlRequest) {
    let job_id = service_job_id(request);
    let started_at = load_service_job(&job_id)
        .and_then(|job| job.started_at)
        .unwrap_or_else(current_timestamp);
    let timeout_ms = request.timeout_ms.unwrap_or_default();
    let allocation_refs = service_job_allocation_refs(request, None);
    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
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
    let allocation_refs = service_job_allocation_refs(request, None);
    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
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
    let allocation_refs = service_job_allocation_refs(request, None);
    persist_service_job(ServiceJob {
        id: job_id,
        action: request.action.clone(),
        service_name: request.service_name.clone(),
        agent_name: request.agent_name.clone(),
        task_name: request.task_name.clone(),
        target_service_id: service_job_optional_command_string(request, "targetServiceId"),
        site_id: service_job_optional_command_string(request, "siteId"),
        login_id: service_job_optional_command_string(request, "loginId"),
        target_service_ids: service_job_target_service_ids(request),
        naming_warnings: request.naming_warnings.clone(),
        has_naming_warning: !request.naming_warnings.is_empty(),
        control_plane_mode: service_job_control_plane_mode(request),
        lifecycle_only: service_job_lifecycle_only(request),
        display_isolation: service_job_display_isolation(request),
        requested_display_allocation_id: allocation_refs.requested_display_allocation_id,
        display_allocation_id: allocation_refs.display_allocation_id,
        requested_remote_view_route_id: allocation_refs.requested_remote_view_route_id,
        remote_view_route_id: allocation_refs.remote_view_route_id,
        route_pool_entry_id: allocation_refs.route_pool_entry_id,
        viewer_lease_id: allocation_refs.viewer_lease_id,
        controller_lease_id: allocation_refs.controller_lease_id,
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

fn service_job_cancelled(job_id: &str) -> bool {
    load_service_job(job_id).is_some_and(|job| job.state == JobState::Cancelled)
}

fn enqueue_due_monitor_run(
    tx: &mpsc::Sender<WorkerMessage>,
    status: &Arc<ControlPlaneStatus>,
    service_job_timeout_ms: Option<u64>,
) {
    if !persisted_due_monitor_work_pending() {
        return;
    }
    let (response_tx, _response_rx) = oneshot::channel();
    let id = format!("service-monitor-run-{}", uuid::Uuid::new_v4());
    let request = ControlRequest {
        id: id.clone(),
        job_id: id.clone(),
        action: SERVICE_MONITORS_RUN_DUE_ACTION.to_string(),
        service_name: Some("agent-browser".to_string()),
        agent_name: Some("service-monitor-scheduler".to_string()),
        task_name: Some("run-due-monitors".to_string()),
        naming_warnings: Vec::new(),
        command: json!({
            "id": id,
            "action": SERVICE_MONITORS_RUN_DUE_ACTION,
        }),
        priority: ControlPriority::Lifecycle,
        timeout_ms: service_job_timeout_ms,
        cancellation: RunningJobCancel::new(),
        submitted_at_wall: current_timestamp(),
        submitted_at_mono: Instant::now(),
        profile_lease_wait_started_at: None,
        profile_lease_wait_profile_id: None,
        profile_lease_wait_conflict_session_ids: Vec::new(),
        profile_lease_wait_retry_after_ms: None,
        response_tx,
    };
    status.queue_depth.fetch_add(1, Ordering::Relaxed);
    persist_service_job_queued(&request);
    match tx.try_send(WorkerMessage::Request(Box::new(request))) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(WorkerMessage::Request(request))) => {
            let request = *request;
            status.queue_depth.fetch_sub(1, Ordering::Relaxed);
            persist_service_job_failed_to_enqueue(&request, "Control queue is full");
        }
        Err(mpsc::error::TrySendError::Closed(WorkerMessage::Request(request))) => {
            let request = *request;
            status.queue_depth.fetch_sub(1, Ordering::Relaxed);
            persist_service_job_failed_to_enqueue(&request, "Control plane worker is stopped");
        }
        Err(mpsc::error::TrySendError::Full(WorkerMessage::Shutdown(_)))
        | Err(mpsc::error::TrySendError::Closed(WorkerMessage::Shutdown(_))) => {
            status.queue_depth.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

fn load_service_job(id: &str) -> Option<ServiceJob> {
    let repository = LockedServiceStateRepository::default_json().ok()?;
    load_service_job_in_repository(&repository, id)
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

fn service_job_control_plane_mode(request: &ControlRequest) -> JobControlPlaneMode {
    if request.action == "cdp_free_launch"
        || request
            .command
            .get("requiresCdpFree")
            .and_then(Value::as_bool)
            == Some(true)
    {
        JobControlPlaneMode::CdpFree
    } else if request.priority == ControlPriority::Lifecycle
        || request.action == "view_takeover"
        || matches!(
            request.action.as_str(),
            "desktop_capture"
                | "desktop_locate"
                | "desktop_evidence_observe"
                | "desktop_prompt_observe"
                | "desktop_interact"
        )
        || request.action.starts_with("service_")
    {
        JobControlPlaneMode::Service
    } else {
        JobControlPlaneMode::Cdp
    }
}

fn service_job_lifecycle_only(request: &ControlRequest) -> bool {
    matches!(
        service_job_control_plane_mode(request),
        JobControlPlaneMode::CdpFree | JobControlPlaneMode::Service
    )
}

fn service_job_display_isolation(request: &ControlRequest) -> Option<String> {
    service_job_display_isolation_from_command(&request.command)
}

fn service_job_display_isolation_from_command(command: &Value) -> Option<String> {
    optional_command_string(command, "displayIsolation")
        .or_else(|| optional_command_string(command, "displayAllocation"))
        .or_else(|| optional_command_string(command, "displayAllocationPolicy"))
        .or_else(|| {
            command
                .get("params")
                .and_then(service_job_display_isolation_from_command)
        })
        .and_then(|value| normalize_service_job_display_isolation(&value))
}

fn normalize_service_job_display_isolation(value: &str) -> Option<String> {
    match value.trim() {
        "private_virtual_display" | "private-virtual-display" | "private" => {
            Some("private_virtual_display".to_string())
        }
        "shared_display" | "shared-display" | "shared" => Some("shared_display".to_string()),
        "ambient_display" | "ambient-display" | "ambient" => Some("ambient_display".to_string()),
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
struct ServiceJobAllocationRefs {
    requested_display_allocation_id: Option<String>,
    display_allocation_id: Option<String>,
    requested_remote_view_route_id: Option<String>,
    remote_view_route_id: Option<String>,
    route_pool_entry_id: Option<String>,
    viewer_lease_id: Option<String>,
    controller_lease_id: Option<String>,
}

fn service_job_allocation_refs(
    request: &ControlRequest,
    response: Option<&Value>,
) -> ServiceJobAllocationRefs {
    let requested_display_allocation_id = service_job_command_string_any(
        &request.command,
        &[
            "requestedDisplayAllocationId",
            "displayAllocationId",
            "displayAllocation",
        ],
    );
    let requested_remote_view_route_id = service_job_command_string_any(
        &request.command,
        &[
            "requestedRemoteViewRouteId",
            "remoteViewRouteId",
            "routeId",
            "viewStreamRouteId",
        ],
    );
    let requested_viewer_lease_id = service_job_command_string_any(
        &request.command,
        &["viewerLeaseId", "requestedViewerLeaseId"],
    );
    let requested_controller_lease_id = service_job_command_string_any(
        &request.command,
        &["controllerLeaseId", "requestedControllerLeaseId"],
    );
    ServiceJobAllocationRefs {
        display_allocation_id: service_job_response_string_any(
            response,
            &["displayAllocationId", "resolvedDisplayAllocationId"],
        )
        .or_else(|| requested_display_allocation_id.clone()),
        remote_view_route_id: service_job_response_string_any(
            response,
            &["remoteViewRouteId", "routeId", "resolvedRemoteViewRouteId"],
        )
        .or_else(|| requested_remote_view_route_id.clone()),
        route_pool_entry_id: service_job_response_string_any(response, &["routePoolEntryId"])
            .or_else(|| service_job_command_string_any(&request.command, &["routePoolEntryId"])),
        viewer_lease_id: service_job_response_string_any(response, &["viewerLeaseId"])
            .or_else(|| requested_viewer_lease_id.clone()),
        controller_lease_id: service_job_response_string_any(response, &["controllerLeaseId"])
            .or(requested_controller_lease_id),
        requested_display_allocation_id,
        requested_remote_view_route_id,
    }
}

fn service_job_command_string_any(command: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = optional_command_string(command, key) {
            return Some(value);
        }
    }
    if let Some(params) = command.get("params") {
        for key in keys {
            if let Some(value) = optional_command_string(params, key) {
                return Some(value);
            }
        }
    }
    None
}

fn service_job_response_string_any(response: Option<&Value>, keys: &[&str]) -> Option<String> {
    let response = response?;
    for container in [response.get("data"), Some(response)].into_iter().flatten() {
        for candidate in [
            container.get("context"),
            container.get("interactionReceipt"),
            Some(container),
        ]
        .into_iter()
        .flatten()
        {
            for key in keys {
                if let Some(value) = optional_command_string(candidate, key) {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn service_job_optional_command_string(request: &ControlRequest, name: &str) -> Option<String> {
    optional_command_string(&request.command, name)
}

fn service_job_target_service_ids(request: &ControlRequest) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "targetServiceId",
        "targetService",
        "siteId",
        "loginId",
        "accountId",
        "account",
        "target_service_id",
        "site_id",
        "login_id",
        "account_id",
    ] {
        if let Some(value) = request.command.get(key).and_then(|value| value.as_str()) {
            merge_service_job_target_service_id(&mut values, value);
        }
    }
    for key in [
        "targetServiceIds",
        "targetServices",
        "siteIds",
        "loginIds",
        "accountIds",
        "accounts",
        "target_service_ids",
        "site_ids",
        "login_ids",
        "account_ids",
    ] {
        if let Some(raw_values) = request.command.get(key).and_then(|value| value.as_array()) {
            for value in raw_values.iter().filter_map(|value| value.as_str()) {
                merge_service_job_target_service_id(&mut values, value);
            }
        }
    }
    values
}

fn merge_service_job_target_service_id(values: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() && !values.iter().any(|existing| existing == trimmed) {
        values.push(trimmed.to_string());
    }
}

fn request_naming_warnings(
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Vec<String> {
    [
        service_name
            .is_none()
            .then_some(SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME),
        agent_name
            .is_none()
            .then_some(SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME),
        task_name
            .is_none()
            .then_some(SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME),
    ]
    .into_iter()
    .flatten()
    .map(str::to_string)
    .collect()
}

fn optional_command_string(command: &Value, name: &str) -> Option<String> {
    command
        .get(name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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
    tx: mpsc::Sender<WorkerMessage>,
    mut rx: mpsc::Receiver<WorkerMessage>,
    status: Arc<ControlPlaneStatus>,
    runtime_options: WorkerRuntimeOptions,
) {
    let mut drain_interval = tokio::time::interval(Duration::from_millis(100));
    drain_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut service_reconcile_interval = runtime_options.service_reconcile_interval_ms.map(|ms| {
        let mut interval = tokio::time::interval(Duration::from_millis(ms));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval
    });
    let mut service_monitor_interval = runtime_options.service_monitor_interval_ms.map(|ms| {
        let mut interval = tokio::time::interval(Duration::from_millis(ms));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval
    });
    let service_job_timeout_ms = runtime_options.service_job_timeout_ms;
    let running_cancellations = runtime_options.running_cancellations;
    status.set_state(WorkerState::Ready);

    loop {
        tokio::select! {
            maybe_message = rx.recv() => {
                let Some(message) = maybe_message else {
                    break;
                };

                match message {
                    WorkerMessage::Request(request) => {
                        let mut request = *request;
                        let queue_wait_ms = u64::try_from(
                            request.submitted_at_mono.elapsed().as_millis(),
                        )
                        .unwrap_or(u64::MAX);
                        status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                        if service_job_cancelled(&request.job_id) {
                            if request.profile_lease_wait_started_at.is_some() {
                                record_profile_lease_wait_ended_event(
                                    &request,
                                    "cancelled",
                                    Some("Service job was cancelled before dispatch"),
                                );
                            }
                            let _ = request.response_tx.send(json!({
                                "id": request.id,
                                "success": false,
                                "error": "Service job was cancelled before dispatch",
                            }));
                            continue;
                        }
                        match scheduler_profile_lease_gate(&mut request, &state.session_id) {
                            SchedulerLeaseDecision::Ready => {
                                if request.profile_lease_wait_started_at.is_some() {
                                    record_profile_lease_wait_ended_event(&request, "ready", None);
                                }
                            }
                            SchedulerLeaseDecision::Reject(error) => {
                                if request.profile_lease_wait_started_at.is_some() {
                                    record_profile_lease_wait_ended_event(
                                        &request,
                                        "timed_out",
                                        Some(&error),
                                    );
                                }
                                persist_service_job_failed_to_enqueue(&request, &error);
                                let _ = request.response_tx.send(json!({
                                    "id": request.id,
                                    "success": false,
                                    "error": error,
                                }));
                                continue;
                            }
                            SchedulerLeaseDecision::Wait {
                                retry_after_ms,
                                profile_id,
                                conflict_session_ids,
                                first_wait,
                            } => {
                                persist_service_job_waiting_profile_lease(
                                    &request,
                                    retry_after_ms,
                                    &profile_id,
                                    &conflict_session_ids,
                                );
                                if first_wait {
                                    record_profile_lease_wait_started_event(
                                        &request,
                                        &profile_id,
                                        &conflict_session_ids,
                                        retry_after_ms,
                                    );
                                }
                                status.queue_depth.fetch_add(1, Ordering::Relaxed);
                                let tx = tx.clone();
                                let status = status.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(Duration::from_millis(retry_after_ms)).await;
                                    if let Err(err) = tx.send(WorkerMessage::Request(Box::new(request))).await {
                                        let WorkerMessage::Request(request) = err.0 else {
                                            status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                                            return;
                                        };
                                        let request = *request;
                                        status.queue_depth.fetch_sub(1, Ordering::Relaxed);
                                        persist_service_job_failed_to_enqueue(
                                            &request,
                                            "Control plane worker is stopped while waiting for profile lease",
                                        );
                                        if request.profile_lease_wait_started_at.is_some() {
                                            record_profile_lease_wait_ended_event(
                                                &request,
                                                "worker_stopped",
                                                Some("Control plane worker is stopped while waiting for profile lease"),
                                            );
                                        }
                                        let _ = request.response_tx.send(json!({
                                            "id": request.id,
                                            "success": false,
                                            "error": "Control plane worker is stopped while waiting for profile lease",
                                        }));
                                    }
                                });
                                continue;
                            }
                        }
                        status.set_state(WorkerState::Busy);
                        persist_service_job_running(&request);
                        if let Ok(mut running) = running_cancellations.lock() {
                            running.insert(request.job_id.clone(), request.cancellation.clone());
                        }
                        let timeout_ms = request.timeout_ms.or(service_job_timeout_ms);
                        let previous_cancellation = state
                            .current_cancellation
                            .replace(request.cancellation.clone());
                        let mut response = if route_bound_action_owns_completion(&request.action) {
                            let execution = await_coordinated_execution(
                                execute_command(&request.command, &mut state),
                                request.cancellation.clone(),
                                timeout_ms,
                            )
                            .await;
                            coordinated_execution_response(&request, execution, timeout_ms)
                        } else {
                            match timeout_ms {
                            Some(ms) if ms > 0 => {
                                tokio::select! {
                                    response = execute_command(&request.command, &mut state) => response,
                                    _ = request.cancellation.cancelled() => {
                                        service_job_cancelled_response(&request)
                                    }
                                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                                        service_job_timed_out_response(&request, ms)
                                    }
                                }
                            }
                            _ => {
                                tokio::select! {
                                    response = execute_command(&request.command, &mut state) => response,
                                    _ = request.cancellation.cancelled() => {
                                        service_job_cancelled_response(&request)
                                    }
                                }
                            }
                            }
                        };
                        if request
                            .command
                            .get("includeTimings")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        {
                            let daemon_total_ms = response
                                .pointer("/timings/daemonTotalMs")
                                .and_then(Value::as_u64)
                                .unwrap_or(0);
                            response["timings"]["queueWaitMs"] = json!(queue_wait_ms);
                            response["timings"]["totalMs"] =
                                json!(queue_wait_ms.saturating_add(daemon_total_ms));
                        }
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
                        }
                        if timed_out {
                            persist_service_job_timed_out(&request);
                        }
                        if !timed_out && !cancelled {
                            persist_service_job_finished(&request, &response);
                        }
                        send_response_before_follow_up(
                            request.response_tx,
                            response,
                            refresh_browser_health(&mut state, &status),
                        )
                        .await;
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
            _ = async {
                match service_monitor_interval.as_mut() {
                    Some(interval) => interval.tick().await,
                    None => std::future::pending::<tokio::time::Instant>().await,
                }
            }, if service_monitor_interval.is_some() => {
                enqueue_due_monitor_run(&tx, &status, service_job_timeout_ms);
            }
        }
    }

    status.set_state(WorkerState::Stopped);
}

async fn close_browser(state: &mut DaemonState) {
    if state.browser.is_some() {
        let close_behavior = shutdown_close_behavior(state.runtime_owner_binding.as_ref());
        if close_behavior == CloseBehavior::Detach {
            if let Some(manager) = state.browser.as_mut() {
                manager.relinquish_browser_for_handoff();
            }
            state.browser = None;
            return;
        }
        state.close_behavior = close_behavior;
        let _ = handle_close(state).await;
    }
}

fn shutdown_close_behavior(
    binding: Option<&crate::runtime_owner_transfer::RuntimeOwnerBinding>,
) -> CloseBehavior {
    let Some(binding) = binding else {
        return CloseBehavior::CloseBrowser;
    };
    let current = LockedServiceStateRepository::default_json()
        .and_then(|repository| {
            crate::runtime_owner_transfer::owner_authority_is_current(&repository, &binding.claim)
        })
        .unwrap_or(false);
    if current {
        CloseBehavior::CloseBrowser
    } else {
        CloseBehavior::Detach
    }
}

async fn cleanup_exited_browser(state: &mut DaemonState) {
    if state.browser.is_some() {
        persist_process_exited_browser_health(state);
    }
    if state.browser.is_some() {
        state.close_behavior = CloseBehavior::CloseBrowser;
        let _ = handle_close(state).await;
    }
}

fn browser_health_requires_cleanup_after_interruption(health: BrowserHealth) -> bool {
    health == BrowserHealth::ProcessExited
}

async fn refresh_browser_health(state: &mut DaemonState, status: &ControlPlaneStatus) {
    let Some(ref mut mgr) = state.browser else {
        status.set_browser_health(BrowserHealth::NotStarted);
        return;
    };

    if mgr.has_process_exited() {
        let health = BrowserHealth::ProcessExited;
        status.set_browser_health(health);
        if browser_health_requires_cleanup_after_interruption(health) {
            cleanup_exited_browser(state).await;
        }
        return;
    }

    if mgr.is_connection_alive().await {
        status.set_browser_health(BrowserHealth::Ready);
    } else {
        status.set_browser_health(BrowserHealth::CdpDisconnected);
    }
}

async fn send_response_before_follow_up<F>(
    response_tx: oneshot::Sender<Value>,
    response: Value,
    follow_up: F,
) where
    F: Future<Output = ()>,
{
    let _ = response_tx.send(response);
    follow_up.await;
}

#[cfg(test)]
mod tests {
    use super::super::service_jobs::{
        cancel_service_job_in_repository, mutate_service_jobs_in_repository, MAX_SERVICE_JOBS,
    };
    use super::super::service_model::{
        BrowserSession, DisplayAllocation, LeaseState, MonitorState, MonitorTarget, SiteMonitor,
        SitePolicy,
    };
    use super::super::service_store::{JsonServiceStateStore, ServiceStateStore};
    use super::*;
    use crate::test_utils::EnvGuard;

    #[test]
    fn shutdown_preserves_browser_when_owner_authority_is_stale() {
        let home = temp_home("control-plane-stale-owner-shutdown");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let binding = crate::runtime_owner_transfer::RuntimeOwnerBinding {
            claim: crate::runtime_owner_transfer::OwnerAuthorityClaim {
                owner_id: "retired-owner".to_string(),
                profile_identity_digest: "1".repeat(64),
                owner_generation: 1,
                logical_browser_id: "browser-a".to_string(),
                daemon_session_route: "session-a".to_string(),
                process_instance_digest: "2".repeat(64),
            },
            effect_capable: true,
        };

        assert_eq!(
            shutdown_close_behavior(Some(&binding)),
            CloseBehavior::Detach
        );
        let repository = LockedServiceStateRepository::default_json().unwrap();
        repository
            .mutate(|state| {
                state.runtime_owner_registry.owners.insert(
                    binding.claim.profile_identity_digest.clone(),
                    crate::runtime_owner_transfer::ProfileOwner {
                        owner_id: binding.claim.owner_id.clone(),
                        profile_identity_digest: binding.claim.profile_identity_digest.clone(),
                        state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                        owner_generation: binding.claim.owner_generation,
                        browser_id: binding.claim.logical_browser_id.clone(),
                        daemon_session_route: binding.claim.daemon_session_route.clone(),
                        process_instance_digest: binding.claim.process_instance_digest.clone(),
                        browser_family: "chrome".to_string(),
                        cdp_endpoint_identity_digest: "3".repeat(64),
                        target_set_digest: "4".repeat(64),
                        pending_transfer: None,
                        last_transition: None,
                    },
                );
                Ok(())
            })
            .unwrap();
        assert_eq!(
            shutdown_close_behavior(Some(&binding)),
            CloseBehavior::CloseBrowser
        );
        assert_eq!(shutdown_close_behavior(None), CloseBehavior::CloseBrowser);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn persisted_job_result_excludes_response_only_desktop_pixels() {
        let request = control_request_for_mode_test(json!({ "action": "desktop_capture" }));
        let response = json!({
            "success": true,
            "data": {
                "context": { "contextId": "desktop-context-1" },
                "frameReceipt": { "frameId": "desktop-frame-1" },
                "imageBase64": "sensitive-pixels"
            }
        });

        let persisted = service_job_persisted_result(&request, &response);

        assert_eq!(persisted, json!({ "success": true }));
        assert!(!persisted.to_string().contains("sensitive-pixels"));
    }

    #[test]
    fn persisted_desktop_interaction_uses_the_stream_safe_receipt_projection() {
        let request = control_request_for_mode_test(json!({ "action": "desktop_interact" }));
        let response = json!({
            "success": true,
            "data": {
                "ok": true,
                "action": "desktop_interact",
                "interactionReceipt": {
                    "transactionId": "transaction-1",
                    "recipeId": "p110-pointer-keyboard-v1",
                    "textLength": 13,
                    "textSha256": "text-digest",
                    "emittedPathSha256": "path-digest",
                    "persistedPixels": false,
                    "imageBase64": "sensitive-pixels",
                    "text": "sensitive-plaintext",
                    "emittedPath": [{ "x": 1, "y": 2 }],
                    "outputPath": "/sensitive/full/path"
                }
            }
        });

        let persisted = service_job_persisted_result(&request, &response);

        assert_eq!(persisted["success"], json!(true));
        assert_eq!(persisted["data"]["ok"], json!(true));
        assert_eq!(
            persisted["data"]["interactionReceipt"]["transactionId"],
            json!("transaction-1")
        );
        assert_eq!(
            persisted["data"]["interactionReceipt"]["emittedPathSha256"],
            json!("path-digest")
        );
        let serialized = persisted.to_string();
        assert!(!serialized.contains("sensitive-pixels"));
        assert!(!serialized.contains("sensitive-plaintext"));
        assert!(!serialized.contains("emittedPath\""));
        assert!(!serialized.contains("/sensitive/full/path"));
    }

    #[test]
    fn persisted_desktop_evidence_removes_response_pixels_and_provider_details() {
        let request = control_request_for_mode_test(json!({
            "action": "desktop_evidence_observe"
        }));
        let persisted = service_job_persisted_result(
            &request,
            &json!({
                "success": true,
                "data": {
                    "ok": true,
                    "action": "desktop_evidence_observe",
                    "evidenceSurface": "stacking_or_occlusion",
                    "episode": { "outcome": "desktop" },
                    "context": {
                        "contextId": "context-1",
                        "browserId": "browser-1",
                        "displayName": ":101"
                    },
                    "frameReceipt": {
                        "frameId": "frame-1",
                        "sha256": "safe-hash",
                        "providerVersion": "private-provider"
                    },
                    "frameBase64": "PRIVATE_PIXELS"
                }
            }),
        );
        let encoded = serde_json::to_string(&persisted).unwrap();

        assert!(encoded.contains("safe-hash"));
        assert!(!encoded.contains("PRIVATE_PIXELS"));
        assert!(!encoded.contains("private-provider"));
        assert!(!encoded.contains(":101"));
    }

    #[test]
    fn persisted_foundation_stress_receipt_hashes_operation_and_excludes_private_handoff() {
        let request = control_request_for_mode_test(json!({ "action": "desktop_interact" }));
        let response = json!({
            "success": true,
            "data": {
                "ok": true,
                "action": "desktop_interact",
                "interactionReceipt": {
                    "transactionId": "transaction-stress-1",
                    "recipeId": "p110-foundation-stress-v1",
                    "operationId": "private-operation-id",
                    "operationRequestSha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "recipeProviderId": "synthetic-fixture-provider",
                    "recipeProviderVersion": "v1",
                    "recipeProviderCapability": "effect_key_dedup_v1",
                    "promptDisposition": {
                        "state": "operator_intervention_required",
                        "reasonCode": "synthetic_prompt_requires_operator_review",
                        "observationSha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                        "promptText": "sensitive prompt"
                    },
                    "humanHandoff": {
                        "state": "required",
                        "reason": "effect_uncertain",
                        "handoffId": "handoff-opaque-1",
                        "handoffUrl": "https://provider.invalid/raw-route"
                    },
                    "entryGate": "closed_live_evidence_required",
                    "effectKeyDigest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                    "effectKeyCount": 3,
                    "attemptedEffectKeyDigest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                    "attemptedEffectKeyCount": 4,
                    "acknowledgedEffectKeyDigest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                    "acknowledgedEffectKeyCount": 3,
                    "attemptedEventOrderSha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "routeId": "raw-route-id",
                    "imageBase64": "sensitive-pixels",
                    "text": "sensitive-plaintext",
                    "outputPath": "/sensitive/full/path"
                }
            }
        });

        let persisted = service_job_persisted_result(&request, &response);
        let receipt = &persisted["data"]["interactionReceipt"];
        assert!(receipt["operationId"].is_null());
        assert!(receipt["operationIdDigest"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(receipt["humanHandoff"]["handoffId"], "handoff-opaque-1");
        assert!(receipt["humanHandoff"]["handoffUrl"].is_null());
        assert!(receipt["routeId"].is_null());
        assert_eq!(receipt["effectKeyCount"], 3);
        assert_eq!(receipt["attemptedEffectKeyCount"], 4);
        assert_eq!(receipt["acknowledgedEffectKeyCount"], 3);
        assert_eq!(receipt["recipeProviderCapability"], "effect_key_dedup_v1");
        let serialized = persisted.to_string();
        for forbidden in [
            "private-operation-id",
            "provider.invalid",
            "raw-route-id",
            "sensitive-pixels",
            "sensitive prompt",
            "sensitive-plaintext",
            "/sensitive/full/path",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "persisted forbidden sentinel: {forbidden}"
            );
        }
    }

    #[test]
    fn persisted_desktop_prompt_observation_uses_safe_receipt_projection() {
        let request = control_request_for_mode_test(json!({ "action": "desktop_prompt_observe" }));
        let response = json!({
            "success": true,
            "data": {
                "ok": true,
                "action": "desktop_prompt_observe",
                "context": { "contextId": "context-1", "displayName": ":99" },
                "frameReceipt": { "frameId": "frame-1", "sha256": "frame-digest" },
                "promptObservation": {
                    "observationId": "observation-1",
                    "detectionStatus": "matched",
                    "pageVisibility": "absent",
                    "classification": "browser_external",
                    "handlingOutcome": "actionable_observation",
                    "blindnessReceipt": {
                        "proofClass": "repository_fixture",
                        "claim": "absent_from_fixture_page_inputs"
                    },
                    "promptText": "sensitive prompt"
                },
                "visualizationBase64": "sensitive-pixels",
                "outputPath": "/sensitive/full/path"
            }
        });

        let persisted = service_job_persisted_result(&request, &response);

        assert_eq!(persisted["success"], json!(true));
        assert_eq!(persisted["data"]["action"], "desktop_prompt_observe");
        assert_eq!(
            persisted["data"]["promptObservation"]["classification"],
            "browser_external"
        );
        assert_eq!(persisted["data"]["visualizationPayload"], "response_only");
        let serialized = persisted.to_string();
        assert!(!serialized.contains("sensitive-pixels"));
        assert!(!serialized.contains("sensitive prompt"));
        assert!(!serialized.contains("displayName"));
        assert!(!serialized.contains("/sensitive/full/path"));
    }

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

    #[derive(Default)]
    struct InMemoryStatusRepository {
        state: Mutex<ServiceState>,
    }

    impl ServiceStateRepository for InMemoryStatusRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            self.state
                .lock()
                .map(|state| state.clone())
                .map_err(|_| "in-memory status repository lock was poisoned".to_string())
        }

        fn mutate<R>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "in-memory status repository lock was poisoned".to_string())?;
            mutator(&mut state)
        }
    }

    #[derive(Debug)]
    struct FixedStatusAuthorityPreparer;

    #[async_trait::async_trait]
    impl super::super::service_status_projection::ServiceStatusAuthorityPreparer
        for FixedStatusAuthorityPreparer
    {
        async fn prepare(&self, service_state: &mut ServiceState) {
            service_state.control_plane = Some(ControlPlaneSnapshot {
                worker_state: "Ready".to_string(),
                browser_health: "NotStarted".to_string(),
                queue_depth: 0,
                queue_capacity: DEFAULT_QUEUE_CAPACITY,
                waiting_profile_lease_job_count: 0,
                service_job_timeout_ms: None,
                service_monitor_interval_ms: None,
                updated_at: Some("2026-08-10T12:00:00.000Z".to_string()),
            });
        }
    }

    #[derive(Debug)]
    struct FixedBrowserAuthority;

    impl super::super::service_status_projection::ServiceStatusBrowserAuthorityProvider
        for FixedBrowserAuthority
    {
        fn snapshot(
            &self,
            _service_state: &ServiceState,
        ) -> super::super::browser_session_authority::BrowserSessionAuthoritySnapshot {
            super::super::browser_session_authority::BrowserSessionAuthoritySnapshot {
                schema_version: 1,
                ..Default::default()
            }
        }
    }

    #[derive(Debug)]
    struct FixedStatusClock;

    impl super::super::service_status_projection::ProjectionClock for FixedStatusClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            chrono::DateTime::parse_from_rfc3339("2026-08-10T12:00:05.000Z")
                .unwrap()
                .with_timezone(&chrono::Utc)
        }
    }

    fn fixed_launch_configuration() -> Value {
        json!({
            "defaultBrowserBuild": null,
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
            "executablePath": null,
            "executablePathSource": null,
            "executablePathExists": null,
            "browserBuildManifests": {},
            "profileSmoke": {
                "available": false,
                "command": "pnpm test:wsl-windows-chromium-profile-live",
                "reason": "stealthcdp_chromium_not_selected",
                "isWsl": false,
                "executableOnWindowsMount": false,
                "description": "fixed no-launch profile smoke"
            },
            "warnings": []
        })
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
        assert_eq!(
            job.naming_warnings,
            vec![
                "missing_service_name".to_string(),
                "missing_agent_name".to_string(),
                "missing_task_name".to_string()
            ]
        );
        assert!(job.has_naming_warning);
        assert_eq!(job.control_plane_mode, JobControlPlaneMode::Cdp);
        assert!(!job.lifecycle_only);
        assert!(job.submitted_at.is_some());
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_some());
        assert_eq!(job.result.as_ref().unwrap()["success"], true);

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn submit_persists_no_naming_warnings_for_named_request() {
        let home = temp_home("control-plane-submit-named");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let response = handle
            .submit(json!({
                "id": "test-named",
                "action": "state_list",
                "serviceName": "JournalDownloader",
                "agentName": "codex",
                "taskName": "probeACSwebsite",
            }))
            .await;

        assert_eq!(
            response.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        let persisted = store.load().unwrap();
        let job = &persisted.jobs["test-named"];
        assert!(job.naming_warnings.is_empty());
        assert!(!job.has_naming_warning);

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn service_job_control_plane_mode_marks_cdp_free_lifecycle_requests() {
        let cdp_free = control_request_for_mode_test(json!({
            "action": "cdp_free_launch",
            "requiresCdpFree": true
        }));
        assert_eq!(
            service_job_control_plane_mode(&cdp_free),
            JobControlPlaneMode::CdpFree
        );
        assert!(service_job_lifecycle_only(&cdp_free));

        let service = control_request_for_mode_test(json!({
            "action": "service_trace"
        }));
        assert_eq!(
            service_job_control_plane_mode(&service),
            JobControlPlaneMode::Service
        );
        assert!(service_job_lifecycle_only(&service));

        let view_takeover = control_request_for_mode_test(json!({
            "action": "view_takeover"
        }));
        assert_eq!(
            service_job_control_plane_mode(&view_takeover),
            JobControlPlaneMode::Service
        );
        assert!(service_job_lifecycle_only(&view_takeover));

        let desktop_capture = control_request_for_mode_test(json!({
            "action": "desktop_capture"
        }));
        assert_eq!(
            service_job_control_plane_mode(&desktop_capture),
            JobControlPlaneMode::Service
        );
        assert!(service_job_lifecycle_only(&desktop_capture));

        let desktop_locate = control_request_for_mode_test(json!({
            "action": "desktop_locate"
        }));
        assert_eq!(
            service_job_control_plane_mode(&desktop_locate),
            JobControlPlaneMode::Service
        );
        assert!(service_job_lifecycle_only(&desktop_locate));

        let desktop_interact = control_request_for_mode_test(json!({
            "action": "desktop_interact"
        }));
        assert_eq!(
            service_job_control_plane_mode(&desktop_interact),
            JobControlPlaneMode::Service
        );
        assert!(service_job_lifecycle_only(&desktop_interact));

        let desktop_prompt_observe = control_request_for_mode_test(json!({
            "action": "desktop_prompt_observe"
        }));
        assert_eq!(
            service_job_control_plane_mode(&desktop_prompt_observe),
            JobControlPlaneMode::Service
        );
        assert!(service_job_lifecycle_only(&desktop_prompt_observe));

        let cdp = control_request_for_mode_test(json!({
            "action": "navigate"
        }));
        assert_eq!(
            service_job_control_plane_mode(&cdp),
            JobControlPlaneMode::Cdp
        );
        assert!(!service_job_lifecycle_only(&cdp));
    }

    #[test]
    fn service_job_allocation_refs_preserve_requested_and_resolved_route_state() {
        let request = control_request_for_mode_test(json!({
            "action": "view_takeover",
            "params": {
                "displayAllocationId": "display-requested",
                "routeId": "route-requested",
                "viewerLeaseId": "viewer-requested",
                "controllerLeaseId": "controller-requested"
            }
        }));
        let queued_refs = service_job_allocation_refs(&request, None);

        assert_eq!(
            queued_refs.requested_display_allocation_id.as_deref(),
            Some("display-requested")
        );
        assert_eq!(
            queued_refs.display_allocation_id.as_deref(),
            Some("display-requested")
        );
        assert_eq!(
            queued_refs.requested_remote_view_route_id.as_deref(),
            Some("route-requested")
        );
        assert_eq!(
            queued_refs.remote_view_route_id.as_deref(),
            Some("route-requested")
        );
        assert_eq!(
            queued_refs.viewer_lease_id.as_deref(),
            Some("viewer-requested")
        );
        assert_eq!(
            queued_refs.controller_lease_id.as_deref(),
            Some("controller-requested")
        );

        let finished_refs = service_job_allocation_refs(
            &request,
            Some(&json!({
                "success": true,
                "data": {
                    "displayAllocationId": "display-resolved",
                    "remoteViewRouteId": "route-resolved",
                    "routePoolEntryId": "pool-1",
                    "viewerLeaseId": "viewer-resolved",
                    "controllerLeaseId": "controller-resolved"
                }
            })),
        );

        assert_eq!(
            finished_refs.requested_display_allocation_id.as_deref(),
            Some("display-requested")
        );
        assert_eq!(
            finished_refs.display_allocation_id.as_deref(),
            Some("display-resolved")
        );
        assert_eq!(
            finished_refs.requested_remote_view_route_id.as_deref(),
            Some("route-requested")
        );
        assert_eq!(
            finished_refs.remote_view_route_id.as_deref(),
            Some("route-resolved")
        );
        assert_eq!(finished_refs.route_pool_entry_id.as_deref(), Some("pool-1"));
        assert_eq!(
            finished_refs.viewer_lease_id.as_deref(),
            Some("viewer-resolved")
        );
        assert_eq!(
            finished_refs.controller_lease_id.as_deref(),
            Some("controller-resolved")
        );

        let desktop_request = control_request_for_mode_test(json!({
            "action": "desktop_capture",
            "browserId": "browser-1"
        }));
        let desktop_refs = service_job_allocation_refs(
            &desktop_request,
            Some(&json!({
                "success": true,
                "data": {
                    "context": {
                        "displayAllocationId": "display-captured",
                        "routeId": "route-captured"
                    }
                }
            })),
        );
        assert_eq!(
            desktop_refs.display_allocation_id.as_deref(),
            Some("display-captured")
        );
        assert_eq!(
            desktop_refs.remote_view_route_id.as_deref(),
            Some("route-captured")
        );

        let locate_request = control_request_for_mode_test(json!({
            "action": "desktop_locate",
            "browserId": "browser-1"
        }));
        let locate_refs = service_job_allocation_refs(
            &locate_request,
            Some(&json!({
                "success": true,
                "data": {
                    "context": {
                        "displayAllocationId": "display-located",
                        "routeId": "route-located"
                    }
                }
            })),
        );
        assert_eq!(
            locate_refs.display_allocation_id.as_deref(),
            Some("display-located")
        );
        assert_eq!(
            locate_refs.remote_view_route_id.as_deref(),
            Some("route-located")
        );
    }

    fn control_request_for_mode_test(command: Value) -> ControlRequest {
        let (response_tx, _response_rx) = oneshot::channel();
        ControlRequest {
            id: "mode-test".to_string(),
            job_id: "mode-test".to_string(),
            action: command
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            service_name: Some("test-service".to_string()),
            agent_name: Some("test-agent".to_string()),
            task_name: Some("test-task".to_string()),
            naming_warnings: Vec::new(),
            command,
            priority: ControlPriority::Normal,
            timeout_ms: None,
            cancellation: RunningJobCancel::new(),
            submitted_at_wall: current_timestamp(),
            submitted_at_mono: Instant::now(),
            profile_lease_wait_started_at: None,
            profile_lease_wait_profile_id: None,
            profile_lease_wait_conflict_session_ids: Vec::new(),
            profile_lease_wait_retry_after_ms: None,
            response_tx,
        }
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
        assert_eq!(
            response
                .pointer("/data/waiting_profile_lease_job_count")
                .and_then(|v| v.as_u64()),
            Some(0)
        );

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn requested_command_timings_include_queue_and_daemon_components() {
        let home = temp_home("control-plane-command-timings");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start(DaemonState::new());

        let response = handle
            .submit(json!({
                "id": "test-command-timings",
                "action": "state_list",
                "includeTimings": true,
            }))
            .await;

        assert_eq!(response["success"], true);
        for field in [
            "queueWaitMs",
            "commandPreparationMs",
            "actionExecutionMs",
            "responseSerializationMs",
            "daemonTotalMs",
            "totalMs",
        ] {
            assert!(response["timings"][field].is_u64(), "{field}");
        }
        assert!(
            response["timings"]["totalMs"].as_u64().unwrap()
                >= response["timings"]["daemonTotalMs"].as_u64().unwrap()
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
                    },
                    "jobs": {
                        "lease-wait": {
                            "id": "lease-wait",
                            "action": "navigate",
                            "state": "waiting_profile_lease",
                            "result": {
                                "profileId": "work",
                                "conflictSessionIds": ["holder"]
                            }
                        },
                        "queued": {
                            "id": "queued",
                            "action": "click",
                            "state": "queued"
                        }
                    },
                    "profiles": {
                        "work": {
                            "id": "work",
                            "name": "Work"
                        }
                    },
                    "browsers": {
                        "browser-work": {
                            "id": "browser-work",
                            "profileId": "work",
                            "host": "local_headed",
                            "health": "ready",
                            "pid": 2147483647u32,
                            "cdpEndpoint": "http://127.0.0.1:9222",
                            "activeSessionIds": ["holder"]
                        }
                    },
                    "sessions": {
                        "holder": {
                            "id": "holder",
                            "profileId": "work",
                            "lease": "exclusive"
                        }
                    }
                }),
                json!({
                    "defaultBrowserBuild": null,
                    "stealthCdpChromiumRequired": false,
                    "stealthCdpChromiumReady": true,
                    "executablePath": null,
                    "executablePathSource": null,
                    "executablePathExists": null,
                    "browserBuildManifests": {},
                    "profileSmoke": {
                        "available": false,
                        "command": "pnpm test:wsl-windows-chromium-profile-live",
                        "reason": "stealthcdp_chromium_not_selected",
                        "isWsl": false,
                        "executableOnWindowsMount": false,
                        "description": "Launches Windows chromium-stealthcdp from WSL with an isolated daemon socket and Windows-mounted profile, then verifies profile writes and Chrome stderr path hygiene."
                    },
                    "warnings": []
                }),
                false,
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
                .pointer("/data/closedTabProjection/mode")
                .and_then(|value| value.as_str()),
            Some("bounded")
        );
        assert_eq!(
            response
                .pointer("/data/control_plane/queue_capacity")
                .and_then(|v| v.as_u64()),
            Some(DEFAULT_QUEUE_CAPACITY as u64)
        );
        assert_eq!(
            response
                .pointer("/data/control_plane/waiting_profile_lease_job_count")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            response
                .pointer("/data/launchConfig/stealthCdpChromiumReady")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            response
                .pointer("/data/browserSessionAuthority/schemaVersion")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            response
                .pointer("/data/browserSessionAuthority/summary/modeledBrowserCount")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
        assert_eq!(
            response
                .pointer("/data/profileAllocations/0/profileId")
                .and_then(|v| v.as_str()),
            Some("work")
        );
        assert_eq!(
            response
                .pointer("/data/profileAllocations/0/recommendedAction")
                .and_then(|v| v.as_str()),
            Some("inspect_waiting_jobs")
        );
        assert!(response
            .pointer("/data/profileAllocations/0/browserSummaries")
            .and_then(|v| v.as_array())
            .is_some_and(|summaries| summaries.is_empty()));
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
                .pointer("/data/service_state/controlPlane/waitingProfileLeaseJobCount")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            response
                .pointer("/data/service_state/reconciliation/browserCount")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
        assert!(response["data"]["service_state"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |event| event.get("kind").and_then(|kind| kind.as_str()) == Some("reconciliation")
            ));
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
        assert!(persisted.events.iter().any(|event| {
            event.kind == crate::native::service_model::ServiceEventKind::Reconciliation
        }));

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn action_and_control_plane_status_both_surface_repository_failure() {
        let home = temp_home("service-status-repository-failure");
        std::fs::remove_dir(&home).unwrap();
        std::fs::write(&home, b"not a directory").unwrap();
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());

        let action_error = super::super::service_status_projection::handle_service_status(&json!({
            "serviceState": {},
            "launchConfig": {},
        }))
        .await
        .unwrap_err();
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let control_response = handle
            .service_status_response(
                "test-service-status-repository-failure",
                json!({}),
                json!({}),
                false,
            )
            .await;

        assert!(action_error.contains("Failed to create service state directory"));
        assert_eq!(control_response["success"], false);
        assert!(control_response["error"]
            .as_str()
            .unwrap()
            .contains("Failed to create service state directory"));

        handle.shutdown().await;
        std::fs::remove_file(&home).unwrap();
    }

    #[tokio::test]
    async fn fixed_input_harness_crosses_real_status_entries_and_transports() {
        let repository = InMemoryStatusRepository::default();
        let preparer = FixedStatusAuthorityPreparer;
        let browser_authority = FixedBrowserAuthority;
        let projector = super::super::service_status_projection::ServiceStatusProjector::new(
            Arc::new(
                super::super::service_status_projection::UnavailableStatusObservationAdapter::new(
                    "fixed no-launch observation",
                ),
            ),
            Arc::new(FixedStatusClock),
        );
        let command = json!({
            "id": "fixed-action-status",
            "action": "service_status",
            "serviceState": {},
            "launchConfig": fixed_launch_configuration(),
            "fullTabHistory": false,
        });
        let action =
            super::super::service_status_projection::handle_service_status_with_dependencies(
                &command,
                super::super::service_status_projection::ServiceStatusProjectionDependencies::new(
                    &repository,
                    &preparer,
                    &browser_authority,
                    &projector,
                ),
            )
            .await
            .unwrap();

        let (tx, _rx) = mpsc::channel(DEFAULT_QUEUE_CAPACITY);
        let status = Arc::new(ControlPlaneStatus::new());
        status.set_state(WorkerState::Ready);
        let handle = ControlPlaneHandle {
            tx,
            status,
            service_job_timeout_ms: None,
            service_monitor_interval_ms: None,
            running_cancellations: Arc::new(Mutex::new(HashMap::new())),
        };
        let control = handle
            .service_status_response_with_dependencies(
                "fixed-control-status",
                json!({}),
                fixed_launch_configuration(),
                false,
                super::super::service_status_projection::ServiceStatusProjectionDependencies::new(
                    &repository,
                    &preparer,
                    &browser_authority,
                    &projector,
                ),
            )
            .await;
        assert_eq!(control["success"], true);
        assert_eq!(control["data"], action);

        let control_text = control.to_string();
        let direct_body = super::super::stream::service_status_http_with_relay(
            "fixed-status-session",
            Some("full-tab-history=false"),
            |session, command| async move {
                assert_eq!(session, "fixed-status-session");
                assert_eq!(command["action"], "service_status");
                assert_eq!(command["fullTabHistory"], false);
                Ok(control_text)
            },
        )
        .await
        .unwrap();
        let direct_http = super::super::stream::service_status_http_fixture(direct_body);
        let dashboard_backend = super::super::stream::dashboard_service_status_with_transports(
            Some(9222),
            "/api/service/status?full-tab-history=false",
            |_port, _path| async move { Ok(direct_http) },
            |_path| async { None },
        )
        .await
        .unwrap();
        let dashboard_backend_body: Value = serde_json::from_slice(
            super::super::stream::service_status_http_body_fixture(&dashboard_backend).unwrap(),
        )
        .unwrap();
        assert_eq!(dashboard_backend_body["data"], action);

        let fallback_http = String::from_utf8(
            super::super::stream::service_status_dashboard_cli_fallback_fixture(action.to_string()),
        )
        .unwrap();
        let dashboard_fallback = super::super::stream::dashboard_service_status_with_transports(
            None,
            "/api/service/status?full-tab-history=false",
            |_port, _path| async { Ok(Vec::new()) },
            |_path| {
                let fallback_http = fallback_http.clone();
                async move { Some(fallback_http) }
            },
        )
        .await
        .unwrap();
        let dashboard_fallback_body: Value = serde_json::from_slice(
            super::super::stream::service_status_http_body_fixture(&dashboard_fallback).unwrap(),
        )
        .unwrap();
        assert_eq!(dashboard_fallback_body, action);
        assert_eq!(
            action["statusProjection"]["authority"]["projectedAt"],
            "2026-08-10T12:00:05.000Z"
        );
        assert_eq!(
            action["statusProjection"]["observations"]["state"],
            "unavailable"
        );

        if std::env::var("AGENT_BROWSER_EMIT_FIXED_STATUS_HARNESS").as_deref() == Ok("1") {
            println!("AGENT_BROWSER_FIXED_STATUS_DATA={action}");
        }
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
    fn cancel_service_job_in_repository_marks_queued_job_cancelled() {
        let home = temp_home("control-plane-cancel-repository");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
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

        let job =
            cancel_service_job_in_repository(&repository, "job-queued", Some("stale")).unwrap();

        assert_eq!(job.state, JobState::Cancelled);
        assert_eq!(job.error.as_deref(), Some("stale"));
        assert_eq!(job.result.as_ref().unwrap()["cancelled"], true);

        let persisted = store.load().unwrap();
        assert_eq!(persisted.jobs["job-queued"].state, JobState::Cancelled);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn cancel_service_job_in_repository_marks_profile_lease_wait_cancelled() {
        let home = temp_home("control-plane-cancel-profile-lease-wait");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
        store
            .save(&ServiceState {
                jobs: std::collections::BTreeMap::from([(
                    "job-waiting".to_string(),
                    ServiceJob {
                        id: "job-waiting".to_string(),
                        action: "navigate".to_string(),
                        state: JobState::WaitingProfileLease,
                        submitted_at: Some("2026-04-22T00:00:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let job =
            cancel_service_job_in_repository(&repository, "job-waiting", Some("stale")).unwrap();

        assert_eq!(job.state, JobState::Cancelled);
        assert_eq!(job.error.as_deref(), Some("stale"));
        assert_eq!(job.result.as_ref().unwrap()["cancelled"], true);

        let persisted = store.load().unwrap();
        assert_eq!(persisted.jobs["job-waiting"].state, JobState::Cancelled);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn service_job_repository_helpers_mutate_prune_and_load() {
        let home = temp_home("control-plane-job-repository");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());

        mutate_service_jobs_in_repository(&repository, |state| {
            for index in 0..=MAX_SERVICE_JOBS {
                let id = format!("job-{index:03}");
                state.jobs.insert(
                    id.clone(),
                    ServiceJob {
                        id,
                        action: "navigate".to_string(),
                        state: JobState::Queued,
                        submitted_at: Some(format!(
                            "2026-04-22T00:{:02}:{:02}Z",
                            index / 60,
                            index % 60
                        )),
                        ..ServiceJob::default()
                    },
                );
            }
        })
        .unwrap();

        let persisted = store.load().unwrap();
        assert_eq!(persisted.jobs.len(), MAX_SERVICE_JOBS);
        assert!(!persisted.jobs.contains_key("job-000"));
        assert!(persisted.jobs.contains_key("job-200"));

        let loaded = load_service_job_in_repository(&repository, "job-200").unwrap();
        assert_eq!(loaded.id, "job-200");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn reconciled_service_state_persists_through_repository() {
        let home = temp_home("control-plane-reconcile-repository");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
        let before = ServiceState {
            browsers: std::collections::BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("work-before".to_string()),
                    health: ServiceBrowserHealth::Ready,
                    active_session_ids: vec!["session-1".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let mut target = before.clone();
        target.browsers.get_mut("browser-1").unwrap().profile_id = Some("work-current".to_string());
        store.save(&target).unwrap();

        let mut reconciled = before.clone();
        reconciled.control_plane = Some(ControlPlaneSnapshot {
            worker_state: "ready".to_string(),
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            ..ControlPlaneSnapshot::default()
        });
        reconciled.browsers.insert(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("work-before".to_string()),
                health: ServiceBrowserHealth::Unreachable,
                last_error: Some("CDP endpoint is unreachable".to_string()),
                active_session_ids: vec!["session-1".to_string()],
                ..BrowserProcess::default()
            },
        );

        persist_reconciled_service_state_in_repository(&repository, &before, &reconciled).unwrap();

        let persisted = store.load().unwrap();
        let browser = &persisted.browsers["browser-1"];
        assert_eq!(browser.profile_id.as_deref(), Some("work-current"));
        assert_eq!(browser.health, ServiceBrowserHealth::Unreachable);
        assert_eq!(
            browser.last_error.as_deref(),
            Some("CDP endpoint is unreachable")
        );
        assert_eq!(
            persisted
                .control_plane
                .as_ref()
                .map(|snapshot| snapshot.queue_capacity),
            Some(DEFAULT_QUEUE_CAPACITY)
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn process_exited_browser_health_records_event_and_removes_operational_browser() {
        let home = temp_home("control-plane-process-exited-repository");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
        let browser_id = service_browser_id("session-1");
        store
            .save(&ServiceState {
                browsers: std::collections::BTreeMap::from([(
                    browser_id.clone(),
                    BrowserProcess {
                        id: browser_id.clone(),
                        profile_id: Some("work".to_string()),
                        host: ServiceBrowserHost::AttachedExisting,
                        health: ServiceBrowserHealth::Ready,
                        active_session_ids: vec!["session-1".to_string()],
                        ..BrowserProcess::default()
                    },
                )]),
                sessions: std::collections::BTreeMap::from([(
                    "session-1".to_string(),
                    crate::native::service_model::BrowserSession {
                        id: "session-1".to_string(),
                        browser_ids: vec![browser_id.clone()],
                        tab_ids: vec!["target:old".to_string()],
                        ..crate::native::service_model::BrowserSession::default()
                    },
                )]),
                tabs: std::collections::BTreeMap::from([(
                    "target:old".to_string(),
                    crate::native::service_model::BrowserTab {
                        id: "target:old".to_string(),
                        browser_id: browser_id.clone(),
                        session_id: Some("session-1".to_string()),
                        ..crate::native::service_model::BrowserTab::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();
        let mut state = DaemonState::new();
        state.session_id = "session-1".to_string();

        persist_process_exited_browser_health_in_repository(&repository, &state).unwrap();

        let persisted = store.load().unwrap();
        assert!(!persisted.browsers.contains_key(&browser_id));
        assert!(!persisted.sessions.contains_key("session-1"));
        assert!(!persisted.tabs.contains_key("target:old"));
        assert!(persisted
            .events
            .iter()
            .any(
                |event| event.browser_id.as_deref() == Some(browser_id.as_str())
                    && event.current_health == Some(ServiceBrowserHealth::ProcessExited)
            ));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn process_exited_browser_health_marks_display_allocation_orphaned() {
        let home = temp_home("control-plane-process-exited-display");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
        let browser_id = service_browser_id("session-1");
        let allocation_id = "display:private_virtual_display:session-session-1".to_string();
        store
            .save(&ServiceState {
                browsers: std::collections::BTreeMap::from([(
                    browser_id.clone(),
                    BrowserProcess {
                        id: browser_id.clone(),
                        profile_id: Some("work".to_string()),
                        host: ServiceBrowserHost::RemoteHeaded,
                        health: ServiceBrowserHealth::Ready,
                        display_isolation: Some("private_virtual_display".to_string()),
                        display_name: Some(":91".to_string()),
                        display_allocation_id: Some(allocation_id.clone()),
                        active_session_ids: vec!["session-1".to_string()],
                        ..BrowserProcess::default()
                    },
                )]),
                display_allocations: std::collections::BTreeMap::from([(
                    allocation_id.clone(),
                    DisplayAllocation {
                        id: allocation_id.clone(),
                        display_name: Some(":91".to_string()),
                        display_isolation: "private_virtual_display".to_string(),
                        owner_browser_id: Some(browser_id.clone()),
                        owner_session_id: Some("session-1".to_string()),
                        state: "ready".to_string(),
                        ..DisplayAllocation::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();
        let mut state = DaemonState::new();
        state.session_id = "session-1".to_string();

        persist_process_exited_browser_health_in_repository(&repository, &state).unwrap();

        let persisted = store.load().unwrap();
        assert!(!persisted.browsers.contains_key(&browser_id));
        let allocation = &persisted.display_allocations[&allocation_id];
        assert_eq!(allocation.state, "orphaned");
        assert_eq!(
            allocation.readiness.as_ref().unwrap()["reason"],
            "browser_process_exited"
        );
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
        assert!(!persisted.browsers.contains_key("browser-1"));
        assert!(persisted.events.iter().any(|event| {
            event.kind == crate::native::service_model::ServiceEventKind::BrowserHealthChanged
                && event.browser_id.as_deref() == Some("browser-1")
        }));
        assert!(
            persisted
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.browser_count)
                .unwrap_or_default()
                == 0
        );
        assert!(persisted
            .reconciliation
            .as_ref()
            .and_then(|snapshot| snapshot.last_reconciled_at.as_deref())
            .is_some());

        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn service_monitor_interval_enqueues_due_monitor_run() {
        let home = temp_home("control-plane-monitor-loop");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                monitors: std::collections::BTreeMap::from([(
                    "policy-heartbeat".to_string(),
                    SiteMonitor {
                        id: "policy-heartbeat".to_string(),
                        name: "Policy heartbeat".to_string(),
                        target: MonitorTarget::SitePolicy("google".to_string()),
                        state: MonitorState::Active,
                        last_checked_at: None,
                        ..SiteMonitor::default()
                    },
                )]),
                site_policies: std::collections::BTreeMap::from([(
                    "google".to_string(),
                    SitePolicy {
                        id: "google".to_string(),
                        ..SitePolicy::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let handle =
            ControlPlaneWorker::start_with_options(DaemonState::new(), None, Some(1_000), Some(25));
        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.shutdown().await;

        let persisted = store.load().unwrap();
        let monitor = &persisted.monitors["policy-heartbeat"];
        assert_eq!(
            monitor.last_result.as_deref(),
            Some("site_policy_available")
        );
        assert!(monitor.last_checked_at.is_some());
        assert!(persisted.jobs.values().any(|job| {
            job.action == SERVICE_MONITORS_RUN_DUE_ACTION && job.state == JobState::Succeeded
        }));
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
    async fn profile_lease_wait_requeues_without_blocking_worker() {
        let home = temp_home("control-plane-profile-lease-wait");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                sessions: std::collections::BTreeMap::from([(
                    "active-session".to_string(),
                    BrowserSession {
                        id: "active-session".to_string(),
                        profile_id: Some("acs-profile".to_string()),
                        lease: LeaseState::Exclusive,
                        ..BrowserSession::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let waiting_handle = handle.clone();
        let waiting = tokio::spawn(async move {
            waiting_handle
                .submit(json!({
                    "id": "lease-wait-job",
                    "action": "state_list",
                    "serviceName": "JournalDownloader",
                    "agentName": "unit-test",
                    "taskName": "profileLeaseWait",
                    "runtimeProfile": "acs-profile",
                    "profileLeasePolicy": "wait",
                    "profileLeaseWaitTimeoutMs": 2_000
                }))
                .await
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let quick_response = handle
            .submit(json!({
                "id": "quick-job",
                "action": "state_list",
                "serviceName": "JournalDownloader",
                "agentName": "unit-test",
                "taskName": "quickWhileLeaseWaiting"
            }))
            .await;
        assert_eq!(quick_response["success"], true);

        let waiting_snapshot = store.load().unwrap();
        let waiting_job = &waiting_snapshot.jobs["lease-wait-job"];
        assert_eq!(waiting_job.state, JobState::WaitingProfileLease);
        assert_eq!(
            waiting_job.result.as_ref().unwrap()["waitingProfileLease"],
            true
        );
        assert_eq!(
            waiting_job.result.as_ref().unwrap()["profileId"],
            "acs-profile"
        );
        assert_eq!(
            waiting_job.result.as_ref().unwrap()["conflictSessionIds"],
            json!(["active-session"])
        );
        assert!(waiting_job.started_at.is_none());
        assert!(waiting_snapshot.events.iter().any(|event| {
            event.kind == ServiceEventKind::ProfileLeaseWaitStarted
                && event.profile_id.as_deref() == Some("acs-profile")
                && event.service_name.as_deref() == Some("JournalDownloader")
                && event.agent_name.as_deref() == Some("unit-test")
                && event.task_name.as_deref() == Some("profileLeaseWait")
                && event.details.as_ref().unwrap()["jobId"] == "lease-wait-job"
                && event.details.as_ref().unwrap()["outcome"] == "started"
                && event.details.as_ref().unwrap()["conflictSessionIds"]
                    == json!(["active-session"])
        }));

        let mut released = store.load().unwrap();
        released.sessions.get_mut("active-session").unwrap().lease = LeaseState::Released;
        store.save(&released).unwrap();

        let waiting_response = waiting.await.unwrap();
        assert_eq!(waiting_response["success"], true);
        handle.shutdown().await;

        let persisted = store.load().unwrap();
        assert_eq!(persisted.jobs["quick-job"].state, JobState::Succeeded);
        assert_eq!(persisted.jobs["lease-wait-job"].state, JobState::Succeeded);
        assert!(persisted.jobs["lease-wait-job"].started_at.is_some());
        assert!(persisted.events.iter().any(|event| {
            event.kind == ServiceEventKind::ProfileLeaseWaitEnded
                && event.profile_id.as_deref() == Some("acs-profile")
                && event.details.as_ref().unwrap()["jobId"] == "lease-wait-job"
                && event.details.as_ref().unwrap()["outcome"] == "ready"
                && event.details.as_ref().unwrap()["waitedMs"]
                    .as_u64()
                    .is_some()
        }));

        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn profile_lease_wait_cancel_records_cancelled_wait_end() {
        let home = temp_home("control-plane-profile-lease-wait-cancel");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                sessions: std::collections::BTreeMap::from([(
                    "active-session".to_string(),
                    BrowserSession {
                        id: "active-session".to_string(),
                        profile_id: Some("acs-profile".to_string()),
                        lease: LeaseState::Exclusive,
                        ..BrowserSession::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let waiting_handle = handle.clone();
        let waiting = tokio::spawn(async move {
            waiting_handle
                .submit(json!({
                    "id": "lease-wait-cancel-job",
                    "action": "state_list",
                    "serviceName": "JournalDownloader",
                    "agentName": "unit-test",
                    "taskName": "profileLeaseWaitCancel",
                    "runtimeProfile": "acs-profile",
                    "profileLeasePolicy": "wait",
                    "profileLeaseWaitTimeoutMs": 2_000
                }))
                .await
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let cancel_response = handle.cancel_job_response(
            "cancel-lease-wait",
            "lease-wait-cancel-job",
            Some("operator cancelled waiting job"),
        );
        assert_eq!(cancel_response["success"], true);
        assert_eq!(cancel_response["data"]["cancelled"], true);

        let waiting_response = waiting.await.unwrap();
        assert_eq!(waiting_response["success"], false);
        assert_eq!(
            waiting_response["error"],
            "Service job was cancelled before dispatch"
        );
        handle.shutdown().await;

        let persisted = store.load().unwrap();
        let waiting_job = &persisted.jobs["lease-wait-cancel-job"];
        assert_eq!(waiting_job.state, JobState::Cancelled);
        assert_eq!(
            waiting_job.error.as_deref(),
            Some("operator cancelled waiting job")
        );
        assert!(persisted.events.iter().any(|event| {
            event.kind == ServiceEventKind::ProfileLeaseWaitStarted
                && event.profile_id.as_deref() == Some("acs-profile")
                && event.task_name.as_deref() == Some("profileLeaseWaitCancel")
                && event.details.as_ref().unwrap()["jobId"] == "lease-wait-cancel-job"
        }));
        assert!(persisted.events.iter().any(|event| {
            event.kind == ServiceEventKind::ProfileLeaseWaitEnded
                && event.profile_id.as_deref() == Some("acs-profile")
                && event.task_name.as_deref() == Some("profileLeaseWaitCancel")
                && event.details.as_ref().unwrap()["jobId"] == "lease-wait-cancel-job"
                && event.details.as_ref().unwrap()["outcome"] == "cancelled"
                && event.details.as_ref().unwrap()["error"]
                    == "Service job was cancelled before dispatch"
                && event.details.as_ref().unwrap()["waitedMs"]
                    .as_u64()
                    .is_some()
        }));

        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn service_job_timeout_marks_running_job_timed_out() {
        let home = temp_home("control-plane-job-timeout");
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        let handle = ControlPlaneWorker::start(DaemonState::new());
        let response = handle
            .submit(json!({
                "id": "test-timeout",
                "action": "__test_sleep",
                "serviceName": "JournalDownloader",
                "agentName": "article-probe-agent",
                "taskName": "probeACSwebsite",
                "ms": 100,
                "jobTimeoutMs": 10,
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

        let next_started = Instant::now();
        let next = handle
            .submit(json!({
                "id": "test-after-timeout",
                "action": "state_list",
            }))
            .await;
        assert_eq!(next.get("success").and_then(Value::as_bool), Some(true));
        assert!(next_started.elapsed() < Duration::from_millis(100));

        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn worker_response_is_delivered_before_follow_up_health_probe_finishes() {
        let (response_tx, mut response_rx) = oneshot::channel();
        let delivery = send_response_before_follow_up(
            response_tx,
            json!({ "success": false, "data": { "timedOut": true } }),
            std::future::pending::<()>(),
        );
        tokio::pin!(delivery);

        let response = tokio::time::timeout(Duration::from_millis(50), async {
            tokio::select! {
                response = &mut response_rx => response.expect("response sender should remain live"),
                _ = &mut delivery => panic!("follow-up probe should remain pending"),
            }
        })
        .await
        .expect("worker response should not wait for the follow-up probe");

        assert_eq!(response.pointer("/data/timedOut"), Some(&Value::Bool(true)));
    }

    #[test]
    fn interrupted_job_cleanup_requires_observed_process_exit() {
        assert!(!browser_health_requires_cleanup_after_interruption(
            BrowserHealth::Ready
        ));
        assert!(!browser_health_requires_cleanup_after_interruption(
            BrowserHealth::CdpDisconnected
        ));
        assert!(browser_health_requires_cleanup_after_interruption(
            BrowserHealth::ProcessExited
        ));
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
                "params": {
                    "displayIsolation": "private_virtual_display"
                },
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
        assert_eq!(
            job.display_isolation.as_deref(),
            Some("private_virtual_display")
        );
        assert_eq!(job.error.as_deref(), Some("Control queue is full"));

        drop(_permit);
        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn queued_command_carries_its_stable_service_job_id() {
        let command =
            command_with_service_job_id(json!({"action": "remote_view_open"}), "job-handoff-a");

        assert_eq!(command["serviceJobId"], "job-handoff-a");
    }

    #[test]
    fn default_job_timeout_is_materialized_for_the_route_supervisor() {
        let (command, timeout_ms) =
            command_with_effective_job_timeout(json!({"action": "remote_view_open"}), Some(12_345));
        assert_eq!(timeout_ms, Some(12_345));
        assert_eq!(command["jobTimeoutMs"], 12_345);

        let (explicit, timeout_ms) = command_with_effective_job_timeout(
            json!({"action": "remote_view_open", "jobTimeoutMs": 4_321}),
            Some(12_345),
        );
        assert_eq!(timeout_ms, Some(4_321));
        assert_eq!(explicit["jobTimeoutMs"], 4_321);
    }

    #[tokio::test]
    async fn coordinated_timeout_drops_unfinished_execution_at_total_deadline() {
        let cancellation = RunningJobCancel::new();
        let observed = cancellation.clone();
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let execution_completed = completed.clone();
        let execution = async move {
            observed.cancelled().await;
            tokio::time::sleep(Duration::from_secs(60)).await;
            execution_completed.store(true, Ordering::SeqCst);
            json!({"terminalState": "rolled_back"})
        };

        let outcome = await_coordinated_execution(execution, cancellation, Some(1)).await;
        match outcome {
            CoordinatedExecution::TimedOutAtDeadline { timeout_ms } => {
                assert_eq!(timeout_ms, 1);
                assert!(!completed.load(Ordering::SeqCst));
            }
            _ => panic!("timeout must release the coordinator future at the total deadline"),
        }
    }

    #[tokio::test]
    async fn coordinated_cancellation_awaits_terminal_compensation() {
        let cancellation = RunningJobCancel::new();
        let observed = cancellation.clone();
        let trigger = cancellation.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            trigger.cancel();
        });
        let execution = async move {
            observed.cancelled().await;
            tokio::task::yield_now().await;
            json!({"terminalState": "rollback_incomplete"})
        };

        let outcome = await_coordinated_execution(execution, cancellation, Some(1_000)).await;
        match outcome {
            CoordinatedExecution::CancelledAfterCompensation(response) => {
                assert_eq!(response["terminalState"], "rollback_incomplete");
            }
            _ => panic!("cancellation must retain the coordinator future through compensation"),
        }
    }
}
