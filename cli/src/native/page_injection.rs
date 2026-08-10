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
    pub(crate) async fn handle_addscript(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let content = cmd
            .get("content")
            .or_else(|| cmd.get("source"))
            .or_else(|| cmd.get("script"))
            .and_then(|v| v.as_str());
        let url = cmd.get("url").and_then(|v| v.as_str());
        if content.is_none() && url.is_none() {
            return Err("At least one of 'content' or 'url' is required".to_string());
        }
        if let Some(src_url) = url {
            let js = format!(
                r#"new Promise((resolve, reject) => {{
                const s = document.createElement('script');
                s.src = {};
                s.onload = () => resolve(true);
                s.onerror = () => reject(new Error('Failed to load script'));
                document.head.appendChild(s);
            }})"#,
                serde_json::to_string(src_url).unwrap_or_default()
            );
            mgr.evaluate(&js, None).await?;
        } else if let Some(source) = content {
            let js = format!(
                r#"(() => {{
                const s = document.createElement('script');
                s.textContent = {};
                document.head.appendChild(s);
            }})()"#,
                serde_json::to_string(source).unwrap_or_default()
            );
            mgr.evaluate(&js, None).await?;
        }
        Ok(json!({ "added" : true }))
    }
    pub(crate) async fn handle_addinitscript(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let source = cmd
            .get("script")
            .or_else(|| cmd.get("source"))
            .or_else(|| cmd.get("content"))
            .and_then(|v| v.as_str())
            .ok_or("Missing 'script' parameter")?;
        let identifier = mgr.add_script_to_evaluate(source).await?;
        Ok(json!({ "added" : true, "identifier" : identifier }))
    }
    pub(crate) async fn handle_addstyle(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let content = cmd
            .get("content")
            .or_else(|| cmd.get("css"))
            .and_then(|v| v.as_str());
        let url = cmd.get("url").and_then(|v| v.as_str());
        if content.is_none() && url.is_none() {
            return Err("At least one of 'content' or 'url' is required".to_string());
        }
        if let Some(href) = url {
            let js = format!(
                r#"new Promise((resolve, reject) => {{
                const link = document.createElement('link');
                link.rel = 'stylesheet';
                link.href = {};
                link.onload = () => resolve(true);
                link.onerror = () => reject(new Error('Failed to load stylesheet'));
                document.head.appendChild(link);
            }})"#,
                serde_json::to_string(href).unwrap_or_default()
            );
            mgr.evaluate(&js, None).await?;
        } else if let Some(css) = content {
            let js = format!(
                r#"(() => {{
                const style = document.createElement('style');
                style.textContent = {};
                document.head.appendChild(style);
            }})()"#,
                serde_json::to_string(css).unwrap_or_default()
            );
            mgr.evaluate(&js, None).await?;
        }
        Ok(json!({ "added" : true }))
    }
}
pub(crate) use action_commands::*;
