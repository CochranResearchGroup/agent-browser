//! Observe X socket credentials in the user manager's namespace when the
//! runtime cannot represent the route account. The delegated observation must
//! name the same live socket peers and process instances as the local probe.
use super::{json, linux, Value};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ENTRY: &str = "--internal-display-owner-observation";
const MAX_BYTES: u64 = 8192;

fn uid_is_identity_mapped(uid: u32, map: &str) -> bool {
    map.lines().any(|line| {
        let fields = line
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>();
        matches!(fields.as_deref(), Ok([inside, outside, count])
            if inside == outside && u64::from(uid) >= *outside
                && u64::from(uid) - outside < *count)
    })
}

pub(super) fn start_ticks(pid: i32) -> Option<String> {
    if pid <= 0 {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(19)
        .map(str::to_string)
}

pub(crate) fn run_entry(args: &[String]) -> Option<Result<(), String>> {
    if args.get(1).map(String::as_str) != Some(ENTRY) {
        return None;
    }
    Some((|| {
        let host_namespace = args.get(4).map(String::as_str) == Some("--host-namespace");
        if args.len() != 4 && !(args.len() == 5 && host_namespace) {
            return Err("display_observer_arguments_invalid".into());
        }
        let display = &args[2];
        let uid = args[3]
            .parse::<u32>()
            .map_err(|_| "display_observer_uid_invalid")?;
        let map = std::fs::read_to_string("/proc/self/uid_map")
            .map_err(|_| "display_observer_uid_map_unavailable")?;
        if host_namespace && !uid_is_identity_mapped(uid, &map) {
            return Err("display_observer_uid_unmapped".into());
        }
        let local = linux::observe(Some(display), Some(uid));
        let value = if host_namespace {
            local
        } else {
            observe_if_needed(Some(display), Some(uid), local)
        };
        let bytes = serde_json::to_string(&value).map_err(|_| "display_observer_encode_failed")?;
        if bytes.len() as u64 >= MAX_BYTES {
            return Err("display_observer_response_too_large".into());
        }
        println!("{bytes}");
        Ok(())
    })())
}

pub(super) fn observe_if_needed(
    display: Option<&str>,
    expected_uid: Option<u32>,
    mut local: Value,
) -> Value {
    let Some(uid) = expected_uid else {
        return local;
    };
    let map = std::fs::read_to_string("/proc/self/uid_map").unwrap_or_default();
    if uid_is_identity_mapped(uid, &map) {
        return local;
    }
    let result = display
        .ok_or_else(|| "display_observer_display_missing".to_string())
        .and_then(|display| observe_once(display, uid))
        .and_then(|remote| validate_observation(&local, remote));
    match result {
        Ok(mut remote) => {
            remote["observationTransport"] = json!("namespace_neutral_user_manager");
            remote["localObservedPeers"] = local["observedPeers"].clone();
            remote
        }
        Err(reason) => {
            local["verified"] = json!(false);
            local["code"] = json!("route_display_owner_unproven");
            local["nextAction"] = json!("repair_route_display_binding");
            local["namespaceObservationError"] = json!(reason);
            local
        }
    }
}

fn observe_once(display: &str, uid: u32) -> Result<Value, String> {
    if cfg!(test) {
        return Err("display_observer_transport_disabled_in_unit_tests".into());
    }
    let executable = std::env::current_exe().map_err(|_| "display_observer_executable_missing")?;
    let mut child = Command::new("/usr/bin/systemd-run")
        .args([
            "--user",
            "--quiet",
            "--wait",
            "--pipe",
            "--collect",
            "--unit",
            &format!("agent-browser-display-observe-{}", uuid::Uuid::new_v4()),
            "--property=NoNewPrivileges=true",
            "--property=PrivateTmp=false",
            "--property=PrivateUsers=false",
            "--property=RuntimeMaxSec=2s",
            "--property=TimeoutStopSec=1s",
        ])
        .arg(executable)
        .arg(ENTRY)
        .arg(display)
        .arg(uid.to_string())
        .arg("--host-namespace")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "display_observer_start_failed")?;
    let deadline = Instant::now() + Duration::from_secs(4);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("display_observer_wait_failed_or_timeout".into());
            }
        }
    };
    if !status.success() {
        return Err("display_observer_exit_failed".into());
    }
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or("display_observer_output_missing")?
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|_| "display_observer_read_failed")?;
    if bytes.len() as u64 >= MAX_BYTES {
        return Err("display_observer_response_too_large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "display_observer_response_invalid".into())
}

fn validate_observation(local: &Value, remote: Value) -> Result<Value, String> {
    if local["displayName"] != remote["displayName"]
        || local["expectedUid"] != remote["expectedUid"]
    {
        return Err("display_observer_subject_mismatch".into());
    }
    let peers = local["observedPeers"]
        .as_array()
        .filter(|p| !p.is_empty())
        .ok_or("display_observer_local_peer_missing")?;
    let observed = remote["observedPeers"]
        .as_array()
        .ok_or("display_observer_remote_peer_missing")?;
    // PrivateTmp may hide one socket spelling. Compare server instances, not
    // the number of reachable abstract/filesystem addresses for that server.
    if observed.iter().any(|after| {
        !peers.iter().any(|before| {
            before["pid"] == after["pid"]
                && before["processStartTicks"] == after["processStartTicks"]
        })
    }) {
        return Err("display_observer_peer_set_changed".into());
    }
    for before in peers {
        let after = observed
            .iter()
            .find(|after| before["pid"] == after["pid"])
            .ok_or("display_observer_peer_set_changed")?;
        let pid = before["pid"]
            .as_i64()
            .and_then(|p| i32::try_from(p).ok())
            .filter(|p| *p > 0)
            .ok_or("display_observer_pid_unproven")?;
        let start = before["processStartTicks"]
            .as_str()
            .ok_or("display_observer_start_unproven")?;
        if before["pid"] != after["pid"]
            || after["processStartTicks"].as_str() != Some(start)
            || start_ticks(pid).as_deref() != Some(start)
        {
            return Err("display_observer_process_instance_changed".into());
        }
    }
    Ok(remote)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unmapped_host_route_uid_requires_delegated_observation() {
        assert!(uid_is_identity_mapped(1010, "0 0 4294967295\n"));
        assert!(uid_is_identity_mapped(1000, "1000 1000 1\n"));
        assert!(!uid_is_identity_mapped(1010, "1000 1000 1\n"));
        assert!(!uid_is_identity_mapped(1010, "0 1010 1\n"));
        assert!(!uid_is_identity_mapped(1010, "malformed"));
    }
    #[test]
    fn delegated_display_proof_preserves_instance_and_negative_owner_result() {
        let pid = std::process::id() as i32;
        let local = json!({"displayName":":123", "expectedUid":1010,
            "observedPeers":[{"pid":pid,"uid":65534,"processStartTicks":start_ticks(pid)}]});
        let mut remote = local.clone();
        remote["observedPeers"][0]["uid"] = json!(1010);
        remote["verified"] = json!(true);
        assert_eq!(
            validate_observation(&local, remote.clone()).unwrap()["verified"],
            true
        );
        remote["verified"] = json!(false);
        assert_eq!(
            validate_observation(&local, remote.clone()).unwrap()["verified"],
            false
        );
        let duplicate = remote["observedPeers"][0].clone();
        remote["observedPeers"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(validate_observation(&local, remote.clone()).is_ok());
        remote["observedPeers"][0]["processStartTicks"] = json!("changed");
        assert!(validate_observation(&local, remote).is_err());
    }
}
