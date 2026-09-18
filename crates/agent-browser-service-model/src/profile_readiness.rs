use serde::{Deserialize, Serialize};

use crate::ProfileSeedingMode;

pub const SERVICE_BROWSER_BUILD_VALUES: [&str; 3] =
    ["stock_chrome", "stealthcdp_chromium", "cdp_free_headed"];

pub const SERVICE_PROFILE_ALLOCATION_VALUES: [&str; 5] = [
    "shared_service",
    "per_service",
    "per_site",
    "per_identity",
    "caller_supplied",
];

pub const SERVICE_PROFILE_KEYRING_VALUES: [&str; 4] = [
    "basic_password_store",
    "real_os_keychain",
    "managed_vault",
    "manual_login_profile",
];

pub const SERVICE_PROFILE_READINESS_VALUES: [&str; 6] = [
    "unknown",
    "needs_manual_seeding",
    "seeded_unknown_freshness",
    "fresh",
    "stale",
    "blocked_by_attached_devtools",
];

/// Browser build or engine variant preference for a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserBuild {
    StockChrome,
    StealthcdpChromium,
    CdpFreeHeaded,
}

impl BrowserBuild {
    /// Parse the established browser-build labels accepted by the durable
    /// configuration and state model.
    pub fn parse_label(value: &str) -> Option<Self> {
        match value.trim() {
            "stock_chrome" | "stock-chrome" | "chrome" | "google_chrome" | "google-chrome" => {
                Some(Self::StockChrome)
            }
            "stealthcdp_chromium"
            | "stealthcdp-chromium"
            | "stealth_chromium"
            | "stealth-chromium"
            | "chromium-stealthcdp" => Some(Self::StealthcdpChromium),
            "cdp_free_headed" | "cdp-free-headed" | "cdp_free" | "cdp-free" => {
                Some(Self::CdpFreeHeaded)
            }
            _ => None,
        }
    }
}

/// How a profile may be allocated or shared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAllocationPolicy {
    #[default]
    SharedService,
    PerService,
    PerSite,
    PerIdentity,
    CallerSupplied,
}

/// Browser credential-store posture for launches using a profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKeyringPolicy {
    #[default]
    BasicPasswordStore,
    RealOsKeychain,
    ManagedVault,
    ManualLoginProfile,
}

/// Profile readiness state for one target identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileReadinessState {
    #[default]
    Unknown,
    NeedsManualSeeding,
    SeededUnknownFreshness,
    Fresh,
    Stale,
    BlockedByAttachedDevtools,
}

/// Browser compatibility evidence retained on externally registered profiles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProfileCompatibilityEvidence {
    pub browser_family: Option<String>,
    pub browser_build: Option<BrowserBuild>,
    pub browser_version: Option<String>,
    pub evidence: String,
    pub observed_at: Option<String>,
    pub source: Option<String>,
}

impl BrowserProfileCompatibilityEvidence {
    /// Decode the established durable compatibility-evidence wire contract.
    pub fn decode_json(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    /// Encode the established durable compatibility-evidence wire contract.
    pub fn encode_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// No-launch view of whether a profile can satisfy a target identity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileTargetReadiness {
    pub target_service_id: String,
    pub login_id: Option<String>,
    pub state: ProfileReadinessState,
    pub manual_seeding_required: bool,
    pub evidence: String,
    pub recommended_action: String,
    pub seeding_mode: ProfileSeedingMode,
    pub cdp_attachment_allowed_during_seeding: bool,
    pub preferred_keyring: Option<ProfileKeyringPolicy>,
    pub setup_scopes: Vec<String>,
    pub last_verified_at: Option<String>,
    pub freshness_expires_at: Option<String>,
}

impl ProfileTargetReadiness {
    /// Decode the established durable target-readiness wire contract.
    pub fn decode_json(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    /// Encode the established durable target-readiness wire contract.
    pub fn encode_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Whether the row carries the existing explicit freshness-evidence signal.
    pub fn has_explicit_freshness_evidence(&self) -> bool {
        matches!(
            self.state,
            ProfileReadinessState::Fresh
                | ProfileReadinessState::Stale
                | ProfileReadinessState::BlockedByAttachedDevtools
        ) || self.last_verified_at.is_some()
            || self.freshness_expires_at.is_some()
    }
}
