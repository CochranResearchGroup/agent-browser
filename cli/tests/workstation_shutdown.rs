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

#[test]
fn shutdown_json_releases_a_retained_session_without_deleting_profile_data() {
    let fixture = TempWorkstation::new();
    let docker_log = fixture.root.join("docker.log");
    let systemctl_log = fixture.root.join("systemctl.log");
    fixture.install_fake_command("docker", "AGENT_BROWSER_FAKE_DOCKER_LOG");
    fixture.install_fake_command("systemctl", "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG");

    let profile_dir = fixture.root.join("profiles/named");
    fs::create_dir_all(&profile_dir).expect("profile directory");
    let marker_path = profile_dir.join("retained-profile-data");
    fs::write(&marker_path, b"keep").expect("profile marker");

    let state_path = fixture.root.join("home/.agent-browser/service/state.json");
    fs::create_dir_all(state_path.parent().expect("service directory")).expect("service directory");
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.service-state.v2",
            "profiles": {
                "named": {
                    "id": "named",
                    "userDataDir": profile_dir,
                    "persistent": true
                }
            },
            "sessions": {
                "session-1": {
                    "id": "session-1",
                    "lease": "exclusive",
                    "profileId": "named"
                }
            }
        }))
        .expect("service state JSON"),
    )
    .expect("service state");

    let first = run_shutdown(&fixture, &docker_log, &systemctl_log);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_receipt: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("first shutdown receipt");
    assert_eq!(first_receipt["success"], true);
    assert_eq!(first_receipt["changed"], true);
    assert_eq!(first_receipt["residue"]["runtimeOwners"], 0);
    assert_eq!(first_receipt["residue"]["activeLeases"], 0);

    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("persisted service state"))
            .expect("persisted service state JSON");
    assert_eq!(persisted["sessions"]["session-1"]["lease"], "released");
    assert_eq!(
        persisted["profiles"]["named"]["userDataDir"],
        profile_dir.to_string_lossy().as_ref()
    );
    assert_eq!(
        fs::read(&marker_path).expect("retained profile marker"),
        b"keep"
    );

    let replay = run_shutdown(&fixture, &docker_log, &systemctl_log);
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let replay_receipt: serde_json::Value =
        serde_json::from_slice(&replay.stdout).expect("replay shutdown receipt");
    assert_eq!(replay_receipt["success"], true);
    assert_eq!(replay_receipt["changed"], false);
    assert_eq!(
        fs::read(&marker_path).expect("retained profile marker"),
        b"keep"
    );
    assert!(!systemctl_log.exists());
}
