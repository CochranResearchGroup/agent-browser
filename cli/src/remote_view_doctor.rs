use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::color;
use crate::native::service_model::ServiceState;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorArgs {
    allow_shared_target: bool,
    session: Option<String>,
    runtime_profile: Option<String>,
    route_id: Option<String>,
}

const DOCTOR_JSON_COMMAND_TIMEOUT: Duration = Duration::from_secs(45);
const DOCTOR_TEXT_COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const DOCTOR_DISPLAY_ACCESS_TIMEOUT: Duration = Duration::from_secs(3);
const DOCTOR_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(25);
static DOCTOR_COMMAND_OUTPUT_COUNTER: AtomicU64 = AtomicU64::new(0);

enum DoctorCommandResult {
    Output(Output),
    Timeout { stdout: String, stderr: String },
    SpawnError(String),
}

/// Run the read-only remote-view doctor. This command inventories existing
/// install, Guacamole, XRDP, user, and route-display state before setup helpers
/// are allowed to suggest creating users or mutating configuration.
pub fn run_remote_view_doctor(raw_args: &[String], json_mode: bool) -> i32 {
    let args = match parse_doctor_args(raw_args) {
        Ok(args) => args,
        Err(error) => {
            if json_mode {
                println!(
                    "{}",
                    json!({
                        "success": false,
                        "type": "usage_error",
                        "code": "invalid_remote_view_doctor_scope",
                        "error": error,
                    })
                );
            } else {
                eprintln!("{} {error}", color::error_indicator());
            }
            return 2;
        }
    };
    let report = remote_view_doctor_report(&args);

    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| {
                r#"{"success":false,"error":"Failed to serialize remote-view doctor"}"#.to_string()
            })
        );
        return 0;
    }

    print_remote_view_doctor_report(&report);
    0
}

fn parse_doctor_args(raw_args: &[String]) -> Result<DoctorArgs, String> {
    let mut args = DoctorArgs {
        allow_shared_target: false,
        session: None,
        runtime_profile: None,
        route_id: None,
    };
    let mut index = raw_args
        .windows(2)
        .position(|window| window[0] == "doctor" && window[1] == "remote-view")
        .map(|index| index + 2)
        .ok_or_else(|| "Usage: agent-browser doctor remote-view [selectors]".to_string())?;
    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--json" => {}
            "--allow-shared-target" => args.allow_shared_target = true,
            option @ ("--session" | "--runtime-profile" | "--route-id") => {
                index += 1;
                let value = raw_args
                    .get(index)
                    .filter(|value| !value.starts_with("--"))
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| format!("{option} requires a value"))?;
                let destination = match option {
                    "--session" => &mut args.session,
                    "--runtime-profile" => &mut args.runtime_profile,
                    "--route-id" => &mut args.route_id,
                    _ => unreachable!(),
                };
                if destination.is_some() {
                    return Err(format!("{option} may be supplied only once"));
                }
                *destination = Some(value.to_string());
            }
            unknown if unknown.starts_with("--") => {
                return Err(format!("unknown remote-view doctor option: {unknown}"));
            }
            _ => {}
        }
        index += 1;
    }
    if let Some(session) = args.session.as_deref() {
        if !crate::validation::is_valid_session_name(session) {
            return Err(crate::validation::session_name_error(session));
        }
    }
    if let Some(profile) = args.runtime_profile.as_deref() {
        if !crate::validation::is_valid_session_name(profile) {
            return Err("runtime profile must use a safe identifier".to_string());
        }
    }
    if let Some(route_id) = args.route_id.as_deref() {
        if route_id.len() > 256
            || !route_id
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || "-_.:/".contains(value))
        {
            return Err("route id contains unsupported characters".to_string());
        }
    }
    Ok(args)
}

