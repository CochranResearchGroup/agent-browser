use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::color;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorArgs {
    allow_shared_target: bool,
}

/// Run the read-only remote-view doctor. This command inventories existing
/// install, Guacamole, XRDP, user, and route-display state before setup helpers
/// are allowed to suggest creating users or mutating configuration.
pub fn run_remote_view_doctor(clean: &[String], json_mode: bool) {
    let args = parse_doctor_args(clean);
    let report = remote_view_doctor_report(&args);

    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| {
                r#"{"success":false,"error":"Failed to serialize remote-view doctor"}"#.to_string()
            })
        );
        return;
    }

    print_remote_view_doctor_report(&report);
}

fn parse_doctor_args(clean: &[String]) -> DoctorArgs {
    DoctorArgs {
        allow_shared_target: clean.iter().any(|arg| arg == "--allow-shared-target"),
    }
}

fn remote_view_doctor_report(args: &DoctorArgs) -> Value {
    let install = run_json_command(
        current_agent_browser_command(),
        &["install", "doctor", "--json"],
        None,
    );
    let route_pool_args = if args.allow_shared_target {
        vec![
            "scripts/smoke-rdp-guac-route-pool-readiness.js",
            "--report-only",
            "--allow-shared-target",
        ]
    } else {
        vec![
            "scripts/smoke-rdp-guac-route-pool-readiness.js",
            "--report-only",
        ]
    };
    let route_pool = run_json_command("node".to_string(), &route_pool_args, Some(repo_root()));
    let route_displays = run_json_command(
        "node".to_string(),
        &["scripts/inspect-rdp-route-displays.js"],
        Some(repo_root()),
    );
    let xrdp = inspect_xrdp();
    let users = inspect_rdp_users();
    let privileges = inspect_privileges();
    let config = inspect_remote_view_config();
    let display_access = inspect_route_display_access(&route_displays);
    let many_to_many = many_to_many_status(&route_pool, &route_displays, &display_access);
    let next_action = recommend_next_action(
        &route_pool,
        &route_displays,
        &display_access,
        &users,
        &privileges,
    );
    let next_command = recommend_next_command(&next_action);
    let drift = drift_findings(&users, &config);

    json!({
        "success": true,
        "data": {
            "status": many_to_many["status"].clone(),
            "install": install,
            "runtime": inspect_runtime(),
            "network": inspect_network(),
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
            "manyToMany": many_to_many,
            "config": config,
            "drift": drift,
            "stateSources": state_sources(),
            "nextAction": next_action,
            "nextCommand": next_command,
        }
    })
}

fn repo_root() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn current_agent_browser_command() -> String {
    env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "agent-browser".to_string())
}

