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
    pub(crate) runtime_lifecycle: Value,
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
    #[serde(
        rename = "presentationCapacity",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) presentation_capacity:
        Option<super::presentation_capacity::PresentationCapacityProjection>,
    #[serde(rename = "desktopEvidencePolicy")]
    pub(crate) desktop_evidence_policy: super::desktop_evidence::DesktopEvidencePolicyProjection,
    #[serde(rename = "browserSessionAuthority")]
    pub(crate) browser_session_authority: BrowserSessionAuthoritySnapshot,
    #[serde(rename = "closedTabProjection")]
    pub(crate) closed_tab_projection: ClosedTabProjectionMetadata,
    #[serde(rename = "launchConfig")]
    pub(crate) launch_config: StatusLaunchConfiguration,
    #[serde(rename = "statusProjection")]
    pub(crate) status_projection: StatusProjection,
    #[serde(rename = "runtimeLifecycle")]
    pub(crate) runtime_lifecycle: Value,
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

        let presentation_capacity =
            authority_state
                .presentation_capacity
                .as_ref()
                .map(|capacity| {
                    capacity.projection_with_service_state(
                        super::presentation_capacity::PressureAdmission::admit(
                            capacity.config.hard_maximum,
                        ),
                        Some(&authority_state),
                    )
                });
        let response = ServiceStatusResponse {
            control_plane: input.control_plane,
            profile_allocations: super::service_model::service_profile_allocations(
                &authority_state,
            ),
            manual_browsers,
            retained_display_allocations: super::service_model::retained_display_allocation_summary(
                &authority_state,
            ),
            presentation_capacity,
            desktop_evidence_policy:
                super::desktop_evidence::DesktopEvidenceCoordinator::policy_projection(),
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
            runtime_lifecycle: input.runtime_lifecycle,
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
    let runtime_lifecycle = crate::install::runtime_lifecycle_status_json_for_registry(
        &service_state.runtime_owner_registry,
    );
    projector
        .project(StatusAuthorityInput {
            service_state,
            control_plane,
            browser_session_authority,
            launch_config,
            full_tab_history,
            runtime_lifecycle,
        })
        .await
}

fn format_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests;
#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::remote_view_attachability::refresh_remote_view_attachability;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::service_health::{
        persist_browser_recovery_started_in_repository,
        persist_closed_browser_health_in_repository,
        persist_current_browser_stale_health_in_repository,
        persist_reconciled_service_state_in_repository,
        persist_service_browser_record_in_repository, reconcile_service_state,
        retry_degraded_service_browser_in_state, retry_persisted_service_browser_in_repository,
        retry_service_browser_in_state, BrowserRecoveryPersistence, BrowserRecoveryPolicyConfig,
        BrowserRecoveryPolicySource, BrowserRecoveryPolicyValueSource, BrowserRecoveryReasonKind,
    };
    use crate::native::service_model::{
        retained_display_allocation_candidates, service_profile_allocations,
        service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
        BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
        BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession,
        BrowserTab, ControlInputProvider, DisplayAllocation, JobState as ServiceJobState,
        LeaseState, MonitorState, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
        ProfileLeaseDisposition, ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease,
        RemoteViewHandoff, RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent,
        ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle,
        ViewStream, ViewStreamProvider, ViewerLease,
    };
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    use crate::native::state;
    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Map, Value};
    use std::sync::Arc;
    pub(crate) async fn handle_service_status(cmd: &Value) -> Result<Value, String> {
        let repository = LockedServiceStateRepository::default_json()?;
        let projector = super::super::service_status_projection::ServiceStatusProjector::local();
        handle_service_status_with_dependencies(
            cmd,
            super::super::service_status_projection::ServiceStatusProjectionDependencies::new(
                &repository,
                &super::super::service_status_projection::ReconcileServiceStatusAuthority,
                &super::super::service_status_projection::ReconciledBrowserSessionAuthority,
                &projector,
            ),
        )
        .await
    }
    pub(crate) async fn handle_service_status_with_dependencies<
        Repository,
        Preparer,
        BrowserAuthority,
    >(
        cmd: &Value,
        dependencies: super::super::service_status_projection::ServiceStatusProjectionDependencies<
            '_,
            Repository,
            Preparer,
            BrowserAuthority,
        >,
    ) -> Result<Value, String>
    where
        Repository: ServiceStateRepository,
        Preparer: super::super::service_status_projection::ServiceStatusAuthorityPreparer,
        BrowserAuthority:
            super::super::service_status_projection::ServiceStatusBrowserAuthorityProvider,
    {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let before = service_state.clone();
        let waiting_profile_lease_job_count = service_state
            .jobs
            .values()
            .filter(|job| job.state == ServiceJobState::WaitingProfileLease)
            .count();
        if let Some(control_plane) = service_state.control_plane.as_mut() {
            control_plane.waiting_profile_lease_job_count = waiting_profile_lease_job_count;
        } else {
            service_state.control_plane = Some(super::super::service_model::ControlPlaneSnapshot {
                worker_state: "Ready".to_string(),
                browser_health: "NotStarted".to_string(),
                waiting_profile_lease_job_count,
                ..super::super::service_model::ControlPlaneSnapshot::default()
            });
        }
        dependencies.preparer.prepare(&mut service_state).await;
        persist_reconciled_service_state_in_repository(
            dependencies.repository,
            &before,
            &service_state,
        )?;
        let browser_session_authority = dependencies.browser_authority.snapshot(&service_state);
        let control_plane = service_state
            .control_plane
            .as_ref()
            .expect("service status always creates a control-plane snapshot");
        let control_plane =
            super::super::service_status_projection::StatusControlPlaneAuthority::try_from(
                control_plane,
            )
            .map_err(|error| error.to_string())?;
        let launch_config =
            super::super::service_status_projection::launch_configuration_from_status_command(cmd);
        let full_tab_history = cmd
            .get("fullTabHistory")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let response =
            super::super::service_status_projection::project_status_with_launch_configuration(
                dependencies.projector,
                service_state,
                control_plane,
                browser_session_authority,
                launch_config,
                full_tab_history,
            )
            .await
            .map_err(|error| error.to_string())?;
        serde_json::to_value(response).map_err(|error| error.to_string())
    }
}
pub(crate) use action_commands::*;