fn remote_view_doctor_report(args: &DoctorArgs) -> Value {
    let script_root = remote_view_script_root();
    let install = run_json_command(
        current_agent_browser_command(),
        &["install", "doctor", "--json"],
        None,
    );
    let route_pool_args = if args.allow_shared_target {
        vec![
            "smoke-rdp-guac-route-pool-readiness.js",
            "--report-only",
            "--allow-shared-target",
        ]
    } else {
        vec!["smoke-rdp-guac-route-pool-readiness.js", "--report-only"]
    };
    let route_pool = run_json_command(
        "node".to_string(),
        &script_args(&script_root, &route_pool_args),
        Some(script_root.clone()),
    );
    let rdp_gateway = run_json_command(
        "node".to_string(),
        &script_args(&script_root, &["smoke-rdp-gateway-readiness.js"]),
        Some(script_root.clone()),
    );
    let route_displays = run_json_command(
        "node".to_string(),
        &script_args(&script_root, &["inspect-rdp-route-displays.js"]),
        Some(script_root.clone()),
    );
    let xrdp = inspect_xrdp();
    let users = inspect_rdp_users();
    let privileges = inspect_privileges();
    let config = inspect_remote_view_config();
    let display_access = inspect_route_display_access(&route_displays);
    let viewer_prerequisites = inspect_viewer_prerequisites(&privileges);
    let many_to_many = many_to_many_status(
        &rdp_gateway,
        &route_pool,
        &route_displays,
        &display_access,
        &viewer_prerequisites,
    );
    let remote_control = remote_control_status(
        &install,
        &rdp_gateway,
        &route_pool,
        &route_displays,
        &viewer_prerequisites,
    );
    let dashboard_runtime = dashboard_runtime_from_install(&install);
    let runtime_inventory = runtime_inventory_from_install(&install);
    let runtime_convergence = runtime_convergence_from_install(&install);
    let next_action = recommend_next_action(RecommendationContext {
        install: &install,
        rdp_gateway: &rdp_gateway,
        route_pool: &route_pool,
        route_displays: &route_displays,
        display_access: &display_access,
        viewer_prerequisites: &viewer_prerequisites,
        users: &users,
        privileges: &privileges,
    });
    let next_command = recommend_next_command(&next_action);
    let drift = drift_findings(&users, &config);
    let issues = remote_view_issues(RemoteViewIssueContext {
        install: &install,
        rdp_gateway: &rdp_gateway,
        route_pool: &route_pool,
        route_displays: &route_displays,
        display_access: &display_access,
        viewer_prerequisites: &viewer_prerequisites,
        users: &users,
        privileges: &privileges,
        next_action: &next_action,
    });
    let requested_scope = requested_scope_report(
        args,
        read_persisted_service_state(),
        &rdp_gateway,
        &privileges,
    );
    let global_advisories = requested_scope_global_advisories(&requested_scope, &issues);

    json!({
        "success": true,
        "data": {
            "status": many_to_many["status"].clone(),
            "install": install,
            "runtime": inspect_runtime(),
            "network": inspect_network(),
            "rdpGateway": rdp_gateway,
            "rdpHost": {
                "xrdp": xrdp,
                "users": users,
                "displayAccess": display_access,
                "privileges": privileges,
            },
            "guacamole": {
                "routePool": route_pool,
                "routeDisplays": route_displays,
            },
            "remoteControl": remote_control,
            "dashboardRuntime": dashboard_runtime,
            "runtimeInventory": runtime_inventory,
            "runtimeConvergence": runtime_convergence,
            "manyToMany": many_to_many,
            "viewerPrerequisites": viewer_prerequisites,
            "config": config,
            "drift": drift,
            "issues": issues,
            "requestedScope": requested_scope,
            "globalAdvisories": global_advisories,
            "scriptRoot": script_root.display().to_string(),
            "stateSources": state_sources(),
            "nextAction": next_action,
            "nextCommand": next_command,
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequestedRouteSubject {
    route_id: String,
    route_aliases: Vec<String>,
    session: Option<String>,
    runtime_profile: Option<String>,
    browser_id: Option<String>,
    display_allocation_id: Option<String>,
    route_state: String,
    route_url_ready: bool,
    display_ready: bool,
    route_pool_ready: bool,
}

fn read_persisted_service_state() -> Result<ServiceState, String> {
    let path = crate::native::service_store::JsonServiceStateStore::default_path()?;
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|error| {
            format!(
                "invalid persisted service state {}: {error}",
                path.display()
            )
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ServiceState::default()),
        Err(error) => Err(format!(
            "could not read persisted service state {}: {error}",
            path.display()
        )),
    }
}

fn requested_route_subjects(state: &ServiceState) -> Vec<RequestedRouteSubject> {
    let mut subjects = Vec::new();
    for route in state.remote_view_routes.values() {
        let browser = route
            .browser_id
            .as_deref()
            .and_then(|browser_id| state.browsers.get(browser_id));
        let display = route
            .display_allocation_id
            .as_deref()
            .and_then(|display_id| state.display_allocations.get(display_id));
        let mut session = route
            .session_id
            .clone()
            .or_else(|| display.and_then(|value| value.owner_session_id.clone()));
        if session.is_none() {
            let active_sessions = browser
                .map(|value| value.active_session_ids.as_slice())
                .unwrap_or_default();
            if active_sessions.len() == 1 {
                session = active_sessions.first().cloned();
            }
        }
        let session_record = session
            .as_deref()
            .and_then(|session_id| state.sessions.get(session_id));
        let runtime_profile = display
            .and_then(|value| value.profile_id.clone())
            .or_else(|| browser.and_then(|value| value.profile_id.clone()))
            .or_else(|| session_record.and_then(|value| value.profile_id.clone()));
        let pool_entry = state.route_pool.values().find(|entry| {
            entry.route_id == route.id
                || route.connection_id.is_some()
                    && route.connection_id.as_deref() == entry.connection_id.as_deref()
        });
        let mut route_aliases = vec![route.id.clone()];
        if let Some(pool_route_id) = pool_entry.map(|entry| entry.route_id.clone()) {
            if !pool_route_id.is_empty() && !route_aliases.contains(&pool_route_id) {
                route_aliases.push(pool_route_id);
            }
        }
        subjects.push(RequestedRouteSubject {
            route_id: route.id.clone(),
            route_aliases,
            session,
            runtime_profile,
            browser_id: route.browser_id.clone(),
            display_allocation_id: route.display_allocation_id.clone(),
            route_state: route.state.clone(),
            route_url_ready: route.frame_url.is_some() || route.external_url.is_some(),
            display_ready: display.is_some_and(|value| value.state == "ready"),
            route_pool_ready: pool_entry.is_some_and(requested_route_pool_entry_ready),
        });
    }
    subjects.sort_by(|left, right| left.route_id.cmp(&right.route_id));
    subjects
}

fn requested_route_pool_entry_ready(entry: &crate::native::service_model::RoutePoolEntry) -> bool {
    entry.state == "ready"
        || matches!(entry.state.as_str(), "checked_out" | "occupied")
            && entry
                .readiness
                .as_ref()
                .and_then(|readiness| readiness.get("state"))
                .and_then(Value::as_str)
                == Some("ready")
}

fn requested_scope_report(
    args: &DoctorArgs,
    state: Result<ServiceState, String>,
    rdp_gateway: &Value,
    privileges: &Value,
) -> Value {
    let selectors = json!({
        "session": args.session,
        "runtimeProfile": args.runtime_profile,
        "routeId": args.route_id,
    });
    if args.session.is_none() && args.runtime_profile.is_none() && args.route_id.is_none() {
        return json!({
            "selectors": selectors,
            "status": "not_requested",
            "issues": [],
            "nextAction": "rerun_with_session_profile_or_route_selector",
            "subject": null,
        });
    }
    let state = match state {
        Ok(state) => state,
        Err(error) => {
            return json!({
                "selectors": selectors,
                "status": "unavailable",
                "issues": [requested_scope_issue("service_state_unavailable", &error)],
                "nextAction": "repair_service_state_readability",
                "subject": null,
            });
        }
    };
    let subjects = requested_route_subjects(&state);
    let matches_selector = |subject: &RequestedRouteSubject| {
        args.session
            .as_deref()
            .is_none_or(|value| subject.session.as_deref() == Some(value))
            && args
                .runtime_profile
                .as_deref()
                .is_none_or(|value| subject.runtime_profile.as_deref() == Some(value))
            && args.route_id.as_deref().is_none_or(|value| {
                subject
                    .route_aliases
                    .iter()
                    .any(|candidate| candidate == value)
            })
    };
    let matches = subjects
        .iter()
        .filter(|subject| matches_selector(subject))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return json!({
            "selectors": selectors,
            "status": "degraded",
            "issues": [requested_scope_issue(
                "ambiguous_requested_scope",
                "The selectors match multiple retained remote-view routes.",
            )],
            "nextAction": "add_a_route_id_selector",
            "candidateRouteIds": matches.iter().map(|value| value.route_id.as_str()).collect::<Vec<_>>(),
            "subject": null,
        });
    }
    let Some(subject) = matches.first().copied() else {
        let individually_available = [
            args.session.as_deref().map(|value| {
                subjects
                    .iter()
                    .any(|subject| subject.session.as_deref() == Some(value))
            }),
            args.runtime_profile.as_deref().map(|value| {
                subjects
                    .iter()
                    .any(|subject| subject.runtime_profile.as_deref() == Some(value))
            }),
            args.route_id.as_deref().map(|value| {
                subjects.iter().any(|subject| {
                    subject
                        .route_aliases
                        .iter()
                        .any(|candidate| candidate == value)
                })
            }),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let contradictory = individually_available.len() > 1
            && individually_available.iter().all(|available| *available);
        return json!({
            "selectors": selectors,
            "status": if contradictory { "degraded" } else { "unavailable" },
            "issues": [requested_scope_issue(
                if contradictory { "contradictory_requested_scope" } else { "requested_scope_not_found" },
                if contradictory {
                    "The selectors resolve to different retained remote-view subjects."
                } else {
                    "No retained remote-view route matches the requested selectors."
                },
            )],
            "nextAction": if contradictory { "correct_the_conflicting_selectors" } else { "inspect_retained_remote_view_routes" },
            "subject": null,
        });
    };
    let mut issues = Vec::new();
    if subject.route_state != "ready" {
        issues.push(requested_scope_issue(
            "requested_route_not_ready",
            "The retained remote-view route is not ready.",
        ));
    }
    if !subject.route_url_ready {
        issues.push(requested_scope_issue(
            "requested_route_url_missing",
            "The retained remote-view route has no operator URL.",
        ));
    }
    if !subject.display_ready {
        issues.push(requested_scope_issue(
            "requested_display_not_ready",
            "The requested route display allocation is not ready.",
        ));
    }
    if !subject.route_pool_ready {
        issues.push(requested_scope_issue(
            "requested_route_pool_entry_not_ready",
            "The requested route has no ready route-pool entry.",
        ));
    }
    if !nested_bool(rdp_gateway, &["data", "success"]) {
        issues.push(requested_scope_issue(
            "requested_route_gateway_not_ready",
            "The RDP gateway is not ready for the requested route.",
        ));
    }
    if privileges.get("ready").and_then(Value::as_bool) != Some(true) {
        issues.push(requested_scope_issue(
            "requested_route_helper_not_ready",
            "The privileged helper contract is not ready for the requested route.",
        ));
    }
    json!({
        "selectors": selectors,
        "status": if issues.is_empty() { "ready" } else { "degraded" },
        "issues": issues,
        "nextAction": if issues.is_empty() { "no_action" } else { "repair_requested_route" },
        "subject": {
            "routeId": subject.route_id,
            "routeAliases": subject.route_aliases,
            "session": subject.session,
            "runtimeProfile": subject.runtime_profile,
            "browserId": subject.browser_id,
            "displayAllocationId": subject.display_allocation_id,
            "routeState": subject.route_state,
            "routeUrlReady": subject.route_url_ready,
            "displayReady": subject.display_ready,
            "routePoolReady": subject.route_pool_ready,
        },
    })
}

fn requested_scope_issue(code: &str, message: &str) -> Value {
    json!({
        "code": code,
        "severity": "warning",
        "message": message,
    })
}

fn requested_scope_global_advisories(requested_scope: &Value, issues: &[Value]) -> Vec<Value> {
    if requested_scope["status"] == "not_requested" {
        Vec::new()
    } else {
        issues.to_vec()
    }
}

const REMOTE_VIEW_HELPER_SCRIPTS: [&str; 3] = [
    "smoke-rdp-guac-route-pool-readiness.js",
    "smoke-rdp-gateway-readiness.js",
    "inspect-rdp-route-displays.js",
];

fn remote_view_script_root() -> PathBuf {
    find_remote_view_script_root().unwrap_or_else(|| {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("scripts")
    })
}

fn find_remote_view_script_root() -> Option<PathBuf> {
    if let Some(configured) = env::var_os("AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT") {
        if let Some(root) = normalize_script_root_candidate(Path::new(&configured)) {
            return Some(root);
        }
    }

    if let Ok(cwd) = env::current_dir() {
        if let Some(root) = find_script_root_in_ancestors(&cwd) {
            return Some(root);
        }
    }

    if let Ok(exe) = env::current_exe() {
        let exe = fs::canonicalize(&exe).unwrap_or(exe);
        if let Some(parent) = exe.parent() {
            if let Some(root) = find_script_root_in_ancestors(parent) {
                return Some(root);
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        if let Some(root) = find_installed_support_script_root(&home, env!("CARGO_PKG_VERSION")) {
            return Some(root);
        }
        let pnpm_global_package =
            home.join(".local/share/pnpm/global/5/node_modules/agent-browser");
        if let Some(root) = normalize_script_root_candidate(&pnpm_global_package) {
            return Some(root);
        }
    }

    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        let manifest_path = Path::new(manifest_dir);
        let repo_candidate = manifest_path.parent().unwrap_or(manifest_path);
        if let Some(root) = normalize_script_root_candidate(repo_candidate) {
            return Some(root);
        }
    }

    None
}

fn find_installed_support_script_root(home: &Path, version: &str) -> Option<PathBuf> {
    let install_root = home.join(".local/lib/agent-browser");
    [
        install_root.join("current/support"),
        install_root.join(version),
        install_root.join(version).join("support"),
    ]
    .into_iter()
    .find_map(|candidate| normalize_script_root_candidate(&candidate))
}

fn find_script_root_in_ancestors(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if let Some(root) = normalize_script_root_candidate(ancestor) {
            return Some(root);
        }
    }
    None
}

fn normalize_script_root_candidate(candidate: &Path) -> Option<PathBuf> {
    if has_remote_view_helper_scripts(candidate) {
        return Some(candidate.to_path_buf());
    }
    let scripts = candidate.join("scripts");
    if has_remote_view_helper_scripts(&scripts) {
        return Some(scripts);
    }
    None
}

fn has_remote_view_helper_scripts(candidate: &Path) -> bool {
    REMOTE_VIEW_HELPER_SCRIPTS
        .iter()
        .all(|script| candidate.join(script).is_file())
}

fn script_args(script_root: &Path, args: &[&str]) -> Vec<String> {
    args.iter()
        .enumerate()
        .map(|(index, arg)| {
            if index == 0 {
                script_root.join(arg).display().to_string()
            } else {
                (*arg).to_string()
            }
        })
        .collect()
}

fn current_agent_browser_command() -> String {
    env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "agent-browser".to_string())
}

fn run_json_command<S: AsRef<str>>(command: String, args: &[S], cwd: Option<PathBuf>) -> Value {
    let mut child = Command::new(&command);
    let args = args
        .iter()
        .map(|arg| arg.as_ref().to_string())
        .collect::<Vec<_>>();
    child.args(&args);
    if let Some(cwd) = cwd {
        child.current_dir(cwd);
    }
    match run_command_with_timeout(child, DOCTOR_JSON_COMMAND_TIMEOUT) {
        DoctorCommandResult::Output(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let parsed = serde_json::from_str::<Value>(stdout.trim()).ok();
            json!({
                "available": true,
                "success": output.status.success(),
                "timedOut": false,
                "exitCode": output.status.code(),
                "command": format!("{} {}", command, args.join(" ")),
                "data": parsed,
                "stderr": redact_text(stderr.trim()),
            })
        }
        DoctorCommandResult::Timeout { stdout, stderr } => {
            let parsed = serde_json::from_str::<Value>(stdout.trim()).ok();
            json!({
                "available": true,
                "success": false,
                "timedOut": true,
                "exitCode": null,
                "command": format!("{} {}", command, args.join(" ")),
                "data": parsed,
                "stderr": redact_text(if stderr.trim().is_empty() {
                    "doctor subcommand timed out"
                } else {
                    stderr.trim()
                }),
            })
        }
        DoctorCommandResult::SpawnError(error) => json!({
            "available": false,
            "success": false,
            "timedOut": false,
            "exitCode": null,
            "command": format!("{} {}", command, args.join(" ")),
            "data": null,
            "stderr": redact_text(error),
        }),
    }
}

fn inspect_runtime() -> Value {
    let service_process = run_text_command("pgrep", &["-af", "agent-browser.*service"]);
    json!({
        "serviceProcess": service_process,
        "note": "remote-view doctor is read-only and does not start the service daemon",
    })
}

fn inspect_network() -> Value {
    let containers = [
        "agent-browser-guacamole",
        "agent-browser-guacd",
        "agent-browser-guacamole-postgres",
    ]
    .iter()
    .map(|name| {
        let running = docker_container_running(name);
        json!({
            "name": name,
            "running": running,
        })
    })
    .collect::<Vec<_>>();
    json!({
        "dockerContainers": containers,
    })
}

fn docker_container_running(name: &str) -> Option<bool> {
    let mut command = Command::new("docker");
    command.args(["inspect", "-f", "{{.State.Running}}", name]);
    match run_command_with_timeout(command, DOCTOR_TEXT_COMMAND_TIMEOUT) {
        DoctorCommandResult::Output(output) => {
            if !output.status.success() {
                return Some(false);
            }
            Some(String::from_utf8_lossy(&output.stdout).trim() == "true")
        }
        DoctorCommandResult::Timeout { .. } => None,
        DoctorCommandResult::SpawnError(_) => None,
    }
}

fn inspect_xrdp() -> Value {
    let path = PathBuf::from("/etc/xrdp/sesman.ini");
    let text = fs::read_to_string(&path).ok();
    let policy = text
        .as_deref()
        .and_then(|text| parse_ini_value(text, "Sessions", "Policy"));
    let max_sessions = text
        .as_deref()
        .and_then(|text| parse_ini_value(text, "Sessions", "MaxSessions"));
    let display_offset = text
        .as_deref()
        .and_then(|text| parse_ini_value(text, "Sessions", "X11DisplayOffset"))
        .or_else(|| {
            text.as_deref()
                .and_then(|text| parse_ini_value(text, "X11rdp", "X11DisplayOffset"))
        })
        .or_else(|| {
            text.as_deref()
                .and_then(|text| parse_ini_value(text, "Xvnc", "X11DisplayOffset"))
        })
        .or_else(|| {
            text.as_deref()
                .and_then(|text| parse_ini_value(text, "Xorg", "X11DisplayOffset"))
        });

    json!({
        "path": path.display().to_string(),
        "exists": path.exists(),
        "policy": policy,
        "maxSessions": max_sessions,
        "x11DisplayOffset": display_offset,
    })
}

fn inspect_rdp_users() -> Value {
    let users = [
        "agent-browser-rdp",
        "agent-browser-rdp-a",
        "agent-browser-rdp-b",
    ]
    .iter()
    .map(|user| inspect_user(user))
    .collect::<Vec<_>>();
    let existing_count = users
        .iter()
        .filter(|user| user.get("exists").and_then(Value::as_bool) == Some(true))
        .count();
    json!({
        "expectedExistingUser": "agent-browser-rdp",
        "routeSpecificUsers": ["agent-browser-rdp-a", "agent-browser-rdp-b"],
        "entries": users,
        "existingCount": existing_count,
    })
}

fn inspect_user(user: &str) -> Value {
    let output = Command::new("getent").args(["passwd", user]).output();
    match output {
        Ok(output) if output.status.success() => {
            let row = String::from_utf8_lossy(&output.stdout);
            let parts = row.trim().split(':').collect::<Vec<_>>();
            json!({
                "user": user,
                "exists": true,
                "uid": parts.get(2).and_then(|value| value.parse::<u64>().ok()),
                "gid": parts.get(3).and_then(|value| value.parse::<u64>().ok()),
                "home": parts.get(5).copied(),
                "shell": parts.get(6).copied(),
            })
        }
        Ok(_) => json!({
            "user": user,
            "exists": false,
        }),
        Err(error) => json!({
            "user": user,
            "exists": false,
            "error": error.to_string(),
        }),
    }
}

fn inspect_privileges() -> Value {
    let group_name =
        env::var("AGENT_BROWSER_PRIVILEGED_GROUP").unwrap_or_else(|_| "agent-browser".to_string());
    let helper_path = env::var("AGENT_BROWSER_PRIVILEGED_HELPER").unwrap_or_else(|_| {
        "/usr/local/libexec/agent-browser/agent-browser-privileged-helper".to_string()
    });
    let sudoers_path = env::var("AGENT_BROWSER_PRIVILEGED_SUDOERS")
        .unwrap_or_else(|_| "/etc/sudoers.d/agent-browser".to_string());
    let current_user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let group_exists = Command::new("getent")
        .args(["group", &group_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    let user_groups = Command::new("id").arg("-nG").output().ok().map(|output| {
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    });
    let user_in_group = user_groups
        .as_ref()
        .map(|groups| groups.iter().any(|group| group == &group_name))
        .unwrap_or(false);
    let helper_exists = Path::new(&helper_path).exists();
    let sudoers_exists = Path::new(&sudoers_path).exists();
    let helper_check = run_text_command("sudo", &["-n", &helper_path, "check"]);
    let helper_status = helper_status_output(run_text_command(
        "sudo",
        &["-n", &helper_path, "status-json"],
    ));
    let helper_desktop_session = remote_view_helper_desktop_session_status(&helper_path);
    let helper_contract = crate::remote_view_helper_contract::helper_contract_report(
        &helper_path,
        &helper_check,
        &helper_status,
        sudoers_exists,
    );
    let ready = group_exists
        && user_in_group
        && helper_exists
        && sudoers_exists
        && helper_desktop_session["ready"].as_bool() == Some(true)
        && helper_contract["ready"].as_bool() == Some(true);

    json!({
        "groupName": group_name,
        "currentUser": current_user,
        "groupExists": group_exists,
        "userInGroup": user_in_group,
        "helperPath": helper_path,
        "helperExists": helper_exists,
        "sudoersPath": sudoers_path,
        "sudoersExists": sudoers_exists,
        "helperCheck": helper_check,
        "helperStatus": helper_status,
        "helperContract": helper_contract,
        "helperDesktopSession": helper_desktop_session,
        "ready": ready,
    })
}

fn helper_status_output(mut report: Value) -> Value {
    let stdout = report
        .get("stdout")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();
    if !stdout.is_empty() {
        match serde_json::from_str::<Value>(stdout) {
            Ok(parsed) => {
                if let Some(object) = report.as_object_mut() {
                    object.insert("parsed".to_string(), parsed);
                }
            }
            Err(error) => {
                if let Some(object) = report.as_object_mut() {
                    object.insert("parseError".to_string(), json!(error.to_string()));
                }
            }
        }
    }
    report
}

fn remote_view_helper_status_contract_ready(report: &Value) -> bool {
    crate::remote_view_helper_contract::status_contract_ready(report)
}

fn remote_view_helper_desktop_session_status(helper_path: &str) -> Value {
    let path = Path::new(helper_path);
    if !path.exists() {
        return json!({
            "ready": false,
            "available": false,
            "state": "helper_missing",
            "terminalStartupDetected": null,
            "startsWindowManager": null,
            "keepsSessionAlive": null,
        });
    }
    let Ok(source) = fs::read_to_string(path) else {
        return json!({
            "ready": false,
            "available": false,
            "state": "helper_unreadable",
            "terminalStartupDetected": null,
            "startsWindowManager": null,
            "keepsSessionAlive": null,
        });
    };
    remote_view_helper_desktop_session_status_from_source(&source)
}

fn remote_view_helper_desktop_session_status_from_source(source: &str) -> Value {
    let template = route_xsession_template(source);
    let terminal_startup = template
        .as_deref()
        .is_some_and(route_xsession_starts_terminal);
    let starts_window_manager = template
        .as_deref()
        .is_some_and(|value| value.contains("openbox-session"));
    let keeps_session_alive = template
        .as_deref()
        .is_some_and(|value| value.contains("while true") && value.contains("sleep 3600"));
    let ready =
        template.is_some() && !terminal_startup && starts_window_manager && keeps_session_alive;
    json!({
        "ready": ready,
        "available": template.is_some(),
        "state": if ready {
            "browser_control_ready_template"
        } else if template.is_none() {
            "xsession_template_missing"
        } else if terminal_startup {
            "terminal_first_template"
        } else {
            "incomplete_template"
        },
        "terminalStartupDetected": terminal_startup,
        "startsWindowManager": starts_window_manager,
        "keepsSessionAlive": keeps_session_alive,
    })
}

fn route_xsession_template(source: &str) -> Option<String> {
    let marker_pos = source.find(".xsession")?;
    let after_marker = &source[marker_pos..];
    let heredoc_pos = after_marker.find("<<'EOF'")?;
    let content_start = marker_pos + heredoc_pos + "<<'EOF'".len();
    let content = source[content_start..].strip_prefix('\n')?;
    let content_end = content.find("\nEOF")?;
    Some(content[..content_end].to_string())
}

fn route_xsession_starts_terminal(template: &str) -> bool {
    let lowered = template.to_ascii_lowercase();
    [
        "xterm",
        "gnome-terminal",
        "xfce4-terminal",
        "konsole",
        "x-terminal-emulator",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn inspect_route_display_access(route_displays: &Value) -> Value {
    let mut entries = Vec::new();
    for label in ["A", "B"] {
        let Some(route) = route_displays
            .pointer(&format!("/data/routeSpecificUsers/{label}"))
            .or_else(|| route_displays.pointer(&format!("/data/routes/{label}")))
        else {
            continue;
        };
        let user = route
            .get("user")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let display_name = route
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if display_name.is_empty() {
            continue;
        }
        let probe = run_display_access_probe(display_name.as_str());
        entries.push(json!({
            "route": label,
            "routeUser": user,
            "displayName": display_name,
            "accessible": probe["success"].as_bool() == Some(true),
            "probe": probe,
        }));
    }
    let expected_count = if nested_bool(route_displays, &["data", "success"]) {
        2
    } else {
        0
    };
    let accessible_count = entries
        .iter()
        .filter(|entry| entry["accessible"].as_bool() == Some(true))
        .count();
    json!({
        "ready": expected_count > 0 && accessible_count >= expected_count,
        "expectedCount": expected_count,
        "accessibleCount": accessible_count,
        "entries": entries,
    })
}

fn inspect_viewer_prerequisites(privileges: &Value) -> Value {
    let client_a_env = env::var("AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE").ok();
    let client_b_env = env::var("AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE").ok();
    // A source-free workstation installs Chrome under agent-browser's managed
    // browser directory, which is intentionally not added to the shell PATH.
    let installed_chrome = crate::install::find_installed_chrome()
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string());
    let apparmor_profile_loaded = privileges
        .pointer("/helperStatus/parsed/managedChromeSandboxPolicy/loaded")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sandbox_policy =
        inspect_managed_chrome_sandbox_policy(installed_chrome.as_deref(), apparmor_profile_loaded);
    let client_a = executable_candidate(
        client_a_env.as_deref(),
        &[
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
        ],
    )
    .or_else(|| installed_chrome.clone());
    let client_b = executable_candidate(
        client_b_env.as_deref(),
        &[
            "brave-browser",
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
        ],
    )
    .or(installed_chrome)
    .or_else(|| client_a.clone());
    let identify = command_path("identify");
    let convert = command_path("convert");
    let tesseract = command_path("tesseract");
    let browser_launch_ready =
        client_a.is_some() && sandbox_policy["ready"].as_bool().unwrap_or(false);
    let ready = browser_launch_ready
        && client_b.is_some()
        && identify.is_some()
        && convert.is_some()
        && tesseract.is_some();

    json!({
        "ready": ready,
        "browserLaunchReady": browser_launch_ready,
        "clients": {
            "A": {
                "env": client_a_env,
                "path": client_a,
            },
            "B": {
                "env": client_b_env,
                "path": client_b,
            },
        },
        "tools": {
            "identify": identify,
            "convert": convert,
            "tesseract": tesseract,
        },
        "managedChromeSandboxPolicy": sandbox_policy,
    })
}

fn inspect_managed_chrome_sandbox_policy(
    installed_chrome: Option<&str>,
    apparmor_profile_loaded: bool,
) -> Value {
    let restriction_enabled =
        fs::read_to_string("/proc/sys/kernel/apparmor_restrict_unprivileged_userns")
            .ok()
            .is_some_and(|value| value.trim() == "1");
    let apparmor_enabled = fs::read_to_string("/sys/module/apparmor/parameters/enabled")
        .ok()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("y"));
    let apparmor_service_active = if apparmor_enabled {
        let mut command = Command::new("systemctl");
        command.args(["is-active", "--quiet", "apparmor"]);
        matches!(
            run_command_with_timeout(command, DOCTOR_TEXT_COMMAND_TIMEOUT),
            DoctorCommandResult::Output(output) if output.status.success()
        )
    } else {
        false
    };
    let policy_path = Path::new("/etc/apparmor.d/agent-browser-managed-chrome");
    let policy_content = fs::read_to_string(policy_path).ok();
    managed_chrome_sandbox_policy_status(
        installed_chrome,
        restriction_enabled,
        apparmor_enabled,
        apparmor_service_active,
        apparmor_profile_loaded,
        policy_path,
        policy_content.as_deref(),
    )
}

fn managed_chrome_sandbox_policy_status(
    installed_chrome: Option<&str>,
    restriction_enabled: bool,
    apparmor_enabled: bool,
    apparmor_service_active: bool,
    apparmor_profile_loaded: bool,
    policy_path: &Path,
    policy_content: Option<&str>,
) -> Value {
    let managed_browser_marker = "/.agent-browser/browsers/";
    let managed_root = installed_chrome
        .and_then(|path| path.split_once(managed_browser_marker))
        .map(|(home, _)| format!("{home}/.agent-browser/browsers/**/chrome"));
    let applicable = installed_chrome.is_some() && restriction_enabled && apparmor_enabled;
    let policy_present = policy_content.is_some();
    let policy_matches_managed_chrome = managed_root.as_ref().is_some_and(|root| {
        policy_content.is_some_and(|content| {
            content.contains("profile agent-browser-managed-chrome")
                && content.contains(&format!("\"{root}\""))
                && content.contains("flags=(unconfined)")
                && content.lines().any(|line| line.trim() == "userns,")
        })
    });
    let ready = !applicable
        || (apparmor_service_active && apparmor_profile_loaded && policy_matches_managed_chrome);
    let state = if installed_chrome.is_none() {
        "not_applicable_no_managed_chrome"
    } else if !restriction_enabled || !apparmor_enabled {
        "not_required"
    } else if !policy_present {
        "policy_missing"
    } else if !policy_matches_managed_chrome {
        "policy_mismatch"
    } else if !apparmor_service_active {
        "apparmor_inactive"
    } else if !apparmor_profile_loaded {
        "policy_not_loaded"
    } else {
        "ready"
    };

    json!({
        "applicable": applicable,
        "ready": ready,
        "state": state,
        "installedChrome": installed_chrome,
        "restrictionEnabled": restriction_enabled,
        "apparmorEnabled": apparmor_enabled,
        "apparmorServiceActive": apparmor_service_active,
        "apparmorProfileLoaded": apparmor_profile_loaded,
        "policyPath": policy_path.display().to_string(),
        "policyPresent": policy_present,
        "policyMatchesManagedChrome": policy_matches_managed_chrome,
        "expectedManagedChromeAttachment": managed_root,
    })
}

fn executable_candidate(explicit: Option<&str>, candidates: &[&str]) -> Option<String> {
    if let Some(path) = explicit {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    candidates
        .iter()
        .find_map(|candidate| command_path(candidate))
}

fn command_path(command: &str) -> Option<String> {
    let mut child = Command::new("sh");
    child.args(["-lc", &format!("command -v {}", shell_quote(command))]);
    match run_command_with_timeout(child, DOCTOR_TEXT_COMMAND_TIMEOUT) {
        DoctorCommandResult::Output(output) => {
            if !output.status.success() {
                return None;
            }
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        }
        DoctorCommandResult::Timeout { .. } | DoctorCommandResult::SpawnError(_) => None,
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn inspect_remote_view_config() -> Value {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from(""));
    let agent_home = env::var("AGENT_BROWSER_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".agent-browser"));
    let paths = [
        agent_home.join(".env"),
        agent_home.join("secrets/guacamole.env"),
        agent_home.join("guacamole/docker-compose.yml"),
        agent_home.join("guacamole/compose.yml"),
    ];
    let files = paths
        .iter()
        .map(|path| inspect_config_file(path))
        .collect::<Vec<_>>();
    json!({
        "agentBrowserHome": agent_home.display().to_string(),
        "files": files,
    })
}

fn inspect_config_file(path: &Path) -> Value {
    let text = fs::read_to_string(path).ok();
    json!({
        "path": path.display().to_string(),
        "exists": path.exists(),
        "keys": text.as_deref().map(parse_env_keys).unwrap_or_default(),
    })
}

fn parse_env_keys(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let (key, _) = trimmed.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        })
        .collect()
}

fn parse_ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = &trimmed[1..trimmed.len() - 1] == section;
            continue;
        }
        if !in_section || trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#')
        {
            continue;
        }
        let Some((candidate, value)) = trimmed.split_once('=') else {
            continue;
        };
        if candidate.trim() == key {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn many_to_many_status(
    rdp_gateway: &Value,
    route_pool: &Value,
    route_displays: &Value,
    display_access: &Value,
    viewer_prerequisites: &Value,
) -> Value {
    let private_display_allocator_ready =
        readiness_component_ready(rdp_gateway, "private_display_allocator");
    let route_pool_ready = nested_bool(route_pool, &["data", "success"]);
    let display_ready = nested_bool(route_displays, &["data", "success"]);
    let display_access_ready = display_access
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let viewer_prerequisites_ready = viewer_prerequisites
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if route_pool_ready && display_ready && display_access_ready {
        if viewer_prerequisites_ready {
            "ready"
        } else {
            "needs_viewer_prerequisites"
        }
    } else if route_pool_ready && display_ready {
        "needs_display_access"
    } else if route_pool_ready {
        "needs_route_displays"
    } else {
        "blocked"
    };
    json!({
        "status": status,
        "privateDisplayAllocatorReady": private_display_allocator_ready,
        "routePoolReady": route_pool_ready,
        "routeDisplaysReady": display_ready,
        "routeDisplayAccessReady": display_access_ready,
        "viewerPrerequisitesReady": viewer_prerequisites_ready,
        "simultaneousViewingReady": route_pool_ready && display_ready && display_access_ready && viewer_prerequisites_ready,
    })
}

#[derive(Debug, PartialEq, Eq)]
struct RemoteControlInstallReadiness {
    doctor_ready: bool,
    ready: bool,
    non_blocking_issue_codes: Vec<String>,
}

/// Classify embedded install-doctor output for single-route remote control.
///
/// The install doctor remains the authority for aggregate workstation health.
/// Remote control may proceed through unrelated duplicate-profile pressure or
/// exact issues copied from inactive optional session supervisors only when the
/// structured report proves that no resource candidate affects readiness.
/// Missing, active-supervisor, or mixed evidence fails closed.
fn remote_control_install_readiness(install: &Value) -> RemoteControlInstallReadiness {
    let doctor_ready = nested_bool(install, &["success"]);
    if doctor_ready {
        return RemoteControlInstallReadiness {
            doctor_ready,
            ready: true,
            non_blocking_issue_codes: Vec::new(),
        };
    }

    let issues = install
        .pointer("/data/data/issues")
        .and_then(Value::as_array);
    let readiness_impacting_candidates = install
        .pointer("/data/data/serviceResources/readinessImpactingCandidates")
        .and_then(Value::as_u64);
    let session_supervisors = install
        .pointer("/data/data/sessionSupervisors")
        .unwrap_or(&Value::Null);
    let inactive_supervisors = session_supervisors
        .get("sessions")
        .and_then(Value::as_array)
        .is_some_and(|sessions| {
            sessions.iter().all(|session| {
                session.get("activeState").and_then(Value::as_str) == Some("inactive")
            })
        });
    let supervisor_issues = session_supervisors
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let non_blocking_issue_codes = issues
        .filter(|issues| !issues.is_empty())
        .and_then(|issues| {
            issues
                .iter()
                .map(|issue| {
                    let code = issue.get("code").and_then(Value::as_str)?;
                    if code == "service_duplicate_profile_pressure"
                        || code == "active_runtime_manual_preservation"
                        || (inactive_supervisors && supervisor_issues.contains(issue))
                    {
                        Some(code.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_default();
    let advisory_issues_only = issues
        .is_some_and(|issues| !issues.is_empty() && non_blocking_issue_codes.len() == issues.len());
    let advisory_issues_are_non_blocking = !doctor_command_timed_out(install)
        && advisory_issues_only
        && readiness_impacting_candidates == Some(0);

    RemoteControlInstallReadiness {
        doctor_ready,
        ready: advisory_issues_are_non_blocking,
        non_blocking_issue_codes: if advisory_issues_are_non_blocking {
            non_blocking_issue_codes
        } else {
            Vec::new()
        },
    }
}

fn remote_control_status(
    install: &Value,
    rdp_gateway: &Value,
    route_pool: &Value,
    route_displays: &Value,
    viewer_prerequisites: &Value,
) -> Value {
    let install_readiness = remote_control_install_readiness(install);
    let install_ready = install_readiness.ready;
    let install_doctor_timed_out = doctor_command_timed_out(install);
    let route_pool_ready = nested_bool(route_pool, &["data", "success"]);
    let rdp_gateway_ready = nested_bool(rdp_gateway, &["data", "success"]);
    let private_display_allocator_ready =
        readiness_component_ready(rdp_gateway, "private_display_allocator");
    let route_entry = route_pool
        .pointer("/data/routePoolJson/0")
        .cloned()
        .unwrap_or(Value::Null);
    let route_id = route_entry
        .get("routeId")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let route_pool_entry_id = route_entry
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let frame_url = route_entry
        .get("frameUrl")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let external_url = route_entry
        .get("externalUrl")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let display_name = selected_single_route_display(route_displays, &route_entry);
    let display_access = display_name
        .as_deref()
        .map(run_display_access_probe)
        .unwrap_or_else(|| {
            json!({
                "available": false,
                "success": false,
                "exitCode": null,
                "stdout": "",
                "stderr": "no selected route display"
            })
        });
    let route_url_ready = frame_url.is_some() || external_url.is_some();
    let display_claimed = display_name.is_some();
    let display_ready = display_claimed && route_pool_ready;
    let display_access_ready = display_access
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_launch_ready = viewer_prerequisites
        .get("browserLaunchReady")
        .or_else(|| viewer_prerequisites.get("ready"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ready = install_ready
        && route_pool_ready
        && rdp_gateway_ready
        && private_display_allocator_ready
        && route_url_ready
        && display_ready
        && display_access_ready
        && browser_launch_ready;
    let status = if ready {
        "ready"
    } else if route_pool_ready
        && rdp_gateway_ready
        && route_url_ready
        && display_ready
        && display_access_ready
    {
        "needs_browser_launch_prerequisites"
    } else if route_pool_ready && rdp_gateway_ready && route_url_ready && display_ready {
        "needs_display_access"
    } else if route_pool_ready && rdp_gateway_ready && route_url_ready {
        "needs_route_display"
    } else if route_pool_ready && rdp_gateway_ready {
        "needs_route_url"
    } else {
        "blocked"
    };
    let next_action = if ready {
        "run_remote_view_open_live_gate".to_string()
    } else if install_doctor_timed_out {
        "rerun_install_doctor_after_timeout".to_string()
    } else if !install_ready {
        "repair_install_drift".to_string()
    } else if !rdp_gateway_ready {
        "run_rdp_gateway_readiness".to_string()
    } else if !route_pool_ready {
        first_failed_route_pool_next_action(route_pool).unwrap_or_else(|| {
            "repair_or_sync_guacamole_route_pool_before_creating_more_users".to_string()
        })
    } else if !route_url_ready {
        "repair_guacamole_route_url".to_string()
    } else if !display_ready {
        "open_or_select_single_rdp_route_display".to_string()
    } else if !display_access_ready {
        "grant_route_display_access".to_string()
    } else if !browser_launch_ready {
        "install_viewer_prerequisites_for_many_to_many_gate".to_string()
    } else {
        "agent_browser_remote_control_recheck".to_string()
    };
    json!({
        "status": status,
        "ready": ready,
        "installReady": install_ready,
        "installDoctorReady": install_readiness.doctor_ready,
        "nonBlockingInstallIssueCodes": install_readiness.non_blocking_issue_codes,
        "installDoctorTimedOut": install_doctor_timed_out,
        "rdpGatewayReady": rdp_gateway_ready,
        "privateDisplayAllocatorReady": private_display_allocator_ready,
        "routePoolReady": route_pool_ready,
        "routeUrlReady": route_url_ready,
        "routeDisplayReady": display_ready,
        "routeDisplayClaimed": display_claimed,
        "routeDisplayAccessReady": display_access_ready,
        "browserLaunchReady": browser_launch_ready,
        "routePoolEntryId": route_pool_entry_id,
        "routeId": route_id,
        "displayName": display_name,
        "frameUrl": frame_url,
        "externalUrl": external_url,
        "displayAccess": display_access,
        "nextAction": next_action,
        "liveGateCommand": "pnpm test:remote-view-open-live",
        "scope": "single_route_remote_control",
    })
}

fn selected_single_route_display(route_displays: &Value, route_entry: &Value) -> Option<String> {
    if let Some(display) = route_entry
        .pointer("/target/displayName")
        .and_then(Value::as_str)
    {
        if !display.trim().is_empty() {
            return Some(display.trim().to_string());
        }
    }
    for pointer in [
        "/data/routes/A/displayName",
        "/data/routes/B/displayName",
        "/data/existingUserRoutes/0/displayName",
        "/data/routeSpecificUsers/A/displayName",
        "/data/routeSpecificUsers/B/displayName",
    ] {
        if let Some(display) = route_displays.pointer(pointer).and_then(Value::as_str) {
            if !display.trim().is_empty() {
                return Some(display.trim().to_string());
            }
        }
    }
    None
}

fn nested_bool(value: &Value, path: &[&str]) -> bool {
    let mut current = value;
    for key in path {
        current = current.get(*key).unwrap_or(&Value::Null);
    }
    current.as_bool().unwrap_or(false)
}

fn first_failed_route_pool_next_action(route_pool: &Value) -> Option<String> {
    route_pool
        .pointer("/data/readiness/components")
        .and_then(Value::as_array)
        .and_then(|components| {
            components.iter().find_map(|component| {
                let status = component
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                if status == "ready" {
                    return None;
                }
                component
                    .get("nextAction")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
            })
        })
}

fn install_has_issue_code(install: &Value, code: &str) -> bool {
    install
        .pointer("/data/data/issues")
        .and_then(Value::as_array)
        .or_else(|| install.pointer("/data/issues").and_then(Value::as_array))
        .map(|issues| {
            issues.iter().any(|issue| {
                issue
                    .get("code")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == code)
            })
        })
        .unwrap_or(false)
}

struct RecommendationContext<'a> {
    install: &'a Value,
    rdp_gateway: &'a Value,
    route_pool: &'a Value,
    route_displays: &'a Value,
    display_access: &'a Value,
    viewer_prerequisites: &'a Value,
    users: &'a Value,
    privileges: &'a Value,
}

/// Returns true only when a child diagnostic exceeded its bound without returning a result.
fn doctor_command_timed_out(result: &Value) -> bool {
    nested_bool(result, &["timedOut"])
}

fn route_display_recovery_action(route_displays: &Value, users: &Value) -> Option<String> {
    if doctor_command_timed_out(route_displays) || nested_bool(route_displays, &["data", "success"])
    {
        return None;
    }
    let route_specific_users_ready = users
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            ["agent-browser-rdp-a", "agent-browser-rdp-b"]
                .iter()
                .all(|name| {
                    entries.iter().any(|entry| {
                        entry.get("user").and_then(Value::as_str) == Some(*name)
                            && entry.get("exists").and_then(Value::as_bool) == Some(true)
                    })
                })
        })
        .unwrap_or(false);
    if route_specific_users_ready {
        return Some("open_route_specific_rdp_sessions_then_rerun_doctor".to_string());
    }
    let existing_route_count = route_displays
        .pointer("/data/existingUserRoutes")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if existing_route_count == 1 {
        return Some(
            "existing_agent_browser_rdp_routes_collapsed_to_one_display_use_route_specific_user_or_xrdp_policy_isolation"
                .to_string(),
        );
    }
    let existing_user = users
        .get("entries")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry["user"] == "agent-browser-rdp")
        })
        .and_then(|entry| entry.get("exists"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if existing_user {
        return Some(
            "open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor"
                .to_string(),
        );
    }
    Some("create_or_repair_single_agent_browser_rdp_user_before_route_specific_users".to_string())
}

fn recommend_next_action(context: RecommendationContext<'_>) -> String {
    let RecommendationContext {
        install,
        rdp_gateway,
        route_pool,
        route_displays,
        display_access,
        viewer_prerequisites,
        users,
        privileges,
    } = context;
    if !remote_control_install_readiness(install).ready && !doctor_command_timed_out(install) {
        if install_has_issue_code(install, "active_runtime_stale_executable")
            || install_has_issue_code(install, "active_runtime_stale_stream_backend")
        {
            return "restart_stale_daemon_sessions_then_rerun_doctor".to_string();
        }
        if install_has_issue_code(install, "dashboard_runtime_stale_or_unreadable") {
            return "converge_local_runtime_then_rerun_doctor".to_string();
        }
        return "repair_install_drift".to_string();
    }
    if doctor_command_timed_out(install)
        || doctor_command_timed_out(rdp_gateway)
        || doctor_command_timed_out(route_pool)
    {
        if let Some(action) = route_display_recovery_action(route_displays, users) {
            return action;
        }
        if doctor_command_timed_out(install) {
            return "rerun_install_doctor_after_timeout".to_string();
        }
        if doctor_command_timed_out(rdp_gateway) {
            return "rerun_rdp_gateway_readiness_after_timeout".to_string();
        }
        return "rerun_route_pool_readiness_after_timeout".to_string();
    }
    if !nested_bool(rdp_gateway, &["available"]) {
        return "run_rdp_gateway_readiness".to_string();
    }
    if !readiness_component_ready(rdp_gateway, "private_display_allocator") {
        return "clear_stale_private_display_locks_or_expand_allocator_range".to_string();
    }
    if !nested_bool(route_pool, &["data", "success"]) {
        if let Some(next_action) = first_failed_route_pool_next_action(route_pool) {
            return next_action;
        }
        return "repair_or_sync_guacamole_route_pool_before_creating_more_users".to_string();
    }
    if let Some(action) = route_display_recovery_action(route_displays, users) {
        return action;
    }
    if !display_access
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if privileges
            .get("ready")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return "grant_route_display_access".to_string();
        }
        return "install_privileged_helper_then_grant_route_display_access".to_string();
    }
    if !privileges
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return "install_privileged_helper_for_recurring_desktop_setup".to_string();
    }
    if !viewer_prerequisites
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return "install_viewer_prerequisites_for_many_to_many_gate".to_string();
    }
    "run_many_to_many_live_gate".to_string()
}

fn recommend_next_command(next_action: &str) -> Value {
    match next_action {
        "repair_or_sync_guacamole_route_pool_before_creating_more_users" => json!({
            "command": "pnpm test:rdp-guac-route-pool-readiness -- --report-only",
            "requiresInteractiveSudo": false,
            "why": "Inspect the current Guacamole route-pool state before mutating users or records."
        }),
        "repair_guacamole_admin_credentials" => json!({
            "command": "pnpm test:rdp-guac-route-pool-readiness -- --report-only",
            "requiresInteractiveSudo": false,
            "why": "The Guacamole token endpoint rejected the configured credentials; repair the agent-browser Guacamole secret file before opening route clients."
        }),
        "run_rdp_gateway_readiness" => json!({
            "command": "pnpm test:rdp-gateway-readiness-live",
            "requiresInteractiveSudo": false,
            "why": "Inspect local RDP gateway services and private display allocator state before route-pool or launch work."
        }),
        "clear_stale_private_display_locks_or_expand_allocator_range" => json!({
            "command": "agent-browser doctor remote-view --json",
            "requiresInteractiveSudo": false,
            "why": "The private display allocator is blocked; inspect stale /tmp/.X*-lock evidence before launching hidden remote-headed browsers."
        }),
        "existing_agent_browser_rdp_routes_collapsed_to_one_display_use_route_specific_user_or_xrdp_policy_isolation" => json!({
            "command": "pnpm setup:rdp-guac-route-pool",
            "requiresInteractiveSudo": true,
            "why": "The existing-user Guacamole route shape already collapsed to one XRDP display; create the explicit isolated route users from an interactive terminal."
        }),
        "open_route_specific_rdp_sessions_then_rerun_doctor" => json!({
            "command": "pnpm open:rdp-route-displays",
            "requiresInteractiveSudo": false,
            "why": "Route-specific users exist; open both Guacamole route clients, authenticate them, then inspect whether XRDP allocated distinct displays."
        }),
        "open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor" => json!({
            "command": "pnpm open:rdp-route-displays",
            "requiresInteractiveSudo": false,
            "why": "Open both existing-user Guacamole route clients, authenticate them, then inspect whether XRDP allocated distinct displays."
        }),
        "repair_rdp_route_display_session" => json!({
            "command": "pnpm open:rdp-route-displays",
            "requiresInteractiveSudo": false,
            "why": "The configured Guacamole routes have no live X11 display sockets. Open and authenticate the route desktops, then rerun the doctor."
        }),
        "install_privileged_helper_then_grant_route_display_access" => json!({
            "command": "pnpm install:privileges -- --apply && newgrp agent-browser",
            "requiresInteractiveSudo": true,
            "why": "The route displays exist, but the agent user cannot open windows on them yet; install the one-time helper before applying display grants."
        }),
        "install_privileged_helper_for_recurring_desktop_setup" => json!({
            "command": "pnpm install:privileges -- --apply && newgrp agent-browser",
            "requiresInteractiveSudo": true,
            "why": "Many-to-many viewing is ready, but the recurring desktop setup helper is not installed for the agent-browser group yet."
        }),
        "grant_route_display_access" => json!({
            "command": "pnpm grant:rdp-route-display-access -- --apply",
            "requiresInteractiveSudo": false,
            "why": "The privileged helper is ready; grant the agent user local X access to the active route displays."
        }),
        "create_or_repair_single_agent_browser_rdp_user_before_route_specific_users" => json!({
            "command": "agent-browser doctor remote-view",
            "requiresInteractiveSudo": false,
            "why": "The reusable agent-browser-rdp user is missing; inspect setup state before creating route-specific users."
        }),
        "repair_install_drift" => json!({
            "command": "agent-browser install doctor --json",
            "requiresInteractiveSudo": false,
            "why": "The installed command, current executable, package binary, workspace binary, or launch configuration is out of sync."
        }),
        "rerun_install_doctor_after_timeout" => json!({
            "command": "agent-browser install doctor --json",
            "requiresInteractiveSudo": false,
            "why": "The embedded install doctor timed out without proving install drift. Run it directly before changing installed state."
        }),
        "rerun_rdp_gateway_readiness_after_timeout" => json!({
            "command": "pnpm test:rdp-gateway-readiness-live",
            "requiresInteractiveSudo": false,
            "why": "The embedded RDP gateway readiness helper timed out without returning a failed readiness result."
        }),
        "rerun_route_pool_readiness_after_timeout" => json!({
            "command": "pnpm test:rdp-guac-route-pool-readiness -- --report-only",
            "requiresInteractiveSudo": false,
            "why": "The embedded route-pool readiness helper timed out without returning a failed readiness result."
        }),
        "rerun_route_display_inspection_after_timeout" => json!({
            "command": "node scripts/inspect-rdp-route-displays.js",
            "requiresInteractiveSudo": false,
            "why": "The embedded route-display inspection helper timed out without returning a display result."
        }),
        "restart_stale_daemon_sessions_then_rerun_doctor" => json!({
            "command": "agent-browser install doctor --json",
            "requiresInteractiveSudo": false,
            "why": "Install doctor reported active stale daemon sessions. Use each issue's remedy.argv to close only the affected session, then rerun the doctor."
        }),
        "converge_local_runtime_then_rerun_doctor" => json!({
            "command": "pnpm converge:local-runtime -- --apply --json",
            "requiresInteractiveSudo": false,
            "why": "Install doctor reported a running dashboard runtime that does not match the current executable. Run the bounded local convergence command, then rerun the doctor."
        }),
        "run_many_to_many_live_gate" => json!({
            "command": "pnpm test:rdp-guac-many-to-many-live",
            "requiresInteractiveSudo": false,
            "why": "Route pool and route displays are ready; run the OCR-backed many-to-many gate."
        }),
        "install_viewer_prerequisites_for_many_to_many_gate" => json!({
            "command": "agent-browser doctor remote-view --json",
            "requiresInteractiveSudo": false,
            "why": "Route pool and route displays are ready, but the OCR/browser viewer prerequisites for the many-to-many gate are missing."
        }),
        _ => json!({
            "command": "agent-browser doctor remote-view",
            "requiresInteractiveSudo": false,
            "why": "Re-run the doctor after resolving the reported state."
        }),
    }
}

struct RemoteViewIssueContext<'a> {
    install: &'a Value,
    rdp_gateway: &'a Value,
    route_pool: &'a Value,
    route_displays: &'a Value,
    display_access: &'a Value,
    viewer_prerequisites: &'a Value,
    users: &'a Value,
    privileges: &'a Value,
    next_action: &'a str,
}

fn remote_view_issues(context: RemoteViewIssueContext<'_>) -> Vec<Value> {
    let mut issues = Vec::new();
    let RemoteViewIssueContext {
        install,
        rdp_gateway,
        route_pool,
        route_displays,
        display_access,
        viewer_prerequisites,
        users,
        privileges,
        next_action,
    } = context;

    if !remote_control_install_readiness(install).ready {
        if doctor_command_timed_out(install) {
            issues.push(remote_view_issue(
                "install_doctor_timed_out",
                "the embedded install doctor timed out without proving install drift",
                "run agent-browser install doctor --json directly and use its result as the install authority",
                false,
                "rerun_install_doctor_after_timeout",
            ));
        } else {
            let initial_issue_count = issues.len();
            let install_issues = install
                .pointer("/data/data/issues")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for issue in install_issues {
                let code = issue
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("install_doctor_not_ready");
                let message = issue
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("agent-browser install doctor reported an issue");
                issues.push(remote_view_issue(
                    &format!("install_{code}"),
                    message,
                    "run agent-browser install doctor --json and resolve the reported install drift before relying on remote-view setup",
                    false,
                    "repair_install_drift",
                ));
            }
            if issues.len() == initial_issue_count {
                issues.push(remote_view_issue(
                    "install_doctor_not_ready",
                    "agent-browser install doctor did not report success",
                    "run agent-browser install doctor --json and resolve the reported install drift",
                    false,
                    "repair_install_drift",
                ));
            }
        }
    }

    if doctor_command_timed_out(route_pool) {
        issues.push(remote_view_issue(
            "route_pool_readiness_timed_out",
            "the embedded Guacamole route-pool readiness helper timed out",
            "run pnpm test:rdp-guac-route-pool-readiness -- --report-only directly and use its returned readiness result",
            false,
            "rerun_route_pool_readiness_after_timeout",
        ));
    } else if !nested_bool(route_pool, &["available"]) {
        issues.push(remote_view_issue(
            "route_pool_readiness_unavailable",
            "the Guacamole route-pool readiness helper could not be run",
            "run pnpm test:rdp-guac-route-pool-readiness -- --report-only from the repo root",
            false,
            "route_pool",
        ));
    } else if !nested_bool(route_pool, &["data", "success"]) {
        let components = route_pool
            .pointer("/data/readiness/components")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for component in components {
            let status = component
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if status == "ready" {
                continue;
            }
            let name = component
                .get("component")
                .and_then(Value::as_str)
                .unwrap_or("route_pool");
            let next_action = component
                .get("nextAction")
                .and_then(Value::as_str)
                .unwrap_or("repair_route_pool");
            let recovery = component
                .get("recovery")
                .and_then(Value::as_str)
                .unwrap_or("repair the Guacamole route-pool prerequisite, then rerun doctor");
            let issue_code = route_pool_component_issue_code(name, status);
            issues.push(remote_view_issue(
                &issue_code,
                component
                    .get("evidence")
                    .and_then(Value::as_str)
                    .unwrap_or("route-pool readiness is not ready"),
                recovery,
                false,
                next_action,
            ));
        }
        if issues.is_empty() {
            issues.push(remote_view_issue(
                "route_pool_not_ready",
                "the Guacamole route pool is not ready",
                "run pnpm test:rdp-guac-route-pool-readiness -- --report-only and repair the first blocked component",
                false,
                "route_pool",
            ));
        }
    }

    if doctor_command_timed_out(rdp_gateway) {
        issues.push(remote_view_issue(
            "rdp_gateway_readiness_timed_out",
            "the embedded RDP gateway readiness helper timed out",
            "run pnpm test:rdp-gateway-readiness-live directly and use its returned readiness result",
            false,
            "rerun_rdp_gateway_readiness_after_timeout",
        ));
    } else if !nested_bool(rdp_gateway, &["available"]) {
        issues.push(remote_view_issue(
            "rdp_gateway_readiness_unavailable",
            "the RDP gateway readiness helper could not be run",
            "run pnpm test:rdp-gateway-readiness-live from the repo root",
            false,
            "run_rdp_gateway_readiness",
        ));
    } else {
        for component in rdp_gateway
            .pointer("/data/readiness/components")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let name = component
                .get("component")
                .and_then(Value::as_str)
                .unwrap_or("rdp_gateway");
            if name != "private_display_allocator" {
                continue;
            }
            let status = component
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if status == "ready" {
                continue;
            }
            issues.push(remote_view_issue(
                &format!("rdp_gateway_{name}_{status}"),
                component
                    .get("evidence")
                    .and_then(Value::as_str)
                    .unwrap_or("private display allocator is not ready"),
                component
                    .get("recovery")
                    .and_then(Value::as_str)
                    .unwrap_or("clear stale private display locks or expand the allocator range"),
                false,
                component
                    .get("nextAction")
                    .and_then(Value::as_str)
                    .unwrap_or("clear_stale_private_display_locks_or_expand_allocator_range"),
            ));
        }
    }

    if doctor_command_timed_out(route_displays) {
        issues.push(remote_view_issue(
            "route_display_inspection_timed_out",
            "the embedded route-display inspection helper timed out",
            "run node scripts/inspect-rdp-route-displays.js directly before changing route-display state",
            false,
            "rerun_route_display_inspection_after_timeout",
        ));
    } else if !nested_bool(route_displays, &["data", "success"]) {
        let route_specific_users_ready = users
            .get("entries")
            .and_then(Value::as_array)
            .map(|entries| {
                ["agent-browser-rdp-a", "agent-browser-rdp-b"]
                    .iter()
                    .all(|name| {
                        entries.iter().any(|entry| {
                            entry.get("user").and_then(Value::as_str) == Some(*name)
                                && entry.get("exists").and_then(Value::as_bool) == Some(true)
                        })
                    })
            })
            .unwrap_or(false);
        let remediation = if route_specific_users_ready {
            "open both route-specific Guacamole/RDP sessions, then rerun agent-browser doctor remote-view"
        } else {
            "run pnpm setup:rdp-guac-route-pool after the display gate proves the existing-user routes collapse to one display"
        };
        issues.push(remote_view_issue(
            "route_displays_missing_or_collapsed",
            "two distinct route displays are not currently visible",
            remediation,
            !route_specific_users_ready,
            next_action,
        ));
    }

    if nested_bool(route_displays, &["data", "success"])
        && !display_access
            .get("ready")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        issues.push(remote_view_issue(
            "route_display_access_missing",
            "the current user cannot open windows on every active route display",
            "run pnpm grant:rdp-route-display-access -- --apply after the one-time privileged helper is installed",
            false,
            "grant_route_display_access",
        ));
    }

    if !viewer_prerequisites
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let mut missing = Vec::new();
        if viewer_prerequisites
            .pointer("/clients/A/path")
            .and_then(Value::as_str)
            .is_none()
        {
            missing.push("client A browser executable");
        }
        if viewer_prerequisites
            .pointer("/clients/B/path")
            .and_then(Value::as_str)
            .is_none()
        {
            missing.push("client B browser executable");
        }
        for tool in ["identify", "convert", "tesseract"] {
            if viewer_prerequisites
                .pointer(&format!("/tools/{tool}"))
                .and_then(Value::as_str)
                .is_none()
            {
                missing.push(tool);
            }
        }
        if !viewer_prerequisites
            .pointer("/managedChromeSandboxPolicy/ready")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        {
            missing.push("managed Chrome AppArmor user-namespace policy");
        }
        issues.push(remote_view_issue(
            "viewer_prerequisites_missing",
            &format!(
                "many-to-many viewer prerequisites are missing: {}",
                missing.join(", ")
            ),
            "rerun agent-browser install workstation --apply to install the managed Chrome AppArmor policy and viewer proof tools",
            false,
            "install_viewer_prerequisites_for_many_to_many_gate",
        ));
    }
    if !viewer_prerequisites
        .pointer("/managedChromeSandboxPolicy/ready")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        issues.push(remote_view_issue(
            "managed_chrome_sandbox_policy_not_ready",
            "managed Chrome cannot use its sandbox while Ubuntu AppArmor user-namespace restrictions are active",
            "rerun agent-browser install workstation --apply to install and load the path-scoped managed Chrome AppArmor policy",
            true,
            "install_viewer_prerequisites_for_many_to_many_gate",
        ));
    }

    if !privileges
        .get("groupExists")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        issues.push(remote_view_issue(
            "remote_view_privileged_group_missing",
            "the agent-browser privileged group is missing",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
            "install_privileged_helper",
        ));
    }
    if !privileges
        .get("userInGroup")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        issues.push(remote_view_issue(
            "remote_view_privileged_group_membership_missing",
            "the current user is not in the agent-browser privileged group",
            "run agent-browser install --with-remote-view-privileges, then open a new shell or run newgrp agent-browser",
            true,
            "install_privileged_helper",
        ));
    }
    if !privileges
        .get("helperExists")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        issues.push(remote_view_issue(
            "remote_view_privileged_helper_missing",
            "the root-owned remote-view privileged helper is missing",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
            "install_privileged_helper",
        ));
    }
    if !privileges
        .get("sudoersExists")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        issues.push(remote_view_issue(
            "remote_view_privileged_sudoers_missing",
            "the remote-view sudoers policy is missing",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
            "install_privileged_helper",
        ));
    }
    if privileges
        .get("helperExists")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && privileges
            .pointer("/helperCheck/success")
            .and_then(Value::as_bool)
            != Some(true)
    {
        issues.push(remote_view_issue(
            "remote_view_privileged_helper_not_usable",
            "the remote-view privileged helper cannot be run with sudo -n",
            "confirm the sudoers file and group membership are active in this shell, then rerun doctor",
            false,
            "install_privileged_helper",
        ));
    }
    if privileges
        .get("helperExists")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && privileges
            .pointer("/helperDesktopSession/ready")
            .and_then(Value::as_bool)
            != Some(true)
    {
        issues.push(remote_view_issue(
            "remote_view_route_desktop_helper_stale",
            "the installed remote-view helper still writes a terminal-first route desktop session",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
            "install_privileged_helper",
        ));
    }
    if privileges
        .get("helperExists")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !remote_view_helper_status_contract_ready(
            privileges.get("helperStatus").unwrap_or(&Value::Null),
        )
    {
        issues.push(remote_view_issue(
            "remote_view_privileged_helper_status_stale",
            "the installed remote-view helper does not expose the current route-desktop and display-access capability contract",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
            "install_privileged_helper",
        ));
    }

    issues
}

