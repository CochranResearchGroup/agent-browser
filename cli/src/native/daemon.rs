use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tokio::sync::{mpsc, Mutex, Notify, RwLock};

use super::action_runtime::DaemonState;
use super::cdp::client::CdpClient;
use super::control_plane::{ControlPlaneHandle, ControlPlaneWorker};
use super::state;
use super::stream::StreamServer;
use crate::connection::write_daemon_process_identity;
use crate::process_identity::capture_process_identity;

const DAEMON_AUTH_TOKEN_ENV: &str = "AGENT_BROWSER_DAEMON_AUTH_TOKEN";
const DAEMON_AUTH_FIELD: &str = "_agentBrowserAuthToken";

/// Build the runtime host on the same bounded stack used for Service State
/// serialization. Commands may own large parsed snapshots until dispatch
/// finishes, so both decoding and value destruction need this stack budget.
pub(crate) fn build_runtime(worker_threads: usize) -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_stack_size(super::service_store::SERVICE_STATE_JSON_STACK_BYTES)
        .enable_all()
        .build()
        .map_err(|error| format!("could not start daemon runtime: {error}"))
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UnixSocketIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn unix_socket_identity(path: &Path) -> Option<UnixSocketIdentity> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path).ok()?;
    Some(UnixSocketIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(unix)]
fn unix_socket_path_is_owned(path: &Path, identity: UnixSocketIdentity) -> bool {
    unix_socket_identity(path) == Some(identity)
}

fn secure_daemon_dir(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
}

