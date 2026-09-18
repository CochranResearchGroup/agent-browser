#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempWorkstation {
    root: PathBuf,
}

impl TempWorkstation {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-workstation-shutdown-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temporary workstation root");
        Self { root }
    }

    fn install_fake_command(&self, name: &str, log_env: &str) -> PathBuf {
        let bin_dir = self.root.join("bin");
        fs::create_dir_all(&bin_dir).expect("fake command directory");
        let path = bin_dir.join(name);
        fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"${{{log_env}}}\"\nexit 1\n"),
        )
        .expect("fake command");
        let mut permissions = fs::metadata(&path)
            .expect("fake command metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("fake command permissions");
        path
    }
}

impl Drop for TempWorkstation {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_shutdown(fixture: &TempWorkstation, docker_log: &Path, systemctl_log: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args(["shutdown", "--json"])
        .current_dir(&fixture.root)
        .env("HOME", fixture.root.join("home"))
        .env("XDG_RUNTIME_DIR", fixture.root.join("runtime"))
        .env(
            "AGENT_BROWSER_WORKSTATION_ROOT",
            fixture.root.join("workstation"),
        )
        .env(
            "AGENT_BROWSER_SOCKET_DIR",
            fixture.root.join("runtime/sockets"),
        )
        .env("AGENT_BROWSER_FAKE_DOCKER_LOG", docker_log)
        .env("AGENT_BROWSER_FAKE_SYSTEMCTL_LOG", systemctl_log)
        .env("PATH", fixture.root.join("bin"))
        .output()
        .expect("shutdown command")
}

#[test]
fn shutdown_json_is_idempotent_in_an_empty_disposable_workstation() {
    let fixture = TempWorkstation::new();
    let docker_log = fixture.root.join("docker.log");
    let systemctl_log = fixture.root.join("systemctl.log");
    fixture.install_fake_command("docker", "AGENT_BROWSER_FAKE_DOCKER_LOG");
    fixture.install_fake_command("systemctl", "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG");

    for run in 1..=2 {
        let output = run_shutdown(&fixture, &docker_log, &systemctl_log);
        assert!(
            output.status.success(),
            "run {run}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("shutdown JSON receipt");
        assert_eq!(
            response["schemaVersion"],
            "agent-browser.workstation-shutdown.v1"
        );
        assert_eq!(response["success"], true);
        assert_eq!(response["changed"], false);
        assert_eq!(
            response["steps"]
                .as_array()
                .expect("shutdown steps")
                .iter()
                .map(|step| step["phase"].as_str().expect("phase"))
                .collect::<Vec<_>>(),
            vec![
                "browsers",
                "user_units",
                "containers",
                "ownership",
                "transient_metadata",
                "verify",
            ]
        );
        assert_eq!(
            response["residue"],
            serde_json::json!({
                "ownedBrowsers": 0,
                "ownedUserUnits": 0,
                "ownedContainers": 0,
                "runtimeOwners": 0,
                "activeLeases": 0,
                "foreignProcesses": 0,
            })
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains("transaction"),
            "shutdown receipt exposed upgrade coordination state"
        );
    }

    let docker_calls = fs::read_to_string(&docker_log).expect("docker calls");
    let expected_once = [
        "top agent-browser-guacamole",
        "top agent-browser-guacd",
        "top agent-browser-guacamole-postgres",
        "top agent-browser-guacamole",
        "top agent-browser-guacd",
        "top agent-browser-guacamole-postgres",
    ];
    assert_eq!(
        docker_calls.lines().collect::<Vec<_>>(),
        expected_once
            .into_iter()
            .chain(expected_once)
            .collect::<Vec<_>>()
    );
    assert!(
        !systemctl_log.exists(),
        "shutdown inspected user units that were not installed"
    );
}
