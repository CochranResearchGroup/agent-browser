//! Durable service-mode contracts.
//!
//! These types describe the browser service state model before the service API
//! and MCP surfaces are wired to runtime behavior. Keep them serializable and
//! conservative so future clients can depend on stable field names.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level snapshot of the browser service control plane.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceState {
    pub control_plane: Option<ControlPlaneSnapshot>,
    pub reconciliation: Option<ServiceReconciliationSnapshot>,
    pub events: Vec<ServiceEvent>,
    pub incidents: Vec<ServiceIncident>,
    pub profiles: BTreeMap<String, BrowserProfile>,
    pub browsers: BTreeMap<String, BrowserProcess>,
    pub sessions: BTreeMap<String, BrowserSession>,
    pub tabs: BTreeMap<String, BrowserTab>,
    pub jobs: BTreeMap<String, ServiceJob>,
    pub monitors: BTreeMap<String, SiteMonitor>,
    pub site_policies: BTreeMap<String, SitePolicy>,
    pub providers: BTreeMap<String, ServiceProvider>,
    pub challenges: BTreeMap<String, Challenge>,
}

impl ServiceState {
    pub fn overlay_configured_entities(&mut self, configured: ServiceState) {
        self.site_policies.extend(configured.site_policies);
        self.providers.extend(configured.providers);
    }

    /// Refresh bounded derived collections before persistence or API exposure.
    pub fn refresh_derived_views(&mut self) {
        let preserved_metadata = self
            .incidents
            .iter()
            .map(|incident| {
                (
                    incident.id.clone(),
                    (
                        incident.acknowledged_at.clone(),
                        incident.acknowledged_by.clone(),
                        incident.acknowledgement_note.clone(),
                        incident.resolved_at.clone(),
                        incident.resolved_by.clone(),
                        incident.resolution_note.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.incidents = derive_service_incidents(self)
            .into_iter()
            .map(|mut incident| {
                if let Some((
                    acknowledged_at,
                    acknowledged_by,
                    acknowledgement_note,
                    resolved_at,
                    resolved_by,
                    resolution_note,
                )) = preserved_metadata.get(&incident.id)
                {
                    incident.acknowledged_at = acknowledged_at.clone();
                    incident.acknowledged_by = acknowledged_by.clone();
                    incident.acknowledgement_note = acknowledgement_note.clone();
                    incident.resolved_at = resolved_at.clone();
                    incident.resolved_by = resolved_by.clone();
                    incident.resolution_note = resolution_note.clone();
                }
                incident
            })
            .collect();
    }
}

/// Bounded service event log entry for operator auditability.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceEvent {
    pub id: String,
    pub timestamp: String,
    pub kind: ServiceEventKind,
    pub message: String,
    pub browser_id: Option<String>,
    pub previous_health: Option<BrowserHealth>,
    pub current_health: Option<BrowserHealth>,
    pub details: Option<serde_json::Value>,
}

/// Grouped service incident derived from event and job history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceIncident {
    pub id: String,
    pub browser_id: Option<String>,
    pub label: String,
    pub state: ServiceIncidentState,
    pub acknowledged_at: Option<String>,
    pub acknowledged_by: Option<String>,
    pub acknowledgement_note: Option<String>,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub resolution_note: Option<String>,
    pub latest_timestamp: String,
    pub latest_message: String,
    pub latest_kind: String,
    pub current_health: Option<BrowserHealth>,
    pub event_ids: Vec<String>,
    pub job_ids: Vec<String>,
}

/// Operator-facing summary state for a grouped incident.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceIncidentState {
    #[default]
    Active,
    Recovered,
    Service,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEventKind {
    #[default]
    Reconciliation,
    BrowserHealthChanged,
    TabLifecycleChanged,
    ReconciliationError,
    IncidentAcknowledged,
    IncidentResolved,
}

fn derive_service_incidents(state: &ServiceState) -> Vec<ServiceIncident> {
    let mut grouped = BTreeMap::<String, ServiceIncident>::new();

    for event in state
        .events
        .iter()
        .filter(|event| service_event_is_incident(event))
    {
        let browser_id = event.browser_id.clone();
        let key = service_event_incident_id(event)
            .or(browser_id.clone())
            .unwrap_or_else(|| "service".to_string());
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                label: browser_id
                    .clone()
                    .unwrap_or_else(|| "Service incidents".to_string()),
                state: classify_incident_state(
                    browser_id.is_some(),
                    event.current_health,
                    event.kind,
                ),
                latest_timestamp: event.timestamp.clone(),
                latest_message: event.message.clone(),
                latest_kind: service_event_kind_name(event.kind).to_string(),
                current_health: event.current_health,
                ..ServiceIncident::default()
            });

        if !service_event_is_handling(event.kind)
            && incident_is_newer(&event.timestamp, &incident.latest_timestamp)
        {
            incident.latest_timestamp = event.timestamp.clone();
            incident.latest_message = event.message.clone();
            incident.latest_kind = service_event_kind_name(event.kind).to_string();
            incident.current_health = event.current_health.or(incident.current_health);
            incident.state = classify_incident_state(
                incident.browser_id.is_some(),
                incident.current_health,
                event.kind,
            );
        }

        incident.event_ids.push(event.id.clone());
    }