fn secure_daemon_file(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

fn file_sha256(path: &Path) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub async fn run_daemon(session: &str) {
    let startup_started = Instant::now();
    let socket_dir = get_daemon_socket_dir();
    let endpoint_key = crate::runtime_host::endpoint_key(session).to_string();
    if !socket_dir.exists() {
        let _ = fs::create_dir_all(&socket_dir);
    }
    secure_daemon_dir(&socket_dir);

    let daemon_auth_token = match env::var(DAEMON_AUTH_TOKEN_ENV) {
        Ok(token) if !token.is_empty() => Arc::new(token),
        _ => {
            let _ = writeln!(
                std::io::stderr(),
                "Missing {} for daemon session authentication",
                DAEMON_AUTH_TOKEN_ENV
            );
            process::exit(1);
        }
    };

    // When debug mode is on, redirect stderr to a log file so daemon
    // output can be inspected (the daemon normally has stderr piped to its
    // parent which drops the read end after startup).
    #[cfg(unix)]
    if env::var("AGENT_BROWSER_DEBUG").is_ok() {
        let log_path = socket_dir.join(format!("{}.log", endpoint_key));
        if let Ok(file) = fs::File::create(&log_path) {
            use std::os::unix::io::IntoRawFd;
            let fd = file.into_raw_fd();
            unsafe {
                libc::dup2(fd, 2);
                libc::close(fd);
            }
            let _ = writeln!(
                std::io::stderr(),
                "[daemon] Debug logging started for session: {}",
                session
            );
            log_startup_milestone(startup_started, "debug-log-ready");
        }
    } else {
        // Redirect stderr to /dev/null to prevent daemon crash when the
        // parent CLI drops the piped stderr handle after startup.  Cloud
        // providers (AgentCore, Browserbase, etc.) may write to stderr
        // during connection setup; a broken pipe would kill the daemon.
        #[cfg(unix)]
        {
            use std::os::unix::io::IntoRawFd;
            if let Ok(devnull) = fs::File::create("/dev/null") {
                let fd = devnull.into_raw_fd();
                unsafe {
                    libc::dup2(fd, 2);
                    libc::close(fd);
                }
            }
        }
    }

    let pid_path = socket_dir.join(format!("{}.pid", endpoint_key));
    let _ = fs::write(&pid_path, process::id().to_string());
    secure_daemon_file(&pid_path);
    log_startup_milestone(startup_started, "pid-written");

    let daemon_executable = env::current_exe().ok();
    let daemon_identity = daemon_executable
        .as_deref()
        .and_then(|path| capture_process_identity(process::id(), Some(path), None));
    let Some(daemon_identity) = daemon_identity else {
        let _ = writeln!(
            std::io::stderr(),
            "Failed to capture daemon process identity"
        );
        process::exit(1);
    };
    if let Err(error) = write_daemon_process_identity(session, &daemon_identity) {
        let _ = writeln!(std::io::stderr(), "{error}");
        process::exit(1);
    }
    log_startup_milestone(startup_started, "process-identity-written");

    let version_path = socket_dir.join(format!("{}.version", endpoint_key));
    let _ = fs::write(&version_path, env!("CARGO_PKG_VERSION"));
    secure_daemon_file(&version_path);
    log_startup_milestone(startup_started, "version-written");

    let executable_sha_path = socket_dir.join(format!("{}.sha256", endpoint_key));
    let _ = fs::write(&executable_sha_path, "pending");
    secure_daemon_file(&executable_sha_path);
    log_startup_milestone(startup_started, "executable-sha-pending");

    // On Unix the daemon listens on a Unix domain socket; on Windows it uses
    // TCP, so there is no .sock file — only a .port file written by the server.
    let socket_path = socket_dir.join(format!("{}.sock", endpoint_key));

    #[cfg(unix)]
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    #[cfg(unix)]
    let socket_listener = match tokio::net::UnixListener::bind(&socket_path) {
        Ok(listener) => {
            secure_daemon_file(&socket_path);
            log_startup_milestone(startup_started, "socket-bound");
            write_executable_sha_in_background(executable_sha_path.clone(), startup_started);
            listener
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Failed to bind socket: {}", e);
            process::exit(1);
        }
    };
    #[cfg(unix)]
    let socket_identity = unix_socket_identity(&socket_path);

    #[cfg(windows)]
    {
        let _ = fs::remove_file(socket_dir.join(format!("{}.port", endpoint_key)));
    }

    let runtime_host_manifest_path = if crate::runtime_host::admission_enabled() {
        let executable_generation = env::current_exe()
            .map_err(std::io::Error::other)
            .and_then(|path| file_sha256(&path));
        match executable_generation.and_then(|generation| {
            crate::runtime_host::write_manifest(&socket_dir, generation)
                .map_err(std::io::Error::other)
        }) {
            Ok((path, manifest)) => Some((path, manifest)),
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "Failed to publish runtime host: {error}");
                #[cfg(unix)]
                let _ = fs::remove_file(&socket_path);
                process::exit(1);
            }
        }
    } else {
        None
    };

    let stream_path = socket_dir.join(format!("{}.stream", session));
    let _ = fs::remove_file(&stream_path);
    let _ = fs::remove_file(socket_dir.join(format!("{}.engine", session)));
    let _ = fs::remove_file(socket_dir.join(format!("{}.provider", session)));
    let _ = fs::remove_file(socket_dir.join(format!("{}.extensions", session)));

    if let Ok(days_str) = env::var("AGENT_BROWSER_STATE_EXPIRE_DAYS") {
        if let Ok(days) = days_str.parse::<u64>() {
            if days > 0 {
                let _ = state::state_clean(days);
            }
        }
    }

    let mut stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>> = None;
    let mut stream_server_instance: Option<Arc<StreamServer>> = None;
    let preferred_port = env::var("AGENT_BROWSER_STREAM_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let strict_stream_port = env::var("AGENT_BROWSER_STREAM_PORT_STRICT")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"));
    match StreamServer::start_without_client(
        preferred_port,
        session.to_string(),
        !strict_stream_port,
    )
    .await
    {
        Ok((stream_server, client_slot)) => {
            stream_client = Some(client_slot.clone());
            if let Err(e) = fs::write(&stream_path, stream_server.port().to_string()) {
                let _ = writeln!(std::io::stderr(), "Failed to write .stream file: {}", e);
            } else {
                secure_daemon_file(&stream_path);
            }
            stream_server_instance = Some(Arc::new(stream_server));
            log_startup_milestone(startup_started, "stream-server-ready");
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Stream server failed to start: {}", e);
            log_startup_milestone(startup_started, "stream-server-failed");
            if strict_stream_port {
                process::exit(1);
            }
        }
    }

    // Do not move stable ingress merely because the replacement control socket
    // exists. Publish it only after the host's stream surface is also ready.
    if let Some((_, manifest)) = runtime_host_manifest_path.as_ref() {
        let ingress_path =
            crate::runtime_host_ingress::RuntimeHostIngressRepository::default_path();
        if ingress_path.is_file() {
            let repository =
                crate::runtime_host_ingress::RuntimeHostIngressRepository::new(ingress_path);
            if let Err(error) = repository.adopt_current_process_replacement(
                socket_dir.clone(),
                manifest.executable_generation.clone(),
                manifest.host_id.clone(),
                manifest.socket_identity.clone(),
            ) {
                let _ = writeln!(
                    std::io::stderr(),
                    "Runtime host ingress reconciliation deferred: {error}"
                );
            }
        }
    }

    // Auto-shutdown the daemon after this many ms of inactivity (no commands received).
    // Disabled when unset or 0.
    let idle_timeout_ms = env::var("AGENT_BROWSER_IDLE_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&ms| ms > 0);
    let service_reconcile_interval_ms = env::var("AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&ms| ms > 0);
    let service_job_timeout_ms = env::var("AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&ms| ms > 0);
    let service_monitor_interval_ms = env::var("AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&ms| ms > 0);

    let result = run_socket_server(
        #[cfg(unix)]
        socket_listener,
        &socket_path,
        session,
        daemon_auth_token,
        stream_client,
        stream_server_instance,
        idle_timeout_ms,
        service_reconcile_interval_ms,
        service_job_timeout_ms,
        service_monitor_interval_ms,
    )
    .await;

    // A retiring executable-handoff daemon must not delete artifacts written
    // by its replacement after the shared session path has been rebound.
    #[cfg(unix)]
    let owns_session_artifacts =
        socket_identity.is_some_and(|identity| unix_socket_path_is_owned(&socket_path, identity));
    #[cfg(windows)]
    let owns_session_artifacts = true;

    #[cfg(unix)]
    if owns_session_artifacts {
        let _ = fs::remove_file(&socket_path);
    }
    #[cfg(windows)]
    {
        let _ = fs::remove_file(socket_dir.join(format!("{}.port", endpoint_key)));
    }
    if owns_session_artifacts {
        let _ = fs::remove_file(&pid_path);
        let _ = fs::remove_file(socket_dir.join(format!("{}.identity.json", endpoint_key)));
        let _ = fs::remove_file(&version_path);
        let _ = fs::remove_file(&executable_sha_path);
        let _ = fs::remove_file(&stream_path);
        let _ = fs::remove_file(socket_dir.join(format!("{}.engine", session)));
        let _ = fs::remove_file(socket_dir.join(format!("{}.provider", session)));
        let _ = fs::remove_file(socket_dir.join(format!("{}.extensions", session)));
        if let Some((path, _)) = runtime_host_manifest_path.as_ref() {
            crate::runtime_host::remove_manifest_if_owned(path);
        }
    }

    if let Err(e) = result {
        let _ = writeln!(std::io::stderr(), "Daemon error: {}", e);
        process::exit(1);
    }
}

fn log_startup_milestone(startup_started: Instant, label: &str) {
    if env::var("AGENT_BROWSER_DEBUG").is_ok() {
        let _ = writeln!(
            std::io::stderr(),
            "[daemon] startup {} at {}ms",
            label,
            startup_started.elapsed().as_millis()
        );
    }
}

fn write_executable_sha_in_background(executable_sha_path: PathBuf, startup_started: Instant) {
    tokio::task::spawn_blocking(move || {
        if let Ok(current_exe) = env::current_exe() {
            if let Ok(sha256) = file_sha256(&current_exe) {
                let _ = fs::write(&executable_sha_path, sha256);
                secure_daemon_file(&executable_sha_path);
                log_startup_milestone(startup_started, "executable-sha-written");
            }
        }
    });
}

#[derive(Clone)]
struct RuntimeLane {
    control_plane: ControlPlaneHandle,
    stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
    stream_server: Option<Arc<StreamServer>>,
    stream_file: Option<PathBuf>,
    config: crate::runtime_host::RuntimeLaneConfig,
    configuration_committed: bool,
}

#[derive(Clone)]
struct RuntimeHostRouter {
    lanes: Arc<crate::runtime_host::RuntimeLaneRegistry<RuntimeLane>>,
    creation_lock: Arc<Mutex<()>>,
    socket_dir: PathBuf,
    service_reconcile_interval_ms: Option<u64>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
}

#[derive(Clone, Copy)]
struct RuntimeHostWorkerOptions {
    service_reconcile_interval_ms: Option<u64>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
}

impl RuntimeHostRouter {
    fn new(
        socket_dir: PathBuf,
        initial_session: &str,
        stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
        stream_server: Option<Arc<StreamServer>>,
        stream_file: Option<PathBuf>,
        options: RuntimeHostWorkerOptions,
    ) -> Result<Self, String> {
        let initial_config = crate::session_supervisor::runtime_host_supervised_lane_configs()
            .ok()
            .and_then(|configs| {
                configs
                    .into_iter()
                    .find(|(session, _)| session == initial_session)
                    .map(|(_, config)| config)
            });
        let lanes = Arc::new(crate::runtime_host::RuntimeLaneRegistry::new(
            crate::runtime_host::DEFAULT_MAX_RUNTIME_LANES,
        ));
        let initial_state = match initial_config.as_ref() {
            Some(config) => DaemonState::new_for_runtime_lane_with_stream(
                initial_session,
                stream_client.clone(),
                stream_server.clone(),
                config,
            )?,
            None => DaemonState::new_for_session_with_stream(
                initial_session,
                stream_client.clone(),
                stream_server.clone(),
            ),
        };
        let control_plane = ControlPlaneWorker::start_with_options(
            initial_state,
            options.service_reconcile_interval_ms,
            options.service_job_timeout_ms,
            options.service_monitor_interval_ms,
        );
        lanes.insert(
            initial_session.to_string(),
            RuntimeLane {
                control_plane,
                stream_client,
                stream_server,
                stream_file,
                config: initial_config.clone().unwrap_or_default(),
                configuration_committed: initial_config.is_some(),
            },
        )?;
        Ok(Self {
            lanes,
            creation_lock: Arc::new(Mutex::new(())),
            socket_dir,
            service_reconcile_interval_ms: options.service_reconcile_interval_ms,
            service_job_timeout_ms: options.service_job_timeout_ms,
            service_monitor_interval_ms: options.service_monitor_interval_ms,
        })
    }

    async fn preload_supervised_lanes(&self, initial_session: &str) -> Result<(), String> {
        if !crate::runtime_host::admission_enabled() {
            return Ok(());
        }
        for (session, config) in crate::session_supervisor::runtime_host_supervised_lane_configs()?
        {
            if session != initial_session {
                self.lane(&session, Some(config)).await?;
            }
        }
        Ok(())
    }

    async fn lane(
        &self,
        session: &str,
        config: Option<crate::runtime_host::RuntimeLaneConfig>,
    ) -> Result<RuntimeLane, String> {
        if let Some(lane) = self.lanes.get(session) {
            if lane.configuration_committed || config.is_none() {
                return Ok(lane);
            }
        }
        if !crate::runtime_host::admission_enabled() {
            return Err(format!("runtime_host_lane_not_admitted: {session}"));
        }

        let _creation_guard = self.creation_lock.lock().await;
        if let Some(lane) = self.lanes.get(session) {
            if !lane.configuration_committed {
                if let Some(config) = config.as_ref() {
                    let old = self
                        .lanes
                        .remove(session)
                        .ok_or_else(|| "runtime_host_bootstrap_lane_disappeared".to_string())?;
                    old.control_plane.shutdown().await;
                    let control_plane = ControlPlaneWorker::start_with_options(
                        DaemonState::new_for_runtime_lane_with_stream(
                            session,
                            old.stream_client.clone(),
                            old.stream_server.clone(),
                            config,
                        )?,
                        config.service_reconcile_interval_ms,
                        config.service_job_timeout_ms,
                        config.service_monitor_interval_ms,
                    );
                    return self.lanes.insert(
                        session.to_string(),
                        RuntimeLane {
                            control_plane,
                            stream_client: old.stream_client,
                            stream_server: old.stream_server,
                            stream_file: old.stream_file,
                            config: config.clone(),
                            configuration_committed: true,
                        },
                    );
                }
            }
            return Ok(lane);
        }

        let configuration_committed = config.is_some();
        let config = config.unwrap_or_else(|| crate::runtime_host::RuntimeLaneConfig {
            service_reconcile_interval_ms: self.service_reconcile_interval_ms,
            service_job_timeout_ms: self.service_job_timeout_ms,
            service_monitor_interval_ms: self.service_monitor_interval_ms,
            ..Default::default()
        });
        let stream_port = config.stream_port.unwrap_or(0);
        let (stream_server, stream_client, stream_file) = match StreamServer::start_without_client(
            stream_port,
            session.to_string(),
            true,
        )
        .await
        {
            Ok((server, client)) => {
                let server = Arc::new(server);
                let path = self.socket_dir.join(format!("{session}.stream"));
                fs::write(&path, server.port().to_string()).map_err(|error| {
                    format!("runtime_host_stream_metadata_write_failed: {error}")
                })?;
                secure_daemon_file(&path);
                (Some(server), Some(client), Some(path))
            }
            Err(error) => {
                return Err(format!("runtime_host_stream_start_failed: {error}"));
            }
        };
        let lane = RuntimeLane {
            control_plane: ControlPlaneWorker::start_with_options(
                DaemonState::new_for_runtime_lane_with_stream(
                    session,
                    stream_client.clone(),
                    stream_server.clone(),
                    &config,
                )?,
                config.service_reconcile_interval_ms,
                config.service_job_timeout_ms,
                config.service_monitor_interval_ms,
            ),
            stream_client,
            stream_server,
            stream_file,
            config,
            configuration_committed,
        };
        self.lanes.insert(session.to_string(), lane)
    }

    async fn close_lane(&self, session: &str) {
        if let Some(lane) = self.lanes.remove(session) {
            lane.control_plane.shutdown().await;
            if let Some(server) = lane.stream_server {
                server.shutdown().await;
            }
            if let Some(path) = lane.stream_file {
                let _ = fs::remove_file(path);
            }
        }
    }

    async fn shutdown(&self) {
        for lane in self.lanes.take_all() {
            lane.control_plane.shutdown().await;
            if let Some(server) = lane.stream_server {
                server.shutdown().await;
            }
            if let Some(path) = lane.stream_file {
                let _ = fs::remove_file(path);
            }
        }
    }
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
async fn run_socket_server(
    listener: tokio::net::UnixListener,
    socket_path: &Path,
    session: &str,
    daemon_auth_token: Arc<String>,
    stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
    stream_server: Option<Arc<StreamServer>>,
    idle_timeout_ms: Option<u64>,
    service_reconcile_interval_ms: Option<u64>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
) -> Result<(), String> {
    let stream_file: Option<PathBuf> = if stream_server.is_some() {
        let dir = socket_path.parent().unwrap_or(std::path::Path::new("."));
        Some(dir.join(format!("{}.stream", session)))
    } else {
        None
    };

    let router = RuntimeHostRouter::new(
        socket_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
        session,
        stream_client,
        stream_server,
        stream_file.clone(),
        RuntimeHostWorkerOptions {
            service_reconcile_interval_ms,
            service_job_timeout_ms,
            service_monitor_interval_ms,
        },
    )?;
    router.preload_supervised_lanes(session).await?;

    let (reset_tx, mut reset_rx) = mpsc::channel::<()>(64);
    let reset_tx = idle_timeout_ms.map(|_| Arc::new(reset_tx));

    // Notifier used by handle_connection to signal the daemon loop to exit
    // after a "close" command, instead of calling process::exit() which skips
    // destructors and can leave Chrome processes orphaned (issue #1113).
    let close_notify = Arc::new(Notify::new());

    let idle_sleep = idle_timeout_ms.map(|ms| tokio::time::sleep(Duration::from_millis(ms)));
    let mut idle_sleep_pin = idle_sleep.map(Box::pin);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let router = router.clone();
                        let reset_tx = reset_tx.clone();
                        let sf = stream_file.clone();
                        let cn = close_notify.clone();
                        let auth = daemon_auth_token.clone();
                        let fallback_session = session.to_string();
                        tokio::spawn(async move {
                            handle_connection(stream, router, &fallback_session, reset_tx, sf, cn, auth).await;
                        });
                    }
                    Err(e) => {
                        let _ = writeln!(std::io::stderr(), "Accept error: {}", e);
                    }
                }
            }
            _ = async {
                match idle_sleep_pin {
                    Some(ref mut s) => s.as_mut().await,
                    None => std::future::pending::<()>().await,
                }
            }, if idle_timeout_ms.is_some() => {
                router.shutdown().await;
                break;
            }
            _ = reset_rx.recv(), if idle_timeout_ms.is_some() => {
                idle_sleep_pin = idle_timeout_ms
                    .map(|ms| Box::pin(tokio::time::sleep(Duration::from_millis(ms))));
                continue;
            }
            _ = close_notify.notified() => {
                // "close" command was handled; browser already closed by
                // handle_close(). Break to run cleanup and exit gracefully
                // so destructors fire.
                router.shutdown().await;
                break;
            }
            _ = shutdown_signal() => {
                router.shutdown().await;
                break;
            }
        }
    }

    Ok(())
}

