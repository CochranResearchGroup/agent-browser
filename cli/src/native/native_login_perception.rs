//! Read-only deterministic login perception over the existing Native owner's
//! bounded PNG/OCR pair. No credential extraction, model, input or site-auth claim.
use image::{Rgba, RgbaImage};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const PROFILE: &str = "english-chrome-v1";
const INVALID: &str = "browser_native_login_evidence_invalid";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct Bounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
impl Bounds {
    fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && u64::from(other.x) + u64::from(other.width)
                <= u64::from(self.x) + u64::from(self.width)
            && u64::from(other.y) + u64::from(other.height)
                <= u64::from(self.y) + u64::from(self.height)
    }
}
fn bounds(v: &Value) -> Result<Bounds, String> {
    let n = |key: &str| {
        v[key]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(INVALID)
    };
    let b = Bounds {
        x: n("x")?,
        y: n("y")?,
        width: n("width")?,
        height: n("height")?,
    };
    if b.width == 0
        || b.height == 0
        || b.x.checked_add(b.width).is_none()
        || b.y.checked_add(b.height).is_none()
    {
        return Err(INVALID.into());
    }
    Ok(b)
}
/// Reject unsupported tuning before an owner observation. Expected-account text
/// is matching input only and is never included in a result or error.
pub(crate) fn validate_request(v: &Value) -> Result<(), String> {
    let o = v
        .as_object()
        .ok_or("browser_native_login_request_invalid")?;
    if v["profile"] != PROFILE
        || o.keys()
            .any(|k| !matches!(k.as_str(), "profile" | "expectedAccount"))
    {
        return Err("browser_native_login_profile_unsupported".into());
    }
    if let Some(a) = v.get("expectedAccount") {
        let a = a
            .as_str()
            .filter(|a| {
                a.len() <= 254
                    && a.is_ascii()
                    && !a.chars().any(char::is_whitespace)
                    && a.split('@').count() == 2
                    && !a.starts_with('@')
                    && !a.ends_with('@')
            })
            .ok_or("browser_native_login_account_invalid")?;
        if a.chars().any(char::is_control) {
            return Err("browser_native_login_account_invalid".into());
        }
    }
    Ok(())
}
fn close(a: Rgba<u8>, b: Rgba<u8>) -> bool {
    (0..3).all(|i| a[i].abs_diff(b[i]) <= 12)
}
/// Flat border/fill contours, corroborated by OCR below. One bounded image scan,
/// finite rectangle sizes and a hard candidate ceiling make execution repeatable.
fn rectangles(image: &RgbaImage) -> Result<Vec<Bounds>, String> {
    let (w, h) = image.dimensions();
    let background = *image.get_pixel(0, 0);
    let mut found = vec![];
    for y in 1..h.saturating_sub(23) {
        let mut x = 1;
        while x + 71 < w {
            let color = *image.get_pixel(x, y);
            if close(color, background)
                || close(color, *image.get_pixel(x - 1, y))
                || close(color, *image.get_pixel(x, y - 1))
            {
                x += 1;
                continue;
            }
            let mut end = x + 1;
            while end < w && close(color, *image.get_pixel(end, y)) {
                end += 1
            }
            if end - x < 72 {
                x = end;
                continue;
            }
            let right = end - 1;
            for bottom in y + 23..(y + 600).min(h.saturating_sub(1)) {
                if close(color, *image.get_pixel(x, bottom + 1))
                    || close(color, *image.get_pixel(right, bottom + 1))
                {
                    continue;
                }
                let horizontal = (x..end)
                    .filter(|xx| close(color, *image.get_pixel(*xx, bottom)))
                    .count();
                if horizontal * 100 < ((end - x) as usize) * 90 {
                    continue;
                }
                let vertical = (y..=bottom)
                    .filter(|yy| {
                        close(color, *image.get_pixel(x, *yy))
                            && close(color, *image.get_pixel(right, *yy))
                    })
                    .count();
                if vertical * 100 >= ((bottom - y + 1) as usize) * 90 {
                    found.push(Bounds {
                        x,
                        y,
                        width: end - x,
                        height: bottom - y + 1,
                    });
                    break;
                }
            }
            if found.len() > 64 {
                return Err("browser_native_login_layout_unsupported".into());
            }
            x = end;
        }
    }
    Ok(found)
}
fn compact_footer(text: &str) -> bool {
    let text = text.trim().to_ascii_lowercase();
    let text = text.strip_prefix("© ").unwrap_or(&text);
    matches!(
        text,
        "manage passwords." | "manage passwords..." | "manage passwords…"
    )
}
fn compact_panel(
    image: &RgbaImage,
    rects: &[Bounds],
    crop: Bounds,
    footer: &Value,
) -> Option<Bounds> {
    if footer["confidence"].as_f64()? < 80.0 || !compact_footer(footer["text"].as_str()?) {
        return None;
    }
    let b = bounds(&footer["bounds"]).ok()?;
    if !crop.contains(b) {
        return None;
    }
    let local = Bounds {
        x: b.x - crop.x,
        y: b.y - crop.y,
        ..b
    };
    let whole = rects
        .iter()
        .copied()
        .filter(|r| {
            r.width >= 150
                && r.width <= 480
                && r.height >= 80
                && r.height <= 180
                && r.contains(local)
                && local.y >= r.y + r.height / 2
                && local.y + local.height + 3 < r.y + r.height
                && image.get_pixel(r.x + r.width / 2, r.y + r.height / 2)[3] == 255
        })
        .min_by_key(|r| r.width * r.height);
    if whole.is_some() {
        return whole;
    }
    // Chrome's compact popup has separate flat entry/footer interiors divided
    // by a horizontal separator. Prove their aligned, adjacent contours rather
    // than requiring one uninterrupted outer fill rectangle.
    let mut paired = vec![];
    for foot in rects.iter().filter(|r| {
        r.width >= 150 && r.width <= 480 && r.height >= 25 && r.height <= 65 && r.contains(local)
    }) {
        for entry in rects
            .iter()
            .filter(|r| r.width >= 150 && r.width <= 480 && r.height >= 40 && r.height <= 100)
        {
            if entry.y + entry.height > foot.y
                || foot.y - (entry.y + entry.height) > 3
                || (entry.x + entry.width / 2).abs_diff(foot.x + foot.width / 2) > 2
                || entry.x.abs_diff(foot.x) > 8
                || (entry.x + entry.width).abs_diff(foot.x + foot.width) > 8
            {
                continue;
            }
            let x = entry.x.min(foot.x);
            let right = (entry.x + entry.width).max(foot.x + foot.width);
            paired.push(Bounds {
                x,
                y: entry.y,
                width: right - x,
                height: foot.y + foot.height - entry.y,
            });
        }
    }
    paired
        .into_iter()
        .min_by_key(|r| (r.y, std::cmp::Reverse(r.width * r.height)))
}
fn compact_masked(image: &RgbaImage, band: Bounds) -> bool {
    if value_state(image, band) == "masked" {
        return true;
    }
    let background = image.get_pixel(band.x + 3, band.y + 3);
    if (0..3).any(|i| background[i] < 230) {
        return false;
    }
    // Small antialiased Chrome bullets may share faint pixels. Their dark
    // cores must still form at least six uniformly spaced dense round dots.
    // This compact-only template does not alter general password masking.
    let mut core = RgbaImage::from_pixel(band.width, band.height, Rgba([255, 255, 255, 255]));
    for y in 0..band.height {
        for x in 0..band.width {
            let p = image.get_pixel(band.x + x, band.y + y);
            if (0..3).all(|i| p[i] < 200) {
                core.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
    }
    let mut runs = vec![];
    let mut start = None;
    for x in 6..band.width - 40 {
        let ink = (5..band.height - 5).any(|y| core.get_pixel(x, y)[0] == 0);
        if ink && start.is_none() {
            start = Some(x);
        }
        if !ink {
            if let Some(a) = start.take() {
                runs.push((a, x - a));
            }
        }
    }
    if start.is_some() || runs.len() < 6 || runs.iter().any(|(_, w)| *w == 0 || *w > 6) {
        return false;
    }
    let pitch = runs[1].0 - runs[0].0;
    if !(3..=10).contains(&pitch)
        || runs
            .windows(2)
            .any(|w| (w[1].0 - w[0].0).abs_diff(pitch) > 1)
        || runs.iter().any(|(_, w)| w.abs_diff(runs[0].1) > 1)
    {
        return false;
    }
    value_state(
        &core,
        Bounds {
            x: 0,
            y: 0,
            width: band.width,
            height: band.height,
        },
    ) == "masked"
}
fn compact_row_count(lines: &[Value], crop: Bounds, panel: Bounds) -> usize {
    let mut rows = Vec::<u32>::new();
    for line in lines
        .iter()
        .filter(|l| l["text"].as_str().is_some_and(|t| t.contains('@')))
    {
        let Ok(b) = bounds(&line["bounds"]) else {
            return usize::MAX;
        };
        if !crop.contains(b) {
            continue;
        }
        let local = Bounds {
            x: b.x - crop.x,
            y: b.y - crop.y,
            ..b
        };
        if panel.contains(local) {
            let center = local.y + local.height / 2;
            if !rows.iter().any(|y| y.abs_diff(center) <= 3) {
                rows.push(center);
            }
        }
    }
    rows.len()
}
fn compact_account(
    image: &RgbaImage,
    panel: Bounds,
    crop: Bounds,
    label: Bounds,
    footer: &Value,
) -> bool {
    let Ok(foot) = bounds(&footer["bounds"]) else {
        return false;
    };
    let foot = Bounds {
        x: foot.x - crop.x,
        y: foot.y - crop.y,
        ..foot
    };
    if !panel.contains(label) || label.y + label.height + 20 >= foot.y || label.x < panel.x + 12 {
        return false;
    }
    let top = label.y + label.height + 3;
    let left = label.x.saturating_sub(6);
    let right = panel.x + panel.width - 8;
    if right <= left + 70 || top + 22 >= foot.y {
        return false;
    }
    compact_masked(
        image,
        Bounds {
            x: left,
            y: top,
            width: right - left,
            height: 26.min(foot.y - top - 3),
        },
    )
}

/// Identify the bounded connected flat interior of one rounded field. Border
/// antialias pixels need not share one stroke color, but the interior must not
/// escape into a panel or a touching/shared widget. Interior glyph holes are not
/// interpreted here and cannot establish a masked-password assertion.
fn filled_field_component(image: &RgbaImage, b: Bounds, fill: Rgba<u8>) -> Option<(u32, u32)> {
    if fill[3] != 255 {
        return None;
    }
    // The shared boundary bands prove this top-middle sample is flat fill.
    // Center pixels may instead be username glyphs or password bullets.
    let (cx, cy) = (b.x + b.width / 2, b.y);
    let seed = *image.get_pixel(cx, cy);
    if seed[3] != 255 || !close(seed, fill) {
        return None;
    }
    let left = b.x.saturating_sub(9);
    let top = b.y.saturating_sub(9);
    let right = (b.x + b.width + 8).min(image.width() - 1);
    let bottom = (b.y + b.height + 8).min(image.height() - 1);
    let width = right - left + 1;
    let index = |x: u32, y: u32| ((y - top) * width + x - left) as usize;
    let mut marked = vec![false; (width * (bottom - top + 1)) as usize];
    let mut pending = vec![(cx, cy)];
    marked[index(cx, cy)] = true;
    let mut minimum = (cy, cx);
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (cx, cy, cx, cy);
    let mut count = 0u32;
    while let Some((x, y)) = pending.pop() {
        // Reaching the local envelope cannot prove an independently bounded field.
        if x == left || x == right || y == top || y == bottom {
            return None;
        }
        count += 1;
        minimum = minimum.min((y, x));
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        for (nx, ny) in [
            (x.saturating_sub(1), y),
            (x + 1, y),
            (x, y.saturating_sub(1)),
            (x, y + 1),
        ] {
            if nx >= left && nx <= right && ny >= top && ny <= bottom && !marked[index(nx, ny)] {
                let p = *image.get_pixel(nx, ny);
                if p[3] == 255 && close(p, fill) {
                    marked[index(nx, ny)] = true;
                    pending.push((nx, ny));
                }
            }
        }
    }
    let component_width = max_x - min_x + 1;
    let component_height = max_y - min_y + 1;
    if component_width < b.width
        || component_height < b.height
        || count * 100 < component_width * component_height * 80
    {
        return None;
    }
    Some(minimum)
}

/// Shared bands must prove continuous border ink or the same bounded connected
/// field interior. Overlap alone, disconnected outlines and transitive chains
/// are insufficient same-widget proof.
fn same_field_outline(image: &RgbaImage, a: Bounds, b: Bounds) -> bool {
    let (ar, ab, br, bb) = (
        a.x + a.width - 1,
        a.y + a.height - 1,
        b.x + b.width - 1,
        b.y + b.height - 1,
    );
    if [
        a.x.abs_diff(b.x),
        a.y.abs_diff(b.y),
        ar.abs_diff(br),
        ab.abs_diff(bb),
    ]
    .into_iter()
    .any(|d| d > 8)
    {
        return false;
    }
    let (left, top, right, bottom) = (a.x.max(b.x), a.y.max(b.y), ar.min(br), ab.min(bb));
    if right <= left
        || bottom <= top
        || (right - left + 1) * 100 < a.width.max(b.width) * 90
        || (bottom - top + 1) * 100 < a.height.max(b.height) * 60
    {
        return false;
    }
    let background = *image.get_pixel(0, 0);
    let center = *image.get_pixel((left + right) / 2, (top + bottom) / 2);
    let fill = *image.get_pixel((left + right) / 2, top);
    let flat = |x, y| {
        let p = *image.get_pixel(x, y);
        p[3] == 255 && close(p, fill)
    };
    let xs = left + (right - left) / 4..=right - (right - left) / 4;
    let ys = top + (bottom - top) / 4..=bottom - (bottom - top) / 4;
    let filled_bands = xs.clone().all(|x| {
        (a.y.min(b.y)..=top).all(|y| flat(x, y)) && (bottom..=ab.max(bb)).all(|y| flat(x, y))
    }) && ys.clone().all(|y| {
        (a.x.min(b.x)..=left).all(|x| flat(x, y)) && (right..=ar.max(br)).all(|x| flat(x, y))
    });
    if filled_bands {
        if let Some(border) = filled_field_component(image, a, fill) {
            if filled_field_component(image, b, fill) == Some(border) {
                return true;
            }
        }
    }
    if !close(center, background) {
        return false;
    }
    let ink = |x, y| {
        let p = *image.get_pixel(x, y);
        p[3] == 255 && !close(p, background)
    };
    let xs = left + (right - left) / 4..=right - (right - left) / 4;
    let ys = top + (bottom - top) / 4..=bottom - (bottom - top) / 4;
    xs.clone().all(|x| {
        (a.y.min(b.y)..=top).all(|y| ink(x, y)) && (bottom..=ab.max(bb)).all(|y| ink(x, y))
    }) && ys.clone().all(|y| {
        (a.x.min(b.x)..=left).all(|x| ink(x, y)) && (right..=ar.max(br)).all(|x| ink(x, y))
    })
}

/// Retain an existing frame-bound opaque target, never fabricate an enlarged
/// clickable region. Conflicting value/mask evidence becomes unverified.
fn canonicalize_field_outlines(image: &RgbaImage, crop: Bounds, controls: &mut Vec<Value>) {
    let local = |c: &Value| {
        bounds(&c["bounds"]).ok().map(|b| Bounds {
            x: b.x - crop.x,
            y: b.y - crop.y,
            ..b
        })
    };
    let related = |a: usize, b: usize| {
        matches!(
            controls[a]["kind"].as_str(),
            Some("username" | "password" | "submit")
        ) && controls[a]["kind"] == controls[b]["kind"]
            && controls[a]["configuredAccountMatch"] == controls[b]["configuredAccountMatch"]
            && local(&controls[a])
                .zip(local(&controls[b]))
                .is_some_and(|(a, b)| same_field_outline(image, a, b))
    };
    let mut groups = vec![];
    let mut visited = vec![false; controls.len()];
    for start in 0..controls.len() {
        if visited[start] {
            continue;
        }
        let mut group = vec![start];
        visited[start] = true;
        let mut cursor = 0;
        while cursor < group.len() {
            for (other, seen) in visited.iter_mut().enumerate() {
                if !*seen && related(group[cursor], other) {
                    *seen = true;
                    group.push(other);
                }
            }
            cursor += 1;
        }
        if group.len() > 1
            && group
                .iter()
                .all(|a| group.iter().all(|b| a == b || related(*a, *b)))
        {
            // A competing kind or account at the same physical region prevents
            // normalization even if some pair of contours is individually sound.
            let conflict = controls.iter().enumerate().any(|(i, c)| {
                !group.contains(&i)
                    && group.iter().any(|j| {
                        local(c).zip(local(&controls[*j])).is_some_and(|(a, b)| {
                            a.x < b.x + b.width
                                && b.x < a.x + a.width
                                && a.y < b.y + b.height
                                && b.y < a.y + a.height
                        })
                    })
            });
            if !conflict {
                groups.push(group);
            }
        }
    }
    let mut remove = vec![false; controls.len()];
    for group in groups {
        let keep = *group
            .iter()
            .min_by_key(|i| {
                let b = local(&controls[**i]).expect("validated control bounds");
                (u64::from(b.width) * u64::from(b.height), b.y, b.x)
            })
            .unwrap();
        if group
            .iter()
            .any(|i| controls[*i]["fieldValueState"] != controls[keep]["fieldValueState"])
        {
            controls[keep]["fieldValueState"] = json!("unverified");
        }
        for i in group {
            remove[i] = i != keep;
        }
    }
    let mut i = 0;
    controls.retain(|_| {
        let keep = !remove[i];
        i += 1;
        keep
    });
}

/// Called only with the current owner-fenced capture. Pixel decoding is bounded
/// before allocation; arbitrary OCR and field values are never projected.
pub(crate) fn observe(capture: &Value, region: &Value, request: &Value) -> Result<Value, String> {
    use base64::Engine;
    validate_request(request)?;
    let crop = bounds(region)?;
    if u64::from(crop.width) * u64::from(crop.height) > 1_000_000 {
        return Err(INVALID.into());
    }
    let encoded = capture
        .pointer("/image/data")
        .and_then(Value::as_str)
        .filter(|s| s.len() <= 5_592_408)
        .ok_or(INVALID)?;
    let png = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| INVALID)?;
    if png.len() > 4_194_304 {
        return Err(INVALID.into());
    }
    let digest = format!("{:x}", Sha256::digest(&png));
    if capture.pointer("/capture/sha256").and_then(Value::as_str) != Some(digest.as_str()) {
        return Err("browser_native_login_frame_changed".into());
    }
    let size = image::ImageReader::with_format(Cursor::new(&png), image::ImageFormat::Png)
        .into_dimensions()
        .map_err(|_| INVALID)?;
    if size != (crop.width, crop.height) {
        return Err(INVALID.into());
    }
    let image = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|_| INVALID)?
        .to_rgba8();
    let rects = rectangles(&image)?;
    if capture["ocr"]["state"] != "observed" {
        return Err("browser_native_login_ocr_unavailable".into());
    }
    let ocr = &capture["ocr"];
    if ocr["engine"] != "tesseract"
        || ocr["engineVersion"] != "5.3.4"
        || ocr["language"] != "eng"
        || ocr["minimumConfidence"].as_f64() != Some(60.0)
        || ocr["modelSha256"] != "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    {
        return Err("browser_native_login_ocr_profile_unsupported".into());
    }

    let lines = capture["ocr"]["lines"]
        .as_array()
        .filter(|l| l.len() <= 512)
        .ok_or(INVALID)?;
    let mut controls = vec![];
    for line in lines {
        let confidence = line["confidence"]
            .as_f64()
            .filter(|c| c.is_finite() && (0.0..=100.0).contains(c))
            .ok_or(INVALID)?;
        let text = line["text"]
            .as_str()
            .filter(|s| s.len() <= 512 && !s.chars().any(char::is_control))
            .ok_or(INVALID)?;
        if confidence < 80.0 {
            continue;
        }
        let label = bounds(&line["bounds"])?;
        if !crop.contains(label) {
            return Err(INVALID.into());
        }
        let local = Bounds {
            x: label.x - crop.x,
            y: label.y - crop.y,
            ..label
        };
        let normalized = text.trim().trim_end_matches(':').to_ascii_lowercase();
        let kind = match normalized.as_str() {
            "email" | "email address" | "username" => "username",
            "password" => "password",
            "log in" | "sign in" | "login" => "submit",
            _ => continue,
        };
        for r in &rects {
            if r.height > 100 || r.width < 100 {
                continue;
            }
            let inside = r.contains(local);
            let above = kind != "submit"
                && r.y >= local.y + local.height
                && r.y - (local.y + local.height) <= 50
                && r.x.abs_diff(local.x) <= 32;
            if !inside && !above {
                continue;
            }
            let absolute = Bounds {
                x: r.x + crop.x,
                y: r.y + crop.y,
                ..*r
            };
            let state = value_state(&image, *r);
            let account_match = kind == "username"
                && request["expectedAccount"].as_str().is_some_and(|expected| {
                    lines.iter().any(|l| {
                        l["text"]
                            .as_str()
                            .is_some_and(|t| t.trim().eq_ignore_ascii_case(expected))
                            && l["confidence"].as_f64().is_some_and(|c| c >= 80.0)
                            && bounds(&l["bounds"])
                                .ok()
                                .is_some_and(|b| absolute.contains(b))
                    })
                });
            let binding = json!({"desktopId":capture["desktopId"],"generation":capture["generation"],"windowId":capture["windowId"],"process":capture["processIdentity"],"frame":digest,"kind":kind,"bounds":absolute});
            let id = format!(
                "login:{}",
                hex(&serde_json::to_vec(&binding).map_err(|_| INVALID)?)
            );
            if !controls.iter().any(|c: &Value| c["targetRef"] == id) {
                controls.push(json!({"targetRef":id,"kind":kind,"label":match kind{"username"=>"Username","password"=>"Password",_=>"Sign in"},"bounds":absolute,"confidence":confidence.min(90.0),"confidenceKind":"rule_score_not_probability","fieldValueState":state,"configuredAccountMatch":account_match}));
            }
        }
    }
    // A chooser entry needs a current flat popup contour and an exact English
    // Password Manager heading above it. Pixels cannot prove browser-native
    // origin, so these references remain read-only and grant no input authority.
    let headers = lines
        .iter()
        .filter(|l| {
            l["text"].as_str().is_some_and(|t| {
                matches!(
                    t.trim().to_ascii_lowercase().as_str(),
                    "password manager" | "passwords"
                )
            }) && l["confidence"].as_f64().is_some_and(|c| c >= 80.0)
        })
        .collect::<Vec<_>>();
    for line in lines {
        let Some(text) = line["text"].as_str() else {
            continue;
        };
        let text = text.trim();
        if text.len() > 254
            || !text.is_ascii()
            || text.chars().any(char::is_whitespace)
            || text.split('@').count() != 2
            || text.starts_with('@')
            || text.ends_with('@')
            || !line["confidence"]
                .as_f64()
                .is_some_and(|c| c.is_finite() && (80.0..=100.0).contains(&c))
        {
            continue;
        }
        let label = bounds(&line["bounds"])?;
        if !crop.contains(label) {
            return Err(INVALID.into());
        }
        let local = Bounds {
            x: label.x - crop.x,
            y: label.y - crop.y,
            ..label
        };
        let compact = lines.iter().any(|footer| {
            compact_panel(&image, &rects, crop, footer).is_some_and(|panel| {
                compact_row_count(lines, crop, panel) == 1
                    && compact_account(&image, panel, crop, local, footer)
            })
        });
        let corroborated = compact
            || rects.iter().any(|r| {
                r.width >= 150
                    && r.height >= 120
                    && r.contains(local)
                    && headers.iter().any(|h| {
                        bounds(&h["bounds"])
                            .ok()
                            .filter(|b| crop.contains(*b))
                            .is_some_and(|b| {
                                r.contains(Bounds {
                                    x: b.x - crop.x,
                                    y: b.y - crop.y,
                                    ..b
                                }) && b.y + b.height < label.y
                            })
                    })
            });
        if !corroborated {
            continue;
        }
        let binding = json!({"desktopId":capture["desktopId"],"generation":capture["generation"],"windowId":capture["windowId"],"process":capture["processIdentity"],"frame":digest,"kind":"credential_entry","bounds":label});
        let id = format!(
            "login:{}",
            hex(&serde_json::to_vec(&binding).map_err(|_| INVALID)?)
        );
        if controls.iter().any(|c: &Value| c["targetRef"] == id) {
            continue;
        }
        controls.push(json!({"targetRef":id,"kind":"credential_entry","label":"Saved account","bounds":label,"configuredAccountMatch":request["expectedAccount"].as_str().is_some_and(|a|a.eq_ignore_ascii_case(text)),"confidence":line["confidence"].as_f64().unwrap().min(90.0),"confidenceKind":"rule_score_not_probability","fieldValueState":"unverified"}));
    }
    canonicalize_field_outlines(&image, crop, &mut controls);
    controls.sort_by_key(|c| {
        (
            c["bounds"]["y"].as_u64(),
            c["bounds"]["x"].as_u64(),
            c["kind"].as_str().unwrap_or("").to_owned(),
        )
    });
    let ambiguous = ["username", "password", "submit"]
        .iter()
        .any(|k| controls.iter().filter(|c| c["kind"] == *k).count() > 1)
        || controls
            .iter()
            .filter(|c| {
                c["kind"] == "credential_entry"
                    && (request.get("expectedAccount").is_none()
                        || c["configuredAccountMatch"] == true)
            })
            .count()
            > 1;
    Ok(
        json!({"schemaVersion":"agent-browser.native-login-controls.v1","profile":PROFILE,"status":if ambiguous{"ambiguous"}else if controls.is_empty(){"not_found"}else{"observed"},"controls":controls,"coordinateSpace":"desktop_physical_pixels","frameSha256":digest,"detector":{"id":"native-login-pixel-ocr","version":"1","minimumOcrConfidence":80,"rgbTolerance":12,"maximumPixels":1000000,"ocrEngine":"tesseract","ocrEngineVersion":"5.3.4","ocrLanguage":"eng","ocrModelSha256":"7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"},"inputAuthorized":false,"authenticated":false,"limitation":"Read-only English flat rectangular control perception; OCR execution and live layouts require separate qualification; field values and browser-native origin are unverified; masked state requires same-frame repeated dot components."}),
    )
}
/// Refine only one independently bounded dark submit widget below an already
/// configured username and pixel-masked password. A second physical candidate
/// or disconnected contour is ambiguity, never a reason to choose by proximity.
fn submit_panel(
    image: &RgbaImage,
    crop: Bounds,
    original: &Value,
    rects: &[Bounds],
    lines: &[Value],
    request: &Value,
) -> Option<Bounds> {
    let controls = original["controls"].as_array()?;
    let users = controls
        .iter()
        .filter(|c| c["kind"] == "username")
        .collect::<Vec<_>>();
    let passwords = controls
        .iter()
        .filter(|c| c["kind"] == "password")
        .collect::<Vec<_>>();
    if users.len() != 1
        || passwords.len() != 1
        || users[0]["configuredAccountMatch"] != true
        || passwords[0]["fieldValueState"] != "masked"
        || controls.iter().any(|c| c["kind"] == "submit")
    {
        return None;
    }
    let user = bounds(&users[0]["bounds"]).ok()?;
    let expected = request["expectedAccount"].as_str()?;
    if lines.iter().any(|l| {
        l["confidence"].as_f64().is_some_and(|c| c >= 80.0)
            && l["text"]
                .as_str()
                .is_some_and(|t| t.contains('@') && !t.trim().eq_ignore_ascii_case(expected))
            && bounds(&l["bounds"]).ok().is_some_and(|b| user.contains(b))
    }) {
        return None;
    }
    let p = bounds(&passwords[0]["bounds"]).ok()?;
    let p = Bounds {
        x: p.x.checked_sub(crop.x)?,
        y: p.y.checked_sub(crop.y)?,
        ..p
    };
    let candidates = rects
        .iter()
        .copied()
        .filter(|r| {
            if r.width < 100
                || r.width > 480
                || r.height < 22
                || r.height > 80
                || r.y < p.y + p.height
                || r.y - (p.y + p.height) > 100
                || r.x.abs_diff(p.x) > 32
                || r.width.abs_diff(p.width) > 64
            {
                return false;
            }
            let fill = *image.get_pixel(r.x + r.width / 2, r.y);
            fill[3] == 255
                && (0..3).all(|i| fill[i] <= 64)
                && filled_field_component(image, *r, fill).is_some()
        })
        .collect::<Vec<_>>();
    if candidates.is_empty()
        || !candidates.iter().all(|a| {
            candidates
                .iter()
                .all(|b| a == b || same_field_outline(image, *a, *b))
        })
    {
        return None;
    }
    candidates
        .into_iter()
        .min_by_key(|r| (r.width * r.height, r.y, r.x))
}

