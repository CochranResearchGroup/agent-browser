use serde_json::{json, Value};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::connection::get_socket_dir;

use super::chat::{chat_status_json, handle_chat_request, handle_models_request};
use super::discovery::discover_sessions;
use super::http::{serve_embedded_file, CORS_HEADERS};

pub async fn run_dashboard_server(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind dashboard server on {}: {}", addr, e);
            return;
        }
    };

    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            break;
        };
        tokio::spawn(async move {
            handle_dashboard_connection(stream).await;
        });
    }
}

async fn handle_dashboard_connection(mut stream: tokio::net::TcpStream) {
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let header_str = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let first_line = header_str.lines().next().unwrap_or("").to_string();
    let method = first_line.split_whitespace().next().unwrap_or("GET");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let origin = header_str.lines().find_map(|line| {
        if line.len() > 8 && line[..8].eq_ignore_ascii_case("origin: ") {
            Some(line[8..].trim().to_string())
        } else {
            None
        }
    });

    if method == "OPTIONS" {
        let response = format!(
            "HTTP/1.1 204 No Content\r\n{CORS_HEADERS}Access-Control-Max-Age: 86400\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/chat" {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        handle_chat_request(&mut stream, &body_str, origin.as_deref()).await;
        return;
    }

    if method == "GET" && path == "/api/models" {
        handle_models_request(&mut stream, origin.as_deref()).await;
        return;
    }

    if path == "/api/service" || path.starts_with("/api/service/") {
        let body_str = if matches!(method, "POST" | "PUT" | "PATCH" | "DELETE") {
            read_post_body(&mut stream, &buf, n).await
        } else {
            String::new()
        };
        handle_service_api_request(&mut stream, method, path, &body_str).await;
        return;
    }

    if method == "POST" && (path == "/api/sessions" || path == "/api/exec" || path == "/api/kill") {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        let result = if path == "/api/exec" {
            exec_cli(&body_str).await
        } else if path == "/api/kill" {
            kill_session(&body_str).await
        } else {
            spawn_session(&body_str).await
        };
        let (status, resp_body) = match result {
            Ok(msg) => ("200 OK", msg),
            Err(e) => (
                "400 Bad Request",
                format!(
                    r#"{{"success":false,"error":{}}}"#,
                    serde_json::to_string(&e).unwrap_or_else(|_| format!("\"{}\"", e))
                ),
            ),
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
            resp_body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.write_all(resp_body.as_bytes()).await;
        return;
    }

    let (status, content_type, body): (&str, &str, Vec<u8>) = if path == "/api/sessions" {
        (
            "200 OK",
            "application/json; charset=utf-8",
            discover_sessions().into_bytes(),
        )
    } else if path == "/api/chat/status" {
        (
            "200 OK",
            "application/json; charset=utf-8",
            chat_status_json().into_bytes(),
        )
    } else {
        serve_embedded_file(path)
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        status,
        content_type,
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(&body).await;
}

async fn handle_service_api_request(
    stream: &mut tokio::net::TcpStream,
    method: &str,
    path: &str,
    body: &str,
) {
    if let Some(port) = dashboard_service_backend_port() {
        match proxy_service_api_request(port, method, path, body).await {
            Ok(response) => {
                let _ = stream.write_all(&response).await;
                return;
            }
            Err(err) => {
                if let Some(response) = service_api_cli_fallback(method, path).await {
                    let _ = stream.write_all(response.as_bytes()).await;
                    return;
                }
                write_json_error(
                    stream,
                    "502 Bad Gateway",
                    &format!("Service API proxy failed: {}", err),
                )
                .await;
                return;
            }
        }
    }

    if let Some(response) = service_api_cli_fallback(method, path).await {
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    write_json_error(
        stream,
        "503 Service Unavailable",
        "No agent-browser session is available to handle service API requests",
    )
    .await;
}

fn dashboard_service_backend_port() -> Option<u16> {
    let sessions: Value = serde_json::from_str(&discover_sessions()).ok()?;
    let sessions = sessions.as_array()?;
    sessions
        .iter()
        .find(|session| session.get("session").and_then(Value::as_str) == Some("default"))
        .or_else(|| sessions.first())
        .and_then(|session| session.get("port"))
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
}

async fn proxy_service_api_request(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
) -> Result<Vec<u8>, String> {
    let mut backend = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|err| err.to_string())?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    backend
        .write_all(request.as_bytes())
        .await
        .map_err(|err| err.to_string())?;
    let mut response = Vec::new();
    backend
        .read_to_end(&mut response)
        .await
        .map_err(|err| err.to_string())?;
    Ok(response)
}

async fn service_api_cli_fallback(method: &str, path: &str) -> Option<String> {
    if method != "GET" && !(method == "POST" && path == "/api/service/reconcile") {
        return None;
    }

    let (raw_path, query) = split_path_query(path);
    let mut args = match (method, raw_path) {
        ("GET", "/api/service/status") => vec!["service".to_string(), "status".to_string()],
        ("GET", "/api/service/jobs") => vec!["service".to_string(), "jobs".to_string()],
        ("GET", "/api/service/events") => vec!["service".to_string(), "events".to_string()],
        ("GET", "/api/service/incidents") => {
            let mut args = vec!["service".to_string(), "incidents".to_string()];
            if query_value(query, "summary").as_deref() == Some("true") {
                args.push("--summary".to_string());
            }
            args
        }
        ("GET", "/api/service/trace") => vec!["service".to_string(), "trace".to_string()],
        ("POST", "/api/service/reconcile") => vec!["service".to_string(), "reconcile".to_string()],
        _ => {
            let incident_activity_id = raw_path
                .strip_prefix("/api/service/incidents/")
                .and_then(|rest| rest.strip_suffix("/activity"));
            if let Some(incident_id) = incident_activity_id.filter(|id| !id.is_empty()) {
                vec![
                    "service".to_string(),
                    "activity".to_string(),
                    incident_id.to_string(),
                ]
            } else {
                return None;
            }
        }
    };

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => args.extend(["--limit".to_string(), value]),
            "kind" => args.extend(["--kind".to_string(), value]),
            "state" => args.extend(["--state".to_string(), value]),
            "severity" => args.extend(["--severity".to_string(), value]),
            "escalation" => args.extend(["--escalation".to_string(), value]),
            "handling-state" | "handlingState" => {
                args.extend(["--handling-state".to_string(), value])
            }
            "browser-id" | "browserId" => args.extend(["--browser-id".to_string(), value]),
            "profile-id" | "profileId" => args.extend(["--profile-id".to_string(), value]),
            "session-id" | "sessionId" => args.extend(["--session-id".to_string(), value]),
            "service-name" | "serviceName" => args.extend(["--service-name".to_string(), value]),
            "agent-name" | "agentName" => args.extend(["--agent-name".to_string(), value]),
            "task-name" | "taskName" => args.extend(["--task-name".to_string(), value]),
            "since" => args.extend(["--since".to_string(), value]),
            _ => {}
        }
    }

    exec_agent_browser_args(args).await.ok().map(|body| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
            body.len()
        ) + &body
    })
}

