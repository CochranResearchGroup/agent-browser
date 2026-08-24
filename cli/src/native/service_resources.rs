use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::service_model::{
    BrowserHealth, LeaseState, ServiceEvent, ServiceEventKind, ServiceState,
};

const TEMP_PROFILE_MIN_AGE_SECONDS: u64 = 30 * 60;
const OWNED_CLOSING_GRACE_SECONDS: u64 = 5;
const GC_REVIEW_TOKEN_TTL_SECONDS: u64 = 10 * 60;
const GC_TERM_WAIT_MS: u64 = 1_500;

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
    candidate_identity: Option<GcCandidateIdentity>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GcCandidateIdentity {
    pid: u32,
    process_group_id: Option<u32>,
    start_token: Option<String>,
    executable_path: Option<String>,
    owner_generation: Option<u64>,
    profile_identity_digest: Option<String>,
    package_launch_identity_digest: Option<String>,
    kind: String,
    action: String,
    command_digest: String,
    browser_id: Option<String>,
    profile_id: Option<String>,
    display_allocation_id: Option<String>,
    profile_path: Option<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResourceRuntimeEnvironment {
    Production,
    Development {
        state_root: PathBuf,
        install_root: PathBuf,
        socket_dir: PathBuf,
    },
}

impl ResourceRuntimeEnvironment {
    fn current() -> Self {
        if std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT").as_deref() != Ok("development") {
            return Self::Production;
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/nonexistent-agent-browser-development-home"));
        let install_root = std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.ancestors()
                    .find(|ancestor| {
                        ancestor
                            .file_name()
                            .is_some_and(|name| name == "generations")
                    })
                    .and_then(Path::parent)
                    .map(Path::to_path_buf)
            })
            .unwrap_or_else(|| home.join(".local/lib/agent-browser-dev"));
        let socket_dir = std::env::var_os("AGENT_BROWSER_SOCKET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/nonexistent-agent-browser-development-socket"));
        Self::Development {
            state_root: home.join(".agent-browser"),
            install_root,
            socket_dir,
        }
    }

    fn requires_positive_ownership(&self) -> bool {
        matches!(self, Self::Development { .. })
    }

    fn proves_ownership(&self, process: &ProcessSample, correlation: &ResourceCorrelation) -> bool {
        let Self::Development {
            state_root,
            install_root,
            socket_dir,
        } = self
        else {
            return true;
        };
        if correlation.browser_id.is_some() {
            return true;
        }
        if correlation
            .profile_path
            .as_deref()
            .is_some_and(|path| Path::new(path).starts_with(state_root))
        {
            return true;
        }
        if process
            .executable
            .as_deref()
            .is_some_and(|path| Path::new(path).starts_with(install_root))
        {
            return true;
        }
        process.command.iter().any(|argument| {
            let path = Path::new(argument);
            path.starts_with(state_root)
                || path.starts_with(install_root)
                || path.starts_with(socket_dir)
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ResourceAuthoritySnapshot {
    pub(crate) summary: ResourceSummary,
    pub(crate) resources: Vec<ResourceRecord>,
    pub(crate) warnings: Vec<Value>,
    pub(crate) collection_warnings: Vec<String>,
    protects_dashboard_main_pid: bool,
}

pub(crate) fn service_resources_response(state: &ServiceState) -> Value {
    let (processes, collection_warnings) = collect_process_samples();
    service_resources_response_from_samples(state, processes, collection_warnings)
}

pub(crate) fn service_resource_authority_snapshot(
    state: &ServiceState,
) -> ResourceAuthoritySnapshot {
    let (processes, collection_warnings) = collect_process_samples();
    service_resource_authority_snapshot_from_samples(state, processes, collection_warnings)
}

pub(crate) fn service_resources_write_monitor_summary_response(
    state: &ServiceState,
) -> Result<Value, String> {
    let response = service_resources_response(state);
    let observed_at = chrono::Utc::now().to_rfc3339();
    let summary = compact_monitor_summary(&response, &observed_at);
    let path = resource_monitor_summary_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create resource monitor summary directory {}: {}",
                parent.display(),
                err
            )
        })?;
    }
    let payload = serde_json::to_string_pretty(&summary)
        .map_err(|err| format!("Failed to serialize resource monitor summary: {}", err))?;
    fs::write(&path, format!("{payload}\n")).map_err(|err| {
        format!(
            "Failed to write resource monitor summary {}: {}",
            path.display(),
            err
        )
    })?;
    Ok(json!({
        "written": true,
        "path": path,
        "summary": summary,
    }))
}

pub(crate) fn service_resources_monitor_summary_response() -> Result<Value, String> {
    let path = resource_monitor_summary_path()?;
    let raw = fs::read_to_string(&path).map_err(|err| {
        format!(
            "Failed to read resource monitor summary {}: {}",
            path.display(),
            err
        )
    })?;
    let summary = serde_json::from_str::<Value>(&raw).map_err(|err| {
        format!(
            "Invalid resource monitor summary JSON {}: {}",
            path.display(),
            err
        )
    })?;
    Ok(json!({
        "path": path,
        "summary": summary,
    }))
}

pub(crate) fn service_gc_dry_run_response(state: &ServiceState) -> Value {
    let response = service_resources_response(state);
    let candidates = candidates_from_response(&response);
    let projected_rss_bytes = projected_rss_bytes(&candidates);
    let issued_at = unix_now_seconds();
    let review_token = review_token_for_candidates(&candidates, issued_at);
    json!({
        "dryRun": true,
        "apply": false,
        "candidateCount": candidates.len(),
        "reviewToken": review_token,
        "reviewExpiresAtEpochSeconds": issued_at + GC_REVIEW_TOKEN_TTL_SECONDS,
        "projectedReclaimed": {
            "rssBytes": projected_rss_bytes,
        },
        "actions": {
            "terminateProcess": candidates,
        },
        "warnings": response.get("warnings").cloned().unwrap_or_else(|| json!([])),
        "policy": response.get("policy").cloned().unwrap_or_else(|| json!({})),
        "recommendedNextStep": if candidates.is_empty() {
            "No GC candidates found. Use service resources --json to review protected and observed processes."
        } else {
            "Review candidates, then rerun service gc --apply --review-token <token> before the token expires."
        },
    })
}

pub(crate) fn service_gc_apply_response(
    state: &mut ServiceState,
    review_token: Option<&str>,
    force_without_review: bool,
) -> Value {
    let (processes, collection_warnings) = collect_process_samples();
    service_gc_apply_response_from_samples(
        state,
        processes,
        collection_warnings,
        review_token,
        force_without_review,
        &LiveInspector,
        &LiveTerminator,
    )
}

pub(crate) fn service_gc_unattended_response(state: &mut ServiceState) -> Value {
    let (processes, collection_warnings) = collect_process_samples();
    service_gc_unattended_response_from_samples(
        state,
        processes,
        collection_warnings,
        &LiveInspector,
        &LiveTerminator,
    )
}

fn service_resources_response_from_samples(
    state: &ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
) -> Value {
    service_resources_response_from_samples_for_environment(
        state,
        processes,
        collection_warnings,
        ResourceRuntimeEnvironment::current(),
    )
}

fn service_resources_response_from_samples_for_environment(
    state: &ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
    environment: ResourceRuntimeEnvironment,
) -> Value {
    let snapshot = service_resource_authority_snapshot_from_samples_for_environment(
        state,
        processes,
        collection_warnings,
        &environment,
    );
    json!({
        "summary": snapshot.summary,
        "resources": snapshot.resources,
        "runtimeLanes": state.runtime_owner_registry.lifecycle_records.values().collect::<Vec<_>>(),
        "warnings": snapshot.warnings,
        "policy": {
            "protectsDashboardMainPid": snapshot.protects_dashboard_main_pid,
            "protectsRetainedBrowserPids": true,
            "protectsNamedManagedProfiles": true,
            "temporaryProfileMinAgeSeconds": TEMP_PROFILE_MIN_AGE_SECONDS,
            "reviewTokenTtlSeconds": GC_REVIEW_TOKEN_TTL_SECONDS,
            "applySupported": true,
            "requiresRuntimeEnvironmentOwnershipForCandidates": environment.requires_positive_ownership(),
        },
    })
}

pub(crate) fn service_resource_authority_snapshot_from_samples(
    state: &ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
) -> ResourceAuthoritySnapshot {
    service_resource_authority_snapshot_from_samples_for_environment(
        state,
        processes,
        collection_warnings,
        &ResourceRuntimeEnvironment::current(),
    )
}

fn service_resource_authority_snapshot_from_samples_for_environment(
    state: &ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
    environment: &ResourceRuntimeEnvironment,
) -> ResourceAuthoritySnapshot {
    let dashboard_main_pid = current_dashboard_main_pid();
    let mut records = processes
        .into_iter()
        .filter_map(|process| classify_process(state, dashboard_main_pid, process, environment))
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.pid);

    let summary = summarize_resources(state, &records);
    let warnings = resource_warnings(state, collection_warnings.clone());
    ResourceAuthoritySnapshot {
        summary,
        resources: records,
        warnings,
        collection_warnings,
        protects_dashboard_main_pid: dashboard_main_pid.is_some(),
    }
}

fn resource_warnings(state: &ServiceState, collection_warnings: Vec<String>) -> Vec<Value> {
    let mut warnings = collection_warnings
        .into_iter()
        .map(|message| {
            json!({
                "code": "process_collection_warning",
                "message": message,
            })
        })
        .collect::<Vec<_>>();
    warnings.extend(duplicate_profile_pressure_warnings(state));
    warnings
}

