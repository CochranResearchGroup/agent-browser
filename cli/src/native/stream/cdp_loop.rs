use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch, Mutex, RwLock};

use crate::native::network;
use crate::native::service_failure_journal::{
    append_service_failure_best_effort, ServiceFailureCategory, ServiceFailureRecord,
    ServiceFailureReferences,
};
use agent_browser_cdp::client::CdpClient;
use agent_browser_cdp::types::{CaptureScreenshotParams, CaptureScreenshotResult};

use super::timestamp_ms;

const INITIAL_SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(2);
const CDP_FRAME_WATCHDOG_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameWatchdogFailure {
    NeverReceived,
    Stalled,
}

struct FrameWatchdogState {
    started_at: Instant,
    last_frame_at: Option<Instant>,
    outage_reported: bool,
}

impl FrameWatchdogState {
    fn new(started_at: Instant, initial_frame_received: bool) -> Self {
        Self {
            started_at,
            last_frame_at: initial_frame_received.then_some(started_at),
            outage_reported: false,
        }
    }

    fn observe_frame(&mut self, observed_at: Instant) {
        self.last_frame_at = Some(observed_at);
        self.outage_reported = false;
    }

    fn poll(&mut self, now: Instant) -> Option<(FrameWatchdogFailure, Duration)> {
        let elapsed = self
            .last_frame_at
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_else(|| now.saturating_duration_since(self.started_at));
        if elapsed < CDP_FRAME_WATCHDOG_INTERVAL || self.outage_reported {
            return None;
        }
        self.outage_reported = true;
        Some((
            if self.last_frame_at.is_some() {
                FrameWatchdogFailure::Stalled
            } else {
                FrameWatchdogFailure::NeverReceived
            },
            elapsed,
        ))
    }
}

