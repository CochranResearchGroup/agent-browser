//! Canonical active-claim authority for service-owned resources.
//!
//! Only `active_claims` may authorize or block effects. `events` is retained
//! append-only history and is never consulted for admission.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::service_store::ServiceStateRepository;

pub(crate) const LEASE_AUTHORITY_SCHEMA_VERSION: &str = "agent-browser.lease-authority.v1";
pub(crate) const LEASE_ACQUISITION_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.lease-acquisition-receipt.v1";
pub(crate) const LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION: &str =
    "agent-browser.lease-effect-authorization.v2";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeaseResourceKind {
    Profile,
    RuntimeLane,
    ServiceSession,
    Tab,
    Viewer,
    Controller,
    PresentationRoute,
    InstallerTransaction,
}

impl LeaseResourceKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::RuntimeLane => "runtime_lane",
            Self::ServiceSession => "service_session",
            Self::Tab => "tab",
            Self::Viewer => "viewer",
            Self::Controller => "controller",
            Self::PresentationRoute => "presentation_route",
            Self::InstallerTransaction => "installer_transaction",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LeaseResourceKey {
    pub(crate) kind: LeaseResourceKind,
    pub(crate) id: String,
}

impl LeaseResourceKey {
    pub(crate) fn profile(id: impl Into<String>) -> Self {
        Self {
            kind: LeaseResourceKind::Profile,
            id: id.into(),
        }
    }

    fn storage_key(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeaseClaimMode {
    Ephemeral,
    Strict,
}

impl LeaseClaimMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::Strict => "strict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeaseEventKind {
    Acquired,
    Rejoined,
    Renewed,
    Released,
    Expired,
    Revoked,
    Recovered,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LeaseAuthorityEvent {
    pub(crate) event_id: String,
    pub(crate) resource: LeaseResourceKey,
    pub(crate) claim_id: String,
    pub(crate) principal_id: String,
    pub(crate) fencing_token: u64,
    pub(crate) kind: LeaseEventKind,
    pub(crate) occurred_at: String,
}

/// Durable result of one logical acquisition operation. Retaining this receipt
/// never retains or recreates operational authority after the claim expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LeaseClaimAcquisitionReceipt {
    schema_version: String,
    receipt_id: String,
    request_digest: String,
    idempotency_key: String,
    resource: LeaseResourceKey,
    principal_id: String,
    capability_id: String,
    #[serde(default)]
    capability_revision: u64,
    claim_id: String,
    claim_revision: u64,
    fencing_token: u64,
    authority_revision: u64,
    occurred_at: String,
}

/// Atomic acquisition result. A replay may intentionally return no current
/// claim while preserving the original receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeaseClaimAcquisitionOutcome {
    pub(crate) claim: Option<ActiveLeaseClaim>,
    pub(crate) receipt: LeaseClaimAcquisitionReceipt,
    pub(crate) replayed: bool,
}

/// Exact claim envelope revalidated against canonical authority immediately
/// before an effect. It contains no raw capability material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LeaseEffectAuthorization {
    schema_version: String,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    capability_id: String,
    capability_revision: u64,
    claim_revision: u64,
    fencing_token: u64,
    owner_generation: Option<u64>,
    proof: String,
}

