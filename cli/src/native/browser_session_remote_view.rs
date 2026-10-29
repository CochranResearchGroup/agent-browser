//! Runtime construction from the public Remote View application inventory.
use super::browser_session_runtime::{BrowserManagerRuntime, BrowserSessionEffectAdapter};
use super::browser_session_store::BrowserSessionSqliteStore;
use super::remote_view_application_http::RemoteViewApplicationHttp;
use agent_browser_service_model::*;
use std::time::Duration;

static POOL_MAINTENANCE: std::sync::LazyLock<tokio::sync::Notify> =
    std::sync::LazyLock::new(tokio::sync::Notify::new);

/// Coalesce launch-triggered spare work without provisioning under the host lock.
pub(crate) fn request_pool_maintenance() {
    if std::env::var_os("AGENT_BROWSER_HOSTS_FILE").is_none() && remote_view_settings_present() {
        POOL_MAINTENANCE.notify_one();
    }
}

pub(crate) async fn pool_maintenance_requested() {
    POOL_MAINTENANCE.notified().await;
}

type LocalEffects = BrowserSessionEffectAdapter<BrowserManagerRuntime>;
type RemoteEffects =
    RemoteViewSessionEffects<LocalEffects, RemoteViewApplicationHttp, BrowserSessionSqliteStore>;

pub(crate) enum RuntimeSessionEffects {
    Local(LocalEffects),
    Remote(Box<RemoteEffects>),
}

pub(super) struct RemoteViewRuntimeContext {
    adapter: RemoteViewApplicationAdapter<RemoteViewApplicationHttp>,
    pool: RemoteViewSessionPool,
}

/// Detect an explicitly selected Remote View lane before loading the host.
/// Partial settings enter validation rather than falling back to local launch.
pub(crate) fn remote_view_settings_present() -> bool {
    [
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
        "AGENT_BROWSER_REMOTE_VIEW_POOL",
        "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
        "AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

/// Runtime setup validates explicit settings without reading or mutating the
/// provider. Capacity preparation belongs to a new browser's placement demand.
pub(super) fn configured_remote_view() -> Result<Option<RemoteViewRuntimeContext>, String> {
    runtime_context(
        setting("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")?,
        setting("AGENT_BROWSER_REMOTE_VIEW_POOL")?,
        setting("AGENT_BROWSER_REMOTE_VIEW_APPLICATION")?,
        setting("AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT")?,
    )
}

type WorkspaceWindows =
    std::collections::BTreeMap<(String, u64), Option<RemoteViewApplicationWindows>>;

#[derive(Clone, Default)]
pub(crate) struct WorkspaceNativeSample {
    pub windows: WorkspaceWindows,
    /// Start of the provider read, never the time a cached result is served.
    pub observed_at_ms: Option<u64>,
    pub age_ms: u64,
}

type WorkspaceProbeKey = (String, String, Vec<(String, u64)>);
#[derive(Default)]
struct WorkspaceProbe {
    key: Option<WorkspaceProbeKey>,
    revision: u64,
    pending: Option<(
        u64,
        std::thread::JoinHandle<(std::time::Instant, WorkspaceNativeSample)>,
    )>,
    sample: Option<(std::time::Instant, WorkspaceNativeSample)>,
}
impl WorkspaceProbe {
    fn fresh(&self, now: std::time::Instant) -> WorkspaceNativeSample {
        self.sample
            .as_ref()
            .filter(|(started, _)| {
                now.saturating_duration_since(*started) < Duration::from_secs(30)
            })
            .map(|(started, sample)| {
                let mut sample = sample.clone();
                sample.age_ms = now.saturating_duration_since(*started).as_millis() as u64;
                sample
            })
            .unwrap_or_default()
    }
}

/// Single-flight read-only native observation. Dashboard requests never join
/// provider I/O. Incomplete, failed, changed-generation and expired reads are
/// unknown. Cached confidence retains its original observation start time.
pub(crate) fn observe_workspace_desktops(
    desktops: &[RemoteViewFixedDesktop],
) -> WorkspaceNativeSample {
    static PROBE: std::sync::LazyLock<std::sync::Mutex<WorkspaceProbe>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(WorkspaceProbe::default()));
    let mut probe = PROBE.lock().unwrap_or_else(|error| error.into_inner());
    let Ok(Some(origin)) = setting("AGENT_BROWSER_REMOTE_VIEW_ORIGIN") else {
        return WorkspaceNativeSample::default();
    };
    let Ok(application) = setting("AGENT_BROWSER_REMOTE_VIEW_APPLICATION") else {
        return WorkspaceNativeSample::default();
    };
    let application = application.unwrap_or_else(|| "agent-browser".into());
    let key = (
        origin.clone(),
        application.clone(),
        desktops
            .iter()
            .map(|d| (d.desktop_id.clone(), d.generation))
            .collect(),
    );
    if probe.key.as_ref() != Some(&key) {
        probe.key = Some(key);
        probe.revision = probe.revision.wrapping_add(1);
        probe.sample = None;
    }
    if probe
        .pending
        .as_ref()
        .is_some_and(|(_, worker)| worker.is_finished())
    {
        let (revision, worker) = probe.pending.take().unwrap();
        if let Ok(sample) = worker.join() {
            if revision == probe.revision {
                probe.sample = Some(sample);
            }
        } else {
            probe.sample = None;
        }
    }
    let now = std::time::Instant::now();
    let result = probe.fresh(now);
    let due = probe.sample.as_ref().is_none_or(|(started, _)| {
        now.saturating_duration_since(*started) >= Duration::from_secs(2)
    });
    if !desktops.is_empty() && !origin.is_empty() && due && probe.pending.is_none() {
        let desktops = desktops.to_vec();
        let revision = probe.revision;
        if let Ok(worker) = std::thread::Builder::new()
            .name("workspace-native-read".into())
            .spawn(move || {
                let observed_at_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|t| t.as_millis() as u64)
                    .unwrap_or_default();
                let windows = (|| {
                    let transport =
                        RemoteViewApplicationHttp::new(&origin, Duration::from_secs(2)).ok()?;
                    let mut adapter =
                        RemoteViewApplicationAdapter::new(application, transport).ok()?;
                    static CURSOR: std::sync::atomic::AtomicUsize =
                        std::sync::atomic::AtomicUsize::new(0);
                    let offset = CURSOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Some(observe_workspace_desktops_with(
                        &mut adapter,
                        &desktops,
                        offset,
                    ))
                })()
                .unwrap_or_default();
                (
                    now,
                    WorkspaceNativeSample {
                        windows,
                        observed_at_ms: Some(observed_at_ms),
                        age_ms: 0,
                    },
                )
            })
        {
            probe.pending = Some((revision, worker));
        }
    }
    result
}

