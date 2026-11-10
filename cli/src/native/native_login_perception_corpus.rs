use super::*;
use base64::Engine;
fn corpus_capture(rects: &[(u32, u32, u32, u32)], lines: Value) -> Value {
    let mut image = RgbaImage::from_pixel(800, 600, Rgba([255, 255, 255, 255]));
    for &(x, y, w, h) in rects {
        for xx in x..x + w {
            image.put_pixel(xx, y, Rgba([100, 100, 100, 255]));
            image.put_pixel(xx, y + h - 1, Rgba([100, 100, 100, 255]));
        }
        for yy in y..y + h {
            image.put_pixel(x, yy, Rgba([100, 100, 100, 255]));
            image.put_pixel(x + w - 1, yy, Rgba([100, 100, 100, 255]));
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = png.into_inner();
    json!({"desktopId":"desktop","generation":7,"windowId":11,"processIdentity":{"pid":123},"capture":{"sha256":hex(&png)},"image":{"data":base64::engine::general_purpose::STANDARD.encode(png)},"ocr":{"engine":"tesseract","engineVersion":"5.3.4","language":"eng","minimumConfidence":60.0,"modelSha256":"7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2","state":"observed","lines":lines}})
}
fn line(text: &str, x: u32, y: u32, w: u32) -> Value {
    json!({"text":text,"confidence":95,"bounds":{"x":x,"y":y,"width":w,"height":18}})
}
fn detect(capture: &Value, account: Option<&str>) -> Value {
    let mut request = json!({"profile":"english-chrome-v1"});
    if let Some(a) = account {
        request["expectedAccount"] = json!(a)
    }
    observe(
        capture,
        &json!({"x":0,"y":0,"width":800,"height":600}),
        &request,
    )
    .unwrap()
}
#[test]
fn native_login_perception_credential_entry_requires_same_frame_panel_and_does_not_export_account()
{
    let c = corpus_capture(
        &[(100, 100, 350, 200)],
        json!([
            line("Password Manager", 110, 115, 160),
            line("saved@example.invalid", 120, 170, 190)
        ]),
    );
    let r = detect(&c, Some("saved@example.invalid"));
    assert_eq!(r["status"], "observed");
    assert_eq!(r["controls"][0]["kind"], "credential_entry");
    assert_eq!(r["controls"][0]["configuredAccountMatch"], true);
    assert!(!r.to_string().contains("saved@example.invalid"));
}

#[test]
fn native_login_perception_corpus_keeps_ambiguity_and_ignores_values_without_pixel_corroboration() {
    let fields = corpus_capture(
        &[(100, 150, 300, 41), (100, 300, 300, 41)],
        json!([
            line("Email address", 100, 120, 110),
            line("Username", 100, 270, 110),
            line("private-value", 110, 160, 120)
        ]),
    );
    let r = detect(&fields, None);
    assert_eq!(r["status"], "ambiguous");
    assert_eq!(r["controls"].as_array().unwrap().len(), 2);
    assert!(!r.to_string().contains("private-value"));
    let absent = corpus_capture(
        &[],
        json!([
            line("Email address", 100, 120, 110),
            line("Password Manager", 100, 200, 160),
            line("saved@example.invalid", 100, 250, 180)
        ]),
    );
    assert_eq!(detect(&absent, None)["status"], "not_found");
}
#[test]
fn native_login_perception_corpus_enumerates_password_submit_and_shifted_scaled_fields() {
    let c = corpus_capture(
        &[
            (200, 180, 450, 61),
            (200, 300, 450, 61),
            (200, 400, 200, 50),
        ],
        json!([
            line("Email address", 200, 150, 150),
            line("Password", 200, 270, 120),
            line("Log in", 230, 415, 70),
            line("secret-password-value", 220, 315, 200)
        ]),
    );
    let r = detect(&c, None);
    let kinds = r["controls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(kinds, vec!["username", "password", "submit"]);
    assert!(!r.to_string().contains("secret-password-value"));
}
#[test]
fn native_login_perception_corpus_multiple_accounts_require_configured_unique_match() {
    let c = corpus_capture(
        &[(100, 100, 350, 250)],
        json!([
            line("Passwords", 110, 115, 100),
            line("first@example.invalid", 120, 170, 190),
            line("second@example.invalid", 120, 220, 190)
        ]),
    );
    assert_eq!(detect(&c, None)["status"], "ambiguous");
    let r = detect(&c, Some("second@example.invalid"));
    assert_eq!(r["status"], "observed");
    assert_eq!(
        r["controls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["configuredAccountMatch"] == true)
            .count(),
        1
    );
}
#[test]
fn native_login_perception_corpus_rejects_changed_frame_and_malformed_ocr() {
    let mut c = corpus_capture(
        &[(100, 150, 300, 41)],
        json!([line("Email", 100, 120, 100)]),
    );
    c["capture"]["sha256"] = json!("0".repeat(64));
    assert!(observe(
        &c,
        &json!({"x":0,"y":0,"width":800,"height":600}),
        &json!({"profile":"english-chrome-v1"})
    )
    .unwrap_err()
    .contains("frame_changed"));
    c["capture"]["sha256"] = json!(hex(&base64::engine::general_purpose::STANDARD
        .decode(c["image"]["data"].as_str().unwrap())
        .unwrap()));
    c["ocr"]["lines"][0]["bounds"]["x"] = json!(900);
    assert!(observe(
        &c,
        &json!({"x":0,"y":0,"width":800,"height":600}),
        &json!({"profile":"english-chrome-v1"})
    )
    .is_err());
}

#[test]
fn native_login_perception_target_selection_is_opaque_and_expires_before_input() {
    let c = corpus_capture(
        &[(100, 150, 300, 41)],
        json!([line("Email", 100, 120, 100)]),
    );
    let controls = detect(&c, None);
    let observation = json!({"binding":{"desktopId":"desktop","generation":7,"windowId":11,"window":{"x":0,"y":0,"width":800,"height":600},"displayWidth":800,"displayHeight":600,"region":{"x":0,"y":0,"width":800,"height":600},"nativeControlEpoch":4,"observedAtMs":1000},"controls":controls});
    let selected = select_target(
        &observation,
        observation["controls"]["controls"][0]["targetRef"]
            .as_str()
            .unwrap(),
        1001,
    )
    .unwrap();
    assert_eq!((selected.x, selected.y), (250, 170));
    assert!(select_target(&observation, "login:forged", 1001).is_err());
    assert!(select_target(
        &observation,
        observation["controls"]["controls"][0]["targetRef"]
            .as_str()
            .unwrap(),
        11001
    )
    .is_err());
}

#[test]
fn native_login_perception_filled_password_is_only_masked_when_pixels_show_repeated_dots() {
    let mut c = corpus_capture(
        &[
            (100, 150, 300, 41),
            (100, 250, 300, 41),
            (100, 350, 200, 41),
        ],
        json!([
            line("Email", 100, 120, 100),
            line("saved@example.invalid", 110, 160, 180),
            line("Password", 100, 220, 100),
            line("Log in", 120, 360, 70)
        ]),
    );
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(c["image"]["data"].as_str().unwrap())
        .unwrap();
    let mut image = image::load_from_memory(&bytes).unwrap().to_rgba8();
    for x in [115, 130, 145, 160] {
        for dx in 0..5 {
            for dy in 0..5 {
                image.put_pixel(x + dx, 265 + dy, Rgba([20, 20, 20, 255]));
            }
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = png.into_inner();
    c["capture"]["sha256"] = json!(hex(&png));
    c["image"]["data"] = json!(base64::engine::general_purpose::STANDARD.encode(png));
    let r = detect(&c, Some("saved@example.invalid"));
    let controls = r["controls"].as_array().unwrap();
    assert!(controls
        .iter()
        .any(|c| c["kind"] == "password" && c["fieldValueState"] == "masked"));
    assert!(controls
        .iter()
        .any(|c| c["kind"] == "username" && c["configuredAccountMatch"] == true));
}

#[test]
fn native_login_perception_plaintext_rings_are_not_masked_password_dots() {
    let mut c = corpus_capture(
        &[(100, 250, 300, 41)],
        json!([line("Password", 100, 220, 100)]),
    );
    let png = base64::engine::general_purpose::STANDARD
        .decode(c["image"]["data"].as_str().unwrap())
        .unwrap();
    let mut image = image::load_from_memory(&png).unwrap().to_rgba8();
    for x in [115, 130, 145, 160] {
        for dx in 0..5 {
            for dy in 0..5 {
                if dx == 0 || dx == 4 || dy == 0 || dy == 4 {
                    image.put_pixel(x + dx, 265 + dy, Rgba([20, 20, 20, 255]));
                }
            }
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = png.into_inner();
    c["capture"]["sha256"] = json!(hex(&png));
    c["image"]["data"] = json!(base64::engine::general_purpose::STANDARD.encode(png));
    let r = detect(&c, None);
    assert_eq!(r["controls"][0]["fieldValueState"], "text_present");
}

#[test]
fn native_login_perception_receiver_refuses_legacy_raw_capture_and_curates_owned_metadata() {
    let legacy = json!({"source":"remote_view_native","capture":{"image":{"data":"private-image-canary"},"ocr":{"lines":[{"text":"private-field-canary"}]}}});
    assert!(curate_response(&legacy).is_err());
    let c = corpus_capture(
        &[(100, 150, 300, 41)],
        json!([line("Email", 100, 120, 100)]),
    );
    let controls = detect(&c, None);
    let b = json!({"desktopId":"desktop","generation":7,"windowId":11,"window":{"x":0,"y":0,"width":800,"height":600},"displayWidth":800,"displayHeight":600,"region":{"x":0,"y":0,"width":800,"height":600},"nativeControlEpoch":4,"observedAtMs":1000});
    let source = json!({"source":"remote_view_native","binding":b,"loginControls":controls,"nativeLoginObservation":{"binding":b,"controls":controls},"unreviewed":"private-field-canary"});
    let clean = curate_response(&source).unwrap();
    assert!(!clean.to_string().contains("private-field-canary"));
    assert_eq!(clean["loginControls"]["controls"][0]["kind"], "username");
}

fn outlined_capture621(lines: Value) -> Value {
    let mut capture = corpus_capture(&[], lines);
    let mut image = RgbaImage::from_pixel(800, 600, Rgba([255, 255, 255, 255]));
    for inset in 0..5u32 {
        let shade = 70 + inset as u8 * 30;
        let (x, y, w, h) = (100 + inset, 150 + inset, 300 - 2 * inset, 49 - 2 * inset);
        for xx in x..x + w {
            image.put_pixel(xx, y, Rgba([shade, shade, shade, 255]));
            image.put_pixel(xx, y + h - 1, Rgba([shade, shade, shade, 255]));
        }
        for yy in y..y + h {
            image.put_pixel(x, yy, Rgba([shade, shade, shade, 255]));
            image.put_pixel(x + w - 1, yy, Rgba([shade, shade, shade, 255]));
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    capture["capture"]["sha256"] = json!(hex(png.get_ref()));
    capture["image"]["data"] =
        json!(base64::engine::general_purpose::STANDARD.encode(png.get_ref()));
    capture
}

#[test]
fn native_login_perception_nested_outlines621_one_contiguous_widget() {
    let capture = outlined_capture621(json!([line("Email", 100, 120, 100)]));
    let result = detect(&capture, None);
    assert_eq!(result["status"], "observed", "{result}");
    assert_eq!(result["controls"].as_array().unwrap().len(), 1);
    assert_eq!(result["controls"][0]["kind"], "username");
    assert_eq!(result["controls"][0]["fieldValueState"], "unverified");
}

#[test]
fn native_login_perception_nested_outlines621_preserves_distinct_controls() {
    for rects in [
        vec![
            (100, 150, 300, 49),
            (102, 152, 296, 45),
            (104, 154, 292, 41),
        ],
        vec![(100, 150, 300, 49), (104, 154, 300, 49)],
        vec![(100, 150, 300, 41), (100, 193, 300, 41)],
    ] {
        let capture = corpus_capture(
            &rects,
            json!([
                line("Email", 100, 120, 100),
                line("Username", 100, 165, 100)
            ]),
        );
        let result = detect(&capture, None);
        assert_eq!(result["status"], "ambiguous", "{rects:?}: {result}");
        assert!(result["controls"].as_array().unwrap().len() >= 2);
    }
    let capture = outlined_capture621(json!([
        line("Email", 100, 120, 100),
        line("Password", 100, 120, 100)
    ]));
    assert_eq!(detect(&capture, None)["status"], "ambiguous");
    let capture = outlined_capture621(json!([
        line("Email", 100, 120, 100),
        line("test@example.invalid", 101, 160, 1)
    ]));
    let result = detect(&capture, Some("test@example.invalid"));
    assert_eq!(result["status"], "ambiguous", "{result}");
    assert!(result["controls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["configuredAccountMatch"] == true));
    assert!(result["controls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["configuredAccountMatch"] == false));
}

#[test]
fn native_login_perception_nested_outlines621_never_promotes_conflicting_mask_evidence() {
    let mut capture = outlined_capture621(json!([line("Password", 100, 120, 100)]));
    let png = base64::engine::general_purpose::STANDARD
        .decode(capture["image"]["data"].as_str().unwrap())
        .unwrap();
    let mut image = image::load_from_memory(&png).unwrap().to_rgba8();
    for x in [140, 152, 164] {
        for dx in 0..4 {
            for dy in 0..4 {
                image.put_pixel(x + dx, 170 + dy, Rgba([0, 0, 0, 255]));
            }
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    capture["capture"]["sha256"] = json!(hex(png.get_ref()));
    capture["image"]["data"] =
        json!(base64::engine::general_purpose::STANDARD.encode(png.get_ref()));
    let result = detect(&capture, None);
    assert_eq!(result["status"], "observed", "{result}");
    assert_eq!(result["controls"].as_array().unwrap().len(), 1);
    assert_eq!(result["controls"][0]["fieldValueState"], "unverified");
}

fn rounded_layout621(fields: &[(u32, u32)], lines: Value) -> Value {
    let mut image = RgbaImage::from_pixel(800, 600, Rgba([255, 255, 255, 255]));
    image.put_pixel(0, 0, Rgba([211, 227, 253, 255]));
    let inside = |x: u32, y: u32, left: u32, top: u32, w: u32, h: u32, r: u32| {
        if x < left || y < top || x >= left + w || y >= top + h {
            return false;
        }
        let dx = (left + r)
            .saturating_sub(x)
            .max(x.saturating_sub(left + w - 1 - r));
        let dy = (top + r)
            .saturating_sub(y)
            .max(y.saturating_sub(top + h - 1 - r));
        dx * dx + dy * dy <= r * r
    };
    // Paint all interiors before all strokes so an overlap fixture retains both
    // distinct physical outlines instead of erasing the first widget's evidence.
    for stroke in [false, true] {
        for &(left, top) in fields {
            for y in top..top + 52 {
                for x in left..left + 360 {
                    let outer = inside(x, y, left, top, 360, 52, 9);
                    let inner = inside(x, y, left + 1, top + 1, 358, 50, 8);
                    if (!stroke && inner) || (stroke && outer && !inner) {
                        image.put_pixel(
                            x,
                            y,
                            if stroke {
                                Rgba([38, 38, 38, 255])
                            } else {
                                Rgba([232, 240, 254, 255])
                            },
                        );
                    }
                }
            }
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let mut capture = corpus_capture(&[], lines);
    capture["capture"]["sha256"] = json!(hex(png.get_ref()));
    capture["image"]["data"] =
        json!(base64::engine::general_purpose::STANDARD.encode(png.get_ref()));
    capture
}
fn rounded_capture621() -> Value {
    rounded_layout621(
        &[(100, 150), (100, 250)],
        json!([
            line("Email", 100, 120, 100),
            line("Password", 100, 220, 100)
        ]),
    )
}

#[test]
fn native_login_perception_nested_outlines621_rounded_filled_fields() {
    let result = detect(&rounded_capture621(), None);
    assert_eq!(result["status"], "observed", "{result}");
    assert_eq!(result["controls"].as_array().unwrap().len(), 2);
    assert_eq!(result["controls"][0]["kind"], "username");
    assert_eq!(result["controls"][1]["kind"], "password");
}

#[test]
fn native_login_perception_nested_outlines621_filled_distinct_and_shared_boundaries() {
    for (fields, labels) in [
        (
            vec![(100, 150), (104, 154)],
            json!([line("Email", 100, 120, 100)]),
        ),
        (
            vec![(100, 150), (100, 201)],
            json!([
                line("Email", 100, 120, 100),
                line("Username", 100, 171, 100)
            ]),
        ),
        (
            vec![(100, 150), (100, 204)],
            json!([
                line("Email", 100, 120, 100),
                line("Username", 100, 174, 100)
            ]),
        ),
    ] {
        let result = detect(&rounded_layout621(&fields, labels), None);
        assert_eq!(result["status"], "ambiguous", "{fields:?}: {result}");
        assert!(result["controls"].as_array().unwrap().len() >= 2);
    }
    // A touching same-fill bridge is unsupported, even though the nearby
    // rectangle edges and their shared fill colors otherwise look compatible.
    let mut capture = rounded_capture621();
    let png = base64::engine::general_purpose::STANDARD
        .decode(capture["image"]["data"].as_str().unwrap())
        .unwrap();
    let mut image = image::load_from_memory(&png).unwrap().to_rgba8();
    for y in 195..260 {
        for x in 248..252 {
            image.put_pixel(x, y, Rgba([232, 240, 254, 255]));
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    capture["capture"]["sha256"] = json!(hex(png.get_ref()));
    capture["image"]["data"] =
        json!(base64::engine::general_purpose::STANDARD.encode(png.get_ref()));
    assert_eq!(detect(&capture, None)["status"], "ambiguous");
}

#[test]
fn native_login_perception_nested_outlines621_center_ink_preserves_filled_form() {
    let mut capture = rounded_capture621();
    let png = base64::engine::general_purpose::STANDARD
        .decode(capture["image"]["data"].as_str().unwrap())
        .unwrap();
    let mut image = image::load_from_memory(&png).unwrap().to_rgba8();
    for y in 169..184 {
        for x in 279..282 {
            image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
    for x in 279..290 {
        for y in 181..184 {
            image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
    for x in [274, 280, 286] {
        for dx in 0..4 {
            for dy in 0..4 {
                image.put_pixel(x + dx, 274 + dy, Rgba([0, 0, 0, 255]));
            }
        }
    }
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    capture["capture"]["sha256"] = json!(hex(png.get_ref()));
    capture["image"]["data"] =
        json!(base64::engine::general_purpose::STANDARD.encode(png.get_ref()));
    capture["ocr"]["lines"].as_array_mut().unwrap().push(line(
        "test@example.invalid",
        150,
        168,
        150,
    ));
    let result = detect(&capture, Some("test@example.invalid"));
    assert_eq!(result["status"], "observed", "{result}");
    assert_eq!(result["controls"].as_array().unwrap().len(), 2);
    assert_eq!(result["controls"][0]["configuredAccountMatch"], true);
    assert_eq!(result["controls"][0]["fieldValueState"], "text_present");
    assert_eq!(result["controls"][1]["fieldValueState"], "masked");
}
