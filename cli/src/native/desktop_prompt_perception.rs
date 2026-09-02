//! Pure perception of repository-owned synthetic browser-external prompts.
//!
//! The evidence in this module proves absence from independently rendered
//! fixture page inputs. It does not claim live CDP blindness or detection of a
//! real browser, extension, native dialog, credential prompt, or challenge.

use image::{ImageFormat, ImageReader, Limits, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Cursor;

use super::desktop_capture::{DesktopCaptureResult, DesktopContext, FrameReceipt};

const PROMPT_PROFILE_ID: &str = "p110-external-prompt-v1";
const PROMPT_PROFILE_VERSION: &str = "p110-prompt-v1";
const TARGET_CLASS: &str = "synthetic_browser_external_confirmation";
const FIXTURE_PROOF_CLASS: &str = "repository_fixture";
const BLINDNESS_CLAIM: &str = "absent_from_fixture_page_inputs";
const COORDINATE_SPACE: &str = "desktop_physical_pixels";
const FIXTURE_PROVIDER_ID: &str = "repository-fixture-compositor";
const FIXTURE_PROVIDER_VERSION: &str = "p110-fixture-renderer-v1";
const PAGE_RENDERER_ID: &str = "repository-page-renderer";
const PAGE_RENDERER_VERSION: &str = "p110-page-renderer-v1";
const REQUIRED_TOKEN_ID: &str = "fixture-external-confirmation";
const NORMALIZATION_VERSION: &str = "rgba8-integer-fixture-v1";
const TEMPLATE_THRESHOLD: u32 = 8_500;
const MAX_EVIDENCE_AGE_MS: u64 = 750;
const MAX_PAIR_SKEW_MS: u64 = 100;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_VISUALIZATION_BYTES: usize = 4 * 1024 * 1024;

#[cfg(test)]
const FIXTURE_SOURCES: &[&str] = &[
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/matched-light-100.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/matched-dark-125.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/extension-panel-150.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/native-modal-manual.json"),
    include_str!(
        "../../../docs/dev/fixtures/desktop-prompt-perception/external-page-lookalike.json"
    ),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/page-decoy-only.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/ambiguous-external.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/not-found.json"),
    include_str!(
        "../../../docs/dev/fixtures/desktop-prompt-perception/occlusion-within-budget.json"
    ),
    include_str!(
        "../../../docs/dev/fixtures/desktop-prompt-perception/occlusion-beyond-budget.json"
    ),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/unsupported-version.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/stale-evidence.json"),
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/stale-binding.json"),
];

#[cfg(test)]
const CORPUS_INDEX_SOURCE: &str =
    include_str!("../../../docs/dev/fixtures/desktop-prompt-perception/corpus-index.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixelBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelBounds {
    fn right(self) -> Option<u32> {
        self.x.checked_add(self.width)
    }

    fn bottom(self) -> Option<u32> {
        self.y.checked_add(self.height)
    }

    fn contains(self, other: Self) -> bool {
        self.width > 0
            && self.height > 0
            && other.width > 0
            && other.height > 0
            && other.x >= self.x
            && other.y >= self.y
            && other.right().zip(self.right()).is_some_and(|(a, b)| a <= b)
            && other
                .bottom()
                .zip(self.bottom())
                .is_some_and(|(a, b)| a <= b)
    }

    fn center(self) -> PixelPoint {
        PixelPoint {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixelPoint {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserSurfaceEvidence {
    pub browser_id: String,
    pub session_name: String,
    pub profile_id: Option<String>,
    pub display_allocation_id: String,
    pub stream_id: String,
    pub route_id: String,
    pub geometry_epoch: String,
    pub coordinate_space: String,
    pub width: u32,
    pub height: u32,
    pub scale_millis: u32,
    pub browser_bounds: PixelBounds,
    pub viewport_bounds: PixelBounds,
    pub surface_identity_digest: String,
    pub browser_process_identity_digest: String,
    pub provider_id: String,
    pub provider_version: String,
    pub viewport_layer_sha256: String,
    pub policy_disposition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntheticPageReference {
    pub proof_class: String,
    pub renderer_id: String,
    pub renderer_version: String,
    pub page_image_bytes: Vec<u8>,
    pub page_image_sha256: String,
    pub normalized_dom_token_ids: Vec<String>,
    pub dom_manifest_sha256: String,
    pub viewport_bounds: PixelBounds,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PromptEvidenceBundle {
    pub frame: DesktopCaptureResult,
    pub surface: BrowserSurfaceEvidence,
    pub page: SyntheticPageReference,
    pub captured_at_ms: u64,
    pub observed_at_ms: u64,
}

pub(crate) trait PromptClock {
    fn now_ms(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptDetectorReceipt {
    pub detector_id: &'static str,
    pub version: &'static str,
    pub evidence_sha256: String,
    pub normalization_version: &'static str,
    pub match_count: usize,
    pub threshold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptCandidate {
    pub candidate_id: String,
    pub target_class: &'static str,
    pub rank: usize,
    pub bounds: PixelBounds,
    pub center: PixelPoint,
    pub score: u32,
    pub disposition: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlindnessReceipt {
    pub proof_class: &'static str,
    pub claim: &'static str,
    pub desktop_frame_sha256: String,
    pub page_frame_sha256: String,
    pub dom_manifest_sha256: String,
    pub prompt_signature_sha256: String,
    pub binding_sha256: String,
    pub desktop_prompt_match_count: usize,
    pub page_screenshot_prompt_match_count: usize,
    pub dom_prompt_match_count: usize,
    pub correspondence_state: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperatorInterventionAction {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub safety: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperatorIntervention {
    pub state: &'static str,
    pub reason_code: &'static str,
    pub title: &'static str,
    pub message: &'static str,
    pub actions: Vec<OperatorInterventionAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptObservation {
    pub observation_id: String,
    pub schema_version: &'static str,
    pub context_id: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub geometry_epoch: String,
    pub coordinate_space: &'static str,
    pub prompt_profile_id: &'static str,
    pub profile_version: &'static str,
    pub profile_sha256: String,
    pub target_class: &'static str,
    pub surface_identity_digest: String,
    pub browser_process_identity_digest: String,
    pub detector_receipts: Vec<PromptDetectorReceipt>,
    pub candidates: Vec<PromptCandidate>,
    pub detection_status: &'static str,
    pub page_visibility: &'static str,
    pub classification: &'static str,
    pub handling_outcome: &'static str,
    pub selected_candidate_id: Option<String>,
    pub blindness_receipt: BlindnessReceipt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_intervention: Option<OperatorIntervention>,
    pub observed_at_ms: u64,
    pub evidence_age_ms: u64,
    pub retention: &'static str,
    pub persisted_pixels: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesktopPromptResult {
    pub context: DesktopContext,
    pub frame_receipt: FrameReceipt,
    pub prompt_observation: PromptObservation,
    pub visualization_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopPromptError {
    code: &'static str,
    message: &'static str,
}

impl DesktopPromptError {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for DesktopPromptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DesktopPromptError {}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    Light,
    Dark,
}

#[cfg(test)]
impl Theme {
    fn from_id(value: &str) -> Result<Self, DesktopPromptError> {
        match value {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(DesktopPromptError::new(
                "desktop_prompt_page_evidence_invalid",
                "fixture theme is unsupported",
            )),
        }
    }

    fn desktop(self) -> Rgba<u8> {
        match self {
            Self::Light => Rgba([232, 235, 240, 255]),
            Self::Dark => Rgba([18, 21, 27, 255]),
        }
    }

    fn chrome(self) -> Rgba<u8> {
        match self {
            Self::Light => Rgba([208, 214, 222, 255]),
            Self::Dark => Rgba([38, 43, 52, 255]),
        }
    }

    fn page(self) -> Rgba<u8> {
        match self {
            Self::Light => Rgba([250, 250, 248, 255]),
            Self::Dark => Rgba([29, 32, 39, 255]),
        }
    }

    fn prompt(self) -> Rgba<u8> {
        match self {
            Self::Light => Rgba([238, 242, 255, 255]),
            Self::Dark => Rgba([49, 56, 72, 255]),
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixturePrompt {
    x: u32,
    y: u32,
    #[serde(default)]
    occlusion_percent: u32,
    #[serde(default = "default_disposition")]
    disposition: String,
}

#[cfg(test)]
fn default_disposition() -> String {
    "actionable".to_string()
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureManifest {
    schema_version: String,
    fixture_id: String,
    renderer_version: String,
    width: u32,
    height: u32,
    theme: String,
    scale_millis: u32,
    browser_bounds: PixelBounds,
    viewport_bounds: PixelBounds,
    #[serde(default)]
    page_prompts: Vec<FixturePrompt>,
    #[serde(default)]
    external_prompts: Vec<FixturePrompt>,
    #[serde(default)]
    dom_tokens: Vec<String>,
    #[serde(default)]
    stale_by_ms: u64,
    #[serde(default)]
    binding_mismatch: bool,
    #[serde(default)]
    golden_hashes: Option<FixtureGoldenHashes>,
    expected_detection_status: String,
    expected_page_visibility: String,
    expected_classification: String,
    expected_handling_outcome: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGoldenHashes {
    manifest_sha256: String,
    profile_sha256: String,
    renderer_sha256: String,
    page_image_sha256: String,
    dom_manifest_sha256: String,
    desktop_frame_sha256: String,
    viewport_layer_sha256: String,
    detector_sha256: String,
    observation_sha256: String,
    visualization_sha256: String,
    paired_receipt_sha256: String,
}

struct PromptProfile {
    profile_sha256: String,
    scale_millis: u32,
    prompt_width: u32,
    prompt_height: u32,
}

#[derive(Debug, Clone)]
struct TemplateMatch {
    bounds: PixelBounds,
    score: u32,
}

/// Observe a prompt from already bound, provider-free evidence.
pub(crate) fn observe_desktop_prompt(
    bundle: PromptEvidenceBundle,
    prompt_profile_id: &str,
    include_visualization: bool,
    clock: &dyn PromptClock,
) -> Result<DesktopPromptResult, DesktopPromptError> {
    let profile = profile(prompt_profile_id, &bundle)?;
    let (desktop, page, age_ms, binding_sha256) =
        validate_evidence(&bundle, &profile, clock.now_ms())?;

    let mut desktop_matches = scan_prompts(&desktop, &profile)?;
    let page_matches = scan_prompts(&page, &profile)?;
    desktop_matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.bounds.y.cmp(&right.bounds.y))
            .then_with(|| left.bounds.x.cmp(&right.bounds.x))
    });
    let dom_match_count = bundle
        .page
        .normalized_dom_token_ids
        .iter()
        .filter(|token| token.as_str() == REQUIRED_TOKEN_ID)
        .count();
    let page_visibility = if page_matches.is_empty() && dom_match_count == 0 {
        "absent"
    } else {
        "present"
    };
    let detection_status = match desktop_matches.len() {
        0 => "not_found",
        1 => "matched",
        _ => "ambiguous",
    };
    let selected_match = (detection_status == "matched" && page_visibility == "absent")
        .then(|| desktop_matches[0].clone());
    let classification = if selected_match.is_some() {
        "browser_external"
    } else if detection_status == "matched" && page_visibility == "present" {
        "page_surface"
    } else {
        "unclassified"
    };

    let candidates = desktop_matches
        .iter()
        .enumerate()
        .map(|(index, candidate)| PromptCandidate {
            candidate_id: candidate_id(
                &bundle.frame.frame_receipt.frame_id,
                candidate.bounds,
                candidate.score,
            ),
            target_class: TARGET_CLASS,
            rank: index + 1,
            bounds: candidate.bounds,
            center: candidate.bounds.center(),
            score: candidate.score,
            disposition: fixture_disposition(&bundle),
        })
        .collect::<Vec<_>>();
    let selected_candidate_id = selected_match.as_ref().map(|candidate| {
        candidate_id(
            &bundle.frame.frame_receipt.frame_id,
            candidate.bounds,
            candidate.score,
        )
    });
    let selected_disposition = selected_match
        .as_ref()
        .map(|_| fixture_disposition(&bundle));
    let handling_outcome = if detection_status == "ambiguous" {
        "operator_intervention_required"
    } else if classification == "browser_external" {
        if selected_disposition == Some("manual_only") {
            "operator_intervention_required"
        } else {
            "actionable_observation"
        }
    } else {
        "none"
    };
    let operator_intervention =
        (handling_outcome == "operator_intervention_required").then(operator_intervention);

    let prompt_signature_sha256 = hash_parts(
        "p110.prompt.signature.v1",
        &[
            &profile.prompt_width.to_string(),
            &profile.prompt_height.to_string(),
            &TEMPLATE_THRESHOLD.to_string(),
        ],
    );
    let detector_receipts = vec![
        PromptDetectorReceipt {
            detector_id: "desktop-fixture-template",
            version: "v1",
            evidence_sha256: bundle.frame.frame_receipt.content_sha256.clone(),
            normalization_version: NORMALIZATION_VERSION,
            match_count: desktop_matches.len(),
            threshold: TEMPLATE_THRESHOLD,
        },
        PromptDetectorReceipt {
            detector_id: "page-fixture-template",
            version: "v1",
            evidence_sha256: bundle.page.page_image_sha256.clone(),
            normalization_version: NORMALIZATION_VERSION,
            match_count: page_matches.len(),
            threshold: TEMPLATE_THRESHOLD,
        },
        PromptDetectorReceipt {
            detector_id: "normalized-fixture-dom",
            version: "v1",
            evidence_sha256: bundle.page.dom_manifest_sha256.clone(),
            normalization_version: "sorted-token-ids-v1",
            match_count: dom_match_count,
            threshold: 0,
        },
    ];
    let blindness_receipt = BlindnessReceipt {
        proof_class: FIXTURE_PROOF_CLASS,
        claim: BLINDNESS_CLAIM,
        desktop_frame_sha256: bundle.frame.frame_receipt.content_sha256.clone(),
        page_frame_sha256: bundle.page.page_image_sha256.clone(),
        dom_manifest_sha256: bundle.page.dom_manifest_sha256.clone(),
        prompt_signature_sha256,
        binding_sha256,
        desktop_prompt_match_count: desktop_matches.len(),
        page_screenshot_prompt_match_count: page_matches.len(),
        dom_prompt_match_count: dom_match_count,
        correspondence_state: "verified",
    };
    let observation_id = format!(
        "desktop-prompt-observation-{}",
        &hash_parts(
            "p110.prompt.observation-id.v1",
            &[
                &bundle.frame.context.context_id,
                &bundle.frame.frame_receipt.frame_id,
                &profile.profile_sha256,
                detection_status,
                page_visibility,
                classification,
                &candidates
                    .iter()
                    .map(|candidate| candidate.candidate_id.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ],
        )[..24]
    );
    let visualization_bytes = if include_visualization {
        let mut visualization = desktop.clone();
        for candidate in &desktop_matches {
            draw_outline(
                &mut visualization,
                candidate.bounds,
                Rgba([22, 163, 74, 255]),
            )?;
        }
        let bytes = encode_png(&visualization)?;
        if bytes.len() > MAX_VISUALIZATION_BYTES {
            return Err(DesktopPromptError::new(
                "desktop_prompt_visualization_failed",
                "fixture visualization exceeds the fixed response budget",
            ));
        }
        Some(bytes)
    } else {
        None
    };

    Ok(DesktopPromptResult {
        context: bundle.frame.context.clone(),
        frame_receipt: bundle.frame.frame_receipt.clone(),
        prompt_observation: PromptObservation {
            observation_id,
            schema_version: "v1",
            context_id: bundle.frame.context.context_id,
            frame_id: bundle.frame.frame_receipt.frame_id,
            frame_sha256: bundle.frame.frame_receipt.content_sha256,
            geometry_epoch: bundle.frame.context.geometry_epoch,
            coordinate_space: COORDINATE_SPACE,
            prompt_profile_id: PROMPT_PROFILE_ID,
            profile_version: PROMPT_PROFILE_VERSION,
            profile_sha256: profile.profile_sha256,
            target_class: TARGET_CLASS,
            surface_identity_digest: bundle.surface.surface_identity_digest,
            browser_process_identity_digest: bundle.surface.browser_process_identity_digest,
            detector_receipts,
            candidates,
            detection_status,
            page_visibility,
            classification,
            handling_outcome,
            selected_candidate_id,
            blindness_receipt,
            operator_intervention,
            observed_at_ms: bundle.observed_at_ms,
            evidence_age_ms: age_ms,
            retention: "ephemeral",
            persisted_pixels: false,
        },
        visualization_bytes,
    })
}

/// Configured PoC 4 dispatch has no production perception provider. It fails
/// before desktop capture or any browser, CDP, process, network, filesystem,
/// route, controller, or input resolution.
pub(crate) async fn handle_desktop_prompt_observe(_command: &Value) -> Result<Value, String> {
    Err(
        "desktop_prompt_provider_unavailable: no production desktop prompt provider is configured"
            .to_string(),
    )
}

/// Keep only privacy-safe prompt receipt fields for long-lived projections.
pub(crate) fn redact_desktop_prompt_stream_result(result: &Value) -> Value {
    let Some(record) = result.as_object() else {
        return Value::Null;
    };
    let mut redacted = serde_json::Map::new();
    for field in ["ok", "action"] {
        if let Some(value) = record.get(field) {
            redacted.insert(field.to_string(), value.clone());
        }
    }
    if let Some(context) = record.get("context").and_then(Value::as_object) {
        redacted.insert(
            "context".to_string(),
            allowlisted_object(
                context,
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
                    "geometryEpoch",
                ],
            ),
        );
    }
    if let Some(receipt) = record.get("frameReceipt").and_then(Value::as_object) {
        redacted.insert(
            "frameReceipt".to_string(),
            allowlisted_object(
                receipt,
                &[
                    "frameId",
                    "schemaVersion",
                    "contextId",
                    "sequence",
                    "capturedAt",
                    "geometryEpoch",
                    "sha256",
                    "freshness",
                    "retention",
                    "persisted",
                ],
            ),
        );
    }
    if let Some(observation) = record.get("promptObservation").and_then(Value::as_object) {
        redacted.insert(
            "promptObservation".to_string(),
            redact_prompt_observation(observation),
        );
    }
    if record.contains_key("visualizationBase64") {
        redacted.insert(
            "visualizationPayload".to_string(),
            Value::String("response_only".to_string()),
        );
    }
    Value::Object(redacted)
}

fn redact_prompt_observation(record: &serde_json::Map<String, Value>) -> Value {
    let mut safe = allowlisted_map(
        record,
        &[
            "observationId",
            "schemaVersion",
            "contextId",
            "frameId",
            "frameSha256",
            "geometryEpoch",
            "coordinateSpace",
            "promptProfileId",
            "profileVersion",
            "profileSha256",
            "targetClass",
            "surfaceIdentityDigest",
            "browserProcessIdentityDigest",
            "detectionStatus",
            "pageVisibility",
            "classification",
            "handlingOutcome",
            "selectedCandidateId",
            "observedAtMs",
            "evidenceAgeMs",
            "retention",
            "persistedPixels",
        ],
    );
    if let Some(receipts) = record.get("detectorReceipts").and_then(Value::as_array) {
        safe.insert(
            "detectorReceipts".to_string(),
            Value::Array(
                receipts
                    .iter()
                    .filter_map(Value::as_object)
                    .map(|receipt| {
                        allowlisted_object(
                            receipt,
                            &[
                                "detectorId",
                                "version",
                                "evidenceSha256",
                                "normalizationVersion",
                                "matchCount",
                                "threshold",
                            ],
                        )
                    })
                    .collect(),
            ),
        );
    }
    if let Some(candidates) = record.get("candidates").and_then(Value::as_array) {
        safe.insert(
            "candidates".to_string(),
            Value::Array(
                candidates
                    .iter()
                    .filter_map(Value::as_object)
                    .map(redact_candidate)
                    .collect(),
            ),
        );
    }
    if let Some(receipt) = record.get("blindnessReceipt").and_then(Value::as_object) {
        safe.insert(
            "blindnessReceipt".to_string(),
            allowlisted_object(
                receipt,
                &[
                    "proofClass",
                    "claim",
                    "desktopFrameSha256",
                    "pageFrameSha256",
                    "domManifestSha256",
                    "promptSignatureSha256",
                    "bindingSha256",
                    "desktopPromptMatchCount",
                    "pageScreenshotPromptMatchCount",
                    "domPromptMatchCount",
                    "correspondenceState",
                ],
            ),
        );
    }
    if let Some(intervention) = record
        .get("operatorIntervention")
        .and_then(Value::as_object)
    {
        let mut projected = allowlisted_map(intervention, &["state", "reasonCode"]);
        if let Some(actions) = intervention.get("actions").and_then(Value::as_array) {
            projected.insert(
                "actions".to_string(),
                Value::Array(
                    actions
                        .iter()
                        .filter_map(Value::as_object)
                        .map(|action| allowlisted_object(action, &["id", "kind", "safety"]))
                        .collect(),
                ),
            );
        }
        safe.insert("operatorIntervention".to_string(), Value::Object(projected));
    }
    Value::Object(safe)
}

fn redact_candidate(record: &serde_json::Map<String, Value>) -> Value {
    let mut safe = allowlisted_map(
        record,
        &["candidateId", "targetClass", "rank", "score", "disposition"],
    );
    for field in ["bounds", "center"] {
        if let Some(point) = record.get(field).and_then(Value::as_object) {
            let fields: &[&str] = if field == "bounds" {
                &["x", "y", "width", "height"]
            } else {
                &["x", "y"]
            };
            safe.insert(field.to_string(), allowlisted_object(point, fields));
        }
    }
    Value::Object(safe)
}

fn allowlisted_map(
    record: &serde_json::Map<String, Value>,
    fields: &[&str],
) -> serde_json::Map<String, Value> {
    fields
        .iter()
        .filter_map(|field| {
            record
                .get(*field)
                .map(|value| ((*field).to_string(), value.clone()))
        })
        .collect()
}

fn allowlisted_object(record: &serde_json::Map<String, Value>, fields: &[&str]) -> Value {
    Value::Object(allowlisted_map(record, fields))
}

fn profile(
    prompt_profile_id: &str,
    bundle: &PromptEvidenceBundle,
) -> Result<PromptProfile, DesktopPromptError> {
    if prompt_profile_id != PROMPT_PROFILE_ID {
        return Err(DesktopPromptError::new(
            "desktop_prompt_profile_not_found",
            "prompt profile is not repository-owned",
        ));
    }
    let scale_millis = exact_scale_millis(bundle.frame.context.scale_factor)?;
    if ![1000, 1250, 1500].contains(&scale_millis) {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture scale is unsupported",
        ));
    }
    let prompt_width = scaled(80, scale_millis)?;
    let prompt_height = scaled(48, scale_millis)?;
    let profile_sha256 = hash_parts(
        "p110.prompt.profile.v1",
        &[
            PROMPT_PROFILE_ID,
            PROMPT_PROFILE_VERSION,
            TARGET_CLASS,
            &scale_millis.to_string(),
            &prompt_width.to_string(),
            &prompt_height.to_string(),
            &TEMPLATE_THRESHOLD.to_string(),
            REQUIRED_TOKEN_ID,
        ],
    );
    Ok(PromptProfile {
        profile_sha256,
        scale_millis,
        prompt_width,
        prompt_height,
    })
}

fn validate_evidence(
    bundle: &PromptEvidenceBundle,
    profile: &PromptProfile,
    now_ms: u64,
) -> Result<(RgbaImage, RgbaImage, u64, String), DesktopPromptError> {
    let frame = &bundle.frame;
    let mismatch = || {
        DesktopPromptError::new(
            "desktop_prompt_binding_mismatch",
            "desktop, surface, and fixture page evidence do not share one exact binding",
        )
    };
    if frame.context.schema_version != "v1"
        || frame.frame_receipt.schema_version != "v1"
        || frame.context.context_id != frame.frame_receipt.context_id
        || frame.context.capture_provider != frame.frame_receipt.capture_provider
        || frame.context.width != frame.frame_receipt.width
        || frame.context.height != frame.frame_receipt.height
        || exact_scale_millis(frame.context.scale_factor).ok()
            != exact_scale_millis(frame.frame_receipt.scale_factor).ok()
        || profile.scale_millis != exact_scale_millis(frame.context.scale_factor)?
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
    let surface = &bundle.surface;
    if surface.browser_id != frame.context.browser_id
        || surface.session_name != frame.context.session_name
        || surface.profile_id != frame.context.profile_id
        || surface.display_allocation_id != frame.context.display_allocation_id
        || surface.stream_id != frame.context.stream_id
        || surface.route_id != frame.context.route_id
        || surface.geometry_epoch != frame.context.geometry_epoch
        || surface.coordinate_space != frame.context.coordinate_space
        || surface.width != frame.context.width
        || surface.height != frame.context.height
        || surface.scale_millis != profile.scale_millis
        || surface.provider_id != FIXTURE_PROVIDER_ID
        || surface.provider_version != FIXTURE_PROVIDER_VERSION
        || !matches!(
            surface.policy_disposition.as_str(),
            "actionable" | "manual_only"
        )
        || surface.surface_identity_digest.len() != 64
        || surface.browser_process_identity_digest.len() != 64
        || !(PixelBounds {
            x: 0,
            y: 0,
            width: surface.width,
            height: surface.height,
        })
        .contains(surface.browser_bounds)
        || !surface.browser_bounds.contains(surface.viewport_bounds)
    {
        return Err(mismatch());
    }
    let page = &bundle.page;
    if page.proof_class != FIXTURE_PROOF_CLASS
        || page.renderer_id != PAGE_RENDERER_ID
        || page.renderer_version != PAGE_RENDERER_VERSION
        || page.page_image_sha256 != digest_bytes(&page.page_image_bytes)
        || page.viewport_bounds != surface.viewport_bounds
        || page.dom_manifest_sha256 != dom_manifest_hash(&page.normalized_dom_token_ids)
        || !page
            .normalized_dom_token_ids
            .iter()
            .all(|token| valid_token_id(token))
    {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "independent fixture page evidence is malformed or does not correspond",
        ));
    }
    let age_ms = now_ms.checked_sub(bundle.captured_at_ms).ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_stale_evidence",
            "fixture capture is from the future",
        )
    })?;
    let page_age_ms = now_ms.checked_sub(page.observed_at_ms).ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_stale_evidence",
            "fixture page observation is from the future",
        )
    })?;
    let observation_age_ms = now_ms.checked_sub(bundle.observed_at_ms).ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_stale_evidence",
            "fixture observation is from the future",
        )
    })?;
    if age_ms > MAX_EVIDENCE_AGE_MS
        || page_age_ms > MAX_EVIDENCE_AGE_MS
        || observation_age_ms > MAX_EVIDENCE_AGE_MS
        || bundle.captured_at_ms.abs_diff(page.observed_at_ms) > MAX_PAIR_SKEW_MS
        || bundle.captured_at_ms.abs_diff(bundle.observed_at_ms) > MAX_PAIR_SKEW_MS
    {
        return Err(DesktopPromptError::new(
            "desktop_prompt_stale_evidence",
            "fixture evidence is stale or outside the fixed pairing window",
        ));
    }
    let desktop = decode_png(
        &frame.image_bytes,
        frame.context.width,
        frame.context.height,
        "desktop_prompt_invalid_image",
    )?;
    let page_image = decode_png(
        &page.page_image_bytes,
        surface.viewport_bounds.width,
        surface.viewport_bounds.height,
        "desktop_prompt_page_evidence_invalid",
    )?;
    let viewport_layer_bytes = derive_viewport_layer(&desktop, surface.viewport_bounds)?;
    let viewport_layer_sha256 = digest_bytes(&viewport_layer_bytes);
    if viewport_layer_bytes != page.page_image_bytes
        || viewport_layer_sha256 != page.page_image_sha256
        || viewport_layer_sha256 != surface.viewport_layer_sha256
    {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "decoded desktop viewport does not equal the independent fixture page image",
        ));
    }
    let binding_sha256 = hash_parts(
        "p110.prompt.binding.v1",
        &[
            &frame.context.context_id,
            &frame.frame_receipt.frame_id,
            &frame.frame_receipt.content_sha256,
            &surface.surface_identity_digest,
            &surface.browser_process_identity_digest,
            &page.page_image_sha256,
            &page.dom_manifest_sha256,
            &surface.geometry_epoch,
            &bounds_projection(surface.browser_bounds),
            &bounds_projection(surface.viewport_bounds),
            &viewport_layer_sha256,
        ],
    );
    Ok((desktop, page_image, age_ms, binding_sha256))
}

fn derive_viewport_layer(
    desktop: &RgbaImage,
    viewport: PixelBounds,
) -> Result<Vec<u8>, DesktopPromptError> {
    let desktop_bounds = PixelBounds {
        x: 0,
        y: 0,
        width: desktop.width(),
        height: desktop.height(),
    };
    if !desktop_bounds.contains(viewport) {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "declared viewport exceeds the decoded desktop frame",
        ));
    }
    let mut layer = RgbaImage::new(viewport.width, viewport.height);
    for y in 0..viewport.height {
        for x in 0..viewport.width {
            layer.put_pixel(x, y, *desktop.get_pixel(viewport.x + x, viewport.y + y));
        }
    }
    encode_png_with_error(
        &layer,
        "desktop_prompt_page_evidence_invalid",
        "derived desktop viewport encoding failed",
    )
}

fn bounds_projection(bounds: PixelBounds) -> String {
    format!(
        "{}:{}:{}:{}",
        bounds.x, bounds.y, bounds.width, bounds.height
    )
}

fn decode_png(
    bytes: &[u8],
    width: u32,
    height: u32,
    code: &'static str,
) -> Result<RgbaImage, DesktopPromptError> {
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > MAX_PIXELS
        || bytes.is_empty()
    {
        return Err(DesktopPromptError::new(
            code,
            "fixture image is outside fixed bounds",
        ));
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| DesktopPromptError::new(code, "fixture image is not a bounded PNG"))?;
    if image.width() != width || image.height() != height {
        return Err(DesktopPromptError::new(
            code,
            "fixture image dimensions do not match evidence",
        ));
    }
    Ok(image.to_rgba8())
}

fn scan_prompts(
    image: &RgbaImage,
    profile: &PromptProfile,
) -> Result<Vec<TemplateMatch>, DesktopPromptError> {
    if profile.prompt_width > image.width() || profile.prompt_height > image.height() {
        return Ok(Vec::new());
    }
    let accent = Rgba([126, 34, 206, 255]);
    let mut matches = Vec::new();
    for y in 0..=image.height() - profile.prompt_height {
        for x in 0..=image.width() - profile.prompt_width {
            if image.get_pixel(x, y) != &accent
                || (x > 0 && image.get_pixel(x - 1, y) == &accent)
                || (y > 0 && image.get_pixel(x, y - 1) == &accent)
            {
                continue;
            }
            let bounds = PixelBounds {
                x,
                y,
                width: profile.prompt_width,
                height: profile.prompt_height,
            };
            let score = template_score(image, bounds)?;
            if score >= TEMPLATE_THRESHOLD {
                matches.push(TemplateMatch { bounds, score });
            }
        }
    }
    Ok(matches)
}

fn template_score(image: &RgbaImage, bounds: PixelBounds) -> Result<u32, DesktopPromptError> {
    if bounds.right().is_none()
        || bounds.bottom().is_none()
        || bounds.right().is_some_and(|right| right > image.width())
        || bounds
            .bottom()
            .is_some_and(|bottom| bottom > image.height())
    {
        return Err(DesktopPromptError::new(
            "desktop_prompt_detector_failed",
            "fixture detector bounds overflowed",
        ));
    }
    let accent = Rgba([126, 34, 206, 255]);
    let mut matching = 0_u64;
    let mut expected_signature_pixels = 0_u64;
    for offset_y in 0..bounds.height {
        for offset_x in 0..bounds.width {
            let border = offset_x < 2
                || offset_y < 2
                || offset_x + 2 >= bounds.width
                || offset_y + 2 >= bounds.height;
            let glyph = offset_y == bounds.height / 2
                && offset_x >= bounds.width / 4
                && offset_x < (bounds.width * 3) / 4;
            let actual = image.get_pixel(bounds.x + offset_x, bounds.y + offset_y);
            if border || glyph {
                expected_signature_pixels += 1;
                if actual == &accent {
                    matching += 1;
                }
            }
        }
    }
    if expected_signature_pixels == 0 {
        return Err(DesktopPromptError::new(
            "desktop_prompt_detector_failed",
            "fixture prompt signature is empty",
        ));
    }
    Ok(((matching * 10_000) / expected_signature_pixels) as u32)
}

fn exact_scale_millis(scale: f64) -> Result<u32, DesktopPromptError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture scale is not exact and positive",
        ));
    }
    let millis = (scale * 1000.0).round();
    if (scale * 1000.0 - millis).abs() > 0.000_001 {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture scale is not exact fixed-point evidence",
        ));
    }
    Ok(millis as u32)
}

