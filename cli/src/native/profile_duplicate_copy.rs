//! Profile duplication owns durable intent and private file copying; its runtime
//! adapter supplies exact stopped-source custody and catalog/browser effects.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::{fs, io::Write};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProfile {
    pub id: String,
    pub name: String,
    pub user_data_dir: PathBuf,
    pub browser_build: String,
    pub generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationProfile {
    pub id: String,
    pub name: String,
    pub user_data_dir: PathBuf,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateRequest {
    pub operation_id: String,
    pub source: SourceProfile,
    pub destination: DestinationProfile,
    pub launch: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Prepared,
    AwaitingOwnerChoice,
    AwaitingGracefulClose,
    Copying,
    Copied,
    Published,
    LaunchDispatched,
    Completed,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub request: DuplicateRequest,
    pub phase: Phase,
    pub affected_task_ids: Vec<String>,
    pub duplicate: Option<DuplicateProfile>,
    pub launch: Option<LaunchReceipt>,
    pub failure: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateProfile {
    pub destination: DestinationProfile,
    pub source_profile_id: String,
    pub operation_id: String,
    pub browser_build: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchReceipt {
    pub task_id: String,
    pub browser_id: String,
    pub handoff_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceObservation {
    pub generation: u64,
    pub running: bool,
    pub affected_task_ids: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Proceed,
    FocusSource,
    CloseAndDuplicate,
    Cancel,
    Reconcile,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseOutcome {
    Stopped,
    Pending,
}

/// Each effect must be keyed by operation ID. The custody guard must exclude
/// profile acquisitions and verify exact original-process clean close, not merely
/// absence from an inventory. No successful copy implies authenticated access.
pub trait DuplicationRuntime {
    type Guard;
    fn inspect_source(&mut self, source: &SourceProfile) -> Result<SourceObservation, String>;
    fn acquire_stopped_source(&mut self, source: &SourceProfile) -> Result<Self::Guard, String>;
    fn focus_source(&mut self, source: &SourceProfile) -> Result<(), String>;
    fn graceful_close_source(
        &mut self,
        source: &SourceProfile,
        operation_id: &str,
    ) -> Result<CloseOutcome, String>;
    fn publish_profile(&mut self, profile: &DuplicateProfile) -> Result<(), String>;
    fn launch_duplicate(&mut self, profile: &DuplicateProfile) -> Result<LaunchReceipt, String>;
    fn reconcile_launch(
        &mut self,
        profile: &DuplicateProfile,
    ) -> Result<Option<LaunchReceipt>, String>;
}

pub struct DuplicationStore {
    connection: Connection,
    lock_base: PathBuf,
}
impl DuplicationStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection =
            Connection::open(path).map_err(|_| "profile_duplicate_store_unavailable")?;
        private_permissions(path, 0o600)?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS profile_duplicate_operations (id TEXT PRIMARY KEY, destination TEXT NOT NULL, terminal INTEGER NOT NULL DEFAULT 0, state TEXT NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS profile_duplicate_active_destination ON profile_duplicate_operations(destination) WHERE terminal=0;").map_err(|_| "profile_duplicate_store_unavailable")?;
        Ok(Self {
            connection,
            lock_base: fs::canonicalize(path).map_err(|_| "profile_duplicate_store_unavailable")?,
        })
    }
    pub fn prepare(&mut self, request: DuplicateRequest) -> Result<Operation, String> {
        if let Some(old) = self.read(&request.operation_id)? {
            return if old.request == request {
                Ok(old)
            } else {
                Err("profile_duplicate_id_conflict".into())
            };
        }
        if [
            &request.operation_id,
            &request.source.id,
            &request.source.name,
            &request.source.browser_build,
            &request.destination.id,
            &request.destination.name,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
            || request.source.id == request.destination.id
            || request.source.generation == 0
            || !request.source.user_data_dir.is_absolute()
            || !request.destination.user_data_dir.is_absolute()
        {
            return Err("profile_duplicate_intent_invalid".into());
        }
        let op = Operation {
            request,
            phase: Phase::Prepared,
            affected_task_ids: vec![],
            duplicate: None,
            launch: None,
            failure: None,
        };
        self.connection
            .execute(
                "INSERT INTO profile_duplicate_operations (id,destination,state) VALUES (?1,?2,?3)",
                params![
                    op.request.operation_id,
                    op.request.destination.id,
                    serde_json::to_string(&op).map_err(|_| "profile_duplicate_encode_failed")?
                ],
            )
            .map_err(|_| "profile_duplicate_destination_conflict")?;
        Ok(op)
    }
    pub fn read(&self, id: &str) -> Result<Option<Operation>, String> {
        let data: Option<String> = self
            .connection
            .query_row(
                "SELECT state FROM profile_duplicate_operations WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "profile_duplicate_read_failed")?;
        data.map(|s| serde_json::from_str(&s).map_err(|_| "profile_duplicate_state_invalid".into()))
            .transpose()
    }
    fn save(&self, op: &Operation) -> Result<(), String> {
        let encoded = serde_json::to_string(op).map_err(|_| "profile_duplicate_encode_failed")?;
        self.connection
            .execute(
                "UPDATE profile_duplicate_operations SET state=?1, terminal=?2 WHERE id=?3",
                params![
                    encoded,
                    matches!(op.phase, Phase::Completed | Phase::Cancelled) as i64,
                    op.request.operation_id
                ],
            )
            .map_err(|_| "profile_duplicate_write_failed")?;
        Ok(())
    }
    pub fn advance(
        &mut self,
        id: &str,
        choice: Choice,
        runtime: &mut impl DuplicationRuntime,
    ) -> Result<Operation, String> {
        let lock_path = self.lock_base.with_extension(format!(
            "duplicate-{:x}.lock",
            Sha256::digest(id.as_bytes())
        ));
        let _operation_lock = OperationLock::acquire(&lock_path)?;
        let result = (|| {
            let mut op = self.read(id)?.ok_or("profile_duplicate_not_found")?;
            if matches!(op.phase, Phase::Completed | Phase::Cancelled) {
                return Ok(op);
            }
            if op.phase == Phase::LaunchDispatched {
                let duplicate = op
                    .duplicate
                    .as_ref()
                    .ok_or("profile_duplicate_copy_record_missing")?;
                if let Some(receipt) = runtime
                    .reconcile_launch(duplicate)
                    .map_err(|_| "profile_duplicate_launch_readback_required")?
                {
                    op.launch = Some(receipt);
                    op.failure = None;
                    op.phase = Phase::Completed;
                    self.save(&op)?;
                }
                return Ok(op);
            }
            if matches!(op.phase, Phase::Copied | Phase::Published) {
                return self.publish_and_launch(op, runtime);
            }
            if choice == Choice::Cancel {
                if op.phase == Phase::Copying {
                    cancel_staging(&op.request)?;
                } else if !matches!(
                    op.phase,
                    Phase::Prepared | Phase::AwaitingOwnerChoice | Phase::AwaitingGracefulClose
                ) {
                    return Err("profile_duplicate_copy_started:reconcile operation".into());
                }
                op.phase = Phase::Cancelled;
                self.save(&op)?;
                return Ok(op);
            }
            let observed = runtime
                .inspect_source(&op.request.source)
                .map_err(|_| "profile_duplicate_source_readback_required")?;
            if observed.generation != op.request.source.generation {
                return Err("profile_duplicate_source_changed:refresh source".into());
            }
            if observed.running {
                op.affected_task_ids = observed.affected_task_ids;
                if choice == Choice::FocusSource {
                    runtime
                        .focus_source(&op.request.source)
                        .map_err(|_| "profile_duplicate_source_focus_failed")?;
                }
                if choice == Choice::CloseAndDuplicate {
                    // Persist before close; a lost reply remains inspectable and is
                    // not another close unless the owner explicitly retries it.
                    op.phase = Phase::AwaitingGracefulClose;
                    self.save(&op)?;
                    match runtime
                        .graceful_close_source(&op.request.source, id)
                        .map_err(|_| "profile_duplicate_close_readback_required")?
                    {
                        CloseOutcome::Pending => return Ok(op),
                        CloseOutcome::Stopped => {
                            let now = runtime
                                .inspect_source(&op.request.source)
                                .map_err(|_| "profile_duplicate_source_readback_required")?;
                            if now.running || now.generation != op.request.source.generation {
                                return Err("profile_duplicate_close_readback_required".into());
                            }
                        }
                    }
                } else {
                    if op.phase != Phase::AwaitingGracefulClose {
                        op.phase = Phase::AwaitingOwnerChoice;
                    }
                    self.save(&op)?;
                    return Ok(op);
                }
            }
            let _custody = runtime
                .acquire_stopped_source(&op.request.source)
                .map_err(|_| "profile_duplicate_source_custody_required")?;
            op.phase = Phase::Copying;
            self.save(&op)?;
            copy_profile(&op.request)?;
            let duplicate = DuplicateProfile {
                destination: op.request.destination.clone(),
                source_profile_id: op.request.source.id.clone(),
                operation_id: op.request.operation_id.clone(),
                browser_build: op.request.source.browser_build.clone(),
            };
            op.duplicate = Some(duplicate.clone());
            op.failure = None;
            op.phase = Phase::Copied;
            self.save(&op)?;
            self.publish_and_launch(op, runtime)
        })();
        if let Err(error) = &result {
            if let Some(mut operation) = self.read(id)? {
                operation.failure = Some(
                    error
                        .split(':')
                        .next()
                        .unwrap_or("profile_duplicate_readback_required")
                        .to_owned(),
                );
                self.save(&operation)?;
            }
        }
        result
    }
    fn publish_and_launch(
        &self,
        mut op: Operation,
        runtime: &mut impl DuplicationRuntime,
    ) -> Result<Operation, String> {
        let duplicate = op
            .duplicate
            .as_ref()
            .ok_or("profile_duplicate_copy_record_missing")?;
        if op.phase == Phase::Copied {
            // Catalog publication must compare operation lineage atomically; an
            // identical publication is an idempotent readback after a lost reply.
            runtime
                .publish_profile(duplicate)
                .map_err(|_| "profile_duplicate_catalog_readback_required")?;
            op.phase = Phase::Published;
            self.save(&op)?;
        }
        if op.request.launch {
            op.phase = Phase::LaunchDispatched;
            self.save(&op)?;
            op.launch = Some(
                runtime
                    .launch_duplicate(duplicate)
                    .map_err(|_| "profile_duplicate_launch_readback_required")?,
            );
        }
        op.failure = None;
        op.phase = Phase::Completed;
        self.save(&op)?;
        Ok(op)
    }
}

struct OperationLock {
    _file: fs::File,
}
impl OperationLock {
    #[cfg(unix)]
    fn acquire(path: &Path) -> Result<Self, String> {
        use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| "profile_duplicate_operation_lock_unavailable")?;
        private_permissions(path, 0o600)?;
        // The kernel releases custody on process exit. Historical lock files do
        // not veto an owner restart or require deleting coordination metadata.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                    "profile_duplicate_operation_busy:read operation and retry"
                } else {
                    "profile_duplicate_operation_lock_unavailable"
                }
                .into(),
            );
        }
        Ok(Self { _file: file })
    }
    #[cfg(not(unix))]
    fn acquire(_path: &Path) -> Result<Self, String> {
        Err("profile_duplicate_operation_lock_unsupported".into())
    }
}
fn private_permissions(path: &Path, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| "profile_duplicate_permissions_failed")?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        return Err("profile_duplicate_permissions_unsupported".into());
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct CopyMarker {
    request: DuplicateRequest,
    completed_digest: Option<String>,
}
fn cancel_staging(request: &DuplicateRequest) -> Result<(), String> {
    let parent = request
        .destination
        .user_data_dir
        .parent()
        .ok_or("profile_duplicate_destination_invalid")?;
    let parent =
        fs::canonicalize(parent).map_err(|_| "profile_duplicate_destination_parent_unavailable")?;
    let stage = parent.join(format!(
        ".agent-browser-duplicate-{:x}",
        Sha256::digest(request.operation_id.as_bytes())
    ));
    if stage.symlink_metadata().is_err() {
        return Ok(());
    }
    if !stage
        .symlink_metadata()
        .map_err(|_| "profile_duplicate_staging_unavailable")?
        .is_dir()
    {
        return Err("profile_duplicate_staging_conflict".into());
    }
    let marker: CopyMarker = serde_json::from_slice(
        &fs::read(stage.join("intent.json")).map_err(|_| "profile_duplicate_staging_conflict")?,
    )
    .map_err(|_| "profile_duplicate_staging_conflict")?;
    if marker.request != *request {
        return Err("profile_duplicate_staging_conflict".into());
    }
    if marker.completed_digest.is_some()
        && request.destination.user_data_dir.symlink_metadata().is_ok()
    {
        return Err("profile_duplicate_copy_readback_required:reconcile operation".into());
    }
    remove_staging_data(&stage)
}
fn copy_profile(request: &DuplicateRequest) -> Result<(), String> {
    let source = fs::canonicalize(&request.source.user_data_dir)
        .map_err(|_| "profile_duplicate_source_unavailable")?;
    let destination = &request.destination.user_data_dir;
    let parent = destination
        .parent()
        .ok_or("profile_duplicate_destination_invalid")?;
    let parent =
        fs::canonicalize(parent).map_err(|_| "profile_duplicate_destination_parent_unavailable")?;
    let destination = parent.join(
        destination
            .file_name()
            .ok_or("profile_duplicate_destination_invalid")?,
    );
    if source.starts_with(&destination) || destination.starts_with(&source) {
        return Err("profile_duplicate_paths_overlap".into());
    }
    let stage = parent.join(format!(
        ".agent-browser-duplicate-{:x}",
        Sha256::digest(request.operation_id.as_bytes())
    ));
    let marker_path = stage.join("intent.json");
    let data = stage.join("user-data");
    let marker = if stage.symlink_metadata().is_ok() {
        if !stage
            .symlink_metadata()
            .map_err(|_| "profile_duplicate_staging_unavailable")?
            .is_dir()
        {
            return Err("profile_duplicate_staging_conflict".into());
        }
        let marker: CopyMarker = serde_json::from_slice(
            &fs::read(&marker_path).map_err(|_| "profile_duplicate_staging_conflict")?,
        )
        .map_err(|_| "profile_duplicate_staging_conflict")?;
        if marker.request != *request {
            return Err("profile_duplicate_staging_conflict".into());
        }
        marker
    } else {
        if destination.symlink_metadata().is_ok() {
            return Err("profile_duplicate_destination_exists".into());
        }
        fs::create_dir(&stage).map_err(|_| "profile_duplicate_staging_create_failed")?;
        private_permissions(&stage, 0o700)?;
        let marker = CopyMarker {
            request: request.clone(),
            completed_digest: None,
        };
        save_marker(&marker_path, &marker)?;
        marker
    };
    // A rename reply or journal publication may have been lost. Only the exact
    // completed copy digest can adopt a destination under this operation.
    if destination.symlink_metadata().is_ok() {
        return if marker.completed_digest.as_deref() == Some(tree_digest(&destination)?.as_str())
            && !data.exists()
        {
            Ok(())
        } else {
            Err("profile_duplicate_destination_exists".into())
        };
    }
    if data.symlink_metadata().is_ok() {
        remove_staging_data(&data)?;
    }
    copy_directory(&source, &data)?;
    let complete = CopyMarker {
        request: request.clone(),
        completed_digest: Some(tree_digest(&data)?),
    };
    save_marker(&marker_path, &complete)?;
    publish_directory(&data, &destination)?;
    fs::File::open(&parent)
        .and_then(|file| file.sync_all())
        .map_err(|_| "profile_duplicate_sync_failed")?;
    Ok(())
}
fn save_marker(path: &Path, marker: &CopyMarker) -> Result<(), String> {
    let temporary = path.with_extension("pending");
    let bytes = serde_json::to_vec(marker).map_err(|_| "profile_duplicate_encode_failed")?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|_| "profile_duplicate_staging_write_failed")?;
    private_permissions(&temporary, 0o600)?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "profile_duplicate_staging_write_failed")?;
    fs::rename(&temporary, path).map_err(|_| "profile_duplicate_staging_write_failed")?;
    fs::File::open(path.parent().ok_or("profile_duplicate_staging_invalid")?)
        .and_then(|file| file.sync_all())
        .map_err(|_| "profile_duplicate_sync_failed")?;
    Ok(())
}
fn remove_staging_data(path: &Path) -> Result<(), String> {
    let metadata = path
        .symlink_metadata()
        .map_err(|_| "profile_duplicate_staging_read_failed")?;
    if metadata.is_dir() {
        private_permissions(path, 0o700)?;
        for entry in fs::read_dir(path).map_err(|_| "profile_duplicate_staging_read_failed")? {
            remove_staging_data(
                &entry
                    .map_err(|_| "profile_duplicate_staging_read_failed")?
                    .path(),
            )?;
        }
        fs::remove_dir(path).map_err(|_| "profile_duplicate_staging_cleanup_failed".to_string())
    } else {
        fs::remove_file(path).map_err(|_| "profile_duplicate_staging_cleanup_failed".to_string())
    }
}
#[cfg(target_os = "linux")]
fn publish_directory(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| "profile_duplicate_path_invalid")?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| "profile_duplicate_path_invalid")?;
    // Atomic no-replace publication: an independent destination is never removed.
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err("profile_duplicate_destination_publication_failed:inspect destination".into())
    }
}
#[cfg(not(target_os = "linux"))]
fn publish_directory(_source: &Path, _destination: &Path) -> Result<(), String> {
    Err("profile_duplicate_atomic_publication_unsupported".into())
}
fn tree_digest(root: &Path) -> Result<String, String> {
    fn visit(path: &Path, hash: &mut Sha256) -> Result<(), String> {
        let metadata = path
            .symlink_metadata()
            .map_err(|_| "profile_duplicate_digest_failed")?;
        if metadata.is_dir() {
            hash.update(b"directory");
            let mut entries = fs::read_dir(path)
                .map_err(|_| "profile_duplicate_digest_failed")?
                .map(|e| e.map_err(|_| "profile_duplicate_digest_failed"))
                .collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let name = entry.file_name();
                let name = name
                    .to_str()
                    .ok_or("profile_duplicate_filename_unsupported")?;
                hash.update((name.len() as u64).to_le_bytes());
                hash.update(name.as_bytes());
                visit(&entry.path(), hash)?;
            }
        } else if metadata.is_file() {
            hash.update(b"file");
            hash.update(metadata.len().to_le_bytes());
            let mut file = fs::File::open(path).map_err(|_| "profile_duplicate_digest_failed")?;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = std::io::Read::read(&mut file, &mut buffer)
                    .map_err(|_| "profile_duplicate_digest_failed")?;
                if read == 0 {
                    break;
                }
                hash.update(&buffer[..read]);
            }
        } else {
            return Err("profile_duplicate_unsupported_file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            hash.update(metadata.permissions().mode().to_le_bytes());
        }
        Ok(())
    }
    let mut hash = Sha256::new();
    visit(root, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}
fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination).map_err(|_| "profile_duplicate_create_failed")?;
    private_permissions(destination, 0o700)?;
    for entry in fs::read_dir(source).map_err(|_| "profile_duplicate_read_failed")? {
        let entry = entry.map_err(|_| "profile_duplicate_read_failed")?;
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some(
                "SingletonLock"
                    | "SingletonCookie"
                    | "SingletonSocket"
                    | "DevToolsActivePort"
                    | "LOCK"
                    | "lockfile"
            )
        ) {
            continue;
        }
        let metadata = entry
            .path()
            .symlink_metadata()
            .map_err(|_| "profile_duplicate_read_failed")?;
        let target = destination.join(name);
        if metadata.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if metadata.is_file() {
            let mut input =
                fs::File::open(entry.path()).map_err(|_| "profile_duplicate_read_failed")?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|_| "profile_duplicate_create_failed")?;
            private_permissions(&target, 0o600)?;
            std::io::copy(&mut input, &mut output).map_err(|_| "profile_duplicate_copy_failed")?;
            output
                .flush()
                .map_err(|_| "profile_duplicate_copy_failed")?;
            output
                .sync_all()
                .map_err(|_| "profile_duplicate_copy_failed")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                private_permissions(&target, metadata.permissions().mode() & 0o600)?;
            }
        } else {
            return Err("profile_duplicate_unsupported_file".into());
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        private_permissions(
            destination,
            fs::metadata(source)
                .map_err(|_| "profile_duplicate_read_failed")?
                .permissions()
                .mode()
                & 0o700,
        )?;
    }
    fs::File::open(destination)
        .and_then(|file| file.sync_all())
        .map_err(|_| "profile_duplicate_sync_failed")?;
    Ok(())
}

