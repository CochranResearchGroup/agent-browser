//! Durable filesystem adapter for the pure candidate coordination ledger.
//!
//! Physical locks cover only read, compare, and atomic commit. They never span
//! builds, installer effects, process convergence, or browser work.

use agent_browser_candidate::{
    CoordinationAction, CoordinationLedger, CoordinationOutcome, CoordinationReceipt,
    CoordinationRequest,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PRODUCTION_ENVIRONMENT_ID: &str = "production";
#[cfg_attr(not(test), allow(dead_code))]
const INSTALL_CUSTODY_SCHEMA_VERSION: &str = "agent-browser.candidate-install-custody.v1";

/// Exact logical custody submitted to one bounded workstation mutation.
///
/// The revision is refreshed after harmless joins. The operation identity and
/// fencing generation never change while the same writer retains custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct CandidateInstallCustody {
    pub(crate) schema_version: String,
    pub(crate) environment_id: String,
    pub(crate) operation_id: String,
    pub(crate) candidate_id: String,
    pub(crate) artifact_id: String,
    pub(crate) revision: u64,
    pub(crate) fencing_generation: u64,
    pub(crate) acquisition_outcome: CoordinationOutcome,
}

#[derive(Debug)]
pub(crate) struct CandidateCoordinationStore {
    ledger_path: PathBuf,
    lock_path: PathBuf,
    environment_id: String,
}

impl CandidateCoordinationStore {
    pub(crate) fn production(root: &Path) -> Self {
        let directory = root.join(".agent-browser/candidates");
        Self {
            ledger_path: directory.join("coordination.json"),
            lock_path: directory.join("coordination.lock"),
            environment_id: PRODUCTION_ENVIRONMENT_ID.to_string(),
        }
    }

    /// Read a stable old-or-new ledger snapshot. Atomic rename prevents a
    /// reader from observing a partial commit, so read-only inspection needs no
    /// physical lock and never creates the candidate directory.
    pub(crate) fn read(&self) -> Result<CoordinationLedger, String> {
        load_ledger(&self.ledger_path, &self.environment_id)
    }

