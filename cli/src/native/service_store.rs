//! Persistent service-state storage.
//!
//! The first service-mode store is JSON-backed and intentionally small. It gives
//! later lifecycle work a durable contract without forcing a database choice yet.

use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, TryLockError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::runtime_owner_transfer::{RuntimeLifecycleRecord, RuntimeOwnerRegistry};

use super::service_model::{RemoteViewHandoff, ServiceState};

const SERVICE_DIR: &str = "service";
const SERVICE_STATE_FILENAME: &str = "state.json";
const REMOTE_VIEW_HANDOFFS_FILENAME: &str = "remote-view-handoffs.json";
const REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION: &str = "agent-browser.remote-view-handoffs.v1";
const REMOTE_VIEW_PRESENTATIONS_FILENAME: &str = "remote-view-presentations.json";
const REMOTE_VIEW_PRESENTATIONS_SCHEMA_VERSION: &str = "agent-browser.remote-view-presentations.v1";
const RUNTIME_OWNER_REGISTRY_FILENAME: &str = "runtime-owner-registry.json";
const RUNTIME_OWNER_REGISTRY_SCHEMA_VERSION: &str = "agent-browser.runtime-owner-registry.v1";
const RUNTIME_LIFECYCLE_REGISTRY_FILENAME: &str = "runtime-lifecycle-registry.json";
const RUNTIME_LIFECYCLE_REGISTRY_SCHEMA_VERSION: &str =
    "agent-browser.runtime-lifecycle-registry.v1";
static SERVICE_STATE_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static SERVICE_STATE_ACTIVE_MUTATION: OnceLock<Mutex<Option<&'static str>>> = OnceLock::new();
static SERVICE_STATE_LOCK_TELEMETRY: OnceLock<Mutex<ServiceStateLockTelemetryState>> =
    OnceLock::new();
static SERVICE_STATE_LOCK_TOKEN: AtomicU64 = AtomicU64::new(1);
const DEFAULT_SERVICE_STATE_LOCK_TIMEOUT: Duration = Duration::from_secs(1);
const SERVICE_STATE_LOCK_RECENT_CAPACITY: usize = 32;
pub(crate) const SERVICE_STATE_JSON_STACK_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateLockActivity {
    pub(crate) lock_kind: &'static str,
    pub(crate) operation: &'static str,
    pub(crate) mode: &'static str,
    pub(crate) phase: &'static str,
    pub(crate) wait_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hold_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateLockCounters {
    pub(crate) process_acquisitions: u64,
    pub(crate) file_acquisitions: u64,
    pub(crate) process_timeouts: u64,
    pub(crate) file_timeouts: u64,
    pub(crate) process_poison_recoveries: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateLockDiagnostics {
    pub(crate) schema_version: &'static str,
    pub(crate) recent_capacity: usize,
    pub(crate) active: Vec<ServiceStateLockActivity>,
    pub(crate) recent: Vec<ServiceStateLockActivity>,
    pub(crate) counters: ServiceStateLockCounters,
}

#[derive(Debug)]
struct ActiveServiceStateLock {
    token: u64,
    lock_kind: &'static str,
    operation: &'static str,
    mode: &'static str,
    acquired_at: Instant,
    wait_ms: u64,
}

#[derive(Debug, Default)]
struct ServiceStateLockTelemetryState {
    active: Vec<ActiveServiceStateLock>,
    recent: VecDeque<ServiceStateLockActivity>,
    counters: ServiceStateLockCounters,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoteViewHandoffRegistry {
    schema_version: String,
    handoffs: BTreeMap<String, RemoteViewHandoff>,
}

/// Upgrade-safe authority state stored outside the legacy-compatible primary
/// service snapshot. Older binaries can rewrite `state.json`, but they cannot
/// erase the current effect-capable owner generation from this sidecar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DurableRuntimeOwnerRegistry {
    schema_version: String,
    registry: RuntimeOwnerRegistry,
}

/// New lifecycle evidence is isolated from the legacy owner-registry shape so
/// an older runtime can keep serving during a hot upgrade. New readers merge
/// this sidecar under the same repository lock.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DurableRuntimeLifecycleRegistry {
    schema_version: String,
    registry_revision: u64,
    records: BTreeMap<String, RuntimeLifecycleRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStateTransaction {
    state_payload: String,
    handoff_payload: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner_registry_payload: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lifecycle_registry_payload: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServiceStateSaveBoundary {
    HandoffWrite,
    OwnerRegistryWrite,
    LifecycleRegistryWrite,
    StateWrite,
    HandoffRename,
    OwnerRegistryRename,
    LifecycleRegistryRename,
    StateRename,
}

pub trait ServiceStateStore {
    fn load(&self) -> Result<ServiceState, String>;
    fn save(&self, state: &ServiceState) -> Result<(), String>;

    /// Load while the repository already holds a lock that guarantees no
    /// transaction is in progress. Stores without transactional recovery may
    /// use their normal load implementation.
    fn load_without_recovery(&self) -> Result<ServiceState, String> {
        self.load()
    }

    fn recovery_required(&self) -> bool {
        false
    }

    fn state_path(&self) -> Option<&Path> {
        None
    }

    /// Prepare a pure durable transaction without acquiring mutation
    /// authority. Stores that return `Some` must commit the exact payload in
    /// `save_prepared` without re-running serialization or derivation.
    fn prepare_save(
        &self,
        _state: &ServiceState,
    ) -> Result<Option<ServiceStateTransaction>, String> {
        Ok(None)
    }

    fn supports_prepared_save(&self) -> bool {
        false
    }

    fn save_prepared(&self, _transaction: &ServiceStateTransaction) -> Result<(), String> {
        Err("service_state_prepared_save_unsupported".to_string())
    }
}

pub trait ServiceStateRepository {
    fn load_snapshot(&self) -> Result<ServiceState, String>;
    fn mutate<R>(
        &self,
        mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
    ) -> Result<R, String>;
}

#[derive(Debug, Clone)]
pub struct JsonServiceStateStore {
    path: PathBuf,
    save_fault: Option<Arc<Mutex<Option<ServiceStateSaveBoundary>>>>,
}

#[derive(Debug, Clone)]
pub struct LockedServiceStateRepository<S> {
    store: S,
    lock_timeout: Duration,
}

impl JsonServiceStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            save_fault: None,
        }
    }

    pub fn default_path() -> Result<PathBuf, String> {
        default_service_state_path()
    }

    #[cfg(test)]
    fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    fn with_save_fault(path: impl Into<PathBuf>, boundary: ServiceStateSaveBoundary) -> Self {
        Self {
            path: path.into(),
            save_fault: Some(Arc::new(Mutex::new(Some(boundary)))),
        }
    }

    fn fail_at(&self, boundary: ServiceStateSaveBoundary) -> Result<(), String> {
        let Some(fault) = self.save_fault.as_ref() else {
            return Ok(());
        };
        let mut fault = fault
            .lock()
            .map_err(|_| "Service state save fault lock was poisoned".to_string())?;
        if fault.as_ref() == Some(&boundary) {
            *fault = None;
            return Err(format!("injected_service_state_save_failure:{boundary:?}"));
        }
        Ok(())
    }
}

impl<S> LockedServiceStateRepository<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            lock_timeout: DEFAULT_SERVICE_STATE_LOCK_TIMEOUT,
        }
    }
}

impl<S> LockedServiceStateRepository<S>
where
    S: Clone,
{
    pub(crate) fn with_lock_timeout(&self, lock_timeout: Duration) -> Self {
        Self {
            store: self.store.clone(),
            lock_timeout,
        }
    }
}

impl LockedServiceStateRepository<JsonServiceStateStore> {
    pub fn default_json() -> Result<Self, String> {
        Ok(Self::new(JsonServiceStateStore::new(
            default_service_state_path()?,
        )))
    }
}

impl ServiceStateStore for JsonServiceStateStore {
    fn load(&self) -> Result<ServiceState, String> {
        recover_service_state_transaction(&self.path)?;
        self.load_without_recovery()
    }

    fn load_without_recovery(&self) -> Result<ServiceState, String> {
        let mut state_file_missing = false;
        let mut state = match fs::read_to_string(&self.path) {
            Ok(raw) => parse_service_state_json(raw, &self.path)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                state_file_missing = true;
                ServiceState::default()
            }
            Err(err) => {
                return Err(format!(
                    "Failed to read service state {}: {}",
                    self.path.display(),
                    err
                ))
            }
        };

        let handoff_registry = load_remote_view_handoff_registry(&self.path)?;
        let presentation_registry = load_remote_view_presentation_registry(&self.path)?;
        let owner_registry = load_runtime_owner_registry(&self.path)?;
        let lifecycle_registry = load_runtime_lifecycle_registry(&self.path)?;
        if state_file_missing
            && handoff_registry.handoffs.is_empty()
            && presentation_registry.handoffs.is_empty()
            && owner_registry.schema_version.is_empty()
            && lifecycle_registry.schema_version.is_empty()
        {
            return Ok(ServiceState::default());
        }
        state.remote_view_handoffs.extend(handoff_registry.handoffs);
        state.remote_view_handoffs = merge_remote_view_handoff_registries(
            presentation_registry,
            RemoteViewHandoffRegistry {
                schema_version: REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION.to_string(),
                handoffs: state.remote_view_handoffs,
            },
        )
        .handoffs;
        if !owner_registry.schema_version.is_empty() {
            state.runtime_owner_registry = owner_registry.registry;
        }
        if !lifecycle_registry.schema_version.is_empty() {
            state.runtime_owner_registry.lifecycle_records = lifecycle_registry.records;
        }
        state.mark_persisted_entity_sources();
        super::presentation_inventory::overlay_provider_inventory_from_environment(&mut state)?;
        state.refresh_derived_views();
        Ok(state)
    }

    fn recovery_required(&self) -> bool {
        service_state_transaction_path(&self.path).exists()
    }

    fn save(&self, state: &ServiceState) -> Result<(), String> {
        let transaction = prepare_service_state_transaction(state)?;
        commit_service_state_transaction(self, &transaction)
    }

    fn state_path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn prepare_save(
        &self,
        state: &ServiceState,
    ) -> Result<Option<ServiceStateTransaction>, String> {
        prepare_service_state_transaction(state).map(Some)
    }

    fn supports_prepared_save(&self) -> bool {
        true
    }

    fn save_prepared(&self, transaction: &ServiceStateTransaction) -> Result<(), String> {
        commit_service_state_transaction(self, transaction)
    }
}

