//! Task-scoped Native Remote View operations. Never opens a debugging connection.
use agent_browser_service_model::{RecordedProcessIdentity, RemoteViewApplicationTarget};
use serde_json::{json, Value};

/// Bounded, owner-minted evidence retained in the existing task-operation store.
/// It contains neither pixels, OCR text nor process executable paths.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeChallengeObservation {
    pub desktop_id: String,
    pub generation: u64,
    pub window_id: u64,
    pub window: NativeChallengeBounds,
    pub display_width: u32,
    pub display_height: u32,
    pub region: NativeChallengeBounds,
    pub frame_sha256: String,
    pub native_control_epoch: u64,
    pub observed_at_ms: u64,
    pub registered_action_id: String,
    pub family: String,
    pub candidate_sha256: String,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeChallengeBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl NativeChallengeObservation {
    /// Use the existing Native classification age bound. No historical desktop
    /// transaction clock or controller lease is imported into this authority.
    pub fn validate(&self, now_ms: u64) -> Result<(), String> {
        let digest =
            |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        let contains = |bounds: &NativeChallengeBounds| {
            i64::from(self.x) >= i64::from(bounds.x)
                && i64::from(self.y) >= i64::from(bounds.y)
                && i64::from(self.x) < i64::from(bounds.x) + i64::from(bounds.width)
                && i64::from(self.y) < i64::from(bounds.y) + i64::from(bounds.height)
        };
        if self.desktop_id.is_empty()
            || self.desktop_id.len() > 256
            || self.region.width == 0
            || self.region.height == 0
            || u64::from(self.region.width) * u64::from(self.region.height) > 1_000_000
            || !digest(&self.frame_sha256)
            || !digest(&self.candidate_sha256)
            || agent_browser_challenge_control::challenge_profile(&self.registered_action_id)
                .is_none()
            || !((self.family == "hcaptcha"
                && self.registered_action_id
                    == agent_browser_challenge_control::HCAPTCHA_PROFILE.profile_id)
                || (self.family == "turnstile"
                    && self.registered_action_id
                        == agent_browser_challenge_control::TURNSTILE_PROFILE.profile_id))
            || !contains(&self.region)
            || !contains(&self.window)
            || u32::from(self.x) >= self.display_width
            || u32::from(self.y) >= self.display_height
            || self.region.x < 0
            || self.region.y < 0
            || self.region.x < self.window.x
            || self.region.y < self.window.y
            || i64::from(self.region.x) + i64::from(self.region.width)
                > i64::from(self.window.x) + i64::from(self.window.width)
            || i64::from(self.region.y) + i64::from(self.region.height)
                > i64::from(self.window.y) + i64::from(self.window.height)
            || i64::from(self.region.x) + i64::from(self.region.width)
                > i64::from(self.display_width)
            || i64::from(self.region.y) + i64::from(self.region.height)
                > i64::from(self.display_height)
        {
            return Err("browser_native_original_observation_invalid".into());
        }
        if now_ms < self.observed_at_ms || now_ms - self.observed_at_ms > 10_000 {
            return Err(
                "browser_native_original_observation_expired:human assistance required".into(),
            );
        }
        Ok(())
    }
}

