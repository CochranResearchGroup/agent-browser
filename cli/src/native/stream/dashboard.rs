use futures_util::{FutureExt, SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{watch, Mutex};
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

use crate::connection::get_socket_dir;
use crate::native::remote_view_handoff::remote_view_handoff_ready_owner_session;
#[cfg(test)]
use crate::native::service_failure_journal::{append_service_failure_at, read_service_failures_at};
use crate::native::service_failure_journal::{
    append_service_failure_best_effort, opaque_identifier_hash, read_service_failures,
    record_client_failure_observation, ServiceFailureCategory, ServiceFailureRecord,
    ServiceFailureReferences,
};
use crate::native::service_model::ServiceState;
use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};

#[cfg(test)]
use super::super::remote_view::{display_content_from_xwininfo, should_probe_route_display};
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
use super::foreign_cdp_control;
use super::http::{
    ensure_service_daemon_session, load_service_state, relay_command_to_daemon,
    runtime_manifest_json, serve_embedded_file, service_request_command_with_dashboard_generation,
    service_request_relay_session, CORS_HEADERS,
};

const DASHBOARD_SERVICE_BACKEND_SESSION: &str = "dashboard-service-backend";
const DASHBOARD_LOCAL_PROXY_TIMEOUT: Duration = Duration::from_secs(2);
const DASHBOARD_REMOTE_VIEW_REQUEST_PROXY_TIMEOUT: Duration = Duration::from_secs(15);
const DASHBOARD_REMOTE_VIEW_HANDOFF_PROXY_TIMEOUT: Duration = Duration::from_secs(60);
const DASHBOARD_STREAM_FRAME_PROXY_TIMEOUT: Duration = Duration::from_secs(7);
const DASHBOARD_CDP_SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(5);
const DASHBOARD_SERVICE_STATUS_CACHE_TTL: Duration = Duration::from_secs(10);
const DASHBOARD_SERVICE_STATUS_PROXY_TIMEOUT: Duration = Duration::from_secs(10);
const DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS: usize = 32;
const GUACAMOLE_PRIMARY_CLAIM_TTL: Duration = Duration::from_secs(10);
const DASHBOARD_SLOW_PROXY_THRESHOLD: Duration = Duration::from_secs(1);

static DASHBOARD_PROXY_INFLIGHT: AtomicUsize = AtomicUsize::new(0);
static DASHBOARD_LOGICAL_REQUEST_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
async fn dashboard_status_cache_test_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK.get_or_init(|| Mutex::new(())).lock().await
}

#[cfg(test)]
#[path = "dashboard_stress_tests.rs"]
mod dashboard_stress_tests;

/// Privacy-bounded terminal telemetry for a dashboard backend request.
///
/// Route, method, body, and error values are closed classifications. Raw request
/// URLs, query strings, headers, bodies, backend messages, and secrets never enter
/// this record or its service-failure-journal projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardHttpTelemetry {
    event: &'static str,
    route_class: &'static str,
    method: &'static str,
    status: Option<u16>,
    status_class: &'static str,
    body_class: &'static str,
    stage: &'static str,
    timing_scope: &'static str,
    elapsed_ms: u64,
    inflight_count: usize,
    response_bytes: Option<usize>,
    backend_error_class: Option<&'static str>,
}

type DashboardHttpObserver = Arc<dyn Fn(DashboardHttpTelemetry) + Send + Sync + 'static>;
type DashboardLogicalFailureObserver = Arc<dyn Fn(ServiceFailureRecord) + Send + Sync + 'static>;

fn production_dashboard_http_observer() -> DashboardHttpObserver {
    static OBSERVER: OnceLock<DashboardHttpObserver> = OnceLock::new();
    OBSERVER
        .get_or_init(|| Arc::new(emit_dashboard_http_telemetry))
        .clone()
}

fn production_dashboard_logical_failure_observer() -> DashboardLogicalFailureObserver {
    Arc::new(|record| {
        #[cfg(not(test))]
        append_service_failure_best_effort(&record);
        #[cfg(test)]
        let _ = record;
    })
}

impl DashboardHttpTelemetry {
    fn failed(&self) -> bool {
        self.status.is_some_and(|status| status >= 500) || self.backend_error_class.is_some()
    }
}

struct DashboardProxyInflightGuard;

impl DashboardProxyInflightGuard {
    fn enter() -> (Self, usize) {
        let inflight = DASHBOARD_PROXY_INFLIGHT.fetch_add(1, Ordering::Relaxed) + 1;
        (Self, inflight)
    }
}

