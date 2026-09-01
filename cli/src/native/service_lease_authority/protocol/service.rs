use serde::{Deserialize, Serialize};
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{
    custody, serve_lease_authority_connection, LeaseAuthorityDurableStore,
    LeaseAuthorityProtectedLoadContext, LeaseAuthorityProtocolError,
};

pub(super) const LEASE_AUTHORITY_SERVICE_PROCESS_ENV: &str =
    "AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_SERVICE";
pub(super) const LEASE_AUTHORITY_BOOTSTRAP_PROCESS_ENV: &str =
    "AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP";
const LEASE_AUTHORITY_BOOTSTRAP_GROUP_ID_ENV: &str =
    "AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_OPERATOR_GROUP_ID";
pub(super) const LEASE_AUTHORITY_SERVICE_STATE_ROOT: &str =
    "/var/lib/agent-browser/lease-authority";
pub(super) const LEASE_AUTHORITY_SERVICE_SOCKET_PATH: &str =
    "/run/agent-browser/lease-authority.sock";
const LEASE_AUTHORITY_SERVICE_GENERATIONS_ROOT: &str =
    "/usr/local/libexec/agent-browser/lease-authority/generations";
const LEASE_AUTHORITY_SERVICE_EXECUTABLE_NAME: &str = "agent-browser";
const LEASE_AUTHORITY_SERVICE_CONFIG_FILE: &str = "service-config.v1.json";
const LEASE_AUTHORITY_SERVICE_CONFIG_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-service-config.v1";
const LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-administrator-capability.v1";
const LEASE_AUTHORITY_ADMINISTRATOR_DIRECTORY: &str = "administrator";
const LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_FILE: &str = "active-capability.v1.json";
const LEASE_AUTHORITY_BOOTSTRAP_ADMINISTRATOR_ID: &str = "administrator:local-root";
const LEASE_AUTHORITY_SERVICE_STORE_DIRECTORY: &str = "store";
const LEASE_AUTHORITY_SERVICE_TRUST_DIRECTORY: &str = "trust";
const LEASE_AUTHORITY_SYSTEMD_LISTEN_FD: i32 = 3;
const LEASE_AUTHORITY_CONNECTION_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityServiceConfig {
    schema_version: String,
    authority_domain_id: String,
    minimum_authority_epoch: u64,
    operator_group_id: u32,
    administrator_id: String,
    administrator_revision: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityAdministratorCapabilityFile {
    schema_version: String,
    administrator_id: String,
    administrator_revision: u64,
    capability_hex: String,
}

impl std::fmt::Debug for LeaseAuthorityAdministratorCapabilityFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseAuthorityAdministratorCapabilityFile")
            .field("schema_version", &self.schema_version)
            .field("administrator_id", &self.administrator_id)
            .field("administrator_revision", &self.administrator_revision)
            .field("capability_hex", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct LeaseAuthorityBootstrapReceipt {
    authority_domain_id: String,
    authority_epoch: u64,
    administrator_id: String,
    administrator_revision: u64,
}

struct LeaseAuthorityBootstrapSecret([u8; 32]);

impl LeaseAuthorityBootstrapSecret {
    fn expose(&self) -> &[u8] {
        &self.0
    }

    fn expose_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl std::fmt::Debug for LeaseAuthorityBootstrapSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl Drop for LeaseAuthorityBootstrapSecret {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

pub(super) fn validate_linux_service_launch(
    effective_uid: u32,
    process_id: u32,
    current_executable: &Path,
) -> Result<(), LeaseAuthorityProtocolError> {
    if effective_uid != 0 {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_service_root_required",
        });
    }
    if process_id <= 1 {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_service_process_identity_invalid",
        });
    }
    let generations_root = Path::new(LEASE_AUTHORITY_SERVICE_GENERATIONS_ROOT);
    let relative = current_executable
        .strip_prefix(generations_root)
        .map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_service_executable_untrusted",
        })?;
    let components: Vec<_> = relative.components().collect();
    if components.len() != 2
        || components[1].as_os_str() != LEASE_AUTHORITY_SERVICE_EXECUTABLE_NAME
        || !components[0]
            .as_os_str()
            .to_str()
            .is_some_and(super::authority_store_generation_component_is_safe)
    {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_service_executable_untrusted",
        });
    }
    Ok(())
}

pub(super) fn validate_systemd_socket_activation(
    listen_pid: Option<u32>,
    listen_fds: Option<u32>,
    process_id: u32,
) -> Result<(), LeaseAuthorityProtocolError> {
    if process_id <= 1 || listen_pid != Some(process_id) || listen_fds != Some(1) {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_service_socket_activation_invalid",
        });
    }
    Ok(())
}