fn route_pool_component_issue_code(component_name: &str, status: &str) -> String {
    match component_name {
        "guacamole_schema" => "guacamole_schema_missing".to_string(),
        "guacamole_login" => "guacamole_login_failed".to_string(),
        "guacamole_connection_permissions" => "guacamole_connection_permission_missing".to_string(),
        _ => format!("route_pool_{component_name}_{status}").replace(':', "_"),
    }
}

fn remote_view_issue(
    code: &str,
    message: &str,
    remediation: &str,
    requires_interactive_sudo: bool,
    next_action: &str,
) -> Value {
    json!({
        "code": code,
        "message": message,
        "remediation": remediation,
        "requiresInteractiveSudo": requires_interactive_sudo,
        "nextAction": next_action,
    })
}

fn drift_findings(users: &Value, config: &Value) -> Vec<Value> {
    let mut findings = Vec::new();
    let route_specific_count = users
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.get("user").and_then(Value::as_str),
                        Some("agent-browser-rdp-a") | Some("agent-browser-rdp-b")
                    ) && entry.get("exists").and_then(Value::as_bool) == Some(true)
                })
                .count()
        })
        .unwrap_or(0);
    if route_specific_count > 0 {
        findings.push(json!({
            "code": "route_specific_rdp_users_present",
            "message": "route-specific RDP users exist; prefer the single agent-browser-rdp user unless display isolation requires them"
        }));
    }

    let guacamole_env_count = config
        .get("files")
        .and_then(Value::as_array)
        .map(|files| {
            files
                .iter()
                .filter(|file| {
                    file.get("path")
                        .and_then(Value::as_str)
                        .is_some_and(|path| path.ends_with("guacamole.env"))
                        && file.get("exists").and_then(Value::as_bool) == Some(true)
                })
                .count()
        })
        .unwrap_or(0);
    if guacamole_env_count > 1 {
        findings.push(json!({
            "code": "multiple_guacamole_env_files",
            "message": "multiple Guacamole env files were found; consolidate before further setup"
        }));
    }
    findings
}

