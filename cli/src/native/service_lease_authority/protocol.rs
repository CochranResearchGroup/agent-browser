use ring::signature::{self, Ed25519KeyPair};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::{
    AcquireLeaseClaimRequest, ActiveLeaseClaim, LeaseAdministrativeAuthorization,
    LeaseAdministrativeIntent, LeaseAdministratorAuthority, LeaseAuthorityEvent,
    LeaseAuthoritySigningKey, LeaseAuthorityState, LeaseAuthorityVerificationKeyring,
    LeaseClaimAcquisitionOutcome, LeaseClaimAcquisitionReceipt, LeaseClaimMode,
    LeaseClaimRecoveryReceipt, LeaseClaimReleaseOutcome, LeaseClaimTerminalReceipt,
    LeaseEffectAuthorization, LeaseEffectIntent, LeaseRecoveryAuthorization, LeaseRecoveryIntent,
    LeaseResourceKey, LeaseResourceKind, RecoverLeaseClaimRequest, ReleaseLeaseClaimRequest,
    RevokeLeaseClaimRequest,
};
use crate::native::service_principal::{
    authenticate_profile_capability, profile_capability_digest, register_profile_capability,
    ServicePrincipalRegistrationRequest, ServicePrincipalRegistry,
};

#[cfg(target_os = "linux")]
pub(super) mod client;
mod custody;
#[cfg(target_os = "linux")]
mod service;

#[cfg(target_os = "linux")]
pub(super) const LEASE_AUTHORITY_SERVICE_PROCESS_ENV: &str =
    service::LEASE_AUTHORITY_SERVICE_PROCESS_ENV;

#[cfg(target_os = "linux")]
pub(super) const LEASE_AUTHORITY_BOOTSTRAP_PROCESS_ENV: &str =
    service::LEASE_AUTHORITY_BOOTSTRAP_PROCESS_ENV;

#[cfg(target_os = "linux")]
pub(super) fn run_linux_lease_authority_service() -> Result<(), String> {
    service::run_linux_service().map_err(|error| error.code().to_string())
}

#[cfg(target_os = "linux")]
pub(super) fn run_linux_lease_authority_bootstrap() -> Result<(), String> {
    service::run_linux_bootstrap().map_err(|error| error.code().to_string())
}

const LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-request.v1";
const LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-response.v1";
const LEASE_AUTHORITY_SERVICE_MAX_FRAME_BYTES: usize = 64 * 1024;
const LEASE_AUTHORITY_PROTECTED_STATE_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-protected-state.v1";
const LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-resource-registry.v1";
const LEASE_AUTHORITY_DOMAIN_SCHEMA_VERSION: &str = "agent-browser.lease-authority-domain.v1";
const LEASE_AUTHORITY_OWNER_REGISTRY_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-owner-registry.v1";
const LEASE_AUTHORITY_HISTORY_SCHEMA_VERSION: &str = "agent-browser.lease-authority-history.v1";
const LEASE_AUTHORITY_STORE_GENERATION_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-store-generation.v1";
const LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION_V1: &str =
    "agent-browser.lease-authority-store-history-manifest.v1";
const LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-store-history-manifest.v2";
const LEASE_AUTHORITY_STORE_MAX_HISTORY_GENERATIONS: usize = 4096;
const LEASE_AUTHORITY_STORE_SELECTOR_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-store-selector.v1";
const LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY: &str = "generations";
const LEASE_AUTHORITY_STORE_PROTECTED_STATE_FILE: &str = "protected-state.v1.json";
const LEASE_AUTHORITY_STORE_HISTORY_FILE: &str = "history.v1.json";
const LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_FILE: &str = "history-manifest.v1.json";
const LEASE_AUTHORITY_STORE_MANIFEST_FILE: &str = "manifest.v1.json";
const LEASE_AUTHORITY_STORE_SELECTOR_FILE: &str = "selected-generation.v1.json";
const LEASE_AUTHORITY_SERVICE_IDENTITY_PROOF_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-service-identity-proof.v1";
const LEASE_AUTHORITY_PROFILE_ENROLLMENT_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-profile-enrollment-receipt.v1";
const LEASE_AUTHORITY_EFFECT_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-effect-receipt.v1";
const MAX_LEASE_AUTHORITY_EFFECT_RECEIPTS: usize = 4096;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityProtocolEnvelope {
    schema_version: String,
    operation: String,
    payload: Value,
}

#[derive(Deserialize, PartialEq, Eq)]
struct LeaseAuthoritySecret(Vec<u8>);

impl LeaseAuthoritySecret {
    fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for LeaseAuthoritySecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl Drop for LeaseAuthoritySecret {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AcquireLeaseAuthorityPayload {
    raw_capability: LeaseAuthoritySecret,
    resource: LeaseResourceKey,
    parent_claim_id: Option<String>,
    mode: LeaseClaimMode,
    #[serde(default)]
    expected_claim_revision: Option<u64>,
    idempotency_key: String,
    recovery_controller_id: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EnrollProfileLeaseAuthorityPayload {
    raw_capability: LeaseAuthoritySecret,
    profile_id: String,
    profile_path: String,
    expected_resource_revision: u64,
    idempotency_key: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReleaseLeaseAuthorityPayload {
    raw_capability: LeaseAuthoritySecret,
    resource: LeaseResourceKey,
    claim_id: String,
    claim_revision: u64,
    fencing_token: u64,
    idempotency_key: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthorizeLeaseEffectPayload {
    raw_capability: LeaseAuthoritySecret,
    resource: LeaseResourceKey,
    claim_id: String,
    claim_revision: u64,
    fencing_token: u64,
    action_class: String,
    audience: String,
    idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LeaseAuthorityEffectState {
    Consumed,
    Completed,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CompleteLeaseEffectResult {
    Completed,
    Uncertain,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteLeaseEffectPayload {
    receipt_id: String,
    result: CompleteLeaseEffectResult,
    completion_evidence_digest: String,
    completion_idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityEffectReceipt {
    schema_version: String,
    receipt_id: String,
    request_digest: String,
    idempotency_key: String,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    capability_id: String,
    capability_revision: u64,
    claim_revision: u64,
    fencing_token: u64,
    action_class: String,
    audience: String,
    executor_identity_digest: String,
    executor_uid: u32,
    authority_revision: u64,
    occurred_at: String,
    authorization_expires_at: String,
    state: LeaseAuthorityEffectState,
    completion_idempotency_key: Option<String>,
    completion_evidence_digest: Option<String>,
    completed_at: Option<String>,
    terminal_authority_revision: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityEffectRecord {
    receipt: LeaseAuthorityEffectReceipt,
    authorization: Option<LeaseEffectAuthorization>,
}

#[derive(Debug)]
struct LeaseAuthorityEffectOutcome {
    receipt: LeaseAuthorityEffectReceipt,
    authorization: Option<LeaseEffectAuthorization>,
    replayed: bool,
}

#[derive(Debug)]
struct LeaseAuthorityEffectCompletionOutcome {
    receipt: LeaseAuthorityEffectReceipt,
    replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityProfileEnrollmentReceipt {
    schema_version: String,
    enrollment_id: String,
    request_digest: String,
    idempotency_key: String,
    principal_id: String,
    capability_id: String,
    capability_revision: u64,
    profile_id: String,
    physical_identity_digest: String,
    resource_revision: u64,
    operator_uid: u32,
    occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseAuthorityProfileEnrollmentOutcome {
    receipt: LeaseAuthorityProfileEnrollmentReceipt,
    replayed: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevokeLeasePlanPayload {
    resource: LeaseResourceKey,
    claim_id: String,
    claim_revision: u64,
    fencing_token: u64,
    idempotency_key: String,
    reason_code: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevokeLeaseApplyPayload {
    plan_id: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoverLeasePlanPayload {
    raw_controller_capability: LeaseAuthoritySecret,
    resource: LeaseResourceKey,
    claim_id: String,
    claim_revision: u64,
    fencing_token: u64,
    idempotency_key: String,
    owner_generation: Option<u64>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoverLeaseApplyPayload {
    raw_controller_capability: LeaseAuthoritySecret,
    plan_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RecoverLeasePlanProjection {
    plan_id: String,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    recovery_controller_id: String,
    recovery_controller_revision: u64,
    claim_revision: u64,
    fencing_token: u64,
    issued_at: String,
    authorization_expires_at: String,
    claim_expires_at: String,
    transition_deadline: String,
    owner_generation: Option<u64>,
    replayed: bool,
}

impl RecoverLeasePlanProjection {
    fn from_outcome(outcome: super::LeaseRecoveryPlanOutcome) -> Self {
        let authorization = outcome.authorization;
        Self {
            plan_id: authorization.plan_id(),
            resource: authorization.resource.clone(),
            claim_id: authorization.claim_id.clone(),
            principal_id: authorization.principal_id.clone(),
            recovery_controller_id: authorization.recovery_controller_id.clone(),
            recovery_controller_revision: authorization.recovery_controller_revision,
            claim_revision: authorization.claim_revision,
            fencing_token: authorization.fencing_token,
            issued_at: authorization.issued_at.clone(),
            authorization_expires_at: authorization.authorization_expires_at.clone(),
            claim_expires_at: authorization.claim_expires_at.clone(),
            transition_deadline: authorization.transition_deadline.clone(),
            owner_generation: authorization.owner_generation,
            replayed: outcome.replayed,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RevokeLeasePlanProjection {
    plan_id: String,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    claim_revision: u64,
    fencing_token: u64,
    reason_code: String,
    issued_at: String,
    authorization_expires_at: String,
    replayed: bool,
}

impl RevokeLeasePlanProjection {
    fn from_outcome(outcome: super::LeaseAdministrativePlanOutcome) -> Self {
        let authorization = outcome.authorization;
        Self {
            plan_id: authorization.plan_id(),
            resource: authorization.resource.clone(),
            claim_id: authorization.claim_id.clone(),
            principal_id: authorization.principal_id.clone(),
            claim_revision: authorization.claim_revision,
            fencing_token: authorization.fencing_token,
            reason_code: authorization.reason_code.clone(),
            issued_at: authorization.issued_at.clone(),
            authorization_expires_at: authorization.authorization_expires_at.clone(),
            replayed: outcome.replayed,
        }
    }
}

struct LeaseAuthorityAdministrativeDispatchContext<'a> {
    administrator_id: &'a str,
    administrator_revision: u64,
    raw_administrator_capability: &'a [u8],
    authority_observed_at: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityServiceChallengeRequest {
    nonce: String,
    expected_authority_domain_id: String,
    minimum_authority_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityServiceIdentityProof {
    schema_version: String,
    authority_domain_id: String,
    authority_epoch: u64,
    boot_epoch: String,
    endpoint_identity_digest: String,
    executable_sha256: String,
    signing_key_id: String,
    signing_key_epoch: u64,
    nonce: String,
    proof: String,
}

#[derive(PartialEq, Eq)]
enum LeaseAuthorityProtocolRequest {
    ServiceChallenge(LeaseAuthorityServiceChallengeRequest),
    EnrollProfile(EnrollProfileLeaseAuthorityPayload),
    Acquire(Box<AcquireLeaseAuthorityPayload>),
    AuthorizeEffect(AuthorizeLeaseEffectPayload),
    CompleteEffect(CompleteLeaseEffectPayload),
    Release(ReleaseLeaseAuthorityPayload),
    RecoverPlan(RecoverLeasePlanPayload),
    Recover(RecoverLeaseApplyPayload),
    RevokePlan(RevokeLeasePlanPayload),
    Revoke(RevokeLeaseApplyPayload),
    Inspect(Value),
}

enum LeaseAuthorityProtocolResponse {
    Acquired(LeaseClaimAcquisitionOutcome),
}

struct LeaseAuthorityProtocolKernel {
    state: LeaseAuthorityProtectedState,
    selected_generation_id: Option<String>,
    publication_authorized: bool,
}

#[derive(Clone, Copy)]
struct LeaseAuthorityProtectedLoadContext<'a> {
    expected_authority_domain_id: &'a str,
    minimum_authority_epoch: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityDomainState {
    schema_version: String,
    authority_domain_id: String,
    authority_epoch: u64,
    boot_epoch: String,
    authority_time_floor: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityProtectedState {
    schema_version: String,
    domain: LeaseAuthorityDomainState,
    #[serde(with = "lease_authority_operational_state_serde")]
    authority: LeaseAuthorityState,
    principals: ServicePrincipalRegistry,
    resources: LeaseAuthorityResourceRegistry,
    owners: LeaseAuthorityOwnerRegistry,
    #[serde(default)]
    profile_enrollment_receipts: BTreeMap<String, LeaseAuthorityProfileEnrollmentReceipt>,
    #[serde(default)]
    effect_receipts: BTreeMap<String, LeaseAuthorityEffectRecord>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityResourceRegistration {
    resource: LeaseResourceKey,
    physical_identity_digest: String,
    revision: u64,
    operator_uid: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityResourceRegistry {
    schema_version: String,
    revision: u64,
    registrations: BTreeMap<String, LeaseAuthorityResourceRegistration>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityOwnerBinding {
    resource: LeaseResourceKey,
    physical_identity_digest: String,
    owner_id: String,
    owner_generation: u64,
    logical_browser_id: String,
    daemon_session_route: String,
    process_instance_digest: String,
    principal_id: String,
    capability_id: String,
    revision: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityOwnerRegistry {
    schema_version: String,
    revision: u64,
    bindings: BTreeMap<String, LeaseAuthorityOwnerBinding>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LeaseAuthorityHistoryRef<'a> {
    schema_version: &'static str,
    events: &'a [LeaseAuthorityEvent],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityHistoryState {
    schema_version: String,
    events: Vec<LeaseAuthorityEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityStoreGenerationManifest {
    schema_version: String,
    generation_id: String,
    authority_domain_id: String,
    authority_epoch: u64,
    authority_revision: u64,
    protected_state_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityStoreHistoryManifest {
    schema_version: String,
    generation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_generation_id: Option<String>,
    history_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityStoreSelector {
    schema_version: String,
    generation_id: String,
    authority_domain_id: String,
    authority_epoch: u64,
    authority_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeaseAuthorityPublicationFault {
    ProtectedStateWritten,
    HistoryWritten,
    ManifestsWritten,
    GenerationPublished,
}

struct LeaseAuthorityDurableStore {
    root: PathBuf,
}

impl LeaseAuthorityDurableStore {
    fn initialize(root: &Path) -> Result<Self, LeaseAuthorityProtocolError> {
        fs::create_dir_all(root).map_err(store_io_error)?;
        super::set_private_directory_permissions(root).map_err(store_io_error)?;
        let generations = root.join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY);
        fs::create_dir_all(&generations).map_err(store_io_error)?;
        super::set_private_directory_permissions(&generations).map_err(store_io_error)?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    fn open_existing(root: &Path) -> Result<Self, LeaseAuthorityProtocolError> {
        super::ensure_private_directory(root).map_err(store_io_error)?;
        super::ensure_private_directory(&root.join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY))
            .map_err(store_io_error)?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    fn publish(
        &self,
        kernel: &LeaseAuthorityProtocolKernel,
        fault: Option<LeaseAuthorityPublicationFault>,
    ) -> Result<(), LeaseAuthorityProtocolError> {
        let lock_path = self.root.join("selection.lock");
        let selection_lock = super::open_private_lock_file(&lock_path).map_err(store_io_error)?;
        selection_lock
            .lock()
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_lock_failed",
            })?;
        let selected_generation_id = self.selected_generation_id()?;
        if selected_generation_id.as_deref() != kernel.selected_generation_id.as_deref() {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_stale_publication",
            });
        }
        if selected_generation_id.is_some() && !kernel.publication_authorized {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_mutation_load_required",
            });
        }

        let protected_state = kernel.encode_protected_state()?;
        let history = kernel.encode_history_state()?;
        let protected_state_sha256 = private_json_file_sha256(&protected_state);
        let history_sha256 = private_json_file_sha256(&history);
        let authority_domain_id = kernel.state.domain.authority_domain_id.clone();
        let authority_epoch = kernel.state.domain.authority_epoch;
        let authority_revision = kernel.state.authority.revision;
        let generation_id = authority_store_generation_id(
            authority_epoch,
            authority_revision,
            &protected_state_sha256,
        )?;
        let generations = self.root.join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY);
        let final_path = generations.join(&generation_id);
        let manifest = LeaseAuthorityStoreGenerationManifest {
            schema_version: LEASE_AUTHORITY_STORE_GENERATION_SCHEMA_VERSION.to_string(),
            generation_id: generation_id.clone(),
            authority_domain_id: authority_domain_id.clone(),
            authority_epoch,
            authority_revision,
            protected_state_sha256,
        };
        let history_manifest = LeaseAuthorityStoreHistoryManifest {
            schema_version: LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION.to_string(),
            generation_id: generation_id.clone(),
            previous_generation_id: selected_generation_id.clone(),
            history_sha256,
        };

        if !final_path.exists() {
            let temporary =
                generations.join(format!(".{generation_id}.{}.tmp", uuid::Uuid::new_v4()));
            fs::create_dir(&temporary).map_err(store_io_error)?;
            super::set_private_directory_permissions(&temporary).map_err(store_io_error)?;
            let staged = (|| {
                super::write_private_signing_key_file(
                    &temporary.join(LEASE_AUTHORITY_STORE_PROTECTED_STATE_FILE),
                    &protected_state,
                )
                .map_err(store_io_error)?;
                inject_publication_fault(
                    fault,
                    LeaseAuthorityPublicationFault::ProtectedStateWritten,
                )?;
                super::write_private_signing_key_file(
                    &temporary.join(LEASE_AUTHORITY_STORE_HISTORY_FILE),
                    &history,
                )
                .map_err(store_io_error)?;
                inject_publication_fault(fault, LeaseAuthorityPublicationFault::HistoryWritten)?;
                let history_manifest_encoded = serde_json::to_vec_pretty(&history_manifest)
                    .map_err(|_| LeaseAuthorityProtocolError {
                        code: "lease_authority_protocol_store_history_manifest_encode_failed",
                    })?;
                super::write_private_signing_key_file(
                    &temporary.join(LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_FILE),
                    &history_manifest_encoded,
                )
                .map_err(store_io_error)?;
                let manifest_encoded = serde_json::to_vec_pretty(&manifest).map_err(|_| {
                    LeaseAuthorityProtocolError {
                        code: "lease_authority_protocol_store_manifest_encode_failed",
                    }
                })?;
                super::write_private_signing_key_file(
                    &temporary.join(LEASE_AUTHORITY_STORE_MANIFEST_FILE),
                    &manifest_encoded,
                )
                .map_err(store_io_error)?;
                inject_publication_fault(fault, LeaseAuthorityPublicationFault::ManifestsWritten)?;
                super::sync_authority_key_directory(&temporary).map_err(store_io_error)?;
                fs::rename(&temporary, &final_path).map_err(store_io_error)?;
                super::sync_authority_key_directory(&generations).map_err(store_io_error)
            })();
            if staged.is_err() && temporary.exists() {
                let _ = fs::remove_dir_all(&temporary);
            }
            staged?;
        }
        self.validate_generation(&final_path, &manifest)?;
        self.validate_history_generation(&final_path, &history_manifest)?;

        if fault == Some(LeaseAuthorityPublicationFault::GenerationPublished) {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_publication_fault_injected",
            });
        }

        super::write_private_json_atomic_replace(
            &self.root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE),
            &LeaseAuthorityStoreSelector {
                schema_version: LEASE_AUTHORITY_STORE_SELECTOR_SCHEMA_VERSION.to_string(),
                generation_id,
                authority_domain_id,
                authority_epoch,
                authority_revision,
            },
        )
        .map_err(store_io_error)
    }

    fn load(
        &self,
        context: LeaseAuthorityProtectedLoadContext<'_>,
    ) -> Result<LeaseAuthorityProtocolKernel, LeaseAuthorityProtocolError> {
        let selector: LeaseAuthorityStoreSelector = super::load_private_json_file(
            &self.root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE),
            "lease_authority_protocol_store_selector_decode_failed",
        )
        .map_err(store_io_error)?;
        if selector.schema_version != LEASE_AUTHORITY_STORE_SELECTOR_SCHEMA_VERSION
            || selector.authority_epoch == 0
            || !valid_sha256_digest(&selector.authority_domain_id)
            || !authority_store_generation_component_is_safe(&selector.generation_id)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_selector_invalid",
            });
        }
        let generation_path = self
            .root
            .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
            .join(&selector.generation_id);
        super::ensure_private_directory(&generation_path).map_err(store_io_error)?;
        let manifest: LeaseAuthorityStoreGenerationManifest = super::load_private_json_file(
            &generation_path.join(LEASE_AUTHORITY_STORE_MANIFEST_FILE),
            "lease_authority_protocol_store_manifest_decode_failed",
        )
        .map_err(store_io_error)?;
        self.validate_generation(&generation_path, &manifest)?;
        if manifest.generation_id != selector.generation_id
            || manifest.authority_domain_id != selector.authority_domain_id
            || manifest.authority_epoch != selector.authority_epoch
            || manifest.authority_revision != selector.authority_revision
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_selection_mismatch",
            });
        }
        let protected_state_path = generation_path.join(LEASE_AUTHORITY_STORE_PROTECTED_STATE_FILE);
        let protected_state = fs::read(&protected_state_path).map_err(store_io_error)?;
        let mut kernel =
            LeaseAuthorityProtocolKernel::from_protected_state(&protected_state, context)?;
        if kernel.state.domain.authority_domain_id != selector.authority_domain_id
            || kernel.state.domain.authority_epoch != selector.authority_epoch
            || kernel.state.authority.revision != selector.authority_revision
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_authority_mismatch",
            });
        }
        kernel.selected_generation_id = Some(selector.generation_id);
        Ok(kernel)
    }

    fn load_for_mutation(
        &self,
        context: LeaseAuthorityProtectedLoadContext<'_>,
    ) -> Result<LeaseAuthorityProtocolKernel, LeaseAuthorityProtocolError> {
        let mut kernel = self.load(context)?;
        kernel.publication_authorized = true;
        Ok(kernel)
    }

    fn selected_generation_id(&self) -> Result<Option<String>, LeaseAuthorityProtocolError> {
        let selector_path = self.root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE);
        match fs::metadata(&selector_path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(store_io_error(error)),
        }
        let selector: LeaseAuthorityStoreSelector = super::load_private_json_file(
            &selector_path,
            "lease_authority_protocol_store_selector_decode_failed",
        )
        .map_err(store_io_error)?;
        if selector.schema_version != LEASE_AUTHORITY_STORE_SELECTOR_SCHEMA_VERSION
            || selector.authority_epoch == 0
            || !valid_sha256_digest(&selector.authority_domain_id)
            || !authority_store_generation_component_is_safe(&selector.generation_id)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_selector_invalid",
            });
        }
        Ok(Some(selector.generation_id))
    }

    fn load_history(&self) -> Result<Vec<LeaseAuthorityEvent>, LeaseAuthorityProtocolError> {
        let loaded = (|| {
            let selector: LeaseAuthorityStoreSelector = super::load_private_json_file(
                &self.root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE),
                "lease_authority_protocol_store_selector_decode_failed",
            )
            .map_err(store_io_error)?;
            if selector.schema_version != LEASE_AUTHORITY_STORE_SELECTOR_SCHEMA_VERSION
                || selector.authority_epoch == 0
                || !valid_sha256_digest(&selector.authority_domain_id)
                || !authority_store_generation_component_is_safe(&selector.generation_id)
            {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_store_selector_invalid",
                });
            }
            let selected_path = self
                .root
                .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
                .join(&selector.generation_id);
            let selected_manifest: LeaseAuthorityStoreGenerationManifest =
                super::load_private_json_file(
                    &selected_path.join(LEASE_AUTHORITY_STORE_MANIFEST_FILE),
                    "lease_authority_protocol_store_manifest_decode_failed",
                )
                .map_err(store_io_error)?;
            if selected_manifest.generation_id != selector.generation_id
                || selected_manifest.authority_domain_id != selector.authority_domain_id
                || selected_manifest.authority_epoch != selector.authority_epoch
                || selected_manifest.authority_revision != selector.authority_revision
            {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_store_selection_mismatch",
                });
            }
            self.validate_generation(&selected_path, &selected_manifest)?;
            let mut generation_id = selector.generation_id.clone();
            let mut visited = BTreeSet::new();
            let mut segments = Vec::new();
            loop {
                if visited.len() >= LEASE_AUTHORITY_STORE_MAX_HISTORY_GENERATIONS
                    || !visited.insert(generation_id.clone())
                {
                    return Err(LeaseAuthorityProtocolError {
                        code: "lease_authority_protocol_history_chain_invalid",
                    });
                }
                let (events, previous_generation_id) =
                    self.load_history_generation(&generation_id)?;
                segments.push(events);
                match previous_generation_id {
                    Some(previous) => generation_id = previous,
                    None => break,
                }
            }
            segments.reverse();
            Ok(segments.into_iter().flatten().collect())
        })();
        loaded.map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_history_unavailable",
        })
    }

    fn load_history_generation(
        &self,
        generation_id: &str,
    ) -> Result<(Vec<LeaseAuthorityEvent>, Option<String>), LeaseAuthorityProtocolError> {
        let loaded = (|| {
            if !authority_store_generation_component_is_safe(generation_id) {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_store_generation_invalid",
                });
            }
            let generation_path = self
                .root
                .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
                .join(generation_id);
            super::ensure_private_directory(&generation_path).map_err(store_io_error)?;
            let manifest: LeaseAuthorityStoreGenerationManifest = super::load_private_json_file(
                &generation_path.join(LEASE_AUTHORITY_STORE_MANIFEST_FILE),
                "lease_authority_protocol_store_manifest_decode_failed",
            )
            .map_err(store_io_error)?;
            if manifest.generation_id != generation_id {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_store_selection_mismatch",
                });
            }
            self.validate_generation(&generation_path, &manifest)?;
            let history_manifest: LeaseAuthorityStoreHistoryManifest =
                super::load_private_json_file(
                    &generation_path.join(LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_FILE),
                    "lease_authority_protocol_store_history_manifest_decode_failed",
                )
                .map_err(store_io_error)?;
            self.validate_history_generation(&generation_path, &history_manifest)?;
            let history: LeaseAuthorityHistoryState = super::load_private_json_file(
                &generation_path.join(LEASE_AUTHORITY_STORE_HISTORY_FILE),
                "lease_authority_protocol_history_decode_failed",
            )
            .map_err(store_io_error)?;
            if history.schema_version != LEASE_AUTHORITY_HISTORY_SCHEMA_VERSION {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_history_schema_unsupported",
                });
            }
            Ok((history.events, history_manifest.previous_generation_id))
        })();
        loaded.map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_history_unavailable",
        })
    }

    fn validate_generation(
        &self,
        generation_path: &Path,
        manifest: &LeaseAuthorityStoreGenerationManifest,
    ) -> Result<(), LeaseAuthorityProtocolError> {
        if manifest.schema_version != LEASE_AUTHORITY_STORE_GENERATION_SCHEMA_VERSION
            || !valid_sha256_digest(&manifest.authority_domain_id)
            || manifest.authority_epoch == 0
            || !valid_bare_sha256(&manifest.protected_state_sha256)
            || authority_store_generation_id(
                manifest.authority_epoch,
                manifest.authority_revision,
                &manifest.protected_state_sha256,
            )? != manifest.generation_id
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_generation_invalid",
            });
        }
        super::verify_file_sha256(
            &generation_path.join(LEASE_AUTHORITY_STORE_PROTECTED_STATE_FILE),
            &manifest.protected_state_sha256,
            "lease_authority_protocol_store_protected_digest_mismatch",
        )
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_protected_state_unavailable",
        })
    }

    fn validate_history_generation(
        &self,
        generation_path: &Path,
        manifest: &LeaseAuthorityStoreHistoryManifest,
    ) -> Result<(), LeaseAuthorityProtocolError> {
        if (manifest.schema_version != LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION
            && manifest.schema_version != LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION_V1)
            || (manifest.schema_version == LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION_V1
                && manifest.previous_generation_id.is_some())
            || generation_path.file_name().and_then(|name| name.to_str())
                != Some(manifest.generation_id.as_str())
            || manifest
                .previous_generation_id
                .as_deref()
                .is_some_and(|previous| {
                    !authority_store_generation_component_is_safe(previous)
                        || previous == manifest.generation_id
                })
            || !valid_bare_sha256(&manifest.history_sha256)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_store_history_generation_invalid",
            });
        }
        super::verify_file_sha256(
            &generation_path.join(LEASE_AUTHORITY_STORE_HISTORY_FILE),
            &manifest.history_sha256,
            "lease_authority_protocol_store_history_digest_mismatch",
        )
        .map_err(store_io_error)
    }
}

