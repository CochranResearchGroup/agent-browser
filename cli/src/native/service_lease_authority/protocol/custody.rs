use sha2::{Digest, Sha256};

const LEASE_AUTHORITY_ROOT_UID: u32 = 0;
const LEASE_AUTHORITY_SYSTEMD_SERVICE_CGROUP: &str =
    "/system.slice/agent-browser-lease-authority.service";

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeaseAuthorityHostRootProjection {
    observed_root_uid: u32,
}

#[cfg(target_os = "linux")]
impl LeaseAuthorityHostRootProjection {
    fn normalize_owner_uid(self, observed_uid: u32) -> Result<u32, LeaseAuthorityCustodyError> {
        (observed_uid == self.observed_root_uid)
            .then_some(LEASE_AUTHORITY_ROOT_UID)
            .ok_or(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_service_identity_unprotected",
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LeaseAuthorityCustodySnapshot {
    service_uid: u32,
    service_pid: u32,
    peer_uid: u32,
    peer_pid: u32,
    state_root: LeaseAuthorityCustodyPath,
    executable_owner_uid: u32,
    executable_owner_gid: u32,
    executable_mode: u32,
    executable_sha256: String,
    socket_owner_uid: u32,
    socket_owner_gid: u32,
    socket_mode: u32,
    socket_device: u64,
    socket_inode: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseAuthorityCustodyPath {
    owner_uid: u32,
    mode: u32,
    is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LeaseAuthorityCustodyIdentity {
    pub(super) endpoint_identity_digest: String,
    pub(super) executable_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LeaseAuthorityRequestPeerIdentity {
    pub(super) uid: u32,
    pub(super) gid: u32,
    pub(super) pid: u32,
}

impl LeaseAuthorityRequestPeerIdentity {
    pub(super) fn is_root_administrator(self) -> bool {
        self.uid == LEASE_AUTHORITY_ROOT_UID && self.pid > 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LeaseAuthorityCustodyError {
    code: &'static str,
}

impl LeaseAuthorityCustodyError {
    pub(super) fn code(&self) -> &'static str {
        self.code
    }
}

impl LeaseAuthorityCustodySnapshot {
    pub(super) fn validate(
        &self,
        expected_group_id: u32,
    ) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
        self.validate_for_peer(expected_group_id, false)
    }

    pub(super) fn validate_systemd_socket_activated_endpoint(
        &self,
        expected_group_id: u32,
    ) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
        self.validate_for_peer(expected_group_id, true)
    }

    pub(super) fn validate_endpoint(
        &self,
        expected_group_id: u32,
    ) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
        if self.service_pid == 1 && self.peer_pid == 1 {
            self.validate_systemd_socket_activated_endpoint(expected_group_id)
        } else {
            self.validate(expected_group_id)
        }
    }

    fn validate_for_peer(
        &self,
        expected_group_id: u32,
        systemd_socket_activated_endpoint: bool,
    ) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
        if self.service_uid != LEASE_AUTHORITY_ROOT_UID || self.peer_uid != LEASE_AUTHORITY_ROOT_UID
        {
            return Err(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_service_identity_unprotected",
            });
        }
        let peer_identity_matches = if systemd_socket_activated_endpoint {
            self.service_pid == 1 && self.peer_pid == 1
        } else {
            self.service_pid > 1 && self.peer_pid == self.service_pid
        };
        if !peer_identity_matches {
            return Err(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_peer_identity_mismatch",
            });
        }
        if self.state_root.owner_uid != LEASE_AUTHORITY_ROOT_UID
            || self.state_root.mode != 0o700
            || !self.state_root.is_directory
        {
            return Err(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_state_root_unprotected",
            });
        }
        if self.executable_owner_uid != LEASE_AUTHORITY_ROOT_UID
            || self.executable_owner_gid != LEASE_AUTHORITY_ROOT_UID
            || self.executable_mode & 0o022 != 0
            || self.executable_mode & 0o111 == 0
            || !valid_sha256_identity(&self.executable_sha256)
        {
            return Err(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_executable_unprotected",
            });
        }
        if self.socket_device == 0 || self.socket_inode == 0 {
            return Err(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_socket_identity_invalid",
            });
        }
        if self.socket_owner_uid != LEASE_AUTHORITY_ROOT_UID
            || self.socket_owner_gid != expected_group_id
            || self.socket_mode != 0o660
        {
            return Err(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_socket_unprotected",
            });
        }
        let mut hasher = Sha256::new();
        hasher.update(b"agent-browser.lease-authority-endpoint-identity.v1\0");
        hasher.update(self.service_uid.to_be_bytes());
        hasher.update(self.service_pid.to_be_bytes());
        hasher.update(self.peer_uid.to_be_bytes());
        hasher.update(self.peer_pid.to_be_bytes());
        hasher.update(expected_group_id.to_be_bytes());
        hasher.update(self.executable_sha256.as_bytes());
        hasher.update(self.socket_owner_uid.to_be_bytes());
        hasher.update(self.socket_owner_gid.to_be_bytes());
        hasher.update(self.socket_mode.to_be_bytes());
        hasher.update(self.socket_device.to_be_bytes());
        hasher.update(self.socket_inode.to_be_bytes());
        Ok(LeaseAuthorityCustodyIdentity {
            endpoint_identity_digest: format!("sha256:{:x}", hasher.finalize()),
            executable_sha256: self.executable_sha256.clone(),
        })
    }
}

