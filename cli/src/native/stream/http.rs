use rust_embed::Embed;
use serde_json::{json, Value};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

#[cfg(windows)]
use crate::connection::resolve_port;
use crate::connection::{attach_daemon_auth_token, get_socket_dir};
use crate::flags::parse_flags;

use super::chat::{chat_status_json, handle_chat_request, handle_models_request};
use super::dashboard::spawn_session;
use super::discovery::discover_sessions;

#[derive(Embed)]
#[folder = "../packages/dashboard/out/"]
struct DashboardAssets;

pub(super) const CORS_HEADERS: &str = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n";

/// Build CORS headers that reflect the request origin only when it passes
/// `is_allowed_origin`. Used for sensitive endpoints (chat, models) so the
/// API key is not accessible from arbitrary web pages.
pub(super) fn cors_headers_for_origin(origin: Option<&str>) -> String {
    let allowed_origin = match origin {
        Some(o) if super::is_allowed_origin(Some(o)) => o,
        _ => "http://localhost",
    };
    format!(
        "Access-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n",
        allowed_origin
    )
}

fn parse_origin(peeked: &[u8]) -> Option<String> {
    let header_str = std::str::from_utf8(peeked).ok()?;
    for line in header_str.lines() {
        if line.len() > 8 && line[..8].eq_ignore_ascii_case("origin: ") {
            return Some(line[8..].trim().to_string());
        }
    }
    None
}

