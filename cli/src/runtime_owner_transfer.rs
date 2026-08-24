//! Provider-neutral ownership transfer for live browser adoption.
//!
//! The registry model deliberately mirrors Plan 0111's canonical profile
//! owner vocabulary. A pending transfer is observation-only. Effect authority
//! changes only when the owner-generation compare-and-swap commits.

use crate::native::service_store::ServiceStateRepository;
use crate::runtime_adoption::BrowserAdoptionMode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProfileOwnerState {
    Reserving,
    Ready,
    Releasing,
    Orphaned,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileOwner {
    pub(crate) owner_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) state: ProfileOwnerState,
    pub(crate) owner_generation: u64,
    pub(crate) browser_id: String,
    pub(crate) daemon_session_route: String,
    pub(crate) process_instance_digest: String,
    pub(crate) browser_family: String,
    pub(crate) cdp_endpoint_identity_digest: String,
    pub(crate) target_set_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pending_transfer: Option<OwnerTransferProposal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_transition: Option<OwnerTransitionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OwnerTransferRequest {
    pub(crate) mode: BrowserAdoptionMode,
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) expected_owner_id: Option<String>,
    pub(crate) expected_owner_generation: u64,
    pub(crate) candidate_owner_id: String,
    pub(crate) candidate_daemon_session_route: String,
    pub(crate) process_instance_digest: String,
    pub(crate) browser_family: String,
    pub(crate) cdp_endpoint_identity_digest: String,
    pub(crate) target_set_digest: String,
    pub(crate) selected_target_identity_digest: String,
    pub(crate) transfer_nonce_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OwnerTransferProposal {
    pub(crate) request: OwnerTransferRequest,
    pub(crate) previous_owner_generation: u64,
    pub(crate) candidate_owner_generation: u64,
    pub(crate) candidate_effect_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CandidateOwnerAttachment {
    pub(crate) candidate_owner_id: String,
    pub(crate) candidate_daemon_session_route: String,
    pub(crate) candidate_owner_generation: u64,
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) process_instance_digest: String,
    pub(crate) browser_family: String,
    pub(crate) cdp_endpoint_identity_digest: String,
    pub(crate) target_set_digest: String,
    pub(crate) selected_target_identity_digest: String,
    pub(crate) transfer_nonce_digest: String,
    pub(crate) effect_capable: bool,
}

impl CandidateOwnerAttachment {
    pub(crate) fn from_request(request: &OwnerTransferRequest, generation: u64) -> Self {
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
pub(crate) struct OwnerAuthorityClaim {
    pub(crate) owner_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) owner_generation: u64,
    pub(crate) logical_browser_id: String,
    pub(crate) daemon_session_route: String,
    pub(crate) process_instance_digest: String,
}

impl OwnerAuthorityClaim {
    pub(crate) fn from_owner(owner: &ProfileOwner) -> Self {
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
pub(crate) struct RuntimeOwnerBinding {
    pub(crate) claim: OwnerAuthorityClaim,
    pub(crate) effect_capable: bool,
}

impl RuntimeOwnerBinding {
    pub(crate) fn observation_only(claim: OwnerAuthorityClaim) -> Self {
        Self {
            claim,
            effect_capable: false,
        }
    }

    pub(crate) fn effect_capable(claim: OwnerAuthorityClaim) -> Self {
        Self {
            claim,
            effect_capable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeOwnerHandoffReceiptAttestation {
    pub(crate) receipt_id: String,
    pub(crate) receipt_sha256: String,
    pub(crate) transition_kind: OwnerTransferTransitionKind,
    pub(crate) owner_generation: u64,
    pub(crate) state: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeOwnerAttestation {
    pub(crate) owner_id: String,
    pub(crate) owner_generation: u64,
    pub(crate) owner_state: ProfileOwnerState,
    pub(crate) logical_browser_id: String,
    pub(crate) daemon_session_route: String,
    pub(crate) process_instance_digest: String,
    pub(crate) effect_capable: bool,
    pub(crate) handoff_receipt: Option<RuntimeOwnerHandoffReceiptAttestation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OwnerTransferTransitionKind {
    Commit,
    Reverse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OwnerTransferReceipt {
    pub(crate) receipt_id: String,
    pub(crate) transition_kind: OwnerTransferTransitionKind,
    pub(crate) mode: BrowserAdoptionMode,
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) process_instance_digest: String,
    pub(crate) cdp_endpoint_identity_digest: String,
    pub(crate) target_set_digest: String,
    pub(crate) selected_target_identity_digest: String,
    pub(crate) previous_owner_id: String,
    pub(crate) candidate_owner_id: String,
    pub(crate) previous_owner_generation: u64,
    pub(crate) candidate_owner_generation: u64,
    pub(crate) transfer_nonce_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileOwnerRollbackSnapshot {
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
pub(crate) struct OwnerTransitionRecord {
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
pub(crate) enum RuntimeLaneLifecycleState {
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
pub(crate) enum CleanupObligationState {
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
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeLifecycleRecord {
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) owner_generation: u64,
    pub(crate) lifecycle_state: RuntimeLaneLifecycleState,
    pub(crate) cleanup_obligation_state: CleanupObligationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) process_group_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) package_launch_identity_digest: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) terminal_evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeOwnerRegistry {
    pub(crate) revision: u64,
    pub(crate) owners: BTreeMap<String, ProfileOwner>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) lifecycle_records: BTreeMap<String, RuntimeLifecycleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReverseOwnerTransferRequest {
    pub(crate) profile_identity_digest: String,
    pub(crate) expected_candidate_owner_id: String,
    pub(crate) expected_candidate_owner_generation: u64,
    pub(crate) transfer_nonce_digest: String,
    pub(crate) reverse_nonce_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerTransferFailureCode {
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
pub(crate) struct OwnerTransferError {
    pub(crate) code: OwnerTransferFailureCode,
    pub(crate) message: &'static str,
}

impl RuntimeOwnerRegistry {
    pub(crate) fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }

    pub(crate) fn from_owner(owner: ProfileOwner) -> Self {
        Self {
            revision: 1,
            owners: BTreeMap::from([(owner.profile_identity_digest.clone(), owner)]),
            lifecycle_records: BTreeMap::new(),
        }
    }

    pub(crate) fn owner(&self, profile_identity_digest: &str) -> Option<&ProfileOwner> {
        self.owners.get(profile_identity_digest)
    }

    pub(crate) fn register_current_owner(
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

    pub(crate) fn authorizes(&self, claim: &OwnerAuthorityClaim) -> bool {
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

    pub(crate) fn revoke_legacy_daemon_owner(
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

    pub(crate) fn binding_for_session(
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
        if matches.len() > 1 {
            return Err(format!(
                "runtime_owner_session_ambiguous: session '{session_id}' matches multiple profile owners"
            ));
        }
        Ok(matches.first().map(|owner| {
            let claim = OwnerAuthorityClaim::from_owner(owner);
            if owner.daemon_session_route == session_id {
                RuntimeOwnerBinding::effect_capable(claim)
            } else {
                RuntimeOwnerBinding::observation_only(claim)
            }
        }))
    }

    pub(crate) fn attestation_for_session(
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

    pub(crate) fn refreshed_claim_after_reverse(
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

    pub(crate) fn begin_transfer(
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

    pub(crate) fn commit_candidate(
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
        self.revision = self.revision.saturating_add(1);
        Ok(receipt)
    }

    pub(crate) fn abort_pending_transfer(
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

    pub(crate) fn reverse_transfer(
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
        self.revision = self.revision.saturating_add(1);
        Ok(receipt)
    }
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
    let registry = &snapshot.runtime_owner_registry;
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
/// observation remains available during transfer and in isolated test runs.
pub(crate) fn action_requires_owner_effect_authority(action: &str) -> bool {
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

pub(crate) fn validate_profile_owner(owner: &ProfileOwner) -> Result<(), OwnerTransferError> {
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
            mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
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
    }

    #[test]
    fn lifecycle_and_cleanup_obligation_schema_round_trips_without_effects() {
        let mut registry = RuntimeOwnerRegistry::from_owner(owner());
        registry.lifecycle_records.insert(
            "browser-a".to_string(),
            RuntimeLifecycleRecord {
                logical_browser_id: "browser-a".to_string(),
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
            decoded.lifecycle_records["browser-a"].lifecycle_state,
            RuntimeLaneLifecycleState::Retained
        );
        assert_eq!(
            decoded.lifecycle_records["browser-a"].cleanup_obligation_state,
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
        let lifecycle = &registry.lifecycle_records["browser-a"];
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
        registry
            .owners
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
                state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(owner());
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
        registry.lifecycle_records.insert(
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
        assert_eq!(persisted.runtime_owner_registry.revision, 5);
        assert_eq!(
            persisted.runtime_owner_registry.lifecycle_records["browser-a"]
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
        assert_eq!(persisted.runtime_owner_registry.revision, 2);
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
            .find("if state.runtime_owner_binding.is_none()")
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
