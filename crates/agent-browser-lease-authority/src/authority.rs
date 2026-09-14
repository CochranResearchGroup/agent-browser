//! Canonical active-claim authority for service-owned resources.
//!
//! Only `active_claims` may authorize or block effects. `events` is retained
//! append-only history and is never consulted for admission.

mod authenticated;
mod protocol;
#[cfg(test)]
mod test_support;

pub use authenticated::{
    authorize_lease_effect, issue_lease_administrative_authorization,
    issue_lease_effect_authorization, issue_lease_recovery_authorization, recover_lease_claim,
    release_lease_claim, revoke_lease_claim, LeaseAuthorityView,
};

#[cfg(target_os = "linux")]
pub const LEASE_AUTHORITY_SERVICE_PROCESS_ENV: &str = protocol::LEASE_AUTHORITY_SERVICE_PROCESS_ENV;

#[cfg(target_os = "linux")]
pub const LEASE_AUTHORITY_BOOTSTRAP_PROCESS_ENV: &str =
    protocol::LEASE_AUTHORITY_BOOTSTRAP_PROCESS_ENV;

#[cfg(target_os = "linux")]
pub fn run_linux_lease_authority_service() -> Result<(), String> {
    protocol::run_linux_lease_authority_service()
}

#[cfg(target_os = "linux")]
pub fn run_linux_lease_authority_bootstrap() -> Result<(), String> {
    protocol::run_linux_lease_authority_bootstrap()
}

#[cfg(target_os = "linux")]
pub use protocol::client::{
    acquire_protected_ephemeral_profile_claim, authorize_protected_browser_launch,
    complete_protected_browser_adoption_success, complete_protected_browser_launch_success,
    enroll_protected_profile, inspect_protected_profile_authority,
    mark_protected_browser_adoption_uncertain, mark_protected_browser_launch_uncertain,
    prepare_protected_browser_adoption, reconcile_protected_browser_owner,
    ProtectedAuthorityObservationState, ProtectedBrowserAdoptionPreparation,
    ProtectedBrowserAdoptionRequest, ProtectedBrowserLaunchPermit, ProtectedBrowserLaunchRequest,
    ProtectedBrowserOwner, ProtectedBrowserOwnerLease, ProtectedBrowserOwnerReconciliationRequest,
    ProtectedEphemeralProfileClaim, ProtectedEphemeralProfileClaimRequest,
    ProtectedProfileEnrollment, ProtectedProfileEnrollmentRequest,
};

use ring::signature::{self, Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const LEASE_AUTHORITY_SCHEMA_VERSION: &str = "agent-browser.lease-authority.v1";
pub const LEASE_ACQUISITION_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.lease-acquisition-receipt.v1";
pub const LEASE_TERMINAL_RECEIPT_SCHEMA_VERSION: &str = "agent-browser.lease-terminal-receipt.v1";
pub const LEASE_RECOVERY_RECEIPT_SCHEMA_VERSION: &str = "agent-browser.lease-recovery-receipt.v1";
pub const LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION: &str =
    "agent-browser.lease-effect-authorization.v5";
pub const LEASE_RECOVERY_AUTHORIZATION_SCHEMA_VERSION: &str =
    "agent-browser.lease-recovery-authorization.v4";
pub const LEASE_ADMINISTRATIVE_AUTHORIZATION_SCHEMA_VERSION: &str =
    "agent-browser.lease-administrative-authorization.v2";
pub const LEASE_AUTHORITY_SIGNING_KEY_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-signing-key.v3";
pub const LEASE_AUTHORITY_VERIFICATION_KEY_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-verification-keyring.v2";

const MAX_LEASE_CLAIM_TENURE_SECONDS: i64 = 300;
const MAX_STRICT_RECOVERY_TENURE_SECONDS: i64 = 300;
const MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS: i64 = 120;
const MAX_LEASE_AUTHORITY_VERIFICATION_KEYS: usize = 8;
const LEASE_AUTHORITY_SIGNING_KEY_FILE: &str = "lease-authority-signing-key.v3.json";
const LEASE_AUTHORITY_VERIFICATION_KEY_FILE: &str = "lease-authority-verification-keyring.v2.json";
const LEASE_AUTHORITY_TRUST_ROOT_DIRECTORY: &str = "lease-authority-trust";
const LEASE_AUTHORITY_TRUST_GENERATIONS_DIRECTORY: &str = "generations";
const LEASE_AUTHORITY_TRUST_GENERATION_MANIFEST_FILE: &str = "manifest.v1.json";
const LEASE_AUTHORITY_TRUST_SELECTOR_FILE: &str = "selected-generation.v1.json";
const LEASE_AUTHORITY_TRUST_GENERATION_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-trust-generation.v1";
const LEASE_AUTHORITY_TRUST_SELECTOR_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-trust-selector.v1";

/// Private signing authority held outside Service State. The key identifier is
/// safe to persist in plans and receipts; the secret is never serialized by
/// this type or included in debug output.
#[derive(Clone, PartialEq, Eq)]
struct LeaseAuthoritySigningKey {
    key_epoch: u64,
    key_id: String,
    private_key: [u8; 32],
    public_key: [u8; 32],
}

impl std::fmt::Debug for LeaseAuthoritySigningKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseAuthoritySigningKey")
            .field("key_id", &self.key_id)
            .field("private_key", &"[REDACTED]")
            .finish()
    }
}

impl LeaseAuthoritySigningKey {
    fn from_private_bytes(private_key: [u8; 32]) -> Self {
        Self::from_private_bytes_at_epoch(private_key, 1)
    }

    fn from_private_bytes_at_epoch(private_key: [u8; 32], key_epoch: u64) -> Self {
        assert!(key_epoch > 0, "lease signing-key epoch must be positive");
        let key = Ed25519KeyPair::from_seed_unchecked(&private_key)
            .expect("32-byte Ed25519 private seed is valid");
        let public_key: [u8; 32] = key
            .public_key()
            .as_ref()
            .try_into()
            .expect("Ed25519 public keys are 32 bytes");
        Self {
            key_epoch,
            key_id: stable_id(
                "lease-authority-ed25519-verification-key-v1",
                &hex::encode(public_key),
            ),
            private_key,
            public_key,
        }
    }

    fn verification_key(&self) -> LeaseAuthorityVerificationKey {
        LeaseAuthorityVerificationKey {
            key_epoch: self.key_epoch,
            key_id: self.key_id.clone(),
            public_key: self.public_key,
        }
    }
}

/// Public verification material. Runtime executors can validate a kernel
/// authorization with this type but cannot use it to mint another one.
#[derive(Clone, PartialEq, Eq)]
struct LeaseAuthorityVerificationKey {
    key_epoch: u64,
    key_id: String,
    public_key: [u8; 32],
}

impl std::fmt::Debug for LeaseAuthorityVerificationKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseAuthorityVerificationKey")
            .field("key_id", &self.key_id)
            .field("public_key", &hex::encode(self.public_key))
            .finish()
    }
}

impl LeaseAuthorityVerificationKey {
    fn from_public_bytes_at_epoch(public_key: [u8; 32], key_epoch: u64) -> Result<Self, String> {
        if key_epoch == 0 {
            return Err("lease_authority_verification_key_epoch_invalid".to_string());
        }
        Ok(Self {
            key_epoch,
            key_id: stable_id(
                "lease-authority-ed25519-verification-key-v1",
                &hex::encode(public_key),
            ),
            public_key,
        })
    }

