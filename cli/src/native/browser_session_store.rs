//! Independent durable storage for the ordinary Browser Session Manager path.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use agent_browser_service_model::{
    BrowserProfileCatalog, BrowserProfileCatalogDiagnostic, BrowserSessionState, RemoteViewHandoff,
    RouteKeeperAuthority, RouteKeeperConnectionCatalog, BROWSER_PROFILE_CATALOG_SCHEMA_V1,
    BROWSER_SESSION_STATE_SCHEMA_V1, ROUTE_KEEPER_AUTHORITY_SCHEMA_V1,
    ROUTE_KEEPER_AUTHORITY_SCHEMA_V2,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::service_store::default_service_state_path;

const BROWSER_SESSION_STATE_FILENAME: &str = "browser-session-state.json";
const BROWSER_PROFILE_CATALOG_FILENAME: &str = "browser-profile-catalog.json";
const BROWSER_RUNTIME_DATABASE_SCHEMA: i64 = 1;
const BROWSER_RUNTIME_DATABASE_FILENAME: &str = "runtime.sqlite3";
const SESSION_STATE_DOCUMENT: &str = "browser_session_state";
const PROFILE_CATALOG_DOCUMENT: &str = "browser_profile_catalog";
const MANAGER_HANDOFF_REGISTRY_DOCUMENT: &str = "manager_handoff_registry";
const ROUTE_KEEPER_AUTHORITY_DOCUMENT: &str = "route_keeper_authority";
const MANAGER_HANDOFF_REGISTRY_SCHEMA_V1: &str = "agent-browser.manager-handoffs.v1";
const RUNTIME_CONFIG_KEY: &str = "runtime";
const BROWSER_RUNTIME_CONFIG_SCHEMA_V1: &str = "agent-browser.runtime-config.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserRuntimeConfig {
    pub(crate) schema_version: String,
    pub(crate) revision: u64,
    pub(crate) minimum_ready: u32,
    pub(crate) warm_target: u32,
    pub(crate) maximum_displays: u32,
    pub(crate) maximum_browsers_per_display: u32,
    pub(crate) maximum_queue_depth: u32,
    pub(crate) request_deadline_ms: u64,
    pub(crate) scale_in_cooldown_ms: u64,
    pub(crate) session_idle_timeout_ms: u64,
    pub(crate) disposable_inactivity_ms: u64,
    pub(crate) maximum_retained_disposable_profiles: u32,
    pub(crate) maximum_disposable_profile_bytes: u64,
    pub(crate) live_database_maximum_bytes: u64,
    pub(crate) exact_url_history_maximum_bytes: u64,
    pub(crate) routine_storage_maximum_bytes: u64,
}

impl Default for BrowserRuntimeConfig {
    fn default() -> Self {
        Self {
            schema_version: BROWSER_RUNTIME_CONFIG_SCHEMA_V1.to_string(),
            revision: 0,
            minimum_ready: 1,
            warm_target: 4,
            maximum_displays: 6,
            maximum_browsers_per_display: 4,
            maximum_queue_depth: 32,
            request_deadline_ms: 90_000,
            scale_in_cooldown_ms: 600_000,
            session_idle_timeout_ms: 300_000,
            disposable_inactivity_ms: 86_400_000,
            maximum_retained_disposable_profiles: 20,
            maximum_disposable_profile_bytes: 10 * 1024 * 1024 * 1024,
            live_database_maximum_bytes: 96 * 1024 * 1024,
            exact_url_history_maximum_bytes: 64 * 1024 * 1024,
            routine_storage_maximum_bytes: 128 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserRuntimeOperationState {
    Prepared,
    Observed,
    Committed,
}

impl BrowserRuntimeOperationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Observed => "observed",
            Self::Committed => "committed",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "observed" => Ok(Self::Observed),
            "committed" => Ok(Self::Committed),
            other => Err(format!("browser_runtime_operation_state_invalid:{other}")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BrowserManagerHandoffRegistry {
    pub(crate) handoffs: BTreeMap<String, RemoteViewHandoff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserRuntimeOperation {
    pub(crate) operation_id: String,
    pub(crate) owner_key: String,
    pub(crate) generation: u64,
    pub(crate) state: BrowserRuntimeOperationState,
    pub(crate) request: serde_json::Value,
    pub(crate) result: Option<serde_json::Value>,
}

pub(crate) struct LegacyBrowserRuntimeSources<'a> {
    pub(crate) session_state_path: &'a Path,
    pub(crate) profile_catalog_path: &'a Path,
    pub(crate) service_state_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserRuntimeMigrationReceipt {
    pub(crate) imported_source_count: usize,
    pub(crate) rejected_record_count: usize,
    pub(crate) rejection_codes: Vec<String>,
    pub(crate) archive_directory: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteKeeperConnectionCatalogPublication {
    Published,
    Unchanged,
}

struct BrowserRuntimeMigrationRejection {
    source_path: PathBuf,
    code: &'static str,
    detail: String,
}

/// Transactional authority for the trusted single-user browser runtime.
///
/// Callers load and save typed aggregates. SQLite schema, journaling, source
/// hashing, and legacy archival stay behind this interface.
pub(crate) struct BrowserRuntimeSqliteStore {
    connection: Connection,
}

impl BrowserRuntimeSqliteStore {
    pub(crate) fn default_sqlite() -> Result<Self, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Self::open(&service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME))
    }

    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        if !path.is_file() {
            return Err(format!(
                "browser_runtime_database_missing:{}",
                path.display()
            ));
        }
        let connection = open_runtime_connection(path)?;
        validate_runtime_schema(&connection)?;
        Ok(Self { connection })
    }

    pub(crate) fn migrate_from_legacy(
        path: &Path,
        sources: LegacyBrowserRuntimeSources<'_>,
    ) -> Result<BrowserRuntimeMigrationReceipt, String> {
        if path.exists() {
            let store = Self::open(path)?;
            let receipt = store.migration_receipt()?;
            set_archive_read_only(&receipt.archive_directory)?;
            return Ok(receipt);
        }
        prepare_private_parent(path)?;

        let session_source = read_optional_source(sources.session_state_path)?;
        let session_raw = match &session_source {
            Some(bytes) => bytes.clone(),
            None => serde_json::to_vec(&BrowserSessionState::default()).map_err(|error| {
                format!("browser_runtime_migration_default_serialize_failed:{error}")
            })?,
        };
        let mut rejections = Vec::new();
        let mut session_imported = false;
        let session_state: BrowserSessionState =
            match serde_json::from_slice::<BrowserSessionState>(&session_raw) {
                Ok(state) if state.schema_version == BROWSER_SESSION_STATE_SCHEMA_V1 => {
                    session_imported = session_source.is_some();
                    state
                }
                Ok(state) => {
                    if session_source.is_some() {
                        rejections.push(BrowserRuntimeMigrationRejection {
                            source_path: sources.session_state_path.to_path_buf(),
                            code: "legacy_session_schema_unsupported",
                            detail: state.schema_version,
                        });
                    }
                    BrowserSessionState::default()
                }
                Err(error) => {
                    if session_source.is_some() {
                        rejections.push(BrowserRuntimeMigrationRejection {
                            source_path: sources.session_state_path.to_path_buf(),
                            code: "legacy_session_invalid",
                            detail: error.to_string(),
                        });
                    }
                    BrowserSessionState::default()
                }
            };

        let service_source = read_optional_source(sources.service_state_path)?;
        let service_raw = service_source.clone().unwrap_or_else(|| b"{}".to_vec());
        let catalog_source = read_optional_source(sources.profile_catalog_path)?;
        let catalog_raw = match &catalog_source {
            Some(bytes) => bytes.clone(),
            None => serde_json::to_vec(&BrowserProfileCatalog::default()).map_err(|error| {
                format!("browser_runtime_migration_catalog_serialize_failed:{error}")
            })?,
        };
        let mut catalog_imported = false;
        let mut service_imported = false;
        let catalog = match serde_json::from_slice::<BrowserProfileCatalog>(&catalog_raw) {
            Ok(catalog)
                if catalog_source.is_some() && validate_catalog_schema(&catalog).is_ok() =>
            {
                catalog_imported = true;
                catalog
            }
            Ok(_catalog) if catalog_source.is_none() => {
                service_imported = service_source.is_some();
                import_legacy_profiles(sources.service_state_path, &service_raw, &mut rejections)
            }
            Ok(catalog) => {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: sources.profile_catalog_path.to_path_buf(),
                    code: "legacy_profile_catalog_schema_unsupported",
                    detail: catalog.schema_version,
                });
                service_imported = service_source.is_some();
                import_legacy_profiles(sources.service_state_path, &service_raw, &mut rejections)
            }
            Err(error) => {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: sources.profile_catalog_path.to_path_buf(),
                    code: "legacy_profile_catalog_invalid",
                    detail: error.to_string(),
                });
                service_imported = service_source.is_some();
                import_legacy_profiles(sources.service_state_path, &service_raw, &mut rejections)
            }
        };

