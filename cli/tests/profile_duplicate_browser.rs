#![cfg(target_os = "linux")]
//! Actual Chrome acceptance for the private copy/custody seam. Public adapters
//! and native handoff delivery remain separate acceptance requirements.
#[path = "../src/native/profile_duplicate_copy.rs"]
#[allow(dead_code)]
mod profile_duplicate_copy;
#[path = "../src/native/profile_duplicate_custody.rs"]
#[allow(dead_code)]
mod profile_duplicate_custody;
use futures_util::{SinkExt, StreamExt};
use profile_duplicate_copy as copy;
use profile_duplicate_custody as custody;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
struct Chrome {
    child: Child,
    websocket: String,
    socket: tokio::sync::Mutex<Option<Socket>>,
}
impl Chrome {
    fn launch(executable: &Path, profile: &Path) -> Self {
        use std::os::unix::process::CommandExt;
        let log = fs::File::create(profile.parent().unwrap().join(format!(
            "chrome-{}.stderr",
            profile.file_name().unwrap().to_string_lossy()
        )))
        .unwrap();
        let child = Command::new(executable)
            .args([
                "--headless",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--disable-gpu",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-sync",
                "--no-first-run",
                "--no-default-browser-check",
                "--remote-debugging-port=0",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .process_group(0)
            .spawn()
            .unwrap();
        let mut chrome = Self {
            child,
            websocket: String::new(),
            socket: tokio::sync::Mutex::new(None),
        };
        let start = Instant::now();
        loop {
            assert!(
                chrome.child.try_wait().unwrap().is_none(),
                "fixture Chrome exited before endpoint readiness"
            );
            if let Ok(port) = fs::read_to_string(profile.join("DevToolsActivePort")) {
                let mut lines = port.lines();
                if let (Some(port), Some(path)) = (lines.next(), lines.next()) {
                    chrome.websocket = format!("ws://127.0.0.1:{port}{path}");
                    break;
                }
            }
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "fixture Chrome endpoint unavailable"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        chrome
    }
    async fn command(&self, method: &str, params: Value) -> Value {
        self.command_session(method, params, None).await
    }
    async fn command_session(&self, method: &str, params: Value, session: Option<&str>) -> Value {
        let mut connection = self.socket.lock().await;
        if connection.is_none() {
            *connection = Some(connect_async(&self.websocket).await.unwrap().0);
        }
        let socket = connection.as_mut().unwrap();
        let mut command = json!({"id":1,"method":method,"params":params});
        if let Some(session) = session {
            command["sessionId"] = json!(session);
        }
        socket
            .send(Message::Text(command.to_string().into()))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(message) = socket.next().await {
                let message = message.unwrap();
                if let Ok(text) = message.to_text() {
                    let value: Value = serde_json::from_str(text).unwrap();
                    if value["id"] == 1 {
                        assert!(
                            value.get("error").is_none(),
                            "CDP fixture command failed: {}",
                            value["error"]
                        );
                        return value["result"].clone();
                    }
                }
            }
            panic!("fixture command reply lost")
        })
        .await
        .unwrap_or_else(|_| panic!("fixture command timed out: {method}"))
    }
    async fn close(&mut self) {
        self.command("Browser.close", json!({})).await;
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "fixture Chrome failed normal shutdown");
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "fixture browser did not close gracefully"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(!Path::new(&format!("/proc/{}", self.child.id())).exists());
    }
}
impl Drop for Chrome {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            // Failure cleanup is confined to this owned child's process group;
            // it is never accepted as a clean-close proof for copying.
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}
struct Server {
    port: u16,
    seen: Arc<Mutex<Vec<(String, String)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let data = seen.clone();
        let done = stop.clone();
        let thread = std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                        let mut buffer = [0; 8192];
                        let Ok(length) = stream.read(&mut buffer) else {
                            continue;
                        };
                        if length == 0 {
                            continue;
                        }
                        let request = String::from_utf8_lossy(&buffer[..length]);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("")
                            .to_owned();
                        let cookie = request
                            .lines()
                            .find_map(|line| {
                                line.split_once(':')
                                    .filter(|(key, _)| key.eq_ignore_ascii_case("cookie"))
                                    .map(|(_, value)| value.trim().to_owned())
                            })
                            .unwrap_or_default();
                        data.lock().unwrap().push((path.clone(), cookie));
                        let set = if path.starts_with("/set-source") {
                            "Set-Cookie: synthetic_auth=source-only; Max-Age=3600; Path=/; HttpOnly\r\n"
                        } else if path.starts_with("/set-initial") {
                            "Set-Cookie: synthetic_auth=initial-authenticated; Max-Age=3600; Path=/; HttpOnly\r\n"
                        } else {
                            ""
                        };
                        let body = "<!doctype html><title>synthetic profile acceptance</title>";
                        let response=format!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html\r\nContent-Length: {}\r\n{set}\r\n{body}",body.len());
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            port,
            seen,
            stop,
            thread: Some(thread),
        }
    }
    async fn visit(&self, chrome: &Chrome, path: &str) -> String {
        let target = chrome
            .command("Target.createTarget", json!({"url":"about:blank"}))
            .await;
        let session = chrome
            .command(
                "Target.attachToTarget",
                json!({"targetId":target["targetId"],"flatten":true}),
            )
            .await;
        let session = session["sessionId"].as_str().unwrap();
        chrome
            .command_session("Page.enable", json!({}), Some(session))
            .await;
        chrome
            .command_session("Runtime.enable", json!({}), Some(session))
            .await;
        chrome
            .command_session("Runtime.runIfWaitingForDebugger", json!({}), Some(session))
            .await;
        chrome
            .command_session("Network.enable", json!({}), Some(session))
            .await;
        let navigation = chrome
            .command_session(
                "Page.navigate",
                json!({"url":format!("http://127.0.0.1:{}{path}",self.port)}),
                Some(session),
            )
            .await;
        assert!(
            navigation.get("errorText").is_none(),
            "synthetic navigation failed: {}",
            navigation["errorText"]
        );
        let start = Instant::now();
        loop {
            if let Some((_, cookie)) = self
                .seen
                .lock()
                .unwrap()
                .iter()
                .find(|(request, _)| request == path)
            {
                return cookie.clone();
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "synthetic HTTP request not observed: {path}; paths={:?}",
                self.seen
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(path, _)| path.clone())
                    .collect::<Vec<_>>()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
struct Runtime {
    database: PathBuf,
    evidence: custody::BackendGracefulCloseEvidence,
    executable: PathBuf,
    guard: Option<Arc<Mutex<custody::SourceCustody>>>,
    duplicate: Option<Chrome>,
}
impl copy::DuplicationRuntime for Runtime {
    type Guard = Arc<Mutex<custody::SourceCustody>>;
    fn inspect_source(
        &mut self,
        source: &copy::SourceProfile,
    ) -> Result<copy::SourceObservation, String> {
        Ok(copy::SourceObservation {
            generation: source.generation,
            running: Path::new(&format!("/proc/{}", self.evidence.original.pid)).exists(),
            affected_task_ids: vec![],
        })
    }
    fn acquire_stopped_source(
        &mut self,
        source: &copy::SourceProfile,
    ) -> Result<Self::Guard, String> {
        assert!(
            source
                .user_data_dir
                .join("SingletonLock")
                .symlink_metadata()
                .is_err(),
            "source must release actual Chrome profile lock"
        );
        let guard = Arc::new(Mutex::new(custody::SourceCustody::acquire(
            &self.database,
            source,
            &self.evidence,
            |pid| {
                if Path::new(&format!("/proc/{pid}")).exists() {
                    custody::OriginalProcessState::Live
                } else {
                    custody::OriginalProcessState::Absent
                }
            },
        )?));
        self.guard = Some(guard.clone());
        Ok(guard)
    }
    fn focus_source(&mut self, _: &copy::SourceProfile) -> Result<(), String> {
        panic!("not requested")
    }
    fn graceful_close_source(
        &mut self,
        _: &copy::SourceProfile,
        _: &str,
    ) -> Result<copy::CloseOutcome, String> {
        panic!("source already closed through real Browser.close")
    }
    fn publish_profile(&mut self, profile: &copy::DuplicateProfile) -> Result<(), String> {
        self.guard
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .publish_and_release(profile)
    }
    fn launch_duplicate(
        &mut self,
        profile: &copy::DuplicateProfile,
    ) -> Result<copy::LaunchReceipt, String> {
        self.duplicate = Some(Chrome::launch(
            &self.executable,
            &profile.destination.user_data_dir,
        ));
        Ok(copy::LaunchReceipt {
            task_id: uuid::Uuid::new_v4().to_string(),
            browser_id: uuid::Uuid::new_v4().to_string(),
            handoff_id: uuid::Uuid::new_v4().to_string(),
        })
    }
    fn reconcile_launch(
        &mut self,
        _: &copy::DuplicateProfile,
    ) -> Result<Option<copy::LaunchReceipt>, String> {
        Ok(None)
    }
}
fn kernel_identity(
    pid: u32,
    executable: &Path,
) -> agent_browser_service_model::RecordedProcessIdentity {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let fields = stat
        .rsplit_once(')')
        .unwrap()
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap();
    agent_browser_service_model::RecordedProcessIdentity {
        pid,
        start_token: format!("{}:{}", boot.trim(), fields[19]),
        executable_path: Some(
            fs::canonicalize(executable)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ),
        browser_family: Some("chrome".into()),
    }
}
fn runtime_database(
    path: &Path,
    source: &copy::SourceProfile,
    original: agent_browser_service_model::RecordedProcessIdentity,
) {
    let db = Connection::open(path).unwrap();
    db.execute_batch("CREATE TABLE state_documents(kind TEXT PRIMARY KEY,schema_version TEXT NOT NULL,json TEXT NOT NULL);PRAGMA user_version=2;").unwrap();
    let assignment: Value = serde_json::from_str(include_str!(
        "../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
    ))
    .unwrap();
    let mut record = agent_browser_service_model::BrowserLaunchCustodyRecord::pending(
        agent_browser_service_model::BrowserLaunchIntent {
            intent_id: uuid::Uuid::new_v4().to_string(),
            profile_id: source.id.clone(),
            assignment: serde_json::from_value(assignment["assignment"].clone()).unwrap(),
        },
    )
    .unwrap();
    record.observed_browser_id = Some("source-browser".into());
    record.observed_pid = Some(original.pid);
    record.observed_process_identity = Some(original);
    record.published = true;
    for (kind, schema, value) in [
        (
            "browser_profile_catalog",
            "agent-browser.browser-profile-catalog.v1",
            json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{source.id.clone():{"id":source.id,"name":source.name,"userDataDir":source.user_data_dir,"kind":"named"}},"disposablePolicies":{}}),
        ),
        (
            "browser_session_state",
            "agent-browser.browser-session-state.v1",
            serde_json::to_value(agent_browser_service_model::BrowserSessionState::default())
                .unwrap(),
        ),
        (
            "browser_launch_custody",
            "agent-browser.browser-launch-custody.v1",
            json!({"records":{record.intent.intent_id.clone():record},"reconciled_absent":[]}),
        ),
    ] {
        db.execute(
            "INSERT INTO state_documents VALUES(?1,?2,?3)",
            params![kind, schema, value.to_string()],
        )
        .unwrap();
    }
}
#[tokio::test]
#[ignore = "launches isolated real Chrome profiles; no production or private-account effects"]
async fn actual_chrome_cookie_copy_keeps_concurrent_profiles_independent() {
    use sha2::{Digest, Sha256};
    let executable = std::env::var_os("AGENT_BROWSER_TEST_CHROME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/google/chrome/chrome"));
    assert!(
        executable.is_file(),
        "pin an installed Chrome executable for the fixture"
    );
    let build = format!(
        "sha256:{:x}",
        Sha256::digest(fs::read(&executable).unwrap())
    );
    let root = std::env::temp_dir().join(format!(
        "agent-browser-profile-copy-real-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir(&root).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let server = Server::start();
    let source = copy::SourceProfile {
        id: "source".into(),
        name: "Synthetic source".into(),
        user_data_dir: root.join("source"),
        browser_build: build.clone(),
        generation: 1,
    };
    let mut original = Chrome::launch(&executable, &source.user_data_dir);
    let identity = kernel_identity(original.child.id(), &executable);
    server.visit(&original, "/set-initial").await;
    assert!(server
        .visit(&original, "/read-initial")
        .await
        .contains("synthetic_auth=initial-authenticated"));
    original.close().await;
    assert!(source
        .user_data_dir
        .join("SingletonLock")
        .symlink_metadata()
        .is_err());
    let database = root.join("runtime.sqlite3");
    runtime_database(&database, &source, identity.clone());
    let evidence = custody::BackendGracefulCloseEvidence {
        owner_operation_id: "fixture-browser-close-acknowledged".into(),
        source_profile_id: source.id.clone(),
        source_generation: 1,
        original: identity,
        polite_close_attempted: true,
        polite_close_succeeded: true,
        polite_close_failed: false,
        exact_process_exited: true,
        profile_lock_released: true,
        force_kill_attempted: false,
    };
    let mut runtime = Runtime {
        database,
        evidence,
        executable: executable.clone(),
        guard: None,
        duplicate: None,
    };
    let mut operations =
        copy::DuplicationStore::open(&root.join("duplicate-operations.sqlite3")).unwrap();
    operations
        .prepare(copy::DuplicateRequest {
            operation_id: "real-cookie-copy".into(),
            source: source.clone(),
            destination: copy::DestinationProfile {
                id: "duplicate".into(),
                name: "Synthetic duplicate".into(),
                user_data_dir: root.join("duplicate"),
            },
            launch: true,
        })
        .unwrap();
    let result = operations
        .advance("real-cookie-copy", copy::Choice::Proceed, &mut runtime)
        .unwrap();
    assert_eq!(result.phase, copy::Phase::Completed);
    assert_eq!(
        result.public_view()["authenticationContinuity"],
        "unverified",
        "copy receipt cannot assert authentication"
    );
    let mut duplicate = runtime.duplicate.take().unwrap();
    assert!(server
        .visit(&duplicate, "/read-duplicate")
        .await
        .contains("synthetic_auth=initial-authenticated"));
    let mut reopened = Chrome::launch(&executable, &source.user_data_dir);
    assert_ne!(reopened.child.id(), duplicate.child.id());
    assert!(server
        .visit(&reopened, "/read-source-preserved")
        .await
        .contains("synthetic_auth=initial-authenticated"));
    server.visit(&reopened, "/set-source").await;
    assert!(server
        .visit(&reopened, "/read-source-diverged")
        .await
        .contains("synthetic_auth=source-only"));
    assert!(server
        .visit(&duplicate, "/read-duplicate-independent")
        .await
        .contains("synthetic_auth=initial-authenticated"));
    assert!(reopened.child.try_wait().unwrap().is_none());
    assert!(duplicate.child.try_wait().unwrap().is_none());
    reopened.close().await;
    duplicate.close().await;
    let start = Instant::now();
    loop {
        let remaining = fs::read_dir("/proc")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| fs::read(entry.path().join("cmdline")).ok())
            .filter(|argv| {
                argv.windows(root.as_os_str().as_encoded_bytes().len())
                    .any(|part| part == root.as_os_str().as_encoded_bytes())
            })
            .count();
        if remaining == 0 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "fixture-owned browser processes remain after normal close"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        build,
        format!(
            "sha256:{:x}",
            Sha256::digest(fs::read(&executable).unwrap())
        )
    );
    drop(operations);
    drop(runtime);
    drop(server);
    fs::remove_dir_all(root).unwrap();
}
