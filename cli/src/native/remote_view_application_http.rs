//! Revision 3 loopback HTTP transport. No provider installation or discovery.
use agent_browser_service_model::{
    RemoteViewApplicationEnvelope, RemoteViewApplicationTransport,
    RemoteViewApplicationTransportError as Error,
};
use serde_json::Value;
use std::time::Duration;

const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;

/// Explicit local origin only. No proxy, redirects, credentials or replay.
/// Synchronous callers may already own a Tokio runtime; each request performs
/// its async I/O on a joined worker, avoiding nested-runtime panics.
pub(crate) struct RemoteViewApplicationHttp {
    endpoint: reqwest::Url,
    timeout: Duration,
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
        Ok(Self { endpoint, timeout })
    }
}
impl RemoteViewApplicationTransport for RemoteViewApplicationHttp {
    fn request(&mut self, envelope: &RemoteViewApplicationEnvelope) -> Result<Value, Error> {
        let body = serde_json::to_vec(envelope).map_err(|_| Error::Rejected)?;
        let endpoint = self.endpoint.clone();
        let timeout = self.timeout;
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
                    let mut response = client
                        .post(endpoint)
                        .header(reqwest::header::CONTENT_TYPE, "application/json")
                        .header(reqwest::header::CONNECTION, "close")
                        .body(body)
                        .send()
                        .await
                        .map_err(|_| Error::OutcomeUnknown)?;
                    if response.status() != reqwest::StatusCode::OK
                        || response
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
                    serde_json::from_slice(&bytes).map_err(|_| Error::OutcomeUnknown)
                })
            })
            .map_err(|_| Error::Unavailable)?
            .join()
            .map_err(|_| Error::OutcomeUnknown)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{RemoteViewApplicationAdapter, RemoteViewApplicationRequest};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
    };

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
