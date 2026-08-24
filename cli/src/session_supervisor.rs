//! Linux user-service supervision for named daemon sessions.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::connection::{
    daemon_ready, generate_daemon_auth_token, get_socket_dir, register_runtime_lane_config,
    send_command, write_daemon_auth_token,
};
use crate::validation::is_valid_session_name;

const SUPERVISOR_SCHEMA_VERSION: &str = "agent-browser.session-supervisor.v1";
const SUPERVISOR_UNIT_NAME: &str = "agent-browser-runtime-host.service";
const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(5);
const SYSTEMCTL_POLL_INTERVAL: Duration = Duration::from_millis(25);
static SYSTEMCTL_OUTPUT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSupervisorManifest {
    pub schema_version: String,
    pub session: String,
    pub executable_path: String,
    pub executable_sha256: String,
    pub stream_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_config_path: Option<String>,
    pub provenance: SessionSupervisorProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSupervisorProvenance {
    pub package_version: String,
    pub installed_at: String,
    pub installed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSupervisorPaths {
    pub manifest_dir: PathBuf,
    pub unit_dir: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SystemdUnitObservation {
    load_state: String,
    active_state: String,
    sub_state: String,
    result: String,
    restart_count: u64,
    main_pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionSupervisorStatus {
    schema_version: &'static str,
    session: String,
    unit: String,
    state: &'static str,
    ready: bool,
    manifest_path: String,
    manifest: Option<SessionSupervisorManifest>,
    load_state: Option<String>,
    active_state: Option<String>,
    sub_state: Option<String>,
    result: Option<String>,
    restart_count: Option<u64>,
    main_pid: Option<u32>,
    stream_port: Option<u16>,
    published_stream_port: Option<u16>,
    stream_reachable: bool,
    executable_matches: bool,
    issues: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallRequest {
    session: String,
    stream_port: u16,
    runtime_profile: Option<String>,
    service_config_path: Option<String>,
}

pub(crate) fn validate_manifest(manifest: &SessionSupervisorManifest) -> Result<(), String> {
    if manifest.schema_version != SUPERVISOR_SCHEMA_VERSION {
        return Err(format!(
            "invalid_manifest: expected schema version {SUPERVISOR_SCHEMA_VERSION}"
        ));
    }
    if !is_valid_session_name(&manifest.session) {
        return Err("invalid_manifest: invalid session name".to_string());
    }
    if manifest.stream_port < 1024 {
        return Err("invalid_manifest: stream port must be between 1024 and 65535".to_string());
    }
    validate_absolute_text_path(&manifest.executable_path, "executable path")?;
    if manifest.executable_sha256.len() != 64
        || !manifest
            .executable_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("invalid_manifest: executable SHA-256 is invalid".to_string());
    }
    if let Some(runtime_profile) = manifest.runtime_profile.as_deref() {
        if !is_valid_session_name(runtime_profile) {
            return Err("invalid_manifest: runtime profile is invalid".to_string());
        }
    }
    if let Some(config_path) = manifest.service_config_path.as_deref() {
        validate_absolute_text_path(config_path, "service config path")?;
    }
    if manifest.provenance.package_version.trim().is_empty()
        || manifest.provenance.installed_at.trim().is_empty()
        || manifest.provenance.installed_by.trim().is_empty()
    {
        return Err("invalid_manifest: provenance is incomplete".to_string());
    }
    Ok(())
}

pub(crate) fn render_unit(executable_path: &str) -> Result<String, String> {
    validate_absolute_text_path(executable_path, "executable path")?;
    let executable = systemd_quote(executable_path);
    Ok(format!(
        "[Unit]\nDescription=Agent Browser user runtime host\nAfter=default.target\nStartLimitIntervalSec=60\nStartLimitBurst=5\n\n[Service]\nType=simple\nExecStart={executable} session supervisor run-host\nRestart=on-failure\nRestartSec=2\nNoNewPrivileges=true\nPrivateTmp=true\n\n[Install]\nWantedBy=default.target\n"
    ))
}

fn render_legacy_forwarder_unit(executable_path: &str) -> Result<String, String> {
    validate_absolute_text_path(executable_path, "executable path")?;
    let executable = systemd_quote(executable_path);
    Ok(format!(
        "[Unit]\nDescription=Agent Browser runtime lane compatibility adapter %i\nRequires={SUPERVISOR_UNIT_NAME}\nAfter={SUPERVISOR_UNIT_NAME}\n\n[Service]\nType=oneshot\nExecStart={executable} session supervisor run-lane %i\nRemainAfterExit=yes\nNoNewPrivileges=true\nPrivateTmp=true\n\n[Install]\nWantedBy=default.target\n"
    ))
}

pub(crate) fn manifest_path(paths: &SessionSupervisorPaths, session: &str) -> PathBuf {
    paths.manifest_dir.join(format!("{session}.json"))
}

pub(crate) fn unit_path(paths: &SessionSupervisorPaths) -> PathBuf {
    paths.unit_dir.join(SUPERVISOR_UNIT_NAME)
}

fn legacy_unit_path(paths: &SessionSupervisorPaths) -> PathBuf {
    paths.unit_dir.join("agent-browser-session@.service")
}

fn validate_absolute_text_path(value: &str, label: &str) -> Result<(), String> {
    if value.contains(['\n', '\r', '\0']) || !Path::new(value).is_absolute() {
        return Err(format!(
            "invalid_manifest: {label} must be an absolute path"
        ));
    }
    Ok(())
}

fn systemd_quote(value: &str) -> String {
    let escaped = value
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{escaped}\"")
}

pub(crate) fn run_session_supervisor(args: &[String], json_mode: bool) -> i32 {
    match run_session_supervisor_inner(args) {
        Ok(report) => {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string(&json!({ "success": true, "data": report }))
                        .unwrap_or_else(|_| "{\"success\":false}".to_string())
                );
            } else {
                print_text_report(&report);
            }
            0
        }
        Err(error) => {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "success": false,
                        "type": "session_supervisor_error",
                        "error": error,
                    }))
                    .unwrap_or_else(|_| "{\"success\":false}".to_string())
                );
            } else {
                eprintln!("{error}");
            }
            1
        }
    }
}

