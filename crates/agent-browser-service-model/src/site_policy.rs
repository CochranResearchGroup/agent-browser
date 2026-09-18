//! Provider-free site, challenge, and provider-policy records and decisions.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{BrowserBuild, BrowserHost, BrowserProfile, ControlInputProvider, ViewStreamProvider};

pub const SERVICE_INTERACTION_MODE_VALUES: [&str; 5] = [
    "cdp_direct",
    "dom_action",
    "browser_input",
    "human_like_input",
    "manual",
];
pub const SERVICE_CHALLENGE_POLICY_VALUES: [&str; 5] = [
    "avoid_first",
    "manual_only",
    "provider_allowed",
    "provider_preferred",
    "deny",
];
pub const SERVICE_PROVIDER_KIND_VALUES: [&str; 8] = [
    "browser_credentials",
    "password_manager",
    "totp",
    "sms",
    "email",
    "manual_approval",
    "intelligence",
    "captcha",
];
pub const SERVICE_PROVIDER_CAPABILITY_VALUES: [&str; 8] = [
    "password_fill",
    "passkey",
    "totp_code",
    "sms_code",
    "email_code",
    "visual_reasoning",
    "captcha_solve",
    "human_approval",
];
pub const SERVICE_CHALLENGE_KIND_VALUES: [&str; 6] = [
    "unknown",
    "captcha",
    "two_factor",
    "passkey",
    "suspicious_login",
    "blocked_flow",
];
pub const SERVICE_CHALLENGE_STATE_VALUES: [&str; 6] = [
    "detected",
    "waiting_for_provider",
    "waiting_for_human",
    "resolved",
    "failed",
    "denied",
];

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

/// Per-site access reliability and interaction policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SitePolicy {
    pub id: String,
    /// Human-readable catalog name for this website or login surface.
    pub name: String,
    /// Safe catalog description shown during discovery.
    pub description: Option<String>,
    /// Alternate names used by deterministic site and profile discovery.
    pub aliases: Vec<String>,
    pub origin_pattern: String,
    /// Additional canonical origins for this site.
    pub origins: Vec<String>,
    /// Login identity labels served by this site.
    pub login_ids: Vec<String>,
    /// Safe account labels supported by this site policy.
    pub account_labels: Vec<String>,
    /// Preferred registered profile for operator launch workflows.
    pub recommended_profile_id: Option<String>,
    /// Service or adapter clients known to consume this login.
    pub adapter_service_ids: Vec<String>,
    /// Searchable catalog tags.
    pub tags: Vec<String>,
    /// Bounded probe or freshness contract identifier.
    pub freshness_contract: Option<String>,
    /// Safe bounded troubleshooting guidance.
    pub troubleshooting: Vec<String>,
    pub browser_host: Option<BrowserHost>,
    /// Optional browser build or engine variant preference for this site.
    pub browser_build: Option<BrowserBuild>,
    pub view_stream: Option<ViewStreamProvider>,
    pub control_input: Option<ControlInputProvider>,
    /// True when this site should launch without a DevTools/CDP attachment.
    pub requires_cdp_free: bool,
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
            name: String::new(),
            description: None,
            aliases: Vec::new(),
            origin_pattern: String::new(),
            origins: Vec::new(),
            login_ids: Vec::new(),
            account_labels: Vec::new(),
            recommended_profile_id: None,
            adapter_service_ids: Vec::new(),
            tags: Vec::new(),
            freshness_contract: None,
            troubleshooting: Vec::new(),
            browser_host: None,
            browser_build: None,
            view_stream: None,
            control_input: None,
            requires_cdp_free: false,
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

/// Deterministic interaction-risk and pacing projection for a selected policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionDecision {
    pub interaction_risk: &'static str,
    pub pacing: Value,
}

/// Deterministic provider selection projection for active challenges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderDecision {
    pub auth_provider_ids: Vec<String>,
    pub challenge_provider_ids: Vec<String>,
    pub missing_challenge_capabilities: Vec<&'static str>,
    pub challenge_strategy: &'static str,
}

