use serde::{Deserialize, Serialize};

use crate::{
    BrowserBuild, BrowserProfileCompatibilityEvidence, ProfileAllocationPolicy,
    ProfileKeyringPolicy, ProfileTargetReadiness, ServiceProfileAccessPolicy,
};

pub const SERVICE_BROWSER_HOST_VALUES: [&str; 6] = [
    "local_headless",
    "local_headed",
    "docker_headed",
    "remote_headed",
    "cloud_provider",
    "attached_existing",
];

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

/// Durable profile identity and launch policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProfile {
    pub id: String,
    pub name: String,
    /// Safe, human-readable catalog description. Do not place credentials or
    /// private authentication artifacts in this field.
    pub description: Option<String>,
    /// Alternate human-facing names used by deterministic profile discovery.
    pub aliases: Vec<String>,
    /// Website origins associated with this profile, such as
    /// `https://x.com`.
    pub origins: Vec<String>,
    /// Login identity labels supported by this profile.
    pub login_ids: Vec<String>,
    /// Safe account labels intended for operator search. Raw secrets and
    /// authentication material are not allowed.
    pub account_labels: Vec<String>,
    /// Ownership boundary for this profile's user-data directory and cleanup.
    pub profile_origin: ProfileOrigin,
    /// Product-level profile class used for reuse and cleanup decisions.
    pub profile_class: ProfileClass,
    /// Revisioned authorization policy. Missing legacy values evaluate as the
    /// trusted single-user `shared-local` preset.
    pub access_policy: Option<ServiceProfileAccessPolicy>,
    pub user_data_dir: Option<String>,
    pub site_policy_ids: Vec<String>,
    /// Target sites or identity providers this profile is intended to satisfy.
    ///
    /// Examples include google, microsoft, acs, and publisher-specific login
    /// systems. These are not caller service names; they describe stored
    /// credential or login-state scope.
    pub target_service_ids: Vec<String>,
    /// Target services currently believed to have usable authenticated state.
    ///
    /// This is advisory until active auth probes can refresh it.
    pub authenticated_service_ids: Vec<String>,
    /// Account identities this profile is intended to satisfy within target
    /// sites, for example a tenant slug, email address, or username.
    pub account_ids: Vec<String>,
    pub default_browser_host: Option<BrowserHost>,
    /// Preferred browser build for this profile. This lets service routing keep
    /// Chrome-native and patched-Chromium identities in separate lanes.
    pub browser_build: Option<BrowserBuild>,
    pub allocation: ProfileAllocationPolicy,
    pub keyring: ProfileKeyringPolicy,
    pub shared_service_ids: Vec<String>,
    pub credential_provider_ids: Vec<String>,
    pub manual_login_preferred: bool,
    /// No-launch readiness rows for target services or login identities.
    ///
    /// These rows are derived from retained profile and site-policy state. They
    /// do not prove live authentication until a future probe records freshness
    /// evidence.
    pub target_readiness: Vec<ProfileTargetReadiness>,
    /// Explicit registration metadata for externally supplied profile lanes.
    pub registration: Option<BrowserProfileRegistration>,
    /// Browser family/build evidence recorded when external profiles are registered.
    pub browser_compatibility_evidence: Vec<BrowserProfileCompatibilityEvidence>,
    pub persistent: bool,
    pub tags: Vec<String>,
}

/// Ownership boundary for a service profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileOrigin {
    #[default]
    AgentBrowserOwned,
    ExternalByop,
    ExternalObserved,
}

pub const SERVICE_PROFILE_CLASS_VALUES: [&str; 4] = [
    "default",
    "managed_one_time",
    "durable_named",
    "operator_supplied",
];

/// Product-level profile class used for one-time task reuse and cleanup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileClass {
    Default,
    ManagedOneTime,
    #[default]
    DurableNamed,
    OperatorSupplied,
}

/// Explicit registration metadata for BYOP or observed external profiles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProfileRegistration {
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub target_service_ids: Vec<String>,
    pub account_ids: Vec<String>,
    pub registered_at: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitePolicySourceRecord {
    pub id: String,
    pub source: String,
    pub overrideable: bool,
    pub precedence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSourceRecord {
    pub id: String,
    pub source: String,
    pub overrideable: bool,
    pub precedence: Vec<String>,
}