    /// Apply one exact compare-and-swap request and durably commit it before
    /// releasing the short-lived coordination lock.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn apply(
        &self,
        request: CoordinationRequest,
    ) -> Result<CoordinationReceipt, String> {
        let _lock = CandidateCoordinationLock::acquire(&self.lock_path)?;
        let ledger = self.read()?;
        self.commit_request(ledger, request)
            .map(|(receipt, _)| receipt)
    }

    fn commit_request(
        &self,
        mut ledger: CoordinationLedger,
        request: CoordinationRequest,
    ) -> Result<(CoordinationReceipt, CoordinationLedger), String> {
        let before = ledger.clone();
        let receipt = ledger.apply(request).map_err(|error| error.to_string())?;
        ledger.validate().map_err(|error| error.to_string())?;
        if ledger != before {
            persist_ledger(&self.ledger_path, &ledger)?;
        }
        Ok((receipt, ledger))
    }

    /// Starts or joins the exact candidate artifact that may drive an install.
    /// A same-candidate request with a different sealed artifact is not treated
    /// as the same writer.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn start_or_join_install(
        &self,
        request_id: &str,
        candidate_id: &str,
        artifact_id: &str,
    ) -> Result<CandidateInstallCustody, String> {
        let _lock = CandidateCoordinationLock::acquire(&self.lock_path)?;
        let ledger = self.read()?;
        if let Some(active) = ledger.active() {
            if active.candidate_id == candidate_id && active.artifact_id != artifact_id {
                return Err(format!(
                    "candidate_install_artifact_contended:{}",
                    active.operation_id
                ));
            }
        }
        let (receipt, current) = self.commit_request(
            ledger.clone(),
            CoordinationRequest {
                request_id: request_id.to_string(),
                candidate_id: candidate_id.to_string(),
                artifact_id: artifact_id.to_string(),
                expected_revision: ledger.revision,
                expected_fencing_generation: ledger.fencing_generation,
                action: CoordinationAction::StartOrJoin,
            },
        )?;
        if !matches!(
            receipt.outcome,
            CoordinationOutcome::Started | CoordinationOutcome::JoinedExisting
        ) {
            return Err(format!(
                "candidate_install_contended:{}",
                receipt.active_operation_id.as_deref().unwrap_or("unknown")
            ));
        }
        let active = current
            .active()
            .ok_or_else(|| "candidate_install_active_operation_missing".to_string())?;
        if active.operation_id != receipt.operation_id.as_deref().unwrap_or_default()
            || active.candidate_id != candidate_id
            || active.artifact_id != artifact_id
            || active.fencing_generation != receipt.fencing_generation
        {
            return Err("candidate_install_active_operation_changed".to_string());
        }
        Ok(CandidateInstallCustody {
            schema_version: INSTALL_CUSTODY_SCHEMA_VERSION.to_string(),
            environment_id: current.environment_id.clone(),
            operation_id: active.operation_id.clone(),
            candidate_id: active.candidate_id.clone(),
            artifact_id: active.artifact_id.clone(),
            revision: current.revision,
            fencing_generation: active.fencing_generation,
            acquisition_outcome: receipt.outcome,
        })
    }

    /// Runs one bounded mutation while the coordination lock protects the
    /// exact operation and fence from cancellation or supersession.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_active_install<T>(
        &self,
        custody: &CandidateInstallCustody,
        mutation: impl FnOnce(&CandidateInstallCustody) -> Result<T, String>,
    ) -> Result<T, String> {
        let _lock = CandidateCoordinationLock::acquire(&self.lock_path)?;
        let ledger = self.read()?;
        let active = ledger.active().filter(|active| {
            active.operation_id == custody.operation_id
                && active.candidate_id == custody.candidate_id
                && active.artifact_id == custody.artifact_id
                && active.fencing_generation == custody.fencing_generation
        });
        let active = active
            .ok_or_else(|| format!("candidate_install_custody_lost:{}", custody.operation_id))?;
        if ledger.environment_id != custody.environment_id {
            return Err("candidate_install_environment_changed".to_string());
        }
        let proof = CandidateInstallCustody {
            schema_version: INSTALL_CUSTODY_SCHEMA_VERSION.to_string(),
            environment_id: ledger.environment_id.clone(),
            operation_id: active.operation_id.clone(),
            candidate_id: active.candidate_id.clone(),
            artifact_id: active.artifact_id.clone(),
            revision: ledger.revision,
            fencing_generation: active.fencing_generation,
            acquisition_outcome: custody.acquisition_outcome,
        };
        mutation(&proof)
    }

    /// Commits the exact active install as terminal only after its bounded
    /// workstation acceptance mutation succeeds under the same lock.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn complete_active_install<T>(
        &self,
        custody: &CandidateInstallCustody,
        request_id: &str,
        mutation: impl FnOnce(&CandidateInstallCustody) -> Result<T, String>,
    ) -> Result<(T, CoordinationReceipt), String> {
        let _lock = CandidateCoordinationLock::acquire(&self.lock_path)?;
        let ledger = self.read()?;
        let active = ledger
            .active()
            .filter(|active| {
                active.operation_id == custody.operation_id
                    && active.candidate_id == custody.candidate_id
                    && active.artifact_id == custody.artifact_id
                    && active.fencing_generation == custody.fencing_generation
            })
            .ok_or_else(|| format!("candidate_install_custody_lost:{}", custody.operation_id))?;
        if ledger.environment_id != custody.environment_id {
            return Err("candidate_install_environment_changed".to_string());
        }
        let proof = CandidateInstallCustody {
            schema_version: INSTALL_CUSTODY_SCHEMA_VERSION.to_string(),
            environment_id: ledger.environment_id.clone(),
            operation_id: active.operation_id.clone(),
            candidate_id: active.candidate_id.clone(),
            artifact_id: active.artifact_id.clone(),
            revision: ledger.revision,
            fencing_generation: active.fencing_generation,
            acquisition_outcome: custody.acquisition_outcome,
        };
        let result = mutation(&proof)?;
        let operation_id = proof.operation_id.clone();
        let (receipt, _) = self.commit_request(
            ledger.clone(),
            CoordinationRequest {
                request_id: request_id.to_string(),
                candidate_id: proof.candidate_id.clone(),
                artifact_id: proof.artifact_id.clone(),
                expected_revision: ledger.revision,
                expected_fencing_generation: ledger.fencing_generation,
                action: CoordinationAction::CompleteActive { operation_id },
            },
        )?;
        if receipt.outcome != CoordinationOutcome::Completed
            || receipt.operation_id.as_deref() != Some(proof.operation_id.as_str())
            || receipt.active_operation_id.is_some()
        {
            return Err("candidate_install_completion_receipt_invalid".to_string());
        }
        Ok((result, receipt))
    }

    /// Finishes a previously accepted install after a crash between the
    /// workstation acceptance commit and the coordination commit. Replaying an
    /// already completed operation is read-only and returns its durable
    /// receipt.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn ensure_install_completed(
        &self,
        custody: &CandidateInstallCustody,
        request_id: &str,
    ) -> Result<CoordinationReceipt, String> {
        let _lock = CandidateCoordinationLock::acquire(&self.lock_path)?;
        let ledger = self.read()?;
        if ledger.environment_id != custody.environment_id {
            return Err("candidate_install_environment_changed".to_string());
        }
        if let Some(receipt) = ledger.receipts().iter().rev().find(|receipt| {
            receipt.outcome == CoordinationOutcome::Completed
                && receipt.operation_id.as_deref() == Some(custody.operation_id.as_str())
                && receipt.fencing_generation > custody.fencing_generation
        }) {
            return Ok(receipt.clone());
        }
        let active = ledger.active().filter(|active| {
            active.operation_id == custody.operation_id
                && active.candidate_id == custody.candidate_id
                && active.artifact_id == custody.artifact_id
                && active.fencing_generation == custody.fencing_generation
        });
        if active.is_none() {
            return Err(format!(
                "candidate_install_custody_lost:{}",
                custody.operation_id
            ));
        }
        let operation_id = custody.operation_id.clone();
        let (receipt, _) = self.commit_request(
            ledger.clone(),
            CoordinationRequest {
                request_id: request_id.to_string(),
                candidate_id: custody.candidate_id.clone(),
                artifact_id: custody.artifact_id.clone(),
                expected_revision: ledger.revision,
                expected_fencing_generation: ledger.fencing_generation,
                action: CoordinationAction::CompleteActive { operation_id },
            },
        )?;
        if receipt.outcome != CoordinationOutcome::Completed
            || receipt.operation_id.as_deref() != Some(custody.operation_id.as_str())
            || receipt.active_operation_id.is_some()
        {
            return Err("candidate_install_completion_receipt_invalid".to_string());
        }
        Ok(receipt)
    }
}