impl LeaseAuthorityProtocolKernel {
    fn bootstrap(
        authority_domain_id: &str,
        authority_epoch: u64,
        boot_epoch: &str,
        authority: LeaseAuthorityState,
        principals: ServicePrincipalRegistry,
    ) -> Result<Self, LeaseAuthorityProtocolError> {
        if !valid_sha256_digest(authority_domain_id)
            || authority_epoch == 0
            || boot_epoch.trim().is_empty()
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_bootstrap_invalid",
            });
        }
        let kernel = Self {
            state: LeaseAuthorityProtectedState {
                schema_version: LEASE_AUTHORITY_PROTECTED_STATE_SCHEMA_VERSION.to_string(),
                domain: LeaseAuthorityDomainState {
                    schema_version: LEASE_AUTHORITY_DOMAIN_SCHEMA_VERSION.to_string(),
                    authority_domain_id: authority_domain_id.to_string(),
                    authority_epoch,
                    boot_epoch: boot_epoch.to_string(),
                    authority_time_floor: "1970-01-01T00:00:00Z".to_string(),
                },
                authority,
                principals,
                resources: LeaseAuthorityResourceRegistry {
                    schema_version: LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION.to_string(),
                    revision: 0,
                    registrations: BTreeMap::new(),
                },
                owners: LeaseAuthorityOwnerRegistry {
                    schema_version: LEASE_AUTHORITY_OWNER_REGISTRY_SCHEMA_VERSION.to_string(),
                    revision: 0,
                    bindings: BTreeMap::new(),
                },
                profile_enrollment_receipts: BTreeMap::new(),
                effect_receipts: BTreeMap::new(),
            },
            selected_generation_id: None,
            publication_authorized: true,
        };
        validate_protected_state(&kernel.state)?;
        Ok(kernel)
    }

    fn bootstrap_profile_resource(
        &mut self,
        profile_id: &str,
        physical_identity_digest: &str,
    ) -> Result<(), LeaseAuthorityProtocolError> {
        if profile_id.trim().is_empty()
            || !valid_sha256_digest(physical_identity_digest)
            || self
                .state
                .resources
                .registrations
                .values()
                .any(|registration| {
                    registration.physical_identity_digest == physical_identity_digest
                        && registration.resource.id != profile_id
                })
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_registration_invalid",
            });
        }
        let resource = LeaseResourceKey::profile(profile_id);
        let storage_key = resource.storage_key();
        if let Some(existing) = self.state.resources.registrations.get(&storage_key) {
            return (existing.resource == resource
                && existing.physical_identity_digest == physical_identity_digest)
                .then_some(())
                .ok_or(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_resource_registration_conflict",
                });
        }
        let revision = self
            .state
            .resources
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_revision_exhausted",
            })?;
        self.state.resources.registrations.insert(
            storage_key,
            LeaseAuthorityResourceRegistration {
                resource,
                physical_identity_digest: physical_identity_digest.to_string(),
                revision,
                operator_uid: 1,
            },
        );
        self.state.resources.revision = revision;
        Ok(())
    }

    fn enroll_profile(
        &mut self,
        request: EnrollProfileLeaseAuthorityPayload,
        operator_uid: u32,
        physical_identity_digest: &str,
        authority_observed_at: &str,
    ) -> Result<LeaseAuthorityProfileEnrollmentOutcome, LeaseAuthorityProtocolError> {
        if operator_uid == 0
            || crate::runtime_profile::validate_runtime_profile_name(&request.profile_id).is_err()
            || request.idempotency_key.trim().is_empty()
            || !valid_sha256_digest(physical_identity_digest)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_profile_enrollment_invalid",
            });
        }
        let raw_capability =
            std::str::from_utf8(request.raw_capability.expose()).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_capability_invalid",
                }
            })?;
        let capability_digest = profile_capability_digest(raw_capability);
        let principal_id = format!(
            "principal:local-uid:{operator_uid}:profile:{}",
            request.profile_id
        );
        let request_digest = profile_enrollment_request_digest(
            &request.profile_id,
            physical_identity_digest,
            operator_uid,
            &capability_digest,
            request.expected_resource_revision,
            &request.idempotency_key,
        );
        if let Some(receipt) = self
            .state
            .profile_enrollment_receipts
            .get(&request.idempotency_key)
        {
            if receipt.request_digest != request_digest {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_profile_enrollment_idempotency_conflict",
                });
            }
            return Ok(LeaseAuthorityProfileEnrollmentOutcome {
                receipt: receipt.clone(),
                replayed: true,
            });
        }

        let resource = LeaseResourceKey::profile(&request.profile_id);
        let storage_key = resource.storage_key();
        let current_registration = self
            .state
            .resources
            .registrations
            .get(&storage_key)
            .cloned();
        let current_revision = current_registration
            .as_ref()
            .map_or(0, |registration| registration.revision);
        if current_revision != request.expected_resource_revision {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_profile_enrollment_stale_revision",
            });
        }
        if current_registration.as_ref().is_some_and(|registration| {
            registration.resource != resource
                || registration.physical_identity_digest != physical_identity_digest
                || registration.operator_uid != operator_uid
        }) || self
            .state
            .resources
            .registrations
            .values()
            .any(|registration| {
                registration.physical_identity_digest == physical_identity_digest
                    && registration.resource != resource
            })
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_registration_conflict",
            });
        }

        let now = self.observe_authority_time(authority_observed_at)?;
        let mut principals = self.state.principals.clone();
        if principals.revision == u64::MAX {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_principal_revision_exhausted",
            });
        }
        let registered = register_profile_capability(
            &mut principals,
            ServicePrincipalRegistrationRequest {
                principal_id: principal_id.clone(),
                display_name: None,
                profile_id: request.profile_id.clone(),
                registered_at: Some(now.clone()),
                registered_by: Some(format!("lease-authority-peer-uid:{operator_uid}")),
            },
            raw_capability,
        )
        .map_err(|error| LeaseAuthorityProtocolError {
            code: error.code.as_str(),
        })?;

        let mut resources = self.state.resources.registrations.clone();
        let resource_revision = if let Some(existing) = current_registration.as_ref() {
            existing.revision
        } else {
            self.state
                .resources
                .revision
                .checked_add(1)
                .filter(|revision| *revision > 0)
                .ok_or(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_resource_revision_exhausted",
                })?
        };
        resources
            .entry(storage_key)
            .or_insert_with(|| LeaseAuthorityResourceRegistration {
                resource,
                physical_identity_digest: physical_identity_digest.to_string(),
                revision: resource_revision,
                operator_uid,
            });
        let enrollment_id = format!(
            "profile-enrollment:{}",
            &request_digest.trim_start_matches("sha256:")[..24]
        );
        let receipt = LeaseAuthorityProfileEnrollmentReceipt {
            schema_version: LEASE_AUTHORITY_PROFILE_ENROLLMENT_RECEIPT_SCHEMA_VERSION.to_string(),
            enrollment_id,
            request_digest,
            idempotency_key: request.idempotency_key.clone(),
            principal_id,
            capability_id: registered.capability.capability_id,
            capability_revision: registered.capability.revision,
            profile_id: request.profile_id,
            physical_identity_digest: physical_identity_digest.to_string(),
            resource_revision,
            operator_uid,
            occurred_at: now,
        };
        self.state.principals = principals;
        self.state.resources.registrations = resources;
        self.state.resources.revision = self.state.resources.revision.max(resource_revision);
        self.state
            .profile_enrollment_receipts
            .insert(request.idempotency_key, receipt.clone());
        validate_protected_state(&self.state)?;
        Ok(LeaseAuthorityProfileEnrollmentOutcome {
            receipt,
            replayed: false,
        })
    }

    fn validate_administrator_capability(
        &self,
        administrator_id: &str,
        administrator_revision: u64,
        raw_capability: &[u8],
    ) -> Result<(), LeaseAuthorityProtocolError> {
        self.state
            .authority
            .authenticate_administrator(administrator_id, administrator_revision, raw_capability)
            .map(|_| ())
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_administrator_identity_invalid",
            })
    }

    fn observe_authority_time(
        &mut self,
        observed_at: &str,
    ) -> Result<String, LeaseAuthorityProtocolError> {
        let observed = chrono::DateTime::parse_from_rfc3339(observed_at).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_clock_invalid",
            }
        })?;
        let floor = chrono::DateTime::parse_from_rfc3339(&self.state.domain.authority_time_floor)
            .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_clock_state_invalid",
        })?;
        if observed > floor {
            self.state.domain.authority_time_floor = observed_at.to_string();
        }
        Ok(self.state.domain.authority_time_floor.clone())
    }

    fn plan_administrative_revocation(
        &mut self,
        request: RevokeLeasePlanPayload,
        context: &LeaseAuthorityAdministrativeDispatchContext<'_>,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<RevokeLeasePlanProjection, LeaseAuthorityProtocolError> {
        let now = self.observe_authority_time(context.authority_observed_at)?;
        let issued_at = chrono::DateTime::parse_from_rfc3339(&now).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_clock_state_invalid",
            }
        })?;
        let authorization_expires_at = (issued_at
            + chrono::Duration::seconds(super::MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS))
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let intent = LeaseAdministrativeIntent {
            administrator_id: context.administrator_id.to_string(),
            administrator_revision: context.administrator_revision,
            idempotency_key: request.idempotency_key,
            reason_code: request.reason_code,
            issued_at: now.clone(),
            authorization_expires_at,
        };
        if let Some(replayed) = self
            .state
            .authority
            .replay_administrative_revocation_plan(
                &request.resource,
                &request.claim_id,
                request.claim_revision,
                request.fencing_token,
                &intent,
                context.raw_administrator_capability,
            )
            .map_err(lease_authority_protocol_error)?
        {
            return Ok(RevokeLeasePlanProjection::from_outcome(replayed));
        }
        let claim = self
            .state
            .authority
            .current_claim(&request.resource, &now)
            .filter(|claim| {
                claim.claim_id == request.claim_id
                    && claim.revision == request.claim_revision
                    && claim.fencing_token == request.fencing_token
            })
            .cloned()
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_stale_claim",
            })?;
        let outcome = self
            .state
            .authority
            .plan_administrative_revocation(
                &claim,
                &intent,
                context.raw_administrator_capability,
                signing_key,
            )
            .map_err(lease_authority_protocol_error)?;
        Ok(RevokeLeasePlanProjection::from_outcome(outcome))
    }

    fn apply_administrative_revocation(
        &mut self,
        request: RevokeLeaseApplyPayload,
        context: &LeaseAuthorityAdministrativeDispatchContext<'_>,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<super::LeaseClaimRevocationOutcome, LeaseAuthorityProtocolError> {
        let now = self.observe_authority_time(context.authority_observed_at)?;
        self.validate_administrator_capability(
            context.administrator_id,
            context.administrator_revision,
            context.raw_administrator_capability,
        )?;
        let authorization = self
            .state
            .authority
            .administrative_authorization_by_plan_id(&request.plan_id)
            .map_err(lease_authority_protocol_error)?
            .clone();
        self.state
            .authority
            .revoke_with_receipt(
                RevokeLeaseClaimRequest { authorization, now },
                &LeaseAuthorityVerificationKeyring::from_active(signing_key),
            )
            .map_err(lease_authority_protocol_error)
    }

    fn authenticate_recovery_controller(
        &self,
        raw_capability: &[u8],
        profile_id: &str,
    ) -> Result<
        crate::native::service_principal::ServiceProfileCapability,
        LeaseAuthorityProtocolError,
    > {
        let raw_capability =
            std::str::from_utf8(raw_capability).map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_recovery_controller_invalid",
            })?;
        let authenticated = authenticate_profile_capability(
            &self.state.principals,
            raw_capability,
            Some(profile_id),
        )
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_recovery_controller_invalid",
        })?;
        self.state
            .principals
            .profile_capabilities
            .get(&authenticated.capability_id)
            .filter(|controller| {
                controller.principal_id == authenticated.principal_id
                    && controller.profile_id == authenticated.profile_id
                    && controller.revision == authenticated.capability_revision
            })
            .cloned()
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_recovery_controller_invalid",
            })
    }

    fn authenticate_recovery_replay(
        &self,
        raw_capability: &[u8],
        authorization: &LeaseRecoveryAuthorization,
    ) -> Result<(), LeaseAuthorityProtocolError> {
        let raw_capability =
            std::str::from_utf8(raw_capability).map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_recovery_controller_invalid",
            })?;
        self.state
            .principals
            .profile_capabilities
            .get(&authorization.recovery_controller_id)
            .filter(|controller| {
                controller.capability_digest == profile_capability_digest(raw_capability)
                    && controller.principal_id == authorization.principal_id
                    && controller.profile_id == authorization.resource.id
                    && controller.revision == authorization.recovery_controller_revision
            })
            .map(|_| ())
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_recovery_controller_invalid",
            })
    }

    fn plan_recovery(
        &mut self,
        request: RecoverLeasePlanPayload,
        authority_observed_at: &str,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<RecoverLeasePlanProjection, LeaseAuthorityProtocolError> {
        let now = self.observe_authority_time(authority_observed_at)?;
        let issued_at = chrono::DateTime::parse_from_rfc3339(&now).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_clock_state_invalid",
            }
        })?;
        let controller = self.authenticate_recovery_controller(
            request.raw_controller_capability.expose(),
            &request.resource.id,
        )?;
        let claim = self
            .state
            .authority
            .current_claim(&request.resource, &now)
            .filter(|claim| {
                claim.claim_id == request.claim_id
                    && claim.revision == request.claim_revision
                    && claim.fencing_token == request.fencing_token
            })
            .cloned()
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_stale_claim",
            })?;
        let intent = LeaseRecoveryIntent {
            idempotency_key: request.idempotency_key,
            issued_at: now,
            authorization_expires_at: (issued_at
                + chrono::Duration::seconds(super::MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS))
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
            claim_expires_at: (issued_at
                + chrono::Duration::seconds(super::MAX_STRICT_RECOVERY_TENURE_SECONDS))
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
            transition_deadline: (issued_at + chrono::Duration::seconds(60))
                .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
            owner_generation: request.owner_generation,
        };
        let outcome = self
            .state
            .authority
            .plan_recovery(&claim, &controller, &intent, signing_key)
            .map_err(lease_authority_protocol_error)?;
        Ok(RecoverLeasePlanProjection::from_outcome(outcome))
    }

    fn apply_recovery(
        &mut self,
        request: RecoverLeaseApplyPayload,
        authority_observed_at: &str,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<super::LeaseClaimRecoveryOutcome, LeaseAuthorityProtocolError> {
        let now = self.observe_authority_time(authority_observed_at)?;
        let authorization = self
            .state
            .authority
            .recovery_authorization_by_plan_id(&request.plan_id)
            .map_err(lease_authority_protocol_error)?
            .clone();
        if self
            .state
            .authority
            .recovery_receipts
            .contains_key(&authorization.idempotency_key)
        {
            self.authenticate_recovery_replay(
                request.raw_controller_capability.expose(),
                &authorization,
            )?;
            return self
                .state
                .authority
                .recover_with_receipt(
                    RecoverLeaseClaimRequest { authorization, now },
                    &LeaseAuthorityVerificationKeyring::from_active(signing_key),
                )
                .map_err(lease_authority_protocol_error);
        }
        let controller = self.authenticate_recovery_controller(
            request.raw_controller_capability.expose(),
            &authorization.resource.id,
        )?;
        if controller.capability_id != authorization.recovery_controller_id
            || controller.revision != authorization.recovery_controller_revision
            || controller.principal_id != authorization.principal_id
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_recovery_controller_invalid",
            });
        }
        self.state
            .authority
            .recover_with_receipt(
                RecoverLeaseClaimRequest { authorization, now },
                &LeaseAuthorityVerificationKeyring::from_active(signing_key),
            )
            .map_err(lease_authority_protocol_error)
    }

    fn issue_service_identity_challenge(
        &self,
        request: &LeaseAuthorityServiceChallengeRequest,
        custody: &custody::LeaseAuthorityCustodyIdentity,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseAuthorityServiceIdentityProof, LeaseAuthorityProtocolError> {
        if !valid_sha256_digest(&request.nonce)
            || request.expected_authority_domain_id != self.state.domain.authority_domain_id
            || request.minimum_authority_epoch == 0
            || self.state.domain.authority_epoch < request.minimum_authority_epoch
            || !valid_sha256_digest(&custody.endpoint_identity_digest)
            || !valid_sha256_digest(&custody.executable_sha256)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_service_identity_request_invalid",
            });
        }
        let mut identity = LeaseAuthorityServiceIdentityProof {
            schema_version: LEASE_AUTHORITY_SERVICE_IDENTITY_PROOF_SCHEMA_VERSION.to_string(),
            authority_domain_id: self.state.domain.authority_domain_id.clone(),
            authority_epoch: self.state.domain.authority_epoch,
            boot_epoch: self.state.domain.boot_epoch.clone(),
            endpoint_identity_digest: custody.endpoint_identity_digest.clone(),
            executable_sha256: custody.executable_sha256.clone(),
            signing_key_id: signing_key.key_id.clone(),
            signing_key_epoch: signing_key.key_epoch,
            nonce: request.nonce.clone(),
            proof: String::new(),
        };
        let key = Ed25519KeyPair::from_seed_unchecked(&signing_key.private_key).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_service_identity_signing_failed",
            }
        })?;
        identity.proof = hex::encode(
            key.sign(service_identity_proof_payload(&identity).as_bytes())
                .as_ref(),
        );
        Ok(identity)
    }

    fn encode_protected_state(&self) -> Result<Vec<u8>, LeaseAuthorityProtocolError> {
        serde_json::to_vec_pretty(&self.state).map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_state_encode_failed",
        })
    }

    fn encode_history_state(&self) -> Result<Vec<u8>, LeaseAuthorityProtocolError> {
        serde_json::to_vec_pretty(&LeaseAuthorityHistoryRef {
            schema_version: LEASE_AUTHORITY_HISTORY_SCHEMA_VERSION,
            events: &self.state.authority.events,
        })
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_history_encode_failed",
        })
    }

    fn from_protected_state(
        encoded: &[u8],
        context: LeaseAuthorityProtectedLoadContext<'_>,
    ) -> Result<Self, LeaseAuthorityProtocolError> {
        let state: LeaseAuthorityProtectedState =
            serde_json::from_slice(encoded).map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_state_invalid",
            })?;
        if state.schema_version != LEASE_AUTHORITY_PROTECTED_STATE_SCHEMA_VERSION {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_state_schema_unsupported",
            });
        }
        validate_protected_state(&state)?;
        if state.domain.authority_domain_id != context.expected_authority_domain_id {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_domain_mismatch",
            });
        }
        if state.domain.authority_epoch < context.minimum_authority_epoch {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_epoch_rollback",
            });
        }
        Ok(Self {
            state,
            selected_generation_id: None,
            publication_authorized: false,
        })
    }

    fn execute(
        &mut self,
        request: LeaseAuthorityProtocolRequest,
        authority_observed_at: &str,
    ) -> Result<LeaseAuthorityProtocolResponse, LeaseAuthorityProtocolError> {
        match request {
            LeaseAuthorityProtocolRequest::Acquire(request) => {
                self.acquire(*request, authority_observed_at)
            }
            LeaseAuthorityProtocolRequest::ServiceChallenge(_)
            | LeaseAuthorityProtocolRequest::EnrollProfile(_)
            | LeaseAuthorityProtocolRequest::AuthorizeEffect(_)
            | LeaseAuthorityProtocolRequest::CompleteEffect(_)
            | LeaseAuthorityProtocolRequest::Release(_)
            | LeaseAuthorityProtocolRequest::RecoverPlan(_)
            | LeaseAuthorityProtocolRequest::Recover(_)
            | LeaseAuthorityProtocolRequest::RevokePlan(_)
            | LeaseAuthorityProtocolRequest::Revoke(_)
            | LeaseAuthorityProtocolRequest::Inspect(_) => Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_operation_not_implemented",
            }),
        }
    }

    fn acquire(
        &mut self,
        request: AcquireLeaseAuthorityPayload,
        authority_observed_at: &str,
    ) -> Result<LeaseAuthorityProtocolResponse, LeaseAuthorityProtocolError> {
        if request.resource.kind != LeaseResourceKind::Profile {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_unsupported",
            });
        }
        if !self
            .state
            .resources
            .registrations
            .contains_key(&request.resource.storage_key())
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_unregistered",
            });
        }
        let raw_capability =
            std::str::from_utf8(request.raw_capability.expose()).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_capability_invalid",
                }
            })?;
        let authenticated = authenticate_profile_capability(
            &self.state.principals,
            raw_capability,
            Some(&request.resource.id),
        )
        .map_err(|error| LeaseAuthorityProtocolError {
            code: error.code.as_str(),
        })?;
        let now = self.observe_authority_time(authority_observed_at)?;
        let observed = chrono::DateTime::parse_from_rfc3339(&now).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_clock_state_invalid",
            }
        })?;
        let expires_at = (observed
            + chrono::Duration::seconds(super::MAX_LEASE_CLAIM_TENURE_SECONDS))
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let transition_deadline = (request.mode == LeaseClaimMode::Strict).then(|| {
            (observed + chrono::Duration::seconds(60))
                .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        });
        let expected_claim_revision = match (request.mode, request.expected_claim_revision) {
            (LeaseClaimMode::Ephemeral, None) => self
                .state
                .authority
                .current_claim_revision(&request.resource, &now),
            (LeaseClaimMode::Strict, None) => {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_strict_expected_revision_required",
                });
            }
            (_, Some(revision)) => revision,
        };
        let owner_generation = self
            .state
            .owners
            .bindings
            .get(&request.resource.storage_key())
            .map(|owner| owner.owner_generation);
        let outcome = self
            .state
            .authority
            .acquire_with_receipt(AcquireLeaseClaimRequest {
                resource: request.resource,
                parent_claim_id: request.parent_claim_id,
                principal_id: authenticated.principal_id,
                capability_id: authenticated.capability_id,
                capability_revision: authenticated.capability_revision,
                mode: request.mode,
                expected_claim_revision,
                idempotency_key: request.idempotency_key,
                now,
                expires_at,
                transition_deadline,
                recovery_controller_id: request.recovery_controller_id,
                boot_epoch: Some(self.state.domain.boot_epoch.clone()),
                owner_generation,
            })
            .map_err(|error| LeaseAuthorityProtocolError {
                code: error.as_str(),
            })?;
        Ok(LeaseAuthorityProtocolResponse::Acquired(outcome))
    }

    fn authorize_effect(
        &mut self,
        request: AuthorizeLeaseEffectPayload,
        executor_uid: u32,
        executor_identity_digest: &str,
        authority_observed_at: &str,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseAuthorityEffectOutcome, LeaseAuthorityProtocolError> {
        if executor_uid == 0
            || request.resource.kind != LeaseResourceKind::Profile
            || request.claim_id.trim().is_empty()
            || request.claim_revision == 0
            || request.fencing_token == 0
            || request.action_class != "browser_launch"
            || request
                .audience
                .strip_prefix("daemon-session:")
                .is_none_or(|audience| {
                    audience.is_empty()
                        || audience.len() > 160
                        || !audience.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric()
                                || matches!(byte, b'-' | b'_' | b':' | b'.')
                        })
                })
            || request.idempotency_key.trim().is_empty()
            || !valid_sha256_digest(executor_identity_digest)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_invalid",
            });
        }
        let registered_operator_uid = self
            .state
            .resources
            .registrations
            .get(&request.resource.storage_key())
            .filter(|registration| registration.operator_uid == executor_uid)
            .map(|registration| registration.operator_uid)
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_executor_mismatch",
            })?;
        let raw_capability =
            std::str::from_utf8(request.raw_capability.expose()).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_capability_invalid",
                }
            })?;
        let authenticated = authenticate_profile_capability(
            &self.state.principals,
            raw_capability,
            Some(&request.resource.id),
        )
        .map_err(|error| LeaseAuthorityProtocolError {
            code: error.code.as_str(),
        })?;
        let storage_key = effect_receipt_storage_key(
            &self.state.domain.authority_domain_id,
            &authenticated.principal_id,
            &request.resource,
            &request.action_class,
            &request.idempotency_key,
        );
        let request_digest = effect_request_digest(
            &self.state.domain.authority_domain_id,
            self.state.domain.authority_epoch,
            &authenticated.principal_id,
            &authenticated.capability_id,
            authenticated.capability_revision,
            &request,
            executor_uid,
            executor_identity_digest,
        );
        let now = self.observe_authority_time(authority_observed_at)?;
        if let Some(record) = self.state.effect_receipts.get(&storage_key) {
            if record.receipt.request_digest != request_digest
                || record.receipt.executor_identity_digest != executor_identity_digest
            {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_idempotency_conflict",
                });
            }
            return Ok(LeaseAuthorityEffectOutcome {
                receipt: record.receipt.clone(),
                // Authorization is a single-use delivery. Once the durable
                // receipt exists, replay proves only that the operation was
                // admitted; it cannot distinguish a lost pre-effect response
                // from a crash after the external effect. Reissuing the bearer
                // would permit a blind duplicate launch.
                authorization: None,
                replayed: true,
            });
        }
        if self.state.effect_receipts.len() >= MAX_LEASE_AUTHORITY_EFFECT_RECEIPTS {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_capacity_exhausted",
            });
        }
        let claim = self
            .state
            .authority
            .current_claim(&request.resource, &now)
            .filter(|claim| {
                claim.claim_id == request.claim_id
                    && claim.revision == request.claim_revision
                    && claim.fencing_token == request.fencing_token
                    && claim.principal_id == authenticated.principal_id
                    && claim.capability_id == authenticated.capability_id
                    && claim.capability_revision == authenticated.capability_revision
            })
            .cloned()
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_stale_claim",
            })?;
        let observed = chrono::DateTime::parse_from_rfc3339(&now).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_clock_state_invalid",
            }
        })?;
        let claim_expires_at =
            chrono::DateTime::parse_from_rfc3339(&claim.expires_at).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_state_invalid",
                }
            })?;
        let authorization_expires_at = std::cmp::min(
            observed + chrono::Duration::seconds(super::MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS),
            claim_expires_at,
        )
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let capability = self
            .state
            .principals
            .profile_capabilities
            .get(claim.capability_id())
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_capability_unavailable",
            })?;
        let authorization = claim
            .effect_authorization(
                capability,
                &LeaseEffectIntent {
                    action_class: request.action_class.clone(),
                    audience: request.audience.clone(),
                    operation_idempotency_key: request.idempotency_key.clone(),
                    executor_identity_digest: Some(executor_identity_digest.to_string()),
                    issued_at: now.clone(),
                    authorization_expires_at: authorization_expires_at.clone(),
                },
                signing_key,
            )
            .map_err(lease_authority_protocol_error)?;
        let next_authority_revision = self
            .state
            .authority
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_counter_exhausted",
            })?;
        let receipt = LeaseAuthorityEffectReceipt {
            schema_version: LEASE_AUTHORITY_EFFECT_RECEIPT_SCHEMA_VERSION.to_string(),
            receipt_id: format!(
                "effect-receipt:{}",
                &request_digest.trim_start_matches("sha256:")[..24]
            ),
            request_digest,
            idempotency_key: request.idempotency_key,
            resource: request.resource,
            claim_id: claim.claim_id,
            principal_id: authenticated.principal_id,
            capability_id: authenticated.capability_id,
            capability_revision: authenticated.capability_revision,
            claim_revision: claim.revision,
            fencing_token: claim.fencing_token,
            action_class: request.action_class,
            audience: request.audience,
            executor_identity_digest: executor_identity_digest.to_string(),
            executor_uid: registered_operator_uid,
            authority_revision: next_authority_revision,
            occurred_at: now,
            authorization_expires_at,
            state: LeaseAuthorityEffectState::Consumed,
            completion_idempotency_key: None,
            completion_evidence_digest: None,
            completed_at: None,
            terminal_authority_revision: None,
        };
        self.state.authority.revision = next_authority_revision;
        self.state.effect_receipts.insert(
            storage_key,
            LeaseAuthorityEffectRecord {
                receipt: receipt.clone(),
                authorization: Some(authorization.clone()),
            },
        );
        validate_protected_state(&self.state)?;
        Ok(LeaseAuthorityEffectOutcome {
            receipt,
            authorization: Some(authorization),
            replayed: false,
        })
    }

    fn complete_effect(
        &mut self,
        request: CompleteLeaseEffectPayload,
        executor_uid: u32,
        executor_identity_digest: &str,
        authority_observed_at: &str,
    ) -> Result<LeaseAuthorityEffectCompletionOutcome, LeaseAuthorityProtocolError> {
        if executor_uid == 0
            || request.receipt_id.trim().is_empty()
            || request.completion_idempotency_key.trim().is_empty()
            || !valid_sha256_digest(&request.completion_evidence_digest)
            || !valid_sha256_digest(executor_identity_digest)
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_completion_invalid",
            });
        }
        let now = self.observe_authority_time(authority_observed_at)?;
        let storage_key = self
            .state
            .effect_receipts
            .iter()
            .find_map(|(storage_key, record)| {
                (record.receipt.receipt_id == request.receipt_id).then(|| storage_key.clone())
            })
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_receipt_unavailable",
            })?;
        let record = self
            .state
            .effect_receipts
            .get(&storage_key)
            .cloned()
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_receipt_unavailable",
            })?;
        let terminal_state = match request.result {
            CompleteLeaseEffectResult::Completed => LeaseAuthorityEffectState::Completed,
            CompleteLeaseEffectResult::Uncertain => LeaseAuthorityEffectState::Uncertain,
        };
        if record.receipt.state != LeaseAuthorityEffectState::Consumed {
            if record.receipt.state == terminal_state
                && record.receipt.completion_idempotency_key.as_deref()
                    == Some(request.completion_idempotency_key.as_str())
                && record.receipt.completion_evidence_digest.as_deref()
                    == Some(request.completion_evidence_digest.as_str())
            {
                return Ok(LeaseAuthorityEffectCompletionOutcome {
                    receipt: record.receipt,
                    replayed: true,
                });
            }
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_idempotency_conflict",
            });
        }
        if record.receipt.executor_uid != executor_uid
            || record.receipt.executor_identity_digest != executor_identity_digest
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_effect_executor_mismatch",
            });
        }
        let next_authority_revision = self
            .state
            .authority
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_counter_exhausted",
            })?;
        let mut receipt = record.receipt;
        receipt.state = terminal_state;
        receipt.completion_idempotency_key = Some(request.completion_idempotency_key);
        receipt.completion_evidence_digest = Some(request.completion_evidence_digest);
        receipt.completed_at = Some(now);
        receipt.terminal_authority_revision = Some(next_authority_revision);
        self.state.authority.revision = next_authority_revision;
        self.state.effect_receipts.insert(
            storage_key,
            LeaseAuthorityEffectRecord {
                receipt: receipt.clone(),
                authorization: None,
            },
        );
        validate_protected_state(&self.state)?;
        Ok(LeaseAuthorityEffectCompletionOutcome {
            receipt,
            replayed: false,
        })
    }

    fn release(
        &mut self,
        request: ReleaseLeaseAuthorityPayload,
        authority_observed_at: &str,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseClaimReleaseOutcome, LeaseAuthorityProtocolError> {
        if request.resource.kind != LeaseResourceKind::Profile
            || request.claim_id.trim().is_empty()
            || request.claim_revision == 0
            || request.fencing_token == 0
            || request.idempotency_key.trim().is_empty()
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_release_invalid",
            });
        }
        let now = self.observe_authority_time(authority_observed_at)?;
        if let Some(receipt) = self
            .state
            .authority
            .terminal_receipts
            .get(&request.idempotency_key)
        {
            if receipt.operation != "release"
                || receipt.resource != request.resource
                || receipt.claim_id != request.claim_id
                || receipt.claim_revision != request.claim_revision
                || receipt.released_fencing_token != request.fencing_token
            {
                return Err(LeaseAuthorityProtocolError {
                    code: "lease_authority_idempotency_conflict",
                });
            }
            return Ok(LeaseClaimReleaseOutcome {
                receipt: receipt.clone(),
                replayed: true,
            });
        }

        let raw_capability =
            std::str::from_utf8(request.raw_capability.expose()).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_capability_invalid",
                }
            })?;
        let authenticated = authenticate_profile_capability(
            &self.state.principals,
            raw_capability,
            Some(&request.resource.id),
        )
        .map_err(|error| LeaseAuthorityProtocolError {
            code: error.code.as_str(),
        })?;
        let claim = self
            .state
            .authority
            .current_claim(&request.resource, &now)
            .filter(|claim| {
                claim.claim_id == request.claim_id
                    && claim.revision == request.claim_revision
                    && claim.fencing_token == request.fencing_token
                    && claim.principal_id == authenticated.principal_id
                    && claim.capability_id == authenticated.capability_id
                    && claim.capability_revision == authenticated.capability_revision
            })
            .cloned()
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_stale_claim",
            })?;
        let observed = chrono::DateTime::parse_from_rfc3339(&now).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_clock_state_invalid",
            }
        })?;
        let claim_expires_at =
            chrono::DateTime::parse_from_rfc3339(&claim.expires_at).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_state_invalid",
                }
            })?;
        let authorization_expires_at = std::cmp::min(
            observed + chrono::Duration::seconds(super::MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS),
            claim_expires_at,
        )
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let capability = self
            .state
            .principals
            .profile_capabilities
            .get(claim.capability_id())
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_capability_unavailable",
            })?;
        let authorization = claim
            .effect_authorization(
                capability,
                &LeaseEffectIntent {
                    action_class: "lease_release".to_string(),
                    audience: "lease_authority_kernel".to_string(),
                    operation_idempotency_key: request.idempotency_key.clone(),
                    executor_identity_digest: None,
                    issued_at: now.clone(),
                    authorization_expires_at,
                },
                signing_key,
            )
            .map_err(lease_authority_protocol_error)?;
        self.state
            .authority
            .release_with_receipt(
                ReleaseLeaseClaimRequest {
                    authorization,
                    idempotency_key: request.idempotency_key,
                    now,
                },
                &LeaseAuthorityVerificationKeyring::from_active(signing_key),
            )
            .map_err(lease_authority_protocol_error)
    }
}