/// Refine at most one compact popup or qualified dark submit widget from this
/// validated owner frame. Unavailable or conflicting OCR remains conservative.
pub(crate) fn observe_refined(
    capture: &Value,
    region: &Value,
    request: &Value,
    mut refine: impl FnMut(&RgbaImage) -> Result<Vec<Value>, String>,
) -> Result<Value, String> {
    let original = observe(capture, region, request)?;
    if original["controls"]
        .as_array()
        .is_some_and(|cs| cs.iter().any(|c| c["kind"] == "credential_entry"))
    {
        return Ok(original);
    }
    use base64::Engine;
    let png = base64::engine::general_purpose::STANDARD
        .decode(capture["image"]["data"].as_str().ok_or(INVALID)?)
        .map_err(|_| INVALID)?;
    let image = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|_| INVALID)?
        .to_rgba8();
    let crop = bounds(region)?;
    let rects = rectangles(&image)?;
    let lines = capture["ocr"]["lines"].as_array().ok_or(INVALID)?;
    if let Some(panel) = submit_panel(&image, crop, &original, &rects, lines, request) {
        let roi = image::imageops::crop_imm(&image, panel.x, panel.y, panel.width, panel.height)
            .to_image();
        let doubled = image::imageops::resize(
            &roi,
            panel.width * 2,
            panel.height * 2,
            image::imageops::FilterType::Nearest,
        );
        let Ok(refined) = refine(&doubled) else {
            return Ok(original);
        };
        if refined.len() > 512 {
            return Ok(original);
        }
        let captions = refined
            .iter()
            .filter(|l| {
                l["confidence"].as_f64().is_some_and(|c| c >= 80.0)
                    && l["text"].as_str().is_some_and(|t| {
                        matches!(
                            t.trim().to_ascii_lowercase().as_str(),
                            "log in" | "sign in" | "login"
                        )
                    })
            })
            .collect::<Vec<_>>();
        if captions.len() != 1 {
            return Ok(original);
        }
        let mut caption = captions[0].clone();
        let Ok(b) = bounds(&caption["bounds"]) else {
            return Ok(original);
        };
        if b.x + b.width > doubled.width() || b.y + b.height > doubled.height() {
            return Ok(original);
        }
        let (x, y) = (b.x / 2, b.y / 2);
        caption["bounds"] = json!({"x":crop.x+panel.x+x,"y":crop.y+panel.y+y,"width":(b.x+b.width).div_ceil(2)-x,"height":(b.y+b.height).div_ceil(2)-y});
        let mut updated = capture.clone();
        let mut combined = lines.clone();
        combined.push(caption);
        if combined.len() > 512 {
            return Ok(original);
        }
        updated["ocr"]["lines"] = json!(combined);
        return observe(&updated, region, request).or(Ok(original));
    }
    let panels = lines
        .iter()
        .filter_map(|f| compact_panel(&image, &rects, crop, f))
        .collect::<Vec<_>>();
    if panels.len() != 1 {
        return Ok(original);
    }
    let panel = panels[0];
    let roi =
        image::imageops::crop_imm(&image, panel.x, panel.y, panel.width, panel.height).to_image();
    let doubled = image::imageops::resize(
        &roi,
        panel.width * 2,
        panel.height * 2,
        image::imageops::FilterType::Nearest,
    );
    let Ok(refined) = refine(&doubled) else {
        return Ok(original);
    };
    if refined.len() > 512 {
        return Ok(original);
    }
    let mut mapped = Vec::new();
    for mut line in refined {
        let Ok(b) = bounds(&line["bounds"]) else {
            return Ok(original);
        };
        if b.x + b.width > doubled.width() || b.y + b.height > doubled.height() {
            return Ok(original);
        }
        let x = b.x / 2;
        let y = b.y / 2;
        line["bounds"] = json!({"x":crop.x+panel.x+x,"y":crop.y+panel.y+y,"width":(b.x+b.width).div_ceil(2)-x,"height":(b.y+b.height).div_ceil(2)-y});
        mapped.push(line);
    }
    if compact_row_count(&mapped, crop, panel) != 1 {
        return Ok(original);
    }
    let qualified = mapped
        .iter()
        .filter(|l| {
            l["confidence"].as_f64().is_some_and(|n| n >= 80.0)
                && l["text"].as_str().is_some_and(|t| t.contains('@'))
        })
        .collect::<Vec<_>>();
    if qualified.len() != 1 {
        return Ok(original);
    }
    let expected = qualified[0]["text"].as_str().unwrap_or("").trim();
    if lines.iter().any(|l| {
        l["confidence"].as_f64().is_some_and(|n| n >= 80.0)
            && l["text"]
                .as_str()
                .is_some_and(|t| t.contains('@') && !t.trim().eq_ignore_ascii_case(expected))
            && bounds(&l["bounds"]).ok().is_some_and(|b| {
                panel.contains(Bounds {
                    x: b.x.saturating_sub(crop.x),
                    y: b.y.saturating_sub(crop.y),
                    ..b
                })
            })
    }) {
        return Ok(original);
    }
    let mut updated = capture.clone();
    let mut combined = lines.clone();
    combined.extend(mapped);
    if combined.len() > 512 {
        return Ok(original);
    }
    updated["ocr"]["lines"] = json!(combined);
    observe(&updated, region, request).or(Ok(original))
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
#[path = "native_login_perception_corpus.rs"]
mod corpus;