/// Background task that subscribes to CDP events and broadcasts screencast frames in real-time.
/// Also handles auto-start/stop of screencast based on WebSocket client count.
#[allow(clippy::too_many_arguments)]
pub(super) async fn cdp_event_loop(
    frame_tx: broadcast::Sender<String>,
    client_slot: Arc<RwLock<Option<Arc<CdpClient>>>>,
    client_notify: Arc<tokio::sync::Notify>,
    screencasting: Arc<Mutex<bool>>,
    client_count: Arc<Mutex<usize>>,
    cdp_session_id: Arc<RwLock<Option<String>>>,
    viewport_width: Arc<Mutex<u32>>,
    viewport_height: Arc<Mutex<u32>>,
    last_frame: Arc<RwLock<Option<String>>>,
    last_tabs: Arc<RwLock<Vec<Value>>>,
    last_engine: Arc<RwLock<String>>,
    recording: Arc<Mutex<bool>>,
    service_session_id: String,
    stream_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    let session_id = cdp_session_id.read().await.clone();
                    if *screencasting.lock().await {
                        if let Some(ref client) = *client_slot.read().await {
                            let _ = client
                                .send_command_no_params("Page.stopScreencast", session_id.as_deref())
                                .await;
                        }
                        let mut sc = screencasting.lock().await;
                        *sc = false;
                    }
                    return;
                }
            }
            _ = client_notify.notified() => {}
        }

        let count = *client_count.lock().await;
        let guard = client_slot.read().await;

        if count > 0 {
            if let Some(ref client) = *guard {
                let mut event_rx = client.subscribe();
                let client_arc = Arc::clone(client);
                drop(guard);

                let session_id = cdp_session_id.read().await.clone();

                let vw = *viewport_width.lock().await;
                let vh = *viewport_height.lock().await;

                let eng = last_engine.read().await.clone();
                let supports_screencast = eng == "chrome";

                let _ = client_arc
                    .send_command_no_params("Runtime.enable", session_id.as_deref())
                    .await;

                let mut initial_frame_received = false;
                let screencast_started = if supports_screencast {
                    initial_frame_received = tokio::time::timeout(
                        INITIAL_SCREENSHOT_TIMEOUT,
                        broadcast_initial_screenshot(
                            &frame_tx,
                            &client_arc,
                            session_id.as_deref(),
                            vw,
                            vh,
                            &last_frame,
                        ),
                    )
                    .await
                    .unwrap_or(false);
                    match client_arc
                        .send_command(
                            "Page.startScreencast",
                            Some(json!({
                                "format": "jpeg",
                                "quality": 80,
                                "maxWidth": vw,
                                "maxHeight": vh,
                                "everyNthFrame": 1,
                            })),
                            session_id.as_deref(),
                        )
                        .await
                    {
                        Ok(_) => true,
                        Err(_) => {
                            record_cdp_stream_failure(
                                &service_session_id,
                                stream_port,
                                "start_screencast",
                                "cdp_screencast_start_failed",
                                "CDP screencast could not be started",
                                None,
                            );
                            false
                        }
                    }
                } else {
                    false
                };

                {
                    let mut sc = screencasting.lock().await;
                    *sc = screencast_started;
                }

                let rec = *recording.lock().await;
                let status = json!({
                    "type": "status",
                    "connected": true,
                    "screencasting": screencast_started,
                    "viewportWidth": vw,
                    "viewportHeight": vh,
                    "engine": eng,
                    "recording": rec,
                });
                let _ = frame_tx.send(status.to_string());

                let mut watchdog_state =
                    FrameWatchdogState::new(Instant::now(), initial_frame_received);
                let mut frame_watchdog = tokio::time::interval(CDP_FRAME_WATCHDOG_INTERVAL);
                frame_watchdog.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                frame_watchdog.tick().await;

                loop {
                    tokio::select! {
                        changed = shutdown_rx.changed() => {
                            if changed.is_err() || *shutdown_rx.borrow() {
                                if screencast_started {
                                    let session_id = cdp_session_id.read().await.clone();
                                    let _ = client_arc
                                        .send_command_no_params("Page.stopScreencast", session_id.as_deref())
                                        .await;
                                }
                                let mut sc = screencasting.lock().await;
                                *sc = false;
                                return;
                            }
                        }
                        event = event_rx.recv() => {
                            match event {
                                Ok(evt) => {
                                    if evt.method == "Page.frameNavigated" {
                                        if let Some(frame) = evt.params.get("frame") {
                                            let is_main = frame
                                                .get("parentId")
                                                .and_then(|v| v.as_str())
                                                .is_none_or(|s| s.is_empty());
                                            if is_main {
                                                if let Some(url) = frame.get("url").and_then(|v| v.as_str()) {
                                                    {
                                                        let mut tabs = last_tabs.write().await;
                                                        for tab in tabs.iter_mut() {
                                                            if tab.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
                                                                tab.as_object_mut().map(|o| o.insert("url".to_string(), json!(url)));
                                                            }
                                                        }
                                                    }
                                                    let msg = json!({
                                                        "type": "url",
                                                        "url": url,
                                                        "timestamp": timestamp_ms(),
                                                    });
                                                    let _ = frame_tx.send(msg.to_string());
                                                }
                                            }
                                        }
                                    } else if evt.method == "Page.screencastFrame" {
                                        if let Some(sid) = evt.params.get("sessionId").and_then(|v| v.as_i64()) {
                                            let _ = client_arc.send_command(
                                                "Page.screencastFrameAck",
                                                Some(json!({ "sessionId": sid })),
                                                evt.session_id.as_deref(),
                                            ).await;
                                        }

                                        if let Some(data) = evt.params.get("data").and_then(|v| v.as_str()) {
                                            watchdog_state.observe_frame(Instant::now());
                                            let meta = evt.params.get("metadata");
                                            let msg = json!({
                                                "type": "frame",
                                                "data": data,
                                                "metadata": {
                                                    "offsetTop": meta.and_then(|m| m.get("offsetTop")).and_then(|v| v.as_f64()).unwrap_or(0.0),
                                                    "pageScaleFactor": meta.and_then(|m| m.get("pageScaleFactor")).and_then(|v| v.as_f64()).unwrap_or(1.0),
                                                    "deviceWidth": vw,
                                                    "deviceHeight": vh,
                                                    "scrollOffsetX": meta.and_then(|m| m.get("scrollOffsetX")).and_then(|v| v.as_f64()).unwrap_or(0.0),
                                                    "scrollOffsetY": meta.and_then(|m| m.get("scrollOffsetY")).and_then(|v| v.as_f64()).unwrap_or(0.0),
                                                    "timestamp": meta.and_then(|m| m.get("timestamp")).and_then(|v| v.as_u64()).unwrap_or(0),
                                                }
                                            });
                                            let msg_str = msg.to_string();
                                            {
                                                let mut lf = last_frame.write().await;
                                                *lf = Some(msg_str.clone());
                                            }
                                            let _ = frame_tx.send(msg_str);
                                        }
                                    } else if evt.method == "Runtime.consoleAPICalled" {
                                        let level = evt.params.get("type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("log");
                                        let raw_args = evt.params.get("args")
                                            .and_then(|v| v.as_array())
                                            .cloned()
                                            .unwrap_or_default();
                                        let text = network::format_console_args(&raw_args);
                                        if !text.is_empty() {
                                            let mut msg = json!({
                                                "type": "console",
                                                "level": level,
                                                "text": text,
                                                "timestamp": timestamp_ms(),
                                            });
                                            if !raw_args.is_empty() {
                                                msg.as_object_mut().unwrap().insert(
                                                    "args".to_string(),
                                                    Value::Array(raw_args),
                                                );
                                            }
                                            let _ = frame_tx.send(msg.to_string());
                                        }
                                    } else if evt.method == "Runtime.exceptionThrown" {
                                        let text = evt.params.get("exceptionDetails")
                                            .and_then(|d| {
                                                d.get("exception")
                                                    .and_then(|e| e.get("description").and_then(|v| v.as_str()))
                                                    .or_else(|| d.get("text").and_then(|v| v.as_str()))
                                            })
                                            .unwrap_or("Unknown error");
                                        let line = evt.params.get("exceptionDetails")
                                            .and_then(|d| d.get("lineNumber").and_then(|v| v.as_i64()));
                                        let column = evt.params.get("exceptionDetails")
                                            .and_then(|d| d.get("columnNumber").and_then(|v| v.as_i64()));
                                        let msg = json!({
                                            "type": "page_error",
                                            "text": text,
                                            "line": line,
                                            "column": column,
                                            "timestamp": timestamp_ms(),
                                        });
                                        let _ = frame_tx.send(msg.to_string());
                                    }
                                }
                                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                                Err(broadcast::error::RecvError::Closed) => break,
                            }
                        }
                        _ = frame_watchdog.tick(), if screencast_started => {
                            if let Some((failure, elapsed)) = watchdog_state.poll(Instant::now()) {
                                let (code, summary) = match failure {
                                    FrameWatchdogFailure::Stalled =>
                                        ("cdp_frame_stream_stalled", "CDP screencast stopped producing frames"),
                                    FrameWatchdogFailure::NeverReceived =>
                                        ("cdp_frame_never_received", "CDP screencast produced no frames"),
                                };
                                record_cdp_stream_failure(
                                    &service_session_id,
                                    stream_port,
                                    "frame_watchdog",
                                    code,
                                    summary,
                                    Some(elapsed.as_millis().min(u128::from(u64::MAX)) as u64),
                                );
                            }
                        }
                        _ = client_notify.notified() => {
                            let count = *client_count.lock().await;
                            let new_session_id = cdp_session_id.read().await.clone();
                            if count == 0 {
                                if screencast_started {
                                    let _ = client_arc
                                        .send_command_no_params("Page.stopScreencast", session_id.as_deref())
                                        .await;
                                }
                                let mut sc = screencasting.lock().await;
                                *sc = false;
                                break;
                            }
                            let client_changed = {
                                let guard = client_slot.read().await;
                                let same = guard
                                    .as_ref()
                                    .is_some_and(|c| Arc::ptr_eq(c, &client_arc));
                                !same
                            };
                            let session_changed = new_session_id != session_id;
                            let new_vw = *viewport_width.lock().await;
                            let new_vh = *viewport_height.lock().await;
                            let viewport_changed = new_vw != vw || new_vh != vh;
                            if client_changed || session_changed || viewport_changed {
                                if screencast_started {
                                    let _ = client_arc
                                        .send_command_no_params("Page.stopScreencast", session_id.as_deref())
                                        .await;
                                }
                                let mut sc = screencasting.lock().await;
                                *sc = false;
                                client_notify.notify_one();
                                break;
                            }
                        }
                    }
                }
            } else {
                drop(guard);
            }
        } else {
            let was_screencasting = *screencasting.lock().await;
            if was_screencasting {
                if let Some(ref client) = *guard {
                    let session_id = cdp_session_id.read().await.clone();
                    let _ = client
                        .send_command_no_params("Page.stopScreencast", session_id.as_deref())
                        .await;
                }
                let mut sc = screencasting.lock().await;
                *sc = false;
            }
            drop(guard);
        }
    }
}

