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
use std::collections::BTreeMap;

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
    pub(crate) service_state_projection: ServiceStateProjectionMode,
}

const DASHBOARD_SUMMARY_MAX_SERVICE_STATE_BYTES: usize = 1024 * 1024;
const DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const DASHBOARD_SUMMARY_MAX_RECORD_BYTES: usize = 8 * 1024;
const DASHBOARD_SUMMARY_PROFILE_ALLOCATION_LIMIT: usize = 128;
const DASHBOARD_SUMMARY_MANUAL_BROWSER_LIMIT: usize = 64;
const DASHBOARD_SUMMARY_BROWSER_VERDICT_LIMIT: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ServiceStateProjectionMode {
    #[default]
    Full,
    DashboardSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateProjectionMetadata {
    pub(crate) schema_version: u8,
    pub(crate) mode: ServiceStateProjectionMode,
    pub(crate) complete: bool,
    pub(crate) included_collections: Vec<&'static str>,
    pub(crate) omitted_collection_counts: BTreeMap<String, usize>,
    pub(crate) detail_routes: BTreeMap<&'static str, &'static str>,
    pub(crate) historical_limits: BTreeMap<&'static str, usize>,
    pub(crate) max_service_state_bytes: Option<usize>,
    pub(crate) max_serialized_response_bytes: Option<usize>,
    pub(crate) serialized_service_state_bytes: usize,
    pub(crate) truncated_record_count: usize,
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
    #[serde(rename = "serviceStateProjection")]
    pub(crate) service_state_projection: ServiceStateProjectionMetadata,
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
    #[serde(rename = "serviceStateLockDiagnostics")]
    pub(crate) service_state_lock_diagnostics: super::service_store::ServiceStateLockDiagnostics,
    #[serde(rename = "runtimeLifecycle")]
    pub(crate) runtime_lifecycle: Value,
    #[serde(rename = "crashRegenerationTransactions")]
    pub(crate) crash_regeneration_transactions:
        Vec<super::service_crash_regeneration::CrashRegenerationStatus>,
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

#[async_trait::async_trait]
pub(crate) trait ServiceStatusBrowserAuthorityProvider: Send + Sync {
    async fn snapshot(&self, service_state: &ServiceState) -> BrowserSessionAuthoritySnapshot;
}

#[derive(Debug, Default)]
pub(crate) struct ReconciledBrowserSessionAuthority;

#[async_trait::async_trait]
impl ServiceStatusBrowserAuthorityProvider for ReconciledBrowserSessionAuthority {
    async fn snapshot(&self, service_state: &ServiceState) -> BrowserSessionAuthoritySnapshot {
        let service_state = service_state.clone();
        tokio::task::spawn_blocking(move || {
            super::browser_session_authority::browser_session_authority_snapshot(&service_state)
        })
        .await
        .unwrap_or_default()
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

        let mut manual_browsers = compatibility::join_manual_browsers(
            &authority_state,
            observations.manual_browsers.clone(),
        );
        let (response_state, closed_tab_projection) =
            authority::project_closed_tabs(&authority_state, input.full_tab_history);
        let response_state =
            compatibility::apply_legacy_observation_mirrors(&response_state, &observations)?;
        let (response_state, mut service_state_projection) =
            project_service_state_for_delivery(response_state, input.service_state_projection);
        let mut profile_allocations =
            super::service_model::service_profile_allocations(&authority_state);
        let mut browser_session_authority = input.browser_session_authority;
        if input.service_state_projection == ServiceStateProjectionMode::DashboardSummary {
            profile_allocations.sort_by(|left, right| {
                let left_active = left.holder_count > 0
                    || left.waiting_job_count > 0
                    || !left.browser_ids.is_empty()
                    || !left.tab_ids.is_empty();
                let right_active = right.holder_count > 0
                    || right.waiting_job_count > 0
                    || !right.browser_ids.is_empty()
                    || !right.tab_ids.is_empty();
                right_active
                    .cmp(&left_active)
                    .then_with(|| left.profile_id.cmp(&right.profile_id))
            });
            truncate_vec_with_metadata(
                &mut profile_allocations,
                DASHBOARD_SUMMARY_PROFILE_ALLOCATION_LIMIT,
                "profileAllocations",
                &mut service_state_projection.omitted_collection_counts,
            );
            truncate_vec_with_metadata(
                &mut manual_browsers,
                DASHBOARD_SUMMARY_MANUAL_BROWSER_LIMIT,
                "manualBrowsers",
                &mut service_state_projection.omitted_collection_counts,
            );
            truncate_vec_with_metadata(
                &mut browser_session_authority.browser_verdicts,
                DASHBOARD_SUMMARY_BROWSER_VERDICT_LIMIT,
                "browserSessionAuthority.browserVerdicts",
                &mut service_state_projection.omitted_collection_counts,
            );
        }

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
            profile_allocations,
            manual_browsers,
            retained_display_allocations: super::service_model::retained_display_allocation_summary(
                &authority_state,
            ),
            presentation_capacity,
            desktop_evidence_policy:
                super::desktop_evidence::DesktopEvidenceCoordinator::policy_projection(),
            browser_session_authority,
            closed_tab_projection,
            launch_config: input.launch_config,
            service_state: response_state,
            service_state_projection,
            status_projection: StatusProjection {
                schema_version: 1,
                authority: StatusProjectionAuthority {
                    source: "reconciled_service_state",
                    projected_at,
                },
                observations,
            },
            service_state_lock_diagnostics: super::service_store::service_state_lock_diagnostics(),
            runtime_lifecycle: input.runtime_lifecycle,
            crash_regeneration_transactions:
                super::service_crash_regeneration::crash_regeneration_statuses(
                    &authority_state.crash_regeneration_transactions,
                ),
        };
        if input.service_state_projection == ServiceStateProjectionMode::DashboardSummary {
            let serialized_bytes = serde_json::to_vec(&response)
                .map_err(|error| ServiceStatusProjectionError::Serialization(error.to_string()))?
                .len();
            if serialized_bytes > DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES {
                return Err(ServiceStatusProjectionError::Serialization(format!(
                    "dashboard summary exceeded {} byte response ceiling",
                    DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES
                )));
            }
        }
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
    service_state_projection: ServiceStateProjectionMode,
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
            service_state_projection,
        })
        .await
}