fn state_sources() -> Vec<Value> {
    vec![
        json!({"source": "agent-browser install doctor", "kind": "install"}),
        json!({"source": "scripts/smoke-rdp-gateway-readiness.js", "kind": "rdp-gateway"}),
        json!({"source": "scripts/smoke-rdp-guac-route-pool-readiness.js --report-only", "kind": "guacamole"}),
        json!({"source": "scripts/inspect-rdp-route-displays.js", "kind": "rdp-displays"}),
        json!({"source": "xdpyinfo with route display names", "kind": "rdp-display-access"}),
        json!({"source": "getent group agent-browser, id -nG, and sudo -n privileged helper check", "kind": "privileges"}),
        json!({"source": "/etc/xrdp/sesman.ini", "kind": "rdp-host"}),
        json!({"source": "getent passwd agent-browser-rdp agent-browser-rdp-a agent-browser-rdp-b", "kind": "users"}),
        json!({"source": "~/.agent-browser/.env and ~/.agent-browser/secrets/guacamole.env", "kind": "config"}),
    ]
}

fn run_command_with_timeout(mut command: Command, timeout: Duration) -> DoctorCommandResult {
    let output_id = DOCTOR_COMMAND_OUTPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let output_prefix = env::temp_dir().join(format!(
        "agent-browser-remote-view-doctor-command-{}-{output_id}",
        std::process::id()
    ));
    let stdout_path = output_prefix.with_extension("stdout");
    let stderr_path = output_prefix.with_extension("stderr");
    let stdout_file = match fs::File::create(&stdout_path) {
        Ok(file) => file,
        Err(error) => return DoctorCommandResult::SpawnError(error.to_string()),
    };
    let stderr_file = match fs::File::create(&stderr_path) {
        Ok(file) => file,
        Err(error) => {
            let _ = fs::remove_file(&stdout_path);
            return DoctorCommandResult::SpawnError(error.to_string());
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
            return DoctorCommandResult::SpawnError(error.to_string());
        }
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return match child.wait() {
                    Ok(status) => {
                        let stdout = fs::read(&stdout_path).unwrap_or_default();
                        let stderr = fs::read(&stderr_path).unwrap_or_default();
                        let _ = fs::remove_file(&stdout_path);
                        let _ = fs::remove_file(&stderr_path);
                        DoctorCommandResult::Output(Output {
                            status,
                            stdout,
                            stderr,
                        })
                    }
                    Err(error) => DoctorCommandResult::SpawnError(error.to_string()),
                };
            }
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let wait_error = child.wait().err().map(|error| error.to_string());
                    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
                    let stderr = wait_error
                        .or_else(|| fs::read_to_string(&stderr_path).ok())
                        .unwrap_or_default();
                    let _ = fs::remove_file(&stdout_path);
                    let _ = fs::remove_file(&stderr_path);
                    return DoctorCommandResult::Timeout { stdout, stderr };
                }
                thread::sleep(DOCTOR_COMMAND_POLL_INTERVAL);
            }
            Err(error) => {
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return DoctorCommandResult::SpawnError(error.to_string());
            }
        }
    }
}

