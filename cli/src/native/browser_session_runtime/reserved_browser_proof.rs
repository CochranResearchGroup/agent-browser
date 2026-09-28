use agent_browser_service_model::{
    BrowserDesktopAssignment, BrowserLaunch, BrowserProfileCatalogEntry, RecordedProcessIdentity,
};

use crate::process_identity::{ObservedProcessIdentity, ProcessObservation};

pub(super) struct ReservedBrowserObservations<'a> {
    pub before: &'a ProcessObservation,
    pub after: &'a ProcessObservation,
}

pub(super) fn prove_reserved_browser(
    profile: &BrowserProfileCatalogEntry,
    desktop: &BrowserDesktopAssignment,
    browser_id: &str,
    endpoint: &str,
    browser_pid: u32,
    observations: ReservedBrowserObservations<'_>,
) -> Result<BrowserLaunch, String> {
    if browser_id.is_empty()
        || endpoint.is_empty()
        || browser_pid == 0
        || profile.user_data_dir.is_empty()
        || desktop.route_id.is_empty()
        || desktop.display_name.is_empty()
    {
        return Err("reserved_browser_proof_input_invalid".to_string());
    }
    let before = observed(observations.before)?;
    let after = observed(observations.after)?;
    if before.pid != browser_pid || after.pid != browser_pid {
        return Err("reserved_browser_proof_pid_mismatch".to_string());
    }
    let start_token = required_equal(
        before.start_token.as_deref(),
        after.start_token.as_deref(),
        "reserved_browser_proof_start_token_missing",
        "reserved_browser_proof_process_reused",
    )?;
    let executable_path = required_equal(
        before.executable_path.as_deref(),
        after.executable_path.as_deref(),
        "reserved_browser_proof_executable_missing",
        "reserved_browser_proof_identity_changed",
    )?;
    let browser_family = required_equal(
        before.browser_family.as_deref(),
        after.browser_family.as_deref(),
        "reserved_browser_proof_browser_family_missing",
        "reserved_browser_proof_identity_changed",
    )?;
    if browser_family != "chrome" {
        return Err("reserved_browser_proof_browser_family_invalid".to_string());
    }
    let before_argv = before
        .command_line
        .as_deref()
        .ok_or_else(|| "reserved_browser_proof_argv_missing".to_string())?;
    let after_argv = after
        .command_line
        .as_deref()
        .ok_or_else(|| "reserved_browser_proof_argv_missing".to_string())?;
    if before_argv != after_argv {
        return Err("reserved_browser_proof_identity_changed".to_string());
    }
    if before_argv.first().map(String::as_str) != Some(executable_path) {
        return Err("reserved_browser_proof_executable_mismatch".to_string());
    }
    if before_argv
        .iter()
        .any(|argument| argument == "--type" || argument.starts_with("--type="))
    {
        return Err("reserved_browser_proof_renderer_process".to_string());
    }
    require_exact_flag(before_argv, "--agent-browser-reservation-id", browser_id)?;
    require_exact_flag(before_argv, "--user-data-dir", &profile.user_data_dir)?;

    Ok(BrowserLaunch {
        browser_id: browser_id.to_string(),
        pid: browser_pid,
        cdp_endpoint: endpoint.to_string(),
        process_identity: Some(RecordedProcessIdentity {
            pid: browser_pid,
            start_token: start_token.to_string(),
            executable_path: Some(executable_path.to_string()),
            browser_family: Some(browser_family.to_string()),
        }),
        desktop: Some(desktop.clone()),
    })
}

fn observed(observation: &ProcessObservation) -> Result<&ObservedProcessIdentity, String> {
    match observation {
        ProcessObservation::Observed(identity) => Ok(identity),
        ProcessObservation::Missing => Err("reserved_browser_proof_process_missing".to_string()),
        ProcessObservation::Failed { .. } => {
            Err("reserved_browser_proof_observation_failed".to_string())
        }
    }
}

fn required_equal<'a>(
    before: Option<&'a str>,
    after: Option<&'a str>,
    missing_error: &str,
    mismatch_error: &str,
) -> Result<&'a str, String> {
    let before = before
        .filter(|value| !value.is_empty())
        .ok_or_else(|| missing_error.to_string())?;
    let after = after
        .filter(|value| !value.is_empty())
        .ok_or_else(|| missing_error.to_string())?;
    if before != after {
        return Err(mismatch_error.to_string());
    }
    Ok(before)
}

