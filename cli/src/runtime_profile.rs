use crate::process_identity::{
    assess_process_ownership, observe_process, LegacyProfileProof, ProcessObservation,
    RecordedProcessIdentity, RuntimeProcessAssessment, RuntimeProcessOwnership,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const AGENT_BROWSER_DIR: &str = ".agent-browser";
const LEGACY_PROFILE_DIR: &str = "profile";
const RUNTIME_PROFILES_DIR: &str = "runtime-profiles";
const USER_DATA_DIR: &str = "user-data";
const RUNTIME_STATE_FILENAME: &str = "runtime-state.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProfile {
    pub runtime_profile: Option<String>,
    pub user_data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub runtime_profile: String,
    pub user_data_dir: String,
    pub browser_pid: u32,
    /// Process-start and executable evidence that distinguishes the launched
    /// browser instance from a later unrelated process that reuses its PID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_identity: Option<RecordedProcessIdentity>,
    pub headed: bool,
    pub launch_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devtools_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_url: Option<String>,
    /// Authoritative launch metadata retained even when the browser has no CDP
    /// endpoint. This keeps manual authentication browsers discoverable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_record: Option<RuntimeLaunchRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct RuntimeLaunchRecord {
    pub target_url: Option<String>,
    pub browser_family: Option<String>,
    pub browser_build: Option<String>,
    pub display: Option<String>,
    pub remote_view_route_id: Option<String>,
    pub remote_view_url: Option<String>,
    pub started_at: Option<String>,
    pub last_observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTarget {
    pub id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub runtime_profile: String,
    pub user_data_dir: String,
    pub state_path: String,
    pub browser_pid: Option<u32>,
    pub browser_alive: bool,
    pub headed: Option<bool>,
    pub launch_mode: Option<String>,
    pub devtools_port: Option<u16>,
    /// True only when the recorded DevTools port answers the runtime target probe.
    pub devtools_reachable: bool,
    pub ws_url: Option<String>,
    pub targets: Vec<RuntimeTarget>,
    pub launch_record: Option<RuntimeLaunchRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfileSummary {
    pub runtime_profile: String,
    pub user_data_dir: String,
    pub state_path: String,
    pub configured: bool,
    pub default: bool,
    pub browser_pid: Option<u32>,
    pub browser_alive: bool,
    pub headed: Option<bool>,
    pub launch_mode: Option<String>,
    pub devtools_port: Option<u16>,
    pub devtools_reachable: bool,
    pub ws_url: Option<String>,
    pub launch_record: Option<RuntimeLaunchRecord>,
}

/// Operator inventory row for a live headed runtime browser that may have no
/// CDP endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManualRuntimeBrowser {
    pub id: String,
    pub runtime_profile: String,
    pub profile_path: String,
    pub pid: u32,
    pub browser_family: Option<String>,
    pub browser_build: Option<String>,
    pub display: Option<String>,
    pub launch_mode: String,
    pub target_url: Option<String>,
    pub devtools_port: Option<u16>,
    pub automation_available: bool,
    pub remote_view_route_id: Option<String>,
    pub remote_view_url: Option<String>,
    pub remote_control_available: bool,
    pub next_safe_action: String,
    pub started_at: Option<String>,
    pub last_observed_at: Option<String>,
}

pub fn looks_like_path(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with('/')
        || value.starts_with("~/")
        || value.starts_with("./")
        || value.starts_with("../")
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
}

pub fn validate_runtime_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Runtime profile name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(format!(
            "Invalid runtime profile '{}'. Must match /^[a-zA-Z0-9_-]+$/",
            name
        ));
    }
    Ok(())
}

pub fn default_runtime_profile_name() -> String {
    "default".to_string()
}

pub fn runtime_profiles_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(AGENT_BROWSER_DIR).join(RUNTIME_PROFILES_DIR))
        .ok_or_else(|| "Could not determine home directory".to_string())
}

pub fn runtime_profile_root(name: &str) -> Result<PathBuf, String> {
    validate_runtime_profile_name(name)?;
    Ok(runtime_profiles_root()?.join(name))
}

pub fn runtime_profile_user_data_dir(name: &str) -> Result<PathBuf, String> {
    Ok(runtime_profile_root(name)?.join(USER_DATA_DIR))
}

