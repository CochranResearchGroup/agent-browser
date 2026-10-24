//! Native task control borrows the existing Remote View desktop owner.
use super::*;
use agent_browser_service_model::{BrowserLaunchCustodyStore, RemoteViewTabHandoffTarget};
use serde_json::{json, Value};

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects> BrowserSessionHost<P, E> {
    /// Resolve current process and viewing assignment without attaching CDP or
    /// acquiring another task. Remote View fences each window effect itself.
    pub(crate) fn native_browser_command(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        command: &Value,
    ) -> Result<Value, String> {
        if self.persistence.load_session_state()? != self.persisted_state
            || self.state.browsers.get(&target.browser.id) != Some(&target.browser)
            || self.state.sessions.get(&target.session.id) != Some(&target.session)
            || self.state.tabs.get(&target.tab.id) != Some(&target.tab)
        {
            return Err("browser_native_target_changed:refresh task".into());
        }
        let original = if let Some(attachment) = self.state.app_browser_attachments.get(&target.browser.id) {
            attachment.request.process.clone()
        } else {
            BrowserSessionSqliteStore::open_read_only(&self.persistence.runtime_database_path()?)?
                .published_launch_process_identity(&target.browser)
                .map_err(|_| "browser_native_original_identity_unavailable")?
                .ok_or("browser_native_original_identity_missing")?
        };
        self.effects.begin_operation(&self.persisted_state)?;
        let assignment = self.effects.native_handoff_assignment_target(target)?;
        let origin = std::env::var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")
            .map_err(|_| "browser_native_owner_unconfigured")?;
        let http = super::super::remote_view_application_http::RemoteViewApplicationHttp::new(
            &origin, std::time::Duration::from_secs(10),
        ).map_err(|_| "browser_native_owner_configuration_invalid")?;
        struct Owner<'a, P> { http: super::super::remote_view_application_http::RemoteViewApplicationHttp, persistence: &'a P, expected: &'a BrowserSessionState }
        impl<P: BrowserSessionPersistence> super::super::browser_native_control::NativeOwner for Owner<'_, P> {
            fn request(&mut self, operation: &str, body: &Value) -> Result<Value, String> {
                if self.persistence.load_session_state()? != *self.expected {
                    return Err("browser_native_target_changed:refresh task".into());
                }
                self.http.native_request(operation, body)
                    .map_err(|error| format!("browser_native_owner_request:{error:?}:observe before retry; input may have been delivered"))
            }
        }
        let mut owner = Owner { http, persistence: &self.persistence, expected: &self.persisted_state };
        super::super::browser_native_control::execute(&original, &assignment, command, &mut owner)
    }

    pub(super) fn execute_native_task(&mut self, session_name:&str, command:&Value) -> Result<Option<Value>,String> {
        let id = self.exact_task_selector(session_name, command)?
            .ok_or("browser_native_exact_task_required")?;
        let session = self.state.sessions.get(id).cloned().ok_or("browser_native_task_missing")?;
        let tab = session.current_tab_id.as_ref().and_then(|id| self.state.tabs.get(id)).cloned().ok_or("browser_native_tab_missing")?;
        let browser = self.state.browsers.get(&session.browser_id).cloned().ok_or("browser_native_browser_missing")?;
        let target = RemoteViewTabHandoffTarget {session,tab,browser};
        let data = self.native_browser_command(&target, command)?;
        Ok(Some(json!({"success":true,"data":data,"browserSession":{"sessionId":target.session.id,"browserId":target.browser.id,"profileId":target.session.profile_id,"tabId":target.tab.id}})))
    }
}
