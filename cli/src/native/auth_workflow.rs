#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::common::*;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::auth::wait_for_any_selector;
    use crate::native::browser_wait::wait_for_selector;
    use crate::native::service_diagnostics::truncate_utf8;
    pub(crate) async fn handle_auth_save(cmd: &Value) -> Result<Value, String> {
        let name = cmd
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'name'")?;
        let url = cmd
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url'")?;
        let username = cmd
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'username'")?;
        let password = cmd
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'password'")?;
        let username_selector = cmd.get("usernameSelector").and_then(|v| v.as_str());
        let password_selector = cmd.get("passwordSelector").and_then(|v| v.as_str());
        let submit_selector = cmd.get("submitSelector").and_then(|v| v.as_str());
        auth::auth_save(
            name,
            url,
            username,
            password,
            username_selector,
            password_selector,
            submit_selector,
        )
    }
    pub(crate) async fn handle_auth_login(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let name = cmd
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'name'")?;
        let cred = auth::credentials_get_full(name)?;
        if cred.url.is_empty() {
            return Err("Credential has no URL".to_string());
        }
        let url = cred.url;
        let username = cred.username;
        let password = cred.password;
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        mgr.navigate(&url, AUTH_LOGIN_WAIT_UNTIL).await?;
        let session_id = mgr.active_session_id()?.to_string();
        let auth_timeout_ms = mgr.default_timeout_ms();
        let preferred_user_selectors = [
            "input[type=email]",
            "input[name=email]",
            "input[id=email]",
            "input[autocomplete=email]",
            "input[autocomplete=username]",
            "input[name=username]",
            "input[name*=email i]",
            "input[name*=user i]",
            "input[id*=email i]",
            "input[id*=user i]",
            "input[type=text][name*=email i]",
            "input[type=text][name*=user i]",
            "input[type=text][id*=email i]",
            "input[type=text][id*=user i]",
            "input[type=text][autocomplete=email]",
            "input[type=text][autocomplete=username]",
        ];
        let fallback_user_selectors = ["input[type=text]", "input:not([type])"];
        let auto_submit_selectors = [
            "button[type=submit]",
            "input[type=submit]",
            "button:not([type])",
        ];
        let username_sel = cmd
            .get("usernameSelector")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or(cred.username_selector);
        let password_sel = cmd
            .get("passwordSelector")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or(cred.password_selector);
        let submit_sel = cmd
            .get("submitSelector")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or(cred.submit_selector);
        let user_sel = if let Some(s) = username_sel {
            wait_for_selector(&mgr.client, &session_id, &s, "visible", auth_timeout_ms)
                .await
                .map_err(|_| format!("Timed out waiting for username selector '{}'", s))?;
            s
        } else {
            let preferred_window_ms = auth_timeout_ms.min(AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS);
            let fallback_window_ms = auth_timeout_ms.saturating_sub(preferred_window_ms);
            match wait_for_any_selector(
                &mgr.client,
                &session_id,
                &preferred_user_selectors,
                preferred_window_ms,
            )
            .await
            {
                Ok(selector) => selector,
                Err(_) => {
                    if fallback_window_ms == 0 {
                        return Err(
                            format!(
                                "Timed out waiting for username field (preferred selectors for {}ms: {})",
                                preferred_window_ms, preferred_user_selectors.join(", ")
                            ),
                        );
                    }
                    wait_for_any_selector(
                            &mgr.client,
                            &session_id,
                            &fallback_user_selectors,
                            fallback_window_ms,
                        )
                        .await
                        .map_err(|_| {
                            format!(
                                "Timed out waiting for username field (preferred selectors for {}ms: {}; fallback selectors for {}ms: {})",
                                preferred_window_ms, preferred_user_selectors.join(", "),
                                fallback_window_ms, fallback_user_selectors.join(", ")
                            )
                        })?
                }
            }
        };
        interaction::fill(
            &mgr.client,
            &session_id,
            &state.ref_map,
            &user_sel,
            &username,
            &state.iframe_sessions,
        )
        .await?;
        let pass_sel = password_sel.unwrap_or_else(|| "input[type=password]".to_string());
        wait_for_selector(
            &mgr.client,
            &session_id,
            &pass_sel,
            "visible",
            auth_timeout_ms,
        )
        .await
        .map_err(|_| format!("Timed out waiting for password selector '{}'", pass_sel))?;
        interaction::fill(
            &mgr.client,
            &session_id,
            &state.ref_map,
            &pass_sel,
            &password,
            &state.iframe_sessions,
        )
        .await?;
        let sub_sel = if let Some(s) = submit_sel {
            wait_for_selector(&mgr.client, &session_id, &s, "visible", auth_timeout_ms)
                .await
                .map_err(|_| format!("Timed out waiting for submit selector '{}'", s))?;
            s
        } else {
            wait_for_any_selector(
                &mgr.client,
                &session_id,
                &auto_submit_selectors,
                auth_timeout_ms,
            )
            .await
            .map_err(|_| {
                format!(
                    "Timed out waiting for submit button (tried selectors: {})",
                    auto_submit_selectors.join(", ")
                )
            })?
        };
        interaction::click(
            &mgr.client,
            &session_id,
            &state.ref_map,
            &sub_sel,
            "left",
            1,
            &state.iframe_sessions,
        )
        .await?;
        let mut rx = mgr.client.subscribe();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
        let mut navigated = false;
        loop {
            let result = tokio::time::timeout_at(deadline, rx.recv()).await;
            match result {
                Ok(Ok(event)) => {
                    if event.session_id.as_deref() == Some(&session_id) {
                        match event.method.as_str() {
                            "Page.frameNavigated" | "Page.loadEventFired" => {
                                navigated = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Err(_)) => break,
                Err(_) => break,
            }
        }
        if !navigated {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        Ok(json!({ "loggedIn" : true, "name" : name }))
    }
    pub(crate) struct ConfirmationExecution {
        pub(crate) action: String,
        pub(crate) command: Value,
        pub(crate) policy: Option<ActionPolicy>,
        pub(crate) confirm_actions: Option<ConfirmActions>,
    }
    impl ConfirmationExecution {
        pub(crate) fn command(&self) -> &Value {
            &self.command
        }
        pub(crate) fn complete(self, state: &mut DaemonState, result: Value) -> Value {
            state.policy = self.policy;
            state.confirm_actions = self.confirm_actions;
            json!({ "confirmed" : true, "action" : self.action, "result" : result })
        }
    }
    pub(crate) fn begin_confirmation(
        state: &mut DaemonState,
    ) -> Result<ConfirmationExecution, String> {
        let pending = state
            .pending_confirmation
            .take()
            .ok_or("No pending confirmation")?;
        Ok(ConfirmationExecution {
            action: pending.action,
            command: pending.cmd,
            policy: state.policy.take(),
            confirm_actions: state.confirm_actions.take(),
        })
    }
    pub(crate) async fn handle_deny(
        _cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let pending = state
            .pending_confirmation
            .take()
            .ok_or("No pending confirmation")?;
        Ok(json!({ "denied" : true, "action" : pending.action }))
    }
}
pub(crate) use action_commands::*;
