//! Authenticated kernel operations over typed authority and principal state.
//!
//! Secret signing material and raw proof construction remain private to the
//! kernel. The CLI supplies repository custody and any runtime-owner joins.

use super::*;
use crate::ServicePrincipalRegistry;

/// Immutable inputs required to authenticate a claim operation.
pub struct LeaseAuthorityView<'a> {
    lease_authority: &'a LeaseAuthorityState,
    service_principals: &'a ServicePrincipalRegistry,
}

impl<'a> LeaseAuthorityView<'a> {
    pub fn new(
        lease_authority: &'a LeaseAuthorityState,
        service_principals: &'a ServicePrincipalRegistry,
    ) -> Self {
        Self {
            lease_authority,
            service_principals,
        }
    }

    fn lease_authority(&self) -> &LeaseAuthorityState {
        self.lease_authority
    }
}

pub fn issue_lease_effect_authorization(
    state: &LeaseAuthorityView<'_>,
    claim: &ActiveLeaseClaim,
    intent: &LeaseEffectIntent,
    raw_capability: &[u8],
) -> Result<LeaseEffectAuthorization, String> {
    let authority = crate::authenticate_profile_capability(
        state.service_principals,
        std::str::from_utf8(raw_capability)
            .map_err(|_| "lease_authority_capability_mismatch".to_string())?,
        claim.profile_id(),
    )
    .map_err(|error| format!("lease_authority_{}", error.code.as_str()))?;
    if authority.principal_id != claim.principal_id
        || authority.capability_id != claim.capability_id
        || authority.capability_revision != claim.capability_revision
    {
        return Err("lease_authority_capability_mismatch".to_string());
    }
    let signing_key = load_or_create_lease_authority_signing_key()?;
    issue_lease_effect_authorization_with_signing_key(state, claim, intent, &signing_key)
}

fn issue_lease_effect_authorization_with_signing_key(
    state: &LeaseAuthorityView<'_>,
    claim: &ActiveLeaseClaim,
    intent: &LeaseEffectIntent,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<LeaseEffectAuthorization, String> {
    validate_effect_intent(intent)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    let current = state
        .lease_authority()
        .current_claim(&claim.resource, &intent.issued_at)
        .filter(|current| {
            current.claim_id == claim.claim_id
                && current.revision == claim.revision
                && current.fencing_token == claim.fencing_token
        })
        .ok_or_else(|| "lease_authority_claim_unavailable".to_string())?;
    if !timestamp_at_or_after(&current.expires_at, &intent.authorization_expires_at) {
        return Err("lease_authority_invalid_request".to_string());
    }
    let capability = state
        .service_principals
        .profile_capabilities
        .get(current.capability_id())
        .ok_or_else(|| "lease_authority_capability_unavailable".to_string())?;
    current
        .effect_authorization(capability, intent, signing_key)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))
}

pub fn issue_lease_recovery_authorization(
    state: &LeaseAuthorityView<'_>,
    claim: &ActiveLeaseClaim,
    controller: &crate::ServiceProfileCapability,
    intent: &LeaseRecoveryIntent,
    raw_capability: &[u8],
) -> Result<LeaseRecoveryAuthorization, String> {
    let authority = crate::authenticate_profile_capability(
        state.service_principals,
        std::str::from_utf8(raw_capability)
            .map_err(|_| "lease_authority_recovery_controller_mismatch".to_string())?,
        claim.profile_id(),
    )
    .map_err(|error| format!("lease_authority_{}", error.code.as_str()))?;
    if authority.principal_id != claim.principal_id
        || authority.capability_id != controller.capability_id
        || authority.capability_revision != controller.revision
    {
        return Err("lease_authority_recovery_controller_mismatch".to_string());
    }
    let signing_key = load_or_create_lease_authority_signing_key()?;
    issue_lease_recovery_authorization_with_signing_key(
        state,
        claim,
        controller,
        intent,
        &signing_key,
    )
}