mod lease_authority_operational_state_serde {
    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct OperationalStateRef<'a> {
        schema_version: &'a str,
        revision: u64,
        active_claims: &'a BTreeMap<String, ActiveLeaseClaim>,
        next_fencing_tokens: &'a BTreeMap<String, u64>,
        acquisition_receipts: &'a BTreeMap<String, LeaseClaimAcquisitionReceipt>,
        terminal_receipts: &'a BTreeMap<String, LeaseClaimTerminalReceipt>,
        recovery_receipts: &'a BTreeMap<String, LeaseClaimRecoveryReceipt>,
        administrators: &'a BTreeMap<String, LeaseAdministratorAuthority>,
        recovery_authorizations: &'a BTreeMap<String, LeaseRecoveryAuthorization>,
        administrative_authorizations: &'a BTreeMap<String, LeaseAdministrativeAuthorization>,
    }

    #[derive(Default, Deserialize)]
    #[serde(default, rename_all = "camelCase", deny_unknown_fields)]
    struct OperationalState {
        schema_version: String,
        revision: u64,
        active_claims: BTreeMap<String, ActiveLeaseClaim>,
        next_fencing_tokens: BTreeMap<String, u64>,
        acquisition_receipts: BTreeMap<String, LeaseClaimAcquisitionReceipt>,
        terminal_receipts: BTreeMap<String, LeaseClaimTerminalReceipt>,
        recovery_receipts: BTreeMap<String, LeaseClaimRecoveryReceipt>,
        administrators: BTreeMap<String, LeaseAdministratorAuthority>,
        recovery_authorizations: BTreeMap<String, LeaseRecoveryAuthorization>,
        administrative_authorizations: BTreeMap<String, LeaseAdministrativeAuthorization>,
    }

    pub(super) fn serialize<S>(
        state: &LeaseAuthorityState,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        OperationalStateRef {
            schema_version: &state.schema_version,
            revision: state.revision,
            active_claims: &state.active_claims,
            next_fencing_tokens: &state.next_fencing_tokens,
            acquisition_receipts: &state.acquisition_receipts,
            terminal_receipts: &state.terminal_receipts,
            recovery_receipts: &state.recovery_receipts,
            administrators: &state.administrators,
            recovery_authorizations: &state.recovery_authorizations,
            administrative_authorizations: &state.administrative_authorizations,
        }
        .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<LeaseAuthorityState, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = OperationalState::deserialize(deserializer)?;
        Ok(LeaseAuthorityState {
            schema_version: state.schema_version,
            revision: state.revision,
            active_claims: state.active_claims,
            next_fencing_tokens: state.next_fencing_tokens,
            events: Vec::new(),
            acquisition_receipts: state.acquisition_receipts,
            terminal_receipts: state.terminal_receipts,
            recovery_receipts: state.recovery_receipts,
            administrators: state.administrators,
            recovery_authorizations: state.recovery_authorizations,
            administrative_authorizations: state.administrative_authorizations,
        })
    }
}