impl Drop for DashboardProxyInflightGuard {
    fn drop(&mut self) {
        DASHBOARD_PROXY_INFLIGHT.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GuacamolePrimaryClaimKey {
    route_id: String,
    connection_id: String,
}

#[derive(Default)]
struct GuacamolePrimaryClaimRegistry {
    claims: HashMap<GuacamolePrimaryClaimKey, Instant>,
}

impl GuacamolePrimaryClaimRegistry {
    fn claim(&mut self, key: GuacamolePrimaryClaimKey, now: Instant) -> (bool, Duration) {
        self.claims.retain(|_, expires_at| *expires_at > now);
        if let Some(expires_at) = self.claims.get(&key) {
            return (false, expires_at.saturating_duration_since(now));
        }
        self.claims.insert(key, now + GUACAMOLE_PRIMARY_CLAIM_TTL);
        (true, GUACAMOLE_PRIMARY_CLAIM_TTL)
    }
}

#[derive(Default)]
struct DashboardServiceStatusCache {
    entries: HashMap<DashboardServiceStatusCacheKey, DashboardServiceStatusCacheEntry>,
    next_request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DashboardServiceStatusCacheKey {
    backend_session: &'static str,
    port: u16,
    path: String,
}

enum DashboardServiceStatusCacheEntry {
    InFlight {
        request_id: u64,
        registered_at: Instant,
        result: watch::Receiver<Option<Result<Vec<u8>, DashboardReadinessError>>>,
        owner_abort: Option<tokio::task::AbortHandle>,
    },
    Ready {
        completed_at: Instant,
        response: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct DashboardReadinessError {
    code: &'static str,
    message: String,
    details: Option<Value>,
}

impl DashboardReadinessError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    fn local_backend(
        code: &'static str,
        message: impl Into<String>,
        port: u16,
        path: &str,
        stage: &str,
    ) -> Self {
        Self::new(code, message).with_details(json!({
            "readinessState": readiness_state_for_gateway_code(code),
            "transport": "local_proxy",
            "port": port,
            "path": path,
            "stage": stage,
        }))
    }
}

impl fmt::Display for DashboardReadinessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn readiness_state_for_gateway_code(code: &str) -> &'static str {
    match code {
        "backend_connect_timeout"
        | "backend_write_timeout"
        | "backend_read_timeout"
        | "backend_unavailable" => "unreachable",
        "backend_empty_response" | "backend_invalid_http" | "invalid_backend_payload" => {
            "invalid_payload"
        }
        "stale_target" => "stale_target",
        "screenshot_failed" => "invalid_payload",
        _ => "error",
    }
}

pub async fn run_dashboard_server(port: u16) {
    if let Err(err) = dashboard_auth::ensure_dashboard_auth_config() {
        eprintln!("Failed to initialize dashboard auth: {}", err);
        return;
    }
    if std::env::var_os("AGENT_BROWSER_DASHBOARD_BACKEND_ONLY").is_none() {
        ensure_dashboard_service_backend().await;
    }
    // Build and cache the manifest before accepting traffic so a newly
    // selected backend cannot lose its first ingress probe to debug-build
    // executable and embedded-asset hashing latency.
    let _ = runtime_manifest_json();

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

pub(crate) async fn ensure_dashboard_service_backend() {
    if let Err(err) = ensure_dashboard_service_backend_with_retry(
        || super::http::ensure_service_daemon_session(DASHBOARD_SERVICE_BACKEND_SESSION, None),
        Duration::from_millis(250),
    )
    .await
    {
        eprintln!("Failed to initialize dashboard service backend: {err}");
    }
}

async fn ensure_dashboard_service_backend_with_retry<F, Fut>(
    mut ensure: F,
    retry_delay: Duration,
) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    const MAX_ATTEMPTS: usize = 3;
    let mut last_error = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match ensure().await {
            Ok(()) => return Ok(()),
            Err(error) => {
                if attempt < MAX_ATTEMPTS {
                    eprintln!(
                        "Dashboard service backend bootstrap attempt {attempt}/{MAX_ATTEMPTS} failed: {error}; retrying"
                    );
                    tokio::time::sleep(retry_delay).await;
                }
                last_error = Some(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "dashboard service backend bootstrap failed".to_string()))
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
        if !response.is_success() {
            append_service_failure_best_effort(
                &ServiceFailureRecord::new(
                    ServiceFailureCategory::DashboardAction,
                    "dashboard_auth",
                    "login",
                    "dashboard_login_failed",
                    "Dashboard authentication did not succeed.",
                )
                .with_action("dashboard_login"),
            );
        }
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

    if method == "GET" && path == "/api/runtime/manifest" {
        write_json_value(&mut stream, "200 OK", runtime_manifest_json()).await;
        return;
    }

    let authenticated_dashboard_user = if path.starts_with("/api/") {
        match dashboard_auth::authenticate_headers(&headers) {
            Ok(Some(identity)) => Some(identity.username),
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
    } else {
        None
    };

    if method == "GET" && path == "/api/runtime/health" {
        write_json_value(&mut stream, "200 OK", crate::install::runtime_health_json()).await;
        return;
    }

    if method == "POST" && path == "/api/guacamole-primary-claim" {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        let (status, value) = guacamole_primary_claim_response(&body_str).await;
        write_json_value(&mut stream, status, value).await;
        return;
    }

    if path.starts_with("/api/stream/") {
        let body_str = if matches!(method, "POST" | "PUT" | "PATCH" | "DELETE") {
            read_post_body(&mut stream, &buf, n).await
        } else {
            String::new()
        };
        if let Some(port) = stream_api_port(path) {
            let proxy_timeout = if method == "GET" && path.ends_with("/frame") {
                DASHBOARD_STREAM_FRAME_PROXY_TIMEOUT
            } else {
                DASHBOARD_LOCAL_PROXY_TIMEOUT
            };
            match proxy_local_http_api_request_with_timeout(
                port,
                method,
                raw_path,
                &body_str,
                proxy_timeout,
            )
            .await
            {
                Ok(response) => {
                    let _ = stream.write_all(&response).await;
                    return;
                }
                Err(err) => {
                    write_json_error_with_code(
                        &mut stream,
                        "502 Bad Gateway",
                        &format!("Stream API proxy failed: {}", err),
                        Some(err.code),
                        err.details.clone(),
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
        let Some(authenticated_dashboard_user) = authenticated_dashboard_user.as_deref() else {
            let response = dashboard_auth::unauthorized_api_response(secure_cookie);
            let _ = stream.write_all(&response.into_http_bytes()).await;
            return;
        };
        handle_service_api_request(
            &mut stream,
            method,
            raw_path,
            &body_str,
            authenticated_dashboard_user,
        )
        .await;
        return;
    }

    if method == "GET" && path == "/api/session-tabs" {
        handle_session_tabs_api_request(&mut stream, query).await;
        return;
    }

    if method == "GET" && path == "/api/session-screenshot" {
        handle_session_screenshot_api_request(&mut stream, query).await;
        return;
    }

    if method == "GET" && path == "/api/foreign-cdp/control" {
        handle_foreign_cdp_control_status(&mut stream, query).await;
        return;
    }

    if method == "POST"
        && matches!(
            path,
            "/api/foreign-cdp/borrow" | "/api/foreign-cdp/release" | "/api/foreign-cdp/input"
        )
    {
        let identity = match dashboard_auth::require_superuser(&headers, secure_cookie) {
            Ok(identity) => identity,
            Err(response) => {
                let _ = stream.write_all(&response.into_http_bytes()).await;
                return;
            }
        };
        let body_str = read_post_body(&mut stream, &buf, n).await;
        handle_foreign_cdp_control_request(
            &mut stream,
            path,
            &body_str,
            identity.username.as_str(),
        )
        .await;
        return;
    }

    if method == "POST" && path == "/api/session-console" {
        let body_str = read_post_body(&mut stream, &buf, n).await;
        handle_session_console_api_request(&mut stream, query, &body_str).await;
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
    authenticated_dashboard_user: &str,
) {
    if method == "GET" && split_path_query(path).0 == "/api/service/failures" {
        let limit = split_path_query(path)
            .1
            .and_then(|query| {
                query.split('&').find_map(|part| {
                    let (key, value) = part.split_once('=')?;
                    (key == "limit")
                        .then(|| value.parse::<usize>().ok())
                        .flatten()
                })
            })
            .unwrap_or(100);
        match read_service_failures(limit) {
            Ok(readback) => {
                write_json_value(
                    stream,
                    "200 OK",
                    json!({
                        "success": true,
                        "data": readback,
                    }),
                )
                .await
            }
            Err(error) => {
                write_json_error_with_code(
                    stream,
                    "500 Internal Server Error",
                    &error,
                    Some("failure_journal_read_failed"),
                    None,
                )
                .await
            }
        }
        return;
    }
    if method == "POST" && split_path_query(path).0 == "/api/service/failure-observation" {
        match record_client_failure_observation(body, authenticated_dashboard_user) {
            Ok(record) => {
                write_json_value(
                    stream,
                    "202 Accepted",
                    json!({
                        "success": true,
                        "data": {
                            "schemaVersion": record.schema_version,
                            "occurrenceId": record.occurrence_id,
                            "recorded": true,
                        }
                    }),
                )
                .await;
            }
            Err(error) => write_json_error(stream, "400 Bad Request", &error).await,
        }
        return;
    }
    if method == "POST" {
        if let Some((session_name, command_body)) = service_request_focus_command_body(path, body) {
            if let Some(port) = session_port_for_name(&session_name) {
                match proxy_local_http_api_request_with_timeout(
                    port,
                    "POST",
                    "/api/command",
                    &command_body,
                    DASHBOARD_REMOTE_VIEW_REQUEST_PROXY_TIMEOUT,
                )
                .await
                {
                    Ok(response) => {
                        let response = match require_json_backend_response(
                            response,
                            port,
                            "POST",
                            "/api/command",
                        ) {
                            Ok(response) => response,
                            Err(err) => {
                                write_json_error_with_code(
                                    stream,
                                    "502 Bad Gateway",
                                    &format!("View focus proxy failed: {}", err),
                                    Some(err.code),
                                    err.details.clone(),
                                )
                                .await;
                                return;
                            }
                        };
                        let _ = stream.write_all(&response).await;
                        return;
                    }
                    Err(err) => {
                        write_json_error_with_code(
                            stream,
                            "502 Bad Gateway",
                            &format!("View focus proxy failed: {}", err),
                            Some(err.code),
                            err.details.clone(),
                        )
                        .await;
                        return;
                    }
                }
            }
        }

        if let Some(command) =
            service_request_handoff_proxy_command_body(path, body, authenticated_dashboard_user)
        {
            let (session_name, command_body) = match command {
                Ok(command) => command,
                Err(err) => {
                    write_json_error(stream, "400 Bad Request", &err).await;
                    return;
                }
            };
            if let Some(port) = session_port_for_name(&session_name) {
                match proxy_dashboard_service_api_request(
                    port,
                    "POST",
                    "/api/command",
                    &command_body,
                    DASHBOARD_REMOTE_VIEW_HANDOFF_PROXY_TIMEOUT,
                )
                .await
                {
                    Ok(response) => {
                        let response = match service_api_handler_backend_response(
                            method, path, response, port,
                        ) {
                            Ok(response) => response,
                            Err(err) => {
                                write_json_error_with_code(
                                    stream,
                                    "502 Bad Gateway",
                                    &format!("Durable handoff owner proxy failed: {err}"),
                                    Some(err.code),
                                    err.details.clone(),
                                )
                                .await;
                                return;
                            }
                        };
                        let response = match sanitize_dashboard_handoff_response(body, response) {
                            Ok(response) => response,
                            Err(err) => {
                                write_json_error_with_code(
                                    stream,
                                    "502 Bad Gateway",
                                    &format!("Durable handoff public response was invalid: {err}"),
                                    Some("durable_handoff_public_response_invalid"),
                                    None,
                                )
                                .await;
                                return;
                            }
                        };
                        if let Some(handoff_id) =
                            authenticated_candidate_handoff_commit_id(body, &response)
                        {
                            if let Some(generation) =
                                std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION")
                                    .ok()
                                    .filter(|value| !value.trim().is_empty())
                            {
                                let commit = tokio::task::spawn_blocking(move || {
                                    crate::dashboard_ingress::
                                        commit_authenticated_dashboard_candidate_from_handoff(
                                            &generation,
                                            &handoff_id,
                                        )
                                })
                                .await
                                .map_err(|err| {
                                    format!("candidate ingress commit task failed: {err}")
                                })
                                .and_then(|result| result);
                                if let Err(err) = commit {
                                    write_json_error_with_code(
                                        stream,
                                        "409 Conflict",
                                        &format!(
                                            "Authenticated candidate handoff could not commit dashboard ingress: {err}"
                                        ),
                                        Some("candidate_ingress_commit_failed"),
                                        None,
                                    )
                                    .await;
                                    return;
                                }
                            }
                        }
                        let _ = stream.write_all(&response).await;
                        return;
                    }
                    Err(err) => {
                        write_json_error_with_code(
                            stream,
                            "502 Bad Gateway",
                            &format!("Durable handoff owner proxy failed: {err}"),
                            Some(err.code),
                            err.details.clone(),
                        )
                        .await;
                        return;
                    }
                }
            }
        }

        if path == "/api/service/request" {
            let state = load_service_state();
            let command = match service_request_command_with_dashboard_generation(
                body,
                Some(&state),
                authenticated_dashboard_user,
                DASHBOARD_SERVICE_BACKEND_SESSION,
                std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION")
                    .ok()
                    .as_deref(),
            ) {
                Ok(command) => command,
                Err(err) => {
                    write_json_error(stream, "400 Bad Request", &err).await;
                    return;
                }
            };
            let session_name =
                service_request_relay_session(DASHBOARD_SERVICE_BACKEND_SESSION, body, &command);
            if service_request_requires_daemon_relay(&command) {
                if let Err(err) = ensure_service_daemon_session(&session_name, Some(&command)).await
                {
                    record_durable_handoff_gateway_failure(
                        &command,
                        &session_name,
                        "durable_handoff_owner_prepare_failed",
                        &err,
                    );
                    write_json_error_with_code(
                        stream,
                        "502 Bad Gateway",
                        &format!("Durable handoff owner preparation failed: {err}"),
                        Some("durable_handoff_owner_prepare_failed"),
                        None,
                    )
                    .await;
                    return;
                }
                let command_body = command.to_string();
                match timeout(
                    DASHBOARD_REMOTE_VIEW_HANDOFF_PROXY_TIMEOUT,
                    relay_command_to_daemon(&session_name, &command_body),
                )
                .await
                {
                    Ok(Ok(response)) => match serde_json::from_str::<Value>(&response) {
                        Ok(response) => write_json_value(stream, "200 OK", response).await,
                        Err(err) => {
                            record_durable_handoff_gateway_failure(
                                &command,
                                &session_name,
                                "durable_handoff_owner_response_invalid",
                                &err.to_string(),
                            );
                            write_json_error_with_code(
                                stream,
                                "502 Bad Gateway",
                                &format!("Durable handoff owner returned invalid JSON: {err}"),
                                Some("durable_handoff_owner_response_invalid"),
                                None,
                            )
                            .await;
                        }
                    },
                    Ok(Err(err)) => {
                        record_durable_handoff_gateway_failure(
                            &command,
                            &session_name,
                            "durable_handoff_owner_relay_failed",
                            &err,
                        );
                        write_json_error_with_code(
                            stream,
                            "502 Bad Gateway",
                            &format!("Durable handoff owner relay failed: {err}"),
                            Some("durable_handoff_owner_relay_failed"),
                            None,
                        )
                        .await;
                    }
                    Err(_) => {
                        record_durable_handoff_gateway_failure(
                            &command,
                            &session_name,
                            "durable_handoff_owner_relay_timeout",
                            "Durable handoff owner relay timed out",
                        );
                        write_json_error_with_code(
                            stream,
                            "504 Gateway Timeout",
                            "Durable handoff owner relay timed out",
                            Some("durable_handoff_owner_relay_timeout"),
                            None,
                        )
                        .await;
                    }
                }
                return;
            }
            let port = if session_name == DASHBOARD_SERVICE_BACKEND_SESSION {
                dashboard_service_backend_port()
            } else {
                if let Err(err) = ensure_service_daemon_session(&session_name, Some(&command)).await
                {
                    write_json_error(stream, "502 Bad Gateway", &err).await;
                    return;
                }
                session_port_for_name(&session_name)
            };
            let Some(port) = port else {
                write_json_error(
                    stream,
                    "503 Service Unavailable",
                    &format!("Service session '{session_name}' has no HTTP route"),
                )
                .await;
                return;
            };
            match proxy_dashboard_service_api_request(
                port,
                "POST",
                "/api/command",
                &command.to_string(),
                DASHBOARD_REMOTE_VIEW_HANDOFF_PROXY_TIMEOUT,
            )
            .await
            {
                Ok(response) => {
                    let _ = stream.write_all(&response).await;
                }
                Err(err) => {
                    write_json_error_with_code(
                        stream,
                        "502 Bad Gateway",
                        &format!("Service request proxy failed: {err}"),
                        Some(err.code),
                        err.details,
                    )
                    .await;
                }
            }
            return;
        }
    }

    if dashboard_service_status_cacheable(method, path) {
        let result = dashboard_service_status_with_transports(
            dashboard_service_backend_port(),
            path,
            |port, owned_path| async move {
                proxy_dashboard_service_api_request(
                    port,
                    "GET",
                    &owned_path,
                    "",
                    DASHBOARD_SERVICE_STATUS_PROXY_TIMEOUT,
                )
                .await
            },
            |owned_path| async move { service_api_cli_fallback("GET", &owned_path).await },
        )
        .await;
        match result {
            Ok(response) => {
                let _ = stream.write_all(&response).await;
            }
            Err(err) => {
                write_json_error_with_code(
                    stream,
                    "502 Bad Gateway",
                    &format!("Service API proxy failed: {err}"),
                    Some(err.code),
                    err.details.clone(),
                )
                .await;
            }
        }
        return;
    }

    if let Some(port) = dashboard_service_backend_port() {
        let request_timeout = service_api_proxy_timeout(method, path, body);
        match proxy_dashboard_service_api_request(port, method, path, body, request_timeout).await {
            Ok(response) => {
                let status = http_response_status(&response).unwrap_or(0);
                if !(200..300).contains(&status) {
                    if let Some(response) = service_api_cli_fallback(method, path).await {
                        let _ = stream.write_all(response.as_bytes()).await;
                        return;
                    }
                }
                let response =
                    match service_api_handler_backend_response(method, path, response, port) {
                        Ok(response) => response,
                        Err(err) => {
                            write_json_error_with_code(
                                stream,
                                "502 Bad Gateway",
                                &format!("Service API proxy failed: {}", err),
                                Some(err.code),
                                err.details.clone(),
                            )
                            .await;
                            return;
                        }
                    };
                let _ = stream.write_all(&response).await;
                return;
            }
            Err(err) => {
                if let Some(response) = service_api_cli_fallback(method, path).await {
                    let _ = stream.write_all(response.as_bytes()).await;
                    return;
                }
                write_json_error_with_code(
                    stream,
                    "502 Bad Gateway",
                    &format!("Service API proxy failed: {}", err),
                    Some(err.code),
                    err.details.clone(),
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

fn service_request_requires_daemon_relay(command: &Value) -> bool {
    command.get("action").and_then(Value::as_str) == Some("service_remote_view_handoff_resolve")
}

fn record_durable_handoff_gateway_failure(
    command: &Value,
    session_name: &str,
    code: &str,
    summary: &str,
) {
    let handoff_id_hash = command
        .get("handoffId")
        .and_then(Value::as_str)
        .map(opaque_identifier_hash);
    append_service_failure_best_effort(
        &ServiceFailureRecord::new(
            ServiceFailureCategory::HandoffLink,
            "dashboard_service_gateway",
            "resolve",
            code,
            summary,
        )
        .with_action("service_remote_view_handoff_resolve")
        .with_references(ServiceFailureReferences {
            runtime_lane_id: Some(DASHBOARD_SERVICE_BACKEND_SESSION.to_string()),
            session_id: Some(session_name.to_string()),
            handoff_id_hash,
            ..ServiceFailureReferences::default()
        }),
    );
}

pub(crate) async fn dashboard_service_status_with_transports<
    Backend,
    BackendFuture,
    Fallback,
    FallbackFuture,
>(
    backend_port: Option<u16>,
    path: &str,
    backend: Backend,
    mut fallback: Fallback,
) -> Result<Vec<u8>, DashboardReadinessError>
where
    Backend: FnOnce(u16, String) -> BackendFuture,
    BackendFuture: std::future::Future<Output = Result<Vec<u8>, DashboardReadinessError>>,
    Fallback: FnMut(String) -> FallbackFuture,
    FallbackFuture: std::future::Future<Output = Option<String>>,
{
    let owned_path = path.to_string();
    let Some(port) = backend_port else {
        return fallback(owned_path.clone())
            .await
            .map(String::into_bytes)
            .ok_or_else(|| {
                DashboardReadinessError::local_backend(
                    "backend_unavailable",
                    "no service status backend or CLI fallback was available",
                    0,
                    &owned_path,
                    "request",
                )
            });
    };
    match backend(port, owned_path.clone()).await {
        Ok(response) => {
            let status = http_response_status(&response).unwrap_or(0);
            if !(200..300).contains(&status) {
                if let Some(fallback) = fallback(owned_path).await {
                    return Ok(fallback.into_bytes());
                }
            }
            service_api_handler_backend_response("GET", path, response, port)
        }
        Err(error) => fallback(owned_path)
            .await
            .map(String::into_bytes)
            .ok_or(error),
    }
}

fn dashboard_service_backend_port() -> Option<u16> {
    if let Some(port) = configured_dashboard_service_backend_port(
        std::env::var("AGENT_BROWSER_DASHBOARD_BACKEND_PORT")
            .ok()
            .as_deref(),
    ) {
        return Some(port);
    }
    let sessions: Value = serde_json::from_str(&discover_sessions()).ok()?;
    dashboard_service_backend_port_from_sessions(sessions.as_array()?)
}

/// Resolve the explicitly managed local dashboard backend before consulting
/// browser-session discovery. A backend-only process is not a browser session
/// and therefore cannot be expected to register in the daemon socket catalog.
fn configured_dashboard_service_backend_port(value: Option<&str>) -> Option<u16> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port > 0)
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

fn service_request_handoff_proxy_command_body(
    path: &str,
    body: &str,
    authenticated_dashboard_user: &str,
) -> Option<Result<(String, String), String>> {
    let state_path = JsonServiceStateStore::default_path().ok()?;
    let state = JsonServiceStateStore::new(state_path).load().ok()?;
    service_request_handoff_proxy_command_body_from_state(
        path,
        body,
        authenticated_dashboard_user,
        &state,
        std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION")
            .ok()
            .as_deref(),
    )
}

fn service_request_handoff_proxy_command_body_from_state(
    path: &str,
    body: &str,
    authenticated_dashboard_user: &str,
    state: &ServiceState,
    dashboard_deployment_generation: Option<&str>,
) -> Option<Result<(String, String), String>> {
    let session_name = service_request_handoff_target_session_name_from_state(path, body, state)?;
    Some(
        service_request_command_with_dashboard_generation(
            body,
            Some(state),
            authenticated_dashboard_user,
            &session_name,
            dashboard_deployment_generation,
        )
        .and_then(|command| {
            serde_json::to_string(&command)
                .map(|command_body| (session_name, command_body))
                .map_err(|err| format!("Failed to serialize service request command: {err}"))
        }),
    )
}

fn service_request_handoff_target_session_name_from_state(
    path: &str,
    body: &str,
    state: &ServiceState,
) -> Option<String> {
    let (path, _) = split_path_query(path);
    if path != "/api/service/request" {
        return None;
    }
    let request: Value = serde_json::from_str(body).ok()?;
    if request.get("action").and_then(Value::as_str) != Some("service_remote_view_handoff_resolve")
    {
        return None;
    }
    let handoff_id = request
        .pointer("/params/handoffId")
        .or_else(|| request.get("handoffId"))
        .and_then(Value::as_str)?
        .trim();
    if handoff_id.is_empty() {
        return None;
    }
    let handoff = state.remote_view_handoffs.get(handoff_id)?;
    if let Some(session_name) = remote_view_handoff_ready_owner_session(state, handoff)
        .and_then(|session_name| normalize_service_request_session_name(&session_name))
    {
        return Some(session_name);
    }
    if handoff.browser_id.is_some()
        || handoff.target_id.is_some()
        || handoff.presentation_receipt.is_some()
    {
        return None;
    }
    handoff
        .session_name
        .as_deref()
        .and_then(normalize_service_request_session_name)
        .or_else(|| service_request_session_candidate(handoff.intent.get("sessionName")))
}

fn service_api_proxy_timeout(method: &str, path: &str, body: &str) -> Duration {
    let (path, _) = split_path_query(path);
    if dashboard_service_status_cacheable(method, path) {
        return DASHBOARD_SERVICE_STATUS_PROXY_TIMEOUT;
    }
    if method == "POST" && path == "/api/service/request" {
        let action = serde_json::from_str::<Value>(body)
            .ok()
            .and_then(|request| {
                request
                    .get("action")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        if action.as_deref() == Some("service_remote_view_handoff_resolve") {
            return DASHBOARD_REMOTE_VIEW_HANDOFF_PROXY_TIMEOUT;
        }
        if action.as_deref().is_some_and(is_remote_view_service_action) {
            return DASHBOARD_REMOTE_VIEW_REQUEST_PROXY_TIMEOUT;
        }
    }
    DASHBOARD_LOCAL_PROXY_TIMEOUT
}

/// Remote-view lifecycle requests may require service-state reconciliation and
/// display readiness probes. Keep their proxy budget bounded but distinct from
/// ordinary local dashboard requests.
fn is_remote_view_service_action(action: &str) -> bool {
    matches!(
        action,
        "remote_view_open"
            | "service_remote_view_route_preflight"
            | "service_remote_view_browser_reattach"
            | "service_remote_view_route_switch"
            | "service_remote_view_route_checkout"
            | "service_remote_view_route_release"
            | "service_route_pool_repair"
            | "service_viewer_lease_request"
            | "service_viewer_lease_heartbeat"
            | "service_viewer_lease_release"
            | "service_controller_lease_takeover"
            | "view_focus"
            | "view_takeover"
    )
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
        crate::runtime_host::SERVICE_REQUEST_EXPLICIT_PROFILE_ROUTING_FIELD: false,
    });
    for key in [
        "serviceName",
        "agentName",
        "taskName",
        "jobTimeoutMs",
        "timeoutMs",
    ] {
        if let Some(value) = request.get(key) {
            command[key] = value.clone();
        }
    }
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

async fn proxy_local_http_api_request(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
) -> Result<Vec<u8>, DashboardReadinessError> {
    proxy_local_http_api_request_with_timeout(
        port,
        method,
        path,
        body,
        DASHBOARD_LOCAL_PROXY_TIMEOUT,
    )
    .await
}

async fn proxy_local_http_api_request_with_timeout(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    request_timeout: Duration,
) -> Result<Vec<u8>, DashboardReadinessError> {
    proxy_local_http_api_request_with_timeout_observed(
        port,
        method,
        path,
        body,
        request_timeout,
        emit_dashboard_http_telemetry,
    )
    .await
}

async fn proxy_local_http_api_request_with_timeout_observed(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    request_timeout: Duration,
    observer: impl Fn(DashboardHttpTelemetry),
) -> Result<Vec<u8>, DashboardReadinessError> {
    let started_at = Instant::now();
    let (_inflight_guard, inflight_count) = DashboardProxyInflightGuard::enter();
    let result =
        proxy_local_http_api_request_unobserved(port, method, path, body, request_timeout).await;
    if let Some(telemetry) = dashboard_http_terminal_telemetry(
        method,
        path,
        started_at.elapsed(),
        inflight_count,
        &result,
    ) {
        observer(telemetry);
    }
    result
}

async fn proxy_local_http_api_request_unobserved(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    request_timeout: Duration,
) -> Result<Vec<u8>, DashboardReadinessError> {
    let mut backend = run_dashboard_backend_io_phase(
        DashboardBackendIoPhase {
            timeout_code: "backend_connect_timeout",
            timeout_message: format!("timed out connecting to 127.0.0.1:{port}"),
            failure_message: format!("failed connecting to 127.0.0.1:{port}"),
            port,
            path,
            stage: "connect",
            timeout: request_timeout,
        },
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    run_dashboard_backend_io_phase(
        DashboardBackendIoPhase {
            timeout_code: "backend_write_timeout",
            timeout_message: format!("timed out writing to 127.0.0.1:{port}{path}"),
            failure_message: format!("failed writing to 127.0.0.1:{port}{path}"),
            port,
            path,
            stage: "write",
            timeout: request_timeout,
        },
        backend.write_all(request.as_bytes()),
    )
    .await?;
    read_local_http_response(&mut backend, port, path, request_timeout).await
}

fn dashboard_http_terminal_telemetry(
    method: &str,
    path: &str,
    elapsed: Duration,
    inflight_count: usize,
    result: &Result<Vec<u8>, DashboardReadinessError>,
) -> Option<DashboardHttpTelemetry> {
    let (status, body_class, stage, response_bytes, backend_error_class) = match result {
        Ok(response) => {
            let status = http_response_status(response);
            (
                status,
                dashboard_http_body_class(response),
                "response",
                Some(response.len()),
                status
                    .is_some_and(|status| status >= 500)
                    .then_some("backend_http_5xx"),
            )
        }
        Err(error) => (
            None,
            "none",
            dashboard_backend_error_stage(error),
            None,
            Some(error.code),
        ),
    };
    let failed = status.is_some_and(|status| status >= 500) || backend_error_class.is_some();
    if !failed && elapsed < DASHBOARD_SLOW_PROXY_THRESHOLD {
        return None;
    }
    Some(DashboardHttpTelemetry {
        event: if failed {
            "dashboard_http_failed"
        } else {
            "dashboard_http_slow"
        },
        route_class: dashboard_http_route_class(path),
        method: dashboard_http_method_class(method),
        status,
        status_class: dashboard_http_status_class(status),
        body_class,
        stage,
        timing_scope: "local_backend_round_trip",
        elapsed_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
        inflight_count,
        response_bytes,
        backend_error_class,
    })
}

fn dashboard_http_route_class(path: &str) -> &'static str {
    match split_path_query(path).0 {
        "/api/command" => "service_command",
        "/api/browser/console" => "browser_console",
        "/json/list" => "cdp_target_list",
        "/api/service/status" => "service_status",
        "/api/service/resources" => "service_resources",
        "/api/service/contracts" => "service_contracts",
        "/api/service/browser-capability-registry" => "browser_capability_registry",
        "/api/service/request" => "service_request",
        path if path.starts_with("/api/service/") => "service_api",
        path if path.starts_with("/api/stream/") => "stream_api",
        path if path.starts_with("/api/") => "dashboard_api",
        _ => "other",
    }
}

fn dashboard_http_method_class(method: &str) -> &'static str {
    match method {
        "GET" => "GET",
        "POST" => "POST",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "OPTIONS" => "OPTIONS",
        _ => "OTHER",
    }
}

fn dashboard_http_status_class(status: Option<u16>) -> &'static str {
    match status {
        Some(100..=199) => "1xx",
        Some(200..=299) => "2xx",
        Some(300..=399) => "3xx",
        Some(400..=499) => "4xx",
        Some(500..=599) => "5xx",
        Some(_) => "other",
        None => "none",
    }
}

fn dashboard_http_body_class(response: &[u8]) -> &'static str {
    match http_response_body(response) {
        None => "missing",
        Some([]) => "empty",
        Some(body) if serde_json::from_slice::<Value>(body).is_ok() => "json",
        Some(_) => "non_json",
    }
}

fn dashboard_backend_error_stage(error: &DashboardReadinessError) -> &'static str {
    match error
        .details
        .as_ref()
        .and_then(|details| details.get("stage"))
        .and_then(Value::as_str)
    {
        Some("connect") => "connect",
        Some("write") => "write",
        Some("read") => "read",
        Some("response") => "response",
        Some("request") => "request",
        Some("owner_panic") => "owner_panic",
        Some("owner_cancelled") => "owner_cancelled",
        _ => "proxy",
    }
}

fn dashboard_http_failure_record(telemetry: &DashboardHttpTelemetry) -> ServiceFailureRecord {
    ServiceFailureRecord::new(
        ServiceFailureCategory::DashboardAction,
        "dashboard_http_gateway",
        telemetry.stage,
        telemetry.backend_error_class.unwrap_or("backend_http_5xx"),
        "Dashboard HTTP gateway request failed.",
    )
    .with_action("dashboard_http_proxy")
    .with_details(serde_json::to_value(telemetry).unwrap_or_else(|_| json!({})))
}

fn emit_dashboard_http_telemetry(telemetry: DashboardHttpTelemetry) {
    let encoded = serde_json::to_string(&telemetry)
        .unwrap_or_else(|_| r#"{"event":"dashboard_http_telemetry_encode_failed"}"#.to_string());
    eprintln!("agent_browser_dashboard_http_telemetry {encoded}");
    if telemetry.failed() {
        let record = dashboard_http_failure_record(&telemetry);
        // Unit tests inject observers and validate the record directly. Never let a
        // provider-free test append synthetic failures to the operator's live journal.
        #[cfg(not(test))]
        append_service_failure_best_effort(&record);
        #[cfg(test)]
        let _ = record;
    }
}

struct DashboardBackendIoPhase<'a> {
    timeout_code: &'static str,
    timeout_message: String,
    failure_message: String,
    port: u16,
    path: &'a str,
    stage: &'static str,
    timeout: Duration,
}

async fn run_dashboard_backend_io_phase<T>(
    phase: DashboardBackendIoPhase<'_>,
    future: impl std::future::Future<Output = std::io::Result<T>>,
) -> Result<T, DashboardReadinessError> {
    timeout(phase.timeout, future)
        .await
        .map_err(|_| {
            DashboardReadinessError::local_backend(
                phase.timeout_code,
                phase.timeout_message,
                phase.port,
                phase.path,
                phase.stage,
            )
        })?
        .map_err(|error| {
            DashboardReadinessError::local_backend(
                "backend_unavailable",
                format!("{}: {error}", phase.failure_message),
                phase.port,
                phase.path,
                phase.stage,
            )
        })
}

fn dashboard_service_status_cacheable(method: &str, path: &str) -> bool {
    method == "GET"
        && matches!(
            split_path_query(path).0,
            "/api/service/status"
                | "/api/service/resources"
                | "/api/service/contracts"
                | "/api/service/browser-capability-registry"
                | "/api/tabs"
        )
}

fn service_api_handler_backend_response(
    method: &str,
    path: &str,
    response: Vec<u8>,
    port: u16,
) -> Result<Vec<u8>, DashboardReadinessError> {
    if dashboard_service_status_cacheable(method, path) {
        return Ok(response);
    }
    require_json_backend_response(response, port, method, path)
}

fn dashboard_service_status_cache() -> &'static Mutex<DashboardServiceStatusCache> {
    static CACHE: OnceLock<Mutex<DashboardServiceStatusCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(DashboardServiceStatusCache::default()))
}

fn guacamole_primary_claim_registry() -> &'static Mutex<GuacamolePrimaryClaimRegistry> {
    static REGISTRY: OnceLock<Mutex<GuacamolePrimaryClaimRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(GuacamolePrimaryClaimRegistry::default()))
}

async fn guacamole_primary_claim_response(body: &str) -> (&'static str, Value) {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return (
            "400 Bad Request",
            json!({ "success": false, "code": "invalid_guacamole_primary_claim" }),
        );
    };
    let route_id = value["routeId"].as_str().map(str::trim).unwrap_or("");
    let connection_id = value["connectionId"].as_str().map(str::trim).unwrap_or("");
    if route_id.is_empty()
        || route_id.len() > 256
        || connection_id.is_empty()
        || connection_id.len() > 256
    {
        return (
            "400 Bad Request",
            json!({ "success": false, "code": "invalid_guacamole_primary_claim" }),
        );
    }

    let (granted, remaining) = guacamole_primary_claim_registry().lock().await.claim(
        GuacamolePrimaryClaimKey {
            route_id: route_id.to_string(),
            connection_id: connection_id.to_string(),
        },
        Instant::now(),
    );
    (
        "200 OK",
        json!({
            "success": true,
            "granted": granted,
            "retryAfterMs": remaining.as_millis().min(u128::from(u32::MAX)) as u32,
        }),
    )
}

async fn proxy_dashboard_service_api_request(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    request_timeout: Duration,
) -> Result<Vec<u8>, DashboardReadinessError> {
    proxy_dashboard_service_api_request_observed(
        port,
        method,
        path,
        body,
        request_timeout,
        production_dashboard_http_observer(),
    )
    .await
}

async fn proxy_dashboard_service_api_request_observed(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    request_timeout: Duration,
    observer: DashboardHttpObserver,
) -> Result<Vec<u8>, DashboardReadinessError> {
    proxy_dashboard_service_api_request_with_observers(
        port,
        method,
        path,
        body,
        request_timeout,
        observer,
        production_dashboard_logical_failure_observer(),
    )
    .await
}

async fn proxy_dashboard_service_api_request_with_observers(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    request_timeout: Duration,
    observer: DashboardHttpObserver,
    logical_failure_observer: DashboardLogicalFailureObserver,
) -> Result<Vec<u8>, DashboardReadinessError> {
    if !dashboard_service_status_cacheable(method, path) {
        let result = proxy_local_http_api_request_with_timeout_observed(
            port,
            method,
            path,
            body,
            request_timeout,
            move |telemetry| observer(telemetry),
        )
        .await;
        observe_dashboard_logical_failure(method, path, None, &result, &logical_failure_observer);
        return result;
    }

    let key = DashboardServiceStatusCacheKey {
        backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
        port,
        path: path.to_string(),
    };
    let mut cache = dashboard_service_status_cache().lock().await;
    prune_expired_dashboard_status_entries(&mut cache);
    if let Some(entry) = cache.entries.get(&key) {
        match entry {
            DashboardServiceStatusCacheEntry::Ready { response, .. } => {
                return Ok(response.clone());
            }
            DashboardServiceStatusCacheEntry::InFlight {
                request_id, result, ..
            } => {
                let request_id = *request_id;
                let result = result.clone();
                drop(cache);
                let result = await_dashboard_status_flight(result, port, path).await;
                observe_dashboard_logical_failure(
                    method,
                    path,
                    Some(request_id),
                    &result,
                    &logical_failure_observer,
                );
                return result;
            }
        }
    }

    evict_oldest_ready_dashboard_status_entry(&mut cache);
    if cache.entries.len() >= DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS {
        drop(cache);
        let result = proxy_local_http_api_request_with_timeout_observed(
            port,
            method,
            path,
            body,
            request_timeout,
            move |telemetry| observer(telemetry),
        )
        .await;
        observe_dashboard_logical_failure(method, path, None, &result, &logical_failure_observer);
        return result;
    }

    cache.next_request_id = cache.next_request_id.wrapping_add(1).max(1);
    let request_id = cache.next_request_id;
    let (result_tx, result_rx) = watch::channel(None);
    let registered_at = Instant::now();
    cache.entries.insert(
        key.clone(),
        DashboardServiceStatusCacheEntry::InFlight {
            request_id,
            registered_at,
            result: result_rx.clone(),
            owner_abort: None,
        },
    );
    drop(cache);

    let owned_method = method.to_string();
    let owned_path = path.to_string();
    let owned_body = body.to_string();
    let owner = tokio::spawn(async move {
        let cleanup = DashboardStatusFlightCleanup::new(DashboardStatusFlightContext {
            key: key.clone(),
            request_id,
            port,
            method: owned_method.clone(),
            path: owned_path.clone(),
            result: result_tx,
            observer,
            started_at: registered_at,
        });
        run_dashboard_status_flight_owner(
            cleanup,
            key,
            request_id,
            proxy_local_http_api_request_unobserved(
                port,
                &owned_method,
                &owned_path,
                &owned_body,
                request_timeout,
            ),
        )
        .await;
    });
    let owner_abort = owner.abort_handle();
    let mut cache = dashboard_service_status_cache().lock().await;
    if let Some(DashboardServiceStatusCacheEntry::InFlight {
        request_id: current,
        owner_abort: slot,
        ..
    }) = cache.entries.get_mut(&DashboardServiceStatusCacheKey {
        backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
        port,
        path: path.to_string(),
    }) {
        if *current == request_id {
            *slot = Some(owner_abort);
        }
    }
    drop(cache);

    let result = await_dashboard_status_flight(result_rx, port, path).await;
    observe_dashboard_logical_failure(
        method,
        path,
        Some(request_id),
        &result,
        &logical_failure_observer,
    );
    result
}

fn observe_dashboard_logical_failure(
    method: &str,
    path: &str,
    flight_request_id: Option<u64>,
    result: &Result<Vec<u8>, DashboardReadinessError>,
    observer: &DashboardLogicalFailureObserver,
) {
    let status = result
        .as_ref()
        .ok()
        .and_then(|response| http_response_status(response));
    let backend_error_class = result.as_ref().err().map(|error| error.code);
    if status.is_none_or(|value| value < 500) && backend_error_class.is_none() {
        return;
    }
    let logical_sequence = DASHBOARD_LOGICAL_REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let flight_id = opaque_identifier_hash(&format!(
        "dashboard-flight:{flight_request_id:?}:{}:{}",
        dashboard_http_method_class(method),
        dashboard_http_route_class(path)
    ));
    let logical_request_id = opaque_identifier_hash(&format!(
        "dashboard-logical-request:{logical_sequence}:{flight_id}"
    ));
    observer(
        ServiceFailureRecord::new(
            ServiceFailureCategory::DashboardAction,
            "dashboard_http_logical_request",
            "response",
            backend_error_class.unwrap_or("backend_http_5xx"),
            "Dashboard HTTP logical request failed.",
        )
        .with_action("dashboard_http_proxy")
        .with_references(ServiceFailureReferences {
            trace_id: Some(flight_id),
            ..ServiceFailureReferences::default()
        })
        .with_details(json!({
            "event": "dashboard_http_logical_request_failed",
            "logicalRequestId": logical_request_id,
            "routeClass": dashboard_http_route_class(path),
            "method": dashboard_http_method_class(method),
            "status": status,
            "statusClass": dashboard_http_status_class(status),
            "bodyClass": result
                .as_ref()
                .ok()
                .map_or("none", |response| dashboard_http_body_class(response)),
        })),
    );
}

struct DashboardStatusFlightCleanup {
    key: DashboardServiceStatusCacheKey,
    request_id: u64,
    port: u16,
    method: String,
    path: String,
    result: watch::Sender<Option<Result<Vec<u8>, DashboardReadinessError>>>,
    observer: DashboardHttpObserver,
    started_at: Instant,
    inflight_count: usize,
    _inflight_guard: DashboardProxyInflightGuard,
    published: bool,
    observed: bool,
    armed: bool,
}

struct DashboardStatusFlightContext {
    key: DashboardServiceStatusCacheKey,
    request_id: u64,
    port: u16,
    method: String,
    path: String,
    result: watch::Sender<Option<Result<Vec<u8>, DashboardReadinessError>>>,
    observer: DashboardHttpObserver,
    started_at: Instant,
}

impl DashboardStatusFlightCleanup {
    fn new(context: DashboardStatusFlightContext) -> Self {
        let (inflight_guard, inflight_count) = DashboardProxyInflightGuard::enter();
        Self {
            key: context.key,
            request_id: context.request_id,
            port: context.port,
            method: context.method,
            path: context.path,
            result: context.result,
            observer: context.observer,
            started_at: context.started_at,
            inflight_count,
            _inflight_guard: inflight_guard,
            published: false,
            observed: false,
            armed: true,
        }
    }

    fn publish(&mut self, result: Result<Vec<u8>, DashboardReadinessError>) {
        let _ = self.result.send(Some(result.clone()));
        self.published = true;
        self.observe(&result);
    }

    fn observe(&mut self, result: &Result<Vec<u8>, DashboardReadinessError>) {
        if self.observed {
            return;
        }
        self.observed = true;
        if let Some(telemetry) = dashboard_http_terminal_telemetry(
            &self.method,
            &self.path,
            self.started_at.elapsed(),
            self.inflight_count,
            result,
        ) {
            (self.observer)(telemetry);
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

async fn run_dashboard_status_flight_owner<F>(
    mut cleanup: DashboardStatusFlightCleanup,
    key: DashboardServiceStatusCacheKey,
    request_id: u64,
    request: F,
) where
    F: Future<Output = Result<Vec<u8>, DashboardReadinessError>>,
{
    let request = std::panic::AssertUnwindSafe(request).catch_unwind().await;
    let result = match request {
        Ok(result) => result,
        Err(_) => Err(DashboardReadinessError::local_backend(
            "backend_unavailable",
            format!(
                "service status backend task panicked for 127.0.0.1:{}{}",
                cleanup.port, cleanup.path
            ),
            cleanup.port,
            &cleanup.path,
            "owner_panic",
        )),
    };
    cleanup.publish(result.clone());
    let mut cache = dashboard_service_status_cache().lock().await;
    apply_dashboard_status_flight_completion(&mut cache, key, request_id, &result, Instant::now());
    cleanup.disarm();
}

impl Drop for DashboardStatusFlightCleanup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if !self.published {
            let error = DashboardReadinessError::local_backend(
                "backend_unavailable",
                format!(
                    "service status backend task ended before publishing 127.0.0.1:{}{}",
                    self.port, self.path
                ),
                self.port,
                &self.path,
                "owner_cancelled",
            );
            let result = Err(error);
            let _ = self.result.send(Some(result.clone()));
            self.observe(&result);
        }
        let key = self.key.clone();
        let request_id = self.request_id;
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let mut cache = dashboard_service_status_cache().lock().await;
                if matches!(
                    cache.entries.get(&key),
                    Some(DashboardServiceStatusCacheEntry::InFlight {
                        request_id: current,
                        ..
                    }) if *current == request_id
                ) {
                    cache.entries.remove(&key);
                }
            });
        }
    }
}

fn apply_dashboard_status_flight_completion(
    cache: &mut DashboardServiceStatusCache,
    key: DashboardServiceStatusCacheKey,
    request_id: u64,
    result: &Result<Vec<u8>, DashboardReadinessError>,
    completed_at: Instant,
) {
    let still_owned = matches!(
        cache.entries.get(&key),
        Some(DashboardServiceStatusCacheEntry::InFlight {
            request_id: current,
            ..
        }) if *current == request_id
    );
    if !still_owned {
        return;
    }
    match result {
        Ok(response)
            if http_response_status(response)
                .is_some_and(|status| (200..300).contains(&status)) =>
        {
            cache.entries.insert(
                key,
                DashboardServiceStatusCacheEntry::Ready {
                    completed_at,
                    response: response.clone(),
                },
            );
        }
        _ => {
            cache.entries.remove(&key);
        }
    }
}

fn prune_expired_dashboard_status_entries(cache: &mut DashboardServiceStatusCache) {
    cache.entries.retain(|_, entry| match entry {
        DashboardServiceStatusCacheEntry::Ready { completed_at, .. } => {
            completed_at.elapsed() < DASHBOARD_SERVICE_STATUS_CACHE_TTL
        }
        DashboardServiceStatusCacheEntry::InFlight { .. } => true,
    });
}

fn evict_oldest_ready_dashboard_status_entry(cache: &mut DashboardServiceStatusCache) {
    if cache.entries.len() < DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS {
        return;
    }
    let oldest = cache
        .entries
        .iter()
        .filter_map(|(key, entry)| match entry {
            DashboardServiceStatusCacheEntry::Ready { completed_at, .. } => {
                Some((key.clone(), *completed_at))
            }
            DashboardServiceStatusCacheEntry::InFlight { registered_at, .. } => {
                let _ = registered_at;
                None
            }
        })
        .min_by_key(|(_, completed_at)| *completed_at)
        .map(|(key, _)| key);
    if let Some(key) = oldest {
        cache.entries.remove(&key);
    }
}

async fn await_dashboard_status_flight(
    mut result: watch::Receiver<Option<Result<Vec<u8>, DashboardReadinessError>>>,
    port: u16,
    path: &str,
) -> Result<Vec<u8>, DashboardReadinessError> {
    loop {
        if let Some(result) = result.borrow().clone() {
            return result;
        }
        if result.changed().await.is_err() {
            return Err(DashboardReadinessError::local_backend(
                "backend_unavailable",
                format!(
                    "service status backend task ended before publishing 127.0.0.1:{port}{path}"
                ),
                port,
                path,
                "request",
            ));
        }
    }
}

fn http_response_status(response: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(response).ok()?;
    let status = text.lines().next()?.split_whitespace().nth(1)?;
    status.parse::<u16>().ok()
}

fn http_response_content_length(response: &[u8]) -> Option<usize> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")?;
    let headers = std::str::from_utf8(&response[..header_end]).ok()?;
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

fn http_response_body(response: &[u8]) -> Option<&[u8]> {
    response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| &response[index + 4..])
}

fn authenticated_candidate_handoff_commit_id(body: &str, response: &[u8]) -> Option<String> {
    let request = serde_json::from_str::<Value>(body).ok()?;
    if request.get("action").and_then(Value::as_str) != Some("service_remote_view_handoff_resolve")
    {
        return None;
    }
    let handoff_id = request
        .pointer("/params/handoffId")
        .or_else(|| request.get("handoffId"))
        .and_then(Value::as_str)?
        .trim();
    if handoff_id.is_empty() || !matches!(http_response_status(response), Some(200..=299)) {
        return None;
    }
    let payload = serde_json::from_slice::<Value>(http_response_body(response)?).ok()?;
    let ready = payload.get("success").and_then(Value::as_bool) == Some(true)
        && payload.pointer("/data/resolved").and_then(Value::as_bool) == Some(true)
        && payload.pointer("/data/status").and_then(Value::as_str) == Some("ready");
    ready.then(|| handoff_id.to_string())
}

/// Keep infrastructure-only route URLs out of the authenticated public dashboard response.
fn sanitize_dashboard_handoff_response(
    request_body: &str,
    response: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let request = serde_json::from_str::<Value>(request_body)
        .map_err(|err| format!("invalid service request JSON: {err}"))?;
    if request.get("action").and_then(Value::as_str) != Some("service_remote_view_handoff_resolve")
    {
        return Ok(response);
    }

    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "backend response did not include HTTP headers".to_string())?;
    let header = std::str::from_utf8(&response[..header_end])
        .map_err(|err| format!("backend response headers were not UTF-8: {err}"))?;
    let status = header
        .lines()
        .next()
        .and_then(|line| {
            line.strip_prefix("HTTP/1.1 ")
                .or_else(|| line.strip_prefix("HTTP/1.0 "))
        })
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "backend response did not include a valid HTTP status line".to_string())?
        .to_string();
    let mut payload = serde_json::from_slice::<Value>(&response[header_end + 4..])
        .map_err(|err| format!("backend response body was not JSON: {err}"))?;
    strip_dashboard_handoff_provider_urls(&mut payload);
    Ok(json_http_response(&status, payload))
}

