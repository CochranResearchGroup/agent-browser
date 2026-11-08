//! Synthetic Authelia owner for authenticated HTTP fixtures. All requests still
//! pass production owner verification; local accounts grant no workspace access.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
};

pub(crate) struct TestIngress {
    stop: Arc<AtomicBool>,
    address: std::net::SocketAddr,
    thread: Option<std::thread::JoinHandle<()>>,
    environment: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl TestIngress {
    pub(crate) fn start() -> Self {
        super::ensure_dashboard_auth_config().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let environment = [
            "AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL",
            "AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL",
            "AGENT_BROWSER_DASHBOARD_INGRESS_USERS",
        ]
        .map(|name| (name, std::env::var_os(name)))
        .to_vec();
        std::env::set_var(
            environment[0].0,
            format!("http://{address}/api/authz/forward-auth"),
        );
        std::env::set_var(environment[1].0, "https://fixture.example/");
        std::env::set_var(
            environment[2].0,
            r#"{"admin":"admin","codex":"codex","operator":"operator"}"#,
        );
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = std::thread::spawn(move || {
            while let Ok((mut stream, _)) = listener.accept() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = [0; 8192];
                let n = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                let principal = request.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if !name.eq_ignore_ascii_case("cookie") {
                        return None;
                    }
                    value.split(';').find_map(|cookie| {
                        let value = cookie.trim().strip_prefix("cooper_authelia_session=")?;
                        matches!(value, "admin" | "codex" | "operator").then_some(value)
                    })
                });
                let status = if principal.is_some() {
                    "200 OK"
                } else {
                    "401 Unauthorized"
                };
                let principal = principal
                    .map(|name| format!("Remote-User: {name}\r\n"))
                    .unwrap_or_default();
                write!(stream,"HTTP/1.1 {status}\r\n{principal}Content-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            }
        });
        Self {
            stop,
            address,
            thread: Some(thread),
            environment,
        }
    }

    pub(crate) fn cookie(&self, username: &str) -> String {
        format!("cooper_authelia_session={username}")
    }
}

impl Drop for TestIngress {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
        for (name, value) in &self.environment {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}
