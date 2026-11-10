//! Public MCP control reconciliation using an isolated existing-owner transport.
use super::*;
use crate::native::browser_session_store::BrowserSessionSqliteStore;
use agent_browser_service_model::BrowserSessionState;
use std::io::{BufRead, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

struct Fixture {
    root: std::path::PathBuf,
    child: std::process::Child,
    commands: Arc<Mutex<Vec<Value>>>,
    response: Arc<Mutex<Value>>,
    publication: Arc<Mutex<Option<&'static str>>>,
    stop: Arc<AtomicBool>,
    server: Option<std::thread::JoinHandle<()>>,
    _guard: crate::test_utils::EnvGuard<'static>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(server) = self.server.take() {
            server.join().unwrap();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!("control611-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let executable = root.join("chrome");
    std::fs::copy("/bin/sleep", &executable).unwrap();
    let child = std::process::Command::new(&executable)
        .arg("120")
        .spawn()
        .unwrap();
    let crate::process_identity::ProcessObservation::Observed(process) =
        crate::process_identity::observe_process(child.id())
    else {
        panic!("owned process")
    };
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
        "AGENT_BROWSER_RUNTIME_HOST",
    ]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    let sockets = root.join("sockets");
    std::fs::create_dir_all(&sockets).unwrap();
    guard.set("AGENT_BROWSER_SOCKET_DIR", sockets.to_str().unwrap());
    guard.set("AGENT_BROWSER_RUNTIME_HOST", "0");
    let mut state = serde_json::to_value(BrowserSessionState::default()).unwrap();
    state["sessions"] = json!({"task":{"id":"task","name":"reconcile611","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":"tab"}});
    state["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":child.id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    state["tabs"] = json!({"tab":{"id":"tab","targetId":"target","browserId":"browser","sessionId":"task","createdAtMs":1,"lastActivityAtMs":1}});
    state["sharedBrowserIds"] = json!(["browser"]);
    state["adoptedBrowserProcessIdentities"] = json!({"browser":{"profileId":"profile","cdpEndpoint":"","process":{"pid":process.pid,"startToken":process.start_token,"executablePath":process.executable_path,"browserFamily":process.browser_family}}});
    BrowserSessionSqliteStore::default_sqlite()
        .unwrap()
        .save_session_state(&serde_json::from_value(state).unwrap())
        .unwrap();
    // A completed owned dispatcher gives positive original-owner absence.
    let mut dispatcher = std::process::Command::new("/bin/true").spawn().unwrap();
    let pid = dispatcher.id();
    dispatcher.wait().unwrap();
    let path = control::path("browser").unwrap();
    std::fs::write(path,json!({"epoch":2,"human":false,"pauseReads":false,"awaitingObservation":false,"actor":"fixture","reason":"returned","flights":{"original":"uncertain_no_replay"},"owners":{"original":[pid,"fixture-original-start"]},"audit":[]}).to_string()).unwrap();
    let listener =
        std::os::unix::net::UnixListener::bind(crate::connection::get_socket_path("reconcile611"))
            .unwrap();
    listener.set_nonblocking(true).unwrap();
    let key = crate::runtime_host::endpoint_key("reconcile611");
    std::fs::write(
        sockets.join(format!("{key}.pid")),
        std::process::id().to_string(),
    )
    .unwrap();
    std::fs::write(
        sockets.join(format!("{key}.version")),
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap();
    crate::connection::write_daemon_auth_token("reconcile611", "fixture-token").unwrap();
    let commands = Arc::new(Mutex::new(Vec::new()));
    let observed = commands.clone();
    let response = Arc::new(Mutex::new(
        json!({"success":true,"data":{"url":"https://example.test/retained"}}),
    ));
    let reply = response.clone();
    let publication = Arc::new(Mutex::new(None));
    let publish = publication.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let server = std::thread::spawn(move || {
        while !stopped.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
                        .unwrap();
                    let mut line = String::new();
                    if std::io::BufReader::new(&stream)
                        .read_line(&mut line)
                        .is_ok()
                        && !line.is_empty()
                    {
                        let command: Value = serde_json::from_str(&line).unwrap();
                        observed.lock().unwrap().push(command);
                        if let Some(change) = *publish.lock().unwrap() {
                            let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
                            let mut state =
                                serde_json::to_value(store.load_session_state().unwrap()).unwrap();
                            match change {
                                "activity" => {
                                    state["sessions"]["task"]["lastActivityAtMs"] = json!(2);
                                    state["sessions"]["task"]["expiresAtMs"] = json!(200);
                                    state["tabs"]["tab"]["lastActivityAtMs"] = json!(2);
                                }
                                "target" => {
                                    state["tabs"]["tab"]["targetId"] = json!("another-target")
                                }
                                "process" => {
                                    state["adoptedBrowserProcessIdentities"]["browser"]["process"]
                                        ["startToken"] = json!("another-process-instance")
                                }
                                "epoch" => {
                                    let path = control::path("browser").unwrap();
                                    let mut current: Value =
                                        serde_json::from_slice(&std::fs::read(&path).unwrap())
                                            .unwrap();
                                    current["epoch"] = json!(3);
                                    std::fs::write(path, current.to_string()).unwrap();
                                }
                                _ => panic!("unsupported fixture publication"),
                            }
                            store
                                .save_session_state(&serde_json::from_value(state).unwrap())
                                .unwrap();
                        }

                        let mut bytes = serde_json::to_vec(&*reply.lock().unwrap()).unwrap();
                        bytes.push(b'\n');
                        let _ = stream.write_all(&bytes);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                Err(error) => panic!("fixture transport: {error}"),
            }
        }
    });
    Fixture {
        root,
        child,
        commands,
        response,
        publication,
        stop,
        server: Some(server),
        _guard: guard,
    }
}
fn request() -> Value {
    json!({"jsonrpc":"2.0","id":611,"method":"tools/call","params":{"name":"browser_control","arguments":{"taskId":"task","operation":"reconcile","expectedEpoch":2,"actor":"fixture-owner","reason":"Freshly observe terminal original without replay","requestId":"original"}}})
}
fn call(request: Value) -> Value {
    crate::mcp::handle_jsonrpc_line(&request.to_string(), "unused").unwrap()
}
fn read_control() -> Value {
    serde_json::from_slice(&std::fs::read(control::path("browser").unwrap()).unwrap()).unwrap()
}
#[test]
fn terminal_control611_public_reconcile_preserves_original_without_human_return() {
    let f = fixture();
    let result = call(request());
    assert!(result.get("error").is_none(), "{result}");
    let status: Value =
        serde_json::from_str(result["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(status["data"]["control"]["epoch"], 2);
    assert_eq!(status["data"]["control"]["owner"], "agent");
    assert_eq!(status["data"]["control"]["pendingOutcomeCount"], 0);
    let retained = read_control();
    assert_eq!(
        retained["flights"]["original"],
        "observed_uncertain_no_replay"
    );
    assert!(retained["owners"]["original"].is_array());
    assert_eq!(retained["audit"][0]["operation"], "reconcile");
    let briefing = call(
        json!({"jsonrpc":"2.0","id":612,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"task_briefing"}}}),
    );
    let briefing: Value =
        serde_json::from_str(briefing["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        briefing["data"]["uncertainOperations"][0]["operationId"],
        "original"
    );
    assert_eq!(
        briefing["data"]["uncertainOperations"][0]["retryAllowed"],
        false
    );
    assert_eq!(
        briefing["data"]["uncertainOperations"][0]["state"],
        "observed_uncertain_no_replay"
    );
    let commands = f.commands.lock().unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0]["action"], "url");
    assert_eq!(commands[0]["targetId"], "target");
    assert_eq!(commands[0]["sessionId"], "task");
}

#[test]
fn terminal_control611_public_refusals_keep_original_and_never_dispatch_input() {
    for (case, expected) in [
        ("human", "browser_control_human"),
        ("active", "browser_control_dispatch_still_in_flight"),
        ("unknown_owner", "browser_control_dispatch_owner_unknown"),
        ("stale_epoch", "browser_control_epoch_changed"),
        (
            "original_process_missing",
            "browser_control_original_process_required",
        ),
        ("cross_target", "browser_control_original_target_required"),
    ] {
        let f = fixture();
        let mut original = read_control();
        match case {
            "human" => original["human"] = json!(true),
            "active" => {
                let crate::process_identity::ProcessObservation::Observed(owner) =
                    crate::process_identity::observe_process(std::process::id())
                else {
                    panic!("fixture owner")
                };
                original["owners"]["original"] = json!([owner.pid, owner.start_token.unwrap()]);
            }
            "unknown_owner" => {
                original["owners"] = json!({});
            }
            _ => {}
        }
        std::fs::write(control::path("browser").unwrap(), original.to_string()).unwrap();
        if matches!(case, "original_process_missing" | "cross_target") {
            let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
            let mut state = serde_json::to_value(store.load_session_state().unwrap()).unwrap();
            if case == "original_process_missing" {
                state["adoptedBrowserProcessIdentities"] = json!({});
            } else {
                state["tabs"]["tab"]["browserId"] = json!("another-browser");
            }
            store
                .save_session_state(&serde_json::from_value(state).unwrap())
                .unwrap();
        }
        let mut intent = request();
        if case == "stale_epoch" {
            intent["params"]["arguments"]["expectedEpoch"] = json!(1);
        }
        let result = call(intent);
        assert!(result.to_string().contains(expected), "{case}: {result}");
        assert_eq!(
            read_control(),
            original,
            "{case} must retain original control/history"
        );
        assert!(
            f.commands.lock().unwrap().is_empty(),
            "{case} must refuse before any transport"
        );
    }
}

#[test]
fn terminal_control611_public_reused_dispatcher_reconciles_once_without_retry() {
    let f = fixture();
    let mut original = read_control();
    original["owners"]["original"] = json!([std::process::id(), "previous-process-instance"]);
    std::fs::write(control::path("browser").unwrap(), original.to_string()).unwrap();
    let first = call(request());
    assert!(first.get("error").is_none(), "{first}");
    let observed = read_control();
    let second = call(request());
    assert!(second
        .to_string()
        .contains("browser_control_outcome_already_observed"));
    assert_eq!(read_control(), observed);
    assert_eq!(f.commands.lock().unwrap().len(), 1);
}

#[test]
fn terminal_control611_public_wrong_or_nonlive_observation_keeps_uncertainty() {
    for response in [
        json!({"url":"https://example.test/other","targetId":"other-target"}),
        json!({"url":"https://example.test/retained","urlObserved":false}),
        json!({"mode":"cdp_free"}),
    ] {
        let f = fixture();
        let original = read_control();
        *f.response.lock().unwrap() = json!({"success":true,"data":response});
        let result = call(request());
        assert!(
            result
                .to_string()
                .contains("browser_control_fresh_target_observation_required"),
            "{result}"
        );
        assert_eq!(read_control(), original);
        let commands = f.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0]["action"], "url");
        assert_eq!(commands[0]["retainedTaskOnly"], true);
    }
}

