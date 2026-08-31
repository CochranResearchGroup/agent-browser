//! Canonical active-claim authority for service-owned resources.
//!
//! Only `active_claims` may authorize or block effects. `events` is retained
//! append-only history and is never consulted for admission.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::service_store::ServiceStateRepository;

pub(crate) const LEASE_AUTHORITY_SCHEMA_VERSION: &str = "agent-browser.lease-authority.v1";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeaseEventKind {
    Acquired,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActiveLeaseClaim {
    pub(crate) schema_version: String,
    pub(crate) claim_id: String,
    pub(crate) resource: LeaseResourceKey,
    pub(crate) parent_claim_id: Option<String>,
    pub(crate) principal_id: String,
    pub(crate) capability_id: String,
    pub(crate) mode: LeaseClaimMode,
    pub(crate) revision: u64,
    pub(crate) fencing_token: u64,
    pub(crate) idempotency_key: String,
    pub(crate) acquired_at: String,
    pub(crate) heartbeat_at: String,
    pub(crate) expires_at: String,
    pub(crate) transition_deadline: Option<String>,
    pub(crate) recovery_controller_id: Option<String>,
    pub(crate) boot_epoch: Option<String>,
    pub(crate) owner_generation: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct LeaseAuthorityState {
    pub(crate) schema_version: String,
    pub(crate) revision: u64,
    pub(crate) active_claims: BTreeMap<String, ActiveLeaseClaim>,
    pub(crate) next_fencing_tokens: BTreeMap<String, u64>,
    pub(crate) events: Vec<LeaseAuthorityEvent>,
}

impl LeaseAuthorityState {
    pub(crate) fn is_empty(&self) -> bool {
        self.active_claims.is_empty()
            && self.next_fencing_tokens.is_empty()
            && self.events.is_empty()
    }

    pub(crate) fn acquire(
        &mut self,
        request: AcquireLeaseClaimRequest,
    ) -> Result<ActiveLeaseClaim, LeaseAuthorityError> {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcquireLeaseClaimRequest {
    pub(crate) resource: LeaseResourceKey,
    pub(crate) parent_claim_id: Option<String>,
    pub(crate) principal_id: String,
    pub(crate) capability_id: String,
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
            .lease_authority
            .acquire(request)
            .map_err(|error| format!("lease_authority_{}", error.as_str()))
    })
}

fn validate_request(request: &AcquireLeaseClaimRequest) -> Result<(), LeaseAuthorityError> {
    if request.resource.id.trim().is_empty()
        || request.principal_id.trim().is_empty()
        || request.capability_id.trim().is_empty()
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
        && claim.mode == request.mode
        && claim.expires_at == request.expires_at
        && claim.transition_deadline == request.transition_deadline
        && claim.recovery_controller_id == request.recovery_controller_id
        && claim.boot_epoch == request.boot_epoch
        && claim.owner_generation == request.owner_generation
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
    fn service_state_round_trips_active_claims_and_history_separately() {
        let mut authority = LeaseAuthorityState::default();
        let claim = authority.acquire(request()).unwrap();
        let state = crate::native::service_model::ServiceState {
            lease_authority: authority.clone(),
            ..crate::native::service_model::ServiceState::default()
        };

        let encoded = serde_json::to_value(&state).unwrap();
        assert_eq!(
            encoded["leaseAuthority"]["activeClaims"]
                .as_object()
                .map(serde_json::Map::len),
            Some(1)
        );
        let decoded: crate::native::service_model::ServiceState =
            serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.lease_authority, authority);
        assert_eq!(
            decoded
                .lease_authority
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
        assert_eq!(state.lease_authority.active_claims.len(), 1);
        assert_eq!(state.lease_authority.events.len(), 1);
    }
}
