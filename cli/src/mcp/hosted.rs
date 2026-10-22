//! Read-only hosted task metadata, separate from the MCP stdio transport.
//!
//! Watch emits NDJSON snapshots, polling only database/WAL file metadata every
//! 250 ms while unchanged. Cursors are process-local `{watchId, sequence}` values;
//! they are not replay tokens. Reconnect starts with a complete snapshot. Missing
//! state or read errors terminate with a nonzero exit; a broken stdout pipe exits
//! cleanly on the next emission (terminate the process for an immediate disconnect). No heartbeat or browser readiness claim is implied by silence.
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::{json, Value};

use crate::native::browser_host_context;
use crate::native::browser_session_store::BrowserSessionSqliteStore;

pub(super) fn task_argument(args: &[String]) -> Result<&str, String> {
    if args.len() != 3 || args[2].trim().is_empty() {
        return Err("Usage: agent-browser --json mcp host-task|watch-host-task <task-id>".into());
    }
    Ok(&args[2])
}

pub(super) fn read_snapshot(task_id: &str) -> Result<Value, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    read_snapshot_at(&path, task_id)
}

fn read_snapshot_at(path: &Path, task_id: &str) -> Result<Value, String> {
    let store = BrowserSessionSqliteStore::open_read_only(path)?;
    browser_host_context::snapshot(&store.load_session_state()?, task_id)
}

#[derive(Debug, PartialEq, Eq)]
struct FileStamp {
    modified: Option<SystemTime>,
    length: u64,
}

fn stamp(path: &Path) -> Result<Option<FileStamp>, String> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(FileStamp {
            modified: metadata.modified().ok(),
            length: metadata.len(),
        })),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("browser_host_watch_metadata_unavailable".into()),
    }
}

fn stamps(path: &Path) -> Result<[Option<FileStamp>; 3], String> {
    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    Ok([
        stamp(path)?,
        stamp(&PathBuf::from(wal))?,
        stamp(
            path.parent()
                .ok_or("browser_host_watch_directory_missing")?,
        )?,
    ])
}

#[derive(Default)]
struct Projection {
    previous: Option<Value>,
    sequence: u64,
}

impl Projection {
    fn changed(&mut self, snapshot: Value, watch_id: &str) -> Option<Value> {
        if self.previous.as_ref() == Some(&snapshot) {
            return None;
        }
        let kind = if self.previous.is_none() {
            "snapshot"
        } else {
            "changed"
        };
        self.sequence += 1;
        self.previous = Some(snapshot.clone());
        Some(json!({
            "version": 1, "event": kind,
            "cursor": {"watchId": watch_id, "sequence": self.sequence},
            "data": snapshot
        }))
    }
}

pub(super) fn watch(args: &[String], mut output: impl Write) -> Result<(), String> {
    let task_id = task_argument(args)?;
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let watch_id = uuid::Uuid::new_v4().to_string();
    let mut projection = Projection::default();
    let mut previous_stamps = None;
    loop {
        // Capture before the read so a concurrent commit triggers another read.
        let current_stamps = stamps(&path)?;
        if previous_stamps.as_ref() != Some(&current_stamps) {
            let snapshot = read_snapshot_at(&path, task_id)?;
            if let Some(event) = projection.changed(snapshot, &watch_id) {
                let result = writeln!(output, "{event}").and_then(|()| output.flush());
                match result {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
                    Err(_) => return Err("browser_host_watch_output_failed".into()),
                }
            }
            previous_stamps = Some(current_stamps);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_watch_emits_initial_and_only_relevant_changes() {
        let mut projection = Projection::default();
        let initial = json!({"taskId":"task", "tabId":"one", "viewerState":"unverified"});
        let first = projection.changed(initial.clone(), "watch").unwrap();
        assert_eq!(first["event"], "snapshot");
        assert_eq!(first["cursor"]["sequence"], 1);
        assert!(projection.changed(initial, "watch").is_none());
        let next = projection
            .changed(json!({"taskId":"task", "tabId":"two"}), "watch")
            .unwrap();
        assert_eq!(next["event"], "changed");
        assert_eq!(next["cursor"]["sequence"], 2);
        assert_eq!(next["data"]["tabId"], "two");
    }

    #[test]
    fn host_watch_reconnect_starts_a_new_complete_snapshot() {
        let mut projection = Projection::default();
        let event = projection
            .changed(json!({"taskId":"task"}), "new-watch")
            .unwrap();
        assert_eq!(event["event"], "snapshot");
        assert_eq!(
            event["cursor"],
            json!({"watchId":"new-watch", "sequence":1})
        );
    }

    #[test]
    fn host_watch_metadata_filter_detects_wal_changes_without_loading_state() {
        let directory = std::env::temp_dir().join(format!("host-watch-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("state.sqlite");
        std::fs::write(&path, b"synthetic metadata only").unwrap();
        let initial = stamps(&path).unwrap();
        assert_eq!(initial, stamps(&path).unwrap());
        let wal = directory.join("state.sqlite-wal");
        std::fs::write(&wal, b"commit").unwrap();
        let changed = stamps(&path).unwrap();
        assert_ne!(initial, changed);
        assert_eq!(changed, stamps(&path).unwrap());
        std::fs::write(&wal, b"larger synthetic commit").unwrap();
        assert_ne!(changed, stamps(&path).unwrap());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn host_task_rejects_missing_or_extra_arguments() {
        assert!(task_argument(&["mcp".into(), "host-task".into()]).is_err());
        assert!(task_argument(&[
            "mcp".into(),
            "host-task".into(),
            "task".into(),
            "extra".into()
        ])
        .is_err());
    }
}