fn load_ledger(path: &Path, environment_id: &str) -> Result<CoordinationLedger, String> {
    let ledger = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("candidate_coordination_ledger_invalid_json:{error}"))?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            CoordinationLedger::new(environment_id)
        }
        Err(error) => return Err(format_io("read candidate coordination ledger", path, error)),
    };
    ledger.validate().map_err(|error| error.to_string())?;
    if ledger.environment_id != environment_id {
        return Err("candidate_coordination_environment_mismatch".to_string());
    }
    Ok(ledger)
}

fn persist_ledger(path: &Path, ledger: &CoordinationLedger) -> Result<(), String> {
    ledger.validate().map_err(|error| error.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "candidate_coordination_parent_missing".to_string())?;
    create_private_directory(parent)?;
    let temporary = parent.join(format!(".coordination.{}.tmp", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary).map_err(|error| {
            format_io(
                "create candidate coordination temporary file",
                &temporary,
                error,
            )
        })?;
        let mut bytes = serde_json::to_vec_pretty(ledger)
            .map_err(|error| format!("candidate_coordination_serialize_failed:{error}"))?;
        bytes.push(b'\n');
        file.write_all(&bytes).map_err(|error| {
            format_io(
                "write candidate coordination temporary file",
                &temporary,
                error,
            )
        })?;
        file.sync_all().map_err(|error| {
            format_io(
                "sync candidate coordination temporary file",
                &temporary,
                error,
            )
        })?;
        atomic_replace(&temporary, path)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination)
        .map_err(|error| format_io("commit candidate coordination ledger", destination, error))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(format_io(
            "commit candidate coordination ledger",
            destination,
            io::Error::last_os_error(),
        ))
    } else {
        Ok(())
    }
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format_io("create candidate coordination directory", path, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format_io("protect candidate coordination directory", path, error))?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format_io("sync candidate coordination directory", path, error))?;
    }
    Ok(())
}

#[derive(Debug)]
struct CandidateCoordinationLock {
    path: PathBuf,
    token: String,
}

