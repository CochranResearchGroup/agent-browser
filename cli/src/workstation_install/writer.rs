//! Exact process custody for workstation writers. PID text remains readable by
//! older binaries; a separate record binds the process instance and operation.
//! Physical locks cover claim read/commit only, never process convergence.

use crate::process_identity::{
    assess_process_ownership, capture_process_identity, observe_process, LegacyProfileProof,
    ProcessObservation, RecordedProcessIdentity, RuntimeProcessOwnership,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriterClaim {
    operation_id: String,
    process_identity: RecordedProcessIdentity,
    pid_file_identity: Option<(u64, u64)>,
}

pub(super) struct WorkstationLock {
    path: PathBuf,
    operation_id: String,
}

fn guard(path: &Path) -> Result<File, String> {
    fs::create_dir_all(path.parent().expect("writer path has a parent"))
        .map_err(|error| format!("workstation_writer_directory_failed:{error}"))?;
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("guard"))
        .map_err(|error| format!("workstation_writer_guard_failed:{error}"))
}

fn claim_path(path: &Path) -> PathBuf {
    path.with_extension("owner.json")
}

fn claim(path: &Path) -> Option<WriterClaim> {
    fs::read(claim_path(path))
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
}

#[cfg(unix)]
fn pid_file_identity(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path)
        .ok()
        .map(|metadata| (metadata.dev(), metadata.ino()))
}

// Workstation effects are Linux-only; keep the module buildable elsewhere.
#[cfg(not(unix))]
fn pid_file_identity(_path: &Path) -> Option<(u64, u64)> {
    None
}

impl WorkstationLock {
    pub(super) fn acquire(root: &Path) -> Result<Self, String> {
        let path = root.join(".agent-browser/convergence/workstation.lock");
        let guard = guard(&path)?;
        match guard.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(
                    "workstation reconciliation is already active: writer claim is being committed"
                        .into(),
                )
            }
            Err(error) => return Err(format!("workstation_writer_lock_failed:{error}")),
        }
        if owner_pid(&path)?.is_some() {
            return Err("workstation reconciliation is already active: an exact writer process is executing".into());
        }
        let identity = capture_process_identity(std::process::id(), None, None)
            .ok_or("workstation_writer_process_identity_unavailable")?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("workstation_writer_stale_record_failed:{error}")),
        }
        let mut pid = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("workstation_writer_publish_failed:{error}"))?;
        writeln!(pid, "{}", identity.pid)
            .map_err(|error| format!("workstation_writer_pid_failed:{error}"))?;
        pid.sync_all()
            .map_err(|error| format!("workstation_writer_sync_failed:{error}"))?;
        let operation_id = uuid::Uuid::new_v4().to_string();
        super::write_private_json_atomic(
            &claim_path(&path),
            &WriterClaim {
                operation_id: operation_id.clone(),
                process_identity: identity,
                pid_file_identity: pid_file_identity(&path),
            },
        )?;
        Ok(Self { path, operation_id })
    }
}

pub(crate) fn workstation_transaction_is_foreign(
    path: &Path,
    current_pid: u32,
) -> Result<bool, String> {
    let guard = guard(path)?;
    match guard.try_lock_shared() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(true),
        Err(error) => {
            return Err(format!(
                "workstation_writer_lock_observation_failed:{error}"
            ))
        }
    }
    Ok(owner_pid(path)?.is_some_and(|pid| pid != current_pid))
}

fn owner_pid(path: &Path) -> Result<Option<u32>, String> {
    let raw = match fs::read(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("workstation_writer_read_failed:{error}")),
    };
    let Some(pid) = std::str::from_utf8(&raw)
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
    else {
        return Ok(None);
    };
    if let Some(claim) = claim(path).filter(|claim| {
        claim.process_identity.pid == pid && claim.pid_file_identity == pid_file_identity(path)
    }) {
        let observed = observe_process(pid);
        if let ProcessObservation::Failed { reason } = &observed {
            return Err(format!("workstation_writer_identity_unavailable:{reason}"));
        }
        let assessment = assess_process_ownership(
            Some(&claim.process_identity),
            observed,
            LegacyProfileProof::Unproven,
        );
        return Ok(
            (assessment.ownership == RuntimeProcessOwnership::MatchingBrowser).then_some(pid),
        );
    }
    // Older writers have PID text only. Require an executing installer mutation,
    // not an unrelated process that inherited its PID or an old inode's claim.
    let ProcessObservation::Observed(observed) = observe_process(pid) else {
        return Ok(None);
    };
    let executable = observed
        .executable_path
        .as_deref()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str());
    let arguments = observed.command_line.unwrap_or_default();
    let mutation = arguments.iter().any(|arg| arg == "--apply")
        || arguments.windows(2).any(|pair| {
            (pair[0] == "transactions"
                && matches!(pair[1].as_str(), "resume" | "rollback" | "close"))
                || (pair[0] == "workstation"
                    && matches!(
                        pair[1].as_str(),
                        "reconcile" | "recover" | "finalize" | "retain-generation"
                    ))
        });
    Ok((executable == Some("agent-browser")
        && arguments.iter().any(|arg| arg == "install")
        && mutation)
        .then_some(pid))
}

impl Drop for WorkstationLock {
    fn drop(&mut self) {
        let Ok(guard) = guard(&self.path) else { return };
        if guard.try_lock().is_err() {
            return;
        }
        if claim(&self.path).is_some_and(|claim| {
            claim.operation_id == self.operation_id
                && claim.pid_file_identity == pid_file_identity(&self.path)
        }) {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_file(claim_path(&self.path));
        }
        // The physical guard inode stays stable even when PID diagnostics are
        // removed for compatibility with older installer release semantics.
    }
}