    for job in state
        .jobs
        .values()
        .filter(|job| service_job_is_incident(job))
    {
        let browser_id = service_job_browser_id(job, state);
        let key = browser_id.clone().unwrap_or_else(|| "service".to_string());
        let timestamp = service_job_incident_timestamp(job);
        let message = service_job_incident_message(job);
        let kind = service_job_incident_kind(job);
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                label: browser_id
                    .clone()
                    .unwrap_or_else(|| "Service incidents".to_string()),
                state: classify_job_incident_state(browser_id.is_some()),
                latest_timestamp: timestamp.to_string(),
                latest_message: message.clone(),
                latest_kind: kind.to_string(),
                current_health: state.browsers.get(&key).map(|browser| browser.health),
                ..ServiceIncident::default()
            });

        if incident_is_newer(&timestamp, &incident.latest_timestamp) {
            incident.latest_timestamp = timestamp.to_string();
            incident.latest_message = message;
            incident.latest_kind = kind.to_string();
            if let Some(browser_id) = incident.browser_id.as_ref() {
                incident.current_health =
                    state.browsers.get(browser_id).map(|browser| browser.health);
            }
            incident.state = classify_job_incident_state(incident.browser_id.is_some());
        }

        incident.job_ids.push(job.id.clone());
    }

    let event_timestamps: BTreeMap<&str, &str> = state
        .events
        .iter()
        .map(|event| (event.id.as_str(), event.timestamp.as_str()))
        .collect();
    let job_timestamps: BTreeMap<&str, &str> = state
        .jobs
        .values()
        .map(|job| (job.id.as_str(), service_job_incident_timestamp(job)))
        .collect();

    let mut incidents = grouped.into_values().collect::<Vec<_>>();
    for incident in &mut incidents {
        incident.event_ids.sort_by(|left, right| {
            job_or_event_timestamp(&event_timestamps, left)
                .cmp(&job_or_event_timestamp(&event_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        incident.job_ids.sort_by(|left, right| {
            job_or_event_timestamp(&job_timestamps, left)
                .cmp(&job_or_event_timestamp(&job_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        if incident.label.is_empty() {
            incident.label = incident
                .browser_id
                .clone()
                .unwrap_or_else(|| "Service incidents".to_string());
        }
        if incident.current_health.is_none() {
            incident.current_health = incident
                .browser_id
                .as_ref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .map(|browser| browser.health);
        }
        if let Some(browser_id) = incident.browser_id.as_ref() {
            if browser_health_is_bad(incident.current_health) {
                incident.state = ServiceIncidentState::Active;
            } else if incident.latest_kind == "browser_health_changed"
                && state
                    .events
                    .iter()
                    .filter(|event| incident.event_ids.contains(&event.id))
                    .any(|event| {
                        event.kind == ServiceEventKind::BrowserHealthChanged
                            && browser_health_is_recovery(
                                event.previous_health,
                                event.current_health,
                            )
                            && event.browser_id.as_deref() == Some(browser_id)
                    })
            {
                incident.state = ServiceIncidentState::Recovered;
            } else {
                incident.state = ServiceIncidentState::Active;
            }
        } else {
            incident.state = ServiceIncidentState::Service;
        }
    }
    incidents.sort_by(|left, right| {
        left.latest_timestamp
            .cmp(&right.latest_timestamp)
            .reverse()
            .then_with(|| left.id.cmp(&right.id))
    });
    incidents
}

fn job_or_event_timestamp<'a>(timestamps: &'a BTreeMap<&str, &'a str>, id: &str) -> &'a str {
    timestamps.get(id).copied().unwrap_or("")
}

fn incident_is_newer(candidate: &str, current: &str) -> bool {
    candidate >= current
}

fn service_event_is_incident(event: &ServiceEvent) -> bool {
    match event.kind {
        ServiceEventKind::ReconciliationError => true,
        ServiceEventKind::IncidentAcknowledged | ServiceEventKind::IncidentResolved => true,
        ServiceEventKind::BrowserHealthChanged => {
            browser_health_is_bad(event.current_health)
                || browser_health_is_recovery(event.previous_health, event.current_health)
        }
        ServiceEventKind::Reconciliation | ServiceEventKind::TabLifecycleChanged => false,
    }
}

fn service_event_is_handling(kind: ServiceEventKind) -> bool {
    matches!(
        kind,
        ServiceEventKind::IncidentAcknowledged | ServiceEventKind::IncidentResolved
    )
}

fn service_event_incident_id(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("incidentId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn browser_health_is_bad(value: Option<BrowserHealth>) -> bool {
    matches!(
        value,
        Some(BrowserHealth::Degraded)
            | Some(BrowserHealth::ProcessExited)
            | Some(BrowserHealth::CdpDisconnected)
            | Some(BrowserHealth::Unreachable)
            | Some(BrowserHealth::Faulted)
    )
}

fn browser_health_is_recovery(
    previous: Option<BrowserHealth>,
    current: Option<BrowserHealth>,
) -> bool {
    browser_health_is_bad(previous) && current == Some(BrowserHealth::Ready)
}

fn service_job_is_incident(job: &ServiceJob) -> bool {
    matches!(job.state, JobState::Cancelled | JobState::TimedOut)
}

fn service_job_incident_kind(job: &ServiceJob) -> &'static str {
    match job.state {
        JobState::TimedOut => "service_job_timeout",
        JobState::Cancelled => "service_job_cancelled",
        _ => "service_job_incident",
    }
}

fn service_job_incident_message(job: &ServiceJob) -> String {
    if job.state == JobState::TimedOut {
        format!("{} timed out", empty_to_service_label(&job.action))
    } else {
        format!("{} was cancelled", empty_to_service_label(&job.action))
    }
}

fn empty_to_service_label(value: &str) -> &str {
    if value.is_empty() {
        "Service job"
    } else {
        value
    }
}

fn service_job_incident_timestamp(job: &ServiceJob) -> &str {
    job.completed_at
        .as_deref()
        .or(job.started_at.as_deref())
        .or(job.submitted_at.as_deref())
        .unwrap_or("")
}

fn service_job_browser_id(job: &ServiceJob, state: &ServiceState) -> Option<String> {
    match &job.target {
        JobTarget::Browser(browser_id) => Some(browser_id.clone()),
        JobTarget::Tab(tab_id) => state.tabs.get(tab_id).map(|tab| tab.browser_id.clone()),
        JobTarget::Service
        | JobTarget::Profile(_)
        | JobTarget::Monitor(_)
        | JobTarget::Challenge(_) => None,
    }
}

fn service_event_kind_name(kind: ServiceEventKind) -> &'static str {
    match kind {
        ServiceEventKind::Reconciliation => "reconciliation",
        ServiceEventKind::BrowserHealthChanged => "browser_health_changed",
        ServiceEventKind::TabLifecycleChanged => "tab_lifecycle_changed",
        ServiceEventKind::ReconciliationError => "reconciliation_error",
        ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
        ServiceEventKind::IncidentResolved => "incident_resolved",
    }
}

fn classify_incident_state(
    has_browser: bool,
    current_health: Option<BrowserHealth>,
    kind: ServiceEventKind,
) -> ServiceIncidentState {
    if !has_browser {
        return ServiceIncidentState::Service;
    }
    if kind == ServiceEventKind::BrowserHealthChanged && !browser_health_is_bad(current_health) {
        return ServiceIncidentState::Recovered;
    }
    ServiceIncidentState::Active
}

fn classify_job_incident_state(has_browser: bool) -> ServiceIncidentState {
    if has_browser {
        ServiceIncidentState::Active
    } else {
        ServiceIncidentState::Service
    }
}

/// Latest persisted service reconciliation result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceReconciliationSnapshot {
    pub last_reconciled_at: Option<String>,
    pub last_error: Option<String>,
    pub browser_count: usize,
    pub changed_browsers: usize,
}

/// Latest persisted control-plane status snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlPlaneSnapshot {
    pub worker_state: String,
    pub browser_health: String,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub service_job_timeout_ms: Option<u64>,
    pub updated_at: Option<String>,
}

/// Durable profile identity and launch policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProfile {
    pub id: String,
    pub name: String,
    pub user_data_dir: Option<String>,
    pub site_policy_ids: Vec<String>,
    pub default_browser_host: Option<BrowserHost>,
    pub persistent: bool,
    pub tags: Vec<String>,
}

/// A supervised or attached browser process known to the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProcess {
    pub id: String,
    pub profile_id: Option<String>,
    pub host: BrowserHost,
    pub health: BrowserHealth,
    pub pid: Option<u32>,
    pub cdp_endpoint: Option<String>,
    pub view_streams: Vec<ViewStream>,
    pub active_session_ids: Vec<String>,
    pub last_error: Option<String>,
}

impl Default for BrowserProcess {
    fn default() -> Self {
        Self {
            id: String::new(),
            profile_id: None,
            host: BrowserHost::LocalHeaded,
            health: BrowserHealth::NotStarted,
            pid: None,
            cdp_endpoint: None,
            view_streams: Vec::new(),
            active_session_ids: Vec::new(),
            last_error: None,
        }
    }
}

/// Logical lease for an agent, human, system task, or API client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserSession {
    pub id: String,
    pub owner: ServiceActor,
    pub lease: LeaseState,
    pub browser_ids: Vec<String>,
    pub tab_ids: Vec<String>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self {
            id: String::new(),
            owner: ServiceActor::System,
            lease: LeaseState::Shared,
            browser_ids: Vec::new(),
            tab_ids: Vec::new(),
            created_at: None,
            expires_at: None,
        }
    }
}

