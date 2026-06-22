//! Health and target probes for persisted service-mode browser records.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use super::browser::BrowserShutdownOutcome;
use super::service_lifecycle::{upsert_service_profile_and_session, ServiceLaunchMetadata};
use super::service_model::{
    BrowserHealth, BrowserHealthObservation, BrowserHost, BrowserProcess, BrowserSession,
    BrowserTab, DisplayAllocation, LeaseState, ServiceEvent, ServiceEventKind, ServiceIncident,
    ServiceReconciliationSnapshot, ServiceState, TabLifecycle, ViewStreamProvider,
};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

const CDP_PROBE_TIMEOUT: Duration = Duration::from_millis(750);
const MAX_SERVICE_EVENTS: usize = 100;

fn mark_cdp_screencast_streams_unavailable(
    browser: &mut BrowserProcess,
    reason: &str,
    health: BrowserHealth,
) {
    for stream in &mut browser.view_streams {
        if stream.provider != ViewStreamProvider::CdpScreencast {
            continue;
        }
        stream.control_input = None;
        stream.url = None;
        stream.frame_url = None;
        stream.external_url = None;
        stream.read_only = true;
        stream.readiness = Some(json!({
            "state": "unavailable",
            "reason": reason,
            "browserId": browser.id,
            "health": health,
        }));
        stream.remote_readiness = None;
    }
}

/// Structured reason for browser recovery, preserved in event details for clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserRecoveryReasonKind {
    ProcessExited,
    CdpDisconnected,
    UnreachableEndpoint,
    DegradedTargets,
    OperatorRequestedClose,
    PersistedUnhealthyState,
    Unknown,
}

impl BrowserRecoveryReasonKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProcessExited => "process_exited",
            Self::CdpDisconnected => "cdp_disconnected",
            Self::UnreachableEndpoint => "unreachable_endpoint",
            Self::DegradedTargets => "degraded_targets",
            Self::OperatorRequestedClose => "operator_requested_close",
            Self::PersistedUnhealthyState => "persisted_unhealthy_state",
            Self::Unknown => "unknown",
        }
    }
}

/// Stable client-facing cause for browser process exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserProcessExitCause {
    UnexpectedProcessExit,
    OperatorRequestedClose,
}

impl BrowserProcessExitCause {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnexpectedProcessExit => "unexpected_process_exit",
            Self::OperatorRequestedClose => "operator_requested_close",
        }
    }
}

