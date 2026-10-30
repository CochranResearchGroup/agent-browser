//! Bounded private readback for automatic browser maintenance, independent of stderr.
use agent_browser_service_model::ReapBrowserSessionsResult;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn error_code(reason: &str) -> &str {
    let code = reason.split(':').next().unwrap_or_default();
    if code.len() <= 96
        && (code.starts_with("browser_")
            || code.starts_with("remote_view_")
            || code.starts_with("service_"))
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        code
    } else {
        "browser_session_reap_failed"
    }
}

/// Run the owner pass once. A receipt failure is observable but never retries
/// or vetoes the maintenance operation. Only bounded codes cross this boundary.
pub(super) fn run_pass(
    path: &Path,
    started_at_ms: u64,
    maintenance: impl FnOnce() -> Result<ReapBrowserSessionsResult, String>,
) -> Result<ReapBrowserSessionsResult, String> {
    let mut receipt = json!({"schemaVersion":1,"producerPid":std::process::id(),
        "startedAtMs":started_at_ms,"completedAtMs":null,"outcome":"running"});
    // A running receipt makes an interrupted or blocked pass distinguishable
    // from an old successful pass. Diagnostics must not prevent cleanup itself.
    let _ = super::browser_session_store::write_private_json_atomic(path, &receipt);
    let result = maintenance();
    receipt["completedAtMs"] = json!(now_ms());
    match &result {
        Ok(outcome) => {
            let deferrals = outcome
                .deferred_browser_errors
                .iter()
                .take(64)
                .map(|(id, reason)| {
                    json!({"browserIdSha256":format!("{:x}", Sha256::digest(id.as_bytes())),
                    "reasonCode":error_code(reason)})
                })
                .collect::<Vec<Value>>();
            receipt["outcome"] = json!(if outcome.deferred_browser_errors.is_empty() {
                "completed"
            } else {
                "deferred"
            });
            receipt["expiredTabCount"] = json!(outcome.expired_tab_ids.len());
            receipt["expiredSessionCount"] = json!(outcome.expired_session_ids.len());
            receipt["closedBrowserCount"] = json!(outcome.closed_browser_ids.len());
            receipt["deletedDisposableProfileCount"] =
                json!(outcome.deleted_disposable_profile_ids.len());
            receipt["quotaEvictedSessionCount"] = json!(outcome.quota_evicted_session_ids.len());
            receipt["deferredBrowserCount"] = json!(outcome.deferred_browser_errors.len());
            receipt["omittedDeferredBrowserCount"] =
                json!(outcome.deferred_browser_errors.len().saturating_sub(64));
            receipt["deferredBrowsers"] = json!(deferrals);
        }
        Err(reason) => {
            receipt["outcome"] = json!("failed");
            receipt["errorCode"] = json!(error_code(reason));
        }
    }
    let written = super::browser_session_store::write_private_json_atomic(path, &receipt);
    let outcome = result?;
    written.map_err(|_| "browser_session_reap_receipt_write_failed".to_string())?;
    if !outcome.deferred_browser_errors.is_empty() {
        return Err("browser_session_reap_deferred".into());
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::cell::Cell;

    #[test]
    fn background_pass_persists_failure_and_next_success_without_replaying_effects() {
        let directory =
            std::env::temp_dir().join(format!("reaper-readback-{}", uuid::Uuid::new_v4()));
        let path = directory.join("browser-session-reaper.json");
        let calls = Cell::new(0);
        let result = run_pass(&path, 100, || {
            calls.set(calls.get() + 1);
            Err("browser_session_idle_readback_required:private detail".into())
        });
        assert!(result.is_err());
        let failed: Value = serde_json::from_slice(
            &std::fs::read(&path).expect("failed pass must remain inspectable without stderr"),
        )
        .unwrap();
        assert_eq!(failed["outcome"], "failed");
        assert_eq!(failed["startedAtMs"], 100);
        assert_eq!(
            failed["errorCode"],
            "browser_session_idle_readback_required"
        );
        assert!(!failed.to_string().contains("private detail"));
        run_pass(&path, 200, || {
            calls.set(calls.get() + 1);
            Ok(ReapBrowserSessionsResult {
                expired_tab_ids: vec!["tab".into()],
                ..Default::default()
            })
        })
        .unwrap();
        let passed: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(passed["outcome"], "completed");
        assert_eq!(passed["expiredTabCount"], 1);
        assert_eq!(passed["startedAtMs"], 200);
        assert!(passed["errorCode"].is_null());
        assert_eq!(calls.get(), 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn deferred_readback_is_bounded_and_omits_private_browser_names() {
        let directory =
            std::env::temp_dir().join(format!("reaper-deferral-{}", uuid::Uuid::new_v4()));
        let path = directory.join("status.json");
        let result = run_pass(&path, 100, || {
            Ok(ReapBrowserSessionsResult {
                deferred_browser_errors: (0..70)
                    .map(|index| {
                        (
                            format!("private-browser-{index}"),
                            "browser_session_idle_original_identity_missing:private detail".into(),
                        )
                    })
                    .collect(),
                ..Default::default()
            })
        });
        assert_eq!(result.unwrap_err(), "browser_session_reap_deferred");
        let raw = std::fs::read_to_string(&path).unwrap();
        let receipt: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(receipt["outcome"], "deferred");
        assert_eq!(receipt["deferredBrowserCount"], 70);
        assert_eq!(receipt["deferredBrowsers"].as_array().unwrap().len(), 64);
        assert_eq!(receipt["omittedDeferredBrowserCount"], 6);
        assert!(!raw.contains("private-browser"));
        assert!(!raw.contains("private detail"));
        assert_eq!(
            receipt["deferredBrowsers"][0]["reasonCode"],
            "browser_session_idle_original_identity_missing"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unavailable_receipt_storage_never_vetoes_or_repeats_the_owner_pass() {
        let path = std::env::temp_dir().join(format!("reaper-storage-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, "occupied by a regular file").unwrap();
        let calls = Cell::new(0);
        let result = run_pass(&path.join("status.json"), 100, || {
            calls.set(calls.get() + 1);
            Ok(ReapBrowserSessionsResult::default())
        });
        assert_eq!(
            result.unwrap_err(),
            "browser_session_reap_receipt_write_failed"
        );
        assert_eq!(calls.get(), 1);
        std::fs::remove_file(path).unwrap();
    }
}
