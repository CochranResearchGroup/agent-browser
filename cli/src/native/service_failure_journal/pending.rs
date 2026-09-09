//! Private durable custody until the append-only journal has synced an occurrence.
//! Recovery holds the journal lock across deduplication, append, and retirement.

use super::ServiceFailureRecord;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const BATCH_SIZE: usize = 256;
const MAX_RECORD_BYTES: u64 = 65_536;

fn directory(journal: &Path) -> PathBuf {
    journal.with_extension("pending")
}

pub(super) fn count(journal: &Path) -> Result<u64, String> {
    let entries = match fs::read_dir(directory(journal)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err("failure_pending_scan_failed".into()),
    };
    let mut count = 0;
    for entry in entries {
        let entry = entry.map_err(|_| "failure_pending_entry_failed")?;
        if entry.path().extension().is_some_and(|ext| ext == "json") {
            count += 1;
        }
    }
    Ok(count)
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| "failure_pending_directory_sync_failed".to_string())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|_| "failure_pending_directory_create_failed")?;
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "failure_pending_directory_unavailable")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("failure_pending_directory_not_regular".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| "failure_pending_permissions_failed")?;
    }
    sync_directory(path)?;
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

/// Success means the complete redacted record and its directory entry are synced
/// on Unix. A random publication name avoids overwriting another writer's record.
pub(super) fn stage(journal: &Path, record: &ServiceFailureRecord) -> Result<(), String> {
    let directory = directory(journal);
    private_directory(&directory)?;
    let bytes = serde_json::to_vec(record).map_err(|_| "failure_pending_encode_failed")?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err("failure_pending_record_too_large".into());
    }
    let temporary = directory.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let published = temporary.with_extension("json");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(|_| "failure_pending_create_failed")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "failure_pending_write_failed")?;
        drop(file);
        fs::rename(&temporary, &published).map_err(|_| "failure_pending_publish_failed")?;
        sync_directory(&directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// One bounded batch and one nonblocking journal-lock attempt. Contention leaves
/// all pending records intact for the next background recovery tick.
pub(super) fn recover(journal: &Path) -> Result<(), String> {
    let directory = directory(journal);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("failure_pending_scan_failed".into()),
    };
    let paths = entries
        .filter_map(|entry| match entry {
            Ok(entry) if entry.path().extension().is_some_and(|ext| ext == "json") => {
                Some(Ok(entry.path()))
            }
            Ok(_) => None,
            Err(_) => Some(Err("failure_pending_entry_failed".to_string())),
        })
        .take(BATCH_SIZE)
        .collect::<Result<Vec<_>, _>>()?;
    if paths.is_empty() {
        return Ok(());
    }
    let mut options = OpenOptions::new();
    options.create(true).read(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(journal)
        .map_err(|_| "failure_pending_journal_open_failed")?;
    match file.try_lock() {
        Ok(()) => (),
        Err(std::fs::TryLockError::WouldBlock) => return Ok(()),
        Err(_) => return Err("failure_pending_journal_lock_failed".into()),
    }
    let mut pending = Vec::new();
    for path in paths {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("failure_pending_record_unavailable".into()),
        };
        if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
            return Err("failure_pending_record_invalid".into());
        }
        let record: ServiceFailureRecord =
            serde_json::from_slice(&fs::read(&path).map_err(|_| "failure_pending_read_failed")?)
                .map_err(|_| "failure_pending_decode_failed")?;
        pending.push((path, record));
    }
    let mut existing = HashMap::new();
    for line in BufReader::new(&file).lines() {
        let line = line.map_err(|_| "failure_pending_journal_read_failed")?;
        if let Ok(record) = serde_json::from_str::<ServiceFailureRecord>(&line) {
            if pending
                .iter()
                .any(|(_, candidate)| candidate.occurrence_id == record.occurrence_id)
            {
                if existing
                    .get(&record.occurrence_id)
                    .is_some_and(|previous| previous != &record)
                {
                    return Err("failure_pending_occurrence_conflict".into());
                }
                existing.insert(record.occurrence_id.clone(), record);
            }
        }
    }
    // A torn append remains diagnostic evidence, but must not swallow a new row.
    if file
        .metadata()
        .map_err(|_| "failure_pending_journal_stat_failed")?
        .len()
        > 0
    {
        file.seek(SeekFrom::End(-1))
            .map_err(|_| "failure_pending_journal_seek_failed")?;
        let mut last = [0];
        file.read_exact(&mut last)
            .map_err(|_| "failure_pending_journal_read_failed")?;
        if last[0] != b'\n' {
            file.write_all(b"\n")
                .map_err(|_| "failure_pending_journal_write_failed")?;
        }
    }
    for (_, record) in &pending {
        if let Some(previous) = existing.get(&record.occurrence_id) {
            if previous != record {
                return Err("failure_pending_occurrence_conflict".into());
            }
            continue;
        }
        let mut bytes = serde_json::to_vec(record).map_err(|_| "failure_pending_encode_failed")?;
        bytes.push(b'\n');
        file.write_all(&bytes)
            .map_err(|_| "failure_pending_journal_write_failed")?;
        existing.insert(record.occurrence_id.clone(), record.clone());
    }
    file.sync_all()
        .map_err(|_| "failure_pending_journal_sync_failed")?;
    sync_directory(journal.parent().ok_or("failure_pending_parent_missing")?)?;
    for (path, _) in pending {
        fs::remove_file(path).map_err(|_| "failure_pending_retirement_failed")?;
    }
    sync_directory(&directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_failure_journal::{
        append_service_failure_at, read_service_failures_at, ServiceFailureCategory,
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("failure-custody-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn journal(&self) -> PathBuf {
            self.0.join("failure-journal.jsonl")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn record() -> ServiceFailureRecord {
        ServiceFailureRecord::new(
            ServiceFailureCategory::ServiceAction,
            "test",
            "admission",
            "missing_action",
            "missing action",
        )
    }

    #[test]
    fn contention_preserves_synced_custody_and_recovery_without_resubmission() {
        let fixture = Fixture::new();
        let journal = fixture.journal();
        let lock = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&journal)
            .unwrap();
        lock.lock().unwrap();
        let record = record();
        stage(&journal, &record).unwrap();
        recover(&journal).unwrap();
        assert_eq!(count(&journal).unwrap(), 1);
        assert_eq!(fs::read(&journal).unwrap().len(), 0);
        assert!(read_service_failures_at(&journal, 10).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(directory(&journal))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            let path = fs::read_dir(directory(&journal))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(lock);
        recover(&journal).unwrap();
        assert_eq!(count(&journal).unwrap(), 0);
        assert_eq!(
            read_service_failures_at(&journal, 10).unwrap().records,
            vec![record]
        );
    }

    #[test]
    fn replay_after_append_before_retirement_does_not_duplicate_or_swallow_torn_line() {
        let fixture = Fixture::new();
        let journal = fixture.journal();
        let first = record();
        stage(&journal, &first).unwrap();
        append_service_failure_at(&journal, &first).unwrap();
        OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap()
            .write_all(b"{torn")
            .unwrap();
        let second = record();
        stage(&journal, &second).unwrap();
        recover(&journal).unwrap();
        recover(&journal).unwrap();
        let readback = read_service_failures_at(&journal, 10).unwrap();
        assert_eq!(readback.records, vec![first, second]);
        assert_eq!(readback.malformed_line_count, 1);
        assert_eq!(count(&journal).unwrap(), 0);
    }

    #[test]
    fn conflicting_occurrence_and_failed_persistence_are_explicit() {
        let fixture = Fixture::new();
        let journal = fixture.journal();
        let first = record();
        append_service_failure_at(&journal, &first).unwrap();
        let mut conflict = first.clone();
        conflict.code = "different_code".into();
        stage(&journal, &conflict).unwrap();
        assert_eq!(
            recover(&journal).unwrap_err(),
            "failure_pending_occurrence_conflict"
        );
        assert_eq!(count(&journal).unwrap(), 1);
        assert_eq!(
            read_service_failures_at(&journal, 10).unwrap().records,
            vec![first]
        );
        let invalid = fixture.0.join("file-parent");
        fs::write(&invalid, b"not a directory").unwrap();
        assert!(stage(&invalid.join("journal.jsonl"), &record()).is_err());
    }
}
