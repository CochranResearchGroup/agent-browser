//! CLI repository and runtime-owner joins for the lease authority kernel.

use crate::native::service_model::ServiceState;
use crate::native::service_store::ServiceStateRepository;
use agent_browser_lease_authority::*;

pub(crate) fn acquire_lease_claim_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: AcquireLeaseClaimRequest,
) -> Result<ActiveLeaseClaim, String> {
    repository.mutate(|state| {
        state
            .acquire_lease_claim(request.clone())
            .map_err(|error| format!("lease_authority_{}", error.as_str()))
    })
}

pub(crate) fn acquire_lease_claim_with_receipt_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: AcquireLeaseClaimRequest,
) -> Result<LeaseClaimAcquisitionOutcome, String> {
    repository.mutate(|state| {
        state
            .acquire_lease_claim_with_receipt(request.clone())
            .map_err(|error| format!("lease_authority_{}", error.as_str()))
    })
}

pub(crate) fn release_lease_claim_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: ReleaseLeaseClaimRequest,
) -> Result<LeaseClaimReleaseOutcome, String> {
    if let Some(replayed) = repository
        .load_snapshot()?
        .replay_lease_claim_release(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    repository.mutate(|state| state.release_lease_claim(request.clone()))
}

pub(crate) fn recover_lease_claim_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: RecoverLeaseClaimRequest,
) -> Result<LeaseClaimRecoveryOutcome, String> {
    if let Some(replayed) = repository
        .load_snapshot()?
        .replay_lease_claim_recovery(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    repository.mutate(|state| state.recover_lease_claim(request.clone()))
}

pub(crate) fn revoke_lease_claim_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: RevokeLeaseClaimRequest,
) -> Result<LeaseClaimRevocationOutcome, String> {
    if let Some(replayed) = repository
        .load_snapshot()?
        .replay_lease_claim_revocation(&request)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?
    {
        return Ok(replayed);
    }
    repository.mutate(|state| state.revoke_lease_claim(request.clone()))
}

pub(crate) fn authorize_lease_effect_in_repository<R: ServiceStateRepository>(
    repository: &R,
    authorization: &LeaseEffectAuthorization,
    now: &str,
    context: &LeaseEffectContext<'_>,
) -> Result<ActiveLeaseClaim, String> {
    authorization
        .validate_schema()
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    let state = repository.load_snapshot()?;
    let claim = authorize_lease_effect(&state.lease_authority_view(), authorization, now, context)?;
    if claim.resource().kind == LeaseResourceKind::Profile {
        let profile = state
            .profiles
            .get(&claim.resource().id)
            .ok_or_else(|| "lease_authority_effect_profile_missing".to_string())?;
        let profile_hint = profile
            .user_data_dir
            .as_deref()
            .ok_or_else(|| "lease_authority_effect_profile_identity_unavailable".to_string())?;
        let resolved =
            crate::runtime_profile::resolve_profile(Some(profile_hint), Some(&profile.id))?;
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(
                &resolved.user_data_dir,
            )?;
        let runtime_authority = state.profile_runtime_authority(&profile_identity_digest);
        let owner_matches = match (claim.owner_generation(), runtime_authority.owner) {
            (None, None) => true,
            (Some(expected), Some(owner)) => {
                owner.owner_generation == expected
                    && owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                    && runtime_authority.principal_binding.is_some_and(|binding| {
                        binding.owner_generation == expected
                            && binding.profile_id == claim.resource().id
                            && binding.principal_id == claim.principal_id()
                            && binding.capability_id == claim.capability_id()
                    })
            }
            _ => false,
        };
        if !owner_matches {
            return Err("lease_authority_owner_generation_stale".to_string());
        }
    }
    Ok(claim)
}

pub(crate) fn issue_lease_effect_authorization_for_state(
    state: &ServiceState,
    claim: &ActiveLeaseClaim,
    intent: &LeaseEffectIntent,
    raw_capability: &[u8],
) -> Result<LeaseEffectAuthorization, String> {
    issue_lease_effect_authorization(&state.lease_authority_view(), claim, intent, raw_capability)
}

