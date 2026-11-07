//! Reconcile publication snapshots, never browser effects. Current resource
//! identities win overlapping edits; private append-only receipts retain the
//! losing observations instead of turning them into permission or replay gates.

use agent_browser_service_model::BrowserSessionState;
use rusqlite::Connection;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn choose(
    base: Option<&Value>,
    local: Option<&Value>,
    current: Option<&Value>,
    path: &str,
    depth: usize,
    conflicts: &mut Vec<Value>,
) -> Option<Value> {
    if local == base || local == current {
        return current.cloned();
    }
    if current == base {
        return local.cloned();
    }
    if let (Some(Value::Object(local)), Some(Value::Object(current))) = (local, current) {
        let base = base.and_then(Value::as_object);
        // Do not splice fields from different physical resources or task routes.
        // Collection keys are caller-selected IDs, not identity fields. In
        // particular a request named "pid" must not turn its task's entire
        // operation map into one atomic record and lose another operation.
        let field = path.split('/').nth(1).unwrap_or("");
        let resource_record = (depth == 2
            && !matches!(field, "taskOperations" | "hostPolicies" | "retentionPolicy"))
            || (depth == 3 && field == "taskOperations");
        let identity_changed = resource_record
            && [
                "pid",
                "cdpEndpoint",
                "profileId",
                "browserId",
                "targetId",
                "desktop",
                "tabId",
                "taskId",
                "requestId",
                "processPid",
                "processIdentity",
                "controlEpoch",
                "fingerprint",
            ]
            .iter()
            .any(|key| {
                (local.contains_key(*key) || current.contains_key(*key))
                    && (base.and_then(|base| base.get(*key)) != local.get(*key)
                        || base.and_then(|base| base.get(*key)) != current.get(*key))
                    && local.get(*key) != current.get(*key)
            });
        if !identity_changed {
            let keys = local
                .keys()
                .chain(current.keys())
                .chain(base.into_iter().flat_map(|b| b.keys()))
                .collect::<BTreeSet<_>>();
            let mut result = serde_json::Map::new();
            for key in keys {
                if let Some(value) = choose(
                    base.and_then(|b| b.get(key)),
                    local.get(key),
                    current.get(key),
                    &format!("{path}/{key}"),
                    depth + 1,
                    conflicts,
                ) {
                    result.insert(key.clone(), value);
                }
            }
            return Some(Value::Object(result));
        }
    }
    if let (Some(Value::Array(local)), Some(Value::Array(current))) = (local, current) {
        let base = base.and_then(Value::as_array);
        let mut result = Vec::new();
        // Keep each writer's additions and honor removals from the shared base.
        for item in current.iter().chain(local.iter()) {
            let was_present = base.is_some_and(|base| base.contains(item));
            if (!was_present || (local.contains(item) && current.contains(item)))
                && !result.contains(item)
            {
                result.push(item.clone());
            }
        }
        return Some(Value::Array(result));
    }
    let monotonic = path.ends_with("/nextSessionSequence")
        || path.ends_with("/nextDisposableSequence")
        || path.ends_with("/lastActivityAtMs");
    if let (true, Some(local), Some(current)) = (
        monotonic,
        local.and_then(Value::as_u64),
        current.and_then(Value::as_u64),
    ) {
        return Some(json!(local.max(current)));
    }
    conflicts.push(json!({"path":path,"base":base,"resident":local,"durable":current}));
    current.cloned()
}

pub(crate) fn reconcile(
    base: &BrowserSessionState,
    local: &BrowserSessionState,
    current: &BrowserSessionState,
) -> Result<(BrowserSessionState, Vec<Value>), String> {
    let mut conflicts = Vec::new();
    let encode = |state: &BrowserSessionState| {
        serde_json::to_value(state)
            .map_err(|_| "browser_session_reconciliation_encode_failed".to_string())
    };
    let merged = choose(
        Some(&encode(base)?),
        Some(&encode(local)?),
        Some(&encode(current)?),
        "",
        0,
        &mut conflicts,
    )
    .ok_or("browser_session_reconciliation_missing")?;
    let mut merged: BrowserSessionState =
        serde_json::from_value(merged).map_err(|_| "browser_session_reconciliation_invalid")?;
    merged.retention_policy.validate()?;
    // This is a derived reverse index, not a competing ownership assertion.
    for browser in merged.browsers.values_mut() {
        browser.active_session_ids = merged
            .sessions
            .values()
            .filter(|session| session.browser_id == browser.id)
            .map(|session| session.id.clone())
            .collect();
    }
    Ok((merged, conflicts))
}