/// Current service view of a CDP target or browser tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserTab {
    pub id: String,
    pub browser_id: String,
    pub target_id: Option<String>,
    pub session_id: Option<String>,
    pub lifecycle: TabLifecycle,
    pub url: Option<String>,
    pub title: Option<String>,
    pub owner_session_id: Option<String>,
    pub latest_snapshot_id: Option<String>,
    pub latest_screenshot_id: Option<String>,
    pub challenge_id: Option<String>,
}

impl Default for BrowserTab {
    fn default() -> Self {
        Self {
            id: String::new(),
            browser_id: String::new(),
            target_id: None,
            session_id: None,
            lifecycle: TabLifecycle::Unknown,
            url: None,
            title: None,
            owner_session_id: None,
            latest_snapshot_id: None,
            latest_screenshot_id: None,
            challenge_id: None,
        }
    }
}

/// Queued or completed service work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceJob {
    pub id: String,
    pub action: String,
    /// Service-level caller label supplied by MCP, CLI, HTTP, or API clients.
    pub service_name: Option<String>,
    /// Agent-level caller label supplied by MCP, CLI, HTTP, or API clients.
    pub agent_name: Option<String>,
    /// Task-level caller label supplied by MCP, CLI, HTTP, or API clients.
    pub task_name: Option<String>,
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
}

