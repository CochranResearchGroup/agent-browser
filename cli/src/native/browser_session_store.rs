//! Independent durable storage for the ordinary Browser Session Manager path.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(test)]
use agent_browser_service_model::BrowserRecoveryPhase;
use agent_browser_service_model::{
    decide_browser_recovery, detach_remote_view_presentation, record_browser_recovery_failure,
    record_browser_recovery_observed_live, record_browser_recovery_success,
    release_remote_view_presentation, retain_remote_view_presentation, BrowserProfileCatalog,
    BrowserProfileCatalogDiagnostic, BrowserRecoveryAdmissionPolicy, BrowserRecoveryDecision,
    BrowserRecoveryDemand, BrowserRecoveryState, BrowserSessionState, OldBrowserUsability,
    RemoteViewDesktopPresentationBinding, RemoteViewJoinedReleaseOutcome,
    RemoteViewPresentationRetention, BROWSER_PROFILE_CATALOG_SCHEMA_V1,
    BROWSER_SESSION_STATE_SCHEMA_V1,
};
use rusqlite::{backup, params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::service_store::default_service_state_path;

#[path = "browser_mode_inspection.rs"]
pub(crate) mod mode_inspection;
#[path = "browser_profile_build_store.rs"]
pub(crate) mod profile_build;
#[path = "browser_terminal_history.rs"]
mod terminal_history;
#[path = "browser_workspace_observation.rs"]
mod workspace_observation;

const BROWSER_SESSION_STATE_FILENAME: &str = "browser-session-state.json";
const BROWSER_PROFILE_CATALOG_FILENAME: &str = "browser-profile-catalog.json";
const BROWSER_RUNTIME_DATABASE_FILENAME: &str = "runtime.sqlite3";
// Schema 2 marks durable idle-retirement obligations. A schema-1-only reader
// must not silently ignore those obligations. Existing schema 1 is read without
// migration; the first custody claim promotes it atomically with its record.
const BROWSER_RUNTIME_DATABASE_MAX_SCHEMA: i64 = 4;
const BROWSER_RUNTIME_BACKUP_DIRECTORY: &str = "browser-runtime-backups";
const BROWSER_RUNTIME_BACKUP_CURRENT: &str = "runtime.current.sqlite3";
const BROWSER_RUNTIME_BACKUP_PREVIOUS: &str = "runtime.previous.sqlite3";
const BROWSER_RUNTIME_BACKUP_MANIFEST: &str = "manifest.json";
pub(super) const SESSION_STATE_DOCUMENT: &str = "browser_session_state";
const PROFILE_CATALOG_DOCUMENT: &str = "browser_profile_catalog";
const BROWSER_RECOVERY_REGISTRY_DOCUMENT: &str = "browser_recovery_registry";
const BROWSER_RECOVERY_REGISTRY_SCHEMA_V1: &str = "agent-browser.browser-recovery-registry.v1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BrowserRecoveryRegistry {
    states: BTreeMap<String, BrowserRecoveryState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserRuntimeMigrationReceipt {
    pub(crate) imported_source_count: usize,
    pub(crate) archive_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserRuntimeBackupManifest {
    pub(crate) schema_version: String,
    pub(crate) created_at: String,
    pub(crate) database_sha256: String,
    pub(crate) database_bytes: u64,
    pub(crate) integrity_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) previous_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserRuntimeBackupStatus {
    pub(crate) state: &'static str,
    pub(crate) sha256: Option<String>,
}

/// Transactional authority for the P220-owned session and profile aggregates.
///
/// Legacy JSON is read only at the one-time migration boundary. Once the
/// database exists it remains authoritative, including when opening it fails.
pub(crate) struct BrowserSessionSqliteStore {
    connection: Connection,
}

impl BrowserSessionSqliteStore {
    #[cfg(test)]
    pub(super) fn launch_custody_fixture(
        connection: Connection,
        initialize: bool,
    ) -> Result<Self, String> {
        if initialize {
            initialize_runtime_schema(&connection)?;
            save_document(
                &connection,
                SESSION_STATE_DOCUMENT,
                BROWSER_SESSION_STATE_SCHEMA_V1,
                &BrowserSessionState::default(),
            )?;
        } else {
            validate_runtime_schema(&connection)?;
        }
        Ok(Self { connection })
    }

    /// The operation adapter uses this exact Browser Runtime connection for
    /// durable request custody; it creates no desktop allocation authority.
    pub(super) fn remote_view_mutation_connection(&mut self) -> &mut Connection {
        &mut self.connection
    }
    pub(crate) fn default_sqlite_path() -> Result<PathBuf, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Ok(service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME))
    }

    pub(crate) fn default_sqlite() -> Result<Self, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = Self::default_sqlite_path()?
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?
            .to_path_buf();
        Self::open_or_migrate(&service_directory, &legacy_state_path).map(|(store, _)| store)
    }

    pub(crate) fn open(service_directory: &Path) -> Result<Self, String> {
        let connection =
            open_runtime_connection(&service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME))?;
        validate_runtime_schema(&connection)?;
        Ok(Self { connection })
    }

    /// Open an existing Browser Runtime database without migrations or writes.
    pub(crate) fn open_read_only(path: &Path) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("browser_runtime_database_read_only_unavailable:{error}"))?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err("browser_runtime_database_read_only_invalid_type".to_string());
        }
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("browser_runtime_database_read_only_open_failed:{error}"))?;
        validate_runtime_schema(&connection)?;
        Ok(Self { connection })
    }

    /// Read the current managed inventory without creating or migrating runtime state.
    /// An absent database is an empty inventory; invalid existing state is an error.
    pub(crate) fn default_session_state_read_only() -> Result<Option<BrowserSessionState>, String> {
        let path = Self::default_sqlite_path()?;
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("browser_runtime_inventory_readback_required".into()),
            Ok(_) => Self::open_read_only(&path)?.load_session_state().map(Some),
        }
    }

    /// Inspect one existing logical task without acquiring a daemon or browser.
    /// The same per-browser evidence is exposed in CLI/HTTP workspace inventory.
    pub(crate) fn inspect_task_mode(&self, task_id: &str) -> Result<serde_json::Value, String> {
        self.inspect_task_mode_with(task_id, |state, browser| self.inspect_mode(state, browser))
    }

    fn inspect_task_mode_with(
        &self,
        task_id: &str,
        inspect: impl FnOnce(
            &BrowserSessionState,
            &agent_browser_service_model::ManagedBrowserInstance,
        ) -> serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let state = self.load_session_state()?;
        let task = state
            .sessions
            .get(task_id)
            .ok_or("browser_task_not_found:refresh inventory")?;
        let browser = state
            .browsers
            .get(&task.browser_id)
            .filter(|browser| browser.profile_id == task.profile_id)
            .ok_or("browser_task_owner_unavailable:refresh inventory")?;
        let mut inspection = inspect(&state, browser);
        let current = self.load_session_state()?;
        if !current.sessions.get(task_id).is_some_and(|current_task| {
            current_task.id == task.id
                && current_task.created_at_ms == task.created_at_ms
                && current_task.browser_id == task.browser_id
                && current_task.profile_id == task.profile_id
        }) || current.browsers.get(&browser.id) != Some(browser)
        {
            return Err("browser_task_changed_during_observation:refresh inventory".into());
        }
        inspection["taskId"] = serde_json::json!(task_id);
        Ok(inspection)
    }

    pub(crate) fn original_browser_identity(
        &self,
        state: &BrowserSessionState,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> Option<agent_browser_service_model::RecordedProcessIdentity> {
        let attached = state.app_browser_attachments.get(&browser.id);
        let primary = if let Some(attached) = attached {
            (attached.profile_id == browser.profile_id
                && attached.request.process.pid == browser.pid
                && attached.request.cdp_endpoint == browser.cdp_endpoint
                && browser.desktop.as_ref() == Some(&attached.desktop))
            .then(|| attached.request.process.clone())
        } else {
            super::browser_launch_custody_store::published_process_identity(
                &self.connection,
                browser,
            )
            .ok()
            .flatten()
        };
        let adopted = state.adopted_browser_process_identities.get(&browser.id);
        if attached.is_some() && primary.is_none()
            || adopted.is_some_and(|identity| {
                identity.process.pid != browser.pid
                    || identity.profile_id != browser.profile_id
                    || identity.cdp_endpoint != browser.cdp_endpoint
            })
            || adopted.is_some() && !state.shared_browser_ids.contains(&browser.id)
            || primary
                .as_ref()
                .zip(adopted)
                .is_some_and(|(left, right)| left != &right.process)
        {
            return None;
        }
        primary.or_else(|| adopted.map(|identity| identity.process.clone()))
    }

    fn inspect_mode(
        &self,
        state: &BrowserSessionState,
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> serde_json::Value {
        let attached = state.app_browser_attachments.get(&browser.id);
        let original = self.original_browser_identity(state, browser);
        let mut result = mode_inspection::inspect(
            browser,
            original.as_ref(),
            if attached.is_some() {
                "app_provided"
            } else {
                "managed"
            },
            crate::process_identity::observe_process,
        );
        if self
            .load_session_state()
            .ok()
            .and_then(|current| current.browsers.get(&browser.id).cloned())
            .as_ref()
            != Some(browser)
        {
            result["mode"] = serde_json::json!("unverified");
            result["reason"] = serde_json::json!("owner_changed_during_observation");
            result["capabilities"]["debugging"] = serde_json::json!("unverified");
        }
        result
    }

    fn enrich_modes(status: &mut serde_json::Value) {
        let Some((store, state)) = Self::default_sqlite_path()
            .ok()
            .and_then(|path| Self::open_read_only(&path).ok())
            .and_then(|store| store.load_session_state().ok().map(|state| (store, state)))
        else {
            return;
        };
        for browser in state.browsers.values() {
            let Some(published) = status
                .pointer_mut("/workspace/browsers")
                .and_then(|browsers| browsers.get_mut(&browser.id))
            else {
                continue;
            };
            if published["pid"].as_u64() == Some(u64::from(browser.pid))
                && published["profileId"] == browser.profile_id
            {
                published["browserMode"] = store.inspect_mode(&state, browser);
            }
        }
    }

    /// Produce a redacted read-only aggregate for Service status and install
    /// doctor. Failure details collapse to stable codes so filesystem paths do
    /// not cross the public status boundary.
    pub(crate) fn default_operational_status_read_only() -> serde_json::Value {
        match Self::default_sqlite_path() {
            Ok(path) => Self::operational_status_read_only(&path),
            Err(error) => unavailable_operational_status(Path::new("."), &error),
        }
    }

    /// Native rail projection reads one owner snapshot, without launch admission,
    /// integrity scans, backup hashing, migrations or Service State reconciliation.
    pub(crate) async fn default_observed_workspace_status() -> Result<serde_json::Value, String> {
        let (state, catalog) = match fs::symlink_metadata(Self::default_sqlite_path()?) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                BrowserSessionState::default(),
                BrowserProfileCatalog::default(),
            ),
            Err(_) => return Err("browser_workspace_inventory_unavailable".into()),
            Ok(_) => {
                Self::open_read_only(&Self::default_sqlite_path()?)?.load_workspace_snapshot()?
            }
        };
        let mut status = serde_json::json!({
            "schemaVersion": "agent-browser.runtime-operational-status.v1",
            "state": "available",
            "workspace": workspace_projection(&state, &catalog)?,
        });
        workspace_observation::enrich(state, &mut status).await;
        Self::enrich_modes(&mut status);
        Ok(status)
    }

    /// Service inventory adds bounded live title/URL reads after releasing SQLite.
    /// Install doctor keeps its purely local read-only aggregate.
    pub(crate) async fn default_observed_operational_status() -> serde_json::Value {
        let mut status = Self::default_operational_status_read_only();
        let state = Self::default_sqlite_path()
            .ok()
            .and_then(|path| Self::open_read_only(&path).ok())
            .and_then(|store| store.load_session_state().ok());
        if let Some(state) = state {
            workspace_observation::enrich(state, &mut status).await;
        }
        Self::enrich_modes(&mut status);
        status
    }

    fn operational_status_read_only(path: &Path) -> serde_json::Value {
        let database_path = path.to_path_buf();
        let admission_path = database_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let launch_admission = super::browser_launch_admission::observe_browser_launch_admission(
            &admission_path,
            None,
        );
        let result = (|| -> Result<serde_json::Value, String> {
            let path = database_path;
            let store = Self::open_read_only(&path)?;
            sqlite_integrity_state(&store.connection)?;
            let migration = store.migration_receipt()?;
            let archive_state = match fs::symlink_metadata(&migration.archive_directory) {
                Ok(metadata)
                    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() =>
                {
                    "present"
                }
                Ok(_) => "invalid",
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing",
                Err(_) => "unavailable",
            };
            let backup = store.backup_status()?;
            let (managed, catalog) = store.load_workspace_snapshot()?;
            Ok(serde_json::json!({
                "schemaVersion": "agent-browser.runtime-operational-status.v1",
                "state": "available",
                // Owner-published inventory is not provider readiness or an OS
                // liveness claim. Debugger endpoints and launch environments
                // remain private to the runtime owner.
                "workspace": workspace_projection(&managed, &catalog)?,
                "launchAdmission": launch_admission,
                "storage": {
                    "integrityState": "ok",
                    "databaseBytes": regular_file_bytes(&path)?,
                    "backupState": backup.state,
                    "backupSha256": backup.sha256,
                    "terminalHistory": terminal_history::status(&store.connection)?,
                },
                "migration": {
                    "importedSourceCount": migration.imported_source_count,
                    "archiveState": archive_state,
                },
            }))
        })();
        result.unwrap_or_else(|error| unavailable_operational_status(&admission_path, &error))
    }

    pub(crate) fn open_or_migrate(
        service_directory: &Path,
        legacy_service_state_path: &Path,
    ) -> Result<(Self, BrowserRuntimeMigrationReceipt), String> {
        let path = service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        if path.exists() {
            let store = Self::open(service_directory)?;
            let receipt = store.migration_receipt()?;
            set_archive_read_only(&receipt.archive_directory)?;
            return Ok((store, receipt));
        }
        prepare_private_parent(&path)?;
        let json_store = BrowserSessionJsonStore::new(service_directory);
        let session = json_store.load_session_state()?;
        let catalog_load = json_store.load_or_import_profile_catalog(legacy_service_state_path)?;
        let mut sources = Vec::new();
        for source in [
            json_store.session_state_path.as_path(),
            json_store.profile_catalog_path.as_path(),
            legacy_service_state_path,
        ] {
            if let Some(bytes) = read_optional_migration_source(source)? {
                sources.push((source, bytes));
            }
        }
        let migration_id = migration_digest(&sources);
        let archive_directory =
            service_directory.join(format!("migration-archive-{}", &migration_id[..16]));
        create_migration_archive(&archive_directory, &sources)?;
        let staged = path.with_extension(format!("sqlite3.migrating-{}", uuid::Uuid::new_v4()));
        let migration_result = (|| -> Result<(), String> {
            let mut connection = open_runtime_connection(&staged)?;
            initialize_runtime_schema(&connection)?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| format!("browser_runtime_migration_begin_failed:{error}"))?;
            save_document(
                &transaction,
                SESSION_STATE_DOCUMENT,
                BROWSER_SESSION_STATE_SCHEMA_V1,
                &session,
            )?;
            save_document(
                &transaction,
                PROFILE_CATALOG_DOCUMENT,
                BROWSER_PROFILE_CATALOG_SCHEMA_V1,
                &catalog_load.catalog,
            )?;
            save_document(
                &transaction,
                BROWSER_RECOVERY_REGISTRY_DOCUMENT,
                BROWSER_RECOVERY_REGISTRY_SCHEMA_V1,
                &BrowserRecoveryRegistry::default(),
            )?;
            transaction
                .execute(
                    "INSERT INTO runtime_metadata(key, value) VALUES ('migration_archive', ?1)",
                    params![archive_directory.to_string_lossy().as_ref()],
                )
                .and_then(|_| {
                    transaction.execute(
                        "INSERT INTO runtime_metadata(key, value) VALUES ('imported_source_count', ?1)",
                        params![sources.len().to_string()],
                    )
                })
                .map_err(|error| format!("browser_runtime_migration_receipt_failed:{error}"))?;
            transaction
                .commit()
                .map_err(|error| format!("browser_runtime_migration_commit_failed:{error}"))?;
            connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|error| format!("browser_runtime_migration_checkpoint_failed:{error}"))?;
            drop(connection);
            set_private_file(&staged)?;
            atomic_replace(&staged, &path)
                .map_err(|error| error.replace("browser_session_json", "browser_runtime_database"))
        })();
        if migration_result.is_err() {
            remove_sqlite_files(&staged);
        }
        migration_result?;
        remove_sqlite_sidecars(&staged);
        set_archive_read_only(&archive_directory)?;
        let store = Self::open(service_directory)?;
        let receipt = store.migration_receipt()?;
        Ok((store, receipt))
    }

    fn migration_receipt(&self) -> Result<BrowserRuntimeMigrationReceipt, String> {
        let read = |key: &str| {
            self.connection
                .query_row(
                    "SELECT value FROM runtime_metadata WHERE key = ?1",
                    params![key],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|error| format!("browser_runtime_migration_receipt_missing:{error}"))
        };
        let archive_directory = PathBuf::from(read("migration_archive")?);
        // Earlier runtime databases recorded each source instead of a summary
        // key. Read that authoritative evidence without reimporting JSON or
        // rewriting the database. A malformed present summary still fails.
        let summary: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM runtime_metadata WHERE key = 'imported_source_count'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("browser_runtime_migration_receipt_failed:{error}"))?;
        let imported_source_count = match summary {
            Some(value) => value
                .parse::<usize>()
                .map_err(|error| format!("browser_runtime_migration_receipt_invalid:{error}"))?,
            None => self
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM migration_sources WHERE imported = 1",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| format!("browser_runtime_migration_receipt_missing:{error}"))
                .and_then(|count| {
                    usize::try_from(count).map_err(|error| {
                        format!("browser_runtime_migration_receipt_invalid:{error}")
                    })
                })?,
        };
        Ok(BrowserRuntimeMigrationReceipt {
            imported_source_count,
            archive_directory,
        })
    }

    pub(crate) fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        load_document(
            &self.connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )
    }

    /// Read archived terminal evidence without loading all history into the host.
    pub(crate) fn archived_terminal_history(
        &self,
        kind: &str,
        limit: usize,
        before: Option<i64>,
    ) -> Result<serde_json::Value, String> {
        terminal_history::page(&self.connection, kind, limit, before)
    }

    #[cfg(test)]
    pub(crate) fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String> {
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        save_document(
            &self.connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )
    }

    /// Publish only against the state loaded by this host. A concurrent
    /// retention or session writer must not be erased by a stale host snapshot.
    pub(crate) fn compare_and_save_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), String> {
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err("browser_session_state_schema_unsupported".into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "browser_session_publication_begin_failed".to_string())?;
        let current: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if current != *expected {
            return Err("browser_session_publication_conflict".into());
        }
        super::browser_idle_retirement_store::reject_retirement_publication(
            &transaction,
            &current,
            state,
            &[],
            &[],
        )
        .map_err(|_| "browser_session_publication_idle_retirement_fenced".to_string())?;
        super::browser_launch_custody_store::reject_fenced_browser_publication(
            &transaction,
            expected,
            state,
        )
        .map_err(|_| "browser_session_publication_release_fenced".to_string())?;

        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )?;
        transaction
            .commit()
            .map_err(|_| "browser_session_publication_commit_failed".to_string())
    }

    pub(crate) fn retain_remote_view_presentation(
        &mut self,
        browser_id: &str,
        session_id: &str,
        tab_id: &str,
        binding: &RemoteViewDesktopPresentationBinding,
    ) -> Result<RemoteViewPresentationRetention, String> {
        self.mutate_session_state(|state| {
            retain_remote_view_presentation(state, browser_id, session_id, tab_id, binding)
        })
    }

    pub(crate) fn release_remote_view_presentation(
        &mut self,
        browser_id: &str,
        release: &RemoteViewJoinedReleaseOutcome,
    ) -> Result<RemoteViewPresentationRetention, String> {
        self.mutate_session_state(|state| {
            release_remote_view_presentation(state, browser_id, release)
        })
    }

    pub(crate) fn detach_remote_view_presentation(
        &mut self,
        browser_id: &str,
    ) -> Result<usize, String> {
        self.mutate_session_state(|state| detach_remote_view_presentation(state, browser_id))
    }

    pub(crate) fn mutate_session_state<T>(
        &mut self,
        mutate: impl FnOnce(&mut BrowserSessionState) -> Result<T, String>,
    ) -> Result<T, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_session_mutation_begin_failed:{error}"))?;
        let mut state: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let previous = state.clone();
        let result = mutate(&mut state)?;
        super::browser_idle_retirement_store::reject_retirement_publication(
            &transaction,
            &previous,
            &state,
            &[],
            &[],
        )
        .map_err(|_| "browser_session_publication_idle_retirement_fenced".to_string())?;

        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &state,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("browser_session_mutation_commit_failed:{error}"))?;
        Ok(result)
    }

    /// One read transaction prevents profile promotion from splitting display state.
    pub(crate) fn load_workspace_snapshot(
        &self,
    ) -> Result<(BrowserSessionState, BrowserProfileCatalog), String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| "browser_workspace_snapshot_failed")?;
        let state = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let catalog = load_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )?;
        transaction
            .commit()
            .map_err(|_| "browser_workspace_snapshot_failed")?;
        Ok((state, catalog))
    }

    /// Edit display metadata and one-way retention in one owner transaction.
    /// Caller retains the Service State effect guard shared by profile cleanup.
    /// Directory, stable ID, tabs, sessions and browser identities never change.
    pub(crate) fn update_profile_properties(
        &mut self,
        profile_id: &str,
        expected_name: &str,
        expected_kind: agent_browser_service_model::BrowserProfileKind,
        name: &str,
        promote: bool,
    ) -> Result<serde_json::Value, String> {
        use agent_browser_service_model::BrowserProfileKind;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
            return Err("profile_properties_name_invalid".into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "profile_properties_begin_failed")?;
        let mut state: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let mut catalog: BrowserProfileCatalog = load_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )?;
        let mut profile = state
            .disposable_profiles
            .get(profile_id)
            .map(|entry| entry.profile.clone())
            .or_else(|| catalog.profiles.get(profile_id).cloned())
            .ok_or("profile_properties_missing")?;
        if profile.name != expected_name || profile.kind != expected_kind {
            return Err("profile_properties_changed".into());
        }
        if !std::path::Path::new(&profile.user_data_dir).is_dir() {
            return Err("profile_properties_directory_unavailable".into());
        }
        super::browser_idle_retirement_store::reject_profile_metadata_mutation(
            &transaction,
            profile_id,
        )
        .map_err(|_| "profile_properties_retirement_pending")?;
        super::browser_launch_custody_store::reject_profile_metadata_mutation(
            &transaction,
            profile_id,
        )
        .map_err(|_| "profile_properties_launch_pending")?;
        let previous = state.clone();
        profile.name = name.to_owned();
        if promote {
            profile.kind = BrowserProfileKind::Named;
        }
        if profile.kind == BrowserProfileKind::Named {
            state.disposable_profiles.remove(profile_id);
            catalog
                .profiles
                .insert(profile_id.to_owned(), profile.clone());
            // Recreated handoffs must select this same now-durable profile.
            for link in state.remote_view_tab_handoffs.values_mut() {
                if let Some(resume) = link
                    .resume
                    .as_mut()
                    .filter(|resume| resume.profile_id == profile_id)
                {
                    resume.disposable_policy_id = None;
                }
            }
        } else if let Some(allocation) = state.disposable_profiles.get_mut(profile_id) {
            allocation.profile = profile.clone();
        } else {
            // One-time legacy profiles can live only in the imported catalog.
            catalog
                .profiles
                .insert(profile_id.to_owned(), profile.clone());
        }
        super::browser_idle_retirement_store::reject_retirement_publication(
            &transaction,
            &previous,
            &state,
            &[],
            &[],
        )
        .map_err(|_| "profile_properties_retirement_pending")?;
        super::browser_launch_custody_store::reject_fenced_browser_publication(
            &transaction,
            &previous,
            &state,
        )
        .map_err(|_| "profile_properties_launch_pending")?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &state,
        )?;
        save_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
            &catalog,
        )?;
        transaction
            .commit()
            .map_err(|_| "profile_properties_commit_failed")?;
        Ok(serde_json::json!({"id":profile.id,"name":profile.name,"kind":profile.kind}))
    }

    pub(crate) fn load_profile_catalog(&self) -> Result<BrowserProfileCatalogLoad, String> {
        let catalog = load_document(
            &self.connection,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )?;
        Ok(BrowserProfileCatalogLoad {
            catalog,
            diagnostics: Vec::new(),
            imported_legacy_profiles: false,
        })
    }

    /// Publish new app profile visibility and borrowed browser/task custody in
    /// one SQLite transaction; either CAS/fence failure leaves both unchanged.
    pub(crate) fn compare_and_save_app_attachment(
        &mut self,
        expected: &BrowserSessionState,
        next: &BrowserSessionState,
        expected_catalog: &BrowserProfileCatalog,
        next_catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        validate_catalog_schema(next_catalog)?;
        if next.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err("browser_session_state_schema_unsupported".into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "browser_attachment_publication_begin_failed")?;
        let current: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let catalog: BrowserProfileCatalog = load_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )?;
        if current != *expected || catalog != *expected_catalog {
            return Err("browser_attachment_publication_conflict".into());
        }
        super::browser_idle_retirement_store::reject_retirement_publication(
            &transaction,
            &current,
            next,
            &[],
            &[],
        )
        .map_err(|_| "browser_attachment_publication_retirement_fenced")?;
        super::browser_launch_custody_store::reject_fenced_browser_publication(
            &transaction,
            expected,
            next,
        )
        .map_err(|_| "browser_attachment_publication_release_fenced")?;
        save_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
            next_catalog,
        )?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            next,
        )?;
        transaction
            .commit()
            .map_err(|_| "browser_attachment_publication_commit_failed".into())
    }

    /// Registering a profile cannot overwrite concurrent display-name or retention edits.
    pub(crate) fn compare_and_save_profile_catalog(
        &self,
        expected: &BrowserProfileCatalog,
        catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| "browser_profile_catalog_begin_failed")?;
        let current: BrowserProfileCatalog = load_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )?;
        if current != *expected {
            return Err("browser_profile_catalog_publication_conflict".into());
        }
        save_document(
            &transaction,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
            catalog,
        )?;
        transaction
            .commit()
            .map_err(|_| "browser_profile_catalog_commit_failed".into())
    }

    pub(crate) fn save_profile_catalog(
        &self,
        catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        validate_catalog_schema(catalog)?;
        save_document(
            &self.connection,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
            catalog,
        )
    }

    pub(crate) fn admit_browser_recovery(
        &mut self,
        browser_id: &str,
        demand: BrowserRecoveryDemand,
        old_browser: OldBrowserUsability,
        now_ms: u64,
        policy: BrowserRecoveryAdmissionPolicy,
    ) -> Result<BrowserRecoveryDecision, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_recovery_begin_failed:{error}"))?;
        let mut registry: BrowserRecoveryRegistry = load_optional_document(
            &transaction,
            BROWSER_RECOVERY_REGISTRY_DOCUMENT,
            BROWSER_RECOVERY_REGISTRY_SCHEMA_V1,
        )?;
        let decision = decide_browser_recovery(
            registry.states.get(browser_id),
            browser_id,
            demand,
            old_browser,
            now_ms,
            policy,
        )?;
        let next = match &decision {
            BrowserRecoveryDecision::AdmitReplacement { state }
            | BrowserRecoveryDecision::Exhausted { state } => Some(state.clone()),
            _ => None,
        };
        if let Some(next) = next {
            registry.states.insert(browser_id.to_string(), next);
            save_document(
                &transaction,
                BROWSER_RECOVERY_REGISTRY_DOCUMENT,
                BROWSER_RECOVERY_REGISTRY_SCHEMA_V1,
                &registry,
            )?;
        }
        transaction
            .commit()
            .map_err(|error| format!("browser_recovery_commit_failed:{error}"))?;
        Ok(decision)
    }

    pub(crate) fn record_browser_recovery_observed_live(
        &mut self,
        browser_id: &str,
        generation: u64,
    ) -> Result<BrowserRecoveryState, String> {
        self.update_browser_recovery(browser_id, |current| {
            record_browser_recovery_observed_live(current, generation)
        })
    }

    pub(crate) fn record_browser_recovery_failure(
        &mut self,
        browser_id: &str,
        generation: u64,
        failed_at_ms: u64,
        policy: BrowserRecoveryAdmissionPolicy,
    ) -> Result<BrowserRecoveryState, String> {
        self.update_browser_recovery(browser_id, |current| {
            record_browser_recovery_failure(current, generation, failed_at_ms, policy)
        })
    }

    pub(crate) fn record_browser_recovery_success(
        &mut self,
        browser_id: &str,
        generation: u64,
        recovered_at_ms: u64,
    ) -> Result<BrowserRecoveryState, String> {
        self.update_browser_recovery(browser_id, |current| {
            record_browser_recovery_success(current, generation, recovered_at_ms)
        })
    }

    pub(crate) fn load_browser_recovery_state(
        &self,
        browser_id: &str,
    ) -> Result<Option<BrowserRecoveryState>, String> {
        let registry: BrowserRecoveryRegistry = load_optional_document(
            &self.connection,
            BROWSER_RECOVERY_REGISTRY_DOCUMENT,
            BROWSER_RECOVERY_REGISTRY_SCHEMA_V1,
        )?;
        Ok(registry.states.get(browser_id).cloned())
    }

    fn update_browser_recovery(
        &mut self,
        browser_id: &str,
        transition: impl FnOnce(&BrowserRecoveryState) -> Result<BrowserRecoveryState, String>,
    ) -> Result<BrowserRecoveryState, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_recovery_begin_failed:{error}"))?;
        let mut registry: BrowserRecoveryRegistry = load_optional_document(
            &transaction,
            BROWSER_RECOVERY_REGISTRY_DOCUMENT,
            BROWSER_RECOVERY_REGISTRY_SCHEMA_V1,
        )?;
        let current = registry
            .states
            .get(browser_id)
            .ok_or_else(|| format!("browser_recovery_state_missing:{browser_id}"))?;
        let next = transition(current)?;
        registry.states.insert(browser_id.to_string(), next.clone());
        save_document(
            &transaction,
            BROWSER_RECOVERY_REGISTRY_DOCUMENT,
            BROWSER_RECOVERY_REGISTRY_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("browser_recovery_commit_failed:{error}"))?;
        Ok(next)
    }

    pub(crate) fn create_verified_backup(&self) -> Result<BrowserRuntimeBackupManifest, String> {
        let database_path = self.database_path()?;
        let backup_directory = database_path
            .parent()
            .ok_or_else(|| "browser_runtime_database_parent_missing".to_string())?
            .join(BROWSER_RUNTIME_BACKUP_DIRECTORY);
        prepare_private_parent(&backup_directory.join("placeholder"))?;
        let current = backup_directory.join(BROWSER_RUNTIME_BACKUP_CURRENT);
        let previous = backup_directory.join(BROWSER_RUNTIME_BACKUP_PREVIOUS);
        let manifest_path = backup_directory.join(BROWSER_RUNTIME_BACKUP_MANIFEST);
        let staged =
            backup_directory.join(format!("runtime.staged-{}.sqlite3", uuid::Uuid::new_v4()));
        let result = (|| -> Result<BrowserRuntimeBackupManifest, String> {
            let mut destination = Connection::open(&staged)
                .map_err(|error| format!("browser_runtime_backup_stage_open_failed:{error}"))?;
            backup::Backup::new(&self.connection, &mut destination)
                .and_then(|backup| backup.run_to_completion(128, Duration::from_millis(10), None))
                .map_err(|error| format!("browser_runtime_backup_copy_failed:{error}"))?;
            drop(destination);
            verify_sqlite_backup(&staged)?;
            set_private_file(&staged)?;
            let previous_sha256 = read_optional_backup_manifest(&manifest_path)?
                .map(|manifest| manifest.database_sha256);
            let manifest = BrowserRuntimeBackupManifest {
                schema_version: "agent-browser.runtime-backup-manifest.v1".to_string(),
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                database_sha256: sha256_file(&staged)?,
                database_bytes: regular_file_bytes(&staged)?,
                integrity_state: "ok".to_string(),
                previous_sha256,
            };
            if current.exists() {
                atomic_replace(&current, &previous)?;
            }
            atomic_replace(&staged, &current)?;
            write_private_json_atomic(&manifest_path, &manifest)?;
            Ok(manifest)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&staged);
        }
        result
    }

    pub(crate) fn backup_status(&self) -> Result<BrowserRuntimeBackupStatus, String> {
        let database_path = self.database_path()?;
        let directory = database_path
            .parent()
            .ok_or_else(|| "browser_runtime_database_parent_missing".to_string())?
            .join(BROWSER_RUNTIME_BACKUP_DIRECTORY);
        let current = directory.join(BROWSER_RUNTIME_BACKUP_CURRENT);
        let manifest =
            read_optional_backup_manifest(&directory.join(BROWSER_RUNTIME_BACKUP_MANIFEST))?;
        match (current.is_file(), manifest) {
            (false, None) => Ok(BrowserRuntimeBackupStatus {
                state: "missing",
                sha256: None,
            }),
            (true, Some(manifest)) => {
                let digest = sha256_file(&current)?;
                let verified = manifest.schema_version
                    == "agent-browser.runtime-backup-manifest.v1"
                    && manifest.database_sha256 == digest
                    && manifest.database_bytes == regular_file_bytes(&current)?
                    && manifest.integrity_state == "ok"
                    && verify_sqlite_backup(&current).is_ok();
                Ok(BrowserRuntimeBackupStatus {
                    state: if verified { "verified" } else { "gap" },
                    sha256: Some(digest),
                })
            }
            _ => Ok(BrowserRuntimeBackupStatus {
                state: "gap",
                sha256: None,
            }),
        }
    }

    pub(crate) fn database_path(&self) -> Result<PathBuf, String> {
        self.connection
            .path()
            .map(PathBuf::from)
            .ok_or_else(|| "browser_runtime_database_path_unavailable".to_string())
    }
}

