use crate::color;
use crate::native::service_health::{
    BrowserRecoveryPolicyConfig, BrowserRecoveryPolicyValueSource,
};
use crate::native::service_model::{
    BrowserBuild, BrowserProfile, BrowserSession, ServiceProvider, ServiceState, SiteMonitor,
    SitePolicy,
};
use crate::native::service_store::load_default_service_state_snapshot;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = ".agent-browser";
const CONFIG_FILENAME: &str = "config.json";
const PROJECT_CONFIG_FILENAME: &str = "agent-browser.json";
/// Default daemon background interval for persisted service browser-health probes.
const DEFAULT_SERVICE_RECONCILE_INTERVAL_MS: u64 = 60_000;
/// Default daemon background interval for due service monitor checks.
const DEFAULT_SERVICE_MONITOR_INTERVAL_MS: u64 = 60_000;

/// Parse idle timeout from user-friendly format.
/// Supports: "10s" (seconds), "3m" (minutes), "1h" (hours), or raw milliseconds.
fn parse_idle_timeout(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty idle timeout".to_string());
    }

    // If the value ends with a unit suffix, convert it to milliseconds.
    if s.chars().last().is_some_and(|c| c.is_ascii_alphabetic()) {
        let (num_str, unit) = s.split_at(s.len() - 1);
        let num: u64 = num_str.parse().map_err(|_| "Invalid number")?;

        let ms = match unit {
            "s" => num * 1000,
            "m" => num * 60 * 1000,
            "h" => num * 60 * 60 * 1000,
            _ => return Err("Invalid idle timeout unit (use s, m, h, or raw ms)".to_string()),
        };
        return Ok(ms.to_string());
    }

    // Pure numbers are already expressed in milliseconds.
    s.parse::<u64>().map_err(|_| "Invalid idle timeout")?;
    Ok(s.to_string())
}

fn parse_idle_timeout_value(value: Option<String>, source: &str) -> Option<String> {
    value.and_then(|raw| match parse_idle_timeout(&raw) {
        Ok(ms) => Some(ms),
        Err(e) => {
            eprintln!(
                "{} invalid idle timeout from {}: {}",
                color::warning_indicator(),
                source,
                e
            );
            None
        }
    })
}

fn service_reconcile_interval_from_sources(config: &Config) -> Option<u64> {
    if let Ok(raw) = env::var("AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS") {
        return match raw.parse::<u64>() {
            Ok(0) => None,
            Ok(ms) => Some(ms),
            Err(_) => {
                eprintln!(
                    "{} invalid service reconcile interval from AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: expected milliseconds, got {}",
                    color::warning_indicator(),
                    raw
                );
                Some(DEFAULT_SERVICE_RECONCILE_INTERVAL_MS)
            }
        };
    }

    if let Some(ms) = config
        .service
        .as_ref()
        .and_then(|service| service.reconcile_interval_ms)
    {
        return (ms > 0).then_some(ms);
    }

    Some(DEFAULT_SERVICE_RECONCILE_INTERVAL_MS)
}

fn service_job_timeout_from_sources(config: &Config) -> Option<u64> {
    if let Ok(raw) = env::var("AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS") {
        return match raw.parse::<u64>() {
            Ok(0) => None,
            Ok(ms) => Some(ms),
            Err(_) => {
                eprintln!(
                    "{} invalid service job timeout from AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS: expected milliseconds, got {}",
                    color::warning_indicator(),
                    raw
                );
                None
            }
        };
    }

    config
        .service
        .as_ref()
        .and_then(|service| service.job_timeout_ms)
        .filter(|ms| *ms > 0)
}

fn service_monitor_interval_from_sources(config: &Config) -> Option<u64> {
    if let Ok(raw) = env::var("AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS") {
        return match raw.parse::<u64>() {
            Ok(0) => None,
            Ok(ms) => Some(ms),
            Err(_) => {
                eprintln!(
                    "{} invalid service monitor interval from AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS: expected milliseconds, got {}",
                    color::warning_indicator(),
                    raw
                );
                Some(DEFAULT_SERVICE_MONITOR_INTERVAL_MS)
            }
        };
    }

    if let Some(ms) = config
        .service
        .as_ref()
        .and_then(|service| service.monitor_interval_ms)
    {
        return (ms > 0).then_some(ms);
    }

    Some(DEFAULT_SERVICE_MONITOR_INTERVAL_MS)
}

