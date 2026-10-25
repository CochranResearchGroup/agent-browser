//! Runtime SQLite custody for stopped-source duplication. This adapter never
//! infers clean shutdown from missing inventory and is not a public wire API.
use super::profile_duplicate_copy::{DuplicateProfile, SourceProfile};
use agent_browser_service_model::{
    BrowserLaunchCustodyRecord, BrowserSessionState, RecordedProcessIdentity,
    BROWSER_SESSION_STATE_SCHEMA_V1,
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

/// Only a native owner adapter may supply this non-deserializable evidence.
/// It must retain the actual close operation and original kernel identity.
/// Neither inventory absence nor a successful force-close qualifies as clean.
#[derive(Clone, Debug)]
pub struct BackendGracefulCloseEvidence {
    pub owner_operation_id: String,
    pub source_profile_id: String,
    pub source_generation: u64,
    pub original: RecordedProcessIdentity,
    pub polite_close_attempted: bool,
    pub polite_close_succeeded: bool,
    pub polite_close_failed: bool,
    pub exact_process_exited: bool,
    pub profile_lock_released: bool,
    pub force_kill_attempted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalProcessState {
    Absent,
    Live,
    Unknown,
}
#[derive(Default, Deserialize)]
struct LaunchLedger {
    #[serde(default)]
    records: BTreeMap<String, BrowserLaunchCustodyRecord>,
    #[serde(default)]
    reconciled_absent: BTreeSet<String>,
}
/// Private pre-close attachment provenance captured from the browser owner.
/// Public commands cannot supply process identities or deserialize this value;
/// only the private operation journal retains it across owner restart.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BorrowedSourceAnchor {
    operation_id: String,
    source_profile_id: String,
    source_generation: u64,
    source_directory: std::path::PathBuf,
    original: RecordedProcessIdentity,
}
impl BorrowedSourceAnchor {
    pub fn capture(
        operation_id: &str,
        source: &SourceProfile,
        state: &BrowserSessionState,
        browser_id: &str,
    ) -> Result<Self, String> {
        let browser = state
            .browsers
            .get(browser_id)
            .ok_or("profile_duplicate_source_browser_missing")?;
        let attachment = state
            .app_browser_attachments
            .get(browser_id)
            .ok_or("profile_duplicate_borrowed_anchor_missing")?;
        attachment
            .request
            .validate()
            .map_err(|_| "profile_duplicate_borrowed_anchor_invalid")?;
        let source_directory = fs::canonicalize(&source.user_data_dir)
            .map_err(|_| "profile_duplicate_source_unavailable")?;
        if operation_id.trim().is_empty()
            || browser.profile_id != source.id
            || attachment.profile_id != source.id
            || browser.pid != attachment.request.process.pid
            || browser.cdp_endpoint != attachment.request.cdp_endpoint
            || browser.desktop.as_ref() != Some(&attachment.desktop)
            || attachment.desktop.generation != source.generation
            || fs::canonicalize(&attachment.request.profile_path)
                .ok()
                .as_ref()
                != Some(&source_directory)
        {
            return Err("profile_duplicate_borrowed_anchor_changed".into());
        }
        Ok(Self {
            operation_id: operation_id.into(),
            source_profile_id: source.id.clone(),
            source_generation: source.generation,
            source_directory,
            original: attachment.request.process.clone(),
        })
    }
    fn matches(
        &self,
        source: &SourceProfile,
        evidence: &BackendGracefulCloseEvidence,
        state: &BrowserSessionState,
    ) -> bool {
        self.operation_id == evidence.owner_operation_id
            && self.source_profile_id == source.id
            && self.source_generation == source.generation
            && self.original == evidence.original
            && fs::canonicalize(&source.user_data_dir).ok().as_ref() == Some(&self.source_directory)
            && state.profile_operation_ids.get(&source.id) == Some(&self.operation_id)
    }
}
#[derive(Clone, Copy)]
pub struct DestinationLaunchConfiguration<'a> {
    pub executable: &'a str,
    pub mode: Option<&'a str>,
}
pub struct SourceCustody {
    connection: Connection,
    source: SourceProfile,
    catalog: Value,
    active: bool,
}
impl SourceCustody {
    /// The real close adapter must qualify the evidence before calling. The
    /// observer must use fresh kernel evidence, not Service inventory. Holding
    /// this writer transaction excludes canonical launch-intent claims, which
    /// also acquire BEGIN IMMEDIATE on this exact runtime database.
    pub fn acquire(
        database: &Path,
        source: &SourceProfile,
        evidence: &BackendGracefulCloseEvidence,
        observe_original: impl FnMut(u32) -> OriginalProcessState,
    ) -> Result<Self, String> {
        Self::acquire_qualified(database, source, evidence, None, observe_original)
    }
    /// A confirmed workstation-owner close may retire an app-provided original.
    /// Retain its exact pre-close provenance instead of inventing a managed launch.
    pub fn acquire_with_borrowed_anchor(
        database: &Path,
        source: &SourceProfile,
        evidence: &BackendGracefulCloseEvidence,
        anchor: &BorrowedSourceAnchor,
        observe_original: impl FnMut(u32) -> OriginalProcessState,
    ) -> Result<Self, String> {
        Self::acquire_qualified(database, source, evidence, Some(anchor), observe_original)
    }
    fn acquire_qualified(
        database: &Path,
        source: &SourceProfile,
        evidence: &BackendGracefulCloseEvidence,
        anchor: Option<&BorrowedSourceAnchor>,
        mut observe_original: impl FnMut(u32) -> OriginalProcessState,
    ) -> Result<Self, String> {
        if evidence.owner_operation_id.trim().is_empty()
            || evidence.source_profile_id != source.id
            || evidence.source_generation != source.generation
            || evidence.original.pid == 0
            || evidence.original.start_token.is_empty()
            || evidence
                .original
                .executable_path
                .as_deref()
                .is_none_or(str::is_empty)
            || !evidence.polite_close_attempted
            || !evidence.polite_close_succeeded
            || evidence.polite_close_failed
            || !evidence.exact_process_exited
            || !evidence.profile_lock_released
            || evidence.force_kill_attempted
        {
            return Err(
                "profile_duplicate_clean_close_evidence_required:observe a graceful close".into(),
            );
        }
        let connection = Connection::open_with_flags(
            database,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| "profile_duplicate_runtime_unavailable")?;
        connection
            .busy_timeout(std::time::Duration::ZERO)
            .map_err(|_| "profile_duplicate_runtime_unavailable")?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| "profile_duplicate_runtime_unavailable")?;
        if !(2..=4).contains(&version) {
            return Err("profile_duplicate_runtime_schema_unsupported".into());
        }
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| "profile_duplicate_source_busy:refresh and retry")?;
        if observe_original(evidence.original.pid) != OriginalProcessState::Absent {
            return Err("profile_duplicate_original_exit_readback_required".into());
        }
        let state: BrowserSessionState = load_document(
            &connection,
            "browser_session_state",
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if state
            .browsers
            .values()
            .any(|browser| browser.profile_id == source.id)
            || state
                .sessions
                .values()
                .any(|task| task.profile_id == source.id)
        {
            return Err("profile_duplicate_source_state_requires_reconciliation".into());
        }
        let catalog: Value = load_document(
            &connection,
            "browser_profile_catalog",
            "agent-browser.browser-profile-catalog.v1",
        )?;
        let profile = catalog
            .pointer(&format!(
                "/profiles/{}",
                source.id.replace('~', "~0").replace('/', "~1")
            ))
            .ok_or("profile_duplicate_source_profile_missing")?;
        let canonical = fs::canonicalize(&source.user_data_dir)
            .map_err(|_| "profile_duplicate_source_unavailable")?;
        if profile["id"] != source.id
            || profile["kind"] != "named"
            || profile["userDataDir"]
                .as_str()
                .and_then(|path| fs::canonicalize(path).ok())
                .as_ref()
                != Some(&canonical)
        {
            return Err("profile_duplicate_source_binding_changed".into());
        }
        let borrowed_qualified =
            anchor.is_some_and(|anchor| anchor.matches(source, evidence, &state));
        if anchor.is_some() && !borrowed_qualified {
            return Err("profile_duplicate_borrowed_anchor_changed".into());
        }
        let has_ledger: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM state_documents WHERE kind='browser_launch_custody')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| "profile_duplicate_runtime_document_unavailable")?;
        let ledger: LaunchLedger = if borrowed_qualified && !has_ledger {
            LaunchLedger::default()
        } else {
            load_document(
                &connection,
                "browser_launch_custody",
                "agent-browser.browser-launch-custody.v1",
            )?
        };
        let mut original_joined = borrowed_qualified;
        for (id, record) in &ledger.records {
            if record.validate().is_err() || record.intent.intent_id != *id {
                return Err("profile_duplicate_launch_custody_invalid".into());
            }
            if record.intent.profile_id != source.id {
                continue;
            }
            if !record.published && !ledger.reconciled_absent.contains(id) {
                return Err(
                    "profile_duplicate_source_launch_unresolved:reconcile current launch".into(),
                );
            }
            original_joined |= record.published
                && record.observed_process_identity.as_ref() == Some(&evidence.original);
        }
        if !original_joined {
            return Err("profile_duplicate_original_identity_unavailable".into());
        }
        Ok(Self {
            connection,
            source: source.clone(),
            catalog,
            active: true,
        })
    }
    /// Publish only the independent copied profile and lineage using the held
    /// connection. Commit releases acquisition fencing before normal launch.
    /// The caller must not open a second runtime writer while this guard lives.
    pub fn publish_and_release(
        &mut self,
        profile: &DuplicateProfile,
        configuration: Option<DestinationLaunchConfiguration<'_>>,
    ) -> Result<(), String> {
        if !self.active {
            return Err("profile_duplicate_source_custody_released".into());
        }
        if profile.source_profile_id != self.source.id
            || profile.browser_build != self.source.browser_build
            || profile.destination.id == self.source.id
            || profile.operation_id.trim().is_empty()
        {
            return Err("profile_duplicate_publication_intent_changed".into());
        }
        let source = fs::canonicalize(&self.source.user_data_dir)
            .map_err(|_| "profile_duplicate_source_unavailable")?;
        let destination = fs::canonicalize(&profile.destination.user_data_dir)
            .map_err(|_| "profile_duplicate_destination_unavailable")?;
        if source.starts_with(&destination) || destination.starts_with(&source) {
            return Err("profile_duplicate_paths_overlap".into());
        }
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS browser_profile_duplication_lineage (profile_id TEXT PRIMARY KEY, source_profile_id TEXT NOT NULL, operation_id TEXT NOT NULL, browser_build TEXT NOT NULL);").map_err(|_| "profile_duplicate_lineage_unavailable")?;
        // Mode metadata is profile behavior, not process or task ownership.
        // Retain a configured no-CDP login posture on the independent profile.
        let manual_login_url = self
            .catalog
            .get("profiles")
            .and_then(|profiles| profiles.get(&self.source.id))
            .and_then(|profile| profile.get("manualLoginUrl"))
            .cloned();
        let profiles = self
            .catalog
            .get_mut("profiles")
            .and_then(Value::as_object_mut)
            .ok_or("profile_duplicate_catalog_invalid")?;
        let lineage:Option<(String,String,String)>=self.connection.query_row("SELECT source_profile_id,operation_id,browser_build FROM browser_profile_duplication_lineage WHERE profile_id=?1",[&profile.destination.id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|_| "profile_duplicate_lineage_unavailable")?;
        if let Some(existing) = profiles.get(&profile.destination.id) {
            if existing["id"] != profile.destination.id
                || existing["userDataDir"]
                    .as_str()
                    .and_then(|path| fs::canonicalize(path).ok())
                    .as_ref()
                    != Some(&destination)
                || lineage
                    != Some((
                        profile.source_profile_id.clone(),
                        profile.operation_id.clone(),
                        profile.browser_build.clone(),
                    ))
            {
                return Err("profile_duplicate_catalog_destination_exists".into());
            }
        } else {
            if lineage.is_some() {
                return Err("profile_duplicate_lineage_conflict".into());
            }
            let mut entry = json!({"id":profile.destination.id,"name":profile.destination.name,"userDataDir":profile.destination.user_data_dir,"kind":"named"});
            if let Some(url) = manual_login_url {
                entry["manualLoginUrl"] = url;
            }
            if let Some(configuration) = configuration {
                if !Path::new(configuration.executable).is_absolute() {
                    return Err("browser_build_absolute_path_required".into());
                }
                if let Some(mode) = configuration.mode {
                    match mode {
                        "cdp" => {
                            entry
                                .as_object_mut()
                                .ok_or("profile_duplicate_catalog_invalid")?
                                .remove("manualLoginUrl");
                        }
                        "cdp_free" => entry["manualLoginUrl"] = json!("about:blank"),
                        _ => return Err("browser_build_mode_invalid".into()),
                    }
                }
                let mut state: BrowserSessionState = load_document(
                    &self.connection,
                    "browser_session_state",
                    BROWSER_SESSION_STATE_SCHEMA_V1,
                )?;
                if state
                    .profile_executables
                    .contains_key(&profile.destination.id)
                    || state
                        .browsers
                        .values()
                        .any(|browser| browser.profile_id == profile.destination.id)
                {
                    return Err("browser_build_destination_configuration_conflict".into());
                }
                state.profile_executables.insert(
                    profile.destination.id.clone(),
                    configuration.executable.into(),
                );
                self.connection.execute("UPDATE state_documents SET json=?1 WHERE kind='browser_session_state' AND schema_version=?2",params![serde_json::to_string(&state).map_err(|_|"profile_duplicate_state_encode_failed")?,BROWSER_SESSION_STATE_SCHEMA_V1]).map_err(|_|"profile_duplicate_state_write_failed")?;
            }
            profiles.insert(profile.destination.id.clone(), entry);
            self.connection.execute("UPDATE state_documents SET json=?1 WHERE kind='browser_profile_catalog' AND schema_version='agent-browser.browser-profile-catalog.v1'",[self.catalog.to_string()]).map_err(|_| "profile_duplicate_catalog_write_failed")?;
            self.connection
                .execute(
                    "INSERT INTO browser_profile_duplication_lineage VALUES(?1,?2,?3,?4)",
                    params![
                        profile.destination.id,
                        profile.source_profile_id,
                        profile.operation_id,
                        profile.browser_build
                    ],
                )
                .map_err(|_| "profile_duplicate_lineage_write_failed")?;
        }
        self.connection
            .execute_batch("COMMIT")
            .map_err(|_| "profile_duplicate_catalog_commit_readback_required")?;
        self.active = false;
        Ok(())
    }
}
impl Drop for SourceCustody {
    fn drop(&mut self) {
        if self.active {
            let _ = self.connection.execute_batch("ROLLBACK");
        }
    }
}
fn load_document<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    kind: &str,
    schema: &str,
) -> Result<T, String> {
    let (actual, value): (String, String) = connection
        .query_row(
            "SELECT schema_version,json FROM state_documents WHERE kind=?1",
            [kind],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "profile_duplicate_runtime_document_unavailable")?;
    if actual != schema {
        return Err("profile_duplicate_runtime_document_schema_unsupported".into());
    }
    serde_json::from_str(&value).map_err(|_| "profile_duplicate_runtime_document_invalid".into())
}