#[cfg(test)]
impl LeaseAuthorityCustodySnapshot {
    pub(super) fn root_owned_fixture() -> Self {
        Self {
            service_uid: 0,
            service_pid: 4100,
            peer_uid: 0,
            peer_pid: 4100,
            state_root: LeaseAuthorityCustodyPath {
                owner_uid: 0,
                mode: 0o700,
                is_directory: true,
            },
            executable_owner_uid: 0,
            executable_owner_gid: 0,
            executable_mode: 0o755,
            executable_sha256:
                "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                    .to_string(),
            socket_owner_uid: 0,
            socket_owner_gid: 991,
            socket_mode: 0o660,
            socket_device: 41,
            socket_inode: 73,
        }
    }

    pub(super) fn with_replaced_socket(mut self) -> Self {
        self.socket_inode += 1;
        self
    }
}

#[cfg(target_os = "linux")]
pub(super) fn inspect_linux_authority_endpoint(
    state_root: &std::path::Path,
    socket_path: &std::path::Path,
    stream: &std::os::unix::net::UnixStream,
    expected_group_id: u32,
) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
    // A systemd-created listening socket reports PID 1 as the connected peer
    // before the socket-activated service accepts the connection. Prove that
    // exact root activator and the protected socket inode here. The service
    // separately proves its own root process and banked executable before it
    // reads any request frame.
    let peer = inspect_linux_request_peer(stream)?;
    let root_projection = inspect_linux_host_root_projection(peer.uid)?;

    let snapshot = if peer.pid == 1 {
        inspect_linux_systemd_socket_activator_snapshot(
            state_root,
            socket_path,
            peer.pid,
            root_projection,
        )
    } else {
        inspect_linux_systemd_service_snapshot(state_root, socket_path, peer.pid, root_projection)
    }?;
    snapshot.validate_endpoint(expected_group_id)
}

#[cfg(target_os = "linux")]
fn inspect_linux_host_root_projection(
    observed_peer_uid: u32,
) -> Result<LeaseAuthorityHostRootProjection, LeaseAuthorityCustodyError> {
    let uid_map =
        std::fs::read_to_string("/proc/self/uid_map").map_err(|_| custody_inspection_error())?;
    let overflow_uid = std::fs::read_to_string("/proc/sys/kernel/overflowuid")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .ok_or_else(custody_inspection_error)?;
    host_root_projection_from_uid_map(observed_peer_uid, overflow_uid, &uid_map).ok_or(
        LeaseAuthorityCustodyError {
            code: "lease_authority_custody_service_identity_unprotected",
        },
    )
}

