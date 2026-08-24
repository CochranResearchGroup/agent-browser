//! User-scoped runtime-host identity and logical lane registry.
//!
//! Named sessions remain public identities, but host admission routes them
//! through one authenticated endpoint. Each lane owns its own serialized
//! control-plane worker, so a stalled lane cannot block unrelated lanes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub(crate) const RUNTIME_HOST_ENDPOINT_KEY: &str = "runtime-host";
pub(crate) const RUNTIME_HOST_LANE_FIELD: &str = "_agentBrowserRuntimeLane";
pub(crate) const RUNTIME_HOST_LANE_CONFIG_FIELD: &str = "_agentBrowserRuntimeLaneConfig";
pub(crate) const RUNTIME_HOST_ENV: &str = "AGENT_BROWSER_RUNTIME_HOST";
pub(crate) const RUNTIME_HOST_PROCESS_ENV: &str = "AGENT_BROWSER_RUNTIME_HOST_PROCESS";
pub(crate) const DEFAULT_MAX_RUNTIME_LANES: usize = 64;

pub(crate) fn admission_enabled() -> bool {
    match std::env::var(RUNTIME_HOST_ENV) {
        Ok(value) => matches!(value.trim(), "1" | "true" | "yes"),
        Err(_) => {
            #[cfg(test)]
            {
                false
            }
            #[cfg(not(test))]
            {
                crate::runtime_host_ingress::selected_socket_dir().is_some()
            }
        }
    }
}

pub(crate) fn endpoint_key(session: &str) -> &str {
    if admission_enabled() {
        RUNTIME_HOST_ENDPOINT_KEY
    } else {
        session
    }
}

pub(crate) fn attach_lane(mut command: Value, session: &str) -> Value {
    if admission_enabled() {
        if let Some(object) = command.as_object_mut() {
            object.insert(
                RUNTIME_HOST_LANE_FIELD.to_string(),
                Value::String(session.to_string()),
            );
        }
    }
    command
}

pub(crate) fn attach_lane_config(mut command: Value, config: &RuntimeLaneConfig) -> Value {
    if admission_enabled() {
        if let Some(object) = command.as_object_mut() {
            if let Ok(value) = serde_json::to_value(config) {
                object.insert(RUNTIME_HOST_LANE_CONFIG_FIELD.to_string(), value);
            }
        }
    }
    command
}

pub(crate) fn take_lane(command: &mut Value, fallback: &str) -> Result<String, String> {
    let lane = command
        .as_object_mut()
        .and_then(|object| object.remove(RUNTIME_HOST_LANE_FIELD))
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| fallback.to_string());
    if !crate::validation::is_valid_session_name(&lane) {
        return Err(format!("runtime_host_lane_invalid: {lane}"));
    }
    Ok(lane)
}

pub(crate) fn take_lane_config(command: &mut Value) -> Result<Option<RuntimeLaneConfig>, String> {
    let Some(value) = command
        .as_object_mut()
        .and_then(|object| object.remove(RUNTIME_HOST_LANE_CONFIG_FIELD))
    else {
        return Ok(None);
    };
    serde_json::from_value(value)
        .map(Some)
        .map_err(|error| format!("runtime_host_lane_config_invalid: {error}"))
}

