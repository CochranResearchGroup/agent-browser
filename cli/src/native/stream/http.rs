use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

#[cfg(windows)]
use crate::connection::resolve_port;
use crate::connection::get_socket_dir;

use super::chat::{chat_status_json, handle_chat_request, handle_models_request};
use super::dashboard::spawn_session;
use super::discovery::discover_sessions;

pub(super) const CORS_HEADERS: &str = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n";

pub(super) async fn handle_http_request(
    mut stream: tokio::net::TcpStream,
    request: &str,
    peeked_len: usize,
    dashboard_dir: Option<&Path>,
    last_tabs: &Arc<RwLock<Vec<Value>>>,
    last_engine: &Arc<RwLock<String>>,
    session_name: &str,
) {
    let mut discard = vec![0u8; peeked_len];
    let _ = stream.read_exact(&mut discard).await;

    let first_line = request.lines().next().unwrap_or("");
    let method = first_line.split_whitespace().next().unwrap_or("GET");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    if method == "OPTIONS" {
        let response = format!(
            "HTTP/1.1 204 No Content\r\n{CORS_HEADERS}Access-Control-Max-Age: 86400\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/sessions" {
        let body_str = extract_http_body(request).unwrap_or("");
        let result = spawn_session(body_str).await;
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

    if method == "POST" && path == "/api/command" {
        let body = extract_http_body(request).unwrap_or("");
        let result = relay_command_to_daemon(session_name, body).await;
        let (status, resp_body) = match result {
            Ok(resp) => ("200 OK", resp),
            Err(e) => (
                "502 Bad Gateway",
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

    if method == "POST" && path == "/api/chat" {
        let body = extract_http_body(request).unwrap_or("");
        handle_chat_request(&mut stream, body).await;
        return;
    }

    if method == "GET" && path == "/api/models" {
        handle_models_request(&mut stream).await;
        return;
    }

    let (status, content_type, body): (&str, &str, Vec<u8>) = if path == "/api/sessions" {
        (
            "200 OK",
            "application/json; charset=utf-8",
            discover_sessions().into_bytes(),
        )
    } else if path == "/api/tabs" {
        let tabs = last_tabs.read().await;
        (
            "200 OK",
            "application/json; charset=utf-8",
            serde_json::to_string(&*tabs)
                .unwrap_or_else(|_| "[]".to_string())
                .into_bytes(),
        )
    } else if path == "/api/status" {
        let engine = last_engine.read().await;
        (
            "200 OK",
            "application/json; charset=utf-8",
            format!(r#"{{"engine":"{}"}}"#, *engine).into_bytes(),
        )
    } else if path == "/api/chat/status" {
        (
            "200 OK",
            "application/json; charset=utf-8",
            chat_status_json().into_bytes(),
        )
    } else {
        match dashboard_dir {
            Some(dir) => serve_static_file(dir, path),
            None => (
                "200 OK",
                "text/html; charset=utf-8",
                DASHBOARD_NOT_INSTALLED_HTML.as_bytes().to_vec(),
            ),
        }
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

pub(super) fn extract_http_body(request: &str) -> Option<&str> {
    request
        .find("\r\n\r\n")
        .map(|pos| &request[pos + 4..])
        .or_else(|| request.find("\n\n").map(|pos| &request[pos + 2..]))
}

pub(super) async fn relay_command_to_daemon(session_name: &str, body: &str) -> Result<String, String> {
    let mut cmd: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    if cmd.get("id").is_none() {
        let id = format!(
            "dash-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        cmd["id"] = json!(id);
    }

    let mut json_str = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    json_str.push('\n');

    #[cfg(unix)]
    let stream = {
        let socket_path = get_socket_dir().join(format!("{}.sock", session_name));
        tokio::net::UnixStream::connect(&socket_path)
            .await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?
    };

    #[cfg(windows)]
    let stream = {
        let port = resolve_port(session_name);
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?
    };

    let (reader, mut writer) = tokio::io::split(stream);

    writer
        .write_all(json_str.as_bytes())
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    let mut buf_reader = tokio::io::BufReader::new(reader);
    let mut response_line = String::new();
    tokio::io::AsyncBufReadExt::read_line(&mut buf_reader, &mut response_line)
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(response_line.trim().to_string())
}

pub(super) fn serve_static_file(dir: &Path, url_path: &str) -> (&'static str, &'static str, Vec<u8>) {
    let clean = url_path.trim_start_matches('/');
    let file_path = if clean.is_empty() {
        dir.join("index.html")
    } else {
        let joined = dir.join(clean);
        if joined.is_file() {
            joined
        } else {
            dir.join("index.html")
        }
    };

    match std::fs::read(&file_path) {
        Ok(content) => {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let ct = match ext {
                "html" => "text/html; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "ico" => "image/x-icon",
                _ => "application/octet-stream",
            };
            ("200 OK", ct, content)
        }
        Err(_) => (
            "404 Not Found",
            "text/html; charset=utf-8",
            b"<html><body><p>404 Not Found</p></body></html>".to_vec(),
        ),
    }
}

pub(super) const DASHBOARD_NOT_INSTALLED_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>agent-browser</title>
<style>
body { font-family: system-ui, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0a0a0a; color: #e5e5e5; }
.card { text-align: center; max-width: 400px; }
code { background: #262626; padding: 2px 8px; border-radius: 4px; font-size: 14px; }
</style>
</head>
<body>
<div class="card">
<h2>Dashboard not installed</h2>
<p>Run <code>agent-browser dashboard install</code> to download the dashboard.</p>
</div>
</body>
</html>"#;
