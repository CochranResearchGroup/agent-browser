//! Explicit disposal of one positively completed copy. Original intent and
//! lineage remain immutable; this is not general persistent-profile retention.
use super::profile_duplicate_custody::{load_document, LaunchLedger};
use super::*;
use agent_browser_service_model::{BrowserSessionState, BROWSER_SESSION_STATE_SCHEMA_V1};
use rusqlite::params;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::fs;
use std::path::{Component, Path, PathBuf};

const TABLE: &str = "browser_profile_copy_disposals";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Identity {
    pub operation_id: String,
    pub profile_id: String,
    pub source_id: String,
    pub path: PathBuf,
    pub quarantine: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub projected_bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Checkpoint {
    Prepared,
    Quarantined,
    CatalogRemoved,
    Deleting,
    Completed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Record {
    pub identity: Identity,
    pub checkpoint: Checkpoint,
    pub failure: Option<String>,
    pub delete_acknowledged: Option<bool>,
    pub completion_evidence: Option<String>,
}
impl Record {
    pub fn fence(&self) -> String {
        fence(&self.identity.operation_id)
    }
    pub fn public(&self) -> Value {
        json!({"receiptId":self.fence(),"destinationProfileId":self.identity.profile_id,
            "state":if self.failure.is_some(){"readback_required".into()}else{serde_json::to_value(self.checkpoint).unwrap()},
            "checkpoint":self.checkpoint,"failure":self.failure,"projectedBytes":self.identity.projected_bytes,
            "deleteAcknowledged":self.delete_acknowledged,"completionEvidence":self.completion_evidence})
    }
}
pub(super) fn fence(operation: &str) -> String {
    format!("copy-disposal:{:x}", Sha256::digest(operation.as_bytes()))
}
pub(super) fn initialize(connection: &Connection) -> Result<(), String> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS browser_profile_copy_disposals(id TEXT PRIMARY KEY,profile_id TEXT NOT NULL UNIQUE,receipt TEXT NOT NULL)").map_err(|_|"profile_duplicate_disposal_journal_unavailable".into())
}
pub(super) fn read_on(connection: &Connection, id: &str) -> Result<Option<Record>, String> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [TABLE],
            |row| row.get(0),
        )
        .map_err(|_| "profile_duplicate_disposal_readback_unavailable")?;
    if !exists {
        return Ok(None);
    }
    let raw: Option<(String, String)> = connection
        .query_row(
            "SELECT profile_id,receipt FROM browser_profile_copy_disposals WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| "profile_duplicate_disposal_readback_unavailable")?;
    raw.map(|(profile, raw)| {
        let record: Record =
            serde_json::from_str(&raw).map_err(|_| "profile_duplicate_disposal_receipt_invalid")?;
        if record.identity.operation_id != id || record.identity.profile_id != profile {
            return Err("profile_duplicate_disposal_receipt_conflict".into());
        }
        Ok(record)
    })
    .transpose()
}
pub(super) fn read(database: &Path, id: &str) -> Result<Option<Record>, String> {
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "profile_duplicate_disposal_readback_unavailable")?;
    read_on(&connection, id)
}
pub(super) fn write_on(connection: &Connection, record: &Record) -> Result<(), String> {
    connection.execute("INSERT INTO browser_profile_copy_disposals(id,profile_id,receipt) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET receipt=excluded.receipt",rusqlite::params![record.identity.operation_id,record.identity.profile_id,serde_json::to_string(record).map_err(|_|"profile_duplicate_disposal_encode_failed")?]).map_err(|_|"profile_duplicate_disposal_write_failed")?;
    Ok(())
}
pub(super) fn overlay(mut view: Value, record: &Record) -> Result<Value, String> {
    if view["operationId"] != record.identity.operation_id
        || view["source"]["id"] != record.identity.source_id
        || view["destination"]["id"] != record.identity.profile_id
    {
        return Err("profile_duplicate_disposal_lineage_changed".into());
    }
    view["cleanup"] = record.public();
    view["actions"] = if record.checkpoint == Checkpoint::Completed {
        json!([])
    } else {
        json!([{"action":"reconcile","requiresConfirmation":false}])
    };
    Ok(view)
}
fn no_symlink_components(path: &Path, allow_missing: bool) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("profile_duplicate_disposal_path_invalid".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir | Component::CurDir) {
            return Err("profile_duplicate_disposal_path_invalid".into());
        }
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("profile_duplicate_disposal_symlink_refused".into())
            }
            Ok(_) => {}
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("profile_duplicate_disposal_path_unavailable".into()),
        }
    }
    Ok(())
}
pub(super) fn paths(database: &Path, operation: &Operation) -> Result<(PathBuf, PathBuf), String> {
    let directory = database
        .parent()
        .ok_or("profile_duplicate_runtime_directory_missing")?
        .join("profile-duplication");
    let root = directory.join("profiles");
    no_symlink_components(&root, false)?;
    let expected = root.join(operation.request.destination.id.replace(':', "-"));
    if operation.request.destination.user_data_dir != expected
        || operation.request.source.id == operation.request.destination.id
        || expected.starts_with(&operation.request.source.user_data_dir)
        || operation
            .request
            .source
            .user_data_dir
            .starts_with(&expected)
    {
        return Err("profile_duplicate_disposal_path_conflict".into());
    }
    let quarantine = directory.join("disposal-quarantine").join(format!(
        "{:x}",
        Sha256::digest(operation.request.operation_id.as_bytes())
    ));
    no_symlink_components(&expected, true)?;
    no_symlink_components(&quarantine, true)?;
    Ok((expected, quarantine))
}
fn tree_bytes(path: &Path) -> Result<u64, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "profile_duplicate_disposal_path_unavailable")?;
    if metadata.file_type().is_symlink() {
        return Err("profile_duplicate_disposal_symlink_refused".into());
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err("profile_duplicate_disposal_special_file_refused".into());
    }
    let mut size = 0u64;
    for entry in fs::read_dir(path).map_err(|_| "profile_duplicate_disposal_tree_unavailable")? {
        size = size
            .checked_add(tree_bytes(
                &entry
                    .map_err(|_| "profile_duplicate_disposal_tree_unavailable")?
                    .path(),
            )?)
            .ok_or("profile_duplicate_disposal_size_overflow")?;
    }
    Ok(size)
}
fn file_identity(path: &Path) -> Result<Option<(u64, u64)>, String> {
    no_symlink_components(path, true)?;
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                Ok(Some((metadata.dev(), metadata.ino())))
            }
            #[cfg(not(unix))]
            {
                let _ = metadata;
                Err("profile_duplicate_disposal_platform_unqualified".into())
            }
        }
        _ => Err("profile_duplicate_disposal_path_unavailable".into()),
    }
}
fn payload(record: &Record) -> Result<bool, String> {
    if file_identity(&record.identity.path)?.is_some() {
        return Err("profile_duplicate_disposal_source_restored".into());
    }
    let Some(actual) = file_identity(&record.identity.quarantine)? else {
        return Ok(false);
    };
    if actual != (record.identity.device, record.identity.inode) {
        return Err("profile_duplicate_disposal_payload_changed".into());
    }
    tree_bytes(&record.identity.quarantine)?;
    Ok(true)
}
fn fault(directory: &Path, checkpoint: &str) -> Result<(), String> {
    #[cfg(test)]
    if fs::read_to_string(directory.join("fixture-disposal-stop-after"))
        .ok()
        .as_deref()
        == Some(checkpoint)
    {
        return Err("profile_duplicate_disposal_fixture_interrupted".into());
    }
    let _ = (directory, checkpoint);
    Ok(())
}
fn guarded<'a>(
    database: &Path,
    journal: &'a DuplicationStore,
    operation: &Operation,
) -> Result<
    (
        profile_duplicate_copy::OperationSnapshot<'a>,
        crate::native::service_store::ServiceStateEffectSnapshot,
        CopyDisposalCustody,
    ),
    String,
