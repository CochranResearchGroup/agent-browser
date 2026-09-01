use serde::Deserialize;
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
const LEASE_AUTHORITY_SERVICE_STORE_DIRECTORY: &str = "store";
const LEASE_AUTHORITY_SERVICE_TRUST_DIRECTORY: &str = "trust";
const LEASE_AUTHORITY_SYSTEMD_LISTEN_FD: i32 = 3;
const LEASE_AUTHORITY_CONNECTION_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityServiceConfig {
    schema_version: String,
    authority_domain_id: String,
    minimum_authority_epoch: u64,
    operator_group_id: u32,
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
            &signing_key,
        )?;
    }
    Ok(())
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
    {
        return Err(service_error("lease_authority_service_config_invalid"));
    }
    Ok(config)
}

fn service_error(code: &'static str) -> LeaseAuthorityProtocolError {
    LeaseAuthorityProtocolError { code }
}