fn validate_protected_state(
    state: &LeaseAuthorityProtectedState,
) -> Result<(), LeaseAuthorityProtocolError> {
    let invalid = || LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_state_invalid",
    };
    if state.domain.schema_version != LEASE_AUTHORITY_DOMAIN_SCHEMA_VERSION
        || !valid_sha256_digest(&state.domain.authority_domain_id)
        || state.domain.authority_epoch == 0
        || state.domain.boot_epoch.trim().is_empty()
        || chrono::DateTime::parse_from_rfc3339(&state.domain.authority_time_floor).is_err()
        || state.resources.schema_version != LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION
    {
        return Err(invalid());
    }

    if !state.authority.is_empty()
        && (state.authority.schema_version != super::LEASE_AUTHORITY_SCHEMA_VERSION
            || state.authority.revision == 0)
    {
        return Err(invalid());
    }
    for (administrator_id, administrator) in &state.authority.administrators {
        if administrator_id != &administrator.administrator_id
            || administrator_id.trim().is_empty()
            || !valid_sha256_digest(&administrator.capability_digest)
            || administrator.revision == 0
            || administrator.revision > state.authority.revision
        {
            return Err(invalid());
        }
    }
    let mut recovery_plan_ids = BTreeSet::new();
    for (idempotency_key, authorization) in &state.authority.recovery_authorizations {
        if idempotency_key != &authorization.idempotency_key
            || idempotency_key.trim().is_empty()
            || authorization.schema_version != super::LEASE_RECOVERY_AUTHORIZATION_SCHEMA_VERSION
            || authorization.signing_key_id.trim().is_empty()
            || authorization.signing_key_epoch == 0
            || authorization.resource.kind != LeaseResourceKind::Profile
            || authorization.resource.id.trim().is_empty()
            || authorization.claim_id.trim().is_empty()
            || authorization.principal_id.trim().is_empty()
            || authorization.claim_revision == 0
            || authorization.fencing_token == 0
            || authorization.proof.trim().is_empty()
            || !recovery_plan_ids.insert(authorization.plan_id())
            || !super::timestamp_precedes(
                &authorization.issued_at,
                &authorization.authorization_expires_at,
            )
            || !super::timestamp_precedes(
                &authorization.issued_at,
                &authorization.transition_deadline,
            )
            || !super::timestamp_precedes(
                &authorization.transition_deadline,
                &authorization.claim_expires_at,
            )
            || !super::timestamp_span_within(
                &authorization.issued_at,
                &authorization.authorization_expires_at,
                super::MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
            )
            || !super::timestamp_span_within(
                &authorization.issued_at,
                &authorization.claim_expires_at,
                super::MAX_STRICT_RECOVERY_TENURE_SECONDS,
            )
        {
            return Err(invalid());
        }
    }
    let mut administrative_plan_ids = BTreeSet::new();
    for (idempotency_key, authorization) in &state.authority.administrative_authorizations {
        let administrator = state
            .authority
            .administrators
            .get(&authorization.administrator_id);
        if idempotency_key != &authorization.idempotency_key
            || idempotency_key.trim().is_empty()
            || authorization.schema_version
                != super::LEASE_ADMINISTRATIVE_AUTHORIZATION_SCHEMA_VERSION
            || authorization.signing_key_id.trim().is_empty()
            || authorization.signing_key_epoch == 0
            || authorization.resource.id.trim().is_empty()
            || authorization.claim_id.trim().is_empty()
            || authorization.principal_id.trim().is_empty()
            || authorization.claim_revision == 0
            || authorization.fencing_token == 0
            || authorization.reason_code.trim().is_empty()
            || authorization.proof.trim().is_empty()
            || !administrative_plan_ids.insert(authorization.plan_id())
            || !super::timestamp_precedes(
                &authorization.issued_at,
                &authorization.authorization_expires_at,
            )
            || !super::timestamp_span_within(
                &authorization.issued_at,
                &authorization.authorization_expires_at,
                super::MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
            )
            || administrator.is_none_or(|administrator| {
                administrator.administrator_id != authorization.administrator_id
                    || administrator.revision != authorization.administrator_revision
            })
        {
            return Err(invalid());
        }
    }

    let mut physical_identities = BTreeSet::new();
    let mut highest_revision = 0;
    for (storage_key, registration) in &state.resources.registrations {
        if registration.resource.kind != LeaseResourceKind::Profile
            || storage_key != &registration.resource.storage_key()
            || !valid_sha256_digest(&registration.physical_identity_digest)
            || registration.revision == 0
            || registration.operator_uid == 0
            || registration.revision > state.resources.revision
            || !physical_identities.insert(&registration.physical_identity_digest)
        {
            return Err(invalid());
        }
        highest_revision = highest_revision.max(registration.revision);
    }

    if highest_revision != state.resources.revision {
        return Err(invalid());
    }

    let mut enrollment_ids = BTreeSet::new();
    for (idempotency_key, receipt) in &state.profile_enrollment_receipts {
        if idempotency_key != &receipt.idempotency_key
            || idempotency_key.trim().is_empty()
            || receipt.schema_version != LEASE_AUTHORITY_PROFILE_ENROLLMENT_RECEIPT_SCHEMA_VERSION
            || !valid_sha256_digest(&receipt.request_digest)
            || receipt.enrollment_id
                != format!(
                    "profile-enrollment:{}",
                    &receipt.request_digest.trim_start_matches("sha256:")[..24]
                )
            || !enrollment_ids.insert(&receipt.enrollment_id)
            || !valid_sha256_digest(&receipt.physical_identity_digest)
            || crate::runtime_profile::validate_runtime_profile_name(&receipt.profile_id).is_err()
            || receipt.principal_id.trim().is_empty()
            || receipt.capability_id.trim().is_empty()
            || receipt.operator_uid == 0
            || receipt.resource_revision == 0
            || receipt.resource_revision > state.resources.revision
            || receipt.capability_revision == 0
            || chrono::DateTime::parse_from_rfc3339(&receipt.occurred_at).is_err()
        {
            return Err(invalid());
        }
    }

    if state.effect_receipts.len() > MAX_LEASE_AUTHORITY_EFFECT_RECEIPTS {
        return Err(invalid());
    }
    let mut effect_receipt_ids = BTreeSet::new();
    for (storage_key, record) in &state.effect_receipts {
        let receipt = &record.receipt;
        let expected_request_digest = effect_request_digest_parts(
            &state.domain.authority_domain_id,
            state.domain.authority_epoch,
            &receipt.principal_id,
            &receipt.capability_id,
            receipt.capability_revision,
            &receipt.resource,
            &receipt.claim_id,
            receipt.claim_revision,
            receipt.fencing_token,
            &receipt.action_class,
            &receipt.audience,
            &receipt.idempotency_key,
            receipt.executor_uid,
            &receipt.executor_identity_digest,
        );
        let authorization_is_valid = match (receipt.state, record.authorization.as_ref()) {
            (LeaseAuthorityEffectState::Consumed, Some(authorization)) => {
                receipt.completion_idempotency_key.is_none()
                    && receipt.completion_evidence_digest.is_none()
                    && receipt.completed_at.is_none()
                    && receipt.terminal_authority_revision.is_none()
                    && authorization.schema_version
                        == super::LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION
                    && authorization.resource == receipt.resource
                    && authorization.claim_id == receipt.claim_id
                    && authorization.principal_id == receipt.principal_id
                    && authorization.capability_id == receipt.capability_id
                    && authorization.capability_revision == receipt.capability_revision
                    && authorization.claim_revision == receipt.claim_revision
                    && authorization.fencing_token == receipt.fencing_token
                    && authorization.action_class == receipt.action_class
                    && authorization.audience == receipt.audience
                    && authorization.operation_idempotency_key == receipt.idempotency_key
                    && authorization.executor_identity_digest.as_deref()
                        == Some(receipt.executor_identity_digest.as_str())
                    && authorization.issued_at == receipt.occurred_at
                    && authorization.authorization_expires_at == receipt.authorization_expires_at
                    && hex::decode(&authorization.proof).is_ok_and(|proof| proof.len() == 64)
            }
            (LeaseAuthorityEffectState::Completed | LeaseAuthorityEffectState::Uncertain, None) => {
                let occurred_at = chrono::DateTime::parse_from_rfc3339(&receipt.occurred_at);
                let completed_at = receipt
                    .completed_at
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc3339);
                receipt
                    .completion_idempotency_key
                    .as_deref()
                    .is_some_and(|key| !key.trim().is_empty())
                    && receipt
                        .completion_evidence_digest
                        .as_deref()
                        .is_some_and(valid_sha256_digest)
                    && receipt.terminal_authority_revision.is_some_and(|revision| {
                        revision > receipt.authority_revision
                            && revision <= state.authority.revision
                    })
                    && matches!((occurred_at, completed_at), (Ok(start), Some(Ok(end))) if end >= start)
            }
            _ => false,
        };
        if storage_key
            != &effect_receipt_storage_key(
                &state.domain.authority_domain_id,
                &receipt.principal_id,
                &receipt.resource,
                &receipt.action_class,
                &receipt.idempotency_key,
            )
            || receipt.schema_version != LEASE_AUTHORITY_EFFECT_RECEIPT_SCHEMA_VERSION
            || receipt.request_digest != expected_request_digest
            || receipt.receipt_id
                != format!(
                    "effect-receipt:{}",
                    &receipt.request_digest.trim_start_matches("sha256:")[..24]
                )
            || !effect_receipt_ids.insert(&receipt.receipt_id)
            || receipt.idempotency_key.trim().is_empty()
            || receipt.claim_id.trim().is_empty()
            || receipt.principal_id.trim().is_empty()
            || receipt.capability_id.trim().is_empty()
            || receipt.capability_revision == 0
            || receipt.claim_revision == 0
            || receipt.fencing_token == 0
            || receipt.action_class.trim().is_empty()
            || receipt.audience.trim().is_empty()
            || receipt.executor_uid == 0
            || !valid_sha256_digest(&receipt.executor_identity_digest)
            || receipt.authority_revision == 0
            || receipt.authority_revision > state.authority.revision
            || chrono::DateTime::parse_from_rfc3339(&receipt.occurred_at).is_err()
            || chrono::DateTime::parse_from_rfc3339(&receipt.authorization_expires_at).is_err()
            || !authorization_is_valid
        {
            return Err(invalid());
        }
    }

    if state.owners.schema_version != LEASE_AUTHORITY_OWNER_REGISTRY_SCHEMA_VERSION {
        return Err(invalid());
    }
    let mut highest_owner_revision = 0;
    for (storage_key, binding) in &state.owners.bindings {
        let registration = state.resources.registrations.get(storage_key);
        let principal = state.principals.principals.get(&binding.principal_id);
        let capability = state
            .principals
            .profile_capabilities
            .get(&binding.capability_id);
        if storage_key != &binding.resource.storage_key()
            || binding.resource.kind != LeaseResourceKind::Profile
            || !valid_sha256_digest(&binding.physical_identity_digest)
            || !valid_sha256_digest(&binding.process_instance_digest)
            || binding.owner_id.trim().is_empty()
            || binding.owner_generation == 0
            || binding.logical_browser_id.trim().is_empty()
            || binding.daemon_session_route.trim().is_empty()
            || binding.revision == 0
            || binding.revision > state.owners.revision
            || registration.is_none_or(|registration| {
                registration.resource != binding.resource
                    || registration.physical_identity_digest != binding.physical_identity_digest
            })
            || principal.is_none_or(|principal| principal.principal_id != binding.principal_id)
            || capability.is_none_or(|capability| {
                capability.capability_id != binding.capability_id
                    || capability.principal_id != binding.principal_id
                    || capability.profile_id != binding.resource.id
            })
        {
            return Err(invalid());
        }
        highest_owner_revision = highest_owner_revision.max(binding.revision);
    }
    if highest_owner_revision != state.owners.revision {
        return Err(invalid());
    }
    Ok(())
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn profile_enrollment_request_digest(
    profile_id: &str,
    physical_identity_digest: &str,
    operator_uid: u32,
    capability_digest: &str,
    expected_resource_revision: u64,
    idempotency_key: &str,
) -> String {
    let payload = format!(
        "agent-browser.lease-authority-profile-enrollment-request.v1\n{profile_id}\n{physical_identity_digest}\n{operator_uid}\n{capability_digest}\n{expected_resource_revision}\n{idempotency_key}"
    );
    format!("sha256:{:x}", Sha256::digest(payload.as_bytes()))
}

fn effect_receipt_storage_key(
    authority_domain_id: &str,
    principal_id: &str,
    resource: &LeaseResourceKey,
    action_class: &str,
    idempotency_key: &str,
) -> String {
    let payload = format!(
        "agent-browser.lease-authority-effect-receipt-key.v1\n{authority_domain_id}\n{principal_id}\n{}\n{action_class}\n{idempotency_key}",
        resource.storage_key()
    );
    format!("effect:{:x}", Sha256::digest(payload.as_bytes()))
}