#[cfg(target_os = "linux")]
fn host_root_projection_from_uid_map(
    observed_peer_uid: u32,
    overflow_uid: u32,
    uid_map: &str,
) -> Option<LeaseAuthorityHostRootProjection> {
    let host_root_is_mapped = uid_map
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _inside_start = fields.next()?.parse::<u64>().ok()?;
            let outside_start = fields.next()?.parse::<u64>().ok()?;
            let length = fields.next()?.parse::<u64>().ok()?;
            Some(outside_start == 0 && length > 0)
        })
        .any(|mapped| mapped);
    let observed_root_uid = if host_root_is_mapped {
        (observed_peer_uid == LEASE_AUTHORITY_ROOT_UID).then_some(LEASE_AUTHORITY_ROOT_UID)?
    } else {
        (observed_peer_uid == overflow_uid).then_some(overflow_uid)?
    };
    Some(LeaseAuthorityHostRootProjection { observed_root_uid })
}

#[cfg(target_os = "linux")]
fn inspect_linux_systemd_service_snapshot(
    state_root: &std::path::Path,
    socket_path: &std::path::Path,
    service_pid: u32,
    root_projection: LeaseAuthorityHostRootProjection,
) -> Result<LeaseAuthorityCustodySnapshot, LeaseAuthorityCustodyError> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let cgroup = std::fs::read_to_string(format!("/proc/{service_pid}/cgroup"))
        .map_err(|_| custody_inspection_error())?;
    if !is_protected_lease_authority_service_cgroup(&cgroup) {
        return Err(LeaseAuthorityCustodyError {
            code: "lease_authority_custody_service_cgroup_unprotected",
        });
    }
    let state_metadata =
        std::fs::symlink_metadata(state_root).map_err(|_| custody_inspection_error())?;
    let socket_metadata =
        std::fs::symlink_metadata(socket_path).map_err(|_| custody_inspection_error())?;
    let service_identity = format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "agent-browser.lease-authority-systemd-service-cgroup.v1\n{service_pid}\n{LEASE_AUTHORITY_SYSTEMD_SERVICE_CGROUP}"
            )
            .as_bytes()
        )
    );

    Ok(LeaseAuthorityCustodySnapshot {
        service_uid: LEASE_AUTHORITY_ROOT_UID,
        service_pid,
        peer_uid: LEASE_AUTHORITY_ROOT_UID,
        peer_pid: service_pid,
        state_root: LeaseAuthorityCustodyPath {
            owner_uid: root_projection.normalize_owner_uid(state_metadata.uid())?,
            mode: state_metadata.mode() & 0o7777,
            is_directory: state_metadata.file_type().is_dir()
                && !state_metadata.file_type().is_symlink(),
        },
        // Hardened procfs denies ordinary clients access to a root service's
        // /proc/<pid>/exe. SO_PEERCRED and exact system-service cgroup custody
        // authenticate the connected endpoint before a request is written.
        // The service independently validates its banked executable before it
        // reads that request.
        executable_owner_uid: LEASE_AUTHORITY_ROOT_UID,
        executable_owner_gid: LEASE_AUTHORITY_ROOT_UID,
        executable_mode: 0o555,
        executable_sha256: service_identity,
        socket_owner_uid: root_projection.normalize_owner_uid(socket_metadata.uid())?,
        socket_owner_gid: socket_metadata.gid(),
        socket_mode: socket_metadata.mode() & 0o7777,
        socket_device: socket_metadata.dev(),
        socket_inode: socket_metadata.ino(),
    })
    .and_then(|snapshot| {
        socket_metadata
            .file_type()
            .is_socket()
            .then_some(snapshot)
            .ok_or(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_socket_unprotected",
            })
    })
}

#[cfg(target_os = "linux")]
fn is_protected_lease_authority_service_cgroup(cgroup: &str) -> bool {
    cgroup.lines().any(|line| {
        let mut fields = line.splitn(3, ':');
        fields.next().is_some()
            && fields.next().is_some()
            && fields.next() == Some(LEASE_AUTHORITY_SYSTEMD_SERVICE_CGROUP)
    })
}