fn run_text_command(command: &str, args: &[&str]) -> Value {
    run_text_command_with_env(command, args, &[])
}

fn run_text_command_with_env(command: &str, args: &[&str], envs: &[(&str, &str)]) -> Value {
    let mut child = Command::new(command);
    child.args(args);
    for (key, value) in envs {
        child.env(key, value);
    }
    match run_command_with_timeout(child, DOCTOR_TEXT_COMMAND_TIMEOUT) {
        DoctorCommandResult::Output(output) => json!({
            "available": true,
            "success": output.status.success(),
            "timedOut": false,
            "exitCode": output.status.code(),
            "stdout": redact_text(String::from_utf8_lossy(&output.stdout).trim()),
            "stderr": redact_text(String::from_utf8_lossy(&output.stderr).trim()),
        }),
        DoctorCommandResult::Timeout { stdout, stderr } => json!({
            "available": true,
            "success": false,
            "timedOut": true,
            "exitCode": null,
            "stdout": redact_text(stdout.trim()),
            "stderr": redact_text(if stderr.trim().is_empty() {
                "doctor text probe timed out"
            } else {
                stderr.trim()
            }),
        }),
        DoctorCommandResult::SpawnError(error) => json!({
            "available": false,
            "success": false,
            "timedOut": false,
            "exitCode": null,
            "stdout": "",
            "stderr": redact_text(error),
        }),
    }
}