pub(super) fn fixed_state_root() -> PathBuf {
    PathBuf::from(LEASE_AUTHORITY_SERVICE_STATE_ROOT)
}

pub(super) fn fixed_socket_path() -> PathBuf {
    PathBuf::from(LEASE_AUTHORITY_SERVICE_SOCKET_PATH)
}

pub(super) fn run_linux_service() -> Result<(), LeaseAuthorityProtocolError> {
    let process_id = std::process::id();
    let effective_uid = unsafe { libc::geteuid() };
    let current_executable = std::fs::canonicalize(format!("/proc/{process_id}/exe"))
        .map_err(|_| service_error("lease_authority_service_process_identity_invalid"))?;
    validate_linux_service_launch(effective_uid, process_id, &current_executable)?;

    let listen_pid = std::env::var("LISTEN_PID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let listen_fds = std::env::var("LISTEN_FDS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    validate_systemd_socket_activation(listen_pid, listen_fds, process_id)?;

    let state_root = fixed_state_root();
    super::super::ensure_private_directory(&state_root)
        .map_err(|_| service_error("lease_authority_service_state_root_unprotected"))?;
    let config = load_service_config(&state_root)?;
    let socket_path = fixed_socket_path();
    let listener = unsafe { UnixListener::from_raw_fd(LEASE_AUTHORITY_SYSTEMD_LISTEN_FD) };
    let local_path = listener
        .local_addr()
        .ok()
        .and_then(|address| address.as_pathname().map(Path::to_path_buf));
    if local_path.as_deref() != Some(socket_path.as_path()) {
        return Err(service_error(
            "lease_authority_service_socket_activation_invalid",
        ));
    }
    let custody = custody::inspect_linux_authority_service_identity(
        &state_root,
        &socket_path,
        config.operator_group_id,
    )
    .map_err(|_| service_error("lease_authority_service_custody_invalid"))?;
    let store_root = state_root.join(LEASE_AUTHORITY_SERVICE_STORE_DIRECTORY);
    let trust_root = state_root.join(LEASE_AUTHORITY_SERVICE_TRUST_DIRECTORY);
    let store = LeaseAuthorityDurableStore::open_existing(&store_root)?;

    for accepted in listener.incoming() {
        let mut stream = accepted
            .map_err(|_| service_error("lease_authority_service_connection_accept_failed"))?;
        stream
            .set_read_timeout(Some(LEASE_AUTHORITY_CONNECTION_TIMEOUT))
            .and_then(|_| stream.set_write_timeout(Some(LEASE_AUTHORITY_CONNECTION_TIMEOUT)))
            .map_err(|_| service_error("lease_authority_service_connection_timeout_failed"))?;
        let peer = custody::inspect_linux_request_peer(&stream)
            .map_err(|_| service_error("lease_authority_service_peer_identity_invalid"))?;
        let signing_key =
            super::super::load_selected_lease_authority_signing_key_in(&trust_root)
                .map_err(|_| service_error("lease_authority_service_trust_unavailable"))?;
        let mut kernel = store.load(LeaseAuthorityProtectedLoadContext {
            expected_authority_domain_id: &config.authority_domain_id,
            minimum_authority_epoch: config.minimum_authority_epoch,
        })?;
        let mut writer = stream
            .try_clone()
            .map_err(|_| service_error("lease_authority_service_connection_clone_failed"))?;
        serve_lease_authority_connection(
            &mut kernel,
            &mut stream,
            &mut writer,
            &custody,
            peer,
            &signing_key,
        )?;
    }
    Ok(())
}

pub(super) fn run_linux_bootstrap() -> Result<(), LeaseAuthorityProtocolError> {
    let process_id = std::process::id();
    let effective_uid = unsafe { libc::geteuid() };
    let current_executable = std::fs::canonicalize(format!("/proc/{process_id}/exe"))
        .map_err(|_| service_error("lease_authority_bootstrap_process_identity_invalid"))?;
    validate_linux_bootstrap_launch(effective_uid, process_id, &current_executable)?;
    let operator_group_id = std::env::var(LEASE_AUTHORITY_BOOTSTRAP_GROUP_ID_ENV)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| service_error("lease_authority_bootstrap_operator_group_invalid"))?;
    bootstrap_state_root(&fixed_state_root(), operator_group_id).map(|_| ())
}

