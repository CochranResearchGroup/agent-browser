//! Task-shaped product action for one configured desktop scene observation.
//!
//! Callers name the evidence surface and browser identity. Provider routes,
//! display names, native windows, coordinates, and presentation slots remain
//! internal to the Desktop Evidence Episode.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};

use super::desktop_evidence::{
    DesktopEpisodeAdapters, DesktopEpisodeOutcome, DesktopEpisodeRequest,
    DesktopEvidenceCoordinator, DesktopSceneSurface, EpisodeInput, EvidenceRequest,
};
use super::desktop_evidence_configured::{
    ConfiguredBlockedInputAdapter, ConfiguredDesktopFrameAdapter, ConfiguredEpisodeCleanupAdapter,
    ConfiguredEpisodeVerificationAdapter, ConfiguredExistingHandoffAdapter,
    ConfiguredPresentationSlotAdapter, ConfiguredSceneStagingAdapter, ConfiguredUnusedCdpAdapter,
    ConfiguredUnusedTriggerAdapter, ConfiguredWindowSemanticAdapter,
};
use super::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};

const ACTION: &str = "desktop_evidence_observe";
const STACKING_OR_OCCLUSION: &str = "stacking_or_occlusion";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfiguredObservationRequest {
    browser_id: String,
    episode_id: String,
    include_frame: bool,
}

pub(crate) async fn handle_desktop_evidence_observe(command: &Value) -> Result<Value, String> {
    let request = parse_request(command)?;
    tokio::task::spawn_blocking(move || {
        let repository = LockedServiceStateRepository::<JsonServiceStateStore>::default_json()?;
        run_configured_observation(request, repository)
    })
    .await
    .map_err(|_| "desktop_evidence_observe task failed".to_string())?
}

/// Remove response-only pixels and provider readiness details before durable
/// job, stream, dashboard, or incident projection.
pub(crate) fn redact_desktop_evidence_stream_result(result: &Value) -> Value {
    let Some(record) = result.as_object() else {
        return Value::Null;
    };
    let mut redacted = serde_json::Map::new();
    for field in ["ok", "action", "evidenceSurface", "episode"] {
        if let Some(value) = record.get(field) {
            redacted.insert(field.to_string(), value.clone());
        }
    }
    for (field, allowed) in [
        (
            "context",
            &[
                "contextId",
                "schemaVersion",
                "browserId",
                "sessionName",
                "profileId",
                "displayAllocationId",
                "streamId",
                "routeId",
                "coordinateSpace",
                "width",
                "height",
                "scaleFactor",
                "geometryEpoch",
            ][..],
        ),
        (
            "frameReceipt",
            &[
                "frameId",
                "schemaVersion",
                "contextId",
                "sequence",
                "capturedAt",
                "width",
                "height",
                "scaleFactor",
                "geometryEpoch",
                "mimeType",
                "byteLength",
                "sha256",
                "freshness",
                "retention",
                "persisted",
            ][..],
        ),
    ] {
        if let Some(source) = record.get(field).and_then(Value::as_object) {
            let value = allowed
                .iter()
                .filter_map(|key| {
                    source
                        .get(*key)
                        .cloned()
                        .map(|value| ((*key).to_string(), value))
                })
                .collect();
            redacted.insert(field.to_string(), Value::Object(value));
        }
    }
    Value::Object(redacted)
}

fn parse_request(command: &Value) -> Result<ConfiguredObservationRequest, String> {
    const ALLOWED_FIELDS: &[&str] = &[
        "action",
        "id",
        "browserId",
        "sessionName",
        "episodeId",
        "evidenceSurface",
        "includeFrame",
        "jobTimeoutMs",
        "serviceName",
        "agentName",
        "taskName",
        "requestId",
        "callerId",
        "requestPrincipalSource",
    ];
    if let Some(field) = command.as_object().and_then(|record| {
        record
            .keys()
            .find(|field| !ALLOWED_FIELDS.contains(&field.as_str()))
    }) {
        return Err(format!("{ACTION} does not accept {field}"));
    }
    if command.get("action").and_then(Value::as_str) != Some(ACTION) {
        return Err(format!("{ACTION} requires action {ACTION}"));
    }
    let browser_id = required_string(command, "browserId")?;
    let evidence_surface = required_string(command, "evidenceSurface")?;
    if evidence_surface != STACKING_OR_OCCLUSION {
        return Err(format!(
            "{ACTION} currently requires evidenceSurface {STACKING_OR_OCCLUSION}"
        ));
    }
    let episode_id = ["episodeId", "requestId"]
        .into_iter()
        .find_map(|field| optional_string(command, field))
        .ok_or_else(|| format!("{ACTION} requires episodeId, operationId, or requestId"))?;
    let include_frame = command
        .get("includeFrame")
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| format!("{ACTION} includeFrame must be a boolean"))
        })
        .transpose()?
        .unwrap_or(false);
    Ok(ConfiguredObservationRequest {
        browser_id,
        episode_id,
        include_frame,
    })
}

