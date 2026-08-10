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
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::service_probe::handle_bounded_service_evaluate;
    pub(crate) async fn handle_url(state: &mut DaemonState) -> Result<Value, String> {
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                let url = wb.get_url().await?;
                return Ok(json!({ "url" : url }));
            }
        }
        let desired_url = state
            .browser
            .as_ref()
            .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
        let first_result = {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            mgr.get_url().await
        };
        let url = match first_result {
            Ok(url) => url,
            Err(err) if is_stale_page_session_error(&err) => {
                let recovered = {
                    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                    recover_browser_command_channel(mgr, &err).await
                };
                if recovered.is_err() {
                    relaunch_and_restore_page(state, desired_url).await?;
                }
                state
                    .browser
                    .as_mut()
                    .ok_or("Browser not launched")?
                    .get_url()
                    .await?
            }
            Err(err) => return Err(err),
        };
        Ok(json!({ "url" : url }))
    }
    pub(crate) fn handle_cdp_url(state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        Ok(json!({ "cdpUrl" : mgr.get_cdp_url() }))
    }
    pub(crate) async fn handle_inspect(state: &mut DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        if let Some(server) = state.inspect_server.take() {
            server.shutdown();
        }
        let target_id = mgr.active_target_id()?.to_string();
        let chrome_hp = mgr.chrome_host_port().to_string();
        let proxy_handle = mgr.client.inspect_handle();
        let server = InspectServer::start(proxy_handle, target_id, chrome_hp).await?;
        let url = format!("http://127.0.0.1:{}", server.port());
        open_url_in_browser(&url);
        state.inspect_server = Some(server);
        Ok(json!({ "opened" : true, "url" : url }))
    }
    pub(crate) fn open_url_in_browser(url: &str) {
        #[cfg(target_os = "macos")]
        let result = std::process::Command::new("open").arg(url).spawn();
        #[cfg(target_os = "linux")]
        let result = std::process::Command::new("xdg-open").arg(url).spawn();
        #[cfg(target_os = "windows")]
        let result = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn();
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let result: Result<std::process::Child, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "unsupported platform",
        ));
        if let Err(e) = result {
            let _ = writeln!(std::io::stderr(), "[inspect] Failed to open browser: {}", e);
        }
    }
    pub(crate) async fn handle_title(state: &mut DaemonState) -> Result<Value, String> {
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                let title = wb.get_title().await?;
                return Ok(json!({ "title" : title }));
            }
        }
        let desired_url = state
            .browser
            .as_ref()
            .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
        let first_result = {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            mgr.get_title().await
        };
        let title = match first_result {
            Ok(title) => title,
            Err(err) if is_stale_page_session_error(&err) => {
                let recovered = {
                    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                    recover_browser_command_channel(mgr, &err).await
                };
                if recovered.is_err() {
                    relaunch_and_restore_page(state, desired_url).await?;
                }
                state
                    .browser
                    .as_mut()
                    .ok_or("Browser not launched")?
                    .get_title()
                    .await?
            }
            Err(err) => return Err(err),
        };
        Ok(json!({ "title" : title }))
    }
    pub(crate) async fn handle_content(state: &mut DaemonState) -> Result<Value, String> {
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                let html = wb.get_content().await?;
                let url = wb.get_url().await.unwrap_or_default();
                return Ok(json!({ "html" : html, "origin" : url }));
            }
        }
        let desired_url = state
            .browser
            .as_ref()
            .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
        let first_result = {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            mgr.get_content().await
        };
        let html = match first_result {
            Ok(html) => html,
            Err(err) if is_stale_page_session_error(&err) => {
                let recovered = {
                    let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                    recover_browser_command_channel(mgr, &err).await
                };
                if recovered.is_err() {
                    relaunch_and_restore_page(state, desired_url).await?;
                }
                state
                    .browser
                    .as_mut()
                    .ok_or("Browser not launched")?
                    .get_content()
                    .await?
            }
            Err(err) => return Err(err),
        };
        let url = state
            .browser
            .as_mut()
            .ok_or("Browser not launched")?
            .get_url()
            .await
            .unwrap_or_default();
        Ok(json!({ "html" : html, "origin" : url }))
    }
    pub(crate) async fn handle_setcontent(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let html = cmd
            .get("html")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'html' parameter")?;
        network::set_content(&mgr.client, &session_id, html).await?;
        Ok(json!({ "set" : true }))
    }
    pub(crate) async fn handle_console(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let clear = cmd.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
        if clear {
            state.event_tracker.clear_console();
            Ok(json!({ "cleared" : true }))
        } else {
            let result = state.event_tracker.get_console_json();
            Ok(result)
        }
    }
    pub(crate) async fn handle_errors(state: &DaemonState) -> Result<Value, String> {
        Ok(state.event_tracker.get_errors_json())
    }
    pub(crate) async fn handle_styles(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let properties = cmd.get("properties").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let styles = super::super::element::get_element_styles(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            properties,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "styles" : styles }))
    }

    pub(crate) fn command_evaluation_timeout_ms(cmd: &Value) -> Option<u64> {
        cmd.get("jobTimeoutMs")
            .and_then(Value::as_u64)
            .filter(|timeout_ms| *timeout_ms > 0)
    }

    pub(crate) async fn handle_evaluate(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        if cmd.get("serviceTabHandle").is_some() {
            return handle_bounded_service_evaluate(cmd, state).await;
        }
        if let Some(ref webdriver) = state.webdriver_backend {
            if state.browser.is_none() {
                let script = cmd
                    .get("script")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'script' parameter")?;
                let result = webdriver.evaluate(script).await?;
                let url = webdriver.get_url().await.unwrap_or_default();
                return Ok(json!({ "result": result, "origin": url }));
            }
        }
        let manager = state.browser.as_ref().ok_or("Browser not launched")?;
        let script = cmd
            .get("script")
            .and_then(Value::as_str)
            .ok_or("Missing 'script' parameter")?;
        let result = if let Some(timeout_ms) = command_evaluation_timeout_ms(cmd) {
            manager.evaluate_with_timeout(script, timeout_ms).await?
        } else {
            manager.evaluate(script, None).await?
        };
        let url = manager.active_page_url().unwrap_or_default().to_string();
        Ok(json!({ "result": result, "origin": url }))
    }
}
pub(crate) use action_commands::*;
