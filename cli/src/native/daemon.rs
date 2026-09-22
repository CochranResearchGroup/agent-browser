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
use tokio::sync::{mpsc, watch, Mutex, Notify, RwLock};

use super::action_runtime::DaemonState;
use super::control_plane::{ControlPlaneHandle, ControlPlaneWorker};
use super::state;
use super::stream::StreamServer;
use crate::connection::write_daemon_process_identity;
use crate::process_identity::capture_process_identity;
use agent_browser_cdp::client::CdpClient;

const DAEMON_AUTH_TOKEN_ENV: &str = "AGENT_BROWSER_DAEMON_AUTH_TOKEN";
const DAEMON_AUTH_FIELD: &str = "_agentBrowserAuthToken";
const CONNECTION_READ_AHEAD_CAPACITY: usize = 8;

fn should_journal_browser_open(
    action_is_open: bool,
    has_named_profile: bool,
    explicit_display: bool,
    has_remote_desktop_routes: bool,
    existing_operation: bool,
) -> bool {
    existing_operation
        || (action_is_open && has_named_profile && !explicit_display && has_remote_desktop_routes)
}

fn require_keeper_handoff_resolution(
    required: bool,
    routes: &[agent_browser_service_model::BrowserDesktopRoute],
) -> Result<(), &'static str> {
    if !required {
        return Ok(());
    }
    if routes.is_empty() {
        return Err("browser_session_presentation_route_unavailable");
    }
    Ok(())
}

fn presentation_keeper_status(
    authority: &agent_browser_service_model::RouteKeeperAuthority,
    probe: &super::stream::RouteKeeperSupervisorProbe,
    config: &super::browser_session_store::BrowserRuntimeConfig,
) -> Result<super::presentation_runtime_status::PresentationKeeperStatus, String> {
    let mut status = super::presentation_runtime_status::keeper_status(
        authority,
        probe.health(),
        Some(probe.host_generation()),
    )?;
    status.apply_runtime_config(config);
    Ok(status)
}

fn current_presentation_keeper_status(
    probe: Option<&super::stream::RouteKeeperSupervisorProbe>,
) -> Result<super::presentation_runtime_status::PresentationKeeperStatus, String> {
    let store = super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
    let authority = store.load_route_keeper_authority()?;
    let mut status = match probe {
        Some(probe) => presentation_keeper_status(&authority, probe, &store.load_runtime_config()?),
        None => super::presentation_runtime_status::keeper_status(
            &authority,
            super::stream::RouteKeeperSupervisorHealth::Unavailable {
                code: "presentation_keeper_unavailable".to_string(),
            },
            None,
        ),
    }?;
    status.apply_runtime_config(&store.load_runtime_config()?);
    let routes = if status.require_ready().is_ok() {
        status.usable_routes(&authority)?
    } else {
        Vec::new()
    };
    let mut allocation = super::presentation_runtime_capacity::allocation_status(
        &store.load_session_state()?,
        &store.load_runtime_config()?,
        &routes,
    )?;
    if status.require_ready().is_err() {
        allocation.state = "unavailable";
    }
    status.allocation = Some(allocation);
    status.queue = Some(super::presentation_request_admission::queue_status(&store)?);
    Ok(status)
}

enum PresentationKeeperObservation<T> {
    Ready(T),
    Pending,
    Terminal(String),
}

/// Bounds readiness and capacity waiting. A cancelled blocking observation may
/// finish publishing route demand afterward; browser effects happen only after
/// this wait succeeds and the host revalidates admission.
async fn wait_for_presentation_keeper_observation<T, F, Fut>(
    deadline: Instant,
    mut observe: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<PresentationKeeperObservation<T>, String>>,
{
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("presentation_keeper_deadline_exceeded".to_string());
        }
        let observation = tokio::time::timeout(remaining, observe())
            .await
            .map_err(|_| "presentation_keeper_deadline_exceeded".to_string())??;
        match observation {
            PresentationKeeperObservation::Ready(value) => return Ok(value),
            PresentationKeeperObservation::Terminal(error) => return Err(error),
            PresentationKeeperObservation::Pending => {
                tokio::time::sleep(
                    Duration::from_millis(25)
                        .min(deadline.saturating_duration_since(Instant::now())),
                )
                .await;
            }
        }
    }
}

