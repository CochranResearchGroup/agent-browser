use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::service_model::ServiceState;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ProcessSample {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub process_group_id: Option<u32>,
    pub start_token: Option<String>,
    pub executable: Option<String>,
    pub command: Vec<String>,
    pub rss_bytes: Option<u64>,
    pub cpu_seconds: Option<u64>,
    pub age_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ResourceCorrelation {
    pub(crate) browser_id: Option<String>,
    pub(crate) profile_id: Option<String>,
    pub(crate) session_ids: Vec<String>,
    pub(crate) display_allocation_id: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) cdp_port: Option<u16>,
    pub(crate) profile_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResourceKind {
    AgentBrowser,
    Browser,
    RemoteDisplay,
    #[default]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResourceDisposition {
    Protected,
    Candidate,
    #[default]
    Observed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ResourceRecord {
    pub(crate) pid: u32,
    pub(crate) ppid: Option<u32>,
    pub(crate) process_group_id: Option<u32>,
    pub(crate) executable: Option<String>,
    pub(crate) command_preview: String,
    pub(crate) kind: ResourceKind,
    pub(crate) correlation: ResourceCorrelation,
    pub(crate) rss_bytes: Option<u64>,
    pub(crate) cpu_seconds: Option<u64>,
    pub(crate) age_seconds: Option<u64>,
    pub(crate) disposition: ResourceDisposition,
    pub(crate) reasons: Vec<String>,
    pub(crate) gc_action: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ResourceSummary {
    pub(crate) total_processes: usize,
    pub(crate) correlated_processes: usize,
    pub(crate) candidate_count: usize,
    pub(crate) protected_count: usize,
    pub(crate) observed_count: usize,
    pub(crate) candidate_rss_bytes: u64,
    pub(crate) protected_rss_bytes: u64,
    pub(crate) observed_rss_bytes: u64,
    pub(crate) total_rss_bytes: u64,
    pub(crate) managed_lane_count: usize,
    pub(crate) cleanup_obligations_owned: usize,
    pub(crate) cleanup_obligations_transferring: usize,
    pub(crate) cleanup_obligations_satisfied: usize,
    pub(crate) cleanup_obligations_unknown: usize,
    pub(crate) challenge_tasks: agent_browser_service_model::ServiceChallengeTaskSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ResourceAuthoritySnapshot {
    pub(crate) summary: ResourceSummary,
    pub(crate) resources: Vec<ResourceRecord>,
    pub(crate) warnings: Vec<Value>,
    pub(crate) collection_warnings: Vec<String>,
}

fn removed_authority_warning() -> Value {
    json!({
        "code": "legacy_resource_authority_removed",
        "severity": "info",
        "message": "Legacy Service State process ownership and cleanup admission are not runtime authority.",
    })
}

pub(crate) fn service_resource_authority_snapshot(
    state: &ServiceState,
) -> ResourceAuthoritySnapshot {
    service_resource_authority_snapshot_from_samples(state, Vec::new(), Vec::new())
}

pub(crate) fn service_resource_authority_snapshot_from_samples(
    state: &ServiceState,
    _processes: Vec<ProcessSample>,
    mut collection_warnings: Vec<String>,
) -> ResourceAuthoritySnapshot {
    collection_warnings.push("legacy_resource_authority_removed".to_string());
    ResourceAuthoritySnapshot {
        summary: ResourceSummary {
            managed_lane_count: state.browsers.len(),
            challenge_tasks: state.service_challenge_task_summary(),
            ..ResourceSummary::default()
        },
        resources: Vec::new(),
        warnings: vec![removed_authority_warning()],
        collection_warnings,
    }
}

pub(crate) fn service_resources_response(state: &ServiceState) -> Value {
    let snapshot = service_resource_authority_snapshot(state);
    json!({
        "summary": snapshot.summary,
        "resources": snapshot.resources,
        "lanes": [],
        "workstation": {
            "browserLaneCount": state.browsers.len(),
            "processCount": 0,
            "rssBytes": 0,
            "budgetExceeded": [],
        },
        "runtimeLanes": [],
        "warnings": snapshot.warnings,
        "collectionWarnings": snapshot.collection_warnings,
        "policy": {
            "cleanupAuthority": "browser_session_manager",
            "legacyGcAvailable": false,
        },
    })
}

fn monitor_summary_path() -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    Ok(home.join(".agent-browser/service/resources-monitor.json"))
}

pub(crate) fn service_resources_write_monitor_summary_response(
    state: &ServiceState,
) -> Result<Value, String> {
    let path = monitor_summary_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create resource monitor directory: {error}"))?;
    }
    let summary = json!({
        "schemaVersion": "agent-browser.service-resource-monitor.v1",
        "observedAt": chrono::Utc::now().to_rfc3339(),
        "resources": service_resources_response(state),
    });
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&summary)
            .map_err(|error| format!("serialize resource monitor summary: {error}"))?,
    )
    .map_err(|error| format!("write resource monitor summary: {error}"))?;
    Ok(json!({ "success": true, "path": path, "summary": summary }))
}