fn require_exact_flag(argv: &[String], flag: &str, expected: &str) -> Result<(), String> {
    let mut value = None;
    let mut index = 0;
    while index < argv.len() {
        let argument = &argv[index];
        let candidate = if argument == flag {
            index += 1;
            argv.get(index)
                .filter(|value| !value.is_empty())
                .map(String::as_str)
        } else if let Some(candidate) = argument.strip_prefix(&format!("{flag}=")) {
            (!candidate.is_empty()).then_some(candidate)
        } else {
            if argument.starts_with(flag) {
                return Err("reserved_browser_proof_flag_malformed".to_string());
            }
            index += 1;
            continue;
        };
        let candidate =
            candidate.ok_or_else(|| "reserved_browser_proof_flag_malformed".to_string())?;
        if value.replace(candidate).is_some() {
            return Err("reserved_browser_proof_flag_ambiguous".to_string());
        }
        index += 1;
    }
    if value != Some(expected) {
        return Err("reserved_browser_proof_flag_mismatch".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use agent_browser_service_model::BrowserProfileKind;

    use super::*;

    fn profile(path: &str) -> BrowserProfileCatalogEntry {
        BrowserProfileCatalogEntry {
            id: "profile".to_string(),
            name: "Profile".to_string(),
            user_data_dir: path.to_string(),
            kind: BrowserProfileKind::Named,
        }
    }

    fn desktop() -> BrowserDesktopAssignment {
        BrowserDesktopAssignment {
            route_id: "route-slot-01".to_string(),
            display_name: ":10".to_string(),
            live_browser_count: 0,
        }
    }

    fn observation(pid: u32, start: &str, executable: &str, argv: Vec<&str>) -> ProcessObservation {
        ProcessObservation::Observed(ObservedProcessIdentity {
            pid,
            start_token: Some(start.to_string()),
            executable_path: Some(executable.to_string()),
            browser_family: Some("chrome".to_string()),
            command_line: Some(argv.into_iter().map(str::to_string).collect()),
        })
    }

    fn prove(
        profile_path: &str,
        observation: &ProcessObservation,
    ) -> Result<BrowserLaunch, String> {
        prove_reserved_browser(
            &profile(profile_path),
            &desktop(),
            "reserved-browser",
            "ws://127.0.0.1/devtools/browser/exact",
            42,
            ReservedBrowserObservations {
                before: observation,
                after: observation,
            },
        )
    }

    #[test]
    fn accepts_linux_macos_and_windows_argv_without_flattened_parsing() {
        for (executable, profile_path, argv) in [
            (
                "/opt/google/chrome",
                "/tmp/profile",
                vec![
                    "/opt/google/chrome",
                    "--agent-browser-reservation-id=reserved-browser",
                    "--user-data-dir=/tmp/profile",
                ],
            ),
            (
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Users/test/Library/Profile",
                vec![
                    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                    "--agent-browser-reservation-id",
                    "reserved-browser",
                    "--user-data-dir",
                    "/Users/test/Library/Profile",
                ],
            ),
            (
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Users\test\Profile",
                vec![
                    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                    "--agent-browser-reservation-id=reserved-browser",
                    r"--user-data-dir=C:\Users\test\Profile",
                ],
            ),
        ] {
            let observed = observation(42, "start-1", executable, argv);
            let launch = prove(profile_path, &observed).unwrap();
            assert_eq!(launch.pid, 42);
            assert_eq!(launch.desktop, Some(desktop()));
        }
    }

    #[test]
    fn rejects_mismatched_ambiguous_and_renderer_flags() {
        for argv in [
            vec![
                "/opt/chrome",
                "--agent-browser-reservation-id=other",
                "--user-data-dir=/tmp/profile",
            ],
            vec![
                "/opt/chrome",
                "--agent-browser-reservation-id=reserved-browser",
                "--agent-browser-reservation-id=reserved-browser",
                "--user-data-dir=/tmp/profile",
            ],
            vec![
                "/opt/chrome",
                "--type=renderer",
                "--agent-browser-reservation-id=reserved-browser",
                "--user-data-dir=/tmp/profile",
            ],
        ] {
            assert!(prove(
                "/tmp/profile",
                &observation(42, "start-1", "/opt/chrome", argv)
            )
            .is_err());
        }
    }

    #[test]
    fn rejects_pid_reuse_and_missing_identity() {
        let before = observation(
            42,
            "start-1",
            "/opt/chrome",
            vec![
                "/opt/chrome",
                "--agent-browser-reservation-id=reserved-browser",
                "--user-data-dir=/tmp/profile",
            ],
        );
        let after = observation(
            42,
            "start-2",
            "/opt/chrome",
            vec![
                "/opt/chrome",
                "--agent-browser-reservation-id=reserved-browser",
                "--user-data-dir=/tmp/profile",
            ],
        );
        assert_eq!(
            prove_reserved_browser(
                &profile("/tmp/profile"),
                &desktop(),
                "reserved-browser",
                "ws://127.0.0.1/devtools/browser/exact",
                42,
                ReservedBrowserObservations {
                    before: &before,
                    after: &after,
                },
            ),
            Err("reserved_browser_proof_process_reused".to_string())
        );
        assert!(prove_reserved_browser(
            &profile("/tmp/profile"),
            &desktop(),
            "reserved-browser",
            "ws://127.0.0.1/devtools/browser/exact",
            42,
            ReservedBrowserObservations {
                before: &ProcessObservation::Missing,
                after: &after,
            },
        )
        .is_err());
    }
}
