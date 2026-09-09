#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::interaction;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use serde_json::{json, Map, Value};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};
    use tokio::sync::{broadcast, oneshot, RwLock};
    pub(crate) async fn handle_download(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let path_str = cmd
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let raw_dest = if std::path::Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
                .join(path_str)
        };
        let download_dir = raw_dest
            .parent()
            .ok_or("Invalid download path: no parent directory")?
            .to_path_buf();
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
        let download_dir = download_dir
            .canonicalize()
            .map_err(|e| format!("Failed to resolve download directory: {}", e))?;
        let dest = download_dir.join(
            raw_dest
                .file_name()
                .ok_or("Invalid download path: no filename")?,
        );
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let pid = mgr
            .browser_pid()
            .or(state.attached_browser_pid)
            .ok_or("download_source_identity_unproven: local browser PID unavailable")?;
        let owner = crate::process_identity::capture_process_identity(pid, None, None)
            .ok_or("download_source_identity_unproven: local browser identity unavailable")?;
        let binding = state
            .runtime_owner_binding
            .as_ref()
            .ok_or("download_source_identity_unproven: current owner binding unavailable")?;
        if crate::native::runtime_lifecycle::digest_json(&owner)?
            != binding.claim.process_instance_digest
        {
            return Err(
                "download_source_identity_unproven: process differs from current owner".into(),
            );
        }
        let tree = mgr
            .client
            .send_command("Page.getFrameTree", None, Some(&session_id))
            .await?;
        let mut observation =
            crate::native::service_download_artifact::DownloadObservation::from_frame_tree(&tree)?;
        const DOWNLOAD_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(30);
        let deadline = tokio::time::Instant::now() + DOWNLOAD_TIMEOUT;
        // The requested path is an artifact delivery destination. Preserve the
        // current browser-context policy so a concurrent peer keeps its own
        // configured download directory.
        let observer = tokio::time::timeout(
            deadline.saturating_duration_since(tokio::time::Instant::now()),
            crate::native::service_download_artifact::subscribe(mgr.get_cdp_url()),
        )
        .await
        .map_err(|_| "download_subscription_failed: setup deadline elapsed")??;
        crate::native::service_download_artifact::verify_process(&owner)?;
        let mut rx = observer.subscribe();
        tokio::time::timeout(
            deadline.saturating_duration_since(tokio::time::Instant::now()),
            interaction::click(
                &mgr.client,
                &session_id,
                &state.ref_map,
                selector,
                "left",
                1,
                &state.iframe_sessions,
            ),
        )
        .await
        .map_err(|_| "download_click_uncertain: click deadline elapsed".to_string())??;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("Timeout waiting for download to complete".to_string());
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(event)) => {
                    if observation.observe(&event.method, &event.params)? {
                        break;
                    }
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                    return Err("download_event_unproven: download observation lost events".into());
                }
                Ok(Err(_)) => {
                    return Err("download_event_unproven: event stream closed".to_string())
                }
                Err(_) => {
                    return Err("download_events_unavailable: no exact completion observed; inspect existing context policy and trace before retry".to_string());
                }
            }
        }
        let source = observation
            .path
            .as_deref()
            .ok_or("download_completion_path_missing: no completed path")?;
        crate::native::service_download_artifact::deliver(&owner, Path::new(source), &dest, None)?;
        let dest_str = dest.to_string_lossy().to_string();
        Ok(json!({ "path" : dest_str }))
    }
    pub(crate) async fn handle_waitfordownload(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let timeout_ms = state.timeout_ms(cmd);
        let expected_path = cmd.get("path").and_then(|v| v.as_str()).map(String::from);
        let initial_file_state = expected_path.as_ref().and_then(|path| {
            std::fs::metadata(path).ok().map(|meta| {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dur| dur.as_nanos());
                (meta.len(), modified)
            })
        });
        let mut rx = mgr.client.subscribe();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("Timeout waiting for download".to_string());
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(event)) => {
                    let is_page_session = event.session_id.as_deref() == Some(&session_id);
                    let is_progress = event.method == "Browser.downloadProgress"
                        || (event.method == "Page.downloadProgress" && is_page_session);
                    if is_progress
                        && event.params.get("state").and_then(|v| v.as_str()) == Some("completed")
                    {
                        let path = expected_path.as_deref().unwrap_or("download");
                        return Ok(json!({ "path" : path }));
                    }
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(_)) => return Err("Event stream closed".to_string()),
                Err(_) => return Err("Timeout waiting for download".to_string()),
            }
            if let Some(ref path) = expected_path {
                if let Ok(meta) = std::fs::metadata(path) {
                    let modified = meta
                        .modified()
                        .ok()
                        .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|dur| dur.as_nanos());
                    let current_state = (meta.len(), modified);
                    if initial_file_state.as_ref() != Some(&current_state) {
                        return Ok(json!({ "path" : path }));
                    }
                }
            }
        }
    }
}
pub(crate) use action_commands::*;
