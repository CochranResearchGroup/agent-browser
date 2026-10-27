//! One explicitly registered static hCaptcha image grid. Pixels and OCR stay
//! process-local; returned geometry is evidence, never caller input authority.
use super::browser_native_control::NativeChallengeBounds;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const PROFILE_ID: &str = "hcaptcha-image-grid-3x3-v1";
pub const CONFIG_REF: &str = "agent-browser:remote-view-native-image-static-3x3:v1";
pub const MAX_AGE_MS: u64 = 60_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeImageCandidate {
    pub candidate_id: String,
    pub bounds: NativeChallengeBounds,
}

/// The default preserves the historical complete-window contract. A region is
/// accepted only when explicitly requested before this owner observation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureScope {
    #[default]
    CompleteWindow,
    CompletePuzzleRegion,
}

/// Owner-observed static grid, bound to one exact full Native frame. No OCR,
/// image bytes, URL, executable path or provider-selected coordinates persist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeImageObservation {
    #[serde(default)]
    pub capture_scope: CaptureScope,
    pub desktop_id: String,
    pub generation: u64,
    pub window_id: u64,
    pub process_digest: String,
    pub window: NativeChallengeBounds,
    pub display_width: u32,
    pub display_height: u32,
    pub region: NativeChallengeBounds,
    pub frame_sha256: String,
    pub native_control_epoch: u64,
    pub observed_at_ms: u64,
    pub registered_action_id: String,
    pub puzzle_sha256: String,
    pub candidates: Vec<NativeImageCandidate>,
    pub verify: NativeChallengeBounds,
}

pub fn digest(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_value(value)
        .and_then(|value| serde_json::to_vec(&value))
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| "browser_native_image_observation_invalid".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
impl Rect {
    fn right(self) -> u32 {
        self.x + self.width
    }
    fn bottom(self) -> u32 {
        self.y + self.height
    }
    fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
    fn bounds(self, region: &NativeChallengeBounds) -> NativeChallengeBounds {
        NativeChallengeBounds {
            x: self.x as i32 + region.x,
            y: self.y as i32 + region.y,
            width: self.width,
            height: self.height,
        }
    }
}

fn ocr_rect(value: &Value, region: &NativeChallengeBounds) -> Option<Rect> {
    let x = u32::try_from(value["x"].as_i64()?.checked_sub(i64::from(region.x))?).ok()?;
    let y = u32::try_from(value["y"].as_i64()?.checked_sub(i64::from(region.y))?).ok()?;
    let width = u32::try_from(value["width"].as_u64()?).ok()?;
    let height = u32::try_from(value["height"].as_u64()?).ok()?;
    if width == 0
        || height == 0
        || x.checked_add(width)? > region.width
        || y.checked_add(height)? > region.height
    {
        return None;
    }
    Some(Rect {
        x,
        y,
        width,
        height,
    })
}
fn unique_line(ocr: &Value, text: &str, region: &NativeChallengeBounds) -> Option<Rect> {
    let lines = ocr["lines"].as_array()?;
    let found = lines
        .iter()
        .filter(|line| {
            line["confidence"]
                .as_f64()
                .is_some_and(|v| v.is_finite() && v >= 70.0)
                && line["text"].as_str().is_some_and(|v| {
                    v.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .eq_ignore_ascii_case(text)
                })
        })
        .collect::<Vec<_>>();
    let [line] = found.as_slice() else {
        return None;
    };
    ocr_rect(&line["bounds"], region)
}

/// Connected dark components propose rectangles; every perimeter is independently
/// checked. Glyphs or a textual grid description cannot manufacture tile geometry.
fn rectangles(image: &image::RgbImage) -> Vec<Rect> {
    let (width, height) = image.dimensions();
    let dark = |x: u32, y: u32| image.get_pixel(x, y).0.iter().all(|channel| *channel <= 80);
    let mut seen = vec![false; (width * height) as usize];
    let mut result = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            if seen[index] || !dark(x, y) {
                continue;
            }
            seen[index] = true;
            let mut queue = vec![(x, y)];
            let mut head = 0;
            let (mut left, mut top, mut right, mut bottom) = (x, y, x, y);
            while head < queue.len() {
                let (cx, cy) = queue[head];
                head += 1;
                left = left.min(cx);
                top = top.min(cy);
                right = right.max(cx);
                bottom = bottom.max(cy);
                for (nx, ny) in [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ] {
                    if nx >= width || ny >= height {
                        continue;
                    }
                    let index = (ny * width + nx) as usize;
                    if !seen[index] && dark(nx, ny) {
                        seen[index] = true;
                        queue.push((nx, ny));
                    }
                }
            }
            let rect = Rect {
                x: left,
                y: top,
                width: right - left + 1,
                height: bottom - top + 1,
            };
            if !(24..=160).contains(&rect.width) || !(24..=128).contains(&rect.height) {
                continue;
            }
            let mut perimeter = 0;
            let mut matched = 0;
            for px in left..=right {
                for py in [top, bottom] {
                    perimeter += 1;
                    if dark(px, py) {
                        matched += 1;
                    }
                }
            }
            for py in top + 1..bottom {
                for px in [left, right] {
                    perimeter += 1;
                    if dark(px, py) {
                        matched += 1;
                    }
                }
            }
            if matched * 100 >= perimeter * 95 {
                result.push(rect);
            }
        }
    }
    result
}