fn split_path_query(raw_path: &str) -> (&str, Option<&str>) {
    match raw_path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (raw_path, None),
    }
}

fn query_value(query: Option<&str>, expected_key: &str) -> Option<String> {
    query_params(query)
        .into_iter()
        .find_map(|(key, value)| (key == expected_key).then_some(value))
}

fn query_params(query: Option<&str>) -> Vec<(String, String)> {
    query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((decode_query_component(key)?, decode_query_component(value)?))
        })
        .collect()
}

fn decode_query_component(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut chars = value.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let high = chars.next()?;
                let low = chars.next()?;
                let high = hex_value(high)?;
                let low = hex_value(low)?;
                bytes.push((high << 4) | low);
            }
            _ => bytes.push(byte),
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

async fn exec_agent_browser_args(args: Vec<String>) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Cannot resolve executable: {}", e))?;
    let output = tokio::process::Command::new(&exe)
        .args(args)
        .arg("--json")
        .env_remove("AGENT_BROWSER_DASHBOARD")
        .env_remove("AGENT_BROWSER_DASHBOARD_PORT")
        .env_remove("AGENT_BROWSER_STREAM_PORT")
        .output()
        .await
        .map_err(|e| format!("Failed to execute: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() || !stdout.is_empty() {
        Ok(stdout)
    } else {
        Err(stderr)
    }
}

