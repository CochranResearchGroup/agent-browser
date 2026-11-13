use super::*;
use crate::test_utils::EnvGuard;
use agent_browser_service_model::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

// The public host uses its real SQLite persistence and real owner HTTP adapter.
// Only the external desktop owner is synthetic; no browser process is launched.
fn disabled_owner(action: &str) -> (Result<Value, String>, usize) {
    let root = std::env::temp_dir().join(format!("native-host-readiness-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let token = root.join("owner.token");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&token)
        .unwrap()
        .write_all(b"synthetic-owner-token-519-123456")
        .unwrap();
    let (store, _) =
        BrowserSessionSqliteStore::open_or_migrate(&root, &root.join("legacy-state.json")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let guard = EnvGuard::new(&[
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
    ]);
    guard.set("AGENT_BROWSER_REMOTE_VIEW_ORIGIN", &origin);
    guard.set(
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
        token.to_str().unwrap(),
    );
    let stop = Arc::new(AtomicBool::new(false));
    let inputs = Arc::new(AtomicUsize::new(0));
    let thread_stop = stop.clone();
    let thread_inputs = inputs.clone();
    let window_only = action.starts_with("native_window_");
    let server = std::thread::spawn(move || {
        while !thread_stop.load(Ordering::SeqCst) {
            let (mut socket, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("fixture accept failed: {e}"),
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = socket.read(&mut chunk).unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(at) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..at]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= at + 4 + length {
                        break;
                    }
                }
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.lines().any(|line| line
                .eq_ignore_ascii_case("authorization: Bearer synthetic-owner-token-519-123456")));
            let first = request.lines().next().unwrap();
            let reply = if first.starts_with("GET /v1/desktops/lifecycle-519 ") {
                assert!(
                    !window_only,
                    "lifecycle window observation must not query input policy"
                );
                json!({"schemaVersion":1,"desktop":{"desktopId":"lifecycle-519","generation":3,"state":"local_ready"},"inputCapability":{"policyState":"disabled"}})
            } else if first.starts_with("POST /v1/agent-desktop/observe ") {
                json!({"desktopId":"viewing-519","generation":7,"windows":{"windows":[{"id":5,"pid":42,"active":true,"x":0,"y":0,"width":400,"height":300,"processIdentity":{"pid":42,"executable":"/synthetic/chrome","boot_id":"fixture-boot","start_token":"99"}}]},"geometry":{"current":{"width":800,"height":600}}})
            } else if first.starts_with("POST /v1/agent-desktop/input ") {
                thread_inputs.fetch_add(1, Ordering::SeqCst);
                json!({"delivery":"accepted"})
            } else {
                panic!("unexpected fixture owner operation")
            };
            let body = serde_json::to_string(&reply).unwrap();
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let host = BrowserSessionHost {
        persistence: store,
        effects: (),
        catalog: BrowserProfileCatalog::default(),
        state: BrowserSessionState::default(),
        persisted_state: BrowserSessionState::default(),
        manager_config: BrowserSessionManagerConfig::default(),
        exact_url_history_maximum_bytes: 0,
        native_use_cursor: 0,
    };
    let original = RecordedProcessIdentity {
        pid: if action == "native_window_wrong_process" {
            43
        } else {
            42
        },
        start_token: "linux:fixture-boot:99".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: None,
    };
    let target = RemoteViewApplicationTarget {
        assignment_id: "assignment-519".into(),
        registration_id: "consumer-519".into(),
        desktop_id: "lifecycle-519".into(),
        lifecycle_generation: 3,
        viewing_desktop_id: "viewing-519".into(),
        viewing_generation: 7,
    };
    let owner_action = if action.starts_with("native_window_") {
        "native_ready"
    } else {
        action
    };
    let command = json!({"action":owner_action,"actions":[{"kind":"key","chord":"Tab"}]});
    let result = if action.starts_with("native_window_") {
        host.native_process_window_ready(&original, &target)
    } else {
        host.native_process_command(&original, &target, &command)
    };
    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
    let count = inputs.load(Ordering::SeqCst);
    drop(host);
    drop(guard);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in std::fs::read_dir(&root).unwrap() {
            let entry = entry.unwrap();
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with("migration-archive-")
            {
                std::fs::set_permissions(entry.path(), std::fs::Permissions::from_mode(0o700))
                    .unwrap();
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
    (result, count)
}

#[test]
fn native_host_readiness_reports_disabled_policy_at_public_process_boundary() {
    let (result, inputs) = disabled_owner("native_ready");
    let result = result.unwrap();
    assert_eq!(result["ready"], false);
    assert_eq!(result["inputReadiness"]["policyState"], "disabled");
    assert_eq!(result["inputReadiness"]["viewerReadiness"], "not_checked");
    assert_eq!(inputs, 0);
}

#[test]
fn native_host_readiness_stops_disabled_input_before_owner_dispatch() {
    let (result, inputs) = disabled_owner("native_input");
    assert_eq!(
        result.unwrap_err(),
        "browser_native_input_disabled:configure_consumer_selective_input"
    );
    assert_eq!(inputs, 0, "disabled consumer must receive no input request");
}

#[test]
fn native_host_lifecycle_window_ready_ignores_disabled_input_policy() {
    let (result, inputs) = disabled_owner("native_window_ready");
    let result = result.unwrap();
    assert_eq!(
        result["ready"], true,
        "visible replacement must complete lifecycle qualification independently of input policy"
    );
    assert!(result.get("inputReadiness").is_none());
    assert_eq!(inputs, 0);
}

#[test]
fn native_host_lifecycle_window_ready_rejects_different_process() {
    let (result, inputs) = disabled_owner("native_window_wrong_process");
    assert!(
        result.is_err(),
        "a different process must not qualify lifecycle completion"
    );
    assert_eq!(inputs, 0);
}
