//! Independent durable storage for the ordinary Browser Session Manager path.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use agent_browser_service_model::{
    BrowserNavigationRecord, BrowserProfileCatalog, BrowserProfileCatalogDiagnostic,
    BrowserSessionState, BrowserTabEndReason, ManagedBrowserInstance, ManagedBrowserSession,
    ManagedBrowserTab, ManagedDisposableProfile, PresentationRequestQueue,
    PresentationRequestState, PresentationScaleInState, RemoteViewHandoff, RouteKeeperAuthority,
    RouteKeeperConnectionCatalog, RouteKeeperReconcileAction, SessionEndReason,
    TerminalBrowserSession, TerminalBrowserTab, BROWSER_PROFILE_CATALOG_SCHEMA_V1,
    BROWSER_SESSION_STATE_SCHEMA_V1, ROUTE_KEEPER_AUTHORITY_SCHEMA_V1,
    ROUTE_KEEPER_AUTHORITY_SCHEMA_V2, ROUTE_KEEPER_AUTHORITY_SCHEMA_V3,
    ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::service_store::default_service_state_path;

mod desktop_control;
mod manual_seeding;
mod provisioning;
pub(crate) use desktop_control::{DesktopControlLease, DesktopControlTransferRequest};
pub(crate) use manual_seeding::{ManualSeedingReservation, ManualSeedingState};
pub(crate) use provisioning::{PresentationProvisioningConfig, PresentationProvisioningOperation};

const BROWSER_SESSION_STATE_FILENAME: &str = "browser-session-state.json";
const BROWSER_PROFILE_CATALOG_FILENAME: &str = "browser-profile-catalog.json";
const BROWSER_RUNTIME_DATABASE_SCHEMA: i64 = 1;
const BROWSER_RUNTIME_DATABASE_FILENAME: &str = "runtime.sqlite3";
const SESSION_STATE_DOCUMENT: &str = "browser_session_state";
const PROFILE_CATALOG_DOCUMENT: &str = "browser_profile_catalog";
const MANAGER_HANDOFF_REGISTRY_DOCUMENT: &str = "manager_handoff_registry";
const PRESENTATION_REQUEST_QUEUE_DOCUMENT: &str = "presentation_request_queue";
const PRESENTATION_SCALE_IN_STATE_DOCUMENT: &str = "presentation_scale_in_state";
const ROUTE_KEEPER_AUTHORITY_DOCUMENT: &str = "route_keeper_authority";
const MANAGER_HANDOFF_REGISTRY_SCHEMA_V1: &str = "agent-browser.manager-handoffs.v1";
const PRESENTATION_REQUEST_QUEUE_SCHEMA_V1: &str = "agent-browser.presentation-request-queue.v1";
const PRESENTATION_SCALE_IN_STATE_SCHEMA_V1: &str = "agent-browser.presentation-scale-in-state.v1";
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserRuntimeConfigPatch {
    pub(crate) minimum_ready: Option<u32>,
    pub(crate) warm_target: Option<u32>,
    pub(crate) maximum_displays: Option<u32>,
    pub(crate) maximum_browsers_per_display: Option<u32>,
    pub(crate) maximum_queue_depth: Option<u32>,
    pub(crate) request_deadline_ms: Option<u64>,
    pub(crate) scale_in_cooldown_ms: Option<u64>,
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

struct BrowserPublication<'a> {
    expected_owner_key: Option<&'static str>,
    expected_observation: Option<&'a serde_json::Value>,
    expected_base_state: &'a BrowserSessionState,
    session_state: &'a BrowserSessionState,
    handoff: &'a RemoteViewHandoff,
    result: serde_json::Value,
}

pub(crate) struct LegacyBrowserRuntimeSources<'a> {
    pub(crate) session_state_path: &'a Path,
    pub(crate) profile_catalog_path: &'a Path,
    pub(crate) service_state_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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

fn catalog_publication_authority(
    current: &RouteKeeperAuthority,
    catalog: RouteKeeperConnectionCatalog,
) -> Result<RouteKeeperAuthority, String> {
    let maximum_slots = u32::try_from(catalog.bindings.len())
        .map_err(|_| "route_keeper_connection_catalog_slots_incomplete".to_string())?;
    if maximum_slots < current.policy.maximum_slots
        || (1..=maximum_slots).any(|sequence| {
            !catalog
                .bindings
                .contains_key(&format!("route-slot-{sequence:02}"))
        })
    {
        return Err("route_keeper_connection_catalog_slots_incomplete".to_string());
    }
    if current.connection_catalog == catalog {
        return Ok(current.clone());
    }
    let mut next = current.clone();
    if maximum_slots > current.policy.maximum_slots {
        next.extend_connection_catalog(catalog, maximum_slots)?;
    } else {
        next.replace_connection_catalog(catalog)?;
    }
    Ok(next)
}