/// A copied profile is independent of its source. Recovery of catalog
/// publication must not close or reacquire a source that has since reopened.
/// The operation must be read from the private durable duplication journal.
pub fn publish_recovered_copy(
    database: &Path,
    operation: &super::profile_duplicate_copy::Operation,
    configuration: Option<DestinationLaunchConfiguration<'_>>,
) -> Result<(), String> {
    use super::profile_duplicate_copy::Phase;
    if operation.phase != Phase::Copied {
        return Err("profile_duplicate_copy_phase_required".into());
    }
    let profile = operation
        .duplicate
        .as_ref()
        .ok_or("profile_duplicate_copy_record_missing")?;
    if profile.operation_id != operation.request.operation_id
        || profile.source_profile_id != operation.request.source.id
        || profile.destination != operation.request.destination
        || profile.browser_build != operation.request.source.browser_build
    {
        return Err("profile_duplicate_copy_record_conflict".into());
    }
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "profile_duplicate_runtime_unavailable")?;
    connection
        .busy_timeout(std::time::Duration::ZERO)
        .map_err(|_| "profile_duplicate_runtime_unavailable")?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| "profile_duplicate_runtime_unavailable")?;
    if !(2..=4).contains(&version) {
        return Err("profile_duplicate_runtime_schema_unsupported".into());
    }
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| "profile_duplicate_catalog_busy:read operation and retry")?;
    let catalog: Value = load_document(
        &connection,
        "browser_profile_catalog",
        "agent-browser.browser-profile-catalog.v1",
    )?;
    let mut publication = SourceCustody {
        connection,
        source: operation.request.source.clone(),
        catalog,
        active: true,
    };
    publication.publish_and_release(profile, configuration)
}
