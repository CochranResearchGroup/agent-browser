use crate::native::service_store::ServiceStateRepository;
use crate::runtime_owner_transfer::{
    CandidateOwnerAttachment, CleanupObligationState, OwnerAuthorityClaim, OwnerTransferError,
    OwnerTransferProposal, OwnerTransferReceipt, OwnerTransferRequest, ProfileOwner,
    ReverseOwnerTransferRequest, RuntimeLaneLifecycleState, RuntimeLifecycleRecord,
    RuntimeOwnerBinding, RuntimeOwnerRegistry,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct ManagedLaneRegistration {
    pub(crate) logical_browser_id: String,
    pub(crate) profile_root: PathBuf,
    pub(crate) daemon_session_route: String,
    pub(crate) process_group_id: Option<u32>,
    pub(crate) process_identity: crate::process_identity::RecordedProcessIdentity,
    pub(crate) browser_family: String,
    pub(crate) cdp_endpoint: String,
    pub(crate) target_ids: Vec<String>,
}

/// The complete set of lifecycle intentions accepted by the concrete runtime
/// lifecycle owner. Callers describe intent and never mutate owner or cleanup
/// state directly.
#[derive(Debug, Clone)]
pub(crate) enum RuntimeLifecycleIntent {
    RegisterCurrentOwner(ProfileOwner),
    RegisterManagedLane {
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
    },
    ActivateTerminalReplacement {
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
    },
    RefreshCurrentOwnerEvidence {
        claim: OwnerAuthorityClaim,
        cdp_endpoint_identity_digest: String,
        target_set_digest: String,
    },
    BeginTransfer(OwnerTransferRequest),
    CommitCandidate(CandidateOwnerAttachment),
    AbortTransfer {
        profile_identity_digest: String,
        expected_owner_id: String,
        expected_owner_generation: u64,
        transfer_nonce_digest: String,
    },
    ReverseTransfer(ReverseOwnerTransferRequest),
    RevokeLegacyOwner {
        profile_identity_digest: String,
        logical_browser_id: String,
        expected_daemon_session_route: String,
        expected_owner_id: String,
        expected_owner_generation: u64,
    },
    PreserveRetained {
        claim: OwnerAuthorityClaim,
    },
    BeginClose {
        claim: OwnerAuthorityClaim,
    },
    CompleteClose {
        logical_browser_id: String,
        profile_identity_digest: String,
        expected_owner_generation: u64,
        terminal_evidence: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeLifecycleTransition {
    OwnerRegistered(ProfileOwner),
    TerminalReplacementActivated(ProfileOwner),
    OwnerEvidenceRefreshed(ProfileOwner),
    TransferPrepared(OwnerTransferProposal),
    CandidateCommitted(OwnerTransferReceipt),
    TransferAborted(bool),
    TransferReversed(OwnerTransferReceipt),
    LegacyOwnerRevoked(ProfileOwner),
    LaneUpdated(RuntimeLifecycleRecord),
}

/// Concrete lifecycle owner backed by the existing locked Service State
/// repository and runtime-owner registry.
pub(crate) struct RuntimeLifecycleAuthority<'a, R: ServiceStateRepository> {
    repository: &'a R,
}

impl<'a, R: ServiceStateRepository> RuntimeLifecycleAuthority<'a, R> {
    pub(crate) fn new(repository: &'a R) -> Self {
        Self { repository }
    }

    pub(crate) fn transition(
        &self,
        intent: RuntimeLifecycleIntent,
    ) -> Result<RuntimeLifecycleTransition, String> {
        self.repository.mutate(|state| {
            let mut registry = state.runtime_owner_registry.clone();
            let transition = apply_transition(&mut registry, intent)?;
            state.runtime_owner_registry = registry;
            Ok(transition)
        })
    }

    pub(crate) fn register_current_owner(
        &self,
        owner: ProfileOwner,
    ) -> Result<ProfileOwner, String> {
        match self.transition(RuntimeLifecycleIntent::RegisterCurrentOwner(owner))? {
            RuntimeLifecycleTransition::OwnerRegistered(owner) => Ok(owner),
            _ => Err("runtime_lifecycle_registration_outcome_mismatch".to_string()),
        }
    }

    pub(crate) fn begin_transfer(
        &self,
        request: OwnerTransferRequest,
    ) -> Result<OwnerTransferProposal, String> {
        match self.transition(RuntimeLifecycleIntent::BeginTransfer(request))? {
            RuntimeLifecycleTransition::TransferPrepared(proposal) => Ok(proposal),
            _ => Err("runtime_lifecycle_transfer_prepare_outcome_mismatch".to_string()),
        }
    }

    pub(crate) fn commit_candidate(
        &self,
        attachment: CandidateOwnerAttachment,
    ) -> Result<OwnerTransferReceipt, String> {
        match self.transition(RuntimeLifecycleIntent::CommitCandidate(attachment))? {
            RuntimeLifecycleTransition::CandidateCommitted(receipt) => Ok(receipt),
            _ => Err("runtime_lifecycle_candidate_commit_outcome_mismatch".to_string()),
        }
    }

    pub(crate) fn abort_transfer(
        &self,
        profile_identity_digest: &str,
        expected_owner_id: &str,
        expected_owner_generation: u64,
        transfer_nonce_digest: &str,
    ) -> Result<bool, String> {
        match self.transition(RuntimeLifecycleIntent::AbortTransfer {
            profile_identity_digest: profile_identity_digest.to_string(),
            expected_owner_id: expected_owner_id.to_string(),
            expected_owner_generation,
            transfer_nonce_digest: transfer_nonce_digest.to_string(),
        })? {
            RuntimeLifecycleTransition::TransferAborted(aborted) => Ok(aborted),
            _ => Err("runtime_lifecycle_transfer_abort_outcome_mismatch".to_string()),
        }
    }

    pub(crate) fn reverse_transfer(
        &self,
        request: ReverseOwnerTransferRequest,
    ) -> Result<OwnerTransferReceipt, String> {
        match self.transition(RuntimeLifecycleIntent::ReverseTransfer(request))? {
            RuntimeLifecycleTransition::TransferReversed(receipt) => Ok(receipt),
            _ => Err("runtime_lifecycle_reverse_outcome_mismatch".to_string()),
        }
    }

    pub(crate) fn revoke_legacy_owner(
        &self,
        profile_identity_digest: &str,
        logical_browser_id: &str,
        expected_daemon_session_route: &str,
        expected_owner_id: &str,
        expected_owner_generation: u64,
    ) -> Result<ProfileOwner, String> {
        match self.transition(RuntimeLifecycleIntent::RevokeLegacyOwner {
            profile_identity_digest: profile_identity_digest.to_string(),
            logical_browser_id: logical_browser_id.to_string(),
            expected_daemon_session_route: expected_daemon_session_route.to_string(),
            expected_owner_id: expected_owner_id.to_string(),
            expected_owner_generation,
        })? {
            RuntimeLifecycleTransition::LegacyOwnerRevoked(owner) => Ok(owner),
            _ => Err("runtime_lifecycle_legacy_revoke_outcome_mismatch".to_string()),
        }
    }

    /// Fence one browser side effect against the current lifecycle owner.
    /// A reversed transfer may refresh the original daemon's generation, but
    /// no observation-only candidate can acquire effect authority here.
    pub(crate) fn authorize_effect(&self, binding: &mut RuntimeOwnerBinding) -> Result<(), String> {
        if !binding.effect_capable {
            return Err("runtime_owner_observation_only: candidate cannot issue browser effects before owner compare-and-swap".to_string());
        }
        let registry = self.repository.load_snapshot()?.runtime_owner_registry;
        if registry.authorizes(&binding.claim) {
            return Ok(());
        }
        if let Some(claim) = registry.refreshed_claim_after_reverse(&binding.claim) {
            binding.claim = claim;
            return Ok(());
        }
        Err(
            "runtime_owner_generation_stale: daemon is no longer the effect-capable browser owner"
                .to_string(),
        )
    }

    pub(crate) fn refresh_managed_lane(
        &self,
        binding: &mut RuntimeOwnerBinding,
        cdp_endpoint: &str,
        mut target_ids: Vec<String>,
    ) -> Result<(), String> {
        self.authorize_effect(binding)?;
        target_ids.sort();
        target_ids.dedup();
        if target_ids.is_empty() {
            return Err("runtime_lifecycle_target_set_empty".to_string());
        }
        let transition = self.transition(RuntimeLifecycleIntent::RefreshCurrentOwnerEvidence {
            claim: binding.claim.clone(),
            cdp_endpoint_identity_digest: digest_text(cdp_endpoint),
            target_set_digest: digest_json(&target_ids)?,
        })?;
        let RuntimeLifecycleTransition::OwnerEvidenceRefreshed(owner) = transition else {
            return Err("runtime_lifecycle_refresh_outcome_mismatch".to_string());
        };
        binding.claim = OwnerAuthorityClaim::from_owner(&owner);
        Ok(())
    }

    /// Permit a stale daemon to relinquish its local process handle only after
    /// another generation owns both effects and the cleanup obligation.
    pub(crate) fn authorize_relinquish_after_transfer(
        &self,
        stale_claim: &OwnerAuthorityClaim,
    ) -> Result<(), String> {
        let registry = self.repository.load_snapshot()?.runtime_owner_registry;
        if registry.authorizes(stale_claim) {
            return Err("runtime_lifecycle_relinquish_before_owner_commit".to_string());
        }
        let current = registry
            .owner(&stale_claim.profile_identity_digest)
            .ok_or_else(|| "runtime_lifecycle_relinquish_owner_missing".to_string())?;
        let lifecycle = registry
            .lifecycle_records
            .get(&stale_claim.logical_browser_id)
            .ok_or_else(|| "runtime_lifecycle_relinquish_record_missing".to_string())?;
        let restored_lifecycle_matches = matches!(
            (current.state, lifecycle.lifecycle_state),
            (
                crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                RuntimeLaneLifecycleState::Ready
            ) | (
                crate::runtime_owner_transfer::ProfileOwnerState::Orphaned,
                RuntimeLaneLifecycleState::Retained
            )
        );
        if !restored_lifecycle_matches
            || current.browser_id != stale_claim.logical_browser_id
            || current.process_instance_digest != stale_claim.process_instance_digest
            || current.owner_generation <= stale_claim.owner_generation
            || lifecycle.owner_generation != current.owner_generation
            || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
        {
            return Err("runtime_lifecycle_relinquish_authority_unproven".to_string());
        }
        Ok(())
    }

    pub(crate) fn reviewed_process_tree(
        &self,
        binding: &RuntimeOwnerBinding,
        root_process: &crate::process_identity::RecordedProcessIdentity,
    ) -> Result<Option<crate::native::runtime_reconciliation::ReviewedProcessTree>, String> {
        let registry = self.repository.load_snapshot()?.runtime_owner_registry;
        if !binding.effect_capable || !registry.authorizes(&binding.claim) {
            return Err("runtime_lifecycle_process_tree_owner_stale".to_string());
        }
        if digest_json(root_process)? != binding.claim.process_instance_digest {
            return Err("runtime_lifecycle_process_tree_identity_mismatch".to_string());
        }
        let lifecycle = registry
            .lifecycle_records
            .get(&binding.claim.logical_browser_id)
            .ok_or_else(|| "runtime_lifecycle_process_tree_record_missing".to_string())?;
        if lifecycle.profile_identity_digest != binding.claim.profile_identity_digest
            || lifecycle.owner_generation != binding.claim.owner_generation
        {
            return Err("runtime_lifecycle_process_tree_generation_mismatch".to_string());
        }
        let (Some(process_group_id), Some(package_launch_identity_digest)) = (
            lifecycle.process_group_id,
            lifecycle.package_launch_identity_digest.clone(),
        ) else {
            return Ok(None);
        };
        Ok(Some(
            crate::native::runtime_reconciliation::ReviewedProcessTree {
                root_process: root_process.clone(),
                process_group_id,
                logical_browser_id: binding.claim.logical_browser_id.clone(),
                profile_identity_digest: binding.claim.profile_identity_digest.clone(),
                owner_generation: binding.claim.owner_generation,
                package_launch_identity_digest,
            },
        ))
    }

    /// Register one newly observed managed browser lane and return the binding
    /// that every subsequent effect must present. A conflicting durable owner
    /// requires an explicit transfer, close, or recovery transition.
    pub(crate) fn register_managed_lane(
        &self,
        mut registration: ManagedLaneRegistration,
    ) -> Result<RuntimeOwnerBinding, String> {
        if registration.target_ids.is_empty() {
            return Err("runtime_lifecycle_target_set_empty".to_string());
        }
        registration.target_ids.sort();
        registration.target_ids.dedup();
        let profile_identity_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&registration.profile_root)?;
        let process_instance_digest = digest_json(&registration.process_identity)?;
        let cdp_endpoint_identity_digest = digest_text(&registration.cdp_endpoint);
        let target_set_digest = digest_json(&registration.target_ids)?;
        let existing = self
            .repository
            .load_snapshot()?
            .runtime_owner_registry
            .owner(&profile_identity_digest)
            .cloned();
        let owner_generation = existing.as_ref().map_or(1, |owner| owner.owner_generation);
        let owner = ProfileOwner {
            owner_id: format!(
                "owner-{}",
                &digest_text(&format!(
                    "{}:{profile_identity_digest}:{process_instance_digest}",
                    registration.daemon_session_route
                ))[..20]
            ),
            profile_identity_digest,
            state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
            owner_generation,
            browser_id: registration.logical_browser_id,
            daemon_session_route: registration.daemon_session_route,
            process_instance_digest,
            browser_family: registration.browser_family,
            cdp_endpoint_identity_digest,
            target_set_digest,
            pending_transfer: None,
            last_transition: None,
        };
        let package_launch_identity_digest =
            package_launch_identity_digest(&owner, registration.process_group_id)?;
        if let Some(current) = existing {
            let lifecycle = self
                .repository
                .load_snapshot()?
                .runtime_owner_registry
                .lifecycle_records
                .get(&current.browser_id)
                .cloned();
            if lifecycle.as_ref().is_some_and(|lifecycle| {
                lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
                    && lifecycle.cleanup_obligation_state == CleanupObligationState::Satisfied
                    && lifecycle.owner_generation == current.owner_generation
            }) {
                let mut replacement = owner;
                replacement.owner_generation = current
                    .owner_generation
                    .checked_add(1)
                    .ok_or_else(|| "runtime_lifecycle_generation_exhausted".to_string())?;
                let activated =
                    match self.transition(RuntimeLifecycleIntent::ActivateTerminalReplacement {
                        owner: replacement,
                        process_group_id: registration.process_group_id,
                        package_launch_identity_digest,
                    })? {
                        RuntimeLifecycleTransition::TerminalReplacementActivated(owner) => owner,
                        _ => {
                            return Err("runtime_lifecycle_replacement_outcome_mismatch".to_string())
                        }
                    };
                return Ok(RuntimeOwnerBinding::effect_capable(
                    OwnerAuthorityClaim::from_owner(&activated),
                ));
            }
            let stable_identity_matches = current.owner_id == owner.owner_id
                && current.profile_identity_digest == owner.profile_identity_digest
                && current.state == owner.state
                && current.owner_generation == owner.owner_generation
                && current.browser_id == owner.browser_id
                && current.daemon_session_route == owner.daemon_session_route
                && current.process_instance_digest == owner.process_instance_digest
                && current.browser_family == owner.browser_family
                && current.pending_transfer.is_none();
            if !stable_identity_matches {
                return Err(
                    "runtime_lifecycle_existing_owner_requires_explicit_transition".to_string(),
                );
            }
            let refreshed =
                match self.transition(RuntimeLifecycleIntent::RefreshCurrentOwnerEvidence {
                    claim: OwnerAuthorityClaim::from_owner(&current),
                    cdp_endpoint_identity_digest: owner.cdp_endpoint_identity_digest,
                    target_set_digest: owner.target_set_digest,
                })? {
                    RuntimeLifecycleTransition::OwnerEvidenceRefreshed(owner) => owner,
                    _ => return Err("runtime_lifecycle_refresh_outcome_mismatch".to_string()),
                };
            return Ok(RuntimeOwnerBinding::effect_capable(
                OwnerAuthorityClaim::from_owner(&refreshed),
            ));
        }
        let registered = match self.transition(RuntimeLifecycleIntent::RegisterManagedLane {
            owner,
            process_group_id: registration.process_group_id,
            package_launch_identity_digest,
        })? {
            RuntimeLifecycleTransition::OwnerRegistered(owner) => owner,
            _ => return Err("runtime_lifecycle_registration_outcome_mismatch".to_string()),
        };
        Ok(RuntimeOwnerBinding::effect_capable(
            OwnerAuthorityClaim::from_owner(&registered),
        ))
    }
}

fn digest_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub(crate) fn digest_json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| format!("runtime_lifecycle_identity_encode_failed: {error}"))
}