pub(crate) struct SelectedLoginTarget {
    pub binding: Value,
    pub frame_sha256: String,
    pub target_ref: String,
    pub kind: String,
    pub x: u16,
    pub y: u16,
}
/// Resolve only a unique opaque target in one owner-retained observation. The
/// caller/journal still supplies exact task and immutable operation admission.
pub(crate) fn select_target(
    observation: &Value,
    target_ref: &str,
    now_ms: u64,
) -> Result<SelectedLoginTarget, String> {
    let binding = &observation["binding"];
    let controls = &observation["controls"];
    let at = binding["observedAtMs"].as_u64().ok_or(INVALID)?;
    if now_ms < at || now_ms - at > 10_000 {
        return Err("browser_native_login_observation_expired".into());
    }
    let frame = controls["frameSha256"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(INVALID)?;
    let window = bounds(&binding["window"])?;
    let region = bounds(&binding["region"])?;
    if !window.contains(region)
        || controls["status"] != "observed"
        || binding["desktopId"].as_str().is_none()
        || binding["generation"].as_u64().is_none()
        || binding["windowId"].as_u64().is_none()
        || binding["nativeControlEpoch"].as_u64().is_none()
    {
        return Err("browser_native_login_observation_unqualified".into());
    }
    let items = controls["controls"]
        .as_array()
        .filter(|a| a.len() <= 64)
        .ok_or(INVALID)?;
    let candidates = items
        .iter()
        .filter(|c| c["targetRef"] == target_ref)
        .collect::<Vec<_>>();
    if candidates.len() != 1
        || !target_ref
            .strip_prefix("login:")
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("browser_native_login_target_invalid".into());
    }
    let target = candidates[0];
    let b = bounds(&target["bounds"])?;
    let kind = target["kind"]
        .as_str()
        .filter(|k| matches!(*k, "username" | "credential_entry" | "submit"))
        .ok_or("browser_native_login_target_unsupported")?;
    if !region.contains(b) || !window.contains(b) {
        return Err(INVALID.into());
    }
    if kind == "credential_entry" && target["configuredAccountMatch"] != true {
        return Err("browser_native_login_account_unmatched".into());
    }
    if kind == "submit"
        && !(items
            .iter()
            .any(|c| c["kind"] == "username" && c["configuredAccountMatch"] == true)
            && items
                .iter()
                .any(|c| c["kind"] == "password" && c["fieldValueState"] == "masked"))
    {
        return Err("browser_native_login_filled_form_unverified".into());
    }
    let x = b.x + b.width / 2;
    let y = b.y + b.height / 2;
    if u64::from(x) >= binding["displayWidth"].as_u64().ok_or(INVALID)?
        || u64::from(y) >= binding["displayHeight"].as_u64().ok_or(INVALID)?
    {
        return Err(INVALID.into());
    }
    Ok(SelectedLoginTarget {
        binding: binding.clone(),
        frame_sha256: frame.into(),
        target_ref: target_ref.into(),
        kind: kind.into(),
        x: u16::try_from(x).map_err(|_| INVALID)?,
        y: u16::try_from(y).map_err(|_| INVALID)?,
    })
}

