//! BrowserManager-backed effects for the ordinary Browser Session Manager.

use std::collections::HashMap;
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
    IsLive {
        browser: ManagedBrowserInstance,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Launch {
        profile: BrowserProfileCatalogEntry,
        desktop: Option<RemoteViewFixedDesktop>,
        environment: Option<std::collections::BTreeMap<String, String>>,
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
                reply,
            },
        )
        .map_err(|_| RemoteViewBrowserProcessError::OutcomeUnknown)
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
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        let (reply, receiver) = mpsc::channel();
        self.request(
            receiver,
            BrowserRuntimeCommand::Launch {
                profile: profile.clone(),
                desktop: desktop.cloned(),
                environment: None,
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
    let mut browsers = HashMap::<String, DaemonState>::new();
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
                environment,
                reply,
            } => {
                let result = runtime.block_on(async {
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
                    super::browser_launch_admission::observe_browser_launch_admission(
                        Path::new(&profile.user_data_dir),
                        config.maximum_browser_processes,
                    )
                    .require_admitted()?;
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
                    let browser_id = format!("browser:{}:{next_browser_sequence}", profile.id);
                    let launch = BrowserLaunch {
                        browser_id: browser_id.clone(),
                        pid,
                        cdp_endpoint: manager.get_cdp_url().to_string(),
                        desktop,
                    };
                    let mut state = DaemonState::new();
                    state.browser = Some(manager);
                    state.browser_session_manager_owned = true;
                    browsers.insert(browser_id, state);
                    Ok(launch)
                });
                let _ = reply.send(result);
            }
            BrowserRuntimeCommand::Close { browser, reply } => {
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
                attributed_target_ids,
                reply,
            } => {
                let result = match browsers.get_mut(&browser.id) {
                    Some(state) => runtime.block_on(async {
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
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
                        let manager = state
                            .browser
                            .as_mut()
                            .ok_or_else(|| "browser_session_runtime_browser_missing".to_string())?;
                        manager
                            .tab_close_target_id_for_release(&tab.target_id)
                            .await?;
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
                        manager.tab_switch_target_id(&tab.target_id).await?;
                        state.session_id = session_id;
                        state.session_name = Some(session_name);
                        Ok(super::actions::execute_command(&command, state).await)
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

async fn attach_recorded_browser(
    browser: &ManagedBrowserInstance,
) -> Result<BrowserManager, String> {
    if !recorded_browser_process_exists(browser) {
        return Err("browser_session_recorded_process_missing".to_string());
    }
    let manager = BrowserManager::connect_cdp(&browser.cdp_endpoint)
        .await
        .map_err(|error| format!("browser_session_reattach_failed:{error}"))?;
    if !manager.is_connection_alive().await {
        return Err("browser_session_reattach_unresponsive".to_string());
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
