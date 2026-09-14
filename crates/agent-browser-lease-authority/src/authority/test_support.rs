//! Private fixture construction for authority kernel tests.

use super::*;

pub(super) const NOW: &str = "2026-08-31T12:00:00Z";

pub(super) fn request() -> AcquireLeaseClaimRequest {
    AcquireLeaseClaimRequest {
        resource: LeaseResourceKey::profile("last30days-social"),
        parent_claim_id: None,
        principal_id: "principal:last30days".to_string(),
        capability_id: "capability:last30days-social".to_string(),
        capability_revision: 1,
        mode: LeaseClaimMode::Ephemeral,
        expected_claim_revision: 0,
        idempotency_key: "acquire:last30days:tick-1".to_string(),
        now: NOW.to_string(),
        expires_at: "2026-08-31T12:05:00Z".to_string(),
        transition_deadline: None,
        recovery_controller_id: None,
        boot_epoch: Some("boot-1".to_string()),
        owner_generation: None,
    }
}

pub(super) fn capability() -> crate::ServiceProfileCapability {
    crate::ServiceProfileCapability {
        capability_id: "capability:last30days-social".to_string(),
        principal_id: "principal:last30days".to_string(),
        profile_id: "last30days-social".to_string(),
        capability_digest: format!(
            "sha256:{:x}",
            Sha256::digest(b"last30days-test-effect-proof-capability")
        ),
        state: crate::ServiceProfileCapabilityState::Active,
        revision: 1,
        issued_at: Some(NOW.to_string()),
    }
}

pub(super) fn signing_key() -> LeaseAuthoritySigningKey {
    LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32])
}

pub(super) fn effect_intent(
    action_class: &str,
    audience: &str,
    operation_idempotency_key: &str,
) -> LeaseEffectIntent {
    LeaseEffectIntent {
        action_class: action_class.to_string(),
        audience: audience.to_string(),
        operation_idempotency_key: operation_idempotency_key.to_string(),
        executor_identity_digest: None,
        issued_at: NOW.to_string(),
        authorization_expires_at: "2026-08-31T12:02:00Z".to_string(),
    }
}
