//! Browser, session, and tab lifecycle for the ordinary single-user path.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    select_least_crowded_remote_view_desktop, BrowserDisposableProfilePolicy,
    BrowserProfileCatalog, BrowserProfileCatalogEntry, BrowserProfileKind,
    RemoteViewDesktopCandidate, RemoteViewFixedDesktop, RemoteViewPresentationRetention,
};

pub const BROWSER_SESSION_STATE_SCHEMA_V1: &str = "agent-browser.browser-session-state.v1";
const MAX_HISTORY_COMPACTION_EVENTS: usize = 512;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrowserSessionManagerConfig {
    pub session_idle_timeout_ms: u64,
    pub remote_view_desktops: Vec<RemoteViewDesktopCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserLaunch {
    pub browser_id: String,
    pub pid: u32,
    pub cdp_endpoint: String,
    pub desktop: Option<RemoteViewFixedDesktop>,
}

/// Replacement physical targets for a retained logical browser and its tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserRecovery {
    pub launch: BrowserLaunch,
    pub target_ids: BTreeMap<String, String>,
}

/// Executes a command for an exact manager-owned browser and tab.
pub trait ManagedBrowserCommandEffects {
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

pub trait BrowserSessionEffects {
    /// Recover only after positive process and profile absence. The adapter
    /// retains launch custody until the host publishes the replacement atomically.
    fn recover_browser(
        &mut self,
        _browser: &ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
        _tabs: &[ManagedBrowserTab],
        _navigation: &[BrowserNavigationRecord],
    ) -> Result<BrowserRecovery, String> {
        Err("browser_session_recovery_unsupported".into())
    }
    /// Select an executable for subsequent launches; existing browsers retain identity.
    fn select_browser_executable(&mut self, _path: String) -> Result<(), String> {
        Err("browser_build_selection_unsupported".into())
    }

    /// Issue or resolve a retained presentation without capacity or browser launch.
    /// The host publishes the request before calling and the outcome afterward.
    fn resolve_remote_view_tab_view(
        &mut self,
        _target: &crate::RemoteViewTabHandoffTarget,
        _view: &crate::RemoteViewTabView,
        _now_ms: u64,
    ) -> Result<crate::RemoteViewApplicationViewIssuance, String> {
        Err("remote_view_tab_view_unsupported".into())
    }

    /// Read installed transport readiness for a published grant without issuing.
    /// Runtime callers separately prove the exact tab and owned foreground window.
    fn observe_remote_view_tab_view(
        &mut self,
        _target: &crate::RemoteViewTabHandoffTarget,
        _issuance: &crate::RemoteViewApplicationViewIssuance,
        _now_ms: u64,
    ) -> Result<crate::RemoteViewApplicationViewObservation, String> {
        Err("remote_view_tab_observation_unsupported".into())
    }

    /// Whether ordinary browser requests may create sessions through this
    /// owner. Local legacy adapters keep explicit session admission.
    fn admits_ordinary_sessions(&self) -> bool {
        false
    }

    /// Refresh provider-owned candidates only when a browser launch needs
    /// placement. None retains the configured candidates for local adapters.
    fn remote_view_desktop_candidates(
        &mut self,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        Ok(None)
    }

    /// Receive the host's durable baseline before any operation effects. A
    /// custody adapter must retain unresolved launch intent across failures.
    fn begin_operation(&mut self, _expected: &BrowserSessionState) -> Result<(), String> {
        Ok(())
    }

    /// An observed launch awaiting atomic session-and-custody publication.
    fn pending_launch_intent(&self) -> Option<crate::BrowserLaunchIntent> {
        None
    }

    /// Called only after durable session-and-custody publication succeeds.
    fn acknowledge_launch_publication(&mut self) {}

    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String>;

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String>;

    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String>;

    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String>;

    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String>;

    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String>;

    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String>;

    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String>;

    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation_id: &str,
        session_name: &str,
    ) -> Result<BrowserProfileCatalogEntry, String>;