fn run_session_supervisor_inner(args: &[String]) -> Result<Value, String> {
    if !cfg!(target_os = "linux") {
        return Err("unsupported: named session supervision currently requires Linux".to_string());
    }
    let base = args
        .windows(2)
        .position(|window| window[0] == "session" && window[1] == "supervisor")
        .ok_or_else(supervisor_usage)?;
    let operation = args
        .get(base + 2)
        .map(String::as_str)
        .ok_or_else(supervisor_usage)?;
    let paths = default_paths()?;
    if operation == "run-host" {
        return run_supervised_host(&paths);
    }
    let session = args
        .get(base + 3)
        .map(String::as_str)
        .ok_or_else(supervisor_usage)?;
    if !is_valid_session_name(session) {
        return Err(crate::validation::session_name_error(session));
    }
    match operation {
        "install" => {
            let request = parse_install_request(args, base, session)?;
            install_supervisor(&paths, &request)
        }
        "status" => status_report(&paths, session).map(|report| json!(report)),
        "remove" => remove_supervisor(&paths, session),
        "run-lane" => run_supervised_lane(&paths, session),
        _ => Err(supervisor_usage()),
    }
}

fn supervisor_usage() -> String {
    "Usage: agent-browser session supervisor <install|status|remove> <session> [--stream-port <port>] [--runtime-profile <id>] [--config <path>]".to_string()
}

fn parse_install_request(
    args: &[String],
    base: usize,
    session: &str,
) -> Result<InstallRequest, String> {
    let mut stream_port = None;
    let mut runtime_profile = None;
    let mut service_config_path = None;
    let mut index = base + 4;
    while index < args.len() {
        let option = args[index].as_str();
        let destination = match option {
            "--stream-port" => &mut stream_port,
            "--runtime-profile" => &mut runtime_profile,
            "--config" => &mut service_config_path,
            "--json" => {
                index += 1;
                continue;
            }
            unknown => return Err(format!("unknown session supervisor option: {unknown}")),
        };
        if destination.is_some() {
            return Err(format!("session supervisor option repeated: {option}"));
        }
        index += 1;
        *destination = Some(
            args.get(index)
                .filter(|value| !value.starts_with("--"))
                .cloned()
                .ok_or_else(|| format!("{option} requires a value"))?,
        );
        index += 1;
    }
    let stream_port = stream_port
        .ok_or_else(|| "session supervisor install requires --stream-port <port>".to_string())?
        .parse::<u16>()
        .map_err(|_| "invalid stream port".to_string())?;
    let request = InstallRequest {
        session: session.to_string(),
        stream_port,
        runtime_profile,
        service_config_path,
    };
    validate_install_request(&request)?;
    Ok(request)
}

fn validate_install_request(request: &InstallRequest) -> Result<(), String> {
    if !is_valid_session_name(&request.session) {
        return Err(crate::validation::session_name_error(&request.session));
    }
    if request.stream_port < 1024 {
        return Err("stream port must be between 1024 and 65535".to_string());
    }
    if let Some(profile) = request.runtime_profile.as_deref() {
        if !is_valid_session_name(profile) {
            return Err("runtime profile must use a safe identifier".to_string());
        }
    }
    if let Some(config) = request.service_config_path.as_deref() {
        validate_absolute_text_path(config, "service config path")?;
    }
    Ok(())
}

fn default_paths() -> Result<SessionSupervisorPaths, String> {
    if let Some(root) = env::var_os("AGENT_BROWSER_SESSION_SUPERVISOR_ROOT") {
        let root = PathBuf::from(root);
        return Ok(SessionSupervisorPaths {
            manifest_dir: root.join("manifests"),
            unit_dir: root.join("units"),
        });
    }
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(SessionSupervisorPaths {
        manifest_dir: home
            .join(".config")
            .join("agent-browser")
            .join("session-supervisors"),
        unit_dir: home.join(".config").join("systemd").join("user"),
    })
}

fn install_supervisor(
    paths: &SessionSupervisorPaths,
    request: &InstallRequest,
) -> Result<Value, String> {
    validate_install_request(request)?;
    let existing_manifests = supervised_manifests(paths)?;
    if let Some(conflict) = existing_manifests.iter().find(|manifest| {
        manifest.session != request.session && manifest.stream_port == request.stream_port
    }) {
        return Err(format!(
            "port_conflict: stream port {} is already assigned to supervised lane {}",
            request.stream_port, conflict.session
        ));
    }
    let mut legacy_sessions = existing_manifests
        .iter()
        .map(|manifest| manifest.session.clone())
        .collect::<Vec<_>>();
    legacy_sessions.push(request.session.clone());
    legacy_sessions.sort();
    legacy_sessions.dedup();
    for session in legacy_sessions {
        if legacy_supervisor_is_active(paths, &session)? {
            return Err(format!(
                "legacy_supervisor_transfer_required: {session} is still owned by an active per-session unit; use the runtime convergence transaction before enabling the shared host"
            ));
        }
    }
    let invoking_executable = env::current_exe()
        .map_err(|error| format!("could not resolve current executable: {error}"))?
        .canonicalize()
        .map_err(|error| format!("could not canonicalize current executable: {error}"))?;
    let accepted_selected_executable = accepted_workstation_selected_executable();
    let executable = accepted_selected_executable
        .clone()
        .unwrap_or(invoking_executable);
    let executable_path = executable.display().to_string();
    let executable_sha256 = sha256_file(&executable)?;
    if let Some(stale) = existing_manifests.iter().find(|manifest| {
        manifest.executable_path != executable_path
            || manifest.executable_sha256 != executable_sha256
    }) {
        if accepted_selected_executable.is_none() {
            return Err(format!(
                "runtime_host_convergence_required: supervised lane {} records another executable generation",
                stale.session
            ));
        }
    }
    let manifest = SessionSupervisorManifest {
        schema_version: SUPERVISOR_SCHEMA_VERSION.to_string(),
        session: request.session.clone(),
        executable_path,
        executable_sha256,
        stream_port: request.stream_port,
        runtime_profile: request.runtime_profile.clone(),
        service_config_path: request.service_config_path.clone(),
        provenance: SessionSupervisorProvenance {
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            installed_at: current_timestamp(),
            installed_by: "agent-browser session supervisor install".to_string(),
        },
    };
    validate_manifest(&manifest)?;
    write_manifest_and_unit(paths, &manifest)?;
    run_systemctl(&["--user", "daemon-reload"])?;
    let unit = unit_name(&request.session);
    run_systemctl(&["--user", "enable", "--now", &unit])?;
    admit_supervised_lane(&manifest)?;
    Ok(json!({
        "schemaVersion": SUPERVISOR_SCHEMA_VERSION,
        "state": "installed",
        "session": request.session,
        "unit": unit,
        "manifestPath": manifest_path(paths, &request.session),
        "streamPort": request.stream_port,
        "browserLaunched": false,
    }))
}