impl Operation {
    /// Public adapters share this bounded projection. Private profile paths and
    /// copy digests stay in governed runtime storage, not UI/MCP/log payloads.
    pub fn public_view(&self) -> serde_json::Value {
        use serde_json::json;
        let actions = match self.phase {
            Phase::Prepared => vec![("reconcile", false), ("cancel", false)],
            Phase::AwaitingOwnerChoice => vec![
                ("focus_source", false),
                ("close_and_duplicate", true),
                ("cancel", false),
            ],
            Phase::AwaitingGracefulClose => vec![
                ("focus_source", false),
                ("close_and_duplicate", true),
                ("reconcile", false),
                ("cancel", false),
            ],
            Phase::Copying => vec![("reconcile", false), ("cancel", false)],
            Phase::Copied | Phase::Published | Phase::LaunchDispatched => {
                vec![("reconcile", false)]
            }
            Phase::Completed | Phase::Cancelled => vec![("reconcile", false)],
        };
        json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":self.request.operation_id,
            "source":{"id":self.request.source.id,"name":self.request.source.name,"browserBuild":self.request.source.browser_build,"generation":self.request.source.generation},
            "destination":{"id":self.request.destination.id,"name":self.request.destination.name},
            "phase":self.phase,"affectedTaskIds":self.affected_task_ids,"launchRequested":self.request.launch,
            "copyComplete":matches!(self.phase,Phase::Copied|Phase::Published|Phase::LaunchDispatched|Phase::Completed),
            "profilePublished":matches!(self.phase,Phase::Published|Phase::LaunchDispatched|Phase::Completed),
            "launch":self.launch,"failure":self.failure,"authenticationContinuity":"unverified",
            "actions":actions.into_iter().map(|(action,confirmation)|json!({"action":action,"requiresConfirmation":confirmation})).collect::<Vec<_>>(),
            "disruptionWarning":"Closing the source affects its tasks. Unsaved forms, downloads and live page state may not be restorable. The source reopens only when selected."})
    }
}