/// Compute the interaction-risk label and pacing projection for a site policy.
pub fn interaction_decision(site_policy: Option<&SitePolicy>) -> InteractionDecision {
    let Some(site_policy) = site_policy else {
        return InteractionDecision {
            interaction_risk: "standard",
            pacing: json!({
                "minActionDelayMs": 0,
                "jitterMs": 0,
                "cooldownMs": null,
                "maxParallelSessions": null,
                "retryBudget": null,
                "rateLimited": false,
                "jittered": false,
                "singleSessionRecommended": false,
            }),
        };
    };
    let min_action_delay_ms = site_policy.rate_limit.min_action_delay_ms.unwrap_or(0);
    let jitter_ms = site_policy.rate_limit.jitter_ms.unwrap_or(0);
    let cooldown_ms = site_policy.rate_limit.cooldown_ms;
    let max_parallel_sessions = site_policy.rate_limit.max_parallel_sessions;
    let retry_budget = site_policy.rate_limit.retry_budget;
    let rate_limited = min_action_delay_ms > 0 || cooldown_ms.unwrap_or(0) > 0;
    let jittered = jitter_ms > 0;
    let single_session_recommended = max_parallel_sessions == Some(1);
    let interaction_risk = if site_policy.manual_login_preferred
        || matches!(site_policy.interaction_mode, InteractionMode::Manual)
    {
        "manual"
    } else if matches!(
        site_policy.interaction_mode,
        InteractionMode::HumanLikeInput
    ) || rate_limited
        || jittered
        || single_session_recommended
    {
        "hardened"
    } else {
        "standard"
    };

    InteractionDecision {
        interaction_risk,
        pacing: json!({
            "minActionDelayMs": min_action_delay_ms,
            "jitterMs": jitter_ms,
            "cooldownMs": cooldown_ms,
            "maxParallelSessions": max_parallel_sessions,
            "retryBudget": retry_budget,
            "rateLimited": rate_limited,
            "jittered": jittered,
            "singleSessionRecommended": single_session_recommended,
        }),
    }
}

/// Compute provider choices and unresolved capabilities without service-state access.
pub fn provider_decision(
    selected_profile: Option<&BrowserProfile>,
    site_policy: Option<&SitePolicy>,
    challenges: &[Challenge],
    providers: &[ServiceProvider],
) -> ProviderDecision {
    let mut auth_provider_ids = providers
        .iter()
        .filter(|provider| {
            selected_profile
                .is_some_and(|profile| profile.credential_provider_ids.contains(&provider.id))
                || site_policy.is_some_and(|policy| policy.auth_providers.contains(&provider.id))
        })
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    let active_challenges = challenges
        .iter()
        .filter(|challenge| !matches!(challenge.state, ChallengeState::Resolved))
        .collect::<Vec<_>>();
    let required_capabilities = active_challenges
        .iter()
        .flat_map(|challenge| challenge_required_capabilities(challenge.kind))
        .collect::<Vec<_>>();
    let mut challenge_provider_ids = providers
        .iter()
        .filter(|provider| {
            required_capabilities
                .iter()
                .any(|capability| provider.capabilities.contains(capability))
        })
        .filter(|provider| provider_allowed_for_challenge(provider, site_policy))
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    let mut missing_challenge_capabilities = active_challenges
        .iter()
        .filter(|challenge| {
            let capabilities = challenge_required_capabilities(challenge.kind);
            !providers.iter().any(|provider| {
                provider_allowed_for_challenge(provider, site_policy)
                    && capabilities
                        .iter()
                        .any(|capability| provider.capabilities.contains(capability))
            })
        })
        .flat_map(|challenge| {
            challenge_required_capabilities(challenge.kind)
                .into_iter()
                .map(provider_capability_wire_name)
        })
        .collect::<Vec<_>>();

    auth_provider_ids.sort();
    auth_provider_ids.dedup();
    challenge_provider_ids.sort();
    challenge_provider_ids.dedup();
    missing_challenge_capabilities.sort();
    missing_challenge_capabilities.dedup();

    let challenge_strategy = match site_policy.map(|policy| policy.challenge_policy) {
        Some(ChallengePolicy::Deny) => "deny",
        _ if active_challenges.is_empty() => "none",
        Some(ChallengePolicy::ManualOnly) => "manual_only",
        Some(ChallengePolicy::ProviderPreferred) if !challenge_provider_ids.is_empty() => {
            "provider_preferred"
        }
        Some(ChallengePolicy::ProviderAllowed) if !challenge_provider_ids.is_empty() => {
            "provider_allowed"
        }
        Some(ChallengePolicy::AvoidFirst) => "avoid_first",
        _ if !missing_challenge_capabilities.is_empty() => "missing_provider",
        _ => "manual_review",
    };

    ProviderDecision {
        auth_provider_ids,
        challenge_provider_ids,
        missing_challenge_capabilities,
        challenge_strategy,
    }
}

