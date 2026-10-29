//! Managed CDP-free windows. Native control belongs to the Remote View owner.
use super::*;
use agent_browser_service_model::{BrowserLaunchCustodyStore, RecordedProcessIdentity};
use serde_json::json;

#[derive(Default)]
pub(super) struct ManualBrowsers {
    owned: HashMap<
        String,
        (
            super::super::cdp::chrome::ChromeProcess,
            RecordedProcessIdentity,
        ),
    >,
}

impl ManualBrowsers {
    #[cfg(target_os = "linux")]
    fn display(&mut self, browser: &ManagedBrowserInstance) -> Result<String, String> {
        // The exact owned child retains the provider's trusted launch display.
        // Sandboxed Chrome may make environ unreadable after launch. Adopted
        // paths without that owned child must still establish current context.
        let owned = self
            .owned
            .get(&browser.id)
            .map(|(child, _)| child.display_name().map(str::to_owned));
        resolve_display(
            owned.as_ref().map(|value| value.as_deref()),
            || self.verify(browser),
            || process_display(browser),
        )
    }
    /// Reap only retained owned children during idle runtime polling. A zombie
    /// must not survive because no later task command happens to verify it.
    pub(super) fn poll_owned_exits(&mut self) {
        for (child, _) in self.owned.values_mut() {
            let _ = child.poll_exit();
        }
    }

    pub(super) fn launch(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<RemoteViewFixedDesktop>,
        options: LaunchOptions,
    ) -> Result<BrowserLaunch, String> {
        if desktop.is_none() {
            return Err("manual_login_requires_native_remote_desktop".into());
        }
        let mut process = super::super::cdp::chrome::launch_chrome_manual_owned(&options)?;
        let identity = crate::process_identity::capture_launched_browser_identity(
            process.id(),
            Path::new(
                options
                    .executable_path
                    .as_deref()
                    .ok_or("manual_login_executable_missing")?,
            ),
            None,
        )
        .ok_or("manual_login_process_identity_unavailable")?;
        if process.poll_exit().is_some() {
            return Err("manual_login_process_exited".into());
        }
        process.mark_lifecycle_managed(None);
        let id = format!("browser:{}:{}", profile.id, uuid::Uuid::new_v4());
        let launch = BrowserLaunch {
            process_identity: Some(identity.clone()),
            browser_id: id.clone(),
            pid: process.id(),
            cdp_endpoint: String::new(),
            desktop,
        };
        self.owned.insert(id, (process, identity));
        Ok(launch)
    }

    pub(super) fn retain_logical_identity(
        &mut self,
        physical_id: &str,
        logical_id: &str,
    ) -> Result<(), String> {
        let child = self
            .owned
            .remove(physical_id)
            .ok_or("mode_operation_launched_child_missing")?;
        self.owned.insert(logical_id.into(), child);
        Ok(())
    }

    /// Verify the immutable original process before reads, focus or close. A
    /// missing identity never becomes permission to attach or replace a browser.
    pub(super) fn verify(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        let original = if let Some((process, identity)) = self.owned.get_mut(&browser.id) {
            if process.poll_exit().is_some() {
                return Ok(false);
            }
            identity.clone()
        } else {
            let path = super::super::browser_session_store::BrowserSessionSqliteStore::default_sqlite_path()?;
            let mut store =
                super::super::browser_session_store::BrowserSessionSqliteStore::open_read_only(
                    &path,
                )?;
            match store.published_launch_process_identity(browser) {
                Ok(Some(identity)) if identity.pid == browser.pid => identity,
                _ => {
                    let records = store
                        .unpublished_launch_records()
                        .map_err(|_| "manual_login_original_identity_unavailable")?;
                    let matches = records
                        .into_iter()
                        .filter(|record| {
                            record.intent.profile_id == browser.profile_id
                                && record.observed_browser_id.as_deref()
                                    == Some(browser.id.as_str())
                                && record.observed_pid == Some(browser.pid)
                                && browser.desktop.as_ref().is_some_and(|desktop| {
                                    desktop.desktop_id == record.intent.assignment.desktop_id
                                        && desktop.generation == record.intent.assignment.generation
                                })
                        })
                        .collect::<Vec<_>>();
                    let [record] = matches.as_slice() else {
                        return Err("manual_login_original_identity_missing".into());
                    };
                    record
                        .observed_process_identity
                        .clone()
                        .ok_or("manual_login_original_identity_missing")?
                }
            }
        };
        match crate::process_identity::observe_process(browser.pid) {
            crate::process_identity::ProcessObservation::Missing => Ok(false),
            crate::process_identity::ProcessObservation::Observed(current)
                if current.pid == original.pid
                    && current.start_token.as_deref() == Some(original.start_token.as_str())
                    && current.executable_path == original.executable_path =>
            {
                let args = current
                    .command_line
                    .ok_or("manual_login_launch_proof_unavailable")?;
                if args
                    .iter()
                    .flat_map(|arg| arg.split_whitespace())
                    .any(|arg| arg.starts_with("--remote-debugging"))
                {
                    return Err("manual_login_debugging_listener_forbidden".into());
                }
                Ok(true)
            }
            _ => Err("manual_login_original_process_changed".into()),
        }
    }

