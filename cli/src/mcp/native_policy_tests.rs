//! Public MCP, CLI and HTTP requests cross real adapters into an isolated owner-policy endpoint.
//! The endpoint substitutes only the process/provider boundary; policy is the production owner.
use super::*;
use crate::native::{browser_host_policy, browser_session_store::BrowserSessionSqliteStore};
use agent_browser_service_model::*;
#[cfg(not(target_os = "linux"))]
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

struct Endpoint {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}

struct NativeHttp {
    runtime: tokio::runtime::Runtime,
    server: crate::native::stream::StreamServer,
}

impl NativeHttp {
    fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (server, _) = runtime
            .block_on(crate::native::stream::StreamServer::start_without_client(
                0,
                "lane-task".into(),
                false,
            ))
            .unwrap();
        Self { runtime, server }
    }

    fn command(&self, task: &str, action: &str, request: &str, grant: Option<&str>) -> Value {
        let mut command = json!({"id":request,"action":action,"sessionId":task,"sessionName":format!("lane-{task}")});
        if action == "native_input" {
            command["actions"] = json!([{"kind":"move","x":1,"y":1}]);
        }
        if let Some(grant) = grant {
            command["policyOverrideId"] = json!(grant);
        }
        self.runtime.block_on(async {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let body = command.to_string();
            let mut client = tokio::net::TcpStream::connect(("127.0.0.1", self.server.port()))
                .await
                .unwrap();
            client.write_all(format!("POST /api/command HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            let mut response = String::new();
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.read_to_string(&mut response),
            )
            .await
            .unwrap()
            .unwrap();
            let (headers, body) = response.split_once("\r\n\r\n").unwrap();
            assert!(headers.starts_with("HTTP/1.1 200 OK"), "{headers}");
            let value: Value = serde_json::from_str(body).unwrap();
            assert_eq!(value["id"], request, "HTTP must retain the scoped request ID");
            assert!(!body.contains("synthetic-transport-token"));
            if let Some(grant) = grant {
                assert!(!body.contains(grant), "HTTP must not echo scoped grants");
            }
            value
        })
    }
}

impl Drop for NativeHttp {
    fn drop(&mut self) {
        self.runtime.block_on(self.server.shutdown());
    }
}