#[cfg(windows)]
async fn run_socket_server(
    socket_path: &PathBuf,
    session: &str,
    daemon_auth_token: Arc<String>,
    stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
    stream_server: Option<Arc<StreamServer>>,
    idle_timeout_ms: Option<u64>,
    service_reconcile_interval_ms: Option<u64>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
) -> Result<(), String> {
    use tokio::net::TcpListener;

    let endpoint_key = crate::runtime_host::endpoint_key(session);
    let preferred_port = get_port_for_session(endpoint_key);
    // Try the hash-derived port first; if it is blocked (e.g. Windows Hyper-V
    // excluded port range), fall back to an OS-assigned ephemeral port.
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", preferred_port)).await {
        Ok(l) => l,
        Err(_) => TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind TCP: {}", e))?,
    };
    let actual_port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .port();

    let socket_dir = socket_path.parent().unwrap_or(std::path::Path::new("."));
    let port_path = socket_dir.join(format!("{}.port", endpoint_key));
    let _ = fs::write(&port_path, actual_port.to_string());
    secure_daemon_file(&port_path);

    let stream_file: Option<PathBuf> = if stream_server.is_some() {
        Some(socket_dir.join(format!("{}.stream", session)))
    } else {
        None
    };

    let router = RuntimeHostRouter::new(
        socket_dir.to_path_buf(),
        session,
        stream_client,
        stream_server,
        stream_file.clone(),
        RuntimeHostWorkerOptions {
            service_reconcile_interval_ms,
            service_job_timeout_ms,
            service_monitor_interval_ms,
        },
    )?;
    router.preload_supervised_lanes(session).await?;

    let (reset_tx, mut reset_rx) = mpsc::channel::<()>(64);
    let reset_tx = idle_timeout_ms.map(|_| Arc::new(reset_tx));

    let close_notify = Arc::new(Notify::new());

    let idle_sleep = idle_timeout_ms.map(|ms| tokio::time::sleep(Duration::from_millis(ms)));
    let mut idle_sleep_pin = idle_sleep.map(Box::pin);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let router = router.clone();
                        let reset_tx = reset_tx.clone();
                        let sf = stream_file.clone();
                        let cn = close_notify.clone();
                        let auth = daemon_auth_token.clone();
                        let fallback_session = session.to_string();
                        tokio::spawn(async move {
                            handle_connection(stream, router, &fallback_session, reset_tx, sf, cn, auth).await;
                        });
                    }
                    Err(e) => {
                        let _ = writeln!(std::io::stderr(), "Accept error: {}", e);
                    }
                }
            }
            _ = async {
                match idle_sleep_pin {
                    Some(ref mut s) => s.as_mut().await,
                    None => std::future::pending::<()>().await,
                }
            }, if idle_timeout_ms.is_some() => {
                router.shutdown().await;
                let _ = fs::remove_file(&port_path);
                break;
            }
            _ = reset_rx.recv(), if idle_timeout_ms.is_some() => {
                idle_sleep_pin = idle_timeout_ms
                    .map(|ms| Box::pin(tokio::time::sleep(Duration::from_millis(ms))));
                continue;
            }
            _ = close_notify.notified() => {
                router.shutdown().await;
                let _ = fs::remove_file(&port_path);
                break;
            }
            _ = shutdown_signal() => {
                router.shutdown().await;
                let _ = fs::remove_file(&port_path);
                break;
            }
        }
    }

    Ok(())
}

