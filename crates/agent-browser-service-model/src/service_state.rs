//! Canonical provider-free Service State aggregate and persisted codec.
//!
//! Persistence, locking, process observation, and effect application remain
//! adapter responsibilities. This module deliberately owns only durable state,
//! compatibility decoding, deterministic encoding, and pure transitions.

use crate::*;
use agent_browser_authentication_control::{
    AuthenticationActionFailure, AuthenticationActionKind, AuthenticationActionReceipt,
    AuthenticationChallengeChannel, AuthenticationRunError, AuthenticationVerifier,
    ProviderWatchReceipt, SiteLoginActionReceipt, SiteLoginObservationReceipt,
};
use agent_browser_challenge_control::ChallengeConsumerAdmissionReceipt;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SERVICE_STATE_SCHEMA_VERSION: &str = "agent-browser.service-state.v2";
pub const LEGACY_SERVICE_STATE_SCHEMA_VERSION: &str = "agent-browser.service-state.unversioned";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStateCodecError {
    Json(String),
    UnsupportedStateSchema { found: String },
    UnsupportedProfileLeaseSchema { found: String },
    Invariant(String),
    RevisionExhausted,
}

impl std::fmt::Display for ServiceStateCodecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(message) | Self::Invariant(message) => formatter.write_str(message),
            Self::UnsupportedStateSchema { found } => {
                write!(formatter, "service_state_schema_unsupported:{found}")
            }
            Self::UnsupportedProfileLeaseSchema { found } => {
                write!(formatter, "profile_lease_schema_unsupported:{found}")
            }
            Self::RevisionExhausted => formatter.write_str("service_state_revision_exhausted"),
        }
    }
}

impl std::error::Error for ServiceStateCodecError {}

/// Repository transport with exactly the existing owner-registry wire shape.
/// The retained registry is deliberately inaccessible as a runtime authority handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeOwnerPersistenceSnapshot {
    registry: agent_browser_lease_authority::RuntimeOwnerRegistry,
}

/// Sidecars already selected by the repository's compatibility rules.
/// `None` preserves embedded state; `Some` replaces it, including empty values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeOwnerPersistenceRestore {
    pub owner_registry: Option<RuntimeOwnerPersistenceSnapshot>,
    pub lifecycle_records:
        Option<BTreeMap<String, agent_browser_lease_authority::RuntimeLifecycleRecord>>,
}

/// Owned persistence projections without exposing the live owner registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOwnerPersistenceParts {
    pub owner_registry: RuntimeOwnerPersistenceSnapshot,
    pub lifecycle_registry_revision: u64,
    pub lifecycle_records: BTreeMap<String, agent_browser_lease_authority::RuntimeLifecycleRecord>,
}

/// Exact sealed recovery identity used to match a retained terminal receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileRecoveryReceiptIdentity<'a> {
    pub recovery_id: &'a str,
    pub plan_id: &'a str,
    pub principal_id: &'a str,
    pub profile_id: &'a str,
    pub producer_build_identity: &'a Value,
}

/// Exact sealed reset identity used to match a retained terminal receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileResetReceiptIdentity<'a> {
    pub reset_id: &'a str,
    pub plan_id: &'a str,
    pub principal_id: &'a str,
    pub profile_id: &'a str,
    pub producer_build_identity: &'a Value,
    pub scope: ProfileResetScope,
    pub target_service_id: Option<&'a str>,
}

/// Receipt matching failures; adapters retain their contextual error formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileReceiptReplayError {
    LeaseAuthorityMismatch,
    RecoveryReceiptConflict,
    ResetReceiptConflict,
}

/// Explicit configuration input for constructing the configured Service State
/// overlay. Durable compatibility metadata and runtime-owned records are not
/// accepted through this interface.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfiguredServiceStateInput {
    pub profiles: BTreeMap<String, BrowserProfile>,
    pub sessions: BTreeMap<String, BrowserSession>,
    pub monitors: BTreeMap<String, SiteMonitor>,
    pub site_policies: BTreeMap<String, SitePolicy>,
    pub providers: BTreeMap<String, ServiceProvider>,
    pub browser_capability_registry: crate::BrowserCapabilityRegistry,
    pub default_browser_build: Option<BrowserBuild>,
}

/// Top-level snapshot of the browser service control plane.
///
/// Documented-hidden fields are temporary migration compatibility access.
/// Their direct adapter callers must be removed before P205 field closure.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceState {
    /// Version of the persisted primary Service State envelope. An absent
    /// value is accepted only as the legacy unversioned input format.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[doc(hidden)]
    pub schema_version: String,
    /// Monotonic durable snapshot revision used to reject stale prepared
    /// transactions before any participating file is replaced.
    #[serde(default)]
    #[doc(hidden)]
    pub state_revision: u64,
    /// Version of the first-class profile lease projection understood by the
    /// writer. This is compatibility metadata, not an authority source.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[doc(hidden)]
    pub profile_lease_schema_version: String,
    pub control_plane: Option<ControlPlaneSnapshot>,
    pub reconciliation: Option<ServiceReconciliationSnapshot>,
    pub events: Vec<ServiceEvent>,
    pub incidents: Vec<ServiceIncident>,
    pub display_allocations: BTreeMap<String, DisplayAllocation>,
    pub remote_view_routes: BTreeMap<String, RemoteViewRoute>,
    pub route_pool: BTreeMap<String, RoutePoolEntry>,
    pub remote_view_acquisition_leases: BTreeMap<String, RemoteViewAcquisitionLease>,
    pub remote_view_handoffs: BTreeMap<String, RemoteViewHandoff>,
    pub viewer_leases: BTreeMap<String, ViewerLease>,
    /// Durable scarce presentation-slot inventory and admission authority.
    /// Logical browsers remain separate and survive slot release or parking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[doc(hidden)]
    pub presentation_capacity: Option<crate::PresentationCapacityAuthority>,
    pub profiles: BTreeMap<String, BrowserProfile>,
    /// Durable, redacted result of materializing missing legacy Profile access
    /// policies. Ambiguous legacy identity remains observable but nonblocking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[doc(hidden)]
    pub profile_policy_migration: Option<crate::ProfilePolicyMigrationReport>,
    /// Registered service principals and their hashed profile capabilities.
    /// Raw capability material is never retained in Service State.
    #[serde(
        default,
        skip_serializing_if = "agent_browser_lease_authority::ServicePrincipalRegistry::is_empty"
    )]
    #[doc(hidden)]
    pub service_principals: agent_browser_lease_authority::ServicePrincipalRegistry,
    /// Canonical current service-resource claims and their append-only history.
    /// Only `active_claims` grants authority; terminal events are never blockers.
    #[serde(
        default,
        skip_serializing_if = "agent_browser_lease_authority::LeaseAuthorityState::is_empty"
    )]
    #[doc(hidden)]
    pub lease_authority: agent_browser_lease_authority::LeaseAuthorityState,
    /// Idempotent receipts for applied profile-lease reconciliation plans.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub profile_lease_reconcile_receipts: BTreeMap<String, crate::ProfileLeaseReconcileReceipt>,
    /// Idempotent terminal receipts for sealed profile acquisition recoveries.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub profile_recovery_receipts: BTreeMap<String, crate::RecoveryReceipt>,
    /// Idempotent terminal receipts for sealed, explicitly scoped profile resets.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub profile_reset_receipts: BTreeMap<String, crate::ProfileResetReceipt>,
    /// Exact, permission-backed Profile lifecycle authorizations. These bind
    /// logical eviction intent to a policy revision before a daemon may join
    /// it with fresh physical target evidence.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub profile_lifecycle_authorizations: BTreeMap<String, crate::ProfileLifecycleAuthorization>,
    /// Idempotent minimal receipts for physically proven Profile lifecycle
    /// effects. Page contents, paths, credentials, and bearer material are
    /// never retained here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub profile_lifecycle_effect_receipts: BTreeMap<String, crate::ProfileLifecycleEffectReceipt>,
    /// Idempotent receipts for exact inert browser-record retirement.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub browser_retirement_receipts: BTreeMap<String, crate::BrowserRetirementReceipt>,
    /// Durable reservations and terminal receipts for exact live abandoned
    /// browser retirement. External process observation and signaling remain
    /// outside replayable Service State mutations.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub abandoned_browser_retirements:
        BTreeMap<String, crate::AbandonedBrowserRetirementTransaction>,
    /// Replayable dependency-ordered crash recovery transactions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub crash_regeneration_transactions: BTreeMap<String, crate::CrashRegenerationTransaction>,
    pub browsers: BTreeMap<String, BrowserProcess>,
    /// Non-authoritative protected-owner observations keyed by Service State
    /// browser id. The root authority remains the only mutation gate.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub protected_browser_owner_observations: BTreeMap<String, ProtectedBrowserOwnerObservation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub browser_process_identities: BTreeMap<String, ServiceBrowserProcessIdentity>,
    /// Cross-generation profile-owner authority used by transactional runtime
    /// adoption. The record contains opaque IDs and digests only.
    #[serde(
        default,
        skip_serializing_if = "agent_browser_lease_authority::RuntimeOwnerRegistry::is_empty"
    )]
    #[doc(hidden)]
    pub runtime_owner_registry: agent_browser_lease_authority::RuntimeOwnerRegistry,
    pub sessions: BTreeMap<String, BrowserSession>,
    pub tabs: BTreeMap<String, BrowserTab>,
    pub jobs: BTreeMap<String, ServiceJob>,
    pub monitors: BTreeMap<String, SiteMonitor>,
    pub site_policies: BTreeMap<String, SitePolicy>,
    pub providers: BTreeMap<String, ServiceProvider>,
    pub challenges: BTreeMap<String, Challenge>,
    /// Durable response-only site-authentication runs. These records contain
    /// opaque references and redacted receipts, never credential or challenge
    /// material.
    #[serde(
        default,
        skip_serializing_if = "crate::authentication_run_map_is_empty"
    )]
    #[doc(hidden)]
    pub authentication_runs: BTreeMap<String, crate::ServiceAuthenticationRunRecord>,
    /// Durable provider-free challenge tasks. Records retain exact authority
    /// bindings and redacted receipts, never credentials or captured evidence.
    #[serde(default, skip_serializing_if = "crate::challenge_task_map_is_empty")]
    #[doc(hidden)]
    pub challenge_tasks: BTreeMap<String, crate::ServiceChallengeTaskRecord>,
    pub profile_seeding_handoffs: BTreeMap<String, ProfileSeedingHandoffRecord>,
    #[serde(
        default,
        skip_serializing_if = "crate::BrowserCapabilityRegistry::is_empty"
    )]
    pub browser_capability_registry: crate::BrowserCapabilityRegistry,
    pub default_browser_build: Option<BrowserBuild>,
    /// Forward-compatible top-level fields written by newer Service State
    /// producers. Older writers round-trip these values without interpreting
    /// or granting authority from them.
    #[serde(flatten)]
    #[doc(hidden)]
    pub unknown_fields: BTreeMap<String, Value>,
    #[serde(skip)]
    pub entity_sources: ServiceEntitySources,
}

/// Decode persisted state, retaining legacy policy materialization and schema checks.
/// Transport values deliberately do not use this persisted compatibility path.
pub fn decode_persisted_service_state_json(
    raw: &str,
) -> Result<ServiceState, ServiceStateCodecError> {
    let value = serde_json::from_str(raw)
        .map_err(|error| ServiceStateCodecError::Json(error.to_string()))?;
    decode_persisted_service_state_value(value)
}

fn decode_persisted_service_state_value(
    value: Value,
) -> Result<ServiceState, ServiceStateCodecError> {
    if let Some(schema) = value.get("schemaVersion").and_then(Value::as_str) {
        if schema != SERVICE_STATE_SCHEMA_VERSION {
            return Err(ServiceStateCodecError::UnsupportedStateSchema {
                found: schema.to_string(),
            });
        }
    }
    if let Some(schema) = value
        .get("profileLeaseSchemaVersion")
        .and_then(Value::as_str)
    {
        if schema != PROFILE_LEASE_SCHEMA_VERSION {
            return Err(ServiceStateCodecError::UnsupportedProfileLeaseSchema {
                found: schema.to_string(),
            });
        }
    }
    let mut state = serde_json::from_value(value)
        .map_err(|error| ServiceStateCodecError::Json(error.to_string()))?;
    prepare_service_state_for_persistence(&mut state)?;
    Ok(state)
}

/// Materialize legacy profile policy and stamp versions without observation or I/O.
pub fn prepare_service_state_for_persistence(
    state: &mut ServiceState,
) -> Result<(), ServiceStateCodecError> {
    materialize_legacy_profile_access_policies(state);
    state.schema_version = SERVICE_STATE_SCHEMA_VERSION.to_string();
    state.profile_lease_schema_version = PROFILE_LEASE_SCHEMA_VERSION.to_string();
    Ok(())
}