pub fn runtime_profile_state_path(name: &str) -> Result<PathBuf, String> {
    Ok(runtime_profile_root(name)?.join(RUNTIME_STATE_FILENAME))
}

pub fn resolve_profile(
    profile: Option<&str>,
    runtime_profile: Option<&str>,
) -> Result<ResolvedProfile, String> {
    if let Some(profile) = profile {
        if looks_like_path(profile) {
            return Ok(ResolvedProfile {
                runtime_profile: runtime_profile.map(str::to_string),
                user_data_dir: expand_tilde(profile),
            });
        }

        validate_runtime_profile_name(profile)?;
        return Ok(ResolvedProfile {
            runtime_profile: Some(profile.to_string()),
            user_data_dir: resolved_runtime_profile_user_data_dir(profile)?,
        });
    }

    let runtime_name = runtime_profile.unwrap_or("default");
    validate_runtime_profile_name(runtime_name)?;
    Ok(ResolvedProfile {
        runtime_profile: Some(runtime_name.to_string()),
        user_data_dir: resolved_runtime_profile_user_data_dir(runtime_name)?,
    })
}

fn resolved_runtime_profile_user_data_dir(runtime_profile: &str) -> Result<PathBuf, String> {
    let target = runtime_profile_user_data_dir(runtime_profile)?;
    if runtime_profile == "default" && !target.exists() {
        if let Some(home) = dirs::home_dir() {
            let legacy = home.join(AGENT_BROWSER_DIR).join(LEGACY_PROFILE_DIR);
            if legacy.exists() {
                return Ok(legacy);
            }
        }
    }
    Ok(target)
}

pub fn write_runtime_state(state: &RuntimeState) -> Result<(), String> {
    let path = runtime_profile_state_path(&state.runtime_profile)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create runtime profile dir {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize runtime state: {}", e))?;
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write runtime state {}: {}", path.display(), e))
}

pub fn read_runtime_state(runtime_profile: &str) -> Result<Option<RuntimeState>, String> {
    let path = runtime_profile_state_path(runtime_profile)?;
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Failed to read runtime state {}: {}",
                path.display(),
                e
            ))
        }
    };

    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| format!("Failed to parse runtime state {}: {}", path.display(), e))
}

pub fn clear_runtime_state(runtime_profile: &str) -> Result<(), String> {
    let path = runtime_profile_state_path(runtime_profile)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "Failed to remove runtime state {}: {}",
            path.display(),
            e
        )),
    }
}

pub fn read_devtools_port(user_data_dir: &Path) -> Option<u16> {
    for path in [
        user_data_dir.join("DevToolsActivePort"),
        user_data_dir.join("Default").join("DevToolsActivePort"),
    ] {
        let raw = fs::read_to_string(path).ok()?;
        if let Some(port) = raw
            .lines()
            .next()
            .and_then(|line| line.trim().parse::<u16>().ok())
        {
            return Some(port);
        }
    }
    None
}

/// Resolve runtime profile status, using a config-provided user-data-dir when
/// no runtime-state file exists yet for that profile.
pub fn runtime_status_with_user_data_dir(
    runtime_profile: &str,
    configured_user_data_dir: Option<&Path>,
) -> Result<RuntimeStatus, String> {
    validate_runtime_profile_name(runtime_profile)?;
    let state_path = runtime_profile_state_path(runtime_profile)?;
    let state = read_runtime_state(runtime_profile)?;
    let user_data_dir = state
        .as_ref()
        .map(|s| PathBuf::from(&s.user_data_dir))
        .or_else(|| configured_user_data_dir.map(Path::to_path_buf))
        .unwrap_or(resolved_runtime_profile_user_data_dir(runtime_profile)?);
    let browser_pid = state.as_ref().map(|s| s.browser_pid);
    let detected_devtools_port = state
        .as_ref()
        .and_then(|s| s.devtools_port)
        .or_else(|| read_devtools_port(&user_data_dir));
    let evaluation = evaluate_runtime_process(
        state.as_ref(),
        &user_data_dir,
        browser_pid.unwrap_or_default(),
        detected_devtools_port,
    );
    let target_probe = evaluation.targets;
    let devtools_reachable = target_probe.is_some();
    let browser_alive = evaluation.assessment.authorizes_adoption();
    let devtools_port = browser_alive.then_some(detected_devtools_port).flatten();
    let targets = target_probe.unwrap_or_default();

    Ok(RuntimeStatus {
        runtime_profile: runtime_profile.to_string(),
        user_data_dir: user_data_dir.display().to_string(),
        state_path: state_path.display().to_string(),
        browser_pid,
        browser_alive,
        headed: state.as_ref().map(|s| s.headed),
        launch_mode: state.as_ref().map(|s| s.launch_mode.clone()),
        devtools_port,
        devtools_reachable,
        ws_url: state.as_ref().and_then(|s| s.ws_url.clone()),
        targets,
        launch_record: state.and_then(|s| s.launch_record),
    })
}