// Count actual contrasting glyph components, never OCR/password text. The
// supported Chrome profile excludes the trailing visibility icon area.
fn value_state(image: &RgbaImage, b: Bounds) -> &'static str {
    if b.width < 70 || b.height < 20 {
        return "unverified";
    }
    let background = *image.get_pixel(b.x + 3, b.y + 3);
    let left = b.x + 6;
    let right = b.x + b.width - 40;
    let top = b.y + 5;
    let bottom = b.y + b.height - 5;
    let mut visited = std::collections::BTreeSet::new();
    let mut components = vec![];
    for y in top..bottom {
        for x in left..right {
            if visited.contains(&(x, y)) || close(*image.get_pixel(x, y), background) {
                continue;
            }
            let mut pending = vec![(x, y)];
            visited.insert((x, y));
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (x, y, x, y);
            let mut ink = 0u32;
            while let Some((px, py)) = pending.pop() {
                ink += 1;
                min_x = min_x.min(px);
                max_x = max_x.max(px);
                min_y = min_y.min(py);
                max_y = max_y.max(py);
                for (nx, ny) in [
                    (px.saturating_sub(1), py),
                    (px + 1, py),
                    (px, py.saturating_sub(1)),
                    (px, py + 1),
                ] {
                    if nx >= left
                        && nx < right
                        && ny >= top
                        && ny < bottom
                        && !visited.contains(&(nx, ny))
                        && !close(*image.get_pixel(nx, ny), background)
                    {
                        visited.insert((nx, ny));
                        pending.push((nx, ny));
                    }
                }
            }
            components.push((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1, ink));
            if components.len() > 64 {
                return "text_present";
            }
        }
    }
    if components.is_empty() {
        return "empty";
    }
    let max_dot = (b.height / 3).min(16);
    let first_center = components[0].1 + components[0].3 / 2;
    if components.len() >= 3
        && components.iter().all(|(_, y, w, h, ink)| {
            *w >= 2
                && *h >= 2
                && *w <= max_dot
                && *h <= max_dot
                && w.abs_diff(*h) <= 3
                && ink * 100 >= w * h * 70
                && (y + h / 2).abs_diff(first_center) <= 3
        })
    {
        "masked"
    } else {
        "text_present"
    }
}