/// Rebinds durable lane manifests after the workstation candidate is accepted.
/// The selected transaction already owns every runtime lane, so the previous
/// user unit is stopped and left enabled for the next boot without starting a
/// second host beside the accepted candidate process.
pub(crate) fn rebind_supervisors_after_accepted_upgrade(
    executable: &Path,
) -> Result<Value, String> {
    let paths = default_paths()?;
    let manifests = supervised_manifests(&paths)?;
    if manifests.is_empty() {
        return Ok(json!({
            "schemaVersion": SUPERVISOR_SCHEMA_VERSION,
            "state": "not_configured",
            "reboundCount": 0,
            "browserLaunched": false,
        }));
    }
    let executable = executable
        .canonicalize()
        .map_err(|error| format!("could not canonicalize selected executable: {error}"))?;
    let executable_path = executable.display().to_string();
    let executable_sha256 = sha256_file(&executable)?;
    let rebound = manifests
        .into_iter()
        .map(|mut manifest| {
            manifest.executable_path = executable_path.clone();
            manifest.executable_sha256 = executable_sha256.clone();
            manifest.provenance = SessionSupervisorProvenance {
                package_version: env!("CARGO_PKG_VERSION").to_string(),
                installed_at: current_timestamp(),
                installed_by: "accepted workstation upgrade".to_string(),
            };
            validate_manifest(&manifest)?;
            Ok(manifest)
        })
        .collect::<Result<Vec<_>, String>>()?;

    let unit = unit_name(&rebound[0].session);
    run_systemctl(&["--user", "stop", &unit])?;
    for manifest in &rebound {
        write_manifest_and_unit(&paths, manifest)?;
    }
    run_systemctl(&["--user", "daemon-reload"])?;
    run_systemctl(&["--user", "enable", &unit])?;
    Ok(json!({
        "schemaVersion": SUPERVISOR_SCHEMA_VERSION,
        "state": "rebound",
        "reboundCount": rebound.len(),
        "executablePath": executable_path,
        "browserLaunched": false,
        "unitStarted": false,
    }))
}

/// A completed workstation transaction may rebind stale lane manifests to the
/// exact selected executable. A clean candidate rollback may use the preserved
/// selected generation only while its authenticated ingress receipt is still
/// current. Every other pre-acceptance state fails closed.
fn accepted_workstation_selected_executable() -> Option<PathBuf> {
    let Ok(status) = crate::workstation_install::workstation_upgrade_status_json() else {
        return None;
    };
    let root = env::var_os("AGENT_BROWSER_WORKSTATION_ROOT")
        .map(PathBuf::from)
        .or_else(dirs::home_dir);
    let root = root?;
    let selected = status.get("selectedGenerationId").and_then(Value::as_str)?;
    let selected_executable = root
        .join(".local/lib/agent-browser/generations")
        .join(selected)
        .join("bin/agent-browser");
    let Ok(selected_executable) = selected_executable.canonicalize() else {
        return None;
    };
    workstation_status_authorizes_supervisor_rebind(&status).then_some(selected_executable)
}

fn workstation_status_authorizes_supervisor_rebind(status: &Value) -> bool {
    if status.get("admissionDraining").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    let accepted = status.get("ready").and_then(Value::as_bool) == Some(true)
        && status
            .pointer("/readiness/runtimeConvergenceReady")
            .and_then(Value::as_bool)
            == Some(true);
    if accepted {
        return true;
    }

    let Some(selected) = status.get("selectedGenerationId").and_then(Value::as_str) else {
        return false;
    };
    status
        .pointer("/latestTransaction/state")
        .and_then(Value::as_str)
        == Some("failed_preserved_old_generation")
        && status
            .pointer("/latestTransaction/terminalResult")
            .and_then(Value::as_str)
            == Some("old_generation_preserved")
        && status
            .pointer("/latestTransaction/oldGenerationId")
            .and_then(Value::as_str)
            == Some(selected)
        && status
            .pointer("/readiness/payloadReady")
            .and_then(Value::as_bool)
            == Some(true)
        && status
            .pointer("/readiness/selectedGenerationReady")
            .and_then(Value::as_bool)
            == Some(true)
        && status
            .pointer("/dashboardIngress/dashboardIngressReady")
            .and_then(Value::as_bool)
            == Some(true)
        && status
            .pointer("/dashboardIngress/operatorJourneyReady")
            .and_then(Value::as_bool)
            == Some(true)
        && status
            .pointer("/dashboardIngress/selectedBackend/generationId")
            .and_then(Value::as_str)
            == Some(selected)
        && status
            .pointer("/dashboardIngress/presentationReceipt/state")
            .and_then(Value::as_str)
            == Some("ready")
        && status
            .pointer("/dashboardIngress/presentationReceipt/coordinatorGeneration")
            .and_then(Value::as_str)
            == Some(selected)
}

fn legacy_supervisor_is_active(
    paths: &SessionSupervisorPaths,
    session: &str,
) -> Result<bool, String> {
    let template = legacy_unit_path(paths);
    if !template.is_file() {
        return Ok(false);
    }
    if fs::read_to_string(&template)
        .ok()
        .is_some_and(|unit| unit.contains("session supervisor run-lane %i"))
    {
        return Ok(false);
    }
    let unit = format!("agent-browser-session@{session}.service");
    let output = run_systemctl_output(&["--user", "is-active", &unit])?;
    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "active")
}

