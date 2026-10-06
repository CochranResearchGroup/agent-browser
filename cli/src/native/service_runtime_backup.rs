//! Operator-facing Browser Runtime backup actions.
//!
//! These actions expose verification and creation for the P220-owned SQLite
//! database. They do not restore data or mutate unrelated Service State.

use serde_json::{json, Value};

use super::browser_session_store::BrowserSessionSqliteStore;

pub(crate) async fn handle_service_runtime_backup_status() -> Result<Value, String> {
    let store = BrowserSessionSqliteStore::default_sqlite()?;
    Ok(json!({ "backup": store.backup_status()? }))
}

pub(crate) async fn handle_service_runtime_backup_create() -> Result<Value, String> {
    let store = BrowserSessionSqliteStore::default_sqlite()?;
    let backup = store.create_verified_backup()?;
    let status = store.backup_status()?;
    Ok(json!({ "backup": backup, "status": status }))
}
