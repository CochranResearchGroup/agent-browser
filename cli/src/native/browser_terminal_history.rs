//! Bounded working history backed by append-only records in the same SQLite transaction.
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(super) fn active_limit() -> Result<usize, String> {
    match std::env::var("AGENT_BROWSER_TERMINAL_HISTORY_LIMIT") {
        Ok(value) => value
            .parse()
            .map_err(|_| "terminal_history_limit_invalid".to_string()),
        Err(std::env::VarError::NotPresent) => Ok(1000),
        Err(_) => Err("terminal_history_limit_invalid".into()),
    }
}

fn retained_indices(value: &Value, kind: &str, limit: usize) -> BTreeSet<usize> {
    let reference_field = if kind == "sessionHistory" {
        "sessionId"
    } else {
        "tabId"
    };
    let pinned: BTreeSet<&str> = value
        .get("remoteViewTabHandoffs")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|links| links.values())
        .filter_map(|link| link.get(reference_field).and_then(Value::as_str))
        .collect();
    let Some(records) = value.get(kind).and_then(Value::as_array) else {
        return BTreeSet::new();
    };
    let mut retained = BTreeSet::new();
    let mut recent = 0;
    for (index, record) in records.iter().enumerate().rev() {
        if record
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| pinned.contains(id))
        {
            retained.insert(index);
        } else if recent < limit {
            retained.insert(index);
            recent += 1;
        }
    }
    retained
}

fn project(value: &mut Value, limit: usize) {
    for kind in ["sessionHistory", "tabHistory"] {
        let retained = retained_indices(value, kind, limit);
        if let Some(records) = value.get_mut(kind).and_then(Value::as_array_mut) {
            let mut index = 0;
            records.retain(|_| {
                let keep = retained.contains(&index);
                index += 1;
                keep
            });
        }
    }
}

/// Read projection only. Older legacy document records stay durable until the
/// next publication archives them and reduces the active document atomically.
pub(super) fn project_json(json: &str) -> Result<String, String> {
    let mut value: Value =
        serde_json::from_str(json).map_err(|_| "terminal_history_document_invalid")?;
    project(&mut value, active_limit()?);
    serde_json::to_string(&value).map_err(|_| "terminal_history_document_serialize_failed".into())
}

/// Caller owns the publication transaction; an archival failure must abort it.
pub(super) fn prepare_json(connection: &Connection, json: &str) -> Result<String, String> {
    let limit = active_limit()?;
    let previous: Option<String> = connection
        .query_row(
            "SELECT json FROM state_documents WHERE kind='browser_session_state'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "terminal_history_previous_read_failed")?;
    let mut value: Value =
        serde_json::from_str(json).map_err(|_| "terminal_history_document_invalid")?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS terminal_history_archive (
          sequence INTEGER PRIMARY KEY, kind TEXT NOT NULL, digest TEXT NOT NULL,
          json TEXT NOT NULL, UNIQUE(kind,digest));",
        )
        .map_err(|_| "terminal_history_archive_unavailable")?;
    let old: Option<Value> = previous
        .map(|text| serde_json::from_str(&text))
        .transpose()
        .map_err(|_| "terminal_history_previous_invalid")?;
    for source in old.iter().chain(std::iter::once(&value)) {
        for kind in ["sessionHistory", "tabHistory"] {
            let retained = retained_indices(source, kind, limit);
            if let Some(records) = source.get(kind).and_then(Value::as_array) {
                for (_, record) in records
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !retained.contains(index))
                {
                    let payload = serde_json::to_string(record)
                        .map_err(|_| "terminal_history_record_invalid")?;
                    let digest = format!("{:x}", Sha256::digest(payload.as_bytes()));
                    connection.execute(
                        "INSERT OR IGNORE INTO terminal_history_archive(kind,digest,json) VALUES (?1,?2,?3)",
                        params![kind, digest, payload],
                    ).map_err(|_| "terminal_history_archive_write_failed")?;
                }
            }
        }
    }
    project(&mut value, limit);
    serde_json::to_string(&value).map_err(|_| "terminal_history_document_serialize_failed".into())
}

fn archive_exists(connection: &Connection) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='terminal_history_archive'",
            [],
            |_| Ok(true),
        )
        .optional()
        .map(|result| result.unwrap_or(false))
        .map_err(|_| "terminal_history_archive_read_failed".into())
}

/// Read a bounded, newest-first archive page without altering the runtime.
pub(super) fn page(
    connection: &Connection,
    kind: &str,
    limit: usize,
    before: Option<i64>,
) -> Result<Value, String> {
    let kind = match kind {
        "sessions" => "sessionHistory",
        "tabs" => "tabHistory",
        _ => return Err("terminal_history_kind_invalid".into()),
    };
    if !(1..=1000).contains(&limit) || before.is_some_and(|value| value <= 0) {
        return Err("terminal_history_page_invalid".into());
    }
    let mut records = Vec::new();
    if archive_exists(connection)? {
        let mut statement = connection.prepare(
            "SELECT sequence,json FROM terminal_history_archive WHERE kind=?1 AND (?2 IS NULL OR sequence<?2) ORDER BY sequence DESC LIMIT ?3",
        ).map_err(|_| "terminal_history_archive_read_failed")?;
        let rows = statement
            .query_map(params![kind, before, limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| "terminal_history_archive_read_failed")?;
        for row in rows {
            let (sequence, json) = row.map_err(|_| "terminal_history_archive_read_failed")?;
            let record: Value = serde_json::from_str(&json)
                .map_err(|_| "terminal_history_archive_record_invalid")?;
            records.push(serde_json::json!({"sequence":sequence,"record":record}));
        }
    }
    let next_cursor = (records.len() == limit)
        .then(|| {
            records
                .last()
                .and_then(|record| record["sequence"].as_i64())
        })
        .flatten();
    Ok(
        serde_json::json!({"records":records,"nextCursor":next_cursor,"activeLimit":active_limit()?}),
    )
}

pub(super) fn status(connection: &Connection) -> Result<Value, String> {
    let mut sessions = 0_i64;
    let mut tabs = 0_i64;
    if archive_exists(connection)? {
        sessions = connection
            .query_row(
                "SELECT COUNT(*) FROM terminal_history_archive WHERE kind='sessionHistory'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| "terminal_history_archive_read_failed")?;
        tabs = connection
            .query_row(
                "SELECT COUNT(*) FROM terminal_history_archive WHERE kind='tabHistory'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| "terminal_history_archive_read_failed")?;
    }
    Ok(
        serde_json::json!({"activeLimit":active_limit()?,"archivedSessionCount":sessions,"archivedTabCount":tabs,"retention":"durable_no_expiry"}),
    )
}
