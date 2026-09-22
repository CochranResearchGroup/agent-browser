//! BrowserManager-backed effects for the ordinary Browser Session Manager.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use agent_browser_service_model::{
    BrowserDesktopAssignment, BrowserDisposableProfilePolicy, BrowserLaunch,
    BrowserProfileCatalogEntry, BrowserProfileKind, BrowserSessionEffects, BrowserTabAcquisition,
    BrowserTabSource, ManagedBrowserInstance, ManagedBrowserTab, ManagedDisposableProfile,
};
use serde_json::Value;

use crate::process_identity::{VerifiedProcessSignal, VerifiedProcessTermination};

use super::action_runtime::runtime::DaemonState;
use super::browser::{BrowserManager, WaitUntil};
use super::browser_session_host::navigation_recovery::{
    NavigationTargetObservation, NavigationTargetObservationEffects,
};
use super::cdp::chrome::LaunchOptions;

const RECORDED_BROWSER_REATTACH_TIMEOUT: Duration = Duration::from_secs(5);

fn reserved_browser_launch_args(browser_id: Option<&str>) -> Vec<String> {
    browser_id
        .map(|browser_id| vec![format!("--agent-browser-reservation-id={browser_id}")])
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct BrowserManagerRuntimeConfig {
    pub(crate) headless: bool,
    pub(crate) executable_path: Option<String>,
    pub(crate) display: Option<String>,
    pub(crate) remote_headed: bool,
}

pub(crate) trait BrowserRuntimeDriver {
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String>;
    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
    ) -> Result<BrowserLaunch, String>;
    fn launch_browser_reserved(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
        browser_id: &str,
    ) -> Result<BrowserLaunch, String> {
        let launch = self.launch_browser(profile, desktop)?;
        if launch.browser_id != browser_id {
            return Err("browser_session_reserved_browser_identity_mismatch".to_string());
        }
        Ok(launch)
    }
    fn recover_browser_reserved(
        &mut self,
        _profile: &BrowserProfileCatalogEntry,
        _desktop: &BrowserDesktopAssignment,
        _browser_id: &str,
    ) -> Result<ReservedBrowserRecovery, String> {
        Ok(ReservedBrowserRecovery::Unproven {
            reason: "causal_process_discovery_unavailable".to_string(),
        })
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String>;
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String>;
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

    fn observe_navigation_target(
        &mut self,
        _browser: &ManagedBrowserInstance,
        _tab: &ManagedBrowserTab,
    ) -> Result<NavigationTargetObservation, String> {
        Err("browser_session_navigation_observation_unsupported".to_string())
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReservedBrowserRecovery {
    Recovered(BrowserLaunch),
    Unproven { reason: String },
}

pub(crate) struct BrowserSessionEffectAdapter<D> {
    runtime: D,
}

impl<D> BrowserSessionEffectAdapter<D> {
    pub(crate) fn new(runtime: D) -> Self {
        Self { runtime }
    }
}

pub(crate) trait ReservedBrowserRecoveryEffects {
    fn recover_browser_reserved(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: &BrowserDesktopAssignment,
        browser_id: &str,
    ) -> Result<ReservedBrowserRecovery, String>;
}

impl<D: BrowserRuntimeDriver> ReservedBrowserRecoveryEffects for BrowserSessionEffectAdapter<D> {
    fn recover_browser_reserved(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: &BrowserDesktopAssignment,
        browser_id: &str,
    ) -> Result<ReservedBrowserRecovery, String> {
        self.runtime
            .recover_browser_reserved(profile, desktop, browser_id)
    }
}

pub(crate) trait ManagedBrowserCommandEffects {
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &Value,
    ) -> Result<Value, String>;
}

impl<D: BrowserRuntimeDriver> NavigationTargetObservationEffects
    for BrowserSessionEffectAdapter<D>
{
    fn observe_navigation_target(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<NavigationTargetObservation, String> {
        self.runtime.observe_navigation_target(browser, tab)
    }
}

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

impl<D: BrowserRuntimeDriver> BrowserSessionEffects for BrowserSessionEffectAdapter<D> {
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        self.runtime.browser_is_live(browser)
    }

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
    ) -> Result<BrowserLaunch, String> {
        self.runtime.launch_browser(profile, desktop)
    }

    fn launch_browser_reserved(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
        browser_id: &str,
    ) -> Result<BrowserLaunch, String> {
        self.runtime
            .launch_browser_reserved(profile, desktop, browser_id)
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
    IsLive {
        browser: ManagedBrowserInstance,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Launch {
        profile: BrowserProfileCatalogEntry,
        desktop: Option<BrowserDesktopAssignment>,
        browser_id: Option<String>,
        reply: mpsc::Sender<Result<BrowserLaunch, String>>,
    },
    Close {
        browser: ManagedBrowserInstance,
        reply: mpsc::Sender<Result<(), String>>,
    },
    AcquireInitialTab {
        browser: ManagedBrowserInstance,
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
    ObserveNavigationTarget {
        browser: ManagedBrowserInstance,
        tab: ManagedBrowserTab,
        reply: mpsc::Sender<Result<NavigationTargetObservation, String>>,
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

impl BrowserRuntimeDriver for BrowserManagerRuntime {
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
        desktop: Option<&BrowserDesktopAssignment>,
    ) -> Result<BrowserLaunch, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Launch {
                profile: profile.clone(),
                desktop: desktop.cloned(),
                browser_id: None,
                reply,
            },
        )
    }

    fn launch_browser_reserved(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
        browser_id: &str,
    ) -> Result<BrowserLaunch, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Launch {
                profile: profile.clone(),
                desktop: desktop.cloned(),
                browser_id: Some(browser_id.to_string()),
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

    fn observe_navigation_target(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<NavigationTargetObservation, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::ObserveNavigationTarget {
                browser: browser.clone(),
                tab: tab.clone(),
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
    config: BrowserManagerRuntimeConfig,
) {
    let mut browsers = HashMap::<String, ManagedBrowserRuntimeState>::new();
    let mut next_browser_sequence = 0_u64;
    while let Ok(command) = receiver.recv() {
        match command {
            BrowserRuntimeCommand::IsLive { browser, reply } => {
                let result = runtime.block_on(recorded_browser_is_live(&mut browsers, &browser));
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Launch {
                profile,
                desktop,
                browser_id,
                reply,
            } => {
                let result = runtime.block_on(async {
                    let display = desktop
                        .as_ref()
                        .map(|desktop| desktop.display_name.clone())
                        .or_else(|| config.display.clone());
                    let mut manager = BrowserManager::launch(
                        LaunchOptions {
                            headless: config.headless && display.is_none(),
                            executable_path: config.executable_path.clone(),
                            profile: Some(profile.user_data_dir.clone()),
                            display,
                            remote_headed: config.remote_headed || desktop.is_some(),
                            args: reserved_browser_launch_args(browser_id.as_deref()),
                            ..LaunchOptions::default()
                        },
                        Some("chrome"),
                    )
                    .await?;
                    manager.approve_lifecycle_close();
                    let pid = manager
                        .browser_pid()
                        .ok_or_else(|| "browser_session_launch_pid_missing".to_string())?;
                    let process_identity = crate::process_identity::capture_process_identity(
                        pid,
                        None,
                        Some("chrome"),
                    )
                    .ok_or_else(|| "browser_session_launch_process_identity_missing".to_string())?;
                    let browser_id = match browser_id {
                        Some(browser_id) => browser_id,
                        None => {
                            next_browser_sequence =
                                next_browser_sequence.checked_add(1).ok_or_else(|| {
                                    "browser_session_browser_sequence_exhausted".to_string()
                                })?;
                            format!("browser:{}:{next_browser_sequence}", profile.id)
                        }
                    };
                    let launch = BrowserLaunch {
                        browser_id: browser_id.clone(),
                        pid,
                        cdp_endpoint: manager.get_cdp_url().to_string(),
                        process_identity: Some(process_identity),
                        desktop,
                    };
                    let mut lifecycle = DaemonState::new();
                    lifecycle.browser = Some(manager);
                    lifecycle.browser_session_manager_owned = true;
                    browsers.insert(
                        browser_id,
                        ManagedBrowserRuntimeState {
                            lifecycle,
                            command_sessions: HashMap::new(),
                        },
                    );
                    Ok(launch)
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Close { browser, reply } => {
                let result = match browsers.remove(&browser.id) {
                    Some(mut state) => runtime.block_on(async {
                        relinquish_command_sessions(&mut state.command_sessions);
                        let manager =
                            state.lifecycle.browser.as_mut().ok_or_else(|| {
                                "browser_session_runtime_browser_missing".to_string()
                            })?;
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
                        match attach_recorded_browser(&browser).await {
                            Ok(manager) => close_reattached_browser(&manager, browser.pid).await,
                            Err(attach_error) => close_unresponsive_recorded_browser(&browser)
                                .map_err(|termination_error| {
                                    format!(
                                        "{attach_error}; browser_session_exact_termination_failed:{termination_error}"
                                    )
                                }),
                        }
                    }),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::AcquireInitialTab {
                browser,
                attributed_target_ids,
                reply,
            } => {
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager =
                            state.lifecycle.browser.as_mut().ok_or_else(|| {
                                "browser_session_runtime_browser_missing".to_string()
                            })?;
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
                        let manager =
                            state.lifecycle.browser.as_mut().ok_or_else(|| {
                                "browser_session_runtime_browser_missing".to_string()
                            })?;
                        let tab = manager.tab_new(None).await?;
                        tab_acquisition(tab, BrowserTabSource::ExplicitNew)
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
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager =
                            state.lifecycle.browser.as_mut().ok_or_else(|| {
                                "browser_session_runtime_browser_missing".to_string()
                            })?;
                        manager
                            .tab_close_target_id_for_release(&tab.target_id)
                            .await?;
                        remove_command_session(&mut state.command_sessions, &tab.session_id);
                        Ok(())
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                };
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
                        let manager =
                            state.lifecycle.browser.as_mut().ok_or_else(|| {
                                "browser_session_runtime_browser_missing".to_string()
                            })?;
                        manager.tab_switch_target_id(&tab.target_id).await?;
                        manager.navigate(&url, WaitUntil::Load).await?;
                        remove_command_session(&mut state.command_sessions, &tab.session_id);
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
                        let manager =
                            state.lifecycle.browser.as_mut().ok_or_else(|| {
                                "browser_session_runtime_browser_missing".to_string()
                            })?;
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
                        let command_state = session_command_state(
                            state,
                            &browser,
                            &tab,
                            &session_id,
                            &session_name,
                        )
                        .await?;
                        Ok(super::actions::execute_command(&command, command_state).await)
                    }),
                    None => Err("browser_session_runtime_browser_missing".to_string()),
                };
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::ObserveNavigationTarget {
                browser,
                tab,
                reply,
            } => {
                let result = runtime.block_on(async {
                    if !recorded_browser_is_live(&mut browsers, &browser).await? {
                        return Err("browser_session_recorded_process_missing".to_string());
                    }
                    let manager = browsers
                        .get(&browser.id)
                        .and_then(|state| state.lifecycle.browser.as_ref())
                        .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                    // Read the exact live target without switching tabs, executing
                    // renderer JavaScript, or repeating a navigation request.
                    let result = manager
                        .client
                        .send_command(
                            "Target.getTargetInfo",
                            Some(serde_json::json!({ "targetId": tab.target_id })),
                            None,
                        )
                        .await?;
                    let info = &result["targetInfo"];
                    let target_id = info["targetId"]
                        .as_str()
                        .ok_or_else(|| "browser_session_navigation_target_missing".to_string())?;
                    let url = info["url"]
                        .as_str()
                        .ok_or_else(|| "browser_session_navigation_url_missing".to_string())?;
                    if target_id != tab.target_id || tab.browser_id != browser.id {
                        return Err("browser_session_navigation_target_mismatch".to_string());
                    }
                    Ok(NavigationTargetObservation {
                        target_id: target_id.to_string(),
                        url: url.to_string(),
                    })
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Shutdown => break,
        }
    }
    for state in browsers.values_mut() {
        relinquish_command_sessions(&mut state.command_sessions);
        if let Some(manager) = state.lifecycle.browser.as_mut() {
            manager.relinquish_browser_for_handoff();
        }
    }
}

struct ManagedBrowserRuntimeState {
    lifecycle: DaemonState,
    command_sessions: HashMap<String, SessionCommandRuntimeState>,
}

struct SessionCommandRuntimeState {
    target_id: String,
    state: DaemonState,
}

async fn session_command_state<'a>(
    runtime_state: &'a mut ManagedBrowserRuntimeState,
    browser: &ManagedBrowserInstance,
    tab: &ManagedBrowserTab,
    session_id: &str,
    session_name: &str,
) -> Result<&'a mut DaemonState, String> {
    let target_changed = runtime_state
        .command_sessions
        .get(session_id)
        .is_some_and(|current| current.target_id != tab.target_id);
    if target_changed {
        remove_command_session(&mut runtime_state.command_sessions, session_id);
    }
    if !runtime_state.command_sessions.contains_key(session_id) {
        let mut manager = attach_recorded_browser(browser).await?;
        manager.tab_switch_target_id(&tab.target_id).await?;
        let mut state = DaemonState::new();
        state.browser = Some(manager);
        state.browser_session_manager_owned = true;
        state.session_id = session_id.to_string();
        state.session_name = Some(session_name.to_string());
        state.subscribe_to_browser_events();
        state.start_fetch_handler();
        state.start_dialog_handler();
        runtime_state.command_sessions.insert(
            session_id.to_string(),
            SessionCommandRuntimeState {
                target_id: tab.target_id.clone(),
                state,
            },
        );
    }
    let command_state = runtime_state
        .command_sessions
        .get_mut(session_id)
        .ok_or_else(|| "browser_session_command_state_missing".to_string())?;
    command_state.state.session_name = Some(session_name.to_string());
    Ok(&mut command_state.state)
}

fn remove_command_session(
    command_sessions: &mut HashMap<String, SessionCommandRuntimeState>,
    session_id: &str,
) {
    if let Some(mut command_state) = command_sessions.remove(session_id) {
        if let Some(manager) = command_state.state.browser.as_mut() {
            manager.relinquish_browser_for_handoff();
        }
    }
}

fn relinquish_command_sessions(command_sessions: &mut HashMap<String, SessionCommandRuntimeState>) {
    for command_state in command_sessions.values_mut() {
        if let Some(manager) = command_state.state.browser.as_mut() {
            manager.relinquish_browser_for_handoff();
        }
    }
    command_sessions.clear();
}

async fn recorded_browser_is_live(
    browsers: &mut HashMap<String, ManagedBrowserRuntimeState>,
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
        let mut lifecycle = DaemonState::new();
        lifecycle.browser = Some(manager);
        lifecycle.browser_session_manager_owned = true;
        browsers.insert(
            browser.id.clone(),
            ManagedBrowserRuntimeState {
                lifecycle,
                command_sessions: HashMap::new(),
            },
        );
    }
    let state = browsers
        .get_mut(&browser.id)
        .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
    let manager = state
        .lifecycle
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

async fn attach_recorded_browser(
    browser: &ManagedBrowserInstance,
) -> Result<BrowserManager, String> {
    attach_recorded_browser_with_timeout(browser, RECORDED_BROWSER_REATTACH_TIMEOUT).await
}

async fn attach_recorded_browser_with_timeout(
    browser: &ManagedBrowserInstance,
    timeout: Duration,
) -> Result<BrowserManager, String> {
    if !recorded_browser_process_exists(browser) {
        return Err("browser_session_recorded_process_missing".to_string());
    }
    tokio::time::timeout(timeout, async {
        let manager = BrowserManager::connect_cdp(&browser.cdp_endpoint)
            .await
            .map_err(|error| format!("browser_session_reattach_failed:{error}"))?;
        if !manager.is_connection_alive().await {
            return Err("browser_session_reattach_unresponsive".to_string());
        }
        Ok(manager)
    })
    .await
    .map_err(|_| "browser_session_reattach_timeout".to_string())?
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

/// Close a manager-recorded browser only when its retained process identity
/// opens an exact kernel-backed termination capability. CDP failure alone is
/// never authority to signal an unverified PID.
fn close_unresponsive_recorded_browser(browser: &ManagedBrowserInstance) -> Result<(), String> {
    let identity = browser
        .process_identity
        .as_ref()
        .ok_or_else(|| "browser_session_close_process_identity_missing".to_string())?;
    if identity.pid != browser.pid {
        return Err("browser_session_close_process_identity_pid_mismatch".to_string());
    }
    let Some(process) = VerifiedProcessTermination::open(identity)? else {
        return Ok(());
    };
    process.signal(VerifiedProcessSignal::Terminate)?;
    let polite_deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < polite_deadline && process.is_running()? {
        thread::sleep(Duration::from_millis(25));
    }
    if process.is_running()? {
        process.signal(VerifiedProcessSignal::Kill)?;
    }
    let exit_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < exit_deadline && process.is_running()? {
        thread::sleep(Duration::from_millis(25));
    }
    if process.is_running()? {
        return Err("browser_session_close_exit_unproven".to_string());
    }
    Ok(())
}

fn recorded_browser_process_exists(browser: &ManagedBrowserInstance) -> bool {
    match browser.process_identity.as_ref() {
        Some(identity) => crate::process_identity::VerifiedProcessTermination::open(identity)
            .ok()
            .flatten()
            .is_some_and(|process| process.is_running().unwrap_or(false)),
        None => process_exists(browser.pid),
    }
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

    #[test]
    fn reserved_browser_launch_has_one_exact_causal_process_marker() {
        assert_eq!(
            reserved_browser_launch_args(Some("browser:work:operation:abc123")),
            ["--agent-browser-reservation-id=browser:work:operation:abc123"]
        );
        assert!(reserved_browser_launch_args(None).is_empty());
    }
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::process::Command;

    #[derive(Default)]
    struct FakeRuntime;

    impl BrowserRuntimeDriver for FakeRuntime {
        fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
            Ok(true)
        }

        fn launch_browser(
            &mut self,
            _profile: &BrowserProfileCatalogEntry,
            _desktop: Option<&BrowserDesktopAssignment>,
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
            process_identity: None,
            desktop: None,
            active_session_ids: Vec::new(),
        };
        assert!(recorded_browser_process_exists(&browser));

        browser.pid = u32::MAX;
        assert!(!recorded_browser_process_exists(&browser));
    }

    #[test]
    fn recorded_browser_reattach_has_an_injected_handshake_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("stalled CDP listener");
        let address = listener.local_addr().expect("stalled CDP address");
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().expect("stalled CDP connection");
            thread::sleep(Duration::from_millis(200));
        });
        let browser = ManagedBrowserInstance {
            id: "browser:stalled-reattach:1".to_string(),
            profile_id: "stalled-reattach".to_string(),
            pid: std::process::id(),
            cdp_endpoint: format!("ws://{address}/devtools/browser/stalled"),
            process_identity: None,
            desktop: None,
            active_session_ids: vec!["session:stalled-reattach:1".to_string()],
        };
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let started = Instant::now();

        let error = match runtime.block_on(attach_recorded_browser_with_timeout(
            &browser,
            Duration::from_millis(50),
        )) {
            Ok(_) => panic!("stalled recorded browser unexpectedly reattached"),
            Err(error) => error,
        };

        assert_eq!(error, "browser_session_reattach_timeout");
        assert!(started.elapsed() < Duration::from_millis(500));
        server.join().expect("stalled CDP server");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unresponsive_recorded_browser_uses_exact_process_termination() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("unresponsive CDP listener");
        let address = listener.local_addr().expect("unresponsive CDP address");
        let server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("unresponsive CDP connection");
                let _ = stream.write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        let mut child = Command::new("/usr/bin/sleep")
            .arg("300")
            .spawn()
            .expect("disposable external browser process");
        let process_identity = crate::process_identity::capture_process_identity(
            child.id(),
            Some(Path::new("/usr/bin/sleep")),
            None,
        )
        .expect("external process identity");
        let browser = ManagedBrowserInstance {
            id: "browser:external-unresponsive:1".to_string(),
            profile_id: "external-unresponsive".to_string(),
            pid: child.id(),
            cdp_endpoint: format!("ws://{address}/devtools/browser/unresponsive"),
            process_identity: Some(process_identity),
            desktop: None,
            active_session_ids: vec!["session:external-unresponsive:1".to_string()],
        };
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default())
            .expect("browser manager runtime");

        assert!(!runtime.browser_is_live(&browser).unwrap());
        let close = runtime.close_browser(&browser);
        if close.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        server.join().expect("unresponsive CDP server");
        assert!(close.is_ok(), "unresponsive exact close failed: {close:?}");
        for _ in 0..100 {
            if child.try_wait().unwrap().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!("unresponsive exact close left the recorded process running");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unresponsive_recorded_browser_without_identity_is_preserved() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("unresponsive CDP listener");
        let address = listener.local_addr().expect("unresponsive CDP address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("unresponsive CDP connection");
            let _ = stream.write_all(
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });
        let mut child = Command::new("/usr/bin/sleep")
            .arg("300")
            .spawn()
            .expect("disposable unverified process");
        let browser = ManagedBrowserInstance {
            id: "browser:external-unverified:1".to_string(),
            profile_id: "external-unverified".to_string(),
            pid: child.id(),
            cdp_endpoint: format!("ws://{address}/devtools/browser/unresponsive"),
            process_identity: None,
            desktop: None,
            active_session_ids: vec!["session:external-unverified:1".to_string()],
        };
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default())
            .expect("browser manager runtime");

        let error = runtime.close_browser(&browser).unwrap_err();
        server.join().expect("unresponsive CDP server");
        assert!(error.contains("browser_session_close_process_identity_missing"));
        assert!(child.try_wait().unwrap().is_none());
        child.kill().unwrap();
        child.wait().unwrap();
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
            process_identity: launch.process_identity,
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
        let listener = TcpListener::bind("127.0.0.1:0").expect("header fixture listener");
        let address = listener.local_addr().expect("header fixture address");
        let (request_tx, request_rx) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("header fixture connection");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("header fixture read timeout");
            let mut request = vec![0_u8; 8192];
            let read = stream.read(&mut request).expect("header fixture request");
            request.truncate(read);
            request_tx
                .send(String::from_utf8_lossy(&request).into_owned())
                .expect("header fixture request receipt");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 21\r\nConnection: close\r\n\r\n<title>header</title>",
                )
                .expect("header fixture response");
        });
        let header_navigation = restarted
            .execute_command(
                &browser,
                &managed_tab,
                &managed_tab.session_id,
                "alice",
                &serde_json::json!({
                    "id": "header-navigation-1",
                    "action": "navigate",
                    "url": format!("http://{address}/guacamole/"),
                    "headers": {"Remote-User": "operator"}
                }),
            )
            .unwrap();
        let request = request_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("header fixture observed request");
        server.join().expect("header fixture server");
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
        assert_eq!(header_navigation["success"], true, "{header_navigation}");
        assert!(request
            .to_ascii_lowercase()
            .contains("remote-user: operator"));
        assert_eq!(title["success"], true);
        assert_eq!(title["data"]["title"], "header");
        assert_eq!(snapshot["success"], true);
        close.unwrap();
        assert!(!recorded_browser_process_exists(&browser));
    }

    #[test]
    #[ignore = "launches a disposable local Chrome process"]
    fn named_sessions_keep_independent_command_reference_state() {
        let directory = TempDirectory::new();
        let profile = BrowserProfileCatalogEntry {
            id: "isolation-fixture".to_string(),
            name: "Isolation fixture".to_string(),
            user_data_dir: directory.0.join("profile").to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
        };
        let mut runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
            headless: true,
            ..BrowserManagerRuntimeConfig::default()
        })
        .unwrap();
        let launch = runtime.launch_browser(&profile, None).unwrap();
        let browser = ManagedBrowserInstance {
            id: launch.browser_id,
            profile_id: profile.id,
            pid: launch.pid,
            cdp_endpoint: launch.cdp_endpoint,
            process_identity: launch.process_identity,
            desktop: None,
            active_session_ids: vec![
                "session:alice:isolation:1".to_string(),
                "session:bob:isolation:2".to_string(),
            ],
        };
        let alice_acquisition = runtime.acquire_initial_tab(&browser, &[]).unwrap();
        let alice_tab = ManagedBrowserTab {
            id: alice_acquisition.tab_id,
            target_id: alice_acquisition.target_id,
            browser_id: browser.id.clone(),
            session_id: browser.active_session_ids[0].clone(),
            created_at_ms: 1,
            last_activity_at_ms: 1,
        };
        let bob_acquisition = runtime.create_tab(&browser).unwrap();
        let bob_tab = ManagedBrowserTab {
            id: bob_acquisition.tab_id,
            target_id: bob_acquisition.target_id,
            browser_id: browser.id.clone(),
            session_id: browser.active_session_ids[1].clone(),
            created_at_ms: 2,
            last_activity_at_ms: 2,
        };
        runtime
            .navigate(
                &browser,
                &alice_tab,
                "data:text/html,<button onclick=\"document.title='alice-clicked'\">Alice</button>",
            )
            .unwrap();
        runtime
            .navigate(
                &browser,
                &bob_tab,
                "data:text/html,<button onclick=\"document.title='bob-clicked'\">Bob</button>",
            )
            .unwrap();

        for (tab, name) in [(&alice_tab, "alice"), (&bob_tab, "bob")] {
            let snapshot = runtime
                .execute_command(
                    &browser,
                    tab,
                    &tab.session_id,
                    name,
                    &serde_json::json!({"id": format!("snapshot-{name}"), "action": "snapshot"}),
                )
                .unwrap();
            assert_eq!(snapshot["success"], true, "{snapshot}");
            assert!(snapshot["data"]["snapshot"]
                .as_str()
                .is_some_and(|value| value.contains("ref=e1")));
        }
        let alice_click = runtime
            .execute_command(
                &browser,
                &alice_tab,
                &alice_tab.session_id,
                "alice",
                &serde_json::json!({"id": "click-alice", "action": "click", "selector": "e1"}),
            )
            .unwrap();
        assert_eq!(alice_click["success"], true, "{alice_click}");
        let alice_title = runtime
            .execute_command(
                &browser,
                &alice_tab,
                &alice_tab.session_id,
                "alice",
                &serde_json::json!({"id": "title-alice", "action": "title"}),
            )
            .unwrap();
        assert_eq!(alice_title["data"]["title"], "alice-clicked");

        runtime.close_browser(&browser).unwrap();
    }

    #[test]
    fn disposable_cleanup_is_limited_to_the_recorded_direct_child() {
        let root = TempDirectory::new();
        let policy = BrowserDisposableProfilePolicy {
            id: "default".to_string(),
            user_data_root: root.0.to_string_lossy().into_owned(),
            cleanup_delay_ms: 0,
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
}
