//! BrowserManager-backed effects for the ordinary Browser Session Manager.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use agent_browser_service_model::{
    BrowserAttachmentObservation, BrowserAttachmentRequest, BrowserDisposableProfilePolicy,
    BrowserLaunch, BrowserLaunchIntent, BrowserProfileCatalogEntry, BrowserProfileKind,
    BrowserSessionEffects, BrowserTabAcquisition, BrowserTabSource, ManagedBrowserInstance,
    ManagedBrowserTab, ManagedDisposableProfile, RemoteViewAssignmentObservation,
    RemoteViewBrowserProcessEffects, RemoteViewBrowserProcessError,
    RemoteViewDesktopRuntimeContext, RemoteViewFixedDesktop, RemoteViewPrivateLaunchEnvironment,
};
use serde_json::Value;

mod manual;

use super::action_runtime::runtime::DaemonState;
use super::browser::{BrowserManager, WaitUntil};
use super::cdp::chrome::LaunchOptions;

#[derive(Debug, Clone, Default)]
pub(crate) struct BrowserManagerRuntimeConfig {
    pub(crate) headless: bool,
    pub(crate) executable_path: Option<String>,
    pub(crate) display: Option<String>,
    pub(crate) remote_headed: bool,
    /// Optional installation capacity ceiling checked before a local browser
    /// process is launched. `None` uses the conservative host default.
    pub(crate) maximum_browser_processes: Option<u32>,
    /// Exact Remote View desktop contexts obtained from its public consumer
    /// contract. Empty means remote desktop launches remain unavailable.
    pub(crate) remote_view_desktop_contexts: Vec<RemoteViewDesktopRuntimeContext>,
}

pub(crate) trait BrowserRuntimeDriver {
    fn attach_app_browser(
        &mut self,
        _request: &BrowserAttachmentRequest,
        _profile: &BrowserProfileCatalogEntry,
        _desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserAttachmentObservation, String> {
        Err("browser_attachment_runtime_unsupported".into())
    }

    fn reconcile_profile_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        browser: Option<&ManagedBrowserInstance>,
        tabs: &[ManagedBrowserTab],
        navigation: &[agent_browser_service_model::BrowserNavigationRecord],
    ) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
        let _ = (profile, browser, tabs, navigation);
        Ok(None)
    }

    /// Pin one profile's next launches without changing another profile's build.
    fn select_profile_browser_executable(
        &mut self,
        _profile_id: String,
        _path: String,
    ) -> Result<(), String> {
        Err("browser_profile_build_selection_unsupported".into())
    }

    fn select_browser_executable(&mut self, _path: String) -> Result<(), String> {
        Err("browser_build_selection_unsupported".into())
    }

    fn begin_operation(
        &mut self,
        _expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        Ok(())
    }
    fn begin_borrowed_retirement_operation(
        &mut self,
        expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        self.begin_operation(expected)
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        None
    }
    fn acknowledge_launch_publication(&mut self) {}
    /// Query current authority before automatic expiry or measured quota retirement.
    fn permits_idle_browser_close(
        &mut self,
        _browser: &ManagedBrowserInstance,
    ) -> Result<bool, String> {
        Ok(true)
    }
    fn acquire_profile_cleanup_guard(
        &mut self,
        _allocation: &ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        Ok(Some(Box::new(())))
    }
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        for browser in browsers {
            if !self.permits_idle_browser_close(browser)? {
                return Ok(None);
            }
        }
        Ok(Some(Box::new(())))
    }
    /// Concurrent runtime drivers must qualify the whole quota group under one lock.
    fn acquire_disposable_retirement_guard(
        &mut self,
        _allocations: &[ManagedDisposableProfile],
        _browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        Err("browser_disposable_group_admission_unsupported".into())
    }
    fn apply_idle_retirement_effect(
        &mut self,
        _scope: &agent_browser_service_model::BrowserIdleRetirementScope,
        _tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        Err("browser_session_idle_original_effect_unavailable".into())
    }
    fn observe_idle_retirement_effect(
        &mut self,
        _scope: &agent_browser_service_model::BrowserIdleRetirementScope,
        _tab: Option<&ManagedBrowserTab>,
    ) -> Result<bool, String> {
        Err("browser_session_idle_effect_readback_unavailable".into())
    }
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String>;
    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String>;
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String>;
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String>;
    fn acquire_session_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
        _session: &agent_browser_service_model::ManagedBrowserSession,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.acquire_initial_tab(browser, attributed_target_ids)
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String>;
    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String>;
    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String>;
    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String>;

    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &Value,
    ) -> Result<Value, String> {
        let _ = (browser, tab, session_id, session_name, command);
        Err("browser_session_runtime_command_unsupported".to_string())
    }
}

pub(crate) struct BrowserSessionEffectAdapter<D> {
    runtime: D,
}

impl<D> BrowserSessionEffectAdapter<D> {
    pub(crate) fn new(runtime: D) -> Self {
        Self { runtime }
    }
}

pub(crate) use agent_browser_service_model::ManagedBrowserCommandEffects;

impl<D: BrowserRuntimeDriver> ManagedBrowserCommandEffects for BrowserSessionEffectAdapter<D> {
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &Value,
    ) -> Result<Value, String> {
        self.runtime
            .execute_command(browser, tab, session_id, session_name, command)
    }
}

impl<D: RemoteViewBrowserProcessEffects> RemoteViewBrowserProcessEffects
    for BrowserSessionEffectAdapter<D>
{
    fn preflight_launch(&mut self, profile: &BrowserProfileCatalogEntry) -> Result<(), String> {
        self.runtime.preflight_launch(profile)
    }

    fn prove_recovery_absence(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
    ) -> Result<(), RemoteViewBrowserProcessError> {
        self.runtime.prove_recovery_absence(browser, profile)
    }
    fn recover_launch(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        self.runtime
            .recover_launch(browser, profile, intent, environment, observation)
    }

    fn launch(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        self.runtime
            .launch(profile, intent, environment, observation)
    }
}

impl<D: BrowserRuntimeDriver> BrowserSessionEffects for BrowserSessionEffectAdapter<D> {
    fn attach_app_browser(
        &mut self,
        request: &BrowserAttachmentRequest,
        profile: &BrowserProfileCatalogEntry,
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserAttachmentObservation, String> {
        self.runtime.attach_app_browser(request, profile, desktop)
    }

    fn acquire_session_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        session: &agent_browser_service_model::ManagedBrowserSession,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.runtime
            .acquire_session_initial_tab(browser, profile, session, attributed_target_ids)
    }
    fn reconcile_profile_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        browser: Option<&ManagedBrowserInstance>,
        tabs: &[ManagedBrowserTab],
        navigation: &[agent_browser_service_model::BrowserNavigationRecord],
    ) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
        self.runtime
            .reconcile_profile_browser(profile, browser, tabs, navigation)
    }

    fn select_profile_browser_executable(
        &mut self,
        profile_id: String,
        path: String,
    ) -> Result<(), String> {
        self.runtime
            .select_profile_browser_executable(profile_id, path)
    }

    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        self.runtime.select_browser_executable(path)
    }

    fn begin_operation(
        &mut self,
        expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        self.runtime.begin_operation(expected)
    }
    fn begin_borrowed_retirement_operation(
        &mut self,
        expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        self.runtime.begin_borrowed_retirement_operation(expected)
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        self.runtime.pending_launch_intent()
    }
    fn acknowledge_launch_publication(&mut self) {
        self.runtime.acknowledge_launch_publication();
    }
    fn acquire_profile_cleanup_guard(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        self.runtime.acquire_profile_cleanup_guard(allocation)
    }
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        self.runtime.browser_is_live(browser)
    }

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        self.runtime.launch_browser(profile, desktop)
    }

    fn permits_idle_browser_close(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<bool, String> {
        self.runtime.permits_idle_browser_close(browser)
    }
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        self.runtime.acquire_idle_retirement_guard(browsers)
    }
    fn acquire_disposable_retirement_guard(
        &mut self,
        allocations: &[ManagedDisposableProfile],
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        self.runtime
            .acquire_disposable_retirement_guard(allocations, browsers)
    }
    fn apply_idle_retirement_effect(
        &mut self,
        scope: &agent_browser_service_model::BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        self.runtime.apply_idle_retirement_effect(scope, tab)
    }
    fn observe_idle_retirement_effect(
        &mut self,
        scope: &agent_browser_service_model::BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<bool, String> {
        self.runtime.observe_idle_retirement_effect(scope, tab)
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        self.runtime.close_browser(browser)
    }

    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.runtime
            .acquire_initial_tab(browser, attributed_target_ids)
    }

    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        self.runtime.create_tab(browser)
    }

    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        self.runtime.close_tab(browser, tab)
    }

    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        self.runtime.navigate(browser, tab, url)
    }

    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        self.runtime.focus_browser(browser, tab)
    }

    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation_id: &str,
        _session_name: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        let root = Path::new(&policy.user_data_root);
        if !root.is_absolute() {
            return Err("browser_disposable_root_not_absolute".to_string());
        }
        if allocation_id.is_empty()
            || allocation_id.contains('/')
            || allocation_id.contains('\\')
            || allocation_id == "."
            || allocation_id == ".."
        {
            return Err("browser_disposable_allocation_id_invalid".to_string());
        }
        create_private_directory(root)?;
        let user_data_dir = root.join(allocation_id);
        create_private_directory(&user_data_dir)?;
        Ok(BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: allocation_id.to_string(),
            name: allocation_id.to_string(),
            user_data_dir: user_data_dir.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Disposable,
        })
    }

    fn delete_disposable_profile(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<(), String> {
        if allocation.profile.kind != BrowserProfileKind::Disposable {
            return Err("browser_disposable_cleanup_kind_invalid".to_string());
        }
        let root = Path::new(&allocation.user_data_root);
        let target = Path::new(&allocation.profile.user_data_dir);
        if !root.is_absolute() || target.parent() != Some(root) || target == root {
            return Err("browser_disposable_cleanup_path_invalid".to_string());
        }
        match fs::remove_dir_all(target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("browser_disposable_cleanup_failed:{error}")),
        }
    }

    fn disposable_profile_size_bytes(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<u64, String> {
        if allocation.profile.kind != BrowserProfileKind::Disposable {
            return Err("browser_disposable_size_kind_invalid".to_string());
        }
        let root = Path::new(&allocation.user_data_root);
        let target = Path::new(&allocation.profile.user_data_dir);
        if !root.is_absolute() || target.parent() != Some(root) || target == root {
            return Err("browser_disposable_size_path_invalid".to_string());
        }
        directory_regular_file_bytes(target)
    }
}

fn directory_regular_file_bytes(root: &Path) -> Result<u64, String> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("browser_disposable_size_failed:{error}")),
    };
    if !metadata.file_type().is_dir() {
        return Err("browser_disposable_size_root_invalid".to_string());
    }
    let mut total = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("browser_disposable_size_failed:{error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("browser_disposable_size_failed:{error}"))?;
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|error| format!("browser_disposable_size_failed:{error}"))?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .ok_or_else(|| "browser_disposable_profile_bytes_exhausted".to_string())?;
            }
        }
    }
    Ok(total)
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("browser_disposable_directory_create_failed:{error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("browser_disposable_directory_permissions_failed:{error}"))?;
    }
    Ok(())
}

#[derive(Clone)]
struct BorrowedAppBrowser {
    profile_id: String,
    process: agent_browser_service_model::RecordedProcessIdentity,
    endpoint: String,
    targets: BTreeSet<String>,
}

/// Forget a stale CDP connection only after proving it owns no launched process.
/// DaemonState Drop only aborts local handlers; connected BrowserManager has no
/// BrowserProcess, so this operation sends no browser close or kernel signal.
fn invalidate_changed_borrowed_connections(
    browsers: &mut HashMap<String, DaemonState>,
    previous: &HashMap<String, BorrowedAppBrowser>,
    next: &HashMap<String, BorrowedAppBrowser>,
) -> Result<(), String> {
    let changed = next
        .iter()
        .filter(|(id, new)| {
            previous
                .get(*id)
                .is_some_and(|old| old.process != new.process || old.endpoint != new.endpoint)
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in &changed {
        if browsers
            .get(id)
            .and_then(|s| s.browser.as_ref())
            .is_some_and(BrowserManager::owns_launched_browser_process)
        {
            return Err("browser_attachment_rebind_cached_process_custody_conflict".into());
        }
    }
    for id in changed {
        browsers.remove(&id);
    }
    Ok(())
}

enum BrowserRuntimeCommand {
    AttachAppBrowser {
        request: BrowserAttachmentRequest,
        profile: BrowserProfileCatalogEntry,
        desktop: RemoteViewFixedDesktop,
        reply: mpsc::Sender<Result<BrowserAttachmentObservation, String>>,
    },
    ReconcileProfile {
        profile: BrowserProfileCatalogEntry,
        browser: Option<ManagedBrowserInstance>,
        tabs: Vec<ManagedBrowserTab>,
        navigation: Vec<agent_browser_service_model::BrowserNavigationRecord>,
        reply: mpsc::Sender<Result<Option<agent_browser_service_model::BrowserRecovery>, String>>,
    },
    OperationBaseline {
        shared_browser_ids: BTreeSet<String>,
        borrowed_app_browsers: HashMap<String, BorrowedAppBrowser>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    PreflightLaunch {
        profile: BrowserProfileCatalogEntry,
        reply: mpsc::Sender<Result<(), String>>,
    },
    SelectProfileExecutable {
        profile_id: String,
        path: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    SelectExecutable {
        path: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    IsLive {
        browser: ManagedBrowserInstance,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Launch {
        profile: BrowserProfileCatalogEntry,
        desktop: Option<RemoteViewFixedDesktop>,
        environment: Option<std::collections::BTreeMap<String, String>>,
        recovery_browser: Option<ManagedBrowserInstance>,
        reply: mpsc::Sender<Result<BrowserLaunch, String>>,
    },
    Close {
        browser: ManagedBrowserInstance,
        reply: mpsc::Sender<Result<(), String>>,
    },
    AcquireInitialTab {
        browser: ManagedBrowserInstance,
        session_context: Option<(
            BrowserProfileCatalogEntry,
            agent_browser_service_model::ManagedBrowserSession,
        )>,
        attributed_target_ids: Vec<String>,
        reply: mpsc::Sender<Result<BrowserTabAcquisition, String>>,
    },
    CreateTab {
        browser: ManagedBrowserInstance,
        reply: mpsc::Sender<Result<BrowserTabAcquisition, String>>,
    },
    CloseTab {
        browser: ManagedBrowserInstance,
        tab: ManagedBrowserTab,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Navigate {
        browser: ManagedBrowserInstance,
        tab: ManagedBrowserTab,
        url: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Focus {
        browser: ManagedBrowserInstance,
        tab: Option<ManagedBrowserTab>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Execute {
        browser: ManagedBrowserInstance,
        tab: ManagedBrowserTab,
        session_id: String,
        session_name: String,
        command: Value,
        reply: mpsc::Sender<Result<Value, String>>,
    },
    IdleRetirement {
        scope: agent_browser_service_model::BrowserIdleRetirementScope,
        tab: Option<ManagedBrowserTab>,
        apply: bool,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Shutdown,
}

pub(crate) struct BrowserManagerRuntime {
    commands: mpsc::Sender<BrowserRuntimeCommand>,
    worker: Option<thread::JoinHandle<()>>,
    #[cfg(test)]
    idle_retirement_after_check: Option<fn(&ManagedBrowserInstance) -> Result<(), String>>,
}

impl BrowserManagerRuntime {
    pub(crate) fn start(config: BrowserManagerRuntimeConfig) -> Result<Self, String> {
        let (commands, receiver) = mpsc::channel();
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("browser-session-runtime".to_string())
            .spawn(move || match super::daemon::build_runtime(2) {
                Ok(runtime) => {
                    let _ = startup_sender.send(Ok(()));
                    run_browser_worker(runtime, receiver, config);
                }
                Err(error) => {
                    let _ = startup_sender.send(Err(error));
                }
            })
            .map_err(|error| format!("browser_session_runtime_thread_failed:{error}"))?;
        match startup_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                commands,
                worker: Some(worker),
                #[cfg(test)]
                idle_retirement_after_check: None,
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                let _ = worker.join();
                Err(format!("browser_session_runtime_startup_failed:{error}"))
            }
        }
    }

    fn request<T>(
        &self,
        receiver: mpsc::Receiver<Result<T, String>>,
        command: BrowserRuntimeCommand,
    ) -> Result<T, String> {
        self.commands
            .send(command)
            .map_err(|_| "browser_session_runtime_stopped".to_string())?;
        receiver
            .recv()
            .map_err(|_| "browser_session_runtime_reply_lost".to_string())?
    }
}

/// Fresh external-mode process ingress. Durable launch admission is owned by
/// the coordinator; this adapter consumes private inputs and never retries.
impl RemoteViewBrowserProcessEffects for BrowserManagerRuntime {
    fn preflight_launch(&mut self, profile: &BrowserProfileCatalogEntry) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::PreflightLaunch {
                profile: profile.clone(),
                reply,
            },
        )
    }

    fn prove_recovery_absence(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
    ) -> Result<(), RemoteViewBrowserProcessError> {
        prove_browser_recovery_absence(browser, profile)
            .map_err(|_| RemoteViewBrowserProcessError::Rejected)
    }
    fn recover_launch(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        if intent.validate().is_err()
            || profile.id != intent.profile_id
            || profile.id != browser.profile_id
        {
            return Err(RemoteViewBrowserProcessError::Rejected);
        }
        self.prove_recovery_absence(browser, profile)?;
        let environment = environment
            .into_environment(observation, &intent.assignment)
            .map_err(|_| RemoteViewBrowserProcessError::Rejected)?;
        let desktop = RemoteViewFixedDesktop {
            desktop_id: intent.assignment.desktop_id.clone(),
            generation: intent.assignment.generation,
            friendly_route_label: String::new(),
        };
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Launch {
                profile: profile.clone(),
                desktop: Some(desktop),
                environment: Some(environment),
                recovery_browser: Some(browser.clone()),
                reply,
            },
        )
        .map_err(|_| RemoteViewBrowserProcessError::OutcomeUnknown)
    }

    fn launch(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        if intent.validate().is_err() || profile.id != intent.profile_id {
            return Err(RemoteViewBrowserProcessError::Rejected);
        }
        let environment = environment
            .into_environment(observation, &intent.assignment)
            .map_err(|_| RemoteViewBrowserProcessError::Rejected)?;
        let desktop = RemoteViewFixedDesktop {
            desktop_id: intent.assignment.desktop_id.clone(),
            generation: intent.assignment.generation,
            friendly_route_label: String::new(),
        };
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Launch {
                profile: profile.clone(),
                desktop: Some(desktop),
                environment: Some(environment),
                recovery_browser: None,
                reply,
            },
        )
        .map_err(|_| RemoteViewBrowserProcessError::OutcomeUnknown)
    }
}

impl BrowserRuntimeDriver for BrowserManagerRuntime {
    fn attach_app_browser(
        &mut self,
        request: &BrowserAttachmentRequest,
        profile: &BrowserProfileCatalogEntry,
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserAttachmentObservation, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::AttachAppBrowser {
                request: request.clone(),
                profile: profile.clone(),
                desktop: desktop.clone(),
                reply,
            },
        )
    }

    fn acquire_session_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        session: &agent_browser_service_model::ManagedBrowserSession,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::AcquireInitialTab {
                browser: browser.clone(),
                session_context: Some((profile.clone(), session.clone())),
                attributed_target_ids: attributed_target_ids.to_vec(),
                reply,
            },
        )
    }
    fn reconcile_profile_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        browser: Option<&ManagedBrowserInstance>,
        tabs: &[ManagedBrowserTab],
        navigation: &[agent_browser_service_model::BrowserNavigationRecord],
    ) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::ReconcileProfile {
                profile: profile.clone(),
                browser: browser.cloned(),
                tabs: tabs.to_vec(),
                navigation: navigation.to_vec(),
                reply,
            },
        )
    }
    fn permits_idle_browser_close(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<bool, String> {
        let state = super::service_store::load_default_service_state_snapshot()?;
        let permitted = super::service_resources::permits_managed_browser_idle_retirement(
            &state,
            browser,
            &chrono::Utc::now().to_rfc3339(),
        )?;
        #[cfg(test)]
        if permitted {
            if let Some(after_check) = self.idle_retirement_after_check {
                after_check(browser)?;
            }
        }
        Ok(permitted)
    }

    fn acquire_profile_cleanup_guard(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        let repository = super::service_store::LockedServiceStateRepository::default_json()?;
        let guarded = repository.hold_snapshot_for_effect()?;
        if !super::browser_session_store::BrowserSessionSqliteStore::default_session_state_read_only()?
            .is_some_and(|state| state.disposable_profiles.get(&allocation.profile.id) == Some(allocation)) {
            return Ok(None);
        }
        if !super::service_resources::permits_disposable_profile_cleanup(
            &guarded.state,
            allocation,
            &chrono::Utc::now().to_rfc3339(),
        )? {
            return Ok(None);
        }
        Ok(Some(Box::new(guarded)))
    }
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        let repository = super::service_store::LockedServiceStateRepository::default_json()?;
        let guarded = repository.hold_snapshot_for_effect()?;
        let now = chrono::Utc::now().to_rfc3339();
        for browser in browsers {
            if !super::service_resources::permits_managed_browser_idle_retirement(
                &guarded.state,
                browser,
                &now,
            )? {
                return Ok(None);
            }
        }
        Ok(Some(Box::new(guarded)))
    }

    fn acquire_disposable_retirement_guard(
        &mut self,
        allocations: &[ManagedDisposableProfile],
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn agent_browser_service_model::BrowserIdleRetirementGuard>>, String>
    {
        let repository = super::service_store::LockedServiceStateRepository::default_json()?;
        let guarded = repository.hold_snapshot_for_effect()?;
        if !super::browser_session_store::BrowserSessionSqliteStore::default_session_state_read_only()?
            .is_some_and(|state| allocations.iter().all(|allocation| state.disposable_profiles.get(&allocation.profile.id) == Some(allocation))) {
            return Ok(None);
        }
        if !super::service_resources::permits_disposable_retirement_group(
            &guarded.state,
            allocations,
            browsers,
            &chrono::Utc::now().to_rfc3339(),
        )? {
            return Ok(None);
        }
        Ok(Some(Box::new(guarded)))
    }

    fn begin_operation(
        &mut self,
        expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::OperationBaseline {
                shared_browser_ids: expected.shared_browser_ids.clone(),
                borrowed_app_browsers: expected
                    .app_browser_attachments
                    .iter()
                    .map(|(id, attachment)| {
                        (
                            id.clone(),
                            BorrowedAppBrowser {
                                profile_id: attachment.profile_id.clone(),
                                process: attachment.request.process.clone(),
                                endpoint: expected
                                    .browsers
                                    .get(id)
                                    .map(|browser| browser.cdp_endpoint.clone())
                                    .unwrap_or_else(|| attachment.request.cdp_endpoint.clone()),
                                targets: attachment.borrowed_tab_ids.clone(),
                            },
                        )
                    })
                    .collect(),
                reply,
            },
        )
    }
    fn select_profile_browser_executable(
        &mut self,
        profile_id: String,
        path: String,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::SelectProfileExecutable {
                profile_id,
                path,
                reply,
            },
        )
    }

    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::SelectExecutable { path, reply },
        )
    }

    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::IsLive {
                browser: browser.clone(),
                reply,
            },
        )
    }

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Launch {
                profile: profile.clone(),
                desktop: desktop.cloned(),
                environment: None,
                recovery_browser: None,
                reply,
            },
        )
    }

    fn apply_idle_retirement_effect(
        &mut self,
        scope: &agent_browser_service_model::BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        let settled = self.request(
            receiver,
            BrowserRuntimeCommand::IdleRetirement {
                scope: scope.clone(),
                tab: tab.cloned(),
                apply: true,
                reply,
            },
        )?;
        settled
            .then_some(())
            .ok_or_else(|| "browser_session_idle_effect_readback_unavailable".into())
    }
    fn observe_idle_retirement_effect(
        &mut self,
        scope: &agent_browser_service_model::BrowserIdleRetirementScope,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<bool, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::IdleRetirement {
                scope: scope.clone(),
                tab: tab.cloned(),
                apply: false,
                reply,
            },
        )
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Close {
                browser: browser.clone(),
                reply,
            },
        )
    }

    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::AcquireInitialTab {
                browser: browser.clone(),
                session_context: None,
                attributed_target_ids: attributed_target_ids.to_vec(),
                reply,
            },
        )
    }

    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::CreateTab {
                browser: browser.clone(),
                reply,
            },
        )
    }

    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::CloseTab {
                browser: browser.clone(),
                tab: tab.clone(),
                reply,
            },
        )
    }

    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Navigate {
                browser: browser.clone(),
                tab: tab.clone(),
                url: url.to_string(),
                reply,
            },
        )
    }

    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Focus {
                browser: browser.clone(),
                tab: tab.cloned(),
                reply,
            },
        )
    }

    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &Value,
    ) -> Result<Value, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Execute {
                browser: browser.clone(),
                tab: tab.clone(),
                session_id: session_id.to_string(),
                session_name: session_name.to_string(),
                command: command.clone(),
                reply,
            },
        )
    }
}