async fn observe_presentation_keeper(
    route_keeper: Arc<Mutex<Option<super::stream::ConfiguredRouteKeeperSupervisorHandle>>>,
    command: Option<Value>,
) -> Result<PresentationKeeperObservation<(super::stream::RouteKeeperSupervisorProbe, bool)>, String>
{
    let probe = route_keeper
        .lock()
        .await
        .as_ref()
        .map(super::stream::ConfiguredRouteKeeperSupervisorHandle::probe)
        .ok_or_else(|| "presentation_keeper_unavailable".to_string())?;
    let probe_for_status = probe.clone();
    let status = tokio::task::spawn_blocking(move || {
        current_presentation_keeper_status(Some(&probe_for_status))
    })
    .await
    .map_err(|error| format!("presentation_keeper_status_join_failed:{error}"))??;
    match status.require_ready() {
        Ok(()) => {
            let capacity_available = status
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.state == "available");
            if let Some(command) = command {
                if status
                    .allocation
                    .as_ref()
                    .is_some_and(|allocation| allocation.state == "pending")
                {
                    tokio::task::spawn_blocking(move || -> Result<(), String> {
                        let mut store = super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
                        let state = store.load_session_state()?;
                        if !super::presentation_runtime_capacity::command_reuses_browser(&command, &state) {
                            let config = store.load_runtime_config()?;
                            let count = status.allocation.as_ref().map(|allocation| allocation.occupied_display_count).unwrap_or_default();
                            super::presentation_runtime_capacity::request_additional_route(&mut store, count, config.maximum_displays)?;
                        }
                        Ok(())
                    }).await.map_err(|_| "presentation_capacity_observation_failed".to_string())??;
                }
            }
            Ok(PresentationKeeperObservation::Ready((
                probe,
                capacity_available,
            )))
        }
        Err(_)
            if matches!(
                probe.health(),
                super::stream::RouteKeeperSupervisorHealth::Recovering
                    | super::stream::RouteKeeperSupervisorHealth::Supervising
            ) && matches!(status.state, "recovering" | "degraded") =>
        {
            Ok(PresentationKeeperObservation::Pending)
        }
        Err(error) => Ok(PresentationKeeperObservation::Terminal(error)),
    }
}

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
    super::service_failure_journal::initialize_failure_journal();
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
    browser_sessions:
        Arc<std::sync::Mutex<Option<super::browser_session_host::DefaultBrowserSessionHost>>>,
    route_keeper: Arc<Mutex<Option<super::stream::ConfiguredRouteKeeperSupervisorHandle>>>,
    creation_lock: Arc<Mutex<()>>,
    socket_dir: PathBuf,
    service_reconcile_interval_ms: Option<u64>,
    service_job_timeout_ms: Option<u64>,
    service_monitor_interval_ms: Option<u64>,
    browser_session_reap_interval_ms: u64,
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
            browser_sessions: Arc::new(std::sync::Mutex::new(None)),
            route_keeper: Arc::new(Mutex::new(None)),
            creation_lock: Arc::new(Mutex::new(())),
            socket_dir,
            service_reconcile_interval_ms: options.service_reconcile_interval_ms,
            service_job_timeout_ms: options.service_job_timeout_ms,
            service_monitor_interval_ms: options.service_monitor_interval_ms,
            browser_session_reap_interval_ms: browser_session_reap_interval_ms(
                options.service_reconcile_interval_ms,
            ),
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

    async fn install_configured_route_keeper(&self) -> Result<(), String> {
        if !crate::runtime_host::admission_enabled() {
            return Ok(());
        }
        #[cfg(not(unix))]
        return Ok(());
        #[cfg(unix)]
        {
            let keeper = super::stream::ConfiguredRouteKeeperSupervisorHandle::start_default()?;
            self.install_route_keeper_owner(keeper).await
        }
    }

    async fn initialize_runtime_host(&self, session: &str) -> Result<(), String> {
        let result = async {
            self.preload_supervised_lanes(session).await?;
            self.install_configured_route_keeper().await
        }
        .await;
        let Err(error) = result else {
            return Ok(());
        };
        match self.shutdown().await {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!(
                "runtime_host_initialization_cleanup_failed:{error}:{cleanup}"
            )),
        }
    }

    async fn install_route_keeper_owner(
        &self,
        mut keeper: super::stream::ConfiguredRouteKeeperSupervisorHandle,
    ) -> Result<(), String> {
        let mut installed = self.route_keeper.lock().await;
        if installed.is_some() {
            drop(installed);
            keeper.shutdown().await?;
            return Err("runtime_host_route_keeper_already_installed".to_string());
        }
        *installed = Some(keeper);
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

    async fn shutdown(&self) -> Result<(), String> {
        let keeper_result = match self.route_keeper.lock().await.take() {
            Some(mut keeper) => keeper.shutdown().await,
            None => Ok(()),
        };
        for lane in self.lanes.take_all() {
            lane.control_plane.shutdown().await;
            if let Some(server) = lane.stream_server {
                server.shutdown().await;
            }
            if let Some(path) = lane.stream_file {
                let _ = fs::remove_file(path);
            }
        }
        let browser_sessions = self.browser_sessions.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut host) = browser_sessions.lock() {
                host.take();
            }
        })
        .await;
        keeper_result
    }

    async fn route_keeper_probe(&self) -> Option<super::stream::RouteKeeperSupervisorProbe> {
        self.route_keeper
            .lock()
            .await
            .as_ref()
            .map(super::stream::ConfiguredRouteKeeperSupervisorHandle::probe)
    }

    async fn wait_for_presentation_keeper(
        &self,
        command: Value,
        existing_handoff: bool,
    ) -> Result<
        (
            super::stream::RouteKeeperSupervisorProbe,
            super::presentation_request_admission::PresentationAdmission,
        ),
        String,
    > {
        let probe = self
            .route_keeper_probe()
            .await
            .ok_or_else(|| "presentation_keeper_unavailable".to_string())?;
        let generation = probe.host_generation();
        let queued_command = command.clone();
        let request = tokio::task::spawn_blocking(move || {
            super::presentation_request_admission::PresentationAdmissionRequest::enqueue(
                &queued_command,
                generation,
                existing_handoff,
            )
        })
        .await
        .map_err(|_| "presentation_queue_enqueue_failed".to_string())??;
        let deadline = Instant::now()
            + Duration::from_millis(
                request
                    .deadline_at_ms
                    .saturating_sub(super::presentation_request_admission::now_ms()),
            );
        let request = Arc::new(request);
        let replay_request = request.clone();
        if let Some(admission) = tokio::task::spawn_blocking(move || replay_request.poll(None))
            .await
            .map_err(|_| "presentation_queue_poll_failed".to_string())??
        {
            return Ok((probe, admission));
        }
        let route_keeper = self.route_keeper.clone();
        let result = wait_for_presentation_keeper_observation(deadline, || {
            let request = request.clone();
            let route_keeper = route_keeper.clone();
            let command = if existing_handoff {
                None
            } else {
                Some(command.clone())
            };
            async move {
                let observation = observe_presentation_keeper(route_keeper, command).await?;
                let readiness = match observation {
                    PresentationKeeperObservation::Ready((observed_probe, capacity)) => {
                        if observed_probe.host_generation() != generation {
                            return Ok(PresentationKeeperObservation::Terminal(
                                "presentation_queue_generation_stale".to_string(),
                            ));
                        }
                        Some(capacity)
                    }
                    PresentationKeeperObservation::Pending => None,
                    PresentationKeeperObservation::Terminal(error) => {
                        return Ok(PresentationKeeperObservation::Terminal(error))
                    }
                };
                let admission = tokio::task::spawn_blocking(move || request.poll(readiness))
                    .await
                    .map_err(|_| "presentation_queue_poll_failed".to_string())??;
                Ok(match admission {
                    Some(admission) => PresentationKeeperObservation::Ready(admission),
                    None => PresentationKeeperObservation::Pending,
                })
            }
        })
        .await?;
        Ok((probe, result))
    }

    async fn handle_browser_session_command(&self, mut command: Value) -> Value {
        let runtime_environment = std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT").ok();
        let publish_manager_handoff =
            match super::browser_session_host::browser_session_navigation_requires_handoff(
                &command,
                runtime_environment.as_deref(),
            ) {
                Ok(required) => required,
                Err(error) => return serde_json::json!({ "success": false, "error": error }),
            };
        let action = command.get("action").and_then(Value::as_str);
        let action_is_open = action == Some("browser_session_open");
        let has_named_profile = command
            .get("profileId")
            .or_else(|| {
                command
                    .get("params")
                    .and_then(|params| params.get("profileId"))
            })
            .and_then(Value::as_str)
            .is_some();
        let explicit_display = std::env::var_os("AGENT_BROWSER_SESSION_DISPLAY").is_some();
        let remote_open_candidate = action_is_open && has_named_profile && !explicit_display;
        let keeper_handoff_required = publish_manager_handoff;
        let keeper_required = keeper_handoff_required || remote_open_candidate;
        if keeper_required && command.get("id").and_then(Value::as_str).is_none() {
            command["id"] = Value::String(uuid::Uuid::new_v4().to_string());
        }
        let (probe, mut permit) = if keeper_required {
            match self
                .wait_for_presentation_keeper(command.clone(), false)
                .await
            {
                Ok((
                    probe,
                    super::presentation_request_admission::PresentationAdmission::Execute(permit),
                )) => (Some(probe), Some(permit)),
                Ok((
                    _,
                    super::presentation_request_admission::PresentationAdmission::Replay(response),
                )) => return response,
                Err(error) => return serde_json::json!({ "success": false, "error": error }),
            }
        } else {
            (None, None)
        };
        let browser_sessions = self.browser_sessions.clone();
        match tokio::task::spawn_blocking(move || -> Result<Value, String> {
            let result = (|| -> Result<Value, String> {
                let action = command.get("action").and_then(Value::as_str);
                let mut host = browser_sessions
                    .lock()
                    .map_err(|_| "browser_session_host_lock_poisoned".to_string())?;
                if let Some(permit) = permit.as_mut() {
                    permit.require_current()?;
                }
                // This is deliberately after taking the host lock. A queued command must
                // revalidate the live supervisor and durable route state immediately before
                // it can create a browser or publish a manager handoff.
                let (keeper_authority, routes) = if let Some(probe) = probe.as_ref() {
                    let store =
                        super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
                    let authority = store.load_route_keeper_authority()?;
                    let status = presentation_keeper_status(
                        &authority,
                        probe,
                        &store.load_runtime_config()?,
                    )?;
                    status.require_ready()?;
                    let routes = status.usable_routes(&authority)?;
                    (Some(authority), routes)
                } else {
                    (None, Vec::new())
                };
                require_keeper_handoff_resolution(keeper_required, &routes)
                    .map_err(str::to_string)?;
                if host.is_none() {
                    *host = Some(super::browser_session_host::load_default_browser_session_host()?);
                }
                let host = host
                    .as_mut()
                    .ok_or_else(|| "browser_session_host_missing".to_string())?;
                if keeper_required {
                    let config =
                        super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?
                            .load_runtime_config()?;
                    host.set_desktop_capacity(
                        config.maximum_displays,
                        config.maximum_browsers_per_display,
                    );
                }
                let operation_id = command.get("id").and_then(Value::as_str);
                let existing_open_operation = operation_id
                    .map(|operation_id| host.has_operation(operation_id))
                    .transpose()?
                    .unwrap_or(false);
                let has_remote_desktop_routes = !routes.is_empty();
                let journaled_open = should_journal_browser_open(
                    action_is_open,
                    has_named_profile,
                    explicit_display,
                    has_remote_desktop_routes,
                    existing_open_operation,
                );
                if action == Some("browser_session_navigate") || journaled_open {
                    host.replace_remote_desktop_routes(routes);
                }
                if keeper_handoff_required {
                    let authority = keeper_authority.as_ref().ok_or_else(|| {
                        "browser_session_keeper_handoff_authority_missing".to_string()
                    })?;
                    host.preflight_keeper_navigation_command(&command, authority)?;
                }
                let mut response = if journaled_open {
                    let repository =
                        super::service_store::LockedServiceStateRepository::default_json()?;
                    let authority = keeper_authority.as_ref().ok_or_else(|| {
                        "browser_session_keeper_handoff_authority_missing".to_string()
                    })?;
                    let response =
                        host.handle_journaled_open_with_keeper_handoff(&command, authority);
                    if response.get("success").and_then(Value::as_bool) == Some(true) {
                        if let Some(handoff) = response
                            .get("data")
                            .and_then(|data| data.get("handoffId"))
                            .and_then(Value::as_str)
                            .and_then(|handoff_id| host.manager_handoff(handoff_id))
                            .cloned()
                        {
                            let _ =
                            super::browser_session_handoff::project_manager_handoff_in_repository(
                                &handoff,
                                &repository,
                            );
                        }
                    }
                    response
                } else if action == Some("browser_session_navigate") && keeper_handoff_required {
                    let authority = keeper_authority.as_ref().ok_or_else(|| {
                        "browser_session_keeper_handoff_authority_missing".to_string()
                    })?;
                    host.handle_journaled_navigation_with_keeper(&command, authority)
                } else if action == Some("browser_session_navigate")
                    && command.get("headers").is_some()
                {
                    host.handle_managed_navigation_command(&command)
                } else {
                    host.handle_command(&command)
                };
                if keeper_handoff_required && action != Some("browser_session_navigate") {
                    let authority = keeper_authority.as_ref().ok_or_else(|| {
                        "browser_session_keeper_handoff_authority_missing".to_string()
                    })?;
                    if let Err(error) = host.attach_keeper_manager_handoff(&mut response, authority)
                    {
                        response["success"] = Value::Bool(false);
                        response["error"] = Value::String(error.clone());
                        response["data"]["operatorVisible"] = serde_json::json!({
                            "state": "unavailable",
                            "reason": error,
                        });
                    }
                }
                Ok(response)
            })();
            match permit {
                Some(permit) => permit.finish(result),
                None => result,
            }
        })
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => serde_json::json!({ "success": false, "error": error }),
            Err(error) => serde_json::json!({
                "success": false,
                "error": format!("browser_session_host_join_failed:{error}"),
            }),
        }
    }

    async fn try_handle_managed_browser_command(
        &self,
        session_name: &str,
        command: Value,
    ) -> Option<Value> {
        let browser_sessions = self.browser_sessions.clone();
        let session_name = session_name.to_string();
        match tokio::task::spawn_blocking(move || -> Result<Option<Value>, String> {
            let mut host = browser_sessions
                .lock()
                .map_err(|_| "browser_session_host_lock_poisoned".to_string())?;
            if host.is_none() {
                let store =
                    super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
                let state = store.load_session_state()?;
                let has_session = state
                    .sessions
                    .values()
                    .any(|session| session.name == session_name);
                if !has_session {
                    return Ok(None);
                }
                *host = Some(super::browser_session_host::load_default_browser_session_host()?);
            }
            host.as_mut()
                .ok_or_else(|| "browser_session_host_missing".to_string())?
                .execute_managed_command(&session_name, &command)
        })
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => Some(serde_json::json!({ "success": false, "error": error })),
            Err(error) => Some(serde_json::json!({
                "success": false,
                "error": format!("browser_session_host_join_failed:{error}"),
            })),
        }
    }

    async fn try_handle_browser_session_focus(&self, command: Value) -> Option<Value> {
        let browser_sessions = self.browser_sessions.clone();
        match tokio::task::spawn_blocking(move || -> Result<Option<Value>, String> {
            let mut host = browser_sessions
                .lock()
                .map_err(|_| "browser_session_host_lock_poisoned".to_string())?;
            if host.is_none() {
                let store =
                    super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
                let state = store.load_session_state()?;
                if browser_session_focus_command(&command, &state).is_none() {
                    return Ok(None);
                }
                *host = Some(super::browser_session_host::load_default_browser_session_host()?);
            } else if host
                .as_ref()
                .is_none_or(|host| browser_session_focus_command(&command, host.state()).is_none())
            {
                return Ok(None);
            }
            let command = browser_session_focus_command(
                &command,
                host.as_ref()
                    .ok_or_else(|| "browser_session_host_missing".to_string())?
                    .state(),
            )
            .ok_or_else(|| "browser_session_focus_route_lost".to_string())?;
            Ok(host.as_mut().map(|host| host.handle_command(&command)))
        })
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => Some(serde_json::json!({ "success": false, "error": error })),
            Err(error) => Some(serde_json::json!({
                "success": false,
                "error": format!("browser_session_host_join_failed:{error}"),
            })),
        }
    }

    /// Resolves a manager handoff only after loading its SQLite registry row and
    /// the current Ready route-keeper binding immediately before browser focus.
    /// Returns `None` only when the identifier is absent or is not manager-owned.
    async fn try_resolve_manager_handoff(&self, mut command: Value) -> Option<Value> {
        let handoff_id = match command
            .get("handoffId")
            .or_else(|| {
                command
                    .get("params")
                    .and_then(|params| params.get("handoffId"))
            })
            .or_else(|| command.get("remoteViewHandoffId"))
            .and_then(Value::as_str)
        {
            Some(handoff_id) => handoff_id.to_string(),
            None => {
                return Some(serde_json::json!({
                    "success": false,
                    "error": "service_remote_view_handoff_resolve requires handoffId",
                }))
            }
        };
        let lookup_id = handoff_id.clone();
        let manager_owned = match tokio::task::spawn_blocking(move || -> Result<bool, String> {
            let store = super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
            let registry = store.load_handoff_registry()?;
            Ok(registry
                .handoffs
                .get(&lookup_id)
                .is_some_and(super::browser_session_handoff::is_manager_handoff))
        })
        .await
        {
            Ok(Ok(manager_owned)) => manager_owned,
            Ok(Err(error)) => return Some(serde_json::json!({ "success": false, "error": error })),
            Err(error) => {
                return Some(serde_json::json!({
                    "success": false,
                    "error": format!("browser_session_host_join_failed:{error}"),
                }))
            }
        };
        if !manager_owned {
            return None;
        }
        if command.get("id").and_then(Value::as_str).is_none() {
            command["id"] = Value::String(uuid::Uuid::new_v4().to_string());
        }
        let (probe, mut permit) = match self
            .wait_for_presentation_keeper(command.clone(), true)
            .await
        {
            Ok((
                probe,
                super::presentation_request_admission::PresentationAdmission::Execute(permit),
            )) => (probe, permit),
            Ok((
                _,
                super::presentation_request_admission::PresentationAdmission::Replay(response),
            )) => return Some(response),
            Err(error) => return Some(serde_json::json!({ "success": false, "error": error })),
        };
        let browser_sessions = self.browser_sessions.clone();
        match tokio::task::spawn_blocking(move || -> Result<Option<Value>, String> {
            let result = (|| -> Result<Option<Value>, String> {
            let handoff_id = command
                .get("handoffId")
                .or_else(|| {
                    command
                        .get("params")
                        .and_then(|params| params.get("handoffId"))
                })
                .or_else(|| command.get("remoteViewHandoffId"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    "service_remote_view_handoff_resolve requires handoffId".to_string()
                })?;
            let store = super::browser_session_store::BrowserRuntimeSqliteStore::default_sqlite()?;
            let registry = store.load_handoff_registry()?;
            let Some(handoff) = registry.handoffs.get(handoff_id).cloned() else {
                return Ok(None);
            };
            if !super::browser_session_handoff::is_manager_handoff(&handoff) {
                return Ok(None);
            }
            let mut host = browser_sessions
                .lock()
                .map_err(|_| "browser_session_host_lock_poisoned".to_string())?;
            permit.require_current()?;
            // The handoff may have waited behind an earlier browser operation, so
            // validate the durable authority and live probe after acquiring the lock.
            let authority = store.load_route_keeper_authority()?;
            let status = presentation_keeper_status(&authority, &probe, &store.load_runtime_config()?)?;
            status.require_ready()?;
            let routes = status.usable_routes(&authority)?;
            let route_id = handoff
                .last_route_id
                .as_deref()
                .ok_or_else(|| "presentation_keeper_route_unavailable".to_string())?;
            let route = authority
                .records
                .get(route_id)
                .ok_or_else(|| "presentation_keeper_route_unavailable".to_string())?;
            if route.fence.host_generation != probe.host_generation() {
                return Err("presentation_keeper_route_stale".to_string());
            }
            if !routes.iter().any(|route| route.id == route_id) {
                return Err("presentation_keeper_route_unavailable".to_string());
            }
            if host.is_none() {
                *host = Some(super::browser_session_host::load_default_browser_session_host()?);
            }
            let host = host
                .as_mut()
                .ok_or_else(|| "browser_session_host_missing".to_string())?;
            let activity_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
                .unwrap_or_default();
            let resolved =
                host.resolve_journaled_manager_handoff_with_keeper(&command, &handoff, &authority, activity_at_ms)?;
            let id = command.get("id").cloned().unwrap_or(Value::Null);
            Ok(Some(serde_json::json!({
                "id": id,
                "success": true,
                "data": resolved,
            })))
            })();
            permit.finish(result.map(|value| value.unwrap_or_else(|| serde_json::json!({"success":false,"error":"browser_session_handoff_missing"})))).map(Some)
        })
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => Some(serde_json::json!({ "success": false, "error": error })),
            Err(error) => Some(serde_json::json!({
                "success": false,
                "error": format!("browser_session_host_join_failed:{error}"),
            })),
        }
    }

    async fn attach_browser_session_state(&self, response: Value) -> Value {
        let browser_sessions = self.browser_sessions.clone();
        let keeper_probe = self.route_keeper_probe().await;
        let keeper_status = tokio::task::spawn_blocking(move || {
            current_presentation_keeper_status(keeper_probe.as_ref()).unwrap_or_else(|_| {
                super::presentation_runtime_status::unavailable_status(
                    "presentation_keeper_status_unavailable",
                )
            })
        })
        .await
        .unwrap_or_else(|_| {
            super::presentation_runtime_status::unavailable_status(
                "presentation_keeper_status_unavailable",
            )
        });
        let snapshot = tokio::task::spawn_blocking(move || {
            let mut host = browser_sessions
                .lock()
                .map_err(|_| "browser_session_host_lock_poisoned".to_string())?;
            if host.is_none() {
                *host = Some(super::browser_session_host::load_default_browser_session_host()?);
            }
            let host = host
                .as_mut()
                .ok_or_else(|| "browser_session_host_missing".to_string())?;
            host.reconcile_liveness_current()?;
            serde_json::to_value(host.state())
                .map_err(|error| format!("browser_session_status_serialize_failed:{error}"))
        })
        .await
        .map_err(|error| format!("browser_session_status_join_failed:{error}"))
        .and_then(|snapshot| snapshot);
        attach_presentation_keeper_status(
            attach_browser_session_state_to_status(response, snapshot),
            keeper_status,
        )
    }

    async fn reap_browser_sessions_if_loaded(&self) -> Result<(), String> {
        let browser_sessions = self.browser_sessions.clone();
        tokio::task::spawn_blocking(move || {
            let mut host = browser_sessions
                .lock()
                .map_err(|_| "browser_session_host_lock_poisoned".to_string())?;
            if let Some(host) = host.as_mut() {
                host.reap_current()?;
            }
            Ok(())
        })
        .await
        .map_err(|error| format!("browser_session_reap_join_failed:{error}"))?
    }
}

