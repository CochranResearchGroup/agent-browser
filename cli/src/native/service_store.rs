//! Persistent service-state storage.
//!
//! The first service-mode store is JSON-backed and intentionally small. It gives
//! later lifecycle work a durable contract without forcing a database choice yet.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, TryLockError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::runtime_owner_transfer::RuntimeOwnerRegistry;

use super::service_model::{RemoteViewHandoff, ServiceState};

const SERVICE_DIR: &str = "service";
const SERVICE_STATE_FILENAME: &str = "state.json";
const REMOTE_VIEW_HANDOFFS_FILENAME: &str = "remote-view-handoffs.json";
const REMOTE_VIEW_HANDOFFS_SCHEMA_VERSION: &str = "agent-browser.remote-view-handoffs.v1";
const REMOTE_VIEW_PRESENTATIONS_FILENAME: &str = "remote-view-presentations.json";
const REMOTE_VIEW_PRESENTATIONS_SCHEMA_VERSION: &str = "agent-browser.remote-view-presentations.v1";
const RUNTIME_OWNER_REGISTRY_FILENAME: &str = "runtime-owner-registry.json";
const RUNTIME_OWNER_REGISTRY_SCHEMA_VERSION: &str = "agent-browser.runtime-owner-registry.v1";
static SERVICE_STATE_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
const DEFAULT_SERVICE_STATE_LOCK_TIMEOUT: Duration = Duration::from_secs(1);
const SERVICE_STATE_JSON_STACK_BYTES: usize = 8 * 1024 * 1024;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceStateTransaction {
    state_payload: String,
    handoff_payload: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner_registry_payload: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServiceStateSaveBoundary {
    HandoffWrite,
    OwnerRegistryWrite,
    StateWrite,
    HandoffRename,
    OwnerRegistryRename,
    StateRename,
}

pub trait ServiceStateStore {
    fn load(&self) -> Result<ServiceState, String>;
    fn save(&self, state: &ServiceState) -> Result<(), String>;

    fn state_path(&self) -> Option<&Path> {
        None
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
        if state_file_missing
            && handoff_registry.handoffs.is_empty()
            && presentation_registry.handoffs.is_empty()
            && owner_registry.schema_version.is_empty()
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
        state.mark_persisted_entity_sources();
        state.refresh_derived_views();
        Ok(state)
    }

    fn save(&self, state: &ServiceState) -> Result<(), String> {
        let transaction = prepare_service_state_transaction(state)?;
        commit_service_state_transaction(self, &transaction)
    }

    fn state_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

fn parse_service_state_json(raw: String, path: &Path) -> Result<ServiceState, String> {
    let display_path = path.display().to_string();
    // Large service histories can exhaust a Tokio worker's comparatively small stack
    // inside serde_json. Keep that recursive work on an explicitly bounded stack.
    std::thread::Builder::new()
        .name("service-state-json".to_string())
        .stack_size(SERVICE_STATE_JSON_STACK_BYTES)
        .spawn(move || serde_json::from_str::<ServiceState>(&raw))
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
                state.refresh_derived_views();
                state.remove_builtin_entity_defaults_for_persistence();
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
                })
            })
            .map_err(|err| format!("Failed to start service state JSON serializer: {err}"))?
            .join()
            .map_err(|_| "Service state JSON serializer panicked".to_string())?
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
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = acquire_service_state_process_lock(lock, deadline)?;
        let _file_guard = self
            .store
            .state_path()
            .map(|path| {
                acquire_service_state_file_lock_until(
                    path,
                    ServiceStateFileLockMode::Exclusive,
                    deadline,
                )
            })
            .transpose()?;
        self.store.load()
    }

    pub(crate) fn mutate_with_lock_timeout<R>(
        &self,
        timeout: Duration,
        mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
    ) -> Result<R, String> {
        let deadline = Instant::now() + timeout.max(Duration::from_millis(1));
        let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = acquire_service_state_process_lock(lock, deadline)?;
        let _file_guard = self
            .store
            .state_path()
            .map(|path| {
                acquire_service_state_file_lock_until(
                    path,
                    ServiceStateFileLockMode::Exclusive,
                    deadline,
                )
            })
            .transpose()?;
        let mut state = self.store.load()?;
        let result = mutator(&mut state)?;
        self.store.save(&state)?;
        Ok(result)
    }
}

fn acquire_service_state_process_lock(
    lock: &'static Mutex<()>,
    deadline: Instant,
) -> Result<MutexGuard<'static, ()>, String> {
    loop {
        match lock.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(_)) => {
                return Err("Service state mutation lock was poisoned".to_string())
            }
            Err(TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err("service_state_lock_timeout: process mutation lock".to_string())
            }
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
        }
    }
}

