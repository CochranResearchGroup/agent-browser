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
}

/// Latest persisted control-plane status snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlPlaneSnapshot {
    pub worker_state: String,
    pub browser_health: String,
    pub queue_depth: usize,
    pub queue_capacity: usize,
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
                updated_at: Some("2026-04-22T00:00:00Z".to_string()),
            }),
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
        assert_eq!(decoded.browsers["browser-1"].health, BrowserHealth::Ready);
        assert_eq!(
            decoded.sessions["session-1"].owner,
            ServiceActor::Agent("codex".to_string())
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