pub(crate) fn release_lease_claim_for_authenticated_state(
    state: &mut ServiceState,
    claim: &ActiveLeaseClaim,
    intent: &LeaseEffectIntent,
    raw_capability: &[u8],
    idempotency_key: String,
    now: String,
) -> Result<LeaseClaimReleaseOutcome, String> {
    let authorization =
        issue_lease_effect_authorization_for_state(state, claim, intent, raw_capability)?;
    state.release_lease_claim(ReleaseLeaseClaimRequest {
        authorization,
        idempotency_key,
        now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::{Arc, Mutex};

    const CAPABILITY: &str = "last30days-test-effect-proof-capability";
    const NOW: &str = "2026-08-31T12:00:00Z";

    fn request() -> AcquireLeaseClaimRequest {
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

    fn capability() -> ServiceProfileCapability {
        ServiceProfileCapability {
            capability_id: "capability:last30days-social".to_string(),
            principal_id: "principal:last30days".to_string(),
            profile_id: "last30days-social".to_string(),
            capability_digest: profile_capability_digest(CAPABILITY),
            state: ServiceProfileCapabilityState::Active,
            revision: 1,
            issued_at: Some(NOW.to_string()),
        }
    }

    fn effect_intent(
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

    fn principal_registry() -> ServicePrincipalRegistry {
        ServicePrincipalRegistry {
            principals: BTreeMap::from([(
                "principal:last30days".to_string(),
                ServicePrincipalRegistration {
                    principal_id: "principal:last30days".to_string(),
                    provenance: ServicePrincipalProvenance::RegisteredCapability,
                    ..ServicePrincipalRegistration::default()
                },
            )]),
            profile_capabilities: BTreeMap::from([(
                "capability:last30days-social".to_string(),
                capability(),
            )]),
            ..ServicePrincipalRegistry::default()
        }
    }
    fn tamper_authorization(
        authorization: &LeaseEffectAuthorization,
        tamper: impl FnOnce(&mut serde_json::Value),
    ) -> LeaseEffectAuthorization {
        let mut value = serde_json::to_value(authorization).unwrap();
        tamper(&mut value);
        serde_json::from_value(value).unwrap()
    }
    #[derive(Clone, Default)]
    struct MemoryRepository {
        state: Arc<Mutex<ServiceState>>,
    }
    impl ServiceStateRepository for MemoryRepository {
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

    #[test]
    fn service_state_round_trips_active_claims_and_history_separately() {
        let mut state = crate::native::service_model::ServiceState::default();
        let claim = state.acquire_lease_claim(request()).unwrap();
        let authority = state.lease_authority().clone();

        let encoded = serde_json::to_value(&state).unwrap();
        assert_eq!(
            encoded["leaseAuthority"]["activeClaims"]
                .as_object()
                .map(serde_json::Map::len),
            Some(1)
        );
        let decoded: crate::native::service_model::ServiceState =
            serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.lease_authority(), &authority);
        assert_eq!(
            decoded
                .lease_authority()
                .current_claim(&LeaseResourceKey::profile("last30days-social"), NOW)
                .map(|current| current.claim_id()),
            Some(claim.claim_id())
        );
    }

    #[test]
    fn repository_boundary_atomically_admits_exactly_one_contender() {
        let repository = MemoryRepository::default();
        let first_repository = repository.clone();
        let second_repository = repository.clone();
        let first_request = request();
        let mut second_request = request();
        second_request.principal_id = "principal:foreign".to_string();
        second_request.capability_id = "capability:foreign".to_string();
        second_request.idempotency_key = "acquire:foreign:tick-1".to_string();

        let first = std::thread::spawn(move || {
            acquire_lease_claim_in_repository(&first_repository, first_request)
        });
        let second = std::thread::spawn(move || {
            acquire_lease_claim_in_repository(&second_repository, second_request)
        });
        let outcomes = [first.join().unwrap(), second.join().unwrap()];

        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes.iter().filter(|outcome| outcome.is_err()).count(),
            1
        );
        let state = repository.load_snapshot().unwrap();
        assert_eq!(
            serde_json::to_value(state.lease_authority()).unwrap()["activeClaims"]
                .as_object()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            serde_json::to_value(state.lease_authority()).unwrap()["events"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn effect_boundary_ignores_orphan_binding_without_owner_generation() {
        let profile_path = "/tmp/agent-browser-lease-orphan-binding";
        let resolved =
            crate::runtime_profile::resolve_profile(Some(profile_path), Some("last30days-social"))
                .unwrap();
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(
                &resolved.user_data_dir,
            )
            .unwrap();
        let mut registry = crate::runtime_owner_transfer::RuntimeOwnerRegistry::default();
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry).principal_records.insert(
            profile_identity_digest.clone(),
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                principal_id: "principal:last30days".to_string(),
                profile_id: "last30days-social".to_string(),
                profile_identity_digest,
                capability_id: "capability:last30days-social".to_string(),
                provenance:
                    agent_browser_lease_authority::ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            },
        );
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "last30days-social".to_string(),
                crate::native::service_model::BrowserProfile {
                    id: "last30days-social".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    ..crate::native::service_model::BrowserProfile::default()
                },
            )]),
            service_principals: principal_registry(),
            runtime_owner_registry: registry,
            ..ServiceState::default()
        };
        let claim = state.acquire_lease_claim(request()).unwrap();
        assert_eq!(claim.owner_generation(), None);

        let guard = crate::test_utils::EnvGuard::new(&["HOME", "USERPROFILE"]);
        let test_home = std::env::temp_dir().join(format!(
            "agent-browser-authority-orphan-binding-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&test_home).unwrap();
        guard.set("HOME", test_home.to_str().unwrap());
        guard.set("USERPROFILE", test_home.to_str().unwrap());
        let intent = effect_intent("browser_launch", "session:last30days", "launch:tick-1");
        let context = LeaseEffectContext {
            action_class: "browser_launch",
            audience: "session:last30days",
            operation_idempotency_key: "launch:tick-1",
        };
        let authorization = issue_lease_effect_authorization_for_state(
            &state,
            &claim,
            &intent,
            CAPABILITY.as_bytes(),
        )
        .unwrap();
        let repository = MemoryRepository {
            state: Arc::new(Mutex::new(state)),
        };

        let authorized =
            authorize_lease_effect_in_repository(&repository, &authorization, NOW, &context)
                .unwrap();
        assert_eq!(authorized.claim_id(), claim.claim_id());
        fs::remove_dir_all(test_home).unwrap();
    }

    #[test]
    fn effect_boundary_rejects_diverged_owner_principal_binding() {
        let profile_path = "/tmp/agent-browser-lease-owner-fence";
        let resolved =
            crate::runtime_profile::resolve_profile(Some(profile_path), Some("last30days-social"))
                .unwrap();
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(
                &resolved.user_data_dir,
            )
            .unwrap();
        let mut registry = crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
            crate::runtime_owner_transfer::ProfileOwner {
                owner_id: "owner:generation-7".to_string(),
                profile_identity_digest: profile_identity_digest.clone(),
                state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                owner_generation: 7,
                browser_id: "browser:last30days".to_string(),
                daemon_session_route: "last30days-route".to_string(),
                process_instance_digest: "a".repeat(64),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "b".repeat(64),
                target_set_digest: "c".repeat(64),
                pending_transfer: None,
                last_transition: None,
            },
        );
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry).principal_records.insert(
            profile_identity_digest.clone(),
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                principal_id: "principal:last30days".to_string(),
                profile_id: "last30days-social".to_string(),
                profile_identity_digest: profile_identity_digest.clone(),
                capability_id: "capability:last30days-social".to_string(),
                provenance:
                    agent_browser_lease_authority::ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            },
        );
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "last30days-social".to_string(),
                crate::native::service_model::BrowserProfile {
                    id: "last30days-social".to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    ..crate::native::service_model::BrowserProfile::default()
                },
            )]),
            service_principals: principal_registry(),
            runtime_owner_registry: registry,
            ..ServiceState::default()
        };
        let mut claim_request = request();
        claim_request.owner_generation = Some(7);
        let claim = state.acquire_lease_claim(claim_request).unwrap();
        let guard = crate::test_utils::EnvGuard::new(&["HOME", "USERPROFILE"]);
        let test_home = std::env::temp_dir().join(format!(
            "agent-browser-authority-adapter-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&test_home).unwrap();
        guard.set("HOME", test_home.to_str().unwrap());
        guard.set("USERPROFILE", test_home.to_str().unwrap());
        let intent = effect_intent("browser_launch", "session:last30days", "launch:tick-1");
        let context = LeaseEffectContext {
            action_class: "browser_launch",
            audience: "session:last30days",
            operation_idempotency_key: "launch:tick-1",
        };
        let authorization = issue_lease_effect_authorization_for_state(
            &state,
            &claim,
            &intent,
            CAPABILITY.as_bytes(),
        )
        .unwrap();
        let repository = MemoryRepository {
            state: Arc::new(Mutex::new(state)),
        };
        authorize_lease_effect_in_repository(&repository, &authorization, NOW, &context).unwrap();

        let wrong_audience = LeaseEffectContext {
            action_class: "browser_launch",
            audience: "session:foreign",
            operation_idempotency_key: "launch:tick-1",
        };
        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &authorization, NOW, &wrong_audience,),
            Err("lease_authority_effect_scope_mismatch".to_string())
        );
        let wrong_operation = LeaseEffectContext {
            action_class: "browser_launch",
            audience: "session:last30days",
            operation_idempotency_key: "launch:tick-2",
        };
        assert_eq!(
            authorize_lease_effect_in_repository(
                &repository,
                &authorization,
                NOW,
                &wrong_operation,
            ),
            Err("lease_authority_effect_scope_mismatch".to_string())
        );
        assert_eq!(
            authorize_lease_effect_in_repository(
                &repository,
                &tamper_authorization(&authorization, |value| value["signingKeyId"] =
                    "unknown-verification-key".into()),
                NOW,
                &context,
            ),
            Err("lease_authority_signing_key_mismatch".to_string())
        );

        let tampered_scope = tamper_authorization(&authorization, |value| {
            value["audience"] = "session:foreign".into()
        });
        assert_eq!(
            authorize_lease_effect_in_repository(
                &repository,
                &tampered_scope,
                NOW,
                &wrong_audience,
            ),
            Err("lease_authority_invalid_effect_proof".to_string())
        );

        let tampered = tamper_authorization(&authorization, |value| {
            value["proof"] = "00".repeat(64).into()
        });
        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &tampered, NOW, &context,),
            Err("lease_authority_invalid_effect_proof".to_string())
        );

        repository
            .mutate(|state| {
                state
                    .service_principals
                    .profile_capabilities
                    .get_mut("capability:last30days-social")
                    .unwrap()
                    .state = agent_browser_lease_authority::ServiceProfileCapabilityState::Revoked;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &authorization, NOW, &context,),
            Err("lease_authority_capability_revoked".to_string())
        );

        repository
            .mutate(|state| {
                state
                    .service_principals
                    .profile_capabilities
                    .get_mut("capability:last30days-social")
                    .unwrap()
                    .state = agent_browser_lease_authority::ServiceProfileCapabilityState::Active;
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .principal_records
                .get_mut(&profile_identity_digest)
                .unwrap()
                .principal_id = "principal:foreign".to_string();
                Ok(())
            })
            .unwrap();

        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &authorization, NOW, &context,),
            Err("lease_authority_owner_generation_stale".to_string())
        );
        fs::remove_dir_all(test_home).unwrap();
    }
}