/// Display fields only. Private resume capsules, CDP endpoints and provider
/// correlation tokens never enter the native workspace response.
fn workspace_projection(
    managed: &BrowserSessionState,
    catalog: &BrowserProfileCatalog,
) -> Result<serde_json::Value, String> {
    let referenced_profiles = managed
        .browsers
        .values()
        .map(|browser| browser.profile_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let browsers: std::collections::BTreeMap<_, _> = managed.browsers.iter().map(|(id, browser)| {
                (id.clone(), serde_json::json!({
                    "id":id,"profileId":browser.profile_id,"pid":browser.pid,
                    "desktop":browser.desktop,"activeSessionIds":browser.active_session_ids,
                    "state":if managed.idle_closed_browsers.contains_key(id) { "sleeping" } else { "reported" }
                }))
            }).collect();
    let tabs: std::collections::BTreeMap<_, _> = managed.tabs.iter().map(|(id, tab)| {
                let destination = managed.navigation_history.iter().filter(|entry| entry.tab_id == *id)
                    .max_by_key(|entry| entry.visited_at_ms).map(|entry| entry.url.as_str());
                let metadata = managed.tab_metadata.get(id).or_else(|| {
                    managed.remote_view_tab_handoffs.values()
                        .filter(|link| link.tab_id == *id && link.session_id == tab.session_id)
                        .filter_map(|link| link.resume.as_ref())
                        .filter(|resume| managed.sessions.get(&tab.session_id).is_some_and(|session| session.profile_id == resume.profile_id))
                        .find_map(|resume| resume.task_metadata.as_ref())
                });
                (id.clone(), serde_json::json!({
                    "id":id,"targetId":tab.target_id,"browserId":tab.browser_id,"sessionId":tab.session_id,
                    "createdAtMs":tab.created_at_ms,"url":destination,"taskMetadata":metadata,
                    "lifetime":managed.tab_lifetimes.get(id),"pendingHuman":managed.pending_human_tabs.get(id)
                }))
            }).collect();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "browser_runtime_clock_readback_required")?
        .as_millis() as u64;
    // Deliberately whitelist display fields. A resume capsule, issued
    // provider view and native correlation are private owner authority.
    let handoffs: std::collections::BTreeMap<_, _> = managed.remote_view_tab_handoffs.iter().map(|(id, link)| {
                let session = managed.sessions.get(&link.session_id);
                let tab = managed.tabs.get(&link.tab_id);
                let resume = link.resume.as_ref();
                let metadata = managed.tab_metadata.get(&link.tab_id)
                    .or_else(|| resume.and_then(|intent| intent.task_metadata.as_ref()));
                let lifetime = managed.tab_lifetimes.get(&link.tab_id)
                    .or_else(|| resume.and_then(|intent| intent.tab_lifetime.as_ref()));
                (id.clone(), serde_json::json!({
                    "handoffId":id,"handoffUrl":format!("/remote-view/{id}"),
                    "sessionId":link.session_id,"tabId":link.tab_id,
                    "sessionName":session.map(|session| session.name.as_str()).or_else(|| resume.map(|intent| intent.session_name.as_str())),
                    "profileId":session.map(|session| session.profile_id.as_str()).or_else(|| resume.map(|intent| intent.profile_id.as_str())),
                    "browserId":tab.map(|tab| tab.browser_id.as_str()),
                    "createdAtMs":link.created_at_ms,"expiresAtMs":link.expires_at_ms,
                    "linkState":if link.check_link_at(now_ms).is_ok() {"active"} else {"expired"},
                    "targetState":if tab.is_some() {"reported"} else if link.target_expired {"expired"} else {"ended"},
                    "taskMetadata":metadata,"tabLifetime":lifetime,
                    "pendingHuman":managed.pending_human_tabs.get(&link.tab_id)
                }))
            }).collect();
    Ok(serde_json::json!({
        "schemaVersion":"agent-browser.managed-workspace.v1",
        "source":"managed-owner","browsers":browsers,
        "sessions":managed.sessions,"tabs":tabs,"handoffs":handoffs,
        "profiles": catalog.profiles.values().chain(managed.disposable_profiles.values().map(|entry| &entry.profile))
            .filter(|profile| referenced_profiles.contains(profile.id.as_str()))
            .map(|profile| (profile.id.clone(), serde_json::json!({"id":profile.id,"name":profile.name,"kind":profile.kind})))
            .collect::<std::collections::BTreeMap<_, _>>(),
    }))
}

