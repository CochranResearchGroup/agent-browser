//! Emergency profile-lease behavior selected by an exact environment value.
//!
//! Normal operation remains fail closed. `fail_open_ephemeral` redirects a
//! blocked launch to an isolated disposable profile. `unsafe_claim_any` is a
//! temporary, explicitly unsafe escape hatch that lets an attributable service
//! request select an exact session/profile route across principal and lease
//! ownership boundaries. It does not disable dashboard authentication,
//! workstation upgrade admission, action policy, confirmation, or presentation
//! controller/viewer authority.

pub(crate) const PROFILE_LEASE_MODE_ENV: &str = "AGENT_BROWSER_PROFILE_LEASE_MODE";
pub(crate) const PROFILE_LEASE_FAIL_OPEN_EPHEMERAL: &str = "fail_open_ephemeral";
pub(crate) const PROFILE_LEASE_UNSAFE_CLAIM_ANY: &str = "unsafe_claim_any";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileLeaseMode {
    Enforced,
    FailOpenEphemeral,
    UnsafeClaimAny,
}

impl ProfileLeaseMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Enforced => "enforced",
            Self::FailOpenEphemeral => PROFILE_LEASE_FAIL_OPEN_EPHEMERAL,
            Self::UnsafeClaimAny => PROFILE_LEASE_UNSAFE_CLAIM_ANY,
        }
    }
}

pub(crate) fn profile_lease_mode_from_env() -> Result<ProfileLeaseMode, String> {
    match std::env::var(PROFILE_LEASE_MODE_ENV) {
        Err(std::env::VarError::NotPresent) => Ok(ProfileLeaseMode::Enforced),
        Err(error) => Err(format!("Could not read {PROFILE_LEASE_MODE_ENV}: {error}")),
        Ok(value) if value == PROFILE_LEASE_FAIL_OPEN_EPHEMERAL => {
            Ok(ProfileLeaseMode::FailOpenEphemeral)
        }
        Ok(value) if value == PROFILE_LEASE_UNSAFE_CLAIM_ANY => {
            Ok(ProfileLeaseMode::UnsafeClaimAny)
        }
        Ok(value) => Err(format!(
            "{PROFILE_LEASE_MODE_ENV} must be '{PROFILE_LEASE_FAIL_OPEN_EPHEMERAL}' or '{PROFILE_LEASE_UNSAFE_CLAIM_ANY}', got '{value}'"
        )),
    }
}
