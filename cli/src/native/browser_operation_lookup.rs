//! Read-only operation discovery. Private journals remain owner-only; public
//! summaries reuse the operation's action projection and contain no paths.
use super::*;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};

fn journal(path: &Path) -> Result<Option<Connection>, String> {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("browser_operation_journal_unavailable".into()),
        Ok(_) => {}
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "browser_operation_journal_unavailable")?;
    connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(|_| "browser_operation_journal_unavailable")?;
    Ok(Some(connection))
}
fn table(connection: &Connection, name: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [name],
            |row| row.get(0),
        )
        .map_err(|_| "browser_operation_journal_unavailable".into())
}
const LOOKUP_LIMIT: usize = 256;

#[derive(Default)]
struct OperationLookup {
    operations: Vec<Value>,
    history_truncated: bool,
}

fn mode_operations(
    database: &Path,
    profile: &str,
    browser: Option<&str>,
    task: Option<&str>,
    id: Option<&str>,
    active_id: Option<&str>,
) -> Result<OperationLookup, String> {
    let Some(connection) = journal(&database.with_extension("mode.sqlite3"))? else {
        return Ok(OperationLookup::default());
    };
    if !table(&connection, "browser_mode_operations")? {
        return Ok(OperationLookup::default());
    }
    let mut query = connection.prepare("SELECT state FROM browser_mode_operations WHERE json_extract(state,'$.request.profile_id')=?1 AND (?2 IS NULL OR id=?2) AND (?3 IS NULL OR json_extract(state,'$.request.browser_id')=?3) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM json_each(state,'$.request.task_ids') WHERE value=?4)) ORDER BY CASE WHEN id=?5 THEN 0 ELSE 1 END, terminal, rowid DESC LIMIT ?6")
        .map_err(|_| "browser_operation_readback_unavailable")?;
    let rows = query
        .query_map(
            rusqlite::params![
                profile,
                id,
                browser,
                task,
                active_id,
                (LOOKUP_LIMIT + 1) as i64
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "browser_operation_readback_unavailable")?;
    let mut lookup = OperationLookup::default();
    for (index, row) in rows.enumerate() {
        if index == LOOKUP_LIMIT {
            lookup.history_truncated = true;
            break;
        }
        let operation: super::super::browser_mode_operation::Operation =
            serde_json::from_str(&row.map_err(|_| "browser_operation_readback_unavailable")?)
                .map_err(|_| "browser_operation_readback_invalid")?;
        if browser.is_some_and(|id| id != operation.request.browser_id)
            || task.is_some_and(|id| !operation.request.task_ids.iter().any(|task| task == id))
        {
            continue;
        }
        let mut view = super::browser_mode_host::public_operation_view(
            &operation,
            task.or_else(|| operation.request.task_ids.first().map(String::as_str))
                .ok_or("browser_operation_task_binding_missing")?,
            None,
        );
        super::browser_mode_host::decorate_retained_build(&connection, &operation, &mut view)?;
        view["kind"] = json!("mode_convert");
        lookup.operations.push(view);
    }
    Ok(lookup)
}
fn copy_operations(
    database: &Path,
    profile: &str,
    browser: Option<&str>,
    task: Option<&str>,
    id: Option<&str>,
    active_id: Option<&str>,
) -> Result<OperationLookup, String> {
    let path = database
        .parent()
        .ok_or("browser_operation_runtime_directory_missing")?
        .join("profile-duplication/operations.sqlite3");
    let Some(connection) = journal(&path)? else {
        return Ok(OperationLookup::default());
    };
    if !table(&connection, "profile_duplicate_host_intents")? {
        return Ok(OperationLookup::default());
    }
    let has_operations = table(&connection, "profile_duplicate_operations")?;
    let has_bindings = table(&connection, "profile_duplicate_owner_bindings")?;
    // Older journals can retain an intent before either auxiliary table exists.
    // SQL must scope every available binding before applying the readback budget.
    let binding = if has_bindings {
        "bindings.binding"
    } else {
        "NULL"
    };
    let state = if has_operations {
        "operations.state"
    } else {
        "NULL"
    };
    let bindings_join = if has_bindings {
        "LEFT JOIN profile_duplicate_owner_bindings AS bindings ON bindings.id=intents.id"
    } else {
        ""
    };
    let operations_join = if has_operations {
        "LEFT JOIN profile_duplicate_operations AS operations ON operations.id=intents.id"
    } else {
        ""
    };
    let terminal = if has_operations {
        "COALESCE(operations.terminal,json_extract(intents.intent,'$.cancelled'),0)"
    } else {
        "COALESCE(json_extract(intents.intent,'$.cancelled'),0)"
    };
    let sql = format!("SELECT intents.id,intents.intent,{binding},{state} FROM profile_duplicate_host_intents AS intents {bindings_join} {operations_join} WHERE json_extract(intents.intent,'$.source_profile_id')=?1 AND (?2 IS NULL OR intents.id=?2) AND (?3 IS NULL OR {binding} IS NULL OR json_extract({binding},'$.browser.id')=?3) AND (?4 IS NULL OR {binding} IS NULL OR EXISTS(SELECT 1 FROM json_each({binding},'$.reviewed_tasks') WHERE value=?4)) ORDER BY CASE WHEN intents.id=?5 THEN 0 ELSE 1 END,{terminal},intents.rowid DESC LIMIT ?6");
    let mut query = connection
        .prepare(&sql)
        .map_err(|_| "browser_operation_readback_unavailable")?;
    let rows = query
        .query_map(
            rusqlite::params![
                profile,
                id,
                browser,
                task,
                active_id,
                (LOOKUP_LIMIT + 1) as i64
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(|_| "browser_operation_readback_unavailable")?;
    let mut lookup = OperationLookup::default();
    for (index, row) in rows.enumerate() {
        let (id, raw, binding, state) =
            row.map_err(|_| "browser_operation_readback_unavailable")?;
        if index == LOOKUP_LIMIT {
            lookup.history_truncated = true;
            break;
        }
        let intent: Value =
            serde_json::from_str(&raw).map_err(|_| "browser_operation_readback_invalid")?;
        if intent["operation_id"] != id || intent["source_profile_id"] != profile {
            return Err("browser_operation_readback_invalid".into());
        }
        let binding: Option<Value> = binding
            .map(|raw| serde_json::from_str(&raw))
            .transpose()
            .map_err(|_| "browser_operation_readback_invalid")?;
        let mut view = if let Some(raw) = state {
            let operation: super::profile_duplicate_host::profile_duplicate_copy::Operation =
                serde_json::from_str(&raw).map_err(|_| "browser_operation_readback_invalid")?;
            operation.public_view()
        } else {
            let cancelled = intent["cancelled"] == true;
            json!({"schemaVersion":"agent-browser.profile-duplication.v1","operationId":id,"phase":if cancelled {"cancelled"} else {"source_preparation_required"},"source":{"id":profile},"destination":{"id":intent["destination_profile_id"],"name":intent["destination_name"]},"launchRequested":intent["launch"],"actions":if cancelled {json!([])} else {json!([{"action":"open_source","requiresConfirmation":true},{"action":"cancel","requiresConfirmation":false}])}})
        };
        view =
            super::profile_duplicate_host::decorate_retained_build(view, &raw, binding.as_ref())?;
        view["kind"] = json!("profile_duplicate");
        view["profileId"] = json!(profile);
        view["browserId"] = binding
            .as_ref()
            .map(|value| value["browser"]["id"].clone())
            .unwrap_or_else(|| json!(browser));
        view["taskId"] = json!(task);
        lookup.operations.push(view);
    }
    Ok(lookup)
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    /// Intercept inspection and known profile contention before any refresh,
    /// binding creation, reaping, close, launch or copy effect is admitted.
    pub(super) fn operation_request_readback(
        &self,
        command: &Value,
    ) -> Result<Option<Value>, String> {
        let kind = match command["action"].as_str() {
            Some("browser_session_mode_convert") => "mode_convert",
            Some("browser_session_profile_duplicate") => "profile_duplicate",
            _ => return Ok(None),
        };
        let inspect = command["choice"] == "inspect";
        let state = self.persistence.load_session_state()?;
        let task_id =
            optional_string(command, "sessionId").or_else(|| optional_string(command, "taskId"));
        let task = task_id
            .map(|id| {
                state
                    .sessions
                    .get(id)
                    .ok_or("browser_operation_task_missing")
            })
            .transpose()?;
        if let Some(task) = task {
            if optional_string(command, "sessionName") != Some(task.name.as_str()) {
                return Err("browser_operation_task_binding_changed".into());
            }
        } else if kind == "mode_convert" {
            return Err("browser_operation_task_required".into());
        }
        let profile = task
            .map(|task| task.profile_id.as_str())
            .or_else(|| optional_string(command, "sourceProfileId"));
        let Some(profile) = profile else {
            return if inspect {
                Err("browser_operation_profile_required".into())
            } else {
                Ok(None)
            };
        };
        if optional_string(command, "sourceProfileId").is_some_and(|id| id != profile)
            || optional_string(command, "browserId")
                .is_some_and(|id| task.is_none_or(|task| task.browser_id != id))
        {
            return Err("browser_operation_target_binding_changed".into());
        }
        if !self.catalog.profiles.contains_key(profile)
            && !state.disposable_profiles.contains_key(profile)
        {
            return Err("browser_operation_profile_missing".into());
        }
        let browser_id = task.map(|task| task.browser_id.as_str());
        for browser in state
            .browsers
            .values()
            .filter(|browser| browser.profile_id == profile)
        {
            super::super::browser_host_control::check(&browser.id, "url")?;
        }
        let database = self.persistence.runtime_database_path()?;
        let id = optional_string(command, "operationId");
        if inspect {
            let lookup = if kind == "mode_convert" {
                mode_operations(
                    &database,
                    profile,
                    browser_id,
                    task_id,
                    id,
                    state.profile_operation_ids.get(profile).map(String::as_str),
                )?
            } else {
                copy_operations(
                    &database,
                    profile,
                    browser_id,
                    task_id,
                    id,
                    state.profile_operation_ids.get(profile).map(String::as_str),
                )?
            };
            let operations = lookup.operations;
            if id.is_some() && operations.is_empty() {
                return Err("browser_operation_not_found".into());
            }
            return Ok(Some(
                json!({"schemaVersion":"agent-browser.browser-operations.v1","kind":kind,"profileId":profile,"browserId":browser_id,"taskId":task_id,"operations":operations,"historyTruncated":lookup.history_truncated}),
            ));
        }
        let mut existing = if let Some(active_id) = state.profile_operation_ids.get(profile) {
            if Some(active_id.as_str()) == id {
                return Ok(None);
            }
            let mut existing = mode_operations(
                &database,
                profile,
                None,
                None,
                Some(active_id),
                Some(active_id),
            )?
            .operations;
            existing.extend(
                copy_operations(
                    &database,
                    profile,
                    None,
                    None,
                    Some(active_id),
                    Some(active_id),
                )?
                .operations,
            );
            existing
        } else {
            // A crash can retain the durable unique browser operation before
            // its profile fence was published. Return that original locator.
            mode_operations(&database, profile, browser_id, None, None, None)?
                .operations
                .into_iter()
                .filter(|operation| {
                    !matches!(operation["phase"].as_str(), Some("completed" | "cancelled"))
                        && operation["operationId"].as_str() != id
                })
                .collect()
        };
        if !existing.is_empty() {
            if existing.len() != 1 {
                return Err(
                    "browser_operation_busy_readback_unavailable:inspect current operation".into(),
                );
            }
            let active = existing.remove(0);
            return Ok(Some(
                json!({"schemaVersion":"agent-browser.browser-operations.v1","kind":kind,"state":"busy","profileId":profile,"browserId":browser_id,"taskId":task_id,"activeOperation":active,"operations":[active]}),
            ));
        }
        Ok(None)
    }
}