fn record_cdp_stream_failure(
    service_session_id: &str,
    stream_port: u16,
    stage: &str,
    code: &str,
    summary: &str,
    elapsed_ms: Option<u64>,
) {
    let mut details = json!({
        "streamPort": stream_port,
        "watchdogIntervalMs": CDP_FRAME_WATCHDOG_INTERVAL.as_millis() as u64,
    });
    if let Some(elapsed_ms) = elapsed_ms {
        details["elapsedMs"] = json!(elapsed_ms);
    }
    let record = ServiceFailureRecord::new(
        ServiceFailureCategory::CdpStream,
        "stream_server",
        stage,
        code,
        summary,
    )
    .with_action("cdp_stream")
    .with_references(ServiceFailureReferences {
        session_id: Some(service_session_id.to_string()),
        ..ServiceFailureReferences::default()
    })
    .with_details(details);
    append_service_failure_best_effort(&record);
}

async fn broadcast_initial_screenshot(
    frame_tx: &broadcast::Sender<String>,
    client: &CdpClient,
    session_id: Option<&str>,
    viewport_width: u32,
    viewport_height: u32,
    last_frame: &Arc<RwLock<Option<String>>>,
) -> bool {
    let params = CaptureScreenshotParams {
        format: Some("jpeg".to_string()),
        quality: Some(80),
        clip: None,
        from_surface: Some(true),
        capture_beyond_viewport: None,
    };

    let Ok(result) = client
        .send_command_typed::<_, CaptureScreenshotResult>(
            "Page.captureScreenshot",
            &params,
            session_id,
        )
        .await
    else {
        return false;
    };

    let msg = json!({
        "type": "frame",
        "data": result.data,
        "metadata": {
            "offsetTop": 0.0,
            "pageScaleFactor": 1.0,
            "deviceWidth": viewport_width,
            "deviceHeight": viewport_height,
            "scrollOffsetX": 0.0,
            "scrollOffsetY": 0.0,
            "timestamp": timestamp_ms(),
        }
    });
    let msg_str = msg.to_string();
    {
        let mut lf = last_frame.write().await;
        *lf = Some(msg_str.clone());
    }
    let _ = frame_tx.send(msg_str);
    true
}

