#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
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

fn run_managed_browser_command(
    fixture: &TempWorkstation,
    route_inventory: &str,
    args: &[&str],
) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-browser"))
        .args(args)
        .current_dir(&fixture.root)
        .env("HOME", fixture.root.join("home"))
        .env("XDG_RUNTIME_DIR", fixture.root.join("runtime"))
        .env(
            "AGENT_BROWSER_SOCKET_DIR",
            fixture.root.join("runtime/sockets"),
        )
        .env("AGENT_BROWSER_RUNTIME_HOST", "1")
        .env("AGENT_BROWSER_EXECUTABLE_PATH", "/usr/bin/google-chrome")
        .env("AGENT_BROWSER_RDP_ROUTE_POOL_JSON", route_inventory)
        .output()
        .expect("managed browser command");
    assert!(
        output.status.success(),
        "args={args:?} stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "managed browser response was not JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn managed_stream_port(fixture: &TempWorkstation, route_inventory: &str) -> u16 {
    let status = run_managed_browser_command(
        fixture,
        route_inventory,
        &["--session", "bootstrap", "stream", "status", "--json"],
    );
    let stream = if status["data"]["enabled"] == true {
        status
    } else {
        run_managed_browser_command(
            fixture,
            route_inventory,
            &["--session", "bootstrap", "stream", "enable", "--json"],
        )
    };
    stream["data"]["port"]
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
        .expect("dashboard stream port")
}

fn dashboard_bootstrap_credentials(fixture: &TempWorkstation) -> (String, String) {
    let path = fixture.root.join("home/.agent-browser/dashboard-auth.env");
    let text = (0..100)
        .find_map(|_| match fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(_) => {
                thread::sleep(Duration::from_millis(20));
                None
            }
        })
        .expect("dashboard bootstrap credentials");
    let value = |key: &str| {
        text.lines().find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key).then(|| value.trim().trim_matches('"').to_string())
        })
    };
    (
        value("AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME").unwrap_or_else(|| "admin".to_string()),
        value("AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD")
            .expect("dashboard admin bootstrap password"),
    )
}

