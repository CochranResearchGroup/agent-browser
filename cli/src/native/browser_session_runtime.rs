//! BrowserManager-backed effects for the ordinary Browser Session Manager.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use agent_browser_service_model::{
    BrowserDisposableProfilePolicy, BrowserLaunch, BrowserLaunchIntent, BrowserProfileCatalogEntry,
    BrowserProfileKind, BrowserSessionEffects, BrowserTabAcquisition, BrowserTabSource,
    ManagedBrowserInstance, ManagedBrowserTab, ManagedDisposableProfile,
    RemoteViewAssignmentObservation, RemoteViewBrowserProcessEffects,
    RemoteViewBrowserProcessError, RemoteViewDesktopRuntimeContext, RemoteViewFixedDesktop,
    RemoteViewPrivateLaunchEnvironment,
};
use serde_json::Value;

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

    fn select_browser_executable(&mut self, _path: String) -> Result<(), String> {
        Err("browser_build_selection_unsupported".into())
    }

    fn begin_operation(
        &mut self,
        _expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        Ok(())
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        None
    }
    fn acknowledge_launch_publication(&mut self) {}
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

    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        self.runtime.select_browser_executable(path)
    }

    fn begin_operation(
        &mut self,
        expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        self.runtime.begin_operation(expected)
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        self.runtime.pending_launch_intent()
    }
    fn acknowledge_launch_publication(&mut self) {
        self.runtime.acknowledge_launch_publication();
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

enum BrowserRuntimeCommand {
    ReconcileProfile {
        profile: BrowserProfileCatalogEntry,
        browser: Option<ManagedBrowserInstance>,
        tabs: Vec<ManagedBrowserTab>,
        navigation: Vec<agent_browser_service_model::BrowserNavigationRecord>,
        reply: mpsc::Sender<Result<Option<agent_browser_service_model::BrowserRecovery>, String>>,
    },
    OperationBaseline {
        shared_browser_ids: BTreeSet<String>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    PreflightLaunch {
        profile: BrowserProfileCatalogEntry,
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
    Shutdown,
}

pub(crate) struct BrowserManagerRuntime {
    commands: mpsc::Sender<BrowserRuntimeCommand>,
    worker: Option<thread::JoinHandle<()>>,
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
    fn begin_operation(
        &mut self,
        expected: &agent_browser_service_model::BrowserSessionState,
    ) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::OperationBaseline {
                shared_browser_ids: expected.shared_browser_ids.clone(),
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

fn run_browser_worker(
    runtime: tokio::runtime::Runtime,
    receiver: mpsc::Receiver<BrowserRuntimeCommand>,
    mut config: BrowserManagerRuntimeConfig,
) {
    let mut browsers = HashMap::<String, DaemonState>::new();
    let mut shared_browser_tabs = HashMap::<String, BTreeSet<String>>::new();
    let mut reconciled_targets = HashMap::<(String, String), String>::new();
    let mut next_browser_sequence = 0_u64;
    while let Ok(command) = receiver.recv() {
        match command {
            BrowserRuntimeCommand::OperationBaseline {
                shared_browser_ids,
                reply,
            } => {
                for id in shared_browser_ids {
                    shared_browser_tabs.entry(id).or_default();
                }
                let _ = reply.send(Ok(()));
            }
            BrowserRuntimeCommand::ReconcileProfile {
                profile,
                browser,
                tabs,
                navigation,
                reply,
            } => {
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
            BrowserRuntimeCommand::SelectExecutable { path, reply } => {
                config.executable_path = Some(path);
                let _ = reply.send(Ok(()));
            }
            BrowserRuntimeCommand::IsLive { browser, reply } => {
                let result = runtime.block_on(recorded_browser_is_live(&mut browsers, &browser));
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Launch {
                profile,
                desktop,
                environment,
                recovery_browser,
                reply,
            } => {
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
                            executable_path: config.executable_path.clone(),
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
                    next_browser_sequence = sequence;
                    let browser_id = recovery_browser.as_ref().map_or_else(
                        || format!("browser:{}:{}", profile.id, uuid::Uuid::new_v4()),
                        |browser| browser.id.clone(),
                    );
                    let launch = BrowserLaunch {
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
                    None if !recorded_browser_process_exists(&browser) => Ok(()),
                    None => runtime.block_on(async {
                        let manager = attach_recorded_browser(&browser).await?;
                        close_reattached_browser(&manager, browser.pid).await
                    }),
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
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
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
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::CloseTab {
                browser,
                tab,
                reply,
            } => {
                let result = runtime.block_on(async {
                    if !recorded_browser_is_live(&mut browsers, &browser).await? {
                        return if !recorded_browser_process_exists(&browser) {
                            Ok(())
                        } else {
                            Err("browser_session_reattach_unresponsive".into())
                        };
                    }
                    let manager = browsers
                        .get_mut(&browser.id)
                        .and_then(|state| state.browser.as_mut())
                        .ok_or("browser_session_runtime_browser_missing")?;
                    manager
                        .tab_close_target_id_for_release(&tab.target_id)
                        .await?;
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
                session_id,
                session_name,
                command,
                reply,
            } => {
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
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
                        manager.tab_switch_target_id(&tab.target_id).await?;
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
                };
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
    let (port, browser_path) = super::cdp::chrome::read_devtools_active_port(&expected)
        .filter(|(port, path)| *port != 0 && path.starts_with("/devtools/browser/"))
        .ok_or("browser_session_profile_debug_endpoint_unavailable")?;
    let endpoint = format!("ws://127.0.0.1:{port}{browser_path}");
    let manager = attach_verified_browser_endpoint(&endpoint, pid, before).await?;
    Ok(Some((
        BrowserLaunch {
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

async fn close_reattached_browser(
    manager: &BrowserManager,
    recorded_pid: u32,
) -> Result<(), String> {
    let close_error = manager
        .client
        .send_command_no_params("Browser.close", None)
        .await
        .err();
    let deadline = Instant::now() + Duration::from_secs(5);
    while process_exists(recorded_pid) {
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
        let browser = ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: profile.id,
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            desktop: None,
            active_session_ids: vec!["session:alice:restart-fixture:1".to_string()],
        };
        assert!(recorded_browser_process_exists(&browser));

        let mut restarted = BrowserManagerRuntime::start(config).unwrap();
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
    }

    #[test]
    fn recovery_absence_accepts_unrelated_pid_and_rejects_unavailable_profile() {
        let root = TempDirectory::new();
        fs::create_dir_all(&root.0).unwrap();
        let profile = BrowserProfileCatalogEntry {
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