fn duplicate_profile_pressure_warnings(state: &ServiceState) -> Vec<Value> {
    let mut warnings = Vec::new();
    let mut browser_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (browser_id, browser) in &state.browsers {
        let Some(profile_id) = browser.profile_id.as_deref() else {
            continue;
        };
        if !browser_health_counts_as_live(browser.health) {
            continue;
        }
        let profile = state.profiles.get(profile_id);
        let provider = browser
            .view_streams
            .first()
            .map(|stream| enum_json_label(&stream.provider))
            .unwrap_or_else(|| "none".to_string());
        let control_input = browser
            .view_streams
            .first()
            .and_then(|stream| stream.control_input)
            .map(|input| enum_json_label(&input))
            .unwrap_or_else(|| "none".to_string());
        let browser_build = profile
            .and_then(|profile| profile.browser_build)
            .map(|build| enum_json_label(&build))
            .unwrap_or_else(|| "unspecified".to_string());
        let key = format!(
            "profile={profile_id}|host={}|display={}|stream={provider}|input={control_input}|build={browser_build}",
            enum_json_label(&browser.host),
            browser
                .display_isolation
                .as_deref()
                .unwrap_or("unspecified")
        );
        browser_groups
            .entry(key)
            .or_default()
            .push(browser_id.clone());
    }

    for (key, mut browser_ids) in browser_groups {
        browser_ids.sort();
        browser_ids.dedup();
        if browser_ids.len() <= 1 {
            continue;
        }
        let count = browser_ids.len();
        let profile_id = key
            .split('|')
            .find_map(|part| part.strip_prefix("profile="))
            .unwrap_or_default();
        let profile = state.profiles.get(profile_id);
        warnings.push(json!({
            "code": "duplicate_live_browsers_for_profile",
            "message": "multiple live browsers share the same retained profile and posture; use access-plan profileReuse route hints before launching another browser",
            "profileId": profile_id,
            "browserIds": browser_ids,
            "count": count,
            "postureKey": key,
            "targetServiceIds": profile.map(|profile| profile.target_service_ids.clone()).unwrap_or_default(),
            "authenticatedServiceIds": profile.map(|profile| profile.authenticated_service_ids.clone()).unwrap_or_default(),
            "accountIds": profile.map(|profile| profile.account_ids.clone()).unwrap_or_default(),
        }));
    }

    let mut lease_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (session_id, session) in &state.sessions {
        let Some(profile_id) = session.profile_id.as_deref() else {
            continue;
        };
        if matches!(
            session.lease,
            LeaseState::Exclusive | LeaseState::HumanTakeover
        ) {
            lease_groups
                .entry(profile_id.to_string())
                .or_default()
                .push(session_id.clone());
        }
    }
    for (profile_id, mut session_ids) in lease_groups {
        session_ids.sort();
        session_ids.dedup();
        if session_ids.len() <= 1 {
            continue;
        }
        if sessions_share_any_live_browser(state, &session_ids) {
            continue;
        }
        let count = session_ids.len();
        warnings.push(json!({
            "code": "duplicate_active_profile_leases",
            "message": "multiple active exclusive sessions hold the same retained profile; use profileLeasePolicy=wait or access-plan copied requests instead of cloning the lane",
            "profileId": profile_id,
            "sessionIds": session_ids,
            "count": count,
        }));
    }

    warnings
}

fn sessions_share_any_live_browser(state: &ServiceState, session_ids: &[String]) -> bool {
    let mut shared_browser_ids: Option<BTreeSet<String>> = None;
    for session_id in session_ids {
        let Some(session) = state.sessions.get(session_id) else {
            return false;
        };
        let live_browser_ids = session
            .browser_ids
            .iter()
            .filter(|browser_id| {
                state
                    .browsers
                    .get(*browser_id)
                    .is_some_and(|browser| browser_health_counts_as_live(browser.health))
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        if live_browser_ids.is_empty() {
            return false;
        }
        shared_browser_ids = Some(match shared_browser_ids {
            Some(existing) => existing
                .intersection(&live_browser_ids)
                .cloned()
                .collect::<BTreeSet<_>>(),
            None => live_browser_ids,
        });
        if shared_browser_ids.as_ref().is_some_and(BTreeSet::is_empty) {
            return false;
        }
    }
    shared_browser_ids.is_some_and(|browser_ids| !browser_ids.is_empty())
}

fn browser_health_counts_as_live(health: BrowserHealth) -> bool {
    !matches!(
        health,
        BrowserHealth::NotStarted
            | BrowserHealth::ProcessExited
            | BrowserHealth::Closing
            | BrowserHealth::Faulted
    )
}

fn enum_json_label<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn compact_monitor_summary(resources_response: &Value, observed_at: &str) -> Value {
    json!({
        "schemaVersion": 1,
        "observedAt": observed_at,
        "summary": resources_response.get("summary").cloned().unwrap_or_else(|| json!({})),
        "warnings": resources_response.get("warnings").cloned().unwrap_or_else(|| json!([])),
        "policy": resources_response.get("policy").cloned().unwrap_or_else(|| json!({})),
    })
}

fn resource_monitor_summary_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".agent-browser")
        .join("service")
        .join("resource-monitor-summary.json"))
}

fn classify_process(
    state: &ServiceState,
    dashboard_main_pid: Option<u32>,
    process: ProcessSample,
    environment: &ResourceRuntimeEnvironment,
) -> Option<ResourceRecord> {
    let profile_path = command_arg_value(&process.command, "--user-data-dir");
    let cdp_port = command_arg_value(&process.command, "--remote-debugging-port")
        .and_then(|value| value.parse::<u16>().ok());
    let kind = resource_kind(&process, profile_path.as_deref(), cdp_port);
    if kind == ResourceKind::Other {
        return None;
    }

    let correlation = correlate_process(state, &process, profile_path, cdp_port);
    let mut disposition = ResourceDisposition::Observed;
    let mut reasons = Vec::new();
    let mut gc_action = None;
    let mut reviewed_tree = None;

    if kind == ResourceKind::AgentBrowser && installed_agent_browser_runtime_surface(&process) {
        disposition = ResourceDisposition::Protected;
        reasons.push("installed_agent_browser_runtime_surface".to_string());
    } else if Some(process.pid) == dashboard_main_pid {
        disposition = ResourceDisposition::Protected;
        reasons.push("dashboard_main_pid".to_string());
    }
    if correlation.browser_id.is_some()
        && retained_browser_pid_is_active(state, correlation.browser_id.as_deref())
    {
        disposition = ResourceDisposition::Protected;
        reasons.push("retained_active_browser".to_string());
    }
    let retained_named_or_persistent_profile = correlation
        .profile_id
        .as_deref()
        .is_some_and(|profile_id| retained_profile_is_named_or_persistent(state, profile_id));
    if correlation
        .display_allocation_id
        .as_deref()
        .is_some_and(|display_id| state.display_allocations.contains_key(display_id))
    {
        disposition = ResourceDisposition::Protected;
        reasons.push("retained_display_allocation".to_string());
    }

    if disposition != ResourceDisposition::Protected {
        if correlation.browser_id.is_some()
            || temporary_profile_path(correlation.profile_path.as_deref())
        {
            let decision =
                crate::native::runtime_reconciliation::RuntimeResourceReconciler::new(state)
                    .classify(
                        crate::native::runtime_reconciliation::RuntimeProcessEvidence {
                            process: crate::process_identity::ObservedProcessIdentity {
                                pid: process.pid,
                                start_token: process.start_token.clone(),
                                executable_path: process.executable.clone(),
                                browser_family: crate::process_identity::browser_family_for_path(
                                    process.executable.as_deref().map(Path::new),
                                ),
                                command_line: Some(process.command.clone()),
                            },
                            process_group_id: process.process_group_id,
                            logical_browser_id: correlation.browser_id.clone(),
                            profile_root: correlation.profile_path.clone(),
                        },
                    );
            match decision {
                crate::native::runtime_reconciliation::RuntimeResourceDecision::Owned(tree)
                    if process
                        .age_seconds
                        .is_some_and(|age| age >= OWNED_CLOSING_GRACE_SECONDS) =>
                {
                    disposition = ResourceDisposition::Candidate;
                    reasons.push("lifecycle_owned_closing_process_tree_after_grace".to_string());
                    gc_action = Some("terminate_process_tree".to_string());
                    reviewed_tree = Some(tree);
                }
                crate::native::runtime_reconciliation::RuntimeResourceDecision::Owned(_) => {
                    disposition = ResourceDisposition::Protected;
                    reasons.push("runtime_lifecycle_closing_grace_active".to_string());
                }
                crate::native::runtime_reconciliation::RuntimeResourceDecision::Protected {
                    reason,
                } => {
                    if retained_named_or_persistent_profile {
                        disposition = ResourceDisposition::Protected;
                        reasons.push("retained_named_or_persistent_profile".to_string());
                        reasons.push(reason.to_string());
                    } else if process
                        .age_seconds
                        .is_some_and(|age| age >= TEMP_PROFILE_MIN_AGE_SECONDS)
                    {
                        disposition = ResourceDisposition::Protected;
                        reasons.push(reason.to_string());
                    } else {
                        reasons.push("temporary_profile_too_fresh_or_unknown_age".to_string());
                    }
                }
            }
        } else if retained_named_or_persistent_profile {
            disposition = ResourceDisposition::Protected;
            reasons.push("retained_named_or_persistent_profile".to_string());
        } else if kind == ResourceKind::RemoteDisplay && correlation.browser_id.is_none() {
            disposition = ResourceDisposition::Candidate;
            reasons.push("orphaned_remote_display_process".to_string());
            gc_action = Some("terminate_process".to_string());
        } else if correlation.browser_id.is_none() && kind == ResourceKind::AgentBrowser {
            reasons.push("agent_browser_process_unowned_by_service_state".to_string());
        } else if temporary_profile_path(correlation.profile_path.as_deref()) {
            reasons.push("temporary_profile_too_fresh_or_unknown_age".to_string());
        } else {
            reasons.push("no_safe_gc_predicate_matched".to_string());
        }
    }

    if disposition == ResourceDisposition::Candidate
        && environment.requires_positive_ownership()
        && !environment.proves_ownership(&process, &correlation)
    {
        // An isolated environment sees the host-wide process table but only its
        // own Service State. Missing local correlation cannot authorize cleanup
        // of a process owned by production or another runtime environment.
        disposition = ResourceDisposition::Protected;
        reasons.clear();
        reasons.push("runtime_environment_ownership_unproven".to_string());
        gc_action = None;
        reviewed_tree = None;
    }

    let candidate_identity = gc_action.as_ref().map(|action| GcCandidateIdentity {
        pid: process.pid,
        process_group_id: process.process_group_id,
        start_token: reviewed_tree
            .as_ref()
            .map(|tree| tree.root_process.start_token.clone()),
        executable_path: reviewed_tree
            .as_ref()
            .and_then(|tree| tree.root_process.executable_path.clone()),
        owner_generation: reviewed_tree.as_ref().map(|tree| tree.owner_generation),
        profile_identity_digest: reviewed_tree
            .as_ref()
            .map(|tree| tree.profile_identity_digest.clone()),
        package_launch_identity_digest: reviewed_tree
            .as_ref()
            .map(|tree| tree.package_launch_identity_digest.clone()),
        kind: resource_kind_name(&kind).to_string(),
        action: action.clone(),
        command_digest: command_digest(&process.command),
        browser_id: correlation.browser_id.clone(),
        profile_id: correlation.profile_id.clone(),
        display_allocation_id: correlation.display_allocation_id.clone(),
        profile_path: correlation.profile_path.clone(),
    });

    Some(ResourceRecord {
        pid: process.pid,
        ppid: process.ppid,
        process_group_id: process.process_group_id,
        executable: process.executable,
        command_preview: sanitize_command_preview(&process.command),
        kind,
        correlation,
        rss_bytes: process.rss_bytes,
        cpu_seconds: process.cpu_seconds,
        age_seconds: process.age_seconds,
        disposition,
        reasons,
        gc_action,
        candidate_identity,
    })
}

