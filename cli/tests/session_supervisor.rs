use sha2::{Digest, Sha256};
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempSupervisor {
    root: PathBuf,
}

impl TempSupervisor {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-session-supervisor-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Drop for TempSupervisor {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap();
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(unix)]
#[test]
fn status_reports_a_ready_named_daemon_without_browser_or_install_effects() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempSupervisor::new();
    let supervisor_root = temp.root.join("supervisor");
    let manifest_dir = supervisor_root.join("manifests");
    let socket_dir = temp.root.join("sockets");
    fs::create_dir_all(&manifest_dir).unwrap();
    fs::create_dir_all(&socket_dir).unwrap();

    let executable = Path::new(env!("CARGO_BIN_EXE_agent-browser"))
        .canonicalize()
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let stream_port = listener.local_addr().unwrap().port();
    let session = "supervisor-contract";
    fs::write(
        socket_dir.join(format!("{session}.stream")),
        stream_port.to_string(),
    )
    .unwrap();
    fs::write(
        manifest_dir.join(format!("{session}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.session-supervisor.v1",
            "session": session,
            "executablePath": executable,
            "executableSha256": sha256(&executable),
            "streamPort": stream_port,
            "provenance": {
                "packageVersion": env!("CARGO_PKG_VERSION"),
                "installedAt": "2026-08-11T12:00:00Z",
                "installedBy": "integration-test"
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let fake_systemctl = temp.root.join("systemctl");
    fs::write(
        &fake_systemctl,
        "#!/bin/sh\nprintf '%s\\n' 'LoadState=loaded' 'ActiveState=active' 'SubState=running' 'Result=success' 'NRestarts=1' 'MainPID=4242'\n",
    )
    .unwrap();
    fs::set_permissions(&fake_systemctl, fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new(&executable)
        .args(["--json", "session", "supervisor", "status", session])
        .env("HOME", &temp.root)
        .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
        .env("AGENT_BROWSER_SESSION_SUPERVISOR_ROOT", &supervisor_root)
        .env(
            "AGENT_BROWSER_SESSION_SUPERVISOR_SYSTEMCTL",
            &fake_systemctl,
        )
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["state"], "ready");
    assert_eq!(response["data"]["ready"], true);
    assert_eq!(response["data"]["streamPort"], stream_port);
    assert_eq!(response["data"]["publishedStreamPort"], stream_port);
    assert_eq!(response["data"]["executableMatches"], true);
    assert_eq!(response["data"]["mainPid"], 4242);
    assert!(!temp.root.join(".agent-browser/runtime-profiles").exists());
}
