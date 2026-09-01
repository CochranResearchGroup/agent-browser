use ring::signature::{self, Ed25519KeyPair};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::{
    AcquireLeaseClaimRequest, ActiveLeaseClaim, LeaseAdministratorAuthority, LeaseAuthorityEvent,
    LeaseAuthoritySigningKey, LeaseAuthorityState, LeaseAuthorityVerificationKeyring,
    LeaseClaimAcquisitionOutcome, LeaseClaimAcquisitionReceipt, LeaseClaimMode,
    LeaseClaimRecoveryReceipt, LeaseClaimTerminalReceipt, LeaseResourceKey, LeaseResourceKind,
};
use crate::native::service_principal::{authenticate_profile_capability, ServicePrincipalRegistry};

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
    expected_authority_revision: u64,
    idempotency_key: String,
    now: String,
    expires_at: String,
    transition_deadline: Option<String>,
    recovery_controller_id: Option<String>,
    boot_epoch: Option<String>,
    owner_generation: Option<u64>,
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
    Acquire(Box<AcquireLeaseAuthorityPayload>),
    AuthorizeEffect(Value),
    Release(Value),
    Recover(Value),
    Revoke(Value),
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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityResourceRegistration {
    resource: LeaseResourceKey,
    physical_identity_digest: String,
    revision: u64,
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
            },
        );
        self.state.resources.revision = revision;
        Ok(())
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
    ) -> Result<LeaseAuthorityProtocolResponse, LeaseAuthorityProtocolError> {
        match request {
            LeaseAuthorityProtocolRequest::Acquire(request) => self.acquire(*request),
            LeaseAuthorityProtocolRequest::ServiceChallenge(_)
            | LeaseAuthorityProtocolRequest::AuthorizeEffect(_)
            | LeaseAuthorityProtocolRequest::Release(_)
            | LeaseAuthorityProtocolRequest::Recover(_)
            | LeaseAuthorityProtocolRequest::Revoke(_)
            | LeaseAuthorityProtocolRequest::Inspect(_) => Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_operation_not_implemented",
            }),
        }
    }

    fn acquire(
        &mut self,
        request: AcquireLeaseAuthorityPayload,
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
                expected_authority_revision: request.expected_authority_revision,
                idempotency_key: request.idempotency_key,
                now: request.now,
                expires_at: request.expires_at,
                transition_deadline: request.transition_deadline,
                recovery_controller_id: request.recovery_controller_id,
                boot_epoch: request.boot_epoch,
                owner_generation: request.owner_generation,
            })
            .map_err(|error| LeaseAuthorityProtocolError {
                code: error.as_str(),
            })?;
        Ok(LeaseAuthorityProtocolResponse::Acquired(outcome))
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
        || state.resources.schema_version != LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION
    {
        return Err(invalid());
    }

    let mut physical_identities = BTreeSet::new();
    let mut highest_revision = 0;
    for (storage_key, registration) in &state.resources.registrations {
        if registration.resource.kind != LeaseResourceKind::Profile
            || storage_key != &registration.resource.storage_key()
            || !valid_sha256_digest(&registration.physical_identity_digest)
            || registration.revision == 0
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
        "acquire" => serde_json::from_value(envelope.payload)
            .map(Box::new)
            .map(LeaseAuthorityProtocolRequest::Acquire)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "authorize_effect" => Ok(LeaseAuthorityProtocolRequest::AuthorizeEffect(
            envelope.payload,
        )),
        "release" => Ok(LeaseAuthorityProtocolRequest::Release(envelope.payload)),
        "recover" => Ok(LeaseAuthorityProtocolRequest::Recover(envelope.payload)),
        "revoke" => Ok(LeaseAuthorityProtocolRequest::Revoke(envelope.payload)),
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

fn dispatch_lease_authority_request(
    kernel: &mut LeaseAuthorityProtocolKernel,
    encoded: &[u8],
    custody: &custody::LeaseAuthorityCustodyIdentity,
    signing_key: &LeaseAuthoritySigningKey,
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
        LeaseAuthorityProtocolRequest::Acquire(_)
        | LeaseAuthorityProtocolRequest::AuthorizeEffect(_)
        | LeaseAuthorityProtocolRequest::Release(_)
        | LeaseAuthorityProtocolRequest::Recover(_)
        | LeaseAuthorityProtocolRequest::Revoke(_)
        | LeaseAuthorityProtocolRequest::Inspect(_) => Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_operation_not_implemented",
        }),
    }
}

fn serve_lease_authority_connection<R: Read, W: Write>(
    kernel: &mut LeaseAuthorityProtocolKernel,
    reader: &mut R,
    writer: &mut W,
    custody: &custody::LeaseAuthorityCustodyIdentity,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<(), LeaseAuthorityProtocolError> {
    let response = match read_lease_authority_frame(reader).and_then(|request| {
        dispatch_lease_authority_request(kernel, &request, custody, signing_key)
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

        let encoded =
            dispatch_lease_authority_request(&mut kernel, request, &custody, &signing_key).unwrap();
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
                "expectedAuthorityRevision":0,
                "idempotencyKey":"acquire:last30days:tick-1",
                "now":"2026-08-31T12:00:00Z",
                "expiresAt":"2026-08-31T12:05:00Z",
                "transitionDeadline":null,
                "recoveryControllerId":null,
                "bootEpoch":"boot-1",
                "ownerGeneration":null
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
                "expectedAuthorityRevision": 0,
                "idempotencyKey": "acquire:last30days:tick-1",
                "now": "2026-08-31T12:00:00Z",
                "expiresAt": "2026-08-31T12:05:00Z",
                "transitionDeadline": null,
                "recoveryControllerId": null,
                "bootEpoch": "boot-1",
                "ownerGeneration": null
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

        let response = match kernel.execute(request) {
            Ok(response) => response,
            Err(error) => panic!("authenticated acquire must succeed: {}", error.code()),
        };
        let LeaseAuthorityProtocolResponse::Acquired(outcome) = response;
        let claim = outcome.claim.expect("fresh acquisition returns a claim");
        assert_eq!(claim.principal_id(), registered.principal.principal_id);
        assert_eq!(claim.capability_id(), registered.capability.capability_id);
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
                    "expectedAuthorityRevision": 0,
                    "idempotencyKey": "acquire:last30days:tick-1",
                    "now": "2026-08-31T12:00:00Z",
                    "expiresAt": "2026-08-31T12:05:00Z",
                    "transitionDeadline": null,
                    "recoveryControllerId": null,
                    "bootEpoch": "boot-1",
                    "ownerGeneration": null
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
            .execute(request)
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
            .execute(replay_request)
            .unwrap_or_else(|error| panic!("replay must succeed: {}", error.code()));
        let LeaseAuthorityProtocolResponse::Acquired(replay) = replay;
        assert!(replay.replayed);
        assert_eq!(replay.claim, first.claim);
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
                "expectedAuthorityRevision": 0,
                "idempotencyKey": "acquire:unregistered:tick-1",
                "now": "2026-08-31T12:00:00Z",
                "expiresAt": "2026-08-31T12:05:00Z",
                "transitionDeadline": null,
                "recoveryControllerId": null,
                "bootEpoch": "boot-1",
                "ownerGeneration": null
            }
        }))
        .unwrap();
        let request = decode_lease_authority_request(&encoded)
            .unwrap_or_else(|error| panic!("request must decode: {}", error.code()));
        let mut kernel = test_kernel(super::super::LeaseAuthorityState::default(), principals);

        let error = match kernel.execute(request) {
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