async fn handle_connection<S>(
    stream: S,
    router: RuntimeHostRouter,
    fallback_session: &str,
    idle_reset_tx: Option<Arc<mpsc::Sender<()>>>,
    stream_file_cleanup: Option<PathBuf>,
    close_notify: Arc<Notify>,
    daemon_auth_token: Arc<String>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if looks_like_http(trimmed) {
                    break;
                }

                let mut cmd: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = serde_json::json!({
                            "success": false,
                            "error": format!("Invalid JSON: {}", e),
                        });
                        let mut resp = serde_json::to_string(&err).unwrap_or_default();
                        resp.push('\n');
                        let _ = writer.write_all(resp.as_bytes()).await;
                        continue;
                    }
                };

                let authenticated = cmd
                    .get(DAEMON_AUTH_FIELD)
                    .and_then(|v| v.as_str())
                    .is_some_and(|token| token == daemon_auth_token.as_str());
                if !authenticated {
                    let err = serde_json::json!({
                        "success": false,
                        "error": "Unauthorized daemon command",
                    });
                    let mut resp = serde_json::to_string(&err).unwrap_or_default();
                    resp.push('\n');
                    let _ = writer.write_all(resp.as_bytes()).await;
                    continue;
                }
                if let Some(obj) = cmd.as_object_mut() {
                    obj.remove(DAEMON_AUTH_FIELD);
                }

                let lane_session = match crate::runtime_host::take_lane(&mut cmd, fallback_session)
                {
                    Ok(session) => session,
                    Err(error) => {
                        let mut response = serde_json::to_string(&serde_json::json!({
                            "success": false,
                            "error": error,
                        }))
                        .unwrap_or_default();
                        response.push('\n');
                        let _ = writer.write_all(response.as_bytes()).await;
                        continue;
                    }
                };
                let lane_config = match crate::runtime_host::take_lane_config(&mut cmd) {
                    Ok(config) => config,
                    Err(error) => {
                        let mut response = serde_json::to_string(&serde_json::json!({
                            "success": false,
                            "error": error,
                        }))
                        .unwrap_or_default();
                        response.push('\n');
                        let _ = writer.write_all(response.as_bytes()).await;
                        continue;
                    }
                };
                let lane = match router.lane(&lane_session, lane_config).await {
                    Ok(lane) => lane,
                    Err(error) => {
                        let mut response = serde_json::to_string(&serde_json::json!({
                            "success": false,
                            "error": error,
                        }))
                        .unwrap_or_default();
                        response.push('\n');
                        let _ = writer.write_all(response.as_bytes()).await;
                        continue;
                    }
                };
                let control_plane = lane.control_plane.clone();
                if crate::runtime_host::command_accepts_lane_profile_defaults(&cmd) {
                    if let (Some(runtime_profile), Some(object)) =
                        (lane.config.runtime_profile.as_ref(), cmd.as_object_mut())
                    {
                        object
                            .entry("runtimeProfile".to_string())
                            .or_insert_with(|| Value::String(runtime_profile.clone()));
                    }
                    if let (Some(profile), Some(object)) =
                        (lane.config.profile.as_ref(), cmd.as_object_mut())
                    {
                        object
                            .entry("profile".to_string())
                            .or_insert_with(|| Value::String(profile.clone()));
                    }
                }

                if let Some(ref tx) = idle_reset_tx {
                    let _ = tx.try_send(());
                }

                let action = cmd.get("action").and_then(|v| v.as_str());
                let exits_daemon = matches!(
                    action,
                    Some("close" | "runtime_handoff_finalize" | "runtime_handoff_rollback")
                );

                let response = if action == Some("worker_status") {
                    let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    control_plane.status_response(id)
                } else if action == Some("service_job_cancel") {
                    let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let job_id = cmd.get("jobId").and_then(|v| v.as_str()).unwrap_or("");
                    let reason = cmd.get("reason").and_then(|v| v.as_str());
                    control_plane.cancel_job_response(id, job_id, reason)
                } else if action == Some("service_status") {
                    let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let service_state = cmd
                        .get("serviceState")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({}));
                    let launch_config =
                        super::service_status_projection::launch_configuration_from_status_command(
                            &cmd,
                        );
                    let full_tab_history = cmd
                        .get("fullTabHistory")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    control_plane
                        .service_status_response(id, service_state, launch_config, full_tab_history)
                        .await
                } else {
                    control_plane.submit(cmd).await
                };

                let mut resp = serde_json::to_string(&response).unwrap_or_default();
                resp.push('\n');
                if writer.write_all(resp.as_bytes()).await.is_err() {
                    break;
                }

                if exits_daemon {
                    if !crate::runtime_host::admission_enabled() {
                        if let Some(ref path) = stream_file_cleanup {
                            let _ = fs::remove_file(path);
                        }
                    }
                    router.close_lane(&lane_session).await;
                    if !crate::runtime_host::admission_enabled() || router.lanes.is_empty() {
                        // Signal the daemon or now-empty runtime host to exit gracefully.
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        close_notify.notify_one();
                    }
                    return;
                }
            }
            Err(_) => break,
        }
    }
}

