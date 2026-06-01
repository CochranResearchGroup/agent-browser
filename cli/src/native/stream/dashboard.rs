use serde_json::{json, Value};
use std::env;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::connection::get_socket_dir;

use super::app_intelligence::{
    app_intelligence_status_json, inspect_workspace_response, operator_confirm_response,
    operator_status_json, operator_turn_response, OperatorIdentity,
    APP_INTELLIGENCE_INSPECT_HTTP_ROUTE, APP_INTELLIGENCE_OPERATOR_CONFIRM_HTTP_ROUTE,
    APP_INTELLIGENCE_OPERATOR_STATUS_HTTP_ROUTE, APP_INTELLIGENCE_OPERATOR_TURN_HTTP_ROUTE,
    APP_INTELLIGENCE_STATUS_HTTP_ROUTE,
};
use super::chat::{chat_status_json, handle_chat_request, handle_models_request};
use super::dashboard_auth;
use super::discovery::discover_sessions;
use super::http::{serve_embedded_file, CORS_HEADERS};

const DASHBOARD_SERVICE_BACKEND_SESSION: &str = "dashboard-service-backend";

pub async fn run_dashboard_server(port: u16) {
    if let Err(err) = dashboard_auth::ensure_dashboard_auth_config() {
        eprintln!("Failed to initialize dashboard auth: {}", err);
        return;
    }

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
    let raw_path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = split_path_query(raw_path);
    let headers = dashboard_auth::parse_headers(header_str);
    let secure_cookie = dashboard_auth::request_is_secure(&headers);
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

    if method == "GET" && path == "/api/dashboard-auth/status" {
        let response = dashboard_auth::auth_status_response(&headers, secure_cookie);
        let _ = stream.write_all(&response.into_http_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/dashboard-auth/login" {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        let response = dashboard_auth::login_response(&headers, &body_str, secure_cookie);
        let _ = stream.write_all(&response.into_http_bytes()).await;
        return;
    }

    if method == "POST" && path == "/api/dashboard-auth/logout" {
        let response = dashboard_auth::logout_response(secure_cookie);
        let _ = stream.write_all(&response.into_http_bytes()).await;
        return;
    }

    if method == "GET" && path == "/api/dashboard-auth/verify" {
        let response = dashboard_auth::verify_forward_auth_response(&headers, secure_cookie);
        let _ = stream.write_all(&response.into_http_bytes()).await;
        return;
    }

    if path.starts_with("/api/") {
        match dashboard_auth::authenticate_headers(&headers) {
            Ok(Some(_)) => {}
            Ok(None) => {
                let response = dashboard_auth::unauthorized_api_response(secure_cookie);
                let _ = stream.write_all(&response.into_http_bytes()).await;
                return;
            }
            Err(err) => {
                write_json_error(&mut stream, "500 Internal Server Error", &err).await;
                return;
            }
        }
    }

    if path.starts_with("/api/stream/") {
        let body_str = if matches!(method, "POST" | "PUT" | "PATCH" | "DELETE") {
            read_post_body(&mut stream, &buf, n).await
        } else {
            String::new()
        };
        if let Some(port) = stream_api_port(path) {
            match proxy_local_http_api_request(port, method, raw_path, &body_str).await {
                Ok(response) => {
                    let _ = stream.write_all(&response).await;
                    return;
                }
                Err(err) => {
                    write_json_error(
                        &mut stream,
                        "502 Bad Gateway",
                        &format!("Stream API proxy failed: {}", err),
                    )
                    .await;
                    return;
                }
            }
        }
        write_json_error(&mut stream, "400 Bad Request", "Invalid stream API port").await;
        return;
    }

    if method == "POST" && path == "/api/chat" {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        handle_chat_request(&mut stream, &body_str, origin.as_deref()).await;
        return;
    }

    if method == "POST" && path == APP_INTELLIGENCE_INSPECT_HTTP_ROUTE {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        let (status, value) = inspect_workspace_response(&body_str);
        write_json_value(&mut stream, status, value).await;
        return;
    }

    if method == "POST" && path == APP_INTELLIGENCE_OPERATOR_TURN_HTTP_ROUTE {
        let identity = match dashboard_auth::require_superuser(&headers, secure_cookie) {
            Ok(identity) => identity,
            Err(response) => {
                let _ = stream.write_all(&response.into_http_bytes()).await;
                return;
            }
        };
        let body_str = read_post_body(&mut stream, &buf, n).await;
        let operator_identity = OperatorIdentity {
            username: identity.username,
            display_name: identity.display_name,
            role: identity.role,
        };
        let (status, value) = operator_turn_response(&body_str, &operator_identity);
        write_json_value(&mut stream, status, value).await;
        return;
    }

    if method == "POST" && path == APP_INTELLIGENCE_OPERATOR_CONFIRM_HTTP_ROUTE {
        let identity = match dashboard_auth::require_superuser(&headers, secure_cookie) {
            Ok(identity) => identity,
            Err(response) => {
                let _ = stream.write_all(&response.into_http_bytes()).await;
                return;
            }
        };
        let body_str = read_post_body(&mut stream, &buf, n).await;
        let operator_identity = OperatorIdentity {
            username: identity.username,
            display_name: identity.display_name,
            role: identity.role,
        };
        let (status, value) = operator_confirm_response(&body_str, &operator_identity);
        write_json_value(&mut stream, status, value).await;
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
        handle_service_api_request(&mut stream, method, raw_path, &body_str).await;
        return;
    }

    if method == "GET" && path == "/api/session-tabs" {
        handle_session_tabs_api_request(&mut stream, query).await;
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
    } else if path == APP_INTELLIGENCE_STATUS_HTTP_ROUTE {
        (
            "200 OK",
            "application/json; charset=utf-8",
            app_intelligence_status_json().to_string().into_bytes(),
        )
    } else if path == APP_INTELLIGENCE_OPERATOR_STATUS_HTTP_ROUTE {
        match dashboard_auth::require_superuser(&headers, secure_cookie) {
            Ok(identity) => {
                let operator_identity = OperatorIdentity {
                    username: identity.username,
                    display_name: identity.display_name,
                    role: identity.role,
                };
                (
                    "200 OK",
                    "application/json; charset=utf-8",
                    operator_status_json(&operator_identity)
                        .to_string()
                        .into_bytes(),
                )
            }
            Err(response) => {
                let _ = stream.write_all(&response.into_http_bytes()).await;
                return;
            }
        }
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

fn stream_api_port(path: &str) -> Option<u16> {
    let rest = path.strip_prefix("/api/stream/")?;
    let raw_port = rest.split('/').next()?;
    let port = raw_port.parse::<u16>().ok()?;
    if port > 0 {
        Some(port)
    } else {
        None
    }
}

async fn handle_service_api_request(
    stream: &mut tokio::net::TcpStream,
    method: &str,
    path: &str,
    body: &str,
) {
    if method == "POST" {
        if let Some((session_name, command_body)) = service_request_focus_command_body(path, body) {
            if let Some(port) = session_port_for_name(&session_name) {
                match proxy_local_http_api_request(port, "POST", "/api/command", &command_body)
                    .await
                {
                    Ok(response) => {
                        let _ = stream.write_all(&response).await;
                        return;
                    }
                    Err(err) => {
                        write_json_error(
                            stream,
                            "502 Bad Gateway",
                            &format!("View focus proxy failed: {}", err),
                        )
                        .await;
                        return;
                    }
                }
            }
        }
    }

    if let Some(port) = dashboard_service_backend_port() {
        match proxy_local_http_api_request(port, method, path, body).await {
            Ok(response) => {
                let response = repair_dashboard_service_status_response(path, response);
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
    dashboard_service_backend_port_from_sessions(sessions.as_array()?)
}

fn session_port_for_name(session_name: &str) -> Option<u16> {
    let sessions: Value = serde_json::from_str(&discover_sessions()).ok()?;
    session_port_from_sessions(sessions.as_array()?, session_name)
}

fn dashboard_service_backend_port_from_sessions(sessions: &[Value]) -> Option<u16> {
    sessions
        .iter()
        .find(|session| {
            session.get("session").and_then(Value::as_str)
                == Some(DASHBOARD_SERVICE_BACKEND_SESSION)
        })
        .or_else(|| {
            sessions
                .iter()
                .find(|session| session.get("session").and_then(Value::as_str) == Some("default"))
        })
        .or_else(|| sessions.first())
        .and_then(|session| session.get("port"))
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
}

fn session_port_from_sessions(sessions: &[Value], session_name: &str) -> Option<u16> {
    sessions
        .iter()
        .find(|session| session.get("session").and_then(Value::as_str) == Some(session_name))
        .and_then(|session| session.get("port"))
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
}

fn service_request_target_session_name(path: &str, body: &str) -> Option<String> {
    let (path, _) = split_path_query(path);
    if path != "/api/service/request" {
        return None;
    }
    let request: Value = serde_json::from_str(body).ok()?;
    if request.get("action").and_then(Value::as_str) != Some("view_focus") {
        return None;
    }
    for value in [
        request.pointer("/params/sessionName"),
        request.pointer("/params/daemonSession"),
        request.pointer("/params/targetSession"),
        request.pointer("/params/targetSessionName"),
        request.pointer("/params/sessionId"),
        request.pointer("/sessionName"),
        request.pointer("/daemonSession"),
        request.pointer("/targetSession"),
        request.pointer("/targetSessionName"),
        request.pointer("/sessionId"),
        request.pointer("/params/browserId"),
        request.pointer("/browserId"),
    ] {
        if let Some(session_name) = service_request_session_candidate(value) {
            return Some(session_name);
        }
    }
    None
}

fn service_request_focus_command_body(path: &str, body: &str) -> Option<(String, String)> {
    let session_name = service_request_target_session_name(path, body)?;
    let request: Value = serde_json::from_str(body).ok()?;
    let mut command = json!({
        "id": request
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("dashboard-view-focus-{}", uuid::Uuid::new_v4())),
        "action": "view_focus",
    });
    if let Some(params) = request.get("params").and_then(Value::as_object) {
        for (key, value) in params {
            if matches!(
                key.as_str(),
                "targetId" | "target_id" | "index" | "maximize"
            ) {
                command[key] = value.clone();
            }
        }
    }
    serde_json::to_string(&command)
        .ok()
        .map(|body| (session_name, body))
}

fn service_request_session_candidate(value: Option<&Value>) -> Option<String> {
    normalize_service_request_session_name(value?.as_str()?)
}

fn normalize_service_request_session_name(value: &str) -> Option<String> {
    let mut trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("browser:") {
        trimmed = rest.trim();
    }
    if let Some(rest) = trimmed.strip_prefix("session:") {
        trimmed = rest.trim();
    }
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn repair_dashboard_service_status_response(path: &str, response: Vec<u8>) -> Vec<u8> {
    let (path, _) = split_path_query(path);
    if path != "/api/service/status" {
        return response;
    }

    let Some(header_end) = find_http_header_end(&response) else {
        return response;
    };
    let headers = &response[..header_end];
    let body = &response[header_end + 4..];
    let Ok(mut value) = serde_json::from_slice::<Value>(body) else {
        return response;
    };
    if !repair_dashboard_service_status_value(&mut value) {
        return response;
    }
    let Ok(body) = serde_json::to_vec(&value) else {
        return response;
    };
    rebuild_http_response(headers, &body).unwrap_or(response)
}

fn repair_dashboard_service_status_value(value: &mut Value) -> bool {
    let Some(browsers) = value
        .pointer_mut("/data/service_state/browsers")
        .and_then(Value::as_object_mut)
    else {
        return false;
    };
    let mut changed = false;
    for browser in browsers.values_mut() {
        if browser.get("host").and_then(Value::as_str) != Some("remote_headed") {
            continue;
        }
        let Some(streams) = browser.get_mut("viewStreams").and_then(Value::as_array_mut) else {
            continue;
        };
        for stream in streams {
            if stream.get("provider").and_then(Value::as_str) != Some("rdp_gateway") {
                continue;
            }
            let root_url = stream.get("url").and_then(Value::as_str);
            let Some(url) = dashboard_guacamole_client_url(root_url) else {
                continue;
            };
            if stream.get("frameUrl").and_then(Value::as_str).is_none() {
                stream["frameUrl"] = Value::String(url.clone());
                changed = true;
            }
            if stream.get("externalUrl").and_then(Value::as_str).is_none() {
                stream["externalUrl"] = Value::String(url);
                changed = true;
            }
        }
    }
    changed
}

fn dashboard_guacamole_client_url(root_url: Option<&str>) -> Option<String> {
    if let Ok(configured_url) = env::var("AGENT_BROWSER_REMOTE_VIEW_URL") {
        let configured_url = configured_url.trim();
        if !configured_url.is_empty() && configured_url.contains("#/client/") {
            return Some(configured_url.to_string());
        }
    }
    let root_url = root_url.map(str::trim).filter(|url| !url.is_empty())?;
    if root_url.contains("#/client/") {
        return Some(root_url.to_string());
    }
    None
}

fn find_http_header_end(response: &[u8]) -> Option<usize> {
    response.windows(4).position(|window| window == b"\r\n\r\n")
}

fn rebuild_http_response(headers: &[u8], body: &[u8]) -> Option<Vec<u8>> {
    let header_text = std::str::from_utf8(headers).ok()?;
    let mut lines = header_text.lines();
    let status_line = lines.next().unwrap_or("HTTP/1.1 200 OK");
    let mut rebuilt = String::new();
    rebuilt.push_str(status_line);
    rebuilt.push_str("\r\n");
    let mut has_content_type = false;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("content-length:") {
            continue;
        }
        if lower.starts_with("content-type:") {
            has_content_type = true;
        }
        rebuilt.push_str(line);
        rebuilt.push_str("\r\n");
    }
    if !has_content_type {
        rebuilt.push_str("Content-Type: application/json; charset=utf-8\r\n");
    }
    rebuilt.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    let mut response = rebuilt.into_bytes();
    response.extend_from_slice(body);
    Some(response)
}

async fn proxy_local_http_api_request(
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

async fn handle_session_tabs_api_request(stream: &mut tokio::net::TcpStream, query: Option<&str>) {
    let Some(port) = query_value(query, "port").and_then(|value| value.parse::<u16>().ok()) else {
        write_json_error(stream, "400 Bad Request", "Missing or invalid session port").await;
        return;
    };

    match proxy_local_http_api_request(port, "GET", "/api/tabs", "").await {
        Ok(response) => {
            let _ = stream.write_all(&response).await;
        }
        Err(err) => {
            write_json_error(
                stream,
                "502 Bad Gateway",
                &format!("Session tabs proxy failed: {}", err),
            )
            .await;
        }
    }
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

async fn write_json_value(stream: &mut tokio::net::TcpStream, status: &str, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| {
        r#"{"success":false,"error":"Failed to serialize JSON response"}"#.to_string()
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use serde_json::json;

    #[test]
    fn dashboard_service_backend_prefers_dedicated_session() {
        let sessions = vec![
            json!({ "session": "default", "port": 1111 }),
            json!({ "session": DASHBOARD_SERVICE_BACKEND_SESSION, "port": 2222 }),
            json!({ "session": "other", "port": 3333 }),
        ];

        assert_eq!(
            dashboard_service_backend_port_from_sessions(&sessions),
            Some(2222)
        );
    }

    #[test]
    fn dashboard_service_backend_falls_back_to_default_then_first() {
        let default_sessions = vec![
            json!({ "session": "other", "port": 1111 }),
            json!({ "session": "default", "port": 2222 }),
        ];
        assert_eq!(
            dashboard_service_backend_port_from_sessions(&default_sessions),
            Some(2222)
        );

        let first_sessions = vec![json!({ "session": "other", "port": 3333 })];
        assert_eq!(
            dashboard_service_backend_port_from_sessions(&first_sessions),
            Some(3333)
        );
    }

    #[test]
    fn dashboard_service_request_target_session_reads_view_focus_session_name() {
        let body = r##"{"action":"view_focus","params":{"sessionName":"odollo-carrier-ups","maximize":true}}"##;

        assert_eq!(
            service_request_target_session_name("/api/service/request", body),
            Some("odollo-carrier-ups".to_string())
        );
    }

    #[test]
    fn dashboard_service_request_target_session_reads_view_focus_browser_id() {
        let body = r##"{"action":"view_focus","params":{"browserId":"browser:session:odollo-carrier-ups","maximize":true}}"##;

        assert_eq!(
            service_request_target_session_name("/api/service/request?source=workspace", body),
            Some("odollo-carrier-ups".to_string())
        );
    }

    #[test]
    fn dashboard_service_request_target_session_ignores_non_focus_actions() {
        let body = r##"{"action":"navigate","params":{"sessionName":"odollo-carrier-ups","url":"https://example.com"}}"##;

        assert_eq!(
            service_request_target_session_name("/api/service/request", body),
            None
        );
    }

    #[test]
    fn dashboard_service_request_target_session_finds_session_port() {
        let sessions = vec![
            json!({ "session": "default", "port": 1111 }),
            json!({ "session": "odollo-carrier-ups", "port": 2222 }),
        ];

        assert_eq!(
            session_port_from_sessions(&sessions, "odollo-carrier-ups"),
            Some(2222)
        );
    }

    #[test]
    fn dashboard_service_request_focus_command_body_strips_service_identity() {
        let body = r##"{"id":"focus-1","action":"view_focus","serviceName":"agent-browser-dashboard","agentName":"operator","taskName":"workspace-viewport-control","params":{"sessionName":"odollo-carrier-ups","targetId":"target-1","index":2,"maximize":true}}"##;
        let (session_name, command_body) =
            service_request_focus_command_body("/api/service/request", body).unwrap();
        let command: Value = serde_json::from_str(&command_body).unwrap();

        assert_eq!(session_name, "odollo-carrier-ups");
        assert_eq!(command["id"], "focus-1");
        assert_eq!(command["action"], "view_focus");
        assert_eq!(command["targetId"], "target-1");
        assert_eq!(command["index"], 2);
        assert_eq!(command["maximize"], true);
        assert!(command.get("serviceName").is_none());
        assert!(command.get("sessionName").is_none());
    }

    #[test]
    fn dashboard_service_status_response_adds_configured_guacamole_route_urls() {
        let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_URL"]);
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_URL",
            "/guacamole/#/client/MQBjAHBvc3RncmVzcWw=",
        );
        let body = json!({
            "success": true,
            "data": {
                "service_state": {
                    "browsers": {
                        "session:odollo-carrier-ups": {
                            "id": "session:odollo-carrier-ups",
                            "host": "remote_headed",
                            "viewStreams": [{
                                "id": "remote-headed-view",
                                "provider": "rdp_gateway",
                                "url": "https://agent-browser.example/guacamole/",
                                "readOnly": false
                            }]
                        }
                    }
                }
            }
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();

        let repaired = repair_dashboard_service_status_response("/api/service/status", response);
        let repaired_text = String::from_utf8(repaired).unwrap();
        let repaired_body = repaired_text.split("\r\n\r\n").nth(1).unwrap();
        let repaired_json: Value = serde_json::from_str(repaired_body).unwrap();

        assert_eq!(
            repaired_json["data"]["service_state"]["browsers"]["session:odollo-carrier-ups"]
                ["viewStreams"][0]["frameUrl"],
            "/guacamole/#/client/MQBjAHBvc3RncmVzcWw="
        );
        assert!(repaired_text.contains(&format!("Content-Length: {}", repaired_body.len())));
    }

    #[test]
    fn dashboard_service_status_response_leaves_guacamole_root_without_route() {
        let _guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_URL"]);
        let body = json!({
            "success": true,
            "data": {
                "service_state": {
                    "browsers": {
                        "session:odollo-carrier-ups": {
                            "id": "session:odollo-carrier-ups",
                            "host": "remote_headed",
                            "viewStreams": [{
                                "id": "remote-headed-view",
                                "provider": "rdp_gateway",
                                "url": "https://agent-browser.example/guacamole/",
                                "readOnly": false
                            }]
                        }
                    }
                }
            }
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();

        let repaired = repair_dashboard_service_status_response("/api/service/status", response);
        let repaired_text = String::from_utf8(repaired).unwrap();
        let repaired_body = repaired_text.split("\r\n\r\n").nth(1).unwrap();
        let repaired_json: Value = serde_json::from_str(repaired_body).unwrap();

        assert_eq!(
            repaired_json["data"]["service_state"]["browsers"]["session:odollo-carrier-ups"]
                ["viewStreams"][0]["url"],
            "https://agent-browser.example/guacamole/"
        );
        assert!(
            repaired_json["data"]["service_state"]["browsers"]["session:odollo-carrier-ups"]
                ["viewStreams"][0]["frameUrl"]
                .is_null()
        );
        assert!(repaired_text.contains(&format!("Content-Length: {}", repaired_body.len())));
    }

    #[test]
    fn dashboard_route_matching_ignores_query_string() {
        assert_eq!(
            split_path_query("/api/session-tabs?port=9223"),
            ("/api/session-tabs", Some("port=9223"))
        );
    }

    #[test]
    fn dashboard_session_tabs_query_decodes_port() {
        assert_eq!(
            query_value(Some("port=9223&ignored=true"), "port"),
            Some("9223".to_string())
        );
    }
}