/// Whether a provider is permitted by the selected site's challenge allow-list.
pub fn provider_allowed_for_challenge(
    provider: &ServiceProvider,
    site_policy: Option<&SitePolicy>,
) -> bool {
    site_policy
        .filter(|policy| !policy.allowed_challenge_providers.is_empty())
        .is_none_or(|policy| policy.allowed_challenge_providers.contains(&provider.id))
}

/// Required provider capabilities for a detected challenge kind.
pub fn challenge_required_capabilities(kind: ChallengeKind) -> Vec<ProviderCapability> {
    match kind {
        ChallengeKind::Captcha => vec![
            ProviderCapability::CaptchaSolve,
            ProviderCapability::VisualReasoning,
            ProviderCapability::HumanApproval,
        ],
        ChallengeKind::TwoFactor => vec![
            ProviderCapability::TotpCode,
            ProviderCapability::SmsCode,
            ProviderCapability::EmailCode,
            ProviderCapability::HumanApproval,
        ],
        ChallengeKind::Passkey => vec![
            ProviderCapability::Passkey,
            ProviderCapability::HumanApproval,
        ],
        ChallengeKind::SuspiciousLogin | ChallengeKind::BlockedFlow | ChallengeKind::Unknown => {
            vec![
                ProviderCapability::VisualReasoning,
                ProviderCapability::HumanApproval,
            ]
        }
    }
}

