use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::service_model::{
    BrowserHealth, LeaseState, ServiceEvent, ServiceEventKind, ServiceState,
};

const TEMP_PROFILE_MIN_AGE_SECONDS: u64 = 30 * 60;
const GC_REVIEW_TOKEN_TTL_SECONDS: u64 = 10 * 60;
const GC_TERM_WAIT_MS: u64 = 500;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ProcessSample {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub process_group_id: Option<u32>,
    pub executable: Option<String>,
    pub command: Vec<String>,
    pub rss_bytes: Option<u64>,
    pub cpu_seconds: Option<u64>,
    pub age_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct ResourceCorrelation {
    browser_id: Option<String>,
    profile_id: Option<String>,
    session_ids: Vec<String>,
    display_allocation_id: Option<String>,
    display_name: Option<String>,
    cdp_port: Option<u16>,
    profile_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResourceKind {
    AgentBrowser,
    Browser,
    RemoteDisplay,
    #[default]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResourceDisposition {
    Protected,
    Candidate,
    #[default]
    Observed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct ResourceRecord {
    pid: u32,
    ppid: Option<u32>,
    process_group_id: Option<u32>,
    executable: Option<String>,
    command_preview: String,
    kind: ResourceKind,
    correlation: ResourceCorrelation,
    rss_bytes: Option<u64>,
    cpu_seconds: Option<u64>,
    age_seconds: Option<u64>,
    disposition: ResourceDisposition,
    reasons: Vec<String>,
    gc_action: Option<String>,
    candidate_identity: Option<GcCandidateIdentity>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct GcCandidateIdentity {
    pid: u32,
    process_group_id: Option<u32>,
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
struct ResourceSummary {
    total_processes: usize,
    correlated_processes: usize,
    candidate_count: usize,
    protected_count: usize,
    observed_count: usize,
    candidate_rss_bytes: u64,
    total_rss_bytes: u64,
}

pub(crate) fn service_resources_response(state: &ServiceState) -> Value {
    let (processes, collection_warnings) = collect_process_samples();
    service_resources_response_from_samples(state, processes, collection_warnings)
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

fn service_resources_response_from_samples(
    state: &ServiceState,
    processes: Vec<ProcessSample>,
    collection_warnings: Vec<String>,
) -> Value {
    let dashboard_main_pid = current_dashboard_main_pid();
    let mut records = processes
        .into_iter()
        .filter_map(|process| classify_process(state, dashboard_main_pid, process))
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.pid);

    let summary = summarize_resources(&records);
    let warnings = resource_warnings(state, collection_warnings);
    json!({
        "summary": summary,
        "resources": records,
        "warnings": warnings,
        "policy": {
            "protectsDashboardMainPid": dashboard_main_pid.is_some(),
            "protectsRetainedBrowserPids": true,
            "protectsNamedManagedProfiles": true,
            "temporaryProfileMinAgeSeconds": TEMP_PROFILE_MIN_AGE_SECONDS,
            "reviewTokenTtlSeconds": GC_REVIEW_TOKEN_TTL_SECONDS,
            "applySupported": true,
        },
    })
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
) -> Option<ResourceRecord> {
    let command_text = process.command.join(" ");
    let profile_path = command_arg_value(&process.command, "--user-data-dir");
    let cdp_port = command_arg_value(&process.command, "--remote-debugging-port")
        .and_then(|value| value.parse::<u16>().ok());
    let kind = resource_kind(&process, &command_text, profile_path.as_deref(), cdp_port);
    if kind == ResourceKind::Other {
        return None;
    }

    let correlation = correlate_process(state, &process, profile_path, cdp_port);
    let mut disposition = ResourceDisposition::Observed;
    let mut reasons = Vec::new();
    let mut gc_action = None;

    if Some(process.pid) == dashboard_main_pid {
        disposition = ResourceDisposition::Protected;
        reasons.push("dashboard_main_pid".to_string());
    }
    if correlation.browser_id.is_some()
        && retained_browser_pid_is_active(state, correlation.browser_id.as_deref())
    {
        disposition = ResourceDisposition::Protected;
        reasons.push("retained_active_browser".to_string());
    }
    if correlation
        .profile_id
        .as_deref()
        .is_some_and(|profile_id| retained_profile_is_named_or_persistent(state, profile_id))
    {
        disposition = ResourceDisposition::Protected;
        reasons.push("retained_named_or_persistent_profile".to_string());
    }
    if correlation
        .display_allocation_id
        .as_deref()
        .is_some_and(|display_id| state.display_allocations.contains_key(display_id))
    {
        disposition = ResourceDisposition::Protected;
        reasons.push("retained_display_allocation".to_string());
    }

    if disposition != ResourceDisposition::Protected {
        if temporary_profile_path(correlation.profile_path.as_deref())
            && process
                .age_seconds
                .is_some_and(|age| age >= TEMP_PROFILE_MIN_AGE_SECONDS)
        {
            disposition = ResourceDisposition::Candidate;
            reasons.push("old_temporary_profile_process".to_string());
            gc_action = Some("terminate_process".to_string());
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

    let candidate_identity = gc_action.as_ref().map(|action| GcCandidateIdentity {
        pid: process.pid,
        process_group_id: process.process_group_id,
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

fn summarize_resources(records: &[ResourceRecord]) -> ResourceSummary {
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
        total_rss_bytes: records.iter().filter_map(|record| record.rss_bytes).sum(),
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
    command_text: &str,
    profile_path: Option<&str>,
    cdp_port: Option<u16>,
) -> ResourceKind {
    let executable = process
        .executable
        .as_deref()
        .or_else(|| process.command.first().map(String::as_str))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if executable.contains("xvfb") || command_text.contains("Xvfb") {
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
    fn terminate(&self, candidate: &Value) -> Value;
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
    fn terminate(&self, candidate: &Value) -> Value {
        let Some(pid) = candidate
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|value| i32::try_from(value).ok())
        else {
            return json!({
                "status": "skipped",
                "reason": "missing_pid",
            });
        };
        terminate_pid(pid)
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
        let outcome = terminator.terminate(candidate);
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
    classify_process(state, dashboard_main_pid, sample).is_some_and(|current| {
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
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

fn terminate_pid(pid: i32) -> Value {
    #[cfg(unix)]
    {
        if !pid_is_running(pid) {
            return json!({
                "status": "already_exited",
                "signal": null,
            });
        }
        if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
            return json!({
                "status": "failed",
                "reason": "sigterm_failed",
            });
        }
        thread::sleep(Duration::from_millis(GC_TERM_WAIT_MS));
        if !pid_is_running(pid) {
            return json!({
                "status": "terminated",
                "signal": "SIGTERM",
            });
        }
        if unsafe { libc::kill(pid, libc::SIGKILL) } != 0 {
            return json!({
                "status": "failed",
                "reason": "sigkill_failed",
            });
        }
        thread::sleep(Duration::from_millis(GC_TERM_WAIT_MS));
        if pid_is_running(pid) {
            json!({
                "status": "failed",
                "reason": "process_survived_sigkill",
            })
        } else {
            json!({
                "status": "terminated",
                "signal": "SIGKILL",
            })
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        json!({
            "status": "skipped",
            "reason": "apply_not_supported_on_this_platform",
        })
    }
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
        match linux_process_sample(pid, boot_time_seconds, uptime_seconds, clock_ticks) {
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
        .as_deref()
        .and_then(Path::file_name)
        .map(|value| value.to_string_lossy().to_string())
        .or_else(|| command.first().cloned());
    Some(ProcessSample {
        pid,
        ppid,
        process_group_id,
        executable,
        command,
        rss_bytes,
        cpu_seconds,
        age_seconds,
    })
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
    fn resources_classify_old_temporary_profile_as_candidate() {
        let response = service_resources_response_from_samples(
            &ServiceState::default(),
            vec![sample(
                202,
                &["chromium", "--user-data-dir=/tmp/agent-browser-plan0026"],
                Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
            )],
            Vec::new(),
        );
        assert_eq!(response["summary"]["candidateCount"], 1);
        assert_eq!(response["resources"][0]["gcAction"], "terminate_process");
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
    fn gc_dry_run_groups_candidates_without_applying() {
        let response = service_resources_response_from_samples(
            &ServiceState::default(),
            vec![sample(
                404,
                &["chromium", "--user-data-dir=/tmp/agent-browser-smoke-old"],
                Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
            )],
            Vec::new(),
        );
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
        fn terminate(&self, _candidate: &Value) -> Value {
            json!({
                "status": "terminated",
                "signal": "SIGTERM",
            })
        }
    }

    #[test]
    fn gc_apply_requires_matching_review_token() {
        let candidate = sample(
            606,
            &["chromium", "--user-data-dir=/tmp/agent-browser-plan0026"],
            Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
        );
        let resources = service_resources_response_from_samples(
            &ServiceState::default(),
            vec![candidate.clone()],
            Vec::new(),
        );
        let candidates = candidates_from_response(&resources);
        let token = review_token_for_candidates(&candidates, unix_now_seconds());
        let mut state = ServiceState::default();

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
    fn gc_apply_refuses_changed_candidate_identity() {
        let candidate = sample(
            707,
            &["chromium", "--user-data-dir=/tmp/agent-browser-plan0026"],
            Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
        );
        let resources = service_resources_response_from_samples(
            &ServiceState::default(),
            vec![candidate.clone()],
            Vec::new(),
        );
        let candidates = candidates_from_response(&resources);
        let token = review_token_for_candidates(&candidates, unix_now_seconds());
        let changed = sample(
            707,
            &["chromium", "--user-data-dir=/tmp/agent-browser-other"],
            Some(TEMP_PROFILE_MIN_AGE_SECONDS + 1),
        );
        let mut state = ServiceState::default();

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