fn parse_service_state_json(raw: String, path: &Path) -> Result<ServiceState, String> {
    let display_path = path.display().to_string();
    // Large service histories can exhaust a Tokio worker's comparatively small stack
    // inside serde_json. Keep that recursive work on an explicitly bounded stack.
    std::thread::Builder::new()
        .name("service-state-json".to_string())
        .stack_size(SERVICE_STATE_JSON_STACK_BYTES)
        .spawn(move || super::service_state_migration::read_service_state(&raw))
        .map_err(|err| {
            format!(
                "Failed to start service state JSON parser for {}: {}",
                display_path, err
            )
        })?
        .join()
        .map_err(|_| format!("Service state JSON parser panicked for {display_path}"))?
        .map_err(|err| format!("Invalid service state JSON {display_path}: {err}"))
}

fn prepare_service_state_transaction(
    state: &ServiceState,
) -> Result<ServiceStateTransaction, String> {
    // Clone, derived-view refresh, and serialization can recurse through the same
    // large state tree, so the complete preparation boundary uses the JSON stack.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("service-state-json".to_string())
            .stack_size(SERVICE_STATE_JSON_STACK_BYTES)
            .spawn_scoped(scope, move || {
                let mut state = state.clone();
                super::service_state_migration::prepare_service_state_for_persistence(&mut state)?;
                state.refresh_derived_views();
                state.remove_builtin_entity_defaults_for_persistence();
                let lifecycle_registry_payload =
                    runtime_lifecycle_registry_payload(&state.runtime_owner_registry)?;
                state.runtime_owner_registry.lifecycle_records.clear();
                let serialized = serde_json::to_string_pretty(&state)
                    .map_err(|err| format!("Failed to serialize service state: {err}"))?;
                Ok(ServiceStateTransaction {
                    state_payload: format!("{serialized}\n"),
                    handoff_payload: remote_view_handoff_registry_payload(
                        &state.remote_view_handoffs,
                    )?,
                    owner_registry_payload: Some(runtime_owner_registry_payload(
                        &state.runtime_owner_registry,
                    )?),
                    lifecycle_registry_payload: Some(lifecycle_registry_payload),
                })
            })
            .map_err(|err| format!("Failed to start service state JSON serializer: {err}"))?
            .join()
            .map_err(|_| "Service state JSON serializer panicked".to_string())?
    })
}

/// Decode an already parsed Service State value on the same bounded stack used
/// for disk JSON. Runtime-host commands can carry large state snapshots, and
/// serde recursion must not consume a Tokio worker's smaller stack.
pub(crate) fn decode_service_state_value(
    value: &serde_json::Value,
) -> Result<ServiceState, String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("service-state-json".to_string())
            .stack_size(SERVICE_STATE_JSON_STACK_BYTES)
            .spawn_scoped(scope, move || {
                serde_json::from_value::<ServiceState>(value.clone())
                    .map_err(|err| format!("Invalid serviceState: {err}"))
            })
            .map_err(|err| format!("Failed to start serviceState decoder: {err}"))?
            .join()
            .map_err(|_| "ServiceState decoder panicked".to_string())?
    })
}

impl<S> ServiceStateRepository for LockedServiceStateRepository<S>
where
    S: ServiceStateStore,
{
    fn load_snapshot(&self) -> Result<ServiceState, String> {
        self.load_snapshot_with_lock_timeout(self.lock_timeout)
    }

    fn mutate<R>(
        &self,
        mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
    ) -> Result<R, String> {
        self.mutate_with_lock_timeout(self.lock_timeout, mutator)
    }
}

impl<S> LockedServiceStateRepository<S>
where
    S: ServiceStateStore,
{
    pub(crate) fn load_snapshot_with_lock_timeout(
        &self,
        timeout: Duration,
    ) -> Result<ServiceState, String> {
        let deadline = Instant::now() + timeout.max(Duration::from_millis(1));
        let Some(path) = self.store.state_path() else {
            let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
            let _guard = acquire_service_state_process_lock(lock, deadline, "snapshot")?;
            return self.store.load();
        };

        let file_guard = acquire_service_state_file_lock_until(
            path,
            ServiceStateFileLockMode::Shared,
            deadline,
            "snapshot",
        )?;
        if !self.store.recovery_required() {
            return self.store.load_without_recovery();
        }

        drop(file_guard);
        let _file_guard = acquire_service_state_file_lock_until(
            path,
            ServiceStateFileLockMode::Exclusive,
            deadline,
            "transaction_recovery",
        )?;
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = acquire_service_state_process_lock(lock, deadline, "transaction_recovery")?;
        self.store.load()
    }

    pub(crate) fn mutate_with_lock_timeout<R>(
        &self,
        timeout: Duration,
        mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
    ) -> Result<R, String> {
        let deadline = Instant::now() + timeout.max(Duration::from_millis(1));
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        if self.store.supports_prepared_save() {
            let baseline = self.load_snapshot_with_lock_timeout(timeout)?;
            let baseline_revision = baseline.state_revision;
            let mut candidate = baseline;
            candidate.state_revision = baseline_revision
                .checked_add(1)
                .ok_or_else(|| "service_state_revision_exhausted".to_string())?;
            let result = mutator(&mut candidate)?;
            let transaction = self
                .store
                .prepare_save(&candidate)?
                .ok_or_else(|| "service_state_prepared_save_missing".to_string())?;
            let path = self
                .store
                .state_path()
                .ok_or_else(|| "service_state_prepared_save_path_missing".to_string())?;
            let commit_deadline = Instant::now() + timeout.max(Duration::from_millis(1));
            let _file_guard = acquire_service_state_file_lock_until(
                path,
                ServiceStateFileLockMode::Exclusive,
                commit_deadline,
                "commit",
            )?;
            let process_guard =
                acquire_service_state_process_lock(lock, commit_deadline, "commit")?;
            let current = if self.store.recovery_required() {
                self.store.load()?
            } else {
                self.store.load_without_recovery()?
            };
            if current.state_revision != baseline_revision {
                return Err(format!(
                    "service_state_stale_revision: expected={baseline_revision}; actual={}",
                    current.state_revision
                ));
            }
            drop(process_guard);
            self.store.save_prepared(&transaction)?;
            return Ok(result);
        }

        if let Some(path) = self.store.state_path() {
            let _file_guard = acquire_service_state_file_lock_until(
                path,
                ServiceStateFileLockMode::Exclusive,
                deadline,
                "mutate",
            )?;
            let process_guard = acquire_service_state_process_lock(lock, deadline, "mutate")?;
            let mut state = self.store.load()?;
            let result = mutator(&mut state)?;
            drop(process_guard);
            self.store.save(&state)?;
            return Ok(result);
        }

        let _process_guard = acquire_service_state_process_lock(lock, deadline, "mutate")?;
        let mut state = self.store.load()?;
        let result = mutator(&mut state)?;
        self.store.save(&state)?;
        Ok(result)
    }
}

fn acquire_service_state_process_lock(
    lock: &'static Mutex<()>,
    deadline: Instant,
    operation: &'static str,
) -> Result<ServiceStateProcessGuard, String> {
    let started = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(guard) => {
                return Ok(service_state_process_guard(
                    guard,
                    operation,
                    started.elapsed(),
                ));
            }
            Err(TryLockError::Poisoned(poisoned)) => {
                record_service_state_lock_terminal(
                    "process",
                    operation,
                    "exclusive",
                    "poison_recovered",
                    started.elapsed(),
                );
                with_service_state_lock_telemetry(|telemetry| {
                    telemetry.counters.process_poison_recoveries += 1;
                });
                lock.clear_poison();
                return Ok(service_state_process_guard(
                    poisoned.into_inner(),
                    operation,
                    started.elapsed(),
                ));
            }
            Err(TryLockError::WouldBlock) if Instant::now() >= deadline => {
                let holder = SERVICE_STATE_ACTIVE_MUTATION
                    .get_or_init(|| Mutex::new(None))
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .unwrap_or("unknown");
                record_service_state_lock_timeout(
                    "process",
                    operation,
                    "exclusive",
                    started.elapsed(),
                );
                return Err(format!(
                    "service_state_lock_timeout: process mutation lock; waited_ms={}; holder_operation={holder}",
                    started.elapsed().as_millis()
                ));
            }
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
        }
    }
}

fn service_state_process_guard(
    guard: MutexGuard<'static, ()>,
    operation: &'static str,
    wait: Duration,
) -> ServiceStateProcessGuard {
    *SERVICE_STATE_ACTIVE_MUTATION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(operation);
    let token = record_service_state_lock_acquired("process", operation, "exclusive", wait);
    ServiceStateProcessGuard { guard, token }
}

struct ServiceStateProcessGuard {
    #[allow(dead_code)]
    guard: MutexGuard<'static, ()>,
    token: u64,
}

