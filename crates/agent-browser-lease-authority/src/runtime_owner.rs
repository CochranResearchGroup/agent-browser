//! Provider-neutral ownership transfer for live browser adoption.
//!
//! The registry model deliberately mirrors Plan 0111's canonical profile
//! owner vocabulary. A pending transfer is observation-only. Effect authority
//! changes only when the owner-generation compare-and-swap commits.

use crate::ServicePrincipalProvenance;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Provider-neutral adoption posture for runtime-owner transfers.
///
/// This preserves the established snake_case wire contract while keeping the
/// pure custody kernel independent from CLI runtime-adoption orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAdoptionMode {
    CooperativeTransfer,
    OrphanAdoption,
    ManualPreservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileOwnerState {
    Reserving,
    Ready,
    Releasing,
    Orphaned,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileOwner {
    pub owner_id: String,
    pub profile_identity_digest: String,
    pub state: ProfileOwnerState,
    pub owner_generation: u64,
    pub browser_id: String,
    pub daemon_session_route: String,
    pub process_instance_digest: String,
    pub browser_family: String,
    pub cdp_endpoint_identity_digest: String,
    pub target_set_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_transfer: Option<OwnerTransferProposal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_transition: Option<OwnerTransitionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerTransferRequest {
    pub mode: BrowserAdoptionMode,
    pub logical_browser_id: String,
    pub profile_identity_digest: String,
    pub expected_owner_id: Option<String>,
    pub expected_owner_generation: u64,
    pub candidate_owner_id: String,
    pub candidate_daemon_session_route: String,
    pub process_instance_digest: String,
    pub browser_family: String,
    pub cdp_endpoint_identity_digest: String,
    pub target_set_digest: String,
    pub selected_target_identity_digest: String,
    pub transfer_nonce_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerTransferProposal {
    pub request: OwnerTransferRequest,
    pub previous_owner_generation: u64,
    pub candidate_owner_generation: u64,
    pub candidate_effect_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateOwnerAttachment {
    pub candidate_owner_id: String,
    pub candidate_daemon_session_route: String,
    pub candidate_owner_generation: u64,
    pub logical_browser_id: String,
    pub profile_identity_digest: String,
    pub process_instance_digest: String,
    pub browser_family: String,
    pub cdp_endpoint_identity_digest: String,
    pub target_set_digest: String,
    pub selected_target_identity_digest: String,
    pub transfer_nonce_digest: String,
    pub effect_capable: bool,
}

impl CandidateOwnerAttachment {
    pub fn from_request(request: &OwnerTransferRequest, generation: u64) -> Self {
        Self {
            candidate_owner_id: request.candidate_owner_id.clone(),
            candidate_daemon_session_route: request.candidate_daemon_session_route.clone(),
            candidate_owner_generation: generation,
            logical_browser_id: request.logical_browser_id.clone(),
            profile_identity_digest: request.profile_identity_digest.clone(),
            process_instance_digest: request.process_instance_digest.clone(),
            browser_family: request.browser_family.clone(),
            cdp_endpoint_identity_digest: request.cdp_endpoint_identity_digest.clone(),
            target_set_digest: request.target_set_digest.clone(),
            selected_target_identity_digest: request.selected_target_identity_digest.clone(),
            transfer_nonce_digest: request.transfer_nonce_digest.clone(),
            effect_capable: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerAuthorityClaim {
    pub owner_id: String,
    pub profile_identity_digest: String,
    pub owner_generation: u64,
    pub logical_browser_id: String,
    pub daemon_session_route: String,
    pub process_instance_digest: String,
}

impl OwnerAuthorityClaim {
    pub fn from_owner(owner: &ProfileOwner) -> Self {
        Self {
            owner_id: owner.owner_id.clone(),
            profile_identity_digest: owner.profile_identity_digest.clone(),
            owner_generation: owner.owner_generation,
            logical_browser_id: owner.browser_id.clone(),
            daemon_session_route: owner.daemon_session_route.clone(),
            process_instance_digest: owner.process_instance_digest.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOwnerBinding {
    pub claim: OwnerAuthorityClaim,
    pub effect_capable: bool,
}

impl RuntimeOwnerBinding {
    pub fn observation_only(claim: OwnerAuthorityClaim) -> Self {
        Self {
            claim,
            effect_capable: false,
        }
    }

    pub fn effect_capable(claim: OwnerAuthorityClaim) -> Self {
        Self {
            claim,
            effect_capable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOwnerHandoffReceiptAttestation {
    pub receipt_id: String,
    pub receipt_sha256: String,
    pub transition_kind: OwnerTransferTransitionKind,
    pub owner_generation: u64,
    pub state: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOwnerAttestation {
    pub owner_id: String,
    pub owner_generation: u64,
    pub owner_state: ProfileOwnerState,
    pub logical_browser_id: String,
    pub daemon_session_route: String,
    pub process_instance_digest: String,
    pub effect_capable: bool,
    pub handoff_receipt: Option<RuntimeOwnerHandoffReceiptAttestation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerTransferTransitionKind {
    Commit,
    Reverse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerTransferReceipt {
    pub receipt_id: String,
    pub transition_kind: OwnerTransferTransitionKind,
    pub mode: BrowserAdoptionMode,
    pub logical_browser_id: String,
    pub profile_identity_digest: String,
    pub process_instance_digest: String,
    pub cdp_endpoint_identity_digest: String,
    pub target_set_digest: String,
    pub selected_target_identity_digest: String,
    pub previous_owner_id: String,
    pub candidate_owner_id: String,
    pub previous_owner_generation: u64,
    pub candidate_owner_generation: u64,
    pub transfer_nonce_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileOwnerRollbackSnapshot {
    owner_id: String,
    state: ProfileOwnerState,
    owner_generation: u64,
    browser_id: String,
    daemon_session_route: String,
    process_instance_digest: String,
    browser_family: String,
    cdp_endpoint_identity_digest: String,
    target_set_digest: String,
}

impl ProfileOwnerRollbackSnapshot {
    fn from_owner(owner: &ProfileOwner) -> Self {
        Self {
            owner_id: owner.owner_id.clone(),
            state: owner.state,
            owner_generation: owner.owner_generation,
            browser_id: owner.browser_id.clone(),
            daemon_session_route: owner.daemon_session_route.clone(),
            process_instance_digest: owner.process_instance_digest.clone(),
            browser_family: owner.browser_family.clone(),
            cdp_endpoint_identity_digest: owner.cdp_endpoint_identity_digest.clone(),
            target_set_digest: owner.target_set_digest.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerTransitionRecord {
    original_owner: ProfileOwnerRollbackSnapshot,
    commit_receipt: OwnerTransferReceipt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reverse_nonce_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reverse_receipt: Option<OwnerTransferReceipt>,
}

/// Durable lifecycle posture for one logical runtime lane.
///
/// `Unknown` is the conservative compatibility default for registries written
/// before lifecycle authority was recorded. Unknown lanes are never eligible
/// for unattended effects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLaneLifecycleState {
    #[default]
    Unknown,
    Planned,
    Launching,
    Ready,
    Retained,
    Transferring,
    Closing,
    Terminal,
    Quarantined,
}

/// Durable cleanup duty carried with a runtime lane owner generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupObligationState {
    #[default]
    Unknown,
    Owned,
    Transferring,
    Reclaimable,
    Reclaiming,
    Satisfied,
    Quarantined,
}

/// Backward-compatible lifecycle ledger entry attached to the existing owner
/// registry. The ledger is intentionally evidence-only in P117 Slice A; later
/// slices route lifecycle effects through this authority.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeLifecycleRecord {
    pub logical_browser_id: String,
    /// Host boot that authenticated process-group and launch observations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub profile_identity_digest: String,
    pub owner_generation: u64,
    pub lifecycle_state: RuntimeLaneLifecycleState,
    pub cleanup_obligation_state: CleanupObligationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_group_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_launch_identity_digest: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub terminal_evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeOwnerRegistry {
    revision: u64,
    owners: BTreeMap<String, ProfileOwner>,
    /// Authenticated service-principal authority for each profile owner.
    ///
    /// Missing bindings identify legacy observation-only owners. The map is
    /// keyed by canonical profile identity digest so it cannot drift from the
    /// existing owner authority into a second owner registry.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    principal_bindings: BTreeMap<String, RuntimeOwnerPrincipalBinding>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    lifecycle_records: BTreeMap<String, RuntimeLifecycleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOwnerPrincipalBinding {
    pub principal_id: String,
    pub profile_id: String,
    pub profile_identity_digest: String,
    pub capability_id: String,
    pub provenance: ServicePrincipalProvenance,
    pub owner_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseOwnerTransferRequest {
    pub profile_identity_digest: String,
    pub expected_candidate_owner_id: String,
    pub expected_candidate_owner_generation: u64,
    pub transfer_nonce_digest: String,
    pub reverse_nonce_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerTransferFailureCode {
    InvalidEvidence,
    OwnerMissing,
    OwnerCompareAndSwapMismatch,
    TransferAlreadyPending,
    UnsupportedOwnerState,
    CandidateEvidenceMismatch,
    GenerationExhausted,
    RollbackEvidenceMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerTransferError {
    pub code: OwnerTransferFailureCode,
    pub message: &'static str,
}

/// A registered capability rotation either mismatches prior custody or fails
/// the existing binding validation after removing the previous binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeOwnerPrincipalRotationError {
    RotationMismatch,
    BindingFailed(OwnerTransferError),
}

/// The complete set of lifecycle intentions accepted by the concrete runtime
/// lifecycle kernel. Host observations and route policy are supplied by callers.
#[derive(Debug, Clone)]
pub enum RuntimeLifecycleIntent {
    RegisterCurrentOwner(ProfileOwner),
    RegisterManagedLane {
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
        boot_epoch: Option<String>,
    },
    ActivateTerminalReplacement {
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
        boot_epoch: Option<String>,
    },
    MigrateTerminalProfileReplacement {
        /// CLI-observed canonical route and logical browser naming policy.
        canonical_route_viewer_profile: bool,
        expected_owner: OwnerAuthorityClaim,
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
        boot_epoch: Option<String>,
    },
    SupersedeObservedOwner {
        expected_owner: OwnerAuthorityClaim,
        owner: ProfileOwner,
        process_group_id: Option<u32>,
        package_launch_identity_digest: String,
        boot_epoch: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLifecycleTransition {
    OwnerRegistered(ProfileOwner),
    TerminalReplacementActivated(ProfileOwner),
    ObservedOwnerSuperseded(ProfileOwner),
    OwnerEvidenceRefreshed(ProfileOwner),
    TransferPrepared(OwnerTransferProposal),
    CandidateCommitted(OwnerTransferReceipt),
    TransferAborted(bool),
    TransferReversed(OwnerTransferReceipt),
    LegacyOwnerRevoked(ProfileOwner),
    LaneUpdated(RuntimeLifecycleRecord),
}

impl RuntimeOwnerRegistry {
    /// Current monotonic registry revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Current owners indexed by canonical profile identity.
    pub fn owners(&self) -> &BTreeMap<String, ProfileOwner> {
        &self.owners
    }

    /// Current authenticated principal bindings.
    pub fn principal_bindings(&self) -> &BTreeMap<String, RuntimeOwnerPrincipalBinding> {
        &self.principal_bindings
    }

    /// Lifecycle evidence indexed by logical browser identity.
    pub fn lifecycle_records(&self) -> &BTreeMap<String, RuntimeLifecycleRecord> {
        &self.lifecycle_records
    }

    /// Restore a selected durable lifecycle sidecar without changing authority
    /// revision. Sidecar selection and precedence belong to the repository.
    pub fn restore_lifecycle_records(&mut self, records: BTreeMap<String, RuntimeLifecycleRecord>) {
        self.lifecycle_records = records;
    }

    /// Project the legacy owner payload while lifecycle evidence is persisted
    /// separately. This neither mutates the source nor advances its revision.
    pub fn persistence_projection_without_lifecycle_records(&self) -> Self {
        let mut registry = self.clone();
        registry.lifecycle_records.clear();
        registry
    }

    /// Replace registered capability custody in the established mutation order.
    /// Removing an old binding advances revision even when no ready owner can
    /// receive the replacement; a successful new binding advances it again.
    pub fn rotate_registered_principal_authority(
        &mut self,
        profile_identity_digest: &str,
        previous_capability_id: &str,
        principal_id: &str,
        profile_id: &str,
        capability_id: &str,
    ) -> Result<bool, RuntimeOwnerPrincipalRotationError> {
        if let Some(existing) = self
            .principal_bindings
            .get(profile_identity_digest)
            .cloned()
        {
            if existing.principal_id != principal_id
                || existing.profile_id != profile_id
                || existing.capability_id != previous_capability_id
                || existing.provenance != ServicePrincipalProvenance::RegisteredCapability
            {
                return Err(RuntimeOwnerPrincipalRotationError::RotationMismatch);
            }
            self.principal_bindings.remove(profile_identity_digest);
            self.revision = self.revision.saturating_add(1);
        }
        let Some(owner) = self.owners.get(profile_identity_digest).cloned() else {
            return Ok(false);
        };
        if owner.state != ProfileOwnerState::Ready {
            return Ok(false);
        }
        self.bind_principal_authority(RuntimeOwnerPrincipalBinding {
            principal_id: principal_id.to_string(),
            profile_id: profile_id.to_string(),
            profile_identity_digest: profile_identity_digest.to_string(),
            capability_id: capability_id.to_string(),
            provenance: ServicePrincipalProvenance::RegisteredCapability,
            owner_generation: owner.owner_generation,
        })
        .map_err(RuntimeOwnerPrincipalRotationError::BindingFailed)?;
        Ok(true)
    }

    /// Terminalize the exact runtime reset target after the CLI has validated
    /// both records and completed its filesystem and Service State effects.
    /// Each application advances revision once and retains prior evidence.
    pub fn apply_runtime_reset_terminalization(
        &mut self,
        profile_identity_digest: &str,
        browser_id: &str,
    ) {
        let owner = self
            .owners
            .get_mut(profile_identity_digest)
            .expect("runtime reset owner was validated before effects");
        owner.state = ProfileOwnerState::Orphaned;
        let lifecycle = self
            .lifecycle_records
            .get_mut(browser_id)
            .expect("runtime reset lifecycle was validated before effects");
        lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Terminal;
        lifecycle.cleanup_obligation_state = CleanupObligationState::Satisfied;
        if !lifecycle
            .terminal_evidence
            .iter()
            .any(|evidence| evidence == "profile_runtime_reset_applied")
        {
            lifecycle
                .terminal_evidence
                .push("profile_runtime_reset_applied".to_string());
        }
        self.revision = self.revision.saturating_add(1);
    }

    /// Release every current runtime owner after an explicit workstation cold
    /// shutdown has proven the owned processes absent. Profile definitions and
    /// lifecycle history remain available for the next clean startup, while no
    /// retained owner or principal binding can authorize effects.
    ///
    /// Replaying the operation against an already unowned registry is a no-op.
    pub fn release_all_for_cold_shutdown(&mut self) -> usize {
        let owners = self.owners.values().cloned().collect::<Vec<_>>();
        if owners.is_empty() && self.principal_bindings.is_empty() {
            return 0;
        }

        for owner in &owners {
            if let Some(lifecycle) = self.lifecycle_records.get_mut(&owner.browser_id) {
                lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Terminal;
                lifecycle.cleanup_obligation_state = CleanupObligationState::Satisfied;
                lifecycle.process_group_id = None;
                lifecycle.package_launch_identity_digest = None;
                if !lifecycle
                    .terminal_evidence
                    .iter()
                    .any(|evidence| evidence == "workstation_cold_shutdown")
                {
                    lifecycle
                        .terminal_evidence
                        .push("workstation_cold_shutdown".to_string());
                }
            }
        }
        let released = owners.len();
        self.owners.clear();
        self.principal_bindings.clear();
        self.revision = self.revision.saturating_add(1);
        released
    }

    /// Apply one pure lifecycle intention, preserving ordered partial mutations.
    /// Repository callers own transactional rollback by applying to a cloned registry.
    pub fn apply_lifecycle_transition(
        &mut self,
        intent: RuntimeLifecycleIntent,
    ) -> Result<RuntimeLifecycleTransition, String> {
        let registry = self;
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
                boot_epoch,
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
                lifecycle.boot_epoch = boot_epoch;
                store_lifecycle(registry, lifecycle)?;
                Ok(RuntimeLifecycleTransition::OwnerRegistered(owner))
            }
            RuntimeLifecycleIntent::ActivateTerminalReplacement {
                owner,
                process_group_id,
                package_launch_identity_digest,
                boot_epoch,
            } => {
                validate_profile_owner(&owner).map_err(owner_error)?;
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
                    || current.pending_transfer.is_some()
                    || lifecycle.profile_identity_digest != owner.profile_identity_digest
                    || lifecycle.owner_generation != current.owner_generation
                    || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Terminal
                    || lifecycle.cleanup_obligation_state != CleanupObligationState::Satisfied
                    || registry
                        .principal_bindings
                        .get(&owner.profile_identity_digest)
                        .is_some_and(|binding| binding.owner_generation > current.owner_generation)
                    || registry
                        .lifecycle_records
                        .iter()
                        .any(|(logical_id, record)| {
                            logical_id != &current.browser_id
                                && (logical_id == &owner.browser_id
                                    || record.profile_identity_digest
                                        == owner.profile_identity_digest)
                        })
                {
                    return Err("runtime_lifecycle_terminal_replacement_rejected".to_string());
                }
                registry
                    .owners
                    .insert(owner.profile_identity_digest.clone(), owner.clone());
                if let Some(binding) = registry
                    .principal_bindings
                    .get_mut(&owner.profile_identity_digest)
                {
                    binding.owner_generation = owner.owner_generation;
                }
                registry.revision = registry.revision.saturating_add(1);
                let mut lifecycle = lifecycle;
                if owner.browser_id != current.browser_id {
                    registry.lifecycle_records.remove(&current.browser_id);
                }
                lifecycle.logical_browser_id = owner.browser_id.clone();
                lifecycle.owner_generation = owner.owner_generation;
                lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
                lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
                lifecycle.process_group_id = process_group_id;
                lifecycle.package_launch_identity_digest = Some(package_launch_identity_digest);
                lifecycle.boot_epoch = boot_epoch;
                lifecycle.terminal_evidence.clear();
                store_lifecycle(registry, lifecycle)?;
                Ok(RuntimeLifecycleTransition::TerminalReplacementActivated(
                    owner,
                ))
            }
            RuntimeLifecycleIntent::MigrateTerminalProfileReplacement {
                canonical_route_viewer_profile,
                expected_owner,
                owner,
                process_group_id,
                package_launch_identity_digest,
                boot_epoch,
            } => {
                validate_profile_owner(&owner).map_err(owner_error)?;
                let lifecycle = registry
                    .lifecycle_records
                    .get(&expected_owner.logical_browser_id)
                    .cloned()
                    .ok_or_else(|| {
                        "runtime_lifecycle_profile_migration_record_missing".to_string()
                    })?;
                if !is_digest(&package_launch_identity_digest)
                    || !registry.authorizes(&expected_owner)
                    || owner.profile_identity_digest == expected_owner.profile_identity_digest
                    || owner.owner_generation != expected_owner.owner_generation.saturating_add(1)
                    || owner.browser_id != expected_owner.logical_browser_id
                    || owner.daemon_session_route != expected_owner.daemon_session_route
                    || !canonical_route_viewer_profile
                    || registry.owner(&owner.profile_identity_digest).is_some()
                    || lifecycle.profile_identity_digest != expected_owner.profile_identity_digest
                    || lifecycle.owner_generation != expected_owner.owner_generation
                    || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Terminal
                    || lifecycle.cleanup_obligation_state != CleanupObligationState::Satisfied
                    || !terminal_cleanup_evidence_complete(&lifecycle)
                    || registry
                        .principal_bindings
                        .contains_key(&expected_owner.profile_identity_digest)
                    || registry
                        .principal_bindings
                        .contains_key(&owner.profile_identity_digest)
                    || registry
                        .lifecycle_records
                        .iter()
                        .any(|(logical_id, record)| {
                            logical_id != &expected_owner.logical_browser_id
                                && (logical_id == &owner.browser_id
                                    || record.profile_identity_digest
                                        == owner.profile_identity_digest)
                        })
                {
                    return Err("runtime_lifecycle_terminal_profile_migration_rejected".to_string());
                }
                registry
                    .owners
                    .remove(&expected_owner.profile_identity_digest);
                registry
                    .owners
                    .insert(owner.profile_identity_digest.clone(), owner.clone());
                registry.revision = registry.revision.saturating_add(1);
                let mut lifecycle = lifecycle;
                lifecycle.profile_identity_digest = owner.profile_identity_digest.clone();
                lifecycle.owner_generation = owner.owner_generation;
                lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Ready;
                lifecycle.cleanup_obligation_state = CleanupObligationState::Owned;
                lifecycle.process_group_id = process_group_id;
                lifecycle.package_launch_identity_digest = Some(package_launch_identity_digest);
                lifecycle.boot_epoch = boot_epoch;
                lifecycle.terminal_evidence.clear();
                store_lifecycle(registry, lifecycle)?;
                Ok(RuntimeLifecycleTransition::TerminalReplacementActivated(
                    owner,
                ))
            }
            RuntimeLifecycleIntent::SupersedeObservedOwner {
                expected_owner,
                owner,
                process_group_id,
                package_launch_identity_digest,
                boot_epoch,
            } => {
                validate_profile_owner(&owner).map_err(owner_error)?;
                if !is_digest(&package_launch_identity_digest)
                    || !registry.authorizes(&expected_owner)
                    || owner.profile_identity_digest != expected_owner.profile_identity_digest
                    || owner.owner_generation != expected_owner.owner_generation.saturating_add(1)
                    || registry
                        .principal_bindings
                        .get(&owner.profile_identity_digest)
                        .is_some_and(|binding| {
                            binding.owner_generation > expected_owner.owner_generation
                        })
                {
                    return Err("runtime_lifecycle_observed_supersession_rejected".to_string());
                }
                registry.lifecycle_records.retain(|logical_id, record| {
                    logical_id == &owner.browser_id
                        || record.profile_identity_digest != owner.profile_identity_digest
                });
                registry
                    .owners
                    .insert(owner.profile_identity_digest.clone(), owner.clone());
                if let Some(binding) = registry
                    .principal_bindings
                    .get_mut(&owner.profile_identity_digest)
                {
                    binding.owner_generation = owner.owner_generation;
                }
                registry.revision = registry.revision.saturating_add(1);
                let lifecycle = RuntimeLifecycleRecord {
                    logical_browser_id: owner.browser_id.clone(),
                    boot_epoch,
                    profile_identity_digest: owner.profile_identity_digest.clone(),
                    owner_generation: owner.owner_generation,
                    lifecycle_state: RuntimeLaneLifecycleState::Ready,
                    cleanup_obligation_state: CleanupObligationState::Owned,
                    process_group_id,
                    package_launch_identity_digest: Some(package_launch_identity_digest),
                    terminal_evidence: Vec::new(),
                };
                store_lifecycle(registry, lifecycle)?;
                Ok(RuntimeLifecycleTransition::ObservedOwnerSuperseded(owner))
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
                    ProfileOwnerState::Ready => RuntimeLaneLifecycleState::Ready,
                    ProfileOwnerState::Orphaned => RuntimeLaneLifecycleState::Retained,
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
            RuntimeLifecycleIntent::BeginRecoveryClose { claim } => {
                let owner = registry
                    .owner(&claim.profile_identity_digest)
                    .cloned()
                    .ok_or_else(|| "runtime_lifecycle_recovery_owner_missing".to_string())?;
                if owner.state != ProfileOwnerState::Orphaned
                    || OwnerAuthorityClaim::from_owner(&owner) != claim
                {
                    return Err("runtime_lifecycle_recovery_owner_mismatch".to_string());
                }
                let lifecycle = registry
                    .lifecycle_records
                    .get_mut(&claim.logical_browser_id)
                    .ok_or_else(|| "runtime_lifecycle_recovery_record_missing".to_string())?;
                if lifecycle.profile_identity_digest != claim.profile_identity_digest
                    || lifecycle.owner_generation != claim.owner_generation
                    || lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Retained
                    || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
                {
                    return Err("runtime_lifecycle_recovery_compare_and_swap_mismatch".to_string());
                }
                lifecycle.lifecycle_state = RuntimeLaneLifecycleState::Closing;
                let lifecycle = lifecycle.clone();
                registry.revision = registry.revision.saturating_add(1);
                Ok(RuntimeLifecycleTransition::LaneUpdated(lifecycle))
            }
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

    pub fn is_empty(&self) -> bool {
        self.owners.is_empty() && self.principal_bindings.is_empty()
    }

    pub fn from_owner(owner: ProfileOwner) -> Self {
        Self {
            revision: 1,
            owners: BTreeMap::from([(owner.profile_identity_digest.clone(), owner)]),
            principal_bindings: BTreeMap::new(),
            lifecycle_records: BTreeMap::new(),
        }
    }

    pub fn bind_principal_authority(
        &mut self,
        binding: RuntimeOwnerPrincipalBinding,
    ) -> Result<RuntimeOwnerPrincipalBinding, OwnerTransferError> {
        if binding.principal_id.trim().is_empty()
            || binding.profile_id.trim().is_empty()
            || binding.capability_id.trim().is_empty()
            || binding.owner_generation == 0
            || binding.provenance == ServicePrincipalProvenance::UnprovenLegacy
            || !is_sha256(&binding.profile_identity_digest)
        {
            return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
        }
        let owner = self
            .owners
            .get(&binding.profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        if owner.state != ProfileOwnerState::Ready
            || owner.owner_generation != binding.owner_generation
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        if let Some(existing) = self
            .principal_bindings
            .get(&binding.profile_identity_digest)
        {
            if existing == &binding {
                return Ok(existing.clone());
            }
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        self.principal_bindings
            .insert(binding.profile_identity_digest.clone(), binding.clone());
        self.revision = self.revision.saturating_add(1);
        Ok(binding)
    }

    pub fn refresh_principal_authority(
        &mut self,
        binding: RuntimeOwnerPrincipalBinding,
    ) -> Result<RuntimeOwnerPrincipalBinding, OwnerTransferError> {
        if binding.principal_id.trim().is_empty()
            || binding.profile_id.trim().is_empty()
            || binding.capability_id.trim().is_empty()
            || binding.owner_generation == 0
            || binding.provenance == ServicePrincipalProvenance::UnprovenLegacy
            || !is_sha256(&binding.profile_identity_digest)
        {
            return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
        }
        let owner = self
            .owners
            .get(&binding.profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        if owner.state != ProfileOwnerState::Ready
            || owner.owner_generation != binding.owner_generation
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        let existing = self
            .principal_bindings
            .get(&binding.profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        let same_capability_authority = existing.principal_id == binding.principal_id
            && existing.profile_id == binding.profile_id
            && existing.profile_identity_digest == binding.profile_identity_digest
            && existing.capability_id == binding.capability_id
            && existing.provenance == binding.provenance;
        if !same_capability_authority || existing.owner_generation > binding.owner_generation {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        if existing == &binding {
            return Ok(existing.clone());
        }
        self.principal_bindings
            .insert(binding.profile_identity_digest.clone(), binding.clone());
        self.revision = self.revision.saturating_add(1);
        Ok(binding)
    }

    pub fn principal_binding_is_current(
        &self,
        binding: Option<&RuntimeOwnerPrincipalBinding>,
    ) -> bool {
        binding.is_some_and(|binding| {
            self.principal_bindings
                .get(&binding.profile_identity_digest)
                .is_some_and(|current| current == binding)
                && self
                    .owners
                    .get(&binding.profile_identity_digest)
                    .is_some_and(|owner| {
                        owner.state == ProfileOwnerState::Ready
                            && owner.owner_generation == binding.owner_generation
                    })
        })
    }

    pub fn owner(&self, profile_identity_digest: &str) -> Option<&ProfileOwner> {
        self.owners.get(profile_identity_digest)
    }

    pub fn register_current_owner(
        &mut self,
        owner: ProfileOwner,
    ) -> Result<ProfileOwner, OwnerTransferError> {
        validate_profile_owner(&owner)?;
        if let Some(existing) = self.owners.get(&owner.profile_identity_digest) {
            if existing == &owner {
                return Ok(existing.clone());
            }
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        self.owners
            .insert(owner.profile_identity_digest.clone(), owner.clone());
        self.revision = self.revision.saturating_add(1);
        Ok(owner)
    }

    pub fn authorizes(&self, claim: &OwnerAuthorityClaim) -> bool {
        self.owners
            .get(&claim.profile_identity_digest)
            .is_some_and(|owner| {
                owner.state == ProfileOwnerState::Ready
                    && owner.owner_id == claim.owner_id
                    && owner.owner_generation == claim.owner_generation
                    && owner.browser_id == claim.logical_browser_id
                    && owner.daemon_session_route == claim.daemon_session_route
                    && owner.process_instance_digest == claim.process_instance_digest
            })
    }

    pub fn revoke_legacy_daemon_owner(
        &mut self,
        profile_identity_digest: &str,
        logical_browser_id: &str,
        expected_daemon_session_route: &str,
        expected_owner_id: &str,
        expected_owner_generation: u64,
    ) -> Result<ProfileOwner, OwnerTransferError> {
        if !is_sha256(profile_identity_digest)
            || logical_browser_id.trim().is_empty()
            || expected_daemon_session_route.trim().is_empty()
            || expected_owner_id.trim().is_empty()
            || expected_owner_generation == 0
        {
            return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
        }
        let owner = self
            .owners
            .get_mut(profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        if owner.state != ProfileOwnerState::Ready || owner.pending_transfer.is_some() {
            return Err(transfer_error(
                OwnerTransferFailureCode::UnsupportedOwnerState,
            ));
        }
        if owner.browser_id != logical_browser_id
            || owner.daemon_session_route != expected_daemon_session_route
            || owner.owner_id != expected_owner_id
            || owner.owner_generation != expected_owner_generation
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        owner.owner_generation = owner
            .owner_generation
            .checked_add(1)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::GenerationExhausted))?;
        owner.state = ProfileOwnerState::Orphaned;
        owner.pending_transfer = None;
        self.revision = self.revision.saturating_add(1);
        Ok(owner.clone())
    }

    pub fn binding_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<RuntimeOwnerBinding>, String> {
        let logical_browser_id = format!("session:{session_id}");
        let matches = self
            .owners
            .values()
            .filter(|owner| {
                let previous_daemon_session_route = owner
                    .last_transition
                    .as_ref()
                    .map(|transition| transition.original_owner.daemon_session_route.as_str());
                owner.state == ProfileOwnerState::Ready
                    && (owner.daemon_session_route == session_id
                        || owner.browser_id == logical_browser_id
                        || previous_daemon_session_route == Some(session_id))
            })
            .collect::<Vec<_>>();
        let current_matches = matches
            .iter()
            .copied()
            .filter(|owner| {
                !self
                    .lifecycle_records
                    .get(&owner.browser_id)
                    .is_some_and(|lifecycle| {
                        lifecycle.profile_identity_digest == owner.profile_identity_digest
                            && lifecycle.owner_generation == owner.owner_generation
                            && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
                            && lifecycle.cleanup_obligation_state
                                == CleanupObligationState::Satisfied
                    })
            })
            .collect::<Vec<_>>();
        // A sole terminal owner remains available to the guarded relaunch path.
        // Once a replacement exists, completed history must not make that
        // current session binding ambiguous.
        let selected_matches = if current_matches.is_empty() {
            &matches
        } else {
            &current_matches
        };
        if selected_matches.len() > 1 {
            return Err(format!(
                "runtime_owner_session_ambiguous: session '{session_id}' matches multiple profile owners"
            ));
        }
        Ok(selected_matches.first().map(|owner| {
            let claim = OwnerAuthorityClaim::from_owner(owner);
            if owner.daemon_session_route == session_id {
                RuntimeOwnerBinding::effect_capable(claim)
            } else {
                RuntimeOwnerBinding::observation_only(claim)
            }
        }))
    }

    pub fn attestation_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<RuntimeOwnerAttestation>, String> {
        let Some(binding) = self.binding_for_session(session_id)? else {
            return Ok(None);
        };
        let owner = self
            .owners
            .get(&binding.claim.profile_identity_digest)
            .ok_or_else(|| "runtime_owner_binding_missing_owner".to_string())?;
        let receipt = owner.last_transition.as_ref().and_then(|transition| {
            transition
                .reverse_receipt
                .as_ref()
                .filter(|receipt| {
                    receipt.candidate_owner_id == owner.owner_id
                        && receipt.candidate_owner_generation == owner.owner_generation
                })
                .or_else(|| {
                    let receipt = &transition.commit_receipt;
                    (receipt.candidate_owner_id == owner.owner_id
                        && receipt.candidate_owner_generation == owner.owner_generation)
                        .then_some(receipt)
                })
        });
        let handoff_receipt = receipt
            .map(|receipt| {
                let serialized = serde_json::to_vec(receipt).map_err(|error| {
                    format!("could not serialize runtime owner handoff receipt: {error}")
                })?;
                Ok::<RuntimeOwnerHandoffReceiptAttestation, String>(
                    RuntimeOwnerHandoffReceiptAttestation {
                        receipt_id: receipt.receipt_id.clone(),
                        receipt_sha256: format!("{:x}", Sha256::digest(serialized)),
                        transition_kind: receipt.transition_kind,
                        owner_generation: receipt.candidate_owner_generation,
                        state: "accepted",
                    },
                )
            })
            .transpose()?;
        Ok(Some(RuntimeOwnerAttestation {
            owner_id: owner.owner_id.clone(),
            owner_generation: owner.owner_generation,
            owner_state: owner.state,
            logical_browser_id: owner.browser_id.clone(),
            daemon_session_route: owner.daemon_session_route.clone(),
            process_instance_digest: owner.process_instance_digest.clone(),
            effect_capable: binding.effect_capable,
            handoff_receipt,
        }))
    }

    pub fn refreshed_claim_after_reverse(
        &self,
        claim: &OwnerAuthorityClaim,
    ) -> Option<OwnerAuthorityClaim> {
        let owner = self.owners.get(&claim.profile_identity_digest)?;
        let reverse = owner.last_transition.as_ref()?.reverse_receipt.as_ref()?;
        (owner.state == ProfileOwnerState::Ready
            && owner.owner_id == claim.owner_id
            && owner.browser_id == claim.logical_browser_id
            && owner.daemon_session_route == claim.daemon_session_route
            && owner.process_instance_digest == claim.process_instance_digest
            && owner.owner_generation > claim.owner_generation
            && reverse.transition_kind == OwnerTransferTransitionKind::Reverse
            && reverse.candidate_owner_id == claim.owner_id
            && reverse.candidate_owner_generation == owner.owner_generation)
            .then(|| OwnerAuthorityClaim::from_owner(owner))
    }

    pub fn begin_transfer(
        &mut self,
        mut request: OwnerTransferRequest,
    ) -> Result<OwnerTransferProposal, OwnerTransferError> {
        validate_request(&request)?;
        if !self.owners.contains_key(&request.profile_identity_digest)
            && request.mode == BrowserAdoptionMode::OrphanAdoption
            && request.expected_owner_id.is_none()
            && request.expected_owner_generation == 0
        {
            let orphan_owner_id = format!(
                "orphan-observation-{}",
                &receipt_id(
                    OwnerTransferTransitionKind::Commit,
                    &request.process_instance_digest,
                    0
                )[15..31]
            );
            self.owners.insert(
                request.profile_identity_digest.clone(),
                ProfileOwner {
                    owner_id: orphan_owner_id.clone(),
                    profile_identity_digest: request.profile_identity_digest.clone(),
                    state: ProfileOwnerState::Orphaned,
                    owner_generation: 0,
                    browser_id: request.logical_browser_id.clone(),
                    daemon_session_route: "orphan-observation".to_string(),
                    process_instance_digest: request.process_instance_digest.clone(),
                    browser_family: request.browser_family.clone(),
                    cdp_endpoint_identity_digest: request.cdp_endpoint_identity_digest.clone(),
                    target_set_digest: request.target_set_digest.clone(),
                    pending_transfer: None,
                    last_transition: None,
                },
            );
            request.expected_owner_id = Some(orphan_owner_id);
        }
        let owner = self
            .owners
            .get_mut(&request.profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;

        if let Some(transition) = owner.last_transition.as_ref() {
            let receipt = &transition.commit_receipt;
            if receipt.transfer_nonce_digest == request.transfer_nonce_digest
                && owner.owner_id == request.candidate_owner_id
                && owner.owner_generation == receipt.candidate_owner_generation
            {
                return Ok(OwnerTransferProposal {
                    request,
                    previous_owner_generation: receipt.previous_owner_generation,
                    candidate_owner_generation: receipt.candidate_owner_generation,
                    candidate_effect_capable: false,
                });
            }
        }
        if let Some(pending) = owner.pending_transfer.as_ref() {
            if pending.request == request {
                return Ok(pending.clone());
            }
            return Err(transfer_error(
                OwnerTransferFailureCode::TransferAlreadyPending,
            ));
        }
        let expected_state = match request.mode {
            BrowserAdoptionMode::CooperativeTransfer => ProfileOwnerState::Ready,
            BrowserAdoptionMode::OrphanAdoption => ProfileOwnerState::Orphaned,
            BrowserAdoptionMode::ManualPreservation => {
                return Err(transfer_error(
                    OwnerTransferFailureCode::UnsupportedOwnerState,
                ));
            }
        };
        if owner.state != expected_state {
            return Err(transfer_error(
                OwnerTransferFailureCode::UnsupportedOwnerState,
            ));
        }
        let target_set_matches = request.mode == BrowserAdoptionMode::OrphanAdoption
            || request.target_set_digest == owner.target_set_digest;
        let logical_browser_matches = request.mode == BrowserAdoptionMode::OrphanAdoption
            || request.logical_browser_id == owner.browser_id;
        if request.expected_owner_id.as_deref() != Some(owner.owner_id.as_str())
            || request.expected_owner_generation != owner.owner_generation
            || !logical_browser_matches
            || request.process_instance_digest != owner.process_instance_digest
            || request.browser_family != owner.browser_family
            || request.cdp_endpoint_identity_digest != owner.cdp_endpoint_identity_digest
            || !target_set_matches
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        let candidate_owner_generation = owner
            .owner_generation
            .checked_add(1)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::GenerationExhausted))?;
        let proposal = OwnerTransferProposal {
            previous_owner_generation: owner.owner_generation,
            candidate_owner_generation,
            candidate_effect_capable: false,
            request,
        };
        owner.pending_transfer = Some(proposal.clone());
        self.revision = self.revision.saturating_add(1);
        Ok(proposal)
    }

    pub fn commit_candidate(
        &mut self,
        attachment: CandidateOwnerAttachment,
    ) -> Result<OwnerTransferReceipt, OwnerTransferError> {
        let owner = self
            .owners
            .get_mut(&attachment.profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        if let Some(transition) = owner.last_transition.as_ref() {
            let receipt = &transition.commit_receipt;
            if receipt.transfer_nonce_digest == attachment.transfer_nonce_digest
                && receipt.candidate_owner_id == attachment.candidate_owner_id
                && receipt.candidate_owner_generation == attachment.candidate_owner_generation
                && owner.owner_id == attachment.candidate_owner_id
                && owner.owner_generation == attachment.candidate_owner_generation
            {
                return Ok(receipt.clone());
            }
        }
        let proposal = owner
            .pending_transfer
            .clone()
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::CandidateEvidenceMismatch))?;
        if !attachment_matches_proposal(&attachment, &proposal) {
            return Err(transfer_error(
                OwnerTransferFailureCode::CandidateEvidenceMismatch,
            ));
        }
        if owner.owner_id
            != proposal
                .request
                .expected_owner_id
                .as_deref()
                .unwrap_or_default()
            || owner.owner_generation != proposal.previous_owner_generation
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        if self
            .principal_bindings
            .get(&attachment.profile_identity_digest)
            .is_some_and(|binding| binding.owner_generation > proposal.previous_owner_generation)
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }

        let original_owner = ProfileOwnerRollbackSnapshot::from_owner(owner);
        let receipt = commit_receipt(
            &proposal.request,
            proposal.previous_owner_generation,
            proposal.candidate_owner_generation,
        );
        owner.owner_id = proposal.request.candidate_owner_id.clone();
        owner.state = ProfileOwnerState::Ready;
        owner.owner_generation = proposal.candidate_owner_generation;
        owner.browser_id = proposal.request.logical_browser_id.clone();
        owner.daemon_session_route = proposal.request.candidate_daemon_session_route.clone();
        owner.process_instance_digest = proposal.request.process_instance_digest.clone();
        owner.browser_family = proposal.request.browser_family.clone();
        owner.cdp_endpoint_identity_digest = proposal.request.cdp_endpoint_identity_digest.clone();
        owner.target_set_digest = proposal.request.target_set_digest.clone();
        owner.pending_transfer = None;
        owner.last_transition = Some(OwnerTransitionRecord {
            original_owner,
            commit_receipt: receipt.clone(),
            reverse_nonce_digest: None,
            reverse_receipt: None,
        });
        if let Some(binding) = self
            .principal_bindings
            .get_mut(&attachment.profile_identity_digest)
        {
            binding.owner_generation = proposal.candidate_owner_generation;
        }
        self.revision = self.revision.saturating_add(1);
        Ok(receipt)
    }

    pub fn abort_pending_transfer(
        &mut self,
        profile_identity_digest: &str,
        expected_owner_id: &str,
        expected_owner_generation: u64,
        transfer_nonce_digest: &str,
    ) -> Result<bool, OwnerTransferError> {
        if !is_sha256(profile_identity_digest) || !is_sha256(transfer_nonce_digest) {
            return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
        }
        let owner = self
            .owners
            .get_mut(profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        let Some(proposal) = owner.pending_transfer.as_ref() else {
            return Ok(false);
        };
        if owner.owner_id != expected_owner_id
            || owner.owner_generation != expected_owner_generation
            || proposal.request.transfer_nonce_digest != transfer_nonce_digest
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        owner.pending_transfer = None;
        self.revision = self.revision.saturating_add(1);
        Ok(true)
    }

    pub fn reverse_transfer(
        &mut self,
        request: ReverseOwnerTransferRequest,
    ) -> Result<OwnerTransferReceipt, OwnerTransferError> {
        if !is_sha256(&request.profile_identity_digest)
            || !is_sha256(&request.transfer_nonce_digest)
            || !is_sha256(&request.reverse_nonce_digest)
        {
            return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
        }
        let owner = self
            .owners
            .get_mut(&request.profile_identity_digest)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::OwnerMissing))?;
        let transition = owner
            .last_transition
            .as_mut()
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::RollbackEvidenceMissing))?;
        if transition.reverse_nonce_digest.as_deref() == Some(&request.reverse_nonce_digest) {
            return transition
                .reverse_receipt
                .clone()
                .ok_or_else(|| transfer_error(OwnerTransferFailureCode::RollbackEvidenceMissing));
        }
        if owner.owner_id != request.expected_candidate_owner_id
            || owner.owner_generation != request.expected_candidate_owner_generation
            || transition.commit_receipt.transfer_nonce_digest != request.transfer_nonce_digest
        {
            return Err(transfer_error(
                OwnerTransferFailureCode::OwnerCompareAndSwapMismatch,
            ));
        }
        let reverse_generation = owner
            .owner_generation
            .checked_add(1)
            .ok_or_else(|| transfer_error(OwnerTransferFailureCode::GenerationExhausted))?;
        let original = transition.original_owner.clone();
        let mut receipt = transition.commit_receipt.clone();
        receipt.receipt_id = receipt_id(
            OwnerTransferTransitionKind::Reverse,
            &request.reverse_nonce_digest,
            reverse_generation,
        );
        receipt.transition_kind = OwnerTransferTransitionKind::Reverse;
        receipt.previous_owner_id = owner.owner_id.clone();
        receipt.candidate_owner_id = original.owner_id.clone();
        receipt.previous_owner_generation = owner.owner_generation;
        receipt.candidate_owner_generation = reverse_generation;
        receipt.transfer_nonce_digest = request.reverse_nonce_digest.clone();

        owner.owner_id = original.owner_id;
        owner.state = original.state;
        owner.owner_generation = reverse_generation;
        owner.browser_id = original.browser_id;
        owner.daemon_session_route = original.daemon_session_route;
        owner.process_instance_digest = original.process_instance_digest;
        owner.browser_family = original.browser_family;
        owner.cdp_endpoint_identity_digest = original.cdp_endpoint_identity_digest;
        owner.target_set_digest = original.target_set_digest;
        owner.pending_transfer = None;
        transition.reverse_nonce_digest = Some(request.reverse_nonce_digest);
        transition.reverse_receipt = Some(receipt.clone());
        if let Some(binding) = self
            .principal_bindings
            .get_mut(&request.profile_identity_digest)
        {
            binding.owner_generation = reverse_generation;
        }
        self.revision = self.revision.saturating_add(1);
        Ok(receipt)
    }
}

pub fn owner_binding_in_registry(
    registry: &RuntimeOwnerRegistry,
    session_id: &str,
) -> Result<Option<RuntimeOwnerBinding>, String> {
    let binding = registry.binding_for_session(session_id)?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    let is_terminal_history = registry
        .lifecycle_records
        .get(&binding.claim.logical_browser_id)
        .is_some_and(|lifecycle| {
            lifecycle.profile_identity_digest == binding.claim.profile_identity_digest
                && lifecycle.owner_generation == binding.claim.owner_generation
                && lifecycle.lifecycle_state == RuntimeLaneLifecycleState::Terminal
                && lifecycle.cleanup_obligation_state == CleanupObligationState::Satisfied
        });
    Ok((!is_terminal_history).then_some(binding))
}

/// Return whether an action must hydrate and validate runtime owner authority.
/// Read-only actions stay independent of the user-scoped owner registry so
fn attachment_matches_proposal(
    attachment: &CandidateOwnerAttachment,
    proposal: &OwnerTransferProposal,
) -> bool {
    let request = &proposal.request;
    !attachment.effect_capable
        && attachment.candidate_owner_id == request.candidate_owner_id
        && attachment.candidate_daemon_session_route == request.candidate_daemon_session_route
        && attachment.candidate_owner_generation == proposal.candidate_owner_generation
        && attachment.logical_browser_id == request.logical_browser_id
        && attachment.profile_identity_digest == request.profile_identity_digest
        && attachment.process_instance_digest == request.process_instance_digest
        && attachment.browser_family == request.browser_family
        && attachment.cdp_endpoint_identity_digest == request.cdp_endpoint_identity_digest
        && attachment.target_set_digest == request.target_set_digest
        && attachment.selected_target_identity_digest == request.selected_target_identity_digest
        && attachment.transfer_nonce_digest == request.transfer_nonce_digest
}

fn validate_request(request: &OwnerTransferRequest) -> Result<(), OwnerTransferError> {
    let opaque_ids = [
        request.logical_browser_id.as_str(),
        request.candidate_owner_id.as_str(),
        request.candidate_daemon_session_route.as_str(),
        request.browser_family.as_str(),
    ];
    let digests = [
        request.profile_identity_digest.as_str(),
        request.process_instance_digest.as_str(),
        request.cdp_endpoint_identity_digest.as_str(),
        request.target_set_digest.as_str(),
        request.selected_target_identity_digest.as_str(),
        request.transfer_nonce_digest.as_str(),
    ];
    if opaque_ids.iter().any(|value| value.trim().is_empty())
        || digests.iter().any(|value| !is_sha256(value))
        || request
            .expected_owner_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        || (request.expected_owner_id.is_none()
            && request.mode != BrowserAdoptionMode::OrphanAdoption)
    {
        return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
    }
    Ok(())
}

pub fn validate_profile_owner(owner: &ProfileOwner) -> Result<(), OwnerTransferError> {
    let opaque_ids = [
        owner.owner_id.as_str(),
        owner.browser_id.as_str(),
        owner.daemon_session_route.as_str(),
        owner.browser_family.as_str(),
    ];
    let digests = [
        owner.profile_identity_digest.as_str(),
        owner.process_instance_digest.as_str(),
        owner.cdp_endpoint_identity_digest.as_str(),
        owner.target_set_digest.as_str(),
    ];
    if owner.state != ProfileOwnerState::Ready
        || owner.owner_generation == 0
        || owner.pending_transfer.is_some()
        || opaque_ids.iter().any(|value| value.trim().is_empty())
        || digests.iter().any(|value| !is_sha256(value))
    {
        return Err(transfer_error(OwnerTransferFailureCode::InvalidEvidence));
    }
    Ok(())
}

fn commit_receipt(
    request: &OwnerTransferRequest,
    previous_owner_generation: u64,
    candidate_owner_generation: u64,
) -> OwnerTransferReceipt {
    OwnerTransferReceipt {
        receipt_id: receipt_id(
            OwnerTransferTransitionKind::Commit,
            &request.transfer_nonce_digest,
            candidate_owner_generation,
        ),
        transition_kind: OwnerTransferTransitionKind::Commit,
        mode: request.mode,
        logical_browser_id: request.logical_browser_id.clone(),
        profile_identity_digest: request.profile_identity_digest.clone(),
        process_instance_digest: request.process_instance_digest.clone(),
        cdp_endpoint_identity_digest: request.cdp_endpoint_identity_digest.clone(),
        target_set_digest: request.target_set_digest.clone(),
        selected_target_identity_digest: request.selected_target_identity_digest.clone(),
        previous_owner_id: request.expected_owner_id.clone().unwrap_or_default(),
        candidate_owner_id: request.candidate_owner_id.clone(),
        previous_owner_generation,
        candidate_owner_generation,
        transfer_nonce_digest: request.transfer_nonce_digest.clone(),
    }
}

fn receipt_id(
    transition_kind: OwnerTransferTransitionKind,
    nonce_digest: &str,
    generation: u64,
) -> String {
    let kind = match transition_kind {
        OwnerTransferTransitionKind::Commit => "commit",
        OwnerTransferTransitionKind::Reverse => "reverse",
    };
    format!(
        "owner-transfer-{:x}",
        Sha256::digest(format!("{kind}:{nonce_digest}:{generation}").as_bytes())
    )
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn transfer_error(code: OwnerTransferFailureCode) -> OwnerTransferError {
    let message = match code {
        OwnerTransferFailureCode::InvalidEvidence => "owner transfer evidence is invalid",
        OwnerTransferFailureCode::OwnerMissing => "profile owner is missing",
        OwnerTransferFailureCode::OwnerCompareAndSwapMismatch => {
            "profile owner changed before compare-and-swap"
        }
        OwnerTransferFailureCode::TransferAlreadyPending => {
            "a different owner transfer is already pending"
        }
        OwnerTransferFailureCode::UnsupportedOwnerState => {
            "profile owner state does not authorize this transfer mode"
        }
        OwnerTransferFailureCode::CandidateEvidenceMismatch => {
            "candidate attachment does not match the pending transfer"
        }
        OwnerTransferFailureCode::GenerationExhausted => "owner generation is exhausted",
        OwnerTransferFailureCode::RollbackEvidenceMissing => "reverse transfer evidence is missing",
    };
    OwnerTransferError { code, message }
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

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
        let mut lifecycle = if key == &owner.browser_id {
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
        lifecycle.logical_browser_id = owner.browser_id.clone();
        return Ok(lifecycle);
    }
    Ok(RuntimeLifecycleRecord {
        logical_browser_id: owner.browser_id.clone(),
        boot_epoch: None,
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

    fn digest(seed: &str) -> String {
        format!("{:x}", Sha256::digest(seed.as_bytes()))
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
            selected_target_identity_digest: digest("target"),
            transfer_nonce_digest: digest("transfer"),
        }
    }

    #[test]
    fn registered_principal_rotation_preserves_removal_and_binding_revision_counts() {
        let owner = owner();
        let binding = RuntimeOwnerPrincipalBinding {
            principal_id: "principal-a".to_string(),
            profile_id: "profile-a".to_string(),
            profile_identity_digest: owner.profile_identity_digest.clone(),
            capability_id: "previous".to_string(),
            provenance: ServicePrincipalProvenance::RegisteredCapability,
            owner_generation: owner.owner_generation,
        };
        for owner_state in [
            None,
            Some(ProfileOwnerState::Orphaned),
            Some(ProfileOwnerState::Ready),
        ] {
            for has_binding in [false, true] {
                let mut registry = RuntimeOwnerRegistry::default();
                if let Some(state) = owner_state {
                    let mut owner = owner.clone();
                    owner.state = state;
                    registry
                        .owners
                        .insert(owner.profile_identity_digest.clone(), owner);
                }
                if has_binding {
                    registry
                        .principal_bindings
                        .insert(binding.profile_identity_digest.clone(), binding.clone());
                }
                let ready = owner_state == Some(ProfileOwnerState::Ready);
                assert_eq!(
                    registry.rotate_registered_principal_authority(
                        &owner.profile_identity_digest,
                        "previous",
                        "principal-a",
                        "profile-a",
                        "next",
                    ),
                    Ok(ready)
                );
                assert_eq!(
                    registry.revision(),
                    u64::from(has_binding) + u64::from(ready)
                );
                assert_eq!(registry.principal_bindings().len(), usize::from(ready));
                if ready {
                    assert_eq!(
                        registry.principal_bindings()[&owner.profile_identity_digest].capability_id,
                        "next"
                    );
                }
            }
        }
        let mut registry = RuntimeOwnerRegistry::from_owner(owner.clone());
        registry
            .principal_bindings
            .insert(binding.profile_identity_digest.clone(), binding);
        let original = registry.clone();
        assert_eq!(
            registry.rotate_registered_principal_authority(
                &owner.profile_identity_digest,
                "wrong",
                "principal-a",
                "profile-a",
                "next",
            ),
            Err(RuntimeOwnerPrincipalRotationError::RotationMismatch)
        );
        assert_eq!(registry, original);
        assert_eq!(
            registry.rotate_registered_principal_authority(
                &owner.profile_identity_digest,
                "previous",
                "principal-a",
                "profile-a",
                "",
            ),
            Err(RuntimeOwnerPrincipalRotationError::BindingFailed(
                transfer_error(OwnerTransferFailureCode::InvalidEvidence)
            ))
        );
        assert_eq!(registry.revision(), original.revision() + 1);
        assert!(registry.principal_bindings().is_empty());
    }

    #[test]
    fn runtime_reset_terminalization_preserves_evidence_and_bumps_once_per_application() {
        let owner = owner();
        let mut registry = RuntimeOwnerRegistry::from_owner(owner.clone());
        let original_record = RuntimeLifecycleRecord {
            logical_browser_id: owner.browser_id.clone(),
            profile_identity_digest: owner.profile_identity_digest.clone(),
            owner_generation: owner.owner_generation,
            boot_epoch: Some("retained-boot".to_string()),
            process_group_id: Some(4100),
            terminal_evidence: vec!["prior-evidence".to_string()],
            ..RuntimeLifecycleRecord::default()
        };
        registry.restore_lifecycle_records(BTreeMap::from([(
            owner.browser_id.clone(),
            original_record.clone(),
        )]));
        for revision in [2, 3] {
            registry.apply_runtime_reset_terminalization(
                &owner.profile_identity_digest,
                &owner.browser_id,
            );
            assert_eq!(registry.revision(), revision);
            assert_eq!(
                registry.owners()[&owner.profile_identity_digest],
                ProfileOwner {
                    state: ProfileOwnerState::Orphaned,
                    ..owner.clone()
                }
            );
            assert_eq!(
                registry.lifecycle_records()[&owner.browser_id],
                RuntimeLifecycleRecord {
                    lifecycle_state: RuntimeLaneLifecycleState::Terminal,
                    cleanup_obligation_state: CleanupObligationState::Satisfied,
                    terminal_evidence: vec![
                        "prior-evidence".to_string(),
                        "profile_runtime_reset_applied".to_string()
                    ],
                    ..original_record.clone()
                }
            );
        }
    }

    #[test]
    fn cold_shutdown_releases_owner_authority_and_preserves_terminal_history() {
        let owner = owner();
        let mut registry = RuntimeOwnerRegistry::from_owner(owner.clone());
        registry.restore_lifecycle_records(BTreeMap::from([(
            owner.browser_id.clone(),
            RuntimeLifecycleRecord {
                logical_browser_id: owner.browser_id.clone(),
                profile_identity_digest: owner.profile_identity_digest.clone(),
                owner_generation: owner.owner_generation,
                lifecycle_state: RuntimeLaneLifecycleState::Ready,
                cleanup_obligation_state: CleanupObligationState::Owned,
                process_group_id: Some(4100),
                package_launch_identity_digest: Some(digest("package-launch")),
                terminal_evidence: vec!["prior-evidence".to_string()],
                ..RuntimeLifecycleRecord::default()
            },
        )]));

        assert_eq!(registry.release_all_for_cold_shutdown(), 1);
        assert!(registry.owners().is_empty());
        assert!(registry.principal_bindings().is_empty());
        assert_eq!(registry.revision(), 2);
        let lifecycle = &registry.lifecycle_records()[&owner.browser_id];
        assert_eq!(
            lifecycle.lifecycle_state,
            RuntimeLaneLifecycleState::Terminal
        );
        assert_eq!(
            lifecycle.cleanup_obligation_state,
            CleanupObligationState::Satisfied
        );
        assert_eq!(lifecycle.process_group_id, None);
        assert_eq!(lifecycle.package_launch_identity_digest, None);
        assert_eq!(
            lifecycle.terminal_evidence,
            vec![
                "prior-evidence".to_string(),
                "workstation_cold_shutdown".to_string()
            ]
        );

        let after_first = registry.clone();
        assert_eq!(registry.release_all_for_cold_shutdown(), 0);
        assert_eq!(registry, after_first);
    }

    #[test]
    fn lifecycle_persistence_projection_and_restore_preserve_authority_revision() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        let records =
            BTreeMap::from([("sidecar-key".to_string(), RuntimeLifecycleRecord::default())]);
        registry.restore_lifecycle_records(records.clone());
        assert_eq!(registry.revision(), 1);
        let projected = registry.persistence_projection_without_lifecycle_records();
        assert!(projected.lifecycle_records().is_empty());
        assert_eq!(projected.revision(), registry.revision());
        assert_eq!(projected.owners(), registry.owners());
        assert_eq!(
            projected.principal_bindings(),
            registry.principal_bindings()
        );
        assert_eq!(registry.lifecycle_records(), &records);
        registry.restore_lifecycle_records(BTreeMap::new());
        assert!(registry.lifecycle_records().is_empty());
        assert_eq!(registry, projected);
    }

    #[test]
    fn lifecycle_managed_registration_preserves_revision_replay_and_boot_observation() {
        let mut registry = RuntimeOwnerRegistry::default();
        let current_owner = owner();
        let intent = RuntimeLifecycleIntent::RegisterManagedLane {
            owner: current_owner.clone(),
            process_group_id: Some(4100),
            package_launch_identity_digest: digest("launch"),
            boot_epoch: Some("boot:injected".to_string()),
        };
        assert_eq!(
            registry.apply_lifecycle_transition(intent.clone()).unwrap(),
            RuntimeLifecycleTransition::OwnerRegistered(current_owner.clone())
        );
        assert_eq!(registry.revision(), 2);
        assert_eq!(registry.owners().len(), 1);
        assert!(registry.principal_bindings().is_empty());
        assert_eq!(
            registry.lifecycle_records()["browser-a"]
                .boot_epoch
                .as_deref(),
            Some("boot:injected")
        );
        registry.apply_lifecycle_transition(intent).unwrap();
        assert_eq!(registry.revision(), 2);
        registry
            .apply_lifecycle_transition(RuntimeLifecycleIntent::PreserveRetained {
                claim: OwnerAuthorityClaim::from_owner(&current_owner),
            })
            .unwrap();
        assert_eq!(registry.revision(), 3);
        assert_eq!(
            registry.lifecycle_records()["browser-a"]
                .boot_epoch
                .as_deref(),
            Some("boot:injected")
        );
    }

    #[test]
    fn lifecycle_failed_transfer_preserves_prior_owner_mutation() {
        let current_owner = owner();
        let mut registry = RuntimeOwnerRegistry::from_owner(current_owner.clone());
        registry.lifecycle_records.insert(
            current_owner.browser_id.clone(),
            RuntimeLifecycleRecord {
                logical_browser_id: current_owner.browser_id.clone(),
                profile_identity_digest: current_owner.profile_identity_digest.clone(),
                owner_generation: current_owner.owner_generation - 1,
                ..RuntimeLifecycleRecord::default()
            },
        );
        let original_record = registry.lifecycle_records()["browser-a"].clone();
        assert_eq!(
            registry.apply_lifecycle_transition(RuntimeLifecycleIntent::BeginTransfer(
                cooperative_request()
            )),
            Err("runtime_lifecycle_generation_mismatch".to_string())
        );
        assert_eq!(registry.revision(), 2);
        assert!(registry.owners()[&current_owner.profile_identity_digest]
            .pending_transfer
            .is_some());
        assert_eq!(registry.lifecycle_records()["browser-a"], original_record);
    }

    #[test]
    fn lifecycle_launch_validation_keeps_variant_specific_first_error() {
        let mut invalid_owner = owner();
        invalid_owner.owner_generation = 0;
        let mut registry = RuntimeOwnerRegistry::default();
        assert_eq!(
            registry.apply_lifecycle_transition(RuntimeLifecycleIntent::RegisterManagedLane {
                owner: invalid_owner.clone(),
                process_group_id: None,
                package_launch_identity_digest: "invalid".to_string(),
                boot_epoch: None,
            }),
            Err("runtime_lifecycle_package_launch_identity_invalid".to_string())
        );
        assert_eq!(
            registry.apply_lifecycle_transition(
                RuntimeLifecycleIntent::ActivateTerminalReplacement {
                    owner: invalid_owner,
                    process_group_id: None,
                    package_launch_identity_digest: "invalid".to_string(),
                    boot_epoch: None,
                }
            ),
            Err(
                "runtime_owner_transfer_InvalidEvidence: owner transfer evidence is invalid"
                    .to_string()
            )
        );
        assert_eq!(registry, RuntimeOwnerRegistry::default());
    }

    #[test]
    fn legacy_registry_defaults_remain_conservative_and_empty_ignores_lifecycle_history() {
        let mut registry: RuntimeOwnerRegistry = serde_json::from_str("{}").unwrap();
        assert!(registry.is_empty());
        assert_eq!(registry.revision, 0);
        registry.lifecycle_records.insert(
            "browser-history".to_string(),
            RuntimeLifecycleRecord::default(),
        );
        assert!(registry.is_empty());
        assert_eq!(
            serde_json::to_value(RuntimeLaneLifecycleState::default()).unwrap(),
            serde_json::json!("unknown")
        );
        assert_eq!(
            serde_json::to_value(CleanupObligationState::default()).unwrap(),
            serde_json::json!("unknown")
        );
    }

    #[test]
    fn generation_seven_commit_replay_and_reverse_preserve_custody_receipts() {
        let initial_owner = owner();
        let profile_digest = initial_owner.profile_identity_digest.clone();
        let old_claim = OwnerAuthorityClaim::from_owner(&initial_owner);
        let mut registry = RuntimeOwnerRegistry::from_owner(initial_owner);
        registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: "principal-a".to_string(),
                profile_id: "profile-a".to_string(),
                profile_identity_digest: profile_digest.clone(),
                capability_id: "capability-a".to_string(),
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 7,
            })
            .unwrap();
        assert!(registry.authorizes(&old_claim));

        let request = cooperative_request();
        let proposal = registry.begin_transfer(request.clone()).unwrap();
        assert_eq!(proposal.previous_owner_generation, 7);
        assert_eq!(proposal.candidate_owner_generation, 8);
        let attachment =
            CandidateOwnerAttachment::from_request(&request, proposal.candidate_owner_generation);
        assert!(!attachment.effect_capable);
        assert!(registry.authorizes(&old_claim));

        let receipt = registry.commit_candidate(attachment.clone()).unwrap();
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        assert_eq!(receipt.candidate_owner_generation, 8);
        assert!(!registry.authorizes(&old_claim));
        assert_eq!(
            registry.principal_bindings[&profile_digest].owner_generation,
            8
        );

        let replay = registry.commit_candidate(attachment).unwrap();
        assert_eq!(serde_json::to_vec(&replay).unwrap(), receipt_bytes);
        assert_eq!(registry.owner(&profile_digest).unwrap().owner_generation, 8);

        let reverse = registry
            .reverse_transfer(ReverseOwnerTransferRequest {
                profile_identity_digest: profile_digest.clone(),
                expected_candidate_owner_id: "owner-new".to_string(),
                expected_candidate_owner_generation: 8,
                transfer_nonce_digest: digest("transfer"),
                reverse_nonce_digest: digest("reverse"),
            })
            .unwrap();
        assert_eq!(
            reverse.transition_kind,
            OwnerTransferTransitionKind::Reverse
        );
        assert_eq!(reverse.candidate_owner_generation, 9);
        assert_eq!(registry.owner(&profile_digest).unwrap().owner_generation, 9);
        assert_eq!(
            registry.principal_bindings[&profile_digest].owner_generation,
            9
        );
        assert!(
            registry.principal_binding_is_current(registry.principal_bindings.get(&profile_digest))
        );

        let binding = owner_binding_in_registry(&registry, "session-old")
            .unwrap()
            .unwrap();
        assert!(binding.effect_capable);
        assert_eq!(binding.claim.owner_generation, 9);
    }
}