fn summarize_resources(state: &ServiceState, records: &[ResourceRecord]) -> ResourceSummary {
    let lifecycle_records = state.runtime_owner_registry.lifecycle_records.values();
    ResourceSummary {
        total_processes: records.len(),
        correlated_processes: records
            .iter()
            .filter(|record| {
                record.correlation.browser_id.is_some()
                    || record.correlation.profile_id.is_some()
                    || record.correlation.display_allocation_id.is_some()
            })
            .count(),
        candidate_count: records
            .iter()
            .filter(|record| record.disposition == ResourceDisposition::Candidate)
            .count(),
        protected_count: records
            .iter()
            .filter(|record| record.disposition == ResourceDisposition::Protected)
            .count(),
        observed_count: records
            .iter()
            .filter(|record| record.disposition == ResourceDisposition::Observed)
            .count(),
        candidate_rss_bytes: records
            .iter()
            .filter(|record| record.disposition == ResourceDisposition::Candidate)
            .filter_map(|record| record.rss_bytes)
            .sum(),
        protected_rss_bytes: records
            .iter()
            .filter(|record| record.disposition == ResourceDisposition::Protected)
            .filter_map(|record| record.rss_bytes)
            .sum(),
        observed_rss_bytes: records
            .iter()
            .filter(|record| record.disposition == ResourceDisposition::Observed)
            .filter_map(|record| record.rss_bytes)
            .sum(),
        total_rss_bytes: records.iter().filter_map(|record| record.rss_bytes).sum(),
        managed_lane_count: state.runtime_owner_registry.lifecycle_records.len(),
        cleanup_obligations_owned: lifecycle_records
            .clone()
            .filter(|record| {
                record.cleanup_obligation_state
                    == crate::runtime_owner_transfer::CleanupObligationState::Owned
            })
            .count(),
        cleanup_obligations_transferring: lifecycle_records
            .clone()
            .filter(|record| {
                record.cleanup_obligation_state
                    == crate::runtime_owner_transfer::CleanupObligationState::Transferring
            })
            .count(),
        cleanup_obligations_satisfied: lifecycle_records
            .clone()
            .filter(|record| {
                record.cleanup_obligation_state
                    == crate::runtime_owner_transfer::CleanupObligationState::Satisfied
            })
            .count(),
        cleanup_obligations_unknown: lifecycle_records
            .filter(|record| {
                record.cleanup_obligation_state
                    == crate::runtime_owner_transfer::CleanupObligationState::Unknown
            })
            .count(),
    }
}

fn correlate_process(
    state: &ServiceState,
    process: &ProcessSample,
    profile_path: Option<String>,
    cdp_port: Option<u16>,
) -> ResourceCorrelation {
    let mut correlation = ResourceCorrelation {
        cdp_port,
        profile_path: profile_path.clone(),
        ..ResourceCorrelation::default()
    };

    if let Some((browser_id, browser)) = state
        .browsers
        .iter()
        .find(|(_, browser)| browser.pid == Some(process.pid))
    {
        correlation.browser_id = Some(browser_id.clone());
        correlation.profile_id = browser.profile_id.clone();
        correlation.session_ids = browser.active_session_ids.clone();
        correlation.display_allocation_id = browser.display_allocation_id.clone();
        correlation.display_name = browser.display_name.clone();
        return correlation;
    }

    if let Some(cdp_port) = cdp_port {
        if let Some((browser_id, browser)) = state.browsers.iter().find(|(_, browser)| {
            browser.cdp_endpoint.as_deref().and_then(port_from_endpoint) == Some(cdp_port)
        }) {
            correlation.browser_id = Some(browser_id.clone());
            correlation.profile_id = browser.profile_id.clone();
            correlation.session_ids = browser.active_session_ids.clone();
            correlation.display_allocation_id = browser.display_allocation_id.clone();
            correlation.display_name = browser.display_name.clone();
        }
    }

    if correlation.profile_id.is_none() {
        if let Some(path) = profile_path.as_deref() {
            if let Some((profile_id, _)) = state.profiles.iter().find(|(_, profile)| {
                profile
                    .user_data_dir
                    .as_deref()
                    .is_some_and(|user_data_dir| same_pathish(user_data_dir, path))
            }) {
                correlation.profile_id = Some(profile_id.clone());
            }
        }
    }

    if correlation.display_allocation_id.is_none() {
        let command_text = process.command.join(" ");
        if let Some((display_id, display)) =
            state.display_allocations.iter().find(|(_, display)| {
                display
                    .display_name
                    .as_deref()
                    .is_some_and(|display_name| command_text.contains(display_name))
            })
        {
            correlation.display_allocation_id = Some(display_id.clone());
            correlation.display_name = display.display_name.clone();
        }
    }

    correlation
}

fn resource_kind(
    process: &ProcessSample,
    profile_path: Option<&str>,
    cdp_port: Option<u16>,
) -> ResourceKind {
    let executable = process
        .executable
        .as_deref()
        .or_else(|| process.command.first().map(String::as_str))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable_name = Path::new(&executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&executable);
    // Process arguments can contain diagnostic prose or commands naming Xvfb.
    // Remote-display authority requires the sampled executable itself.
    if executable_name == "xvfb" {
        return ResourceKind::RemoteDisplay;
    }
    if executable.contains("agent-browser") {
        return ResourceKind::AgentBrowser;
    }
    if executable.contains("chrome")
        || executable.contains("chromium")
        || profile_path.is_some()
        || cdp_port.is_some()
    {
        return ResourceKind::Browser;
    }
    ResourceKind::Other
}

fn installed_agent_browser_runtime_surface(process: &ProcessSample) -> bool {
    let executable = process
        .executable
        .as_deref()
        .or_else(|| process.command.first().map(String::as_str))
        .unwrap_or_default();
    executable.ends_with("/.local/bin/agent-browser")
        || executable.contains("/.local/lib/agent-browser/generations/")
        || executable.contains("/.local/lib/agent-browser-dev/generations/")
}

fn resource_kind_name(kind: &ResourceKind) -> &'static str {
    match kind {
        ResourceKind::AgentBrowser => "agent_browser",
        ResourceKind::Browser => "browser",
        ResourceKind::RemoteDisplay => "remote_display",
        ResourceKind::Other => "other",
    }
}