impl LeaseEffectAuthorization {
    pub(crate) fn profile_id(&self) -> Option<&str> {
        (self.resource.kind == LeaseResourceKind::Profile).then_some(self.resource.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActiveLeaseClaim {
    schema_version: String,
    claim_id: String,
    resource: LeaseResourceKey,
    parent_claim_id: Option<String>,
    principal_id: String,
    capability_id: String,
    #[serde(default)]
    capability_revision: u64,
    mode: LeaseClaimMode,
    revision: u64,
    fencing_token: u64,
    idempotency_key: String,
    acquired_at: String,
    heartbeat_at: String,
    expires_at: String,
    transition_deadline: Option<String>,
    recovery_controller_id: Option<String>,
    boot_epoch: Option<String>,
    owner_generation: Option<u64>,
}

impl ActiveLeaseClaim {
    pub(crate) fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub(crate) fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub(crate) fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub(crate) fn profile_id(&self) -> Option<&str> {
        (self.resource.kind == LeaseResourceKind::Profile).then_some(self.resource.id.as_str())
    }

    pub(crate) fn mode(&self) -> LeaseClaimMode {
        self.mode
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn fencing_token(&self) -> u64 {
        self.fencing_token
    }

    pub(crate) fn heartbeat_at(&self) -> &str {
        &self.heartbeat_at
    }

    pub(crate) fn expires_at(&self) -> &str {
        &self.expires_at
    }

    pub(crate) fn owner_generation(&self) -> Option<u64> {
        self.owner_generation
    }

    pub(crate) fn effect_authorization(
        &self,
        capability: &super::service_principal::ServiceProfileCapability,
    ) -> Result<LeaseEffectAuthorization, LeaseAuthorityError> {
        if capability.capability_id != self.capability_id
            || capability.principal_id != self.principal_id
            || capability.revision != self.capability_revision
            || capability.state != super::service_principal::ServiceProfileCapabilityState::Active
            || self
                .profile_id()
                .is_some_and(|profile_id| capability.profile_id != profile_id)
        {
            return Err(LeaseAuthorityError::CapabilityMismatch);
        }
        let mut authorization = LeaseEffectAuthorization {
            schema_version: LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION.to_string(),
            resource: self.resource.clone(),
            claim_id: self.claim_id.clone(),
            principal_id: self.principal_id.clone(),
            capability_id: self.capability_id.clone(),
            capability_revision: self.capability_revision,
            claim_revision: self.revision,
            fencing_token: self.fencing_token,
            owner_generation: self.owner_generation,
            proof: String::new(),
        };
        authorization.proof = sign_effect_authorization(&authorization, capability)?;
        Ok(authorization)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct LeaseAuthorityState {
    schema_version: String,
    revision: u64,
    active_claims: BTreeMap<String, ActiveLeaseClaim>,
    next_fencing_tokens: BTreeMap<String, u64>,
    events: Vec<LeaseAuthorityEvent>,
    acquisition_receipts: BTreeMap<String, LeaseClaimAcquisitionReceipt>,
}

impl LeaseAuthorityState {
    pub(crate) fn is_empty(&self) -> bool {
        self.active_claims.is_empty()
            && self.next_fencing_tokens.is_empty()
            && self.events.is_empty()
            && self.acquisition_receipts.is_empty()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn acquire_with_receipt(
        &mut self,
        request: AcquireLeaseClaimRequest,
    ) -> Result<LeaseClaimAcquisitionOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(replayed) = self.replay_acquisition(&request)? {
            return Ok(replayed);
        }
        let request_digest = acquisition_request_digest(&request);
        validate_request(&request)?;
        if request.expected_authority_revision != self.revision {
            return Err(LeaseAuthorityError::StaleAuthorityRevision);
        }

        let resource_key = request.resource.storage_key();
        if let Some(current) = self.active_claims.get(&resource_key).cloned() {
            if timestamp_precedes(&request.now, &current.expires_at)
                && ephemeral_claim_can_be_rejoined(&current, &request)
            {
                let next_authority_revision = self
                    .revision
                    .checked_add(1)
                    .filter(|revision| *revision > 0)
                    .ok_or(LeaseAuthorityError::CounterExhausted)?;
                let receipt = acquisition_receipt(
                    &request.idempotency_key,
                    request_digest,
                    &current,
                    next_authority_revision,
                    &request.now,
                );
                let event = LeaseAuthorityEvent {
                    event_id: stable_id(
                        "lease-event-v1",
                        &format!(
                            "{}\0rejoined\0{}\0{}",
                            current.claim_id, request.idempotency_key, next_authority_revision
                        ),
                    ),
                    resource: current.resource.clone(),
                    claim_id: current.claim_id.clone(),
                    principal_id: current.principal_id.clone(),
                    fencing_token: current.fencing_token,
                    kind: LeaseEventKind::Rejoined,
                    occurred_at: request.now,
                };
                self.revision = next_authority_revision;
                self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
                self.events.push(event);
                self.acquisition_receipts
                    .insert(request.idempotency_key, receipt.clone());
                return Ok(LeaseClaimAcquisitionOutcome {
                    claim: Some(current),
                    receipt,
                    replayed: false,
                });
            }
        }

        let idempotency_key = request.idempotency_key.clone();
        let occurred_at = request.now.clone();
        let claim = self.acquire(request)?;
        let receipt = acquisition_receipt(
            &idempotency_key,
            request_digest,
            &claim,
            self.revision,
            &occurred_at,
        );
        self.acquisition_receipts
            .insert(idempotency_key, receipt.clone());
        Ok(LeaseClaimAcquisitionOutcome {
            claim: Some(claim),
            receipt,
            replayed: false,
        })
    }

    pub(crate) fn replay_acquisition(
        &self,
        request: &AcquireLeaseClaimRequest,
    ) -> Result<Option<LeaseClaimAcquisitionOutcome>, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        let Some(receipt) = self.acquisition_receipts.get(&request.idempotency_key) else {
            return Ok(None);
        };
        if receipt.schema_version != LEASE_ACQUISITION_RECEIPT_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        if receipt.request_digest != acquisition_request_digest(request) {
            return Err(LeaseAuthorityError::IdempotencyConflict);
        }
        let claim = self
            .active_claims
            .get(&request.resource.storage_key())
            .filter(|claim| {
                claim.claim_id == receipt.claim_id
                    && timestamp_precedes(&request.now, &claim.expires_at)
            })
            .cloned();
        Ok(Some(LeaseClaimAcquisitionOutcome {
            claim,
            receipt: receipt.clone(),
            replayed: true,
        }))
    }

    pub(crate) fn acquire(
        &mut self,
        request: AcquireLeaseClaimRequest,
    ) -> Result<ActiveLeaseClaim, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        let resource_key = request.resource.storage_key();
        if let Some(current) = self.active_claims.get(&resource_key) {
            if current.idempotency_key == request.idempotency_key {
                if claim_matches_request(current, &request) {
                    return Ok(current.clone());
                }
                return Err(LeaseAuthorityError::IdempotencyConflict);
            }
        }
        if request.expected_authority_revision != self.revision {
            return Err(LeaseAuthorityError::StaleAuthorityRevision);
        }
        validate_request(&request)?;
        if let Some(parent_claim_id) = request.parent_claim_id.as_deref() {
            let parent_is_active = self.active_claims.values().any(|claim| {
                claim.claim_id == parent_claim_id
                    && timestamp_precedes(&request.now, &claim.expires_at)
            });
            if !parent_is_active {
                return Err(LeaseAuthorityError::ParentClaimUnavailable);
            }
        }

        if let Some(current) = self.active_claims.get(&resource_key) {
            if timestamp_precedes(&request.now, &current.expires_at) {
                return Err(LeaseAuthorityError::ClaimConflict);
            }
        }
        let fencing_high_water = self
            .next_fencing_tokens
            .get(&resource_key)
            .copied()
            .into_iter()
            .chain(
                self.active_claims
                    .get(&resource_key)
                    .map(|claim| claim.fencing_token),
            )
            .max()
            .unwrap_or(0);
        let fencing_token = fencing_high_water
            .checked_add(1)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let next_authority_revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        if let Some(expired) = self.active_claims.remove(&resource_key) {
            self.events.push(terminal_event(
                &expired,
                LeaseEventKind::Expired,
                &request.now,
            ));
        }

        self.next_fencing_tokens
            .insert(resource_key.clone(), fencing_token);
        self.revision = next_authority_revision;
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        let claim_id = stable_id(
            "lease-claim-v1",
            &format!(
                "{}\0{}\0{}",
                resource_key, request.principal_id, request.idempotency_key
            ),
        );
        let claim = ActiveLeaseClaim {
            schema_version: LEASE_AUTHORITY_SCHEMA_VERSION.to_string(),
            claim_id,
            resource: request.resource,
            parent_claim_id: request.parent_claim_id,
            principal_id: request.principal_id,
            capability_id: request.capability_id,
            capability_revision: request.capability_revision,
            mode: request.mode,
            revision: 1,
            fencing_token,
            idempotency_key: request.idempotency_key,
            acquired_at: request.now.clone(),
            heartbeat_at: request.now.clone(),
            expires_at: request.expires_at,
            transition_deadline: request.transition_deadline,
            recovery_controller_id: request.recovery_controller_id,
            boot_epoch: request.boot_epoch,
            owner_generation: request.owner_generation,
        };
        self.events.push(LeaseAuthorityEvent {
            event_id: stable_id(
                "lease-event-v1",
                &format!("{}\0acquired\0{}", claim.claim_id, self.revision),
            ),
            resource: claim.resource.clone(),
            claim_id: claim.claim_id.clone(),
            principal_id: claim.principal_id.clone(),
            fencing_token: claim.fencing_token,
            kind: LeaseEventKind::Acquired,
            occurred_at: request.now,
        });
        self.active_claims.insert(resource_key, claim.clone());
        Ok(claim)
    }

    pub(crate) fn current_claim(
        &self,
        resource: &LeaseResourceKey,
        now: &str,
    ) -> Option<&ActiveLeaseClaim> {
        self.active_claims
            .get(&resource.storage_key())
            .filter(|claim| timestamp_precedes(now, &claim.expires_at))
    }

    pub(crate) fn current_profile_claims<'a>(
        &'a self,
        now: &'a str,
    ) -> impl Iterator<Item = &'a ActiveLeaseClaim> + 'a {
        self.active_claims.values().filter(move |claim| {
            claim.resource.kind == LeaseResourceKind::Profile
                && timestamp_precedes(now, &claim.expires_at)
        })
    }

    pub(crate) fn authorize_effect(
        &self,
        authorization: &LeaseEffectAuthorization,
        now: &str,
    ) -> Result<&ActiveLeaseClaim, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if authorization.schema_version != LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        let claim = self
            .active_claims
            .get(&authorization.resource.storage_key())
            .ok_or(LeaseAuthorityError::ClaimUnavailable)?;
        if claim.schema_version != LEASE_AUTHORITY_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        if !timestamp_precedes(now, &claim.expires_at) {
            return Err(LeaseAuthorityError::ClaimExpired);
        }
        if claim.claim_id != authorization.claim_id
            || claim.principal_id != authorization.principal_id
            || claim.capability_id != authorization.capability_id
            || claim.capability_revision != authorization.capability_revision
            || claim.revision != authorization.claim_revision
            || claim.fencing_token != authorization.fencing_token
            || claim.owner_generation != authorization.owner_generation
        {
            return Err(LeaseAuthorityError::StaleClaim);
        }
        Ok(claim)
    }

    fn ensure_supported_schema(&self) -> Result<(), LeaseAuthorityError> {
        if self.is_empty() || self.schema_version == LEASE_AUTHORITY_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(LeaseAuthorityError::UnsupportedSchema)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcquireLeaseClaimRequest {
    pub(crate) resource: LeaseResourceKey,
    pub(crate) parent_claim_id: Option<String>,
    pub(crate) principal_id: String,
    pub(crate) capability_id: String,
    pub(crate) capability_revision: u64,
    pub(crate) mode: LeaseClaimMode,
    pub(crate) expected_authority_revision: u64,
    pub(crate) idempotency_key: String,
    pub(crate) now: String,
    pub(crate) expires_at: String,
    pub(crate) transition_deadline: Option<String>,
    pub(crate) recovery_controller_id: Option<String>,
    pub(crate) boot_epoch: Option<String>,
    pub(crate) owner_generation: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaseAuthorityError {
    InvalidRequest,
    StaleAuthorityRevision,
    ClaimConflict,
    IdempotencyConflict,
    ParentClaimUnavailable,
    StrictRecoveryRequired,
    CounterExhausted,
    ClaimUnavailable,
    ClaimExpired,
    StaleClaim,
    CapabilityUnavailable,
    CapabilityRevoked,
    CapabilityMismatch,
    InvalidEffectProof,
    UnsupportedSchema,
}

impl LeaseAuthorityError {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::StaleAuthorityRevision => "stale_authority_revision",
            Self::ClaimConflict => "claim_conflict",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::ParentClaimUnavailable => "parent_claim_unavailable",
            Self::StrictRecoveryRequired => "strict_recovery_required",
            Self::CounterExhausted => "counter_exhausted",
            Self::ClaimUnavailable => "claim_unavailable",
            Self::ClaimExpired => "claim_expired",
            Self::StaleClaim => "stale_claim",
            Self::CapabilityUnavailable => "capability_unavailable",
            Self::CapabilityRevoked => "capability_revoked",
            Self::CapabilityMismatch => "capability_mismatch",
            Self::InvalidEffectProof => "invalid_effect_proof",
            Self::UnsupportedSchema => "unsupported_schema",
        }
    }
}

/// Atomically acquires one claim inside the canonical Service State mutation
/// boundary. A read-side plan never grants authority.
pub(crate) fn acquire_lease_claim_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: AcquireLeaseClaimRequest,
) -> Result<ActiveLeaseClaim, String> {
    repository.mutate(|state| {
        state
            .acquire_lease_claim(request)
            .map_err(|error| format!("lease_authority_{}", error.as_str()))
    })
}

pub(crate) fn acquire_lease_claim_with_receipt_in_repository<R: ServiceStateRepository>(
    repository: &R,
    request: AcquireLeaseClaimRequest,
) -> Result<LeaseClaimAcquisitionOutcome, String> {
    repository.mutate(|state| {
        state
            .acquire_lease_claim_with_receipt(request)
            .map_err(|error| format!("lease_authority_{}", error.as_str()))
    })
}

pub(crate) fn authorize_lease_effect_in_repository<R: ServiceStateRepository>(
    repository: &R,
    authorization: &LeaseEffectAuthorization,
    now: &str,
) -> Result<ActiveLeaseClaim, String> {
    let state = repository.load_snapshot()?;
    let claim = state
        .lease_authority()
        .authorize_effect(authorization, now)
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
    if capability.state != super::service_principal::ServiceProfileCapabilityState::Active {
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
    verify_effect_authorization(authorization, capability)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    if claim.resource.kind == LeaseResourceKind::Profile {
        let profile = state
            .profiles
            .get(&claim.resource.id)
            .ok_or_else(|| "lease_authority_effect_profile_missing".to_string())?;
        let profile_hint = profile
            .user_data_dir
            .as_deref()
            .ok_or_else(|| "lease_authority_effect_profile_identity_unavailable".to_string())?;
        let resolved =
            crate::runtime_profile::resolve_profile(Some(profile_hint), Some(&profile.id))?;
        let profile_identity_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&resolved.user_data_dir)?;
        let current_owner = state.runtime_owner_registry.owner(&profile_identity_digest);
        let owner_matches = match (claim.owner_generation, current_owner) {
            (None, None) => true,
            (Some(expected), Some(owner)) => {
                owner.owner_generation == expected
                    && owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                    && state
                        .runtime_owner_registry
                        .principal_bindings
                        .get(&profile_identity_digest)
                        .is_some_and(|binding| {
                            binding.owner_generation == expected
                                && binding.profile_id == claim.resource.id
                                && binding.principal_id == claim.principal_id
                                && binding.capability_id == claim.capability_id
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
    state: &super::service_model::ServiceState,
    claim: &ActiveLeaseClaim,
) -> Result<LeaseEffectAuthorization, String> {
    let current = state
        .lease_authority()
        .current_claim(&claim.resource, &claim.heartbeat_at)
        .filter(|current| {
            current.claim_id == claim.claim_id
                && current.revision == claim.revision
                && current.fencing_token == claim.fencing_token
        })
        .ok_or_else(|| "lease_authority_claim_unavailable".to_string())?;
    let capability = state
        .service_principals
        .profile_capabilities
        .get(current.capability_id())
        .ok_or_else(|| "lease_authority_capability_unavailable".to_string())?;
    current
        .effect_authorization(capability)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))
}

fn validate_request(request: &AcquireLeaseClaimRequest) -> Result<(), LeaseAuthorityError> {
    if request.resource.id.trim().is_empty()
        || request.principal_id.trim().is_empty()
        || request.capability_id.trim().is_empty()
        || request.capability_revision == 0
        || request.idempotency_key.trim().is_empty()
        || !timestamp_precedes(&request.now, &request.expires_at)
    {
        return Err(LeaseAuthorityError::InvalidRequest);
    }
    if request.mode == LeaseClaimMode::Strict
        && (request
            .recovery_controller_id
            .as_deref()
            .is_none_or(str::is_empty)
            || request
                .transition_deadline
                .as_deref()
                .is_none_or(|deadline| !timestamp_precedes(&request.now, deadline)))
    {
        return Err(LeaseAuthorityError::StrictRecoveryRequired);
    }
    Ok(())
}

fn timestamp_precedes(left: &str, right: &str) -> bool {
    let Ok(left) = chrono::DateTime::parse_from_rfc3339(left) else {
        return false;
    };
    let Ok(right) = chrono::DateTime::parse_from_rfc3339(right) else {
        return false;
    };
    left < right
}

fn claim_matches_request(claim: &ActiveLeaseClaim, request: &AcquireLeaseClaimRequest) -> bool {
    claim.resource == request.resource
        && claim.parent_claim_id == request.parent_claim_id
        && claim.principal_id == request.principal_id
        && claim.capability_id == request.capability_id
        && claim.capability_revision == request.capability_revision
        && claim.mode == request.mode
        && claim.expires_at == request.expires_at
        && claim.transition_deadline == request.transition_deadline
        && claim.recovery_controller_id == request.recovery_controller_id
        && claim.boot_epoch == request.boot_epoch
        && claim.owner_generation == request.owner_generation
}

fn ephemeral_claim_can_be_rejoined(
    claim: &ActiveLeaseClaim,
    request: &AcquireLeaseClaimRequest,
) -> bool {
    claim.mode == LeaseClaimMode::Ephemeral
        && request.mode == LeaseClaimMode::Ephemeral
        && claim.resource == request.resource
        && claim.parent_claim_id == request.parent_claim_id
        && claim.principal_id == request.principal_id
        && claim.capability_id == request.capability_id
        && claim.capability_revision == request.capability_revision
}

fn acquisition_receipt(
    idempotency_key: &str,
    request_digest: String,
    claim: &ActiveLeaseClaim,
    authority_revision: u64,
    occurred_at: &str,
) -> LeaseClaimAcquisitionReceipt {
    LeaseClaimAcquisitionReceipt {
        schema_version: LEASE_ACQUISITION_RECEIPT_SCHEMA_VERSION.to_string(),
        receipt_id: stable_id(
            "lease-acquisition-receipt-v1",
            &format!("{}\0{}", idempotency_key, claim.claim_id),
        ),
        request_digest,
        idempotency_key: idempotency_key.to_string(),
        resource: claim.resource.clone(),
        principal_id: claim.principal_id.clone(),
        capability_id: claim.capability_id.clone(),
        capability_revision: claim.capability_revision,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.revision,
        fencing_token: claim.fencing_token,
        authority_revision,
        occurred_at: occurred_at.to_string(),
    }
}

fn acquisition_request_digest(request: &AcquireLeaseClaimRequest) -> String {
    stable_id(
        "lease-acquisition-request-v1",
        &format!(
            "{}\0{}\0{}\0{}\0{}\0{}\0{}",
            request.resource.storage_key(),
            request.mode.as_str(),
            request.parent_claim_id.as_deref().unwrap_or_default(),
            request.principal_id,
            request.capability_id,
            request.capability_revision,
            request
                .recovery_controller_id
                .as_deref()
                .unwrap_or_default(),
        ),
    )
}

fn sign_effect_authorization(
    authorization: &LeaseEffectAuthorization,
    capability: &super::service_principal::ServiceProfileCapability,
) -> Result<String, LeaseAuthorityError> {
    let key = capability_effect_proof_key(capability)?;
    let mut mac =
        HmacSha256::new_from_slice(&key).map_err(|_| LeaseAuthorityError::CapabilityMismatch)?;
    mac.update(effect_authorization_payload(authorization).as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn verify_effect_authorization(
    authorization: &LeaseEffectAuthorization,
    capability: &super::service_principal::ServiceProfileCapability,
) -> Result<(), LeaseAuthorityError> {
    let key = capability_effect_proof_key(capability)?;
    let proof =
        hex::decode(&authorization.proof).map_err(|_| LeaseAuthorityError::InvalidEffectProof)?;
    let mut mac =
        HmacSha256::new_from_slice(&key).map_err(|_| LeaseAuthorityError::CapabilityMismatch)?;
    mac.update(effect_authorization_payload(authorization).as_bytes());
    mac.verify_slice(&proof)
        .map_err(|_| LeaseAuthorityError::InvalidEffectProof)
}

fn capability_effect_proof_key(
    capability: &super::service_principal::ServiceProfileCapability,
) -> Result<Vec<u8>, LeaseAuthorityError> {
    let encoded = capability
        .capability_digest
        .strip_prefix("sha256:")
        .ok_or(LeaseAuthorityError::CapabilityMismatch)?;
    let key = hex::decode(encoded).map_err(|_| LeaseAuthorityError::CapabilityMismatch)?;
    (key.len() == 32)
        .then_some(key)
        .ok_or(LeaseAuthorityError::CapabilityMismatch)
}

fn effect_authorization_payload(authorization: &LeaseEffectAuthorization) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        authorization.schema_version,
        authorization.resource.storage_key(),
        authorization.claim_id,
        authorization.principal_id,
        authorization.capability_id,
        authorization.capability_revision,
        authorization.claim_revision,
        authorization.fencing_token,
        authorization.owner_generation.unwrap_or_default(),
    )
}

fn terminal_event(
    claim: &ActiveLeaseClaim,
    kind: LeaseEventKind,
    occurred_at: &str,
) -> LeaseAuthorityEvent {
    LeaseAuthorityEvent {
        event_id: stable_id(
            "lease-event-v1",
            &format!("{}\0{:?}\0{}", claim.claim_id, kind, occurred_at),
        ),
        resource: claim.resource.clone(),
        claim_id: claim.claim_id.clone(),
        principal_id: claim.principal_id.clone(),
        fencing_token: claim.fencing_token,
        kind,
        occurred_at: occurred_at.to_string(),
    }
}

fn stable_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    format!("{prefix}:{:x}", digest)[..prefix.len() + 1 + 32].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::ServiceState;
    use std::sync::{Arc, Mutex};

    const NOW: &str = "2026-08-31T12:00:00Z";

    fn request() -> AcquireLeaseClaimRequest {
        AcquireLeaseClaimRequest {
            resource: LeaseResourceKey::profile("last30days-social"),
            parent_claim_id: None,
            principal_id: "principal:last30days".to_string(),
            capability_id: "capability:last30days-social".to_string(),
            capability_revision: 1,
            mode: LeaseClaimMode::Ephemeral,
            expected_authority_revision: 0,
            idempotency_key: "acquire:last30days:tick-1".to_string(),
            now: NOW.to_string(),
            expires_at: "2026-08-31T12:05:00Z".to_string(),
            transition_deadline: None,
            recovery_controller_id: None,
            boot_epoch: Some("boot-1".to_string()),
            owner_generation: None,
        }
    }

    fn capability() -> crate::native::service_principal::ServiceProfileCapability {
        crate::native::service_principal::ServiceProfileCapability {
            capability_id: "capability:last30days-social".to_string(),
            principal_id: "principal:last30days".to_string(),
            profile_id: "last30days-social".to_string(),
            capability_digest: format!(
                "sha256:{:x}",
                Sha256::digest(b"last30days-test-effect-proof-capability")
            ),
            state: crate::native::service_principal::ServiceProfileCapabilityState::Active,
            revision: 1,
            issued_at: Some(NOW.to_string()),
        }
    }

    #[test]
    fn terminal_events_never_block_atomic_acquisition() {
        let resource = LeaseResourceKey::profile("last30days-social");
        let mut authority = LeaseAuthorityState {
            schema_version: LEASE_AUTHORITY_SCHEMA_VERSION.to_string(),
            next_fencing_tokens: BTreeMap::from([(resource.storage_key(), 41)]),
            events: vec![LeaseAuthorityEvent {
                event_id: "event-old-release".to_string(),
                resource: resource.clone(),
                claim_id: "claim-old".to_string(),
                principal_id: "principal:old-worker".to_string(),
                fencing_token: 41,
                kind: LeaseEventKind::Released,
                occurred_at: "2026-08-01T12:00:00Z".to_string(),
            }],
            ..LeaseAuthorityState::default()
        };

        let claim = authority.acquire(request()).unwrap();

        assert_eq!(claim.resource, resource);
        assert_eq!(claim.fencing_token, 42);
        assert_eq!(authority.active_claims.len(), 1);
        assert_eq!(authority.events.len(), 2);
    }

    #[test]
    fn acquisition_revision_compare_and_swap_has_one_winner() {
        let mut authority = LeaseAuthorityState::default();
        let first = authority.acquire(request()).unwrap();
        let mut contender = request();
        contender.principal_id = "principal:foreign".to_string();
        contender.capability_id = "capability:foreign".to_string();
        contender.idempotency_key = "acquire:foreign:tick-1".to_string();

        assert_eq!(
            authority.acquire(contender),
            Err(LeaseAuthorityError::StaleAuthorityRevision)
        );
        assert_eq!(authority.active_claims.len(), 1);
        assert_eq!(
            authority
                .current_claim(&LeaseResourceKey::profile("last30days-social"), NOW)
                .map(|claim| claim.claim_id.as_str()),
            Some(first.claim_id.as_str())
        );
    }

    #[test]
    fn strict_claim_requires_first_class_recovery_metadata() {
        let mut authority = LeaseAuthorityState::default();
        let mut strict = request();
        strict.mode = LeaseClaimMode::Strict;

        assert_eq!(
            authority.acquire(strict),
            Err(LeaseAuthorityError::StrictRecoveryRequired)
        );
        assert!(authority.active_claims.is_empty());
    }

    #[test]
    fn exhausted_fencing_counter_fails_before_authority_mutation() {
        let resource = LeaseResourceKey::profile("last30days-social");
        let mut authority = LeaseAuthorityState {
            schema_version: LEASE_AUTHORITY_SCHEMA_VERSION.to_string(),
            next_fencing_tokens: BTreeMap::from([(resource.storage_key(), u64::MAX)]),
            ..LeaseAuthorityState::default()
        };
        let before = authority.clone();

        assert_eq!(
            authority.acquire(request()),
            Err(LeaseAuthorityError::CounterExhausted)
        );
        assert_eq!(authority, before);
    }

    #[test]
    fn unsupported_authority_schema_fails_before_acquisition_mutation() {
        let resource = LeaseResourceKey::profile("last30days-social");
        let mut authority = LeaseAuthorityState {
            schema_version: "agent-browser.lease-authority.v0".to_string(),
            next_fencing_tokens: BTreeMap::from([(resource.storage_key(), 41)]),
            ..LeaseAuthorityState::default()
        };
        let before = authority.clone();

        assert_eq!(
            authority.acquire_with_receipt(request()),
            Err(LeaseAuthorityError::UnsupportedSchema)
        );
        assert_eq!(authority, before);
    }

    #[test]
    fn unsupported_receipt_schema_cannot_be_replayed() {
        let mut authority = LeaseAuthorityState::default();
        authority.acquire_with_receipt(request()).unwrap();
        authority
            .acquisition_receipts
            .get_mut("acquire:last30days:tick-1")
            .unwrap()
            .schema_version = "agent-browser.lease-acquisition-receipt.v0".to_string();
        let before = authority.clone();

        assert_eq!(
            authority.acquire_with_receipt(request()),
            Err(LeaseAuthorityError::UnsupportedSchema)
        );
        assert_eq!(authority, before);
    }

    #[test]
    fn acquisition_receipt_replay_after_expiry_grants_no_new_authority() {
        let mut authority = LeaseAuthorityState::default();
        let first = authority.acquire_with_receipt(request()).unwrap();
        assert!(!first.replayed);
        assert!(first.claim.is_some());
        let after_first = authority.clone();
        let mut replay = request();
        replay.now = "2026-08-31T12:10:00Z".to_string();
        replay.expires_at = "2026-08-31T12:15:00Z".to_string();
        replay.boot_epoch = Some("boot-2".to_string());
        replay.owner_generation = Some(9);
        replay.expected_authority_revision = authority.revision;

        let replayed = authority.acquire_with_receipt(replay).unwrap();

        assert!(replayed.replayed);
        assert!(replayed.claim.is_none());
        assert_eq!(replayed.receipt, first.receipt);
        assert_eq!(authority, after_first);
    }

    #[test]
    fn expired_claim_cannot_authorize_an_effect() {
        let mut authority = LeaseAuthorityState::default();
        let acquired = authority.acquire_with_receipt(request()).unwrap();
        let authorization = acquired
            .claim
            .unwrap()
            .effect_authorization(&capability())
            .unwrap();

        assert!(authority
            .authorize_effect(&authorization, "2026-08-31T12:04:59Z")
            .is_ok());
        assert_eq!(
            authority.authorize_effect(&authorization, "2026-08-31T12:05:00Z"),
            Err(LeaseAuthorityError::ClaimExpired)
        );
    }

    #[test]
    fn same_principal_new_operation_rejoins_current_claim() {
        let mut authority = LeaseAuthorityState::default();
        let first = authority.acquire_with_receipt(request()).unwrap();
        let first_claim = first.claim.unwrap();
        let mut rejoin = request();
        rejoin.expected_authority_revision = authority.revision;
        rejoin.idempotency_key = "acquire:last30days:tick-2".to_string();

        let joined = authority.acquire_with_receipt(rejoin).unwrap();

        let joined_claim = joined.claim.unwrap();
        assert!(!joined.replayed);
        assert_eq!(joined_claim.claim_id(), first_claim.claim_id());
        assert_eq!(joined_claim.fencing_token(), first_claim.fencing_token());
        assert_eq!(joined_claim.expires_at(), first_claim.expires_at());
        assert_eq!(authority.active_claims.len(), 1);
        assert_eq!(authority.acquisition_receipts.len(), 2);
    }

    #[test]
    fn strict_claim_cannot_implicitly_rejoin() {
        let mut authority = LeaseAuthorityState::default();
        let mut strict = request();
        strict.mode = LeaseClaimMode::Strict;
        strict.recovery_controller_id = Some("controller:lease-recovery".to_string());
        strict.transition_deadline = Some("2026-08-31T12:04:00Z".to_string());
        let first = authority.acquire_with_receipt(strict.clone()).unwrap();
        let mut rejoin = strict;
        rejoin.expected_authority_revision = authority.revision;
        rejoin.idempotency_key = "acquire:last30days:strict-tick-2".to_string();

        assert_eq!(
            authority.acquire_with_receipt(rejoin),
            Err(LeaseAuthorityError::ClaimConflict)
        );
        assert_eq!(authority.active_claims.len(), 1);
        assert_eq!(authority.acquisition_receipts.len(), 1);
        assert_eq!(
            authority
                .active_claims
                .values()
                .next()
                .map(ActiveLeaseClaim::claim_id),
            first.claim.as_ref().map(ActiveLeaseClaim::claim_id)
        );
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
                .map(|current| current.claim_id.as_str()),
            Some(claim.claim_id.as_str())
        );
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
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "memory_repository_poisoned".to_string())?;
            mutator(&mut state)
        }
    }

    #[test]
    fn effect_boundary_rejects_diverged_owner_principal_binding() {
        let profile_path = "/tmp/agent-browser-lease-owner-fence";
        let resolved =
            crate::runtime_profile::resolve_profile(Some(profile_path), Some("last30days-social"))
                .unwrap();
        let profile_identity_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&resolved.user_data_dir)
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
        registry.principal_bindings.insert(
            profile_identity_digest.clone(),
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                principal_id: "principal:last30days".to_string(),
                profile_id: "last30days-social".to_string(),
                profile_identity_digest: profile_identity_digest.clone(),
                capability_id: "capability:last30days-social".to_string(),
                provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
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
            service_principals: crate::native::service_principal::ServicePrincipalRegistry {
                profile_capabilities: BTreeMap::from([(
                    "capability:last30days-social".to_string(),
                    capability(),
                )]),
                ..crate::native::service_principal::ServicePrincipalRegistry::default()
            },
            runtime_owner_registry: registry,
            ..ServiceState::default()
        };
        let mut claim_request = request();
        claim_request.owner_generation = Some(7);
        let claim = state.acquire_lease_claim(claim_request).unwrap();
        let authorization = issue_lease_effect_authorization_for_state(&state, &claim).unwrap();
        let repository = MemoryRepository {
            state: Arc::new(Mutex::new(state)),
        };
        authorize_lease_effect_in_repository(&repository, &authorization, NOW).unwrap();

        let mut tampered = authorization.clone();
        tampered.proof.replace_range(..2, "00");
        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &tampered, NOW),
            Err("lease_authority_invalid_effect_proof".to_string())
        );

        repository
            .mutate(|state| {
                state
                    .service_principals
                    .profile_capabilities
                    .get_mut("capability:last30days-social")
                    .unwrap()
                    .state =
                    crate::native::service_principal::ServiceProfileCapabilityState::Revoked;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &authorization, NOW),
            Err("lease_authority_capability_revoked".to_string())
        );

        repository
            .mutate(|state| {
                state
                    .service_principals
                    .profile_capabilities
                    .get_mut("capability:last30days-social")
                    .unwrap()
                    .state =
                    crate::native::service_principal::ServiceProfileCapabilityState::Active;
                state
                    .runtime_owner_registry
                    .principal_bindings
                    .get_mut(&profile_identity_digest)
                    .unwrap()
                    .principal_id = "principal:foreign".to_string();
                Ok(())
            })
            .unwrap();

        assert_eq!(
            authorize_lease_effect_in_repository(&repository, &authorization, NOW),
            Err("lease_authority_owner_generation_stale".to_string())
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
        assert_eq!(state.lease_authority().active_claims.len(), 1);
        assert_eq!(state.lease_authority().events.len(), 1);
    }
}