pub(crate) fn service_state_projection_from_status_command(
    command: &Value,
) -> ServiceStateProjectionMode {
    match command.get("statusProjection").and_then(Value::as_str) {
        Some("dashboard_summary") => ServiceStateProjectionMode::DashboardSummary,
        _ => ServiceStateProjectionMode::Full,
    }
}

fn project_service_state_for_delivery(
    service_state: Value,
    mode: ServiceStateProjectionMode,
) -> (Value, ServiceStateProjectionMetadata) {
    const SCALAR_FIELDS: &[&str] = &[
        "browserCapabilityRegistry",
        "controlPlane",
        "profilePolicyMigration",
        "reconciliation",
    ];
    const COLLECTION_LIMITS: &[(&str, usize)] = &[
        ("browsers", 32),
        ("displayAllocations", 32),
        ("events", 32),
        ("incidents", 32),
        ("jobs", 64),
        ("profiles", 128),
        ("providers", 64),
        ("remoteViewRoutes", 32),
        ("routePool", 32),
        ("sessions", 64),
        ("sitePolicies", 64),
        ("tabs", 128),
    ];
    let detail_routes = BTreeMap::from([
        ("events", "/api/service/events"),
        ("jobs", "/api/service/jobs"),
        ("profiles", "/api/service/profiles"),
        ("remoteViewRoutes", "/api/service/remote-view-routes"),
        ("viewerLeases", "/api/service/viewer-leases"),
    ]);
    if mode == ServiceStateProjectionMode::Full {
        let included_collections = service_state.as_object().map_or_else(Vec::new, |state| {
            state
                .keys()
                .filter_map(|key| {
                    SCALAR_FIELDS
                        .iter()
                        .copied()
                        .chain(COLLECTION_LIMITS.iter().map(|(name, _)| *name))
                        .find(|name| *name == key)
                })
                .collect()
        });
        let serialized_service_state_bytes =
            serde_json::to_vec(&service_state).map_or(0, |bytes| bytes.len());
        return (
            service_state,
            ServiceStateProjectionMetadata {
                schema_version: 1,
                mode,
                complete: true,
                included_collections,
                omitted_collection_counts: BTreeMap::new(),
                detail_routes,
                historical_limits: BTreeMap::new(),
                max_service_state_bytes: None,
                max_serialized_response_bytes: None,
                serialized_service_state_bytes,
                truncated_record_count: 0,
            },
        );
    }

    let Some(source) = service_state.as_object() else {
        return (
            service_state,
            ServiceStateProjectionMetadata {
                schema_version: 1,
                mode,
                complete: false,
                included_collections: Vec::new(),
                omitted_collection_counts: BTreeMap::new(),
                detail_routes,
                historical_limits: COLLECTION_LIMITS.iter().copied().collect(),
                max_service_state_bytes: Some(DASHBOARD_SUMMARY_MAX_SERVICE_STATE_BYTES),
                max_serialized_response_bytes: Some(DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES),
                serialized_service_state_bytes: 0,
                truncated_record_count: 0,
            },
        );
    };
    let mut projected = Map::new();
    let mut omitted_collection_counts = BTreeMap::new();
    let mut truncated_record_count = 0;
    for &name in SCALAR_FIELDS {
        let Some(value) = source.get(name) else {
            continue;
        };
        projected.insert(
            name.to_string(),
            compact_summary_value(value, &mut truncated_record_count),
        );
    }
    for &(name, limit) in COLLECTION_LIMITS {
        let Some(original) = source.get(name) else {
            continue;
        };
        let value = bounded_collection(name, original, limit, &mut truncated_record_count);
        let original_count = collection_len(source.get(name));
        let projected_count = collection_len(Some(&value));
        if original_count > projected_count {
            omitted_collection_counts.insert(name.to_string(), original_count - projected_count);
        }
        projected.insert(name.to_string(), value);
    }
    for (name, value) in source {
        if !SCALAR_FIELDS.contains(&name.as_str())
            && !COLLECTION_LIMITS
                .iter()
                .any(|(included, _)| included == name)
        {
            let count = collection_len(Some(value));
            if count > 0 {
                omitted_collection_counts.insert(name.clone(), count);
            }
        }
    }
    enforce_dashboard_summary_byte_ceiling(&mut projected, &mut omitted_collection_counts);
    let serialized_service_state_bytes =
        serde_json::to_vec(&projected).map_or(0, |bytes| bytes.len());
    let included_collections = projected
        .keys()
        .filter_map(|key| {
            SCALAR_FIELDS
                .iter()
                .copied()
                .chain(COLLECTION_LIMITS.iter().map(|(name, _)| *name))
                .find(|name| *name == key)
        })
        .collect();
    (
        Value::Object(projected),
        ServiceStateProjectionMetadata {
            schema_version: 1,
            mode,
            complete: false,
            included_collections,
            omitted_collection_counts,
            detail_routes,
            historical_limits: COLLECTION_LIMITS.iter().copied().collect(),
            max_service_state_bytes: Some(DASHBOARD_SUMMARY_MAX_SERVICE_STATE_BYTES),
            max_serialized_response_bytes: Some(DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES),
            serialized_service_state_bytes,
            truncated_record_count,
        },
    )
}