pub fn profile_lock_process_assessment(user_data_dir: &Path, pid: u32) -> RuntimeProcessAssessment {
    let state = runtime_state_for_user_data_dir(user_data_dir, pid);
    let detected_devtools_port = state
        .as_ref()
        .and_then(|state| state.devtools_port)
        .or_else(|| read_devtools_port(user_data_dir));
    evaluate_runtime_process(state.as_ref(), user_data_dir, pid, detected_devtools_port).assessment
}

pub fn runtime_process_assessment(
    runtime_profile: Option<&str>,
    pid: u32,
) -> RuntimeProcessAssessment {
    let state = runtime_profile
        .and_then(|name| read_runtime_state(name).ok().flatten())
        .filter(|state| state.browser_pid == pid);
    let user_data_dir = state
        .as_ref()
        .map(|state| PathBuf::from(&state.user_data_dir))
        .or_else(|| {
            runtime_profile.and_then(|name| resolved_runtime_profile_user_data_dir(name).ok())
        })
        .unwrap_or_default();
    let detected_devtools_port = state
        .as_ref()
        .and_then(|state| state.devtools_port)
        .or_else(|| read_devtools_port(&user_data_dir));
    evaluate_runtime_process(state.as_ref(), &user_data_dir, pid, detected_devtools_port).assessment
}

struct RuntimeProcessEvaluation {
    assessment: RuntimeProcessAssessment,
    targets: Option<Vec<RuntimeTarget>>,
}

fn evaluate_runtime_process(
    state: Option<&RuntimeState>,
    user_data_dir: &Path,
    pid: u32,
    detected_devtools_port: Option<u16>,
) -> RuntimeProcessEvaluation {
    let observation = if pid == 0 {
        ProcessObservation::Missing
    } else {
        observe_process(pid)
    };
    let initial = assess_process_ownership(
        state.and_then(|state| state.process_identity.as_ref()),
        observation.clone(),
        LegacyProfileProof::Unproven,
    );
    let may_probe_exact = initial.ownership == RuntimeProcessOwnership::MatchingBrowser;
    let may_probe_legacy = state.is_some_and(|state| {
        state.process_identity.is_none()
            && state.browser_pid == pid
            && paths_refer_to_same_location(Path::new(&state.user_data_dir), user_data_dir)
            && detected_devtools_port.is_some_and(|port| {
                observation_command_line_matches_profile(&observation, user_data_dir, port)
            })
    });
    let targets = if may_probe_exact || may_probe_legacy {
        detected_devtools_port.and_then(|port| fetch_runtime_targets(port).ok())
    } else {
        None
    };
    let assessment = if may_probe_legacy && targets.is_some() {
        assess_process_ownership(
            state.and_then(|state| state.process_identity.as_ref()),
            observation,
            LegacyProfileProof::ProfileConsistent,
        )
    } else {
        initial
    };
    RuntimeProcessEvaluation {
        assessment,
        targets,
    }
}

fn observation_command_line_matches_profile(
    observation: &ProcessObservation,
    user_data_dir: &Path,
    devtools_port: u16,
) -> bool {
    let ProcessObservation::Observed(observed) = observation else {
        return false;
    };
    if observed.browser_family.is_none() {
        return false;
    }
    let Some(arguments) = observed.command_line.as_deref() else {
        return false;
    };
    command_line_option_value(arguments, "--user-data-dir")
        .is_some_and(|value| paths_refer_to_same_location(Path::new(value), user_data_dir))
        && command_line_option_value(arguments, "--remote-debugging-port")
            .and_then(|value| value.parse::<u16>().ok())
            == Some(devtools_port)
}