#[allow(clippy::too_many_arguments)]
fn effect_request_digest(
    authority_domain_id: &str,
    authority_epoch: u64,
    principal_id: &str,
    capability_id: &str,
    capability_revision: u64,
    request: &AuthorizeLeaseEffectPayload,
    executor_uid: u32,
    executor_identity_digest: &str,
) -> String {
    effect_request_digest_parts(
        authority_domain_id,
        authority_epoch,
        principal_id,
        capability_id,
        capability_revision,
        &request.resource,
        &request.claim_id,
        request.claim_revision,
        request.fencing_token,
        &request.action_class,
        &request.audience,
        &request.idempotency_key,
        executor_uid,
        executor_identity_digest,
    )
}

#[allow(clippy::too_many_arguments)]
fn effect_request_digest_parts(
    authority_domain_id: &str,
    authority_epoch: u64,
    principal_id: &str,
    capability_id: &str,
    capability_revision: u64,
    resource: &LeaseResourceKey,
    claim_id: &str,
    claim_revision: u64,
    fencing_token: u64,
    action_class: &str,
    audience: &str,
    idempotency_key: &str,
    executor_uid: u32,
    executor_identity_digest: &str,
) -> String {
    let payload = format!(
        "agent-browser.lease-authority-effect-request.v1\n{authority_domain_id}\n{authority_epoch}\n{principal_id}\n{capability_id}\n{capability_revision}\n{}\n{claim_id}\n{claim_revision}\n{fencing_token}\n{action_class}\n{audience}\n{idempotency_key}\n{executor_uid}\n{executor_identity_digest}",
        resource.storage_key()
    );
    format!("sha256:{:x}", Sha256::digest(payload.as_bytes()))
}

fn verify_service_identity_challenge(
    identity: &LeaseAuthorityServiceIdentityProof,
    request: &LeaseAuthorityServiceChallengeRequest,
    observed_custody: &custody::LeaseAuthorityCustodyIdentity,
    verification_keys: &LeaseAuthorityVerificationKeyring,
) -> Result<(), LeaseAuthorityProtocolError> {
    if identity.schema_version != LEASE_AUTHORITY_SERVICE_IDENTITY_PROOF_SCHEMA_VERSION
        || identity.authority_domain_id != request.expected_authority_domain_id
        || identity.authority_epoch < request.minimum_authority_epoch
        || identity.nonce != request.nonce
        || identity.endpoint_identity_digest != observed_custody.endpoint_identity_digest
        || identity.executable_sha256 != observed_custody.executable_sha256
        || identity.boot_epoch.trim().is_empty()
        || !valid_sha256_digest(&identity.authority_domain_id)
        || !valid_sha256_digest(&identity.endpoint_identity_digest)
        || !valid_sha256_digest(&identity.executable_sha256)
    {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_service_identity_proof_invalid",
        });
    }
    let verification_key = verification_keys
        .verification_key(&identity.signing_key_id, identity.signing_key_epoch)
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_service_identity_proof_invalid",
        })?;
    let proof = hex::decode(&identity.proof).map_err(|_| LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_service_identity_proof_invalid",
    })?;
    signature::UnparsedPublicKey::new(&signature::ED25519, verification_key.public_key)
        .verify(service_identity_proof_payload(identity).as_bytes(), &proof)
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_service_identity_proof_invalid",
        })
}

fn service_identity_proof_payload(identity: &LeaseAuthorityServiceIdentityProof) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        identity.schema_version,
        identity.authority_domain_id,
        identity.authority_epoch,
        identity.boot_epoch,
        identity.endpoint_identity_digest,
        identity.executable_sha256,
        identity.signing_key_id,
        identity.signing_key_epoch,
        identity.nonce
    )
}

fn valid_bare_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn private_json_file_sha256(encoded: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(encoded);
    hasher.update(b"\n");
    format!("{:x}", hasher.finalize())
}

fn authority_store_generation_id(
    authority_epoch: u64,
    authority_revision: u64,
    protected_state_sha256: &str,
) -> Result<String, LeaseAuthorityProtocolError> {
    if authority_epoch == 0 || !valid_bare_sha256(protected_state_sha256) {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_store_generation_invalid",
        });
    }
    Ok(format!(
        "epoch-{authority_epoch}-revision-{authority_revision}-{}",
        &protected_state_sha256[..24]
    ))
}

fn authority_store_generation_component_is_safe(generation_id: &str) -> bool {
    !generation_id.is_empty()
        && generation_id.len() <= 128
        && generation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn store_io_error<E: std::fmt::Display>(_error: E) -> LeaseAuthorityProtocolError {
    LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_store_io_failed",
    }
}

fn inject_publication_fault(
    actual: Option<LeaseAuthorityPublicationFault>,
    expected: LeaseAuthorityPublicationFault,
) -> Result<(), LeaseAuthorityProtocolError> {
    if actual == Some(expected) {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_publication_fault_injected",
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeaseAuthorityProtocolError {
    code: &'static str,
}

impl LeaseAuthorityProtocolError {
    fn code(self) -> &'static str {
        self.code
    }
}

fn lease_authority_protocol_error(
    error: super::LeaseAuthorityError,
) -> LeaseAuthorityProtocolError {
    LeaseAuthorityProtocolError {
        code: error.as_str(),
    }
}

fn decode_lease_authority_request(
    encoded: &[u8],
) -> Result<LeaseAuthorityProtocolRequest, LeaseAuthorityProtocolError> {
    let envelope: LeaseAuthorityProtocolEnvelope =
        serde_json::from_slice(encoded).map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_request_invalid",
        })?;
    if envelope.schema_version != LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_schema_unsupported",
        });
    }
    match envelope.operation.as_str() {
        "service_challenge" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::ServiceChallenge)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "enroll_profile" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::EnrollProfile)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "acquire" => serde_json::from_value(envelope.payload)
            .map(Box::new)
            .map(LeaseAuthorityProtocolRequest::Acquire)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "authorize_effect" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::AuthorizeEffect)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "complete_effect" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::CompleteEffect)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "release" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::Release)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "recover_plan" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::RecoverPlan)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "recover" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::Recover)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "revoke_plan" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::RevokePlan)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "revoke" => serde_json::from_value(envelope.payload)
            .map(LeaseAuthorityProtocolRequest::Revoke)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "inspect" => Ok(LeaseAuthorityProtocolRequest::Inspect(envelope.payload)),
        _ => Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_operation_unsupported",
        }),
    }
}

fn read_lease_authority_frame<R: Read>(
    reader: &mut R,
) -> Result<Vec<u8>, LeaseAuthorityProtocolError> {
    let mut header = [0u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_read_failed",
        })?;
    let length =
        usize::try_from(u32::from_be_bytes(header)).map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_invalid",
        })?;
    if length == 0 {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_invalid",
        });
    }
    if length > LEASE_AUTHORITY_SERVICE_MAX_FRAME_BYTES {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_too_large",
        });
    }
    let mut encoded = vec![0u8; length];
    reader
        .read_exact(&mut encoded)
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_read_failed",
        })?;
    Ok(encoded)
}

fn write_lease_authority_frame<W: Write>(
    writer: &mut W,
    encoded: &[u8],
) -> Result<(), LeaseAuthorityProtocolError> {
    if encoded.is_empty() {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_invalid",
        });
    }
    if encoded.len() > LEASE_AUTHORITY_SERVICE_MAX_FRAME_BYTES {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_too_large",
        });
    }
    let length = u32::try_from(encoded.len()).map_err(|_| LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_frame_too_large",
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|_| writer.write_all(encoded))
        .and_then(|_| writer.flush())
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_write_failed",
        })
}

#[cfg(unix)]
fn derive_profile_enrollment_identity(
    profile_path: &str,
    peer: custody::LeaseAuthorityRequestPeerIdentity,
) -> Result<String, LeaseAuthorityProtocolError> {
    use std::os::unix::fs::MetadataExt;

    let requested = Path::new(profile_path);
    if peer.uid == 0 || profile_path.trim().is_empty() || !requested.is_absolute() {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_profile_enrollment_path_invalid",
        });
    }
    let canonical = fs::canonicalize(requested).map_err(|_| LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_profile_enrollment_path_unavailable",
    })?;
    let metadata = fs::metadata(&canonical).map_err(|_| LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_profile_enrollment_path_unavailable",
    })?;
    if !metadata.is_dir() || metadata.uid() != peer.uid || metadata.mode() & 0o022 != 0 {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_profile_enrollment_path_unprotected",
        });
    }
    let digest =
        crate::runtime_profile::canonical_profile_identity_digest(&canonical).map_err(|_| {
            LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_profile_enrollment_path_invalid",
            }
        })?;
    Ok(format!("sha256:{digest}"))
}

#[cfg(target_os = "linux")]
fn derive_effect_executor_identity(
    peer: custody::LeaseAuthorityRequestPeerIdentity,
) -> Result<String, LeaseAuthorityProtocolError> {
    if peer.uid == 0 || peer.pid <= 1 {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_effect_executor_invalid",
        });
    }
    let stat = fs::read_to_string(format!("/proc/{}/stat", peer.pid)).map_err(|_| {
        LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_effect_executor_unavailable",
        }
    })?;
    let after_comm = stat
        .rfind(')')
        .and_then(|index| stat.get(index + 2..))
        .ok_or(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_effect_executor_invalid",
        })?;
    let start_time = after_comm
        .split_whitespace()
        .nth(19)
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_effect_executor_invalid",
        })?;
    let executable_path = fs::canonicalize(format!("/proc/{}/exe", peer.pid)).map_err(|_| {
        LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_effect_executor_unavailable",
        }
    })?;
    let executable = fs::read(&executable_path).map_err(|_| LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_effect_executor_unavailable",
    })?;
    let executable_digest = format!("sha256:{:x}", Sha256::digest(executable));
    let payload = format!(
        "agent-browser.lease-authority-effect-executor.v1\n{}\n{}\n{}\n{start_time}\n{}\n{executable_digest}",
        peer.uid,
        peer.gid,
        peer.pid,
        executable_path.display()
    );
    Ok(format!("sha256:{:x}", Sha256::digest(payload.as_bytes())))
}

#[cfg(not(target_os = "linux"))]
fn derive_effect_executor_identity(
    _peer: custody::LeaseAuthorityRequestPeerIdentity,
) -> Result<String, LeaseAuthorityProtocolError> {
    Err(LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_effect_executor_platform_unsupported",
    })
}

#[cfg(not(unix))]
fn derive_profile_enrollment_identity(
    _profile_path: &str,
    _peer: custody::LeaseAuthorityRequestPeerIdentity,
) -> Result<String, LeaseAuthorityProtocolError> {
    Err(LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_profile_enrollment_platform_unsupported",
    })
}