/// Stable wire label for a provider capability.
pub fn provider_capability_wire_name(capability: ProviderCapability) -> &'static str {
    match capability {
        ProviderCapability::PasswordFill => "password_fill",
        ProviderCapability::Passkey => "passkey",
        ProviderCapability::TotpCode => "totp_code",
        ProviderCapability::SmsCode => "sms_code",
        ProviderCapability::EmailCode => "email_code",
        ProviderCapability::VisualReasoning => "visual_reasoning",
        ProviderCapability::CaptchaSolve => "captcha_solve",
        ProviderCapability::HumanApproval => "human_approval",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_preserve_defaults_and_wire_names() {
        let policy = SitePolicy::default();
        assert_eq!(policy.interaction_mode, InteractionMode::CdpDirect);
        assert_eq!(policy.challenge_policy, ChallengePolicy::AvoidFirst);
        assert_eq!(policy.rate_limit.min_action_delay_ms, Some(0));
        assert_eq!(policy.rate_limit.jitter_ms, Some(0));
        assert!(!policy.requires_cdp_free);

        let provider = ServiceProvider::default();
        assert_eq!(provider.kind, ProviderKind::ManualApproval);
        assert!(provider.enabled);
        let challenge = Challenge::default();
        assert_eq!(challenge.kind, ChallengeKind::Unknown);
        assert_eq!(challenge.state, ChallengeState::Detected);

        let policy_value = serde_json::to_value(policy).unwrap();
        assert_eq!(policy_value["interactionMode"], "cdp_direct");
        assert_eq!(policy_value["challengePolicy"], "avoid_first");
        assert!(policy_value.get("origin_pattern").is_none());
        let provider_value = serde_json::to_value(provider).unwrap();
        assert_eq!(provider_value["kind"], "manual_approval");
        let challenge_value = serde_json::to_value(challenge).unwrap();
        assert_eq!(challenge_value["state"], "detected");
    }

    #[test]
    fn interaction_decision_preserves_manual_and_hardened_semantics() {
        let standard = interaction_decision(None);
        assert_eq!(standard.interaction_risk, "standard");
        assert_eq!(standard.pacing["rateLimited"], false);

        let manual = interaction_decision(Some(&SitePolicy {
            interaction_mode: InteractionMode::Manual,
            rate_limit: RateLimitPolicy {
                cooldown_ms: Some(50),
                ..RateLimitPolicy::default()
            },
            ..SitePolicy::default()
        }));
        assert_eq!(manual.interaction_risk, "manual");
        assert_eq!(manual.pacing["cooldownMs"], 50);

        let hardened = interaction_decision(Some(&SitePolicy {
            rate_limit: RateLimitPolicy {
                max_parallel_sessions: Some(1),
                ..RateLimitPolicy::default()
            },
            ..SitePolicy::default()
        }));
        assert_eq!(hardened.interaction_risk, "hardened");
        assert_eq!(hardened.pacing["singleSessionRecommended"], true);
    }

    #[test]
    fn provider_decision_is_sorted_deduplicated_and_policy_limited() {
        let policy = SitePolicy {
            auth_providers: vec!["auth".to_string()],
            allowed_challenge_providers: vec!["captcha".to_string()],
            challenge_policy: ChallengePolicy::ProviderAllowed,
            ..SitePolicy::default()
        };
        let challenges = vec![Challenge {
            kind: ChallengeKind::Captcha,
            ..Challenge::default()
        }];
        let providers = vec![
            ServiceProvider {
                id: "captcha".to_string(),
                capabilities: vec![ProviderCapability::CaptchaSolve],
                ..ServiceProvider::default()
            },
            ServiceProvider {
                id: "auth".to_string(),
                capabilities: vec![ProviderCapability::HumanApproval],
                ..ServiceProvider::default()
            },
            ServiceProvider {
                id: "other".to_string(),
                capabilities: vec![
                    ProviderCapability::CaptchaSolve,
                    ProviderCapability::HumanApproval,
                ],
                ..ServiceProvider::default()
            },
        ];

        let profile = BrowserProfile {
            credential_provider_ids: vec!["auth".to_string(), "auth".to_string()],
            ..BrowserProfile::default()
        };
        let decision = provider_decision(Some(&profile), Some(&policy), &challenges, &providers);
        assert_eq!(decision.auth_provider_ids, vec!["auth"]);
        assert_eq!(decision.challenge_provider_ids, vec!["captcha"]);
        assert_eq!(decision.missing_challenge_capabilities, Vec::<&str>::new());
        assert_eq!(decision.challenge_strategy, "provider_allowed");
    }

    #[test]
    fn provider_decision_preserves_missing_capability_and_deny_precedence() {
        let challenge = Challenge {
            kind: ChallengeKind::TwoFactor,
            ..Challenge::default()
        };
        let missing = provider_decision(None, None, &[challenge.clone()], &[]);
        assert_eq!(missing.challenge_strategy, "missing_provider");
        assert_eq!(
            missing.missing_challenge_capabilities,
            vec!["email_code", "human_approval", "sms_code", "totp_code"]
        );

        let denied = provider_decision(
            None,
            Some(&SitePolicy {
                challenge_policy: ChallengePolicy::Deny,
                ..SitePolicy::default()
            }),
            &[challenge],
            &[],
        );
        assert_eq!(denied.challenge_strategy, "deny");
    }

    #[test]
    fn constants_and_capability_names_match_wire_variants() {
        assert_eq!(SERVICE_INTERACTION_MODE_VALUES[3], "human_like_input");
        assert_eq!(SERVICE_CHALLENGE_POLICY_VALUES[4], "deny");
        assert_eq!(SERVICE_PROVIDER_KIND_VALUES[5], "manual_approval");
        assert_eq!(SERVICE_PROVIDER_CAPABILITY_VALUES[6], "captcha_solve");
        assert_eq!(SERVICE_CHALLENGE_KIND_VALUES[2], "two_factor");
        assert_eq!(SERVICE_CHALLENGE_STATE_VALUES[2], "waiting_for_human");
        assert_eq!(
            provider_capability_wire_name(ProviderCapability::HumanApproval),
            "human_approval"
        );
        assert_eq!(
            serde_json::to_value(ChallengeKind::SuspiciousLogin).unwrap(),
            "suspicious_login"
        );
    }
}