fn truncate_vec_with_metadata<T>(
    values: &mut Vec<T>,
    limit: usize,
    name: &str,
    omitted: &mut BTreeMap<String, usize>,
) {
    if values.len() <= limit {
        return;
    }
    let omitted_count = values.len() - limit;
    values.truncate(limit);
    omitted.insert(name.to_string(), omitted_count);
}

fn collection_len(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(values)) => values.len(),
        Some(Value::Object(values)) => values.len(),
        Some(Value::Null) | None => 0,
        Some(_) => 1,
    }
}

fn bounded_collection(
    name: &str,
    value: &Value,
    limit: usize,
    truncated_record_count: &mut usize,
) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .iter()
                .rev()
                .take(limit)
                .rev()
                .map(|value| compact_summary_record(name, value, truncated_record_count))
                .collect(),
        ),
        Value::Object(values) => {
            let mut ranked = values.iter().collect::<Vec<_>>();
            ranked.sort_by(|(left_id, left), (right_id, right)| {
                summary_record_is_active(name, right)
                    .cmp(&summary_record_is_active(name, left))
                    .then_with(|| right_id.cmp(left_id))
            });
            ranked.truncate(limit);
            ranked.sort_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                ranked
                    .into_iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            compact_summary_record(name, value, truncated_record_count),
                        )
                    })
                    .collect(),
            )
        }
        _ => compact_summary_record(name, value, truncated_record_count),
    }
}

