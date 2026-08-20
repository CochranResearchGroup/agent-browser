use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub(crate) const RUNTIME_MULTIPLICITY_SCHEMA_VERSION: &str =
    "agent-browser.runtime-multiplicity.v1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeProcessEvidence {
    pub(crate) pid: u32,
    pub(crate) executable_path: String,
    pub(crate) generation_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RuntimeConvergenceWindowEvidence {
    pub(crate) transaction_id: String,
    pub(crate) state: String,
    pub(crate) deadline: Option<String>,
    pub(crate) active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RuntimeMultiplicityObservation {
    pub(crate) available: bool,
    pub(crate) dashboard_processes: Vec<RuntimeProcessEvidence>,
    pub(crate) runtime_hosts: Vec<RuntimeProcessEvidence>,
    pub(crate) legacy_daemons: Vec<RuntimeProcessEvidence>,
    pub(crate) selected_generation_id: Option<String>,
    pub(crate) executable_generation_ids: Vec<String>,
    pub(crate) convergence_window: Option<RuntimeConvergenceWindowEvidence>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeMultiplicityState {
    SteadyCurrent,
    ConvergenceWindow,
    Drift,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeMultiplicityCounts {
    pub(crate) dashboard_processes: usize,
    pub(crate) runtime_hosts: usize,
    pub(crate) legacy_daemons: usize,
    pub(crate) executable_generations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeMultiplicityReport {
    pub(crate) schema_version: &'static str,
    pub(crate) state: RuntimeMultiplicityState,
    pub(crate) steady_state: bool,
    pub(crate) counts: RuntimeMultiplicityCounts,
    pub(crate) dashboard_processes: Vec<RuntimeProcessEvidence>,
    pub(crate) runtime_hosts: Vec<RuntimeProcessEvidence>,
    pub(crate) legacy_daemons: Vec<RuntimeProcessEvidence>,
    pub(crate) selected_generation_id: Option<String>,
    pub(crate) executable_generation_ids: Vec<String>,
    pub(crate) convergence_window: Option<RuntimeConvergenceWindowEvidence>,
    pub(crate) issues: Vec<String>,
}

/// Classifies dashboard, runtime-host, legacy-daemon, and executable-generation
/// multiplicity without performing lifecycle effects.
pub(crate) fn runtime_multiplicity_report(
    mut observation: RuntimeMultiplicityObservation,
) -> RuntimeMultiplicityReport {
    observation.executable_generation_ids.sort();
    observation.executable_generation_ids.dedup();
    let counts = RuntimeMultiplicityCounts {
        dashboard_processes: observation.dashboard_processes.len(),
        runtime_hosts: observation.runtime_hosts.len(),
        legacy_daemons: observation.legacy_daemons.len(),
        executable_generations: observation.executable_generation_ids.len(),
    };
    let active_window = observation
        .convergence_window
        .as_ref()
        .is_some_and(|window| window.active);
    let steady_state = observation.available
        && !active_window
        && counts.dashboard_processes == 1
        && counts.runtime_hosts == 1
        && counts.legacy_daemons == 0
        && counts.executable_generations == 1
        && observation
            .selected_generation_id
            .as_ref()
            .is_some_and(|selected| {
                observation
                    .executable_generation_ids
                    .iter()
                    .any(|generation| generation == selected)
            });
    let bounded_window = observation.available
        && active_window
        && (1..=2).contains(&counts.dashboard_processes)
        && (1..=2).contains(&counts.runtime_hosts)
        && counts.legacy_daemons == 0
        && (1..=2).contains(&counts.executable_generations);
    let state = if steady_state {
        RuntimeMultiplicityState::SteadyCurrent
    } else if bounded_window {
        RuntimeMultiplicityState::ConvergenceWindow
    } else if observation.available {
        RuntimeMultiplicityState::Drift
    } else {
        RuntimeMultiplicityState::Unknown
    };
    let mut issues = Vec::new();
    if !observation.available {
        issues.push("multiplicity_observation_unavailable".to_string());
    }
    if counts.dashboard_processes != 1 && !active_window {
        issues.push("dashboard_process_count_not_one".to_string());
    }
    if counts.runtime_hosts != 1 && !active_window {
        issues.push("runtime_host_count_not_one".to_string());
    }
    if counts.legacy_daemons > 0 {
        issues.push("legacy_session_daemons_present".to_string());
    }
    let allowed_generations = if active_window { 2 } else { 1 };
    if counts.executable_generations > allowed_generations {
        issues.push("executable_generation_multiplicity".to_string());
    }
    if observation
        .selected_generation_id
        .as_ref()
        .is_some_and(|selected| {
            !observation
                .executable_generation_ids
                .iter()
                .any(|generation| generation == selected)
        })
    {
        issues.push("selected_generation_not_executing".to_string());
    }
    if active_window && !bounded_window {
        issues.push("convergence_window_multiplicity_exceeded".to_string());
    }

    RuntimeMultiplicityReport {
        schema_version: RUNTIME_MULTIPLICITY_SCHEMA_VERSION,
        state,
        steady_state,
        counts,
        dashboard_processes: observation.dashboard_processes,
        runtime_hosts: observation.runtime_hosts,
        legacy_daemons: observation.legacy_daemons,
        selected_generation_id: observation.selected_generation_id,
        executable_generation_ids: observation.executable_generation_ids,
        convergence_window: observation.convergence_window,
        issues,
    }
}

/// Builds the read-only install-doctor projection from current process and
/// workstation transaction evidence.
pub(crate) fn runtime_multiplicity_report_from_doctor_inputs(
    daemon_listener_inventory: &Value,
    runtime_inventory: &Value,
    live_dashboard_runtime: &Value,
    workstation_payload: &Value,
) -> Value {
    let dashboard_process = dashboard_backend_process();
    let mut runtime_hosts = Vec::new();
    let mut legacy_daemons = Vec::new();
    for listener in daemon_listener_inventory
        .get("listeners")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(pid) = listener
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
        else {
            continue;
        };
        let executable_path = listener
            .get("exe")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let evidence = RuntimeProcessEvidence {
            pid,
            generation_id: generation_id_from_executable_path(&executable_path),
            executable_path,
        };
        let socket_path = listener
            .get("socketPath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if socket_path.ends_with("/runtime-host.sock") {
            runtime_hosts.push(evidence);
        } else {
            legacy_daemons.push(evidence);
        }
    }
    let dashboard_processes = dashboard_process.into_iter().collect::<Vec<_>>();
    let selected_generation_id = workstation_payload
        .get("selectedGenerationId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let executable_generation_ids = dashboard_processes
        .iter()
        .chain(runtime_hosts.iter())
        .chain(legacy_daemons.iter())
        .filter_map(|process| process.generation_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let transaction = live_dashboard_runtime.pointer("/workstationUpgrade/latestTransaction");
    let convergence_window = transaction.and_then(|transaction| {
        let state = transaction.get("state")?.as_str()?.to_string();
        Some(RuntimeConvergenceWindowEvidence {
            transaction_id: transaction
                .get("transactionId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            deadline: transaction
                .get("deadline")
                .and_then(Value::as_str)
                .map(str::to_string),
            active: transaction_state_is_active(&state),
            state,
        })
    });
    let available = cfg!(target_os = "linux")
        && daemon_listener_inventory
            .get("available")
            .and_then(Value::as_bool)
            == Some(true)
        && runtime_inventory
            .get("status")
            .and_then(Value::as_str)
            .is_some();
    serde_json::to_value(runtime_multiplicity_report(
        RuntimeMultiplicityObservation {
            available,
            dashboard_processes,
            runtime_hosts,
            legacy_daemons,
            selected_generation_id,
            executable_generation_ids,
            convergence_window,
        },
    ))
    .unwrap_or_else(|_| {
        serde_json::json!({
            "schemaVersion": RUNTIME_MULTIPLICITY_SCHEMA_VERSION,
            "state": "unknown",
            "steadyState": false,
            "issues": ["multiplicity_report_serialization_failed"]
        })
    })
}

fn generation_id_from_executable_path(path: &str) -> Option<String> {
    let components = Path::new(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    components
        .windows(2)
        .find(|pair| pair[0] == "generations")
        .map(|pair| pair[1].clone())
        .filter(|generation| !generation.trim().is_empty())
}

fn transaction_state_is_active(state: &str) -> bool {
    matches!(
        state,
        "planned"
            | "candidate_staged"
            | "census_stable"
            | "admission_draining"
            | "transferring"
            | "candidate_ready"
            | "generation_committed"
            | "post_commit_validating"
            | "rolling_back"
    )
}

fn dashboard_backend_process() -> Option<RuntimeProcessEvidence> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args([
                "--user",
                "show",
                "agent-browser-dashboard-backend.service",
                "--property=MainPID",
                "--value",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let pid = String::from_utf8(output.stdout)
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|pid| *pid > 0)?;
        let executable_path = std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()?
            .display()
            .to_string();
        Some(RuntimeProcessEvidence {
            pid,
            generation_id: generation_id_from_executable_path(&executable_path),
            executable_path,
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, generation: &str) -> RuntimeProcessEvidence {
        RuntimeProcessEvidence {
            pid,
            executable_path: format!(
                "/tmp/agent-browser/generations/{generation}/bin/agent-browser"
            ),
            generation_id: Some(generation.to_string()),
        }
    }

    #[test]
    fn legacy_session_daemons_are_reported_as_multiplicity_drift() {
        let report = runtime_multiplicity_report(RuntimeMultiplicityObservation {
            available: true,
            dashboard_processes: vec![process(100, "current")],
            runtime_hosts: Vec::new(),
            legacy_daemons: vec![
                process(201, "current"),
                process(202, "current"),
                process(203, "current"),
            ],
            selected_generation_id: Some("current".to_string()),
            executable_generation_ids: vec!["current".to_string()],
            convergence_window: None,
        });

        assert_eq!(report.state, RuntimeMultiplicityState::Drift);
        assert!(!report.steady_state);
        assert_eq!(report.counts.dashboard_processes, 1);
        assert_eq!(report.counts.runtime_hosts, 0);
        assert_eq!(report.counts.legacy_daemons, 3);
        assert_eq!(report.counts.executable_generations, 1);
        assert_eq!(
            report.issues,
            vec![
                "runtime_host_count_not_one",
                "legacy_session_daemons_present"
            ]
        );
    }

    #[test]
    fn one_dashboard_one_host_and_one_selected_generation_is_steady() {
        let report = runtime_multiplicity_report(RuntimeMultiplicityObservation {
            available: true,
            dashboard_processes: vec![process(100, "current")],
            runtime_hosts: vec![process(200, "current")],
            legacy_daemons: Vec::new(),
            selected_generation_id: Some("current".to_string()),
            executable_generation_ids: vec!["current".to_string()],
            convergence_window: None,
        });

        assert_eq!(report.state, RuntimeMultiplicityState::SteadyCurrent);
        assert!(report.steady_state);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn transaction_window_allows_two_hosts_and_two_generations_only() {
        let report = runtime_multiplicity_report(RuntimeMultiplicityObservation {
            available: true,
            dashboard_processes: vec![process(100, "old"), process(101, "candidate")],
            runtime_hosts: vec![process(200, "old"), process(201, "candidate")],
            legacy_daemons: Vec::new(),
            selected_generation_id: Some("old".to_string()),
            executable_generation_ids: vec!["old".to_string(), "candidate".to_string()],
            convergence_window: Some(RuntimeConvergenceWindowEvidence {
                transaction_id: "upgrade-fixture".to_string(),
                state: "transferring".to_string(),
                deadline: Some("2026-08-20T20:00:00Z".to_string()),
                active: true,
            }),
        });

        assert_eq!(report.state, RuntimeMultiplicityState::ConvergenceWindow);
        assert!(!report.steady_state);
        assert!(report.issues.is_empty());
    }
}