/// Encode the exact prepared snapshot with one trailing newline and no derivation.
pub fn encode_prepared_service_state_pretty(
    state: &ServiceState,
) -> Result<Vec<u8>, ServiceStateCodecError> {
    let mut bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| ServiceStateCodecError::Json(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Keep runtime freshness evidence writable when a configured profile supplies
/// the static routing policy for the same profile ID.
fn overlay_persisted_profile_freshness(
    configured: &mut BrowserProfile,
    persisted: &BrowserProfile,
) {
    for target_service_id in &persisted.target_service_ids {
        if !configured.target_service_ids.contains(target_service_id) {
            configured
                .target_service_ids
                .push(target_service_id.clone());
        }
    }
    for account_id in &persisted.account_ids {
        if !configured.account_ids.contains(account_id) {
            configured.account_ids.push(account_id.clone());
        }
    }

    for readiness in &persisted.target_readiness {
        let target_service_id = &readiness.target_service_id;
        if let Some(existing) = configured
            .target_readiness
            .iter_mut()
            .find(|existing| existing.target_service_id == *target_service_id)
        {
            *existing = readiness.clone();
        } else {
            configured.target_readiness.push(readiness.clone());
        }

        configured
            .authenticated_service_ids
            .retain(|existing| existing != target_service_id);
        if persisted
            .authenticated_service_ids
            .contains(target_service_id)
        {
            configured
                .authenticated_service_ids
                .push(target_service_id.clone());
        }
    }
}

impl ServiceState {
    /// Replay by idempotency key and principal only. The adapter checks current
    /// authority before this lookup; seal, expiry, and CAS checks follow a miss.
    /// Only the returned clone receives the replay marker.
    pub fn replay_profile_lease_reconciliation(
        &self,
        idempotency_key: &str,
        principal_id: &str,
    ) -> Result<Option<ProfileLeaseReconcileReceipt>, ProfileReceiptReplayError> {
        let Some(receipt) = self.profile_lease_reconcile_receipts.get(idempotency_key) else {
            return Ok(None);
        };
        if receipt.principal_id != principal_id {
            return Err(ProfileReceiptReplayError::LeaseAuthorityMismatch);
        }
        let mut replay = receipt.clone();
        replay.replayed = true;
        Ok(Some(replay))
    }

    /// Record after the adapter's guarded transitions, deriving the map key
    /// from the receipt and preserving replacement semantics. This does not
    /// advance a revision or perform replay, authority, or persistence checks.
    pub fn record_profile_lease_reconciliation(&mut self, receipt: ProfileLeaseReconcileReceipt) {
        self.profile_lease_reconcile_receipts
            .insert(receipt.idempotency_key.clone(), receipt);
    }

    /// Inspect one recovery receipt; status authentication remains in the adapter.
    pub fn profile_recovery_receipt(&self, recovery_id: &str) -> Option<&RecoveryReceipt> {
        self.profile_recovery_receipts.get(recovery_id)
    }

    /// Match the existing recovery replay identity without checking current
    /// owner state, plan expiry, or authority. Adapters verify the seal first.
    pub fn replay_profile_recovery(
        &self,
        expected: ProfileRecoveryReceiptIdentity<'_>,
    ) -> Result<Option<RecoveryReceipt>, ProfileReceiptReplayError> {
        let Some(receipt) = self.profile_recovery_receipts.get(expected.recovery_id) else {
            return Ok(None);
        };
        if receipt.plan_id != expected.plan_id
            || receipt.recovery_id != expected.recovery_id
            || receipt.principal_id != expected.principal_id
            || receipt.profile_id != expected.profile_id
            || receipt.producer_build_identity.as_ref() != Some(expected.producer_build_identity)
            || receipt.terminal_result != "applied"
        {
            return Err(ProfileReceiptReplayError::RecoveryReceiptConflict);
        }
        Ok(Some(receipt.clone()))
    }

    /// Record after recovery postconditions and reference repair, deriving the
    /// key from the receipt and preserving replacement semantics. Replay checks,
    /// revision selection, acquisition effects, and persistence remain in adapters.
    pub fn record_profile_recovery(&mut self, receipt: RecoveryReceipt) {
        self.profile_recovery_receipts
            .insert(receipt.recovery_id.clone(), receipt);
    }

    /// Match the existing reset replay identity without checking current
    /// owner state, plan expiry, or authority. Adapters verify the seal first.
    pub fn replay_profile_reset(
        &self,
        expected: ProfileResetReceiptIdentity<'_>,
    ) -> Result<Option<ProfileResetReceipt>, ProfileReceiptReplayError> {
        let Some(receipt) = self.profile_reset_receipts.get(expected.reset_id) else {
            return Ok(None);
        };
        if receipt.plan_id != expected.plan_id
            || receipt.reset_id != expected.reset_id
            || receipt.principal_id != expected.principal_id
            || receipt.profile_id != expected.profile_id
            || &receipt.producer_build_identity != expected.producer_build_identity
            || receipt.scope != expected.scope
            || receipt.target_service_id.as_deref() != expected.target_service_id
            || receipt.terminal_result != "applied"
        {
            return Err(ProfileReceiptReplayError::ResetReceiptConflict);
        }
        Ok(Some(receipt.clone()))
    }

    /// Record after the adapter's scoped reset mutations, deriving the key from
    /// the receipt and preserving replacement semantics. This does not perform
    /// effects, validate replay identity, or change the supplied revision.
    pub fn record_profile_reset(&mut self, receipt: ProfileResetReceipt) {
        self.profile_reset_receipts
            .insert(receipt.reset_id.clone(), receipt);
    }

    /// Inspect one crash transaction without granting mutable record access.
    pub fn crash_regeneration_transaction(
        &self,
        id: &str,
    ) -> Option<&CrashRegenerationTransaction> {
        self.crash_regeneration_transactions.get(id)
    }

    /// Project crash status in deterministic transaction-map order.
    pub fn crash_regeneration_statuses(&self) -> Vec<CrashRegenerationStatus> {
        crate::crash_regeneration::crash_regeneration_statuses(
            &self.crash_regeneration_transactions,
        )
    }

    /// Insert or replay a transaction only after its identity checks succeed.
    pub fn begin_or_resume_crash_regeneration(
        &mut self,
        request: &CrashRegenerationRequest,
    ) -> Result<CrashRegenerationTransaction, String> {
        let transaction = crate::crash_regeneration::begin_or_resume(
            self.crash_regeneration_transactions
                .get(&request.transaction_id),
            request,
        )?;
        self.crash_regeneration_transactions
            .insert(transaction.transaction_id.clone(), transaction.clone());
        Ok(transaction)
    }

    /// Compare the persisted identity and phase before applying a validated receipt.
    pub fn apply_crash_regeneration_phase(
        &mut self,
        expected: &CrashRegenerationTransaction,
        phase: CrashRegenerationPhase,
        receipt: CrashRegenerationPhaseReceipt,
    ) -> Result<CrashRegenerationTransaction, String> {
        let current =
            self.crash_regeneration_transaction_for_transition(&expected.transaction_id)?;
        if current.revision != expected.revision
            || current.boot_epoch != expected.boot_epoch
            || current.stable_identities != expected.stable_identities
            || crate::crash_regeneration::next_phase(current) != Some(phase)
        {
            return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
        }
        let updated = crate::crash_regeneration::apply_phase_receipt(expected, phase, receipt)?;
        *current = updated.clone();
        Ok(updated)
    }

    /// Persist an interruption only when the revision and pending phase still match.
    pub fn interrupt_crash_regeneration(
        &mut self,
        expected: &CrashRegenerationTransaction,
        phase: CrashRegenerationPhase,
        error: &str,
    ) -> Result<CrashRegenerationTransaction, String> {
        let current =
            self.crash_regeneration_transaction_for_transition(&expected.transaction_id)?;
        if current.revision != expected.revision
            || crate::crash_regeneration::next_phase(current) != Some(phase)
        {
            return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
        }
        let updated = crate::crash_regeneration::interrupt(expected, phase, error)?;
        *current = updated.clone();
        Ok(updated)
    }

    /// Finish a persisted transaction after revision and phase-completion checks.
    pub fn finish_crash_regeneration(
        &mut self,
        expected: &CrashRegenerationTransaction,
    ) -> Result<CrashRegenerationTransaction, String> {
        let current =
            self.crash_regeneration_transaction_for_transition(&expected.transaction_id)?;
        if current.revision != expected.revision
            || crate::crash_regeneration::next_phase(current).is_some()
        {
            return Err("crash_regeneration_compare_and_swap_mismatch".to_string());
        }
        let updated = crate::crash_regeneration::finish_ready(expected)?;
        *current = updated.clone();
        Ok(updated)
    }

    /// Clone the aggregate without private crash evidence for public projection.
    pub fn without_crash_regeneration_transactions(&self) -> Self {
        let mut state = self.clone();
        state.crash_regeneration_transactions.clear();
        state
    }

    fn crash_regeneration_transaction_for_transition(
        &mut self,
        id: &str,
    ) -> Result<&mut CrashRegenerationTransaction, String> {
        self.crash_regeneration_transactions
            .get_mut(id)
            .ok_or_else(|| "crash_regeneration_transaction_missing".to_string())
    }

    /// Inspect an authentication run without granting mutable record access.
    pub fn service_authentication_run(&self, id: &str) -> Option<&ServiceAuthenticationRunRecord> {
        self.authentication_runs.get(id)
    }

    /// Resolve replay before the adapter performs external challenge admission.
    pub fn prepare_service_authentication_run_start(
        &self,
        input: ServiceAuthenticationRunStartInput,
    ) -> Result<ServiceAuthenticationRunStartDecision, ServiceAuthenticationRunError> {
        prepare_service_authentication_run_start(&self.authentication_runs, input)
    }

    /// Insert only after all existing start-completion checks have succeeded.
    pub fn complete_service_authentication_run_start(
        &mut self,
        prepared: PreparedServiceAuthenticationRunStart,
        challenge_admission: Option<ChallengeConsumerAdmissionReceipt>,
    ) -> Result<ServiceAuthenticationRunRecord, ServiceAuthenticationRunError> {
        let record = complete_service_authentication_run_start(prepared, challenge_admission)?;
        self.authentication_runs
            .insert(record.run.run_id.clone(), record.clone());
        Ok(record)
    }

    /// Reserve the exact effect using adapter-supplied identity and timestamps.
    pub fn reserve_service_authentication_effect(
        &mut self,
        run_id: &str,
        operation_id: &str,
        action: AuthenticationActionKind,
        state_instance_id: &str,
        reserved_at: &str,
        observed_at: &str,
    ) -> Result<ServiceAuthenticationRunRecord, ServiceAuthenticationRunStateError> {
        let record = self.authentication_run_for_transition(run_id)?;
        reserve_authentication_effect(
            record,
            operation_id,
            action,
            state_instance_id,
            reserved_at,
            observed_at,
        )?;
        Ok(record.clone())
    }

    /// Check liveness before applying the observation; retain any failed
    /// transition's mutations in the aggregate.
    pub fn observe_service_authentication_run(
        &mut self,
        run_id: &str,
        operation_id: &str,
        receipt: SiteLoginObservationReceipt,
        observed_at: &str,
    ) -> Result<ServiceAuthenticationRunRecord, ServiceAuthenticationRunStateError> {
        let record = self.authentication_run_for_transition(run_id)?;
        require_live_authentication_run(record, observed_at)?;
        record.run.observe_site_login_state(operation_id, receipt)?;
        Ok(record.clone())
    }

    /// Check liveness before accepting an already-observed provider watch.
    pub fn prepare_service_authentication_watch(
        &mut self,
        run_id: &str,
        operation_id: &str,
        challenge_id: &str,
        channel: AuthenticationChallengeChannel,
        watch: ProviderWatchReceipt,
        observed_at: &str,
    ) -> Result<ServiceAuthenticationRunRecord, ServiceAuthenticationRunStateError> {
        let record = self.authentication_run_for_transition(run_id)?;
        require_live_authentication_run(record, observed_at)?;
        record
            .run
            .prepare_watch(operation_id, challenge_id, channel, watch)?;
        Ok(record.clone())
    }

    /// Return the post-transition record even when verification failed, allowing
    /// the adapter to persist that outcome before reporting its contextual error.
    pub fn verify_service_authentication_run(
        &mut self,
        run_id: &str,
        operation_id: &str,
        verifier: &mut impl AuthenticationVerifier,
    ) -> Result<
        (
            ServiceAuthenticationRunRecord,
            Option<AuthenticationRunError>,
        ),
        ServiceAuthenticationRunStateError,
    > {
        let record = self.authentication_run_for_transition(run_id)?;
        let result = record.run.verify_exact_target(operation_id, verifier);
        Ok((record.clone(), result.err()))
    }

    /// Consume the matching pending fence and retain the site transition outcome.
    pub fn complete_service_authentication_site_action(
        &mut self,
        run_id: &str,
        operation_id: &str,
        expected_action: AuthenticationActionKind,
        outcome: Result<SiteLoginActionReceipt, AuthenticationActionFailure>,
    ) -> Result<
        (
            ServiceAuthenticationRunRecord,
            ServiceAuthenticationRunCompletion,
        ),
        ServiceAuthenticationRunStateError,
    > {
        let record = self.authentication_run_for_transition(run_id)?;
        let completion =
            complete_site_authentication_action(record, operation_id, expected_action, outcome)?;
        Ok((record.clone(), completion))
    }

    /// Consume the matching credential-delivery fence and retain transition failures.
    pub fn complete_service_authentication_delivery_action(
        &mut self,
        run_id: &str,
        operation_id: &str,
        challenge_id: &str,
        outcome: Result<AuthenticationActionReceipt, AuthenticationActionFailure>,
    ) -> Result<
        (
            ServiceAuthenticationRunRecord,
            ServiceAuthenticationRunCompletion,
        ),
        ServiceAuthenticationRunStateError,
    > {
        let record = self.authentication_run_for_transition(run_id)?;
        let completion =
            complete_credential_delivery_action(record, operation_id, challenge_id, outcome)?;
        Ok((record.clone(), completion))
    }

    /// Consume the matching challenge fence and retain transition failures.
    pub fn complete_service_authentication_challenge_action(
        &mut self,
        run_id: &str,
        operation_id: &str,
        challenge_id: &str,
        outcome: Result<AuthenticationActionReceipt, AuthenticationActionFailure>,
    ) -> Result<
        (
            ServiceAuthenticationRunRecord,
            ServiceAuthenticationRunCompletion,
        ),
        ServiceAuthenticationRunStateError,
    > {
        let record = self.authentication_run_for_transition(run_id)?;
        let completion =
            complete_challenge_authentication_action(record, operation_id, challenge_id, outcome)?;
        Ok((record.clone(), completion))
    }

    /// Cancel through the canonical transition without introducing a liveness check.
    pub fn cancel_service_authentication_run(
        &mut self,
        run_id: &str,
        operation_id: &str,
    ) -> Result<ServiceAuthenticationRunRecord, ServiceAuthenticationRunStateError> {
        let record = self.authentication_run_for_transition(run_id)?;
        cancel_authentication_run(record, operation_id)?;
        Ok(record.clone())
    }

    fn authentication_run_for_transition(
        &mut self,
        run_id: &str,
    ) -> Result<&mut ServiceAuthenticationRunRecord, ServiceAuthenticationRunStateError> {
        self.authentication_runs
            .get_mut(run_id)
            .ok_or(ServiceAuthenticationRunStateError::NotFound)
    }

    pub fn from_configured_entities(input: ConfiguredServiceStateInput) -> Self {
        let mut state = Self {
            profiles: input.profiles,
            sessions: input.sessions,
            monitors: input.monitors,
            site_policies: input.site_policies,
            providers: input.providers,
            browser_capability_registry: input.browser_capability_registry,
            default_browser_build: input.default_browser_build,
            ..Self::default()
        };
        state.mark_config_entity_sources();
        state
    }

    pub fn state_revision(&self) -> u64 {
        self.state_revision
    }
    /// Return a fully cloned next-revision candidate before an adapter applies
    /// any mutation. Persistence adapters retain their own CAS and locks.
    pub fn checked_successor(&self) -> Result<Self, ServiceStateCodecError> {
        let mut successor = self.clone();
        successor.state_revision = self
            .state_revision
            .checked_add(1)
            .ok_or(ServiceStateCodecError::RevisionExhausted)?;
        Ok(successor)
    }
    /// Compare snapshots while ignoring only the revision fence, retaining
    /// ephemeral entity-source equality just as the repository no-op check does.
    pub fn payload_eq_ignoring_revision(&self, other: &Self) -> bool {
        let mut left = self.clone();
        let mut right = other.clone();
        left.state_revision = 0;
        right.state_revision = 0;
        left == right
    }
    pub fn unknown_top_level_field_names(&self) -> Vec<String> {
        self.unknown_fields.keys().cloned().collect()
    }

    /// Returns the immutable profile-policy migration compatibility projection.
    pub fn profile_policy_migration(&self) -> Option<&crate::ProfilePolicyMigrationReport> {
        self.profile_policy_migration.as_ref()
    }

    /// Restore selected owner state before selected lifecycle evidence. Historical
    /// lifecycle-sidecar revisions do not participate in owner authority restoration.
    /// Selection, envelope parsing, and filesystem custody remain in the repository.
    pub fn restore_runtime_owner_persistence(&mut self, input: RuntimeOwnerPersistenceRestore) {
        if let Some(snapshot) = input.owner_registry {
            self.runtime_owner_registry = snapshot.registry;
        }
        if let Some(records) = input.lifecycle_records {
            self.runtime_owner_registry
                .restore_lifecycle_records(records);
        }
    }

    /// Project the legacy owner payload and separate lifecycle evidence while
    /// preserving the source aggregate and its revisions. The repository owns
    /// serialization order and strips its prepared clone only after lifecycle encoding.
    pub fn runtime_owner_persistence_parts(&self) -> RuntimeOwnerPersistenceParts {
        RuntimeOwnerPersistenceParts {
            owner_registry: RuntimeOwnerPersistenceSnapshot {
                registry: self
                    .runtime_owner_registry
                    .persistence_projection_without_lifecycle_records(),
            },
            lifecycle_registry_revision: self.runtime_owner_registry.revision(),
            lifecycle_records: self.runtime_owner_registry.lifecycle_records().clone(),
        }
    }

    /// Remove only lifecycle evidence from a repository-prepared clone. Owner
    /// authority, principal bindings, and both registry and envelope revisions remain intact.
    pub fn strip_runtime_lifecycle_for_persistence(&mut self) {
        self.runtime_owner_registry = self
            .runtime_owner_registry
            .persistence_projection_without_lifecycle_records();
    }

    /// Authenticate retained session work using its explicit observation time.
    pub fn authenticated_session_work_authority(
        &self,
        session_id: &str,
        now: &str,
    ) -> Option<agent_browser_lease_authority::AuthenticatedServicePrincipal> {
        crate::principal_continuity::authenticated_session_work_authority(self, session_id, now)
    }

    /// Derive continuity recourse without changing owner or subordinate work state.
    pub fn principal_continuity_decision(
        &self,
        authority: &agent_browser_lease_authority::AuthenticatedServicePrincipal,
    ) -> PrincipalContinuityDecision {
        crate::principal_continuity::principal_continuity_decision(self, authority)
    }

    /// Project ordered legacy migration decisions without promoting labels to authority.
    pub fn plan_legacy_session_principal_migration(
        &self,
    ) -> Vec<LegacySessionPrincipalMigrationPlan> {
        crate::principal_continuity::plan_legacy_session_principal_migration(self)
    }

    /// Bind one subordinate session using an externally observed boot epoch.
    /// Owner authority, registry revisions, and the envelope revision are unchanged.
    pub fn bind_session_work_lease(
        &mut self,
        session_id: &str,
        authority: &agent_browser_lease_authority::AuthenticatedServicePrincipal,
        expires_at: String,
        boot_epoch: Option<String>,
    ) -> Result<BrowserSession, agent_browser_lease_authority::ServicePrincipalError> {
        crate::principal_continuity::bind_session_work_lease(
            self, session_id, authority, expires_at, boot_epoch,
        )
    }

    /// Bind one subordinate tab after checking its owner's principal and profile.
    pub fn bind_tab_work_lease(
        &mut self,
        tab_id: &str,
        authority: &agent_browser_lease_authority::AuthenticatedServicePrincipal,
        expires_at: String,
    ) -> Result<BrowserTab, agent_browser_lease_authority::ServicePrincipalError> {
        crate::principal_continuity::bind_tab_work_lease(self, tab_id, authority, expires_at)
    }

    /// Project the principal registry CAS revision without advancing it.
    pub fn service_principal_registry_revision(&self) -> u64 {
        self.service_principals.revision
    }

    /// Inspect one registration without filtering state or interpreting provenance.
    pub fn service_principal(
        &self,
        principal_id: &str,
    ) -> Option<&agent_browser_lease_authority::ServicePrincipalRegistration> {
        self.service_principals.principals.get(principal_id)
    }

    /// Inspect one capability without granting mutable authority or filtering state.
    pub fn profile_capability(
        &self,
        capability_id: &str,
    ) -> Option<&agent_browser_lease_authority::ServiceProfileCapability> {
        self.service_principals
            .profile_capabilities
            .get(capability_id)
    }

    /// Project all capabilities, including revoked records, in persisted map-key order.
    pub fn profile_capabilities(
        &self,
    ) -> impl Iterator<Item = &agent_browser_lease_authority::ServiceProfileCapability> + '_ {
        self.service_principals.profile_capabilities.values()
    }

    /// Authenticate against this snapshot using the kernel's existing error precedence.
    pub fn authenticate_profile_capability(
        &self,
        raw_capability: &str,
        expected_profile_id: Option<&str>,
    ) -> Result<
        agent_browser_lease_authority::AuthenticatedServicePrincipal,
        agent_browser_lease_authority::ServicePrincipalError,
    > {
        agent_browser_lease_authority::authenticate_profile_capability(
            &self.service_principals,
            raw_capability,
            expected_profile_id,
        )
    }

    /// Check the stronger current-authority predicate, including exact revision
    /// and registered provenance, without changing token-authentication semantics.
    pub fn authenticated_authority_is_current(
        &self,
        authority: &agent_browser_lease_authority::AuthenticatedServicePrincipal,
    ) -> bool {
        agent_browser_lease_authority::authenticated_authority_is_current(
            &self.service_principals,
            authority,
        )
    }

    /// Register through the staged principal kernel. Owner binding, capability
    /// files, events, envelope revision, and persistence remain adapter concerns.
    pub fn register_profile_capability(
        &mut self,
        request: agent_browser_lease_authority::ServicePrincipalRegistrationRequest,
        raw_capability: &str,
    ) -> Result<
        agent_browser_lease_authority::RegisteredProfileCapability,
        agent_browser_lease_authority::ServicePrincipalError,
    > {
        agent_browser_lease_authority::register_profile_capability(
            &mut self.service_principals,
            request,
            raw_capability,
        )
    }

    /// Rotate the exact capability under the existing registry CAS. The adapter
    /// fences active work first and updates runtime-owner binding afterward.
    pub fn rotate_profile_capability(
        &mut self,
        request: agent_browser_lease_authority::ServicePrincipalRegistrationRequest,
        expected_capability_id: &str,
        expected_registry_revision: u64,
        raw_capability: &str,
    ) -> Result<
        agent_browser_lease_authority::RotatedProfileCapability,
        agent_browser_lease_authority::ServicePrincipalError,
    > {
        agent_browser_lease_authority::rotate_profile_capability(
            &mut self.service_principals,
            request,
            expected_capability_id,
            expected_registry_revision,
            raw_capability,
        )
    }

    /// Join both authorities from this snapshot without loading signing keys or
    /// performing effects. Authenticated operations remain adapter responsibilities.
    pub fn lease_authority_view(&self) -> agent_browser_lease_authority::LeaseAuthorityView<'_> {
        agent_browser_lease_authority::LeaseAuthorityView::new(
            &self.lease_authority,
            &self.service_principals,
        )
    }

    /// Returns the immutable canonical lease authority projection. Mutations
    /// stay behind the authority kernel so sibling subsystems cannot edit its
    /// active index, fencing counters, or history independently.
    pub fn lease_authority(&self) -> &agent_browser_lease_authority::LeaseAuthorityState {
        &self.lease_authority
    }

    /// Return the current unexpired claim without exposing mutable authority.
    pub fn current_lease_claim(
        &self,
        resource: &agent_browser_lease_authority::LeaseResourceKey,
        now: &str,
    ) -> Option<&agent_browser_lease_authority::ActiveLeaseClaim> {
        self.lease_authority.current_claim(resource, now)
    }

    /// Read an existing release receipt without loading verification keys.
    pub fn replay_lease_claim_release(
        &self,
        request: &agent_browser_lease_authority::ReleaseLeaseClaimRequest,
    ) -> Result<
        Option<agent_browser_lease_authority::LeaseClaimReleaseOutcome>,
        agent_browser_lease_authority::LeaseAuthorityError,
    > {
        self.lease_authority.replay_release(request)
    }

    /// Read an existing recovery receipt without loading verification keys.
    pub fn replay_lease_claim_recovery(
        &self,
        request: &agent_browser_lease_authority::RecoverLeaseClaimRequest,
    ) -> Result<
        Option<agent_browser_lease_authority::LeaseClaimRecoveryOutcome>,
        agent_browser_lease_authority::LeaseAuthorityError,
    > {
        self.lease_authority.replay_recovery(request)
    }

    /// Read an existing revocation receipt without loading verification keys.
    pub fn replay_lease_claim_revocation(
        &self,
        request: &agent_browser_lease_authority::RevokeLeaseClaimRequest,
    ) -> Result<
        Option<agent_browser_lease_authority::LeaseClaimRevocationOutcome>,
        agent_browser_lease_authority::LeaseAuthorityError,
    > {
        self.lease_authority.replay_revocation(request)
    }

    /// Release through the authority kernel, preserving replay before key loading
    /// and binding holder capabilities to this snapshot's principal registry.
    pub fn release_lease_claim(
        &mut self,
        request: agent_browser_lease_authority::ReleaseLeaseClaimRequest,
    ) -> Result<agent_browser_lease_authority::LeaseClaimReleaseOutcome, String> {
        agent_browser_lease_authority::release_lease_claim(
            &mut self.lease_authority,
            &self.service_principals,
            request,
        )
    }

    /// Recover through the authority kernel, preserving replay before key loading
    /// and binding the recovery controller to this snapshot's principal registry.
    pub fn recover_lease_claim(
        &mut self,
        request: agent_browser_lease_authority::RecoverLeaseClaimRequest,
    ) -> Result<agent_browser_lease_authority::LeaseClaimRecoveryOutcome, String> {
        agent_browser_lease_authority::recover_lease_claim(
            &mut self.lease_authority,
            &self.service_principals,
            request,
        )
    }

    /// Revoke through the authority kernel, preserving replay before key loading
    /// and the existing administrative verification and terminal receipt rules.
    pub fn revoke_lease_claim(
        &mut self,
        request: agent_browser_lease_authority::RevokeLeaseClaimRequest,
    ) -> Result<agent_browser_lease_authority::LeaseClaimRevocationOutcome, String> {
        agent_browser_lease_authority::revoke_lease_claim(&mut self.lease_authority, request)
    }

    pub fn acquire_lease_claim(
        &mut self,
        request: agent_browser_lease_authority::AcquireLeaseClaimRequest,
    ) -> Result<
        agent_browser_lease_authority::ActiveLeaseClaim,
        agent_browser_lease_authority::LeaseAuthorityError,
    > {
        if blocks_profile_claim(&self.abandoned_browser_retirements, &request.resource) {
            return Err(agent_browser_lease_authority::LeaseAuthorityError::ClaimUnavailable);
        }
        self.lease_authority.acquire(request)
    }

    pub fn acquire_lease_claim_with_receipt(
        &mut self,
        request: agent_browser_lease_authority::AcquireLeaseClaimRequest,
    ) -> Result<
        agent_browser_lease_authority::LeaseClaimAcquisitionOutcome,
        agent_browser_lease_authority::LeaseAuthorityError,
    > {
        if blocks_profile_claim(&self.abandoned_browser_retirements, &request.resource) {
            return Err(agent_browser_lease_authority::LeaseAuthorityError::ClaimUnavailable);
        }
        self.lease_authority.acquire_with_receipt(request)
    }

    pub fn mark_persisted_entity_sources(&mut self) {
        for id in self.profiles.keys() {
            self.entity_sources
                .profiles
                .entry(id.clone())
                .or_insert(ServiceEntitySource::PersistedState);
        }
        for id in self.site_policies.keys() {
            self.entity_sources
                .site_policies
                .entry(id.clone())
                .or_insert(ServiceEntitySource::PersistedState);
        }
    }

    pub fn mark_config_entity_sources(&mut self) {
        for id in self.profiles.keys() {
            self.entity_sources
                .profiles
                .insert(id.clone(), ServiceEntitySource::Config);
        }
        for id in self.site_policies.keys() {
            self.entity_sources
                .site_policies
                .insert(id.clone(), ServiceEntitySource::Config);
        }
    }

    pub fn profile_source(&self, id: &str) -> Option<ServiceEntitySource> {
        self.entity_sources.profiles.get(id).copied()
    }

    pub fn mark_runtime_observed_profile_source(&mut self, id: &str) {
        self.entity_sources
            .profiles
            .entry(id.to_string())
            .or_insert(ServiceEntitySource::RuntimeObserved);
    }

    pub fn site_policy_source(&self, id: &str) -> Option<ServiceEntitySource> {
        self.entity_sources.site_policies.get(id).copied()
    }

    pub fn remove_builtin_entity_defaults_for_persistence(&mut self) {
        let builtin_ids = self
            .entity_sources
            .site_policies
            .iter()
            .filter(|(_id, source)| **source == ServiceEntitySource::Builtin)
            .map(|(id, _source)| id.clone())
            .collect::<Vec<_>>();
        for id in builtin_ids {
            self.site_policies.remove(&id);
            self.entity_sources.site_policies.remove(&id);
        }
    }

    pub fn overlay_configured_entities(&mut self, configured: ServiceState) {
        let mut configured = configured;
        configured.mark_config_entity_sources();
        let live_browser_ids = self.browsers.keys().cloned().collect::<BTreeSet<_>>();
        let active_route_ids = self
            .remote_view_routes
            .iter()
            .filter(|(_, route)| {
                route.state == "ready"
                    && route
                        .browser_id
                        .as_ref()
                        .is_some_and(|browser_id| live_browser_ids.contains(browser_id))
            })
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        for (id, mut display) in configured.display_allocations {
            if let Some(existing) = self.display_allocations.get(&id) {
                if existing
                    .owner_browser_id
                    .as_ref()
                    .is_some_and(|browser_id| live_browser_ids.contains(browser_id))
                {
                    display.owner_browser_id = existing.owner_browser_id.clone();
                    display.owner_session_id = existing.owner_session_id.clone();
                    display.profile_id = existing.profile_id.clone();
                    display.browser_build = existing.browser_build.clone();
                    display.pid_hints = existing.pid_hints.clone();
                    display.created_at = existing.created_at.clone();
                    display.updated_at = existing.updated_at.clone();
                    display.last_health_check_at = existing.last_health_check_at.clone();
                    display.readiness = existing.readiness.clone();
                    for route_id in &existing.route_ids {
                        if !display.route_ids.contains(route_id) {
                            display.route_ids.push(route_id.clone());
                        }
                    }
                }
            }
            self.display_allocations.insert(id, display);
        }
        for (id, mut route) in configured.remote_view_routes {
            if let Some(existing) = self.remote_view_routes.get(&id) {
                if active_route_ids.contains(&id) {
                    route.browser_id = existing.browser_id.clone();
                    route.session_id = existing.session_id.clone();
                    route.route_source = existing.route_source.clone();
                    route.viewer_lease_ids = existing.viewer_lease_ids.clone();
                    route.controller_lease_id = existing.controller_lease_id.clone();
                    route.controller_epoch = existing.controller_epoch;
                    route.last_provider_event = existing.last_provider_event.clone();
                    route.readiness = existing.readiness.clone();
                }
            }
            self.remote_view_routes.insert(id, route);
        }
        for (id, mut entry) in configured.route_pool {
            if let Some(existing) = self.route_pool.get(&id) {
                if existing.state == "checked_out" && active_route_ids.contains(&existing.route_id)
                {
                    entry.state = existing.state.clone();
                    entry.current_route_allocation_id =
                        existing.current_route_allocation_id.clone();
                    entry.readiness = existing.readiness.clone();
                }
            }
            self.route_pool.insert(id, entry);
        }
        self.viewer_leases.extend(configured.viewer_leases);
        for (id, mut profile) in configured.profiles {
            if let Some(persisted) = self.profiles.get(&id) {
                overlay_persisted_profile_freshness(&mut profile, persisted);
            }
            self.profiles.insert(id.clone(), profile);
            self.entity_sources
                .profiles
                .insert(id, ServiceEntitySource::Config);
        }
        self.sessions.extend(configured.sessions);
        for (id, monitor) in configured.monitors {
            let mut monitor = monitor;
            if let Some(existing) = self.monitors.get(&id) {
                if monitor.last_checked_at.is_none() {
                    monitor.last_checked_at = existing.last_checked_at.clone();
                }
                if monitor.last_succeeded_at.is_none() {
                    monitor.last_succeeded_at = existing.last_succeeded_at.clone();
                }
                if monitor.last_failed_at.is_none() {
                    monitor.last_failed_at = existing.last_failed_at.clone();
                }
                if monitor.last_result.is_none() {
                    monitor.last_result = existing.last_result.clone();
                }
                if monitor.consecutive_failures == 0 && existing.consecutive_failures > 0 {
                    monitor.consecutive_failures = existing.consecutive_failures;
                }
                if monitor.state == MonitorState::Active && existing.state == MonitorState::Faulted
                {
                    monitor.state = MonitorState::Faulted;
                }
            }
            self.monitors.insert(id, monitor);
        }
        for (id, policy) in configured.site_policies {
            self.site_policies.insert(id.clone(), policy);
            self.entity_sources
                .site_policies
                .insert(id, ServiceEntitySource::Config);
        }
        self.providers.extend(configured.providers);
        if !configured.browser_capability_registry.is_empty() {
            self.browser_capability_registry = configured.browser_capability_registry;
        }
        if configured.default_browser_build.is_some() {
            self.default_browser_build = configured.default_browser_build;
        }
    }

    /// Add shipped site-policy defaults without overriding local policy.
    pub fn apply_builtin_site_policies(&mut self) {
        for policy in builtin_site_policies() {
            let id = policy.id.clone();
            if self.site_policies.contains_key(&id) {
                continue;
            }
            self.site_policies.insert(id.clone(), policy);
            self.entity_sources
                .site_policies
                .insert(id, ServiceEntitySource::Builtin);
        }
    }

    /// Refresh profile target-readiness rows from retained service policy.
    pub fn refresh_profile_readiness(&mut self) {
        self.refresh_profile_readiness_for_browser_build(None);
    }

    /// Refresh readiness for an access-plan build override without persisting
    /// authentication evidence or changing the configured browser default.
    pub fn refresh_profile_readiness_for_browser_build(
        &mut self,
        browser_build_override: Option<BrowserBuild>,
    ) {
        self.apply_builtin_site_policies();
        let site_policies = self.site_policies.clone();
        let default_browser_build = self.default_browser_build;
        for profile in self.profiles.values_mut() {
            profile.target_readiness = derive_profile_target_readiness(
                profile,
                &site_policies,
                default_browser_build,
                browser_build_override,
            );
        }
    }

    /// Refresh bounded derived collections before persistence or API exposure.
    pub fn refresh_derived_views(&mut self) {
        self.refresh_profile_readiness();
        self.refresh_service_tab_handles();
        let preserved_metadata = self
            .incidents
            .iter()
            .map(|incident| {
                (
                    incident.id.clone(),
                    (
                        incident.acknowledged_at.clone(),
                        incident.acknowledged_by.clone(),
                        incident.acknowledgement_note.clone(),
                        incident.resolved_at.clone(),
                        incident.resolved_by.clone(),
                        incident.resolution_note.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.incidents = derive_service_incidents(self)
            .into_iter()
            .map(|mut incident| {
                if let Some((
                    acknowledged_at,
                    acknowledged_by,
                    acknowledgement_note,
                    resolved_at,
                    resolved_by,
                    resolution_note,
                )) = preserved_metadata.get(&incident.id)
                {
                    incident.acknowledged_at = acknowledged_at.clone();
                    incident.acknowledged_by = acknowledged_by.clone();
                    incident.acknowledgement_note = acknowledgement_note.clone();
                    incident.resolved_at = resolved_at.clone();
                    incident.resolved_by = resolved_by.clone();
                    incident.resolution_note = resolution_note.clone();
                    if resolved_at
                        .as_deref()
                        .is_some_and(|timestamp| incident.latest_timestamp.as_str() <= timestamp)
                    {
                        incident.state = ServiceIncidentState::Recovered;
                        incident.current_health = None;
                        let (severity, escalation, recommended_action) =
                            classify_incident_escalation(&incident);
                        incident.severity = severity;
                        incident.escalation = escalation;
                        incident.recommended_action = recommended_action.to_string();
                    }
                }
                incident
            })
            .collect();
    }

    /// Expire active session leases whose deadline has passed or whose recorded
    /// browser ownership no longer resolves to any retained browser.
    ///
    /// Expiry is explicit instead of hidden inside `refresh_derived_views()` so
    /// snapshot reads stay deterministic. The retained browser and tab records
    /// are preserved while the expired session is removed from active browser
    /// ownership.
    pub fn expire_stale_session_leases(
        &mut self,
        observed_at: &str,
        current_boot_epoch: Option<&str>,
    ) -> Vec<String> {
        let expired_session_ids = self
            .sessions
            .iter()
            .filter(|(_, session)| {
                if session.lease.is_inactive() {
                    return false;
                }
                let deadline_expired = session
                    .expires_at
                    .as_deref()
                    .is_some_and(|expires_at| service_timestamp_is_due(expires_at, observed_at));
                let browser_ownership_orphaned = !session.browser_ids.is_empty()
                    && session
                        .browser_ids
                        .iter()
                        .all(|browser_id| !self.browsers.contains_key(browser_id));
                deadline_expired || browser_ownership_orphaned
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if expired_session_ids.is_empty() {
            return expired_session_ids;
        }

        let expired_session_id_set = expired_session_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<String>>();
        for session_id in &expired_session_ids {
            if let Some(session) = self.sessions.get_mut(session_id) {
                session.lease = LeaseState::Expired;
                session.last_lease_observed_at = Some(observed_at.to_string());
                session.boot_epoch = current_boot_epoch.map(str::to_string);
            }
        }
        for browser in self.browsers.values_mut() {
            browser
                .active_session_ids
                .retain(|session_id| !expired_session_id_set.contains(session_id));
        }
        self.refresh_derived_views();
        expired_session_ids
    }

    /// Refresh service-owned tab handles before API, MCP, CLI, or trace exposure.
    pub fn refresh_service_tab_handles(&mut self) {
        let tab_ids = self.tabs.keys().cloned().collect::<Vec<_>>();
        let mut tab_handles = BTreeMap::<String, Vec<ServiceTabHandle>>::new();

        for tab_id in tab_ids {
            if let Some(handle) = self.service_tab_handle(&tab_id) {
                if let Some(tab) = self.tabs.get_mut(&tab_id) {
                    tab.service_tab_handle = Some(handle.clone());
                }
                tab_handles
                    .entry(handle.browser_id.clone())
                    .or_default()
                    .push(handle);
            }
        }

        for browser in self.browsers.values_mut() {
            browser.tab_handles = tab_handles.remove(&browser.id).unwrap_or_default();
            browser
                .tab_handles
                .sort_by(|left, right| left.tab_id.cmp(&right.tab_id));
        }
    }

    /// Mark only children owned by one closed transport connection as
    /// reconnectable. Stable subject identity remains attached to each child.
    pub fn mark_profile_connection_disconnected(&mut self, connection_instance_id: &str) -> usize {
        let mut changed = 0;
        for tab in self.tabs.values_mut() {
            let Some(access) = tab.profile_access.as_mut() else {
                continue;
            };
            if access.connection_instance_id.as_deref() == Some(connection_instance_id)
                && access.connection_state == ProfileConnectionState::Active
            {
                access.connection_state = ProfileConnectionState::Disconnected;
                changed += 1;
            }
        }
        if changed > 0 {
            self.refresh_service_tab_handles();
        }
        changed
    }

    pub fn service_challenge_task(&self, id: &str) -> Option<&ServiceChallengeTaskRecord> {
        self.challenge_tasks.get(id)
    }

    pub fn service_challenge_task_summary(&self) -> ServiceChallengeTaskSummary {
        challenge_task_summary(&self.challenge_tasks)
    }

    pub fn start_service_challenge_task(
        &mut self,
        input: ServiceChallengeTaskStartInput,
    ) -> Result<(ServiceChallengeTaskRecord, bool), ServiceChallengeTaskError> {
        match prepare_service_challenge_task_start(&self.challenge_tasks, input)? {
            ServiceChallengeTaskStartDecision::Replayed(record) => Ok((*record, true)),
            ServiceChallengeTaskStartDecision::Create(prepared) => {
                let record = complete_service_challenge_task_start(*prepared)?;
                self.challenge_tasks
                    .insert(record.task_id.clone(), record.clone());
                Ok((record, false))
            }
        }
    }

    pub fn status_service_challenge_task(
        &self,
        id: &str,
        principal: &str,
    ) -> Result<ServiceChallengeTaskRecord, ServiceChallengeTaskError> {
        service_challenge_task_status(&self.challenge_tasks, id, principal)
    }

    pub fn resume_service_challenge_task(
        &mut self,
        input: ServiceChallengeTaskResumeInput,
    ) -> Result<(ServiceChallengeTaskRecord, bool), ServiceChallengeTaskError> {
        match prepare_service_challenge_task_resume(&self.challenge_tasks, input)? {
            ServiceChallengeTaskResumeDecision::Replayed(record) => Ok((*record, true)),
            ServiceChallengeTaskResumeDecision::Execute(prepared) => {
                let current_service_tab_handle = self
                    .service_tab_handle(prepared.service_tab_id())
                    .ok_or(ServiceChallengeTaskError::ServiceTabHandleMissing)?;
                let record =
                    complete_service_challenge_task_resume(*prepared, current_service_tab_handle)?;
                self.challenge_tasks
                    .insert(record.task_id.clone(), record.clone());
                Ok((record, false))
            }
        }
    }

    pub fn cancel_service_challenge_task(
        &mut self,
        input: ServiceChallengeTaskCancelInput,
    ) -> Result<(ServiceChallengeTaskRecord, bool), ServiceChallengeTaskError> {
        match cancel_service_challenge_task(&self.challenge_tasks, input)? {
            ServiceChallengeTaskCancelDecision::Replayed(record) => Ok((*record, true)),
            ServiceChallengeTaskCancelDecision::Cancelled(record) => {
                self.challenge_tasks
                    .insert(record.task_id.clone(), (*record).clone());
                Ok((*record, false))
            }
        }
    }

    pub fn service_tab_handle(&self, tab_id: &str) -> Option<ServiceTabHandle> {
        let tab = self.tabs.get(tab_id)?;
        let browser = self.browsers.get(&tab.browser_id);
        let session_id = tab
            .owner_session_id
            .clone()
            .or_else(|| tab.session_id.clone())
            .or_else(|| {
                browser
                    .and_then(|browser| browser.active_session_ids.first())
                    .cloned()
            });
        let session = session_id
            .as_deref()
            .and_then(|session_id| self.sessions.get(session_id));
        let profile_id = session
            .and_then(|session| session.profile_id.clone())
            .or_else(|| browser.and_then(|browser| browser.profile_id.clone()));
        let profile_origin = profile_id
            .as_deref()
            .and_then(|profile_id| self.profiles.get(profile_id))
            .map(|profile| profile.profile_origin)
            .unwrap_or_default();
        let cleanup_policy = session.map(|session| session.cleanup);
        let lease_state = session.map(|session| session.lease);
        let job_id = self.latest_job_id_for_tab(tab_id);
        let stale_reason = service_tab_handle_stale_reason(tab, browser)
            .or_else(|| service_tab_handle_lease_stale_reason(lease_state));

        Some(ServiceTabHandle {
            browser_id: tab.browser_id.clone(),
            session_name: session_id.clone(),
            tab_id: tab.id.clone(),
            target_id: tab.target_id.clone(),
            url: tab.url.clone(),
            title: tab.title.clone(),
            profile_id,
            profile_origin,
            lease_id: session_id.clone(),
            lease_state,
            cleanup_policy,
            lease_heartbeat_expected: lease_state.is_some(),
            owner_session_id: tab.owner_session_id.clone(),
            profile_access: tab.profile_access.clone(),
            job_id,
            trace_filter: ServiceTabHandleTraceFilter {
                browser_id: Some(tab.browser_id.clone()),
                profile_id: profile_id_from_tab_handle_context(self, tab, browser),
                session_id: session_id.clone(),
                service_name: session.and_then(|session| session.service_name.clone()),
                agent_name: session.and_then(|session| session.agent_name.clone()),
                task_name: session.and_then(|session| session.task_name.clone()),
            },
            valid: stale_reason.is_none(),
            stale_reason,
        })
    }

    fn latest_job_id_for_tab(&self, tab_id: &str) -> Option<String> {
        self.jobs
            .values()
            .filter(|job| matches!(&job.target, JobTarget::Tab(id) if id == tab_id))
            .max_by(|left, right| {
                service_job_sort_key(left)
                    .cmp(service_job_sort_key(right))
                    .then_with(|| left.id.cmp(&right.id))
            })
            .map(|job| job.id.clone())
    }
}

fn profile_id_from_tab_handle_context(
    state: &ServiceState,
    tab: &BrowserTab,
    browser: Option<&BrowserProcess>,
) -> Option<String> {
    let session_id = tab
        .owner_session_id
        .as_deref()
        .or(tab.session_id.as_deref())
        .or_else(|| {
            browser
                .and_then(|browser| browser.active_session_ids.first())
                .map(String::as_str)
        });
    session_id
        .and_then(|session_id| state.sessions.get(session_id))
        .and_then(|session| session.profile_id.clone())
        .or_else(|| browser.and_then(|browser| browser.profile_id.clone()))
}

fn service_job_sort_key(job: &ServiceJob) -> &str {
    job.completed_at
        .as_deref()
        .or(job.started_at.as_deref())
        .or(job.submitted_at.as_deref())
        .unwrap_or("")
}

fn service_timestamp_is_due(candidate: &str, observed_at: &str) -> bool {
    let Ok(candidate) = DateTime::parse_from_rfc3339(candidate) else {
        return false;
    };
    let Ok(observed_at) = DateTime::parse_from_rfc3339(observed_at) else {
        return false;
    };
    candidate <= observed_at
}

fn service_tab_handle_lease_stale_reason(lease_state: Option<LeaseState>) -> Option<String> {
    match lease_state {
        Some(LeaseState::Released) => Some("lease_released".to_string()),
        Some(LeaseState::Expired) => Some("lease_expired".to_string()),
        Some(LeaseState::Shared | LeaseState::Exclusive | LeaseState::HumanTakeover) | None => None,
    }
}

fn service_tab_handle_stale_reason(
    tab: &BrowserTab,
    browser: Option<&BrowserProcess>,
) -> Option<String> {
    match tab.lifecycle {
        TabLifecycle::Closed => return Some("tab_closed".to_string()),
        TabLifecycle::Crashed => return Some("tab_crashed".to_string()),
        TabLifecycle::Unknown
        | TabLifecycle::Opening
        | TabLifecycle::Loading
        | TabLifecycle::Ready
        | TabLifecycle::Closing => {}
    }

    let browser = match browser {
        Some(browser) => browser,
        None => return Some("browser_missing".to_string()),
    };

    match browser.health {
        BrowserHealth::Ready | BrowserHealth::Launching | BrowserHealth::Reconnecting => None,
        BrowserHealth::NotStarted => Some("browser_not_started".to_string()),
        BrowserHealth::Degraded => Some("browser_degraded".to_string()),
        BrowserHealth::Unreachable => Some("browser_unreachable".to_string()),
        BrowserHealth::ProcessExited => Some("browser_process_exited".to_string()),
        BrowserHealth::CdpDisconnected => Some("browser_cdp_disconnected".to_string()),
        BrowserHealth::Closing => Some("browser_closing".to_string()),
        BrowserHealth::Faulted => Some("browser_faulted".to_string()),
    }
}

pub fn service_profile_sources(service_state: &ServiceState) -> Vec<ProfileSourceRecord> {
    service_state
        .profiles
        .keys()
        .map(|id| {
            let source = service_state
                .profile_source(id)
                .unwrap_or(ServiceEntitySource::PersistedState);
            ProfileSourceRecord {
                id: id.clone(),
                source: source.as_str().to_string(),
                overrideable: source.overrideable(),
                precedence: vec![
                    "config".to_string(),
                    "runtime_observed".to_string(),
                    "persisted_state".to_string(),
                ],
            }
        })
        .collect()
}

fn derive_profile_target_readiness(
    profile: &BrowserProfile,
    site_policies: &BTreeMap<String, SitePolicy>,
    default_browser_build: Option<BrowserBuild>,
    browser_build_override: Option<BrowserBuild>,
) -> Vec<ProfileTargetReadiness> {
    let explicit_readiness = profile
        .target_readiness
        .iter()
        .filter(|row| !row.target_service_id.is_empty())
        .filter(|row| row.has_explicit_freshness_evidence())
        .map(|row| (row.target_service_id.clone(), row.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut target_service_ids = profile
        .target_service_ids
        .iter()
        .chain(profile.authenticated_service_ids.iter())
        .chain(explicit_readiness.keys())
        .filter(|target| !target.is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();

    for site_policy_id in &profile.site_policy_ids {
        if let Some(policy) = site_policies.get(site_policy_id) {
            if !policy.id.is_empty() {
                target_service_ids.insert(policy.id.clone());
            }
        }
    }

    target_service_ids
        .into_iter()
        .map(|target_service_id| {
            let derived = derive_target_readiness_for_profile(
                profile,
                site_policies,
                &target_service_id,
                default_browser_build,
                browser_build_override,
            );
            if let Some(explicit) = explicit_readiness.get(&target_service_id) {
                normalize_explicit_target_readiness(explicit, derived)
            } else {
                derived
            }
        })
        .collect()
}

fn normalize_explicit_target_readiness(
    explicit: &ProfileTargetReadiness,
    derived: ProfileTargetReadiness,
) -> ProfileTargetReadiness {
    let recommended_action = if explicit.recommended_action.is_empty() {
        match explicit.state {
            ProfileReadinessState::Fresh => "use_profile",
            ProfileReadinessState::Stale => "probe_target_auth_or_reseed_if_needed",
            ProfileReadinessState::BlockedByAttachedDevtools => {
                "close_attached_devtools_then_verify_profile"
            }
            ProfileReadinessState::NeedsManualSeeding => {
                "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
            }
            ProfileReadinessState::SeededUnknownFreshness => {
                "probe_target_auth_or_reuse_if_acceptable"
            }
            ProfileReadinessState::Unknown => "verify_or_seed_profile_before_authenticated_work",
        }
        .to_string()
    } else {
        explicit.recommended_action.clone()
    };
    let seeding_mode = if explicit.manual_seeding_required
        || matches!(explicit.state, ProfileReadinessState::NeedsManualSeeding)
    {
        ProfileSeedingMode::DetachedHeadedNoCdp
    } else {
        explicit.seeding_mode
    };
    let preferred_keyring = explicit
        .preferred_keyring
        .or(derived.preferred_keyring)
        .or_else(|| {
            matches!(seeding_mode, ProfileSeedingMode::DetachedHeadedNoCdp)
                .then_some(ProfileKeyringPolicy::BasicPasswordStore)
        });
    let setup_scopes = if explicit.setup_scopes.is_empty() {
        derived.setup_scopes
    } else {
        explicit.setup_scopes.clone()
    };

    ProfileTargetReadiness {
        target_service_id: explicit.target_service_id.clone(),
        login_id: explicit.login_id.clone().or(derived.login_id),
        state: explicit.state,
        manual_seeding_required: matches!(
            explicit.state,
            ProfileReadinessState::NeedsManualSeeding
        ) || (explicit.manual_seeding_required
            && !matches!(explicit.state, ProfileReadinessState::Fresh)),
        evidence: if explicit.evidence.is_empty() {
            derived.evidence
        } else {
            explicit.evidence.clone()
        },
        recommended_action,
        seeding_mode,
        cdp_attachment_allowed_during_seeding: matches!(
            seeding_mode,
            ProfileSeedingMode::AttachableOk
        ),
        preferred_keyring,
        setup_scopes,
        last_verified_at: explicit
            .last_verified_at
            .clone()
            .or(derived.last_verified_at),
        freshness_expires_at: explicit
            .freshness_expires_at
            .clone()
            .or(derived.freshness_expires_at),
    }
}

pub fn builtin_site_policy(id: &str) -> Option<SitePolicy> {
    builtin_site_policies()
        .into_iter()
        .find(|policy| policy.id == id)
}

pub fn service_site_policy_sources(service_state: &ServiceState) -> Vec<SitePolicySourceRecord> {
    service_state
        .site_policies
        .keys()
        .map(|id| {
            let source = service_state
                .site_policy_source(id)
                .unwrap_or(ServiceEntitySource::PersistedState);
            SitePolicySourceRecord {
                id: id.clone(),
                source: source.as_str().to_string(),
                overrideable: source.overrideable(),
                precedence: vec![
                    "config".to_string(),
                    "persisted_state".to_string(),
                    "builtin".to_string(),
                ],
            }
        })
        .collect()
}

pub fn builtin_site_policies() -> Vec<SitePolicy> {
    vec![
        SitePolicy {
            id: "canva".to_string(),
            origin_pattern: "https://www.canva.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            browser_build: Some(BrowserBuild::CdpFreeHeaded),
            requires_cdp_free: true,
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(700),
                jitter_ms: Some(600),
                cooldown_ms: Some(3_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            manual_login_preferred: true,
            profile_required: true,
            challenge_policy: ChallengePolicy::ManualOnly,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: Some(
                "Canva can reject sessions when a DevTools port is attached; prefer headed Chrome without CDP."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "ups".to_string(),
            origin_pattern: "https://www.ups.com".to_string(),
            browser_host: Some(BrowserHost::RemoteHeaded),
            browser_build: Some(BrowserBuild::StealthcdpChromium),
            view_stream: Some(ViewStreamProvider::RdpGateway),
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(500),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            profile_required: true,
            challenge_policy: ChallengePolicy::AvoidFirst,
            notes: Some(
                "UPS tracking failed in true headless stealth Chromium on 2026-05-17 with HTTP/2 navigation errors; prefer a headed, remotely viewable stealth Chromium session."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "google".to_string(),
            origin_pattern: "https://accounts.google.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            browser_build: Some(BrowserBuild::StealthcdpChromium),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(500),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            manual_login_preferred: false,
            profile_required: true,
            challenge_policy: ChallengePolicy::ManualOnly,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: Some(
                "Google sign-in should use headed stealth Chromium; a failed sign-in or encountered challenge may require operator intervention."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "gmail".to_string(),
            origin_pattern: "https://mail.google.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            browser_build: Some(BrowserBuild::StealthcdpChromium),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(500),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            manual_login_preferred: false,
            profile_required: true,
            challenge_policy: ChallengePolicy::ManualOnly,
            allowed_challenge_providers: vec!["manual".to_string()],
            notes: Some(
                "Gmail inherits headed stealth Chromium sign-in and should prefer a persistent profile."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "google_sheets".to_string(),
            origin_pattern: "https://docs.google.com/spreadsheets".to_string(),
            browser_host: Some(BrowserHost::RemoteHeaded),
            browser_build: Some(BrowserBuild::StealthcdpChromium),
            view_stream: Some(ViewStreamProvider::RdpGateway),
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(500),
                jitter_ms: Some(400),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(1),
            },
            profile_required: true,
            challenge_policy: ChallengePolicy::AvoidFirst,
            notes: Some(
                "Google Sheets work should stay on the managed stealth Chromium lane so rendered documents are inspectable through the service viewport."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
        SitePolicy {
            id: "microsoft".to_string(),
            origin_pattern: "https://login.microsoftonline.com".to_string(),
            browser_host: Some(BrowserHost::LocalHeaded),
            browser_build: Some(BrowserBuild::StockChrome),
            interaction_mode: InteractionMode::HumanLikeInput,
            rate_limit: RateLimitPolicy {
                min_action_delay_ms: Some(450),
                jitter_ms: Some(300),
                cooldown_ms: Some(2_000),
                max_parallel_sessions: Some(1),
                retry_budget: Some(2),
            },
            manual_login_preferred: true,
            profile_required: true,
            challenge_policy: ChallengePolicy::ProviderAllowed,
            allowed_challenge_providers: vec![
                "manual".to_string(),
                "totp".to_string(),
                "sms".to_string(),
                "email".to_string(),
            ],
            notes: Some(
                "Microsoft sign-in should prefer persistent headed profiles with conservative pacing."
                    .to_string(),
            ),
            ..SitePolicy::default()
        },
    ]
}

fn derive_target_readiness_for_profile(
    profile: &BrowserProfile,
    site_policies: &BTreeMap<String, SitePolicy>,
    target_service_id: &str,
    default_browser_build: Option<BrowserBuild>,
    browser_build_override: Option<BrowserBuild>,
) -> ProfileTargetReadiness {
    let authenticated = profile
        .authenticated_service_ids
        .iter()
        .any(|id| id == target_service_id);
    let manual_seeding_required = !authenticated
        && target_requires_detached_manual_seeding(
            profile,
            site_policies,
            target_service_id,
            default_browser_build,
            browser_build_override,
        );
    let (state, evidence, recommended_action, seeding_mode, preferred_keyring, setup_scopes) =
        if authenticated {
            (
                ProfileReadinessState::SeededUnknownFreshness,
                "profile_authenticated_service_hint".to_string(),
                "probe_target_auth_or_reuse_if_acceptable".to_string(),
                ProfileSeedingMode::NotRequired,
                None,
                Vec::new(),
            )
        } else if manual_seeding_required {
            (
                ProfileReadinessState::NeedsManualSeeding,
                "manual_seed_required_without_authenticated_hint".to_string(),
                "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
                    .to_string(),
                ProfileSeedingMode::DetachedHeadedNoCdp,
                Some(ProfileKeyringPolicy::BasicPasswordStore),
                profile_seeding_setup_scopes(target_service_id),
            )
        } else {
            (
                ProfileReadinessState::Unknown,
                "no_authenticated_service_hint".to_string(),
                "verify_or_seed_profile_before_authenticated_work".to_string(),
                ProfileSeedingMode::AttachableOk,
                Some(profile.keyring),
                vec!["signin".to_string()],
            )
        };

    ProfileTargetReadiness {
        target_service_id: target_service_id.to_string(),
        login_id: None,
        state,
        manual_seeding_required,
        evidence,
        recommended_action,
        seeding_mode,
        cdp_attachment_allowed_during_seeding: matches!(
            seeding_mode,
            ProfileSeedingMode::AttachableOk
        ),
        preferred_keyring,
        setup_scopes,
        last_verified_at: None,
        freshness_expires_at: None,
    }
}

fn target_requires_detached_manual_seeding(
    profile: &BrowserProfile,
    site_policies: &BTreeMap<String, SitePolicy>,
    target_service_id: &str,
    default_browser_build: Option<BrowserBuild>,
    browser_build_override: Option<BrowserBuild>,
) -> bool {
    let site_policy = site_policies.get(target_service_id);
    if site_policy.is_some_and(|policy| policy.requires_cdp_free) {
        return true;
    }
    let effective_browser_build = browser_build_override
        .or_else(|| site_policy.and_then(|policy| policy.browser_build))
        .or(profile.browser_build)
        .or(default_browser_build);
    if effective_browser_build == Some(BrowserBuild::StealthcdpChromium) {
        return false;
    }
    profile.manual_login_preferred
        || target_is_google_signin(target_service_id)
        || site_policy.is_some_and(|policy| policy.manual_login_preferred)
}

fn target_is_google_signin(target_service_id: &str) -> bool {
    let normalized = target_service_id.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "google" | "gmail" | "google-login" | "google_signin" | "google-signin"
    )
}

pub fn default_profile_seeding_url(target_service_id: &str) -> &'static str {
    if target_is_google_signin(target_service_id) {
        "https://accounts.google.com"
    } else {
        "about:blank"
    }
}

fn profile_seeding_setup_scopes(target_service_id: &str) -> Vec<String> {
    if target_is_google_signin(target_service_id) {
        vec![
            "signin".to_string(),
            "chrome_sync".to_string(),
            "passkeys".to_string(),
            "browser_plugins".to_string(),
        ]
    } else {
        vec!["signin".to_string()]
    }
}

fn derive_service_incidents(state: &ServiceState) -> Vec<ServiceIncident> {
    let mut grouped = BTreeMap::<String, ServiceIncident>::new();

    for event in state
        .events
        .iter()
        .filter(|event| service_event_is_incident(event))
    {
        let browser_id = event.browser_id.clone();
        let monitor_id = service_event_monitor_id(event);
        let key = service_event_incident_id(event)
            .or(browser_id.clone())
            .unwrap_or_else(|| "service".to_string());
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                monitor_id: monitor_id.clone(),
                monitor_target: service_event_monitor_target(event),
                monitor_result: service_event_monitor_result(event),
                label: incident_label(state, browser_id.as_deref(), monitor_id.as_deref()),
                state: classify_incident_state(
                    browser_id.is_some(),
                    monitor_id.is_some(),
                    event.current_health,
                    event.kind,
                ),
                latest_timestamp: event.timestamp.clone(),
                latest_message: event.message.clone(),
                latest_kind: service_event_kind_name(event.kind).to_string(),
                current_health: event.current_health,
                ..ServiceIncident::default()
            });

        if !service_event_is_handling(event.kind)
            && incident_is_newer(&event.timestamp, &incident.latest_timestamp)
        {
            incident.latest_timestamp = event.timestamp.clone();
            incident.latest_message = event.message.clone();
            incident.latest_kind = service_event_kind_name(event.kind).to_string();
            incident.current_health = event.current_health.or(incident.current_health);
            incident.monitor_id = monitor_id.clone().or(incident.monitor_id.clone());
            incident.monitor_target =
                service_event_monitor_target(event).or_else(|| incident.monitor_target.clone());
            incident.monitor_result =
                service_event_monitor_result(event).or_else(|| incident.monitor_result.clone());
            incident.state = classify_incident_state(
                incident.browser_id.is_some(),
                incident.monitor_id.is_some(),
                incident.current_health,
                event.kind,
            );
        }

        incident.event_ids.push(event.id.clone());
    }

    for job in state
        .jobs
        .values()
        .filter(|job| service_job_is_incident(job))
    {
        let browser_id = service_job_browser_id(job, state);
        let key = browser_id.clone().unwrap_or_else(|| "service".to_string());
        let timestamp = service_job_incident_timestamp(job);
        let message = service_job_incident_message(job);
        let kind = service_job_incident_kind(job);
        let incident = grouped
            .entry(key.clone())
            .or_insert_with(|| ServiceIncident {
                id: key.clone(),
                browser_id: browser_id.clone(),
                label: browser_id
                    .clone()
                    .unwrap_or_else(|| "Service incidents".to_string()),
                state: classify_job_incident_state(browser_id.is_some()),
                latest_timestamp: timestamp.to_string(),
                latest_message: message.clone(),
                latest_kind: kind.to_string(),
                current_health: state.browsers.get(&key).map(|browser| browser.health),
                ..ServiceIncident::default()
            });

        if incident_is_newer(timestamp, &incident.latest_timestamp) {
            incident.latest_timestamp = timestamp.to_string();
            incident.latest_message = message;
            incident.latest_kind = kind.to_string();
            if let Some(browser_id) = incident.browser_id.as_ref() {
                incident.current_health =
                    state.browsers.get(browser_id).map(|browser| browser.health);
            }
            incident.state = classify_job_incident_state(incident.browser_id.is_some());
        }

        incident.job_ids.push(job.id.clone());
    }

    for incident in derive_remote_view_incidents(state) {
        grouped.insert(incident.id.clone(), incident);
    }
    for incident in derive_shared_profile_coordination_incidents(state) {
        grouped.insert(incident.id.clone(), incident);
    }

    let event_timestamps: BTreeMap<&str, &str> = state
        .events
        .iter()
        .map(|event| (event.id.as_str(), event.timestamp.as_str()))
        .collect();
    let job_timestamps: BTreeMap<&str, &str> = state
        .jobs
        .values()
        .map(|job| (job.id.as_str(), service_job_incident_timestamp(job)))
        .collect();

    let mut incidents = grouped.into_values().collect::<Vec<_>>();
    for incident in &mut incidents {
        incident.event_ids.sort_by(|left, right| {
            job_or_event_timestamp(&event_timestamps, left)
                .cmp(job_or_event_timestamp(&event_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        incident.job_ids.sort_by(|left, right| {
            job_or_event_timestamp(&job_timestamps, left)
                .cmp(job_or_event_timestamp(&job_timestamps, right))
                .reverse()
                .then_with(|| left.cmp(right))
        });
        if incident.label.is_empty() {
            incident.label = incident_label(
                state,
                incident.browser_id.as_deref(),
                incident.monitor_id.as_deref(),
            );
        }
        if incident.current_health.is_none() {
            incident.current_health = incident
                .browser_id
                .as_ref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .map(|browser| browser.health);
        }
        if incident.monitor_id.is_some() {
            incident.state = ServiceIncidentState::Active;
        } else if let Some(browser_id) = incident.browser_id.as_ref() {
            if browser_health_is_bad(incident.current_health) {
                incident.state = ServiceIncidentState::Active;
            } else if incident.latest_kind == "browser_health_changed"
                && state
                    .events
                    .iter()
                    .filter(|event| incident.event_ids.contains(&event.id))
                    .any(|event| {
                        event.kind == ServiceEventKind::BrowserHealthChanged
                            && browser_health_is_recovery(
                                event.previous_health,
                                event.current_health,
                            )
                            && event.browser_id.as_deref() == Some(browser_id)
                    })
            {
                incident.state = ServiceIncidentState::Recovered;
            } else {
                incident.state = ServiceIncidentState::Active;
            }
        } else {
            incident.state = ServiceIncidentState::Service;
        }
        let (severity, escalation, recommended_action) = classify_incident_escalation(incident);
        incident.severity = severity;
        incident.escalation = escalation;
        incident.recommended_action = recommended_action.to_string();
    }
    incidents.sort_by(|left, right| {
        left.latest_timestamp
            .cmp(&right.latest_timestamp)
            .reverse()
            .then_with(|| left.id.cmp(&right.id))
    });
    incidents
}

fn classify_incident_escalation(
    incident: &ServiceIncident,
) -> (
    ServiceIncidentSeverity,
    ServiceIncidentEscalation,
    &'static str,
) {
    if incident.state == ServiceIncidentState::Recovered {
        return (
            ServiceIncidentSeverity::Info,
            ServiceIncidentEscalation::None,
            "No operator action required.",
        );
    }

    match incident.current_health {
        Some(BrowserHealth::Faulted) => (
            ServiceIncidentSeverity::Critical,
            ServiceIncidentEscalation::OsDegradedPossible,
            "Inspect the host OS and process table before retrying browser automation.",
        ),
        Some(BrowserHealth::Degraded) => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::BrowserDegraded,
            "Inspect browser health and retry or relaunch the affected browser if needed.",
        ),
        Some(BrowserHealth::ProcessExited)
        | Some(BrowserHealth::CdpDisconnected)
        | Some(BrowserHealth::Unreachable) => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::BrowserRecovery,
            "Review recovery trace and retry or relaunch the affected browser.",
        ),
        _ if incident.latest_kind == "service_job_timeout" => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::JobAttention,
            "Inspect the timed-out service job and retry only after checking browser state.",
        ),
        _ if incident.latest_kind == "service_job_cancelled" => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::JobAttention,
            "Confirm the cancellation was intentional before resubmitting the task.",
        ),
        _ if incident.monitor_id.is_some() => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::MonitorAttention,
            "Inspect the failed monitor target and last result; fix the target, refresh login state, pause the monitor, or reset reviewed failures before rerunning.",
        ),
        _ if incident.latest_kind.starts_with("remote_view_") => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect remote-view route readiness, display allocation state, provider auth, and route-pool availability before reopening the workspace view.",
        ),
        _ if incident.latest_kind == "profile_lease_conflict" => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect the retained profile holder and route new clients through the existing browser session or release the stale holder.",
        ),
        _ if incident.latest_kind == "shared_tab_abandoned" => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect the shared tab handle, release or refresh the affected client tab, then prune closed retained tab records when safe.",
        ),
        _ if incident.state == ServiceIncidentState::Service => (
            ServiceIncidentSeverity::Error,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect service logs, reconciliation state, and recent jobs.",
        ),
        _ => (
            ServiceIncidentSeverity::Warning,
            ServiceIncidentEscalation::ServiceTriage,
            "Inspect the incident activity timeline.",
        ),
    }
}

fn job_or_event_timestamp<'a>(timestamps: &'a BTreeMap<&str, &'a str>, id: &str) -> &'a str {
    timestamps.get(id).copied().unwrap_or("")
}

fn incident_is_newer(candidate: &str, current: &str) -> bool {
    candidate >= current
}

fn service_event_is_incident(event: &ServiceEvent) -> bool {
    match event.kind {
        ServiceEventKind::JobTerminal => event
            .terminal_outcome
            .as_ref()
            .is_some_and(|outcome| outcome.state != ServiceTerminalState::Succeeded),
        ServiceEventKind::ReconciliationError => true,
        ServiceEventKind::IncidentAcknowledged
        | ServiceEventKind::IncidentResolved
        | ServiceEventKind::BrowserRecoveryOverride => true,
        ServiceEventKind::BrowserHealthChanged => {
            browser_health_is_bad(event.current_health)
                || browser_health_is_recovery(event.previous_health, event.current_health)
        }
        ServiceEventKind::TabLifecycleChanged => {
            event
                .details
                .as_ref()
                .and_then(|details| details.get("result"))
                .and_then(serde_json::Value::as_str)
                == Some("target_crashed")
        }
        ServiceEventKind::Reconciliation
        | ServiceEventKind::BrowserLaunchRecorded
        | ServiceEventKind::BrowserRecoveryStarted
        | ServiceEventKind::ProfileLeaseWaitStarted
        | ServiceEventKind::ProfileLeaseWaitEnded
        | ServiceEventKind::ProfileLeaseLifecycleChanged
        | ServiceEventKind::ViewerTakeoverRequested
        | ServiceEventKind::ViewerConnected
        | ServiceEventKind::ViewerDisconnected
        | ServiceEventKind::ControllerRequested
        | ServiceEventKind::ControllerGranted
        | ServiceEventKind::ControllerDenied
        | ServiceEventKind::RouteReleased => false,
    }
}

fn service_event_is_handling(kind: ServiceEventKind) -> bool {
    matches!(
        kind,
        ServiceEventKind::IncidentAcknowledged
            | ServiceEventKind::IncidentResolved
            | ServiceEventKind::BrowserRecoveryOverride
    )
}

fn service_event_incident_id(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("incidentId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn service_event_monitor_id(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("monitorId"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn service_event_monitor_target(event: &ServiceEvent) -> Option<serde_json::Value> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("target"))
        .cloned()
}

fn service_event_monitor_result(event: &ServiceEvent) -> Option<String> {
    event
        .details
        .as_ref()
        .and_then(|details| details.get("result"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn incident_label(
    state: &ServiceState,
    browser_id: Option<&str>,
    monitor_id: Option<&str>,
) -> String {
    if let Some(browser_id) = browser_id {
        return browser_id.to_string();
    }
    if let Some(monitor_id) = monitor_id {
        return state
            .monitors
            .get(monitor_id)
            .map(|monitor| {
                if monitor.name.trim().is_empty() {
                    format!("Monitor {}", monitor_id)
                } else {
                    format!("Monitor {}", monitor.name)
                }
            })
            .unwrap_or_else(|| format!("Monitor {}", monitor_id));
    }
    "Service incidents".to_string()
}

fn browser_health_is_bad(value: Option<BrowserHealth>) -> bool {
    matches!(
        value,
        Some(BrowserHealth::Degraded)
            | Some(BrowserHealth::ProcessExited)
            | Some(BrowserHealth::CdpDisconnected)
            | Some(BrowserHealth::Unreachable)
            | Some(BrowserHealth::Faulted)
    )
}

fn browser_health_is_recovery(
    previous: Option<BrowserHealth>,
    current: Option<BrowserHealth>,
) -> bool {
    browser_health_is_bad(previous) && current == Some(BrowserHealth::Ready)
}

fn service_job_is_incident(job: &ServiceJob) -> bool {
    matches!(job.state, JobState::Cancelled | JobState::TimedOut)
}

fn service_job_incident_kind(job: &ServiceJob) -> &'static str {
    match job.state {
        JobState::TimedOut => "service_job_timeout",
        JobState::Cancelled => "service_job_cancelled",
        _ => "service_job_incident",
    }
}

fn service_job_incident_message(job: &ServiceJob) -> String {
    if job.state == JobState::TimedOut {
        format!("{} timed out", empty_to_service_label(&job.action))
    } else {
        format!("{} was cancelled", empty_to_service_label(&job.action))
    }
}

fn empty_to_service_label(value: &str) -> &str {
    if value.is_empty() {
        "Service job"
    } else {
        value
    }
}

fn service_job_incident_timestamp(job: &ServiceJob) -> &str {
    job.completed_at
        .as_deref()
        .or(job.started_at.as_deref())
        .or(job.submitted_at.as_deref())
        .unwrap_or("")
}

fn derive_shared_profile_coordination_incidents(state: &ServiceState) -> Vec<ServiceIncident> {
    let mut incidents = Vec::new();

    for session in state
        .sessions
        .values()
        .filter(|session| !session.lease.is_inactive())
    {
        if !session.profile_lease_conflict_session_ids.is_empty() {
            let profile_id = session.profile_id.as_deref().unwrap_or("unknown-profile");
            incidents.push(ServiceIncident {
                id: format!("profile-lease-conflict:{}", session.id),
                browser_id: session.browser_ids.first().cloned(),
                label: format!("Profile lease conflict {}", profile_id),
                state: ServiceIncidentState::Service,
                latest_timestamp: session_timestamp(session),
                latest_message: format!(
                    "Session '{}' conflicts with profile holder session(s) {} for profile '{}'",
                    session.id,
                    session.profile_lease_conflict_session_ids.join(", "),
                    profile_id
                ),
                latest_kind: "profile_lease_conflict".to_string(),
                ..ServiceIncident::default()
            });
        }

        for tab_id in &session.tab_ids {
            let Some((browser_id, reason)) = abandoned_shared_tab_reason(state, session, tab_id)
            else {
                continue;
            };
            incidents.push(ServiceIncident {
                id: format!("shared-tab-abandoned:{}:{}", session.id, tab_id),
                browser_id,
                label: format!("Shared tab {}", tab_id),
                state: ServiceIncidentState::Service,
                latest_timestamp: session_timestamp(session),
                latest_message: format!(
                    "Active session '{}' still references shared tab '{}' that is {}",
                    session.id, tab_id, reason
                ),
                latest_kind: "shared_tab_abandoned".to_string(),
                ..ServiceIncident::default()
            });
        }
    }

    incidents
}

fn abandoned_shared_tab_reason(
    state: &ServiceState,
    session: &BrowserSession,
    tab_id: &str,
) -> Option<(Option<String>, &'static str)> {
    let Some(tab) = state.tabs.get(tab_id) else {
        return Some((session.browser_ids.first().cloned(), "missing"));
    };
    let reason = match tab.lifecycle {
        TabLifecycle::Closed => "closed",
        TabLifecycle::Crashed => "crashed",
        _ => return None,
    };
    Some((Some(tab.browser_id.clone()), reason))
}

fn session_timestamp(session: &BrowserSession) -> String {
    session
        .last_lease_observed_at
        .as_deref()
        .or(session.created_at.as_deref())
        .unwrap_or("")
        .to_string()
}

fn derive_remote_view_incidents(state: &ServiceState) -> Vec<ServiceIncident> {
    let mut incidents = Vec::new();

    for route in state.remote_view_routes.values() {
        let Some((kind, message)) = remote_view_route_incident(route, state) else {
            continue;
        };
        incidents.push(ServiceIncident {
            id: format!("remote-view-route:{}", route.id),
            browser_id: route.browser_id.clone(),
            label: format!("Remote route {}", route.id),
            state: ServiceIncidentState::Service,
            latest_timestamp: remote_view_route_timestamp(route, state),
            latest_message: message,
            latest_kind: kind.to_string(),
            current_health: route
                .browser_id
                .as_ref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .map(|browser| browser.health),
            ..ServiceIncident::default()
        });
    }

    for entry in state.route_pool.values() {
        let Some((kind, message)) = remote_view_route_pool_entry_incident(entry, state) else {
            continue;
        };
        incidents.push(ServiceIncident {
            id: format!("remote-view-route-pool:{}", entry.id),
            label: format!("Remote route pool {}", entry.id),
            state: ServiceIncidentState::Service,
            latest_message: message,
            latest_kind: kind.to_string(),
            ..ServiceIncident::default()
        });
    }

    for allocation in state.display_allocations.values().filter(|allocation| {
        allocation.display_isolation == "private_virtual_display" && allocation.state == "ready"
    }) {
        let has_ready_route = state.remote_view_routes.values().any(|route| {
            route.display_allocation_id.as_deref() == Some(allocation.id.as_str())
                && matches!(
                    route.state.as_str(),
                    "ready" | "allocating" | "reconnecting"
                )
        });
        if has_ready_route || state.route_pool.is_empty() {
            continue;
        }
        let has_available_pool_entry = state.route_pool.values().any(|entry| {
            entry.state == "available" && route_pool_entry_targets_allocation(entry, allocation)
        });
        if has_available_pool_entry {
            continue;
        }
        incidents.push(ServiceIncident {
            id: format!("remote-view-route-pool-exhausted:{}", allocation.id),
            browser_id: allocation.owner_browser_id.clone(),
            label: format!("Remote route pool exhausted for {}", allocation.id),
            state: ServiceIncidentState::Service,
            latest_timestamp: allocation
                .updated_at
                .as_deref()
                .or(allocation.last_health_check_at.as_deref())
                .unwrap_or("")
                .to_string(),
            latest_message: format!(
                "No available remote-view route-pool entry for display allocation '{}'",
                allocation.id
            ),
            latest_kind: "remote_view_route_pool_exhausted".to_string(),
            current_health: allocation
                .owner_browser_id
                .as_ref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .map(|browser| browser.health),
            ..ServiceIncident::default()
        });
    }

    incidents
}

fn remote_view_route_incident(
    route: &RemoteViewRoute,
    state: &ServiceState,
) -> Option<(&'static str, String)> {
    if matches!(
        route.state.as_str(),
        "ready" | "allocating" | "reconnecting" | "released"
    ) {
        return None;
    }
    if let Some(display_allocation_id) = route.display_allocation_id.as_ref() {
        if !state
            .display_allocations
            .contains_key(display_allocation_id)
        {
            return Some((
                "remote_view_display_missing",
                format!(
                    "Remote route '{}' references missing display allocation '{}'",
                    route.id, display_allocation_id
                ),
            ));
        }
        if let Some(allocation) = state.display_allocations.get(display_allocation_id) {
            if !matches!(allocation.state.as_str(), "ready" | "allocating") {
                if let Some(lease) =
                    completed_route_bound_acquisition_for_route(state, route, display_allocation_id)
                {
                    return Some((
                        "remote_view_finalization_incomplete",
                        format!(
                            "Remote route '{}' has completed acquisition lease '{}' but display allocation '{}' is {}",
                            route.id, lease.id, display_allocation_id, allocation.state
                        ),
                    ));
                }
            }
        }
    }
    let readiness_text = route
        .readiness
        .as_ref()
        .map(remote_view_readiness_text)
        .unwrap_or_default();
    let kind = remote_view_failure_kind(&readiness_text).unwrap_or("remote_view_route_unreachable");
    Some((
        kind,
        format!(
            "Remote route '{}' is {}{}",
            route.id,
            route.state,
            if readiness_text.is_empty() {
                String::new()
            } else {
                format!(": {readiness_text}")
            }
        ),
    ))
}

fn remote_view_route_pool_entry_incident(
    entry: &RoutePoolEntry,
    state: &ServiceState,
) -> Option<(&'static str, String)> {
    if matches!(
        entry.state.as_str(),
        "available" | "checked_out" | "unknown"
    ) {
        return None;
    }
    if entry.state == "pending" {
        if let Some(lease) = completed_route_bound_acquisition_for_pool_entry(state, entry) {
            return Some((
                "remote_view_finalization_incomplete",
                format!(
                    "Remote route-pool entry '{}' has completed acquisition lease '{}' but remains pending",
                    entry.id, lease.id
                ),
            ));
        }
    }
    let readiness_text = entry
        .readiness
        .as_ref()
        .map(remote_view_readiness_text)
        .unwrap_or_default();
    let kind = remote_view_failure_kind(&readiness_text).unwrap_or("remote_view_route_unreachable");
    Some((
        kind,
        format!(
            "Remote route-pool entry '{}' is {}{}",
            entry.id,
            entry.state,
            if readiness_text.is_empty() {
                String::new()
            } else {
                format!(": {readiness_text}")
            }
        ),
    ))
}

fn completed_route_bound_acquisition_for_route<'a>(
    state: &'a ServiceState,
    route: &RemoteViewRoute,
    display_allocation_id: &str,
) -> Option<&'a RemoteViewAcquisitionLease> {
    state.remote_view_acquisition_leases.values().find(|lease| {
        lease.state == "completed"
            && lease.phase == "checked_out"
            && lease.route_id == route.id
            && lease.display_allocation_id == display_allocation_id
    })
}

fn completed_route_bound_acquisition_for_pool_entry<'a>(
    state: &'a ServiceState,
    entry: &RoutePoolEntry,
) -> Option<&'a RemoteViewAcquisitionLease> {
    state.remote_view_acquisition_leases.values().find(|lease| {
        lease.state == "completed"
            && lease.phase == "checked_out"
            && lease.route_pool_entry_id.as_deref() == Some(entry.id.as_str())
            && entry.current_route_allocation_id.as_deref() == Some(lease.route_id.as_str())
    })
}

fn remote_view_failure_kind(readiness_text: &str) -> Option<&'static str> {
    let text = readiness_text.to_ascii_lowercase();
    if text.contains("provider_auth") || text.contains("auth") || text.contains("login") {
        Some("remote_view_provider_auth_failed")
    } else if text.contains("finalization") {
        Some("remote_view_finalization_incomplete")
    } else if text.contains("iframe") || text.contains("frame") || text.contains("x-frame") {
        Some("remote_view_iframe_blocked")
    } else if text.contains("display_allocation_missing") || text.contains("display missing") {
        Some("remote_view_display_missing")
    } else if text.contains("unreachable")
        || text.contains("timeout")
        || text.contains("failed")
        || text.contains("closed")
    {
        Some("remote_view_route_unreachable")
    } else {
        None
    }
}

fn remote_view_readiness_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Array(items) => items
            .iter()
            .map(remote_view_readiness_text)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        Value::Object(record) => {
            let mut parts = Vec::new();
            for key in [
                "state",
                "status",
                "readiness",
                "lastProviderEvent",
                "component",
                "reason",
                "failureClass",
                "evidence",
                "message",
                "recovery",
            ] {
                if let Some(value) = record.get(key).and_then(Value::as_str) {
                    if !value.trim().is_empty() {
                        parts.push(value.to_string());
                    }
                }
            }
            for key in ["components", "checks", "results"] {
                if let Some(value) = record.get(key) {
                    let text = remote_view_readiness_text(value);
                    if !text.is_empty() {
                        parts.push(text);
                    }
                }
            }
            parts.join(" ")
        }
        _ => String::new(),
    }
}

fn route_pool_entry_targets_allocation(
    entry: &RoutePoolEntry,
    allocation: &DisplayAllocation,
) -> bool {
    route_pool_entry_matches_display(entry, &allocation.id, Some(allocation))
}

fn remote_view_route_timestamp(route: &RemoteViewRoute, state: &ServiceState) -> String {
    route
        .display_allocation_id
        .as_ref()
        .and_then(|id| state.display_allocations.get(id))
        .and_then(|allocation| {
            allocation
                .updated_at
                .as_deref()
                .or(allocation.last_health_check_at.as_deref())
        })
        .unwrap_or("")
        .to_string()
}

fn service_job_browser_id(job: &ServiceJob, state: &ServiceState) -> Option<String> {
    match &job.target {
        JobTarget::Browser(browser_id) => Some(browser_id.clone()),
        JobTarget::Tab(tab_id) => state.tabs.get(tab_id).map(|tab| tab.browser_id.clone()),
        JobTarget::Service
        | JobTarget::Profile(_)
        | JobTarget::Monitor(_)
        | JobTarget::Challenge(_) => None,
    }
}

fn service_event_kind_name(kind: ServiceEventKind) -> &'static str {
    match kind {
        ServiceEventKind::Reconciliation => "reconciliation",
        ServiceEventKind::BrowserLaunchRecorded => "browser_launch_recorded",
        ServiceEventKind::BrowserHealthChanged => "browser_health_changed",
        ServiceEventKind::BrowserRecoveryStarted => "browser_recovery_started",
        ServiceEventKind::BrowserRecoveryOverride => "browser_recovery_override",
        ServiceEventKind::TabLifecycleChanged => "tab_lifecycle_changed",
        ServiceEventKind::ProfileLeaseWaitStarted => "profile_lease_wait_started",
        ServiceEventKind::ProfileLeaseWaitEnded => "profile_lease_wait_ended",
        ServiceEventKind::ProfileLeaseLifecycleChanged => "profile_lease_lifecycle_changed",
        ServiceEventKind::ViewerTakeoverRequested => "viewer_takeover_requested",
        ServiceEventKind::ViewerConnected => "viewer_connected",
        ServiceEventKind::ViewerDisconnected => "viewer_disconnected",
        ServiceEventKind::ControllerRequested => "controller_requested",
        ServiceEventKind::ControllerGranted => "controller_granted",
        ServiceEventKind::ControllerDenied => "controller_denied",
        ServiceEventKind::RouteReleased => "route_released",
        ServiceEventKind::ReconciliationError => "reconciliation_error",
        ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
        ServiceEventKind::IncidentResolved => "incident_resolved",
        ServiceEventKind::JobTerminal => "job_terminal",
    }
}

fn classify_incident_state(
    has_browser: bool,
    has_monitor: bool,
    current_health: Option<BrowserHealth>,
    kind: ServiceEventKind,
) -> ServiceIncidentState {
    if has_monitor {
        return ServiceIncidentState::Active;
    }
    if !has_browser {
        return ServiceIncidentState::Service;
    }
    if kind == ServiceEventKind::BrowserHealthChanged && !browser_health_is_bad(current_health) {
        return ServiceIncidentState::Recovered;
    }
    ServiceIncidentState::Active
}

fn classify_job_incident_state(has_browser: bool) -> ServiceIncidentState {
    if has_browser {
        ServiceIncidentState::Active
    } else {
        ServiceIncidentState::Service
    }
}

fn materialize_legacy_profile_access_policies(state: &mut ServiceState) {
    let source_revision = state.state_revision;
    let profile_ids = state
        .profiles
        .iter()
        .filter_map(|(profile_id, profile)| profile.access_policy.is_none().then_some(profile_id))
        .cloned()
        .collect::<Vec<_>>();
    if profile_ids.is_empty() {
        return;
    }

    let mut entries = Vec::new();
    for profile_id in profile_ids {
        let matching_sessions = state
            .sessions
            .values()
            .filter(|session| session.profile_id.as_deref() == Some(profile_id.as_str()))
            .collect::<Vec<_>>();
        let proven_principal = matching_sessions
            .first()
            .and_then(|session| session.principal_id.as_deref())
            .filter(|principal| {
                !matching_sessions.is_empty()
                    && matching_sessions.iter().all(|session| {
                        session.lease == LeaseState::Exclusive
                            && session.principal_id.as_deref() == Some(principal)
                            && session.principal_provenance.is_some()
                    })
            })
            .map(str::to_string);

        let (policy, entry) = if let Some(principal_id) = proven_principal {
            let mut policy = ServiceProfileAccessPolicy::shared_local_default(&profile_id);
            policy.mode = ProfileAccessMode::Exclusive;
            policy.default_permissions.clear();
            policy.grants = vec![ProfileAccessGrant {
                subject_id: principal_id,
                minimum_assurance: ProfileIdentityAssurance::RegisteredCapability,
                permissions: ProfileAccessPreset::Administrator.permissions(),
            }];
            (
                policy,
                ProfilePolicyMigrationEntry {
                    profile_id: profile_id.clone(),
                    classification: "proven-strict-compatibility".to_string(),
                    target_mode: ProfileAccessMode::Exclusive,
                    ambiguity: false,
                    blocking: false,
                    reason: "One provenance-backed principal held every exclusive legacy session."
                        .to_string(),
                },
            )
        } else {
            let profile = &state.profiles[&profile_id];
            let ordinary_shared = matching_sessions.is_empty()
                && (profile.shared_service_ids.len() > 1
                    || profile.allocation == ProfileAllocationPolicy::SharedService);
            let classification = if ordinary_shared {
                "shared-local-default"
            } else {
                "ambiguous-legacy"
            };
            let reason = if ordinary_shared {
                "Legacy sharing configuration maps to the trusted local participant preset."
            } else {
                "Legacy identity evidence is insufficient for strict access and remains a nonblocking observation."
            };
            (
                ServiceProfileAccessPolicy::shared_local_default(&profile_id),
                ProfilePolicyMigrationEntry {
                    profile_id: profile_id.clone(),
                    classification: classification.to_string(),
                    target_mode: ProfileAccessMode::SharedLocal,
                    ambiguity: !ordinary_shared,
                    blocking: false,
                    reason: reason.to_string(),
                },
            )
        };
        if let Some(profile) = state.profiles.get_mut(&profile_id) {
            profile.access_policy = Some(policy);
        }
        entries.push(entry);
    }
    let material = serde_json::to_vec(&(source_revision, &entries)).unwrap_or_default();
    let migration_id = format!("profile-policy-migration-{:x}", Sha256::digest(material));
    state.profile_policy_migration = Some(ProfilePolicyMigrationReport {
        schema_version: PROFILE_POLICY_MIGRATION_SCHEMA_VERSION.to_string(),
        migration_id,
        source_revision,
        target_revision: source_revision.saturating_add(1),
        entries,
        blocking_issue_count: 0,
    });
}

/// Explicitly validate cross-record references without normalizing or observing state.
/// Ordinary decode and persistence preparation do not invoke this migration gate.
pub fn validate_service_state_invariants(
    state: &ServiceState,
) -> Result<(), ServiceStateCodecError> {
    for (key, profile) in &state.profiles {
        if profile.id.trim().is_empty() || profile.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_profile_key_mismatch:{key}"
            )));
        }
    }
    for (key, browser) in &state.browsers {
        if browser.id.trim().is_empty() || browser.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_browser_key_mismatch:{key}"
            )));
        }
        if let Some(profile_id) = &browser.profile_id {
            if !state.profiles.contains_key(profile_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_browser_profile_missing:{key}:{profile_id}"
                )));
            }
        }
        if let Some(allocation_id) = &browser.display_allocation_id {
            if !state.display_allocations.contains_key(allocation_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_browser_display_missing:{key}:{allocation_id}"
                )));
            }
        }
        for session_id in &browser.active_session_ids {
            if !state.sessions.contains_key(session_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_browser_session_missing:{key}:{session_id}"
                )));
            }
        }
    }
    for (key, session) in &state.sessions {
        if session.id.trim().is_empty() || session.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_session_key_mismatch:{key}"
            )));
        }
        if let Some(profile_id) = &session.profile_id {
            if !state.profiles.contains_key(profile_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_session_profile_missing:{key}:{profile_id}"
                )));
            }
        }
        for browser_id in &session.browser_ids {
            if !state.browsers.contains_key(browser_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_session_browser_missing:{key}:{browser_id}"
                )));
            }
        }
        for tab_id in &session.tab_ids {
            if !state.tabs.contains_key(tab_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_session_tab_missing:{key}:{tab_id}"
                )));
            }
        }
    }
    for (key, tab) in &state.tabs {
        if tab.id.trim().is_empty() || tab.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_tab_key_mismatch:{key}"
            )));
        }
        if !tab.browser_id.trim().is_empty() && !state.browsers.contains_key(&tab.browser_id) {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_tab_browser_missing:{key}:{}",
                tab.browser_id
            )));
        }
        if let Some(session_id) = &tab.owner_session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_tab_session_missing:{key}:{session_id}"
                )));
            }
        }
        if let Some(session_id) = &tab.session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_tab_routed_session_missing:{key}:{session_id}"
                )));
            }
        }
    }
    for key in state.browser_process_identities.keys() {
        if !state.browsers.contains_key(key) {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_process_browser_missing:{key}"
            )));
        }
    }
    for (key, allocation) in &state.display_allocations {
        if allocation.id.trim().is_empty() || allocation.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_display_key_mismatch:{key}"
            )));
        }
        if let Some(browser_id) = &allocation.owner_browser_id {
            if !state.browsers.contains_key(browser_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_display_browser_missing:{key}:{browser_id}"
                )));
            }
        }
        if let Some(session_id) = &allocation.owner_session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_display_session_missing:{key}:{session_id}"
                )));
            }
        }
        if let Some(profile_id) = &allocation.profile_id {
            if !state.profiles.contains_key(profile_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_display_profile_missing:{key}:{profile_id}"
                )));
            }
        }
        for route_id in &allocation.route_ids {
            if !state.remote_view_routes.contains_key(route_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_display_route_missing:{key}:{route_id}"
                )));
            }
        }
    }
    for (key, route) in &state.remote_view_routes {
        if route.id.trim().is_empty() || route.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_route_key_mismatch:{key}"
            )));
        }
        if let Some(allocation_id) = &route.display_allocation_id {
            if !state.display_allocations.contains_key(allocation_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_route_display_missing:{key}:{allocation_id}"
                )));
            }
        }
        if let Some(browser_id) = &route.browser_id {
            if !state.browsers.contains_key(browser_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_route_browser_missing:{key}:{browser_id}"
                )));
            }
        }
        if let Some(session_id) = &route.session_id {
            if !state.sessions.contains_key(session_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_route_session_missing:{key}:{session_id}"
                )));
            }
        }
        for lease_id in &route.viewer_lease_ids {
            if !state.viewer_leases.contains_key(lease_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_route_viewer_lease_missing:{key}:{lease_id}"
                )));
            }
        }
        if let Some(lease_id) = &route.controller_lease_id {
            if !state.viewer_leases.contains_key(lease_id) {
                return Err(ServiceStateCodecError::Invariant(format!(
                    "service_state_route_controller_lease_missing:{key}:{lease_id}"
                )));
            }
        }
    }
    for (key, entry) in &state.route_pool {
        if entry.id.trim().is_empty() || entry.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_route_pool_key_mismatch:{key}"
            )));
        }
    }
    for (key, lease) in &state.remote_view_acquisition_leases {
        if lease.id.trim().is_empty() || lease.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_acquisition_lease_key_mismatch:{key}"
            )));
        }
    }
    for (key, lease) in &state.viewer_leases {
        if lease.id.trim().is_empty() || lease.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_viewer_lease_key_mismatch:{key}"
            )));
        }
    }
    for (key, handoff) in &state.remote_view_handoffs {
        if handoff.id.trim().is_empty() || handoff.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_handoff_key_mismatch:{key}"
            )));
        }
    }
    for (key, handoff) in &state.profile_seeding_handoffs {
        if handoff.id.trim().is_empty() || handoff.id != *key {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_seeding_handoff_key_mismatch:{key}"
            )));
        }
        if !state.profiles.contains_key(&handoff.profile_id) {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_seeding_handoff_profile_missing:{key}:{}",
                handoff.profile_id
            )));
        }
    }
    for (profile_digest, binding) in state.runtime_owner_registry.principal_bindings() {
        if profile_digest != &binding.profile_identity_digest
            || !state
                .runtime_owner_registry
                .owners()
                .contains_key(profile_digest)
        {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_principal_owner_binding_mismatch:{profile_digest}"
            )));
        }
        if !state.profiles.contains_key(&binding.profile_id) {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_principal_profile_missing:{}",
                binding.profile_id
            )));
        }
        if !state
            .service_principals
            .profile_capabilities
            .contains_key(&binding.capability_id)
        {
            return Err(ServiceStateCodecError::Invariant(format!(
                "service_state_principal_capability_missing:{}",
                binding.capability_id
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod runtime_owner_persistence {
        use super::*;
        use agent_browser_lease_authority::{RuntimeLifecycleRecord, RuntimeOwnerRegistry};

        fn registry(browser_id: &str, revision: u64) -> RuntimeOwnerRegistry {
            // Persistence accepts the existing wire contract without new authority validation.
            serde_json::from_value(json!({
                "revision": revision,
                "owners": {"profile-digest": {
                    "ownerId": format!("owner:{browser_id}"),
                    "profileIdentityDigest": "profile-digest",
                    "state": "ready", "ownerGeneration": 7,
                    "browserId": browser_id, "daemonSessionRoute": "route",
                    "processInstanceDigest": "process-digest", "browserFamily": "chrome",
                    "cdpEndpointIdentityDigest": "cdp-digest", "targetSetDigest": "target-digest",
                    "pendingTransfer": null, "lastTransition": null
                }},
                "principalBindings": {"profile-digest": {
                    "principalId": "principal:test", "profileId": "profile:test",
                    "profileIdentityDigest": "profile-digest", "capabilityId": "capability:test",
                    "provenance": "registered_capability", "ownerGeneration": 7
                }},
                "lifecycleRecords": {browser_id: {
                    "logicalBrowserId": browser_id, "profileIdentityDigest": "profile-digest",
                    "ownerGeneration": 7, "lifecycleState": "ready",
                    "cleanupObligationState": "owned", "bootEpoch": "synthetic-boot"
                }}
            }))
            .unwrap()
        }

        fn snapshot(registry: &RuntimeOwnerRegistry) -> RuntimeOwnerPersistenceSnapshot {
            serde_json::from_value(serde_json::to_value(registry).unwrap()).unwrap()
        }

        fn embedded_state() -> ServiceState {
            let mut state = ServiceState {
                state_revision: 73,
                runtime_owner_registry: registry("embedded", 19),
                ..ServiceState::default()
            };
            state
                .unknown_fields
                .insert("futureField".to_string(), json!({"retain": true}));
            state.profiles.insert(
                "profile:test".to_string(),
                BrowserProfile {
                    id: "profile:test".to_string(),
                    ..BrowserProfile::default()
                },
            );
            state
        }

        #[test]
        fn restoration_preserves_selection_precedence_empty_values_and_revisions() {
            let embedded = embedded_state();
            let owner = registry("sidecar-owner", 3);
            let lifecycle = registry("sidecar-lifecycle", u64::MAX)
                .lifecycle_records()
                .clone();
            let empty = RuntimeOwnerRegistry::default();
            let cases = [
                (
                    "none",
                    None,
                    None,
                    embedded.runtime_owner_registry.clone(),
                    None,
                ),
                ("owner", Some(snapshot(&owner)), None, owner.clone(), None),
                (
                    "lifecycle",
                    None,
                    Some(lifecycle.clone()),
                    embedded.runtime_owner_registry.clone(),
                    Some(lifecycle.clone()),
                ),
                (
                    "both",
                    Some(snapshot(&owner)),
                    Some(lifecycle.clone()),
                    owner.clone(),
                    Some(lifecycle.clone()),
                ),
                (
                    "empty owner",
                    Some(snapshot(&empty)),
                    None,
                    empty.clone(),
                    None,
                ),
                (
                    "empty lifecycle",
                    None,
                    Some(BTreeMap::new()),
                    embedded.runtime_owner_registry.clone(),
                    Some(BTreeMap::new()),
                ),
                (
                    "empty both",
                    Some(snapshot(&empty)),
                    Some(BTreeMap::new()),
                    empty.clone(),
                    Some(BTreeMap::new()),
                ),
                (
                    "empty owner with lifecycle",
                    Some(snapshot(&empty)),
                    Some(lifecycle.clone()),
                    empty,
                    Some(lifecycle),
                ),
            ];
            for (
                name,
                owner_registry,
                lifecycle_records,
                mut expected_registry,
                expected_records,
            ) in cases
            {
                if let Some(records) = expected_records {
                    expected_registry.restore_lifecycle_records(records);
                }
                let expected_revision = expected_registry.revision();
                let mut expected = embedded.clone();
                expected.runtime_owner_registry = expected_registry;
                let mut actual = embedded.clone();
                actual.restore_runtime_owner_persistence(RuntimeOwnerPersistenceRestore {
                    owner_registry,
                    lifecycle_records,
                });
                assert_eq!(actual, expected, "{name}");
                assert_eq!(actual.state_revision(), 73, "{name}");
                assert_eq!(
                    actual.runtime_owner_registry.revision(),
                    expected_revision,
                    "{name}"
                );
            }
        }

        #[test]
        fn parts_are_owned_preserve_source_and_separate_lifecycle_from_owner_wire() {
            let state = embedded_state();
            let before = state.clone();
            let mut parts = state.runtime_owner_persistence_parts();
            assert_eq!(state, before);
            assert_eq!(parts.lifecycle_registry_revision, 19);
            assert_eq!(
                &parts.lifecycle_records,
                state.runtime_owner_registry.lifecycle_records()
            );
            let mut expected_owner_wire =
                serde_json::to_value(&state.runtime_owner_registry).unwrap();
            expected_owner_wire
                .as_object_mut()
                .unwrap()
                .remove("lifecycleRecords");
            assert_eq!(
                serde_json::to_value(&parts.owner_registry).unwrap(),
                expected_owner_wire
            );
            let retained_snapshot = parts.owner_registry.clone();
            parts.lifecycle_records.clear();
            assert_eq!(state, before);
            assert_eq!(parts.owner_registry, retained_snapshot);
            assert!(!state.runtime_owner_registry.lifecycle_records().is_empty());
        }

        #[test]
        fn strip_changes_only_lifecycle_records_and_is_idempotent() {
            let mut state = embedded_state();
            let before = state.clone();
            let parts = state.runtime_owner_persistence_parts();
            state.strip_runtime_lifecycle_for_persistence();
            let mut expected = before.clone();
            expected
                .runtime_owner_registry
                .restore_lifecycle_records(BTreeMap::new());
            assert_eq!(state, expected);
            assert_eq!(
                serde_json::to_value(&state.runtime_owner_registry).unwrap(),
                serde_json::to_value(&parts.owner_registry).unwrap()
            );
            assert_eq!(
                state.runtime_owner_registry.revision(),
                before.runtime_owner_registry.revision()
            );
            assert_eq!(state.state_revision(), before.state_revision());
            state.strip_runtime_lifecycle_for_persistence();
            assert_eq!(state, expected);
            assert!(!before.runtime_owner_registry.lifecycle_records().is_empty());
        }

        #[test]
        fn snapshot_wire_is_transparent_and_retains_registry_compatibility() {
            let registry = registry("wire", 29);
            let wire = serde_json::to_value(&registry).unwrap();
            let snapshot: RuntimeOwnerPersistenceSnapshot =
                serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(&snapshot).unwrap(), wire);
            assert!(wire.get("registry").is_none());
            assert_eq!(wire["revision"], 29);
            assert!(wire.get("principalBindings").is_some());
            assert!(wire.get("lifecycleRecords").is_some());
            let empty: RuntimeOwnerPersistenceSnapshot = serde_json::from_value(json!({})).unwrap();
            assert_eq!(
                serde_json::to_value(empty).unwrap(),
                serde_json::to_value(RuntimeOwnerRegistry::default()).unwrap()
            );
            let mut future = wire;
            future["futureRegistryField"] = json!(true);
            let ordinary: RuntimeOwnerRegistry = serde_json::from_value(future.clone()).unwrap();
            let wrapped: RuntimeOwnerPersistenceSnapshot = serde_json::from_value(future).unwrap();
            assert_eq!(
                serde_json::to_value(wrapped).unwrap(),
                serde_json::to_value(ordinary).unwrap()
            );
        }

        #[test]
        fn lifecycle_restore_overrides_conflicting_owner_evidence_without_advancing_authority() {
            let mut state = embedded_state();
            let owner = registry("same-browser", 2);
            let replacement = BTreeMap::from([(
                "same-browser".to_string(),
                RuntimeLifecycleRecord {
                    logical_browser_id: "same-browser".to_string(),
                    owner_generation: 99,
                    terminal_evidence: vec!["selected-sidecar-evidence".to_string()],
                    ..RuntimeLifecycleRecord::default()
                },
            )]);
            state.restore_runtime_owner_persistence(RuntimeOwnerPersistenceRestore {
                owner_registry: Some(snapshot(&owner)),
                lifecycle_records: Some(replacement.clone()),
            });
            assert_eq!(
                state.runtime_owner_registry.lifecycle_records(),
                &replacement
            );
            assert_eq!(state.runtime_owner_registry.owners(), owner.owners());
            assert_eq!(
                state.runtime_owner_registry.principal_bindings(),
                owner.principal_bindings()
            );
            assert_eq!(state.runtime_owner_registry.revision(), 2);
            assert_eq!(state.state_revision(), 73);
        }
    }

    mod principal_registry_boundary {
        use super::*;
        use agent_browser_lease_authority::{
            AuthenticatedServicePrincipal, RegisteredProfileCapability,
            ServicePrincipalFailureCode, ServicePrincipalProvenance,
            ServicePrincipalRegistrationRequest, ServicePrincipalState,
            ServiceProfileCapabilityState,
        };

        const TOKEN: &str = "synthetic-principal-capability-at-least-thirty-two-characters";
        const REPLACEMENT: &str = "replacement-principal-capability-at-least-thirty-two-characters";
        const NOW: &str = "2026-09-17T12:00:00Z";

        fn request() -> ServicePrincipalRegistrationRequest {
            ServicePrincipalRegistrationRequest {
                principal_id: "principal:test".to_string(),
                display_name: Some("Synthetic principal".to_string()),
                profile_id: "profile:test".to_string(),
                registered_at: Some(NOW.to_string()),
                registered_by: Some("test".to_string()),
            }
        }

        fn registered() -> (ServiceState, RegisteredProfileCapability) {
            let mut state = ServiceState::default();
            let registration = state.register_profile_capability(request(), TOKEN).unwrap();
            (state, registration)
        }

        fn authenticated(state: &ServiceState) -> AuthenticatedServicePrincipal {
            state
                .authenticate_profile_capability(TOKEN, Some("profile:test"))
                .unwrap()
        }

        #[test]
        fn projections_preserve_map_order_revocation_and_missing_records() {
            let (mut state, registration) = registered();
            assert_eq!(state.service_principal_registry_revision(), 1);
            assert_eq!(
                state.service_principal("principal:test"),
                Some(&registration.principal)
            );
            assert_eq!(state.service_principal("missing"), None);
            assert_eq!(state.profile_capability("missing"), None);
            let mut revoked = registration.capability.clone();
            revoked.capability_id = "z-embedded-id".to_string();
            revoked.state = ServiceProfileCapabilityState::Revoked;
            let mut active = registration.capability;
            active.capability_id = "a-embedded-id".to_string();
            state.service_principals.profile_capabilities = BTreeMap::from([
                ("a-key".to_string(), revoked.clone()),
                ("z-key".to_string(), active.clone()),
            ]);
            let before = state.clone();
            assert_eq!(state.profile_capability("a-key"), Some(&revoked));
            assert_eq!(
                state.profile_capabilities().collect::<Vec<_>>(),
                vec![&revoked, &active]
            );
            assert_eq!(state, before);
        }

        #[test]
        fn authentication_and_current_authority_keep_distinct_provenance_rules() {
            let (mut state, registration) = registered();
            let authority = authenticated(&state);
            assert!(state.authenticated_authority_is_current(&authority));
            let mut stale = authority.clone();
            stale.capability_revision += 1;
            assert!(!state.authenticated_authority_is_current(&stale));
            let mut unproven = authority.clone();
            unproven.provenance = ServicePrincipalProvenance::UnprovenLegacy;
            assert!(!state.authenticated_authority_is_current(&unproven));
            state
                .service_principals
                .principals
                .get_mut(&registration.principal.principal_id)
                .unwrap()
                .provenance = ServicePrincipalProvenance::UnprovenLegacy;
            assert_eq!(authenticated(&state), authority);
            assert!(!state.authenticated_authority_is_current(&authority));
        }

        #[test]
        fn authentication_preserves_failure_precedence() {
            let (mut state, registration) = registered();
            assert_eq!(
                state
                    .authenticate_profile_capability(" ", None)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::CapabilityMissing
            );
            assert_eq!(
                state
                    .authenticate_profile_capability("unknown", None)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::CapabilityMismatch
            );
            assert_eq!(
                state
                    .authenticate_profile_capability(TOKEN, Some("other"))
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::ProfileMismatch
            );
            let mut duplicate = registration.capability.clone();
            duplicate.capability_id = "duplicate".to_string();
            state
                .service_principals
                .profile_capabilities
                .insert("duplicate".to_string(), duplicate);
            assert_eq!(
                state
                    .authenticate_profile_capability(TOKEN, None)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::CapabilityMismatch
            );
            state
                .service_principals
                .profile_capabilities
                .remove("duplicate");
            state
                .service_principals
                .principals
                .get_mut("principal:test")
                .unwrap()
                .state = ServicePrincipalState::Suspended;
            assert_eq!(
                state
                    .authenticate_profile_capability(TOKEN, Some("other"))
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::ProfileMismatch
            );
            assert_eq!(
                state
                    .authenticate_profile_capability(TOKEN, None)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::PrincipalUnavailable
            );
            state
                .service_principals
                .profile_capabilities
                .get_mut(&registration.capability.capability_id)
                .unwrap()
                .state = ServiceProfileCapabilityState::Revoked;
            assert_eq!(
                state
                    .authenticate_profile_capability(TOKEN, Some("other"))
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::CapabilityRevoked
            );
        }

        #[test]
        fn registration_replays_and_conflicts_without_partial_state_or_secrets() {
            let (mut state, first) = registered();
            let before = state.clone();
            assert_eq!(
                state.register_profile_capability(request(), TOKEN).unwrap(),
                first
            );
            assert_eq!(state, before);
            assert_eq!(
                state
                    .register_profile_capability(request(), REPLACEMENT)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::RegistrationConflict
            );
            assert_eq!(state, before);
            assert_eq!(
                state
                    .register_profile_capability(request(), "short")
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::InvalidRegistration
            );
            assert_eq!(state, before);
            let wire = serde_json::to_string(&state).unwrap();
            assert!(!wire.contains(TOKEN));
            assert!(!wire.contains(REPLACEMENT));
            assert!(wire.contains("sha256:"));
            assert_eq!(state.state_revision(), 0);
        }

        #[test]
        fn rotation_preserves_cas_rollback_revocation_and_revisions() {
            let (mut state, registration) = registered();
            let authority = authenticated(&state);
            let revision = state.service_principal_registry_revision();
            let before = state.clone();
            let id = &registration.capability.capability_id;
            assert_eq!(
                state
                    .rotate_profile_capability(request(), "wrong", revision + 1, "short")
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::RegistryRevisionMismatch
            );
            assert_eq!(state, before);
            assert_eq!(
                state
                    .rotate_profile_capability(request(), "wrong", revision, REPLACEMENT)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::CapabilityRotationConflict
            );
            assert_eq!(state, before);
            assert_eq!(
                state
                    .rotate_profile_capability(request(), id, revision, "short")
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::InvalidRegistration
            );
            assert_eq!(state, before);
            let rotated = state
                .rotate_profile_capability(request(), id, revision, REPLACEMENT)
                .unwrap();
            assert_eq!(rotated.registry_revision, revision + 2);
            assert_eq!(state.service_principal_registry_revision(), revision + 2);
            assert_eq!(
                rotated.previous_capability.revision,
                registration.capability.revision + 1
            );
            assert_eq!(
                state.profile_capability(id),
                Some(&rotated.previous_capability)
            );
            assert_eq!(
                rotated.previous_capability.state,
                ServiceProfileCapabilityState::Revoked
            );
            assert!(!state.authenticated_authority_is_current(&authority));
            assert_eq!(
                state
                    .authenticate_profile_capability(TOKEN, None)
                    .unwrap_err()
                    .code,
                ServicePrincipalFailureCode::CapabilityRevoked
            );
            let replacement = state
                .authenticate_profile_capability(REPLACEMENT, None)
                .unwrap();
            assert!(state.authenticated_authority_is_current(&replacement));
            assert_eq!(
                replacement.capability_id,
                rotated.registered.capability.capability_id
            );
            assert_eq!(state.state_revision(), before.state_revision());
            assert_eq!(state.runtime_owner_registry, before.runtime_owner_registry);
        }

        #[test]
        fn authority_view_uses_current_snapshot_principal_without_loading_trust_keys() {
            use agent_browser_lease_authority::{
                issue_lease_effect_authorization, AcquireLeaseClaimRequest, LeaseClaimMode,
                LeaseEffectIntent, LeaseResourceKey,
            };
            let (mut state, registration) = registered();
            let authority = authenticated(&state);
            let claim = state
                .acquire_lease_claim(AcquireLeaseClaimRequest {
                    resource: LeaseResourceKey::profile("profile:test"),
                    parent_claim_id: None,
                    principal_id: authority.principal_id.clone(),
                    capability_id: authority.capability_id.clone(),
                    capability_revision: authority.capability_revision,
                    mode: LeaseClaimMode::Ephemeral,
                    expected_claim_revision: 0,
                    idempotency_key: "acquire:test".to_string(),
                    now: NOW.to_string(),
                    expires_at: "2026-09-17T12:05:00Z".to_string(),
                    transition_deadline: None,
                    recovery_controller_id: None,
                    boot_epoch: Some("synthetic-boot".to_string()),
                    owner_generation: None,
                })
                .unwrap();
            let intent = LeaseEffectIntent {
                action_class: "browser_launch".to_string(),
                audience: "session:test".to_string(),
                operation_idempotency_key: "launch:test".to_string(),
                executor_identity_digest: None,
                issued_at: NOW.to_string(),
                authorization_expires_at: "2026-09-17T12:02:00Z".to_string(),
            };
            let before = state.clone();
            // Rejection occurs during authentication, before the kernel loads keys.
            assert_eq!(
                issue_lease_effect_authorization(
                    &state.lease_authority_view(),
                    &claim,
                    &intent,
                    b"wrong"
                )
                .unwrap_err(),
                "lease_authority_capability_mismatch"
            );
            assert_eq!(state, before);
            state
                .service_principals
                .profile_capabilities
                .get_mut(&registration.capability.capability_id)
                .unwrap()
                .state = ServiceProfileCapabilityState::Revoked;
            assert!(!state.authenticated_authority_is_current(&authority));
            assert_eq!(
                issue_lease_effect_authorization(
                    &state.lease_authority_view(),
                    &claim,
                    &intent,
                    TOKEN.as_bytes()
                )
                .unwrap_err(),
                "lease_authority_capability_revoked"
            );
        }
    }

    fn reconcile_receipt_fixture() -> ProfileLeaseReconcileReceipt {
        ProfileLeaseReconcileReceipt {
            schema_version: PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION.to_string(),
            idempotency_key: "reconcile-key".to_string(),
            plan_id: "plan".to_string(),
            lease_id: "lease".to_string(),
            principal_id: "principal".to_string(),
            applied_at: "2026-09-17T12:00:00Z".to_string(),
            replayed: false,
            transition_count: 2,
            resulting_lease_revision: "lease-revision".to_string(),
        }
    }

    fn recovery_receipt_fixture() -> RecoveryReceipt {
        RecoveryReceipt {
            schema_version: PROFILE_RECOVERY_RECEIPT_SCHEMA_V1.to_string(),
            recovery_id: "recovery".to_string(),
            plan_id: "plan".to_string(),
            principal_id: "principal".to_string(),
            profile_id: "profile".to_string(),
            producer_build_identity: Some(json!({"sourceRevision": "build"})),
            terminal_result: "applied".to_string(),
            precondition_comparison: "matched".to_string(),
            attempted_operation_ids: vec!["operation".to_string()],
            compensation_result: "not_required".to_string(),
            final_state_revision: 12,
            acquisition_retry_state: ProfileAcquisitionState::Acquired,
            browser_id: "browser".to_string(),
            daemon_session_route: "route".to_string(),
        }
    }

    fn recovery_receipt_identity(receipt: &RecoveryReceipt) -> ProfileRecoveryReceiptIdentity<'_> {
        ProfileRecoveryReceiptIdentity {
            recovery_id: &receipt.recovery_id,
            plan_id: &receipt.plan_id,
            principal_id: &receipt.principal_id,
            profile_id: &receipt.profile_id,
            producer_build_identity: receipt.producer_build_identity.as_ref().unwrap(),
        }
    }

    fn reset_receipt_fixture() -> ProfileResetReceipt {
        ProfileResetReceipt {
            schema_version: PROFILE_RESET_RECEIPT_SCHEMA_V1.to_string(),
            reset_id: "reset".to_string(),
            plan_id: "plan".to_string(),
            principal_id: "principal".to_string(),
            profile_id: "profile".to_string(),
            producer_build_identity: json!({"sourceRevision": "build"}),
            scope: ProfileResetScope::Authentication,
            target_service_id: Some("target".to_string()),
            terminal_result: "applied".to_string(),
            applied_at: "2026-09-17T12:00:00Z".to_string(),
            final_state_revision: 42,
            browser_cookies_erased: false,
            seeding_handoff: None,
        }
    }

    fn reset_receipt_identity(receipt: &ProfileResetReceipt) -> ProfileResetReceiptIdentity<'_> {
        ProfileResetReceiptIdentity {
            reset_id: &receipt.reset_id,
            plan_id: &receipt.plan_id,
            principal_id: &receipt.principal_id,
            profile_id: &receipt.profile_id,
            producer_build_identity: &receipt.producer_build_identity,
            scope: receipt.scope,
            target_service_id: receipt.target_service_id.as_deref(),
        }
    }

    #[test]
    fn profile_receipt_lookup_misses_do_not_change_state() {
        let state = ServiceState::default();
        let original = state.clone();
        assert_eq!(
            state.replay_profile_lease_reconciliation("missing", "principal"),
            Ok(None)
        );
        assert_eq!(state.profile_recovery_receipt("missing"), None);
        assert_eq!(
            state.replay_profile_recovery(recovery_receipt_identity(&recovery_receipt_fixture())),
            Ok(None)
        );
        assert_eq!(
            state.replay_profile_reset(reset_receipt_identity(&reset_receipt_fixture())),
            Ok(None)
        );
        assert_eq!(state, original);
    }

    #[test]
    fn profile_reconciliation_replay_matches_only_principal_and_marks_only_clone() {
        let mut state = ServiceState::default();
        let receipt = reconcile_receipt_fixture();
        state.record_profile_lease_reconciliation(receipt.clone());
        let original = state.clone();
        assert_eq!(
            state.replay_profile_lease_reconciliation(&receipt.idempotency_key, "foreign"),
            Err(ProfileReceiptReplayError::LeaseAuthorityMismatch)
        );
        let mut expected = receipt.clone();
        expected.replayed = true;
        assert_eq!(
            state.replay_profile_lease_reconciliation(
                &receipt.idempotency_key,
                &receipt.principal_id
            ),
            Ok(Some(expected))
        );
        assert_eq!(state, original);
        assert!(!state.profile_lease_reconcile_receipts[&receipt.idempotency_key].replayed);

        // A persisted map key, rather than receipt metadata, selects replay.
        let mut legacy = receipt.clone();
        legacy.idempotency_key = "different-embedded-key".to_string();
        legacy.plan_id.clear();
        legacy.lease_id.clear();
        state
            .profile_lease_reconcile_receipts
            .insert(receipt.idempotency_key.clone(), legacy.clone());
        legacy.replayed = true;
        assert_eq!(
            state.replay_profile_lease_reconciliation(
                &receipt.idempotency_key,
                &receipt.principal_id
            ),
            Ok(Some(legacy))
        );
    }

    #[test]
    fn profile_recovery_receipt_replay_preserves_exact_identity_predicates() {
        let mut state = ServiceState::default();
        let receipt = recovery_receipt_fixture();
        state.record_profile_recovery(receipt.clone());
        assert_eq!(
            state.profile_recovery_receipt(&receipt.recovery_id),
            Some(&receipt)
        );
        let original = state.clone();
        assert_eq!(
            state.replay_profile_recovery(recovery_receipt_identity(&receipt)),
            Ok(Some(receipt.clone()))
        );
        assert_eq!(state, original);
        for (field, value) in [
            ("planId", json!("other")),
            ("recoveryId", json!("other")),
            ("principalId", json!("other")),
            ("profileId", json!("other")),
            ("producerBuildIdentity", json!({"sourceRevision": "other"})),
            ("terminalResult", json!("failed")),
        ] {
            let mut wire = serde_json::to_value(&receipt).unwrap();
            wire[field] = value;
            state.profile_recovery_receipts.insert(
                receipt.recovery_id.clone(),
                serde_json::from_value(wire).unwrap(),
            );
            let before = state.clone();
            assert_eq!(
                state.replay_profile_recovery(recovery_receipt_identity(&receipt)),
                Err(ProfileReceiptReplayError::RecoveryReceiptConflict),
                "{field} must match"
            );
            assert_eq!(state, before);
        }
        let mut legacy = serde_json::to_value(&receipt).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("producerBuildIdentity");
        state.record_profile_recovery(serde_json::from_value(legacy).unwrap());
        assert_eq!(
            state.replay_profile_recovery(recovery_receipt_identity(&receipt)),
            Err(ProfileReceiptReplayError::RecoveryReceiptConflict)
        );
    }

    #[test]
    fn profile_reset_receipt_replay_preserves_exact_identity_predicates() {
        let mut state = ServiceState::default();
        let receipt = reset_receipt_fixture();
        state.record_profile_reset(receipt.clone());
        let original = state.clone();
        assert_eq!(
            state.replay_profile_reset(reset_receipt_identity(&receipt)),
            Ok(Some(receipt.clone()))
        );
        assert_eq!(state, original);
        for (field, value) in [
            ("planId", json!("other")),
            ("resetId", json!("other")),
            ("principalId", json!("other")),
            ("profileId", json!("other")),
            ("producerBuildIdentity", json!({"sourceRevision": "other"})),
            ("scope", json!("runtime")),
            ("targetServiceId", json!("other")),
            ("targetServiceId", Value::Null),
            ("terminalResult", json!("failed")),
        ] {
            let mut wire = serde_json::to_value(&receipt).unwrap();
            wire[field] = value;
            state.profile_reset_receipts.insert(
                receipt.reset_id.clone(),
                serde_json::from_value(wire).unwrap(),
            );
            let before = state.clone();
            assert_eq!(
                state.replay_profile_reset(reset_receipt_identity(&receipt)),
                Err(ProfileReceiptReplayError::ResetReceiptConflict),
                "{field} must match"
            );
            assert_eq!(state, before);
        }
        let mut untargeted = receipt;
        untargeted.scope = ProfileResetScope::Runtime;
        untargeted.target_service_id = None;
        state.record_profile_reset(untargeted.clone());
        assert_eq!(
            state.replay_profile_reset(reset_receipt_identity(&untargeted)),
            Ok(Some(untargeted.clone()))
        );
    }

    #[test]
    fn profile_receipt_recording_derives_keys_and_replaces_without_revision_changes() {
        let mut state = ServiceState::default();
        let revision = state.state_revision();
        let owner_revision = state.runtime_owner_registry.revision();
        let mut lease = reconcile_receipt_fixture();
        let mut recovery = recovery_receipt_fixture();
        let mut reset = reset_receipt_fixture();
        state.record_profile_lease_reconciliation(lease.clone());
        state.record_profile_recovery(recovery.clone());
        state.record_profile_reset(reset.clone());
        // Changed identity still replaces: insertion does not add a conflict gate.
        lease.principal_id = "replacement".to_string();
        recovery.principal_id = "replacement".to_string();
        reset.principal_id = "replacement".to_string();
        state.record_profile_lease_reconciliation(lease.clone());
        state.record_profile_recovery(recovery.clone());
        state.record_profile_reset(reset.clone());
        assert_eq!(
            state.profile_lease_reconcile_receipts,
            BTreeMap::from([(lease.idempotency_key.clone(), lease)])
        );
        assert_eq!(
            state.profile_recovery_receipts,
            BTreeMap::from([(recovery.recovery_id.clone(), recovery)])
        );
        assert_eq!(
            state.profile_reset_receipts,
            BTreeMap::from([(reset.reset_id.clone(), reset)])
        );
        assert_eq!(state.state_revision(), revision);
        assert_eq!(state.runtime_owner_registry.revision(), owner_revision);
    }

    #[test]
    fn current_lease_claim_requires_the_exact_resource_and_unexpired_claim() {
        use agent_browser_lease_authority::{
            AcquireLeaseClaimRequest, LeaseClaimMode, LeaseResourceKey,
        };

        let mut state = ServiceState::default();
        let resource = LeaseResourceKey::profile("profile-1");
        let now = "2026-09-17T12:00:00Z";
        let expires_at = "2026-09-17T12:05:00Z";
        assert!(state.current_lease_claim(&resource, now).is_none());
        let claim = state
            .acquire_lease_claim(AcquireLeaseClaimRequest {
                resource: resource.clone(),
                parent_claim_id: None,
                principal_id: "principal-1".to_string(),
                capability_id: "capability-1".to_string(),
                capability_revision: 1,
                mode: LeaseClaimMode::Ephemeral,
                expected_claim_revision: 0,
                idempotency_key: "acquire-1".to_string(),
                now: now.to_string(),
                expires_at: expires_at.to_string(),
                transition_deadline: None,
                recovery_controller_id: None,
                boot_epoch: None,
                owner_generation: None,
            })
            .unwrap();
        let before = state.clone();
        assert_eq!(state.current_lease_claim(&resource, now), Some(&claim));
        assert!(state
            .current_lease_claim(&LeaseResourceKey::profile("profile-2"), now)
            .is_none());
        assert!(state.current_lease_claim(&resource, expires_at).is_none());
        assert!(state.current_lease_claim(&resource, "invalid").is_none());
        assert_eq!(state, before);
    }

    #[test]
    fn lease_claim_replay_misses_and_schema_errors_do_not_mutate_state() {
        use agent_browser_lease_authority::{
            LeaseAuthorityError, RecoverLeaseClaimRequest, ReleaseLeaseClaimRequest,
            RevokeLeaseClaimRequest,
        };

        // Deliberately unverified wire envelopes exercise only receipt lookup and
        // schema rejection. They never reach signature verification or key I/O.
        let common = json!({
            "schemaVersion": "unverified-fixture",
            "signingKeyId": "unverified-fixture",
            "signingKeyEpoch": 1,
            "resource": { "kind": "profile", "id": "profile-1" },
            "claimId": "claim-1",
            "principalId": "principal-1",
            "claimRevision": 1,
            "fencingToken": 1,
            "issuedAt": "2026-09-17T12:00:00Z",
            "authorizationExpiresAt": "2026-09-17T12:01:00Z",
            "proof": "unverified-fixture"
        });
        let mut release = common.clone();
        release.as_object_mut().unwrap().extend(
            json!({
                "capabilityId": "capability-1",
                "capabilityRevision": 1,
                "ownerGeneration": null,
                "actionClass": "lease_release",
                "audience": "lease_authority_kernel",
                "operationIdempotencyKey": "release-1"
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let release = ReleaseLeaseClaimRequest {
            authorization: serde_json::from_value(release).unwrap(),
            idempotency_key: "release-1".to_string(),
            now: "2026-09-17T12:00:30Z".to_string(),
        };
        let mut recovery = common.clone();
        recovery.as_object_mut().unwrap().extend(
            json!({
                "recoveryControllerId": "controller-1",
                "recoveryControllerRevision": 1,
                "idempotencyKey": "recover-1",
                "claimExpiresAt": "2026-09-17T12:05:00Z",
                "transitionDeadline": "2026-09-17T12:02:00Z",
                "ownerGeneration": null
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let recovery = RecoverLeaseClaimRequest {
            authorization: serde_json::from_value(recovery).unwrap(),
            now: release.now.clone(),
        };
        let mut revocation = common;
        revocation.as_object_mut().unwrap().extend(
            json!({
                "administratorId": "administrator-1",
                "administratorRevision": 1,
                "idempotencyKey": "revoke-1",
                "reasonCode": "test-revocation"
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let revocation = RevokeLeaseClaimRequest {
            authorization: serde_json::from_value(revocation).unwrap(),
            now: release.now.clone(),
        };
        let state = ServiceState::default();
        let before = state.clone();
        assert_eq!(state.replay_lease_claim_release(&release), Ok(None));
        assert_eq!(state.replay_lease_claim_recovery(&recovery), Ok(None));
        assert_eq!(state.replay_lease_claim_revocation(&revocation), Ok(None));
        assert_eq!(state, before);

        let mut unsupported: ServiceState = serde_json::from_value(json!({
            "leaseAuthority": {
                "schemaVersion": "unsupported",
                "revision": 1,
                "nextFencingTokens": { "profile:profile-1": 1 }
            }
        }))
        .unwrap();
        let before = unsupported.clone();
        assert_eq!(
            unsupported.replay_lease_claim_release(&release),
            Err(LeaseAuthorityError::UnsupportedSchema)
        );
        assert_eq!(
            unsupported.replay_lease_claim_recovery(&recovery),
            Err(LeaseAuthorityError::UnsupportedSchema)
        );
        assert_eq!(
            unsupported.replay_lease_claim_revocation(&revocation),
            Err(LeaseAuthorityError::UnsupportedSchema)
        );
        let expected_error = "lease_authority_unsupported_schema".to_string();
        assert_eq!(
            unsupported.release_lease_claim(release),
            Err(expected_error.clone())
        );
        assert_eq!(
            unsupported.recover_lease_claim(recovery),
            Err(expected_error.clone())
        );
        assert_eq!(
            unsupported.revoke_lease_claim(revocation),
            Err(expected_error)
        );
        assert_eq!(unsupported, before);
    }

    fn challenge_task_state() -> ServiceState {
        ServiceState {
            profiles: BTreeMap::from([(
                "profile-1".to_string(),
                BrowserProfile {
                    id: "profile-1".to_string(),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("profile-1".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-1".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    service_name: Some("consumer-service".to_string()),
                    agent_name: Some("challenge-worker".to_string()),
                    task_name: Some("challenge-aware-task".to_string()),
                    profile_id: Some("profile-1".to_string()),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["browser-1".to_string()],
                    tab_ids: vec!["tab-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-1".to_string(),
                BrowserTab {
                    id: "tab-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    target_id: Some("target-1".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    owner_session_id: Some("session-1".to_string()),
                    profile_access: Some(ProfileChildAccess {
                        subject_id: Some("principal-1".to_string()),
                        ..ProfileChildAccess::default()
                    }),
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        }
    }

    fn challenge_task_start_input(
        current_service_tab_handle: ServiceTabHandle,
        idempotency_key: &str,
    ) -> ServiceChallengeTaskStartInput {
        ServiceChallengeTaskStartInput {
            service_name: "consumer-service".to_string(),
            agent_name: "challenge-worker".to_string(),
            task_name: "challenge-aware-task".to_string(),
            principal_id: "principal-1".to_string(),
            challenge_profile_id: "turnstile-checkbox-p169-v1".to_string(),
            site_policy_digest: "a".repeat(64),
            downstream_intent_id: "authenticate-account".to_string(),
            fixture: agent_browser_challenge_control::ChallengeTaskFixture::PassAfterAcknowledgedResolution,
            idempotency_key: idempotency_key.to_string(),
            deadline_ms: 120_000,
            max_transitions: 8,
            current_service_tab_handle,
            created_at: "2026-09-16T00:00:00Z".to_string(),
        }
    }

    fn start_challenge_task(
        state: &mut ServiceState,
        idempotency_key: &str,
    ) -> ServiceChallengeTaskRecord {
        let input =
            challenge_task_start_input(state.service_tab_handle("tab-1").unwrap(), idempotency_key);
        let (record, replayed) = state.start_service_challenge_task(input).unwrap();
        assert!(!replayed);
        record
    }

    #[test]
    fn challenge_task_start_replays_or_rejects_conflicting_requests() {
        let mut state = challenge_task_state();
        let input = challenge_task_start_input(
            state.service_tab_handle("tab-1").unwrap(),
            "start-replay-key",
        );
        let (created, replayed) = state.start_service_challenge_task(input.clone()).unwrap();
        assert!(!replayed);
        assert_eq!(
            state.service_challenge_task(&created.task_id),
            Some(&created)
        );

        let (replayed_record, replayed) = state.start_service_challenge_task(input).unwrap();
        assert!(replayed);
        assert_eq!(replayed_record, created);
        assert_eq!(state.service_challenge_task_summary().total_count, 1);

        let mut conflicting = challenge_task_start_input(
            state.service_tab_handle("tab-1").unwrap(),
            "start-replay-key",
        );
        conflicting.deadline_ms += 1;
        assert_eq!(
            state.start_service_challenge_task(conflicting).unwrap_err(),
            ServiceChallengeTaskError::IdempotencyConflict
        );
    }

    #[test]
    fn challenge_task_resume_requires_a_current_handle_after_preparation() {
        let mut state = challenge_task_state();
        let created = start_challenge_task(&mut state, "resume-key");
        let input = ServiceChallengeTaskResumeInput {
            task_id: created.task_id.clone(),
            principal_id: "principal-1".to_string(),
            operation_id: "resume-1".to_string(),
            resumed_at: "2026-09-16T00:01:00Z".to_string(),
        };
        let tab = state.tabs.remove("tab-1").unwrap();
        assert_eq!(
            state
                .resume_service_challenge_task(input.clone())
                .unwrap_err(),
            ServiceChallengeTaskError::ServiceTabHandleMissing
        );
        assert_eq!(
            state
                .service_challenge_task(&created.task_id)
                .unwrap()
                .state,
            ServiceChallengeTaskState::Ready
        );

        state.tabs.insert("tab-1".to_string(), tab);
        let (completed, replayed) = state.resume_service_challenge_task(input.clone()).unwrap();
        assert!(!replayed);
        assert_eq!(completed.state, ServiceChallengeTaskState::Completed);

        state.tabs.remove("tab-1");
        let mut replay_input = input;
        replay_input.resumed_at = "invalid".to_string();
        let (replayed_record, replayed) =
            state.resume_service_challenge_task(replay_input).unwrap();
        assert!(replayed);
        assert_eq!(replayed_record, completed);
    }

    #[test]
    fn challenge_task_cancel_replays_without_a_second_insert() {
        let mut state = challenge_task_state();
        let created = start_challenge_task(&mut state, "cancel-key");
        let input = ServiceChallengeTaskCancelInput {
            task_id: created.task_id.clone(),
            principal_id: "principal-1".to_string(),
            operation_id: "cancel-1".to_string(),
            cancelled_at: "2026-09-16T00:01:00Z".to_string(),
        };
        let (cancelled, replayed) = state.cancel_service_challenge_task(input.clone()).unwrap();
        assert!(!replayed);
        assert_eq!(cancelled.state, ServiceChallengeTaskState::Cancelled);

        let mut replay_input = input;
        replay_input.cancelled_at = "invalid".to_string();
        let (replayed_record, replayed) =
            state.cancel_service_challenge_task(replay_input).unwrap();
        assert!(replayed);
        assert_eq!(replayed_record, cancelled);
        assert_eq!(state.service_challenge_task_summary().total_count, 1);
    }

    #[test]
    fn challenge_task_status_enforces_the_record_principal() {
        let mut state = challenge_task_state();
        let created = start_challenge_task(&mut state, "status-key");
        assert_eq!(
            state
                .status_service_challenge_task(&created.task_id, "principal-1")
                .unwrap(),
            created
        );
        assert_eq!(
            state
                .status_service_challenge_task(&created.task_id, "principal-2")
                .unwrap_err(),
            ServiceChallengeTaskError::PrincipalMismatch
        );
    }

    #[test]
    fn challenge_task_summary_and_lookup_are_aggregate_accessors() {
        let mut state = challenge_task_state();
        let created = start_challenge_task(&mut state, "summary-key");
        assert_eq!(
            state.service_challenge_task(&created.task_id),
            Some(&created)
        );
        assert_eq!(
            state.service_challenge_task_summary(),
            ServiceChallengeTaskSummary {
                total_count: 1,
                active_count: 1,
                terminal_count: 0,
                cooldown_count: 0,
                intervention_count: 0,
                pending_effect_count: 0,
            }
        );
    }

    #[test]
    fn builtin_policy_inventory_and_default_seeding_url_are_canonical() {
        let policies = builtin_site_policies();
        assert_eq!(
            policies
                .iter()
                .map(|policy| policy.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "canva",
                "ups",
                "google",
                "gmail",
                "google_sheets",
                "microsoft"
            ]
        );
        assert_eq!(
            default_profile_seeding_url("google-login"),
            "https://accounts.google.com"
        );
        assert_eq!(default_profile_seeding_url("canva"), "about:blank");
    }

    #[test]
    fn persisted_codec_preserves_unknown_values_and_deterministic_bytes() {
        let raw = json!({
            "stateRevision": 41,
            "futureScalar": 7,
            "futureObject": {"retained": true},
            "futureArray": [null, "opaque", {"n": 3}],
            "futureNull": null
        });
        let state = decode_persisted_service_state_json(&raw.to_string()).unwrap();
        assert_eq!(state.state_revision(), 41);
        assert_eq!(state.schema_version, SERVICE_STATE_SCHEMA_VERSION);
        assert_eq!(
            state.profile_lease_schema_version,
            PROFILE_LEASE_SCHEMA_VERSION
        );
        assert_eq!(
            state.unknown_top_level_field_names(),
            vec!["futureArray", "futureNull", "futureObject", "futureScalar"]
        );
        let bytes = encode_prepared_service_state_pretty(&state).unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert!(!bytes.ends_with(b"\n\n"));
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        for key in state.unknown_top_level_field_names() {
            assert_eq!(value[&key], raw[&key]);
        }
        let decoded =
            decode_persisted_service_state_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(
            encode_prepared_service_state_pretty(&decoded).unwrap(),
            bytes
        );
    }

    #[test]
    fn persisted_codec_rejects_schema_changes_with_stable_error_identities() {
        for (raw, expected) in [
            (
                r#"{"schemaVersion":"future"}"#,
                "service_state_schema_unsupported:future",
            ),
            (
                r#"{"profileLeaseSchemaVersion":"future"}"#,
                "profile_lease_schema_unsupported:future",
            ),
        ] {
            assert_eq!(
                decode_persisted_service_state_json(raw)
                    .unwrap_err()
                    .to_string(),
                expected
            );
        }
        assert!(matches!(
            decode_persisted_service_state_json("{"),
            Err(ServiceStateCodecError::Json(_))
        ));
    }

    #[test]
    fn legacy_policy_materialization_and_transport_decode_remain_distinct() {
        let raw = json!({
            "stateRevision": 4,
            "profiles": {"shared": {"id": "shared", "sharedServiceIds": ["a", "b"]}}
        });
        let transport: ServiceState = serde_json::from_value(raw.clone()).unwrap();
        assert!(transport.profiles["shared"].access_policy.is_none());
        assert!(transport.schema_version.is_empty());
        let persisted = decode_persisted_service_state_json(&raw.to_string()).unwrap();
        assert_eq!(
            persisted.profiles["shared"]
                .access_policy
                .as_ref()
                .unwrap()
                .mode,
            ProfileAccessMode::SharedLocal
        );
        let report = persisted.profile_policy_migration.as_ref().unwrap();
        assert_eq!((report.source_revision, report.target_revision), (4, 5));
        assert_eq!(report.entries[0].classification, "shared-local-default");
        let mut prepared_again = persisted.clone();
        prepare_service_state_for_persistence(&mut prepared_again).unwrap();
        assert_eq!(prepared_again, persisted);
    }

    #[test]
    fn profile_policy_migration_projection_preserves_optional_borrow() {
        let report = ProfilePolicyMigrationReport {
            schema_version: PROFILE_POLICY_MIGRATION_SCHEMA_VERSION.to_string(),
            migration_id: "profile-policy-migration-test".to_string(),
            source_revision: 3,
            target_revision: 4,
            entries: Vec::new(),
            blocking_issue_count: 0,
        };
        let state = ServiceState {
            profile_policy_migration: Some(report.clone()),
            ..ServiceState::default()
        };

        assert_eq!(state.profile_policy_migration(), Some(&report));
        assert_eq!(ServiceState::default().profile_policy_migration(), None);
    }

    #[test]
    fn configured_input_constructs_and_marks_only_config_owned_entities() {
        let input = ConfiguredServiceStateInput {
            profiles: BTreeMap::from([(
                "profile-a".to_string(),
                BrowserProfile {
                    id: "profile-a".to_string(),
                    ..BrowserProfile::default()
                },
            )]),
            providers: BTreeMap::from([(
                "provider-a".to_string(),
                ServiceProvider {
                    id: "provider-a".to_string(),
                    ..ServiceProvider::default()
                },
            )]),
            default_browser_build: Some(BrowserBuild::StockChrome),
            ..ConfiguredServiceStateInput::default()
        };

        let state = ServiceState::from_configured_entities(input);

        assert_eq!(state.default_browser_build, Some(BrowserBuild::StockChrome));
        assert_eq!(
            state.profile_source("profile-a"),
            Some(ServiceEntitySource::Config)
        );
        assert_eq!(state.providers["provider-a"].id, "provider-a");
        assert!(state.jobs.is_empty());
    }

    #[test]
    fn ordinary_codec_does_not_add_invariant_or_derived_view_gates() {
        let mut state = decode_persisted_service_state_json(
            r#"{"browsers":{"browser":{"id":"browser","profileId":"missing"}}}"#,
        )
        .unwrap();
        assert_eq!(
            validate_service_state_invariants(&state).unwrap_err(),
            ServiceStateCodecError::Invariant(
                "service_state_browser_profile_missing:browser:missing".into()
            )
        );
        state
            .entity_sources
            .profiles
            .insert("source".into(), ServiceEntitySource::Config);
        let before = state.clone();
        let bytes = encode_prepared_service_state_pretty(&state).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(state, before);
        assert!(state.site_policies.is_empty());
        assert!(value.get("entitySources").is_none());
        assert_eq!(value["browsers"]["browser"]["profileId"], "missing");
    }

    #[test]
    fn revision_successor_and_noop_comparison_ignore_only_revision() {
        let mut baseline = ServiceState {
            state_revision: 41,
            ..ServiceState::default()
        };
        let mut candidate = baseline.checked_successor().unwrap();
        assert_eq!(baseline.state_revision(), 41);
        assert_eq!(candidate.state_revision(), 42);
        assert!(candidate.payload_eq_ignoring_revision(&baseline));
        candidate
            .unknown_fields
            .insert("future".into(), json!({"retained": true}));
        assert!(!candidate.payload_eq_ignoring_revision(&baseline));
        candidate.unknown_fields.clear();
        candidate
            .entity_sources
            .profiles
            .insert("profile".into(), ServiceEntitySource::Config);
        assert!(!candidate.payload_eq_ignoring_revision(&baseline));
        baseline.state_revision = u64::MAX;
        assert_eq!(
            baseline.checked_successor().unwrap_err(),
            ServiceStateCodecError::RevisionExhausted
        );
        assert_eq!(baseline.state_revision(), u64::MAX);
    }

    #[test]
    fn refresh_derived_views_preserves_incident_operator_metadata() {
        let mut state = ServiceState {
            incidents: vec![ServiceIncident {
                id: "browser-1".to_string(),
                acknowledged_at: Some("2026-04-22T00:09:00Z".to_string()),
                acknowledged_by: Some("operator".to_string()),
                acknowledgement_note: Some("Investigating".to_string()),
                resolved_at: Some("2026-04-22T00:10:00Z".to_string()),
                resolved_by: Some("operator".to_string()),
                resolution_note: Some("Recovered".to_string()),
                ..ServiceIncident::default()
            }],
            events: vec![ServiceEvent {
                id: "event-crash".to_string(),
                timestamp: "2026-04-22T00:02:00Z".to_string(),
                kind: ServiceEventKind::BrowserHealthChanged,
                message: "Browser browser-1 health changed from Ready to ProcessExited".to_string(),
                browser_id: Some("browser-1".to_string()),
                previous_health: Some(BrowserHealth::Ready),
                current_health: Some(BrowserHealth::ProcessExited),
                ..ServiceEvent::default()
            }],
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::ProcessExited,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_derived_views();

        assert_eq!(state.incidents.len(), 1);
        assert_eq!(
            state.incidents[0].acknowledged_at.as_deref(),
            Some("2026-04-22T00:09:00Z")
        );
        assert_eq!(state.incidents[0].resolved_by.as_deref(), Some("operator"));
        assert_eq!(
            state.incidents[0].resolution_note.as_deref(),
            Some("Recovered")
        );
        assert_eq!(state.incidents[0].state, ServiceIncidentState::Recovered);
        assert_eq!(state.incidents[0].current_health, None);
        assert_eq!(state.incidents[0].severity, ServiceIncidentSeverity::Info);
        assert_eq!(
            state.incidents[0].escalation,
            ServiceIncidentEscalation::None
        );
        assert_eq!(
            state.incidents[0].recommended_action,
            "No operator action required."
        );
    }

    #[test]
    fn configured_profile_preserves_persisted_freshness_evidence() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "Persisted BILL".to_string(),
                    allocation: ProfileAllocationPolicy::PerService,
                    target_service_ids: vec!["bill".to_string()],
                    authenticated_service_ids: vec!["bill".to_string()],
                    account_ids: vec!["soylei".to_string()],
                    target_readiness: vec![ProfileTargetReadiness {
                        target_service_id: "bill".to_string(),
                        state: ProfileReadinessState::Fresh,
                        evidence: "authenticated_bill_home".to_string(),
                        recommended_action: "use_profile".to_string(),
                        last_verified_at: Some("2026-09-09T16:40:40Z".to_string()),
                        ..ProfileTargetReadiness::default()
                    }],
                    shared_service_ids: vec![
                        "BooksReceipts".to_string(),
                        "books-receipts".to_string(),
                    ],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };
        let configured = ServiceState {
            profiles: BTreeMap::from([(
                "bill-soylei".to_string(),
                BrowserProfile {
                    id: "bill-soylei".to_string(),
                    name: "Configured BILL".to_string(),
                    allocation: ProfileAllocationPolicy::SharedService,
                    target_service_ids: vec!["bill".to_string()],
                    shared_service_ids: vec![
                        "BooksReceipts".to_string(),
                        "books-receipts".to_string(),
                    ],
                    ..BrowserProfile::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.overlay_configured_entities(configured);

        let profile = &state.profiles["bill-soylei"];
        assert_eq!(profile.name, "Configured BILL");
        assert_eq!(profile.allocation, ProfileAllocationPolicy::SharedService);
        assert_eq!(profile.account_ids, vec!["soylei"]);
        assert_eq!(profile.authenticated_service_ids, vec!["bill"]);
        assert_eq!(profile.target_readiness.len(), 1);
        assert_eq!(
            profile.target_readiness[0].evidence,
            "authenticated_bill_home"
        );
        assert_eq!(
            state.profile_source("bill-soylei"),
            Some(ServiceEntitySource::Config)
        );
    }

    #[test]
    fn refresh_service_tab_handles_derives_valid_and_stale_handles() {
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "profile-1".to_string(),
                BrowserProfile {
                    id: "profile-1".to_string(),
                    name: "Profile 1".to_string(),
                    profile_origin: ProfileOrigin::ExternalByop,
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("profile-1".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-1".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    service_name: Some("service-a".to_string()),
                    agent_name: Some("agent-a".to_string()),
                    task_name: Some("task-a".to_string()),
                    profile_id: Some("profile-1".to_string()),
                    lease: LeaseState::Exclusive,
                    cleanup: SessionCleanupPolicy::CloseTabs,
                    browser_ids: vec!["browser-1".to_string()],
                    tab_ids: vec!["tab-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([
                (
                    "tab-1".to_string(),
                    BrowserTab {
                        id: "tab-1".to_string(),
                        browser_id: "browser-1".to_string(),
                        target_id: Some("target-1".to_string()),
                        lifecycle: TabLifecycle::Ready,
                        url: Some("https://example.com".to_string()),
                        title: Some("Example".to_string()),
                        owner_session_id: Some("session-1".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-closed".to_string(),
                    BrowserTab {
                        id: "tab-closed".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Closed,
                        owner_session_id: Some("session-1".to_string()),
                        ..BrowserTab::default()
                    },
                ),
            ]),
            jobs: BTreeMap::from([(
                "job-tab".to_string(),
                ServiceJob {
                    id: "job-tab".to_string(),
                    target: JobTarget::Tab("tab-1".to_string()),
                    completed_at: Some("2026-06-13T12:00:00Z".to_string()),
                    ..ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };

        state.refresh_service_tab_handles();

        let handle = state.tabs["tab-1"].service_tab_handle.as_ref().unwrap();
        assert_eq!(handle.browser_id, "browser-1");
        assert_eq!(handle.session_name.as_deref(), Some("session-1"));
        assert_eq!(handle.tab_id, "tab-1");
        assert_eq!(handle.target_id.as_deref(), Some("target-1"));
        assert_eq!(handle.profile_id.as_deref(), Some("profile-1"));
        assert_eq!(handle.profile_origin, ProfileOrigin::ExternalByop);
        assert_eq!(handle.lease_state, Some(LeaseState::Exclusive));
        assert_eq!(handle.cleanup_policy, Some(SessionCleanupPolicy::CloseTabs));
        assert_eq!(handle.job_id.as_deref(), Some("job-tab"));
        assert_eq!(
            handle.trace_filter.service_name.as_deref(),
            Some("service-a")
        );
        assert_eq!(handle.trace_filter.agent_name.as_deref(), Some("agent-a"));
        assert_eq!(handle.trace_filter.task_name.as_deref(), Some("task-a"));
        assert!(handle.valid);
        assert_eq!(handle.stale_reason, None);
        assert_eq!(
            state.browsers["browser-1"].tab_handles[0],
            state.tabs["tab-1"].service_tab_handle.clone().unwrap()
        );

        let stale = state.tabs["tab-closed"]
            .service_tab_handle
            .as_ref()
            .unwrap();
        assert!(!stale.valid);
        assert_eq!(stale.stale_reason.as_deref(), Some("tab_closed"));
    }

    #[test]
    fn expire_stale_session_leases_preserves_browser_and_marks_handles_stale() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("shared-profile".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec![
                        "expired-session".to_string(),
                        "fresh-session".to_string(),
                        "invalid-session".to_string(),
                        "released-session".to_string(),
                    ],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([
                (
                    "expired-session".to_string(),
                    BrowserSession {
                        id: "expired-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Shared,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-expired".to_string()],
                        expires_at: Some("2026-06-19T22:44:59Z".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "fresh-session".to_string(),
                    BrowserSession {
                        id: "fresh-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Shared,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-fresh".to_string()],
                        expires_at: Some("2026-06-19T22:45:01Z".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "invalid-session".to_string(),
                    BrowserSession {
                        id: "invalid-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Exclusive,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-invalid".to_string()],
                        expires_at: Some("not-a-timestamp".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "released-session".to_string(),
                    BrowserSession {
                        id: "released-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Released,
                        browser_ids: vec!["browser-1".to_string()],
                        tab_ids: vec!["tab-released".to_string()],
                        expires_at: Some("2026-06-19T22:44:00Z".to_string()),
                        ..BrowserSession::default()
                    },
                ),
                (
                    "orphaned-session".to_string(),
                    BrowserSession {
                        id: "orphaned-session".to_string(),
                        profile_id: Some("shared-profile".to_string()),
                        lease: LeaseState::Exclusive,
                        browser_ids: vec!["missing-browser".to_string()],
                        ..BrowserSession::default()
                    },
                ),
            ]),
            tabs: BTreeMap::from([
                (
                    "tab-expired".to_string(),
                    BrowserTab {
                        id: "tab-expired".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("expired-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-fresh".to_string(),
                    BrowserTab {
                        id: "tab-fresh".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("fresh-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-invalid".to_string(),
                    BrowserTab {
                        id: "tab-invalid".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("invalid-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
                (
                    "tab-released".to_string(),
                    BrowserTab {
                        id: "tab-released".to_string(),
                        browser_id: "browser-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        owner_session_id: Some("released-session".to_string()),
                        ..BrowserTab::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let expired =
            state.expire_stale_session_leases("2026-06-19T22:45:00Z", Some("test-boot-epoch"));

        assert_eq!(
            expired,
            vec![
                "expired-session".to_string(),
                "orphaned-session".to_string()
            ]
        );
        assert_eq!(state.sessions["expired-session"].lease, LeaseState::Expired);
        assert_eq!(
            state.sessions["expired-session"].boot_epoch.as_deref(),
            Some("test-boot-epoch")
        );
        assert_eq!(
            state.sessions["orphaned-session"].lease,
            LeaseState::Expired
        );
        assert_eq!(
            state.sessions["expired-session"]
                .last_lease_observed_at
                .as_deref(),
            Some("2026-06-19T22:45:00Z")
        );
        assert_eq!(state.sessions["fresh-session"].lease, LeaseState::Shared);
        assert_eq!(
            state.sessions["invalid-session"].lease,
            LeaseState::Exclusive
        );
        assert_eq!(
            state.sessions["released-session"].lease,
            LeaseState::Released
        );
        assert_eq!(
            state.browsers["browser-1"].active_session_ids,
            vec![
                "fresh-session".to_string(),
                "invalid-session".to_string(),
                "released-session".to_string()
            ]
        );
        assert_eq!(state.browsers["browser-1"].health, BrowserHealth::Ready);
        assert_eq!(
            state.tabs["tab-expired"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .stale_reason
                .as_deref(),
            Some("lease_expired")
        );
        assert!(
            state.tabs["tab-fresh"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .valid
        );
        assert!(
            state.tabs["tab-invalid"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .valid
        );
        assert_eq!(
            state.tabs["tab-released"]
                .service_tab_handle
                .as_ref()
                .unwrap()
                .stale_reason
                .as_deref(),
            Some("lease_released")
        );
    }
}
