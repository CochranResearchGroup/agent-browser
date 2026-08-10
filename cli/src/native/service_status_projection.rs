//! Canonical Service Status response projection.
//!
//! Reconciled Service State and Browser Session Authority cross this module as
//! typed authority. Host-local runtime facts cross a substitutable observation
//! adapter and carry availability and freshness without becoming persisted
//! browser, route, proof, inventory, or actionability truth.

mod authority;
mod compatibility;
mod local_observation;
mod observation;

use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::browser_session_authority::BrowserSessionAuthoritySnapshot;
use super::service_model::{ControlPlaneSnapshot, ServiceProfileAllocation, ServiceState};

#[cfg(test)]
pub(crate) use local_observation::InMemoryStatusObservationAdapter;
pub(crate) use local_observation::{
    LocalStatusObservationAdapter, UnavailableStatusObservationAdapter,
};
#[cfg(test)]
pub(crate) use observation::{
    StatusObservationComponentState, StatusObservationError, StatusObservationErrorCode,
    StatusObservationSourceKind, StatusObservationState, StatusRoutePresentationSource,
    StatusViewStreamObservationState,
};
pub(crate) use observation::{
    StatusObservationRequest, StatusObservationSnapshot, StatusObservationSource,
};

pub const ORDINARY_CLOSED_TAB_CAP: usize = 50;

/// Compatibility projection used by the Chrome diagnostic surface. Canonical
/// Service Status reads obtain the same raw discovery through the observation
/// adapter before this authority join is applied.
pub fn manual_runtime_browser_projection(
    state: &ServiceState,
) -> Vec<crate::runtime_profile::ManualRuntimeBrowser> {
    compatibility::join_manual_browsers(
        state,
        crate::runtime_profile::list_manual_runtime_browsers().unwrap_or_default(),
    )
}

#[derive(Debug, Clone)]
pub(crate) struct StatusAuthorityInput {
    pub(crate) service_state: ServiceState,
    pub(crate) control_plane: StatusControlPlaneAuthority,
    pub(crate) browser_session_authority: BrowserSessionAuthoritySnapshot,
    pub(crate) launch_config: StatusLaunchConfiguration,
    pub(crate) full_tab_history: bool,
}

pub(crate) struct ServiceStatusProjectionDependencies<'a, Repository, Preparer, BrowserAuthority> {
    pub(crate) repository: &'a Repository,
    pub(crate) preparer: &'a Preparer,
    pub(crate) browser_authority: &'a BrowserAuthority,
    pub(crate) projector: &'a ServiceStatusProjector,
}