fn scaled(value: u32, scale_millis: u32) -> Result<u32, DesktopPromptError> {
    value
        .checked_mul(scale_millis)
        .and_then(|value| value.checked_add(500))
        .map(|value| value / 1000)
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            DesktopPromptError::new(
                "desktop_prompt_detector_failed",
                "fixture scale transform overflowed",
            )
        })
}

fn candidate_id(frame_id: &str, bounds: PixelBounds, score: u32) -> String {
    format!(
        "desktop-prompt-candidate-{}",
        &hash_parts(
            "p110.prompt.candidate-id.v1",
            &[
                frame_id,
                &bounds.x.to_string(),
                &bounds.y.to_string(),
                &bounds.width.to_string(),
                &bounds.height.to_string(),
                &score.to_string(),
            ],
        )[..24]
    )
}

fn operator_intervention() -> OperatorIntervention {
    OperatorIntervention {
        state: "required",
        reason_code: "synthetic_prompt_requires_operator_review",
        title: "Review synthetic desktop prompt",
        message: "The repository fixture requires operator review; no input was attempted.",
        actions: vec![OperatorInterventionAction {
            id: "review_in_remote_view",
            label: "Review in remote view",
            kind: "operator_instruction",
            safety: "safe",
            description:
                "Review the already authorized durable remote view without invoking input.",
        }],
    }
}

