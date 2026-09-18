//! Provider-free service job and event records.
//!
//! Persistence, ServiceState joins, observation, transport, and runtime
//! effects remain in CLI Adapters. This module owns durable record shape.

use serde::{Deserialize, Serialize};

use crate::{
    BrowserHealth, ServiceActor, ServiceFailureRecourse, ServiceRequestProvenance,
    ServiceTerminalOutcome,
};

pub const SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME: &str = "missing_service_name";
pub const SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME: &str = "missing_agent_name";
pub const SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME: &str = "missing_task_name";
pub const SERVICE_JOB_NAMING_WARNING_VALUES: [&str; 3] = [
    SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME,
];
pub const SERVICE_EVENT_KIND_VALUES: [&str; 20] = [
    "reconciliation",
    "browser_launch_recorded",
    "browser_health_changed",
    "browser_recovery_started",
    "browser_recovery_override",
    "tab_lifecycle_changed",
    "profile_lease_wait_started",
    "profile_lease_wait_ended",
    "profile_lease_lifecycle_changed",
    "viewer_takeover_requested",
    "viewer_connected",
    "viewer_disconnected",
    "controller_requested",
    "controller_granted",
    "controller_denied",
    "route_released",
    "reconciliation_error",
    "incident_acknowledged",
    "incident_resolved",
    "job_terminal",
];
pub const SERVICE_JOB_STATE_VALUES: [&str; 7] = [
    "queued",
    "waiting_profile_lease",
    "running",
    "succeeded",
    "failed",
    "cancelled",
    "timed_out",
];
pub const SERVICE_JOB_PRIORITY_VALUES: [&str; 3] = ["low", "normal", "lifecycle"];
pub const SERVICE_JOB_CONTROL_PLANE_MODE_VALUES: [&str; 3] = ["cdp", "cdp_free", "service"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceEvent {
    pub id: String,
    pub timestamp: String,
    pub kind: ServiceEventKind,
    pub message: String,
    pub browser_id: Option<String>,
    pub profile_id: Option<String>,
    pub session_id: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub provenance: Option<ServiceRequestProvenance>,
    pub terminal_outcome: Option<ServiceTerminalOutcome>,
    pub previous_health: Option<BrowserHealth>,
    pub current_health: Option<BrowserHealth>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEventKind {
    #[default]
    Reconciliation,
    BrowserLaunchRecorded,
    BrowserHealthChanged,
    BrowserRecoveryStarted,
    BrowserRecoveryOverride,
    TabLifecycleChanged,
    ProfileLeaseWaitStarted,
    ProfileLeaseWaitEnded,
    ProfileLeaseLifecycleChanged,
    ViewerTakeoverRequested,
    ViewerConnected,
    ViewerDisconnected,
    ControllerRequested,
    ControllerGranted,
    ControllerDenied,
    RouteReleased,
    ReconciliationError,
    IncidentAcknowledged,
    IncidentResolved,
    JobTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceJob {
    pub id: String,
    pub action: String,
    pub provenance: ServiceRequestProvenance,
    pub terminal_outcome: Option<ServiceTerminalOutcome>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub target_service_id: Option<String>,
    pub site_id: Option<String>,
    pub login_id: Option<String>,
    pub target_service_ids: Vec<String>,
    pub naming_warnings: Vec<String>,
    pub has_naming_warning: bool,
    pub control_plane_mode: JobControlPlaneMode,
    pub lifecycle_only: bool,
    pub display_isolation: Option<String>,
    pub requested_display_allocation_id: Option<String>,
    pub display_allocation_id: Option<String>,
    pub requested_remote_view_route_id: Option<String>,
    pub remote_view_route_id: Option<String>,
    pub route_pool_entry_id: Option<String>,
    pub viewer_lease_id: Option<String>,
    pub controller_lease_id: Option<String>,
    pub target: JobTarget,
    pub owner: ServiceActor,
    pub state: JobState,
    pub priority: JobPriority,
    pub submitted_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub timeout_ms: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<ServiceFailureRecourse>,
}

impl Default for ServiceJob {
    fn default() -> Self {
        Self {
            id: String::new(),
            action: String::new(),
            provenance: ServiceRequestProvenance::default(),
            terminal_outcome: None,
            service_name: None,
            agent_name: None,
            task_name: None,
            target_service_id: None,
            site_id: None,
            login_id: None,
            target_service_ids: Vec::new(),
            naming_warnings: Vec::new(),
            has_naming_warning: false,
            control_plane_mode: JobControlPlaneMode::Cdp,
            lifecycle_only: false,
            display_isolation: None,
            requested_display_allocation_id: None,
            display_allocation_id: None,
            requested_remote_view_route_id: None,
            remote_view_route_id: None,
            route_pool_entry_id: None,
            viewer_lease_id: None,
            controller_lease_id: None,
            target: JobTarget::Service,
            owner: ServiceActor::System,
            state: JobState::Queued,
            priority: JobPriority::Normal,
            submitted_at: None,
            started_at: None,
            completed_at: None,
            timeout_ms: None,
            result: None,
            error: None,
            failure: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobTarget {
    Service,
    Browser(String),
    Tab(String),
    Profile(String),
    Monitor(String),
    Challenge(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    WaitingProfileLease,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    Low,
    Normal,
    Lifecycle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobControlPlaneMode {
    #[default]
    Cdp,
    CdpFree,
    Service,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn service_job_default_preserves_queue_contract() {
        let value = serde_json::to_value(ServiceJob::default()).unwrap();
        assert_eq!(value["target"], "service");
        assert_eq!(value["owner"], "system");
        assert_eq!(value["state"], "queued");
        assert_eq!(value["priority"], "normal");
        assert_eq!(value["controlPlaneMode"], "cdp");
        assert_eq!(value["namingWarnings"], json!([]));
        assert_eq!(value["hasNamingWarning"], false);
        assert!(value.get("failure").is_none());
    }

    #[test]
    fn service_job_and_event_use_stable_wire_names() {
        let job = ServiceJob {
            id: "job-1".into(),
            action: "navigate".into(),
            naming_warnings: SERVICE_JOB_NAMING_WARNING_VALUES
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            has_naming_warning: true,
            target: JobTarget::Browser("browser-1".into()),
            state: JobState::TimedOut,
            priority: JobPriority::Lifecycle,
            control_plane_mode: JobControlPlaneMode::CdpFree,
            ..ServiceJob::default()
        };
        let event = ServiceEvent {
            id: "event-1".into(),
            timestamp: "2026-09-02T12:00:00Z".into(),
            kind: ServiceEventKind::BrowserHealthChanged,
            message: "Browser changed".into(),
            ..ServiceEvent::default()
        };
        let job_value = serde_json::to_value(job).unwrap();
        let event_value = serde_json::to_value(event).unwrap();
        assert_eq!(job_value["targetServiceIds"], json!([]));
        assert_eq!(job_value["namingWarnings"][0], "missing_service_name");
        assert_eq!(job_value["hasNamingWarning"], true);
        assert_eq!(job_value["controlPlaneMode"], "cdp_free");
        assert_eq!(job_value["target"], json!({ "browser": "browser-1" }));
        assert_eq!(event_value["kind"], "browser_health_changed");
        assert_eq!(event_value["previousHealth"], Value::Null);
    }
}