#[cfg(target_os = "linux")]
pub(super) fn inspect_linux_request_peer(
    stream: &std::os::unix::net::UnixStream,
) -> Result<LeaseAuthorityRequestPeerIdentity, LeaseAuthorityCustodyError> {
    use std::os::fd::AsRawFd;

    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut credentials_length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut credentials_length,
        )
    };
    if result != 0
        || credentials_length as usize != std::mem::size_of::<libc::ucred>()
        || credentials.pid < 1
    {
        return Err(custody_inspection_error());
    }
    Ok(LeaseAuthorityRequestPeerIdentity {
        uid: credentials.uid,
        gid: credentials.gid,
        pid: u32::try_from(credentials.pid).map_err(|_| custody_inspection_error())?,
    })
}

#[cfg(target_os = "linux")]
pub(super) fn inspect_linux_authority_service_identity(
    state_root: &std::path::Path,
    socket_path: &std::path::Path,
    expected_group_id: u32,
) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
    let service_pid = std::process::id();
    let service_uid = unsafe { libc::geteuid() };
    inspect_linux_authority_identity(
        state_root,
        socket_path,
        service_pid,
        service_uid,
        expected_group_id,
    )
}

#[cfg(target_os = "linux")]
fn inspect_linux_authority_identity(
    state_root: &std::path::Path,
    socket_path: &std::path::Path,
    service_pid: u32,
    service_uid: u32,
    expected_group_id: u32,
) -> Result<LeaseAuthorityCustodyIdentity, LeaseAuthorityCustodyError> {
    inspect_linux_authority_identity_snapshot(state_root, socket_path, service_pid, service_uid)
        .and_then(|snapshot| snapshot.validate(expected_group_id))
}

#[cfg(target_os = "linux")]
fn inspect_linux_authority_identity_snapshot(
    state_root: &std::path::Path,
    socket_path: &std::path::Path,
    service_pid: u32,
    service_uid: u32,
) -> Result<LeaseAuthorityCustodySnapshot, LeaseAuthorityCustodyError> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let state_metadata =
        std::fs::symlink_metadata(state_root).map_err(|_| custody_inspection_error())?;
    let socket_metadata =
        std::fs::symlink_metadata(socket_path).map_err(|_| custody_inspection_error())?;
    let executable_path = std::fs::canonicalize(format!("/proc/{service_pid}/exe"))
        .map_err(|_| custody_inspection_error())?;
    let executable_metadata =
        std::fs::metadata(&executable_path).map_err(|_| custody_inspection_error())?;
    let executable = std::fs::read(&executable_path).map_err(|_| custody_inspection_error())?;
    let mut executable_hasher = Sha256::new();
    executable_hasher.update(executable);

    Ok(LeaseAuthorityCustodySnapshot {
        service_uid,
        service_pid,
        peer_uid: service_uid,
        peer_pid: service_pid,
        state_root: LeaseAuthorityCustodyPath {
            owner_uid: state_metadata.uid(),
            mode: state_metadata.mode() & 0o7777,
            is_directory: state_metadata.file_type().is_dir()
                && !state_metadata.file_type().is_symlink(),
        },
        executable_owner_uid: executable_metadata.uid(),
        executable_owner_gid: executable_metadata.gid(),
        executable_mode: executable_metadata.mode() & 0o7777,
        executable_sha256: format!("sha256:{:x}", executable_hasher.finalize()),
        socket_owner_uid: socket_metadata.uid(),
        socket_owner_gid: socket_metadata.gid(),
        socket_mode: socket_metadata.mode() & 0o7777,
        socket_device: socket_metadata.dev(),
        socket_inode: socket_metadata.ino(),
    })
    .and_then(|snapshot| {
        socket_metadata
            .file_type()
            .is_socket()
            .then_some(snapshot)
            .ok_or(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_socket_unprotected",
            })
    })
}

