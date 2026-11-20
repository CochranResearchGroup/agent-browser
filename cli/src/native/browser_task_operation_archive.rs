//! Durable original-operation archive and live capacity policy in the runtime database.
//! All publications share the session writer transaction, so an archive failure
//! leaves both the original history and dispatch admission unchanged.
use super::{
    load_optional_document, save_document, BrowserSessionSqliteStore, SESSION_STATE_DOCUMENT,
};
use agent_browser_service_model::{
    BrowserSessionState, BrowserTaskOperation, BrowserTaskOperationState,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const POLICY_DOCUMENT: &str = "task_operation_capacity";
const POLICY_SCHEMA: &str = "agent-browser.task-operation-capacity.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapacityPolicy {
    pub(crate) initial_capacity: usize,
    pub(crate) extension_increment: usize,
    pub(crate) maximum_capacity: usize,
    /// Per episode; live updates do not reset unresolved task input.
    #[serde(default = "default_login_steps")]
    pub(crate) maximum_login_steps: usize,
    /// Admission limit only; already completed artifact metadata is always retained.
    #[serde(default = "default_artifact_references")]
    pub(crate) maximum_artifact_references: usize,
}
fn default_artifact_references() -> usize {
    256
}
fn default_login_steps() -> usize {
    32
}
impl Default for CapacityPolicy {
    fn default() -> Self {
        // The active working set starts at its historical size. Sixteen bounded
        // increments allow unresolved work to grow without eagerly allocating it.
        Self {
            initial_capacity: 256,
            extension_increment: 256,
            maximum_capacity: 4096,
            maximum_login_steps: default_login_steps(),
            maximum_artifact_references: default_artifact_references(),
        }
    }
}
impl CapacityPolicy {
    fn validate(&self) -> Result<(), String> {
        if self.initial_capacity == 0
            || self.extension_increment == 0
            || self.maximum_capacity < self.initial_capacity
            || self.maximum_login_steps == 0
            || self.maximum_artifact_references == 0
        {
            return Err("browser_task_operation_capacity_policy_invalid: use positive initialCapacity and extensionIncrement with maximumCapacity at least initialCapacity; maximumLoginSteps and maximumArtifactReferences must be positive".into());
        }
        Ok(())
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct CapacityConfiguration {
    defaults: CapacityPolicy,
    task_overrides: BTreeMap<String, CapacityPolicy>,
    capacities: BTreeMap<String, usize>,
}
impl CapacityConfiguration {
    fn effective(&self, task: &str) -> (&CapacityPolicy, &'static str) {
        self.task_overrides
            .get(task)
            .map(|p| (p, "task_override"))
            .unwrap_or((&self.defaults, "system_default"))
    }
    fn capacity(&self, task: &str, policy: &CapacityPolicy) -> usize {
        self.capacities
            .get(task)
            .copied()
            .unwrap_or(policy.initial_capacity)
            .max(policy.initial_capacity)
            .min(policy.maximum_capacity)
    }
}
fn configuration(connection: &Connection) -> Result<CapacityConfiguration, String> {
    load_optional_document(connection, POLICY_DOCUMENT, POLICY_SCHEMA)
}
fn has_archive(connection: &Connection) -> Result<bool, String> {
    connection.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name='task_operation_archive'", [], |_| Ok(())).optional().map(|v| v.is_some()).map_err(|_| "browser_task_operation_archive_read_failed: retry capacity inspection; original history is retained".into())
}
fn initialize(connection: &Connection) -> Result<(), String> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS task_operation_archive(task_id TEXT NOT NULL, request_id TEXT NOT NULL, json TEXT NOT NULL, PRIMARY KEY(task_id,request_id));").map_err(|_| "browser_task_operation_archive_unavailable: inspect runtime storage and retry the same request; no new effect was dispatched")?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| "browser_task_operation_archive_unavailable")?;
    if version < 6 {
        connection
            .pragma_update(None, "user_version", 6)
            .map_err(|_| "browser_task_operation_archive_unavailable")?;
    }
    Ok(())
}
/// Return an immutable original without loading unrelated archived history.
pub(crate) fn archived(
    connection: &Connection,
    task: &str,
    request: &str,
) -> Result<Option<BrowserTaskOperation>, String> {
    if !has_archive(connection)? {
        return Ok(None);
    }
    let encoded: Option<String> = connection.query_row("SELECT json FROM task_operation_archive WHERE task_id=?1 AND request_id=?2", params![task,request], |r|r.get(0)).optional().map_err(|_| "browser_task_operation_archive_read_failed: inspect runtime storage; do not replay original requests")?;
    encoded.map(|s| serde_json::from_str::<BrowserTaskOperation>(&s).map_err(|_| "browser_task_operation_archive_record_invalid: restore runtime storage from its supported backup; do not replay".to_string())).transpose()?.map(|row| {
        if row.task_id != task || row.request_id != request { Err("browser_task_operation_archive_identity_conflict".into()) } else { Ok(row) }
    }).transpose()
}
pub(super) fn history(
    connection: &Connection,
    task: &str,
) -> Result<BTreeMap<String, BrowserTaskOperation>, String> {
    let mut records = BTreeMap::new();
    if !has_archive(connection)? {
        return Ok(records);
    }
    let mut statement = connection
        .prepare("SELECT request_id,json FROM task_operation_archive WHERE task_id=?1")
        .map_err(|_| "browser_task_operation_archive_read_failed")?;
    let rows = statement
        .query_map([task], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| "browser_task_operation_archive_read_failed")?;
    for row in rows {
        let (request, encoded) = row.map_err(|_| "browser_task_operation_archive_read_failed")?;
        let record: BrowserTaskOperation = serde_json::from_str(&encoded)
            .map_err(|_| "browser_task_operation_archive_record_invalid")?;
        if record.task_id != task || record.request_id != request {
            return Err("browser_task_operation_archive_identity_conflict".into());
        }
        records.insert(request, record);
    }
    Ok(records)
}
pub(super) fn has_history(connection: &Connection) -> Result<bool, String> {
    if !has_archive(connection)? {
        return Ok(false);
    }
    connection
        .query_row("SELECT 1 FROM task_operation_archive LIMIT 1", [], |_| {
            Ok(())
        })
        .optional()
        .map(|v| v.is_some())
        .map_err(|_| "browser_task_operation_archive_read_failed".into())
}
/// Called under the immediate session transaction before admitting a new effect.
pub(crate) fn make_room(
    connection: &Connection,
    state: &mut BrowserSessionState,
    task: &str,
) -> Result<(), String> {
    let mut config = configuration(connection)?;
    let (policy, _) = config.effective(task);
    policy.validate()?;
    let mut capacity = config.capacity(task, policy);
    let rows = state.task_operations.entry(task.into()).or_default();
    if rows.len() < capacity {
        return Ok(());
    }
    let completed: Vec<String> = rows
        .iter()
        .filter(|(_, row)| row.state.is_terminal() && row.completed_at_ms.is_some())
        .map(|(id, _)| id.clone())
        .collect();
    if !completed.is_empty() {
        initialize(connection)?;
        for request in completed {
            let row = &rows[&request];
            let encoded = serde_json::to_string(row)
                .map_err(|_| "browser_task_operation_archive_record_invalid")?;
            connection.execute("INSERT INTO task_operation_archive(task_id,request_id,json) VALUES(?1,?2,?3) ON CONFLICT(task_id,request_id) DO NOTHING", params![task,request,encoded]).map_err(|_| "browser_task_operation_archive_write_failed: inspect runtime storage and retry the same request; no new effect was dispatched")?;
            if archived(connection, task, &request)?.as_ref() != Some(row) {
                return Err("browser_task_operation_archive_identity_conflict: inspect the original operation; no replay".into());
            }
            rows.remove(&request);
        }
    }
    let (policy, _) = config.effective(task);
    while rows.len() >= capacity && capacity < policy.maximum_capacity {
        capacity = capacity
            .saturating_add(policy.extension_increment)
            .min(policy.maximum_capacity);
    }
    if rows.len() >= capacity {
        return Err(format!("browser_task_operation_capacity_reached: activeCount={};capacity={capacity};maximumCapacity={}; pending and uncertain originals retained; call browser_command action=task_operation params={{\"operation\":\"capacity\"}} for this task, reconcile unresolved originals or raise its maximumCapacity with the same capacity operation and policy; no new effect dispatched",rows.len(),policy.maximum_capacity));
    }
    config.capacities.insert(task.into(), capacity);
    save_document(connection, POLICY_DOCUMENT, POLICY_SCHEMA, &config)
}
/// Prevent an older resident host snapshot from reintroducing archived originals.
pub(super) fn strip_archived_json(
    connection: &Connection,
    encoded: String,
) -> Result<String, String> {
    if !has_archive(connection)? {
        return Ok(encoded);
    }
    // Usually the publisher already holds a bounded active snapshot. Avoid
    // another Rust parse/serialization unless a stale host resurrected history.
    let overlaps: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM json_each(?1,'$.taskOperations') tasks JOIN json_each(tasks.value) operations JOIN task_operation_archive archived ON archived.task_id=tasks.key AND archived.request_id=operations.key)",
        [&encoded], |row| row.get(0),
    ).map_err(|_| "browser_task_operation_archive_read_failed")?;
    if !overlaps {
        return Ok(encoded);
    }
    let mut value: Value =
        serde_json::from_str(&encoded).map_err(|_| "browser_task_operation_document_invalid")?;
    if let Some(tasks) = value
        .get_mut("taskOperations")
        .and_then(Value::as_object_mut)
    {
        for (task, records) in tasks {
            if let Some(records) = records.as_object_mut() {
                let requests = serde_json::to_string(&records.keys().collect::<Vec<_>>())
                    .map_err(|_| "browser_task_operation_document_invalid")?;
                let mut statement = connection.prepare("SELECT request_id FROM task_operation_archive WHERE task_id=?1 AND request_id IN (SELECT value FROM json_each(?2))").map_err(|_| "browser_task_operation_archive_read_failed")?;
                let archived = statement
                    .query_map(params![task, requests], |row| row.get::<_, String>(0))
                    .map_err(|_| "browser_task_operation_archive_read_failed")?;
                for request in archived {
                    records.remove(
                        &request.map_err(|_| "browser_task_operation_archive_read_failed")?,
                    );
                }
            }
        }
    }
    serde_json::to_string(&value).map_err(|_| "browser_task_operation_document_invalid".into())
}
impl BrowserSessionSqliteStore {
    pub(crate) fn artifact_reference_limit(&self, task: &str) -> Result<usize, String> {
        let config = configuration(&self.connection)?;
        let (policy, _) = config.effective(task);
        policy.validate()?;
        Ok(policy.maximum_artifact_references)
    }
    pub(crate) fn login_step_limit(&self, task: &str) -> Result<usize, String> {
        let config = configuration(&self.connection)?;
        let (policy, _) = config.effective(task);
        policy.validate()?;
        Ok(policy.maximum_login_steps)
    }
    pub(crate) fn archived_task_operation(
        &self,
        task: &str,
        request: &str,
    ) -> Result<Option<BrowserTaskOperation>, String> {
        archived(&self.connection, task, request)
    }
    pub(crate) fn archived_task_operation_history(
        &self,
        task: &str,
    ) -> Result<BTreeMap<String, BrowserTaskOperation>, String> {
        history(&self.connection, task)
    }
    pub(crate) fn has_archived_task_operations(&self) -> Result<bool, String> {
        has_history(&self.connection)
    }
    /// Inert inspection and live policy update; no daemon, browser or request replay.
    pub(crate) fn task_operation_capacity(
        &mut self,
        task: &str,
        scope: &str,
        policy: Option<CapacityPolicy>,
    ) -> Result<Value, String> {
        if !matches!(scope, "task" | "defaults") {
            return Err(
                "browser_task_operation_capacity_scope_invalid: choose task or defaults".into(),
            );
        }
        if let Some(policy) = &policy {
            policy.validate()?;
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "browser_task_operation_capacity_store_busy: retry capacity inspection")?;
        let state: BrowserSessionState = super::load_document(
            &transaction,
            SESSION_STATE_DOCUMENT,
            agent_browser_service_model::BROWSER_SESSION_STATE_SCHEMA_V1,
        )?;
        if !state.sessions.contains_key(task) {
            return Err("browser_task_not_found: inspect retained task inventory".into());
        }
        let mut config = configuration(&transaction)?;
        if let Some(policy) = policy {
            if scope == "defaults" {
                config.defaults = policy;
            } else {
                config.task_overrides.insert(task.into(), policy);
            }
            save_document(&transaction, POLICY_DOCUMENT, POLICY_SCHEMA, &config)?;
        }
        let (policy, source) = config.effective(task);
        let capacity = config.capacity(task, policy);
        let active = state.task_operations.get(task).map_or(0, BTreeMap::len);
        let archived = if has_archive(&transaction)? {
            let count = transaction
                .query_row(
                    "SELECT COUNT(*) FROM task_operation_archive WHERE task_id=?1",
                    [task],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(|_| "browser_task_operation_archive_read_failed")?;
            u64::try_from(count).map_err(|_| "browser_task_operation_archive_count_invalid")?
        } else {
            0
        };
        let unresolved = state
            .task_operations
            .get(task)
            .into_iter()
            .flat_map(|r| r.values())
            .filter(|r| !r.state.is_terminal())
            .count();
        let artifact_count = state.task_artifact_references.get(task).map_or(0, Vec::len);
        let result = json!({"artifactReferenceCount":artifact_count,"artifactReferenceLimit":policy.maximum_artifact_references,"schemaVersion":POLICY_SCHEMA,"taskId":task,"policy":policy,"policySource":source,"capacity":capacity,"activeCount":active,"archivedCount":archived,"unresolvedCount":unresolved,"automaticArchival":true,"automaticExtension":true,"requiresRestart":false,"nextAction":{"tool":"browser_command","arguments":{"taskId":task,"action":"task_operation","params":{"operation":"capacity"}}}});
        transaction.commit().map_err(|_| {
            "browser_task_operation_capacity_commit_failed: inspect effective policy before retry"
        })?;
        Ok(result)
    }
}

#[cfg(test)]
mod artifact_capacity_tests {
    use super::*;