pub(crate) fn package_launch_identity_digest(
    owner: &ProfileOwner,
    process_group_id: Option<u32>,
) -> Result<String, String> {
    digest_json(&(
        owner.browser_id.as_str(),
        owner.profile_identity_digest.as_str(),
        owner.owner_generation,
        owner.process_instance_digest.as_str(),
        process_group_id,
    ))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn apply_transition(
    registry: &mut RuntimeOwnerRegistry,
    intent: RuntimeLifecycleIntent,
) -> Result<RuntimeLifecycleTransition, String> {
    match intent {
        RuntimeLifecycleIntent::RegisterCurrentOwner(owner) => {
            let owner = registry
                .register_current_owner(owner)
                .map_err(owner_error)?;
            let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
            lifecycle.owner_generation = owner.owner_generation;
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::OwnerRegistered(owner))
        }
        RuntimeLifecycleIntent::RegisterManagedLane {
            owner,
            process_group_id,
            package_launch_identity_digest,
        } => {
            if !is_digest(&package_launch_identity_digest) {
                return Err("runtime_lifecycle_package_launch_identity_invalid".to_string());
            }
            let owner = registry
                .register_current_owner(owner)
                .map_err(owner_error)?;
            let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
            lifecycle.owner_generation = owner.owner_generation;
            lifecycle.process_group_id = process_group_id;
            lifecycle.package_launch_identity_digest = Some(package_launch_identity_digest);
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::OwnerRegistered(owner))
        }
        RuntimeLifecycleIntent::ActivateTerminalReplacement {
            owner,
            process_group_id,
            package_launch_identity_digest,
        } => {
            crate::runtime_owner_transfer::validate_profile_owner(&owner).map_err(owner_error)?;
            if !is_digest(&package_launch_identity_digest) {
                return Err("runtime_lifecycle_package_launch_identity_invalid".to_string());
            }
            let current = registry
                .owner(&owner.profile_identity_digest)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_replacement_owner_missing".to_string())?;
            let lifecycle = registry
                .lifecycle_records
                .get(&current.browser_id)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_replacement_record_missing".to_string())?;
            if owner.owner_generation != current.owner_generation.saturating_add(1)
                || owner.browser_id != current.browser_id
                || lifecycle.profile_identity_digest != owner.profile_identity_digest
                || lifecycle.owner_generation != current.owner_generation
                || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Terminal
                || lifecycle.cleanup_obligation_state != CleanupObligationState::Satisfied
            {
                return Err("runtime_lifecycle_terminal_replacement_rejected".to_string());
            }
            registry
                .owners
                .insert(owner.profile_identity_digest.clone(), owner.clone());
            registry.revision = registry.revision.saturating_add(1);
            let mut lifecycle = lifecycle;
            lifecycle.owner_generation = owner.owner_generation;
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
            lifecycle.process_group_id = process_group_id;
            lifecycle.package_launch_identity_digest = Some(package_launch_identity_digest);
            lifecycle.terminal_evidence.clear();
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::TerminalReplacementActivated(
                owner,
            ))
        }
        RuntimeLifecycleIntent::RefreshCurrentOwnerEvidence {
            claim,
            cdp_endpoint_identity_digest,
            target_set_digest,
        } => {
            if !registry.authorizes(&claim)
                || !is_digest(&cdp_endpoint_identity_digest)
                || !is_digest(&target_set_digest)
            {
                return Err("runtime_lifecycle_owner_evidence_refresh_rejected".to_string());
            }
            let owner = registry
                .owners
                .get_mut(&claim.profile_identity_digest)
                .ok_or_else(|| "runtime_lifecycle_owner_missing".to_string())?;
            if owner.cdp_endpoint_identity_digest != cdp_endpoint_identity_digest
                || owner.target_set_digest != target_set_digest
            {
                owner.cdp_endpoint_identity_digest = cdp_endpoint_identity_digest;
                owner.target_set_digest = target_set_digest;
                registry.revision = registry.revision.saturating_add(1);
            }
            Ok(RuntimeLifecycleTransition::OwnerEvidenceRefreshed(
                owner.clone(),
            ))
        }
        RuntimeLifecycleIntent::BeginTransfer(request) => {
            let current_owner = registry
                .owner(&request.profile_identity_digest)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_owner_missing".to_string())?;
            let proposal = registry.begin_transfer(request).map_err(owner_error)?;
            let mut lifecycle = take_or_bootstrap_lifecycle(registry, &current_owner)?;
            if lifecycle.owner_generation != proposal.previous_owner_generation {
                return Err("runtime_lifecycle_generation_mismatch".to_string());
            }
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Transferring;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Transferring;
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::TransferPrepared(proposal))
        }
        RuntimeLifecycleIntent::CommitCandidate(attachment) => {
            let profile_identity_digest = attachment.profile_identity_digest.clone();
            let receipt = registry.commit_candidate(attachment).map_err(owner_error)?;
            let owner = registry
                .owner(&profile_identity_digest)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_owner_missing_after_commit".to_string())?;
            let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
            lifecycle.owner_generation = owner.owner_generation;
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::CandidateCommitted(receipt))
        }
        RuntimeLifecycleIntent::AbortTransfer {
            profile_identity_digest,
            expected_owner_id,
            expected_owner_generation,
            transfer_nonce_digest,
        } => {
            let owner = registry
                .owner(&profile_identity_digest)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_owner_missing".to_string())?;
            let aborted = registry
                .abort_pending_transfer(
                    &profile_identity_digest,
                    &expected_owner_id,
                    expected_owner_generation,
                    &transfer_nonce_digest,
                )
                .map_err(owner_error)?;
            if aborted {
                let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
                lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
                lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
                lifecycle.owner_generation = owner.owner_generation;
                store_lifecycle(registry, lifecycle)?;
            }
            Ok(RuntimeLifecycleTransition::TransferAborted(aborted))
        }
        RuntimeLifecycleIntent::ReverseTransfer(request) => {
            let profile_identity_digest = request.profile_identity_digest.clone();
            let receipt = registry.reverse_transfer(request).map_err(owner_error)?;
            let owner = registry
                .owner(&profile_identity_digest)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_owner_missing_after_reverse".to_string())?;
            let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
            lifecycle.owner_generation = owner.owner_generation;
            lifecycle.lifecycle_state = match owner.state {
                crate::runtime_owner_transfer::ProfileOwnerState::Ready => {
                    RuntimeLaneLifecycleState::Ready
                }
                crate::runtime_owner_transfer::ProfileOwnerState::Orphaned => {
                    RuntimeLaneLifecycleState::Retained
                }
                _ => return Err("runtime_lifecycle_reverse_owner_state_invalid".to_string()),
            };
            lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::TransferReversed(receipt))
        }
        RuntimeLifecycleIntent::RevokeLegacyOwner {
            profile_identity_digest,
            logical_browser_id,
            expected_daemon_session_route,
            expected_owner_id,
            expected_owner_generation,
        } => {
            let owner = registry
                .revoke_legacy_daemon_owner(
                    &profile_identity_digest,
                    &logical_browser_id,
                    &expected_daemon_session_route,
                    &expected_owner_id,
                    expected_owner_generation,
                )
                .map_err(owner_error)?;
            let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
            lifecycle.owner_generation = owner.owner_generation;
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Retained;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
            store_lifecycle(registry, lifecycle)?;
            Ok(RuntimeLifecycleTransition::LegacyOwnerRevoked(owner))
        }
        RuntimeLifecycleIntent::PreserveRetained { claim } => update_effect_owned_lane(
            registry,
            &claim,
            RuntimeLaneLifecycleState::Retained,
            CleanupObligationState::Owned,
        ),
        RuntimeLifecycleIntent::BeginClose { claim } => update_effect_owned_lane(
            registry,
            &claim,
            RuntimeLaneLifecycleState::Closing,
            CleanupObligationState::Owned,
        ),
        RuntimeLifecycleIntent::CompleteClose {
            logical_browser_id,
            profile_identity_digest,
            expected_owner_generation,
            terminal_evidence,
        } => {
            if terminal_evidence.is_empty()
                || terminal_evidence
                    .iter()
                    .any(|evidence| evidence.trim().is_empty())
            {
                return Err("runtime_lifecycle_terminal_evidence_missing".to_string());
            }
            let lifecycle = registry
                .lifecycle_records
                .get_mut(&logical_browser_id)
                .ok_or_else(|| "runtime_lifecycle_record_missing".to_string())?;
            if lifecycle.profile_identity_digest != profile_identity_digest
                || lifecycle.owner_generation != expected_owner_generation
                || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Closing
                || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
            {
                return Err("runtime_lifecycle_close_compare_and_swap_mismatch".to_string());
            }
            lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Terminal;
            lifecycle.cleanup_obligation_state = CleanupObligationState::Satisfied;
            lifecycle.terminal_evidence = terminal_evidence;
            let lifecycle = lifecycle.clone();
            registry.revision = registry.revision.saturating_add(1);
            Ok(RuntimeLifecycleTransition::LaneUpdated(lifecycle))
        }
    }
}

