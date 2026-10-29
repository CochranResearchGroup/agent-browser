//! Fresh identity-only batch observation. Arguments remain in the caller's
//! local process census; no profile data or command lines cross this transport.
use super::super::{linux_process_identity, ProcessObservation};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_PIDS: usize = 8192;
const MAX_BYTES: u64 = 4 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(4);

type Census = Vec<(u32, ProcessObservation)>;

fn requested(pids: &[u32]) -> Result<(), String> {
    if pids.is_empty()
        || pids.len() > MAX_PIDS
        || pids.contains(&0)
        || pids.iter().copied().collect::<BTreeSet<_>>().len() != pids.len()
    {
        return Err("process_observer_census_request_invalid".into());
    }
    Ok(())
}

fn local_observation(pid: u32) -> ProcessObservation {
    let mut observation = linux_process_identity(pid, false);
    if let ProcessObservation::Observed(value) = &mut observation {
        value.command_line = None;
    }
    observation
}

pub(super) fn run_entry() -> Result<(), String> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "process_observer_census_input_failed")?;
    if bytes.len() > 128 * 1024 {
        return Err("process_observer_census_input_too_large".into());
    }
    let pids: Vec<u32> =
        serde_json::from_slice(&bytes).map_err(|_| "process_observer_census_request_invalid")?;
    requested(&pids)?;
    let observations = pids
        .into_iter()
        .map(|pid| {
            let observation = match std::fs::metadata(format!("/proc/{pid}")) {
                Ok(metadata) if metadata.uid() != unsafe { libc::geteuid() } => {
                    ProcessObservation::Failed {
                        reason: "process_observer_foreign_uid".into(),
                    }
                }
                _ => local_observation(pid),
            };
            (pid, observation)
        })
        .collect::<Census>();
    let encoded =
        serde_json::to_vec(&observations).map_err(|_| "process_observer_census_encode_failed")?;
    if encoded.len() as u64 > MAX_BYTES {
        return Err("process_observer_census_output_too_large".into());
    }
    std::io::stdout()
        .write_all(&encoded)
        .map_err(|_| "process_observer_census_output_failed".into())
}

fn start(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    if super::super::linux_process_state(&stat) == Some("Z") {
        return None;
    }
    let ticks = super::super::linux_start_ticks(&stat)?;
    Some(format!("{}:{ticks}", super::super::current_boot_epoch()?))
}

fn validate(
    expected: &BTreeMap<u32, String>,
    observations: Census,
    mut current_start: impl FnMut(u32) -> Option<String>,
) -> Result<BTreeMap<u32, ProcessObservation>, String> {
    if observations.len() != expected.len() {
        return Err("process_observer_census_response_incomplete".into());
    }
    let mut result = BTreeMap::new();
    for (pid, observation) in observations {
        let before = expected
            .get(&pid)
            .ok_or("process_observer_census_response_unexpected_pid")?;
        if result.contains_key(&pid) {
            return Err("process_observer_census_response_duplicate_pid".into());
        }
        let after = current_start(pid);
        let observation = match observation {
            ProcessObservation::Observed(value)
                if value.pid == pid
                    && value.start_token.as_ref() == Some(before)
                    && after.as_ref() == Some(before)
                    && value.command_line.is_none()
                    && value
                        .executable_path
                        .as_ref()
                        .is_some_and(|p| std::path::Path::new(p).is_absolute()) =>
            {
                ProcessObservation::Observed(value)
            }
            ProcessObservation::Missing if after.is_none() => ProcessObservation::Missing,
            ProcessObservation::Failed { reason } => ProcessObservation::Failed { reason },
            _ => return Err("process_observer_identity_changed_or_unproven".into()),
        };
        result.insert(pid, observation);
    }
    Ok(result)
}

/// One fresh bounded observation for the complete requested set. A malformed or
/// incomplete transport cannot establish absence or grant a cleanup exemption.
pub(crate) fn observe(
    expected: &BTreeMap<u32, String>,
) -> Result<BTreeMap<u32, ProcessObservation>, String> {
    requested(&expected.keys().copied().collect::<Vec<_>>())?;
    let mut observations = Vec::new();
    let mut delegated = BTreeMap::new();
    for (&pid, before) in expected {
        let observation = local_observation(pid);
        if matches!(&observation, ProcessObservation::Failed { reason } if reason == "linux_proc_exe_permission_denied")
            && std::fs::metadata(format!("/proc/{pid}"))
                .ok()
                .map(|m| m.uid())
                == Some(unsafe { libc::geteuid() })
        {
            delegated.insert(pid, before.clone());
        } else {
            observations.push((pid, observation));
        }
    }
    if !delegated.is_empty() {
        let bytes = transport(&delegated.keys().copied().collect::<Vec<_>>())?;
        let remote: Census = serde_json::from_slice(&bytes)
            .map_err(|_| "process_observer_census_response_invalid")?;
        observations.extend(validate(&delegated, remote, start)?);
    }
    validate(expected, observations, start)
}