fn unavailable_operational_status(admission_path: &Path, error: &str) -> serde_json::Value {
    let failure_code = error
        .split(':')
        .next()
        .unwrap_or("browser_runtime_status_failed");
    serde_json::json!({
        "schemaVersion": "agent-browser.runtime-operational-status.v1",
        "state": "unavailable",
        "failureCode": failure_code,
        "launchAdmission": super::browser_launch_admission::observe_browser_launch_admission(
            admission_path,
            None,
        ),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserProfileCatalogLoad {
    pub(crate) catalog: BrowserProfileCatalog,
    pub(crate) diagnostics: Vec<BrowserProfileCatalogDiagnostic>,
    pub(crate) imported_legacy_profiles: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSessionJsonStore {
    session_state_path: PathBuf,
    profile_catalog_path: PathBuf,
}

impl BrowserSessionJsonStore {
    pub(crate) fn new(service_directory: impl Into<PathBuf>) -> Self {
        let service_directory = service_directory.into();
        Self {
            session_state_path: service_directory.join(BROWSER_SESSION_STATE_FILENAME),
            profile_catalog_path: service_directory.join(BROWSER_PROFILE_CATALOG_FILENAME),
        }
    }

    pub(crate) fn default_json() -> Result<Self, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Ok(Self::new(service_directory))
    }

    pub(crate) fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        let state: BrowserSessionState = read_json_or_default(&self.session_state_path)?;
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        Ok(state)
    }

    pub(crate) fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String> {
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        write_private_json_atomic(&self.session_state_path, state)
    }

    pub(crate) fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        if self.profile_catalog_path.exists() {
            let catalog: BrowserProfileCatalog = read_json(&self.profile_catalog_path)?;
            validate_catalog_schema(&catalog)?;
            return Ok(BrowserProfileCatalogLoad {
                catalog,
                diagnostics: Vec::new(),
                imported_legacy_profiles: false,
            });
        }

        let legacy_raw = match fs::read_to_string(legacy_service_state_path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
            Err(error) => {
                return Err(format!(
                    "browser_profile_legacy_state_read_failed:{}:{error}",
                    legacy_service_state_path.display()
                ))
            }
        };
        serde_json::from_str::<serde_json::Value>(&legacy_raw).map_err(|error| {
            format!(
                "browser_profile_legacy_state_json_invalid:{}:{error}",
                legacy_service_state_path.display()
            )
        })?;
        let imported = BrowserProfileCatalog::import_legacy_service_state_json(&legacy_raw);
        write_private_json_atomic(&self.profile_catalog_path, &imported.catalog)?;
        Ok(BrowserProfileCatalogLoad {
            catalog: imported.catalog,
            diagnostics: imported.diagnostics,
            imported_legacy_profiles: true,
        })
    }

    pub(crate) fn save_profile_catalog(
        &self,
        catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        validate_catalog_schema(catalog)?;
        write_private_json_atomic(&self.profile_catalog_path, catalog)
    }
}

fn open_runtime_connection(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| {
        format!(
            "browser_runtime_database_open_failed:{}:{error}",
            path.display()
        )
    })?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| format!("browser_runtime_database_busy_timeout_failed:{error}"))?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|error| format!("browser_runtime_database_pragma_failed:{error}"))?;
    Ok(connection)
}

