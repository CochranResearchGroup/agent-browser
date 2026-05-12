use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use std::time::Duration;

use crate::flags::load_config;

use super::service_model::{
    JobState, MonitorState, MonitorTarget, ProfileReadinessState, ServiceEvent, ServiceEventKind,
    ServiceState, SiteMonitor, TabLifecycle,
};
use super::service_store::{
    load_default_service_state_snapshot, LockedServiceStateRepository, ServiceStateRepository,
};

const MAX_SERVICE_EVENTS: usize = 100;
const MONITOR_PROBE_TIMEOUT_MS: u64 = 10_000;
pub const SERVICE_MONITORS_RUN_DUE_ACTION: &str = "service_monitors_run_due";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRunSummary {
    pub checked: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub monitor_ids: Vec<String>,
    pub results: Vec<MonitorRunResultSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRunResultSummary {
    pub monitor_id: String,
    pub checked_at: String,
    pub success: bool,
    pub result: String,
    pub target: MonitorTarget,
    pub stale_profile_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonitorCollectionFilters {
    pub state: Option<MonitorState>,
    pub failed_only: bool,
    pub summary: bool,
}

#[derive(Debug, Clone)]
struct MonitorProbeResult {
    monitor: SiteMonitor,
    monitor_id: String,
    checked_at: String,
    success: bool,
    result: String,
    event_kind: ServiceEventKind,
    message: String,
    target: MonitorTarget,
    stale_profile_ids: Vec<String>,
}

pub fn persisted_due_monitor_work_pending() -> bool {
    match configured_service_state_snapshot() {
        Ok(state) => due_monitor_work_pending(&state, Utc::now()),
        Err(_) => false,
    }
}

pub async fn run_due_persisted_monitors() -> Result<MonitorRunSummary, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = apply_configured_service_state(repository.load_snapshot()?);
    run_due_monitors_with_snapshot(&repository, snapshot).await
}

pub async fn run_due_monitors_in_repository<S>(
    repository: &LockedServiceStateRepository<S>,
) -> Result<MonitorRunSummary, String>
where
    S: super::service_store::ServiceStateStore,
{
    let snapshot = repository.load_snapshot()?;
    run_due_monitors_with_snapshot(repository, snapshot).await
}

pub fn service_monitors_response(
    state: &ServiceState,
    filters: MonitorCollectionFilters,
) -> serde_json::Value {
    let total = state.monitors.len();
    let mut monitors = state
        .monitors
        .values()
        .filter(|monitor| monitor_matches_filters(monitor, &filters))
        .cloned()
        .collect::<Vec<_>>();
    monitors.sort_by(|left, right| left.id.cmp(&right.id));
    let matched = monitors.len();

    let mut response = json!({
        "monitors": monitors,
        "count": matched,
        "matched": matched,
        "total": total,
        "filters": {
            "state": filters.state,
            "failedOnly": filters.failed_only,
            "summary": filters.summary,
        },
    });
    if filters.summary {
        response["summary"] = monitor_collection_summary(
            response["monitors"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
    }
    response
}

pub fn parse_monitor_state(value: &str) -> Option<MonitorState> {
    match value {
        "active" => Some(MonitorState::Active),
        "paused" => Some(MonitorState::Paused),
        "faulted" => Some(MonitorState::Faulted),
        _ => None,
    }
}

async fn run_due_monitors_with_snapshot<S>(
    repository: &LockedServiceStateRepository<S>,
    snapshot: ServiceState,
) -> Result<MonitorRunSummary, String>
where
    S: super::service_store::ServiceStateStore,
{
    let now = Utc::now();
    let due_monitors = due_monitors(&snapshot, now);
    if due_monitors.is_empty() {
        return Ok(MonitorRunSummary {
            checked: 0,
            succeeded: 0,
            failed: 0,
            monitor_ids: Vec::new(),
            results: Vec::new(),
        });
    }

    let mut results = Vec::with_capacity(due_monitors.len());
    for monitor in due_monitors {
        results.push(run_monitor_probe(&monitor, &snapshot).await);
    }

    let summary = summarize_results(&results);
    repository.mutate(|state| {
        for result in results {
            apply_monitor_result(state, result);
        }
        Ok(())
    })?;
    Ok(summary)
}

fn due_monitor_work_pending(state: &ServiceState, now: DateTime<Utc>) -> bool {
    !has_running_monitor_job(state)
        && state
            .monitors
            .values()
            .any(|monitor| monitor_due(monitor, now))
}

fn has_running_monitor_job(state: &ServiceState) -> bool {
    state.jobs.values().any(|job| {
        job.action == SERVICE_MONITORS_RUN_DUE_ACTION
            && matches!(
                job.state,
                JobState::Queued | JobState::WaitingProfileLease | JobState::Running
            )
    })
}

fn due_monitors(state: &ServiceState, now: DateTime<Utc>) -> Vec<SiteMonitor> {
    state
        .monitors
        .values()
        .filter(|monitor| monitor_due(monitor, now))
        .cloned()
        .collect()
}

fn monitor_due(monitor: &SiteMonitor, now: DateTime<Utc>) -> bool {
    if monitor.state != MonitorState::Active {
        return false;
    }
    let Some(last_checked_at) = monitor.last_checked_at.as_deref() else {
        return true;
    };
    let Ok(last_checked_at) = DateTime::parse_from_rfc3339(last_checked_at) else {
        return true;
    };
    let elapsed_ms = now
        .signed_duration_since(last_checked_at.with_timezone(&Utc))
        .num_milliseconds();
    elapsed_ms >= 0 && elapsed_ms as u64 >= monitor.interval_ms
}

async fn run_monitor_probe(monitor: &SiteMonitor, state: &ServiceState) -> MonitorProbeResult {
    let checked_at = current_timestamp();
    let probe = match &monitor.target {
        MonitorTarget::Url(url) => MonitorProbeOutcome::new(probe_url(url).await),
        MonitorTarget::Tab(tab_id) => MonitorProbeOutcome::new(probe_tab(state, tab_id)),
        MonitorTarget::SitePolicy(site_policy_id) => {
            MonitorProbeOutcome::new(probe_site_policy(state, site_policy_id))
        }
        MonitorTarget::ProfileReadiness(target_service_id) => {
            probe_profile_readiness(state, target_service_id, &checked_at)
        }
    };
    let event_kind = if probe.success {
        ServiceEventKind::Reconciliation
    } else {
        ServiceEventKind::ReconciliationError
    };
    let message = if probe.success {
        format!("Service monitor {} succeeded: {}", monitor.id, probe.result)
    } else {
        format!("Service monitor {} failed: {}", monitor.id, probe.result)
    };

    MonitorProbeResult {
        monitor: monitor.clone(),
        monitor_id: monitor.id.clone(),
        checked_at,
        success: probe.success,
        result: probe.result,
        event_kind,
        message,
        target: monitor.target.clone(),
        stale_profile_ids: probe.stale_profile_ids,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MonitorProbeOutcome {
    success: bool,
    result: String,
    stale_profile_ids: Vec<String>,
}

impl MonitorProbeOutcome {
    fn new((success, result): (bool, String)) -> Self {
        Self {
            success,
            result,
            stale_profile_ids: Vec::new(),
        }
    }
}

async fn probe_url(url: &str) -> (bool, String) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(MONITOR_PROBE_TIMEOUT_MS))
        .user_agent("agent-browser-service-monitor")
        .build()
    {
        Ok(client) => client,
        Err(err) => return (false, format!("client_error:{}", err)),
    };
    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_server_error() {
                (false, format!("http_{}", status.as_u16()))
            } else {
                (true, format!("http_{}", status.as_u16()))
            }
        }
        Err(err) => (false, format!("request_error:{}", err)),
    }
}

fn probe_tab(state: &ServiceState, tab_id: &str) -> (bool, String) {
    let Some(tab) = state.tabs.get(tab_id) else {
        return (false, "tab_missing".to_string());
    };
    match tab.lifecycle {
        TabLifecycle::Closed | TabLifecycle::Crashed => (
            false,
            format!("tab_{:?}", tab.lifecycle).to_ascii_lowercase(),
        ),
        lifecycle => (true, format!("tab_{:?}", lifecycle).to_ascii_lowercase()),
    }
}

fn probe_site_policy(state: &ServiceState, site_policy_id: &str) -> (bool, String) {
    if state.site_policies.contains_key(site_policy_id) {
        (true, "site_policy_available".to_string())
    } else {
        (false, "site_policy_missing".to_string())
    }
}

fn probe_profile_readiness(
    state: &ServiceState,
    target_service_id: &str,
    checked_at: &str,
) -> MonitorProbeOutcome {
    let Ok(checked_at) = DateTime::parse_from_rfc3339(checked_at) else {
        return MonitorProbeOutcome {
            success: false,
            result: "profile_readiness_clock_error".to_string(),
            stale_profile_ids: Vec::new(),
        };
    };
    let checked_at = checked_at.with_timezone(&Utc);
    let mut observed_rows = 0_usize;
    let mut fresh_rows = 0_usize;
    let mut expired_profile_ids = Vec::new();

    for profile in state.profiles.values() {
        for row in profile
            .target_readiness
            .iter()
            .filter(|row| row.target_service_id == target_service_id)
        {
            observed_rows += 1;
            if row.state != ProfileReadinessState::Fresh {
                continue;
            }
            if freshness_expired(row.freshness_expires_at.as_deref(), checked_at) {
                expired_profile_ids.push(profile.id.clone());
            } else {
                fresh_rows += 1;
            }
        }
    }

    expired_profile_ids.sort();
    expired_profile_ids.dedup();

    if !expired_profile_ids.is_empty() {
        return MonitorProbeOutcome {
            success: false,
            result: "profile_readiness_expired".to_string(),
            stale_profile_ids: expired_profile_ids,
        };
    }
    if fresh_rows > 0 {
        return MonitorProbeOutcome {
            success: true,
            result: "profile_readiness_fresh".to_string(),
            stale_profile_ids: Vec::new(),
        };
    }
    if observed_rows > 0 {
        MonitorProbeOutcome {
            success: false,
            result: "profile_readiness_not_fresh".to_string(),
            stale_profile_ids: Vec::new(),
        }
    } else {
        MonitorProbeOutcome {
            success: false,
            result: "profile_readiness_missing".to_string(),
            stale_profile_ids: Vec::new(),
        }
    }
}

fn freshness_expired(freshness_expires_at: Option<&str>, checked_at: DateTime<Utc>) -> bool {
    let Some(freshness_expires_at) = freshness_expires_at else {
        return false;
    };
    DateTime::parse_from_rfc3339(freshness_expires_at)
        .map(|expires_at| expires_at.with_timezone(&Utc) <= checked_at)
        .unwrap_or(true)
}

fn summarize_results(results: &[MonitorProbeResult]) -> MonitorRunSummary {
    MonitorRunSummary {
        checked: results.len(),
        succeeded: results.iter().filter(|result| result.success).count(),
        failed: results.iter().filter(|result| !result.success).count(),
        monitor_ids: results
            .iter()
            .map(|result| result.monitor_id.clone())
            .collect(),
        results: results
            .iter()
            .map(|result| MonitorRunResultSummary {
                monitor_id: result.monitor_id.clone(),
                checked_at: result.checked_at.clone(),
                success: result.success,
                result: result.result.clone(),
                target: result.target.clone(),
                stale_profile_ids: result.stale_profile_ids.clone(),
            })
            .collect(),
    }
}

fn monitor_matches_filters(monitor: &SiteMonitor, filters: &MonitorCollectionFilters) -> bool {
    if let Some(state) = filters.state {
        if monitor.state != state {
            return false;
        }
    }
    if filters.failed_only && !monitor_is_failing(monitor) {
        return false;
    }
    true
}

fn monitor_is_failing(monitor: &SiteMonitor) -> bool {
    monitor.state == MonitorState::Faulted || monitor.consecutive_failures > 0
}

fn monitor_collection_summary(monitors: &[serde_json::Value]) -> serde_json::Value {
    let mut active = 0_u64;
    let mut paused = 0_u64;
    let mut faulted = 0_u64;
    let mut failing = Vec::new();
    let mut repeated_failures = Vec::new();
    let mut never_checked = Vec::new();
    let mut last_failed_at: Option<String> = None;

    for monitor in monitors {
        match monitor.get("state").and_then(|value| value.as_str()) {
            Some("active") => active += 1,
            Some("paused") => paused += 1,
            Some("faulted") => faulted += 1,
            _ => {}
        }
        let id = monitor
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown-monitor")
            .to_string();
        let consecutive_failures = monitor
            .get("consecutiveFailures")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        if monitor.get("state").and_then(|value| value.as_str()) == Some("faulted")
            || consecutive_failures > 0
        {
            failing.push(id.clone());
        }
        if consecutive_failures > 1 {
            repeated_failures.push(id.clone());
        }
        if monitor
            .get("lastCheckedAt")
            .and_then(|value| value.as_str())
            .is_none()
        {
            never_checked.push(id);
        }
        if let Some(failed_at) = monitor.get("lastFailedAt").and_then(|value| value.as_str()) {
            if last_failed_at
                .as_deref()
                .map(|current| failed_at > current)
                .unwrap_or(true)
            {
                last_failed_at = Some(failed_at.to_string());
            }
        }
    }

    json!({
        "total": monitors.len(),
        "active": active,
        "paused": paused,
        "faulted": faulted,
        "failing": failing.len(),
        "repeatedFailures": repeated_failures.len(),
        "neverChecked": never_checked.len(),
        "failingMonitorIds": failing,
        "repeatedFailureMonitorIds": repeated_failures,
        "neverCheckedMonitorIds": never_checked,
        "lastFailedAt": last_failed_at,
    })
}

fn apply_monitor_result(state: &mut ServiceState, result: MonitorProbeResult) {
    mark_expired_profile_readiness_stale(state, &result);
    let monitor = state
        .monitors
        .entry(result.monitor_id.clone())
        .or_insert_with(|| result.monitor.clone());
    monitor.last_checked_at = Some(result.checked_at.clone());
    monitor.last_result = Some(result.result.clone());
    monitor.state = if result.success {
        monitor.last_succeeded_at = Some(result.checked_at.clone());
        monitor.consecutive_failures = 0;
        MonitorState::Active
    } else {
        monitor.last_failed_at = Some(result.checked_at.clone());
        monitor.consecutive_failures = monitor.consecutive_failures.saturating_add(1);
        MonitorState::Faulted
    };
    state.events.push(ServiceEvent {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        timestamp: result.checked_at,
        kind: result.event_kind,
        message: result.message,
        details: Some(json!({
            "incidentId": format!("monitor:{}", result.monitor_id),
            "monitorId": result.monitor_id,
            "target": result.target,
            "success": result.success,
            "result": result.result,
            "staleProfileIds": result.stale_profile_ids,
        })),
        ..ServiceEvent::default()
    });
    if state.events.len() > MAX_SERVICE_EVENTS {
        let excess = state.events.len() - MAX_SERVICE_EVENTS;
        state.events.drain(0..excess);
    }
}

fn mark_expired_profile_readiness_stale(state: &mut ServiceState, result: &MonitorProbeResult) {
    let MonitorTarget::ProfileReadiness(target_service_id) = &result.target else {
        return;
    };
    for profile_id in &result.stale_profile_ids {
        let Some(profile) = state.profiles.get_mut(profile_id) else {
            continue;
        };
        for row in profile
            .target_readiness
            .iter_mut()
            .filter(|row| row.target_service_id == *target_service_id)
            .filter(|row| row.state == ProfileReadinessState::Fresh)
        {
            row.state = ProfileReadinessState::Stale;
            row.manual_seeding_required = false;
            row.evidence = format!("freshness_expired_by_monitor:{}", result.monitor_id);
            row.recommended_action = "probe_target_auth_or_reseed_if_needed".to_string();
        }
        profile
            .authenticated_service_ids
            .retain(|id| id != target_service_id);
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn configured_service_state_snapshot() -> Result<ServiceState, String> {
    load_default_service_state_snapshot().map(apply_configured_service_state)
}

fn apply_configured_service_state(mut state: ServiceState) -> ServiceState {
    if let Ok(config) = load_config(&[]) {
        state.overlay_configured_entities(config.service_state_snapshot());
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProfile, BrowserTab, ProfileTargetReadiness, SitePolicy,
    };
    use crate::native::service_store::JsonServiceStateStore;
    use std::collections::BTreeMap;

    fn unique_state_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("state.json")
    }

    #[test]
    fn due_monitor_work_ignores_paused_and_running_job_state() {
        let now = Utc::now();
        let state = ServiceState {
            monitors: BTreeMap::from([(
                "google".to_string(),
                SiteMonitor {
                    id: "google".to_string(),
                    state: MonitorState::Paused,
                    last_checked_at: None,
                    ..SiteMonitor::default()
                },
            )]),
            ..ServiceState::default()
        };

        assert!(!due_monitor_work_pending(&state, now));

        let state = ServiceState {
            monitors: BTreeMap::from([(
                "google".to_string(),
                SiteMonitor {
                    id: "google".to_string(),
                    state: MonitorState::Active,
                    last_checked_at: None,
                    ..SiteMonitor::default()
                },
            )]),
            jobs: BTreeMap::from([(
                "monitor-run".to_string(),
                super::super::service_model::ServiceJob {
                    id: "monitor-run".to_string(),
                    action: SERVICE_MONITORS_RUN_DUE_ACTION.to_string(),
                    state: JobState::Running,
                    ..super::super::service_model::ServiceJob::default()
                },
            )]),
            ..ServiceState::default()
        };

        assert!(!due_monitor_work_pending(&state, now));
    }

    #[tokio::test]
    async fn run_due_monitors_updates_records_and_events() {
        let path = unique_state_path("monitor-runner");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|state| {
                state.monitors.insert(
                    "tab-heartbeat".to_string(),
                    SiteMonitor {
                        id: "tab-heartbeat".to_string(),
                        name: "Tab heartbeat".to_string(),
                        target: MonitorTarget::Tab("tab-1".to_string()),
                        state: MonitorState::Active,
                        last_checked_at: None,
                        ..SiteMonitor::default()
                    },
                );
                state.tabs.insert(
                    "tab-1".to_string(),
                    BrowserTab {
                        id: "tab-1".to_string(),
                        lifecycle: TabLifecycle::Ready,
                        ..BrowserTab::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let summary = run_due_monitors_in_repository(&repository).await.unwrap();
        let state = repository.load_snapshot().unwrap();

        assert_eq!(summary.checked, 1);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.results.len(), 1);
        assert_eq!(summary.results[0].monitor_id, "tab-heartbeat");
        assert_eq!(summary.results[0].result, "tab_ready");
        assert!(summary.results[0].success);
        let monitor = &state.monitors["tab-heartbeat"];
        assert_eq!(monitor.state, MonitorState::Active);
        assert_eq!(monitor.last_result.as_deref(), Some("tab_ready"));
        assert!(monitor.last_checked_at.is_some());
        assert_eq!(monitor.last_succeeded_at, monitor.last_checked_at);
        assert_eq!(monitor.last_failed_at, None);
        assert_eq!(monitor.consecutive_failures, 0);
        assert_eq!(state.events.len(), 1);
        assert_eq!(state.events[0].kind, ServiceEventKind::Reconciliation);
    }

    #[tokio::test]
    async fn run_due_monitors_faults_failed_site_policy_probe() {
        let path = unique_state_path("monitor-runner-fault");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|state| {
                state.monitors.insert(
                    "policy-heartbeat".to_string(),
                    SiteMonitor {
                        id: "policy-heartbeat".to_string(),
                        name: "Policy heartbeat".to_string(),
                        target: MonitorTarget::SitePolicy("missing".to_string()),
                        state: MonitorState::Active,
                        last_checked_at: None,
                        ..SiteMonitor::default()
                    },
                );
                state.site_policies.insert(
                    "google".to_string(),
                    SitePolicy {
                        id: "google".to_string(),
                        ..SitePolicy::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let summary = run_due_monitors_in_repository(&repository).await.unwrap();
        let state = repository.load_snapshot().unwrap();

        assert_eq!(summary.checked, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.results.len(), 1);
        assert_eq!(summary.results[0].monitor_id, "policy-heartbeat");
        assert_eq!(summary.results[0].result, "site_policy_missing");
        assert!(!summary.results[0].success);
        let monitor = &state.monitors["policy-heartbeat"];
        assert_eq!(monitor.state, MonitorState::Faulted);
        assert_eq!(monitor.last_result.as_deref(), Some("site_policy_missing"));
        assert_eq!(monitor.last_failed_at, monitor.last_checked_at);
        assert_eq!(monitor.last_succeeded_at, None);
        assert_eq!(monitor.consecutive_failures, 1);
        assert_eq!(state.events[0].kind, ServiceEventKind::ReconciliationError);
        assert_eq!(
            state.events[0].details.as_ref().unwrap()["incidentId"],
            "monitor:policy-heartbeat"
        );
        assert_eq!(state.incidents.len(), 1);
        assert_eq!(state.incidents[0].id, "monitor:policy-heartbeat");
        assert_eq!(
            state.incidents[0].monitor_id.as_deref(),
            Some("policy-heartbeat")
        );
        assert_eq!(
            state.incidents[0].monitor_result.as_deref(),
            Some("site_policy_missing")
        );
        assert_eq!(
            state.incidents[0].escalation,
            super::super::service_model::ServiceIncidentEscalation::MonitorAttention
        );
    }

    #[tokio::test]
    async fn run_due_monitors_marks_expired_profile_readiness_stale() {
        let path = unique_state_path("monitor-profile-readiness-expired");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|state| {
                state.monitors.insert(
                    "acs-freshness".to_string(),
                    SiteMonitor {
                        id: "acs-freshness".to_string(),
                        name: "ACS freshness".to_string(),
                        target: MonitorTarget::ProfileReadiness("acs".to_string()),
                        state: MonitorState::Active,
                        last_checked_at: None,
                        ..SiteMonitor::default()
                    },
                );
                state.profiles.insert(
                    "journal-acs".to_string(),
                    BrowserProfile {
                        id: "journal-acs".to_string(),
                        target_service_ids: vec!["acs".to_string()],
                        authenticated_service_ids: vec!["acs".to_string()],
                        target_readiness: vec![ProfileTargetReadiness {
                            target_service_id: "acs".to_string(),
                            login_id: Some("acs".to_string()),
                            state: ProfileReadinessState::Fresh,
                            manual_seeding_required: false,
                            evidence: "auth_probe_cookie_present".to_string(),
                            recommended_action: "use_profile".to_string(),
                            last_verified_at: Some("2026-05-01T00:00:00Z".to_string()),
                            freshness_expires_at: Some("2026-05-01T00:00:01Z".to_string()),
                            ..ProfileTargetReadiness::default()
                        }],
                        ..BrowserProfile::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let summary = run_due_monitors_in_repository(&repository).await.unwrap();
        let state = repository.load_snapshot().unwrap();

        assert_eq!(summary.checked, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(
            summary.results[0].stale_profile_ids,
            vec!["journal-acs".to_string()]
        );
        let monitor = &state.monitors["acs-freshness"];
        assert_eq!(monitor.state, MonitorState::Faulted);
        assert_eq!(
            monitor.last_result.as_deref(),
            Some("profile_readiness_expired")
        );
        let profile = &state.profiles["journal-acs"];
        assert!(profile.authenticated_service_ids.is_empty());
        let readiness = &profile.target_readiness[0];
        assert_eq!(readiness.state, ProfileReadinessState::Stale);
        assert_eq!(
            readiness.evidence,
            "freshness_expired_by_monitor:acs-freshness"
        );
        assert_eq!(
            readiness.recommended_action,
            "probe_target_auth_or_reseed_if_needed"
        );
        assert_eq!(state.events[0].kind, ServiceEventKind::ReconciliationError);
        assert_eq!(
            state.events[0].details.as_ref().unwrap()["staleProfileIds"],
            json!(["journal-acs"])
        );
    }

    #[tokio::test]
    async fn run_due_monitors_succeeds_for_current_profile_readiness() {
        let path = unique_state_path("monitor-profile-readiness-fresh");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|state| {
                state.monitors.insert(
                    "acs-freshness".to_string(),
                    SiteMonitor {
                        id: "acs-freshness".to_string(),
                        name: "ACS freshness".to_string(),
                        target: MonitorTarget::ProfileReadiness("acs".to_string()),
                        state: MonitorState::Active,
                        last_checked_at: None,
                        ..SiteMonitor::default()
                    },
                );
                state.profiles.insert(
                    "journal-acs".to_string(),
                    BrowserProfile {
                        id: "journal-acs".to_string(),
                        target_service_ids: vec!["acs".to_string()],
                        authenticated_service_ids: vec!["acs".to_string()],
                        target_readiness: vec![ProfileTargetReadiness {
                            target_service_id: "acs".to_string(),
                            login_id: Some("acs".to_string()),
                            state: ProfileReadinessState::Fresh,
                            manual_seeding_required: false,
                            evidence: "auth_probe_cookie_present".to_string(),
                            recommended_action: "use_profile".to_string(),
                            last_verified_at: Some("2026-05-01T00:00:00Z".to_string()),
                            freshness_expires_at: Some("2999-05-01T00:00:01Z".to_string()),
                            ..ProfileTargetReadiness::default()
                        }],
                        ..BrowserProfile::default()
                    },
                );
                Ok(())
            })
            .unwrap();

        let summary = run_due_monitors_in_repository(&repository).await.unwrap();
        let state = repository.load_snapshot().unwrap();

        assert_eq!(summary.checked, 1);
        assert_eq!(summary.succeeded, 1);
        let monitor = &state.monitors["acs-freshness"];
        assert_eq!(monitor.state, MonitorState::Active);
        assert_eq!(
            monitor.last_result.as_deref(),
            Some("profile_readiness_fresh")
        );
        let profile = &state.profiles["journal-acs"];
        assert_eq!(profile.authenticated_service_ids, vec!["acs".to_string()]);
        assert_eq!(
            profile.target_readiness[0].state,
            ProfileReadinessState::Fresh
        );
        assert_eq!(state.events[0].kind, ServiceEventKind::Reconciliation);
    }
}
