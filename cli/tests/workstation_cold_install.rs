#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::Duration;
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
        make_tree_owner_writable(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_tree_owner_writable(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if !metadata.is_dir() {
        return;
    }
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions.mode() | 0o700);
    let _ = fs::set_permissions(path, permissions);
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            make_tree_owner_writable(&entry.path());
        }
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

fn run_installed(fixture: &TempWorkstation, args: &[&str]) -> Output {
    Command::new(fixture.root.join("workstation/.local/bin/agent-browser"))
        .args(args)
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
        .env("AGENT_BROWSER_RUNTIME_HOST", "1")
        .env("AGENT_BROWSER_RDP_ROUTE_POOL_JSON", "[]")
        .env(
            "AGENT_BROWSER_FAKE_DOCKER_LOG",
            fixture.root.join("docker.log"),
        )
        .env(
            "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG",
            fixture.root.join("systemctl.log"),
        )
        .env("PATH", fixture.root.join("bin"))
        .output()
        .expect("installed agent-browser command")
}

fn successful_json(output: &Output, context: &str) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{context}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{context}: invalid JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn process_start_token(pid: u32) -> Option<String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(") ")?.1;
    let start_ticks = after_name.split_whitespace().nth(19)?;
    let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    Some(format!("linux:{}:{start_ticks}", boot_id.trim()))
}

fn read_live_process_token(path: &Path) -> (u32, String) {
    for _ in 0..100 {
        if let Ok(pid) = fs::read_to_string(path)
            .and_then(|value| value.trim().parse::<u32>().map_err(std::io::Error::other))
        {
            if let Some(token) = process_start_token(pid) {
                return (pid, token);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("live process metadata unavailable: {}", path.display());
}

fn request_dashboard_auth_status(port: u64) {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port as u16)).expect("installed dashboard HTTP listener");
    stream
        .write_all(
            format!(
                "GET /api/dashboard-auth/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .expect("installed dashboard auth status request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("installed dashboard auth status response");
    assert!(
        response.starts_with(b"HTTP/1.1 200"),
        "installed dashboard auth status failed: {}",
        String::from_utf8_lossy(&response)
    );
}

struct InstalledRuntimeGuard<'a> {
    fixture: &'a TempWorkstation,
    armed: bool,
}

impl InstalledRuntimeGuard<'_> {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for InstalledRuntimeGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = run_installed(self.fixture, &["shutdown", "--json"]);
        }
    }
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

#[test]
fn fresh_cold_install_starts_one_runtime_host_and_dashboard_for_service_use() {
    let fixture = TempWorkstation::new();
    let docker_log = fixture.root.join("docker.log");
    let systemctl_log = fixture.root.join("systemctl.log");
    fixture.install_fake_command("docker", "AGENT_BROWSER_FAKE_DOCKER_LOG");
    fixture.install_fake_command("systemctl", "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG");

    let install = run_apply(&fixture, &docker_log, &systemctl_log, None);
    let install_receipt = successful_json(&install, "cold install");
    assert_eq!(install_receipt["success"], true);
    let installed_binary = fixture.root.join("workstation/.local/bin/agent-browser");
    assert!(installed_binary.is_file(), "selected binary is unavailable");
    let mut runtime_guard = InstalledRuntimeGuard {
        fixture: &fixture,
        armed: true,
    };

    let initial_stream = successful_json(
        &run_installed(
            &fixture,
            &["--session", "bootstrap", "stream", "status", "--json"],
        ),
        "initial stream status",
    );
    let stream = if initial_stream["data"]["enabled"] == true {
        initial_stream
    } else {
        successful_json(
            &run_installed(
                &fixture,
                &["--session", "bootstrap", "stream", "enable", "--json"],
            ),
            "stream enable",
        )
    };
    let stream_port = stream["data"]["port"]
        .as_u64()
        .filter(|port| *port > 0)
        .expect("dashboard stream port");
    request_dashboard_auth_status(stream_port);

    let status = successful_json(
        &run_installed(
            &fixture,
            &["--json", "--session", "bootstrap", "service", "status"],
        ),
        "fresh service status",
    );
    assert_eq!(status["success"], true);
    assert_eq!(
        status["data"]["browserSessionState"]["schemaVersion"],
        "agent-browser.browser-session-state.v1"
    );
    let profile_catalog: serde_json::Value = serde_json::from_slice(
        &fs::read(
            fixture
                .root
                .join("home/.agent-browser/service/browser-profile-catalog.json"),
        )
        .expect("startup browser profile catalog"),
    )
    .expect("startup browser profile catalog JSON");
    assert_eq!(
        profile_catalog["schemaVersion"],
        "agent-browser.browser-profile-catalog.v1"
    );
    assert!(profile_catalog["profiles"]
        .as_object()
        .is_some_and(serde_json::Map::is_empty));
    assert_eq!(
        profile_catalog["disposablePolicies"]["default"]["id"],
        "default"
    );
    assert!(
        profile_catalog["disposablePolicies"]["default"]["userDataRoot"]
            .as_str()
            .is_some_and(|path| Path::new(path).is_absolute())
    );

    let socket_dir = fixture.root.join("runtime/sockets");
    let process_metadata = [socket_dir.join("runtime-host.pid")];
    let first_processes = process_metadata
        .iter()
        .map(|path| read_live_process_token(path))
        .collect::<Vec<_>>();
    let runtime_identity: serde_json::Value = serde_json::from_slice(
        &fs::read(socket_dir.join("runtime-host.identity.json"))
            .expect("runtime host process identity"),
    )
    .expect("runtime host process identity JSON");
    assert_eq!(
        fs::canonicalize(
            runtime_identity["executablePath"]
                .as_str()
                .expect("runtime host executable path")
        )
        .expect("runtime host executable canonical path"),
        fs::canonicalize(&installed_binary).expect("installed binary canonical path")
    );

    let repeated_stream = successful_json(
        &run_installed(
            &fixture,
            &["--session", "bootstrap", "stream", "status", "--json"],
        ),
        "repeated stream status",
    );
    assert_eq!(repeated_stream["data"]["enabled"], true);
    assert_eq!(repeated_stream["data"]["port"], stream_port);
    let repeated_status = successful_json(
        &run_installed(
            &fixture,
            &["--json", "--session", "bootstrap", "service", "status"],
        ),
        "repeated service status",
    );
    assert_eq!(repeated_status["success"], true);
    let repeated_processes = process_metadata
        .iter()
        .map(|path| read_live_process_token(path))
        .collect::<Vec<_>>();
    assert_eq!(repeated_processes, first_processes);

    let shutdown = successful_json(
        &run_installed(&fixture, &["shutdown", "--json"]),
        "installed shutdown",
    );
    assert_eq!(shutdown["success"], true);
    for ((pid, token), path) in first_processes.iter().zip(process_metadata.iter()) {
        for _ in 0..100 {
            if process_start_token(*pid).as_deref() != Some(token.as_str()) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert_ne!(
            process_start_token(*pid).as_deref(),
            Some(token.as_str()),
            "installed shutdown left process from {} running",
            path.display()
        );
        assert!(
            !path.exists(),
            "installed shutdown retained {}",
            path.display()
        );
    }
    runtime_guard.disarm();
}