fn update_effect_owned_lane(
    registry: &mut RuntimeOwnerRegistry,
    claim: &OwnerAuthorityClaim,
    lifecycle_state: RuntimeLaneLifecycleState,
    cleanup_obligation_state: CleanupObligationState,
) -> Result<RuntimeLifecycleTransition, String> {
    if !registry.authorizes(claim) {
        return Err("runtime_lifecycle_owner_generation_stale".to_string());
    }
    let owner = registry
        .owner(&claim.profile_identity_digest)
        .cloned()
        .ok_or_else(|| "runtime_lifecycle_owner_missing".to_string())?;
    let mut lifecycle = take_or_bootstrap_lifecycle(registry, &owner)?;
    lifecycle.owner_generation = owner.owner_generation;
    lifecycle.lifecycle_state = lifecycle_state;
    lifecycle.cleanup_obligation_state = cleanup_obligation_state;
    store_lifecycle(registry, lifecycle.clone())?;
    Ok(RuntimeLifecycleTransition::LaneUpdated(lifecycle))
}

fn take_or_bootstrap_lifecycle(
    registry: &mut RuntimeOwnerRegistry,
    owner: &ProfileOwner,
) -> Result<RuntimeLifecycleRecord, String> {
    let matching_keys = registry
        .lifecycle_records
        .iter()
        .filter(|(_, lifecycle)| {
            lifecycle.logical_browser_id == owner.browser_id
                || lifecycle.profile_identity_digest == owner.profile_identity_digest
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    if matching_keys.len() > 1 {
        return Err("runtime_lifecycle_record_ambiguous".to_string());
    }
    if let Some(key) = matching_keys.first() {
        let lifecycle = if key == &owner.browser_id {
            registry
                .lifecycle_records
                .get(key)
                .cloned()
                .ok_or_else(|| "runtime_lifecycle_record_missing".to_string())?
        } else {
            registry
                .lifecycle_records
                .remove(key)
                .ok_or_else(|| "runtime_lifecycle_record_missing".to_string())?
        };
        if lifecycle.profile_identity_digest != owner.profile_identity_digest {
            return Err("runtime_lifecycle_profile_identity_mismatch".to_string());
        }
        return Ok(lifecycle);
    }
    Ok(RuntimeLifecycleRecord {
        logical_browser_id: owner.browser_id.clone(),
        profile_identity_digest: owner.profile_identity_digest.clone(),
        owner_generation: owner.owner_generation,
        lifecycle_state: RuntimeLaneLifecycleState::Unknown,
        cleanup_obligation_state: CleanupObligationState::Unknown,
        process_group_id: None,
        package_launch_identity_digest: None,
        terminal_evidence: Vec::new(),
    })
}

fn store_lifecycle(
    registry: &mut RuntimeOwnerRegistry,
    lifecycle: RuntimeLifecycleRecord,
) -> Result<(), String> {
    if lifecycle.logical_browser_id.trim().is_empty()
        || lifecycle.profile_identity_digest.trim().is_empty()
        || lifecycle.owner_generation == 0
    {
        return Err("runtime_lifecycle_record_invalid".to_string());
    }
    if registry
        .lifecycle_records
        .get(&lifecycle.logical_browser_id)
        == Some(&lifecycle)
    {
        return Ok(());
    }
    registry
        .lifecycle_records
        .insert(lifecycle.logical_browser_id.clone(), lifecycle);
    registry.revision = registry.revision.saturating_add(1);
    Ok(())
}

fn owner_error(error: OwnerTransferError) -> String {
    format!("runtime_owner_transfer_{:?}: {}", error.code, error.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::ServiceState;
    use crate::native::service_store::ServiceStateStore;
    use crate::runtime_owner_transfer::ProfileOwnerState;
    use sha2::{Digest, Sha256};
    use std::sync::Mutex;

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

        fn mutate<T>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut state = self.state.lock().unwrap();
            mutator(&mut state)
        }
    }

    fn digest(seed: &str) -> String {
        format!("{:x}", Sha256::digest(seed.as_bytes()))
    }

    fn owner() -> ProfileOwner {
        ProfileOwner {
            owner_id: "owner-a".to_string(),
            profile_identity_digest: digest("profile-a"),
            state: ProfileOwnerState::Ready,
            owner_generation: 7,
            browser_id: "browser-a".to_string(),
            daemon_session_route: "session-a".to_string(),
            process_instance_digest: digest("process-a"),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: digest("cdp-a"),
            target_set_digest: digest("targets-a"),
            pending_transfer: None,
            last_transition: None,
        }
    }

    fn cooperative_request() -> OwnerTransferRequest {
        OwnerTransferRequest {
            mode: crate::runtime_adoption::BrowserAdoptionMode::CooperativeTransfer,
            logical_browser_id: "browser-a".to_string(),
            profile_identity_digest: digest("profile-a"),
            expected_owner_id: Some("owner-a".to_string()),
            expected_owner_generation: 7,
            candidate_owner_id: "owner-b".to_string(),
            candidate_daemon_session_route: "session-b".to_string(),
            process_instance_digest: digest("process-a"),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: digest("cdp-a"),
            target_set_digest: digest("targets-a"),
            selected_target_identity_digest: digest("target-a"),
            transfer_nonce_digest: digest("transfer-a"),
        }
    }

    fn registered_repository() -> MemoryRepository {
        let repository = MemoryRepository::default();
        RuntimeLifecycleAuthority::new(&repository)
            .transition(RuntimeLifecycleIntent::RegisterCurrentOwner(owner()))
            .unwrap();
        repository
    }

    #[test]
    fn registration_atomically_creates_owner_and_cleanup_accountability() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);

        let transition = authority
            .transition(RuntimeLifecycleIntent::RegisterCurrentOwner(owner()))
            .unwrap();

        assert!(matches!(
            transition,
            RuntimeLifecycleTransition::OwnerRegistered(_)
        ));
        let state = repository.load_snapshot().unwrap();
        assert_eq!(state.runtime_owner_registry.owners.len(), 1);
        let lifecycle = &state.runtime_owner_registry.lifecycle_records["browser-a"];
        assert_eq!(lifecycle.owner_generation, 7);
        assert_eq!(
            lifecycle.lifecycle_state,
            crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Ready
        );
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            crate::runtime_owner_transfer::CleanupObligationState::Owned
        );
    }

    #[test]
    fn managed_lane_registration_derives_identity_and_returns_effect_binding() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let registration = ManagedLaneRegistration {
            logical_browser_id: "session:alpha".to_string(),
            profile_root: std::env::temp_dir().join("agent-browser-lifecycle-profile-alpha"),
            daemon_session_route: "alpha".to_string(),
            process_group_id: Some(4100),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4100,
                start_token: "linux:boot:4100".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            browser_family: "chrome".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9444/devtools/browser/example".to_string(),
            target_ids: vec!["target-b".to_string(), "target-a".to_string()],
        };

        let binding = authority
            .register_managed_lane(registration.clone())
            .unwrap();
        assert!(binding.effect_capable);
        let registered = repository.load_snapshot().unwrap();
        let lifecycle = &registered.runtime_owner_registry.lifecycle_records["session:alpha"];
        assert_eq!(lifecycle.process_group_id, Some(4100));
        assert!(lifecycle
            .package_launch_identity_digest
            .as_deref()
            .is_some_and(is_digest));
        let mut effect_binding = binding.clone();
        authority.authorize_effect(&mut effect_binding).unwrap();
        let revision = repository
            .load_snapshot()
            .unwrap()
            .runtime_owner_registry
            .revision;
        let initial_target_set_digest = repository
            .load_snapshot()
            .unwrap()
            .runtime_owner_registry
            .owner(&binding.claim.profile_identity_digest)
            .unwrap()
            .target_set_digest
            .clone();

        assert_eq!(
            authority
                .register_managed_lane(registration.clone())
                .unwrap(),
            binding
        );
        assert_eq!(
            repository
                .load_snapshot()
                .unwrap()
                .runtime_owner_registry
                .revision,
            revision
        );

        let mut refreshed_registration = registration;
        refreshed_registration
            .target_ids
            .push("target-c".to_string());
        let refreshed = authority
            .register_managed_lane(refreshed_registration)
            .unwrap();
        assert_eq!(
            refreshed.claim.owner_generation,
            binding.claim.owner_generation
        );
        assert_ne!(
            repository
                .load_snapshot()
                .unwrap()
                .runtime_owner_registry
                .owner(&binding.claim.profile_identity_digest)
                .unwrap()
                .target_set_digest,
            initial_target_set_digest
        );
    }

    #[test]
    fn terminal_lane_can_activate_one_explicit_replacement_generation() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let profile_root = std::env::temp_dir().join("agent-browser-lifecycle-replacement");
        let registration = ManagedLaneRegistration {
            logical_browser_id: "session:replacement".to_string(),
            profile_root,
            daemon_session_route: "replacement".to_string(),
            process_group_id: Some(4200),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4200,
                start_token: "linux:boot:4200".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            browser_family: "chrome".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9555/devtools/browser/old".to_string(),
            target_ids: vec!["target-old".to_string()],
        };
        let binding = authority
            .register_managed_lane(registration.clone())
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::BeginClose {
                claim: binding.claim.clone(),
            })
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::CompleteClose {
                logical_browser_id: binding.claim.logical_browser_id.clone(),
                profile_identity_digest: binding.claim.profile_identity_digest.clone(),
                expected_owner_generation: binding.claim.owner_generation,
                terminal_evidence: vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
            })
            .unwrap();

        let mut replacement = registration;
        replacement.process_identity.pid = 4201;
        replacement.process_identity.start_token = "linux:boot:4201".to_string();
        replacement.process_group_id = Some(4201);
        replacement.cdp_endpoint = "ws://127.0.0.1:9556/devtools/browser/new".to_string();
        replacement.target_ids = vec!["target-new".to_string()];
        let replacement_binding = authority.register_managed_lane(replacement).unwrap();

        assert_eq!(replacement_binding.claim.owner_generation, 2);
        let state = repository.load_snapshot().unwrap();
        let lifecycle = &state.runtime_owner_registry.lifecycle_records["session:replacement"];
        assert_eq!(lifecycle.lifecycle_state, RuntimeLaneLifecycleState::Ready);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        assert!(lifecycle.terminal_evidence.is_empty());
    }

    #[test]
    fn transfer_commit_abort_and_reverse_move_cleanup_with_authority() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let request = cooperative_request();
        let original_claim = OwnerAuthorityClaim::from_owner(&owner());

        authority
            .transition(RuntimeLifecycleIntent::BeginTransfer(request.clone()))
            .unwrap();
        let transferring = repository.load_snapshot().unwrap();
        assert_eq!(
            transferring.runtime_owner_registry.lifecycle_records["browser-a"]
                .cleanup_obligation_state,
            CleanupObligationState::Transferring
        );
        assert!(authority
            .authorize_relinquish_after_transfer(&original_claim)
            .is_err());

        authority
            .transition(RuntimeLifecycleIntent::AbortTransfer {
                profile_identity_digest: request.profile_identity_digest.clone(),
                expected_owner_id: request.expected_owner_id.clone().unwrap(),
                expected_owner_generation: request.expected_owner_generation,
                transfer_nonce_digest: request.transfer_nonce_digest.clone(),
            })
            .unwrap();
        let aborted = repository.load_snapshot().unwrap();
        assert_eq!(
            aborted.runtime_owner_registry.lifecycle_records["browser-a"].cleanup_obligation_state,
            CleanupObligationState::Owned
        );

        authority
            .transition(RuntimeLifecycleIntent::BeginTransfer(request.clone()))
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::CommitCandidate(
                CandidateOwnerAttachment::from_request(&request, 8),
            ))
            .unwrap();
        let committed = repository.load_snapshot().unwrap();
        let lifecycle = &committed.runtime_owner_registry.lifecycle_records["browser-a"];
        assert_eq!(lifecycle.owner_generation, 8);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        authority
            .authorize_relinquish_after_transfer(&original_claim)
            .unwrap();
        let candidate_claim = OwnerAuthorityClaim::from_owner(
            committed
                .runtime_owner_registry
                .owner(&digest("profile-a"))
                .unwrap(),
        );
        let mut candidate_binding = RuntimeOwnerBinding::effect_capable(candidate_claim.clone());
        authority
            .refresh_managed_lane(
                &mut candidate_binding,
                "ws://127.0.0.1:9555/devtools/browser/refreshed",
                vec!["target-refreshed".to_string()],
            )
            .unwrap();
        assert_eq!(candidate_binding.claim, candidate_claim);

        authority
            .transition(RuntimeLifecycleIntent::ReverseTransfer(
                ReverseOwnerTransferRequest {
                    profile_identity_digest: digest("profile-a"),
                    expected_candidate_owner_id: "owner-b".to_string(),
                    expected_candidate_owner_generation: 8,
                    transfer_nonce_digest: digest("transfer-a"),
                    reverse_nonce_digest: digest("reverse-a"),
                },
            ))
            .unwrap();
        let reversed = repository.load_snapshot().unwrap();
        let lifecycle = &reversed.runtime_owner_registry.lifecycle_records["browser-a"];
        assert_eq!(lifecycle.owner_generation, 9);
        assert_eq!(lifecycle.lifecycle_state, RuntimeLaneLifecycleState::Ready);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        authority
            .authorize_relinquish_after_transfer(&candidate_claim)
            .unwrap();
    }

    #[test]
    fn failed_transition_commits_neither_owner_nor_cleanup_state() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let request = cooperative_request();
        authority
            .transition(RuntimeLifecycleIntent::BeginTransfer(request.clone()))
            .unwrap();
        let before = repository.load_snapshot().unwrap().runtime_owner_registry;
        let mut mismatched = CandidateOwnerAttachment::from_request(&request, 8);
        mismatched.target_set_digest = digest("wrong-targets");

        assert!(authority
            .transition(RuntimeLifecycleIntent::CommitCandidate(mismatched))
            .is_err());

        assert_eq!(
            repository.load_snapshot().unwrap().runtime_owner_registry,
            before
        );
    }

    #[test]
    fn retained_and_close_effects_require_the_exact_owner_generation() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let claim = OwnerAuthorityClaim::from_owner(&owner());
        let mut stale = claim.clone();
        stale.owner_generation -= 1;

        assert!(authority
            .transition(RuntimeLifecycleIntent::PreserveRetained { claim: stale })
            .is_err());
        authority
            .transition(RuntimeLifecycleIntent::PreserveRetained {
                claim: claim.clone(),
            })
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::BeginClose { claim })
            .unwrap();

        let closing = repository.load_snapshot().unwrap();
        let lifecycle = &closing.runtime_owner_registry.lifecycle_records["browser-a"];
        assert_eq!(
            lifecycle.lifecycle_state,
            RuntimeLaneLifecycleState::Closing
        );
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
    }

    #[test]
    fn terminal_state_requires_exact_close_identity_and_terminal_evidence() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        authority
            .transition(RuntimeLifecycleIntent::BeginClose {
                claim: OwnerAuthorityClaim::from_owner(&owner()),
            })
            .unwrap();

        assert!(authority
            .transition(RuntimeLifecycleIntent::CompleteClose {
                logical_browser_id: "browser-a".to_string(),
                profile_identity_digest: digest("profile-a"),
                expected_owner_generation: 7,
                terminal_evidence: Vec::new(),
            })
            .is_err());
        authority
            .transition(RuntimeLifecycleIntent::CompleteClose {
                logical_browser_id: "browser-a".to_string(),
                profile_identity_digest: digest("profile-a"),
                expected_owner_generation: 7,
                terminal_evidence: vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
            })
            .unwrap();

        let terminal = repository.load_snapshot().unwrap();
        let lifecycle = &terminal.runtime_owner_registry.lifecycle_records["browser-a"];
        assert_eq!(
            lifecycle.lifecycle_state,
            RuntimeLaneLifecycleState::Terminal
        );
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Satisfied
        );
    }

    #[test]
    fn legacy_owner_revocation_preserves_one_cleanup_obligation() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);

        authority
            .transition(RuntimeLifecycleIntent::RevokeLegacyOwner {
                profile_identity_digest: digest("profile-a"),
                logical_browser_id: "browser-a".to_string(),
                expected_daemon_session_route: "session-a".to_string(),
                expected_owner_id: "owner-a".to_string(),
                expected_owner_generation: 7,
            })
            .unwrap();

        let state = repository.load_snapshot().unwrap();
        assert_eq!(state.runtime_owner_registry.lifecycle_records.len(), 1);
        let lifecycle = &state.runtime_owner_registry.lifecycle_records["browser-a"];
        assert_eq!(lifecycle.owner_generation, 8);
        assert_eq!(
            lifecycle.lifecycle_state,
            RuntimeLaneLifecycleState::Retained
        );
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
    }

    #[test]
    fn reversed_orphan_adoption_authorizes_candidate_relinquish() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let orphan = authority
            .revoke_legacy_owner(&digest("profile-a"), "browser-a", "session-a", "owner-a", 7)
            .unwrap();
        let mut request = cooperative_request();
        request.mode = crate::runtime_adoption::BrowserAdoptionMode::OrphanAdoption;
        request.expected_owner_id = Some(orphan.owner_id.clone());
        request.expected_owner_generation = orphan.owner_generation;
        request.transfer_nonce_digest = digest("orphan-transfer");
        let proposal = authority.begin_transfer(request.clone()).unwrap();
        let committed = authority
            .commit_candidate(CandidateOwnerAttachment::from_request(
                &request,
                proposal.candidate_owner_generation,
            ))
            .unwrap();
        let candidate = repository
            .load_snapshot()
            .unwrap()
            .runtime_owner_registry
            .owner(&digest("profile-a"))
            .cloned()
            .unwrap();
        let candidate_claim = OwnerAuthorityClaim::from_owner(&candidate);

        authority
            .reverse_transfer(ReverseOwnerTransferRequest {
                profile_identity_digest: digest("profile-a"),
                expected_candidate_owner_id: committed.candidate_owner_id,
                expected_candidate_owner_generation: committed.candidate_owner_generation,
                transfer_nonce_digest: request.transfer_nonce_digest,
                reverse_nonce_digest: digest("orphan-reverse"),
            })
            .unwrap();

        authority
            .authorize_relinquish_after_transfer(&candidate_claim)
            .expect("a reversed orphan adoption must release the candidate daemon");
        let restored = repository.load_snapshot().unwrap();
        let owner = restored
            .runtime_owner_registry
            .owner(&digest("profile-a"))
            .unwrap();
        assert_eq!(owner.state, ProfileOwnerState::Orphaned);
        assert!(owner.owner_generation > candidate_claim.owner_generation);
    }
}