impl Default for ServiceJob {
    fn default() -> Self {
        Self {
            id: String::new(),
            action: String::new(),
            service_name: None,
            agent_name: None,
            task_name: None,
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
        }
    }
}

/// Site or tab heartbeat managed by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SiteMonitor {
    pub id: String,
    pub name: String,
    pub target: MonitorTarget,
    pub interval_ms: u64,
    pub state: MonitorState,
    pub last_checked_at: Option<String>,
    pub last_result: Option<String>,
}

impl Default for SiteMonitor {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            target: MonitorTarget::Url(String::new()),
            interval_ms: 60_000,
            state: MonitorState::Paused,
            last_checked_at: None,
            last_result: None,
        }
    }
}

/// Per-site access reliability and interaction policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SitePolicy {
    pub id: String,
    pub origin_pattern: String,
    pub browser_host: Option<BrowserHost>,
    pub view_stream: Option<ViewStreamProvider>,
    pub control_input: Option<ControlInputProvider>,
    pub interaction_mode: InteractionMode,
    pub rate_limit: RateLimitPolicy,
    pub manual_login_preferred: bool,
    pub profile_required: bool,
    pub auth_providers: Vec<String>,
    pub challenge_policy: ChallengePolicy,
    pub allowed_challenge_providers: Vec<String>,
    pub notes: Option<String>,
}