fn validate_linux_bootstrap_launch(
    effective_uid: u32,
    process_id: u32,
    current_executable: &Path,
) -> Result<(), LeaseAuthorityProtocolError> {
    validate_linux_service_launch(effective_uid, process_id, current_executable).map_err(|error| {
        match error.code {
            "lease_authority_service_root_required" => {
                service_error("lease_authority_bootstrap_root_required")
            }
            "lease_authority_service_process_identity_invalid" => {
                service_error("lease_authority_bootstrap_process_identity_invalid")
            }
            _ => service_error("lease_authority_bootstrap_executable_untrusted"),
        }
    })
}

fn bootstrap_state_root(
    state_root: &Path,
    operator_group_id: u32,
) -> Result<LeaseAuthorityBootstrapReceipt, LeaseAuthorityProtocolError> {
    if operator_group_id == 0 {
        return Err(service_error(
            "lease_authority_bootstrap_operator_group_invalid",
        ));
    }
    if std::fs::symlink_metadata(state_root).is_ok() {
        return Err(service_error("lease_authority_bootstrap_state_exists"));
    }
    let parent = state_root
        .parent()
        .ok_or_else(|| service_error("lease_authority_bootstrap_state_root_invalid"))?;
    let parent_metadata = std::fs::symlink_metadata(parent)
        .map_err(|_| service_error("lease_authority_bootstrap_parent_unavailable"))?;
    if !parent_metadata.file_type().is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(service_error(
            "lease_authority_bootstrap_parent_unprotected",
        ));
    }

    let state_name = state_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| service_error("lease_authority_bootstrap_state_root_invalid"))?;
    let staging = parent.join(format!(".{state_name}.{}.tmp", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging)
        .map_err(|_| service_error("lease_authority_bootstrap_stage_failed"))?;
    super::super::set_private_directory_permissions(&staging)
        .map_err(|_| service_error("lease_authority_bootstrap_stage_failed"))?;

    let initialized = (|| {
        let mut domain_seed = [0u8; 32];
        let mut boot_seed = [0u8; 32];
        let mut private_key = [0u8; 32];
        let mut administrator_capability = LeaseAuthorityBootstrapSecret([0u8; 32]);
        getrandom::getrandom(&mut domain_seed)
            .and_then(|_| getrandom::getrandom(&mut boot_seed))
            .and_then(|_| getrandom::getrandom(&mut private_key))
            .and_then(|_| getrandom::getrandom(administrator_capability.expose_mut()))
            .map_err(|_| service_error("lease_authority_bootstrap_entropy_unavailable"))?;
        let authority_domain_id = format!("sha256:{}", hex::encode(domain_seed));
        let boot_epoch = format!("sha256:{}", hex::encode(boot_seed));
        let authority_epoch = 1;

        let trust_root = staging.join(LEASE_AUTHORITY_SERVICE_TRUST_DIRECTORY);
        std::fs::create_dir(&trust_root)
            .map_err(|_| service_error("lease_authority_bootstrap_trust_failed"))?;
        super::super::set_private_directory_permissions(&trust_root)
            .map_err(|_| service_error("lease_authority_bootstrap_trust_failed"))?;
        let signing_key = super::super::LeaseAuthoritySigningKey::from_private_bytes(private_key);
        super::super::persist_lease_authority_trust_generation_in(
            &trust_root,
            &signing_key,
            &super::super::LeaseAuthorityVerificationKeyring::from_active(&signing_key),
        )
        .map_err(|_| service_error("lease_authority_bootstrap_trust_failed"))?;

        let administrator_root = staging.join(LEASE_AUTHORITY_ADMINISTRATOR_DIRECTORY);
        std::fs::create_dir(&administrator_root)
            .map_err(|_| service_error("lease_authority_bootstrap_administrator_failed"))?;
        super::super::set_private_directory_permissions(&administrator_root)
            .map_err(|_| service_error("lease_authority_bootstrap_administrator_failed"))?;
        let administrator_document = LeaseAuthorityAdministratorCapabilityFile {
            schema_version: LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_SCHEMA_VERSION.to_string(),
            administrator_id: LEASE_AUTHORITY_BOOTSTRAP_ADMINISTRATOR_ID.to_string(),
            administrator_revision: 1,
            capability_hex: hex::encode(administrator_capability.expose()),
        };
        super::super::write_private_json_atomic_replace(
            &administrator_root.join(LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_FILE),
            &administrator_document,
        )
        .map_err(|_| service_error("lease_authority_bootstrap_administrator_failed"))?;

        let store = LeaseAuthorityDurableStore::initialize(
            &staging.join(LEASE_AUTHORITY_SERVICE_STORE_DIRECTORY),
        )?;
        let mut authority = super::super::LeaseAuthorityState::default();
        authority
            .bootstrap_administrator(
                LEASE_AUTHORITY_BOOTSTRAP_ADMINISTRATOR_ID,
                administrator_capability.expose(),
            )
            .map_err(|_| service_error("lease_authority_bootstrap_administrator_failed"))?;
        let kernel = super::LeaseAuthorityProtocolKernel::bootstrap(
            &authority_domain_id,
            authority_epoch,
            &boot_epoch,
            authority,
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        )?;
        store.publish(&kernel, None)?;

        let config = LeaseAuthorityServiceConfig {
            schema_version: LEASE_AUTHORITY_SERVICE_CONFIG_SCHEMA_VERSION.to_string(),
            authority_domain_id: authority_domain_id.clone(),
            minimum_authority_epoch: authority_epoch,
            operator_group_id,
            administrator_id: LEASE_AUTHORITY_BOOTSTRAP_ADMINISTRATOR_ID.to_string(),
            administrator_revision: 1,
        };
        super::super::write_private_json_atomic_replace(
            &staging.join(LEASE_AUTHORITY_SERVICE_CONFIG_FILE),
            &config,
        )
        .map_err(|_| service_error("lease_authority_bootstrap_config_failed"))?;
        super::super::sync_authority_key_directory(&staging)
            .map_err(|_| service_error("lease_authority_bootstrap_sync_failed"))?;
        std::fs::rename(&staging, state_root)
            .map_err(|_| service_error("lease_authority_bootstrap_publish_failed"))?;
        super::super::sync_authority_key_directory(parent)
            .map_err(|_| service_error("lease_authority_bootstrap_sync_failed"))?;
        Ok(LeaseAuthorityBootstrapReceipt {
            authority_domain_id,
            authority_epoch,
            administrator_id: LEASE_AUTHORITY_BOOTSTRAP_ADMINISTRATOR_ID.to_string(),
            administrator_revision: 1,
        })
    })();
    if initialized.is_err() && staging.exists() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    initialized
}

