//! Read-only executable observation across sibling user namespaces. PrivateTmp
//! runtimes cannot inspect retained browsers in another generation's namespace.
//! The user manager starts the same executable without namespace isolation for
//! this one read; the runtime's own isolation and ownership predicates stay intact.

use super::{linux_process_identity, ProcessObservation};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ENTRY: &str = "--internal-process-observation";
const MAX_BYTES: u64 = 8192;

pub(super) fn run_entry(args: &[String]) -> Option<Result<(), String>> {
    if args.get(1).map(String::as_str) != Some(ENTRY) {
        return None;
    }
    Some((|| {
        let pid = args
            .get(2)
            .filter(|_| args.len() == 3)
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|pid| *pid > 0)
            .ok_or("process_observer_pid_invalid")?;
        let mut observation = linux_process_identity(pid, false);
        // Command lines may contain private URLs or credentials. This transport
        // needs executable and start identity only, and never exports arguments.
        if let ProcessObservation::Observed(value) = &mut observation {
            value.command_line = None;
        }
        let encoded =
            serde_json::to_string(&observation).map_err(|_| "process_observer_encode_failed")?;
        if encoded.len() as u64 >= MAX_BYTES {
            return Err("process_observer_response_too_large".to_string());
        }
        println!("{encoded}");
        Ok(())
    })())
}

pub(super) fn observe(pid: u32, before_start: Option<&str>) -> ProcessObservation {
    let result = observe_once(pid, before_start).map(|mut observation| {
        if let ProcessObservation::Observed(value) = &mut observation {
            value.command_line = std::fs::read(format!("/proc/{pid}/cmdline"))
                .ok()
                .map(|bytes| {
                    bytes
                        .split(|byte| *byte == 0)
                        .filter(|value| !value.is_empty())
                        .map(|value| String::from_utf8_lossy(value).into_owned())
                        .collect()
                });
        }
        observation
    });
    result.unwrap_or_else(|reason| ProcessObservation::Failed {
        reason: format!("linux_proc_exe_permission_denied; {reason}"),
    })
}

fn observe_once(pid: u32, before_start: Option<&str>) -> Result<ProcessObservation, String> {
    use std::os::unix::fs::MetadataExt;
    if std::fs::metadata(format!("/proc/{pid}"))
        .ok()
        .map(|metadata| metadata.uid())
        != Some(unsafe { libc::geteuid() })
    {
        return Err("process_observer_foreign_uid".into());
    }
    if before_start.is_none() {
        return Err("process_observer_start_identity_missing".into());
    }
    // Unit tests must never create host services implicitly. The opt-in installed
    // namespace smoke exercises this transport with the real executable.
    if cfg!(test) {
        return Err("process_observer_transport_disabled_in_unit_tests".into());
    }
    let executable =
        std::env::current_exe().map_err(|_| "process_observer_executable_unavailable")?;
    let unit = format!("agent-browser-process-observe-{}", uuid::Uuid::new_v4());
    let mut child = Command::new("/usr/bin/systemd-run")
        .args([
            "--user",
            "--quiet",
            "--wait",
            "--pipe",
            "--collect",
            "--unit",
            &unit,
            "--property=NoNewPrivileges=true",
            "--property=PrivateTmp=false",
            "--property=RuntimeMaxSec=2s",
            "--property=TimeoutStopSec=1s",
        ])
        .arg(executable)
        .arg(ENTRY)
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("process_observer_start_failed: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(4);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(if result.is_err() {
                    "process_observer_wait_failed"
                } else {
                    "process_observer_timeout"
                }
                .into());
            }
        }
    };
    if !status.success() {
        return Err(format!("process_observer_exit_failed: {status}"));
    }
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or("process_observer_output_missing")?
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|_| "process_observer_read_failed")?;
    if bytes.len() as u64 >= MAX_BYTES {
        return Err("process_observer_response_too_large".into());
    }
    let observation: ProcessObservation =
        serde_json::from_slice(&bytes).map_err(|_| "process_observer_response_invalid")?;
    // Re-read local process identity without invoking another helper. Even when
    // exe remains denied, boot/start are separately available through proc stat.
    let after_start = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| super::linux_start_ticks(&stat).map(ToString::to_string))
        .zip(super::current_boot_epoch())
        .map(|(ticks, boot)| format!("{boot}:{ticks}"));
    validate_observation(pid, before_start, after_start.as_deref(), observation)
}

fn validate_observation(
    pid: u32,
    before: Option<&str>,
    after: Option<&str>,
    observation: ProcessObservation,
) -> Result<ProcessObservation, String> {
    match &observation {
        ProcessObservation::Observed(value)
            if value.pid == pid
                && before.is_some()
                && before == after
                && value.start_token.as_deref() == before
                && value
                    .executable_path
                    .as_ref()
                    .is_some_and(|path| std::path::Path::new(path).is_absolute()) =>
        {
            Ok(observation)
        }
        ProcessObservation::Failed { reason } => {
            Err(format!("process_observer_observation_failed: {reason}"))
        }
        _ => Err("process_observer_identity_changed_or_unproven".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_identity::ObservedProcessIdentity;

    #[test]
    fn delegated_observation_requires_exact_before_after_process_instance() {
        let observation = ProcessObservation::Observed(ObservedProcessIdentity {
            pid: 42,
            start_token: Some("linux:boot:123".into()),
            executable_path: Some("/opt/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line: None,
        });
        assert!(validate_observation(
            42,
            Some("linux:boot:123"),
            Some("linux:boot:123"),
            observation.clone()
        )
        .is_ok());
        for (pid, before, after) in [
            (43, Some("linux:boot:123"), Some("linux:boot:123")),
            (42, None, None),
            (42, Some("linux:boot:123"), Some("linux:boot:124")),
            (42, Some("linux:other:123"), Some("linux:other:123")),
        ] {
            assert!(validate_observation(pid, before, after, observation.clone()).is_err());
        }
        assert!(validate_observation(
            42,
            Some("linux:boot:123"),
            None,
            ProcessObservation::Missing
        )
        .is_err());
    }
}
