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
            "agent-browser-workstation-cold-install-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temporary workstation root");
        Self { root }
    }

    fn install_fake_command(&self, name: &str, log_env: &str) {
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
    }
}

impl Drop for TempWorkstation {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_apply(
    fixture: &TempWorkstation,
    docker_log: &Path,
    systemctl_log: &Path,
    fail_after: Option<&str>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agent-browser"));
    command
        .args(["install", "workstation", "--apply", "--json"])
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
        .env("PATH", fixture.root.join("bin"));
    if let Some(fail_after) = fail_after {
        command.env("AGENT_BROWSER_WORKSTATION_FAIL_AFTER", fail_after);
    }
    command.output().expect("cold workstation apply")
}

#[test]
fn ordinary_apply_ignores_stale_hot_upgrade_metadata_and_selects_a_cold_generation() {
    let fixture = TempWorkstation::new();
    let docker_log = fixture.root.join("docker.log");
    let systemctl_log = fixture.root.join("systemctl.log");
    fixture.install_fake_command("docker", "AGENT_BROWSER_FAKE_DOCKER_LOG");
    fixture.install_fake_command("systemctl", "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG");

    let stale_root = fixture
        .root
        .join("workstation/.agent-browser/runtime-adoption");
    fs::create_dir_all(stale_root.join("upgrade-transactions")).expect("stale metadata root");
    fs::write(
        stale_root.join("upgrade-transactions/interrupted.json"),
        b"contradictory retained transaction metadata",
    )
    .expect("stale transaction");
    fs::write(
        stale_root.join("admission-drain.json"),
        b"stale admission drain",
    )
    .expect("stale drain");

    let output = run_apply(&fixture, &docker_log, &systemctl_log, None);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cold install receipt");
    assert_eq!(
        receipt["schemaVersion"],
        "agent-browser.workstation-cold-install.v1"
    );
    assert_eq!(receipt["success"], true);
    assert_eq!(
        receipt["steps"]
            .as_array()
            .expect("cold steps")
            .iter()
            .map(|step| step["phase"].as_str().expect("phase"))
            .collect::<Vec<_>>(),
        vec!["stop", "replace", "start", "readiness"]
    );
    assert!(receipt.get("transactionId").is_none());

    let workstation = fixture.root.join("workstation");
    let selected = fs::read_link(workstation.join(".local/lib/agent-browser/current"))
        .expect("selected generation");
    assert!(selected.starts_with("generations/"));
    assert!(workstation.join(".local/bin/agent-browser").is_file());
    assert!(stale_root
        .join("upgrade-transactions/interrupted.json")
        .is_file());
    assert!(stale_root.join("admission-drain.json").is_file());
    assert!(!systemctl_log.exists());

    let selected_before_failure = selected;
    let failed = run_apply(
        &fixture,
        &docker_log,
        &systemctl_log,
        Some("selector-committed"),
    );
    assert!(!failed.status.success());
    let failure: serde_json::Value =
        serde_json::from_slice(&failed.stdout).expect("failed cold install receipt");
    assert_eq!(failure["success"], false);
    assert_eq!(
        failure["originalError"],
        "Injected workstation install failure after selector-committed"
    );
    assert_eq!(failure["rollback"]["phase"], "rollback");
    assert!(failure["rollback"].get("error").is_none());
    assert_eq!(
        fs::read_link(workstation.join(".local/lib/agent-browser/current"))
            .expect("restored selected generation"),
        selected_before_failure
    );
}
