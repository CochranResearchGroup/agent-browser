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
}

impl OwnerAuthorityClaim {
    pub(crate) fn from_owner(owner: &ProfileOwner) -> Self {
        Self {
            owner_id: owner.owner_id.clone(),
            profile_identity_digest: owner.profile_identity_digest.clone(),
            owner_generation: owner.owner_generation,
            logical_browser_id: owner.browser_id.clone(),
            daemon_session_route: owner.daemon_session_route.clone(),
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeOwnerRegistry {
    pub(crate) revision: u64,
    pub(crate) owners: BTreeMap<String, ProfileOwner>,
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
        }
    }

    pub(crate) fn owner(&self, profile_identity_digest: &str) -> Option<&ProfileOwner> {
        self.owners.get(profile_identity_digest)
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
            })
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
        if request.expected_owner_id.as_deref() != Some(owner.owner_id.as_str())
            || request.expected_owner_generation != owner.owner_generation
            || request.logical_browser_id != owner.browser_id
            || request.process_instance_digest != owner.process_instance_digest
            || request.browser_family != owner.browser_family
            || request.cdp_endpoint_identity_digest != owner.cdp_endpoint_identity_digest
            || request.target_set_digest != owner.target_set_digest
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
        owner.state = ProfileOwnerState::Ready;
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

pub(crate) fn begin_owner_transfer(
    repository: &impl ServiceStateRepository,
    request: OwnerTransferRequest,
) -> Result<OwnerTransferProposal, String> {
    repository.mutate(|state| {
        state
            .runtime_owner_registry
            .begin_transfer(request)
            .map_err(owner_transfer_error_text)
    })
}

pub(crate) fn commit_candidate_owner(
    repository: &impl ServiceStateRepository,
    attachment: CandidateOwnerAttachment,
) -> Result<OwnerTransferReceipt, String> {
    repository.mutate(|state| {
        state
            .runtime_owner_registry
            .commit_candidate(attachment)
            .map_err(owner_transfer_error_text)
    })
}

pub(crate) fn reverse_candidate_owner(
    repository: &impl ServiceStateRepository,
    request: ReverseOwnerTransferRequest,
) -> Result<OwnerTransferReceipt, String> {
    repository.mutate(|state| {
        state
            .runtime_owner_registry
            .reverse_transfer(request)
            .map_err(owner_transfer_error_text)
    })
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

/// Fence browser effects for daemons participating in owner transfer.
/// Legacy daemons remain unbound until a transfer begins. Once bound, reads
/// may continue but every browser effect requires the current registry owner.
pub(crate) fn require_owner_effect_authority(
    repository: &impl ServiceStateRepository,
    binding: &RuntimeOwnerBinding,
    action: &str,
) -> Result<(), String> {
    if action_is_observation_only(action) {
        return Ok(());
    }
    if !binding.effect_capable {
        return Err("runtime_owner_observation_only: candidate cannot issue browser effects before owner compare-and-swap".to_string());
    }
    if !owner_authority_is_current(repository, &binding.claim)? {
        return Err(
            "runtime_owner_generation_stale: daemon is no longer the effect-capable browser owner"
                .to_string(),
        );
    }
    Ok(())
}

fn action_is_observation_only(action: &str) -> bool {
    matches!(
        action,
        "browser_pid"
            | "cdp_url"
            | "console"
            | "content"
            | "cookies_get"
            | "diagnostics"
            | "desktop_prompt_observe"
            | "errors"
            | "inspect"
            | "probe"
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
    use crate::native::service_model::ServiceState;
    use crate::native::service_store::{ServiceStateRepository, ServiceStateStore};
    use std::sync::Mutex;

    const OWNER_TRANSFER_FIXTURES: &str =
        include_str!("../../docs/dev/fixtures/runtime-adoption/owner-transfer.v1.json");

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
        assert!(!registry.authorizes(&OwnerAuthorityClaim {
            owner_id: "owner-old".to_string(),
            profile_identity_digest: digest("profile"),
            owner_generation: 7,
            logical_browser_id: "browser-a".to_string(),
            daemon_session_route: "session-old".to_string(),
        }));
        assert!(registry.authorizes(&OwnerAuthorityClaim::from_owner(
            registry.owner(&request.profile_identity_digest).unwrap()
        )));
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
        assert_eq!(
            registry.owner(&digest("profile")).unwrap().owner_id,
            "owner-adopter"
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
        let old_binding = RuntimeOwnerBinding::effect_capable(OwnerAuthorityClaim::from_owner(
            repository
                .load_snapshot()
                .unwrap()
                .runtime_owner_registry
                .owner(&digest("profile"))
                .unwrap(),
        ));
        let candidate_binding = RuntimeOwnerBinding::observation_only(OwnerAuthorityClaim {
            owner_id: "owner-new".to_string(),
            profile_identity_digest: digest("profile"),
            owner_generation: 8,
            logical_browser_id: "browser-a".to_string(),
            daemon_session_route: "session-new".to_string(),
        });

        assert!(require_owner_effect_authority(&repository, &old_binding, "navigate").is_ok());
        assert!(
            require_owner_effect_authority(&repository, &candidate_binding, "navigate").is_err()
        );
        begin_owner_transfer(&repository, request.clone()).unwrap();
        commit_candidate_owner(
            &repository,
            CandidateOwnerAttachment::from_request(&request, 8),
        )
        .unwrap();

        assert!(require_owner_effect_authority(&repository, &old_binding, "navigate").is_err());
        assert!(require_owner_effect_authority(&repository, &old_binding, "tab_list").is_ok());
        let committed_candidate = RuntimeOwnerBinding::effect_capable(candidate_binding.claim);
        assert!(
            require_owner_effect_authority(&repository, &committed_candidate, "navigate").is_ok()
        );
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

        begin_owner_transfer(&repository, request.clone()).unwrap();
        let receipt = commit_candidate_owner(
            &repository,
            CandidateOwnerAttachment::from_request(&request, 8),
        )
        .unwrap();
        let persisted = repository.load_snapshot().unwrap();

        assert_eq!(receipt.candidate_owner_generation, 8);
        assert_eq!(persisted.runtime_owner_registry.revision, 3);
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
            .find("if let Some(binding) = state.runtime_owner_binding.as_ref()")
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