fn write_manifest_and_unit(
    paths: &SessionSupervisorPaths,
    manifest: &SessionSupervisorManifest,
) -> Result<(), String> {
    validate_manifest(manifest)?;
    fs::create_dir_all(&paths.manifest_dir)
        .map_err(|error| format!("could not create supervisor manifest directory: {error}"))?;
    fs::create_dir_all(&paths.unit_dir)
        .map_err(|error| format!("could not create systemd user directory: {error}"))?;
    set_directory_private(&paths.manifest_dir)?;
    let manifest_bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("could not encode supervisor manifest: {error}"))?;
    replace_file(
        &manifest_path(paths, &manifest.session),
        &[manifest_bytes.as_slice(), b"\n"].concat(),
        true,
    )?;
    replace_file(
        &unit_path(paths),
        render_unit(&manifest.executable_path)?.as_bytes(),
        false,
    )?;
    replace_file(
        &legacy_unit_path(paths),
        render_legacy_forwarder_unit(&manifest.executable_path)?.as_bytes(),
        false,
    )
}

fn remove_supervisor(paths: &SessionSupervisorPaths, session: &str) -> Result<Value, String> {
    let path = manifest_path(paths, session);
    let manifest = load_manifest(&path)?;
    if manifest.session != session {
        return Err("invalid_manifest: session does not match requested removal".to_string());
    }
    let unit = unit_name(session);
    env::set_var(crate::runtime_host::RUNTIME_HOST_ENV, "1");
    if daemon_ready(session) {
        let _ = send_command(
            json!({
                "id": format!("supervisor-remove-{session}"),
                "action": "close",
            }),
            session,
        );
    }
    let legacy_unit = format!("agent-browser-session@{session}.service");
    let _ = run_systemctl(&["--user", "disable", "--now", &legacy_unit]);
    fs::remove_file(&path)
        .map_err(|error| format!("could not remove supervisor manifest: {error}"))?;
    let remaining = supervised_manifests(paths)?;
    if remaining.is_empty() {
        run_systemctl(&["--user", "disable", "--now", &unit])?;
    }
    run_systemctl(&["--user", "daemon-reload"])?;
    Ok(json!({
        "schemaVersion": SUPERVISOR_SCHEMA_VERSION,
        "state": "removed",
        "session": session,
        "unit": unit,
        "preserved": ["runtime_profiles", "browser_storage", "service_state", "unrelated_units"],
    }))
}

fn run_supervised_host(paths: &SessionSupervisorPaths) -> Result<Value, String> {
    let manifests = supervised_manifests(paths)?;
    let manifest = manifests
        .first()
        .ok_or("runtime_host_supervisor_empty: no lane manifests are installed")?;
    for candidate in &manifests {
        verify_manifest_executable(candidate)?;
        let listener =
            TcpListener::bind(("127.0.0.1", candidate.stream_port)).map_err(|error| {
                format!(
                    "port_conflict: loopback port {} for lane {} is unavailable: {error}",
                    candidate.stream_port, candidate.session
                )
            })?;
        drop(listener);
    }
    env::set_var(crate::runtime_host::RUNTIME_HOST_ENV, "1");
    let token = generate_daemon_auth_token()?;
    write_daemon_auth_token(&manifest.session, &token)?;
    for (name, value) in supervised_daemon_environment(manifest, &token) {
        env::set_var(name, value);
    }
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("could not start daemon runtime: {error}"))?;
    runtime.block_on(crate::native::daemon::run_daemon(&manifest.session));
    Ok(json!({
        "schemaVersion": SUPERVISOR_SCHEMA_VERSION,
        "state": "stopped",
        "unit": SUPERVISOR_UNIT_NAME,
    }))
}

fn run_supervised_lane(paths: &SessionSupervisorPaths, session: &str) -> Result<Value, String> {
    let manifest = load_manifest(&manifest_path(paths, session))?;
    verify_manifest_executable(&manifest)?;
    admit_supervised_lane(&manifest)?;
    Ok(json!({
        "schemaVersion": SUPERVISOR_SCHEMA_VERSION,
        "state": "admitted",
        "session": session,
        "unit": SUPERVISOR_UNIT_NAME,
    }))
}

fn supervised_manifests(
    paths: &SessionSupervisorPaths,
) -> Result<Vec<SessionSupervisorManifest>, String> {
    if !paths.manifest_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut manifests = fs::read_dir(&paths.manifest_dir)
        .map_err(|error| format!("could not read supervisor manifests: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .map(|entry| load_manifest(&entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    manifests.sort_by(|left, right| left.session.cmp(&right.session));
    let mut ports = std::collections::BTreeSet::new();
    for manifest in &manifests {
        if !ports.insert(manifest.stream_port) {
            return Err(format!(
                "port_conflict: supervised stream port {} is assigned more than once",
                manifest.stream_port
            ));
        }
    }
    Ok(manifests)
}

fn runtime_lane_config_from_manifest(
    manifest: &SessionSupervisorManifest,
) -> crate::runtime_host::RuntimeLaneConfig {
    crate::runtime_host::RuntimeLaneConfig {
        runtime_profile: manifest.runtime_profile.clone(),
        stream_port: Some(manifest.stream_port),
        ..Default::default()
    }
}

pub(crate) fn runtime_host_supervised_lane_configs(
) -> Result<Vec<(String, crate::runtime_host::RuntimeLaneConfig)>, String> {
    let paths = default_paths()?;
    Ok(supervised_manifests(&paths)?
        .into_iter()
        .map(|manifest| {
            let config = runtime_lane_config_from_manifest(&manifest);
            (manifest.session, config)
        })
        .collect())
}

fn admit_supervised_lane(manifest: &SessionSupervisorManifest) -> Result<(), String> {
    env::set_var(crate::runtime_host::RUNTIME_HOST_ENV, "1");
    register_runtime_lane_config(
        &manifest.session,
        runtime_lane_config_from_manifest(manifest),
    );
    let started = Instant::now();
    while !daemon_ready(&manifest.session) {
        if started.elapsed() >= SYSTEMCTL_TIMEOUT {
            return Err(
                "runtime_host_start_timeout: supervised host did not become ready".to_string(),
            );
        }
        thread::sleep(SYSTEMCTL_POLL_INTERVAL);
    }
    let response = send_command(
        json!({
            "id": format!("supervisor-admit-{}", manifest.session),
            "action": "worker_status",
        }),
        &manifest.session,
    )?;
    if response.success {
        Ok(())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "runtime_host_lane_admission_failed".to_string()))
    }
}