/// Receiver-side allowlist. In particular a newer client cannot accidentally
/// export raw PNG/OCR from an older Native host that ignored the opt-in flag.
pub(crate) fn curate_observation(value: &Value) -> Result<Value, String> {
    let b = &value["binding"];
    let c = &value["controls"];
    let id = b["desktopId"]
        .as_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|v| v.is_ascii_alphanumeric() || b"-_: .".contains(&v))
        })
        .ok_or(INVALID)?;
    let n = |key: &str| b[key].as_u64().ok_or(INVALID);
    let window = bounds(&b["window"])?;
    let region = bounds(&b["region"])?;
    let width = n("displayWidth")?;
    let height = n("displayHeight")?;
    if !window.contains(region)
        || u64::from(region.x) + u64::from(region.width) > width
        || u64::from(region.y) + u64::from(region.height) > height
        || u64::from(region.width) * u64::from(region.height) > 1_000_000
        || n("generation")? == 0
        || n("windowId")? == 0
    {
        return Err(INVALID.into());
    }
    if c["schemaVersion"] != "agent-browser.native-login-controls.v1"
        || c["profile"] != PROFILE
        || !matches!(
            c["status"].as_str(),
            Some("observed" | "ambiguous" | "not_found")
        )
    {
        return Err(INVALID.into());
    }
    let sha = c["frameSha256"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(INVALID)?;
    let items = c["controls"]
        .as_array()
        .filter(|items| items.len() <= 64)
        .ok_or(INVALID)?;
    let mut controls = vec![];
    for item in items {
        let reference = item["targetRef"]
            .as_str()
            .filter(|s| {
                s.strip_prefix("login:")
                    .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            })
            .ok_or(INVALID)?;
        let kind = item["kind"]
            .as_str()
            .filter(|k| matches!(*k, "username" | "password" | "submit" | "credential_entry"))
            .ok_or(INVALID)?;
        let bb = bounds(&item["bounds"])?;
        if !region.contains(bb) {
            return Err(INVALID.into());
        }
        let state = item["fieldValueState"]
            .as_str()
            .filter(|s| matches!(*s, "empty" | "masked" | "text_present" | "unverified"))
            .ok_or(INVALID)?;
        let matched = item
            .get("configuredAccountMatch")
            .map(|v| v.as_bool().ok_or(INVALID))
            .transpose()?
            .unwrap_or(false);
        let mut clean = json!({"targetRef":reference,"kind":kind,"label":match kind{"username"=>"Username","password"=>"Password","submit"=>"Sign in",_=>"Saved account"},"bounds":bb,"configuredAccountMatch":matched,"fieldValueState":state});
        if let Some(score) = item.get("confidence") {
            let score = score
                .as_f64()
                .filter(|v| v.is_finite() && (0.0..=100.0).contains(v))
                .ok_or(INVALID)?;
            clean["confidence"] = json!(score);
            clean["confidenceKind"] = json!("rule_score_not_probability");
        }
        controls.push(clean);
    }
    let binding = json!({"desktopId":id,"generation":n("generation")?,"windowId":n("windowId")?,"window":window,"region":region,"displayWidth":width,"displayHeight":height,"nativeControlEpoch":n("nativeControlEpoch")?,"observedAtMs":n("observedAtMs")?});
    Ok(
        json!({"binding":binding,"controls":{"schemaVersion":"agent-browser.native-login-controls.v1","profile":PROFILE,"status":c["status"],"controls":controls,"frameSha256":sha,"coordinateSpace":"desktop_physical_pixels","detector":{"id":"native-login-pixel-ocr","version":"1","minimumOcrConfidence":80,"rgbTolerance":12,"maskMinimumInkPercent":70,"maximumPixels":1000000,"ocrEngine":"tesseract","ocrEngineVersion":"5.3.4","ocrLanguage":"eng","ocrModelSha256":"7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"},"inputAuthorized":false,"authenticated":false}}),
    )
}
pub(crate) fn curate_response(value: &Value) -> Result<Value, String> {
    if value["source"] != "remote_view_native"
        || value["binding"] != value["nativeLoginObservation"]["binding"]
        || value["loginControls"] != value["nativeLoginObservation"]["controls"]
    {
        return Err("browser_native_login_producer_contract_unavailable".into());
    }
    let observation = curate_observation(&value["nativeLoginObservation"])?;
    Ok(
        json!({"source":"remote_view_native","binding":observation["binding"],"loginControls":observation["controls"],"nativeLoginObservation":observation,"taskCompleted":false}),
    )
}