pub fn default_service_state_path() -> Result<PathBuf, String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Could not determine home directory for service state".to_string());
    };
    Ok(home
        .join(".agent-browser")
        .join(SERVICE_DIR)
        .join(SERVICE_STATE_FILENAME))
}

/// Load a stable point-in-time snapshot of the default JSON service state.
///
/// Readers take the same mutex as mutators so they do not observe a snapshot
/// while a serialized read-modify-write operation is in progress. This does
/// not make the snapshot live after it is returned; callers that later write
/// must still use merge-aware mutation helpers.
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
    let state_temp = match write_temporary(state_path, &transaction.state_payload, "service state")
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
    replace_from_temporary(&handoff_temp, &handoff_path, "remote-view handoff registry")?;
    if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
        replace_from_temporary(
            owner_registry_temp,
            &owner_registry_path,
            "runtime-owner registry",
        )?;
    }
    replace_from_temporary(&state_temp, state_path, "service state")?;
    fs::remove_file(&transaction_path).map_err(|error| {
        format!(
            "Failed to clear service state transaction {}: {error}",
            transaction_path.display()
        )
    })
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
    let transaction_path = service_state_transaction_path(&store.path);
    let prior_handoff = fs::read(&handoff_path).ok();
    let prior_owner_registry = fs::read(&owner_registry_path).ok();

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
    if let Err(error) = store.fail_at(ServiceStateSaveBoundary::StateWrite) {
        let _ = fs::remove_file(&handoff_temp);
        if let Some(owner_registry_temp) = owner_registry_temp.as_ref() {
            let _ = fs::remove_file(owner_registry_temp);
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

    let state_result = store
        .fail_at(ServiceStateSaveBoundary::StateRename)
        .and_then(|()| replace_from_temporary(&state_temp, &store.path, "service state"));
    if let Err(error) = state_result {
        let owner_restore_result =
            restore_file(&owner_registry_path, prior_owner_registry.as_deref());
        let handoff_restore_result = restore_file(&handoff_path, prior_handoff.as_deref());
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
    fs::remove_file(&transaction_path).map_err(|error| {
        format!(
            "Failed to clear service state transaction {}: {error}",
            transaction_path.display()
        )
    })
}

#[derive(Debug, Clone, Copy)]
enum ServiceStateFileLockMode {
    Shared,
    Exclusive,
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
) -> Result<File, String> {
    acquire_service_state_file_lock_until(
        state_path,
        mode,
        Instant::now() + DEFAULT_SERVICE_STATE_LOCK_TIMEOUT,
    )
}

fn acquire_service_state_file_lock_until(
    state_path: &Path,
    mode: ServiceStateFileLockMode,
    deadline: Instant,
) -> Result<File, String> {
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
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err(format!(
                    "service_state_lock_timeout: {}",
                    lock_path.display()
                ))
            }
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
            Err(error) => {
                return Err(format!(
                    "Failed to acquire service state lock {}: {}",
                    lock_path.display(),
                    error
                ))
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

    #[test]
    fn missing_state_file_loads_default_state() {
        let store = JsonServiceStateStore::new(unique_state_path("missing-service-state"));

        let state = store.load().expect("missing state should load default");

        assert_eq!(state, ServiceState::default());
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

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_stack_size(128 * 1024)
            .enable_all()
            .build()
            .expect("fixture runtime should build");
        let loaded = runtime
            .block_on(async move {
                tokio::spawn(async move {
                    let stack_pressure = [0_u8; 64 * 1024];
                    std::hint::black_box(&stack_pressure);
                    let result = store.load();
                    if let Ok(state) = result.as_ref() {
                        store
                            .save(state)
                            .expect("large service state should save from a constrained worker");
                    }
                    std::hint::black_box(&stack_pressure);
                    result
                })
                .await
                .expect("service-state loader task should not crash")
            })
            .expect("large fixture should load");

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

        let started = Instant::now();
        let error = repository
            .load_snapshot_with_lock_timeout(Duration::from_millis(10))
            .expect_err("repository must not wait indefinitely for the file lock");

        assert!(error.contains("service_state_lock_timeout"));
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(held);
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
    fn three_file_service_state_commit_is_atomic_at_every_write_and_rename_boundary() {
        for boundary in [
            ServiceStateSaveBoundary::HandoffWrite,
            ServiceStateSaveBoundary::OwnerRegistryWrite,
            ServiceStateSaveBoundary::StateWrite,
            ServiceStateSaveBoundary::HandoffRename,
            ServiceStateSaveBoundary::OwnerRegistryRename,
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
                    runtime_owner_registry: RuntimeOwnerRegistry::from_owner(runtime_owner(
                        "before",
                    )),
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
                    runtime_owner_registry: RuntimeOwnerRegistry::from_owner(runtime_owner(
                        "after",
                    )),
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