pub fn recovery_reason_kind_for_health(health: BrowserHealth) -> BrowserRecoveryReasonKind {
    match health {
        BrowserHealth::ProcessExited => BrowserRecoveryReasonKind::ProcessExited,
        BrowserHealth::CdpDisconnected => BrowserRecoveryReasonKind::CdpDisconnected,
        BrowserHealth::Unreachable => BrowserRecoveryReasonKind::UnreachableEndpoint,
        BrowserHealth::Degraded => BrowserRecoveryReasonKind::DegradedTargets,
        BrowserHealth::Closing => BrowserRecoveryReasonKind::OperatorRequestedClose,
        BrowserHealth::Faulted => BrowserRecoveryReasonKind::Unknown,
        BrowserHealth::NotStarted
        | BrowserHealth::Launching
        | BrowserHealth::Ready
        | BrowserHealth::Reconnecting => BrowserRecoveryReasonKind::PersistedUnhealthyState,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserRecoveryPolicy {
    pub attempt: u64,
    pub retry_budget: u64,
    pub retry_budget_exceeded: bool,
    pub next_retry_delay_ms: u64,
    pub source: BrowserRecoveryPolicySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserRecoveryPolicyValueSource {
    Default,
    Config,
    Env,
    Cli,
}

impl BrowserRecoveryPolicyValueSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Config => "config",
            Self::Env => "env",
            Self::Cli => "cli",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "config" => Self::Config,
            "env" => Self::Env,
            "cli" => Self::Cli,
            _ => Self::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserRecoveryPolicySource {
    pub retry_budget: BrowserRecoveryPolicyValueSource,
    pub base_backoff_ms: BrowserRecoveryPolicyValueSource,
    pub max_backoff_ms: BrowserRecoveryPolicyValueSource,
}

impl Default for BrowserRecoveryPolicySource {
    fn default() -> Self {
        Self {
            retry_budget: BrowserRecoveryPolicyValueSource::Default,
            base_backoff_ms: BrowserRecoveryPolicyValueSource::Default,
            max_backoff_ms: BrowserRecoveryPolicyValueSource::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserRecoveryPolicyConfig {
    pub retry_budget: u64,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub source: BrowserRecoveryPolicySource,
}

impl Default for BrowserRecoveryPolicyConfig {
    fn default() -> Self {
        Self {
            retry_budget: 3,
            base_backoff_ms: 1_000,
            max_backoff_ms: 30_000,
            source: BrowserRecoveryPolicySource::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserRecoveryPersistence {
    NotRecorded,
    Recorded,
    Blocked(String),
}

impl BrowserRecoveryPersistence {
    pub(crate) fn recorded(&self) -> bool {
        matches!(self, Self::Recorded | Self::Blocked(_))
    }
}

pub(crate) fn recovery_policy_for_next_attempt(
    service_state: &ServiceState,
    browser_id: &str,
    policy_config: BrowserRecoveryPolicyConfig,
) -> BrowserRecoveryPolicy {
    let prior_attempts = service_state
        .events
        .iter()
        .rev()
        .take_while(|event| {
            let is_ready_boundary = event.kind == ServiceEventKind::BrowserHealthChanged
                && event.browser_id.as_deref() == Some(browser_id)
                && event.current_health == Some(BrowserHealth::Ready);
            let is_override_boundary = event.kind == ServiceEventKind::BrowserRecoveryOverride
                && event.browser_id.as_deref() == Some(browser_id);
            !(is_ready_boundary || is_override_boundary)
        })
        .filter(|event| {
            event.kind == ServiceEventKind::BrowserRecoveryStarted
                && event.browser_id.as_deref() == Some(browser_id)
        })
        .count() as u64;
    let attempt = prior_attempts + 1;
    BrowserRecoveryPolicy {
        attempt,
        retry_budget: policy_config.retry_budget,
        retry_budget_exceeded: attempt > policy_config.retry_budget,
        next_retry_delay_ms: recovery_backoff_delay_ms(attempt, policy_config),
        source: policy_config.source,
    }
}

fn recovery_backoff_delay_ms(attempt: u64, policy_config: BrowserRecoveryPolicyConfig) -> u64 {
    let multiplier = 1_u64
        .checked_shl(attempt.saturating_sub(1) as u32)
        .unwrap_or(u64::MAX);
    policy_config
        .base_backoff_ms
        .saturating_mul(multiplier)
        .min(policy_config.max_backoff_ms)
}

fn recovery_reason_kind_value_for_health(health: BrowserHealth) -> Option<serde_json::Value> {
    matches!(
        health,
        BrowserHealth::Degraded
            | BrowserHealth::Unreachable
            | BrowserHealth::ProcessExited
            | BrowserHealth::CdpDisconnected
            | BrowserHealth::Closing
            | BrowserHealth::Faulted
    )
    .then(|| serde_json::json!(recovery_reason_kind_for_health(health).as_str()))
}

fn process_exit_cause_for_health(health: BrowserHealth) -> Option<BrowserProcessExitCause> {
    match health {
        BrowserHealth::ProcessExited => Some(BrowserProcessExitCause::UnexpectedProcessExit),
        BrowserHealth::Closing => Some(BrowserProcessExitCause::OperatorRequestedClose),
        _ => None,
    }
}

fn process_exit_cause_for_recovery_reason(
    reason_kind: BrowserRecoveryReasonKind,
) -> Option<BrowserProcessExitCause> {
    match reason_kind {
        BrowserRecoveryReasonKind::ProcessExited => {
            Some(BrowserProcessExitCause::UnexpectedProcessExit)
        }
        BrowserRecoveryReasonKind::OperatorRequestedClose => {
            Some(BrowserProcessExitCause::OperatorRequestedClose)
        }
        _ => None,
    }
}

fn failure_class_for_health(health: BrowserHealth) -> Option<&'static str> {
    match health {
        BrowserHealth::ProcessExited => Some("browser_process_exited"),
        BrowserHealth::CdpDisconnected => Some("cdp_unresponsive"),
        BrowserHealth::Unreachable => Some("cdp_endpoint_unreachable"),
        BrowserHealth::Degraded => Some("target_discovery_failed"),
        BrowserHealth::Faulted => Some("browser_recovery_faulted"),
        _ => None,
    }
}

fn failure_class_for_recovery_reason(
    reason_kind: BrowserRecoveryReasonKind,
) -> Option<&'static str> {
    match reason_kind {
        BrowserRecoveryReasonKind::ProcessExited => Some("browser_process_exited"),
        BrowserRecoveryReasonKind::CdpDisconnected => Some("cdp_unresponsive"),
        BrowserRecoveryReasonKind::UnreachableEndpoint => Some("cdp_endpoint_unreachable"),
        BrowserRecoveryReasonKind::DegradedTargets => Some("target_discovery_failed"),
        BrowserRecoveryReasonKind::Unknown => Some("browser_recovery_faulted"),
        _ => None,
    }
}

fn browser_health_observation(
    current: &BrowserProcess,
    details: Option<&serde_json::Value>,
) -> BrowserHealthObservation {
    BrowserHealthObservation {
        observed_at: current_timestamp(),
        health: current.health,
        reason_kind: details
            .and_then(|value| value.get("currentReasonKind"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        failure_class: details
            .and_then(|value| value.get("failureClass"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        process_exit_cause: details
            .and_then(|value| value.get("processExitCause"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        message: current.last_error.clone(),
        details: details.cloned(),
    }
}

pub fn browser_health_observation_details(
    current: &BrowserProcess,
    extra_details: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut details = serde_json::json!({
        "currentError": current.last_error,
    });
    if let Some(reason_kind) = recovery_reason_kind_value_for_health(current.health) {
        details["currentReasonKind"] = reason_kind;
    }
    if let Some(failure_class) = failure_class_for_health(current.health) {
        details["failureClass"] = serde_json::json!(failure_class);
    }
    if let Some(cause) = process_exit_cause_for_health(current.health) {
        details["processExitCause"] = serde_json::json!(cause.as_str());
    }
    if let Some(serde_json::Value::Object(extra)) = extra_details {
        if let Some(details) = details.as_object_mut() {
            for (key, value) in extra {
                details.insert(key, value);
            }
        }
    }
    details
}

pub fn apply_browser_health_observation(
    browser: &mut BrowserProcess,
    details: Option<&serde_json::Value>,
) {
    if matches!(
        browser.health,
        BrowserHealth::Ready | BrowserHealth::NotStarted
    ) {
        browser.last_health_observation = None;
    } else {
        browser.last_health_observation = Some(browser_health_observation(browser, details));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceReconcileSummary {
    pub browser_count: usize,
    pub changed_browsers: usize,
    pub expired_session_leases: Vec<String>,
}

pub async fn reconcile_service_state(state: &mut ServiceState) -> ServiceReconcileSummary {
    let before = state.clone();
    let reconciled_at = current_timestamp();
    refresh_persisted_browser_health(state).await;
    let merged_duplicate_browsers = merge_duplicate_live_browser_records(state);
    reconcile_live_browser_targets(state).await;
    let remote_view_repair = reconcile_remote_view_state(state);
    let expired_session_leases = state.expire_stale_session_leases(reconciled_at.as_str());
    state.refresh_service_tab_handles();
    record_health_transition_events(state, &before);
    record_tab_lifecycle_events(state, &before);
    let removed_terminated_browsers = remove_post_termination_browser_history(state);
    state.refresh_service_tab_handles();
    let changed_browsers = changed_browser_count(state, &before);

    let summary = ServiceReconcileSummary {
        browser_count: state.browsers.len(),
        changed_browsers,
        expired_session_leases,
    };
    state.reconciliation = Some(ServiceReconciliationSnapshot {
        last_reconciled_at: Some(reconciled_at),
        last_error: None,
        browser_count: summary.browser_count,
        changed_browsers: summary.changed_browsers,
    });
    push_service_event(
        state,
        ServiceEvent {
            kind: ServiceEventKind::Reconciliation,
            message: format!(
                "Reconciled {} browser records, {} changed",
                summary.browser_count, summary.changed_browsers
            ),
            details: Some(serde_json::json!({
                "browserCount": summary.browser_count,
                "changedBrowsers": summary.changed_browsers,
                "removedTerminatedBrowsers": removed_terminated_browsers,
                "mergedDuplicateBrowsers": merged_duplicate_browsers,
                "expiredSessionLeases": summary.expired_session_leases.clone(),
                "expiredSessionLeaseCount": summary.expired_session_leases.len(),
                "tabCount": state.tabs.len(),
                "changedTabs": changed_tab_count(state, &before),
                "remoteView": {
                    "orphanedDisplayAllocations": remote_view_repair.orphaned_display_allocations,
                    "orphanedRoutes": remote_view_repair.orphaned_routes,
                    "releasedViewerLeases": remote_view_repair.released_viewer_leases,
                    "expiredViewerLeases": remote_view_repair.expired_viewer_leases,
                    "clearedControllerLeases": remote_view_repair.cleared_controller_leases,
                },
            })),
            ..new_service_event()
        },
    );
    summary
}

fn merge_duplicate_live_browser_records(state: &mut ServiceState) -> usize {
    let mut endpoint_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (browser_id, browser) in &state.browsers {
        if browser.health != BrowserHealth::Ready {
            continue;
        }
        let Some(endpoint) = browser.cdp_endpoint.as_deref().map(str::trim) else {
            continue;
        };
        if endpoint.is_empty() {
            continue;
        }
        endpoint_groups
            .entry(endpoint.to_string())
            .or_default()
            .push(browser_id.clone());
    }

    let mut merged = 0;
    for mut browser_ids in endpoint_groups.into_values() {
        browser_ids.sort_by(|left, right| {
            let left_browser = state.browsers.get(left);
            let right_browser = state.browsers.get(right);
            duplicate_browser_canonical_score(right_browser)
                .cmp(&duplicate_browser_canonical_score(left_browser))
                .then_with(|| left.cmp(right))
        });
        let Some(canonical_browser_id) = browser_ids.first().cloned() else {
            continue;
        };
        for duplicate_browser_id in browser_ids.into_iter().skip(1) {
            if duplicate_browser_id == canonical_browser_id {
                continue;
            }
            if merge_duplicate_browser_record(state, &canonical_browser_id, &duplicate_browser_id) {
                merged += 1;
            }
        }
    }
    merged
}

fn duplicate_browser_canonical_score(browser: Option<&BrowserProcess>) -> (u8, u8, usize, usize) {
    let Some(browser) = browser else {
        return (0, 0, 0, 0);
    };
    (
        u8::from(browser.pid.is_some()),
        u8::from(browser.display_name.is_some()),
        browser.active_session_ids.len(),
        browser.view_streams.len(),
    )
}

fn merge_duplicate_browser_record(
    state: &mut ServiceState,
    canonical_browser_id: &str,
    duplicate_browser_id: &str,
) -> bool {
    let Some(duplicate_browser) = state.browsers.remove(duplicate_browser_id) else {
        return false;
    };
    let Some(canonical_browser) = state.browsers.get_mut(canonical_browser_id) else {
        state
            .browsers
            .insert(duplicate_browser_id.to_string(), duplicate_browser);
        return false;
    };

    for session_id in &duplicate_browser.active_session_ids {
        merge_unique(
            &mut canonical_browser.active_session_ids,
            session_id.clone(),
        );
    }
    for stream in duplicate_browser.view_streams {
        if !canonical_browser
            .view_streams
            .iter()
            .any(|candidate| candidate.id == stream.id)
        {
            canonical_browser.view_streams.push(stream);
        }
    }
    if canonical_browser.display_allocation_id.is_none() {
        canonical_browser.display_allocation_id = duplicate_browser.display_allocation_id.clone();
    }

    for session in state.sessions.values_mut() {
        let referenced_duplicate = session
            .browser_ids
            .iter()
            .any(|browser_id| browser_id == duplicate_browser_id);
        if referenced_duplicate {
            session
                .browser_ids
                .retain(|browser_id| browser_id != duplicate_browser_id);
            merge_unique(&mut session.browser_ids, canonical_browser_id.to_string());
        }
    }
    clear_same_browser_profile_lease_conflicts(state, canonical_browser_id);

    for tab in state.tabs.values_mut() {
        if tab.browser_id == duplicate_browser_id {
            tab.browser_id = canonical_browser_id.to_string();
        }
    }
    for allocation in state.display_allocations.values_mut() {
        if allocation.owner_browser_id.as_deref() == Some(duplicate_browser_id) {
            allocation.owner_browser_id = Some(canonical_browser_id.to_string());
            if duplicate_browser.display_allocation_id.as_deref() == Some(allocation.id.as_str())
                && state
                    .browsers
                    .get(canonical_browser_id)
                    .and_then(|browser| browser.display_allocation_id.as_deref())
                    != Some(allocation.id.as_str())
            {
                allocation.state = "released".to_string();
                allocation.updated_at = Some(current_timestamp());
                allocation.readiness = Some(json!({
                    "state": "released",
                    "reason": "duplicate_browser_record_merged",
                    "canonicalBrowserId": canonical_browser_id,
                    "duplicateBrowserId": duplicate_browser_id,
                    "updatedAt": allocation.updated_at,
                }));
            }
        }
    }
    for route in state.remote_view_routes.values_mut() {
        if route.browser_id.as_deref() == Some(duplicate_browser_id) {
            route.browser_id = Some(canonical_browser_id.to_string());
        }
    }
    for lease in state.viewer_leases.values_mut() {
        if lease.browser_id.as_deref() == Some(duplicate_browser_id) {
            lease.browser_id = Some(canonical_browser_id.to_string());
        }
    }

    push_service_event(
        state,
        ServiceEvent {
            kind: ServiceEventKind::Reconciliation,
            message: format!(
                "Merged duplicate browser record {} into {}",
                duplicate_browser_id, canonical_browser_id
            ),
            browser_id: Some(canonical_browser_id.to_string()),
            details: Some(json!({
                "action": "duplicate_browser_record_merged",
                "canonicalBrowserId": canonical_browser_id,
                "duplicateBrowserId": duplicate_browser_id,
                "cdpEndpoint": duplicate_browser.cdp_endpoint,
                "activeSessionIds": duplicate_browser.active_session_ids,
            })),
            ..new_service_event()
        },
    );
    true
}

fn clear_same_browser_profile_lease_conflicts(state: &mut ServiceState, browser_id: &str) {
    let session_ids = state
        .sessions
        .iter()
        .filter_map(|(session_id, session)| {
            session
                .browser_ids
                .iter()
                .any(|candidate| candidate == browser_id)
                .then_some(session_id.clone())
        })
        .collect::<BTreeSet<_>>();
    if session_ids.len() <= 1 {
        return;
    }
    for session_id in &session_ids {
        if let Some(session) = state.sessions.get_mut(session_id) {
            session
                .profile_lease_conflict_session_ids
                .retain(|conflict_id| !session_ids.contains(conflict_id));
        }
    }
}

pub async fn reconcile_persisted_service_state() -> Result<ServiceReconcileSummary, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    reconcile_service_state_in_repository(&repository).await
}

pub async fn reconcile_service_state_in_repository(
    repository: &impl ServiceStateRepository,
) -> Result<ServiceReconcileSummary, String> {
    let before = repository.load_snapshot()?;
    let mut reconciled_state = before.clone();
    let summary = reconcile_service_state(&mut reconciled_state).await;
    persist_reconciled_service_state_in_repository(repository, &before, &reconciled_state)?;
    Ok(summary)
}

pub fn persist_reconciled_service_state_in_repository(
    repository: &impl ServiceStateRepository,
    before: &ServiceState,
    reconciled: &ServiceState,
) -> Result<(), String> {
    let before = before.clone();
    let reconciled = reconciled.clone();
    repository.mutate(|state| {
        merge_reconciled_service_state(state, &before, &reconciled);
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn persist_service_browser_record_in_repository(
    repository: &impl ServiceStateRepository,
    session_id: &str,
    host: BrowserHost,
    health: BrowserHealth,
    pid: Option<u32>,
    cdp_endpoint: Option<String>,
    last_error: Option<String>,
    metadata: Option<ServiceLaunchMetadata>,
) -> Result<(), String> {
    repository.mutate(|service_state| {
        let id = service_browser_id_for_session(session_id);
        let previous = service_state.browsers.get(&id).cloned();
        let profile_id = metadata
            .as_ref()
            .and_then(|metadata| metadata.profile_id.clone())
            .or_else(|| {
                previous
                    .as_ref()
                    .and_then(|browser| browser.profile_id.clone())
            });
        let (view_streams, display_isolation, display_name) = match metadata.as_ref() {
            Some(metadata) => (
                metadata.view_streams.clone(),
                metadata.display_isolation.clone(),
                metadata.display_name.clone(),
            ),
            None => (
                previous
                    .as_ref()
                    .map(|browser| browser.view_streams.clone())
                    .unwrap_or_default(),
                previous
                    .as_ref()
                    .and_then(|browser| browser.display_isolation.clone()),
                previous
                    .as_ref()
                    .and_then(|browser| browser.display_name.clone()),
            ),
        };
        let mut browser = BrowserProcess {
            id: id.clone(),
            profile_id: profile_id.clone(),
            host,
            health,
            display_isolation,
            display_name,
            display_allocation_id: previous
                .as_ref()
                .and_then(|browser| browser.display_allocation_id.clone()),
            pid,
            cdp_endpoint,
            view_streams,
            active_session_ids: vec![session_id.to_string()],
            tab_handles: previous
                .as_ref()
                .map(|browser| browser.tab_handles.clone())
                .unwrap_or_default(),
            last_error,
            last_health_observation: None,
        };
        upsert_browser_display_allocation(
            service_state,
            session_id,
            &mut browser,
            metadata.as_ref(),
        );
        let observation_details = browser_health_observation_details(&browser, None);
        apply_browser_health_observation(&mut browser, Some(&observation_details));
        let metadata_changed = if let Some(metadata) = metadata.as_ref() {
            let previous_profile = profile_id
                .as_ref()
                .and_then(|profile_id| service_state.profiles.get(profile_id).cloned());
            let previous_session = service_state.sessions.get(session_id).cloned();
            upsert_service_profile_and_session(
                service_state,
                session_id,
                profile_id.clone(),
                metadata,
            );
            let current_profile = profile_id
                .as_ref()
                .and_then(|profile_id| service_state.profiles.get(profile_id).cloned());
            let current_session = service_state.sessions.get(session_id).cloned();
            previous_profile != current_profile || previous_session != current_session
        } else {
            false
        };
        record_browser_health_changed_event(service_state, &id, previous.as_ref(), &browser);
        if metadata_changed {
            record_browser_launch_recorded_event(
                service_state,
                &id,
                previous.as_ref(),
                &browser,
                metadata.as_ref(),
            );
        }
        service_state.browsers.insert(id, browser);
        Ok(())
    })
}

pub(crate) fn remove_browser_operational_record(
    service_state: &mut ServiceState,
    browser_id: &str,
    explicit_session_id: Option<&str>,
) -> usize {
    let removed_browser = service_state.browsers.remove(browser_id);
    let mut related_session_ids = BTreeSet::new();
    if let Some(session_id) = explicit_session_id {
        related_session_ids.insert(session_id.to_string());
    }
    if let Some(browser) = removed_browser.as_ref() {
        related_session_ids.extend(browser.active_session_ids.iter().cloned());
    }
    for (session_id, session) in &service_state.sessions {
        if session.browser_ids.iter().any(|id| id == browser_id) {
            related_session_ids.insert(session_id.clone());
        }
    }

    let tab_ids = service_state
        .tabs
        .iter()
        .filter(|(_, tab)| tab.browser_id == browser_id)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    for tab_id in &tab_ids {
        service_state.tabs.remove(tab_id);
    }

    for session in service_state.sessions.values_mut() {
        session.tab_ids.retain(|tab_id| !tab_ids.contains(tab_id));
        session.browser_ids.retain(|id| id != browser_id);
    }

    let removable_session_ids = related_session_ids
        .into_iter()
        .filter(|session_id| {
            service_state
                .sessions
                .get(session_id)
                .is_some_and(|session| session.browser_ids.is_empty() && session.tab_ids.is_empty())
        })
        .collect::<Vec<_>>();
    for session_id in &removable_session_ids {
        service_state.sessions.remove(session_id);
    }
    for browser in service_state.browsers.values_mut() {
        browser
            .active_session_ids
            .retain(|session_id| !removable_session_ids.contains(session_id));
    }
    service_state.refresh_derived_views();
    usize::from(removed_browser.is_some())
}

fn upsert_browser_display_allocation(
    service_state: &mut ServiceState,
    session_id: &str,
    browser: &mut BrowserProcess,
    metadata: Option<&ServiceLaunchMetadata>,
) {
    if browser.host != BrowserHost::RemoteHeaded {
        browser.display_allocation_id = None;
        return;
    }
    let Some(display_isolation) = browser.display_isolation.clone() else {
        return;
    };
    let allocation_id = browser.display_allocation_id.clone().unwrap_or_else(|| {
        display_allocation_id_for_browser(
            session_id,
            &display_isolation,
            browser.display_name.as_deref(),
        )
    });
    let now = current_timestamp();
    let allocation = service_state
        .display_allocations
        .entry(allocation_id.clone())
        .or_insert_with(|| DisplayAllocation {
            id: allocation_id.clone(),
            display_isolation: display_isolation.clone(),
            created_at: Some(now.clone()),
            ..DisplayAllocation::default()
        });
    allocation.display_name = browser.display_name.clone();
    allocation.display_isolation = display_isolation;
    allocation.owner_browser_id = Some(browser.id.clone());
    allocation.owner_session_id = Some(session_id.to_string());
    allocation.profile_id = browser.profile_id.clone();
    allocation.browser_build = metadata
        .and_then(|metadata| metadata.browser_capability_launch.as_ref())
        .and_then(|launch| launch.get("browserBuild"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    allocation.host = Some(browser.host);
    allocation.state = match browser.health {
        BrowserHealth::Ready => "ready",
        BrowserHealth::ProcessExited => "orphaned",
        _ => "allocating",
    }
    .to_string();
    allocation.pid_hints = browser
        .pid
        .map(|pid| serde_json::json!({ "browserPid": pid }));
    allocation.updated_at = Some(now);
    browser.display_allocation_id = Some(allocation_id.clone());
    for stream in &mut browser.view_streams {
        if stream.display_allocation_id.is_none() {
            stream.display_allocation_id = Some(allocation_id.clone());
        }
    }
}

fn display_allocation_id_for_browser(
    session_id: &str,
    display_isolation: &str,
    display_name: Option<&str>,
) -> String {
    let scope = match display_isolation {
        "private_virtual_display" => format!("session:{}", session_id),
        "shared_display" | "ambient_display" => display_name.unwrap_or(session_id).to_string(),
        _ => session_id.to_string(),
    };
    format!(
        "display:{}:{}",
        display_isolation,
        scope
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>()
            .trim_matches('-')
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn persist_current_browser_stale_health_in_repository(
    repository: &impl ServiceStateRepository,
    session_id: &str,
    pid: Option<u32>,
    cdp_endpoint: Option<String>,
    policy_config: BrowserRecoveryPolicyConfig,
    health: BrowserHealth,
    reason_kind: BrowserRecoveryReasonKind,
    last_error: String,
    event_details: Option<Value>,
) -> BrowserRecoveryPersistence {
    repository
        .mutate(|service_state| {
            let id = service_browser_id_for_session(session_id);
            let previous = service_state.browsers.get(&id).cloned();
            let mut browser = stale_browser_process_record(
                &id,
                session_id,
                previous.as_ref(),
                pid,
                cdp_endpoint.clone(),
                health,
                last_error.clone(),
            );
            let observation_details =
                browser_health_observation_details(&browser, event_details.clone());
            apply_browser_health_observation(&mut browser, Some(&observation_details));
            record_browser_health_changed_event_with_details(
                service_state,
                &id,
                previous.as_ref(),
                &browser,
                event_details.clone(),
            );
            let policy = recovery_policy_for_next_attempt(service_state, &id, policy_config);
            if policy.retry_budget_exceeded {
                let faulted_error = format!(
                    "Browser recovery retry budget exceeded after {} attempts; next retry delay {} ms",
                    policy.retry_budget, policy.next_retry_delay_ms
                );
                let mut faulted = BrowserProcess {
                    health: BrowserHealth::Faulted,
                    last_error: Some(faulted_error.clone()),
                    ..browser.clone()
                };
                let observation_details = browser_health_observation_details(&faulted, None);
                apply_browser_health_observation(&mut faulted, Some(&observation_details));
                record_browser_health_changed_event(service_state, &id, Some(&browser), &faulted);
                service_state.browsers.insert(id, faulted);
                return Ok(BrowserRecoveryPersistence::Blocked(faulted_error));
            }
            record_browser_recovery_started_event_with_details(
                service_state,
                &id,
                &browser,
                reason_kind,
                &last_error,
                Some(policy),
                event_details,
            );
            service_state.browsers.insert(id, browser);
            Ok(BrowserRecoveryPersistence::Recorded)
        })
        .unwrap_or(BrowserRecoveryPersistence::NotRecorded)
}

pub(crate) fn persist_browser_recovery_started_in_repository(
    repository: &impl ServiceStateRepository,
    session_id: &str,
    policy_config: BrowserRecoveryPolicyConfig,
    reason: &str,
) -> BrowserRecoveryPersistence {
    repository
        .mutate(|service_state| {
            let id = service_browser_id_for_session(session_id);
            let Some(browser) = service_state.browsers.get(&id).cloned() else {
                return Ok(BrowserRecoveryPersistence::NotRecorded);
            };
            if !matches!(
                browser.health,
                BrowserHealth::Degraded
                    | BrowserHealth::ProcessExited
                    | BrowserHealth::CdpDisconnected
                    | BrowserHealth::Unreachable
                    | BrowserHealth::Faulted
            ) {
                return Ok(BrowserRecoveryPersistence::NotRecorded);
            }
            let event_reason = browser.last_error.as_deref().unwrap_or(reason);
            let policy = recovery_policy_for_next_attempt(service_state, &id, policy_config);
            if policy.retry_budget_exceeded {
                let faulted_error = format!(
                    "Browser recovery retry budget exceeded after {} attempts; next retry delay {} ms",
                    policy.retry_budget, policy.next_retry_delay_ms
                );
                let mut faulted = BrowserProcess {
                    health: BrowserHealth::Faulted,
                    last_error: Some(faulted_error.clone()),
                    ..browser.clone()
                };
                let observation_details = browser_health_observation_details(&faulted, None);
                apply_browser_health_observation(&mut faulted, Some(&observation_details));
                record_browser_health_changed_event(service_state, &id, Some(&browser), &faulted);
                service_state.browsers.insert(id, faulted);
                return Ok(BrowserRecoveryPersistence::Blocked(faulted_error));
            }
            record_browser_recovery_started_event(
                service_state,
                &id,
                &browser,
                recovery_reason_kind_for_health(browser.health),
                event_reason,
                Some(policy),
            );
            Ok(BrowserRecoveryPersistence::Recorded)
        })
        .unwrap_or(BrowserRecoveryPersistence::NotRecorded)
}

pub(crate) fn stale_browser_process_record(
    id: &str,
    session_id: &str,
    previous: Option<&BrowserProcess>,
    pid: Option<u32>,
    cdp_endpoint: Option<String>,
    health: BrowserHealth,
    last_error: String,
) -> BrowserProcess {
    let mut browser = BrowserProcess {
        id: id.to_string(),
        profile_id: previous.and_then(|browser| browser.profile_id.clone()),
        host: previous
            .map(|browser| browser.host)
            .unwrap_or(BrowserHost::AttachedExisting),
        health,
        display_isolation: previous.and_then(|browser| browser.display_isolation.clone()),
        display_name: previous.and_then(|browser| browser.display_name.clone()),
        display_allocation_id: previous.and_then(|browser| browser.display_allocation_id.clone()),
        pid,
        cdp_endpoint,
        view_streams: previous
            .map(|browser| browser.view_streams.clone())
            .unwrap_or_default(),
        active_session_ids: vec![session_id.to_string()],
        tab_handles: previous
            .map(|browser| browser.tab_handles.clone())
            .unwrap_or_default(),
        last_error: Some(last_error),
        last_health_observation: None,
    };
    mark_cdp_screencast_streams_unavailable(&mut browser, "browser_not_ready", health);
    browser
}

pub(crate) fn close_health_from_outcome(
    outcome: Option<&BrowserShutdownOutcome>,
) -> (BrowserHealth, Option<String>) {
    let Some(outcome) = outcome else {
        return (BrowserHealth::NotStarted, None);
    };
    if outcome.os_degraded_possible() {
        return (
            BrowserHealth::Faulted,
            Some(format!(
                "Force kill failed; OS may be degraded. {}",
                outcome.errors.join("; ")
            )),
        );
    }
    if outcome.browser_degraded() {
        return (
            BrowserHealth::Degraded,
            Some(format!(
                "Polite browser close failed; force kill was required. {}",
                outcome.errors.join("; ")
            )),
        );
    }
    (BrowserHealth::NotStarted, None)
}

fn release_browser_display_allocation_after_close(
    service_state: &mut ServiceState,
    browser: &BrowserProcess,
    session_id: &str,
    outcome: Option<&BrowserShutdownOutcome>,
) {
    let Some(display_allocation_id) = browser.display_allocation_id.as_ref() else {
        return;
    };
    let (state, failure_class) = match outcome {
        Some(outcome) if outcome.os_degraded_possible() => {
            ("degraded", "browser_shutdown_force_kill_failed")
        }
        Some(outcome) if outcome.browser_degraded() => ("degraded", "browser_shutdown_degraded"),
        _ => ("released", "operator_requested_close"),
    };
    let now = current_timestamp();
    let route_ids = {
        let Some(allocation) = service_state
            .display_allocations
            .get_mut(display_allocation_id)
        else {
            return;
        };
        if allocation.owner_browser_id.as_deref() != Some(browser.id.as_str())
            || allocation.owner_session_id.as_deref() != Some(session_id)
        {
            return;
        }

        let route_ids = allocation.route_ids.clone();
        allocation.state = state.to_string();
        allocation.updated_at = Some(now.clone());
        allocation.readiness = Some(serde_json::json!({
            "state": state,
            "reason": "operator_requested_close",
            "failureClass": failure_class,
            "updatedAt": now,
        }));
        if state == "released" {
            allocation.route_ids.clear();
        }
        route_ids
    };
    if state == "released" {
        for route_id in route_ids {
            if let Some(route) = service_state.remote_view_routes.get_mut(&route_id) {
                if route.display_allocation_id.as_deref() == Some(display_allocation_id.as_str())
                    && route.browser_id.as_deref() == Some(browser.id.as_str())
                {
                    route.state = "released".to_string();
                    route.last_provider_event =
                        Some("route_released_after_browser_close".to_string());
                    route.controller_lease_id = None;
                    route.viewer_lease_ids.clear();
                    route.readiness = Some(serde_json::json!({
                        "state": "released",
                        "reason": "browser_closed",
                        "updatedAt": now,
                    }));
                }
            }
            for entry in service_state.route_pool.values_mut() {
                if entry.current_route_allocation_id.as_deref() == Some(route_id.as_str()) {
                    entry.state = "available".to_string();
                    entry.current_route_allocation_id = None;
                }
            }
        }
    }
}

pub(crate) fn persist_closed_browser_health_in_repository(
    repository: &impl ServiceStateRepository,
    session_id: &str,
    outcome: Option<&BrowserShutdownOutcome>,
) -> Result<(), String> {
    repository.mutate(|service_state| {
        let id = service_browser_id_for_session(session_id);
        let previous = service_state.browsers.get(&id).cloned();
        let host = previous
            .as_ref()
            .map(|browser| browser.host)
            .unwrap_or(BrowserHost::LocalHeaded);
        let (health, last_error) = close_health_from_outcome(outcome);
        let mut browser = BrowserProcess {
            id: id.clone(),
            profile_id: previous
                .as_ref()
                .and_then(|browser| browser.profile_id.clone()),
            host,
            health,
            display_isolation: previous
                .as_ref()
                .and_then(|browser| browser.display_isolation.clone()),
            display_name: previous
                .as_ref()
                .and_then(|browser| browser.display_name.clone()),
            display_allocation_id: previous
                .as_ref()
                .and_then(|browser| browser.display_allocation_id.clone()),
            last_error,
            view_streams: previous
                .as_ref()
                .map(|browser| browser.view_streams.clone())
                .unwrap_or_default(),
            active_session_ids: vec![session_id.to_string()],
            ..BrowserProcess::default()
        };
        mark_cdp_screencast_streams_unavailable(&mut browser, "operator_requested_close", health);
        let shutdown_details = outcome.map(|outcome| {
            let failure_class = if outcome.os_degraded_possible() {
                "browser_shutdown_force_kill_failed"
            } else if outcome.browser_degraded() {
                "browser_shutdown_degraded"
            } else {
                "operator_requested_close"
            };
            serde_json::json!({
                "shutdownReasonKind": BrowserRecoveryReasonKind::OperatorRequestedClose.as_str(),
                "processExitCause": BrowserProcessExitCause::OperatorRequestedClose.as_str(),
                "failureClass": failure_class,
                "shutdownRequested": true,
                "politeCloseAttempted": outcome.polite_close_attempted,
                "politeCloseSucceeded": outcome.polite_close_succeeded,
                "politeCloseFailed": outcome.polite_close_failed,
                "forceKillAttempted": outcome.force_kill_attempted,
                "forceKillSucceeded": outcome.force_kill_succeeded,
                "forceKillFailed": outcome.force_kill_failed,
            })
        });
        if let Some(details) = shutdown_details.as_ref() {
            let observation_details =
                browser_health_observation_details(&browser, Some(details.clone()));
            apply_browser_health_observation(&mut browser, Some(&observation_details));
        } else {
            let observation_details = browser_health_observation_details(&browser, None);
            apply_browser_health_observation(&mut browser, Some(&observation_details));
        }
        record_browser_health_changed_event_with_details(
            service_state,
            &id,
            previous.as_ref(),
            &browser,
            shutdown_details,
        );
        release_browser_display_allocation_after_close(
            service_state,
            &browser,
            session_id,
            outcome,
        );
        if let Some(session) = service_state.sessions.get_mut(session_id) {
            session.lease = LeaseState::Released;
            session.last_lease_observed_at = Some(current_timestamp());
            session.profile_lease_conflict_session_ids.clear();
        }
        if health == BrowserHealth::NotStarted {
            remove_browser_operational_record(service_state, &id, Some(session_id));
        } else {
            service_state.browsers.insert(id, browser);
        }
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn retry_persisted_service_browser_in_repository(
    repository: &impl ServiceStateRepository,
    browser_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Result<(BrowserProcess, Option<ServiceIncident>), String> {
    repository
        .mutate(|state| {
            retry_service_browser_in_state(
                state,
                browser_id,
                timestamp,
                actor,
                note,
                service_name,
                agent_name,
                task_name,
            )
        })
        .map_err(|err| {
            if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
                format!("Unable to load service state: {}", err)
            } else {
                err
            }
        })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn retry_service_browser_in_state(
    state: &mut ServiceState,
    browser_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Result<(BrowserProcess, Option<ServiceIncident>), String> {
    enable_service_browser_retry_in_state(
        state,
        browser_id,
        timestamp,
        actor,
        note,
        service_name,
        agent_name,
        task_name,
        BrowserHealth::Faulted,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn retry_degraded_service_browser_in_state(
    state: &mut ServiceState,
    browser_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> Result<(BrowserProcess, Option<ServiceIncident>), String> {
    enable_service_browser_retry_in_state(
        state,
        browser_id,
        timestamp,
        actor,
        note,
        service_name,
        agent_name,
        task_name,
        BrowserHealth::Degraded,
    )
}

#[allow(clippy::too_many_arguments)]
fn enable_service_browser_retry_in_state(
    state: &mut ServiceState,
    browser_id: &str,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
    expected_health: BrowserHealth,
) -> Result<(BrowserProcess, Option<ServiceIncident>), String> {
    let previous = state
        .browsers
        .get(browser_id)
        .cloned()
        .ok_or_else(|| format!("Service browser not found: {}", browser_id))?;
    if previous.health != expected_health {
        return Err(format!(
            "Service browser {} is not {}; current health is {}",
            browser_id,
            service_browser_health_name(expected_health),
            service_browser_health_name(previous.health)
        ));
    }

    let mut retryable = previous.clone();
    retryable.health = BrowserHealth::ProcessExited;
    retryable.last_error = Some(format!(
        "Browser retry requested by {}; previous {} state: {}",
        actor,
        service_browser_health_name(previous.health),
        previous
            .last_error
            .as_deref()
            .unwrap_or("no prior detail recorded")
    ));
    let observation_details = browser_health_observation_details(&retryable, None);
    apply_browser_health_observation(&mut retryable, Some(&observation_details));
    record_browser_health_changed_event(state, browser_id, Some(&previous), &retryable);
    state
        .browsers
        .insert(browser_id.to_string(), retryable.clone());
    push_service_event(
        state,
        service_browser_recovery_override_event(
            &retryable,
            previous.health,
            timestamp,
            actor,
            note,
            service_name,
            agent_name,
            task_name,
        ),
    );
    state.refresh_derived_views();
    let incident = state
        .incidents
        .iter()
        .find(|incident| incident.id == browser_id)
        .cloned();
    Ok((retryable, incident))
}

/// Merge async reconciliation results without clobbering newer state mutations.
///
/// Reconciliation may spend time probing PIDs and CDP endpoints. During that
/// window, queued service commands can append jobs, change config, or record
/// operator events. This merge applies only the fields reconciliation owns.
pub fn merge_reconciled_service_state(
    target: &mut ServiceState,
    before: &ServiceState,
    reconciled: &ServiceState,
) {
    if reconciled.control_plane.is_some() {
        target.control_plane = reconciled.control_plane.clone();
    }
    if reconciled.reconciliation.is_some() {
        target.reconciliation = reconciled.reconciliation.clone();
    }

    for (id, reconciled_browser) in &reconciled.browsers {
        match target.browsers.get_mut(id) {
            Some(target_browser) => {
                target_browser.health = reconciled_browser.health;
                target_browser.last_error = reconciled_browser.last_error.clone();
                target_browser.last_health_observation =
                    reconciled_browser.last_health_observation.clone();
            }
            None => {
                target
                    .browsers
                    .insert(id.clone(), reconciled_browser.clone());
            }
        }
    }
    for id in before.browsers.keys() {
        if reconciled.browsers.contains_key(id) {
            continue;
        }
        let should_remove = target
            .browsers
            .get(id)
            .zip(before.browsers.get(id))
            .is_some_and(|(target_browser, before_browser)| target_browser == before_browser);
        if should_remove {
            target.browsers.remove(id);
        }
    }

    for (id, reconciled_tab) in &reconciled.tabs {
        target.tabs.insert(id.clone(), reconciled_tab.clone());
    }
    for id in before.tabs.keys() {
        if reconciled.tabs.contains_key(id) {
            continue;
        }
        let should_remove = target
            .tabs
            .get(id)
            .zip(before.tabs.get(id))
            .is_some_and(|(target_tab, before_tab)| target_tab == before_tab);
        if should_remove {
            target.tabs.remove(id);
        }
    }

    for (id, reconciled_session) in &reconciled.sessions {
        let target_session = target
            .sessions
            .entry(id.clone())
            .or_insert_with(|| reconciled_session.clone());
        target_session.browser_ids = reconciled_session.browser_ids.clone();
        target_session.tab_ids = reconciled_session.tab_ids.clone();
    }
    for id in before.sessions.keys() {
        if reconciled.sessions.contains_key(id) {
            continue;
        }
        let should_remove = target
            .sessions
            .get(id)
            .zip(before.sessions.get(id))
            .is_some_and(|(target_session, before_session)| target_session == before_session);
        if should_remove {
            target.sessions.remove(id);
        }
    }

    for (id, reconciled_allocation) in &reconciled.display_allocations {
        target
            .display_allocations
            .insert(id.clone(), reconciled_allocation.clone());
    }

    for (id, reconciled_route) in &reconciled.remote_view_routes {
        target
            .remote_view_routes
            .insert(id.clone(), reconciled_route.clone());
    }

    for (id, reconciled_lease) in &reconciled.viewer_leases {
        target
            .viewer_leases
            .insert(id.clone(), reconciled_lease.clone());
    }

    for (id, reconciled_pool_entry) in &reconciled.route_pool {
        target
            .route_pool
            .insert(id.clone(), reconciled_pool_entry.clone());
    }

    let before_event_ids = before
        .events
        .iter()
        .map(|event| event.id.clone())
        .collect::<BTreeSet<_>>();
    let mut target_event_ids = target
        .events
        .iter()
        .map(|event| event.id.clone())
        .collect::<BTreeSet<_>>();
    for event in &reconciled.events {
        if before_event_ids.contains(&event.id) || target_event_ids.contains(&event.id) {
            continue;
        }
        target_event_ids.insert(event.id.clone());
        push_service_event(target, event.clone());
    }

    target.refresh_derived_views();
}

pub async fn refresh_persisted_browser_health(state: &mut ServiceState) {
    for browser in state.browsers.values_mut() {
        refresh_browser_record_health(browser).await;
    }
}

async fn refresh_browser_record_health(browser: &mut BrowserProcess) {
    if matches!(
        browser.health,
        BrowserHealth::NotStarted | BrowserHealth::Launching | BrowserHealth::Closing
    ) {
        return;
    }

    if let Some(pid) = browser.pid {
        if !pid_is_running(pid) {
            browser.health = BrowserHealth::ProcessExited;
            browser.last_error = Some(format!("Recorded browser PID {} is no longer running", pid));
            let details = serde_json::json!({
                "currentReasonKind": recovery_reason_kind_for_health(browser.health).as_str(),
                "failureClass": "browser_process_exited",
                "processExitCause": BrowserProcessExitCause::UnexpectedProcessExit.as_str(),
                "processExitDetection": "persisted_pid_probe",
                "processExitPid": pid,
            });
            apply_browser_health_observation(browser, Some(&details));
            return;
        }
    }

    if let Some(endpoint) = browser.cdp_endpoint.as_deref() {
        if cdp_endpoint_reachable(endpoint).await {
            browser.health = BrowserHealth::Ready;
            browser.last_error = None;
            browser.last_health_observation = None;
        } else if browser.pid.is_some() {
            browser.health = BrowserHealth::CdpDisconnected;
            browser.last_error = Some(format!("CDP endpoint is unreachable: {}", endpoint));
            let details = serde_json::json!({
                "currentReasonKind": recovery_reason_kind_for_health(browser.health).as_str(),
                "failureClass": "cdp_unresponsive",
                "cdpProbe": "Browser.getVersion",
                "cdpEndpoint": endpoint,
            });
            apply_browser_health_observation(browser, Some(&details));
        } else {
            browser.health = BrowserHealth::Unreachable;
            browser.last_error = Some(format!("CDP endpoint is unreachable: {}", endpoint));
            let details = serde_json::json!({
                "currentReasonKind": recovery_reason_kind_for_health(browser.health).as_str(),
                "failureClass": "cdp_endpoint_unreachable",
                "cdpProbe": "Browser.getVersion",
                "cdpEndpoint": endpoint,
            });
            apply_browser_health_observation(browser, Some(&details));
        }
    }
}

async fn reconcile_live_browser_targets(state: &mut ServiceState) {
    let browser_records = state
        .browsers
        .iter()
        .map(|(id, browser)| (id.clone(), browser.clone()))
        .collect::<Vec<_>>();

    for (browser_id, browser) in browser_records {
        if browser.health != BrowserHealth::Ready {
            close_browser_tabs(state, &browser_id);
            continue;
        }
        let Some(endpoint) = browser.cdp_endpoint.as_deref() else {
            close_browser_tabs(state, &browser_id);
            continue;
        };
        let targets = match fetch_cdp_targets(endpoint).await {
            Ok(targets) => targets,
            Err(err) => {
                if let Some(browser) = state.browsers.get_mut(&browser_id) {
                    browser.health = BrowserHealth::Degraded;
                    browser.last_error = Some(err.clone());
                    let details = serde_json::json!({
                        "currentReasonKind": recovery_reason_kind_for_health(browser.health).as_str(),
                        "failureClass": "target_discovery_failed",
                        "cdpProbe": "/json/list",
                        "cdpEndpoint": endpoint,
                    });
                    apply_browser_health_observation(browser, Some(&details));
                }
                close_browser_tabs(state, &browser_id);
                continue;
            }
        };
        reconcile_browser_targets(state, &browser_id, &browser, targets);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct RemoteViewReconcileRepair {
    orphaned_display_allocations: usize,
    orphaned_routes: usize,
    released_viewer_leases: usize,
    expired_viewer_leases: usize,
    cleared_controller_leases: usize,
}

fn reconcile_remote_view_state(state: &mut ServiceState) -> RemoteViewReconcileRepair {
    let now = current_timestamp();
    let browser_health = state
        .browsers
        .iter()
        .map(|(id, browser)| (id.clone(), browser.health))
        .collect::<BTreeMap<_, _>>();
    let mut repair = RemoteViewReconcileRepair::default();

    for allocation in state.display_allocations.values_mut() {
        if matches!(allocation.state.as_str(), "released" | "failed") {
            continue;
        }
        let Some(owner_browser_id) = allocation.owner_browser_id.as_deref() else {
            continue;
        };
        if browser_health.get(owner_browser_id) == Some(&BrowserHealth::Ready) {
            continue;
        }
        allocation.state = "orphaned".to_string();
        allocation.updated_at = Some(now.clone());
        allocation.last_health_check_at = Some(now.clone());
        allocation.readiness = Some(json!({
            "state": "orphaned",
            "component": "browser_process",
            "reason": "owner_browser_not_ready",
            "browserId": owner_browser_id,
            "browserHealth": browser_health
                .get(owner_browser_id)
                .map(|health| service_browser_health_name(*health))
                .unwrap_or("missing"),
            "updatedAt": now,
        }));
        repair.orphaned_display_allocations += 1;
    }

    let display_states = state
        .display_allocations
        .iter()
        .map(|(id, allocation)| (id.clone(), allocation.state.clone()))
        .collect::<BTreeMap<_, _>>();

    for route in state.remote_view_routes.values_mut() {
        if matches!(route.state.as_str(), "released" | "failed") {
            continue;
        }
        let display_problem = route
            .display_allocation_id
            .as_ref()
            .and_then(|id| {
                display_states
                    .get(id)
                    .map(|state| (id.clone(), state.clone()))
            })
            .and_then(|(id, state)| {
                if matches!(state.as_str(), "ready" | "allocating") {
                    None
                } else {
                    Some(("display_allocation_unavailable", id, state))
                }
            })
            .or_else(|| {
                route
                    .display_allocation_id
                    .as_ref()
                    .filter(|id| !display_states.contains_key(*id))
                    .map(|id| {
                        (
                            "display_allocation_missing",
                            id.clone(),
                            "missing".to_string(),
                        )
                    })
            });
        let browser_problem = route
            .browser_id
            .as_ref()
            .and_then(|id| browser_health.get(id).map(|health| (id.clone(), *health)))
            .and_then(|(id, health)| {
                if health == BrowserHealth::Ready {
                    None
                } else {
                    Some((
                        "browser_not_ready",
                        id,
                        service_browser_health_name(health).to_string(),
                    ))
                }
            })
            .or_else(|| {
                route
                    .browser_id
                    .as_ref()
                    .filter(|id| !browser_health.contains_key(*id))
                    .map(|id| ("browser_missing", id.clone(), "missing".to_string()))
            });
        let Some((reason, entity_id, entity_state)) = display_problem.or(browser_problem) else {
            continue;
        };
        route.state = "orphaned".to_string();
        route.last_provider_event = Some(reason.to_string());
        route.readiness = Some(json!({
            "state": "orphaned",
            "component": if reason.starts_with("display") { "display_allocation" } else { "browser_process" },
            "reason": reason,
            "entityId": entity_id,
            "entityState": entity_state,
            "updatedAt": now,
        }));
        repair.orphaned_routes += 1;
    }

    let route_states = state
        .remote_view_routes
        .iter()
        .map(|(id, route)| (id.clone(), route.state.clone()))
        .collect::<BTreeMap<_, _>>();

    for lease in state.viewer_leases.values_mut() {
        if !viewer_lease_is_reconcile_active(lease) {
            continue;
        }
        let route_unavailable = lease
            .route_id
            .as_ref()
            .map(|id| {
                !matches!(
                    route_states.get(id).map(String::as_str),
                    Some("ready" | "reconnecting" | "allocating")
                )
            })
            .unwrap_or(true);
        if viewer_lease_is_expired(lease, &now) {
            lease.state = "expired".to_string();
            lease.last_viewer_event = Some("expired".to_string());
            lease.updated_at = Some(now.clone());
            lease.last_heartbeat_at = Some(now.clone());
            repair.expired_viewer_leases += 1;
        } else if route_unavailable {
            lease.state = "disconnected".to_string();
            lease.last_viewer_event = Some("route_unavailable".to_string());
            lease.updated_at = Some(now.clone());
            lease.last_heartbeat_at = Some(now.clone());
            repair.released_viewer_leases += 1;
        }
    }

    let active_viewer_leases = state
        .viewer_leases
        .iter()
        .filter(|(_id, lease)| viewer_lease_is_reconcile_active(lease))
        .map(|(id, _lease)| id.clone())
        .collect::<BTreeSet<_>>();
    for route in state.remote_view_routes.values_mut() {
        let before_controller = route.controller_lease_id.clone();
        route
            .viewer_lease_ids
            .retain(|id| active_viewer_leases.contains(id));
        if route
            .controller_lease_id
            .as_ref()
            .is_some_and(|id| !active_viewer_leases.contains(id))
        {
            route.controller_lease_id = None;
        }
        if before_controller.is_some() && route.controller_lease_id.is_none() {
            repair.cleared_controller_leases += 1;
        }
    }

    for browser in state.browsers.values_mut() {
        for stream in &mut browser.view_streams {
            stream
                .viewer_lease_ids
                .retain(|id| active_viewer_leases.contains(id));
            if stream
                .controller_lease_id
                .as_ref()
                .is_some_and(|id| !active_viewer_leases.contains(id))
            {
                stream.controller_lease_id = None;
            }
            if let Some(route_id) = stream.route_id.as_ref() {
                if let Some(route) = state.remote_view_routes.get(route_id) {
                    stream.remote_readiness = route.readiness.clone();
                }
            }
        }
    }

    repair
}

fn viewer_lease_is_reconcile_active(lease: &super::service_model::ViewerLease) -> bool {
    !matches!(
        lease.state.as_str(),
        "disconnected" | "expired" | "failed" | "released"
    )
}

fn viewer_lease_is_expired(lease: &super::service_model::ViewerLease, now: &str) -> bool {
    lease
        .expires_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|expires_at| expires_at <= now)
}

fn reconcile_browser_targets(
    state: &mut ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
    targets: Vec<CdpHttpTargetInfo>,
) {
    let mut live_tab_ids = BTreeSet::new();
    let owner_session_id = browser.active_session_ids.first().cloned();

    for target in targets.into_iter().filter(should_track_service_target) {
        let tab_id = format!("target:{}", target.id);
        live_tab_ids.insert(tab_id.clone());
        state.tabs.insert(
            tab_id.clone(),
            BrowserTab {
                id: tab_id.clone(),
                browser_id: browser_id.to_string(),
                target_id: Some(target.id),
                lifecycle: TabLifecycle::Ready,
                url: empty_to_none(target.url),
                title: empty_to_none(target.title),
                owner_session_id: owner_session_id.clone(),
                ..state.tabs.get(&tab_id).cloned().unwrap_or_default()
            },
        );
    }

    for tab in state.tabs.values_mut() {
        if tab.browser_id == browser_id && !live_tab_ids.contains(&tab.id) {
            tab.lifecycle = TabLifecycle::Closed;
        }
    }

    if let Some(repair) = refresh_session_tab_relationships_for_browser(
        state,
        browser_id,
        owner_session_id.clone(),
        &live_tab_ids,
    ) {
        record_session_tab_ownership_repaired_event(state, browser_id, browser, repair);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionTabOwnershipRepair {
    owner_session_id: Option<String>,
    live_tab_ids: Vec<String>,
    removed_relations: Vec<SessionTabOwnershipRepairRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionTabOwnershipRepairRelation {
    session_id: String,
    tab_id: String,
}

fn refresh_session_tab_relationships_for_browser(
    state: &mut ServiceState,
    browser_id: &str,
    owner_session_id: Option<String>,
    live_tab_ids: &BTreeSet<String>,
) -> Option<SessionTabOwnershipRepair> {
    let browser_tab_ids = state
        .tabs
        .iter()
        .filter_map(|(tab_id, tab)| {
            if tab.browser_id == browser_id {
                Some(tab_id.clone())
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();

    let mut removed_relations = Vec::new();
    for (session_id, session) in state.sessions.iter_mut() {
        session.tab_ids.retain(|tab_id| {
            if !browser_tab_ids.contains(tab_id) {
                return true;
            }
            let is_current_owner_live_tab = owner_session_id.as_deref()
                == Some(session_id.as_str())
                && live_tab_ids.contains(tab_id);
            if !is_current_owner_live_tab {
                removed_relations.push(SessionTabOwnershipRepairRelation {
                    session_id: session_id.clone(),
                    tab_id: tab_id.clone(),
                });
            }
            false
        });
    }

    if let Some(owner_session_id) = owner_session_id.clone() {
        let session = state
            .sessions
            .entry(owner_session_id.clone())
            .or_insert_with(|| BrowserSession {
                id: owner_session_id.clone(),
                ..BrowserSession::default()
            });
        merge_unique(&mut session.browser_ids, browser_id.to_string());
        for tab_id in live_tab_ids {
            merge_unique(&mut session.tab_ids, tab_id.clone());
        }
    }

    (!removed_relations.is_empty()).then(|| SessionTabOwnershipRepair {
        owner_session_id,
        live_tab_ids: live_tab_ids.iter().cloned().collect(),
        removed_relations,
    })
}

fn record_session_tab_ownership_repaired_event(
    state: &mut ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
    repair: SessionTabOwnershipRepair,
) {
    let removed_relations = repair
        .removed_relations
        .iter()
        .map(|relation| {
            serde_json::json!({
                "sessionId": relation.session_id,
                "tabId": relation.tab_id,
            })
        })
        .collect::<Vec<_>>();
    let mut event = ServiceEvent {
        kind: ServiceEventKind::Reconciliation,
        message: format!(
            "Repaired {} stale session tab ownership relationships for browser {}",
            repair.removed_relations.len(),
            browser_id
        ),
        browser_id: Some(browser_id.to_string()),
        details: Some(serde_json::json!({
            "action": "session_tab_ownership_repaired",
            "browserId": browser_id,
            "ownerSessionId": repair.owner_session_id,
            "liveTabIds": repair.live_tab_ids,
            "removedRelationshipCount": repair.removed_relations.len(),
            "removedRelations": removed_relations,
        })),
        ..new_service_event()
    };
    enrich_service_event_with_browser_context(&mut event, state, browser_id, browser);
    push_service_event(state, event);
}

fn close_browser_tabs(state: &mut ServiceState, browser_id: &str) {
    let closed_tab_ids = state
        .tabs
        .values_mut()
        .filter_map(|tab| {
            if tab.browser_id == browser_id && tab.lifecycle != TabLifecycle::Closed {
                tab.lifecycle = TabLifecycle::Closed;
                Some(tab.id.clone())
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();

    if closed_tab_ids.is_empty() {
        return;
    }

    for session in state.sessions.values_mut() {
        session
            .tab_ids
            .retain(|tab_id| !closed_tab_ids.contains(tab_id));
    }
}

async fn fetch_cdp_targets(endpoint: &str) -> Result<Vec<CdpHttpTargetInfo>, String> {
    let Some(url) = cdp_list_url(endpoint) else {
        return Err("Invalid CDP endpoint".to_string());
    };
    let client = reqwest::Client::builder()
        .timeout(CDP_PROBE_TIMEOUT)
        .build()
        .map_err(|err| format!("Failed to build CDP target client: {}", err))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("Failed to fetch CDP targets: {}", err))?;
    if !response.status().is_success() {
        return Err(format!("CDP target list returned {}", response.status()));
    }
    response
        .json::<Vec<CdpHttpTargetInfo>>()
        .await
        .map_err(|err| format!("Failed to parse CDP targets: {}", err))
}

async fn cdp_endpoint_reachable(endpoint: &str) -> bool {
    let Some(url) = cdp_version_url(endpoint) else {
        return false;
    };
    let Ok(client) = reqwest::Client::builder()
        .timeout(CDP_PROBE_TIMEOUT)
        .build()
    else {
        return false;
    };
    let Ok(response) = client.get(url).send().await else {
        return false;
    };
    response.status().is_success()
}

fn cdp_list_url(endpoint: &str) -> Option<String> {
    let mut url = cdp_root_url(endpoint)?;
    url.set_path("/json/list");
    Some(url.to_string())
}

fn cdp_version_url(endpoint: &str) -> Option<String> {
    let mut url = cdp_root_url(endpoint)?;
    url.set_path("/json/version");
    Some(url.to_string())
}

fn cdp_root_url(endpoint: &str) -> Option<url::Url> {
    let mut url = url::Url::parse(endpoint).ok()?;
    match url.scheme() {
        "ws" => url.set_scheme("http").ok()?,
        "wss" => url.set_scheme("https").ok()?,
        "http" | "https" => {}
        _ => return None,
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url)
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn new_service_event() -> ServiceEvent {
    let timestamp = current_timestamp();
    ServiceEvent {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        timestamp,
        ..ServiceEvent::default()
    }
}

fn push_service_event(state: &mut ServiceState, event: ServiceEvent) {
    state.events.push(event);
    if state.events.len() > MAX_SERVICE_EVENTS {
        let excess = state.events.len() - MAX_SERVICE_EVENTS;
        state.events.drain(0..excess);
    }
}

fn service_browser_health_name(health: BrowserHealth) -> &'static str {
    match health {
        BrowserHealth::NotStarted => "not_started",
        BrowserHealth::Launching => "launching",
        BrowserHealth::Ready => "ready",
        BrowserHealth::Degraded => "degraded",
        BrowserHealth::ProcessExited => "process_exited",
        BrowserHealth::CdpDisconnected => "cdp_disconnected",
        BrowserHealth::Unreachable => "unreachable",
        BrowserHealth::Closing => "closing",
        BrowserHealth::Faulted => "faulted",
        BrowserHealth::Reconnecting => "reconnecting",
    }
}

#[allow(clippy::too_many_arguments)]
fn service_browser_recovery_override_event(
    browser: &BrowserProcess,
    previous_health: BrowserHealth,
    timestamp: &str,
    actor: &str,
    note: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> ServiceEvent {
    let mut details = serde_json::json!({
        "incidentId": browser.id,
        "actor": actor,
        "action": "retry_enabled",
        "previousHealth": service_browser_health_name(previous_health),
        "currentHealth": service_browser_health_name(browser.health),
    });
    if let Some(note) = note {
        details["note"] = serde_json::json!(note);
    }
    ServiceEvent {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        timestamp: timestamp.to_string(),
        kind: ServiceEventKind::BrowserRecoveryOverride,
        message: format!("Browser {} recovery retry enabled by {}", browser.id, actor),
        browser_id: Some(browser.id.clone()),
        profile_id: browser.profile_id.clone(),
        session_id: browser.active_session_ids.first().cloned(),
        service_name: service_name.map(str::to_string),
        agent_name: agent_name.map(str::to_string),
        task_name: task_name.map(str::to_string),
        previous_health: Some(previous_health),
        current_health: Some(browser.health),
        details: Some(details),
    }
}

pub fn record_browser_health_changed_event(
    state: &mut ServiceState,
    browser_id: &str,
    previous: Option<&BrowserProcess>,
    current: &BrowserProcess,
) {
    record_browser_health_changed_event_with_details(state, browser_id, previous, current, None);
}

pub fn record_browser_health_changed_event_with_details(
    state: &mut ServiceState,
    browser_id: &str,
    previous: Option<&BrowserProcess>,
    current: &BrowserProcess,
    extra_details: Option<serde_json::Value>,
) {
    let Some(previous) = previous else {
        return;
    };
    if previous.health == current.health && previous.last_error == current.last_error {
        return;
    }
    let mut details = serde_json::json!({
        "previousError": previous.last_error,
        "currentError": current.last_error,
    });
    if let Some(reason_kind) = recovery_reason_kind_value_for_health(current.health) {
        details["currentReasonKind"] = reason_kind;
    }
    if let Some(failure_class) = failure_class_for_health(current.health) {
        details["failureClass"] = serde_json::json!(failure_class);
    }
    if let Some(cause) = process_exit_cause_for_health(current.health) {
        details["processExitCause"] = serde_json::json!(cause.as_str());
    }
    if let Some(reason_kind) = recovery_reason_kind_value_for_health(previous.health) {
        details["previousReasonKind"] = reason_kind;
    }
    if let Some(serde_json::Value::Object(extra)) = extra_details {
        if let Some(details) = details.as_object_mut() {
            for (key, value) in extra {
                details.insert(key, value);
            }
        }
    }
    let mut event = ServiceEvent {
        kind: ServiceEventKind::BrowserHealthChanged,
        message: format!(
            "Browser {} health changed from {:?} to {:?}",
            browser_id, previous.health, current.health
        ),
        browser_id: Some(browser_id.to_string()),
        previous_health: Some(previous.health),
        current_health: Some(current.health),
        details: Some(details),
        ..new_service_event()
    };
    enrich_service_event_with_browser_context(&mut event, state, browser_id, current);
    push_service_event(state, event);
}

pub fn record_browser_launch_recorded_event(
    state: &mut ServiceState,
    browser_id: &str,
    previous: Option<&BrowserProcess>,
    current: &BrowserProcess,
    metadata: Option<&ServiceLaunchMetadata>,
) {
    let profile_selection_reason = current
        .active_session_ids
        .first()
        .and_then(|session_id| state.sessions.get(session_id))
        .and_then(|session| session.profile_selection_reason);
    let profile_lease_disposition = current
        .active_session_ids
        .first()
        .and_then(|session_id| state.sessions.get(session_id))
        .and_then(|session| session.profile_lease_disposition);
    let profile_lease_conflict_session_ids = current
        .active_session_ids
        .first()
        .and_then(|session_id| state.sessions.get(session_id))
        .map(|session| session.profile_lease_conflict_session_ids.clone())
        .unwrap_or_default();
    let mut details = serde_json::json!({
        "previousProfileId": previous.and_then(|browser| browser.profile_id.clone()),
        "currentProfileId": current.profile_id,
        "previousSessionIds": previous
            .map(|browser| browser.active_session_ids.clone())
            .unwrap_or_default(),
        "currentSessionIds": current.active_session_ids,
        "host": current.host,
        "pid": current.pid,
        "cdpEndpoint": current.cdp_endpoint,
        "profileSelectionReason": profile_selection_reason,
        "profileLeaseDisposition": profile_lease_disposition,
        "profileLeaseConflictSessionIds": profile_lease_conflict_session_ids,
    });
    if let Some(path) = metadata.and_then(|metadata| metadata.browser_stderr_log_path.as_deref()) {
        details["browserStderrLogPath"] = serde_json::json!(path);
    }
    if let Some(browser_capability_launch) =
        metadata.and_then(|metadata| metadata.browser_capability_launch.clone())
    {
        details["browserCapabilityLaunch"] = browser_capability_launch;
    }
    let mut event = ServiceEvent {
        kind: ServiceEventKind::BrowserLaunchRecorded,
        message: format!("Browser {} launch metadata recorded", browser_id),
        browser_id: Some(browser_id.to_string()),
        current_health: Some(current.health),
        details: Some(details),
        ..new_service_event()
    };
    enrich_service_event_with_browser_context(&mut event, state, browser_id, current);
    push_service_event(state, event);
}

pub fn record_browser_recovery_started_event(
    state: &mut ServiceState,
    browser_id: &str,
    current: &BrowserProcess,
    reason_kind: BrowserRecoveryReasonKind,
    reason: &str,
    policy: Option<BrowserRecoveryPolicy>,
) {
    record_browser_recovery_started_event_with_details(
        state,
        browser_id,
        current,
        reason_kind,
        reason,
        policy,
        None,
    );
}

pub fn record_browser_recovery_started_event_with_details(
    state: &mut ServiceState,
    browser_id: &str,
    current: &BrowserProcess,
    reason_kind: BrowserRecoveryReasonKind,
    reason: &str,
    policy: Option<BrowserRecoveryPolicy>,
    extra_details: Option<serde_json::Value>,
) {
    let mut details = serde_json::json!({
        "reasonKind": reason_kind.as_str(),
        "reason": reason,
        "pid": current.pid,
        "cdpEndpoint": current.cdp_endpoint,
    });
    if let Some(cause) = process_exit_cause_for_recovery_reason(reason_kind) {
        details["processExitCause"] = serde_json::json!(cause.as_str());
    }
    if let Some(failure_class) = failure_class_for_recovery_reason(reason_kind) {
        details["failureClass"] = serde_json::json!(failure_class);
    }
    if let Some(policy) = policy {
        details["attempt"] = serde_json::json!(policy.attempt);
        details["retryBudget"] = serde_json::json!(policy.retry_budget);
        details["retryBudgetExceeded"] = serde_json::json!(policy.retry_budget_exceeded);
        details["nextRetryDelayMs"] = serde_json::json!(policy.next_retry_delay_ms);
        details["policySource"] = serde_json::json!({
            "retryBudget": policy.source.retry_budget.as_str(),
            "baseBackoffMs": policy.source.base_backoff_ms.as_str(),
            "maxBackoffMs": policy.source.max_backoff_ms.as_str(),
        });
    }
    if let Some(serde_json::Value::Object(extra)) = extra_details {
        if let Some(details) = details.as_object_mut() {
            for (key, value) in extra {
                details.insert(key, value);
            }
        }
    }

    let mut event = ServiceEvent {
        kind: ServiceEventKind::BrowserRecoveryStarted,
        message: format!("Browser {} recovery started", browser_id),
        browser_id: Some(browser_id.to_string()),
        current_health: Some(current.health),
        details: Some(details),
        ..new_service_event()
    };
    enrich_service_event_with_browser_context(&mut event, state, browser_id, current);
    push_service_event(state, event);
}

fn enrich_service_event_with_browser_context(
    event: &mut ServiceEvent,
    state: &ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
) {
    let session_id = browser
        .active_session_ids
        .first()
        .cloned()
        .or_else(|| session_id_for_browser(state, browser_id));
    let session = session_id
        .as_ref()
        .and_then(|session_id| state.sessions.get(session_id));

    event.profile_id = browser
        .profile_id
        .clone()
        .or_else(|| session.and_then(|session| session.profile_id.clone()));
    event.session_id = session_id;
    event.service_name = session.and_then(|session| session.service_name.clone());
    event.agent_name = session.and_then(|session| session.agent_name.clone());
    event.task_name = session.and_then(|session| session.task_name.clone());
}

fn session_id_for_browser(state: &ServiceState, browser_id: &str) -> Option<String> {
    state.sessions.iter().find_map(|(session_id, session)| {
        session
            .browser_ids
            .iter()
            .any(|id| id == browser_id)
            .then(|| session_id.clone())
    })
}

fn record_health_transition_events(state: &mut ServiceState, before: &ServiceState) {
    let transitions: Vec<(String, BrowserProcess, BrowserProcess)> = state
        .browsers
        .iter()
        .filter_map(|(id, browser)| {
            before
                .browsers
                .get(id)
                .map(|previous| (id.clone(), previous.clone(), browser.clone()))
        })
        .collect();

    for (id, previous, current) in transitions {
        record_browser_health_changed_event(state, &id, Some(&previous), &current);
    }
}

fn record_tab_lifecycle_events(state: &mut ServiceState, before: &ServiceState) {
    let mut events = Vec::new();
    for (id, tab) in &state.tabs {
        let previous = before.tabs.get(id);
        if previous.is_some_and(|previous| previous == tab) {
            continue;
        }

        let message = match previous {
            None => format!("Tab {} opened", id),
            Some(previous) if previous.lifecycle != tab.lifecycle => format!(
                "Tab {} lifecycle changed from {:?} to {:?}",
                id, previous.lifecycle, tab.lifecycle
            ),
            Some(_) => format!("Tab {} metadata changed", id),
        };

        events.push(ServiceEvent {
            kind: ServiceEventKind::TabLifecycleChanged,
            message,
            browser_id: Some(tab.browser_id.clone()),
            details: Some(serde_json::json!({
                "tabId": id,
                "targetId": tab.target_id.clone(),
                "previousLifecycle": previous.map(|tab| tab.lifecycle),
                "currentLifecycle": tab.lifecycle,
                "previousUrl": previous.and_then(|tab| tab.url.clone()),
                "currentUrl": tab.url.clone(),
                "previousTitle": previous.and_then(|tab| tab.title.clone()),
                "currentTitle": tab.title.clone(),
                "ownerSessionId": tab.owner_session_id.clone(),
                "serviceTabHandle": tab.service_tab_handle.clone(),
            })),
            ..new_service_event()
        });
    }
    for event in events {
        push_service_event(state, event);
    }
}

fn changed_tab_count(state: &ServiceState, before: &ServiceState) -> usize {
    state
        .tabs
        .iter()
        .filter(|(id, tab)| before.tabs.get(*id) != Some(*tab))
        .count()
}

fn changed_browser_count(state: &ServiceState, before: &ServiceState) -> usize {
    let changed_or_new = state
        .browsers
        .iter()
        .filter(|(id, browser)| {
            before
                .browsers
                .get(*id)
                .map(|previous| {
                    previous.health != browser.health || previous.last_error != browser.last_error
                })
                .unwrap_or(true)
        })
        .count();
    let removed = before
        .browsers
        .keys()
        .filter(|id| !state.browsers.contains_key(*id))
        .count();
    changed_or_new + removed
}

fn remove_post_termination_browser_history(state: &mut ServiceState) -> usize {
    let browser_ids = state
        .browsers
        .iter()
        .filter(|(id, browser)| historical_browser_placeholder(state, id, browser))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    browser_ids
        .iter()
        .map(|id| remove_browser_operational_record(state, id, None))
        .sum()
}

fn historical_browser_placeholder(
    state: &ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
) -> bool {
    if !matches!(
        browser.health,
        BrowserHealth::NotStarted
            | BrowserHealth::ProcessExited
            | BrowserHealth::Faulted
            | BrowserHealth::Unreachable
    ) {
        return false;
    }
    if browser.pid.is_some() {
        return browser.health == BrowserHealth::ProcessExited;
    }
    if browser.cdp_endpoint.is_some() {
        return matches!(
            browser.health,
            BrowserHealth::ProcessExited | BrowserHealth::Unreachable
        );
    }
    !state
        .tabs
        .values()
        .any(|tab| tab.browser_id == browser_id && tab.lifecycle != TabLifecycle::Closed)
}

fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn merge_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn service_browser_id_for_session(session_id: &str) -> String {
    format!("session:{}", session_id)
}

fn should_track_service_target(target: &CdpHttpTargetInfo) -> bool {
    (target.target_type == "page" || target.target_type == "webview")
        && !target.url.starts_with("chrome://")
        && !target.url.starts_with("chrome-extension://")
        && !target.url.starts_with("devtools://")
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct CdpHttpTargetInfo {
    id: String,
    #[serde(rename = "type")]
    target_type: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
}

#[cfg(unix)]
fn pid_is_running(pid: u32) -> bool {
    let rc = unsafe { libc::kill(pid as i32, 0) };
    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn pid_is_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        let mut exit_code = 0;
        let ok = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        ok != 0 && exit_code == STILL_ACTIVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        ControlInputProvider, DisplayAllocation, JobState, RemoteViewRoute, RoutePoolEntry,
        ServiceJob, ServiceProvider, SitePolicy, ViewStream, ViewStreamProvider, ViewerLease,
    };
    use crate::native::service_store::{
        mutate_default_service_state, JsonServiceStateStore, ServiceStateStore,
    };
    use crate::test_utils::EnvGuard;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    fn service_state_with_browser(browser: BrowserProcess) -> ServiceState {
        ServiceState {
            browsers: BTreeMap::from([(browser.id.clone(), browser)]),
            ..ServiceState::default()
        }
    }

    fn temp_home(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "agent-browser-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn merge_reconciled_service_state_preserves_newer_mutations() {
        let before = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    profile_id: Some("work-before".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-1".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    service_name: Some("JournalDownloader".to_string()),
                    browser_ids: vec!["browser-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            events: vec![ServiceEvent {
                id: "event-before".to_string(),
                kind: ServiceEventKind::Reconciliation,
                message: "before".to_string(),
                ..ServiceEvent::default()
            }],
            ..ServiceState::default()
        };
        let mut reconciled = before.clone();
        reconciled.control_plane = Some(crate::native::service_model::ControlPlaneSnapshot {
            worker_state: "ready".to_string(),
            queue_capacity: 256,
            ..crate::native::service_model::ControlPlaneSnapshot::default()
        });
        reconciled.reconciliation = Some(ServiceReconciliationSnapshot {
            last_reconciled_at: Some("2026-04-22T00:00:01Z".to_string()),
            browser_count: 1,
            changed_browsers: 1,
            ..ServiceReconciliationSnapshot::default()
        });
        reconciled.browsers.insert(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("work-before".to_string()),
                health: BrowserHealth::Unreachable,
                last_error: Some("CDP endpoint is unreachable".to_string()),
                active_session_ids: vec!["session-1".to_string()],
                ..BrowserProcess::default()
            },
        );
        reconciled.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                lifecycle: TabLifecycle::Ready,
                title: Some("Example".to_string()),
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        reconciled.sessions.get_mut("session-1").unwrap().tab_ids =
            vec!["target:page-1".to_string()];
        reconciled.events.push(ServiceEvent {
            id: "event-reconcile".to_string(),
            kind: ServiceEventKind::BrowserHealthChanged,
            message: "health changed".to_string(),
            browser_id: Some("browser-1".to_string()),
            ..ServiceEvent::default()
        });

        let mut target = before.clone();
        target.browsers.get_mut("browser-1").unwrap().profile_id = Some("work-current".to_string());
        target.jobs.insert(
            "job-current".to_string(),
            ServiceJob {
                id: "job-current".to_string(),
                action: "navigate".to_string(),
                state: JobState::Queued,
                ..ServiceJob::default()
            },
        );
        target.site_policies.insert(
            "google".to_string(),
            SitePolicy {
                id: "google".to_string(),
                origin_pattern: "https://accounts.google.com".to_string(),
                ..SitePolicy::default()
            },
        );
        target.providers.insert(
            "manual".to_string(),
            ServiceProvider {
                id: "manual".to_string(),
                display_name: "Manual approval".to_string(),
                ..ServiceProvider::default()
            },
        );
        target.events.push(ServiceEvent {
            id: "event-current".to_string(),
            kind: ServiceEventKind::IncidentAcknowledged,
            message: "current".to_string(),
            ..ServiceEvent::default()
        });

        merge_reconciled_service_state(&mut target, &before, &reconciled);

        let browser = &target.browsers["browser-1"];
        assert_eq!(browser.profile_id.as_deref(), Some("work-current"));
        assert_eq!(browser.health, BrowserHealth::Unreachable);
        assert_eq!(
            browser.last_error.as_deref(),
            Some("CDP endpoint is unreachable")
        );
        assert!(target.jobs.contains_key("job-current"));
        assert!(target.site_policies.contains_key("google"));
        assert!(target.providers.contains_key("manual"));
        assert!(target
            .events
            .iter()
            .any(|event| event.id == "event-current"));
        assert!(target
            .events
            .iter()
            .any(|event| event.id == "event-reconcile"));
        assert_eq!(
            target.sessions["session-1"].service_name.as_deref(),
            Some("JournalDownloader")
        );
        assert_eq!(
            target.sessions["session-1"].tab_ids,
            vec!["target:page-1".to_string()]
        );
        assert_eq!(
            target.tabs["target:page-1"].title.as_deref(),
            Some("Example")
        );
        assert_eq!(
            target
                .control_plane
                .as_ref()
                .map(|snapshot| snapshot.queue_capacity),
            Some(256)
        );
        assert_eq!(
            target
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.changed_browsers),
            Some(1)
        );
    }

    #[test]
    fn merge_reconciled_service_state_removes_unchanged_terminated_records() {
        let before = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-1".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    browser_ids: vec!["browser-1".to_string()],
                    tab_ids: vec!["target:old".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            tabs: BTreeMap::from([(
                "target:old".to_string(),
                BrowserTab {
                    id: "target:old".to_string(),
                    browser_id: "browser-1".to_string(),
                    session_id: Some("session-1".to_string()),
                    lifecycle: TabLifecycle::Ready,
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        };
        let mut target = before.clone();
        let mut reconciled = before.clone();
        remove_browser_operational_record(&mut reconciled, "browser-1", Some("session-1"));

        merge_reconciled_service_state(&mut target, &before, &reconciled);

        assert!(!target.browsers.contains_key("browser-1"));
        assert!(!target.sessions.contains_key("session-1"));
        assert!(!target.tabs.contains_key("target:old"));
    }

    #[tokio::test]
    async fn reconcile_service_state_in_repository_persists_summary() {
        let home = temp_home("service-health-reconcile-repository");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
        store
            .save(&ServiceState {
                browsers: BTreeMap::from([(
                    "browser-1".to_string(),
                    BrowserProcess {
                        id: "browser-1".to_string(),
                        health: BrowserHealth::Ready,
                        active_session_ids: vec!["session-1".to_string()],
                        ..BrowserProcess::default()
                    },
                )]),
                sessions: BTreeMap::from([(
                    "session-1".to_string(),
                    BrowserSession {
                        id: "session-1".to_string(),
                        browser_ids: vec!["browser-1".to_string()],
                        ..BrowserSession::default()
                    },
                )]),
                site_policies: BTreeMap::from([(
                    "google".to_string(),
                    SitePolicy {
                        id: "google".to_string(),
                        origin_pattern: "https://accounts.google.com".to_string(),
                        ..SitePolicy::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let summary = reconcile_service_state_in_repository(&repository)
            .await
            .unwrap();

        let persisted = store.load().unwrap();
        assert_eq!(summary.browser_count, 1);
        assert_eq!(
            persisted
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.browser_count),
            Some(1)
        );
        assert!(persisted.site_policies.contains_key("google"));
        assert!(persisted
            .events
            .iter()
            .any(|event| event.kind == ServiceEventKind::Reconciliation));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn reconcile_orphans_remote_view_state_for_unavailable_browser() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::ProcessExited,
                    display_allocation_id: Some("display-1".to_string()),
                    view_streams: vec![ViewStream {
                        id: "remote-headed-view".to_string(),
                        route_id: Some("route-1".to_string()),
                        display_allocation_id: Some("display-1".to_string()),
                        viewer_lease_ids: vec!["lease-1".to_string()],
                        controller_lease_id: Some("lease-1".to_string()),
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    state: "ready".to_string(),
                    route_ids: vec!["route-1".to_string()],
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    display_allocation_id: Some("display-1".to_string()),
                    browser_id: Some("browser-1".to_string()),
                    state: "ready".to_string(),
                    viewer_lease_ids: vec!["lease-1".to_string()],
                    controller_lease_id: Some("lease-1".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            viewer_leases: BTreeMap::from([(
                "lease-1".to_string(),
                ViewerLease {
                    id: "lease-1".to_string(),
                    route_id: Some("route-1".to_string()),
                    browser_id: Some("browser-1".to_string()),
                    state: "observing".to_string(),
                    ..ViewerLease::default()
                },
            )]),
            ..ServiceState::default()
        };

        let summary = reconcile_service_state(&mut state).await;

        assert_eq!(summary.browser_count, 0);
        assert_eq!(state.display_allocations["display-1"].state, "orphaned");
        assert_eq!(state.remote_view_routes["route-1"].state, "orphaned");
        assert_eq!(state.viewer_leases["lease-1"].state, "disconnected");
        assert!(state.remote_view_routes["route-1"]
            .viewer_lease_ids
            .is_empty());
        assert!(state.remote_view_routes["route-1"]
            .controller_lease_id
            .is_none());
        assert!(!state.browsers.contains_key("browser-1"));
        let remote_view = &state.events.last().unwrap().details.as_ref().unwrap()["remoteView"];
        assert_eq!(remote_view["orphanedDisplayAllocations"], 1);
        assert_eq!(remote_view["orphanedRoutes"], 1);
        assert_eq!(remote_view["releasedViewerLeases"], 1);
        assert_eq!(remote_view["clearedControllerLeases"], 1);
    }

    #[tokio::test]
    async fn reconcile_expires_remote_viewer_leases_without_releasing_healthy_route() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    display_allocation_id: Some("display-1".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    display_allocation_id: Some("display-1".to_string()),
                    browser_id: Some("browser-1".to_string()),
                    state: "ready".to_string(),
                    viewer_lease_ids: vec!["expired-lease".to_string(), "active-lease".to_string()],
                    controller_lease_id: Some("expired-lease".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            viewer_leases: BTreeMap::from([
                (
                    "expired-lease".to_string(),
                    ViewerLease {
                        id: "expired-lease".to_string(),
                        route_id: Some("route-1".to_string()),
                        browser_id: Some("browser-1".to_string()),
                        state: "controlling".to_string(),
                        expires_at: Some("2000-01-01T00:00:00Z".to_string()),
                        ..ViewerLease::default()
                    },
                ),
                (
                    "active-lease".to_string(),
                    ViewerLease {
                        id: "active-lease".to_string(),
                        route_id: Some("route-1".to_string()),
                        browser_id: Some("browser-1".to_string()),
                        state: "observing".to_string(),
                        ..ViewerLease::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        reconcile_service_state(&mut state).await;

        assert_eq!(state.display_allocations["display-1"].state, "ready");
        assert_eq!(state.remote_view_routes["route-1"].state, "ready");
        assert_eq!(state.viewer_leases["expired-lease"].state, "expired");
        assert_eq!(state.viewer_leases["active-lease"].state, "observing");
        assert_eq!(
            state.remote_view_routes["route-1"].viewer_lease_ids,
            vec!["active-lease".to_string()]
        );
        assert!(state.remote_view_routes["route-1"]
            .controller_lease_id
            .is_none());
        let remote_view = &state.events.last().unwrap().details.as_ref().unwrap()["remoteView"];
        assert_eq!(remote_view["expiredViewerLeases"], 1);
        assert_eq!(remote_view["clearedControllerLeases"], 1);
    }

    #[tokio::test]
    async fn persisted_reconcile_preserves_overlapping_state_mutations() {
        let home = temp_home("service-reconcile-overlap");
        fs::create_dir_all(&home).unwrap();
        let guard = EnvGuard::new(&["HOME"]);
        guard.set("HOME", home.to_str().unwrap());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (version_requested_tx, version_requested_rx) = oneshot::channel();
        let (version_release_tx, version_release_rx) = oneshot::channel();
        let server = tokio::spawn(serve_cdp_version_and_list_with_delayed_version(
            listener,
            r#"[
                {"id":"page-1","type":"page","title":"Example","url":"https://example.com"}
            ]"#,
            version_requested_tx,
            version_release_rx,
        ));

        let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path().unwrap());
        store
            .save(&ServiceState {
                browsers: BTreeMap::from([(
                    "browser-1".to_string(),
                    BrowserProcess {
                        id: "browser-1".to_string(),
                        health: BrowserHealth::Ready,
                        cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
                        active_session_ids: vec!["session-1".to_string()],
                        ..BrowserProcess::default()
                    },
                )]),
                sessions: BTreeMap::from([(
                    "session-1".to_string(),
                    BrowserSession {
                        id: "session-1".to_string(),
                        browser_ids: vec!["browser-1".to_string()],
                        ..BrowserSession::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        let reconcile = tokio::spawn(async { reconcile_persisted_service_state().await });
        version_requested_rx.await.unwrap();

        mutate_default_service_state(|state| {
            state.jobs.insert(
                "job-overlap".to_string(),
                ServiceJob {
                    id: "job-overlap".to_string(),
                    action: "navigate".to_string(),
                    state: JobState::Queued,
                    ..ServiceJob::default()
                },
            );
            state.site_policies.insert(
                "google".to_string(),
                SitePolicy {
                    id: "google".to_string(),
                    origin_pattern: "https://accounts.google.com".to_string(),
                    ..SitePolicy::default()
                },
            );
            state.providers.insert(
                "manual".to_string(),
                ServiceProvider {
                    id: "manual".to_string(),
                    display_name: "Manual approval".to_string(),
                    ..ServiceProvider::default()
                },
            );
            state.events.push(ServiceEvent {
                id: "event-overlap".to_string(),
                kind: ServiceEventKind::IncidentAcknowledged,
                message: "operator acknowledged overlapping incident".to_string(),
                ..ServiceEvent::default()
            });
            Ok(())
        })
        .unwrap();

        version_release_tx.send(()).unwrap();
        let summary = reconcile.await.unwrap().unwrap();
        server.await.unwrap();

        assert_eq!(summary.browser_count, 1);
        let persisted = store.load().unwrap();
        assert!(persisted.jobs.contains_key("job-overlap"));
        assert!(persisted.site_policies.contains_key("google"));
        assert!(persisted.providers.contains_key("manual"));
        assert!(persisted
            .events
            .iter()
            .any(|event| event.id == "event-overlap"));
        assert!(persisted
            .events
            .iter()
            .any(|event| event.kind == ServiceEventKind::Reconciliation));
        assert!(persisted.tabs.contains_key("target:page-1"));
        assert_eq!(
            persisted.sessions["session-1"].tab_ids,
            vec!["target:page-1".to_string()]
        );

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn remote_headed_browser_record_upserts_private_display_allocation() {
        let home = temp_home("service-health-display-allocation");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());

        persist_service_browser_record_in_repository(
            &repository,
            "persist-session",
            BrowserHost::RemoteHeaded,
            BrowserHealth::Ready,
            Some(1234),
            Some("http://127.0.0.1:9222".to_string()),
            None,
            Some(ServiceLaunchMetadata {
                profile_id: Some("work".to_string()),
                browser_capability_launch: Some(serde_json::json!({
                    "browserBuild": "stealthcdp_chromium"
                })),
                view_streams: vec![ViewStream {
                    id: "remote-headed-view".to_string(),
                    provider: ViewStreamProvider::RdpGateway,
                    control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                    url: Some("/guacamole/#/client/browser-1".to_string()),
                    display_allocation_id: None,
                    ..ViewStream::default()
                }],
                display_isolation: Some("private_virtual_display".to_string()),
                display_name: Some(":91".to_string()),
                ..ServiceLaunchMetadata::default()
            }),
        )
        .unwrap();

        let state = store.load().unwrap();
        let browser = &state.browsers["session:persist-session"];
        let allocation_id = browser.display_allocation_id.as_ref().unwrap();
        assert_eq!(
            allocation_id,
            "display:private_virtual_display:session-persist-session"
        );
        assert_eq!(
            browser.view_streams[0].display_allocation_id.as_deref(),
            Some(allocation_id.as_str())
        );

        let allocation = &state.display_allocations[allocation_id];
        assert_eq!(allocation.display_name.as_deref(), Some(":91"));
        assert_eq!(allocation.display_isolation, "private_virtual_display");
        assert_eq!(
            allocation.owner_browser_id.as_deref(),
            Some("session:persist-session")
        );
        assert_eq!(
            allocation.owner_session_id.as_deref(),
            Some("persist-session")
        );
        assert_eq!(allocation.profile_id.as_deref(), Some("work"));
        assert_eq!(
            allocation.browser_build.as_deref(),
            Some("stealthcdp_chromium")
        );
        assert_eq!(allocation.host, Some(BrowserHost::RemoteHeaded));
        assert_eq!(allocation.state, "ready");
        assert_eq!(
            allocation.pid_hints.as_ref().unwrap()["browserPid"],
            serde_json::json!(1234)
        );

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn private_display_allocation_ids_are_scoped_per_session() {
        let home = temp_home("service-health-display-allocation-distinct");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());

        for session_id in ["browser-a", "browser-b"] {
            persist_service_browser_record_in_repository(
                &repository,
                session_id,
                BrowserHost::RemoteHeaded,
                BrowserHealth::Ready,
                Some(1234),
                Some("http://127.0.0.1:9222".to_string()),
                None,
                Some(ServiceLaunchMetadata {
                    profile_id: Some(format!("work-{session_id}")),
                    view_streams: vec![ViewStream {
                        id: format!("view-{session_id}"),
                        provider: ViewStreamProvider::RdpGateway,
                        display_allocation_id: None,
                        ..ViewStream::default()
                    }],
                    display_isolation: Some("private_virtual_display".to_string()),
                    display_name: Some(":91".to_string()),
                    ..ServiceLaunchMetadata::default()
                }),
            )
            .unwrap();
        }

        let state = store.load().unwrap();
        let browser_a = &state.browsers["session:browser-a"];
        let browser_b = &state.browsers["session:browser-b"];
        assert_ne!(
            browser_a.display_allocation_id,
            browser_b.display_allocation_id
        );
        assert_eq!(
            browser_a.display_allocation_id.as_deref(),
            Some("display:private_virtual_display:session-browser-a")
        );
        assert_eq!(
            browser_b.display_allocation_id.as_deref(),
            Some("display:private_virtual_display:session-browser-b")
        );
        assert_eq!(state.display_allocations.len(), 2);

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn close_releases_only_owned_display_allocation() {
        let home = temp_home("service-health-close-display-release");
        let store = JsonServiceStateStore::new(home.join("state.json"));
        let repository = LockedServiceStateRepository::new(store.clone());
        let browser_a = "session:browser-a".to_string();
        let browser_b = "session:browser-b".to_string();
        let allocation_a = "display:private_virtual_display:session-browser-a".to_string();
        let allocation_b = "display:private_virtual_display:session-browser-b".to_string();
        store
            .save(&ServiceState {
                browsers: BTreeMap::from([
                    (
                        browser_a.clone(),
                        BrowserProcess {
                            id: browser_a.clone(),
                            host: BrowserHost::RemoteHeaded,
                            health: BrowserHealth::Ready,
                            display_isolation: Some("private_virtual_display".to_string()),
                            display_name: Some(":91".to_string()),
                            display_allocation_id: Some(allocation_a.clone()),
                            active_session_ids: vec!["browser-a".to_string()],
                            ..BrowserProcess::default()
                        },
                    ),
                    (
                        browser_b.clone(),
                        BrowserProcess {
                            id: browser_b.clone(),
                            host: BrowserHost::RemoteHeaded,
                            health: BrowserHealth::Ready,
                            display_isolation: Some("private_virtual_display".to_string()),
                            display_name: Some(":92".to_string()),
                            display_allocation_id: Some(allocation_b.clone()),
                            active_session_ids: vec!["browser-b".to_string()],
                            ..BrowserProcess::default()
                        },
                    ),
                ]),
                display_allocations: BTreeMap::from([
                    (
                        allocation_a.clone(),
                        DisplayAllocation {
                            id: allocation_a.clone(),
                            display_name: Some(":91".to_string()),
                            display_isolation: "private_virtual_display".to_string(),
                            owner_browser_id: Some(browser_a.clone()),
                            owner_session_id: Some("browser-a".to_string()),
                            state: "ready".to_string(),
                            route_ids: vec!["guacamole:4".to_string()],
                            ..DisplayAllocation::default()
                        },
                    ),
                    (
                        allocation_b.clone(),
                        DisplayAllocation {
                            id: allocation_b.clone(),
                            display_name: Some(":92".to_string()),
                            display_isolation: "private_virtual_display".to_string(),
                            owner_browser_id: Some(browser_b.clone()),
                            owner_session_id: Some("browser-b".to_string()),
                            state: "ready".to_string(),
                            ..DisplayAllocation::default()
                        },
                    ),
                ]),
                remote_view_routes: BTreeMap::from([(
                    "guacamole:4".to_string(),
                    RemoteViewRoute {
                        id: "guacamole:4".to_string(),
                        provider: ViewStreamProvider::RdpGateway,
                        display_allocation_id: Some(allocation_a.clone()),
                        browser_id: Some(browser_a.clone()),
                        session_id: Some("browser-a".to_string()),
                        state: "ready".to_string(),
                        viewer_lease_ids: vec!["viewer:guacamole:4:a".to_string()],
                        controller_lease_id: Some("viewer:guacamole:4:a".to_string()),
                        ..RemoteViewRoute::default()
                    },
                )]),
                route_pool: BTreeMap::from([(
                    "pool-a".to_string(),
                    RoutePoolEntry {
                        id: "pool-a".to_string(),
                        provider: ViewStreamProvider::RdpGateway,
                        route_id: "guacamole:4".to_string(),
                        state: "checked_out".to_string(),
                        current_route_allocation_id: Some("guacamole:4".to_string()),
                        ..RoutePoolEntry::default()
                    },
                )]),
                ..ServiceState::default()
            })
            .unwrap();

        persist_closed_browser_health_in_repository(
            &repository,
            "browser-a",
            Some(&BrowserShutdownOutcome {
                polite_close_attempted: true,
                polite_close_succeeded: true,
                ..BrowserShutdownOutcome::default()
            }),
        )
        .unwrap();

        let state = store.load().unwrap();
        assert!(!state.browsers.contains_key(&browser_a));
        let released = &state.display_allocations[&allocation_a];
        assert_eq!(released.state, "released");
        assert_eq!(
            released.readiness.as_ref().unwrap()["reason"],
            "operator_requested_close"
        );
        assert!(released.route_ids.is_empty());
        assert_eq!(state.remote_view_routes["guacamole:4"].state, "released");
        assert_eq!(state.route_pool["pool-a"].state, "available");
        assert_eq!(state.route_pool["pool-a"].current_route_allocation_id, None);
        assert_eq!(state.display_allocations[&allocation_b].state, "ready");
        assert_eq!(state.browsers[&browser_b].health, BrowserHealth::Ready);

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn launch_recorded_event_copies_browser_session_context() {
        let mut state = ServiceState {
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    profile_id: Some("work".to_string()),
                    service_name: Some("JournalDownloader".to_string()),
                    agent_name: Some("codex".to_string()),
                    task_name: Some("probeACSwebsite".to_string()),
                    browser_ids: vec!["browser-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("work".to_string()),
            active_session_ids: vec!["session-1".to_string()],
            health: BrowserHealth::Ready,
            ..BrowserProcess::default()
        };

        let metadata = ServiceLaunchMetadata {
            browser_stderr_log_path: Some(
                "/home/user/.agent-browser/tmp/chrome-launches/chrome-123.stderr.log".to_string(),
            ),
            browser_capability_launch: Some(serde_json::json!({
                "applied": false,
                "reason": "profile_compatibility_missing_or_blocked",
                "browserBuild": "stealthcdp_chromium"
            })),
            ..ServiceLaunchMetadata::default()
        };
        record_browser_launch_recorded_event(
            &mut state,
            "browser-1",
            None,
            &browser,
            Some(&metadata),
        );

        let event = state.events.first().unwrap();
        assert_eq!(event.kind, ServiceEventKind::BrowserLaunchRecorded);
        assert_eq!(event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(event.profile_id.as_deref(), Some("work"));
        assert_eq!(event.session_id.as_deref(), Some("session-1"));
        assert_eq!(event.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(event.agent_name.as_deref(), Some("codex"));
        assert_eq!(event.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(
            event.details.as_ref().unwrap()["browserStderrLogPath"],
            "/home/user/.agent-browser/tmp/chrome-launches/chrome-123.stderr.log"
        );
        assert_eq!(
            event.details.as_ref().unwrap()["browserCapabilityLaunch"]["reason"],
            "profile_compatibility_missing_or_blocked"
        );
    }

    #[test]
    fn recovery_started_event_copies_browser_session_context() {
        let mut state = ServiceState {
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    profile_id: Some("work".to_string()),
                    service_name: Some("JournalDownloader".to_string()),
                    agent_name: Some("codex".to_string()),
                    task_name: Some("probeACSwebsite".to_string()),
                    browser_ids: vec!["browser-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("work".to_string()),
            active_session_ids: vec!["session-1".to_string()],
            health: BrowserHealth::CdpDisconnected,
            pid: Some(1234),
            cdp_endpoint: Some("ws://127.0.0.1:9222/devtools/browser/old".to_string()),
            ..BrowserProcess::default()
        };

        record_browser_recovery_started_event(
            &mut state,
            "browser-1",
            &browser,
            BrowserRecoveryReasonKind::CdpDisconnected,
            "cdp lost",
            Some(BrowserRecoveryPolicy {
                attempt: 2,
                retry_budget: 3,
                retry_budget_exceeded: false,
                next_retry_delay_ms: 2_000,
                source: BrowserRecoveryPolicySource {
                    retry_budget: BrowserRecoveryPolicyValueSource::Config,
                    base_backoff_ms: BrowserRecoveryPolicyValueSource::Env,
                    max_backoff_ms: BrowserRecoveryPolicyValueSource::Cli,
                },
            }),
        );

        let event = state.events.first().unwrap();
        assert_eq!(event.kind, ServiceEventKind::BrowserRecoveryStarted);
        assert_eq!(event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(event.profile_id.as_deref(), Some("work"));
        assert_eq!(event.session_id.as_deref(), Some("session-1"));
        assert_eq!(event.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(event.agent_name.as_deref(), Some("codex"));
        assert_eq!(event.task_name.as_deref(), Some("probeACSwebsite"));
        assert_eq!(event.current_health, Some(BrowserHealth::CdpDisconnected));
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("reasonKind"))
                .and_then(|reason| reason.as_str()),
            Some("cdp_disconnected")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("reason"))
                .and_then(|reason| reason.as_str()),
            Some("cdp lost")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("attempt"))
                .and_then(|attempt| attempt.as_u64()),
            Some(2)
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("retryBudget"))
                .and_then(|budget| budget.as_u64()),
            Some(3)
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("retryBudgetExceeded"))
                .and_then(|exceeded| exceeded.as_bool()),
            Some(false)
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("nextRetryDelayMs"))
                .and_then(|delay| delay.as_u64()),
            Some(2_000)
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("policySource"))
                .and_then(|source| source.get("retryBudget"))
                .and_then(|source| source.as_str()),
            Some("config")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("policySource"))
                .and_then(|source| source.get("baseBackoffMs"))
                .and_then(|source| source.as_str()),
            Some("env")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("policySource"))
                .and_then(|source| source.get("maxBackoffMs"))
                .and_then(|source| source.as_str()),
            Some("cli")
        );
    }

    #[test]
    fn recovery_started_event_marks_unexpected_process_exit_cause() {
        let mut state = ServiceState::default();
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            pid: Some(1234),
            ..BrowserProcess::default()
        };

        record_browser_recovery_started_event(
            &mut state,
            "browser-1",
            &browser,
            BrowserRecoveryReasonKind::ProcessExited,
            "browser process exited",
            None,
        );

        let event = state.events.first().unwrap();
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("processExitCause"))
                .and_then(|cause| cause.as_str()),
            Some("unexpected_process_exit")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("failureClass"))
                .and_then(|class| class.as_str()),
            Some("browser_process_exited")
        );
    }

    #[test]
    fn recovery_reason_kind_tracks_browser_health() {
        assert_eq!(
            recovery_reason_kind_for_health(BrowserHealth::ProcessExited),
            BrowserRecoveryReasonKind::ProcessExited
        );
        assert_eq!(
            recovery_reason_kind_for_health(BrowserHealth::CdpDisconnected),
            BrowserRecoveryReasonKind::CdpDisconnected
        );
        assert_eq!(
            recovery_reason_kind_for_health(BrowserHealth::Unreachable),
            BrowserRecoveryReasonKind::UnreachableEndpoint
        );
        assert_eq!(
            recovery_reason_kind_for_health(BrowserHealth::Degraded),
            BrowserRecoveryReasonKind::DegradedTargets
        );
        assert_eq!(
            recovery_reason_kind_for_health(BrowserHealth::Closing),
            BrowserRecoveryReasonKind::OperatorRequestedClose
        );
        assert_eq!(
            recovery_reason_kind_for_health(BrowserHealth::Ready),
            BrowserRecoveryReasonKind::PersistedUnhealthyState
        );
    }

    #[test]
    fn process_exit_cause_tracks_client_facing_exit_vocabulary() {
        assert_eq!(
            process_exit_cause_for_health(BrowserHealth::ProcessExited),
            Some(BrowserProcessExitCause::UnexpectedProcessExit)
        );
        assert_eq!(
            process_exit_cause_for_health(BrowserHealth::Closing),
            Some(BrowserProcessExitCause::OperatorRequestedClose)
        );
        assert_eq!(process_exit_cause_for_health(BrowserHealth::Ready), None);
    }

    #[test]
    fn health_changed_event_includes_structured_reason_kinds() {
        let mut state = ServiceState::default();
        let previous = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            ..BrowserProcess::default()
        };
        let current = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::CdpDisconnected,
            last_error: Some("CDP endpoint is unreachable: ws://old".to_string()),
            ..BrowserProcess::default()
        };

        record_browser_health_changed_event(&mut state, "browser-1", Some(&previous), &current);

        let event = state.events.first().unwrap();
        assert_eq!(event.kind, ServiceEventKind::BrowserHealthChanged);
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("currentReasonKind"))
                .and_then(|reason| reason.as_str()),
            Some("cdp_disconnected")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("failureClass"))
                .and_then(|class| class.as_str()),
            Some("cdp_unresponsive")
        );
        assert!(event
            .details
            .as_ref()
            .and_then(|details| details.get("previousReasonKind"))
            .is_none());
    }

    #[test]
    fn health_changed_event_marks_unexpected_process_exit_cause() {
        let mut state = ServiceState::default();
        let previous = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            ..BrowserProcess::default()
        };
        let current = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            last_error: Some("Recorded browser PID 1234 is no longer running".to_string()),
            ..BrowserProcess::default()
        };

        record_browser_health_changed_event(&mut state, "browser-1", Some(&previous), &current);

        let event = state.events.first().unwrap();
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("processExitCause"))
                .and_then(|cause| cause.as_str()),
            Some("unexpected_process_exit")
        );
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("failureClass"))
                .and_then(|class| class.as_str()),
            Some("browser_process_exited")
        );
    }

    #[test]
    fn health_observation_retains_failure_evidence_on_browser_record() {
        let mut browser = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            last_error: Some("Recorded browser PID 1234 is no longer running".to_string()),
            ..BrowserProcess::default()
        };
        let details = browser_health_observation_details(
            &browser,
            Some(serde_json::json!({
                "processExitDetection": "persisted_pid_probe",
                "processExitPid": 1234,
            })),
        );

        apply_browser_health_observation(&mut browser, Some(&details));

        let observation = browser.last_health_observation.as_ref().unwrap();
        assert_eq!(observation.health, BrowserHealth::ProcessExited);
        assert_eq!(observation.reason_kind.as_deref(), Some("process_exited"));
        assert_eq!(
            observation.failure_class.as_deref(),
            Some("browser_process_exited")
        );
        assert_eq!(
            observation.process_exit_cause.as_deref(),
            Some("unexpected_process_exit")
        );
        assert_eq!(
            observation
                .details
                .as_ref()
                .and_then(|details| details.get("processExitPid"))
                .and_then(|pid| pid.as_u64()),
            Some(1234)
        );
    }

    #[test]
    fn health_observation_clears_when_browser_is_ready() {
        let mut browser = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            last_health_observation: Some(BrowserHealthObservation {
                observed_at: "2026-04-27T00:00:00Z".to_string(),
                health: BrowserHealth::ProcessExited,
                failure_class: Some("browser_process_exited".to_string()),
                ..BrowserHealthObservation::default()
            }),
            ..BrowserProcess::default()
        };
        browser.health = BrowserHealth::Ready;

        apply_browser_health_observation(&mut browser, None);

        assert!(browser.last_health_observation.is_none());
    }

    #[test]
    fn health_changed_event_carries_previous_reason_kind_on_recovery() {
        let mut state = ServiceState::default();
        let previous = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            last_error: Some("Recorded browser PID 1234 is no longer running".to_string()),
            ..BrowserProcess::default()
        };
        let current = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            ..BrowserProcess::default()
        };

        record_browser_health_changed_event(&mut state, "browser-1", Some(&previous), &current);

        let event = state.events.first().unwrap();
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("previousReasonKind"))
                .and_then(|reason| reason.as_str()),
            Some("process_exited")
        );
        assert!(event
            .details
            .as_ref()
            .and_then(|details| details.get("currentReasonKind"))
            .is_none());
    }

    async fn serve_json_version(listener: TcpListener) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf).await;
        let body = r#"{"Browser":"Chrome/123"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }

    async fn serve_cdp_version_and_list(listener: TcpListener, list_body: &'static str) {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let read = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..read]);
            let body = if request.contains("/json/list") {
                list_body
            } else {
                r#"{"Browser":"Chrome/123"}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    }

    async fn serve_cdp_version_and_list_with_delayed_version(
        listener: TcpListener,
        list_body: &'static str,
        version_requested_tx: oneshot::Sender<()>,
        version_release_rx: oneshot::Receiver<()>,
    ) {
        let mut version_requested_tx = Some(version_requested_tx);
        let mut version_release_rx = Some(version_release_rx);

        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let read = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..read]);
            let body = if request.contains("/json/list") {
                list_body
            } else {
                if let Some(tx) = version_requested_tx.take() {
                    let _ = tx.send(());
                }
                if let Some(rx) = version_release_rx.take() {
                    let _ = rx.await;
                }
                r#"{"Browser":"Chrome/123"}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    }

    #[test]
    fn cdp_version_url_normalizes_websocket_endpoint() {
        assert_eq!(
            cdp_version_url("ws://127.0.0.1:9222/devtools/browser/abc?token=1").as_deref(),
            Some("http://127.0.0.1:9222/json/version")
        );
        assert_eq!(
            cdp_version_url("https://example.com/devtools/browser/abc").as_deref(),
            Some("https://example.com/json/version")
        );
        assert!(cdp_version_url("file:///tmp/not-cdp").is_none());
    }

    #[test]
    fn cdp_list_url_normalizes_websocket_endpoint() {
        assert_eq!(
            cdp_list_url("ws://127.0.0.1:9222/devtools/browser/abc?token=1").as_deref(),
            Some("http://127.0.0.1:9222/json/list")
        );
    }

    #[tokio::test]
    async fn refresh_marks_reachable_cdp_ready() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_json_version(listener));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Unreachable,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            last_error: Some("previous failure".to_string()),
            ..BrowserProcess::default()
        });

        refresh_persisted_browser_health(&mut state).await;
        server.await.unwrap();

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::Ready);
        assert_eq!(browser.last_error, None);
    }

    #[tokio::test]
    async fn reconcile_discovers_live_cdp_targets() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(
            listener,
            r#"[
                {"id":"page-1","type":"page","title":"Example","url":"https://example.com"},
                {"id":"devtools-1","type":"page","title":"DevTools","url":"devtools://devtools/bundled/inspector.html"}
            ]"#,
        ));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:stale".to_string(),
            BrowserTab {
                id: "target:stale".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("stale".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:stale".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        let tab = &state.tabs["target:page-1"];
        assert_eq!(tab.browser_id, "browser-1");
        assert_eq!(tab.target_id.as_deref(), Some("page-1"));
        assert_eq!(tab.lifecycle, TabLifecycle::Ready);
        assert_eq!(tab.url.as_deref(), Some("https://example.com"));
        assert_eq!(tab.title.as_deref(), Some("Example"));
        assert_eq!(tab.owner_session_id.as_deref(), Some("session-1"));
        assert!(!state.tabs.contains_key("target:devtools-1"));

        let session = &state.sessions["session-1"];
        assert_eq!(session.browser_ids, vec!["browser-1"]);
        assert_eq!(session.tab_ids, vec!["target:page-1"]);
        assert_eq!(state.tabs["target:stale"].lifecycle, TabLifecycle::Closed);
        assert_eq!(
            state.events.last().unwrap().details.as_ref().unwrap()["changedTabs"],
            2
        );
        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(tab_event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(
            tab_event.details.as_ref().unwrap()["tabId"],
            "target:page-1"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentLifecycle"],
            "ready"
        );
    }

    #[test]
    fn reconcile_reassigns_tab_relationships_when_owner_session_changes() {
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            active_session_ids: vec!["session-new".to_string()],
            ..BrowserProcess::default()
        };
        let mut state = service_state_with_browser(browser.clone());
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-old".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-old".to_string(),
            BrowserSession {
                id: "session-old".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:page-1".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_browser_targets(
            &mut state,
            "browser-1",
            &browser,
            vec![CdpHttpTargetInfo {
                id: "page-1".to_string(),
                target_type: "page".to_string(),
                title: "Example".to_string(),
                url: "https://example.com".to_string(),
            }],
        );

        assert!(state.sessions["session-old"].tab_ids.is_empty());
        assert_eq!(
            state.sessions["session-new"].tab_ids,
            vec!["target:page-1".to_string()]
        );
        assert_eq!(
            state.tabs["target:page-1"].owner_session_id.as_deref(),
            Some("session-new")
        );
        let repair_event = state
            .events
            .iter()
            .find(|event| {
                event.kind == ServiceEventKind::Reconciliation
                    && event.details.as_ref().is_some_and(|details| {
                        details["action"] == "session_tab_ownership_repaired"
                    })
            })
            .unwrap();
        assert_eq!(repair_event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(
            repair_event.details.as_ref().unwrap()["ownerSessionId"],
            "session-new"
        );
        assert_eq!(
            repair_event.details.as_ref().unwrap()["removedRelationshipCount"],
            1
        );
        assert_eq!(
            repair_event.details.as_ref().unwrap()["removedRelations"][0]["sessionId"],
            "session-old"
        );
        assert_eq!(
            repair_event.details.as_ref().unwrap()["removedRelations"][0]["tabId"],
            "target:page-1"
        );
    }

    #[test]
    fn reconcile_removes_session_tab_relationships_without_owner_session() {
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            active_session_ids: Vec::new(),
            ..BrowserProcess::default()
        };
        let mut state = service_state_with_browser(browser.clone());
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-old".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-old".to_string(),
            BrowserSession {
                id: "session-old".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:page-1".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_browser_targets(
            &mut state,
            "browser-1",
            &browser,
            vec![CdpHttpTargetInfo {
                id: "page-1".to_string(),
                target_type: "page".to_string(),
                title: "Example".to_string(),
                url: "https://example.com".to_string(),
            }],
        );

        assert!(state.sessions["session-old"].tab_ids.is_empty());
        assert!(!state.sessions.contains_key(""));
        assert_eq!(state.tabs["target:page-1"].owner_session_id, None);
        let repair_event = state
            .events
            .iter()
            .find(|event| {
                event.kind == ServiceEventKind::Reconciliation
                    && event.details.as_ref().is_some_and(|details| {
                        details["action"] == "session_tab_ownership_repaired"
                    })
            })
            .unwrap();
        assert_eq!(repair_event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(
            repair_event.details.as_ref().unwrap()["ownerSessionId"],
            serde_json::Value::Null
        );
        assert_eq!(
            repair_event.details.as_ref().unwrap()["removedRelationshipCount"],
            1
        );
    }

    #[tokio::test]
    async fn reconcile_marks_target_list_failure_degraded() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(listener, r#"not-json"#));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:page-1".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::Degraded);
        assert!(browser
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("Failed to parse CDP targets"));
        assert_eq!(
            state
                .events
                .iter()
                .find(|event| event.kind == ServiceEventKind::BrowserHealthChanged)
                .unwrap()
                .current_health,
            Some(BrowserHealth::Degraded)
        );
        assert_eq!(
            state
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.changed_browsers),
            Some(1)
        );
        assert_eq!(state.tabs["target:page-1"].lifecycle, TabLifecycle::Closed);
        assert!(state.sessions["session-1"].tab_ids.is_empty());
    }

    #[tokio::test]
    async fn reconcile_closes_tabs_for_non_ready_browser() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:page-1".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_service_state(&mut state).await;

        assert!(!state.browsers.contains_key("browser-1"));
        assert!(!state.tabs.contains_key("target:page-1"));
        assert!(!state.sessions.contains_key("session-1"));
        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentLifecycle"],
            "closed"
        );
    }

    #[tokio::test]
    async fn reconcile_marks_missing_live_targets_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(listener, r#"[]"#));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:stale".to_string(),
            BrowserTab {
                id: "target:stale".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("stale".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        assert_eq!(state.tabs["target:stale"].lifecycle, TabLifecycle::Closed);
        assert_eq!(
            state.events.last().unwrap().details.as_ref().unwrap()["changedTabs"],
            1
        );
        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(tab_event.details.as_ref().unwrap()["tabId"], "target:stale");
        assert_eq!(
            tab_event.details.as_ref().unwrap()["previousLifecycle"],
            "ready"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentLifecycle"],
            "closed"
        );
    }

    #[test]
    fn reconcile_merges_duplicate_ready_browsers_with_same_cdp_endpoint() {
        let mut state = ServiceState::default();
        state.browsers.insert(
            "session:odollo-carrier-ups".to_string(),
            BrowserProcess {
                id: "session:odollo-carrier-ups".to_string(),
                profile_id: Some("stealthcdp-default".to_string()),
                health: BrowserHealth::Ready,
                cdp_endpoint: Some("ws://127.0.0.1:37173/devtools/browser/shared".to_string()),
                active_session_ids: vec!["odollo-carrier-ups".to_string()],
                display_allocation_id: Some(
                    "display:private_virtual_display:session-odollo-carrier-ups".to_string(),
                ),
                ..BrowserProcess::default()
            },
        );
        state.browsers.insert(
            "session:odollo-carrier-usps".to_string(),
            BrowserProcess {
                id: "session:odollo-carrier-usps".to_string(),
                profile_id: Some("stealthcdp-default".to_string()),
                health: BrowserHealth::Ready,
                pid: Some(14983),
                display_name: Some(":115".to_string()),
                cdp_endpoint: Some("ws://127.0.0.1:37173/devtools/browser/shared".to_string()),
                active_session_ids: vec!["odollo-carrier-usps".to_string()],
                display_allocation_id: Some(
                    "display:private_virtual_display:session-odollo-carrier-usps".to_string(),
                ),
                ..BrowserProcess::default()
            },
        );
        for (session_id, browser_id, tab_ids, conflicts) in [
            (
                "odollo-carrier-ups",
                "session:odollo-carrier-ups",
                Vec::<String>::new(),
                vec!["odollo-carrier-usps".to_string()],
            ),
            (
                "odollo-carrier-usps",
                "session:odollo-carrier-usps",
                vec!["target:usps".to_string()],
                vec!["odollo-carrier-ups".to_string()],
            ),
        ] {
            state.sessions.insert(
                session_id.to_string(),
                BrowserSession {
                    id: session_id.to_string(),
                    profile_id: Some("stealthcdp-default".to_string()),
                    lease: LeaseState::Exclusive,
                    browser_ids: vec![browser_id.to_string()],
                    tab_ids,
                    profile_lease_conflict_session_ids: conflicts,
                    ..BrowserSession::default()
                },
            );
        }
        state.tabs.insert(
            "target:usps".to_string(),
            BrowserTab {
                id: "target:usps".to_string(),
                browser_id: "session:odollo-carrier-usps".to_string(),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        assert_eq!(merge_duplicate_live_browser_records(&mut state), 1);

        assert!(!state.browsers.contains_key("session:odollo-carrier-ups"));
        let canonical = &state.browsers["session:odollo-carrier-usps"];
        assert_eq!(canonical.pid, Some(14983));
        assert_eq!(
            canonical.active_session_ids,
            vec!["odollo-carrier-usps", "odollo-carrier-ups"]
        );
        assert_eq!(
            state.sessions["odollo-carrier-ups"].browser_ids,
            vec!["session:odollo-carrier-usps"]
        );
        assert_eq!(
            state.sessions["odollo-carrier-usps"].browser_ids,
            vec!["session:odollo-carrier-usps"]
        );
        assert!(state.sessions["odollo-carrier-ups"]
            .profile_lease_conflict_session_ids
            .is_empty());
        assert!(state.sessions["odollo-carrier-usps"]
            .profile_lease_conflict_session_ids
            .is_empty());
        assert_eq!(
            state.events.last().unwrap().details.as_ref().unwrap()["action"],
            "duplicate_browser_record_merged"
        );
    }

    #[tokio::test]
    async fn reconcile_records_tab_metadata_change_events() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(
            listener,
            r#"[
                {"id":"page-1","type":"page","title":"New Title","url":"https://example.com/new"}
            ]"#,
        ));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                url: Some("https://example.com/old".to_string()),
                title: Some("Old Title".to_string()),
                ..BrowserTab::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(tab_event.message, "Tab target:page-1 metadata changed");
        assert_eq!(
            tab_event.details.as_ref().unwrap()["previousUrl"],
            "https://example.com/old"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentUrl"],
            "https://example.com/new"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["previousTitle"],
            "Old Title"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentTitle"],
            "New Title"
        );
    }

    #[tokio::test]
    async fn reconcile_records_summary_snapshot() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            ..BrowserProcess::default()
        });

        let summary = reconcile_service_state(&mut state).await;

        assert_eq!(summary.browser_count, 0);
        assert_eq!(summary.changed_browsers, 1);
        let reconciliation = state.reconciliation.as_ref().unwrap();
        assert_eq!(reconciliation.browser_count, 0);
        assert_eq!(reconciliation.changed_browsers, 1);
        assert!(reconciliation.last_reconciled_at.is_some());
        assert_eq!(reconciliation.last_error, None);
        assert_eq!(state.events.len(), 2);
        assert_eq!(state.events[0].kind, ServiceEventKind::BrowserHealthChanged);
        assert_eq!(state.events[0].browser_id.as_deref(), Some("browser-1"));
        assert_eq!(state.events[0].previous_health, Some(BrowserHealth::Ready));
        assert_eq!(
            state.events[0].current_health,
            Some(BrowserHealth::Unreachable)
        );
        assert!(!state.browsers.contains_key("browser-1"));
        assert_eq!(state.events[1].kind, ServiceEventKind::Reconciliation);
    }

    #[tokio::test]
    async fn reconcile_expires_stale_session_leases_with_response_evidence() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Launching,
            active_session_ids: vec!["expired-session".to_string(), "fresh-session".to_string()],
            ..BrowserProcess::default()
        });
        state.sessions.insert(
            "expired-session".to_string(),
            BrowserSession {
                id: "expired-session".to_string(),
                lease: LeaseState::Shared,
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["tab-expired".to_string()],
                expires_at: Some("2026-01-01T00:00:00Z".to_string()),
                ..BrowserSession::default()
            },
        );
        state.sessions.insert(
            "fresh-session".to_string(),
            BrowserSession {
                id: "fresh-session".to_string(),
                lease: LeaseState::Shared,
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["tab-fresh".to_string()],
                expires_at: Some("2999-01-01T00:00:00Z".to_string()),
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "tab-expired".to_string(),
            BrowserTab {
                id: "tab-expired".to_string(),
                browser_id: "browser-1".to_string(),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("expired-session".to_string()),
                ..BrowserTab::default()
            },
        );
        state.tabs.insert(
            "tab-fresh".to_string(),
            BrowserTab {
                id: "tab-fresh".to_string(),
                browser_id: "browser-1".to_string(),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("fresh-session".to_string()),
                ..BrowserTab::default()
            },
        );

        let summary = reconcile_service_state(&mut state).await;

        assert_eq!(
            summary.expired_session_leases,
            vec!["expired-session".to_string()]
        );
        assert_eq!(state.sessions["expired-session"].lease, LeaseState::Expired);
        assert_eq!(state.sessions["fresh-session"].lease, LeaseState::Shared);
        assert!(state.browsers.contains_key("browser-1"));
        assert_eq!(
            state.browsers["browser-1"].active_session_ids,
            vec!["fresh-session".to_string()]
        );
        assert!(state.tabs.contains_key("tab-expired"));
        let reconciliation = state.events.last().unwrap();
        assert_eq!(reconciliation.kind, ServiceEventKind::Reconciliation);
        assert_eq!(
            reconciliation.details.as_ref().unwrap()["expiredSessionLeases"],
            serde_json::json!(["expired-session"])
        );
        assert_eq!(
            reconciliation.details.as_ref().unwrap()["expiredSessionLeaseCount"],
            1
        );
    }

    #[tokio::test]
    async fn reconcile_removes_process_exited_browser_operational_state_after_event() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            pid: Some(i32::MAX as u32),
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        });
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:old".to_string()],
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "target:old".to_string(),
            BrowserTab {
                id: "target:old".to_string(),
                browser_id: "browser-1".to_string(),
                session_id: Some("session-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        let summary = reconcile_service_state(&mut state).await;

        assert_eq!(summary.browser_count, 0);
        assert_eq!(summary.changed_browsers, 1);
        assert!(!state.browsers.contains_key("browser-1"));
        assert!(!state.sessions.contains_key("session-1"));
        assert!(!state.tabs.contains_key("target:old"));
        let health_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::BrowserHealthChanged)
            .unwrap();
        assert_eq!(health_event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(health_event.previous_health, Some(BrowserHealth::Ready));
        assert_eq!(
            health_event.current_health,
            Some(BrowserHealth::ProcessExited)
        );
        let reconciliation = state.events.last().unwrap();
        assert_eq!(reconciliation.kind, ServiceEventKind::Reconciliation);
        assert_eq!(
            reconciliation.details.as_ref().unwrap()["removedTerminatedBrowsers"],
            1
        );
    }

    #[tokio::test]
    async fn reconcile_removes_empty_historical_browser_placeholders() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([
                (
                    "session:missing-session".to_string(),
                    BrowserProcess {
                        id: "session:missing-session".to_string(),
                        health: BrowserHealth::NotStarted,
                        active_session_ids: vec!["missing-session".to_string()],
                        profile_id: Some("default".to_string()),
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "session:released-faulted".to_string(),
                    BrowserProcess {
                        id: "session:released-faulted".to_string(),
                        health: BrowserHealth::Faulted,
                        active_session_ids: vec!["released-faulted".to_string()],
                        view_streams: vec![ViewStream {
                            id: "stale-stream".to_string(),
                            ..ViewStream::default()
                        }],
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            sessions: BTreeMap::from([(
                "released-faulted".to_string(),
                BrowserSession {
                    id: "released-faulted".to_string(),
                    lease: LeaseState::Released,
                    browser_ids: vec!["session:released-faulted".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };

        let summary = reconcile_service_state(&mut state).await;

        assert_eq!(summary.browser_count, 0);
        assert_eq!(summary.changed_browsers, 2);
        assert!(state.browsers.is_empty());
        assert!(state.sessions.is_empty());
        assert_eq!(
            state.events.last().unwrap().details.as_ref().unwrap()["removedTerminatedBrowsers"],
            2
        );
    }

    #[tokio::test]
    async fn reconcile_event_log_is_bounded() {
        let mut state = ServiceState {
            events: (0..MAX_SERVICE_EVENTS)
                .map(|i| ServiceEvent {
                    id: format!("old-{i}"),
                    timestamp: "2026-04-22T00:00:00Z".to_string(),
                    kind: ServiceEventKind::Reconciliation,
                    message: "old".to_string(),
                    ..ServiceEvent::default()
                })
                .collect(),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::NotStarted,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        reconcile_service_state(&mut state).await;

        assert_eq!(state.events.len(), MAX_SERVICE_EVENTS);
        assert_ne!(state.events[0].id, "old-0");
        assert_eq!(
            state.events.last().map(|event| event.kind),
            Some(ServiceEventKind::Reconciliation)
        );
    }

    #[tokio::test]
    async fn refresh_marks_unreachable_cdp_without_pid_unreachable() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            ..BrowserProcess::default()
        });

        refresh_persisted_browser_health(&mut state).await;

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::Unreachable);
        assert!(browser
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("CDP endpoint is unreachable"));
    }

    #[tokio::test]
    async fn refresh_marks_dead_pid_process_exited() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            pid: Some(i32::MAX as u32),
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            ..BrowserProcess::default()
        });

        refresh_persisted_browser_health(&mut state).await;

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::ProcessExited);
        assert!(browser
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("no longer running"));
    }
}