fn run_configured_observation<R>(
    request: ConfiguredObservationRequest,
    repository: R,
) -> Result<Value, String>
where
    R: ServiceStateRepository + Clone + 'static,
{
    let initial_state = repository.load_snapshot()?;
    let admitted_maximum = initial_state
        .presentation_capacity
        .as_ref()
        .map(|capacity| capacity.config.hard_maximum)
        .ok_or_else(|| "presentation_capacity_unavailable".to_string())?;
    let mut cdp = ConfiguredUnusedCdpAdapter;
    let mut slots = ConfiguredPresentationSlotAdapter::new(
        repository.clone(),
        request.episode_id.clone(),
        admitted_maximum,
    );
    let mut staging =
        ConfiguredSceneStagingAdapter::new(repository.clone(), request.episode_id.clone());
    let mut windows =
        ConfiguredWindowSemanticAdapter::new(repository.clone(), request.episode_id.clone());
    let mut trigger = ConfiguredUnusedTriggerAdapter;
    let mut frames = ConfiguredDesktopFrameAdapter::new();
    let mut input = ConfiguredBlockedInputAdapter;
    let mut verification =
        ConfiguredEpisodeVerificationAdapter::new(repository, request.episode_id.clone());
    let mut handoff =
        ConfiguredExistingHandoffAdapter::from_state(&initial_state, &request.browser_id);
    let mut cleanup = ConfiguredEpisodeCleanupAdapter;
    let outcome = DesktopEvidenceCoordinator::run(
        DesktopEpisodeRequest {
            episode_id: request.episode_id,
            browser_id: request.browser_id,
            evidence: EvidenceRequest::desktop_scene(DesktopSceneSurface::StackingOrOcclusion),
            input: EpisodeInput::None,
        },
        &mut DesktopEpisodeAdapters {
            cdp: &mut cdp,
            slots: &mut slots,
            staging: &mut staging,
            windows: &mut windows,
            trigger: &mut trigger,
            frames: &mut frames,
            input: &mut input,
            verification: &mut verification,
            handoff: &mut handoff,
            cleanup: &mut cleanup,
        },
    );
    let capture = frames.take_capture();
    if matches!(outcome, DesktopEpisodeOutcome::Desktop { .. }) && capture.is_none() {
        return Err("desktop_evidence_capture_receipt_missing".to_string());
    }
    let mut response = json!({
        "ok": true,
        "action": ACTION,
        "evidenceSurface": STACKING_OR_OCCLUSION,
        "episode": outcome,
    });
    if let Some(capture) = capture {
        response["context"] = serde_json::to_value(capture.context)
            .map_err(|error| format!("desktop_evidence_response_failed: {error}"))?;
        response["frameReceipt"] = serde_json::to_value(capture.frame_receipt)
            .map_err(|error| format!("desktop_evidence_response_failed: {error}"))?;
        if request.include_frame {
            response["frameBase64"] = Value::String(BASE64_STANDARD.encode(capture.image_bytes));
        }
    }
    Ok(response)
}

fn required_string(command: &Value, field: &str) -> Result<String, String> {
    optional_string(command, field).ok_or_else(|| format!("{ACTION} requires nonempty {field}"))
}

fn optional_string(command: &Value, field: &str) -> Option<String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_task_shaped_and_accepts_no_provider_plumbing() {
        let request = parse_request(&json!({
            "action": ACTION,
            "browserId": "browser-1",
            "episodeId": "episode-1",
            "evidenceSurface": STACKING_OR_OCCLUSION,
            "includeFrame": true,
        }))
        .unwrap();

        assert_eq!(request.browser_id, "browser-1");
        assert_eq!(request.episode_id, "episode-1");
        assert!(request.include_frame);

        for field in [
            "displayName",
            "routeId",
            "providerUrl",
            "windowId",
            "coordinates",
            "cdpUrl",
            "params",
        ] {
            let mut unsafe_request = json!({
                "action": ACTION,
                "browserId": "browser-1",
                "episodeId": "episode-1",
                "evidenceSurface": STACKING_OR_OCCLUSION,
            });
            unsafe_request[field] = json!("caller-controlled");
            let error = parse_request(&unsafe_request).unwrap_err();
            assert_eq!(error, format!("{ACTION} does not accept {field}"));
        }
    }

    #[test]
    fn request_rejects_prompt_perception_and_generic_cdp_failure() {
        for surface in ["browser_external_prompt", "cdp_failure"] {
            let error = parse_request(&json!({
                "action": ACTION,
                "browserId": "browser-1",
                "episodeId": "episode-1",
                "evidenceSurface": surface,
            }))
            .unwrap_err();
            assert!(error.contains(STACKING_OR_OCCLUSION));
        }
    }

    #[test]
    fn request_defaults_to_receipts_without_returning_pixels() {
        let request = parse_request(&json!({
            "action": ACTION,
            "browserId": "browser-1",
            "requestId": "request-1",
            "evidenceSurface": STACKING_OR_OCCLUSION,
        }))
        .unwrap();

        assert!(!request.include_frame);
        assert_eq!(request.episode_id, "request-1");
    }

    #[test]
    fn stream_redaction_removes_pixels_and_provider_readiness() {
        let redacted = redact_desktop_evidence_stream_result(&json!({
            "ok": true,
            "action": ACTION,
            "evidenceSurface": STACKING_OR_OCCLUSION,
            "episode": { "outcome": "desktop" },
            "context": {
                "contextId": "context-1",
                "browserId": "browser-1",
                "readinessEvidence": { "private": true },
                "displayName": ":101"
            },
            "frameReceipt": {
                "frameId": "frame-1",
                "sha256": "safe-hash",
                "providerVersion": "private-provider"
            },
            "frameBase64": "PRIVATE_PIXELS"
        }));
        let encoded = serde_json::to_string(&redacted).unwrap();

        assert!(!encoded.contains("PRIVATE_PIXELS"));
        assert!(!encoded.contains("private-provider"));
        assert!(!encoded.contains(":101"));
        assert!(!encoded.contains("readinessEvidence"));
        assert_eq!(redacted["frameReceipt"]["sha256"], "safe-hash");
    }
}