fn browser_session_focus_command(
    command: &Value,
    state: &agent_browser_service_model::BrowserSessionState,
) -> Option<Value> {
    if command.get("action").and_then(Value::as_str) != Some("view_focus") {
        return None;
    }
    let browser_id = command.get("browserId").and_then(Value::as_str)?;
    if !state.browsers.contains_key(browser_id) {
        return None;
    }
    let mut routed = command.clone();
    routed["action"] = Value::String("browser_session_focus".to_string());
    Some(routed)
}

fn browser_session_reap_interval_ms(service_reconcile_interval_ms: Option<u64>) -> u64 {
    const DEFAULT_REAP_INTERVAL_MS: u64 = 30_000;
    service_reconcile_interval_ms
        .unwrap_or(DEFAULT_REAP_INTERVAL_MS)
        .max(1)
}

fn spawn_browser_session_reaper(router: RuntimeHostRouter) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(
            router.browser_session_reap_interval_ms,
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = router.reap_browser_sessions_if_loaded().await {
                let _ = writeln!(std::io::stderr(), "Browser session reaper error: {error}");
            }
        }
    })
}

fn attach_browser_session_state_to_status(
    mut response: Value,
    snapshot: Result<Value, String>,
) -> Value {
    let Some(data) = response.get_mut("data").and_then(Value::as_object_mut) else {
        return response;
    };
    match snapshot {
        Ok(snapshot) => {
            data.insert("browserSessionState".to_string(), snapshot);
        }
        Err(error) => {
            data.insert("browserSessionState".to_string(), Value::Null);
            data.insert("browserSessionStateError".to_string(), Value::String(error));
        }
    }
    response
}