pub(super) async fn handle_http_request(
    mut stream: tokio::net::TcpStream,
    peeked: &[u8],
    last_tabs: &Arc<RwLock<Vec<Value>>>,
    last_engine: &Arc<RwLock<String>>,
    session_name: &str,
) {
    let peeked_len = peeked.len();
    let mut discard = vec![0u8; peeked_len];
    let _ = stream.read_exact(&mut discard).await;

    let request = String::from_utf8_lossy(peeked);
    let first_line = request.lines().next().unwrap_or("");
    let method = first_line.split_whitespace().next().unwrap_or("GET");
    let raw_path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = split_path_query(raw_path);
    let origin = parse_origin(peeked);

    if method == "OPTIONS" {
        let response = format!(
            "HTTP/1.1 204 No Content\r\n{CORS_HEADERS}Access-Control-Max-Age: 86400\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    if method == "POST" {
        let full_body = read_full_body(&mut stream, peeked).await;
        if full_body.is_none()
            && (path == "/api/chat" || path == "/api/sessions" || path == "/api/command")
        {
            let body = r#"{"error":"Request body too large"}"#;
            let response = format!(
                "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(body.as_bytes()).await;
            return;
        }
        let body_str = full_body.as_deref().unwrap_or("");

        if path == "/api/sessions" {
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

        if path == "/api/command" {
            let result = relay_command_to_daemon(session_name, body_str).await;
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

        if path == "/api/service/reconcile" {
            let result = relay_service_command(session_name, service_reconcile_command()).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(job_id) = service_job_cancel_id(path) {
            let result =
                relay_service_command(session_name, service_job_cancel_command(job_id)).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if path == "/api/chat" {
            handle_chat_request(&mut stream, body_str, origin.as_deref()).await;
            return;
        }
    }

    if method == "GET" && path == "/api/service/status" {
        let result = relay_service_command(session_name, service_status_command()).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/events" {
        let cmd = match service_events_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/incidents" {
        let cmd = match service_incidents_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path.starts_with("/api/service/incidents/") {
        let Some(incident_id) = path
            .strip_prefix("/api/service/incidents/")
            .filter(|id| !id.is_empty())
        else {
            write_json_result(
                &mut stream,
                Err("Missing service incident id".to_string()),
                "400 Bad Request",
            )
            .await;
            return;
        };
        let mut cmd = match service_incidents_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        cmd["incidentId"] = json!(incident_id);
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path.starts_with("/api/service/jobs/") {
        let Some(job_id) = path
            .strip_prefix("/api/service/jobs/")
            .filter(|id| !id.is_empty())
        else {
            write_json_result(
                &mut stream,
                Err("Missing service job id".to_string()),
                "400 Bad Request",
            )
            .await;
            return;
        };
        let mut cmd = match service_jobs_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        cmd["jobId"] = json!(job_id);
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/jobs" {
        let cmd = match service_jobs_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/models" {
        handle_models_request(&mut stream, origin.as_deref()).await;
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

fn split_path_query(raw_path: &str) -> (&str, Option<&str>) {
    match raw_path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (raw_path, None),
    }
}

async fn write_json_result(
    stream: &mut tokio::net::TcpStream,
    result: Result<String, String>,
    error_status: &str,
) {
    let (status, resp_body) = match result {
        Ok(resp) => ("200 OK", resp),
        Err(e) => (
            error_status,
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
}

fn service_status_command() -> Value {
    json!({
        "action": "service_status",
        "serviceState": load_service_state_snapshot(),
    })
}

fn service_reconcile_command() -> Value {
    json!({
        "action": "service_reconcile",
        "serviceState": load_service_state_snapshot(),
    })
}

fn service_job_cancel_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/jobs/")
        .and_then(|rest| rest.strip_suffix("/cancel"))
        .filter(|id| !id.is_empty())
}

fn service_job_cancel_command(job_id: &str) -> Value {
    json!({
        "action": "service_job_cancel",
        "jobId": job_id,
    })
}

fn service_events_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_events",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "kind" => match value.as_str() {
                "reconciliation"
                | "browser_health_changed"
                | "tab_lifecycle_changed"
                | "reconciliation_error" => {
                    cmd["kind"] = json!(value);
                }
                _ => return Err(format!("Invalid kind value: {}", value)),
            },
            "browserId" | "browser_id" | "browser-id" => {
                cmd["browserId"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => return Err(format!("Unknown service events query parameter: {}", key)),
        }
    }

    Ok(cmd)
}

fn service_incidents_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_incidents",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "id" | "incidentId" | "incident_id" | "incident-id" => {
                cmd["incidentId"] = json!(value);
            }
            "state" => match value.as_str() {
                "active" | "recovered" | "service" => {
                    cmd["state"] = json!(value);
                }
                _ => return Err(format!("Invalid state value: {}", value)),
            },
            "kind" => match value.as_str() {
                "browser_health_changed"
                | "reconciliation_error"
                | "service_job_timeout"
                | "service_job_cancelled" => {
                    cmd["kind"] = json!(value);
                }
                _ => return Err(format!("Invalid kind value: {}", value)),
            },
            "browserId" | "browser_id" | "browser-id" => {
                cmd["browserId"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => {
                return Err(format!(
                    "Unknown service incidents query parameter: {}",
                    key
                ))
            }
        }
    }

    Ok(cmd)
}

fn service_jobs_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_jobs",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "id" | "jobId" | "job_id" | "job-id" => {
                cmd["jobId"] = json!(value);
            }
            "state" => match value.as_str() {
                "queued" | "running" | "succeeded" | "failed" | "cancelled" | "timed_out" => {
                    cmd["state"] = json!(value);
                }
                _ => return Err(format!("Invalid state value: {}", value)),
            },
            "action" | "jobAction" | "job_action" | "job-action" => {
                cmd["jobAction"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => return Err(format!("Unknown service jobs query parameter: {}", key)),
        }
    }

    Ok(cmd)
}

fn query_params(query: Option<&str>) -> Vec<(String, String)> {
    query
        .map(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default()
}

fn load_service_state_snapshot() -> Value {
    let args = vec!["service".to_string(), "status".to_string()];
    serde_json::to_value(parse_flags(&args).service_state).unwrap_or_else(|_| json!({}))
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| buf.windows(2).position(|w| w == b"\n\n").map(|p| p + 2))
}

fn parse_content_length_bytes(headers: &[u8]) -> Option<usize> {
    let header_str = std::str::from_utf8(headers).ok()?;
    for line in header_str.lines() {
        if line.len() > 16 && line[..16].eq_ignore_ascii_case("content-length: ") {
            return line[16..].trim().parse().ok();
        }
    }
    None
}

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

async fn read_full_body(stream: &mut tokio::net::TcpStream, peeked: &[u8]) -> Option<String> {
    let body_offset = find_header_end(peeked)?;
    let content_length = parse_content_length_bytes(&peeked[..body_offset])?;
    if content_length == 0 {
        return Some(String::new());
    }
    if content_length > MAX_BODY_SIZE {
        return None;
    }

    let peeked_body = &peeked[body_offset..];
    let peeked_body_len = peeked_body.len().min(content_length);

    let mut body = Vec::with_capacity(content_length);
    body.extend_from_slice(&peeked_body[..peeked_body_len]);

    let remaining = content_length - peeked_body_len;
    if remaining > 0 {
        let mut rest = vec![0u8; remaining];
        if stream.read_exact(&mut rest).await.is_err() {
            return String::from_utf8(body).ok();
        }
        body.extend_from_slice(&rest);
    }

    String::from_utf8(body).ok()
}

pub(super) async fn relay_command_to_daemon(
    session_name: &str,
    body: &str,
) -> Result<String, String> {
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

    let authenticated_cmd = attach_daemon_auth_token(&cmd, session_name)?;
    let mut json_str = serde_json::to_string(&authenticated_cmd).map_err(|e| e.to_string())?;
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

async fn relay_service_command(session_name: &str, cmd: Value) -> Result<String, String> {
    let body = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    relay_command_to_daemon(session_name, &body).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_path_query_returns_path_and_query() {
        assert_eq!(
            split_path_query("/api/service/events?limit=2&kind=reconciliation"),
            ("/api/service/events", Some("limit=2&kind=reconciliation"))
        );
        assert_eq!(
            split_path_query("/api/service/status"),
            ("/api/service/status", None)
        );
    }

    #[test]
    fn service_events_command_maps_query_filters() {
        let cmd = service_events_command(Some(
            "limit=7&kind=browser_health_changed&browser-id=browser-1&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_events");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["kind"], "browser_health_changed");
        assert_eq!(cmd["browserId"], "browser-1");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd.get("serviceState").is_some());
    }

    #[test]
    fn service_events_command_accepts_tab_lifecycle_kind() {
        let cmd = service_events_command(Some("kind=tab_lifecycle_changed")).unwrap();

        assert_eq!(cmd["kind"], "tab_lifecycle_changed");
    }

    #[test]
    fn service_incidents_command_maps_query_filters() {
        let cmd = service_incidents_command(Some(
            "id=incident-1&limit=7&state=active&kind=service_job_timeout&browser-id=browser-1&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_incidents");
        assert_eq!(cmd["incidentId"], "incident-1");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["state"], "active");
        assert_eq!(cmd["kind"], "service_job_timeout");
        assert_eq!(cmd["browserId"], "browser-1");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd.get("serviceState").is_some());
    }

    #[test]
    fn split_path_query_handles_service_incident_detail() {
        assert_eq!(
            split_path_query("/api/service/incidents/incident-123?since=2026-04-22T00%3A00%3A00Z"),
            (
                "/api/service/incidents/incident-123",
                Some("since=2026-04-22T00%3A00%3A00Z")
            )
        );
    }

    #[test]
    fn service_jobs_command_maps_query_filters() {
        let cmd = service_jobs_command(Some(
            "limit=7&state=failed&action=navigate&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_jobs");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["state"], "failed");
        assert_eq!(cmd["jobAction"], "navigate");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd.get("serviceState").is_some());
    }

    #[test]
    fn service_jobs_command_maps_id_filter() {
        let cmd = service_jobs_command(Some("id=job-123")).unwrap();

        assert_eq!(cmd["action"], "service_jobs");
        assert_eq!(cmd["jobId"], "job-123");
    }

    #[test]
    fn service_job_cancel_id_maps_path() {
        assert_eq!(
            service_job_cancel_id("/api/service/jobs/job-123/cancel"),
            Some("job-123")
        );
        assert_eq!(service_job_cancel_id("/api/service/jobs//cancel"), None);
        assert_eq!(service_job_cancel_id("/api/service/jobs/job-123"), None);
    }

    #[test]
    fn service_job_cancel_command_maps_id() {
        let cmd = service_job_cancel_command("job-123");

        assert_eq!(cmd["action"], "service_job_cancel");
        assert_eq!(cmd["jobId"], "job-123");
    }

    #[test]
    fn service_jobs_command_rejects_invalid_state() {
        let err = service_jobs_command(Some("state=broken")).unwrap_err();

        assert!(err.contains("Invalid state value"));
    }

    #[test]
    fn service_events_command_rejects_unknown_query() {
        let err = service_events_command(Some("bogus=true")).unwrap_err();

        assert!(err.contains("Unknown service events query parameter"));
    }

    #[test]
    fn service_events_command_rejects_invalid_kind() {
        let err = service_events_command(Some("kind=crash")).unwrap_err();

        assert!(err.contains("Invalid kind value"));
    }

    #[test]
    fn service_incidents_command_rejects_invalid_state() {
        let err = service_incidents_command(Some("state=failed")).unwrap_err();

        assert!(err.contains("Invalid state value"));
    }

    #[test]
    fn service_incidents_command_rejects_unknown_query() {
        let err = service_incidents_command(Some("bogus=true")).unwrap_err();

        assert!(err.contains("Unknown service incidents query parameter"));
    }
}

pub(super) fn serve_embedded_file(url_path: &str) -> (&'static str, &'static str, Vec<u8>) {
    let clean = url_path.trim_start_matches('/');
    let key = if clean.is_empty() {
        "index.html"
    } else {
        clean
    };

    let file = DashboardAssets::get(key).or_else(|| DashboardAssets::get("index.html"));

    match file {
        Some(content) => {
            let ext = key.rsplit('.').next().unwrap_or("");
            let ct = match ext {
                "html" => "text/html; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "ico" => "image/x-icon",
                "woff2" => "font/woff2",
                "woff" => "font/woff",
                "txt" => "text/plain; charset=utf-8",
                _ => "application/octet-stream",
            };
            ("200 OK", ct, content.data.to_vec())
        }
        None => (
            "404 Not Found",
            "text/html; charset=utf-8",
            b"<html><body><p>404 Not Found</p></body></html>".to_vec(),
        ),
    }
}
