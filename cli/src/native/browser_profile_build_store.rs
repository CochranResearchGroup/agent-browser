//! Atomic offline build intent and idempotent receipts in the runtime owner DB.
use super::*;
use serde_json::{json, Value};

pub(crate) struct ProfileBuildSelection<'a> {
    pub operation_id: &'a str,
    pub profile: &'a agent_browser_service_model::BrowserProfileCatalogEntry,
    pub expected_pin: Option<&'a str>,
    pub intent: &'a str,
    pub executable: &'a str,
    pub mode: &'a str,
    pub public_build: Value,
}
fn retained(connection: &Connection, id: &str, intent: &str) -> Result<Option<Value>, String> {
    let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='browser_profile_build_selections')", [], |r| r.get(0)).map_err(|_|"browser_build_receipt_unavailable")?;
    if !exists {
        return Ok(None);
    }
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT intent,receipt FROM browser_profile_build_selections WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|_| "browser_build_receipt_unavailable")?;
    row.map(|(previous, receipt)| {
        if previous != intent {
            return Err("browser_build_operation_id_conflict".into());
        }
        serde_json::from_str(&receipt).map_err(|_| "browser_build_receipt_invalid".into())
    })
    .transpose()
}
impl BrowserSessionSqliteStore {
    pub(crate) fn profile_build_selection_receipt(
        &self,
        id: &str,
        intent: &str,
    ) -> Result<Option<Value>, String> {
        retained(&self.connection, id, intent)
    }
    pub(crate) fn current_profile_build_selection(
        &self,
        profile_id: &str,
        build_id: &str,
        mode: &str,
    ) -> Result<Option<String>, String> {
        let exists:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='browser_profile_build_selections')",[],|row|row.get(0)).map_err(|_|"browser_build_receipt_unavailable")?;
        if !exists {
            return Ok(None);
        }
        self.connection.query_row("SELECT id FROM browser_profile_build_selections WHERE json_extract(receipt,'$.profileId')=?1 AND json_extract(receipt,'$.selectedBuild.id')=?2 AND json_extract(receipt,'$.requestedMode')=?3 ORDER BY rowid DESC LIMIT 1",params![profile_id,build_id,mode],|row|row.get(0)).optional().map_err(|_|"browser_build_receipt_unavailable".into())
    }
    pub(crate) fn profile_build_selection_by_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> Result<Option<Value>, String> {
        let exists:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='browser_profile_build_selections')",[],|row|row.get(0)).map_err(|_|"browser_build_receipt_unavailable")?;
        if !exists {
            return Ok(None);
        }
        let raw: Option<String> = self
            .connection
            .query_row(
                "SELECT receipt FROM browser_profile_build_selections WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "browser_build_receipt_unavailable")?;
        raw.map(|raw| {
            let receipt: Value =
                serde_json::from_str(&raw).map_err(|_| "browser_build_receipt_invalid")?;
            if receipt["profileId"] != profile_id {
                return Err("browser_build_receipt_profile_conflict".into());
            }
            Ok(receipt)
        })
        .transpose()
    }
    /// Pin only future explicit launches. Profile bytes and physical browsers are untouched.
    /// The pin, independent mode intent, and replay receipt commit together.
    pub(crate) fn select_profile_build(
        &mut self,
        selection: ProfileBuildSelection<'_>,
        verify: impl FnOnce() -> Result<(), String>,
    ) -> Result<Value, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "browser_build_begin_failed")?;
        if let Some(receipt) = retained(&transaction, selection.operation_id, selection.intent)? {
            return Ok(receipt);
        }
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
        let id = &selection.profile.id;
        let mut profile = catalog
            .profiles
            .get(id)
            .cloned()
            .or_else(|| state.disposable_profiles.get(id).map(|p| p.profile.clone()))
            .ok_or("browser_build_profile_missing")?;
        if &profile != selection.profile
            || state.profile_executables.get(id).map(String::as_str) != selection.expected_pin
        {
            return Err("browser_build_profile_changed:inspect compatibility again".into());
        }
        if state.browsers.values().any(|b| &b.profile_id == id)
            || state.profile_operation_ids.contains_key(id)
        {
            return Err(
                "browser_build_live_profile:use original-profile conversion or duplicate-and-test"
                    .into(),
            );
        }
        super::super::browser_idle_retirement_store::reject_profile_metadata_mutation(
            &transaction,
            id,
        )
        .map_err(|_| "browser_build_retirement_pending")?;
        super::super::browser_launch_custody_store::reject_profile_metadata_mutation(
            &transaction,
            id,
        )
        .map_err(|_| "browser_build_launch_pending")?;
        // Revalidate the exact reviewed bytes inside the serialized owner transaction.
        verify()?;
        state
            .profile_executables
            .insert(id.clone(), selection.executable.into());
        profile.manual_login_url = (selection.mode == "cdp_free").then(|| "about:blank".into());
        if let Some(row) = state.disposable_profiles.get_mut(id) {
            row.profile = profile.clone();
        }
        if catalog.profiles.contains_key(id) {
            catalog.profiles.insert(id.clone(), profile);
        }
        let receipt = json!({"schemaVersion":"agent-browser.build-selection.v1","operationId":selection.operation_id,"profileId":id,"phase":"selected_for_next_launch","profileDirectoryExisted":Path::new(&selection.profile.user_data_dir).is_dir(),"requestedMode":selection.mode,"observedMode":null,"selectedBuild":selection.public_build,"authenticationContinuity":"unverified","profileDataReadable":"unverified","warning":"Build selection applies to a future explicit launch. It may migrate profile data or lose login. Selecting the previous executable does not restore profile bytes."});
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS browser_profile_build_selections(id TEXT PRIMARY KEY,intent TEXT NOT NULL,receipt TEXT NOT NULL)").map_err(|_|"browser_build_receipt_unavailable")?;
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
            .execute(
                "INSERT INTO browser_profile_build_selections VALUES(?1,?2,?3)",
                params![
                    selection.operation_id,
                    selection.intent,
                    serde_json::to_string(&receipt)
                        .map_err(|_| "browser_build_receipt_encode_failed")?
                ],
            )
            .map_err(|_| "browser_build_receipt_write_failed")?;
        transaction
            .commit()
            .map_err(|_| "browser_build_commit_failed")?;
        Ok(receipt)
    }
}