fn command_line_option_value<'a>(arguments: &'a [String], option: &str) -> Option<&'a str> {
    for (index, argument) in arguments.iter().enumerate() {
        if let Some(value) = argument.strip_prefix(&format!("{option}=")) {
            return Some(value);
        }
        if argument == option {
            return arguments.get(index + 1).map(String::as_str);
        }
    }
    None
}

fn runtime_state_for_user_data_dir(user_data_dir: &Path, pid: u32) -> Option<RuntimeState> {
    let root = runtime_profiles_root().ok()?;
    fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            read_runtime_state(&name).ok().flatten()
        })
        .find(|state| {
            state.browser_pid == pid
                && paths_refer_to_same_location(Path::new(&state.user_data_dir), user_data_dir)
        })
}

fn paths_refer_to_same_location(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// Merge configured runtime profiles with any on-disk managed profiles so
/// callers can inspect the full runtime-profile namespace in one command.
pub fn list_runtime_profiles(
    configured_profiles: &[(String, Option<PathBuf>)],
    default_runtime_profile: Option<&str>,
) -> Result<Vec<RuntimeProfileSummary>, String> {
    let mut names = BTreeSet::new();

    for (name, _) in configured_profiles {
        validate_runtime_profile_name(name)?;
        names.insert(name.clone());
    }

    if let Some(default_name) = default_runtime_profile {
        validate_runtime_profile_name(default_name)?;
        names.insert(default_name.to_string());
    }

    if let Ok(root) = runtime_profiles_root() {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if !file_type.is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if validate_runtime_profile_name(&name).is_ok() {
                    names.insert(name);
                }
            }
        }
    }

    if names.is_empty() {
        names.insert(default_runtime_profile_name());
    }

    let default_name = default_runtime_profile.unwrap_or("default");
    let mut items = Vec::with_capacity(names.len());
    for name in names {
        let configured_user_data_dir = configured_profiles
            .iter()
            .find(|(profile_name, _)| profile_name == &name)
            .and_then(|(_, path)| path.clone());
        let status = runtime_status_with_user_data_dir(&name, configured_user_data_dir.as_deref())?;
        items.push(RuntimeProfileSummary {
            runtime_profile: status.runtime_profile,
            user_data_dir: status.user_data_dir,
            state_path: status.state_path,
            configured: configured_profiles
                .iter()
                .any(|(profile_name, _)| profile_name == &name),
            default: name == default_name,
            browser_pid: status.browser_pid,
            browser_alive: status.browser_alive,
            headed: status.headed,
            launch_mode: status.launch_mode,
            devtools_port: status.devtools_port,
            devtools_reachable: status.devtools_reachable,
            ws_url: status.ws_url,
            launch_record: status.launch_record,
        });
    }

    Ok(items)
}

/// Discover live manual runtime browsers from authoritative runtime-state
/// records without requiring CDP attachment.
pub fn list_manual_runtime_browsers() -> Result<Vec<ManualRuntimeBrowser>, String> {
    let mut browsers = list_runtime_profiles(&[], None)?
        .into_iter()
        .filter_map(|profile| {
            let launch_mode = profile.launch_mode.clone()?;
            if !profile.browser_alive || !launch_mode.starts_with("manual") {
                return None;
            }
            let pid = profile.browser_pid?;
            let launch = profile.launch_record.unwrap_or_default();
            let automation_available =
                profile.devtools_port.is_some() && profile.devtools_reachable;
            Some(ManualRuntimeBrowser {
                id: format!("manual-runtime:{}:{pid}", profile.runtime_profile),
                runtime_profile: profile.runtime_profile,
                profile_path: profile.user_data_dir,
                pid,
                browser_family: launch.browser_family,
                browser_build: launch.browser_build,
                display: launch.display,
                launch_mode,
                target_url: launch.target_url,
                devtools_port: profile.devtools_port,
                automation_available,
                remote_view_route_id: launch.remote_view_route_id,
                remote_view_url: launch.remote_view_url.clone(),
                remote_control_available: launch.remote_view_url.is_some(),
                next_safe_action: if automation_available {
                    "reuse_or_add_tab".to_string()
                } else if launch.remote_view_url.is_some() {
                    "open_remote_view_or_finish_login_then_close".to_string()
                } else {
                    "finish_login_then_close_or_relaunch_attachable".to_string()
                },
                started_at: launch.started_at,
                last_observed_at: launch.last_observed_at,
            })
        })
        .collect::<Vec<_>>();
    browsers.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(browsers)
}

