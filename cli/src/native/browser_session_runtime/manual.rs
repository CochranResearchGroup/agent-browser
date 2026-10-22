//! Human-only managed windows. No CDP client or debugging endpoint is created.
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
            super::super::browser_session_store::BrowserSessionSqliteStore::open_read_only(&path)?
                .published_launch_process_identity(browser)
                .map_err(|_| "manual_login_original_identity_unavailable")?
                .ok_or("manual_login_original_identity_missing")?
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
            let env = fs::read(format!("/proc/{}/environ", browser.pid))
                .map_err(|_| "manual_login_display_unavailable")?;
            let display = env
                .split(|byte| *byte == 0)
                .find_map(|entry| entry.strip_prefix(b"DISPLAY="))
                .and_then(|value| std::str::from_utf8(value).ok())
                .ok_or("manual_login_display_unavailable")?;
            if !self.verify(browser)? {
                return Err("manual_login_process_exited".into());
            }
            let until = Instant::now() + Duration::from_secs(5);
            loop {
                match super::super::browser::focus_x11_browser_window(browser.pid, display) {
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
        // Request a normal Chrome shutdown before the bounded owned-process
        // wait, so sign-in cookies and profile storage can be flushed.
        #[cfg(unix)]
        if self.owned.contains_key(&browser.id)
            && unsafe { libc::kill(browser.pid as i32, libc::SIGTERM) } != 0
        {
            return Err("manual_login_close_failed".into());
        }
        if let Some((mut process, identity)) = self.owned.remove(&browser.id) {
            process.approve_lifecycle_close();
            let outcome = process.wait_or_kill(Duration::from_secs(5));
            if !outcome.exact_process_exited {
                process.mark_lifecycle_managed(None);
                self.owned.insert(browser.id.clone(), (process, identity));
                return Err("manual_login_close_readback_required".into());
            }
            // The exact child was reaped. Do not repeat a group signal from
            // Drop after its numeric PID has become available for reuse.
            process.relinquish_for_handoff();
            return Ok(());
        }
        #[cfg(unix)]
        {
            // The exact original kernel identity was checked above. Never signal
            // a PID taken only from a stale runtime/profile-lock record.
            if unsafe { libc::kill(browser.pid as i32, libc::SIGTERM) } != 0 {
                return Err("manual_login_close_failed".into());
            }
            let until = Instant::now() + Duration::from_secs(5);
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
        #[cfg(not(unix))]
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
            _ => return Err("manual_login_automation_unavailable: finish login, close this task, then reopen the preserved profile for automation".into()),
        };
        Ok(json!({"id":command["id"],"success":true,"data":data}))
    }
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
