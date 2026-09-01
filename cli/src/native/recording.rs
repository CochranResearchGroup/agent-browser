use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;

use agent_browser_cdp::client::CdpClient;
use agent_browser_cdp::types::{CaptureScreenshotParams, CaptureScreenshotResult};

const CAPTURE_INTERVAL_MS: u64 = 100;
const CAPTURE_FPS: u32 = 10;

pub struct RecordingState {
    pub active: bool,
    pub output_path: String,
    pub frame_count: u64,
    pub capture_task: Option<tokio::task::JoinHandle<Result<(), String>>>,
    pub shared_frame_count: Option<Arc<AtomicU64>>,
    pub cancel_tx: Option<oneshot::Sender<()>>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self {
            active: false,
            output_path: String::new(),
            frame_count: 0,
            capture_task: None,
            shared_frame_count: None,
            cancel_tx: None,
        }
    }
}

pub fn recording_start(state: &mut RecordingState, path: &str) -> Result<Value, String> {
    if state.active {
        return Err("Recording already active".to_string());
    }

    state.active = true;
    state.output_path = path.to_string();
    state.frame_count = 0;

    Ok(json!({ "started": true, "path": path }))
}

pub fn recording_stop(state: &mut RecordingState) -> Result<Value, String> {
    if !state.active {
        return Err("No recording in progress".to_string());
    }

    state.active = false;

    if state.frame_count == 0 {
        return Err("No frames captured".to_string());
    }

    Ok(json!({ "path": &state.output_path, "frames": state.frame_count }))
}

pub fn recording_restart(state: &mut RecordingState, path: &str) -> Result<Value, String> {
    let previous = if state.active {
        let stop_result = recording_stop(state);
        stop_result
            .ok()
            .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(String::from))
    } else {
        None
    };

    recording_start(state, path)?;

    Ok(json!({
        "restarted": true,
        "previousPath": previous,
        "path": path,
    }))
}

fn build_ffmpeg_command(output_path: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("ffmpeg");

    cmd.args(["-y"])
        .args(["-avioflags", "direct"])
        .args([
            "-fpsprobesize",
            "0",
            "-probesize",
            "32",
            "-analyzeduration",
            "0",
        ])
        .args([
            "-f",
            "image2pipe",
            "-c:v",
            "mjpeg",
            "-framerate",
            &CAPTURE_FPS.to_string(),
            "-i",
            "pipe:0",
        ])
        .args(["-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2"]);

    if output_path.ends_with(".webm") {
        cmd.args(["-c:v", "libvpx", "-crf", "30", "-b:v", "1M"]);
    } else {
        cmd.args(["-c:v", "libx264", "-preset", "ultrafast"]);
    }

    cmd.args(["-pix_fmt", "yuv420p", "-threads", "1"])
        .arg(output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    cmd
}

/// Spawn a background task that captures screenshots at a fixed interval
/// and pipes them to ffmpeg in real-time.
pub fn spawn_recording_task(
    client: Arc<CdpClient>,
    session_id: String,
    output_path: String,
    shared_count: Arc<AtomicU64>,
    cancel_rx: oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<Result<(), String>> {
    tokio::spawn(async move {
        let mut cancel_rx = std::pin::pin!(cancel_rx);

        let mut ffmpeg = build_ffmpeg_command(&output_path).spawn().map_err(|e| {
            format!(
                "ffmpeg not found or failed to execute: {}. Install ffmpeg to enable recording.",
                e
            )
        })?;

        let mut stdin = ffmpeg
            .stdin
            .take()
            .ok_or_else(|| "Failed to open ffmpeg stdin".to_string())?;

        let mut interval = tokio::time::interval(Duration::from_millis(CAPTURE_INTERVAL_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let params = CaptureScreenshotParams {
            format: Some("jpeg".to_string()),
            quality: Some(80),
            clip: None,
            from_surface: Some(true),
            capture_beyond_viewport: None,
        };

        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                _ = interval.tick() => {}
            }

            let result: Result<CaptureScreenshotResult, _> = client
                .send_command_typed("Page.captureScreenshot", &params, Some(&session_id))
                .await;

            let screenshot = match result {
                Ok(s) => s,
                Err(e) => {
                    if e.contains("Target closed") || e.contains("not found") {
                        break;
                    }
                    continue;
                }
            };

            let bytes = match base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &screenshot.data,
            ) {
                Ok(b) => b,
                Err(_) => continue,
            };

            if stdin.write_all(&bytes).await.is_err() {
                break;
            }
            shared_count.fetch_add(1, Ordering::Relaxed);
        }

        drop(stdin);

        let output = ffmpeg
            .wait_with_output()
            .await
            .map_err(|e| format!("ffmpeg wait failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "ffmpeg failed: {}",
                stderr.chars().take(300).collect::<String>()
            ));
        }

        Ok(())
    })
}