    pub(super) fn focus(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        if !self.verify(browser)? {
            return Err("manual_login_process_exited".into());
        }
        #[cfg(target_os = "linux")]
        {
            // Read only this verified process's launch display. Remote View then
            // independently verifies the assignment and foreground window.
            let display = self.display(browser)?;
            if !self.verify(browser)? {
                return Err("manual_login_process_exited".into());
            }
            let until = Instant::now() + Duration::from_secs(5);
            loop {
                match super::super::browser::focus_viewable_x11_browser_window(
                    browser.pid,
                    &display,
                ) {
                    Ok(_) => return Ok(()),
                    Err(reason)
                        if reason.starts_with("No X11 window found") && Instant::now() < until =>
                    {
                        if !self.verify(browser)? {
                            return Err("manual_login_process_exited".into());
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => return Err("manual_login_native_window_unavailable".into()),
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        Err("manual_login_native_focus_unsupported".into())
    }

    pub(super) fn close(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        if !self.verify(browser)? {
            self.owned.remove(&browser.id);
            return Ok(());
        }
        #[cfg(target_os = "linux")]
        {
            let display = self.display(browser)?;
            if !self.verify(browser)? {
                self.owned.remove(&browser.id);
                return Ok(());
            }
            // SIGTERM can discard buffered cookies. WM_DELETE_WINDOW follows
            // normal Chrome shutdown, without CDP or a forced-close fallback.
            super::super::browser::close_x11_browser_windows(browser.pid, &display)
                .map_err(|_| "manual_login_native_close_unavailable")?;
            let until = Instant::now() + Duration::from_secs(5);
            if let Some((mut process, identity)) = self.owned.remove(&browser.id) {
                loop {
                    if let Some(exit) = process.poll_exit() {
                        if exit.poll_error.is_none() {
                            process.relinquish_for_handoff();
                            return Ok(());
                        }
                        self.owned.insert(browser.id.clone(), (process, identity));
                        return Err("manual_login_close_readback_required".into());
                    }
                    if Instant::now() >= until {
                        self.owned.insert(browser.id.clone(), (process, identity));
                        return Err("manual_login_close_readback_required: finish any browser close prompt, then close this task again".into());
                    }
                    thread::sleep(Duration::from_millis(20));
                }
            }
            loop {
                if !self.verify(browser)? {
                    return Ok(());
                }
                if Instant::now() >= until {
                    return Err("manual_login_close_readback_required".into());
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        #[cfg(not(target_os = "linux"))]
        Err("manual_login_close_unsupported".into())
    }

    pub(super) fn execute(
        &mut self,
        browser: &ManagedBrowserInstance,
        command: &Value,
    ) -> Result<Value, String> {
        if !self.verify(browser)? {
            return Err("manual_login_process_exited".into());
        }
        use super::super::browser_session_host::BrowserSessionPersistence;
        let path =
            super::super::browser_session_store::BrowserSessionSqliteStore::default_sqlite_path()?;
        let catalog =
            super::super::browser_session_store::BrowserSessionSqliteStore::open_read_only(&path)?
                .load_or_import_profile_catalog(Path::new(""))?
                .catalog;
        let requested = catalog
            .profiles
            .get(&browser.profile_id)
            .and_then(|profile| profile.manual_login_url.as_deref())
            .ok_or("manual_login_profile_mode_missing")?;
        let data = match command["action"].as_str() {
            Some("url") => json!({"url":requested,"urlObserved":false,"mode":"manual_login","authentication":"not_verified"}),
            Some("navigate") if command["url"] == requested =>
                json!({"url":requested,"urlObserved":false,"mode":"manual_login","authentication":"not_verified"}),
            _ => return Err("manual_login_automation_unavailable: use native_capture/native_input for visual control, or close and reopen this preserved profile for DOM automation".into()),
        };
        Ok(json!({"id":command["id"],"success":true,"data":data}))
    }
}

#[cfg(target_os = "linux")]
fn resolve_display(
    owned: Option<Option<&str>>,
    verify: impl FnOnce() -> Result<bool, String>,
    read: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    if !verify()? {
        return Err("manual_login_process_exited".into());
    }
    match owned {
        Some(Some(display))
            if !display.is_empty()
                && !display.chars().any(char::is_whitespace)
                && !display.chars().any(char::is_control) =>
        {
            Ok(display.into())
        }
        Some(_) => Err("manual_login_owned_display_unavailable".into()),
        None => read(),
    }
}

#[cfg(target_os = "linux")]
fn process_display(browser: &ManagedBrowserInstance) -> Result<String, String> {
    let env = fs::read(format!("/proc/{}/environ", browser.pid))
        .map_err(|_| "manual_login_display_unavailable")?;
    env.split(|byte| *byte == 0)
        .find_map(|entry| entry.strip_prefix(b"DISPLAY="))
        .and_then(|value| std::str::from_utf8(value).ok())
        .map(str::to_owned)
        .ok_or_else(|| "manual_login_display_unavailable".into())
}

pub(super) fn is_manual(browser: &ManagedBrowserInstance) -> bool {
    browser.cdp_endpoint.is_empty()
}

pub(super) fn window_tab(browser: &ManagedBrowserInstance) -> BrowserTabAcquisition {
    BrowserTabAcquisition {
        tab_id: uuid::Uuid::new_v4().to_string(),
        target_id: format!("manual-window:{}", browser.id),
        source: BrowserTabSource::Bootstrap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn owned_manual_display_survives_unreadable_process_environment() {
        let result = resolve_display(
            Some(Some(":6850")),
            || Ok(true),
            || Err("environ_unreadable".into()),
        );
        assert_eq!(result.unwrap(), ":6850");
        for owned in [Some(Some(":6850")), None] {
            assert_eq!(
                resolve_display(
                    owned,
                    || Err("original_context_changed".into()),
                    || panic!("must not borrow display")
                )
                .unwrap_err(),
                "original_context_changed"
            );
        }
        assert!(resolve_display(Some(None), || Ok(true), || Ok(":9999".into())).is_err());
    }

    #[test]
    fn manual_login_guard_rejects_debugging_arguments_before_spawn() {
        let options = LaunchOptions {
            headless: false,
            manual_login: true,
            args: vec!["--remote-debugging-pipe".into()],
            ..LaunchOptions::default()
        };
        assert_eq!(
            super::super::super::cdp::chrome::launch_chrome_manual_owned(&options)
                .err()
                .unwrap(),
            "manual_login_debugging_argument_forbidden"
        );
    }
}

/// Rejoin an observed, unpublished mode replacement without opening debugging.
pub(super) fn reconcile_mode(
    profile: &BrowserProfileCatalogEntry,
    browser: &ManagedBrowserInstance,
    tabs: &[ManagedBrowserTab],
) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
    let path =
        super::super::browser_session_store::BrowserSessionSqliteStore::default_sqlite_path()?;
    let mut store =
        super::super::browser_session_store::BrowserSessionSqliteStore::open_read_only(&path)?;
    let records = store
        .unpublished_launch_records()
        .map_err(|_| "mode_operation_launch_readback_required")?;
    let candidates = records
        .into_iter()
        .filter(|record| {
            record.intent.profile_id == profile.id
                && record.observed_browser_id.as_deref() == Some(&browser.id)
                && record.observed_process_identity.is_some()
        })
        .collect::<Vec<_>>();
    let [record] = candidates.as_slice() else {
        return if candidates.is_empty() {
            Ok(None)
        } else {
            Err("mode_operation_launch_ambiguous".into())
        };
    };
    let original = record
        .observed_process_identity
        .as_ref()
        .ok_or("mode_operation_launch_identity_missing")?;
    let current = match crate::process_identity::observe_process(original.pid) {
        crate::process_identity::ProcessObservation::Observed(current) => current,
        _ => return Ok(None),
    };
    if current.start_token.as_deref() != Some(original.start_token.as_str())
        || current.executable_path != original.executable_path
        || current
            .command_line
            .as_ref()
            .is_none_or(|args| args.iter().any(|arg| arg.starts_with("--remote-debugging")))
    {
        return Err("mode_operation_launch_posture_changed".into());
    }
    Ok(Some(agent_browser_service_model::BrowserRecovery {
        launch: BrowserLaunch {
            process_identity: Some(original.clone()),
            browser_id: browser.id.clone(),
            pid: original.pid,
            cdp_endpoint: String::new(),
            desktop: browser.desktop.clone(),
        },
        target_ids: tabs
            .iter()
            .map(|tab| {
                (
                    tab.id.clone(),
                    format!("native-context:{}:{}", browser.id, tab.id),
                )
            })
            .collect(),
    }))
}

/// Rejoin one positively observed first launch after owner publication loss.
/// Unobserved custody and ambiguous profiles remain unresolved, never relaunched.
pub(super) fn reconcile_initial(
    profile: &BrowserProfileCatalogEntry,
) -> Result<Option<agent_browser_service_model::BrowserRecovery>, String> {
    let path =
        super::super::browser_session_store::BrowserSessionSqliteStore::default_sqlite_path()?;
    let mut store =
        super::super::browser_session_store::BrowserSessionSqliteStore::open_read_only(&path)?;
    let records = store
        .unpublished_launch_records()
        .map_err(|_| "manual_login_launch_readback_required")?;
    let candidates = records
        .into_iter()
        .filter(|record| record.intent.profile_id == profile.id)
        .collect::<Vec<_>>();
    let [record] = candidates.as_slice() else {
        return if candidates.is_empty() {
            Ok(None)
        } else {
            Err("manual_login_launch_ambiguous".into())
        };
    };
    let (Some(original), Some(browser_id)) = (
        &record.observed_process_identity,
        &record.observed_browser_id,
    ) else {
        return Err("manual_login_launch_readback_required".into());
    };
    #[cfg(target_os = "linux")]
    {
        let Some(current) =
            super::super::browser_session_profile_reuse::current_profile_browser_identity(
                Path::new(&profile.user_data_dir),
            )?
        else {
            return Err("manual_login_launch_readback_required".into());
        };
        if current.pid != original.pid
            || current.start_token.as_deref() != Some(original.start_token.as_str())
            || current.executable_path != original.executable_path
            || current
                .command_line
                .as_ref()
                .is_none_or(|args| args.iter().any(|arg| arg.starts_with("--remote-debugging")))
        {
            return Err("manual_login_launch_posture_changed".into());
        }
        Ok(Some(agent_browser_service_model::BrowserRecovery {
            launch: BrowserLaunch {
                process_identity: Some(original.clone()),
                browser_id: browser_id.clone(),
                pid: original.pid,
                cdp_endpoint: String::new(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: record.intent.assignment.desktop_id.clone(),
                    generation: record.intent.assignment.generation,
                    friendly_route_label: "Recovered destination".into(),
                }),
            },
            target_ids: Default::default(),
        }))
    }
    #[cfg(not(target_os = "linux"))]
    Err("manual_login_launch_reconciliation_unsupported".into())
}
