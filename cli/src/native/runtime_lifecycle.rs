use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::runtime_owner_transfer::{
    CandidateOwnerAttachment, CleanupObligationState, OwnerAuthorityClaim, OwnerTransferProposal,
    OwnerTransferReceipt, OwnerTransferRequest, ProfileOwner, ReverseOwnerTransferRequest,
    RuntimeLaneLifecycleState, RuntimeLifecycleRecord, RuntimeOwnerBinding, RuntimeOwnerRegistry,
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
    MigrateTerminalProfileReplacement {
        expected_owner: OwnerAuthorityClaim,
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
    },
    SupersedeObservedOwner {
        expected_owner: OwnerAuthorityClaim,
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
    BeginRecoveryClose {
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

pub(crate) use agent_browser_lease_authority::RuntimeLifecycleTransition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeEffectAdmission {
    CurrentOwner,
    TerminalReplacement,
}

/// Hydrate and validate the daemon's effect-capable owner binding through the
/// concrete lifecycle authority. The command dispatcher supplies only its
/// cached binding and action context; repository ownership stays in this deep
/// module.
pub(crate) fn admit_default_action_effect(
    binding: &mut Option<RuntimeOwnerBinding>,
    action: &str,
    session_id: &str,
) -> Result<(), String> {
    let repository = LockedServiceStateRepository::default_json()?;
    if binding.is_none() {
        *binding =
            crate::runtime_owner_transfer::owner_binding_for_session(&repository, session_id)?;
    }
    let admission = binding
        .as_mut()
        .map(|binding| {
            RuntimeLifecycleAuthority::new(&repository)
                .admit_action_effect(binding, action, session_id)
        })
        .transpose()?;
    if admission == Some(RuntimeEffectAdmission::TerminalReplacement) {
        *binding = None;
    }
    Ok(())
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
            state.apply_runtime_lifecycle_transition_atomically(prepare_lifecycle_intent(
                intent.clone(),
            ))
        })
    }

    fn transition_terminal_replacement_with_profile_sync(
        &self,
        intent: RuntimeLifecycleIntent,
        profile_id: &str,
        profile_root: &std::path::Path,
    ) -> Result<RuntimeLifecycleTransition, String> {
        let user_data_dir = profile_root
            .to_str()
            .ok_or_else(|| "runtime_lifecycle_profile_path_invalid".to_string())?
            .to_string();
        self.repository.mutate(|state| {
            let profile = state
                .profiles
                .get(profile_id)
                .ok_or_else(|| "runtime_lifecycle_profile_record_missing".to_string())?;
            if profile.id != profile_id
                || !canonical_route_viewer_runtime_profile(profile_id)
                || profile_id.trim().is_empty()
            {
                return Err("runtime_lifecycle_profile_record_sync_rejected".to_string());
            }
            let prepared_intent = prepare_lifecycle_intent(intent.clone());
            state.apply_runtime_lifecycle_transition_with_profile_sync_atomically(
                profile_id,
                user_data_dir.clone(),
                prepared_intent,
            )
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
        let authorizes_effect = |claim: &OwnerAuthorityClaim| {
            registry.authorizes(claim)
                && registry
                    .lifecycle_records()
                    .get(&claim.logical_browser_id)
                    .is_some_and(|lifecycle| {
                        lifecycle.profile_identity_digest == claim.profile_identity_digest
                            && lifecycle.owner_generation == claim.owner_generation
                            && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Ready
                            && lifecycle.cleanup_obligation_state == CleanupObligationState::Owned
                    })
        };
        if authorizes_effect(&binding.claim) {
            return Ok(());
        }
        if let Some(claim) = registry
            .refreshed_claim_after_reverse(&binding.claim)
            .filter(|claim| authorizes_effect(claim))
        {
            binding.claim = claim;
            return Ok(());
        }
        Err(
            "runtime_owner_generation_stale: daemon is no longer the effect-capable browser owner"
                .to_string(),
        )
    }

    /// Admit an ordinary owner effect, or release the in-memory observation
    /// binding that would otherwise make an exact terminal replacement
    /// unreachable for browser creation. The replacement still has to win the registry transition
    /// in `register_managed_lane` before it receives effect authority.
    pub(crate) fn admit_action_effect(
        &self,
        binding: &mut RuntimeOwnerBinding,
        action: &str,
        candidate_session: &str,
    ) -> Result<RuntimeEffectAdmission, String> {
        if matches!(action, "remote_view_open" | "tab_new" | "window_new")
            && binding.claim.logical_browser_id == format!("session:{candidate_session}")
        {
            let registry = self.repository.load_snapshot()?.runtime_owner_registry;
            let owner = registry.owner(&binding.claim.profile_identity_digest);
            let lifecycle = registry
                .lifecycle_records()
                .get(&binding.claim.logical_browser_id);
            if registry.authorizes(&binding.claim)
                && owner.is_some_and(|owner| owner.pending_transfer.is_none())
                && lifecycle.is_some_and(|lifecycle| {
                    lifecycle.profile_identity_digest == binding.claim.profile_identity_digest
                        && lifecycle.owner_generation == binding.claim.owner_generation
                        && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
                        && lifecycle.cleanup_obligation_state == CleanupObligationState::Satisfied
                })
            {
                return Ok(RuntimeEffectAdmission::TerminalReplacement);
            }
        }
        self.authorize_effect(binding)?;
        Ok(RuntimeEffectAdmission::CurrentOwner)
    }

    pub(crate) fn complete_close_and_release_binding(
        &self,
        binding: &mut Option<RuntimeOwnerBinding>,
        claim: OwnerAuthorityClaim,
        terminal_evidence: Vec<String>,
    ) -> Result<RuntimeLifecycleRecord, String> {
        if !binding
            .as_ref()
            .is_some_and(|binding| binding.claim == claim)
        {
            return Err("runtime_lifecycle_close_binding_mismatch".to_string());
        }
        let transition = self.transition(RuntimeLifecycleIntent::CompleteClose {
            logical_browser_id: claim.logical_browser_id,
            profile_identity_digest: claim.profile_identity_digest,
            expected_owner_generation: claim.owner_generation,
            terminal_evidence,
        })?;
        let RuntimeLifecycleTransition::LaneUpdated(record) = transition else {
            return Err("runtime_lifecycle_close_outcome_mismatch".to_string());
        };
        *binding = None;
        Ok(record)
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
    /// another generation owns both effects and the cleanup obligation. A
    /// verified orphan adoption may canonicalize a historical browser alias,
    /// so the restored current owner's lifecycle key is authoritative.
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
            .lifecycle_records()
            .get(&current.browser_id)
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
            || current.process_instance_digest != stale_claim.process_instance_digest
            || current.owner_generation <= stale_claim.owner_generation
            || lifecycle.profile_identity_digest != stale_claim.profile_identity_digest
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
            .lifecycle_records()
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
            agent_browser_lease_authority::canonical_profile_identity_digest(
                &registration.profile_root,
            )?;
        let process_instance_digest = digest_json(&registration.process_identity)?;
        let cdp_endpoint_identity_digest = digest_text(&registration.cdp_endpoint);
        let target_set_digest = digest_json(&registration.target_ids)?;
        let registry = self.repository.load_snapshot()?.runtime_owner_registry;
        let existing = registry.owner(&profile_identity_digest).cloned();
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
            browser_id: registration.logical_browser_id.clone(),
            daemon_session_route: registration.daemon_session_route.clone(),
            process_instance_digest,
            browser_family: registration.browser_family,
            cdp_endpoint_identity_digest,
            target_set_digest,
            pending_transfer: None,
            last_transition: None,
        };
        if existing.is_none()
            && canonical_route_viewer_runtime_profile(&owner.daemon_session_route)
            && owner.browser_id == format!("session:{}", owner.daemon_session_route)
        {
            let mut migration_sources = registry.owners().values().filter(|current| {
                current.profile_identity_digest != owner.profile_identity_digest
                    && current.browser_id == owner.browser_id
                    && current.daemon_session_route == owner.daemon_session_route
            });
            let migration_source = migration_sources.next().cloned();
            if migration_sources.next().is_some() {
                return Err("runtime_lifecycle_terminal_profile_migration_ambiguous".to_string());
            }
            if let Some(current) = migration_source {
                let lifecycle = registry.lifecycle_records().get(&current.browser_id);
                let migration_is_exact = current.pending_transfer.is_none()
                    && lifecycle.is_some_and(|lifecycle| {
                        lifecycle.profile_identity_digest == current.profile_identity_digest
                            && lifecycle.owner_generation == current.owner_generation
                            && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
                            && lifecycle.cleanup_obligation_state
                                == CleanupObligationState::Satisfied
                            && terminal_cleanup_evidence_complete(lifecycle)
                    })
                    && !registry
                        .principal_bindings()
                        .contains_key(&current.profile_identity_digest)
                    && !registry
                        .principal_bindings()
                        .contains_key(&owner.profile_identity_digest);
                if migration_is_exact {
                    let expected_owner = OwnerAuthorityClaim::from_owner(&current);
                    let mut replacement = owner;
                    replacement.owner_generation = current
                        .owner_generation
                        .checked_add(1)
                        .ok_or_else(|| "runtime_lifecycle_generation_exhausted".to_string())?;
                    let package_launch_identity_digest = package_launch_identity_digest(
                        &replacement,
                        registration.process_group_id,
                    )?;
                    let activated = match self.transition_terminal_replacement_with_profile_sync(
                        RuntimeLifecycleIntent::MigrateTerminalProfileReplacement {
                            expected_owner,
                            owner: replacement,
                            process_group_id: registration.process_group_id,
                            package_launch_identity_digest,
                        },
                        &registration.daemon_session_route,
                        &registration.profile_root,
                    )? {
                        RuntimeLifecycleTransition::TerminalReplacementActivated(owner) => owner,
                        _ => {
                            return Err(
                                "runtime_lifecycle_profile_migration_outcome_mismatch".to_string()
                            )
                        }
                    };
                    return Ok(RuntimeOwnerBinding::effect_capable(
                        OwnerAuthorityClaim::from_owner(&activated),
                    ));
                }
            }
        }
        if let Some(current) = existing {
            let lifecycle = self
                .repository
                .load_snapshot()?
                .runtime_owner_registry
                .lifecycle_records()
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
                let replacement_package_launch_identity_digest =
                    package_launch_identity_digest(&replacement, registration.process_group_id)?;
                let replacement_intent = RuntimeLifecycleIntent::ActivateTerminalReplacement {
                    owner: replacement,
                    process_group_id: registration.process_group_id,
                    package_launch_identity_digest: replacement_package_launch_identity_digest,
                };
                let transition =
                    if canonical_route_viewer_runtime_profile(&registration.daemon_session_route)
                        && registration.logical_browser_id
                            == format!("session:{}", registration.daemon_session_route)
                    {
                        self.transition_terminal_replacement_with_profile_sync(
                            replacement_intent,
                            &registration.daemon_session_route,
                            &registration.profile_root,
                        )
                    } else {
                        self.transition(replacement_intent)
                    }?;
                let activated = match transition {
                    RuntimeLifecycleTransition::TerminalReplacementActivated(owner) => owner,
                    _ => return Err("runtime_lifecycle_replacement_outcome_mismatch".to_string()),
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
                let expected_owner = OwnerAuthorityClaim::from_owner(&current);
                let mut replacement = owner;
                replacement.owner_generation = current
                    .owner_generation
                    .checked_add(1)
                    .ok_or_else(|| "runtime_lifecycle_generation_exhausted".to_string())?;
                let package_launch_identity_digest =
                    package_launch_identity_digest(&replacement, registration.process_group_id)?;
                let activated =
                    match self.transition(RuntimeLifecycleIntent::SupersedeObservedOwner {
                        expected_owner,
                        owner: replacement,
                        process_group_id: registration.process_group_id,
                        package_launch_identity_digest,
                    })? {
                        RuntimeLifecycleTransition::ObservedOwnerSuperseded(owner) => owner,
                        _ => {
                            return Err("runtime_lifecycle_observed_supersession_outcome_mismatch"
                                .to_string())
                        }
                    };
                return Ok(RuntimeOwnerBinding::effect_capable(
                    OwnerAuthorityClaim::from_owner(&activated),
                ));
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
        let package_launch_identity_digest =
            package_launch_identity_digest(&owner, registration.process_group_id)?;
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

    /// Observe retained owner history without promoting it into launch
    /// authority. A live profile collision is enforced by the browser's
    /// process lock. After a successful launch, registration atomically
    /// supersedes the exact observed generation so stale daemons remain fenced.
    pub(crate) fn ensure_managed_lane_launch_allowed(
        &self,
        profile_root: &std::path::Path,
    ) -> Result<(), String> {
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(profile_root)?;
        let _observed_owner = self
            .repository
            .load_snapshot()?
            .runtime_owner_registry
            .owner(&profile_identity_digest)
            .cloned();
        Ok(())
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

pub(crate) fn canonical_route_viewer_runtime_profile(profile_id: &str) -> bool {
    profile_id
        .strip_prefix("rdp-guac-route-")
        .and_then(|suffix| suffix.strip_suffix("-viewer"))
        .is_some_and(|route| {
            route.len() == 1 && route.bytes().all(|byte| byte.is_ascii_lowercase())
        })
}

fn terminal_cleanup_evidence_complete(lifecycle: &RuntimeLifecycleRecord) -> bool {
    let process_absence_proven = lifecycle.terminal_evidence.iter().any(|evidence| {
        evidence == "exact_process_exited"
            || evidence.starts_with("service_reconcile_process_group_absent:")
    });
    let profile_lock_release_proven = lifecycle.terminal_evidence.iter().any(|evidence| {
        evidence == "profile_lock_released"
            || evidence == "service_reconcile_profile_lock_absent"
            || evidence.starts_with("service_reconcile_profile_lock_stale_pid_absent:")
    });
    process_absence_proven && profile_lock_release_proven
}

fn apply_transition(
    registry: &mut RuntimeOwnerRegistry,
    intent: RuntimeLifecycleIntent,
) -> Result<RuntimeLifecycleTransition, String> {
    registry.apply_lifecycle_transition(prepare_lifecycle_intent(intent))
}

fn prepare_lifecycle_intent(
    intent: RuntimeLifecycleIntent,
) -> agent_browser_lease_authority::RuntimeLifecycleIntent {
    use agent_browser_lease_authority::RuntimeLifecycleIntent as KernelIntent;

    // Host boot observation and canonical route naming remain CLI responsibilities.
    match intent {
        RuntimeLifecycleIntent::RegisterCurrentOwner(owner) => {
            KernelIntent::RegisterCurrentOwner(owner)
        }
        RuntimeLifecycleIntent::RegisterManagedLane {
            owner,
            process_group_id,
            package_launch_identity_digest,
        } => KernelIntent::RegisterManagedLane {
            owner,
            process_group_id,
            package_launch_identity_digest,
            boot_epoch: crate::process_identity::current_boot_epoch(),
        },
        RuntimeLifecycleIntent::ActivateTerminalReplacement {
            owner,
            process_group_id,
            package_launch_identity_digest,
        } => KernelIntent::ActivateTerminalReplacement {
            owner,
            process_group_id,
            package_launch_identity_digest,
            boot_epoch: crate::process_identity::current_boot_epoch(),
        },
        RuntimeLifecycleIntent::MigrateTerminalProfileReplacement {
            expected_owner,
            owner,
            process_group_id,
            package_launch_identity_digest,
        } => KernelIntent::MigrateTerminalProfileReplacement {
            canonical_route_viewer_profile: owner.browser_id
                == format!("session:{}", owner.daemon_session_route)
                && canonical_route_viewer_runtime_profile(&owner.daemon_session_route),
            expected_owner,
            owner,
            process_group_id,
            package_launch_identity_digest,
            boot_epoch: crate::process_identity::current_boot_epoch(),
        },
        RuntimeLifecycleIntent::SupersedeObservedOwner {
            expected_owner,
            owner,
            process_group_id,
            package_launch_identity_digest,
        } => KernelIntent::SupersedeObservedOwner {
            expected_owner,
            owner,
            process_group_id,
            package_launch_identity_digest,
            boot_epoch: crate::process_identity::current_boot_epoch(),
        },
        RuntimeLifecycleIntent::RefreshCurrentOwnerEvidence {
            claim,
            cdp_endpoint_identity_digest,
            target_set_digest,
        } => KernelIntent::RefreshCurrentOwnerEvidence {
            claim,
            cdp_endpoint_identity_digest,
            target_set_digest,
        },
        RuntimeLifecycleIntent::BeginTransfer(request) => KernelIntent::BeginTransfer(request),
        RuntimeLifecycleIntent::CommitCandidate(attachment) => {
            KernelIntent::CommitCandidate(attachment)
        }
        RuntimeLifecycleIntent::AbortTransfer {
            profile_identity_digest,
            expected_owner_id,
            expected_owner_generation,
            transfer_nonce_digest,
        } => KernelIntent::AbortTransfer {
            profile_identity_digest,
            expected_owner_id,
            expected_owner_generation,
            transfer_nonce_digest,
        },
        RuntimeLifecycleIntent::ReverseTransfer(request) => KernelIntent::ReverseTransfer(request),
        RuntimeLifecycleIntent::RevokeLegacyOwner {
            profile_identity_digest,
            logical_browser_id,
            expected_daemon_session_route,
            expected_owner_id,
            expected_owner_generation,
        } => KernelIntent::RevokeLegacyOwner {
            profile_identity_digest,
            logical_browser_id,
            expected_daemon_session_route,
            expected_owner_id,
            expected_owner_generation,
        },
        RuntimeLifecycleIntent::PreserveRetained { claim } => {
            KernelIntent::PreserveRetained { claim }
        }
        RuntimeLifecycleIntent::BeginRecoveryClose { claim } => {
            KernelIntent::BeginRecoveryClose { claim }
        }
        RuntimeLifecycleIntent::BeginClose { claim } => KernelIntent::BeginClose { claim },
        RuntimeLifecycleIntent::CompleteClose {
            logical_browser_id,
            profile_identity_digest,
            expected_owner_generation,
            terminal_evidence,
        } => KernelIntent::CompleteClose {
            logical_browser_id,
            profile_identity_digest,
            expected_owner_generation,
            terminal_evidence,
        },
    }
}

/// Begin an exact abandoned-browser close inside an already locked, pure
/// Service State mutation. Callers must first validate inactivity, retention,
/// and the sealed lifecycle baseline; this helper preserves the lifecycle
/// authority's owner-generation transition instead of editing registry rows.
pub(crate) fn begin_abandoned_browser_close(
    registry: &mut RuntimeOwnerRegistry,
    claim: OwnerAuthorityClaim,
) -> Result<RuntimeLifecycleRecord, String> {
    let transition = apply_transition(registry, RuntimeLifecycleIntent::BeginClose { claim })?;
    let RuntimeLifecycleTransition::LaneUpdated(record) = transition else {
        return Err("runtime_lifecycle_abandoned_close_outcome_mismatch".to_string());
    };
    Ok(record)
}

/// Complete a closing lane inside an already locked Service State mutation.
///
/// Service reconciliation uses this seam only after independently proving
/// that the recorded process group and profile lock are both absent. The
/// lifecycle compare-and-swap remains authoritative for the exact logical
/// browser, profile identity, and owner generation.
pub(crate) fn complete_reconciled_close(
    registry: &mut RuntimeOwnerRegistry,
    logical_browser_id: String,
    profile_identity_digest: String,
    expected_owner_generation: u64,
    terminal_evidence: Vec<String>,
) -> Result<RuntimeLifecycleRecord, String> {
    let transition = apply_transition(
        registry,
        RuntimeLifecycleIntent::CompleteClose {
            logical_browser_id,
            profile_identity_digest,
            expected_owner_generation,
            terminal_evidence,
        },
    )?;
    let RuntimeLifecycleTransition::LaneUpdated(record) = transition else {
        return Err("runtime_lifecycle_reconciled_close_outcome_mismatch".to_string());
    };
    Ok(record)
}

/// Reconcile a ready lane whose exact process, profile lock, browser, session,
/// and tab projections are all absent. This is the crash/rollback counterpart
/// to the ordinary closing-lane path and preserves the same generation CAS.
pub(crate) fn complete_reconciled_abandoned_ready_lane(
    registry: &mut RuntimeOwnerRegistry,
    logical_browser_id: String,
    profile_identity_digest: String,
    expected_owner_generation: u64,
    terminal_evidence: Vec<String>,
) -> Result<RuntimeLifecycleRecord, String> {
    let owner = registry
        .owner(&profile_identity_digest)
        .filter(|owner| {
            owner.browser_id == logical_browser_id
                && owner.owner_generation == expected_owner_generation
                && owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                && owner.pending_transfer.is_none()
        })
        .cloned()
        .ok_or_else(|| "runtime_lifecycle_abandoned_owner_mismatch".to_string())?;
    let lifecycle = registry
        .lifecycle_records()
        .get(&logical_browser_id)
        .ok_or_else(|| "runtime_lifecycle_record_missing".to_string())?;
    if lifecycle.profile_identity_digest != profile_identity_digest
        || lifecycle.owner_generation != expected_owner_generation
        || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Ready
        || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
    {
        return Err("runtime_lifecycle_abandoned_ready_compare_and_swap_mismatch".to_string());
    }
    apply_transition(
        registry,
        RuntimeLifecycleIntent::BeginClose {
            claim: OwnerAuthorityClaim::from_owner(&owner),
        },
    )?;
    complete_reconciled_close(
        registry,
        logical_browser_id,
        profile_identity_digest,
        expected_owner_generation,
        terminal_evidence,
    )
}

/// Reconcile a transfer lifecycle row after its exact owner has already
/// returned to ready without pending transfer authority. Service reconciliation
/// calls this only after proving that the process group, profile lock, browser,
/// session, and tab projections are absent.
pub(crate) fn complete_reconciled_abandoned_transfer_lane(
    registry: &mut RuntimeOwnerRegistry,
    logical_browser_id: String,
    profile_identity_digest: String,
    expected_owner_generation: u64,
    terminal_evidence: Vec<String>,
) -> Result<RuntimeLifecycleRecord, String> {
    let owner = registry
        .owner(&profile_identity_digest)
        .filter(|owner| {
            owner.browser_id == logical_browser_id
                && owner.owner_generation == expected_owner_generation
                && owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                && owner.pending_transfer.is_none()
        })
        .cloned()
        .ok_or_else(|| "runtime_lifecycle_abandoned_transfer_owner_mismatch".to_string())?;
    let lifecycle = registry
        .lifecycle_records()
        .get(&logical_browser_id)
        .ok_or_else(|| "runtime_lifecycle_record_missing".to_string())?;
    if lifecycle.profile_identity_digest != profile_identity_digest
        || lifecycle.owner_generation != expected_owner_generation
        || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Transferring
        || lifecycle.cleanup_obligation_state != CleanupObligationState::Transferring
    {
        return Err("runtime_lifecycle_abandoned_transfer_compare_and_swap_mismatch".to_string());
    }
    apply_transition(
        registry,
        RuntimeLifecycleIntent::BeginClose {
            claim: OwnerAuthorityClaim::from_owner(&owner),
        },
    )?;
    complete_reconciled_close(
        registry,
        logical_browser_id,
        profile_identity_digest,
        expected_owner_generation,
        terminal_evidence,
    )
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
            mut mutator: impl FnMut(&mut ServiceState) -> Result<T, String>,
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
        assert_eq!(state.runtime_owner_registry.owners().len(), 1);
        let lifecycle = &state.runtime_owner_registry.lifecycle_records()["browser-a"];
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
    fn retained_lifecycle_transition_does_not_reauthenticate_prior_process_evidence() {
        let current_owner = owner();
        let mut registry = RuntimeOwnerRegistry::from_owner(current_owner.clone());
        crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
            .lifecycle_rows
            .insert(
                current_owner.browser_id.clone(),
                RuntimeLifecycleRecord {
                    logical_browser_id: current_owner.browser_id.clone(),
                    boot_epoch: Some("boot:synthetic-previous".to_string()),
                    profile_identity_digest: current_owner.profile_identity_digest.clone(),
                    owner_generation: current_owner.owner_generation,
                    lifecycle_state: RuntimeLaneLifecycleState::Ready,
                    cleanup_obligation_state: CleanupObligationState::Owned,
                    process_group_id: Some(4100),
                    package_launch_identity_digest: Some(digest("launch")),
                    terminal_evidence: Vec::new(),
                },
            );

        let RuntimeLifecycleTransition::LaneUpdated(lifecycle) = apply_transition(
            &mut registry,
            RuntimeLifecycleIntent::PreserveRetained {
                claim: OwnerAuthorityClaim::from_owner(&current_owner),
            },
        )
        .unwrap() else {
            panic!("retained transition must return its lifecycle record");
        };
        assert_eq!(
            lifecycle.boot_epoch.as_deref(),
            Some("boot:synthetic-previous")
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
        let lifecycle = &registered.runtime_owner_registry.lifecycle_records()["session:alpha"];
        assert_eq!(lifecycle.process_group_id, Some(4100));
        assert_eq!(
            lifecycle.boot_epoch,
            crate::process_identity::current_boot_epoch()
        );
        assert!(lifecycle
            .package_launch_identity_digest
            .as_deref()
            .is_some_and(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }));
        let mut effect_binding = binding.clone();
        authority.authorize_effect(&mut effect_binding).unwrap();
        let revision = repository
            .load_snapshot()
            .unwrap()
            .runtime_owner_registry
            .revision();
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
                .revision(),
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
    fn managed_lane_registration_replaces_legacy_browser_alias_after_transfer() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let mut registration = ManagedLaneRegistration {
            logical_browser_id: "session:legacy-route-alias".to_string(),
            profile_root: std::env::temp_dir().join("agent-browser-lifecycle-transfer-alias"),
            daemon_session_route: "candidate-route".to_string(),
            process_group_id: Some(4200),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4200,
                start_token: "linux:boot:4200".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            browser_family: "chrome".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9444/devtools/browser/example".to_string(),
            target_ids: vec!["target-a".to_string()],
        };
        let legacy = authority
            .register_managed_lane(registration.clone())
            .unwrap();
        repository
            .mutate(|state| {
                state
                    .runtime_owner_registry
                    .bind_principal_authority(
                        crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                            principal_id: "principal:test".to_string(),
                            profile_id: "profile:test".to_string(),
                            profile_identity_digest: legacy
                                .claim
                                .profile_identity_digest
                                .clone(),
                            capability_id: "profile-capability-v1:test".to_string(),
                            provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                            owner_generation: legacy.claim.owner_generation,
                        },
                    )
                    .map_err(|error| format!("{error:?}"))?;
                Ok(())
            })
            .unwrap();

        registration.logical_browser_id = "session:stable-browser".to_string();
        let canonical = authority.register_managed_lane(registration).unwrap();

        assert_eq!(canonical.claim.logical_browser_id, "session:stable-browser");
        assert_eq!(
            canonical.claim.owner_generation,
            legacy.claim.owner_generation + 1
        );
        let state = repository.load_snapshot().unwrap();
        assert!(!state
            .runtime_owner_registry
            .lifecycle_records()
            .contains_key("session:legacy-route-alias"));
        assert!(state
            .runtime_owner_registry
            .lifecycle_records()
            .contains_key("session:stable-browser"));
        let principal_binding = state
            .runtime_owner_registry
            .principal_bindings()
            .get(&canonical.claim.profile_identity_digest)
            .unwrap();
        assert_eq!(
            principal_binding.owner_generation,
            canonical.claim.owner_generation
        );
        assert!(state
            .runtime_owner_registry
            .principal_binding_is_current(Some(principal_binding)));
    }

    #[test]
    fn retained_owner_history_cannot_veto_managed_lane_launch() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let profile_root = std::env::temp_dir().join("agent-browser-lifecycle-prelaunch-admission");
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(&profile_root)
                .unwrap();
        let mut current = owner();
        current.profile_identity_digest = profile_identity_digest.clone();
        current.browser_id = "session:prelaunch-owner".to_string();
        repository
            .mutate(|state| {
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(current.clone());
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .insert(
                    current.browser_id.clone(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: current.browser_id.clone(),
                        boot_epoch: None,
                        profile_identity_digest: profile_identity_digest.clone(),
                        owner_generation: current.owner_generation,
                        lifecycle_state: RuntimeLaneLifecycleState::Ready,
                        cleanup_obligation_state: CleanupObligationState::Owned,
                        process_group_id: Some(4100),
                        package_launch_identity_digest: Some(digest("launch")),
                        terminal_evidence: Vec::new(),
                    },
                );
                Ok(())
            })
            .unwrap();

        authority
            .ensure_managed_lane_launch_allowed(&profile_root)
            .unwrap();

        repository
            .mutate(|state| {
                let mut registry_fixture = crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                );
                let lifecycle = registry_fixture
                    .lifecycle_rows
                    .get_mut(&current.browser_id)
                    .unwrap();
                lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Terminal;
                lifecycle.cleanup_obligation_state = CleanupObligationState::Satisfied;
                drop(registry_fixture);
                Ok(())
            })
            .unwrap();
        authority
            .ensure_managed_lane_launch_allowed(&profile_root)
            .unwrap();
    }

    #[test]
    fn successful_launch_supersedes_exact_nonterminal_observation_generation() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let profile_root =
            std::env::temp_dir().join("agent-browser-lifecycle-observed-supersession");
        let profile_identity_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(&profile_root)
                .unwrap();
        let mut current = owner();
        current.profile_identity_digest = profile_identity_digest.clone();
        current.browser_id = "session:stale-transferring".to_string();
        current.daemon_session_route = "stale-transferring".to_string();
        current.state = crate::runtime_owner_transfer::ProfileOwnerState::Ready;
        repository
            .mutate(|state| {
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(current.clone());
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .insert(
                    current.browser_id.clone(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: current.browser_id.clone(),
                        boot_epoch: None,
                        profile_identity_digest: profile_identity_digest.clone(),
                        owner_generation: current.owner_generation,
                        lifecycle_state: RuntimeLaneLifecycleState::Transferring,
                        cleanup_obligation_state: CleanupObligationState::Transferring,
                        process_group_id: Some(4100),
                        package_launch_identity_digest: Some(digest("stale-launch")),
                        terminal_evidence: Vec::new(),
                    },
                );
                Ok(())
            })
            .unwrap();

        let replacement = authority
            .register_managed_lane(ManagedLaneRegistration {
                logical_browser_id: "session:fresh".to_string(),
                profile_root,
                daemon_session_route: "fresh".to_string(),
                process_group_id: Some(4200),
                process_identity: crate::process_identity::RecordedProcessIdentity {
                    pid: 4200,
                    start_token: "linux:boot:4200".to_string(),
                    executable_path: Some("/opt/agent-browser/chrome".to_string()),
                    browser_family: Some("chrome".to_string()),
                },
                browser_family: "chrome".to_string(),
                cdp_endpoint: "ws://127.0.0.1:9555/devtools/browser/fresh".to_string(),
                target_ids: vec!["target-fresh".to_string()],
            })
            .unwrap();

        assert_eq!(
            replacement.claim.owner_generation,
            current.owner_generation + 1
        );
        assert_eq!(replacement.claim.logical_browser_id, "session:fresh");
        let state = repository.load_snapshot().unwrap();
        assert!(!state
            .runtime_owner_registry
            .lifecycle_records()
            .contains_key("session:stale-transferring"));
        assert_eq!(
            state.runtime_owner_registry.lifecycle_records()["session:fresh"].lifecycle_state,
            RuntimeLaneLifecycleState::Ready
        );
    }

    #[test]
    fn terminal_replacement_admission_releases_only_the_matching_observation_binding() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let registration = ManagedLaneRegistration {
            logical_browser_id: "session:replacement".to_string(),
            profile_root: std::env::temp_dir().join("agent-browser-lifecycle-admission"),
            daemon_session_route: "previous-route".to_string(),
            process_group_id: Some(4199),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4199,
                start_token: "linux:boot:4199".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            browser_family: "chrome".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9554/devtools/browser/old".to_string(),
            target_ids: vec!["target-old".to_string()],
        };
        let binding = authority.register_managed_lane(registration).unwrap();
        let mut ready_owner = binding.clone();
        assert_eq!(
            authority
                .admit_action_effect(&mut ready_owner, "remote_view_open", "replacement")
                .unwrap(),
            RuntimeEffectAdmission::CurrentOwner
        );
        authority
            .transition(RuntimeLifecycleIntent::BeginClose {
                claim: binding.claim.clone(),
            })
            .unwrap();
        let mut binding_slot = Some(binding.clone());
        authority
            .complete_close_and_release_binding(
                &mut binding_slot,
                binding.claim.clone(),
                vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
            )
            .unwrap();
        assert!(binding_slot.is_none());

        let mut terminal_effect = binding.clone();
        assert_eq!(
            authority
                .authorize_effect(&mut terminal_effect)
                .unwrap_err(),
            "runtime_owner_generation_stale: daemon is no longer the effect-capable browser owner"
        );

        let mut observation_binding = RuntimeOwnerBinding::observation_only(binding.claim.clone());
        assert_eq!(
            authority
                .admit_action_effect(&mut observation_binding, "remote_view_open", "replacement",)
                .unwrap(),
            RuntimeEffectAdmission::TerminalReplacement
        );

        let mut wrong_action = RuntimeOwnerBinding::observation_only(binding.claim.clone());
        for action in ["tab_new", "window_new"] {
            let mut reopening = RuntimeOwnerBinding::observation_only(binding.claim.clone());
            assert_eq!(
                authority
                    .admit_action_effect(&mut reopening, action, "replacement")
                    .unwrap(),
                RuntimeEffectAdmission::TerminalReplacement
            );
        }
        assert!(authority
            .admit_action_effect(&mut wrong_action, "navigate", "replacement")
            .is_err());

        let mut wrong_session = RuntimeOwnerBinding::observation_only(binding.claim.clone());
        assert!(authority
            .admit_action_effect(&mut wrong_session, "remote_view_open", "other-session")
            .is_err());

        let mut terminal_current_owner = binding;
        assert_eq!(
            authority
                .admit_action_effect(
                    &mut terminal_current_owner,
                    "remote_view_open",
                    "replacement",
                )
                .unwrap(),
            RuntimeEffectAdmission::TerminalReplacement
        );
    }

    #[test]
    fn terminal_lane_reopens_the_same_managed_process_as_a_new_generation() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let registration = ManagedLaneRegistration {
            logical_browser_id: "session:reopened".to_string(),
            profile_root: std::env::temp_dir().join("agent-browser-lifecycle-reopen"),
            daemon_session_route: "reopened".to_string(),
            process_group_id: Some(4250),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4250,
                start_token: "linux:boot:4250".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            browser_family: "chrome".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9557/devtools/browser/reopened".to_string(),
            target_ids: vec!["target-reopened".to_string()],
        };
        let binding = authority
            .register_managed_lane(registration.clone())
            .unwrap();
        repository
            .mutate(|state| {
                state
                    .runtime_owner_registry
                    .bind_principal_authority(
                        crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                            principal_id: "principal:test".to_string(),
                            profile_id: "profile:test".to_string(),
                            profile_identity_digest: binding
                                .claim
                                .profile_identity_digest
                                .clone(),
                            capability_id: "profile-capability-v1:test".to_string(),
                            provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                            owner_generation: binding.claim.owner_generation,
                        },
                    )
                    .map_err(|error| format!("{error:?}"))?;
                Ok(())
            })
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

        let replacement = authority.register_managed_lane(registration).unwrap();
        assert_eq!(replacement.claim.owner_generation, 2);
        authority
            .authorize_effect(&mut replacement.clone())
            .unwrap();
        let state = repository.load_snapshot().unwrap();
        let lifecycle = &state.runtime_owner_registry.lifecycle_records()["session:reopened"];
        assert_eq!(lifecycle.lifecycle_state, RuntimeLaneLifecycleState::Ready);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        assert!(lifecycle.terminal_evidence.is_empty());
        let principal_binding = state
            .runtime_owner_registry
            .principal_bindings()
            .get(&replacement.claim.profile_identity_digest)
            .unwrap();
        assert_eq!(
            principal_binding.owner_generation,
            replacement.claim.owner_generation
        );
        assert!(state
            .runtime_owner_registry
            .principal_binding_is_current(Some(principal_binding)));
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
        replacement.logical_browser_id = "session:new-replacement".to_string();
        replacement.daemon_session_route = "new-replacement".to_string();
        replacement.process_identity.pid = 4201;
        replacement.process_identity.start_token = "linux:boot:4201".to_string();
        replacement.process_group_id = Some(4201);
        replacement.cdp_endpoint = "ws://127.0.0.1:9556/devtools/browser/new".to_string();
        replacement.target_ids = vec!["target-new".to_string()];
        let replacement_binding = authority.register_managed_lane(replacement).unwrap();

        assert_eq!(replacement_binding.claim.owner_generation, 2);
        assert_eq!(
            replacement_binding.claim.logical_browser_id,
            "session:new-replacement"
        );
        let state = repository.load_snapshot().unwrap();
        assert!(!state
            .runtime_owner_registry
            .lifecycle_records()
            .contains_key("session:replacement"));
        let lifecycle =
            &state.runtime_owner_registry.lifecycle_records()["session:new-replacement"];
        assert_eq!(lifecycle.logical_browser_id, "session:new-replacement");
        assert_eq!(lifecycle.lifecycle_state, RuntimeLaneLifecycleState::Ready);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        assert!(lifecycle.terminal_evidence.is_empty());
        assert_eq!(
            lifecycle.package_launch_identity_digest,
            Some(
                package_launch_identity_digest(
                    state
                        .runtime_owner_registry
                        .owner(&replacement_binding.claim.profile_identity_digest)
                        .unwrap(),
                    Some(4201),
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn terminal_canonical_route_can_migrate_profile_identity() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let route = "rdp-guac-route-a-viewer";
        let browser_id = format!("session:{route}");
        let legacy_profile = std::env::temp_dir().join("agent-browser-route-a-legacy-profile");
        let stable_profile = std::env::temp_dir().join("agent-browser-route-a-stable-profile");
        let registration = ManagedLaneRegistration {
            logical_browser_id: browser_id.clone(),
            profile_root: legacy_profile,
            daemon_session_route: route.to_string(),
            process_group_id: Some(4250),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4250,
                start_token: "linux:boot:4250".to_string(),
                executable_path: Some("/opt/agent-browser/chromium".to_string()),
                browser_family: Some("chromium".to_string()),
            },
            browser_family: "chromium".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9570/devtools/browser/legacy".to_string(),
            target_ids: vec!["target-legacy".to_string()],
        };
        let legacy_binding = authority
            .register_managed_lane(registration.clone())
            .unwrap();
        repository
            .mutate(|state| {
                state.profiles.insert(
                    route.to_string(),
                    crate::native::service_model::BrowserProfile {
                        id: route.to_string(),
                        user_data_dir: Some(
                            registration.profile_root.to_string_lossy().into_owned(),
                        ),
                        ..crate::native::service_model::BrowserProfile::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::BeginClose {
                claim: legacy_binding.claim.clone(),
            })
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::CompleteClose {
                logical_browser_id: browser_id.clone(),
                profile_identity_digest: legacy_binding.claim.profile_identity_digest.clone(),
                expected_owner_generation: legacy_binding.claim.owner_generation,
                terminal_evidence: vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
            })
            .unwrap();

        let mut replacement = registration;
        replacement.profile_root = stable_profile.clone();
        replacement.process_group_id = Some(4251);
        replacement.process_identity.pid = 4251;
        replacement.process_identity.start_token = "linux:boot:4251".to_string();
        replacement.cdp_endpoint = "ws://127.0.0.1:9571/devtools/browser/stable".to_string();
        replacement.target_ids = vec!["target-stable".to_string()];
        let replacement_binding = authority.register_managed_lane(replacement).unwrap();

        let stable_digest =
            agent_browser_lease_authority::canonical_profile_identity_digest(&stable_profile)
                .unwrap();
        assert_eq!(
            replacement_binding.claim.owner_generation,
            legacy_binding.claim.owner_generation + 1
        );
        assert_eq!(
            replacement_binding.claim.profile_identity_digest,
            stable_digest
        );
        let state = repository.load_snapshot().unwrap();
        assert_eq!(
            state.profiles[route].user_data_dir.as_deref(),
            stable_profile.to_str()
        );
        assert!(state
            .runtime_owner_registry
            .owner(&legacy_binding.claim.profile_identity_digest)
            .is_none());
        assert!(state
            .runtime_owner_registry
            .owner(&replacement_binding.claim.profile_identity_digest)
            .is_some());
        let lifecycle = &state.runtime_owner_registry.lifecycle_records()[&browser_id];
        assert_eq!(
            lifecycle.profile_identity_digest,
            replacement_binding.claim.profile_identity_digest
        );
        assert_eq!(lifecycle.owner_generation, 2);
        assert_eq!(lifecycle.lifecycle_state, RuntimeLaneLifecycleState::Ready);
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Owned
        );
        assert!(lifecycle.terminal_evidence.is_empty());
    }

    #[test]
    fn terminal_profile_migration_rejects_noncanonical_or_incomplete_evidence() {
        for (route, terminal_evidence, principal_bound) in [
            (
                "ordinary-route",
                vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
                false,
            ),
            (
                "rdp-guac-route-b-viewer",
                vec!["exact_process_exited".to_string()],
                false,
            ),
            (
                "rdp-guac-route-c-viewer",
                vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
                true,
            ),
        ] {
            let browser_id = format!("session:{route}");
            let mut current = owner();
            current.profile_identity_digest = digest(&format!("legacy:{route}"));
            current.browser_id = browser_id.clone();
            current.daemon_session_route = route.to_string();
            let expected_owner = OwnerAuthorityClaim::from_owner(&current);
            let mut registry = RuntimeOwnerRegistry::from_owner(current.clone());
            crate::runtime_owner_transfer::edit_registry_fixture(&mut registry)
                .lifecycle_rows
                .insert(
                    browser_id.clone(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: browser_id.clone(),
                        profile_identity_digest: current.profile_identity_digest.clone(),
                        owner_generation: current.owner_generation,
                        lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                        cleanup_obligation_state: CleanupObligationState::Satisfied,
                        terminal_evidence,
                        ..RuntimeLifecycleRecord::default()
                    },
                );
            if principal_bound {
                crate::runtime_owner_transfer::edit_registry_fixture(&mut registry).principal_records.insert(
                    current.profile_identity_digest.clone(),
                    crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                        principal_id: "principal:protected".to_string(),
                        profile_id: route.to_string(),
                        profile_identity_digest: current.profile_identity_digest.clone(),
                        capability_id: "profile-capability-v1:protected".to_string(),
                        provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                        owner_generation: current.owner_generation,
                    },
                );
            }
            let mut replacement = current.clone();
            replacement.owner_id = format!("owner-replacement:{route}");
            replacement.profile_identity_digest = digest(&format!("stable:{route}"));
            replacement.owner_generation += 1;

            assert_eq!(
                apply_transition(
                    &mut registry,
                    RuntimeLifecycleIntent::MigrateTerminalProfileReplacement {
                        expected_owner,
                        owner: replacement,
                        process_group_id: Some(4300),
                        package_launch_identity_digest: digest("replacement-launch"),
                    },
                )
                .unwrap_err(),
                "runtime_lifecycle_terminal_profile_migration_rejected"
            );
            assert_eq!(
                registry
                    .owner(&current.profile_identity_digest)
                    .unwrap()
                    .owner_generation,
                current.owner_generation
            );
        }
    }

    #[test]
    fn terminal_replacement_rejects_a_colliding_logical_browser_record() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let profile_root = std::env::temp_dir().join("agent-browser-lifecycle-collision");
        let registration = ManagedLaneRegistration {
            logical_browser_id: "session:old".to_string(),
            profile_root,
            daemon_session_route: "old".to_string(),
            process_group_id: Some(4300),
            process_identity: crate::process_identity::RecordedProcessIdentity {
                pid: 4300,
                start_token: "linux:boot:4300".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            browser_family: "chrome".to_string(),
            cdp_endpoint: "ws://127.0.0.1:9560/devtools/browser/old".to_string(),
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
                terminal_evidence: vec!["profile_lock_released".to_string()],
            })
            .unwrap();
        repository
            .mutate(|state| {
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .insert(
                    "session:occupied".to_string(),
                    RuntimeLifecycleRecord {
                        logical_browser_id: "session:occupied".to_string(),
                        profile_identity_digest: digest("other-profile"),
                        owner_generation: 9,
                        lifecycle_state: RuntimeLaneLifecycleState::Ready,
                        cleanup_obligation_state: CleanupObligationState::Owned,
                        ..RuntimeLifecycleRecord::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let mut replacement = registration;
        replacement.logical_browser_id = "session:occupied".to_string();
        replacement.daemon_session_route = "occupied".to_string();
        replacement.process_group_id = Some(4301);
        replacement.process_identity.pid = 4301;
        replacement.process_identity.start_token = "linux:boot:4301".to_string();
        replacement.cdp_endpoint = "ws://127.0.0.1:9561/devtools/browser/new".to_string();
        replacement.target_ids = vec!["target-new".to_string()];

        assert_eq!(
            authority.register_managed_lane(replacement).unwrap_err(),
            "runtime_lifecycle_terminal_replacement_rejected"
        );
        let state = repository.load_snapshot().unwrap();
        assert_eq!(
            state
                .runtime_owner_registry
                .owner(&binding.claim.profile_identity_digest)
                .unwrap()
                .browser_id,
            "session:old"
        );
        assert_eq!(
            state.runtime_owner_registry.lifecycle_records()["session:old"].lifecycle_state,
            RuntimeLaneLifecycleState::Terminal
        );
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
            transferring.runtime_owner_registry.lifecycle_records()["browser-a"]
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
            aborted.runtime_owner_registry.lifecycle_records()["browser-a"]
                .cleanup_obligation_state,
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
        let lifecycle = &committed.runtime_owner_registry.lifecycle_records()["browser-a"];
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
        let lifecycle = &reversed.runtime_owner_registry.lifecycle_records()["browser-a"];
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
    fn terminal_profile_sync_cli_preflight_precedes_aggregate_mutation() {
        let repository = MemoryRepository::default();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let profile_root = std::env::temp_dir().join("agent-browser-lifecycle-preflight");

        repository
            .mutate(|state| {
                state.profiles.insert(
                    "ordinary-route".to_string(),
                    crate::native::service_model::BrowserProfile {
                        id: "ordinary-route".to_string(),
                        ..crate::native::service_model::BrowserProfile::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        let before = repository.load_snapshot().unwrap();

        assert_eq!(
            authority
                .transition_terminal_replacement_with_profile_sync(
                    RuntimeLifecycleIntent::RegisterCurrentOwner(owner()),
                    "ordinary-route",
                    &profile_root,
                )
                .unwrap_err(),
            "runtime_lifecycle_profile_record_sync_rejected"
        );
        assert_eq!(repository.load_snapshot().unwrap(), before);

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;

            let invalid_profile_root =
                std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![0xff]));
            assert_eq!(
                authority
                    .transition_terminal_replacement_with_profile_sync(
                        RuntimeLifecycleIntent::RegisterCurrentOwner(owner()),
                        "rdp-guac-missing-viewer",
                        &invalid_profile_root,
                    )
                    .unwrap_err(),
                "runtime_lifecycle_profile_path_invalid"
            );
            assert_eq!(repository.load_snapshot().unwrap(), before);
        }
    }

    #[test]
    fn partial_kernel_failure_rolls_back_inside_the_aggregate() {
        let repository = MemoryRepository::default();
        let current_owner = owner();
        let profile_id = "rdp-guac-route-a-viewer";
        let original_profile_root = std::env::temp_dir().join("agent-browser-lifecycle-original");
        let replacement_profile_root =
            std::env::temp_dir().join("agent-browser-lifecycle-replacement");
        let record = RuntimeLifecycleRecord {
            logical_browser_id: current_owner.browser_id.clone(),
            profile_identity_digest: current_owner.profile_identity_digest.clone(),
            owner_generation: current_owner.owner_generation,
            terminal_evidence: vec!["retained-evidence".to_string()],
            ..RuntimeLifecycleRecord::default()
        };
        repository
            .mutate(|state| {
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .extend([
                    (current_owner.browser_id.clone(), record.clone()),
                    ("historical-browser".to_string(), record.clone()),
                ]);
                state.profiles.insert(
                    profile_id.to_string(),
                    crate::native::service_model::BrowserProfile {
                        id: profile_id.to_string(),
                        user_data_dir: Some(original_profile_root.display().to_string()),
                        ..crate::native::service_model::BrowserProfile::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        let before = repository.load_snapshot().unwrap();
        let intent = RuntimeLifecycleIntent::RegisterCurrentOwner(current_owner.clone());
        let mut raw_registry = before.runtime_owner_registry.clone();

        assert_eq!(
            apply_transition(&mut raw_registry, intent.clone()).unwrap_err(),
            "runtime_lifecycle_record_ambiguous"
        );
        assert!(raw_registry
            .owner(&current_owner.profile_identity_digest)
            .is_some());
        assert_ne!(
            raw_registry.revision(),
            before.runtime_owner_registry.revision()
        );

        assert_eq!(
            RuntimeLifecycleAuthority::new(&repository)
                .transition(intent.clone())
                .unwrap_err(),
            "runtime_lifecycle_record_ambiguous"
        );
        assert_eq!(repository.load_snapshot().unwrap(), before);

        assert_eq!(
            RuntimeLifecycleAuthority::new(&repository)
                .transition_terminal_replacement_with_profile_sync(
                    intent,
                    profile_id,
                    &replacement_profile_root,
                )
                .unwrap_err(),
            "runtime_lifecycle_record_ambiguous"
        );
        assert_eq!(repository.load_snapshot().unwrap(), before);
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
        let lifecycle = &closing.runtime_owner_registry.lifecycle_records()["browser-a"];
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
        let lifecycle = &terminal.runtime_owner_registry.lifecycle_records()["browser-a"];
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
        assert_eq!(state.runtime_owner_registry.lifecycle_records().len(), 1);
        let lifecycle = &state.runtime_owner_registry.lifecycle_records()["browser-a"];
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
    fn orphaned_owner_requires_explicit_recovery_close_before_terminal_replacement() {
        let repository = registered_repository();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        assert!(authority
            .transition(RuntimeLifecycleIntent::BeginRecoveryClose {
                claim: OwnerAuthorityClaim::from_owner(&owner()),
            })
            .is_err());

        let orphan = authority
            .revoke_legacy_owner(&digest("profile-a"), "browser-a", "session-a", "owner-a", 7)
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::BeginRecoveryClose {
                claim: OwnerAuthorityClaim::from_owner(&orphan),
            })
            .unwrap();
        authority
            .transition(RuntimeLifecycleIntent::CompleteClose {
                logical_browser_id: "browser-a".to_string(),
                profile_identity_digest: digest("profile-a"),
                expected_owner_generation: 8,
                terminal_evidence: vec![
                    "exact_process_exited".to_string(),
                    "profile_lock_released".to_string(),
                ],
            })
            .unwrap();

        let state = repository.load_snapshot().unwrap();
        let lifecycle = &state.runtime_owner_registry.lifecycle_records()["browser-a"];
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
    fn reversed_orphan_adoption_with_rebound_alias_authorizes_candidate_relinquish() {
        let repository = registered_repository();
        repository
            .mutate(|state| {
                let mut lifecycle = crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .remove("browser-a")
                .expect("registered owner must have lifecycle accountability");
                lifecycle.logical_browser_id = "session:historical-alias".to_string();
                crate::runtime_owner_transfer::edit_registry_fixture(
                    &mut state.runtime_owner_registry,
                )
                .lifecycle_rows
                .insert(lifecycle.logical_browser_id.clone(), lifecycle);
                Ok(())
            })
            .unwrap();
        let authority = RuntimeLifecycleAuthority::new(&repository);
        let orphan = authority
            .revoke_legacy_owner(&digest("profile-a"), "browser-a", "session-a", "owner-a", 7)
            .unwrap();
        let mut request = cooperative_request();
        request.mode = crate::runtime_adoption::BrowserAdoptionMode::OrphanAdoption;
        request.logical_browser_id = "session:canonical-browser".to_string();
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
        assert_eq!(
            candidate_claim.logical_browser_id,
            "session:canonical-browser"
        );

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
        assert!(restored
            .runtime_owner_registry
            .lifecycle_records()
            .contains_key(&owner.browser_id));
        assert!(!restored
            .runtime_owner_registry
            .lifecycle_records()
            .contains_key("session:historical-alias"));
    }
}
