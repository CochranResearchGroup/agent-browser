use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{Read, Write};
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
            if let Some(session) =
                external_chrome_session_from_cmdline(pid, &cmdline, session_names, ports)
            {
                sessions.push(session);
            }
        }
        sessions
    }
}

#[cfg(target_family = "unix")]
fn external_chrome_session_from_cmdline(
    pid: u32,
    cmdline: &[String],
    session_names: &mut HashSet<String>,
    ports: &mut HashSet<u16>,
) -> Option<Value> {
    if !is_browser_process(cmdline) {
        return None;
    }
    let profile_path = command_arg_value(cmdline, "--user-data-dir")?;
    let profile = PathBuf::from(&profile_path);
    if is_agent_browser_owned_profile(&profile) {
        return None;
    }
    let requested_port = command_arg_value(cmdline, "--remote-debugging-port")
        .and_then(|value| value.parse::<u16>().ok());
    let (port, port_source) = resolve_cdp_port(&profile, requested_port)?;
    if port == 0 || ports.contains(&port) {
        return None;
    }
    let cdp_probe = probe_cdp_endpoint(port)?;
    ports.insert(port);
    let session = unique_detected_session_name(&profile, port, session_names);
    session_names.insert(session.clone());
    Some(json!({
        "session": session,
        "port": port,
        "engine": "chrome",
        "provider": "detected-cdp",
        "detected": true,
        "ownership": "foreign_cdp",
        "addressability": "cdp_reachable",
        "cdpPort": port,
        "cdpUrl": format!("http://127.0.0.1:{port}"),
        "profilePath": profile_path,
        "pid": pid,
        "capabilities": {
            "inspect": true,
            "screenshot": true,
            "stream": true,
            "mutateRequiresBorrow": true,
            "lifecycle": false,
        },
        "borrow": {
            "state": "not_borrowed",
            "expiresAt": null,
            "owner": null,
        },
        "source": {
            "kind": "process_scan",
            "pid": pid,
            "cdpPortSource": port_source,
            "probe": cdp_probe,
        },
    }))
}

#[cfg(target_family = "unix")]
fn read_proc_cmdline(pid: u32) -> Option<Vec<String>> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let args = parse_proc_cmdline_bytes(&bytes);
    if args.is_empty() {
        None
    } else {
        Some(args)
    }
}