> {
    let operations = journal.hold_profile_operations(
        &operation.request.destination.id,
        &operation.request.operation_id,
    )?;
    let service = crate::native::service_store::LockedServiceStateRepository::default_json()?
        .hold_snapshot_for_effect()?;
    let custody = CopyDisposalCustody::acquire(database, operation)?;
    let (path, quarantine) = paths(database, operation)?;
    crate::native::service_resources::prove_copy_profile_unoccupied(
        &service.state,
        &operation.request.destination.id,
        &[path, quarantine],
        &chrono::Utc::now().to_rfc3339(),
    )?;
    Ok((operations, service, custody))
}
pub(super) fn route<
    P: BrowserSessionPersistence,
    E: BrowserSessionEffects + ManagedBrowserCommandEffects,
>(
    host: &mut BrowserSessionHost<P, E>,
    directory: &Path,
    operation_id: &str,
    command: &Value,
    intent: Option<&PublicIntent>,
) -> Result<Value, String> {
    let database = host.persistence.runtime_database_path()?;
    let journal = DuplicationStore::open(&directory.join("operations.sqlite3"))?;
    let _operation = journal.lock_operation(operation_id)?;
    let operation = journal
        .read(operation_id)?
        .filter(|operation| {
            operation.phase == Phase::Completed
                && operation.failure.is_none()
                && operation.duplicate.is_some()
        })
        .ok_or("profile_duplicate_disposal_completed_copy_required")?;
    let intent = intent.ok_or("profile_duplicate_disposal_intent_missing")?;
    if intent.cancelled
        || intent.operation_id != operation_id
        || intent.source_profile_id != operation.request.source.id
        || intent.destination_profile_id != operation.request.destination.id
        || intent.destination_name != operation.request.destination.name
        || intent.launch != operation.request.launch
    {
        return Err("profile_duplicate_disposal_intent_changed".into());
    }
    let duplicate = operation.duplicate.as_ref().unwrap();
    if duplicate.destination != operation.request.destination
        || duplicate.operation_id != operation_id
        || duplicate.source_profile_id != operation.request.source.id
        || duplicate.browser_build != operation.request.source.browser_build
    {
        return Err("profile_duplicate_disposal_lineage_changed".into());
    }
    if command
        .get("sourceProfileId")
        .is_some_and(|value| value.as_str() != Some(operation.request.source.id.as_str()))
        || [
            "destinationProfileId",
            "path",
            "actor",
            "requestPrincipalSource",
            "principal",
            "buildId",
            "executablePath",
            "mode",
            "destinationName",
        ]
        .iter()
        .any(|field| command.get(*field).is_some())
    {
        return Err("profile_duplicate_disposal_target_changed".into());
    }
    let choice = command["choice"].as_str().unwrap_or("prepare");
    let mut record = read(&database, operation_id)?;
    if let Some(retained) = &record {
        if retained.checkpoint == Checkpoint::Completed
            || !matches!(choice, "dispose_copy" | "reconcile")
        {
            return overlay(operation.public_view(), retained);
        }
    } else {
        if choice != "dispose_copy" || command["ownerConfirmed"] != true {
            return Err("profile_duplicate_owner_confirmation_required".into());
        }
        let (_operations, _service, mut custody) = guarded(&database, &journal, &operation)?;
        let (path, quarantine) = paths(&database, &operation)?;
        let approved = directory.join("profiles");
        let canonical = crate::runtime_retention::validate_reclaim_target(&path, &[approved])
            .map_err(|_| "profile_duplicate_disposal_path_unavailable")?;
        if canonical != path {
            return Err("profile_duplicate_disposal_path_conflict".into());
        }
        let (device, inode) =
            file_identity(&path)?.ok_or("profile_duplicate_disposal_payload_unacknowledged")?;
        let retained = Record {
            identity: Identity {
                operation_id: operation_id.into(),
                profile_id: operation.request.destination.id.clone(),
                source_id: operation.request.source.id.clone(),
                path,
                quarantine,
                device,
                inode,
                projected_bytes: tree_bytes(&canonical)?,
            },
            checkpoint: Checkpoint::Prepared,
            failure: None,
            delete_acknowledged: None,
            completion_evidence: None,
        };
        custody.prepare(&retained)?;
        record = Some(retained);
    }
    let mut record = record.unwrap();
    let effect = (|| {
        fault(directory, "prepared")?;
        if record.checkpoint == Checkpoint::Prepared {
            let (_operations, _service, mut custody) = guarded(&database, &journal, &operation)?;
            let source = file_identity(&record.identity.path)?;
            let quarantined = file_identity(&record.identity.quarantine)?;
            let expected = (record.identity.device, record.identity.inode);
            match (source, quarantined) {
                (Some(actual), None) if actual == expected => {
                    tree_bytes(&record.identity.path)?;
                    let parent = record.identity.quarantine.parent().unwrap();
                    no_symlink_components(parent, true)?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::DirBuilderExt;
                        fs::DirBuilder::new()
                            .recursive(true)
                            .mode(0o700)
                            .create(parent)
                            .map_err(|_| "profile_duplicate_disposal_quarantine_unavailable")?;
                    }
                    #[cfg(not(unix))]
                    return Err("profile_duplicate_disposal_platform_unqualified".into());
                    profile_duplicate_copy::publish_directory(
                        &record.identity.path,
                        &record.identity.quarantine,
                    )?;
                }
                (None, Some(actual)) if actual == expected => {
                    tree_bytes(&record.identity.quarantine)?;
                }
                _ => return Err("profile_duplicate_disposal_payload_unacknowledged".into()),
            }
            fault(directory, "renamed")?;
            record.checkpoint = Checkpoint::Quarantined;
            record.failure = None;
            custody.save(&record)?;
        }
        fault(directory, "quarantined")?;
        if record.checkpoint == Checkpoint::Quarantined {
            let (_operations, _service, mut custody) = guarded(&database, &journal, &operation)?;
            if !payload(&record)? {
                return Err("profile_duplicate_disposal_payload_unacknowledged".into());
            }
            record.checkpoint = Checkpoint::CatalogRemoved;
            record.failure = None;
            custody.remove_catalog(&record)?;
        }
        fault(directory, "catalog_removed")?;
        if record.checkpoint == Checkpoint::CatalogRemoved {
            let (_operations, _service, mut custody) = guarded(&database, &journal, &operation)?;
            if !payload(&record)? {
                return Err("profile_duplicate_disposal_payload_unacknowledged".into());
            }
            record.checkpoint = Checkpoint::Deleting;
            record.failure = None;
            custody.save(&record)?;
        }
        fault(directory, "deleting")?;
        if record.checkpoint == Checkpoint::Deleting {
            let present = {
                let (_operations, _service, _custody) = guarded(&database, &journal, &operation)?;
                payload(&record)?
            };
            if present {
                fs::remove_dir_all(&record.identity.quarantine)
                    .map_err(|_| "profile_duplicate_disposal_delete_unacknowledged")?;
            }
            fault(directory, "deleted")?;
            let (_operations, _service, mut custody) = guarded(&database, &journal, &operation)?;
            if payload(&record)? {
                return Err("profile_duplicate_disposal_payload_present".into());
            }
            record.checkpoint = Checkpoint::Completed;
            record.failure = None;
            record.delete_acknowledged = Some(present);
            record.completion_evidence = Some(
                if present {
                    "acknowledged_delete"
                } else {
                    "absent_after_retained_delete_intent"
                }
                .into(),
            );
            custody.finish(&record)?;
        }
        Ok::<(), String>(())
    })();
    if let Err(error) = effect {
        CopyDisposalCustody::record_failure(
            &database,
            operation_id,
            error
                .split(':')
                .next()
                .unwrap_or("profile_duplicate_disposal_readback_required"),
        )?;
    }
    host.catalog = host
        .persistence
        .load_or_import_profile_catalog(Path::new(""))?
        .catalog;
    host.state = host.persistence.load_session_state()?;
    host.persisted_state = host.state.clone();
    let retained =
        read(&database, operation_id)?.ok_or("profile_duplicate_disposal_receipt_missing")?;
    overlay(operation.public_view(), &retained)
}