fn initialize_runtime_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE runtime_metadata (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE TABLE state_documents (
               kind TEXT PRIMARY KEY,
               schema_version TEXT NOT NULL,
               json TEXT NOT NULL
             );
             PRAGMA user_version=1;",
        )
        .map_err(|error| format!("browser_runtime_database_schema_failed:{error}"))
}

fn validate_runtime_schema(connection: &Connection) -> Result<(), String> {
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(|error| format!("browser_runtime_database_schema_read_failed:{error}"))?;
    if !(1..=BROWSER_RUNTIME_DATABASE_MAX_SCHEMA).contains(&version) {
        return Err(format!(
            "browser_runtime_database_schema_unsupported:{version}"
        ));
    }
    let has_documents = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='state_documents'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| format!("browser_runtime_database_schema_read_failed:{error}"))?
        .is_some();
    if !has_documents {
        return Err("browser_runtime_database_schema_incomplete".to_string());
    }
    Ok(())
}

pub(super) fn save_document(
    connection: &Connection,
    kind: &str,
    schema_version: &str,
    value: &impl Serialize,
) -> Result<(), String> {
    let json = serde_json::to_string(value)
        .map_err(|error| format!("browser_runtime_document_serialize_failed:{error}"))?;
    let json = if kind == SESSION_STATE_DOCUMENT {
        terminal_history::prepare_json(connection, &json)?
    } else {
        json
    };
    connection
        .execute(
            "INSERT INTO state_documents(kind, schema_version, json) VALUES (?1, ?2, ?3)
             ON CONFLICT(kind) DO UPDATE SET schema_version=excluded.schema_version, json=excluded.json",
            params![kind, schema_version, json],
        )
        .map_err(|error| format!("browser_runtime_document_save_failed:{error}"))?;
    Ok(())
}