impl Drop for ServiceStateProcessGuard {
    fn drop(&mut self) {
        record_service_state_lock_released(self.token);
        *SERVICE_STATE_ACTIVE_MUTATION
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

fn elapsed_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn with_service_state_lock_telemetry<R>(
    update: impl FnOnce(&mut ServiceStateLockTelemetryState) -> R,
) -> R {
    let mut telemetry = SERVICE_STATE_LOCK_TELEMETRY
        .get_or_init(|| Mutex::new(ServiceStateLockTelemetryState::default()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    update(&mut telemetry)
}

fn push_recent_service_state_lock_activity(
    telemetry: &mut ServiceStateLockTelemetryState,
    activity: ServiceStateLockActivity,
) {
    if telemetry.recent.len() == SERVICE_STATE_LOCK_RECENT_CAPACITY {
        telemetry.recent.pop_front();
    }
    telemetry.recent.push_back(activity);
}

fn record_service_state_lock_acquired(
    lock_kind: &'static str,
    operation: &'static str,
    mode: &'static str,
    wait: Duration,
) -> u64 {
    let token = SERVICE_STATE_LOCK_TOKEN.fetch_add(1, Ordering::Relaxed);
    with_service_state_lock_telemetry(|telemetry| {
        if lock_kind == "process" {
            telemetry.counters.process_acquisitions += 1;
        } else {
            telemetry.counters.file_acquisitions += 1;
        }
        telemetry.active.push(ActiveServiceStateLock {
            token,
            lock_kind,
            operation,
            mode,
            acquired_at: Instant::now(),
            wait_ms: elapsed_millis(wait),
        });
    });
    token
}

fn record_service_state_lock_released(token: u64) {
    with_service_state_lock_telemetry(|telemetry| {
        let Some(index) = telemetry
            .active
            .iter()
            .position(|active| active.token == token)
        else {
            return;
        };
        let active = telemetry.active.remove(index);
        let hold_ms = elapsed_millis(active.acquired_at.elapsed());
        push_recent_service_state_lock_activity(
            telemetry,
            ServiceStateLockActivity {
                lock_kind: active.lock_kind,
                operation: active.operation,
                mode: active.mode,
                phase: "released",
                wait_ms: active.wait_ms,
                hold_ms: Some(hold_ms),
            },
        );
    });
}

fn record_service_state_lock_timeout(
    lock_kind: &'static str,
    operation: &'static str,
    mode: &'static str,
    wait: Duration,
) {
    with_service_state_lock_telemetry(|telemetry| {
        if lock_kind == "process" {
            telemetry.counters.process_timeouts += 1;
        } else {
            telemetry.counters.file_timeouts += 1;
        }
        push_recent_service_state_lock_activity(
            telemetry,
            ServiceStateLockActivity {
                lock_kind,
                operation,
                mode,
                phase: "timeout",
                wait_ms: elapsed_millis(wait),
                hold_ms: None,
            },
        );
    });
}

fn record_service_state_lock_terminal(
    lock_kind: &'static str,
    operation: &'static str,
    mode: &'static str,
    phase: &'static str,
    wait: Duration,
) {
    with_service_state_lock_telemetry(|telemetry| {
        push_recent_service_state_lock_activity(
            telemetry,
            ServiceStateLockActivity {
                lock_kind,
                operation,
                mode,
                phase,
                wait_ms: elapsed_millis(wait),
                hold_ms: None,
            },
        );
    });
}

/// Returns bounded process-local lock diagnostics without reading or writing
/// the durable Service State repository.
pub(crate) fn service_state_lock_diagnostics() -> ServiceStateLockDiagnostics {
    let now = Instant::now();
    with_service_state_lock_telemetry(|telemetry| ServiceStateLockDiagnostics {
        schema_version: "agent-browser.service-state-lock-diagnostics.v1",
        recent_capacity: SERVICE_STATE_LOCK_RECENT_CAPACITY,
        active: telemetry
            .active
            .iter()
            .map(|active| ServiceStateLockActivity {
                lock_kind: active.lock_kind,
                operation: active.operation,
                mode: active.mode,
                phase: "holding",
                wait_ms: active.wait_ms,
                hold_ms: Some(elapsed_millis(now.duration_since(active.acquired_at))),
            })
            .collect(),
        recent: telemetry.recent.iter().cloned().collect(),
        counters: telemetry.counters.clone(),
    })
}

pub fn default_service_state_path() -> Result<PathBuf, String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Could not determine home directory for service state".to_string());
    };
    #[cfg(test)]
    let home = if std::env::var("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME")
        .ok()
        .as_deref()
        != Some("1")
    {
        static TEST_HOME: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        TEST_HOME
            .get_or_init(|| {
                let root = std::env::temp_dir().join(format!(
                    "agent-browser-test-service-home-{}",
                    std::process::id()
                ));
                let _ = fs::create_dir_all(&root);
                root
            })
            .clone()
    } else {
        home
    };
    Ok(home
        .join(".agent-browser")
        .join(SERVICE_DIR)
        .join(SERVICE_STATE_FILENAME))
}

/// Load a stable point-in-time snapshot of the default JSON service state.
///
/// Readers take a shared cross-process file lock and do not take the process
/// mutation mutex. A snapshot therefore resolves to one complete durable
/// revision without queuing behind mutation-only preparation. This does not
/// make the snapshot live after it is returned; callers that later write must
/// still use revision-aware mutation helpers.
pub fn load_default_service_state_snapshot() -> Result<ServiceState, String> {
    LockedServiceStateRepository::default_json()?.load_snapshot()
}

/// Serialize read-modify-write operations against the default JSON service state.
///
/// The JSON store is intentionally simple, but callers that mutate state must
/// not race independent load/save cycles. This helper provides the narrow
/// service-state control point used by queued service mutations and job audit
/// updates until a dedicated service database exists.
pub fn mutate_default_service_state<R>(
    mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
) -> Result<R, String> {
    LockedServiceStateRepository::default_json()?.mutate(mutator)
}

fn temp_state_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SERVICE_STATE_FILENAME);
    path.with_file_name(format!("{}.tmp.{}", file_name, std::process::id()))
}

fn remote_view_handoff_registry_path(state_path: &Path) -> PathBuf {
    state_path.with_file_name(REMOTE_VIEW_HANDOFFS_FILENAME)
}

fn remote_view_presentation_registry_path(state_path: &Path) -> PathBuf {
    state_path.with_file_name(REMOTE_VIEW_PRESENTATIONS_FILENAME)
}

fn runtime_owner_registry_path(state_path: &Path) -> PathBuf {
    state_path.with_file_name(RUNTIME_OWNER_REGISTRY_FILENAME)
}

fn runtime_lifecycle_registry_path(state_path: &Path) -> PathBuf {
    state_path.with_file_name(RUNTIME_LIFECYCLE_REGISTRY_FILENAME)
}

fn load_remote_view_handoff_registry(
    state_path: &Path,
) -> Result<RemoteViewHandoffRegistry, String> {
    let path = remote_view_handoff_registry_path(state_path);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RemoteViewHandoffRegistry::default())
        }
        Err(err) => {
            return Err(format!(
                "Failed to read remote-view handoff registry {}: {}",
                path.display(),
                err
            ))
        }
    };
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Invalid remote-view handoff registry JSON {}: {}",
            path.display(),
            err
        )
    })
}

fn load_remote_view_presentation_registry(
    state_path: &Path,
) -> Result<RemoteViewHandoffRegistry, String> {
    let path = remote_view_presentation_registry_path(state_path);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RemoteViewHandoffRegistry::default())
        }
        Err(err) => {
            return Err(format!(
                "Failed to read remote-view presentation registry {}: {}",
                path.display(),
                err
            ))
        }
    };
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Invalid remote-view presentation registry JSON {}: {}",
            path.display(),
            err
        )
    })
}

fn remote_view_handoff_registry_payload(
    handoffs: &BTreeMap<String, RemoteViewHandoff>,
) -> Result<String, String> {
    let registry = RemoteViewHandoffRegistry {
        schema_version: REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION.to_string(),
        handoffs: handoffs.clone(),
    };
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&registry)
            .map_err(|err| format!("Failed to serialize remote-view handoff registry: {err}"))?
    ))
}

/// Preserve the highest durable presentation generation when a writer prepared
/// its state snapshot before another process completed handoff resolution.
fn merge_remote_view_handoff_registries(
    current: RemoteViewHandoffRegistry,
    mut incoming: RemoteViewHandoffRegistry,
) -> RemoteViewHandoffRegistry {
    for (handoff_id, current_handoff) in current.handoffs {
        let current_generation = current_handoff
            .presentation_receipt
            .as_ref()
            .map_or(0, |receipt| receipt.generation);
        let incoming_generation = incoming
            .handoffs
            .get(&handoff_id)
            .and_then(|handoff| handoff.presentation_receipt.as_ref())
            .map_or(0, |receipt| receipt.generation);
        if !incoming.handoffs.contains_key(&handoff_id) || current_generation > incoming_generation
        {
            incoming.handoffs.insert(handoff_id, current_handoff);
        }
    }
    if incoming.schema_version.is_empty() {
        incoming.schema_version = REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION.to_string();
    }
    incoming
}

fn merge_current_remote_view_handoff_registry(
    state_path: &Path,
    transaction: &mut ServiceStateTransaction,
) -> Result<(), String> {
    let current = load_remote_view_handoff_registry(state_path)?;
    let incoming: RemoteViewHandoffRegistry = serde_json::from_str(&transaction.handoff_payload)
        .map_err(|error| format!("Invalid prepared remote-view handoff registry JSON: {error}"))?;
    let merged = merge_remote_view_handoff_registries(current, incoming);
    transaction.handoff_payload = format!(
        "{}\n",
        serde_json::to_string_pretty(&merged).map_err(|error| {
            format!("Failed to serialize merged remote-view handoff registry: {error}")
        })?
    );
    Ok(())
}

/// Persist ready presentation generations in a sidecar unknown to older
/// binaries. The registry is monotonic by generation, so a legacy writer can
/// rewrite the compatibility handoff file without erasing newer ready proof.
fn persist_durable_remote_view_presentations(
    state_path: &Path,
    transaction: &ServiceStateTransaction,
) -> Result<(), String> {
    let current = load_remote_view_presentation_registry(state_path)?;
    let incoming: RemoteViewHandoffRegistry = serde_json::from_str(&transaction.handoff_payload)
        .map_err(|error| format!("Invalid prepared remote-view handoff registry JSON: {error}"))?;
    let receipted = RemoteViewHandoffRegistry {
        schema_version: REMOTE_VIEW_PRESENTATIONS_SCHEMA_VERSION.to_string(),
        handoffs: incoming
            .handoffs
            .into_iter()
            .filter(|(_, handoff)| handoff.presentation_receipt.is_some())
            .collect(),
    };
    let mut merged = merge_remote_view_handoff_registries(current, receipted);
    if merged.handoffs.is_empty() {
        return Ok(());
    }
    merged.schema_version = REMOTE_VIEW_PRESENTATIONS_SCHEMA_VERSION.to_string();
    let payload = format!(
        "{}\n",
        serde_json::to_string_pretty(&merged).map_err(|error| {
            format!("Failed to serialize remote-view presentation registry: {error}")
        })?
    );
    let path = remote_view_presentation_registry_path(state_path);
    let temporary = write_temporary(&path, &payload, "remote-view presentation registry")?;
    replace_from_temporary(&temporary, &path, "remote-view presentation registry")
}

fn load_runtime_owner_registry(state_path: &Path) -> Result<DurableRuntimeOwnerRegistry, String> {
    let path = runtime_owner_registry_path(state_path);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DurableRuntimeOwnerRegistry::default())
        }
        Err(err) => {
            return Err(format!(
                "Failed to read runtime-owner registry {}: {}",
                path.display(),
                err
            ))
        }
    };
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Invalid runtime-owner registry JSON {}: {}",
            path.display(),
            err
        )
    })
}

fn runtime_owner_registry_payload(registry: &RuntimeOwnerRegistry) -> Result<String, String> {
    let registry = DurableRuntimeOwnerRegistry {
        schema_version: RUNTIME_OWNER_REGISTRY_SCHEMA_VERSION.to_string(),
        registry: registry.clone(),
    };
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&registry)
            .map_err(|err| format!("Failed to serialize runtime-owner registry: {err}"))?
    ))
}

fn load_runtime_lifecycle_registry(
    state_path: &Path,
) -> Result<DurableRuntimeLifecycleRegistry, String> {
    let path = runtime_lifecycle_registry_path(state_path);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DurableRuntimeLifecycleRegistry::default())
        }
        Err(err) => {
            return Err(format!(
                "Failed to read runtime-lifecycle registry {}: {}",
                path.display(),
                err
            ))
        }
    };
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Invalid runtime-lifecycle registry JSON {}: {}",
            path.display(),
            err
        )
    })
}