fn valid_token_id(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 64
        && token
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn dom_manifest_hash(tokens: &[String]) -> String {
    let sorted = tokens.iter().cloned().collect::<BTreeSet<_>>();
    hash_parts(
        "p110.prompt.dom-manifest.v1",
        &sorted.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

fn hash_parts(domain: &str, parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    for part in parts {
        digest.update([0]);
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

#[cfg(test)]
fn renderer_sha256() -> String {
    hash_parts(
        "p110.prompt.renderer.v1",
        &[
            FIXTURE_PROVIDER_VERSION,
            PAGE_RENDERER_VERSION,
            NORMALIZATION_VERSION,
            &TEMPLATE_THRESHOLD.to_string(),
            "integer-rgba-primitives",
            "png-best-adaptive",
        ],
    )
}

#[cfg(test)]
fn manifest_projection_sha256(source: &str) -> String {
    let mut value: Value = serde_json::from_str(source).expect("valid fixture manifest");
    value
        .as_object_mut()
        .expect("fixture manifest object")
        .remove("goldenHashes");
    digest_bytes(&serde_json::to_vec(&value).expect("canonical manifest projection"))
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn draw_outline(
    image: &mut RgbaImage,
    bounds: PixelBounds,
    color: Rgba<u8>,
) -> Result<(), DesktopPromptError> {
    let right = bounds.right().ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_visualization_failed",
            "visualization coordinate overflowed",
        )
    })?;
    let bottom = bounds.bottom().ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_visualization_failed",
            "visualization coordinate overflowed",
        )
    })?;
    if right > image.width() || bottom > image.height() {
        return Err(DesktopPromptError::new(
            "desktop_prompt_visualization_failed",
            "visualization bounds exceed the fixture frame",
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

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, DesktopPromptError> {
    encode_png_with_error(
        image,
        "desktop_prompt_visualization_failed",
        "fixture PNG encoding failed",
    )
}

fn encode_png_with_error(
    image: &RgbaImage,
    code: &'static str,
    message: &'static str,
) -> Result<Vec<u8>, DesktopPromptError> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::{ColorType, ImageEncoder};

    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, FilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|_| DesktopPromptError::new(code, message))?;
    Ok(bytes)
}

fn fixture_disposition(bundle: &PromptEvidenceBundle) -> &'static str {
    if bundle.surface.policy_disposition == "manual_only" {
        "manual_only"
    } else {
        "actionable"
    }
}

#[cfg(test)]
fn observe_repository_fixture(
    fixture_id: &str,
    include_visualization: bool,
) -> Result<DesktopPromptResult, DesktopPromptError> {
    let manifest = fixture_manifest(fixture_id)?;
    let now_ms = 10_000;
    let bundle = render_fixture(&manifest, now_ms)?;
    observe_desktop_prompt(
        bundle,
        PROMPT_PROFILE_ID,
        include_visualization,
        &FixedClock(now_ms),
    )
}

#[cfg(test)]
fn fixture_manifest(fixture_id: &str) -> Result<FixtureManifest, DesktopPromptError> {
    FIXTURE_SOURCES
        .iter()
        .filter_map(|source| serde_json::from_str::<FixtureManifest>(source).ok())
        .find(|fixture| fixture.fixture_id == fixture_id)
        .ok_or_else(|| {
            DesktopPromptError::new(
                "desktop_prompt_profile_not_found",
                "repository fixture is not known",
            )
        })
}

#[cfg(test)]
struct FixedClock(u64);

#[cfg(test)]
impl PromptClock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
fn render_fixture(
    fixture: &FixtureManifest,
    now_ms: u64,
) -> Result<PromptEvidenceBundle, DesktopPromptError> {
    use super::desktop_capture::{DesktopContext, FrameReceipt};
    use super::service_model::ViewStreamProvider;

    if fixture.schema_version != "p110-desktop-prompt-fixture.v1"
        || fixture.renderer_version != FIXTURE_PROVIDER_VERSION
    {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture manifest version is unsupported",
        ));
    }
    let theme = Theme::from_id(&fixture.theme)?;
    let page = render_page(fixture, theme)?;
    let page_bytes = encode_png(&page)?;
    let mut desktop = RgbaImage::from_pixel(fixture.width, fixture.height, theme.desktop());
    fill_rect(&mut desktop, fixture.browser_bounds, theme.chrome())?;
    copy_layer(&mut desktop, &page, fixture.viewport_bounds)?;
    for prompt in &fixture.external_prompts {
        draw_prompt(
            &mut desktop,
            PixelBounds {
                x: prompt.x,
                y: prompt.y,
                width: scaled(80, fixture.scale_millis)?,
                height: scaled(48, fixture.scale_millis)?,
            },
            theme,
            prompt.occlusion_percent,
        )?;
    }
    let desktop_bytes = encode_png(&desktop)?;
    let context_id = format!(
        "desktop-context-{}",
        &hash_parts(
            "p110.prompt.fixture-context.v1",
            &[&fixture.fixture_id, &fixture.scale_millis.to_string()],
        )[..24]
    );
    let geometry_epoch = hash_parts(
        "p110.prompt.geometry.v1",
        &[
            &fixture.width.to_string(),
            &fixture.height.to_string(),
            &fixture.scale_millis.to_string(),
        ],
    );
    let frame_sha256 = digest_bytes(&desktop_bytes);
    let page_sha256 = digest_bytes(&page_bytes);
    let observed_at_ms = now_ms.saturating_sub(fixture.stale_by_ms);
    let frame_receipt = FrameReceipt {
        frame_id: format!("desktop-frame-{}", &frame_sha256[..24]),
        schema_version: "v1",
        context_id: context_id.clone(),
        capture_provider: "fixture-renderer",
        provider_version: FIXTURE_PROVIDER_VERSION.to_string(),
        sequence: 1,
        captured_at: "2026-08-12T12:00:00Z".to_string(),
        width: fixture.width,
        height: fixture.height,
        scale_factor: fixture.scale_millis as f64 / 1000.0,
        geometry_epoch: geometry_epoch.clone(),
        mime_type: "image/png",
        byte_length: desktop_bytes.len(),
        content_sha256: frame_sha256,
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
        display_isolation: "private_virtual_display".to_string(),
        coordinate_space: COORDINATE_SPACE,
        width: fixture.width,
        height: fixture.height,
        scale_factor: fixture.scale_millis as f64 / 1000.0,
        geometry_epoch: geometry_epoch.clone(),
        resolved_at: "2026-08-12T12:00:00Z".to_string(),
        readiness: serde_json::json!({"state": "ready", "displayContentState": "browser_window_visible"}),
    };
    let surface_identity_digest = hash_parts("p110.prompt.surface.v1", &[&fixture.fixture_id]);
    let mut surface = BrowserSurfaceEvidence {
        browser_id: context.browser_id.clone(),
        session_name: context.session_name.clone(),
        profile_id: context.profile_id.clone(),
        display_allocation_id: context.display_allocation_id.clone(),
        stream_id: context.stream_id.clone(),
        route_id: context.route_id.clone(),
        geometry_epoch: geometry_epoch.clone(),
        coordinate_space: COORDINATE_SPACE.to_string(),
        width: fixture.width,
        height: fixture.height,
        scale_millis: fixture.scale_millis,
        browser_bounds: fixture.browser_bounds,
        viewport_bounds: fixture.viewport_bounds,
        surface_identity_digest,
        browser_process_identity_digest: hash_parts(
            "p110.prompt.browser-process.v1",
            &[&fixture.fixture_id],
        ),
        provider_id: FIXTURE_PROVIDER_ID.to_string(),
        provider_version: FIXTURE_PROVIDER_VERSION.to_string(),
        viewport_layer_sha256: page_sha256.clone(),
        policy_disposition: fixture
            .external_prompts
            .first()
            .map(|prompt| prompt.disposition.clone())
            .unwrap_or_else(default_disposition),
    };
    if fixture.binding_mismatch {
        surface.route_id = "route-mismatch".to_string();
    }
    Ok(PromptEvidenceBundle {
        frame: DesktopCaptureResult {
            context,
            frame_receipt,
            image_bytes: desktop_bytes,
        },
        surface,
        page: SyntheticPageReference {
            proof_class: FIXTURE_PROOF_CLASS.to_string(),
            renderer_id: PAGE_RENDERER_ID.to_string(),
            renderer_version: PAGE_RENDERER_VERSION.to_string(),
            page_image_bytes: page_bytes,
            page_image_sha256: page_sha256,
            normalized_dom_token_ids: fixture.dom_tokens.clone(),
            dom_manifest_sha256: dom_manifest_hash(&fixture.dom_tokens),
            viewport_bounds: fixture.viewport_bounds,
            observed_at_ms,
        },
        captured_at_ms: observed_at_ms,
        observed_at_ms,
    })
}