fn issue_lease_recovery_authorization_with_signing_key(
    state: &LeaseAuthorityView<'_>,
    claim: &ActiveLeaseClaim,
    controller: &crate::ServiceProfileCapability,
    intent: &LeaseRecoveryIntent,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<LeaseRecoveryAuthorization, String> {
    let current = state
        .lease_authority()
        .current_claim(&claim.resource, &intent.issued_at)
        .filter(|current| {
            current.claim_id == claim.claim_id
                && current.revision == claim.revision
                && current.fencing_token == claim.fencing_token
        })
        .ok_or_else(|| "lease_authority_claim_unavailable".to_string())?;
    let controller_id = current
        .recovery_controller_id
        .as_deref()
        .ok_or_else(|| "lease_authority_strict_recovery_required".to_string())?;
    let registered = state
        .service_principals
        .profile_capabilities
        .get(controller_id)
        .ok_or_else(|| "lease_authority_capability_unavailable".to_string())?;
    if registered != controller
        || controller.state != crate::ServiceProfileCapabilityState::Active
        || controller.principal_id != current.principal_id
        || current
            .profile_id()
            .is_some_and(|profile_id| controller.profile_id != profile_id)
    {
        return Err("lease_authority_recovery_controller_mismatch".to_string());
    }
    super::issue_lease_recovery_authorization_with_signing_key(
        state.lease_authority(),
        current,
        controller,
        intent,
        signing_key,
    )
    .map_err(|error| format!("lease_authority_{}", error.as_str()))
}

pub fn issue_lease_administrative_authorization(
    state: &LeaseAuthorityView<'_>,
    claim: &ActiveLeaseClaim,
    intent: &LeaseAdministrativeIntent,
    raw_administrator_capability: &[u8],
) -> Result<LeaseAdministrativeAuthorization, String> {
    state
        .lease_authority()
        .authenticate_administrator(
            &intent.administrator_id,
            intent.administrator_revision,
            raw_administrator_capability,
        )
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    let signing_key = load_or_create_lease_authority_signing_key()?;
    issue_lease_administrative_authorization_with_signing_key(state, claim, intent, &signing_key)
}

fn issue_lease_administrative_authorization_with_signing_key(
    state: &LeaseAuthorityView<'_>,
    claim: &ActiveLeaseClaim,
    intent: &LeaseAdministrativeIntent,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<LeaseAdministrativeAuthorization, String> {
    super::issue_lease_administrative_authorization_with_signing_key(
        state.lease_authority(),
        claim,
        intent,
        signing_key,
    )
}

pub fn authorize_lease_effect(
    state: &LeaseAuthorityView<'_>,
    authorization: &LeaseEffectAuthorization,
    now: &str,
    context: &LeaseEffectContext<'_>,
) -> Result<ActiveLeaseClaim, String> {
    if authorization.schema_version != LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::UnsupportedSchema.as_str()
        ));
    }
    let verification_key = load_existing_lease_authority_verification_key()?;
    authorize_lease_effect_with_verification_key(
        state,
        authorization,
        now,
        context,
        &verification_key,
    )
}

fn authorize_lease_effect_with_verification_key(
    state: &LeaseAuthorityView<'_>,
    authorization: &LeaseEffectAuthorization,
    now: &str,
    context: &LeaseEffectContext<'_>,
    verification_key: &LeaseAuthorityVerificationKeyring,
) -> Result<ActiveLeaseClaim, String> {
    let claim = state
        .lease_authority()
        .authorize_effect(authorization, now, context)
        .cloned()
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    let capability = state
        .service_principals
        .profile_capabilities
        .get(&claim.capability_id)
        .ok_or_else(|| {
            format!(
                "lease_authority_{}",
                LeaseAuthorityError::CapabilityUnavailable.as_str()
            )
        })?;
    if capability.state != crate::ServiceProfileCapabilityState::Active {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::CapabilityRevoked.as_str()
        ));
    }
    if capability.capability_id != claim.capability_id
        || capability.principal_id != claim.principal_id
        || capability.revision != claim.capability_revision
        || claim
            .profile_id()
            .is_some_and(|profile_id| capability.profile_id != profile_id)
    {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::CapabilityMismatch.as_str()
        ));
    }
    verify_effect_authorization(authorization, verification_key)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    Ok(claim)
}

