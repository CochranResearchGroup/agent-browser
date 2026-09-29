//! Independent durable storage for the ordinary Browser Session Manager path.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(test)]
use agent_browser_service_model::BrowserRecoveryPhase;
use agent_browser_service_model::{
    decide_browser_recovery, record_browser_recovery_failure,
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

const BROWSER_SESSION_STATE_FILENAME: &str = "browser-session-state.json";
const BROWSER_PROFILE_CATALOG_FILENAME: &str = "browser-profile-catalog.json";
const BROWSER_RUNTIME_DATABASE_FILENAME: &str = "runtime.sqlite3";
const BROWSER_RUNTIME_DATABASE_SCHEMA: i64 = 1;
const BROWSER_RUNTIME_BACKUP_DIRECTORY: &str = "browser-runtime-backups";
const BROWSER_RUNTIME_BACKUP_CURRENT: &str = "runtime.current.sqlite3";
const BROWSER_RUNTIME_BACKUP_PREVIOUS: &str = "runtime.previous.sqlite3";
const BROWSER_RUNTIME_BACKUP_MANIFEST: &str = "manifest.json";
const SESSION_STATE_DOCUMENT: &str = "browser_session_state";
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
    pub(crate) fn default_sqlite() -> Result<Self, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Self::open_or_migrate(service_directory, &legacy_state_path).map(|(store, _)| store)
    }

    pub(crate) fn open(service_directory: &Path) -> Result<Self, String> {
        let connection =
            open_runtime_connection(&service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME))?;
        validate_runtime_schema(&connection)?;
        Ok(Self { connection })
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
        let imported_source_count = read("imported_source_count")?
            .parse::<usize>()
            .map_err(|error| format!("browser_runtime_migration_receipt_invalid:{error}"))?;
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

    fn mutate_session_state<T>(
        &mut self,
        mutate: impl FnOnce(&mut BrowserSessionState) -> Result<T, String>,
    ) -> Result<T, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_session_mutation_begin_failed:{error}"))?;
        let mut state = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let result = mutate(&mut state)?;
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

    fn database_path(&self) -> Result<PathBuf, String> {
        self.connection
            .path()
            .map(PathBuf::from)
            .ok_or_else(|| "browser_runtime_database_path_unavailable".to_string())
    }
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
    if version != BROWSER_RUNTIME_DATABASE_SCHEMA {
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

fn save_document(
    connection: &Connection,
    kind: &str,
    schema_version: &str,
    value: &impl Serialize,
) -> Result<(), String> {
    let json = serde_json::to_string(value)
        .map_err(|error| format!("browser_runtime_document_serialize_failed:{error}"))?;
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
    serde_json::from_str(&json)
        .map_err(|error| format!("browser_runtime_document_invalid:{kind}:{error}"))
}

fn load_optional_document<T: DeserializeOwned + Default>(
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

fn write_private_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
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
        let released = restarted
            .release_remote_view_presentation(
                "browser-a",
                &RemoteViewJoinedReleaseOutcome {
                    assignment: RemoteViewAssignmentRecord {
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