fn dispatch_lease_authority_request(
    kernel: &mut LeaseAuthorityProtocolKernel,
    encoded: &[u8],
    custody: &custody::LeaseAuthorityCustodyIdentity,
    peer: custody::LeaseAuthorityRequestPeerIdentity,
    signing_key: &LeaseAuthoritySigningKey,
    administrative_context: Option<&LeaseAuthorityAdministrativeDispatchContext<'_>>,
) -> Result<Vec<u8>, LeaseAuthorityProtocolError> {
    if encoded.len() > LEASE_AUTHORITY_SERVICE_MAX_FRAME_BYTES {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_frame_too_large",
        });
    }
    match decode_lease_authority_request(encoded)? {
        LeaseAuthorityProtocolRequest::ServiceChallenge(request) => {
            let proof = kernel.issue_service_identity_challenge(&request, custody, signing_key)?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "service_identity",
                "payload": proof,
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::EnrollProfile(request) => {
            let physical_identity_digest =
                derive_profile_enrollment_identity(&request.profile_path, peer)?;
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let outcome = kernel.enroll_profile(
                request,
                peer.uid,
                &physical_identity_digest,
                &observed_at,
            )?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "profile_enrolled",
                "payload": {
                    "receipt": outcome.receipt,
                    "replayed": outcome.replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::Acquire(request) => {
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let LeaseAuthorityProtocolResponse::Acquired(outcome) =
                kernel.acquire(*request, &observed_at)?;
            let LeaseClaimAcquisitionOutcome {
                claim,
                receipt,
                replayed,
            } = outcome;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "acquired",
                "payload": {
                    "claim": claim,
                    "receipt": receipt,
                    "replayed": replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::AuthorizeEffect(request) => {
            let executor_identity_digest = derive_effect_executor_identity(peer)?;
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let outcome = kernel.authorize_effect(
                request,
                peer.uid,
                &executor_identity_digest,
                &observed_at,
                signing_key,
            )?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "effect_authorized",
                "payload": {
                    "authorization": outcome.authorization,
                    "receipt": outcome.receipt,
                    "replayed": outcome.replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::CompleteEffect(request) => {
            let executor_identity_digest = derive_effect_executor_identity(peer)?;
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let outcome = kernel.complete_effect(
                request,
                peer.uid,
                &executor_identity_digest,
                &observed_at,
            )?;
            let response_outcome = match outcome.receipt.state {
                LeaseAuthorityEffectState::Completed => "effect_completed",
                LeaseAuthorityEffectState::Uncertain => "effect_uncertain",
                LeaseAuthorityEffectState::Consumed => {
                    return Err(LeaseAuthorityProtocolError {
                        code: "lease_authority_protocol_state_invalid",
                    });
                }
            };
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": response_outcome,
                "payload": {
                    "receipt": outcome.receipt,
                    "replayed": outcome.replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::Release(request) => {
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let outcome = kernel.release(request, &observed_at, signing_key)?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "released",
                "payload": {
                    "receipt": outcome.receipt,
                    "replayed": outcome.replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::RevokePlan(_) | LeaseAuthorityProtocolRequest::Revoke(_)
            if !peer.is_root_administrator() =>
        {
            Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_administrator_peer_required",
            })
        }
        LeaseAuthorityProtocolRequest::RevokePlan(request) => {
            let context = administrative_context.ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_administrator_context_required",
            })?;
            let plan = kernel.plan_administrative_revocation(request, context, signing_key)?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "revocation_planned",
                "payload": plan,
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::Revoke(request) => {
            let context = administrative_context.ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_administrator_context_required",
            })?;
            let outcome = kernel.apply_administrative_revocation(request, context, signing_key)?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "revoked",
                "payload": {
                    "receipt": outcome.receipt,
                    "replayed": outcome.replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::RecoverPlan(request) => {
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let plan = kernel.plan_recovery(request, &observed_at, signing_key)?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "recovery_planned",
                "payload": plan,
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::Recover(request) => {
            let observed_at =
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            let outcome = kernel.apply_recovery(request, &observed_at, signing_key)?;
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
                "outcome": "recovered",
                "payload": {
                    "claim": outcome.claim,
                    "receipt": outcome.receipt,
                    "replayed": outcome.replayed,
                },
            }))
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_response_encode_failed",
            })
        }
        LeaseAuthorityProtocolRequest::Inspect(_) => Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_operation_not_implemented",
        }),
    }
}

#[cfg(test)]
fn serve_lease_authority_connection<R: Read, W: Write>(
    kernel: &mut LeaseAuthorityProtocolKernel,
    reader: &mut R,
    writer: &mut W,
    custody: &custody::LeaseAuthorityCustodyIdentity,
    peer: custody::LeaseAuthorityRequestPeerIdentity,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<(), LeaseAuthorityProtocolError> {
    let response = match read_lease_authority_frame(reader).and_then(|request| {
        dispatch_lease_authority_request(kernel, &request, custody, peer, signing_key, None)
    }) {
        Ok(response) => response,
        Err(error) => serde_json::to_vec(&serde_json::json!({
            "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
            "outcome": "error",
            "error": {"code": error.code()},
        }))
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_response_encode_failed",
        })?,
    };
    write_lease_authority_frame(writer, &response)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_AUTHORITY_DOMAIN_ID: &str =
        "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn test_kernel(
        authority: super::super::LeaseAuthorityState,
        principals: crate::native::service_principal::ServicePrincipalRegistry,
    ) -> LeaseAuthorityProtocolKernel {
        LeaseAuthorityProtocolKernel::bootstrap(
            TEST_AUTHORITY_DOMAIN_ID,
            7,
            "boot-1",
            authority,
            principals,
        )
        .unwrap()
    }

    fn test_load_context() -> LeaseAuthorityProtectedLoadContext<'static> {
        LeaseAuthorityProtectedLoadContext {
            expected_authority_domain_id: TEST_AUTHORITY_DOMAIN_ID,
            minimum_authority_epoch: 7,
        }
    }

    fn test_peer(uid: u32) -> custody::LeaseAuthorityRequestPeerIdentity {
        custody::LeaseAuthorityRequestPeerIdentity {
            uid,
            gid: if uid == 0 { 0 } else { 991 },
            pid: 4101,
        }
    }

    #[test]
    fn protocol_rejects_generic_signing_and_state_mutation_oracles() {
        for encoded in [
            br#"{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"sign","payload":{}}"#.as_slice(),
            br#"{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"mutate_state","payload":{}}"#.as_slice(),
        ] {
            let error = match decode_lease_authority_request(encoded) {
                Ok(_) => panic!("generic authority oracle must be rejected"),
                Err(error) => error,
            };
            assert_eq!(error.code(), "lease_authority_protocol_operation_unsupported");
        }
    }

    #[test]
    fn framed_transport_rejects_an_oversized_request_before_reading_its_payload() {
        let mut encoded = std::io::Cursor::new(
            u32::try_from(LEASE_AUTHORITY_SERVICE_MAX_FRAME_BYTES + 1)
                .unwrap()
                .to_be_bytes(),
        );
        let error = read_lease_authority_frame(&mut encoded).unwrap_err();
        assert_eq!(error.code(), "lease_authority_protocol_frame_too_large");
        assert_eq!(encoded.position(), 4);
    }

    #[test]
    fn typed_dispatcher_returns_only_a_nonce_bound_service_challenge() {
        let mut kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]);
        let request = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"service_challenge",
            "payload":{
                "nonce":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "expectedAuthorityDomainId":"sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "minimumAuthorityEpoch":7
            }
        }"#;

        let encoded = dispatch_lease_authority_request(
            &mut kernel,
            request,
            &custody,
            test_peer(1000),
            &signing_key,
            None,
        )
        .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            response["schemaVersion"],
            "agent-browser.lease-authority-response.v1"
        );
        assert_eq!(response["outcome"], "service_identity");
        let proof: LeaseAuthorityServiceIdentityProof =
            serde_json::from_value(response["payload"].clone()).unwrap();
        let challenge = LeaseAuthorityServiceChallengeRequest {
            nonce: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            expected_authority_domain_id: TEST_AUTHORITY_DOMAIN_ID.to_string(),
            minimum_authority_epoch: 7,
        };
        verify_service_identity_challenge(
            &proof,
            &challenge,
            &custody,
            &LeaseAuthorityVerificationKeyring::from_active(&signing_key),
        )
        .unwrap();
    }

    #[test]
    fn framed_service_returns_a_typed_error_for_a_generic_signing_oracle() {
        let mut kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]);
        let request =
            br#"{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"sign","payload":{}}"#;
        let mut framed_request = Vec::new();
        write_lease_authority_frame(&mut framed_request, request).unwrap();
        let mut reader = std::io::Cursor::new(framed_request);
        let mut framed_response = Vec::new();

        serve_lease_authority_connection(
            &mut kernel,
            &mut reader,
            &mut framed_response,
            &custody,
            test_peer(1000),
            &signing_key,
        )
        .unwrap();

        let mut response_reader = std::io::Cursor::new(framed_response);
        let response: serde_json::Value =
            serde_json::from_slice(&read_lease_authority_frame(&mut response_reader).unwrap())
                .unwrap();
        assert_eq!(response["outcome"], "error");
        assert_eq!(
            response["error"]["code"],
            "lease_authority_protocol_operation_unsupported"
        );
    }

    #[test]
    fn administrative_dispatch_requires_a_kernel_authenticated_root_peer() {
        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]);
        let request = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"revoke_plan",
            "payload":{
                "resource":{"kind":"profile","id":"last30days-social"},
                "claimId":"claim:last30days",
                "claimRevision":1,
                "fencingToken":1,
                "idempotencyKey":"revoke:last30days:1",
                "reasonCode":"abandoned_strict_holder"
            }
        }"#;

        let mut unprivileged_kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let error = dispatch_lease_authority_request(
            &mut unprivileged_kernel,
            request,
            &custody,
            test_peer(1000),
            &signing_key,
            None,
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_administrator_peer_required"
        );

        let mut root_kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let error = dispatch_lease_authority_request(
            &mut root_kernel,
            request,
            &custody,
            test_peer(0),
            &signing_key,
            None,
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_administrator_context_required"
        );
    }

    #[test]
    fn administrative_revoke_plan_and_apply_are_authority_timed_durable_and_replayable() {
        let administrator_capability = b"root-administrator-capability-material-v1";
        let mut authority = super::super::LeaseAuthorityState::default();
        authority
            .bootstrap_administrator("administrator:local-root", administrator_capability)
            .unwrap();
        let claim = authority
            .acquire(super::super::AcquireLeaseClaimRequest {
                resource: LeaseResourceKey::profile("last30days-social"),
                parent_claim_id: None,
                principal_id: "principal:last30days".to_string(),
                capability_id: "capability:last30days-social".to_string(),
                capability_revision: 1,
                mode: LeaseClaimMode::Strict,
                expected_claim_revision: 0,
                idempotency_key: "acquire:last30days:strict-1".to_string(),
                now: "2026-08-31T12:00:00Z".to_string(),
                expires_at: "2026-08-31T12:05:00Z".to_string(),
                transition_deadline: Some("2026-08-31T12:01:00Z".to_string()),
                recovery_controller_id: Some("capability:last30days-social".to_string()),
                boot_epoch: Some("boot-1".to_string()),
                owner_generation: Some(57),
            })
            .unwrap();
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-admin-dispatch-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        store
            .publish(
                &test_kernel(
                    authority,
                    crate::native::service_principal::ServicePrincipalRegistry::default(),
                ),
                None,
            )
            .unwrap();
        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]);
        let plan_request = format!(
            r#"{{
                "schemaVersion":"agent-browser.lease-authority-request.v1",
                "operation":"revoke_plan",
                "payload":{{
                    "resource":{{"kind":"profile","id":"last30days-social"}},
                    "claimId":"{}",
                    "claimRevision":{},
                    "fencingToken":{},
                    "idempotencyKey":"revoke:last30days:strict-1",
                    "reasonCode":"abandoned_strict_holder"
                }}
            }}"#,
            claim.claim_id, claim.revision, claim.fencing_token
        );

        let mut planning = store.load_for_mutation(test_load_context()).unwrap();
        let plan_context = LeaseAuthorityAdministrativeDispatchContext {
            administrator_id: "administrator:local-root",
            administrator_revision: 1,
            raw_administrator_capability: administrator_capability,
            authority_observed_at: "2026-08-31T12:01:00Z",
        };
        let planned = dispatch_lease_authority_request(
            &mut planning,
            plan_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&plan_context),
        )
        .unwrap();
        let planned: serde_json::Value = serde_json::from_slice(&planned).unwrap();
        assert_eq!(planned["outcome"], "revocation_planned");
        assert_eq!(planned["payload"]["issuedAt"], "2026-08-31T12:01:00Z");
        assert_eq!(
            planned["payload"]["authorizationExpiresAt"],
            "2026-08-31T12:03:00.000000000Z"
        );
        assert_eq!(planned["payload"]["replayed"], false);
        assert!(planned.to_string().find("proof").is_none());
        let plan_id = planned["payload"]["planId"].as_str().unwrap().to_string();
        store.publish(&planning, None).unwrap();

        let mut replaying_plan = store.load_for_mutation(test_load_context()).unwrap();
        let replay_context = LeaseAuthorityAdministrativeDispatchContext {
            authority_observed_at: "2026-08-31T12:01:15Z",
            ..plan_context
        };
        let replayed = dispatch_lease_authority_request(
            &mut replaying_plan,
            plan_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&replay_context),
        )
        .unwrap();
        let replayed: serde_json::Value = serde_json::from_slice(&replayed).unwrap();
        assert_eq!(replayed["payload"]["planId"], plan_id);
        assert_eq!(replayed["payload"]["issuedAt"], "2026-08-31T12:01:00Z");
        assert_eq!(replayed["payload"]["replayed"], true);
        store.publish(&replaying_plan, None).unwrap();

        let apply_request = format!(
            r#"{{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"revoke","payload":{{"planId":"{plan_id}"}}}}"#
        );
        let mut applying = store.load_for_mutation(test_load_context()).unwrap();
        let apply_context = LeaseAuthorityAdministrativeDispatchContext {
            authority_observed_at: "2026-08-31T12:01:30Z",
            ..plan_context
        };
        let applied = dispatch_lease_authority_request(
            &mut applying,
            apply_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&apply_context),
        )
        .unwrap();
        let applied: serde_json::Value = serde_json::from_slice(&applied).unwrap();
        assert_eq!(applied["outcome"], "revoked");
        assert_eq!(applied["payload"]["replayed"], false);
        assert_eq!(applied["payload"]["receipt"]["terminalResult"], "revoked");
        let receipt = applied["payload"]["receipt"].clone();
        store.publish(&applying, None).unwrap();

        let restarted = store.load(test_load_context()).unwrap();
        assert!(restarted
            .state
            .authority
            .current_claim(
                &LeaseResourceKey::profile("last30days-social"),
                "2026-08-31T12:02:00Z"
            )
            .is_none());
        assert_eq!(
            restarted.state.domain.authority_time_floor,
            "2026-08-31T12:01:30Z"
        );

        let mut replaying_terminal_plan = store.load_for_mutation(test_load_context()).unwrap();
        let terminal_plan_context = LeaseAuthorityAdministrativeDispatchContext {
            authority_observed_at: "2026-08-31T12:01:45Z",
            ..plan_context
        };
        let replayed_terminal_plan = dispatch_lease_authority_request(
            &mut replaying_terminal_plan,
            plan_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&terminal_plan_context),
        )
        .unwrap();
        let replayed_terminal_plan: serde_json::Value =
            serde_json::from_slice(&replayed_terminal_plan).unwrap();
        assert_eq!(replayed_terminal_plan["payload"]["planId"], plan_id);
        assert_eq!(replayed_terminal_plan["payload"]["replayed"], true);
        store.publish(&replaying_terminal_plan, None).unwrap();

        let mut replaying_apply = store.load_for_mutation(test_load_context()).unwrap();
        let late_context = LeaseAuthorityAdministrativeDispatchContext {
            authority_observed_at: "2026-08-31T13:00:00Z",
            ..plan_context
        };
        let replayed = dispatch_lease_authority_request(
            &mut replaying_apply,
            apply_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&late_context),
        )
        .unwrap();
        let replayed: serde_json::Value = serde_json::from_slice(&replayed).unwrap();
        assert_eq!(replayed["payload"]["replayed"], true);
        assert_eq!(replayed["payload"]["receipt"], receipt);
        store.publish(&replaying_apply, None).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn authority_time_floor_survives_restart_and_cannot_move_backward() {
        let mut kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        assert_eq!(
            kernel
                .observe_authority_time("2026-08-31T12:00:00Z")
                .unwrap(),
            "2026-08-31T12:00:00Z"
        );
        let restarted = LeaseAuthorityProtocolKernel::from_protected_state(
            &kernel.encode_protected_state().unwrap(),
            test_load_context(),
        )
        .unwrap();
        let mut restarted = restarted;
        assert_eq!(
            restarted
                .observe_authority_time("2026-08-31T11:00:00Z")
                .unwrap(),
            "2026-08-31T12:00:00Z"
        );
    }

    #[test]
    fn caller_time_and_stale_claim_revision_cannot_drive_administrative_revoke() {
        let caller_timed = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"revoke_plan",
            "payload":{
                "resource":{"kind":"profile","id":"last30days-social"},
                "claimId":"claim:last30days",
                "claimRevision":1,
                "fencingToken":1,
                "idempotencyKey":"revoke:last30days:1",
                "reasonCode":"abandoned_strict_holder",
                "issuedAt":"2099-01-01T00:00:00Z"
            }
        }"#;
        let error = match decode_lease_authority_request(caller_timed) {
            Ok(_) => panic!("caller-supplied administrative time must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_request_invalid");

        let administrator_capability = b"root-administrator-capability-material-v1";
        let mut authority = super::super::LeaseAuthorityState::default();
        authority
            .bootstrap_administrator("administrator:local-root", administrator_capability)
            .unwrap();
        let claim = authority
            .acquire(super::super::AcquireLeaseClaimRequest {
                resource: LeaseResourceKey::profile("last30days-social"),
                parent_claim_id: None,
                principal_id: "principal:last30days".to_string(),
                capability_id: "capability:last30days-social".to_string(),
                capability_revision: 1,
                mode: LeaseClaimMode::Strict,
                expected_claim_revision: 0,
                idempotency_key: "acquire:last30days:strict-stale".to_string(),
                now: "2026-08-31T12:00:00Z".to_string(),
                expires_at: "2026-08-31T12:05:00Z".to_string(),
                transition_deadline: Some("2026-08-31T12:01:00Z".to_string()),
                recovery_controller_id: Some("capability:last30days-social".to_string()),
                boot_epoch: Some("boot-1".to_string()),
                owner_generation: Some(57),
            })
            .unwrap();
        let mut kernel = test_kernel(
            authority,
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]);
        let context = LeaseAuthorityAdministrativeDispatchContext {
            administrator_id: "administrator:local-root",
            administrator_revision: 1,
            raw_administrator_capability: administrator_capability,
            authority_observed_at: "2026-08-31T12:01:00Z",
        };
        let plan_request = format!(
            r#"{{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"revoke_plan","payload":{{"resource":{{"kind":"profile","id":"last30days-social"}},"claimId":"{}","claimRevision":{},"fencingToken":{},"idempotencyKey":"revoke:last30days:stale","reasonCode":"abandoned_strict_holder"}}}}"#,
            claim.claim_id, claim.revision, claim.fencing_token
        );
        let planned = dispatch_lease_authority_request(
            &mut kernel,
            plan_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&context),
        )
        .unwrap();
        let planned: serde_json::Value = serde_json::from_slice(&planned).unwrap();
        let plan_id = planned["payload"]["planId"].as_str().unwrap();
        kernel
            .state
            .authority
            .active_claims
            .get_mut(&LeaseResourceKey::profile("last30days-social").storage_key())
            .unwrap()
            .revision += 1;
        let apply_request = format!(
            r#"{{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"revoke","payload":{{"planId":"{plan_id}"}}}}"#
        );
        let stale_context = LeaseAuthorityAdministrativeDispatchContext {
            authority_observed_at: "2026-08-31T12:01:30Z",
            ..context
        };
        let error = dispatch_lease_authority_request(
            &mut kernel,
            apply_request.as_bytes(),
            &custody,
            test_peer(0),
            &signing_key,
            Some(&stale_context),
        )
        .unwrap_err();
        assert_eq!(error.code(), "stale_claim");
        assert!(kernel
            .state
            .authority
            .current_claim(
                &LeaseResourceKey::profile("last30days-social"),
                "2026-08-31T12:02:00Z"
            )
            .is_some());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn protected_service_rejects_non_root_before_consulting_installed_paths() {
        let error = service::validate_linux_service_launch(
            1000,
            4100,
            std::path::Path::new("/missing/candidate"),
        )
        .unwrap_err();
        assert_eq!(error.code(), "lease_authority_service_root_required");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn protected_service_accepts_only_a_banked_root_generation_path() {
        service::validate_linux_service_launch(
            0,
            4100,
            std::path::Path::new(
                "/usr/local/libexec/agent-browser/lease-authority/generations/generation-7/agent-browser",
            ),
        )
        .unwrap();
        for candidate in [
            "/home/operator/agent-browser",
            "/usr/local/libexec/agent-browser/lease-authority/generations/agent-browser",
            "/usr/local/libexec/agent-browser/lease-authority/generations/../candidate/agent-browser",
            "/usr/local/libexec/agent-browser/lease-authority/generations/generation-7/other",
        ] {
            let error = service::validate_linux_service_launch(
                0,
                4100,
                std::path::Path::new(candidate),
            )
            .unwrap_err();
            assert_eq!(
                error.code(),
                "lease_authority_service_executable_untrusted"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn protected_service_accepts_only_its_exact_systemd_socket_activation() {
        service::validate_systemd_socket_activation(Some(4100), Some(1), 4100).unwrap();
        for (listen_pid, listen_fds) in [
            (None, Some(1)),
            (Some(4099), Some(1)),
            (Some(4100), None),
            (Some(4100), Some(0)),
            (Some(4100), Some(2)),
        ] {
            let error = service::validate_systemd_socket_activation(listen_pid, listen_fds, 4100)
                .unwrap_err();
            assert_eq!(
                error.code(),
                "lease_authority_service_socket_activation_invalid"
            );
        }
    }

    #[test]
    fn protected_service_store_open_never_bootstraps_missing_state() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-missing-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let error = match LeaseAuthorityDurableStore::open_existing(&root) {
            Ok(_) => panic!("the online service must not create its own authority store"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_store_io_failed");
        assert!(!root.exists());
    }

    #[test]
    fn acquire_request_is_typed_and_redacts_the_profile_capability() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"acquire",
            "payload":{
                "rawCapability":[108,97,115,116,51,48,100,97,121,115,45,115,101,99,114,101,116],
                "resource":{"kind":"profile","id":"last30days-social"},
                "parentClaimId":null,
                "mode":"ephemeral",
                "expectedClaimRevision":0,
                "idempotencyKey":"acquire:last30days:tick-1",
                "recoveryControllerId":null
            }
        }"#;
        let request = match decode_lease_authority_request(encoded) {
            Ok(request) => request,
            Err(error) => panic!("typed acquire request must decode: {}", error.code()),
        };
        let LeaseAuthorityProtocolRequest::Acquire(acquire) = request else {
            panic!("decoded operation must remain acquire");
        };
        assert_eq!(
            acquire.resource,
            super::super::LeaseResourceKey::profile("last30days-social")
        );
        assert_eq!(acquire.mode, super::super::LeaseClaimMode::Ephemeral);
        assert_eq!(acquire.raw_capability.expose(), b"last30days-secret");
        assert!(!format!("{acquire:?}").contains("last30days-secret"));
    }

    #[test]
    fn profile_enrollment_request_has_no_caller_owned_principal_or_physical_identity() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"enroll_profile",
            "payload":{
                "rawCapability":[108,97,115,116,51,48,100,97,121,115,45,115,101,99,114,101,116],
                "profileId":"last30days-social",
                "profilePath":"/home/operator/.agent-browser/runtime-profiles/last30days-social/user-data",
                "expectedResourceRevision":0,
                "idempotencyKey":"enroll:last30days:1"
            }
        }"#;
        assert!(decode_lease_authority_request(encoded).is_ok());

        for forbidden in [
            r#""principalId":"principal:invented","#,
            r#""physicalIdentityDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
            r#""operatorUid":1000,"#,
            r#""registeredAt":"2099-01-01T00:00:00Z","#,
        ] {
            let injected = String::from_utf8(encoded.to_vec())
                .unwrap()
                .replace("\"profileId\":", &format!("{forbidden}\"profileId\":"));
            assert!(decode_lease_authority_request(injected.as_bytes()).is_err());
        }
    }

    #[test]
    fn release_request_has_no_caller_owned_time_identity_or_authorization() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"release",
            "payload":{
                "rawCapability":[108,97,115,116,51,48,100,97,121,115,45,115,101,99,114,101,116],
                "resource":{"kind":"profile","id":"last30days-social"},
                "claimId":"claim:last30days",
                "claimRevision":1,
                "fencingToken":1,
                "idempotencyKey":"release:last30days:1"
            }
        }"#;
        assert!(decode_lease_authority_request(encoded).is_ok());

        for forbidden in [
            r#""principalId":"principal:invented","#,
            r#""capabilityId":"capability:invented","#,
            r#""now":"2099-01-01T00:00:00Z","#,
            r#""authorization":{"proof":"invented"},"#,
        ] {
            let injected = String::from_utf8(encoded.to_vec())
                .unwrap()
                .replace("\"resource\":", &format!("{forbidden}\"resource\":"));
            assert!(decode_lease_authority_request(injected.as_bytes()).is_err());
        }
    }

    #[test]
    fn effect_request_has_no_caller_owned_time_identity_or_proof() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"authorize_effect",
            "payload":{
                "rawCapability":[108,97,115,116,51,48,100,97,121,115,45,115,101,99,114,101,116],
                "resource":{"kind":"profile","id":"last30days-social"},
                "claimId":"claim:last30days",
                "claimRevision":1,
                "fencingToken":1,
                "actionClass":"browser_launch",
                "audience":"daemon-session:last30days",
                "idempotencyKey":"launch:last30days:1"
            }
        }"#;
        assert!(decode_lease_authority_request(encoded).is_ok());

        for forbidden in [
            r#""principalId":"principal:invented","#,
            r#""capabilityId":"capability:invented","#,
            r#""executorIdentityDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
            r#""issuedAt":"2099-01-01T00:00:00Z","#,
            r#""authorizationExpiresAt":"2099-01-01T00:02:00Z","#,
            r#""proof":"invented","#,
        ] {
            let injected = String::from_utf8(encoded.to_vec())
                .unwrap()
                .replace("\"resource\":", &format!("{forbidden}\"resource\":"));
            assert!(decode_lease_authority_request(injected.as_bytes()).is_err());
        }
    }

    #[test]
    fn effect_receipt_keys_are_domain_principal_resource_and_action_scoped() {
        let domain_a = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let domain_b = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let baseline = effect_receipt_storage_key(
            domain_a,
            "principal:last30days",
            &LeaseResourceKey::profile("last30days-social"),
            "browser_launch",
            "launch:tick-1",
        );
        for distinct in [
            effect_receipt_storage_key(
                domain_b,
                "principal:last30days",
                &LeaseResourceKey::profile("last30days-social"),
                "browser_launch",
                "launch:tick-1",
            ),
            effect_receipt_storage_key(
                domain_a,
                "principal:foreign",
                &LeaseResourceKey::profile("last30days-social"),
                "browser_launch",
                "launch:tick-1",
            ),
            effect_receipt_storage_key(
                domain_a,
                "principal:last30days",
                &LeaseResourceKey::profile("other-profile"),
                "browser_launch",
                "launch:tick-1",
            ),
            effect_receipt_storage_key(
                domain_a,
                "principal:last30days",
                &LeaseResourceKey::profile("last30days-social"),
                "tab_mutation",
                "launch:tick-1",
            ),
        ] {
            assert_ne!(distinct, baseline);
        }
    }

    #[test]
    fn effect_completion_request_has_no_caller_owned_time_executor_or_authorization() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"complete_effect",
            "payload":{
                "receiptId":"effect-receipt:0123456789abcdef01234567",
                "result":"completed",
                "completionEvidenceDigest":"sha256:5555555555555555555555555555555555555555555555555555555555555555",
                "completionIdempotencyKey":"complete:last30days:1"
            }
        }"#;
        assert!(decode_lease_authority_request(encoded).is_ok());

        for forbidden in [
            r#""executorIdentityDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
            r#""executorUid":1000,"#,
            r#""completedAt":"2099-01-01T00:00:00Z","#,
            r#""authorization":{"proof":"invented"},"#,
        ] {
            let injected = String::from_utf8(encoded.to_vec())
                .unwrap()
                .replace("\"receiptId\":", &format!("{forbidden}\"receiptId\":"));
            assert!(decode_lease_authority_request(injected.as_bytes()).is_err());
        }
    }

    #[test]
    fn protected_profile_enrollment_is_uid_bound_durable_and_acquisition_ready() {
        let raw_capability = "last30days-profile-capability-secret-v1";
        let physical_identity_digest =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let request = || EnrollProfileLeaseAuthorityPayload {
            raw_capability: LeaseAuthoritySecret(raw_capability.as_bytes().to_vec()),
            profile_id: "last30days-social".to_string(),
            profile_path: "/private/path/never-persisted".to_string(),
            expected_resource_revision: 0,
            idempotency_key: "enroll:last30days:1".to_string(),
        };
        let mut kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );

        let enrolled = kernel
            .enroll_profile(
                request(),
                1000,
                physical_identity_digest,
                "2026-09-01T12:00:00Z",
            )
            .unwrap();
        assert!(!enrolled.replayed);
        assert_eq!(
            enrolled.receipt.principal_id,
            "principal:local-uid:1000:profile:last30days-social"
        );
        assert_eq!(enrolled.receipt.operator_uid, 1000);
        assert_eq!(enrolled.receipt.resource_revision, 1);
        assert_eq!(enrolled.receipt.occurred_at, "2026-09-01T12:00:00Z");

        let encoded = kernel.encode_protected_state().unwrap();
        let encoded_text = String::from_utf8(encoded.clone()).unwrap();
        assert!(!encoded_text.contains(raw_capability));
        assert!(!encoded_text.contains("/private/path/never-persisted"));
        let mut restarted =
            LeaseAuthorityProtocolKernel::from_protected_state(&encoded, test_load_context())
                .unwrap();
        let replayed = restarted
            .enroll_profile(
                request(),
                1000,
                physical_identity_digest,
                "2026-09-01T12:00:30Z",
            )
            .unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.receipt, enrolled.receipt);

        let mut rebound =
            LeaseAuthorityProtocolKernel::from_protected_state(&encoded, test_load_context())
                .unwrap();
        rebound.state.principals = ServicePrincipalRegistry::default();
        let registration = rebound
            .state
            .resources
            .registrations
            .get_mut(&LeaseResourceKey::profile("last30days-social").storage_key())
            .unwrap();
        registration.physical_identity_digest =
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
        registration.revision = 2;
        rebound.state.resources.revision = 2;
        LeaseAuthorityProtocolKernel::from_protected_state(
            &rebound.encode_protected_state().unwrap(),
            test_load_context(),
        )
        .expect("historical enrollment receipt must not invalidate later rotation or rebinding");

        let acquire = AcquireLeaseAuthorityPayload {
            raw_capability: LeaseAuthoritySecret(raw_capability.as_bytes().to_vec()),
            resource: LeaseResourceKey::profile("last30days-social"),
            parent_claim_id: None,
            mode: LeaseClaimMode::Ephemeral,
            expected_claim_revision: Some(0),
            idempotency_key: "acquire:last30days:after-enrollment".to_string(),
            recovery_controller_id: None,
        };
        let LeaseAuthorityProtocolResponse::Acquired(acquired) =
            restarted.acquire(acquire, "2026-09-01T12:01:00Z").unwrap();
        let claim = acquired.claim.unwrap();
        assert_eq!(claim.principal_id(), enrolled.receipt.principal_id);
        assert_eq!(claim.capability_id(), enrolled.receipt.capability_id);

        let LeaseAuthorityProtocolResponse::Acquired(rejoined) = restarted
            .acquire(
                AcquireLeaseAuthorityPayload {
                    raw_capability: LeaseAuthoritySecret(raw_capability.as_bytes().to_vec()),
                    resource: LeaseResourceKey::profile("last30days-social"),
                    parent_claim_id: None,
                    mode: LeaseClaimMode::Ephemeral,
                    expected_claim_revision: None,
                    idempotency_key: "acquire:last30days:new-worker".to_string(),
                    recovery_controller_id: None,
                },
                "2026-09-01T12:01:10Z",
            )
            .unwrap();
        let rejoined_claim = rejoined.claim.unwrap();
        assert!(!rejoined.replayed);
        assert_eq!(rejoined_claim.claim_id(), claim.claim_id());
        assert_eq!(rejoined_claim.revision(), claim.revision());
        assert_eq!(rejoined_claim.fencing_token(), claim.fencing_token());
        assert_eq!(rejoined_claim.expires_at(), claim.expires_at());

        let strict_error = match restarted.acquire(
            AcquireLeaseAuthorityPayload {
                raw_capability: LeaseAuthoritySecret(raw_capability.as_bytes().to_vec()),
                resource: LeaseResourceKey::profile("last30days-social"),
                parent_claim_id: None,
                mode: LeaseClaimMode::Strict,
                expected_claim_revision: None,
                idempotency_key: "acquire:last30days:strict-without-cas".to_string(),
                recovery_controller_id: Some(enrolled.receipt.capability_id.clone()),
            },
            "2026-09-01T12:01:20Z",
        ) {
            Ok(_) => panic!("strict acquisition without an explicit revision must fail"),
            Err(error) => error,
        };
        assert_eq!(
            strict_error.code(),
            "lease_authority_protocol_strict_expected_revision_required"
        );

        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x6b; 32]);
        let executor_identity_digest =
            "sha256:3333333333333333333333333333333333333333333333333333333333333333";
        let effect_request = || AuthorizeLeaseEffectPayload {
            raw_capability: LeaseAuthoritySecret(raw_capability.as_bytes().to_vec()),
            resource: LeaseResourceKey::profile("last30days-social"),
            claim_id: claim.claim_id().to_string(),
            claim_revision: claim.revision(),
            fencing_token: claim.fencing_token(),
            action_class: "browser_launch".to_string(),
            audience: "daemon-session:last30days".to_string(),
            idempotency_key: "launch:last30days:after-enrollment".to_string(),
        };
        let mut unsupported_scope = effect_request();
        unsupported_scope.action_class = "arbitrary_signing_oracle".to_string();
        let error = restarted
            .authorize_effect(
                unsupported_scope,
                1000,
                executor_identity_digest,
                "2026-09-01T12:01:20Z",
                &signing_key,
            )
            .unwrap_err();
        assert_eq!(error.code(), "lease_authority_protocol_effect_invalid");
        let authorized = restarted
            .authorize_effect(
                effect_request(),
                1000,
                executor_identity_digest,
                "2026-09-01T12:01:30Z",
                &signing_key,
            )
            .unwrap();
        assert!(!authorized.replayed);
        assert_eq!(
            authorized
                .authorization
                .as_ref()
                .and_then(|authorization| authorization.executor_identity_digest.as_deref()),
            Some(executor_identity_digest)
        );
        super::super::verify_effect_authorization(
            authorized.authorization.as_ref().unwrap(),
            &LeaseAuthorityVerificationKeyring::from_active(&signing_key),
        )
        .unwrap();
        let authority_revision = restarted.state.authority.revision();
        let encoded_with_effect = restarted.encode_protected_state().unwrap();
        let mut restarted = LeaseAuthorityProtocolKernel::from_protected_state(
            &encoded_with_effect,
            test_load_context(),
        )
        .unwrap();
        let replayed = restarted
            .authorize_effect(
                effect_request(),
                1000,
                executor_identity_digest,
                "2026-09-01T12:01:45Z",
                &signing_key,
            )
            .unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.receipt, authorized.receipt);
        assert!(replayed.authorization.is_none());
        assert_eq!(restarted.state.authority.revision(), authority_revision);

        let different_executor =
            "sha256:4444444444444444444444444444444444444444444444444444444444444444";
        let error = restarted
            .authorize_effect(
                effect_request(),
                1000,
                different_executor,
                "2026-09-01T12:01:50Z",
                &signing_key,
            )
            .unwrap_err();
        assert_eq!(error.code(), "lease_authority_idempotency_conflict");

        let mut tampered = authorized.authorization.unwrap();
        tampered.executor_identity_digest = Some(different_executor.to_string());
        assert_eq!(
            super::super::verify_effect_authorization(
                &tampered,
                &LeaseAuthorityVerificationKeyring::from_active(&signing_key),
            ),
            Err(super::super::LeaseAuthorityError::InvalidEffectProof)
        );

        let completion = restarted
            .complete_effect(
                CompleteLeaseEffectPayload {
                    receipt_id: authorized.receipt.receipt_id.clone(),
                    result: CompleteLeaseEffectResult::Uncertain,
                    completion_evidence_digest:
                        "sha256:5555555555555555555555555555555555555555555555555555555555555555"
                            .to_string(),
                    completion_idempotency_key: "uncertain:last30days:1".to_string(),
                },
                1000,
                executor_identity_digest,
                "2026-09-01T12:02:00Z",
            )
            .unwrap();
        assert!(!completion.replayed);
        assert_eq!(
            completion.receipt.state,
            LeaseAuthorityEffectState::Uncertain
        );
        assert!(
            restarted
                .state
                .effect_receipts
                .values()
                .all(|record| record.authorization.is_none()),
            "terminal effect records must scrub the executable bearer"
        );
        LeaseAuthorityProtocolKernel::from_protected_state(
            &restarted.encode_protected_state().unwrap(),
            test_load_context(),
        )
        .expect("uncertain terminal receipt must remain restart-valid");
    }

    #[cfg(unix)]
    #[test]
    fn profile_enrollment_path_identity_is_peer_owned_and_alias_canonical() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "agent-browser-profile-enrollment-{}",
            uuid::Uuid::new_v4()
        ));
        let profile = root.join("profile");
        fs::create_dir_all(&profile).unwrap();
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o700)).unwrap();
        let current_uid = unsafe { libc::geteuid() };
        let operator_uid = if current_uid == 0 { 1000 } else { current_uid };
        if current_uid == 0 {
            let path = std::ffi::CString::new(profile.as_os_str().as_bytes()).unwrap();
            let result = unsafe { libc::chown(path.as_ptr(), operator_uid, u32::MAX) };
            assert_eq!(result, 0);
        }
        let peer = custody::LeaseAuthorityRequestPeerIdentity {
            uid: operator_uid,
            gid: 991,
            pid: 4101,
        };
        let direct = derive_profile_enrollment_identity(profile.to_str().unwrap(), peer).unwrap();
        let alias = root.join("profile-alias");
        std::os::unix::fs::symlink(&profile, &alias).unwrap();
        let through_alias =
            derive_profile_enrollment_identity(alias.to_str().unwrap(), peer).unwrap();
        assert_eq!(direct, through_alias);

        let wrong_peer = custody::LeaseAuthorityRequestPeerIdentity {
            uid: operator_uid.saturating_add(1),
            gid: 991,
            pid: 4102,
        };
        let error =
            derive_profile_enrollment_identity(profile.to_str().unwrap(), wrong_peer).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_profile_enrollment_path_unprotected"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn acquire_request_rejects_caller_owned_time_and_owner_evidence() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"acquire",
            "payload":{
                "rawCapability":[108,97,115,116,51,48,100,97,121,115,45,115,101,99,114,101,116],
                "resource":{"kind":"profile","id":"last30days-social"},
                "parentClaimId":null,
                "mode":"ephemeral",
                "expectedClaimRevision":0,
                "idempotencyKey":"acquire:last30days:caller-authority",
                "recoveryControllerId":null,
                "now":"2099-01-01T00:00:00Z",
                "expiresAt":"2199-01-01T00:00:00Z",
                "bootEpoch":"invented-boot",
                "ownerGeneration":999
            }
        }"#;
        let error = match decode_lease_authority_request(encoded) {
            Ok(_) => panic!("caller-owned acquisition authority must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_request_invalid");
    }

    #[test]
    fn strict_recovery_plan_is_controller_authenticated_durable_and_exact() {
        let raw_controller =
            "abpc_v1_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let mut principals = ServicePrincipalRegistry::default();
        let registered = crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:last30days".to_string(),
                display_name: None,
                profile_id: "last30days-social".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("test".to_string()),
            },
            raw_controller,
        )
        .unwrap();
        let mut authority = LeaseAuthorityState::default();
        let claim = authority
            .acquire_with_receipt(super::super::AcquireLeaseClaimRequest {
                resource: LeaseResourceKey::profile("last30days-social"),
                parent_claim_id: None,
                principal_id: registered.principal.principal_id.clone(),
                capability_id: registered.capability.capability_id.clone(),
                capability_revision: registered.capability.revision,
                mode: LeaseClaimMode::Strict,
                expected_claim_revision: 0,
                idempotency_key: "acquire:last30days:strict-recovery-protocol".to_string(),
                now: "2026-08-31T12:00:00Z".to_string(),
                expires_at: "2026-08-31T12:05:00Z".to_string(),
                transition_deadline: Some("2026-08-31T12:01:00Z".to_string()),
                recovery_controller_id: Some(registered.capability.capability_id.clone()),
                boot_epoch: Some("boot-1".to_string()),
                owner_generation: Some(57),
            })
            .unwrap()
            .claim
            .unwrap();
        let mut kernel = test_kernel(authority, principals);
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x4d; 32]);
        let request = RecoverLeasePlanPayload {
            raw_controller_capability: LeaseAuthoritySecret(raw_controller.as_bytes().to_vec()),
            resource: claim.resource.clone(),
            claim_id: claim.claim_id.clone(),
            claim_revision: claim.revision,
            fencing_token: claim.fencing_token,
            idempotency_key: "recover:last30days:protected-plan".to_string(),
            owner_generation: Some(58),
        };
        let plan = kernel
            .plan_recovery(request, "2026-08-31T12:00:30Z", &signing_key)
            .unwrap();
        let encoded_plan = serde_json::to_string(&plan).unwrap();
        assert!(!encoded_plan.contains("proof"));
        assert!(!encoded_plan.contains(raw_controller));

        let encoded_state = kernel.encode_protected_state().unwrap();
        let mut restarted =
            LeaseAuthorityProtocolKernel::from_protected_state(&encoded_state, test_load_context())
                .unwrap();
        assert_eq!(
            restarted
                .state
                .authority
                .recovery_authorization_by_plan_id(&plan.plan_id)
                .unwrap()
                .claim_id(),
            claim.claim_id
        );
        let before_wrong_controller = restarted.state.authority.clone();
        let wrong_controller = restarted
            .apply_recovery(
                RecoverLeaseApplyPayload {
                    raw_controller_capability: LeaseAuthoritySecret(
                        b"abpc_v1_wrong-controller-capability-which-is-not-registered".to_vec(),
                    ),
                    plan_id: plan.plan_id.clone(),
                },
                "2026-08-31T12:00:45Z",
                &signing_key,
            )
            .unwrap_err();
        assert_eq!(
            wrong_controller.code(),
            "lease_authority_protocol_recovery_controller_invalid"
        );
        assert_eq!(restarted.state.authority, before_wrong_controller);
        let recovered = restarted
            .apply_recovery(
                RecoverLeaseApplyPayload {
                    raw_controller_capability: LeaseAuthoritySecret(
                        raw_controller.as_bytes().to_vec(),
                    ),
                    plan_id: plan.plan_id.clone(),
                },
                "2026-08-31T12:01:00Z",
                &signing_key,
            )
            .unwrap();
        assert!(!recovered.replayed);
        assert_eq!(recovered.claim.as_ref().unwrap().owner_generation, Some(58));

        restarted
            .state
            .principals
            .profile_capabilities
            .get_mut(&registered.capability.capability_id)
            .unwrap()
            .state = crate::native::service_principal::ServiceProfileCapabilityState::Revoked;
        let encoded_recovered = restarted.encode_protected_state().unwrap();
        let mut replay_kernel = LeaseAuthorityProtocolKernel::from_protected_state(
            &encoded_recovered,
            test_load_context(),
        )
        .unwrap();
        let replay = replay_kernel
            .apply_recovery(
                RecoverLeaseApplyPayload {
                    raw_controller_capability: LeaseAuthoritySecret(
                        raw_controller.as_bytes().to_vec(),
                    ),
                    plan_id: plan.plan_id,
                },
                "2026-08-31T12:01:15Z",
                &signing_key,
            )
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.receipt, recovered.receipt);
    }

    #[test]
    fn recovery_protocol_rejects_caller_owned_time() {
        let error = match decode_lease_authority_request(
            br#"{
                "schemaVersion":"agent-browser.lease-authority-request.v1",
                "operation":"recover_plan",
                "payload":{
                    "rawControllerCapability":[97,98,112,99],
                    "resource":{"kind":"profile","id":"last30days-social"},
                    "claimId":"claim-1",
                    "claimRevision":1,
                    "fencingToken":1,
                    "idempotencyKey":"recover:last30days:caller-time",
                    "ownerGeneration":58,
                    "now":"2099-01-01T00:00:00Z"
                }
            }"#,
        ) {
            Ok(_) => panic!("caller-owned recovery time must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_request_invalid");
    }

    #[test]
    fn authenticated_acquire_derives_holder_identity_inside_the_kernel() {
        let raw_capability = "last30days-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        let registered = crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:last30days".to_string(),
                display_name: Some("Last30days".to_string()),
                profile_id: "last30days-social".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let encoded = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-request.v1",
            "operation": "acquire",
            "payload": {
                "rawCapability": raw_capability.as_bytes(),
                "resource": {"kind": "profile", "id": "last30days-social"},
                "parentClaimId": null,
                "mode": "ephemeral",
                "expectedClaimRevision": 0,
                "idempotencyKey": "acquire:last30days:tick-1",
                "recoveryControllerId": null
            }
        }))
        .unwrap();
        let request = match decode_lease_authority_request(&encoded) {
            Ok(request) => request,
            Err(error) => panic!("typed acquire request must decode: {}", error.code()),
        };
        let mut kernel = test_kernel(super::super::LeaseAuthorityState::default(), principals);
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();

        let response = match kernel.execute(request, "2026-08-31T12:00:00Z") {
            Ok(response) => response,
            Err(error) => panic!("authenticated acquire must succeed: {}", error.code()),
        };
        let LeaseAuthorityProtocolResponse::Acquired(outcome) = response;
        let claim = outcome.claim.expect("fresh acquisition returns a claim");
        assert_eq!(claim.principal_id(), registered.principal.principal_id);
        assert_eq!(claim.capability_id(), registered.capability.capability_id);
        assert_eq!(claim.acquired_at, "2026-08-31T12:00:00Z");
        assert_eq!(claim.expires_at, "2026-08-31T12:05:00.000000000Z");
        assert_eq!(claim.boot_epoch.as_deref(), Some("boot-1"));
        assert_eq!(claim.owner_generation, None);

        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let encoded_response = dispatch_lease_authority_request(
            &mut kernel,
            &encoded,
            &custody,
            test_peer(1000),
            &LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]),
            None,
        )
        .unwrap();
        let response: Value = serde_json::from_slice(&encoded_response).unwrap();
        assert_eq!(response["outcome"], "acquired");
        assert_eq!(response["payload"]["replayed"], true);
        assert_eq!(
            response["payload"]["receipt"]["idempotencyKey"],
            "acquire:last30days:tick-1"
        );
        assert!(!String::from_utf8(encoded_response)
            .unwrap()
            .contains(raw_capability));
    }

    #[test]
    fn protected_state_round_trip_preserves_replay_without_persisting_the_bearer() {
        let raw_capability = "last30days-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:last30days".to_string(),
                display_name: Some("Last30days".to_string()),
                profile_id: "last30days-social".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let encode_request = || {
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": "agent-browser.lease-authority-request.v1",
                "operation": "acquire",
                "payload": {
                    "rawCapability": raw_capability.as_bytes(),
                    "resource": {"kind": "profile", "id": "last30days-social"},
                    "parentClaimId": null,
                    "mode": "ephemeral",
                    "expectedClaimRevision": 0,
                    "idempotencyKey": "acquire:last30days:tick-1",
                    "recoveryControllerId": null
                }
            }))
            .unwrap()
        };
        let request = decode_lease_authority_request(&encode_request())
            .unwrap_or_else(|error| panic!("typed acquire request must decode: {}", error.code()));
        let mut kernel = test_kernel(super::super::LeaseAuthorityState::default(), principals);
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        let first = kernel
            .execute(request, "2026-08-31T12:00:00Z")
            .unwrap_or_else(|error| panic!("fresh acquire must succeed: {}", error.code()));
        let LeaseAuthorityProtocolResponse::Acquired(first) = first;
        assert!(!first.replayed);

        let protected = kernel.encode_protected_state().unwrap();
        assert!(!String::from_utf8_lossy(&protected).contains(raw_capability));
        let protected_json: serde_json::Value = serde_json::from_slice(&protected).unwrap();
        assert!(
            protected_json.pointer("/authority/events").is_none(),
            "lease-event history must not share the protected authority load path"
        );
        let history: serde_json::Value =
            serde_json::from_slice(&kernel.encode_history_state().unwrap()).unwrap();
        assert_eq!(
            history["schemaVersion"],
            "agent-browser.lease-authority-history.v1"
        );
        assert_eq!(history["events"].as_array().map(Vec::len), Some(1));
        let mut restarted =
            LeaseAuthorityProtocolKernel::from_protected_state(&protected, test_load_context())
                .unwrap();
        let replay_request = decode_lease_authority_request(&encode_request())
            .unwrap_or_else(|error| panic!("replay acquire request must decode: {}", error.code()));
        let replay = restarted
            .execute(replay_request, "2026-08-31T12:00:30Z")
            .unwrap_or_else(|error| panic!("replay must succeed: {}", error.code()));
        let LeaseAuthorityProtocolResponse::Acquired(replay) = replay;
        assert!(replay.replayed);
        assert_eq!(replay.claim, first.claim);
    }

    #[test]
    fn protected_state_persists_exact_administrative_intent_but_projection_does_not() {
        let administrator_capability = b"root-administrator-capability-material-v1";
        let mut authority = super::super::LeaseAuthorityState::default();
        authority
            .bootstrap_administrator("administrator:local-root", administrator_capability)
            .unwrap();
        let claim = authority
            .acquire(super::super::AcquireLeaseClaimRequest {
                resource: super::super::LeaseResourceKey::profile("last30days-social"),
                parent_claim_id: None,
                principal_id: "principal:last30days".to_string(),
                capability_id: "capability:last30days-social".to_string(),
                capability_revision: 1,
                mode: super::super::LeaseClaimMode::Strict,
                expected_claim_revision: 0,
                idempotency_key: "acquire:last30days:strict-1".to_string(),
                now: "2026-08-31T12:00:00Z".to_string(),
                expires_at: "2026-08-31T12:05:00Z".to_string(),
                transition_deadline: Some("2026-08-31T12:01:00Z".to_string()),
                recovery_controller_id: Some("capability:last30days-social".to_string()),
                boot_epoch: Some("boot-1".to_string()),
                owner_generation: Some(57),
            })
            .unwrap();
        let signing_key = LeaseAuthoritySigningKey::from_private_bytes([0x5a; 32]);
        let planned = authority
            .plan_administrative_revocation(
                &claim,
                &super::super::LeaseAdministrativeIntent {
                    administrator_id: "administrator:local-root".to_string(),
                    administrator_revision: 1,
                    idempotency_key: "revoke:last30days:strict-1".to_string(),
                    reason_code: "abandoned_strict_holder".to_string(),
                    issued_at: "2026-08-31T12:00:30Z".to_string(),
                    authorization_expires_at: "2026-08-31T12:01:30Z".to_string(),
                },
                administrator_capability,
                &signing_key,
            )
            .unwrap();
        let projection = serde_json::to_string(&authority).unwrap();
        assert!(!projection.contains("administrativeAuthorizations"));
        assert!(!projection.contains(&planned.authorization.proof));

        let kernel = test_kernel(
            authority,
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let protected = kernel.encode_protected_state().unwrap();
        let protected_text = String::from_utf8(protected.clone()).unwrap();
        assert!(protected_text.contains("administrativeAuthorizations"));
        assert!(protected_text.contains(&planned.authorization.proof));
        assert!(!protected_text.contains(std::str::from_utf8(administrator_capability).unwrap()));
        let history: serde_json::Value =
            serde_json::from_slice(&kernel.encode_history_state().unwrap()).unwrap();
        assert_eq!(history["events"][1]["kind"], "revocation_planned");

        let restarted =
            LeaseAuthorityProtocolKernel::from_protected_state(&protected, test_load_context())
                .unwrap();
        assert_eq!(
            restarted
                .state
                .authority
                .administrative_authorizations
                .get("revoke:last30days:strict-1"),
            Some(&planned.authorization)
        );

        let mut tampered: serde_json::Value = serde_json::from_slice(&protected).unwrap();
        tampered["authority"]["administrativeAuthorizations"]["revoke:last30days:strict-1"]
            ["claimRevision"] = serde_json::Value::from(0);
        let error = match LeaseAuthorityProtocolKernel::from_protected_state(
            &serde_json::to_vec(&tampered).unwrap(),
            test_load_context(),
        ) {
            Ok(_) => panic!("a malformed retained administrative intent must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }

    #[test]
    fn authenticated_acquire_cannot_invent_an_unregistered_profile_resource() {
        let raw_capability = "unregistered-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:unregistered".to_string(),
                display_name: None,
                profile_id: "unregistered-profile".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let encoded = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-request.v1",
            "operation": "acquire",
            "payload": {
                "rawCapability": raw_capability.as_bytes(),
                "resource": {"kind": "profile", "id": "unregistered-profile"},
                "parentClaimId": null,
                "mode": "ephemeral",
                "expectedClaimRevision": 0,
                "idempotencyKey": "acquire:unregistered:tick-1",
                "recoveryControllerId": null
            }
        }))
        .unwrap();
        let request = decode_lease_authority_request(&encoded)
            .unwrap_or_else(|error| panic!("request must decode: {}", error.code()));
        let mut kernel = test_kernel(super::super::LeaseAuthorityState::default(), principals);

        let error = match kernel.execute(request, "2026-08-31T12:00:00Z") {
            Ok(_) => panic!("unregistered profile must not become authority"),
            Err(error) => error,
        };
        assert_eq!(
            error.code(),
            "lease_authority_protocol_resource_unregistered"
        );
    }

    #[test]
    fn protected_state_rejects_two_profile_ids_for_one_physical_identity() {
        let mut kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        let mut protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();
        let first = protected
            .pointer("/resources/registrations/profile:last30days-social")
            .cloned()
            .expect("registered profile is present");
        let mut alias = first;
        alias["resource"]["id"] = serde_json::Value::String("last30days-alias".to_string());
        alias["revision"] = serde_json::Value::from(2);
        protected["resources"]["revision"] = serde_json::Value::from(2);
        protected["resources"]["registrations"]["profile:last30days-alias"] = alias;
        let encoded = serde_json::to_vec(&protected).unwrap();

        let error =
            match LeaseAuthorityProtocolKernel::from_protected_state(&encoded, test_load_context())
            {
                Ok(_) => panic!("one physical profile must not load as two resources"),
                Err(error) => error,
            };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }

    #[test]
    fn protected_state_rejects_noncanonical_physical_identity_digest() {
        let mut kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap();
        let mut protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();
        protected["resources"]["registrations"]["profile:last30days-social"]
            ["physicalIdentityDigest"] = serde_json::Value::String(
            "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        );
        let protected = serde_json::to_vec(&protected).unwrap();

        let error = match LeaseAuthorityProtocolKernel::from_protected_state(
            &protected,
            test_load_context(),
        ) {
            Ok(_) => panic!("physical identity digests must have one canonical spelling"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }

    #[test]
    fn protected_state_cannot_prove_its_own_epoch_after_rollback() {
        let kernel = LeaseAuthorityProtocolKernel::bootstrap(
            "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            7,
            "boot-1",
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        )
        .unwrap();
        let protected = kernel.encode_protected_state().unwrap();
        let load_context = LeaseAuthorityProtectedLoadContext {
            expected_authority_domain_id:
                "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            minimum_authority_epoch: 8,
        };

        let error =
            match LeaseAuthorityProtocolKernel::from_protected_state(&protected, load_context) {
                Ok(_) => panic!("a restored authority cannot lower the external epoch floor"),
                Err(error) => error,
            };
        assert_eq!(error.code(), "lease_authority_protocol_epoch_rollback");
    }

    #[test]
    fn protected_state_rejects_owner_for_unregistered_physical_resource() {
        let kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let mut protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();
        let profile_digest =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        protected["owners"]["revision"] = serde_json::Value::from(1);
        protected["owners"]["bindings"]["profile:unregistered-profile"] = serde_json::json!({
            "resource": {"kind": "profile", "id": "unregistered-profile"},
            "physicalIdentityDigest": profile_digest,
            "ownerId": "owner-unregistered",
            "ownerGeneration": 1,
            "logicalBrowserId": "browser-unregistered",
            "daemonSessionRoute": "session-unregistered",
            "processInstanceDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
            "principalId": "principal:unregistered",
            "capabilityId": "profile-capability-v1:unregistered",
            "revision": 1
        });
        let protected = serde_json::to_vec(&protected).unwrap();

        let error = match LeaseAuthorityProtocolKernel::from_protected_state(
            &protected,
            test_load_context(),
        ) {
            Ok(_) => panic!("an owner cannot invent an unregistered physical resource"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }

    #[test]
    fn protected_state_rejects_owner_binding_without_registered_capability() {
        let raw_capability = "last30days-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:last30days".to_string(),
                display_name: Some("Last30days".to_string()),
                profile_id: "last30days-social".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let mut kernel = test_kernel(super::super::LeaseAuthorityState::default(), principals);
        let profile_digest =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        kernel
            .bootstrap_profile_resource("last30days-social", profile_digest)
            .unwrap();
        let mut protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();
        protected["owners"]["revision"] = serde_json::Value::from(1);
        protected["owners"]["bindings"]["profile:last30days-social"] = serde_json::json!({
            "resource": {"kind": "profile", "id": "last30days-social"},
            "physicalIdentityDigest": profile_digest,
            "ownerId": "owner-last30days",
            "ownerGeneration": 1,
            "logicalBrowserId": "browser-last30days",
            "daemonSessionRoute": "session-last30days",
            "processInstanceDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
            "principalId": "principal:last30days",
            "capabilityId": "profile-capability-v1:invented",
            "revision": 1
        });
        let protected = serde_json::to_vec(&protected).unwrap();

        let error = match LeaseAuthorityProtocolKernel::from_protected_state(
            &protected,
            test_load_context(),
        ) {
            Ok(_) => panic!("an owner binding cannot invent capability authority"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }

    #[test]
    fn protected_owner_registry_cannot_serialize_runtime_lifecycle_history() {
        let kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();

        assert_eq!(
            protected["owners"],
            serde_json::json!({
                "schemaVersion": "agent-browser.lease-authority-owner-registry.v1",
                "revision": 0,
                "bindings": {}
            }),
            "protected authority must not ingest lifecycle or terminal owner history"
        );
    }

    #[test]
    fn publication_crash_before_selector_keeps_prior_generation_selected() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let first = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        store.publish(&first, None).unwrap();

        for fault in [
            LeaseAuthorityPublicationFault::ProtectedStateWritten,
            LeaseAuthorityPublicationFault::HistoryWritten,
            LeaseAuthorityPublicationFault::ManifestsWritten,
            LeaseAuthorityPublicationFault::GenerationPublished,
        ] {
            let mut interrupted = store.load_for_mutation(test_load_context()).unwrap();
            interrupted
                .bootstrap_profile_resource(
                    "last30days-social",
                    "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                )
                .unwrap();
            let error = store.publish(&interrupted, Some(fault)).unwrap_err();
            assert_eq!(
                error.code(),
                "lease_authority_protocol_publication_fault_injected"
            );

            let loaded = store.load(test_load_context()).unwrap();
            assert!(loaded.state.resources.registrations.is_empty());
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn stale_publisher_cannot_reselect_an_older_valid_generation() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let first = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        store.publish(&first, None).unwrap();
        let stale = store.load_for_mutation(test_load_context()).unwrap();

        let mut current = store.load_for_mutation(test_load_context()).unwrap();
        current
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        store.publish(&current, None).unwrap();

        let error = store.publish(&stale, None).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_store_stale_publication"
        );
        let selected = store.load(test_load_context()).unwrap();
        assert!(selected
            .state
            .resources
            .registrations
            .contains_key("profile:last30days-social"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn only_mutation_load_can_publish_without_rewriting_prior_history() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let mut first = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        first.state.authority.events.push(LeaseAuthorityEvent {
            event_id: "event:historical".to_string(),
            resource: LeaseResourceKey::profile("last30days-social"),
            claim_id: "claim:historical".to_string(),
            principal_id: "principal:last30days".to_string(),
            fencing_token: 1,
            kind: super::super::LeaseEventKind::Released,
            occurred_at: "2026-08-31T12:00:00Z".to_string(),
        });
        store.publish(&first, None).unwrap();

        let read_only = store.load(test_load_context()).unwrap();
        let error = store.publish(&read_only, None).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_store_mutation_load_required"
        );

        let mut mutation = store.load_for_mutation(test_load_context()).unwrap();
        mutation
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        let current_event = LeaseAuthorityEvent {
            event_id: "event:current".to_string(),
            resource: LeaseResourceKey::profile("last30days-social"),
            claim_id: "claim:current".to_string(),
            principal_id: "principal:last30days".to_string(),
            fencing_token: 2,
            kind: super::super::LeaseEventKind::Acquired,
            occurred_at: "2026-08-31T12:01:00Z".to_string(),
        };
        mutation.state.authority.events.push(current_event.clone());
        store.publish(&mutation, None).unwrap();
        let mut expected_history = first.state.authority.events;
        expected_history.push(current_event);
        assert_eq!(store.load_history().unwrap(), expected_history);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn segmented_history_keeps_legacy_full_snapshot_generation_readable() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let historical_event = LeaseAuthorityEvent {
            event_id: "event:legacy".to_string(),
            resource: LeaseResourceKey::profile("last30days-social"),
            claim_id: "claim:legacy".to_string(),
            principal_id: "principal:last30days".to_string(),
            fencing_token: 1,
            kind: super::super::LeaseEventKind::Released,
            occurred_at: "2026-08-31T12:00:00Z".to_string(),
        };
        let mut legacy = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        legacy.state.authority.events.push(historical_event.clone());
        store.publish(&legacy, None).unwrap();

        let selector: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE)).unwrap(),
        )
        .unwrap();
        let generation_id = selector["generationId"].as_str().unwrap();
        let history_manifest_path = root
            .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
            .join(generation_id)
            .join(LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_FILE);
        let mut history_manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&history_manifest_path).unwrap()).unwrap();
        history_manifest["schemaVersion"] = serde_json::Value::String(
            LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_SCHEMA_VERSION_V1.into(),
        );
        history_manifest
            .as_object_mut()
            .unwrap()
            .remove("previousGenerationId");
        std::fs::write(
            &history_manifest_path,
            serde_json::to_vec_pretty(&history_manifest).unwrap(),
        )
        .unwrap();

        let mut mutation = store.load_for_mutation(test_load_context()).unwrap();
        mutation
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        store.publish(&mutation, None).unwrap();
        assert_eq!(store.load_history().unwrap(), vec![historical_event]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn corrupt_history_degrades_history_without_blocking_current_authority() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        store.publish(&kernel, None).unwrap();
        let selector: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE)).unwrap(),
        )
        .unwrap();
        let generation_id = selector["generationId"].as_str().unwrap();
        let history_path = root
            .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
            .join(generation_id)
            .join(LEASE_AUTHORITY_STORE_HISTORY_FILE);
        std::fs::write(&history_path, b"corrupt-history\n").unwrap();

        store.load(test_load_context()).unwrap();
        let mut mutation = store.load_for_mutation(test_load_context()).unwrap();
        mutation
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        store.publish(&mutation, None).unwrap();
        let current = store.load(test_load_context()).unwrap();
        assert!(current
            .state
            .resources
            .registrations
            .contains_key("profile:last30days-social"));
        let error = store.load_history().unwrap_err();
        assert_eq!(error.code(), "lease_authority_protocol_history_unavailable");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn corrupt_selected_authority_never_falls_back_to_an_older_generation() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let first = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        store.publish(&first, None).unwrap();
        let mut selected = store.load_for_mutation(test_load_context()).unwrap();
        selected
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        store.publish(&selected, None).unwrap();
        let selector: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE)).unwrap(),
        )
        .unwrap();
        let generation_id = selector["generationId"].as_str().unwrap();
        let protected_path = root
            .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
            .join(generation_id)
            .join(LEASE_AUTHORITY_STORE_PROTECTED_STATE_FILE);
        std::fs::write(&protected_path, b"corrupt-protected-state\n").unwrap();

        let error = match store.load(test_load_context()) {
            Ok(_) => panic!("corrupt selected authority must not fall back to an older generation"),
            Err(error) => error,
        };
        assert_eq!(
            error.code(),
            "lease_authority_protocol_protected_state_unavailable"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn corrupt_history_manifest_degrades_history_only() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-store-{}",
            uuid::Uuid::new_v4()
        ));
        let store = LeaseAuthorityDurableStore::initialize(&root).unwrap();
        let kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        store.publish(&kernel, None).unwrap();
        let selector: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(LEASE_AUTHORITY_STORE_SELECTOR_FILE)).unwrap(),
        )
        .unwrap();
        let generation_id = selector["generationId"].as_str().unwrap();
        let history_manifest_path = root
            .join(LEASE_AUTHORITY_STORE_GENERATIONS_DIRECTORY)
            .join(generation_id)
            .join(LEASE_AUTHORITY_STORE_HISTORY_MANIFEST_FILE);
        std::fs::write(&history_manifest_path, b"corrupt-history-manifest\n").unwrap();

        store.load(test_load_context()).unwrap();
        let error = store.load_history().unwrap_err();
        assert_eq!(error.code(), "lease_authority_protocol_history_unavailable");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn service_identity_challenge_binds_nonce_domain_epoch_and_custody() {
        let kernel = test_kernel(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        let signing_key = super::super::LeaseAuthoritySigningKey::from_private_bytes([7u8; 32]);
        let verification_keys =
            super::super::LeaseAuthorityVerificationKeyring::from_active(&signing_key);
        let custody = custody::LeaseAuthorityCustodySnapshot::root_owned_fixture()
            .validate(991)
            .unwrap();
        let request = LeaseAuthorityServiceChallengeRequest {
            nonce: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            expected_authority_domain_id: TEST_AUTHORITY_DOMAIN_ID.to_string(),
            minimum_authority_epoch: 7,
        };

        let proof = kernel
            .issue_service_identity_challenge(&request, &custody, &signing_key)
            .unwrap();
        verify_service_identity_challenge(&proof, &request, &custody, &verification_keys).unwrap();

        let replacement_snapshot =
            custody::LeaseAuthorityCustodySnapshot::root_owned_fixture().with_replaced_socket();
        let replacement_custody = replacement_snapshot.validate(991).unwrap();
        let error = verify_service_identity_challenge(
            &proof,
            &request,
            &replacement_custody,
            &verification_keys,
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_service_identity_proof_invalid"
        );

        let mut tampered = proof;
        tampered.endpoint_identity_digest =
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
        let error =
            verify_service_identity_challenge(&tampered, &request, &custody, &verification_keys)
                .unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_service_identity_proof_invalid"
        );
    }
}