        let mut source_records = Vec::new();
        if let Some(bytes) = &session_source {
            source_records.push((
                sources.session_state_path,
                bytes.as_slice(),
                session_imported,
            ));
        }
        if let Some(bytes) = &catalog_source {
            source_records.push((
                sources.profile_catalog_path,
                bytes.as_slice(),
                catalog_imported,
            ));
        }
        if let Some(bytes) = &service_source {
            source_records.push((
                sources.service_state_path,
                bytes.as_slice(),
                service_imported,
            ));
        }
        let migration_id = migration_digest(&source_records);
        let archive_directory = path
            .parent()
            .ok_or_else(|| "browser_runtime_database_parent_missing".to_string())?
            .join(format!("migration-archive-{}", &migration_id[..16]));
        create_migration_archive(&archive_directory, &source_records)?;

        let staged_database =
            path.with_extension(format!("sqlite3.migrating-{}", uuid::Uuid::new_v4()));
        let migration_result = (|| -> Result<(), String> {
            let mut connection = open_runtime_connection(&staged_database)?;
            initialize_runtime_schema(&connection)?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| format!("browser_runtime_migration_begin_failed:{error}"))?;
            save_document(
                &transaction,
                SESSION_STATE_DOCUMENT,
                BROWSER_SESSION_STATE_SCHEMA_V1,
                &session_state,
            )?;
            save_document(
                &transaction,
                PROFILE_CATALOG_DOCUMENT,
                BROWSER_PROFILE_CATALOG_SCHEMA_V1,
                &catalog,
            )?;
            save_document(
                &transaction,
                MANAGER_HANDOFF_REGISTRY_DOCUMENT,
                MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
                &BrowserManagerHandoffRegistry::default(),
            )?;
            save_document(
                &transaction,
                ROUTE_KEEPER_AUTHORITY_DOCUMENT,
                ROUTE_KEEPER_AUTHORITY_SCHEMA_V2,
                &RouteKeeperAuthority::default(),
            )?;
            save_runtime_config_row(&transaction, &BrowserRuntimeConfig::default())?;
            for (source_path, bytes, imported) in &source_records {
                transaction
                    .execute(
                        "INSERT INTO migration_sources(source_path, sha256, archive_name, imported) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            source_path.to_string_lossy().as_ref(),
                            sha256_hex(bytes),
                            source_path.file_name().and_then(|name| name.to_str()).unwrap_or("source"),
                            i64::from(*imported)
                        ],
                    )
                    .map_err(|error| format!("browser_runtime_migration_source_record_failed:{error}"))?;
            }
            for rejection in &rejections {
                transaction
                    .execute(
                        "INSERT INTO migration_rejections(source_path, code, detail) VALUES (?1, ?2, ?3)",
                        params![
                            rejection.source_path.to_string_lossy().as_ref(),
                            rejection.code,
                            rejection.detail
                        ],
                    )
                    .map_err(|error| format!("browser_runtime_migration_rejection_record_failed:{error}"))?;
            }
            transaction
                .execute(
                    "INSERT INTO runtime_metadata(key, value) VALUES ('migration_id', ?1)",
                    params![migration_id],
                )
                .and_then(|_| {
                    transaction.execute(
                        "INSERT INTO runtime_metadata(key, value) VALUES ('migration_archive', ?1)",
                        params![archive_directory.to_string_lossy().as_ref()],
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
            set_private_file(&staged_database)?;
            atomic_replace(&staged_database, path).map_err(|error| {
                error.replace("browser_session_json", "browser_runtime_database")
            })?;
            Ok(())
        })();
        if migration_result.is_err() {
            remove_sqlite_files(&staged_database);
        }
        migration_result?;
        remove_sqlite_sidecars(&staged_database);
        set_archive_read_only(&archive_directory)?;

        Self::open(path)?.migration_receipt()
    }