fn attach_presentation_keeper_status(
    mut response: Value,
    status: super::presentation_runtime_status::PresentationKeeperStatus,
) -> Value {
    let Some(data) = response.get_mut("data").and_then(Value::as_object_mut) else {
        return response;
    };
    data.insert(
        "presentationKeeper".to_string(),
        serde_json::to_value(status).unwrap_or_else(|_| {
            serde_json::to_value(super::presentation_runtime_status::unavailable_status(
                "presentation_keeper_status_unavailable",
            ))
            .expect("presentation keeper unavailable status must serialize")
        }),
    );
    response
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
    router.initialize_runtime_host(session).await?;
    let browser_session_reaper = spawn_browser_session_reaper(router.clone());

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
                break;
            }
            _ = shutdown_signal() => {
                break;
            }
        }
    }

    let shutdown_result = router.shutdown().await;
    browser_session_reaper.abort();
    let _ = browser_session_reaper.await;
    shutdown_result?;

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
    router.initialize_runtime_host(session).await?;
    let browser_session_reaper = spawn_browser_session_reaper(router.clone());

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
                let _ = fs::remove_file(&port_path);
                break;
            }
            _ = reset_rx.recv(), if idle_timeout_ms.is_some() => {
                idle_sleep_pin = idle_timeout_ms
                    .map(|ms| Box::pin(tokio::time::sleep(Duration::from_millis(ms))));
                continue;
            }
            _ = close_notify.notified() => {
                let _ = fs::remove_file(&port_path);
                break;
            }
            _ = shutdown_signal() => {
                let _ = fs::remove_file(&port_path);
                break;
            }
        }
    }

    let shutdown_result = router.shutdown().await;
    browser_session_reaper.abort();
    let _ = browser_session_reaper.await;
    shutdown_result?;

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
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let (line_tx, mut line_rx) = mpsc::channel(CONNECTION_READ_AHEAD_CAPACITY);
    let (disconnect_tx, mut disconnect_rx) = watch::channel(false);
    // Read independently from command execution so EOF can revoke this exact
    // transport's profile ownership while its queued work reaches a terminal
    // state in the control-plane worker.
    let reader_task = tokio::spawn(async move {
        let mut buf_reader = BufReader::new(reader);
        loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) if line_tx.send(line).await.is_err() => return,
                Ok(_) => {}
            }
        }
        let _ = disconnect_tx.send(true);
    });
    let connection_instance_id = super::service_connection_lifetime::new_connection_id();
    let _disconnect_guard = ProfileConnectionDisconnectGuard(&connection_instance_id);

    'connection: loop {
        match line_rx.recv().await {
            None => break,
            Some(line) => {
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
                let action = cmd
                    .get("action")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned);
                if action.as_deref() == Some("service_remote_view_handoff_resolve") {
                    if let Some(response) = router.try_resolve_manager_handoff(cmd.clone()).await {
                        if let Some(ref tx) = idle_reset_tx {
                            let _ = tx.try_send(());
                        }
                        let mut serialized = serialize_daemon_response(response).await;
                        serialized.push('\n');
                        if writer.write_all(serialized.as_bytes()).await.is_err() {
                            break;
                        }
                        continue;
                    }
                }
                if action.as_deref() == Some("view_focus") {
                    if let Some(response) =
                        router.try_handle_browser_session_focus(cmd.clone()).await
                    {
                        if let Some(ref tx) = idle_reset_tx {
                            let _ = tx.try_send(());
                        }
                        let mut serialized = serialize_daemon_response(response).await;
                        serialized.push('\n');
                        if writer.write_all(serialized.as_bytes()).await.is_err() {
                            break;
                        }
                        continue;
                    }
                }
                if action
                    .as_deref()
                    .is_some_and(|action| action.starts_with("browser_session_"))
                {
                    if let Some(ref tx) = idle_reset_tx {
                        let _ = tx.try_send(());
                    }
                    let response = router.handle_browser_session_command(cmd).await;
                    let mut serialized = serialize_daemon_response(response).await;
                    serialized.push('\n');
                    if writer.write_all(serialized.as_bytes()).await.is_err() {
                        break;
                    }
                    continue;
                }
                if action.as_deref().is_some_and(|action| {
                    !super::actions::action_skips_browser_launch(action)
                        && !matches!(action, "tab_new" | "tab_switch" | "window_new")
                }) {
                    if let Some(response) = router
                        .try_handle_managed_browser_command(&lane_session, cmd.clone())
                        .await
                    {
                        if let Some(ref tx) = idle_reset_tx {
                            let _ = tx.try_send(());
                        }
                        let mut serialized = serialize_daemon_response(response).await;
                        serialized.push('\n');
                        if writer.write_all(serialized.as_bytes()).await.is_err() {
                            break;
                        }
                        continue;
                    }
                }
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
                crate::runtime_host::reconcile_lane_profile_defaults(&mut cmd, &lane.config);
                if !super::actions::action_skips_browser_launch(
                    cmd["action"].as_str().unwrap_or(""),
                ) || matches!(
                    cmd["action"].as_str(),
                    Some("close" | "tab_close" | "tab_handle_release" | "tab_handle_refresh")
                ) {
                    super::service_request_provenance::attribute_native_session(
                        &mut cmd,
                        &lane_session,
                    );
                }

                if let Some(ref tx) = idle_reset_tx {
                    let _ = tx.try_send(());
                }

                let exits_daemon = matches!(
                    action.as_deref(),
                    Some("close" | "runtime_handoff_finalize" | "runtime_handoff_rollback")
                );

                let response_future = async {
                    if action.as_deref() == Some("worker_status") {
                        let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        control_plane.status_response(id)
                    } else if action.as_deref() == Some("service_job_cancel") {
                        let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let job_id = cmd.get("jobId").and_then(|v| v.as_str()).unwrap_or("");
                        let reason = cmd.get("reason").and_then(|v| v.as_str());
                        control_plane.cancel_job_response(id, job_id, reason)
                    } else if action.as_deref() == Some("service_status") {
                        let id = cmd.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let service_state = cmd
                            .get("serviceState")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let launch_config =
                            super::service_status_projection::launch_configuration_from_status_command(
                                &cmd,
                            );
                        let full_tab_history = cmd
                            .get("fullTabHistory")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        let service_state_projection =
                            super::service_status_projection::service_state_projection_from_status_command(
                                &cmd,
                            );
                        control_plane
                            .service_status_response(
                                id,
                                service_state,
                                launch_config,
                                full_tab_history,
                                service_state_projection,
                            )
                            .await
                    } else {
                        control_plane
                            .submit_from_connection(cmd, &connection_instance_id)
                            .await
                    }
                };
                tokio::pin!(response_future);
                let mut response = if exits_daemon {
                    response_future.await
                } else {
                    tokio::select! {
                        biased;
                        response = &mut response_future => response,
                        _ = async {
                            if !*disconnect_rx.borrow() {
                                let _ = disconnect_rx.changed().await;
                            }
                        } => break 'connection,
                    }
                };
                if action.as_deref() == Some("service_status") {
                    response = router.attach_browser_session_state(response).await;
                }

                let mut resp = serialize_daemon_response(response).await;
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
                    if !crate::runtime_host::admission_enabled() {
                        // Legacy daemons exit with their lane; shared hosts accept future lanes.
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        close_notify.notify_one();
                    }
                    break;
                }
            }
        }
    }
    reader_task.abort();
}