#[cfg(target_family = "unix")]
fn parse_proc_cmdline_bytes(bytes: &[u8]) -> Vec<String> {
    let args = bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .filter_map(|part| String::from_utf8(part.to_vec()).ok())
        .collect::<Vec<_>>();

    if args.len() == 1 && looks_like_single_string_cmdline(&args[0]) {
        split_single_string_cmdline(&args[0])
    } else {
        args
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
        && command_arg_value(args, "--user-data-dir").is_some()
        && !args.iter().any(|arg| arg.starts_with("--type="))
}

#[cfg(target_family = "unix")]
fn resolve_cdp_port(profile: &Path, requested_port: Option<u16>) -> Option<(u16, &'static str)> {
    match requested_port {
        Some(0) => read_devtools_active_port(profile).map(|port| (port, "devtools_active_port")),
        None => read_devtools_active_port(profile).map(|port| (port, "devtools_active_port")),
        Some(port) => Some((port, "explicit_flag")),
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
fn probe_cdp_endpoint(port: u16) -> Option<Value> {
    let version_ok = http_get_localhost(port, "/json/version").is_some();
    let list_ok = http_get_localhost(port, "/json/list").is_some();
    if version_ok || list_ok {
        Some(json!({
            "jsonVersion": version_ok,
            "jsonList": list_ok,
        }))
    } else {
        None
    }
}

#[cfg(target_family = "unix")]
fn is_agent_browser_owned_profile(profile: &Path) -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return false;
    };
    let agent_browser_root = PathBuf::from(home).join(".agent-browser");
    profile.starts_with(agent_browser_root)
}

#[cfg(target_family = "unix")]
fn http_get_localhost(port: u16, path: &str) -> Option<String> {
    let Ok(addr) = format!("127.0.0.1:{port}").parse() else {
        return None;
    };
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(150)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    for _ in 0..8 {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                if response.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let response = String::from_utf8_lossy(&response);
    if response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200") {
        Some(response.to_string())
    } else {
        None
    }
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

#[cfg(target_family = "unix")]
fn looks_like_single_string_cmdline(value: &str) -> bool {
    value.contains(" --") || value.contains(" --user-data-dir") || value.contains(" --type=")
}

#[cfg(target_family = "unix")]
fn split_single_string_cmdline(value: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }

    if !current.is_empty() {
        args.push(current);
    }
    args
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

#[cfg(all(test, target_family = "unix"))]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_single_string_chrome_cmdline() {
        let raw = b"/opt/google/chrome/chrome --remote-debugging-port=45011 --new-window --user-data-dir=/home/ecochran76/.auracall/browser-profiles/default/chatgpt";
        let args = parse_proc_cmdline_bytes(raw);

        assert_eq!(
            args.first().map(String::as_str),
            Some("/opt/google/chrome/chrome")
        );
        assert_eq!(
            command_arg_value(&args, "--remote-debugging-port").as_deref(),
            Some("45011")
        );
        assert_eq!(
            command_arg_value(&args, "--user-data-dir").as_deref(),
            Some("/home/ecochran76/.auracall/browser-profiles/default/chatgpt")
        );
        assert!(is_browser_process(&args));
    }

    #[test]
    fn parses_nul_separated_chrome_cmdline_and_excludes_renderers() {
        let raw = [
            b"/opt/google/chrome/chrome".as_slice(),
            b"--user-data-dir=/tmp/profile".as_slice(),
            b"--remote-debugging-port".as_slice(),
            b"45013".as_slice(),
        ]
        .join(&0);
        let args = parse_proc_cmdline_bytes(&raw);

        assert_eq!(
            command_arg_value(&args, "--remote-debugging-port").as_deref(),
            Some("45013")
        );
        assert!(is_browser_process(&args));

        let renderer = vec![
            "/opt/google/chrome/chrome".to_string(),
            "--type=renderer".to_string(),
            "--user-data-dir=/tmp/profile".to_string(),
            "--remote-debugging-port=45013".to_string(),
        ];
        assert!(!is_browser_process(&renderer));
    }

    #[test]
    fn resolves_dynamic_port_from_devtools_active_port() {
        let profile = temp_profile_dir("agent-browser-devtools-port");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(
            profile.join("DevToolsActivePort"),
            "37107\n/devtools/browser/test\n",
        )
        .unwrap();

        assert_eq!(
            resolve_cdp_port(&profile, Some(0)),
            Some((37107, "devtools_active_port"))
        );
        assert_eq!(
            resolve_cdp_port(&profile, None),
            Some((37107, "devtools_active_port"))
        );
        assert_eq!(
            resolve_cdp_port(&profile, Some(45011)),
            Some((45011, "explicit_flag"))
        );

        let _ = std::fs::remove_dir_all(profile);
    }

    #[test]
    fn cdp_probe_requires_json_endpoint_success() {
        let (port, handle) = spawn_cdp_probe_server();

        let probe = probe_cdp_endpoint(port).expect("probe should succeed");
        assert_eq!(probe["jsonVersion"], true);
        assert_eq!(probe["jsonList"], true);
        handle.join().unwrap();
    }

    #[test]
    fn builds_foreign_cdp_row_from_auracall_style_cmdline() {
        let (port, handle) = spawn_cdp_probe_server();
        let profile = temp_profile_dir("auracall-chatgpt");
        let raw = format!(
            "/opt/google/chrome/chrome --remote-debugging-port={port} --new-window --user-data-dir={} about:blank",
            profile.display()
        );
        let args = parse_proc_cmdline_bytes(raw.as_bytes());
        let mut session_names = HashSet::new();
        let mut ports = HashSet::new();

        let row = external_chrome_session_from_cmdline(4242, &args, &mut session_names, &mut ports)
            .expect("foreign CDP row");

        assert_eq!(row["detected"], true);
        assert_eq!(row["provider"], "detected-cdp");
        assert_eq!(row["ownership"], "foreign_cdp");
        assert_eq!(row["addressability"], "cdp_reachable");
        assert_eq!(row["cdpPort"], port);
        assert_eq!(row["pid"], 4242);
        assert_eq!(row["capabilities"]["stream"], true);
        assert_eq!(row["capabilities"]["mutateRequiresBorrow"], true);
        assert_eq!(row["capabilities"]["lifecycle"], false);
        assert_eq!(row["source"]["cdpPortSource"], "explicit_flag");
        handle.join().unwrap();
    }

    #[test]
    fn builds_foreign_cdp_row_from_devtools_active_port() {
        let (port, handle) = spawn_cdp_probe_server();
        let profile = temp_profile_dir("im-receipts-google-messages");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(
            profile.join("DevToolsActivePort"),
            format!("{port}\n/devtools/browser/test\n"),
        )
        .unwrap();
        let args = vec![
            "/opt/google/chrome/chrome".to_string(),
            format!("--user-data-dir={}", profile.display()),
            "--remote-debugging-port=0".to_string(),
            "https://messages.google.com/web/".to_string(),
        ];
        let mut session_names = HashSet::new();
        let mut ports = HashSet::new();

        let row = external_chrome_session_from_cmdline(5151, &args, &mut session_names, &mut ports)
            .expect("foreign CDP row");

        assert_eq!(row["ownership"], "foreign_cdp");
        assert_eq!(row["addressability"], "cdp_reachable");
        assert_eq!(row["cdpPort"], port);
        assert_eq!(row["source"]["cdpPortSource"], "devtools_active_port");
        assert!(row["session"]
            .as_str()
            .unwrap_or("")
            .starts_with("detected-"));

        handle.join().unwrap();
        let _ = std::fs::remove_dir_all(profile);
    }

    #[test]
    fn skips_agent_browser_owned_profiles() {
        let profile = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap()
            .join(".agent-browser/runtime-profiles/default/user-data");
        let args = vec![
            "/opt/google/chrome/chrome".to_string(),
            format!("--user-data-dir={}", profile.display()),
            "--remote-debugging-port=45011".to_string(),
        ];
        let mut session_names = HashSet::new();
        let mut ports = HashSet::new();

        assert!(
            external_chrome_session_from_cmdline(6161, &args, &mut session_names, &mut ports)
                .is_none()
        );
    }

    fn temp_profile_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{label}-{stamp}"))
    }

    fn spawn_cdp_probe_server() -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                let body = "{}";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (port, handle)
    }
}