    fn delete_disposable_profile(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<(), String>;

    /// Measure regular-file bytes below the recorded disposable profile root.
    /// Implementations must not follow symbolic links.
    fn disposable_profile_size_bytes(
        &mut self,
        _allocation: &ManagedDisposableProfile,
    ) -> Result<u64, String> {
        Ok(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserTabSource {
    Bootstrap,
    SessionInitial,
    Current,
    ExplicitNew,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTabAcquisition {
    pub tab_id: String,
    pub target_id: String,
    pub source: BrowserTabSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseBrowserTabResult {
    pub closed_tab_id: String,
    pub current_tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserProfileIntent {
    Exact { profile_id: String },
    Disposable { policy_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenBrowserSession {
    pub session_name: String,
    pub profile_intent: BrowserProfileIntent,
    pub activity_at_ms: u64,
}

impl OpenBrowserSession {
    pub fn exact_profile(
        session_name: impl Into<String>,
        profile_id: impl Into<String>,
        activity_at_ms: u64,
    ) -> Self {
        Self {
            session_name: session_name.into(),
            profile_intent: BrowserProfileIntent::Exact {
                profile_id: profile_id.into(),
            },
            activity_at_ms,
        }
    }

    pub fn disposable(
        session_name: impl Into<String>,
        policy_id: impl Into<String>,
        activity_at_ms: u64,
    ) -> Self {
        Self {
            session_name: session_name.into(),
            profile_intent: BrowserProfileIntent::Disposable {
                policy_id: policy_id.into(),
            },
            activity_at_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionBrowserDisposition {
    Launched,
    Reused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRecordDisposition {
    Created,
    Reused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    ExplicitClose,
    HeartbeatExpired,
    QuotaEvicted,
    BrowserTerminated,
    BrowserUnresponsive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCloseDisposition {
    BrowserPreserved,
    BrowserClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseBrowserSessionResult {
    pub session_id: String,
    pub browser_id: String,
    pub disposition: SessionCloseDisposition,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReapBrowserSessionsResult {
    pub expired_session_ids: Vec<String>,
    pub quota_evicted_session_ids: Vec<String>,
    pub closed_browser_ids: Vec<String>,
    pub deleted_disposable_profile_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenBrowserSessionResult {
    pub session_id: String,
    pub session_name: String,
    pub profile_id: String,
    pub browser_id: String,
    pub disposition: SessionBrowserDisposition,
    pub session_disposition: SessionRecordDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBrowserSession {
    pub id: String,
    pub name: String,
    pub profile_id: String,
    pub browser_id: String,
    pub created_at_ms: u64,
    pub last_activity_at_ms: u64,
    pub expires_at_ms: u64,
    pub current_tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBrowserInstance {
    pub id: String,
    pub profile_id: String,
    pub pid: u32,
    pub cdp_endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<RemoteViewFixedDesktop>,
    pub active_session_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedDisposableProfile {
    pub profile: BrowserProfileCatalogEntry,
    pub policy_id: String,
    pub session_name: String,
    pub user_data_root: String,
    pub created_at_ms: u64,
    pub cleanup_delay_ms: u64,
    pub cleanup_eligible_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBrowserTab {
    pub id: String,
    pub target_id: String,
    pub browser_id: String,
    pub session_id: String,
    pub created_at_ms: u64,
    pub last_activity_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserTabEndReason {
    ExplicitClose,
    SessionEnded,
    BrowserEnded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalBrowserTab {
    pub id: String,
    pub target_id: String,
    pub browser_id: String,
    pub session_id: String,
    pub profile_id: String,
    pub created_at_ms: u64,
    pub last_activity_at_ms: u64,
    pub closed_at_ms: u64,
    pub reason: BrowserTabEndReason,
    pub session_end_reason: Option<SessionEndReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserNavigationRecord {
    pub profile_id: String,
    pub session_id: String,
    pub browser_id: String,
    pub tab_id: String,
    pub target_id: String,
    pub url: String,
    pub visited_at_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incident_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserNavigationDailySummary {
    pub utc_day: String,
    pub profile_id: String,
    pub session_id: String,
    pub browser_id: String,
    pub tab_id: String,
    pub target_id: String,
    pub first_url: String,
    pub last_url: String,
    pub navigation_count: u64,
    pub first_visited_at_ms: u64,
    pub last_visited_at_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incident_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHistoryCompactionEvent {
    pub compacted_at_ms: u64,
    pub exact_bytes_before: u64,
    pub exact_bytes_after: u64,
    pub removed_navigation_count: u64,
    pub affected_summary_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusBrowserResult {
    pub browser_id: String,
    pub tab_id: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalBrowserSession {
    pub id: String,
    pub name: String,
    pub profile_id: String,
    pub browser_id: String,
    pub created_at_ms: u64,
    pub last_activity_at_ms: u64,
    pub ended_at_ms: u64,
    pub reason: SessionEndReason,
}

/// A normal tab request is retained before its first browser effect. A missing
/// result requires reconciliation; it never permits another creation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedBrowserTabRequest {
    pub session_name: String,
    pub command_digest: String,
    pub profile_id: Option<String>,
    pub response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserSessionState {
    pub schema_version: String,
    pub next_session_sequence: u64,
    pub next_disposable_sequence: u64,
    pub browsers: BTreeMap<String, ManagedBrowserInstance>,
    pub remote_view_presentations: BTreeMap<String, RemoteViewPresentationRetention>,
    /// Logical tab bindings survive provider route and desktop changes.
    pub remote_view_tab_handoffs: BTreeMap<String, crate::RemoteViewTabHandoff>,
    pub managed_tab_requests: BTreeMap<String, ManagedBrowserTabRequest>,
    pub sessions: BTreeMap<String, ManagedBrowserSession>,
    pub disposable_profiles: BTreeMap<String, ManagedDisposableProfile>,
    pub tabs: BTreeMap<String, ManagedBrowserTab>,
    pub tab_history: Vec<TerminalBrowserTab>,
    pub navigation_history: Vec<BrowserNavigationRecord>,
    pub navigation_daily_summaries: Vec<BrowserNavigationDailySummary>,
    pub history_compaction_events: Vec<BrowserHistoryCompactionEvent>,
    pub session_history: Vec<TerminalBrowserSession>,
}

impl Default for BrowserSessionState {
    fn default() -> Self {
        Self {
            schema_version: BROWSER_SESSION_STATE_SCHEMA_V1.to_string(),
            next_session_sequence: 0,
            next_disposable_sequence: 0,
            browsers: BTreeMap::new(),
            remote_view_presentations: BTreeMap::new(),
            remote_view_tab_handoffs: BTreeMap::new(),
            managed_tab_requests: BTreeMap::new(),
            sessions: BTreeMap::new(),
            disposable_profiles: BTreeMap::new(),
            tabs: BTreeMap::new(),
            tab_history: Vec::new(),
            navigation_history: Vec::new(),
            navigation_daily_summaries: Vec::new(),
            history_compaction_events: Vec::new(),
            session_history: Vec::new(),
        }
    }
}

impl BrowserSessionState {
    /// Keep exact navigation rows within the configured serialized-byte budget
    /// while preserving deterministic daily identity summaries.
    pub fn compact_navigation_history(
        &mut self,
        maximum_exact_bytes: u64,
    ) -> Result<Option<BrowserHistoryCompactionEvent>, String> {
        if maximum_exact_bytes < 2 {
            return Err("browser_navigation_history_limit_invalid".to_string());
        }
        let exact_bytes_before = serde_json::to_vec(&self.navigation_history)
            .map_err(|error| format!("browser_navigation_history_measure_failed:{error}"))?
            .len() as u64;
        if exact_bytes_before <= maximum_exact_bytes {
            return Ok(None);
        }

        let mut oldest_indices: Vec<usize> = (0..self.navigation_history.len()).collect();
        oldest_indices.sort_by(|left_index, right_index| {
            let left = &self.navigation_history[*left_index];
            let right = &self.navigation_history[*right_index];
            (
                left.visited_at_ms,
                left.profile_id.as_str(),
                left.session_id.as_str(),
                left.browser_id.as_str(),
                left.tab_id.as_str(),
                left.target_id.as_str(),
                left.url.as_str(),
            )
                .cmp(&(
                    right.visited_at_ms,
                    right.profile_id.as_str(),
                    right.session_id.as_str(),
                    right.browser_id.as_str(),
                    right.tab_id.as_str(),
                    right.target_id.as_str(),
                    right.url.as_str(),
                ))
                .then_with(|| left_index.cmp(right_index))
        });

        let measured_after_removing = |count: usize| -> Result<u64, String> {
            let removed: BTreeSet<usize> = oldest_indices.iter().take(count).copied().collect();
            let retained: Vec<&BrowserNavigationRecord> = self
                .navigation_history
                .iter()
                .enumerate()
                .filter_map(|(index, record)| (!removed.contains(&index)).then_some(record))
                .collect();
            serde_json::to_vec(&retained)
                .map(|value| value.len() as u64)
                .map_err(|error| format!("browser_navigation_history_measure_failed:{error}"))
        };
        let mut low = 1usize;
        let mut high = oldest_indices.len();
        while low < high {
            let middle = low + (high - low) / 2;
            if measured_after_removing(middle)? <= maximum_exact_bytes {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        let removed_indices: BTreeSet<usize> = oldest_indices.iter().take(low).copied().collect();
        let mut removed = Vec::with_capacity(low);
        let mut retained = Vec::with_capacity(self.navigation_history.len() - low);
        for (index, record) in self.navigation_history.drain(..).enumerate() {
            if removed_indices.contains(&index) {
                removed.push(record);
            } else {
                retained.push(record);
            }
        }
        self.navigation_history = retained;
        removed.sort_by(|left, right| {
            (left.visited_at_ms, left.url.as_str()).cmp(&(right.visited_at_ms, right.url.as_str()))
        });

        let mut summaries: BTreeMap<
            (String, String, String, String, String, String),
            BrowserNavigationDailySummary,
        > = self
            .navigation_daily_summaries
            .drain(..)
            .map(|summary| {
                (
                    (
                        summary.utc_day.clone(),
                        summary.profile_id.clone(),
                        summary.session_id.clone(),
                        summary.browser_id.clone(),
                        summary.tab_id.clone(),
                        summary.target_id.clone(),
                    ),
                    summary,
                )
            })
            .collect();
        let mut affected = BTreeSet::new();
        for record in &removed {
            let visited_at_ms = i64::try_from(record.visited_at_ms)
                .map_err(|_| "browser_navigation_timestamp_invalid".to_string())?;
            let utc_day = chrono::DateTime::from_timestamp_millis(visited_at_ms)
                .ok_or_else(|| "browser_navigation_timestamp_invalid".to_string())?
                .format("%Y-%m-%d")
                .to_string();
            let key = (
                utc_day.clone(),
                record.profile_id.clone(),
                record.session_id.clone(),
                record.browser_id.clone(),
                record.tab_id.clone(),
                record.target_id.clone(),
            );
            affected.insert(key.clone());
            let summary = summaries
                .entry(key)
                .or_insert_with(|| BrowserNavigationDailySummary {
                    utc_day,
                    profile_id: record.profile_id.clone(),
                    session_id: record.session_id.clone(),
                    browser_id: record.browser_id.clone(),
                    tab_id: record.tab_id.clone(),
                    target_id: record.target_id.clone(),
                    first_url: record.url.clone(),
                    last_url: record.url.clone(),
                    navigation_count: 0,
                    first_visited_at_ms: record.visited_at_ms,
                    last_visited_at_ms: record.visited_at_ms,
                    incident_ids: Vec::new(),
                });
            if record.visited_at_ms < summary.first_visited_at_ms
                || (record.visited_at_ms == summary.first_visited_at_ms
                    && record.url.as_str() < summary.first_url.as_str())
            {
                summary.first_visited_at_ms = record.visited_at_ms;
                summary.first_url = record.url.clone();
            }
            if record.visited_at_ms > summary.last_visited_at_ms
                || (record.visited_at_ms == summary.last_visited_at_ms
                    && record.url.as_str() > summary.last_url.as_str())
            {
                summary.last_visited_at_ms = record.visited_at_ms;
                summary.last_url = record.url.clone();
            }
            summary.navigation_count = summary.navigation_count.saturating_add(1);
            summary
                .incident_ids
                .extend(record.incident_ids.iter().cloned());
            summary.incident_ids.sort();
            summary.incident_ids.dedup();
        }
        self.navigation_daily_summaries = summaries.into_values().collect();
        let exact_bytes_after = serde_json::to_vec(&self.navigation_history)
            .map_err(|error| format!("browser_navigation_history_measure_failed:{error}"))?
            .len() as u64;
        let event = BrowserHistoryCompactionEvent {
            compacted_at_ms: removed
                .iter()
                .map(|record| record.visited_at_ms)
                .max()
                .unwrap_or_default(),
            exact_bytes_before,
            exact_bytes_after,
            removed_navigation_count: removed.len() as u64,
            affected_summary_count: affected.len() as u64,
        };
        self.history_compaction_events.push(event.clone());
        if self.history_compaction_events.len() > MAX_HISTORY_COMPACTION_EVENTS {
            let excess = self.history_compaction_events.len() - MAX_HISTORY_COMPACTION_EVENTS;
            self.history_compaction_events.drain(..excess);
        }
        Ok(Some(event))
    }
}

pub struct BrowserSessionManager<'a, E> {
    state: &'a mut BrowserSessionState,
    catalog: &'a BrowserProfileCatalog,
    effects: &'a mut E,
    config: BrowserSessionManagerConfig,
    protected_session_ids: BTreeSet<String>,
}

impl<'a, E: BrowserSessionEffects> BrowserSessionManager<'a, E> {
    pub fn new(
        state: &'a mut BrowserSessionState,
        catalog: &'a BrowserProfileCatalog,
        effects: &'a mut E,
        config: BrowserSessionManagerConfig,
    ) -> Self {
        // A valid retained operator handoff is current occupancy. Its grant may
        // expire independently, but inactivity must not destroy its exact tab.
        // Historical, closed and conflicting bindings confer no retention.
        let protected_session_ids = state
            .remote_view_tab_handoffs
            .keys()
            .filter_map(|id| crate::resolve_remote_view_tab_handoff(state, id).ok())
            .map(|target| target.session.id)
            .collect();
        Self {
            state,
            catalog,
            effects,
            config,
            protected_session_ids,
        }
    }

    /// Add sessions referenced by current external authority or a pending
    /// operation to the retained-handoff protection for this manager action.
    pub fn with_protected_sessions(mut self, session_ids: BTreeSet<String>) -> Self {
        self.protected_session_ids.extend(session_ids);
        self
    }

    pub fn open(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        let expires_at_ms = request
            .activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        let profile = self.resolve_profile_for_open(&request)?;
        let expired_matching_sessions = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.name == request.session_name
                    && session.profile_id == profile.id
                    && session.expires_at_ms <= request.activity_at_ms
                    && !self.protected_session_ids.contains(&session.id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        for session_id in expired_matching_sessions {
            self.close_session(
                &session_id,
                SessionEndReason::HeartbeatExpired,
                request.activity_at_ms,
            )?;
        }
        let existing_session = self
            .state
            .sessions
            .values()
            .find(|session| {
                session.name == request.session_name
                    && session.profile_id == profile.id
                    && (request.activity_at_ms < session.expires_at_ms
                        || self.protected_session_ids.contains(&session.id))
            })
            .cloned();
        if let Some(session) = existing_session {
            let browser = self
                .state
                .browsers
                .get(&session.browser_id)
                .cloned()
                .ok_or_else(|| "browser_session_browser_missing".to_string())?;
            if self.effects.browser_is_live(&browser)? {
                let stored = self
                    .state
                    .sessions
                    .get_mut(&session.id)
                    .ok_or_else(|| "browser_session_missing_during_refresh".to_string())?;
                stored.last_activity_at_ms = request.activity_at_ms;
                stored.expires_at_ms = expires_at_ms;
                return Ok(OpenBrowserSessionResult {
                    session_id: session.id,
                    session_name: session.name,
                    profile_id: session.profile_id,
                    browser_id: session.browser_id,
                    disposition: SessionBrowserDisposition::Reused,
                    session_disposition: SessionRecordDisposition::Reused,
                });
            }
            if self.protected_session_ids.contains(&session.id) {
                self.recover_retained_browser(&browser.id)?;
                let stored = self
                    .state
                    .sessions
                    .get_mut(&session.id)
                    .ok_or("browser_session_missing_during_recovery")?;
                stored.last_activity_at_ms = request.activity_at_ms;
                stored.expires_at_ms = expires_at_ms;
                return Ok(OpenBrowserSessionResult {
                    session_id: session.id,
                    session_name: session.name,
                    profile_id: session.profile_id,
                    browser_id: session.browser_id,
                    disposition: SessionBrowserDisposition::Launched,
                    session_disposition: SessionRecordDisposition::Reused,
                });
            }
            self.retire_browser(
                &browser,
                SessionEndReason::BrowserUnresponsive,
                request.activity_at_ms,
            )?;
        }
        let next_session_sequence = self
            .state
            .next_session_sequence
            .checked_add(1)
            .ok_or_else(|| "browser_session_sequence_exhausted".to_string())?;
        let reusable_browser = self
            .state
            .browsers
            .values()
            .find(|browser| browser.profile_id == profile.id)
            .cloned();
        let (browser_id, disposition, launched) = if let Some(browser) = reusable_browser {
            if self.effects.browser_is_live(&browser)? {
                (browser.id, SessionBrowserDisposition::Reused, None)
            } else {
                self.retire_browser(
                    &browser,
                    SessionEndReason::BrowserUnresponsive,
                    request.activity_at_ms,
                )?;
                let launch = self.launch_browser(&profile)?;
                (
                    launch.browser_id.clone(),
                    SessionBrowserDisposition::Launched,
                    Some(launch),
                )
            }
        } else {
            let launch = self.launch_browser(&profile)?;
            (
                launch.browser_id.clone(),
                SessionBrowserDisposition::Launched,
                Some(launch),
            )
        };

        self.state.next_session_sequence = next_session_sequence;
        let session_id = format!(
            "session:{}:{}:{}",
            request.session_name, profile.id, self.state.next_session_sequence
        );

        if let Some(launch) = launched {
            self.state.browsers.insert(
                browser_id.clone(),
                ManagedBrowserInstance {
                    id: browser_id.clone(),
                    profile_id: profile.id.clone(),
                    pid: launch.pid,
                    cdp_endpoint: launch.cdp_endpoint,
                    desktop: launch.desktop,
                    active_session_ids: Vec::new(),
                },
            );
        }
        let browser = self
            .state
            .browsers
            .get_mut(&browser_id)
            .ok_or_else(|| "browser_session_browser_missing_after_open".to_string())?;
        browser.active_session_ids.push(session_id.clone());
        self.state.sessions.insert(
            session_id.clone(),
            ManagedBrowserSession {
                id: session_id.clone(),
                name: request.session_name.clone(),
                profile_id: profile.id.clone(),
                browser_id: browser_id.clone(),
                created_at_ms: request.activity_at_ms,
                last_activity_at_ms: request.activity_at_ms,
                expires_at_ms,
                current_tab_id: None,
            },
        );
        if let Some(allocation) = self.state.disposable_profiles.get_mut(&profile.id) {
            allocation.cleanup_eligible_at_ms = None;
        }

        Ok(OpenBrowserSessionResult {
            session_id,
            session_name: request.session_name,
            profile_id: profile.id.clone(),
            browser_id,
            disposition,
            session_disposition: SessionRecordDisposition::Created,
        })
    }

    /// Recover a retained logical browser without ending sessions or tabs.
    /// Physical absence and launch custody belong to the effect adapter.
    pub fn recover_retained_browser(&mut self, browser_id: &str) -> Result<(), String> {
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .cloned()
            .ok_or("browser_session_recovery_browser_missing")?;
        if self.effects.browser_is_live(&browser)? {
            return Ok(());
        }
        if !browser
            .active_session_ids
            .iter()
            .any(|id| self.protected_session_ids.contains(id))
        {
            return Err("browser_session_recovery_not_retained".into());
        }
        let profile = self
            .catalog
            .profiles
            .get(&browser.profile_id)
            .or_else(|| {
                self.state
                    .disposable_profiles
                    .get(&browser.profile_id)
                    .map(|p| &p.profile)
            })
            .ok_or("browser_session_recovery_profile_missing")?
            .clone();
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser.id)
            .cloned()
            .collect::<Vec<_>>();
        let recovered = self.effects.recover_browser(
            &browser,
            &profile,
            &tabs,
            &self.state.navigation_history,
        )?;
        let targets = recovered.target_ids.values().collect::<BTreeSet<_>>();
        if recovered.launch.browser_id != browser.id
            || recovered.launch.pid == 0
            || recovered.launch.cdp_endpoint.is_empty()
            || recovered.launch.desktop != browser.desktop
            || recovered.target_ids.len() != tabs.len()
            || targets.len() != tabs.len()
            || tabs.iter().any(|tab| {
                recovered
                    .target_ids
                    .get(&tab.id)
                    .is_none_or(|id| id.is_empty())
            })
        {
            return Err("browser_session_recovery_identity_conflict".into());
        }
        let mut current = browser;
        current.pid = recovered.launch.pid;
        current.cdp_endpoint = recovered.launch.cdp_endpoint;
        self.state.browsers.insert(current.id.clone(), current);
        for tab in tabs {
            self.state
                .tabs
                .get_mut(&tab.id)
                .expect("retained recovery tab")
                .target_id = recovered.target_ids[&tab.id].clone();
        }
        Ok(())
    }

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
    ) -> Result<BrowserLaunch, String> {
        let refreshed = self.effects.remote_view_desktop_candidates()?;
        let candidates = refreshed
            .as_deref()
            .unwrap_or(&self.config.remote_view_desktops);
        let desktop = if candidates.is_empty() {
            None
        } else {
            let live_desktop_ids = self
                .state
                .browsers
                .values()
                .filter_map(|browser| {
                    browser
                        .desktop
                        .as_ref()
                        .map(|desktop| desktop.desktop_id.clone())
                })
                .collect::<Vec<_>>();
            Some(select_least_crowded_remote_view_desktop(candidates, &live_desktop_ids)?.desktop)
        };
        let mut launch = self.effects.launch_browser(profile, desktop.as_ref())?;
        if launch.desktop != desktop {
            return Err("browser_session_launch_desktop_mismatch".to_string());
        }
        launch.desktop = desktop;
        Ok(launch)
    }

    fn resolve_profile_for_open(
        &mut self,
        request: &OpenBrowserSession,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        match &request.profile_intent {
            BrowserProfileIntent::Exact { profile_id } => {
                let profile = self
                    .catalog
                    .profiles
                    .get(profile_id)
                    .cloned()
                    .ok_or_else(|| format!("browser_profile_not_found:{profile_id}"))?;
                if profile.kind != BrowserProfileKind::Named {
                    return Err(format!(
                        "browser_profile_requires_disposable_intent:{profile_id}"
                    ));
                }
                Ok(profile)
            }
            BrowserProfileIntent::Disposable { policy_id } => {
                let policy = self
                    .catalog
                    .disposable_policies
                    .get(policy_id)
                    .cloned()
                    .ok_or_else(|| format!("browser_disposable_policy_not_found:{policy_id}"))?;
                if let Some(allocation) =
                    self.state.disposable_profiles.values().find(|allocation| {
                        allocation.policy_id == policy.id
                            && allocation.session_name == request.session_name
                    })
                {
                    return Ok(allocation.profile.clone());
                }
                self.enforce_disposable_quota(
                    policy_id,
                    &self.protected_session_ids.clone(),
                    1,
                    0,
                    request.activity_at_ms,
                )?;
                self.state.next_disposable_sequence = self
                    .state
                    .next_disposable_sequence
                    .checked_add(1)
                    .ok_or_else(|| "browser_disposable_sequence_exhausted".to_string())?;
                let allocation_id = format!(
                    "disposable:{}:{}",
                    policy.id, self.state.next_disposable_sequence
                );
                let cleanup_eligible_at_ms = request
                    .activity_at_ms
                    .checked_add(policy.cleanup_delay_ms)
                    .ok_or_else(|| "browser_disposable_cleanup_expiry_exhausted".to_string())?;
                let profile = self.effects.allocate_disposable_profile(
                    &policy,
                    &allocation_id,
                    &request.session_name,
                )?;
                if profile.id != allocation_id || profile.kind != BrowserProfileKind::Disposable {
                    return Err("browser_disposable_allocation_identity_invalid".to_string());
                }
                let root = std::path::Path::new(&policy.user_data_root);
                let allocated_path = std::path::Path::new(&profile.user_data_dir);
                if allocated_path == root || !allocated_path.starts_with(root) {
                    return Err("browser_disposable_allocation_path_invalid".to_string());
                }
                self.state.disposable_profiles.insert(
                    profile.id.clone(),
                    ManagedDisposableProfile {
                        profile: profile.clone(),
                        policy_id: policy.id,
                        session_name: request.session_name.clone(),
                        user_data_root: policy.user_data_root,
                        created_at_ms: request.activity_at_ms,
                        cleanup_delay_ms: policy.cleanup_delay_ms,
                        cleanup_eligible_at_ms: Some(cleanup_eligible_at_ms),
                    },
                );
                Ok(profile)
            }
        }
    }

    pub fn close_session(
        &mut self,
        session_id: &str,
        reason: SessionEndReason,
        ended_at_ms: u64,
    ) -> Result<CloseBrowserSessionResult, String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let final_session =
            browser.active_session_ids.len() == 1 && browser.active_session_ids[0] == session.id;
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.session_id == session.id)
            .cloned()
            .collect::<Vec<_>>();
        if !final_session && !tabs.is_empty() {
            let remaining_session_id = browser
                .active_session_ids
                .iter()
                .find(|active_id| active_id.as_str() != session.id)
                .cloned()
                .ok_or_else(|| "browser_session_remaining_session_missing".to_string())?;
            let remaining_has_tab = self
                .state
                .tabs
                .values()
                .any(|tab| tab.session_id == remaining_session_id);
            if !remaining_has_tab {
                let attributed_target_ids = self
                    .state
                    .tabs
                    .values()
                    .filter(|tab| tab.browser_id == browser.id)
                    .map(|tab| tab.target_id.clone())
                    .collect::<Vec<_>>();
                let acquisition = self
                    .effects
                    .acquire_initial_tab(&browser, &attributed_target_ids)?;
                if !matches!(
                    acquisition.source,
                    BrowserTabSource::Bootstrap | BrowserTabSource::SessionInitial
                ) {
                    return Err("browser_tab_initial_source_invalid".to_string());
                }
                if self.state.tabs.contains_key(&acquisition.tab_id) {
                    return Err("browser_tab_already_attributed".to_string());
                }
                self.state.tabs.insert(
                    acquisition.tab_id.clone(),
                    ManagedBrowserTab {
                        id: acquisition.tab_id.clone(),
                        target_id: acquisition.target_id,
                        browser_id: browser.id.clone(),
                        session_id: remaining_session_id.clone(),
                        created_at_ms: ended_at_ms,
                        last_activity_at_ms: ended_at_ms,
                    },
                );
                let remaining_session = self
                    .state
                    .sessions
                    .get_mut(&remaining_session_id)
                    .ok_or_else(|| "browser_session_remaining_session_missing".to_string())?;
                remaining_session.current_tab_id = Some(acquisition.tab_id);
            }
        }
        if final_session {
            self.effects.close_browser(&browser)?;
        } else {
            for tab in &tabs {
                self.effects.close_tab(&browser, tab)?;
            }
        }

        self.state.sessions.remove(session_id);
        for tab in tabs {
            self.state.tabs.remove(&tab.id);
            self.state.tab_history.push(TerminalBrowserTab {
                id: tab.id,
                target_id: tab.target_id,
                browser_id: tab.browser_id,
                session_id: tab.session_id,
                profile_id: session.profile_id.clone(),
                created_at_ms: tab.created_at_ms,
                last_activity_at_ms: tab.last_activity_at_ms,
                closed_at_ms: ended_at_ms,
                reason: BrowserTabEndReason::SessionEnded,
                session_end_reason: Some(reason),
            });
        }
        self.state.session_history.push(TerminalBrowserSession {
            id: session.id.clone(),
            name: session.name,
            profile_id: session.profile_id.clone(),
            browser_id: session.browser_id.clone(),
            created_at_ms: session.created_at_ms,
            last_activity_at_ms: session.last_activity_at_ms,
            ended_at_ms,
            reason,
        });

        let disposition = if final_session {
            self.state.browsers.remove(&session.browser_id);
            SessionCloseDisposition::BrowserClosed
        } else {
            let stored_browser = self
                .state
                .browsers
                .get_mut(&session.browser_id)
                .ok_or_else(|| "browser_session_browser_missing_during_close".to_string())?;
            stored_browser
                .active_session_ids
                .retain(|active_id| active_id != session_id);
            SessionCloseDisposition::BrowserPreserved
        };
        self.mark_disposable_cleanup_eligible(&session.profile_id, ended_at_ms)?;

        Ok(CloseBrowserSessionResult {
            session_id: session.id,
            browser_id: session.browser_id,
            disposition,
        })
    }

    pub fn tab_for_navigation(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<BrowserTabAcquisition, String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let expires_at_ms = activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;

        if let Some(tab_id) = &session.current_tab_id {
            let tab = self
                .state
                .tabs
                .get_mut(tab_id)
                .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
            tab.last_activity_at_ms = activity_at_ms;
            let target_id = tab.target_id.clone();
            let stored_session = self
                .state
                .sessions
                .get_mut(session_id)
                .ok_or_else(|| "browser_session_missing_during_tab_refresh".to_string())?;
            stored_session.last_activity_at_ms = activity_at_ms;
            stored_session.expires_at_ms = expires_at_ms;
            return Ok(BrowserTabAcquisition {
                tab_id: tab_id.clone(),
                target_id,
                source: BrowserTabSource::Current,
            });
        }

        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let attributed_target_ids = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser.id)
            .map(|tab| tab.target_id.clone())
            .collect::<Vec<_>>();
        let acquisition = self
            .effects
            .acquire_initial_tab(&browser, &attributed_target_ids)?;
        if !matches!(
            acquisition.source,
            BrowserTabSource::Bootstrap | BrowserTabSource::SessionInitial
        ) {
            return Err("browser_tab_initial_source_invalid".to_string());
        }
        if self.state.tabs.contains_key(&acquisition.tab_id) {
            return Err("browser_tab_already_attributed".to_string());
        }
        self.state.tabs.insert(
            acquisition.tab_id.clone(),
            ManagedBrowserTab {
                id: acquisition.tab_id.clone(),
                target_id: acquisition.target_id.clone(),
                browser_id: browser.id,
                session_id: session.id.clone(),
                created_at_ms: activity_at_ms,
                last_activity_at_ms: activity_at_ms,
            },
        );
        let stored_session = self
            .state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_missing_during_tab_adoption".to_string())?;
        stored_session.current_tab_id = Some(acquisition.tab_id.clone());
        stored_session.last_activity_at_ms = activity_at_ms;
        stored_session.expires_at_ms = expires_at_ms;
        Ok(acquisition)
    }

    pub fn new_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<BrowserTabAcquisition, String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let expires_at_ms = activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        let acquisition = self.effects.create_tab(&browser)?;
        if acquisition.source != BrowserTabSource::ExplicitNew {
            return Err("browser_tab_new_source_invalid".to_string());
        }
        if self.state.tabs.contains_key(&acquisition.tab_id) {
            return Err("browser_tab_already_attributed".to_string());
        }
        self.state.tabs.insert(
            acquisition.tab_id.clone(),
            ManagedBrowserTab {
                id: acquisition.tab_id.clone(),
                target_id: acquisition.target_id.clone(),
                browser_id: browser.id,
                session_id: session.id.clone(),
                created_at_ms: activity_at_ms,
                last_activity_at_ms: activity_at_ms,
            },
        );
        let stored_session = self
            .state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_missing_during_tab_creation".to_string())?;
        stored_session.current_tab_id = Some(acquisition.tab_id.clone());
        stored_session.last_activity_at_ms = activity_at_ms;
        stored_session.expires_at_ms = expires_at_ms;
        Ok(acquisition)
    }

    pub fn close_current_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<CloseBrowserTabResult, String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let tab_id = session
            .current_tab_id
            .as_deref()
            .ok_or_else(|| "browser_session_current_tab_absent".to_string())?;
        let tab = self
            .state
            .tabs
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        if tab.session_id != session.id {
            return Err("browser_tab_session_attribution_mismatch".to_string());
        }
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let expires_at_ms = activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        self.effects.close_tab(&browser, &tab)?;
        self.state.tabs.remove(&tab.id);
        self.state.tab_history.push(TerminalBrowserTab {
            id: tab.id.clone(),
            target_id: tab.target_id.clone(),
            browser_id: tab.browser_id.clone(),
            session_id: tab.session_id.clone(),
            profile_id: session.profile_id.clone(),
            created_at_ms: tab.created_at_ms,
            last_activity_at_ms: tab.last_activity_at_ms,
            closed_at_ms: activity_at_ms,
            reason: BrowserTabEndReason::ExplicitClose,
            session_end_reason: None,
        });
        let current_tab_id = self
            .state
            .tabs
            .values()
            .filter(|candidate| candidate.session_id == session.id)
            .max_by_key(|candidate| {
                (
                    candidate.last_activity_at_ms,
                    candidate.created_at_ms,
                    candidate.id.as_str(),
                )
            })
            .map(|candidate| candidate.id.clone());
        let stored_session = self
            .state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_missing_during_tab_close".to_string())?;
        stored_session.current_tab_id = current_tab_id.clone();
        stored_session.last_activity_at_ms = activity_at_ms;
        stored_session.expires_at_ms = expires_at_ms;
        Ok(CloseBrowserTabResult {
            closed_tab_id: tab.id,
            current_tab_id,
        })
    }

    pub fn record_navigation(
        &mut self,
        session_id: &str,
        url: &str,
        visited_at_ms: u64,
    ) -> Result<BrowserNavigationRecord, String> {
        if url.trim().is_empty() {
            return Err("browser_navigation_url_empty".to_string());
        }
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let tab_id = session
            .current_tab_id
            .as_deref()
            .ok_or_else(|| "browser_session_current_tab_absent".to_string())?;
        let tab = self
            .state
            .tabs
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        if tab.session_id != session.id {
            return Err("browser_tab_session_attribution_mismatch".to_string());
        }
        let expires_at_ms = visited_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        let record = BrowserNavigationRecord {
            profile_id: session.profile_id,
            session_id: session.id,
            browser_id: session.browser_id,
            tab_id: tab.id.clone(),
            target_id: tab.target_id,
            url: url.to_string(),
            visited_at_ms,
            incident_ids: Vec::new(),
        };
        self.state.navigation_history.push(record.clone());
        let stored_tab =
            self.state.tabs.get_mut(&tab.id).ok_or_else(|| {
                "browser_session_current_tab_missing_during_navigation".to_string()
            })?;
        stored_tab.last_activity_at_ms = visited_at_ms;
        let stored_session = self
            .state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_missing_during_navigation".to_string())?;
        stored_session.last_activity_at_ms = visited_at_ms;
        stored_session.expires_at_ms = expires_at_ms;
        Ok(record)
    }

    pub fn navigate(
        &mut self,
        session_id: &str,
        url: &str,
        activity_at_ms: u64,
    ) -> Result<BrowserNavigationRecord, String> {
        let acquisition = self.tab_for_navigation(session_id, activity_at_ms)?;
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        let tab = self
            .state
            .tabs
            .get(&acquisition.tab_id)
            .cloned()
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        self.effects.navigate(&browser, &tab, url)?;
        self.record_navigation(session_id, url, activity_at_ms)
    }

    pub fn focus_browser(
        &mut self,
        browser_id: &str,
        target_id: Option<&str>,
        activity_at_ms: u64,
    ) -> Result<FocusBrowserResult, String> {
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_browser_not_found:{browser_id}"))?;
        if !self.effects.browser_is_live(&browser)? {
            return Err(format!("browser_session_browser_not_live:{browser_id}"));
        }
        let tab = match target_id {
            Some(target_id) => Some(
                self.state
                    .tabs
                    .values()
                    .find(|tab| tab.browser_id == browser_id && tab.target_id == target_id)
                    .cloned()
                    .ok_or_else(|| format!("browser_session_target_not_found:{target_id}"))?,
            ),
            None => self
                .state
                .tabs
                .values()
                .filter(|tab| tab.browser_id == browser_id)
                .max_by_key(|tab| (tab.last_activity_at_ms, tab.created_at_ms))
                .cloned(),
        };
        self.effects.focus_browser(&browser, tab.as_ref())?;
        if let Some(tab) = tab.as_ref() {
            if let Some(current) = self.state.tabs.get_mut(&tab.id) {
                current.last_activity_at_ms = activity_at_ms;
            }
            if let Some(session) = self.state.sessions.get_mut(&tab.session_id) {
                session.current_tab_id = Some(tab.id.clone());
                session.last_activity_at_ms = activity_at_ms;
                session.expires_at_ms = activity_at_ms
                    .checked_add(self.config.session_idle_timeout_ms)
                    .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
            }
        }
        Ok(FocusBrowserResult {
            browser_id: browser.id,
            tab_id: tab.as_ref().map(|tab| tab.id.clone()),
            target_id: tab.map(|tab| tab.target_id),
        })
    }

    pub fn reap(&mut self, now_ms: u64) -> Result<ReapBrowserSessionsResult, String> {
        self.reap_with_protected_sessions(now_ms, &BTreeSet::new())
    }

    pub fn reap_with_protected_sessions(
        &mut self,
        now_ms: u64,
        protected_session_ids: &BTreeSet<String>,
    ) -> Result<ReapBrowserSessionsResult, String> {
        let protected_session_ids = self
            .protected_session_ids
            .union(protected_session_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let expired_session_ids = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.expires_at_ms <= now_ms && !protected_session_ids.contains(&session.id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        let mut result = ReapBrowserSessionsResult::default();
        for session_id in expired_session_ids {
            let closed =
                self.close_session(&session_id, SessionEndReason::HeartbeatExpired, now_ms)?;
            result.expired_session_ids.push(session_id);
            if closed.disposition == SessionCloseDisposition::BrowserClosed {
                result.closed_browser_ids.push(closed.browser_id);
            }
        }
        let disposable_profile_ids = self
            .state
            .disposable_profiles
            .values()
            .filter(|allocation| {
                allocation
                    .cleanup_eligible_at_ms
                    .is_some_and(|eligible_at_ms| eligible_at_ms <= now_ms)
                    && !self
                        .state
                        .sessions
                        .values()
                        .any(|session| session.profile_id == allocation.profile.id)
                    && !self
                        .state
                        .browsers
                        .values()
                        .any(|browser| browser.profile_id == allocation.profile.id)
            })
            .map(|allocation| allocation.profile.id.clone())
            .collect::<Vec<_>>();
        for profile_id in disposable_profile_ids {
            let allocation = self
                .state
                .disposable_profiles
                .get(&profile_id)
                .cloned()
                .ok_or_else(|| "browser_disposable_profile_missing_during_reap".to_string())?;
            self.effects.delete_disposable_profile(&allocation)?;
            self.state.disposable_profiles.remove(&profile_id);
            result.deleted_disposable_profile_ids.push(profile_id);
        }
        let policy_ids = self
            .state
            .disposable_profiles
            .values()
            .map(|allocation| allocation.policy_id.clone())
            .collect::<BTreeSet<_>>();
        for policy_id in policy_ids {
            let quota =
                self.enforce_disposable_quota(&policy_id, &protected_session_ids, 0, 0, now_ms)?;
            result
                .quota_evicted_session_ids
                .extend(quota.quota_evicted_session_ids);
            result.closed_browser_ids.extend(quota.closed_browser_ids);
            result
                .deleted_disposable_profile_ids
                .extend(quota.deleted_disposable_profile_ids);
        }
        Ok(result)
    }

    /// Converge one disposable policy before admission or scheduled reaping.
    /// Candidate selection proves unprotected profiles can satisfy both limits
    /// before any browser or filesystem effect occurs.
    pub fn enforce_disposable_quota(
        &mut self,
        policy_id: &str,
        protected_session_ids: &BTreeSet<String>,
        reserve_profiles: u32,
        reserve_bytes: u64,
        now_ms: u64,
    ) -> Result<ReapBrowserSessionsResult, String> {
        let policy = self
            .catalog
            .disposable_policies
            .get(policy_id)
            .cloned()
            .ok_or_else(|| format!("browser_disposable_policy_not_found:{policy_id}"))?;
        let mut retained_count = 0_u32;
        let mut retained_bytes = 0_u64;
        let mut candidates = Vec::new();
        for allocation in self
            .state
            .disposable_profiles
            .values()
            .filter(|allocation| allocation.policy_id == policy_id)
        {
            retained_count = retained_count
                .checked_add(1)
                .ok_or_else(|| "browser_disposable_profile_count_exhausted".to_string())?;
            let bytes = self.effects.disposable_profile_size_bytes(allocation)?;
            retained_bytes = retained_bytes
                .checked_add(bytes)
                .ok_or_else(|| "browser_disposable_profile_bytes_exhausted".to_string())?;
            let sessions = self
                .state
                .sessions
                .values()
                .filter(|session| session.profile_id == allocation.profile.id)
                .cloned()
                .collect::<Vec<_>>();
            let protected_session = sessions
                .iter()
                .any(|session| protected_session_ids.contains(&session.id));
            let browser_without_session = sessions.is_empty()
                && self
                    .state
                    .browsers
                    .values()
                    .any(|browser| browser.profile_id == allocation.profile.id);
            let protected = protected_session || browser_without_session;
            let last_activity_at_ms = sessions
                .iter()
                .map(|session| session.last_activity_at_ms)
                .max()
                .unwrap_or(allocation.created_at_ms);
            candidates.push((
                last_activity_at_ms,
                allocation.created_at_ms,
                allocation.profile.id.clone(),
                bytes,
                protected,
            ));
        }
        let target_count = retained_count
            .checked_add(reserve_profiles)
            .ok_or_else(|| "browser_disposable_profile_count_exhausted".to_string())?;
        let target_bytes = retained_bytes
            .checked_add(reserve_bytes)
            .ok_or_else(|| "browser_disposable_profile_bytes_exhausted".to_string())?;
        if target_count <= policy.maximum_retained_profiles
            && target_bytes <= policy.maximum_total_bytes
        {
            return Ok(ReapBrowserSessionsResult::default());
        }

        candidates
            .sort_by(|left, right| (left.0, left.1, &left.2).cmp(&(right.0, right.1, &right.2)));
        let mut projected_count = target_count;
        let mut projected_bytes = target_bytes;
        let mut selected = Vec::new();
        for (_, _, profile_id, bytes, protected) in candidates {
            if projected_count <= policy.maximum_retained_profiles
                && projected_bytes <= policy.maximum_total_bytes
            {
                break;
            }
            if protected {
                continue;
            }
            projected_count = projected_count.saturating_sub(1);
            projected_bytes = projected_bytes.saturating_sub(bytes);
            selected.push(profile_id);
        }
        if projected_count > policy.maximum_retained_profiles {
            return Err("browser_disposable_profile_count_quota_protected".to_string());
        }
        if projected_bytes > policy.maximum_total_bytes {
            return Err("browser_disposable_profile_bytes_quota_protected".to_string());
        }

        let mut result = ReapBrowserSessionsResult::default();
        for profile_id in selected {
            let session_ids = self
                .state
                .sessions
                .values()
                .filter(|session| session.profile_id == profile_id)
                .map(|session| session.id.clone())
                .collect::<Vec<_>>();
            for session_id in session_ids {
                let closed =
                    self.close_session(&session_id, SessionEndReason::QuotaEvicted, now_ms)?;
                result.quota_evicted_session_ids.push(session_id);
                if closed.disposition == SessionCloseDisposition::BrowserClosed {
                    result.closed_browser_ids.push(closed.browser_id);
                }
            }
            if self
                .state
                .sessions
                .values()
                .any(|session| session.profile_id == profile_id)
                || self
                    .state
                    .browsers
                    .values()
                    .any(|browser| browser.profile_id == profile_id)
            {
                return Err("browser_disposable_profile_still_referenced".to_string());
            }
            let allocation = self
                .state
                .disposable_profiles
                .get(&profile_id)
                .cloned()
                .ok_or_else(|| "browser_disposable_profile_missing_during_quota".to_string())?;
            self.effects.delete_disposable_profile(&allocation)?;
            self.state.disposable_profiles.remove(&profile_id);
            result.deleted_disposable_profile_ids.push(profile_id);
        }
        Ok(result)
    }

    fn mark_disposable_cleanup_eligible(
        &mut self,
        profile_id: &str,
        ended_at_ms: u64,
    ) -> Result<(), String> {
        let Some(allocation) = self.state.disposable_profiles.get_mut(profile_id) else {
            return Ok(());
        };
        allocation.cleanup_eligible_at_ms = Some(
            ended_at_ms
                .checked_add(allocation.cleanup_delay_ms)
                .ok_or_else(|| "browser_disposable_cleanup_expiry_exhausted".to_string())?,
        );
        Ok(())
    }

    fn retire_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        reason: SessionEndReason,
        ended_at_ms: u64,
    ) -> Result<(), String> {
        self.effects.close_browser(browser)?;
        let session_ids = browser.active_session_ids.clone();
        for session_id in session_ids {
            if let Some(session) = self.state.sessions.remove(&session_id) {
                let tabs = self
                    .state
                    .tabs
                    .values()
                    .filter(|tab| tab.session_id == session.id)
                    .cloned()
                    .collect::<Vec<_>>();
                for tab in tabs {
                    self.state.tabs.remove(&tab.id);
                    self.state.tab_history.push(TerminalBrowserTab {
                        id: tab.id,
                        target_id: tab.target_id,
                        browser_id: tab.browser_id,
                        session_id: tab.session_id,
                        profile_id: session.profile_id.clone(),
                        created_at_ms: tab.created_at_ms,
                        last_activity_at_ms: tab.last_activity_at_ms,
                        closed_at_ms: ended_at_ms,
                        reason: BrowserTabEndReason::BrowserEnded,
                        session_end_reason: Some(reason),
                    });
                }
                self.state.session_history.push(TerminalBrowserSession {
                    id: session.id,
                    name: session.name,
                    profile_id: session.profile_id.clone(),
                    browser_id: session.browser_id,
                    created_at_ms: session.created_at_ms,
                    last_activity_at_ms: session.last_activity_at_ms,
                    ended_at_ms,
                    reason,
                });
                self.mark_disposable_cleanup_eligible(&session.profile_id, ended_at_ms)?;
            }
        }
        self.state.browsers.remove(&browser.id);
        Ok(())
    }
}