/// Whether daemon-lane profile defaults may be projected into this command.
///
/// Desktop observation and interaction actions resolve an already retained
/// service browser by opaque identity. Their bounded contracts deliberately
/// reject profile and runtime-profile routing, so lane defaults must remain
/// control-plane metadata instead of becoming caller-visible action fields.
pub(crate) fn command_accepts_lane_profile_defaults(command: &Value) -> bool {
    !matches!(
        command.get("action").and_then(Value::as_str),
        Some(
            "desktop_capture"
                | "desktop_locate"
                | "desktop_evidence_observe"
                | "desktop_prompt_observe"
                | "desktop_interact"
        )
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeLaneConfig {
    pub(crate) runtime_profile: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) session_name: Option<String>,
    pub(crate) allowed_domains: Option<String>,
    pub(crate) action_policy: Option<String>,
    pub(crate) confirm_actions: Option<String>,
    pub(crate) no_auto_dialog: bool,
    pub(crate) engine: Option<String>,
    pub(crate) default_timeout_ms: Option<u64>,
    pub(crate) stream_port: Option<u16>,
    pub(crate) service_reconcile_interval_ms: Option<u64>,
    pub(crate) service_job_timeout_ms: Option<u64>,
    pub(crate) service_monitor_interval_ms: Option<u64>,
    pub(crate) recovery_retry_budget: u64,
    pub(crate) recovery_base_backoff_ms: u64,
    pub(crate) recovery_max_backoff_ms: u64,
    pub(crate) recovery_retry_budget_source: String,
    pub(crate) recovery_base_backoff_ms_source: String,
    pub(crate) recovery_max_backoff_ms_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeHostManifest {
    pub(crate) schema_version: String,
    pub(crate) host_id: String,
    pub(crate) pid: u32,
    pub(crate) executable_generation: String,
    pub(crate) socket_identity: String,
    pub(crate) authentication_record: String,
    pub(crate) max_lanes: usize,
}

pub(crate) fn write_manifest(
    socket_dir: &Path,
    executable_generation: String,
) -> Result<PathBuf, String> {
    let path = socket_dir.join("runtime-host.json");
    let socket_path = socket_dir.join(format!("{RUNTIME_HOST_ENDPOINT_KEY}.sock"));
    let manifest = RuntimeHostManifest {
        schema_version: "agent-browser.runtime-host.v1".to_string(),
        host_id: format!("runtime-host:{}", std::process::id()),
        pid: std::process::id(),
        executable_generation,
        socket_identity: runtime_host_socket_identity(&socket_path)?,
        authentication_record: format!("{RUNTIME_HOST_ENDPOINT_KEY}.token"),
        max_lanes: DEFAULT_MAX_RUNTIME_LANES,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("runtime_host_manifest_encode_failed: {error}"))?;
    fs::write(&path, bytes)
        .map_err(|error| format!("runtime_host_manifest_write_failed: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("runtime_host_manifest_permissions_failed: {error}"))?;
    }
    Ok(path)
}

#[cfg(unix)]
fn runtime_host_socket_identity(path: &Path) -> Result<String, String> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path)
        .map_err(|error| format!("runtime_host_socket_identity_failed: {error}"))?;
    Ok(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn runtime_host_socket_identity(path: &Path) -> Result<String, String> {
    Ok(path.display().to_string())
}

pub(crate) fn remove_manifest_if_owned(path: &Path) {
    let owned = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<RuntimeHostManifest>(&bytes).ok())
        .is_some_and(|manifest| manifest.pid == std::process::id());
    if owned {
        let _ = fs::remove_file(path);
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeLaneRegistry<T> {
    max_lanes: usize,
    lanes: RwLock<BTreeMap<String, T>>,
}

impl<T: Clone> RuntimeLaneRegistry<T> {
    pub(crate) fn new(max_lanes: usize) -> Self {
        Self {
            max_lanes,
            lanes: RwLock::new(BTreeMap::new()),
        }
    }

    pub(crate) fn get(&self, lane: &str) -> Option<T> {
        self.lanes.read().ok()?.get(lane).cloned()
    }

    pub(crate) fn insert(&self, lane: String, value: T) -> Result<T, String> {
        let mut lanes = self
            .lanes
            .write()
            .map_err(|_| "runtime_host_lane_registry_poisoned".to_string())?;
        if let Some(existing) = lanes.get(&lane) {
            return Ok(existing.clone());
        }
        if lanes.len() >= self.max_lanes {
            return Err("runtime_host_lane_capacity_exhausted".to_string());
        }
        lanes.insert(lane, value.clone());
        Ok(value)
    }

    pub(crate) fn remove(&self, lane: &str) -> Option<T> {
        self.lanes.write().ok()?.remove(lane)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lanes
            .read()
            .map(|lanes| lanes.is_empty())
            .unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) fn snapshot(&self) -> Vec<String> {
        self.lanes
            .read()
            .map(|lanes| lanes.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn take_all(&self) -> Vec<T> {
        self.lanes
            .write()
            .map(|mut lanes| std::mem::take(&mut *lanes).into_values().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use std::sync::Arc;

    #[test]
    fn host_admission_maps_named_sessions_to_one_endpoint() {
        let guard = EnvGuard::new(&[RUNTIME_HOST_ENV]);
        guard.set(RUNTIME_HOST_ENV, "1");
        assert_eq!(endpoint_key("alpha"), RUNTIME_HOST_ENDPOINT_KEY);
        assert_eq!(endpoint_key("beta"), RUNTIME_HOST_ENDPOINT_KEY);
        let command = attach_lane(serde_json::json!({"action": "get_url"}), "beta");
        assert_eq!(command[RUNTIME_HOST_LANE_FIELD], "beta");
    }

    #[test]
    fn disabled_admission_preserves_the_forwarding_only_legacy_endpoint() {
        let guard = EnvGuard::new(&[RUNTIME_HOST_ENV]);
        guard.set(RUNTIME_HOST_ENV, "0");
        assert_eq!(endpoint_key("alpha"), "alpha");
        assert!(attach_lane(serde_json::json!({}), "alpha")
            .get(RUNTIME_HOST_LANE_FIELD)
            .is_none());
    }

    #[test]
    fn lane_admission_rejects_invalid_or_injected_names() {
        let mut command = serde_json::json!({
            RUNTIME_HOST_LANE_FIELD: "../other-user"
        });
        assert_eq!(
            take_lane(&mut command, "default").unwrap_err(),
            "runtime_host_lane_invalid: ../other-user"
        );
        assert!(command.get(RUNTIME_HOST_LANE_FIELD).is_none());
    }

    #[test]
    fn lane_configuration_round_trips_and_is_removed_before_dispatch() {
        let guard = EnvGuard::new(&[RUNTIME_HOST_ENV]);
        guard.set(RUNTIME_HOST_ENV, "1");
        let config = RuntimeLaneConfig {
            runtime_profile: Some("profile-a".to_string()),
            profile: Some("/tmp/profile-a".to_string()),
            session_name: Some("durable-auth".to_string()),
            allowed_domains: Some("example.com,*.example.com".to_string()),
            action_policy: Some("/tmp/policy.json".to_string()),
            confirm_actions: Some("click,fill".to_string()),
            no_auto_dialog: true,
            engine: Some("chrome".to_string()),
            default_timeout_ms: Some(12_345),
            stream_port: Some(39_716),
            service_reconcile_interval_ms: Some(1_000),
            service_job_timeout_ms: Some(2_000),
            service_monitor_interval_ms: Some(3_000),
            recovery_retry_budget: 4,
            recovery_base_backoff_ms: 500,
            recovery_max_backoff_ms: 5_000,
            recovery_retry_budget_source: "cli".to_string(),
            recovery_base_backoff_ms_source: "config".to_string(),
            recovery_max_backoff_ms_source: "env".to_string(),
        };
        let mut command = attach_lane_config(serde_json::json!({"action": "status"}), &config);
        assert_eq!(take_lane_config(&mut command).unwrap(), Some(config));
        assert!(command.get(RUNTIME_HOST_LANE_CONFIG_FIELD).is_none());
    }

    #[test]
    fn bounded_desktop_commands_do_not_accept_lane_profile_defaults() {
        for action in [
            "desktop_capture",
            "desktop_locate",
            "desktop_evidence_observe",
            "desktop_prompt_observe",
            "desktop_interact",
        ] {
            assert!(!command_accepts_lane_profile_defaults(
                &serde_json::json!({"action": action})
            ));
        }
        assert!(command_accepts_lane_profile_defaults(
            &serde_json::json!({"action": "navigate"})
        ));
        assert!(command_accepts_lane_profile_defaults(
            &serde_json::json!({"action": "remote_view_open"})
        ));
    }

    #[tokio::test]
    async fn three_lanes_share_one_registry_and_a_stalled_lane_does_not_starve_another() {
        let registry = RuntimeLaneRegistry::new(3);
        let alpha = registry
            .insert("alpha".to_string(), Arc::new(tokio::sync::Mutex::new(())))
            .unwrap();
        registry
            .insert("beta".to_string(), Arc::new(tokio::sync::Mutex::new(())))
            .unwrap();
        let gamma = registry
            .insert("gamma".to_string(), Arc::new(tokio::sync::Mutex::new(())))
            .unwrap();
        let _alpha_stalled = alpha.lock().await;
        let _gamma_available =
            tokio::time::timeout(std::time::Duration::from_millis(50), gamma.lock())
                .await
                .expect("an unrelated lane must remain schedulable");
        assert_eq!(registry.snapshot(), vec!["alpha", "beta", "gamma"]);
        assert_eq!(
            registry
                .insert("delta".to_string(), Arc::new(tokio::sync::Mutex::new(())))
                .unwrap_err(),
            "runtime_host_lane_capacity_exhausted"
        );
    }

    #[test]
    fn duplicate_lane_admission_is_idempotent_and_lane_close_is_scoped() {
        let registry = RuntimeLaneRegistry::new(3);
        assert_eq!(registry.insert("alpha".to_string(), 1).unwrap(), 1);
        assert_eq!(registry.insert("alpha".to_string(), 2).unwrap(), 1);
        assert_eq!(registry.insert("beta".to_string(), 3).unwrap(), 3);
        assert_eq!(registry.snapshot(), vec!["alpha", "beta"]);
        assert_eq!(registry.remove("alpha"), Some(1));
        assert_eq!(registry.get("beta"), Some(3));
    }

    #[test]
    fn taking_all_lanes_is_an_idempotent_host_shutdown_boundary() {
        let registry = RuntimeLaneRegistry::new(2);
        registry.insert("alpha".to_string(), 1).unwrap();
        registry.insert("beta".to_string(), 2).unwrap();
        assert_eq!(registry.take_all(), vec![1, 2]);
        assert!(registry.take_all().is_empty());
        assert!(registry.snapshot().is_empty());
    }
}