#[cfg(test)]
fn render_page(fixture: &FixtureManifest, theme: Theme) -> Result<RgbaImage, DesktopPromptError> {
    let mut page = RgbaImage::from_pixel(
        fixture.viewport_bounds.width,
        fixture.viewport_bounds.height,
        theme.page(),
    );
    for prompt in &fixture.page_prompts {
        draw_prompt(
            &mut page,
            PixelBounds {
                x: prompt.x,
                y: prompt.y,
                width: scaled(80, fixture.scale_millis)?,
                height: scaled(48, fixture.scale_millis)?,
            },
            theme,
            prompt.occlusion_percent,
        )?;
    }
    Ok(page)
}

#[cfg(test)]
fn draw_prompt(
    image: &mut RgbaImage,
    bounds: PixelBounds,
    theme: Theme,
    occlusion_percent: u32,
) -> Result<(), DesktopPromptError> {
    if occlusion_percent > 100 {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture occlusion is outside fixed bounds",
        ));
    }
    fill_rect(image, bounds, theme.prompt())?;
    let accent = Rgba([126, 34, 206, 255]);
    let right = bounds.right().ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_detector_failed",
            "fixture prompt overflowed",
        )
    })?;
    let bottom = bounds.bottom().ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_detector_failed",
            "fixture prompt overflowed",
        )
    })?;
    for offset in 0..2 {
        for x in bounds.x..right {
            image.put_pixel(x, bounds.y + offset, accent);
            image.put_pixel(x, bottom - 1 - offset, accent);
        }
        for y in bounds.y..bottom {
            image.put_pixel(bounds.x + offset, y, accent);
            image.put_pixel(right - 1 - offset, y, accent);
        }
    }
    for x in bounds.x + bounds.width / 4..bounds.x + (bounds.width * 3) / 4 {
        image.put_pixel(x, bounds.y + bounds.height / 2, accent);
    }
    let occluded_width = bounds.width.saturating_mul(occlusion_percent) / 100;
    if occluded_width > 0 {
        let occluded_height = if occlusion_percent <= 20 {
            (bounds.height / 4).max(1)
        } else {
            bounds.height
        };
        fill_rect(
            image,
            PixelBounds {
                x: right - occluded_width,
                y: bottom - occluded_height,
                width: occluded_width,
                height: occluded_height,
            },
            theme.desktop(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
fn fill_rect(
    image: &mut RgbaImage,
    bounds: PixelBounds,
    color: Rgba<u8>,
) -> Result<(), DesktopPromptError> {
    let right = bounds.right().ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture rectangle overflowed",
        )
    })?;
    let bottom = bounds.bottom().ok_or_else(|| {
        DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture rectangle overflowed",
        )
    })?;
    if bounds.width == 0 || bounds.height == 0 || right > image.width() || bottom > image.height() {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture rectangle exceeds its independently rendered surface",
        ));
    }
    for y in bounds.y..bottom {
        for x in bounds.x..right {
            image.put_pixel(x, y, color);
        }
    }
    Ok(())
}

