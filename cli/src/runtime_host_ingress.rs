//! Atomic selection of the runtime host socket directory across upgrades.
//!
//! Candidate hosts use an explicit socket directory while they are observed.
//! Once their exact identity is proven, the upgrade transaction stages and
//! commits that backend here. Normal clients then resolve the selected backend
//! without requiring the old and candidate hosts to share mutable socket files.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(crate) const RUNTIME_HOST_INGRESS_SCHEMA_VERSION: &str =
    "agent-browser.runtime-host-ingress.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeHostTopology {
    SingleHost,
    LegacyPerSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeHostBackend {
    pub(crate) topology: RuntimeHostTopology,
    pub(crate) generation_id: String,
    pub(crate) socket_dir: PathBuf,
    pub(crate) binary_sha256: String,
    pub(crate) host_id: String,
    pub(crate) pid: u32,
    pub(crate) socket_identity: String,
}

impl RuntimeHostBackend {
    fn validate(&self) -> Result<(), String> {
        if self.generation_id.trim().is_empty()
            || !self.socket_dir.is_absolute()
            || self.binary_sha256.trim().is_empty()
            || self.host_id.trim().is_empty()
            || self.socket_identity.trim().is_empty()
        {
            return Err("runtime host backend identity is incomplete".to_string());
        }
        if self.topology == RuntimeHostTopology::SingleHost && self.pid == 0 {
            return Err("runtime host backend identity is incomplete".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeHostIngressRegistry {
    pub(crate) schema_version: String,
    pub(crate) revision: u64,
    pub(crate) active_transaction_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_transaction_id: Option<String>,
    selected_backend: RuntimeHostBackend,
    candidate_backend: Option<RuntimeHostBackend>,
    fallback_backend: Option<RuntimeHostBackend>,
}

impl RuntimeHostIngressRegistry {
    pub(crate) fn new(selected_backend: RuntimeHostBackend) -> Result<Self, String> {
        selected_backend.validate()?;
        Ok(Self {
            schema_version: RUNTIME_HOST_INGRESS_SCHEMA_VERSION.to_string(),
            revision: 1,
            active_transaction_id: None,
            selected_transaction_id: None,
            selected_backend,
            candidate_backend: None,
            fallback_backend: None,
        })
    }

    pub(crate) fn selected_backend(&self) -> &RuntimeHostBackend {
        &self.selected_backend
    }

    pub(crate) fn candidate_backend(&self) -> Option<&RuntimeHostBackend> {
        self.candidate_backend.as_ref()
    }

    pub(crate) fn fallback_backend(&self) -> Option<&RuntimeHostBackend> {
        self.fallback_backend.as_ref()
    }

    fn stage_candidate(
        &mut self,
        transaction_id: &str,
        candidate: RuntimeHostBackend,
    ) -> Result<(), String> {
        candidate.validate()?;
        if transaction_id.trim().is_empty() {
            return Err("runtime host ingress transaction ID is missing".to_string());
        }
        if self
            .active_transaction_id
            .as_deref()
            .is_some_and(|active| active != transaction_id)
        {
            return Err("another runtime host ingress transaction is active".to_string());
        }
        if candidate.topology != RuntimeHostTopology::SingleHost {
            return Err("runtime host candidate must use the single-host topology".to_string());
        }
        if candidate.generation_id == self.selected_backend.generation_id {
            return Err("runtime host candidate generation is already selected".to_string());
        }
        self.active_transaction_id = Some(transaction_id.to_string());
        self.candidate_backend = Some(candidate);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn commit_candidate(
        &mut self,
        transaction_id: &str,
        expected_generation: &str,
    ) -> Result<(), String> {
        self.require_transaction(transaction_id)?;
        let candidate = self
            .candidate_backend
            .take()
            .ok_or_else(|| "runtime host candidate is not staged".to_string())?;
        if candidate.generation_id != expected_generation {
            self.candidate_backend = Some(candidate);
            return Err("runtime host candidate generation changed before commit".to_string());
        }
        let prior = std::mem::replace(&mut self.selected_backend, candidate);
        self.fallback_backend = Some(prior);
        self.active_transaction_id = None;
        self.selected_transaction_id = Some(transaction_id.to_string());
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn rollback(&mut self, transaction_id: &str, expected_generation: &str) -> Result<(), String> {
        if self.active_transaction_id.as_deref() == Some(transaction_id) {
            let candidate = self
                .candidate_backend
                .as_ref()
                .ok_or_else(|| "runtime host candidate is not staged".to_string())?;
            if candidate.generation_id != expected_generation {
                return Err("runtime host candidate generation changed before rollback".to_string());
            }
            self.candidate_backend = None;
            self.active_transaction_id = None;
            self.revision = self.revision.saturating_add(1);
            return Ok(());
        }
        if self.active_transaction_id.is_some() {
            return Err("runtime host ingress transaction changed before rollback".to_string());
        }
        if self.selected_transaction_id.as_deref() != Some(transaction_id) {
            return Err("runtime host ingress transaction changed before rollback".to_string());
        }
        if self.selected_backend.generation_id != expected_generation {
            return Err("runtime host selected generation changed before rollback".to_string());
        }
        let fallback = self
            .fallback_backend
            .take()
            .ok_or_else(|| "runtime host rollback backend is missing".to_string())?;
        let failed = std::mem::replace(&mut self.selected_backend, fallback);
        self.fallback_backend = Some(failed);
        self.selected_transaction_id = None;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn recover_dead_selected_backend(
        &mut self,
        expected_selected_generation: &str,
        expected_selected_pid: u32,
        expected_fallback_generation: &str,
    ) -> Result<(), String> {
        if self.active_transaction_id.is_some() || self.candidate_backend.is_some() {
            return Err("runtime host ingress transaction changed before recovery".to_string());
        }
        if self.selected_backend.generation_id != expected_selected_generation
            || self.selected_backend.pid != expected_selected_pid
        {
            return Err("runtime host selected backend changed before recovery".to_string());
        }
        let fallback = self
            .fallback_backend
            .take()
            .ok_or_else(|| "runtime host recovery backend is missing".to_string())?;
        if fallback.generation_id != expected_fallback_generation {
            self.fallback_backend = Some(fallback);
            return Err("runtime host recovery backend changed".to_string());
        }
        let failed = std::mem::replace(&mut self.selected_backend, fallback);
        self.fallback_backend = Some(failed);
        self.selected_transaction_id = None;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn require_transaction(&self, transaction_id: &str) -> Result<(), String> {
        if self.active_transaction_id.as_deref() != Some(transaction_id) {
            return Err("runtime host ingress transaction changed".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeHostIngressRepository {
    path: PathBuf,
}

impl RuntimeHostIngressRepository {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn default_path() -> PathBuf {
        std::env::var_os("AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE")
            .map(PathBuf::from)
            .or_else(|| {
                dirs::home_dir().map(|home| {
                    home.join(".agent-browser")
                        .join("runtime-host-ingress.json")
                })
            })
            .unwrap_or_else(|| std::env::temp_dir().join("agent-browser-runtime-host-ingress.json"))
    }

    pub(crate) fn load(&self) -> Result<RuntimeHostIngressRegistry, String> {
        let _lock = acquire_lock(&self.path)?;
        load_registry(&self.path)
    }

    pub(crate) fn initialize(
        &self,
        selected_backend: RuntimeHostBackend,
    ) -> Result<RuntimeHostIngressRegistry, String> {
        let _lock = acquire_lock(&self.path)?;
        if self.path.exists() {
            return load_registry(&self.path);
        }
        let registry = RuntimeHostIngressRegistry::new(selected_backend)?;
        write_registry_atomic(&self.path, &registry)?;
        Ok(registry)
    }

    pub(crate) fn stage_candidate(
        &self,
        expected_revision: u64,
        transaction_id: &str,
        candidate: RuntimeHostBackend,
    ) -> Result<RuntimeHostIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            registry.stage_candidate(transaction_id, candidate)
        })
    }

    pub(crate) fn commit_candidate(
        &self,
        expected_revision: u64,
        transaction_id: &str,
        expected_generation: &str,
    ) -> Result<RuntimeHostIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            registry.commit_candidate(transaction_id, expected_generation)
        })
    }

    pub(crate) fn rollback(
        &self,
        expected_revision: u64,
        transaction_id: &str,
        expected_generation: &str,
    ) -> Result<RuntimeHostIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            registry.rollback(transaction_id, expected_generation)
        })
    }

    pub(crate) fn recover_dead_selected_backend(
        &self,
        expected_revision: u64,
        expected_selected_generation: &str,
        expected_selected_pid: u32,
        expected_fallback_generation: &str,
    ) -> Result<RuntimeHostIngressRegistry, String> {
        self.mutate(expected_revision, |registry| {
            registry.recover_dead_selected_backend(
                expected_selected_generation,
                expected_selected_pid,
                expected_fallback_generation,
            )
        })
    }

    fn mutate(
        &self,
        expected_revision: u64,
        mutator: impl FnOnce(&mut RuntimeHostIngressRegistry) -> Result<(), String>,
    ) -> Result<RuntimeHostIngressRegistry, String> {
        let _lock = acquire_lock(&self.path)?;
        let mut registry = load_registry(&self.path)?;
        require_revision(&registry, expected_revision)?;
        mutator(&mut registry)?;
        write_registry_atomic(&self.path, &registry)?;
        Ok(registry)
    }
}

pub(crate) fn selected_socket_dir() -> Option<PathBuf> {
    let repository =
        RuntimeHostIngressRepository::new(RuntimeHostIngressRepository::default_path());
    repository.load().ok().and_then(|registry| {
        (registry.selected_backend().topology == RuntimeHostTopology::SingleHost)
            .then(|| registry.selected_backend().socket_dir.clone())
    })
}

fn require_revision(
    registry: &RuntimeHostIngressRegistry,
    expected_revision: u64,
) -> Result<(), String> {
    if registry.revision != expected_revision {
        return Err(format!(
            "runtime host ingress revision changed: expected {expected_revision}, current {}",
            registry.revision
        ));
    }
    Ok(())
}

fn load_registry(path: &Path) -> Result<RuntimeHostIngressRegistry, String> {
    let body = fs::read(path).map_err(|error| {
        format!(
            "Unable to read runtime host ingress state {}: {error}",
            path.display()
        )
    })?;
    let registry: RuntimeHostIngressRegistry = serde_json::from_slice(&body).map_err(|error| {
        format!(
            "Unable to parse runtime host ingress state {}: {error}",
            path.display()
        )
    })?;
    if registry.schema_version != RUNTIME_HOST_INGRESS_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported runtime host ingress schema: {}",
            registry.schema_version
        ));
    }
    registry.selected_backend.validate()?;
    if let Some(candidate) = registry.candidate_backend.as_ref() {
        candidate.validate()?;
    }
    if let Some(fallback) = registry.fallback_backend.as_ref() {
        fallback.validate()?;
    }
    Ok(registry)
}