fn strip_dashboard_handoff_provider_urls(value: &mut Value) {
    const FORBIDDEN_KEYS: [&str; 5] = [
        "providerExternalUrl",
        "routeBinding",
        "localEmbedUrl",
        "dashboardEmbedUrl",
        "healthUrl",
    ];
    match value {
        Value::Object(object) => {
            for key in FORBIDDEN_KEYS {
                object.remove(key);
            }
            for nested in object.values_mut() {
                strip_dashboard_handoff_provider_urls(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_dashboard_handoff_provider_urls(item);
            }
        }
        _ => {}
    }
}

fn require_json_backend_response(
    response: Vec<u8>,
    port: u16,
    method: &str,
    path: &str,
) -> Result<Vec<u8>, DashboardReadinessError> {
    require_json_backend_response_observed(
        response,
        port,
        method,
        path,
        emit_dashboard_http_telemetry,
    )
}

fn require_json_backend_response_observed(
    response: Vec<u8>,
    port: u16,
    method: &str,
    path: &str,
    observer: impl Fn(DashboardHttpTelemetry),
) -> Result<Vec<u8>, DashboardReadinessError> {
    let status = http_response_status(&response);
    let body_class = dashboard_http_body_class(&response);
    let response_bytes = response.len();
    let result = require_json_backend_response_unobserved(response, port, path);
    if let Err(error) = &result {
        if status.is_none_or(|status| status < 500) {
            observer(DashboardHttpTelemetry {
                event: "dashboard_http_failed",
                route_class: dashboard_http_route_class(path),
                method: dashboard_http_method_class(method),
                status,
                status_class: dashboard_http_status_class(status),
                body_class,
                stage: "response",
                timing_scope: "response_validation",
                elapsed_ms: 0,
                inflight_count: DASHBOARD_PROXY_INFLIGHT.load(Ordering::Relaxed),
                response_bytes: Some(response_bytes),
                backend_error_class: Some(error.code),
            });
        }
    }
    result
}

fn require_json_backend_response_unobserved(
    response: Vec<u8>,
    port: u16,
    path: &str,
) -> Result<Vec<u8>, DashboardReadinessError> {
    let Some(body) = http_response_body(&response) else {
        return Err(DashboardReadinessError::local_backend(
            "invalid_backend_payload",
            format!("backend response from 127.0.0.1:{port}{path} did not include a body"),
            port,
            path,
            "response",
        ));
    };
    if body.is_empty() {
        return Err(DashboardReadinessError::local_backend(
            "backend_empty_response",
            format!("empty JSON response from 127.0.0.1:{port}{path}"),
            port,
            path,
            "response",
        ));
    }
    serde_json::from_slice::<Value>(body).map_err(|err| {
        DashboardReadinessError::local_backend(
            "invalid_backend_payload",
            format!("invalid JSON response from 127.0.0.1:{port}{path}: {err}"),
            port,
            path,
            "response",
        )
    })?;
    Ok(response)
}

async fn read_local_http_response(
    backend: &mut tokio::net::TcpStream,
    port: u16,
    path: &str,
    request_timeout: Duration,
) -> Result<Vec<u8>, DashboardReadinessError> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 8192];
    let max_response_bytes = 16 * 1024 * 1024;
    loop {
        let n = run_dashboard_backend_io_phase(
            DashboardBackendIoPhase {
                timeout_code: "backend_read_timeout",
                timeout_message: format!("timed out reading from 127.0.0.1:{port}{path}"),
                failure_message: format!("failed reading from 127.0.0.1:{port}{path}"),
                port,
                path,
                stage: "read",
                timeout: request_timeout,
            },
            backend.read(&mut buffer),
        )
        .await?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buffer[..n]);
        if response.len() > max_response_bytes {
            return Err(DashboardReadinessError::local_backend(
                "invalid_backend_payload",
                format!("response from 127.0.0.1:{port}{path} exceeded proxy limit"),
                port,
                path,
                "read",
            ));
        }
        if let (Some(body), Some(content_length)) = (
            http_response_body(&response),
            http_response_content_length(&response),
        ) {
            if body.len() >= content_length {
                break;
            }
        }
    }
    if response.is_empty() {
        return Err(DashboardReadinessError::local_backend(
            "backend_empty_response",
            format!("empty response from 127.0.0.1:{port}{path}"),
            port,
            path,
            "response",
        ));
    }
    if http_response_status(&response).is_none() {
        return Err(DashboardReadinessError::local_backend(
            "backend_invalid_http",
            format!("invalid HTTP response from 127.0.0.1:{port}{path}"),
            port,
            path,
            "response",
        ));
    }
    Ok(response)
}

