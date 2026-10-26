//! Task-scoped Native Remote View operations. Never opens a debugging connection.
use agent_browser_service_model::{RecordedProcessIdentity, RemoteViewApplicationTarget};
use serde_json::{json, Value};

pub trait NativeOwner {
    fn request(&mut self, operation: &str, body: &Value) -> Result<Value, String>;
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
    let observation_started = std::time::Instant::now();
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "browser_native_observation_clock_invalid")?
        .as_millis() as u64;
    let actions = if command["action"] == "native_input" {
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
    if command["action"] == "native_close" {
        // Remote View window.close destroys the window. Normal WM close is
        // requested through native input; exit remains independent evidence.
        body["actions"] = json!([{"kind":"key","chord":"alt+F4"}]);
    }
    if command["action"] == "native_capture" {
        body.as_object_mut().unwrap().remove("actions");
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
            recognition = Some(json!({"engine":capture["ocr"]["engine"],
                "modelSha256":capture["ocr"]["modelSha256"],"lineCount":lines.len(),
                "scoreMeaning":"OCR engine score, not a probability"}));
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
        }
        return Ok(result);
    }
    if let Some(epoch) = command.get("controlEpoch") {
        body["controlEpoch"] = epoch.clone();
    }
    let reply = owner.request("input", &body)?;
    Ok(
        json!({"source":"remote_view_native","delivery":reply["delivery"].as_str().unwrap_or("unknown"),"taskCompleted":false}),
    )
}