fn runtime_lifecycle_registry_payload(registry: &RuntimeOwnerRegistry) -> Result<String, String> {
    let registry = DurableRuntimeLifecycleRegistry {
        schema_version: RUNTIME_LIFECYCLE_REGISTRY_SCHEMA_VERSION.to_string(),
        registry_revision: registry.revision,
        records: registry.lifecycle_records.clone(),
    };
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&registry)
            .map_err(|err| format!("Failed to serialize runtime-lifecycle registry: {err}"))?
    ))
}

fn service_state_transaction_path(state_path: &Path) -> PathBuf {
    let file_name = state_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SERVICE_STATE_FILENAME);
    state_path.with_file_name(format!("{file_name}.transaction.json"))
}

fn write_temporary(path: &Path, payload: &str, label: &str) -> Result<PathBuf, String> {
    let temp_path = temp_state_path(path);
    fs::write(&temp_path, payload).map_err(|error| {
        format!(
            "Failed to write temporary {label} {}: {error}",
            temp_path.display()
        )
    })?;
    Ok(temp_path)
}

fn replace_from_temporary(temp_path: &Path, path: &Path, label: &str) -> Result<(), String> {
    fs::rename(temp_path, path)
        .map_err(|error| format!("Failed to replace {label} {}: {error}", path.display()))
}

fn restore_file(path: &Path, prior: Option<&[u8]>) -> Result<(), String> {
    match prior {
        Some(payload) => fs::write(path, payload)
            .map_err(|error| format!("Failed to restore {}: {error}", path.display())),
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("Failed to remove {}: {error}", path.display())),
        },
    }
}

fn clear_service_state_transaction(transaction_path: &Path) -> Result<(), String> {
    match fs::remove_file(transaction_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Failed to clear service state transaction {}: {error}",
            transaction_path.display()
        )),
    }
}

fn recover_service_state_transaction(state_path: &Path) -> Result<(), String> {
    let transaction_path = service_state_transaction_path(state_path);
    let raw = match fs::read_to_string(&transaction_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to read service state transaction {}: {error}",
                transaction_path.display()
            ))
        }
    };
    let transaction: ServiceStateTransaction = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Invalid service state transaction {}: {error}",
            transaction_path.display()
        )
    })?;
    let handoff_path = remote_view_handoff_registry_path(state_path);
    let handoff_temp = write_temporary(
        &handoff_path,
        &transaction.handoff_payload,
        "remote-view handoff registry",
    )?;
    let owner_registry_path = runtime_owner_registry_path(state_path);
    let owner_registry_temp = match transaction
        .owner_registry_payload
        .as_deref()
        .map(|payload| write_temporary(&owner_registry_path, payload, "runtime-owner registry"))
        .transpose()
    {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&handoff_temp);
            return Err(error);
        }
    };
    let lifecycle_registry_path = runtime_lifecycle_registry_path(state_path);
    let lifecycle_registry_temp = match transaction
        .lifecycle_registry_payload
        .as_deref()
        .map(|payload| {
            write_temporary(
                &lifecycle_registry_path,
                payload,
                "runtime-lifecycle registry",
            )
        })
        .transpose()
    {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&handoff_temp);
            if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
                let _ = fs::remove_file(owner_registry_temp);
            }
            return Err(error);
        }
    };
    let state_temp = match write_temporary(state_path, &transaction.state_payload, "service state")
    {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&handoff_temp);
            if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
                let _ = fs::remove_file(owner_registry_temp);
            }
            if let Some(lifecycle_registry_temp) = lifecycle_registry_temp.as_ref() {
                let _ = fs::remove_file(lifecycle_registry_temp);
            }
            return Err(error);
        }
    };
    replace_from_temporary(&handoff_temp, &handoff_path, "remote-view handoff registry")?;
    if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
        replace_from_temporary(
            owner_registry_temp,
            &owner_registry_path,
            "runtime-owner registry",
        )?;
    }
    if let Some(lifecycle_registry_temp) = lifecycle_registry_temp.as_ref() {
        replace_from_temporary(
            lifecycle_registry_temp,
            &lifecycle_registry_path,
            "runtime-lifecycle registry",
        )?;
    }
    replace_from_temporary(&state_temp, state_path, "service state")?;
    clear_service_state_transaction(&transaction_path)
}

fn commit_service_state_transaction(
    store: &JsonServiceStateStore,
    transaction: &ServiceStateTransaction,
) -> Result<(), String> {
    if let Some(parent) = store.path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create service state directory {}: {error}",
                parent.display()
            )
        })?;
    }
    recover_service_state_transaction(&store.path)?;
    let mut transaction = transaction.clone();
    merge_current_remote_view_handoff_registry(&store.path, &mut transaction)?;
    let handoff_path = remote_view_handoff_registry_path(&store.path);
    let owner_registry_path = runtime_owner_registry_path(&store.path);
    let lifecycle_registry_path = runtime_lifecycle_registry_path(&store.path);
    let transaction_path = service_state_transaction_path(&store.path);
    let prior_handoff = fs::read(&handoff_path).ok();
    let prior_owner_registry = fs::read(&owner_registry_path).ok();
    let prior_lifecycle_registry = fs::read(&lifecycle_registry_path).ok();

    store.fail_at(ServiceStateSaveBoundary::HandoffWrite)?;
    persist_durable_remote_view_presentations(&store.path, &transaction)?;
    let handoff_temp = write_temporary(
        &handoff_path,
        &transaction.handoff_payload,
        "remote-view handoff registry",
    )?;
    if let Err(error) = store.fail_at(ServiceStateSaveBoundary::OwnerRegistryWrite) {
        let _ = fs::remove_file(&handoff_temp);
        return Err(error);
    }
    let owner_registry_temp = match transaction
        .owner_registry_payload
        .as_deref()
        .map(|payload| write_temporary(&owner_registry_path, payload, "runtime-owner registry"))
        .transpose()
    {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&handoff_temp);
            return Err(error);
        }
    };
    if let Err(error) = store.fail_at(ServiceStateSaveBoundary::LifecycleRegistryWrite) {
        let _ = fs::remove_file(&handoff_temp);
        if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
            let _ = fs::remove_file(owner_registry_temp);
        }
        return Err(error);
    }
    let lifecycle_registry_temp = match transaction
        .lifecycle_registry_payload
        .as_deref()
        .map(|payload| {
            write_temporary(
                &lifecycle_registry_path,
                payload,
                "runtime-lifecycle registry",
            )
        })
        .transpose()
    {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&handoff_temp);
            if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
                let _ = fs::remove_file(owner_registry_temp);
            }
            return Err(error);
        }
    };
    if let Err(error) = store.fail_at(ServiceStateSaveBoundary::StateWrite) {
        let _ = fs::remove_file(&handoff_temp);
        if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
            let _ = fs::remove_file(owner_registry_temp);
        }
        if let Some(lifecycle_registry_temp) = lifecycle_registry_temp.as_ref() {
            let _ = fs::remove_file(lifecycle_registry_temp);
        }
        return Err(error);
    }
    let state_temp = write_temporary(&store.path, &transaction.state_payload, "service state")?;
    let transaction_payload = format!(
        "{}\n",
        serde_json::to_string_pretty(&transaction)
            .map_err(|error| format!("Failed to serialize service state transaction: {error}"))?
    );
    let transaction_temp = write_temporary(
        &transaction_path,
        &transaction_payload,
        "service state transaction",
    )?;
    replace_from_temporary(
        &transaction_temp,
        &transaction_path,
        "service state transaction",
    )?;

    if let Err(error) = store.fail_at(ServiceStateSaveBoundary::HandoffRename) {
        let _ = fs::remove_file(&handoff_temp);
        if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
            let _ = fs::remove_file(owner_registry_temp);
        }
        if let Some(lifecycle_registry_temp) = lifecycle_registry_temp.as_ref() {
            let _ = fs::remove_file(lifecycle_registry_temp);
        }
        let _ = fs::remove_file(&state_temp);
        let _ = fs::remove_file(&transaction_path);
        return Err(error);
    }
    if let Err(error) =
        replace_from_temporary(&handoff_temp, &handoff_path, "remote-view handoff registry")
    {
        if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
            let _ = fs::remove_file(owner_registry_temp);
        }
        if let Some(lifecycle_registry_temp) = lifecycle_registry_temp.as_ref() {
            let _ = fs::remove_file(lifecycle_registry_temp);
        }
        let _ = fs::remove_file(&state_temp);
        let _ = fs::remove_file(&transaction_path);
        return Err(error);
    }

    if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
        let owner_registry_result = store
            .fail_at(ServiceStateSaveBoundary::OwnerRegistryRename)
            .and_then(|()| {
                replace_from_temporary(
                    owner_registry_temp,
                    &owner_registry_path,
                    "runtime-owner registry",
                )
            });
        if let Err(error) = owner_registry_result {
            let restore_result = restore_file(&handoff_path, prior_handoff.as_deref());
            let _ = fs::remove_file(owner_registry_temp);
            let _ = fs::remove_file(&state_temp);
            if restore_result.is_ok() {
                let _ = fs::remove_file(&transaction_path);
            }
            return match restore_result {
                Ok(()) => Err(error),
                Err(restore_error) => Err(format!(
                    "{error}; service_state_transaction_recovery_required: {restore_error}"
                )),
            };
        }
    }

    if let Some(lifecycle_registry_temp) = lifecycle_registry_temp.as_ref() {
        let lifecycle_registry_result = store
            .fail_at(ServiceStateSaveBoundary::LifecycleRegistryRename)
            .and_then(|()| {
                replace_from_temporary(
                    lifecycle_registry_temp,
                    &lifecycle_registry_path,
                    "runtime-lifecycle registry",
                )
            });
        if let Err(error) = lifecycle_registry_result {
            let owner_restore_result =
                restore_file(&owner_registry_path, prior_owner_registry.as_deref());
            let handoff_restore_result = restore_file(&handoff_path, prior_handoff.as_deref());
            let _ = fs::remove_file(lifecycle_registry_temp);
            let _ = fs::remove_file(&state_temp);
            if owner_restore_result.is_ok() && handoff_restore_result.is_ok() {
                let _ = fs::remove_file(&transaction_path);
            }
            return match (owner_restore_result, handoff_restore_result) {
                (Ok(()), Ok(())) => Err(error),
                (owner_result, handoff_result) => Err(format!(
                    "{error}; service_state_transaction_recovery_required: owner_registry={}, remote_view_handoffs={}",
                    owner_result.err().unwrap_or_else(|| "restored".to_string()),
                    handoff_result.err().unwrap_or_else(|| "restored".to_string())
                )),
            };
        }
    }

    let state_result = store
        .fail_at(ServiceStateSaveBoundary::StateRename)
        .and_then(|()| replace_from_temporary(&state_temp, &store.path, "service state"));
    if let Err(error) = state_result {
        let owner_restore_result =
            restore_file(&owner_registry_path, prior_owner_registry.as_deref());
        let lifecycle_restore_result = restore_file(
            &lifecycle_registry_path,
            prior_lifecycle_registry.as_deref(),
        );
        let handoff_restore_result = restore_file(&handoff_path, prior_handoff.as_deref());
        let _ = fs::remove_file(&state_temp);
        if owner_restore_result.is_ok()
            && lifecycle_restore_result.is_ok()
            && handoff_restore_result.is_ok()
        {
            let _ = fs::remove_file(&transaction_path);
        }
        return match (
            owner_restore_result,
            lifecycle_restore_result,
            handoff_restore_result,
        ) {
            (Ok(()), Ok(()), Ok(())) => Err(error),
            (owner_result, lifecycle_result, handoff_result) => Err(format!(
                "{error}; service_state_transaction_recovery_required: owner_registry={}, runtime_lifecycle_registry={}, remote_view_handoffs={}",
                owner_result.err().unwrap_or_else(|| "restored".to_string()),
                lifecycle_result.err().unwrap_or_else(|| "restored".to_string()),
                handoff_result.err().unwrap_or_else(|| "restored".to_string())
            )),
        };
    }
    clear_service_state_transaction(&transaction_path)
}