fn json_http_response(status: &str, value: Value) -> Vec<u8> {
    let body = value.to_string();
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn cdp_json_list_to_dashboard_tabs(body: &[u8]) -> Result<Vec<Value>, String> {
    let pages: Vec<Value> = serde_json::from_slice(body).map_err(|err| err.to_string())?;
    Ok(pages
        .into_iter()
        .enumerate()
        .filter_map(|(index, page)| {
            if page.get("type").and_then(Value::as_str) != Some("page") {
                return None;
            }
            let url = page
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let title = page
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let target_id = page
                .get("id")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.to_string());
            let mut tab = json!({
                "index": index,
                "active": index == 0,
                "title": title,
                "url": url,
                "type": "page",
            });
            if let Some(target_id) = target_id {
                tab["targetId"] = json!(target_id);
            }
            Some(tab)
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForeignCdpScreenshotTarget {
    id: String,
    title: String,
    url: String,
    web_socket_debugger_url: String,
}

fn cdp_json_list_screenshot_target(
    body: &[u8],
    requested_target_id: Option<&str>,
) -> Result<ForeignCdpScreenshotTarget, String> {
    let pages: Vec<Value> = serde_json::from_slice(body).map_err(|err| err.to_string())?;
    let requested_target_id = requested_target_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut fallback: Option<ForeignCdpScreenshotTarget> = None;
    let mut requested_found_without_ws = false;

    for page in pages {
        if page.get("type").and_then(Value::as_str) != Some("page") {
            continue;
        }
        let id = page
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if id.is_empty() {
            continue;
        }
        let target = page
            .get("webSocketDebuggerUrl")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(|web_socket_debugger_url| ForeignCdpScreenshotTarget {
                id: id.clone(),
                title: page
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                url: page
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                web_socket_debugger_url: web_socket_debugger_url.to_string(),
            });
        if requested_target_id == Some(id.as_str()) {
            if let Some(target) = target {
                return Ok(target);
            }
            requested_found_without_ws = true;
        } else if fallback.is_none() {
            fallback = target;
        }
    }

    if requested_found_without_ws {
        return Err("Requested CDP target does not expose a page WebSocket".to_string());
    }
    if requested_target_id.is_some() {
        return Err("Requested CDP target is no longer present".to_string());
    }
    fallback.ok_or_else(|| "No screenshot-capable CDP page target was found".to_string())
}

fn bounded_input_number(input: &Value, field: &str, max_abs: f64) -> Result<f64, String> {
    let value = input
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("Foreign CDP input requires numeric {field}"))?;
    if !value.is_finite() || value.abs() > max_abs {
        return Err(format!(
            "Foreign CDP input {field} is outside the allowed range"
        ));
    }
    Ok(value)
}

/// Convert a dashboard input event into one of the fixed CDP input commands.
/// Arbitrary CDP methods, script evaluation, navigation, and lifecycle commands
/// are intentionally not accepted by this boundary.
fn foreign_cdp_input_command(input: &Value) -> Result<(&'static str, Value), String> {
    let kind = input
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "Foreign CDP input requires a kind".to_string())?;
    match kind {
        "mouse" => {
            let event_type = input
                .get("eventType")
                .and_then(Value::as_str)
                .ok_or_else(|| "Mouse input requires eventType".to_string())?;
            if !matches!(event_type, "mousePressed" | "mouseReleased" | "mouseMoved") {
                return Err("Mouse eventType is not allowed".to_string());
            }
            let x = bounded_input_number(input, "x", 100_000.0)?;
            let y = bounded_input_number(input, "y", 100_000.0)?;
            if x < 0.0 || y < 0.0 {
                return Err("Mouse coordinates must not be negative".to_string());
            }
            let button = input
                .get("button")
                .and_then(Value::as_str)
                .unwrap_or("none");
            if !matches!(
                button,
                "none" | "left" | "middle" | "right" | "back" | "forward"
            ) {
                return Err("Mouse button is not allowed".to_string());
            }
            let click_count = input.get("clickCount").and_then(Value::as_u64).unwrap_or(0);
            if click_count > 3 {
                return Err("Mouse clickCount is outside the allowed range".to_string());
            }
            let mut params = json!({
                "type": event_type,
                "x": x,
                "y": y,
                "button": button,
                "clickCount": click_count,
            });
            if let Some(modifiers) = input.get("modifiers").and_then(Value::as_u64) {
                if modifiers > 15 {
                    return Err("Mouse modifiers are outside the allowed range".to_string());
                }
                params["modifiers"] = json!(modifiers);
            }
            Ok(("Input.dispatchMouseEvent", params))
        }
        "wheel" => {
            let x = bounded_input_number(input, "x", 100_000.0)?;
            let y = bounded_input_number(input, "y", 100_000.0)?;
            if x < 0.0 || y < 0.0 {
                return Err("Wheel coordinates must not be negative".to_string());
            }
            Ok((
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mouseWheel",
                    "x": x,
                    "y": y,
                    "deltaX": bounded_input_number(input, "deltaX", 100_000.0)?,
                    "deltaY": bounded_input_number(input, "deltaY", 100_000.0)?,
                }),
            ))
        }
        "keyboard" => {
            let event_type = input
                .get("eventType")
                .and_then(Value::as_str)
                .ok_or_else(|| "Keyboard input requires eventType".to_string())?;
            if !matches!(event_type, "keyDown" | "keyUp" | "char") {
                return Err("Keyboard eventType is not allowed".to_string());
            }
            let key = input.get("key").and_then(Value::as_str).unwrap_or("");
            let code = input.get("code").and_then(Value::as_str).unwrap_or("");
            let text = input.get("text").and_then(Value::as_str).unwrap_or("");
            if key.len() > 64 || code.len() > 64 || text.len() > 4096 {
                return Err("Keyboard input exceeds the allowed size".to_string());
            }
            let mut params = json!({
                "type": event_type,
                "key": key,
                "code": code,
            });
            if !text.is_empty() {
                params["text"] = json!(text);
            }
            if let Some(modifiers) = input.get("modifiers").and_then(Value::as_u64) {
                if modifiers > 15 {
                    return Err("Keyboard modifiers are outside the allowed range".to_string());
                }
                params["modifiers"] = json!(modifiers);
            }
            Ok(("Input.dispatchKeyEvent", params))
        }
        _ => Err("Foreign CDP input kind is not allowed".to_string()),
    }
}