impl Drop for BrowserManagerRuntime {
    fn drop(&mut self) {
        let _ = self.commands.send(BrowserRuntimeCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

// The receive boundary owns no browser state and never dispatches recovery.
fn receive_runtime_command<T>(
    receiver: &mpsc::Receiver<T>,
    mut poll_owned_children: impl FnMut(),
) -> Result<T, mpsc::RecvError> {
    loop {
        // Poll before dispatch as well as while idle so a busy queue cannot
        // indefinitely retain exited direct children. No browser state changes.
        poll_owned_children();
        match receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(command) => return Ok(command),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(mpsc::RecvError),
        }
    }
}

fn run_browser_worker(
    runtime: tokio::runtime::Runtime,
    receiver: mpsc::Receiver<BrowserRuntimeCommand>,
    mut config: BrowserManagerRuntimeConfig,
) {
    let mut browsers = HashMap::<String, DaemonState>::new();
    let mut manual = manual::ManualBrowsers::default();
    let mut profile_executables: HashMap<String, String> = HashMap::new();
    let mut shared_browser_tabs = HashMap::<String, BTreeSet<String>>::new();
    let mut borrowed_app_browsers = HashMap::<String, BorrowedAppBrowser>::new();
    let mut reconciled_targets = HashMap::<(String, String), String>::new();
    let mut next_browser_sequence = 0_u64;
    while let Ok(command) = receive_runtime_command(&receiver, || {
        manual.poll_owned_exits();
        for state in browsers.values_mut() {
            if let Some(manager) = state.browser.as_mut() {
                if manager.owns_launched_browser_process() {
                    // try_wait reaps only this manager's Child; retain all state.
                    let _ = manager.poll_process_exit();
                }
            }
        }
    }) {
        match command {
            BrowserRuntimeCommand::ReconcileProfile {
                profile,
                browser,
                reply,
                tabs,
                ..
            } if profile.manual_login_url.is_some() => {
                if let Some(browser) = browser.as_ref().filter(|browser| {
                    super::browser_session_host::browser_mode_host::recovery_authorized(
                        &profile, browser,
                    )
                }) {
                    let _ = reply.send(manual::reconcile_mode(&profile, browser, &tabs));
                    continue;
                }
                if browser.is_none() {
                    let _ = reply.send(manual::reconcile_initial(&profile));
                    continue;
                }
                let result = browser
                    .as_ref()
                    .map_or(Ok(()), |browser| {
                        if !manual::is_manual(browser) {
                            return Err("browser_login_mode_conflict".into());
                        }
                        manual.verify(browser).and_then(|live| {
                            if live {
                                Ok(())
                            } else {
                                Err("manual_login_ended: close the login task before reopening"
                                    .into())
                            }
                        })
                    })
                    .map(|()| None);
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Launch {
                profile,
                desktop,
                environment,
                recovery_browser,
                reply,
            } if profile.manual_login_url.is_some() => {
                let result = (|| {
                    if recovery_browser.as_ref().is_some_and(|browser| {
                        !super::browser_session_host::browser_mode_host::recovery_authorized(
                            &profile, browser,
                        )
                    }) {
                        return Err("manual_login_recovery_requires_explicit_reopen".into());
                    }
                    let private =
                        environment.ok_or("manual_login_requires_native_remote_desktop")?;
                    let display = private
                        .get("DISPLAY")
                        .cloned()
                        .ok_or("manual_login_display_unavailable")?;
                    let executable = profile_executables
                        .get(&profile.id)
                        .cloned()
                        .or_else(|| config.executable_path.clone())
                        .or_else(|| {
                            super::cdp::chrome::find_chrome()
                                .map(|p| p.to_string_lossy().into_owned())
                        })
                        .ok_or("manual_login_browser_unavailable")?;
                    let options = LaunchOptions {
                        headless: false,
                        manual_login: true,
                        attachable: false,
                        executable_path: Some(executable),
                        profile: Some(profile.user_data_dir.clone()),
                        args: vec![profile
                            .manual_login_url
                            .clone()
                            .expect("matched login mode")],
                        display: Some(display),
                        remote_headed: true,
                        private_launch_environment: Some(private),
                        ..LaunchOptions::default()
                    };
                    let mut launch = manual.launch(&profile, desktop, options)?;
                    if let Some(browser) = recovery_browser {
                        manual.retain_logical_identity(&launch.browser_id, &browser.id)?;
                        launch.browser_id = browser.id;
                    }
                    Ok(launch)
                })();
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::IsLive { browser, reply } if manual::is_manual(&browser) => {
                let _ = reply.send(manual.verify(&browser));
            }
            BrowserRuntimeCommand::Close { browser, reply } if manual::is_manual(&browser) => {
                let _ = reply.send(manual.close(&browser));
            }
            BrowserRuntimeCommand::AcquireInitialTab {
                browser,
                attributed_target_ids,
                reply,
                ..
            } if manual::is_manual(&browser) => {
                let result = manual.verify(&browser).and_then(|live| {
                    if !live {
                        return Err("manual_login_process_exited".into());
                    }
                    if !attributed_target_ids.is_empty() {
                        return Err("manual_login_profile_in_use".into());
                    }
                    Ok(manual::window_tab(&browser))
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::CreateTab { browser, reply } if manual::is_manual(&browser) => {
                let _ = reply.send(Err("manual_login_additional_tabs_unavailable".into()));
            }
            BrowserRuntimeCommand::CloseTab { browser, reply, .. }
                if manual::is_manual(&browser) =>
            {
                // The single human window is closed by the manager's final-task
                // browser close, with exact original-process readback.
                let _ = reply.send(manual.verify(&browser).map(|_| ()));
            }
            BrowserRuntimeCommand::Navigate {
                browser,
                url,
                reply,
                ..
            } if manual::is_manual(&browser) => {
                let _ = reply.send(
                    manual
                        .execute(
                            &browser,
                            &serde_json::json!({"action":"navigate","url":url}),
                        )
                        .map(|_| ()),
                );
            }
            BrowserRuntimeCommand::Focus { browser, reply, .. } if manual::is_manual(&browser) => {
                let _ = reply.send(manual.focus(&browser));
            }
            BrowserRuntimeCommand::Execute {
                browser,
                command,
                reply,
                ..
            } if manual::is_manual(&browser) => {
                let _ = reply.send(manual.execute(&browser, &command));
            }
            BrowserRuntimeCommand::IdleRetirement {
                scope,
                apply,
                reply,
                ..
            } if manual::is_manual(&scope.browser) => {
                // A manual task owns one native window. Its only synthetic tab
                // retires with that exact process; another task cannot share it.
                let result = if apply {
                    manual.close(&scope.browser).map(|()| true)
                } else {
                    manual.verify(&scope.browser).map(|live| !live)
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::OperationBaseline {
                shared_browser_ids,
                borrowed_app_browsers: retained_attachments,
                reply,
            } => {
                for id in shared_browser_ids {
                    shared_browser_tabs.entry(id).or_default();
                }
                if let Err(error) = invalidate_changed_borrowed_connections(
                    &mut browsers,
                    &borrowed_app_browsers,
                    &retained_attachments,
                ) {
                    let _ = reply.send(Err(error));
                    continue;
                }
                borrowed_app_browsers.extend(retained_attachments);
                let _ = reply.send(Ok(()));
            }
            BrowserRuntimeCommand::AttachAppBrowser {
                request,
                profile,
                desktop,
                reply,
            } => {
                let result = runtime.block_on(async {
                    let (observation, manager) =
                        attach_requested_app_browser(&request, &profile, &desktop).await?;
                    let id = observation.launch.browser_id.clone();
                    let mut state = DaemonState::new();
                    state.browser = Some(manager);
                    state.browser_session_manager_owned = true;
                    borrowed_app_browsers.insert(
                        id.clone(),
                        BorrowedAppBrowser {
                            profile_id: profile.id.clone(),
                            process: request.process,
                            endpoint: observation.launch.cdp_endpoint.clone(),
                            targets: observation.borrowed_target_ids.clone(),
                        },
                    );
                    shared_browser_tabs.entry(id.clone()).or_default();
                    browsers.insert(id, state);
                    Ok(observation)
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::ReconcileProfile {
                profile,
                browser,
                tabs,
                navigation,
                reply,
            } => {
                if borrowed_app_browsers
                    .values()
                    .any(|attachment| attachment.profile_id == profile.id)
                {
                    let _ = reply.send(Err(
                        "browser_attachment_recovery_requires_app_authority".into()
                    ));
                    continue;
                }
                let result = runtime.block_on(async {
                    if browser
                        .as_ref()
                        .is_some_and(|browser| browser.profile_id != profile.id)
                    {
                        return Err("browser_session_recovery_identity_conflict".into());
                    }
                    let Some((launch, mut manager)) = attach_current_profile_browser(
                        &profile,
                        browser.as_ref().map(|browser| browser.id.as_str()),
                        None,
                    )
                    .await?
                    else {
                        return Ok(None);
                    };
                    let live_targets = manager
                        .tab_list(true)
                        .iter()
                        .filter_map(|tab| tab["targetId"].as_str().map(str::to_owned))
                        .collect::<BTreeSet<_>>();
                    let mut target_ids = std::collections::BTreeMap::new();
                    for tab in &tabs {
                        let key = (launch.browser_id.clone(), tab.id.clone());
                        let target = if live_targets.contains(&tab.target_id) {
                            tab.target_id.clone()
                        } else if let Some(target) = reconciled_targets
                            .get(&key)
                            .filter(|target| live_targets.contains(*target))
                        {
                            target.clone()
                        } else {
                            let url = navigation
                                .iter()
                                .filter(|row| {
                                    row.tab_id == tab.id && row.browser_id == tab.browser_id
                                })
                                .max_by_key(|row| row.visited_at_ms)
                                .map(|row| row.url.as_str());
                            #[cfg(target_os = "linux")]
                            let target = {
                                let physical = ManagedBrowserInstance {
                                    id: launch.browser_id.clone(),
                                    profile_id: profile.id.clone(),
                                    pid: launch.pid,
                                    cdp_endpoint: launch.cdp_endpoint.clone(),
                                    desktop: launch.desktop.clone(),
                                    active_session_ids: Vec::new(),
                                };
                                let attributed = tabs
                                    .iter()
                                    .map(|tab| tab.target_id.clone())
                                    .chain(target_ids.values().cloned())
                                    .collect::<Vec<_>>();
                                let acquired =
                                    super::browser_session_profile_reuse::acquire_task_tab(
                                        &mut manager,
                                        &physical,
                                        &profile,
                                        "retained-tab",
                                        &tab.id,
                                        &attributed,
                                    )
                                    .await?;
                                if acquired.created {
                                    if let Some(url) = url {
                                        manager.navigate(url, WaitUntil::DomContentLoaded).await?;
                                    }
                                }
                                acquired.acquisition.target_id
                            };
                            #[cfg(not(target_os = "linux"))]
                            let target = {
                                let created = manager.tab_new_background(url).await?;
                                created["targetId"]
                                    .as_str()
                                    .ok_or("browser_session_recovery_target_missing")?
                                    .to_owned()
                            };
                            reconciled_targets.insert(key, target.clone());
                            target
                        };
                        target_ids.insert(tab.id.clone(), target);
                    }
                    let mut state = DaemonState::new();
                    state.browser = Some(manager);
                    state.browser_session_manager_owned = true;
                    browsers.insert(launch.browser_id.clone(), state);
                    shared_browser_tabs
                        .entry(launch.browser_id.clone())
                        .or_default()
                        .extend(target_ids.values().cloned());
                    Ok(Some(agent_browser_service_model::BrowserRecovery {
                        launch,
                        target_ids,
                    }))
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::PreflightLaunch { profile, reply } => {
                let path = Path::new(&profile.user_data_dir);
                let result = if !path.is_absolute() {
                    Err("browser_session_profile_path_not_absolute".to_string())
                } else if path.exists() && !path.is_dir() {
                    Err("browser_session_profile_path_not_directory".to_string())
                } else {
                    super::browser_launch_admission::wait_for_browser_launch_admission(
                        path,
                        config.maximum_browser_processes,
                    )
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::SelectProfileExecutable {
                profile_id,
                path,
                reply,
            } => {
                profile_executables.insert(profile_id, path);
                let _ = reply.send(Ok(()));
            }
            BrowserRuntimeCommand::SelectExecutable { path, reply } => {
                config.executable_path = Some(path);
                let _ = reply.send(Ok(()));
            }
            BrowserRuntimeCommand::IsLive { browser, reply } => {
                let result = if let Some(attachment) = borrowed_app_browsers.get(&browser.id) {
                    verify_borrowed_app_browser(&browser, attachment).and_then(|()| {
                        runtime.block_on(recorded_browser_is_live(&mut browsers, &browser))
                    })
                } else {
                    runtime.block_on(recorded_browser_is_live(&mut browsers, &browser))
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Launch {
                profile,
                desktop,
                environment,
                recovery_browser,
                reply,
            } => {
                if borrowed_app_browsers
                    .values()
                    .any(|attachment| attachment.profile_id == profile.id)
                {
                    let _ = reply.send(Err(
                        "browser_attachment_launch_requires_app_authority".into()
                    ));
                    continue;
                }
                let result = runtime.block_on(async {
                    if recovery_browser.is_none() && environment.is_none() && desktop.is_none() {
                        if let Some((launch, manager)) =
                            attach_current_profile_browser(&profile, None, None).await?
                        {
                            let mut state = DaemonState::new();
                            state.browser = Some(manager);
                            state.browser_session_manager_owned = true;
                            browsers.insert(launch.browser_id.clone(), state);
                            shared_browser_tabs.insert(launch.browser_id.clone(), BTreeSet::new());
                            return Ok(launch);
                        }
                    }
                    if let Some(browser) = &recovery_browser {
                        prove_browser_recovery_absence(browser, &profile)?;
                        browsers.remove(&browser.id);
                    }
                    let sequence = next_browser_sequence
                        .checked_add(1)
                        .ok_or_else(|| "browser_session_browser_sequence_exhausted".to_string())?;
                    let remote_display = if let Some(environment) = &environment {
                        Some(environment.get("DISPLAY").cloned().ok_or_else(|| {
                            "browser_private_launch_environment_invalid".to_string()
                        })?)
                    } else {
                        desktop
                            .as_ref()
                            .map(|desktop| {
                                resolve_remote_view_display(
                                    desktop,
                                    &config.remote_view_desktop_contexts,
                                )
                            })
                            .transpose()?
                    };
                    super::browser_launch_admission::wait_for_browser_launch_admission(
                        Path::new(&profile.user_data_dir),
                        config.maximum_browser_processes,
                    )?;
                    let display = remote_display.or_else(|| config.display.clone());
                    let mut manager = BrowserManager::launch(
                        LaunchOptions {
                            headless: config.headless && display.is_none(),
                            executable_path: profile_executables
                                .get(&profile.id)
                                .cloned()
                                .or_else(|| config.executable_path.clone()),
                            profile: Some(profile.user_data_dir.clone()),
                            display,
                            remote_headed: config.remote_headed || desktop.is_some(),
                            private_launch_environment: environment,
                            ..LaunchOptions::default()
                        },
                        Some("chrome"),
                    )
                    .await?;
                    manager.approve_lifecycle_close();
                    let pid = manager
                        .browser_pid()
                        .ok_or_else(|| "browser_session_launch_pid_missing".to_string())?;
                    // The unreaped child handle prevents PID reuse while this
                    // launch observation is captured. Missing evidence is retained
                    // as missing and must never be backfilled during retirement.
                    let process_identity = if manager.poll_process_exit().is_none() {
                        crate::process_identity::capture_process_identity(pid, None, None)
                    } else {
                        None
                    };
                    if manager.poll_process_exit().is_some() {
                        return Err("browser_session_launch_process_exited".into());
                    }
                    next_browser_sequence = sequence;
                    let browser_id = recovery_browser.as_ref().map_or_else(
                        || format!("browser:{}:{}", profile.id, uuid::Uuid::new_v4()),
                        |browser| browser.id.clone(),
                    );
                    let launch = BrowserLaunch {
                        process_identity,
                        browser_id: browser_id.clone(),
                        pid,
                        cdp_endpoint: manager.get_cdp_url().to_string(),
                        desktop,
                    };
                    let mut state = DaemonState::new();
                    state.browser = Some(manager);
                    state.browser_session_manager_owned = true;
                    shared_browser_tabs.remove(&browser_id);
                    reconciled_targets.retain(|(id, _), _| id != &browser_id);
                    browsers.insert(browser_id, state);
                    Ok(launch)
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Close { browser, reply } => {
                if borrowed_app_browsers.contains_key(&browser.id) {
                    let _ =
                        reply.send(Err("browser_attachment_close_requires_app_authority".into()));
                    continue;
                }
                if let Some(targets) = shared_browser_tabs.remove(&browser.id) {
                    let result = runtime.block_on(async {
                        if let Some(mut state) = browsers.remove(&browser.id) {
                            if let Some(manager) = state.browser.as_mut() {
                                for target in targets {
                                    manager.tab_close_target_id_for_release(&target).await?;
                                }
                            }
                        }
                        Ok(())
                    });
                    let _ = reply.send(result);
                    continue;
                }
                let result = match browsers.remove(&browser.id) {
                    Some(mut state) => runtime.block_on(async {
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                        if manager.owns_launched_browser_process() {
                            manager.approve_lifecycle_close();
                            let outcome = manager.close_with_outcome().await?;
                            if !outcome.exact_process_exited {
                                return Err("browser_session_close_exit_unproven".to_string());
                            }
                            Ok(())
                        } else {
                            close_reattached_browser(manager, browser.pid).await
                        }
                    }),
                    None => runtime.block_on(close_unattached_browser(
                        &browser,
                        crate::process_identity::observe_process(browser.pid),
                    )),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::AcquireInitialTab {
                browser,
                session_context,
                attributed_target_ids,
                reply,
            } => {
                #[cfg(not(target_os = "linux"))]
                let _ = session_context.as_ref();
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                        if let Some(targets) = shared_browser_tabs.get_mut(&browser.id) {
                            #[cfg(target_os = "linux")]
                            if let Some((profile, session)) = session_context.as_ref() {
                                let acquisition =
                                    super::browser_session_profile_reuse::acquire_task_tab(
                                        manager,
                                        &browser,
                                        profile,
                                        "session",
                                        &session.name,
                                        &attributed_target_ids,
                                    )
                                    .await?
                                    .acquisition;
                                targets.insert(acquisition.target_id.clone());
                                return Ok(acquisition);
                            }
                            let tab = manager.tab_new_background(None).await?;
                            let acquisition =
                                tab_acquisition(tab, BrowserTabSource::SessionInitial)?;
                            targets.insert(acquisition.target_id.clone());
                            return Ok(acquisition);
                        }
                        if let Some(tab) = manager.tab_list(true).into_iter().find(|tab| {
                            tab.get("targetId")
                                .and_then(Value::as_str)
                                .is_some_and(|target_id| {
                                    !attributed_target_ids.iter().any(|id| id == target_id)
                                })
                        }) {
                            return tab_acquisition(tab, BrowserTabSource::Bootstrap);
                        }
                        let tab = manager.tab_new(None).await?;
                        tab_acquisition(tab, BrowserTabSource::SessionInitial)
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::CreateTab { browser, reply } => {
                let result = runtime.block_on(async {
                    // A retained owner record survives runtime restart; the
                    // process-local CDP manager must be verified and reattached
                    // before creating a new target in that recorded browser.
                    if !recorded_browser_is_live(&mut browsers, &browser).await? {
                        return Err("browser_session_runtime_browser_missing".into());
                    }
                    let manager = browsers
                        .get_mut(&browser.id)
                        .and_then(|state| state.browser.as_mut())
                        .ok_or("browser_session_runtime_browser_missing")?;
                    let tab = if shared_browser_tabs.contains_key(&browser.id) {
                        manager.tab_new_background(None).await?
                    } else {
                        manager.tab_new(None).await?
                    };
                    let acquisition = tab_acquisition(tab, BrowserTabSource::ExplicitNew)?;
                    if let Some(targets) = shared_browser_tabs.get_mut(&browser.id) {
                        targets.insert(acquisition.target_id.clone());
                    }
                    Ok(acquisition)
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::CloseTab {
                browser,
                tab,
                reply,
            } => {
                if borrowed_app_browsers
                    .get(&browser.id)
                    .is_some_and(|attachment| attachment.targets.contains(&tab.target_id))
                {
                    let _ = reply.send(Err(
                        "browser_attachment_borrowed_tab_close_requires_app_authority".into(),
                    ));
                    continue;
                }
                let result = runtime.block_on(async {
                    if !recorded_browser_is_live(&mut browsers, &browser).await? {
                        return if !observed_process_presence(
                            crate::process_identity::observe_process(browser.pid),
                        )? {
                            Ok(())
                        } else {
                            Err("browser_session_reattach_unresponsive".into())
                        };
                    }
                    let manager = browsers
                        .get_mut(&browser.id)
                        .and_then(|state| state.browser.as_mut())
                        .ok_or("browser_session_runtime_browser_missing")?;
                    close_owned_task_tab(manager, &browser, &tab).await?;
                    if let Some(targets) = shared_browser_tabs.get_mut(&browser.id) {
                        targets.remove(&tab.target_id);
                    }
                    Ok(())
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Navigate {
                browser,
                tab,
                url,
                reply,
            } => {
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                        attach_owned_target_if_missing(manager, &tab.target_id).await?;
                        manager.tab_switch_target_id(&tab.target_id).await?;
                        manager.navigate(&url, WaitUntil::Load).await?;
                        Ok(())
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Focus {
                browser,
                tab,
                reply,
            } => {
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                        if let Some(tab) = tab {
                            attach_owned_target_if_missing(manager, &tab.target_id).await?;
                            manager.tab_switch_target_id(&tab.target_id).await?;
                        }
                        manager.focus_for_view(true).await?;
                        Ok(())
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Execute {
                browser,
                tab,
                command,
                reply,
                ..
            } if command["action"] == "tab_close" && command["operatorClose"] == true => {
                let result = super::browser_host_control::guarded(&browser.id, &command, || {
                    runtime.block_on(async {
                        use super::service_store::{
                            LockedServiceStateRepository, ServiceStateRepository,
                        };
                        let repository = LockedServiceStateRepository::default_json()?;
                        super::stream::verify_operator_tab_close(
                            &repository.load_snapshot()?,
                            &command,
                        )?;
                        if command["targetId"] != tab.target_id
                            || command["browserId"] != browser.id
                            || !recorded_browser_is_live(&mut browsers, &browser).await?
                        {
                            return Err("operator_tab_close_process_changed".to_string());
                        }
                        let state = browsers
                            .get_mut(&browser.id)
                            .ok_or("operator_tab_close_process_changed")?;
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or("operator_tab_close_process_changed")?;
                        if manager.browser_pid() != Some(browser.pid)
                            || manager.get_cdp_url() != browser.cdp_endpoint
                        {
                            return Err("operator_tab_close_process_changed".into());
                        }
                        super::stream::verify_operator_tab_close(
                            &repository.load_snapshot()?,
                            &command,
                        )?;
                        let data = manager
                            .tab_close_target_id_for_release(&tab.target_id)
                            .await?;
                        if let Some(targets) = shared_browser_tabs.get_mut(&browser.id) {
                            targets.remove(&tab.target_id);
                        }
                        state.ref_map.clear();
                        state.iframe_sessions.clear();
                        state.active_frame_id = None;
                        Ok(serde_json::json!({"id":command["id"],"success":true,"data":data}))
                    })
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Execute {
                browser,
                tab,
                session_id,
                session_name,
                command,
                reply,
            } => {
                let mut command = command;
                if let Err(error) = super::browser_host_policy::bind_command_task(
                    &mut command,
                    &session_id,
                    &session_name,
                ) {
                    let _ = reply.send(Err(error));
                    continue;
                }
                let result = super::browser_host_control::guarded(&browser.id, &command, || {
                    match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        if command["action"] == "view_focus" && command["operatorFocus"] == true {
                            // No marker clearing or target activation may precede
                            // the current operator proof and native effect fence.
                            let data = super::browser_lifecycle::handle_view_focus(&command, state)
                                .await?;
                            return Ok(
                                serde_json::json!({"id":command["id"],"success":true,"data":data}),
                            );
                        }
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                        if let Some(marker) = manager
                            .pages_list()
                            .into_iter()
                            .find(|page| {
                                page.target_id == tab.target_id
                                    && page.url.starts_with("about:blank#agent-browser-task-")
                            })
                            .map(|page| page.url)
                        {
                            let _ = manager
                                .clear_initial_task_marker(&tab.target_id, &marker)
                                .await;
                        }
                        attach_owned_target_if_missing(manager, &tab.target_id).await?;
                        // Read the addressed task or popup inventory without
                        // replacing the person's physical selection. Other
                        // actions retain exact-target activation semantics.
                        if super::browser_host_control::observation(command["action"].as_str().unwrap_or_default()) {
                            manager.bind_observation_target(&tab.target_id).await?;
                        } else {
                            manager.tab_switch_target_id(&tab.target_id).await?;
                        }
                        if command["action"] == "__observe_owned_popup_targets" {
                            let observed = manager
                                .client
                                .send_command_no_params("Target.getTargets", None)
                                .await?;
                            let children = observed["targetInfos"]
                                .as_array()
                                .ok_or("browser_session_target_inventory_unavailable")?
                                .iter()
                                .filter(|target| target["type"] == "page")
                                .filter_map(|target| {
                                    Some(serde_json::json!({
                                        "targetId":target["targetId"].as_str()?,
                                        "openerId":target["openerId"].as_str()?,
                                    }))
                                })
                                .collect::<Vec<_>>();
                            return Ok(serde_json::json!({"children":children}));
                        }
                        state.session_id = session_id;
                        state.session_name = Some(session_name);
                        Ok(super::actions::execute_managed_target_command(
                            &command,
                            state,
                            &tab.target_id,
                        )
                        .await)
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                }
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::IdleRetirement {
                scope,
                tab,
                apply,
                reply,
            } => {
                if borrowed_app_browsers
                    .get(&scope.browser.id)
                    .is_some_and(|attachment| {
                        tab.as_ref()
                            .is_none_or(|tab| attachment.targets.contains(&tab.target_id))
                    })
                {
                    let _ = reply.send(Err(
                        "browser_attachment_retirement_requires_app_authority".into()
                    ));
                    continue;
                }
                let mut release_original_cache = false;
                let result = runtime.block_on(async {
                    if apply
                        && tab.is_none()
                        && original_retirement_process(
                            &scope,
                            crate::process_identity::observe_process(scope.browser.pid),
                        )?
                        .is_some()
                    {
                        if let Some(manager) = browsers
                            .get_mut(&scope.browser.id)
                            .and_then(|state| state.browser.as_mut())
                        {
                            if manager.browser_pid() == Some(scope.browser.pid)
                                && manager.get_cdp_url() == scope.browser.cdp_endpoint
                            {
                                manager.approve_lifecycle_close();
                                release_original_cache = true;
                            }
                        }
                    }
                    original_retirement_effect(&scope, tab.as_ref(), apply).await
                });
                if release_original_cache && result.as_ref().is_ok_and(|settled| *settled) {
                    browsers.remove(&scope.browser.id);
                }
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Shutdown => break,
        }
    }
    for state in browsers.values_mut() {
        if let Some(manager) = state.browser.as_mut() {
            manager.relinquish_browser_for_handoff();
        }
    }
}

/// Reconcile only the owner-selected task target through the verified browser.
/// Fresh absence is outcome evidence, never authority to act on another target.
async fn close_owned_task_tab(
    manager: &mut BrowserManager,
    browser: &ManagedBrowserInstance,
    tab: &ManagedBrowserTab,
) -> Result<(), String> {
    if tab.browser_id != browser.id
        || tab.target_id.is_empty()
        || manager.get_cdp_url() != browser.cdp_endpoint
    {
        return Err("browser_session_owned_target_identity_conflict".into());
    }
    let before = match crate::process_identity::observe_process(browser.pid) {
        crate::process_identity::ProcessObservation::Observed(identity)
            if identity.start_token.is_some() && identity.executable_path.is_some() =>
        {
            identity
        }
        _ => return Err("browser_session_recorded_process_missing".into()),
    };
    let inventory = tokio::time::timeout(
        Duration::from_secs(2),
        manager
            .client
            .send_command_no_params("Target.getTargets", None),
    )
    .await
    .map_err(|_| "browser_session_target_inventory_unavailable")??;
    let targets = inventory
        .get("targetInfos")
        .and_then(Value::as_array)
        .ok_or("browser_session_target_inventory_unavailable")?;
    let mut present = false;
    for target in targets {
        let id = target
            .get("targetId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or("browser_session_target_inventory_unavailable")?;
        target
            .get("type")
            .and_then(Value::as_str)
            .filter(|kind| !kind.is_empty())
            .ok_or("browser_session_target_inventory_unavailable")?;
        present |= id == tab.target_id;
    }
    if present {
        return manager
            .tab_close_target_id_for_release(&tab.target_id)
            .await
            .map(|_| ());
    }
    let processes = tokio::time::timeout(
        Duration::from_secs(2),
        manager
            .client
            .send_command_no_params("SystemInfo.getProcessInfo", None),
    )
    .await
    .map_err(|_| "browser_session_profile_endpoint_identity_conflict")??;
    let bound = processes
        .get("processInfo")
        .and_then(Value::as_array)
        .is_some_and(|processes| {
            processes.iter().any(|process| {
                process["type"] == "browser"
                    && process["id"].as_u64() == Some(u64::from(browser.pid))
            })
        });
    if !bound
        || crate::process_identity::observe_process(browser.pid)
            != crate::process_identity::ProcessObservation::Observed(before)
    {
        return Err("browser_session_profile_endpoint_identity_conflict".into());
    }
    manager.remove_page_by_target_id(&tab.target_id);
    Ok(())
}

/// Attach a freshly attributed popup through the existing browser transport.
/// The caller has already selected a durable tab owned by this exact browser.
async fn attach_owned_target_if_missing(
    manager: &mut BrowserManager,
    target_id: &str,
) -> Result<(), String> {
    if manager.has_target(target_id) {
        return Ok(());
    }
    let observed = manager
        .client
        .send_command(
            "Target.getTargetInfo",
            Some(serde_json::json!({"targetId":target_id})),
            None,
        )
        .await?;
    if observed["targetInfo"]["targetId"] != target_id || observed["targetInfo"]["type"] != "page" {
        return Err("browser_session_owned_target_missing".into());
    }
    let attached = manager
        .client
        .send_command(
            "Target.attachToTarget",
            Some(serde_json::json!({"targetId":target_id,"flatten":true})),
            None,
        )
        .await?;
    let session_id = attached["sessionId"]
        .as_str()
        .ok_or("browser_session_popup_attach_invalid")?
        .to_owned();
    if let Err(error) = manager.enable_domains_pub(&session_id).await {
        let _ = manager
            .client
            .send_command(
                "Target.detachFromTarget",
                Some(serde_json::json!({"sessionId":session_id})),
                None,
            )
            .await;
        return Err(error);
    }
    manager.add_page_with_activation(
        super::browser::PageInfo {
            target_id: target_id.to_owned(),
            session_id,
            url: String::new(),
            title: String::new(),
            target_type: "page".into(),
        },
        false,
    );
    Ok(())
}

fn resolve_remote_view_display(
    desktop: &RemoteViewFixedDesktop,
    contexts: &[RemoteViewDesktopRuntimeContext],
) -> Result<String, String> {
    let matching_id = contexts
        .iter()
        .filter(|context| context.desktop_id == desktop.desktop_id)
        .collect::<Vec<_>>();
    if matching_id.is_empty() {
        return Err(format!(
            "remote_view_desktop_runtime_context_unavailable:{}:{}",
            desktop.desktop_id, desktop.generation
        ));
    }
    let context = matching_id
        .into_iter()
        .find(|context| context.generation == desktop.generation)
        .ok_or_else(|| {
            format!(
                "remote_view_desktop_runtime_context_stale:{}:{}",
                desktop.desktop_id, desktop.generation
            )
        })?;
    if context.display_name.is_empty()
        || context.display_name.contains(char::is_whitespace)
        || context.display_name.contains('\0')
    {
        return Err(format!(
            "remote_view_desktop_runtime_context_invalid:{}:{}",
            desktop.desktop_id, desktop.generation
        ));
    }
    Ok(context.display_name.clone())
}

/// Validate current OS evidence before touching the CDP transport.
fn validate_app_attachment_process(
    request: &BrowserAttachmentRequest,
    profile: &BrowserProfileCatalogEntry,
    observed: &crate::process_identity::ObservedProcessIdentity,
) -> Result<(), String> {
    let expected = &request.process;
    if expected.pid == 0
        || expected.start_token.is_empty()
        || expected
            .executable_path
            .as_ref()
            .is_none_or(|path| path.is_empty())
        || observed.pid != expected.pid
        || observed.start_token.as_deref() != Some(expected.start_token.as_str())
        || observed.executable_path != expected.executable_path
        || expected
            .browser_family
            .as_ref()
            .is_some_and(|family| observed.browser_family.as_ref() != Some(family))
    {
        return Err("browser_attachment_process_identity_conflict".into());
    }
    let expected_profile = fs::canonicalize(&profile.user_data_dir)
        .map_err(|_| "browser_attachment_profile_unavailable")?;
    let requested_profile = fs::canonicalize(&request.profile_path)
        .map_err(|_| "browser_attachment_profile_unavailable")?;
    if requested_profile != expected_profile {
        return Err("browser_attachment_profile_identity_conflict".into());
    }
    let arguments = observed
        .command_line
        .as_ref()
        .ok_or("browser_attachment_profile_identity_unavailable")?;
    let supplied = crate::process_identity::command_line_option_value(arguments, "--user-data-dir")
        .ok_or("browser_attachment_profile_identity_unavailable")?;
    let observed_profile =
        fs::canonicalize(supplied).map_err(|_| "browser_attachment_profile_unavailable")?;
    if observed_profile != expected_profile {
        return Err("browser_attachment_profile_identity_conflict".into());
    }
    Ok(())
}

fn verify_borrowed_app_browser(
    browser: &ManagedBrowserInstance,
    attachment: &BorrowedAppBrowser,
) -> Result<(), String> {
    let observed = match crate::process_identity::observe_process(attachment.process.pid) {
        crate::process_identity::ProcessObservation::Observed(observed) => observed,
        _ => return Err("browser_attachment_process_unavailable".into()),
    };
    if browser.pid != attachment.process.pid
        || browser.cdp_endpoint != attachment.endpoint
        || observed.start_token.as_deref() != Some(attachment.process.start_token.as_str())
        || observed.executable_path != attachment.process.executable_path
    {
        return Err("browser_attachment_process_identity_conflict".into());
    }
    Ok(())
}

fn app_attachment_browser_id(
    request: &BrowserAttachmentRequest,
    profile: &BrowserProfileCatalogEntry,
    endpoint: &str,
) -> Result<String, String> {
    use sha2::Digest;
    let canonical_profile = fs::canonicalize(&profile.user_data_dir)
        .map_err(|_| "browser_attachment_profile_unavailable")?;
    let bytes = serde_json::to_vec(&(&request.process, canonical_profile, endpoint))
        .map_err(|_| "browser_attachment_identity_invalid")?;
    Ok(format!("browser:app:{:x}", sha2::Sha256::digest(bytes)))
}

/// Every page already present belongs to the app, including pre-existing
/// popups whose opener is the requested tab. Later targets are not backfilled.
fn attachment_borrowed_target_census(
    snapshot: &Value,
    selected: &str,
) -> Result<BTreeSet<String>, String> {
    let targets = snapshot["targetInfos"]
        .as_array()
        .ok_or("browser_attachment_target_census_unavailable")?;
    let mut borrowed = BTreeSet::new();
    for target in targets.iter().filter(|target| target["type"] == "page") {
        let id = target["targetId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("browser_attachment_target_census_invalid")?;
        if !borrowed.insert(id.to_owned()) {
            return Err("browser_attachment_target_census_invalid".into());
        }
    }
    if !borrowed.contains(selected) {
        return Err("browser_attachment_target_missing".into());
    }
    Ok(borrowed)
}

async fn attach_requested_app_browser(
    request: &BrowserAttachmentRequest,
    profile: &BrowserProfileCatalogEntry,
    desktop: &RemoteViewFixedDesktop,
) -> Result<(BrowserAttachmentObservation, BrowserManager), String> {
    request.validate()?;
    let before = match crate::process_identity::observe_process(request.process.pid) {
        crate::process_identity::ProcessObservation::Observed(before) => before,
        _ => return Err("browser_attachment_process_unavailable".into()),
    };
    validate_app_attachment_process(request, profile, &before)?;
    let manager =
        attach_verified_browser_endpoint(&request.cdp_endpoint, request.process.pid, before)
            .await?;
    let target_snapshot = manager
        .client
        .send_command_no_params("Target.getTargets", None)
        .await?;
    let borrowed_target_ids =
        attachment_borrowed_target_census(&target_snapshot, &request.target_id)?;
    let after = match crate::process_identity::observe_process(request.process.pid) {
        crate::process_identity::ProcessObservation::Observed(after) => after,
        _ => return Err("browser_attachment_process_unavailable".into()),
    };
    validate_app_attachment_process(request, profile, &after)?;
    let observation = BrowserAttachmentObservation {
        borrowed_target_ids,
        assignment: None,
        launch: BrowserLaunch {
            process_identity: Some(request.process.clone()),
            browser_id: app_attachment_browser_id(request, profile, manager.get_cdp_url())?,
            pid: request.process.pid,
            cdp_endpoint: manager.get_cdp_url().into(),
            desktop: Some(desktop.clone()),
        },
        target_id: request.target_id.clone(),
    };
    Ok((observation, manager))
}

/// Resolve endpoint metadata only after this exact process/profile is qualified.
#[cfg(target_os = "linux")]
fn current_profile_debug_endpoint(
    expected: &Path,
    before: &crate::process_identity::ObservedProcessIdentity,
) -> Result<String, String> {
    match fs::symlink_metadata(expected.join("DevToolsActivePort")) {
        Ok(_) => {
            let (port, browser_path) = super::cdp::chrome::read_devtools_active_port(expected)
                .filter(|(port, path)| *port != 0 && path.starts_with("/devtools/browser/"))
                .ok_or("browser_session_profile_debug_endpoint_unavailable")?;
            Ok(format!("ws://127.0.0.1:{port}{browser_path}"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // The snapshot belongs to the process whose exact profile was just
            // proved. Discovery stays on that explicit loopback port; the
            // existing attach fence still proves CDP PID and kernel continuity.
            let port = before
                .command_line
                .as_ref()
                .and_then(|arguments| {
                    crate::process_identity::command_line_option_value(
                        arguments,
                        "--remote-debugging-port",
                    )
                })
                .and_then(|value| value.parse::<u16>().ok())
                .filter(|port| *port != 0)
                .ok_or("browser_session_profile_debug_endpoint_unavailable")?;
            Ok(format!("http://127.0.0.1:{port}"))
        }
        Err(_) => Err("browser_session_profile_debug_endpoint_unavailable".into()),
    }
}

/// Current OS and CDP evidence, rather than saved inventory, qualifies reuse.
#[cfg(target_os = "linux")]
async fn attach_current_profile_browser(
    profile: &BrowserProfileCatalogEntry,
    browser_id: Option<&str>,
    desktop: Option<RemoteViewFixedDesktop>,
) -> Result<Option<(BrowserLaunch, BrowserManager)>, String> {
    let path = Path::new(&profile.user_data_dir);
    let Some(before) =
        super::browser_session_profile_reuse::current_profile_browser_identity(path)?
    else {
        return Ok(None);
    };
    let pid = before.pid;
    let expected = path
        .canonicalize()
        .map_err(|_| "browser_session_profile_unavailable")?;
    let endpoint = current_profile_debug_endpoint(&expected, &before)?;
    let process_identity = before
        .start_token
        .clone()
        .zip(before.executable_path.clone())
        .map(|(start_token, executable_path)| {
            agent_browser_service_model::RecordedProcessIdentity {
                pid,
                start_token,
                executable_path: Some(executable_path),
                browser_family: before.browser_family.clone(),
            }
        });
    let manager = attach_verified_browser_endpoint(&endpoint, pid, before).await?;
    Ok(Some((
        BrowserLaunch {
            process_identity,
            browser_id: browser_id
                .map(str::to_owned)
                .unwrap_or_else(|| format!("browser:{}:{}", profile.id, uuid::Uuid::new_v4())),
            pid,
            cdp_endpoint: manager.get_cdp_url().into(),
            desktop,
        },
        manager,
    )))
}

#[cfg(not(target_os = "linux"))]
async fn attach_current_profile_browser(
    _profile: &BrowserProfileCatalogEntry,
    _browser_id: Option<&str>,
    _desktop: Option<RemoteViewFixedDesktop>,
) -> Result<Option<(BrowserLaunch, BrowserManager)>, String> {
    Ok(None)
}

async fn recorded_browser_is_live(
    browsers: &mut HashMap<String, DaemonState>,
    browser: &ManagedBrowserInstance,
) -> Result<bool, String> {
    if !recorded_browser_process_exists(browser) {
        browsers.remove(&browser.id);
        return Ok(false);
    }
    if !browsers.contains_key(&browser.id) {
        let manager = match attach_recorded_browser(browser).await {
            Ok(manager) => manager,
            Err(_) => return Ok(false),
        };
        let mut state = DaemonState::new();
        state.browser = Some(manager);
        state.browser_session_manager_owned = true;
        browsers.insert(browser.id.clone(), state);
    }
    let state = browsers
        .get_mut(&browser.id)
        .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
    let manager = state
        .browser
        .as_mut()
        .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
    let process_matches = if manager.owns_launched_browser_process() {
        manager.browser_pid() == Some(browser.pid) && manager.poll_process_exit().is_none()
    } else {
        recorded_browser_process_exists(browser)
    };
    if !process_matches {
        browsers.remove(&browser.id);
        return Ok(false);
    }
    Ok(manager.is_connection_alive().await)
}

pub(super) async fn attach_verified_browser_endpoint(
    endpoint: &str,
    pid: u32,
    before: crate::process_identity::ObservedProcessIdentity,
) -> Result<BrowserManager, String> {
    let endpoint = super::browser::resolve_cdp_url(endpoint).await?;
    let client =
        std::sync::Arc::new(agent_browser_cdp::client::CdpClient::connect(&endpoint).await?);
    let processes = client
        .send_command_no_params("SystemInfo.getProcessInfo", None)
        .await?;
    let matches = processes
        .get("processInfo")
        .and_then(Value::as_array)
        .is_some_and(|processes| {
            processes.iter().any(|process| {
                process["type"] == "browser" && process["id"].as_u64() == Some(u64::from(pid))
            })
        });
    if !matches
        || crate::process_identity::observe_process(pid)
            != crate::process_identity::ProcessObservation::Observed(before)
    {
        return Err("browser_session_profile_endpoint_identity_conflict".into());
    }
    BrowserManager::connect_cdp_background(endpoint, client).await
}

async fn attach_recorded_browser(
    browser: &ManagedBrowserInstance,
) -> Result<BrowserManager, String> {
    // The current CDP browser PID and stable kernel identity qualify a retained
    // endpoint. Executable-name classification is only a hint, not custody.
    let before = match crate::process_identity::observe_process(browser.pid) {
        crate::process_identity::ProcessObservation::Observed(identity)
            if identity.executable_path.is_some() && identity.start_token.is_some() =>
        {
            identity
        }
        _ => return Err("browser_session_recorded_process_missing".into()),
    };
    let manager = attach_verified_browser_endpoint(&browser.cdp_endpoint, browser.pid, before)
        .await
        .map_err(|error| format!("browser_session_reattach_failed:{error}"))?;
    if !manager.is_connection_alive().await {
        return Err("browser_session_reattach_unresponsive".into());
    }
    Ok(manager)
}

fn original_retirement_process(
    scope: &agent_browser_service_model::BrowserIdleRetirementScope,
    observation: crate::process_identity::ProcessObservation,
) -> Result<Option<crate::process_identity::ObservedProcessIdentity>, String> {
    use crate::process_identity::ProcessObservation;
    if scope.browser.pid == 0
        || scope.browser.cdp_endpoint.is_empty()
        || scope.process_identity.as_ref().is_some_and(|identity| {
            identity.pid != scope.browser.pid
                || identity.start_token.is_empty()
                || identity
                    .executable_path
                    .as_ref()
                    .is_none_or(|path| path.is_empty())
        })
    {
        return Err("browser_session_idle_original_identity_conflict".into());
    }
    let observed = match observation {
        ProcessObservation::Missing => return Ok(None),
        ProcessObservation::Failed { .. } => {
            return Err("browser_session_idle_effect_readback_unavailable".into());
        }
        ProcessObservation::Observed(observed) => observed,
    };
    let original = scope
        .process_identity
        .as_ref()
        .ok_or("browser_session_idle_original_identity_missing")?;
    if original.pid != scope.browser.pid
        || observed.pid != original.pid
        || original.start_token.is_empty()
        || original
            .executable_path
            .as_ref()
            .is_none_or(|path| path.is_empty())
    {
        return Err("browser_session_idle_original_identity_conflict".into());
    }
    let start = observed
        .start_token
        .as_ref()
        .ok_or("browser_session_idle_effect_readback_unavailable")?;
    if *start != original.start_token {
        // Positive kernel-instance replacement proves the original ended. This
        // observation never supplies authority over the current process.
        return Ok(None);
    }
    if observed.executable_path != original.executable_path {
        return Err("browser_session_idle_original_identity_conflict".into());
    }
    Ok(Some(observed))
}

async fn original_retirement_targets(
    manager: &BrowserManager,
    scope: &agent_browser_service_model::BrowserIdleRetirementScope,
) -> Result<Option<BTreeSet<String>>, String> {
    if original_retirement_process(
        scope,
        crate::process_identity::observe_process(scope.browser.pid),
    )?
    .is_none()
    {
        return Ok(None);
    }
    let result = manager
        .client
        .send_command_no_params("Target.getTargets", None)
        .await
        .map_err(|_| "browser_session_idle_effect_readback_unavailable")?;
    if original_retirement_process(
        scope,
        crate::process_identity::observe_process(scope.browser.pid),
    )?
    .is_none()
    {
        return Ok(None);
    }
    let targets = result
        .get("targetInfos")
        .and_then(Value::as_array)
        .ok_or("browser_session_idle_effect_readback_unavailable")?;
    let mut pages = BTreeSet::new();
    for target in targets {
        if matches!(target["type"].as_str(), Some("page" | "webview")) {
            let id = target["targetId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or("browser_session_idle_effect_readback_unavailable")?;
            pages.insert(id.to_string());
        }
    }
    Ok(Some(pages))
}

async fn original_retirement_effect(
    scope: &agent_browser_service_model::BrowserIdleRetirementScope,
    tab: Option<&ManagedBrowserTab>,
    apply: bool,
) -> Result<bool, String> {
    if tab.is_some_and(|tab| {
        !scope.tabs.iter().any(|original| {
            original.id == tab.id
                && original.target_id == tab.target_id
                && original.browser_id == tab.browser_id
                && tab.browser_id == scope.browser.id
        })
    }) {
        return Err("browser_session_idle_original_target_conflict".into());
    }
    let Some(before) = original_retirement_process(
        scope,
        crate::process_identity::observe_process(scope.browser.pid),
    )?
    else {
        return Ok(true);
    };
    if tab.is_none() && !apply {
        return Ok(false);
    }
    let mut manager = match attach_verified_browser_endpoint(
        &scope.browser.cdp_endpoint,
        scope.browser.pid,
        before,
    )
    .await
    {
        Ok(manager) => manager,
        Err(_) => {
            return match original_retirement_process(
                scope,
                crate::process_identity::observe_process(scope.browser.pid),
            )? {
                None => Ok(true),
                Some(_) => Err("browser_session_idle_effect_readback_unavailable".into()),
            };
        }
    };
    if original_retirement_process(
        scope,
        crate::process_identity::observe_process(scope.browser.pid),
    )?
    .is_none()
    {
        return Ok(true);
    }
    if let Some(tab) = tab {
        let Some(targets) = original_retirement_targets(&manager, scope).await? else {
            return Ok(true);
        };
        if !targets.contains(&tab.target_id) {
            return Ok(true);
        }
        if !apply {
            return Ok(false);
        }
        // The existing target-release owner proves disappearance and preserves
        // every physical peer. Background attachment never activates a peer.
        manager
            .tab_close_target_id_for_release(&tab.target_id)
            .await
            .map_err(|_| "browser_session_idle_effect_readback_unavailable")?;
        return match original_retirement_targets(&manager, scope).await? {
            None => Ok(true),
            Some(targets) if !targets.contains(&tab.target_id) => Ok(true),
            Some(_) => Err("browser_session_idle_effect_readback_unavailable".into()),
        };
    }
    let _close_reply = manager
        .client
        .send_command_no_params("Browser.close", None)
        .await;
    let deadline = Instant::now() + Duration::from_secs(5);
    while original_retirement_process(
        scope,
        crate::process_identity::observe_process(scope.browser.pid),
    )?
    .is_some()
    {
        if Instant::now() >= deadline {
            return Err("browser_session_idle_effect_readback_unavailable".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(true)
}

async fn close_unattached_browser(
    browser: &ManagedBrowserInstance,
    observation: crate::process_identity::ProcessObservation,
) -> Result<(), String> {
    if !observed_process_presence(observation)? {
        return Ok(());
    }
    let manager = attach_recorded_browser(browser).await?;
    close_reattached_browser(&manager, browser.pid).await
}

async fn close_reattached_browser(
    manager: &BrowserManager,
    recorded_pid: u32,
) -> Result<(), String> {
    let close_error = manager
        .client
        .send_command_no_params("Browser.close", None)
        .await
        .err();
    wait_for_recorded_process_exit(
        || crate::process_identity::observe_process(recorded_pid),
        close_error,
    )
    .await
}

async fn wait_for_recorded_process_exit(
    mut observe: impl FnMut() -> crate::process_identity::ProcessObservation,
    close_error: Option<String>,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while observed_process_presence(observe())? {
        if Instant::now() >= deadline {
            return Err(match close_error {
                Some(error) => {
                    format!("browser_session_close_exit_unproven:close_request_failed:{error}")
                }
                None => "browser_session_close_exit_unproven".to_string(),
            });
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}

fn prove_browser_recovery_absence(
    browser: &ManagedBrowserInstance,
    profile: &BrowserProfileCatalogEntry,
) -> Result<(), String> {
    if browser.profile_id != profile.id {
        return Err("browser_session_recovery_process_absence_unproven".into());
    }
    #[cfg(not(target_os = "linux"))]
    if browser.pid == 0 {
        return Err("browser_session_recovery_process_absence_unproven".into());
    }
    let expected = fs::canonicalize(&profile.user_data_dir)
        .map_err(|_| "browser_session_recovery_profile_unavailable")?;
    // On Linux the requested profile's current lock and open files qualify
    // occupancy. A saved PID may now belong to unrelated work.
    #[cfg(target_os = "linux")]
    prove_current_profile_lock_absence(&expected)?;
    #[cfg(not(target_os = "linux"))]
    prove_recovery_pid_absence(
        crate::process_identity::observe_process(browser.pid),
        &expected,
    )?;
    prove_profile_process_census_absence(&expected)
}

/// Positive current absence for an unpublished launch. Unsupported platforms
/// cannot substitute missing journal evidence for an operating-system proof.
pub(crate) fn prove_profile_launch_absence(
    profile: &BrowserProfileCatalogEntry,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let expected = fs::canonicalize(&profile.user_data_dir)
            .map_err(|_| "browser_session_recovery_profile_unavailable")?;
        prove_current_profile_lock_absence(&expected)?;
        prove_profile_process_census_absence(&expected)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = profile;
        Err("browser_session_launch_absence_unsupported".into())
    }
}

fn prove_profile_process_census_absence(expected: &Path) -> Result<(), String> {
    let system = sysinfo::System::new_all();
    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        if !name.contains("chrome") && !name.contains("chromium") {
            continue;
        }
        let args = process
            .cmd()
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        if args.is_empty() {
            #[cfg(target_os = "linux")]
            continue;
            #[cfg(not(target_os = "linux"))]
            return Err("browser_session_recovery_profile_census_unavailable".into());
        }
        if let Some(path) =
            crate::process_identity::command_line_option_value(&args, "--user-data-dir")
        {
            let observed = match fs::canonicalize(path) {
                Ok(path) => path,
                #[cfg(target_os = "linux")]
                Err(_) => continue,
                #[cfg(not(target_os = "linux"))]
                Err(_) => return Err("browser_session_recovery_profile_census_unavailable".into()),
            };
            if observed == expected {
                return Err("browser_session_recovery_profile_occupied".into());
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn prove_current_profile_lock_absence(expected: &Path) -> Result<(), String> {
    if super::browser_session_profile_reuse::current_profile_browser_identity(expected)?.is_some() {
        Err("browser_session_recovery_profile_occupied".into())
    } else {
        Ok(())
    }
}

#[cfg(any(not(target_os = "linux"), test))]
fn prove_recovery_pid_absence(
    observation: crate::process_identity::ProcessObservation,
    expected_profile: &std::path::Path,
) -> Result<(), String> {
    use crate::process_identity::ProcessObservation;
    let unrelated = match observation {
        ProcessObservation::Missing => true,
        ProcessObservation::Failed { .. } => false,
        ProcessObservation::Observed(identity) => {
            if identity.executable_path.is_none() {
                false
            } else if identity.browser_family.is_none() {
                true
            } else {
                identity.command_line.as_ref().is_some_and(|arguments| {
                    crate::process_identity::command_line_option_value(arguments, "--user-data-dir")
                        .and_then(|path| fs::canonicalize(path).ok())
                        .is_some_and(|path| path != expected_profile)
                })
            }
        }
    };
    unrelated
        .then_some(())
        .ok_or_else(|| "browser_session_recovery_process_absence_unproven".into())
}

/// A failed census is neither positive liveness nor positive exit evidence.
fn observed_process_presence(
    observation: crate::process_identity::ProcessObservation,
) -> Result<bool, String> {
    match observation {
        crate::process_identity::ProcessObservation::Observed(_) => Ok(true),
        crate::process_identity::ProcessObservation::Missing => Ok(false),
        crate::process_identity::ProcessObservation::Failed { .. } => {
            Err("browser_session_process_observation_unavailable".into())
        }
    }
}

fn recorded_browser_process_exists(browser: &ManagedBrowserInstance) -> bool {
    process_exists(browser.pid)
}

fn process_exists(pid: u32) -> bool {
    matches!(
        crate::process_identity::observe_process(pid),
        crate::process_identity::ProcessObservation::Observed(_)
    )
}

fn tab_acquisition(
    value: Value,
    source: BrowserTabSource,
) -> Result<BrowserTabAcquisition, String> {
    let target_id = value
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "browser_session_tab_target_missing".to_string())?
        .to_string();
    Ok(BrowserTabAcquisition {
        tab_id: target_id.clone(),
        target_id,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn profile_build_pin_does_not_change_unrelated_launches() {
        let root = TempDirectory::new();
        let default = root
            .0
            .join("missing-default-chrome")
            .to_string_lossy()
            .into_owned();
        let pinned = root
            .0
            .join("missing-pinned-chrome")
            .to_string_lossy()
            .into_owned();
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
            headless: true,
            executable_path: Some(default.clone()),
            ..Default::default()
        })
        .unwrap();
        runtime
            .select_profile_browser_executable("source".into(), pinned.clone())
            .unwrap();
        for (id, expected) in [("source", &pinned), ("unrelated", &default)] {
            let directory = root.0.join(id);
            fs::create_dir_all(&directory).unwrap();
            let profile = BrowserProfileCatalogEntry {
                id: id.into(),
                name: id.into(),
                user_data_dir: directory.to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Named,
                manual_login_url: None,
            };
            let failure = runtime.launch_browser(&profile, None).unwrap_err();
            assert!(
                failure.contains(expected),
                "wrong executable selected: {failure}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn owner_profile_pin_keeps_real_launch_executables_isolated_after_restart() {
        use agent_browser_service_model::{
            BrowserProfileCatalog, BrowserSessionManager, BrowserSessionManagerConfig,
            BrowserSessionState, OpenBrowserSession,
        };
        use std::os::unix::fs::PermissionsExt;
        struct Peer(std::process::Child);
        impl Drop for Peer {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut peer = Peer(
            std::process::Command::new("/bin/sleep")
                .arg("300")
                .spawn()
                .unwrap(),
        );
        let observe_peer = |pid| match crate::process_identity::observe_process(pid) {
            crate::process_identity::ProcessObservation::Observed(observed) => (
                observed.pid,
                observed.start_token.expect("peer kernel start is required"),
                observed
                    .executable_path
                    .expect("peer executable is required"),
            ),
            observation => panic!("owned live peer must remain observable: {observation:?}"),
        };
        let original_peer = observe_peer(peer.0.id());
        assert_eq!(
            original_peer.2,
            fs::canonicalize("/bin/sleep")
                .unwrap()
                .to_string_lossy()
                .as_ref()
        );
        let root = TempDirectory::new();
        let mut catalog = BrowserProfileCatalog::default();
        for id in ["pinned", "unrelated"] {
            let directory = root.0.join(id);
            fs::create_dir_all(&directory).unwrap();
            catalog.profiles.insert(
                id.into(),
                BrowserProfileCatalogEntry {
                    id: id.into(),
                    name: id.into(),
                    user_data_dir: directory.to_string_lossy().into_owned(),
                    kind: BrowserProfileKind::Named,
                    manual_login_url: None,
                },
            );
        }
        // These reviewed synthetic executables record the launch boundary then
        // exit. They cannot open a browser, attach CDP or access a private profile.
        for name in ["default", "selected"] {
            let path = root.0.join(format!("{name}-chrome"));
            fs::write(
                &path,
                "#!/bin/sh\nprofile=\nfor argument in \"$@\"; do\n  case \"$argument\" in --user-data-dir=*) profile=${argument#--user-data-dir=} ;; esac\ndone\nprintf '%s\\n' \"$profile\" >> \"$0.launches\"\nexit 71\n",
            )
            .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut state = BrowserSessionState::default();
        state.profile_executables.insert(
            "pinned".into(),
            root.0
                .join("selected-chrome")
                .to_string_lossy()
                .into_owned(),
        );
        let mut previous_launches = [0; 2];
        for order in [["pinned", "unrelated"], ["unrelated", "pinned"]] {
            // The second generation restores persisted pins and reverses launch order.
            state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
            let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
                headless: true,
                executable_path: Some(root.0.join("default-chrome").to_string_lossy().into_owned()),
                ..Default::default()
            })
            .unwrap();
            let mut effects = BrowserSessionEffectAdapter::new(runtime);
            for id in order {
                let result = BrowserSessionManager::new(
                    &mut state,
                    &catalog,
                    &mut effects,
                    BrowserSessionManagerConfig::default(),
                )
                .open(OpenBrowserSession::exact_profile(id, id, 100));
                assert!(
                    result.is_err(),
                    "synthetic launcher deliberately exits before readiness"
                );
            }
            drop(effects);
            for (index, (name, profile_id)) in [("selected", "pinned"), ("default", "unrelated")]
                .into_iter()
                .enumerate()
            {
                let attempts = fs::read_to_string(root.0.join(format!("{name}-chrome.launches")))
                    .unwrap_or_default();
                let profiles = attempts.lines().collect::<Vec<_>>();
                assert!(
                    profiles.len() > previous_launches[index],
                    "each generation must exercise the {name} executable"
                );
                let expected_profile = root.0.join(profile_id);
                for actual_profile in &profiles[previous_launches[index]..] {
                    assert_eq!(
                        *actual_profile,
                        expected_profile.to_string_lossy(),
                        "every startup attempt must use its own profile's executable"
                    );
                }
                previous_launches[index] = profiles.len();
            }
            assert!(peer.0.try_wait().unwrap().is_none());
            assert_eq!(observe_peer(peer.0.id()), original_peer);
        }
        assert!(
            state.browsers.is_empty(),
            "failed synthetic launches publish no ready browser"
        );
        peer.0.kill().unwrap();
        peer.0.wait().unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn runtime_idle_receive_reaps_exited_owned_child_without_touching_live_peer() {
        use std::process::{Child, Command, Stdio};
        struct OwnedChildren(Child, Child);
        impl Drop for OwnedChildren {
            fn drop(&mut self) {
                for child in [&mut self.0, &mut self.1] {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        fn start(pid: u32) -> String {
            let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
            stat.rsplit_once(") ")
                .unwrap()
                .1
                .split_whitespace()
                .nth(19)
                .unwrap()
                .to_string()
        }
        let spawn = || {
            Command::new("/bin/sleep")
                .arg("60")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        };
        let mut children = OwnedChildren(spawn(), spawn());
        let dead_pid = children.0.id();
        let peer_pid = children.1.id();
        let peer_start = start(peer_pid);
        children.0.kill().unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut children = children;
            let command = receive_runtime_command(&receiver, || {
                let _ = children.0.try_wait().unwrap();
                assert!(children.1.try_wait().unwrap().is_none());
            });
            assert_eq!(command.unwrap(), "stop");
        });
        // No command is sent while absence is checked. Only the idle owner polls.
        let deadline = Instant::now() + Duration::from_secs(3);
        while Path::new(&format!("/proc/{dead_pid}")).exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let reaped_while_idle = !Path::new(&format!("/proc/{dead_pid}")).exists();
        let peer_preserved =
            Path::new(&format!("/proc/{peer_pid}")).exists() && start(peer_pid) == peer_start;
        // Unblock and join even on failure so both exact child handles are reaped.
        sender.send("stop").unwrap();
        worker.join().unwrap();
        assert!(
            reaped_while_idle,
            "owned exited child remained while runtime was idle"
        );
        assert!(
            peer_preserved,
            "live peer changed during owned-child polling"
        );
        assert!(!Path::new(&format!("/proc/{dead_pid}")).exists());
        assert!(!Path::new(&format!("/proc/{peer_pid}")).exists());
    }

    #[test]
    fn runtime_idle_receive_exits_when_command_channel_disconnects() {
        let (sender, receiver) = mpsc::channel::<()>();
        drop(sender);
        assert!(receive_runtime_command(&receiver, || {}).is_err());
    }

    #[test]
    fn app_attachment_process_requires_exact_kernel_executable_and_profile_identity() {
        let root = TempDirectory::new();
        let profile_path = root.0.join("profile");
        let other_path = root.0.join("peer");
        fs::create_dir_all(&profile_path).unwrap();
        fs::create_dir_all(&other_path).unwrap();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "app".into(),
            name: "App".into(),
            user_data_dir: profile_path.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let request = BrowserAttachmentRequest {
            version: 1,
            profile_path: profile.user_data_dir.clone(),
            process: agent_browser_service_model::RecordedProcessIdentity {
                pid: 42,
                start_token: "kernel-start".into(),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: Some("chrome".into()),
            },
            cdp_endpoint: "ws://127.0.0.1:1/devtools/browser/fixture".into(),
            target_id: "app-target".into(),
            shared_with: vec![],
        };
        let observed = crate::process_identity::ObservedProcessIdentity {
            pid: 42,
            start_token: Some("kernel-start".into()),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line: Some(vec![format!("--user-data-dir={}", profile_path.display())]),
        };
        validate_app_attachment_process(&request, &profile, &observed).unwrap();
        for changed in [
            crate::process_identity::ObservedProcessIdentity {
                pid: 43,
                ..observed.clone()
            },
            crate::process_identity::ObservedProcessIdentity {
                start_token: Some("reused-pid".into()),
                ..observed.clone()
            },
            crate::process_identity::ObservedProcessIdentity {
                executable_path: Some("/peer/chrome".into()),
                ..observed.clone()
            },
            crate::process_identity::ObservedProcessIdentity {
                command_line: None,
                ..observed.clone()
            },
            crate::process_identity::ObservedProcessIdentity {
                command_line: Some(vec![format!("--user-data-dir={}", other_path.display())]),
                ..observed.clone()
            },
        ] {
            assert!(validate_app_attachment_process(&request, &profile, &changed).is_err());
        }
        let mut wrong_profile = request.clone();
        wrong_profile.profile_path = other_path.to_string_lossy().into_owned();
        assert_eq!(
            validate_app_attachment_process(&wrong_profile, &profile, &observed).unwrap_err(),
            "browser_attachment_profile_identity_conflict"
        );
        let mut missing_executable = request.clone();
        missing_executable.process.executable_path = None;
        assert!(validate_app_attachment_process(&missing_executable, &profile, &observed).is_err());
    }

    #[test]
    fn app_attachment_census_preserves_preexisting_child_pages_without_claiming_later_popups() {
        let snapshot = serde_json::json!({"targetInfos":[
            {"type":"page","targetId":"selected"},
            {"type":"page","targetId":"preexisting-popup","openerId":"selected"},
            {"type":"page","targetId":"peer"},
            {"type":"service_worker","targetId":"worker"}
        ]});
        let borrowed = attachment_borrowed_target_census(&snapshot, "selected").unwrap();
        assert_eq!(
            borrowed,
            BTreeSet::from(["selected".into(), "preexisting-popup".into(), "peer".into()])
        );
        assert!(!borrowed.contains("later-popup"));
        assert!(attachment_borrowed_target_census(&snapshot, "worker").is_err());
        assert!(attachment_borrowed_target_census(&serde_json::json!({}), "selected").is_err());
    }

    #[test]
    fn app_attachment_id_is_stable_for_same_process_and_changes_with_pid_epoch() {
        let root = TempDirectory::new();
        fs::create_dir_all(&root.0).unwrap();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "profile".into(),
            name: "Profile".into(),
            user_data_dir: root.0.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let request = BrowserAttachmentRequest {
            version: 1,
            profile_path: profile.user_data_dir.clone(),
            process: agent_browser_service_model::RecordedProcessIdentity {
                pid: 42,
                start_token: "start".into(),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: None,
            },
            cdp_endpoint: "ws://127.0.0.1:1/devtools/browser/test".into(),
            target_id: "first".into(),
            shared_with: vec![],
        };
        let first = app_attachment_browser_id(&request, &profile, &request.cdp_endpoint).unwrap();
        let mut changed = request.clone();
        changed.target_id = "second".into();
        assert_eq!(
            first,
            app_attachment_browser_id(&changed, &profile, &changed.cdp_endpoint).unwrap()
        );
        changed.process.start_token = "new-epoch".into();
        assert_ne!(
            first,
            app_attachment_browser_id(&changed, &profile, &changed.cdp_endpoint).unwrap()
        );
    }

    #[test]
    fn app_attachment_baseline_blocks_borrowed_close_and_recovery_without_process_effects() {
        let mut runtime =
            BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default()).unwrap();
        let request = BrowserAttachmentRequest {
            version: 1,
            profile_path: "/synthetic/profile".into(),
            process: agent_browser_service_model::RecordedProcessIdentity {
                pid: u32::MAX,
                start_token: "original".into(),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: None,
            },
            cdp_endpoint: "ws://127.0.0.1:1/devtools/browser/test".into(),
            target_id: "borrowed-target".into(),
            shared_with: vec![],
        };
        let desktop = RemoteViewFixedDesktop {
            desktop_id: "desktop".into(),
            generation: 1,
            friendly_route_label: "app".into(),
        };
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "profile".into(),
            name: "Profile".into(),
            user_data_dir: "/synthetic/profile".into(),
            kind: BrowserProfileKind::Named,
        };
        let browser = ManagedBrowserInstance {
            id: "borrowed-browser".into(),
            profile_id: profile.id.clone(),
            pid: request.process.pid,
            cdp_endpoint: request.cdp_endpoint.clone(),
            desktop: Some(desktop.clone()),
            active_session_ids: vec![],
        };
        let mut state = agent_browser_service_model::BrowserSessionState::default();
        state.shared_browser_ids.insert(browser.id.clone());
        state.app_browser_attachments.insert(
            browser.id.clone(),
            agent_browser_service_model::BrowserAppAttachment {
                assignment: None,
                request,
                owner: agent_browser_service_model::BrowserHostIdentity {
                    host_id: "app".into(),
                    participant_id: "participant".into(),
                    session_id: "session".into(),
                },
                borrowed_tab_ids: BTreeSet::from(["borrowed-target".into()]),
                profile_id: profile.id.clone(),
                desktop,
            },
        );
        runtime.begin_operation(&state).unwrap();
        assert_eq!(
            runtime.close_browser(&browser).unwrap_err(),
            "browser_attachment_close_requires_app_authority"
        );
        let tab = ManagedBrowserTab {
            id: "logical-tab".into(),
            target_id: "borrowed-target".into(),
            browser_id: browser.id.clone(),
            session_id: "task".into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        };
        assert_eq!(
            runtime.close_tab(&browser, &tab).unwrap_err(),
            "browser_attachment_borrowed_tab_close_requires_app_authority"
        );
        assert_eq!(
            runtime
                .reconcile_profile_browser(&profile, Some(&browser), &[tab], &[])
                .unwrap_err(),
            "browser_attachment_recovery_requires_app_authority"
        );
        assert_eq!(
            runtime.launch_browser(&profile, None).unwrap_err(),
            "browser_attachment_launch_requires_app_authority"
        );
    }

    #[cfg(target_os = "linux")]
    fn endpoint_observation(
        command_line: Option<Vec<String>>,
    ) -> crate::process_identity::ObservedProcessIdentity {
        crate::process_identity::ObservedProcessIdentity {
            pid: 42,
            start_token: Some("linux:boot:123".into()),
            executable_path: Some("/opt/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line,
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn existing_profile_endpoint_uses_same_observed_process_explicit_port_when_file_absent() {
        let root = std::env::temp_dir().join(format!("ab-endpoint-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let observed = endpoint_observation(Some(vec![format!(
            "/opt/chrome --user-data-dir={} --remote-debugging-port=45015",
            root.display()
        )]));
        let result = current_profile_debug_endpoint(&root, &observed);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(result, Ok("http://127.0.0.1:45015".into()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn existing_profile_endpoint_preserves_port_file_and_rejects_present_bad_metadata() {
        let root = std::env::temp_dir().join(format!("ab-endpoint-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = root.join("DevToolsActivePort");
        let observed = endpoint_observation(Some(vec![
            "/opt/chrome --remote-debugging-port=45015".into(),
        ]));
        for (contents, endpoint) in [
            (
                "43123\n/devtools/browser/kept\n",
                Some("ws://127.0.0.1:43123/devtools/browser/kept"),
            ),
            ("not-a-port\n/devtools/browser/kept\n", None),
            ("0\n/devtools/browser/kept\n", None),
            ("43123\n/devtools/page/wrong\n", None),
            ("43123\n", None),
        ] {
            fs::write(&path, contents).unwrap();
            let result = current_profile_debug_endpoint(&root, &observed);
            assert_eq!(result.ok().as_deref(), endpoint, "{contents:?}");
            assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        }
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(root.join("missing-port-file"), &path).unwrap();
        assert!(current_profile_debug_endpoint(&root, &observed).is_err());
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(current_profile_debug_endpoint(&root, &observed).is_err());
        assert!(path.is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn existing_profile_endpoint_rejects_unavailable_zero_or_invalid_process_port() {
        let root = std::env::temp_dir().join(format!("ab-endpoint-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        for arguments in [
            None,
            Some(vec![]),
            Some(vec!["/opt/chrome".into()]),
            Some(vec!["--remote-debugging-port=0".into()]),
            Some(vec!["--remote-debugging-port=".into()]),
            Some(vec!["--remote-debugging-port=invalid".into()]),
            Some(vec!["--remote-debugging-port=65536".into()]),
        ] {
            assert!(
                current_profile_debug_endpoint(&root, &endpoint_observation(arguments)).is_err()
            );
        }
        let conventional = endpoint_observation(Some(vec![
            "/opt/chrome".into(),
            "--remote-debugging-port".into(),
            "45015".into(),
        ]));
        assert_eq!(
            current_profile_debug_endpoint(&root, &conventional),
            Ok("http://127.0.0.1:45015".into())
        );
        assert!(!root.join("DevToolsActivePort").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn app_attachment_endpoint_pid_mismatch_stops_before_target_or_page_commands() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let methods = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let recorded = methods.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            while let Some(Ok(message)) = socket.next().await {
                if !message.is_text() {
                    continue;
                }
                let command: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                recorded
                    .lock()
                    .unwrap()
                    .push(command["method"].as_str().unwrap().into());
                socket.send(Message::Text(serde_json::json!({"id":command["id"],"result":{"processInfo":[{"type":"browser","id":u64::from(std::process::id())+1}]}}).to_string())).await.unwrap();
            }
        });
        let before = match crate::process_identity::observe_process(std::process::id()) {
            crate::process_identity::ProcessObservation::Observed(before) => before,
            _ => panic!("test process must be observable"),
        };
        let result = attach_verified_browser_endpoint(&endpoint, std::process::id(), before).await;
        assert!(
            matches!(result,Err(ref error) if error=="browser_session_profile_endpoint_identity_conflict")
        );
        assert_eq!(*methods.lock().unwrap(), vec!["SystemInfo.getProcessInfo"]);
        server.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn owned_tab_retirement_reconciles_only_verified_fresh_absence() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;
        for scenario in ["absent", "cached_absent", "malformed", "wrong_pid"] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("ws://{}", listener.local_addr().unwrap());
            let methods = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let recorded = methods.clone();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                let mut inventories = 0;
                let mut process_observations = 0;
                while let Some(Ok(message)) = socket.next().await {
                    if !message.is_text() {
                        continue;
                    }
                    let command: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                    let method = command["method"].as_str().unwrap();
                    recorded.lock().unwrap().push(method.into());
                    let result = match method {
                        "SystemInfo.getProcessInfo" => {
                            process_observations += 1;
                            serde_json::json!({"processInfo":[{
                                "type":"browser", "id":u64::from(std::process::id())
                                    + u64::from(scenario == "wrong_pid" && process_observations > 1)
                            }]})
                        }
                        "Page.enable"
                        | "Runtime.enable"
                        | "Network.enable"
                        | "Runtime.runIfWaitingForDebugger"
                        | "Target.setAutoAttach" => serde_json::json!({}),
                        "Target.attachToTarget" => {
                            assert!(matches!(
                                command["params"]["targetId"].as_str(),
                                Some("owned-target" | "replacement-owned")
                            ));
                            serde_json::json!({"sessionId":"owned-session"})
                        }
                        "Target.createTarget" => {
                            assert_eq!(scenario, "cached_absent");
                            serde_json::json!({"targetId":"replacement-owned"})
                        }
                        "Target.activateTarget" => {
                            assert_eq!(command["params"]["targetId"], "replacement-owned");
                            serde_json::json!({})
                        }
                        "Target.setDiscoverTargets" | "Browser.getVersion" => serde_json::json!({}),
                        "Target.getTargets" => {
                            inventories += 1;
                            if inventories == 1 && scenario == "cached_absent" {
                                serde_json::json!({"targetInfos":[{
                                    "targetId":"owned-target", "type":"page", "title":"", "url":"about:blank", "attached":false
                                }]})
                            } else if inventories == 1 {
                                serde_json::json!({"targetInfos":[]})
                            } else if scenario == "malformed" {
                                serde_json::json!({"unverified":true})
                            } else {
                                serde_json::json!({"targetInfos":[{
                                    "targetId":"protected-peer", "type":"page", "title":"", "url":"about:blank", "attached":false
                                }]})
                            }
                        }
                        other => panic!(
                            "retirement must not close, activate or manufacture a peer: {other}"
                        ),
                    };
                    socket
                        .send(Message::Text(
                            serde_json::json!({"id":command["id"],"result":result}).to_string(),
                        ))
                        .await
                        .unwrap();
                }
            });
            let browser = ManagedBrowserInstance {
                id: "borrowed-browser".into(),
                profile_id: "borrowed-profile".into(),
                pid: std::process::id(),
                cdp_endpoint: endpoint,
                desktop: None,
                active_session_ids: vec!["task".into()],
            };
            let tab = ManagedBrowserTab {
                id: "owned-tab".into(),
                target_id: "owned-target".into(),
                browser_id: browser.id.clone(),
                session_id: "task".into(),
                created_at_ms: 100,
                last_activity_at_ms: 100,
            };
            let mut runtime =
                BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default()).unwrap();
            let result = runtime.close_tab(&browser, &tab);
            assert_eq!(
                result.is_ok(),
                scenario.ends_with("absent"),
                "{scenario}: {result:?}"
            );
            let methods = methods.lock().unwrap().clone();
            assert!(!methods.iter().any(|m| matches!(
                m.as_str(),
                "Target.closeTarget" | "Target.activateTarget" | "Target.createTarget"
            )));
            if scenario == "absent" {
                assert!(
                    methods
                        .iter()
                        .filter(|m| m.as_str() == "Target.getTargets")
                        .count()
                        >= 2,
                    "cached absence cannot replace fresh physical inventory"
                );
            }
            if scenario == "cached_absent" {
                let acquired = runtime.acquire_initial_tab(&browser, &[]).unwrap();
                assert_eq!(
                    acquired.target_id, "replacement-owned",
                    "a positively absent cached task target cannot become another task's bootstrap"
                );
            }
            drop(runtime);
            server.abort();
        }
    }

    #[derive(Default)]
    struct FakeRuntime;

    impl BrowserRuntimeDriver for FakeRuntime {
        fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
            Ok(true)
        }

        fn launch_browser(
            &mut self,
            _profile: &BrowserProfileCatalogEntry,
            _desktop: Option<&RemoteViewFixedDesktop>,
        ) -> Result<BrowserLaunch, String> {
            Err("unused".to_string())
        }

        fn close_browser(&mut self, _browser: &ManagedBrowserInstance) -> Result<(), String> {
            Ok(())
        }

        fn acquire_initial_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _attributed_target_ids: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            Err("unused".to_string())
        }

        fn create_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            Err("unused".to_string())
        }

        fn close_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
        ) -> Result<(), String> {
            Ok(())
        }

        fn navigate(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
            _url: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        fn focus_browser(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: Option<&ManagedBrowserTab>,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "agent-browser-disposable-profile-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            )))
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn native_idle_retirement_queries_current_controller_and_operation_authority() {
        use super::super::service_store::{
            default_service_state_path, JsonServiceStateStore, ServiceStateStore,
        };
        use agent_browser_service_model::{
            JobState, JobTarget, ServiceJob, ServiceState, ViewerLease,
        };
        assert_ne!(
            std::env::var("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME")
                .ok()
                .as_deref(),
            Some("1")
        );
        let path = default_service_state_path().unwrap();
        struct RestoreSnapshot {
            path: std::path::PathBuf,
            prior: Option<Vec<u8>>,
        }
        impl Drop for RestoreSnapshot {
            fn drop(&mut self) {
                if let Some(prior) = &self.prior {
                    let _ = fs::write(&self.path, prior);
                } else {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
        let _restore = RestoreSnapshot {
            prior: fs::read(&path).ok(),
            path: path.clone(),
        };
        let store = JsonServiceStateStore::new(&path);
        let browser = ManagedBrowserInstance {
            id: "pressure-runtime-browser".into(),
            profile_id: "pressure-profile".into(),
            pid: 42,
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: None,
            active_session_ids: vec![],
        };
        let mut effects = BrowserSessionEffectAdapter::new(
            BrowserManagerRuntime::start(Default::default()).unwrap(),
        );
        let mut state = ServiceState::default();
        state.viewer_leases.insert(
            "controller".into(),
            ViewerLease {
                id: "controller".into(),
                browser_id: Some(browser.id.clone()),
                viewer_role: "controller".into(),
                state: "active".into(),
                ..Default::default()
            },
        );
        store.save(&state).unwrap();
        assert!(
            !effects.permits_idle_browser_close(&browser).unwrap(),
            "current controller protects exact browser"
        );
        state.viewer_leases.get_mut("controller").unwrap().state = "released".into();
        state.jobs.insert(
            "operation".into(),
            ServiceJob {
                id: "operation".into(),
                target: JobTarget::Browser(browser.id.clone()),
                state: JobState::Running,
                ..Default::default()
            },
        );
        store.save(&state).unwrap();
        assert!(
            !effects.permits_idle_browser_close(&browser).unwrap(),
            "current operation protects exact browser"
        );
        state.jobs.get_mut("operation").unwrap().state = JobState::Succeeded;
        store.save(&state).unwrap();
        assert!(
            effects.permits_idle_browser_close(&browser).unwrap(),
            "released work allows otherwise eligible retirement"
        );
        state.jobs.get_mut("operation").unwrap().target =
            JobTarget::Profile(browser.profile_id.clone());
        state.jobs.get_mut("operation").unwrap().state = JobState::Running;
        store.save(&state).unwrap();
        assert!(
            !effects.permits_idle_browser_close(&browser).unwrap(),
            "profile operation protects its browser"
        );
        state.jobs.get_mut("operation").unwrap().state = JobState::Succeeded;
        state.browsers.insert(
            browser.id.clone(),
            agent_browser_service_model::BrowserProcess {
                id: browser.id.clone(),
                pid: Some(99),
                profile_id: Some(browser.profile_id.clone()),
                cdp_endpoint: Some(browser.cdp_endpoint.clone()),
                ..Default::default()
            },
        );
        store.save(&state).unwrap();
        assert_eq!(
            effects.permits_idle_browser_close(&browser).unwrap_err(),
            "browser_session_idle_activity_identity_conflict"
        );
        fs::write(&path, b"invalid synthetic state").unwrap();
        assert!(
            effects.permits_idle_browser_close(&browser).is_err(),
            "unreadable current authority fails closed"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_disposable_group_guard_checks_profile_jobs_and_releases_exclusion() {
        // This guard fixture proves current jobs/lease exclusion, not host
        // process discovery. Supply a complete synthetic unrelated process table
        // rather than implicitly reading peer namespaces or starting services.
        let process_census = super::super::service_resources::CleanupCensusFixture::new(vec![
            super::super::service_resources::ProcessSample {
                pid: 42,
                command: vec!["synthetic-unrelated-worker".into()],
                ..Default::default()
            },
        ]);
        use super::super::service_store::{
            default_service_state_path, JsonServiceStateStore, ServiceStateStore,
        };
        use agent_browser_service_model::{JobState, JobTarget, ServiceJob, ServiceState};
        assert_ne!(
            std::env::var("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME")
                .ok()
                .as_deref(),
            Some("1")
        );
        let path = default_service_state_path().unwrap();
        struct Restore {
            path: PathBuf,
            prior: Option<Vec<u8>>,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                if let Some(prior) = &self.prior {
                    let _ = fs::write(&self.path, prior);
                } else {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
        let _restore = Restore {
            prior: fs::read(&path).ok(),
            path: path.clone(),
        };
        let root = TempDirectory::new();
        fs::create_dir_all(root.0.join("orphan")).unwrap();
        let allocation = ManagedDisposableProfile {
            profile: BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "group-orphan".into(),
                name: "synthetic".into(),
                user_data_dir: root.0.join("orphan").to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Disposable,
            },
            policy_id: "default".into(),
            session_name: "alice".into(),
            user_data_root: root.0.to_string_lossy().into_owned(),
            created_at_ms: 1,
            cleanup_delay_ms: 0,
            cleanup_eligible_at_ms: Some(2),
        };
        struct RestoreRuntime(agent_browser_service_model::BrowserSessionState);
        impl Drop for RestoreRuntime {
            fn drop(&mut self) {
                let result = (|| {
                    let mut store = super::super::browser_session_store::BrowserSessionSqliteStore::default_sqlite()?;
                    let current = store.load_session_state()?;
                    store.compare_and_save_session_state(&current, &self.0)
                })();
                if !std::thread::panicking() {
                    assert!(result.is_ok());
                }
            }
        }
        let mut runtime =
            super::super::browser_session_store::BrowserSessionSqliteStore::default_sqlite()
                .unwrap();
        let prior = runtime.load_session_state().unwrap();
        let _restore_runtime = RestoreRuntime(prior.clone());
        let mut current = prior.clone();
        current
            .disposable_profiles
            .insert(allocation.profile.id.clone(), allocation.clone());
        runtime
            .compare_and_save_session_state(&prior, &current)
            .unwrap();
        let store = JsonServiceStateStore::new(&path);
        let mut state = ServiceState::default();
        state.jobs.insert(
            "profile-work".into(),
            ServiceJob {
                id: "profile-work".into(),
                target: JobTarget::Profile(allocation.profile.id.clone()),
                state: JobState::Running,
                ..Default::default()
            },
        );
        store.save(&state).unwrap();
        let mut effects = BrowserSessionEffectAdapter::new(
            BrowserManagerRuntime::start(Default::default()).unwrap(),
        );
        let allocations = [allocation];
        assert!(effects
            .acquire_disposable_retirement_guard(&allocations, &[])
            .unwrap()
            .is_none());
        assert!(Path::new(&allocations[0].profile.user_data_dir).is_dir());
        state.jobs.clear();
        let unclaimed = state.clone();
        let now = chrono::Utc::now();
        state
            .acquire_lease_claim(agent_browser_lease_authority::AcquireLeaseClaimRequest {
                resource: agent_browser_lease_authority::LeaseResourceKey::profile(
                    &allocations[0].profile.id,
                ),
                parent_claim_id: None,
                principal_id: "synthetic-profile-user".into(),
                capability_id: "synthetic-profile-capability".into(),
                capability_revision: 1,
                mode: agent_browser_lease_authority::LeaseClaimMode::Ephemeral,
                expected_claim_revision: 0,
                idempotency_key: "synthetic-profile-claim".into(),
                now: now.to_rfc3339(),
                expires_at: (now + chrono::Duration::minutes(5)).to_rfc3339(),
                transition_deadline: None,
                recovery_controller_id: None,
                boot_epoch: Some("synthetic-boot".into()),
                owner_generation: None,
            })
            .unwrap();
        store.save(&state).unwrap();
        assert!(
            effects
                .acquire_disposable_retirement_guard(&allocations, &[])
                .unwrap()
                .is_none(),
            "current canonical profile claim protects an orphan allocation"
        );
        assert!(Path::new(&allocations[0].profile.user_data_dir).is_dir());
        // Restore the independent unclaimed fixture, not a simulated claim release.
        state = unclaimed;
        store.save(&state).unwrap();
        process_census.fail("process_observer_timeout");
        assert_eq!(
            effects
                .acquire_disposable_retirement_guard(&allocations, &[])
                .err()
                .unwrap(),
            "process_observer_timeout"
        );
        assert!(Path::new(&allocations[0].profile.user_data_dir).is_dir());
        process_census.restore();
        let guard = effects
            .acquire_disposable_retirement_guard(&allocations, &[])
            .unwrap()
            .unwrap();
        let child = |blocked: bool| {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "native::service_store::tests::idle_retirement_publication_helper",
                    "--test-threads=1",
                ])
                .env("AGENT_BROWSER_TEST_RETIREMENT_PUBLICATION_PATH", &path)
                .env_remove("AGENT_BROWSER_TEST_RETIREMENT_EXPECT_BLOCKED");
            if blocked {
                command.env("AGENT_BROWSER_TEST_RETIREMENT_EXPECT_BLOCKED", "1");
            }
            let result = command.output().unwrap();
            assert!(
                result.status.success(),
                "{} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        };
        child(true);
        effects.delete_disposable_profile(&allocations[0]).unwrap();
        assert!(!Path::new(&allocations[0].profile.user_data_dir).exists());
        drop(guard);
        child(false);
        fs::write(&path, b"invalid synthetic authority").unwrap();
        assert!(effects
            .acquire_disposable_retirement_guard(&allocations, &[])
            .is_err());
        // Failed admission releases the repository lock, permitting restoration.
        store.save(&state).unwrap();
        assert!(effects
            .acquire_disposable_retirement_guard(&allocations, &[])
            .unwrap()
            .is_some());
        let before = runtime.load_session_state().unwrap();
        let mut promoted = before.clone();
        promoted
            .disposable_profiles
            .remove(&allocations[0].profile.id);
        runtime
            .compare_and_save_session_state(&before, &promoted)
            .unwrap();
        assert!(
            effects
                .acquire_disposable_retirement_guard(&allocations, &[])
                .unwrap()
                .is_none(),
            "a stale allocation cannot authorize cleanup after promotion"
        );
    }

    #[test]
    fn runtime_original_retirement_does_not_adopt_a_reused_pid_or_missing_identity() {
        let pid = std::process::id();
        let current = crate::process_identity::capture_process_identity(pid, None, None).unwrap();
        let mut scope = agent_browser_service_model::BrowserIdleRetirementScope {
            browser: ManagedBrowserInstance {
                id: "synthetic-old-browser".into(),
                profile_id: "synthetic-profile".into(),
                pid,
                cdp_endpoint: "ws://127.0.0.1:1/unreachable".into(),
                desktop: None,
                active_session_ids: vec![],
            },
            process_identity: Some(current.clone()),
            tabs: vec![],
        };
        scope
            .process_identity
            .as_mut()
            .unwrap()
            .start_token
            .push_str(":prior-instance");
        let mut driver = BrowserManagerRuntime::start(Default::default()).unwrap();
        assert!(driver.observe_idle_retirement_effect(&scope, None).unwrap());
        driver.apply_idle_retirement_effect(&scope, None).unwrap();
        assert_eq!(
            crate::process_identity::capture_process_identity(pid, None, None),
            Some(current.clone())
        );
        scope.process_identity = None;
        assert_eq!(
            driver
                .apply_idle_retirement_effect(&scope, None)
                .unwrap_err(),
            "browser_session_idle_original_identity_missing"
        );
        assert_eq!(
            driver
                .observe_idle_retirement_effect(&scope, None)
                .unwrap_err(),
            "browser_session_idle_original_identity_missing"
        );
        scope.process_identity = Some(current);
        assert!(original_retirement_process(
            &scope,
            crate::process_identity::ProcessObservation::Failed {
                reason: "synthetic unavailable census".into(),
            }
        )
        .is_err());
        scope.browser.pid = u32::MAX;
        assert!(original_retirement_process(
            &scope,
            crate::process_identity::ProcessObservation::Missing
        )
        .is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn runtime_original_retirement_reads_exact_targets_and_preserves_the_peer_after_restart() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "original-retirement-fixture".into(),
            name: "Original retirement fixture".into(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let config = BrowserManagerRuntimeConfig {
            headless: true,
            ..Default::default()
        };
        let mut first = BrowserManagerRuntime::start(config.clone()).unwrap();
        let launch = first.launch_browser(&profile, None).unwrap();
        let identity = launch
            .process_identity
            .clone()
            .expect("capture original live child identity");
        struct CleanupOriginal(crate::process_identity::VerifiedProcessTermination);
        impl Drop for CleanupOriginal {
            fn drop(&mut self) {
                use crate::process_identity::VerifiedProcessSignal;
                if self.0.is_running().unwrap_or(false) {
                    let _ = self.0.signal(VerifiedProcessSignal::Terminate);
                    let until = Instant::now() + Duration::from_secs(2);
                    while self.0.is_running().unwrap_or(false) && Instant::now() < until {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    if self.0.is_running().unwrap_or(false) {
                        let _ = self.0.signal(VerifiedProcessSignal::Kill);
                    }
                }
            }
        }
        let _cleanup = CleanupOriginal(
            crate::process_identity::VerifiedProcessTermination::open(&identity)
                .unwrap()
                .unwrap(),
        );
        let browser = ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: profile.id,
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: None,
            active_session_ids: vec![],
        };
        let original = first.acquire_initial_tab(&browser, &[]).unwrap();
        let peer = first.create_tab(&browser).unwrap();
        let tab = |item: BrowserTabAcquisition, session: &str| ManagedBrowserTab {
            id: item.tab_id,
            target_id: item.target_id,
            browser_id: browser.id.clone(),
            session_id: session.into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        };
        let original = tab(original, "original");
        let peer = tab(peer, "peer");
        first
            .navigate(
                &browser,
                &original,
                "data:text/html,<title>original</title>",
            )
            .unwrap();
        first
            .navigate(&browser, &peer, "data:text/html,<title>peer</title>")
            .unwrap();
        let scope = agent_browser_service_model::BrowserIdleRetirementScope {
            browser,
            process_identity: Some(identity),
            tabs: vec![original.clone(), peer.clone()],
        };
        drop(first);
        let mut restarted = BrowserManagerRuntime::start(config).unwrap();
        assert!(!restarted
            .observe_idle_retirement_effect(&scope, Some(&original))
            .unwrap());
        restarted
            .apply_idle_retirement_effect(&scope, Some(&original))
            .unwrap();
        assert!(restarted
            .observe_idle_retirement_effect(&scope, Some(&original))
            .unwrap());
        assert!(!restarted
            .observe_idle_retirement_effect(&scope, Some(&peer))
            .unwrap());
        assert!(restarted.browser_is_live(&scope.browser).unwrap());
        let title = restarted
            .execute_command(
                &scope.browser,
                &peer,
                &peer.session_id,
                "peer",
                &serde_json::json!({"id":"preserved-peer", "action":"title"}),
            )
            .unwrap();
        assert_eq!(title["success"], true);
        assert_eq!(title["data"]["title"], "peer");
        restarted
            .apply_idle_retirement_effect(&scope, None)
            .unwrap();
        assert!(restarted
            .observe_idle_retirement_effect(&scope, None)
            .unwrap());
        assert!(!recorded_browser_process_exists(&scope.browser));
    }

    #[test]
    fn unattached_browser_close_requires_positive_process_absence() {
        use crate::process_identity::ProcessObservation;
        let browser = ManagedBrowserInstance {
            id: "browser:unknown-close".into(),
            profile_id: "work".into(),
            pid: u32::MAX,
            cdp_endpoint: "ws://127.0.0.1:1/unreachable".into(),
            desktop: None,
            active_session_ids: vec![],
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        assert_eq!(
            runtime
                .block_on(close_unattached_browser(
                    &browser,
                    ProcessObservation::Failed {
                        reason: "synthetic census unavailable".into()
                    }
                ))
                .unwrap_err(),
            "browser_session_process_observation_unavailable"
        );
        runtime
            .block_on(close_unattached_browser(
                &browser,
                ProcessObservation::Missing,
            ))
            .unwrap();
    }

    #[test]
    fn browser_close_wait_requires_positive_process_absence() {
        use crate::process_identity::{ObservedProcessIdentity, ProcessObservation};
        let observed = ProcessObservation::Observed(ObservedProcessIdentity {
            pid: 42,
            start_token: Some("original-process".into()),
            executable_path: None,
            browser_family: None,
            command_line: None,
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut unavailable = [
            observed.clone(),
            ProcessObservation::Failed {
                reason: "synthetic process census failure".into(),
            },
        ]
        .into_iter();
        assert_eq!(
            runtime
                .block_on(wait_for_recorded_process_exit(
                    || unavailable.next().expect("stop after failed observation"),
                    None,
                ))
                .unwrap_err(),
            "browser_session_process_observation_unavailable"
        );
        let mut exited = [observed, ProcessObservation::Missing].into_iter();
        runtime
            .block_on(wait_for_recorded_process_exit(
                || exited.next().expect("stop after positive absence"),
                Some("lost close reply".into()),
            ))
            .unwrap();
    }

    #[test]
    fn recorded_browser_liveness_uses_the_persisted_pid() {
        let mut browser = ManagedBrowserInstance {
            id: "browser:work:1".to_string(),
            profile_id: "work".to_string(),
            pid: std::process::id(),
            cdp_endpoint: "ws://127.0.0.1:1/devtools/browser/test".to_string(),
            desktop: None,
            active_session_ids: Vec::new(),
        };
        assert!(recorded_browser_process_exists(&browser));

        browser.pid = u32::MAX;
        assert!(!recorded_browser_process_exists(&browser));
    }

    #[test]
    fn private_external_runtime_rejects_stale_viewing_generation_before_process_ingress() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let assignment = serde_json::from_value(fixture["assignment"].clone()).unwrap();
        let observation: RemoteViewAssignmentObservation =
            serde_json::from_value(fixture["assignmentObservation"].clone()).unwrap();
        let intent = BrowserLaunchIntent {
            intent_id: "33333333-3333-3333-3333-333333333333".to_string(),
            profile_id: "profile-a".to_string(),
            assignment,
        };
        let environment = RemoteViewPrivateLaunchEnvironment::from_response(
            fixture["launchEnvironment"].clone(),
            &observation,
            &intent.assignment,
        )
        .unwrap();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: intent.profile_id.clone(),
            name: "Synthetic".to_string(),
            user_data_dir: "/synthetic/profile-a".to_string(),
            kind: BrowserProfileKind::Named,
        };
        let mut stale = observation;
        stale.target.viewing_generation += 1;
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default())
            .expect("runtime starts without launching a browser");
        assert_eq!(
            RemoteViewBrowserProcessEffects::launch(
                &mut runtime,
                &profile,
                &intent,
                environment,
                &stale,
            ),
            Err(RemoteViewBrowserProcessError::Rejected)
        );
    }

    #[test]
    fn remote_view_desktop_launch_fails_before_local_display_inference() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "remote-view-fixture".to_string(),
            name: "Remote View fixture".to_string(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let desktop = RemoteViewFixedDesktop {
            desktop_id: "11111111-1111-1111-1111-111111111111".to_string(),
            friendly_route_label: "desktop-a".to_string(),
            generation: 1,
        };
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default())
            .expect("runtime starts without launching a browser");

        assert_eq!(
            runtime.launch_browser(&profile, Some(&desktop)),
            Err(
                "remote_view_desktop_runtime_context_unavailable:11111111-1111-1111-1111-111111111111:1"
                    .to_string()
            )
        );
    }

    #[test]
    fn remote_view_display_requires_exact_desktop_generation() {
        let desktop = RemoteViewFixedDesktop {
            desktop_id: "11111111-1111-1111-1111-111111111111".to_string(),
            friendly_route_label: "desktop-a".to_string(),
            generation: 2,
        };
        let contexts = vec![RemoteViewDesktopRuntimeContext {
            desktop_id: desktop.desktop_id.clone(),
            generation: 1,
            display_name: ":21".to_string(),
        }];

        assert_eq!(
            resolve_remote_view_display(&desktop, &contexts),
            Err(
                "remote_view_desktop_runtime_context_stale:11111111-1111-1111-1111-111111111111:2"
                    .to_string()
            )
        );
    }

    #[test]
    fn remote_view_display_uses_authoritative_runtime_context() {
        let desktop = RemoteViewFixedDesktop {
            desktop_id: "11111111-1111-1111-1111-111111111111".to_string(),
            friendly_route_label: "route-label-is-not-a-display".to_string(),
            generation: 3,
        };
        let contexts = vec![RemoteViewDesktopRuntimeContext {
            desktop_id: desktop.desktop_id.clone(),
            generation: desktop.generation,
            display_name: ":47".to_string(),
        }];

        assert_eq!(
            resolve_remote_view_display(&desktop, &contexts),
            Ok(":47".to_string())
        );
    }

    // This seam includes the real process adapter and launch coordinator. Neither
    // provider input acquisition nor durable custody is permitted on preflight failure.
    struct NoLaunchEffects;
    impl agent_browser_service_model::RemoteViewApplicationTransport for NoLaunchEffects {
        fn request(
            &mut self,
            _: &agent_browser_service_model::RemoteViewApplicationEnvelope,
        ) -> Result<Value, agent_browser_service_model::RemoteViewApplicationTransportError>
        {
            panic!("preflight must precede provider inputs")
        }
    }
    impl agent_browser_service_model::BrowserLaunchCustodyStore for NoLaunchEffects {
        fn admit_launch_intent(
            &mut self,
            _: &BrowserLaunchIntent,
            _: &agent_browser_service_model::BrowserSessionState,
        ) -> Result<
            agent_browser_service_model::LaunchCustodyAdmission,
            agent_browser_service_model::LaunchCustodyStoreError,
        > {
            panic!("preflight must precede durable custody")
        }
        fn observe_launch_intent(
            &mut self,
            _: &BrowserLaunchIntent,
            _: &BrowserLaunch,
        ) -> Result<(), agent_browser_service_model::LaunchCustodyStoreError> {
            panic!("no launch")
        }
        fn publish_launch_intent(
            &mut self,
            _: &BrowserLaunchIntent,
            _: &agent_browser_service_model::BrowserSessionState,
            _: &agent_browser_service_model::BrowserSessionState,
        ) -> Result<(), agent_browser_service_model::LaunchCustodyStoreError> {
            panic!("no publication")
        }
    }

    #[test]
    fn native_launch_preflight_rejects_relative_profile_before_custody() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let intent = BrowserLaunchIntent {
            intent_id: "33333333-3333-3333-3333-333333333333".into(),
            profile_id: "Default".into(),
            assignment: serde_json::from_value(fixture["assignment"].clone()).unwrap(),
        };
        let directory = TempDirectory::new();
        let cases = [
            (
                "Default".to_string(),
                None,
                "browser_session_profile_path_not_absolute",
            ),
            (
                directory.0.to_string_lossy().into_owned(),
                Some(0),
                "browser_launch_resource_pressure:browser_process_capacity_exhausted",
            ),
        ];
        for (path, limit, reason) in cases {
            let profile = BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "Default".into(),
                name: "Default".into(),
                user_data_dir: path,
                kind: BrowserProfileKind::Named,
            };
            let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
                maximum_browser_processes: limit,
                ..BrowserManagerRuntimeConfig::default()
            })
            .unwrap();
            let mut process = BrowserSessionEffectAdapter::new(runtime);
            let mut adapter = agent_browser_service_model::RemoteViewApplicationAdapter::new(
                "agent-browser".into(),
                NoLaunchEffects,
            )
            .unwrap();
            let error = agent_browser_service_model::launch_remote_view_browser(
                &mut adapter,
                &mut NoLaunchEffects,
                &mut process,
                &intent,
                &agent_browser_service_model::BrowserSessionState::default(),
                &profile,
            )
            .unwrap_err();
            assert_eq!(
                error,
                agent_browser_service_model::RemoteViewBrowserLaunchError::Preflight(reason.into())
            );
        }
    }

    #[test]
    fn resource_pressure_rejects_before_local_browser_launch() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "resource-pressure-fixture".to_string(),
            name: "Resource pressure fixture".to_string(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
            maximum_browser_processes: Some(0),
            ..BrowserManagerRuntimeConfig::default()
        })
        .expect("runtime starts without launching a browser");

        let error = runtime.launch_browser(&profile, None).unwrap_err();
        assert!(error.starts_with("browser_launch_resource_pressure:"));
        assert!(error.contains("browser_process_capacity_exhausted"));
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn existing_profile_browser_is_reused_without_saved_inventory() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "reuse-fixture".into(),
            name: "Reuse fixture".into(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let config = BrowserManagerRuntimeConfig {
            headless: true,
            ..Default::default()
        };
        let mut owner = BrowserManagerRuntime::start(config.clone()).unwrap();
        let original = owner.launch_browser(&profile, None).unwrap();
        let browser = ManagedBrowserInstance {
            id: original.browser_id.clone(),
            profile_id: profile.id.clone(),
            pid: original.pid,
            cdp_endpoint: original.cdp_endpoint.clone(),
            desktop: None,
            active_session_ids: vec![],
        };
        let mut reuse_config = config;
        reuse_config.executable_path = Some(
            directory
                .0
                .join("next-launch-only")
                .to_string_lossy()
                .into_owned(),
        );
        let mut restarted = BrowserManagerRuntime::start(reuse_config).unwrap();
        let reused = restarted.launch_browser(&profile, None);
        #[cfg(target_os = "linux")]
        let parent_reuse = {
            let lock =
                fs::read_link(Path::new(&profile.user_data_dir).join("SingletonLock")).unwrap();
            std::os::unix::fs::symlink(lock, directory.0.join("SingletonLock")).unwrap();
            let mut parent_profile = profile.clone();
            parent_profile.user_data_dir = directory.0.to_string_lossy().into_owned();
            restarted.reconcile_profile_browser(&parent_profile, None, &[], &[])
        };
        // Always release only the synthetic owner's exact process on failure.
        owner.close_browser(&browser).unwrap();
        let reused = reused.expect("opening the exact live profile must reuse its browser");
        assert_eq!(reused.pid, original.pid);
        assert_eq!(reused.cdp_endpoint, original.cdp_endpoint);
        #[cfg(target_os = "linux")]
        assert!(
            parent_reuse.unwrap().is_none(),
            "a parent directory is not the same physical profile"
        );
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn shared_profile_acquisition_and_close_preserve_peer_work() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "peer-fixture".into(),
            name: "Peer fixture".into(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let config = BrowserManagerRuntimeConfig {
            headless: true,
            ..Default::default()
        };
        let mut owner = BrowserManagerRuntime::start(config.clone()).unwrap();
        let original = owner.launch_browser(&profile, None).unwrap();
        let peer = ManagedBrowserInstance {
            id: original.browser_id,
            profile_id: profile.id.clone(),
            pid: original.pid,
            cdp_endpoint: original.cdp_endpoint,
            desktop: None,
            active_session_ids: vec![],
        };
        let acquired = owner.acquire_initial_tab(&peer, &[]).unwrap();
        let peer_tab = ManagedBrowserTab {
            id: acquired.tab_id,
            target_id: acquired.target_id,
            browser_id: peer.id.clone(),
            session_id: "bob".into(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        };
        owner
            .navigate(
                &peer,
                &peer_tab,
                "data:text/html,<title>Bob retained work</title>",
            )
            .unwrap();
        let mut requester = BrowserManagerRuntime::start(config).unwrap();
        let result = (|| -> Result<(), String> {
            let launch = requester.launch_browser(&profile, None)?;
            let alice = ManagedBrowserInstance {
                id: launch.browser_id,
                profile_id: profile.id.clone(),
                pid: launch.pid,
                cdp_endpoint: launch.cdp_endpoint,
                desktop: None,
                active_session_ids: vec![],
            };
            let acquired = requester.acquire_initial_tab(&alice, &[])?;
            if acquired.target_id == peer_tab.target_id {
                return Err("Alice acquisition adopted Bob's existing tab".into());
            }
            requester.close_browser(&alice)?;
            if !recorded_browser_process_exists(&peer) {
                return Err("closing Alice closed Bob's browser".into());
            }
            let title = owner.execute_command(
                &peer,
                &peer_tab,
                "bob",
                "bob",
                &serde_json::json!({"id":"peer-title", "action":"title"}),
            )?;
            if title["data"]["title"] != "Bob retained work" {
                return Err("Bob's original work changed".into());
            }
            Ok(())
        })();
        owner.close_browser(&peer).unwrap();
        result.expect("shared profile task must preserve its peer");
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn shared_profile_open_replay_after_lost_publication_reuses_its_owned_tab() {
        use agent_browser_service_model::{
            BrowserProfileCatalog, BrowserSessionManager, BrowserSessionManagerConfig,
            BrowserSessionState, OpenBrowserSession,
        };
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "publication-replay".into(),
            name: "Publication replay".into(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let config = BrowserManagerRuntimeConfig {
            headless: true,
            ..Default::default()
        };
        let mut owner = BrowserManagerRuntime::start(config.clone()).unwrap();
        let launch = owner.launch_browser(&profile, None).unwrap();
        let physical = ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: profile.id.clone(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: None,
            active_session_ids: vec![],
        };
        let catalog = BrowserProfileCatalog {
            profiles: std::collections::BTreeMap::from([(profile.id.clone(), profile.clone())]),
            ..Default::default()
        };
        let policy = BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: vec![],
        };
        let result = (|| -> Result<(), String> {
            let mut uncommitted = BrowserSessionState::default();
            let mut first =
                BrowserSessionEffectAdapter::new(BrowserManagerRuntime::start(config.clone())?);
            let target = {
                let mut manager = BrowserSessionManager::new(
                    &mut uncommitted,
                    &catalog,
                    &mut first,
                    policy.clone(),
                );
                let acquired = manager.open(OpenBrowserSession::exact_profile(
                    "alice",
                    &profile.id,
                    1_000,
                ))?;
                manager
                    .tab_for_navigation(&acquired.session_id, 1_000)?
                    .target_id
            };
            // The tab exists, but publication failed and a fresh process loads the old baseline.
            drop(first);
            let mut committed = BrowserSessionState::default();
            let mut restarted =
                BrowserSessionEffectAdapter::new(BrowserManagerRuntime::start(config.clone())?);
            let (resumed, replayed_target) = {
                let mut manager =
                    BrowserSessionManager::new(&mut committed, &catalog, &mut restarted, policy);
                let resumed = manager.open(OpenBrowserSession::exact_profile(
                    "alice",
                    &profile.id,
                    2_000,
                ))?;
                let target = manager
                    .tab_for_navigation(&resumed.session_id, 2_000)?
                    .target_id;
                (resumed, target)
            };
            if replayed_target != target {
                return Err("publication retry created another Alice tab".into());
            }
            if committed.browsers[&resumed.browser_id].pid != physical.pid {
                return Err("publication retry replaced the shared browser".into());
            }
            Ok(())
        })();
        owner.close_browser(&physical).unwrap();
        result.expect("initial acquisition must survive publication loss and requester restart");
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn free_profile_recovery_ignores_unrelated_browser_profile_metadata() {
        let directory = TempDirectory::new();
        let occupied = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "unrelated-browser".into(),
            name: "Unrelated browser".into(),
            user_data_dir: directory.0.join("occupied").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let mut owner = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
            headless: true,
            ..Default::default()
        })
        .unwrap();
        let launch = owner.launch_browser(&occupied, None).unwrap();
        let live = ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: occupied.id.clone(),
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: None,
            active_session_ids: vec![],
        };
        // The unrelated process remains live while its old argv path becomes stale.
        fs::rename(&occupied.user_data_dir, directory.0.join("moved-occupied")).unwrap();
        let free = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "free-profile".into(),
            name: "Free profile".into(),
            user_data_dir: directory.0.join("free").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        fs::create_dir(&free.user_data_dir).unwrap();
        let retained = ManagedBrowserInstance {
            id: "retained-free".into(),
            profile_id: free.id.clone(),
            pid: live.pid,
            cdp_endpoint: "ws://127.0.0.1:1/devtools/browser/stale".into(),
            desktop: None,
            active_session_ids: vec![],
        };
        let result = owner.prove_recovery_absence(&retained, &free);
        let peer_survived = recorded_browser_process_exists(&live);
        owner.close_browser(&live).unwrap();
        assert!(
            peer_survived,
            "absence checks must leave unrelated browsers running"
        );
        result.expect("unrelated profile metadata must not veto a free profile");
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn handoff_tab_expiry_preserves_peer_and_last_close_exits_real_browser() {
        use super::super::browser_session_host::{BrowserSessionHost, BrowserSessionHostConfig};
        use super::super::browser_session_store::BrowserSessionSqliteStore;
        use agent_browser_service_model::BrowserProfileCatalog;
        fn acquire_late_controller(browser: &ManagedBrowserInstance) -> Result<(), String> {
            use super::super::service_store::{
                default_service_state_path, JsonServiceStateStore, ServiceStateStore,
            };
            let store = JsonServiceStateStore::new(default_service_state_path()?);
            let mut state = store.load()?;
            state.viewer_leases.insert(
                "p237-late-controller".into(),
                agent_browser_service_model::ViewerLease {
                    id: "p237-late-controller".into(),
                    browser_id: Some(browser.id.clone()),
                    viewer_role: "controller".into(),
                    state: "active".into(),
                    ..Default::default()
                },
            );
            store.save(&state)?;
            let persisted = store.load()?;
            assert!(
                !super::super::service_resources::permits_managed_browser_idle_retirement(
                    &persisted,
                    browser,
                    &chrono::Utc::now().to_rfc3339(),
                )?,
                "fresh authority must recognize the newly acquired controller"
            );
            Ok(())
        }
        assert_ne!(
            std::env::var("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME")
                .ok()
                .as_deref(),
            Some("1")
        );
        for late_controller in [false, true] {
            struct RestoreSnapshot {
                path: PathBuf,
                prior: Option<Vec<u8>>,
            }
            impl Drop for RestoreSnapshot {
                fn drop(&mut self) {
                    if let Some(prior) = &self.prior {
                        let _ = fs::write(&self.path, prior);
                    } else {
                        let _ = fs::remove_file(&self.path);
                    }
                }
            }
            use super::super::service_store::{
                default_service_state_path, JsonServiceStateStore, ServiceStateStore,
            };
            let service_path = default_service_state_path().unwrap();
            let _restore = RestoreSnapshot {
                prior: fs::read(&service_path).ok(),
                path: service_path.clone(),
            };
            JsonServiceStateStore::new(&service_path)
                .save(&agent_browser_service_model::ServiceState::default())
                .unwrap();
            let directory = TempDirectory::new();
            fs::create_dir_all(&directory.0).unwrap();
            let profile = BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: "p237-real-lifetime".into(),
                name: "Synthetic P237".into(),
                user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Named,
            };
            let service = directory.0.join("service");
            let legacy = directory.0.join("unused-state.json");
            let (store, _) = BrowserSessionSqliteStore::open_or_migrate(&service, &legacy).unwrap();
            store
                .save_profile_catalog(&BrowserProfileCatalog {
                    profiles: std::collections::BTreeMap::from([(
                        profile.id.clone(),
                        profile.clone(),
                    )]),
                    ..Default::default()
                })
                .unwrap();
            let config = BrowserManagerRuntimeConfig {
                headless: true,
                ..Default::default()
            };
            struct Cleanup {
                browser: Option<ManagedBrowserInstance>,
                config: BrowserManagerRuntimeConfig,
            }
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    if let Some(browser) = &self.browser {
                        if Path::new(&format!("/proc/{}", browser.pid)).exists() {
                            if let Ok(mut runtime) =
                                BrowserManagerRuntime::start(self.config.clone())
                            {
                                let _ = runtime.close_browser(browser);
                            }
                        }
                    }
                }
            }
            let mut cleanup = Cleanup {
                browser: None,
                config: config.clone(),
            };
            let mut host = BrowserSessionHost::load(
                store,
                BrowserSessionEffectAdapter::new(
                    BrowserManagerRuntime::start(config.clone()).unwrap(),
                ),
                &legacy,
                BrowserSessionHostConfig {
                    session_idle_timeout_ms: 300_000,
                    remote_view_desktops: vec![],
                    default_disposable_policy: None,
                    exact_url_history_maximum_bytes: 4096,
                },
            )
            .unwrap();
            let opened = host
                .open(
                    agent_browser_service_model::OpenBrowserSession::exact_profile(
                        "Alice",
                        &profile.id,
                        100,
                    ),
                )
                .unwrap();
            cleanup.browser = Some(host.state().browsers[&opened.browser_id].clone());
            let alice = host
                .execute_managed_command(
                    "Alice",
                    &serde_json::json!({
                        "action":"navigate","url":"data:text/html,<title>Alice work</title>",
                        "profileId":profile.id,"activityAtMs":100
                    }),
                )
                .unwrap()
                .unwrap();
            assert_eq!(alice["success"], true);
            let browser_id = alice["browserSession"]["browserId"]
                .as_str()
                .unwrap()
                .to_string();
            let original = host.state().browsers[&browser_id].clone();
            cleanup.browser = Some(original.clone());
            host.open(
                agent_browser_service_model::OpenBrowserSession::exact_profile(
                    "Bob",
                    &profile.id,
                    110,
                ),
            )
            .unwrap();
            let bob = host
                .execute_managed_command(
                    "Bob",
                    &serde_json::json!({
                        "action":"navigate","url":"data:text/html,<title>Bob work</title>",
                        "profileId":profile.id,"activityAtMs":110
                    }),
                )
                .unwrap()
                .unwrap();
            assert_eq!(bob["success"], true);
            assert_eq!(bob["browserSession"]["browserId"], browser_id);
            // The bare local adapter does not publish Remote View links. Establish
            // only that logical fixture; all browser and tab effects remain real.
            let handoff_id = uuid::Uuid::new_v4().to_string();
            let current = host.state().clone();
            let mut retained = current.clone();
            agent_browser_service_model::retain_remote_view_tab_handoff_at(
                &mut retained,
                &handoff_id,
                &opened.session_id,
                alice["browserSession"]["tabId"].as_str().unwrap(),
                110,
            )
            .unwrap();
            let mut persistence = BrowserSessionSqliteStore::open(&service).unwrap();
            persistence
                .compare_and_save_session_state(&current, &retained)
                .unwrap();
            drop(host);
            let mut restarted_runtime = BrowserManagerRuntime::start(config.clone()).unwrap();
            if late_controller {
                restarted_runtime.idle_retirement_after_check = Some(acquire_late_controller);
            }
            let mut host = BrowserSessionHost::load(
                persistence,
                BrowserSessionEffectAdapter::new(restarted_runtime),
                &legacy,
                BrowserSessionHostConfig {
                    session_idle_timeout_ms: 300_000,
                    remote_view_desktops: vec![],
                    default_disposable_policy: None,
                    exact_url_history_maximum_bytes: 4096,
                },
            )
            .unwrap();
            let configured = host.handle_command(&serde_json::json!({
                "action":"service_remote_view_handoff_resolve","handoffId":handoff_id,
                "handoffOperation":"set_tab_lifetime","tabInactivityMs":30,"activityAtMs":110
            }));
            assert_eq!(configured["success"], true, "{configured}");
            host.reap(129).unwrap();
            assert!(host
                .state()
                .tabs
                .contains_key(alice["browserSession"]["tabId"].as_str().unwrap()));
            host.reap(130).unwrap();
            if late_controller {
                assert!(
                    host.state()
                        .tabs
                        .contains_key(alice["browserSession"]["tabId"].as_str().unwrap()),
                    "controller acquired after eligibility must preserve its live tab"
                );
                assert!(host.state().browsers.contains_key(&browser_id));
                continue;
            }
            assert!(!host
                .state()
                .tabs
                .contains_key(alice["browserSession"]["tabId"].as_str().unwrap()));
            assert!(host.state().remote_view_tab_handoffs[&handoff_id].target_expired);
            assert!(host.state().remote_view_tab_handoffs[&handoff_id]
                .expires_at_ms
                .is_none());
            assert!(Path::new(&format!("/proc/{}", original.pid)).exists());
            let title = host
                .execute_managed_command(
                    "Bob",
                    &serde_json::json!({
                        "action":"title","profileId":profile.id,"activityAtMs":131
                    }),
                )
                .unwrap()
                .unwrap();
            assert_eq!(title["data"]["title"], "Bob work");
            assert_eq!(host.state().browsers[&browser_id].pid, original.pid);
            assert_eq!(
                host.state().browsers[&browser_id].cdp_endpoint,
                original.cdp_endpoint
            );
            let closed = host.handle_command(&serde_json::json!({
                "action":"browser_session_tab_close","sessionId":bob["browserSession"]["sessionId"],
                "activityAtMs":140
            }));
            assert_eq!(closed["success"], true, "{closed}");
            assert!(host.state().browsers.is_empty());
            for _ in 0..50 {
                if !process_exists(original.pid) {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
            assert!(
                !process_exists(original.pid),
                "owned Chrome remained live after last-tab close"
            );
            // Reloading the worker inside this same OS process drops the original
            // Child handle. Reap only this fixture's exited direct child, which a
            // real owner-process exit would instead reparent to the OS reaper.
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let mut status = 0;
                let reaped =
                    unsafe { libc::waitpid(original.pid as i32, &mut status, libc::WNOHANG) };
                let error = (reaped == -1).then(|| std::io::Error::last_os_error().raw_os_error());
                if reaped == original.pid as i32 || error == Some(Some(libc::ECHILD)) {
                    break;
                }
                assert!(
                    reaped == 0 || error == Some(Some(libc::EINTR)),
                    "fixture child reaping failed: {error:?}"
                );
                assert!(
                    Instant::now() < deadline,
                    "fixture child reaping exceeded deadline"
                );
                thread::sleep(Duration::from_millis(20));
            }
            assert!(
                !Path::new(&format!("/proc/{}", original.pid)).exists(),
                "fixture child remained after reaping"
            );
            assert!(host
                .state()
                .remote_view_tab_handoffs
                .contains_key(&handoff_id));
        }
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn ordinary_open_reconciles_stale_inventory_and_preserves_alice_bob() {
        use agent_browser_service_model::{
            BrowserProfileCatalog, BrowserSessionManager, BrowserSessionManagerConfig,
            BrowserSessionState, ManagedBrowserCommandEffects, OpenBrowserSession,
            SessionEndReason,
        };
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "ordinary-reuse".into(),
            name: "Ordinary reuse".into(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let catalog = BrowserProfileCatalog {
            profiles: std::collections::BTreeMap::from([(profile.id.clone(), profile.clone())]),
            ..Default::default()
        };
        let config = BrowserManagerRuntimeConfig {
            headless: true,
            ..Default::default()
        };
        let policy = BrowserSessionManagerConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: vec![],
        };
        let mut owner =
            BrowserSessionEffectAdapter::new(BrowserManagerRuntime::start(config.clone()).unwrap());
        let mut state = BrowserSessionState::default();
        let (alice, bob) = {
            let mut manager =
                BrowserSessionManager::new(&mut state, &catalog, &mut owner, policy.clone());
            let alice = manager
                .open(OpenBrowserSession::exact_profile(
                    "alice",
                    &profile.id,
                    1_000,
                ))
                .unwrap();
            manager
                .navigate(
                    &alice.session_id,
                    "data:text/html,<title>Alice work</title>",
                    1_000,
                )
                .unwrap();
            let bob = manager
                .open(OpenBrowserSession::exact_profile("bob", &profile.id, 2_000))
                .unwrap();
            manager
                .navigate(
                    &bob.session_id,
                    "data:text/html,<title>Bob work</title>",
                    2_000,
                )
                .unwrap();
            (alice, bob)
        };
        let original = state.browsers[&alice.browser_id].clone();
        let alice_tab = state.tabs[state.sessions[&alice.session_id]
            .current_tab_id
            .as_ref()
            .unwrap()]
        .clone();
        owner.close_tab(&original, &alice_tab).unwrap();
        state
            .leave_open_session_ids
            .extend([alice.session_id.clone(), bob.session_id.clone()]);
        state.browsers.get_mut(&alice.browser_id).unwrap().pid = u32::MAX;
        state
            .browsers
            .get_mut(&alice.browser_id)
            .unwrap()
            .cdp_endpoint = "ws://127.0.0.1:1/devtools/browser/stale".into();
        let mut state: BrowserSessionState =
            serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        let unpublished_baseline = state.clone();
        let mut requester =
            BrowserSessionEffectAdapter::new(BrowserManagerRuntime::start(config.clone()).unwrap());
        let result = (|| -> Result<(), String> {
            {
                let mut manager = BrowserSessionManager::new(
                    &mut state,
                    &catalog,
                    &mut requester,
                    policy.clone(),
                );
                let resumed = manager.open(OpenBrowserSession::exact_profile(
                    "alice",
                    &profile.id,
                    3_000,
                ))?;
                if resumed.session_id != alice.session_id || resumed.browser_id != alice.browser_id
                {
                    return Err("continuation changed Alice's logical identity".into());
                }
                let repeated = manager.open(OpenBrowserSession::exact_profile(
                    "alice",
                    &profile.id,
                    4_000,
                ))?;
                if repeated != resumed {
                    return Err("replay changed acquisition identity".into());
                }
            }
            let restored = state.tabs[&alice_tab.id].clone();
            let progress = requester.execute_command(
                &state.browsers[&alice.browser_id], &restored, &alice.session_id, "alice",
                &serde_json::json!({"id":"alice-progress", "action":"evaluate", "script":"document.title='Alice progressed'"}),
            )?;
            if progress["success"] != true {
                return Err(format!("Alice progress fixture failed: {progress}"));
            }
            let progressed = requester.execute_command(
                &state.browsers[&alice.browser_id],
                &restored,
                &alice.session_id,
                "alice",
                &serde_json::json!({"id":"alice-progress-observed", "action":"title"}),
            )?;
            if progressed["data"]["title"] != "Alice progressed" {
                return Err("Alice progress fixture did not change its page".into());
            }
            drop(requester);
            state = unpublished_baseline.clone();
            requester =
                BrowserSessionEffectAdapter::new(BrowserManagerRuntime::start(config.clone())?);
            {
                let mut manager = BrowserSessionManager::new(
                    &mut state,
                    &catalog,
                    &mut requester,
                    policy.clone(),
                );
                manager.open(OpenBrowserSession::exact_profile(
                    "alice",
                    &profile.id,
                    4_500,
                ))?;
            }
            if state.tabs[&alice_tab.id].target_id != restored.target_id {
                return Err("retained recovery publication retry duplicated Alice's target".into());
            }
            let title = requester.execute_command(
                &state.browsers[&alice.browser_id],
                &state.tabs[&alice_tab.id],
                &alice.session_id,
                "alice",
                &serde_json::json!({"id":"alice-preserved", "action":"title"}),
            )?;
            if title["data"]["title"] != "Alice progressed" {
                return Err("recovery replay reset Alice's work".into());
            }
            state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
            drop(requester);
            requester =
                BrowserSessionEffectAdapter::new(BrowserManagerRuntime::start(config.clone())?);
            requester.begin_operation(&state)?;
            {
                let mut manager = BrowserSessionManager::new(
                    &mut state,
                    &catalog,
                    &mut requester,
                    policy.clone(),
                );
                manager.close_session(&alice.session_id, SessionEndReason::ExplicitClose, 5_000)?;
            }
            let browser = &state.browsers[&bob.browser_id];
            let bob_tab = &state.tabs[state.sessions[&bob.session_id]
                .current_tab_id
                .as_ref()
                .unwrap()];
            let title = requester.execute_command(
                browser,
                bob_tab,
                &bob.session_id,
                "bob",
                &serde_json::json!({"id":"bob-after-alice", "action":"title"}),
            )?;
            if title["data"]["title"] != "Bob work" {
                return Err("Alice continuation or close changed Bob's work".into());
            }
            Ok(())
        })();
        owner.close_browser(&original).unwrap();
        result.expect("ordinary stale-inventory continuation must preserve both tasks");
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn runtime_restart_reattaches_and_closes_recorded_browser() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "restart-fixture".to_string(),
            name: "Restart fixture".to_string(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let config = BrowserManagerRuntimeConfig {
            headless: true,
            ..BrowserManagerRuntimeConfig::default()
        };
        let launch = {
            let mut first = BrowserManagerRuntime::start(config.clone()).unwrap();
            first.launch_browser(&profile, None).unwrap()
        };
        let original_process_identity = launch
            .process_identity
            .clone()
            .expect("the original live child must supply kernel identity");
        assert_eq!(original_process_identity.pid, launch.pid);
        assert!(!original_process_identity.start_token.is_empty());
        let browser = ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: profile.id,
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: None,
            active_session_ids: vec!["session:alice:restart-fixture:1".to_string()],
        };
        assert!(recorded_browser_process_exists(&browser));
        assert_eq!(
            crate::process_identity::capture_process_identity(browser.pid, None, None),
            Some(original_process_identity)
        );

        let mut restarted = BrowserManagerRuntime::start(config).unwrap();
        // Creating a tab must itself reattach after an owner restart.
        // Keep the result until exact cleanup so a regression cannot orphan Chrome.
        let new_tab = restarted.create_tab(&browser);
        let live = restarted.browser_is_live(&browser).unwrap();
        let tab = restarted.acquire_initial_tab(&browser, &[]).unwrap();
        let managed_tab = ManagedBrowserTab {
            id: tab.tab_id,
            target_id: tab.target_id,
            browser_id: browser.id.clone(),
            session_id: "session:alice:restart-fixture:1".to_string(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        };
        restarted
            .navigate(
                &browser,
                &managed_tab,
                "data:text/html,<title>managed-session</title><main>ready</main>",
            )
            .unwrap();
        let title = restarted
            .execute_command(
                &browser,
                &managed_tab,
                &managed_tab.session_id,
                "alice",
                &serde_json::json!({ "id": "title-1", "action": "title" }),
            )
            .unwrap();
        let snapshot = restarted
            .execute_command(
                &browser,
                &managed_tab,
                &managed_tab.session_id,
                "alice",
                &serde_json::json!({ "id": "snapshot-1", "action": "snapshot" }),
            )
            .unwrap();
        let close = restarted.close_browser(&browser);

        assert!(live);
        assert_eq!(title["success"], true);
        assert_eq!(title["data"]["title"], "managed-session");
        assert_eq!(snapshot["success"], true);
        close.unwrap();
        assert!(!recorded_browser_process_exists(&browser));
        assert!(
            new_tab.is_ok(),
            "tab creation failed after restart: {new_tab:?}"
        );
    }

    #[test]
    fn recovery_absence_accepts_unrelated_pid_and_rejects_unavailable_profile() {
        let root = TempDirectory::new();
        fs::create_dir_all(&root.0).unwrap();
        let profile = BrowserProfileCatalogEntry {
            manual_login_url: None,
            id: "profile-a".into(),
            name: "Synthetic".into(),
            user_data_dir: root.0.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let mut browser = ManagedBrowserInstance {
            id: "browser-a".into(),
            profile_id: profile.id.clone(),
            pid: std::process::id(),
            cdp_endpoint: "http://127.0.0.1:9222".into(),
            desktop: None,
            active_session_ids: vec![],
        };
        prove_browser_recovery_absence(&browser, &profile).unwrap();
        browser.pid = u32::MAX;
        let mut missing = profile.clone();
        missing.user_data_dir = root.0.join("absent").to_string_lossy().into_owned();
        assert_eq!(
            prove_browser_recovery_absence(&browser, &missing).unwrap_err(),
            "browser_session_recovery_profile_unavailable"
        );
        prove_browser_recovery_absence(&browser, &profile).unwrap();
    }

    #[test]
    fn recovery_pid_reuse_requires_positive_unrelated_identity() {
        use crate::process_identity::{ObservedProcessIdentity, ProcessObservation};
        let root = TempDirectory::new();
        let expected = root.0.join("retained");
        let peer = root.0.join("peer");
        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&peer).unwrap();
        let expected = fs::canonicalize(expected).unwrap();
        let mut identity = ObservedProcessIdentity {
            pid: 42,
            start_token: Some("new-process".into()),
            executable_path: Some("/synthetic/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line: Some(vec![format!("--user-data-dir={}", peer.display())]),
        };
        prove_recovery_pid_absence(ProcessObservation::Observed(identity.clone()), &expected)
            .unwrap();
        identity.command_line = Some(vec![format!("--user-data-dir={}", expected.display())]);
        assert!(prove_recovery_pid_absence(
            ProcessObservation::Observed(identity.clone()),
            &expected
        )
        .is_err());
        identity.command_line = None;
        assert!(prove_recovery_pid_absence(
            ProcessObservation::Observed(identity.clone()),
            &expected
        )
        .is_err());
        identity.browser_family = None;
        prove_recovery_pid_absence(ProcessObservation::Observed(identity.clone()), &expected)
            .unwrap();
        identity.executable_path = None;
        assert!(
            prove_recovery_pid_absence(ProcessObservation::Observed(identity), &expected).is_err()
        );
        assert!(prove_recovery_pid_absence(
            ProcessObservation::Failed {
                reason: "unavailable".into()
            },
            &expected
        )
        .is_err());
    }

    #[test]
    fn disposable_cleanup_is_limited_to_the_recorded_direct_child() {
        let root = TempDirectory::new();
        let policy = BrowserDisposableProfilePolicy {
            id: "default".to_string(),
            user_data_root: root.0.to_string_lossy().into_owned(),
            cleanup_delay_ms: 0,
            maximum_retained_profiles: 20,
            maximum_total_bytes: 10 * 1024 * 1024 * 1024,
        };
        let mut effects = BrowserSessionEffectAdapter::new(FakeRuntime);

        let profile = effects
            .allocate_disposable_profile(&policy, "disposable:default:1", "alice")
            .unwrap();
        let allocation = ManagedDisposableProfile {
            profile: profile.clone(),
            policy_id: policy.id,
            session_name: "alice".to_string(),
            user_data_root: policy.user_data_root,
            created_at_ms: 1,
            cleanup_delay_ms: 0,
            cleanup_eligible_at_ms: Some(1),
        };
        assert!(Path::new(&profile.user_data_dir).is_dir());

        effects.delete_disposable_profile(&allocation).unwrap();

        assert!(!Path::new(&profile.user_data_dir).exists());
        assert!(root.0.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn disposable_size_counts_regular_files_without_following_symlinks() {
        use std::os::unix::fs::symlink;

        let root = TempDirectory::new();
        let outside = TempDirectory::new();
        let profile = root.0.join("profile");
        fs::create_dir_all(profile.join("nested")).unwrap();
        fs::create_dir_all(&outside.0).unwrap();
        fs::write(profile.join("one.bin"), [0_u8; 3]).unwrap();
        fs::write(profile.join("nested/two.bin"), [0_u8; 5]).unwrap();
        fs::write(outside.0.join("outside.bin"), [0_u8; 64]).unwrap();
        symlink(outside.0.join("outside.bin"), profile.join("file-link")).unwrap();
        symlink(&outside.0, profile.join("directory-link")).unwrap();

        assert_eq!(directory_regular_file_bytes(&profile).unwrap(), 8);
    }
}

#[cfg(test)]
mod hosted_reconnect_connection_tests {
    use super::*;
    #[test]
    fn hosted_reconnect_replaces_only_changed_borrowed_connection_without_close() {
        let row = BorrowedAppBrowser {
            profile_id: "profile".into(),
            process: agent_browser_service_model::RecordedProcessIdentity {
                pid: 10,
                start_token: "old".into(),
                executable_path: Some("/synthetic/chrome".into()),
                browser_family: None,
            },
            endpoint: "ws://127.0.0.1:1/devtools/browser/old".into(),
            targets: BTreeSet::new(),
        };
        let previous = HashMap::from([
            ("logical".into(), row.clone()),
            ("peer".into(), row.clone()),
        ]);
        let mut next = previous.clone();
        next.get_mut("logical").unwrap().process.start_token = "new".into();
        let mut browsers = HashMap::from([
            ("logical".into(), DaemonState::new()),
            ("peer".into(), DaemonState::new()),
        ]);
        invalidate_changed_borrowed_connections(&mut browsers, &previous, &next).unwrap();
        assert!(!browsers.contains_key("logical"));
        assert!(browsers.contains_key("peer"));
        invalidate_changed_borrowed_connections(&mut browsers, &next, &next).unwrap();
        assert!(browsers.contains_key("peer"));
    }
}

/// Read-only Native workspace presence for a CDP-free managed process. This
/// checks the published original identity without creating a CDP connection.
pub(super) fn manual_browser_is_live(browser: &ManagedBrowserInstance) -> Result<bool, String> {
    manual::ManualBrowsers::default().verify(browser)
}