/// Observes only the registered English bus prompt, one unique static 3x3
/// square-tile layout and its visible Verify button. Semantics are supplied by
/// an external image reader later; this detector never chooses tiles.
pub fn observe(
    png: &[u8],
    ocr: &Value,
    mut observation: NativeImageObservation,
) -> Result<Option<NativeImageObservation>, String> {
    if ocr["state"] != "observed" {
        return Ok(None);
    }
    let region = &observation.region;
    if ocr["lines"].as_array().is_some_and(|lines| {
        lines.iter().any(|line| {
            line["confidence"].as_f64().is_some_and(|v| v >= 70.0)
                && line["text"].as_str().is_some_and(|text| {
                    [
                        "new images",
                        "replace selected",
                        "keep selecting",
                        "once there are none",
                        "click each image",
                        "select all squares",
                    ]
                    .iter()
                    .any(|marker| text.to_ascii_lowercase().contains(marker))
                })
        })
    }) {
        return Ok(None);
    }
    let Some(prompt) = unique_line(ocr, "Select all images containing buses", region) else {
        return Ok(None);
    };
    let Some(brand) = unique_line(ocr, "hCaptcha", region) else {
        return Ok(None);
    };
    let Some(verify_word) = unique_line(ocr, "Verify", region) else {
        return Ok(None);
    };
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(png), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(region.width);
    limits.max_image_height = Some(region.height);
    limits.max_alloc = Some(16_000_000);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| "browser_native_image_pixels_invalid")?;
    if decoded.to_rgba8().pixels().any(|pixel| pixel.0[3] != 255) {
        return Ok(None);
    }
    let image = decoded.to_rgb8();
    if image.dimensions() != (region.width, region.height)
        || u64::from(region.width) * u64::from(region.height) > 1_000_000
    {
        return Err("browser_native_image_pixels_invalid".into());
    }
    let rects = rectangles(&image);
    let mut tiles = rects
        .iter()
        .copied()
        .filter(|r| {
            (48..=128).contains(&r.width)
                && r.width == r.height
                && r.y > prompt.bottom()
                && r.bottom() < verify_word.y
        })
        .collect::<Vec<_>>();
    if tiles.len() != 9 {
        return Ok(None);
    }
    tiles.sort_by_key(|r| (r.y, r.x));
    let first = tiles[0];
    let gap_x = tiles[1].x.checked_sub(first.right());
    let gap_y = tiles[3].y.checked_sub(first.bottom());
    let (Some(gap_x @ 2..=8), Some(gap_y @ 2..=8)) = (gap_x, gap_y) else {
        return Ok(None);
    };
    if tiles.iter().enumerate().any(|(index, tile)| {
        tile.width != first.width
            || tile.height != first.height
            || tile.x != first.x + (index as u32 % 3) * (first.width + gap_x)
            || tile.y != first.y + (index as u32 / 3) * (first.height + gap_y)
    }) {
        return Ok(None);
    }
    if first.y - prompt.bottom() > 64
        || brand.y.abs_diff(prompt.y) > 64
        || brand.x.abs_diff(first.x) > 400
    {
        return Ok(None);
    }
    let buttons = rects
        .iter()
        .copied()
        .filter(|r| {
            r.width >= 48
                && r.width >= r.height * 2
                && r.x >= first.x
                && r.right() <= tiles[2].right()
                && r.contains(verify_word)
                && r.y >= tiles[8].bottom()
                && r.y - tiles[8].bottom() <= 64
        })
        .collect::<Vec<_>>();
    let [verify] = buttons.as_slice() else {
        return Ok(None);
    };
    observation.candidates = tiles
        .iter()
        .enumerate()
        .map(|(index, bounds)| NativeImageCandidate {
            candidate_id: format!("tile:{}:{}", index + 1, &observation.frame_sha256[..16]),
            bounds: bounds.bounds(region),
        })
        .collect();
    observation.verify = verify.bounds(region);
    observation.puzzle_sha256 = digest(&(
        &observation.frame_sha256,
        &observation.candidates,
        &observation.verify,
        PROFILE_ID,
    ))?;
    Ok(Some(observation))
}