fn write_registry_atomic(path: &Path, registry: &RuntimeHostIngressRegistry) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "runtime host ingress state path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Unable to create runtime host ingress directory {}: {error}",
            parent.display()
        )
    })?;
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(registry)
        .map_err(|error| format!("Unable to serialize runtime host ingress state: {error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|error| format!("Unable to stage runtime host ingress state: {error}"))?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("Unable to persist runtime host ingress state: {error}"))?;
        set_private_file(&staged)?;
        fs::rename(&staged, path)
            .map_err(|error| format!("Unable to commit runtime host ingress state: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

fn acquire_lock(state_path: &Path) -> Result<File, String> {
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Unable to create runtime host ingress directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let lock_path = state_path.with_extension("json.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| format!("Unable to open runtime host ingress lock: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err("runtime_host_ingress_lock_timeout".to_string());
            }
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
            Err(error) => {
                return Err(format!(
                    "Unable to lock runtime host ingress state: {error}"
                ))
            }
        }
    }
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("Unable to protect runtime host ingress state: {error}"))
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(root: &Path, generation: &str, pid: u32) -> RuntimeHostBackend {
        RuntimeHostBackend {
            topology: RuntimeHostTopology::SingleHost,
            generation_id: generation.to_string(),
            socket_dir: root.join(generation),
            binary_sha256: format!("sha-{generation}"),
            host_id: format!("host-{generation}"),
            pid,
            socket_identity: format!("socket-{generation}"),
        }
    }

    fn legacy_backend(root: &Path) -> RuntimeHostBackend {
        RuntimeHostBackend {
            topology: RuntimeHostTopology::LegacyPerSession,
            generation_id: "legacy".to_string(),
            socket_dir: root.join("legacy-sockets"),
            binary_sha256: "b".repeat(64),
            host_id: "legacy-runtime-set:census".to_string(),
            pid: 0,
            socket_identity: "legacy-per-session-endpoints".to_string(),
        }
    }

    #[test]
    fn candidate_commit_and_rollback_are_transaction_and_revision_fenced() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-runtime-host-ingress-{}",
            uuid::Uuid::new_v4()
        ));
        let path = root.join("ingress.json");
        let repository = RuntimeHostIngressRepository::new(&path);
        let old = backend(&root, "old", 101);
        let candidate = backend(&root, "candidate", 202);

        let initial = repository.initialize(old.clone()).unwrap();
        let staged = repository
            .stage_candidate(initial.revision, "tx-1", candidate.clone())
            .unwrap();
        assert_eq!(staged.selected_backend(), &old);
        assert_eq!(staged.candidate_backend(), Some(&candidate));
        assert!(repository
            .commit_candidate(initial.revision, "tx-1", "candidate")
            .unwrap_err()
            .contains("revision changed"));
        assert!(repository
            .commit_candidate(staged.revision, "tx-2", "candidate")
            .unwrap_err()
            .contains("transaction changed"));

        let committed = repository
            .commit_candidate(staged.revision, "tx-1", "candidate")
            .unwrap();
        assert_eq!(committed.selected_backend(), &candidate);
        assert_eq!(committed.fallback_backend(), Some(&old));

        let rolled_back = repository
            .rollback(committed.revision, "tx-1", "candidate")
            .unwrap();
        assert_eq!(rolled_back.selected_backend(), &old);
        assert_eq!(rolled_back.fallback_backend(), Some(&candidate));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_committed_transaction_cannot_roll_back_a_newer_same_generation_commit() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-runtime-host-ingress-stale-rollback-{}",
            uuid::Uuid::new_v4()
        ));
        let repository = RuntimeHostIngressRepository::new(root.join("ingress.json"));
        let old = backend(&root, "old", 101);
        let candidate = backend(&root, "candidate", 202);

        let initial = repository.initialize(old.clone()).unwrap();
        let first_staged = repository
            .stage_candidate(initial.revision, "tx-1", candidate.clone())
            .unwrap();
        let first_committed = repository
            .commit_candidate(first_staged.revision, "tx-1", "candidate")
            .unwrap();
        let first_rolled_back = repository
            .rollback(first_committed.revision, "tx-1", "candidate")
            .unwrap();

        let second_staged = repository
            .stage_candidate(first_rolled_back.revision, "tx-2", candidate.clone())
            .unwrap();
        let second_committed = repository
            .commit_candidate(second_staged.revision, "tx-2", "candidate")
            .unwrap();
        let error = repository
            .rollback(second_committed.revision, "tx-1", "candidate")
            .unwrap_err();

        assert!(error.contains("transaction changed before rollback"));
        let current = repository.load().unwrap();
        assert_eq!(current.selected_backend(), &candidate);
        assert_eq!(current.fallback_backend(), Some(&old));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dead_selected_backend_can_restore_the_exact_fallback_before_restaging() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-runtime-host-ingress-dead-selected-{}",
            uuid::Uuid::new_v4()
        ));
        let repository = RuntimeHostIngressRepository::new(root.join("ingress.json"));
        let old = backend(&root, "old", 101);
        let candidate = backend(&root, "candidate", 202);

        let initial = repository.initialize(old.clone()).unwrap();
        let staged = repository
            .stage_candidate(initial.revision, "tx-1", candidate.clone())
            .unwrap();
        let committed = repository
            .commit_candidate(staged.revision, "tx-1", "candidate")
            .unwrap();
        let recovered = repository
            .recover_dead_selected_backend(committed.revision, "candidate", 202, "old")
            .unwrap();

        assert_eq!(recovered.selected_backend(), &old);
        assert_eq!(recovered.fallback_backend(), Some(&candidate));
        let restaged = repository
            .stage_candidate(recovered.revision, "tx-2", candidate.clone())
            .unwrap();
        assert_eq!(restaged.selected_backend(), &old);
        assert_eq!(restaged.candidate_backend(), Some(&candidate));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_staged_candidate_rollback_never_changes_selected_backend() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-runtime-host-ingress-stage-{}",
            uuid::Uuid::new_v4()
        ));
        let repository = RuntimeHostIngressRepository::new(root.join("ingress.json"));
        let old = backend(&root, "old", 101);
        let initial = repository.initialize(old.clone()).unwrap();
        let staged = repository
            .stage_candidate(initial.revision, "tx-1", backend(&root, "candidate", 202))
            .unwrap();
        let rolled_back = repository
            .rollback(staged.revision, "tx-1", "candidate")
            .unwrap();
        assert_eq!(rolled_back.selected_backend(), &old);
        assert!(rolled_back.candidate_backend().is_none());
        assert!(rolled_back.fallback_backend().is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn first_single_host_upgrade_can_commit_and_rollback_to_legacy_routing() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-runtime-host-ingress-legacy-{}",
            uuid::Uuid::new_v4()
        ));
        let repository = RuntimeHostIngressRepository::new(root.join("ingress.json"));
        let legacy = legacy_backend(&root);
        let candidate = backend(&root, "candidate", 202);
        let initial = repository.initialize(legacy.clone()).unwrap();
        let staged = repository
            .stage_candidate(initial.revision, "tx-legacy", candidate.clone())
            .unwrap();
        let committed = repository
            .commit_candidate(staged.revision, "tx-legacy", "candidate")
            .unwrap();
        assert_eq!(committed.selected_backend(), &candidate);
        assert_eq!(committed.fallback_backend(), Some(&legacy));
        let rolled_back = repository
            .rollback(committed.revision, "tx-legacy", "candidate")
            .unwrap();
        assert_eq!(rolled_back.selected_backend(), &legacy);
        assert_eq!(
            rolled_back.selected_backend().topology,
            RuntimeHostTopology::LegacyPerSession
        );
        let _ = fs::remove_dir_all(root);
    }
}