    fn migration_receipt(&self) -> Result<BrowserRuntimeMigrationReceipt, String> {
        let archive: String = self
            .connection
            .query_row(
                "SELECT value FROM runtime_metadata WHERE key = 'migration_archive'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("browser_runtime_migration_receipt_missing:{error}"))?;
        let imported_source_count = self
            .connection
            .query_row(
                "SELECT count(*) FROM migration_sources WHERE imported = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| format!("browser_runtime_migration_receipt_read_failed:{error}"))?
            as usize;
        let rejected_record_count = self
            .connection
            .query_row("SELECT count(*) FROM migration_rejections", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|error| format!("browser_runtime_migration_receipt_read_failed:{error}"))?
            as usize;
        let mut statement = self
            .connection
            .prepare("SELECT code FROM migration_rejections ORDER BY code, sequence")
            .map_err(|error| format!("browser_runtime_migration_receipt_read_failed:{error}"))?;
        let rejection_codes = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("browser_runtime_migration_receipt_read_failed:{error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("browser_runtime_migration_receipt_read_failed:{error}"))?;
        Ok(BrowserRuntimeMigrationReceipt {
            imported_source_count,
            rejected_record_count,
            rejection_codes,
            archive_directory: PathBuf::from(archive),
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

    pub(crate) fn load_profile_catalog(&self) -> Result<BrowserProfileCatalog, String> {
        load_document(
            &self.connection,
            PROFILE_CATALOG_DOCUMENT,
            BROWSER_PROFILE_CATALOG_SCHEMA_V1,
        )
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

    pub(crate) fn load_handoff_registry(&self) -> Result<BrowserManagerHandoffRegistry, String> {
        load_optional_document(
            &self.connection,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )
    }

    pub(crate) fn load_route_keeper_authority(&self) -> Result<RouteKeeperAuthority, String> {
        load_route_keeper_authority_document(&self.connection)
    }

    pub(crate) fn publish_route_keeper_connection_catalog(
        &mut self,
        catalog: RouteKeeperConnectionCatalog,
    ) -> Result<RouteKeeperConnectionCatalogPublication, String> {
        let current = self.load_route_keeper_authority()?;
        if current.records.keys().collect::<Vec<_>>() != catalog.bindings.keys().collect::<Vec<_>>()
        {
            return Err("route_keeper_connection_catalog_slots_incomplete".to_string());
        }
        if current.connection_catalog == catalog {
            return Ok(RouteKeeperConnectionCatalogPublication::Unchanged);
        }
        let mut next = current.clone();
        next.replace_connection_catalog(catalog)?;
        self.compare_and_swap_route_keeper_authority(&current, &next)?;
        Ok(RouteKeeperConnectionCatalogPublication::Published)
    }

    pub(crate) fn compare_and_swap_route_keeper_authority(
        &mut self,
        expected: &RouteKeeperAuthority,
        next: &RouteKeeperAuthority,
    ) -> Result<(), String> {
        expected.projection()?;
        next.projection()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("route_keeper_authority_begin_failed:{error}"))?;
        let current = load_route_keeper_authority_document(&transaction)?;
        if current != *expected {
            return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
        }
        if current.connection_catalog != next.connection_catalog
            && current
                .records
                .values()
                .any(|record| record.phase != agent_browser_service_model::RouteKeeperPhase::Absent)
        {
            return Err("route_keeper_connection_catalog_active".to_string());
        }
        for (slot_id, current_record) in &current.records {
            let next_record = next
                .records
                .get(slot_id)
                .ok_or_else(|| "route_keeper_authority_generation_regression".to_string())?;
            if next_record.fence.host_generation < current_record.fence.host_generation
                || (next_record.fence.host_generation == current_record.fence.host_generation
                    && next_record.fence.operation_generation
                        < current_record.fence.operation_generation)
            {
                return Err("route_keeper_authority_generation_regression".to_string());
            }
        }
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V2,
            next,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("route_keeper_authority_commit_failed:{error}"))
    }

    pub(crate) fn load_runtime_config(&self) -> Result<BrowserRuntimeConfig, String> {
        load_runtime_config_row(&self.connection)
    }

    pub(crate) fn compare_and_swap_runtime_config(
        &mut self,
        expected_revision: u64,
        mut next: BrowserRuntimeConfig,
    ) -> Result<BrowserRuntimeConfig, String> {
        validate_runtime_config(&next)?;
        if next.revision != expected_revision {
            return Err(format!(
                "browser_runtime_config_proposed_revision_invalid:{}:{expected_revision}",
                next.revision
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_config_begin_failed:{error}"))?;
        let current = load_runtime_config_row(&transaction)?;
        if current.revision != expected_revision {
            return Err(format!(
                "browser_runtime_config_revision_conflict:{expected_revision}:{}",
                current.revision
            ));
        }
        next.revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| "browser_runtime_config_revision_exhausted".to_string())?;
        save_runtime_config_row(&transaction, &next)?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_config_commit_failed:{error}"))?;
        Ok(next)
    }

    pub(crate) fn reserve_operation(
        &mut self,
        operation_id: &str,
        owner_key: &str,
        request: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        if operation_id.is_empty() || owner_key.is_empty() {
            return Err("browser_runtime_operation_identity_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_operation_begin_failed:{error}"))?;
        if let Some(existing) = load_optional_operation(&transaction, operation_id)? {
            if existing.owner_key == owner_key && existing.request == request {
                return Ok(existing);
            }
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        let current_generation = load_owner_generation(&transaction, owner_key)?;
        let generation = current_generation
            .checked_add(1)
            .ok_or_else(|| "browser_runtime_operation_generation_exhausted".to_string())?;
        let generation_sql = i64::try_from(generation)
            .map_err(|_| "browser_runtime_operation_generation_exhausted".to_string())?;
        transaction
            .execute(
                "INSERT INTO operation_generations(owner_key, generation) VALUES (?1, ?2)
                 ON CONFLICT(owner_key) DO UPDATE SET generation=excluded.generation",
                params![owner_key, generation_sql],
            )
            .map_err(|error| format!("browser_runtime_operation_generation_save_failed:{error}"))?;
        let request_json = serde_json::to_string(&request)
            .map_err(|error| format!("browser_runtime_operation_serialize_failed:{error}"))?;
        transaction
            .execute(
                "INSERT INTO operation_records(operation_id, owner_key, generation, state, request_json, result_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                params![
                    operation_id,
                    owner_key,
                    generation_sql,
                    BrowserRuntimeOperationState::Prepared.as_str(),
                    request_json
                ],
            )
            .map_err(|error| format!("browser_runtime_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_operation_commit_failed:{error}"))?;
        Ok(BrowserRuntimeOperation {
            operation_id: operation_id.to_string(),
            owner_key: owner_key.to_string(),
            generation,
            state: BrowserRuntimeOperationState::Prepared,
            request,
            result: None,
        })
    }

    pub(crate) fn commit_operation(
        &mut self,
        operation_id: &str,
        generation: u64,
        result: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_operation_begin_failed:{error}"))?;
        let mut operation = load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| format!("browser_runtime_operation_missing:{operation_id}"))?;
        if operation.generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_mismatch:{operation_id}:{generation}:{}",
                operation.generation
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Committed {
            if operation.result.as_ref() == Some(&result) {
                return Ok(operation);
            }
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Observed
            && operation.result.as_ref() != Some(&result)
        {
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        let current_generation = load_owner_generation(&transaction, &operation.owner_key)?;
        if current_generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_stale:{operation_id}:{generation}:{current_generation}"
            ));
        }
        let result_json = serde_json::to_string(&result)
            .map_err(|error| format!("browser_runtime_operation_serialize_failed:{error}"))?;
        transaction
            .execute(
                "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                params![
                    operation_id,
                    BrowserRuntimeOperationState::Committed.as_str(),
                    result_json
                ],
            )
            .map_err(|error| format!("browser_runtime_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_operation_commit_failed:{error}"))?;
        operation.state = BrowserRuntimeOperationState::Committed;
        operation.result = Some(result);
        Ok(operation)
    }

    pub(crate) fn record_operation_observation(
        &mut self,
        operation_id: &str,
        generation: u64,
        observation: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_operation_begin_failed:{error}"))?;
        let mut operation = load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| format!("browser_runtime_operation_missing:{operation_id}"))?;
        if operation.generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_mismatch:{operation_id}:{generation}:{}",
                operation.generation
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Committed {
            if operation.result.as_ref() == Some(&observation) {
                return Ok(operation);
            }
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        let current_generation = load_owner_generation(&transaction, &operation.owner_key)?;
        if current_generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_stale:{operation_id}:{generation}:{current_generation}"
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Observed
            && operation.result.as_ref() == Some(&observation)
        {
            return Ok(operation);
        }
        let observation_json = serde_json::to_string(&observation)
            .map_err(|error| format!("browser_runtime_operation_serialize_failed:{error}"))?;
        transaction
            .execute(
                "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                params![
                    operation_id,
                    BrowserRuntimeOperationState::Observed.as_str(),
                    observation_json
                ],
            )
            .map_err(|error| format!("browser_runtime_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_operation_commit_failed:{error}"))?;
        operation.state = BrowserRuntimeOperationState::Observed;
        operation.result = Some(observation);
        Ok(operation)
    }

    pub(crate) fn list_pending_operations(&self) -> Result<Vec<BrowserRuntimeOperation>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT operation_id, owner_key, generation, state, request_json, result_json
                 FROM operation_records WHERE state IN ('prepared', 'observed')
                 ORDER BY owner_key, generation, operation_id",
            )
            .map_err(|error| format!("browser_runtime_operation_read_failed:{error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|error| format!("browser_runtime_operation_read_failed:{error}"))?;
        rows.map(|row| {
            let (operation_id, owner_key, generation, state, request_json, result_json) =
                row.map_err(|error| format!("browser_runtime_operation_read_failed:{error}"))?;
            decode_operation(
                operation_id,
                owner_key,
                generation,
                state,
                request_json,
                result_json,
            )
        })
        .collect()
    }

    pub(crate) fn commit_browser_open(
        &mut self,
        operation_id: &str,
        generation: u64,
        expected_base_state: &BrowserSessionState,
        session_state: &BrowserSessionState,
        handoff: &RemoteViewHandoff,
        result: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_operation_begin_failed:{error}"))?;
        let mut operation = load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| format!("browser_runtime_operation_missing:{operation_id}"))?;
        if operation.generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_mismatch:{operation_id}:{generation}:{}",
                operation.generation
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Committed {
            if operation.result.as_ref() == Some(&result) {
                return Ok(operation);
            }
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Observed
            && operation.result.as_ref() != Some(&result)
        {
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        let current_generation = load_owner_generation(&transaction, &operation.owner_key)?;
        if current_generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_stale:{operation_id}:{generation}:{current_generation}"
            ));
        }
        if session_state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                session_state.schema_version
            ));
        }
        if handoff.id.is_empty() {
            return Err("browser_runtime_handoff_identity_invalid".to_string());
        }
        let current_session_state: BrowserSessionState = load_optional_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if current_session_state != *expected_base_state {
            return Err("browser_runtime_operation_base_state_conflict".to_string());
        }
        let mut registry: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        registry
            .handoffs
            .insert(handoff.id.clone(), handoff.clone());
        let result_json = serde_json::to_string(&result)
            .map_err(|error| format!("browser_runtime_operation_serialize_failed:{error}"))?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            session_state,
        )?;
        save_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .execute(
                "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
                params![
                    operation_id,
                    BrowserRuntimeOperationState::Committed.as_str(),
                    result_json
                ],
            )
            .map_err(|error| format!("browser_runtime_operation_save_failed:{error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_operation_commit_failed:{error}"))?;
        operation.state = BrowserRuntimeOperationState::Committed;
        operation.result = Some(result);
        Ok(operation)
    }

    pub(crate) fn load_operation(
        &self,
        operation_id: &str,
    ) -> Result<BrowserRuntimeOperation, String> {
        load_optional_operation(&self.connection, operation_id)?
            .ok_or_else(|| format!("browser_runtime_operation_missing:{operation_id}"))
    }

    pub(crate) fn find_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<BrowserRuntimeOperation>, String> {
        load_optional_operation(&self.connection, operation_id)
    }
}

/// Reads manager state while stopping either side of the one-time cold
/// upgrade boundary. A present SQLite database is authoritative, including
/// when it is corrupt; legacy JSON is considered only before the database has
/// ever been created.
pub(crate) fn load_session_state_for_cold_upgrade(
    service_directory: &Path,
) -> Result<BrowserSessionState, String> {
    let database_path = service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME);
    if database_path.exists() {
        return BrowserRuntimeSqliteStore::open(&database_path)?.load_session_state();
    }
    BrowserSessionJsonStore::new(service_directory).load_session_state()
}