async fn serialize_daemon_response(response: Value) -> String {
    tokio::task::spawn_blocking(move || serde_json::to_string(&response))
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_else(|| {
            r#"{"success":false,"error":"Daemon response serialization failed"}"#.to_string()
        })
}

struct ProfileConnectionDisconnectGuard<'a>(&'a str);

impl Drop for ProfileConnectionDisconnectGuard<'_> {
    fn drop(&mut self) {
        if let Err(error) = super::control_plane::persist_profile_connection_disconnected(self.0) {
            let _ = writeln!(
                std::io::stderr(),
                "Could not mark service connection {} disconnected: {error}",
                self.0
            );
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
    use super::super::browser_session_store::{
        BrowserRuntimeSqliteStore, LegacyBrowserRuntimeSources,
    };
    use super::super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    #[allow(unused_imports)]
    use super::*;
    use agent_browser_service_model::{
        BrowserDesktopAssignment, BrowserSessionState, ControlInputProvider,
        ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab, RemoteViewHandoff,
        ViewStreamProvider,
    };

    #[test]
    fn journaled_open_routing_preserves_explicit_display_and_existing_replay() {
        assert!(should_journal_browser_open(true, true, false, true, false));
        assert!(!should_journal_browser_open(true, true, true, true, false));
        assert!(!should_journal_browser_open(
            true, false, false, true, false
        ));
        assert!(!should_journal_browser_open(
            true, true, false, false, false
        ));
        assert!(should_journal_browser_open(true, true, true, false, true));
    }

    #[test]
    fn remote_open_requires_at_least_one_ready_keeper_route() {
        assert_eq!(
            require_keeper_handoff_resolution(true, &[]),
            Err("browser_session_presentation_route_unavailable")
        );
        assert_eq!(
            require_keeper_handoff_resolution(
                true,
                &[agent_browser_service_model::BrowserDesktopRoute {
                    id: "route-slot-01".to_string(),
                    display_name: ":10".to_string(),
                    healthy: true,
                }],
            ),
            Ok(())
        );
        assert_eq!(require_keeper_handoff_resolution(false, &[]), Ok(()));
    }

    #[tokio::test]
    async fn presentation_keeper_wait_retries_recovery_until_ready() {
        let observations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = observations.clone();

        let result = wait_for_presentation_keeper_observation(
            Instant::now() + Duration::from_secs(1),
            move || {
                let index = observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move {
                    Ok(if index == 0 {
                        PresentationKeeperObservation::Pending
                    } else {
                        PresentationKeeperObservation::Ready("ready")
                    })
                }
            },
        )
        .await;

        assert_eq!(result, Ok("ready"));
        assert_eq!(observations.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn presentation_keeper_wait_returns_terminal_failure_without_retry() {
        let observations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = observations.clone();

        let result: Result<(), String> = wait_for_presentation_keeper_observation(
            Instant::now() + Duration::from_secs(1),
            move || {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async {
                    Ok(PresentationKeeperObservation::Terminal(
                        "presentation_keeper_failed".to_string(),
                    ))
                }
            },
        )
        .await;

        assert_eq!(result, Err("presentation_keeper_failed".to_string()));
        assert_eq!(observations.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn presentation_keeper_wait_deadline_starts_no_observation() {
        let observations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = observations.clone();

        let result: Result<(), String> = wait_for_presentation_keeper_observation(
            Instant::now() - Duration::from_millis(1),
            move || {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok(PresentationKeeperObservation::Ready(())) }
            },
        )
        .await;

        assert_eq!(
            result,
            Err("presentation_keeper_deadline_exceeded".to_string())
        );
        assert_eq!(observations.load(std::sync::atomic::Ordering::SeqCst), 0);

        let stalled: Result<(), String> = wait_for_presentation_keeper_observation(
            Instant::now() + Duration::from_millis(10),
            || std::future::pending(),
        )
        .await;
        assert_eq!(
            stalled,
            Err("presentation_keeper_deadline_exceeded".to_string())
        );
    }

    #[tokio::test]
    async fn manager_handoff_resolution_loads_sqlite_keeper_authority_before_focus() {
        let guard =
            crate::test_utils::EnvGuard::new(&["HOME", "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME"]);
        let home = std::env::temp_dir().join(format!(
            "ab-manager-handoff-daemon-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");

        let service_state_path = super::super::service_store::default_service_state_path().unwrap();
        let service_directory = service_state_path.parent().unwrap();
        fs::create_dir_all(service_directory).unwrap();
        fs::write(
            &service_state_path,
            serde_json::json!({
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": home.join("work"),
                        "profileClass": "durable_named"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let session_state_path = service_directory.join("browser-session-state.json");
        let profile_catalog_path = service_directory.join("browser-profile-catalog.json");
        let database_path = BrowserRuntimeSqliteStore::default_sqlite_path().unwrap();
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &session_state_path,
                profile_catalog_path: &profile_catalog_path,
                service_state_path: &service_state_path,
            },
        )
        .unwrap();

        let mut state = BrowserSessionState::default();
        state.sessions.insert(
            "session-a".to_string(),
            ManagedBrowserSession {
                id: "session-a".to_string(),
                name: "alice".to_string(),
                profile_id: "work".to_string(),
                browser_id: "browser-a".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: u64::MAX,
                current_tab_id: Some("tab-a".to_string()),
            },
        );
        state.browsers.insert(
            "browser-a".to_string(),
            ManagedBrowserInstance {
                id: "browser-a".to_string(),
                profile_id: "work".to_string(),
                pid: std::process::id(),
                cdp_endpoint: "http://127.0.0.1:1".to_string(),
                process_identity: None,
                desktop: Some(BrowserDesktopAssignment {
                    route_id: "route-slot-01".to_string(),
                    display_name: ":10".to_string(),
                    live_browser_count: 0,
                }),
                active_session_ids: vec!["session-a".to_string()],
            },
        );
        state.tabs.insert(
            "tab-a".to_string(),
            ManagedBrowserTab {
                id: "tab-a".to_string(),
                target_id: "target-a".to_string(),
                browser_id: "browser-a".to_string(),
                session_id: "session-a".to_string(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
        let handoff = RemoteViewHandoff {
            id: "handoff-a".to_string(),
            state: "ready".to_string(),
            intent: serde_json::json!({
                "browserSessionManager": true,
                "sessionId": "session-a",
                "presentationSlotId": "route-slot-01"
            }),
            handoff_url: Some("https://dashboard.example/remote-view/handoff-a".to_string()),
            profile_id: Some("work".to_string()),
            browser_id: Some("browser-a".to_string()),
            session_name: Some("alice".to_string()),
            tab_id: Some("tab-a".to_string()),
            target_id: Some("target-a".to_string()),
            view_stream_provider: Some(ViewStreamProvider::RdpGateway),
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            last_route_id: Some("route-slot-01".to_string()),
            ..RemoteViewHandoff::default()
        };
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        store.save_session_state(&state).unwrap();
        store.save_manager_handoff(&handoff).unwrap();

        let router = RuntimeHostRouter::new(
            home.join("socket"),
            "cold",
            None,
            None,
            None,
            RuntimeHostWorkerOptions {
                service_reconcile_interval_ms: None,
                service_job_timeout_ms: None,
                service_monitor_interval_ms: None,
            },
        )
        .unwrap();
        let response = router
            .try_resolve_manager_handoff(serde_json::json!({
                "id": "resolve-a",
                "action": "service_remote_view_handoff_resolve",
                "params": { "handoffId": "handoff-a" }
            }))
            .await
            .unwrap();

        assert_eq!(response["success"], false);
        assert_eq!(response["error"], "presentation_keeper_unavailable");
        assert_eq!(
            BrowserRuntimeSqliteStore::open(&database_path)
                .unwrap()
                .load_session_state()
                .unwrap()
                .sessions["session-a"]
                .last_activity_at_ms,
            1
        );

        router.shutdown().await.unwrap();
        let _ = fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn closing_last_runtime_lane_does_not_stop_shared_host() {
        let guard = crate::test_utils::EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_RUNTIME_HOST",
            "AGENT_BROWSER_SESSION_SUPERVISOR_ROOT",
        ]);
        let home = std::env::temp_dir().join(format!("ab-last-lane-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        guard.set(
            "AGENT_BROWSER_HOME",
            home.join("agent-home").to_str().unwrap(),
        );
        guard.set("AGENT_BROWSER_RUNTIME_HOST", "1");
        guard.set(
            "AGENT_BROWSER_SESSION_SUPERVISOR_ROOT",
            home.join("supervisor").to_str().unwrap(),
        );
        let router = RuntimeHostRouter::new(
            home.clone(),
            "cold",
            None,
            None,
            None,
            RuntimeHostWorkerOptions {
                service_reconcile_interval_ms: None,
                service_job_timeout_ms: None,
                service_monitor_interval_ms: None,
            },
        )
        .unwrap();
        let notify = Arc::new(Notify::new());
        let (client, server) = tokio::io::duplex(8192);
        let task = tokio::spawn(handle_connection(
            server,
            router.clone(),
            "cold",
            None,
            None,
            notify.clone(),
            Arc::new("fixture-auth".into()),
        ));
        let (reader, mut writer) = tokio::io::split(client);
        writer.write_all(b"{\"id\":\"close-last\",\"action\":\"close\",\"_agentBrowserAuthToken\":\"fixture-auth\"}\n").await.unwrap();
        let mut response = String::new();
        BufReader::new(reader)
            .read_line(&mut response)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&response).unwrap()["success"],
            true
        );
        task.await.unwrap();
        assert!(router.lanes.is_empty());
        assert!(
            tokio::time::timeout(Duration::from_millis(10), notify.notified())
                .await
                .is_err(),
            "closing the last lane must not stop the shared host"
        );
        let (keeper, keeper_shutdowns) =
            crate::native::stream::configured_route_keeper_supervisor_fixture();
        router.install_route_keeper_owner(keeper).await.unwrap();
        let (duplicate, duplicate_shutdowns) =
            crate::native::stream::configured_route_keeper_supervisor_fixture();
        assert_eq!(
            router.install_route_keeper_owner(duplicate).await,
            Err("runtime_host_route_keeper_already_installed".to_string())
        );
        assert_eq!(
            duplicate_shutdowns.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        router.shutdown().await.unwrap();
        assert_eq!(
            keeper_shutdowns.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        router.shutdown().await.unwrap();
        assert_eq!(
            keeper_shutdowns.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[tokio::test]
    async fn route_keeper_initialization_failure_shuts_down_preloaded_lanes() {
        let guard = crate::test_utils::EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_RUNTIME_HOST",
            "AGENT_BROWSER_SESSION_SUPERVISOR_ROOT",
        ]);
        let home = std::env::temp_dir().join(format!(
            "ab-route-keeper-initialization-failure-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        guard.set(
            "AGENT_BROWSER_HOME",
            home.join("agent-home").to_str().unwrap(),
        );
        guard.set("AGENT_BROWSER_RUNTIME_HOST", "1");
        guard.set(
            "AGENT_BROWSER_SESSION_SUPERVISOR_ROOT",
            home.join("supervisor").to_str().unwrap(),
        );
        let router = RuntimeHostRouter::new(
            home.clone(),
            "cold",
            None,
            None,
            None,
            RuntimeHostWorkerOptions {
                service_reconcile_interval_ms: None,
                service_job_timeout_ms: None,
                service_monitor_interval_ms: None,
            },
        )
        .unwrap();

        let error = router.initialize_runtime_host("cold").await.unwrap_err();
        assert!(
            error.starts_with("browser_runtime_database_missing:"),
            "unexpected initialization error: {error}"
        );
        assert!(router.lanes.is_empty());
        assert!(router.route_keeper.lock().await.is_none());
        fs::remove_dir_all(home).unwrap();
    }

    #[tokio::test]
    async fn disconnected_client_releases_connection_while_command_is_running() {
        let guard = crate::test_utils::EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_RUNTIME_HOST",
            "AGENT_BROWSER_SESSION_SUPERVISOR_ROOT",
        ]);
        let home =
            std::env::temp_dir().join(format!("ab-disconnected-command-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&home).unwrap();
        guard.set("HOME", home.to_str().unwrap());
        guard.set(
            "AGENT_BROWSER_HOME",
            home.join("agent-home").to_str().unwrap(),
        );
        guard.set("AGENT_BROWSER_RUNTIME_HOST", "1");
        guard.set(
            "AGENT_BROWSER_SESSION_SUPERVISOR_ROOT",
            home.join("supervisor").to_str().unwrap(),
        );
        let router = RuntimeHostRouter::new(
            home.clone(),
            "disconnect-test",
            None,
            None,
            None,
            RuntimeHostWorkerOptions {
                service_reconcile_interval_ms: None,
                service_job_timeout_ms: Some(1_000),
                service_monitor_interval_ms: None,
            },
        )
        .unwrap();
        let notify = Arc::new(Notify::new());
        let (mut client, server) = tokio::io::duplex(8192);
        let task = tokio::spawn(handle_connection(
            server,
            router.clone(),
            "disconnect-test",
            None,
            None,
            notify,
            Arc::new("fixture-auth".into()),
        ));

        let command = serde_json::json!({
            "id": "pending-command",
            "action": "dependent_batch",
            "bail": true,
            "commands": [
                {"id": "step-1", "action": "__test_sleep", "ms": 250},
                {"id": "step-2", "action": "__test_sleep", "ms": 250}
            ],
            "_agentBrowserAuthToken": "fixture-auth"
        });
        let mut encoded = serde_json::to_vec(&command).unwrap();
        encoded.push(b'\n');
        client.write_all(&encoded).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(client);

        tokio::time::timeout(Duration::from_millis(150), task)
            .await
            .expect("client EOF must release its connection before the command finishes")
            .unwrap();

        router.shutdown().await.unwrap();
        let snapshot = LockedServiceStateRepository::default_json()
            .unwrap()
            .load_snapshot()
            .unwrap();
        let job = snapshot
            .jobs
            .get("pending-command")
            .expect("disconnected command must still reach a terminal job state");
        assert_eq!(job.state, super::super::service_model::JobState::Succeeded);
        assert_eq!(
            job.result.as_ref().unwrap()["success"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            job.terminal_outcome.as_ref().unwrap().state,
            super::super::service_terminal_outcome::ServiceTerminalState::Succeeded
        );
        fs::remove_dir_all(home).unwrap();
    }

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn daemon_serializes_two_clients_and_500_large_reads_without_starving_runtime() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::sync::Arc;

        let running = Arc::new(AtomicBool::new(true));
        let heartbeat_count = Arc::new(AtomicUsize::new(0));
        let heartbeat_running = running.clone();
        let heartbeat_observations = heartbeat_count.clone();
        let heartbeat = tokio::spawn(async move {
            while heartbeat_running.load(Ordering::Relaxed) {
                heartbeat_observations.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
            }
        });

        let client = |client_id: usize| async move {
            for ordinal in 0..250 {
                let response = serde_json::json!({
                    "success": true,
                    "clientId": client_id,
                    "ordinal": ordinal,
                    "data": "x".repeat(32 * 1024),
                });
                let serialized = serialize_daemon_response(response).await;
                assert!(serialized.starts_with("{\"clientId\":"));
                assert!(serialized.len() > 32 * 1024);
            }
        };
        tokio::time::timeout(Duration::from_secs(15), async {
            tokio::join!(client(1), client(2));
        })
        .await
        .expect("two constrained-runtime clients must finish all 500 serializations");
        running.store(false, Ordering::Relaxed);
        heartbeat.await.unwrap();

        assert!(
            heartbeat_count.load(Ordering::Relaxed) > 100,
            "the two-worker runtime must continue scheduling unrelated work"
        );
    }

    #[test]
    fn service_status_adds_browser_session_state_without_replacing_legacy_state() {
        let response = serde_json::json!({
            "id": "status-1",
            "success": true,
            "data": {
                "service_state": { "browsers": { "legacy": { "id": "legacy" } } }
            }
        });
        let state = serde_json::json!({
            "schemaVersion": "agent-browser.browser-session-state.v1",
            "browsers": { "browser:work:1": { "id": "browser:work:1" } }
        });

        let joined = attach_browser_session_state_to_status(response, Ok(state));

        assert_eq!(
            joined["data"]["service_state"]["browsers"]["legacy"]["id"],
            "legacy"
        );
        assert_eq!(
            joined["data"]["browserSessionState"]["browsers"]["browser:work:1"]["id"],
            "browser:work:1"
        );
    }

    #[test]
    fn service_status_reports_nonblocking_browser_session_projection_failure() {
        let response = serde_json::json!({ "success": true, "data": {} });

        let joined = attach_browser_session_state_to_status(
            response,
            Err("browser-session-state unreadable".to_string()),
        );

        assert_eq!(joined["success"], true);
        assert!(joined["data"]["browserSessionState"].is_null());
        assert_eq!(
            joined["data"]["browserSessionStateError"],
            "browser-session-state unreadable"
        );
    }

    #[test]
    fn service_status_adds_presentation_keeper_diagnostics() {
        let response = serde_json::json!({ "success": true, "data": {} });
        let joined = attach_presentation_keeper_status(
            response,
            crate::native::presentation_runtime_status::unavailable_status(
                "presentation_keeper_unavailable",
            ),
        );

        assert_eq!(joined["success"], true);
        assert_eq!(joined["data"]["presentationKeeper"]["state"], "unavailable");
        assert_eq!(
            joined["data"]["presentationKeeper"]["supervisor"]["code"],
            "presentation_keeper_unavailable"
        );
    }

    #[test]
    fn browser_session_reaper_uses_service_reconcile_interval() {
        assert_eq!(browser_session_reap_interval_ms(None), 30_000);
        assert_eq!(browser_session_reap_interval_ms(Some(1_250)), 1_250);
        assert_eq!(browser_session_reap_interval_ms(Some(0)), 1);
    }

    #[test]
    fn view_focus_routes_only_manager_owned_browser_ids() {
        let mut state = agent_browser_service_model::BrowserSessionState::default();
        state.browsers.insert(
            "browser-work".to_string(),
            agent_browser_service_model::ManagedBrowserInstance {
                id: "browser-work".to_string(),
                profile_id: "work".to_string(),
                pid: 4242,
                cdp_endpoint: "ws://127.0.0.1:9422/devtools/browser/test".to_string(),
                process_identity: None,
                desktop: None,
                active_session_ids: Vec::new(),
            },
        );
        let owned = serde_json::json!({
            "action": "view_focus",
            "browserId": "browser-work",
            "targetId": "target-1"
        });
        let foreign = serde_json::json!({
            "action": "view_focus",
            "browserId": "legacy-browser"
        });

        let routed = browser_session_focus_command(&owned, &state).unwrap();
        assert_eq!(routed["action"], "browser_session_focus");
        assert_eq!(routed["targetId"], "target-1");
        assert!(browser_session_focus_command(&foreign, &state).is_none());
    }
}
