use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::native::service_model::ServiceState;
use crate::runtime_profile::ManualRuntimeBrowser;

#[derive(Debug, Clone)]
pub(crate) struct StatusObservationRequest {
    pub(crate) service_state: ServiceState,
}

impl StatusObservationRequest {
    pub(crate) fn from_state(state: &ServiceState) -> Self {
        Self {
            service_state: state.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatusObservationState {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum StatusObservationSourceKind {
    #[serde(rename = "local_status_observation_adapter")]
    Local,
    #[serde(rename = "unavailable_status_observation_adapter")]
    Unavailable,
    #[serde(rename = "in_memory_status_observation_adapter")]
    InMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatusObservationComponentState {
    Observed,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatusViewStreamObservationState {
    Observed,
    TimedOut,
    Unsupported,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatusObservationErrorCode {
    DisplayProbeTimeout,
    DisplayProbeUnsupported,
    DisplayProbeUnavailable,
    DisplayProbeFailed,
    RuntimeProfileUnavailable,
    ProcessInventoryUnavailable,
    ConfiguredRouteUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatusRoutePresentationSource {
    RouteDescriptor,
    RetainedStream,
    ConfiguredClientUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusObservationError {
    pub(crate) code: StatusObservationErrorCode,
    pub(crate) subject: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusRoutePresentation {
    pub(crate) frame_url: String,
    pub(crate) external_url: String,
    pub(crate) source: StatusRoutePresentationSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusViewStreamObservation {
    pub(crate) browser_id: String,
    pub(crate) stream_id: String,
    pub(crate) state: StatusViewStreamObservationState,
    pub(crate) observed_at: Option<String>,
    pub(crate) valid_until: Option<String>,
    pub(crate) max_age_ms: u64,
    pub(crate) route_presentation: Option<StatusRoutePresentation>,
    pub(crate) display_content: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusObservationSnapshot {
    pub(crate) state: StatusObservationState,
    pub(crate) source: StatusObservationSourceKind,
    pub(crate) source_host_id: Option<String>,
    pub(crate) observed_at: Option<String>,
    pub(crate) valid_until: Option<String>,
    pub(crate) max_age_ms: u64,
    pub(crate) manual_browsers_state: StatusObservationComponentState,
    pub(crate) browser_process_state: StatusObservationComponentState,
    pub(crate) errors: Vec<StatusObservationError>,
    pub(crate) view_streams: Vec<StatusViewStreamObservation>,
    #[serde(skip)]
    pub(crate) manual_browsers: Vec<ManualRuntimeBrowser>,
    #[serde(skip)]
    pub(crate) browser_process_stats: BTreeMap<String, Value>,
}

impl StatusObservationSnapshot {
    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_source_host_id(self.source_host_id.as_deref())?;
        validate_time_window(
            "observations",
            self.observed_at.as_deref(),
            self.valid_until.as_deref(),
            self.max_age_ms,
        )?;

        match self.state {
            StatusObservationState::Unavailable => {
                if self.observed_at.is_some() || self.valid_until.is_some() {
                    return Err("unavailable observations require null timestamps".to_string());
                }
            }
            StatusObservationState::Complete | StatusObservationState::Partial => {
                if self.source_host_id.is_none()
                    || self.observed_at.is_none()
                    || self.valid_until.is_none()
                {
                    return Err(
                        "complete or partial observations require source host and timestamps"
                            .to_string(),
                    );
                }
            }
        }

        if self.state == StatusObservationState::Complete
            && (self.manual_browsers_state != StatusObservationComponentState::Observed
                || self.browser_process_state != StatusObservationComponentState::Observed
                || self
                    .view_streams
                    .iter()
                    .any(|stream| stream.state != StatusViewStreamObservationState::Observed)
                || !self.errors.is_empty())
        {
            return Err("complete observations contain an incomplete component".to_string());
        }

        for stream in &self.view_streams {
            if stream.browser_id.trim().is_empty() || stream.stream_id.trim().is_empty() {
                return Err("stream observations require browserId and streamId".to_string());
            }
            validate_time_window(
                &format!("stream {}:{}", stream.browser_id, stream.stream_id),
                stream.observed_at.as_deref(),
                stream.valid_until.as_deref(),
                stream.max_age_ms,
            )?;
            if stream.state == StatusViewStreamObservationState::Observed {
                if stream.observed_at.is_none()
                    || stream.valid_until.is_none()
                    || (stream.route_presentation.is_none() && stream.display_content.is_none())
                {
                    return Err(format!(
                        "observed stream {}:{} requires timestamps and observed content",
                        stream.browser_id, stream.stream_id
                    ));
                }
            } else if stream.observed_at.is_some()
                || stream.valid_until.is_some()
                || stream.route_presentation.is_some()
                || stream.display_content.is_some()
            {
                return Err(format!(
                    "non-observed stream {}:{} requires null observation values",
                    stream.browser_id, stream.stream_id
                ));
            }
        }
        Ok(())
    }
}

fn validate_source_host_id(source_host_id: Option<&str>) -> Result<(), String> {
    let Some(source_host_id) = source_host_id else {
        return Ok(());
    };
    let Some(digest) = source_host_id.strip_prefix("sha256:") else {
        return Err("sourceHostId must use the sha256 prefix".to_string());
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("sourceHostId must contain 64 lowercase hexadecimal characters".to_string());
    }
    Ok(())
}

fn validate_time_window(
    subject: &str,
    observed_at: Option<&str>,
    valid_until: Option<&str>,
    max_age_ms: u64,
) -> Result<(), String> {
    match (observed_at, valid_until) {
        (None, None) => Ok(()),
        (Some(observed_at), Some(valid_until)) => {
            let observed = DateTime::parse_from_rfc3339(observed_at)
                .map_err(|error| format!("{subject} observedAt is invalid: {error}"))?
                .with_timezone(&Utc);
            let valid = DateTime::parse_from_rfc3339(valid_until)
                .map_err(|error| format!("{subject} validUntil is invalid: {error}"))?
                .with_timezone(&Utc);
            if valid < observed {
                return Err(format!("{subject} validUntil precedes observedAt"));
            }
            let window = valid.signed_duration_since(observed).num_milliseconds();
            if window < 0 || u64::try_from(window).unwrap_or(u64::MAX) > max_age_ms {
                return Err(format!("{subject} freshness exceeds maxAgeMs"));
            }
            Ok(())
        }
        _ => Err(format!(
            "{subject} observedAt and validUntil must both be null or both be timestamps"
        )),
    }
}

#[async_trait]
pub(crate) trait StatusObservationSource: Send + Sync {
    async fn snapshot(&self, request: StatusObservationRequest) -> StatusObservationSnapshot;
}
