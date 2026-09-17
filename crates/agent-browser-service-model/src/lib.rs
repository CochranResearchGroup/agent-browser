//! Provider-free durable record models for Agent Browser.
//!
//! This Module owns record compatibility and deterministic lifecycle decisions.
//! Adapters own persistence, browser observation, process control, and transport.

mod profile_readiness;
mod profile_seeding;

pub use profile_readiness::{
    BrowserBuild, BrowserProfileCompatibilityEvidence, ProfileAllocationPolicy,
    ProfileKeyringPolicy, ProfileReadinessState, ProfileTargetReadiness,
    SERVICE_BROWSER_BUILD_VALUES, SERVICE_PROFILE_ALLOCATION_VALUES,
    SERVICE_PROFILE_KEYRING_VALUES, SERVICE_PROFILE_READINESS_VALUES,
};
pub use profile_seeding::{
    profile_seeding_handoff_id, ProfileSeedingHandoffRecord, ProfileSeedingHandoffState,
    ProfileSeedingMode, SERVICE_PROFILE_SEEDING_HANDOFF_STATE_VALUES,
    SERVICE_PROFILE_SEEDING_MODE_VALUES,
};