pub(crate) fn service_resources_monitor_summary_response() -> Result<Value, String> {
    let path = monitor_summary_path()?;
    let value = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode resource monitor summary: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Value::Null,
        Err(error) => return Err(format!("read resource monitor summary: {error}")),
    };
    Ok(json!({ "success": true, "path": path, "summary": value }))
}

fn service_gc_response(state: &ServiceState, apply: bool) -> Value {
    json!({
        "success": true,
        "applied": apply,
        "candidateCount": 0,
        "candidates": [],
        "cleanupAuthority": "browser_session_manager",
        "resources": service_resources_response(state),
    })
}

pub(crate) mod service_commands {
    use super::*;
    use crate::native::action_runtime::runtime::{
        account_ids_from_command, browser_build_from_command, browser_build_is_explicit,
        browser_host_from_command, optional_command_string, parse_control_input_provider,
        parse_view_stream_provider, remote_headed_display_isolation_from_command,
        runtime_profile_from_sources, target_service_ids_from_command, target_url_from_command,
    };
    use crate::native::service_access::{
        access_plan_browser_build_selection_summary, service_access_plan_for_state,
        ServiceAccessPlanRequest,
    };
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};

    pub(crate) async fn handle_service_resources(cmd: &Value) -> Result<Value, String> {
        Ok(service_resources_response(
            &load_service_state_for_maintenance(cmd)?,
        ))
    }

    pub(crate) async fn handle_service_resources_monitor_summary() -> Result<Value, String> {
        service_resources_monitor_summary_response()
    }

    pub(crate) async fn handle_service_resources_write_monitor_summary(
        cmd: &Value,
    ) -> Result<Value, String> {
        service_resources_write_monitor_summary_response(&load_service_state_for_maintenance(cmd)?)
    }

    pub(crate) async fn handle_service_gc(cmd: &Value) -> Result<Value, String> {
        let state = load_service_state_for_maintenance(cmd)?;
        Ok(service_gc_response(
            &state,
            cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
        ))
    }

    pub(crate) fn load_service_state_for_maintenance(cmd: &Value) -> Result<ServiceState, String> {
        if let Some(service_state) = cmd.get("serviceState") {
            serde_json::from_value(service_state.clone())
                .map_err(|error| format!("Invalid serviceState: {error}"))
        } else {
            LockedServiceStateRepository::default_json()?.load_snapshot()
        }
    }

    pub(crate) async fn handle_service_access_plan(cmd: &Value) -> Result<Value, String> {
        let mut state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|error| format!("Invalid serviceState: {error}"))?
            .unwrap_or_default();
        state.refresh_profile_readiness();
        let request = ServiceAccessPlanRequest {
            service_name: optional_command_string(cmd, "serviceName"),
            agent_name: optional_command_string(cmd, "agentName"),
            task_name: optional_command_string(cmd, "taskName"),
            client_subject_id: optional_command_string(cmd, "clientSubjectId"),
            identity_assurance: optional_command_string(cmd, "identityAssurance"),
            session_name: optional_command_string(cmd, "sessionName"),
            target_service_ids: target_service_ids_from_command(cmd),
            account_ids: account_ids_from_command(cmd),
            target_url: target_url_from_command(cmd),
            site_policy_id: optional_command_string(cmd, "sitePolicyId"),
            challenge_id: optional_command_string(cmd, "challengeId"),
            readiness_profile_id: optional_command_string(cmd, "readinessProfileId"),
            runtime_profile: runtime_profile_from_sources(cmd, false),
            browser_build: browser_build_from_command(cmd),
            browser_build_explicit: browser_build_is_explicit(cmd),
            browser_host: browser_host_from_command(cmd),
            view_stream_provider: optional_command_string(cmd, "viewStreamProvider")
                .or_else(|| optional_command_string(cmd, "viewStream"))
                .and_then(|value| parse_view_stream_provider(&value)),
            control_input_provider: optional_command_string(cmd, "controlInputProvider")
                .or_else(|| optional_command_string(cmd, "controlInput"))
                .and_then(|value| parse_control_input_provider(&value)),
            display_isolation: remote_headed_display_isolation_from_command(cmd),
        };
        let mut plan = service_access_plan_for_state(&state, request);
        let summary = access_plan_browser_build_selection_summary(&plan);
        if let Some(object) = plan.as_object_mut() {
            object.insert("browserBuildSelectionSummary".to_string(), summary);
        }
        Ok(plan)
    }
}
