//! Local operator maintenance for a legacy transport proven older than every
//! current producer of the same Service State. Not a consumer service action.

use super::service_model::ServiceState;
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Plan {
    schema_version: String,
    state_path: PathBuf,
    custody_snapshot_path: PathBuf,
    custody_snapshot_sha256: String,
    connection_id: String,
    expected_tabs: BTreeMap<String, Value>,
}

fn custody(tab: &super::service_model::BrowserTab) -> Value {
    json!({"browserId":tab.browser_id,"targetId":tab.target_id,
        "ownerSessionId":tab.owner_session_id,"profileAccess":tab.profile_access})
}

fn validate_custody(state: &ServiceState, plan: &Plan) -> Result<(), String> {
    if plan.connection_id.starts_with("connection-v2-")
        || !plan.connection_id.starts_with("connection-")
        || plan.expected_tabs.is_empty()
    {
        return Err("legacy_connection_plan_invalid".into());
    }
    let actual = state
        .tabs
        .iter()
        .filter(|(_, tab)| {
            tab.profile_access.as_ref().is_some_and(|access| {
                access.connection_instance_id.as_deref() == Some(&plan.connection_id)
            })
        })
        .map(|(id, tab)| (id.clone(), custody(tab)))
        .collect::<BTreeMap<_, _>>();
    if actual != plan.expected_tabs {
        return Err("legacy_connection_custody_changed".into());
    }
    if actual
        .values()
        .any(|tab| tab["profileAccess"]["connectionState"] != "active")
    {
        return Err("legacy_connection_not_active".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn producer_census(snapshot_ms: u128) -> Result<Value, String> {
    use std::{fs, os::unix::fs::MetadataExt, process::Command, time::UNIX_EPOCH};
    let home = dirs::home_dir().ok_or("legacy_connection_home_unknown")?;
    let home = fs::canonicalize(home).map_err(|e| e.to_string())?;
    let mut producers = Vec::new();
    for entry in fs::read_dir("/proc").map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if pid == std::process::id() {
            continue;
        }
        let proc = entry.path();
        let Ok(metadata) = fs::metadata(&proc) else {
            continue;
        };
        if metadata.uid() != unsafe { libc::geteuid() } {
            continue;
        }
        let executable = match fs::read_link(proc.join("exe")) {
            Ok(path) => path,
            Err(_)
                if fs::read_to_string(proc.join("comm"))
                    .is_ok_and(|name| name.trim() == "agent-browser") =>
            {
                return Err("legacy_connection_producer_unreadable".into())
            }
            Err(_) => continue,
        };
        if !executable
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("agent-browser"))
        {
            continue;
        }
        // MCP relays open sockets; the receiving host mints the custody ID.
        let arguments = fs::read(proc.join("cmdline"))
            .map_err(|_| "legacy_connection_producer_arguments_unknown")?;
        let arguments = arguments.split(|byte| *byte == 0).collect::<Vec<_>>();
        if arguments
            .windows(2)
            .any(|pair| pair == [b"mcp".as_slice(), b"serve".as_slice()])
        {
            continue;
        }
        let identity =
            crate::process_identity::capture_process_identity(pid, Some(&executable), None)
                .ok_or("legacy_connection_producer_identity_unknown")?;
        let environment = fs::read(proc.join("environ"))
            .map_err(|_| "legacy_connection_producer_environment_unknown")?;
        let producer_home = environment
            .split(|b| *b == 0)
            .find_map(|field| field.strip_prefix(b"HOME="))
            .ok_or("legacy_connection_producer_home_unknown")?;
        let producer_home = std::str::from_utf8(producer_home)
            .map_err(|_| "legacy_connection_producer_home_unknown")?;
        if fs::canonicalize(producer_home).map_err(|_| "legacy_connection_producer_home_unknown")?
            != home
        {
            continue;
        }
        let cgroup = fs::read_to_string(proc.join("cgroup"))
            .map_err(|_| "legacy_connection_producer_unit_unknown")?;
        let unit = cgroup
            .rsplit('/')
            .find(|part| part.trim().ends_with(".service"))
            .ok_or("legacy_connection_producer_unit_unknown")?
            .trim();
        let output = Command::new("systemctl")
            .args([
                "--user",
                "show",
                unit,
                "--property=MainPID",
                "--property=ExecMainStartTimestamp",
                "--property=ActiveState",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("legacy_connection_producer_unit_unreadable".into());
        }
        let raw = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let properties = raw
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect::<BTreeMap<_, _>>();
        if properties
            .get("MainPID")
            .and_then(|pid| pid.parse::<u32>().ok())
            != Some(pid)
            || properties.get("ActiveState") != Some(&"active")
        {
            return Err(format!("legacy_connection_producer_unit_mismatch: pid={pid} unit={unit} mainPid={:?} activeState={:?}",properties.get("MainPID"),properties.get("ActiveState")));
        }
        let timestamp = properties
            .get("ExecMainStartTimestamp")
            .ok_or("legacy_connection_producer_start_unknown")?;
        let parsed = Command::new("date")
            .args(["--date", timestamp, "+%s%3N"])
            .output()
            .map_err(|e| e.to_string())?;
        if !parsed.status.success() {
            return Err("legacy_connection_producer_start_unknown".into());
        }
        let started_ms = String::from_utf8(parsed.stdout)
            .map_err(|e| e.to_string())?
            .trim()
            .parse::<u128>()
            .map_err(|_| "legacy_connection_producer_start_unknown")?;
        if started_ms <= snapshot_ms {
            return Err("legacy_connection_live_predecessor_possible".into());
        }
        if crate::process_identity::recorded_process_is_running(&identity) != Ok(true) {
            return Err("legacy_connection_producer_changed".into());
        }
        producers.push(json!({"identity":identity,"unit":unit,"startedAtUnixMs":started_ms}));
    }
    if producers.is_empty() {
        return Err("legacy_connection_producer_missing".into());
    }
    Ok(
        json!({"observedAtUnixMs":std::time::SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis(),"producers":producers}),
    )
}

#[cfg(not(target_os = "linux"))]
fn producer_census(_: u128) -> Result<Value, String> {
    Err("legacy_connection_reconciliation_requires_linux_systemd".into())
}

/// Preview by default. Apply rechecks evidence and exact custody under the
/// canonical repository lock, preserving the store's multi-file transaction.
pub(crate) fn dispatch(cmd: &Value) -> Option<Result<Value, String>> {
    if cmd["action"] != "service_connections_reconcile" {
        return None;
    }
    Some(run(cmd))
}

fn run(cmd: &Value) -> Result<Value, String> {
    let path = PathBuf::from(
        cmd["planPath"]
            .as_str()
            .ok_or("legacy_connection_plan_required")?,
    );
    if !path.is_absolute() {
        return Err("legacy_connection_absolute_plan_required".into());
    }
    let plan_bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let plan: Plan = serde_json::from_slice(&plan_bytes).map_err(|e| e.to_string())?;
    if plan.schema_version != "agent-browser.legacy-connection-reconciliation.v1"
        || plan.state_path != super::service_store::default_service_state_path()?
        || !plan.custody_snapshot_path.is_absolute()
    {
        return Err("legacy_connection_plan_scope_mismatch".into());
    }
    let snapshot_bytes = std::fs::read(&plan.custody_snapshot_path).map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&snapshot_bytes)) != plan.custody_snapshot_sha256 {
        return Err("legacy_connection_snapshot_digest_mismatch".into());
    }
    let snapshot: Value = serde_json::from_slice(&snapshot_bytes).map_err(|e| e.to_string())?;
    for (id, expected) in &plan.expected_tabs {
        for key in ["browserId", "targetId", "ownerSessionId", "profileAccess"] {
            if snapshot[id][key] != expected[key] {
                return Err("legacy_connection_snapshot_custody_mismatch".into());
            }
        }
    }
    let snapshot_ms = std::fs::metadata(&plan.custody_snapshot_path)
        .and_then(|m| m.modified())
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let repository = LockedServiceStateRepository::default_json()?;
    let apply = cmd["apply"].as_bool() == Some(true);
    let producer_evidence = producer_census(snapshot_ms)?;
    let plan_sha256 = format!("{:x}", Sha256::digest(&plan_bytes));
    let event_id =
        apply.then(|| format!("legacy-connection-reconciliation-{}", uuid::Uuid::new_v4()));
    let event_timestamp = apply.then(|| chrono::Utc::now().to_rfc3339());
    let evidence = if apply {
        repository.mutate(|state| {
            validate_custody(state, &plan)?;
            let changed = state.mark_profile_connection_disconnected(&plan.connection_id);
            if changed != plan.expected_tabs.len() {
                return Err("legacy_connection_changed_count_mismatch".into());
            }
            state.events.push(super::service_model::ServiceEvent {
                id: event_id.clone().expect("apply has an event ID"),
                timestamp: event_timestamp.clone().expect("apply has a timestamp"),
                message: "Local operator reconciled a legacy connection after verified producer replacement".into(),
                details: Some(json!({"planSha256":plan_sha256,"connectionIdSha256":format!("{:x}",Sha256::digest(plan.connection_id.as_bytes())),"affectedTabIds":plan.expected_tabs.keys().collect::<Vec<_>>(),"producerEvidence":producer_evidence})),
                ..Default::default()
            });
            Ok(producer_evidence.clone())
        })?
    } else {
        validate_custody(&repository.load_snapshot()?, &plan)?;
        producer_evidence
    };
    Ok(
        json!({"schemaVersion":"agent-browser.legacy-connection-reconciliation-receipt.v1","applied":apply,"eventId":event_id,"planSha256":format!("{:x}",Sha256::digest(&plan_bytes)),"affectedTabIds":plan.expected_tabs.keys().collect::<Vec<_>>(),"producerEvidence":evidence}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{
        service_model::BrowserTab, service_profile_access_policy::ProfileChildAccess,
    };

    #[test]
    fn legacy_reconciliation_rejects_changed_custody_and_extra_children() {
        let mut state = ServiceState::default();
        let child = ProfileChildAccess {
            subject_id: Some("owner".into()),
            connection_instance_id: Some("connection-legacy".into()),
            ..Default::default()
        };
        state.tabs.insert(
            "tab".into(),
            BrowserTab {
                id: "tab".into(),
                browser_id: "browser".into(),
                target_id: Some("target".into()),
                profile_access: Some(child),
                ..Default::default()
            },
        );
        let plan = Plan {
            schema_version: String::new(),
            state_path: PathBuf::new(),
            custody_snapshot_path: PathBuf::new(),
            custody_snapshot_sha256: String::new(),
            connection_id: "connection-legacy".into(),
            expected_tabs: BTreeMap::from([("tab".into(), custody(&state.tabs["tab"]))]),
        };
        validate_custody(&state, &plan).unwrap();
        let mut changed = state.clone();
        changed
            .tabs
            .get_mut("tab")
            .unwrap()
            .profile_access
            .as_mut()
            .unwrap()
            .subject_id = Some("new-owner".into());
        assert!(validate_custody(&changed, &plan).is_err());
        let mut extra = state.clone();
        extra.tabs.insert("peer".into(), state.tabs["tab"].clone());
        assert!(validate_custody(&extra, &plan).is_err());
        assert_eq!(
            state.mark_profile_connection_disconnected(&plan.connection_id),
            1
        );
        assert!(validate_custody(&state, &plan).is_err());
        let access = state.tabs["tab"].profile_access.as_ref().unwrap();
        assert_eq!(access.subject_id.as_deref(), Some("owner"));
        assert_eq!(
            access.connection_instance_id.as_deref(),
            Some("connection-legacy")
        );
        assert_eq!(state.tabs["tab"].browser_id, "browser");
        assert_eq!(state.tabs["tab"].target_id.as_deref(), Some("target"));
    }
}