fn request_dashboard_auth_status(port: u16) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("dashboard HTTP listener");
    stream
        .write_all(
            format!(
                "GET /api/dashboard-auth/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .expect("dashboard auth status request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("dashboard auth status response");
    assert!(
        response.starts_with(b"HTTP/1.1 200"),
        "dashboard auth status response was not successful: {}",
        String::from_utf8_lossy(&response)
    );
}

struct XvfbFixture {
    child: Child,
    display_name: String,
}

impl XvfbFixture {
    fn spawn() -> Self {
        let display_number = 2_000 + std::process::id() % 20_000;
        let display_name = format!(":{display_number}");
        let socket = PathBuf::from(format!("/tmp/.X11-unix/X{display_number}"));
        let socket_ready = || {
            socket.exists()
                || fs::read_to_string("/proc/net/unix").is_ok_and(|sockets| {
                    sockets
                        .lines()
                        .any(|line| line.contains(&format!("@{}", socket.display())))
                })
        };
        assert!(!socket_ready(), "fixture display socket already exists");
        let mut child = Command::new("/usr/bin/Xvfb")
            .args([
                display_name.as_str(),
                "-screen",
                "0",
                "1280x720x24",
                "-nolisten",
                "tcp",
                "-ac",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Xvfb fixture");
        for _ in 0..100 {
            if socket_ready() {
                return Self {
                    child,
                    display_name,
                };
            }
            if child.try_wait().expect("Xvfb status").is_some() {
                panic!("Xvfb fixture exited before its display became ready");
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!("Xvfb fixture did not become ready");
    }
}

impl Drop for XvfbFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn linux_process_start_token(pid: u32) -> Option<String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(") ")?.1;
    let start_ticks = after_name.split_whitespace().nth(19)?;
    let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    Some(format!("linux:{}:{start_ticks}", boot_id.trim()))
}

struct LocalAuthServer {
    base_url: String,
    stopping: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl LocalAuthServer {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("local auth listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking local auth listener");
        let address = listener.local_addr().expect("local auth address");
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = stopping.clone();
        let worker = thread::spawn(move || {
            while !worker_stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_millis(100)))
                            .expect("local auth client read timeout");
                        let mut request = [0_u8; 8_192];
                        let length = stream.read(&mut request).unwrap_or_default();
                        let request = String::from_utf8_lossy(&request[..length]);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("/");
                        let response = match path {
                            "/protected" => concat!(
                                "HTTP/1.1 302 Found\r\n",
                                "Location: /login\r\n",
                                "Content-Length: 0\r\n",
                                "Connection: close\r\n\r\n"
                            )
                            .to_string(),
                            "/login" => http_html_response(
                                "<title>Login</title><button onclick=\"document.title='alice-clicked'\">Alice</button>",
                            ),
                            "/account" => {
                                http_html_response("<title>Account</title><p>Signed in</p>")
                            }
                            _ => http_html_response("<title>Not found</title>"),
                        };
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.flush();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            base_url: format!("http://{address}"),
            stopping,
            worker: Some(worker),
        }
    }
}

impl Drop for LocalAuthServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn local_auth_server_drop_does_not_wait_for_an_idle_client() {
    let server = LocalAuthServer::spawn();
    let address = server
        .base_url
        .strip_prefix("http://")
        .expect("local auth server origin");
    let _idle_client = TcpStream::connect(address).expect("idle local auth client");
    thread::sleep(Duration::from_millis(50));

    let (dropped_tx, dropped_rx) = mpsc::channel();
    thread::spawn(move || {
        drop(server);
        let _ = dropped_tx.send(());
    });

    assert!(
        dropped_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "local auth server teardown blocked on an idle accepted client"
    );
}

fn http_html_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

struct ManagedProcessFixture {
    child: Child,
}

impl ManagedProcessFixture {
    fn spawn() -> Self {
        Self {
            child: Command::new("/bin/sleep")
                .arg("300")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("managed browser fixture process"),
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn process_identity(&self) -> serde_json::Value {
        let stat =
            fs::read_to_string(format!("/proc/{}/stat", self.pid())).expect("managed process stat");
        let after_name = stat.rsplit_once(") ").expect("process stat name").1;
        let start_ticks = after_name
            .split_whitespace()
            .nth(19)
            .expect("process start token");
        let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id").expect("Linux boot ID");
        serde_json::json!({
            "pid": self.pid(),
            "startToken": format!("linux:{}:{start_ticks}", boot_id.trim()),
            "executablePath": fs::canonicalize(format!("/proc/{}/exe", self.pid()))
                .expect("managed process executable"),
        })
    }

    fn is_running(&mut self) -> bool {
        self.child
            .try_wait()
            .expect("managed process status")
            .is_none()
    }
}

impl Drop for ManagedProcessFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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
        "stdout={} stderr={}",
        String::from_utf8_lossy(&first.stdout),
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

#[test]
fn shutdown_stops_exact_manager_owned_browser_and_terminalizes_manager_state() {
    let fixture = TempWorkstation::new();
    let docker_log = fixture.root.join("docker.log");
    let systemctl_log = fixture.root.join("systemctl.log");
    fixture.install_fake_command("docker", "AGENT_BROWSER_FAKE_DOCKER_LOG");
    fixture.install_fake_command("systemctl", "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG");

    let profile_dir = fixture.root.join("profiles/manager-owned");
    fs::create_dir_all(&profile_dir).expect("profile directory");
    let marker_path = profile_dir.join("retained-profile-data");
    fs::write(&marker_path, b"keep").expect("profile marker");

    let mut managed_process = ManagedProcessFixture::spawn();
    let mut foreign_process = ManagedProcessFixture::spawn();
    let pid = managed_process.pid();
    let state_path = fixture
        .root
        .join("home/.agent-browser/service/browser-session-state.json");
    fs::create_dir_all(state_path.parent().expect("service directory")).expect("service directory");
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.browser-session-state.v1",
            "browsers": {
                "browser:named:1": {
                    "id": "browser:named:1",
                    "profileId": "named",
                    "pid": pid,
                    "cdpEndpoint": "ws://127.0.0.1:1/devtools/browser/fixture",
                    "processIdentity": managed_process.process_identity(),
                    "activeSessionIds": ["session:alice:named:1"]
                }
            },
            "sessions": {
                "session:alice:named:1": {
                    "id": "session:alice:named:1",
                    "name": "alice",
                    "profileId": "named",
                    "browserId": "browser:named:1",
                    "createdAtMs": 1,
                    "lastActivityAtMs": 1,
                    "expiresAtMs": 60001,
                    "currentTabId": "tab:fixture"
                }
            },
            "tabs": {
                "tab:fixture": {
                    "id": "tab:fixture",
                    "targetId": "target:fixture",
                    "browserId": "browser:named:1",
                    "sessionId": "session:alice:named:1",
                    "createdAtMs": 1,
                    "lastActivityAtMs": 1
                }
            }
        }))
        .expect("manager state JSON"),
    )
    .expect("manager state");

    let first = run_shutdown(&fixture, &docker_log, &systemctl_log);
    assert!(
        first.status.success(),
        "manager shutdown stdout={} stderr={}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("shutdown receipt");
    assert_eq!(receipt["success"], true);
    assert_eq!(receipt["residue"]["ownedBrowsers"], 0);
    assert!(
        !managed_process.is_running(),
        "manager-owned process survived"
    );
    assert!(
        foreign_process.is_running(),
        "shutdown terminated an unreferenced foreign process"
    );

    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("persisted manager state"))
            .expect("persisted manager state JSON");
    assert_eq!(persisted["browsers"], serde_json::json!({}));
    assert_eq!(persisted["sessions"], serde_json::json!({}));
    assert_eq!(persisted["tabs"], serde_json::json!({}));
    assert_eq!(
        persisted["sessionHistory"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(fs::read(&marker_path).expect("profile marker"), b"keep");

    let replay = run_shutdown(&fixture, &docker_log, &systemctl_log);
    assert!(replay.status.success());
    let replay_receipt: serde_json::Value =
        serde_json::from_slice(&replay.stdout).expect("replay receipt");
    assert_eq!(replay_receipt["success"], true);
    assert_eq!(replay_receipt["changed"], false);
}

#[test]
#[ignore = "launches a disposable local Chrome and Xvfb"]
fn e2e_named_sessions_share_one_named_profile_browser() {
    assert!(Path::new("/usr/bin/google-chrome").is_file());
    assert!(Path::new("/usr/bin/Xvfb").is_file());

    let fixture = TempWorkstation::new();
    let xvfb = XvfbFixture::spawn();
    let auth_server = LocalAuthServer::spawn();
    let route_inventory = serde_json::json!([{
        "id": "slot-fixture",
        "target": {"displayName": xvfb.display_name}
    }])
    .to_string();
    let stream_port = managed_stream_port(&fixture, &route_inventory);
    let dashboard_origin = format!("http://127.0.0.1:{stream_port}");
    let viewer_url = format!("{}/viewer", auth_server.base_url);
    let profile_dir = fixture.root.join("profiles/work");
    let dashboard_profile_dir = fixture.root.join("profiles/dashboard");
    fs::create_dir_all(&profile_dir).expect("named profile directory");
    fs::create_dir_all(&dashboard_profile_dir).expect("dashboard profile directory");
    let service_dir = fixture.root.join("home/.agent-browser/service");
    fs::create_dir_all(&service_dir).expect("service directory");
    fs::write(
        service_dir.join("browser-profile-catalog.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.browser-profile-catalog.v1",
            "profiles": {
                "work": {
                    "id": "work",
                    "name": "Work",
                    "userDataDir": profile_dir,
                    "kind": "named"
                },
                "dashboard": {
                    "id": "dashboard",
                    "name": "Dashboard",
                    "userDataDir": dashboard_profile_dir,
                    "kind": "named"
                }
            }
        }))
        .expect("profile catalog JSON"),
    )
    .expect("profile catalog");
    fs::write(
        service_dir.join("state.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "agent-browser.service-state.v2",
            "displayAllocations": {
                "display-fixture": {
                    "id": "display-fixture",
                    "displayName": xvfb.display_name,
                    "state": "ready",
                    "routeIds": ["route-fixture"],
                    "readiness": {"state": "ready"}
                }
            },
            "routePool": {
                "slot-fixture": {
                    "id": "slot-fixture",
                    "routeId": "route-fixture",
                    "state": "ready"
                }
            },
            "remoteViewRoutes": {
                "route-fixture": {
                    "id": "route-fixture",
                    "provider": "rdp_gateway",
                    "displayAllocationId": "display-fixture",
                    "frameUrl": viewer_url,
                    "externalUrl": viewer_url,
                    "routeDescriptor": {
                        "publicOperatorUrl": dashboard_origin,
                        "localEmbedUrl": viewer_url,
                        "dashboardEmbedUrl": viewer_url
                    },
                    "controlInput": "manual_attached_desktop",
                    "state": "ready",
                    "readiness": {"state": "ready"}
                }
            }
        }))
        .expect("service state JSON"),
    )
    .expect("service state");
    let alice = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "alice",
            "--runtime-profile",
            "work",
            "open",
            &format!("{}/protected", auth_server.base_url),
            "--json",
        ],
    );
    let bob = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "bob",
            "--runtime-profile",
            "work",
            "open",
            "data:text/html,<button onclick=\"document.title='bob-clicked'\">Bob</button>",
            "--json",
        ],
    );
    assert_eq!(alice["data"]["browserId"], bob["data"]["browserId"]);
    assert_ne!(alice["data"]["sessionId"], bob["data"]["sessionId"]);
    assert_ne!(alice["data"]["tabId"], bob["data"]["tabId"]);
    assert_ne!(alice["data"]["targetId"], bob["data"]["targetId"]);
    for response in [&alice, &bob] {
        assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
        assert!(response["data"]["handoffUrl"]
            .as_str()
            .is_some_and(|url| url.starts_with(&format!("{dashboard_origin}/remote-view/"))));
    }
    let alice_login_url = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "alice", "get", "url", "--json"],
    );
    assert_eq!(
        alice_login_url["data"]["url"],
        format!("{}/login", auth_server.base_url)
    );

    let state_path = service_dir.join("browser-session-state.json");
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("browser session state after opens"))
            .expect("browser session state JSON");
    assert_eq!(
        state["browsers"].as_object().map(|items| items.len()),
        Some(1)
    );
    assert_eq!(
        state["sessions"].as_object().map(|items| items.len()),
        Some(2)
    );
    assert_eq!(state["tabs"].as_object().map(|items| items.len()), Some(2));
    let browser = state["browsers"]
        .as_object()
        .and_then(|items| items.values().next())
        .expect("shared browser");
    let browser_pid = browser["pid"].as_u64().expect("browser pid") as u32;
    let browser_start_token = browser["processIdentity"]["startToken"]
        .as_str()
        .expect("browser start token")
        .to_string();

    for session in ["alice", "bob"] {
        let snapshot = run_managed_browser_command(
            &fixture,
            &route_inventory,
            &["--session", session, "snapshot", "--json"],
        );
        assert!(snapshot["data"]["snapshot"]
            .as_str()
            .is_some_and(|value| value.contains("ref=e1")));
    }
    let alice_click = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "alice", "click", "e1", "--json"],
    );
    assert_eq!(alice_click["success"], true);
    let alice_title = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "alice", "get", "title", "--json"],
    );
    let bob_title = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "bob", "get", "title", "--json"],
    );
    assert_eq!(alice_title["data"]["title"], "alice-clicked");
    assert_ne!(bob_title["data"]["title"], "alice-clicked");
    let bob_click = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "bob", "click", "e1", "--json"],
    );
    assert_eq!(bob_click["success"], true);
    let bob_clicked_title = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "bob", "get", "title", "--json"],
    );
    assert_eq!(bob_clicked_title["data"]["title"], "bob-clicked");

    let alice_account = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "alice",
            "--runtime-profile",
            "work",
            "open",
            &format!("{}/account", auth_server.base_url),
            "--json",
        ],
    );
    assert_eq!(
        alice_account["data"]["browserId"],
        alice["data"]["browserId"]
    );
    assert_eq!(
        alice_account["data"]["sessionId"],
        alice["data"]["sessionId"]
    );
    assert_eq!(alice_account["data"]["tabId"], alice["data"]["tabId"]);
    assert_eq!(alice_account["data"]["targetId"], alice["data"]["targetId"]);
    assert_eq!(
        alice_account["data"]["handoffId"],
        alice["data"]["handoffId"]
    );
    let alice_account_title = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "alice", "get", "title", "--json"],
    );
    assert_eq!(alice_account_title["data"]["title"], "Account");
    let bob_after_redirect = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "bob", "get", "title", "--json"],
    );
    assert_eq!(bob_after_redirect["data"]["title"], "bob-clicked");

    let alice_handoff_url = alice["data"]["handoffUrl"]
        .as_str()
        .expect("Alice opaque dashboard handoff")
        .to_string();
    assert!(alice_handoff_url.starts_with(&format!("{dashboard_origin}/remote-view/")));
    let operator_open = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "operator",
            "--runtime-profile",
            "dashboard",
            "open",
            &alice_handoff_url,
            "--json",
        ],
    );
    assert_eq!(operator_open["success"], true);
    request_dashboard_auth_status(stream_port);
    let (dashboard_username, dashboard_password) = dashboard_bootstrap_credentials(&fixture);
    let login_script = format!(
        r#"(async () => {{
            const response = await fetch('/api/dashboard-auth/login', {{
                method: 'POST',
                credentials: 'same-origin',
                headers: {{'Content-Type': 'application/json'}},
                body: JSON.stringify({{username: {}, password: {}}}),
            }});
            const payload = await response.json().catch(() => ({{}}));
            return {{ok: response.ok, authenticated: payload.authenticated === true}};
        }})()"#,
        serde_json::to_string(&dashboard_username).unwrap(),
        serde_json::to_string(&dashboard_password).unwrap(),
    );
    let login = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "operator", "eval", &login_script, "--json"],
    );
    assert_eq!(login["data"]["result"]["ok"], true);
    assert_eq!(login["data"]["result"]["authenticated"], true);
    let authenticated_open = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "operator",
            "--runtime-profile",
            "dashboard",
            "open",
            &alice_handoff_url,
            "--json",
        ],
    );
    assert_eq!(authenticated_open["success"], true);
    let dashboard_state_script = r#"(() => {
        const viewport = document.querySelector('[aria-label="Workspace remote viewport"]');
        const frame = viewport?.querySelector('iframe');
        const body = document.body?.innerText || '';
        return {
            url: location.href,
            readiness: viewport?.getAttribute('data-readiness-status') || null,
            frameSrc: frame?.getAttribute('src') || null,
            controlMode: body.includes('Workspace viewport / control'),
            converging: body.includes('Restoring remote view'),
            viewOnly: body.includes('viewport is view-only'),
        };
    })()"#;
    let mut dashboard_state = serde_json::Value::Null;
    for _ in 0..120 {
        let evaluated = run_managed_browser_command(
            &fixture,
            &route_inventory,
            &[
                "--session",
                "operator",
                "eval",
                dashboard_state_script,
                "--json",
            ],
        );
        dashboard_state = evaluated["data"]["result"].clone();
        if dashboard_state["readiness"] == "ready"
            && dashboard_state["frameSrc"] == viewer_url
            && dashboard_state["controlMode"] == true
        {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    assert_eq!(dashboard_state["readiness"], "ready", "{dashboard_state}");
    assert_eq!(dashboard_state["frameSrc"], viewer_url, "{dashboard_state}");
    assert_eq!(dashboard_state["controlMode"], true, "{dashboard_state}");
    assert_eq!(dashboard_state["converging"], false, "{dashboard_state}");
    assert_eq!(dashboard_state["viewOnly"], false, "{dashboard_state}");
    let service_after_dashboard: serde_json::Value = serde_json::from_slice(
        &fs::read(service_dir.join("state.json")).expect("service state after dashboard resolve"),
    )
    .expect("service state after dashboard resolve JSON");
    assert!(service_after_dashboard["remoteViewRoutes"]["route-fixture"]
        .get("controllerLeaseId")
        .is_none_or(serde_json::Value::is_null));
    let operator_close = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "operator",
            "--runtime-profile",
            "dashboard",
            "close",
            "--json",
        ],
    );
    assert_eq!(operator_close["data"]["disposition"], "browser_closed");

    let alice_close = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "alice",
            "--runtime-profile",
            "work",
            "close",
            "--json",
        ],
    );
    assert_eq!(alice_close["data"]["disposition"], "browser_preserved");
    let bob_after_alice_close = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &["--session", "bob", "get", "title", "--json"],
    );
    assert_eq!(bob_after_alice_close["data"]["title"], "bob-clicked");

    let bob_close = run_managed_browser_command(
        &fixture,
        &route_inventory,
        &[
            "--session",
            "bob",
            "--runtime-profile",
            "work",
            "close",
            "--json",
        ],
    );
    assert_eq!(bob_close["data"]["disposition"], "browser_closed");
    let terminal_state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).expect("terminal browser session state"))
            .expect("terminal browser session state JSON");
    assert_eq!(terminal_state["browsers"], serde_json::json!({}));
    assert_eq!(terminal_state["sessions"], serde_json::json!({}));
    assert_eq!(terminal_state["tabs"], serde_json::json!({}));
    for _ in 0..100 {
        if linux_process_start_token(browser_pid).as_deref() != Some(browser_start_token.as_str()) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_ne!(
        linux_process_start_token(browser_pid).as_deref(),
        Some(browser_start_token.as_str()),
        "final session close did not terminate the exact browser process"
    );

    let docker_log = fixture.root.join("docker.log");
    let systemctl_log = fixture.root.join("systemctl.log");
    fixture.install_fake_command("docker", "AGENT_BROWSER_FAKE_DOCKER_LOG");
    fixture.install_fake_command("systemctl", "AGENT_BROWSER_FAKE_SYSTEMCTL_LOG");
    let cleanup = run_shutdown(&fixture, &docker_log, &systemctl_log);
    assert!(
        cleanup.status.success(),
        "cleanup stdout={} stderr={}",
        String::from_utf8_lossy(&cleanup.stdout),
        String::from_utf8_lossy(&cleanup.stderr)
    );
}