/// One short Native owner transaction. Closed task/launch history is retained;
/// it is not substituted for current task, process or operation absence.
pub(super) struct CopyDisposalCustody {
    connection: Connection,
    state: BrowserSessionState,
    catalog: agent_browser_service_model::BrowserProfileCatalog,
    operation: super::profile_duplicate_copy::Operation,
    retained: Option<super::profile_duplicate_disposal::Record>,
    active: bool,
}
impl CopyDisposalCustody {
    pub fn acquire(
        database: &Path,
        operation: &super::profile_duplicate_copy::Operation,
    ) -> Result<Self, String> {
        use super::profile_duplicate_disposal::{self as disposal, Checkpoint};
        let connection = Connection::open_with_flags(
            database,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| "profile_duplicate_disposal_runtime_unavailable")?;
        connection
            .busy_timeout(std::time::Duration::ZERO)
            .map_err(|_| "profile_duplicate_disposal_runtime_unavailable")?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| "profile_duplicate_disposal_runtime_unavailable")?;
        if !(2..=crate::native::browser_session_store::BROWSER_RUNTIME_DATABASE_MAX_SCHEMA)
            .contains(&version)
        {
            return Err("profile_duplicate_disposal_schema_unqualified".into());
        }
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| "profile_duplicate_disposal_runtime_busy")?;
        let result = (|| {
            disposal::initialize(&connection)?;
            let state: BrowserSessionState = load_document(
                &connection,
                "browser_session_state",
                BROWSER_SESSION_STATE_SCHEMA_V1,
            )?;
            let catalog: agent_browser_service_model::BrowserProfileCatalog = load_document(
                &connection,
                "browser_profile_catalog",
                "agent-browser.browser-profile-catalog.v1",
            )?;
            let retained = disposal::read_on(&connection, &operation.request.operation_id)?;
            let profile = &operation.request.destination;
            let (path, quarantine) = disposal::paths(database, operation)?;
            if let Some(record) = &retained {
                if record.identity.operation_id != operation.request.operation_id
                    || record.identity.profile_id != profile.id
                    || record.identity.source_id != operation.request.source.id
                    || record.identity.path != path
                    || record.identity.quarantine != quarantine
                {
                    return Err("profile_duplicate_disposal_receipt_conflict".into());
                }
            }
            let expected_fence = disposal::fence(&operation.request.operation_id);
            if retained.is_none() && state.profile_operation_ids.contains_key(&profile.id)
                || state
                    .profile_operation_ids
                    .get(&profile.id)
                    .is_some_and(|id| id != &expected_fence)
                || retained
                    .as_ref()
                    .is_some_and(|record| record.checkpoint != Checkpoint::Completed)
                    && state.profile_operation_ids.get(&profile.id) != Some(&expected_fence)
            {
                return Err("profile_duplicate_disposal_foreign_fence".into());
            }
            if state
                .browsers
                .values()
                .any(|browser| browser.profile_id == profile.id)
                || state
                    .sessions
                    .values()
                    .any(|task| task.profile_id == profile.id)
                || state.disposable_profiles.contains_key(&profile.id)
                || state
                    .app_browser_attachments
                    .values()
                    .any(|attachment| attachment.profile_id == profile.id)
                || operation.launch.as_ref().is_some_and(|launch| {
                    state
                        .tabs
                        .values()
                        .any(|tab| tab.browser_id == launch.browser_id)
                })
            {
                return Err("profile_duplicate_disposal_native_reference_active".into());
            }
            crate::native::browser_launch_custody_store::reject_profile_metadata_mutation(
                &connection,
                &profile.id,
            )
            .map_err(|_| "profile_duplicate_disposal_launch_pending")?;
            crate::native::browser_idle_retirement_store::reject_profile_metadata_mutation(
                &connection,
                &profile.id,
            )
            .map_err(|_| "profile_duplicate_disposal_retirement_pending")?;
            let lineage:Option<(String,String,String)>=connection.query_row("SELECT source_profile_id,operation_id,browser_build FROM browser_profile_duplication_lineage WHERE profile_id=?1",[&profile.id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|_|"profile_duplicate_disposal_lineage_unavailable")?;
            if lineage
                != Some((
                    operation.request.source.id.clone(),
                    operation.request.operation_id.clone(),
                    operation.request.source.browser_build.clone(),
                ))
            {
                return Err("profile_duplicate_disposal_lineage_changed".into());
            }
            let catalog_removed = retained
                .as_ref()
                .is_some_and(|record| record.checkpoint >= Checkpoint::CatalogRemoved);
            match catalog.profiles.get(&profile.id) {
                Some(entry)
                    if !catalog_removed
                        && entry.id == profile.id
                        && entry.user_data_dir == profile.user_data_dir.to_string_lossy()
                        && entry.kind == agent_browser_service_model::BrowserProfileKind::Named => {
                }
                None if catalog_removed => {}
                _ => return Err("profile_duplicate_disposal_catalog_changed".into()),
            }
            for (id, entry) in &catalog.profiles {
                if id == &profile.id {
                    continue;
                }
                let other = Path::new(&entry.user_data_dir);
                for target in [&path, &quarantine] {
                    if other == target
                        || other.starts_with(target)
                        || target.starts_with(other)
                        || fs::canonicalize(other)
                            .ok()
                            .zip(fs::canonicalize(target).ok())
                            .is_some_and(|(a, b)| a == b || a.starts_with(&b) || b.starts_with(&a))
                    {
                        return Err("profile_duplicate_disposal_catalog_alias".into());
                    }
                }
            }
            let has_ledger:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM state_documents WHERE kind='browser_launch_custody')",[],|row|row.get(0)).map_err(|_|"profile_duplicate_disposal_launch_history_unavailable")?;
            let ledger: LaunchLedger = if has_ledger {
                load_document(
                    &connection,
                    "browser_launch_custody",
                    "agent-browser.browser-launch-custody.v1",
                )?
            } else {
                LaunchLedger::default()
            };
            let mut launched = operation.launch.is_none();
            for (id, record) in &ledger.records {
                if record.intent.profile_id != profile.id {
                    continue;
                }
                if record.validate().is_err() || record.intent.intent_id != *id {
                    return Err("profile_duplicate_disposal_launch_history_invalid".into());
                }
                if !record.published && !ledger.reconciled_absent.contains(id) {
                    return Err("profile_duplicate_disposal_launch_pending".into());
                }
                if let Some(identity) = &record.observed_process_identity {
                    if crate::process_identity::recorded_process_is_running(identity)
                        .map_err(|_| "profile_duplicate_disposal_process_unverified")?
                    {
                        return Err("profile_duplicate_disposal_process_present".into());
                    }
                    launched |= record.published
                        && operation.launch.as_ref().is_some_and(|launch| {
                            record.observed_browser_id.as_ref() == Some(&launch.browser_id)
                        });
                } else if record.published {
                    return Err("profile_duplicate_disposal_process_unverified".into());
                }
            }
            if !launched {
                return Err("profile_duplicate_disposal_launch_history_unavailable".into());
            }
            Ok((state, catalog, retained))
        })();
        match result {
            Ok((state, catalog, retained)) => Ok(Self {
                connection,
                state,
                catalog,
                operation: operation.clone(),
                retained,
                active: true,
            }),
            Err(error) => {
                let _ = connection.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }
    fn publish(
        &mut self,
        record: &super::profile_duplicate_disposal::Record,
    ) -> Result<(), String> {
        if self
            .retained
            .as_ref()
            .is_some_and(|old| old.identity != record.identity)
        {
            return Err("profile_duplicate_disposal_receipt_conflict".into());
        }
        super::profile_duplicate_disposal::write_on(&self.connection, record)?;
        for (kind, value) in [
            ("browser_session_state", serde_json::to_string(&self.state)),
            (
                "browser_profile_catalog",
                serde_json::to_string(&self.catalog),
            ),
        ] {
            let changed = self
                .connection
                .execute(
                    "UPDATE state_documents SET json=?1 WHERE kind=?2",
                    params![
                        value.map_err(|_| "profile_duplicate_disposal_encode_failed")?,
                        kind
                    ],
                )
                .map_err(|_| "profile_duplicate_disposal_publication_failed")?;
            if changed != 1 {
                return Err("profile_duplicate_disposal_publication_failed".into());
            }
        }
        self.connection
            .execute_batch("COMMIT")
            .map_err(|_| "profile_duplicate_disposal_commit_readback_required")?;
        self.active = false;
        Ok(())
    }
    pub fn prepare(
        &mut self,
        record: &super::profile_duplicate_disposal::Record,
    ) -> Result<(), String> {
        if self.retained.is_some() {
            return Err("profile_duplicate_disposal_receipt_conflict".into());
        }
        self.state.profile_operation_ids.insert(
            self.operation.request.destination.id.clone(),
            record.fence(),
        );
        self.publish(record)
    }
    pub fn save(
        &mut self,
        record: &super::profile_duplicate_disposal::Record,
    ) -> Result<(), String> {
        self.publish(record)
    }
    pub fn remove_catalog(
        &mut self,
        record: &super::profile_duplicate_disposal::Record,
    ) -> Result<(), String> {
        self.catalog
            .profiles
            .remove(&self.operation.request.destination.id);
        self.state
            .profile_executables
            .remove(&self.operation.request.destination.id);
        self.publish(record)
    }
    pub fn finish(
        &mut self,
        record: &super::profile_duplicate_disposal::Record,
    ) -> Result<(), String> {
        if self
            .state
            .profile_operation_ids
            .get(&self.operation.request.destination.id)
            != Some(&record.fence())
        {
            return Err("profile_duplicate_disposal_foreign_fence".into());
        }
        self.state
            .profile_operation_ids
            .remove(&self.operation.request.destination.id);
        self.publish(record)
    }
    pub fn record_failure(database: &Path, id: &str, failure: &str) -> Result<(), String> {
        let connection = Connection::open_with_flags(
            database,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| "profile_duplicate_disposal_readback_unavailable")?;
        connection
            .busy_timeout(std::time::Duration::ZERO)
            .map_err(|_| "profile_duplicate_disposal_readback_unavailable")?;
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| "profile_duplicate_disposal_readback_busy")?;
        let result = (|| {
            let mut record = super::profile_duplicate_disposal::read_on(&connection, id)?
                .ok_or("profile_duplicate_disposal_receipt_missing")?;
            if record.checkpoint != super::profile_duplicate_disposal::Checkpoint::Completed {
                record.failure = Some(failure.into());
                super::profile_duplicate_disposal::write_on(&connection, &record)?;
            }
            connection
                .execute_batch("COMMIT")
                .map_err(|_| "profile_duplicate_disposal_commit_readback_required".into())
        })();
        if result.is_err() {
            let _ = connection.execute_batch("ROLLBACK");
        }
        result
    }
}
impl Drop for CopyDisposalCustody {
    fn drop(&mut self) {
        if self.active {
            let _ = self.connection.execute_batch("ROLLBACK");
        }
    }
}