fn run_json_command(command: String, args: &[&str], cwd: Option<PathBuf>) -> Value {
    let mut child = Command::new(&command);
    child.args(args);
    if let Some(cwd) = cwd {
        child.current_dir(cwd);
    }
    let output = child.output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let parsed = serde_json::from_str::<Value>(stdout.trim()).ok();
            json!({
                "available": true,
                "success": output.status.success(),
                "exitCode": output.status.code(),
                "command": format!("{} {}", command, args.join(" ")),
                "data": parsed,
                "stderr": redact_text(stderr.trim()),
            })
        }
        Err(error) => json!({
            "available": false,
            "success": false,
            "exitCode": null,
            "command": format!("{} {}", command, args.join(" ")),
            "data": null,
            "stderr": redact_text(error.to_string()),
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
    let output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", name])
        .output()
        .ok()?;
    if !output.status.success() {
        return Some(false);
    }
    Some(String::from_utf8_lossy(&output.stdout).trim() == "true")
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
    let helper_check = run_text_command("sudo", &["-n", &helper_path, "check"]);

    json!({
        "groupName": group_name,
        "currentUser": current_user,
        "groupExists": group_exists,
        "userInGroup": user_in_group,
        "helperPath": helper_path,
        "helperExists": helper_exists,
        "helperCheck": helper_check,
        "ready": group_exists && user_in_group && helper_exists && helper_check["success"].as_bool() == Some(true),
    })
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
    route_pool: &Value,
    route_displays: &Value,
    display_access: &Value,
) -> Value {
    let route_pool_ready = nested_bool(route_pool, &["data", "success"]);
    let display_ready = nested_bool(route_displays, &["data", "success"]);
    let display_access_ready = display_access
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if route_pool_ready && display_ready && display_access_ready {
        "ready"
    } else if route_pool_ready && display_ready {
        "needs_display_access"
    } else if route_pool_ready {
        "needs_route_displays"
    } else {
        "blocked"
    };
    json!({
        "status": status,
        "routePoolReady": route_pool_ready,
        "routeDisplaysReady": display_ready,
        "routeDisplayAccessReady": display_access_ready,
        "simultaneousViewingReady": route_pool_ready && display_ready && display_access_ready,
    })
}

fn nested_bool(value: &Value, path: &[&str]) -> bool {
    let mut current = value;
    for key in path {
        current = current.get(*key).unwrap_or(&Value::Null);
    }
    current.as_bool().unwrap_or(false)
}

fn recommend_next_action(
    route_pool: &Value,
    route_displays: &Value,
    display_access: &Value,
    users: &Value,
    privileges: &Value,
) -> String {
    if !nested_bool(route_pool, &["data", "success"]) {
        return "repair_or_sync_guacamole_route_pool_before_creating_more_users".to_string();
    }
    if !nested_bool(route_displays, &["data", "success"]) {
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
            return "open_route_specific_rdp_sessions_then_rerun_doctor".to_string();
        }
        let existing_route_count = route_displays
            .pointer("/data/existingUserRoutes")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if existing_route_count == 1 {
            return "existing_agent_browser_rdp_routes_collapsed_to_one_display_use_route_specific_user_or_xrdp_policy_isolation".to_string();
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
            return "open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor".to_string();
        }
        return "create_or_repair_single_agent_browser_rdp_user_before_route_specific_users"
            .to_string();
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
    "run_many_to_many_live_gate".to_string()
}

fn recommend_next_command(next_action: &str) -> Value {
    match next_action {
        "repair_or_sync_guacamole_route_pool_before_creating_more_users" => json!({
            "command": "pnpm test:rdp-guac-route-pool-readiness -- --report-only",
            "requiresInteractiveSudo": false,
            "why": "Inspect the current Guacamole route-pool state before mutating users or records."
        }),
        "existing_agent_browser_rdp_routes_collapsed_to_one_display_use_route_specific_user_or_xrdp_policy_isolation" => json!({
            "command": "pnpm setup:rdp-guac-route-pool",
            "requiresInteractiveSudo": true,
            "why": "The existing-user Guacamole route shape already collapsed to one XRDP display; create the explicit isolated route users from an interactive terminal."
        }),
        "open_route_specific_rdp_sessions_then_rerun_doctor" => json!({
            "command": "pnpm inspect:rdp-route-displays",
            "requiresInteractiveSudo": false,
            "why": "Route-specific users exist; open both route-specific Guacamole connections, then inspect whether XRDP allocated distinct displays."
        }),
        "open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor" => json!({
            "command": "pnpm inspect:rdp-route-displays",
            "requiresInteractiveSudo": false,
            "why": "Open both existing-user Guacamole route sessions, then inspect whether XRDP allocated distinct displays."
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
        "run_many_to_many_live_gate" => json!({
            "command": "pnpm test:rdp-guac-many-to-many-live",
            "requiresInteractiveSudo": false,
            "why": "Route pool and route displays are ready; run the OCR-backed many-to-many gate."
        }),
        _ => json!({
            "command": "agent-browser doctor remote-view",
            "requiresInteractiveSudo": false,
            "why": "Re-run the doctor after resolving the reported state."
        }),
    }
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
        json!({"source": "scripts/smoke-rdp-guac-route-pool-readiness.js --report-only", "kind": "guacamole"}),
        json!({"source": "scripts/inspect-rdp-route-displays.js", "kind": "rdp-displays"}),
        json!({"source": "xdpyinfo with route display names", "kind": "rdp-display-access"}),
        json!({"source": "getent group agent-browser, id -nG, and sudo -n privileged helper check", "kind": "privileges"}),
        json!({"source": "/etc/xrdp/sesman.ini", "kind": "rdp-host"}),
        json!({"source": "getent passwd agent-browser-rdp agent-browser-rdp-a agent-browser-rdp-b", "kind": "users"}),
        json!({"source": "~/.agent-browser/.env and ~/.agent-browser/secrets/guacamole.env", "kind": "config"}),
    ]
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
    let output = child.output();
    match output {
        Ok(output) => json!({
            "available": true,
            "success": output.status.success(),
            "exitCode": output.status.code(),
            "stdout": redact_text(String::from_utf8_lossy(&output.stdout).trim()),
            "stderr": redact_text(String::from_utf8_lossy(&output.stderr).trim()),
        }),
        Err(error) => json!({
            "available": false,
            "success": false,
            "exitCode": null,
            "stdout": "",
            "stderr": redact_text(error.to_string()),
        }),
    }
}

fn run_display_access_probe(display_name: &str) -> Value {
    let mut child = Command::new("xdpyinfo");
    child.env("DISPLAY", display_name);
    match child.output() {
        Ok(output) => {
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
                "exitCode": output.status.code(),
                "stdout": evidence,
                "stderr": redact_text(String::from_utf8_lossy(&output.stderr).trim()),
            })
        }
        Err(error) => json!({
            "available": false,
            "success": false,
            "exitCode": null,
            "stdout": "",
            "stderr": redact_text(error.to_string()),
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
        "simultaneous viewing ready: {}",
        display_value(&data["manyToMany"]["simultaneousViewingReady"])
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
        let route_displays = json!({"data": {"success": false}});
        let display_access = json!({"ready": false});
        let privileges = json!({"ready": false});
        let users = json!({
            "entries": [
                {"user": "agent-browser-rdp", "exists": true},
                {"user": "agent-browser-rdp-a", "exists": false},
                {"user": "agent-browser-rdp-b", "exists": false}
            ]
        });
        assert_eq!(
            recommend_next_action(
                &route_pool,
                &route_displays,
                &display_access,
                &users,
                &privileges
            ),
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
    fn recommend_next_action_opens_route_specific_sessions_after_users_exist() {
        let route_pool = json!({"data": {"success": true}});
        let route_displays = json!({"data": {"success": false}});
        let display_access = json!({"ready": false});
        let privileges = json!({"ready": false});
        let users = json!({
            "entries": [
                {"user": "agent-browser-rdp", "exists": true},
                {"user": "agent-browser-rdp-a", "exists": true},
                {"user": "agent-browser-rdp-b", "exists": true}
            ]
        });
        assert_eq!(
            recommend_next_action(
                &route_pool,
                &route_displays,
                &display_access,
                &users,
                &privileges
            ),
            "open_route_specific_rdp_sessions_then_rerun_doctor"
        );
    }

    #[test]
    fn recommend_next_action_installs_helper_before_display_access_grant() {
        let route_pool = json!({"data": {"success": true}});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": false});
        let privileges = json!({"ready": false});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(
                &route_pool,
                &route_displays,
                &display_access,
                &users,
                &privileges
            ),
            "install_privileged_helper_then_grant_route_display_access"
        );
    }

    #[test]
    fn recommend_next_action_grants_display_access_after_helper_ready() {
        let route_pool = json!({"data": {"success": true}});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": false});
        let privileges = json!({"ready": true});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(
                &route_pool,
                &route_displays,
                &display_access,
                &users,
                &privileges
            ),
            "grant_route_display_access"
        );
    }

    #[test]
    fn recommend_next_action_installs_helper_after_viewing_is_ready() {
        let route_pool = json!({"data": {"success": true}});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": true});
        let privileges = json!({"ready": false});
        let users = json!({"entries": []});
        assert_eq!(
            recommend_next_action(
                &route_pool,
                &route_displays,
                &display_access,
                &users,
                &privileges
            ),
            "install_privileged_helper_for_recurring_desktop_setup"
        );
    }

    #[test]
    fn many_to_many_requires_route_display_access() {
        let route_pool = json!({"data": {"success": true}});
        let route_displays = json!({"data": {"success": true}});
        let display_access = json!({"ready": false});
        let status = many_to_many_status(&route_pool, &route_displays, &display_access);
        assert_eq!(status["status"], "needs_display_access");
        assert_eq!(status["simultaneousViewingReady"], false);
    }
}