fn run_display_access_probe(display_name: &str) -> Value {
    let mut child = Command::new("xdpyinfo");
    child.env("DISPLAY", display_name);
    match run_command_with_timeout(child, DOCTOR_DISPLAY_ACCESS_TIMEOUT) {
        DoctorCommandResult::Output(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let evidence = stdout
                .lines()
                .find(|line| line.trim_start().starts_with("name of display:"))
                .map(|line| line.trim().to_string())
                .unwrap_or_else(|| {
                    if output.status.success() {
                        "xdpyinfo connected to display".to_string()
                    } else {
                        "".to_string()
                    }
                });
            json!({
                "available": true,
                "success": output.status.success(),
                "timedOut": false,
                "exitCode": output.status.code(),
                "stdout": evidence,
                "stderr": redact_text(String::from_utf8_lossy(&output.stderr).trim()),
            })
        }
        DoctorCommandResult::Timeout { stdout, stderr } => json!({
            "available": true,
            "success": false,
            "timedOut": true,
            "exitCode": null,
            "stdout": redact_text(stdout.trim()),
            "stderr": redact_text(if stderr.trim().is_empty() {
                "xdpyinfo timed out"
            } else {
                stderr.trim()
            }),
        }),
        DoctorCommandResult::SpawnError(error) => json!({
            "available": false,
            "success": false,
            "timedOut": false,
            "exitCode": null,
            "stdout": "",
            "stderr": redact_text(error),
        }),
    }
}

fn redact_text(text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    text.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.contains("password") || lower.contains("secret") || lower.contains("token") {
                if let Some((key, _)) = line.split_once('=') {
                    format!("{}=<redacted>", key.trim())
                } else {
                    "<redacted>".to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_remote_view_doctor_report(report: &Value) {
    let data = &report["data"];
    println!(
        "{} agent-browser doctor remote-view",
        color::success_indicator()
    );
    println!("status: {}", display_value(&data["status"]));
    println!(
        "private display allocator ready: {}",
        display_value(&data["manyToMany"]["privateDisplayAllocatorReady"])
    );
    println!(
        "route pool ready: {}",
        display_value(&data["manyToMany"]["routePoolReady"])
    );
    println!(
        "route displays ready: {}",
        display_value(&data["manyToMany"]["routeDisplaysReady"])
    );
    println!(
        "route display access ready: {}",
        display_value(&data["manyToMany"]["routeDisplayAccessReady"])
    );
    println!(
        "single-route remote control ready: {}",
        display_value(&data["remoteControl"]["ready"])
    );
    println!(
        "single-route display: {}",
        display_value(&data["remoteControl"]["displayName"])
    );
    println!(
        "single-route external URL: {}",
        display_value(&data["remoteControl"]["externalUrl"])
    );
    println!(
        "dashboard runtime contract: {}",
        display_value(&data["dashboardRuntime"]["serviceContractVersion"])
    );
    println!(
        "dashboard runtime sha: {}",
        display_value(&data["dashboardRuntime"]["dashboard"]["sha256"])
    );
    println!(
        "dashboard runtime executable: {}",
        display_value(&data["dashboardRuntime"]["executable"]["sha256"])
    );
    println!(
        "runtime convergence: {}",
        display_value(&data["runtimeConvergence"]["status"])
    );
    println!(
        "requested scope: {}",
        display_value(&data["requestedScope"]["status"])
    );
    println!(
        "requested route: {}",
        display_value(&data["requestedScope"]["subject"]["routeId"])
    );
    println!(
        "global advisories: {}",
        data["globalAdvisories"]
            .as_array()
            .map_or_else(|| "unknown".to_string(), |values| values.len().to_string())
    );
    println!(
        "simultaneous viewing ready: {}",
        display_value(&data["manyToMany"]["simultaneousViewingReady"])
    );
    println!(
        "guacamole local embed ready: {}",
        display_value(&data["guacamole"]["routePool"]["data"]["guacamole"]["localEmbedReady"])
    );
    println!(
        "guacamole public operator ready: {}",
        display_value(&data["guacamole"]["routePool"]["data"]["guacamole"]["publicOperatorReady"])
    );
    println!("next action: {}", display_value(&data["nextAction"]));
    println!(
        "next command: {}",
        display_value(&data["nextCommand"]["command"])
    );
    println!(
        "requires interactive sudo: {}",
        display_value(&data["nextCommand"]["requiresInteractiveSudo"])
    );
    println!("rdp users:");
    if let Some(entries) = data["rdpHost"]["users"]["entries"].as_array() {
        for entry in entries {
            println!(
                "  - {} exists={}",
                display_value(&entry["user"]),
                display_value(&entry["exists"])
            );
        }
    }
    println!(
        "privileged helper: ready={} group={} userInGroup={} path={}",
        display_value(&data["rdpHost"]["privileges"]["ready"]),
        display_value(&data["rdpHost"]["privileges"]["groupName"]),
        display_value(&data["rdpHost"]["privileges"]["userInGroup"]),
        display_value(&data["rdpHost"]["privileges"]["helperPath"])
    );
    println!(
        "route desktop helper: state={} ready={}",
        display_value(&data["rdpHost"]["privileges"]["helperDesktopSession"]["state"]),
        display_value(&data["rdpHost"]["privileges"]["helperDesktopSession"]["ready"])
    );
    println!("display access:");
    if let Some(entries) = data["rdpHost"]["displayAccess"]["entries"].as_array() {
        for entry in entries {
            println!(
                "  - route {} display={} accessible={}",
                display_value(&entry["route"]),
                display_value(&entry["displayName"]),
                display_value(&entry["accessible"])
            );
        }
    }
    println!("config files:");
    if let Some(files) = data["config"]["files"].as_array() {
        for file in files {
            let key_count = file
                .get("keys")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            println!(
                "  - {} exists={} keys={}",
                display_value(&file["path"]),
                display_value(&file["exists"]),
                key_count
            );
        }
    }
    if let Some(findings) = data["drift"].as_array() {
        if !findings.is_empty() {
            println!("drift:");
            for finding in findings {
                println!(
                    "  - {}: {}",
                    display_value(&finding["code"]),
                    display_value(&finding["message"])
                );
            }
        }
    }
}

fn dashboard_runtime_from_install(install: &Value) -> Value {
    install
        .pointer("/data/data/dashboardRuntime")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "available": false,
                "reason": "install doctor did not report dashboard runtime manifest",
            })
        })
}

fn runtime_inventory_from_install(install: &Value) -> Value {
    install
        .pointer("/data/data/runtimeInventory")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "available": false,
                "reason": "install doctor did not report runtime inventory",
            })
        })
}

fn runtime_convergence_from_install(install: &Value) -> Value {
    install
        .pointer("/data/data/runtimeConvergence")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "available": false,
                "reason": "install doctor did not report runtime convergence summary",
            })
        })
}