fn looks_like_http(line: &str) -> bool {
    let prefixes = [
        "GET ", "POST ", "PUT ", "DELETE ", "PATCH ", "HEAD ", "OPTIONS ", "CONNECT ", "TRACE ",
    ];
    prefixes.iter().any(|p| line.starts_with(p))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigint = match signal::unix::signal(signal::unix::SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(std::io::stderr(), "Failed to install SIGINT handler: {}", e);
                process::exit(1);
            }
        };
        let mut sigterm = match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "Failed to install SIGTERM handler: {}",
                    e
                );
                process::exit(1);
            }
        };
        let mut sighup = match signal::unix::signal(signal::unix::SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(std::io::stderr(), "Failed to install SIGHUP handler: {}", e);
                process::exit(1);
            }
        };

        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
            _ = sighup.recv() => {}
        }
    }

    #[cfg(windows)]
    {
        if let Err(e) = signal::ctrl_c().await {
            let _ = writeln!(std::io::stderr(), "Failed to install Ctrl+C handler: {}", e);
            process::exit(1);
        }
    }
}

fn get_daemon_socket_dir() -> PathBuf {
    if let Ok(dir) = env::var("AGENT_BROWSER_SOCKET_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }

    if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("agent-browser");
        }
    }

    if let Some(home) = dirs::home_dir() {
        return home.join(".agent-browser");
    }

    std::env::temp_dir().join("agent-browser")
}