fn candidates_from_response(response: &Value) -> Vec<Value> {
    response
        .get("resources")
        .and_then(Value::as_array)
        .map(|resources| {
            resources
                .iter()
                .filter(|resource| {
                    resource.get("disposition").and_then(Value::as_str) == Some("candidate")
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn projected_rss_bytes(candidates: &[Value]) -> u64 {
    candidates
        .iter()
        .filter_map(|candidate| candidate.get("rssBytes").and_then(Value::as_u64))
        .sum::<u64>()
}

fn review_token_for_candidates(candidates: &[Value], issued_at: u64) -> String {
    format!(
        "abgc1:{issued_at}:{}",
        candidate_digest_from_values(candidates)
    )
}

fn validate_review_token(candidates: &[Value], token: &str, now: u64) -> Result<(), String> {
    let mut parts = token.split(':');
    let Some(prefix) = parts.next() else {
        return Err("missing_review_token_prefix".to_string());
    };
    let Some(issued_at) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
        return Err("invalid_review_token_timestamp".to_string());
    };
    let Some(digest) = parts.next() else {
        return Err("missing_review_token_digest".to_string());
    };
    if prefix != "abgc1" || parts.next().is_some() {
        return Err("invalid_review_token_format".to_string());
    }
    if issued_at > now {
        return Err("review_token_from_future".to_string());
    }
    if now.saturating_sub(issued_at) > GC_REVIEW_TOKEN_TTL_SECONDS {
        return Err("review_token_expired".to_string());
    }
    let expected = candidate_digest_from_values(candidates);
    if digest != expected {
        return Err("review_token_candidate_mismatch".to_string());
    }
    Ok(())
}

fn candidate_digest_from_values(candidates: &[Value]) -> String {
    let mut identities = candidates
        .iter()
        .filter_map(|candidate| candidate.get("candidateIdentity"))
        .map(canonical_json)
        .collect::<Vec<_>>();
    identities.sort();
    digest_string(identities.join("\n").as_bytes())
}

fn command_digest(command: &[String]) -> String {
    digest_string(command.join("\0").as_bytes())
}

fn digest_string(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn unix_now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

trait ProcessTerminator {
    fn terminate(&self, state: &ServiceState, candidate: &Value) -> Value;
}

trait ProcessInspector {
    fn sample(&self, pid: u32) -> Option<ProcessSample>;
}

struct LiveInspector;
struct LiveTerminator;

impl ProcessInspector for LiveInspector {
    fn sample(&self, pid: u32) -> Option<ProcessSample> {
        live_process_sample(pid)
    }
}

impl ProcessTerminator for LiveTerminator {
    fn terminate(&self, state: &ServiceState, candidate: &Value) -> Value {
        terminate_reviewed_candidate(state, candidate)
    }
}

fn service_gc_apply_response_from_samples(
    state: &mut ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
    review_token: Option<&str>,
    force_without_review: bool,
    inspector: &dyn ProcessInspector,
    terminator: &dyn ProcessTerminator,
) -> Value {
    let resources_response =
        service_resources_response_from_samples(state, processes, collection_warnings);
    let candidates = candidates_from_response(&resources_response);
    let now = unix_now_seconds();
    let token_status = if force_without_review {
        json!({
            "accepted": true,
            "mode": "force_without_review",
        })
    } else if let Some(token) = review_token {
        match validate_review_token(&candidates, token, now) {
            Ok(()) => json!({
                "accepted": true,
                "mode": "review_token",
            }),
            Err(reason) => {
                return json!({
                    "dryRun": false,
                    "apply": true,
                    "applied": false,
                    "candidateCount": candidates.len(),
                    "error": reason,
                    "token": {
                        "accepted": false,
                    },
                    "recommendedNextStep": "Run service gc --dry-run --json again, review the candidates, then rerun apply with the fresh reviewToken.",
                });
            }
        }
    } else {
        return json!({
            "dryRun": false,
            "apply": true,
            "applied": false,
            "candidateCount": candidates.len(),
            "error": "review_token_required",
            "token": {
                "accepted": false,
            },
            "recommendedNextStep": "Run service gc --dry-run --json, review the candidates, then rerun apply with --review-token <token> or --force-without-review.",
        });
    };

    let mut terminated = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    for candidate in &candidates {
        let Some(identity) = candidate.get("candidateIdentity").cloned() else {
            skipped.push(json!({
                "pid": candidate.get("pid").cloned().unwrap_or(Value::Null),
                "reason": "missing_candidate_identity",
            }));
            continue;
        };
        if !candidate_identity_still_matches(state, candidate, &identity, inspector) {
            skipped.push(json!({
                "pid": candidate.get("pid").cloned().unwrap_or(Value::Null),
                "reason": "candidate_identity_changed",
                "candidateIdentity": identity,
            }));
            continue;
        }
        let outcome = terminator.terminate(state, candidate);
        match outcome.get("status").and_then(Value::as_str) {
            Some("terminated") | Some("already_exited") => terminated.push(json!({
                "pid": candidate.get("pid").cloned().unwrap_or(Value::Null),
                "kind": candidate.get("kind").cloned().unwrap_or(Value::Null),
                "gcAction": candidate.get("gcAction").cloned().unwrap_or(Value::Null),
                "outcome": outcome,
            })),
            Some("skipped") => skipped.push(json!({
                "pid": candidate.get("pid").cloned().unwrap_or(Value::Null),
                "outcome": outcome,
            })),
            _ => failed.push(json!({
                "pid": candidate.get("pid").cloned().unwrap_or(Value::Null),
                "outcome": outcome,
            })),
        }
    }

    let response = json!({
        "dryRun": false,
        "apply": true,
        "applied": failed.is_empty(),
        "candidateCount": candidates.len(),
        "token": token_status,
        "counts": {
            "terminated": terminated.len(),
            "skipped": skipped.len(),
            "failed": failed.len(),
        },
        "terminated": terminated,
        "skipped": skipped,
        "failed": failed,
        "projectedReclaimed": {
            "rssBytes": projected_rss_bytes(&candidates),
        },
        "warnings": resources_response.get("warnings").cloned().unwrap_or_else(|| json!([])),
    });
    append_gc_event(state, &response);
    response
}

fn service_gc_unattended_response_from_samples(
    state: &mut ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
    inspector: &dyn ProcessInspector,
    terminator: &dyn ProcessTerminator,
) -> Value {
    let resources = service_resources_response_from_samples(
        state,
        processes.clone(),
        collection_warnings.clone(),
    );
    let candidates = candidates_from_response(&resources);
    let token = review_token_for_candidates(&candidates, unix_now_seconds());
    let mut response = service_gc_apply_response_from_samples(
        state,
        processes,
        collection_warnings,
        Some(&token),
        false,
        inspector,
        terminator,
    );
    response["authority"] = json!("unattended_policy");
    response["reviewRequired"] = json!(false);
    if let Some(token) = response.get_mut("token") {
        token["mode"] = json!("unattended_policy");
    }
    response
}

fn candidate_identity_still_matches(
    state: &ServiceState,
    candidate: &Value,
    expected_identity: &Value,
    inspector: &dyn ProcessInspector,
) -> bool {
    let Some(pid) = candidate
        .get("pid")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
    else {
        return false;
    };
    let Some(sample) = inspector.sample(pid) else {
        return false;
    };
    let dashboard_main_pid = current_dashboard_main_pid();
    classify_process(
        state,
        dashboard_main_pid,
        sample,
        &ResourceRuntimeEnvironment::current(),
    )
    .is_some_and(|current| {
        current.disposition == ResourceDisposition::Candidate
            && current
                .candidate_identity
                .as_ref()
                .and_then(|identity| serde_json::to_value(identity).ok())
                .as_ref()
                == Some(expected_identity)
    })
}

fn append_gc_event(state: &mut ServiceState, response: &Value) {
    let timestamp = chrono::Utc::now().to_rfc3339();
    state.events.push(ServiceEvent {
        id: format!("event:resource-gc:{}", unix_now_seconds()),
        timestamp,
        kind: ServiceEventKind::Reconciliation,
        message: "Service resource GC apply completed".to_string(),
        details: Some(json!({
            "resourceGc": {
                "candidateCount": response.get("candidateCount").cloned().unwrap_or(Value::Null),
                "counts": response.get("counts").cloned().unwrap_or(Value::Null),
                "tokenMode": response
                    .get("token")
                    .and_then(|token| token.get("mode"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "projectedReclaimed": response.get("projectedReclaimed").cloned().unwrap_or(Value::Null),
            }
        })),
        ..ServiceEvent::default()
    });
}

fn live_process_sample(pid: u32) -> Option<ProcessSample> {
    #[cfg(target_os = "linux")]
    {
        linux_process_sample(
            pid,
            linux_boot_time_seconds(),
            linux_uptime_seconds(),
            linux_clock_ticks(),
            linux_boot_id(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

fn terminate_reviewed_candidate(state: &ServiceState, candidate: &Value) -> Value {
    let Some(identity_value) = candidate.get("candidateIdentity").cloned() else {
        return json!({ "status": "skipped", "reason": "missing_candidate_identity" });
    };
    let Ok(identity) = serde_json::from_value::<GcCandidateIdentity>(identity_value.clone()) else {
        return json!({ "status": "skipped", "reason": "invalid_candidate_identity" });
    };
    let (
        Some(process_group_id),
        Some(start_token),
        Some(executable_path),
        Some(owner_generation),
        Some(profile_identity_digest),
        Some(package_launch_identity_digest),
        Some(logical_browser_id),
        Some(profile_root),
    ) = (
        identity.process_group_id,
        identity.start_token,
        identity.executable_path,
        identity.owner_generation,
        identity.profile_identity_digest,
        identity.package_launch_identity_digest,
        identity.browser_id,
        identity.profile_path,
    )
    else {
        return json!({ "status": "skipped", "reason": "incomplete_reviewed_process_tree" });
    };
    let reviewed = crate::native::runtime_reconciliation::ReviewedProcessTree {
        root_process: crate::process_identity::RecordedProcessIdentity {
            pid: identity.pid,
            start_token,
            browser_family: crate::process_identity::browser_family_for_path(Some(Path::new(
                &executable_path,
            ))),
            executable_path: Some(executable_path),
        },
        process_group_id,
        logical_browser_id,
        profile_identity_digest,
        owner_generation,
        package_launch_identity_digest,
    };
    let mut runtime = LiveGcProcessTreeRuntime {
        state,
        candidate,
        expected_identity: identity_value,
    };
    let outcome = crate::native::runtime_reconciliation::shutdown_reviewed_process_tree(
        &reviewed,
        Path::new(&profile_root),
        &mut runtime,
    );
    if let Some(reason) = outcome.blocked_reason {
        return json!({ "status": "skipped", "reason": reason });
    }
    if !outcome.errors.is_empty() || !outcome.exact_process_exited || !outcome.profile_lock_released
    {
        return json!({
            "status": "failed",
            "reason": if outcome.errors.is_empty() {
                "process_tree_or_profile_lock_not_terminal"
            } else {
                "process_tree_shutdown_failed"
            },
            "errors": outcome.errors,
            "exactProcessExited": outcome.exact_process_exited,
            "profileLockReleased": outcome.profile_lock_released,
        });
    }
    json!({
        "status": "terminated",
        "signal": if outcome.kill_sent { "SIGKILL" } else { "SIGTERM" },
        "processGroupId": process_group_id,
        "exactProcessExited": true,
        "profileLockReleased": true,
    })
}

struct LiveGcProcessTreeRuntime<'a> {
    state: &'a ServiceState,
    candidate: &'a Value,
    expected_identity: Value,
}

impl crate::native::runtime_reconciliation::ReviewedProcessTreeRuntime
    for LiveGcProcessTreeRuntime<'_>
{
    fn recheck(
        &mut self,
        _reviewed: &crate::native::runtime_reconciliation::ReviewedProcessTree,
    ) -> Result<(), String> {
        candidate_identity_still_matches(
            self.state,
            self.candidate,
            &self.expected_identity,
            &LiveInspector,
        )
        .then_some(())
        .ok_or_else(|| "candidate_identity_changed".to_string())
    }

    fn signal_group(
        &mut self,
        process_group_id: u32,
        signal: crate::native::runtime_reconciliation::ProcessTreeSignal,
    ) -> Result<(), String> {
        #[cfg(unix)]
        {
            let signal = match signal {
                crate::native::runtime_reconciliation::ProcessTreeSignal::Terminate => {
                    libc::SIGTERM
                }
                crate::native::runtime_reconciliation::ProcessTreeSignal::Kill => libc::SIGKILL,
            };
            let result = unsafe { libc::kill(-(process_group_id as i32), signal) };
            if result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                Ok(())
            } else {
                Err(format!(
                    "process_group_signal_failed: {}",
                    std::io::Error::last_os_error()
                ))
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (process_group_id, signal);
            Err("process_group_shutdown_unsupported".to_string())
        }
    }

    fn wait_after_signal(&mut self) {
        thread::sleep(Duration::from_millis(GC_TERM_WAIT_MS));
    }

    fn process_exited(&mut self, root_pid: u32) -> Result<bool, String> {
        let process_group_id = self
            .expected_identity
            .get("processGroupId")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| "candidate_process_group_missing".to_string())?;
        Ok(!pid_is_running(root_pid as i32) && !process_group_is_running(process_group_id))
    }

    fn profile_lock_released(&mut self, profile_root: &Path) -> Result<bool, String> {
        let lock_path = profile_root.join("SingletonLock");
        match fs::symlink_metadata(&lock_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Ok(_) => {
                fs::remove_file(&lock_path)
                    .map_err(|error| format!("stale_profile_lock_cleanup_failed: {error}"))?;
                Ok(true)
            }
            Err(error) => Err(format!("profile_lock_observation_failed: {error}")),
        }
    }
}

#[cfg(unix)]
fn process_group_is_running(process_group_id: u32) -> bool {
    let result = unsafe { libc::kill(-(process_group_id as i32), 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
fn process_group_is_running(_process_group_id: u32) -> bool {
    false
}

#[cfg(unix)]
fn pid_is_running(pid: i32) -> bool {
    #[cfg(target_os = "linux")]
    {
        if fs::read_to_string(format!("/proc/{pid}/status"))
            .ok()
            .and_then(|status| {
                status.lines().find_map(|line| {
                    line.strip_prefix("State:")
                        .and_then(|value| value.split_whitespace().next())
                        .map(|state| state == "Z")
                })
            })
            .unwrap_or(false)
        {
            return false;
        }
    }
    unsafe { libc::kill(pid, 0) == 0 }
}

fn retained_browser_pid_is_active(state: &ServiceState, browser_id: Option<&str>) -> bool {
    let Some(browser_id) = browser_id else {
        return false;
    };
    state.browsers.get(browser_id).is_some_and(|browser| {
        !matches!(
            browser.health,
            BrowserHealth::NotStarted | BrowserHealth::ProcessExited | BrowserHealth::Faulted
        )
    })
}

fn retained_profile_is_named_or_persistent(state: &ServiceState, profile_id: &str) -> bool {
    state.profiles.get(profile_id).is_some_and(|profile| {
        profile.persistent
            || !profile.name.trim().is_empty()
            || profile
                .user_data_dir
                .as_deref()
                .is_some_and(|path| !temporary_profile_path(Some(path)))
    })
}

fn temporary_profile_path(path: Option<&str>) -> bool {
    let Some(path) = path else {
        return false;
    };
    path.starts_with("/tmp/")
        || path.contains("/tmp/")
        || path.contains("agent-browser-plan")
        || path.contains("agent-browser-smoke")
        || path.contains("chromium-stealthcdp")
}

fn command_arg_value(command: &[String], flag: &str) -> Option<String> {
    let rewritten;
    let command = if command.len() == 1 && command[0].contains(char::is_whitespace) {
        rewritten = command[0]
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        rewritten.as_slice()
    } else {
        command
    };
    for (index, arg) in command.iter().enumerate() {
        if arg == flag {
            return command.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Some(value.to_string());
        }
    }
    None
}

fn sanitize_command_preview(command: &[String]) -> String {
    command
        .iter()
        .take(16)
        .map(|arg| {
            if arg.contains("token=")
                || arg.contains("password")
                || arg.contains("secret")
                || arg.contains("Authorization")
            {
                "<redacted>".to_string()
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn port_from_endpoint(endpoint: &str) -> Option<u16> {
    endpoint
        .split('/')
        .next()
        .and_then(|host_port| {
            host_port
                .rsplit_once(':')
                .map(|(_, port)| port)
                .or(Some(host_port))
        })
        .and_then(|port| port.parse::<u16>().ok())
}

fn same_pathish(a: &str, b: &str) -> bool {
    let a = a.trim_end_matches('/');
    let b = b.trim_end_matches('/');
    a == b || a.ends_with(b) || b.ends_with(a)
}

fn collect_process_samples() -> (Vec<ProcessSample>, Vec<String>) {
    #[cfg(target_os = "linux")]
    {
        linux_collect_process_samples()
    }
    #[cfg(not(target_os = "linux"))]
    {
        (
            Vec::new(),
            vec!["process_table_unavailable_on_this_platform".to_string()],
        )
    }
}

fn current_dashboard_main_pid() -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("systemctl")
            .args([
                "--user",
                "show",
                "agent-browser-dashboard.service",
                "--property=MainPID",
                "--value",
            ])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|value| value.trim().parse::<u32>().ok())
            .filter(|pid| *pid > 0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn linux_collect_process_samples() -> (Vec<ProcessSample>, Vec<String>) {
    let mut warnings = Vec::new();
    let boot_time_seconds = linux_boot_time_seconds();
    let uptime_seconds = linux_uptime_seconds();
    let clock_ticks = linux_clock_ticks();
    let boot_id = linux_boot_id();
    let mut samples = Vec::new();
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(err) => {
            return (
                Vec::new(),
                vec![format!("process_table_read_failed: {}", err)],
            )
        }
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        match linux_process_sample(
            pid,
            boot_time_seconds,
            uptime_seconds,
            clock_ticks,
            boot_id.clone(),
        ) {
            Some(sample) => samples.push(sample),
            None => continue,
        }
    }
    if samples.is_empty() {
        warnings.push("process_table_empty".to_string());
    }
    (samples, warnings)
}

#[cfg(target_os = "linux")]
fn linux_process_sample(
    pid: u32,
    boot_time_seconds: Option<u64>,
    uptime_seconds: Option<u64>,
    clock_ticks: u64,
    boot_id: Option<String>,
) -> Option<ProcessSample> {
    let proc_path = format!("/proc/{pid}");
    let command = fs::read(format!("{proc_path}/cmdline"))
        .ok()
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part).to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let stat = fs::read_to_string(format!("{proc_path}/stat")).ok()?;
    let stat_tail = stat.rsplit_once(") ")?.1;
    let fields = stat_tail.split_whitespace().collect::<Vec<_>>();
    let ppid = fields.get(1).and_then(|value| value.parse::<u32>().ok());
    let process_group_id = fields.get(2).and_then(|value| value.parse::<u32>().ok());
    let utime = fields
        .get(11)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let stime = fields
        .get(12)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let start_ticks = fields.get(19).and_then(|value| value.parse::<u64>().ok());
    let start_token = match (boot_id, start_ticks) {
        (Some(boot_id), Some(start_ticks)) => Some(format!("linux:{boot_id}:{start_ticks}")),
        _ => None,
    };
    let cpu_seconds = Some((utime + stime) / clock_ticks.max(1));
    let age_seconds = match (boot_time_seconds, uptime_seconds, start_ticks) {
        (Some(_), Some(uptime), Some(start_ticks)) => {
            let started_after_boot = start_ticks / clock_ticks.max(1);
            uptime.checked_sub(started_after_boot)
        }
        _ => None,
    };
    let rss_bytes = fs::read_to_string(format!("{proc_path}/status"))
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                let value = line.strip_prefix("VmRSS:")?.split_whitespace().next()?;
                value.parse::<u64>().ok().map(|kib| kib * 1024)
            })
        });
    let executable = fs::read_link(format!("{proc_path}/exe"))
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
        .or_else(|| command.first().cloned());
    Some(ProcessSample {
        pid,
        ppid,
        process_group_id,
        start_token,
        executable,
        command,
        rss_bytes,
        cpu_seconds,
        age_seconds,
    })
}

#[cfg(target_os = "linux")]
fn linux_boot_id() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(target_os = "linux")]
fn linux_boot_time_seconds() -> Option<u64> {
    fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("btime ")
                .and_then(|value| value.trim().parse::<u64>().ok())
        })
}

#[cfg(target_os = "linux")]
fn linux_uptime_seconds() -> Option<u64> {
    fs::read_to_string("/proc/uptime")
        .ok()?
        .split_whitespace()
        .next()?
        .split('.')
        .next()?
        .parse::<u64>()
        .ok()
}

#[cfg(target_os = "linux")]
fn linux_clock_ticks() -> u64 {
    unsafe {
        let ticks = libc::sysconf(libc::_SC_CLK_TCK);
        if ticks > 0 {
            return ticks as u64;
        }
    }
    100
}

#[cfg(test)]
mod tests {
    use super::super::service_model::{
        BrowserHost, BrowserProcess, BrowserProfile, BrowserSession, DisplayAllocation, LeaseState,
        ServiceBrowserProcessIdentity,
    };
    use super::*;

    fn sample(pid: u32, command: &[&str], age_seconds: Option<u64>) -> ProcessSample {
        ProcessSample {
            pid,
            command: command.iter().map(|value| value.to_string()).collect(),
            executable: command.first().map(|value| value.to_string()),
            age_seconds,
            rss_bytes: Some(10),
            ..ProcessSample::default()
        }
    }

    #[test]
    fn resource_summary_separates_protected_reclaimable_and_unowned_rss() {
        let records = vec![
            ResourceRecord {
                pid: 101,
                rss_bytes: Some(1_024),
                disposition: ResourceDisposition::Protected,
                ..ResourceRecord::default()
            },
            ResourceRecord {
                pid: 102,
                rss_bytes: Some(2_048),
                disposition: ResourceDisposition::Candidate,
                ..ResourceRecord::default()
            },
            ResourceRecord {
                pid: 103,
                rss_bytes: Some(4_096),
                disposition: ResourceDisposition::Observed,
                ..ResourceRecord::default()
            },
        ];

        let summary = summarize_resources(&ServiceState::default(), &records);

        assert_eq!(summary.protected_rss_bytes, 1_024);
        assert_eq!(summary.candidate_rss_bytes, 2_048);
        assert_eq!(summary.observed_rss_bytes, 4_096);
        assert_eq!(summary.total_rss_bytes, 7_168);
    }

    fn owned_closing_candidate(pid: u32, profile_root: &str) -> (ServiceState, ProcessSample) {
        let executable = "/opt/agent-browser/chromium";
        let start_token = format!("linux:fixture:{pid}");
        let recorded = crate::process_identity::RecordedProcessIdentity {
            pid,
            start_token: start_token.clone(),
            executable_path: Some(executable.to_string()),
            browser_family: Some("chromium".to_string()),
        };
        let profile_identity_digest =
            crate::runtime_profile::canonical_profile_identity_digest(Path::new(profile_root))
                .unwrap();
        let owner = crate::runtime_owner_transfer::ProfileOwner {
            owner_id: format!("owner-{pid}"),
            profile_identity_digest: profile_identity_digest.clone(),
            state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
            owner_generation: 4,
            browser_id: format!("browser-{pid}"),
            daemon_session_route: format!("session-{pid}"),
            process_instance_digest: crate::native::runtime_lifecycle::digest_json(&recorded)
                .unwrap(),
            browser_family: "chromium".to_string(),
            cdp_endpoint_identity_digest: "c".repeat(64),
            target_set_digest: "d".repeat(64),
            pending_transfer: None,
            last_transition: None,
        };
        let package_launch_identity_digest =
            crate::native::runtime_lifecycle::package_launch_identity_digest(&owner, Some(pid))
                .unwrap();
        let mut state = ServiceState::default();
        state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(owner.clone());
        state.runtime_owner_registry.lifecycle_records.insert(
            owner.browser_id.clone(),
            crate::runtime_owner_transfer::RuntimeLifecycleRecord {
                logical_browser_id: owner.browser_id.clone(),
                profile_identity_digest,
                owner_generation: owner.owner_generation,
                lifecycle_state: crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Closing,
                cleanup_obligation_state:
                    crate::runtime_owner_transfer::CleanupObligationState::Owned,
                process_group_id: Some(pid),
                package_launch_identity_digest: Some(package_launch_identity_digest),
                terminal_evidence: Vec::new(),
            },
        );
        state.browsers.insert(
            owner.browser_id.clone(),
            BrowserProcess {
                id: owner.browser_id.clone(),
                host: BrowserHost::LocalHeadless,
                health: BrowserHealth::Faulted,
                pid: Some(pid),
                ..BrowserProcess::default()
            },
        );
        state.browser_process_identities.insert(
            owner.browser_id,
            ServiceBrowserProcessIdentity {
                process_identity: recorded,
                user_data_dir: Some(profile_root.to_string()),
                runtime_profile: None,
            },
        );
        let candidate = ProcessSample {
            pid,
            ppid: Some(1),
            process_group_id: Some(pid),
            start_token: Some(start_token),
            executable: Some(executable.to_string()),
            command: vec![
                executable.to_string(),
                format!("--user-data-dir={profile_root}"),
            ],
            rss_bytes: Some(1024),
            cpu_seconds: Some(1),
            age_seconds: Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
        };
        (state, candidate)
    }

    #[test]
    fn resources_protect_retained_active_browser_profile() {
        let mut state = ServiceState::default();
        state.profiles.insert(
            "default".to_string(),
            BrowserProfile {
                id: "default".to_string(),
                name: "Default".to_string(),
                user_data_dir: Some("/home/me/.agent-browser/runtime-profiles/default".to_string()),
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        state.browsers.insert(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("default".to_string()),
                host: BrowserHost::LocalHeaded,
                health: BrowserHealth::Ready,
                pid: Some(101),
                ..BrowserProcess::default()
            },
        );
        let response = service_resources_response_from_samples(
            &state,
            vec![sample(
                101,
                &[
                    "chrome",
                    "--user-data-dir=/home/me/.agent-browser/runtime-profiles/default",
                ],
                Some(3600),
            )],
            Vec::new(),
        );
        assert_eq!(response["summary"]["protectedCount"], 1);
        assert_eq!(response["resources"][0]["disposition"], "protected");
        assert_eq!(
            response["resources"][0]["correlation"]["browserId"],
            "browser-1"
        );
    }

    #[test]
    fn resources_warn_about_duplicate_live_browsers_for_profile() {
        let mut state = ServiceState::default();
        state.profiles.insert(
            "work".to_string(),
            BrowserProfile {
                id: "work".to_string(),
                name: "Work".to_string(),
                target_service_ids: vec!["canva".to_string()],
                account_ids: vec!["acct-1".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        for (id, pid) in [("browser-a", 101), ("browser-b", 102)] {
            state.browsers.insert(
                id.to_string(),
                BrowserProcess {
                    id: id.to_string(),
                    profile_id: Some("work".to_string()),
                    host: BrowserHost::LocalHeaded,
                    health: BrowserHealth::Ready,
                    display_isolation: Some("private_virtual_display".to_string()),
                    pid: Some(pid),
                    ..BrowserProcess::default()
                },
            );
        }

        let response = service_resources_response_from_samples(
            &state,
            vec![
                sample(
                    101,
                    &["chrome", "--user-data-dir=/profiles/work-a"],
                    Some(3600),
                ),
                sample(
                    102,
                    &["chrome", "--user-data-dir=/profiles/work-b"],
                    Some(3600),
                ),
            ],
            Vec::new(),
        );

        let warnings = response["warnings"].as_array().unwrap();
        let warning = warnings
            .iter()
            .find(|warning| warning["code"] == "duplicate_live_browsers_for_profile")
            .expect("duplicate live browser warning should be reported");
        assert_eq!(warning["profileId"], "work");
        assert_eq!(warning["count"], 2);
        assert_eq!(warning["browserIds"][0], "browser-a");
        assert_eq!(warning["browserIds"][1], "browser-b");
    }

    #[test]
    fn resources_warn_about_duplicate_active_profile_leases() {
        let mut state = ServiceState::default();
        for id in ["session-a", "session-b"] {
            state.sessions.insert(
                id.to_string(),
                BrowserSession {
                    id: id.to_string(),
                    profile_id: Some("work".to_string()),
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            );
        }

        let response = service_resources_response_from_samples(&state, Vec::new(), Vec::new());
        let warnings = response["warnings"].as_array().unwrap();
        let warning = warnings
            .iter()
            .find(|warning| warning["code"] == "duplicate_active_profile_leases")
            .expect("duplicate active lease warning should be reported");
        assert_eq!(warning["profileId"], "work");
        assert_eq!(warning["count"], 2);
        assert_eq!(warning["sessionIds"][0], "session-a");
        assert_eq!(warning["sessionIds"][1], "session-b");
    }

    #[test]
    fn resources_allow_multiple_active_sessions_on_same_live_browser() {
        let mut state = ServiceState::default();
        state.browsers.insert(
            "browser-shared".to_string(),
            BrowserProcess {
                id: "browser-shared".to_string(),
                profile_id: Some("work".to_string()),
                health: BrowserHealth::Ready,
                ..BrowserProcess::default()
            },
        );
        for id in ["session-a", "session-b"] {
            state.sessions.insert(
                id.to_string(),
                BrowserSession {
                    id: id.to_string(),
                    profile_id: Some("work".to_string()),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec!["browser-shared".to_string()],
                    ..BrowserSession::default()
                },
            );
        }

        let response = service_resources_response_from_samples(&state, Vec::new(), Vec::new());
        let warnings = response["warnings"].as_array().unwrap();
        assert!(!warnings
            .iter()
            .any(|warning| warning["code"] == "duplicate_active_profile_leases"));
    }

    #[test]
    fn resources_project_lifecycle_cleanup_accountability() {
        let mut state = ServiceState::default();
        state.runtime_owner_registry.lifecycle_records.insert(
            "browser-owned".to_string(),
            crate::runtime_owner_transfer::RuntimeLifecycleRecord {
                logical_browser_id: "browser-owned".to_string(),
                profile_identity_digest: "a".repeat(64),
                owner_generation: 3,
                lifecycle_state: crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Retained,
                cleanup_obligation_state:
                    crate::runtime_owner_transfer::CleanupObligationState::Owned,
                process_group_id: Some(4100),
                package_launch_identity_digest: Some("b".repeat(64)),
                terminal_evidence: Vec::new(),
            },
        );

        let response = service_resources_response_from_samples(&state, Vec::new(), Vec::new());

        assert_eq!(response["summary"]["managedLaneCount"], 1);
        assert_eq!(response["summary"]["cleanupObligationsOwned"], 1);
        assert_eq!(
            response["runtimeLanes"][0]["logicalBrowserId"],
            "browser-owned"
        );
        assert_eq!(response["runtimeLanes"][0]["lifecycleState"], "retained");
    }

    #[test]
    fn resources_protect_old_temporary_profile_without_exact_lifecycle_ownership() {
        let response = service_resources_response_from_samples(
            &ServiceState::default(),
            vec![sample(
                202,
                &["chromium", "--user-data-dir=/tmp/agent-browser-plan0026"],
                Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
            )],
            Vec::new(),
        );
        assert_eq!(response["summary"]["candidateCount"], 0);
        assert_eq!(response["resources"][0]["disposition"], "protected");
        assert_eq!(
            response["resources"][0]["reasons"][0],
            "runtime_lifecycle_browser_unproven"
        );
    }

    #[test]
    fn rewritten_single_field_chrome_argv_retains_supplementary_flag_evidence() {
        let flattened = vec![
            "chrome --user-data-dir=/tmp/agent-browser-fixture/profile-a --remote-debugging-port=9222"
                .to_string(),
        ];

        assert_eq!(
            command_arg_value(&flattened, "--user-data-dir").as_deref(),
            Some("/tmp/agent-browser-fixture/profile-a")
        );
        assert_eq!(
            command_arg_value(&flattened, "--remote-debugging-port").as_deref(),
            Some("9222")
        );
    }

    #[test]
    fn exact_owned_gc_candidate_targets_the_reviewed_process_tree() {
        let (state, candidate_sample) =
            owned_closing_candidate(4200, "/tmp/agent-browser-fixture/profile-b");
        let response =
            service_resources_response_from_samples(&state, vec![candidate_sample], Vec::new());

        let candidate = &response["resources"][0];
        assert_eq!(candidate["candidateIdentity"]["processGroupId"], 4200);
        assert_eq!(candidate["candidateIdentity"]["ownerGeneration"], 4);
        assert_eq!(candidate["pid"], 4200);
        assert_eq!(candidate["gcAction"], "terminate_process_tree");
    }

    #[test]
    fn exact_owned_closing_tree_does_not_delete_its_named_profile_policy() {
        let profile_root =
            "/tmp/ab-managed-resource-gc-fixture/.agent-browser/runtime-profiles/default/user-data";
        let (mut state, candidate_sample) = owned_closing_candidate(4300, profile_root);
        let browser_id = "browser-4300";
        state.profiles.insert(
            "default".to_string(),
            BrowserProfile {
                id: "default".to_string(),
                name: "Default".to_string(),
                user_data_dir: Some(profile_root.to_string()),
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        state.browsers.get_mut(browser_id).unwrap().profile_id = Some("default".to_string());

        let response =
            service_resources_response_from_samples(&state, vec![candidate_sample], Vec::new());

        assert_eq!(response["resources"][0]["disposition"], "candidate");
        assert_eq!(
            response["resources"][0]["gcAction"],
            "terminate_process_tree"
        );
        assert_eq!(
            response["resources"][0]["correlation"]["profileId"],
            "default"
        );
    }

    #[test]
    fn resources_do_not_gc_fresh_temporary_profile() {
        let response = service_resources_response_from_samples(
            &ServiceState::default(),
            vec![sample(
                303,
                &["chromium", "--user-data-dir=/tmp/agent-browser-plan0026"],
                Some(60),
            )],
            Vec::new(),
        );
        assert_eq!(response["summary"]["candidateCount"], 0);
        assert_eq!(response["resources"][0]["disposition"], "observed");
    }

    #[test]
    fn resources_protect_retained_display_allocation_process() {
        let mut state = ServiceState::default();
        state.display_allocations.insert(
            "display:private_virtual_display:session-default".to_string(),
            DisplayAllocation {
                id: "display:private_virtual_display:session-default".to_string(),
                display_name: Some(":107".to_string()),
                ..DisplayAllocation::default()
            },
        );
        let response = service_resources_response_from_samples(
            &state,
            vec![sample(
                505,
                &["/usr/bin/Xvfb", ":107", "-screen", "0", "1280x720x24"],
                Some(3600),
            )],
            Vec::new(),
        );
        assert_eq!(response["summary"]["candidateCount"], 0);
        assert_eq!(response["summary"]["protectedCount"], 1);
        assert_eq!(
            response["resources"][0]["reasons"][0],
            "retained_display_allocation"
        );
    }

    #[test]
    fn development_resources_do_not_claim_uncorrelated_remote_display() {
        let response = service_resources_response_from_samples_for_environment(
            &ServiceState::default(),
            vec![sample(
                506,
                &["/usr/bin/Xvfb", ":92", "-screen", "0", "1280x720x24"],
                Some(3600),
            )],
            Vec::new(),
            ResourceRuntimeEnvironment::Development {
                state_root: PathBuf::from("/home/dev/.agent-browser"),
                install_root: PathBuf::from("/home/dev/.local/lib/agent-browser-dev"),
                socket_dir: PathBuf::from("/run/user/1000/agent-browser-dev"),
            },
        );
        assert_eq!(response["summary"]["candidateCount"], 0);
        assert_eq!(response["resources"][0]["disposition"], "protected");
        assert_eq!(
            response["resources"][0]["reasons"][0],
            "runtime_environment_ownership_unproven"
        );
    }

    #[test]
    fn production_resources_preserve_orphan_remote_display_candidate_behavior() {
        let response = service_resources_response_from_samples_for_environment(
            &ServiceState::default(),
            vec![sample(
                507,
                &["/usr/bin/Xvfb", ":93", "-screen", "0", "1280x720x24"],
                Some(3600),
            )],
            Vec::new(),
            ResourceRuntimeEnvironment::Production,
        );
        assert_eq!(response["summary"]["candidateCount"], 1);
        assert_eq!(response["resources"][0]["disposition"], "candidate");
        assert_eq!(
            response["resources"][0]["reasons"][0],
            "orphaned_remote_display_process"
        );
    }

    #[test]
    fn production_resources_protect_installed_production_and_development_runtime_processes() {
        let response = service_resources_response_from_samples_for_environment(
            &ServiceState::default(),
            vec![
                sample(
                    601,
                    &["/home/dev/.local/lib/agent-browser/generations/current/bin/agent-browser"],
                    Some(60),
                ),
                sample(
                    602,
                    &["/home/dev/.local/lib/agent-browser-dev/generations/current/bin/agent-browser"],
                    Some(60),
                ),
                sample(
                    603,
                    &["/home/dev/.local/bin/agent-browser"],
                    Some(1),
                ),
            ],
            Vec::new(),
            ResourceRuntimeEnvironment::Production,
        );

        assert_eq!(response["summary"]["protectedCount"], 3);
        assert_eq!(response["summary"]["observedCount"], 0);
        for resource in response["resources"].as_array().unwrap() {
            assert_eq!(resource["disposition"], "protected");
            assert_eq!(
                resource["reasons"][0],
                "installed_agent_browser_runtime_surface"
            );
        }
    }

    #[test]
    fn resources_do_not_classify_shell_arguments_as_remote_displays() {
        let response = service_resources_response_from_samples_for_environment(
            &ServiceState::default(),
            vec![sample(
                509,
                &[
                    "/usr/bin/zsh",
                    "-c",
                    "inspect the Xvfb process table without starting a display",
                ],
                Some(5),
            )],
            Vec::new(),
            ResourceRuntimeEnvironment::Production,
        );
        assert_eq!(response["summary"]["candidateCount"], 0);
        assert_eq!(response["resources"], json!([]));
    }

    #[test]
    fn development_resources_keep_exact_owned_lifecycle_candidate() {
        let profile_root = "/home/dev/.agent-browser/runtime-profiles/disposable/user-data";
        let (state, candidate) = owned_closing_candidate(508, profile_root);
        let response = service_resources_response_from_samples_for_environment(
            &state,
            vec![candidate],
            Vec::new(),
            ResourceRuntimeEnvironment::Development {
                state_root: PathBuf::from("/home/dev/.agent-browser"),
                install_root: PathBuf::from("/home/dev/.local/lib/agent-browser-dev"),
                socket_dir: PathBuf::from("/run/user/1000/agent-browser-dev"),
            },
        );
        assert_eq!(response["summary"]["candidateCount"], 1);
        assert_eq!(response["resources"][0]["disposition"], "candidate");
        assert_eq!(
            response["resources"][0]["reasons"][0],
            "lifecycle_owned_closing_process_tree_after_grace"
        );
    }

    #[test]
    fn gc_dry_run_groups_candidates_without_applying() {
        let (state, candidate) = owned_closing_candidate(404, "/tmp/agent-browser-smoke-old");
        let response = service_resources_response_from_samples(&state, vec![candidate], Vec::new());
        let candidates = response["resources"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|resource| resource["disposition"] == "candidate")
            .count();
        assert_eq!(candidates, 1);
    }

    struct FakeInspector {
        sample: Option<ProcessSample>,
    }

    impl ProcessInspector for FakeInspector {
        fn sample(&self, _pid: u32) -> Option<ProcessSample> {
            self.sample.clone()
        }
    }

    struct FakeTerminator;

    impl ProcessTerminator for FakeTerminator {
        fn terminate(&self, _state: &ServiceState, _candidate: &Value) -> Value {
            json!({
                "status": "terminated",
                "signal": "SIGTERM",
            })
        }
    }

    #[test]
    fn gc_apply_requires_matching_review_token() {
        let (mut state, candidate) = owned_closing_candidate(606, "/tmp/agent-browser-plan0026");
        let resources =
            service_resources_response_from_samples(&state, vec![candidate.clone()], Vec::new());
        let candidates = candidates_from_response(&resources);
        let token = review_token_for_candidates(&candidates, unix_now_seconds());
        let response = service_gc_apply_response_from_samples(
            &mut state,
            vec![candidate.clone()],
            Vec::new(),
            Some(&token),
            false,
            &FakeInspector {
                sample: Some(candidate),
            },
            &FakeTerminator,
        );

        assert_eq!(response["applied"], true);
        assert_eq!(response["counts"]["terminated"], 1);
        assert_eq!(state.events.len(), 1);
    }

    #[test]
    fn unattended_gc_uses_the_same_candidate_and_identity_authority() {
        let (mut state, candidate) =
            owned_closing_candidate(607, "/tmp/agent-browser-plan0117-unattended");
        let response = service_gc_unattended_response_from_samples(
            &mut state,
            vec![candidate.clone()],
            Vec::new(),
            &FakeInspector {
                sample: Some(candidate),
            },
            &FakeTerminator,
        );

        assert_eq!(response["authority"], "unattended_policy");
        assert_eq!(response["reviewRequired"], false);
        assert_eq!(response["token"]["mode"], "unattended_policy");
        assert_eq!(response["counts"]["terminated"], 1);
        assert_eq!(state.events.len(), 1);
    }

    #[test]
    fn gc_apply_refuses_changed_candidate_identity() {
        let (mut state, candidate) = owned_closing_candidate(707, "/tmp/agent-browser-plan0026");
        let resources =
            service_resources_response_from_samples(&state, vec![candidate.clone()], Vec::new());
        let candidates = candidates_from_response(&resources);
        let token = review_token_for_candidates(&candidates, unix_now_seconds());
        let mut changed = candidate.clone();
        changed.start_token = Some("linux:fixture:reused".to_string());

        let response = service_gc_apply_response_from_samples(
            &mut state,
            vec![candidate],
            Vec::new(),
            Some(&token),
            false,
            &FakeInspector {
                sample: Some(changed),
            },
            &FakeTerminator,
        );

        assert_eq!(response["counts"]["terminated"], 0);
        assert_eq!(response["counts"]["skipped"], 1);
        assert_eq!(
            response["skipped"][0]["reason"],
            "candidate_identity_changed"
        );
    }

    #[test]
    fn gc_apply_rejects_missing_review_token_without_force() {
        let candidate = sample(
            808,
            &["chromium", "--user-data-dir=/tmp/agent-browser-plan0026"],
            Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
        );
        let mut state = ServiceState::default();

        let response = service_gc_apply_response_from_samples(
            &mut state,
            vec![candidate],
            Vec::new(),
            None,
            false,
            &FakeInspector { sample: None },
            &FakeTerminator,
        );

        assert_eq!(response["applied"], false);
        assert_eq!(response["error"], "review_token_required");
        assert!(state.events.is_empty());
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::runtime::{
        account_ids_from_command, browser_build_from_command, browser_host_from_command,
        is_stale_page_session_error, optional_command_string, parse_control_input_provider,
        parse_view_stream_provider, recover_browser_command_channel, relaunch_and_restore_page,
        remote_headed_display_isolation_from_command, runtime_profile_from_sources,
        service_browser_id, target_service_ids_from_command, target_url_from_command,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_access::access_plan_browser_build_selection_summary;
    use crate::native::service_access::{service_access_plan_for_state, ServiceAccessPlanRequest};
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::service_model::{
        retained_display_allocation_candidates, service_profile_allocations,
        service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
        BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
        BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession,
        BrowserTab, ControlInputProvider, DisplayAllocation, JobState as ServiceJobState,
        LeaseState, MonitorState, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
        ProfileLeaseDisposition, ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease,
        RemoteViewHandoff, RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent,
        ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle,
        ViewStream, ViewStreamProvider, ViewerLease,
    };
    use crate::native::service_resources::{
        service_gc_apply_response, service_gc_dry_run_response,
        service_resources_monitor_summary_response, service_resources_response,
        service_resources_write_monitor_summary_response,
    };
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    use crate::native::state;
    use crate::runtime_profile::{
        clear_runtime_state, looks_like_path, read_devtools_port, read_runtime_state,
        runtime_profile_user_data_dir,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Map, Value};
    use sha2::{Digest, Sha256};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    pub(crate) async fn handle_service_resources(cmd: &Value) -> Result<Value, String> {
        let service_state = load_service_state_for_maintenance(cmd)?;
        Ok(service_resources_response(&service_state))
    }
    pub(crate) async fn handle_service_resources_monitor_summary() -> Result<Value, String> {
        service_resources_monitor_summary_response()
    }
    pub(crate) async fn handle_service_resources_write_monitor_summary(
        cmd: &Value,
    ) -> Result<Value, String> {
        let service_state = load_service_state_for_maintenance(cmd)?;
        service_resources_write_monitor_summary_response(&service_state)
    }
    pub(crate) async fn handle_service_gc(cmd: &Value) -> Result<Value, String> {
        let apply = cmd.get("apply").and_then(Value::as_bool).unwrap_or(false);
        if apply {
            let review_token = cmd.get("reviewToken").and_then(Value::as_str);
            let force_without_review = cmd
                .get("forceWithoutReview")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let repository = LockedServiceStateRepository::default_json()?;
            repository.mutate(|state| {
                let response = service_gc_apply_response(state, review_token, force_without_review);
                if let Some(error) = response.get("error").and_then(Value::as_str) {
                    Err(error.to_string())
                } else {
                    Ok(response)
                }
            })
        } else {
            let service_state = load_service_state_for_maintenance(cmd)?;
            Ok(service_gc_dry_run_response(&service_state))
        }
    }
    pub(crate) fn load_service_state_for_maintenance(cmd: &Value) -> Result<ServiceState, String> {
        if let Some(service_state) = cmd.get("serviceState") {
            serde_json::from_value::<ServiceState>(service_state.clone())
                .map_err(|err| format!("Invalid serviceState: {}", err))
        } else {
            LockedServiceStateRepository::default_json()?.load_snapshot()
        }
    }
    /// Return the no-launch service access plan from the current service state.
    pub(crate) async fn handle_service_access_plan(cmd: &Value) -> Result<Value, String> {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        service_state.refresh_profile_readiness();
        let request = ServiceAccessPlanRequest {
            service_name: optional_command_string(cmd, "serviceName"),
            agent_name: optional_command_string(cmd, "agentName"),
            task_name: optional_command_string(cmd, "taskName"),
            session_name: optional_command_string(cmd, "sessionName"),
            target_service_ids: target_service_ids_from_command(cmd),
            account_ids: account_ids_from_command(cmd),
            target_url: target_url_from_command(cmd),
            site_policy_id: optional_command_string(cmd, "sitePolicyId"),
            challenge_id: optional_command_string(cmd, "challengeId"),
            readiness_profile_id: optional_command_string(cmd, "readinessProfileId"),
            runtime_profile: runtime_profile_from_sources(cmd, false),
            browser_build: browser_build_from_command(cmd),
            browser_build_explicit: cmd.get("browserBuild").and_then(Value::as_str).is_some(),
            browser_host: browser_host_from_command(cmd),
            view_stream_provider: optional_command_string(cmd, "viewStreamProvider")
                .or_else(|| optional_command_string(cmd, "viewStream"))
                .or_else(|| {
                    cmd.get("params").and_then(|params| {
                        optional_command_string(params, "viewStreamProvider")
                            .or_else(|| optional_command_string(params, "viewStream"))
                    })
                })
                .and_then(|value| parse_view_stream_provider(&value)),
            control_input_provider: optional_command_string(cmd, "controlInputProvider")
                .or_else(|| optional_command_string(cmd, "controlInput"))
                .or_else(|| {
                    cmd.get("params").and_then(|params| {
                        optional_command_string(params, "controlInputProvider")
                            .or_else(|| optional_command_string(params, "controlInput"))
                    })
                })
                .and_then(|value| parse_control_input_provider(&value)),
            display_isolation: remote_headed_display_isolation_from_command(cmd),
        };
        let mut plan = service_access_plan_for_state(&service_state, request);
        let summary = access_plan_browser_build_selection_summary(&plan);
        if let Some(object) = plan.as_object_mut() {
            object.insert("browserBuildSelectionSummary".to_string(), summary);
        }
        Ok(plan)
    }
}
pub(crate) use service_commands::*;