fn load_document<T: DeserializeOwned>(
    connection: &Connection,
    kind: &str,
    expected_schema: &str,
) -> Result<T, String> {
    let (schema, json) = connection
        .query_row(
            "SELECT schema_version, json FROM state_documents WHERE kind = ?1",
            params![kind],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|error| format!("browser_runtime_document_read_failed:{kind}:{error}"))?;
    if schema != expected_schema {
        return Err(format!(
            "browser_runtime_document_schema_unsupported:{kind}:{schema}"
        ));
    }
    let json = if kind == SESSION_STATE_DOCUMENT {
        terminal_history::project_json(&json)?
    } else {
        json
    };
    serde_json::from_str(&json)
        .map_err(|error| format!("browser_runtime_document_invalid:{kind}:{error}"))
}

pub(super) fn load_optional_document<T: DeserializeOwned + Default>(
    connection: &Connection,
    kind: &str,
    expected_schema: &str,
) -> Result<T, String> {
    let row = connection
        .query_row(
            "SELECT schema_version, json FROM state_documents WHERE kind = ?1",
            params![kind],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| format!("browser_runtime_document_read_failed:{kind}:{error}"))?;
    let Some((schema, json)) = row else {
        return Ok(T::default());
    };
    if schema != expected_schema {
        return Err(format!(
            "browser_runtime_document_schema_unsupported:{kind}:{schema}"
        ));
    }
    let json = if kind == SESSION_STATE_DOCUMENT {
        terminal_history::project_json(&json)?
    } else {
        json
    };
    serde_json::from_str(&json)
        .map_err(|error| format!("browser_runtime_document_invalid:{kind}:{error}"))
}

fn migration_digest(sources: &[(&Path, Vec<u8>)]) -> String {
    let mut digest = Sha256::new();
    for (path, bytes) in sources {
        digest.update(path.as_os_str().as_encoded_bytes());
        digest.update([0]);
        digest.update(bytes);
        digest.update([0]);
    }
    hex::encode(digest.finalize())
}

fn create_migration_archive(
    archive_directory: &Path,
    sources: &[(&Path, Vec<u8>)],
) -> Result<(), String> {
    fs::create_dir_all(archive_directory)
        .map_err(|error| format!("browser_runtime_migration_archive_failed:{error}"))?;
    for (source, bytes) in sources {
        let name = source
            .file_name()
            .ok_or_else(|| "browser_runtime_migration_source_name_missing".to_string())?;
        let destination = archive_directory.join(name);
        if !destination.exists() {
            fs::write(&destination, bytes)
                .map_err(|error| format!("browser_runtime_migration_archive_failed:{error}"))?;
        }
    }
    Ok(())
}

fn read_optional_migration_source(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "browser_runtime_migration_source_read_failed:{}:{error}",
            path.display()
        )),
    }
}

fn set_archive_read_only(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("browser_runtime_migration_archive_missing".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in fs::read_dir(path)
            .map_err(|error| format!("browser_runtime_migration_archive_read_failed:{error}"))?
        {
            let entry = entry.map_err(|error| {
                format!("browser_runtime_migration_archive_read_failed:{error}")
            })?;
            fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o400)).map_err(
                |error| format!("browser_runtime_migration_archive_permissions_failed:{error}"),
            )?;
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o500)).map_err(|error| {
            format!("browser_runtime_migration_archive_permissions_failed:{error}")
        })?;
    }
    #[cfg(windows)]
    for entry in fs::read_dir(path)
        .map_err(|error| format!("browser_runtime_migration_archive_read_failed:{error}"))?
    {
        let entry = entry
            .map_err(|error| format!("browser_runtime_migration_archive_read_failed:{error}"))?;
        let mut permissions = entry
            .metadata()
            .map_err(|error| format!("browser_runtime_migration_archive_read_failed:{error}"))?
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(entry.path(), permissions).map_err(|error| {
            format!("browser_runtime_migration_archive_permissions_failed:{error}")
        })?;
    }
    Ok(())
}

fn prepare_private_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "browser_runtime_database_parent_missing".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("browser_runtime_database_directory_failed:{error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!("browser_runtime_database_directory_permissions_failed:{error}")
        })?;
    }
    Ok(())
}

fn set_private_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("browser_runtime_database_permissions_failed:{error}"))?;
    }
    Ok(())
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn remove_sqlite_sidecars(path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(sqlite_sidecar_path(path, suffix));
    }
}

fn remove_sqlite_files(path: &Path) {
    let _ = fs::remove_file(path);
    remove_sqlite_sidecars(path);
}

fn sqlite_integrity_state(connection: &Connection) -> Result<(), String> {
    let result = connection
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map_err(|error| format!("browser_runtime_integrity_check_failed:{error}"))?;
    if result == "ok" {
        Ok(())
    } else {
        Err("browser_runtime_integrity_check_not_ok".to_string())
    }
}

fn verify_sqlite_backup(path: &Path) -> Result<(), String> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("browser_runtime_backup_open_failed:{error}"))?;
    sqlite_integrity_state(&connection)?;
    validate_runtime_schema(&connection)
        .map_err(|error| format!("browser_runtime_backup_schema_invalid:{error}"))
}

fn regular_file_bytes(path: &Path) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("browser_runtime_storage_metadata_failed:{error}"))?;
    if metadata.file_type().is_file() {
        Ok(metadata.len())
    } else {
        Err("browser_runtime_storage_path_not_regular".to_string())
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
        .map_err(|error| format!("browser_runtime_backup_digest_failed:{error}"))
}

fn read_optional_backup_manifest(
    path: &Path,
) -> Result<Option<BrowserRuntimeBackupManifest>, String> {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(|error| format!("browser_runtime_backup_manifest_invalid:{error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "browser_runtime_backup_manifest_read_failed:{error}"
        )),
    }
}

fn validate_catalog_schema(catalog: &BrowserProfileCatalog) -> Result<(), String> {
    if catalog.schema_version != BROWSER_PROFILE_CATALOG_SCHEMA_V1 {
        return Err(format!(
            "browser_profile_catalog_schema_unsupported:{}",
            catalog.schema_version
        ));
    }
    Ok(())
}

fn read_json_or_default<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw)
            .map_err(|error| format!("browser_session_json_invalid:{}:{error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(format!(
            "browser_session_json_read_failed:{}:{error}",
            path.display()
        )),
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "browser_session_json_read_failed:{}:{error}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("browser_session_json_invalid:{}:{error}", path.display()))
}