pub(crate) fn load_default_session_state_for_cold_upgrade() -> Result<BrowserSessionState, String> {
    let legacy_state_path = default_service_state_path()?;
    let service_directory = legacy_state_path
        .parent()
        .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
    load_session_state_for_cold_upgrade(service_directory)
}

pub(crate) fn save_session_state_for_cold_upgrade(
    service_directory: &Path,
    state: &BrowserSessionState,
) -> Result<(), String> {
    let database_path = service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME);
    if database_path.exists() {
        return BrowserRuntimeSqliteStore::open(&database_path)?.save_session_state(state);
    }
    BrowserSessionJsonStore::new(service_directory).save_session_state(state)
}

pub(crate) fn save_default_session_state_for_cold_upgrade(
    state: &BrowserSessionState,
) -> Result<(), String> {
    let legacy_state_path = default_service_state_path()?;
    let service_directory = legacy_state_path
        .parent()
        .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
    save_session_state_for_cold_upgrade(service_directory, state)
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
        .busy_timeout(std::time::Duration::from_secs(5))
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
            "PRAGMA auto_vacuum=INCREMENTAL;
             CREATE TABLE IF NOT EXISTS runtime_metadata (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS state_documents (
               kind TEXT PRIMARY KEY,
               schema_version TEXT NOT NULL,
               json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS migration_sources (
               source_path TEXT PRIMARY KEY,
               sha256 TEXT NOT NULL,
               archive_name TEXT NOT NULL,
               imported INTEGER NOT NULL CHECK(imported IN (0, 1))
             );
             CREATE TABLE IF NOT EXISTS migration_rejections (
               sequence INTEGER PRIMARY KEY AUTOINCREMENT,
               source_path TEXT NOT NULL,
               code TEXT NOT NULL,
               detail TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS operation_records (
               operation_id TEXT PRIMARY KEY,
               owner_key TEXT NOT NULL,
               generation INTEGER NOT NULL,
               state TEXT NOT NULL,
               request_json TEXT NOT NULL,
               result_json TEXT
             );
             CREATE TABLE IF NOT EXISTS operation_generations (
               owner_key TEXT PRIMARY KEY,
               generation INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS history_events (
               sequence INTEGER PRIMARY KEY AUTOINCREMENT,
               recorded_at TEXT NOT NULL,
               event_type TEXT NOT NULL,
               payload_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS runtime_config (
               key TEXT PRIMARY KEY,
               value_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS provider_credentials (
               key TEXT PRIMARY KEY,
               value BLOB NOT NULL
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
        .map_err(|error| format!("browser_runtime_document_serialize_failed:{kind}:{error}"))?;
    connection
        .execute(
            "INSERT INTO state_documents(kind, schema_version, json) VALUES (?1, ?2, ?3)
             ON CONFLICT(kind) DO UPDATE SET schema_version=excluded.schema_version, json=excluded.json",
            params![kind, schema_version, json],
        )
        .map_err(|error| format!("browser_runtime_document_save_failed:{kind}:{error}"))?;
    Ok(())
}

fn load_document<T: DeserializeOwned>(
    connection: &Connection,
    kind: &str,
    expected_schema: &str,
) -> Result<T, String> {
    let (schema, json): (String, String) = connection
        .query_row(
            "SELECT schema_version, json FROM state_documents WHERE kind = ?1",
            params![kind],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("browser_runtime_document_missing:{kind}:{error}"))?;
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

fn load_route_keeper_authority_document(
    connection: &Connection,
) -> Result<RouteKeeperAuthority, String> {
    for _ in 0..3 {
        let row = connection
            .query_row(
                "SELECT schema_version, json FROM state_documents WHERE kind = ?1",
                params![ROUTE_KEEPER_AUTHORITY_DOCUMENT],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| {
                format!(
                    "browser_runtime_document_read_failed:{}:{error}",
                    ROUTE_KEEPER_AUTHORITY_DOCUMENT
                )
            })?;
        let Some((schema, json)) = row else {
            return Ok(RouteKeeperAuthority::default());
        };
        let mut authority: RouteKeeperAuthority = serde_json::from_str(&json).map_err(|error| {
            format!(
                "browser_runtime_document_invalid:{}:{error}",
                ROUTE_KEEPER_AUTHORITY_DOCUMENT
            )
        })?;
        if schema == ROUTE_KEEPER_AUTHORITY_SCHEMA_V1 {
            authority = authority.upgrade_from_v1()?;
            if compare_and_swap_route_keeper_v1_upgrade(connection, &json, &authority)? {
                authority.projection()?;
                return Ok(authority);
            }
            continue;
        }
        if schema != ROUTE_KEEPER_AUTHORITY_SCHEMA_V2 {
            return Err(format!(
                "browser_runtime_document_schema_unsupported:{}:{schema}",
                ROUTE_KEEPER_AUTHORITY_DOCUMENT
            ));
        }
        authority.projection()?;
        return Ok(authority);
    }
    Err("route_keeper_authority_migration_compare_and_swap_exhausted".to_string())
}

fn compare_and_swap_route_keeper_v1_upgrade(
    connection: &Connection,
    expected_json: &str,
    upgraded: &RouteKeeperAuthority,
) -> Result<bool, String> {
    let upgraded_json = serde_json::to_string(upgraded).map_err(|error| {
        format!(
            "browser_runtime_document_serialize_failed:{}:{error}",
            ROUTE_KEEPER_AUTHORITY_DOCUMENT
        )
    })?;
    let changed = connection
        .execute(
            "UPDATE state_documents
             SET schema_version = ?1, json = ?2
             WHERE kind = ?3 AND schema_version = ?4 AND json = ?5",
            params![
                ROUTE_KEEPER_AUTHORITY_SCHEMA_V2,
                upgraded_json,
                ROUTE_KEEPER_AUTHORITY_DOCUMENT,
                ROUTE_KEEPER_AUTHORITY_SCHEMA_V1,
                expected_json
            ],
        )
        .map_err(|error| format!("route_keeper_authority_migration_save_failed:{error}"))?;
    Ok(changed == 1)
}

fn load_runtime_config_row(connection: &Connection) -> Result<BrowserRuntimeConfig, String> {
    let json: String = connection
        .query_row(
            "SELECT value_json FROM runtime_config WHERE key = ?1",
            params![RUNTIME_CONFIG_KEY],
            |row| row.get(0),
        )
        .map_err(|error| format!("browser_runtime_config_missing:{error}"))?;
    let config: BrowserRuntimeConfig = serde_json::from_str(&json)
        .map_err(|error| format!("browser_runtime_config_invalid:{error}"))?;
    validate_runtime_config(&config)?;
    Ok(config)
}

fn save_runtime_config_row(
    connection: &Connection,
    config: &BrowserRuntimeConfig,
) -> Result<(), String> {
    validate_runtime_config(config)?;
    let json = serde_json::to_string(config)
        .map_err(|error| format!("browser_runtime_config_serialize_failed:{error}"))?;
    connection
        .execute(
            "INSERT INTO runtime_config(key, value_json) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            params![RUNTIME_CONFIG_KEY, json],
        )
        .map_err(|error| format!("browser_runtime_config_save_failed:{error}"))?;
    Ok(())
}

fn validate_runtime_config(config: &BrowserRuntimeConfig) -> Result<(), String> {
    if config.schema_version != BROWSER_RUNTIME_CONFIG_SCHEMA_V1 {
        return Err(format!(
            "browser_runtime_config_schema_unsupported:{}",
            config.schema_version
        ));
    }
    if config.minimum_ready == 0 {
        return Err("browser_runtime_config_minimum_ready_invalid".to_string());
    }
    if config.maximum_displays < config.minimum_ready {
        return Err("browser_runtime_config_maximum_displays_invalid".to_string());
    }
    if config.warm_target < config.minimum_ready || config.warm_target > config.maximum_displays {
        return Err("browser_runtime_config_warm_target_invalid".to_string());
    }
    if config.maximum_browsers_per_display == 0 {
        return Err("browser_runtime_config_display_density_invalid".to_string());
    }
    if config.maximum_queue_depth == 0 {
        return Err("browser_runtime_config_queue_depth_invalid".to_string());
    }
    if config.request_deadline_ms == 0 {
        return Err("browser_runtime_config_request_deadline_invalid".to_string());
    }
    if config.scale_in_cooldown_ms == 0 {
        return Err("browser_runtime_config_scale_in_cooldown_invalid".to_string());
    }
    if config.session_idle_timeout_ms == 0 {
        return Err("browser_runtime_config_session_idle_timeout_invalid".to_string());
    }
    if config.disposable_inactivity_ms == 0 {
        return Err("browser_runtime_config_disposable_inactivity_invalid".to_string());
    }
    if config.maximum_retained_disposable_profiles == 0 {
        return Err("browser_runtime_config_disposable_count_invalid".to_string());
    }
    if config.maximum_disposable_profile_bytes == 0 {
        return Err("browser_runtime_config_disposable_bytes_invalid".to_string());
    }
    if config.exact_url_history_maximum_bytes == 0
        || config.exact_url_history_maximum_bytes > config.live_database_maximum_bytes
    {
        return Err("browser_runtime_config_url_history_bytes_invalid".to_string());
    }
    if config.live_database_maximum_bytes == 0
        || config.live_database_maximum_bytes > config.routine_storage_maximum_bytes
    {
        return Err("browser_runtime_config_database_bytes_invalid".to_string());
    }
    Ok(())
}

fn load_owner_generation(connection: &Connection, owner_key: &str) -> Result<u64, String> {
    let generation = connection
        .query_row(
            "SELECT generation FROM operation_generations WHERE owner_key = ?1",
            params![owner_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| format!("browser_runtime_operation_generation_read_failed:{error}"))?
        .unwrap_or(0);
    u64::try_from(generation)
        .map_err(|_| "browser_runtime_operation_generation_invalid".to_string())
}

fn load_optional_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<BrowserRuntimeOperation>, String> {
    let row = connection
        .query_row(
            "SELECT owner_key, generation, state, request_json, result_json
             FROM operation_records WHERE operation_id = ?1",
            params![operation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("browser_runtime_operation_read_failed:{error}"))?;
    let Some((owner_key, generation, state, request_json, result_json)) = row else {
        return Ok(None);
    };
    Ok(Some(decode_operation(
        operation_id.to_string(),
        owner_key,
        generation,
        state,
        request_json,
        result_json,
    )?))
}

fn decode_operation(
    operation_id: String,
    owner_key: String,
    generation: i64,
    state: String,
    request_json: String,
    result_json: Option<String>,
) -> Result<BrowserRuntimeOperation, String> {
    let generation = u64::try_from(generation)
        .map_err(|_| "browser_runtime_operation_generation_invalid".to_string())?;
    let request = serde_json::from_str(&request_json)
        .map_err(|error| format!("browser_runtime_operation_request_invalid:{error}"))?;
    let result = result_json
        .map(|json| {
            serde_json::from_str(&json)
                .map_err(|error| format!("browser_runtime_operation_result_invalid:{error}"))
        })
        .transpose()?;
    Ok(BrowserRuntimeOperation {
        operation_id,
        owner_key,
        generation,
        state: BrowserRuntimeOperationState::parse(&state)?,
        request,
        result,
    })
}

fn prepare_private_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "browser_runtime_database_parent_missing".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "browser_runtime_database_directory_failed:{}:{error}",
            parent.display()
        )
    })?;
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

fn read_optional_source(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "browser_runtime_migration_source_read_failed:{}:{error}",
            path.display()
        )),
    }
}

fn import_legacy_profiles(
    service_state_path: &Path,
    service_raw: &[u8],
    rejections: &mut Vec<BrowserRuntimeMigrationRejection>,
) -> BrowserProfileCatalog {
    let legacy = String::from_utf8_lossy(service_raw);
    let imported = BrowserProfileCatalog::import_legacy_service_state_json(&legacy);
    for diagnostic in imported.diagnostics {
        rejections.push(BrowserRuntimeMigrationRejection {
            source_path: service_state_path.to_path_buf(),
            code: "legacy_profile_rejected",
            detail: match diagnostic.profile_key {
                Some(profile_key) => format!("{profile_key}:{}", diagnostic.reason),
                None => diagnostic.reason,
            },
        });
    }
    imported.catalog
}

fn migration_digest(sources: &[(&Path, &[u8], bool)]) -> String {
    let mut digest = Sha256::new();
    for (path, bytes, imported) in sources {
        digest.update(path.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(bytes);
        digest.update([0]);
        digest.update([u8::from(*imported)]);
    }
    hex::encode(digest.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn create_migration_archive(
    archive_directory: &Path,
    sources: &[(&Path, &[u8], bool)],
) -> Result<(), String> {
    fs::create_dir_all(archive_directory).map_err(|error| {
        format!(
            "browser_runtime_migration_archive_directory_failed:{}:{error}",
            archive_directory.display()
        )
    })?;
    for (source, bytes, _) in sources {
        let name = source
            .file_name()
            .ok_or_else(|| "browser_runtime_migration_source_name_missing".to_string())?;
        fs::write(archive_directory.join(name), bytes)
            .map_err(|error| format!("browser_runtime_migration_archive_write_failed:{error}"))?;
    }
    Ok(())
}

fn set_archive_read_only(path: &Path) -> Result<(), String> {
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
    Ok(())
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
        BrowserProfileKind, RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog,
    };

    fn route_keeper_catalog() -> RouteKeeperConnectionCatalog {
        RouteKeeperConnectionCatalog::new((1_u32..=6).map(|sequence| {
            RouteKeeperConnectionBinding {
                slot_id: format!("route-slot-{sequence:02}"),
                connection_key: format!("route-{sequence:02}"),
                connection_name: format!("Agent Browser Route {sequence:02}"),
                guacamole_connection_id: u64::from(sequence),
            }
        }))
        .unwrap()
    }

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
    fn cold_migration_makes_sqlite_authoritative_and_archives_json_sources() {
        let directory = TempDirectory::new("browser-runtime-sqlite-migration");
        let service_directory = directory.0.join("service");
        fs::create_dir_all(&service_directory).unwrap();
        let session_path = service_directory.join(BROWSER_SESSION_STATE_FILENAME);
        let catalog_path = service_directory.join(BROWSER_PROFILE_CATALOG_FILENAME);
        let legacy_service_path = service_directory.join("state.json");
        let database_path = service_directory.join("runtime.sqlite3");

        let mut session_state = BrowserSessionState::default();
        session_state.next_session_sequence = 42;
        write_private_json_atomic(&session_path, &session_state).unwrap();
        fs::write(
            &catalog_path,
            serde_json::json!({
                "schemaVersion": BROWSER_PROFILE_CATALOG_SCHEMA_V1,
                "profiles": {},
                "disposablePolicies": {}
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            &legacy_service_path,
            serde_json::json!({"profiles": {}, "obsoleteRoute": "guacamole:1"}).to_string(),
        )
        .unwrap();

        let migration = BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &legacy_service_path,
            },
        )
        .unwrap();

        assert_eq!(migration.imported_source_count, 2);
        assert!(migration.archive_directory.is_dir());
        assert!(migration
            .archive_directory
            .join(BROWSER_SESSION_STATE_FILENAME)
            .is_file());
        assert!(migration
            .archive_directory
            .join(BROWSER_PROFILE_CATALOG_FILENAME)
            .is_file());
        assert!(migration.archive_directory.join("state.json").is_file());
        assert!(fs::read_dir(&service_directory).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".migrating-")
        }));
        let replayed = BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &legacy_service_path,
            },
        )
        .unwrap();
        assert_eq!(replayed, migration);

        fs::write(&session_path, "not authoritative after migration").unwrap();
        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            store.load_session_state().unwrap().next_session_sequence,
            42
        );
    }

    #[test]
    fn sqlite_store_round_trips_session_state_and_profile_catalog() {
        let directory = TempDirectory::new("browser-runtime-sqlite-round-trip");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        let session_path = directory.0.join(BROWSER_SESSION_STATE_FILENAME);
        let catalog_path = directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME);
        let service_path = directory.0.join("state.json");
        write_private_json_atomic(&session_path, &BrowserSessionState::default()).unwrap();
        write_private_json_atomic(&catalog_path, &BrowserProfileCatalog::default()).unwrap();
        fs::write(&service_path, "{}").unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &service_path,
            },
        )
        .unwrap();

        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let mut state = store.load_session_state().unwrap();
        state.next_session_sequence = 17;
        store.save_session_state(&state).unwrap();
        let mut catalog = store.load_profile_catalog().unwrap();
        catalog.profiles.insert(
            "work".to_string(),
            agent_browser_service_model::BrowserProfileCatalogEntry {
                id: "work".to_string(),
                name: "Work".to_string(),
                user_data_dir: "/managed/work".to_string(),
                kind: BrowserProfileKind::Named,
            },
        );
        store.save_profile_catalog(&catalog).unwrap();
        drop(store);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(reopened.load_session_state().unwrap(), state);
        assert_eq!(reopened.load_profile_catalog().unwrap(), catalog);
    }

    #[test]
    fn sqlite_store_seeds_and_isolates_route_keeper_authority() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-authority");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();

        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let session_before = store.load_session_state().unwrap();
        let handoffs_before = store.load_handoff_registry().unwrap();
        let original_authority = store.load_route_keeper_authority().unwrap();
        assert_eq!(original_authority, RouteKeeperAuthority::default());
        let mut authority = original_authority.clone();
        authority
            .replace_connection_catalog(route_keeper_catalog())
            .unwrap();

        let action = authority.next_reconcile_action().unwrap();
        let (slot_id, fence) = match action {
            agent_browser_service_model::RouteKeeperReconcileAction::Start {
                slot_id,
                fence,
                ..
            } => (slot_id, fence),
            other => panic!("expected start action, got {other:?}"),
        };
        authority.record_observing(&slot_id, &fence).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&original_authority, &authority)
            .unwrap();

        assert_eq!(store.load_route_keeper_authority().unwrap(), authority);
        assert_eq!(store.load_session_state().unwrap(), session_before);
        assert_eq!(store.load_handoff_registry().unwrap(), handoffs_before);
        drop(store);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(reopened.load_route_keeper_authority().unwrap(), authority);
    }

    #[test]
    fn route_keeper_catalog_publication_atomically_updates_the_complete_authority() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-catalog-publish");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let catalog = route_keeper_catalog();

        assert_eq!(
            store
                .publish_route_keeper_connection_catalog(catalog.clone())
                .unwrap(),
            RouteKeeperConnectionCatalogPublication::Published
        );
        let authority = store.load_route_keeper_authority().unwrap();
        assert_eq!(authority.connection_catalog, catalog);
        let digest = authority.connection_catalog.digest().unwrap();
        assert!(authority
            .records
            .values()
            .all(|record| record.fence.connection_catalog_digest == digest));
    }

    #[test]
    fn exact_catalog_replay_is_unchanged_while_a_keeper_is_active() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-catalog-replay");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let catalog = route_keeper_catalog();
        store
            .publish_route_keeper_connection_catalog(catalog.clone())
            .unwrap();
        let before_start = store.load_route_keeper_authority().unwrap();
        let mut active = before_start.clone();
        active.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&before_start, &active)
            .unwrap();

        assert_eq!(
            store
                .publish_route_keeper_connection_catalog(catalog)
                .unwrap(),
            RouteKeeperConnectionCatalogPublication::Unchanged
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), active);
    }

    #[test]
    fn changed_catalog_is_rejected_while_a_keeper_is_active() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-catalog-active");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let catalog = route_keeper_catalog();
        store
            .publish_route_keeper_connection_catalog(catalog.clone())
            .unwrap();
        let before_start = store.load_route_keeper_authority().unwrap();
        let mut active = before_start.clone();
        active.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&before_start, &active)
            .unwrap();
        let mut changed = catalog;
        changed
            .bindings
            .get_mut("route-slot-01")
            .unwrap()
            .guacamole_connection_id = 101;

        assert_eq!(
            store.publish_route_keeper_connection_catalog(changed),
            Err("route_keeper_connection_catalog_active".to_string())
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), active);
    }

    #[test]
    fn catalog_publication_rejects_an_incomplete_slot_set() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-catalog-incomplete");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let before = store.load_route_keeper_authority().unwrap();
        let mut bindings = route_keeper_catalog().bindings;
        bindings.remove("route-slot-06");
        let incomplete = RouteKeeperConnectionCatalog::new(bindings.into_values()).unwrap();

        assert_eq!(
            store.publish_route_keeper_connection_catalog(incomplete),
            Err("route_keeper_connection_catalog_slots_incomplete".to_string())
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), before);
    }

    #[test]
    fn empty_catalog_replay_is_rejected_when_the_authority_has_slots() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-catalog-empty-replay");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();

        assert_eq!(
            store.publish_route_keeper_connection_catalog(RouteKeeperConnectionCatalog::default()),
            Err("route_keeper_connection_catalog_slots_incomplete".to_string())
        );
    }

    #[test]
    fn equal_partial_catalog_replay_is_rejected() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-catalog-partial-replay");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let before = store.load_route_keeper_authority().unwrap();
        let mut bindings = route_keeper_catalog().bindings;
        bindings.remove("route-slot-06");
        let partial = RouteKeeperConnectionCatalog::new(bindings.into_values()).unwrap();
        let mut partial_authority = before.clone();
        partial_authority
            .replace_connection_catalog(partial.clone())
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&before, &partial_authority)
            .unwrap();

        assert_eq!(
            store.publish_route_keeper_connection_catalog(partial),
            Err("route_keeper_connection_catalog_slots_incomplete".to_string())
        );
        assert_eq!(
            store.load_route_keeper_authority().unwrap(),
            partial_authority
        );
    }

    #[test]
    fn route_keeper_authority_compare_and_swap_rejects_stale_and_regressing_writers() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-cas");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let empty = store.load_route_keeper_authority().unwrap();
        let mut configured = empty.clone();
        configured
            .replace_connection_catalog(route_keeper_catalog())
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&empty, &configured)
            .unwrap();
        let stale = store.load_route_keeper_authority().unwrap();
        let mut current = stale.clone();
        current.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&stale, &current)
            .unwrap();

        let mut stale_next = stale.clone();
        stale_next.next_reconcile_action().unwrap();
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(&stale, &stale_next),
            Err("route_keeper_authority_compare_and_swap_conflict".to_string())
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), current);

        let mut regressing = current.clone();
        let record = regressing.records.get_mut("route-slot-01").unwrap();
        record.fence.operation_generation = 0;
        record.fence.operation_id.clear();
        record.phase = agent_browser_service_model::RouteKeeperPhase::Absent;
        record.protocol_ready = None;
        record.adoption = None;
        record.cleanup_obligation = None;
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(&current, &regressing),
            Err("route_keeper_authority_generation_regression".to_string())
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), current);
    }

    #[test]
    fn missing_route_keeper_document_loads_a_fresh_authority() {
        let directory = TempDirectory::new("browser-runtime-missing-route-keeper-document");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        store
            .connection
            .execute(
                "DELETE FROM state_documents WHERE kind = ?1",
                params![ROUTE_KEEPER_AUTHORITY_DOCUMENT],
            )
            .unwrap();

        assert_eq!(
            store.load_route_keeper_authority().unwrap(),
            RouteKeeperAuthority::default()
        );
    }

    #[test]
    fn route_keeper_v1_document_migrates_in_place_to_v2_with_empty_catalog() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-v1-migration");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let mut legacy = serde_json::to_value(RouteKeeperAuthority::default()).unwrap();
        legacy["schemaVersion"] = serde_json::json!(ROUTE_KEEPER_AUTHORITY_SCHEMA_V1);
        legacy.as_object_mut().unwrap().remove("connectionCatalog");
        for record in legacy["records"].as_object_mut().unwrap().values_mut() {
            record["fence"]
                .as_object_mut()
                .unwrap()
                .remove("connectionCatalogDigest");
        }
        let legacy_json = serde_json::to_string(&legacy).unwrap();
        store
            .connection
            .execute(
                "UPDATE state_documents SET schema_version = ?1, json = ?2 WHERE kind = ?3",
                params![
                    ROUTE_KEEPER_AUTHORITY_SCHEMA_V1,
                    legacy_json,
                    ROUTE_KEEPER_AUTHORITY_DOCUMENT
                ],
            )
            .unwrap();

        let migrated = store.load_route_keeper_authority().unwrap();
        assert_eq!(
            migrated.schema_version,
            agent_browser_service_model::ROUTE_KEEPER_AUTHORITY_SCHEMA_V2
        );
        assert!(migrated.connection_catalog.bindings.is_empty());
        let digest = migrated.connection_catalog.digest().unwrap();
        assert!(migrated
            .records
            .values()
            .all(|record| record.fence.connection_catalog_digest == digest));
        let persisted_schema: String = store
            .connection
            .query_row(
                "SELECT schema_version FROM state_documents WHERE kind = ?1",
                params![ROUTE_KEEPER_AUTHORITY_DOCUMENT],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(persisted_schema, ROUTE_KEEPER_AUTHORITY_SCHEMA_V2);

        let mut newer = migrated.clone();
        newer
            .replace_connection_catalog(route_keeper_catalog())
            .unwrap();
        let (slot_id, keeper_id, fence) = match newer.next_reconcile_action().unwrap() {
            agent_browser_service_model::RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                ..
            } => (slot_id, keeper_id, fence),
            other => panic!("expected start action, got {other:?}"),
        };
        newer
            .record_protocol_ready(
                agent_browser_service_model::RouteKeeperProtocolReadyReceipt {
                    slot_id,
                    keeper_id,
                    fence,
                    guacamole_connection_uuid: "guacamole-01".to_string(),
                    xrdp_session_id: "xrdp-01".to_string(),
                    display_name: ":10".to_string(),
                    observed_at: "2026-09-19T23:00:00Z".to_string(),
                },
            )
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&migrated, &newer)
            .unwrap();

        let stale: RouteKeeperAuthority = serde_json::from_str(&legacy_json).unwrap();
        let stale = stale.upgrade_from_v1().unwrap();
        assert!(
            !compare_and_swap_route_keeper_v1_upgrade(&store.connection, &legacy_json, &stale,)
                .unwrap()
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), newer);

        fn remove_catalog_digests(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Object(object) => {
                    object.remove("connectionCatalogDigest");
                    for nested in object.values_mut() {
                        remove_catalog_digests(nested);
                    }
                }
                serde_json::Value::Array(values) => {
                    for nested in values {
                        remove_catalog_digests(nested);
                    }
                }
                _ => {}
            }
        }
        let mut active_v1 = serde_json::to_value(&newer).unwrap();
        active_v1["schemaVersion"] = serde_json::json!(ROUTE_KEEPER_AUTHORITY_SCHEMA_V1);
        active_v1
            .as_object_mut()
            .unwrap()
            .remove("connectionCatalog");
        remove_catalog_digests(&mut active_v1);
        store
            .connection
            .execute(
                "UPDATE state_documents SET schema_version = ?1, json = ?2 WHERE kind = ?3",
                params![
                    ROUTE_KEEPER_AUTHORITY_SCHEMA_V1,
                    serde_json::to_string(&active_v1).unwrap(),
                    ROUTE_KEEPER_AUTHORITY_DOCUMENT
                ],
            )
            .unwrap();
        assert_eq!(
            store.load_route_keeper_authority(),
            Err("route_keeper_v1_active_migration_unproven".to_string())
        );
        let retained_schema: String = store
            .connection
            .query_row(
                "SELECT schema_version FROM state_documents WHERE kind = ?1",
                params![ROUTE_KEEPER_AUTHORITY_DOCUMENT],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained_schema, ROUTE_KEEPER_AUTHORITY_SCHEMA_V1);
    }

    #[test]
    fn cold_migration_rejects_bad_records_without_blocking_valid_legacy_profiles() {
        let directory = TempDirectory::new("browser-runtime-tolerant-migration");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        let session_path = directory.0.join(BROWSER_SESSION_STATE_FILENAME);
        let catalog_path = directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME);
        let service_path = directory.0.join("state.json");
        fs::write(
            &session_path,
            serde_json::json!({"schemaVersion": "future-session-schema"}).to_string(),
        )
        .unwrap();
        fs::write(&catalog_path, "not-json").unwrap();
        fs::write(
            &service_path,
            serde_json::json!({
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": "/managed/work",
                        "profileClass": "durable_named"
                    },
                    "broken": {
                        "id": "broken",
                        "name": "",
                        "profileClass": "durable_named"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let receipt = BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &service_path,
            },
        )
        .unwrap();

        assert_eq!(receipt.rejected_record_count, 3);
        assert_eq!(
            receipt.rejection_codes,
            vec![
                "legacy_profile_catalog_invalid",
                "legacy_profile_rejected",
                "legacy_session_schema_unsupported",
            ]
        );
        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert!(store.load_session_state().unwrap().sessions.is_empty());
        assert!(store
            .load_profile_catalog()
            .unwrap()
            .profiles
            .contains_key("work"));
        assert_eq!(
            fs::read_to_string(
                receipt
                    .archive_directory
                    .join(BROWSER_SESSION_STATE_FILENAME)
            )
            .unwrap(),
            serde_json::json!({"schemaVersion": "future-session-schema"}).to_string()
        );
        assert_eq!(
            fs::read_to_string(
                receipt
                    .archive_directory
                    .join(BROWSER_PROFILE_CATALOG_FILENAME)
            )
            .unwrap(),
            "not-json"
        );
    }

    #[test]
    fn clean_migration_does_not_invent_legacy_source_history() {
        let directory = TempDirectory::new("browser-runtime-clean-migration");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        let session_path = directory.0.join(BROWSER_SESSION_STATE_FILENAME);
        let catalog_path = directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME);
        let service_path = directory.0.join("state.json");

        let receipt = BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &service_path,
            },
        )
        .unwrap();

        assert_eq!(receipt.imported_source_count, 0);
        assert_eq!(receipt.rejected_record_count, 0);
        assert_eq!(fs::read_dir(&receipt.archive_directory).unwrap().count(), 0);
        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            store.load_session_state().unwrap(),
            BrowserSessionState::default()
        );
        assert_eq!(
            store.load_profile_catalog().unwrap(),
            BrowserProfileCatalog::default()
        );
    }

    #[test]
    fn runtime_config_defaults_and_compare_and_swap_are_durable() {
        let directory = TempDirectory::new("browser-runtime-config");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();

        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let defaults = store.load_runtime_config().unwrap();
        assert_eq!(defaults.revision, 0);
        assert_eq!(defaults.minimum_ready, 1);
        assert_eq!(defaults.warm_target, 4);
        assert_eq!(defaults.maximum_displays, 6);
        assert_eq!(defaults.maximum_browsers_per_display, 4);
        assert_eq!(defaults.maximum_queue_depth, 32);
        assert_eq!(defaults.request_deadline_ms, 90_000);
        assert_eq!(defaults.scale_in_cooldown_ms, 600_000);
        assert_eq!(defaults.session_idle_timeout_ms, 300_000);
        assert_eq!(defaults.disposable_inactivity_ms, 86_400_000);
        assert_eq!(defaults.maximum_retained_disposable_profiles, 20);
        assert_eq!(
            defaults.maximum_disposable_profile_bytes,
            10 * 1024 * 1024 * 1024
        );

        let mut changed = defaults.clone();
        changed.maximum_displays = 5;
        let committed = store.compare_and_swap_runtime_config(0, changed).unwrap();
        assert_eq!(committed.revision, 1);
        assert_eq!(committed.maximum_displays, 5);
        let stale = store
            .compare_and_swap_runtime_config(0, defaults.clone())
            .unwrap_err();
        assert_eq!(stale, "browser_runtime_config_revision_conflict:0:1");
        drop(store);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(reopened.load_runtime_config().unwrap(), committed);
    }

    #[test]
    fn runtime_config_rejects_invalid_capacity_without_mutation() {
        let directory = TempDirectory::new("browser-runtime-config-validation");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let defaults = store.load_runtime_config().unwrap();
        let mut invalid = defaults.clone();
        invalid.maximum_displays = 0;

        assert_eq!(
            store.compare_and_swap_runtime_config(0, invalid),
            Err("browser_runtime_config_maximum_displays_invalid".to_string())
        );
        assert_eq!(store.load_runtime_config().unwrap(), defaults);
    }

    #[test]
    fn operation_journal_replays_requests_and_fences_stale_effect_commits() {
        let directory = TempDirectory::new("browser-runtime-operation-journal");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();

        let first = store
            .reserve_operation("open-1", "session:alice", serde_json::json!({"url": "a"}))
            .unwrap();
        assert_eq!(first.generation, 1);
        assert_eq!(first.state, BrowserRuntimeOperationState::Prepared);
        assert_eq!(
            store
                .reserve_operation("open-1", "session:alice", serde_json::json!({"url": "a"}))
                .unwrap(),
            first
        );
        assert_eq!(
            store.reserve_operation(
                "open-1",
                "session:alice",
                serde_json::json!({"url": "different"})
            ),
            Err("browser_runtime_operation_replay_mismatch:open-1".to_string())
        );

        let second = store
            .reserve_operation("open-2", "session:alice", serde_json::json!({"url": "b"}))
            .unwrap();
        assert_eq!(second.generation, 2);
        assert_eq!(
            store.commit_operation("open-1", 1, serde_json::json!({"browserId": "stale"})),
            Err("browser_runtime_operation_generation_stale:open-1:1:2".to_string())
        );
        let committed = store
            .commit_operation("open-2", 2, serde_json::json!({"browserId": "current"}))
            .unwrap();
        assert_eq!(committed.state, BrowserRuntimeOperationState::Committed);
        assert_eq!(
            committed.result,
            Some(serde_json::json!({"browserId": "current"}))
        );
        assert_eq!(
            store
                .commit_operation("open-2", 2, serde_json::json!({"browserId": "current"}))
                .unwrap(),
            committed
        );

        let unrelated = store
            .reserve_operation("open-bob", "session:bob", serde_json::json!({}))
            .unwrap();
        assert_eq!(unrelated.generation, 1);
        drop(store);
        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(reopened.load_operation("open-2").unwrap(), committed);
    }

    #[test]
    fn browser_open_transaction_publishes_atomically_and_replays_exactly() {
        let directory = TempDirectory::new("browser-runtime-open-transaction");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let reserved = store
            .reserve_operation(
                "open-atomic",
                "session:atomic",
                serde_json::json!({"url": "https://example.test"}),
            )
            .unwrap();
        let browser_observation = serde_json::json!({
            "phase": "browser_opened",
            "browserId": "browser-atomic"
        });
        store
            .record_operation_observation(
                &reserved.operation_id,
                reserved.generation,
                browser_observation,
            )
            .unwrap();
        let ready_observation = serde_json::json!({
            "phase": "ready",
            "browserId": "browser-atomic",
            "tabId": "tab-atomic"
        });
        let observed = store
            .record_operation_observation(
                &reserved.operation_id,
                reserved.generation,
                ready_observation.clone(),
            )
            .unwrap();
        assert_eq!(observed.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(observed.result, Some(ready_observation.clone()));
        assert_eq!(
            store.list_pending_operations().unwrap(),
            vec![observed.clone()]
        );
        drop(store);

        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(store.load_operation("open-atomic").unwrap(), observed);
        let base_state = store.load_session_state().unwrap();
        let mut published_state = base_state.clone();
        published_state.next_session_sequence = 41;
        let published_handoff = RemoteViewHandoff {
            id: "handoff-atomic".to_string(),
            state: "ready".to_string(),
            browser_id: Some("browser-atomic".to_string()),
            tab_id: Some("tab-atomic".to_string()),
            ..RemoteViewHandoff::default()
        };
        let mut intervening_state = base_state.clone();
        intervening_state.next_session_sequence = 7;
        store.save_session_state(&intervening_state).unwrap();
        assert_eq!(
            store.commit_browser_open(
                "open-atomic",
                reserved.generation,
                &base_state,
                &published_state,
                &published_handoff,
                ready_observation.clone(),
            ),
            Err("browser_runtime_operation_base_state_conflict".to_string())
        );
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());
        assert_eq!(
            store.load_operation("open-atomic").unwrap().state,
            BrowserRuntimeOperationState::Observed
        );
        store.save_session_state(&base_state).unwrap();
        let committed = store
            .commit_browser_open(
                "open-atomic",
                reserved.generation,
                &base_state,
                &published_state,
                &published_handoff,
                ready_observation.clone(),
            )
            .unwrap();
        assert_eq!(committed.state, BrowserRuntimeOperationState::Committed);
        assert_eq!(store.load_session_state().unwrap(), published_state);
        assert_eq!(
            store
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .get("handoff-atomic"),
            Some(&published_handoff)
        );
        assert!(store.list_pending_operations().unwrap().is_empty());

        let mut replay_state = published_state.clone();
        replay_state.next_session_sequence = 99;
        let mut replay_handoff = published_handoff.clone();
        replay_handoff.state = "must-not-republish".to_string();
        assert_eq!(
            store
                .commit_browser_open(
                    "open-atomic",
                    reserved.generation,
                    &published_state,
                    &replay_state,
                    &replay_handoff,
                    ready_observation.clone(),
                )
                .unwrap(),
            committed
        );
        assert_eq!(
            store.commit_browser_open(
                "open-atomic",
                reserved.generation,
                &published_state,
                &replay_state,
                &replay_handoff,
                serde_json::json!({"phase": "different"}),
            ),
            Err("browser_runtime_operation_replay_mismatch:open-atomic".to_string())
        );
        assert_eq!(store.load_session_state().unwrap(), published_state);
        assert_eq!(
            store
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .get("handoff-atomic"),
            Some(&published_handoff)
        );
        drop(store);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(reopened.load_operation("open-atomic").unwrap(), committed);
        assert_eq!(reopened.load_session_state().unwrap(), published_state);
        assert_eq!(
            reopened
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .get("handoff-atomic"),
            Some(&published_handoff)
        );
    }

    #[test]
    fn stale_browser_open_generation_cannot_observe_or_publish() {
        let directory = TempDirectory::new("browser-runtime-open-stale-generation");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let original_state = store.load_session_state().unwrap();
        let stale = store
            .reserve_operation("open-stale", "session:shared", serde_json::json!({}))
            .unwrap();
        let current = store
            .reserve_operation("open-current", "session:shared", serde_json::json!({}))
            .unwrap();
        assert_eq!(current.generation, stale.generation + 1);
        let observation = serde_json::json!({"phase": "browser_opened"});
        let stale_error = format!(
            "browser_runtime_operation_generation_stale:open-stale:{}:{}",
            stale.generation, current.generation
        );
        assert_eq!(
            store.record_operation_observation("open-stale", stale.generation, observation.clone()),
            Err(stale_error.clone())
        );
        let mut unpublished_state = original_state.clone();
        unpublished_state.next_session_sequence = 77;
        let unpublished_handoff = RemoteViewHandoff {
            id: "handoff-stale".to_string(),
            state: "ready".to_string(),
            ..RemoteViewHandoff::default()
        };
        assert_eq!(
            store.commit_browser_open(
                "open-stale",
                stale.generation,
                &original_state,
                &unpublished_state,
                &unpublished_handoff,
                observation,
            ),
            Err(stale_error)
        );
        assert_eq!(store.load_session_state().unwrap(), original_state);
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());
        assert_eq!(
            store.load_operation("open-stale").unwrap().state,
            BrowserRuntimeOperationState::Prepared
        );
    }

    #[test]
    fn missing_manager_handoff_document_loads_as_an_empty_registry() {
        let directory = TempDirectory::new("browser-runtime-missing-handoff-document");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        store
            .connection
            .execute(
                "DELETE FROM state_documents WHERE kind = ?1",
                params![MANAGER_HANDOFF_REGISTRY_DOCUMENT],
            )
            .unwrap();
        drop(store);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        assert_eq!(
            reopened.load_handoff_registry().unwrap(),
            BrowserManagerHandoffRegistry::default()
        );
    }

    #[test]
    fn cold_upgrade_state_reader_prefers_existing_sqlite_without_json_fallback() {
        let directory = TempDirectory::new("browser-runtime-cold-upgrade-reader");
        let database_path = directory.0.join(BROWSER_RUNTIME_DATABASE_FILENAME);
        let session_path = directory.0.join(BROWSER_SESSION_STATE_FILENAME);
        let catalog_path = directory.0.join(BROWSER_PROFILE_CATALOG_FILENAME);
        let service_path = directory.0.join("state.json");
        let mut migrated = BrowserSessionState::default();
        migrated.next_session_sequence = 7;
        write_private_json_atomic(&session_path, &migrated).unwrap();
        write_private_json_atomic(&catalog_path, &BrowserProfileCatalog::default()).unwrap();
        fs::write(&service_path, "{}").unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_path,
                profile_catalog_path: &catalog_path,
                service_state_path: &service_path,
            },
        )
        .unwrap();
        let mut stale_json = BrowserSessionState::default();
        stale_json.next_session_sequence = 99;
        write_private_json_atomic(&session_path, &stale_json).unwrap();

        assert_eq!(
            load_session_state_for_cold_upgrade(&directory.0)
                .unwrap()
                .next_session_sequence,
            7
        );
        fs::write(&database_path, "corrupt").unwrap();
        assert!(load_session_state_for_cold_upgrade(&directory.0).is_err());
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
}