#[cfg(target_os = "linux")]
fn inspect_linux_systemd_socket_activator_snapshot(
    state_root: &std::path::Path,
    socket_path: &std::path::Path,
    activator_pid: u32,
    root_projection: LeaseAuthorityHostRootProjection,
) -> Result<LeaseAuthorityCustodySnapshot, LeaseAuthorityCustodyError> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let state_metadata =
        std::fs::symlink_metadata(state_root).map_err(|_| custody_inspection_error())?;
    let socket_metadata =
        std::fs::symlink_metadata(socket_path).map_err(|_| custody_inspection_error())?;
    let activator_identity = format!(
        "sha256:{:x}",
        Sha256::digest(b"agent-browser.lease-authority-systemd-socket-activator-pid1.v1")
    );

    Ok(LeaseAuthorityCustodySnapshot {
        service_uid: LEASE_AUTHORITY_ROOT_UID,
        service_pid: activator_pid,
        peer_uid: LEASE_AUTHORITY_ROOT_UID,
        peer_pid: activator_pid,
        state_root: LeaseAuthorityCustodyPath {
            owner_uid: root_projection.normalize_owner_uid(state_metadata.uid())?,
            mode: state_metadata.mode() & 0o7777,
            is_directory: state_metadata.file_type().is_dir()
                && !state_metadata.file_type().is_symlink(),
        },
        // PID 1 is authenticated by SO_PEERCRED. Hardened procfs commonly
        // denies ordinary clients access to /proc/1/exe, so endpoint custody
        // binds a domain-separated activator identity rather than pretending
        // that the client observed PID 1's executable. The activated service
        // independently validates its exact banked executable before reading.
        executable_owner_uid: LEASE_AUTHORITY_ROOT_UID,
        executable_owner_gid: LEASE_AUTHORITY_ROOT_UID,
        executable_mode: 0o555,
        executable_sha256: activator_identity,
        socket_owner_uid: root_projection.normalize_owner_uid(socket_metadata.uid())?,
        socket_owner_gid: socket_metadata.gid(),
        socket_mode: socket_metadata.mode() & 0o7777,
        socket_device: socket_metadata.dev(),
        socket_inode: socket_metadata.ino(),
    })
    .and_then(|snapshot| {
        socket_metadata
            .file_type()
            .is_socket()
            .then_some(snapshot)
            .ok_or(LeaseAuthorityCustodyError {
                code: "lease_authority_custody_socket_unprotected",
            })
    })
}

#[cfg(target_os = "linux")]
fn custody_inspection_error() -> LeaseAuthorityCustodyError {
    LeaseAuthorityCustodyError {
        code: "lease_authority_custody_inspection_failed",
    }
}

