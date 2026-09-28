//! Data-only provenance carried by persisted session and profile records.
//!
//! Provenance describes the source of an identity observation. It grants no
//! admission, lease, ownership, cleanup, or runtime authority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServicePrincipalProvenance {
    RegisteredCapability,
    AuthenticatedTransport,
    #[default]
    UnprovenLegacy,
}

impl ServicePrincipalProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegisteredCapability => "registered_capability",
            Self::AuthenticatedTransport => "authenticated_transport",
            Self::UnprovenLegacy => "unproven_legacy",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_values_remain_compatible_without_authority_behavior() {
        for (value, expected) in [
            (
                ServicePrincipalProvenance::RegisteredCapability,
                "registered_capability",
            ),
            (
                ServicePrincipalProvenance::AuthenticatedTransport,
                "authenticated_transport",
            ),
            (
                ServicePrincipalProvenance::UnprovenLegacy,
                "unproven_legacy",
            ),
        ] {
            assert_eq!(value.as_str(), expected);
            assert_eq!(
                serde_json::to_string(&value).unwrap(),
                format!("\"{expected}\"")
            );
            assert_eq!(
                serde_json::from_str::<ServicePrincipalProvenance>(&format!("\"{expected}\""))
                    .unwrap(),
                value
            );
        }
    }
}