pub async fn start_screencast(
    client: &CdpClient,
    session_id: &str,
    format: &str,
    quality: i32,
    max_width: i32,
    max_height: i32,
) -> Result<(), String> {
    client
        .send_command(
            "Page.startScreencast",
            Some(json!({
                "format": format,
                "quality": quality,
                "maxWidth": max_width,
                "maxHeight": max_height,
                "everyNthFrame": 1,
            })),
            Some(session_id),
        )
        .await?;
    Ok(())
}

pub async fn stop_screencast(client: &CdpClient, session_id: &str) -> Result<(), String> {
    client
        .send_command_no_params("Page.stopScreencast", Some(session_id))
        .await?;
    Ok(())
}

pub async fn ack_screencast_frame(
    client: &CdpClient,
    session_id: &str,
    screencast_session_id: i64,
) -> Result<(), String> {
    client
        .send_command(
            "Page.screencastFrameAck",
            Some(json!({ "sessionId": screencast_session_id })),
            Some(session_id),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_watchdog_reports_each_outage_once_and_rearms_after_a_frame() {
        let started = Instant::now();
        let mut watchdog = FrameWatchdogState::new(started, false);
        assert_eq!(
            watchdog.poll(started + CDP_FRAME_WATCHDOG_INTERVAL),
            Some((
                FrameWatchdogFailure::NeverReceived,
                CDP_FRAME_WATCHDOG_INTERVAL
            ))
        );
        assert_eq!(
            watchdog.poll(started + CDP_FRAME_WATCHDOG_INTERVAL * 2),
            None,
            "one outage must not flood the journal"
        );

        let resumed = started + CDP_FRAME_WATCHDOG_INTERVAL * 2;
        watchdog.observe_frame(resumed);
        assert_eq!(
            watchdog.poll(resumed + CDP_FRAME_WATCHDOG_INTERVAL),
            Some((FrameWatchdogFailure::Stalled, CDP_FRAME_WATCHDOG_INTERVAL))
        );
    }

    #[test]
    fn initial_screenshot_counts_as_a_frame_before_continuous_stream_stalls() {
        let started = Instant::now();
        let mut watchdog = FrameWatchdogState::new(started, true);
        assert_eq!(
            watchdog.poll(started + CDP_FRAME_WATCHDOG_INTERVAL),
            Some((FrameWatchdogFailure::Stalled, CDP_FRAME_WATCHDOG_INTERVAL))
        );
    }
}
