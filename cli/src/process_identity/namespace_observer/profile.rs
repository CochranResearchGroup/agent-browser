//! Exact profile database proof, without exporting descriptor targets or argv.
use super::super::{linux_process_identity, ObservedProcessIdentity, ProcessObservation};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileProof {
    identity: ObservedProcessIdentity,
    profile: PathBuf,
    matched: bool,
}

pub(super) fn run_profile_entry(args: &[String]) -> Result<(), String> {
    let pid = args
        .get(2)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|pid| *pid > 0)
        .ok_or("process_observer_pid_invalid")?;
    let profile = Path::new(&args[3]);
    if !profile.is_absolute() {
        return Err("process_observer_profile_invalid".into());
    }
    let expected = fs::canonicalize(profile).map_err(|_| "process_observer_profile_unavailable")?;
    if fs::metadata(&expected).ok().map(|m| m.uid()) != Some(unsafe { libc::geteuid() }) {
        return Err("process_observer_foreign_profile_uid".into());
    }
    if fs::metadata(format!("/proc/{pid}")).ok().map(|m| m.uid())
        != Some(unsafe { libc::geteuid() })
    {
        return Err("process_observer_foreign_uid".into());
    }
    let before = match linux_process_identity(pid, false) {
        ProcessObservation::Observed(value)
            if value.start_token.is_some()
                && value.executable_path.is_some()
                && value.browser_family.is_some() =>
        {
            value
        }
        _ => return Err("process_observer_identity_unavailable".into()),
    };
    let matched = fs::read_dir(format!("/proc/{pid}/fd"))
        .map_err(|_| "process_observer_profile_files_unavailable")?
        .flatten()
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .any(|file| {
            [
                "Default/History",
                "Default/Web Data",
                "Default/Network/Cookies",
            ]
            .iter()
            .any(|relative| file == expected.join(relative))
        });
    if linux_process_identity(pid, false) != ProcessObservation::Observed(before.clone())
        || fs::canonicalize(profile).ok().as_ref() != Some(&expected)
    {
        return Err("process_observer_identity_changed_or_unproven".into());
    }
    let mut identity = before;
    identity.command_line = None;
    let encoded = serde_json::to_string(&ProfileProof {
        identity,
        profile: expected,
        matched,
    })
    .map_err(|_| "process_observer_encode_failed")?;
    if encoded.len() as u64 >= super::MAX_BYTES {
        return Err("process_observer_response_too_large".into());
    }
    println!("{encoded}");
    Ok(())
}

pub(crate) fn observe_profile_files(
    identity: &ObservedProcessIdentity,
    profile: &Path,
) -> Result<bool, String> {
    if fs::metadata(format!("/proc/{}", identity.pid))
        .ok()
        .map(|m| m.uid())
        != Some(unsafe { libc::geteuid() })
    {
        return Err("process_observer_foreign_uid".into());
    }
    let expected = fs::canonicalize(profile).map_err(|_| "process_observer_profile_unavailable")?;
    let bytes = super::transport(identity.pid, Some(&expected))?;
    let proof: ProfileProof =
        serde_json::from_slice(&bytes).map_err(|_| "process_observer_response_invalid")?;
    let after = super::super::observe_process(identity.pid);
    validate_profile_proof(
        identity,
        &expected,
        proof,
        after,
        fs::canonicalize(profile).ok(),
    )
}

fn validate_profile_proof(
    before: &ObservedProcessIdentity,
    expected: &Path,
    proof: ProfileProof,
    after: ProcessObservation,
    after_profile: Option<PathBuf>,
) -> Result<bool, String> {
    let mut expected_identity = before.clone();
    expected_identity.command_line = None;
    let mut after_identity = match after {
        ProcessObservation::Observed(value) => value,
        _ => return Err("process_observer_identity_changed_or_unproven".into()),
    };
    after_identity.command_line = None;
    if expected_identity.start_token.is_none()
        || expected_identity.executable_path.is_none()
        || expected_identity.browser_family.is_none()
        || proof.identity != expected_identity
        || after_identity != expected_identity
        || proof.profile != expected
        || after_profile.as_deref() != Some(expected)
    {
        return Err("process_observer_identity_changed_or_unproven".into());
    }
    Ok(proof.matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ObservedProcessIdentity {
        ObservedProcessIdentity {
            pid: 42,
            start_token: Some("linux:boot:100".into()),
            executable_path: Some("/opt/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line: None,
        }
    }

    #[test]
    fn delegated_profile_proof_binds_exact_process_and_canonical_profile() {
        let before = identity();
        let expected = Path::new("/private/profile");
        for changed in [
            "none",
            "pid",
            "start",
            "executable",
            "profile",
            "after_pid",
            "after_start",
            "after_profile",
        ] {
            let mut proof = ProfileProof {
                identity: before.clone(),
                profile: expected.into(),
                matched: true,
            };
            let mut after = before.clone();
            let mut after_profile = Some(expected.to_path_buf());
            match changed {
                "pid" => proof.identity.pid = 43,
                "start" => proof.identity.start_token = Some("linux:boot:101".into()),
                "executable" => proof.identity.executable_path = Some("/different/chrome".into()),
                "profile" => proof.profile = "/private/other".into(),
                "after_pid" => after.pid = 43,
                "after_start" => after.start_token = Some("linux:other-boot:100".into()),
                "after_profile" => after_profile = Some("/private/replaced".into()),
                _ => (),
            }
            let result = validate_profile_proof(
                &before,
                expected,
                proof,
                ProcessObservation::Observed(after),
                after_profile,
            );
            assert_eq!(result.is_ok(), changed == "none", "{changed}");
        }
        let proof = ProfileProof {
            identity: before.clone(),
            profile: expected.into(),
            matched: false,
        };
        let serialized = serde_json::to_string(&proof).unwrap();
        assert!(!serialized.contains("Default/History"));
        assert!(!serialized.contains("--user-data-dir"));
        assert_eq!(
            validate_profile_proof(
                &before,
                expected,
                proof,
                ProcessObservation::Observed(before.clone()),
                Some(expected.into())
            ),
            Ok(false)
        );
    }
}
