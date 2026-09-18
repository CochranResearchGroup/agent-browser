//! CLI compatibility facade for runtime-owner custody.
//!
//! Canonical provider-neutral values and transitions live in Lease Authority.
//! This module retains only Service State repository joins, CLI action
//! admission policy, and user-facing error presentation.

#[cfg(test)]
use crate::native::service_principal::ServicePrincipalProvenance;
use crate::native::service_store::ServiceStateRepository;
#[cfg(test)]
use serde::Deserialize;

#[allow(unused_imports)]
pub(crate) use agent_browser_lease_authority::{
    owner_binding_in_registry, validate_profile_owner, BrowserAdoptionMode,
    CandidateOwnerAttachment, CleanupObligationState, OwnerAuthorityClaim, OwnerTransferError,
    OwnerTransferFailureCode, OwnerTransferProposal, OwnerTransferReceipt, OwnerTransferRequest,
    OwnerTransferTransitionKind, OwnerTransitionRecord, ProfileOwner, ProfileOwnerRollbackSnapshot,
    ProfileOwnerState, ReverseOwnerTransferRequest, RuntimeLaneLifecycleState,
    RuntimeLifecycleRecord, RuntimeOwnerAttestation, RuntimeOwnerBinding,
    RuntimeOwnerHandoffReceiptAttestation, RuntimeOwnerPrincipalBinding, RuntimeOwnerRegistry,
};

/// Wire-shaped fixture data for exercising legacy and deliberately invalid
/// registry snapshots without exposing production mutation escape hatches.
#[cfg(test)]
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct RuntimeOwnerRegistryFixture {
    #[serde(rename = "revision")]
    pub(crate) registry_revision: u64,
    #[serde(rename = "owners")]
    pub(crate) owner_records: std::collections::BTreeMap<String, ProfileOwner>,
    #[serde(rename = "principalBindings")]
    pub(crate) principal_records: std::collections::BTreeMap<String, RuntimeOwnerPrincipalBinding>,
    #[serde(rename = "lifecycleRecords")]
    pub(crate) lifecycle_rows: std::collections::BTreeMap<String, RuntimeLifecycleRecord>,
}

#[cfg(test)]
impl RuntimeOwnerRegistryFixture {
    pub(crate) fn into_registry(self) -> RuntimeOwnerRegistry {
        serde_json::from_value(serde_json::to_value(self).unwrap()).unwrap()
    }
}

/// A test-only editable wire snapshot that writes back through the production
/// decoder when the fixture edit ends. It cannot escape into a normal build.
#[cfg(test)]
pub(crate) struct RuntimeOwnerRegistryFixtureEdit<'a> {
    registry: &'a mut RuntimeOwnerRegistry,
    fixture: RuntimeOwnerRegistryFixture,
}

#[cfg(test)]
impl std::ops::Deref for RuntimeOwnerRegistryFixtureEdit<'_> {
    type Target = RuntimeOwnerRegistryFixture;

    fn deref(&self) -> &Self::Target {
        &self.fixture
    }
}

#[cfg(test)]
impl std::ops::DerefMut for RuntimeOwnerRegistryFixtureEdit<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixture
    }
}

#[cfg(test)]
impl Drop for RuntimeOwnerRegistryFixtureEdit<'_> {
    fn drop(&mut self) {
        *self.registry =
            serde_json::from_value(serde_json::to_value(&self.fixture).unwrap()).unwrap();
    }
}

#[cfg(test)]
pub(crate) fn edit_registry_fixture(
    registry: &mut RuntimeOwnerRegistry,
) -> RuntimeOwnerRegistryFixtureEdit<'_> {
    let fixture = serde_json::from_value(serde_json::to_value(&*registry).unwrap()).unwrap();
    RuntimeOwnerRegistryFixtureEdit { registry, fixture }
}

pub(crate) fn owner_authority_is_current(
    repository: &impl ServiceStateRepository,
    claim: &OwnerAuthorityClaim,
) -> Result<bool, String> {
    Ok(repository
        .load_snapshot()?
        .runtime_owner_registry
        .authorizes(claim))
}

pub(crate) fn owner_binding_for_session(
    repository: &impl ServiceStateRepository,
    session_id: &str,
) -> Result<Option<RuntimeOwnerBinding>, String> {
    let snapshot = repository.load_snapshot()?;
    owner_binding_in_registry(&snapshot.runtime_owner_registry, session_id)
}

/// Return whether an action must hydrate and validate runtime owner authority.
/// Read-only actions stay independent of the user-scoped owner registry so
/// observation remains available during transfer and in isolated test runs.
pub(crate) fn action_requires_owner_effect_authority(action: &str) -> bool {
    action_requires_runtime_admission(action)
        && !matches!(
            action,
            "service_profile_recovery_apply"
                | "service_profile_repair_apply"
                | "service_profile_reset_apply"
        )
}

/// Return whether an action is effect-capable and must respect the runtime
/// admission drain. Profile recovery apply owns its authority through a sealed
/// recovery plan and exact graph compare-and-swap, so it bypasses only the
/// stale daemon-owner fence while remaining an admitted runtime effect.
pub(crate) fn action_requires_runtime_admission(action: &str) -> bool {
    !action_is_observation_only(action)
}

