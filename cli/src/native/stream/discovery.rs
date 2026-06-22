use serde_json::{json, Value};
use std::collections::HashSet;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::connection::get_socket_dir;

pub(super) fn discover_sessions() -> String {
    let dir = get_socket_dir();
    let mut sessions = Vec::new();
    let mut session_names = HashSet::new();
    let mut ports = HashSet::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(session) = name_str.strip_suffix(".stream") {
                if let Ok(port_str) = std::fs::read_to_string(entry.path()) {
                    if let Ok(port) = port_str.trim().parse::<u16>() {
                        let pid_path = dir.join(format!("{}.pid", session));
                        if is_process_alive(&pid_path) {
                            let engine_path = dir.join(format!("{}.engine", session));
                            let engine = std::fs::read_to_string(&engine_path)
                                .ok()
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or_else(|| "chrome".to_string());

                            let provider_path = dir.join(format!("{}.provider", session));
                            let provider = std::fs::read_to_string(&provider_path)
                                .ok()
                                .filter(|s| !s.trim().is_empty());

                            let extensions = read_extensions_metadata(&dir, session);

                            let mut entry = json!({
                                "session": session,
                                "port": port,
                                "engine": engine.trim(),
                            });
                            if let Some(ref p) = provider {
                                entry["provider"] = json!(p.trim());
                            }
                            if !extensions.is_empty() {
                                entry["extensions"] = json!(extensions);
                            }
                            session_names.insert(session.to_string());
                            ports.insert(port);
                            sessions.push(entry);
                        } else {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    for detected in discover_external_chrome_sessions(&mut session_names, &mut ports) {
        sessions.push(detected);
    }

    serde_json::to_string(&sessions).unwrap_or_else(|_| "[]".to_string())
}

fn discover_external_chrome_sessions(
    session_names: &mut HashSet<String>,
    ports: &mut HashSet<u16>,
) -> Vec<Value> {
    let mut sessions = Vec::new();
    #[cfg(not(target_family = "unix"))]
    {
        let _ = session_names;
        let _ = ports;
        return sessions;
    }
    #[cfg(target_family = "unix")]
    {
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return sessions;
        };
        for entry in entries.flatten() {
            let pid = match entry.file_name().to_string_lossy().parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };
            let cmdline = match read_proc_cmdline(pid) {
                Some(cmdline) => cmdline,
                None => continue,
            };
            if !is_browser_process(&cmdline) {
                continue;
            }
            let Some(profile_path) = command_arg_value(&cmdline, "--user-data-dir") else {
                continue;
            };
            let profile = PathBuf::from(&profile_path);
            let requested_port = command_arg_value(&cmdline, "--remote-debugging-port")
                .and_then(|value| value.parse::<u16>().ok());
            let Some(port) = resolve_cdp_port(&profile, requested_port) else {
                continue;
            };
            if port == 0 || ports.contains(&port) || !is_port_reachable(port) {
                continue;
            }
            ports.insert(port);
            let session = unique_detected_session_name(&profile, port, session_names);
            session_names.insert(session.clone());
            sessions.push(json!({
                "session": session,
                "port": port,
                "engine": "chrome",
                "provider": "detected-cdp",
                "detected": true,
                "cdpPort": port,
                "profilePath": profile_path,
                "pid": pid,
            }));
        }
        sessions
    }
}

#[cfg(target_family = "unix")]
fn read_proc_cmdline(pid: u32) -> Option<Vec<String>> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let args = bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .filter_map(|part| String::from_utf8(part.to_vec()).ok())
        .collect::<Vec<_>>();
    if args.is_empty() {
        None
    } else {
        Some(args)
    }
}

#[cfg(target_family = "unix")]
fn is_browser_process(args: &[String]) -> bool {
    let Some(executable) = args.first() else {
        return false;
    };
    let executable_name = Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(executable);
    let browser_binary = executable_name.contains("chrome") || executable_name.contains("chromium");
    browser_binary
        && args.iter().any(|arg| arg.starts_with("--user-data-dir"))
        && !args.iter().any(|arg| arg.starts_with("--type="))
}

#[cfg(target_family = "unix")]
fn resolve_cdp_port(profile: &Path, requested_port: Option<u16>) -> Option<u16> {
    match requested_port {
        Some(0) | None => read_devtools_active_port(profile),
        Some(port) => Some(port),
    }
}

#[cfg(target_family = "unix")]
fn read_devtools_active_port(profile: &Path) -> Option<u16> {
    let content = std::fs::read_to_string(profile.join("DevToolsActivePort")).ok()?;
    content.lines().next()?.trim().parse::<u16>().ok()
}

#[cfg(target_family = "unix")]
fn command_arg_value(args: &[String], name: &str) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            return Some(value.to_string());
        }
        if arg == name {
            return args.get(index + 1).cloned();
        }
    }
    None
}

#[cfg(target_family = "unix")]
fn is_port_reachable(port: u16) -> bool {
    let Ok(addr) = format!("127.0.0.1:{port}").parse() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

#[cfg(target_family = "unix")]
fn unique_detected_session_name(profile: &Path, port: u16, existing: &HashSet<String>) -> String {
    let label = profile
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("chrome");
    let base = format!("detected-{}-{port}", sanitize_session_part(label));
    if !existing.contains(&base) {
        return base;
    }
    let mut index = 2;
    loop {
        let candidate = format!("{base}-{index}");
        if !existing.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(target_family = "unix")]
fn sanitize_session_part(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    sanitized.trim_matches('-').to_string()
}

fn read_extensions_metadata(dir: &std::path::Path, session: &str) -> Vec<Value> {
    let ext_path = dir.join(format!("{}.extensions", session));
    let ext_str = match std::fs::read_to_string(&ext_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    ext_str
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .filter_map(|path| {
            let manifest_path = std::path::Path::new(path).join("manifest.json");
            let manifest_str = std::fs::read_to_string(&manifest_path).ok()?;
            let manifest: Value = serde_json::from_str(&manifest_str).ok()?;

            let name = manifest
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let version = manifest
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let description = manifest
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut ext = json!({
                "name": name,
                "version": version,
                "path": path,
            });
            if let Some(desc) = description {
                ext["description"] = json!(desc);
            }
            Some(ext)
        })
        .collect()
}

fn is_process_alive(pid_path: &Path) -> bool {
    let pid_str = match std::fs::read_to_string(pid_path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}