fn supervised_daemon_environment(
    manifest: &SessionSupervisorManifest,
    token: &str,
) -> Vec<(&'static str, String)> {
    let mut values = vec![
        ("AGENT_BROWSER_DAEMON_AUTH_TOKEN", token.to_string()),
        (crate::runtime_host::RUNTIME_HOST_ENV, "1".to_string()),
        ("AGENT_BROWSER_SESSION", manifest.session.clone()),
        (
            "AGENT_BROWSER_STREAM_PORT",
            manifest.stream_port.to_string(),
        ),
        ("AGENT_BROWSER_STREAM_PORT_STRICT", "1".to_string()),
    ];
    if let Some(profile) = manifest.runtime_profile.as_ref() {
        values.push(("AGENT_BROWSER_RUNTIME_PROFILE", profile.clone()));
    }
    if let Some(config) = manifest.service_config_path.as_ref() {
        values.push(("AGENT_BROWSER_CONFIG", config.clone()));
    }
    values
}

fn status_report(
    paths: &SessionSupervisorPaths,
    session: &str,
) -> Result<SessionSupervisorStatus, String> {
    let path = manifest_path(paths, session);
    let manifest_result = load_manifest(&path);
    let systemd = observe_systemd_unit(session);
    let published_stream_port =
        fs::read_to_string(get_socket_dir().join(format!("{session}.stream")))
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok());
    let stream_port = manifest_result
        .as_ref()
        .ok()
        .map(|manifest| manifest.stream_port);
    let stream_reachable = stream_port.is_some_and(loopback_port_reachable);
    let executable_matches = manifest_result
        .as_ref()
        .is_ok_and(|manifest| verify_manifest_executable(manifest).is_ok());
    Ok(classify_status(
        session,
        &path,
        manifest_result,
        systemd,
        published_stream_port,
        stream_reachable,
        executable_matches,
    ))
}

pub(crate) fn session_supervisor_health_json() -> Value {
    let paths = match default_paths() {
        Ok(paths) => paths,
        Err(error) => {
            return json!({
                "schemaVersion": "agent-browser.session-supervisor-health.v1",
                "ready": false,
                "count": 0,
                "degradedCount": 0,
                "sessions": [],
                "issues": [issue("supervisor_inventory_unavailable", &error)],
            });
        }
    };
    session_supervisor_health_for_paths(&paths)
}