fn summary_record_is_active(collection: &str, value: &Value) -> bool {
    let state = value
        .get("state")
        .or_else(|| value.get("lifecycle"))
        .or_else(|| value.get("health"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    match collection {
        "jobs" => matches!(state, "queued" | "running" | "waiting_profile_lease"),
        "tabs" => matches!(state, "ready" | "loading" | "active"),
        "sessions" => {
            matches!(state, "active" | "human_takeover")
                || value
                    .get("browserIds")
                    .and_then(Value::as_array)
                    .is_some_and(|v| !v.is_empty())
                || value
                    .get("tabIds")
                    .and_then(Value::as_array)
                    .is_some_and(|v| !v.is_empty())
        }
        "browsers" => !matches!(
            state,
            "process_exited"
                | "closed"
                | "stopped"
                | "retired"
                | "completed"
                | "succeeded"
                | "failed"
                | "timed_out"
                | "disconnected"
        ),
        "incidents" => value.get("resolvedAt").is_none_or(Value::is_null),
        "remoteViewRoutes" => matches!(state, "ready" | "connecting" | "checked_out"),
        _ => matches!(state, "ready" | "active" | "observing" | "checked_out"),
    }
}

fn compact_summary_record(
    collection: &str,
    value: &Value,
    truncated_record_count: &mut usize,
) -> Value {
    let compacted = compact_summary_value(value, truncated_record_count);
    if serde_json::to_vec(&compacted).map_or(0, |bytes| bytes.len())
        <= DASHBOARD_SUMMARY_MAX_RECORD_BYTES
    {
        return compacted;
    }
    *truncated_record_count += 1;
    let Some(record) = compacted.as_object() else {
        return Value::Null;
    };
    const IDENTITY_FIELDS: &[&str] = &[
        "id",
        "name",
        "state",
        "health",
        "lifecycle",
        "profileId",
        "browserId",
        "sessionId",
        "routeId",
        "displayAllocationId",
        "serviceName",
        "agentName",
        "taskName",
        "createdAt",
        "updatedAt",
        "completedAt",
        "lastObservedAt",
    ];
    const PROFILE_ACTIONABILITY_FIELDS: &[&str] = &[
        "profileOrigin",
        "profileClass",
        "accessPolicy",
        "userDataDir",
        "sitePolicyIds",
        "targetServiceIds",
        "authenticatedServiceIds",
        "accountIds",
        "defaultBrowserHost",
        "browserBuild",
        "allocation",
        "keyring",
        "sharedServiceIds",
        "credentialProviderIds",
        "manualLoginPreferred",
        "targetReadiness",
        "persistent",
        "tags",
    ];
    const BROWSER_ACTIONABILITY_FIELDS: &[&str] = &[
        "host",
        "pid",
        "cdpEndpoint",
        "displayName",
        "displayAllocationId",
        "processStats",
        "viewStreams",
        "attachability",
        "activeSessionIds",
        "lastError",
        "inventoryClass",
        "inventoryPlacement",
        "lifecycleState",
        "routeBoundOwnership",
        "operatorVisibleProof",
        "lifecycleActions",
        "presentationActionCeilings",
        "diagnostics",
    ];
    const SESSION_ACTIONABILITY_FIELDS: &[&str] = &[
        "owner",
        "lease",
        "browserIds",
        "tabIds",
        "cleanup",
        "profileLeaseDisposition",
        "profileLeaseConflictSessionIds",
        "lastLeaseObservedAt",
        "expiresAt",
    ];
    const TAB_ACTIONABILITY_FIELDS: &[&str] = &[
        "targetId",
        "ownerSessionId",
        "url",
        "title",
        "latestSnapshotId",
        "latestScreenshotId",
        "challengeId",
        "serviceTabHandle",
    ];
    const ROUTE_ACTIONABILITY_FIELDS: &[&str] = &[
        "provider",
        "url",
        "frameUrl",
        "externalUrl",
        "routePoolEntryId",
        "connectionId",
        "connectionName",
        "routeSource",
        "providerMode",
        "viewerLeaseIds",
        "controllerLeaseId",
        "readiness",
        "remoteReadiness",
        "attachability",
        "displayContent",
        "readOnly",
        "controlInput",
        "routeBoundOwnership",
    ];
    let actionability_fields = match collection {
        "profiles" => PROFILE_ACTIONABILITY_FIELDS,
        "browsers" => BROWSER_ACTIONABILITY_FIELDS,
        "sessions" => SESSION_ACTIONABILITY_FIELDS,
        "tabs" => TAB_ACTIONABILITY_FIELDS,
        "remoteViewRoutes" => ROUTE_ACTIONABILITY_FIELDS,
        _ => &[],
    };
    Value::Object(
        IDENTITY_FIELDS
            .iter()
            .chain(actionability_fields)
            .filter_map(|field| {
                record
                    .get(*field)
                    .cloned()
                    .map(|value| ((*field).to_string(), value))
            })
            .collect(),
    )
}

fn compact_summary_value(value: &Value, truncated_record_count: &mut usize) -> Value {
    match value {
        Value::String(value) if value.len() > 1024 => {
            *truncated_record_count += 1;
            Value::String(value.chars().take(1024).collect())
        }
        Value::Array(values) if values.len() > 64 => {
            *truncated_record_count += 1;
            Value::Array(
                values[values.len() - 64..]
                    .iter()
                    .map(|value| compact_summary_value(value, truncated_record_count))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| compact_summary_value(value, truncated_record_count))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        compact_summary_value(value, truncated_record_count),
                    )
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn enforce_dashboard_summary_byte_ceiling(
    state: &mut Map<String, Value>,
    omitted: &mut BTreeMap<String, usize>,
) {
    const TRIM_ORDER: &[&str] = &[
        "events",
        "jobs",
        "profiles",
        "providers",
        "sitePolicies",
        "tabs",
        "sessions",
        "displayAllocations",
        "routePool",
        "remoteViewRoutes",
        "browsers",
        "incidents",
    ];
    while serde_json::to_vec(state).map_or(usize::MAX, |bytes| bytes.len())
        > DASHBOARD_SUMMARY_MAX_SERVICE_STATE_BYTES
    {
        let mut removed = false;
        for name in TRIM_ORDER {
            let Some(value) = state.get_mut(*name) else {
                continue;
            };
            let did_remove = match value {
                Value::Array(values) => (!values.is_empty()).then(|| values.pop()).is_some(),
                Value::Object(values) => values
                    .keys()
                    .next_back()
                    .cloned()
                    .and_then(|key| values.remove(&key))
                    .is_some(),
                _ => false,
            };
            if did_remove {
                *omitted.entry((*name).to_string()).or_default() += 1;
                removed = true;
                break;
            }
        }
        if !removed {
            break;
        }
    }
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
        persist_service_browser_record_in_repository, reconcile_service_state,
        retry_degraded_service_browser_in_state, retry_persisted_service_browser_in_repository,
        retry_service_browser_in_state, BrowserRecoveryPersistence, BrowserRecoveryPolicyConfig,
        BrowserRecoveryPolicySource, BrowserRecoveryPolicyValueSource, BrowserRecoveryReasonKind,
    };
    use crate::native::service_model::{
        retained_display_allocation_candidates, service_profile_allocations,
        service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
        BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
        BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile,
        BrowserRecordAuthoritySource, BrowserRecordLifecycleClassification,
        BrowserRecordProvenance, BrowserRecordSource, BrowserSession, BrowserTab,
        ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState,
        MonitorState, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
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
        let caller_projection = cmd.get("serviceState").is_some();
        let mut service_state = if let Some(projected) = cmd.get("serviceState") {
            serde_json::from_value::<ServiceState>(projected.clone())
                .map_err(|err| format!("Invalid serviceState: {}", err))?
        } else {
            dependencies.repository.load_snapshot()?
        };
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
        project_browser_record_provenance(&mut service_state, caller_projection);
        let browser_session_authority = dependencies
            .browser_authority
            .snapshot(&service_state)
            .await;
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
        let service_state_projection =
            super::super::service_status_projection::service_state_projection_from_status_command(
                cmd,
            );
        let response =
            super::super::service_status_projection::project_status_with_launch_configuration(
                dependencies.projector,
                service_state,
                control_plane,
                browser_session_authority,
                launch_config,
                full_tab_history,
                service_state_projection,
            )
            .await
            .map_err(|error| error.to_string())?;
        serde_json::to_value(response).map_err(|error| error.to_string())
    }

    fn project_browser_record_provenance(state: &mut ServiceState, caller_projection: bool) {
        let process_backed = state
            .browser_process_identities
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let managed = state
            .runtime_owner_registry
            .lifecycle_records
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let session_references = state
            .sessions
            .values()
            .flat_map(|session| session.browser_ids.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>();
        for (browser_id, browser) in &mut state.browsers {
            let last_observed_at = browser
                .last_health_observation
                .as_ref()
                .map(|observation| observation.observed_at.clone())
                .filter(|value| !value.is_empty());
            let (source, authority_source) = if caller_projection {
                (
                    BrowserRecordSource::CallerProjection,
                    BrowserRecordAuthoritySource::CallerProjection,
                )
            } else if managed.contains(browser_id) {
                (
                    BrowserRecordSource::ManagedRuntime,
                    BrowserRecordAuthoritySource::ManagedRuntime,
                )
            } else if process_backed.contains(browser_id) {
                (
                    BrowserRecordSource::RuntimeObserved,
                    BrowserRecordAuthoritySource::ProcessIdentity,
                )
            } else {
                (
                    BrowserRecordSource::PersistedState,
                    BrowserRecordAuthoritySource::LegacyUnproven,
                )
            };
            let process_authority = process_backed.contains(browser_id) && browser.pid.is_some();
            let managed_authority = managed.contains(browser_id);
            let unreferenced = !session_references.contains(browser_id)
                && browser.active_session_ids.is_empty()
                && browser.display_allocation_id.is_none()
                && browser.view_streams.is_empty();
            let (lifecycle_classification, recommended_action) =
                if (process_authority || managed_authority) && browser.cdp_endpoint.is_some() {
                    (BrowserRecordLifecycleClassification::Reattachable, "close")
                } else if process_authority || managed_authority {
                    (BrowserRecordLifecycleClassification::Live, "close")
                } else if browser.pid.is_none() && unreferenced {
                    (BrowserRecordLifecycleClassification::InertLegacy, "retire")
                } else {
                    (
                        BrowserRecordLifecycleClassification::ReviewRequired,
                        "review",
                    )
                };
            let mut evidence_record = browser.clone();
            evidence_record.record_provenance = None;
            let evidence_digest =
                crate::native::runtime_lifecycle::digest_json(&evidence_record).unwrap_or_default();
            let record_revision = browser
                .record_provenance
                .as_ref()
                .map(|provenance| provenance.record_revision)
                .unwrap_or(0);
            browser.record_provenance = Some(BrowserRecordProvenance {
                source,
                authority_source,
                created_at: None,
                last_observed_at,
                lifecycle_classification,
                recommended_action: recommended_action.to_string(),
                record_revision,
                evidence_digest,
            });
        }
    }

    #[cfg(test)]
    mod slice_d_tests {
        use super::*;
        use crate::native::service_status_projection::{
            ReconciledBrowserSessionAuthority, ServiceStatusAuthorityPreparer,
            ServiceStatusProjectionDependencies, ServiceStatusProjector,
        };
        use std::collections::BTreeMap;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;

        struct MemoryRepository {
            state: Mutex<ServiceState>,
            mutation_count: AtomicUsize,
        }

        impl MemoryRepository {
            fn new(state: ServiceState) -> Self {
                Self {
                    state: Mutex::new(state),
                    mutation_count: AtomicUsize::new(0),
                }
            }
        }

        impl ServiceStateRepository for MemoryRepository {
            fn load_snapshot(&self) -> Result<ServiceState, String> {
                Ok(self.state.lock().unwrap().clone())
            }

            fn mutate<R>(
                &self,
                mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
            ) -> Result<R, String> {
                self.mutation_count.fetch_add(1, Ordering::SeqCst);
                let mut state = self.state.lock().unwrap();
                mutator(&mut state)
            }
        }

        #[derive(Default)]
        struct NoopPreparer;

        #[async_trait::async_trait]
        impl ServiceStatusAuthorityPreparer for NoopPreparer {
            async fn prepare(&self, _service_state: &mut ServiceState) {}
        }

        fn browser_state(id: &str) -> ServiceState {
            ServiceState {
                browsers: BTreeMap::from([(
                    id.to_string(),
                    BrowserProcess {
                        id: id.to_string(),
                        ..BrowserProcess::default()
                    },
                )]),
                ..ServiceState::default()
            }
        }

        #[tokio::test]
        async fn caller_supplied_status_state_is_projection_only_and_never_persisted() {
            let repository = MemoryRepository::new(browser_state("durable-browser"));
            let supplied = browser_state("browser-cdp");
            let projector = ServiceStatusProjector::unavailable("provider-free projection test");
            let response = handle_service_status_with_dependencies(
                &json!({ "serviceState": supplied }),
                ServiceStatusProjectionDependencies::new(
                    &repository,
                    &NoopPreparer,
                    &ReconciledBrowserSessionAuthority,
                    &projector,
                ),
            )
            .await
            .unwrap();

            assert_eq!(repository.mutation_count.load(Ordering::SeqCst), 0);
            assert!(repository
                .load_snapshot()
                .unwrap()
                .browsers
                .contains_key("durable-browser"));
            assert!(!repository
                .load_snapshot()
                .unwrap()
                .browsers
                .contains_key("browser-cdp"));
            assert_eq!(
                response["service_state"]["browsers"]["browser-cdp"]["recordProvenance"]["source"],
                "caller_projection"
            );
            assert_eq!(
                response["service_state"]["browsers"]["browser-cdp"]["recordProvenance"]
                    ["lifecycleClassification"],
                "inert_legacy"
            );
            assert_eq!(
                response["service_state"]["browsers"]["browser-cdp"]["recordProvenance"]
                    ["recommendedAction"],
                "retire"
            );
        }

        #[tokio::test]
        async fn repository_status_read_is_read_only_and_marks_unproven_legacy_authority() {
            let repository = MemoryRepository::new(browser_state("session:odollo-carrier-ups"));
            let before = repository.load_snapshot().unwrap();
            let projector = ServiceStatusProjector::unavailable("provider-free projection test");
            let response = handle_service_status_with_dependencies(
                &json!({}),
                ServiceStatusProjectionDependencies::new(
                    &repository,
                    &NoopPreparer,
                    &ReconciledBrowserSessionAuthority,
                    &projector,
                ),
            )
            .await
            .unwrap();

            assert_eq!(repository.mutation_count.load(Ordering::SeqCst), 0);
            assert_eq!(repository.load_snapshot().unwrap(), before);
            assert_eq!(
                response["service_state"]["browsers"]["session:odollo-carrier-ups"]
                    ["recordProvenance"]["authoritySource"],
                "legacy_unproven"
            );
            assert_eq!(
                response["service_state"]["browsers"]["session:odollo-carrier-ups"]
                    ["recordProvenance"]["lifecycleClassification"],
                "inert_legacy"
            );
        }
    }
}
pub(crate) use action_commands::*;
