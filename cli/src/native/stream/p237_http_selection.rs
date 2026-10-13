/// Task-local wire adapter around production dashboard and stream HTTP handlers.
/// Its Unix endpoint invokes the actual native focus handler; it does not
/// emulate provider acquisition or claim runtime-host startup qualification.
#[cfg(target_os = "linux")]
struct P237SelectionHttp {
    address: SocketAddr,
    cookie: String,
    dispatchers: Vec<(crate::native::daemon::P237NativeDispatch, String)>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    advance_controller_before_execution: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(target_os = "linux")]
impl Drop for P237SelectionHttp {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

#[cfg(target_os = "linux")]
impl P237SelectionHttp {
    async fn exchange(
        address: SocketAddr,
        path: &str,
        body: &str,
        cookie: &str,
    ) -> Result<String, String> {
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .map_err(|e| e.to_string())?;
        let request = format!("POST {path} HTTP/1.1\r\nHost: synthetic.example\r\nContent-Type: application/json\r\nCookie: {cookie}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
        client
            .write_all(request.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), client.read_to_end(&mut response))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        String::from_utf8(response).map_err(|e| e.to_string())
    }

    async fn start(daemons: Arc<tokio::sync::Mutex<Vec<DaemonState>>>, credentials: &Path) -> Self {
        let mut tasks = Vec::new();
        let mut dispatchers = Vec::new();
        let advance_controller_before_execution =
            Arc::new(std::sync::atomic::AtomicBool::new(false));
        std::env::set_var("AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY", "disabled");
        for index in 0..2 {
            let session = format!("session-{index}");
            let socket = get_socket_path(&session);
            std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
            let unix = tokio::net::UnixListener::bind(&socket).unwrap();
            // These listeners run in this exact fixture process, whose PID is
            // required by production discovery independently of socket readiness.
            std::fs::write(
                get_socket_dir().join(format!("{session}.pid")),
                std::process::id().to_string(),
            )
            .unwrap();
            let token = crate::connection::generate_daemon_auth_token().unwrap();
            crate::connection::write_daemon_auth_token(&session, &token).unwrap();
            let dispatcher = if std::env::var_os("P237_NATIVE_SELECTION_DISPATCH").is_some() {
                let state = std::mem::replace(
                    &mut daemons.lock().await[index],
                    DaemonState::new_for_session_with_stream(&session, None, None),
                );
                let dispatcher = crate::native::daemon::P237NativeDispatch::new(state).await;
                dispatchers.push((dispatcher.clone(), token.clone()));
                Some(dispatcher)
            } else {
                None
            };
            let shared = daemons.clone();
            let advance_controller = advance_controller_before_execution.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    let (stream, _) = unix.accept().await.unwrap();
                    let (reader, mut writer) = tokio::io::split(stream);
                    let mut line = String::new();
                    if tokio::io::AsyncBufReadExt::read_line(
                        &mut tokio::io::BufReader::new(reader),
                        &mut line,
                    )
                    .await
                    .unwrap()
                        == 0
                    {
                        continue;
                    }
                    let command: Value = serde_json::from_str(&line).unwrap();
                    assert_eq!(command["_agentBrowserAuthToken"], token);
                    // Deterministic race at the wire boundary after production
                    // HTTP issued the proof, immediately before native execution.
                    if advance_controller.swap(false, std::sync::atomic::Ordering::SeqCst) {
                        use crate::native::service_store::{
                            LockedServiceStateRepository, ServiceStateRepository,
                        };
                        LockedServiceStateRepository::default_json()
                            .unwrap()
                            .mutate(|state| {
                                state
                                    .remote_view_routes
                                    .get_mut(command["routeId"].as_str().unwrap())
                                    .unwrap()
                                    .controller_epoch += 1;
                                Ok(())
                            })
                            .unwrap();
                    }
                    let response = if let Some(dispatcher) = dispatcher.as_ref() {
                        dispatcher.dispatch(&line, &token).await
                    } else {
                        match crate::native::browser_lifecycle::handle_view_focus(
                            &command,
                            &mut shared.lock().await[index],
                        )
                        .await
                        {
                            Ok(data) => json!({"success":true,"data":data}),
                            Err(error) => json!({"success":false,"error":error}),
                        }
                    };
                    writer
                        .write_all(format!("{response}\n").as_bytes())
                        .await
                        .unwrap();
                }
            }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            std::fs::write(
                get_socket_dir().join(format!("{session}.stream")),
                port.to_string(),
            )
            .unwrap();
            tasks.push(tokio::spawn(async move {
                let tabs = Arc::new(RwLock::new(Vec::new()));
                let engine = Arc::new(RwLock::new("chrome".into()));
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    let mut peeked = vec![0u8; 8192];
                    let count = stream.peek(&mut peeked).await.unwrap();
                    if count == 0 {
                        continue;
                    }
                    handle_http_request(stream, &peeked[..count], &tabs, &engine, &session).await;
                }
            }));
        }
        let dashboard = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = dashboard.local_addr().unwrap();
        tasks.push(tokio::spawn(
            crate::native::stream::run_native_handoff_http_fixture(dashboard),
        ));
        let text = std::fs::read_to_string(credentials).unwrap();
        let password = text
            .lines()
            .find_map(|line| {
                line.strip_prefix("AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD=\"")
                    .and_then(|value| value.strip_suffix('"'))
            })
            .unwrap();
        let login = Self::exchange(
            address,
            "/api/dashboard-auth/login",
            &json!({"username":"admin","password":password}).to_string(),
            "",
        )
        .await
        .unwrap();
        assert!(
            login.starts_with("HTTP/1.1 200"),
            "synthetic login failed: {login}"
        );
        let cookie = login
            .lines()
            .find_map(|line| {
                line.strip_prefix("Set-Cookie: ")
                    .and_then(|value| value.split(';').next())
            })
            .unwrap()
            .to_owned();
        Self {
            address,
            cookie,
            dispatchers,
            tasks,
            advance_controller_before_execution,
        }
    }

    async fn close_dispatchers(&self) {
        for (dispatcher, token) in &self.dispatchers {
            dispatcher.close(token).await;
        }
    }

    async fn focus(&self, body: &str) -> Result<Value, String> {
        let response =
            Self::exchange(self.address, "/api/service/request", body, &self.cookie).await?;
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or("HTTP response headers missing")?;
        if !headers.starts_with("HTTP/1.1 200") {
            return Err(format!("physical HTTP focus failed: {response}"));
        }
        let value: Value =
            serde_json::from_str(body).map_err(|e| format!("HTTP focus JSON: {e}; {response}"))?;
        if value["success"] != true {
            return Err(format!("physical HTTP focus rejected: {value}"));
        }
        Ok(value["data"].clone())
    }
}