fn session_supervisor_health_for_paths(paths: &SessionSupervisorPaths) -> Value {
    if !paths.manifest_dir.is_dir() {
        return json!({
            "schemaVersion": "agent-browser.session-supervisor-health.v1",
            "ready": true,
            "count": 0,
            "degradedCount": 0,
            "sessions": [],
            "issues": [],
        });
    }
    let mut names = fs::read_dir(&paths.manifest_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|value| value.to_str()) == Some("json"))
                .then(|| path.file_stem()?.to_str().map(str::to_string))
                .flatten()
        })
        .filter(|session| is_valid_session_name(session))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    let sessions = names
        .iter()
        .map(|session| status_report(paths, session).map(|status| json!(status)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| {
            vec![json!({
                "state": "unavailable",
                "ready": false,
                "issues": [issue("supervisor_inventory_unavailable", &error)],
            })]
        });
    let degraded_count = sessions
        .iter()
        .filter(|status| status.get("ready").and_then(Value::as_bool) != Some(true))
        .count();
    let issues = sessions
        .iter()
        .flat_map(|status| {
            status
                .get("issues")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    json!({
        "schemaVersion": "agent-browser.session-supervisor-health.v1",
        "ready": degraded_count == 0,
        "count": sessions.len(),
        "degradedCount": degraded_count,
        "sessions": sessions,
        "issues": issues,
    })
}

fn classify_status(
    session: &str,
    path: &Path,
    manifest_result: Result<SessionSupervisorManifest, String>,
    systemd: Result<SystemdUnitObservation, String>,
    published_stream_port: Option<u16>,
    stream_reachable: bool,
    executable_matches: bool,
) -> SessionSupervisorStatus {
    let manifest = manifest_result.as_ref().ok().cloned();
    let stream_port = manifest.as_ref().map(|value| value.stream_port);
    let mut issues = Vec::new();
    let (state, ready) = if let Err(error) = manifest_result.as_ref() {
        issues.push(issue("invalid_manifest", error));
        ("invalid_manifest", false)
    } else if !executable_matches {
        issues.push(issue(
            "executable_drift",
            "The supervisor manifest executable no longer matches the running command.",
        ));
        ("executable_drift", false)
    } else if stream_reachable && published_stream_port != stream_port {
        issues.push(issue(
            "port_conflict",
            "The fixed loopback port is reachable without matching session stream metadata.",
        ));
        ("port_conflict", false)
    } else if systemd
        .as_ref()
        .is_ok_and(|unit| unit.result == "start-limit-hit")
    {
        issues.push(issue(
            "restart_exhausted",
            "The user service exhausted its bounded restart rate.",
        ));
        ("restart_exhausted", false)
    } else if systemd
        .as_ref()
        .is_ok_and(|unit| unit.active_state == "active")
        && stream_reachable
        && published_stream_port == stream_port
    {
        ("ready", true)
    } else if systemd
        .as_ref()
        .is_ok_and(|unit| unit.active_state == "activating")
    {
        issues.push(issue_with_severity(
            "supervisor_starting",
            "error",
            "The shared runtime host supervisor is activating but not ready.",
        ));
        ("starting", false)
    } else {
        if let Err(error) = systemd.as_ref() {
            issues.push(issue("supervisor_unavailable", error));
        } else {
            let has_live_expectation = stream_reachable
                || systemd.as_ref().is_ok_and(|unit| {
                    unit.active_state == "active" || unit.main_pid.is_some_and(|pid| pid > 0)
                });
            issues.push(issue_with_severity(
                "supervisor_stopped",
                if has_live_expectation {
                    "error"
                } else {
                    "warning"
                },
                "The shared runtime host supervisor is not active and ready.",
            ));
        }
        ("stopped", false)
    };
    let unit = systemd.as_ref().ok();
    SessionSupervisorStatus {
        schema_version: SUPERVISOR_SCHEMA_VERSION,
        session: session.to_string(),
        unit: unit_name(session),
        state,
        ready,
        manifest_path: path.display().to_string(),
        manifest,
        load_state: unit.map(|value| value.load_state.clone()),
        active_state: unit.map(|value| value.active_state.clone()),
        sub_state: unit.map(|value| value.sub_state.clone()),
        result: unit.map(|value| value.result.clone()),
        restart_count: unit.map(|value| value.restart_count),
        main_pid: unit.and_then(|value| value.main_pid),
        stream_port,
        published_stream_port,
        stream_reachable,
        executable_matches,
        issues,
    }
}

fn issue(code: &str, message: &str) -> Value {
    issue_with_severity(code, "error", message)
}

fn issue_with_severity(code: &str, severity: &str, message: &str) -> Value {
    json!({
        "code": code,
        "severity": severity,
        "message": message,
        "recommendedAction": "agent-browser session supervisor status",
    })
}

fn observe_systemd_unit(session: &str) -> Result<SystemdUnitObservation, String> {
    let unit = unit_name(session);
    let output = run_systemctl_output(&[
        "--user",
        "show",
        &unit,
        "--property=LoadState",
        "--property=ActiveState",
        "--property=SubState",
        "--property=Result",
        "--property=NRestarts",
        "--property=MainPID",
    ])?;
    if !output.status.success() {
        return Err(format!(
            "systemctl status unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let values = parse_systemd_properties(&String::from_utf8_lossy(&output.stdout));
    Ok(SystemdUnitObservation {
        load_state: values.get("LoadState").cloned().unwrap_or_default(),
        active_state: values.get("ActiveState").cloned().unwrap_or_default(),
        sub_state: values.get("SubState").cloned().unwrap_or_default(),
        result: values.get("Result").cloned().unwrap_or_default(),
        restart_count: values
            .get("NRestarts")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        main_pid: values
            .get("MainPID")
            .and_then(|value| value.parse().ok())
            .filter(|pid| *pid > 0),
    })
}

fn parse_systemd_properties(text: &str) -> std::collections::BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn run_systemctl(args: &[&str]) -> Result<(), String> {
    let output = run_systemctl_output(args)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "systemctl failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn run_systemctl_output(args: &[&str]) -> Result<Output, String> {
    let command = env::var("AGENT_BROWSER_SESSION_SUPERVISOR_SYSTEMCTL")
        .unwrap_or_else(|_| "systemctl".to_string());
    let mut command = Command::new(command);
    command.args(args);
    run_command_with_timeout(command, SYSTEMCTL_TIMEOUT)
}

fn run_command_with_timeout(mut command: Command, timeout: Duration) -> Result<Output, String> {
    let output_id = SYSTEMCTL_OUTPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let output_prefix = env::temp_dir().join(format!(
        "agent-browser-session-supervisor-command-{}-{output_id}",
        std::process::id()
    ));
    let stdout_path = output_prefix.with_extension("stdout");
    let stderr_path = output_prefix.with_extension("stderr");
    let stdout_file = fs::File::create(&stdout_path)
        .map_err(|error| format!("could not prepare systemctl stdout: {error}"))?;
    let stderr_file = match fs::File::create(&stderr_path) {
        Ok(file) => file,
        Err(error) => {
            let _ = fs::remove_file(&stdout_path);
            return Err(format!("could not prepare systemctl stderr: {error}"));
        }
    };
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Err(format!("could not execute systemctl: {error}"));
        }
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = fs::read(&stdout_path).unwrap_or_default();
                let stderr = fs::read(&stderr_path).unwrap_or_default();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return Err(format!(
                    "systemctl timed out after {} ms",
                    timeout.as_millis()
                ));
            }
            Ok(None) => thread::sleep(SYSTEMCTL_POLL_INTERVAL),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return Err(format!("could not wait for systemctl: {error}"));
            }
        }
    }
}

fn load_manifest(path: &Path) -> Result<SessionSupervisorManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("manifest unavailable: {error}"))?;
    let manifest: SessionSupervisorManifest =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid_manifest: {error}"))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn verify_manifest_executable(manifest: &SessionSupervisorManifest) -> Result<(), String> {
    let current = env::current_exe()
        .map_err(|error| format!("executable_drift: {error}"))?
        .canonicalize()
        .map_err(|error| format!("executable_drift: {error}"))?;
    let intended = Path::new(&manifest.executable_path)
        .canonicalize()
        .map_err(|error| format!("executable_drift: {error}"))?;
    if current != intended || sha256_file(&current)? != manifest.executable_sha256 {
        return Err("executable_drift: current executable differs from manifest".to_string());
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("could not open executable for hashing: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("could not hash executable: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn replace_file(path: &Path, bytes: &[u8], private: bool) -> Result<(), String> {
    let staged = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&staged, bytes)
        .map_err(|error| format!("could not stage {}: {error}", path.display()))?;
    if private {
        set_file_private(&staged)?;
    }
    fs::rename(&staged, path)
        .map_err(|error| format!("could not replace {}: {error}", path.display()))
}

fn set_directory_private(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("could not secure {}: {error}", path.display()))?;
    }
    Ok(())
}

fn set_file_private(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("could not secure {}: {error}", path.display()))?;
    }
    Ok(())
}

fn unit_name(_session: &str) -> String {
    SUPERVISOR_UNIT_NAME.to_string()
}

fn loopback_port_reachable(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(200),
    )
    .is_ok()
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        })
}