fn valid_sha256_identity(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_user_service_cannot_claim_protected_authority_custody() {
        let mut snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        snapshot.service_uid = 1000;
        snapshot.peer_uid = 1000;

        let error = snapshot.validate(991).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_custody_service_identity_unprotected"
        );
    }

    #[test]
    fn user_writable_state_root_cannot_hold_operational_authority() {
        let mut snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        snapshot.state_root.owner_uid = 1000;
        snapshot.state_root.mode = 0o700;

        let error = snapshot.validate(991).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_custody_state_root_unprotected"
        );
    }

    #[test]
    fn protected_endpoint_identity_is_bound_to_the_exact_socket_instance() {
        let snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        let first = snapshot.validate(991).unwrap();
        let mut replaced = snapshot;
        replaced.socket_inode += 1;
        let second = replaced.validate(991).unwrap();

        assert!(valid_sha256_identity(&first.endpoint_identity_digest));
        assert_ne!(
            first.endpoint_identity_digest,
            second.endpoint_identity_digest
        );
    }

    #[test]
    fn candidate_owned_socket_cannot_impersonate_the_authority_endpoint() {
        let mut snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        snapshot.socket_owner_uid = 1000;

        let error = snapshot.validate(991).unwrap_err();
        assert_eq!(error.code(), "lease_authority_custody_socket_unprotected");
    }

    #[test]
    fn candidate_writable_executable_cannot_be_the_stable_authority() {
        let mut snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        snapshot.executable_mode = 0o775;

        let error = snapshot.validate(991).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_custody_executable_unprotected"
        );
    }

    #[test]
    fn socket_peer_must_be_the_exact_root_authority_process() {
        let mut snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        snapshot.peer_pid = snapshot.service_pid + 1;

        let error = snapshot.validate(991).unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_custody_peer_identity_mismatch"
        );
    }

    #[test]
    fn systemd_socket_activator_is_valid_endpoint_custody_but_not_service_custody() {
        let mut snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        snapshot.service_pid = 1;
        snapshot.peer_pid = 1;

        let endpoint = snapshot
            .validate_systemd_socket_activated_endpoint(991)
            .unwrap();
        assert!(valid_sha256_identity(&endpoint.endpoint_identity_digest));
        assert_eq!(
            snapshot.validate(991).unwrap_err().code(),
            "lease_authority_custody_peer_identity_mismatch"
        );
    }

    #[test]
    fn endpoint_custody_accepts_both_socket_activator_and_active_service_states() {
        let active_service = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        active_service.validate_endpoint(991).unwrap();

        let mut socket_activator = LeaseAuthorityCustodySnapshot::root_owned_fixture();
        socket_activator.service_pid = 1;
        socket_activator.peer_pid = 1;
        socket_activator.validate_endpoint(991).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn active_service_cgroup_requires_the_exact_protected_system_unit() {
        assert!(is_protected_lease_authority_service_cgroup(
            "0::/system.slice/agent-browser-lease-authority.service\n"
        ));
        assert!(is_protected_lease_authority_service_cgroup(
            "11:memory:/system.slice/agent-browser-lease-authority.service\n"
        ));
        assert!(!is_protected_lease_authority_service_cgroup(
            "0::/user.slice/user-1000.slice/agent-browser-lease-authority.service\n"
        ));
        assert!(!is_protected_lease_authority_service_cgroup(
            "0::/system.slice/agent-browser-lease-authority.service-impostor\n"
        ));
        assert!(!is_protected_lease_authority_service_cgroup(
            "0::/system.slice/agent-browser-lease-authority.socket\n"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn host_root_projection_accepts_direct_and_unmapped_kernel_identities_only() {
        let direct =
            host_root_projection_from_uid_map(0, 65_534, "         0          0 4294967295\n")
                .unwrap();
        assert_eq!(direct.observed_root_uid, 0);
        assert_eq!(direct.normalize_owner_uid(0).unwrap(), 0);
        assert!(direct.normalize_owner_uid(1000).is_err());

        let unmapped =
            host_root_projection_from_uid_map(65_534, 65_534, "      1000       1000          1\n")
                .unwrap();
        assert_eq!(unmapped.observed_root_uid, 65_534);
        assert_eq!(unmapped.normalize_owner_uid(65_534).unwrap(), 0);
        assert!(unmapped.normalize_owner_uid(1000).is_err());

        assert!(host_root_projection_from_uid_map(
            1000,
            65_534,
            "      1000       1000          1\n"
        )
        .is_none());
        assert!(
            host_root_projection_from_uid_map(0, 65_534, "      1000       1000          1\n")
                .is_none()
        );
        assert!(host_root_projection_from_uid_map(
            65_534,
            65_534,
            "         0          0 4294967295\n"
        )
        .is_none());
    }

    #[test]
    fn socket_activated_endpoint_rejects_a_root_process_other_than_pid_one() {
        let snapshot = LeaseAuthorityCustodySnapshot::root_owned_fixture();

        assert_eq!(
            snapshot
                .validate_systemd_socket_activated_endpoint(991)
                .unwrap_err()
                .code(),
            "lease_authority_custody_peer_identity_mismatch"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_request_peer_identity_comes_from_the_connected_socket() {
        let (client, server) = std::os::unix::net::UnixStream::pair().unwrap();
        let peer = inspect_linux_request_peer(&server).unwrap();
        assert_eq!(peer.uid, unsafe { libc::geteuid() });
        assert_eq!(peer.gid, unsafe { libc::getegid() });
        assert_eq!(peer.pid, std::process::id());
        assert_eq!(peer.is_root_administrator(), peer.uid == 0);
        drop(client);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_endpoint_inspection_rejects_a_user_owned_impostor() {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::net::{UnixListener, UnixStream};

        let root = std::env::temp_dir().join(format!(
            "agent-browser-lease-custody-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket_path = root.join("authority.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o660)).unwrap();
        let client = UnixStream::connect(&socket_path).unwrap();
        let (_server, _) = listener.accept().unwrap();

        let error = inspect_linux_authority_endpoint(&root, &socket_path, &client, unsafe {
            libc::getegid()
        })
        .unwrap_err();
        assert_eq!(
            error.code(),
            "lease_authority_custody_service_identity_unprotected"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