fn fetch_runtime_targets(port: u16) -> Result<Vec<RuntimeTarget>, String> {
    let json = http_get_json(port, "/json/list")?;
    let list = json
        .as_array()
        .ok_or_else(|| "Invalid /json/list response".to_string())?;
    Ok(list
        .iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            let target_type = entry.get("type")?.as_str()?.to_string();
            let title = entry
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let url = entry
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Some(RuntimeTarget {
                id,
                target_type,
                title,
                url,
            })
        })
        .collect())
}

fn http_get_json(port: u16, path: &str) -> Result<Value, String> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .map_err(|e| format!("Invalid DevTools address: {}", e))?,
        Duration::from_millis(500),
    )
    .map_err(|e| format!("Failed to connect to DevTools port {}: {}", port, e))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| format!("Failed to set read timeout: {}", e))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| format!("Failed to set write timeout: {}", e))?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nUser-Agent: agent-browser\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        path, port
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to write HTTP request: {}", e))?;

    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&buffer[..n]),
            Err(e)
                if !bytes.is_empty()
                    && matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
            {
                break;
            }
            Err(e) => return Err(format!("Failed to read HTTP response: {}", e)),
        }
    }
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "Malformed HTTP response from DevTools".to_string())?;
    let header_bytes = &bytes[..header_end];
    let body_bytes = &bytes[header_end + 4..];
    let headers = String::from_utf8(header_bytes.to_vec())
        .map_err(|e| format!("Failed to decode DevTools HTTP headers: {}", e))?;
    let is_chunked = headers.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("Transfer-Encoding")
                && value
                    .split(',')
                    .any(|encoding| encoding.trim().eq_ignore_ascii_case("chunked"))
        })
    });
    let body = if is_chunked {
        decode_chunked_body(body_bytes)?
    } else {
        body_bytes.to_vec()
    };
    let body = String::from_utf8(body)
        .map_err(|e| format!("Failed to decode DevTools JSON body: {}", e))?;
    serde_json::from_str(&body).map_err(|e| format!("Failed to parse DevTools JSON: {}", e))
}

