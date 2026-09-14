//! Pure service-principal registration and capability authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const SERVICE_PRINCIPAL_SCHEMA_VERSION: &str = "agent-browser.service-principal.v1";
pub const SERVICE_PROFILE_CAPABILITY_SCHEMA_VERSION: &str =
    "agent-browser.service-profile-capability.v1";

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServicePrincipalState {
    #[default]
    Active,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceProfileCapabilityState {
    #[default]
    Active,
    Revoked,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ServicePrincipalRegistration {
    pub principal_id: String,
    pub display_name: Option<String>,
    pub provenance: ServicePrincipalProvenance,
    pub state: ServicePrincipalState,
    pub revision: u64,
    pub registered_at: Option<String>,
    pub registered_by: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceProfileCapability {
    pub capability_id: String,
    pub principal_id: String,
    pub profile_id: String,
    pub capability_digest: String,
    pub state: ServiceProfileCapabilityState,
    pub revision: u64,
    pub issued_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ServicePrincipalRegistry {
    pub schema_version: String,
    pub profile_capability_schema_version: String,
    pub revision: u64,
    pub principals: BTreeMap<String, ServicePrincipalRegistration>,
    pub profile_capabilities: BTreeMap<String, ServiceProfileCapability>,
}

impl ServicePrincipalRegistry {
    pub fn is_empty(&self) -> bool {
        self.principals.is_empty() && self.profile_capabilities.is_empty()
    }

    fn ensure_schema_versions(&mut self) {
        if self.schema_version.is_empty() {
            self.schema_version = SERVICE_PRINCIPAL_SCHEMA_VERSION.to_string();
        }
        if self.profile_capability_schema_version.is_empty() {
            self.profile_capability_schema_version =
                SERVICE_PROFILE_CAPABILITY_SCHEMA_VERSION.to_string();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePrincipalRegistrationRequest {
    pub principal_id: String,
    pub display_name: Option<String>,
    pub profile_id: String,
    pub registered_at: Option<String>,
    pub registered_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredProfileCapability {
    pub principal: ServicePrincipalRegistration,
    pub capability: ServiceProfileCapability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotatedProfileCapability {
    pub previous_capability: ServiceProfileCapability,
    pub registered: RegisteredProfileCapability,
    pub registry_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedServicePrincipal {
    pub principal_id: String,
    pub profile_id: String,
    pub capability_id: String,
    pub capability_revision: u64,
    pub provenance: ServicePrincipalProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServicePrincipalFailureCode {
    InvalidRegistration,
    RegistrationConflict,
    RegistryRevisionMismatch,
    CapabilityRotationConflict,
    CapabilityMissing,
    CapabilityMismatch,
    CapabilityRevoked,
    PrincipalUnavailable,
    ProfileMismatch,
    WorkLeaseConflict,
}

impl ServicePrincipalFailureCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRegistration => "invalid_registration",
            Self::RegistrationConflict => "registration_conflict",
            Self::RegistryRevisionMismatch => "registry_revision_mismatch",
            Self::CapabilityRotationConflict => "capability_rotation_conflict",
            Self::CapabilityMissing => "capability_missing",
            Self::CapabilityMismatch => "capability_mismatch",
            Self::CapabilityRevoked => "capability_revoked",
            Self::PrincipalUnavailable => "principal_unavailable",
            Self::ProfileMismatch => "profile_mismatch",
            Self::WorkLeaseConflict => "work_lease_conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePrincipalError {
    pub code: ServicePrincipalFailureCode,
    pub message: &'static str,
}
pub fn generate_profile_capability_token() -> String {
    format!(
        "abpc_v1_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn register_profile_capability(
    registry: &mut ServicePrincipalRegistry,
    request: ServicePrincipalRegistrationRequest,
    raw_capability: &str,
) -> Result<RegisteredProfileCapability, ServicePrincipalError> {
    validate_registration(&request, raw_capability)?;
    let mut staged = registry.clone();
    staged.ensure_schema_versions();

    let principal = ServicePrincipalRegistration {
        principal_id: request.principal_id.clone(),
        display_name: request.display_name,
        provenance: ServicePrincipalProvenance::RegisteredCapability,
        state: ServicePrincipalState::Active,
        revision: 1,
        registered_at: request.registered_at.clone(),
        registered_by: request.registered_by,
    };
    if let Some(existing) = staged.principals.get(&request.principal_id) {
        if existing.state != ServicePrincipalState::Active
            || existing.provenance != ServicePrincipalProvenance::RegisteredCapability
        {
            return Err(principal_error(
                ServicePrincipalFailureCode::RegistrationConflict,
            ));
        }
    } else {
        staged
            .principals
            .insert(request.principal_id.clone(), principal.clone());
    }

    let capability_digest = profile_capability_digest(raw_capability);
    let capability_id = profile_capability_id(
        &request.principal_id,
        &request.profile_id,
        &capability_digest,
    );
    let capability = ServiceProfileCapability {
        capability_id: capability_id.clone(),
        principal_id: request.principal_id,
        profile_id: request.profile_id,
        capability_digest,
        state: ServiceProfileCapabilityState::Active,
        revision: 1,
        issued_at: request.registered_at,
    };
    if let Some(existing) = staged.profile_capabilities.get(&capability_id) {
        if existing != &capability {
            return Err(principal_error(
                ServicePrincipalFailureCode::RegistrationConflict,
            ));
        }
    } else if staged.profile_capabilities.values().any(|existing| {
        existing.principal_id == capability.principal_id
            && existing.profile_id == capability.profile_id
            && existing.state == ServiceProfileCapabilityState::Active
    }) {
        return Err(principal_error(
            ServicePrincipalFailureCode::RegistrationConflict,
        ));
    } else {
        staged
            .profile_capabilities
            .insert(capability_id, capability.clone());
        staged.revision = staged.revision.saturating_add(1);
    }

    let registered = RegisteredProfileCapability {
        principal: staged
            .principals
            .get(&principal.principal_id)
            .cloned()
            .expect("registered principal must exist"),
        capability,
    };
    *registry = staged;
    Ok(registered)
}

/// Rotates one exact active profile capability without accepting the lost raw
/// capability as proof. The caller must compare-and-swap both the registry
/// revision and the public capability ID. Active work is fenced by the service
/// profile-lease command before this registry-only transition is attempted.
pub fn rotate_profile_capability(
    registry: &mut ServicePrincipalRegistry,
    request: ServicePrincipalRegistrationRequest,
    expected_capability_id: &str,
    expected_registry_revision: u64,
    raw_capability: &str,
) -> Result<RotatedProfileCapability, ServicePrincipalError> {
    if registry.revision != expected_registry_revision {
        return Err(principal_error(
            ServicePrincipalFailureCode::RegistryRevisionMismatch,
        ));
    }
    let active = registry
        .profile_capabilities
        .values()
        .filter(|capability| {
            capability.principal_id == request.principal_id
                && capability.profile_id == request.profile_id
                && capability.state == ServiceProfileCapabilityState::Active
        })
        .collect::<Vec<_>>();
    if active.len() != 1 || active[0].capability_id != expected_capability_id {
        return Err(principal_error(
            ServicePrincipalFailureCode::CapabilityRotationConflict,
        ));
    }

    let mut staged = registry.clone();
    let previous_capability = staged
        .profile_capabilities
        .get_mut(expected_capability_id)
        .expect("validated capability must remain present");
    previous_capability.state = ServiceProfileCapabilityState::Revoked;
    previous_capability.revision = previous_capability.revision.saturating_add(1);
    let previous_capability = previous_capability.clone();
    staged.revision = staged.revision.saturating_add(1);
    let registered = register_profile_capability(&mut staged, request, raw_capability)?;
    let registry_revision = staged.revision;
    *registry = staged;
    Ok(RotatedProfileCapability {
        previous_capability,
        registered,
        registry_revision,
    })
}

pub fn authenticate_profile_capability(
    registry: &ServicePrincipalRegistry,
    raw_capability: &str,
    expected_profile_id: Option<&str>,
) -> Result<AuthenticatedServicePrincipal, ServicePrincipalError> {
    if raw_capability.trim().is_empty() {
        return Err(principal_error(
            ServicePrincipalFailureCode::CapabilityMissing,
        ));
    }
    let digest = profile_capability_digest(raw_capability);
    let matches = registry
        .profile_capabilities
        .values()
        .filter(|capability| capability.capability_digest == digest)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(principal_error(
            ServicePrincipalFailureCode::CapabilityMismatch,
        ));
    }
    let capability = matches[0];
    if capability.state != ServiceProfileCapabilityState::Active {
        return Err(principal_error(
            ServicePrincipalFailureCode::CapabilityRevoked,
        ));
    }
    if expected_profile_id.is_some_and(|profile_id| profile_id != capability.profile_id) {
        return Err(principal_error(
            ServicePrincipalFailureCode::ProfileMismatch,
        ));
    }
    let principal = registry
        .principals
        .get(&capability.principal_id)
        .filter(|principal| principal.state == ServicePrincipalState::Active)
        .ok_or_else(|| principal_error(ServicePrincipalFailureCode::PrincipalUnavailable))?;
    Ok(AuthenticatedServicePrincipal {
        principal_id: principal.principal_id.clone(),
        profile_id: capability.profile_id.clone(),
        capability_id: capability.capability_id.clone(),
        capability_revision: capability.revision,
        provenance: ServicePrincipalProvenance::RegisteredCapability,
    })
}

pub fn authenticated_authority_is_current(
    registry: &ServicePrincipalRegistry,
    authority: &AuthenticatedServicePrincipal,
) -> bool {
    registry
        .profile_capabilities
        .get(&authority.capability_id)
        .is_some_and(|capability| {
            authority.provenance == ServicePrincipalProvenance::RegisteredCapability
                && capability.state == ServiceProfileCapabilityState::Active
                && capability.principal_id == authority.principal_id
                && capability.profile_id == authority.profile_id
                && capability.revision == authority.capability_revision
                && registry
                    .principals
                    .get(&authority.principal_id)
                    .is_some_and(|principal| {
                        principal.state == ServicePrincipalState::Active
                            && principal.provenance
                                == ServicePrincipalProvenance::RegisteredCapability
                    })
        })
}

fn validate_registration(
    request: &ServicePrincipalRegistrationRequest,
    raw_capability: &str,
) -> Result<(), ServicePrincipalError> {
    if !valid_stable_id(&request.principal_id)
        || !valid_stable_id(&request.profile_id)
        || raw_capability.trim().len() < 32
    {
        return Err(principal_error(
            ServicePrincipalFailureCode::InvalidRegistration,
        ));
    }
    Ok(())
}

fn valid_stable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character))
}

pub fn profile_capability_digest(raw_capability: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-browser.profile-capability.v1\0");
    hasher.update(raw_capability.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn profile_capability_id(principal_id: &str, profile_id: &str, digest: &str) -> String {
    let canonical = format!("{principal_id}\0{profile_id}\0{digest}");
    format!(
        "profile-capability-v1:{}",
        digest_prefix(canonical.as_bytes())
    )
}

fn digest_prefix(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))[..24].to_string()
}

fn principal_error(code: ServicePrincipalFailureCode) -> ServicePrincipalError {
    let message = match code {
        ServicePrincipalFailureCode::InvalidRegistration => {
            "principal registration or capability is invalid"
        }
        ServicePrincipalFailureCode::RegistrationConflict => {
            "principal or profile capability conflicts with current registration"
        }
        ServicePrincipalFailureCode::RegistryRevisionMismatch => {
            "service principal registry revision changed"
        }
        ServicePrincipalFailureCode::CapabilityRotationConflict => {
            "profile capability rotation does not match the one active grant"
        }
        ServicePrincipalFailureCode::CapabilityMissing => "profile capability is missing",
        ServicePrincipalFailureCode::CapabilityMismatch => {
            "profile capability does not match a current registered grant"
        }
        ServicePrincipalFailureCode::CapabilityRevoked => "profile capability is revoked",
        ServicePrincipalFailureCode::PrincipalUnavailable => "registered principal is unavailable",
        ServicePrincipalFailureCode::ProfileMismatch => {
            "profile capability does not authorize the requested profile"
        }
        ServicePrincipalFailureCode::WorkLeaseConflict => {
            "subordinate work lease conflicts with current principal authority"
        }
    };
    ServicePrincipalError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAPABILITY: &str = "synthetic-capability-token-with-more-than-thirty-two-characters";

    fn principal_registry() -> (ServicePrincipalRegistry, AuthenticatedServicePrincipal) {
        let mut registry = ServicePrincipalRegistry::default();
        register_profile_capability(
            &mut registry,
            ServicePrincipalRegistrationRequest {
                principal_id: "principal:synthetic-service".to_string(),
                display_name: Some("Synthetic service".to_string()),
                profile_id: "synthetic-profile".to_string(),
                registered_at: Some("2026-09-01T23:00:00Z".to_string()),
                registered_by: Some("test-operator".to_string()),
            },
            CAPABILITY,
        )
        .unwrap();
        let authority =
            authenticate_profile_capability(&registry, CAPABILITY, Some("synthetic-profile"))
                .unwrap();
        (registry, authority)
    }

    #[test]
    fn registered_profile_capability_authenticates_without_persisting_secret() {
        let (registry, authority) = principal_registry();
        assert_eq!(authority.principal_id, "principal:synthetic-service");
        assert_eq!(authority.profile_id, "synthetic-profile");
        assert!(authenticated_authority_is_current(&registry, &authority));
        let serialized = serde_json::to_string(&registry).unwrap();
        assert!(!serialized.contains(CAPABILITY));
        assert!(serialized.contains("sha256:"));
    }

    #[test]
    fn rotation_revokes_one_exact_capability_under_registry_compare_and_swap() {
        let (mut registry, authority) = principal_registry();
        let before = registry.clone();
        let next_capability = "replacement-capability-token-with-more-than-thirty-two-characters";

        let mismatch = rotate_profile_capability(
            &mut registry,
            ServicePrincipalRegistrationRequest {
                principal_id: authority.principal_id.clone(),
                display_name: Some("Synthetic service".to_string()),
                profile_id: authority.profile_id.clone(),
                registered_at: Some("2026-09-01T23:00:00Z".to_string()),
                registered_by: Some("test-operator".to_string()),
            },
            &authority.capability_id,
            before.revision.saturating_add(1),
            next_capability,
        )
        .unwrap_err();
        assert_eq!(
            mismatch.code,
            ServicePrincipalFailureCode::RegistryRevisionMismatch
        );
        assert_eq!(registry, before);

        let rotated = rotate_profile_capability(
            &mut registry,
            ServicePrincipalRegistrationRequest {
                principal_id: authority.principal_id.clone(),
                display_name: Some("Synthetic service".to_string()),
                profile_id: authority.profile_id.clone(),
                registered_at: Some("2026-09-01T23:00:00Z".to_string()),
                registered_by: Some("test-operator".to_string()),
            },
            &authority.capability_id,
            before.revision,
            next_capability,
        )
        .unwrap();

        assert_eq!(
            rotated.previous_capability.state,
            ServiceProfileCapabilityState::Revoked
        );
        assert_eq!(rotated.registry_revision, before.revision + 2);
        assert_ne!(
            rotated.registered.capability.capability_id,
            authority.capability_id
        );
        assert_eq!(
            authenticate_profile_capability(&registry, CAPABILITY, Some(&authority.profile_id))
                .unwrap_err()
                .code,
            ServicePrincipalFailureCode::CapabilityRevoked
        );
        let replacement = authenticate_profile_capability(
            &registry,
            next_capability,
            Some(&authority.profile_id),
        )
        .unwrap();
        assert_eq!(
            replacement.capability_id,
            rotated.registered.capability.capability_id
        );
    }

    #[test]
    fn failed_registration_does_not_partially_mutate_registry() {
        let (mut registry, authority) = principal_registry();
        let before = registry.clone();
        let error = register_profile_capability(
            &mut registry,
            ServicePrincipalRegistrationRequest {
                principal_id: authority.principal_id,
                display_name: Some("Synthetic service".to_string()),
                profile_id: authority.profile_id,
                registered_at: Some("2026-08-27T00:20:00Z".to_string()),
                registered_by: Some("local-operator".to_string()),
            },
            "different-capability-token-with-more-than-thirty-two-characters",
        )
        .unwrap_err();

        assert_eq!(
            error.code,
            ServicePrincipalFailureCode::RegistrationConflict
        );
        assert_eq!(registry, before);
    }
}