#[cfg(windows)]
fn get_port_for_session(session: &str) -> u16 {
    let mut hash: i32 = 0;
    for c in session.chars() {
        hash = ((hash << 5).wrapping_sub(hash)).wrapping_add(c as i32);
    }
    49152 + ((hash.unsigned_abs() as u32 % 16383) as u16)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn executable_hashing_streams_file_contents() {
        let path = std::env::temp_dir().join(format!(
            "agent-browser-daemon-hash-{}-{}",
            process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should follow Unix epoch")
                .as_nanos()
        ));
        fs::write(&path, b"abc").expect("hash fixture should be written");

        let digest = file_sha256(&path).expect("hash fixture should be readable");

        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        fs::remove_file(path).expect("hash fixture should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn retiring_daemon_does_not_own_replacement_socket_path() {
        use std::os::unix::net::UnixListener;

        let fixture_dir = PathBuf::from("/tmp").join(format!(
            "ab-daemon-socket-{}-{}",
            process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should follow Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&fixture_dir).expect("fixture directory should be created");
        let socket_path = fixture_dir.join("handoff.sock");

        let retiring_listener =
            UnixListener::bind(&socket_path).expect("retiring daemon socket should bind");
        let retiring_identity =
            unix_socket_identity(&socket_path).expect("retiring socket should have an identity");
        assert!(unix_socket_path_is_owned(&socket_path, retiring_identity));

        fs::remove_file(&socket_path).expect("retiring socket path should be unlinked");
        let replacement_listener =
            UnixListener::bind(&socket_path).expect("replacement daemon socket should bind");
        assert!(
            !unix_socket_path_is_owned(&socket_path, retiring_identity),
            "retiring daemon must not own the replacement socket path"
        );

        drop(retiring_listener);
        drop(replacement_listener);
        fs::remove_file(&socket_path).expect("replacement socket path should be removed");
        fs::remove_dir(&fixture_dir).expect("fixture directory should be removed");
    }

    #[cfg(windows)]
    #[test]
    fn test_port_matches_client_algorithm() {
        assert_eq!(get_port_for_session("default"), 50838);
        assert_eq!(get_port_for_session("my-session"), 63105);
        assert_eq!(get_port_for_session("work"), 51184);
        assert_eq!(get_port_for_session(""), 49152);
    }

    /// Guard against re-introducing `waitpid(-1)` in daemon code.
    ///
    /// Issue #1035: a SIGCHLD handler that called `waitpid(-1, WNOHANG)` was
    /// added in v0.22.3 to reap zombie Chrome processes. This races with
    /// Rust's `Child::try_wait()` / `Child::wait()` because `waitpid(-1)`
    /// reaps *any* child, stealing the exit status before Rust can collect
    /// it. The result is ECHILD errors in `BrowserManager::has_process_exited()`
    /// and `ChromeProcess::kill()`, which can leave the daemon in a broken
    /// state or cause hangs on certain Linux configurations.
    ///
    /// The fix uses the existing 500ms drain interval to call
    /// `has_process_exited()` (which delegates to `Child::try_wait()`)
    /// for targeted, race-free zombie detection.
    #[test]
    fn test_no_waitpid_minus_one_in_daemon() {
        let source = include_str!("daemon.rs");
        // Only check production code (everything before `#[cfg(test)]`)
        let production_code = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            !production_code.contains("waitpid(-1"),
            "daemon.rs production code must not call waitpid(-1, ...). \
             Use Child::try_wait() via has_process_exited() instead. \
             See issue #1035."
        );
    }

    /// Verify that `Child::try_wait()` correctly detects a crashed child
    /// without needing a global SIGCHLD handler or `waitpid(-1)`.
    /// This is what `has_process_exited()` uses in the fixed code.
    #[cfg(unix)]
    #[test]
    fn test_child_try_wait_detects_exit_without_sigchld_handler() {
        use std::process::{Command, Stdio};

        let mut child = Command::new("/bin/sh")
            .args(["-c", "exit 42"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn child");

        std::thread::sleep(std::time::Duration::from_millis(200));

        match child.try_wait() {
            Ok(Some(status)) => {
                assert!(
                    !status.success(),
                    "child exited with code 42, should not be success"
                );
            }
            Ok(None) => panic!("try_wait() returned None but child should have exited"),
            Err(e) => panic!("try_wait() should succeed without waitpid(-1): {}", e),
        }
    }

    /// Regression test for #1101: idle timeout must fire even while the
    /// drain interval ticks every 500 ms.  The bug was that `sleep_future`
    /// was created **inside** the loop, so each drain tick dropped the
    /// in-progress sleep and replaced it with a fresh one – the timer
    /// could never reach its deadline.
    #[tokio::test]
    async fn test_idle_timeout_fires_despite_drain_interval() {
        use tokio::sync::mpsc;

        let idle_timeout_ms: u64 = 1000;
        let mut drain_interval = tokio::time::interval(Duration::from_millis(500));
        drain_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let (_reset_tx, mut reset_rx) = mpsc::channel::<()>(64);

        let start = tokio::time::Instant::now();

        let exited = tokio::time::timeout(Duration::from_secs(5), async {
            let mut idle_sleep_pin = Some(Box::pin(tokio::time::sleep(Duration::from_millis(
                idle_timeout_ms,
            ))));

            loop {
                tokio::select! {
                    _ = drain_interval.tick() => {}
                    _ = async {
                        match idle_sleep_pin {
                            Some(ref mut s) => s.as_mut().await,
                            None => std::future::pending::<()>().await,
                        }
                    } => {
                        break;
                    }
                    _ = reset_rx.recv() => {
                        idle_sleep_pin = Some(Box::pin(
                            tokio::time::sleep(Duration::from_millis(idle_timeout_ms)),
                        ));
                        continue;
                    }
                }
            }
        })
        .await;

        let elapsed = start.elapsed();

        assert!(
            exited.is_ok(),
            "idle timeout never fired – loop ran for >5 s (bug #1101)"
        );
        assert!(
            elapsed < Duration::from_millis(idle_timeout_ms + 500),
            "idle timeout took too long: {:?} (expected ~{} ms)",
            elapsed,
            idle_timeout_ms,
        );
    }

    /// Verify that `ChromeProcess::has_exited()` (which uses `Child::try_wait()`)
    /// correctly detects a killed child, the same way the drain interval does
    /// in the fixed daemon code. This ensures crash detection works without
    /// a SIGCHLD handler.
    #[cfg(unix)]
    #[test]
    fn test_has_exited_detects_killed_process() {
        use std::process::{Command, Stdio};

        let mut child = Command::new("/bin/sh")
            .args(["-c", "sleep 60"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn child");

        // Process should be running
        match child.try_wait() {
            Ok(None) => {} // expected
            other => panic!("expected Ok(None) for running process, got {:?}", other),
        }

        // Kill it (simulates Chrome crash)
        child.kill().expect("failed to kill child");
        std::thread::sleep(std::time::Duration::from_millis(100));

        // try_wait should detect the exit
        match child.try_wait() {
            Ok(Some(_)) => {} // expected: detected the crash
            other => panic!(
                "expected Ok(Some(_)) after kill, got {:?}. \
                 Crash detection via try_wait() must work for the drain \
                 interval fix (issue #1035) to function correctly.",
                other
            ),
        }
    }
}
