//! Process-instance custody for workstation writers. The file is a durable
//! claim, not an admission marker. Physical locks cover only claim read/commit.

use crate::process_identity::{
    assess_process_ownership, capture_process_identity, observe_process, LegacyProfileProof,
    ProcessObservation, RecordedProcessIdentity, RuntimeProcessOwnership,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriterClaim {
    operation_id: String,
    process_identity: RecordedProcessIdentity,
}

pub(super) struct WorkstationLock {
    path: PathBuf,
    operation_id: String,
}

impl WorkstationLock {
    pub(super) fn acquire(root: &Path) -> Result<Self, String> {
        let path = root.join(".agent-browser/convergence/workstation.lock");
        fs::create_dir_all(path.parent().expect("writer path has a parent"))
            .map_err(|error| format!("workstation_writer_directory_failed:{error}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("workstation_writer_open_failed:{error}"))?;
        file.try_lock()
            .map_err(|error| format!("workstation reconciliation is already active: {error}"))?;
        if owner_pid(&mut file)?.is_some() {
            return Err("workstation reconciliation is already active: an exact writer process is executing".into());
        }
        let identity = capture_process_identity(std::process::id(), None, None)
            .ok_or("workstation_writer_process_identity_unavailable")?;
        let operation_id = uuid::Uuid::new_v4().to_string();
        let claim = WriterClaim {
            operation_id: operation_id.clone(),
            process_identity: identity,
        };
        file.set_len(0)
            .map_err(|error| format!("workstation_writer_truncate_failed:{error}"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|error| format!("workstation_writer_seek_failed:{error}"))?;
        serde_json::to_writer(&mut file, &claim)
            .map_err(|error| format!("workstation_writer_publish_failed:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("workstation_writer_sync_failed:{error}"))?;
        // Closing releases the short physical lock; the exact process claim
        // coordinates the long operation without holding a mutex across waits.
        Ok(Self { path, operation_id })
    }
}

pub(crate) fn workstation_transaction_is_foreign(
    path: &Path,
    current_pid: u32,
) -> Result<bool, String> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("workstation_writer_observation_failed:{error}")),
    };
    if file.try_lock().is_err() {
        return Ok(true);
    }
    Ok(owner_pid(&mut file)?.is_some_and(|pid| pid != current_pid))
}

fn owner_pid(file: &mut File) -> Result<Option<u32>, String> {
    let mut raw = String::new();
    file.take(8192)
        .read_to_string(&mut raw)
        .map_err(|error| format!("workstation_writer_read_failed:{error}"))?;
    if let Ok(claim) = serde_json::from_str::<WriterClaim>(&raw) {
        let observed = observe_process(claim.process_identity.pid);
        if let ProcessObservation::Failed { reason } = &observed {
            return Err(format!("workstation_writer_identity_unavailable:{reason}"));
        }
        let assessment = assess_process_ownership(
            Some(&claim.process_identity),
            observed,
            LegacyProfileProof::Unproven,
        );
        return Ok(
            (assessment.ownership == RuntimeProcessOwnership::MatchingBrowser)
                .then_some(claim.process_identity.pid),
        );
    }
    // Compatibility with an executing older installer. A naked PID pointing
    // at another program is only stale diagnostics, including after PID reuse.
    let Some(pid) = raw.trim().parse::<u32>().ok() else {
        return Ok(None);
    };
    let ProcessObservation::Observed(observed) = observe_process(pid) else {
        return Ok(None);
    };
    let executable = observed
        .executable_path
        .as_deref()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str());
    let arguments = observed.command_line.unwrap_or_default();
    Ok((executable == Some("agent-browser")
        && arguments.iter().any(|arg| arg == "install")
        && arguments.iter().any(|arg| arg == "--apply"))
    .then_some(pid))
}

impl Drop for WorkstationLock {
    fn drop(&mut self) {
        let Ok(mut file) = OpenOptions::new().read(true).write(true).open(&self.path) else {
            return;
        };
        if file.try_lock().is_err() {
            return;
        }
        let mut raw = String::new();
        if file.read_to_string(&mut raw).is_ok()
            && serde_json::from_str::<WriterClaim>(&raw)
                .is_ok_and(|claim| claim.operation_id == self.operation_id)
        {
            let _ = file.set_len(0);
            let _ = file.sync_all();
        }
        // Keep the inode stable so a waiting contender cannot lock a removed
        // file while another process opens a replacement at the same path.
    }
}
