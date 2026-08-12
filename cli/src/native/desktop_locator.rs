//! Deterministic, observe-only location of repository-owned desktop fixtures.
//!
//! The module accepts only a capture produced by `desktop_capture`, validates
//! every binding before detector work, and returns stable integer-scored
//! candidates. Source and visualization pixels remain response-only.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ColorType, ImageEncoder, ImageFormat, ImageReader, Limits, Rgba, RgbaImage};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Cursor;

use super::desktop_capture::{
    capture_configured_desktop_frame, DesktopCaptureRequest, DesktopCaptureResult, DesktopContext,
    FrameReceipt, DEFAULT_MAX_BYTES,
};

const OBSERVATION_SCHEMA_VERSION: &str = "v1";
const PROFILE_VERSION: &str = "p110-v1";
const LOCATOR_ID: &str = "p110-control-v1";
const TARGET_CLASS: &str = "synthetic_verification_control";
const COORDINATE_SPACE: &str = "desktop_physical_pixels";
const NORMALIZATION_VERSION: &str = "rgba8-srgb-integer-v1";
const TEMPLATE_DETECTOR_ID: &str = "rgba-template";
const GEOMETRY_DETECTOR_ID: &str = "desktop-geometry";
const OCR_DETECTOR_ID: &str = "normalized-token-fusion";
const TEMPLATE_THRESHOLD: u32 = 9_900;
const AMBIGUITY_MARGIN: u32 = 250;
const MAX_LOCATOR_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_TEMPLATE_EVALUATIONS: u32 = 4_096;
const MAX_VISUALIZATION_BYTES: usize = 4 * 1024 * 1024;

pub(crate) const DEFAULT_MAX_CANDIDATES: usize = 8;
pub(crate) const HARD_MAX_CANDIDATES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopLocatorError {
    code: &'static str,
    message: String,
}

