use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempRuntime {
    root: std::path::PathBuf,
}

impl TempRuntime {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-close-scope-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn socket_dir(&self) -> std::path::PathBuf {
        self.root.join("sockets")
    }
}

impl Drop for TempRuntime {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
struct ChildGuard(std::process::Child);

#[cfg(unix)]
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn explicit_session_global_close_rejects_before_runtime_inspection() {
    let runtime = TempRuntime::new();
    let socket_dir = runtime.socket_dir();
    fs::create_dir_all(&socket_dir).unwrap();
    let pid_path = socket_dir.join("unrelated.pid");
    fs::write(&pid_path, "2147483647\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args(["--session", "scoped", "close", "--all"])
        .env("HOME", &runtime.root)
        .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("close --all is global"), "{stderr}");
    assert!(
        stderr.contains("agent-browser --session scoped close"),
        "{stderr}"
    );
    assert!(
        pid_path.exists(),
        "conflicting close inspected runtime metadata"
    );
}

#[test]
fn explicit_session_global_close_json_matches_the_typed_usage_contract() {
    let runtime = TempRuntime::new();
    let socket_dir = runtime.socket_dir();
    fs::create_dir_all(&socket_dir).unwrap();
    let pid_path = socket_dir.join("unrelated.pid");
    fs::write(&pid_path, "2147483647\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args(["--json", "--session", "scoped", "close", "--all"])
        .env("HOME", &runtime.root)
        .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["success"], false);
    assert_eq!(response["type"], "usage_error");
    assert_eq!(response["code"], "explicit_session_with_global_close");
    assert_eq!(
        response["suggestion"],
        "Use `agent-browser --session scoped close` to close only that session"
    );
    assert!(
        pid_path.exists(),
        "conflicting close inspected runtime metadata"
    );
}

#[cfg(unix)]
#[test]
fn global_close_refuses_an_identity_mismatched_daemon_pid() {
    let runtime = TempRuntime::new();
    let socket_dir = runtime.socket_dir();
    fs::create_dir_all(&socket_dir).unwrap();
    let mut child = ChildGuard(Command::new("sleep").arg("60").spawn().unwrap());
    let pid_path = socket_dir.join("mismatched.pid");
    let identity_path = socket_dir.join("mismatched.identity.json");
    fs::write(&pid_path, format!("{}\n", child.0.id())).unwrap();
    fs::write(
        &identity_path,
        serde_json::to_vec(&serde_json::json!({
            "pid": child.0.id(),
            "startToken": "definitely-not-the-observed-process",
            "executablePath": "/usr/bin/sleep"
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args(["--json", "close", "--all"])
        .env("HOME", &runtime.root)
        .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
        .output()
        .unwrap();

    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success(), "{response}");
    assert_eq!(response["success"], false);
    assert_eq!(response["data"]["closed"], 0);
    assert!(response["data"]["failed"][0]["error"]
        .as_str()
        .unwrap()
        .contains("identity"));
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "mismatched PID was signaled"
    );
    assert!(
        pid_path.exists(),
        "mismatched identity evidence was removed"
    );
    assert!(
        identity_path.exists(),
        "mismatched identity evidence was removed"
    );
}

#[cfg(unix)]
#[test]
fn single_session_close_refuses_an_identity_mismatched_daemon_pid() {
    let runtime = TempRuntime::new();
    let socket_dir = runtime.socket_dir();
    fs::create_dir_all(&socket_dir).unwrap();
    let mut child = ChildGuard(Command::new("sleep").arg("60").spawn().unwrap());
    let pid_path = socket_dir.join("mismatched.pid");
    let identity_path = socket_dir.join("mismatched.identity.json");
    fs::write(&pid_path, format!("{}\n", child.0.id())).unwrap();
    fs::write(
        &identity_path,
        serde_json::to_vec(&serde_json::json!({
            "pid": child.0.id(),
            "startToken": "definitely-not-the-observed-process",
            "executablePath": "/usr/bin/sleep"
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args(["--json", "--session", "mismatched", "close"])
        .env("HOME", &runtime.root)
        .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
        .output()
        .unwrap();

    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success(), "{response}");
    assert_eq!(response["success"], false);
    assert!(response["error"].as_str().unwrap().contains("identity"));
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "mismatched PID was signaled"
    );
    assert!(
        pid_path.exists(),
        "mismatched identity evidence was removed"
    );
    assert!(
        identity_path.exists(),
        "mismatched identity evidence was removed"
    );
}