    fn verify_key_identity(&self, key_id: &str, key_epoch: u64) -> Result<(), LeaseAuthorityError> {
        (self.key_id == key_id && self.key_epoch == key_epoch)
            .then_some(())
            .ok_or(LeaseAuthorityError::SigningKeyMismatch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseAuthorityVerificationKeyring {
    key_epoch: u64,
    active_key_id: String,
    keys: BTreeMap<String, LeaseAuthorityVerificationKey>,
}

impl LeaseAuthorityVerificationKeyring {
    fn from_active(signing_key: &LeaseAuthoritySigningKey) -> Self {
        let verification_key = signing_key.verification_key();
        Self {
            key_epoch: signing_key.key_epoch,
            active_key_id: signing_key.key_id.clone(),
            keys: BTreeMap::from([(verification_key.key_id.clone(), verification_key)]),
        }
    }

    fn verification_key(
        &self,
        key_id: &str,
        key_epoch: u64,
    ) -> Result<&LeaseAuthorityVerificationKey, LeaseAuthorityError> {
        if key_epoch == 0 || key_epoch > self.key_epoch {
            return Err(LeaseAuthorityError::SigningKeyMismatch);
        }
        let key = self
            .keys
            .get(key_id)
            .ok_or(LeaseAuthorityError::SigningKeyMismatch)?;
        key.verify_key_identity(key_id, key_epoch)?;
        Ok(key)
    }

    fn with_rotated_active(&self, signing_key: &LeaseAuthoritySigningKey) -> Result<Self, String> {
        let expected_epoch = self
            .key_epoch
            .checked_add(1)
            .ok_or_else(|| "lease_authority_signing_key_epoch_exhausted".to_string())?;
        if signing_key.key_epoch != expected_epoch {
            return Err("lease_authority_signing_key_epoch_mismatch".to_string());
        }
        if self.keys.len() >= MAX_LEASE_AUTHORITY_VERIFICATION_KEYS {
            return Err("lease_authority_verification_keyring_capacity_exhausted".to_string());
        }
        let mut keys = self.keys.clone();
        let verification_key = signing_key.verification_key();
        keys.insert(verification_key.key_id.clone(), verification_key);
        Ok(Self {
            key_epoch: signing_key.key_epoch,
            active_key_id: signing_key.key_id.clone(),
            keys,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthoritySigningKeyFile {
    schema_version: String,
    key_epoch: u64,
    key_id: String,
    private_key_hex: String,
    public_key_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityVerificationKeyFileEntry {
    key_epoch: u64,
    key_id: String,
    public_key_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityVerificationKeyFile {
    schema_version: String,
    key_epoch: u64,
    active_key_id: String,
    keys: Vec<LeaseAuthorityVerificationKeyFileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityTrustGenerationManifest {
    schema_version: String,
    generation_id: String,
    key_epoch: u64,
    active_key_id: String,
    signing_key_sha256: String,
    verification_keyring_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityTrustSelector {
    schema_version: String,
    generation_id: String,
    key_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseAuthorityTrustGeneration {
    selector: LeaseAuthorityTrustSelector,
    manifest: LeaseAuthorityTrustGenerationManifest,
    path: PathBuf,
}

/// Loads the selected trust generation, creating generation one only when no
/// legacy or partially staged trust material exists. The selector is published
/// after both key documents and their manifest are crash durable, so a reader
/// never pairs files from different generations.
fn load_or_create_lease_authority_signing_key() -> Result<LeaseAuthoritySigningKey, String> {
    if lease_authority_trust_selector_path()?.exists() {
        return load_selected_lease_authority_signing_key();
    }
    let legacy_paths = existing_legacy_authority_key_paths()?;
    if !legacy_paths.is_empty() {
        return Err(format!(
            "lease_authority_trust_migration_required:{}",
            legacy_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if lease_authority_trust_generation_present()? {
        return Err("lease_authority_trust_selection_recovery_required".to_string());
    }

    let mut private_key = [0u8; 32];
    getrandom::getrandom(&mut private_key)
        .map_err(|error| format!("lease_authority_signing_key_generation_failed:{error}"))?;
    let key = LeaseAuthoritySigningKey::from_private_bytes(private_key);
    initialize_lease_authority_trust_generation(&key)
}

/// Loads only public verification material. Effect and recovery executors do
/// not read the private signing root and cannot initialize an authority domain.
fn load_existing_lease_authority_verification_key(
) -> Result<LeaseAuthorityVerificationKeyring, String> {
    load_existing_lease_authority_verification_key_in(&lease_authority_trust_root_path()?)
}

fn load_existing_lease_authority_verification_key_in(
    root: &Path,
) -> Result<LeaseAuthorityVerificationKeyring, String> {
    let generation = load_selected_lease_authority_trust_generation_in(root)?;
    let path = generation.path.join(LEASE_AUTHORITY_VERIFICATION_KEY_FILE);
    verify_file_sha256(
        &path,
        &generation.manifest.verification_keyring_sha256,
        "lease_authority_verification_keyring_digest_mismatch",
    )?;
    let keyring = load_lease_authority_verification_key_file(&path)?;
    validate_selected_keyring(&generation, &keyring)?;
    Ok(keyring)
}

fn lease_authority_service_directory() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".agent-browser").join("service"))
        .ok_or_else(|| "lease_authority_signing_key_home_unavailable".to_string())
}

fn lease_authority_trust_root_path() -> Result<PathBuf, String> {
    Ok(lease_authority_service_directory()?.join(LEASE_AUTHORITY_TRUST_ROOT_DIRECTORY))
}

fn lease_authority_trust_generations_path() -> Result<PathBuf, String> {
    Ok(lease_authority_trust_generations_path_in(
        &lease_authority_trust_root_path()?,
    ))
}

fn lease_authority_trust_selector_path() -> Result<PathBuf, String> {
    Ok(lease_authority_trust_selector_path_in(
        &lease_authority_trust_root_path()?,
    ))
}

fn lease_authority_trust_generations_path_in(root: &Path) -> PathBuf {
    root.join(LEASE_AUTHORITY_TRUST_GENERATIONS_DIRECTORY)
}

fn lease_authority_trust_selector_path_in(root: &Path) -> PathBuf {
    root.join(LEASE_AUTHORITY_TRUST_SELECTOR_FILE)
}

fn existing_legacy_authority_key_paths() -> Result<Vec<PathBuf>, String> {
    Ok(existing_legacy_authority_key_paths_in(
        &lease_authority_service_directory()?,
    ))
}

fn existing_legacy_authority_key_paths_in(service: &Path) -> Vec<PathBuf> {
    [
        LEASE_AUTHORITY_SIGNING_KEY_FILE,
        LEASE_AUTHORITY_VERIFICATION_KEY_FILE,
        "lease-authority-signing-key.v2.json",
        "lease-authority-verification-key.v1.json",
    ]
    .into_iter()
    .map(|name| service.join(name))
    .filter(|path| path.exists())
    .collect()
}

fn lease_authority_trust_generation_present() -> Result<bool, String> {
    lease_authority_trust_generation_present_in(&lease_authority_trust_root_path()?)
}

fn lease_authority_trust_generation_present_in(root: &Path) -> Result<bool, String> {
    let generations = lease_authority_trust_generations_path_in(root);
    if !generations.exists() {
        return Ok(false);
    }
    fs::read_dir(&generations)
        .map_err(|error| {
            format!(
                "lease_authority_trust_generations_read_failed:{}:{error}",
                generations.display()
            )
        })?
        .next()
        .transpose()
        .map(|entry| entry.is_some())
        .map_err(|error| {
            format!(
                "lease_authority_trust_generations_read_failed:{}:{error}",
                generations.display()
            )
        })
}

fn lease_authority_trust_generation_id(key_epoch: u64, key_id: &str) -> Result<String, String> {
    let digest = key_id
        .strip_prefix("lease-authority-ed25519-verification-key-v1:")
        .filter(|digest| digest.len() == 32 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| "lease_authority_trust_generation_key_id_invalid".to_string())?;
    if key_epoch == 0 {
        return Err("lease_authority_trust_generation_epoch_invalid".to_string());
    }
    Ok(format!("epoch-{key_epoch}-{digest}"))
}

fn lease_authority_trust_generation_component_is_safe(generation_id: &str) -> bool {
    !generation_id.is_empty()
        && generation_id.len() <= 96
        && generation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn load_selected_lease_authority_signing_key() -> Result<LeaseAuthoritySigningKey, String> {
    load_selected_lease_authority_signing_key_in(&lease_authority_trust_root_path()?)
}

fn load_selected_lease_authority_signing_key_in(
    root: &Path,
) -> Result<LeaseAuthoritySigningKey, String> {
    let generation = load_selected_lease_authority_trust_generation_in(root)?;
    let signing_path = generation.path.join(LEASE_AUTHORITY_SIGNING_KEY_FILE);
    let verification_path = generation.path.join(LEASE_AUTHORITY_VERIFICATION_KEY_FILE);
    if !signing_path.exists() && verification_path.exists() {
        return Err(format!(
            "lease_authority_signing_key_recovery_required:{}:{}",
            signing_path.display(),
            verification_path.display()
        ));
    }
    verify_file_sha256(
        &signing_path,
        &generation.manifest.signing_key_sha256,
        "lease_authority_signing_key_digest_mismatch",
    )?;
    let signing_key = load_lease_authority_signing_key_file(&signing_path)?;
    verify_file_sha256(
        &verification_path,
        &generation.manifest.verification_keyring_sha256,
        "lease_authority_verification_keyring_digest_mismatch",
    )?;
    let keyring = load_lease_authority_verification_key_file(&verification_path)?;
    validate_selected_keyring(&generation, &keyring)?;
    if signing_key.key_epoch != generation.selector.key_epoch
        || signing_key.key_id != generation.manifest.active_key_id
        || keyring.active_key_id != signing_key.key_id
    {
        return Err("lease_authority_trust_generation_key_mismatch".to_string());
    }
    Ok(signing_key)
}

fn load_selected_lease_authority_trust_generation_in(
    root: &Path,
) -> Result<LeaseAuthorityTrustGeneration, String> {
    let selector_path = lease_authority_trust_selector_path_in(root);
    if !selector_path.exists() {
        return Err(format!(
            "lease_authority_trust_selector_unavailable:{}",
            selector_path.display()
        ));
    }
    let selector: LeaseAuthorityTrustSelector = load_private_json_file(
        &selector_path,
        "lease_authority_trust_selector_decode_failed",
    )?;
    if selector.schema_version != LEASE_AUTHORITY_TRUST_SELECTOR_SCHEMA_VERSION
        || selector.key_epoch == 0
        || !lease_authority_trust_generation_component_is_safe(&selector.generation_id)
    {
        return Err("lease_authority_trust_selector_invalid".to_string());
    }
    let generation_path =
        lease_authority_trust_generations_path_in(root).join(&selector.generation_id);
    ensure_private_directory(&generation_path)?;
    let manifest_path = generation_path.join(LEASE_AUTHORITY_TRUST_GENERATION_MANIFEST_FILE);
    let manifest: LeaseAuthorityTrustGenerationManifest = load_private_json_file(
        &manifest_path,
        "lease_authority_trust_manifest_decode_failed",
    )?;
    if manifest.schema_version != LEASE_AUTHORITY_TRUST_GENERATION_SCHEMA_VERSION
        || manifest.generation_id != selector.generation_id
        || manifest.key_epoch != selector.key_epoch
        || manifest.active_key_id.trim().is_empty()
        || manifest.signing_key_sha256.len() != 64
        || !manifest
            .signing_key_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || manifest.verification_keyring_sha256.len() != 64
        || !manifest
            .verification_keyring_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("lease_authority_trust_generation_invalid".to_string());
    }
    Ok(LeaseAuthorityTrustGeneration {
        selector,
        manifest,
        path: generation_path,
    })
}

fn validate_selected_keyring(
    generation: &LeaseAuthorityTrustGeneration,
    keyring: &LeaseAuthorityVerificationKeyring,
) -> Result<(), String> {
    let canonical_generation_id =
        lease_authority_trust_generation_id(keyring.key_epoch, &keyring.active_key_id)?;
    (keyring.key_epoch == generation.selector.key_epoch
        && keyring.active_key_id == generation.manifest.active_key_id
        && canonical_generation_id == generation.selector.generation_id)
        .then_some(())
        .ok_or_else(|| "lease_authority_trust_generation_keyring_mismatch".to_string())
}

fn initialize_lease_authority_trust_generation(
    key: &LeaseAuthoritySigningKey,
) -> Result<LeaseAuthoritySigningKey, String> {
    let root = lease_authority_trust_root_path()?;
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "lease_authority_trust_directory_create_failed:{}:{error}",
            root.display()
        )
    })?;
    set_private_directory_permissions(&root)?;
    let lock_path = root.join("selection.lock");
    let lock = open_private_lock_file(&lock_path)?;
    lock.lock().map_err(|error| {
        format!(
            "lease_authority_trust_selection_lock_failed:{}:{error}",
            lock_path.display()
        )
    })?;
    if lease_authority_trust_selector_path()?.exists() {
        drop(lock);
        return load_selected_lease_authority_signing_key();
    }
    if lease_authority_trust_generation_present()? {
        return Err("lease_authority_trust_selection_recovery_required".to_string());
    }
    persist_lease_authority_trust_generation(
        key,
        &LeaseAuthorityVerificationKeyring::from_active(key),
    )?;
    drop(lock);
    load_selected_lease_authority_signing_key()
}

fn persist_lease_authority_trust_generation(
    signing_key: &LeaseAuthoritySigningKey,
    keyring: &LeaseAuthorityVerificationKeyring,
) -> Result<(), String> {
    persist_lease_authority_trust_generation_in(
        &lease_authority_trust_root_path()?,
        signing_key,
        keyring,
    )
}

fn persist_lease_authority_trust_generation_in(
    root: &Path,
    signing_key: &LeaseAuthoritySigningKey,
    keyring: &LeaseAuthorityVerificationKeyring,
) -> Result<(), String> {
    if signing_key.key_epoch != keyring.key_epoch || signing_key.key_id != keyring.active_key_id {
        return Err("lease_authority_trust_generation_key_mismatch".to_string());
    }
    let generations = lease_authority_trust_generations_path_in(root);
    fs::create_dir_all(&generations).map_err(|error| {
        format!(
            "lease_authority_trust_generations_create_failed:{}:{error}",
            generations.display()
        )
    })?;
    set_private_directory_permissions(&generations)?;
    let generation_id =
        lease_authority_trust_generation_id(signing_key.key_epoch, &signing_key.key_id)?;
    let final_path = generations.join(&generation_id);
    if final_path.exists() {
        validate_persisted_lease_authority_trust_generation(
            &final_path,
            &generation_id,
            signing_key,
            keyring,
        )?;
        let selector = LeaseAuthorityTrustSelector {
            schema_version: LEASE_AUTHORITY_TRUST_SELECTOR_SCHEMA_VERSION.to_string(),
            generation_id,
            key_epoch: signing_key.key_epoch,
        };
        return write_private_json_atomic_replace(
            &lease_authority_trust_selector_path_in(root),
            &selector,
        );
    }
    let temporary = generations.join(format!(".{generation_id}.{}.tmp", uuid::Uuid::new_v4()));
    fs::create_dir(&temporary).map_err(|error| {
        format!(
            "lease_authority_trust_generation_stage_failed:{}:{error}",
            temporary.display()
        )
    })?;
    set_private_directory_permissions(&temporary)?;
    let result = (|| {
        let signing_document = LeaseAuthoritySigningKeyFile {
            schema_version: LEASE_AUTHORITY_SIGNING_KEY_SCHEMA_VERSION.to_string(),
            key_epoch: signing_key.key_epoch,
            key_id: signing_key.key_id.clone(),
            private_key_hex: hex::encode(signing_key.private_key),
            public_key_hex: hex::encode(signing_key.public_key),
        };
        let signing_encoded = serde_json::to_vec_pretty(&signing_document)
            .map_err(|error| format!("lease_authority_signing_key_encode_failed:{error}"))?;
        let signing_path = temporary.join(LEASE_AUTHORITY_SIGNING_KEY_FILE);
        write_private_signing_key_file(&signing_path, &signing_encoded)?;

        let verification_document = verification_keyring_document(keyring);
        let verification_encoded = serde_json::to_vec_pretty(&verification_document)
            .map_err(|error| format!("lease_authority_verification_key_encode_failed:{error}"))?;
        let verification_path = temporary.join(LEASE_AUTHORITY_VERIFICATION_KEY_FILE);
        write_private_signing_key_file(&verification_path, &verification_encoded)?;

        let manifest = LeaseAuthorityTrustGenerationManifest {
            schema_version: LEASE_AUTHORITY_TRUST_GENERATION_SCHEMA_VERSION.to_string(),
            generation_id: generation_id.clone(),
            key_epoch: signing_key.key_epoch,
            active_key_id: signing_key.key_id.clone(),
            signing_key_sha256: file_sha256(&signing_path)?,
            verification_keyring_sha256: file_sha256(&verification_path)?,
        };
        let manifest_encoded = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("lease_authority_trust_manifest_encode_failed:{error}"))?;
        write_private_signing_key_file(
            &temporary.join(LEASE_AUTHORITY_TRUST_GENERATION_MANIFEST_FILE),
            &manifest_encoded,
        )?;
        sync_authority_key_directory(&temporary)?;
        fs::rename(&temporary, &final_path).map_err(|error| {
            format!(
                "lease_authority_trust_generation_publish_failed:{}:{error}",
                final_path.display()
            )
        })?;
        sync_authority_key_directory(&generations)?;
        let selector = LeaseAuthorityTrustSelector {
            schema_version: LEASE_AUTHORITY_TRUST_SELECTOR_SCHEMA_VERSION.to_string(),
            generation_id,
            key_epoch: signing_key.key_epoch,
        };
        write_private_json_atomic_replace(&lease_authority_trust_selector_path_in(root), &selector)
    })();
    if result.is_err() && temporary.exists() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}

fn validate_persisted_lease_authority_trust_generation(
    path: &Path,
    generation_id: &str,
    signing_key: &LeaseAuthoritySigningKey,
    keyring: &LeaseAuthorityVerificationKeyring,
) -> Result<(), String> {
    ensure_private_directory(path)?;
    let manifest: LeaseAuthorityTrustGenerationManifest = load_private_json_file(
        &path.join(LEASE_AUTHORITY_TRUST_GENERATION_MANIFEST_FILE),
        "lease_authority_trust_manifest_decode_failed",
    )?;
    let signing_path = path.join(LEASE_AUTHORITY_SIGNING_KEY_FILE);
    let verification_path = path.join(LEASE_AUTHORITY_VERIFICATION_KEY_FILE);
    if manifest.schema_version != LEASE_AUTHORITY_TRUST_GENERATION_SCHEMA_VERSION
        || manifest.generation_id != generation_id
        || manifest.key_epoch != signing_key.key_epoch
        || manifest.active_key_id != signing_key.key_id
        || manifest.signing_key_sha256 != file_sha256(&signing_path)?
        || manifest.verification_keyring_sha256 != file_sha256(&verification_path)?
        || load_lease_authority_signing_key_file(&signing_path)? != *signing_key
        || load_lease_authority_verification_key_file(&verification_path)? != *keyring
    {
        return Err("lease_authority_trust_generation_existing_mismatch".to_string());
    }
    Ok(())
}

#[cfg(test)]
fn rotate_lease_authority_trust_generation_in(
    root: &Path,
    expected: &LeaseAuthorityTrustSelector,
    signing_key: &LeaseAuthoritySigningKey,
    keyring: &LeaseAuthorityVerificationKeyring,
) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| {
        format!(
            "lease_authority_trust_directory_create_failed:{}:{error}",
            root.display()
        )
    })?;
    set_private_directory_permissions(root)?;
    let lock_path = root.join("selection.lock");
    let lock = open_private_lock_file(&lock_path)?;
    lock.lock().map_err(|error| {
        format!(
            "lease_authority_trust_selection_lock_failed:{}:{error}",
            lock_path.display()
        )
    })?;
    let current = load_selected_lease_authority_trust_generation_in(root)?;
    if current.selector != *expected {
        return Err("lease_authority_trust_selector_stale".to_string());
    }
    let expected_epoch = expected
        .key_epoch
        .checked_add(1)
        .ok_or_else(|| "lease_authority_signing_key_epoch_exhausted".to_string())?;
    if signing_key.key_epoch != expected_epoch
        || keyring.key_epoch != expected_epoch
        || keyring.active_key_id != signing_key.key_id
    {
        return Err("lease_authority_signing_key_epoch_mismatch".to_string());
    }
    persist_lease_authority_trust_generation_in(root, signing_key, keyring)
}

fn verification_keyring_document(
    keyring: &LeaseAuthorityVerificationKeyring,
) -> LeaseAuthorityVerificationKeyFile {
    LeaseAuthorityVerificationKeyFile {
        schema_version: LEASE_AUTHORITY_VERIFICATION_KEY_SCHEMA_VERSION.to_string(),
        key_epoch: keyring.key_epoch,
        active_key_id: keyring.active_key_id.clone(),
        keys: keyring
            .keys
            .values()
            .map(|key| LeaseAuthorityVerificationKeyFileEntry {
                key_epoch: key.key_epoch,
                key_id: key.key_id.clone(),
                public_key_hex: hex::encode(key.public_key),
            })
            .collect(),
    }
}

fn load_lease_authority_signing_key_file(path: &Path) -> Result<LeaseAuthoritySigningKey, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "lease_authority_signing_key_metadata_failed:{}:{error}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "lease_authority_signing_key_not_private_file:{}",
            path.display()
        ));
    }
    ensure_private_file_permissions(path, &metadata)?;
    let encoded = fs::read(path).map_err(|error| {
        format!(
            "lease_authority_signing_key_read_failed:{}:{error}",
            path.display()
        )
    })?;
    let document: LeaseAuthoritySigningKeyFile = serde_json::from_slice(&encoded)
        .map_err(|error| format!("lease_authority_signing_key_decode_failed:{error}"))?;
    if document.schema_version != LEASE_AUTHORITY_SIGNING_KEY_SCHEMA_VERSION {
        return Err("lease_authority_signing_key_schema_unsupported".to_string());
    }
    let decoded = hex::decode(&document.private_key_hex)
        .map_err(|_| "lease_authority_signing_key_private_key_invalid".to_string())?;
    let private_key: [u8; 32] = decoded
        .try_into()
        .map_err(|_| "lease_authority_signing_key_private_key_invalid".to_string())?;
    let key =
        LeaseAuthoritySigningKey::from_private_bytes_at_epoch(private_key, document.key_epoch);
    if key.key_id != document.key_id || hex::encode(key.public_key) != document.public_key_hex {
        return Err("lease_authority_signing_key_id_mismatch".to_string());
    }
    Ok(key)
}

fn load_lease_authority_verification_key_file(
    path: &Path,
) -> Result<LeaseAuthorityVerificationKeyring, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "lease_authority_verification_key_metadata_failed:{}:{error}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "lease_authority_verification_key_not_regular_file:{}",
            path.display()
        ));
    }
    ensure_private_file_permissions(path, &metadata)?;
    let encoded = fs::read(path).map_err(|error| {
        format!(
            "lease_authority_verification_key_read_failed:{}:{error}",
            path.display()
        )
    })?;
    let document: LeaseAuthorityVerificationKeyFile = serde_json::from_slice(&encoded)
        .map_err(|error| format!("lease_authority_verification_key_decode_failed:{error}"))?;
    if document.schema_version != LEASE_AUTHORITY_VERIFICATION_KEY_SCHEMA_VERSION {
        return Err("lease_authority_verification_key_schema_unsupported".to_string());
    }
    if document.key_epoch == 0
        || document.active_key_id.trim().is_empty()
        || document.keys.is_empty()
        || document.keys.len() > MAX_LEASE_AUTHORITY_VERIFICATION_KEYS
    {
        return Err("lease_authority_verification_keyring_invalid".to_string());
    }
    let mut keys = BTreeMap::new();
    for entry in document.keys {
        if entry.key_epoch == 0 || entry.key_epoch > document.key_epoch {
            return Err("lease_authority_verification_key_epoch_invalid".to_string());
        }
        let decoded = hex::decode(&entry.public_key_hex)
            .map_err(|_| "lease_authority_verification_key_invalid".to_string())?;
        let public_key: [u8; 32] = decoded
            .try_into()
            .map_err(|_| "lease_authority_verification_key_invalid".to_string())?;
        let key =
            LeaseAuthorityVerificationKey::from_public_bytes_at_epoch(public_key, entry.key_epoch)?;
        if key.key_id != entry.key_id || keys.insert(key.key_id.clone(), key).is_some() {
            return Err("lease_authority_verification_key_id_mismatch".to_string());
        }
    }
    let active = keys
        .get(&document.active_key_id)
        .ok_or_else(|| "lease_authority_verification_active_key_missing".to_string())?;
    if active.key_epoch != document.key_epoch {
        return Err("lease_authority_verification_key_id_mismatch".to_string());
    }
    Ok(LeaseAuthorityVerificationKeyring {
        key_epoch: document.key_epoch,
        active_key_id: document.active_key_id,
        keys,
    })
}

