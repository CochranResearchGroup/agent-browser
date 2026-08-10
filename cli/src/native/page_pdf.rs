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
    pub(crate) async fn handle_pdf(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let params = json!(
            { "printBackground" : cmd.get("printBackground").and_then(| v | v.as_bool())
            .unwrap_or(true), "landscape" : cmd.get("landscape").and_then(| v | v
            .as_bool()).unwrap_or(false), "preferCSSPageSize" : cmd
            .get("preferCSSPageSize").and_then(| v | v.as_bool()).unwrap_or(false), }
        );
        let result = mgr
            .client
            .send_command("Page.printToPDF", Some(params), Some(&session_id))
            .await?;
        let data = result
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or("No PDF data returned")?;
        let path = cmd.get("path").and_then(|v| v.as_str());
        let save_path = match path {
            Some(p) => p.to_string(),
            None => {
                let dir = dirs::home_dir()
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".agent-browser")
                    .join("tmp")
                    .join("pdfs");
                let _ = std::fs::create_dir_all(&dir);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                dir.join(format!("page-{}.pdf", timestamp))
                    .to_string_lossy()
                    .to_string()
            }
        };
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
            .map_err(|e| format!("Failed to decode PDF: {}", e))?;
        std::fs::write(&save_path, &bytes).map_err(|e| format!("Failed to save PDF: {}", e))?;
        Ok(json!({ "path" : save_path }))
    }
}
pub(crate) use action_commands::*;