impl Default for SitePolicy {
    fn default() -> Self {
        Self {
            id: String::new(),
            origin_pattern: String::new(),
            browser_host: None,
            view_stream: None,
            control_input: None,
            interaction_mode: InteractionMode::CdpDirect,
            rate_limit: RateLimitPolicy::default(),
            manual_login_preferred: false,
            profile_required: false,
            auth_providers: Vec::new(),
            challenge_policy: ChallengePolicy::AvoidFirst,
            allowed_challenge_providers: Vec::new(),
            notes: None,
        }
    }
}

/// Pacing and concurrency limits for a site policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimitPolicy {
    pub min_action_delay_ms: Option<u64>,
    pub jitter_ms: Option<u64>,
    pub cooldown_ms: Option<u64>,
    pub max_parallel_sessions: Option<u32>,
    pub retry_budget: Option<u32>,
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self {
            min_action_delay_ms: Some(0),
            jitter_ms: Some(0),
            cooldown_ms: None,
            max_parallel_sessions: None,
            retry_budget: None,
        }
    }
}

/// External or built-in integration available to service workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceProvider {
    pub id: String,
    pub kind: ProviderKind,
    pub display_name: String,
    pub enabled: bool,
    pub config_ref: Option<String>,
    pub capabilities: Vec<ProviderCapability>,
}

impl Default for ServiceProvider {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: ProviderKind::ManualApproval,
            display_name: String::new(),
            enabled: true,
            config_ref: None,
            capabilities: Vec::new(),
        }
    }
}

/// Detected auth, 2FA, captcha, passkey, or blocked-flow challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Challenge {
    pub id: String,
    pub tab_id: Option<String>,
    pub kind: ChallengeKind,
    pub state: ChallengeState,
    pub detected_at: Option<String>,
    pub provider_id: Option<String>,
    pub policy_decision: Option<String>,
    pub human_approved: bool,
    pub result: Option<String>,
}

impl Default for Challenge {
    fn default() -> Self {
        Self {
            id: String::new(),
            tab_id: None,
            kind: ChallengeKind::Unknown,
            state: ChallengeState::Detected,
            detected_at: None,
            provider_id: None,
            policy_decision: None,
            human_approved: false,
            result: None,
        }
    }
}

/// Browser host execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHost {
    LocalHeadless,
    LocalHeaded,
    DockerHeaded,
    RemoteHeaded,
    CloudProvider,
    AttachedExisting,
}

/// Browser process health as seen by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHealth {
    NotStarted,
    Launching,
    Ready,
    Degraded,
    Unreachable,
    ProcessExited,
    CdpDisconnected,
    Reconnecting,
    Closing,
    Faulted,
}

/// Dashboard viewing mechanism for a browser or tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ViewStream {
    pub id: String,
    pub provider: ViewStreamProvider,
    pub url: Option<String>,
    pub read_only: bool,
}

impl Default for ViewStream {
    fn default() -> Self {
        Self {
            id: String::new(),
            provider: ViewStreamProvider::CdpScreencast,
            url: None,
            read_only: true,
        }
    }
}

/// Supported live-view transport families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewStreamProvider {
    CdpScreencast,
    ChromeTabWebrtc,
    VirtualDisplayWebrtc,
    Novnc,
    ExternalUrl,
}

/// Supported remote-input transport families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlInputProvider {
    CdpInput,
    WebrtcInput,
    VncInput,
    ManualAttachedDesktop,
}