impl BrowserRuntimeSqliteStore {
    pub(crate) fn default_sqlite_path() -> Result<PathBuf, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Ok(service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME))
    }

    pub(crate) fn default_sqlite() -> Result<Self, String> {
        Self::open(&Self::default_sqlite_path()?)
    }

    pub(crate) fn migrate_default_from_legacy() -> Result<BrowserRuntimeMigrationReceipt, String> {
        let service_state_path = default_service_state_path()?;
        let service_directory = service_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Self::migrate_from_legacy(
            &service_directory.join(BROWSER_RUNTIME_DATABASE_FILENAME),
            LegacyBrowserRuntimeSources {
                session_state_path: &service_directory.join(BROWSER_SESSION_STATE_FILENAME),
                profile_catalog_path: &service_directory.join(BROWSER_PROFILE_CATALOG_FILENAME),
                service_state_path: &service_state_path,
            },
        )
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
        let session_state = import_legacy_session_state(
            sources.session_state_path,
            &session_raw,
            session_source.is_some(),
            &mut session_imported,
            &mut rejections,
        );

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
                ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
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

    /// Commit a manager state change with handoffs explicitly closed by their
    /// session or tab, or expired with their session. A browser loss is left
    /// available for recovery. Readers see both documents change together.
    pub(crate) fn publish_session_state_and_terminal_handoffs(
        &mut self,
        state: &BrowserSessionState,
    ) -> Result<Vec<RemoteViewHandoff>, String> {
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_session_commit_begin_failed:{error}"))?;
        let mut registry: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        let mut terminal = Vec::new();
        let observed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        for handoff in registry.handoffs.values_mut() {
            if handoff.state != "ready"
                || !super::browser_session_handoff::is_manager_handoff(handoff)
            {
                continue;
            }
            let session_id = handoff
                .intent
                .get("sessionId")
                .and_then(serde_json::Value::as_str);
            let session_closed = session_id.is_some_and(|session_id| {
                !state.sessions.contains_key(session_id)
                    && state.session_history.iter().any(|ended| {
                        ended.id == session_id
                            && matches!(
                                ended.reason,
                                SessionEndReason::ExplicitClose
                                    | SessionEndReason::HeartbeatExpired
                            )
                    })
            });
            let tab_closed =
                session_id
                    .zip(handoff.tab_id.as_deref())
                    .is_some_and(|(session_id, tab_id)| {
                        !state.tabs.contains_key(tab_id)
                            && state.tab_history.iter().any(|ended| {
                                ended.id == tab_id
                                    && ended.session_id == session_id
                                    && handoff.browser_id.as_deref()
                                        == Some(ended.browser_id.as_str())
                                    && handoff.target_id.as_deref()
                                        == Some(ended.target_id.as_str())
                                    && ended.reason == BrowserTabEndReason::ExplicitClose
                            })
                    });
            if !session_closed && !tab_closed {
                continue;
            }
            handoff.state = "closed".to_string();
            handoff.updated_at = Some(observed_at.clone());
            handoff.last_resolution = Some(serde_json::json!({
                "status": "closed",
                "reason": "manager_session_or_tab_ended",
            }));
            if let Some(receipt) = handoff.presentation_receipt.as_mut() {
                receipt.state = "closed".to_string();
            }
            terminal.push(handoff.clone());
        }
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )?;
        if !terminal.is_empty() {
            save_document(
                &transaction,
                MANAGER_HANDOFF_REGISTRY_DOCUMENT,
                MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
                &registry,
            )?;
        }
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_session_commit_failed:{error}"))?;
        Ok(terminal)
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

    pub(crate) fn load_presentation_queue(&self) -> Result<PresentationRequestQueue, String> {
        let queue: PresentationRequestQueue = load_optional_document(
            &self.connection,
            PRESENTATION_REQUEST_QUEUE_DOCUMENT,
            PRESENTATION_REQUEST_QUEUE_SCHEMA_V1,
        )?;
        queue.validate()?;
        Ok(queue)
    }

    pub(crate) fn mutate_presentation_queue<T>(
        &mut self,
        mutate: impl FnOnce(&mut PresentationRequestQueue) -> Result<T, String>,
    ) -> Result<T, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("presentation_request_queue_begin_failed:{error}"))?;
        let mut queue: PresentationRequestQueue = load_optional_document(
            &transaction,
            PRESENTATION_REQUEST_QUEUE_DOCUMENT,
            PRESENTATION_REQUEST_QUEUE_SCHEMA_V1,
        )?;
        queue.validate()?;
        let result = mutate(&mut queue)?;
        queue.validate()?;
        save_document(
            &transaction,
            PRESENTATION_REQUEST_QUEUE_DOCUMENT,
            PRESENTATION_REQUEST_QUEUE_SCHEMA_V1,
            &queue,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("presentation_request_queue_commit_failed:{error}"))?;
        Ok(result)
    }

    pub(crate) fn save_manager_handoff(
        &mut self,
        handoff: &RemoteViewHandoff,
    ) -> Result<(), String> {
        if handoff.id.is_empty() {
            return Err("browser_runtime_handoff_identity_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_handoff_begin_failed:{error}"))?;
        let mut registry: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        registry
            .handoffs
            .insert(handoff.id.clone(), handoff.clone());
        save_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_handoff_commit_failed:{error}"))
    }

    /// Atomically publish manager-owned session membership and its handoff.
    pub(crate) fn publish_manager_handoff(
        &mut self,
        state: &BrowserSessionState,
        handoff: &RemoteViewHandoff,
    ) -> Result<(), String> {
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        if handoff.id.is_empty() {
            return Err("browser_runtime_handoff_identity_invalid".to_string());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_handoff_begin_failed:{error}"))?;
        let mut registry: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        registry
            .handoffs
            .insert(handoff.id.clone(), handoff.clone());
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            state,
        )?;
        save_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
            &registry,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_handoff_commit_failed:{error}"))
    }

    pub(crate) fn load_route_keeper_authority(&self) -> Result<RouteKeeperAuthority, String> {
        load_route_keeper_authority_document(&self.connection)
    }

    /// Publish a complete installed catalog, preserving active evidence for
    /// append-only growth. Logical capacity settings remain independently owned.
    #[cfg(test)]
    pub(crate) fn publish_route_keeper_connection_catalog(
        &mut self,
        catalog: RouteKeeperConnectionCatalog,
    ) -> Result<RouteKeeperConnectionCatalogPublication, String> {
        self.publish_route_keeper_configuration(catalog, None)
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
        let effect_transition_preserves_demand =
            next.requested_ready_slots == expected.requested_ready_slots;
        let effect_transition_preserves_policy = next.policy == expected.policy;
        let mut current_normalized = current.clone();
        if effect_transition_preserves_demand {
            current_normalized.requested_ready_slots = expected.requested_ready_slots;
        }
        if effect_transition_preserves_policy {
            current_normalized.policy = expected.policy.clone();
        }
        let mut persisted = next.clone();
        if current.connection_catalog != expected.connection_catalog
            && next.connection_catalog == expected.connection_catalog
            && next.retained_connection_catalogs == expected.retained_connection_catalogs
            && next.host_process_claims == expected.host_process_claims
            && next.policy.maximum_slots == expected.policy.maximum_slots
            && next.records.keys().eq(expected.records.keys())
        {
            // An independently published append must not discard an in-flight
            // old-route receipt or force supervisor shutdown. Prove the exact
            // extension, including every unchanged retained record, first.
            current_normalized.policy.maximum_slots = current.policy.maximum_slots;
            let mut extended_expected = expected.clone();
            extended_expected
                .extend_connection_catalog(
                    current.connection_catalog.clone(),
                    current.policy.maximum_slots,
                )
                .map_err(|_| "route_keeper_authority_compare_and_swap_conflict".to_string())?;
            // More than one independently validated append may have completed.
            // Preserve those intermediate snapshots without losing any catalog
            // already required by the expected state.
            if extended_expected
                .retained_connection_catalogs
                .iter()
                .any(|(digest, catalog)| {
                    current.retained_connection_catalogs.get(digest) != Some(catalog)
                })
            {
                return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
            }
            extended_expected.retained_connection_catalogs =
                current.retained_connection_catalogs.clone();
            for (slot_id, record) in &current.records {
                if !expected.records.contains_key(slot_id) {
                    current
                        .connection_binding_for_fence(slot_id, &record.fence)
                        .map_err(|_| {
                            "route_keeper_authority_compare_and_swap_conflict".to_string()
                        })?;
                    let expected_record =
                        extended_expected.records.get(slot_id).ok_or_else(|| {
                            "route_keeper_authority_compare_and_swap_conflict".to_string()
                        })?;
                    let mut normalized_record = record.clone();
                    // A slot appended in an intermediate catalog retains that
                    // validated digest. Every other pristine-slot field must
                    // still match the deterministic extension exactly.
                    normalized_record.fence.connection_catalog_digest =
                        expected_record.fence.connection_catalog_digest.clone();
                    if normalized_record != *expected_record {
                        return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
                    }
                    extended_expected
                        .records
                        .insert(slot_id.clone(), record.clone());
                }
            }
            if current_normalized != extended_expected {
                return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
            }
            persisted.connection_catalog = current.connection_catalog.clone();
            persisted.retained_connection_catalogs = current.retained_connection_catalogs.clone();
            persisted.policy.maximum_slots = current.policy.maximum_slots;
            for (slot_id, record) in &current.records {
                if !expected.records.contains_key(slot_id) {
                    persisted.records.insert(slot_id.clone(), record.clone());
                }
            }
        } else if current_normalized != *expected {
            return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
        }
        if current.connection_catalog != persisted.connection_catalog
            && current
                .records
                .values()
                .any(|record| record.phase != agent_browser_service_model::RouteKeeperPhase::Absent)
        {
            // Active publication is only an exact additive transition. It must
            // not smuggle changes to retained records or policy intent.
            let mut extension = current.clone();
            extension
                .extend_connection_catalog(
                    persisted.connection_catalog.clone(),
                    persisted.policy.maximum_slots,
                )
                .map_err(|_| "route_keeper_connection_catalog_active".to_string())?;
            if extension != persisted {
                return Err("route_keeper_connection_catalog_active".to_string());
            }
        }
        for (generation, claim) in &current.host_process_claims {
            if persisted.host_process_claims.get(generation) != Some(claim) {
                return Err(format!(
                    "route_keeper_host_process_claim_changed:{generation}"
                ));
            }
        }
        for (slot_id, current_record) in &current.records {
            let next_record = persisted
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
        if effect_transition_preserves_demand {
            persisted.requested_ready_slots = current.requested_ready_slots;
        }
        if effect_transition_preserves_policy {
            persisted.policy = current.policy.clone();
        }
        // A waiter may have read the old cap before a configuration update.
        // Fence its new demand against the current settings in this transaction.
        if !effect_transition_preserves_demand {
            persisted.requested_ready_slots = persisted
                .requested_ready_slots
                .min(load_runtime_config_row(&transaction)?.maximum_displays);
        }
        // Revalidate the merged intent against any policy change in this transition.
        persisted.projection()?;
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
            &persisted,
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

    pub(crate) fn update_runtime_config(
        &mut self,
        patch: BrowserRuntimeConfigPatch,
    ) -> Result<BrowserRuntimeConfig, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_config_begin_failed:{error}"))?;
        let current = load_runtime_config_row(&transaction)?;
        let authority = load_route_keeper_authority_document(&transaction)?;
        let mut next = current.clone();
        if let Some(value) = patch.minimum_ready {
            next.minimum_ready = value;
        }
        if let Some(value) = patch.warm_target {
            next.warm_target = value;
        }
        if let Some(value) = patch.maximum_displays {
            next.maximum_displays = value;
        }
        if let Some(value) = patch.maximum_browsers_per_display {
            next.maximum_browsers_per_display = value;
        }
        if let Some(value) = patch.maximum_queue_depth {
            next.maximum_queue_depth = value;
        }
        if let Some(value) = patch.request_deadline_ms {
            next.request_deadline_ms = value;
        }
        if let Some(value) = patch.scale_in_cooldown_ms {
            next.scale_in_cooldown_ms = value;
        }
        provisioning::authorize_growth_config(
            &transaction,
            next.maximum_displays,
            authority.policy.maximum_slots,
            patch.maximum_displays.is_some(),
        )?;
        validate_runtime_config(&next)?;
        let mut next_authority = authority.clone();
        next_authority.policy.minimum_ready =
            next.minimum_ready.min(authority.policy.maximum_slots);
        next_authority.policy.warm_target = next.warm_target.min(authority.policy.maximum_slots);
        next_authority.requested_ready_slots = next_authority
            .requested_ready_slots
            .min(next.maximum_displays);
        next_authority.projection()?;

        let config_changed = next != current;
        let authority_changed = next_authority != authority;
        if !config_changed && !authority_changed {
            transaction
                .commit()
                .map_err(|_| "browser_runtime_config_commit_failed".to_string())?;
            return Ok(current);
        }
        if config_changed {
            next.revision = current
                .revision
                .checked_add(1)
                .ok_or_else(|| "browser_runtime_config_revision_exhausted".to_string())?;
        }
        save_runtime_config_row(&transaction, &next)?;
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
            &next_authority,
        )?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_config_commit_failed:{error}"))?;
        Ok(next)
    }

    pub(crate) fn reserve_idle_route_stop(
        &mut self,
        now_ms: u64,
    ) -> Result<Option<(RouteKeeperAuthority, RouteKeeperReconcileAction)>, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("presentation_scale_in_begin_failed:{error}"))?;
        let config = load_runtime_config_row(&transaction)?;
        let authority = load_route_keeper_authority_document(&transaction)?;
        let session_state: BrowserSessionState = load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        let handoffs: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        let queue: PresentationRequestQueue = load_optional_document(
            &transaction,
            PRESENTATION_REQUEST_QUEUE_DOCUMENT,
            PRESENTATION_REQUEST_QUEUE_SCHEMA_V1,
        )?;
        queue.validate()?;
        let pending_operations: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM operation_records WHERE state != 'committed'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("browser_runtime_operation_read_failed:{error}"))?;

        let mut referenced_slots = BTreeSet::new();
        let mut admission_pending = pending_operations != 0;
        for browser in session_state.browsers.values() {
            let Some(desktop) = &browser.desktop else {
                admission_pending = true;
                continue;
            };
            if desktop.route_id.is_empty() {
                admission_pending = true;
            } else {
                referenced_slots.insert(desktop.route_id.clone());
            }
        }
        if session_state
            .sessions
            .values()
            .any(|session| !session_state.browsers.contains_key(&session.browser_id))
            || session_state
                .tabs
                .values()
                .any(|tab| !session_state.browsers.contains_key(&tab.browser_id))
        {
            admission_pending = true;
        }
        for handoff in handoffs.handoffs.values() {
            let mut has_route_reference = false;
            for route_id in [&handoff.last_route_id, &handoff.last_route_pool_entry_id]
                .into_iter()
                .flatten()
            {
                if route_id.is_empty() {
                    admission_pending = true;
                } else {
                    referenced_slots.insert(route_id.clone());
                    has_route_reference = true;
                }
            }
            if !has_route_reference {
                admission_pending = true;
            }
        }
        if referenced_slots
            .iter()
            .any(|slot_id| !authority.records.contains_key(slot_id))
        {
            admission_pending = true;
        }
        if authority.records.values().any(|record| {
            !matches!(
                record.phase,
                agent_browser_service_model::RouteKeeperPhase::Absent
                    | agent_browser_service_model::RouteKeeperPhase::Ready
            )
        }) {
            admission_pending = true;
        }
        if queue.entries.values().any(|entry| {
            matches!(&entry.state, PresentationRequestState::Admitted { .. })
                || matches!(
                    &entry.state,
                    PresentationRequestState::Queued if entry.deadline_ms > now_ms
                )
                || matches!(
                    &entry.state,
                    PresentationRequestState::Retryable {
                        recovery_required: true
                    }
                )
        }) {
            admission_pending = true;
        }

        let mut scale_in: PresentationScaleInState = load_optional_document(
            &transaction,
            PRESENTATION_SCALE_IN_STATE_DOCUMENT,
            PRESENTATION_SCALE_IN_STATE_SCHEMA_V1,
        )?;
        let selected = scale_in.observe_and_select(
            &authority,
            &referenced_slots,
            admission_pending,
            now_ms,
            config.scale_in_cooldown_ms,
        )?;
        save_document(
            &transaction,
            PRESENTATION_SCALE_IN_STATE_DOCUMENT,
            PRESENTATION_SCALE_IN_STATE_SCHEMA_V1,
            &scale_in,
        )?;
        let result = if let Some(slot_id) = selected {
            let fence = authority
                .records
                .get(&slot_id)
                .ok_or_else(|| "presentation_scale_in_slot_missing".to_string())?
                .fence
                .clone();
            let mut stopping = authority.clone();
            let reference_target = u32::try_from(referenced_slots.len())
                .unwrap_or(u32::MAX)
                .max(stopping.policy.warm_target);
            stopping.requested_ready_slots = stopping.requested_ready_slots.min(reference_target);
            let action = stopping.begin_stop(&slot_id, &fence)?;
            stopping.projection()?;
            save_document(
                &transaction,
                ROUTE_KEEPER_AUTHORITY_DOCUMENT,
                ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
                &stopping,
            )?;
            Some((stopping, action))
        } else {
            None
        };
        transaction
            .commit()
            .map_err(|error| format!("presentation_scale_in_commit_failed:{error}"))?;
        Ok(result)
    }

    pub(crate) fn reserve_operation(
        &mut self,
        operation_id: &str,
        owner_key: &str,
        request: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_operation_begin_failed:{error}"))?;
        let operation =
            reserve_operation_in_transaction(&transaction, operation_id, owner_key, request)?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_operation_commit_failed:{error}"))?;
        Ok(operation)
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
        let operation =
            observe_operation_in_transaction(&transaction, operation_id, generation, observation)?;
        transaction
            .commit()
            .map_err(|error| format!("browser_runtime_operation_commit_failed:{error}"))?;
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
        self.commit_browser_publication(
            operation_id,
            generation,
            BrowserPublication {
                expected_owner_key: None,
                expected_observation: None,
                expected_base_state,
                session_state,
                handoff,
                result,
            },
        )
    }

    fn commit_browser_publication(
        &mut self,
        operation_id: &str,
        generation: u64,
        publication: BrowserPublication<'_>,
    ) -> Result<BrowserRuntimeOperation, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("browser_runtime_operation_begin_failed:{error}"))?;
        let mut operation = load_optional_operation(&transaction, operation_id)?
            .ok_or_else(|| format!("browser_runtime_operation_missing:{operation_id}"))?;
        if publication
            .expected_owner_key
            .is_some_and(|owner_key| operation.owner_key != owner_key)
        {
            return Err(format!(
                "browser_runtime_operation_owner_mismatch:{operation_id}"
            ));
        }
        if operation.generation != generation {
            return Err(format!(
                "browser_runtime_operation_generation_mismatch:{operation_id}:{generation}:{}",
                operation.generation
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Committed {
            if operation.result.as_ref() == Some(&publication.result) {
                return Ok(operation);
            }
            return Err(format!(
                "browser_runtime_operation_replay_mismatch:{operation_id}"
            ));
        }
        if publication.expected_observation.is_some()
            && operation.state != BrowserRuntimeOperationState::Observed
        {
            return Err(format!(
                "browser_runtime_operation_observation_missing:{operation_id}"
            ));
        }
        if operation.state == BrowserRuntimeOperationState::Observed
            && operation.result.as_ref()
                != publication
                    .expected_observation
                    .or(Some(&publication.result))
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
        if publication.session_state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                publication.session_state.schema_version
            ));
        }
        if publication.handoff.id.is_empty() {
            return Err("browser_runtime_handoff_identity_invalid".to_string());
        }
        let current_session_state: BrowserSessionState = load_optional_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if current_session_state != *publication.expected_base_state {
            return Err("browser_runtime_operation_base_state_conflict".to_string());
        }
        let mut registry: BrowserManagerHandoffRegistry = load_optional_document(
            &transaction,
            MANAGER_HANDOFF_REGISTRY_DOCUMENT,
            MANAGER_HANDOFF_REGISTRY_SCHEMA_V1,
        )?;
        registry
            .handoffs
            .insert(publication.handoff.id.clone(), publication.handoff.clone());
        let result_json = serde_json::to_string(&publication.result)
            .map_err(|error| format!("browser_runtime_operation_serialize_failed:{error}"))?;
        save_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            publication.session_state,
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
        operation.result = Some(publication.result);
        Ok(operation)
    }

    /// Atomically publishes navigation state, its refreshed handoff, and the journal result.
    /// Navigation has the same compare-and-swap publication contract as browser open.
    pub(crate) fn commit_browser_navigation(
        &mut self,
        operation: &BrowserRuntimeOperation,
        expected_base_state: &BrowserSessionState,
        session_state: &BrowserSessionState,
        handoff: &RemoteViewHandoff,
        result: serde_json::Value,
    ) -> Result<BrowserRuntimeOperation, String> {
        let expected_observation = operation.result.as_ref().ok_or_else(|| {
            format!(
                "browser_runtime_operation_observation_missing:{}",
                operation.operation_id
            )
        })?;
        self.commit_browser_publication(
            &operation.operation_id,
            operation.generation,
            BrowserPublication {
                expected_owner_key: Some("browser-runtime-navigation"),
                expected_observation: Some(expected_observation),
                expected_base_state,
                session_state,
                handoff,
                result,
            },
        )
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
        if schema == ROUTE_KEEPER_AUTHORITY_SCHEMA_V1
            || schema == ROUTE_KEEPER_AUTHORITY_SCHEMA_V2
            || schema == ROUTE_KEEPER_AUTHORITY_SCHEMA_V3
        {
            authority = if schema == ROUTE_KEEPER_AUTHORITY_SCHEMA_V1 {
                authority.upgrade_from_v1()?
            } else if schema == ROUTE_KEEPER_AUTHORITY_SCHEMA_V2 {
                authority.upgrade_from_v2()?
            } else {
                authority.upgrade_from_v3()?
            };
            if compare_and_swap_route_keeper_schema_upgrade(connection, &schema, &json, &authority)?
            {
                authority.projection()?;
                return Ok(authority);
            }
            continue;
        }
        if schema != ROUTE_KEEPER_AUTHORITY_SCHEMA_V4 {
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

fn compare_and_swap_route_keeper_schema_upgrade(
    connection: &Connection,
    source_schema: &str,
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
                ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
                upgraded_json,
                ROUTE_KEEPER_AUTHORITY_DOCUMENT,
                source_schema,
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

/// Reserve an operation inside the caller's SQLite transaction so another
/// authority record may be committed with the same generation and request.
fn reserve_operation_in_transaction(
    connection: &Connection,
    operation_id: &str,
    owner_key: &str,
    request: serde_json::Value,
) -> Result<BrowserRuntimeOperation, String> {
    if operation_id.is_empty() || owner_key.is_empty() {
        return Err("browser_runtime_operation_identity_invalid".to_string());
    }
    if let Some(existing) = load_optional_operation(connection, operation_id)? {
        if existing.owner_key == owner_key && existing.request == request {
            return Ok(existing);
        }
        return Err(format!(
            "browser_runtime_operation_replay_mismatch:{operation_id}"
        ));
    }
    if owner_key == "browser-runtime-open" {
        if let Some(profile_id) = request
            .pointer("/intent/browser/profileId")
            .and_then(serde_json::Value::as_str)
        {
            if manual_seeding::profile_reserved_by_manual_seeding(connection, profile_id)? {
                return Err(format!(
                    "browser_runtime_open_profile_reserved_by_manual_seeding:{profile_id}"
                ));
            }
        }
        if let Some(slot_id) = request
            .pointer("/intent/slot/routeId")
            .and_then(serde_json::Value::as_str)
        {
            if manual_seeding::route_slot_reserved_by_manual_seeding(connection, slot_id)? {
                return Err(format!(
                    "browser_runtime_open_route_reserved_by_manual_seeding:{slot_id}"
                ));
            }
        }
    }
    let current_generation = load_owner_generation(connection, owner_key)?;
    let generation = current_generation
        .checked_add(1)
        .ok_or_else(|| "browser_runtime_operation_generation_exhausted".to_string())?;
    let generation_sql = i64::try_from(generation)
        .map_err(|_| "browser_runtime_operation_generation_exhausted".to_string())?;
    connection
        .execute(
            "INSERT INTO operation_generations(owner_key, generation) VALUES (?1, ?2)
             ON CONFLICT(owner_key) DO UPDATE SET generation=excluded.generation",
            params![owner_key, generation_sql],
        )
        .map_err(|error| format!("browser_runtime_operation_generation_save_failed:{error}"))?;
    let request_json = serde_json::to_string(&request)
        .map_err(|error| format!("browser_runtime_operation_serialize_failed:{error}"))?;
    connection
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
    Ok(BrowserRuntimeOperation {
        operation_id: operation_id.to_string(),
        owner_key: owner_key.to_string(),
        generation,
        state: BrowserRuntimeOperationState::Prepared,
        request,
        result: None,
    })
}

fn observe_operation_in_transaction(
    connection: &Connection,
    operation_id: &str,
    generation: u64,
    observation: serde_json::Value,
) -> Result<BrowserRuntimeOperation, String> {
    let mut operation = load_optional_operation(connection, operation_id)?
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
    let current_generation = load_owner_generation(connection, &operation.owner_key)?;
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
    connection
        .execute(
            "UPDATE operation_records SET state = ?2, result_json = ?3 WHERE operation_id = ?1",
            params![
                operation_id,
                BrowserRuntimeOperationState::Observed.as_str(),
                observation_json
            ],
        )
        .map_err(|error| format!("browser_runtime_operation_save_failed:{error}"))?;
    operation.state = BrowserRuntimeOperationState::Observed;
    operation.result = Some(observation);
    Ok(operation)
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

/// Import valid active and history records independently. The unchanged source
/// is archived with typed reject records; rejected history never becomes a
/// live admission or handoff-close signal.
fn import_legacy_session_state(
    source_path: &Path,
    raw: &[u8],
    source_present: bool,
    imported: &mut bool,
    rejections: &mut Vec<BrowserRuntimeMigrationRejection>,
) -> BrowserSessionState {
    let mut document: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(document) => document,
        Err(error) => {
            if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code: "legacy_session_invalid",
                    detail: error.to_string(),
                });
            }
            return BrowserSessionState::default();
        }
    };
    let Some(object) = document.as_object_mut() else {
        if source_present {
            rejections.push(BrowserRuntimeMigrationRejection {
                source_path: source_path.to_path_buf(),
                code: "legacy_session_invalid",
                detail: "session document is not an object".to_string(),
            });
        }
        return BrowserSessionState::default();
    };
    for (key, code) in [
        ("browsers", "legacy_browser_entry_invalid"),
        ("sessions", "legacy_active_session_entry_invalid"),
        ("tabs", "legacy_active_tab_entry_invalid"),
        (
            "disposableProfiles",
            "legacy_disposable_profile_entry_invalid",
        ),
    ] {
        let Some(rows) = object.get_mut(key) else {
            continue;
        };
        let Some(entries) = rows.as_object() else {
            if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code,
                    detail: format!("{key} is not an object"),
                });
            }
            *rows = serde_json::json!({});
            continue;
        };
        let mut retained = serde_json::Map::new();
        for (id, entry) in entries {
            let valid = match key {
                "browsers" => serde_json::from_value::<ManagedBrowserInstance>(entry.clone())
                    .is_ok_and(|browser| browser.id == *id && !browser.profile_id.is_empty()),
                "sessions" => serde_json::from_value::<ManagedBrowserSession>(entry.clone())
                    .is_ok_and(|session| {
                        session.id == *id
                            && !session.name.is_empty()
                            && !session.profile_id.is_empty()
                            && !session.browser_id.is_empty()
                    }),
                "tabs" => {
                    serde_json::from_value::<ManagedBrowserTab>(entry.clone()).is_ok_and(|tab| {
                        tab.id == *id
                            && !tab.target_id.is_empty()
                            && !tab.browser_id.is_empty()
                            && !tab.session_id.is_empty()
                    })
                }
                _ => serde_json::from_value::<ManagedDisposableProfile>(entry.clone())
                    .is_ok_and(|profile| profile.profile.id == *id),
            };
            if valid {
                retained.insert(id.clone(), entry.clone());
            } else if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code,
                    detail: format!("{key}[{id}] has an invalid record or key mismatch"),
                });
            }
        }
        *rows = serde_json::Value::Object(retained);
    }
    for (key, code) in [
        ("sessionHistory", "legacy_session_history_entry_invalid"),
        ("tabHistory", "legacy_tab_history_entry_invalid"),
        (
            "navigationHistory",
            "legacy_navigation_history_entry_invalid",
        ),
    ] {
        let Some(history) = object.get_mut(key) else {
            continue;
        };
        let Some(entries) = history.as_array() else {
            if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code,
                    detail: format!("{key} is not an array"),
                });
            }
            *history = serde_json::json!([]);
            continue;
        };
        let mut retained = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let valid = match key {
                "sessionHistory" => {
                    serde_json::from_value::<TerminalBrowserSession>(entry.clone()).is_ok()
                }
                "tabHistory" => serde_json::from_value::<TerminalBrowserTab>(entry.clone()).is_ok(),
                _ => serde_json::from_value::<BrowserNavigationRecord>(entry.clone()).is_ok(),
            };
            if valid {
                retained.push(entry.clone());
            } else if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code,
                    detail: format!("{key}[{index}] has an invalid record"),
                });
            }
        }
        *history = serde_json::Value::Array(retained);
    }
    let mut state: BrowserSessionState = match serde_json::from_value(document) {
        Ok(state) => state,
        Err(error) => {
            if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code: "legacy_session_invalid",
                    detail: error.to_string(),
                });
            }
            return BrowserSessionState::default();
        }
    };
    if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
        if source_present {
            rejections.push(BrowserRuntimeMigrationRejection {
                source_path: source_path.to_path_buf(),
                code: "legacy_session_schema_unsupported",
                detail: state.schema_version,
            });
        }
        return BrowserSessionState::default();
    }
    state.sessions.retain(|id, session| {
        let valid = state
            .browsers
            .get(&session.browser_id)
            .is_some_and(|browser| {
                browser.profile_id == session.profile_id && browser.id == session.browser_id
            });
        if !valid && source_present {
            rejections.push(BrowserRuntimeMigrationRejection {
                source_path: source_path.to_path_buf(),
                code: "legacy_active_session_browser_conflict",
                detail: format!("session {id} has no matching browser and profile"),
            });
        }
        valid
    });
    state.tabs.retain(|id, tab| {
        let valid = state.sessions.get(&tab.session_id).is_some_and(|session| {
            session.browser_id == tab.browser_id && state.browsers.contains_key(&tab.browser_id)
        });
        if !valid && source_present {
            rejections.push(BrowserRuntimeMigrationRejection {
                source_path: source_path.to_path_buf(),
                code: "legacy_active_tab_session_conflict",
                detail: format!("tab {id} has no matching active session and browser"),
            });
        }
        valid
    });
    for session in state.sessions.values_mut() {
        if session.current_tab_id.as_deref().is_some_and(|tab_id| {
            state
                .tabs
                .get(tab_id)
                .is_none_or(|tab| tab.session_id != session.id)
        }) {
            if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code: "legacy_active_session_tab_conflict",
                    detail: format!("session {} has no matching current tab", session.id),
                });
            }
            session.current_tab_id = None;
        }
    }
    for browser in state.browsers.values_mut() {
        let expected = state
            .sessions
            .values()
            .filter(|session| session.browser_id == browser.id)
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        let actual_ids = browser.active_session_ids.iter().collect::<BTreeSet<_>>();
        let expected_ids = expected.iter().collect::<BTreeSet<_>>();
        if actual_ids != expected_ids || browser.active_session_ids.len() != expected.len() {
            if source_present {
                rejections.push(BrowserRuntimeMigrationRejection {
                    source_path: source_path.to_path_buf(),
                    code: "legacy_browser_session_membership_repaired",
                    detail: format!("browser {} active session membership changed", browser.id),
                });
            }
            browser.active_session_ids = expected;
        }
    }
    let mut session_counts = BTreeMap::<String, usize>::new();
    for ended in &state.session_history {
        *session_counts.entry(ended.id.clone()).or_default() += 1;
    }
    state.session_history.retain(|ended| {
        let valid =
            !state.sessions.contains_key(&ended.id) && session_counts.get(&ended.id) == Some(&1);
        if !valid && source_present {
            rejections.push(BrowserRuntimeMigrationRejection {
                source_path: source_path.to_path_buf(),
                code: "legacy_session_history_conflict",
                detail: format!(
                    "history identity {} conflicts with active or repeated state",
                    ended.id
                ),
            });
        }
        valid
    });
    let mut tab_counts = BTreeMap::<String, usize>::new();
    for ended in &state.tab_history {
        *tab_counts.entry(ended.id.clone()).or_default() += 1;
    }
    state.tab_history.retain(|ended| {
        let valid = !state.tabs.contains_key(&ended.id) && tab_counts.get(&ended.id) == Some(&1);
        if !valid && source_present {
            rejections.push(BrowserRuntimeMigrationRejection {
                source_path: source_path.to_path_buf(),
                code: "legacy_tab_history_conflict",
                detail: format!(
                    "history identity {} conflicts with active or repeated state",
                    ended.id
                ),
            });
        }
        valid
    });
    *imported = source_present;
    state
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
    use super::manual_seeding::{ManualSeedingReservation, ManualSeedingState};
    use super::*;
    use agent_browser_service_model::{
        BrowserProfileKind, PresentationRequestPriority, RecordedProcessIdentity,
        RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog, RouteKeeperHostProcessClaim,
    };

    #[test]
    fn legacy_history_rejects_bad_rows_without_dropping_active_session() {
        let mut state = BrowserSessionState::default();
        state.browsers.insert(
            "browser-a".to_string(),
            ManagedBrowserInstance {
                id: "browser-a".to_string(),
                profile_id: "work".to_string(),
                pid: 4_242,
                cdp_endpoint: "http://127.0.0.1:4242".to_string(),
                process_identity: None,
                desktop: None,
                active_session_ids: vec!["session-a".to_string()],
            },
        );
        state.sessions.insert(
            "session-a".to_string(),
            agent_browser_service_model::ManagedBrowserSession {
                id: "session-a".to_string(),
                name: "alice".to_string(),
                profile_id: "work".to_string(),
                browser_id: "browser-a".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: u64::MAX,
                current_tab_id: None,
                handoff_ids: Vec::new(),
            },
        );
        state.session_history.push(TerminalBrowserSession {
            id: "session-a".to_string(),
            name: "alice".to_string(),
            profile_id: "work".to_string(),
            browser_id: "browser-a".to_string(),
            created_at_ms: 1,
            last_activity_at_ms: 2,
            ended_at_ms: 3,
            reason: SessionEndReason::ExplicitClose,
        });
        state.navigation_history.push(BrowserNavigationRecord {
            profile_id: "work".to_string(),
            session_id: "session-a".to_string(),
            browser_id: "browser-a".to_string(),
            tab_id: "tab-a".to_string(),
            target_id: "target-a".to_string(),
            url: "https://example.test/".to_string(),
            visited_at_ms: 4,
        });
        let mut source = serde_json::to_value(state).unwrap();
        source["sessionHistory"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"bad"}));
        source["tabHistory"] = serde_json::json!("malformed");
        source["navigationHistory"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"url":2}));
        let mut imported = false;
        let mut rejections = Vec::new();
        let restored = import_legacy_session_state(
            Path::new("/tmp/legacy-browser-session-state.json"),
            serde_json::to_vec(&source).unwrap().as_slice(),
            true,
            &mut imported,
            &mut rejections,
        );
        assert!(imported);
        assert_eq!(restored.sessions["session-a"].name, "alice");
        assert!(restored.session_history.is_empty());
        assert!(restored.tab_history.is_empty());
        assert_eq!(restored.navigation_history.len(), 1);
        let codes = rejections
            .iter()
            .map(|row| row.code)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            codes,
            BTreeSet::from([
                "legacy_session_history_entry_invalid",
                "legacy_session_history_conflict",
                "legacy_tab_history_entry_invalid",
                "legacy_navigation_history_entry_invalid",
            ])
        );
    }

    fn route_keeper_host_claim(host_generation: u64) -> RouteKeeperHostProcessClaim {
        RouteKeeperHostProcessClaim {
            host_generation,
            boot_epoch: format!("linux:boot:{host_generation}"),
            process_identity: RecordedProcessIdentity {
                pid: u32::try_from(4_000 + host_generation).unwrap(),
                start_token: format!("linux:start:{host_generation}"),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            },
        }
    }

    fn route_keeper_catalog() -> RouteKeeperConnectionCatalog {
        RouteKeeperConnectionCatalog::new((1_u32..=6).map(|sequence| {
            RouteKeeperConnectionBinding {
                slot_id: format!("route-slot-{sequence:02}"),
                connection_key: format!("route-{sequence:02}"),
                connection_name: format!("Agent Browser Route {sequence:02}"),
                route_user: format!("agent-browser-rdp-{sequence}"),
                guacamole_connection_id: u64::from(sequence),
            }
        }))
        .unwrap()
    }

    fn route_keeper_ready_receipt(
        slot_id: String,
        keeper_id: String,
        fence: agent_browser_service_model::RouteKeeperFence,
    ) -> agent_browser_service_model::RouteKeeperProtocolReadyReceipt {
        let sequence = slot_id.rsplit('-').next().unwrap().parse::<u32>().unwrap();
        let session_id = format!("xrdp-{sequence}");
        let display_name = format!(":{sequence}");
        agent_browser_service_model::RouteKeeperProtocolReadyReceipt {
            slot_id,
            keeper_id,
            fence,
            guacamole_connection_uuid: format!("guacamole-{sequence}"),
            xrdp_session_id: session_id.clone(),
            display_name: display_name.clone(),
            xrdp_ownership: Some(
                agent_browser_service_model::RouteKeeperXrdpOwnershipWitness {
                    schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                    boot_id: "boot-fixture".to_string(),
                    route_user: format!("agent-browser-rdp-{sequence}"),
                    route_uid: 2_000 + sequence,
                    session_id: session_id.clone(),
                    session_service: "xrdp-sesman".to_string(),
                    session_scope: format!("session-{session_id}.scope"),
                    scope_invocation_id: format!("invocation-{sequence}"),
                    cgroup_path: format!(
                        "/user.slice/user-{}.slice/session-{session_id}.scope",
                        2_000 + sequence
                    ),
                    cgroup_device: 28,
                    cgroup_inode: 1_000 + u64::from(sequence),
                    leader_pid: 4_100 + sequence,
                    leader_start_ticks: 5_100 + u64::from(sequence),
                    x_server_pid: 4_200 + sequence,
                    x_server_start_ticks: 5_200 + u64::from(sequence),
                    display_name,
                    x11_socket_inode: 6_100 + u64::from(sequence),
                },
            ),
            observed_at: "2026-09-22T12:00:00Z".to_string(),
        }
    }

    fn publish_ready_route_authority(
        store: &mut BrowserRuntimeSqliteStore,
        ready_slots: u32,
        requested_ready_slots: u32,
    ) -> RouteKeeperAuthority {
        let expected = store.load_route_keeper_authority().unwrap();
        let mut authority = expected.clone();
        authority
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
        authority
            .replace_connection_catalog(route_keeper_catalog())
            .unwrap();
        authority
            .request_ready_slots(requested_ready_slots)
            .unwrap();
        for _ in 0..ready_slots {
            let (slot_id, keeper_id, fence) = match authority.next_reconcile_action().unwrap() {
                agent_browser_service_model::RouteKeeperReconcileAction::Start {
                    slot_id,
                    keeper_id,
                    fence,
                    ..
                } => (slot_id, keeper_id, fence),
                other => panic!("expected start action, got {other:?}"),
            };
            authority.record_observing(&slot_id, &fence).unwrap();
            authority
                .record_protocol_ready(route_keeper_ready_receipt(slot_id, keeper_id, fence))
                .unwrap();
        }
        store
            .compare_and_swap_route_keeper_authority(&expected, &authority)
            .unwrap();
        authority
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

    fn sqlite_store(label: &str) -> (TempDirectory, BrowserRuntimeSqliteStore) {
        let directory = TempDirectory::new(label);
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
        (
            directory,
            BrowserRuntimeSqliteStore::open(&database_path).unwrap(),
        )
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
        session_state.browsers.insert(
            "browser-a".to_string(),
            ManagedBrowserInstance {
                id: "browser-a".to_string(),
                profile_id: "work".to_string(),
                pid: 4_242,
                cdp_endpoint: "http://127.0.0.1:4242".to_string(),
                process_identity: None,
                desktop: None,
                active_session_ids: vec!["session-a".to_string()],
            },
        );
        session_state.sessions.insert(
            "session-a".to_string(),
            ManagedBrowserSession {
                id: "session-a".to_string(),
                name: "alice".to_string(),
                profile_id: "work".to_string(),
                browser_id: "browser-a".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: u64::MAX,
                current_tab_id: None,
                handoff_ids: Vec::new(),
            },
        );
        let mut session_source = serde_json::to_value(&session_state).unwrap();
        session_source["sessions"]["broken"] = serde_json::json!({"id":"broken"});
        session_source["sessions"]["orphan"] = serde_json::json!({
            "id": "orphan",
            "name": "orphan",
            "profileId": "work",
            "browserId": "missing-browser",
            "createdAtMs": 1,
            "lastActivityAtMs": 2,
            "expiresAtMs": u64::MAX,
            "currentTabId": null
        });
        session_source["tabs"]["orphan-tab"] = serde_json::json!({
            "id": "orphan-tab",
            "targetId": "target-orphan",
            "browserId": "missing-browser",
            "sessionId": "orphan",
            "createdAtMs": 1,
            "lastActivityAtMs": 2
        });
        session_source["browsers"]["browser-a"]["activeSessionIds"] =
            serde_json::json!(["session-a", "orphan"]);
        session_source["browsers"]["bad-key"] =
            serde_json::json!({"id":"other","profileId":"work"});
        let session_raw = serde_json::to_vec(&session_source).unwrap();
        fs::write(&session_path, &session_raw).unwrap();
        fs::write(
            &catalog_path,
            serde_json::json!({
                "schemaVersion": BROWSER_PROFILE_CATALOG_SCHEMA_V1,
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": "/managed/work",
                        "kind": "named"
                    }
                },
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
        assert_eq!(migration.rejected_record_count, 5);
        assert_eq!(
            migration.rejection_codes,
            vec![
                "legacy_active_session_browser_conflict",
                "legacy_active_session_entry_invalid",
                "legacy_active_tab_session_conflict",
                "legacy_browser_entry_invalid",
                "legacy_browser_session_membership_repaired"
            ]
        );
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
        assert_eq!(
            fs::read(
                migration
                    .archive_directory
                    .join(BROWSER_SESSION_STATE_FILENAME)
            )
            .unwrap(),
            session_raw
        );
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
        assert_eq!(
            store.load_session_state().unwrap().sessions["session-a"].name,
            "alice"
        );
        assert!(!store
            .load_session_state()
            .unwrap()
            .sessions
            .contains_key("orphan"));
        assert!(!store
            .load_session_state()
            .unwrap()
            .tabs
            .contains_key("orphan-tab"));
        assert_eq!(
            store.load_session_state().unwrap().browsers["browser-a"].active_session_ids,
            vec!["session-a"]
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
    fn sqlite_presentation_queue_persists_mutations_and_rolls_back_errors() {
        let directory = TempDirectory::new("browser-runtime-presentation-queue");
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
        assert!(store.load_presentation_queue().unwrap().entries.is_empty());

        let entry = store
            .mutate_presentation_queue(|queue| {
                queue.enqueue(
                    "request-a".to_string(),
                    "fingerprint-a".to_string(),
                    PresentationRequestPriority::NewOpen,
                    10,
                    100,
                    2,
                )
            })
            .unwrap();
        assert_eq!(entry.key, "request-a");
        let persisted = store.load_presentation_queue().unwrap();
        assert_eq!(persisted.entries.len(), 1);
        assert!(persisted.entries.contains_key("request-a"));

        assert_eq!(
            store.mutate_presentation_queue(|queue| -> Result<(), String> {
                queue.enqueue(
                    "request-rollback".to_string(),
                    "fingerprint-rollback".to_string(),
                    PresentationRequestPriority::NewOpen,
                    20,
                    100,
                    2,
                )?;
                Err("presentation_queue_fixture_failure".to_string())
            }),
            Err("presentation_queue_fixture_failure".to_string())
        );
        let after_error = store.load_presentation_queue().unwrap();
        assert_eq!(after_error.entries.len(), 1);
        assert_eq!(after_error.entries["request-a"].key, "request-a");
        assert!(!after_error.entries.contains_key("request-rollback"));
        drop(store);

        let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let reopened = reopened.load_presentation_queue().unwrap();
        assert_eq!(reopened.entries.len(), 1);
        assert_eq!(reopened.entries["request-a"].key, "request-a");
    }

    #[test]
    fn reserve_idle_route_stop_waits_for_cooldown_resets_on_pending_work_and_reserves_exact_stop() {
        let (_directory, mut store) = sqlite_store("browser-runtime-scale-in-cooldown");
        let authority = publish_ready_route_authority(&mut store, 5, 6);
        store
            .update_runtime_config(BrowserRuntimeConfigPatch {
                scale_in_cooldown_ms: Some(100),
                ..BrowserRuntimeConfigPatch::default()
            })
            .unwrap();

        assert_eq!(store.reserve_idle_route_stop(1_000).unwrap(), None);

        let pending = store
            .reserve_operation("pending-scale-in", "browser:fixture", serde_json::json!({}))
            .unwrap();
        assert_eq!(store.reserve_idle_route_stop(1_100).unwrap(), None);
        store
            .commit_operation(
                &pending.operation_id,
                pending.generation,
                serde_json::json!({"committed": true}),
            )
            .unwrap();

        assert_eq!(
            store.reserve_idle_route_stop(1_199).unwrap(),
            None,
            "the pending obligation clears prior idle evidence"
        );
        assert_eq!(store.reserve_idle_route_stop(1_298).unwrap(), None);
        let (stopping, action) = store.reserve_idle_route_stop(1_299).unwrap().unwrap();
        let (slot_id, fence) = match action {
            RouteKeeperReconcileAction::Stop { slot_id, fence, .. } => (slot_id, fence),
            other => panic!("expected stop action, got {other:?}"),
        };
        assert_eq!(slot_id, "route-slot-05");
        assert_eq!(fence, authority.records[&slot_id].fence);
        assert_eq!(
            stopping.records[&slot_id].phase,
            agent_browser_service_model::RouteKeeperPhase::Stopping
        );
        assert_eq!(stopping.requested_ready_slots, 4);
        assert_eq!(store.load_route_keeper_authority().unwrap(), stopping);
    }

    #[test]
    fn reserve_idle_route_stop_keeps_browser_and_handoff_references_out_of_selection() {
        let (_directory, mut store) = sqlite_store("browser-runtime-scale-in-references");
        let authority = publish_ready_route_authority(&mut store, 5, 5);
        store
            .update_runtime_config(BrowserRuntimeConfigPatch {
                scale_in_cooldown_ms: Some(100),
                ..BrowserRuntimeConfigPatch::default()
            })
            .unwrap();

        let mut state = BrowserSessionState::default();
        state.browsers.insert(
            "browser-01".to_string(),
            agent_browser_service_model::ManagedBrowserInstance {
                id: "browser-01".to_string(),
                profile_id: "profile-01".to_string(),
                pid: 4_001,
                cdp_endpoint: "http://127.0.0.1:9222".to_string(),
                process_identity: None,
                desktop: Some(agent_browser_service_model::BrowserDesktopAssignment {
                    route_id: "route-slot-01".to_string(),
                    display_name: ":1".to_string(),
                    live_browser_count: 1,
                }),
                active_session_ids: Vec::new(),
            },
        );
        store.save_session_state(&state).unwrap();
        for sequence in 2..=5 {
            store
                .save_manager_handoff(&RemoteViewHandoff {
                    id: format!("handoff-{sequence}"),
                    last_route_id: Some(format!("route-slot-{sequence:02}")),
                    ..RemoteViewHandoff::default()
                })
                .unwrap();
        }

        assert_eq!(store.reserve_idle_route_stop(1_000).unwrap(), None);
        assert_eq!(
            store.reserve_idle_route_stop(1_100).unwrap(),
            None,
            "every ready route has a durable browser or handoff reference"
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), authority);
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
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
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
    fn additive_catalog_publication_preserves_active_routes_and_enables_new_capacity() {
        let (_directory, mut store) = sqlite_store("browser-runtime-catalog-growth");
        let before = publish_ready_route_authority(&mut store, 6, 6);
        let mut catalog = before.connection_catalog.clone();
        catalog.bindings.insert(
            "route-slot-07".to_string(),
            agent_browser_service_model::RouteKeeperConnectionBinding {
                slot_id: "route-slot-07".to_string(),
                connection_key: "route-07".to_string(),
                connection_name: "Agent Browser Route 07".to_string(),
                route_user: "agent-browser-rdp-7".to_string(),
                guacamole_connection_id: 7,
            },
        );
        assert_eq!(
            store
                .publish_route_keeper_connection_catalog(catalog.clone())
                .unwrap(),
            RouteKeeperConnectionCatalogPublication::Published
        );
        let expanded = store.load_route_keeper_authority().unwrap();
        for (slot, record) in &before.records {
            assert_eq!(&expanded.records[slot], record);
        }
        assert_eq!(expanded.policy.maximum_slots, 7);
        assert_eq!(store.load_runtime_config().unwrap().maximum_displays, 6);
        assert_eq!(
            store
                .publish_route_keeper_connection_catalog(catalog)
                .unwrap(),
            RouteKeeperConnectionCatalogPublication::Unchanged
        );
        // A receipt that began before the append merges the new catalog.
        store
            .compare_and_swap_route_keeper_authority(&before, &before)
            .unwrap();
        assert_eq!(store.load_route_keeper_authority().unwrap(), expanded);
        store
            .update_runtime_config(BrowserRuntimeConfigPatch {
                maximum_displays: Some(7),
                warm_target: Some(7),
                ..Default::default()
            })
            .unwrap();
        let current = store.load_route_keeper_authority().unwrap();
        let mut starting = current.clone();
        let action = starting.next_reconcile_action().unwrap();
        let RouteKeeperReconcileAction::Start { slot_id, fence, .. } = action else {
            panic!("expected the newly provisioned slot to start");
        };
        assert_eq!(slot_id, "route-slot-07");
        assert_eq!(
            fence.connection_catalog_digest,
            starting.connection_catalog.digest().unwrap()
        );
        store
            .compare_and_swap_route_keeper_authority(&current, &starting)
            .unwrap();
        let mut rebound = starting.connection_catalog.clone();
        rebound
            .bindings
            .get_mut("route-slot-01")
            .unwrap()
            .route_user = "other-user".to_string();
        assert!(store
            .publish_route_keeper_connection_catalog(rebound)
            .is_err());
        assert_eq!(store.load_route_keeper_authority().unwrap(), starting);
    }

    #[test]
    fn catalog_growth_merges_in_flight_stop_but_rejects_changed_route_evidence() {
        let (_directory, mut store) = sqlite_store("browser-runtime-catalog-receipt-race");
        let before = publish_ready_route_authority(&mut store, 6, 6);
        let mut stopping = before.clone();
        stopping
            .begin_stop("route-slot-01", &before.records["route-slot-01"].fence)
            .unwrap();
        let mut catalog = before.connection_catalog.clone();
        for sequence in 7..=8 {
            let mut binding = catalog.bindings["route-slot-06"].clone();
            binding.slot_id = format!("route-slot-{sequence:02}");
            binding.connection_key = format!("route-{sequence:02}");
            binding.connection_name = format!("Agent Browser Route {sequence:02}");
            binding.route_user = format!("agent-browser-rdp-{sequence}");
            binding.guacamole_connection_id = sequence;
            catalog.bindings.insert(binding.slot_id.clone(), binding);
            store
                .publish_route_keeper_connection_catalog(catalog.clone())
                .unwrap();
        }
        let expanded = store.load_route_keeper_authority().unwrap();
        let mut new_host = before.clone();
        new_host
            .register_host_process_claim(route_keeper_host_claim(2))
            .unwrap();
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(&before, &new_host),
            Err("route_keeper_authority_compare_and_swap_conflict".to_string())
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), expanded);
        store
            .compare_and_swap_route_keeper_authority(&before, &stopping)
            .unwrap();
        let merged = store.load_route_keeper_authority().unwrap();
        assert_eq!(
            merged.records["route-slot-01"],
            stopping.records["route-slot-01"]
        );
        assert_eq!(merged.connection_catalog, catalog);
        assert_eq!(merged.records.len(), 8);
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(&before, &before),
            Err("route_keeper_authority_compare_and_swap_conflict".to_string())
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), merged);
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
        active
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
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
        active
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
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
        current
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
        current.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&stale, &current)
            .unwrap();

        let mut stale_next = stale.clone();
        stale_next
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
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
    fn route_keeper_effect_cas_preserves_concurrent_demand_but_not_other_conflicts() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-demand-cas");
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
        let sessions_before = store.load_session_state().unwrap();
        let handoffs_before = store.load_handoff_registry().unwrap();

        let empty = store.load_route_keeper_authority().unwrap();
        let mut base = empty.clone();
        base.replace_connection_catalog(route_keeper_catalog())
            .unwrap();
        base.register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&empty, &base)
            .unwrap();

        let mut effect_next = base.clone();
        let (slot_id, start_fence) = match effect_next.next_reconcile_action().unwrap() {
            agent_browser_service_model::RouteKeeperReconcileAction::Start {
                slot_id,
                fence,
                ..
            } => (slot_id, fence),
            other => panic!("expected start action, got {other:?}"),
        };
        let mut demand_writer = base.clone();
        demand_writer.request_ready_slots(2).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&base, &demand_writer)
            .unwrap();
        store
            .update_runtime_config(BrowserRuntimeConfigPatch {
                minimum_ready: Some(2),
                warm_target: Some(3),
                ..BrowserRuntimeConfigPatch::default()
            })
            .unwrap();

        store
            .compare_and_swap_route_keeper_authority(&base, &effect_next)
            .unwrap();
        let after_effect = store.load_route_keeper_authority().unwrap();
        assert_eq!(after_effect.requested_ready_slots, 2);
        assert_eq!(after_effect.policy.minimum_ready, 2);
        assert_eq!(after_effect.policy.warm_target, 3);
        assert_eq!(
            after_effect.records[&slot_id].phase,
            agent_browser_service_model::RouteKeeperPhase::Starting
        );
        assert_eq!(store.load_session_state().unwrap(), sessions_before);
        assert_eq!(store.load_handoff_registry().unwrap(), handoffs_before);

        let mut phase_writer = after_effect.clone();
        phase_writer
            .record_observing(&slot_id, &start_fence)
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&after_effect, &phase_writer)
            .unwrap();
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(&after_effect, &phase_writer),
            Err("route_keeper_authority_compare_and_swap_conflict".to_string()),
            "a stale phase is not an independent intent merge"
        );
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(&base, &effect_next),
            Err("route_keeper_authority_compare_and_swap_conflict".to_string()),
            "only concurrent policy and demand intent may merge; stale fences remain refused"
        );

        let demand_snapshot = store.load_route_keeper_authority().unwrap();
        let mut first_demand_writer = demand_snapshot.clone();
        first_demand_writer.request_ready_slots(3).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&demand_snapshot, &first_demand_writer)
            .unwrap();
        let mut competing_demand_writer = demand_snapshot.clone();
        competing_demand_writer.request_ready_slots(1).unwrap();
        assert_eq!(
            store.compare_and_swap_route_keeper_authority(
                &demand_snapshot,
                &competing_demand_writer
            ),
            Err("route_keeper_authority_compare_and_swap_conflict".to_string()),
            "demand writers must retry from a fresh authority"
        );
        assert_eq!(
            store
                .load_route_keeper_authority()
                .unwrap()
                .requested_ready_slots,
            3
        );
    }

    #[test]
    fn historical_v4_authority_without_demand_loads_and_cas_replays_zero() {
        let directory = TempDirectory::new("browser-runtime-route-keeper-demand-v4");
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
        let mut historical =
            serde_json::to_value(store.load_route_keeper_authority().unwrap()).unwrap();
        historical
            .as_object_mut()
            .unwrap()
            .remove("requestedReadySlots");
        store
            .connection
            .execute(
                "UPDATE state_documents SET json = ?1 WHERE kind = ?2",
                params![historical.to_string(), ROUTE_KEEPER_AUTHORITY_DOCUMENT],
            )
            .unwrap();

        let loaded = store.load_route_keeper_authority().unwrap();
        assert_eq!(loaded.requested_ready_slots, 0);
        store
            .compare_and_swap_route_keeper_authority(&loaded, &loaded)
            .unwrap();
        assert_eq!(
            store
                .load_route_keeper_authority()
                .unwrap()
                .requested_ready_slots,
            0
        );
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
    fn route_keeper_v1_document_migrates_in_place_to_v4_without_process_claim() {
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
            agent_browser_service_model::ROUTE_KEEPER_AUTHORITY_SCHEMA_V4
        );
        assert!(migrated.host_process_claims.is_empty());
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
        assert_eq!(persisted_schema, ROUTE_KEEPER_AUTHORITY_SCHEMA_V4);

        let mut newer = migrated.clone();
        newer
            .register_host_process_claim(route_keeper_host_claim(2))
            .unwrap();
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
                    xrdp_ownership: Some(
                        agent_browser_service_model::RouteKeeperXrdpOwnershipWitness {
                            schema_version: "agent-browser.route-keeper-xrdp-ownership.v1"
                                .to_string(),
                            boot_id: "boot-fixture".to_string(),
                            route_user: "agent-browser-rdp-1".to_string(),
                            route_uid: 2001,
                            session_id: "xrdp-01".to_string(),
                            session_service: "xrdp-sesman".to_string(),
                            session_scope: "session-xrdp-01.scope".to_string(),
                            scope_invocation_id: "invocation-fixture".to_string(),
                            cgroup_path: "/user.slice/user-2001.slice/session-xrdp-01.scope"
                                .to_string(),
                            cgroup_device: 28,
                            cgroup_inode: 1001,
                            leader_pid: 4101,
                            leader_start_ticks: 5101,
                            x_server_pid: 4102,
                            x_server_start_ticks: 5102,
                            display_name: ":10".to_string(),
                            x11_socket_inode: 6101,
                        },
                    ),
                    observed_at: "2026-09-19T23:00:00Z".to_string(),
                },
            )
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&migrated, &newer)
            .unwrap();

        let stale: RouteKeeperAuthority = serde_json::from_str(&legacy_json).unwrap();
        let stale = stale.upgrade_from_v1().unwrap();
        assert!(!compare_and_swap_route_keeper_schema_upgrade(
            &store.connection,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V1,
            &legacy_json,
            &stale,
        )
        .unwrap());
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
        active_v1
            .as_object_mut()
            .unwrap()
            .remove("hostProcessClaims");
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

        let mut active_v2 = serde_json::to_value(&newer).unwrap();
        active_v2["schemaVersion"] = serde_json::json!(ROUTE_KEEPER_AUTHORITY_SCHEMA_V2);
        active_v2
            .as_object_mut()
            .unwrap()
            .remove("hostProcessClaims");
        store
            .connection
            .execute(
                "UPDATE state_documents SET schema_version = ?1, json = ?2 WHERE kind = ?3",
                params![
                    ROUTE_KEEPER_AUTHORITY_SCHEMA_V2,
                    serde_json::to_string(&active_v2).unwrap(),
                    ROUTE_KEEPER_AUTHORITY_DOCUMENT
                ],
            )
            .unwrap();
        assert_eq!(
            store.load_route_keeper_authority(),
            Err("route_keeper_v2_active_host_claim_migration_unproven".to_string())
        );
        let retained_schema: String = store
            .connection
            .query_row(
                "SELECT schema_version FROM state_documents WHERE kind = ?1",
                params![ROUTE_KEEPER_AUTHORITY_DOCUMENT],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained_schema, ROUTE_KEEPER_AUTHORITY_SCHEMA_V2);
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
    fn runtime_config_patch_is_atomic_idempotent_and_syncs_keeper_policy() {
        let directory = TempDirectory::new("browser-runtime-config-patch");
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
        let original_config = store.load_runtime_config().unwrap();
        let original_authority = store.load_route_keeper_authority().unwrap();
        let patch = BrowserRuntimeConfigPatch {
            minimum_ready: Some(2),
            warm_target: Some(3),
            maximum_displays: Some(5),
            maximum_browsers_per_display: Some(2),
            maximum_queue_depth: Some(12),
            request_deadline_ms: Some(45_000),
            scale_in_cooldown_ms: Some(120_000),
        };

        let updated = store.update_runtime_config(patch.clone()).unwrap();
        assert_eq!(updated.revision, original_config.revision + 1);
        assert_eq!(updated.minimum_ready, 2);
        assert_eq!(updated.warm_target, 3);
        assert_eq!(updated.maximum_displays, 5);
        assert_eq!(updated.maximum_browsers_per_display, 2);
        assert_eq!(updated.maximum_queue_depth, 12);
        assert_eq!(updated.request_deadline_ms, 45_000);
        assert_eq!(updated.scale_in_cooldown_ms, 120_000);
        let synchronized = store.load_route_keeper_authority().unwrap();
        assert_eq!(synchronized.policy.minimum_ready, 2);
        assert_eq!(synchronized.policy.warm_target, 3);
        assert_eq!(
            synchronized.policy.maximum_slots, original_authority.policy.maximum_slots,
            "logical display limits do not alter physical provisioned slots"
        );
        assert_eq!(synchronized.records, original_authority.records);
        assert_eq!(
            synchronized.connection_catalog,
            original_authority.connection_catalog
        );

        assert_eq!(store.update_runtime_config(patch).unwrap(), updated);
        assert_eq!(store.load_runtime_config().unwrap(), updated);

        let before_active = store.load_route_keeper_authority().unwrap();
        let mut active = before_active.clone();
        active
            .replace_connection_catalog(route_keeper_catalog())
            .unwrap();
        active
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
        active.request_ready_slots(4).unwrap();
        active.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&before_active, &active)
            .unwrap();
        let active_records = active.records.clone();

        let lowered = store
            .update_runtime_config(BrowserRuntimeConfigPatch {
                maximum_displays: Some(3),
                ..BrowserRuntimeConfigPatch::default()
            })
            .unwrap();
        assert_eq!(lowered.revision, updated.revision + 1);
        assert_eq!(lowered.maximum_displays, 3);
        let lowered_authority = store.load_route_keeper_authority().unwrap();
        assert_eq!(lowered_authority.requested_ready_slots, 3);
        assert_eq!(
            lowered_authority.records, active_records,
            "lowering logical capacity does not stop or rewrite active routes"
        );
        let mut stale_cap_demand = lowered_authority.clone();
        stale_cap_demand.request_ready_slots(5).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&lowered_authority, &stale_cap_demand)
            .unwrap();
        assert_eq!(
            store.load_route_keeper_authority().unwrap(),
            lowered_authority,
            "a waiter using a stale cap cannot restore excess demand"
        );

        assert_eq!(
            store.update_runtime_config(BrowserRuntimeConfigPatch {
                maximum_displays: Some(original_authority.policy.maximum_slots + 1),
                ..BrowserRuntimeConfigPatch::default()
            }),
            Err(format!(
                "browser_runtime_config_provisioned_capacity_exceeded:{}:{}",
                original_authority.policy.maximum_slots + 1,
                original_authority.policy.maximum_slots
            ))
        );
        assert_eq!(store.load_runtime_config().unwrap(), lowered);
        assert_eq!(
            store.load_route_keeper_authority().unwrap(),
            lowered_authority
        );

        assert_eq!(
            store.update_runtime_config(BrowserRuntimeConfigPatch {
                minimum_ready: Some(3),
                warm_target: Some(2),
                ..BrowserRuntimeConfigPatch::default()
            }),
            Err("browser_runtime_config_warm_target_invalid".to_string())
        );
        assert_eq!(store.load_runtime_config().unwrap(), lowered);
        assert_eq!(
            store.load_route_keeper_authority().unwrap(),
            lowered_authority
        );
    }

    #[test]
    fn manual_seeding_launch_observation_commits_with_operation_and_current_route() {
        let (directory, mut store) = sqlite_store("manual-seeding-launch-observation");
        let mut catalog = store.load_profile_catalog().unwrap();
        catalog.profiles.insert(
            "work".to_string(),
            agent_browser_service_model::BrowserProfileCatalogEntry {
                id: "work".to_string(),
                name: "Work".to_string(),
                user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
                kind: agent_browser_service_model::BrowserProfileKind::Named,
            },
        );
        store.save_profile_catalog(&catalog).unwrap();
        let request = ManualSeedingReservation {
            operation_id: "seed-op".to_string(),
            profile_id: "work".to_string(),
            target_service_id: "service-a".to_string(),
            handoff_id: "seed-handoff".to_string(),
            requested_url: Some("https://example.test/login".to_string()),
            executable_path: "/opt/chrome".to_string(),
        };
        let (reserved, operation) = store.reserve_manual_seeding(&request).unwrap();
        assert_eq!(reserved.state, ManualSeedingState::Reserved);

        let expected = store.load_route_keeper_authority().unwrap();
        let mut authority = expected.clone();
        authority
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
        let mut connections = route_keeper_catalog();
        connections.public_operator_url = Some("https://dashboard.example/remote-view".to_string());
        authority.replace_connection_catalog(connections).unwrap();
        authority.request_ready_slots(1).unwrap();
        let (slot_id, keeper_id, fence) = match authority.next_reconcile_action().unwrap() {
            agent_browser_service_model::RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                ..
            } => (slot_id, keeper_id, fence),
            other => panic!("expected start action, got {other:?}"),
        };
        authority.record_observing(&slot_id, &fence).unwrap();
        authority
            .record_protocol_ready(route_keeper_ready_receipt(
                slot_id.clone(),
                keeper_id,
                fence,
            ))
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&expected, &authority)
            .unwrap();
        let binding = authority.ready_handoff_binding(&slot_id, ":1").unwrap();
        let pending_open = store
            .reserve_operation(
                "open-before-seeding",
                "browser-runtime-open",
                serde_json::json!({"intent":{"slot":{"routeId":slot_id}}}),
            )
            .unwrap();
        assert_eq!(
            store.bind_manual_seeding_route("work", "seed-op", operation.generation, &binding),
            Err("manual_seeding_route_busy".to_string())
        );
        store
            .commit_operation(
                &pending_open.operation_id,
                pending_open.generation,
                serde_json::json!({"cancelledBeforeEffect":true}),
            )
            .unwrap();
        let mut occupied = BrowserSessionState::default();
        occupied.browsers.insert(
            "browser-occupied".to_string(),
            ManagedBrowserInstance {
                id: "browser-occupied".to_string(),
                profile_id: "other-profile".to_string(),
                pid: 4_000,
                cdp_endpoint: "http://127.0.0.1:9222".to_string(),
                process_identity: None,
                desktop: Some(agent_browser_service_model::BrowserDesktopAssignment {
                    route_id: slot_id.clone(),
                    display_name: ":1".to_string(),
                    live_browser_count: 1,
                }),
                active_session_ids: Vec::new(),
            },
        );
        store.save_session_state(&occupied).unwrap();
        assert_eq!(
            store.bind_manual_seeding_route("work", "seed-op", operation.generation, &binding),
            Err("manual_seeding_route_busy".to_string())
        );
        store
            .save_session_state(&BrowserSessionState::default())
            .unwrap();
        let bound = store
            .bind_manual_seeding_route("work", "seed-op", operation.generation, &binding)
            .unwrap();
        assert_eq!(bound.route_slot_id.as_deref(), Some(slot_id.as_str()));
        assert_eq!(
            store
                .bind_manual_seeding_route("work", "seed-op", operation.generation, &binding)
                .unwrap(),
            bound
        );
        store.save_session_state(&occupied).unwrap();
        assert_eq!(
            store.bind_manual_seeding_route("work", "seed-op", operation.generation, &binding),
            Err("manual_seeding_route_busy".to_string())
        );
        store
            .save_session_state(&BrowserSessionState::default())
            .unwrap();
        assert_eq!(
            store.reserve_operation(
                "open-collision",
                "browser-runtime-open",
                serde_json::json!({"intent":{"slot":{"routeId":slot_id}}}),
            ),
            Err(format!(
                "browser_runtime_open_route_reserved_by_manual_seeding:{slot_id}"
            ))
        );
        assert!(store.find_operation("open-collision").unwrap().is_none());
        assert_eq!(
            store.reserve_operation(
                "open-profile-collision",
                "browser-runtime-open",
                serde_json::json!({
                    "intent": {
                        "browser": {"profileId": "work"},
                        "slot": {"routeId": "different-slot"}
                    }
                }),
            ),
            Err("browser_runtime_open_profile_reserved_by_manual_seeding:work".to_string())
        );
        let issued = store
            .mark_manual_seeding_launch_issued("work", "seed-op", operation.generation, &binding)
            .unwrap();
        assert_eq!(issued.0.state, ManualSeedingState::LaunchIssued);
        assert_eq!(issued.1.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(
            store.mark_manual_seeding_launch_issued(
                "work",
                "seed-op",
                operation.generation,
                &binding
            ),
            Err("manual_seeding_launch_issue_state_invalid".to_string())
        );
        assert_eq!(
            store.abort_manual_seeding_before_launch(
                "work",
                "seed-op",
                operation.generation,
                "late_abort",
            ),
            Err("manual_seeding_abort_after_launch_forbidden".to_string())
        );
        let process = RecordedProcessIdentity {
            pid: i32::MAX as u32,
            start_token: "linux:start:4242".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        let observed = store
            .observe_manual_seeding_launch(
                "work",
                "seed-op",
                operation.generation,
                &binding,
                &process,
            )
            .unwrap();
        assert_eq!(observed.0.state, ManualSeedingState::LaunchObserved);
        assert_eq!(observed.0.process_identity, Some(process.clone()));
        assert_eq!(observed.1.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(
            store
                .observe_manual_seeding_launch(
                    "work",
                    "seed-op",
                    operation.generation,
                    &binding,
                    &process
                )
                .unwrap(),
            observed
        );
        let mut changed = binding.clone();
        changed.guacamole_connection_uuid = "changed".to_string();
        assert_eq!(
            store.observe_manual_seeding_launch(
                "work",
                "seed-op",
                operation.generation,
                &changed,
                &process
            ),
            Err("manual_seeding_route_binding_changed".to_string())
        );
        assert_eq!(
            store.load_manual_seeding_record("work").unwrap(),
            Some(observed.0)
        );
        assert_eq!(store.load_operation("seed-op").unwrap(), observed.1);
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());
        let operator_visible = serde_json::json!({
            "state": "ready",
            "routeId": slot_id,
            "displayName": ":1",
            "manualSeedingProcess": {"state": "ready", "pid": process.pid},
            "components": {"guacamole": {"externalUrl": "https://provider.invalid/guacamole/#/client/raw"}},
        });
        let mut not_visible = operator_visible.clone();
        not_visible["state"] = serde_json::json!("not_ready");
        assert_eq!(
            store.publish_manual_seeding_ready(
                "work",
                "seed-op",
                operation.generation,
                &binding,
                &process,
                &not_visible,
                "dashboard-generation-a",
                "2026-09-25T00:00:00Z",
            ),
            Err("manual_seeding_operator_visibility_unproven".to_string())
        );
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());
        let published = store
            .publish_manual_seeding_ready(
                "work",
                "seed-op",
                operation.generation,
                &binding,
                &process,
                &operator_visible,
                "dashboard-generation-a",
                "2026-09-25T00:00:00Z",
            )
            .unwrap();
        assert_eq!(published.0.state, "ready");
        assert!(!published
            .1
            .result
            .as_ref()
            .unwrap()
            .to_string()
            .contains("provider.invalid"));
        assert!(published
            .0
            .handoff_url
            .as_deref()
            .unwrap()
            .contains("/remote-view/seed-handoff"));
        assert_eq!(published.1.state, BrowserRuntimeOperationState::Committed);
        assert_eq!(
            store
                .load_manual_seeding_record("work")
                .unwrap()
                .unwrap()
                .state,
            ManualSeedingState::Ready
        );
        assert_eq!(
            store
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .get("seed-handoff"),
            Some(&published.0)
        );
        assert_eq!(store.load_operation("seed-op").unwrap(), published.1);
        assert_eq!(
            store
                .publish_manual_seeding_ready(
                    "work",
                    "seed-op",
                    operation.generation,
                    &binding,
                    &process,
                    &operator_visible,
                    "dashboard-generation-a",
                    "2026-09-25T00:00:00Z",
                )
                .unwrap(),
            published
        );
        let closed = store
            .close_manual_seeding_after_process_exit(
                "work",
                "service-a",
                "seed-handoff",
                &process,
                "2026-09-25T00:01:00Z",
            )
            .unwrap();
        assert_eq!(closed.0.state, ManualSeedingState::Closed);
        assert_eq!(closed.1.as_ref().unwrap().state, "closed");
        assert_eq!(
            store
                .load_handoff_registry()
                .unwrap()
                .handoffs
                .get("seed-handoff"),
            closed.1.as_ref()
        );
        assert_eq!(
            store
                .close_manual_seeding_after_process_exit(
                    "work",
                    "service-a",
                    "seed-handoff",
                    &process,
                    "2026-09-25T00:01:00Z",
                )
                .unwrap(),
            closed
        );
    }

    #[test]
    fn uncertain_manual_seeding_launch_retains_pid_and_blocks_reuse() {
        let (directory, mut store) = sqlite_store("manual-seeding-uncertain-launch");
        let mut catalog = store.load_profile_catalog().unwrap();
        catalog.profiles.insert(
            "work".to_string(),
            agent_browser_service_model::BrowserProfileCatalogEntry {
                id: "work".to_string(),
                name: "Work".to_string(),
                user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
                kind: agent_browser_service_model::BrowserProfileKind::Named,
            },
        );
        store.save_profile_catalog(&catalog).unwrap();
        let request = ManualSeedingReservation {
            operation_id: "seed-uncertain".to_string(),
            profile_id: "work".to_string(),
            target_service_id: "service-a".to_string(),
            handoff_id: "seed-uncertain-handoff".to_string(),
            requested_url: None,
            executable_path: "/opt/chrome".to_string(),
        };
        let (_, operation) = store.reserve_manual_seeding(&request).unwrap();
        let expected = store.load_route_keeper_authority().unwrap();
        let mut authority = expected.clone();
        authority
            .register_host_process_claim(route_keeper_host_claim(1))
            .unwrap();
        let mut connections = route_keeper_catalog();
        connections.public_operator_url = Some("https://dashboard.example/remote-view".to_string());
        authority.replace_connection_catalog(connections).unwrap();
        authority.request_ready_slots(1).unwrap();
        let (slot_id, keeper_id, fence) = match authority.next_reconcile_action().unwrap() {
            agent_browser_service_model::RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                ..
            } => (slot_id, keeper_id, fence),
            other => panic!("expected start action, got {other:?}"),
        };
        authority.record_observing(&slot_id, &fence).unwrap();
        authority
            .record_protocol_ready(route_keeper_ready_receipt(
                slot_id.clone(),
                keeper_id,
                fence,
            ))
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&expected, &authority)
            .unwrap();
        let binding = authority.ready_handoff_binding(&slot_id, ":1").unwrap();
        store
            .bind_manual_seeding_route("work", "seed-uncertain", operation.generation, &binding)
            .unwrap();
        store
            .mark_manual_seeding_launch_issued(
                "work",
                "seed-uncertain",
                operation.generation,
                &binding,
            )
            .unwrap();
        assert_eq!(
            store.observe_uncertain_manual_seeding_launch(
                "work",
                "seed-uncertain",
                operation.generation,
                &binding,
                0,
            ),
            Err("manual_seeding_uncertain_pid_invalid".to_string())
        );
        let observed = store
            .observe_uncertain_manual_seeding_launch(
                "work",
                "seed-uncertain",
                operation.generation,
                &binding,
                i32::MAX as u32,
            )
            .unwrap();
        assert_eq!(observed.0.state, ManualSeedingState::RecoveryRequired);
        assert_eq!(observed.0.uncertain_launch_pid, Some(i32::MAX as u32));
        assert_eq!(observed.1.state, BrowserRuntimeOperationState::Observed);
        assert_eq!(
            store
                .observe_uncertain_manual_seeding_launch(
                    "work",
                    "seed-uncertain",
                    operation.generation,
                    &binding,
                    i32::MAX as u32,
                )
                .unwrap(),
            observed
        );
        assert_eq!(
            store.reserve_manual_seeding(&ManualSeedingReservation {
                operation_id: "seed-retry".to_string(),
                handoff_id: "seed-retry-handoff".to_string(),
                ..request.clone()
            }),
            Err("manual_seeding_profile_busy:work".to_string())
        );
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());
        let reconciled = store
            .reconcile_absent_uncertain_manual_seeding_launch(
                "work",
                "seed-uncertain",
                operation.generation,
                i32::MAX as u32,
            )
            .unwrap();
        assert_eq!(reconciled.0.state, ManualSeedingState::Closed);
        assert_eq!(reconciled.1.state, BrowserRuntimeOperationState::Committed);
        assert_eq!(
            reconciled.1.result.as_ref().unwrap()["retryRequiresNewOperation"],
            true
        );
        assert_eq!(
            store
                .reconcile_absent_uncertain_manual_seeding_launch(
                    "work",
                    "seed-uncertain",
                    operation.generation,
                    i32::MAX as u32,
                )
                .unwrap(),
            reconciled
        );
        assert!(store.load_handoff_registry().unwrap().handoffs.is_empty());
        let retry = ManualSeedingReservation {
            operation_id: "seed-retry".to_string(),
            handoff_id: "seed-retry-handoff".to_string(),
            ..request
        };
        let (_, next_operation) = store.reserve_manual_seeding(&retry).unwrap();
        store
            .bind_manual_seeding_route("work", "seed-retry", next_operation.generation, &binding)
            .unwrap();
        store
            .mark_manual_seeding_launch_issued(
                "work",
                "seed-retry",
                next_operation.generation,
                &binding,
            )
            .unwrap();
        store
            .observe_uncertain_manual_seeding_launch(
                "work",
                "seed-retry",
                next_operation.generation,
                &binding,
                std::process::id(),
            )
            .unwrap();
        assert_eq!(
            store.reconcile_absent_uncertain_manual_seeding_launch(
                "work",
                "seed-retry",
                next_operation.generation,
                std::process::id(),
            ),
            Err("manual_seeding_uncertain_pid_not_absent".to_string())
        );
        assert_eq!(
            store
                .load_manual_seeding_record("work")
                .unwrap()
                .unwrap()
                .state,
            ManualSeedingState::RecoveryRequired
        );
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
