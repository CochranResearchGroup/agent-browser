use serde_json::{json, Value};
use similar::{ChangeTag, TextDiff};

pub struct ScreenshotDiffResult {
    pub total_pixels: u64,
    pub different_pixels: u64,
    pub mismatch_percentage: f64,
    pub matched: bool,
    pub diff_image: Option<Vec<u8>>,
    pub dimension_mismatch: Option<Value>,
}

pub struct SnapshotDiffResult {
    pub diff: String,
    pub additions: usize,
    pub removals: usize,
    pub unchanged: usize,
    pub changed: bool,
}

pub fn diff_screenshot(
    baseline: &[u8],
    current: &[u8],
    threshold: f64,
) -> Result<ScreenshotDiffResult, String> {
    let img_a = image::load_from_memory(baseline)
        .map_err(|e| format!("Failed to decode baseline image: {}", e))?;
    let img_b = image::load_from_memory(current)
        .map_err(|e| format!("Failed to decode current image: {}", e))?;

    let (wa, ha) = (img_a.width(), img_a.height());
    let (wb, hb) = (img_b.width(), img_b.height());

    if wa != wb || ha != hb {
        return Ok(ScreenshotDiffResult {
            total_pixels: (wa as u64) * (ha as u64),
            different_pixels: (wa as u64) * (ha as u64),
            mismatch_percentage: 100.0,
            matched: false,
            diff_image: None,
            dimension_mismatch: Some(json!({
                "expected": { "width": wa, "height": ha },
                "actual": { "width": wb, "height": hb },
            })),
        });
    }

    let rgba_a = img_a.to_rgba8();
    let rgba_b = img_b.to_rgba8();
    let total = (wa as u64) * (ha as u64);
    let max_color_distance = threshold * 255.0 * (3.0_f64).sqrt();
    let mut different = 0u64;

    let mut diff_img = image::RgbaImage::new(wa, ha);

    for y in 0..ha {
        for x in 0..wa {
            let pa = rgba_a.get_pixel(x, y);
            let pb = rgba_b.get_pixel(x, y);
            let dr = (pa[0] as f64) - (pb[0] as f64);
            let dg = (pa[1] as f64) - (pb[1] as f64);
            let db = (pa[2] as f64) - (pb[2] as f64);
            let dist = (dr * dr + dg * dg + db * db).sqrt();

            if dist > max_color_distance {
                different += 1;
                diff_img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            } else {
                let gray = ((pa[0] as u16 + pa[1] as u16 + pa[2] as u16) / 3) as u8;
                let dimmed = (gray as f64 * 0.3) as u8;
                diff_img.put_pixel(x, y, image::Rgba([dimmed, dimmed, dimmed, 255]));
            }
        }
    }

    let mismatch = if total > 0 {
        (different as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let diff_bytes = if different > 0 {
        let mut buf = std::io::Cursor::new(Vec::new());
        diff_img
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode diff image: {}", e))?;
        Some(buf.into_inner())
    } else {
        None
    };

    Ok(ScreenshotDiffResult {
        total_pixels: total,
        different_pixels: different,
        mismatch_percentage: mismatch,
        matched: different == 0,
        diff_image: diff_bytes,
        dimension_mismatch: None,
    })
}

/// Compute a snapshot diff using the Myers algorithm via the `similar` crate.
pub fn diff_snapshots(before: &str, after: &str) -> SnapshotDiffResult {
    // Fast path: identical inputs.
    // This avoids constructing the `similar` TextDiff object and running the diff
    // iteration when agents compare a snapshot to itself (common in retry/loop
    // workloads).
    if before == after {
        let unchanged = before.lines().count();
        return SnapshotDiffResult {
            diff: String::new(),
            additions: 0,
            removals: 0,
            unchanged,
            changed: false,
        };
    }

    let text_diff = TextDiff::from_lines(before, after);

    let mut additions = 0usize;
    let mut removals = 0usize;
    let mut unchanged = 0usize;

    for change in text_diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => removals += 1,
            ChangeTag::Equal => unchanged += 1,
        }
    }

    let changed = additions > 0 || removals > 0;

    let diff = text_diff
        .unified_diff()
        .context_radius(3)
        .header("before", "after")
        .to_string();

    SnapshotDiffResult {
        diff,
        additions,
        removals,
        unchanged,
        changed,
    }
}