fn action_is_observation_only(action: &str) -> bool {
    matches!(
        action,
        "browser_pid"
            | "cdp_url"
            | "confirm"
            | "console"
            | "content"
            | "cookies_get"
            | "dependent_batch"
            | "diagnostics"
            | "desktop_evidence_observe"
            | "desktop_prompt_observe"
            | "deny"
            | "errors"
            | "inspect"
            | "probe"
            | "runtime_handoff_finalize"
            | "service_browser_capability_preflight"
            | "service_browser_capability_preference_guide"
            | "service_browsers"
            | "service_challenges"
            | "service_events"
            | "service_incident_activity"
            | "service_incidents"
            | "service_jobs"
            | "service_monitors"
            | "service_profile_lookup"
            | "service_profile_seeding_handoff"
            | "service_profiles"
            | "service_providers"
            | "service_remote_view_handoff_resolve"
            | "service_remote_view_route_preflight"
            | "service_resources"
            | "service_resources_monitor_summary"
            | "service_sessions"
            | "service_site_policies"
            | "service_status"
            | "service_tabs"
            | "service_trace"
            | "screenshot"
            | "snapshot"
            | "storage_get"
            | "tab_list"
            | "title"
            | "url"
    )
}

fn owner_transfer_error_text(error: OwnerTransferError) -> String {
    format!("runtime_owner_transfer_{:?}: {}", error.code, error.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::runtime_lifecycle::RuntimeLifecycleAuthority;
    use crate::native::service_model::ServiceState;
    use crate::native::service_store::{ServiceStateRepository, ServiceStateStore};
    use std::sync::Mutex;

    const OWNER_TRANSFER_FIXTURES: &str =
        include_str!("../../docs/dev/fixtures/runtime-adoption/owner-transfer.v1.json");
    const P117_CONFIRMED_GAPS: &str =
        include_str!("../../docs/dev/fixtures/runtime-lifecycle/confirmed-gaps.v1.json");

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct OwnerTransferFixtureCorpus {
        schema_version: String,
        fixtures: Vec<OwnerTransferFixture>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct OwnerTransferFixture {
        fixture_id: String,
        mode: BrowserAdoptionMode,
        initial_owner_state: String,
        previous_owner_generation: u64,
        candidate_owner_generation: u64,
        candidate_observation_effect_capable: bool,
        expected_authority_timeline: Vec<String>,
    }

    #[derive(Default)]
    struct MemoryRepository {
        state: Mutex<ServiceState>,
    }

    impl ServiceStateStore for MemoryRepository {
        fn load(&self) -> Result<ServiceState, String> {
            Ok(self.state.lock().unwrap().clone())
        }

        fn save(&self, state: &ServiceState) -> Result<(), String> {
            *self.state.lock().unwrap() = state.clone();
            Ok(())
        }
    }

    impl ServiceStateRepository for MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            self.load()
        }

        fn mutate<R>(
            &self,
            mut mutator: impl FnMut(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            let mut state = self.state.lock().unwrap();
            mutator(&mut state)
        }
    }

    fn authorize_action(
        authority: &RuntimeLifecycleAuthority<'_, MemoryRepository>,
        binding: &mut RuntimeOwnerBinding,
        action: &str,
    ) -> Result<(), String> {
        if action_is_observation_only(action) {
            Ok(())
        } else {
            authority.authorize_effect(binding)
        }
    }
    use sha2::Digest;

    fn digest(seed: &str) -> String {
        format!("{:x}", sha2::Sha256::digest(seed.as_bytes()))
    }

    fn owner() -> ProfileOwner {
        ProfileOwner {
            owner_id: "owner-old".to_string(),
            profile_identity_digest: digest("profile"),
            state: ProfileOwnerState::Ready,
            owner_generation: 7,
            browser_id: "browser-a".to_string(),
            daemon_session_route: "session-old".to_string(),
            process_instance_digest: digest("process"),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: digest("cdp"),
            target_set_digest: digest("targets"),
            pending_transfer: None,
            last_transition: None,
        }
    }

    fn cooperative_request() -> OwnerTransferRequest {
        OwnerTransferRequest {
            mode: BrowserAdoptionMode::CooperativeTransfer,
            logical_browser_id: "browser-a".to_string(),
            profile_identity_digest: digest("profile"),
            expected_owner_id: Some("owner-old".to_string()),
            expected_owner_generation: 7,
            candidate_owner_id: "owner-new".to_string(),
            candidate_daemon_session_route: "session-new".to_string(),
            process_instance_digest: digest("process"),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: digest("cdp"),
            target_set_digest: digest("targets"),
            selected_target_identity_digest: digest("target-a"),
            transfer_nonce_digest: digest("transfer"),
        }
    }

    #[test]
    fn legacy_owner_registry_loads_with_conservative_lifecycle_defaults() {
        let legacy = serde_json::json!({
            "revision": 1,
            "owners": {
                digest("profile"): owner()
            }
        });

        let registry: RuntimeOwnerRegistry = serde_json::from_value(legacy).unwrap();
        let encoded = serde_json::to_value(registry).unwrap();

        assert!(encoded.get("lifecycleRecords").is_none());
        assert!(encoded.get("principalBindings").is_none());
    }

    #[test]
    fn principal_binding_requires_the_exact_ready_owner_generation() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let mut binding = RuntimeOwnerPrincipalBinding {
            principal_id: "principal:odollo-fulfillment".to_string(),
            profile_id: "odollo-fulfillment".to_string(),
            profile_identity_digest: digest("profile"),
            capability_id: "profile-capability-v1:synthetic".to_string(),
            provenance: ServicePrincipalProvenance::RegisteredCapability,
            owner_generation: 6,
        };

        let mismatch = registry
            .bind_principal_authority(binding.clone())
            .unwrap_err();
        assert_eq!(
            mismatch.code,
            OwnerTransferFailureCode::OwnerCompareAndSwapMismatch
        );
        assert!(registry.principal_bindings().is_empty());

        binding.owner_generation = 7;
        let committed = registry.bind_principal_authority(binding).unwrap();
        assert!(registry.principal_binding_is_current(Some(&committed)));

        let mut conflicting = committed.clone();
        conflicting.principal_id = "principal:foreign".to_string();
        let conflict = registry.bind_principal_authority(conflicting).unwrap_err();
        assert_eq!(
            conflict.code,
            OwnerTransferFailureCode::OwnerCompareAndSwapMismatch
        );
        assert_eq!(
            registry.principal_bindings()[&digest("profile")].principal_id,
            "principal:odollo-fulfillment"
        );
    }

    #[test]
    fn principal_binding_refresh_requires_same_capability_and_current_owner() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let binding = RuntimeOwnerPrincipalBinding {
            principal_id: "principal:odollo-fulfillment".to_string(),
            profile_id: "odollo-fulfillment".to_string(),
            profile_identity_digest: digest("profile"),
            capability_id: "profile-capability-v1:synthetic".to_string(),
            provenance: ServicePrincipalProvenance::RegisteredCapability,
            owner_generation: 7,
        };
        registry.bind_principal_authority(binding.clone()).unwrap();
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .owner_records
            .get_mut(&digest("profile"))
            .unwrap()
            .owner_generation = 8;

        let mut refreshed = binding.clone();
        refreshed.owner_generation = 8;
        let committed = registry
            .refresh_principal_authority(refreshed.clone())
            .unwrap();
        assert_eq!(committed, refreshed);
        assert!(registry.principal_binding_is_current(Some(&committed)));

        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .owner_records
            .get_mut(&digest("profile"))
            .unwrap()
            .owner_generation = 9;
        let mut foreign = refreshed;
        foreign.owner_generation = 9;
        foreign.capability_id = "profile-capability-v1:foreign".to_string();
        let error = registry.refresh_principal_authority(foreign).unwrap_err();
        assert_eq!(
            error.code,
            OwnerTransferFailureCode::OwnerCompareAndSwapMismatch
        );
        assert_eq!(
            registry.principal_bindings()[&digest("profile")].capability_id,
            "profile-capability-v1:synthetic"
        );
    }

    #[test]
    fn lifecycle_and_cleanup_obligation_schema_round_trips_without_effects() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .lifecycle_rows
            .insert(
                "browser-a".to_string(),
                RuntimeLifecycleRecord {
                    logical_browser_id: "browser-a".to_string(),
                    boot_epoch: None,
                    profile_identity_digest: digest("profile"),
                    owner_generation: 7,
                    lifecycle_state: RuntimeLaneLifecycleState::Retained,
                    cleanup_obligation_state: CleanupObligationState::Owned,
                    process_group_id: Some(4100),
                    package_launch_identity_digest: Some(digest("launch")),
                    terminal_evidence: Vec::new(),
                },
            );

        let encoded = serde_json::to_string(&registry).unwrap();
        let decoded: RuntimeOwnerRegistry = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, registry);
        assert_eq!(
            decoded.lifecycle_records()["browser-a"].lifecycle_state,
            RuntimeLaneLifecycleState::Retained
        );
        assert_eq!(
            decoded.lifecycle_records()["browser-a"].cleanup_obligation_state,
            CleanupObligationState::Owned
        );
    }

    #[test]
    fn p117_slice_a_fixture_ledger_freezes_all_confirmed_gaps() {
        let corpus: serde_json::Value = serde_json::from_str(P117_CONFIRMED_GAPS).unwrap();
        assert_eq!(
            corpus["schemaVersion"],
            "agent-browser.runtime-lifecycle-red-fixtures.v1"
        );
        let fixtures = corpus["fixtures"].as_array().unwrap();
        assert_eq!(fixtures.len(), 6);
        let fixture_ids = fixtures
            .iter()
            .filter_map(|fixture| fixture["fixtureId"].as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(fixture_ids.len(), 6);
        assert!(fixtures.iter().all(|fixture| {
            fixture["sourceAnchors"]
                .as_array()
                .is_some_and(|anchors| !anchors.is_empty())
                && fixture["sanitizedEvidence"].is_object()
                && fixture["currentBehavior"].is_string()
                && fixture["requiredBehavior"].is_string()
                && fixture["expectedRedReason"].is_string()
        }));
    }

    #[test]
    fn handoff_fixture_moves_cleanup_accountability_with_owner_generation() {
        let repository = MemoryRepository::default();
        repository
            .mutate(|state| {
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(owner());
                Ok(())
            })
            .unwrap();
        let request = cooperative_request();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        authority.begin_transfer(request.clone()).unwrap();
        authority
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();
        let registry = repository.load_snapshot().unwrap().runtime_owner_registry;

        assert_eq!(
            registry.owner(&digest("profile")).unwrap().owner_generation,
            8
        );
        let lifecycle = &registry.lifecycle_records()["browser-a"];
        assert_eq!(lifecycle.owner_generation, 8);
        assert_eq!(lifecycle.lifecycle_state, RuntimeLaneLifecycleState::Ready);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
    }

    #[test]
    fn cooperative_transfer_keeps_old_authority_until_candidate_commit() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let profile_digest = digest("profile");
        registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: "principal:odollo-fulfillment".to_string(),
                profile_id: "odollo-fulfillment".to_string(),
                profile_identity_digest: profile_digest.clone(),
                capability_id: "profile-capability-v1:synthetic".to_string(),
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            })
            .unwrap();
        // Preserve compatibility with registries affected by the historical
        // transfer bug, where the principal authority lagged several otherwise
        // valid owner generations.
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .principal_records
            .get_mut(&profile_digest)
            .unwrap()
            .owner_generation = 5;
        let request = cooperative_request();

        let proposal = registry.begin_transfer(request.clone()).unwrap();
        assert_eq!(proposal.previous_owner_generation, 7);
        assert_eq!(proposal.candidate_owner_generation, 8);
        assert!(!proposal.candidate_effect_capable);
        assert!(registry.authorizes(&OwnerAuthorityClaim::from_owner(
            registry.owner(&request.profile_identity_digest).unwrap()
        )));

        let receipt = registry
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();
        assert_eq!(receipt.previous_owner_generation, 7);
        assert_eq!(receipt.candidate_owner_generation, 8);
        let committed = registry.owner(&request.profile_identity_digest).unwrap();
        assert_eq!(committed.browser_id, request.logical_browser_id);
        assert_eq!(
            committed.process_instance_digest,
            request.process_instance_digest
        );
        assert_eq!(committed.target_set_digest, request.target_set_digest);
        assert!(!registry.authorizes(&OwnerAuthorityClaim {
            owner_id: "owner-old".to_string(),
            profile_identity_digest: digest("profile"),
            owner_generation: 7,
            logical_browser_id: "browser-a".to_string(),
            daemon_session_route: "session-old".to_string(),
            process_instance_digest: digest("process"),
        }));
        assert!(registry.authorizes(&OwnerAuthorityClaim::from_owner(
            registry.owner(&request.profile_identity_digest).unwrap()
        )));
        assert_eq!(
            registry.principal_bindings()[&profile_digest].owner_generation,
            8
        );
        assert!(registry
            .principal_binding_is_current(registry.principal_bindings().get(&profile_digest)));
    }

    #[test]
    fn current_owner_attestation_requires_and_hashes_current_handoff_receipt() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let before = registry
            .attestation_for_session("session-old")
            .unwrap()
            .unwrap();
        assert!(before.effect_capable);
        assert!(before.handoff_receipt.is_none());

        let request = cooperative_request();
        registry.begin_transfer(request.clone()).unwrap();
        let receipt = registry
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();

        let current = registry
            .attestation_for_session("session-new")
            .unwrap()
            .unwrap();
        let handoff = current.handoff_receipt.unwrap();
        assert!(current.effect_capable);
        assert_eq!(current.owner_generation, 8);
        assert_eq!(handoff.receipt_id, receipt.receipt_id);
        assert_eq!(handoff.owner_generation, 8);
        assert_eq!(handoff.state, "accepted");
        assert_eq!(handoff.receipt_sha256.len(), 64);

        let superseded = registry
            .attestation_for_session("session-old")
            .unwrap()
            .unwrap();
        assert!(!superseded.effect_capable);
    }

    #[test]
    fn commit_replay_is_idempotent_and_does_not_advance_generation_twice() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let request = cooperative_request();
        registry.begin_transfer(request.clone()).unwrap();
        let attachment = CandidateOwnerAttachment::from_request(&request, 8);

        let first = registry.commit_candidate(attachment.clone()).unwrap();
        let replay = registry.commit_candidate(attachment).unwrap();

        assert_eq!(first, replay);
        assert_eq!(
            registry.owner(&digest("profile")).unwrap().owner_generation,
            8
        );
    }

    #[test]
    fn precommit_abort_is_exact_idempotent_and_preserves_old_authority() {
        let repository = MemoryRepository::default();
        repository
            .mutate(|state| {
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(owner());
                Ok(())
            })
            .unwrap();
        let request = cooperative_request();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        authority.begin_transfer(request.clone()).unwrap();

        assert!(authority
            .abort_transfer(
                &request.profile_identity_digest,
                request.expected_owner_id.as_deref().unwrap(),
                request.expected_owner_generation,
                &request.transfer_nonce_digest,
            )
            .unwrap());
        assert!(!authority
            .abort_transfer(
                &request.profile_identity_digest,
                request.expected_owner_id.as_deref().unwrap(),
                request.expected_owner_generation,
                &request.transfer_nonce_digest,
            )
            .unwrap());
        let persisted = repository.load_snapshot().unwrap();
        let owner = persisted
            .runtime_owner_registry
            .owner(&request.profile_identity_digest)
            .unwrap();
        assert!(owner.pending_transfer.is_none());
        assert!(persisted
            .runtime_owner_registry
            .authorizes(&OwnerAuthorityClaim::from_owner(owner)));
    }

    #[test]
    fn orphan_adoption_uses_the_same_compare_and_swap_seam() {
        let mut registry = RuntimeOwnerRegistry::default();
        let mut request = cooperative_request();
        request.mode = BrowserAdoptionMode::OrphanAdoption;
        request.expected_owner_id = None;
        request.expected_owner_generation = 0;
        request.candidate_owner_id = "owner-adopter".to_string();
        request.candidate_daemon_session_route = "session-adopter".to_string();

        let proposal = registry.begin_transfer(request.clone()).unwrap();
        assert_eq!(proposal.previous_owner_generation, 0);
        assert!(!registry.authorizes(&OwnerAuthorityClaim::from_owner(
            registry.owner(&digest("profile")).unwrap()
        )));
        let receipt = registry
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 1))
            .unwrap();

        assert_eq!(receipt.mode, BrowserAdoptionMode::OrphanAdoption);
        let adopted = registry.owner(&digest("profile")).unwrap();
        assert_eq!(adopted.owner_id, "owner-adopter");
        assert_eq!(adopted.browser_id, request.logical_browser_id);
        assert_eq!(
            adopted.process_instance_digest,
            request.process_instance_digest
        );
        assert_eq!(adopted.target_set_digest, request.target_set_digest);

        registry
            .reverse_transfer(ReverseOwnerTransferRequest {
                profile_identity_digest: digest("profile"),
                expected_candidate_owner_id: "owner-adopter".to_string(),
                expected_candidate_owner_generation: 1,
                transfer_nonce_digest: digest("transfer"),
                reverse_nonce_digest: digest("orphan-reverse"),
            })
            .unwrap();
        let restored = registry.owner(&digest("profile")).unwrap();
        assert_eq!(restored.state, ProfileOwnerState::Orphaned);
        assert!(!registry.authorizes(&OwnerAuthorityClaim::from_owner(restored)));
    }

    #[test]
    fn orphan_adoption_rebinds_historical_logical_id_only_with_exact_identity() {
        let mut registry = RuntimeOwnerRegistry::default();
        let mut orphan = owner();
        orphan.state = ProfileOwnerState::Orphaned;
        orphan.browser_id = "browser-historical".to_string();
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .owner_records
            .insert(orphan.profile_identity_digest.clone(), orphan.clone());
        let mut request = cooperative_request();
        request.mode = BrowserAdoptionMode::OrphanAdoption;
        request.expected_owner_id = Some(orphan.owner_id);
        request.expected_owner_generation = orphan.owner_generation;
        request.candidate_owner_id = "owner-adopter".to_string();
        request.logical_browser_id = "browser-current".to_string();

        let mut mismatched_registry = registry.clone();
        let mut mismatched_request = request.clone();
        mismatched_request.process_instance_digest = digest("different-process");
        assert_eq!(
            mismatched_registry
                .begin_transfer(mismatched_request)
                .unwrap_err()
                .code,
            OwnerTransferFailureCode::OwnerCompareAndSwapMismatch
        );

        let proposal = registry.begin_transfer(request.clone()).unwrap();
        let receipt = registry
            .commit_candidate(CandidateOwnerAttachment::from_request(
                &request,
                proposal.candidate_owner_generation,
            ))
            .unwrap();

        assert_eq!(receipt.logical_browser_id, "browser-current");
        assert_eq!(
            registry
                .owner(&request.profile_identity_digest)
                .unwrap()
                .browser_id,
            "browser-current"
        );
    }

    #[test]
    fn verified_legacy_daemon_revocation_advances_to_orphan_before_adoption() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let stale_claim =
            OwnerAuthorityClaim::from_owner(registry.owner(&digest("profile")).unwrap());

        let mismatch = registry
            .revoke_legacy_daemon_owner(
                &digest("profile"),
                "browser-a",
                "wrong-session",
                "owner-old",
                7,
            )
            .unwrap_err();
        assert_eq!(
            mismatch.code,
            OwnerTransferFailureCode::OwnerCompareAndSwapMismatch
        );
        assert!(registry.authorizes(&stale_claim));

        let generation_mismatch = registry
            .revoke_legacy_daemon_owner(
                &digest("profile"),
                "browser-a",
                "session-old",
                "owner-old",
                8,
            )
            .unwrap_err();
        assert_eq!(
            generation_mismatch.code,
            OwnerTransferFailureCode::OwnerCompareAndSwapMismatch
        );
        assert!(registry.authorizes(&stale_claim));

        let orphan = registry
            .revoke_legacy_daemon_owner(
                &digest("profile"),
                "browser-a",
                "session-old",
                "owner-old",
                7,
            )
            .unwrap();
        assert_eq!(orphan.state, ProfileOwnerState::Orphaned);
        assert_eq!(orphan.owner_generation, 8);
        assert!(!registry.authorizes(&stale_claim));

        let mut request = cooperative_request();
        request.mode = BrowserAdoptionMode::OrphanAdoption;
        request.expected_owner_id = Some(orphan.owner_id.clone());
        request.expected_owner_generation = orphan.owner_generation;
        request.candidate_owner_id = "owner-adopter".to_string();
        request.candidate_daemon_session_route = "session-adopter".to_string();
        request.target_set_digest = digest("targets-observed-after-revocation");
        request.transfer_nonce_digest = digest("legacy-orphan-transfer");

        let proposal = registry.begin_transfer(request.clone()).unwrap();
        assert_eq!(proposal.previous_owner_generation, 8);
        assert_eq!(proposal.candidate_owner_generation, 9);
        let receipt = registry
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 9))
            .unwrap();
        assert_eq!(receipt.mode, BrowserAdoptionMode::OrphanAdoption);
        assert_eq!(receipt.previous_owner_generation, 8);
        assert_eq!(receipt.candidate_owner_generation, 9);
        assert_eq!(
            registry
                .owner(&digest("profile"))
                .unwrap()
                .target_set_digest,
            digest("targets-observed-after-revocation")
        );
    }

    #[test]
    fn reverse_transfer_is_receipted_and_uses_a_new_generation() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let profile_digest = digest("profile");
        registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: "principal:odollo-fulfillment".to_string(),
                profile_id: "odollo-fulfillment".to_string(),
                profile_identity_digest: profile_digest.clone(),
                capability_id: "profile-capability-v1:synthetic".to_string(),
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            })
            .unwrap();
        let request = cooperative_request();
        registry.begin_transfer(request.clone()).unwrap();
        registry
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();

        let rollback = registry
            .reverse_transfer(ReverseOwnerTransferRequest {
                profile_identity_digest: digest("profile"),
                expected_candidate_owner_id: "owner-new".to_string(),
                expected_candidate_owner_generation: 8,
                transfer_nonce_digest: digest("transfer"),
                reverse_nonce_digest: digest("reverse"),
            })
            .unwrap();

        assert_eq!(rollback.previous_owner_generation, 8);
        assert_eq!(rollback.candidate_owner_generation, 9);
        let restored = registry.owner(&digest("profile")).unwrap();
        assert_eq!(restored.owner_id, "owner-old");
        assert_eq!(restored.owner_generation, 9);
        assert_eq!(
            registry.principal_bindings()[&profile_digest].owner_generation,
            9
        );
        assert!(registry
            .principal_binding_is_current(registry.principal_bindings().get(&profile_digest)));
    }

    #[test]
    fn mismatched_candidate_never_revokes_the_old_owner() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let request = cooperative_request();
        registry.begin_transfer(request.clone()).unwrap();
        let mut attachment = CandidateOwnerAttachment::from_request(&request, 8);
        attachment.target_set_digest = digest("different-targets");

        assert_eq!(
            registry.commit_candidate(attachment).unwrap_err().code,
            OwnerTransferFailureCode::CandidateEvidenceMismatch
        );
        assert_eq!(
            registry.owner(&digest("profile")).unwrap().owner_generation,
            7
        );
        assert!(registry.authorizes(&OwnerAuthorityClaim::from_owner(
            registry.owner(&digest("profile")).unwrap()
        )));
    }

    #[test]
    fn manual_preservation_never_creates_effect_authority() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let mut request = cooperative_request();
        request.mode = BrowserAdoptionMode::ManualPreservation;

        assert_eq!(
            registry.begin_transfer(request).unwrap_err().code,
            OwnerTransferFailureCode::UnsupportedOwnerState
        );
        let preserved = registry.owner(&digest("profile")).unwrap();
        assert_eq!(preserved.owner_id, "owner-old");
        assert_eq!(preserved.owner_generation, 7);
        assert!(preserved.pending_transfer.is_none());
    }

    #[test]
    fn daemon_effect_gate_fences_the_old_generation_after_commit() {
        let repository = MemoryRepository::default();
        repository
            .mutate(|state| {
                let current = owner();
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(current.clone());
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .insert(
                    current.browser_id.clone(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: current.browser_id,
                        profile_identity_digest: current.profile_identity_digest,
                        owner_generation: current.owner_generation,
                        lifecycle_state: RuntimeLaneLifecycleState::Ready,
                        cleanup_obligation_state: CleanupObligationState::Owned,
                        ..RuntimeLifecycleRecord::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        let request = cooperative_request();
        let mut old_binding = RuntimeOwnerBinding::effect_capable(OwnerAuthorityClaim::from_owner(
            repository
                .load_snapshot()
                .unwrap()
                .runtime_owner_registry
                .owner(&digest("profile"))
                .unwrap(),
        ));
        let mut candidate_binding = RuntimeOwnerBinding::observation_only(OwnerAuthorityClaim {
            owner_id: "owner-new".to_string(),
            profile_identity_digest: digest("profile"),
            owner_generation: 8,
            logical_browser_id: "browser-a".to_string(),
            daemon_session_route: "session-new".to_string(),
            process_instance_digest: digest("process"),
        });
        let authority = RuntimeLifecycleAuthority::new(&repository);

        assert!(authorize_action(&authority, &mut old_binding, "navigate").is_ok());
        assert!(authorize_action(&authority, &mut candidate_binding, "navigate").is_err());
        authority.begin_transfer(request.clone()).unwrap();
        authority
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();

        assert!(authorize_action(&authority, &mut old_binding, "navigate").is_err());
        assert!(authorize_action(&authority, &mut old_binding, "tab_list").is_ok());
        assert!(!action_requires_owner_effect_authority("dependent_batch"));
        assert!(!action_requires_owner_effect_authority("service_incidents"));
        assert!(action_requires_runtime_admission(
            "service_profile_recovery_apply"
        ));
        assert!(!action_requires_owner_effect_authority(
            "service_profile_recovery_apply"
        ));
        assert!(action_requires_owner_effect_authority(
            "service_incident_resolve"
        ));
        let mut committed_candidate = RuntimeOwnerBinding::effect_capable(candidate_binding.claim);
        assert!(authorize_action(&authority, &mut committed_candidate, "navigate").is_ok());
    }

    #[test]
    fn session_binding_fences_restarted_supervisor_and_refreshes_reversed_owner() {
        let repository = MemoryRepository {
            state: Mutex::new(ServiceState {
                runtime_owner_registry: RuntimeOwnerRegistry::from_owner(owner()),
                ..ServiceState::default()
            }),
        };
        let original = owner_binding_for_session(&repository, "session-old")
            .unwrap()
            .unwrap();
        assert!(original.effect_capable);

        let authority = RuntimeLifecycleAuthority::new(&repository);
        let proposal = authority.begin_transfer(cooperative_request()).unwrap();
        let attachment = CandidateOwnerAttachment::from_request(
            &proposal.request,
            proposal.candidate_owner_generation,
        );
        authority.commit_candidate(attachment).unwrap();

        let restarted_old = owner_binding_for_session(&repository, "session-old")
            .unwrap()
            .unwrap();
        assert!(!restarted_old.effect_capable);
        let candidate = owner_binding_for_session(&repository, "session-new")
            .unwrap()
            .unwrap();
        assert!(candidate.effect_capable);

        authority
            .reverse_transfer(ReverseOwnerTransferRequest {
                profile_identity_digest: digest("profile"),
                expected_candidate_owner_id: "owner-new".to_string(),
                expected_candidate_owner_generation: proposal.candidate_owner_generation,
                transfer_nonce_digest: digest("transfer"),
                reverse_nonce_digest: digest("reverse"),
            })
            .unwrap();

        let mut stale_old = original;
        authorize_action(&authority, &mut stale_old, "navigate").unwrap();
        assert!(stale_old.effect_capable);
        assert_eq!(stale_old.claim.owner_generation, 9);
    }

    #[test]
    fn terminal_cleanup_satisfied_owner_is_history_not_a_rehydrated_effect_binding() {
        let current = owner();
        let mut registry = RuntimeOwnerRegistry::from_owner(current.clone());
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .lifecycle_rows
            .insert(
                current.browser_id.clone(),
                RuntimeLifecycleRecord {
                    logical_browser_id: current.browser_id.clone(),
                    profile_identity_digest: current.profile_identity_digest.clone(),
                    owner_generation: current.owner_generation,
                    lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                    cleanup_obligation_state: CleanupObligationState::Satisfied,
                    terminal_evidence: vec![
                        "exact_process_exited".to_string(),
                        "profile_lock_released".to_string(),
                    ],
                    ..RuntimeLifecycleRecord::default()
                },
            );
        assert!(registry
            .binding_for_session("session-old")
            .unwrap()
            .is_some());
        let repository = MemoryRepository {
            state: Mutex::new(ServiceState {
                runtime_owner_registry: registry,
                ..ServiceState::default()
            }),
        };

        assert!(owner_binding_for_session(&repository, "session-old")
            .unwrap()
            .is_none());
    }

    #[test]
    fn terminal_history_does_not_make_a_replacement_session_binding_ambiguous() {
        let historical = owner();
        let mut replacement = historical.clone();
        replacement.owner_id = "owner-current".to_string();
        replacement.profile_identity_digest = digest("profile-current");
        replacement.browser_id = "browser-current".to_string();
        replacement.process_instance_digest = digest("process-current");
        replacement.cdp_endpoint_identity_digest = digest("cdp-current");
        replacement.target_set_digest = digest("targets-current");

        let mut registry = RuntimeOwnerRegistry::from_owner(historical.clone());
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .owner_records
            .insert(
                replacement.profile_identity_digest.clone(),
                replacement.clone(),
            );
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .lifecycle_rows
            .insert(
                historical.browser_id.clone(),
                RuntimeLifecycleRecord {
                    logical_browser_id: historical.browser_id,
                    profile_identity_digest: historical.profile_identity_digest,
                    owner_generation: historical.owner_generation,
                    lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                    cleanup_obligation_state: CleanupObligationState::Satisfied,
                    terminal_evidence: vec![
                        "exact_process_exited".to_string(),
                        "profile_lock_released".to_string(),
                    ],
                    ..RuntimeLifecycleRecord::default()
                },
            );

        let binding = registry
            .binding_for_session("session-old")
            .unwrap()
            .unwrap();
        assert_eq!(
            binding.claim.profile_identity_digest,
            replacement.profile_identity_digest
        );
        assert!(binding.effect_capable);

        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .lifecycle_rows
            .get_mut("browser-a")
            .unwrap()
            .cleanup_obligation_state = CleanupObligationState::Owned;
        assert!(registry
            .binding_for_session("session-old")
            .unwrap_err()
            .contains("runtime_owner_session_ambiguous"));
    }

    #[test]
    fn service_state_repository_persists_the_single_owner_registry() {
        let repository = MemoryRepository::default();
        repository
            .mutate(|state| {
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(owner());
                Ok(())
            })
            .unwrap();
        let request = cooperative_request();
        let authority = RuntimeLifecycleAuthority::new(&repository);

        authority.begin_transfer(request.clone()).unwrap();
        let receipt = authority
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();
        let persisted = repository.load_snapshot().unwrap();

        assert_eq!(receipt.candidate_owner_generation, 8);
        assert_eq!(persisted.runtime_owner_registry.revision(), 5);
        assert_eq!(
            persisted.runtime_owner_registry.lifecycle_records()["browser-a"]
                .cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        assert_eq!(
            persisted
                .runtime_owner_registry
                .owner(&digest("profile"))
                .unwrap()
                .owner_id,
            "owner-new"
        );
    }

    #[test]
    fn current_owner_registration_is_idempotent_and_rejects_conflicts() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let registered = authority.register_current_owner(owner()).unwrap();
        let replay = authority.register_current_owner(owner()).unwrap();
        assert_eq!(registered, replay);

        let mut conflict = owner();
        conflict.process_instance_digest = digest("different-process");
        assert!(authority.register_current_owner(conflict).is_err());

        let persisted = repository.load_snapshot().unwrap();
        assert_eq!(persisted.runtime_owner_registry.revision(), 2);
        assert_eq!(
            persisted
                .runtime_owner_registry
                .owner(&digest("profile"))
                .unwrap()
                .process_instance_digest,
            digest("process")
        );
    }

    #[test]
    fn owner_transfer_fixture_corpus_freezes_authority_at_every_boundary() {
        let corpus: OwnerTransferFixtureCorpus =
            serde_json::from_str(OWNER_TRANSFER_FIXTURES).unwrap();

        assert_eq!(
            corpus.schema_version,
            crate::runtime_adoption::RUNTIME_ADOPTION_SCHEMA_VERSION
        );
        assert_eq!(corpus.fixtures.len(), 7);
        assert!(corpus.fixtures.iter().all(|fixture| {
            !fixture.fixture_id.is_empty()
                && !fixture.initial_owner_state.is_empty()
                && !fixture.candidate_observation_effect_capable
                && fixture.expected_authority_timeline.len() == 3
        }));
        assert!(corpus.fixtures.iter().any(|fixture| {
            fixture.mode == BrowserAdoptionMode::OrphanAdoption
                && fixture.previous_owner_generation == 0
                && fixture.candidate_owner_generation == 1
                && fixture.expected_authority_timeline == ["none", "none", "candidate"]
        }));
        assert!(corpus.fixtures.iter().any(|fixture| {
            fixture.fixture_id == "cooperative-transfer-reverse"
                && fixture
                    .expected_authority_timeline
                    .last()
                    .map(String::as_str)
                    == Some("old_at_generation_19")
        }));
    }

    #[test]
    fn daemon_owner_gate_precedes_stream_broadcast_and_browser_recovery() {
        let source = include_str!("native/actions.rs");
        let gate = source
            .find("crate::native::runtime_lifecycle::admit_default_action_effect(")
            .expect("runtime owner gate must be installed");
        let desktop_effect = source
            .find("if action == \"desktop_interact\"")
            .expect("desktop effect anchor must remain present");
        let broadcast = source
            .find("server.broadcast_command(action, &id, cmd)")
            .expect("stream broadcast anchor must remain present");
        let recovery = source
            .find("detect_browser_stale_state(state).await")
            .expect("browser recovery anchor must remain present");

        assert!(gate < desktop_effect);
        assert!(gate < broadcast);
        assert!(gate < recovery);
    }
}
