//! Lease-authority kernel primitives.
//!
//! Product profile discovery and runtime orchestration remain in the CLI. This
//! crate owns profile identity, principal capability registration, typed lease
//! claims, authenticated operations and the protected authority protocol and
//! store. Secret signing material and mutable authority indexes remain private.

#[allow(dead_code)]
mod authority;
mod principal;
mod profile_identity;
mod runtime_owner;

pub use authority::*;
pub use principal::{
    authenticate_profile_capability, authenticated_authority_is_current,
    generate_profile_capability_token, profile_capability_digest, register_profile_capability,
    rotate_profile_capability, AuthenticatedServicePrincipal, RegisteredProfileCapability,
    RotatedProfileCapability, ServicePrincipalError, ServicePrincipalFailureCode,
    ServicePrincipalProvenance, ServicePrincipalRegistration, ServicePrincipalRegistrationRequest,
    ServicePrincipalRegistry, ServicePrincipalState, ServiceProfileCapability,
    ServiceProfileCapabilityState, SERVICE_PRINCIPAL_SCHEMA_VERSION,
    SERVICE_PROFILE_CAPABILITY_SCHEMA_VERSION,
};
pub use profile_identity::{canonical_profile_identity_digest, validate_runtime_profile_name};
pub use runtime_owner::*;