fn decode_chunked_body(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut rest = body;
    let mut decoded = Vec::new();

    loop {
        let size_line_end = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "Malformed chunked DevTools response".to_string())?;
        let size_line = std::str::from_utf8(&rest[..size_line_end])
            .map_err(|e| format!("Invalid chunk header encoding in DevTools response: {}", e))?;
        let after_size = &rest[size_line_end + 2..];
        let size_hex = size_line.split(';').next().unwrap_or(size_line).trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|e| format!("Invalid chunk size in DevTools response: {}", e))?;
        if size == 0 {
            return Ok(decoded);
        }
        if after_size.len() < size + 2 {
            return Err("Truncated chunked DevTools response".to_string());
        }
        decoded.extend_from_slice(&after_size[..size]);
        rest = &after_size[size..];
        if !rest.starts_with(b"\r\n") {
            return Err("Malformed chunk terminator in DevTools response".to_string());
        }
        rest = &rest[2..];
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use std::env;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("/tmp/foo"));
        assert!(looks_like_path("~/foo"));
        assert!(looks_like_path("./foo"));
        assert!(looks_like_path("../foo"));
        assert!(looks_like_path("relative/path"));
        assert!(!looks_like_path("default"));
    }

    #[test]
    fn test_validate_runtime_profile_name() {
        assert!(validate_runtime_profile_name("default").is_ok());
        assert!(validate_runtime_profile_name("work_2").is_ok());
        assert!(validate_runtime_profile_name("bad/name").is_err());
        assert!(validate_runtime_profile_name("").is_err());
    }

    #[test]
    fn test_legacy_runtime_state_without_process_identity_deserializes() {
        let state: RuntimeState = serde_json::from_value(serde_json::json!({
            "runtimeProfile": "legacy",
            "userDataDir": "/tmp/legacy",
            "browserPid": 42,
            "headed": true,
            "launchMode": "manual",
            "devtoolsPort": null,
            "wsUrl": null,
            "launchRecord": null
        }))
        .unwrap();

        assert_eq!(state.browser_pid, 42);
        assert_eq!(state.process_identity, None);
    }

    #[test]
    fn test_http_get_json() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut _buf = [0u8; 1024];
                let _ = stream.read(&mut _buf);
                let body = r#"[{"id":"page-1","type":"page","title":"Example","url":"https://example.com"}]"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let json = http_get_json(port, "/json/list").unwrap();
        assert_eq!(json[0]["id"], "page-1");
    }

    #[test]
    fn test_http_get_json_decodes_chunked_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut _buf = [0u8; 1024];
                let _ = stream.read(&mut _buf);
                let chunk = r#"[{"id":"page-2","type":"page","title":"Chunked","url":"https://example.com"}]"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
                    chunk.len(),
                    chunk
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let json = http_get_json(port, "/json/list").unwrap();
        assert_eq!(json[0]["id"], "page-2");
    }

    #[test]
    fn test_http_get_json_decodes_chunked_body_with_split_utf8() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut _buf = [0u8; 1024];
                let _ = stream.read(&mut _buf);
                let first = br#"[{"id":"page-3","type":"page","title":"caf"#;
                let second = &[0xc3, 0xa9];
                let third = br#"","url":"https://example.com"}]"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{:x}\r\n",
                    first.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(first);
                let _ = stream.write_all(b"\r\n");
                let response = format!("{:x}\r\n", second.len());
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(second);
                let _ = stream.write_all(b"\r\n");
                let response = format!("{:x}\r\n", third.len());
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(third);
                let _ = stream.write_all(b"\r\n0\r\n\r\n");
            }
        });

        let json = http_get_json(port, "/json/list").unwrap();
        assert_eq!(json[0]["title"], "café");
    }

    #[test]
    fn test_runtime_status_uses_configured_user_data_dir_without_state() {
        let runtime_profile = format!(
            "testcfg{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let configured_user_data_dir =
            env::temp_dir().join(format!("{}-user-data", runtime_profile));

        let _ = clear_runtime_state(&runtime_profile);
        let status =
            runtime_status_with_user_data_dir(&runtime_profile, Some(&configured_user_data_dir))
                .unwrap();

        assert_eq!(
            status.user_data_dir,
            configured_user_data_dir.display().to_string()
        );
    }

    #[test]
    fn test_runtime_status_marks_unreachable_devtools_port() {
        let guard = EnvGuard::new(&["HOME"]);
        let home = env::temp_dir().join(format!(
            "runtime-unreachable-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        let runtime_profile = "legacy-unreachable-runtime";
        let user_data_dir = runtime_profile_user_data_dir(runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.to_string(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: std::process::id(),
            process_identity: None,
            headed: true,
            launch_mode: "automation".to_string(),
            devtools_port: Some(9),
            ws_url: Some("ws://127.0.0.1:9/devtools/browser/stale".to_string()),
            launch_record: None,
        })
        .unwrap();

        let status = runtime_status_with_user_data_dir(runtime_profile, None).unwrap();

        assert!(!status.browser_alive);
        assert_eq!(status.devtools_port, None);
        assert!(!status.devtools_reachable);
        assert!(status.targets.is_empty());

        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn test_runtime_status_rejects_reused_unrelated_pid() {
        let guard = EnvGuard::new(&["HOME"]);
        let fixture_id = format!(
            "pid-reuse-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let home = env::temp_dir().join(&fixture_id);
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());

        let runtime_profile = "reused-unrelated-pid";
        let user_data_dir = runtime_profile_user_data_dir(runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.to_string(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: std::process::id(),
            process_identity: None,
            headed: true,
            launch_mode: "automation".to_string(),
            devtools_port: Some(9),
            ws_url: Some("ws://127.0.0.1:9/devtools/browser/stale".to_string()),
            launch_record: None,
        })
        .unwrap();

        let status = runtime_status_with_user_data_dir(runtime_profile, None).unwrap();

        assert!(
            !status.browser_alive,
            "a live unrelated process that reused a stale browser PID must not own the runtime"
        );
        assert!(crate::process_identity::process_exists(std::process::id()));

        fs::remove_dir_all(&home).unwrap();
    }

    #[tokio::test]
    async fn test_runtime_close_refuses_to_signal_reused_unrelated_pid() {
        let guard = EnvGuard::new(&["HOME"]);
        let home = env::temp_dir().join(format!(
            "runtime-close-pid-reuse-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        let runtime_profile = "runtime-close-reused-pid";
        let user_data_dir = runtime_profile_user_data_dir(runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        let pid = std::process::id();
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.to_string(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: pid,
            process_identity: Some(RecordedProcessIdentity {
                pid,
                start_token: "linux:stale-boot:1".to_string(),
                executable_path: Some("/opt/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            }),
            headed: true,
            launch_mode: "manual".to_string(),
            devtools_port: None,
            ws_url: None,
            launch_record: None,
        })
        .unwrap();

        let outcome = crate::native::action_runtime::runtime::terminate_runtime_browser(
            Some(runtime_profile.to_string()),
            pid,
        )
        .await;

        assert!(!outcome.polite_close_attempted);
        assert!(!outcome.force_kill_attempted);
        assert!(outcome
            .errors
            .iter()
            .any(|error| error.contains("Refusing to signal PID")));
        assert!(
            read_runtime_state(runtime_profile).unwrap().is_some(),
            "a refused termination must preserve durable ownership evidence"
        );
        assert!(crate::process_identity::process_exists(pid));
        fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_runtime_status_keeps_matching_manual_no_cdp_process_live() {
        let guard = EnvGuard::new(&["HOME"]);
        let home = env::temp_dir().join(format!(
            "manual-process-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        let runtime_profile = "matching-manual-runtime";
        let user_data_dir = runtime_profile_user_data_dir(runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        let executable = home.join("manual-chrome");
        fs::copy("/bin/sleep", &executable).unwrap();
        let mut child = std::process::Command::new(&executable)
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        let process_identity = crate::process_identity::capture_process_identity(
            pid,
            Some(&executable),
            Some("chrome"),
        )
        .unwrap();
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.to_string(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: pid,
            process_identity: Some(process_identity),
            headed: true,
            launch_mode: "manual".to_string(),
            devtools_port: None,
            ws_url: None,
            launch_record: Some(RuntimeLaunchRecord {
                browser_family: Some("chrome".to_string()),
                ..RuntimeLaunchRecord::default()
            }),
        })
        .unwrap();

        let status = runtime_status_with_user_data_dir(runtime_profile, None).unwrap();

        assert!(status.browser_alive);
        assert_eq!(status.devtools_port, None);
        assert!(!status.devtools_reachable);
        let _ = child.kill();
        let _ = child.wait();
        fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_legacy_browser_with_unrelated_devtools_endpoint_stays_ambiguous() {
        let guard = EnvGuard::new(&["HOME"]);
        let home = env::temp_dir().join(format!(
            "legacy-browser-endpoint-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        let executable = home.join("legacy-chrome");
        fs::copy("/bin/sleep", &executable).unwrap();
        let mut child = std::process::Command::new(&executable)
            .arg("30")
            .spawn()
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let runtime_profile = "legacy-browser-endpoint";
        let user_data_dir = runtime_profile_user_data_dir(runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.to_string(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: child.id(),
            process_identity: None,
            headed: true,
            launch_mode: "automation".to_string(),
            devtools_port: Some(port),
            ws_url: None,
            launch_record: None,
        })
        .unwrap();

        let status = runtime_status_with_user_data_dir(runtime_profile, None).unwrap();

        assert!(!status.browser_alive);
        assert!(!status.devtools_reachable);
        assert_eq!(status.devtools_port, None);
        assert!(status.targets.is_empty());
        drop(listener);
        let _ = child.kill();
        let _ = child.wait();
        fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_profile_consistent_legacy_browser_retains_compatibility() {
        let guard = EnvGuard::new(&["HOME"]);
        let home = env::temp_dir().join(format!(
            "legacy-profile-consistent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        let runtime_profile = "legacy-profile-consistent";
        let user_data_dir = runtime_profile_user_data_dir(runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let executable = home.join("legacy-chrome");
        fs::copy("/bin/bash", &executable).unwrap();
        let mut child = std::process::Command::new(&executable)
            .arg("-c")
            .arg("sleep 30 & wait")
            .arg("legacy-chrome")
            .arg(format!("--user-data-dir={}", user_data_dir.display()))
            .arg(format!("--remote-debugging-port={port}"))
            .spawn()
            .unwrap();
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request);
                let body = r#"[{"id":"page-legacy","type":"page","title":"Legacy","url":"https://example.test"}]"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.to_string(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: child.id(),
            process_identity: None,
            headed: true,
            launch_mode: "automation".to_string(),
            devtools_port: Some(port),
            ws_url: None,
            launch_record: None,
        })
        .unwrap();

        let status = runtime_status_with_user_data_dir(runtime_profile, None).unwrap();

        assert!(status.browser_alive);
        assert!(status.devtools_reachable);
        assert_eq!(status.devtools_port, Some(port));
        assert_eq!(status.targets[0].id, "page-legacy");
        server.join().unwrap();
        let _ = child.kill();
        let _ = child.wait();
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn test_list_runtime_profiles_merges_config_and_disk() {
        let disk_profile = format!(
            "testdisk{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let configured_profile = format!("{}cfg", disk_profile);
        let disk_root = runtime_profile_root(&disk_profile).unwrap();
        fs::create_dir_all(&disk_root).unwrap();

        let configured_user_data_dir =
            env::temp_dir().join(format!("{}-user-data", configured_profile));
        let items = list_runtime_profiles(
            &[(
                configured_profile.clone(),
                Some(configured_user_data_dir.clone()),
            )],
            Some(&configured_profile),
        )
        .unwrap();

        let configured = items
            .iter()
            .find(|item| item.runtime_profile == configured_profile)
            .unwrap();
        assert!(configured.configured);
        assert!(configured.default);
        assert_eq!(
            configured.user_data_dir,
            configured_user_data_dir.display().to_string()
        );

        let disk = items
            .iter()
            .find(|item| item.runtime_profile == disk_profile)
            .unwrap();
        assert!(!disk.configured);

        let _ = fs::remove_dir_all(&disk_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_list_manual_runtime_browsers_keeps_non_cdp_browser_visible() {
        let guard = EnvGuard::new(&["HOME"]);
        let home = env::temp_dir().join(format!(
            "manual-inventory-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        let runtime_profile = format!(
            "testmanual{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let user_data_dir = runtime_profile_user_data_dir(&runtime_profile).unwrap();
        fs::create_dir_all(&user_data_dir).unwrap();
        let executable = home.join("inventory-chrome");
        fs::copy("/bin/sleep", &executable).unwrap();
        let mut child = std::process::Command::new(&executable)
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        write_runtime_state(&RuntimeState {
            runtime_profile: runtime_profile.clone(),
            user_data_dir: user_data_dir.display().to_string(),
            browser_pid: pid,
            process_identity: crate::process_identity::capture_process_identity(
                pid,
                Some(&executable),
                Some("chrome"),
            ),
            headed: true,
            launch_mode: "manual_detached_login".to_string(),
            devtools_port: None,
            ws_url: None,
            launch_record: Some(RuntimeLaunchRecord {
                target_url: Some("https://x.com/".to_string()),
                browser_family: Some("chrome".to_string()),
                browser_build: Some("stock_chrome".to_string()),
                display: Some(":11".to_string()),
                started_at: Some("2026-07-25T12:00:00Z".to_string()),
                last_observed_at: Some("2026-07-25T12:01:00Z".to_string()),
                ..RuntimeLaunchRecord::default()
            }),
        })
        .unwrap();

        let browsers = list_manual_runtime_browsers().unwrap();
        let browser = browsers
            .iter()
            .find(|browser| browser.runtime_profile == runtime_profile)
            .expect("manual runtime browser should remain in the operator inventory");

        assert_eq!(browser.pid, pid);
        assert_eq!(browser.target_url.as_deref(), Some("https://x.com/"));
        assert_eq!(browser.display.as_deref(), Some(":11"));
        assert!(!browser.automation_available);
        assert_eq!(
            browser.next_safe_action,
            "finish_login_then_close_or_relaunch_attachable"
        );

        let _ = child.kill();
        let _ = child.wait();
        let _ = clear_runtime_state(&runtime_profile);
        let _ = fs::remove_dir_all(home);
    }
}