fn observe_workspace_desktops_with<T: RemoteViewApplicationTransport>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    desktops: &[RemoteViewFixedDesktop],
    offset: usize,
) -> std::collections::BTreeMap<(String, u64), Option<RemoteViewApplicationWindows>> {
    let mut result = std::collections::BTreeMap::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    let Ok(inventory) = adapter.inventory() else {
        return result;
    };
    for desktop in desktops
        .iter()
        .cycle()
        .skip(offset % desktops.len().max(1))
        .take(desktops.len())
    {
        if std::time::Instant::now() >= deadline {
            break;
        }
        let key = (desktop.desktop_id.clone(), desktop.generation);
        match active_workspace_assignment(&inventory, desktop) {
            Ok(Some(assignment)) => {
                if let Ok(windows) = adapter.windows(assignment) {
                    result.insert(key, Some(windows));
                }
            }
            Ok(None) => {
                result.insert(key, None);
            }
            Err(()) => {}
        }
    }
    result
}

fn active_workspace_assignment<'a>(
    inventory: &'a RemoteViewApplicationInventory,
    desktop: &RemoteViewFixedDesktop,
) -> Result<Option<&'a RemoteViewAssignmentRecord>, ()> {
    let mut matching = inventory.assignments.iter().filter(|assignment| {
        assignment.desktop_id == desktop.desktop_id
            && assignment.generation == desktop.generation
            && assignment.state == RemoteViewAssignmentState::Active
    });
    let assignment = matching.next();
    if matching.next().is_some() {
        return Err(());
    }
    Ok(assignment)
}

/// Run from the background reaper after releasing the session-host mutex.
/// Public acquisition custody coalesces requests from other daemon processes;
/// no foreground operation waits for spare provisioning.
pub(crate) fn replenish_configured_remote_view_pool() -> Result<(), String> {
    // A host registry makes spare growth and retirement app-owned. Check its
    // presence before opening state, validating provider configuration or I/O.
    if std::env::var_os("AGENT_BROWSER_HOSTS_FILE").is_some() {
        return Ok(());
    }
    let Some(mut context) = configured_remote_view()? else {
        return Ok(());
    };
    let mut store = BrowserSessionSqliteStore::default_sqlite()?;
    let state = store.load_session_state()?;
    maintain_only_standalone_pool(false, &state, || {
        super::browser_session_pool_maintenance::maintain_pool(
            &mut context.adapter,
            &mut store,
            &context.pool,
        )?;
        Ok(())
    })
}

/// Even ended hosted metadata protects app ownership after configuration loss.
/// The effect closure is never entered for a hosted runtime.
fn maintain_only_standalone_pool(
    host_registry_present: bool,
    state: &BrowserSessionState,
    maintain: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if host_registry_present || !state.host_bindings.is_empty() {
        return Ok(());
    }
    maintain()
}

fn runtime_context(
    origin: Option<String>,
    pool: Option<String>,
    application: Option<String>,
    count: Option<String>,
) -> Result<Option<RemoteViewRuntimeContext>, String> {
    let (origin, pool, application, count) = match (origin, pool, application, count) {
        (None, None, None, None) => return Ok(None),
        (Some(origin), Some(pool), application, count) if !pool.is_empty() => {
            (origin, pool, application, count)
        }
        _ => return Err("remote_view_runtime_config_incomplete".into()),
    };
    let desired_desktops = count
        .as_deref()
        .unwrap_or("1")
        .parse::<u32>()
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| "remote_view_runtime_desktop_count_invalid".to_string())?;
    let transport = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(15))
        .map_err(|_| "remote_view_runtime_origin_invalid".to_string())?;
    let adapter = RemoteViewApplicationAdapter::new(
        application.unwrap_or_else(|| "agent-browser".into()),
        transport,
    )
    .map_err(|_| "remote_view_runtime_application_invalid".to_string())?;
    Ok(Some(RemoteViewRuntimeContext {
        adapter,
        pool: RemoteViewSessionPool {
            name: pool,
            desired_desktops,
        },
    }))
}