#[test]
fn native_policy_override_public_mcp_preserves_exact_scope_and_refusal() {
    let root = std::env::temp_dir().join(format!("ab-native-policy-{}", uuid::Uuid::new_v4()));
    let sockets = root.join("s");
    std::fs::create_dir_all(&sockets).unwrap();
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
        "AGENT_BROWSER_RUNTIME_HOST",
    ]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_SOCKET_DIR", sockets.to_str().unwrap());
    guard.set("AGENT_BROWSER_RUNTIME_HOST", "1");
    let mut state = BrowserSessionState::default();
    state.browsers.insert(
        "browser".into(),
        ManagedBrowserInstance {
            id: "browser".into(),
            profile_id: "profile".into(),
            pid: std::process::id(),
            cdp_endpoint: String::new(),
            desktop: None,
            active_session_ids: vec!["task".into(), "peer".into(), "unrestricted".into()],
        },
    );
    for id in ["task", "peer", "unrestricted"] {
        state.sessions.insert(
            id.into(),
            ManagedBrowserSession {
                id: id.into(),
                name: format!("lane-{id}"),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 0,
                current_tab_id: None,
            },
        );
        state.host_bindings.insert(
            id.into(),
            BrowserHostTaskBinding {
                request: BrowserHostBinding {
                    version: 1,
                    host_id: "app".into(),
                    participant_id: id.into(),
                    session_id: format!("app-{id}"),
                    placement: BrowserPlacement::Required {
                        desktop: BrowserDesktopBinding {
                            desktop_id: "desktop".into(),
                            generation: 1,
                        },
                    },
                },
                desktop: RemoteViewFixedDesktop {
                    desktop_id: "desktop".into(),
                    generation: 1,
                    friendly_route_label: "Fixture".into(),
                },
            },
        );
    }
    for task in ["task", "peer"] {
        state
            .host_policies
            .update(
                state.host_policies.revision,
                "app",
                task,
                "set_task",
                Some(BrowserTaskPolicy {
                    denied_actions: BTreeSet::from([
                        "native_input".into(),
                        "native_capture".into(),
                    ]),
                    ..Default::default()
                }),
                "fixture-operator",
                "Deny synthetic Native actions",
            )
            .unwrap();
    }
    for action in ["native_input", "native_capture"] {
        state
            .host_policies
            .issue_override(
                state.host_policies.revision,
                format!("grant-{action}"),
                "app",
                "task",
                action,
                &format!("request-{action}"),
                "fixture-operator",
                "One exact synthetic Native request",
            )
            .unwrap();
    }
    let path = BrowserSessionSqliteStore::default_sqlite_path().unwrap();
    assert!(path.starts_with(&root));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    store.save_session_state(&state).unwrap();
    let listener = UnixListener::bind(crate::connection::get_socket_path("lane-task")).unwrap();
    listener.set_nonblocking(true).unwrap();
    std::fs::write(
        sockets.join("runtime-host.pid"),
        std::process::id().to_string(),
    )
    .unwrap();
    std::fs::write(
        sockets.join("runtime-host.version"),
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap();
    crate::connection::write_daemon_auth_token("lane-task", "synthetic-transport-token").unwrap();
    // Non-Linux hosts compare the exact current executable digest rather than PID/inode.
    #[cfg(not(target_os = "linux"))]
    std::fs::write(
        sockets.join("runtime-host.sha256"),
        format!(
            "{:x}",
            Sha256::digest(std::fs::read(std::env::current_exe().unwrap()).unwrap())
        ),
    )
    .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let effects = Arc::new(Mutex::new(Vec::new()));
    let recorded = effects.clone();
    let thread = std::thread::spawn(move || {
        while !stop_thread.load(Ordering::SeqCst) {
            let (socket, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("{error}"),
            };
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(socket);
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                continue;
            }
            let command: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(
                command["_agentBrowserAuthToken"],
                "synthetic-transport-token"
            );
            let admitted = browser_host_policy::check_state(&state, "unused", &command);
            let response = match admitted {
                Err(error) => json!({"id":command["id"],"success":false,"error":error}),
                Ok(()) => {
                    recorded.lock().unwrap().push((
                        command["sessionId"].clone(),
                        command["action"].clone(),
                        command["id"].clone(),
                    ));
                    if matches!(
                        command["id"].as_str(),
                        Some("uncertain-input" | "uncertain-cli-input" | "uncertain-http-input")
                    ) {
                        json!({"id":command["id"],"success":false,"error":"native_input_uncertain: inspect before any new effect"})
                    } else {
                        json!({"id":command["id"],"success":true,"data":{"admitted":true,"capture":{"image":{"mimeType":"image/png","data":"iVBORw0KGgo="}}}})
                    }
                }
            };
            writeln!(reader.get_mut(), "{response}").unwrap();
        }
    });
    let endpoint = Endpoint {
        stop,
        thread: Some(thread),
    };
    let http = NativeHttp::start();
    let call = |task: &str, action: &str, request: Option<&str>, grant: Option<&str>| {
        let params = if action == "native_input" {
            json!({"actions":[{"kind":"move","x":1,"y":1}]})
        } else {
            json!({})
        };
        let mut arguments = json!({"taskId":task,"action":action,"params":params});
        if let Some(request) = request {
            arguments["requestId"] = json!(request);
        }
        if let Some(grant) = grant {
            arguments["policyOverrideId"] = json!(grant);
        }
        handle_jsonrpc_line(&json!({"jsonrpc":"2.0","id":"public","method":"tools/call","params":{"name":"browser_command","arguments":arguments}}).to_string(),"unused").unwrap()
    };
    for action in ["native_input", "native_capture"] {
        let request = format!("request-{action}");
        let grant = format!("grant-{action}");
        let allowed = call("task", action, Some(&request), Some(&grant));
        assert!(allowed.get("error").is_none(), "{allowed}");
        assert!(
            !allowed.to_string().contains(&grant),
            "responses must not echo scoped grants"
        );
        let http_allowed = http.command("task", action, &request, Some(&grant));
        assert_eq!(http_allowed["success"], true, "{http_allowed}");
        assert_eq!(http_allowed["data"]["admitted"], true);
        for (task, actual_action, request, grant) in [
            ("task", action, "wrong-request", Some(grant.as_str())),
            ("peer", action, request.as_str(), Some(grant.as_str())),
            (
                "task",
                if action == "native_input" {
                    "native_capture"
                } else {
                    "native_input"
                },
                request.as_str(),
                Some(grant.as_str()),
            ),
            ("task", action, request.as_str(), None),
            ("task", action, request.as_str(), Some("unknown-grant")),
        ] {
            let denied = call(task, actual_action, Some(request), grant);
            assert!(
                denied["error"]["data"]["message"]
                    .as_str()
                    .unwrap()
                    .starts_with("browser_policy_action_denied:"),
                "{denied}"
            );
            if let Some(grant) = grant {
                assert!(!denied.to_string().contains(grant));
            }
            let http_denied = http.command(task, actual_action, request, grant);
            assert_eq!(http_denied["success"], false, "{http_denied}");
            assert!(
                http_denied["error"]
                    .as_str()
                    .unwrap()
                    .starts_with("browser_policy_action_denied:"),
                "{http_denied}"
            );
        }
        let mut cli_params = if action == "native_input" {
            json!({"actions":[{"kind":"move","x":1,"y":1}]})
        } else {
            json!({})
        };
        cli_params["requestId"] = json!(request);
        cli_params["policyOverrideId"] = json!(grant);
        let cli = mcp_command_response(&[
            "mcp".into(),
            "browser-native".into(),
            "task".into(),
            action.into(),
            cli_params.to_string(),
        ])
        .unwrap();
        assert_eq!(cli["data"]["admitted"], true, "{cli}");
        assert!(!cli.to_string().contains(&grant));
        let mut wrong_cli = cli_params;
        wrong_cli["requestId"] = json!("wrong-cli-request");
        let refused = mcp_command_response(&[
            "mcp".into(),
            "browser-native".into(),
            "task".into(),
            action.into(),
            wrong_cli.to_string(),
        ])
        .unwrap_err();
        assert!(
            refused.starts_with("browser_policy_action_denied:"),
            "{refused}"
        );
        let peer = call("unrestricted", action, Some("unrestricted-request"), None);
        assert!(peer.get("error").is_none(), "{peer}");
        let http_peer = http.command("unrestricted", action, "unrestricted-request", None);
        assert_eq!(http_peer["success"], true, "{http_peer}");
    }
    let uncertain = call(
        "unrestricted",
        "native_input",
        Some("uncertain-input"),
        None,
    );
    assert!(uncertain.to_string().contains("native_input_uncertain"));
    let cli_uncertain = mcp_command_response(&[
        "mcp".into(),
        "browser-native".into(),
        "unrestricted".into(),
        "native_input".into(),
        json!({"requestId":"uncertain-cli-input","actions":[{"kind":"move","x":1,"y":1}]})
            .to_string(),
    ])
    .unwrap_err();
    assert!(cli_uncertain.contains("native_input_uncertain"));
    let http_uncertain = http.command("unrestricted", "native_input", "uncertain-http-input", None);
    assert_eq!(http_uncertain["success"], false);
    assert!(http_uncertain["error"]
        .as_str()
        .unwrap()
        .contains("native_input_uncertain"));
    let dispatched = effects.lock().unwrap().clone();
    assert_eq!(
        dispatched.len(),
        13,
        "refused requests have no Native effect and uncertain input is not replayed"
    );
    for action in ["native_input", "native_capture"] {
        assert_eq!(
            dispatched
                .iter()
                .filter(|(task, actual_action, request)| {
                    task == "task"
                        && actual_action == action
                        && request == &format!("request-{action}")
                })
                .count(),
            3,
            "MCP, CLI and HTTP must use the same task/action/request binding"
        );
    }
    for request in [
        "uncertain-input",
        "uncertain-cli-input",
        "uncertain-http-input",
    ] {
        assert_eq!(
            dispatched
                .iter()
                .filter(|(_, _, actual_request)| actual_request == request)
                .count(),
            1,
            "uncertain Native input is never automatically replayed"
        );
    }
    assert_eq!(
        store
            .load_session_state()
            .unwrap()
            .host_policies
            .operator_overrides
            .len(),
        2,
        "unrelated grants remain intact"
    );
    drop(http);
    drop(endpoint);
    drop(store);
    fn remove_fixture(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::symlink_metadata(path).unwrap().is_dir() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            for child in std::fs::read_dir(path).unwrap() {
                remove_fixture(&child.unwrap().path());
            }
            std::fs::remove_dir(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    remove_fixture(&root);
}