fn print_text_report(report: &Value) {
    if let Some(state) = report.get("state").and_then(Value::as_str) {
        println!("Session supervisor: {state}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(report).unwrap_or_else(|_| report.to_string())
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn manifest() -> SessionSupervisorManifest {
        SessionSupervisorManifest {
            schema_version: SUPERVISOR_SCHEMA_VERSION.to_string(),
            session: "messages-v4".to_string(),
            executable_path: "/home/test/.local/bin/agent-browser".to_string(),
            executable_sha256: "a".repeat(64),
            stream_port: 39716,
            runtime_profile: Some("messages-v4".to_string()),
            service_config_path: Some("/home/test/.agent-browser/config.json".to_string()),
            provenance: SessionSupervisorProvenance {
                package_version: env!("CARGO_PKG_VERSION").to_string(),
                installed_at: "2026-08-11T12:00:00Z".to_string(),
                installed_by: "agent-browser session supervisor install".to_string(),
            },
        }
    }

    #[test]
    fn accepted_converged_workstation_authorizes_selected_manifest_rebinding() {
        let ready = json!({
            "ready": true,
            "admissionDraining": false,
            "readiness": {"runtimeConvergenceReady": true}
        });
        assert!(workstation_status_authorizes_supervisor_rebind(&ready));

        let mut draining = ready.clone();
        draining["admissionDraining"] = json!(true);
        assert!(!workstation_status_authorizes_supervisor_rebind(&draining));

        let preserved = json!({
            "ready": false,
            "selectedGenerationId": "generation-old",
            "admissionDraining": false,
            "readiness": {
                "payloadReady": true,
                "selectedGenerationReady": true,
                "runtimeConvergenceReady": false
            },
            "latestTransaction": {
                "state": "failed_preserved_old_generation",
                "terminalResult": "old_generation_preserved",
                "oldGenerationId": "generation-old"
            },
            "dashboardIngress": {
                "dashboardIngressReady": true,
                "operatorJourneyReady": true,
                "selectedBackend": {"generationId": "generation-old"},
                "presentationReceipt": {
                    "state": "ready",
                    "coordinatorGeneration": "generation-old"
                }
            }
        });
        assert!(workstation_status_authorizes_supervisor_rebind(&preserved));

        let mut mismatched_receipt = preserved.clone();
        mismatched_receipt["dashboardIngress"]["presentationReceipt"]["coordinatorGeneration"] =
            json!("generation-candidate");
        assert!(!workstation_status_authorizes_supervisor_rebind(
            &mismatched_receipt
        ));
    }

    #[test]
    fn manifest_validation_is_fail_closed() {
        assert!(validate_manifest(&manifest()).is_ok());
        for invalid in [
            {
                let mut value = manifest();
                value.schema_version = "wrong".to_string();
                value
            },
            {
                let mut value = manifest();
                value.session = "../escape".to_string();
                value
            },
            {
                let mut value = manifest();
                value.stream_port = 80;
                value
            },
            {
                let mut value = manifest();
                value.executable_path = "relative/agent-browser".to_string();
                value
            },
            {
                let mut value = manifest();
                value.executable_sha256 = "not-a-sha".to_string();
                value
            },
        ] {
            assert!(validate_manifest(&invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn quiescent_stopped_supervisor_is_advisory_but_live_expectation_is_blocking() {
        let path = Path::new("/tmp/messages-v4.json");
        let quiescent = classify_status(
            "messages-v4",
            path,
            Ok(manifest()),
            Ok(SystemdUnitObservation {
                load_state: "loaded".to_string(),
                active_state: "inactive".to_string(),
                sub_state: "dead".to_string(),
                result: "success".to_string(),
                restart_count: 0,
                main_pid: None,
            }),
            None,
            false,
            true,
        );
        assert_eq!(quiescent.state, "stopped");
        assert_eq!(quiescent.issues[0]["code"], "supervisor_stopped");
        assert_eq!(quiescent.issues[0]["severity"], "warning");

        let live_expected = classify_status(
            "messages-v4",
            path,
            Ok(manifest()),
            Ok(SystemdUnitObservation {
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                result: "success".to_string(),
                restart_count: 0,
                main_pid: Some(4242),
            }),
            Some(39716),
            false,
            true,
        );
        assert_eq!(live_expected.state, "stopped");
        assert_eq!(live_expected.issues[0]["severity"], "error");
    }

    #[test]
    fn unit_is_one_runtime_host_service_without_shell_interpolation() {
        let unit = render_unit("/home/test/Agent Browser/bin/agent-browser").expect("unit");
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("StartLimitBurst=5"));
        assert!(unit.contains("StartLimitIntervalSec=60"));
        assert!(unit.contains(
            "ExecStart=\"/home/test/Agent Browser/bin/agent-browser\" session supervisor run-host"
        ));
        assert!(!unit.contains("/bin/sh"));
        assert!(!unit.contains("sh -c"));
        assert!(!unit.contains("AGENT_BROWSER_HEADED"));
        let forwarder = render_legacy_forwarder_unit("/home/test/Agent Browser/bin/agent-browser")
            .expect("forwarder");
        assert!(forwarder.contains("Requires=agent-browser-runtime-host.service"));
        assert!(forwarder.contains("session supervisor run-lane %i"));
        assert!(forwarder.contains("[Install]\nWantedBy=default.target"));
        assert!(!forwarder.contains("session supervisor run %i"));
    }

    #[test]
    fn paths_are_exactly_scoped_to_the_requested_session() {
        let paths = SessionSupervisorPaths {
            manifest_dir: PathBuf::from("/tmp/manifests"),
            unit_dir: PathBuf::from("/tmp/units"),
        };
        assert_eq!(
            manifest_path(&paths, "messages-v4"),
            PathBuf::from("/tmp/manifests/messages-v4.json")
        );
        assert_eq!(
            unit_path(&paths),
            PathBuf::from("/tmp/units/agent-browser-runtime-host.service")
        );
        assert_eq!(
            legacy_unit_path(&paths),
            PathBuf::from("/tmp/units/agent-browser-session@.service")
        );
    }

    #[test]
    fn manifest_and_unit_writes_are_private_and_atomic() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-supervisor-write-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = SessionSupervisorPaths {
            manifest_dir: root.join("manifests"),
            unit_dir: root.join("units"),
        };
        write_manifest_and_unit(&paths, &manifest()).expect("write supervisor files");
        let decoded = load_manifest(&manifest_path(&paths, "messages-v4")).expect("manifest");
        assert_eq!(decoded, manifest());
        assert!(fs::read_to_string(unit_path(&paths))
            .expect("unit")
            .contains("session supervisor run-host"));
        assert!(fs::read_to_string(legacy_unit_path(&paths))
            .expect("legacy forwarder")
            .contains("session supervisor run-lane %i"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(manifest_path(&paths, "messages-v4"))
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&paths.manifest_dir)
                    .expect("directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn host_inventory_preloads_distinct_fixed_port_lanes() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-supervisor-host-inventory-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = SessionSupervisorPaths {
            manifest_dir: root.join("manifests"),
            unit_dir: root.join("units"),
        };
        let alpha = manifest();
        let mut beta = manifest();
        beta.session = "messages-v5".to_string();
        beta.stream_port = 39_717;
        beta.runtime_profile = Some("messages-v5".to_string());
        write_manifest_and_unit(&paths, &alpha).unwrap();
        write_manifest_and_unit(&paths, &beta).unwrap();

        let manifests = supervised_manifests(&paths).unwrap();
        assert_eq!(
            manifests
                .iter()
                .map(|manifest| (manifest.session.as_str(), manifest.stream_port))
                .collect::<Vec<_>>(),
            vec![("messages-v4", 39_716), ("messages-v5", 39_717)]
        );
        let config = runtime_lane_config_from_manifest(&manifests[1]);
        assert_eq!(config.runtime_profile.as_deref(), Some("messages-v5"));
        assert_eq!(config.stream_port, Some(39_717));

        beta.stream_port = 39_716;
        write_manifest_and_unit(&paths, &beta).unwrap();
        assert!(supervised_manifests(&paths)
            .unwrap_err()
            .contains("assigned more than once"));

        let _ = fs::remove_dir_all(root);
    }

    fn active_unit() -> SystemdUnitObservation {
        SystemdUnitObservation {
            load_state: "loaded".to_string(),
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            result: "success".to_string(),
            restart_count: 2,
            main_pid: Some(4242),
        }
    }

    #[test]
    fn status_classifies_ready_drift_conflict_and_restart_exhaustion() {
        let path = Path::new("/tmp/messages-v4.json");
        let ready = classify_status(
            "messages-v4",
            path,
            Ok(manifest()),
            Ok(active_unit()),
            Some(39716),
            true,
            true,
        );
        assert_eq!(ready.state, "ready");
        assert!(ready.ready);

        let drift = classify_status(
            "messages-v4",
            path,
            Ok(manifest()),
            Ok(active_unit()),
            Some(39716),
            true,
            false,
        );
        assert_eq!(drift.state, "executable_drift");

        let conflict = classify_status(
            "messages-v4",
            path,
            Ok(manifest()),
            Ok(active_unit()),
            None,
            true,
            true,
        );
        assert_eq!(conflict.state, "port_conflict");

        let mut exhausted = active_unit();
        exhausted.active_state = "failed".to_string();
        exhausted.result = "start-limit-hit".to_string();
        let exhausted = classify_status(
            "messages-v4",
            path,
            Ok(manifest()),
            Ok(exhausted),
            None,
            false,
            true,
        );
        assert_eq!(exhausted.state, "restart_exhausted");
    }

    #[test]
    fn supervised_daemon_environment_cannot_auto_launch_a_browser() {
        let environment = supervised_daemon_environment(&manifest(), "secret")
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(environment["AGENT_BROWSER_SESSION"], "messages-v4");
        assert_eq!(environment["AGENT_BROWSER_STREAM_PORT"], "39716");
        assert_eq!(environment["AGENT_BROWSER_STREAM_PORT_STRICT"], "1");
        assert_eq!(environment["AGENT_BROWSER_RUNTIME_PROFILE"], "messages-v4");
        let names = environment.keys().copied().collect::<BTreeSet<_>>();
        for forbidden in [
            "AGENT_BROWSER_HEADED",
            "AGENT_BROWSER_URL",
            "AGENT_BROWSER_AUTO_CONNECT",
            "AGENT_BROWSER_BROWSER_HOST",
        ] {
            assert!(!names.contains(forbidden));
        }
    }

    #[test]
    fn parser_requires_fixed_port_and_preserves_optional_identity() {
        let args = [
            "session",
            "supervisor",
            "install",
            "messages-v4",
            "--stream-port",
            "39716",
            "--runtime-profile",
            "messages-v4",
            "--config",
            "/tmp/service.json",
        ]
        .map(str::to_string);
        let request = parse_install_request(&args, 0, "messages-v4").expect("request");
        assert_eq!(request.stream_port, 39716);
        assert_eq!(request.runtime_profile.as_deref(), Some("messages-v4"));
        assert_eq!(
            request.service_config_path.as_deref(),
            Some("/tmp/service.json")
        );

        let missing = ["session", "supervisor", "install", "messages-v4"].map(str::to_string);
        assert!(parse_install_request(&missing, 0, "messages-v4").is_err());

        let unknown = [
            "session",
            "supervisor",
            "install",
            "messages-v4",
            "--stream-port",
            "39716",
            "--headed",
        ]
        .map(str::to_string);
        assert!(parse_install_request(&unknown, 0, "messages-v4").is_err());
    }

    #[test]
    fn empty_supervisor_inventory_is_ready_without_systemd_probe() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-supervisor-empty-{}",
            uuid::Uuid::new_v4()
        ));
        let health = session_supervisor_health_for_paths(&SessionSupervisorPaths {
            manifest_dir: root.join("manifests"),
            unit_dir: root.join("units"),
        });
        assert_eq!(health["ready"], true);
        assert_eq!(health["count"], 0);
        assert_eq!(health["sessions"], json!([]));
    }

    #[cfg(unix)]
    #[test]
    fn systemctl_probe_is_bounded_when_the_child_stalls() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10"]);
        let started = Instant::now();
        let error = run_command_with_timeout(command, Duration::from_millis(50))
            .expect_err("stalled systemctl probe must time out");
        assert!(error.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