pub async fn stop_recording_task(state: &mut RecordingState) -> Result<(), String> {
    if let Some(tx) = state.cancel_tx.take() {
        let _ = tx.send(());
    }

    let counter = state.shared_frame_count.take();
    let handle = state.capture_task.take();

    let result = if let Some(h) = handle {
        match h.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(format!("Recording task panicked: {}", e)),
        }
    } else {
        Ok(())
    };

    if let Some(c) = counter {
        state.frame_count = c.load(Ordering::Relaxed);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_state_new() {
        let state = RecordingState::new();
        assert!(!state.active);
        assert!(state.output_path.is_empty());
        assert_eq!(state.frame_count, 0);
    }

    #[test]
    fn test_recording_start_sets_active() {
        let mut state = RecordingState::new();
        let result = recording_start(&mut state, "/tmp/test.mp4");
        assert!(result.is_ok());
        assert!(state.active);
        assert_eq!(state.output_path, "/tmp/test.mp4");
        assert_eq!(state.frame_count, 0);
    }

    #[test]
    fn test_recording_start_while_active() {
        let mut state = RecordingState::new();
        recording_start(&mut state, "/tmp/test1.mp4").unwrap();
        let result = recording_start(&mut state, "/tmp/test2.mp4");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already active"));
    }

    #[test]
    fn test_recording_stop_not_active() {
        let mut state = RecordingState::new();
        let result = recording_stop(&mut state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No recording"));
    }

    #[test]
    fn test_recording_stop_no_frames() {
        let mut state = RecordingState::new();
        recording_start(&mut state, "/tmp/test.mp4").unwrap();
        let result = recording_stop(&mut state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No frames"));
        assert!(!state.active);
    }

    #[test]
    fn test_recording_restart_while_inactive() {
        let mut state = RecordingState::new();
        let result = recording_restart(&mut state, "/tmp/new.webm");
        assert!(result.is_ok());
        assert!(state.active);
        assert_eq!(state.output_path, "/tmp/new.webm");
    }

    #[test]
    fn test_recording_restart_while_active() {
        let mut state = RecordingState::new();
        recording_start(&mut state, "/tmp/old.webm").unwrap();
        state.frame_count = 10;
        let result = recording_restart(&mut state, "/tmp/new.webm").unwrap();
        assert!(state.active);
        assert_eq!(state.output_path, "/tmp/new.webm");
        assert_eq!(state.frame_count, 0);
        assert_eq!(result["previousPath"], "/tmp/old.webm");
    }

    #[test]
    fn test_build_ffmpeg_command_webm() {
        let cmd = build_ffmpeg_command("/tmp/out.webm");
        let args: Vec<&std::ffi::OsStr> = cmd.as_std().get_args().collect();
        let args_str: Vec<&str> = args.iter().filter_map(|a| a.to_str()).collect();
        assert!(args_str.contains(&"libvpx"));
        assert!(args_str.contains(&"/tmp/out.webm"));
    }

    #[test]
    fn test_build_ffmpeg_command_mp4() {
        let cmd = build_ffmpeg_command("/tmp/out.mp4");
        let args: Vec<&std::ffi::OsStr> = cmd.as_std().get_args().collect();
        let args_str: Vec<&str> = args.iter().filter_map(|a| a.to_str()).collect();
        assert!(args_str.contains(&"libx264"));
        assert!(args_str.contains(&"/tmp/out.mp4"));
    }
}
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
    use crate::native::browser::{
        should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo,
        ProcessExitObservation, WaitUntil,
    };
    use crate::native::cookies;
    use crate::native::recording::{self, RecordingState};
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use agent_browser_cdp::client::CdpClient;
    use agent_browser_cdp::types::{
        AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
        DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
        TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
    };
    use serde_json::{json, Map, Value};
    use std::process::{Command, Stdio};
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::{broadcast, oneshot, RwLock};
    pub(crate) async fn handle_recording_start(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let path = cmd
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let recording_url = cmd
            .get("url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let (client, recording_session_id) = {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            let active_session_id = mgr.active_session_id()?.to_string();
            let current_url = mgr
                .get_url()
                .await
                .unwrap_or_else(|_| "about:blank".to_string());
            if recording_url.is_none_or(|u| u == current_url) {
                (mgr.client.clone(), active_session_id)
            } else {
                let nav_url = recording_url.unwrap_or("about:blank").to_string();
                let cookies_result = mgr
                    .client
                    .send_command_no_params("Network.getAllCookies", Some(&active_session_id))
                    .await
                    .ok();
                let ctx_result = mgr
                    .client
                    .send_command_no_params("Target.createBrowserContext", None)
                    .await?;
                let context_id = ctx_result
                    .get("browserContextId")
                    .and_then(|v| v.as_str())
                    .ok_or("Failed to get browserContextId")?
                    .to_string();
                let create_result: CreateTargetResult = mgr
                    .client
                    .send_command_typed(
                        "Target.createTarget",
                        &json!(
                            { "url" : "about:blank", "browserContextId" : context_id }
                        ),
                        None,
                    )
                    .await?;
                let attach_result: AttachToTargetResult = mgr
                    .client
                    .send_command_typed(
                        "Target.attachToTarget",
                        &AttachToTargetParams {
                            target_id: create_result.target_id.clone(),
                            flatten: true,
                        },
                        None,
                    )
                    .await?;
                let new_session_id = attach_result.session_id.clone();
                mgr.enable_domains_pub(&new_session_id).await?;
                if let Some(ref dl_path) = mgr.download_path {
                    let _ = mgr
                        .client
                        .send_command(
                            "Browser.setDownloadBehavior",
                            Some(json!(
                                { "behavior" : "allow", "downloadPath" : dl_path,
                                "browserContextId" : context_id, "eventsEnabled" : true }
                            )),
                            None,
                        )
                        .await;
                }
                if let Some(ref cr) = cookies_result {
                    if let Some(cookie_arr) = cr.get("cookies").and_then(|v| v.as_array()) {
                        if !cookie_arr.is_empty() {
                            let _ = mgr
                                .client
                                .send_command(
                                    "Network.setCookies",
                                    Some(json!({ "cookies" : cookie_arr })),
                                    Some(&new_session_id),
                                )
                                .await;
                        }
                    }
                }
                if mgr.ignore_https_errors {
                    let _ = mgr
                        .client
                        .send_command(
                            "Security.setIgnoreCertificateErrors",
                            Some(json!({ "ignore" : true })),
                            Some(&new_session_id),
                        )
                        .await;
                }
                mgr.add_page(super::super::browser::PageInfo {
                    target_id: create_result.target_id,
                    session_id: new_session_id.clone(),
                    url: nav_url.clone(),
                    title: String::new(),
                    target_type: "page".to_string(),
                });
                if nav_url != "about:blank" {
                    let _ = mgr
                        .client
                        .send_command(
                            "Page.navigate",
                            Some(json!({ "url" : nav_url })),
                            Some(&new_session_id),
                        )
                        .await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }
                (mgr.client.clone(), new_session_id)
            }
        };
        let result = recording::recording_start(&mut state.recording_state, path)?;
        state
            .start_recording_task(client, recording_session_id)
            .await?;
        if let Some(ref server) = state.stream_server {
            server.set_recording(true, &state.engine).await;
        }
        Ok(result)
    }
    pub(crate) async fn handle_recording_stop(state: &mut DaemonState) -> Result<Value, String> {
        state.stop_recording_task().await?;
        let result = recording::recording_stop(&mut state.recording_state);
        if let Some(ref server) = state.stream_server {
            server.set_recording(false, &state.engine).await;
        }
        result
    }
    pub(crate) async fn handle_recording_restart(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let path = cmd
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let _ = state.stop_recording_task().await;
        let result = recording::recording_restart(&mut state.recording_state, path)?;
        if let Some(ref browser) = state.browser {
            let session_id = browser.active_session_id()?.to_string();
            state
                .start_recording_task(browser.client.clone(), session_id)
                .await?;
        }
        Ok(result)
    }
    pub(crate) async fn handle_video_start(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let path = cmd
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        if state.recording_state.active {
            return Err("A recording is already in progress".to_string());
        }
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        recording::recording_start(&mut state.recording_state, path)?;
        state
            .start_recording_task(mgr.client.clone(), session_id)
            .await?;
        Ok(json!(
            { "started" : true, "note" :
            "Video recording started. Use video_stop to save the recording." }
        ))
    }
    pub(crate) async fn handle_video_stop(state: &mut DaemonState) -> Result<Value, String> {
        if !state.recording_state.active {
            return Ok(json!(
                { "stopped" : false, "note" :
                "No video recording was started. Use recording_stop if you used recording_start."
                }
            ));
        }
        state.stop_recording_task().await?;
        recording::recording_stop(&mut state.recording_state)
    }
    /// Begin capturing network traffic for a later HAR export.
    pub(crate) async fn handle_har_start(state: &mut DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        mgr.client
            .send_command_no_params("Network.enable", Some(&session_id))
            .await?;
        for iframe_sid in state.iframe_sessions.values() {
            let _ = mgr
                .client
                .send_command_no_params("Network.enable", Some(iframe_sid.as_str()))
                .await;
        }
        state.har_recording = true;
        state.har_entries.clear();
        Ok(json!({ "started" : true }))
    }
}
pub(crate) use action_commands::*;