fn write_private_signing_key_file(path: &Path, encoded: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        format!(
            "lease_authority_signing_key_stage_failed:{}:{error}",
            path.display()
        )
    })?;
    file.write_all(encoded).map_err(|error| {
        format!(
            "lease_authority_signing_key_write_failed:{}:{error}",
            path.display()
        )
    })?;
    file.write_all(b"\n").map_err(|error| {
        format!(
            "lease_authority_signing_key_write_failed:{}:{error}",
            path.display()
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "lease_authority_signing_key_sync_failed:{}:{error}",
            path.display()
        )
    })
}

fn open_private_lock_file(path: &Path) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|error| {
        format!(
            "lease_authority_trust_selection_lock_open_failed:{}:{error}",
            path.display()
        )
    })?;
    let metadata = file.metadata().map_err(|error| {
        format!(
            "lease_authority_trust_selection_lock_metadata_failed:{}:{error}",
            path.display()
        )
    })?;
    ensure_private_file_permissions(path, &metadata)?;
    Ok(file)
}

fn write_private_json_atomic_replace<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "lease_authority_trust_selector_parent_missing".to_string())?;
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("lease_authority_trust_selector_encode_failed:{error}"))?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(LEASE_AUTHORITY_TRUST_SELECTOR_FILE),
        uuid::Uuid::new_v4()
    ));
    write_private_signing_key_file(&temporary, &encoded)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "lease_authority_trust_selector_publish_failed:{}:{error}",
            path.display()
        ));
    }
    sync_authority_key_directory(parent)
}

fn load_private_json_file<T: for<'de> Deserialize<'de>>(
    path: &Path,
    decode_error: &str,
) -> Result<T, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "lease_authority_trust_file_metadata_failed:{}:{error}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "lease_authority_trust_file_not_regular:{}",
            path.display()
        ));
    }
    ensure_private_file_permissions(path, &metadata)?;
    let encoded = fs::read(path).map_err(|error| {
        format!(
            "lease_authority_trust_file_read_failed:{}:{error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&encoded).map_err(|error| format!("{decode_error}:{error}"))
}

fn file_sha256(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| {
            format!(
                "lease_authority_trust_file_hash_failed:{}:{error}",
                path.display()
            )
        })
}

fn verify_file_sha256(path: &Path, expected: &str, code: &str) -> Result<(), String> {
    (file_sha256(path)? == expected)
        .then_some(())
        .ok_or_else(|| format!("{code}:{}", path.display()))
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "lease_authority_trust_directory_metadata_failed:{}:{error}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "lease_authority_trust_directory_not_private:{}",
            path.display()
        ));
    }
    ensure_private_directory_permissions(path, &metadata)
}

#[cfg(unix)]
fn sync_authority_key_directory(path: &Path) -> Result<(), String> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            format!(
                "lease_authority_key_directory_sync_failed:{}:{error}",
                path.display()
            )
        })
}