struct ObserverChild(std::process::Child);
impl Drop for ObserverChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn transport(pids: &[u32]) -> Result<Vec<u8>, String> {
    if cfg!(test) {
        return Err("process_observer_transport_disabled_in_unit_tests".into());
    }
    requested(pids)?;
    let executable =
        std::env::current_exe().map_err(|_| "process_observer_executable_unavailable")?;
    let unit = format!("agent-browser-process-observe-{}", uuid::Uuid::new_v4());
    let child = Command::new("/usr/bin/systemd-run")
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
        .args([super::ENTRY, "census"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("process_observer_start_failed: {e}"))?;
    let input = serde_json::to_vec(pids).map_err(|_| "process_observer_census_encode_failed")?;
    exchange(child, input, DEADLINE)
}

fn exchange(
    child: std::process::Child,
    input: Vec<u8>,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let deadline = Instant::now() + timeout;
    let mut child = ObserverChild(child);
    let mut stdin = child
        .0
        .stdin
        .take()
        .ok_or("process_observer_input_missing")?;
    let stdout = child
        .0
        .stdout
        .take()
        .ok_or("process_observer_output_missing")?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    // Drain while the child runs: a complete census can exceed pipe capacity.
    let reader = std::thread::spawn(move || {
        let result = (|| {
            stdin
                .write_all(&input)
                .map_err(|_| "process_observer_census_input_failed")?;
            drop(stdin);
            let mut bytes = Vec::new();
            stdout
                .take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "process_observer_read_failed")?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("process_observer_census_output_too_large");
            }
            Ok(bytes)
        })();
        let _ = sender.send(result);
    });
    let result = loop {
        match receiver.try_recv() {
            Ok(result) => break result.map_err(str::to_owned),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                break Err("process_observer_read_failed".into())
            }
            _ if Instant::now() >= deadline => break Err("process_observer_timeout".into()),
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    if result.is_err() {
        let _ = child.0.kill();
        let _ = child.0.wait();
        // The helper's RuntimeMaxSec bounds inherited pipes after runner kill.
        return result;
    }
    let _ = reader.join();
    loop {
        match child.0.try_wait() {
            Ok(Some(status)) if status.success() => return result,
            Ok(Some(status)) => return Err(format!("process_observer_exit_failed: {status}")),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.0.kill();
                let _ = child.0.wait();
                return Err("process_observer_timeout".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_identity::ObservedProcessIdentity;

    fn identity(pid: u32) -> ProcessObservation {
        ProcessObservation::Observed(ObservedProcessIdentity {
            pid,
            start_token: Some("linux:boot:100".into()),
            executable_path: Some("/opt/chrome".into()),
            browser_family: Some("chrome".into()),
            command_line: None,
        })
    }

    #[test]
    fn cleanup_census_requires_complete_exact_fresh_identity_only_response() {
        let expected = BTreeMap::from([(7, "linux:boot:100".into()), (8, "linux:boot:100".into())]);
        for (case, response) in [
            ("complete", vec![(7, identity(7)), (8, identity(8))]),
            ("missing", vec![(7, identity(7))]),
            ("duplicate", vec![(7, identity(7)), (7, identity(7))]),
            ("extra", vec![(7, identity(7)), (9, identity(9))]),
            ("wrong_identity", vec![(7, identity(8)), (8, identity(8))]),
        ] {
            assert_eq!(
                validate(&expected, response, |_| Some("linux:boot:100".into())).is_ok(),
                case == "complete",
                "{case}"
            );
        }
        for after in [
            None,
            Some("linux:boot:101".into()),
            Some("linux:other:100".into()),
        ] {
            assert!(
                validate(&expected, vec![(7, identity(7)), (8, identity(8))], |_| {
                    after.clone()
                })
                .is_err()
            );
        }
        let denied = ProcessObservation::Failed {
            reason: "denied".into(),
        };
        let result = validate(
            &expected,
            vec![(7, denied.clone()), (8, ProcessObservation::Missing)],
            |pid| (pid == 7).then(|| "linux:boot:100".into()),
        )
        .unwrap();
        assert_eq!(result[&7], denied);
        assert_eq!(result[&8], ProcessObservation::Missing);
        let mut private = identity(7);
        if let ProcessObservation::Observed(value) = &mut private {
            value.command_line = Some(vec!["private".into()]);
        }
        assert!(
            validate(&expected, vec![(7, private), (8, identity(8))], |_| Some(
                "linux:boot:100".into()
            ))
            .is_err()
        );
        for pids in [vec![], vec![0], vec![7, 7], vec![7; MAX_PIDS + 1]] {
            assert!(requested(&pids).is_err());
        }
    }

    #[test]
    fn observer_exchange_drains_large_output_and_reaps_failed_owned_child() {
        let spawn = |script: &str| {
            Command::new("/bin/sh")
                .args(["-c", script])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        };
        let bytes = exchange(
            spawn("cat >/dev/null; head -c 131072 /dev/zero"),
            b"request".to_vec(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(bytes.len(), 131072);
        let child = spawn("exec cat");
        let pid = child.id();
        let bytes = exchange(child, b"fresh".to_vec(), Duration::from_secs(2)).unwrap();
        assert_eq!(bytes, b"fresh");
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        let child = spawn("exec sleep 30");
        let pid = child.id();
        assert!(exchange(child, vec![], Duration::from_millis(100))
            .unwrap_err()
            .contains("timeout"));
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        let child = spawn("cat >/dev/null; head -c 4194305 /dev/zero");
        assert!(exchange(child, vec![], Duration::from_secs(2))
            .unwrap_err()
            .contains("too_large"));
    }

    #[test]
    fn fresh_cleanup_census_observes_local_process_without_namespace_services() {
        let pid = std::process::id();
        let before = start(pid).unwrap();
        let values = observe(&BTreeMap::from([(pid, before.clone())])).unwrap();
        assert!(matches!(&values[&pid], ProcessObservation::Observed(value)
            if value.start_token.as_ref() == Some(&before) && value.command_line.is_none()));
        assert!(observe(&BTreeMap::from([(pid, "linux:wrong-boot:100".into())])).is_err());
        assert!(transport(&[pid])
            .unwrap_err()
            .contains("disabled_in_unit_tests"));
    }
}