#[test]
fn terminal_control611_public_activity_publication_keeps_stable_identity() {
    let f = fixture();
    *f.publication.lock().unwrap() = Some("activity");
    let result = call(request());
    assert!(result.get("error").is_none(), "{result}");
    assert_eq!(
        read_control()["flights"]["original"],
        "observed_uncertain_no_replay"
    );
    assert_eq!(read_control()["epoch"], 2);
    assert_eq!(f.commands.lock().unwrap().len(), 1);
}
#[test]
fn terminal_control611_public_target_process_epoch_races_preserve_original() {
    for (change, expected) in [
        ("target", "browser_control_observation_target_changed"),
        ("process", "browser_control_original_process_changed"),
        ("epoch", "browser_control_epoch_changed"),
    ] {
        let f = fixture();
        *f.publication.lock().unwrap() = Some(change);
        let result = call(request());
        assert!(result.to_string().contains(expected), "{change}: {result}");
        let control = read_control();
        assert_eq!(control["flights"]["original"], "uncertain_no_replay");
        assert_eq!(control["audit"], json!([]));
        let commands = f.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0]["action"], "url");
    }
}

#[test]
fn terminal_request619_public_completed_dom_request_with_live_supervisor_reconciles() {
    let f = fixture();
    let crate::process_identity::ProcessObservation::Observed(owner) =
        crate::process_identity::observe_process(std::process::id())
    else {
        panic!("owner")
    };
    let mut original = read_control();
    original["owners"]["original"] = json!([owner.pid, owner.start_token]);
    std::fs::write(control::path("browser").unwrap(), original.to_string()).unwrap();
    BrowserSessionSqliteStore::default_sqlite().unwrap().mutate_session_state(|state| {
        let identity = state.adopted_browser_process_identities["browser"].process.clone();
        let value = json!({"requestId":"original","taskId":"task","profileId":"profile","browserId":"browser","tabId":"tab","targetId":"target","processPid":identity.pid,"processIdentity":identity,"controlEpoch":2,"action":"click","fingerprint":"a".repeat(64),"state":"uncertain","createdAtMs":1,"completedAtMs":2,"evidenceReferences":{}});
        state.task_operations.entry("task".into()).or_default().insert("original".into(), serde_json::from_value(value).unwrap());
        Ok(())
    }).unwrap();
    let result = call(request());
    assert!(result.get("error").is_none(), "{result}");
    assert_eq!(
        read_control()["flights"]["original"],
        "observed_uncertain_no_replay"
    );
    assert_eq!(f.commands.lock().unwrap().len(), 1);
}
#[test]
fn terminal_request619_public_incomplete_or_private_request_keeps_live_dispatch_guard() {
    for case in [
        "pending",
        "native",
        "missing_completion",
        "epoch",
        "target",
        "active_flight",
    ] {
        let f = fixture();
        let crate::process_identity::ProcessObservation::Observed(owner) =
            crate::process_identity::observe_process(std::process::id())
        else {
            panic!("owner")
        };
        let mut original = read_control();
        original["owners"]["original"] = json!([owner.pid, owner.start_token]);
        std::fs::write(control::path("browser").unwrap(), original.to_string()).unwrap();
        BrowserSessionSqliteStore::default_sqlite().unwrap().mutate_session_state(|state| {
        let identity = state.adopted_browser_process_identities["browser"].process.clone();
        let value = json!({"requestId":"original","taskId":"task","profileId":"profile","browserId":"browser","tabId":"tab","targetId":"target","processPid":identity.pid,"processIdentity":identity,"controlEpoch":2,"action":"click","fingerprint":"a".repeat(64),"state":"uncertain","createdAtMs":1,"completedAtMs":2,"evidenceReferences":{}});
        state.task_operations.entry("task".into()).or_default().insert("original".into(), serde_json::from_value(value).unwrap());
        Ok(())
    }).unwrap();
        if case == "active_flight" {
            let mut control_state = read_control();
            control_state["flights"]["original"] = json!("dispatched");
            std::fs::write(control::path("browser").unwrap(), control_state.to_string()).unwrap();
        } else {
            BrowserSessionSqliteStore::default_sqlite()
                .unwrap()
                .mutate_session_state(|state| {
                    let row = state
                        .task_operations
                        .get_mut("task")
                        .unwrap()
                        .get_mut("original")
                        .unwrap();
                    match case {
                        "pending" => {
                            row.state =
                                agent_browser_service_model::BrowserTaskOperationState::Pending
                        }
                        "native" => row.action = "native_input".into(),
                        "missing_completion" => row.completed_at_ms = None,
                        "epoch" => row.control_epoch = 1,
                        "target" => row.target_id = "other".into(),
                        _ => unreachable!(),
                    }
                    Ok(())
                })
                .unwrap();
        }
        let before = read_control();
        let result = call(request());
        assert!(
            result
                .to_string()
                .contains("browser_control_dispatch_still_in_flight"),
            "{case}: {result}"
        );
        assert_eq!(read_control(), before);
        assert!(f.commands.lock().unwrap().is_empty());
    }
}