impl<'a, Repository, Preparer, BrowserAuthority>
    ServiceStatusProjectionDependencies<'a, Repository, Preparer, BrowserAuthority>
{
    pub(crate) fn new(
        repository: &'a Repository,
        preparer: &'a Preparer,
        browser_authority: &'a BrowserAuthority,
        projector: &'a ServiceStatusProjector,
    ) -> Self {
        Self {
            repository,
            preparer,
            browser_authority,
            projector,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StatusWorkerState {
    #[serde(alias = "starting")]
    Starting,
    #[serde(alias = "ready")]
    Ready,
    #[serde(alias = "busy")]
    Busy,
    #[serde(alias = "draining")]
    Draining,
    #[serde(alias = "closing")]
    Closing,
    #[serde(alias = "stopped")]
    Stopped,
    #[serde(alias = "faulted")]
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StatusBrowserHealth {
    #[serde(alias = "not_started")]
    NotStarted,
    #[serde(alias = "launching")]
    Launching,
    #[serde(alias = "ready")]
    Ready,
    #[serde(alias = "unreachable")]
    Unreachable,
    #[serde(alias = "process_exited")]
    ProcessExited,
    #[serde(alias = "cdp_disconnected")]
    CdpDisconnected,
    #[serde(alias = "closing")]
    Closing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StatusControlPlaneAuthority {
    pub(crate) worker_state: StatusWorkerState,
    pub(crate) browser_health: StatusBrowserHealth,
    pub(crate) queue_depth: usize,
    pub(crate) queue_capacity: usize,
    pub(crate) waiting_profile_lease_job_count: usize,
    pub(crate) service_job_timeout_ms: Option<u64>,
    pub(crate) service_monitor_interval_ms: Option<u64>,
}

impl TryFrom<&ControlPlaneSnapshot> for StatusControlPlaneAuthority {
    type Error = ServiceStatusProjectionError;

    fn try_from(snapshot: &ControlPlaneSnapshot) -> Result<Self, Self::Error> {
        let authority = serde_json::from_value(serde_json::json!({
            "worker_state": snapshot.worker_state,
            "browser_health": snapshot.browser_health,
            "queue_depth": snapshot.queue_depth,
            "queue_capacity": snapshot.queue_capacity,
            "waiting_profile_lease_job_count": snapshot.waiting_profile_lease_job_count,
            "service_job_timeout_ms": snapshot.service_job_timeout_ms,
            "service_monitor_interval_ms": snapshot.service_monitor_interval_ms,
        }))
        .map_err(|error| {
            ServiceStatusProjectionError::InvalidAuthority(format!(
                "invalid control-plane snapshot: {error}"
            ))
        })?;
        validate_control_plane_authority(authority)
    }
}

impl TryFrom<Value> for StatusControlPlaneAuthority {
    type Error = ServiceStatusProjectionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let authority = serde_json::from_value(value).map_err(|error| {
            ServiceStatusProjectionError::InvalidAuthority(format!(
                "invalid control-plane snapshot: {error}"
            ))
        })?;
        validate_control_plane_authority(authority)
    }
}

fn validate_control_plane_authority(
    authority: StatusControlPlaneAuthority,
) -> Result<StatusControlPlaneAuthority, ServiceStatusProjectionError> {
    if authority.queue_depth > authority.queue_capacity {
        return Err(ServiceStatusProjectionError::InvalidAuthority(
            "control-plane queueDepth exceeds queueCapacity".to_string(),
        ));
    }
    Ok(authority)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusLaunchConfiguration {
    #[serde(deserialize_with = "deserialize_required_nullable")]
    default_browser_build: Option<String>,
    stealth_cdp_chromium_required: bool,
    stealth_cdp_chromium_ready: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    executable_path: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    executable_path_source: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    executable_path_exists: Option<bool>,
    browser_build_manifests: Map<String, Value>,
    profile_smoke: StatusLaunchProfileSmoke,
    warnings: Vec<StatusLaunchWarning>,
    #[serde(flatten)]
    additional_properties: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusLaunchProfileSmoke {
    available: bool,
    command: String,
    reason: String,
    is_wsl: bool,
    executable_on_windows_mount: bool,
    description: String,
    #[serde(flatten)]
    additional_properties: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StatusLaunchWarning {
    code: String,
    severity: String,
    message: String,
    #[serde(flatten)]
    additional_properties: Map<String, Value>,
}

fn deserialize_required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl TryFrom<Value> for StatusLaunchConfiguration {
    type Error = ServiceStatusProjectionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Object(value) = value else {
            return Err(ServiceStatusProjectionError::InvalidAuthority(
                "launchConfig must be an object".to_string(),
            ));
        };
        serde_json::from_value(Value::Object(value)).map_err(|error| {
            ServiceStatusProjectionError::InvalidAuthority(format!("invalid launchConfig: {error}"))
        })
    }
}

impl StatusLaunchConfiguration {
    fn legacy_ingress_default() -> Self {
        Self {
            default_browser_build: None,
            stealth_cdp_chromium_required: false,
            stealth_cdp_chromium_ready: true,
            executable_path: None,
            executable_path_source: None,
            executable_path_exists: None,
            browser_build_manifests: Map::new(),
            profile_smoke: StatusLaunchProfileSmoke {
                available: false,
                command: "pnpm test:wsl-windows-chromium-profile-live".to_string(),
                reason: "stealthcdp_chromium_not_selected".to_string(),
                is_wsl: false,
                executable_on_windows_mount: false,
                description: "Launches Windows chromium-stealthcdp from WSL with an isolated daemon socket and Windows-mounted profile, then verifies profile writes and Chrome stderr path hygiene.".to_string(),
                additional_properties: Map::new(),
            },
            warnings: Vec::new(),
            additional_properties: Map::new(),
        }
    }
}

pub(crate) fn launch_configuration_from_status_command(command: &Value) -> Value {
    command.get("launchConfig").cloned().unwrap_or_else(|| {
        serde_json::to_value(StatusLaunchConfiguration::legacy_ingress_default())
            .expect("typed legacy launch configuration always serializes")
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedTabProjectionMetadata {
    pub mode: &'static str,
    pub cap: Option<usize>,
    pub total_closed_count: usize,
    pub retained_closed_count: usize,
    pub omitted_closed_count: usize,
    pub ordering: &'static str,
    pub diagnostic_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ServiceStatusResponse {
    pub(crate) control_plane: StatusControlPlaneAuthority,
    pub(crate) service_state: Value,
    #[serde(rename = "profileAllocations")]
    pub(crate) profile_allocations: Vec<ServiceProfileAllocation>,
    #[serde(rename = "manualBrowsers")]
    pub(crate) manual_browsers: Vec<crate::runtime_profile::ManualRuntimeBrowser>,
    #[serde(rename = "retainedDisplayAllocations")]
    pub(crate) retained_display_allocations: Value,
    #[serde(rename = "browserSessionAuthority")]
    pub(crate) browser_session_authority: BrowserSessionAuthoritySnapshot,
    #[serde(rename = "closedTabProjection")]
    pub(crate) closed_tab_projection: ClosedTabProjectionMetadata,
    #[serde(rename = "launchConfig")]
    pub(crate) launch_config: StatusLaunchConfiguration,
    #[serde(rename = "statusProjection")]
    pub(crate) status_projection: StatusProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusProjection {
    pub(crate) schema_version: u8,
    pub(crate) authority: StatusProjectionAuthority,
    pub(crate) observations: StatusObservationSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusProjectionAuthority {
    pub(crate) source: &'static str,
    pub(crate) projected_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServiceStatusProjectionError {
    InvalidAuthority(String),
    InvalidObservation(String),
    Serialization(String),
}

impl std::fmt::Display for ServiceStatusProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAuthority(message) => {
                write!(formatter, "invalid status authority: {message}")
            }
            Self::InvalidObservation(message) => {
                write!(formatter, "invalid status observation: {message}")
            }
            Self::Serialization(message) => {
                write!(formatter, "status serialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for ServiceStatusProjectionError {}

pub(crate) trait ProjectionClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[async_trait::async_trait]
pub(crate) trait ServiceStatusAuthorityPreparer: Send + Sync {
    async fn prepare(&self, service_state: &mut ServiceState);
}

#[derive(Debug, Default)]
pub(crate) struct ReconcileServiceStatusAuthority;

#[async_trait::async_trait]
impl ServiceStatusAuthorityPreparer for ReconcileServiceStatusAuthority {
    async fn prepare(&self, service_state: &mut ServiceState) {
        super::service_health::reconcile_service_state(service_state).await;
    }
}

pub(crate) trait ServiceStatusBrowserAuthorityProvider: Send + Sync {
    fn snapshot(&self, service_state: &ServiceState) -> BrowserSessionAuthoritySnapshot;
}

#[derive(Debug, Default)]
pub(crate) struct ReconciledBrowserSessionAuthority;

impl ServiceStatusBrowserAuthorityProvider for ReconciledBrowserSessionAuthority {
    fn snapshot(&self, service_state: &ServiceState) -> BrowserSessionAuthoritySnapshot {
        super::browser_session_authority::browser_session_authority_snapshot(service_state)
    }
}

#[derive(Debug, Default)]
struct SystemProjectionClock;

impl ProjectionClock for SystemProjectionClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub(crate) struct ServiceStatusProjector {
    observations: Arc<dyn StatusObservationSource>,
    clock: Arc<dyn ProjectionClock>,
}

impl ServiceStatusProjector {
    pub(crate) fn local() -> Self {
        Self::new(
            Arc::new(LocalStatusObservationAdapter),
            Arc::new(SystemProjectionClock),
        )
    }

    pub(crate) fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(
            Arc::new(UnavailableStatusObservationAdapter::new(reason)),
            Arc::new(SystemProjectionClock),
        )
    }

    pub(crate) fn new(
        observations: Arc<dyn StatusObservationSource>,
        clock: Arc<dyn ProjectionClock>,
    ) -> Self {
        Self {
            observations,
            clock,
        }
    }

    /// Projects one complete v1 Service Status response without mutating or
    /// persisting the supplied reconciled authority snapshot.
    pub(crate) async fn project(
        &self,
        input: StatusAuthorityInput,
    ) -> Result<ServiceStatusResponse, ServiceStatusProjectionError> {
        authority::validate_authority(&input.service_state)?;
        input.browser_session_authority.validate()?;

        let mut authority_state = input.service_state.clone();
        super::action_runtime::refresh_cdp_screencast_view_streams(&mut authority_state);
        super::remote_view_attachability::refresh_remote_view_attachability(&mut authority_state);
        authority_state.refresh_profile_readiness();

        let request = StatusObservationRequest::from_state(&authority_state);
        let observations = self.observations.snapshot(request).await;
        observations
            .validate()
            .map_err(ServiceStatusProjectionError::InvalidObservation)?;
        let projected_at = format_timestamp(self.clock.now());

        let manual_browsers = compatibility::join_manual_browsers(
            &authority_state,
            observations.manual_browsers.clone(),
        );
        let (response_state, closed_tab_projection) =
            authority::project_closed_tabs(&authority_state, input.full_tab_history);
        let response_state =
            compatibility::apply_legacy_observation_mirrors(&response_state, &observations)?;

        let response = ServiceStatusResponse {
            control_plane: input.control_plane,
            profile_allocations: super::service_model::service_profile_allocations(
                &authority_state,
            ),
            manual_browsers,
            retained_display_allocations: super::service_model::retained_display_allocation_summary(
                &authority_state,
            ),
            browser_session_authority: input.browser_session_authority,
            closed_tab_projection,
            launch_config: input.launch_config,
            service_state: response_state,
            status_projection: StatusProjection {
                schema_version: 1,
                authority: StatusProjectionAuthority {
                    source: "reconciled_service_state",
                    projected_at,
                },
                observations,
            },
        };
        serde_json::to_value(&response)
            .map_err(|error| ServiceStatusProjectionError::Serialization(error.to_string()))?;
        Ok(response)
    }
}

pub(crate) async fn project_status_with_launch_configuration(
    projector: &ServiceStatusProjector,
    service_state: ServiceState,
    control_plane: StatusControlPlaneAuthority,
    browser_session_authority: BrowserSessionAuthoritySnapshot,
    launch_config: Value,
    full_tab_history: bool,
) -> Result<ServiceStatusResponse, ServiceStatusProjectionError> {
    let launch_config = StatusLaunchConfiguration::try_from(launch_config)?;
    projector
        .project(StatusAuthorityInput {
            service_state,
            control_plane,
            browser_session_authority,
            launch_config,
            full_tab_history,
        })
        .await
}

fn format_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests;
