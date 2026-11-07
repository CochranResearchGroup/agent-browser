//! Real private HTTP admission, no browser or provider is started.
use super::*;
use crate::test_utils::EnvGuard;

#[tokio::test]
async fn native_mode_callback_requires_gateway_capability_and_rejects_browser_selected_scope() {
    let directory = std::env::temp_dir().join(format!("native-mode-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let env = EnvGuard::new(&["AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE"]);
    let token = directory.join("token");
    std::fs::write(&token, b"synthetic-native-mode-capability-123456789").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    env.set(
        "AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE",
        token.to_str().unwrap(),
    );
    let valid = r#"{"slot":1,"generation":1,"choice":"inspect"}"#;
    for (headers, body, status) in [
        ("", valid, 403),
        ("Authorization: Bearer wrong-capability\r\nX-Remote-View-Operator: operator\r\n", valid, 403),
        ("Authorization: Bearer synthetic-native-mode-capability-123456789\r\nX-Remote-View-Operator: operator\r\nCookie: forged=1\r\n", valid, 403),
        ("Authorization: Bearer synthetic-native-mode-capability-123456789\r\nX-Remote-View-Operator: operator\r\n", r#"{"slot":1,"generation":1,"choice":"prepare","sessionId":"foreign","operationId":"op","mode":"cdp"}"#, 400),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { let (stream, _) = listener.accept().await.unwrap(); handle_dashboard_connection(stream).await; });
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        client.write_all(format!("POST /api/native-remote-view/retained-task/mode HTTP/1.1\r\nHost: localhost\r\n{headers}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        let mut reply = Vec::new(); timeout(Duration::from_secs(3), client.read_to_end(&mut reply)).await.unwrap().unwrap(); task.await.unwrap();
        assert_eq!(http_response_status(&reply), Some(status), "{headers}");
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn native_mode_callback_relays_only_the_retained_task_and_never_acquires_another_owner() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use tokio::io::AsyncBufReadExt;
    struct Tasks(Vec<tokio::task::JoinHandle<()>>);
    impl Drop for Tasks {
        fn drop(&mut self) {
            for task in &self.0 {
                task.abort();
            }
        }
    }
    let root = std::env::temp_dir().join(format!(
        "native-mode-scope-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let env = EnvGuard::new(&[
        "AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE",
        "AGENT_BROWSER_SOCKET_DIR",
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        "AGENT_BROWSER_REMOTE_VIEW_POOL",
        "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
        "AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT",
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
    ]);
    let token = root.join("token");
    std::fs::write(&token, b"synthetic-native-mode-capability-123456789").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    env.set(
        "AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE",
        token.to_str().unwrap(),
    );
    env.set("AGENT_BROWSER_SOCKET_DIR", root.join("s").to_str().unwrap());
    std::fs::create_dir_all(crate::connection::get_socket_dir()).unwrap();
    let owner = "retained-owner";
    crate::connection::write_daemon_auth_token(owner, "synthetic-daemon-token").unwrap();
    let listener =
        tokio::net::UnixListener::bind(crate::connection::get_socket_path(owner)).unwrap();
    let commands = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let observed = commands.clone();
    let mut tasks = Tasks(vec![tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let mut reader = tokio::io::BufReader::new(socket);
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            let command: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(command["sessionId"], "retained-session");
            assert_eq!(command["sessionName"], "retained-owner");
            assert_eq!(command["browserId"], "retained-browser");
            assert_eq!(command["action"], "browser_session_mode_convert");
            assert_eq!(command["_agentBrowserAuthToken"], "synthetic-daemon-token");
            let data = if command["choice"] == "inspect" {
                json!({"schemaVersion":"agent-browser.browser-operations.v1","operations":[]})
            } else {
                json!({"schemaVersion":"agent-browser.mode-operation.v1","operationId":command["operationId"],"phase":"prepared","requestedMode":"cdp_free","affectedTaskIds":["retained-session"],"actions":[{"action":"relaunch","requiresConfirmation":true}]})
            };
            observed.lock().unwrap().push(command);
            reader
                .get_mut()
                .write_all(format!("{}\n", json!({"success":true,"data":data})).as_bytes())
                .await
                .unwrap();
        }
    })]);
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let mut inventory = fixture["inventory"].clone();
    inventory["desktops"][0]["viewing"] = json!({"lifecycleGeneration":7,"desktopId":"33333333-3333-4333-8333-333333333333","generation":4,"publicRoute":"/1"});
    let provider = TcpListener::bind("127.0.0.1:0").await.unwrap();
    env.set(
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        &format!("http://{}", provider.local_addr().unwrap()),
    );
    env.set("AGENT_BROWSER_REMOTE_VIEW_POOL", "main");
    for name in [
        "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
        "AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT",
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
    ] {
        env.remove(name);
    }
    tasks.0.push(tokio::spawn(async move {
        loop {
            let (mut stream, _) = provider.accept().await.unwrap();
            let mut buf = vec![0; 8192];
            let n = stream.read(&mut buf).await.unwrap();
            let body = read_post_body(&mut stream, &buf, n).await;
            let request: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(request["request"]["operation"], "inventory");
            write_json_value(&mut stream, "200 OK", inventory.clone()).await;
        }
    }));
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let original = store.load_session_state().unwrap();
    struct Restore(agent_browser_service_model::BrowserSessionState);
    impl Drop for Restore {
        fn drop(&mut self) {
            BrowserSessionSqliteStore::default_sqlite()
                .unwrap()
                .save_session_state(&self.0)
                .unwrap();
        }
    }
    let _restore = Restore(original.clone());
    let mut state = original;
    state.sessions.insert("retained-session".into(), serde_json::from_value(json!({"id":"retained-session","name":owner,"profileId":"retained-profile","browserId":"retained-browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":u64::MAX,"currentTabId":"retained-tab"})).unwrap());
    state.tabs.insert("retained-tab".into(), serde_json::from_value(json!({"id":"retained-tab","targetId":"retained-target","browserId":"retained-browser","sessionId":"retained-session","createdAtMs":1,"lastActivityAtMs":1})).unwrap());
    state.browsers.insert("retained-browser".into(), serde_json::from_value(json!({"id":"retained-browser","profileId":"retained-profile","pid":42,"cdpEndpoint":"","activeSessionIds":["retained-session"],"desktop":{"desktop_id":"22222222-2222-4222-8222-222222222222","friendly_route_label":"Synthetic","generation":7}})).unwrap());
    state.remote_view_tab_handoffs.insert("retained-task".into(), serde_json::from_value(json!({"id":"retained-task","sessionId":"retained-session","tabId":"retained-tab","createdAtMs":1})).unwrap());
    store.save_session_state(&state).unwrap();
    async fn exchange(body: Value) -> (u16, Value) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_dashboard_connection(stream).await;
        });
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        let body = body.to_string();
        client.write_all(format!("POST /api/native-remote-view/retained-task/mode HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer synthetic-native-mode-capability-123456789\r\nX-Remote-View-Operator: operator\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        let mut reply = Vec::new();
        timeout(Duration::from_secs(5), client.read_to_end(&mut reply))
            .await
            .unwrap()
            .unwrap();
        task.await.unwrap();
        (
            http_response_status(&reply).unwrap(),
            serde_json::from_slice(http_response_body(&reply).unwrap()).unwrap(),
        )
    }
    let (status, inspection) = exchange(json!({"slot":1,"generation":1,"choice":"inspect"})).await;
    assert_eq!(status, 200);
    assert_eq!(
        inspection["data"]["schemaVersion"],
        "agent-browser.handoff-mode.v1"
    );
    assert_eq!(
        inspection["data"]["mode"], "unverified",
        "missing process evidence is never CDP-off evidence"
    );
    assert!(
        inspection["data"]["nativeControlTarget"].is_null(),
        "missing original process evidence cannot authorize Native human input"
    );
    #[cfg(target_os = "linux")]
    {
        // Synthetic root executable with a real kernel process identity. This
        // qualifies the HTTP owner seam without starting a browser or provider.
        let executable = root.join("chrome");
        std::fs::copy("/bin/sleep", &executable).unwrap();
        struct OwnedChild(std::process::Child);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let child = OwnedChild(
            std::process::Command::new(&executable)
                .arg("60")
                .spawn()
                .unwrap(),
        );
        let crate::process_identity::ProcessObservation::Observed(process) =
            crate::process_identity::observe_process(child.0.id())
        else {
            panic!("owned synthetic process unavailable")
        };
        assert_eq!(process.browser_family.as_deref(), Some("chrome"));
        let original = agent_browser_service_model::RecordedProcessIdentity {
            pid: child.0.id(),
            start_token: process.start_token.unwrap(),
            executable_path: process.executable_path,
            browser_family: process.browser_family,
        };
        state.browsers.get_mut("retained-browser").unwrap().pid = original.pid;
        state.shared_browser_ids.insert("retained-browser".into());
        state.adopted_browser_process_identities.insert(
            "retained-browser".into(),
            agent_browser_service_model::BrowserAdoptedProcessIdentity {
                profile_id: "retained-profile".into(),
                cdp_endpoint: String::new(),
                process: original,
            },
        );
        let resolved =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&state, "retained-task")
                .unwrap();
        let digest = agent_browser_service_model::remote_view_native_target_digest(
            "retained-task",
            &resolved,
        )
        .unwrap();
        let target = json!({"assignmentId":"assignment-external-1","registrationId":"registration-agent-browser",
            "desktopId":"22222222-2222-4222-8222-222222222222","lifecycleGeneration":7,
            "viewingDesktopId":"33333333-3333-4333-8333-333333333333","viewingGeneration":4});
        state.remote_view_tab_handoffs.get_mut("retained-task").unwrap().native_observation = Some(
            serde_json::from_value(json!({"targetDigest":digest,"providerTarget":target,"idempotencyKey":"synthetic",
                "observation":{"correlationId":"a".repeat(64),"requestDigest":"b".repeat(64),"application":"agent-browser",
                    "target":target,"targetDigest":digest,"clock":{"bootId":"fixture-boot","elapsedMs":1},"lifetimeMs":300000,"used":null}})).unwrap());
        store.save_session_state(&state).unwrap();
        let (status, inspection) =
            exchange(json!({"slot":1,"generation":4,"choice":"inspect"})).await;
        assert_eq!(status, 200);
        assert_eq!(
            inspection["data"]["nativeControlTarget"]["schemaVersion"],
            "agent-browser.native-control-target.v1",
            "{inspection}"
        );
        assert_eq!(
            inspection["data"]["nativeControlTarget"]["targetDigest"],
            digest
        );
        assert_eq!(inspection["data"]["nativeControlTarget"]["target"], target);
        assert_eq!(
            inspection["data"]["nativeControlTarget"]["handoffIntent"],
            "view"
        );
        let (status, changed_generation) =
            exchange(json!({"slot":1,"generation":5,"choice":"inspect"})).await;
        assert_eq!(status, 200);
        assert!(changed_generation["data"]["nativeControlTarget"].is_null());
    }
    let (status, prepared) = exchange(json!({"slot":1,"generation":1,"choice":"prepare","mode":"cdp_free","operationId":"original:operation/v1","ownerConfirmed":false})).await;
    assert_eq!(status, 200);
    assert_eq!(prepared["data"]["operationId"], "original:operation/v1");
    let before = commands.lock().unwrap().len();
    let (status, _) = exchange(json!({"slot":2,"generation":1,"choice":"prepare","mode":"cdp_free","operationId":"wrong-desktop"})).await;
    assert_eq!(status, 503);
    state
        .remote_view_tab_handoffs
        .get_mut("retained-task")
        .unwrap()
        .expires_at_ms = Some(1);
    store.save_session_state(&state).unwrap();
    let (status, _) = exchange(json!({"slot":1,"generation":1,"choice":"inspect"})).await;
    assert_eq!(status, 410);
    assert_eq!(
        commands.lock().unwrap().len(),
        before,
        "stale desktop or expired handoff must not reach another owner"
    );
    drop(tasks);
    drop(_restore);
    std::fs::remove_dir_all(root).unwrap();
}