#[cfg(not(unix))]
fn sync_authority_key_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        format!(
            "lease_authority_signing_key_directory_permissions_failed:{}:{error}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn ensure_private_directory_permissions(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!(
            "lease_authority_trust_directory_permissions_too_broad:{}",
            path.display()
        ));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(format!(
            "lease_authority_trust_directory_owner_mismatch:{}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_directory_permissions(
    _path: &Path,
    _metadata: &fs::Metadata,
) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn ensure_private_file_permissions(path: &Path, metadata: &fs::Metadata) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!(
            "lease_authority_signing_key_permissions_too_broad:{}",
            path.display()
        ));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(format!(
            "lease_authority_signing_key_owner_mismatch:{}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_file_permissions(_path: &Path, _metadata: &fs::Metadata) -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseResourceKind {
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
pub struct LeaseResourceKey {
    pub kind: LeaseResourceKind,
    pub id: String,
}

impl LeaseResourceKey {
    pub fn profile(id: impl Into<String>) -> Self {
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
pub enum LeaseClaimMode {
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
pub enum LeaseEventKind {
    Acquired,
    Rejoined,
    Renewed,
    Released,
    Expired,
    Revoked,
    Recovered,
    RecoveryPlanned,
    RevocationPlanned,
    Superseded,
    OwnerCommitted,
    OwnerReconciled,
    OwnerAdoptionPrepared,
    OwnerAdopted,
    OwnerAdoptionUncertain,
    OwnerAdoptionAborted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseAuthorityEvent {
    pub event_id: String,
    pub resource: LeaseResourceKey,
    pub claim_id: String,
    pub principal_id: String,
    pub fencing_token: u64,
    pub kind: LeaseEventKind,
    pub occurred_at: String,
}

/// Durable result of one logical acquisition operation. Retaining this receipt
/// never retains or recreates operational authority after the claim expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseClaimAcquisitionReceipt {
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
pub struct LeaseClaimAcquisitionOutcome {
    pub claim: Option<ActiveLeaseClaim>,
    pub receipt: LeaseClaimAcquisitionReceipt,
    pub replayed: bool,
}

/// Authenticated, exact holder request to terminalize one current claim.
/// The authorization is verified inside the same repository mutation that
/// advances the fence and persists the terminal receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseLeaseClaimRequest {
    pub authorization: LeaseEffectAuthorization,
    pub idempotency_key: String,
    pub now: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseClaimTerminalReceipt {
    schema_version: String,
    receipt_id: String,
    request_digest: String,
    idempotency_key: String,
    operation: String,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    capability_id: String,
    capability_revision: u64,
    claim_revision: u64,
    released_fencing_token: u64,
    terminal_fencing_token: u64,
    authority_revision: u64,
    terminal_result: String,
    occurred_at: String,
}

impl LeaseClaimTerminalReceipt {
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub fn capability_revision(&self) -> u64 {
        self.capability_revision
    }

    pub fn profile_id(&self) -> Option<&str> {
        (self.resource.kind == LeaseResourceKind::Profile).then_some(self.resource.id.as_str())
    }

    pub fn claim_revision(&self) -> u64 {
        self.claim_revision
    }

    pub fn released_fencing_token(&self) -> u64 {
        self.released_fencing_token
    }

    pub fn authority_revision(&self) -> u64 {
        self.authority_revision
    }

    pub fn terminal_fencing_token(&self) -> u64 {
        self.terminal_fencing_token
    }

    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseClaimReleaseOutcome {
    pub receipt: LeaseClaimTerminalReceipt,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEffectIntent {
    pub action_class: String,
    pub audience: String,
    pub operation_idempotency_key: String,
    pub executor_identity_digest: Option<String>,
    pub issued_at: String,
    pub authorization_expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEffectContext<'a> {
    pub action_class: &'a str,
    pub audience: &'a str,
    pub operation_idempotency_key: &'a str,
}

/// Named strict-controller request to re-fence and resume one exact claim.
/// It deliberately carries no global authority revision, so unrelated claims
/// cannot prevent crash recovery for this resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverLeaseClaimRequest {
    pub authorization: LeaseRecoveryAuthorization,
    pub now: String,
}

/// Exact administrative request to fence and terminalize one current claim.
/// It is independent of the target holder capability and optional runtime
/// projections. The administrative authorization is separately authenticated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeLeaseClaimRequest {
    pub authorization: LeaseAdministrativeAuthorization,
    pub now: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseAdministrativeIntent {
    pub administrator_id: String,
    pub administrator_revision: u64,
    pub idempotency_key: String,
    pub reason_code: String,
    pub issued_at: String,
    pub authorization_expires_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseAdministratorState {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseAdministratorAuthority {
    administrator_id: String,
    capability_digest: String,
    revision: u64,
    state: LeaseAdministratorState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRecoveryIntent {
    pub idempotency_key: String,
    pub issued_at: String,
    pub authorization_expires_at: String,
    pub claim_expires_at: String,
    pub transition_deadline: String,
    pub owner_generation: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseClaimRecoveryReceipt {
    schema_version: String,
    receipt_id: String,
    request_digest: String,
    idempotency_key: String,
    operation: String,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    recovery_controller_id: String,
    recovery_controller_revision: u64,
    previous_claim_revision: u64,
    claim_revision: u64,
    previous_fencing_token: u64,
    fencing_token: u64,
    authority_revision: u64,
    expires_at: String,
    transition_deadline: String,
    owner_generation: Option<u64>,
    terminal_result: String,
    occurred_at: String,
}

impl LeaseClaimRecoveryReceipt {
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseClaimRecoveryOutcome {
    pub claim: Option<ActiveLeaseClaim>,
    pub receipt: LeaseClaimRecoveryReceipt,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRecoveryPlanOutcome {
    pub authorization: LeaseRecoveryAuthorization,
    pub replayed: bool,
}

/// Exact claim envelope revalidated against canonical authority immediately
/// before an effect. It contains no raw capability material.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseEffectAuthorization {
    schema_version: String,
    signing_key_id: String,
    signing_key_epoch: u64,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    capability_id: String,
    capability_revision: u64,
    claim_revision: u64,
    fencing_token: u64,
    owner_generation: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    executor_identity_digest: Option<String>,
    action_class: String,
    audience: String,
    operation_idempotency_key: String,
    issued_at: String,
    authorization_expires_at: String,
    proof: String,
}

/// Authenticated bearer for the one recovery controller named by a strict
/// claim. Observable claim fields alone cannot construct this value.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseRecoveryAuthorization {
    schema_version: String,
    signing_key_id: String,
    signing_key_epoch: u64,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    recovery_controller_id: String,
    recovery_controller_revision: u64,
    claim_revision: u64,
    fencing_token: u64,
    idempotency_key: String,
    issued_at: String,
    authorization_expires_at: String,
    claim_expires_at: String,
    transition_deadline: String,
    owner_generation: Option<u64>,
    proof: String,
}

/// Scoped administrative authority for one exact claim revision and fence.
/// Observable claim fields and target-holder capabilities cannot construct a
/// valid proof. Debug output always redacts the signature.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseAdministrativeAuthorization {
    schema_version: String,
    signing_key_id: String,
    signing_key_epoch: u64,
    administrator_id: String,
    administrator_revision: u64,
    resource: LeaseResourceKey,
    claim_id: String,
    principal_id: String,
    claim_revision: u64,
    fencing_token: u64,
    idempotency_key: String,
    reason_code: String,
    issued_at: String,
    authorization_expires_at: String,
    proof: String,
}

impl std::fmt::Debug for LeaseAdministrativeAuthorization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseAdministrativeAuthorization")
            .field("schema_version", &self.schema_version)
            .field("signing_key_id", &self.signing_key_id)
            .field("signing_key_epoch", &self.signing_key_epoch)
            .field("administrator_id", &self.administrator_id)
            .field("administrator_revision", &self.administrator_revision)
            .field("resource", &self.resource)
            .field("claim_id", &self.claim_id)
            .field("principal_id", &self.principal_id)
            .field("claim_revision", &self.claim_revision)
            .field("fencing_token", &self.fencing_token)
            .field("idempotency_key", &self.idempotency_key)
            .field("reason_code", &self.reason_code)
            .field("issued_at", &self.issued_at)
            .field("authorization_expires_at", &self.authorization_expires_at)
            .field("proof", &"[REDACTED]")
            .finish()
    }
}

impl LeaseAdministrativeAuthorization {
    pub fn plan_id(&self) -> String {
        stable_id(
            "lease-administrative-revoke-plan-v1",
            &administrative_authorization_payload(self),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseAdministrativePlanOutcome {
    pub authorization: LeaseAdministrativeAuthorization,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseClaimRevocationOutcome {
    pub receipt: LeaseClaimTerminalReceipt,
    pub replayed: bool,
}

impl LeaseRecoveryAuthorization {
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn recovery_controller_id(&self) -> &str {
        &self.recovery_controller_id
    }

    pub fn plan_id(&self) -> String {
        stable_id(
            "lease-recovery-plan-v1",
            &recovery_authorization_payload(self),
        )
    }
}

impl std::fmt::Debug for LeaseRecoveryAuthorization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseRecoveryAuthorization")
            .field("schema_version", &self.schema_version)
            .field("signing_key_id", &self.signing_key_id)
            .field("resource", &self.resource)
            .field("claim_id", &self.claim_id)
            .field("principal_id", &self.principal_id)
            .field("recovery_controller_id", &self.recovery_controller_id)
            .field(
                "recovery_controller_revision",
                &self.recovery_controller_revision,
            )
            .field("claim_revision", &self.claim_revision)
            .field("fencing_token", &self.fencing_token)
            .field("idempotency_key", &self.idempotency_key)
            .field("issued_at", &self.issued_at)
            .field("authorization_expires_at", &self.authorization_expires_at)
            .field("claim_expires_at", &self.claim_expires_at)
            .field("transition_deadline", &self.transition_deadline)
            .field("owner_generation", &self.owner_generation)
            .field("proof", &"[REDACTED]")
            .finish()
    }
}

impl LeaseEffectAuthorization {
    /// Reject unsupported bearer envelopes before repository or trust lookup.
    pub fn validate_schema(&self) -> Result<(), LeaseAuthorityError> {
        if self.schema_version == LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(LeaseAuthorityError::UnsupportedSchema)
        }
    }

    pub fn profile_id(&self) -> Option<&str> {
        (self.resource.kind == LeaseResourceKind::Profile).then_some(self.resource.id.as_str())
    }

    pub fn operation_idempotency_key(&self) -> &str {
        &self.operation_idempotency_key
    }
}

impl std::fmt::Debug for LeaseEffectAuthorization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseEffectAuthorization")
            .field("schema_version", &self.schema_version)
            .field("signing_key_id", &self.signing_key_id)
            .field("resource", &self.resource)
            .field("claim_id", &self.claim_id)
            .field("principal_id", &self.principal_id)
            .field("capability_id", &self.capability_id)
            .field("capability_revision", &self.capability_revision)
            .field("claim_revision", &self.claim_revision)
            .field("fencing_token", &self.fencing_token)
            .field("owner_generation", &self.owner_generation)
            .field("executor_identity_digest", &self.executor_identity_digest)
            .field("action_class", &self.action_class)
            .field("audience", &self.audience)
            .field("operation_idempotency_key", &self.operation_idempotency_key)
            .field("issued_at", &self.issued_at)
            .field("authorization_expires_at", &self.authorization_expires_at)
            .field("proof", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveLeaseClaim {
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
    /// The exact resource fenced by this claim.
    pub fn resource(&self) -> &LeaseResourceKey {
        &self.resource
    }

    /// Optional runtime-owner generation bound when the claim was acquired.
    pub fn owner_generation(&self) -> Option<u64> {
        self.owner_generation
    }

    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub fn profile_id(&self) -> Option<&str> {
        (self.resource.kind == LeaseResourceKind::Profile).then_some(self.resource.id.as_str())
    }

    pub fn mode(&self) -> LeaseClaimMode {
        self.mode
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn fencing_token(&self) -> u64 {
        self.fencing_token
    }

    pub fn heartbeat_at(&self) -> &str {
        &self.heartbeat_at
    }

    pub fn expires_at(&self) -> &str {
        &self.expires_at
    }

    fn effect_authorization(
        &self,
        capability: &crate::ServiceProfileCapability,
        intent: &LeaseEffectIntent,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseEffectAuthorization, LeaseAuthorityError> {
        if capability.capability_id != self.capability_id
            || capability.principal_id != self.principal_id
            || capability.revision != self.capability_revision
            || capability.state != crate::ServiceProfileCapabilityState::Active
            || self
                .profile_id()
                .is_some_and(|profile_id| capability.profile_id != profile_id)
        {
            return Err(LeaseAuthorityError::CapabilityMismatch);
        }
        validate_effect_intent(intent)?;
        let mut authorization = LeaseEffectAuthorization {
            schema_version: LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION.to_string(),
            signing_key_id: signing_key.key_id.clone(),
            signing_key_epoch: signing_key.key_epoch,
            resource: self.resource.clone(),
            claim_id: self.claim_id.clone(),
            principal_id: self.principal_id.clone(),
            capability_id: self.capability_id.clone(),
            capability_revision: self.capability_revision,
            claim_revision: self.revision,
            fencing_token: self.fencing_token,
            owner_generation: self.owner_generation,
            executor_identity_digest: intent.executor_identity_digest.clone(),
            action_class: intent.action_class.clone(),
            audience: intent.audience.clone(),
            operation_idempotency_key: intent.operation_idempotency_key.clone(),
            issued_at: intent.issued_at.clone(),
            authorization_expires_at: intent.authorization_expires_at.clone(),
            proof: String::new(),
        };
        authorization.proof = sign_effect_authorization(&authorization, signing_key)?;
        Ok(authorization)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LeaseAuthorityState {
    schema_version: String,
    revision: u64,
    active_claims: BTreeMap<String, ActiveLeaseClaim>,
    next_fencing_tokens: BTreeMap<String, u64>,
    events: Vec<LeaseAuthorityEvent>,
    acquisition_receipts: BTreeMap<String, LeaseClaimAcquisitionReceipt>,
    terminal_receipts: BTreeMap<String, LeaseClaimTerminalReceipt>,
    recovery_receipts: BTreeMap<String, LeaseClaimRecoveryReceipt>,
    administrators: BTreeMap<String, LeaseAdministratorAuthority>,
    #[serde(skip)]
    recovery_authorizations: BTreeMap<String, LeaseRecoveryAuthorization>,
    #[serde(skip)]
    administrative_authorizations: BTreeMap<String, LeaseAdministrativeAuthorization>,
}

impl LeaseAuthorityState {
    fn bootstrap_administrator(
        &mut self,
        administrator_id: &str,
        raw_capability: &[u8],
    ) -> Result<LeaseAdministratorAuthority, LeaseAuthorityError> {
        if !self.is_empty() || administrator_id.trim().is_empty() || raw_capability.len() < 32 {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        let administrator = LeaseAdministratorAuthority {
            administrator_id: administrator_id.to_string(),
            capability_digest: administrator_capability_digest(raw_capability),
            revision: 1,
            state: LeaseAdministratorState::Active,
        };
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        self.revision = 1;
        self.administrators
            .insert(administrator_id.to_string(), administrator.clone());
        Ok(administrator)
    }

    fn authenticate_administrator(
        &self,
        administrator_id: &str,
        administrator_revision: u64,
        raw_capability: &[u8],
    ) -> Result<&LeaseAdministratorAuthority, LeaseAuthorityError> {
        if raw_capability.len() < 32 {
            return Err(LeaseAuthorityError::AdministrativeAuthorityMismatch);
        }
        let administrator = self
            .administrators
            .get(administrator_id)
            .ok_or(LeaseAuthorityError::AdministrativeAuthorityMismatch)?;
        if administrator.state != LeaseAdministratorState::Active
            || administrator.revision != administrator_revision
            || administrator.capability_digest != administrator_capability_digest(raw_capability)
        {
            return Err(LeaseAuthorityError::AdministrativeAuthorityMismatch);
        }
        Ok(administrator)
    }

    fn plan_administrative_revocation(
        &mut self,
        claim: &ActiveLeaseClaim,
        intent: &LeaseAdministrativeIntent,
        raw_administrator_capability: &[u8],
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseAdministrativePlanOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(replayed) = self.replay_administrative_revocation_plan(
            &claim.resource,
            &claim.claim_id,
            claim.revision,
            claim.fencing_token,
            intent,
            raw_administrator_capability,
        )? {
            return Ok(replayed);
        }
        let authorization = issue_lease_administrative_authorization_with_signing_key(
            self,
            claim,
            intent,
            signing_key,
        )
        .map_err(|_| LeaseAuthorityError::InvalidRequest)?;
        let next_revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        self.revision = next_revision;
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        self.administrative_authorizations
            .insert(intent.idempotency_key.clone(), authorization.clone());
        self.events.push(LeaseAuthorityEvent {
            event_id: stable_id(
                "lease-event-v1",
                &format!(
                    "{}\0revocation_planned\0{}\0{}",
                    claim.claim_id, intent.idempotency_key, next_revision
                ),
            ),
            resource: claim.resource.clone(),
            claim_id: claim.claim_id.clone(),
            principal_id: claim.principal_id.clone(),
            fencing_token: claim.fencing_token,
            kind: LeaseEventKind::RevocationPlanned,
            occurred_at: intent.issued_at.clone(),
        });
        Ok(LeaseAdministrativePlanOutcome {
            authorization,
            replayed: false,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn replay_administrative_revocation_plan(
        &self,
        resource: &LeaseResourceKey,
        claim_id: &str,
        claim_revision: u64,
        fencing_token: u64,
        intent: &LeaseAdministrativeIntent,
        raw_administrator_capability: &[u8],
    ) -> Result<Option<LeaseAdministrativePlanOutcome>, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        self.authenticate_administrator(
            &intent.administrator_id,
            intent.administrator_revision,
            raw_administrator_capability,
        )?;
        let Some(existing) = self
            .administrative_authorizations
            .get(&intent.idempotency_key)
        else {
            return Ok(None);
        };
        if existing.administrator_id != intent.administrator_id
            || existing.administrator_revision != intent.administrator_revision
            || existing.resource != *resource
            || existing.claim_id != claim_id
            || existing.claim_revision != claim_revision
            || existing.fencing_token != fencing_token
            || existing.reason_code != intent.reason_code
        {
            return Err(LeaseAuthorityError::IdempotencyConflict);
        }
        Ok(Some(LeaseAdministrativePlanOutcome {
            authorization: existing.clone(),
            replayed: true,
        }))
    }

    fn administrative_authorization_by_plan_id(
        &self,
        plan_id: &str,
    ) -> Result<&LeaseAdministrativeAuthorization, LeaseAuthorityError> {
        if plan_id.trim().is_empty() {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        self.administrative_authorizations
            .values()
            .find(|authorization| authorization.plan_id() == plan_id)
            .ok_or(LeaseAuthorityError::InvalidAdministrativeProof)
    }

    fn plan_recovery(
        &mut self,
        claim: &ActiveLeaseClaim,
        controller: &crate::ServiceProfileCapability,
        intent: &LeaseRecoveryIntent,
        signing_key: &LeaseAuthoritySigningKey,
    ) -> Result<LeaseRecoveryPlanOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(existing) = self.recovery_authorizations.get(&intent.idempotency_key) {
            if existing.resource != claim.resource
                || existing.claim_id != claim.claim_id
                || existing.principal_id != claim.principal_id
                || existing.recovery_controller_id != controller.capability_id
                || existing.recovery_controller_revision != controller.revision
                || existing.claim_revision != claim.revision
                || existing.fencing_token != claim.fencing_token
                || existing.owner_generation != intent.owner_generation
            {
                return Err(LeaseAuthorityError::IdempotencyConflict);
            }
            return Ok(LeaseRecoveryPlanOutcome {
                authorization: existing.clone(),
                replayed: true,
            });
        }
        if claim.mode != LeaseClaimMode::Strict
            || claim.recovery_controller_id.as_deref() != Some(controller.capability_id.as_str())
            || controller.state != crate::ServiceProfileCapabilityState::Active
            || controller.principal_id != claim.principal_id
            || claim
                .profile_id()
                .is_some_and(|profile_id| controller.profile_id != profile_id)
        {
            return Err(LeaseAuthorityError::RecoveryControllerMismatch);
        }
        let authorization = issue_lease_recovery_authorization_with_signing_key(
            self,
            claim,
            controller,
            intent,
            signing_key,
        )?;
        let next_revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        self.revision = next_revision;
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        self.recovery_authorizations
            .insert(intent.idempotency_key.clone(), authorization.clone());
        self.events.push(LeaseAuthorityEvent {
            event_id: stable_id(
                "lease-event-v1",
                &format!(
                    "{}\0recovery_planned\0{}\0{}",
                    claim.claim_id, intent.idempotency_key, next_revision
                ),
            ),
            resource: claim.resource.clone(),
            claim_id: claim.claim_id.clone(),
            principal_id: claim.principal_id.clone(),
            fencing_token: claim.fencing_token,
            kind: LeaseEventKind::RecoveryPlanned,
            occurred_at: intent.issued_at.clone(),
        });
        Ok(LeaseRecoveryPlanOutcome {
            authorization,
            replayed: false,
        })
    }

    fn recovery_authorization_by_plan_id(
        &self,
        plan_id: &str,
    ) -> Result<&LeaseRecoveryAuthorization, LeaseAuthorityError> {
        if plan_id.trim().is_empty() {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        self.recovery_authorizations
            .values()
            .find(|authorization| authorization.plan_id() == plan_id)
            .ok_or(LeaseAuthorityError::InvalidRecoveryProof)
    }

    pub fn is_empty(&self) -> bool {
        self.active_claims.is_empty()
            && self.next_fencing_tokens.is_empty()
            && self.events.is_empty()
            && self.acquisition_receipts.is_empty()
            && self.terminal_receipts.is_empty()
            && self.recovery_receipts.is_empty()
            && self.administrators.is_empty()
            && self.recovery_authorizations.is_empty()
            && self.administrative_authorizations.is_empty()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn acquire_with_receipt(
        &mut self,
        request: AcquireLeaseClaimRequest,
    ) -> Result<LeaseClaimAcquisitionOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(replayed) = self.replay_acquisition(&request)? {
            return Ok(replayed);
        }
        let request_digest = acquisition_request_digest(&request);
        validate_request(&request)?;
        let current_claim_revision = self.current_claim_revision(&request.resource, &request.now);
        if request.expected_claim_revision != current_claim_revision {
            return Err(LeaseAuthorityError::StaleClaimRevision);
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

    pub fn replay_acquisition(
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

    pub fn acquire(
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
        validate_request(&request)?;
        let current_claim_revision = self.current_claim_revision(&request.resource, &request.now);
        if request.expected_claim_revision != current_claim_revision {
            return Err(LeaseAuthorityError::StaleClaimRevision);
        }
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

    fn release_with_receipt(
        &mut self,
        request: ReleaseLeaseClaimRequest,
        verification_key: &LeaseAuthorityVerificationKeyring,
    ) -> Result<LeaseClaimReleaseOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(replayed) = self.replay_release(&request)? {
            return Ok(replayed);
        }
        verify_effect_authorization(&request.authorization, verification_key)?;
        let request_digest = release_request_digest(&request);
        if request.idempotency_key.trim().is_empty()
            || chrono::DateTime::parse_from_rfc3339(&request.now).is_err()
        {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        let resource_key = request.authorization.resource.storage_key();
        let release_context = LeaseEffectContext {
            action_class: "lease_release",
            audience: "lease_authority_kernel",
            operation_idempotency_key: &request.idempotency_key,
        };
        let claim = self
            .authorize_effect(&request.authorization, &request.now, &release_context)?
            .clone();
        let fencing_high_water = self
            .next_fencing_tokens
            .get(&resource_key)
            .copied()
            .unwrap_or(claim.fencing_token)
            .max(claim.fencing_token);
        let terminal_fencing_token = fencing_high_water
            .checked_add(1)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let next_authority_revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let receipt = LeaseClaimTerminalReceipt {
            schema_version: LEASE_TERMINAL_RECEIPT_SCHEMA_VERSION.to_string(),
            receipt_id: stable_id(
                "lease-terminal-receipt-v1",
                &format!("release\0{}\0{}", claim.claim_id, request.idempotency_key),
            ),
            request_digest,
            idempotency_key: request.idempotency_key.clone(),
            operation: "release".to_string(),
            resource: claim.resource.clone(),
            claim_id: claim.claim_id.clone(),
            principal_id: claim.principal_id.clone(),
            capability_id: claim.capability_id.clone(),
            capability_revision: claim.capability_revision,
            claim_revision: claim.revision,
            released_fencing_token: claim.fencing_token,
            terminal_fencing_token,
            authority_revision: next_authority_revision,
            terminal_result: "released".to_string(),
            occurred_at: request.now.clone(),
        };
        let mut event = terminal_event(&claim, LeaseEventKind::Released, &request.now);
        event.fencing_token = terminal_fencing_token;

        self.active_claims.remove(&resource_key);
        self.next_fencing_tokens
            .insert(resource_key, terminal_fencing_token);
        self.revision = next_authority_revision;
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        self.events.push(event);
        self.terminal_receipts
            .insert(request.idempotency_key, receipt.clone());
        Ok(LeaseClaimReleaseOutcome {
            receipt,
            replayed: false,
        })
    }

    /// Read the existing terminal receipt without reopening mutation custody.
    pub fn replay_release(
        &self,
        request: &ReleaseLeaseClaimRequest,
    ) -> Result<Option<LeaseClaimReleaseOutcome>, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        let Some(receipt) = self.terminal_receipts.get(&request.idempotency_key) else {
            return Ok(None);
        };
        if receipt.schema_version != LEASE_TERMINAL_RECEIPT_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        if receipt.request_digest != release_request_digest(request)
            || receipt.operation != "release"
        {
            return Err(LeaseAuthorityError::IdempotencyConflict);
        }
        Ok(Some(LeaseClaimReleaseOutcome {
            receipt: receipt.clone(),
            replayed: true,
        }))
    }

    fn revoke_with_receipt(
        &mut self,
        request: RevokeLeaseClaimRequest,
        verification_key: &LeaseAuthorityVerificationKeyring,
    ) -> Result<LeaseClaimRevocationOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(replayed) = self.replay_revocation(&request)? {
            return Ok(replayed);
        }
        verify_administrative_authorization(&request.authorization, verification_key)?;
        let authorization = &request.authorization;
        if self
            .administrative_authorizations
            .get(&authorization.idempotency_key)
            != Some(authorization)
        {
            return Err(LeaseAuthorityError::InvalidAdministrativeProof);
        }
        if authorization.idempotency_key.trim().is_empty()
            || authorization.reason_code.trim().is_empty()
            || authorization.administrator_id.trim().is_empty()
            || authorization.administrator_revision == 0
            || !timestamp_at_or_after(&request.now, &authorization.issued_at)
            || !timestamp_precedes(&request.now, &authorization.authorization_expires_at)
            || !timestamp_span_within(
                &authorization.issued_at,
                &authorization.authorization_expires_at,
                MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
            )
        {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        let resource_key = authorization.resource.storage_key();
        let claim = self
            .authorize_administrative_revocation(authorization, &request.now)?
            .clone();
        let fencing_high_water = self
            .next_fencing_tokens
            .get(&resource_key)
            .copied()
            .unwrap_or(claim.fencing_token)
            .max(claim.fencing_token);
        let terminal_fencing_token = fencing_high_water
            .checked_add(1)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let next_authority_revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let receipt = LeaseClaimTerminalReceipt {
            schema_version: LEASE_TERMINAL_RECEIPT_SCHEMA_VERSION.to_string(),
            receipt_id: stable_id(
                "lease-terminal-receipt-v1",
                &format!(
                    "revoke\0{}\0{}",
                    claim.claim_id, authorization.idempotency_key
                ),
            ),
            request_digest: revocation_request_digest(&request),
            idempotency_key: authorization.idempotency_key.clone(),
            operation: "revoke".to_string(),
            resource: claim.resource.clone(),
            claim_id: claim.claim_id.clone(),
            principal_id: claim.principal_id.clone(),
            capability_id: claim.capability_id.clone(),
            capability_revision: claim.capability_revision,
            claim_revision: claim.revision,
            released_fencing_token: claim.fencing_token,
            terminal_fencing_token,
            authority_revision: next_authority_revision,
            terminal_result: "revoked".to_string(),
            occurred_at: request.now.clone(),
        };
        let mut event = terminal_event(&claim, LeaseEventKind::Revoked, &request.now);
        event.fencing_token = terminal_fencing_token;

        self.active_claims.remove(&resource_key);
        self.next_fencing_tokens
            .insert(resource_key, terminal_fencing_token);
        self.revision = next_authority_revision;
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        self.events.push(event);
        self.terminal_receipts
            .insert(authorization.idempotency_key.clone(), receipt.clone());
        Ok(LeaseClaimRevocationOutcome {
            receipt,
            replayed: false,
        })
    }

    /// Read a completed administrative result without granting new authority.
    pub fn replay_revocation(
        &self,
        request: &RevokeLeaseClaimRequest,
    ) -> Result<Option<LeaseClaimRevocationOutcome>, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        let Some(receipt) = self
            .terminal_receipts
            .get(&request.authorization.idempotency_key)
        else {
            return Ok(None);
        };
        if receipt.schema_version != LEASE_TERMINAL_RECEIPT_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        if receipt.request_digest != revocation_request_digest(request)
            || receipt.operation != "revoke"
        {
            return Err(LeaseAuthorityError::IdempotencyConflict);
        }
        Ok(Some(LeaseClaimRevocationOutcome {
            receipt: receipt.clone(),
            replayed: true,
        }))
    }

    fn recover_with_receipt(
        &mut self,
        request: RecoverLeaseClaimRequest,
        verification_key: &LeaseAuthorityVerificationKeyring,
    ) -> Result<LeaseClaimRecoveryOutcome, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if let Some(replayed) = self.replay_recovery(&request)? {
            return Ok(replayed);
        }
        let retained = self.recovery_authorization_by_plan_id(&request.authorization.plan_id())?;
        if retained != &request.authorization {
            return Err(LeaseAuthorityError::InvalidRecoveryProof);
        }
        verify_recovery_authorization(&request.authorization, verification_key)?;
        let authorization = &request.authorization;
        if authorization.idempotency_key.trim().is_empty()
            || !timestamp_at_or_after(&request.now, &authorization.issued_at)
            || !timestamp_precedes(&request.now, &authorization.authorization_expires_at)
            || !timestamp_precedes(&request.now, &authorization.transition_deadline)
            || !timestamp_precedes(
                &authorization.transition_deadline,
                &authorization.claim_expires_at,
            )
            || !timestamp_span_within(
                &request.now,
                &authorization.claim_expires_at,
                MAX_STRICT_RECOVERY_TENURE_SECONDS,
            )
        {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        let resource_key = request.authorization.resource.storage_key();
        let current = self.authorize_recovery(&request.authorization, &request.now)?;
        let previous_claim_revision = current.revision;
        let previous_fencing_token = current.fencing_token;
        let next_claim_revision = previous_claim_revision
            .checked_add(1)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let fencing_high_water = self
            .next_fencing_tokens
            .get(&resource_key)
            .copied()
            .unwrap_or(previous_fencing_token)
            .max(previous_fencing_token);
        let next_fencing_token = fencing_high_water
            .checked_add(1)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let next_authority_revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityError::CounterExhausted)?;
        let mut recovered = current.clone();
        recovered.revision = next_claim_revision;
        recovered.fencing_token = next_fencing_token;
        recovered.heartbeat_at = request.now.clone();
        recovered.expires_at = authorization.claim_expires_at.clone();
        recovered.transition_deadline = Some(authorization.transition_deadline.clone());
        recovered.owner_generation = authorization.owner_generation;
        let receipt = LeaseClaimRecoveryReceipt {
            schema_version: LEASE_RECOVERY_RECEIPT_SCHEMA_VERSION.to_string(),
            receipt_id: stable_id(
                "lease-recovery-receipt-v1",
                &format!(
                    "{}\0{}",
                    request.authorization.claim_id, request.authorization.idempotency_key
                ),
            ),
            request_digest: recovery_request_digest(&request),
            idempotency_key: authorization.idempotency_key.clone(),
            operation: "recover".to_string(),
            resource: recovered.resource.clone(),
            claim_id: recovered.claim_id.clone(),
            principal_id: recovered.principal_id.clone(),
            recovery_controller_id: request.authorization.recovery_controller_id.clone(),
            recovery_controller_revision: request.authorization.recovery_controller_revision,
            previous_claim_revision,
            claim_revision: next_claim_revision,
            previous_fencing_token,
            fencing_token: next_fencing_token,
            authority_revision: next_authority_revision,
            expires_at: authorization.claim_expires_at.clone(),
            transition_deadline: authorization.transition_deadline.clone(),
            owner_generation: authorization.owner_generation,
            terminal_result: "recovered".to_string(),
            occurred_at: request.now.clone(),
        };
        let event = LeaseAuthorityEvent {
            event_id: stable_id(
                "lease-event-v1",
                &format!(
                    "{}\0recovered\0{}\0{}",
                    recovered.claim_id, recovered.revision, next_authority_revision
                ),
            ),
            resource: recovered.resource.clone(),
            claim_id: recovered.claim_id.clone(),
            principal_id: recovered.principal_id.clone(),
            fencing_token: recovered.fencing_token,
            kind: LeaseEventKind::Recovered,
            occurred_at: request.now,
        };

        self.revision = next_authority_revision;
        self.schema_version = LEASE_AUTHORITY_SCHEMA_VERSION.to_string();
        self.next_fencing_tokens
            .insert(resource_key.clone(), next_fencing_token);
        self.active_claims.insert(resource_key, recovered.clone());
        self.events.push(event);
        self.recovery_receipts
            .insert(authorization.idempotency_key.clone(), receipt.clone());
        Ok(LeaseClaimRecoveryOutcome {
            claim: Some(recovered),
            receipt,
            replayed: false,
        })
    }

    /// Read a recovery result without mutating its recorded claim or fence.
    pub fn replay_recovery(
        &self,
        request: &RecoverLeaseClaimRequest,
    ) -> Result<Option<LeaseClaimRecoveryOutcome>, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        let Some(receipt) = self
            .recovery_receipts
            .get(&request.authorization.idempotency_key)
        else {
            return Ok(None);
        };
        if receipt.schema_version != LEASE_RECOVERY_RECEIPT_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        if receipt.operation != "recover"
            || receipt.request_digest != recovery_request_digest(request)
        {
            return Err(LeaseAuthorityError::IdempotencyConflict);
        }
        let claim = self
            .active_claims
            .get(&receipt.resource.storage_key())
            .filter(|claim| {
                claim.claim_id == receipt.claim_id
                    && claim.revision == receipt.claim_revision
                    && claim.fencing_token == receipt.fencing_token
                    && timestamp_precedes(&request.now, &claim.expires_at)
            })
            .cloned();
        Ok(Some(LeaseClaimRecoveryOutcome {
            claim,
            receipt: receipt.clone(),
            replayed: true,
        }))
    }

    fn authorize_administrative_revocation(
        &self,
        authorization: &LeaseAdministrativeAuthorization,
        now: &str,
    ) -> Result<&ActiveLeaseClaim, LeaseAuthorityError> {
        if authorization.schema_version != LEASE_ADMINISTRATIVE_AUTHORIZATION_SCHEMA_VERSION
            || !timestamp_at_or_after(now, &authorization.issued_at)
            || !timestamp_precedes(now, &authorization.authorization_expires_at)
        {
            return Err(LeaseAuthorityError::InvalidRequest);
        }
        let administrator = self
            .administrators
            .get(&authorization.administrator_id)
            .ok_or(LeaseAuthorityError::AdministrativeAuthorityMismatch)?;
        if administrator.state != LeaseAdministratorState::Active
            || administrator.revision != authorization.administrator_revision
        {
            return Err(LeaseAuthorityError::AdministrativeAuthorityMismatch);
        }
        let claim = self
            .active_claims
            .get(&authorization.resource.storage_key())
            .ok_or(LeaseAuthorityError::ClaimUnavailable)?;
        if !timestamp_precedes(now, &claim.expires_at) {
            return Err(LeaseAuthorityError::ClaimExpired);
        }
        if claim.claim_id != authorization.claim_id
            || claim.principal_id != authorization.principal_id
            || claim.revision != authorization.claim_revision
            || claim.fencing_token != authorization.fencing_token
        {
            return Err(LeaseAuthorityError::StaleClaim);
        }
        Ok(claim)
    }

    fn authorize_recovery(
        &self,
        authorization: &LeaseRecoveryAuthorization,
        now: &str,
    ) -> Result<&ActiveLeaseClaim, LeaseAuthorityError> {
        if authorization.schema_version != LEASE_RECOVERY_AUTHORIZATION_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        let claim = self
            .active_claims
            .get(&authorization.resource.storage_key())
            .ok_or(LeaseAuthorityError::ClaimUnavailable)?;
        if !timestamp_precedes(now, &claim.expires_at) {
            return Err(LeaseAuthorityError::ClaimExpired);
        }
        if claim.mode != LeaseClaimMode::Strict {
            return Err(LeaseAuthorityError::StrictClaimRequired);
        }
        if claim.recovery_controller_id.as_deref()
            != Some(authorization.recovery_controller_id.as_str())
        {
            return Err(LeaseAuthorityError::RecoveryControllerMismatch);
        }
        if claim.claim_id != authorization.claim_id
            || claim.principal_id != authorization.principal_id
            || claim.revision != authorization.claim_revision
            || claim.fencing_token != authorization.fencing_token
        {
            return Err(LeaseAuthorityError::StaleClaim);
        }
        Ok(claim)
    }

    pub fn current_claim(
        &self,
        resource: &LeaseResourceKey,
        now: &str,
    ) -> Option<&ActiveLeaseClaim> {
        self.active_claims
            .get(&resource.storage_key())
            .filter(|claim| timestamp_precedes(now, &claim.expires_at))
    }

    pub fn current_claim_revision(&self, resource: &LeaseResourceKey, now: &str) -> u64 {
        self.active_claims
            .get(&resource.storage_key())
            .filter(|claim| timestamp_precedes(now, &claim.expires_at))
            .map_or(0, |claim| claim.revision)
    }

    pub fn current_claim_by_id(&self, claim_id: &str, now: &str) -> Option<&ActiveLeaseClaim> {
        self.active_claims
            .values()
            .find(|claim| claim.claim_id == claim_id && timestamp_precedes(now, &claim.expires_at))
    }

    /// Returns historical proof only. Callers must authenticate the requesting
    /// principal and match every holder axis before exposing a replay.
    pub fn terminal_release_receipt(
        &self,
        idempotency_key: &str,
    ) -> Option<&LeaseClaimTerminalReceipt> {
        self.terminal_receipts
            .get(idempotency_key)
            .filter(|receipt| receipt.operation == "release")
    }

    pub fn current_profile_claims<'a>(
        &'a self,
        now: &'a str,
    ) -> impl Iterator<Item = &'a ActiveLeaseClaim> + 'a {
        self.active_claims.values().filter(move |claim| {
            claim.resource.kind == LeaseResourceKind::Profile
                && timestamp_precedes(now, &claim.expires_at)
        })
    }

    fn authorize_effect(
        &self,
        authorization: &LeaseEffectAuthorization,
        now: &str,
        context: &LeaseEffectContext<'_>,
    ) -> Result<&ActiveLeaseClaim, LeaseAuthorityError> {
        self.ensure_supported_schema()?;
        if authorization.schema_version != LEASE_EFFECT_AUTHORIZATION_SCHEMA_VERSION {
            return Err(LeaseAuthorityError::UnsupportedSchema);
        }
        if !timestamp_at_or_after(now, &authorization.issued_at)
            || !timestamp_precedes(now, &authorization.authorization_expires_at)
            || !timestamp_span_within(
                &authorization.issued_at,
                &authorization.authorization_expires_at,
                MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
            )
        {
            return Err(LeaseAuthorityError::EffectAuthorizationExpired);
        }
        if authorization.action_class != context.action_class
            || authorization.audience != context.audience
            || authorization.operation_idempotency_key != context.operation_idempotency_key
        {
            return Err(LeaseAuthorityError::EffectScopeMismatch);
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
pub struct AcquireLeaseClaimRequest {
    pub resource: LeaseResourceKey,
    pub parent_claim_id: Option<String>,
    pub principal_id: String,
    pub capability_id: String,
    pub capability_revision: u64,
    pub mode: LeaseClaimMode,
    pub expected_claim_revision: u64,
    pub idempotency_key: String,
    pub now: String,
    pub expires_at: String,
    pub transition_deadline: Option<String>,
    pub recovery_controller_id: Option<String>,
    pub boot_epoch: Option<String>,
    pub owner_generation: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseAuthorityError {
    InvalidRequest,
    StaleClaimRevision,
    ClaimConflict,
    IdempotencyConflict,
    ParentClaimUnavailable,
    StrictRecoveryRequired,
    StrictClaimRequired,
    RecoveryControllerMismatch,
    CounterExhausted,
    ClaimUnavailable,
    ClaimExpired,
    StaleClaim,
    CapabilityUnavailable,
    CapabilityRevoked,
    CapabilityMismatch,
    SigningKeyMismatch,
    EffectScopeMismatch,
    EffectAuthorizationExpired,
    InvalidEffectProof,
    InvalidRecoveryProof,
    InvalidAdministrativeProof,
    AdministrativeAuthorityMismatch,
    UnsupportedSchema,
}

impl LeaseAuthorityError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::StaleClaimRevision => "stale_claim_revision",
            Self::ClaimConflict => "claim_conflict",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::ParentClaimUnavailable => "parent_claim_unavailable",
            Self::StrictRecoveryRequired => "strict_recovery_required",
            Self::StrictClaimRequired => "strict_claim_required",
            Self::RecoveryControllerMismatch => "recovery_controller_mismatch",
            Self::CounterExhausted => "counter_exhausted",
            Self::ClaimUnavailable => "claim_unavailable",
            Self::ClaimExpired => "claim_expired",
            Self::StaleClaim => "stale_claim",
            Self::CapabilityUnavailable => "capability_unavailable",
            Self::CapabilityRevoked => "capability_revoked",
            Self::CapabilityMismatch => "capability_mismatch",
            Self::SigningKeyMismatch => "signing_key_mismatch",
            Self::EffectScopeMismatch => "effect_scope_mismatch",
            Self::EffectAuthorizationExpired => "effect_authorization_expired",
            Self::InvalidEffectProof => "invalid_effect_proof",
            Self::InvalidRecoveryProof => "invalid_recovery_proof",
            Self::InvalidAdministrativeProof => "invalid_administrative_proof",
            Self::AdministrativeAuthorityMismatch => "administrative_authority_mismatch",
            Self::UnsupportedSchema => "unsupported_schema",
        }
    }
}

fn issue_lease_recovery_authorization_with_signing_key(
    authority: &LeaseAuthorityState,
    claim: &ActiveLeaseClaim,
    controller: &crate::ServiceProfileCapability,
    intent: &LeaseRecoveryIntent,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<LeaseRecoveryAuthorization, LeaseAuthorityError> {
    let current = authority
        .current_claim(&claim.resource, &intent.issued_at)
        .filter(|current| {
            current.claim_id == claim.claim_id
                && current.revision == claim.revision
                && current.fencing_token == claim.fencing_token
        })
        .ok_or(LeaseAuthorityError::ClaimUnavailable)?;
    if intent.idempotency_key.trim().is_empty()
        || current.mode != LeaseClaimMode::Strict
        || current.recovery_controller_id.as_deref() != Some(controller.capability_id.as_str())
        || controller.state != crate::ServiceProfileCapabilityState::Active
        || controller.principal_id != current.principal_id
        || current
            .profile_id()
            .is_some_and(|profile_id| controller.profile_id != profile_id)
        || !timestamp_precedes(&intent.issued_at, &intent.authorization_expires_at)
        || !timestamp_precedes(&intent.issued_at, &intent.transition_deadline)
        || !timestamp_precedes(&intent.transition_deadline, &intent.claim_expires_at)
        || !timestamp_span_within(
            &intent.issued_at,
            &intent.authorization_expires_at,
            MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
        )
        || !timestamp_span_within(
            &intent.issued_at,
            &intent.claim_expires_at,
            MAX_STRICT_RECOVERY_TENURE_SECONDS,
        )
    {
        return Err(LeaseAuthorityError::InvalidRequest);
    }
    let mut authorization = LeaseRecoveryAuthorization {
        schema_version: LEASE_RECOVERY_AUTHORIZATION_SCHEMA_VERSION.to_string(),
        signing_key_id: signing_key.key_id.clone(),
        signing_key_epoch: signing_key.key_epoch,
        resource: claim.resource.clone(),
        claim_id: current.claim_id.clone(),
        principal_id: current.principal_id.clone(),
        recovery_controller_id: controller.capability_id.clone(),
        recovery_controller_revision: controller.revision,
        claim_revision: current.revision,
        fencing_token: current.fencing_token,
        idempotency_key: intent.idempotency_key.clone(),
        issued_at: intent.issued_at.clone(),
        authorization_expires_at: intent.authorization_expires_at.clone(),
        claim_expires_at: intent.claim_expires_at.clone(),
        transition_deadline: intent.transition_deadline.clone(),
        owner_generation: intent.owner_generation,
        proof: String::new(),
    };
    authorization.proof = sign_recovery_authorization(&authorization, signing_key)?;
    Ok(authorization)
}

fn issue_lease_administrative_authorization_with_signing_key(
    authority: &LeaseAuthorityState,
    claim: &ActiveLeaseClaim,
    intent: &LeaseAdministrativeIntent,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<LeaseAdministrativeAuthorization, String> {
    if intent.administrator_id.trim().is_empty()
        || intent.administrator_revision == 0
        || intent.idempotency_key.trim().is_empty()
        || intent.reason_code.trim().is_empty()
        || !timestamp_precedes(&intent.issued_at, &intent.authorization_expires_at)
        || !timestamp_span_within(
            &intent.issued_at,
            &intent.authorization_expires_at,
            MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
        )
    {
        return Err("lease_authority_invalid_request".to_string());
    }
    let current = authority
        .current_claim(&claim.resource, &intent.issued_at)
        .filter(|current| {
            current.claim_id == claim.claim_id
                && current.revision == claim.revision
                && current.fencing_token == claim.fencing_token
        })
        .ok_or_else(|| "lease_authority_claim_unavailable".to_string())?;
    let mut authorization = LeaseAdministrativeAuthorization {
        schema_version: LEASE_ADMINISTRATIVE_AUTHORIZATION_SCHEMA_VERSION.to_string(),
        signing_key_id: signing_key.key_id.clone(),
        signing_key_epoch: signing_key.key_epoch,
        administrator_id: intent.administrator_id.clone(),
        administrator_revision: intent.administrator_revision,
        resource: current.resource.clone(),
        claim_id: current.claim_id.clone(),
        principal_id: current.principal_id.clone(),
        claim_revision: current.revision,
        fencing_token: current.fencing_token,
        idempotency_key: intent.idempotency_key.clone(),
        reason_code: intent.reason_code.clone(),
        issued_at: intent.issued_at.clone(),
        authorization_expires_at: intent.authorization_expires_at.clone(),
        proof: String::new(),
    };
    authorization.proof = sign_administrative_authorization(&authorization, signing_key)
        .map_err(|error| format!("lease_authority_{}", error.as_str()))?;
    Ok(authorization)
}

fn administrator_capability_digest(raw_capability: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(raw_capability))
}

fn validate_request(request: &AcquireLeaseClaimRequest) -> Result<(), LeaseAuthorityError> {
    if request.resource.id.trim().is_empty()
        || request.principal_id.trim().is_empty()
        || request.capability_id.trim().is_empty()
        || request.capability_revision == 0
        || request.idempotency_key.trim().is_empty()
        || !timestamp_precedes(&request.now, &request.expires_at)
        || !timestamp_span_within(
            &request.now,
            &request.expires_at,
            MAX_LEASE_CLAIM_TENURE_SECONDS,
        )
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

fn validate_effect_intent(intent: &LeaseEffectIntent) -> Result<(), LeaseAuthorityError> {
    if intent.action_class.trim().is_empty()
        || intent.audience.trim().is_empty()
        || intent.operation_idempotency_key.trim().is_empty()
        || intent
            .executor_identity_digest
            .as_deref()
            .is_some_and(|digest| {
                digest.strip_prefix("sha256:").is_none_or(|hex| {
                    hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
            })
        || !timestamp_precedes(&intent.issued_at, &intent.authorization_expires_at)
        || !timestamp_span_within(
            &intent.issued_at,
            &intent.authorization_expires_at,
            MAX_EFFECT_AUTHORIZATION_TENURE_SECONDS,
        )
    {
        return Err(LeaseAuthorityError::InvalidRequest);
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

fn timestamp_at_or_after(left: &str, right: &str) -> bool {
    let Ok(left) = chrono::DateTime::parse_from_rfc3339(left) else {
        return false;
    };
    let Ok(right) = chrono::DateTime::parse_from_rfc3339(right) else {
        return false;
    };
    left >= right
}

fn timestamp_span_within(start: &str, end: &str, maximum_seconds: i64) -> bool {
    let Ok(start) = chrono::DateTime::parse_from_rfc3339(start) else {
        return false;
    };
    let Ok(end) = chrono::DateTime::parse_from_rfc3339(end) else {
        return false;
    };
    let seconds = end.signed_duration_since(start).num_seconds();
    seconds > 0 && seconds <= maximum_seconds
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

fn release_request_digest(request: &ReleaseLeaseClaimRequest) -> String {
    stable_id(
        "lease-release-request-v2",
        &format!(
            "{}\0{}\0{}",
            effect_authorization_payload(&request.authorization),
            request.authorization.proof,
            request.idempotency_key,
        ),
    )
}

fn recovery_request_digest(request: &RecoverLeaseClaimRequest) -> String {
    stable_id(
        "lease-recovery-request-v1",
        &format!(
            "{}\0{}",
            recovery_authorization_payload(&request.authorization),
            request.authorization.proof
        ),
    )
}

fn revocation_request_digest(request: &RevokeLeaseClaimRequest) -> String {
    stable_id(
        "lease-revocation-request-v1",
        &format!(
            "{}\0{}",
            administrative_authorization_payload(&request.authorization),
            request.authorization.proof
        ),
    )
}

fn sign_effect_authorization(
    authorization: &LeaseEffectAuthorization,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<String, LeaseAuthorityError> {
    let key = Ed25519KeyPair::from_seed_unchecked(&signing_key.private_key)
        .map_err(|_| LeaseAuthorityError::SigningKeyMismatch)?;
    Ok(hex::encode(
        key.sign(effect_authorization_payload(authorization).as_bytes())
            .as_ref(),
    ))
}

fn verify_effect_authorization(
    authorization: &LeaseEffectAuthorization,
    verification_keys: &LeaseAuthorityVerificationKeyring,
) -> Result<(), LeaseAuthorityError> {
    let verification_key = verification_keys.verification_key(
        &authorization.signing_key_id,
        authorization.signing_key_epoch,
    )?;
    let proof =
        hex::decode(&authorization.proof).map_err(|_| LeaseAuthorityError::InvalidEffectProof)?;
    signature::UnparsedPublicKey::new(&signature::ED25519, verification_key.public_key)
        .verify(
            effect_authorization_payload(authorization).as_bytes(),
            &proof,
        )
        .map_err(|_| LeaseAuthorityError::InvalidEffectProof)
}

fn sign_recovery_authorization(
    authorization: &LeaseRecoveryAuthorization,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<String, LeaseAuthorityError> {
    let key = Ed25519KeyPair::from_seed_unchecked(&signing_key.private_key)
        .map_err(|_| LeaseAuthorityError::SigningKeyMismatch)?;
    Ok(hex::encode(
        key.sign(recovery_authorization_payload(authorization).as_bytes())
            .as_ref(),
    ))
}

fn verify_recovery_authorization(
    authorization: &LeaseRecoveryAuthorization,
    verification_keys: &LeaseAuthorityVerificationKeyring,
) -> Result<(), LeaseAuthorityError> {
    let verification_key = verification_keys.verification_key(
        &authorization.signing_key_id,
        authorization.signing_key_epoch,
    )?;
    let proof =
        hex::decode(&authorization.proof).map_err(|_| LeaseAuthorityError::InvalidRecoveryProof)?;
    signature::UnparsedPublicKey::new(&signature::ED25519, verification_key.public_key)
        .verify(
            recovery_authorization_payload(authorization).as_bytes(),
            &proof,
        )
        .map_err(|_| LeaseAuthorityError::InvalidRecoveryProof)
}

fn sign_administrative_authorization(
    authorization: &LeaseAdministrativeAuthorization,
    signing_key: &LeaseAuthoritySigningKey,
) -> Result<String, LeaseAuthorityError> {
    let key = Ed25519KeyPair::from_seed_unchecked(&signing_key.private_key)
        .map_err(|_| LeaseAuthorityError::SigningKeyMismatch)?;
    Ok(hex::encode(
        key.sign(administrative_authorization_payload(authorization).as_bytes())
            .as_ref(),
    ))
}

fn verify_administrative_authorization(
    authorization: &LeaseAdministrativeAuthorization,
    verification_keys: &LeaseAuthorityVerificationKeyring,
) -> Result<(), LeaseAuthorityError> {
    let verification_key = verification_keys.verification_key(
        &authorization.signing_key_id,
        authorization.signing_key_epoch,
    )?;
    let proof = hex::decode(&authorization.proof)
        .map_err(|_| LeaseAuthorityError::InvalidAdministrativeProof)?;
    signature::UnparsedPublicKey::new(&signature::ED25519, verification_key.public_key)
        .verify(
            administrative_authorization_payload(authorization).as_bytes(),
            &proof,
        )
        .map_err(|_| LeaseAuthorityError::InvalidAdministrativeProof)
}

fn effect_authorization_payload(authorization: &LeaseEffectAuthorization) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        authorization.schema_version,
        authorization.signing_key_id,
        authorization.signing_key_epoch,
        authorization.resource.storage_key(),
        authorization.claim_id,
        authorization.principal_id,
        authorization.capability_id,
        authorization.capability_revision,
        authorization.claim_revision,
        authorization.fencing_token,
        authorization.owner_generation.unwrap_or_default(),
        authorization
            .executor_identity_digest
            .as_deref()
            .unwrap_or_default(),
        authorization.action_class,
        authorization.audience,
        authorization.operation_idempotency_key,
        authorization.issued_at,
        authorization.authorization_expires_at,
    )
}

fn recovery_authorization_payload(authorization: &LeaseRecoveryAuthorization) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        authorization.schema_version,
        authorization.signing_key_id,
        authorization.signing_key_epoch,
        authorization.resource.storage_key(),
        authorization.claim_id,
        authorization.principal_id,
        authorization.recovery_controller_id,
        authorization.recovery_controller_revision,
        authorization.claim_revision,
        authorization.fencing_token,
        authorization.idempotency_key,
        authorization.issued_at,
        authorization.authorization_expires_at,
        authorization.claim_expires_at,
        authorization.transition_deadline,
        authorization.owner_generation.unwrap_or_default(),
    )
}

fn administrative_authorization_payload(
    authorization: &LeaseAdministrativeAuthorization,
) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        authorization.schema_version,
        authorization.signing_key_id,
        authorization.signing_key_epoch,
        authorization.administrator_id,
        authorization.administrator_revision,
        authorization.resource.storage_key(),
        authorization.claim_id,
        authorization.principal_id,
        authorization.claim_revision,
        authorization.fencing_token,
        authorization.idempotency_key,
        authorization.reason_code,
        authorization.issued_at,
        authorization.authorization_expires_at,
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
    use super::test_support::*;
    use super::*;

    #[test]
    fn signing_key_file_is_private_stable_and_never_serialized_into_a_bearer() {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-lease-signing-key-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(LEASE_AUTHORITY_SIGNING_KEY_FILE);
        let signing_key = signing_key();
        let document = LeaseAuthoritySigningKeyFile {
            schema_version: LEASE_AUTHORITY_SIGNING_KEY_SCHEMA_VERSION.to_string(),
            key_epoch: signing_key.key_epoch,
            key_id: signing_key.key_id.clone(),
            private_key_hex: hex::encode(signing_key.private_key),
            public_key_hex: hex::encode(signing_key.public_key),
        };
        write_private_signing_key_file(&path, &serde_json::to_vec(&document).unwrap()).unwrap();

        let loaded = load_lease_authority_signing_key_file(&path).unwrap();
        assert_eq!(loaded, signing_key);
        let debug = format!("{loaded:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(&document.private_key_hex));

        let verification_path = directory.join(LEASE_AUTHORITY_VERIFICATION_KEY_FILE);
        let verification_key = loaded.verification_key();
        let verification_keyring = LeaseAuthorityVerificationKeyring::from_active(&loaded);
        let verification_document = LeaseAuthorityVerificationKeyFile {
            schema_version: LEASE_AUTHORITY_VERIFICATION_KEY_SCHEMA_VERSION.to_string(),
            key_epoch: verification_keyring.key_epoch,
            active_key_id: verification_keyring.active_key_id.clone(),
            keys: vec![LeaseAuthorityVerificationKeyFileEntry {
                key_epoch: verification_key.key_epoch,
                key_id: verification_key.key_id.clone(),
                public_key_hex: hex::encode(verification_key.public_key),
            }],
        };
        let encoded_verification = serde_json::to_vec(&verification_document).unwrap();
        write_private_signing_key_file(&verification_path, &encoded_verification).unwrap();
        let loaded_verification =
            load_lease_authority_verification_key_file(&verification_path).unwrap();
        assert_eq!(loaded_verification, verification_keyring);
        assert!(!String::from_utf8(encoded_verification)
            .unwrap()
            .contains(&document.private_key_hex));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(load_lease_authority_signing_key_file(&path)
                .unwrap_err()
                .starts_with("lease_authority_signing_key_permissions_too_broad:"));
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let mut authority = LeaseAuthorityState::default();
        let claim = authority.acquire(request()).unwrap();
        let authorization = claim
            .effect_authorization(
                &capability(),
                &effect_intent("browser_launch", "session:last30days", "launch:tick-1"),
                &loaded,
            )
            .unwrap();
        let encoded_authorization = serde_json::to_string(&authorization).unwrap();
        assert!(encoded_authorization.contains(&loaded.key_id));
        assert!(!encoded_authorization.contains(&document.private_key_hex));
        verify_effect_authorization(&authorization, &loaded_verification).unwrap();

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn selected_trust_generation_is_atomic_rotatable_and_stale_safe() {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-lease-trust-generation-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&directory).unwrap();
        set_private_directory_permissions(&directory).unwrap();
        let first_signer = LeaseAuthoritySigningKey::from_private_bytes_at_epoch([0x5a; 32], 1);
        let first_keyring = LeaseAuthorityVerificationKeyring::from_active(&first_signer);

        persist_lease_authority_trust_generation_in(&directory, &first_signer, &first_keyring)
            .unwrap();
        let first_generation =
            load_selected_lease_authority_trust_generation_in(&directory).unwrap();
        assert_eq!(first_generation.selector.key_epoch, 1);
        assert_eq!(
            load_selected_lease_authority_signing_key_in(&directory).unwrap(),
            first_signer
        );
        assert_eq!(
            load_existing_lease_authority_verification_key_in(&directory).unwrap(),
            first_keyring
        );

        let second_signer = LeaseAuthoritySigningKey::from_private_bytes_at_epoch([0x6b; 32], 2);
        let second_keyring = first_keyring.with_rotated_active(&second_signer).unwrap();
        rotate_lease_authority_trust_generation_in(
            &directory,
            &first_generation.selector,
            &second_signer,
            &second_keyring,
        )
        .unwrap();
        let second_generation =
            load_selected_lease_authority_trust_generation_in(&directory).unwrap();
        assert_eq!(second_generation.selector.key_epoch, 2);
        assert_ne!(
            second_generation.selector.generation_id,
            first_generation.selector.generation_id
        );
        assert_eq!(
            load_selected_lease_authority_signing_key_in(&directory).unwrap(),
            second_signer
        );
        assert_eq!(
            load_existing_lease_authority_verification_key_in(&directory).unwrap(),
            second_keyring
        );
        assert_eq!(
            rotate_lease_authority_trust_generation_in(
                &directory,
                &first_generation.selector,
                &second_signer,
                &second_keyring,
            ),
            Err("lease_authority_trust_selector_stale".to_string())
        );

        let mut unsafe_selector = second_generation.selector.clone();
        unsafe_selector.generation_id = "../outside-trust-root".to_string();
        write_private_json_atomic_replace(
            &lease_authority_trust_selector_path_in(&directory),
            &unsafe_selector,
        )
        .unwrap();
        assert_eq!(
            load_selected_lease_authority_trust_generation_in(&directory),
            Err("lease_authority_trust_selector_invalid".to_string())
        );
        write_private_json_atomic_replace(
            &lease_authority_trust_selector_path_in(&directory),
            &second_generation.selector,
        )
        .unwrap();

        write_private_json_atomic_replace(
            &lease_authority_trust_selector_path_in(&directory),
            &first_generation.selector,
        )
        .unwrap();
        persist_lease_authority_trust_generation_in(&directory, &second_signer, &second_keyring)
            .unwrap();
        assert_eq!(
            load_selected_lease_authority_trust_generation_in(&directory)
                .unwrap()
                .selector,
            second_generation.selector
        );

        let selected_signer = second_generation
            .path
            .join(LEASE_AUTHORITY_SIGNING_KEY_FILE);
        let removed_signer = second_generation.path.join("removed-signing-key.json");
        fs::rename(&selected_signer, &removed_signer).unwrap();
        assert!(load_selected_lease_authority_signing_key_in(&directory)
            .unwrap_err()
            .starts_with("lease_authority_signing_key_recovery_required:"));
        fs::rename(&removed_signer, &selected_signer).unwrap();

        let selected_verifier = second_generation
            .path
            .join(LEASE_AUTHORITY_VERIFICATION_KEY_FILE);
        fs::write(&selected_verifier, b"{}\n").unwrap();
        assert!(
            load_existing_lease_authority_verification_key_in(&directory)
                .unwrap_err()
                .starts_with("lease_authority_verification_keyring_digest_mismatch:")
        );

        let legacy_service = directory.join("legacy-service");
        fs::create_dir_all(&legacy_service).unwrap();
        fs::write(
            legacy_service.join("lease-authority-verification-key.v1.json"),
            b"legacy",
        )
        .unwrap();
        assert_eq!(
            existing_legacy_authority_key_paths_in(&legacy_service),
            vec![legacy_service.join("lease-authority-verification-key.v1.json")]
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn verifier_keyring_preserves_bounded_old_proofs_and_rejects_epoch_rollback() {
        let first_signer = LeaseAuthoritySigningKey::from_private_bytes_at_epoch([0x5a; 32], 1);
        let second_signer = LeaseAuthoritySigningKey::from_private_bytes_at_epoch([0x6b; 32], 2);
        let first_keyring = LeaseAuthorityVerificationKeyring::from_active(&first_signer);
        let rotated_keyring = first_keyring.with_rotated_active(&second_signer).unwrap();
        let mut authority = LeaseAuthorityState::default();
        let claim = authority.acquire(request()).unwrap();

        let first_authorization = claim
            .effect_authorization(
                &capability(),
                &effect_intent("browser_launch", "session:last30days", "launch:epoch-1"),
                &first_signer,
            )
            .unwrap();
        let second_authorization = claim
            .effect_authorization(
                &capability(),
                &effect_intent("browser_launch", "session:last30days", "launch:epoch-2"),
                &second_signer,
            )
            .unwrap();

        verify_effect_authorization(&first_authorization, &rotated_keyring).unwrap();
        verify_effect_authorization(&second_authorization, &rotated_keyring).unwrap();
        assert_eq!(
            verify_effect_authorization(&second_authorization, &first_keyring),
            Err(LeaseAuthorityError::SigningKeyMismatch)
        );

        let mut wrong_epoch = second_authorization;
        wrong_epoch.signing_key_epoch = 1;
        assert_eq!(
            verify_effect_authorization(&wrong_epoch, &rotated_keyring),
            Err(LeaseAuthorityError::SigningKeyMismatch)
        );

        let mut capped_keyring = first_keyring;
        for epoch in 2..=MAX_LEASE_AUTHORITY_VERIFICATION_KEYS as u64 {
            let signer =
                LeaseAuthoritySigningKey::from_private_bytes_at_epoch([epoch as u8; 32], epoch);
            capped_keyring = capped_keyring.with_rotated_active(&signer).unwrap();
        }
        let over_capacity = LeaseAuthoritySigningKey::from_private_bytes_at_epoch(
            [0x7f; 32],
            MAX_LEASE_AUTHORITY_VERIFICATION_KEYS as u64 + 1,
        );
        assert_eq!(
            capped_keyring.with_rotated_active(&over_capacity),
            Err("lease_authority_verification_keyring_capacity_exhausted".to_string())
        );
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
    fn acquisition_claim_revision_compare_and_swap_has_one_winner() {
        let mut authority = LeaseAuthorityState::default();
        let first = authority.acquire(request()).unwrap();
        let mut contender = request();
        contender.principal_id = "principal:foreign".to_string();
        contender.capability_id = "capability:foreign".to_string();
        contender.idempotency_key = "acquire:foreign:tick-1".to_string();

        assert_eq!(
            authority.acquire(contender),
            Err(LeaseAuthorityError::StaleClaimRevision)
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
    fn unrelated_authority_activity_cannot_create_a_profile_acquisition_conflict() {
        let mut authority = LeaseAuthorityState::default();
        authority
            .bootstrap_administrator(
                "administrator:local-root",
                b"root-administrator-capability-material-v1",
            )
            .unwrap();
        assert_eq!(authority.revision(), 1);

        let first = authority.acquire(request()).unwrap();
        assert_eq!(first.revision(), 1);
        let mut unrelated = request();
        unrelated.resource = LeaseResourceKey::profile("unrelated-profile");
        unrelated.principal_id = "principal:unrelated".to_string();
        unrelated.capability_id = "capability:unrelated".to_string();
        unrelated.idempotency_key = "acquire:unrelated:tick-1".to_string();
        unrelated.expected_claim_revision = 0;

        let unrelated = authority.acquire(unrelated).unwrap();
        assert_eq!(unrelated.revision(), 1);
        assert_eq!(authority.active_claims.len(), 2);
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
    fn caller_cannot_create_an_unbounded_claim() {
        let mut authority = LeaseAuthorityState::default();
        let mut excessive = request();
        excessive.expires_at = "2026-08-31T12:05:01Z".to_string();
        let before = authority.clone();

        assert_eq!(
            authority.acquire(excessive),
            Err(LeaseAuthorityError::InvalidRequest)
        );
        assert_eq!(authority, before);
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
        replay.expected_claim_revision = 0;

        let replayed = authority.acquire_with_receipt(replay).unwrap();

        assert!(replayed.replayed);
        assert!(replayed.claim.is_none());
        assert_eq!(replayed.receipt, first.receipt);
        assert_eq!(authority, after_first);
    }

    #[test]
    fn effect_authorization_is_redacted_and_expires_independently() {
        let mut authority = LeaseAuthorityState::default();
        let acquired = authority.acquire_with_receipt(request()).unwrap();
        let authorization = acquired
            .claim
            .unwrap()
            .effect_authorization(
                &capability(),
                &effect_intent("browser_launch", "session:last30days", "launch:tick-1"),
                &signing_key(),
            )
            .unwrap();
        let proof = authorization.proof.clone();
        let debug = format!("{authorization:?}");
        assert!(!debug.contains(&proof));
        assert!(debug.contains("[REDACTED]"));

        let context = LeaseEffectContext {
            action_class: "browser_launch",
            audience: "session:last30days",
            operation_idempotency_key: "launch:tick-1",
        };
        assert!(authority
            .authorize_effect(&authorization, "2026-08-31T12:01:59Z", &context)
            .is_ok());
        assert_eq!(
            authority.authorize_effect(&authorization, "2026-08-31T12:02:00Z", &context),
            Err(LeaseAuthorityError::EffectAuthorizationExpired)
        );
    }

    #[test]
    fn same_principal_new_operation_rejoins_current_claim() {
        let mut authority = LeaseAuthorityState::default();
        let first = authority.acquire_with_receipt(request()).unwrap();
        let first_claim = first.claim.unwrap();
        let mut rejoin = request();
        rejoin.expected_claim_revision = first_claim.revision();
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
        rejoin.expected_claim_revision = first.claim.as_ref().unwrap().revision();
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
}