impl BrowserSessionSqliteStore {
    /// Only positive copy publication admits an existing-profile broker fallback.
    pub(crate) fn published_copy_profile(
        &self,
        profile_id: &str,
    ) -> Result<Option<agent_browser_service_model::BrowserProfileCatalogEntry>, String> {
        let table:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='browser_profile_duplication_lineage')",[],|row|row.get(0)).map_err(|_|"browser_build_copy_lineage_unavailable")?;
        if !table {
            return Ok(None);
        }
        let lineage:Option<(String,String,String)>=self.connection.query_row("SELECT source_profile_id,operation_id,browser_build FROM browser_profile_duplication_lineage WHERE profile_id=?1",[profile_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|_|"browser_build_copy_lineage_unavailable")?;
        let Some((source, operation, build)) = lineage else {
            return Ok(None);
        };
        if source == profile_id
            || [source, operation, build]
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err("browser_build_copy_lineage_invalid".into());
        }
        let catalog = self.load_profile_catalog()?.catalog;
        let profile = catalog
            .profiles
            .get(profile_id)
            .filter(|profile| {
                profile.kind == agent_browser_service_model::BrowserProfileKind::Named
            })
            .cloned()
            .ok_or("browser_build_copy_catalog_missing:inspect the original copy operation")?;
        let directory = Path::new(&profile.user_data_dir);
        if !directory.is_dir() || directory.canonicalize().ok().as_deref() != Some(directory) {
            return Err("browser_build_copy_directory_unavailable:restore the preserved directory or choose a new profile".into());
        }
        Ok(Some(profile))
    }
}