/// Legacy JSON diff output for backwards compatibility.
pub fn diff_text(a: &str, b: &str) -> Value {
    let result = diff_snapshots(a, b);
    json!({
        "identical": !result.changed,
        "additions": result.additions,
        "removals": result.removals,
        "deletions": result.removals,
        "unchanged": result.unchanged,
        "changed": result.changed,
    })
}

pub fn diff_unified(a: &str, b: &str) -> String {
    diff_snapshots(a, b).diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_identical() {
        let result = diff_text("hello\nworld", "hello\nworld");
        assert_eq!(result.get("identical").unwrap(), true);
        assert_eq!(result.get("changed").unwrap(), false);
        assert_eq!(result.get("unchanged").unwrap(), 2);
    }

    #[test]
    fn test_diff_additions() {
        let result = diff_text("hello\n", "hello\nworld\n");
        assert_eq!(result.get("identical").unwrap(), false);
        assert_eq!(result.get("changed").unwrap(), true);
        assert!(result.get("additions").unwrap().as_i64().unwrap() > 0);
    }

    #[test]
    fn test_diff_deletions() {
        let result = diff_text("hello\nworld\n", "hello\n");
        assert_eq!(result.get("identical").unwrap(), false);
        assert!(result.get("removals").unwrap().as_i64().unwrap() > 0);
    }

    #[test]
    fn test_diff_unified_output() {
        let output = diff_unified("a\nb\nc\n", "a\nx\nc\n");
        assert!(output.contains("---"));
        assert!(output.contains("+++"));
    }

    #[test]
    fn test_snapshot_diff_struct() {
        let result = diff_snapshots("line1\nline2\n", "line1\nline3\n");
        assert!(result.changed);
        assert_eq!(result.additions, 1);
        assert_eq!(result.removals, 1);
        assert_eq!(result.unchanged, 1);
        assert!(!result.diff.is_empty());
    }

    #[test]
    fn test_diff_snapshots_identical_fast_path() {
        let input = "hello\nworld\n";
        let result = diff_snapshots(input, input);
        assert!(!result.changed);
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 0);
        assert_eq!(result.unchanged, input.lines().count());
        assert!(result.diff.is_empty());
    }

    #[test]
    #[ignore]
    fn bench_diff_snapshots_identical_and_changed() {
        use std::hint::black_box;
        use std::time::Instant;

        let identical_a = (0..200)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let identical_b = identical_a.clone();

        let changed_a = identical_a.clone();
        let changed_b = (0..200)
            .map(|i| {
                if i == 123 {
                    format!("line {i} changed")
                } else {
                    format!("line {i}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Keep the iteration count high enough to measure, but low enough
        // to avoid long CI times when someone runs `--ignored`.
        let iters = 50_000usize;

        let start = Instant::now();
        let mut acc_changed = 0usize;
        for _ in 0..iters {
            let r = diff_snapshots(black_box(&identical_a), black_box(&identical_b));
            acc_changed ^= r.unchanged;
        }
        let identical_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let mut acc_changed2 = 0usize;
        for _ in 0..iters {
            let r = diff_snapshots(black_box(&changed_a), black_box(&changed_b));
            acc_changed2 ^= r.additions;
        }
        let changed_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Prevent the compiler from optimizing everything away.
        black_box(acc_changed);
        black_box(acc_changed2);

        println!(
            "bench_diff_snapshots_identical_and_changed: iters={iters} identical_ms={identical_ms:.2} changed_ms={changed_ms:.2}"
        );
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::common::*;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    pub(crate) async fn handle_diff_snapshot(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let compact = cmd
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_depth = cmd
            .get("maxDepth")
            .and_then(|v| v.as_u64())
            .map(|d| d as usize);
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .map(String::from);
        let options = SnapshotOptions {
            compact,
            depth: max_depth,
            selector,
            ..SnapshotOptions::default()
        };
        let current = snapshot::take_snapshot(
            &mgr.client,
            &session_id,
            &options,
            &mut state.ref_map,
            state.active_frame_id.as_deref(),
            &state.iframe_sessions,
        )
        .await?;
        let baseline = cmd.get("baseline").and_then(|v| v.as_str());
        let baseline_text = match baseline {
            Some(b) if std::path::Path::new(b).exists() => {
                std::fs::read_to_string(b).map_err(|e| format!("Failed to read baseline: {}", e))?
            }
            Some(b) => b.to_string(),
            None => String::new(),
        };
        let result = diff::diff_snapshots(&baseline_text, &current);
        Ok(json!(
            { "diff" : result.diff, "additions" : result.additions, "removals" :
            result.removals, "unchanged" : result.unchanged, "changed" : result
            .changed, }
        ))
    }
    pub(crate) async fn handle_diff_url(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        let url1 = cmd
            .get("url1")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url1' parameter")?;
        let url2 = cmd
            .get("url2")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url2' parameter")?;
        let wait_until = cmd
            .get("waitUntil")
            .and_then(|v| v.as_str())
            .map(WaitUntil::from_str)
            .unwrap_or(WaitUntil::Load);
        mgr.navigate(url1, wait_until).await?;
        let session_id = mgr.active_session_id()?.to_string();
        let options = SnapshotOptions::default();
        let snap1 = snapshot::take_snapshot(
            &mgr.client,
            &session_id,
            &options,
            &mut state.ref_map,
            None,
            &state.iframe_sessions,
        )
        .await?;
        mgr.navigate(url2, wait_until).await?;
        state.ref_map.clear();
        let snap2 = snapshot::take_snapshot(
            &mgr.client,
            &session_id,
            &options,
            &mut state.ref_map,
            None,
            &state.iframe_sessions,
        )
        .await?;
        let result = diff::diff_text(&snap1, &snap2);
        Ok(json!(
            { "diff" : result, "url1" : url1, "url2" : url2, "snapshot1" : snap1,
            "snapshot2" : snap2, }
        ))
    }
    pub(crate) async fn handle_diff_screenshot(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let baseline_path = cmd
            .get("baseline")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'baseline' parameter")?;
        let threshold = cmd.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.1);
        let options = ScreenshotOptions {
            selector: cmd
                .get("selector")
                .and_then(|v| v.as_str())
                .map(String::from),
            path: None,
            full_page: cmd
                .get("fullPage")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            format: "png".to_string(),
            quality: None,
            annotate: false,
            output_dir: None,
        };
        let result = screenshot::take_screenshot(
            &mgr.client,
            &session_id,
            &state.ref_map,
            &options,
            &state.iframe_sessions,
        )
        .await?;
        let current_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.base64)
                .map_err(|e| format!("Failed to decode screenshot: {}", e))?;
        let baseline_bytes =
            std::fs::read(baseline_path).map_err(|e| format!("Failed to read baseline: {}", e))?;
        let result = diff::diff_screenshot(&baseline_bytes, &current_bytes, threshold)?;
        let output_path = cmd.get("output").and_then(|v| v.as_str());
        if let (Some(out_path), Some(ref diff_data)) = (output_path, &result.diff_image) {
            std::fs::write(out_path, diff_data)
                .map_err(|e| format!("Failed to write diff image: {}", e))?;
        }
        Ok(json!(
            { "match" : result.matched, "mismatchPercentage" : result
            .mismatch_percentage, "totalPixels" : result.total_pixels,
            "differentPixels" : result.different_pixels, "diffPath" : output_path,
            "dimensionMismatch" : result.dimension_mismatch, }
        ))
    }
}
pub(crate) use action_commands::*;
