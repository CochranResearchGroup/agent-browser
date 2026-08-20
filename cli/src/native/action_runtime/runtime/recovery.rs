#![allow(unused_imports)]
use super::capability::service_browser_id;
use super::cdp_free_plan::{
    browser_host_from_command, optional_command_string,
    remote_headed_display_isolation_from_command,
};
use super::daemon::{
    debug_session_events_enabled, launch_hash, BackendType, CloseBehavior, DrainedEvents,
    FetchPausedRequest, HarEntry, MouseState, PendingConfirmation, PendingDialog, RouteEntry,
    TrackedRequest,
};
use super::launch::browser_recovery_policy_config_from_env;
use super::profile_lease::{
    allow_duplicate_profile_lane_from_command, service_browser_health_counts_as_live,
};
use crate::native::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::cancellation::CancellationToken;
use crate::native::cdp::chrome::{launch_chrome_detached, LaunchOptions, ManualChromeLaunch};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::{
    AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
    DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
    TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
};
use crate::native::element::RefMap;
use crate::native::inspect_server::InspectServer;
use crate::native::network::resolve_fetch_paused;
use crate::native::network::{self, DomainFilter, EventTracker};
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::policy::{ActionPolicy, ConfirmActions, PolicyResult};
use crate::native::recording::{self, RecordingState};
use crate::native::service_health::{
    persist_browser_recovery_started_in_repository, persist_closed_browser_health_in_repository,
    persist_current_browser_stale_health_in_repository,
    persist_reconciled_service_state_in_repository, persist_service_browser_record_in_repository,
    reconcile_service_state, retry_degraded_service_browser_in_state,
    retry_persisted_service_browser_in_repository, retry_service_browser_in_state,
    BrowserRecoveryPersistence, BrowserRecoveryPolicyConfig, BrowserRecoveryPolicySource,
    BrowserRecoveryPolicyValueSource, BrowserRecoveryReasonKind,
};
use crate::native::service_lifecycle::{
    profile_lease_telemetry, select_service_profile_for_request, service_profile_id,
    ProfileSelectionRequest, ServiceLaunchMetadata,
};
use crate::native::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStream,
    ViewStreamProvider, ViewerLease,
};
use crate::native::service_renderer_crash::{
    correlate_renderer_crash, persist_renderer_crash_in_repository, renderer_crash_targets_context,
    RendererCrashCommandContext, RendererCrashObservation, RendererCrashPersistence,
    RendererCrashSignal,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::storage;
use crate::native::stream::{self, StreamServer};
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use crate::native::tracing::{self as native_tracing, TracingState};
use crate::native::webdriver::appium::AppiumManager;
use crate::native::webdriver::backend::{
    BrowserBackend, WebDriverBackend, WEBDRIVER_UNSUPPORTED_ACTIONS,
};
use crate::native::webdriver::safari;
use crate::runtime_profile::{
    clear_runtime_state, looks_like_path, read_devtools_port, read_runtime_state,
    runtime_profile_user_data_dir,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, RwLock};
pub(crate) fn persist_browser_recovery_started_from_persisted_state(
    state: &DaemonState,
    reason: &str,
) -> BrowserRecoveryPersistence {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        return persist_browser_recovery_started_in_repository(
            &repository,
            &state.session_id,
            state.browser_recovery_policy_config,
            reason,
        );
    }
    BrowserRecoveryPersistence::NotRecorded
}
pub(crate) fn persist_closed_browser_health(
    state: &DaemonState,
    outcome: Option<&BrowserShutdownOutcome>,
) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ =
            persist_closed_browser_health_in_repository(&repository, &state.session_id, outcome);
    }
}
pub(crate) struct DaemonState {
    pub browser: Option<BrowserManager>,
    pub appium: Option<AppiumManager>,
    pub safari_driver: Option<safari::SafariDriverProcess>,
    pub webdriver_backend: Option<super::super::super::webdriver::backend::WebDriverBackend>,
    pub backend_type: BackendType,
    pub ref_map: RefMap,
    pub domain_filter: Arc<RwLock<Option<DomainFilter>>>,
    pub event_tracker: EventTracker,
    pub session_name: Option<String>,
    pub session_id: String,
    pub tracing_state: TracingState,
    pub recording_state: RecordingState,
    pub(crate) event_rx: Option<broadcast::Receiver<CdpEvent>>,
    pub screencasting: bool,
    pub policy: Option<ActionPolicy>,
    pub pending_confirmation: Option<PendingConfirmation>,
    pub har_recording: bool,
    pub har_entries: Vec<HarEntry>,
    pub confirm_actions: Option<ConfirmActions>,
    pub inspect_server: Option<InspectServer>,
    pub routes: Arc<RwLock<Vec<RouteEntry>>>,
    pub tracked_requests: Vec<TrackedRequest>,
    pub request_tracking: bool,
    pub active_frame_id: Option<String>,
    /// Cross-origin iframe frame_id → dedicated CDP session_id.
    /// Populated by Target.attachedToTarget events from Target.setAutoAttach.
    pub iframe_sessions: HashMap<String, String>,
    /// Origin-scoped extra HTTP headers set via `--headers` on navigate.
    /// Key is the origin (scheme + host + port), value is the headers map.
    /// Wrapped in Arc<RwLock<>> so the background Fetch handler can read it.
    pub origin_headers: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    /// Proxy authentication credentials (username, password) for handling
    /// Fetch.authRequired events from authenticated proxies.
    pub proxy_credentials: Arc<RwLock<Option<(String, String)>>>,
    /// Background task that processes Fetch.requestPaused events in real-time,
    /// handling domain filtering, route interception, and origin-scoped headers
    /// without deadlocking navigation/evaluate.
    pub(crate) fetch_handler_task: Option<tokio::task::JoinHandle<()>>,
    /// Background task that auto-accepts `alert` and `beforeunload` dialogs
    /// so they never block the agent.
    pub(crate) dialog_handler_task: Option<tokio::task::JoinHandle<()>>,
    pub mouse_state: MouseState,
    /// Tracks the currently open JavaScript dialog (alert/confirm/prompt), if any.
    pub pending_dialog: Option<PendingDialog>,
    /// When true, automatically dismiss `beforeunload` dialogs and accept `alert`
    /// dialogs so they never block the agent.  Enabled by default.
    pub auto_dialog: bool,
    /// Shared slot for stream server to receive CDP client when browser launches.
    pub stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
    /// Stream server instance kept alive so the broadcast channel remains open.
    pub stream_server: Option<Arc<StreamServer>>,
    /// Hash of launch options used for the current browser, for relaunch detection.
    pub(crate) launch_hash: Option<u64>,
    /// Runtime profile for a browser attached through CDP that this daemon owns logically.
    pub(crate) attached_runtime_profile: Option<String>,
    /// Process ID for an attached runtime-profile browser, used for explicit close.
    pub(crate) attached_browser_pid: Option<u32>,
    /// Whether closing this daemon session should shut down the browser or detach.
    pub(crate) close_behavior: CloseBehavior,
    /// Browser engine name (e.g. "chrome", "lightpanda") for observability.
    pub engine: String,
    /// Default timeout for wait operations, from AGENT_BROWSER_DEFAULT_TIMEOUT env var.
    pub default_timeout_ms: u64,
    /// Retry budget and backoff used when a stale browser is relaunched.
    pub browser_recovery_policy_config: BrowserRecoveryPolicyConfig,
    /// Cancellation token for the currently running service job, if this
    /// command is executing inside the service control-plane worker.
    pub current_cancellation: Option<CancellationToken>,
    /// Launch-time shared-profile acquisition evidence to attach to the next
    /// command response that consumes the auto-launched tab.
    pub(crate) pending_shared_profile_acquisition: Option<Value>,
    /// Present only while this daemon participates in a generation-bound
    /// browser owner transfer. Browser effects then fail closed against the
    /// locked service-state owner generation.
    pub(crate) runtime_owner_binding: Option<crate::runtime_owner_transfer::RuntimeOwnerBinding>,
    /// Storage mutations made through agent-browser storage commands, keyed by origin.
    /// This preserves cross-origin storage for state saves even after navigation.
    pub(crate) tracked_origin_storage: HashMap<String, state::OriginStorage>,
}
impl DaemonState {
    pub fn new() -> Self {
        Self {
            browser: None,
            appium: None,
            safari_driver: None,
            webdriver_backend: None,
            backend_type: BackendType::Cdp,
            ref_map: RefMap::new(),
            domain_filter: Arc::new(RwLock::new(
                env::var("AGENT_BROWSER_ALLOWED_DOMAINS")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(|s| DomainFilter::new(&s)),
            )),
            event_tracker: EventTracker::new(),
            session_name: env::var("AGENT_BROWSER_SESSION_NAME").ok(),
            session_id: env::var("AGENT_BROWSER_SESSION").unwrap_or_else(|_| "default".to_string()),
            tracing_state: TracingState::new(),
            recording_state: RecordingState::new(),
            event_rx: None,
            screencasting: false,
            policy: ActionPolicy::load_if_exists(),
            pending_confirmation: None,
            har_recording: false,
            har_entries: Vec::new(),
            confirm_actions: ConfirmActions::from_env(),
            inspect_server: None,
            routes: Arc::new(RwLock::new(Vec::new())),
            tracked_requests: Vec::new(),
            request_tracking: false,
            active_frame_id: None,
            iframe_sessions: HashMap::new(),
            origin_headers: Arc::new(RwLock::new(HashMap::new())),
            proxy_credentials: Arc::new(RwLock::new(None)),
            fetch_handler_task: None,
            dialog_handler_task: None,
            mouse_state: MouseState::default(),
            pending_dialog: None,
            auto_dialog: !matches!(
                env::var("AGENT_BROWSER_NO_AUTO_DIALOG").as_deref(),
                Ok("1" | "true" | "yes")
            ),
            stream_client: None,
            stream_server: None,
            launch_hash: None,
            attached_runtime_profile: None,
            attached_browser_pid: None,
            close_behavior: CloseBehavior::CloseBrowser,
            engine: env::var("AGENT_BROWSER_ENGINE").unwrap_or_else(|_| "chrome".to_string()),
            default_timeout_ms: env::var("AGENT_BROWSER_DEFAULT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30_000),
            browser_recovery_policy_config: browser_recovery_policy_config_from_env(),
            current_cancellation: None,
            pending_shared_profile_acquisition: None,
            runtime_owner_binding: None,
            tracked_origin_storage: HashMap::new(),
        }
    }
    /// Extract the timeout from a command JSON, falling back to the
    /// configured `default_timeout_ms` (from `AGENT_BROWSER_DEFAULT_TIMEOUT`).
    /// All wait-family handlers should use this instead of reading the
    /// timeout field and providing their own fallback.
    pub(crate) fn timeout_ms(&self, cmd: &Value) -> u64 {
        cmd.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout_ms)
    }
    pub(crate) fn reset_input_state(&mut self) {
        self.mouse_state = MouseState::default();
    }
    /// Create state with an optional stream client slot and server instance
    /// (for daemon startup with stream server).
    pub fn new_with_stream(
        stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
        stream_server: Option<Arc<StreamServer>>,
    ) -> Self {
        let mut s = Self::new();
        if stream_server.is_some() {
            s.request_tracking = true;
        }
        s.stream_client = stream_client;
        s.stream_server = stream_server;
        s
    }