async fn foreign_cdp_tabs_response(port: u16) -> Result<Vec<u8>, String> {
    let response = proxy_local_http_api_request(port, "GET", "/json/list", "")
        .await
        .map_err(|err| err.to_string())?;
    let status = http_response_status(&response).unwrap_or(0);
    if status != 200 {
        return Err(format!("CDP /json/list returned HTTP {status}"));
    }
    let body = http_response_body(&response)
        .ok_or_else(|| "CDP /json/list response missing body".to_string())?;
    let tabs = cdp_json_list_to_dashboard_tabs(body)?;
    Ok(json_http_response("200 OK", json!(tabs)))
}

async fn capture_foreign_cdp_screenshot(
    target: &ForeignCdpScreenshotTarget,
    format: &str,
) -> Result<String, String> {
    let mut params = json!({
        "format": format,
        "fromSurface": true,
    });
    if format == "jpeg" {
        params["quality"] = json!(60);
    }
    let result = send_foreign_cdp_command(target, "Page.captureScreenshot", params).await?;
    result
        .get("data")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| "CDP screenshot response did not include image data".to_string())
}

async fn send_foreign_cdp_command(
    target: &ForeignCdpScreenshotTarget,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let connect = timeout(
        DASHBOARD_CDP_SCREENSHOT_TIMEOUT,
        tokio_tungstenite::connect_async(&target.web_socket_debugger_url),
    )
    .await
    .map_err(|_| "Timed out connecting to CDP page WebSocket".to_string())?;
    let (mut ws, _) = connect.map_err(|err| format!("CDP page WebSocket connect failed: {err}"))?;
    let command = json!({
        "id": 1,
        "method": method,
        "params": params,
    });
    ws.send(Message::Text(command.to_string()))
        .await
        .map_err(|err| format!("CDP command send failed: {err}"))?;

    let response = timeout(DASHBOARD_CDP_SCREENSHOT_TIMEOUT, async {
        while let Some(message) = ws.next().await {
            let message = message.map_err(|err| format!("CDP command response failed: {err}"))?;
            let Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)
                .map_err(|err| format!("CDP command response was not JSON: {err}"))?;
            if value.get("id").and_then(Value::as_u64) != Some(1) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("CDP command failed: {error}"));
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| "CDP command response did not include a result".to_string());
        }
        Err("CDP page WebSocket closed before a command response arrived".to_string())
    })
    .await
    .map_err(|_| "Timed out waiting for CDP command response".to_string())??;

    let _ = ws.close(None).await;
    Ok(response)
}

async fn resolve_foreign_cdp_target(
    port: u16,
    requested_target_id: Option<&str>,
) -> Result<ForeignCdpScreenshotTarget, String> {
    let response = proxy_local_http_api_request(port, "GET", "/json/list", "")
        .await
        .map_err(|err| err.to_string())?;
    let status = http_response_status(&response).unwrap_or(0);
    if status != 200 {
        return Err(format!("CDP /json/list returned HTTP {status}"));
    }
    let body = http_response_body(&response)
        .ok_or_else(|| "CDP /json/list response missing body".to_string())?;
    cdp_json_list_screenshot_target(body, requested_target_id)
}

async fn foreign_cdp_screenshot_response(
    port: u16,
    requested_target_id: Option<&str>,
    format: &str,
) -> Result<Vec<u8>, String> {
    let target = resolve_foreign_cdp_target(port, requested_target_id).await?;
    let data = capture_foreign_cdp_screenshot(&target, format).await?;
    Ok(json_http_response(
        "200 OK",
        json!({
            "success": true,
            "provider": "cdp_snapshot",
            "port": port,
            "targetId": target.id,
            "title": target.title,
            "url": target.url,
            "format": format,
            "data": data,
            "dataUrl": format!("data:image/{};base64,{}", format, data),
        }),
    ))
}

fn detected_foreign_cdp_port(port: u16) -> Result<bool, String> {
    let sessions: Vec<Value> = serde_json::from_str(&discover_sessions())
        .map_err(|err| format!("Could not read detected browser inventory: {err}"))?;
    Ok(sessions.iter().any(|session| {
        session.get("port").and_then(Value::as_u64) == Some(u64::from(port))
            && session.get("ownership").and_then(Value::as_str) == Some("foreign_cdp")
    }))
}

fn foreign_cdp_grant_json(grant: &foreign_cdp_control::ForeignCdpBorrowGrant) -> Value {
    json!({
        "active": true,
        "grantId": grant.id,
        "port": grant.port,
        "targetId": grant.target_id,
        "owner": grant.owner,
        "reason": grant.reason,
        "issuedAt": grant.issued_at.to_rfc3339(),
        "expiresAt": grant.expires_at.to_rfc3339(),
        "allowedOperations": ["pointer", "keyboard", "wheel"],
        "lifecycleOwnership": false,
    })
}

fn required_request_port(body: &Value) -> Result<u16, String> {
    body.get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
        .ok_or_else(|| "Missing or invalid foreign CDP port".to_string())
}

fn required_request_string<'a>(body: &'a Value, field: &str) -> Result<&'a str, String> {
    body.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Missing or invalid {field}"))
}

async fn handle_foreign_cdp_control_status(
    stream: &mut tokio::net::TcpStream,
    query: Option<&str>,
) {
    let Some(port) = query_value(query, "port").and_then(|value| value.parse::<u16>().ok()) else {
        write_json_error(
            stream,
            "400 Bad Request",
            "Missing or invalid foreign CDP port",
        )
        .await;
        return;
    };
    let Some(target_id) = query_value(query, "targetId") else {
        write_json_error(stream, "400 Bad Request", "Missing foreign CDP targetId").await;
        return;
    };
    match foreign_cdp_control::status(port, &target_id) {
        Ok(Some(grant)) => {
            write_json_value(stream, "200 OK", foreign_cdp_grant_json(&grant)).await;
        }
        Ok(None) => {
            write_json_value(
                stream,
                "200 OK",
                json!({
                    "active": false,
                    "port": port,
                    "targetId": target_id,
                    "allowedOperations": [],
                    "lifecycleOwnership": false,
                }),
            )
            .await;
        }
        Err(err) => write_json_error(stream, "500 Internal Server Error", &err).await,
    }
}

async fn handle_foreign_cdp_control_request(
    stream: &mut tokio::net::TcpStream,
    path: &str,
    body_str: &str,
    operator: &str,
) {
    let body: Value = match serde_json::from_str(body_str) {
        Ok(body) => body,
        Err(err) => {
            write_json_error(
                stream,
                "400 Bad Request",
                &format!("Invalid foreign CDP control request JSON: {err}"),
            )
            .await;
            return;
        }
    };
    let port = match required_request_port(&body) {
        Ok(port) => port,
        Err(err) => {
            write_json_error(stream, "400 Bad Request", &err).await;
            return;
        }
    };
    let target_id = match required_request_string(&body, "targetId") {
        Ok(target_id) => target_id,
        Err(err) => {
            write_json_error(stream, "400 Bad Request", &err).await;
            return;
        }
    };

    if path == "/api/foreign-cdp/borrow" {
        match detected_foreign_cdp_port(port) {
            Ok(true) => {}
            Ok(false) => {
                write_json_error(
                    stream,
                    "409 Conflict",
                    "Borrow is available only for a currently detected foreign CDP browser",
                )
                .await;
                return;
            }
            Err(err) => {
                write_json_error(stream, "500 Internal Server Error", &err).await;
                return;
            }
        }
        if let Err(err) = resolve_foreign_cdp_target(port, Some(target_id)).await {
            write_json_error(stream, "409 Conflict", &err).await;
            return;
        }
        let reason = match required_request_string(&body, "reason") {
            Ok(reason) => reason,
            Err(err) => {
                write_json_error(stream, "400 Bad Request", &err).await;
                return;
            }
        };
        let ttl_seconds = body
            .get("ttlSeconds")
            .and_then(Value::as_u64)
            .unwrap_or(300);
        match foreign_cdp_control::borrow(port, target_id, operator, reason, ttl_seconds) {
            Ok(grant) => {
                eprintln!(
                    "Foreign CDP Borrow granted: operator={} port={} target={} expires={}",
                    operator,
                    port,
                    target_id,
                    grant.expires_at.to_rfc3339()
                );
                write_json_value(stream, "200 OK", foreign_cdp_grant_json(&grant)).await;
            }
            Err(err) => write_json_error(stream, "409 Conflict", &err).await,
        }
        return;
    }

    let grant_id = match required_request_string(&body, "grantId") {
        Ok(grant_id) => grant_id,
        Err(err) => {
            write_json_error(stream, "400 Bad Request", &err).await;
            return;
        }
    };

    if path == "/api/foreign-cdp/release" {
        match foreign_cdp_control::release(port, target_id, grant_id, operator) {
            Ok(grant) => {
                eprintln!(
                    "Foreign CDP Borrow released: operator={} port={} target={}",
                    operator, port, target_id
                );
                write_json_value(
                    stream,
                    "200 OK",
                    json!({
                        "active": false,
                        "released": true,
                        "port": grant.port,
                        "targetId": grant.target_id,
                        "lifecycleOwnership": false,
                    }),
                )
                .await;
            }
            Err(err) => write_json_error(stream, "403 Forbidden", &err).await,
        }
        return;
    }

    let input = match body.get("input") {
        Some(input) => input,
        None => {
            write_json_error(stream, "400 Bad Request", "Missing foreign CDP input").await;
            return;
        }
    };
    let (method, params) = match foreign_cdp_input_command(input) {
        Ok(command) => command,
        Err(err) => {
            write_json_error(stream, "400 Bad Request", &err).await;
            return;
        }
    };
    let grant = match foreign_cdp_control::authorize(port, target_id, grant_id, operator) {
        Ok(grant) => grant,
        Err(err) => {
            write_json_error(stream, "403 Forbidden", &err).await;
            return;
        }
    };
    let target = match resolve_foreign_cdp_target(port, Some(target_id)).await {
        Ok(target) => target,
        Err(err) => {
            write_json_error(stream, "409 Conflict", &err).await;
            return;
        }
    };
    match send_foreign_cdp_command(&target, method, params).await {
        Ok(_) => {
            write_json_value(
                stream,
                "200 OK",
                json!({
                    "success": true,
                    "port": port,
                    "targetId": target_id,
                    "expiresAt": grant.expires_at.to_rfc3339(),
                    "lifecycleOwnership": false,
                }),
            )
            .await;
        }
        Err(err) => write_json_error(stream, "502 Bad Gateway", &err).await,
    }
}

async fn handle_session_tabs_api_request(stream: &mut tokio::net::TcpStream, query: Option<&str>) {
    let Some(port) = query_value(query, "port").and_then(|value| value.parse::<u16>().ok()) else {
        write_json_error(stream, "400 Bad Request", "Missing or invalid session port").await;
        return;
    };

    match proxy_dashboard_service_api_request(
        port,
        "GET",
        "/api/tabs",
        "",
        DASHBOARD_SERVICE_STATUS_PROXY_TIMEOUT,
    )
    .await
    {
        Ok(response) if http_response_status(&response) == Some(200) => {
            let _ = stream.write_all(&response).await;
        }
        Ok(_) | Err(_) => match foreign_cdp_tabs_response(port).await {
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
        },
    }
}

async fn handle_session_screenshot_api_request(
    stream: &mut tokio::net::TcpStream,
    query: Option<&str>,
) {
    let Some(port) = query_value(query, "port").and_then(|value| value.parse::<u16>().ok()) else {
        write_json_error(stream, "400 Bad Request", "Missing or invalid session port").await;
        return;
    };
    let requested_target_id = query_value(query, "targetId");
    let format = match query_value(query, "format")
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "png",
        _ => "jpeg",
    };

    match foreign_cdp_screenshot_response(port, requested_target_id.as_deref(), format).await {
        Ok(response) => {
            let _ = stream.write_all(&response).await;
        }
        Err(err) => {
            let readiness_error = normalize_screenshot_error(err);
            write_json_error_with_code(
                stream,
                "502 Bad Gateway",
                &format!("Session screenshot proxy failed: {}", readiness_error),
                Some(readiness_error.code),
                readiness_error.details.clone(),
            )
            .await;
        }
    }
}

fn normalize_screenshot_error(message: String) -> DashboardReadinessError {
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("requested cdp target")
        || lower.contains("no screenshot-capable cdp page target")
    {
        "stale_target"
    } else if lower.contains("not json")
        || lower.contains("missing body")
        || lower.contains("did not include image data")
        || lower.contains("returned http")
    {
        "invalid_backend_payload"
    } else if lower.contains("timed out connecting") {
        "backend_connect_timeout"
    } else if lower.contains("timed out") {
        "backend_read_timeout"
    } else if lower.contains("websocket connect failed")
        || lower.contains("websocket closed")
        || lower.contains("response failed")
    {
        "backend_unavailable"
    } else {
        "screenshot_failed"
    };
    DashboardReadinessError::new(code, message).with_details(json!({
        "readinessState": readiness_state_for_gateway_code(code),
        "transport": "cdp_screenshot",
    }))
}

async fn handle_session_console_api_request(
    stream: &mut tokio::net::TcpStream,
    query: Option<&str>,
    body: &str,
) {
    let Some(port) = query_value(query, "port").and_then(|value| value.parse::<u16>().ok()) else {
        write_json_error(stream, "400 Bad Request", "Missing or invalid session port").await;
        return;
    };
    let request_body = if body.trim().is_empty() { "{}" } else { body };
    if let Some(session_name) = query_value(query, "session")
        .and_then(|value| normalize_service_request_session_name(&value))
    {
        let mut command = serde_json::from_str::<Value>(request_body).unwrap_or_else(|_| json!({}));
        command["action"] = json!("console");
        if command.get("id").is_none() {
            command["id"] = json!(format!(
                "dashboard-session-console-{}",
                uuid::Uuid::new_v4()
            ));
        }
        match relay_command_to_daemon(&session_name, &command.to_string()).await {
            Ok(response) => {
                write_json_body(stream, "200 OK", &response).await;
            }
            Err(err) => {
                write_json_error(
                    stream,
                    "502 Bad Gateway",
                    &format!("Session console daemon relay failed: {}", err),
                )
                .await;
            }
        }
        return;
    }

    match proxy_local_http_api_request(port, "POST", "/api/browser/console", request_body).await {
        Ok(response) => {
            let _ = stream.write_all(&response).await;
        }
        Err(err) => {
            write_json_error_with_code(
                stream,
                "502 Bad Gateway",
                &format!("Session console proxy failed: {}", err),
                Some(err.code),
                err.details.clone(),
            )
            .await;
        }
    }
}

async fn write_json_body(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(body.as_bytes()).await;
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
            let incident_id = incident_activity_id.filter(|id| !id.is_empty())?;
            vec![
                "service".to_string(),
                "activity".to_string(),
                incident_id.to_string(),
            ]
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

    exec_agent_browser_args(args)
        .await
        .ok()
        .map(dashboard_cli_fallback_http_response)
}

fn dashboard_cli_fallback_http_response(body: String) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        body.len()
    ) + &body
}

#[cfg(test)]
pub(crate) fn service_status_dashboard_cli_fallback_fixture(body: String) -> Vec<u8> {
    dashboard_cli_fallback_http_response(body).into_bytes()
}

#[cfg(test)]
pub(crate) fn service_status_http_body_fixture(response: &[u8]) -> Option<&[u8]> {
    http_response_body(response)
}

#[cfg(test)]
pub(crate) fn service_status_handler_fixture(response: Vec<u8>) -> Vec<u8> {
    service_api_handler_backend_response(
        "GET",
        "/api/service/status?full-tab-history=false",
        response,
        9222,
    )
    .expect("status handler must forward backend success bytes")
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
    write_json_error_with_code(stream, status, error, None, None).await;
}

async fn write_json_error_with_code(
    stream: &mut tokio::net::TcpStream,
    status: &str,
    error: &str,
    code: Option<&str>,
    details: Option<Value>,
) {
    let response = json_error_http_response(status, error, code, details);
    let _ = stream.write_all(&response).await;
}