fn service_recovery_value_from_sources(
    env_name: &str,
    config_value: Option<u64>,
    default_value: u64,
) -> (u64, BrowserRecoveryPolicyValueSource) {
    if let Ok(raw) = env::var(env_name) {
        return match raw.parse::<u64>() {
            Ok(value) => (value, BrowserRecoveryPolicyValueSource::Env),
            Err(_) => {
                eprintln!(
                    "{} invalid service recovery value from {}: expected unsigned integer, got {}",
                    color::warning_indicator(),
                    env_name,
                    raw
                );
                (default_value, BrowserRecoveryPolicyValueSource::Default)
            }
        };
    }

    if let Some(value) = config_value {
        (value, BrowserRecoveryPolicyValueSource::Config)
    } else {
        (default_value, BrowserRecoveryPolicyValueSource::Default)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeProfileLaunchConfig {
    pub headed: Option<bool>,
    pub leave_open: Option<bool>,
    pub executable_path: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub profile: Option<String>,
    pub state: Option<String>,
    pub proxy: Option<String>,
    pub proxy_bypass: Option<String>,
    pub args: Option<String>,
    pub user_agent: Option<String>,
    pub ignore_https_errors: Option<bool>,
    pub allow_file_access: Option<bool>,
    pub color_scheme: Option<String>,
    pub download_path: Option<String>,
    pub engine: Option<String>,
    pub screenshot_dir: Option<String>,
    pub screenshot_quality: Option<u32>,
    pub screenshot_format: Option<String>,
    pub no_auto_dialog: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeProfileAuthConfig {
    pub session_name: Option<String>,
    pub manual_login_preferred: Option<bool>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeProfileServiceConfig {
    pub manual_login_preferred: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeProfilePreferencesConfig {
    pub wait_for_network_idle: Option<bool>,
    pub default_viewport: Option<String>,
    pub annotation_style: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeProfileConfig {
    pub user_data_dir: Option<String>,
    pub browser_family: Option<String>,
    pub launch: Option<RuntimeProfileLaunchConfig>,
    pub auth: Option<RuntimeProfileAuthConfig>,
    pub services: Option<HashMap<String, RuntimeProfileServiceConfig>>,
    pub preferences: Option<RuntimeProfilePreferencesConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserBuildManifestConfig {
    #[serde(alias = "path")]
    pub manifest_path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceConfig {
    pub profiles: Option<BTreeMap<String, BrowserProfile>>,
    pub sessions: Option<BTreeMap<String, BrowserSession>>,
    pub monitors: Option<BTreeMap<String, SiteMonitor>>,
    pub site_policies: Option<BTreeMap<String, SitePolicy>>,
    pub providers: Option<BTreeMap<String, ServiceProvider>>,
    pub reconcile_interval_ms: Option<u64>,
    pub job_timeout_ms: Option<u64>,
    pub monitor_interval_ms: Option<u64>,
    pub recovery_retry_budget: Option<u64>,
    pub recovery_base_backoff_ms: Option<u64>,
    pub recovery_max_backoff_ms: Option<u64>,
    pub default_browser_build: Option<BrowserBuild>,
    pub browser_build_manifests: Option<BTreeMap<String, BrowserBuildManifestConfig>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub default_runtime_profile: Option<String>,
    pub runtime_profiles: Option<HashMap<String, RuntimeProfileConfig>>,
    pub service_defaults: Option<HashMap<String, RuntimeProfileServiceConfig>>,
    pub service: Option<ServiceConfig>,
    pub headed: Option<bool>,
    pub leave_open: Option<bool>,
    pub json: Option<bool>,
    pub debug: Option<bool>,
    pub session: Option<String>,
    pub runtime_profile: Option<String>,
    pub session_name: Option<String>,
    pub executable_path: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub profile: Option<String>,
    pub state: Option<String>,
    pub proxy: Option<String>,
    pub proxy_bypass: Option<String>,
    pub args: Option<String>,
    pub user_agent: Option<String>,
    pub provider: Option<String>,
    pub device: Option<String>,
    pub ignore_https_errors: Option<bool>,
    pub allow_file_access: Option<bool>,
    pub cdp: Option<String>,
    pub auto_connect: Option<bool>,
    pub headers: Option<String>,
    pub annotate: Option<bool>,
    pub color_scheme: Option<String>,
    pub download_path: Option<String>,
    pub content_boundaries: Option<bool>,
    pub max_output: Option<usize>,
    pub allowed_domains: Option<Vec<String>>,
    pub action_policy: Option<String>,
    pub confirm_actions: Option<String>,
    pub confirm_interactive: Option<bool>,
    pub engine: Option<String>,
    pub screenshot_dir: Option<String>,
    pub screenshot_quality: Option<u32>,
    pub screenshot_format: Option<String>,
    pub idle_timeout: Option<String>,
    pub no_auto_dialog: Option<bool>,
    pub model: Option<String>,
    pub default_viewport: Option<String>,
}

impl Config {
    pub fn service_state_snapshot(&self) -> ServiceState {
        let Some(service) = self.service.as_ref() else {
            return ServiceState::default();
        };

        let mut state = ServiceState {
            profiles: service.profiles.clone().unwrap_or_default(),
            sessions: service.sessions.clone().unwrap_or_default(),
            monitors: service.monitors.clone().unwrap_or_default(),
            site_policies: service.site_policies.clone().unwrap_or_default(),
            providers: service.providers.clone().unwrap_or_default(),
            default_browser_build: service.default_browser_build,
            ..ServiceState::default()
        };
        state.mark_config_entity_sources();
        state
    }

    fn merge(self, other: Config) -> Config {
        Config {
            default_runtime_profile: other
                .default_runtime_profile
                .or(self.default_runtime_profile),
            runtime_profiles: merge_runtime_profile_maps(
                self.runtime_profiles,
                other.runtime_profiles,
            ),
            service_defaults: merge_service_config_maps(
                self.service_defaults,
                other.service_defaults,
            ),
            service: merge_service_configs(self.service, other.service),
            headed: other.headed.or(self.headed),
            leave_open: other.leave_open.or(self.leave_open),
            json: other.json.or(self.json),
            debug: other.debug.or(self.debug),
            session: other.session.or(self.session),
            runtime_profile: other.runtime_profile.or(self.runtime_profile),
            session_name: other.session_name.or(self.session_name),
            executable_path: other.executable_path.or(self.executable_path),
            extensions: match (self.extensions, other.extensions) {
                (Some(mut a), Some(b)) => {
                    a.extend(b);
                    Some(a)
                }
                (a, b) => b.or(a),
            },
            profile: other.profile.or(self.profile),
            state: other.state.or(self.state),
            proxy: other.proxy.or(self.proxy),
            proxy_bypass: other.proxy_bypass.or(self.proxy_bypass),
            args: other.args.or(self.args),
            user_agent: other.user_agent.or(self.user_agent),
            provider: other.provider.or(self.provider),
            device: other.device.or(self.device),
            ignore_https_errors: other.ignore_https_errors.or(self.ignore_https_errors),
            allow_file_access: other.allow_file_access.or(self.allow_file_access),
            cdp: other.cdp.or(self.cdp),
            auto_connect: other.auto_connect.or(self.auto_connect),
            headers: other.headers.or(self.headers),
            annotate: other.annotate.or(self.annotate),
            color_scheme: other.color_scheme.or(self.color_scheme),
            download_path: other.download_path.or(self.download_path),
            content_boundaries: other.content_boundaries.or(self.content_boundaries),
            max_output: other.max_output.or(self.max_output),
            allowed_domains: other.allowed_domains.or(self.allowed_domains),
            action_policy: other.action_policy.or(self.action_policy),
            confirm_actions: other.confirm_actions.or(self.confirm_actions),
            confirm_interactive: other.confirm_interactive.or(self.confirm_interactive),
            engine: other.engine.or(self.engine),
            screenshot_dir: other.screenshot_dir.or(self.screenshot_dir),
            screenshot_quality: other.screenshot_quality.or(self.screenshot_quality),
            screenshot_format: other.screenshot_format.or(self.screenshot_format),
            idle_timeout: other.idle_timeout.or(self.idle_timeout),
            no_auto_dialog: other.no_auto_dialog.or(self.no_auto_dialog),
            model: other.model.or(self.model),
            default_viewport: other.default_viewport.or(self.default_viewport),
        }
    }
}

fn service_state_from_store(configured: ServiceState) -> ServiceState {
    let mut state = match load_default_service_state_snapshot() {
        Ok(state) => state,
        Err(err) => {
            eprintln!("{} {}", color::warning_indicator(), err);
            ServiceState::default()
        }
    };
    state.overlay_configured_entities(configured);
    state
}

fn merge_runtime_profile_maps(
    base: Option<HashMap<String, RuntimeProfileConfig>>,
    overlay: Option<HashMap<String, RuntimeProfileConfig>>,
) -> Option<HashMap<String, RuntimeProfileConfig>> {
    match (base, overlay) {
        (None, None) => None,
        (Some(map), None) | (None, Some(map)) => Some(map),
        (Some(mut base_map), Some(overlay_map)) => {
            for (name, config) in overlay_map {
                let merged = if let Some(existing) = base_map.remove(&name) {
                    merge_runtime_profile_config(existing, config)
                } else {
                    config
                };
                base_map.insert(name, merged);
            }
            Some(base_map)
        }
    }
}

fn merge_service_config_maps(
    base: Option<HashMap<String, RuntimeProfileServiceConfig>>,
    overlay: Option<HashMap<String, RuntimeProfileServiceConfig>>,
) -> Option<HashMap<String, RuntimeProfileServiceConfig>> {
    match (base, overlay) {
        (None, None) => None,
        (Some(map), None) | (None, Some(map)) => Some(map),
        (Some(mut base_map), Some(overlay_map)) => {
            for (name, config) in overlay_map {
                let merged = if let Some(existing) = base_map.remove(&name) {
                    RuntimeProfileServiceConfig {
                        manual_login_preferred: config
                            .manual_login_preferred
                            .or(existing.manual_login_preferred),
                    }
                } else {
                    config
                };
                base_map.insert(name, merged);
            }
            Some(base_map)
        }
    }
}

fn merge_service_model_maps<T>(
    base: Option<BTreeMap<String, T>>,
    overlay: Option<BTreeMap<String, T>>,
) -> Option<BTreeMap<String, T>> {
    match (base, overlay) {
        (None, None) => None,
        (Some(map), None) | (None, Some(map)) => Some(map),
        (Some(mut base_map), Some(overlay_map)) => {
            base_map.extend(overlay_map);
            Some(base_map)
        }
    }
}

fn merge_service_configs(
    base: Option<ServiceConfig>,
    overlay: Option<ServiceConfig>,
) -> Option<ServiceConfig> {
    match (base, overlay) {
        (None, None) => None,
        (Some(config), None) | (None, Some(config)) => Some(config),
        (Some(base), Some(overlay)) => Some(ServiceConfig {
            profiles: merge_service_model_maps(base.profiles, overlay.profiles),
            sessions: merge_service_model_maps(base.sessions, overlay.sessions),
            monitors: merge_service_model_maps(base.monitors, overlay.monitors),
            site_policies: merge_service_model_maps(base.site_policies, overlay.site_policies),
            providers: merge_service_model_maps(base.providers, overlay.providers),
            reconcile_interval_ms: overlay.reconcile_interval_ms.or(base.reconcile_interval_ms),
            job_timeout_ms: overlay.job_timeout_ms.or(base.job_timeout_ms),
            monitor_interval_ms: overlay.monitor_interval_ms.or(base.monitor_interval_ms),
            recovery_retry_budget: overlay.recovery_retry_budget.or(base.recovery_retry_budget),
            recovery_base_backoff_ms: overlay
                .recovery_base_backoff_ms
                .or(base.recovery_base_backoff_ms),
            recovery_max_backoff_ms: overlay
                .recovery_max_backoff_ms
                .or(base.recovery_max_backoff_ms),
            default_browser_build: overlay.default_browser_build.or(base.default_browser_build),
            browser_build_manifests: merge_service_model_maps(
                base.browser_build_manifests,
                overlay.browser_build_manifests,
            ),
        }),
    }
}

fn merge_runtime_profile_config(
    base: RuntimeProfileConfig,
    overlay: RuntimeProfileConfig,
) -> RuntimeProfileConfig {
    RuntimeProfileConfig {
        user_data_dir: overlay.user_data_dir.or(base.user_data_dir),
        browser_family: overlay.browser_family.or(base.browser_family),
        launch: match (base.launch, overlay.launch) {
            (None, None) => None,
            (Some(cfg), None) | (None, Some(cfg)) => Some(cfg),
            (Some(base_launch), Some(overlay_launch)) => Some(RuntimeProfileLaunchConfig {
                headed: overlay_launch.headed.or(base_launch.headed),
                leave_open: overlay_launch.leave_open.or(base_launch.leave_open),
                executable_path: overlay_launch
                    .executable_path
                    .or(base_launch.executable_path),
                extensions: match (base_launch.extensions, overlay_launch.extensions) {
                    (Some(mut a), Some(b)) => {
                        a.extend(b);
                        Some(a)
                    }
                    (a, b) => b.or(a),
                },
                profile: overlay_launch.profile.or(base_launch.profile),
                state: overlay_launch.state.or(base_launch.state),
                proxy: overlay_launch.proxy.or(base_launch.proxy),
                proxy_bypass: overlay_launch.proxy_bypass.or(base_launch.proxy_bypass),
                args: overlay_launch.args.or(base_launch.args),
                user_agent: overlay_launch.user_agent.or(base_launch.user_agent),
                ignore_https_errors: overlay_launch
                    .ignore_https_errors
                    .or(base_launch.ignore_https_errors),
                allow_file_access: overlay_launch
                    .allow_file_access
                    .or(base_launch.allow_file_access),
                color_scheme: overlay_launch.color_scheme.or(base_launch.color_scheme),
                download_path: overlay_launch.download_path.or(base_launch.download_path),
                engine: overlay_launch.engine.or(base_launch.engine),
                screenshot_dir: overlay_launch.screenshot_dir.or(base_launch.screenshot_dir),
                screenshot_quality: overlay_launch
                    .screenshot_quality
                    .or(base_launch.screenshot_quality),
                screenshot_format: overlay_launch
                    .screenshot_format
                    .or(base_launch.screenshot_format),
                no_auto_dialog: overlay_launch.no_auto_dialog.or(base_launch.no_auto_dialog),
            }),
        },
        auth: match (base.auth, overlay.auth) {
            (None, None) => None,
            (Some(cfg), None) | (None, Some(cfg)) => Some(cfg),
            (Some(base_auth), Some(overlay_auth)) => Some(RuntimeProfileAuthConfig {
                session_name: overlay_auth.session_name.or(base_auth.session_name),
                manual_login_preferred: overlay_auth
                    .manual_login_preferred
                    .or(base_auth.manual_login_preferred),
            }),
        },
        services: merge_service_config_maps(base.services, overlay.services),
        preferences: match (base.preferences, overlay.preferences) {
            (None, None) => None,
            (Some(cfg), None) | (None, Some(cfg)) => Some(cfg),
            (Some(base_pref), Some(overlay_pref)) => Some(RuntimeProfilePreferencesConfig {
                wait_for_network_idle: overlay_pref
                    .wait_for_network_idle
                    .or(base_pref.wait_for_network_idle),
                default_viewport: overlay_pref.default_viewport.or(base_pref.default_viewport),
                annotation_style: overlay_pref.annotation_style.or(base_pref.annotation_style),
            }),
        },
    }
}

fn expand_config_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn browser_build_manifest_configs_from_sources(
    config: &Config,
) -> BTreeMap<String, BrowserBuildManifestConfig> {
    let mut manifests = config
        .service
        .as_ref()
        .and_then(|service| service.browser_build_manifests.clone())
        .unwrap_or_default();
    if let Ok(path) = env::var("AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH") {
        manifests.insert(
            "stealthcdp_chromium".to_string(),
            BrowserBuildManifestConfig {
                manifest_path: Some(path),
            },
        );
    }
    manifests
}

fn value_string(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn resolve_browser_build_manifests(
    manifests: &BTreeMap<String, BrowserBuildManifestConfig>,
) -> BTreeMap<String, Value> {
    manifests
        .iter()
        .map(|(build, config)| resolve_browser_build_manifest(build, config))
        .collect()
}

fn resolve_browser_build_manifest(
    build: &str,
    config: &BrowserBuildManifestConfig,
) -> (String, Value) {
    let Some(raw_manifest_path) = config.manifest_path.as_deref() else {
        return (
            build.to_string(),
            json!({
                "manifestPath": null,
                "manifestExists": false,
                "manifestValid": false,
                "ready": false,
                "warnings": [{
                    "code": "browser_build_manifest_path_missing",
                    "severity": "warning",
                    "message": "browser build manifest is configured without manifestPath"
                }]
            }),
        );
    };

    let manifest_path = expand_config_path(raw_manifest_path);
    if !manifest_path.is_file() {
        return (
            build.to_string(),
            json!({
                "manifestPath": manifest_path.display().to_string(),
                "manifestExists": false,
                "manifestValid": false,
                "ready": false,
                "warnings": [{
                    "code": "browser_build_manifest_missing",
                    "severity": "warning",
                    "message": "browser build manifest path does not exist"
                }]
            }),
        );
    }

    let manifest_text = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(err) => {
            return (
                build.to_string(),
                json!({
                    "manifestPath": manifest_path.display().to_string(),
                    "manifestExists": true,
                    "manifestValid": false,
                    "ready": false,
                    "warnings": [{
                        "code": "browser_build_manifest_unreadable",
                        "severity": "warning",
                        "message": format!("browser build manifest could not be read: {err}")
                    }]
                }),
            );
        }
    };

    let manifest = match serde_json::from_str::<Value>(&manifest_text) {
        Ok(value) => value,
        Err(err) => {
            return (
                build.to_string(),
                json!({
                    "manifestPath": manifest_path.display().to_string(),
                    "manifestExists": true,
                    "manifestValid": false,
                    "ready": false,
                    "warnings": [{
                        "code": "browser_build_manifest_invalid_json",
                        "severity": "warning",
                        "message": format!("browser build manifest is not valid JSON: {err}")
                    }]
                }),
            );
        }
    };

    let artifact_dir = manifest_path
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let executable_relative = value_string(&manifest, "/executable/relativePath");
    let executable_path = executable_relative
        .as_ref()
        .map(|relative| artifact_dir.join(relative));
    let executable_exists = executable_path.as_ref().is_some_and(|path| path.is_file());
    let smoke_relative = value_string(&manifest, "/smoke/relativePath");
    let smoke_path = smoke_relative
        .as_ref()
        .map(|relative| artifact_dir.join(relative));
    let smoke_value = smoke_path
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let smoke_success = smoke_value
        .as_ref()
        .and_then(|value| value.get("success"))
        .and_then(|value| value.as_bool());
    let navigator_webdriver = smoke_value
        .as_ref()
        .and_then(|value| value.pointer("/checks/navigatorWebdriver"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    let mut warnings = Vec::new();
    let schema = value_string(&manifest, "/schema");
    if schema.as_deref() != Some("chromium-stealthcdp.artifact.v1") {
        warnings.push(json!({
            "code": "browser_build_manifest_schema_unsupported",
            "severity": "warning",
            "message": "browser build manifest schema is unsupported"
        }));
    }
    if executable_relative.is_none() {
        warnings.push(json!({
            "code": "browser_build_manifest_executable_missing",
            "severity": "warning",
            "message": "browser build manifest does not provide executable.relativePath"
        }));
    } else if !executable_exists {
        warnings.push(json!({
            "code": "browser_build_manifest_executable_not_found",
            "severity": "warning",
            "message": "browser build manifest executable path does not exist"
        }));
    }
    if smoke_success != Some(true) || navigator_webdriver.as_deref() != Some("false") {
        warnings.push(json!({
            "code": "browser_build_manifest_smoke_not_fresh",
            "severity": "warning",
            "message": "browser build manifest smoke does not prove success with navigator.webdriver=false"
        }));
    }

    let ready = warnings.is_empty();
    (
        build.to_string(),
        json!({
            "manifestPath": manifest_path.display().to_string(),
            "manifestExists": true,
            "manifestValid": true,
            "ready": ready,
            "artifactDir": artifact_dir.display().to_string(),
            "artifactName": value_string(&manifest, "/artifactName"),
            "schema": schema,
            "chromeVersion": value_string(&manifest, "/chromeVersion"),
            "chromiumSha": value_string(&manifest, "/chromium/sourceSha"),
            "patchsetSha": value_string(&manifest, "/patchset/repoSha"),
            "executablePath": executable_path.map(|path| path.display().to_string()),
            "executablePathExists": executable_exists,
            "executableSha256": value_string(&manifest, "/executable/sha256"),
            "smokePath": smoke_path.map(|path| path.display().to_string()),
            "smokeSuccess": smoke_success,
            "navigatorWebdriver": navigator_webdriver,
            "warnings": warnings
        }),
    )
}

fn manifest_executable_path(
    manifest_status: &BTreeMap<String, Value>,
    build: &str,
) -> Option<String> {
    manifest_status
        .get(build)
        .and_then(|status| status.get("executablePath"))
        .and_then(|path| path.as_str())
        .map(|path| path.to_string())
}

fn selected_runtime_profile_from_sources(args: &[String], config: &Config) -> Option<String> {
    extract_named_value(args, "--runtime-profile")
        .or_else(|| env::var("AGENT_BROWSER_RUNTIME_PROFILE").ok())
        .or_else(|| config.runtime_profile.clone())
        .or_else(|| config.default_runtime_profile.clone())
}

fn extract_named_value(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}

fn apply_runtime_profile_overrides(config: &mut Config, runtime_profile_name: &str) {
    let Some(runtime_profiles) = config.runtime_profiles.as_ref() else {
        config.runtime_profile = Some(runtime_profile_name.to_string());
        return;
    };
    let Some(runtime_profile) = runtime_profiles.get(runtime_profile_name) else {
        config.runtime_profile = Some(runtime_profile_name.to_string());
        return;
    };

    if config.profile.is_none() {
        config.profile = runtime_profile.user_data_dir.clone();
    }

    if let Some(launch) = runtime_profile.launch.as_ref() {
        config.headed = launch.headed.or(config.headed);
        config.leave_open = launch.leave_open.or(config.leave_open);
        config.executable_path = launch
            .executable_path
            .clone()
            .or(config.executable_path.take());
        config.extensions = match (config.extensions.take(), launch.extensions.clone()) {
            (Some(mut a), Some(b)) => {
                a.extend(b);
                Some(a)
            }
            (a, b) => b.or(a),
        };
        config.profile = launch.profile.clone().or(config.profile.take());
        config.state = launch.state.clone().or(config.state.take());
        config.proxy = launch.proxy.clone().or(config.proxy.take());
        config.proxy_bypass = launch.proxy_bypass.clone().or(config.proxy_bypass.take());
        config.args = launch.args.clone().or(config.args.take());
        config.user_agent = launch.user_agent.clone().or(config.user_agent.take());
        config.ignore_https_errors = launch.ignore_https_errors.or(config.ignore_https_errors);
        config.allow_file_access = launch.allow_file_access.or(config.allow_file_access);
        config.color_scheme = launch.color_scheme.clone().or(config.color_scheme.take());
        config.download_path = launch.download_path.clone().or(config.download_path.take());
        config.engine = launch.engine.clone().or(config.engine.take());
        config.screenshot_dir = launch
            .screenshot_dir
            .clone()
            .or(config.screenshot_dir.take());
        config.screenshot_quality = launch.screenshot_quality.or(config.screenshot_quality);
        config.screenshot_format = launch
            .screenshot_format
            .clone()
            .or(config.screenshot_format.take());
        config.no_auto_dialog = launch.no_auto_dialog.or(config.no_auto_dialog);
    }

    if let Some(auth) = runtime_profile.auth.as_ref() {
        config.session_name = auth.session_name.clone().or(config.session_name.take());
    }

    if let Some(preferences) = runtime_profile.preferences.as_ref() {
        config.default_viewport = preferences
            .default_viewport
            .clone()
            .or(config.default_viewport.take());
    }

    config.runtime_profile = Some(runtime_profile_name.to_string());
}

fn normalize_service_config(config: &mut Config) {
    let Some(service) = config.service.as_mut() else {
        return;
    };

    if let Some(site_policies) = service.site_policies.as_mut() {
        for (id, policy) in site_policies {
            if policy.id.is_empty() {
                policy.id = id.clone();
            }
        }
    }

    if let Some(providers) = service.providers.as_mut() {
        for (id, provider) in providers {
            if provider.id.is_empty() {
                provider.id = id.clone();
            }
        }
    }
}

fn manual_login_preferred_services(config: &Config) -> Vec<String> {
    let mut services = config.service_defaults.clone().unwrap_or_default();

    if let Some(runtime_profile_name) = config.runtime_profile.as_deref() {
        if let Some(runtime_services) = config
            .runtime_profiles
            .as_ref()
            .and_then(|profiles| profiles.get(runtime_profile_name))
            .and_then(|profile| profile.services.clone())
        {
            services = merge_service_config_maps(Some(services), Some(runtime_services))
                .unwrap_or_default();
        }
    }

    let mut names = services
        .into_iter()
        .filter_map(|(name, service)| {
            service
                .manual_login_preferred
                .unwrap_or(false)
                .then(|| name.to_ascii_lowercase())
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn read_config_file(path: &Path) -> Option<Config> {
    let content = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Config>(&content) {
        Ok(mut config) => {
            config.idle_timeout = parse_idle_timeout_value(
                config.idle_timeout.take(),
                &format!("config file {}", path.display()),
            );
            normalize_service_config(&mut config);
            Some(config)
        }
        Err(e) => {
            eprintln!(
                "{} invalid config file {}: {}",
                color::warning_indicator(),
                path.display(),
                e
            );
            None
        }
    }
}

pub fn user_config_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|d| d.join(CONFIG_DIR).join(CONFIG_FILENAME))
        .ok_or_else(|| "Could not determine home directory for user config".to_string())
}

/// Create or update a runtime-profile entry in the user config without
/// rewriting unrelated config keys.
pub fn upsert_runtime_profile_in_user_config(
    runtime_profile_name: &str,
    user_data_dir: Option<&str>,
    set_default: bool,
    browser_family: Option<&str>,
) -> Result<PathBuf, String> {
    let path = user_config_path()?;
    let mut root = match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).map_err(|e| {
            format!(
                "Failed to parse existing user config {}: {}",
                path.display(),
                e
            )
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => {
            return Err(format!(
                "Failed to read user config {}: {}",
                path.display(),
                e
            ))
        }
    };

    if !root.is_object() {
        return Err(format!(
            "User config {} must contain a JSON object",
            path.display()
        ));
    }

    let root_obj = root.as_object_mut().expect("checked above");
    let runtime_profiles = root_obj
        .entry("runtimeProfiles")
        .or_insert_with(|| Value::Object(Map::new()));

    if !runtime_profiles.is_object() {
        return Err(format!(
            "User config {} has a non-object runtimeProfiles field",
            path.display()
        ));
    }

    let runtime_profiles_obj = runtime_profiles
        .as_object_mut()
        .expect("runtimeProfiles must be object");
    let profile_entry = runtime_profiles_obj
        .entry(runtime_profile_name.to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    if !profile_entry.is_object() {
        return Err(format!(
            "User config {} has a non-object runtimeProfiles.{} field",
            path.display(),
            runtime_profile_name
        ));
    }

    if let Some(user_data_dir) = user_data_dir {
        profile_entry
            .as_object_mut()
            .expect("profile entry must be object")
            .insert(
                "userDataDir".to_string(),
                Value::String(user_data_dir.to_string()),
            );
    }

    if let Some(browser_family) = browser_family {
        profile_entry
            .as_object_mut()
            .expect("profile entry must be object")
            .insert(
                "browserFamily".to_string(),
                Value::String(browser_family.to_string()),
            );
    }

    if set_default {
        root_obj.insert(
            "defaultRuntimeProfile".to_string(),
            Value::String(runtime_profile_name.to_string()),
        );
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create user config directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let serialized = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize user config: {}", e))?;
    fs::write(&path, format!("{}\n", serialized))
        .map_err(|e| format!("Failed to write user config {}: {}", path.display(), e))?;

    Ok(path)
}

/// Check if a boolean environment variable is set to a truthy value.
/// Returns false when unset, empty, or set to "0", "false", or "no" (case-insensitive).
fn env_var_is_truthy(name: &str) -> bool {
    match env::var(name) {
        Ok(val) => !matches!(val.to_lowercase().as_str(), "0" | "false" | "no" | ""),
        Err(_) => false,
    }
}

/// Parse an optional boolean value after a flag. Returns (value, consumed_next_arg).
/// Recognizes "true" as true, "false" as false. Bare flag defaults to true.
fn parse_bool_arg(args: &[String], i: usize) -> (bool, bool) {
    if let Some(v) = args.get(i + 1) {
        match v.as_str() {
            "true" => (true, true),
            "false" => (false, true),
            _ => (true, false),
        }
    } else {
        (true, false)
    }
}

/// Extract --config <path> from args before full flag parsing.
/// Returns `Some(Some(path))` if --config <path> found, `Some(None)` if --config
/// was the last arg with no value, `None` if --config not present.
///
/// Only flags that consume a following argument need to be listed here.
/// Boolean flags (--content-boundaries, --confirm-interactive, etc.) are
/// intentionally absent -- they don't take a value, so they can't cause
/// the next argument to be mis-consumed.
fn extract_config_path(args: &[String]) -> Option<Option<String>> {
    const FLAGS_WITH_VALUE: &[&str] = &[
        "--session",
        "--runtime-profile",
        "--headers",
        "--executable-path",
        "--cdp",
        "--extension",
        "--profile",
        "--state",
        "--proxy",
        "--proxy-bypass",
        "--args",
        "--user-agent",
        "-p",
        "--provider",
        "--device",
        "--session-name",
        "--color-scheme",
        "--download-path",
        "--max-output",
        "--allowed-domains",
        "--action-policy",
        "--confirm-actions",
        "--engine",
        "--screenshot-dir",
        "--screenshot-quality",
        "--screenshot-format",
        "--idle-timeout",
        "--service-reconcile-interval",
        "--service-job-timeout",
        "--service-recovery-retry-budget",
        "--service-recovery-base-backoff",
        "--service-recovery-max-backoff",
        "--model",
    ];
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--config" {
            return Some(args.get(i + 1).cloned());
        }
        if FLAGS_WITH_VALUE.contains(&args[i].as_str()) {
            i += 1;
        }
        i += 1;
    }
    None
}

pub fn load_config(args: &[String]) -> Result<Config, String> {
    let explicit = extract_config_path(args)
        .map(|p| ("--config", p))
        .or_else(|| {
            env::var("AGENT_BROWSER_CONFIG")
                .ok()
                .map(|p| ("AGENT_BROWSER_CONFIG", Some(p)))
        });

    if let Some((source, maybe_path)) = explicit {
        let path_str = maybe_path.ok_or_else(|| format!("{} requires a file path", source))?;
        let path = PathBuf::from(&path_str);
        if !path.exists() {
            return Err(format!("config file not found: {}", path_str));
        }
        return read_config_file(&path)
            .ok_or_else(|| format!("failed to load config from {}", path_str));
    }

    let user_config = dirs::home_dir()
        .map(|d| d.join(CONFIG_DIR).join(CONFIG_FILENAME))
        .and_then(|p| read_config_file(&p))
        .unwrap_or_default();

    let project_config = read_config_file(&PathBuf::from(PROJECT_CONFIG_FILENAME));

    let mut merged = match project_config {
        Some(project) => user_config.merge(project),
        None => user_config,
    };

    if let Some(runtime_profile_name) = selected_runtime_profile_from_sources(args, &merged) {
        apply_runtime_profile_overrides(&mut merged, &runtime_profile_name);
    }

    Ok(merged)
}

pub struct Flags {
    pub json: bool,
    pub headed: bool,
    pub leave_open: bool,
    pub debug: bool,
    pub session: String,
    pub default_runtime_profile: Option<String>,
    pub configured_runtime_profiles: HashMap<String, Option<String>>,
    pub configured_runtime_profile_browser_families: HashMap<String, String>,
    pub manual_login_preferred_services: Vec<String>,
    pub configured_service_state: ServiceState,
    pub service_state: ServiceState,
    pub service_reconcile_interval_ms: Option<u64>,
    pub service_job_timeout_ms: Option<u64>,
    pub service_monitor_interval_ms: Option<u64>,
    pub service_recovery_retry_budget: u64,
    pub service_recovery_base_backoff_ms: u64,
    pub service_recovery_max_backoff_ms: u64,
    pub service_recovery_retry_budget_source: BrowserRecoveryPolicyValueSource,
    pub service_recovery_base_backoff_ms_source: BrowserRecoveryPolicyValueSource,
    pub service_recovery_max_backoff_ms_source: BrowserRecoveryPolicyValueSource,
    pub browser_build_manifest_status: BTreeMap<String, Value>,
    pub runtime_profile: Option<String>,
    pub headers: Option<String>,
    pub executable_path: Option<String>,
    pub executable_path_source: Option<String>,
    pub cdp: Option<String>,
    pub extensions: Vec<String>,
    pub profile: Option<String>,
    pub state: Option<String>,
    pub proxy: Option<String>,
    pub proxy_bypass: Option<String>,
    pub args: Option<String>,
    pub user_agent: Option<String>,
    pub provider: Option<String>,
    pub ignore_https_errors: bool,
    pub allow_file_access: bool,
    pub device: Option<String>,
    pub auto_connect: bool,
    pub session_name: Option<String>,
    pub annotate: bool,
    pub color_scheme: Option<String>,
    pub download_path: Option<String>,
    pub content_boundaries: bool,
    pub max_output: Option<usize>,
    pub allowed_domains: Option<Vec<String>>,
    pub action_policy: Option<String>,
    pub confirm_actions: Option<String>,
    pub confirm_interactive: bool,
    pub engine: Option<String>,
    pub screenshot_dir: Option<String>,
    pub screenshot_quality: Option<u32>,
    pub screenshot_format: Option<String>,
    pub idle_timeout: Option<String>, // Canonical milliseconds string for AGENT_BROWSER_IDLE_TIMEOUT_MS
    pub default_timeout: Option<u64>, // AGENT_BROWSER_DEFAULT_TIMEOUT in ms
    pub no_auto_dialog: bool,
    pub model: Option<String>,
    pub default_viewport: Option<String>,
    pub verbose: bool,
    pub quiet: bool,

    // Track which launch-time options were explicitly passed via CLI
    // (as opposed to being set only via environment variables)
    pub cli_executable_path: bool,
    pub cli_extensions: bool,
    pub cli_profile: bool,
    pub cli_state: bool,
    pub cli_args: bool,
    pub cli_user_agent: bool,
    pub cli_proxy: bool,
    pub cli_proxy_bypass: bool,
    pub cli_allow_file_access: bool,
    pub cli_annotate: bool,
    pub cli_download_path: bool,
    pub cli_headed: bool,
    pub cli_leave_open: bool,
    pub cli_runtime_profile: bool,
}

/// Build the no-launch browser executable readiness view used by service status.
pub fn launch_config_status(flags: &Flags) -> Value {
    let default_browser_build = flags.service_state.default_browser_build;
    let stealth_default = default_browser_build == Some(BrowserBuild::StealthcdpChromium);
    let stealth_site_policy = flags
        .service_state
        .site_policies
        .values()
        .any(|policy| policy.browser_build == Some(BrowserBuild::StealthcdpChromium));
    let stealth_cdp_chromium_required = stealth_default || stealth_site_policy;
    let executable_path_exists = flags
        .executable_path
        .as_ref()
        .map(|path| Path::new(path).is_file());
    let stealth_manifest = flags
        .browser_build_manifest_status
        .get("stealthcdp_chromium");
    let stealth_manifest_ready = stealth_manifest
        .and_then(|status| status.get("ready"))
        .and_then(|value| value.as_bool());

    let mut warnings = Vec::new();
    if stealth_cdp_chromium_required && flags.executable_path.is_none() {
        warnings.push(json!({
            "code": "stealthcdp_executable_missing",
            "severity": "warning",
            "message": "stealthcdp_chromium is selected, but no executablePath, AGENT_BROWSER_EXECUTABLE_PATH, or browser build manifest executable is configured"
        }));
    } else if stealth_cdp_chromium_required && executable_path_exists == Some(false) {
        warnings.push(json!({
            "code": "stealthcdp_executable_not_found",
            "severity": "warning",
            "message": "stealthcdp_chromium is selected, but the configured executable path does not exist"
        }));
    }
    if stealth_cdp_chromium_required && stealth_manifest_ready == Some(false) {
        warnings.push(json!({
            "code": "stealthcdp_manifest_not_ready",
            "severity": "warning",
            "message": "stealthcdp_chromium manifest is configured but not ready"
        }));
    }

    let stealth_ready = !stealth_cdp_chromium_required
        || (executable_path_exists == Some(true) && stealth_manifest_ready.unwrap_or(true));

    json!({
        "defaultBrowserBuild": default_browser_build,
        "stealthCdpChromiumRequired": stealth_cdp_chromium_required,
        "stealthCdpChromiumReady": stealth_ready,
        "executablePath": flags.executable_path.clone(),
        "executablePathSource": flags.executable_path_source.clone(),
        "executablePathExists": executable_path_exists,
        "browserBuildManifests": flags.browser_build_manifest_status.clone(),
        "warnings": warnings
    })
}

pub fn parse_flags(args: &[String]) -> Flags {
    let config = load_config(args).unwrap_or_else(|e| {
        eprintln!("{} {}", color::warning_indicator(), e);
        std::process::exit(1);
    });

    let default_runtime_profile = config.default_runtime_profile.clone();
    let configured_runtime_profiles = config
        .runtime_profiles
        .as_ref()
        .map(|profiles| {
            profiles
                .iter()
                .map(|(name, profile)| (name.clone(), profile.user_data_dir.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let configured_runtime_profile_browser_families = config
        .runtime_profiles
        .as_ref()
        .map(|profiles| {
            profiles
                .iter()
                .filter_map(|(name, profile)| {
                    profile
                        .browser_family
                        .as_ref()
                        .map(|family| (name.clone(), family.clone()))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let manual_login_preferred_services = manual_login_preferred_services(&config);
    let configured_service_state = config.service_state_snapshot();
    let service_state = service_state_from_store(configured_service_state.clone());
    let service_reconcile_interval_ms = service_reconcile_interval_from_sources(&config);
    let service_job_timeout_ms = service_job_timeout_from_sources(&config);
    let service_monitor_interval_ms = service_monitor_interval_from_sources(&config);
    let recovery_defaults = BrowserRecoveryPolicyConfig::default();
    let (service_recovery_retry_budget, service_recovery_retry_budget_source) =
        service_recovery_value_from_sources(
            "AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET",
            config
                .service
                .as_ref()
                .and_then(|service| service.recovery_retry_budget),
            recovery_defaults.retry_budget,
        );
    let (service_recovery_base_backoff_ms, service_recovery_base_backoff_ms_source) =
        service_recovery_value_from_sources(
            "AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS",
            config
                .service
                .as_ref()
                .and_then(|service| service.recovery_base_backoff_ms),
            recovery_defaults.base_backoff_ms,
        );
    let (service_recovery_max_backoff_ms, service_recovery_max_backoff_ms_source) =
        service_recovery_value_from_sources(
            "AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS",
            config
                .service
                .as_ref()
                .and_then(|service| service.recovery_max_backoff_ms),
            recovery_defaults.max_backoff_ms,
        );
    let browser_build_manifest_configs = browser_build_manifest_configs_from_sources(&config);
    let browser_build_manifest_status =
        resolve_browser_build_manifests(&browser_build_manifest_configs);
    let env_executable_path = env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok();
    let config_executable_path = config.executable_path.clone();
    let manifest_executable_path =
        manifest_executable_path(&browser_build_manifest_status, "stealthcdp_chromium");
    let executable_path = env_executable_path
        .clone()
        .or(config_executable_path.clone())
        .or(manifest_executable_path.clone());
    let executable_path_source = if env_executable_path.is_some() {
        Some("env".to_string())
    } else if config_executable_path.is_some() {
        Some("config".to_string())
    } else if manifest_executable_path.is_some() {
        Some("manifest".to_string())
    } else {
        None
    };

    let extensions_env = env::var("AGENT_BROWSER_EXTENSIONS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let extensions = if !extensions_env.is_empty() {
        extensions_env
    } else {
        config.extensions.unwrap_or_default()
    };

    let mut flags = Flags {
        json: env_var_is_truthy("AGENT_BROWSER_JSON") || config.json.unwrap_or(false),
        headed: env_var_is_truthy("AGENT_BROWSER_HEADED") || config.headed.unwrap_or(false),
        leave_open: config.leave_open.unwrap_or(false),
        debug: env_var_is_truthy("AGENT_BROWSER_DEBUG") || config.debug.unwrap_or(false),
        session: env::var("AGENT_BROWSER_SESSION")
            .ok()
            .or(config.session)
            .unwrap_or_else(|| "default".to_string()),
        default_runtime_profile,
        configured_runtime_profiles,
        configured_runtime_profile_browser_families,
        manual_login_preferred_services,
        configured_service_state,
        service_state,
        service_reconcile_interval_ms,
        service_job_timeout_ms,
        service_monitor_interval_ms,
        service_recovery_retry_budget,
        service_recovery_base_backoff_ms,
        service_recovery_max_backoff_ms,
        service_recovery_retry_budget_source,
        service_recovery_base_backoff_ms_source,
        service_recovery_max_backoff_ms_source,
        browser_build_manifest_status,
        runtime_profile: env::var("AGENT_BROWSER_RUNTIME_PROFILE")
            .ok()
            .or(config.runtime_profile),
        headers: config.headers,
        executable_path,
        executable_path_source,
        cdp: config.cdp,
        extensions,
        profile: env::var("AGENT_BROWSER_PROFILE").ok().or(config.profile),
        state: env::var("AGENT_BROWSER_STATE").ok().or(config.state),
        proxy: env::var("AGENT_BROWSER_PROXY")
            .ok()
            .or(config.proxy)
            .or_else(|| env::var("HTTP_PROXY").ok())
            .or_else(|| env::var("http_proxy").ok())
            .or_else(|| env::var("HTTPS_PROXY").ok())
            .or_else(|| env::var("https_proxy").ok())
            .or_else(|| env::var("ALL_PROXY").ok())
            .or_else(|| env::var("all_proxy").ok()),
        proxy_bypass: env::var("AGENT_BROWSER_PROXY_BYPASS")
            .ok()
            .or(config.proxy_bypass)
            .or_else(|| env::var("NO_PROXY").ok())
            .or_else(|| env::var("no_proxy").ok()),
        args: env::var("AGENT_BROWSER_ARGS").ok().or(config.args),
        user_agent: env::var("AGENT_BROWSER_USER_AGENT")
            .ok()
            .or(config.user_agent),
        provider: env::var("AGENT_BROWSER_PROVIDER").ok().or(config.provider),
        ignore_https_errors: env_var_is_truthy("AGENT_BROWSER_IGNORE_HTTPS_ERRORS")
            || config.ignore_https_errors.unwrap_or(false),
        allow_file_access: env_var_is_truthy("AGENT_BROWSER_ALLOW_FILE_ACCESS")
            || config.allow_file_access.unwrap_or(false),
        device: env::var("AGENT_BROWSER_IOS_DEVICE").ok().or(config.device),
        auto_connect: env_var_is_truthy("AGENT_BROWSER_AUTO_CONNECT")
            || config.auto_connect.unwrap_or(false),
        session_name: env::var("AGENT_BROWSER_SESSION_NAME")
            .ok()
            .or(config.session_name),
        annotate: env_var_is_truthy("AGENT_BROWSER_ANNOTATE") || config.annotate.unwrap_or(false),
        color_scheme: env::var("AGENT_BROWSER_COLOR_SCHEME")
            .ok()
            .or(config.color_scheme),
        download_path: env::var("AGENT_BROWSER_DOWNLOAD_PATH")
            .ok()
            .or(config.download_path),
        content_boundaries: env_var_is_truthy("AGENT_BROWSER_CONTENT_BOUNDARIES")
            || config.content_boundaries.unwrap_or(false),
        max_output: env::var("AGENT_BROWSER_MAX_OUTPUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .or(config.max_output),
        allowed_domains: env::var("AGENT_BROWSER_ALLOWED_DOMAINS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|d| d.trim().to_lowercase())
                    .filter(|d| !d.is_empty())
                    .collect()
            })
            .or(config.allowed_domains),
        action_policy: env::var("AGENT_BROWSER_ACTION_POLICY")
            .ok()
            .or(config.action_policy),
        confirm_actions: env::var("AGENT_BROWSER_CONFIRM_ACTIONS")
            .ok()
            .or(config.confirm_actions),
        confirm_interactive: env_var_is_truthy("AGENT_BROWSER_CONFIRM_INTERACTIVE")
            || config.confirm_interactive.unwrap_or(false),
        engine: env::var("AGENT_BROWSER_ENGINE").ok().or(config.engine),
        screenshot_dir: env::var("AGENT_BROWSER_SCREENSHOT_DIR")
            .ok()
            .or(config.screenshot_dir),
        screenshot_quality: env::var("AGENT_BROWSER_SCREENSHOT_QUALITY")
            .ok()
            .and_then(|s| s.parse().ok())
            .or(config.screenshot_quality),
        screenshot_format: env::var("AGENT_BROWSER_SCREENSHOT_FORMAT")
            .ok()
            .or(config.screenshot_format)
            .filter(|s| s == "png" || s == "jpeg"),
        idle_timeout: parse_idle_timeout_value(
            env::var("AGENT_BROWSER_IDLE_TIMEOUT_MS").ok(),
            "AGENT_BROWSER_IDLE_TIMEOUT_MS",
        )
        .or(config.idle_timeout),
        default_timeout: env::var("AGENT_BROWSER_DEFAULT_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok()),
        no_auto_dialog: env_var_is_truthy("AGENT_BROWSER_NO_AUTO_DIALOG")
            || config.no_auto_dialog.unwrap_or(false),
        model: env::var("AI_GATEWAY_MODEL").ok().or(config.model),
        default_viewport: config.default_viewport,
        verbose: false,
        quiet: false,
        cli_executable_path: false,
        cli_extensions: false,
        cli_profile: false,
        cli_state: false,
        cli_args: false,
        cli_user_agent: false,
        cli_proxy: false,
        cli_proxy_bypass: false,
        cli_allow_file_access: false,
        cli_annotate: false,
        cli_download_path: false,
        cli_headed: false,
        cli_leave_open: false,
        cli_runtime_profile: false,
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.json = val;
                if consumed {
                    i += 1;
                }
            }
            "--headed" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.headed = val;
                flags.cli_headed = true;
                if consumed {
                    i += 1;
                }
            }
            "--leave-open" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.leave_open = val;
                flags.cli_leave_open = true;
                if consumed {
                    i += 1;
                }
            }
            "--debug" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.debug = val;
                if consumed {
                    i += 1;
                }
            }
            "--session" => {
                if let Some(s) = args.get(i + 1) {
                    flags.session = s.clone();
                    i += 1;
                }
            }
            "--runtime-profile" => {
                if let Some(s) = args.get(i + 1) {
                    flags.runtime_profile = Some(s.clone());
                    flags.cli_runtime_profile = true;
                    i += 1;
                }
            }
            "--idle-timeout" => {
                if let Some(s) = args.get(i + 1) {
                    match parse_idle_timeout(s) {
                        Ok(ms) => flags.idle_timeout = Some(ms),
                        Err(e) => eprintln!(
                            "{} Invalid --idle-timeout: {}",
                            color::warning_indicator(),
                            e
                        ),
                    }
                    i += 1;
                }
            }
            "--service-reconcile-interval" => {
                if let Some(s) = args.get(i + 1) {
                    match s.parse::<u64>() {
                        Ok(0) => flags.service_reconcile_interval_ms = None,
                        Ok(ms) => flags.service_reconcile_interval_ms = Some(ms),
                        Err(_) => eprintln!(
                            "{} Invalid --service-reconcile-interval: expected milliseconds, got {}",
                            color::warning_indicator(),
                            s
                        ),
                    }
                    i += 1;
                }
            }
            "--service-job-timeout" => {
                if let Some(s) = args.get(i + 1) {
                    match s.parse::<u64>() {
                        Ok(0) => flags.service_job_timeout_ms = None,
                        Ok(ms) => flags.service_job_timeout_ms = Some(ms),
                        Err(_) => eprintln!(
                            "{} Invalid --service-job-timeout: expected milliseconds, got {}",
                            color::warning_indicator(),
                            s
                        ),
                    }
                    i += 1;
                }
            }
            "--service-monitor-interval" => {
                if let Some(s) = args.get(i + 1) {
                    match s.parse::<u64>() {
                        Ok(0) => flags.service_monitor_interval_ms = None,
                        Ok(ms) => flags.service_monitor_interval_ms = Some(ms),
                        Err(_) => eprintln!(
                            "{} Invalid --service-monitor-interval: expected milliseconds, got {}",
                            color::warning_indicator(),
                            s
                        ),
                    }
                    i += 1;
                }
            }
            "--service-recovery-retry-budget" => {
                if let Some(s) = args.get(i + 1) {
                    match s.parse::<u64>() {
                        Ok(value) => {
                            flags.service_recovery_retry_budget = value;
                            flags.service_recovery_retry_budget_source =
                                BrowserRecoveryPolicyValueSource::Cli;
                        }
                        Err(_) => eprintln!(
                            "{} Invalid --service-recovery-retry-budget: expected unsigned integer, got {}",
                            color::warning_indicator(),
                            s
                        ),
                    }
                    i += 1;
                }
            }
            "--service-recovery-base-backoff" => {
                if let Some(s) = args.get(i + 1) {
                    match s.parse::<u64>() {
                        Ok(ms) => {
                            flags.service_recovery_base_backoff_ms = ms;
                            flags.service_recovery_base_backoff_ms_source =
                                BrowserRecoveryPolicyValueSource::Cli;
                        }
                        Err(_) => eprintln!(
                            "{} Invalid --service-recovery-base-backoff: expected milliseconds, got {}",
                            color::warning_indicator(),
                            s
                        ),
                    }
                    i += 1;
                }
            }
            "--service-recovery-max-backoff" => {
                if let Some(s) = args.get(i + 1) {
                    match s.parse::<u64>() {
                        Ok(ms) => {
                            flags.service_recovery_max_backoff_ms = ms;
                            flags.service_recovery_max_backoff_ms_source =
                                BrowserRecoveryPolicyValueSource::Cli;
                        }
                        Err(_) => eprintln!(
                            "{} Invalid --service-recovery-max-backoff: expected milliseconds, got {}",
                            color::warning_indicator(),
                            s
                        ),
                    }
                    i += 1;
                }
            }
            "--headers" => {
                if let Some(h) = args.get(i + 1) {
                    flags.headers = Some(h.clone());
                    i += 1;
                }
            }
            "--executable-path" => {
                if let Some(s) = args.get(i + 1) {
                    flags.executable_path = Some(s.clone());
                    flags.cli_executable_path = true;
                    i += 1;
                }
            }
            "--extension" => {
                if let Some(s) = args.get(i + 1) {
                    flags.extensions.push(s.clone());
                    flags.cli_extensions = true;
                    i += 1;
                }
            }
            "--cdp" => {
                if let Some(s) = args.get(i + 1) {
                    flags.cdp = Some(s.clone());
                    i += 1;
                }
            }
            "--profile" => {
                if let Some(s) = args.get(i + 1) {
                    flags.profile = Some(s.clone());
                    flags.cli_profile = true;
                    i += 1;
                }
            }
            "--state" => {
                if let Some(s) = args.get(i + 1) {
                    flags.state = Some(s.clone());
                    flags.cli_state = true;
                    i += 1;
                }
            }
            "--proxy" => {
                if let Some(p) = args.get(i + 1) {
                    flags.proxy = Some(p.clone());
                    flags.cli_proxy = true;
                    i += 1;
                }
            }
            "--proxy-bypass" => {
                if let Some(s) = args.get(i + 1) {
                    flags.proxy_bypass = Some(s.clone());
                    flags.cli_proxy_bypass = true;
                    i += 1;
                }
            }
            "--args" => {
                if let Some(s) = args.get(i + 1) {
                    flags.args = Some(s.clone());
                    flags.cli_args = true;
                    i += 1;
                }
            }
            "--user-agent" => {
                if let Some(s) = args.get(i + 1) {
                    flags.user_agent = Some(s.clone());
                    flags.cli_user_agent = true;
                    i += 1;
                }
            }
            "-p" | "--provider" => {
                if let Some(p) = args.get(i + 1) {
                    flags.provider = Some(p.clone());
                    i += 1;
                }
            }
            "--ignore-https-errors" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.ignore_https_errors = val;
                if consumed {
                    i += 1;
                }
            }
            "--allow-file-access" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.allow_file_access = val;
                flags.cli_allow_file_access = true;
                if consumed {
                    i += 1;
                }
            }
            "--device" => {
                if let Some(d) = args.get(i + 1) {
                    flags.device = Some(d.clone());
                    i += 1;
                }
            }
            "--auto-connect" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.auto_connect = val;
                if consumed {
                    i += 1;
                }
            }
            "--session-name" => {
                if let Some(s) = args.get(i + 1) {
                    flags.session_name = Some(s.clone());
                    i += 1;
                }
            }
            "--annotate" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.annotate = val;
                flags.cli_annotate = true;
                if consumed {
                    i += 1;
                }
            }
            "--color-scheme" => {
                if let Some(s) = args.get(i + 1) {
                    flags.color_scheme = Some(s.clone());
                    i += 1;
                }
            }
            "--download-path" => {
                if let Some(s) = args.get(i + 1) {
                    flags.download_path = Some(s.clone());
                    flags.cli_download_path = true;
                    i += 1;
                }
            }
            "--content-boundaries" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.content_boundaries = val;
                if consumed {
                    i += 1;
                }
            }
            "--max-output" => {
                if let Some(s) = args.get(i + 1) {
                    if let Ok(n) = s.parse::<usize>() {
                        flags.max_output = Some(n);
                    }
                    i += 1;
                }
            }
            "--allowed-domains" => {
                if let Some(s) = args.get(i + 1) {
                    flags.allowed_domains = Some(
                        s.split(',')
                            .map(|d| d.trim().to_lowercase())
                            .filter(|d| !d.is_empty())
                            .collect(),
                    );
                    i += 1;
                }
            }
            "--action-policy" => {
                if let Some(s) = args.get(i + 1) {
                    flags.action_policy = Some(s.clone());
                    i += 1;
                }
            }
            "--confirm-actions" => {
                if let Some(s) = args.get(i + 1) {
                    flags.confirm_actions = Some(s.clone());
                    i += 1;
                }
            }
            "--confirm-interactive" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.confirm_interactive = val;
                if consumed {
                    i += 1;
                }
            }
            "--engine" => {
                if let Some(s) = args.get(i + 1) {
                    flags.engine = Some(s.clone());
                    i += 1;
                }
            }
            "--screenshot-dir" => {
                if let Some(s) = args.get(i + 1) {
                    flags.screenshot_dir = Some(s.clone());
                    i += 1;
                }
            }
            "--screenshot-quality" => {
                if let Some(s) = args.get(i + 1) {
                    if let Ok(n) = s.parse::<u32>() {
                        if n <= 100 {
                            flags.screenshot_quality = Some(n);
                        } else {
                            eprintln!(
                                "{} --screenshot-quality must be 0-100, got {}",
                                color::warning_indicator(),
                                n
                            );
                        }
                    }
                    i += 1;
                }
            }
            "--screenshot-format" => {
                if let Some(s) = args.get(i + 1) {
                    if s == "png" || s == "jpeg" {
                        flags.screenshot_format = Some(s.clone());
                    } else {
                        eprintln!(
                            "{} --screenshot-format must be png or jpeg, got '{}'",
                            color::warning_indicator(),
                            s
                        );
                    }
                    i += 1;
                }
            }
            "--no-auto-dialog" => {
                let (val, consumed) = parse_bool_arg(args, i);
                flags.no_auto_dialog = val;
                if consumed {
                    i += 1;
                }
            }
            "--model" => {
                if let Some(s) = args.get(i + 1) {
                    flags.model = Some(s.clone());
                    i += 1;
                }
            }
            "-v" | "--verbose" => {
                flags.verbose = true;
            }
            "-q" | "--quiet" => {
                flags.quiet = true;
            }
            "--config" => {
                // Already handled by load_config(); skip the value
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    flags
}

pub fn clean_args(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut skip_next = false;

    // Boolean flags that optionally take true/false
    const GLOBAL_BOOL_FLAGS: &[&str] = &[
        "--json",
        "--headed",
        "--leave-open",
        "--debug",
        "--ignore-https-errors",
        "--allow-file-access",
        "--auto-connect",
        "--annotate",
        "--content-boundaries",
        "--confirm-interactive",
        "--no-auto-dialog",
        "-v",
        "--verbose",
        "-q",
        "--quiet",
    ];
    // Global flags that always take a value (need to skip the next arg too)
    const GLOBAL_FLAGS_WITH_VALUE: &[&str] = &[
        "--session",
        "--runtime-profile",
        "--headers",
        "--executable-path",
        "--cdp",
        "--extension",
        "--profile",
        "--state",
        "--proxy",
        "--proxy-bypass",
        "--args",
        "--user-agent",
        "-p",
        "--provider",
        "--device",
        "--session-name",
        "--color-scheme",
        "--download-path",
        "--max-output",
        "--allowed-domains",
        "--action-policy",
        "--confirm-actions",
        "--config",
        "--engine",
        "--screenshot-dir",
        "--screenshot-quality",
        "--screenshot-format",
        "--idle-timeout",
        "--service-reconcile-interval",
        "--service-job-timeout",
        "--service-recovery-retry-budget",
        "--service-recovery-base-backoff",
        "--service-recovery-max-backoff",
        "--model",
    ];

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if skip_next {
            skip_next = false;
            i += 1;
            continue;
        }
        if GLOBAL_FLAGS_WITH_VALUE.contains(&arg.as_str()) {
            skip_next = true;
            i += 1;
            continue;
        }
        if GLOBAL_BOOL_FLAGS.contains(&arg.as_str()) {
            if let Some(v) = args.get(i + 1) {
                if matches!(v.as_str(), "true" | "false") {
                    i += 1;
                }
            }
            i += 1;
            continue;
        }
        result.push(arg.clone());
        i += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};
    use crate::test_utils::EnvGuard;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn test_parse_headers_flag() {
        let flags = parse_flags(&args(r#"open example.com --headers {"Auth":"token"}"#));
        assert_eq!(flags.headers, Some(r#"{"Auth":"token"}"#.to_string()));
    }

    #[test]
    fn test_parse_idle_timeout_raw_ms() {
        assert_eq!(parse_idle_timeout("10").unwrap(), "10");
    }

    #[test]
    fn test_parse_idle_timeout_seconds() {
        assert_eq!(parse_idle_timeout("10s").unwrap(), "10000");
    }

    #[test]
    fn test_parse_idle_timeout_minutes() {
        assert_eq!(parse_idle_timeout("3m").unwrap(), "180000");
    }

    #[test]
    fn test_parse_idle_timeout_hours() {
        assert_eq!(parse_idle_timeout("1h").unwrap(), "3600000");
    }

    #[test]
    fn test_parse_idle_timeout_rejects_capital_m() {
        assert!(parse_idle_timeout("10M").is_err());
    }

    #[test]
    fn test_parse_idle_timeout_rejects_unknown_unit() {
        assert!(parse_idle_timeout("10x").is_err());
    }

    #[test]
    fn test_parse_headers_flag_with_spaces() {
        // Headers JSON is passed as a single quoted argument in shell
        let input: Vec<String> = vec![
            "open".to_string(),
            "example.com".to_string(),
            "--headers".to_string(),
            r#"{"Authorization": "Bearer token"}"#.to_string(),
        ];
        let flags = parse_flags(&input);
        assert_eq!(
            flags.headers,
            Some(r#"{"Authorization": "Bearer token"}"#.to_string())
        );
    }

    #[test]
    fn test_parse_no_headers_flag() {
        let flags = parse_flags(&args("open example.com"));
        assert!(flags.headers.is_none());
    }

    #[test]
    fn test_clean_args_removes_headers() {
        let input: Vec<String> = vec![
            "open".to_string(),
            "example.com".to_string(),
            "--headers".to_string(),
            r#"{"Auth":"token"}"#.to_string(),
        ];
        let clean = clean_args(&input);
        assert_eq!(clean, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_headers_at_start() {
        let input: Vec<String> = vec![
            "--headers".to_string(),
            r#"{"Auth":"token"}"#.to_string(),
            "open".to_string(),
            "example.com".to_string(),
        ];
        let clean = clean_args(&input);
        assert_eq!(clean, vec!["open", "example.com"]);
    }

    #[test]
    fn test_headers_with_other_flags() {
        let input: Vec<String> = vec![
            "open".to_string(),
            "example.com".to_string(),
            "--headers".to_string(),
            r#"{"Auth":"token"}"#.to_string(),
            "--json".to_string(),
            "--headed".to_string(),
        ];
        let flags = parse_flags(&input);
        assert_eq!(flags.headers, Some(r#"{"Auth":"token"}"#.to_string()));
        assert!(flags.json);
        assert!(flags.headed);

        let clean = clean_args(&input);
        assert_eq!(clean, vec!["open", "example.com"]);
    }

    #[test]
    fn test_parse_executable_path_flag() {
        let flags = parse_flags(&args(
            "--executable-path /path/to/chromium open example.com",
        ));
        assert_eq!(flags.executable_path, Some("/path/to/chromium".to_string()));
    }

    #[test]
    fn test_parse_executable_path_flag_no_value() {
        let config_path = std::env::temp_dir().join(format!(
            "agent-browser-empty-config-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&config_path, "{}").unwrap();
        let flags = parse_flags(&args(&format!(
            "--config {} --executable-path",
            config_path.display()
        )));
        let _ = std::fs::remove_file(config_path);
        assert_eq!(flags.executable_path, None);
    }

    #[test]
    fn test_manifest_resolves_stealthcdp_executable_path() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-build-manifest-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let home = dir.join("home");
        let artifact = dir.join("artifact");
        let chrome_dir = artifact.join("chrome-linux");
        fs::create_dir_all(&chrome_dir).unwrap();
        fs::create_dir_all(&home).unwrap();
        let chrome_path = chrome_dir.join("chrome");
        fs::write(&chrome_path, "chrome").unwrap();
        let smoke_path = artifact.join("smoke.json");
        fs::write(
            &smoke_path,
            r#"{"success":true,"checks":{"navigatorWebdriver":"false"}}"#,
        )
        .unwrap();
        let manifest_path = artifact.join("manifest.json");
        fs::write(
            &manifest_path,
            r#"{
              "schema": "chromium-stealthcdp.artifact.v1",
              "artifactName": "150.0.7835.0+stealthcdp.test",
              "chromeVersion": "Chromium 150.0.7835.0",
              "chromium": { "sourceSha": "chromium-sha" },
              "patchset": { "repoSha": "patchset-sha" },
              "executable": { "relativePath": "chrome-linux/chrome", "sha256": "unused" },
              "smoke": { "relativePath": "smoke.json", "sha256": "unused" }
            }"#,
        )
        .unwrap();
        let config_path = dir.join("agent-browser.json");
        fs::write(
            &config_path,
            format!(
                r#"{{
                  "service": {{
                    "defaultBrowserBuild": "stealthcdp_chromium",
                    "browserBuildManifests": {{
                      "stealthcdp_chromium": {{ "manifestPath": "{}" }}
                    }}
                  }}
                }}"#,
                manifest_path.display()
            ),
        )
        .unwrap();

        let guard = EnvGuard::new(&[
            "AGENT_BROWSER_EXECUTABLE_PATH",
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH",
            "HOME",
        ]);
        guard.remove("AGENT_BROWSER_EXECUTABLE_PATH");
        guard.remove("AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH");
        guard.set("HOME", home.to_str().unwrap());

        let flags = parse_flags(&args(&format!(
            "--config {} service status",
            config_path.display()
        )));
        let launch_config = launch_config_status(&flags);

        assert_eq!(flags.executable_path.as_deref(), chrome_path.to_str());
        assert_eq!(flags.executable_path_source.as_deref(), Some("manifest"));
        assert_eq!(launch_config["stealthCdpChromiumReady"], true);
        assert_eq!(launch_config["executablePathSource"], "manifest");
        assert_eq!(
            launch_config["browserBuildManifests"]["stealthcdp_chromium"]["patchsetSha"],
            "patchset-sha"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_clean_args_removes_executable_path() {
        let cleaned = clean_args(&args(
            "--executable-path /path/to/chromium open example.com",
        ));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_executable_path_with_other_flags() {
        let cleaned = clean_args(&args(
            "--json --executable-path /path/to/chromium --headed open example.com",
        ));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_idle_timeout_before_command() {
        let cleaned = clean_args(&args("--idle-timeout 10s open example.com"));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_service_reconcile_interval_before_command() {
        let cleaned = clean_args(&args("--service-reconcile-interval 1000 open example.com"));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_service_job_timeout_before_command() {
        let cleaned = clean_args(&args("--service-job-timeout 1000 open example.com"));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_service_recovery_flags_before_command() {
        let cleaned = clean_args(&args(
            "--service-recovery-retry-budget 5 --service-recovery-base-backoff 250 --service-recovery-max-backoff 10000 open example.com",
        ));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_parse_idle_timeout_flag_converts_to_ms() {
        let flags = parse_flags(&args("--idle-timeout 10s open example.com"));
        assert_eq!(flags.idle_timeout.as_deref(), Some("10000"));
    }

    #[test]
    fn test_parse_flags_with_session_and_executable_path() {
        let flags = parse_flags(&args(
            "--session test --executable-path /custom/chrome open example.com",
        ));
        assert_eq!(flags.session, "test");
        assert_eq!(flags.executable_path, Some("/custom/chrome".to_string()));
    }

    #[test]
    fn test_cli_executable_path_tracking() {
        // When --executable-path is passed via CLI, cli_executable_path should be true
        let flags = parse_flags(&args("--executable-path /path/to/chrome snapshot"));
        assert!(flags.cli_executable_path);
        assert_eq!(flags.executable_path, Some("/path/to/chrome".to_string()));
    }

    #[test]
    fn test_cli_executable_path_not_set_without_flag() {
        // When no --executable-path is passed, cli_executable_path should be false
        // (even if env var sets executable_path to Some value, which we can't test here)
        let flags = parse_flags(&args("snapshot"));
        assert!(!flags.cli_executable_path);
    }

    #[test]
    fn test_cli_extension_tracking() {
        let flags = parse_flags(&args("--extension /path/to/ext snapshot"));
        assert!(flags.cli_extensions);
    }

    #[test]
    fn test_cli_profile_tracking() {
        let flags = parse_flags(&args("--profile /path/to/profile snapshot"));
        assert!(flags.cli_profile);
    }

    #[test]
    fn test_cli_annotate_tracking() {
        let flags = parse_flags(&args("--annotate screenshot"));
        assert!(flags.cli_annotate);
        assert!(flags.annotate);
    }

    #[test]
    fn test_cli_annotate_not_set_without_flag() {
        let flags = parse_flags(&args("screenshot"));
        assert!(!flags.cli_annotate);
    }

    #[test]
    fn test_cli_download_path_tracking() {
        let flags = parse_flags(&args("--download-path /tmp/dl snapshot"));
        assert!(flags.cli_download_path);
        assert_eq!(flags.download_path, Some("/tmp/dl".to_string()));
    }

    #[test]
    fn test_cli_download_path_not_set_without_flag() {
        let flags = parse_flags(&args("snapshot"));
        assert!(!flags.cli_download_path);
    }

    #[test]
    fn test_cli_multiple_flags_tracking() {
        let flags = parse_flags(&args(
            "--executable-path /chrome --profile /profile --proxy http://proxy snapshot",
        ));
        assert!(flags.cli_executable_path);
        assert!(flags.cli_profile);
        assert!(flags.cli_proxy);
        assert!(!flags.cli_extensions);
        assert!(!flags.cli_state);
    }

    // === Config file tests ===

    #[test]
    fn test_config_deserialize_full() {
        let json = r#"{
            "defaultRuntimeProfile": "work",
            "runtimeProfiles": {
                "work": {
                    "userDataDir": "/tmp/work-profile",
                    "launch": {
                        "headed": true,
                        "proxy": "http://runtime-proxy:8080"
                    },
                    "preferences": {
                        "defaultViewport": "960x640"
                    },
                    "auth": {
                        "sessionName": "runtime-session"
                    }
                }
            },
            "service": {
                "sitePolicies": {
                    "google": {
                        "originPattern": "https://accounts.google.com",
                        "browserHost": "docker_headed",
                        "viewStream": "virtual_display_webrtc",
                        "controlInput": "webrtc_input",
                        "interactionMode": "human_like_input",
                        "manualLoginPreferred": true,
                        "profileRequired": true,
                        "authProviders": ["browser", "gog"],
                        "challengePolicy": "avoid_first",
                        "allowedChallengeProviders": ["manual"]
                    }
                },
                "providers": {
                    "manual": {
                        "kind": "manual_approval",
                        "displayName": "Dashboard approval",
                        "enabled": true,
                        "capabilities": ["human_approval"]
                    }
                }
            },
            "headed": true,
            "json": true,
            "debug": true,
            "session": "test-session",
            "sessionName": "my-app",
            "executablePath": "/usr/bin/chromium",
            "extensions": ["/ext1", "/ext2"],
            "profile": "/tmp/profile",
            "state": "/tmp/state.json",
            "proxy": "http://proxy:8080",
            "proxyBypass": "localhost",
            "args": "--no-sandbox",
            "userAgent": "test-agent",
            "provider": "ios",
            "device": "iPhone 15",
            "ignoreHttpsErrors": true,
            "allowFileAccess": true,
            "cdp": "9222",
            "autoConnect": true,
            "headers": "{\"Auth\":\"token\"}"
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.default_runtime_profile.as_deref(), Some("work"));
        assert!(config
            .runtime_profiles
            .as_ref()
            .is_some_and(|m| m.contains_key("work")));
        assert_eq!(
            config
                .runtime_profiles
                .as_ref()
                .and_then(|m| m.get("work"))
                .and_then(|p| p.preferences.as_ref())
                .and_then(|p| p.default_viewport.as_deref()),
            Some("960x640")
        );
        let service = config.service.as_ref().unwrap();
        let google_policy = service
            .site_policies
            .as_ref()
            .and_then(|policies| policies.get("google"))
            .unwrap();
        assert_eq!(
            google_policy.browser_host,
            Some(crate::native::service_model::BrowserHost::DockerHeaded)
        );
        assert_eq!(
            google_policy.interaction_mode,
            crate::native::service_model::InteractionMode::HumanLikeInput
        );
        let manual_provider = service
            .providers
            .as_ref()
            .and_then(|providers| providers.get("manual"))
            .unwrap();
        assert_eq!(
            manual_provider.kind,
            crate::native::service_model::ProviderKind::ManualApproval
        );
        assert_eq!(config.headed, Some(true));
        assert_eq!(config.json, Some(true));
        assert_eq!(config.debug, Some(true));
        assert_eq!(config.session.as_deref(), Some("test-session"));
        assert_eq!(config.session_name.as_deref(), Some("my-app"));
        assert_eq!(config.executable_path.as_deref(), Some("/usr/bin/chromium"));
        assert_eq!(
            config.extensions,
            Some(vec!["/ext1".to_string(), "/ext2".to_string()])
        );
        assert_eq!(config.profile.as_deref(), Some("/tmp/profile"));
        assert_eq!(config.state.as_deref(), Some("/tmp/state.json"));
        assert_eq!(config.proxy.as_deref(), Some("http://proxy:8080"));
        assert_eq!(config.proxy_bypass.as_deref(), Some("localhost"));
        assert_eq!(config.args.as_deref(), Some("--no-sandbox"));
        assert_eq!(config.user_agent.as_deref(), Some("test-agent"));
        assert_eq!(config.provider.as_deref(), Some("ios"));
        assert_eq!(config.device.as_deref(), Some("iPhone 15"));
        assert_eq!(config.ignore_https_errors, Some(true));
        assert_eq!(config.allow_file_access, Some(true));
        assert_eq!(config.cdp.as_deref(), Some("9222"));
        assert_eq!(config.auto_connect, Some(true));
        assert_eq!(config.headers.as_deref(), Some("{\"Auth\":\"token\"}"));
    }

    #[test]
    fn test_config_deserialize_partial() {
        let json = r#"{"headed": true, "proxy": "http://localhost:8080"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.headed, Some(true));
        assert_eq!(config.proxy.as_deref(), Some("http://localhost:8080"));
        assert_eq!(config.session, None);
        assert_eq!(config.extensions, None);
        assert_eq!(config.debug, None);
    }

    #[test]
    fn test_config_deserialize_empty() {
        let config: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(config.headed, None);
        assert_eq!(config.session, None);
        assert_eq!(config.proxy, None);
    }

    #[test]
    fn test_config_ignores_unknown_keys() {
        let json = r#"{"headed": true, "unknownFutureKey": "value", "anotherOne": 42}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.headed, Some(true));
    }

    #[test]
    fn test_config_merge_project_overrides_user() {
        let user = Config {
            runtime_profiles: Some(HashMap::from([(
                "default".to_string(),
                RuntimeProfileConfig {
                    user_data_dir: Some("/user/runtime".to_string()),
                    ..RuntimeProfileConfig::default()
                },
            )])),
            headed: Some(true),
            proxy: Some("http://user-proxy:8080".to_string()),
            profile: Some("/user/profile".to_string()),
            service: Some(ServiceConfig {
                profiles: None,
                sessions: None,
                monitors: Some(BTreeMap::from([(
                    "google-login-freshness".to_string(),
                    SiteMonitor {
                        id: "google-login-freshness".to_string(),
                        name: "Google login freshness".to_string(),
                        ..SiteMonitor::default()
                    },
                )])),
                site_policies: Some(BTreeMap::from([(
                    "google".to_string(),
                    SitePolicy {
                        id: "google".to_string(),
                        origin_pattern: "https://accounts.google.com".to_string(),
                        profile_required: true,
                        ..SitePolicy::default()
                    },
                )])),
                providers: Some(BTreeMap::from([(
                    "manual".to_string(),
                    ServiceProvider {
                        id: "manual".to_string(),
                        display_name: "Manual".to_string(),
                        ..ServiceProvider::default()
                    },
                )])),
                reconcile_interval_ms: Some(60_000),
                job_timeout_ms: Some(120_000),
                monitor_interval_ms: Some(60_000),
                recovery_retry_budget: Some(3),
                recovery_base_backoff_ms: Some(1_000),
                recovery_max_backoff_ms: Some(30_000),
                default_browser_build: None,
                browser_build_manifests: None,
            }),
            ..Config::default()
        };
        let project = Config {
            runtime_profiles: Some(HashMap::from([(
                "default".to_string(),
                RuntimeProfileConfig {
                    launch: Some(RuntimeProfileLaunchConfig {
                        headed: Some(false),
                        proxy: Some("http://runtime-proxy:9090".to_string()),
                        ..RuntimeProfileLaunchConfig::default()
                    }),
                    ..RuntimeProfileConfig::default()
                },
            )])),
            proxy: Some("http://project-proxy:9090".to_string()),
            debug: Some(true),
            service: Some(ServiceConfig {
                profiles: None,
                sessions: None,
                monitors: Some(BTreeMap::from([(
                    "github-heartbeat".to_string(),
                    SiteMonitor {
                        id: "github-heartbeat".to_string(),
                        name: "GitHub heartbeat".to_string(),
                        ..SiteMonitor::default()
                    },
                )])),
                site_policies: Some(BTreeMap::from([
                    (
                        "google".to_string(),
                        SitePolicy {
                            id: "google".to_string(),
                            origin_pattern: "https://accounts.google.com".to_string(),
                            manual_login_preferred: true,
                            ..SitePolicy::default()
                        },
                    ),
                    (
                        "github".to_string(),
                        SitePolicy {
                            id: "github".to_string(),
                            origin_pattern: "https://github.com".to_string(),
                            ..SitePolicy::default()
                        },
                    ),
                ])),
                providers: None,
                reconcile_interval_ms: Some(30_000),
                job_timeout_ms: Some(90_000),
                monitor_interval_ms: Some(45_000),
                recovery_retry_budget: Some(5),
                recovery_base_backoff_ms: Some(500),
                recovery_max_backoff_ms: Some(10_000),
                default_browser_build: Some(BrowserBuild::StealthcdpChromium),
                browser_build_manifests: Some(BTreeMap::from([(
                    "stealthcdp_chromium".to_string(),
                    BrowserBuildManifestConfig {
                        manifest_path: Some("/project/manifest.json".to_string()),
                    },
                )])),
            }),
            ..Config::default()
        };
        let merged = user.merge(project);
        assert_eq!(merged.headed, Some(true)); // kept from user
        assert_eq!(merged.proxy.as_deref(), Some("http://project-proxy:9090")); // overridden by project
        assert_eq!(merged.profile.as_deref(), Some("/user/profile")); // kept from user
        assert_eq!(merged.debug, Some(true)); // added by project
        let runtime = merged
            .runtime_profiles
            .as_ref()
            .and_then(|m| m.get("default"))
            .unwrap();
        assert_eq!(runtime.user_data_dir.as_deref(), Some("/user/runtime"));
        assert_eq!(
            runtime.launch.as_ref().and_then(|l| l.proxy.as_deref()),
            Some("http://runtime-proxy:9090")
        );
        let service = merged.service.as_ref().unwrap();
        let monitors = service.monitors.as_ref().unwrap();
        assert_eq!(monitors.len(), 2);
        assert_eq!(monitors["github-heartbeat"].name, "GitHub heartbeat");
        let site_policies = service.site_policies.as_ref().unwrap();
        assert_eq!(site_policies.len(), 2);
        assert!(site_policies["google"].manual_login_preferred);
        assert_eq!(site_policies["github"].origin_pattern, "https://github.com");
        assert_eq!(
            service
                .providers
                .as_ref()
                .and_then(|providers| providers.get("manual"))
                .map(|provider| provider.display_name.as_str()),
            Some("Manual")
        );
        assert_eq!(service.reconcile_interval_ms, Some(30_000));
        assert_eq!(service.job_timeout_ms, Some(90_000));
        assert_eq!(service.monitor_interval_ms, Some(45_000));
        assert_eq!(service.recovery_retry_budget, Some(5));
        assert_eq!(service.recovery_base_backoff_ms, Some(500));
        assert_eq!(service.recovery_max_backoff_ms, Some(10_000));
    }

    #[test]
    fn test_apply_runtime_profile_overrides_sets_launch_fields() {
        let mut config = Config {
            runtime_profiles: Some(HashMap::from([(
                "work".to_string(),
                RuntimeProfileConfig {
                    user_data_dir: Some("/tmp/work-user-data".to_string()),
                    launch: Some(RuntimeProfileLaunchConfig {
                        headed: Some(true),
                        leave_open: Some(true),
                        proxy: Some("http://runtime-proxy:8080".to_string()),
                        ..RuntimeProfileLaunchConfig::default()
                    }),
                    auth: Some(RuntimeProfileAuthConfig {
                        session_name: Some("runtime-session".to_string()),
                        manual_login_preferred: Some(true),
                    }),
                    ..RuntimeProfileConfig::default()
                },
            )])),
            ..Config::default()
        };

        apply_runtime_profile_overrides(&mut config, "work");
        assert_eq!(config.runtime_profile.as_deref(), Some("work"));
        assert_eq!(config.profile.as_deref(), Some("/tmp/work-user-data"));
        assert_eq!(config.proxy.as_deref(), Some("http://runtime-proxy:8080"));
        assert_eq!(config.headed, Some(true));
        assert_eq!(config.leave_open, Some(true));
        assert_eq!(config.session_name.as_deref(), Some("runtime-session"));
    }

    #[test]
    fn test_cli_leave_open_tracking() {
        let flags = parse_flags(&args("--leave-open snapshot"));
        assert!(flags.cli_leave_open);
        assert!(flags.leave_open);
    }

    #[test]
    fn test_clean_args_removes_leave_open() {
        let cleaned = clean_args(&args("--leave-open open example.com"));
        assert_eq!(cleaned, args("open example.com"));
    }

    #[test]
    fn test_manual_login_preferred_services_uses_selected_runtime_profile_overlay() {
        let mut config = Config {
            default_runtime_profile: Some("work".to_string()),
            service_defaults: Some(HashMap::from([
                (
                    "google".to_string(),
                    RuntimeProfileServiceConfig {
                        manual_login_preferred: Some(false),
                    },
                ),
                (
                    "github".to_string(),
                    RuntimeProfileServiceConfig {
                        manual_login_preferred: Some(true),
                    },
                ),
            ])),
            runtime_profiles: Some(HashMap::from([(
                "work".to_string(),
                RuntimeProfileConfig {
                    services: Some(HashMap::from([(
                        "google".to_string(),
                        RuntimeProfileServiceConfig {
                            manual_login_preferred: Some(true),
                        },
                    )])),
                    ..RuntimeProfileConfig::default()
                },
            )])),
            ..Config::default()
        };

        apply_runtime_profile_overrides(&mut config, "work");
        assert_eq!(
            manual_login_preferred_services(&config),
            vec!["github".to_string(), "google".to_string()]
        );
    }

    #[test]
    fn test_selected_runtime_profile_from_sources_prefers_arg_then_env_then_config() {
        let config = Config {
            runtime_profile: Some("legacy".to_string()),
            default_runtime_profile: Some("default".to_string()),
            ..Config::default()
        };
        assert_eq!(
            selected_runtime_profile_from_sources(&args("--runtime-profile work open"), &config)
                .as_deref(),
            Some("work")
        );

        let guard = EnvGuard::new(&["AGENT_BROWSER_RUNTIME_PROFILE"]);
        guard.set("AGENT_BROWSER_RUNTIME_PROFILE", "env-profile");
        assert_eq!(
            selected_runtime_profile_from_sources(&args("open"), &config).as_deref(),
            Some("env-profile")
        );
        guard.remove("AGENT_BROWSER_RUNTIME_PROFILE");

        assert_eq!(
            selected_runtime_profile_from_sources(&args("open"), &config).as_deref(),
            Some("legacy")
        );
    }

    #[test]
    fn test_runtime_profile_default_viewport_applies_to_config() {
        let mut config = Config {
            runtime_profiles: Some(HashMap::from([(
                "work".to_string(),
                RuntimeProfileConfig {
                    preferences: Some(RuntimeProfilePreferencesConfig {
                        default_viewport: Some("960x640".to_string()),
                        ..RuntimeProfilePreferencesConfig::default()
                    }),
                    ..RuntimeProfileConfig::default()
                },
            )])),
            ..Config::default()
        };

        apply_runtime_profile_overrides(&mut config, "work");

        assert_eq!(config.default_viewport.as_deref(), Some("960x640"));
    }

    #[test]
    fn test_upsert_runtime_profile_in_user_config_creates_profile_and_default() {
        let temp_home = std::env::temp_dir().join(format!(
            "agent-browser-config-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&temp_home).unwrap();

        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", temp_home.to_str().unwrap());

        let path = upsert_runtime_profile_in_user_config(
            "work",
            Some("/tmp/work-user-data"),
            true,
            Some("chromium"),
        )
        .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(value["defaultRuntimeProfile"], "work");
        assert_eq!(
            value["runtimeProfiles"]["work"]["userDataDir"],
            "/tmp/work-user-data"
        );
        assert_eq!(
            value["runtimeProfiles"]["work"]["browserFamily"],
            "chromium"
        );
    }

    #[test]
    fn test_config_merge_none_does_not_override() {
        let user = Config {
            headed: Some(true),
            proxy: Some("http://proxy:8080".to_string()),
            ..Config::default()
        };
        let project = Config::default();
        let merged = user.merge(project);
        assert_eq!(merged.headed, Some(true));
        assert_eq!(merged.proxy.as_deref(), Some("http://proxy:8080"));
    }

    #[test]
    fn test_load_config_from_file() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("ab-test-config");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("test-config.json");
        let mut f = fs::File::create(&config_path).unwrap();
        writeln!(f, r#"{{"headed": true, "proxy": "http://test:1234"}}"#).unwrap();

        let config = read_config_file(&config_path).unwrap();
        assert_eq!(config.headed, Some(true));
        assert_eq!(config.proxy.as_deref(), Some("http://test:1234"));

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_load_config_normalizes_service_model_ids() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "ab-test-service-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("service-config.json");
        let mut f = fs::File::create(&config_path).unwrap();
        writeln!(
            f,
            r#"{{
                "service": {{
                    "sitePolicies": {{
                        "google": {{
                            "originPattern": "https://accounts.google.com",
                            "browserHost": "docker_headed"
                        }}
                    }},
                    "providers": {{
                        "manual": {{
                            "kind": "manual_approval",
                            "displayName": "Dashboard approval",
                            "capabilities": ["human_approval"]
                        }}
                    }}
                }}
            }}"#
        )
        .unwrap();

        let config = read_config_file(&config_path).unwrap();
        let service = config.service.as_ref().unwrap();

        assert_eq!(
            service
                .site_policies
                .as_ref()
                .and_then(|policies| policies.get("google"))
                .map(|policy| policy.id.as_str()),
            Some("google")
        );
        assert_eq!(
            service
                .providers
                .as_ref()
                .and_then(|providers| providers.get("manual"))
                .map(|provider| provider.id.as_str()),
            Some("manual")
        );

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_state_snapshot_contains_configured_service_entities() {
        let config = Config {
            service: Some(ServiceConfig {
                profiles: Some(BTreeMap::from([(
                    "work".to_string(),
                    BrowserProfile {
                        id: "work".to_string(),
                        name: "Work".to_string(),
                        allocation:
                            crate::native::service_model::ProfileAllocationPolicy::PerService,
                        keyring:
                            crate::native::service_model::ProfileKeyringPolicy::BasicPasswordStore,
                        shared_service_ids: vec!["JournalDownloader".to_string()],
                        ..BrowserProfile::default()
                    },
                )])),
                sessions: Some(BTreeMap::from([(
                    "journal-session".to_string(),
                    BrowserSession {
                        id: "journal-session".to_string(),
                        service_name: Some("JournalDownloader".to_string()),
                        profile_id: Some("work".to_string()),
                        lease: crate::native::service_model::LeaseState::Exclusive,
                        cleanup: crate::native::service_model::SessionCleanupPolicy::CloseTabs,
                        ..BrowserSession::default()
                    },
                )])),
                monitors: Some(BTreeMap::from([(
                    "google-login-freshness".to_string(),
                    SiteMonitor {
                        id: "google-login-freshness".to_string(),
                        name: "Google login freshness".to_string(),
                        target: crate::native::service_model::MonitorTarget::SitePolicy(
                            "google".to_string(),
                        ),
                        state: crate::native::service_model::MonitorState::Active,
                        ..SiteMonitor::default()
                    },
                )])),
                site_policies: Some(BTreeMap::from([(
                    "google".to_string(),
                    SitePolicy {
                        id: "google".to_string(),
                        origin_pattern: "https://accounts.google.com".to_string(),
                        manual_login_preferred: true,
                        ..SitePolicy::default()
                    },
                )])),
                providers: Some(BTreeMap::from([(
                    "manual".to_string(),
                    ServiceProvider {
                        id: "manual".to_string(),
                        display_name: "Dashboard approval".to_string(),
                        ..ServiceProvider::default()
                    },
                )])),
                reconcile_interval_ms: Some(45_000),
                job_timeout_ms: Some(120_000),
                monitor_interval_ms: Some(60_000),
                recovery_retry_budget: None,
                recovery_base_backoff_ms: None,
                recovery_max_backoff_ms: None,
                default_browser_build: Some(BrowserBuild::StealthcdpChromium),
                browser_build_manifests: None,
            }),
            ..Config::default()
        };

        let state = config.service_state_snapshot();

        assert_eq!(state.profiles.len(), 1);
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.monitors.len(), 1);
        assert_eq!(state.site_policies.len(), 1);
        assert_eq!(state.providers.len(), 1);
        assert_eq!(
            state.profiles["work"].shared_service_ids,
            vec!["JournalDownloader"]
        );
        assert_eq!(
            state.sessions["journal-session"].profile_id.as_deref(),
            Some("work")
        );
        assert!(state.site_policies["google"].manual_login_preferred);
        assert_eq!(
            state.monitors["google-login-freshness"].name,
            "Google login freshness"
        );
        assert_eq!(state.providers["manual"].display_name, "Dashboard approval");
        assert!(state.browsers.is_empty());
    }

    #[test]
    fn test_parse_flags_loads_service_reconcile_interval_from_config() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-service-reconcile-config-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("agent-browser.json");
        fs::write(&config_path, r#"{"service":{"reconcileIntervalMs":5000}}"#).unwrap();

        let flags = parse_flags(&args(&format!(
            "--config {} service status",
            config_path.display()
        )));

        assert_eq!(flags.service_reconcile_interval_ms, Some(5000));
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_parse_flags_defaults_service_reconcile_interval() {
        let home = std::env::temp_dir().join(format!(
            "agent-browser-service-reconcile-default-home-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS", "HOME"]);
        guard.remove("AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS");
        guard.set("HOME", home.to_str().unwrap());

        let flags = parse_flags(&args("service status"));

        assert_eq!(
            flags.service_reconcile_interval_ms,
            Some(DEFAULT_SERVICE_RECONCILE_INTERVAL_MS)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_service_reconcile_interval_zero_disables_default() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-service-reconcile-disable-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("agent-browser.json");
        fs::write(&config_path, r#"{"service":{"reconcileIntervalMs":0}}"#).unwrap();

        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS");
        let flags = parse_flags(&args(&format!(
            "--config {} service status",
            config_path.display()
        )));

        assert_eq!(flags.service_reconcile_interval_ms, None);
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_reconcile_interval_flag_overrides_config() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-service-reconcile-flag-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("agent-browser.json");
        fs::write(&config_path, r#"{"service":{"reconcileIntervalMs":5000}}"#).unwrap();

        let flags = parse_flags(&args(&format!(
            "--config {} --service-reconcile-interval 250 service status",
            config_path.display()
        )));

        assert_eq!(flags.service_reconcile_interval_ms, Some(250));
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_reconcile_interval_flag_zero_disables_default() {
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS");

        let flags = parse_flags(&args("--service-reconcile-interval 0 service status"));

        assert_eq!(flags.service_reconcile_interval_ms, None);
    }

    #[test]
    fn test_parse_flags_loads_service_monitor_interval_from_config() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-monitor-interval-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS");
        let config_path = dir.join("agent-browser.json");
        fs::write(&config_path, r#"{"service":{"monitorIntervalMs":5000}}"#).unwrap();

        let flags = parse_flags(&args(&format!(
            "--config {} service status",
            config_path.display()
        )));

        assert_eq!(flags.service_monitor_interval_ms, Some(5000));
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_monitor_interval_flag_zero_disables_default() {
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS");

        let flags = parse_flags(&args("--service-monitor-interval 0 service status"));

        assert_eq!(flags.service_monitor_interval_ms, None);
    }

    #[test]
    fn test_parse_flags_loads_service_job_timeout_from_config() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-job-timeout-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS");
        let config_path = dir.join("agent-browser.json");
        fs::write(&config_path, r#"{"service":{"jobTimeoutMs":5000}}"#).unwrap();

        let flags = parse_flags(&args(&format!(
            "--config {} service status",
            config_path.display()
        )));

        assert_eq!(flags.service_job_timeout_ms, Some(5000));
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_job_timeout_flag_overrides_config() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-job-timeout-flag-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS");
        let config_path = dir.join("agent-browser.json");
        fs::write(&config_path, r#"{"service":{"jobTimeoutMs":5000}}"#).unwrap();

        let flags = parse_flags(&args(&format!(
            "--config {} --service-job-timeout 250 service status",
            config_path.display()
        )));

        assert_eq!(flags.service_job_timeout_ms, Some(250));
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_job_timeout_flag_zero_disables_timeout() {
        let guard = EnvGuard::new(&["AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS"]);
        guard.remove("AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS");

        let flags = parse_flags(&args("--service-job-timeout 0 service status"));

        assert_eq!(flags.service_job_timeout_ms, None);
    }

    #[test]
    fn test_parse_flags_loads_service_recovery_policy_from_config() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-recovery-policy-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("agent-browser.json");
        fs::write(
            &config_path,
            r#"{"service":{"recoveryRetryBudget":5,"recoveryBaseBackoffMs":250,"recoveryMaxBackoffMs":10000}}"#,
        )
        .unwrap();

        let guard = EnvGuard::new(&[
            "AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET",
            "AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS",
            "AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS",
        ]);
        guard.remove("AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET");
        guard.remove("AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS");
        guard.remove("AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS");
        let flags = parse_flags(&args(&format!(
            "--config {} service status",
            config_path.display()
        )));

        assert_eq!(flags.service_recovery_retry_budget, 5);
        assert_eq!(flags.service_recovery_base_backoff_ms, 250);
        assert_eq!(flags.service_recovery_max_backoff_ms, 10_000);
        assert_eq!(
            flags.service_recovery_retry_budget_source,
            BrowserRecoveryPolicyValueSource::Config
        );
        assert_eq!(
            flags.service_recovery_base_backoff_ms_source,
            BrowserRecoveryPolicyValueSource::Config
        );
        assert_eq!(
            flags.service_recovery_max_backoff_ms_source,
            BrowserRecoveryPolicyValueSource::Config
        );
        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_service_recovery_policy_flags_override_config() {
        let flags = parse_flags(&args(
            "--service-recovery-retry-budget 7 --service-recovery-base-backoff 500 --service-recovery-max-backoff 15000 service status",
        ));

        assert_eq!(flags.service_recovery_retry_budget, 7);
        assert_eq!(flags.service_recovery_base_backoff_ms, 500);
        assert_eq!(flags.service_recovery_max_backoff_ms, 15_000);
        assert_eq!(
            flags.service_recovery_retry_budget_source,
            BrowserRecoveryPolicyValueSource::Cli
        );
        assert_eq!(
            flags.service_recovery_base_backoff_ms_source,
            BrowserRecoveryPolicyValueSource::Cli
        );
        assert_eq!(
            flags.service_recovery_max_backoff_ms_source,
            BrowserRecoveryPolicyValueSource::Cli
        );
    }

    #[test]
    fn test_service_state_from_store_overlays_configured_entities() {
        let temp_home = std::env::temp_dir().join(format!(
            "agent-browser-service-store-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&temp_home).unwrap();
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", temp_home.to_str().unwrap());

        let persisted = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                crate::native::service_model::BrowserProcess {
                    id: "browser-1".to_string(),
                    health: crate::native::service_model::BrowserHealth::Ready,
                    ..crate::native::service_model::BrowserProcess::default()
                },
            )]),
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "persisted".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            ..ServiceState::default()
        };
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store.save(&persisted).unwrap();

        let configured = ServiceState {
            site_policies: BTreeMap::from([(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "configured".to_string(),
                    ..SitePolicy::default()
                },
            )]),
            providers: BTreeMap::from([(
                "manual".to_string(),
                ServiceProvider {
                    id: "manual".to_string(),
                    display_name: "Manual approval".to_string(),
                    ..ServiceProvider::default()
                },
            )]),
            ..ServiceState::default()
        };

        let state = service_state_from_store(configured);

        assert_eq!(state.browsers["browser-1"].id, "browser-1");
        assert_eq!(state.site_policies["google"].origin_pattern, "configured");
        assert_eq!(state.providers["manual"].display_name, "Manual approval");
        let _ = std::fs::remove_dir_all(&temp_home);
    }

    #[test]
    fn test_parse_flags_loads_persisted_service_state() {
        let temp_home = std::env::temp_dir().join(format!(
            "agent-browser-parse-service-store-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&temp_home).unwrap();
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", temp_home.to_str().unwrap());

        let persisted = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                crate::native::service_model::BrowserProcess {
                    id: "browser-1".to_string(),
                    health: crate::native::service_model::BrowserHealth::Ready,
                    ..crate::native::service_model::BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store.save(&persisted).unwrap();

        let flags = parse_flags(&args("service status"));

        assert_eq!(flags.service_state.browsers["browser-1"].id, "browser-1");
        let _ = std::fs::remove_dir_all(&temp_home);
    }

    #[test]
    fn test_load_config_from_file_parses_idle_timeout() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("ab-test-idle-timeout-config");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("test-config.json");
        let mut f = fs::File::create(&config_path).unwrap();
        writeln!(f, r#"{{"idleTimeout": "10s"}}"#).unwrap();

        let config = read_config_file(&config_path).unwrap();
        assert_eq!(config.idle_timeout.as_deref(), Some("10000"));

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_load_config_missing_file_returns_none() {
        let result = read_config_file(&PathBuf::from("/nonexistent/agent-browser.json"));
        assert!(result.is_none());
    }

    #[test]
    fn test_load_config_malformed_json_returns_none() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("ab-test-malformed");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("bad-config.json");
        let mut f = fs::File::create(&config_path).unwrap();
        writeln!(f, "{{not valid json}}").unwrap();

        let result = read_config_file(&config_path);
        assert!(result.is_none());

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_extract_config_path() {
        assert_eq!(
            extract_config_path(&args("--config ./my-config.json open example.com")),
            Some(Some("./my-config.json".to_string()))
        );
    }

    #[test]
    fn test_extract_config_path_missing() {
        assert_eq!(extract_config_path(&args("open example.com")), None);
    }

    #[test]
    fn test_extract_config_path_no_value() {
        assert_eq!(extract_config_path(&args("--config")), Some(None));
    }

    #[test]
    fn test_extract_config_path_skips_flag_values() {
        assert_eq!(extract_config_path(&args("--args --config open")), None);
    }

    #[test]
    fn test_clean_args_removes_config() {
        let cleaned = clean_args(&args("--config ./config.json open example.com"));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_load_config_with_config_flag() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("ab-test-flag-config");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("custom.json");
        let mut f = fs::File::create(&config_path).unwrap();
        writeln!(f, r#"{{"headed": true, "session": "custom"}}"#).unwrap();

        let flag_args = vec![
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "open".to_string(),
            "example.com".to_string(),
        ];
        let config = load_config(&flag_args).unwrap();
        assert_eq!(config.headed, Some(true));
        assert_eq!(config.session.as_deref(), Some("custom"));

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_load_config_error_missing_config_value() {
        let result = load_config(&args("--config"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a file path"));
    }

    #[test]
    fn test_load_config_error_nonexistent_file() {
        let result = load_config(&args("--config /nonexistent/config.json open"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("config file not found"));
    }

    #[test]
    fn test_load_config_error_malformed_explicit() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("ab-test-explicit-malformed");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("bad.json");
        let mut f = fs::File::create(&config_path).unwrap();
        writeln!(f, "{{not valid}}").unwrap();

        let flag_args = vec![
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
        ];
        let result = load_config(&flag_args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to load config"));

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_dir(&dir);
    }

    // === Boolean flag value tests ===

    #[test]
    fn test_headed_false() {
        let flags = parse_flags(&args("--headed false open example.com"));
        assert!(!flags.headed);
    }

    #[test]
    fn test_headed_true_explicit() {
        let flags = parse_flags(&args("--headed true open example.com"));
        assert!(flags.headed);
    }

    #[test]
    fn test_headed_bare_defaults_true() {
        let flags = parse_flags(&args("--headed open example.com"));
        assert!(flags.headed);
    }

    #[test]
    fn test_debug_false() {
        let flags = parse_flags(&args("--debug false open example.com"));
        assert!(!flags.debug);
    }

    #[test]
    fn test_json_false() {
        let flags = parse_flags(&args("--json false open example.com"));
        assert!(!flags.json);
    }

    #[test]
    fn test_ignore_https_errors_false() {
        let flags = parse_flags(&args("--ignore-https-errors false open"));
        assert!(!flags.ignore_https_errors);
    }

    #[test]
    fn test_allow_file_access_false() {
        let flags = parse_flags(&args("--allow-file-access false open"));
        assert!(!flags.allow_file_access);
        assert!(flags.cli_allow_file_access);
    }

    #[test]
    fn test_auto_connect_false() {
        let flags = parse_flags(&args("--auto-connect false open"));
        assert!(!flags.auto_connect);
    }

    #[test]
    fn test_clean_args_removes_bool_flag_with_value() {
        let cleaned = clean_args(&args("--headed false --debug true open example.com"));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    #[test]
    fn test_clean_args_removes_bare_bool_flag() {
        let cleaned = clean_args(&args("--headed --debug open example.com"));
        assert_eq!(cleaned, vec!["open", "example.com"]);
    }

    // === Extensions merge tests ===

    #[test]
    fn test_config_merge_extensions_concatenated() {
        let user = Config {
            extensions: Some(vec!["/ext1".to_string()]),
            ..Config::default()
        };
        let project = Config {
            extensions: Some(vec!["/ext2".to_string(), "/ext3".to_string()]),
            ..Config::default()
        };
        let merged = user.merge(project);
        assert_eq!(
            merged.extensions,
            Some(vec![
                "/ext1".to_string(),
                "/ext2".to_string(),
                "/ext3".to_string()
            ])
        );
    }

    #[test]
    fn test_config_merge_extensions_user_only() {
        let user = Config {
            extensions: Some(vec!["/ext1".to_string()]),
            ..Config::default()
        };
        let project = Config::default();
        let merged = user.merge(project);
        assert_eq!(merged.extensions, Some(vec!["/ext1".to_string()]));
    }

    #[test]
    fn test_config_merge_extensions_project_only() {
        let user = Config::default();
        let project = Config {
            extensions: Some(vec!["/ext2".to_string()]),
            ..Config::default()
        };
        let merged = user.merge(project);
        assert_eq!(merged.extensions, Some(vec!["/ext2".to_string()]));
    }

    #[test]
    fn test_no_auto_dialog_flag() {
        let flags = parse_flags(&args("open example.com --no-auto-dialog"));
        assert!(flags.no_auto_dialog);
    }

    #[test]
    fn test_no_auto_dialog_default_false() {
        let flags = parse_flags(&args("open example.com"));
        assert!(!flags.no_auto_dialog);
    }

    #[test]
    fn test_clean_args_removes_no_auto_dialog() {
        let input: Vec<String> = vec![
            "open".to_string(),
            "example.com".to_string(),
            "--no-auto-dialog".to_string(),
        ];
        let clean = clean_args(&input);
        assert_eq!(clean, vec!["open", "example.com"]);
    }
}