pub(super) fn write_private_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "browser_session_json_parent_missing".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "browser_session_json_directory_failed:{}:{error}",
            parent.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!("browser_session_json_directory_permissions_failed:{error}")
        })?;
    }
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("browser_session_json_serialize_failed:{error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|error| format!("browser_session_json_stage_failed:{error}"))?;
        file.write_all(&body)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("browser_session_json_write_failed:{error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&staged, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("browser_session_json_permissions_failed:{error}"))?;
        }
        atomic_replace(&staged, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination)
        .map_err(|error| format!("browser_session_json_commit_failed:{error}"))
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
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(format!(
            "browser_session_json_commit_failed:{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{
        BrowserProfileKind, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
        RemoteViewAssignmentRecord, RemoteViewAssignmentState, RemoteViewDesktopViewingRetirement,
        RemoteViewFixedDesktop, RemoteViewPresentationRetentionState,
    };

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn existing_profile_original_identity_requires_exact_retained_adoption_provenance() {
        let directory = TempDirectory::new("adoption-original-identity");
        let connection = Connection::open(directory.0.join("runtime.sqlite3")).unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let store = BrowserSessionSqliteStore { connection };
        let mut state = BrowserSessionState::default();
        let browser = ManagedBrowserInstance {
            id: "borrowed".into(),
            profile_id: "profile".into(),
            pid: 42,
            cdp_endpoint: "ws://127.0.0.1:9/devtools/browser/original".into(),
            desktop: None,
            active_session_ids: vec![],
        };
        state.browsers.insert(browser.id.clone(), browser.clone());
        state.shared_browser_ids.insert(browser.id.clone());
        store.save_session_state(&state).unwrap();
        assert_eq!(store.original_browser_identity(&state, &browser), None);
        let mut encoded = serde_json::to_value(&state).unwrap();
        encoded["adoptedBrowserProcessIdentities"] = serde_json::json!({"borrowed":{
            "profileId":"profile","cdpEndpoint":browser.cdp_endpoint,
            "process":{"pid":42,"startToken":"boot:original","executablePath":"/fixture/chrome","browserFamily":"chrome"}}});
        state = serde_json::from_value(encoded).unwrap();
        store.save_session_state(&state).unwrap();
        assert_eq!(
            store
                .original_browser_identity(&state, &browser)
                .unwrap()
                .start_token,
            "boot:original"
        );
        let mut conflicting = browser.clone();
        conflicting.profile_id = "peer-profile".into();
        assert_eq!(store.original_browser_identity(&state, &conflicting), None);
        conflicting = browser.clone();
        conflicting.pid = 43;
        assert_eq!(store.original_browser_identity(&state, &conflicting), None);
        conflicting = browser.clone();
        conflicting.cdp_endpoint.push_str("-replacement");
        assert_eq!(store.original_browser_identity(&state, &conflicting), None);
        state.shared_browser_ids.clear();
        assert_eq!(store.original_browser_identity(&state, &browser), None);
    }

    #[test]
    fn browser_mode_inspection_rechecks_addressed_task_after_observation() {
        let directory = TempDirectory::new("mode-task-race");
        let path = directory.0.join("runtime.sqlite3");
        let connection = Connection::open(&path).unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let writer = BrowserSessionSqliteStore { connection };
        let mut state = BrowserSessionState::default();
        state.sessions.insert(
            "task".into(),
            ManagedBrowserSession {
                id: "task".into(),
                name: "Task".into(),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 99999,
                current_tab_id: None,
            },
        );
        state.browsers.insert(
            "browser".into(),
            ManagedBrowserInstance {
                id: "browser".into(),
                profile_id: "profile".into(),
                pid: 42,
                cdp_endpoint: String::new(),
                desktop: None,
                active_session_ids: vec!["task".into()],
            },
        );
        writer.save_session_state(&state).unwrap();
        let reader = BrowserSessionSqliteStore {
            connection: Connection::open(&path).unwrap(),
        };
        let result = reader.inspect_task_mode_with("task", |_, _| {
            // The original browser remains for peers while this logical task ends.
            state.sessions.remove("task");
            writer.save_session_state(&state).unwrap();
            serde_json::json!({"mode":"cdp","browserId":"browser","profileId":"profile"})
        });
        assert!(result
            .unwrap_err()
            .starts_with("browser_task_changed_during_observation"));
    }

    #[test]
    fn app_attachment_publication_failure_never_exposes_an_unfenced_profile_after_restart() {
        let directory = TempDirectory::new("attachment-atomic");
        let mut store = BrowserSessionSqliteStore::open_or_migrate(
            &directory.0,
            &directory.0.join("legacy.json"),
        )
        .unwrap()
        .0;
        let before = BrowserSessionState::default();
        let before_catalog = BrowserProfileCatalog::default();
        store.save_session_state(&before).unwrap();
        store.save_profile_catalog(&before_catalog).unwrap();
        let mut next = before.clone();
        next.next_session_sequence = 1;
        let mut catalog = before_catalog.clone();
        catalog.profiles.insert(
            "app".into(),
            agent_browser_service_model::BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "app".into(),
                name: "App".into(),
                user_data_dir: "/synthetic/app".into(),
                kind: BrowserProfileKind::Named,
            },
        );
        store.connection.execute_batch("CREATE TRIGGER refuse_attachment_state BEFORE UPDATE ON state_documents WHEN NEW.kind='browser_session_state' BEGIN SELECT RAISE(ABORT,'synthetic publication failure'); END;").unwrap();
        assert!(store
            .compare_and_save_app_attachment(&before, &next, &before_catalog, &catalog)
            .is_err());
        drop(store);
        let mut store = BrowserSessionSqliteStore::open(&directory.0).unwrap();
        assert_eq!(
            store.load_workspace_snapshot().unwrap(),
            (before.clone(), before_catalog.clone())
        );
        store
            .connection
            .execute_batch("DROP TRIGGER refuse_attachment_state")
            .unwrap();
        store
            .compare_and_save_app_attachment(&before, &next, &before_catalog, &catalog)
            .unwrap();
        assert_eq!(
            store.load_workspace_snapshot().unwrap(),
            (next.clone(), catalog.clone())
        );
        assert!(store
            .compare_and_save_app_attachment(&before, &next, &before_catalog, &catalog)
            .is_err());
    }

    fn assert_workspace_projection_declared(value: &serde_json::Value) {
        fn check(value: &serde_json::Value, schema: &serde_json::Value) {
            if let Some(object) = value.as_object() {
                for (key, child) in object {
                    let declared = schema["properties"].get(key);
                    if schema["additionalProperties"] == false {
                        assert!(
                            declared.is_some(),
                            "public workspace field must be declared: {key}"
                        );
                    }
                    if let Some(declared) = declared {
                        check(child, declared);
                    } else if schema["additionalProperties"].is_object() {
                        check(child, &schema["additionalProperties"]);
                    }
                }
            }
        }
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-status-response.v1.schema.json"
        ))
        .unwrap();
        check(
            value,
            &schema["properties"]["browserRuntime"]["properties"]["workspace"],
        );
    }

    #[test]
    fn profile_properties_catalog_only_disposable_rename_persists() {
        use agent_browser_service_model::BrowserProfileCatalogEntry;
        let directory = TempDirectory::new("catalog-only-profile");
        let connection = Connection::open(directory.0.join("runtime.sqlite3")).unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut store = BrowserSessionSqliteStore { connection };
        store
            .save_session_state(&BrowserSessionState::default())
            .unwrap();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "legacy-profile".into(),
            name: "Old label".into(),
            user_data_dir: directory.0.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Disposable,
        };
        store
            .save_profile_catalog(&BrowserProfileCatalog {
                profiles: BTreeMap::from([(profile.id.clone(), profile)]),
                ..Default::default()
            })
            .unwrap();
        store
            .update_profile_properties(
                "legacy-profile",
                "Old label",
                BrowserProfileKind::Disposable,
                "New label",
                false,
            )
            .unwrap();
        assert_eq!(
            store.load_profile_catalog().unwrap().catalog.profiles["legacy-profile"].name,
            "New label"
        );
    }

    #[test]
    fn profile_properties_projection_recovers_only_the_original_logical_tab_context() {
        let mut state = BrowserSessionState::default();
        state.sessions.insert(
            "session-a".into(),
            ManagedBrowserSession {
                id: "session-a".into(),
                name: "Research".into(),
                profile_id: "profile-a".into(),
                browser_id: "browser-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: 99999,
                current_tab_id: Some("tab-a".into()),
            },
        );
        state.tabs.insert(
            "tab-a".into(),
            ManagedBrowserTab {
                id: "tab-a".into(),
                browser_id: "browser-a".into(),
                session_id: "session-a".into(),
                target_id: "target-a".into(),
                created_at_ms: 123,
                last_activity_at_ms: 2,
            },
        );
        let capsule = serde_json::json!({"id":"link-a","sessionId":"session-a","tabId":"tab-a","resume":{
            "sessionName":"Research","profileId":"profile-a","disposablePolicyId":null,"url":null,
            "taskMetadata":{"serviceName":"Original producer","agentName":"Original worker","taskName":"Research task","attribution":"supplied"}}});
        state
            .remote_view_tab_handoffs
            .insert("link-a".into(), serde_json::from_value(capsule).unwrap());
        let mut catalog = BrowserProfileCatalog::default();
        catalog.profiles.insert(
            "unrelated-history".into(),
            agent_browser_service_model::BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "unrelated-history".into(),
                name: "Unused historical profile".into(),
                user_data_dir: "/synthetic/private".into(),
                kind: BrowserProfileKind::Named,
            },
        );
        let projected = workspace_projection(&state, &catalog).unwrap();
        assert_workspace_projection_declared(&projected);
        assert!(
            projected["profiles"].get("unrelated-history").is_none(),
            "rail projection must not include the entire historical catalog"
        );
        assert_eq!(
            projected["tabs"]["tab-a"]["taskMetadata"]["serviceName"],
            "Original producer"
        );
        assert_eq!(projected["tabs"]["tab-a"]["createdAtMs"], 123);
        state
            .remote_view_tab_handoffs
            .get_mut("link-a")
            .unwrap()
            .session_id = "peer-session".into();
        assert!(
            workspace_projection(&state, &catalog).unwrap()["tabs"]["tab-a"]["taskMetadata"]
                .is_null()
        );
    }

    #[test]
    fn profile_properties_promotes_atomically_and_rejects_stale_reaper_publication() {
        use agent_browser_service_model::{BrowserProfileCatalogEntry, ManagedDisposableProfile};
        let directory = TempDirectory::new("profile-properties");
        let data = directory.0.join("browser-data");
        fs::create_dir(&data).unwrap();
        fs::write(data.join("retained-marker"), b"retained").unwrap();
        let connection = Connection::open(directory.0.join("runtime.sqlite3")).unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut store = BrowserSessionSqliteStore { connection };
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "stable-id".into(),
            name: "Temporary research".into(),
            user_data_dir: data.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Disposable,
        };
        let mut state = BrowserSessionState::default();
        state.disposable_profiles.insert(
            profile.id.clone(),
            ManagedDisposableProfile {
                profile: profile.clone(),
                policy_id: "default".into(),
                session_name: "task".into(),
                user_data_root: directory.0.to_string_lossy().into_owned(),
                created_at_ms: 1,
                cleanup_delay_ms: 1,
                cleanup_eligible_at_ms: Some(2),
            },
        );
        state.browsers.insert(
            "browser-a".into(),
            ManagedBrowserInstance {
                id: "browser-a".into(),
                profile_id: "stable-id".into(),
                pid: 42,
                cdp_endpoint: "ws://synthetic.invalid".into(),
                desktop: None,
                active_session_ids: vec!["session-a".into()],
            },
        );
        state.sessions.insert(
            "session-a".into(),
            ManagedBrowserSession {
                id: "session-a".into(),
                name: "Research".into(),
                profile_id: "stable-id".into(),
                browser_id: "browser-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: 99999,
                current_tab_id: Some("tab-a".into()),
            },
        );
        state.tabs.insert(
            "tab-a".into(),
            ManagedBrowserTab {
                id: "tab-a".into(),
                browser_id: "browser-a".into(),
                session_id: "session-a".into(),
                target_id: "target-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
            },
        );
        store.save_session_state(&state).unwrap();
        store
            .save_profile_catalog(&BrowserProfileCatalog::default())
            .unwrap();
        let renamed = store
            .update_profile_properties(
                "stable-id",
                "Temporary research",
                BrowserProfileKind::Disposable,
                "Research",
                false,
            )
            .unwrap();
        assert_eq!(renamed["kind"], "disposable");
        assert_eq!(
            store.load_session_state().unwrap().disposable_profiles["stable-id"]
                .profile
                .name,
            "Research"
        );
        assert_eq!(
            store
                .update_profile_properties(
                    "stable-id",
                    "Temporary research",
                    BrowserProfileKind::Disposable,
                    "Wrong",
                    true
                )
                .unwrap_err(),
            "profile_properties_changed"
        );
        let stale = store.load_session_state().unwrap();
        let promoted = store
            .update_profile_properties(
                "stable-id",
                "Research",
                BrowserProfileKind::Disposable,
                "Kept research",
                true,
            )
            .unwrap();
        assert_eq!(promoted["kind"], "named");
        let after = store.load_session_state().unwrap();
        assert!(after.disposable_profiles.is_empty());
        assert_eq!(after.browsers, state.browsers);
        assert_eq!(after.sessions, state.sessions);
        assert_eq!(after.tabs, state.tabs);
        assert_eq!(
            store
                .compare_and_save_profile_catalog(
                    &BrowserProfileCatalog::default(),
                    &BrowserProfileCatalog::default()
                )
                .unwrap_err(),
            "browser_profile_catalog_publication_conflict"
        );
        let catalog = store.load_profile_catalog().unwrap().catalog;
        assert_eq!(
            catalog.profiles["stable-id"].user_data_dir,
            profile.user_data_dir
        );
        assert_eq!(
            catalog.profiles["stable-id"].kind,
            BrowserProfileKind::Named
        );
        assert_eq!(fs::read(data.join("retained-marker")).unwrap(), b"retained");
        assert_eq!(
            store
                .compare_and_save_session_state(&stale, &stale)
                .unwrap_err(),
            "browser_session_publication_conflict"
        );
        assert_eq!(
            store
                .update_profile_properties(
                    "stable-id",
                    "Kept research",
                    BrowserProfileKind::Named,
                    "  ",
                    false
                )
                .unwrap_err(),
            "profile_properties_name_invalid"
        );
        assert_workspace_projection_declared(&workspace_projection(&after, &catalog).unwrap());
        assert_eq!(
            workspace_projection(&after, &catalog).unwrap()["profiles"]["stable-id"]["name"],
            "Kept research"
        );
        drop(store);
        let reopened = BrowserSessionSqliteStore::open(&directory.0).unwrap();
        assert!(reopened
            .load_session_state()
            .unwrap()
            .disposable_profiles
            .is_empty());
        assert_eq!(
            reopened.load_profile_catalog().unwrap().catalog.profiles["stable-id"].name,
            "Kept research"
        );
    }

    #[test]
    fn terminal_history_projection_is_bounded_after_publication_and_restart() {
        let directory = TempDirectory::new("terminal-history");
        let database_path = directory.0.join("runtime.sqlite3");
        let connection = Connection::open(&database_path).unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut store = BrowserSessionSqliteStore { connection };
        let initial = BrowserSessionState::default();
        store.save_session_state(&initial).unwrap();
        let mut proposed = initial.clone();
        proposed.last_remote_view_desktop_id = Some("preserved-live-desktop".into());
        for index in 0..1001 {
            proposed
                .session_history
                .push(agent_browser_service_model::TerminalBrowserSession {
                    id: format!("ended-{index}"),
                    name: format!("task-{index}"),
                    profile_id: "profile".into(),
                    browser_id: "browser".into(),
                    created_at_ms: index,
                    last_activity_at_ms: index,
                    ended_at_ms: index,
                    reason: agent_browser_service_model::SessionEndReason::ExplicitClose,
                });
            proposed
                .tab_history
                .push(agent_browser_service_model::TerminalBrowserTab {
                    id: format!("tab-ended-{index}"),
                    target_id: format!("target-{index}"),
                    browser_id: "browser".into(),
                    session_id: format!("ended-{index}"),
                    profile_id: "profile".into(),
                    created_at_ms: index,
                    last_activity_at_ms: index,
                    closed_at_ms: index,
                    reason: agent_browser_service_model::BrowserTabEndReason::ExplicitClose,
                    session_end_reason: None,
                });
        }
        store
            .compare_and_save_session_state(&initial, &proposed)
            .unwrap();
        drop(store);
        let restarted = BrowserSessionSqliteStore {
            connection: Connection::open(&database_path).unwrap(),
        };
        let active = restarted.load_session_state().unwrap();
        assert_eq!(active.session_history.len(), 1000);
        assert_eq!(active.session_history.first().unwrap().id, "ended-1");
        assert_eq!(active.session_history.last().unwrap().id, "ended-1000");
        assert_eq!(
            active.last_remote_view_desktop_id.as_deref(),
            Some("preserved-live-desktop")
        );
        let archived = restarted
            .archived_terminal_history("sessions", 10, None)
            .unwrap();
        assert_eq!(archived["records"].as_array().unwrap().len(), 1);
        assert_eq!(archived["records"][0]["record"]["id"], "ended-0");
        assert_eq!(archived["nextCursor"], serde_json::Value::Null);
        assert_eq!(active.tab_history.len(), 1000);
        assert_eq!(
            restarted
                .archived_terminal_history("tabs", 1, None)
                .unwrap()["records"][0]["record"]["id"],
            "tab-ended-0"
        );
        let mut restarted = restarted;
        let mut newer = active.clone();
        newer.next_session_sequence = 77;
        restarted
            .compare_and_save_session_state(&active, &newer)
            .unwrap();
        assert!(restarted
            .compare_and_save_session_state(&active, &proposed)
            .is_err());
        assert_eq!(restarted.load_session_state().unwrap(), newer);
        assert_eq!(
            restarted
                .archived_terminal_history("sessions", 10, None)
                .unwrap(),
            archived
        );
    }

    #[test]
    fn archive_failure_aborts_publication_and_preserves_handoff_references() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut store = BrowserSessionSqliteStore { connection };
        let initial = BrowserSessionState::default();
        store.save_session_state(&initial).unwrap();
        let mut proposed = initial.clone();
        for index in 0..1002 {
            proposed
                .session_history
                .push(agent_browser_service_model::TerminalBrowserSession {
                    id: format!("ended-{index}"),
                    name: format!("task-{index}"),
                    profile_id: "profile".into(),
                    browser_id: "browser".into(),
                    created_at_ms: index,
                    last_activity_at_ms: index,
                    ended_at_ms: index,
                    reason: agent_browser_service_model::SessionEndReason::HeartbeatExpired,
                });
        }
        proposed.remote_view_tab_handoffs.insert(
            "retained-link".into(),
            agent_browser_service_model::RemoteViewTabHandoff {
                id: "retained-link".into(),
                session_id: "ended-0".into(),
                tab_id: "retained-target".into(),
                created_at_ms: None,
                expires_at_ms: None,
                view: None,
                resume: None,
                recreation: None,
                target_expired: false,
                native_observation: None,
            },
        );
        store.connection.execute_batch("CREATE TRIGGER fail_archive BEFORE INSERT ON terminal_history_archive BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(store
            .compare_and_save_session_state(&initial, &proposed)
            .is_err());
        assert_eq!(store.load_session_state().unwrap(), initial);
        assert!(store
            .archived_terminal_history("sessions", 10, None)
            .unwrap()["records"]
            .as_array()
            .unwrap()
            .is_empty());
        store
            .connection
            .execute_batch("DROP TRIGGER fail_archive")
            .unwrap();
        store
            .compare_and_save_session_state(&initial, &proposed)
            .unwrap();
        let active = store.load_session_state().unwrap();
        assert_eq!(
            active.session_history.len(),
            1001,
            "Recent budget plus exact handoff evidence"
        );
        assert_eq!(active.session_history.first().unwrap().id, "ended-0");
        assert_eq!(
            active.remote_view_tab_handoffs,
            proposed.remote_view_tab_handoffs
        );
        let first = store
            .archived_terminal_history("sessions", 1, None)
            .unwrap();
        assert_eq!(first["records"][0]["record"]["id"], "ended-1");
        let cursor = first["nextCursor"].as_i64().unwrap();
        assert!(store
            .archived_terminal_history("sessions", 1, Some(cursor))
            .unwrap()["records"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn sqlite_session_publication_rejects_stale_snapshot_without_overwriting_current_state() {
        let directory = TempDirectory::new("session-publication");
        let database_path = directory.0.join("runtime.sqlite3");
        let connection = Connection::open(&database_path).unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut store = BrowserSessionSqliteStore { connection };
        let mut peer = BrowserSessionSqliteStore {
            connection: Connection::open(&database_path).unwrap(),
        };
        let initial = BrowserSessionState::default();
        store.save_session_state(&initial).unwrap();
        let mut current = initial.clone();
        current.next_session_sequence = 7;
        store
            .compare_and_save_session_state(&initial, &current)
            .unwrap();
        let mut stale = initial.clone();
        stale.next_session_sequence = 99;
        assert_eq!(
            peer.compare_and_save_session_state(&initial, &stale)
                .unwrap_err(),
            "browser_session_publication_conflict"
        );
        assert_eq!(store.load_session_state().unwrap(), current);
        store
            .compare_and_save_session_state(&current, &stale)
            .unwrap();
        assert_eq!(store.load_session_state().unwrap(), stale);
    }

    #[test]
    fn session_state_round_trips_in_its_independent_file() {
        let directory = TempDirectory::new("browser-session-store");
        let store = BrowserSessionJsonStore::new(&directory.0);
        let mut state = BrowserSessionState::default();
        state.next_session_sequence = 42;

        store.save_session_state(&state).unwrap();
        state.next_session_sequence = 43;
        store.save_session_state(&state).unwrap();
        let loaded = store.load_session_state().unwrap();

        assert_eq!(loaded, state);
        assert!(directory.0.join(BROWSER_SESSION_STATE_FILENAME).exists());
        assert!(!directory.0.join("state.json").exists());
    }

    #[test]
    fn first_catalog_load_imports_profiles_once_and_ignores_other_legacy_fields() {
        let directory = TempDirectory::new("browser-profile-catalog");
        let store = BrowserSessionJsonStore::new(&directory.0);
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": "/managed/work",
                        "profileClass": "durable_named"
                    }
                },
                "sessions": "malformed but irrelevant"
            })
            .to_string(),
        )
        .unwrap();

        let imported = store.load_or_import_profile_catalog(&legacy_path).unwrap();
        assert!(imported.imported_legacy_profiles);
        assert_eq!(
            imported.catalog.profiles["work"].kind,
            BrowserProfileKind::Named
        );

        fs::write(&legacy_path, r#"{"profiles":{}}"#).unwrap();
        let authoritative = store.load_or_import_profile_catalog(&legacy_path).unwrap();
        assert!(!authoritative.imported_legacy_profiles);
        assert!(authoritative.catalog.profiles.contains_key("work"));
    }

    #[test]
    fn issue79_preserves_legacy_sqlite_migration_provenance_and_state() {
        let directory = TempDirectory::new("legacy-sqlite-receipt");
        let legacy_path = directory.0.join("state.json");
        let json_store = BrowserSessionJsonStore::new(&directory.0);
        let mut state = BrowserSessionState::default();
        state.next_session_sequence = 42;
        json_store.save_session_state(&state).unwrap();
        let (store, original) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        store.connection.execute_batch("DELETE FROM runtime_metadata WHERE key='imported_source_count';
            CREATE TABLE migration_sources(source_path TEXT PRIMARY KEY, sha256 TEXT NOT NULL, archive_name TEXT NOT NULL, imported INTEGER NOT NULL);
            INSERT INTO migration_sources VALUES ('a','digest','a',1),('b','digest','b',1),('c','digest','c',0);").unwrap();
        drop(store);
        fs::write(&legacy_path, "{} ").unwrap();
        let (store, receipt) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        assert_eq!(receipt.imported_source_count, 2);
        assert_eq!(receipt.archive_directory, original.archive_directory);
        assert_eq!(
            store.load_session_state().unwrap().next_session_sequence,
            42
        );
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT count(*) FROM runtime_metadata WHERE key='imported_source_count'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn sqlite_migration_is_one_time_and_existing_database_is_authoritative() {
        let directory = TempDirectory::new("browser-session-sqlite-migration");
        let legacy_path = directory.0.join("state.json");
        let json_store = BrowserSessionJsonStore::new(&directory.0);
        let mut state = BrowserSessionState::default();
        state.next_session_sequence = 7;
        json_store.save_session_state(&state).unwrap();
        fs::write(&legacy_path, "{}").unwrap();

        let (store, receipt) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        assert_eq!(store.load_session_state().unwrap().next_session_sequence, 7);
        assert_eq!(receipt.imported_source_count, 3);
        assert!(receipt.archive_directory.is_dir());
        drop(store);

        state.next_session_sequence = 99;
        json_store.save_session_state(&state).unwrap();
        let (store, replay) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        assert_eq!(store.load_session_state().unwrap().next_session_sequence, 7);
        assert_eq!(replay, receipt);
        drop(store);

        fs::write(
            directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME),
            "corrupt",
        )
        .unwrap();
        assert!(BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).is_err());
    }

    #[test]
    fn sqlite_backup_is_verified_restart_safe_and_rotates_one_previous_copy() {
        let directory = TempDirectory::new("browser-session-sqlite-backup");
        let legacy_path = directory.0.join("state.json");
        fs::write(&legacy_path, "{}").unwrap();
        let (store, _) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();

        let first = store.create_verified_backup().unwrap();
        let second = store.create_verified_backup().unwrap();
        assert_eq!(second.previous_sha256, Some(first.database_sha256));
        assert_eq!(store.backup_status().unwrap().state, "verified");
        drop(store);

        let reopened = BrowserSessionSqliteStore::open(&directory.0).unwrap();
        assert_eq!(
            reopened.backup_status().unwrap().sha256,
            Some(second.database_sha256)
        );
        assert!(directory
            .0
            .join(BROWSER_RUNTIME_BACKUP_DIRECTORY)
            .join(BROWSER_RUNTIME_BACKUP_PREVIOUS)
            .is_file());
    }

    #[test]
    fn operational_status_lists_retained_handoffs_without_private_resume_routes() {
        let directory = TempDirectory::new("handoff-inventory");
        let legacy_path = directory.0.join("state.json");
        let (store, _) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        let mut state = store.load_session_state().unwrap();
        state.remote_view_tab_handoffs.insert("opaque-link".into(), serde_json::from_value(serde_json::json!({
            "id":"opaque-link", "sessionId":"ended-task", "tabId":"expired-tab",
            "createdAtMs":100, "expiresAtMs":null, "targetExpired":true,
            "resume":{"sessionName":"original-task","profileId":"profile-a","disposablePolicyId":null,
                "url":"https://provider.invalid/PRIVATE-RESUME-CANARY"}
        })).unwrap());
        store.save_session_state(&state).unwrap();
        drop(store);
        let path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        let before = sha256_file(&path).unwrap();
        let status = BrowserSessionSqliteStore::operational_status_read_only(&path);
        let link = &status["workspace"]["handoffs"]["opaque-link"];
        assert_eq!(link["handoffUrl"], "/remote-view/opaque-link");
        assert_eq!(link["createdAtMs"], 100);
        assert!(link["expiresAtMs"].is_null());
        assert_eq!(link["profileId"], "profile-a");
        assert_eq!(link["targetState"], "expired");
        assert!(!status.to_string().contains("PRIVATE-RESUME-CANARY"));
        assert!(link.get("resume").is_none());
        assert_eq!(sha256_file(&path).unwrap(), before);
    }

    #[test]
    fn operational_status_is_redacted_and_does_not_create_a_backup() {
        let directory = TempDirectory::new("browser-runtime-operational-status");
        let legacy_path = directory.0.join("state.json");
        fs::write(&legacy_path, "{}").unwrap();
        let (store, _) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        drop(store);
        let digest_before = sha256_file(&database_path).unwrap();

        let status = BrowserSessionSqliteStore::operational_status_read_only(&database_path);

        assert_eq!(
            status["schemaVersion"],
            "agent-browser.runtime-operational-status.v1"
        );
        assert_eq!(status["state"], "available");
        assert_eq!(status["storage"]["integrityState"], "ok");
        assert_eq!(status["storage"]["backupState"], "missing");
        assert_eq!(status["migration"]["archiveState"], "present");
        assert!(matches!(
            status["launchAdmission"]["state"].as_str(),
            Some("admitted" | "rejected")
        ));
        assert!(status.get("databasePath").is_none());
        assert!(!status
            .to_string()
            .contains(directory.0.to_string_lossy().as_ref()));
        assert_eq!(sha256_file(&database_path).unwrap(), digest_before);
        assert!(!directory.0.join(BROWSER_RUNTIME_BACKUP_DIRECTORY).exists());

        let missing_path = directory.0.join("missing-runtime.sqlite3");
        let unavailable = BrowserSessionSqliteStore::operational_status_read_only(&missing_path);
        assert_eq!(unavailable["state"], "unavailable");
        assert_eq!(
            unavailable["failureCode"],
            "browser_runtime_database_read_only_unavailable"
        );
        assert!(!unavailable
            .to_string()
            .contains(directory.0.to_string_lossy().as_ref()));
    }

    #[test]
    fn sqlite_session_document_retains_remote_view_public_identity_across_restart() {
        let directory = TempDirectory::new("remote-view-retention-restart");
        let legacy_path = directory.0.join("state.json");
        let (mut store, _) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        let mut state = store.load_session_state().unwrap();
        state.browsers.insert(
            "browser-a".into(),
            ManagedBrowserInstance {
                profile_id: "profile-a".into(),
                id: "browser-a".into(),
                pid: 42,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: "11111111-1111-1111-1111-111111111111".into(),
                    friendly_route_label: "desktop-a".into(),
                    generation: 7,
                }),
                active_session_ids: vec!["session-a".into()],
            },
        );
        state.sessions.insert(
            "session-a".into(),
            ManagedBrowserSession {
                id: "session-a".into(),
                name: "work".into(),
                profile_id: "profile-a".into(),
                browser_id: "browser-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: 10,
                current_tab_id: Some("tab-a".into()),
            },
        );
        state.tabs.insert(
            "tab-a".into(),
            ManagedBrowserTab {
                id: "tab-a".into(),
                target_id: "target-a".into(),
                browser_id: "browser-a".into(),
                session_id: "session-a".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
            },
        );
        store.save_session_state(&state).unwrap();
        let retained = store
            .retain_remote_view_presentation(
                "browser-a",
                "session-a",
                "tab-a",
                &RemoteViewDesktopPresentationBinding {
                    registration_id: "registration-a".into(),
                    pool_id: "pool-a".into(),
                    desktop_id: "11111111-1111-1111-1111-111111111111".into(),
                    generation: 7,
                    assignment_id: "assignment-a".into(),
                    placement_id: "placement-a".into(),
                    route_id: "22222222-2222-2222-2222-222222222222".into(),
                    viewer_session_ids: vec!["viewer-a".into()],
                },
            )
            .unwrap();
        assert_eq!(retained.state, RemoteViewPresentationRetentionState::Active);
        drop(store);

        let mut restarted = BrowserSessionSqliteStore::open(&directory.0).unwrap();
        assert_eq!(
            restarted
                .load_session_state()
                .unwrap()
                .remote_view_presentations["browser-a"],
            retained
        );
        assert_eq!(
            restarted
                .detach_remote_view_presentation("browser-a")
                .unwrap(),
            0
        );
        assert_eq!(
            restarted
                .load_session_state()
                .unwrap()
                .remote_view_presentations["browser-a"]
                .state,
            RemoteViewPresentationRetentionState::Detached
        );
        let released = restarted
            .release_remote_view_presentation(
                "browser-a",
                &RemoteViewJoinedReleaseOutcome {
                    assignment: RemoteViewAssignmentRecord {
                        presentation: None,
                        assignment_id: "assignment-a".into(),
                        registration_id: "registration-a".into(),
                        pool_id: "pool-a".into(),
                        desktop_id: "11111111-1111-1111-1111-111111111111".into(),
                        generation: 7,
                        state: RemoteViewAssignmentState::Released,
                    },
                    retirement: RemoteViewDesktopViewingRetirement {
                        schema_version: 1,
                        desktop_id: "11111111-1111-1111-1111-111111111111".into(),
                        generation: 7,
                        routes: vec!["22222222-2222-2222-2222-222222222222".into()],
                        sessions: vec!["viewer-a".into()],
                    },
                },
            )
            .unwrap();
        assert_eq!(
            released.state,
            RemoteViewPresentationRetentionState::Released
        );
        drop(restarted);
        assert_eq!(
            BrowserSessionSqliteStore::open(&directory.0)
                .unwrap()
                .load_session_state()
                .unwrap()
                .remote_view_presentations["browser-a"]
                .state,
            RemoteViewPresentationRetentionState::Released
        );
    }

    #[test]
    fn sqlite_recovery_admission_is_restart_safe_and_generation_fenced() {
        let directory = TempDirectory::new("browser-session-sqlite-recovery");
        let legacy_path = directory.0.join("state.json");
        fs::write(&legacy_path, "{}").unwrap();
        let (mut store, _) =
            BrowserSessionSqliteStore::open_or_migrate(&directory.0, &legacy_path).unwrap();
        let policy = BrowserRecoveryAdmissionPolicy {
            maximum_attempts: 3,
            base_backoff_ms: 10,
            maximum_backoff_ms: 15,
            deadline_ms: 100,
        };
        let admitted = store
            .admit_browser_recovery(
                "browser",
                BrowserRecoveryDemand::ExactClientResume,
                OldBrowserUsability::ProvenUnusable,
                1_000,
                policy,
            )
            .unwrap();
        let generation = match admitted {
            BrowserRecoveryDecision::AdmitReplacement { state } => state.generation,
            other => panic!("expected admission, got {other:?}"),
        };
        drop(store);

        let mut restarted = BrowserSessionSqliteStore::open(&directory.0).unwrap();
        assert_eq!(
            restarted
                .admit_browser_recovery(
                    "browser",
                    BrowserRecoveryDemand::ExactClientResume,
                    OldBrowserUsability::ProvenUnusable,
                    1_001,
                    policy,
                )
                .unwrap(),
            BrowserRecoveryDecision::AlreadyAdmitted { generation }
        );
        assert!(restarted
            .record_browser_recovery_observed_live("browser", generation + 1)
            .is_err());
        restarted
            .record_browser_recovery_observed_live("browser", generation)
            .unwrap();
        restarted
            .record_browser_recovery_success("browser", generation, 1_002)
            .unwrap();
        assert_eq!(
            restarted
                .load_browser_recovery_state("browser")
                .unwrap()
                .unwrap()
                .phase,
            BrowserRecoveryPhase::Recovered
        );
    }
}