impl NativeImageObservation {
    pub fn validate(&self, now_ms: u64) -> Result<(), String> {
        let hex = |v: &str| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit());
        let contains = |b: &NativeChallengeBounds| {
            b.width > 0
                && b.height > 0
                && b.x >= self.region.x
                && b.y >= self.region.y
                && i64::from(b.x) + i64::from(b.width)
                    <= i64::from(self.region.x) + i64::from(self.region.width)
                && i64::from(b.y) + i64::from(b.height)
                    <= i64::from(self.region.y) + i64::from(self.region.height)
        };
        if self.registered_action_id != PROFILE_ID
            || self.desktop_id.is_empty()
            || self.desktop_id.len() > 256
            || !hex(&self.frame_sha256)
            || !hex(&self.process_digest)
            || !hex(&self.puzzle_sha256)
            || (self.capture_scope == CaptureScope::CompleteWindow && self.region != self.window)
            || self.region.width == 0
            || self.region.height == 0
            || self.region.x < self.window.x
            || self.region.y < self.window.y
            || i64::from(self.region.x) + i64::from(self.region.width)
                > i64::from(self.window.x) + i64::from(self.window.width)
            || i64::from(self.region.y) + i64::from(self.region.height)
                > i64::from(self.window.y) + i64::from(self.window.height)
            || self.window.x < 0
            || self.window.y < 0
            || self.display_width > u32::from(u16::MAX)
            || self.display_height > u32::from(u16::MAX)
            || self.window.width == 0
            || self.window.height == 0
            || u64::from(self.region.width) * u64::from(self.region.height) > 1_000_000
            || i64::from(self.window.x) + i64::from(self.window.width)
                > i64::from(self.display_width)
            || i64::from(self.window.y) + i64::from(self.window.height)
                > i64::from(self.display_height)
            || self.candidates.len() != 9
            || !contains(&self.verify)
        {
            return Err("browser_native_image_observation_invalid".into());
        }
        let first = &self.candidates[0].bounds;
        if !(48..=128).contains(&first.width) || first.width != first.height {
            return Err("browser_native_image_observation_invalid".into());
        }
        let gap_x =
            i64::from(self.candidates[1].bounds.x) - i64::from(first.x) - i64::from(first.width);
        let gap_y =
            i64::from(self.candidates[3].bounds.y) - i64::from(first.y) - i64::from(first.height);
        if !(2..=8).contains(&gap_x)
            || !(2..=8).contains(&gap_y)
            || self.candidates.iter().enumerate().any(|(index, c)| {
                !contains(&c.bounds)
                    || c.candidate_id != format!("tile:{}:{}", index + 1, &self.frame_sha256[..16])
                    || c.bounds.width != first.width
                    || c.bounds.height != first.height
                    || i64::from(c.bounds.x)
                        != i64::from(first.x)
                            + (index as i64 % 3) * (i64::from(first.width) + gap_x)
                    || i64::from(c.bounds.y)
                        != i64::from(first.y)
                            + (index as i64 / 3) * (i64::from(first.height) + gap_y)
            })
            || self.verify.x < first.x
            || i64::from(self.verify.x) + i64::from(self.verify.width)
                > i64::from(self.candidates[2].bounds.x) + i64::from(first.width)
            || i64::from(self.verify.y)
                < i64::from(self.candidates[8].bounds.y) + i64::from(first.height)
            || self.puzzle_sha256
                != digest(&(
                    &self.frame_sha256,
                    &self.candidates,
                    &self.verify,
                    PROFILE_ID,
                ))?
        {
            return Err("browser_native_image_observation_invalid".into());
        }
        if now_ms < self.observed_at_ms || now_ms - self.observed_at_ms > MAX_AGE_MS {
            return Err(
                "browser_native_image_observation_expired:human assistance required".into(),
            );
        }
        Ok(())
    }
    /// Only owner-observed candidate IDs become points. Caller coordinates,
    /// effect counts and arbitrary provider actions are never read.
    pub fn actions(&self, selected: &[String]) -> Result<Vec<Value>, String> {
        if selected.is_empty()
            || selected.len() > 4
            || selected
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != selected.len()
            || selected
                .iter()
                .any(|id| !self.candidates.iter().any(|c| &c.candidate_id == id))
        {
            return Err("browser_native_image_selection_invalid".into());
        }
        Ok(self.candidates.iter().filter(|c|selected.contains(&c.candidate_id)).map(|c|&c.bounds).chain(std::iter::once(&self.verify)).map(|b|serde_json::json!({"kind":"click","x":b.x+i32::try_from(b.width/2).unwrap(),"y":b.y+i32::try_from(b.height/2).unwrap(),"button":1,"count":1})).collect())
    }
}