pub trait NativeOwner {
    fn request(&mut self, operation: &str, body: &Value) -> Result<Value, String>;
    /// Local adapter for the existing registered detector, not provider input.
    fn challenge_target(
        &mut self,
        _png: &[u8],
        _ocr: &Value,
        _action: &str,
        _region: &Value,
    ) -> Result<Value, String> {
        Err("browser_native_registered_locator_unavailable".into())
    }
    /// Reopen the pending original operation and its owner-minted capture. The
    /// default cannot turn caller parameters into an input permit.
    fn native_attempt(&mut self, _command: &Value) -> Result<NativeChallengeObservation, String> {
        Err("browser_native_original_operation_unavailable".into())
    }
    /// Separate owner-minted static image evidence, never a checkbox envelope.
    fn native_image_attempt(
        &mut self,
        _command: &Value,
    ) -> Result<super::browser_native_image::NativeImageObservation, String> {
        Err("browser_native_original_operation_unavailable".into())
    }
}
/// The caller retains browser ownership and establishes current mode. The native
/// owner independently fences the process, window, geometry and input epoch.
pub fn execute(
    original: &RecordedProcessIdentity,
    target: &RemoteViewApplicationTarget,
    command: &Value,
    owner: &mut impl NativeOwner,
) -> Result<Value, String> {
    if !matches!(
        command["action"].as_str(),
        Some("native_input" | "native_capture" | "native_close" | "native_focus" | "native_ready")
    ) {
        return Err("browser_native_action_unavailable".into());
    }
    let classify = match command.get("classifyChallenges") {
        None => false,
        Some(value) => value
            .as_bool()
            .ok_or("browser_native_classification_flag_invalid")?,
    };
    if classify && command["action"] != "native_capture" {
        return Err("browser_native_classification_requires_capture".into());
    }
    let image_scope = match command.get("imageCaptureScope") {
        None => super::browser_native_image::CaptureScope::CompleteWindow,
        Some(value)
            if value == "complete_puzzle_region"
                && command["action"] == "native_capture"
                && classify
                && command["registeredActionId"] == super::browser_native_image::PROFILE_ID
                && command.get("region").is_some() =>
        {
            super::browser_native_image::CaptureScope::CompletePuzzleRegion
        }
        Some(_) => return Err("browser_native_image_capture_scope_invalid".into()),
    };
    let observation_started = std::time::Instant::now();
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "browser_native_observation_clock_invalid")?
        .as_millis() as u64;
    let image_selected = if command.get("imageAttempt").is_some() {
        Some(super::browser_native_image::selection(command)?)
    } else {
        None
    };
    let attempt = if command.get("nativeAttempt").is_some() {
        if command["action"] != "native_input" {
            return Err("browser_native_attempt_requires_input".into());
        }
        let observation = owner.native_attempt(command)?;
        observation.validate(observed_at_ms)?;
        Some(observation)
    } else {
        None
    };
    let image_attempt = if image_selected.is_some() {
        let evidence = owner.native_image_attempt(command)?;
        evidence.validate(observed_at_ms)?;
        Some(evidence)
    } else {
        None
    };
    let registered_action = command
        .get("registeredActionId")
        .map(|value| {
            value
                .as_str()
                .filter(|id| {
                    agent_browser_challenge_control::challenge_profile(id).is_some()
                        || *id == super::browser_native_image::PROFILE_ID
                })
                .ok_or("browser_native_registered_action_invalid")
        })
        .transpose()?;
    if registered_action.is_some() && !classify {
        return Err("browser_native_registered_action_requires_classification".into());
    }
    let mut derived_actions = attempt.as_ref().map(|evidence| {
        vec![json!({"kind":"click","x":evidence.x,"y":evidence.y,"button":1,"count":1})]
    });
    if let (Some(evidence), Some(selected)) = (&image_attempt, &image_selected) {
        derived_actions = Some(evidence.actions(selected)?);
    }
    let actions = if let Some(actions) = derived_actions.as_ref() {
        Some(actions)
    } else if command["action"] == "native_input" {
        Some(
            command["actions"]
                .as_array()
                .filter(|a| !a.is_empty() && a.len() <= 16)
                .ok_or("browser_native_actions_required")?,
        )
    } else {
        None
    };
    let snapshot = owner.request("observe", &json!({"desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation}))?;
    if snapshot["desktopId"] != target.viewing_desktop_id
        || snapshot["generation"] != target.viewing_generation
    {
        return Err("browser_native_generation_changed:refresh inventory".into());
    }
    let windows = snapshot
        .pointer("/windows/windows")
        .and_then(Value::as_array)
        .ok_or("browser_native_window_unverified:refresh inventory")?;
    let candidates: Vec<&Value> = windows
        .iter()
        .filter(|window| {
            let process = &window["processIdentity"];
            window["pid"].as_u64() == Some(u64::from(original.pid))
                && process["pid"].as_u64() == Some(u64::from(original.pid))
                && original
                    .executable_path
                    .as_deref()
                    .is_some_and(|path| process["executable"] == path)
                && process["boot_id"]
                    .as_str()
                    .zip(process["start_token"].as_str())
                    .is_some_and(|(boot, start)| {
                        original.start_token == format!("linux:{boot}:{start}")
                    })
                && window["width"].as_u64().is_some_and(|n| n > 0)
                && window["height"].as_u64().is_some_and(|n| n > 0)
        })
        .collect();
    let active: Vec<&Value> = candidates
        .iter()
        .copied()
        .filter(|window| window["active"] == true)
        .collect();
    let window = match (active.as_slice(), candidates.as_slice()) {
        ([window], _) | ([], [window]) => *window,
        _ => {
            return Err(
                "browser_native_window_unverified:focus browser and refresh inventory".into(),
            )
        }
    };
    if command["action"] == "native_ready" {
        return Ok(
            json!({"source":"remote_view_native","ready":true,"windowId":window["id"],"window":{"x":window["x"],"y":window["y"],"width":window["width"],"height":window["height"],"active":window["active"]},"nativeContextScope":"browser","taskCompleted":false}),
        );
    }
    if command["action"] == "native_focus" {
        let action = "activate";
        let reply = owner.request("window", &json!({
            "desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation,
            "windowId":window["id"],"expectedProcessIdentity":window["processIdentity"],"action":action
        }))?;
        return Ok(
            json!({"source":"remote_view_native","delivery":reply["delivery"].as_str().unwrap_or("unknown"),"taskCompleted":false}),
        );
    }
    if command["action"] == "native_close" && window["active"] != true {
        owner.request(
            "window",
            &json!({"desktopId":target.viewing_desktop_id,
            "expectedGeneration":target.viewing_generation,"windowId":window["id"],
            "expectedProcessIdentity":window["processIdentity"],"action":"activate"}),
        )?;
        // Observe and fence again after focus. This recursion is bounded: the
        // input action cannot take the close/focus branch a second time.
        return execute(
            original,
            target,
            &json!({"action":"native_input",
            "actions":[{"kind":"key","chord":"alt+F4"}]}),
            owner,
        );
    }
    let width = snapshot
        .pointer("/geometry/current/width")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or("browser_native_geometry_unverified")?;
    let height = snapshot
        .pointer("/geometry/current/height")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or("browser_native_geometry_unverified")?;
    let mut body = json!({"desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation,
        "windowId":window["id"],"expectedProcessIdentity":window["processIdentity"],"expectedWidth":width,"expectedHeight":height,
        "expectedWindow":{"x":window["x"],"y":window["y"],"width":window["width"],"height":window["height"]},"actions":actions});
    if let Some(evidence) = &attempt {
        if evidence.desktop_id != target.viewing_desktop_id
            || evidence.generation != target.viewing_generation
            || window["id"] != evidence.window_id
            || body["expectedWindow"] != serde_json::to_value(&evidence.window).unwrap()
            || width != u64::from(evidence.display_width)
            || height != u64::from(evidence.display_height)
        {
            return Err("browser_native_original_observation_context_changed".into());
        }
        let control = owner.request("control", &json!({"desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation,"operation":"status"}))?;
        if control["desktopId"] != target.viewing_desktop_id
            || control["generation"] != target.viewing_generation
            || control["owner"] != "agent"
            || control["epoch"] != evidence.native_control_epoch
        {
            return Err("browser_native_original_observation_control_changed".into());
        }
        body["controlEpoch"] = json!(evidence.native_control_epoch);
        body["expectedVisual"] = json!({"region":evidence.region,"sha256":evidence.frame_sha256});
    }
    if let Some(evidence) = &image_attempt {
        if evidence.desktop_id != target.viewing_desktop_id
            || evidence.generation != target.viewing_generation
            || window["id"] != evidence.window_id
            || body["expectedWindow"] != serde_json::to_value(&evidence.window).unwrap()
            || width != u64::from(evidence.display_width)
            || height != u64::from(evidence.display_height)
            || super::browser_native_image::digest(&window["processIdentity"])?
                != evidence.process_digest
        {
            return Err("browser_native_original_observation_context_changed".into());
        }
        let control=owner.request("control",&json!({"desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation,"operation":"status"}))?;
        if control["desktopId"] != target.viewing_desktop_id
            || control["generation"] != target.viewing_generation
            || control["owner"] != "agent"
            || control["epoch"] != evidence.native_control_epoch
        {
            return Err("browser_native_original_observation_control_changed".into());
        }
        body["controlEpoch"] = json!(evidence.native_control_epoch);
        body["expectedVisual"] = json!({"region":evidence.region,"sha256":evidence.frame_sha256});
    }
    if command["action"] == "native_close" {
        // Remote View window.close destroys the window. Normal WM close is
        // requested through native input; exit remains independent evidence.
        body["actions"] = json!([{"kind":"key","chord":"alt+F4"}]);
    }
    if command["action"] == "native_capture" {
        body.as_object_mut().unwrap().remove("actions");
        if classify {
            // Read the existing Native owner's epoch. The cooperative browser
            // epoch is a different authority and cannot authorize this capture.
            let control = owner.request(
                "control",
                &json!({"desktopId":target.viewing_desktop_id,
                "expectedGeneration":target.viewing_generation,"operation":"status"}),
            )?;
            if control["desktopId"] != target.viewing_desktop_id
                || control["generation"] != target.viewing_generation
                || control["owner"] != "agent"
                || control["epoch"].as_u64().is_none()
            {
                return Err("browser_native_classification_control_changed:observe control".into());
            }
            body["controlEpoch"] = control["epoch"].clone();
        } else if let Some(epoch) = command.get("controlEpoch") {
            body["controlEpoch"] = epoch.clone();
        }
        let full_width = window["width"]
            .as_u64()
            .ok_or("browser_native_geometry_unverified")?;
        let full_height = window["height"]
            .as_u64()
            .ok_or("browser_native_geometry_unverified")?;
        // Native capture_region admits at most one million pixels. Return an
        // explicitly bounded tile, with geometry for requesting the remainder.
        let tile_width = full_width.min(1_000_000);
        let tile_height = full_height.min(1_000_000 / tile_width);
        body["region"] = command.get("region").cloned().unwrap_or_else(|| {
            json!({
                "x":window["x"],"y":window["y"],"width":tile_width,"height":tile_height
            })
        });
        let mut capture = owner.request(if classify { "ocr" } else { "capture_region" }, &body)?;
        let mut classification = None;
        let mut recognition = None;
        let mut native_observation = None;
        let mut native_image_observation = None;
        let mut registered_target = None;
        let mut corroborated_target = None;
        if classify {
            use base64::Engine;
            use sha2::{Digest, Sha256};
            if capture["desktopId"] != target.viewing_desktop_id
                || capture["generation"] != target.viewing_generation
                || capture["windowId"] != window["id"]
                || capture["processIdentity"] != window["processIdentity"]
            {
                return Err("browser_native_classification_context_changed:refresh capture".into());
            }
            let encoded = capture
                .pointer("/image/data")
                .and_then(Value::as_str)
                .filter(|data| data.len() <= 5_592_408)
                .ok_or("browser_native_classification_image_invalid")?;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| "browser_native_classification_image_invalid")?;
            let digest = format!("{:x}", Sha256::digest(&bytes));
            if bytes.len() > 4_194_304
                || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || capture.pointer("/capture/sha256").and_then(Value::as_str)
                    != Some(digest.as_str())
            {
                return Err("browser_native_classification_frame_changed:refresh capture".into());
            }
            let lines = capture
                .pointer("/ocr/lines")
                .and_then(Value::as_array)
                .ok_or("browser_native_classification_ocr_unavailable")?;
            let words = lines
                .iter()
                .map(|line| {
                    Ok(agent_browser_challenge_control::NativeChallengeText {
                        text: line["text"]
                            .as_str()
                            .ok_or("browser_native_classification_ocr_invalid")?,
                        confidence: line["confidence"]
                            .as_f64()
                            .ok_or("browser_native_classification_ocr_invalid")?,
                    })
                })
                .collect::<Result<Vec<_>, &str>>()?;
            classification = Some(agent_browser_challenge_control::classify_native_challenge(
                &words,
                &digest,
                observation_started.elapsed().as_millis() <= 10_000,
                body["region"]
                    == json!({"x":window["x"],"y":window["y"],"width":full_width,"height":full_height}),
            ));
            // Existing hCaptcha TSV normalization already recognizes these
            // joined glyphs. Text alone remains unknown: only this same owner
            // frame's unique registered checkbox can corroborate the join.
            if capture["ocr"]["state"] == "observed"
                && classification.as_ref().is_some_and(|value| {
                    value.kind == agent_browser_challenge_control::NativeChallengeKind::Unknown
                        && value.complete_window
                        && value.rule_ids.is_empty()
                })
                && words.iter().any(|line| {
                    line.confidence >= 70.0 && line.text.trim().eq_ignore_ascii_case("hcaptcha")
                })
                && words.iter().any(|line| {
                    line.confidence >= 70.0
                        && matches!(
                            line.text
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .to_ascii_lowercase()
                                .as_str(),
                            "iam human" | "lam human" | "tam human"
                        )
                })
            {
                let mut bounded_ocr = capture["ocr"].clone();
                if let Some(words) = bounded_ocr["words"].as_array_mut() {
                    // This lift requires every contributing word at the current
                    // Native confidence floor, including the brand. Do not
                    // inherit the older detector's permissive brand threshold.
                    words.retain(|word| {
                        let text = word["text"].as_str().unwrap_or("");
                        let normalized = text
                            .chars()
                            .filter(char::is_ascii_alphabetic)
                            .collect::<String>()
                            .to_ascii_lowercase();
                        // Only the exact strong brand may participate in the
                        // matched candidate's existing spatial contract. An
                        // unrelated exact brand elsewhere cannot sponsor a
                        // nearby legacy captcha/hhcaptcha alias.
                        let exact_brand = !["hcaptcha", "hhcaptcha", "captcha"]
                            .contains(&normalized.as_str())
                            || text.trim().eq_ignore_ascii_case("hcaptcha");
                        exact_brand
                            && word["confidence"].as_f64().is_some_and(|score| {
                                score.is_finite() && (70.0..=100.0).contains(&score)
                            })
                    });
                }
                corroborated_target = owner
                    .challenge_target(
                        &bytes,
                        &bounded_ocr,
                        agent_browser_challenge_control::HCAPTCHA_PROFILE.profile_id,
                        &body["region"],
                    )
                    .ok()
                    .filter(|located| located["status"] == "matched" && located["visible"] == true);
                if corroborated_target.is_some()
                    && observation_started.elapsed().as_millis() <= 10_000
                {
                    let value = classification.as_mut().unwrap();
                    value.corpus_version = "native-challenge-corpus.v2";
                    value.limitation = "Bounded English OCR plus same-frame registered hCaptcha pixels; classification is not attempt authority, clearance or site admission; OCR scores are not probabilities";
                    value.kind = agent_browser_challenge_control::NativeChallengeKind::Checkbox;
                    value.family = Some("hcaptcha");
                    value.registered_profile_id =
                        Some(agent_browser_challenge_control::HCAPTCHA_PROFILE.profile_id);
                    value
                        .rule_ids
                        .push("registered_hcaptcha_joined_prompt_visual_v1");
                }
            }
            recognition = Some(json!({"engine":capture["ocr"]["engine"],
                "modelSha256":capture["ocr"]["modelSha256"],"lineCount":lines.len(),
                "scoreMeaning":"OCR engine score, not a probability"}));
            if registered_action == Some(super::browser_native_image::PROFILE_ID) {
                if classification.as_ref().is_some_and(|value| {
                    value.kind == agent_browser_challenge_control::NativeChallengeKind::ImagePuzzle
                        && value.family == Some("hcaptcha")
                        && (value.complete_window
                            || image_scope
                                == super::browser_native_image::CaptureScope::CompletePuzzleRegion)
                }) {
                    native_image_observation = super::browser_native_image::observe(&bytes, &capture["ocr"], serde_json::from_value(json!({
                        "desktopId":target.viewing_desktop_id,"generation":target.viewing_generation,"windowId":window["id"],
                        "processDigest":super::browser_native_image::digest(&window["processIdentity"] )?,
                        "window":body["expectedWindow"],"displayWidth":width,"displayHeight":height,"region":body["region"],
                        "frameSha256":digest,"nativeControlEpoch":body["controlEpoch"],"observedAtMs":observed_at_ms,
                        "captureScope":image_scope,"registeredActionId":super::browser_native_image::PROFILE_ID,"puzzleSha256":"", "candidates":[],"verify":{"x":0,"y":0,"width":0,"height":0}
                    })).map_err(|_| "browser_native_image_observation_invalid")?)?;
                }
            } else if let Some(action) = registered_action {
                let located = match corroborated_target.as_ref().filter(|_| {
                    action == agent_browser_challenge_control::HCAPTCHA_PROFILE.profile_id
                }) {
                    Some(located) => located.clone(),
                    None => {
                        owner.challenge_target(&bytes, &capture["ocr"], action, &body["region"])?
                    }
                };
                if classification.as_ref().is_some_and(|value| {
                    value.kind == agent_browser_challenge_control::NativeChallengeKind::Checkbox
                        && value.complete_window
                        && value.registered_profile_id == Some(action)
                }) && located["status"] == "matched"
                    && located["visible"] == true
                {
                    let evidence: NativeChallengeObservation = serde_json::from_value(json!({
                        "desktopId":target.viewing_desktop_id,"generation":target.viewing_generation,"windowId":window["id"],
                        "window":body["expectedWindow"],"displayWidth":width,"displayHeight":height,"region":body["region"],
                        "frameSha256":digest,"nativeControlEpoch":body["controlEpoch"],"observedAtMs":observed_at_ms,
                        "registeredActionId":action,"family":classification.as_ref().and_then(|value| value.family),"candidateSha256":located["candidateSha256"],"x":located["x"],"y":located["y"]
                    })).map_err(|_| "browser_native_registered_target_invalid")?;
                    evidence.validate(
                        observed_at_ms + observation_started.elapsed().as_millis() as u64,
                    )?;
                    native_observation = Some(evidence);
                }
                registered_target = Some(located);
            }
            // Text stays process-local. The returned image and redacted rule IDs
            // describe this one Native-fenced frame, never task/site completion.
            capture
                .as_object_mut()
                .ok_or("browser_native_classification_ocr_invalid")?
                .remove("ocr");
        }
        let mut result = json!({"source":"remote_view_native","capture":capture,"region":body["region"],
            "windowBounds":{"x":window["x"],"y":window["y"],"width":full_width,"height":full_height},
            "partialWindow":body["region"]!=json!({"x":window["x"],"y":window["y"],"width":full_width,"height":full_height}),
            "taskCompleted":false});
        if let Some(classification) = classification {
            result["challenge"] = serde_json::to_value(classification)
                .map_err(|_| "browser_native_classification_invalid")?;
            result["recognition"] = recognition.unwrap_or(Value::Null);
            result["observedAtMs"] = json!(observed_at_ms);
            result["evidenceAgeMs"] = json!(observation_started.elapsed().as_millis() as u64);
            result["nativeControlEpoch"] = body["controlEpoch"].clone();
        }
        if let Some(located) = registered_target {
            result["registeredTarget"] = located;
        }
        if let Some(evidence) = native_image_observation {
            result["nativeImageObservation"] = serde_json::to_value(evidence)
                .map_err(|_| "browser_native_image_observation_invalid")?;
        }
        if let Some(evidence) = native_observation {
            result["nativeObservation"] = serde_json::to_value(evidence)
                .map_err(|_| "browser_native_original_observation_invalid")?;
        }
        return Ok(result);
    }
    if attempt.is_none() {
        if let Some(epoch) = command.get("controlEpoch") {
            body["controlEpoch"] = epoch.clone();
        }
    }
    if let Some(evidence) = &image_attempt {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "browser_native_observation_clock_invalid")?
            .as_millis() as u64;
        evidence.validate(now)?;
    }
    if let Some(evidence) = &attempt {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "browser_native_observation_clock_invalid")?
            .as_millis() as u64;
        evidence.validate(now)?;
    }
    let reply = owner.request("input", &body)?;
    Ok(
        json!({"source":"remote_view_native","delivery":reply["delivery"].as_str().unwrap_or("unknown"),"taskCompleted":false}),
    )
}