impl DesktopLocatorError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for DesktopLocatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DesktopLocatorError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixelBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelBounds {
    fn center(self) -> PixelCenter {
        PixelCenter {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixelCenter {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OcrTokenEvidence {
    pub token_id: String,
    pub bounds: PixelBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OcrEvidence {
    pub provider_version: String,
    pub evidence_hash: String,
    pub tokens: Vec<OcrTokenEvidence>,
}

pub(crate) trait OcrEvidenceProvider: Send + Sync {
    fn evidence(
        &self,
        image: &RgbaImage,
        profile: &LocatorProfile,
    ) -> Result<OcrEvidence, DesktopLocatorError>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundFrame {
    pub context: DesktopContext,
    pub frame_receipt: FrameReceipt,
    pub image_bytes: Vec<u8>,
}

impl From<DesktopCaptureResult> for BoundFrame {
    fn from(capture: DesktopCaptureResult) -> Self {
        Self {
            context: capture.context,
            frame_receipt: capture.frame_receipt,
            image_bytes: capture.image_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetectorReceipt {
    pub detector_id: &'static str,
    pub version: &'static str,
    pub evidence_sha256: String,
    pub normalization_version: &'static str,
    pub integer_parameters: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CandidateEvidence {
    pub detector_id: &'static str,
    pub evidence_id: String,
    pub score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocatorCandidate {
    pub candidate_id: String,
    pub target_class: &'static str,
    pub rank: usize,
    pub bounds: PixelBounds,
    pub center: PixelCenter,
    pub score: u32,
    pub supporting_evidence: Vec<CandidateEvidence>,
    pub decoy_evidence: Vec<&'static str>,
    pub ambiguity_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualizationReceipt {
    pub schema_version: &'static str,
    pub mime_type: &'static str,
    pub byte_length: usize,
    #[serde(rename = "sha256")]
    pub content_sha256: String,
    pub retention: &'static str,
    pub persisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Observation {
    pub observation_id: String,
    pub schema_version: &'static str,
    pub context_id: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub geometry_epoch: String,
    pub coordinate_space: &'static str,
    pub locator_id: &'static str,
    pub profile_version: &'static str,
    pub profile_sha256: String,
    pub target_class: &'static str,
    pub detector_receipts: Vec<DetectorReceipt>,
    pub status: &'static str,
    pub selected_candidate_id: Option<String>,
    pub candidates: Vec<LocatorCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visualization_receipt: Option<VisualizationReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesktopLocateResult {
    pub context: DesktopContext,
    pub frame_receipt: FrameReceipt,
    pub observation: Observation,
    pub visualization_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    Light,
    Dark,
}

impl Theme {
    fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn background(self) -> Rgba<u8> {
        match self {
            Self::Light => Rgba([245, 245, 245, 255]),
            Self::Dark => Rgba([24, 24, 27, 255]),
        }
    }

    fn foreground(self) -> Rgba<u8> {
        match self {
            Self::Light => Rgba([30, 41, 59, 255]),
            Self::Dark => Rgba([226, 232, 240, 255]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocatorProfile {
    locator_id: &'static str,
    profile_sha256: String,
    required_token_id: &'static str,
}

fn locator_profile(locator_id: &str) -> Result<LocatorProfile, DesktopLocatorError> {
    if locator_id != LOCATOR_ID {
        return Err(DesktopLocatorError::new(
            "desktop_locator_not_found",
            format!("locator profile {locator_id} is not repository-owned"),
        ));
    }
    Ok(LocatorProfile {
        locator_id: LOCATOR_ID,
        profile_sha256: digest_text(
            "p110-control-v1\x00p110-v1\x00light,dark\x001000,1250,1500\x009900\x00250\x00verify-control",
        ),
        required_token_id: "verify-control",
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocateRequest {
    browser_id: String,
    session_name: Option<String>,
    locator_id: String,
    max_candidates: usize,
    include_visualization: bool,
}

/// Locate a profile-owned synthetic target in one exactly bound captured frame.
pub(crate) fn locate_bound_frame(
    frame: BoundFrame,
    locator_id: &str,
    max_candidates: usize,
    include_visualization: bool,
    ocr_provider: &dyn OcrEvidenceProvider,
) -> Result<DesktopLocateResult, DesktopLocatorError> {
    if max_candidates == 0 || max_candidates > HARD_MAX_CANDIDATES {
        return Err(DesktopLocatorError::new(
            "desktop_locator_unsupported",
            format!("maxCandidates must be between 1 and {HARD_MAX_CANDIDATES}"),
        ));
    }
    let profile = locator_profile(locator_id)?;
    let image = validate_bound_frame(&frame)?;
    let scale_millis = supported_scale(frame.context.scale_factor)?;
    let theme = detect_theme(&image)?;
    let ocr = ocr_provider.evidence(&image, &profile)?;
    validate_ocr_evidence(&ocr, image.width(), image.height())?;

    let template_size = scaled(12, scale_millis)?;
    let raw_matches = scan_template(&image, theme, template_size)?;
    let mut candidates = Vec::new();
    for template_match in raw_matches {
        let Some(token) = corroborating_token(
            template_match.bounds,
            &ocr.tokens,
            profile.required_token_id,
            scale_millis,
        ) else {
            continue;
        };
        let score = (template_match.score * 7 + 10_000 * 3) / 10;
        if score < TEMPLATE_THRESHOLD {
            continue;
        }
        let candidate_id = format!(
            "desktop-candidate-{}",
            &digest_text(&format!(
                "{}\0{}\0{}\0{}\0{}\0{}",
                frame.frame_receipt.frame_id,
                profile.locator_id,
                template_match.bounds.x,
                template_match.bounds.y,
                template_match.bounds.width,
                template_match.bounds.height
            ))[..24]
        );
        candidates.push(LocatorCandidate {
            candidate_id,
            target_class: TARGET_CLASS,
            rank: 0,
            bounds: template_match.bounds,
            center: template_match.bounds.center(),
            score,
            supporting_evidence: vec![
                CandidateEvidence {
                    detector_id: GEOMETRY_DETECTOR_ID,
                    evidence_id: format!("in-bounds:{}x{}", image.width(), image.height()),
                    score: 10_000,
                },
                CandidateEvidence {
                    detector_id: TEMPLATE_DETECTOR_ID,
                    evidence_id: template_match.evidence_id,
                    score: template_match.score,
                },
                CandidateEvidence {
                    detector_id: OCR_DETECTOR_ID,
                    evidence_id: digest_text(&format!(
                        "{}\0{}\0{}\0{}\0{}",
                        token.token_id,
                        token.bounds.x,
                        token.bounds.y,
                        token.bounds.width,
                        token.bounds.height
                    )),
                    score: 10_000,
                },
            ],
            decoy_evidence: vec!["template_threshold_met", "required_token_corroborated"],
            ambiguity_evidence: Vec::new(),
        });
    }
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.bounds.y.cmp(&right.bounds.y))
            .then_with(|| left.bounds.x.cmp(&right.bounds.x))
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    candidates.dedup_by(|left, right| overlap_ratio(left.bounds, right.bounds) >= 8_000);
    let ambiguity_gap = candidates
        .first()
        .zip(candidates.get(1))
        .map(|(leader, runner_up)| leader.score.saturating_sub(runner_up.score));
    let status = if candidates.is_empty() {
        "not_found"
    } else if ambiguity_gap.is_some_and(|gap| gap < AMBIGUITY_MARGIN) {
        let gap = ambiguity_gap.expect("checked above");
        candidates[0]
            .ambiguity_evidence
            .push(format!("runner_up_gap:{gap}"));
        candidates[1]
            .ambiguity_evidence
            .push(format!("leader_gap:{gap}"));
        "ambiguous"
    } else {
        "matched"
    };
    candidates.truncate(max_candidates);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = index + 1;
    }
    let selected_candidate_id = (status == "matched").then(|| candidates[0].candidate_id.clone());
    let detector_receipts =
        detector_receipts(&frame, &profile, &ocr, theme, scale_millis, template_size);
    let observation_id = format!(
        "desktop-observation-{}",
        &digest_text(&format!(
            "{}\0{}\0{}\0{}\0{}",
            frame.context.context_id,
            frame.frame_receipt.frame_id,
            profile.profile_sha256,
            status,
            candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ))[..24]
    );
    let visualization_bytes = if include_visualization {
        Some(render_visualization(&image, &candidates, status)?)
    } else {
        None
    };
    let visualization_receipt = visualization_bytes
        .as_ref()
        .map(|bytes| VisualizationReceipt {
            schema_version: "v1",
            mime_type: "image/png",
            byte_length: bytes.len(),
            content_sha256: digest_bytes(bytes),
            retention: "ephemeral",
            persisted: false,
        });
    Ok(DesktopLocateResult {
        context: frame.context.clone(),
        frame_receipt: frame.frame_receipt.clone(),
        observation: Observation {
            observation_id,
            schema_version: OBSERVATION_SCHEMA_VERSION,
            context_id: frame.context.context_id,
            frame_id: frame.frame_receipt.frame_id,
            frame_sha256: frame.frame_receipt.content_sha256,
            geometry_epoch: frame.context.geometry_epoch,
            coordinate_space: COORDINATE_SPACE,
            locator_id: profile.locator_id,
            profile_version: PROFILE_VERSION,
            profile_sha256: profile.profile_sha256,
            target_class: TARGET_CLASS,
            detector_receipts,
            status,
            selected_candidate_id,
            candidates,
            visualization_receipt,
        },
        visualization_bytes,
    })
}

/// Capture and locate in one blocking transaction. The request never accepts
/// caller pixels, paths, display identities, coordinates, or detector tuning.
pub(crate) async fn handle_desktop_locate(cmd: &Value) -> Result<Value, String> {
    let request = parse_request(cmd).map_err(|error| error.to_string())?;
    let result = tokio::task::spawn_blocking(move || {
        let capture = capture_configured_desktop_frame(DesktopCaptureRequest {
            browser_id: request.browser_id,
            session_name: request.session_name,
            max_bytes: DEFAULT_MAX_BYTES,
        })
        .map_err(|error| error.to_string())?;
        locate_bound_frame(
            capture.into(),
            &request.locator_id,
            request.max_candidates,
            request.include_visualization,
            &PinnedGlyphOcrProvider,
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "desktop_locator_detector_failed: desktop locate task failed".to_string())??;

    let mut response = json!({
        "ok": true,
        "action": "desktop_locate",
        "context": result.context,
        "frameReceipt": result.frame_receipt,
        "observation": result.observation,
    });
    if let Some(bytes) = result.visualization_bytes {
        response["visualizationBase64"] = Value::String(BASE64_STANDARD.encode(bytes));
    }
    Ok(response)
}

/// Remove response-only visualization pixels before long-lived projection.
pub(crate) fn redact_desktop_locate_stream_result(data: &Value) -> Value {
    let mut redacted = data.clone();
    if let Some(record) = redacted.as_object_mut() {
        if record.remove("visualizationBase64").is_some() {
            record.insert(
                "visualizationPayload".to_string(),
                Value::String("response_only".to_string()),
            );
        }
    }
    redacted
}

fn parse_request(cmd: &Value) -> Result<LocateRequest, DesktopLocatorError> {
    for forbidden in [
        "imageBase64",
        "frameId",
        "contextId",
        "coordinates",
        "template",
        "ocr",
        "assetPath",
        "outputPath",
        "displayName",
        "providerUrl",
        "crop",
        "threshold",
        "detectors",
    ] {
        if cmd.get(forbidden).is_some() {
            return Err(DesktopLocatorError::new(
                "desktop_locator_frame_mismatch",
                format!("desktop_locate does not accept caller-controlled {forbidden}"),
            ));
        }
    }
    let locator = cmd
        .get("locator")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            DesktopLocatorError::new(
                "desktop_locator_not_found",
                "desktop_locate requires locator",
            )
        })?;
    for key in locator.keys() {
        if key != "locatorId" && key != "maxCandidates" {
            return Err(DesktopLocatorError::new(
                "desktop_locator_unsupported",
                format!("desktop locator parameter {key} is not supported"),
            ));
        }
    }
    let browser_id = cmd
        .get("browserId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            DesktopLocatorError::new(
                "desktop_locator_frame_mismatch",
                "desktop_locate requires browserId",
            )
        })?;
    let locator_id = locator
        .get("locatorId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            DesktopLocatorError::new(
                "desktop_locator_not_found",
                "desktop_locate requires locator.locatorId",
            )
        })?;
    locator_profile(locator_id)?;
    let max_candidates = locator
        .get("maxCandidates")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_MAX_CANDIDATES as u64);
    if max_candidates == 0 || max_candidates > HARD_MAX_CANDIDATES as u64 {
        return Err(DesktopLocatorError::new(
            "desktop_locator_unsupported",
            format!("maxCandidates must be between 1 and {HARD_MAX_CANDIDATES}"),
        ));
    }
    Ok(LocateRequest {
        browser_id: browser_id.to_string(),
        session_name: cmd
            .get("sessionName")
            .and_then(Value::as_str)
            .map(str::to_string),
        locator_id: locator_id.to_string(),
        max_candidates: max_candidates as usize,
        include_visualization: cmd
            .get("includeVisualization")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn validate_bound_frame(frame: &BoundFrame) -> Result<RgbaImage, DesktopLocatorError> {
    let mismatch = || {
        DesktopLocatorError::new(
            "desktop_locator_frame_mismatch",
            "desktop context, receipt, and captured frame do not describe one exact frame",
        )
    };
    if frame.context.schema_version != "v1"
        || frame.frame_receipt.schema_version != "v1"
        || frame.context.context_id != frame.frame_receipt.context_id
        || frame.context.width != frame.frame_receipt.width
        || frame.context.height != frame.frame_receipt.height
        || scale_millis(frame.context.scale_factor)
            != scale_millis(frame.frame_receipt.scale_factor)
        || frame.context.geometry_epoch != frame.frame_receipt.geometry_epoch
        || frame.context.coordinate_space != COORDINATE_SPACE
        || frame.frame_receipt.mime_type != "image/png"
        || frame.frame_receipt.byte_length != frame.image_bytes.len()
        || frame.frame_receipt.content_sha256 != digest_bytes(&frame.image_bytes)
        || frame.frame_receipt.freshness != "fresh_capture"
        || frame.frame_receipt.retention != "ephemeral"
        || frame.frame_receipt.persisted
    {
        return Err(mismatch());
    }
    let mut reader = ImageReader::with_format(Cursor::new(&frame.image_bytes), ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(frame.context.width);
    limits.max_image_height = Some(frame.context.height);
    limits.max_alloc = Some(MAX_LOCATOR_PIXELS * 4);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| {
        DesktopLocatorError::new(
            "desktop_locator_invalid_image",
            "captured frame is not a bounded PNG",
        )
    })?;
    if u64::from(image.width()) * u64::from(image.height()) > MAX_LOCATOR_PIXELS {
        return Err(DesktopLocatorError::new(
            "desktop_locator_invalid_image",
            "captured frame exceeds the locator pixel budget",
        ));
    }
    if image.width() != frame.context.width || image.height() != frame.context.height {
        return Err(mismatch());
    }
    Ok(image.to_rgba8())
}

fn scale_millis(scale: f64) -> Option<u32> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let millis = (scale * 1000.0).round();
    ((scale * 1000.0 - millis).abs() <= 0.000_001).then_some(millis as u32)
}

fn supported_scale(scale: f64) -> Result<u32, DesktopLocatorError> {
    let scale = scale_millis(scale).ok_or_else(|| {
        DesktopLocatorError::new(
            "desktop_locator_unsupported",
            "desktop scale is not an exact finite fixed-point value",
        )
    })?;
    if ![1000, 1250, 1500].contains(&scale) {
        return Err(DesktopLocatorError::new(
            "desktop_locator_unsupported",
            format!("desktop scale {scale} is not supported by {LOCATOR_ID}"),
        ));
    }
    Ok(scale)
}

fn scaled(value: u32, scale_millis: u32) -> Result<u32, DesktopLocatorError> {
    value
        .checked_mul(scale_millis)
        .and_then(|scaled| scaled.checked_add(500))
        .map(|scaled| scaled / 1000)
        .filter(|scaled| *scaled > 0)
        .ok_or_else(|| {
            DesktopLocatorError::new(
                "desktop_locator_unsupported",
                "locator scale transform overflowed",
            )
        })
}

fn detect_theme(image: &RgbaImage) -> Result<Theme, DesktopLocatorError> {
    let corners = [
        image.get_pixel(0, 0),
        image.get_pixel(image.width() - 1, 0),
        image.get_pixel(0, image.height() - 1),
        image.get_pixel(image.width() - 1, image.height() - 1),
    ];
    let luminance: u32 = corners
        .iter()
        .map(|pixel| u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2]))
        .sum::<u32>()
        / 12;
    if luminance >= 180 {
        Ok(Theme::Light)
    } else if luminance <= 80 {
        Ok(Theme::Dark)
    } else {
        Err(DesktopLocatorError::new(
            "desktop_locator_unsupported",
            "captured frame theme is not supported by the locator profile",
        ))
    }
}

#[derive(Debug, Clone)]
struct TemplateMatch {
    bounds: PixelBounds,
    score: u32,
    evidence_id: String,
}

fn scan_template(
    image: &RgbaImage,
    theme: Theme,
    size: u32,
) -> Result<Vec<TemplateMatch>, DesktopLocatorError> {
    if size > image.width() || size > image.height() {
        return Ok(Vec::new());
    }
    let mut matches = Vec::new();
    let foreground = theme.foreground();
    let mut evaluated = 0_u32;
    for y in 0..=image.height() - size {
        for x in 0..=image.width() - size {
            if image.get_pixel(x, y) != &foreground
                || image.get_pixel(x + size - 1, y) != &foreground
                || image.get_pixel(x, y + size - 1) != &foreground
                || image.get_pixel(x + size - 1, y + size - 1) != &foreground
            {
                continue;
            }
            evaluated += 1;
            if evaluated > MAX_TEMPLATE_EVALUATIONS {
                return Err(DesktopLocatorError::new(
                    "desktop_locator_detector_failed",
                    "template detector exceeded its fixed evaluation budget",
                ));
            }
            let score = template_score(
                image,
                theme,
                PixelBounds {
                    x,
                    y,
                    width: size,
                    height: size,
                },
            );
            if score >= TEMPLATE_THRESHOLD {
                let bounds = PixelBounds {
                    x,
                    y,
                    width: size,
                    height: size,
                };
                matches.push(TemplateMatch {
                    bounds,
                    score,
                    evidence_id: digest_text(&format!(
                        "{}\0{}\0{}\0{}\0{}",
                        theme.id(),
                        x,
                        y,
                        size,
                        score
                    )),
                });
            }
        }
    }
    Ok(matches)
}

fn template_score(image: &RgbaImage, theme: Theme, bounds: PixelBounds) -> u32 {
    let mut difference = 0_u64;
    let mut channels = 0_u64;
    for offset_y in 0..bounds.height {
        for offset_x in 0..bounds.width {
            let border = offset_x == 0
                || offset_y == 0
                || offset_x + 1 == bounds.width
                || offset_y + 1 == bounds.height;
            let expected = if border {
                theme.foreground()
            } else {
                theme.background()
            };
            let actual = image.get_pixel(bounds.x + offset_x, bounds.y + offset_y);
            for channel in 0..3 {
                difference += u64::from(actual[channel].abs_diff(expected[channel]));
                channels += 1;
            }
        }
    }
    let maximum = channels * 255;
    10_000_u32.saturating_sub(((difference * 10_000) / maximum) as u32)
}

fn corroborating_token<'a>(
    candidate: PixelBounds,
    tokens: &'a [OcrTokenEvidence],
    required_id: &str,
    scale_millis: u32,
) -> Option<&'a OcrTokenEvidence> {
    let maximum_gap = scaled(8, scale_millis).ok()?;
    tokens.iter().find(|token| {
        token.token_id == required_id
            && token.bounds.x >= candidate.x + candidate.width
            && token.bounds.x <= candidate.x + candidate.width + maximum_gap
            && ranges_overlap(
                candidate.y,
                candidate.y + candidate.height,
                token.bounds.y,
                token.bounds.y + token.bounds.height,
            )
    })
}

fn ranges_overlap(left_start: u32, left_end: u32, right_start: u32, right_end: u32) -> bool {
    left_start < right_end && right_start < left_end
}

fn overlap_ratio(left: PixelBounds, right: PixelBounds) -> u32 {
    let intersection_width = (left.x + left.width)
        .min(right.x + right.width)
        .saturating_sub(left.x.max(right.x));
    let intersection_height = (left.y + left.height)
        .min(right.y + right.height)
        .saturating_sub(left.y.max(right.y));
    let intersection = u64::from(intersection_width) * u64::from(intersection_height);
    let union = u64::from(left.width) * u64::from(left.height)
        + u64::from(right.width) * u64::from(right.height)
        - intersection;
    if union == 0 {
        0
    } else {
        ((intersection * 10_000) / union) as u32
    }
}

fn validate_ocr_evidence(
    evidence: &OcrEvidence,
    width: u32,
    height: u32,
) -> Result<(), DesktopLocatorError> {
    if !["fixture-ocr-v1", "pinned-glyph-v1"].contains(&evidence.provider_version.as_str())
        || evidence.evidence_hash.len() != 64
        || !evidence
            .evidence_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || evidence.tokens.len() > 256
    {
        return Err(DesktopLocatorError::new(
            "desktop_locator_detector_failed",
            "OCR provider returned malformed or oversized normalized evidence",
        ));
    }
    if evidence.tokens.iter().any(|token| {
        token.token_id.trim().is_empty()
            || token.token_id.len() > 64
            || token.bounds.width == 0
            || token.bounds.height == 0
            || token.bounds.x.checked_add(token.bounds.width).is_none()
            || token.bounds.y.checked_add(token.bounds.height).is_none()
            || token.bounds.x + token.bounds.width > width
            || token.bounds.y + token.bounds.height > height
    }) {
        return Err(DesktopLocatorError::new(
            "desktop_locator_detector_failed",
            "OCR provider returned out-of-bounds normalized evidence",
        ));
    }
    Ok(())
}

fn detector_receipts(
    frame: &BoundFrame,
    profile: &LocatorProfile,
    ocr: &OcrEvidence,
    theme: Theme,
    scale_millis: u32,
    template_size: u32,
) -> Vec<DetectorReceipt> {
    let mut geometry_parameters = BTreeMap::new();
    geometry_parameters.insert("frameWidth".to_string(), frame.context.width);
    geometry_parameters.insert("frameHeight".to_string(), frame.context.height);
    geometry_parameters.insert("scaleMillis".to_string(), scale_millis);
    let mut template_parameters = BTreeMap::new();
    template_parameters.insert("templateSize".to_string(), template_size);
    template_parameters.insert("threshold".to_string(), TEMPLATE_THRESHOLD);
    template_parameters.insert("maximumEvaluations".to_string(), MAX_TEMPLATE_EVALUATIONS);
    let mut ocr_parameters = BTreeMap::new();
    ocr_parameters.insert("maximumTokens".to_string(), 256);
    ocr_parameters.insert("ambiguityMargin".to_string(), AMBIGUITY_MARGIN);
    vec![
        DetectorReceipt {
            detector_id: GEOMETRY_DETECTOR_ID,
            version: "v1",
            evidence_sha256: digest_text(&format!(
                "{}\0{}\0{}",
                frame.context.geometry_epoch, frame.context.width, frame.context.height
            )),
            normalization_version: NORMALIZATION_VERSION,
            integer_parameters: geometry_parameters,
        },
        DetectorReceipt {
            detector_id: TEMPLATE_DETECTOR_ID,
            version: "v1",
            evidence_sha256: digest_text(&format!(
                "{}\0{}\0{}\0{}",
                profile.profile_sha256,
                theme.id(),
                scale_millis,
                frame.frame_receipt.content_sha256
            )),
            normalization_version: NORMALIZATION_VERSION,
            integer_parameters: template_parameters,
        },
        DetectorReceipt {
            detector_id: OCR_DETECTOR_ID,
            version: "v1",
            evidence_sha256: ocr.evidence_hash.clone(),
            normalization_version: NORMALIZATION_VERSION,
            integer_parameters: ocr_parameters,
        },
    ]
}

fn render_visualization(
    source: &RgbaImage,
    candidates: &[LocatorCandidate],
    status: &str,
) -> Result<Vec<u8>, DesktopLocatorError> {
    let mut image = source.clone();
    let color = if status == "matched" {
        Rgba([22, 163, 74, 255])
    } else {
        Rgba([234, 88, 12, 255])
    };
    for candidate in candidates {
        draw_bounds(&mut image, candidate.bounds, color)?;
    }
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, FilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|_| {
            DesktopLocatorError::new(
                "desktop_locator_visualization_failed",
                "bounded visualization encoding failed",
            )
        })?;
    if bytes.len() > MAX_VISUALIZATION_BYTES {
        return Err(DesktopLocatorError::new(
            "desktop_locator_visualization_failed",
            "bounded visualization exceeds the response budget",
        ));
    }
    Ok(bytes)
}

fn draw_bounds(
    image: &mut RgbaImage,
    bounds: PixelBounds,
    color: Rgba<u8>,
) -> Result<(), DesktopLocatorError> {
    let right = bounds.x.checked_add(bounds.width).ok_or_else(|| {
        DesktopLocatorError::new(
            "desktop_locator_visualization_failed",
            "visualization coordinate overflow",
        )
    })?;
    let bottom = bounds.y.checked_add(bounds.height).ok_or_else(|| {
        DesktopLocatorError::new(
            "desktop_locator_visualization_failed",
            "visualization coordinate overflow",
        )
    })?;
    if bounds.width == 0 || bounds.height == 0 || right > image.width() || bottom > image.height() {
        return Err(DesktopLocatorError::new(
            "desktop_locator_visualization_failed",
            "visualization candidate is out of bounds",
        ));
    }
    for x in bounds.x..right {
        image.put_pixel(x, bounds.y, color);
        image.put_pixel(x, bottom - 1, color);
    }
    for y in bounds.y..bottom {
        image.put_pixel(bounds.x, y, color);
        image.put_pixel(right - 1, y, color);
    }
    Ok(())
}

/// Synthetic-glyph adapter used only by the repository-owned PoC profile.
/// It never shells out, writes files, or returns raw recognized text.
struct PinnedGlyphOcrProvider;

impl OcrEvidenceProvider for PinnedGlyphOcrProvider {
    fn evidence(
        &self,
        image: &RgbaImage,
        profile: &LocatorProfile,
    ) -> Result<OcrEvidence, DesktopLocatorError> {
        let theme = detect_theme(image)?;
        let foreground = theme.foreground();
        let mut tokens = Vec::new();
        for y in 0..image.height() {
            let mut x = 0;
            while x < image.width() {
                if image.get_pixel(x, y) != &foreground {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < image.width() && image.get_pixel(x, y) == &foreground {
                    x += 1;
                }
                let run = x - start;
                if run >= 16 {
                    tokens.push(OcrTokenEvidence {
                        token_id: profile.required_token_id.to_string(),
                        bounds: PixelBounds {
                            x: start,
                            y,
                            width: run,
                            height: 1,
                        },
                    });
                }
            }
        }
        tokens.dedup_by(|left, right| {
            left.bounds.x == right.bounds.x && left.bounds.y.abs_diff(right.bounds.y) <= 1
        });
        let evidence_hash = hash_tokens("pinned-glyph-v1", &tokens);
        Ok(OcrEvidence {
            provider_version: "pinned-glyph-v1".to_string(),
            evidence_hash,
            tokens,
        })
    }
}

fn hash_tokens(provider_version: &str, tokens: &[OcrTokenEvidence]) -> String {
    digest_text(&format!(
        "{}\0{}",
        provider_version,
        tokens
            .iter()
            .map(|token| format!(
                "{}:{}:{}:{}:{}",
                token.token_id,
                token.bounds.x,
                token.bounds.y,
                token.bounds.width,
                token.bounds.height
            ))
            .collect::<Vec<_>>()
            .join("|")
    ))
}

fn digest_text(value: &str) -> String {
    digest_bytes(value.as_bytes())
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::ViewStreamProvider;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        fixture_id: String,
        width: u32,
        height: u32,
        theme: String,
        scale_millis: u32,
        targets: Vec<FixturePoint>,
        decoys: Vec<FixturePoint>,
        expected_status: String,
        expected_candidate_count: usize,
        expected_visualization_sha256: String,
    }

    #[derive(Debug, Deserialize)]
    struct FixturePoint {
        x: u32,
        y: u32,
    }

    struct FixtureOcrProvider {
        tokens: Vec<OcrTokenEvidence>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl FixtureOcrProvider {
        fn new(tokens: Vec<OcrTokenEvidence>) -> Self {
            Self {
                tokens,
                calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    impl OcrEvidenceProvider for FixtureOcrProvider {
        fn evidence(
            &self,
            _image: &RgbaImage,
            _profile: &LocatorProfile,
        ) -> Result<OcrEvidence, DesktopLocatorError> {
            self.calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(OcrEvidence {
                provider_version: "fixture-ocr-v1".to_string(),
                evidence_hash: hash_tokens("fixture-ocr-v1", &self.tokens),
                tokens: self.tokens.clone(),
            })
        }
    }

    const FIXTURES: [&str; 7] = [
        include_str!("../../../docs/dev/fixtures/desktop-locator/single-light-100.json"),
        include_str!("../../../docs/dev/fixtures/desktop-locator/single-dark-100.json"),
        include_str!("../../../docs/dev/fixtures/desktop-locator/single-light-125.json"),
        include_str!("../../../docs/dev/fixtures/desktop-locator/single-dark-150.json"),
        include_str!("../../../docs/dev/fixtures/desktop-locator/ambiguous-equal.json"),
        include_str!("../../../docs/dev/fixtures/desktop-locator/decoy-only.json"),
        include_str!("../../../docs/dev/fixtures/desktop-locator/geometry-edge.json"),
    ];

    #[test]
    fn unknown_locator_profile_is_typed() {
        let error = locator_profile("unknown").expect_err("unknown profile must fail");
        assert_eq!(error.code(), "desktop_locator_not_found");
    }

    #[test]
    fn synthetic_corpus_has_exact_stable_location_results() {
        for source in FIXTURES {
            let fixture: Fixture = serde_json::from_str(source).expect("valid fixture manifest");
            let (frame, tokens) = render_fixture(&fixture);
            let provider = FixtureOcrProvider::new(tokens);
            let first = locate_bound_frame(frame.clone(), LOCATOR_ID, 8, true, &provider)
                .unwrap_or_else(|error| panic!("{}: {error}", fixture.fixture_id));
            let second = locate_bound_frame(frame, LOCATOR_ID, 8, true, &provider)
                .unwrap_or_else(|error| panic!("{} repeat: {error}", fixture.fixture_id));

            assert_eq!(first.observation.status, fixture.expected_status);
            assert_eq!(
                first.observation.candidates.len(),
                fixture.expected_candidate_count
            );
            assert_eq!(
                serde_json::to_vec(&first.observation).unwrap(),
                serde_json::to_vec(&second.observation).unwrap(),
                "{} observation determinism",
                fixture.fixture_id
            );
            assert_eq!(first.visualization_bytes, second.visualization_bytes);
            let visualization_sha256 = &first
                .observation
                .visualization_receipt
                .as_ref()
                .unwrap()
                .content_sha256;
            assert_eq!(
                visualization_sha256, &fixture.expected_visualization_sha256,
                "{} visualization hash",
                fixture.fixture_id
            );
            if fixture.expected_status == "matched" {
                let expected = &fixture.targets[0];
                let selected = &first.observation.candidates[0];
                assert_eq!(
                    (selected.bounds.x, selected.bounds.y),
                    (expected.x, expected.y)
                );
                assert_eq!(
                    first.observation.selected_candidate_id.as_deref(),
                    Some(selected.candidate_id.as_str())
                );
            } else {
                assert!(first.observation.selected_candidate_id.is_none());
            }
        }
    }

    #[test]
    fn stale_binding_fails_before_detector_invocation() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../docs/dev/fixtures/desktop-locator/stale-binding.json"
        ))
        .unwrap();
        let (mut frame, tokens) = render_fixture(&fixture);
        frame.frame_receipt.geometry_epoch = "stale-geometry".to_string();
        let provider = FixtureOcrProvider::new(tokens);

        let error = locate_bound_frame(frame, LOCATOR_ID, 8, false, &provider).unwrap_err();

        assert_eq!(error.code(), "desktop_locator_frame_mismatch");
        assert_eq!(provider.calls(), 0);
    }

    #[test]
    fn ambiguity_cannot_be_hidden_by_a_one_candidate_response_limit() {
        let fixture: Fixture = serde_json::from_str(FIXTURES[4]).unwrap();
        let (frame, tokens) = render_fixture(&fixture);
        let result = locate_bound_frame(
            frame,
            LOCATOR_ID,
            1,
            false,
            &FixtureOcrProvider::new(tokens),
        )
        .unwrap();

        assert_eq!(result.observation.status, "ambiguous");
        assert!(result.observation.selected_candidate_id.is_none());
        assert_eq!(result.observation.candidates.len(), 1);
        assert_eq!(
            result.observation.candidates[0].ambiguity_evidence,
            ["runner_up_gap:0"]
        );
    }

    #[test]
    fn repository_glyph_provider_locates_without_ambient_ocr() {
        let fixture: Fixture = serde_json::from_str(FIXTURES[0]).unwrap();
        let (frame, _) = render_fixture(&fixture);

        let result = locate_bound_frame(frame, LOCATOR_ID, 8, false, &PinnedGlyphOcrProvider)
            .expect("pinned synthetic glyphs should normalize to token evidence");

        assert_eq!(result.observation.status, "matched");
        assert_eq!(
            result.observation.detector_receipts[2].detector_id,
            OCR_DETECTOR_ID
        );
    }

    #[test]
    fn unavailable_provider_preserves_a_typed_redacted_failure() {
        struct Unavailable;
        impl OcrEvidenceProvider for Unavailable {
            fn evidence(
                &self,
                _image: &RgbaImage,
                _profile: &LocatorProfile,
            ) -> Result<OcrEvidence, DesktopLocatorError> {
                Err(DesktopLocatorError::new(
                    "desktop_locator_detector_unavailable",
                    "normalized OCR evidence provider is unavailable",
                ))
            }
        }
        let fixture: Fixture = serde_json::from_str(FIXTURES[0]).unwrap();
        let (frame, _) = render_fixture(&fixture);

        let error = locate_bound_frame(frame, LOCATOR_ID, 8, false, &Unavailable).unwrap_err();

        assert_eq!(error.code(), "desktop_locator_detector_unavailable");
        assert!(!error.to_string().contains("stderr"));
    }

    #[test]
    fn malformed_png_unsupported_scale_and_bad_ocr_fail_closed() {
        let fixture: Fixture = serde_json::from_str(FIXTURES[0]).unwrap();
        let (mut invalid, tokens) = render_fixture(&fixture);
        invalid.image_bytes = vec![1, 2, 3];
        invalid.frame_receipt.byte_length = 3;
        invalid.frame_receipt.content_sha256 = digest_bytes(&invalid.image_bytes);
        let provider = FixtureOcrProvider::new(tokens.clone());
        assert_eq!(
            locate_bound_frame(invalid, LOCATOR_ID, 8, false, &provider)
                .unwrap_err()
                .code(),
            "desktop_locator_invalid_image"
        );

        let (mut unsupported, _) = render_fixture(&fixture);
        unsupported.context.scale_factor = 2.0;
        unsupported.frame_receipt.scale_factor = 2.0;
        assert_eq!(
            locate_bound_frame(unsupported, LOCATOR_ID, 8, false, &provider)
                .unwrap_err()
                .code(),
            "desktop_locator_unsupported"
        );

        struct BadOcr;
        impl OcrEvidenceProvider for BadOcr {
            fn evidence(
                &self,
                _image: &RgbaImage,
                _profile: &LocatorProfile,
            ) -> Result<OcrEvidence, DesktopLocatorError> {
                Ok(OcrEvidence {
                    provider_version: "bad-v1".to_string(),
                    evidence_hash: "raw provider output is private".to_string(),
                    tokens: Vec::new(),
                })
            }
        }
        let (frame, _) = render_fixture(&fixture);
        let error = locate_bound_frame(frame, LOCATOR_ID, 8, false, &BadOcr).unwrap_err();
        assert_eq!(error.code(), "desktop_locator_detector_failed");
        assert!(!error.to_string().contains("raw provider output is private"));
    }

    #[test]
    fn request_rejects_caller_evidence_and_bounds_candidate_count() {
        let valid = json!({
            "browserId": "browser-1",
            "locator": {"locatorId": LOCATOR_ID}
        });
        assert_eq!(parse_request(&valid).unwrap().max_candidates, 8);
        let mut injected = valid.clone();
        injected["imageBase64"] = json!("pixels");
        assert_eq!(
            parse_request(&injected).unwrap_err().code(),
            "desktop_locator_frame_mismatch"
        );
        let oversized = json!({
            "browserId": "browser-1",
            "locator": {"locatorId": LOCATOR_ID, "maxCandidates": 33}
        });
        assert_eq!(
            parse_request(&oversized).unwrap_err().code(),
            "desktop_locator_unsupported"
        );
    }

    #[test]
    fn response_only_visualization_is_removed_from_stream_projection() {
        let data = json!({
            "observation": {"observationId": "observation-1"},
            "visualizationBase64": "private-overlay"
        });
        let redacted = redact_desktop_locate_stream_result(&data);
        assert!(redacted.get("visualizationBase64").is_none());
        assert_eq!(redacted["visualizationPayload"], "response_only");
        assert_eq!(data["visualizationBase64"], "private-overlay");
    }

    fn render_fixture(fixture: &Fixture) -> (BoundFrame, Vec<OcrTokenEvidence>) {
        let theme = match fixture.theme.as_str() {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            other => panic!("unsupported fixture theme {other}"),
        };
        let mut image = RgbaImage::from_pixel(fixture.width, fixture.height, theme.background());
        let size = scaled(12, fixture.scale_millis).unwrap();
        let label_width = scaled(20, fixture.scale_millis).unwrap();
        let label_gap = scaled(4, fixture.scale_millis).unwrap();
        let mut tokens = Vec::new();
        for point in &fixture.targets {
            draw_fixture_control(&mut image, point, size, theme, false);
            let token_bounds = PixelBounds {
                x: point.x + size + label_gap,
                y: point.y + size / 3,
                width: label_width,
                height: 1,
            };
            for x in token_bounds.x..token_bounds.x + token_bounds.width {
                image.put_pixel(x, token_bounds.y, theme.foreground());
            }
            tokens.push(OcrTokenEvidence {
                token_id: "verify-control".to_string(),
                bounds: token_bounds,
            });
        }
        for point in &fixture.decoys {
            draw_fixture_control(&mut image, point, size, theme, true);
        }
        let bytes = encode_png(&image);
        let context_id = "desktop-context-fixture".to_string();
        let geometry_epoch = digest_text(&format!(
            "{}:{}:{}",
            fixture.width, fixture.height, fixture.scale_millis
        ));
        let frame_receipt = FrameReceipt {
            frame_id: "desktop-frame-fixture".to_string(),
            schema_version: "v1",
            context_id: context_id.clone(),
            capture_provider: "fixture-renderer",
            provider_version: "fixture-renderer-v1".to_string(),
            sequence: 1,
            captured_at: "2026-08-12T12:00:00Z".to_string(),
            width: fixture.width,
            height: fixture.height,
            scale_factor: fixture.scale_millis as f64 / 1000.0,
            geometry_epoch: geometry_epoch.clone(),
            mime_type: "image/png",
            byte_length: bytes.len(),
            content_sha256: digest_bytes(&bytes),
            freshness: "fresh_capture",
            retention: "ephemeral",
            persisted: false,
        };
        let context = DesktopContext {
            context_id,
            schema_version: "v1",
            browser_id: "browser-fixture".to_string(),
            session_name: "session-fixture".to_string(),
            profile_id: Some("profile-fixture".to_string()),
            display_allocation_id: "display-fixture".to_string(),
            stream_id: "stream-fixture".to_string(),
            route_id: "route-fixture".to_string(),
            capture_provider: "fixture-renderer",
            view_stream_provider: ViewStreamProvider::RdpGateway,
            display_isolation: "dedicated_display".to_string(),
            coordinate_space: COORDINATE_SPACE,
            width: fixture.width,
            height: fixture.height,
            scale_factor: fixture.scale_millis as f64 / 1000.0,
            geometry_epoch,
            resolved_at: "2026-08-12T12:00:00Z".to_string(),
            readiness: json!({"state": "ready", "displayContentState": "browser_window_visible"}),
        };
        (
            BoundFrame {
                context,
                frame_receipt,
                image_bytes: bytes,
            },
            tokens,
        )
    }

    fn draw_fixture_control(
        image: &mut RgbaImage,
        point: &FixturePoint,
        size: u32,
        theme: Theme,
        decoy: bool,
    ) {
        assert!(point.x + size <= image.width());
        assert!(point.y + size <= image.height());
        for y in 0..size {
            for x in 0..size {
                let border = x == 0 || y == 0 || x + 1 == size || y + 1 == size;
                let color = if border && !(decoy && y + 1 == size) {
                    theme.foreground()
                } else {
                    theme.background()
                };
                image.put_pixel(point.x + x, point.y + y, color);
            }
        }
    }

    fn encode_png(image: &RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, FilterType::Adaptive)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ColorType::Rgba8.into(),
            )
            .unwrap();
        bytes
    }
}