/// Called under the same SQLite IMMEDIATE transaction as state publication.
/// These receipts are private runtime evidence, never a public response payload.
pub(crate) fn reconcile_and_record(
    connection: &Connection,
    base: &BrowserSessionState,
    local: &BrowserSessionState,
    current: &BrowserSessionState,
) -> Result<BrowserSessionState, String> {
    if base == current {
        return Ok(local.clone());
    }
    let (merged, conflicts) = reconcile(base, local, current)?;
    if !conflicts.is_empty() {
        let payload = serde_json::to_string(&conflicts)
            .map_err(|_| "browser_session_reconciliation_encode_failed")?;
        let digest = format!("{:x}", Sha256::digest(payload.as_bytes()));
        connection.execute_batch("CREATE TABLE IF NOT EXISTS browser_session_reconciliation_receipts (digest TEXT PRIMARY KEY, recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, differences_json TEXT NOT NULL)").map_err(|_| "browser_session_reconciliation_receipt_failed")?;
        connection.execute("INSERT OR IGNORE INTO browser_session_reconciliation_receipts(digest,differences_json) VALUES (?1,?2)", rusqlite::params![digest,payload]).map_err(|_| "browser_session_reconciliation_receipt_failed")?;
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_named_pid_cannot_erase_an_independent_uncertain_operation() {
        use agent_browser_service_model::{BrowserTaskOperation, BrowserTaskOperationState};
        let row = |request: &str, state| BrowserTaskOperation {
            request_id: request.into(),
            task_id: "task".into(),
            profile_id: "profile".into(),
            browser_id: "browser".into(),
            tab_id: "tab".into(),
            target_id: "target".into(),
            process_pid: 42,
            process_identity: None,
            control_epoch: 1,
            action: "evaluate".into(),
            fingerprint: format!("synthetic-{request}"),
            state,
            created_at_ms: 1,
            completed_at_ms: None,
            evidence_references: Default::default(),
        };
        let mut base = BrowserSessionState::default();
        base.task_operations
            .entry("task".into())
            .or_default()
            .insert("pid".into(), row("pid", BrowserTaskOperationState::Pending));
        let mut local = base.clone();
        let uncertain = row("independent", BrowserTaskOperationState::Uncertain);
        local
            .task_operations
            .get_mut("task")
            .unwrap()
            .insert("independent".into(), uncertain.clone());
        let mut current = base.clone();
        current
            .task_operations
            .get_mut("task")
            .unwrap()
            .get_mut("pid")
            .unwrap()
            .state = BrowserTaskOperationState::OwnerAcknowledged;
        let (merged, conflicts) = reconcile(&base, &local, &current).unwrap();
        assert_eq!(merged.task_operations["task"]["independent"], uncertain);
        assert_eq!(
            merged.task_operations["task"]["pid"].state,
            BrowserTaskOperationState::OwnerAcknowledged
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn resource_identity_is_never_spliced_with_stale_resident_fields() {
        let base = json!({"pid":1,"cdpEndpoint":"http://synthetic/old","lastActivityAtMs":1});
        let local = json!({"pid":1,"cdpEndpoint":"http://synthetic/old","lastActivityAtMs":3});
        let current = json!({"pid":2,"cdpEndpoint":"http://synthetic/new","lastActivityAtMs":2});
        let mut conflicts = Vec::new();
        let merged = choose(
            Some(&base),
            Some(&local),
            Some(&current),
            "/browsers/owned",
            2,
            &mut conflicts,
        );
        assert_eq!(merged, Some(current));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0]["resident"], local);
    }

    #[test]
    fn concurrent_additions_survive_without_resurrecting_removed_history() {
        let base = json!(["archived"]);
        let local = json!(["archived", "resident-addition"]);
        let current = json!(["peer-addition"]);
        let mut conflicts = Vec::new();
        assert_eq!(
            choose(
                Some(&base),
                Some(&local),
                Some(&current),
                "/sessionHistory",
                1,
                &mut conflicts
            ),
            Some(json!(["peer-addition", "resident-addition"]))
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn disjoint_changes_and_sequence_advances_survive_without_replay() {
        let base = BrowserSessionState::default();
        let mut local = base.clone();
        local
            .profile_executables
            .insert("alice".into(), "/synthetic/alice".into());
        local.next_session_sequence = 3;
        let mut current = base.clone();
        current
            .profile_executables
            .insert("bob".into(), "/synthetic/bob".into());
        current.next_session_sequence = 4;
        let (merged, conflicts) = reconcile(&base, &local, &current).unwrap();
        assert_eq!(merged.profile_executables.len(), 2);
        assert_eq!(merged.next_session_sequence, 4);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn current_identity_wins_and_overlapping_observation_is_receipted_atomically() {
        let connection = Connection::open_in_memory().unwrap();
        let base = BrowserSessionState::default();
        let mut local = base.clone();
        local
            .profile_executables
            .insert("same".into(), "/synthetic/local".into());
        let mut current = base.clone();
        current
            .profile_executables
            .insert("same".into(), "/synthetic/current".into());
        let transaction = connection.unchecked_transaction().unwrap();
        let merged = reconcile_and_record(&transaction, &base, &local, &current).unwrap();
        assert_eq!(merged.profile_executables["same"], "/synthetic/current");
        let payload: String = transaction
            .query_row(
                "SELECT differences_json FROM browser_session_reconciliation_receipts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(payload.contains("/synthetic/local"));
        transaction.rollback().unwrap();
        let table_count: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE name='browser_session_reconciliation_receipts'", [], |row| row.get(0)).unwrap();
        assert_eq!(
            table_count, 0,
            "receipt must roll back with failed publication"
        );
    }
}
