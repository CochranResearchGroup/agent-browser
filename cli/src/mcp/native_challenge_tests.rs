//! Addressed public Native challenge requests must resolve their original
//! observation before a transport can see an input request.
use super::*;

#[test]
fn native_challenge_missing_original_capture_refuses_before_transport() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::BrowserSessionState;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    let root = std::env::temp_dir().join(format!("native-challenge-{}", uuid::Uuid::new_v4()));
    let sockets = root.join("s");
    std::fs::create_dir_all(&sockets).unwrap();
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
        "AGENT_BROWSER_RUNTIME_HOST",
    ]);
    for key in ["HOME", "AGENT_BROWSER_HOME"] {
        guard.set(key, root.to_str().unwrap());
    }
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set("AGENT_BROWSER_SOCKET_DIR", sockets.to_str().unwrap());
    guard.set("AGENT_BROWSER_RUNTIME_HOST", "1");
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["sessions"] = json!({"task":{"id":"task","name":"challenge-lane","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":null}});
    value["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":std::process::id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    let state: BrowserSessionState = serde_json::from_value(value).unwrap();
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    store.save_session_state(&state).unwrap();
    let listener =
        UnixListener::bind(crate::connection::get_socket_path("challenge-lane")).unwrap();
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
    crate::connection::write_daemon_auth_token("challenge-lane", "synthetic-native-token").unwrap();
    #[cfg(not(target_os = "linux"))]
    {
        use sha2::Digest;
        std::fs::write(
            sockets.join("runtime-host.sha256"),
            format!(
                "{:x}",
                sha2::Sha256::digest(std::fs::read(std::env::current_exe().unwrap()).unwrap())
            ),
        )
        .unwrap();
    }
    let stop = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicUsize::new(0));
    let (stopped, recorded) = (stop.clone(), requests.clone());
    let endpoint = std::thread::spawn(move || {
        while !stopped.load(Ordering::SeqCst) {
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
            recorded.fetch_add(1, Ordering::SeqCst);
            writeln!(reader.get_mut(), "{}", json!({"id":command["id"],"success":true,"data":{"source":"remote_view_native","delivery":"dispatched","taskCompleted":false}})).unwrap();
        }
    });
    let request = json!({"jsonrpc":"2.0","id":"native-challenge","method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"single-attempt","params":{"nativeAttempt":{"observationRequestId":"missing-original","registeredActionId":"hcaptcha-checkbox-p181-v2"},"actions":[{"kind":"click","x":5,"y":5,"button":1,"count":1}]}}}});
    let response = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
    let mut refused = Vec::new();
    for (label, native_params, addressed, identified) in [
        (
            "no-task",
            json!({"nativeAttempt":{"observationRequestId":"missing-original","registeredActionId":"hcaptcha-checkbox-p181-v2"}}),
            false,
            true,
        ),
        (
            "no-request",
            json!({"nativeAttempt":{"observationRequestId":"missing-original","registeredActionId":"hcaptcha-checkbox-p181-v2"}}),
            true,
            false,
        ),
        (
            "wrong-type",
            json!({"nativeAttempt":"caller-label"}),
            true,
            true,
        ),
        ("null", json!({"nativeAttempt":null}), true, true),
        (
            "missing-field",
            json!({"nativeAttempt":{"observationRequestId":"missing-original"}}),
            true,
            true,
        ),
        (
            "unknown-action",
            json!({"nativeAttempt":{"observationRequestId":"missing-original","registeredActionId":"caller-template"}}),
            true,
            true,
        ),
    ] {
        let mut request = request.clone();
        request["params"]["arguments"]["params"] = native_params;
        if !addressed {
            request["params"]["arguments"]
                .as_object_mut()
                .unwrap()
                .remove("taskId");
        }
        if !identified {
            request["params"]["arguments"]
                .as_object_mut()
                .unwrap()
                .remove("requestId");
        }
        refused.push((
            label,
            handle_jsonrpc_line(&request.to_string(), "unused").unwrap(),
        ));
    }
    stop.store(true, Ordering::SeqCst);
    endpoint.join().unwrap();
    let effects = requests.load(Ordering::SeqCst);
    let refusal_results = refused;
    drop(store);
    fn remove_owned(path: &std::path::Path) {
        if !path.is_dir() {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                remove_owned(&entry.path());
            }
        }
    }
    remove_owned(&root);
    std::fs::remove_dir_all(&root).unwrap();
    for (label, response) in refusal_results {
        assert!(response.get("error").is_some(), "{label}:{response}");
    }
    assert!(
        response
            .to_string()
            .contains("browser_native_original_observation_missing"),
        "{response}"
    );
    assert_eq!(
        effects, 0,
        "Missing original evidence must stop before transport"
    );
}