impl CandidateCoordinationLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        let parent = path
            .parent()
            .ok_or_else(|| "candidate_coordination_lock_parent_missing".to_string())?;
        create_private_directory(parent)?;
        let token = format!("{}:{}", std::process::id(), Uuid::new_v4());
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                format!("candidate_coordination_lock_contended:{}", path.display())
            } else {
                format_io("acquire candidate coordination lock", path, error)
            }
        })?;
        writeln!(file, "{token}")
            .map_err(|error| format_io("write candidate coordination lock", path, error))?;
        file.sync_all()
            .map_err(|error| format_io("sync candidate coordination lock", path, error))?;
        Ok(Self {
            path: path.to_path_buf(),
            token,
        })
    }
}

impl Drop for CandidateCoordinationLock {
    fn drop(&mut self) {
        let owns_lock =
            fs::read_to_string(&self.path).is_ok_and(|value| value.trim() == self.token);
        if owns_lock {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn format_io(action: &str, path: &Path, error: io::Error) -> String {
    format!("Unable to {action} {}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                root: std::env::temp_dir().join(format!(
                    "agent-browser-candidate-coordination-test-{}",
                    Uuid::new_v4()
                )),
            }
        }

        fn store(&self) -> CandidateCoordinationStore {
            CandidateCoordinationStore::production(&self.root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn start_request(ledger: &CoordinationLedger) -> CoordinationRequest {
        CoordinationRequest {
            request_id: "request-a".to_string(),
            candidate_id: "candidate-a".to_string(),
            artifact_id: "artifact-a".to_string(),
            expected_revision: ledger.revision,
            expected_fencing_generation: ledger.fencing_generation,
            action: CoordinationAction::StartOrJoin,
        }
    }

    #[test]
    fn read_only_open_does_not_create_state() {
        let fixture = Fixture::new();
        let store = fixture.store();

        let ledger = store.read().unwrap();

        assert_eq!(ledger.environment_id, PRODUCTION_ENVIRONMENT_ID);
        assert!(!store.ledger_path.exists());
        assert!(!store.lock_path.exists());
    }

    #[test]
    fn apply_atomically_persists_and_exact_replay_does_not_rewrite() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let request = start_request(&store.read().unwrap());

        let receipt = store.apply(request.clone()).unwrap();
        assert_eq!(receipt.outcome, CoordinationOutcome::Started);
        let first_bytes = fs::read(&store.ledger_path).unwrap();
        let replay = store.apply(request).unwrap();

        assert_eq!(replay, receipt);
        assert_eq!(fs::read(&store.ledger_path).unwrap(), first_bytes);

        let current = store.read().unwrap();
        let queued = store
            .apply(CoordinationRequest {
                request_id: "request-b".to_string(),
                candidate_id: "candidate-b".to_string(),
                artifact_id: "artifact-b".to_string(),
                expected_revision: current.revision,
                expected_fencing_generation: current.fencing_generation,
                action: CoordinationAction::Queue,
            })
            .unwrap();
        assert_eq!(queued.outcome, CoordinationOutcome::Queued);
        assert_ne!(fs::read(&store.ledger_path).unwrap(), first_bytes);
        assert!(!store.lock_path.exists());
        assert_eq!(
            fs::read_dir(store.ledger_path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .count(),
            0
        );
    }

    #[test]
    fn corrupt_ledger_and_lock_contention_fail_without_rewrite() {
        let fixture = Fixture::new();
        let store = fixture.store();
        create_private_directory(store.ledger_path.parent().unwrap()).unwrap();
        fs::write(&store.ledger_path, b"{\"schemaVersion\":\"wrong\"}\n").unwrap();
        let corrupt = fs::read(&store.ledger_path).unwrap();
        assert!(store.read().unwrap_err().contains("invalid_json"));
        assert_eq!(fs::read(&store.ledger_path).unwrap(), corrupt);

        fs::remove_file(&store.ledger_path).unwrap();
        fs::write(&store.lock_path, b"foreign\n").unwrap();
        let request = start_request(&store.read().unwrap());
        assert!(store.apply(request).unwrap_err().contains("lock_contended"));
        assert!(!store.ledger_path.exists());
    }

    #[test]
    fn bounded_install_mutation_holds_exact_custody_and_fences_superseded_writer() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let custody = store
            .start_or_join_install("install-request-a", "candidate-a", "artifact-a")
            .unwrap();

        let joined = fixture
            .store()
            .start_or_join_install("install-request-a-join", "candidate-a", "artifact-a")
            .unwrap();
        assert_eq!(joined.operation_id, custody.operation_id);
        assert_eq!(joined.fencing_generation, custody.fencing_generation);
        assert!(joined.revision > custody.revision);

        let observed = store
            .with_active_install(&custody, |proof| {
                assert_eq!(proof.operation_id, custody.operation_id);
                assert_eq!(proof.revision, joined.revision);
                assert_eq!(proof.fencing_generation, custody.fencing_generation);

                let current = fixture.store().read().unwrap();
                let error = fixture
                    .store()
                    .apply(CoordinationRequest {
                        request_id: "supersede-while-mutating".to_string(),
                        candidate_id: "candidate-b".to_string(),
                        artifact_id: "artifact-b".to_string(),
                        expected_revision: current.revision,
                        expected_fencing_generation: current.fencing_generation,
                        action: CoordinationAction::Supersede {
                            operation_id: custody.operation_id.clone(),
                        },
                    })
                    .unwrap_err();
                assert!(error.contains("lock_contended"));
                Ok(proof.clone())
            })
            .unwrap();
        assert_eq!(observed.revision, joined.revision);

        let current = store.read().unwrap();
        store
            .apply(CoordinationRequest {
                request_id: "supersede-after-mutation".to_string(),
                candidate_id: "candidate-b".to_string(),
                artifact_id: "artifact-b".to_string(),
                expected_revision: current.revision,
                expected_fencing_generation: current.fencing_generation,
                action: CoordinationAction::Supersede {
                    operation_id: custody.operation_id.clone(),
                },
            })
            .unwrap();

        assert!(store
            .with_active_install(&custody, |_| Ok(()))
            .unwrap_err()
            .contains("candidate_install_custody_lost"));
    }

    #[test]
    fn same_candidate_with_different_seal_cannot_join_install_custody() {
        let fixture = Fixture::new();
        let store = fixture.store();
        store
            .start_or_join_install("install-a", "candidate-a", "artifact-a")
            .unwrap();
        let before = fs::read(&store.ledger_path).unwrap();

        let error = store
            .start_or_join_install("install-b", "candidate-a", "artifact-b")
            .unwrap_err();

        assert!(error.contains("candidate_install_artifact_contended"));
        assert_eq!(fs::read(&store.ledger_path).unwrap(), before);
    }

    #[test]
    fn terminal_install_commit_holds_custody_through_acceptance_mutation() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let custody = store
            .start_or_join_install("install-a", "candidate-a", "artifact-a")
            .unwrap();

        let (observed_revision, receipt) = store
            .complete_active_install(&custody, "complete-install-a", |proof| {
                let current = fixture.store().read().unwrap();
                let error = fixture
                    .store()
                    .apply(CoordinationRequest {
                        request_id: "supersede-during-completion".to_string(),
                        candidate_id: "candidate-b".to_string(),
                        artifact_id: "artifact-b".to_string(),
                        expected_revision: current.revision,
                        expected_fencing_generation: current.fencing_generation,
                        action: CoordinationAction::Supersede {
                            operation_id: custody.operation_id.clone(),
                        },
                    })
                    .unwrap_err();
                assert!(error.contains("lock_contended"));
                Ok(proof.revision)
            })
            .unwrap();

        assert_eq!(observed_revision, custody.revision);
        assert_eq!(receipt.outcome, CoordinationOutcome::Completed);
        assert_eq!(
            receipt.operation_id.as_deref(),
            Some(custody.operation_id.as_str())
        );
        let ledger = store.read().unwrap();
        assert!(ledger.active().is_none());
        assert!(ledger.fencing_generation > custody.fencing_generation);
        assert!(!store.lock_path.exists());
    }

    #[test]
    fn accepted_install_completion_recovery_is_idempotent() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let custody = store
            .start_or_join_install("install-a", "candidate-a", "artifact-a")
            .unwrap();

        let receipt = store
            .ensure_install_completed(&custody, "complete-install-a")
            .unwrap();
        let bytes = fs::read(&store.ledger_path).unwrap();
        let replay = store
            .ensure_install_completed(&custody, "complete-install-a")
            .unwrap();

        assert_eq!(replay, receipt);
        assert_eq!(receipt.outcome, CoordinationOutcome::Completed);
        assert_eq!(fs::read(&store.ledger_path).unwrap(), bytes);
        assert!(store.read().unwrap().active().is_none());
    }
}
