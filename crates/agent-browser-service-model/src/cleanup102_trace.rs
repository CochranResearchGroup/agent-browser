//! Temporary, opt-in publication diagnostics. Never prints record contents.
use std::sync::atomic::{AtomicUsize, Ordering};

static EVENTS: AtomicUsize = AtomicUsize::new(0);
const LIMIT: usize = 32;

fn reserve(counter: &AtomicUsize) -> bool {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
            (count < LIMIT).then_some(count + 1)
        })
        .is_ok()
}

/// Temporary diagnostics only; labels are source literals, never user data.
pub fn guard(label: &'static str) {
    if enabled() && reserve(&EVENTS) {
        eprintln!("[DEBUG-cleanup101] guard={label}");
    }
}

/// Whether this process has explicitly opted into the temporary trace.
pub fn enabled() -> bool {
    std::env::var_os("AGENT_BROWSER_DEBUG_CLEANUP102").as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn changed_fields(before: &serde_json::Value, after: &serde_json::Value) -> String {
    let names = before
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
        .chain(
            after
                .as_object()
                .into_iter()
                .flat_map(|object| object.keys()),
        )
        .collect::<std::collections::BTreeSet<_>>();
    names
        .into_iter()
        .filter(|name| {
            name.len() <= 48
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                && before.get(*name) != after.get(*name)
        })
        .take(32)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

fn context_digest(label: &'static str, value: &serde_json::Value) {
    use sha2::Digest;
    if let Ok(bytes) = serde_json::to_vec(value) {
        if reserve(&EVENTS) {
            eprintln!(
                "[DEBUG-cleanup101] guard={label} sha256={:x}",
                sha2::Sha256::digest(bytes)
            );
        }
    }
}

/// Only hashes and names of changed top-level struct fields leave this function.
/// This is called once after a rejected publication, never on its success path.
pub fn publication_context(
    expected: &crate::BrowserSessionState,
    actual: &crate::BrowserSessionState,
    proposed: &crate::BrowserSessionState,
    merged: &crate::BrowserSessionState,
) {
    if !enabled() {
        return;
    }
    let (Ok(expected), Ok(actual), Ok(proposed), Ok(merged)) = (
        serde_json::to_value(expected),
        serde_json::to_value(actual),
        serde_json::to_value(proposed),
        serde_json::to_value(merged),
    ) else {
        guard("publication.context_unavailable");
        return;
    };
    for (label, value) in [
        ("publication.expected", &expected),
        ("publication.actual", &actual),
        ("publication.proposed", &proposed),
        ("publication.merged", &merged),
    ] {
        context_digest(label, value);
    }
    for (label, before, after) in [
        ("publication.expected_vs_actual", &expected, &actual),
        ("publication.proposed_vs_actual", &proposed, &actual),
        ("publication.merged_vs_actual", &merged, &actual),
    ] {
        if reserve(&EVENTS) {
            eprintln!(
                "[DEBUG-cleanup101] guard={label} fields={}",
                changed_fields(before, after)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_budget_stops_after_thirty_two_events() {
        let counter = AtomicUsize::new(0);
        for _ in 0..LIMIT {
            assert!(reserve(&counter));
        }
        assert!(!reserve(&counter));
        assert!(!reserve(&counter));
        assert_eq!(counter.load(Ordering::Relaxed), LIMIT);
    }

    #[test]
    fn field_summary_never_contains_values_or_unbounded_field_names() {
        let before = serde_json::json!({"browsers":"SYNTHETIC_PRIVATE_VALUE", "bad\nkey":1});
        let after = serde_json::json!({"browsers":"OTHER_PRIVATE_VALUE", "bad\nkey":2});
        assert_eq!(changed_fields(&before, &after), "browsers");
    }
}