fn setting(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err("remote_view_runtime_config_invalid".into()),
    }
}

pub(super) fn runtime_effects(
    mut local: LocalEffects,
    context: Option<RemoteViewRuntimeContext>,
) -> Result<RuntimeSessionEffects, String> {
    match context {
        None => Ok(RuntimeSessionEffects::Local(local)),
        Some(context) => {
            let mut store = BrowserSessionSqliteStore::default_sqlite()?;
            reconcile_pending_recoveries(&mut local, &mut store)?;
            Ok(RuntimeSessionEffects::Remote(Box::new(
                RemoteViewSessionEffects::new(local, context.adapter, store, Vec::new(), || {
                    uuid::Uuid::new_v4().to_string()
                })?
                .with_pool(context.pool)?
                .with_view_clock(current_view_time_ms),
            )))
        }
    }
}

/// Retire interrupted recovery bookkeeping only after current profile/process
/// observation proves absence. Live observed launches are joined during ordinary
/// profile reconciliation; history never substitutes for current liveness.
pub(super) fn reconcile_pending_recoveries<E: RemoteViewBrowserProcessEffects>(
    local: &mut E,
    store: &mut BrowserSessionSqliteStore,
) -> Result<(), String> {
    let pending = match store.unpublished_launch_records() {
        Ok(pending) => pending,
        // Keep malformed history intact for diagnostics. It cannot veto host
        // startup or authorize recovery, launch, or destructive cleanup.
        Err(LaunchCustodyStoreError::InvalidRecord) => return Ok(()),
        Err(_) => return Err("remote_view_session_custody_unavailable".into()),
    };
    if pending.is_empty() {
        return Ok(());
    }
    let state = store.load_session_state()?;
    let catalog = store.load_profile_catalog()?.catalog;
    for claim in pending {
        let Some(browser) = state
            .browsers
            .values()
            .find(|browser| browser.profile_id == claim.intent.profile_id)
        else {
            continue;
        };
        let Some(profile) = catalog.profiles.get(&browser.profile_id).or_else(|| {
            state
                .disposable_profiles
                .get(&browser.profile_id)
                .map(|p| &p.profile)
        }) else {
            continue;
        };
        let mut observed = browser.clone();
        if let Some(pid) = claim.observed_pid {
            observed.pid = pid;
        }
        if local.prove_recovery_absence(&observed, profile).is_err() {
            continue;
        }
        match store.reconcile_absent_recovery(&claim, &state, browser) {
            Ok(()) | Err(LaunchCustodyStoreError::Conflict) => {}
            Err(_) => return Err("remote_view_session_custody_unavailable".into()),
        }
    }
    Ok(())
}