    /// Create one isolated logical lane inside the user-scoped runtime host.
    pub(crate) fn new_for_session_with_stream(
        session_id: &str,
        stream_client: Option<Arc<RwLock<Option<Arc<CdpClient>>>>>,
        stream_server: Option<Arc<StreamServer>>,
    ) -> Self {
        let mut state = Self::new_with_stream(stream_client, stream_server);
        state.session_id = session_id.to_string();
        state
    }
    pub(crate) fn subscribe_to_browser_events(&mut self) {
        if let Some(ref browser) = self.browser {
            self.event_rx = Some(browser.client.subscribe());
        }
    }
    /// Start the background task that processes Fetch.requestPaused and
    /// Fetch.authRequired events in real-time (domain filtering, route
    /// interception, origin-scoped headers, proxy authentication).
    /// Must be called after the browser is set and events are subscribed.
    pub(crate) fn start_fetch_handler(&mut self) {
        if let Some(task) = self.fetch_handler_task.take() {
            task.abort();
        }
        let Some(ref browser) = self.browser else {
            return;
        };
        let client = browser.client.clone();
        let mut rx = browser.client.subscribe();
        let domain_filter = self.domain_filter.clone();
        let routes = self.routes.clone();
        let origin_headers = self.origin_headers.clone();
        let proxy_credentials = self.proxy_credentials.clone();
        self.fetch_handler_task = Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) if event.method == "Fetch.authRequired" => {
                        let request_id = event
                            .params
                            .get("requestId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let sid = event.session_id.clone().unwrap_or_default();
                        let creds = proxy_credentials.read().await;
                        if let Some((ref user, ref pass)) = *creds {
                            let _ = client
                                .send_command(
                                    "Fetch.continueWithAuth",
                                    Some(json!(
                                        { "requestId" : request_id, "authChallengeResponse" : {
                                        "response" : "ProvideCredentials", "username" : user,
                                        "password" : pass, } }
                                    )),
                                    Some(&sid),
                                )
                                .await;
                        } else {
                            let _ = client
                                .send_command(
                                    "Fetch.continueWithAuth",
                                    Some(json!(
                                        { "requestId" : request_id, "authChallengeResponse" : {
                                        "response" : "CancelAuth", } }
                                    )),
                                    Some(&sid),
                                )
                                .await;
                        }
                    }
                    Ok(event) if event.method == "Fetch.requestPaused" => {
                        let request_id = event
                            .params
                            .get("requestId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let request_url = event
                            .params
                            .get("request")
                            .and_then(|r| r.get("url"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let resource_type = event
                            .params
                            .get("resourceType")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let request_headers = event
                            .params
                            .get("request")
                            .and_then(|r| r.get("headers"))
                            .and_then(|h| h.as_object())
                            .cloned();
                        let sid = event.session_id.clone().unwrap_or_default();
                        let paused = FetchPausedRequest {
                            request_id,
                            url: request_url,
                            resource_type,
                            session_id: sid,
                            request_headers,
                        };
                        let df = domain_filter.read().await;
                        let rt = routes.read().await;
                        let oh = origin_headers.read().await;
                        resolve_fetch_paused(&client, df.as_ref(), &rt, &oh, &paused).await;
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }));
    }
    /// Start the background task that auto-accepts `alert` and `beforeunload`
    /// dialogs so they never block the agent. `confirm` and `prompt` dialogs
    /// are left for the agent to handle explicitly.
    pub(crate) fn start_dialog_handler(&mut self) {
        if let Some(task) = self.dialog_handler_task.take() {
            task.abort();
        }
        if !self.auto_dialog {
            return;
        }
        let Some(ref browser) = self.browser else {
            return;
        };
        let client = browser.client.clone();
        let mut rx = browser.client.subscribe();
        self.dialog_handler_task = Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) if event.method == "Page.javascriptDialogOpening" => {
                        let dialog_type = event
                            .params
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if matches!(dialog_type, "beforeunload" | "alert") {
                            let message = event
                                .params
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            eprintln!("[auto-dismiss] {} dialog: {}", dialog_type, message);
                            let sid = event.session_id.clone().unwrap_or_default();
                            if let Err(e) = client
                                .send_command(
                                    "Page.handleJavaScriptDialog",
                                    Some(json!({ "accept" : true })),
                                    Some(&sid),
                                )
                                .await
                            {
                                eprintln!(
                                    "[auto-dismiss] failed to dismiss {} dialog: {}",
                                    dialog_type, e
                                );
                            }
                        }
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }));
    }
    /// Update the stream server's CDP client slot when browser is set or cleared.
    pub async fn update_stream_client(&self) {
        if let Some(ref slot) = self.stream_client {
            let mut guard = slot.write().await;
            *guard = self.browser.as_ref().map(|m| Arc::clone(&m.client));
        }
        if let Some(ref server) = self.stream_server {
            let session_id = self
                .browser
                .as_ref()
                .and_then(|m| m.active_session_id().ok().map(|s| s.to_string()));
            server.set_cdp_session_id(session_id).await;
            let connected = self.browser.is_some();
            let sc = server.is_screencasting().await;
            let (vw, vh) = server.viewport().await;
            server
                .broadcast_status(connected, sc, vw, vh, &self.engine)
                .await;
            if let Some(ref mgr) = self.browser {
                server.broadcast_tabs(&mgr.tab_list(false)).await;
            } else {
                server.broadcast_tabs(&[]).await;
            }
            server.notify_client_changed();
        }
    }
    pub(crate) async fn try_recover_browser_connection(&mut self) -> Result<bool, String> {
        let Some(browser) = self.browser.as_mut() else {
            return Ok(false);
        };
        if browser.has_process_exited() || browser.is_connection_alive().await {
            return Ok(false);
        }
        browser.reconnect_client().await?;
        self.subscribe_to_browser_events();
        self.start_fetch_handler();
        self.start_dialog_handler();
        self.update_stream_client().await;
        Ok(true)
    }
    /// Spawn a background task that polls screenshots and pipes them to ffmpeg.
    pub(crate) async fn start_recording_task(
        &mut self,
        client: Arc<CdpClient>,
        session_id: String,
    ) -> Result<(), String> {
        let shared_count = Arc::new(AtomicU64::new(0));
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let handle = recording::spawn_recording_task(
            client,
            session_id,
            self.recording_state.output_path.clone(),
            shared_count.clone(),
            cancel_rx,
        );
        self.recording_state.capture_task = Some(handle);
        self.recording_state.shared_frame_count = Some(shared_count);
        self.recording_state.cancel_tx = Some(cancel_tx);
        Ok(())
    }
    pub(crate) async fn stop_recording_task(&mut self) -> Result<(), String> {
        recording::stop_recording_task(&mut self.recording_state).await
    }
    pub async fn drain_cdp_events_background(&mut self) {
        let drained = self.drain_cdp_events();
        self.project_renderer_crashes(&drained.renderer_crashes, None);
        self.apply_drained_events(drained).await;
    }
    pub(crate) async fn drain_cdp_events_for_command(
        &mut self,
        context: &RendererCrashCommandContext,
    ) -> Option<(
        RendererCrashObservation,
        Result<RendererCrashPersistence, String>,
    )> {
        tokio::task::yield_now().await;
        let drained = self.drain_cdp_events();
        let matched = self.project_renderer_crashes(&drained.renderer_crashes, Some(context));
        self.apply_drained_events(drained).await;
        matched
    }
    pub(crate) fn persist_renderer_crash_observation(
        &self,
        observation: &RendererCrashObservation,
    ) -> Result<RendererCrashPersistence, String> {
        let repository = LockedServiceStateRepository::default_json()?;
        persist_renderer_crash_in_repository(&repository, observation)
    }
    pub(crate) fn renderer_crash_command_context(
        &self,
        cmd: &Value,
    ) -> RendererCrashCommandContext {
        let (target_id, page_session_id, detected_profile, pid, endpoint, stderr_path) = self
            .browser
            .as_ref()
            .map(|browser| {
                (
                    browser.active_target_id().ok().map(str::to_string),
                    browser.active_session_id().ok().map(str::to_string),
                    browser.runtime_profile_name().map(str::to_string),
                    browser.browser_pid(),
                    Some(browser.get_cdp_url().to_string()),
                    browser
                        .browser_stderr_log_path()
                        .map(|path| path.display().to_string()),
                )
            })
            .unwrap_or_default();
        RendererCrashCommandContext {
            action: optional_command_string(cmd, "action").unwrap_or_default(),
            request_id: optional_command_string(cmd, "id").unwrap_or_default(),
            local_principal: optional_command_string(cmd, "principal").or_else(|| {
                env::var("USER")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("local:{value}"))
            }),
            service_name: optional_command_string(cmd, "serviceName"),
            agent_name: optional_command_string(cmd, "agentName"),
            task_name: optional_command_string(cmd, "taskName"),
            daemon_session: self.session_id.clone(),
            requested_profile: optional_command_string(cmd, "runtimeProfile")
                .or_else(|| optional_command_string(cmd, "profile"))
                .or_else(|| optional_command_string(cmd, "profileId")),
            detected_profile,
            browser_id: Some(service_browser_id(&self.session_id)),
            pid,
            endpoint,
            browser_build: optional_command_string(cmd, "browserBuild")
                .or_else(|| (!self.engine.trim().is_empty()).then(|| self.engine.clone())),
            stderr_path,
            target_id: target_id.or_else(|| optional_command_string(cmd, "targetId")),
            page_session_id: page_session_id
                .or_else(|| optional_command_string(cmd, "pageSessionId")),
        }
    }
    fn renderer_crash_background_context(
        &self,
        signal: &RendererCrashSignal,
    ) -> RendererCrashCommandContext {
        let mut context = self.renderer_crash_command_context(&json!({
            "action": "background_event",
            "id": ""
        }));
        context.target_id = signal.target_id.clone();
        context.page_session_id = signal.page_session_id.clone().or_else(|| {
            signal.target_id.as_deref().and_then(|target_id| {
                self.browser
                    .as_ref()
                    .and_then(|browser| browser.page_session_for_target(target_id))
                    .map(str::to_string)
            })
        });
        context
    }
    fn project_renderer_crashes(
        &self,
        signals: &[RendererCrashSignal],
        command_context: Option<&RendererCrashCommandContext>,
    ) -> Option<(
        RendererCrashObservation,
        Result<RendererCrashPersistence, String>,
    )> {
        let repository = LockedServiceStateRepository::default_json();
        let mut matched = None;
        for signal in signals {
            let command_target_matches = command_context
                .is_some_and(|context| renderer_crash_targets_context(signal, context));
            let observation = command_context
                .and_then(|context| correlate_renderer_crash(signal.clone(), context))
                .or_else(|| {
                    if command_target_matches {
                        return None;
                    }
                    let background = self.renderer_crash_background_context(signal);
                    correlate_renderer_crash(signal.clone(), &background)
                });
            let Some(observation) = observation else {
                continue;
            };
            let persistence = repository
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|repository| {
                    persist_renderer_crash_in_repository(repository, &observation)
                });
            if command_target_matches && matched.is_none() {
                matched = Some((observation, persistence));
            }
        }
        matched
    }
    pub(crate) async fn apply_drained_events(&mut self, drained: DrainedEvents) {
        if debug_session_events_enabled() {
            if let Some(ref mgr) = self.browser {
                eprintln!(
                    "[agent-browser][sessions] before active={} pages={:?} attached_page={:?} detached_page={:?} changed_targets={} destroyed_targets={:?}",
                    mgr.active_session_id().unwrap_or("<none>"), mgr.pages_list().iter()
                    .map(| p | format!("{} {} {}", p.target_id, p.session_id, p.url))
                    .collect::< Vec < _ >> (), drained.attached_page_sessions, drained
                    .detached_page_sessions, drained.changed_targets.len(), drained
                    .destroyed_targets
                );
            }
        }
        if !drained.pending_acks.is_empty() {
            if let Some(ref browser) = self.browser {
                if let Ok(session_id) = browser.active_session_id() {
                    for ack_sid in drained.pending_acks {
                        let _ = stream::ack_screencast_frame(&browser.client, session_id, ack_sid)
                            .await;
                    }
                }
            }
        }
        for target_id in &drained.destroyed_targets {
            if let Some(ref mut mgr) = self.browser {
                mgr.remove_page_by_target_id(target_id);
            }
        }
        for (target_id, page_sid) in &drained.attached_page_sessions {
            if let Some(ref mut mgr) = self.browser {
                let should_update =
                    mgr.page_session_for_target(target_id)
                        .is_some_and(|current_sid| {
                            drained
                                .detached_page_sessions
                                .iter()
                                .any(|detached_sid| detached_sid == current_sid)
                        });
                if should_update && mgr.update_page_session(target_id, page_sid) {
                    let _ = mgr.enable_domains_pub(page_sid).await;
                }
            }
        }
        for (frame_id, iframe_sid) in &drained.attached_iframe_sessions {
            self.iframe_sessions
                .insert(frame_id.clone(), iframe_sid.clone());
            if let Some(ref mgr) = self.browser {
                let _ = mgr
                    .client
                    .send_command_no_params(
                        "Runtime.runIfWaitingForDebugger",
                        Some(iframe_sid.as_str()),
                    )
                    .await;
                let _ = mgr
                    .client
                    .send_command_no_params("DOM.enable", Some(iframe_sid.as_str()))
                    .await;
                let _ = mgr
                    .client
                    .send_command_no_params("Accessibility.enable", Some(iframe_sid.as_str()))
                    .await;
                if self.har_recording || self.request_tracking {
                    let _ = mgr
                        .client
                        .send_command_no_params("Network.enable", Some(iframe_sid.as_str()))
                        .await;
                }
            }
        }
        for sid in &drained.detached_iframe_sessions {
            self.iframe_sessions.retain(|_, v| v != sid);
        }
        for te in &drained.new_targets {
            if let Some(ref mut mgr) = self.browser {
                let attach_result: Result<AttachToTargetResult, String> = mgr
                    .client
                    .send_command_typed(
                        "Target.attachToTarget",
                        &AttachToTargetParams {
                            target_id: te.target_info.target_id.clone(),
                            flatten: true,
                        },
                        None,
                    )
                    .await;
                if let Ok(attach) = attach_result {
                    let _ = mgr.enable_domains_pub(&attach.session_id).await;
                    let df = self.domain_filter.read().await;
                    if let Some(ref filter) = *df {
                        let has_proxy_creds = self.proxy_credentials.read().await.is_some();
                        let _ = network::install_domain_filter(
                            &mgr.client,
                            &attach.session_id,
                            &filter.allowed_domains,
                            has_proxy_creds,
                        )
                        .await;
                    }
                    mgr.add_page_with_activation(
                        super::super::super::browser::PageInfo {
                            target_id: te.target_info.target_id.clone(),
                            session_id: attach.session_id,
                            url: te.target_info.url.clone(),
                            title: te.target_info.title.clone(),
                            target_type: te.target_info.target_type.clone(),
                        },
                        false,
                    );
                }
            }
        }
        for te in &drained.changed_targets {
            if let Some(ref mut mgr) = self.browser {
                mgr.update_page_target_info(&te.target_info);
            }
        }
        if debug_session_events_enabled() {
            if let Some(ref mgr) = self.browser {
                eprintln!(
                    "[agent-browser][sessions] after active={} pages={:?}",
                    mgr.active_session_id().unwrap_or("<none>"),
                    mgr.pages_list()
                        .iter()
                        .map(|p| format!("{} {} {}", p.target_id, p.session_id, p.url))
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    pub(crate) fn drain_cdp_events(&mut self) -> DrainedEvents {
        let rx = match self.event_rx.as_mut() {
            Some(rx) => rx,
            None => return DrainedEvents::default(),
        };
        let mut pending_acks: Vec<i64> = Vec::new();
        let mut new_targets: Vec<TargetCreatedEvent> = Vec::new();
        let mut new_target_ids: HashSet<String> = HashSet::new();
        let mut changed_targets: Vec<TargetInfoChangedEvent> = Vec::new();
        let mut destroyed_targets: Vec<String> = Vec::new();
        let mut attached_page_sessions: Vec<(String, String)> = Vec::new();
        let mut attached_iframe_sessions: Vec<(String, String)> = Vec::new();
        let mut detached_page_sessions: Vec<String> = Vec::new();
        let mut detached_iframe_sessions: Vec<String> = Vec::new();
        let mut renderer_crashes: Vec<RendererCrashSignal> = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    if let Some(signal) = RendererCrashSignal::from_cdp_event(&event) {
                        renderer_crashes.push(signal);
                        continue;
                    }
                    match event.method.as_str() {
                        "Target.targetCreated" => {
                            if let Ok(te) =
                                serde_json::from_value::<TargetCreatedEvent>(event.params.clone())
                            {
                                if should_track_target(&te.target_info) {
                                    let already_tracked = self
                                        .browser
                                        .as_ref()
                                        .is_none_or(|b| b.has_target(&te.target_info.target_id));
                                    if !already_tracked {
                                        new_target_ids.insert(te.target_info.target_id.clone());
                                        new_targets.push(te);
                                    }
                                }
                            }
                            continue;
                        }
                        "Target.targetInfoChanged" => {
                            if let Ok(te) = serde_json::from_value::<TargetInfoChangedEvent>(
                                event.params.clone(),
                            ) {
                                if should_track_target(&te.target_info) {
                                    let already_tracked = self
                                        .browser
                                        .as_ref()
                                        .is_some_and(|b| b.has_target(&te.target_info.target_id));
                                    if already_tracked
                                        || new_target_ids.contains(&te.target_info.target_id)
                                    {
                                        changed_targets.push(te);
                                    } else {
                                        new_target_ids.insert(te.target_info.target_id.clone());
                                        new_targets.push(TargetCreatedEvent {
                                            target_info: te.target_info,
                                        });
                                    }
                                }
                            }
                            continue;
                        }
                        "Target.targetDestroyed" => {
                            if let Ok(te) =
                                serde_json::from_value::<TargetDestroyedEvent>(event.params.clone())
                            {
                                destroyed_targets.push(te.target_id);
                            }
                            continue;
                        }
                        "Target.attachedToTarget" => {
                            if let (Some(sid), Some(target_info)) = (
                                event.params.get("sessionId").and_then(|v| v.as_str()),
                                event.params.get("targetInfo"),
                            ) {
                                let target_type = target_info
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if target_type == "iframe" {
                                    if let Some(target_id) =
                                        target_info.get("targetId").and_then(|v| v.as_str())
                                    {
                                        attached_iframe_sessions
                                            .push((target_id.to_string(), sid.to_string()));
                                    }
                                } else if matches!(target_type, "page" | "webview") {
                                    if let Some(target_id) =
                                        target_info.get("targetId").and_then(|v| v.as_str())
                                    {
                                        attached_page_sessions
                                            .push((target_id.to_string(), sid.to_string()));
                                    }
                                }
                            }
                            continue;
                        }
                        "Target.detachedFromTarget" => {
                            if let Some(sid) =
                                event.params.get("sessionId").and_then(|v| v.as_str())
                            {
                                let is_page_session = self.browser.as_ref().is_some_and(|b| {
                                    b.pages_list().iter().any(|p| p.session_id == sid)
                                });
                                if is_page_session {
                                    detached_page_sessions.push(sid.to_string());
                                } else {
                                    detached_iframe_sessions.push(sid.to_string());
                                }
                            }
                            continue;
                        }
                        _ => {}
                    }
                    let session_matches = if let Some(ref browser) = self.browser {
                        event.session_id.as_deref() == browser.active_session_id().ok()
                    } else {
                        false
                    };
                    let iframe_network_event = !session_matches
                        && (self.har_recording || self.request_tracking)
                        && event.method.starts_with("Network.")
                        && event
                            .session_id
                            .as_ref()
                            .is_some_and(|sid| self.iframe_sessions.values().any(|v| v == sid));
                    if !session_matches && !iframe_network_event {
                        continue;
                    }
                    match event.method.as_str() {
                        "Runtime.consoleAPICalled" => {
                            let level = event
                                .params
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("log");
                            let raw_args: Vec<Value> = event
                                .params
                                .get("args")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let text = network::format_console_args(&raw_args);
                            if let Some(ref server) = self.stream_server {
                                server.broadcast_console(level, &text, &raw_args);
                            }
                            self.event_tracker.add_console(level, &text, raw_args);
                        }
                        "Runtime.exceptionThrown" => {
                            if let Ok(ex_event) =
                                serde_json::from_value::<ExceptionThrownEvent>(event.params.clone())
                            {
                                let details = &ex_event.exception_details;
                                let text = details
                                    .exception
                                    .as_ref()
                                    .and_then(|e| e.description.as_deref())
                                    .unwrap_or(&details.text);
                                self.event_tracker.add_error(
                                    text,
                                    None,
                                    details.line_number,
                                    details.column_number,
                                );
                                if let Some(ref server) = self.stream_server {
                                    server.broadcast_page_error(
                                        text,
                                        details.line_number,
                                        details.column_number,
                                    );
                                }
                            }
                        }
                        "Network.requestWillBeSent"
                            if self.har_recording || self.request_tracking =>
                        {
                            if let Some(request) = event.params.get("request") {
                                let method = request
                                    .get("method")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("GET")
                                    .to_string();
                                let url = request
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let request_id = event
                                    .params
                                    .get("requestId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if self.har_recording {
                                    let wall_time = event
                                        .params
                                        .get("wallTime")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    let request_headers =
                                        har_extract_headers(request.get("headers"));
                                    let post_data = request
                                        .get("postData")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                    let request_body_size =
                                        post_data.as_ref().map(|s| s.len() as i64).unwrap_or(0);
                                    let resource_type = event
                                        .params
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Other")
                                        .to_string();
                                    self.har_entries.push(HarEntry {
                                        request_id: request_id.clone(),
                                        wall_time,
                                        method: method.clone(),
                                        url: url.clone(),
                                        request_headers,
                                        post_data,
                                        request_body_size,
                                        resource_type,
                                        status: None,
                                        status_text: String::new(),
                                        http_version: "HTTP/1.1".to_string(),
                                        response_headers: Vec::new(),
                                        mime_type: String::new(),
                                        redirect_url: String::new(),
                                        response_body_size: -1,
                                        cdp_timing: None,
                                        loading_finished_timestamp: None,
                                    });
                                }
                                if self.request_tracking {
                                    let headers =
                                        request.get("headers").cloned().unwrap_or(json!({}));
                                    let resource_type = event
                                        .params
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Other")
                                        .to_string();
                                    let timestamp = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_millis() as u64)
                                        .unwrap_or(0);
                                    self.tracked_requests.push(TrackedRequest {
                                        url,
                                        method,
                                        headers,
                                        timestamp,
                                        resource_type,
                                        request_id,
                                        post_data: request
                                            .get("postData")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                        status: None,
                                        response_headers: None,
                                        mime_type: None,
                                    });
                                }
                            }
                        }
                        "Network.responseReceived"
                            if self.har_recording || self.request_tracking =>
                        {
                            if let Some(response) = event.params.get("response") {
                                let request_id = event
                                    .params
                                    .get("requestId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let status = response.get("status").and_then(|v| v.as_i64());
                                let status_text = response
                                    .get("statusText")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let mime_type = response
                                    .get("mimeType")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let http_version = response
                                    .get("protocol")
                                    .and_then(|v| v.as_str())
                                    .map(har_cdp_protocol_to_http_version)
                                    .unwrap_or_else(|| "HTTP/1.1".to_string());
                                let response_headers = har_extract_headers(response.get("headers"));
                                let redirect_url = response_headers
                                    .iter()
                                    .find(|(k, _)| k.eq_ignore_ascii_case("location"))
                                    .map(|(_, v)| v.clone())
                                    .unwrap_or_default();
                                let encoded_data_length = response
                                    .get("encodedDataLength")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(-1);
                                let cdp_timing = response.get("timing").cloned();
                                if self.har_recording {
                                    if let Some(entry) = self
                                        .har_entries
                                        .iter_mut()
                                        .rev()
                                        .find(|e| e.request_id == request_id)
                                    {
                                        entry.status = status;
                                        entry.status_text = status_text;
                                        entry.mime_type = mime_type;
                                        entry.http_version = http_version;
                                        entry.response_headers = response_headers;
                                        entry.redirect_url = redirect_url;
                                        entry.response_body_size = encoded_data_length;
                                        entry.cdp_timing = cdp_timing;
                                    }
                                }
                                if self.request_tracking {
                                    let resp_headers = response.get("headers").cloned();
                                    let resp_mime = response
                                        .get("mimeType")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                    if let Some(entry) = self
                                        .tracked_requests
                                        .iter_mut()
                                        .rev()
                                        .find(|e| e.request_id == request_id)
                                    {
                                        entry.status = status;
                                        entry.mime_type = resp_mime;
                                        entry.response_headers = resp_headers;
                                    }
                                }
                            }
                        }
                        "Network.loadingFinished" if self.har_recording => {
                            let request_id = event
                                .params
                                .get("requestId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let timestamp = event.params.get("timestamp").and_then(|v| v.as_f64());
                            let encoded_data_length = event
                                .params
                                .get("encodedDataLength")
                                .and_then(|v| v.as_i64());
                            if let Some(entry) = self
                                .har_entries
                                .iter_mut()
                                .rev()
                                .find(|e| e.request_id == request_id)
                            {
                                if let Some(ts) = timestamp {
                                    entry.loading_finished_timestamp = Some(ts);
                                }
                                if let Some(len) = encoded_data_length {
                                    entry.response_body_size = len;
                                }
                            }
                        }
                        "Network.loadingFailed" if self.har_recording => {
                            let request_id = event
                                .params
                                .get("requestId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let timestamp = event.params.get("timestamp").and_then(|v| v.as_f64());
                            let error_text = event
                                .params
                                .get("errorText")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Failed");
                            if let Some(entry) = self
                                .har_entries
                                .iter_mut()
                                .rev()
                                .find(|e| e.request_id == request_id)
                            {
                                if entry.status.is_none() {
                                    entry.status = Some(0);
                                    entry.status_text = error_text.to_string();
                                }
                                if let Some(ts) = timestamp {
                                    entry.loading_finished_timestamp = Some(ts);
                                }
                            }
                        }
                        "Page.screencastFrame" if self.stream_server.is_none() => {
                            if let Some(sid) =
                                event.params.get("sessionId").and_then(|v| v.as_i64())
                            {
                                pending_acks.push(sid);
                            }
                        }
                        "Page.javascriptDialogOpening" => {
                            if let Ok(dialog_event) =
                                serde_json::from_value::<JavascriptDialogOpeningEvent>(
                                    event.params.clone(),
                                )
                            {
                                let auto_handled = self.auto_dialog
                                    && matches!(
                                        dialog_event.dialog_type.as_str(),
                                        "beforeunload" | "alert"
                                    );
                                if !auto_handled {
                                    self.pending_dialog = Some(PendingDialog {
                                        dialog_type: dialog_event.dialog_type,
                                        message: dialog_event.message,
                                        url: dialog_event.url,
                                        default_prompt: dialog_event.default_prompt,
                                    });
                                }
                            }
                        }
                        "Page.javascriptDialogClosed" => {
                            self.pending_dialog = None;
                        }
                        _ => {}
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    eprintln!(
                        "[agent-browser] Warning: CDP event buffer overflowed, {} events dropped. Network requests may be missing from HAR output.",
                        n
                    );
                    continue;
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    self.event_rx = None;
                    break;
                }
            }
        }
        DrainedEvents {
            pending_acks,
            new_targets,
            changed_targets,
            destroyed_targets,
            attached_page_sessions,
            attached_iframe_sessions,
            detached_page_sessions,
            detached_iframe_sessions,
            renderer_crashes,
        }
    }
}
pub(crate) fn runtime_profile_pid(runtime_profile: Option<&str>) -> Option<u32> {
    let runtime_profile = runtime_profile?;
    let state = read_runtime_state(runtime_profile).ok().flatten()?;
    crate::runtime_profile::runtime_process_assessment(Some(runtime_profile), state.browser_pid)
        .authorizes_adoption()
        .then_some(state.browser_pid)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedRuntimeAttachTarget {
    pub(crate) runtime_profile: String,
    pub(crate) browser_pid: u32,
    pub(crate) cdp_port: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedProfileAttachTarget {
    pub(crate) browser_id: String,
    pub(crate) runtime_profile: String,
    pub(crate) cdp_endpoint: String,
    pub(crate) browser_pid: Option<u32>,
    pub(crate) owner_session_ids: Vec<String>,
}
pub(crate) fn managed_runtime_attach_target(
    runtime_profile: Option<&str>,
) -> Option<ManagedRuntimeAttachTarget> {
    let runtime_profile = runtime_profile?;
    let state = read_runtime_state(runtime_profile).ok().flatten()?;
    let assessment = crate::runtime_profile::runtime_process_assessment(
        Some(runtime_profile),
        state.browser_pid,
    );
    if !assessment.authorizes_adoption() {
        return None;
    }
    let browser_pid = state.browser_pid;
    let cdp_port = state
        .devtools_port
        .or_else(|| read_devtools_port(std::path::Path::new(&state.user_data_dir)))?;
    Some(ManagedRuntimeAttachTarget {
        runtime_profile: runtime_profile.to_string(),
        browser_pid,
        cdp_port,
    })
}
pub(crate) fn can_attach_managed_runtime_for_launch(options: &LaunchOptions) -> bool {
    options.headless && !options.remote_headed
}
pub(crate) fn shared_profile_attach_target_for_auto_launch(
    metadata: &ServiceLaunchMetadata,
    command: &Value,
    session_id: &str,
) -> Option<SharedProfileAttachTarget> {
    let action = command.get("action").and_then(Value::as_str)?;
    if !matches!(action, "open" | "navigate" | "tab_new") {
        return None;
    }
    if command.get("browserId").is_some() || command.get("sessionName").is_some() {
        return None;
    }
    if allow_duplicate_profile_lane_from_command(command) {
        return None;
    }
    let profile_id = metadata.profile_id.as_deref()?;
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let requested_host = browser_host_from_command(command);
    let requested_display_isolation = remote_headed_display_isolation_from_command(command);
    let current_browser_id = service_browser_id(session_id);
    let mut candidates = service_state
        .browsers
        .values()
        .filter(|browser| browser.profile_id.as_deref() == Some(profile_id))
        .filter(|browser| service_browser_health_counts_as_live(browser.health))
        .filter(|browser| {
            requested_host.is_none_or(|host| {
                host == browser.host || host == ServiceBrowserHost::AttachedExisting
            })
        })
        .filter(|browser| {
            requested_display_isolation
                .as_deref()
                .is_none_or(|display_isolation| {
                    browser
                        .display_isolation
                        .as_deref()
                        .is_none_or(|owner_display_isolation| {
                            owner_display_isolation == display_isolation
                        })
                })
        })
        .filter_map(|browser| {
            browser
                .cdp_endpoint
                .as_deref()
                .map(str::trim)
                .filter(|endpoint| !endpoint.is_empty())
                .map(|endpoint| (browser, endpoint.to_string()))
        })
        .collect::<Vec<(&BrowserProcess, String)>>();
    candidates.sort_by(|left, right| {
        let left_current = left.0.id == current_browser_id;
        let right_current = right.0.id == current_browser_id;
        right_current
            .cmp(&left_current)
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let (browser, cdp_endpoint) = candidates.into_iter().next()?;
    Some(SharedProfileAttachTarget {
        browser_id: browser.id.clone(),
        runtime_profile: profile_id.to_string(),
        cdp_endpoint,
        browser_pid: browser.pid,
        owner_session_ids: browser.active_session_ids.clone(),
    })
}
pub(crate) fn retained_session_attach_target_for_auto_launch(
    command: &Value,
    session_id: &str,
) -> Option<SharedProfileAttachTarget> {
    let action = command.get("action").and_then(Value::as_str)?;
    if matches!(
        action,
        "launch" | "cdp_free_launch" | "open" | "navigate" | "tab_new"
    ) {
        return None;
    }
    if optional_command_string(command, "sessionName")
        .is_some_and(|requested| requested != session_id)
    {
        return None;
    }
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let requested_browser_id = optional_command_string(command, "browserId");
    let current_browser_id = service_browser_id(session_id);
    let mut candidates = service_state
        .browsers
        .values()
        .filter(|browser| service_browser_health_counts_as_live(browser.health))
        .filter(|browser| {
            browser.id == current_browser_id
                || browser
                    .active_session_ids
                    .iter()
                    .any(|owner_session_id| owner_session_id == session_id)
        })
        .filter(|browser| {
            requested_browser_id
                .as_deref()
                .is_none_or(|requested| requested == browser.id)
        })
        .filter_map(|browser| {
            let runtime_profile = browser.profile_id.as_deref()?.trim();
            let cdp_endpoint = browser.cdp_endpoint.as_deref()?.trim();
            if runtime_profile.is_empty() || cdp_endpoint.is_empty() {
                return None;
            }
            Some((
                browser,
                runtime_profile.to_string(),
                cdp_endpoint.to_string(),
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_current = left.0.id == current_browser_id;
        let right_current = right.0.id == current_browser_id;
        right_current
            .cmp(&left_current)
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let (browser, runtime_profile, cdp_endpoint) = candidates.into_iter().next()?;
    Some(SharedProfileAttachTarget {
        browser_id: browser.id.clone(),
        runtime_profile,
        cdp_endpoint,
        browser_pid: browser.pid,
        owner_session_ids: browser.active_session_ids.clone(),
    })
}