fn json_error_http_response(
    status: &str,
    error: &str,
    code: Option<&str>,
    details: Option<Value>,
) -> Vec<u8> {
    let mut body = json!({
        "success": false,
        "error": error,
    });
    if let Some(code) = code {
        body["code"] = json!(code);
    }
    if let Some(details) = details {
        body["details"] = details;
    }
    let body = body.to_string();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        body.len()
    );
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn guacamole_primary_claim_allows_one_owner_until_expiry() {
        let mut registry = GuacamolePrimaryClaimRegistry::default();
        let key = GuacamolePrimaryClaimKey {
            route_id: "route-1".to_string(),
            connection_id: "17".to_string(),
        };
        let start = Instant::now();

        assert_eq!(
            registry.claim(key.clone(), start),
            (true, GUACAMOLE_PRIMARY_CLAIM_TTL)
        );
        let (granted, remaining) = registry.claim(key.clone(), start + Duration::from_secs(1));
        assert!(!granted);
        assert_eq!(remaining, Duration::from_secs(9));
        assert_eq!(
            registry.claim(key, start + GUACAMOLE_PRIMARY_CLAIM_TTL),
            (true, GUACAMOLE_PRIMARY_CLAIM_TTL)
        );
    }

    #[tokio::test]
    async fn dashboard_service_backend_bootstrap_retries_after_runtime_host_convergence() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let observed = attempts.clone();
        let result = ensure_dashboard_service_backend_with_retry(
            move || {
                let attempt = observed.fetch_add(1, Ordering::SeqCst);
                std::future::ready(if attempt == 0 {
                    Err("runtime host is still converging".to_string())
                } else {
                    Ok(())
                })
            },
            Duration::ZERO,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

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
    fn dashboard_service_backend_rejects_foreign_only_sessions() {
        let sessions = vec![
            json!({
                "session": "detected-chatgpt-45013",
                "port": 45013,
                "detected": true,
                "ownership": "foreign_cdp"
            }),
            json!({
                "session": "detected-chatgpt-45015",
                "port": 45015,
                "detected": true,
                "ownership": "foreign_cdp"
            }),
        ];

        assert_eq!(
            dashboard_service_backend_port_from_sessions(&sessions),
            None
        );
    }

    #[test]
    fn dashboard_service_read_cache_coalesces_pressure_sensitive_reads() {
        assert!(dashboard_service_status_cacheable(
            "GET",
            "/api/service/status"
        ));
        assert!(dashboard_service_status_cacheable(
            "GET",
            "/api/service/status?full-tab-history=true"
        ));
        assert!(dashboard_service_status_cacheable(
            "GET",
            "/api/service/resources"
        ));
        assert!(dashboard_service_status_cacheable(
            "GET",
            "/api/service/browser-capability-registry"
        ));
        assert!(dashboard_service_status_cacheable(
            "GET",
            "/api/service/contracts"
        ));
        assert!(dashboard_service_status_cacheable("GET", "/api/tabs"));
        assert!(!dashboard_service_status_cacheable(
            "POST",
            "/api/service/status"
        ));
        assert!(!dashboard_service_status_cacheable(
            "GET",
            "/api/service/jobs"
        ));
        assert_ne!(
            DashboardServiceStatusCacheKey {
                backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                port: 9222,
                path: "/api/service/status?full-tab-history=true".to_string(),
            },
            DashboardServiceStatusCacheKey {
                backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                port: 9222,
                path: "/api/service/status?full-tab-history=false".to_string(),
            }
        );
    }

    #[test]
    fn service_status_handler_forwards_success_bytes_without_json_interpretation() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 8\r\n\r\nnot-json".to_vec();

        assert_eq!(
            service_api_handler_backend_response(
                "GET",
                "/api/service/status?full-tab-history=true",
                response.clone(),
                9222,
            )
            .unwrap(),
            response
        );
        assert_eq!(
            service_api_handler_backend_response(
                "GET",
                "/api/service/jobs",
                b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nnot-json".to_vec(),
                9222,
            )
            .unwrap_err()
            .code,
            "invalid_backend_payload"
        );
    }

    #[tokio::test]
    async fn dashboard_service_resources_single_flight_forwards_backend_bytes_unchanged() {
        let _guard = dashboard_status_cache_test_guard().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_count = request_count.clone();
        let body = r#"{"success":true,"data":{"statusProjection":{"schemaVersion":1}}}"#;
        let expected = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        let server_response = expected.clone();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            server_count.fetch_add(1, Ordering::SeqCst);
            let mut request = vec![0_u8; 4096];
            let _ = stream.read(&mut request).await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
            stream.write_all(&server_response).await.unwrap();
        });
        let path = format!("/api/service/resources?single-flight-port={port}");
        let first =
            proxy_dashboard_service_api_request(port, "GET", &path, "", Duration::from_secs(1));
        let second =
            proxy_dashboard_service_api_request(port, "GET", &path, "", Duration::from_secs(1));
        let (first, second) = tokio::join!(first, second);

        assert_eq!(first.unwrap(), expected);
        assert_eq!(second.unwrap(), expected);
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dashboard_service_status_flight_survives_first_waiter_cancellation() {
        let _guard = dashboard_status_cache_test_guard().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_count = request_count.clone();
        let accepted = Arc::new(tokio::sync::Notify::new());
        let server_accepted = accepted.clone();
        let body = r#"{"success":true,"data":{"survivedCancellation":true}}"#;
        let expected = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        let server_response = expected.clone();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            server_count.fetch_add(1, Ordering::SeqCst);
            let mut request = vec![0_u8; 4096];
            let _ = stream.read(&mut request).await.unwrap();
            server_accepted.notify_one();
            tokio::time::sleep(Duration::from_millis(20)).await;
            stream.write_all(&server_response).await.unwrap();
        });
        let path = format!("/api/service/status?cancelled-waiter-port={port}");
        let first_path = path.clone();
        let first = tokio::spawn(async move {
            proxy_dashboard_service_api_request(
                port,
                "GET",
                &first_path,
                "",
                Duration::from_secs(1),
            )
            .await
        });

        accepted.notified().await;
        first.abort();
        let _ = first.await;
        let second =
            proxy_dashboard_service_api_request(port, "GET", &path, "", Duration::from_secs(1))
                .await
                .unwrap();

        assert_eq!(second, expected);
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dashboard_service_status_owner_cancellation_removes_request_id_and_allows_retry() {
        let _guard = dashboard_status_cache_test_guard().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let first_accepted = Arc::new(tokio::sync::Notify::new());
        let server_accepted = first_accepted.clone();
        let expected_body = r#"{"success":true,"data":{"retried":true}}"#;
        let expected = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            expected_body.len(),
            expected_body
        )
        .into_bytes();
        let server_response = expected.clone();
        tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let _ = first.read(&mut request).await.unwrap();
            server_accepted.notify_one();
            let (mut retry, _) = listener.accept().await.unwrap();
            let _ = retry.read(&mut request).await.unwrap();
            retry.write_all(&server_response).await.unwrap();
        });
        let path = format!("/api/service/status?owned-cancellation-port={port}");
        let journal_root = std::env::temp_dir().join(format!(
            "agent-browser-dashboard-owner-cancel-{}",
            uuid::Uuid::new_v4()
        ));
        let journal_path = journal_root.join("journal.jsonl");
        let observer_journal_path = journal_path.clone();
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = observed.clone();
        let observer: DashboardHttpObserver = Arc::new(move |telemetry| {
            append_service_failure_at(
                &observer_journal_path,
                &dashboard_http_failure_record(&telemetry),
            )
            .unwrap();
            captured.lock().unwrap().push(telemetry);
        });
        let request_path = path.clone();
        let request_observer = observer.clone();
        let first = tokio::spawn(async move {
            proxy_dashboard_service_api_request_observed(
                port,
                "GET",
                &request_path,
                "",
                Duration::from_secs(2),
                request_observer,
            )
            .await
        });
        first_accepted.notified().await;
        let key = DashboardServiceStatusCacheKey {
            backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
            port,
            path: path.clone(),
        };
        let owner_abort = {
            let cache = dashboard_service_status_cache().lock().await;
            match cache.entries.get(&key).unwrap() {
                DashboardServiceStatusCacheEntry::InFlight { owner_abort, .. } => {
                    owner_abort.clone().unwrap()
                }
                DashboardServiceStatusCacheEntry::Ready { .. } => panic!("flight completed early"),
            }
        };
        owner_abort.abort();
        assert!(first.await.unwrap().is_err());
        for _ in 0..100 {
            if !dashboard_service_status_cache()
                .lock()
                .await
                .entries
                .contains_key(&key)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(!dashboard_service_status_cache()
            .lock()
            .await
            .entries
            .contains_key(&key));
        {
            let events = observed.lock().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].stage, "owner_cancelled");
            assert_eq!(events[0].timing_scope, "local_backend_round_trip");
            assert_eq!(events[0].backend_error_class, Some("backend_unavailable"));
        }
        let readback = read_service_failures_at(&journal_path, 10).unwrap();
        assert_eq!(readback.records.len(), 1);
        assert_eq!(readback.records[0].stage, "owner_cancelled");

        let retry = proxy_dashboard_service_api_request_observed(
            port,
            "GET",
            &path,
            "",
            Duration::from_secs(2),
            observer,
        )
        .await
        .unwrap();
        assert_eq!(retry, expected);
        assert_eq!(observed.lock().unwrap().len(), 1);
        assert_eq!(
            read_service_failures_at(&journal_path, 10)
                .unwrap()
                .records
                .len(),
            1
        );
        let _ = std::fs::remove_dir_all(journal_root);
    }

    #[tokio::test]
    async fn dashboard_service_status_owner_panic_is_observed_exactly_once() {
        let _guard = dashboard_status_cache_test_guard().await;
        let path = "/api/service/status?panic-fixture=true".to_string();
        let key = DashboardServiceStatusCacheKey {
            backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
            port: 9222,
            path: path.clone(),
        };
        let request_id = 77;
        let (result_tx, result_rx) = watch::channel(None);
        dashboard_service_status_cache()
            .lock()
            .await
            .entries
            .insert(
                key.clone(),
                DashboardServiceStatusCacheEntry::InFlight {
                    request_id,
                    registered_at: Instant::now(),
                    result: result_rx.clone(),
                    owner_abort: None,
                },
            );
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = observed.clone();
        let journal_root = std::env::temp_dir().join(format!(
            "agent-browser-dashboard-owner-panic-{}",
            uuid::Uuid::new_v4()
        ));
        let journal_path = journal_root.join("journal.jsonl");
        let observer_journal_path = journal_path.clone();
        let cleanup = DashboardStatusFlightCleanup::new(DashboardStatusFlightContext {
            key: key.clone(),
            request_id,
            port: 9222,
            method: "GET".to_string(),
            path,
            result: result_tx,
            observer: Arc::new(move |telemetry| {
                append_service_failure_at(
                    &observer_journal_path,
                    &dashboard_http_failure_record(&telemetry),
                )
                .unwrap();
                captured.lock().unwrap().push(telemetry);
            }),
            started_at: Instant::now(),
        });

        run_dashboard_status_flight_owner(cleanup, key.clone(), request_id, async {
            panic!("injected owner panic");
            #[allow(unreachable_code)]
            Ok(Vec::new())
        })
        .await;

        let error = result_rx.borrow().clone().unwrap().unwrap_err();
        assert_eq!(error.code, "backend_unavailable");
        {
            let events = observed.lock().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].stage, "owner_panic");
            assert_eq!(events[0].backend_error_class, Some("backend_unavailable"));
        }
        let readback = read_service_failures_at(&journal_path, 10).unwrap();
        assert_eq!(readback.records.len(), 1);
        assert_eq!(readback.records[0].stage, "owner_panic");
        assert!(!dashboard_service_status_cache()
            .lock()
            .await
            .entries
            .contains_key(&key));
        let _ = std::fs::remove_dir_all(journal_root);
    }

    #[tokio::test]
    async fn dashboard_service_status_shares_failures_without_caching_them() {
        let _guard = dashboard_status_cache_test_guard().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_count = request_count.clone();
        let body = r#"{"success":false,"error":"not ready"}"#;
        let response = format!(
            "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        let server_response = response.clone();
        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                server_count.fetch_add(1, Ordering::SeqCst);
                let mut request = vec![0_u8; 4096];
                let _ = stream.read(&mut request).await.unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
                stream.write_all(&server_response).await.unwrap();
            }
        });
        let path = format!("/api/service/status?shared-failure-port={port}");
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = observed.clone();
        let observer: DashboardHttpObserver =
            Arc::new(move |telemetry| captured.lock().unwrap().push(telemetry));
        let first = proxy_dashboard_service_api_request_observed(
            port,
            "GET",
            &path,
            "",
            Duration::from_secs(1),
            observer.clone(),
        );
        let second = proxy_dashboard_service_api_request_observed(
            port,
            "GET",
            &path,
            "",
            Duration::from_secs(1),
            observer.clone(),
        );
        let (first, second) = tokio::join!(first, second);
        assert_eq!(first.unwrap(), response);
        assert_eq!(second.unwrap(), response);
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
        assert_eq!(observed.lock().unwrap().len(), 1);

        let key = DashboardServiceStatusCacheKey {
            backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
            port,
            path: path.clone(),
        };
        for _ in 0..100 {
            if !dashboard_service_status_cache()
                .lock()
                .await
                .entries
                .contains_key(&key)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        let retry = proxy_dashboard_service_api_request_observed(
            port,
            "GET",
            &path,
            "",
            Duration::from_secs(1),
            observer,
        )
        .await
        .unwrap();

        assert_eq!(retry, response);
        assert_eq!(request_count.load(Ordering::SeqCst), 2);
        assert_eq!(observed.lock().unwrap().len(), 2);
    }

    #[test]
    fn dashboard_service_status_cache_evicts_only_oldest_ready_entry_at_capacity() {
        let mut cache = DashboardServiceStatusCache::default();
        let now = Instant::now();
        for index in 0..DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS {
            cache.entries.insert(
                DashboardServiceStatusCacheKey {
                    backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                    port: 10_000 + index as u16,
                    path: format!("/api/service/status?key={index}"),
                },
                DashboardServiceStatusCacheEntry::Ready {
                    completed_at: now - Duration::from_millis((index + 1) as u64),
                    response: vec![index as u8],
                },
            );
        }

        evict_oldest_ready_dashboard_status_entry(&mut cache);

        assert_eq!(
            cache.entries.len(),
            DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS - 1
        );
        assert!(
            !cache.entries.contains_key(&DashboardServiceStatusCacheKey {
                backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                port: 10_000 + (DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS - 1) as u16,
                path: format!(
                    "/api/service/status?key={}",
                    DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS - 1
                ),
            })
        );
    }

    #[test]
    fn dashboard_service_status_cache_keeps_all_32_inflight_entries_at_capacity() {
        let mut cache = DashboardServiceStatusCache::default();
        let mut senders = Vec::new();
        for index in 0..DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS {
            let (sender, receiver) = watch::channel(None);
            senders.push(sender);
            cache.entries.insert(
                DashboardServiceStatusCacheKey {
                    backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                    port: 20_000 + index as u16,
                    path: format!("/api/service/status?inflight={index}"),
                },
                DashboardServiceStatusCacheEntry::InFlight {
                    request_id: (index + 1) as u64,
                    registered_at: Instant::now(),
                    result: receiver,
                    owner_abort: None,
                },
            );
        }

        evict_oldest_ready_dashboard_status_entry(&mut cache);

        assert_eq!(cache.entries.len(), DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS);
        assert!(cache
            .entries
            .values()
            .all(|entry| matches!(entry, DashboardServiceStatusCacheEntry::InFlight { .. })));
        drop(senders);
    }

    #[tokio::test]
    async fn dashboard_service_status_uses_uncached_transport_for_33rd_inflight_key() {
        let _guard = dashboard_status_cache_test_guard().await;
        let mut senders = Vec::new();
        {
            let mut cache = dashboard_service_status_cache().lock().await;
            cache.entries.clear();
            for index in 0..DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS {
                let (sender, receiver) = watch::channel(None);
                senders.push(sender);
                cache.entries.insert(
                    DashboardServiceStatusCacheKey {
                        backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                        port: 31_000 + index as u16,
                        path: format!("/api/service/status?occupied={index}"),
                    },
                    DashboardServiceStatusCacheEntry::InFlight {
                        request_id: (index + 1) as u64,
                        registered_at: Instant::now(),
                        result: receiver,
                        owner_abort: None,
                    },
                );
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = r#"{"success":true,"data":{"overflow":"uncached"}}"#;
        let expected = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        let server_response = expected.clone();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await.unwrap();
            socket.write_all(&server_response).await.unwrap();
        });
        let path = format!("/api/service/status?overflow-port={port}");

        let response =
            proxy_dashboard_service_api_request(port, "GET", &path, "", Duration::from_secs(1))
                .await
                .unwrap();

        assert_eq!(response, expected);
        let mut cache = dashboard_service_status_cache().lock().await;
        assert_eq!(cache.entries.len(), DASHBOARD_SERVICE_STATUS_CACHE_MAX_KEYS);
        assert!(
            !cache.entries.contains_key(&DashboardServiceStatusCacheKey {
                backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
                port,
                path,
            })
        );
        cache.entries.clear();
        drop(senders);
    }

    #[test]
    fn dashboard_service_status_cache_prunes_ready_entries_only_after_ttl() {
        assert_eq!(DASHBOARD_SERVICE_STATUS_CACHE_TTL, Duration::from_secs(10));
        let mut cache = DashboardServiceStatusCache::default();
        let fresh_key = DashboardServiceStatusCacheKey {
            backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
            port: 30001,
            path: "/api/service/status?fresh=true".to_string(),
        };
        let expired_key = DashboardServiceStatusCacheKey {
            backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
            port: 30002,
            path: "/api/service/status?expired=true".to_string(),
        };
        cache.entries.insert(
            fresh_key.clone(),
            DashboardServiceStatusCacheEntry::Ready {
                completed_at: Instant::now(),
                response: b"fresh".to_vec(),
            },
        );
        cache.entries.insert(
            expired_key.clone(),
            DashboardServiceStatusCacheEntry::Ready {
                completed_at: Instant::now() - DASHBOARD_SERVICE_STATUS_CACHE_TTL,
                response: b"expired".to_vec(),
            },
        );

        prune_expired_dashboard_status_entries(&mut cache);

        assert!(cache.entries.contains_key(&fresh_key));
        assert!(!cache.entries.contains_key(&expired_key));
    }

    #[test]
    fn late_status_completion_cannot_replace_newer_request_id() {
        let mut cache = DashboardServiceStatusCache::default();
        let key = DashboardServiceStatusCacheKey {
            backend_session: DASHBOARD_SERVICE_BACKEND_SESSION,
            port: 30003,
            path: "/api/service/status?late=true".to_string(),
        };
        let (_sender, receiver) = watch::channel(None);
        cache.entries.insert(
            key.clone(),
            DashboardServiceStatusCacheEntry::InFlight {
                request_id: 2,
                registered_at: Instant::now(),
                result: receiver,
                owner_abort: None,
            },
        );
        let response = Ok(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec());

        apply_dashboard_status_flight_completion(
            &mut cache,
            key.clone(),
            1,
            &response,
            Instant::now(),
        );

        assert!(matches!(
            cache.entries.get(&key),
            Some(DashboardServiceStatusCacheEntry::InFlight { request_id: 2, .. })
        ));
    }

    #[test]
    fn dashboard_service_backend_falls_back_to_default_and_rejects_unknown_first() {
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
            None
        );
    }

    #[test]
    fn dashboard_service_backend_accepts_only_an_explicit_nonzero_port() {
        assert_eq!(
            configured_dashboard_service_backend_port(Some("4949")),
            Some(4949)
        );
        assert_eq!(
            configured_dashboard_service_backend_port(Some(" 4949 ")),
            Some(4949)
        );
        assert_eq!(configured_dashboard_service_backend_port(Some("0")), None);
        assert_eq!(
            configured_dashboard_service_backend_port(Some("not-a-port")),
            None
        );
        assert_eq!(configured_dashboard_service_backend_port(None), None);
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
    fn dashboard_durable_handoff_resolution_targets_retained_owner_session() {
        let state = crate::native::service_model::ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "handoff-a".to_string(),
                crate::native::service_model::RemoteViewHandoff {
                    id: "handoff-a".to_string(),
                    session_name: Some("im-receipts-google-messages-stock-v4".to_string()),
                    ..crate::native::service_model::RemoteViewHandoff::default()
                },
            )]),
            ..crate::native::service_model::ServiceState::default()
        };
        let body = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;

        assert_eq!(
            service_request_handoff_target_session_name_from_state(
                "/api/service/request",
                body,
                &state,
            ),
            Some("im-receipts-google-messages-stock-v4".to_string())
        );
    }

    #[test]
    fn dashboard_structured_handoff_without_current_owner_never_falls_back_to_stale_session() {
        let state = crate::native::service_model::ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "handoff-a".to_string(),
                crate::native::service_model::RemoteViewHandoff {
                    id: "handoff-a".to_string(),
                    state: "ready".to_string(),
                    session_name: Some("stale-owner-session".to_string()),
                    browser_id: Some("browser-a".to_string()),
                    target_id: Some("target-a".to_string()),
                    ..crate::native::service_model::RemoteViewHandoff::default()
                },
            )]),
            ..crate::native::service_model::ServiceState::default()
        };
        let body = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;

        assert_eq!(
            service_request_handoff_target_session_name_from_state(
                "/api/service/request",
                body,
                &state,
            ),
            None
        );
    }

    #[test]
    fn dashboard_durable_handoff_resolution_targets_transferred_ready_owner_session() {
        use sha2::{Digest, Sha256};

        let logical_browser_id = "session:im-receipts";
        let process_identity = crate::process_identity::RecordedProcessIdentity {
            pid: 4242,
            start_token: "linux:boot:4242".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        let process_digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&process_identity).unwrap())
        );
        let handoff = crate::native::service_model::RemoteViewHandoff {
            id: "handoff-a".to_string(),
            session_name: Some("retained-owner".to_string()),
            browser_id: Some(logical_browser_id.to_string()),
            target_id: Some("target-a".to_string()),
            view_stream_provider: Some(
                crate::native::service_model::ViewStreamProvider::RdpGateway,
            ),
            ..crate::native::service_model::RemoteViewHandoff::default()
        };
        let state = crate::native::service_model::ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(handoff.id.clone(), handoff)]),
            browsers: std::collections::BTreeMap::from([(
                logical_browser_id.to_string(),
                crate::native::service_model::BrowserProcess {
                    id: logical_browser_id.to_string(),
                    pid: Some(process_identity.pid),
                    health: crate::native::service_model::BrowserHealth::Ready,
                    active_session_ids: vec!["current-owner".to_string()],
                    tab_handles: vec![crate::native::service_model::ServiceTabHandle {
                        browser_id: logical_browser_id.to_string(),
                        session_name: Some("current-owner".to_string()),
                        target_id: Some("target-a".to_string()),
                        valid: true,
                        ..crate::native::service_model::ServiceTabHandle::default()
                    }],
                    ..crate::native::service_model::BrowserProcess::default()
                },
            )]),
            browser_process_identities: std::collections::BTreeMap::from([(
                logical_browser_id.to_string(),
                crate::native::service_model::ServiceBrowserProcessIdentity {
                    process_identity,
                    user_data_dir: None,
                    runtime_profile: Some("im-receipts-main".to_string()),
                },
            )]),
            runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-current".to_string(),
                    profile_identity_digest: "profile-digest".to_string(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                    owner_generation: 11,
                    browser_id: logical_browser_id.to_string(),
                    daemon_session_route: "current-owner".to_string(),
                    process_instance_digest: process_digest,
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                    target_set_digest: "target-set-digest".to_string(),
                    pending_transfer: None,
                    last_transition: None,
                },
            ),
            ..crate::native::service_model::ServiceState::default()
        };
        let body = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;

        assert_eq!(
            service_request_handoff_target_session_name_from_state(
                "/api/service/request",
                body,
                &state,
            ),
            Some("current-owner".to_string())
        );
    }

    #[test]
    fn dashboard_durable_handoff_proxy_stamps_authenticated_candidate_generation() {
        let state = crate::native::service_model::ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "handoff-a".to_string(),
                crate::native::service_model::RemoteViewHandoff {
                    id: "handoff-a".to_string(),
                    session_name: Some("retained-owner".to_string()),
                    ..crate::native::service_model::RemoteViewHandoff::default()
                },
            )]),
            ..crate::native::service_model::ServiceState::default()
        };
        let body = r##"{"action":"service_remote_view_handoff_resolve","serviceName":"agent-browser-dashboard","agentName":"codex","taskName":"durable-remote-view-handoff","params":{"handoffId":"handoff-a"}}"##;

        let (session_name, command_body) = service_request_handoff_proxy_command_body_from_state(
            "/api/service/request",
            body,
            "codex",
            &state,
            Some("generation-candidate"),
        )
        .unwrap()
        .unwrap();
        let command: Value = serde_json::from_str(&command_body).unwrap();

        assert_eq!(session_name, "retained-owner");
        assert_eq!(command["action"], "service_remote_view_handoff_resolve");
        assert_eq!(command["handoffId"], "handoff-a");
        assert_eq!(command["sessionName"], "retained-owner");
        assert_eq!(
            command["dashboardDeploymentGeneration"],
            "generation-candidate"
        );
        assert_eq!(command["requestPrincipalSource"], "explicit_labels");
    }

    #[test]
    fn dashboard_durable_handoff_proxy_rejects_public_generation_metadata() {
        let state = crate::native::service_model::ServiceState {
            remote_view_handoffs: std::collections::BTreeMap::from([(
                "handoff-a".to_string(),
                crate::native::service_model::RemoteViewHandoff {
                    id: "handoff-a".to_string(),
                    session_name: Some("retained-owner".to_string()),
                    ..crate::native::service_model::RemoteViewHandoff::default()
                },
            )]),
            ..crate::native::service_model::ServiceState::default()
        };
        let body = r##"{"action":"service_remote_view_handoff_resolve","serviceName":"agent-browser-dashboard","agentName":"codex","taskName":"durable-remote-view-handoff","dashboardDeploymentGeneration":"forged","params":{"handoffId":"handoff-a"}}"##;

        let result = service_request_handoff_proxy_command_body_from_state(
            "/api/service/request",
            body,
            "codex",
            &state,
            Some("generation-candidate"),
        )
        .unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn dashboard_service_request_allows_durable_handoff_resolution_to_finish() {
        let body = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;

        assert_eq!(
            service_api_proxy_timeout("POST", "/api/service/request", body),
            Duration::from_secs(60)
        );
        assert_eq!(
            service_api_proxy_timeout("POST", "/api/service/request", r##"{"action":"navigate"}"##),
            DASHBOARD_LOCAL_PROXY_TIMEOUT
        );
        assert_eq!(
            service_api_proxy_timeout("GET", "/api/service/status", ""),
            DASHBOARD_SERVICE_STATUS_PROXY_TIMEOUT
        );
        assert_eq!(
            service_api_proxy_timeout("GET", "/api/service/resources", ""),
            DASHBOARD_SERVICE_STATUS_PROXY_TIMEOUT
        );
        assert_eq!(
            service_api_proxy_timeout(
                "POST",
                "/api/service/request",
                r##"{"action":"view_focus"}"##
            ),
            DASHBOARD_REMOTE_VIEW_REQUEST_PROXY_TIMEOUT
        );
        assert!(dashboard_service_status_cacheable(
            "GET",
            "/api/service/resources"
        ));
    }

    #[test]
    fn authenticated_ready_handoff_response_selects_candidate_commit_identity() {
        let request = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;
        let ready = json_http_response(
            "200 OK",
            json!({
                "success": true,
                "data": {"status": "ready", "resolved": true},
            }),
        );

        assert_eq!(
            authenticated_candidate_handoff_commit_id(request, &ready),
            Some("handoff-a".to_string())
        );
    }

    #[test]
    fn public_dashboard_handoff_response_omits_provider_route_urls() {
        let request = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;
        let response = json_http_response(
            "200 OK",
            json!({
                "success": true,
                "data": {
                    "status": "ready",
                    "resolved": true,
                    "handoffUrl": "https://dashboard.example.test/remote-view/handoff-a",
                    "providerExternalUrl": "http://127.0.0.1:8080/guacamole/",
                    "localEmbedUrl": "http://127.0.0.1:8080/guacamole/",
                    "dashboardEmbedUrl": "https://dashboard.example.test/guacamole/",
                    "healthUrl": "http://127.0.0.1:8080/health",
                    "routeBinding": {
                        "externalUrl": "http://127.0.0.1:8080/guacamole/",
                        "frameUrl": "http://127.0.0.1:8080/guacamole/"
                    },
                    "open": {
                        "intent": {"url": "http://127.0.0.1:19058/fixture"},
                        "providerExternalUrl": "http://127.0.0.1:8080/guacamole/",
                        "routeBinding": {"healthUrl": "http://127.0.0.1:8080/health"}
                    }
                }
            }),
        );

        let sanitized = sanitize_dashboard_handoff_response(request, response).unwrap();
        let payload: Value =
            serde_json::from_slice(http_response_body(&sanitized).unwrap()).unwrap();
        let data = payload.get("data").unwrap();
        assert_eq!(
            data.get("handoffUrl").and_then(Value::as_str),
            Some("https://dashboard.example.test/remote-view/handoff-a")
        );
        assert_eq!(
            data.pointer("/open/intent/url").and_then(Value::as_str),
            Some("http://127.0.0.1:19058/fixture")
        );
        for pointer in [
            "/providerExternalUrl",
            "/localEmbedUrl",
            "/dashboardEmbedUrl",
            "/healthUrl",
            "/routeBinding",
            "/open/providerExternalUrl",
            "/open/routeBinding",
        ] {
            assert!(
                data.pointer(pointer).is_none(),
                "retained forbidden field {pointer}"
            );
        }
    }

    #[test]
    fn incomplete_handoff_response_cannot_select_candidate() {
        let request = r##"{"action":"service_remote_view_handoff_resolve","params":{"handoffId":"handoff-a"}}"##;
        for payload in [
            json!({"success": true, "data": {"status": "converging", "resolved": false}}),
            json!({"success": true, "data": {"status": "ready", "resolved": false}}),
            json!({"success": false, "error": "blocked"}),
        ] {
            let response = json_http_response("200 OK", payload);
            assert_eq!(
                authenticated_candidate_handoff_commit_id(request, &response),
                None
            );
        }

        let unrelated = r##"{"action":"service_status","params":{"handoffId":"handoff-a"}}"##;
        let ready = json_http_response(
            "200 OK",
            json!({"success": true, "data": {"status": "ready", "resolved": true}}),
        );
        assert_eq!(
            authenticated_candidate_handoff_commit_id(unrelated, &ready),
            None
        );
    }

    #[test]
    fn dashboard_service_request_allows_remote_view_recovery_to_finish() {
        for action in [
            "remote_view_open",
            "service_remote_view_route_preflight",
            "service_remote_view_browser_reattach",
            "service_remote_view_route_switch",
            "service_remote_view_route_checkout",
            "service_remote_view_route_release",
            "service_route_pool_repair",
            "service_viewer_lease_request",
            "service_viewer_lease_heartbeat",
            "service_viewer_lease_release",
            "service_controller_lease_takeover",
            "view_takeover",
        ] {
            let body = serde_json::json!({ "action": action }).to_string();
            assert_eq!(
                service_api_proxy_timeout("POST", "/api/service/request", &body),
                Duration::from_secs(15),
                "unexpected proxy timeout for {action}"
            );
        }
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
    fn dashboard_service_request_focus_command_body_preserves_job_identity() {
        let body = r##"{"id":"focus-1","action":"view_focus","serviceName":"agent-browser-dashboard","agentName":"operator","taskName":"workspace-viewport-control","jobTimeoutMs":5000,"params":{"sessionName":"odollo-carrier-ups","targetId":"target-1","index":2,"maximize":true}}"##;
        let (session_name, command_body) =
            service_request_focus_command_body("/api/service/request", body).unwrap();
        let command: Value = serde_json::from_str(&command_body).unwrap();

        assert_eq!(session_name, "odollo-carrier-ups");
        assert_eq!(command["id"], "focus-1");
        assert_eq!(command["action"], "view_focus");
        assert_eq!(command["targetId"], "target-1");
        assert_eq!(command["index"], 2);
        assert_eq!(command["maximize"], true);
        assert_eq!(command["serviceName"], "agent-browser-dashboard");
        assert_eq!(command["agentName"], "operator");
        assert_eq!(command["taskName"], "workspace-viewport-control");
        assert_eq!(command["jobTimeoutMs"], 5000);
        assert_eq!(
            command[crate::runtime_host::SERVICE_REQUEST_EXPLICIT_PROFILE_ROUTING_FIELD],
            false
        );
        assert!(command.get("sessionName").is_none());
    }

    #[test]
    fn dashboard_display_content_detects_terminal_only_route() {
        let content = display_content_from_xwininfo(
            ":12",
            r#"
        0x60011f "Openbox": ("" (none))  1x1+-100+-100  +-100+-100
        0x40000e "agent-browser-rdp-a@cooper: ~": ("xterm" "XTerm")  604x368+1+22  +41+62
"#,
        );

        assert_eq!(content["state"], "terminal_only");
        assert_eq!(content["displayName"], ":12");
        assert_eq!(content["windowCount"], 2);
        assert_eq!(content["windows"][1]["className"], "XTerm");
    }

    #[test]
    fn dashboard_display_content_detects_browser_window() {
        let content = display_content_from_xwininfo(
            ":12",
            r#"
        0x60011f "Openbox": ("" (none))  1x1+-100+-100  +-100+-100
        0x800003 "Example Domain - Chromium": ("chromium-browser (/tmp/profile)" "Chromium-browser")  504x320+0+0  +0+0
        0x40000e "agent-browser-rdp-a@cooper: ~": ("xterm" "XTerm")  604x368+1+22  +41+62
"#,
        );

        assert_eq!(content["state"], "browser_window_visible");
        assert_eq!(content["windowCount"], 3);
        assert_eq!(content["windows"][1]["title"], "Example Domain - Chromium");
    }

    #[test]
    fn dashboard_display_content_probe_skips_stale_service_displays() {
        let guard = EnvGuard::new(&[
            "AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME",
            "AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME",
            "AGENT_BROWSER_REMOTE_HEADED_DISPLAY",
        ]);
        guard.set("AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME", ":12");
        guard.set("AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME", ":11");

        assert!(should_probe_route_display(":12"));
        assert!(should_probe_route_display(":11"));
        assert!(!should_probe_route_display(":99.0"));
        assert!(!should_probe_route_display(":106"));
        assert!(!should_probe_route_display("localhost:12"));
        assert!(!should_probe_route_display(""));
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

    #[test]
    fn cdp_json_list_pages_become_dashboard_tabs() {
        let body = json!([
            {
                "id": "target-page-1",
                "type": "page",
                "title": "Foreign page",
                "url": "https://example.test/foreign",
                "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/target-page-1"
            },
            {
                "id": "worker-1",
                "type": "service_worker",
                "title": "Worker",
                "url": "https://example.test/worker.js"
            },
            {
                "id": "target-page-2",
                "type": "page",
                "title": "Second page",
                "url": "about:blank"
            }
        ])
        .to_string();

        let tabs = cdp_json_list_to_dashboard_tabs(body.as_bytes()).unwrap();

        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0]["index"], 0);
        assert_eq!(tabs[0]["active"], true);
        assert_eq!(tabs[0]["targetId"], "target-page-1");
        assert_eq!(tabs[0]["title"], "Foreign page");
        assert_eq!(tabs[0]["url"], "https://example.test/foreign");
        assert_eq!(tabs[1]["index"], 2);
        assert_eq!(tabs[1]["active"], false);
        assert_eq!(tabs[1]["targetId"], "target-page-2");
    }

    #[test]
    fn cdp_json_list_screenshot_target_prefers_requested_page_with_websocket() {
        let body = json!([
            {
                "id": "target-page-1",
                "type": "page",
                "title": "First page",
                "url": "https://example.test/first",
                "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/target-page-1"
            },
            {
                "id": "target-page-2",
                "type": "page",
                "title": "Second page",
                "url": "https://example.test/second",
                "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/target-page-2"
            }
        ])
        .to_string();

        let target =
            cdp_json_list_screenshot_target(body.as_bytes(), Some("target-page-2")).unwrap();

        assert_eq!(target.id, "target-page-2");
        assert_eq!(target.title, "Second page");
        assert_eq!(target.url, "https://example.test/second");
        assert_eq!(
            target.web_socket_debugger_url,
            "ws://127.0.0.1:9222/devtools/page/target-page-2"
        );
    }

    #[test]
    fn cdp_json_list_screenshot_target_falls_back_to_first_page_websocket() {
        let body = json!([
            {
                "id": "worker-1",
                "type": "service_worker",
                "title": "Worker"
            },
            {
                "id": "target-page-1",
                "type": "page",
                "title": "First page",
                "url": "https://example.test/first",
                "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/target-page-1"
            }
        ])
        .to_string();

        let target = cdp_json_list_screenshot_target(body.as_bytes(), None).unwrap();

        assert_eq!(target.id, "target-page-1");
    }

    #[test]
    fn cdp_json_list_screenshot_target_rejects_a_missing_requested_target() {
        let body = json!([{
            "id": "target-page-1",
            "type": "page",
            "title": "First page",
            "url": "https://example.test/first",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/target-page-1"
        }])
        .to_string();

        assert!(cdp_json_list_screenshot_target(body.as_bytes(), Some("missing-target")).is_err());
    }

    #[test]
    fn json_http_response_sets_json_headers_and_body() {
        let response = json_http_response("200 OK", json!([{"index": 0, "title": "ok"}]));
        let text = String::from_utf8(response).unwrap();

        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: application/json; charset=utf-8"));
        assert_eq!(
            serde_json::from_str::<Value>(
                std::str::from_utf8(http_response_body(text.as_bytes()).unwrap()).unwrap()
            )
            .unwrap(),
            json!([{"index": 0, "title": "ok"}])
        );
    }

    #[test]
    fn http_response_content_length_is_case_insensitive() {
        let response = b"HTTP/1.1 200 OK\r\ncontent-length: 17\r\nConnection: keep-alive\r\n\r\n{\"ok\":true,\"n\":1}";

        assert_eq!(http_response_status(response), Some(200));
        assert_eq!(http_response_content_length(response), Some(17));
        assert_eq!(
            http_response_body(response).unwrap(),
            b"{\"ok\":true,\"n\":1}"
        );
    }

    #[tokio::test]
    async fn dashboard_gateway_proxy_normalizes_empty_backend_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = socket.read(&mut buffer).await;
        });

        let err = proxy_local_http_api_request_with_timeout_observed(
            port,
            "GET",
            "/api/empty",
            "",
            Duration::from_secs(1),
            |_| {},
        )
        .await
        .unwrap_err();

        assert_eq!(err.code, "backend_empty_response");
        assert_eq!(
            err.message,
            format!("empty response from 127.0.0.1:{port}/api/empty")
        );
        assert_eq!(
            err.details.unwrap()["readinessState"],
            json!("invalid_payload")
        );
    }

    #[tokio::test]
    async fn dashboard_gateway_proxy_normalizes_invalid_http_response() {
        use std::io::{Read as _, Write as _};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let backend = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = socket.read(&mut buffer);
            let _ = socket.write_all(b"not an http response");
        });

        let err = proxy_local_http_api_request_with_timeout_observed(
            port,
            "GET",
            "/api/invalid",
            "",
            Duration::from_millis(100),
            |_| {},
        )
        .await
        .unwrap_err();
        backend.join().unwrap();

        assert_eq!(err.code, "backend_invalid_http");
        assert_eq!(
            err.message,
            format!("invalid HTTP response from 127.0.0.1:{port}/api/invalid")
        );
    }

    #[tokio::test]
    async fn dashboard_gateway_proxy_normalizes_local_read_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = socket.read(&mut buffer).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        });

        let err = proxy_local_http_api_request_with_timeout_observed(
            port,
            "GET",
            "/api/slow",
            "",
            Duration::from_millis(20),
            |_| {},
        )
        .await
        .unwrap_err();

        assert_eq!(err.code, "backend_read_timeout");
        assert_eq!(
            err.message,
            format!("timed out reading from 127.0.0.1:{port}/api/slow")
        );
        assert_eq!(err.details.unwrap()["readinessState"], json!("unreachable"));
    }

    #[test]
    fn dashboard_gateway_timeout_codes_keep_compatibility_details() {
        let connect = DashboardReadinessError::local_backend(
            "backend_connect_timeout",
            "connect timeout",
            9222,
            "/api/service/status",
            "connect",
        );
        let write = DashboardReadinessError::local_backend(
            "backend_write_timeout",
            "write timeout",
            9222,
            "/api/service/status",
            "write",
        );

        assert_eq!(connect.code, "backend_connect_timeout");
        assert_eq!(
            connect.details.unwrap()["readinessState"],
            json!("unreachable")
        );
        assert_eq!(write.code, "backend_write_timeout");
        assert_eq!(write.details.unwrap()["stage"], json!("write"));
    }

    #[tokio::test]
    async fn dashboard_gateway_independently_bounds_connect_write_and_read_phases() {
        for (code, stage) in [
            ("backend_connect_timeout", "connect"),
            ("backend_write_timeout", "write"),
            ("backend_read_timeout", "read"),
        ] {
            let error = run_dashboard_backend_io_phase(
                DashboardBackendIoPhase {
                    timeout_code: code,
                    timeout_message: format!("{stage} timed out"),
                    failure_message: format!("{stage} failed"),
                    port: 9222,
                    path: "/api/service/status",
                    stage,
                    timeout: Duration::from_millis(1),
                },
                std::future::pending::<std::io::Result<()>>(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code, code);
            assert_eq!(error.details.unwrap()["stage"], stage);
        }
    }

    #[test]
    fn dashboard_gateway_rejects_invalid_json_backend_payload() {
        let body = b"not-json";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        )
        .into_bytes();

        let err = require_json_backend_response_observed(
            response,
            9222,
            "GET",
            "/api/service/status",
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code, "invalid_backend_payload");
        assert_eq!(
            err.details.unwrap()["readinessState"],
            json!("invalid_payload")
        );
    }

    #[tokio::test]
    async fn dashboard_gateway_observes_injected_502_and_504_once_each() {
        for (status, expected_status) in [
            ("502 Bad Gateway", 502_u16),
            ("504 Gateway Timeout", 504_u16),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: 17\r\n\r\n{{\"success\":false}}"
            )
            .into_bytes();
            let backend_response = response.clone();
            tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 1024];
                let _ = socket.read(&mut request).await.unwrap();
                socket.write_all(&backend_response).await.unwrap();
            });
            let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
            let captured = observed.clone();

            let actual = proxy_local_http_api_request_with_timeout_observed(
                port,
                "GET",
                "/api/service/status?token=must-not-appear",
                "",
                Duration::from_secs(1),
                move |telemetry| captured.lock().unwrap().push(telemetry),
            )
            .await
            .unwrap();

            assert_eq!(actual, response);
            let events = observed.lock().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].status, Some(expected_status));
            assert_eq!(events[0].status_class, "5xx");
            assert_eq!(events[0].route_class, "service_status");
            assert_eq!(events[0].body_class, "json");
            assert_eq!(events[0].response_bytes, Some(actual.len()));
        }
    }

    #[test]
    fn dashboard_gateway_non_json_failure_is_redacted_and_has_journal_parity() {
        let body = b"not-json-secret";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        )
        .into_bytes();
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = observed.clone();

        let error = require_json_backend_response_observed(
            response,
            9222,
            "GET",
            "/api/service/jobs?token=raw-secret&url=https://private.test",
            move |telemetry| captured.lock().unwrap().push(telemetry),
        )
        .unwrap_err();

        assert_eq!(error.code, "invalid_backend_payload");
        let events = observed.lock().unwrap();
        assert_eq!(events.len(), 1);
        let telemetry = &events[0];
        assert_eq!(telemetry.route_class, "service_api");
        assert_eq!(telemetry.method, "GET");
        assert_eq!(telemetry.status, Some(200));
        assert_eq!(telemetry.body_class, "non_json");
        assert_eq!(
            telemetry.backend_error_class,
            Some("invalid_backend_payload")
        );
        let encoded = serde_json::to_string(telemetry).unwrap();
        assert!(!encoded.contains("raw-secret"));
        assert!(!encoded.contains("private.test"));
        assert!(!encoded.contains("not-json-secret"));

        let record = dashboard_http_failure_record(telemetry);
        assert_eq!(
            record.details,
            Some(serde_json::to_value(telemetry).unwrap())
        );
        assert_eq!(record.code, "invalid_backend_payload");
        assert_eq!(record.stage, "response");
    }

    #[test]
    fn dashboard_gateway_does_not_duplicate_http_5xx_during_json_validation() {
        let response = b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: 8\r\n\r\nnot-json".to_vec();
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let terminal = dashboard_http_terminal_telemetry(
            "POST",
            "/api/command?secret=value",
            Duration::from_millis(5),
            1,
            &Ok(response.clone()),
        )
        .unwrap();
        observed.lock().unwrap().push(terminal);
        let captured = observed.clone();

        let error = require_json_backend_response_observed(
            response,
            9222,
            "POST",
            "/api/command?secret=value",
            move |telemetry| captured.lock().unwrap().push(telemetry),
        )
        .unwrap_err();

        assert_eq!(error.code, "invalid_backend_payload");
        let events = observed.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, Some(502));
        assert_eq!(events[0].backend_error_class, Some("backend_http_5xx"));
    }

    #[test]
    fn dashboard_gateway_omits_fast_success_but_records_slow_success() {
        let response = Ok(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec());
        assert!(dashboard_http_terminal_telemetry(
            "GET",
            "/api/service/status",
            Duration::from_millis(10),
            1,
            &response,
        )
        .is_none());
        let slow = dashboard_http_terminal_telemetry(
            "GET",
            "/api/service/status",
            DASHBOARD_SLOW_PROXY_THRESHOLD,
            3,
            &response,
        )
        .unwrap();
        assert_eq!(slow.event, "dashboard_http_slow");
        assert_eq!(slow.inflight_count, 3);
        assert!(!slow.failed());
    }

    #[test]
    fn dashboard_gateway_json_error_response_includes_code_and_details() {
        let response = json_error_http_response(
            "502 Bad Gateway",
            "Service API proxy failed: invalid JSON response",
            Some("invalid_backend_payload"),
            Some(json!({
                "transport": "local_proxy",
                "stage": "response",
            })),
        );
        let body = http_response_body(&response).unwrap();
        let value: Value = serde_json::from_slice(body).unwrap();

        assert_eq!(http_response_status(&response), Some(502));
        assert_eq!(value["success"], false);
        assert_eq!(value["code"], "invalid_backend_payload");
        assert_eq!(value["details"]["stage"], "response");
    }

    #[test]
    fn durable_handoff_resolution_uses_daemon_relay_without_an_http_lane() {
        assert!(service_request_requires_daemon_relay(&json!({
            "action": "service_remote_view_handoff_resolve",
            "sessionName": "retained-owner-without-http-route",
        })));
        assert!(!service_request_requires_daemon_relay(&json!({
            "action": "tab_new",
            "sessionName": "ordinary-http-routed-lane",
        })));
    }

    #[test]
    fn screenshot_errors_map_to_stable_readiness_codes() {
        let invalid =
            normalize_screenshot_error("CDP screenshot response did not include image data".into());
        assert_eq!(invalid.code, "invalid_backend_payload");
        assert_eq!(
            invalid.details.unwrap()["readinessState"],
            json!("invalid_payload")
        );

        let stale = normalize_screenshot_error(
            "Requested CDP target does not expose a page WebSocket".into(),
        );
        assert_eq!(stale.code, "stale_target");

        let unreachable =
            normalize_screenshot_error("Timed out waiting for CDP screenshot response".into());
        assert_eq!(unreachable.code, "backend_read_timeout");
    }

    #[test]
    fn foreign_cdp_input_accepts_only_bounded_pointer_keyboard_and_wheel_events() {
        assert_eq!(
            foreign_cdp_input_command(&json!({
                "kind": "mouse",
                "eventType": "mousePressed",
                "x": 120.5,
                "y": 80,
                "button": "left",
                "clickCount": 1
            }))
            .unwrap(),
            (
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mousePressed",
                    "x": 120.5,
                    "y": 80.0,
                    "button": "left",
                    "clickCount": 1
                })
            )
        );
        assert_eq!(
            foreign_cdp_input_command(&json!({
                "kind": "wheel",
                "x": 10,
                "y": 20,
                "deltaX": 0,
                "deltaY": 150
            }))
            .unwrap()
            .0,
            "Input.dispatchMouseEvent"
        );
        assert_eq!(
            foreign_cdp_input_command(&json!({
                "kind": "keyboard",
                "eventType": "keyDown",
                "key": "a",
                "code": "KeyA",
                "text": "a"
            }))
            .unwrap()
            .0,
            "Input.dispatchKeyEvent"
        );
    }

    #[test]
    fn foreign_cdp_input_rejects_arbitrary_cdp_and_out_of_bounds_coordinates() {
        assert!(foreign_cdp_input_command(&json!({
            "kind": "cdp",
            "method": "Browser.close"
        }))
        .is_err());
        assert!(foreign_cdp_input_command(&json!({
            "kind": "mouse",
            "eventType": "mousePressed",
            "x": -1,
            "y": 20
        }))
        .is_err());
        assert!(foreign_cdp_input_command(&json!({
            "kind": "keyboard",
            "eventType": "rawKeyDown",
            "key": "a",
            "code": "KeyA"
        }))
        .is_err());
    }
}
