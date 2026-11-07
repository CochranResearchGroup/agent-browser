//! Offline reset publication. Callers must prove the production writer and owned
//! browsers have exited before entering this transaction. Profiles and effect
//! journals are never reset, and the same operation ID cannot end later tasks.
use super::*;
use agent_browser_service_model::{
    BrowserTabEndReason, SessionEndReason, TerminalBrowserSession, TerminalBrowserTab,
};
use serde_json::{json, Value};

pub(crate) fn reset_preview() -> Result<Value, String> {
    let Some(state) = BrowserSessionSqliteStore::default_session_state_read_only()? else {
        return Ok(json!({"sessions":0,"tabs":0,"browsers":0}));
    };
    Ok(counts(&state))
}

fn counts(state: &BrowserSessionState) -> Value {
    json!({"sessions":state.sessions.len(),"tabs":state.tabs.len(),"browsers":state.browsers.len()})
}

pub(crate) fn reset_backup(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Err("runtime_reset_backup_already_exists".into());
    }
    let source = BrowserSessionSqliteStore::open_read_only(
        &BrowserSessionSqliteStore::default_sqlite_path()?,
    )?;
    let staged = path.with_extension(format!("staged-{}", uuid::Uuid::new_v4()));
    let mut target =
        Connection::open(&staged).map_err(|e| format!("runtime_reset_backup_open:{e}"))?;
    backup::Backup::new(&source.connection, &mut target)
        .and_then(|backup| backup.run_to_completion(128, Duration::from_millis(10), None))
        .map_err(|e| format!("runtime_reset_backup_failed:{e}"))?;
    let integrity: String = target
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("runtime_reset_backup_integrity:{e}"))?;
    if integrity != "ok" {
        return Err("runtime_reset_backup_integrity_failed".into());
    }
    drop(target);
    fs::File::open(&staged)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("runtime_reset_backup_sync:{e}"))?;
    fs::rename(&staged, path).map_err(|e| format!("runtime_reset_backup_publish:{e}"))?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn reset_backup_digest(path: &Path) -> Result<String, String> {
    let store = BrowserSessionSqliteStore::open_read_only(path)?;
    sqlite_integrity_state(&store.connection)?;
    let bytes = fs::read(path).map_err(|e| format!("runtime_reset_backup_read:{e}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Read the transaction's durable outcome before resuming any process effects.
/// A lost external receipt write must not let the same reset end newer tasks.
pub(crate) fn reset_result(id: &str) -> Result<Option<Value>, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let store = BrowserSessionSqliteStore::open_read_only(&path)?;
    let raw: Option<String> = store
        .connection
        .query_row(
            "SELECT json FROM state_documents WHERE kind=?1",
            [format!("runtime_reset:{id}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    raw.map(|text| {
        serde_json::from_str(&text).map_err(|e| format!("runtime_reset_outcome_invalid:{e}"))
    })
    .transpose()
}

pub(crate) fn commit_reset(
    id: &str,
    now: u64,
    coordination: &crate::candidate_coordination::CandidateCoordinationStore,
    custody: &crate::candidate_coordination::CandidateInstallCustody,
) -> Result<Value, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    if !path.exists() {
        return Err("runtime_reset_database_missing".into());
    }
    let parent = path
        .parent()
        .ok_or("runtime_reset_database_parent_missing")?;
    let mut store = BrowserSessionSqliteStore::open(parent)?;
    commit_fenced(&mut store.connection, id, now, |transaction| {
        coordination.with_active_install(custody, |_| {
            transaction
                .commit()
                .map_err(|e| format!("runtime_reset_commit:{e}"))
        })
    })
}

#[cfg(test)]
fn commit(connection: &mut Connection, id: &str, now: u64) -> Result<Value, String> {
    commit_fenced(connection, id, now, |transaction| {
        transaction.commit().map_err(|e| e.to_string())
    })
}
fn commit_fenced(
    connection: &mut Connection,
    id: &str,
    now: u64,
    fence: impl FnOnce(rusqlite::Transaction<'_>) -> Result<(), String>,
) -> Result<Value, String> {
    let key = format!("runtime_reset:{id}");
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("runtime_reset_transaction:{e}"))?;
    let previous: Option<String> = transaction
        .query_row(
            "SELECT json FROM state_documents WHERE kind=?1",
            [&key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("runtime_reset_receipt_read:{e}"))?;
    if let Some(previous) = previous {
        return serde_json::from_str(&previous)
            .map_err(|e| format!("runtime_reset_receipt_invalid:{e}"));
    }
    let mut state: BrowserSessionState = load_document(
        &transaction,
        SESSION_STATE_DOCUMENT,
        BROWSER_SESSION_STATE_SCHEMA_V1,
    )?;
    let before = counts(&state);
    retire(&mut state, now)?;
    let result = json!({"before":before,"after":counts(&state),"operationId":id,"profilesPreserved":true,"journalsPreserved":true});
    save_document(
        &transaction,
        SESSION_STATE_DOCUMENT,
        BROWSER_SESSION_STATE_SCHEMA_V1,
        &state,
    )?;
    save_document(
        &transaction,
        &key,
        "agent-browser.runtime-reset-store.v1",
        &result,
    )?;
    // Serialization and archive preparation finish before the coordinator's
    // bounded final commit fence. No shared coordination lock spans them.
    fence(transaction)?;
    Ok(result)
}

fn retire(state: &mut BrowserSessionState, now: u64) -> Result<(), String> {
    for tab in state.tabs.values() {
        let profile_id = state
            .sessions
            .get(&tab.session_id)
            .map(|s| s.profile_id.clone())
            .or_else(|| {
                state
                    .browsers
                    .get(&tab.browser_id)
                    .map(|b| b.profile_id.clone())
            })
            .ok_or("runtime_reset_tab_profile_unresolved")?;
        state.tab_history.push(TerminalBrowserTab {
            id: tab.id.clone(),
            target_id: tab.target_id.clone(),
            browser_id: tab.browser_id.clone(),
            session_id: tab.session_id.clone(),
            profile_id,
            created_at_ms: tab.created_at_ms,
            last_activity_at_ms: tab.last_activity_at_ms,
            closed_at_ms: now,
            reason: BrowserTabEndReason::SessionEnded,
            session_end_reason: Some(SessionEndReason::ExplicitClose),
        });
    }
    for session in state.sessions.values() {
        state.session_history.push(TerminalBrowserSession {
            id: session.id.clone(),
            name: session.name.clone(),
            profile_id: session.profile_id.clone(),
            browser_id: session.browser_id.clone(),
            created_at_ms: session.created_at_ms,
            last_activity_at_ms: session.last_activity_at_ms,
            ended_at_ms: now,
            reason: SessionEndReason::ExplicitClose,
        });
    }
    state.sessions.clear();
    state.tabs.clear();
    state.browsers.clear();
    state.idle_closed_browsers.clear();
    state.shared_browser_ids.clear();
    state.adopted_browser_process_identities.clear();
    state.leave_open_session_ids.clear();
    state.retention_claims.clear();
    state.tab_lifetimes.clear();
    state.tab_metadata.clear();
    state.pending_human_tabs.clear();
    state.remote_view_presentations.clear();
    state.app_browser_attachments.clear();
    state.attached_task_targets.clear();
    state.host_bindings.clear();
    // managed_tab_requests, task_operations and profile_operation_ids preserve
    // replay and uncertain-effect custody. Links resolve against terminal history.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_reset_commit_is_atomic_and_replay_cannot_end_new_work() {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut state = BrowserSessionState {
            next_session_sequence: 47,
            ..Default::default()
        };
        state.sessions.insert(
            "session".into(),
            agent_browser_service_model::ManagedBrowserSession {
                id: "session".into(),
                name: "workflow".into(),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
                expires_at_ms: 500,
                current_tab_id: Some("tab".into()),
            },
        );
        state.tabs.insert(
            "tab".into(),
            agent_browser_service_model::ManagedBrowserTab {
                id: "tab".into(),
                target_id: "target".into(),
                browser_id: "browser".into(),
                session_id: "session".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
            },
        );
        state
            .profile_operation_ids
            .insert("profile".into(), "uncertain-operation".into());
        save_document(
            &connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &state,
        )
        .unwrap();
        let first = commit(&mut connection, "reset-1", 100).unwrap();
        let mut later: BrowserSessionState = load_document(
            &connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )
        .unwrap();
        assert_eq!(
            later.profile_operation_ids["profile"],
            "uncertain-operation"
        );
        assert_eq!(later.next_session_sequence, 47);
        assert!(later.sessions.is_empty() && later.tabs.is_empty());
        assert_eq!(later.session_history[0].id, "session");
        assert_eq!(
            later.session_history[0].reason,
            SessionEndReason::ExplicitClose
        );
        assert_eq!(later.tab_history[0].profile_id, "profile");
        assert_eq!(later.tab_history[0].closed_at_ms, 100);
        later.next_session_sequence = 48;
        save_document(
            &connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &later,
        )
        .unwrap();
        assert_eq!(commit(&mut connection, "reset-1", 200).unwrap(), first);
        let preserved: BrowserSessionState = load_document(
            &connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )
        .unwrap();
        assert_eq!(preserved.next_session_sequence, 48);
    }
    #[test]
    fn runtime_reset_invalid_tab_rolls_back_every_document() {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_runtime_schema(&connection).unwrap();
        let mut state = BrowserSessionState::default();
        state.tabs.insert(
            "tab".into(),
            agent_browser_service_model::ManagedBrowserTab {
                id: "tab".into(),
                target_id: "target".into(),
                browser_id: "missing".into(),
                session_id: "missing".into(),
                created_at_ms: 1,
                last_activity_at_ms: 2,
            },
        );
        save_document(
            &connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
            &state,
        )
        .unwrap();
        assert_eq!(
            commit(&mut connection, "reset-2", 100).unwrap_err(),
            "runtime_reset_tab_profile_unresolved"
        );
        let preserved: BrowserSessionState = load_document(
            &connection,
            SESSION_STATE_DOCUMENT,
            BROWSER_SESSION_STATE_SCHEMA_V1,
        )
        .unwrap();
        assert_eq!(preserved, state);
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM state_documents WHERE kind LIKE 'runtime_reset:%'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
