//! Read-only control reservation held until the owning SQLite commit finishes.
use super::Control;
use std::fs::File;
use std::path::Path;

pub(crate) struct RetirementGuard {
    _lock: File,
    epoch: u64,
}
impl RetirementGuard {
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }
}
pub(crate) fn retirement_guard(browser_id: &str) -> Result<RetirementGuard, String> {
    lock_at(&super::path(browser_id)?)
}
#[cfg(test)]
pub(crate) fn fixture_guard(path: &Path) -> Result<RetirementGuard, String> {
    lock_at(path)
}

fn lock_at(path: &Path) -> Result<RetirementGuard, String> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let held = || "browser_task_retirement_control_unverified".to_string();
    // SAFETY: geteuid has no arguments or failure path.
    let uid = unsafe { libc::geteuid() };
    let parent = path.parent().ok_or_else(held)?;
    let directory = std::fs::symlink_metadata(parent).map_err(|_| held())?;
    if !directory.is_dir()
        || directory.uid() != uid
        || directory.mode() & 0o022 != 0
        || std::fs::canonicalize(parent).map_err(|_| held())? != parent
    {
        return Err(held());
    }
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path.with_extension("lock"))
        .map_err(|_| held())?;
    let metadata = lock.metadata().map_err(|_| held())?;
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(held());
    }
    // Non-blocking: a concurrent owner keeps retirement held rather than hanging
    // while another path could need the SQLite writer. Caller can inspect later.
    lock.try_lock().map_err(|_| held())?;
    let mut state = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| held())?;
    let metadata = state.metadata().map_err(|_| held())?;
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
        || metadata.len() > 4 * 1024 * 1024
    {
        return Err(held());
    }
    let mut raw = Vec::new();
    state
        .by_ref()
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|_| held())?;
    if raw.len() > 4 * 1024 * 1024 {
        return Err(held());
    }
    let control: Control = serde_json::from_slice(&raw).map_err(|_| held())?;
    if control.human
        || control.awaiting_observation
        || control.flights.values().any(|s| s != "observed_completed")
    {
        return Err(held());
    }
    Ok(RetirementGuard {
        _lock: lock,
        epoch: control.epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn lost_task_retirement_control_lock_is_held_and_unknown_flights_refused() {
        let dir = std::env::temp_dir().join(format!("auth650-control-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let path = dir.join("control.json");
        let mut control = Control {
            epoch: 7,
            ..Default::default()
        };
        std::fs::write(&path, serde_json::to_vec(&control).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let original = std::fs::read(&path).unwrap();
        let guard = lock_at(&path).unwrap();
        assert_eq!(guard.epoch, 7);
        let competing = File::options()
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))
            .unwrap();
        assert!(competing.try_lock().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        drop(guard);
        competing.try_lock().unwrap();
        drop(competing);
        for flight in [
            "dispatched",
            "uncertain_in_flight",
            "uncertain_no_replay",
            "observed_uncertain_no_replay",
        ] {
            control.flights.insert("old-request".into(), flight.into());
            std::fs::write(&path, serde_json::to_vec(&control).unwrap()).unwrap();
            assert!(lock_at(&path).is_err());
        }
        control.flights.clear();
        control.human = true;
        std::fs::write(&path, serde_json::to_vec(&control).unwrap()).unwrap();
        assert!(lock_at(&path).is_err());
        control.human = false;
        control.awaiting_observation = true;
        std::fs::write(&path, serde_json::to_vec(&control).unwrap()).unwrap();
        assert!(lock_at(&path).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(lock_at(&path).is_err()); // Missing is not default authority.
    }
}
