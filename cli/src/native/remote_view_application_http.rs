//! Revision 3 loopback HTTP transport. No provider installation or discovery.
use agent_browser_service_model::{
    RemoteViewApplicationEnvelope, RemoteViewApplicationTransport,
    RemoteViewApplicationTransportError as Error,
};
use serde_json::Value;
use std::time::Duration;

const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;

/// Explicit local origin only. No proxy, redirects, URL credentials or replay.
/// Optional control authentication reads the private file named by
/// AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE; the bearer is never public metadata.
/// Synchronous callers may already own a Tokio runtime; each request performs
/// its async I/O on a joined worker, avoiding nested-runtime panics.
pub(crate) struct RemoteViewApplicationHttp {
    endpoint: reqwest::Url,
    timeout: Duration,
    authorization: Option<reqwest::header::HeaderValue>,
}
impl RemoteViewApplicationHttp {
    pub(crate) fn new(origin: &str, timeout: Duration) -> Result<Self, Error> {
        let mut endpoint = reqwest::Url::parse(origin).map_err(|_| Error::Rejected)?;
        let loopback = endpoint.host_str().is_some_and(|host| {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        if !loopback
            || endpoint.scheme() != "http"
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
            || endpoint.port_or_known_default() == Some(0)
            || timeout.is_zero()
        {
            return Err(Error::Rejected);
        }
        endpoint.set_path("/v1/consumer");
        Ok(Self {
            endpoint,
            timeout,
            authorization: control_authorization()?,
        })
    }
}
impl RemoteViewApplicationHttp {
    /// Existing Remote View agent-desktop API. No browser/CDP attachment or
    /// controller takeover is performed by this transport.
    pub(crate) fn native_request(&mut self, operation: &str, body: &Value) -> Result<Value, Error> {
        if !matches!(
            operation,
            "observe"
                | "control"
                | "capture"
                | "capture_region"
                | "ocr"
                | "input"
                | "window"
                | "input_cancel"
        ) {
            return Err(Error::Rejected);
        }
        if operation == "control" && body["operation"] != "status" {
            return Err(Error::Rejected);
        }
        let mut endpoint = self.endpoint.clone();
        endpoint.set_path(&format!("/v1/agent-desktop/{operation}"));
        let authorization = self.authorization.clone();
        let timeout = self.timeout;
        let body = serde_json::to_vec(body).map_err(|_| Error::Rejected)?;
        if body.len() > 1_048_576 {
            return Err(Error::Rejected);
        }
        std::thread::Builder::new()
            .name("remote-view-native-http".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| Error::Unavailable)?;
                runtime.block_on(async move {
                    let client = reqwest::Client::builder()
                        .no_proxy()
                        .redirect(reqwest::redirect::Policy::none())
                        .http1_only()
                        .pool_max_idle_per_host(0)
                        .timeout(timeout)
                        .connect_timeout(timeout)
                        .build()
                        .map_err(|_| Error::Unavailable)?;
                    let mut request = client
                        .post(endpoint)
                        .header(reqwest::header::CONTENT_TYPE, "application/json")
                        .header(reqwest::header::CONNECTION, "close")
                        .body(body);
                    if let Some(authorization) = authorization {
                        request = request.header(reqwest::header::AUTHORIZATION, authorization);
                    }
                    let mut response = request.send().await.map_err(|_| Error::OutcomeUnknown)?;
                    if matches!(
                        response.status(),
                        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
                    ) {
                        return Err(Error::Rejected);
                    }
                    if response.status() != reqwest::StatusCode::OK {
                        return Err(Error::OutcomeUnknown);
                    }
                    // Remote View bounds raw captures at 4 MiB; allow base64 and metadata.
                    const LIMIT: usize = 6 * 1024 * 1024;
                    if response
                        .content_length()
                        .is_some_and(|size| size > LIMIT as u64)
                    {
                        return Err(Error::OutcomeUnknown);
                    }
                    let mut bytes = Vec::new();
                    while let Some(chunk) =
                        response.chunk().await.map_err(|_| Error::OutcomeUnknown)?
                    {
                        if bytes.len().saturating_add(chunk.len()) > LIMIT {
                            return Err(Error::OutcomeUnknown);
                        }
                        bytes.extend_from_slice(&chunk);
                    }
                    serde_json::from_slice(&bytes).map_err(|_| Error::OutcomeUnknown)
                })
            })
            .map_err(|_| Error::Unavailable)?
            .join()
            .map_err(|_| Error::OutcomeUnknown)?
    }
}
impl RemoteViewApplicationTransport for RemoteViewApplicationHttp {
    fn request(&mut self, envelope: &RemoteViewApplicationEnvelope) -> Result<Value, Error> {
        let body = serde_json::to_vec(envelope).map_err(|_| Error::Rejected)?;
        let envelope_operation = serde_json::to_value(&envelope.request)
            .map_err(|_| Error::Rejected)?["operation"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let endpoint = self.endpoint.clone();
        let timeout = self.timeout;
        let authorization = self.authorization.clone();
        std::thread::Builder::new()
            .name("remote-view-http".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| Error::Unavailable)?;
                runtime.block_on(async move {
                    let client = reqwest::Client::builder()
                        .no_proxy()
                        .redirect(reqwest::redirect::Policy::none())
                        .http1_only()
                        .pool_max_idle_per_host(0)
                        .timeout(timeout)
                        .connect_timeout(timeout)
                        .build()
                        .map_err(|_| Error::Unavailable)?;
                    let mut request = client
                        .post(endpoint)
                        .header(reqwest::header::CONTENT_TYPE, "application/json")
                        .header(reqwest::header::CONNECTION, "close")
                        .body(body);
                    if let Some(authorization) = authorization {
                        request = request.header(reqwest::header::AUTHORIZATION, authorization);
                    }
                    let mut response = request.send().await.map_err(|_| Error::OutcomeUnknown)?;
                    let status = response.status();
                    if response
                        .content_length()
                        .is_some_and(|length| length > RESPONSE_LIMIT as u64)
                        || response
                            .headers()
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .is_none_or(|value| value.split(';').next() != Some("application/json"))
                    {
                        return Err(Error::OutcomeUnknown);
                    }
                    let mut bytes = Vec::new();
                    while let Some(chunk) =
                        response.chunk().await.map_err(|_| Error::OutcomeUnknown)?
                    {
                        if chunk.len() > RESPONSE_LIMIT.saturating_sub(bytes.len()) {
                            return Err(Error::OutcomeUnknown);
                        }
                        bytes.extend_from_slice(&chunk);
                    }
                    let value: Value =
                        serde_json::from_slice(&bytes).map_err(|_| Error::OutcomeUnknown)?;
                    if status == reqwest::StatusCode::BAD_REQUEST
                        && envelope_operation == "prepare_idle_retirement"
                        && value.get("schema_version").and_then(Value::as_u64) == Some(1)
                        && value.get("retryable").and_then(Value::as_bool) == Some(true)
                        && matches!(
                            (
                                value.get("code").and_then(Value::as_str),
                                value.get("message").and_then(Value::as_str)
                            ),
                            (
                                Some("idle_retirement_viewer_active"),
                                Some("native retirement exclusion unavailable")
                            ) | (
                                Some("idle_retirement_activity_changed"),
                                Some("viewing retirement target changed")
                            )
                        )
                    {
                        return Err(Error::IdleRetirementDeferred);
                    }
                    if status == reqwest::StatusCode::BAD_REQUEST
                        && envelope_operation == "observe_native_handoff_use"
                        && value.get("schema_version").and_then(Value::as_u64) == Some(1)
                        && value.get("code").and_then(Value::as_str) == Some("native_use_missing")
                        && value.get("message").and_then(Value::as_str)
                            == Some("native handoff observation unavailable")
                        && value.get("retryable").and_then(Value::as_bool) == Some(false)
                    {
                        return Err(Error::NativeObservationMissing);
                    }
                    if status == reqwest::StatusCode::BAD_REQUEST
                        && matches!(envelope_operation.as_str(), "issue_view")
                        && value.get("schema_version").and_then(Value::as_u64) == Some(1)
                        && value.get("code").and_then(Value::as_str)
                            == Some("consumer_grant_unavailable")
                        && value.get("message").and_then(Value::as_str) == Some("grant is terminal")
                        && value.get("retryable").and_then(Value::as_bool) == Some(false)
                    {
                        return Err(Error::ViewGrantTerminal);
                    }
                    if status != reqwest::StatusCode::OK {
                        return Err(Error::OutcomeUnknown);
                    }
                    Ok(value)
                })
            })
            .map_err(|_| Error::Unavailable)?
            .join()
            .map_err(|_| Error::OutcomeUnknown)?
    }
}