#[cfg(test)]
fn copy_layer(
    desktop: &mut RgbaImage,
    page: &RgbaImage,
    viewport: PixelBounds,
) -> Result<(), DesktopPromptError> {
    if page.width() != viewport.width || page.height() != viewport.height {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture page and viewport dimensions disagree",
        ));
    }
    if viewport
        .right()
        .is_some_and(|right| right > desktop.width())
        || viewport
            .bottom()
            .is_some_and(|bottom| bottom > desktop.height())
    {
        return Err(DesktopPromptError::new(
            "desktop_prompt_page_evidence_invalid",
            "fixture viewport exceeds the desktop scene",
        ));
    }
    for y in 0..page.height() {
        for x in 0..page.width() {
            desktop.put_pixel(viewport.x + x, viewport.y + y, *page.get_pixel(x, y));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matched_fixture_is_external_only_when_page_inputs_are_absent() {
        let result = observe_repository_fixture("matched-light-100", true)
            .expect("repository fixture should be observable");

        assert_eq!(result.prompt_observation.detection_status, "matched");
        assert_eq!(result.prompt_observation.page_visibility, "absent");
        assert_eq!(result.prompt_observation.classification, "browser_external");
        assert_eq!(
            result.prompt_observation.handling_outcome,
            "actionable_observation"
        );
        assert_eq!(
            result.prompt_observation.blindness_receipt.claim,
            "absent_from_fixture_page_inputs"
        );
        assert_eq!(
            result
                .prompt_observation
                .blindness_receipt
                .correspondence_state,
            "verified"
        );
        assert_eq!(
            result
                .prompt_observation
                .blindness_receipt
                .page_screenshot_prompt_match_count,
            0
        );
        assert_eq!(
            result
                .prompt_observation
                .blindness_receipt
                .dom_prompt_match_count,
            0
        );
        assert!(result.visualization_bytes.is_some());
    }

    #[test]
    fn repository_corpus_has_frozen_typed_outcomes() {
        for source in FIXTURE_SOURCES {
            let fixture: FixtureManifest = serde_json::from_str(source).expect("valid manifest");
            let result = observe_repository_fixture(&fixture.fixture_id, false);
            if fixture.renderer_version != FIXTURE_PROVIDER_VERSION {
                assert_eq!(
                    result.unwrap_err().code(),
                    "desktop_prompt_page_evidence_invalid"
                );
                continue;
            }
            if fixture.stale_by_ms > MAX_EVIDENCE_AGE_MS {
                assert_eq!(result.unwrap_err().code(), "desktop_prompt_stale_evidence");
                continue;
            }
            if fixture.binding_mismatch {
                assert_eq!(
                    result.unwrap_err().code(),
                    "desktop_prompt_binding_mismatch"
                );
                continue;
            }
            let result = result.expect("valid repository fixture");
            assert_eq!(
                result.prompt_observation.detection_status, fixture.expected_detection_status,
                "{} detection status",
                fixture.fixture_id
            );
            assert_eq!(
                result.prompt_observation.page_visibility, fixture.expected_page_visibility,
                "{} page visibility",
                fixture.fixture_id
            );
            assert_eq!(
                result.prompt_observation.classification, fixture.expected_classification,
                "{} classification",
                fixture.fixture_id
            );
            assert_eq!(
                result.prompt_observation.handling_outcome, fixture.expected_handling_outcome,
                "{} handling outcome",
                fixture.fixture_id
            );
        }
    }

    #[test]
    fn corpus_manifest_and_evidence_hashes_match_literal_goldens() {
        let corpus: Value = serde_json::from_str(CORPUS_INDEX_SOURCE).expect("valid corpus index");
        let expected_manifest_hashes = corpus["manifestSha256"]
            .as_object()
            .expect("corpus manifest hash ledger");
        let mut mismatches = Vec::new();
        for source in FIXTURE_SOURCES {
            let fixture: FixtureManifest = serde_json::from_str(source).expect("valid manifest");
            let manifest_sha256 = manifest_projection_sha256(source);
            let expected_manifest = expected_manifest_hashes
                .get(&fixture.fixture_id)
                .and_then(Value::as_str)
                .unwrap_or("missing");
            if manifest_sha256 != expected_manifest {
                mismatches.push(format!(
                    "{} manifestSha256={manifest_sha256}",
                    fixture.fixture_id
                ));
            }
            let Some(expected) = fixture.golden_hashes.as_ref() else {
                continue;
            };
            if expected.manifest_sha256 != manifest_sha256 {
                mismatches.push(format!(
                    "{} golden.manifestSha256={manifest_sha256}",
                    fixture.fixture_id
                ));
            }
            let bundle = render_fixture(&fixture, 10_000).expect("golden fixture renders");
            let desktop = decode_png(
                &bundle.frame.image_bytes,
                bundle.frame.context.width,
                bundle.frame.context.height,
                "desktop_prompt_invalid_image",
            )
            .unwrap();
            let viewport_layer = derive_viewport_layer(&desktop, bundle.surface.viewport_bounds)
                .expect("golden viewport derives");
            let page_sha256 = bundle.page.page_image_sha256.clone();
            let dom_sha256 = bundle.page.dom_manifest_sha256.clone();
            let desktop_sha256 = bundle.frame.frame_receipt.content_sha256.clone();
            let result =
                observe_desktop_prompt(bundle, PROMPT_PROFILE_ID, true, &FixedClock(10_000))
                    .expect("golden fixture observes");
            let actual = FixtureGoldenHashes {
                manifest_sha256,
                profile_sha256: result.prompt_observation.profile_sha256.clone(),
                renderer_sha256: renderer_sha256(),
                page_image_sha256: page_sha256,
                dom_manifest_sha256: dom_sha256,
                desktop_frame_sha256: desktop_sha256,
                viewport_layer_sha256: digest_bytes(&viewport_layer),
                detector_sha256: digest_bytes(
                    &serde_json::to_vec(&result.prompt_observation.detector_receipts).unwrap(),
                ),
                observation_sha256: digest_bytes(
                    &serde_json::to_vec(&result.prompt_observation).unwrap(),
                ),
                visualization_sha256: digest_bytes(
                    result
                        .visualization_bytes
                        .as_deref()
                        .expect("golden visualization"),
                ),
                paired_receipt_sha256: digest_bytes(
                    &serde_json::to_vec(&result.prompt_observation.blindness_receipt).unwrap(),
                ),
            };
            if expected != &actual {
                mismatches.push(format!(
                    "{} goldenHashes={}",
                    fixture.fixture_id,
                    serde_json::to_string(&actual).unwrap()
                ));
            }
        }
        let mut corpus_projection = corpus.clone();
        corpus_projection
            .as_object_mut()
            .expect("corpus object")
            .remove("corpusSha256");
        let actual_corpus_sha256 = digest_bytes(&serde_json::to_vec(&corpus_projection).unwrap());
        if corpus["corpusSha256"].as_str() != Some(actual_corpus_sha256.as_str()) {
            mismatches.push(format!("corpusSha256={actual_corpus_sha256}"));
        }
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }

    #[test]
    fn binding_and_freshness_fail_before_prompt_detection() {
        let mismatch = observe_repository_fixture("stale-binding", false).unwrap_err();
        assert_eq!(mismatch.code(), "desktop_prompt_binding_mismatch");

        let stale = observe_repository_fixture("stale-evidence", false).unwrap_err();
        assert_eq!(stale.code(), "desktop_prompt_stale_evidence");
    }

    #[test]
    fn replaced_desktop_viewport_fails_even_with_an_updated_frame_hash() {
        let fixture = fixture_manifest("matched-light-100").unwrap();
        let mut bundle = render_fixture(&fixture, 10_000).unwrap();
        let mut desktop = decode_png(
            &bundle.frame.image_bytes,
            bundle.frame.context.width,
            bundle.frame.context.height,
            "desktop_prompt_invalid_image",
        )
        .unwrap();
        let viewport = bundle.surface.viewport_bounds;
        desktop.put_pixel(viewport.x, viewport.y, Rgba([1, 2, 3, 255]));
        bundle.frame.image_bytes = encode_png(&desktop).unwrap();
        bundle.frame.frame_receipt.byte_length = bundle.frame.image_bytes.len();
        bundle.frame.frame_receipt.content_sha256 = digest_bytes(&bundle.frame.image_bytes);
        bundle.frame.frame_receipt.frame_id = format!(
            "desktop-frame-{}",
            &bundle.frame.frame_receipt.content_sha256[..24]
        );

        let error = observe_desktop_prompt(bundle, PROMPT_PROFILE_ID, false, &FixedClock(10_000))
            .unwrap_err();

        assert_eq!(error.code(), "desktop_prompt_page_evidence_invalid");
    }

    #[test]
    fn malformed_oversized_overflowed_and_versioned_evidence_fail_typed() {
        let fixture = fixture_manifest("matched-light-100").unwrap();

        assert_eq!(
            scaled(u32::MAX, u32::MAX).unwrap_err().code(),
            "desktop_prompt_detector_failed"
        );

        let mut malformed = render_fixture(&fixture, 10_000).unwrap();
        malformed.frame.image_bytes = b"not-a-png".to_vec();
        malformed.frame.frame_receipt.byte_length = malformed.frame.image_bytes.len();
        malformed.frame.frame_receipt.content_sha256 = digest_bytes(&malformed.frame.image_bytes);
        let error =
            observe_desktop_prompt(malformed, PROMPT_PROFILE_ID, false, &FixedClock(10_000))
                .unwrap_err();
        assert_eq!(error.code(), "desktop_prompt_invalid_image");

        let mut oversized = render_fixture(&fixture, 10_000).unwrap();
        oversized.frame.context.width = 5_000;
        oversized.frame.context.height = 5_000;
        oversized.frame.frame_receipt.width = 5_000;
        oversized.frame.frame_receipt.height = 5_000;
        oversized.surface.width = 5_000;
        oversized.surface.height = 5_000;
        let error =
            observe_desktop_prompt(oversized, PROMPT_PROFILE_ID, false, &FixedClock(10_000))
                .unwrap_err();
        assert_eq!(error.code(), "desktop_prompt_invalid_image");

        let mut overflowed = render_fixture(&fixture, 10_000).unwrap();
        overflowed.frame.context.scale_factor = u32::MAX as f64 / 1000.0;
        overflowed.frame.frame_receipt.scale_factor = u32::MAX as f64 / 1000.0;
        overflowed.surface.scale_millis = u32::MAX;
        let error =
            observe_desktop_prompt(overflowed, PROMPT_PROFILE_ID, false, &FixedClock(10_000))
                .unwrap_err();
        assert_eq!(error.code(), "desktop_prompt_page_evidence_invalid");

        let mut versioned = render_fixture(&fixture, 10_000).unwrap();
        versioned.page.renderer_version = "unsupported-page-renderer-v9".to_string();
        let error =
            observe_desktop_prompt(versioned, PROMPT_PROFILE_ID, false, &FixedClock(10_000))
                .unwrap_err();
        assert_eq!(error.code(), "desktop_prompt_page_evidence_invalid");
    }

    #[test]
    fn page_lookalikes_never_classify_as_browser_external() {
        for fixture_id in ["external-page-lookalike", "page-decoy-only"] {
            let result = observe_repository_fixture(fixture_id, false).unwrap();
            assert_eq!(result.prompt_observation.page_visibility, "present");
            assert_ne!(result.prompt_observation.classification, "browser_external");
            assert!(result.prompt_observation.selected_candidate_id.is_none());
        }
    }

    #[test]
    fn ambiguous_and_manual_only_scenes_require_no_effect_operator_review() {
        for fixture_id in ["ambiguous-external", "native-modal-manual"] {
            let result = observe_repository_fixture(fixture_id, false).unwrap();
            assert_eq!(
                result.prompt_observation.handling_outcome,
                "operator_intervention_required"
            );
            let intervention = result.prompt_observation.operator_intervention.unwrap();
            assert_eq!(intervention.state, "required");
            let serialized = serde_json::to_value(intervention).unwrap();
            assert!(serialized.get("url").is_none());
            assert!(serialized.get("command").is_none());
        }
    }

    #[test]
    fn repeated_observation_and_visualization_are_byte_identical() {
        let first = observe_repository_fixture("matched-dark-125", true).unwrap();
        let second = observe_repository_fixture("matched-dark-125", true).unwrap();
        assert_eq!(
            serde_json::to_vec(&first.prompt_observation).unwrap(),
            serde_json::to_vec(&second.prompt_observation).unwrap()
        );
        assert_eq!(first.visualization_bytes, second.visualization_bytes);
    }

    #[tokio::test]
    async fn configured_dispatch_is_unavailable_before_evidence_resolution() {
        let error = handle_desktop_prompt_observe(&serde_json::json!({
            "action": "desktop_prompt_observe",
            "browserId": "must-not-be-resolved"
        }))
        .await
        .unwrap_err();
        assert!(error.starts_with("desktop_prompt_provider_unavailable:"));
    }

    #[test]
    fn strict_redactor_excludes_all_response_only_and_private_evidence() {
        let input = serde_json::json!({
            "ok": true,
            "action": "desktop_prompt_observe",
            "context": {
                "contextId": "context-1",
                "browserId": "browser-1",
                "providerUrl": "private-provider",
                "readinessEvidence": {"private": true}
            },
            "frameReceipt": {
                "frameId": "frame-1",
                "sha256": "safe-hash",
                "providerVersion": "private-provider-version"
            },
            "promptObservation": {
                "observationId": "observation-1",
                "detectionStatus": "matched",
                "pageVisibility": "absent",
                "classification": "browser_external",
                "handlingOutcome": "actionable_observation",
                "rawPromptText": "FORBIDDEN_PROMPT_SENTINEL",
                "pageImageBase64": "FORBIDDEN_PAGE_SENTINEL",
                "domManifest": "FORBIDDEN_DOM_SENTINEL"
            },
            "visualizationBase64": "FORBIDDEN_VISUALIZATION_SENTINEL",
            "desktopFrameBase64": "FORBIDDEN_DESKTOP_SENTINEL"
        });
        let redacted = redact_desktop_prompt_stream_result(&input);
        let serialized = serde_json::to_string(&redacted).unwrap();
        for forbidden in [
            "FORBIDDEN_PROMPT_SENTINEL",
            "FORBIDDEN_PAGE_SENTINEL",
            "FORBIDDEN_DOM_SENTINEL",
            "FORBIDDEN_VISUALIZATION_SENTINEL",
            "FORBIDDEN_DESKTOP_SENTINEL",
            "private-provider",
        ] {
            assert!(!serialized.contains(forbidden));
        }
        assert_eq!(redacted["visualizationPayload"], "response_only");
    }

    #[test]
    fn unknown_profile_is_typed_before_detection() {
        let fixture = fixture_manifest("matched-light-100").unwrap();
        let bundle = render_fixture(&fixture, 10_000).unwrap();
        let error = observe_desktop_prompt(bundle, "caller-profile", false, &FixedClock(10_000))
            .unwrap_err();
        assert_eq!(error.code(), "desktop_prompt_profile_not_found");
    }
}