    #[test]
    fn retained_task_artifact_limit_updates_live_without_dropping_references() {
        let root = std::env::temp_dir().join(format!("artifact-capacity-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let (mut store, _) =
            BrowserSessionSqliteStore::open_or_migrate(&root, &root.join("legacy.json")).unwrap();
        let mut state = BrowserSessionState::default();
        state.sessions.insert("task".into(), serde_json::from_value(json!({"id":"task","name":"lane","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0})).unwrap());
        state.task_artifact_references.insert(
            "task".into(),
            vec![agent_browser_service_model::BrowserTaskArtifactReference {
                artifact_id: "retained-original".into(),
                kind: "screenshot".into(),
                handle: "reports".into(),
                observed_at_ms: 1,
            }],
        );
        store.save_session_state(&state).unwrap();
        assert_eq!(store.artifact_reference_limit("task").unwrap(), 256);
        let policy: CapacityPolicy = serde_json::from_value(json!({"initialCapacity":256,"extensionIncrement":256,"maximumCapacity":4096,"maximumArtifactReferences":512})).unwrap();
        let result = store
            .task_operation_capacity("task", "task", Some(policy))
            .unwrap();
        assert_eq!(result["artifactReferenceLimit"], 512);
        drop(store);
        let store = BrowserSessionSqliteStore::open(&root).unwrap();
        assert_eq!(store.artifact_reference_limit("task").unwrap(), 512);
        assert_eq!(store.load_session_state().unwrap(), state);
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