fn readiness_component_ready(value: &Value, component_name: &str) -> bool {
    value
        .pointer("/data/readiness/components")
        .and_then(Value::as_array)
        .and_then(|components| {
            components.iter().find(|component| {
                component.get("component").and_then(Value::as_str) == Some(component_name)
            })
        })
        .and_then(|component| component.get("status"))
        .and_then(Value::as_str)
        == Some("ready")
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => "unknown".to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "unknown".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;

    fn ready_rdp_gateway() -> Value {
        json!({
            "available": true,
            "data": {
                "success": true,
                "readiness": {
                    "components": [
                        {
                            "component": "private_display_allocator",
                            "status": "ready"
                        }
                    ]
                }
            }
        })
    }

    fn duplicate_pressure_only_install() -> Value {
        json!({
            "available": true,
            "success": false,
            "timedOut": false,
            "data": {
                "success": false,
                "data": {
                    "issues": [
                        {
                            "code": "service_duplicate_profile_pressure",
                            "message": "duplicate live browser or profile lease pressure"
                        }
                    ],
                    "serviceResources": {
                        "candidateCount": 0,
                        "readinessImpactingCandidates": 0,
                        "duplicateProfilePressureWarnings": 2
                    }
                }
            }
        })
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "agent-browser-remote-view-doctor-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn write_remote_view_helper_scripts(script_root: &Path) {
        fs::create_dir_all(script_root).unwrap();
        for script in REMOTE_VIEW_HELPER_SCRIPTS {
            fs::write(script_root.join(script), "console.log('{}');\n").unwrap();
        }
    }

    #[test]
    fn doctor_command_large_output_helper() {
        if env::var("AGENT_BROWSER_REMOTE_VIEW_DOCTOR_LARGE_OUTPUT_HELPER").as_deref() == Ok("1") {
            print!("{}", "x".repeat(256 * 1024));
        }
    }

    #[test]
    fn doctor_command_capture_does_not_deadlock_on_large_json_output() {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "remote_view_doctor::tests::doctor_command_large_output_helper",
                "--nocapture",
            ])
            .env("AGENT_BROWSER_REMOTE_VIEW_DOCTOR_LARGE_OUTPUT_HELPER", "1");

        let output = run_command_with_timeout(command, Duration::from_secs(10));

        match output {
            DoctorCommandResult::Output(output) => {
                assert!(output.status.success());
                assert!(output.stdout.len() >= 256 * 1024);
            }
            DoctorCommandResult::Timeout { .. } => {
                panic!("large child output blocked until the doctor timeout")
            }
            DoctorCommandResult::SpawnError(error) => panic!("child command failed: {error}"),
        }
    }

    #[test]
    fn dashboard_runtime_from_install_lifts_manifest() {
        let install = json!({
            "available": true,
            "data": {
                "success": true,
                "data": {
                    "dashboardRuntime": {
                        "schemaVersion": "agent-browser.runtime-manifest.v1",
                        "serviceContractVersion": "service-ui-runtime.v1",
                        "dashboard": {"sha256": "abc"},
                        "executable": {"sha256": "def"}
                    }
                }
            }
        });

        let runtime = dashboard_runtime_from_install(&install);

        assert_eq!(
            runtime["schemaVersion"],
            "agent-browser.runtime-manifest.v1"
        );
        assert_eq!(runtime["serviceContractVersion"], "service-ui-runtime.v1");
        assert_eq!(runtime["dashboard"]["sha256"], "abc");
        assert_eq!(runtime["executable"]["sha256"], "def");
    }

    #[test]
    fn dashboard_runtime_from_install_reports_unavailable_when_missing() {
        let runtime = dashboard_runtime_from_install(&json!({"success": true}));

        assert_eq!(runtime["available"], false);
        assert_eq!(
            runtime["reason"],
            "install doctor did not report dashboard runtime manifest"
        );
    }

    #[test]
    fn runtime_inventory_from_install_lifts_inventory() {
        let install = json!({
            "available": true,
            "data": {
                "success": true,
                "data": {
                    "runtimeInventory": {
                        "schemaVersion": "agent-browser.runtime-inventory.v1",
                        "status": "converged",
                        "runtimeCount": 1
                    }
                }
            }
        });

        let inventory = runtime_inventory_from_install(&install);

        assert_eq!(
            inventory["schemaVersion"],
            "agent-browser.runtime-inventory.v1"
        );
        assert_eq!(inventory["status"], "converged");
        assert_eq!(inventory["runtimeCount"], 1);
    }

    #[test]
    fn runtime_inventory_from_install_reports_unavailable_when_missing() {
        let inventory = runtime_inventory_from_install(&json!({"success": true}));

        assert_eq!(inventory["available"], false);
        assert_eq!(
            inventory["reason"],
            "install doctor did not report runtime inventory"
        );
    }

    #[test]
    fn runtime_convergence_from_install_lifts_summary() {
        let install = json!({
            "available": true,
            "data": {
                "success": true,
                "data": {
                    "runtimeConvergence": {
                        "schemaVersion": "agent-browser.runtime-convergence.v1",
                        "status": "converged"
                    }
                }
            }
        });

        let convergence = runtime_convergence_from_install(&install);

        assert_eq!(
            convergence["schemaVersion"],
            "agent-browser.runtime-convergence.v1"
        );
        assert_eq!(convergence["status"], "converged");
    }

    #[test]
    fn runtime_convergence_from_install_reports_unavailable_when_missing() {
        let convergence = runtime_convergence_from_install(&json!({"success": true}));

        assert_eq!(convergence["available"], false);
        assert_eq!(
            convergence["reason"],
            "install doctor did not report runtime convergence summary"
        );
    }

    #[test]
    fn normalizes_repo_root_to_remote_view_script_root() {
        let root = unique_temp_dir("repo-root");
        let scripts = root.join("scripts");
        write_remote_view_helper_scripts(&scripts);

        let resolved = normalize_script_root_candidate(&root).unwrap();
        assert_eq!(resolved, scripts);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_direct_remote_view_script_root() {
        let scripts = unique_temp_dir("direct-scripts");
        write_remote_view_helper_scripts(&scripts);

        let resolved = normalize_script_root_candidate(&scripts).unwrap();
        assert_eq!(resolved, scripts);

        let _ = fs::remove_dir_all(resolved);
    }

    #[test]
    fn prefers_explicit_installed_remote_view_script_root() {
        let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT"]);
        let scripts = unique_temp_dir("configured-scripts");
        write_remote_view_helper_scripts(&scripts);
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT",
            scripts.to_str().unwrap(),
        );

        let resolved = find_remote_view_script_root().unwrap();
        assert_eq!(resolved, scripts);

        let _ = fs::remove_dir_all(resolved);
    }

    #[test]
    fn discovers_versioned_source_free_support_script_root() {
        let home = unique_temp_dir("installed-support-home");
        let support_root = home
            .join(".local/lib/agent-browser")
            .join(env!("CARGO_PKG_VERSION"));
        let scripts = support_root.join("scripts");
        write_remote_view_helper_scripts(&scripts);

        let resolved =
            find_installed_support_script_root(&home, env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(resolved, scripts);

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn discovers_current_workstation_support_script_root() {
        let home = unique_temp_dir("current-workstation-support-home");
        let scripts = home.join(".local/lib/agent-browser/current/support/scripts");
        write_remote_view_helper_scripts(&scripts);

        let resolved =
            find_installed_support_script_root(&home, env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(resolved, scripts);

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn script_args_use_absolute_helper_path_without_cwd_dependent_prefix() {
        let script_root = PathBuf::from("/tmp/agent-browser-scripts");
        let args = script_args(
            &script_root,
            &[
                "smoke-rdp-guac-route-pool-readiness.js",
                "--report-only",
                "--allow-shared-target",
            ],
        );
        assert_eq!(
            args,
            vec![
                script_root
                    .join("smoke-rdp-guac-route-pool-readiness.js")
                    .display()
                    .to_string(),
                "--report-only".to_string(),
                "--allow-shared-target".to_string(),
            ]
        );
    }

    #[test]
    fn parse_env_keys_ignores_comments_and_values() {
        let keys = parse_env_keys(
            r#"
# comment
XRDP_AGENT_BROWSER_USERNAME=agent-browser-rdp
XRDP_AGENT_BROWSER_PASSWORD=secret
"#,
        );
        assert_eq!(
            keys,
            vec![
                "XRDP_AGENT_BROWSER_USERNAME".to_string(),
                "XRDP_AGENT_BROWSER_PASSWORD".to_string()
            ]
        );
    }

    #[test]
    fn parse_ini_value_reads_selected_section_only() {
        let text = r#"
[Other]
Policy=Bad
[Sessions]
Policy=Default
MaxSessions=50
"#;
        assert_eq!(
            parse_ini_value(text, "Sessions", "Policy"),
            Some("Default".to_string())
        );
        assert_eq!(
            parse_ini_value(text, "Sessions", "MaxSessions"),
            Some("50".to_string())
        );
    }

    #[test]
    fn recommend_next_action_reuses_existing_rdp_user_before_route_users() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({"success": true});
        let route_displays = json!({"data": {"success": false}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": false});
        let users = json!({
            "entries": [
                {"user": "agent-browser-rdp", "exists": true},
                {"user": "agent-browser-rdp-a", "exists": false},
                {"user": "agent-browser-rdp-b", "exists": false}
            ]
        });
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor"
        );
    }

    #[test]
    fn recommend_next_command_marks_route_user_fallback_as_interactive_sudo() {
        let command = recommend_next_command(
            "existing_agent_browser_rdp_routes_collapsed_to_one_display_use_route_specific_user_or_xrdp_policy_isolation",
        );
        assert_eq!(command["command"], "pnpm setup:rdp-guac-route-pool");
        assert_eq!(command["requiresInteractiveSudo"], true);
    }

    #[test]
    fn recommend_next_action_promotes_route_pool_component_action() {
        let route_pool = json!({
            "data": {
                "success": false,
                "readiness": {
                    "components": [
                        {"component": "guacamole_web", "status": "ready"},
                        {
                            "component": "guacamole_login",
                            "status": "failed",
                            "nextAction": "repair_guacamole_admin_credentials"
                        }
                    ]
                }
            }
        });
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({"success": true});
        let route_displays = json!({"data": {"success": false}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": false});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "repair_guacamole_admin_credentials"
        );
    }

    #[test]
    fn recommend_next_action_restarts_stale_daemon_sessions_before_generic_install_drift() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({
            "success": false,
            "data": {
                "data": {
                    "issues": [
                        {
                            "code": "active_runtime_stale_executable",
                            "session": "default"
                        }
                    ]
                }
            }
        });
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": true});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "restart_stale_daemon_sessions_then_rerun_doctor"
        );
    }

    #[test]
    fn recommend_next_action_restarts_stale_stream_backend_before_generic_install_drift() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({
            "success": false,
            "data": {
                "data": {
                    "issues": [
                        {
                            "code": "active_runtime_stale_stream_backend",
                            "session": "default"
                        }
                    ]
                }
            }
        });
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": true});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "restart_stale_daemon_sessions_then_rerun_doctor"
        );
    }

    #[test]
    fn recommend_next_action_converges_stale_dashboard_runtime_before_generic_install_drift() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({
            "success": false,
            "data": {
                "data": {
                    "issues": [
                        {
                            "code": "dashboard_runtime_stale_or_unreadable"
                        }
                    ]
                }
            }
        });
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": true});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "converge_local_runtime_then_rerun_doctor"
        );
    }

    #[test]
    fn recommend_next_action_prefers_known_missing_route_displays_over_helper_timeouts() {
        let timed_out = json!({"available": true, "success": false, "timedOut": true});
        let route_displays = json!({
            "available": true,
            "success": false,
            "timedOut": false,
            "data": {"success": false, "existingUserRoutes": []}
        });
        let users = json!({
            "entries": [
                {"user": "agent-browser-rdp", "exists": true},
                {"user": "agent-browser-rdp-a", "exists": true},
                {"user": "agent-browser-rdp-b", "exists": true}
            ]
        });

        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &timed_out,
                rdp_gateway: &timed_out,
                route_pool: &timed_out,
                route_displays: &route_displays,
                display_access: &json!({"ready": false}),
                viewer_prerequisites: &json!({"ready": true}),
                users: &users,
                privileges: &json!({"ready": true}),
            }),
            "open_route_specific_rdp_sessions_then_rerun_doctor"
        );
    }

    #[test]
    fn remote_control_distinguishes_install_timeout_from_install_drift() {
        let install = json!({"available": true, "success": false, "timedOut": true});
        let status = remote_control_status(
            &install,
            &ready_rdp_gateway(),
            &json!({"data": {"success": false}}),
            &json!({"data": {"success": false}}),
            &json!({"ready": true}),
        );

        assert_eq!(status["installReady"], false);
        assert_eq!(status["installDoctorTimedOut"], true);
        assert_eq!(status["nextAction"], "rerun_install_doctor_after_timeout");
    }

    #[test]
    fn duplicate_pressure_without_readiness_candidates_is_nonblocking_for_remote_control() {
        let install = duplicate_pressure_only_install();
        let route_pool = json!({"data": {"success": false}});
        let route_displays = json!({"data": {"success": false}});
        let viewer_prerequisites = json!({"ready": true});
        let rdp_gateway = ready_rdp_gateway();

        let status = remote_control_status(
            &install,
            &rdp_gateway,
            &route_pool,
            &route_displays,
            &viewer_prerequisites,
        );
        assert_eq!(status["installDoctorReady"], false);
        assert_eq!(status["installReady"], true);
        assert_eq!(
            status["nonBlockingInstallIssueCodes"],
            json!(["service_duplicate_profile_pressure"])
        );
        assert_eq!(
            status["nextAction"],
            "repair_or_sync_guacamole_route_pool_before_creating_more_users"
        );

        let next_action = recommend_next_action(RecommendationContext {
            install: &install,
            rdp_gateway: &rdp_gateway,
            route_pool: &route_pool,
            route_displays: &route_displays,
            display_access: &json!({"ready": false}),
            viewer_prerequisites: &viewer_prerequisites,
            users: &json!({"entries": []}),
            privileges: &json!({"ready": true}),
        });
        assert_eq!(
            next_action,
            "repair_or_sync_guacamole_route_pool_before_creating_more_users"
        );

        let issues = remote_view_issues(RemoteViewIssueContext {
            install: &install,
            rdp_gateway: &rdp_gateway,
            route_pool: &route_pool,
            route_displays: &route_displays,
            display_access: &json!({"ready": false}),
            viewer_prerequisites: &viewer_prerequisites,
            users: &json!({"entries": []}),
            privileges: &json!({
                "groupExists": true,
                "userInGroup": true,
                "helperExists": true,
                "sudoersExists": true,
                "helperCheck": {"success": true},
                "helperDesktopSession": {"ready": true},
                "helperStatus": {
                    "success": true,
                    "parsed": {
                        "routeDesktopSession": {"ready": true},
                        "displayAccess": {
                            "supportsFilesystemX11Socket": true,
                            "supportsAbstractX11Socket": true,
                            "boundedXhostTimeoutSeconds": 2
                        }
                    }
                }
            }),
            next_action: &next_action,
        });
        assert!(!issues.iter().any(|issue| {
            issue.get("code").and_then(Value::as_str)
                == Some("install_service_duplicate_profile_pressure")
        }));
    }

    #[test]
    fn inactive_supervisor_drift_is_nonblocking_for_remote_control() {
        let mut install = duplicate_pressure_only_install();
        let supervisor_issue = json!({
            "code": "executable_drift",
            "message": "inactive supervisor executable drift",
            "severity": "warning"
        });
        install["data"]["data"]["issues"]
            .as_array_mut()
            .unwrap()
            .push(supervisor_issue.clone());
        install["data"]["data"]["sessionSupervisors"] = json!({
            "sessions": [{"activeState": "inactive"}],
            "issues": [supervisor_issue]
        });

        let readiness = remote_control_install_readiness(&install);
        assert!(!readiness.doctor_ready);
        assert!(readiness.ready);
        assert_eq!(
            readiness.non_blocking_issue_codes,
            vec![
                "service_duplicate_profile_pressure".to_string(),
                "executable_drift".to_string()
            ]
        );

        install["data"]["data"]["sessionSupervisors"]["sessions"][0]["activeState"] =
            json!("active");
        assert!(!remote_control_install_readiness(&install).ready);
    }

    #[test]
    fn accepted_manual_runtime_preservation_is_nonblocking_for_remote_control() {
        let mut install = duplicate_pressure_only_install();
        install["data"]["data"]["issues"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "code": "active_runtime_manual_preservation",
                "session": "p0417-preflight",
                "severity": "warning"
            }));

        let readiness = remote_control_install_readiness(&install);
        assert!(!readiness.doctor_ready);
        assert!(readiness.ready);
        assert_eq!(
            readiness.non_blocking_issue_codes,
            vec![
                "service_duplicate_profile_pressure".to_string(),
                "active_runtime_manual_preservation".to_string(),
            ]
        );
    }

    #[test]
    fn remote_control_install_readiness_fails_closed_for_incomplete_or_mixed_evidence() {
        let mut readiness_candidate = duplicate_pressure_only_install();
        readiness_candidate["data"]["data"]["serviceResources"]["readinessImpactingCandidates"] =
            json!(1);

        let mut mixed_issue = duplicate_pressure_only_install();
        mixed_issue["data"]["data"]["issues"] = json!([
            {"code": "service_duplicate_profile_pressure"},
            {"code": "active_runtime_stale_executable"}
        ]);

        let mut missing_candidate_summary = duplicate_pressure_only_install();
        missing_candidate_summary["data"]["data"]
            .as_object_mut()
            .unwrap()
            .remove("serviceResources");

        let mut malformed_issue = duplicate_pressure_only_install();
        malformed_issue["data"]["data"]["issues"] = json!([{"message": "missing code"}]);

        for install in [
            readiness_candidate,
            mixed_issue,
            missing_candidate_summary,
            malformed_issue,
            json!({"available": true, "success": false, "timedOut": true}),
        ] {
            let readiness = remote_control_install_readiness(&install);
            assert!(!readiness.doctor_ready);
            assert!(!readiness.ready);
            assert!(readiness.non_blocking_issue_codes.is_empty());
        }
    }

    #[test]
    fn managed_chrome_sandbox_policy_requires_matching_userns_profile() {
        let chrome = "/home/agent/.agent-browser/browsers/chrome-140/chrome-linux64/chrome";
        let missing = managed_chrome_sandbox_policy_status(
            Some(chrome),
            true,
            true,
            true,
            true,
            Path::new("/etc/apparmor.d/agent-browser-managed-chrome"),
            None,
        );
        assert_eq!(missing["ready"], false);
        assert_eq!(missing["state"], "policy_missing");

        let policy = r#"abi <abi/4.0>,
include <tunables/global>

profile agent-browser-managed-chrome "/home/agent/.agent-browser/browsers/**/chrome" flags=(unconfined) {
  userns,
}
"#;
        let ready = managed_chrome_sandbox_policy_status(
            Some(chrome),
            true,
            true,
            true,
            true,
            Path::new("/etc/apparmor.d/agent-browser-managed-chrome"),
            Some(policy),
        );
        assert_eq!(ready["ready"], true);
        assert_eq!(ready["state"], "ready");
        assert_eq!(ready["policyMatchesManagedChrome"], true);
    }

    #[test]
    fn remote_control_reports_viewer_prerequisite_readiness() {
        let status = remote_control_status(
            &json!({"success": true}),
            &ready_rdp_gateway(),
            &json!({"data": {"success": false}}),
            &json!({"data": {"success": false}}),
            &json!({"ready": false}),
        );

        assert_eq!(status["ready"], false);
        assert_eq!(status["browserLaunchReady"], false);
    }

    #[test]
    fn remote_view_issues_classifies_timeouts_without_claiming_install_drift() {
        let timed_out = json!({"available": true, "success": false, "timedOut": true});
        let route_displays = json!({
            "available": true,
            "success": false,
            "timedOut": false,
            "data": {"success": false}
        });
        let users = json!({
            "entries": [
                {"user": "agent-browser-rdp-a", "exists": true},
                {"user": "agent-browser-rdp-b", "exists": true}
            ]
        });
        let privileges = json!({
            "groupExists": true,
            "userInGroup": true,
            "helperExists": true,
            "sudoersExists": true,
            "helperCheck": {"success": true},
            "helperDesktopSession": {"ready": true},
            "helperStatus": {
                "success": true,
                "parsed": {
                    "schemaVersion": 1,
                    "routeDesktopSession": {"ready": true, "terminalStartupDetected": false},
                    "displayAccess": {
                        "supportsFilesystemX11Socket": true,
                        "supportsAbstractX11Socket": true,
                        "boundedXhostTimeoutSeconds": 2
                    }
                }
            }
        });

        let issues = remote_view_issues(RemoteViewIssueContext {
            install: &timed_out,
            rdp_gateway: &timed_out,
            route_pool: &timed_out,
            route_displays: &route_displays,
            display_access: &json!({"ready": false}),
            viewer_prerequisites: &json!({"ready": true}),
            users: &users,
            privileges: &privileges,
            next_action: "open_route_specific_rdp_sessions_then_rerun_doctor",
        });
        let codes = issues
            .iter()
            .filter_map(|issue| issue.get("code").and_then(Value::as_str))
            .collect::<Vec<_>>();

        assert!(codes.contains(&"install_doctor_timed_out"));
        assert!(codes.contains(&"route_pool_readiness_timed_out"));
        assert!(codes.contains(&"rdp_gateway_readiness_timed_out"));
        assert!(codes.contains(&"route_displays_missing_or_collapsed"));
        assert!(!codes.contains(&"install_doctor_not_ready"));
    }

    #[test]
    fn recommend_next_command_points_stale_daemon_action_at_issue_remedies() {
        let command = recommend_next_command("restart_stale_daemon_sessions_then_rerun_doctor");

        assert_eq!(command["command"], "agent-browser install doctor --json");
        assert_eq!(command["requiresInteractiveSudo"], false);
        assert!(command["why"].as_str().unwrap().contains("remedy.argv"));
    }

    #[test]
    fn recommend_next_command_points_dashboard_runtime_drift_at_convergence_command() {
        let command = recommend_next_command("converge_local_runtime_then_rerun_doctor");

        assert_eq!(
            command["command"],
            "pnpm converge:local-runtime -- --apply --json"
        );
        assert_eq!(command["requiresInteractiveSudo"], false);
    }

    #[test]
    fn recommend_next_action_opens_route_specific_sessions_after_users_exist() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({"success": true});
        let route_displays = json!({"data": {"success": false}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": false});
        let users = json!({
            "entries": [
                {"user": "agent-browser-rdp", "exists": true},
                {"user": "agent-browser-rdp-a", "exists": true},
                {"user": "agent-browser-rdp-b", "exists": true}
            ]
        });
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "open_route_specific_rdp_sessions_then_rerun_doctor"
        );
    }

    #[test]
    fn recommend_next_command_opens_guacamole_route_clients() {
        for next_action in [
            "open_route_specific_rdp_sessions_then_rerun_doctor",
            "open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor",
            "repair_rdp_route_display_session",
        ] {
            let command = recommend_next_command(next_action);
            assert_eq!(command["command"], "pnpm open:rdp-route-displays");
            assert_eq!(command["requiresInteractiveSudo"], false);
        }
    }

    #[test]
    fn recommend_next_action_installs_helper_before_display_access_grant() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({"success": true});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": false});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "install_privileged_helper_then_grant_route_display_access"
        );
    }

    #[test]
    fn recommend_next_action_grants_display_access_after_helper_ready() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({"success": true});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": true});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "grant_route_display_access"
        );
    }

    #[test]
    fn recommend_next_action_installs_helper_after_viewing_is_ready() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let install = json!({"success": true});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let viewer_prerequisites = json!({"ready": true});
        let privileges = json!({"ready": false});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(RecommendationContext {
                install: &install,
                rdp_gateway: &rdp_gateway,
                route_pool: &route_pool,
                route_displays: &route_displays,
                display_access: &display_access,
                viewer_prerequisites: &viewer_prerequisites,
                users: &users,
                privileges: &privileges,
            }),
            "install_privileged_helper_for_recurring_desktop_setup"
        );
    }

    #[test]
    fn many_to_many_requires_route_display_access() {
        let route_pool = json!({"data": {"success": true}});
        let rdp_gateway = ready_rdp_gateway();
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let status = many_to_many_status(
            &rdp_gateway,
            &route_pool,
            &route_displays,
            &display_access,
            &viewer_prerequisites,
        );
        assert_eq!(status["status"], "needs_display_access");
        assert_eq!(status["simultaneousViewingReady"], false);
    }

    #[test]
    fn route_pool_component_issue_codes_use_remote_view_taxonomy() {
        assert_eq!(
            route_pool_component_issue_code("guacamole_schema", "failed"),
            "guacamole_schema_missing"
        );
        assert_eq!(
            route_pool_component_issue_code("guacamole_connection_permissions", "blocked"),
            "guacamole_connection_permission_missing"
        );
        assert_eq!(
            route_pool_component_issue_code("guacamole_login", "failed"),
            "guacamole_login_failed"
        );
        assert_eq!(
            route_pool_component_issue_code("rdp_backend_tcp:1", "failed"),
            "route_pool_rdp_backend_tcp_1_failed"
        );
    }

    #[test]
    fn remote_view_helper_desktop_session_status_rejects_terminal_first_template() {
        let source = r#"
cat > "$home/.xsession" <<'EOF'
#!/bin/sh
xterm &
openbox-session
EOF
"#;

        let status = remote_view_helper_desktop_session_status_from_source(source);

        assert_eq!(status["ready"], false);
        assert_eq!(status["state"], "terminal_first_template");
        assert_eq!(status["terminalStartupDetected"], true);
    }

    #[test]
    fn remote_view_helper_desktop_session_status_accepts_idle_openbox_template() {
        let source = r#"
cat > "$home/.xsession" <<'EOF'
#!/bin/sh
xsetroot -solid '#20252b' 2>/dev/null || true
if command -v openbox-session >/dev/null 2>&1; then
  openbox-session &
fi
while true; do
  sleep 3600
done
EOF
"#;

        let status = remote_view_helper_desktop_session_status_from_source(source);

        assert_eq!(status["ready"], true);
        assert_eq!(status["state"], "browser_control_ready_template");
        assert_eq!(status["terminalStartupDetected"], false);
        assert_eq!(status["startsWindowManager"], true);
        assert_eq!(status["keepsSessionAlive"], true);
    }

    #[test]
    fn remote_view_helper_status_contract_accepts_current_capabilities() {
        let report = json!({
            "success": true,
            "parsed": {
                "schemaVersion": 1,
                "helperVersion": "2026-06-23.p44-route-desktop-v2",
                "routeDesktopSession": {
                    "ready": true,
                    "terminalStartupDetected": false
                },
                "displayAccess": {
                    "supportsFilesystemX11Socket": true,
                    "supportsAbstractX11Socket": true,
                    "boundedXhostTimeoutSeconds": 2
                }
            }
        });

        assert!(remote_view_helper_status_contract_ready(&report));
    }

    #[test]
    fn remote_view_helper_status_contract_rejects_missing_abstract_socket_support() {
        let report = json!({
            "success": true,
            "parsed": {
                "schemaVersion": 1,
                "helperVersion": "2026-06-23.p44-route-desktop-v2",
                "routeDesktopSession": {
                    "ready": true,
                    "terminalStartupDetected": false
                },
                "displayAccess": {
                    "supportsFilesystemX11Socket": true,
                    "supportsAbstractX11Socket": false,
                    "boundedXhostTimeoutSeconds": 2
                }
            }
        });

        assert!(!remote_view_helper_status_contract_ready(&report));
    }

    #[test]
    fn remote_view_issues_reports_stale_remote_view_helper_desktop_template() {
        let install = json!({"success": true});
        let rdp_gateway = ready_rdp_gateway();
        let route_pool = json!({"available": true, "data": {"success": true}});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let viewer_prerequisites = json!({"ready": true});
        let users = json!({"entries": []});
        let privileges = json!({
            "groupExists": true,
            "userInGroup": true,
            "helperExists": true,
            "sudoersExists": true,
            "helperCheck": {"success": true},
            "helperDesktopSession": {"ready": false},
        });

        let issues = remote_view_issues(RemoteViewIssueContext {
            install: &install,
            rdp_gateway: &rdp_gateway,
            route_pool: &route_pool,
            route_displays: &route_displays,
            display_access: &display_access,
            viewer_prerequisites: &viewer_prerequisites,
            users: &users,
            privileges: &privileges,
            next_action: "install_privileged_helper",
        });

        assert!(issues.iter().any(|issue| {
            issue.get("code").and_then(Value::as_str)
                == Some("remote_view_route_desktop_helper_stale")
        }));
    }

    #[test]
    fn remote_view_issues_reports_stale_remote_view_helper_status_contract() {
        let install = json!({"success": true});
        let rdp_gateway = ready_rdp_gateway();
        let route_pool = json!({"available": true, "data": {"success": true}});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let viewer_prerequisites = json!({"ready": true});
        let users = json!({"entries": []});
        let privileges = json!({
            "groupExists": true,
            "userInGroup": true,
            "helperExists": true,
            "sudoersExists": true,
            "helperCheck": {"success": true},
            "helperDesktopSession": {"ready": true},
            "helperStatus": {"success": false},
        });

        let issues = remote_view_issues(RemoteViewIssueContext {
            install: &install,
            rdp_gateway: &rdp_gateway,
            route_pool: &route_pool,
            route_displays: &route_displays,
            display_access: &display_access,
            viewer_prerequisites: &viewer_prerequisites,
            users: &users,
            privileges: &privileges,
            next_action: "install_privileged_helper",
        });

        assert!(issues.iter().any(|issue| {
            issue.get("code").and_then(Value::as_str)
                == Some("remote_view_privileged_helper_status_stale")
        }));
    }

    #[test]
    fn remote_view_issues_reports_guacamole_schema_and_permission_taxonomy() {
        let install = json!({"success": true});
        let rdp_gateway = ready_rdp_gateway();
        let route_pool = json!({
            "available": true,
            "data": {
                "success": false,
                "readiness": {
                    "components": [
                        {
                            "component": "guacamole_schema",
                            "status": "failed",
                            "evidence": "missing Guacamole table(s): guacamole_connection_permission",
                            "nextAction": "initialize_guacamole_schema",
                            "recovery": "Run pnpm ensure:rdp-guac-postgres -- --apply to repair an empty initialized Guacamole PostgreSQL database before validating route-pool entries."
                        },
                        {
                            "component": "guacamole_connection_permissions",
                            "status": "blocked",
                            "evidence": "missing READ permission for Guacamole connection id(s): 1",
                            "nextAction": "repair_guacamole_connection_permissions",
                            "recovery": "Grant READ permission on every selected Guacamole route connection before treating the route pool as ready."
                        }
                    ]
                }
            }
        });
        let route_displays = json!({"data": {"success": false}});
        let display_access = json!({"ready": false});
        let viewer_prerequisites = json!({"ready": true});
        let users = json!({"entries": []});
        let privileges = json!({
            "groupExists": true,
            "userInGroup": true,
            "helperExists": true,
            "sudoersExists": true,
            "helperCheck": {"success": true}
        });
        let issues = remote_view_issues(RemoteViewIssueContext {
            install: &install,
            rdp_gateway: &rdp_gateway,
            route_pool: &route_pool,
            route_displays: &route_displays,
            display_access: &display_access,
            viewer_prerequisites: &viewer_prerequisites,
            users: &users,
            privileges: &privileges,
            next_action: "repair_or_sync_guacamole_route_pool_before_creating_more_users",
        });
        let codes = issues
            .iter()
            .filter_map(|issue| issue.get("code").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(codes.contains(&"guacamole_schema_missing"));
        assert!(codes.contains(&"guacamole_connection_permission_missing"));
    }

    fn scoped_doctor_args(
        session: Option<&str>,
        runtime_profile: Option<&str>,
        route_id: Option<&str>,
    ) -> DoctorArgs {
        DoctorArgs {
            allow_shared_target: false,
            session: session.map(str::to_string),
            runtime_profile: runtime_profile.map(str::to_string),
            route_id: route_id.map(str::to_string),
        }
    }

    fn scoped_service_state(shared_profile: bool) -> ServiceState {
        serde_json::from_value(json!({
            "sessions": {
                "session-a": {"id": "session-a", "profileId": "profile-a", "browserIds": ["browser-a"]},
                "session-b": {"id": "session-b", "profileId": if shared_profile { "profile-a" } else { "profile-b" }, "browserIds": ["browser-b"]}
            },
            "browsers": {
                "browser-a": {"id": "browser-a", "profileId": "profile-a", "activeSessionIds": ["session-a"]},
                "browser-b": {"id": "browser-b", "profileId": if shared_profile { "profile-a" } else { "profile-b" }, "activeSessionIds": ["session-b"]}
            },
            "displayAllocations": {
                "display-a": {"id": "display-a", "state": "ready", "profileId": "profile-a", "ownerSessionId": "session-a"},
                "display-b": {"id": "display-b", "state": "ready", "profileId": if shared_profile { "profile-a" } else { "profile-b" }, "ownerSessionId": "session-b"}
            },
            "routePool": {
                "pool-a": {"id": "pool-a", "routeId": "guacamole:1", "connectionId": "1", "state": "ready"},
                "pool-b": {"id": "pool-b", "routeId": "guacamole:2", "connectionId": "2", "state": "ready"}
            },
            "remoteViewRoutes": {
                "route-a": {
                    "id": "route-a", "state": "ready", "sessionId": "session-a", "browserId": "browser-a",
                    "displayAllocationId": "display-a", "connectionId": "1", "frameUrl": "/guacamole/#/client/a"
                },
                "route-b": {
                    "id": "route-b", "state": "ready", "sessionId": "session-b", "browserId": "browser-b",
                    "displayAllocationId": "display-b", "connectionId": "2", "frameUrl": "/guacamole/#/client/b"
                }
            }
        }))
        .expect("scoped service state")
    }

    #[test]
    fn requested_route_can_be_ready_while_unrelated_global_drift_remains_advisory() {
        let requested = requested_scope_report(
            &scoped_doctor_args(Some("session-a"), Some("profile-a"), Some("guacamole:1")),
            Ok(scoped_service_state(false)),
            &json!({"data": {"success": true}}),
            &json!({"ready": true}),
        );
        let global_issue = json!({
            "code": "runtime_executable_out_of_sync",
            "message": "unrelated stale daemon",
            "sessions": ["unrelated-session"]
        });
        let advisories =
            requested_scope_global_advisories(&requested, std::slice::from_ref(&global_issue));

        assert_eq!(requested["status"], "ready");
        assert_eq!(requested["subject"]["routeId"], "route-a");
        assert_eq!(advisories, vec![global_issue]);
    }

    #[test]
    fn requested_route_accepts_ready_checked_out_pool_entry() {
        let mut state = scoped_service_state(false);
        let pool_entry = state.route_pool.get_mut("pool-a").unwrap();
        pool_entry.state = "checked_out".to_string();
        pool_entry.readiness = Some(json!({"state": "ready"}));

        let requested = requested_scope_report(
            &scoped_doctor_args(Some("session-a"), Some("profile-a"), Some("guacamole:1")),
            Ok(state),
            &json!({"data": {"success": true}}),
            &json!({"ready": true}),
        );

        assert_eq!(requested["status"], "ready");
        assert_eq!(requested["subject"]["routePoolReady"], true);
    }

    #[test]
    fn contradictory_session_and_route_selectors_fail_closed() {
        let requested = requested_scope_report(
            &scoped_doctor_args(Some("session-a"), None, Some("guacamole:2")),
            Ok(scoped_service_state(false)),
            &json!({"data": {"success": true}}),
            &json!({"ready": true}),
        );

        assert_eq!(requested["status"], "degraded");
        assert_eq!(
            requested["issues"][0]["code"],
            "contradictory_requested_scope"
        );
        assert_eq!(requested["subject"], Value::Null);
    }

    #[test]
    fn profile_selector_matching_multiple_routes_is_ambiguous() {
        let requested = requested_scope_report(
            &scoped_doctor_args(None, Some("profile-a"), None),
            Ok(scoped_service_state(true)),
            &json!({"data": {"success": true}}),
            &json!({"ready": true}),
        );

        assert_eq!(requested["status"], "degraded");
        assert_eq!(requested["issues"][0]["code"], "ambiguous_requested_scope");
        assert_eq!(
            requested["candidateRouteIds"],
            json!(["route-a", "route-b"])
        );
    }

    #[test]
    fn selector_parser_rejects_repeated_and_unsafe_values() {
        let repeated = [
            "doctor",
            "remote-view",
            "--route-id",
            "route-a",
            "--route-id",
            "route-b",
        ]
        .map(str::to_string);
        assert!(parse_doctor_args(&repeated).is_err());

        let unsafe_route =
            ["doctor", "remote-view", "--route-id", "../route?bad"].map(str::to_string);
        assert!(parse_doctor_args(&unsafe_route).is_err());
    }
}