/// Policy-selected action backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    CdpDirect,
    DomAction,
    BrowserInput,
    HumanLikeInput,
    Manual,
}

/// Challenge resolution posture for a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengePolicy {
    AvoidFirst,
    ManualOnly,
    ProviderAllowed,
    ProviderPreferred,
    Deny,
}

/// Service-side actor category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceActor {
    Agent(String),
    Human(String),
    ApiClient(String),
    System,
}

/// Lease semantics for a session or tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Shared,
    Exclusive,
    HumanTakeover,
    Released,
    Expired,
}

/// Current tab lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabLifecycle {
    Unknown,
    Opening,
    Loading,
    Ready,
    Closing,
    Closed,
    Crashed,
}

/// Queue target for a service job.
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

/// Queue lifecycle for a service job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

/// Job dispatch priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    Low,
    Normal,
    Lifecycle,
}

/// Monitor target variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorTarget {
    Url(String),
    Tab(String),
    SitePolicy(String),
}

/// Monitor execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorState {
    Active,
    Paused,
    Faulted,
}

/// Provider family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    BrowserCredentials,
    PasswordManager,
    Totp,
    Sms,
    Email,
    ManualApproval,
    Intelligence,
    Captcha,
}

/// Capability advertised by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    PasswordFill,
    Passkey,
    TotpCode,
    SmsCode,
    EmailCode,
    VisualReasoning,
    CaptchaSolve,
    HumanApproval,
}

/// Challenge category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeKind {
    Unknown,
    Captcha,
    TwoFactor,
    Passkey,
    SuspiciousLogin,
    BlockedFlow,
}