pub fn release_lease_claim(
    authority: &mut LeaseAuthorityState,
    principals: &ServicePrincipalRegistry,
    request: ReleaseLeaseClaimRequest,
) -> Result<LeaseClaimReleaseOutcome, String> {
    if let Some(replayed) = authority
        .replay_release(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    let verification_key = load_existing_lease_authority_verification_key()?;
    release_lease_claim_with_verification_key(authority, principals, request, &verification_key)
}

fn release_lease_claim_with_verification_key(
    authority: &mut LeaseAuthorityState,
    principals: &ServicePrincipalRegistry,
    request: ReleaseLeaseClaimRequest,
    verification_key: &LeaseAuthorityVerificationKeyring,
) -> Result<LeaseClaimReleaseOutcome, String> {
    if let Some(replayed) = authority
        .replay_release(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    let release_context = LeaseEffectContext {
        action_class: "lease_release",
        audience: "lease_authority_kernel",
        operation_idempotency_key: &request.idempotency_key,
    };
    let claim = authority
        .authorize_effect(&request.authorization, &request.now, &release_context)
        .cloned()
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    let capability = principals
        .profile_capabilities
        .get(claim.capability_id())
        .ok_or_else(|| {
            format!(
                "lease_authority_{}",
                LeaseAuthorityError::CapabilityUnavailable.as_str()
            )
        })?;
    if capability.state != crate::ServiceProfileCapabilityState::Active {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::CapabilityRevoked.as_str()
        ));
    }
    if capability.capability_id != claim.capability_id
        || capability.principal_id != claim.principal_id
        || capability.revision != claim.capability_revision
        || claim
            .profile_id()
            .is_some_and(|profile_id| capability.profile_id != profile_id)
    {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::CapabilityMismatch.as_str()
        ));
    }
    authority
        .release_with_receipt(request.clone(), verification_key)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))
}

pub fn recover_lease_claim(
    authority: &mut LeaseAuthorityState,
    principals: &ServicePrincipalRegistry,
    request: RecoverLeaseClaimRequest,
) -> Result<LeaseClaimRecoveryOutcome, String> {
    if let Some(replayed) = authority
        .replay_recovery(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    let verification_key = load_existing_lease_authority_verification_key()?;
    recover_lease_claim_with_verification_key(authority, principals, request, &verification_key)
}

fn recover_lease_claim_with_verification_key(
    authority: &mut LeaseAuthorityState,
    principals: &ServicePrincipalRegistry,
    request: RecoverLeaseClaimRequest,
    verification_key: &LeaseAuthorityVerificationKeyring,
) -> Result<LeaseClaimRecoveryOutcome, String> {
    if let Some(replayed) = authority
        .replay_recovery(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    let controller = principals
        .profile_capabilities
        .get(&request.authorization.recovery_controller_id)
        .cloned()
        .ok_or_else(|| {
            format!(
                "lease_authority_{}",
                LeaseAuthorityError::CapabilityUnavailable.as_str()
            )
        })?;
    if controller.state != crate::ServiceProfileCapabilityState::Active {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::CapabilityRevoked.as_str()
        ));
    }
    if controller.capability_id != request.authorization.recovery_controller_id
        || controller.revision != request.authorization.recovery_controller_revision
        || controller.principal_id != request.authorization.principal_id
        || (request.authorization.resource.kind == LeaseResourceKind::Profile
            && controller.profile_id != request.authorization.resource.id)
    {
        return Err(format!(
            "lease_authority_{}",
            LeaseAuthorityError::RecoveryControllerMismatch.as_str()
        ));
    }
    authority
        .recover_with_receipt(request.clone(), verification_key)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))
}

