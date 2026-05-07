use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use std::time::Duration;

use crate::flags::load_config;

use super::service_model::{
    JobState, MonitorState, MonitorTarget, ServiceEvent, ServiceEventKind, ServiceState,
    SiteMonitor, TabLifecycle,
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
    let (success, result) = match &monitor.target {
        MonitorTarget::Url(url) => probe_url(url).await,
        MonitorTarget::Tab(tab_id) => probe_tab(state, tab_id),
        MonitorTarget::SitePolicy(site_policy_id) => probe_site_policy(state, site_policy_id),
    };
    let event_kind = if success {
        ServiceEventKind::Reconciliation
    } else {
        ServiceEventKind::ReconciliationError
    };
    let message = if success {
        format!("Service monitor {} succeeded: {}", monitor.id, result)
    } else {
        format!("Service monitor {} failed: {}", monitor.id, result)
    };

    MonitorProbeResult {
        monitor: monitor.clone(),
        monitor_id: monitor.id.clone(),
        checked_at,
        success,
        result,
        event_kind,
        message,
        target: monitor.target.clone(),
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

fn summarize_results(results: &[MonitorProbeResult]) -> MonitorRunSummary {
    MonitorRunSummary {
        checked: results.len(),
        succeeded: results.iter().filter(|result| result.success).count(),
        failed: results.iter().filter(|result| !result.success).count(),
        monitor_ids: results
            .iter()
            .map(|result| result.monitor_id.clone())
            .collect(),
    }
}

fn apply_monitor_result(state: &mut ServiceState, result: MonitorProbeResult) {
    let monitor = state
        .monitors
        .entry(result.monitor_id.clone())
        .or_insert_with(|| result.monitor.clone());
    monitor.last_checked_at = Some(result.checked_at.clone());
    monitor.last_result = Some(result.result.clone());
    monitor.state = if result.success {
        MonitorState::Active
    } else {
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
        })),
        ..ServiceEvent::default()
    });
    if state.events.len() > MAX_SERVICE_EVENTS {
        let excess = state.events.len() - MAX_SERVICE_EVENTS;
        state.events.drain(0..excess);
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
    use crate::native::service_model::{BrowserTab, SitePolicy};
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
        let monitor = &state.monitors["tab-heartbeat"];
        assert_eq!(monitor.state, MonitorState::Active);
        assert_eq!(monitor.last_result.as_deref(), Some("tab_ready"));
        assert!(monitor.last_checked_at.is_some());
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
        let monitor = &state.monitors["policy-heartbeat"];
        assert_eq!(monitor.state, MonitorState::Faulted);
        assert_eq!(monitor.last_result.as_deref(), Some("site_policy_missing"));
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
}