#[derive(Debug, Clone, Copy)]
enum ServiceStateFileLockMode {
    Shared,
    Exclusive,
}

impl ServiceStateFileLockMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Exclusive => "exclusive",
        }
    }
}

struct ServiceStateFileGuard {
    file: File,
    token: u64,
}

impl Deref for ServiceStateFileGuard {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl Drop for ServiceStateFileGuard {
    fn drop(&mut self) {
        record_service_state_lock_released(self.token);
    }
}

fn service_state_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SERVICE_STATE_FILENAME);
    path.with_file_name(format!("{file_name}.lock"))
}

fn acquire_service_state_file_lock(
    state_path: &Path,
    mode: ServiceStateFileLockMode,
) -> Result<ServiceStateFileGuard, String> {
    acquire_service_state_file_lock_until(
        state_path,
        mode,
        Instant::now() + DEFAULT_SERVICE_STATE_LOCK_TIMEOUT,
        "direct_file_lock",
    )
}

fn acquire_service_state_file_lock_until(
    state_path: &Path,
    mode: ServiceStateFileLockMode,
    deadline: Instant,
    operation: &'static str,
) -> Result<ServiceStateFileGuard, String> {
    let started = Instant::now();
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create service state directory {}: {}",
                parent.display(),
                err
            )
        })?;
    }
    let lock_path = service_state_lock_path(state_path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|err| {
            format!(
                "Failed to open service state lock {}: {}",
                lock_path.display(),
                err
            )
        })?;
    loop {
        let result = match mode {
            ServiceStateFileLockMode::Shared => file.try_lock_shared(),
            ServiceStateFileLockMode::Exclusive => file.try_lock(),
        };
        match result {
            Ok(()) => {
                let token = record_service_state_lock_acquired(
                    "file",
                    operation,
                    mode.as_str(),
                    started.elapsed(),
                );
                return Ok(ServiceStateFileGuard { file, token });
            }
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() >= deadline => {
                record_service_state_lock_timeout(
                    "file",
                    operation,
                    mode.as_str(),
                    started.elapsed(),
                );
                return Err(format!(
                    "service_state_lock_timeout: file lock; waited_ms={}",
                    started.elapsed().as_millis()
                ));
            }
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
            Err(error) => {
                record_service_state_lock_terminal(
                    "file",
                    operation,
                    mode.as_str(),
                    "error",
                    started.elapsed(),
                );
                return Err(format!(
                    "Failed to acquire service state lock {}: {}",
                    lock_path.display(),
                    error
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserHost, BrowserProcess, DisplayAllocation, RemoteViewAcquisitionLease,
        RemoteViewHandoff, SitePolicy,
    };
    use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};
    use crate::test_utils::EnvGuard;
    use std::collections::BTreeMap;

    fn unique_state_path(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("state.json")
    }

    #[test]
    fn test_build_default_state_path_never_targets_process_home_without_explicit_escape_hatch() {
        let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME"]);
        let claimed_live_home = std::env::temp_dir().join(format!(
            "agent-browser-claimed-live-home-{}",
            std::process::id()
        ));
        guard.set("HOME", claimed_live_home.to_str().unwrap());
        guard.remove("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME");

        let state_path = default_service_state_path().unwrap();
        assert!(!state_path.starts_with(&claimed_live_home));
        assert!(state_path.starts_with(std::env::temp_dir()));
        assert!(state_path.ends_with(".agent-browser/service/state.json"));
    }

    fn runtime_owner(label: &str) -> ProfileOwner {
        ProfileOwner {
            owner_id: format!("owner-{label}"),
            profile_identity_digest: format!("{:0<64}", label),
            state: ProfileOwnerState::Ready,
            owner_generation: 1,
            browser_id: format!("browser-{label}"),
            daemon_session_route: format!("session-{label}"),
            process_instance_digest: "1".repeat(64),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "2".repeat(64),
            target_set_digest: "3".repeat(64),
            pending_transfer: None,
            last_transition: None,
        }
    }

    fn runtime_registry(label: &str) -> RuntimeOwnerRegistry {
        let owner = runtime_owner(label);
        let mut registry = RuntimeOwnerRegistry::from_owner(owner.clone());
        registry.lifecycle_records.insert(
            owner.browser_id.clone(),
            RuntimeLifecycleRecord {
                logical_browser_id: owner.browser_id,
                profile_identity_digest: owner.profile_identity_digest,
                owner_generation: owner.owner_generation,
                ..RuntimeLifecycleRecord::default()
            },
        );
        registry
    }

    #[test]
    fn missing_state_file_loads_default_state() {
        let store = JsonServiceStateStore::new(unique_state_path("missing-service-state"));

        let state = store.load().expect("missing state should load default");

        assert_eq!(state, ServiceState::default());
    }

    #[test]
    fn clearing_an_already_recovered_transaction_is_idempotent() {
        let path = unique_state_path("already-recovered-service-state-transaction");
        let transaction_path = service_state_transaction_path(&path);

        clear_service_state_transaction(&transaction_path)
            .expect("a transaction recovered by another process should already be clear");

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn save_and_load_round_trips_service_state() {
        let path = unique_state_path("round-trip-service-state");
        let store = JsonServiceStateStore::new(&path);
        let state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    host: BrowserHost::DockerHeaded,
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "https://accounts.google.com".to_string(),
                    manual_login_preferred: true,
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };

        store.save(&state).expect("state should save");
        let loaded = store.load().expect("state should load");

        assert_eq!(loaded.browsers, state.browsers);
        assert_eq!(
            loaded.site_policies["google"].origin_pattern,
            "https://accounts.google.com"
        );
        assert_eq!(
            loaded.site_policy_source("google"),
            Some(super::super::service_model::ServiceEntitySource::PersistedState)
        );
        assert_eq!(
            loaded.site_policy_source("microsoft"),
            Some(super::super::service_model::ServiceEntitySource::Builtin)
        );
        assert_eq!(store.path(), path.as_path());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn large_remote_view_lease_state_round_trips_on_a_constrained_tokio_worker() {
        let path = unique_state_path("large-remote-view-lease-state");
        let store = JsonServiceStateStore::new(&path);
        let nested_readiness = serde_json::json!({
            "state": "pending",
            "evidence": {
                "route": {
                    "display": {
                        "probe": {
                            "attempts": [{
                                "result": {
                                    "details": {
                                        "message": "synthetic bounded fixture"
                                    }
                                }
                            }]
                        }
                    }
                }
            }
        });
        let remote_view_acquisition_leases = (0..640)
            .map(|index| {
                let id = format!("lease-{index}");
                (
                    id.clone(),
                    RemoteViewAcquisitionLease {
                        id,
                        browser_id: "browser-1".to_string(),
                        session_id: "session-1".to_string(),
                        route_id: "route-1".to_string(),
                        display_allocation_id: "display-1".to_string(),
                        previous_display_allocation: Some(DisplayAllocation {
                            id: "display-1".to_string(),
                            readiness: Some(nested_readiness.clone()),
                            ..DisplayAllocation::default()
                        }),
                        ..RemoteViewAcquisitionLease::default()
                    },
                )
            })
            .collect();
        store
            .save(&ServiceState {
                remote_view_acquisition_leases,
                ..ServiceState::default()
            })
            .expect("large fixture should save");
        let embedded = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&path).expect("large fixture JSON should be readable"),
        )
        .expect("large fixture JSON should parse as a value");

        let runtime =
            crate::native::daemon::build_runtime(1).expect("fixture daemon runtime should build");
        let (decoded, loaded) = runtime.block_on(async move {
            tokio::spawn(async move {
                let decoded = decode_service_state_value(&embedded);
                let result = store.load();
                if let Ok(state) = result.as_ref() {
                    store
                        .save(state)
                        .expect("large service state should save from a constrained worker");
                }
                (decoded, result)
            })
            .await
            .expect("service-state loader task should not crash")
        });

        let decoded = decoded.expect("large embedded fixture should decode");
        let loaded = loaded.expect("large fixture should load from disk");

        assert_eq!(decoded.remote_view_acquisition_leases.len(), 640);
        assert_eq!(loaded.remote_view_acquisition_leases.len(), 640);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn locked_repository_mutates_and_loads_snapshot() {
        let path = unique_state_path("repository-service-state");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));

        let result = repository
            .mutate(|state| {
                state.browsers.insert(
                    "browser-1".to_string(),
                    BrowserProcess {
                        id: "browser-1".to_string(),
                        host: BrowserHost::LocalHeaded,
                        health: BrowserHealth::Ready,
                        ..BrowserProcess::default()
                    },
                );
                Ok("mutated")
            })
            .expect("repository mutation should save");

        let state = repository
            .load_snapshot()
            .expect("repository snapshot should load");

        assert_eq!(result, "mutated");
        assert_eq!(state.browsers["browser-1"].health, BrowserHealth::Ready);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn service_state_file_lock_excludes_an_independent_writer() {
        let path = unique_state_path("cross-process-service-state-lock");
        let first = acquire_service_state_file_lock(&path, ServiceStateFileLockMode::Exclusive)
            .expect("first writer should acquire the service-state lock");
        let lock_path = service_state_lock_path(&path);
        let second = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .expect("second writer should open the stable lock file");

        let error = second
            .try_lock()
            .expect_err("an independent writer must not enter while the lock is held");

        assert!(matches!(error, std::fs::TryLockError::WouldBlock));
        drop(first);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn route_repository_lock_acquisition_is_bounded() {
        let path = unique_state_path("bounded-service-state-lock");
        let held = acquire_service_state_file_lock(&path, ServiceStateFileLockMode::Exclusive)
            .expect("fixture should hold the service-state lock");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        let timeouts_before = service_state_lock_diagnostics().counters.file_timeouts;

        let started = Instant::now();
        let error = repository
            .load_snapshot_with_lock_timeout(Duration::from_millis(10))
            .expect_err("repository must not wait indefinitely for the file lock");

        assert!(error.contains("service_state_lock_timeout"));
        assert!(started.elapsed() < Duration::from_secs(1));
        let diagnostics = service_state_lock_diagnostics();
        assert!(diagnostics.counters.file_timeouts > timeouts_before);
        assert!(diagnostics.recent.iter().any(|activity| {
            activity.lock_kind == "file"
                && activity.operation == "snapshot"
                && activity.phase == "timeout"
                && activity.hold_ms.is_none()
        }));
        drop(held);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn stable_snapshot_readers_share_the_service_state_file_lock() {
        let path = unique_state_path("shared-service-state-snapshot-lock");
        let store = JsonServiceStateStore::new(&path);
        store
            .save(&ServiceState::default())
            .expect("fixture state should save");
        let first_reader = acquire_service_state_file_lock(&path, ServiceStateFileLockMode::Shared)
            .expect("first snapshot reader should acquire the shared lock");
        let repository = LockedServiceStateRepository::new(store);

        let snapshot = repository
            .load_snapshot_with_lock_timeout(Duration::from_millis(20))
            .expect("a stable snapshot reader should not exclude another reader");

        assert!(snapshot.jobs.is_empty());
        drop(first_reader);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn stable_snapshot_reader_does_not_wait_for_the_process_mutation_mutex() {
        let path = unique_state_path("snapshot-with-process-mutation-mutex-held");
        let store = JsonServiceStateStore::new(&path);
        store
            .save(&ServiceState::default())
            .expect("fixture state should save");
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let process_guard = lock
            .lock()
            .expect("fixture should hold the process mutation mutex");
        let repository = LockedServiceStateRepository::new(store);

        let snapshot = repository
            .load_snapshot_with_lock_timeout(Duration::from_millis(20))
            .expect("stable snapshot reads should not use the mutation mutex");

        assert!(snapshot.jobs.is_empty());
        drop(process_guard);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn process_mutation_holder_metadata_clears_when_guard_drops() {
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let guard = acquire_service_state_process_lock(
            lock,
            Instant::now() + Duration::from_millis(20),
            "fixture_mutation",
        )
        .expect("fixture mutation should acquire the process mutex");
        assert_eq!(
            *SERVICE_STATE_ACTIVE_MUTATION
                .get_or_init(|| Mutex::new(None))
                .lock()
                .expect("holder metadata should remain readable"),
            Some("fixture_mutation")
        );

        drop(guard);

        assert_eq!(
            *SERVICE_STATE_ACTIVE_MUTATION
                .get_or_init(|| Mutex::new(None))
                .lock()
                .expect("holder metadata should remain readable"),
            None
        );
    }

    #[test]
    fn service_state_lock_diagnostics_track_active_and_released_file_holds() {
        let path = unique_state_path("service-state-file-lock-telemetry");
        let guard = acquire_service_state_file_lock_until(
            &path,
            ServiceStateFileLockMode::Exclusive,
            Instant::now() + Duration::from_millis(20),
            "telemetry_file_fixture",
        )
        .expect("fixture file lock should be acquired");

        let active = service_state_lock_diagnostics();
        assert!(active.active.iter().any(|activity| {
            activity.lock_kind == "file"
                && activity.operation == "telemetry_file_fixture"
                && activity.phase == "holding"
                && activity.mode == "exclusive"
        }));

        drop(guard);

        let released = service_state_lock_diagnostics();
        assert!(!released
            .active
            .iter()
            .any(|activity| activity.operation == "telemetry_file_fixture"));
        assert!(released.recent.iter().any(|activity| {
            activity.operation == "telemetry_file_fixture"
                && activity.phase == "released"
                && activity.hold_ms.is_some()
        }));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn process_lock_diagnostics_cleanup_and_recover_after_unwind() {
        let lock = Box::leak(Box::new(Mutex::new(())));
        let panic_result = std::panic::catch_unwind(|| {
            let _guard = acquire_service_state_process_lock(
                lock,
                Instant::now() + Duration::from_millis(20),
                "telemetry_unwind_fixture",
            )
            .expect("fixture process lock should be acquired");
            panic!("intentional lock-guard unwind");
        });
        assert!(panic_result.is_err());
        assert!(!service_state_lock_diagnostics()
            .active
            .iter()
            .any(|activity| activity.operation == "telemetry_unwind_fixture"));

        let recoveries_before = service_state_lock_diagnostics()
            .counters
            .process_poison_recoveries;
        let recovered = acquire_service_state_process_lock(
            lock,
            Instant::now() + Duration::from_millis(20),
            "telemetry_poison_recovery_fixture",
        )
        .expect("a poisoned process mutex should be safely recovered");
        drop(recovered);
        let diagnostics = service_state_lock_diagnostics();
        assert!(diagnostics.counters.process_poison_recoveries > recoveries_before);
        assert!(diagnostics.recent.iter().any(|activity| {
            activity.operation == "telemetry_poison_recovery_fixture"
                && activity.phase == "released"
        }));
    }

    #[test]
    fn process_lock_timeout_is_recorded_without_a_false_active_contender() {
        let lock = Box::leak(Box::new(Mutex::new(())));
        let held = lock.lock().expect("fixture should hold the process mutex");
        let timeouts_before = service_state_lock_diagnostics().counters.process_timeouts;

        let error = match acquire_service_state_process_lock(
            lock,
            Instant::now() + Duration::from_millis(10),
            "telemetry_process_timeout_fixture",
        ) {
            Ok(_) => panic!("contender should time out before mutation entry"),
            Err(error) => error,
        };
        assert!(error.starts_with("service_state_lock_timeout: process mutation lock"));
        let diagnostics = service_state_lock_diagnostics();
        assert!(diagnostics.counters.process_timeouts > timeouts_before);
        assert!(!diagnostics
            .active
            .iter()
            .any(|activity| activity.operation == "telemetry_process_timeout_fixture"));
        assert!(diagnostics.recent.iter().any(|activity| {
            activity.lock_kind == "process"
                && activity.operation == "telemetry_process_timeout_fixture"
                && activity.phase == "timeout"
                && activity.hold_ms.is_none()
        }));
        drop(held);
    }

    #[test]
    fn file_lock_diagnostics_cleanup_after_early_return() {
        fn acquire_and_return(path: &Path) -> Result<(), &'static str> {
            let _guard = acquire_service_state_file_lock_until(
                path,
                ServiceStateFileLockMode::Shared,
                Instant::now() + Duration::from_millis(20),
                "telemetry_early_return_fixture",
            )
            .expect("fixture file lock should be acquired");
            Err("cancelled")
        }

        let path = unique_state_path("service-state-lock-early-return");
        assert_eq!(acquire_and_return(&path), Err("cancelled"));
        let diagnostics = service_state_lock_diagnostics();
        assert!(!diagnostics
            .active
            .iter()
            .any(|activity| activity.operation == "telemetry_early_return_fixture"));
        assert!(diagnostics.recent.iter().any(|activity| {
            activity.operation == "telemetry_early_return_fixture" && activity.phase == "released"
        }));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn service_state_lock_diagnostics_are_bounded() {
        for _ in 0..(SERVICE_STATE_LOCK_RECENT_CAPACITY + 5) {
            record_service_state_lock_terminal(
                "process",
                "telemetry_capacity_fixture",
                "exclusive",
                "error",
                Duration::from_millis(1),
            );
        }

        let diagnostics = service_state_lock_diagnostics();
        assert_eq!(
            diagnostics.recent_capacity,
            SERVICE_STATE_LOCK_RECENT_CAPACITY
        );
        assert_eq!(diagnostics.recent.len(), SERVICE_STATE_LOCK_RECENT_CAPACITY);
    }

    #[test]
    fn json_backed_persistence_does_not_hold_the_process_mutation_mutex() {
        struct BlockingSaveStore {
            path: PathBuf,
            save_entered: Arc<std::sync::Barrier>,
            save_release: Arc<std::sync::Barrier>,
        }

        impl ServiceStateStore for BlockingSaveStore {
            fn load(&self) -> Result<ServiceState, String> {
                Ok(ServiceState::default())
            }

            fn save(&self, _state: &ServiceState) -> Result<(), String> {
                self.save_entered.wait();
                self.save_release.wait();
                Ok(())
            }

            fn state_path(&self) -> Option<&Path> {
                Some(&self.path)
            }
        }

        let path = unique_state_path("blocking-save-process-mutex");
        let save_entered = Arc::new(std::sync::Barrier::new(2));
        let save_release = Arc::new(std::sync::Barrier::new(2));
        let repository = LockedServiceStateRepository::new(BlockingSaveStore {
            path: path.clone(),
            save_entered: Arc::clone(&save_entered),
            save_release: Arc::clone(&save_release),
        });
        let mutation = std::thread::spawn(move || repository.mutate(|_| Ok(())));

        save_entered.wait();
        let process_lock_available = SERVICE_STATE_MUTATION_LOCK
            .get_or_init(|| Mutex::new(()))
            .try_lock()
            .is_ok();
        save_release.wait();

        mutation
            .join()
            .expect("mutation thread should not panic")
            .expect("mutation should finish after save is released");
        assert!(
            process_lock_available,
            "durable serialization and commit must not retain the process mutex"
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn stale_prepared_transaction_is_rejected_before_commit_effect() {
        struct StaleRevisionStore {
            path: PathBuf,
            state: Arc<Mutex<ServiceState>>,
            prepare_entered: Arc<std::sync::Barrier>,
            prepare_release: Arc<std::sync::Barrier>,
            commit_count: Arc<std::sync::atomic::AtomicUsize>,
        }

        impl ServiceStateStore for StaleRevisionStore {
            fn load(&self) -> Result<ServiceState, String> {
                Ok(self.state.lock().unwrap().clone())
            }

            fn load_without_recovery(&self) -> Result<ServiceState, String> {
                self.load()
            }

            fn save(&self, _state: &ServiceState) -> Result<(), String> {
                Err("unexpected_unprepared_save".to_string())
            }

            fn state_path(&self) -> Option<&Path> {
                Some(&self.path)
            }

            fn supports_prepared_save(&self) -> bool {
                true
            }

            fn prepare_save(
                &self,
                _state: &ServiceState,
            ) -> Result<Option<ServiceStateTransaction>, String> {
                self.prepare_entered.wait();
                self.prepare_release.wait();
                Ok(Some(ServiceStateTransaction {
                    state_payload: String::new(),
                    handoff_payload: String::new(),
                    owner_registry_payload: None,
                    lifecycle_registry_payload: None,
                }))
            }

            fn save_prepared(&self, _transaction: &ServiceStateTransaction) -> Result<(), String> {
                self.commit_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }

        let path = unique_state_path("stale-prepared-transaction");
        let state = Arc::new(Mutex::new(ServiceState::default()));
        let prepare_entered = Arc::new(std::sync::Barrier::new(2));
        let prepare_release = Arc::new(std::sync::Barrier::new(2));
        let commit_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let repository = LockedServiceStateRepository::new(StaleRevisionStore {
            path: path.clone(),
            state: Arc::clone(&state),
            prepare_entered: Arc::clone(&prepare_entered),
            prepare_release: Arc::clone(&prepare_release),
            commit_count: Arc::clone(&commit_count),
        });
        let mutation = std::thread::spawn(move || repository.mutate(|_| Ok(())));

        prepare_entered.wait();
        state.lock().unwrap().state_revision = 1;
        prepare_release.wait();

        let error = mutation
            .join()
            .expect("mutation thread should not panic")
            .expect_err("stale prepared state must fail before commit");
        assert_eq!(error, "service_state_stale_revision: expected=0; actual=1");
        assert_eq!(commit_count.load(std::sync::atomic::Ordering::SeqCst), 0);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn realistic_state_mixed_burst_has_no_lock_timeouts_or_duplicate_effects() {
        const READER_COUNT: usize = 6;
        const WRITER_COUNT: usize = 2;
        const MINIMUM_FIXTURE_BYTES: usize = 2_900_000;

        let path = unique_state_path("realistic-mixed-service-state-burst");
        let store = JsonServiceStateStore::new(&path);
        let mut fixture = ServiceState::default();
        let payload = "x".repeat(20_000);
        for index in 0..150 {
            let id = format!("retained-job-{index:03}");
            fixture.jobs.insert(
                id.clone(),
                crate::native::service_model::ServiceJob {
                    id,
                    action: ["launch", "remote_view_open", "tab_new", "viewport"][index % 4]
                        .to_string(),
                    result: Some(serde_json::json!({ "boundedFixturePayload": payload.clone() })),
                    ..crate::native::service_model::ServiceJob::default()
                },
            );
        }
        store.save(&fixture).expect("realistic fixture should save");
        let fixture_bytes = fs::metadata(&path)
            .expect("realistic fixture should exist")
            .len() as usize;
        assert!(
            fixture_bytes >= MINIMUM_FIXTURE_BYTES,
            "fixture was {fixture_bytes} bytes"
        );

        let start = Arc::new(std::sync::Barrier::new(READER_COUNT + WRITER_COUNT + 1));
        let lock_timeout_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let duplicate_effect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut workers = Vec::new();
        for _ in 0..READER_COUNT {
            let path = path.clone();
            let start = Arc::clone(&start);
            let lock_timeout_count = Arc::clone(&lock_timeout_count);
            workers.push(std::thread::spawn(move || {
                start.wait();
                let repository =
                    LockedServiceStateRepository::new(JsonServiceStateStore::new(path));
                if let Err(error) = repository.load_snapshot() {
                    if error.starts_with("service_state_lock_timeout:") {
                        lock_timeout_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    return Err(error);
                }
                Ok(())
            }));
        }
        for writer_index in 0..WRITER_COUNT {
            let path = path.clone();
            let start = Arc::clone(&start);
            let lock_timeout_count = Arc::clone(&lock_timeout_count);
            let duplicate_effect_count = Arc::clone(&duplicate_effect_count);
            workers.push(std::thread::spawn(move || {
                start.wait();
                let repository =
                    LockedServiceStateRepository::new(JsonServiceStateStore::new(path));
                let effect_id = format!("burst-effect-{writer_index}");
                for attempt in 0..=1 {
                    let outcome = repository.mutate(|state| {
                        if state.jobs.contains_key(&effect_id) {
                            duplicate_effect_count
                                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            return Ok(());
                        }
                        state.jobs.insert(
                            effect_id.clone(),
                            crate::native::service_model::ServiceJob {
                                id: effect_id.clone(),
                                action: if writer_index == 0 {
                                    "tab_new".to_string()
                                } else {
                                    "remote_view_open".to_string()
                                },
                                ..crate::native::service_model::ServiceJob::default()
                            },
                        );
                        Ok(())
                    });
                    match outcome {
                        Ok(()) => return Ok(()),
                        Err(error)
                            if error.starts_with("service_state_stale_revision:")
                                && attempt == 0 =>
                        {
                            continue
                        }
                        Err(error) => {
                            if error.starts_with("service_state_lock_timeout:") {
                                lock_timeout_count
                                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                            return Err(error);
                        }
                    }
                }
                unreachable!("bounded retry loop always returns")
            }));
        }

        let started = Instant::now();
        start.wait();
        for worker in workers {
            worker
                .join()
                .expect("burst worker should not panic")
                .expect("burst worker should finish with a classified outcome");
        }
        assert_eq!(
            lock_timeout_count.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(
            duplicate_effect_count.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        let final_state = store.load().expect("final burst state should load");
        assert_eq!(final_state.state_revision, WRITER_COUNT as u64);
        assert!(final_state.jobs.contains_key("burst-effect-0"));
        assert!(final_state.jobs.contains_key("burst-effect-1"));
        let elapsed = started.elapsed();
        let diagnostics = service_state_lock_diagnostics();
        let mut hold_samples = diagnostics
            .recent
            .iter()
            .filter_map(|activity| activity.hold_ms)
            .collect::<Vec<_>>();
        hold_samples.sort_unstable();
        let p95_index = hold_samples
            .len()
            .saturating_mul(95)
            .div_ceil(100)
            .saturating_sub(1);
        let hold_p95_ms = hold_samples.get(p95_index).copied().unwrap_or_default();
        let hold_max_ms = hold_samples.last().copied().unwrap_or_default();
        eprintln!(
            "service_state_burst_receipt fixture_bytes={fixture_bytes} readers={READER_COUNT} writers={WRITER_COUNT} lock_timeouts=0 duplicate_effects=0 elapsed_ms={} lock_hold_samples={} lock_hold_p95_ms={hold_p95_ms} lock_hold_max_ms={hold_max_ms}",
            elapsed.as_millis(),
            hold_samples.len(),
        );
        assert!(hold_p95_ms < elapsed_millis(DEFAULT_SERVICE_STATE_LOCK_TIMEOUT));
        assert!(hold_max_ms < elapsed_millis(DEFAULT_SERVICE_STATE_LOCK_TIMEOUT));
        assert!(elapsed < Duration::from_secs(10));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn cross_process_file_lock_helper() {
        let Some(state_path) = std::env::var_os("AGENT_BROWSER_TEST_LOCK_HELPER_STATE_PATH") else {
            return;
        };
        let ready_path = PathBuf::from(
            std::env::var_os("AGENT_BROWSER_TEST_LOCK_HELPER_READY_PATH")
                .expect("lock helper ready path should be supplied"),
        );
        let release_path = PathBuf::from(
            std::env::var_os("AGENT_BROWSER_TEST_LOCK_HELPER_RELEASE_PATH")
                .expect("lock helper release path should be supplied"),
        );
        let _guard = acquire_service_state_file_lock(
            Path::new(&state_path),
            ServiceStateFileLockMode::Exclusive,
        )
        .expect("child process should acquire the fixture file lock");
        fs::write(&ready_path, b"ready").expect("child should publish lock readiness");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !release_path.exists() {
            assert!(
                Instant::now() < deadline,
                "parent did not release helper lock"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn independent_process_file_lock_timeout_is_classified_before_mutation() {
        let path = unique_state_path("cross-process-service-state-lock");
        let ready_path = path.with_extension("helper-ready");
        let release_path = path.with_extension("helper-release");
        let mut child = std::process::Command::new(
            std::env::current_exe().expect("current Rust test executable should resolve"),
        )
        .args([
            "--exact",
            "native::service_store::tests::cross_process_file_lock_helper",
            "--nocapture",
        ])
        .env("AGENT_BROWSER_TEST_LOCK_HELPER_STATE_PATH", &path)
        .env("AGENT_BROWSER_TEST_LOCK_HELPER_READY_PATH", &ready_path)
        .env("AGENT_BROWSER_TEST_LOCK_HELPER_RELEASE_PATH", &release_path)
        .spawn()
        .expect("cross-process lock helper should start");
        let ready_deadline = Instant::now() + Duration::from_secs(5);
        while !ready_path.exists() {
            assert!(
                Instant::now() < ready_deadline,
                "cross-process lock helper did not become ready"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        let error = repository
            .mutate_with_lock_timeout(Duration::from_millis(20), |state| {
                state.state_revision = 999;
                Ok(())
            })
            .expect_err("independent process lock must block mutation entry");
        assert!(error.starts_with("service_state_lock_timeout: file lock; waited_ms="));

        fs::write(&release_path, b"release").expect("parent should release helper lock");
        assert!(child.wait().expect("lock helper should exit").success());
        assert!(
            !path.exists(),
            "blocked mutation must not create Service State"
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn durable_remote_view_handoffs_survive_a_legacy_state_writer() {
        let path = unique_state_path("legacy-writer-remote-view-handoff");
        let store = JsonServiceStateStore::new(&path);
        let handoff = RemoteViewHandoff {
            id: "handoff-a".to_string(),
            state: "ready".to_string(),
            browser_id: Some("browser-a".to_string()),
            session_name: Some("session-a".to_string()),
            view_stream_provider: Some(
                crate::native::service_model::ViewStreamProvider::RdpGateway,
            ),
            ..RemoteViewHandoff::default()
        };
        store
            .save(&ServiceState {
                remote_view_handoffs: BTreeMap::from([(handoff.id.clone(), handoff)]),
                ..ServiceState::default()
            })
            .expect("handoff should save");

        let mut legacy_state: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("saved state should be readable"),
        )
        .expect("saved state should be JSON");
        legacy_state
            .as_object_mut()
            .expect("service state should be an object")
            .remove("remoteViewHandoffs");
        fs::write(
            &path,
            serde_json::to_string_pretty(&legacy_state).expect("legacy state should serialize"),
        )
        .expect("legacy writer should replace the primary state file");

        let loaded = store
            .load()
            .expect("state should load after legacy rewrite");

        assert_eq!(loaded.remote_view_handoffs["handoff-a"].id, "handoff-a");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn newer_handoff_presentation_generation_survives_a_stale_state_save() {
        let path = unique_state_path("stale-writer-remote-view-presentation");
        let store = JsonServiceStateStore::new(&path);
        let mut current_handoff = RemoteViewHandoff {
            id: "handoff-a".to_string(),
            state: "ready".to_string(),
            browser_id: Some("browser-a".to_string()),
            target_id: Some("target-new".to_string()),
            ..RemoteViewHandoff::default()
        };
        current_handoff.presentation_receipt = Some(
            crate::native::service_model::DurableHandoffPresentationReceipt {
                schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                generation: 2,
                dashboard_deployment_generation: "generation-new".to_string(),
                logical_browser_id: "browser-a".to_string(),
                daemon_owner_generation: Some(2),
                process_instance_digest: Some("process-new".to_string()),
                target_id: "target-new".to_string(),
                required_stream_provider:
                    crate::native::service_model::ViewStreamProvider::RdpGateway,
                observed_stream_provider:
                    crate::native::service_model::ViewStreamProvider::RdpGateway,
                route_id: "route-new".to_string(),
                display_allocation_id: "display-new".to_string(),
                observed_at: "2026-08-17T00:00:00Z".to_string(),
                state: "ready".to_string(),
            },
        );
        store
            .save(&ServiceState {
                remote_view_handoffs: BTreeMap::from([(
                    current_handoff.id.clone(),
                    current_handoff.clone(),
                )]),
                ..ServiceState::default()
            })
            .expect("newer presentation should save");

        let stale_handoff = RemoteViewHandoff {
            id: "handoff-a".to_string(),
            state: "resolving".to_string(),
            browser_id: Some("browser-a".to_string()),
            target_id: Some("target-old".to_string()),
            ..RemoteViewHandoff::default()
        };
        let stale_transaction = prepare_service_state_transaction(&ServiceState {
            remote_view_handoffs: BTreeMap::from([(stale_handoff.id.clone(), stale_handoff)]),
            ..ServiceState::default()
        })
        .expect("stale legacy payload should prepare");
        fs::write(&path, stale_transaction.state_payload)
            .expect("legacy writer should replace primary state");
        fs::write(
            remote_view_handoff_registry_path(&path),
            stale_transaction.handoff_payload,
        )
        .expect("legacy writer should replace the known handoff sidecar");

        let loaded = store.load().expect("merged handoff should load");
        assert_eq!(loaded.remote_view_handoffs["handoff-a"], current_handoff);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn durable_runtime_owner_registry_survives_a_legacy_state_writer() {
        let path = unique_state_path("legacy-writer-runtime-owner-registry");
        let store = JsonServiceStateStore::new(&path);
        let owner = runtime_owner("legacy");
        store
            .save(&ServiceState {
                runtime_owner_registry: RuntimeOwnerRegistry::from_owner(owner.clone()),
                ..ServiceState::default()
            })
            .expect("runtime owner should save");

        let mut legacy_state: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("saved state should be readable"),
        )
        .expect("saved state should be JSON");
        legacy_state
            .as_object_mut()
            .expect("service state should be an object")
            .remove("runtimeOwnerRegistry");
        fs::write(
            &path,
            serde_json::to_string_pretty(&legacy_state).expect("legacy state should serialize"),
        )
        .expect("legacy writer should replace the primary state file");

        let loaded = store
            .load()
            .expect("state should load after legacy rewrite");

        assert_eq!(
            loaded
                .runtime_owner_registry
                .owner(&owner.profile_identity_digest),
            Some(&owner)
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn lifecycle_sidecar_preserves_new_evidence_without_breaking_legacy_registry_readers() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct LegacyRuntimeOwnerRegistry {
            revision: u64,
            owners: BTreeMap<String, ProfileOwner>,
        }

        let path = unique_state_path("legacy-compatible-runtime-lifecycle-registry");
        let store = JsonServiceStateStore::new(&path);
        let owner = runtime_owner("lifecycle");
        let registry = runtime_registry("lifecycle");
        store
            .save(&ServiceState {
                runtime_owner_registry: registry.clone(),
                ..ServiceState::default()
            })
            .expect("runtime lifecycle should save");

        let state_raw = fs::read_to_string(&path).expect("state should be readable");
        let owner_raw = fs::read_to_string(runtime_owner_registry_path(&path))
            .expect("owner registry should be readable");
        let lifecycle_raw = fs::read_to_string(runtime_lifecycle_registry_path(&path))
            .expect("lifecycle sidecar should be readable");
        assert!(!state_raw.contains("lifecycleRecords"));
        assert!(!owner_raw.contains("lifecycleRecords"));
        assert!(lifecycle_raw.contains("records"));
        assert!(lifecycle_raw.contains(&owner.browser_id));
        let state_json: serde_json::Value =
            serde_json::from_str(&state_raw).expect("state should be valid JSON");
        let legacy_state_registry: LegacyRuntimeOwnerRegistry =
            serde_json::from_value(state_json["runtimeOwnerRegistry"].clone())
                .expect("legacy state reader should accept the embedded registry");
        let owner_json: serde_json::Value =
            serde_json::from_str(&owner_raw).expect("owner sidecar should be valid JSON");
        let legacy_owner_registry: LegacyRuntimeOwnerRegistry =
            serde_json::from_value(owner_json["registry"].clone())
                .expect("legacy owner reader should accept the durable registry");
        assert_eq!(legacy_state_registry.revision, registry.revision);
        assert_eq!(legacy_state_registry.owners, registry.owners);
        assert_eq!(legacy_owner_registry.revision, registry.revision);
        assert_eq!(legacy_owner_registry.owners, registry.owners);

        let loaded = store.load().expect("new reader should merge the sidecar");
        assert_eq!(loaded.runtime_owner_registry, registry);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn four_file_service_state_commit_is_atomic_at_every_write_and_rename_boundary() {
        for boundary in [
            ServiceStateSaveBoundary::HandoffWrite,
            ServiceStateSaveBoundary::OwnerRegistryWrite,
            ServiceStateSaveBoundary::LifecycleRegistryWrite,
            ServiceStateSaveBoundary::StateWrite,
            ServiceStateSaveBoundary::HandoffRename,
            ServiceStateSaveBoundary::OwnerRegistryRename,
            ServiceStateSaveBoundary::LifecycleRegistryRename,
            ServiceStateSaveBoundary::StateRename,
        ] {
            let path = unique_state_path(&format!("atomic-service-state-{boundary:?}"));
            let baseline_store = JsonServiceStateStore::new(&path);
            baseline_store
                .save(&ServiceState {
                    remote_view_handoffs: BTreeMap::from([(
                        "handoff-before".to_string(),
                        RemoteViewHandoff {
                            id: "handoff-before".to_string(),
                            state: "ready".to_string(),
                            ..RemoteViewHandoff::default()
                        },
                    )]),
                    runtime_owner_registry: runtime_registry("before"),
                    ..ServiceState::default()
                })
                .expect("baseline state should save");

            let failing_store = JsonServiceStateStore::with_save_fault(&path, boundary);
            let error = failing_store
                .save(&ServiceState {
                    browsers: BTreeMap::from([(
                        "browser-after".to_string(),
                        BrowserProcess {
                            id: "browser-after".to_string(),
                            ..BrowserProcess::default()
                        },
                    )]),
                    remote_view_handoffs: BTreeMap::from([(
                        "handoff-after".to_string(),
                        RemoteViewHandoff {
                            id: "handoff-after".to_string(),
                            state: "ready".to_string(),
                            ..RemoteViewHandoff::default()
                        },
                    )]),
                    runtime_owner_registry: runtime_registry("after"),
                    ..ServiceState::default()
                })
                .expect_err("injected boundary must fail the transaction");
            assert!(error.contains("injected_service_state_save_failure"));

            let loaded = baseline_store
                .load()
                .expect("failed transaction must leave a readable baseline");
            assert!(loaded.remote_view_handoffs.contains_key("handoff-before"));
            assert!(!loaded.remote_view_handoffs.contains_key("handoff-after"));
            assert!(loaded
                .runtime_owner_registry
                .owner(&runtime_owner("before").profile_identity_digest)
                .is_some());
            assert!(loaded
                .runtime_owner_registry
                .owner(&runtime_owner("after").profile_identity_digest)
                .is_none());
            assert!(loaded
                .runtime_owner_registry
                .lifecycle_records
                .contains_key("browser-before"));
            assert!(!loaded
                .runtime_owner_registry
                .lifecycle_records
                .contains_key("browser-after"));
            assert!(!loaded.browsers.contains_key("browser-after"));
            assert!(!service_state_transaction_path(&path).exists());
            let _ = fs::remove_dir_all(path.parent().unwrap());
        }
    }

    #[test]
    fn invalid_state_file_returns_error() {
        let path = unique_state_path("bad-service-state");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{not-json").unwrap();
        let store = JsonServiceStateStore::new(&path);

        let err = store.load().expect_err("invalid state should fail");

        assert!(err.contains("Invalid service state JSON"));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_backfills_derived_incidents() {
        let path = unique_state_path("service-state-incidents");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{
  "events": [
    {
      "id": "event-1",
      "timestamp": "2026-04-22T00:00:00Z",
      "kind": "reconciliation_error",
      "message": "Failed to reconcile service state"
    }
  ]
}"#,
        )
        .unwrap();
        let store = JsonServiceStateStore::new(&path);

        let state = store.load().expect("state should load");

        assert_eq!(
            state.incidents,
            vec![crate::native::service_model::ServiceIncident {
                id: "service".to_string(),
                label: "Service incidents".to_string(),
                state: crate::native::service_model::ServiceIncidentState::Service,
                severity: crate::native::service_model::ServiceIncidentSeverity::Error,
                escalation: crate::native::service_model::ServiceIncidentEscalation::ServiceTriage,
                recommended_action: "Inspect service logs, reconciliation state, and recent jobs."
                    .to_string(),
                latest_timestamp: "2026-04-22T00:00:00Z".to_string(),
                latest_message: "Failed to reconcile service state".to_string(),
                latest_kind: "reconciliation_error".to_string(),
                event_ids: vec!["event-1".to_string()],
                ..crate::native::service_model::ServiceIncident::default()
            }]
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