/// Read once at transport construction; invalid explicit credentials reject
/// before network effects. A token grants control API admission, not viewer use.
fn control_authorization() -> Result<Option<reqwest::header::HeaderValue>, Error> {
    let Some(path) = std::env::var_os("AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE") else {
        return Ok(None);
    };
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() {
        return Err(Error::Rejected);
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| Error::Rejected)?;
    let metadata = file.metadata().map_err(|_| Error::Rejected)?;
    if !metadata.is_file() || metadata.len() > 257 {
        return Err(Error::Rejected);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            return Err(Error::Rejected);
        }
    }
    use std::io::Read;
    let mut raw = String::new();
    file.take(258)
        .read_to_string(&mut raw)
        .map_err(|_| Error::Rejected)?;
    let token = raw.strip_suffix('\n').unwrap_or(&raw);
    if !(16..=256).contains(&token.len()) || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Rejected);
    }
    let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| Error::Rejected)?;
    value.set_sensitive(true);
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{RemoteViewApplicationAdapter, RemoteViewApplicationRequest};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
    };

    #[test]
    fn configured_native_control_token_authenticates_direct_inventory() {
        use crate::test_utils::EnvGuard;
        let path =
            std::env::temp_dir().join(format!("native-control-token-{}", uuid::Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path).unwrap();
        file.write_all(b"test-native-control-token-1234567890\n")
            .unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE"]);
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
            path.to_str().unwrap(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let request = read_request(&mut socket);
            let authorized = request.lines().any(|line| {
                line.eq_ignore_ascii_case(
                    "authorization: Bearer test-native-control-token-1234567890",
                )
            });
            let (status, body) = if authorized {
                ("200 OK", "{\"admitted\":true}")
            } else {
                ("401 Unauthorized", "{\"error\":\"control_token_required\"}")
            };
            write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        let mut client = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5)).unwrap();
        let result = client.request(&envelope());
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(
            result.is_ok(),
            "configured private token must reach the native control service without a connector"
        );
    }

    #[test]
    fn native_input_uses_existing_owner_endpoint_with_exact_target_and_no_retry() {
        let guard =
            crate::test_utils::EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE"]);
        guard.remove("AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let end = std::time::Instant::now() + Duration::from_secs(2);
            let mut socket = loop {
                if let Ok((socket, _)) = listener.accept() {
                    break socket;
                }
                if std::time::Instant::now() >= end {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(2));
            };
            let request = read_request(&mut socket);
            let body = "{\"schemaVersion\":1,\"dispatched\":true}";
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            Some(request)
        });
        let mut client = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(2)).unwrap();
        let target = serde_json::json!({"desktopId":"synthetic","expectedGeneration":7,"windowId":42,"controlEpoch":3,"actions":[{"kind":"key","chord":"Tab"}]});
        let result = client.native_request("input", &target);
        let request = server.join().unwrap();
        assert_eq!(result.unwrap()["dispatched"], true);
        let request = request.expect("native request reached its owner");
        assert!(request.starts_with("POST /v1/agent-desktop/input HTTP/1.1"));
        let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body, target);
    }

    #[cfg(unix)]
    #[test]
    fn configured_native_control_token_rejects_unsafe_files_before_network() {
        use crate::test_utils::EnvGuard;
        use std::os::unix::fs::{symlink, PermissionsExt};
        let path =
            std::env::temp_dir().join(format!("native-control-rejection-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, "test-private-native-control-token").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE"]);
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
            path.to_str().unwrap(),
        );
        assert!(
            RemoteViewApplicationHttp::new("http://127.0.0.1:1", Duration::from_secs(1)).is_err()
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = path.with_extension("link");
        symlink(&path, &link).unwrap();
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
            link.to_str().unwrap(),
        );
        assert!(
            RemoteViewApplicationHttp::new("http://127.0.0.1:1", Duration::from_secs(1)).is_err()
        );
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
            path.to_str().unwrap(),
        );
        std::fs::write(&path, "token-with-newline\nin-the-middle-of-secret").unwrap();
        assert!(
            RemoteViewApplicationHttp::new("http://127.0.0.1:1", Duration::from_secs(1)).is_err()
        );
        std::fs::remove_file(link).unwrap();
        std::fs::remove_file(path).unwrap();
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
            "relative-control.token",
        );
        assert!(
            RemoteViewApplicationHttp::new("http://127.0.0.1:1", Duration::from_secs(1)).is_err()
        );
    }

    #[test]
    fn idle_retirement_denials_are_scoped_to_prepare_and_exact_pre_effect_errors() {
        for (operation, code, message, retryable, expected) in [
            (
                "prepare_idle_retirement",
                "idle_retirement_viewer_active",
                "native retirement exclusion unavailable",
                true,
                Error::IdleRetirementDeferred,
            ),
            (
                "prepare_idle_retirement",
                "idle_retirement_activity_changed",
                "viewing retirement target changed",
                true,
                Error::IdleRetirementDeferred,
            ),
            (
                "inventory",
                "idle_retirement_viewer_active",
                "native retirement exclusion unavailable",
                true,
                Error::OutcomeUnknown,
            ),
            (
                "prepare_idle_retirement",
                "idle_retirement_viewer_active",
                "native retirement exclusion unavailable",
                false,
                Error::OutcomeUnknown,
            ),
            (
                "prepare_idle_retirement",
                "idle_retirement_viewer_active",
                "different failure",
                true,
                Error::OutcomeUnknown,
            ),
            (
                "prepare_idle_retirement",
                "idle_retirement_native_unavailable",
                "native retirement exclusion unavailable",
                true,
                Error::OutcomeUnknown,
            ),
        ] {
            let body = serde_json::json!({"schema_version":1,"code":code,"message":message,"retryable":retryable}).to_string();
            let (origin, handle) = server(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body));
            let request = if operation == "inventory" {
                serde_json::json!({"operation":operation})
            } else {
                serde_json::json!({"operation":operation,"assignment_id":"assignment-a","expected_generation":7,
                    "expected_viewing_generation":19,"idempotency_key":"close-one","expected_revision":0,
                    "expected_retirement":{"schemaVersion":1,"desktopId":"22222222-2222-4222-8222-222222222222",
                        "generation":7,"routes":[],"sessions":[]}})
            };
            let envelope = serde_json::from_value(
                serde_json::json!({"application":"agent-browser","request":request}),
            )
            .unwrap();
            let mut transport =
                RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5)).unwrap();
            assert_eq!(transport.request(&envelope), Err(expected));
            handle.join().unwrap();
        }
    }

    #[test]
    fn terminal_grant_response_is_scoped_to_issue_view_and_exact_error() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        for (index, code, expected) in [
            (7, "consumer_grant_unavailable", Error::ViewGrantTerminal),
            (7, "provider_unavailable", Error::OutcomeUnknown),
            (2, "consumer_grant_unavailable", Error::OutcomeUnknown),
        ] {
            let body = serde_json::json!({"schema_version":1,"code":code,"message":"grant is terminal","retryable":false}).to_string();
            let (origin, handle) = server(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body));
            let mut transport =
                RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5)).unwrap();
            let envelope = serde_json::from_value(fixture["requests"][index].clone()).unwrap();
            assert_eq!(transport.request(&envelope), Err(expected));
            handle.join().unwrap();
        }
    }

    #[test]
    fn native_missing_response_is_scoped_to_exact_private_read() {
        for (operation, code, schema, retryable, expected) in [
            (
                "observe_native_handoff_use",
                "native_use_missing",
                1,
                false,
                Error::NativeObservationMissing,
            ),
            (
                "inventory",
                "native_use_missing",
                1,
                false,
                Error::OutcomeUnknown,
            ),
            (
                "observe_native_handoff_use",
                "provider_unavailable",
                1,
                false,
                Error::OutcomeUnknown,
            ),
            (
                "observe_native_handoff_use",
                "native_use_missing",
                2,
                false,
                Error::OutcomeUnknown,
            ),
            (
                "observe_native_handoff_use",
                "native_use_missing",
                1,
                true,
                Error::OutcomeUnknown,
            ),
        ] {
            let body = serde_json::json!({"schema_version":schema,"code":code,"message":"native handoff observation unavailable","retryable":retryable}).to_string();
            let (origin, handle) = server(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body));
            let request = if operation == "inventory" {
                serde_json::json!({"operation":operation})
            } else {
                serde_json::json!({"operation":operation,"correlation_id":"a".repeat(64),"target_digest":"synthetic-target"})
            };
            let envelope = serde_json::from_value(
                serde_json::json!({"application":"agent-browser","request":request}),
            )
            .unwrap();
            let mut transport =
                RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5)).unwrap();
            assert_eq!(transport.request(&envelope), Err(expected));
            handle.join().unwrap();
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let header_end = loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
            assert!(bytes.len() < 8192);
            if bytes.ends_with(b"\r\n\r\n") {
                break bytes.len();
            }
        };
        let headers = String::from_utf8(bytes.clone()).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|value| value.trim().parse().unwrap())
            })
            .unwrap();
        bytes.resize(header_end + length, 0);
        stream.read_exact(&mut bytes[header_end..]).unwrap();
        String::from_utf8(bytes).unwrap()
    }
    fn envelope() -> RemoteViewApplicationEnvelope {
        RemoteViewApplicationEnvelope {
            application: "agent-browser".into(),
            request: RemoteViewApplicationRequest::Inventory {},
        }
    }
    fn server(response: String) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            stream.write_all(response.as_bytes()).unwrap();
            request
        });
        (origin, handle)
    }
    #[tokio::test]
    async fn http_consumer_uses_exact_wire_inside_existing_runtime() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let body = fixture["assignmentObservation"].to_string();
        let (origin, handle) = server(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body));
        let transport = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5)).unwrap();
        let mut adapter =
            RemoteViewApplicationAdapter::new("agent-browser".into(), transport).unwrap();
        let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
        let observation = adapter.observe_assignment(&assignment).unwrap();
        assert_eq!(
            serde_json::to_value(observation).unwrap(),
            fixture["assignmentObservation"]
        );
        let request = handle.join().unwrap();
        assert!(request.starts_with("POST /v1/consumer HTTP/1.1\r\n"));
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        let payload: Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(payload["application"], "agent-browser");
        assert_eq!(payload["request"]["operation"], "observe_assignment");
    }
    #[test]
    fn http_consumer_lost_reply_is_unknown_without_a_second_post() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            drop(stream);
            listener.set_nonblocking(true).unwrap();
            (listener, request)
        });
        let mut transport =
            RemoteViewApplicationHttp::new(&origin, Duration::from_secs(2)).unwrap();
        assert_eq!(transport.request(&envelope()), Err(Error::OutcomeUnknown));
        let (listener, request) = handle.join().unwrap();
        assert!(request.starts_with("POST /v1/consumer "));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert!(
            RemoteViewApplicationHttp::new("http://[::1]:9000", Duration::from_secs(1)).is_ok()
        );
    }
    #[test]
    fn http_consumer_rejects_redirects_and_invalid_replies_without_echo() {
        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        for response in [
            format!("HTTP/1.1 302 Found\r\nLocation: http://{}/redirect\r\nContent-Length: 0\r\n\r\n", target.local_addr().unwrap()),
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 14\r\n\r\nprivate-secret".into(),
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 14\r\n\r\nprivate-secret".into(),
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", RESPONSE_LIMIT + 1),
        ] {
            let (origin, handle) = server(response);
            let mut transport = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(5)).unwrap();
            assert_eq!(transport.request(&envelope()).unwrap_err(), Error::OutcomeUnknown);
            assert!(handle.join().unwrap().starts_with("POST /v1/consumer "));
        }
        assert_eq!(
            target.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        for origin in [
            "http://example.test",
            "https://127.0.0.1:9000",
            "http://user:secret@127.0.0.1:9000",
            "http://127.0.0.1:9000/path",
            "http://127.0.0.1:9000/?token=secret",
        ] {
            assert!(RemoteViewApplicationHttp::new(origin, Duration::from_secs(1)).is_err());
        }
    }
}