pub fn revoke_lease_claim(
    authority: &mut LeaseAuthorityState,
    request: RevokeLeaseClaimRequest,
) -> Result<LeaseClaimRevocationOutcome, String> {
    if let Some(replayed) = authority
        .replay_revocation(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    let verification_key = load_existing_lease_authority_verification_key()?;
    revoke_lease_claim_with_verification_key(authority, request, &verification_key)
}

fn revoke_lease_claim_with_verification_key(
    authority: &mut LeaseAuthorityState,
    request: RevokeLeaseClaimRequest,
    verification_key: &LeaseAuthorityVerificationKeyring,
) -> Result<LeaseClaimRevocationOutcome, String> {
    if let Some(replayed) = authority
        .replay_revocation(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    authority
        .revoke_with_receipt(request.clone(), verification_key)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default, Debug, PartialEq, Eq)]
    struct ServiceState {
        lease_authority: LeaseAuthorityState,
        service_principals: ServicePrincipalRegistry,
    }
    impl ServiceState {
        fn lease_authority(&self) -> &LeaseAuthorityState {
            &self.lease_authority
        }
        fn acquire_lease_claim(
            &mut self,
            request: AcquireLeaseClaimRequest,
        ) -> Result<ActiveLeaseClaim, LeaseAuthorityError> {
            self.lease_authority.acquire(request)
        }
        fn acquire_lease_claim_with_receipt(
            &mut self,
            request: AcquireLeaseClaimRequest,
        ) -> Result<LeaseClaimAcquisitionOutcome, LeaseAuthorityError> {
            self.lease_authority.acquire_with_receipt(request)
        }
    }
    fn view(state: &ServiceState) -> LeaseAuthorityView<'_> {
        LeaseAuthorityView::new(&state.lease_authority, &state.service_principals)
    }
    #[derive(Clone, Default)]
    struct MemoryRepository {
        state: Arc<Mutex<ServiceState>>,
    }
    impl MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            self.state
                .lock()
                .map(|state| state.clone())
                .map_err(|_| "memory_repository_poisoned".to_string())
        }
        fn mutate<T>(
            &self,
            mut mutator: impl FnMut(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "memory_repository_poisoned".to_string())?;
            mutator(&mut state)
        }
    }
    fn acquire_lease_claim_in_repository(
        repository: &MemoryRepository,
        request: AcquireLeaseClaimRequest,
    ) -> Result<ActiveLeaseClaim, String> {
        repository.mutate(|state| {
            state
                .acquire_lease_claim(request.clone())
                .map_err(|error| format!("lease_authority_{}", error.as_str()))
        })
    }
    fn issue_lease_effect_authorization_for_state(
        state: &ServiceState,
        claim: &ActiveLeaseClaim,
        intent: &LeaseEffectIntent,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseEffectAuthorization, String> {
        issue_lease_effect_authorization_with_signing_key(&view(state), claim, intent, signing_key)
    }
    fn issue_lease_recovery_authorization_for_state(
        state: &ServiceState,
        claim: &ActiveLeaseClaim,
        controller: &crate::ServiceProfileCapability,
        intent: &LeaseRecoveryIntent,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseRecoveryAuthorization, String> {
        issue_lease_recovery_authorization_with_signing_key(
            &view(state),
            claim,
            controller,
            intent,
            signing_key,
        )
    }
    fn issue_lease_administrative_authorization_for_state_with_signing_key(
        state: &ServiceState,
        claim: &ActiveLeaseClaim,
        intent: &LeaseAdministrativeIntent,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseAdministrativeAuthorization, String> {
        issue_lease_administrative_authorization_with_signing_key(
            &view(state),
            claim,
            intent,
            signing_key,
        )
    }
    fn authorize_lease_effect_in_repository_with_signing_key(
        repository: &MemoryRepository,
        authorization: &LeaseEffectAuthorization,
        now: &str,
        context: &LeaseEffectContext<'_>,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<ActiveLeaseClaim, String> {
        let state = repository.load_snapshot()?;
        authorize_lease_effect_with_verification_key(
            &view(&state),
            authorization,
            now,
            context,
            &LeaseAuthorityVerificationKeyring::from_active(signing_key),
        )
    }
    fn release_lease_claim_in_repository_with_signing_key(
        repository: &MemoryRepository,
        request: ReleaseLeaseClaimRequest,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseClaimReleaseOutcome, String> {
        repository.mutate(|state| {
            release_lease_claim_with_verification_key(
                &mut state.lease_authority,
                &state.service_principals,
                request.clone(),
                &LeaseAuthorityVerificationKeyring::from_active(signing_key),
            )
        })
    }
    fn recover_lease_claim_in_repository_with_signing_key(
        repository: &MemoryRepository,
        request: RecoverLeaseClaimRequest,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseClaimRecoveryOutcome, String> {
        repository.mutate(|state| {
            recover_lease_claim_with_verification_key(
                &mut state.lease_authority,
                &state.service_principals,
                request.clone(),
                &LeaseAuthorityVerificationKeyring::from_active(signing_key),
            )
        })
    }
    fn revoke_lease_claim_in_repository_with_signing_key(
        repository: &MemoryRepository,
        request: RevokeLeaseClaimRequest,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseClaimRevocationOutcome, String> {
        repository.mutate(|state| {
            revoke_lease_claim_with_verification_key(
                &mut state.lease_authority,
                request.clone(),
                &LeaseAuthorityVerificationKeyring::from_active(signing_key),
            )
        })
    }

    #[test]
    fn public_signing_oracle_requires_the_exact_private_profile_capability() {
        let mut state = ServiceState {
            service_principals: crate::ServicePrincipalRegistry {
                profile_capabilities: BTreeMap::from([(
                    "capability:last30days-social".to_string(),
                    capability(),
                )]),
                ..crate::ServicePrincipalRegistry::default()
            },
            ..ServiceState::default()
        };
        let claim = state.acquire_lease_claim(request()).unwrap();
        let error = super::issue_lease_effect_authorization(
            &view(&state),
            &claim,
            &effect_intent("browser_launch", "session:last30days", "launch:tick-1"),
            b"wrong-private-capability-material-with-sufficient-length",
        )
        .unwrap_err();
        assert!(error.starts_with("lease_authority_capability_"));
    }

    #[test]
    fn exact_holder_release_fences_authority_and_replays_terminal_receipt() {
        let mut state = ServiceState {
            service_principals: crate::ServicePrincipalRegistry {
                profile_capabilities: BTreeMap::from([(
                    "capability:last30days-social".to_string(),
                    capability(),
                )]),
                ..crate::ServicePrincipalRegistry::default()
            },
            ..ServiceState::default()
        };
        let acquired = state.acquire_lease_claim_with_receipt(request()).unwrap();
        let claim = acquired.claim.unwrap();
        let signing_key = signing_key();
        let release_intent = effect_intent(
            "lease_release",
            "lease_authority_kernel",
            "release:last30days:tick-1",
        );
        let release_context = LeaseEffectContext {
            action_class: "lease_release",
            audience: "lease_authority_kernel",
            operation_idempotency_key: "release:last30days:tick-1",
        };
        let authorization = issue_lease_effect_authorization_for_state(
            &state,
            &claim,
            &release_intent,
            &signing_key,
        )
        .unwrap();
        let mut unrelated = request();
        unrelated.resource = LeaseResourceKey {
            kind: LeaseResourceKind::ServiceSession,
            id: "unrelated-session".to_string(),
        };
        unrelated.expected_claim_revision = 0;
        unrelated.idempotency_key = "acquire:unrelated-session".to_string();
        state.acquire_lease_claim_with_receipt(unrelated).unwrap();
        let repository = MemoryRepository {
            state: Arc::new(Mutex::new(state)),
        };
        let release = ReleaseLeaseClaimRequest {
            authorization: authorization.clone(),
            idempotency_key: "release:last30days:tick-1".to_string(),
            now: "2026-08-31T12:01:00Z".to_string(),
        };

        let before_tamper = repository.load_snapshot().unwrap();
        let mut tampered = release.clone();
        tampered.authorization.proof.replace_range(..2, "00");
        assert_eq!(
            release_lease_claim_in_repository_with_signing_key(&repository, tampered, &signing_key,),
            Err("lease_authority_invalid_effect_proof".to_string())
        );
        assert_eq!(repository.load_snapshot().unwrap(), before_tamper);

        let first = release_lease_claim_in_repository_with_signing_key(
            &repository,
            release.clone(),
            &signing_key,
        )
        .unwrap();
        assert!(!first.replayed);
        assert_eq!(first.receipt.terminal_fencing_token, 2);
        let after_release = repository.load_snapshot().unwrap();
        assert!(after_release
            .lease_authority()
            .current_claim(
                &LeaseResourceKey::profile("last30days-social"),
                release.now.as_str()
            )
            .is_none());
        assert_eq!(
            authorize_lease_effect_in_repository_with_signing_key(
                &repository,
                &authorization,
                release.now.as_str(),
                &release_context,
                &signing_key,
            ),
            Err("lease_authority_claim_unavailable".to_string())
        );

        let replayed = release_lease_claim_in_repository_with_signing_key(
            &repository,
            release,
            &LeaseAuthoritySigningKey::from_private_bytes([0x7c; 32]),
        )
        .unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.receipt, first.receipt);

        let mut next = request();
        next.expected_claim_revision = 0;
        next.idempotency_key = "acquire:last30days:tick-2".to_string();
        next.now = "2026-08-31T12:02:00Z".to_string();
        next.expires_at = "2026-08-31T12:07:00Z".to_string();
        let next_claim = acquire_lease_claim_in_repository(&repository, next).unwrap();
        assert_eq!(next_claim.fencing_token(), 3);
    }

    #[test]
    fn strict_controller_recovery_advances_the_fence_and_replays_after_controller_revocation() {
        let controller = capability();
        let signing_key = signing_key();
        let mut strict_request = request();
        strict_request.mode = LeaseClaimMode::Strict;
        strict_request.recovery_controller_id = Some(controller.capability_id.clone());
        strict_request.transition_deadline = Some("2026-08-31T12:02:00Z".to_string());
        let mut state = ServiceState {
            service_principals: crate::ServicePrincipalRegistry {
                profile_capabilities: BTreeMap::from([(
                    controller.capability_id.clone(),
                    controller.clone(),
                )]),
                ..crate::ServicePrincipalRegistry::default()
            },
            ..ServiceState::default()
        };
        let claim = state
            .acquire_lease_claim_with_receipt(strict_request)
            .unwrap()
            .claim
            .unwrap();
        let expired_plan_intent = LeaseRecoveryIntent {
            idempotency_key: "recover:last30days:expired-plan".to_string(),
            issued_at: "2026-08-31T12:05:01Z".to_string(),
            authorization_expires_at: "2026-08-31T12:06:00Z".to_string(),
            claim_expires_at: "2026-08-31T12:10:00Z".to_string(),
            transition_deadline: "2026-08-31T12:06:00Z".to_string(),
            owner_generation: Some(58),
        };
        assert_eq!(
            issue_lease_recovery_authorization_for_state(
                &state,
                &claim,
                &controller,
                &expired_plan_intent,
                &signing_key,
            ),
            Err("lease_authority_claim_unavailable".to_string())
        );
        assert_eq!(
            issue_lease_effect_authorization_for_state(
                &state,
                &claim,
                &LeaseEffectIntent {
                    action_class: "browser_launch".to_string(),
                    audience: "session:last30days".to_string(),
                    operation_idempotency_key: "launch:expired".to_string(),
                    executor_identity_digest: None,
                    issued_at: "2026-08-31T12:05:01Z".to_string(),
                    authorization_expires_at: "2026-08-31T12:06:00Z".to_string(),
                },
                &signing_key,
            ),
            Err("lease_authority_claim_unavailable".to_string())
        );
        let stale_effect_intent = LeaseEffectIntent {
            action_class: "browser_launch".to_string(),
            audience: "session:last30days".to_string(),
            operation_idempotency_key: "launch:strict-recovery".to_string(),
            executor_identity_digest: None,
            issued_at: "2026-08-31T12:00:30Z".to_string(),
            authorization_expires_at: "2026-08-31T12:02:00Z".to_string(),
        };
        let stale_effect = issue_lease_effect_authorization_for_state(
            &state,
            &claim,
            &stale_effect_intent,
            &signing_key,
        )
        .unwrap();
        let stale_effect_context = LeaseEffectContext {
            action_class: "browser_launch",
            audience: "session:last30days",
            operation_idempotency_key: "launch:strict-recovery",
        };

        let mut unrelated = request();
        unrelated.resource = LeaseResourceKey::profile("unrelated-profile");
        unrelated.idempotency_key = "acquire:unrelated:strict-recovery".to_string();
        unrelated.expected_claim_revision = 0;
        state.acquire_lease_claim_with_receipt(unrelated).unwrap();

        let intent = LeaseRecoveryIntent {
            idempotency_key: "recover:last30days:strict-1".to_string(),
            issued_at: "2026-08-31T12:00:30Z".to_string(),
            authorization_expires_at: "2026-08-31T12:02:00Z".to_string(),
            claim_expires_at: "2026-08-31T12:05:30Z".to_string(),
            transition_deadline: "2026-08-31T12:03:00Z".to_string(),
            owner_generation: Some(58),
        };
        let authorization = state
            .lease_authority
            .plan_recovery(&claim, &controller, &intent, &signing_key)
            .unwrap()
            .authorization;
        let request = RecoverLeaseClaimRequest {
            authorization,
            now: "2026-08-31T12:01:00Z".to_string(),
        };
        let proof = request.authorization.proof.clone();
        let debug = format!("{:?}", request.authorization);
        assert!(!debug.contains(&proof));
        assert!(debug.contains("[REDACTED]"));
        let repository = MemoryRepository {
            state: Arc::new(Mutex::new(state)),
        };
        let before_tamper = repository.load_snapshot().unwrap();
        let mut tampered = request.clone();
        tampered.authorization.proof.replace_range(..2, "00");
        assert_eq!(
            recover_lease_claim_in_repository_with_signing_key(&repository, tampered, &signing_key,),
            Err("lease_authority_invalid_recovery_proof".to_string())
        );
        assert_eq!(repository.load_snapshot().unwrap(), before_tamper);

        let mut tampered_plan = request.clone();
        tampered_plan.authorization.claim_expires_at = "2026-08-31T12:05:00Z".to_string();
        assert_eq!(
            recover_lease_claim_in_repository_with_signing_key(
                &repository,
                tampered_plan,
                &signing_key,
            ),
            Err("lease_authority_invalid_recovery_proof".to_string())
        );
        assert_eq!(repository.load_snapshot().unwrap(), before_tamper);

        let recovered = recover_lease_claim_in_repository_with_signing_key(
            &repository,
            request.clone(),
            &signing_key,
        )
        .unwrap();
        assert!(!recovered.replayed);
        let recovered_claim = recovered.claim.as_ref().unwrap();
        assert_eq!(recovered_claim.claim_id, claim.claim_id);
        assert_eq!(recovered_claim.revision, claim.revision + 1);
        assert_eq!(recovered_claim.fencing_token, claim.fencing_token + 1);
        assert_eq!(recovered_claim.owner_generation, Some(58));
        assert_eq!(recovered_claim.expires_at, intent.claim_expires_at);
        assert_eq!(recovered.receipt.terminal_result, "recovered");
        assert_eq!(
            authorize_lease_effect_in_repository_with_signing_key(
                &repository,
                &stale_effect,
                request.now.as_str(),
                &stale_effect_context,
                &signing_key,
            ),
            Err("lease_authority_stale_claim".to_string())
        );

        repository
            .mutate(|state| {
                state
                    .service_principals
                    .profile_capabilities
                    .get_mut(&controller.capability_id)
                    .unwrap()
                    .state = crate::ServiceProfileCapabilityState::Revoked;
                Ok(())
            })
            .unwrap();

        let replay = recover_lease_claim_in_repository_with_signing_key(
            &repository,
            request,
            &LeaseAuthoritySigningKey::from_private_bytes([0x7c; 32]),
        )
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.receipt, recovered.receipt);
        assert_eq!(replay.claim, recovered.claim);
    }

    #[test]
    fn administrative_revocation_is_exact_holder_independent_and_replayable() {
        let mut state = ServiceState {
            service_principals: crate::ServicePrincipalRegistry {
                profile_capabilities: BTreeMap::from([(
                    "capability:last30days-social".to_string(),
                    capability(),
                )]),
                ..crate::ServicePrincipalRegistry::default()
            },
            ..ServiceState::default()
        };
        state.lease_authority.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        state.lease_authority.administrators.insert(
            "administrator:local-supervisor".to_string(),
            LeaseAdministratorAuthority {
                administrator_id: "administrator:local-supervisor".to_string(),
                capability_digest: administrator_capability_digest(
                    b"local-supervisor-private-administrator-capability",
                ),
                revision: 1,
                state: LeaseAdministratorState::Active,
            },
        );
        let claim = state.acquire_lease_claim(request()).unwrap();
        let signing_key = signing_key();
        let stale_effect = issue_lease_effect_authorization_for_state(
            &state,
            &claim,
            &effect_intent("browser_launch", "session:last30days", "launch:tick-1"),
            &signing_key,
        )
        .unwrap();
        let administrative_intent = LeaseAdministrativeIntent {
            administrator_id: "administrator:local-supervisor".to_string(),
            administrator_revision: 1,
            idempotency_key: "revoke:last30days:abandoned-1".to_string(),
            reason_code: "abandoned_strict_holder".to_string(),
            issued_at: NOW.to_string(),
            authorization_expires_at: "2026-08-31T12:02:00Z".to_string(),
        };
        assert_eq!(
            super::issue_lease_administrative_authorization(
                &view(&state),
                &claim,
                &administrative_intent,
                b"wrong-private-administrator-capability-material",
            ),
            Err("lease_authority_administrative_authority_mismatch".to_string())
        );
        let offline_authorization =
            issue_lease_administrative_authorization_for_state_with_signing_key(
                &state,
                &claim,
                &administrative_intent,
                &signing_key,
            )
            .unwrap();
        let unplanned_request = RevokeLeaseClaimRequest {
            authorization: offline_authorization,
            now: "2026-08-31T12:01:00Z".to_string(),
        };
        let unplanned_repository = MemoryRepository {
            state: Arc::new(Mutex::new(state.clone())),
        };
        assert_eq!(
            revoke_lease_claim_in_repository_with_signing_key(
                &unplanned_repository,
                unplanned_request,
                &signing_key,
            ),
            Err("lease_authority_invalid_administrative_proof".to_string())
        );

        let planned = state
            .lease_authority
            .plan_administrative_revocation(
                &claim,
                &administrative_intent,
                b"local-supervisor-private-administrator-capability",
                &signing_key,
            )
            .unwrap();
        assert!(!planned.replayed);
        assert!(!planned.authorization.plan_id().is_empty());
        let revision_after_plan = state.lease_authority.revision();
        let replayed_plan = state
            .lease_authority
            .plan_administrative_revocation(
                &claim,
                &administrative_intent,
                b"local-supervisor-private-administrator-capability",
                &LeaseAuthoritySigningKey::from_private_bytes([0x7c; 32]),
            )
            .unwrap();
        assert!(replayed_plan.replayed);
        assert_eq!(replayed_plan.authorization, planned.authorization);
        assert_eq!(state.lease_authority.revision(), revision_after_plan);

        let authorization = planned.authorization;
        let proof = authorization.proof.clone();
        assert!(!format!("{authorization:?}").contains(&proof));
        let request = RevokeLeaseClaimRequest {
            authorization,
            now: "2026-08-31T12:01:00Z".to_string(),
        };
        let repository = MemoryRepository {
            state: Arc::new(Mutex::new(state)),
        };

        let before_tamper = repository.load_snapshot().unwrap();
        let mut tampered = request.clone();
        tampered.authorization.reason_code = "unreviewed_force_unlock".to_string();
        assert_eq!(
            revoke_lease_claim_in_repository_with_signing_key(&repository, tampered, &signing_key,),
            Err("lease_authority_invalid_administrative_proof".to_string())
        );
        assert_eq!(repository.load_snapshot().unwrap(), before_tamper);

        repository
            .mutate(|state| {
                state
                    .lease_authority
                    .administrators
                    .get_mut("administrator:local-supervisor")
                    .unwrap()
                    .state = LeaseAdministratorState::Revoked;
                Ok(())
            })
            .unwrap();
        let before_revoked_administrator = repository.load_snapshot().unwrap();
        assert_eq!(
            revoke_lease_claim_in_repository_with_signing_key(
                &repository,
                request.clone(),
                &signing_key,
            ),
            Err("lease_authority_administrative_authority_mismatch".to_string())
        );
        assert_eq!(
            repository.load_snapshot().unwrap(),
            before_revoked_administrator
        );

        repository
            .mutate(|state| {
                state
                    .lease_authority
                    .administrators
                    .get_mut("administrator:local-supervisor")
                    .unwrap()
                    .state = LeaseAdministratorState::Active;
                state
                    .service_principals
                    .profile_capabilities
                    .get_mut("capability:last30days-social")
                    .unwrap()
                    .state = crate::ServiceProfileCapabilityState::Revoked;
                Ok(())
            })
            .unwrap();
        let revoked = revoke_lease_claim_in_repository_with_signing_key(
            &repository,
            request.clone(),
            &signing_key,
        )
        .unwrap();
        assert!(!revoked.replayed);
        assert_eq!(revoked.receipt.operation, "revoke");
        assert_eq!(revoked.receipt.terminal_result, "revoked");
        assert_eq!(revoked.receipt.released_fencing_token, claim.fencing_token);
        assert_eq!(
            revoked.receipt.terminal_fencing_token,
            claim.fencing_token + 1
        );
        assert!(repository
            .load_snapshot()
            .unwrap()
            .lease_authority()
            .current_claim(&claim.resource, &request.now)
            .is_none());
        assert_eq!(
            authorize_lease_effect_in_repository_with_signing_key(
                &repository,
                &stale_effect,
                &request.now,
                &LeaseEffectContext {
                    action_class: "browser_launch",
                    audience: "session:last30days",
                    operation_idempotency_key: "launch:tick-1",
                },
                &signing_key,
            ),
            Err("lease_authority_claim_unavailable".to_string())
        );

        let replay = revoke_lease_claim_in_repository_with_signing_key(
            &repository,
            request,
            &LeaseAuthoritySigningKey::from_private_bytes([0x7c; 32]),
        )
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.receipt, revoked.receipt);
    }
}
