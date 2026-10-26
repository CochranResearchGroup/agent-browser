//! Read-only mode evidence. Complete argv and immutable original process identity
//! are required; a missing debugger endpoint is never negative CDP evidence.
use crate::process_identity::{ProcessObservation, RecordedProcessIdentity};
use agent_browser_service_model::ManagedBrowserInstance;
use serde_json::{json, Value};

pub(crate) fn inspect(
    browser: &ManagedBrowserInstance,
    original: Option<&RecordedProcessIdentity>,
    ownership: &str,
    mut observe: impl FnMut(u32) -> ProcessObservation,
) -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_millis() as u64)
        .unwrap_or_default();
    let mut result = json!({
        "schemaVersion":"agent-browser.browser-mode.v1", "browserId":browser.id,
        "profileId":browser.profile_id, "pid":browser.pid, "mode":"unverified",
        "observedAtMs":now, "ownership":ownership, "reason":"original_identity_unavailable",
        "executable":{"name":null,"family":null,"version":null},
        "capabilities":{"debugging":"unverified","nativeObservation":"unverified","nativeInput":"unverified"}
    });
    let Some(original) = original else {
        return result;
    };
    let before = observe(browser.pid);
    let ProcessObservation::Observed(process) = &before else {
        result["reason"] = json!("process_unavailable");
        return result;
    };
    if original.pid != browser.pid
        || process.pid != browser.pid
        || process.start_token.as_deref() != Some(original.start_token.as_str())
        || original.start_token.is_empty()
        || process.executable_path.is_none()
        || process.executable_path != original.executable_path
        || !matches!(
            process.browser_family.as_deref(),
            Some("chrome" | "chromium")
        )
        || process.browser_family != original.browser_family
    {
        result["reason"] = json!("process_identity_changed");
        return result;
    }
    let Some(args) = process
        .command_line
        .as_ref()
        .filter(|args| !args.is_empty())
    else {
        result["reason"] = json!("command_line_unavailable");
        return result;
    };
    // Chrome can collapse its proc argv into a single space-delimited entry
    // after startup. Normalize that shape only; ordinary argv entries may
    // contain spaces inside a URL or value and must retain their boundaries.
    let args: Vec<&str> = if args.len() == 1 {
        args[0].split_ascii_whitespace().collect()
    } else {
        args.iter().map(String::as_str).collect()
    };
    if args
        .iter()
        .copied()
        .any(|arg| arg == "--type" || arg.starts_with("--type="))
    {
        result["reason"] = json!("not_browser_root");
        return result;
    }
    if observe(browser.pid) != before {
        result["reason"] = json!("process_changed_during_observation");
        return result;
    }
    result["executable"]["name"] = json!(process
        .executable_path
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .map(|name| name.to_string_lossy().into_owned()));
    result["executable"]["family"] = json!(process.browser_family);
    let debugging = args.iter().copied().any(|arg| {
        ["--remote-debugging-port", "--remote-debugging-pipe"]
            .iter()
            .copied()
            .any(|flag| {
                arg == flag
                    || arg
                        .strip_prefix(flag)
                        .is_some_and(|rest| rest.starts_with('='))
            })
    });
    if debugging {
        result["mode"] = json!("cdp");
        result["capabilities"]["debugging"] = json!("enabled");
        result["reason"] = Value::Null;
    } else if browser.cdp_endpoint.is_empty() {
        result["mode"] = json!("cdp_free");
        result["capabilities"]["debugging"] = json!("disabled");
        result["reason"] = Value::Null;
    } else {
        result["reason"] = json!("retained_debugger_endpoint");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_identity::ObservedProcessIdentity;
    fn browser(endpoint: &str) -> ManagedBrowserInstance {
        ManagedBrowserInstance {
            id: "browser".into(),
            profile_id: "profile".into(),
            pid: 42,
            cdp_endpoint: endpoint.into(),
            desktop: None,
            active_session_ids: vec![],
        }
    }
    fn original() -> RecordedProcessIdentity {
        RecordedProcessIdentity {
            pid: 42,
            start_token: "boot:original".into(),
            executable_path: Some("/opt/chrome/chrome".into()),
            browser_family: Some("chrome".into()),
        }
    }
    fn observation(args: &[&str]) -> ProcessObservation {
        ProcessObservation::Observed(ObservedProcessIdentity {
            pid: 42,
            start_token: Some("boot:original".into()),
            executable_path: Some("/opt/chrome/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line: Some(args.iter().map(|s| s.to_string()).collect()),
        })
    }
    #[test]
    fn browser_mode_inspection_proves_current_original_process_debugging_posture() {
        let observed = observation(&[
            "/opt/chrome/chrome",
            "--remote-debugging-port=0",
            "https://private.test/?secret=CANARY",
        ]);
        let mode = inspect(
            &browser("ws://127.0.0.1:1234"),
            Some(&original()),
            "managed",
            |_| observed.clone(),
        );
        assert_eq!(mode["mode"], "cdp");
        assert_eq!(mode["browserId"], "browser");
        assert_eq!(mode["profileId"], "profile");
        assert_eq!(mode["executable"]["name"], "chrome");
        assert_eq!(mode["capabilities"]["nativeInput"], "unverified");
        assert!(!mode.to_string().contains("CANARY"));
        assert!(!mode.to_string().contains("/opt/"));
    }
    #[test]
    fn browser_mode_inspection_recognizes_collapsed_original_browser_arguments() {
        // The owned CfT fixture exposes one NUL-delimited proc argument after
        // startup, while retaining the full space-delimited browser command.
        let observed = observation(&[
            "/opt/chrome/chrome --remote-debugging-port=0 --user-data-dir=/private/profile",
        ]);
        let mode = inspect(
            &browser("ws://127.0.0.1:1234"),
            Some(&original()),
            "app_provided",
            |_| observed.clone(),
        );
        assert_eq!(mode["mode"], "cdp");
        assert_eq!(mode["capabilities"]["debugging"], "enabled");
        assert!(!mode.to_string().contains("/private/"));
        let child = observation(&["/opt/chrome/chrome --type=renderer"]);
        assert_eq!(
            inspect(&browser(""), Some(&original()), "app_provided", |_| child
                .clone())["mode"],
            "unverified",
            "collapsed renderer arguments cannot prove a browser root"
        );
        let ordinary = observation(&[
            "/opt/chrome/chrome",
            "https://example.test/ --remote-debugging-port=0",
        ]);
        assert_eq!(
            inspect(&browser(""), Some(&original()), "managed", |_| ordinary
                .clone())["mode"],
            "cdp_free",
            "spaces inside an ordinary URL argument are not switch boundaries"
        );
    }

    #[test]
    fn browser_mode_inspection_no_cdp_requires_complete_original_browser_evidence() {
        let observed = observation(&[
            "/opt/chrome/chrome",
            "--user-data-dir=/private/profile",
            "https://example.test",
        ]);
        let mode = inspect(&browser(""), Some(&original()), "managed", |_| {
            observed.clone()
        });
        assert_eq!(mode["mode"], "cdp_free");
        assert_eq!(mode["capabilities"]["debugging"], "disabled");
        assert_eq!(mode["reason"], Value::Null);
        let missing_original = inspect(&browser(""), None, "managed", |_| observed.clone());
        assert_eq!(
            missing_original["mode"], "unverified",
            "missing debugger metadata is not proof"
        );
        let contradictory = inspect(
            &browser("ws://127.0.0.1:1"),
            Some(&original()),
            "managed",
            |_| observed.clone(),
        );
        assert_eq!(
            contradictory["mode"], "unverified",
            "retained attachment cannot be reported no-CDP"
        );
    }
    #[test]
    fn browser_mode_inspection_child_or_replaced_process_is_not_no_cdp_proof() {
        let child = observation(&["/opt/chrome/chrome", "--type=renderer"]);
        assert_eq!(
            inspect(&browser(""), Some(&original()), "managed", |_| child
                .clone())["mode"],
            "unverified"
        );
        let mut replacement = original();
        replacement.start_token = "boot:replacement".into();
        let current = observation(&["/opt/chrome/chrome", "--remote-debugging-pipe"]);
        assert_eq!(
            inspect(&browser(""), Some(&replacement), "managed", |_| current
                .clone())["mode"],
            "unverified"
        );
        let mut reads = 0;
        let changed = inspect(&browser(""), Some(&original()), "managed", |_| {
            reads += 1;
            if reads == 1 {
                current.clone()
            } else {
                ProcessObservation::Missing
            }
        });
        assert_eq!(changed["mode"], "unverified");
    }
}