impl BrowserSessionEffects for RuntimeSessionEffects {
    fn attach_app_browser(
        &mut self,
        request: &BrowserAttachmentRequest,
        profile: &BrowserProfileCatalogEntry,
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserAttachmentObservation, String> {
        match self {
            Self::Local(effects) => effects.attach_app_browser(request, profile, desktop),
            Self::Remote(effects) => effects.attach_app_browser(request, profile, desktop),
        }
    }

    fn native_handoff_assignment_target(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
    ) -> Result<RemoteViewApplicationTarget, String> {
        match self {
            Self::Local(e) => e.native_handoff_assignment_target(target),
            Self::Remote(e) => e.native_handoff_assignment_target(target),
        }
    }
    fn register_native_handoff_observation(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        provider_target: &RemoteViewApplicationTarget,
        target_digest: &str,
        idempotency_key: &str,
    ) -> Result<RemoteViewNativeHandoffObservation, String> {
        match self {
            Self::Local(e) => e.register_native_handoff_observation(
                target,
                provider_target,
                target_digest,
                idempotency_key,
            ),
            Self::Remote(e) => e.register_native_handoff_observation(
                target,
                provider_target,
                target_digest,
                idempotency_key,
            ),
        }
    }
    fn observe_native_handoff_use(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        registered: &RemoteViewNativeHandoffObservation,
    ) -> Result<RemoteViewNativeHandoffUseRead, String> {
        match self {
            Self::Local(e) => e.observe_native_handoff_use(target, registered),
            Self::Remote(e) => e.observe_native_handoff_use(target, registered),
        }
    }
    fn reconcile_profile_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        browser: Option<&ManagedBrowserInstance>,
        tabs: &[ManagedBrowserTab],
        navigation: &[agent_browser_service_model::BrowserNavigationRecord],
    ) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
        match self {
            Self::Local(e) => e.reconcile_profile_browser(profile, browser, tabs, navigation),
            Self::Remote(e) => e.reconcile_profile_browser(profile, browser, tabs, navigation),
        }
    }

    fn recover_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
    ) -> Result<BrowserRecovery, String> {
        match self {
            Self::Local(effects) => effects.recover_browser(browser, profile, tabs, navigation),
            Self::Remote(effects) => effects.recover_browser(browser, profile, tabs, navigation),
        }
    }

    fn recover_browser_at(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserRecovery, String> {
        match self {
            Self::Local(effects) => {
                effects.recover_browser_at(browser, profile, tabs, navigation, desktop)
            }
            Self::Remote(effects) => {
                effects.recover_browser_at(browser, profile, tabs, navigation, desktop)
            }
        }
    }

    fn select_profile_browser_executable(
        &mut self,
        profile_id: String,
        path: String,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.select_profile_browser_executable(profile_id, path),
            Self::Remote(effects) => effects.select_profile_browser_executable(profile_id, path),
        }
    }
    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.select_browser_executable(path),
            Self::Remote(effects) => effects.select_browser_executable(path),
        }
    }

    fn resolve_remote_view_tab_view(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        view: &RemoteViewTabView,
        now_ms: u64,
    ) -> Result<RemoteViewApplicationViewIssuance, String> {
        match self {
            Self::Local(effects) => effects.resolve_remote_view_tab_view(target, view, now_ms),
            Self::Remote(effects) => effects.resolve_remote_view_tab_view(target, view, now_ms),
        }
    }
    fn observe_remote_view_tab_view(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        issuance: &RemoteViewApplicationViewIssuance,
        now_ms: u64,
    ) -> Result<RemoteViewApplicationViewObservation, String> {
        match self {
            Self::Local(effects) => effects.observe_remote_view_tab_view(target, issuance, now_ms),
            Self::Remote(effects) => effects.observe_remote_view_tab_view(target, issuance, now_ms),
        }
    }
    fn admits_ordinary_sessions(&self) -> bool {
        match self {
            Self::Local(effects) => effects.admits_ordinary_sessions(),
            Self::Remote(effects) => effects.admits_ordinary_sessions(),
        }
    }
    fn remote_view_desktop_candidates(
        &mut self,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        match self {
            Self::Local(effects) => effects.remote_view_desktop_candidates(),
            Self::Remote(effects) => effects.remote_view_desktop_candidates(),
        }
    }
    fn remote_view_placement_candidates(
        &mut self,
        placement: &BrowserPlacement,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        match self {
            Self::Local(effects) => effects.remote_view_placement_candidates(placement),
            Self::Remote(effects) => effects.remote_view_placement_candidates(placement),
        }
    }
    fn begin_operation(&mut self, expected: &BrowserSessionState) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.begin_operation(expected),
            Self::Remote(effects) => effects.begin_operation(expected),
        }
    }
    fn begin_borrowed_retirement_operation(
        &mut self,
        expected: &BrowserSessionState,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.begin_borrowed_retirement_operation(expected),
            Self::Remote(effects) => effects.begin_borrowed_retirement_operation(expected),
        }
    }
    fn pending_browser_activity_ids(&self) -> Vec<String> {
        match self {
            Self::Local(effects) => effects.pending_browser_activity_ids(),
            Self::Remote(effects) => effects.pending_browser_activity_ids(),
        }
    }
    fn pending_idle_retirement_ids(&self) -> Vec<String> {
        match self {
            Self::Local(effects) => effects.pending_idle_retirement_ids(),
            Self::Remote(effects) => effects.pending_idle_retirement_ids(),
        }
    }
    fn finish_idle_retirement_publication(&mut self, ids: &[String]) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.finish_idle_retirement_publication(ids),
            Self::Remote(effects) => effects.finish_idle_retirement_publication(ids),
        }
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        match self {
            Self::Local(effects) => effects.pending_launch_intent(),
            Self::Remote(effects) => effects.pending_launch_intent(),
        }
    }
    fn acknowledge_launch_publication(&mut self) {
        match self {
            Self::Local(effects) => effects.acknowledge_launch_publication(),
            Self::Remote(effects) => effects.acknowledge_launch_publication(),
        }
    }
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        match self {
            Self::Local(effects) => effects.browser_is_live(browser),
            Self::Remote(effects) => effects.browser_is_live(browser),
        }
    }
    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        match self {
            Self::Local(effects) => effects.launch_browser(profile, desktop),
            Self::Remote(effects) => effects.launch_browser(profile, desktop),
        }
    }
    fn permits_idle_browser_close(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<bool, String> {
        match self {
            Self::Local(effects) => effects.permits_idle_browser_close(browser),
            Self::Remote(effects) => effects.permits_idle_browser_close(browser),
        }
    }
    fn acquire_profile_cleanup_guard(
        &mut self,
        allocation: &agent_browser_service_model::ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        match self {
            Self::Local(effects) => effects.acquire_profile_cleanup_guard(allocation),
            Self::Remote(effects) => effects.acquire_profile_cleanup_guard(allocation),
        }
    }
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        match self {
            Self::Local(effects) => effects.acquire_idle_retirement_guard(browsers),
            Self::Remote(effects) => effects.acquire_idle_retirement_guard(browsers),
        }
    }
    fn acquire_disposable_retirement_guard(
        &mut self,
        allocations: &[agent_browser_service_model::ManagedDisposableProfile],
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        match self {
            Self::Local(effects) => {
                effects.acquire_disposable_retirement_guard(allocations, browsers)
            }
            Self::Remote(effects) => {
                effects.acquire_disposable_retirement_guard(allocations, browsers)
            }
        }
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.close_browser(browser),
            Self::Remote(effects) => effects.close_browser(browser),
        }
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        match self {
            Self::Local(effects) => effects.acquire_initial_tab(browser, attributed_target_ids),
            Self::Remote(effects) => effects.acquire_initial_tab(browser, attributed_target_ids),
        }
    }
    fn acquire_session_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        session: &agent_browser_service_model::ManagedBrowserSession,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        match self {
            Self::Local(effects) => effects.acquire_session_initial_tab(
                browser,
                profile,
                session,
                attributed_target_ids,
            ),
            Self::Remote(effects) => effects.acquire_session_initial_tab(
                browser,
                profile,
                session,
                attributed_target_ids,
            ),
        }
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        match self {
            Self::Local(effects) => effects.create_tab(browser),
            Self::Remote(effects) => effects.create_tab(browser),
        }
    }
    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.close_tab(browser, tab),
            Self::Remote(effects) => effects.close_tab(browser, tab),
        }
    }
    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.navigate(browser, tab, url),
            Self::Remote(effects) => effects.navigate(browser, tab, url),
        }
    }
    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.focus_browser(browser, tab),
            Self::Remote(effects) => effects.focus_browser(browser, tab),
        }
    }
    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation_id: &str,
        session_name: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        match self {
            Self::Local(effects) => {
                effects.allocate_disposable_profile(policy, allocation_id, session_name)
            }
            Self::Remote(effects) => {
                effects.allocate_disposable_profile(policy, allocation_id, session_name)
            }
        }
    }
    fn delete_disposable_profile(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.delete_disposable_profile(allocation),
            Self::Remote(effects) => effects.delete_disposable_profile(allocation),
        }
    }
    fn disposable_profile_size_bytes(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<u64, String> {
        match self {
            Self::Local(effects) => effects.disposable_profile_size_bytes(allocation),
            Self::Remote(effects) => effects.disposable_profile_size_bytes(allocation),
        }
    }
}
impl ManagedBrowserCommandEffects for RuntimeSessionEffects {
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self {
            Self::Local(effects) => {
                effects.execute_command(browser, tab, session_id, session_name, command)
            }
            Self::Remote(effects) => {
                effects.execute_command(browser, tab, session_id, session_name, command)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_runtime_wrapper_dispatches_app_attachment_without_provider_io() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let local = BrowserSessionEffectAdapter::new(
            BrowserManagerRuntime::start(Default::default()).unwrap(),
        );
        let adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            RemoteViewApplicationHttp::new(&endpoint, Duration::from_secs(1)).unwrap(),
        )
        .unwrap();
        let store = BrowserSessionSqliteStore::launch_custody_fixture(
            rusqlite::Connection::open_in_memory().unwrap(),
            true,
        )
        .unwrap();
        let mut effects = RuntimeSessionEffects::Remote(Box::new(
            RemoteViewSessionEffects::new(local, adapter, store, vec![], || {
                uuid::Uuid::new_v4().to_string()
            })
            .unwrap(),
        ));
        let request = BrowserAttachmentRequest {
            version: 1,
            process: RecordedProcessIdentity {
                pid: 0,
                start_token: "synthetic".into(),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: None,
            },
            profile_path: "/synthetic/profile".into(),
            cdp_endpoint: "ws://127.0.0.1:1/devtools/browser/synthetic".into(),
            target_id: "target".into(),
            shared_with: vec![],
        };
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "profile".into(),
            name: "Profile".into(),
            user_data_dir: request.profile_path.clone(),
            kind: BrowserProfileKind::Named,
        };
        let desktop = RemoteViewFixedDesktop {
            desktop_id: "desktop".into(),
            generation: 1,
            friendly_route_label: "App".into(),
        };
        // These are the concrete Remote View adapter's admission failures. The
        // BrowserSessionEffects default instead reports capability unavailable.
        assert_eq!(
            effects
                .attach_app_browser(&request, &profile, &desktop)
                .unwrap_err(),
            "remote_view_session_operation_baseline_missing"
        );
        effects
            .begin_operation(&BrowserSessionState::default())
            .unwrap();
        assert_eq!(
            effects
                .attach_app_browser(&request, &profile, &desktop)
                .unwrap_err(),
            "browser_attachment_desktop_observation_unavailable"
        );
        assert!(
            matches!(listener.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock)
        );
        drop(effects);
        let local = BrowserSessionEffectAdapter::new(
            BrowserManagerRuntime::start(Default::default()).unwrap(),
        );
        let mut effects = RuntimeSessionEffects::Local(local);
        assert_eq!(
            effects
                .attach_app_browser(&request, &profile, &desktop)
                .unwrap_err(),
            "browser_attachment_identity_invalid"
        );
        drop(effects);
    }

    #[test]
    fn hosted_pool_maintenance_never_enters_provider_effects_even_after_task_end() {
        let mut state = BrowserSessionState::default();
        maintain_only_standalone_pool(true, &state, || {
            panic!("host configuration must suppress provider I/O")
        })
        .unwrap();
        state.host_bindings.insert(
            "ended-task".into(),
            BrowserHostTaskBinding {
                request: BrowserHostBinding {
                    version: 1,
                    host_id: "app".into(),
                    participant_id: "participant".into(),
                    session_id: "session".into(),
                    placement: BrowserPlacement::Required {
                        desktop: BrowserDesktopBinding {
                            desktop_id: "desktop".into(),
                            generation: 1,
                        },
                    },
                },
                desktop: RemoteViewFixedDesktop {
                    desktop_id: "desktop".into(),
                    generation: 1,
                    friendly_route_label: "app".into(),
                },
            },
        );
        assert!(state.sessions.is_empty());
        maintain_only_standalone_pool(false, &state, || {
            panic!("lost registry cannot authorize app-owned pool effects")
        })
        .unwrap();
        let mut calls = 0;
        maintain_only_standalone_pool(false, &BrowserSessionState::default(), || {
            calls += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(
            maintain_only_standalone_pool(false, &BrowserSessionState::default(), || Err(
                "standalone-provider-failure".into()
            ))
            .unwrap_err(),
            "standalone-provider-failure"
        );
    }

    #[test]
    fn browser_session_runtime_configuration_performs_no_provider_request() {
        assert!(runtime_context(None, None, None, None).unwrap().is_none());
        let origin = Some("http://127.0.0.1:1".into());
        let pool = Some("main".into());
        let context = runtime_context(origin.clone(), pool.clone(), None, None)
            .unwrap()
            .unwrap();
        assert_eq!(context.pool.desired_desktops, 1);
        assert_eq!(context.pool.name, "main");
        assert!(runtime_context(origin.clone(), None, None, None).is_err());
        assert!(runtime_context(None, pool.clone(), None, None).is_err());
        assert!(runtime_context(None, None, None, Some("1".into())).is_err());
        for count in ["0", "-1", "private-secret", "4294967296"] {
            let error = runtime_context(origin.clone(), pool.clone(), None, Some(count.into()))
                .err()
                .unwrap();
            assert_eq!(error, "remote_view_runtime_desktop_count_invalid");
        }
        assert!(runtime_context(Some("http://example.test".into()), pool, None, None).is_err());
    }
}

/// Resolve the owned desktop's provider route without issuing a viewer grant.
/// Remote View owns viewer authentication and lifetime; this read only joins
/// the retained handoff to its current external desktop assignment.
pub(crate) fn resolve_handoff_desktop_route(id: &str) -> Result<String, String> {
    read_handoff_desktop_presentation(id, false).map(|(route, _)| route)
}

/// Read the owned route and current private observation together. This does not
/// register context, acknowledge use, recover a browser or focus a target.
pub(crate) fn resolve_handoff_desktop_presentation(
    id: &str,
) -> Result<(String, Option<String>), String> {
    read_handoff_desktop_presentation(id, true)
}

fn read_handoff_desktop_presentation(
    id: &str,
    include_native_context: bool,
) -> Result<(String, Option<String>), String> {
    let store = BrowserSessionSqliteStore::default_sqlite()?;
    let state = store.load_session_state()?;
    state
        .remote_view_tab_handoffs
        .get(id)
        .ok_or("remote_view_tab_handoff_missing")?
        .check_link_at(current_view_time_ms())?;
    let target = resolve_remote_view_tab_handoff(&state, id)?;
    // A persisted logical target can outlive its browser process. Public
    // navigation must reach native authentication before owner recovery;
    // a surviving desktop alone does not make the browser task available.
    // This absence check grants no custody over a present or reused PID.
    if include_native_context
        && matches!(
            crate::process_identity::observe_process(target.browser.pid),
            crate::process_identity::ProcessObservation::Missing
        )
    {
        return Err("remote_view_tab_handoff_browser_unavailable".into());
    }
    let desktop = target
        .browser
        .desktop
        .as_ref()
        .ok_or("remote_view_handoff_desktop_missing")?;
    let mut context = configured_remote_view()?.ok_or("remote_view_runtime_config_missing")?;
    let inventory = context
        .adapter
        .inventory()
        .map_err(|_| "remote_view_runtime_inventory_unavailable")?;
    let route = owned_desktop_route(&inventory, &desktop.desktop_id, desktop.generation)?;
    let binding = include_native_context
        .then(|| {
            state.remote_view_tab_handoffs[id]
                .native_observation
                .as_ref()
        })
        .flatten();
    let correlation = match binding {
        None => None,
        Some(binding) => {
            let digest = remote_view_native_target_digest(id, &target)?;
            let observation = binding
                .observation
                .as_ref()
                .ok_or("remote_view_native_registration_missing")?;
            if binding.target_digest != digest
                || observation.target_digest != digest
                || observation.target != binding.provider_target
                || binding.provider_target.desktop_id != desktop.desktop_id
                || binding.provider_target.lifecycle_generation != desktop.generation
            {
                return Err("remote_view_human_use_target_changed".into());
            }
            let assignment = inventory
                .assignments
                .iter()
                .find(|assignment| {
                    binding
                        .provider_target
                        .validate_assignment(assignment)
                        .is_ok()
                })
                .ok_or("remote_view_human_use_target_changed")?;
            let read = context
                .adapter
                .observe_native_handoff_use_state(assignment, observation)
                .map_err(|_| "remote_view_native_use_unavailable")?;
            if read.state == RemoteViewNativeHandoffUseState::Expired {
                return Err("remote_view_native_observation_expired".into());
            }
            // Mutable activity does not fence a physical target. Rejoin only
            // its exact identity and registration after the private read.
            let current = store.load_session_state()?;
            let current_link = current
                .remote_view_tab_handoffs
                .get(id)
                .ok_or("remote_view_tab_handoff_missing")?;
            current_link.check_link_at(current_view_time_ms())?;
            let current_target = resolve_remote_view_tab_handoff(&current, id)?;
            if remote_view_native_target_digest(id, &current_target)? != digest
                || current_link.native_observation.as_ref() != Some(binding)
            {
                return Err("remote_view_human_use_target_changed".into());
            }
            Some(observation.correlation_id.clone())
        }
    };
    Ok((route, correlation))
}

fn owned_desktop_route(
    inventory: &RemoteViewApplicationInventory,
    desktop_id: &str,
    generation: u64,
) -> Result<String, String> {
    if !inventory.assignments.iter().any(|assignment| {
        assignment.desktop_id == desktop_id
            && assignment.generation == generation
            && assignment.state == RemoteViewAssignmentState::Active
    }) {
        return Err("remote_view_handoff_desktop_not_owned".into());
    }
    let desktop = inventory
        .desktops
        .iter()
        .find(|desktop| desktop.desktop_id == desktop_id)
        .ok_or("remote_view_handoff_desktop_missing")?;
    let lifecycle = desktop
        .lifecycle
        .as_ref()
        .ok_or("remote_view_handoff_desktop_unavailable")?;
    let viewing = desktop
        .viewing
        .as_ref()
        .ok_or("remote_view_handoff_desktop_unavailable")?;
    if lifecycle.generation != generation
        || lifecycle.state != RemoteViewResourceState::Ready
        || lifecycle.readiness_scope != RemoteViewApplicationReadinessScope::LiveResource
        || viewing.lifecycle_generation != generation
        || viewing.desktop_id.is_empty()
        || viewing.generation == 0
    {
        return Err("remote_view_handoff_desktop_unavailable".into());
    }
    Ok(viewing.public_route.clone())
}

#[cfg(test)]
mod desktop_handoff_tests {
    use super::*;

    #[test]
    fn workspace_native_sample_age_cannot_be_renewed_by_reads() {
        let start = std::time::Instant::now();
        let probe = WorkspaceProbe {
            sample: Some((
                start,
                WorkspaceNativeSample {
                    windows: std::collections::BTreeMap::from([(("desktop".into(), 7), None)]),
                    observed_at_ms: Some(1234),
                    age_ms: 0,
                },
            )),
            ..Default::default()
        };
        let sample = probe.fresh(start + Duration::from_secs(27));
        assert_eq!(sample.observed_at_ms, Some(1234));
        assert_eq!(sample.age_ms, 27_000);
        assert_eq!(sample.windows.len(), 1);
        assert!(probe
            .fresh(start + Duration::from_secs(30))
            .windows
            .is_empty());
        assert!(probe
            .fresh(start + Duration::from_secs(31))
            .observed_at_ms
            .is_none());
    }

    #[test]
    fn workspace_delayed_native_http_converges_without_blocking_status() {
        use std::io::{Read, Write};
        use std::time::Instant;
        let guard = crate::test_utils::EnvGuard::new(&[
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
        ]);
        guard.remove("AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE");
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let desktop = RemoteViewFixedDesktop {
            desktop_id: fixture["assignment"]["desktopId"].as_str().unwrap().into(),
            generation: 7,
            friendly_route_label: "Synthetic".into(),
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        guard.set(
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            &format!("http://{}", listener.local_addr().unwrap()),
        );
        guard.set("AGENT_BROWSER_REMOTE_VIEW_APPLICATION", "agent-browser");
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(4);
            let mut operations = Vec::new();
            while operations.len() < 4 && Instant::now() < deadline {
                let Ok((mut socket, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut byte = [0];
                while !bytes.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                }
                let headers = String::from_utf8(bytes).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let operation = body["request"]["operation"].as_str().unwrap();
                let key = match operation {
                    "inventory" => "inventory",
                    "observe_assignment" => "assignmentObservation",
                    "windows" => "windows",
                    _ => panic!("observation attempted an effect"),
                };
                operations.push(operation.to_owned());
                std::thread::sleep(Duration::from_millis(if operation == "inventory" {
                    550
                } else {
                    150
                }));
                let reply = fixture[key].to_string();
                let _ = write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply);
            }
            operations
        });
        let start = Instant::now();
        let first = observe_workspace_desktops(std::slice::from_ref(&desktop));
        let first_elapsed = start.elapsed();
        let mut result = first.windows;
        while result.is_empty() && start.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(10));
            result = observe_workspace_desktops(std::slice::from_ref(&desktop)).windows;
        }
        let operations = server.join().unwrap();
        assert!(
            first_elapsed < Duration::from_millis(80),
            "dashboard waited for native I/O: {first_elapsed:?}"
        );
        assert!(
            result
                .get(&(desktop.desktop_id.clone(), 7))
                .is_some_and(Option::is_some),
            "slow healthy native provider never became observable"
        );
        assert_eq!(
            operations,
            [
                "inventory",
                "observe_assignment",
                "windows",
                "observe_assignment"
            ]
        );
        // The same provider now refuses connections. Completed failure must
        // invalidate the previous positive sample rather than extend its lease.
        std::thread::sleep(Duration::from_secs(2));
        let cached = observe_workspace_desktops(std::slice::from_ref(&desktop));
        assert!(!cached.windows.is_empty());
        let failed_at = Instant::now();
        loop {
            let failed = observe_workspace_desktops(std::slice::from_ref(&desktop));
            if failed.observed_at_ms != cached.observed_at_ms {
                assert!(failed.windows.is_empty());
                break;
            }
            assert!(failed_at.elapsed() < Duration::from_secs(1));
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut replacement = desktop;
        replacement.generation += 1;
        assert!(observe_workspace_desktops(&[replacement])
            .windows
            .is_empty());
    }

    #[test]
    fn workspace_assignment_ignores_released_history_and_rejects_ambiguity() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let mut inventory: RemoteViewApplicationInventory =
            serde_json::from_value(fixture["inventory"].clone()).unwrap();
        let active = inventory.assignments[0].clone();
        let desktop = RemoteViewFixedDesktop {
            desktop_id: active.desktop_id.clone(),
            generation: active.generation,
            friendly_route_label: "Synthetic".into(),
        };
        let mut released = active.clone();
        released.assignment_id = "old-released".into();
        released.state = RemoteViewAssignmentState::Released;
        inventory.assignments.push(released);
        assert_eq!(
            active_workspace_assignment(&inventory, &desktop).unwrap(),
            Some(&active)
        );
        inventory.assignments.push(active.clone());
        assert!(active_workspace_assignment(&inventory, &desktop).is_err());
        inventory
            .assignments
            .retain(|assignment| assignment.state == RemoteViewAssignmentState::Released);
        assert_eq!(
            active_workspace_assignment(&inventory, &desktop).unwrap(),
            None
        );
    }

    #[test]
    fn workspace_native_observation_uses_only_public_reads() {
        struct Reads {
            fixture: serde_json::Value,
            operations: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
            fail: bool,
        }
        impl RemoteViewApplicationTransport for Reads {
            fn request(
                &mut self,
                envelope: &RemoteViewApplicationEnvelope,
            ) -> Result<serde_json::Value, RemoteViewApplicationTransportError> {
                let key = match &envelope.request {
                    RemoteViewApplicationRequest::Inventory {} => "inventory",
                    RemoteViewApplicationRequest::ObserveAssignment { .. } => {
                        "assignmentObservation"
                    }
                    RemoteViewApplicationRequest::Windows { .. } => "windows",
                    _ => panic!("presence observation attempted an effect"),
                };
                self.operations.lock().unwrap().push(key);
                if self.fail {
                    return Err(RemoteViewApplicationTransportError::Unavailable);
                }
                Ok(self.fixture[key].clone())
            }
        }
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let desktop = RemoteViewFixedDesktop {
            desktop_id: fixture["assignment"]["desktopId"].as_str().unwrap().into(),
            generation: 7,
            friendly_route_label: "Synthetic".into(),
        };
        let operations = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Reads {
                fixture: fixture.clone(),
                operations: operations.clone(),
                fail: false,
            },
        )
        .unwrap();
        let result =
            observe_workspace_desktops_with(&mut adapter, std::slice::from_ref(&desktop), 0);
        assert!(result[&(desktop.desktop_id.clone(), 7)].is_some());
        assert_eq!(
            *operations.lock().unwrap(),
            [
                "inventory",
                "assignmentObservation",
                "windows",
                "assignmentObservation"
            ]
        );
        let mut adapter = RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            Reads {
                fixture,
                operations,
                fail: true,
            },
        )
        .unwrap();
        assert!(
            observe_workspace_desktops_with(&mut adapter, &[desktop], 0).is_empty(),
            "provider failure is unknown, not empty-desktop proof"
        );
    }

    #[test]
    fn desktop_handoff_requires_current_owned_assignment_and_viewing_generation() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let mut inventory: RemoteViewApplicationInventory =
            serde_json::from_value(fixture["inventory"].clone()).unwrap();
        let id = inventory.desktops[0].desktop_id.clone();
        inventory.desktops[0].viewing = Some(RemoteViewApplicationDesktopViewing {
            lifecycle_generation: 7,
            desktop_id: "33333333-3333-4333-8333-333333333333".into(),
            generation: 4,
            public_route: "/1".into(),
        });
        assert_eq!(owned_desktop_route(&inventory, &id, 7).unwrap(), "/1");
        assert!(owned_desktop_route(&inventory, &id, 8).is_err());
        inventory.desktops[0]
            .viewing
            .as_mut()
            .unwrap()
            .lifecycle_generation = 8;
        assert!(owned_desktop_route(&inventory, &id, 7).is_err());
        inventory.desktops[0]
            .viewing
            .as_mut()
            .unwrap()
            .lifecycle_generation = 7;
        inventory.assignments.clear();
        assert!(owned_desktop_route(&inventory, &id, 7).is_err());
    }
}

fn current_view_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

pub(crate) fn public_origin(value: &str) -> Result<url::Url, String> {
    let origin = url::Url::parse(value).map_err(|_| "remote_view_public_origin_invalid")?;
    if origin.scheme() != "https"
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || origin.path() != "/"
        || origin.port() == Some(0)
        || origin
            .host_str()
            .is_none_or(|host| host == "localhost" || host.ends_with(".localhost"))
        || match origin.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_unspecified(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback() || ip.is_unspecified(),
            _ => false,
        }
    {
        return Err("remote_view_public_origin_invalid".into());
    }
    Ok(origin)
}