async fn write_json_error(stream: &mut tokio::net::TcpStream, status: &str, error: &str) {
    let body = json!({
        "success": false,
        "error": error,
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(body.as_bytes()).await;
}

async fn read_post_body(stream: &mut tokio::net::TcpStream, initial: &[u8], n: usize) -> String {
    use tokio::io::AsyncReadExt;

    let header_end = initial[..n]
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| {
            initial[..n]
                .windows(2)
                .position(|w| w == b"\n\n")
                .map(|p| p + 2)
        });
    let Some(header_end) = header_end else {
        return String::new();
    };

    let header_str = String::from_utf8_lossy(&initial[..header_end]);
    let content_length: usize = header_str
        .lines()
        .find_map(|l| {
            if l.len() > 16 && l[..16].eq_ignore_ascii_case("content-length: ") {
                l[16..].trim().parse().ok()
            } else {
                let lower = l.to_lowercase();
                lower
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse().ok())
            }
        })
        .unwrap_or(0);

    if content_length == 0 {
        return String::new();
    }

    let read_body = &initial[header_end..n];
    let already_read = read_body.len().min(content_length);

    let mut body = Vec::with_capacity(content_length);
    body.extend_from_slice(&read_body[..already_read]);

    let remaining = content_length - already_read;
    if remaining > 0 {
        let mut rest = vec![0u8; remaining];
        if stream.read_exact(&mut rest).await.is_ok() {
            body.extend_from_slice(&rest);
        }
    }

    String::from_utf8(body).unwrap_or_default()
}

async fn exec_cli(body: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;
    let args: Vec<String> = parsed
        .get("args")
        .and_then(|v| v.as_array())
        .ok_or("Missing \"args\" array")?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    if args.is_empty() {
        return Err("Empty args array".to_string());
    }

    let exe = std::env::current_exe().map_err(|e| format!("Cannot resolve executable: {}", e))?;

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.args(&args)
        .arg("--json")
        .env_remove("AGENT_BROWSER_DASHBOARD")
        .env_remove("AGENT_BROWSER_DASHBOARD_PORT")
        .env_remove("AGENT_BROWSER_STREAM_PORT");

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to execute: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    Ok(json!({
        "success": output.status.success(),
        "exit_code": output.status.code(),
        "stdout": stdout,
        "stderr": stderr,
    })
    .to_string())
}

async fn kill_session(body: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;
    let session = parsed
        .get("session")
        .and_then(|v| v.as_str())
        .ok_or("Missing \"session\" field")?;

    if session.is_empty() || session.len() > 64 {
        return Err("Session name must be 1-64 characters".to_string());
    }

    let dir = get_socket_dir();
    let pid_path = dir.join(format!("{}.pid", session));

    let pid_str = std::fs::read_to_string(&pid_path)
        .map_err(|_| format!("No PID file for session '{}'", session))?;
    let pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|_| format!("Invalid PID in file: {}", pid_str.trim()))?;

    #[cfg(unix)]
    {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if unsafe { libc::kill(pid as i32, 0) } == 0 {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }

    for ext in &["pid", "sock", "stream", "engine", "extensions"] {
        let _ = std::fs::remove_file(dir.join(format!("{}.{}", session, ext)));
    }

    Ok(json!({ "success": true, "killed_pid": pid }).to_string())
}

pub(super) async fn spawn_session(body: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;
    let session = parsed
        .get("session")
        .and_then(|v| v.as_str())
        .ok_or("Missing \"session\" field")?;

    if session.is_empty() || session.len() > 64 {
        return Err("Session name must be 1-64 characters".to_string());
    }

    let exe = std::env::current_exe().map_err(|e| format!("Cannot resolve executable: {}", e))?;

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg("open")
        .arg("about:blank")
        .arg("--session")
        .arg(session);

    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let status = cmd
        .status()
        .await
        .map_err(|e| format!("Failed to spawn session: {}", e))?;

    if status.success() {
        Ok(format!(
            r#"{{"success":true,"session":{}}}"#,
            serde_json::to_string(session).unwrap_or_default()
        ))
    } else {
        Err(format!("Session process exited with {}", status))
    }
}
