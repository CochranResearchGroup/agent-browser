use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::native::remote_view::{
    route_display_observation_for_source, RouteDisplayObservationState,
};
use crate::native::service_model::{BrowserHost, ViewStreamProvider};

use super::observation::{
    StatusObservationComponentState, StatusObservationError, StatusObservationErrorCode,
    StatusObservationRequest, StatusObservationSnapshot, StatusObservationSource,
    StatusObservationSourceKind, StatusObservationState, StatusRoutePresentation,
    StatusRoutePresentationSource, StatusViewStreamObservation, StatusViewStreamObservationState,
};

const OBSERVATION_MAX_AGE: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
pub(crate) struct LocalStatusObservationAdapter;

#[async_trait]
impl StatusObservationSource for LocalStatusObservationAdapter {
    async fn snapshot(&self, request: StatusObservationRequest) -> StatusObservationSnapshot {
        let task = tokio::task::spawn_blocking(move || local_snapshot(request));
        match task.await {
            Ok(snapshot) => snapshot,
            Err(error) => unavailable_snapshot(
                StatusObservationSourceKind::Local,
                "host",
                StatusObservationErrorCode::DisplayProbeFailed,
                format!("local observation task failed: {error}"),
            ),
        }
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableStatusObservationAdapter {
    reason: String,
}

impl UnavailableStatusObservationAdapter {
    pub(crate) fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl StatusObservationSource for UnavailableStatusObservationAdapter {
    async fn snapshot(&self, _request: StatusObservationRequest) -> StatusObservationSnapshot {
        unavailable_snapshot(
            StatusObservationSourceKind::Unavailable,
            "host",
            StatusObservationErrorCode::DisplayProbeUnavailable,
            self.reason.clone(),
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InMemoryStatusObservationAdapter {
    snapshot: StatusObservationSnapshot,
}

impl InMemoryStatusObservationAdapter {
    pub(crate) fn new(snapshot: StatusObservationSnapshot) -> Self {
        Self { snapshot }
    }
}

#[async_trait]
impl StatusObservationSource for InMemoryStatusObservationAdapter {
    async fn snapshot(&self, _request: StatusObservationRequest) -> StatusObservationSnapshot {
        self.snapshot.clone()
    }
}

fn local_snapshot(request: StatusObservationRequest) -> StatusObservationSnapshot {
    let source_host_id = source_host_id();
    let display_cache_host_id = source_host_id
        .as_deref()
        .unwrap_or("unavailable_source_host");
    let mut errors = Vec::new();
    let (manual_browsers_state, manual_browsers) =
        match crate::runtime_profile::list_manual_runtime_browsers() {
            Ok(browsers) => (StatusObservationComponentState::Observed, browsers),
            Err(error) => {
                errors.push(StatusObservationError {
                    code: StatusObservationErrorCode::RuntimeProfileUnavailable,
                    subject: "host".to_string(),
                    message: error,
                });
                (StatusObservationComponentState::Unavailable, Vec::new())
            }
        };

    let mut browser_process_stats = BTreeMap::new();
    for (browser_id, browser) in &request.service_state.browsers {
        if let Some(stats) = browser.pid.and_then(process_stats_for_pid) {
            browser_process_stats.insert(browser_id.clone(), stats);
        }
    }
    let modeled_browser_ids = request
        .service_state
        .browsers
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let browser_process_state = browser_process_observation_state(
        &modeled_browser_ids,
        &browser_process_stats,
        &mut errors,
    );

    let base_observed_at = Utc::now();
    let base_observed_at_text = base_observed_at.to_rfc3339_opts(SecondsFormat::Millis, true);
    let base_valid_until_text = (base_observed_at
        + chrono::Duration::milliseconds(OBSERVATION_MAX_AGE.as_millis() as i64))
    .to_rfc3339_opts(SecondsFormat::Millis, true);

    let configured_client_url = env::var("AGENT_BROWSER_REMOTE_VIEW_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| value.contains("#/client/"));
    let mut view_streams = Vec::new();
    for (browser_id, browser) in &request.service_state.browsers {
        if browser.host != BrowserHost::RemoteHeaded {
            continue;
        }
        for stream in &browser.view_streams {
            if stream.provider != ViewStreamProvider::RdpGateway {
                continue;
            }
            let display_name = stream
                .display_allocation_id
                .as_deref()
                .and_then(|id| request.service_state.display_allocations.get(id))
                .and_then(|allocation| allocation.display_name.as_deref())
                .or(browser.display_name.as_deref());
            let display = display_name.and_then(|display| {
                route_display_observation_for_source(display_cache_host_id, display)
            });
            let route_presentation =
                configured_client_url
                    .as_ref()
                    .map(|url| StatusRoutePresentation {
                        frame_url: url.clone(),
                        external_url: url.clone(),
                        source: StatusRoutePresentationSource::ConfiguredClientUrl,
                    });
            let display_state = display.as_ref().map(|display| display.state);
            let display_observed = display_state == Some(RouteDisplayObservationState::Observed);
            if let Some(display) = display
                .as_ref()
                .filter(|display| display.state != RouteDisplayObservationState::Observed)
            {
                errors.push(StatusObservationError {
                    code: display_error_code(display.state),
                    subject: format!("{browser_id}:{}", stream.id),
                    message: display
                        .error
                        .clone()
                        .unwrap_or_else(|| "display observation failed".to_string()),
                });
            }
            if display.is_none() && route_presentation.is_none() {
                errors.push(StatusObservationError {
                    code: StatusObservationErrorCode::ConfiguredRouteUnavailable,
                    subject: format!("{browser_id}:{}", stream.id),
                    message: "no configured route or authorized display was available".to_string(),
                });
            }
            let state = if display_observed || route_presentation.is_some() {
                StatusViewStreamObservationState::Observed
            } else {
                display_state
                    .map(display_stream_state)
                    .unwrap_or(StatusViewStreamObservationState::Unavailable)
            };
            let route_observed_at = route_presentation
                .as_ref()
                .map(|_| base_observed_at_text.clone());
            let route_valid_until = route_presentation
                .as_ref()
                .map(|_| base_valid_until_text.clone());
            let observed_at = if state == StatusViewStreamObservationState::Observed {
                earliest_timestamp(
                    route_observed_at,
                    display
                        .as_ref()
                        .and_then(|display| display.observed_at.clone()),
                )
            } else {
                None
            };
            let valid_until = if state == StatusViewStreamObservationState::Observed {
                earliest_timestamp(
                    route_valid_until,
                    display
                        .as_ref()
                        .and_then(|display| display.valid_until.clone()),
                )
            } else {
                None
            };
            view_streams.push(StatusViewStreamObservation {
                browser_id: browser_id.clone(),
                stream_id: stream.id.clone(),
                state,
                observed_at,
                valid_until,
                max_age_ms: OBSERVATION_MAX_AGE.as_millis() as u64,
                route_presentation,
                display_content: display.and_then(|display| display.content),
            });
        }
    }
    view_streams.sort_by(|left, right| {
        (&left.browser_id, &left.stream_id).cmp(&(&right.browser_id, &right.stream_id))
    });
    errors.sort_by(|left, right| {
        (&left.subject, &left.code, &left.message).cmp(&(
            &right.subject,
            &right.code,
            &right.message,
        ))
    });
    let completed =
        usize::from(manual_browsers_state != StatusObservationComponentState::Unavailable)
            + usize::from(browser_process_state != StatusObservationComponentState::Unavailable)
            + view_streams
                .iter()
                .filter(|stream| stream.state == StatusViewStreamObservationState::Observed)
                .count();
    let requested = 2 + view_streams.len();
    let state = if completed == requested
        && manual_browsers_state == StatusObservationComponentState::Observed
        && browser_process_state == StatusObservationComponentState::Observed
        && errors.is_empty()
    {
        StatusObservationState::Complete
    } else if completed == 0 {
        StatusObservationState::Unavailable
    } else {
        StatusObservationState::Partial
    };
    let mut observed_at = (completed > 0).then(|| base_observed_at_text.clone());
    let mut valid_until = (completed > 0).then(|| base_valid_until_text.clone());
    for stream in &view_streams {
        observed_at = earliest_timestamp(observed_at, stream.observed_at.clone());
        valid_until = earliest_timestamp(valid_until, stream.valid_until.clone());
    }
    StatusObservationSnapshot {
        state,
        source: StatusObservationSourceKind::Local,
        source_host_id: (completed > 0).then_some(source_host_id).flatten(),
        observed_at,
        valid_until,
        max_age_ms: OBSERVATION_MAX_AGE.as_millis() as u64,
        manual_browsers_state,
        browser_process_state,
        errors,
        view_streams,
        manual_browsers,
        browser_process_stats,
    }
}

fn browser_process_observation_state(
    modeled_browser_ids: &[String],
    browser_process_stats: &BTreeMap<String, Value>,
    errors: &mut Vec<StatusObservationError>,
) -> StatusObservationComponentState {
    if modeled_browser_ids.is_empty() || browser_process_stats.len() == modeled_browser_ids.len() {
        return StatusObservationComponentState::Observed;
    }
    if browser_process_stats.is_empty() {
        errors.push(StatusObservationError {
            code: StatusObservationErrorCode::ProcessInventoryUnavailable,
            subject: "host".to_string(),
            message: "process inventory was unavailable for every modeled browser".to_string(),
        });
        return StatusObservationComponentState::Unavailable;
    }
    for browser_id in modeled_browser_ids {
        if !browser_process_stats.contains_key(browser_id) {
            errors.push(StatusObservationError {
                code: StatusObservationErrorCode::ProcessInventoryUnavailable,
                subject: browser_id.clone(),
                message: "process inventory was unavailable for this modeled browser".to_string(),
            });
        }
    }
    StatusObservationComponentState::Partial
}

fn unavailable_snapshot(
    source: StatusObservationSourceKind,
    subject: &str,
    code: StatusObservationErrorCode,
    message: String,
) -> StatusObservationSnapshot {
    StatusObservationSnapshot {
        state: StatusObservationState::Unavailable,
        source,
        source_host_id: None,
        observed_at: None,
        valid_until: None,
        max_age_ms: OBSERVATION_MAX_AGE.as_millis() as u64,
        manual_browsers_state: StatusObservationComponentState::Unavailable,
        browser_process_state: StatusObservationComponentState::Unavailable,
        errors: vec![StatusObservationError {
            code,
            subject: subject.to_string(),
            message,
        }],
        view_streams: Vec::new(),
        manual_browsers: Vec::new(),
        browser_process_stats: BTreeMap::new(),
    }
}

fn display_error_code(state: RouteDisplayObservationState) -> StatusObservationErrorCode {
    match state {
        RouteDisplayObservationState::TimedOut => StatusObservationErrorCode::DisplayProbeTimeout,
        RouteDisplayObservationState::Unsupported => {
            StatusObservationErrorCode::DisplayProbeUnsupported
        }
        RouteDisplayObservationState::Unavailable => {
            StatusObservationErrorCode::DisplayProbeUnavailable
        }
        RouteDisplayObservationState::Failed | RouteDisplayObservationState::Observed => {
            StatusObservationErrorCode::DisplayProbeFailed
        }
    }
}

fn display_stream_state(state: RouteDisplayObservationState) -> StatusViewStreamObservationState {
    match state {
        RouteDisplayObservationState::Observed => StatusViewStreamObservationState::Observed,
        RouteDisplayObservationState::TimedOut => StatusViewStreamObservationState::TimedOut,
        RouteDisplayObservationState::Unsupported => StatusViewStreamObservationState::Unsupported,
        RouteDisplayObservationState::Unavailable => StatusViewStreamObservationState::Unavailable,
        RouteDisplayObservationState::Failed => StatusViewStreamObservationState::Failed,
    }
}

fn earliest_timestamp(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn source_host_id() -> Option<String> {
    let identity = fs::read_to_string("/etc/machine-id")
        .ok()
        .or_else(|| env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "local-host-unidentified".to_string());
    let identity = identity.trim();
    let identity = if identity.is_empty() {
        "local-host-unidentified"
    } else {
        identity
    };
    let digest = Sha256::digest(identity.as_bytes());
    Some(format!("sha256:{}", hex::encode(digest)))
}

fn process_stats_for_pid(pid: u32) -> Option<Value> {
    #[cfg(target_os = "linux")]
    {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let stat_tail = stat.rsplit_once(") ")?.1;
        let fields = stat_tail.split_whitespace().collect::<Vec<_>>();
        let utime = fields.get(11)?.parse::<u64>().ok()?;
        let stime = fields.get(12)?.parse::<u64>().ok()?;
        let rss_bytes = fs::read_to_string(format!("/proc/{pid}/status"))
            .ok()
            .and_then(|status| {
                status.lines().find_map(|line| {
                    let value = line.strip_prefix("VmRSS:")?.split_whitespace().next()?;
                    value.parse::<u64>().ok().map(|kib| kib * 1024)
                })
            });
        let mut stats = json!({
            "pid": pid,
            "running": fs::metadata(format!("/proc/{pid}")).is_ok(),
            "cpuSeconds": ((utime + stime) as f64 / 100.0),
        });
        if let Some(rss_bytes) = rss_bytes {
            stats["rssBytes"] = json!(rss_bytes);
        }
        Some(stats)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_process_inventory_marks_only_missing_browser_unknown() {
        let ids = vec!["observed".to_string(), "missing".to_string()];
        let stats = BTreeMap::from([("observed".to_string(), json!({"pid": 100}))]);
        let mut errors = Vec::new();

        let state = browser_process_observation_state(&ids, &stats, &mut errors);

        assert_eq!(state, StatusObservationComponentState::Partial);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].subject, "missing");
        assert_eq!(
            errors[0].code,
            StatusObservationErrorCode::ProcessInventoryUnavailable
        );
    }

    #[test]
    fn display_terminal_states_remain_distinct_and_have_null_observation_values() {
        for (display, stream, code) in [
            (
                RouteDisplayObservationState::TimedOut,
                StatusViewStreamObservationState::TimedOut,
                StatusObservationErrorCode::DisplayProbeTimeout,
            ),
            (
                RouteDisplayObservationState::Unsupported,
                StatusViewStreamObservationState::Unsupported,
                StatusObservationErrorCode::DisplayProbeUnsupported,
            ),
            (
                RouteDisplayObservationState::Unavailable,
                StatusViewStreamObservationState::Unavailable,
                StatusObservationErrorCode::DisplayProbeUnavailable,
            ),
            (
                RouteDisplayObservationState::Failed,
                StatusViewStreamObservationState::Failed,
                StatusObservationErrorCode::DisplayProbeFailed,
            ),
        ] {
            assert_eq!(display_stream_state(display), stream);
            assert_eq!(display_error_code(display), code);
            let observation = StatusViewStreamObservation {
                browser_id: "browser".to_string(),
                stream_id: "stream".to_string(),
                state: stream,
                observed_at: None,
                valid_until: None,
                max_age_ms: 5_000,
                route_presentation: None,
                display_content: None,
            };
            assert_eq!(observation.observed_at, None);
            assert_eq!(observation.valid_until, None);
            assert_eq!(observation.display_content, None);
        }
    }

    #[test]
    fn cached_display_timestamp_wins_over_newer_route_timestamp() {
        assert_eq!(
            earliest_timestamp(
                Some("2026-08-10T10:00:05.000Z".to_string()),
                Some("2026-08-10T10:00:01.000Z".to_string()),
            ),
            Some("2026-08-10T10:00:01.000Z".to_string())
        );
    }
}