pub struct NativeImagePlan {
    pub policy: agent_browser_challenge_control::VisualRoundPolicy,
    pub snapshot: agent_browser_challenge_control::VisualRoundSnapshot,
    pub intent: agent_browser_challenge_control::VisualRoundIntent,
}

/// Reuse the pure visual-round state machine behind the existing durable task
/// journal. One pending original consumes the shared taskwide attempt budget.
pub fn plan(
    task: &str,
    attempt: &str,
    observation: &NativeImageObservation,
    selected: &[String],
    binding: &Value,
    now_ms: u64,
) -> Result<NativeImagePlan, String> {
    use agent_browser_challenge_control::*;
    observation.validate(now_ms)?;
    let actions = observation.actions(selected)?;
    let policy_digest = binding["digest"]
        .as_str()
        .filter(|v| v.len() == 64)
        .ok_or("browser_native_site_policy_invalid")?
        .to_owned();
    let provider = binding["providerId"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or("browser_native_site_policy_provider_missing")?;
    let profile_digest = digest(&PROFILE_ID)?;
    let capability = VisualProviderCapability {
        capability_id: provider.into(),
        capability_version: PROFILE_ID.into(),
        capability_digest: digest(&(provider, CONFIG_REF, &policy_digest))?,
    };
    let policy = VisualRoundPolicy {
        policy_digest: policy_digest.clone(),
        profile_digest: profile_digest.clone(),
        provider_capability: capability.clone(),
        max_rounds: 1,
        max_selections_per_round: 4,
        max_steps_per_round: 1,
        max_pointer_events_per_round: 15,
        max_key_events_per_round: 0,
        max_total_selections: 4,
        max_total_steps: 1,
        max_total_pointer_events: 15,
        max_total_key_events: 0,
        deadline_at_ms: observation
            .observed_at_ms
            .checked_add(MAX_AGE_MS)
            .ok_or("browser_native_image_observation_invalid")?,
    };
    let candidate_ids = observation
        .candidates
        .iter()
        .map(|c| c.candidate_id.clone())
        .collect::<Vec<_>>();
    let geometry_digest = if observation.capture_scope == CaptureScope::CompletePuzzleRegion {
        digest(&(
            observation.capture_scope,
            &observation.region,
            &observation.window,
            observation.display_width,
            observation.display_height,
            &observation.candidates,
            &observation.verify,
        ))?
    } else {
        digest(&(
            &observation.window,
            observation.display_width,
            observation.display_height,
            &observation.candidates,
            &observation.verify,
        ))?
    };
    let mut evidence = VisualRoundEvidence {
        task_id: task.into(),
        attempt_id: attempt.into(),
        profile_digest,
        policy_digest,
        round_index: 1,
        round_id: observation.puzzle_sha256.clone(),
        evidence_digest: String::new(),
        frame_digest: observation.frame_sha256.clone(),
        context_digest: digest(&(
            &observation.desktop_id,
            observation.generation,
            observation.window_id,
            &observation.process_digest,
            observation.native_control_epoch,
        ))?,
        geometry_digest,
        candidate_set_digest: visual_candidate_set_digest(&candidate_ids),
        candidate_ids,
        observed_at_ms: observation.observed_at_ms,
        expires_at_ms: policy.deadline_at_ms,
        provider_capability: capability.clone(),
    };
    evidence.evidence_digest = visual_round_evidence_digest(&evidence);
    let initial = VisualRoundSnapshot::new(task, attempt)
        .map_err(|_| "browser_native_image_round_invalid")?;
    let observed = decide_visual_round(
        &policy,
        &initial,
        VisualRoundEvent::Observe {
            evidence: evidence.clone(),
            now_ms,
        },
    )
    .map_err(|_| "browser_native_image_round_invalid")?;
    let selected = VisualRoundSelection {
        round_id: evidence.round_id,
        evidence_digest: evidence.evidence_digest,
        candidate_set_digest: evidence.candidate_set_digest,
        selected_candidate_ids: selected.to_vec(),
        planned_steps: 1,
        planned_pointer_events: u8::try_from(actions.len() * 3)
            .map_err(|_| "browser_native_image_selection_invalid")?,
        planned_key_events: 0,
        provider_capability: capability,
    };
    let decision = decide_visual_round(
        &policy,
        observed.snapshot(),
        VisualRoundEvent::Select {
            selection: selected,
            now_ms,
        },
    )
    .map_err(|_| "browser_native_image_round_invalid")?;
    let VisualRoundDecision::PermitIntent { snapshot, intent } = decision else {
        return Err("browser_native_image_round_refused".into());
    };
    validate_visual_round_intent(&policy, &snapshot, &intent)
        .map_err(|_| "browser_native_image_round_invalid")?;
    Ok(NativeImagePlan {
        policy,
        snapshot,
        intent: *intent,
    })
}

/// Presence of imageAttempt always selects this typed path, including malformed
/// values. It cannot fall through to legacy arbitrary Native input.
pub fn selection(command: &Value) -> Result<Vec<String>, String> {
    if command["action"] != "native_input"
        || ["actions", "nativeAttempt", "controlEpoch"]
            .iter()
            .any(|field| command.get(*field).is_some())
    {
        return Err("browser_native_image_attempt_invalid".into());
    }
    let attempt = command["imageAttempt"]
        .as_object()
        .filter(|a| {
            a.len() == 2
                && a.contains_key("observationRequestId")
                && a.contains_key("selectedCandidateIds")
        })
        .ok_or("browser_native_image_attempt_invalid")?;
    if !attempt["observationRequestId"].as_str().is_some_and(|id| {
        !id.is_empty()
            && id.len() <= 128
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
    }) {
        return Err("browser_native_original_observation_request_invalid".into());
    }
    let selected: Vec<String> = serde_json::from_value(attempt["selectedCandidateIds"].clone())
        .map_err(|_| "browser_native_image_selection_invalid")?;
    if selected.is_empty()
        || selected.len() > 4
        || selected.iter().any(|id| id.is_empty() || id.len() > 128)
        || selected
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != selected.len()
    {
        return Err("browser_native_image_selection_invalid".into());
    }
    Ok(selected)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RetainedRound {
    snapshot: agent_browser_challenge_control::VisualRoundSnapshot,
    intent: agent_browser_challenge_control::VisualRoundIntent,
    deadline_at_ms: u64,
}
pub fn retain_plan(plan: &NativeImagePlan) -> Result<String, String> {
    let value = serde_json::to_string(&RetainedRound {
        snapshot: plan.snapshot.clone(),
        intent: plan.intent.clone(),
        deadline_at_ms: plan.policy.deadline_at_ms,
    })
    .map_err(|_| "browser_native_image_round_invalid")?;
    if value.len() > 32768 {
        return Err("browser_native_image_round_invalid".into());
    }
    Ok(value)
}
pub fn check_retained_plan(encoded: &str, plan: &NativeImagePlan) -> Result<(), String> {
    if encoded.len() > 32768 {
        return Err("browser_native_image_round_invalid".into());
    }
    let retained: RetainedRound =
        serde_json::from_str(encoded).map_err(|_| "browser_native_image_round_invalid")?;
    if retained.intent != plan.intent
        || retained.snapshot != plan.snapshot
        || retained.deadline_at_ms != plan.policy.deadline_at_ms
    {
        return Err("browser_native_image_round_changed".into());
    }
    Ok(())
}
pub fn finish_round(
    encoded: &str,
    acknowledged: bool,
    request: &str,
) -> Result<(String, String), String> {
    use agent_browser_challenge_control::*;
    if encoded.len() > 32768 {
        return Err("browser_native_image_round_invalid".into());
    }
    let retained: RetainedRound =
        serde_json::from_str(encoded).map_err(|_| "browser_native_image_round_invalid")?;
    let evidence = retained
        .snapshot
        .active_evidence
        .as_ref()
        .ok_or("browser_native_image_round_invalid")?;
    let policy = VisualRoundPolicy {
        policy_digest: evidence.policy_digest.clone(),
        profile_digest: digest(&PROFILE_ID)?,
        provider_capability: retained.intent.provider_capability.clone(),
        max_rounds: 1,
        max_selections_per_round: 4,
        max_steps_per_round: 1,
        max_pointer_events_per_round: 15,
        max_key_events_per_round: 0,
        max_total_selections: 4,
        max_total_steps: 1,
        max_total_pointer_events: 15,
        max_total_key_events: 0,
        deadline_at_ms: retained.deadline_at_ms,
    };
    validate_visual_round_intent(&policy, &retained.snapshot, &retained.intent)
        .map_err(|_| "browser_native_image_round_invalid")?;
    // These are the reserved worst-case budget counts, not application events.
    // Uncertain delivery consumes the complete reservation and cannot restart.
    let receipt = VisualRoundEffectReceipt {
        round_id: retained.intent.round_id.clone(),
        evidence_digest: retained.intent.evidence_digest.clone(),
        intent_digest: retained.intent.intent_digest.clone(),
        receipt_ref: format!("native-image-original:{request}"),
        delivery: if acknowledged {
            DeliveryState::Acknowledged
        } else {
            DeliveryState::Uncertain
        },
        steps: retained.intent.planned_steps,
        pointer_events: retained.intent.planned_pointer_events,
        key_events: 0,
    };
    let result = decide_visual_round(
        &policy,
        &retained.snapshot,
        VisualRoundEvent::EffectFinished(receipt),
    )
    .map_err(|_| "browser_native_image_round_invalid")?;
    let state = if matches!(result, VisualRoundDecision::AwaitAfterState { .. }) {
        "verification_in_progress"
    } else {
        "human_intervention_required"
    };
    let encoded = serde_json::to_string(&RetainedRound {
        snapshot: result.snapshot().clone(),
        intent: retained.intent,
        deadline_at_ms: retained.deadline_at_ms,
    })
    .map_err(|_| "browser_native_image_round_invalid")?;
    Ok((state.into(), encoded))
}
