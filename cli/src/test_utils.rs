use std::sync::{Mutex, MutexGuard};

/// Global mutex shared across all test modules to prevent parallel tests from
/// interfering with each other when mutating environment variables.
pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard that locks [`ENV_MUTEX`] and restores environment variables on drop.
pub struct EnvGuard<'a> {
    _lock: MutexGuard<'a, ()>,
    vars: Vec<(String, Option<String>)>,
}

impl<'a> EnvGuard<'a> {
    pub fn new(var_names: &[&str]) -> Self {
        let lock = ENV_MUTEX
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let vars = var_names
            .iter()
            .map(|&name| (name.to_string(), std::env::var(name).ok()))
            .collect();
        Self { _lock: lock, vars }
    }

    pub fn set(&self, name: &str, value: &str) {
        debug_assert!(
            self.vars.iter().any(|(n, _)| n == name),
            "EnvGuard::set called with unregistered var: {name}"
        );
        std::env::set_var(name, value);
    }

    pub fn remove(&self, name: &str) {
        debug_assert!(
            self.vars.iter().any(|(n, _)| n == name),
            "EnvGuard::remove called with unregistered var: {name}"
        );
        std::env::remove_var(name);
    }
}

impl Drop for EnvGuard<'_> {
    fn drop(&mut self) {
        for (name, value) in &self.vars {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Remove only a caller-owned disposable fixture root after its stores close.
/// Runtime custody seals directories; restore traversal/removal permissions only
/// on actual directories. Refuse a symlink root and never recurse into symlinks.
pub(crate) fn remove_owned_fixture_dir(root: &std::path::Path) -> std::io::Result<()> {
    fn unseal(path: &std::path::Path) -> std::io::Result<()> {
        if !std::fs::symlink_metadata(path)?.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "owned fixture root must be an actual directory",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                unseal(&entry.path())?;
            }
        }
        Ok(())
    }
    unseal(root)?;
    std::fs::remove_dir_all(root)
}
