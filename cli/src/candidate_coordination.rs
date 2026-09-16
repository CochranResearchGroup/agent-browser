//! Durable filesystem adapter for the pure candidate coordination ledger.
//!
//! Physical locks cover only read, compare, and atomic commit. They never span
//! builds, installer effects, process convergence, or browser work.

use agent_browser_candidate::{CoordinationLedger, CoordinationReceipt, CoordinationRequest};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PRODUCTION_ENVIRONMENT_ID: &str = "production";

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
        let mut ledger = self.read()?;
        let before = ledger.clone();
        let receipt = ledger.apply(request).map_err(|error| error.to_string())?;
        ledger.validate().map_err(|error| error.to_string())?;
        if ledger != before {
            persist_ledger(&self.ledger_path, &ledger)?;
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
    use agent_browser_candidate::{CoordinationAction, CoordinationOutcome};

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
}
