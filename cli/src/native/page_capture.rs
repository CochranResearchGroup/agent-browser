#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::cancellation::cancellable;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::screenshot::{self, ScreenshotOptions};
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::snapshot::{self, SnapshotOptions};
    use crate::native::state;
    use crate::native::webdriver::backend::BrowserBackend;
    use serde_json::{json, Map, Value};
    use std::env;
    use std::fs;
    pub(crate) async fn handle_screenshot(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let cancellation = state.current_cancellation.clone();
        let annotate = cmd
            .get("annotate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                if annotate {
                    return Err(
                        "Annotated screenshots are not yet implemented on the WebDriver backend"
                            .to_string(),
                    );
                }
                let base64_data = cancellable(wb.screenshot(), cancellation.clone()).await?;
                let path = cmd.get("path").and_then(|v| v.as_str());
                if let Some(p) = path {
                    let bytes = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        &base64_data,
                    )
                    .map_err(|e| format!("Base64 decode error: {}", e))?;
                    std::fs::write(p, bytes)
                        .map_err(|e| format!("Failed to write screenshot: {}", e))?;
                    return Ok(json!({ "path" : p }));
                }
                let tmp = format!(
                    "/tmp/screenshot-{}.png",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                );
                let bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &base64_data,
                )
                .map_err(|e| format!("Base64 decode error: {}", e))?;
                std::fs::write(&tmp, bytes)
                    .map_err(|e| format!("Failed to write screenshot: {}", e))?;
                return Ok(json!({ "path" : tmp }));
            }
        }
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let format = cmd
            .get("format")
            .or_else(|| cmd.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("png")
            .to_string();
        let options = ScreenshotOptions {
            selector: cmd
                .get("selector")
                .and_then(|v| v.as_str())
                .map(String::from),
            path: cmd.get("path").and_then(|v| v.as_str()).map(String::from),
            full_page: cmd
                .get("fullPage")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            format,
            quality: cmd
                .get("quality")
                .and_then(|v| v.as_i64())
                .map(|q| q as i32),
            annotate,
            output_dir: cmd
                .get("screenshotDir")
                .and_then(|v| v.as_str())
                .map(String::from),
        };
        if annotate {
            state.ref_map.clear();
            let _ = cancellable(
                snapshot::take_snapshot(
                    &mgr.client,
                    &session_id,
                    &SnapshotOptions {
                        interactive: true,
                        ..SnapshotOptions::default()
                    },
                    &mut state.ref_map,
                    state.active_frame_id.as_deref(),
                    &state.iframe_sessions,
                ),
                cancellation.clone(),
            )
            .await?;
        }
        let result = match cancellable(
            screenshot::take_screenshot(
                &mgr.client,
                &session_id,
                &state.ref_map,
                &options,
                &state.iframe_sessions,
            ),
            cancellation.clone(),
        )
        .await
        {
            Ok(result) => result,
            Err(err)
                if err.contains("CDP response channel closed")
                    || err.contains("Trying to work with closed connection")
                    || err.contains("Session with given id not found")
                    || err.contains("No session with given id") =>
            {
                let desired_url = state
                    .browser
                    .as_ref()
                    .and_then(|mgr| mgr.active_page_url().map(|s| s.to_string()));
                let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                if recover_browser_command_channel(mgr, &err).await.is_err() {
                    relaunch_and_restore_page(state, desired_url).await?;
                }
                let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
                let fresh_session_id = mgr.active_session_id()?.to_string();
                cancellable(
                    screenshot::take_screenshot(
                        &mgr.client,
                        &fresh_session_id,
                        &state.ref_map,
                        &options,
                        &state.iframe_sessions,
                    ),
                    cancellation,
                )
                .await?
            }
            Err(err) => return Err(err),
        };
        let mut response = json!({ "path" : result.path });
        if !result.annotations.is_empty() {
            response["annotations"] = serde_json::to_value(&result.annotations)
                .map_err(|e| format!("Failed to serialize annotations: {}", e))?;
        }
        Ok(response)
    }

    pub(crate) async fn handle_pdf(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let params = json!({
            "printBackground": cmd
                .get("printBackground")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            "landscape": cmd
                .get("landscape")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "preferCSSPageSize": cmd
                .get("preferCSSPageSize")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        });
        let result = mgr
            .client
            .send_command("Page.printToPDF", Some(params), Some(&session_id))
            .await?;
        let data = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or("No PDF data returned")?;
        let save_path = match cmd.get("path").and_then(Value::as_str) {
            Some(path) => path.to_string(),
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
                dir.join(format!("page-{timestamp}.pdf"))
                    .to_string_lossy()
                    .to_string()
            }
        };
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
            .map_err(|error| format!("Failed to decode PDF: {error}"))?;
        std::fs::write(&save_path, &bytes)
            .map_err(|error| format!("Failed to save PDF: {error}"))?;
        Ok(json!({ "path": save_path }))
    }
}
pub(crate) use action_commands::*;
