//! Persistent service-state storage.
//!
//! The first service-mode store is JSON-backed and intentionally small. It gives
//! later lifecycle work a durable contract without forcing a database choice yet.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::service_model::ServiceState;

const SERVICE_DIR: &str = "service";
const SERVICE_STATE_FILENAME: &str = "state.json";
static SERVICE_STATE_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub trait ServiceStateStore {
    fn load(&self) -> Result<ServiceState, String>;
    fn save(&self, state: &ServiceState) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct JsonServiceStateStore {
    path: PathBuf,
}

impl JsonServiceStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> Result<PathBuf, String> {
        default_service_state_path()
    }

    #[cfg(test)]
    fn path(&self) -> &Path {
        &self.path
    }
}

impl ServiceStateStore for JsonServiceStateStore {
    fn load(&self) -> Result<ServiceState, String> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ServiceState::default())
            }
            Err(err) => {
                return Err(format!(
                    "Failed to read service state {}: {}",
                    self.path.display(),
                    err
                ))
            }
        };

        let mut state: ServiceState = serde_json::from_str(&raw).map_err(|err| {
            format!(
                "Invalid service state JSON {}: {}",
                self.path.display(),
                err
            )
        })?;
        state.refresh_derived_views();
        Ok(state)
    }

    fn save(&self, state: &ServiceState) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "Failed to create service state directory {}: {}",
                    parent.display(),
                    err
                )
            })?;
        }

        let mut normalized = state.clone();
        normalized.refresh_derived_views();
        let serialized = serde_json::to_string_pretty(&normalized)
            .map_err(|err| format!("Failed to serialize service state: {}", err))?;
        let temp_path = temp_state_path(&self.path);
        fs::write(&temp_path, format!("{}\n", serialized)).map_err(|err| {
            format!(
                "Failed to write temporary service state {}: {}",
                temp_path.display(),
                err
            )
        })?;
        fs::rename(&temp_path, &self.path).map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            format!(
                "Failed to replace service state {}: {}",
                self.path.display(),
                err
            )
        })?;

        Ok(())
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
    let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .map_err(|_| "Service state mutation lock was poisoned".to_string())?;
    let store = JsonServiceStateStore::new(default_service_state_path()?);
    store.load()
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
    let lock = SERVICE_STATE_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .map_err(|_| "Service state mutation lock was poisoned".to_string())?;
    let store = JsonServiceStateStore::new(default_service_state_path()?);
    let mut state = store.load()?;
    let result = mutator(&mut state)?;
    store.save(&state)?;
    Ok(result)
}

fn temp_state_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SERVICE_STATE_FILENAME);
    path.with_file_name(format!("{}.tmp.{}", file_name, std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserHealth, BrowserHost, BrowserProcess, SitePolicy};
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

        assert_eq!(loaded, state);
        assert_eq!(store.path(), path.as_path());
        let _ = fs::remove_dir_all(path.parent().unwrap());
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