/// Challenge lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeState {
    Detected,
    WaitingForProvider,
    WaitingForHuman,
    Resolved,
    Failed,
    Denied,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn site_policy_serializes_stable_wire_names() {
        let policy = SitePolicy {
            id: "google".to_string(),
            origin_pattern: "https://accounts.google.com".to_string(),
            browser_host: Some(BrowserHost::DockerHeaded),
            view_stream: Some(ViewStreamProvider::VirtualDisplayWebrtc),
            control_input: Some(ControlInputProvider::WebrtcInput),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(250),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(2),
            },
            manual_login_preferred: true,
            profile_required: true,
            auth_providers: vec!["gog".to_string(), "imcli".to_string()],
            challenge_policy: ChallengePolicy::AvoidFirst,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: None,
        };

        let value = serde_json::to_value(policy).unwrap();
        assert_eq!(value["browserHost"], "docker_headed");
        assert_eq!(value["viewStream"], "virtual_display_webrtc");
        assert_eq!(value["controlInput"], "webrtc_input");
        assert_eq!(value["interactionMode"], "human_like_input");
        assert_eq!(value["rateLimit"]["minActionDelayMs"], 250);
        assert_eq!(value["challengePolicy"], "avoid_first");
    }

    #[test]
    fn service_state_round_trips_nested_entities() {
        let state = ServiceState {
            control_plane: Some(ControlPlaneSnapshot {
                worker_state: "Ready".to_string(),
                browser_health: "Ready".to_string(),
                queue_depth: 0,
                queue_capacity: 256,
                service_job_timeout_ms: Some(5000),
                updated_at: Some("2026-04-22T00:00:00Z".to_string()),
            }),
            reconciliation: Some(ServiceReconciliationSnapshot {
                last_reconciled_at: Some("2026-04-22T00:01:00Z".to_string()),
                last_error: None,
                browser_count: 1,
                changed_browsers: 0,
            }),
            events: vec![ServiceEvent {
                id: "event-1".to_string(),
                timestamp: "2026-04-22T00:01:00Z".to_string(),
                kind: ServiceEventKind::Reconciliation,
                message: "Reconciled 1 browser records, 0 changed".to_string(),
                details: Some(json!({"browserCount": 1, "changedBrowsers": 0})),
                ..ServiceEvent::default()
            }],
            profiles: BTreeMap::from([(
                "work".to_string(),
                BrowserProfile {
                    id: "work".to_string(),
                    name: "Work".to_string(),
                    persistent: true,
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("work".to_string()),
                    host: BrowserHost::LocalHeaded,
                    health: BrowserHealth::Ready,
                    pid: Some(42),
                    cdp_endpoint: Some("http://127.0.0.1:9222".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    owner: ServiceActor::Agent("codex".to_string()),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["browser-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    lifecycle: TabLifecycle::Ready,
                    url: Some("https://example.com".to_string()),
                    ..BrowserTab::default()
                },
            )]),
            jobs: BTreeMap::from([(
                "job-1".to_string(),
                ServiceJob {
                    id: "job-1".to_string(),
                    action: "navigate".to_string(),
                    service_name: Some("JournalDownloader".to_string()),
                    agent_name: Some("article-probe-agent".to_string()),
                    task_name: Some("probeACSwebsite".to_string()),
                    target: JobTarget::Tab("tab-1".to_string()),
                    result: Some(json!({"ok": true})),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };

        let encoded = serde_json::to_string(&state).unwrap();
        let decoded: ServiceState = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, state);
        assert_eq!(
            decoded
                .control_plane
                .as_ref()
                .map(|snapshot| snapshot.worker_state.as_str()),
            Some("Ready")
        );
        assert_eq!(
            decoded
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.browser_count),
            Some(1)
        );
        assert_eq!(decoded.events.len(), 1);
        assert_eq!(decoded.events[0].kind, ServiceEventKind::Reconciliation);
        assert_eq!(decoded.browsers["browser-1"].health, BrowserHealth::Ready);
        assert_eq!(
            decoded.sessions["session-1"].owner,
            ServiceActor::Agent("codex".to_string())
        );
        assert_eq!(
            decoded.jobs["job-1"].service_name.as_deref(),
            Some("JournalDownloader")
        );
        assert_eq!(
            decoded.jobs["job-1"].agent_name.as_deref(),
            Some("article-probe-agent")
        );
        assert_eq!(
            decoded.jobs["job-1"].task_name.as_deref(),
            Some("probeACSwebsite")
        );
    }

    #[test]
    fn refresh_derived_views_groups_incidents_by_browser() {
        let mut state = ServiceState {
            events: vec![
                ServiceEvent {
                    id: "event-crash".to_string(),
                    timestamp: "2026-04-22T00:02:00Z".to_string(),
                    kind: ServiceEventKind::BrowserHealthChanged,
                    message: "Browser browser-1 health changed from Ready to ProcessExited"
                        .to_string(),
                    browser_id: Some("browser-1".to_string()),
                    previous_health: Some(BrowserHealth::Ready),
                    current_health: Some(BrowserHealth::ProcessExited),
                    ..ServiceEvent::default()
                },
                ServiceEvent {
                    id: "event-recover".to_string(),
                    timestamp: "2026-04-22T00:03:00Z".to_string(),
                    kind: ServiceEventKind::BrowserHealthChanged,
                    message: "Browser browser-1 health changed from ProcessExited to Ready"
                        .to_string(),
                    browser_id: Some("browser-1".to_string()),
                    previous_health: Some(BrowserHealth::ProcessExited),
                    current_health: Some(BrowserHealth::Ready),
                    ..ServiceEvent::default()
                },
                ServiceEvent {
                    id: "event-reconcile-error".to_string(),
                    timestamp: "2026-04-22T00:04:00Z".to_string(),
                    kind: ServiceEventKind::ReconciliationError,
                    message: "Failed to reconcile service state".to_string(),
                    ..ServiceEvent::default()
                },
            ],
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    ..BrowserTab::default()
                },
            )]),
            jobs: BTreeMap::from([
                (
                    "job-cancelled".to_string(),
                    ServiceJob {
                        id: "job-cancelled".to_string(),
                        action: "navigate".to_string(),
                        target: JobTarget::Tab("tab-1".to_string()),
                        state: JobState::Cancelled,
                        completed_at: Some("2026-04-22T00:05:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                ),
                (
                    "job-timeout".to_string(),
                    ServiceJob {
                        id: "job-timeout".to_string(),
                        action: "snapshot".to_string(),
                        target: JobTarget::Service,
                        state: JobState::TimedOut,
                        completed_at: Some("2026-04-22T00:06:00Z".to_string()),
                        ..ServiceJob::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        assert_eq!(state.incidents.len(), 2);
        assert_eq!(state.incidents[0].id, "service");
        assert_eq!(state.incidents[0].state, ServiceIncidentState::Service);
        assert_eq!(state.incidents[0].latest_kind, "service_job_timeout");
        assert_eq!(state.incidents[0].event_ids, vec!["event-reconcile-error"]);
        assert_eq!(state.incidents[0].job_ids, vec!["job-timeout"]);
        assert_eq!(state.incidents[1].id, "browser-1");
        assert_eq!(state.incidents[1].browser_id.as_deref(), Some("browser-1"));
        assert_eq!(state.incidents[1].state, ServiceIncidentState::Active);
        assert_eq!(state.incidents[1].latest_kind, "service_job_cancelled");
        assert_eq!(
            state.incidents[1].event_ids,
            vec!["event-recover", "event-crash"]
        );
        assert_eq!(state.incidents[1].job_ids, vec!["job-cancelled"]);
        assert_eq!(
            state.incidents[1].current_health,
            Some(BrowserHealth::Ready)
        );
    }

    #[test]
    fn refresh_derived_views_preserves_incident_operator_metadata() {
        let mut state = ServiceState {
            incidents: vec![ServiceIncident {
                id: "browser-1".to_string(),
                acknowledged_at: Some("2026-04-22T00:09:00Z".to_string()),
                acknowledged_by: Some("operator".to_string()),
                acknowledgement_note: Some("Investigating".to_string()),
                resolved_at: Some("2026-04-22T00:10:00Z".to_string()),
                resolved_by: Some("operator".to_string()),
                resolution_note: Some("Recovered".to_string()),
                ..ServiceIncident::default()
            }],
            events: vec![ServiceEvent {
                id: "event-crash".to_string(),
                timestamp: "2026-04-22T00:02:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser browser-1 health changed from Ready to ProcessExited".to_string(),
                browser_id: Some("browser-1".to_string()),
                previous_health: Some(BrowserHealth::Ready),
                current_health: Some(BrowserHealth::ProcessExited),
                ..ServiceEvent::default()
            }],
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::ProcessExited,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        assert_eq!(state.incidents.len(), 1);
        assert_eq!(
            state.incidents[0].acknowledged_at.as_deref(),
            Some("2026-04-22T00:09:00Z")
        );
        assert_eq!(state.incidents[0].resolved_by.as_deref(), Some("operator"));
        assert_eq!(
            state.incidents[0].resolution_note.as_deref(),
            Some("Recovered")
        );
    }

    #[test]
    fn configured_entities_overlay_persisted_state() {
        let mut persisted = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "persisted".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };
        let configured = ServiceState {
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "configured".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            providers: BTreeMap::from([(
                "manual".to_string(),
                ServiceProvider {
                    id: "manual".to_string(),
                    display_name: "Dashboard approval".to_string(),
                    ..ServiceProvider::default()
                },
            )]),
            ..ServiceState::default()
        };

        persisted.overlay_configured_entities(configured);

        assert!(persisted.browsers.contains_key("browser-1"));
        assert_eq!(
            persisted.site_policies["google"].origin_pattern,
            "configured"
        );
        assert_eq!(
            persisted.providers["manual"].display_name,
            "Dashboard approval"
        );
    }

    #[test]
    fn provider_and_challenge_model_capabilities() {
        let provider = ServiceProvider {
            id: "manual".to_string(),
            kind: ProviderKind::ManualApproval,
            display_name: "Dashboard approval".to_string(),
            capabilities: vec![ProviderCapability::HumanApproval],
            ..ServiceProvider::default()
        };
        let challenge = Challenge {
            id: "challenge-1".to_string(),
            kind: ChallengeKind::TwoFactor,
            state: ChallengeState::WaitingForHuman,
            provider_id: Some(provider.id.clone()),
            policy_decision: Some("manual_only".to_string()),
            ..Challenge::default()
        };

        let provider_value = serde_json::to_value(provider).unwrap();
        let challenge_value = serde_json::to_value(challenge).unwrap();

        assert_eq!(provider_value["kind"], "manual_approval");
        assert_eq!(provider_value["capabilities"][0], "human_approval");
        assert_eq!(challenge_value["kind"], "two_factor");
        assert_eq!(challenge_value["state"], "waiting_for_human");
    }
}