fn load_service_config(
    state_root: &Path,
) -> Result<LeaseAuthorityServiceConfig, LeaseAuthorityProtocolError> {
    let config_path = state_root.join(LEASE_AUTHORITY_SERVICE_CONFIG_FILE);
    let metadata = std::fs::symlink_metadata(&config_path)
        .map_err(|_| service_error("lease_authority_service_config_unavailable"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(service_error("lease_authority_service_config_unprotected"));
    }
    super::super::ensure_private_file_permissions(&config_path, &metadata)
        .map_err(|_| service_error("lease_authority_service_config_unprotected"))?;
    let config: LeaseAuthorityServiceConfig = super::super::load_private_json_file(
        &config_path,
        "lease_authority_service_config_decode_failed",
    )
    .map_err(|_| service_error("lease_authority_service_config_invalid"))?;
    if config.schema_version != LEASE_AUTHORITY_SERVICE_CONFIG_SCHEMA_VERSION
        || !super::valid_sha256_digest(&config.authority_domain_id)
        || config.minimum_authority_epoch == 0
        || config.operator_group_id == 0
        || config.administrator_id.trim().is_empty()
        || config.administrator_revision == 0
    {
        return Err(service_error("lease_authority_service_config_invalid"));
    }
    Ok(config)
}

#[cfg(test)]
fn load_administrator_capability(
    state_root: &Path,
    config: &LeaseAuthorityServiceConfig,
    kernel: &super::LeaseAuthorityProtocolKernel,
) -> Result<Vec<u8>, LeaseAuthorityProtocolError> {
    let path = state_root
        .join(LEASE_AUTHORITY_ADMINISTRATOR_DIRECTORY)
        .join(LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_FILE);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|_| service_error("lease_authority_administrator_identity_unavailable"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(service_error(
            "lease_authority_administrator_identity_unprotected",
        ));
    }
    super::super::ensure_private_file_permissions(&path, &metadata)
        .map_err(|_| service_error("lease_authority_administrator_identity_unprotected"))?;
    let document: LeaseAuthorityAdministratorCapabilityFile = super::super::load_private_json_file(
        &path,
        "lease_authority_administrator_identity_decode_failed",
    )
    .map_err(|_| service_error("lease_authority_administrator_identity_invalid"))?;
    let mut capability = hex::decode(&document.capability_hex)
        .map_err(|_| service_error("lease_authority_administrator_identity_invalid"))?;
    if document.schema_version != LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_SCHEMA_VERSION
        || document.administrator_id != config.administrator_id
        || document.administrator_revision != config.administrator_revision
        || capability.len() < 32
    {
        capability.fill(0);
        return Err(service_error(
            "lease_authority_administrator_identity_invalid",
        ));
    }
    kernel.validate_administrator_capability(
        &document.administrator_id,
        document.administrator_revision,
        &capability,
    )?;
    Ok(capability)
}

fn service_error(code: &'static str) -> LeaseAuthorityProtocolError {
    LeaseAuthorityProtocolError { code }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_parent(label: &str) -> PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "agent-browser-lease-authority-bootstrap-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&parent).unwrap();
        parent
    }

    #[test]
    fn bootstrap_is_atomic_single_use_and_loadable() {
        let parent = temp_parent("single-use");
        let state_root = parent.join("lease-authority");
        let receipt = bootstrap_state_root(&state_root, 991).unwrap();
        assert_eq!(receipt.authority_epoch, 1);
        assert!(super::super::valid_sha256_digest(
            &receipt.authority_domain_id
        ));
        let config = load_service_config(&state_root).unwrap();
        assert_eq!(config.authority_domain_id, receipt.authority_domain_id);
        assert_eq!(config.minimum_authority_epoch, 1);
        assert_eq!(config.operator_group_id, 991);
        assert_eq!(config.administrator_id, "administrator:local-root");
        assert_eq!(config.administrator_revision, 1);
        super::super::super::load_selected_lease_authority_signing_key_in(
            &state_root.join(LEASE_AUTHORITY_SERVICE_TRUST_DIRECTORY),
        )
        .unwrap();
        let kernel = LeaseAuthorityDurableStore::open_existing(
            &state_root.join(LEASE_AUTHORITY_SERVICE_STORE_DIRECTORY),
        )
        .unwrap()
        .load(LeaseAuthorityProtectedLoadContext {
            expected_authority_domain_id: &receipt.authority_domain_id,
            minimum_authority_epoch: 1,
        })
        .unwrap();
        let administrator_capability =
            load_administrator_capability(&state_root, &config, &kernel).unwrap();
        assert_eq!(administrator_capability.len(), 32);
        let administrator_document: LeaseAuthorityAdministratorCapabilityFile =
            super::super::super::load_private_json_file(
                &state_root
                    .join(LEASE_AUTHORITY_ADMINISTRATOR_DIRECTORY)
                    .join(LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_FILE),
                "fixture_decode_failed",
            )
            .unwrap();
        let debug = format!("{administrator_document:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(&administrator_document.capability_hex));

        let mismatched_document = LeaseAuthorityAdministratorCapabilityFile {
            capability_hex: hex::encode([0x5a; 32]),
            ..administrator_document
        };
        super::super::super::write_private_json_atomic_replace(
            &state_root
                .join(LEASE_AUTHORITY_ADMINISTRATOR_DIRECTORY)
                .join(LEASE_AUTHORITY_ADMINISTRATOR_CAPABILITY_FILE),
            &mismatched_document,
        )
        .unwrap();
        let error = load_administrator_capability(&state_root, &config, &kernel).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_protocol_administrator_identity_invalid"
        );

        let error = bootstrap_state_root(&state_root, 991).unwrap_err();
        assert_eq!(error.code(), "lease_authority_bootstrap_state_exists");
        let _ = std::fs::remove_dir_all(parent);
    }

    #[test]
    fn bootstrap_rejects_zero_group_without_creating_state() {
        let parent = temp_parent("group");
        let state_root = parent.join("lease-authority");
        let error = bootstrap_state_root(&state_root, 0).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_bootstrap_operator_group_invalid"
        );
        assert!(!state_root.exists());
        let _ = std::fs::remove_dir_all(parent);
    }

    #[test]
    fn bootstrap_requires_root_and_a_banked_executable() {
        let banked = Path::new(
            "/usr/local/libexec/agent-browser/lease-authority/generations/generation-1/agent-browser",
        );
        let error = validate_linux_bootstrap_launch(1000, 4100, banked).unwrap_err();
        assert_eq!(error.code(), "lease_authority_bootstrap_root_required");

        let error = validate_linux_bootstrap_launch(
            0,
            4100,
            Path::new("/home/operator/candidate/agent-browser"),
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_bootstrap_executable_untrusted"
        );
    }
}
