//! Browser, session, and tab lifecycle for the ordinary single-user path.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    select_least_crowded_browser_desktop, BrowserDesktopAssignment, BrowserDesktopRoute,
    BrowserDisposableProfilePolicy, BrowserProfileCatalog, BrowserProfileCatalogEntry,
    BrowserProfileKind, RecordedProcessIdentity,
};

pub const BROWSER_SESSION_STATE_SCHEMA_V1: &str = "agent-browser.browser-session-state.v1";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrowserSessionManagerConfig {
    pub session_idle_timeout_ms: u64,
    pub remote_desktop_routes: Vec<BrowserDesktopRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserLaunch {
    pub browser_id: String,
    pub pid: u32,
    pub cdp_endpoint: String,
    pub process_identity: Option<RecordedProcessIdentity>,
    pub desktop: Option<BrowserDesktopAssignment>,
}

pub trait BrowserSessionEffects {
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String>;

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
    ) -> Result<BrowserLaunch, String>;

    fn launch_browser_reserved(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&BrowserDesktopAssignment>,
        browser_id: &str,
    ) -> Result<BrowserLaunch, String> {
        let launch = self.launch_browser(profile, desktop)?;
        if launch.browser_id != browser_id {
            return Err("browser_session_reserved_browser_identity_mismatch".to_string());
        }
        Ok(launch)
    }

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
    /// Opaque durable handoffs owned by this exact logical session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handoff_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBrowserInstance {
    pub id: String,
    pub profile_id: String,
    pub pid: u32,
    pub cdp_endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_identity: Option<RecordedProcessIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<BrowserDesktopAssignment>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserSessionState {
    pub schema_version: String,
    pub next_session_sequence: u64,
    pub next_disposable_sequence: u64,
    pub browsers: BTreeMap<String, ManagedBrowserInstance>,
    pub sessions: BTreeMap<String, ManagedBrowserSession>,
    pub disposable_profiles: BTreeMap<String, ManagedDisposableProfile>,
    pub tabs: BTreeMap<String, ManagedBrowserTab>,
    pub tab_history: Vec<TerminalBrowserTab>,
    pub navigation_history: Vec<BrowserNavigationRecord>,
    pub session_history: Vec<TerminalBrowserSession>,
}

impl Default for BrowserSessionState {
    fn default() -> Self {
        Self {
            schema_version: BROWSER_SESSION_STATE_SCHEMA_V1.to_string(),
            next_session_sequence: 0,
            next_disposable_sequence: 0,
            browsers: BTreeMap::new(),
            sessions: BTreeMap::new(),
            disposable_profiles: BTreeMap::new(),
            tabs: BTreeMap::new(),
            tab_history: Vec::new(),
            navigation_history: Vec::new(),
            session_history: Vec::new(),
        }
    }
}

impl BrowserSessionState {
    /// Bind a durable manager handoff to the logical session that prepared it.
    pub fn bind_manager_handoff(
        &mut self,
        session_id: &str,
        handoff_id: &str,
    ) -> Result<(), String> {
        if handoff_id.is_empty() {
            return Err("browser_session_handoff_id_missing".to_string());
        }
        if self.sessions.iter().any(|(id, session)| {
            id != session_id && session.handoff_ids.iter().any(|owned| owned == handoff_id)
        }) {
            return Err("browser_session_handoff_session_identity_changed".to_string());
        }
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_handoff_session_missing".to_string())?;
        if !session.handoff_ids.iter().any(|id| id == handoff_id) {
            session.handoff_ids.push(handoff_id.to_string());
        }
        Ok(())
    }

    /// Refresh only the session that owns a successfully resolved handoff.
    pub fn refresh_manager_handoff_activity(
        &mut self,
        session_id: &str,
        handoff_id: &str,
        activity_at_ms: u64,
        idle_timeout_ms: u64,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_handoff_session_ended".to_string())?;
        if !session.handoff_ids.iter().any(|id| id == handoff_id) {
            return Err("browser_session_handoff_session_identity_changed".to_string());
        }
        if session.expires_at_ms < activity_at_ms {
            return Err("browser_session_handoff_session_expired".to_string());
        }
        let expires_at_ms = activity_at_ms
            .checked_add(idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        session.last_activity_at_ms = session.last_activity_at_ms.max(activity_at_ms);
        session.expires_at_ms = session.expires_at_ms.max(expires_at_ms);
        Ok(())
    }

    /// Record every active manager session and tab as ended after an adapter
    /// has independently proved that all manager-owned browser processes are
    /// stopped. Named profile catalog entries and profile directories are not
    /// part of this state and are intentionally untouched.
    pub fn terminalize_after_cold_shutdown(&mut self, ended_at_ms: u64) {
        let sessions = std::mem::take(&mut self.sessions);
        let tabs = std::mem::take(&mut self.tabs);
        for tab in tabs.into_values() {
            let profile_id = sessions
                .get(&tab.session_id)
                .map(|session| session.profile_id.clone())
                .or_else(|| {
                    self.browsers
                        .get(&tab.browser_id)
                        .map(|browser| browser.profile_id.clone())
                })
                .unwrap_or_default();
            self.tab_history.push(TerminalBrowserTab {
                id: tab.id,
                target_id: tab.target_id,
                browser_id: tab.browser_id,
                session_id: tab.session_id,
                profile_id,
                created_at_ms: tab.created_at_ms,
                last_activity_at_ms: tab.last_activity_at_ms,
                closed_at_ms: ended_at_ms,
                reason: BrowserTabEndReason::BrowserEnded,
                session_end_reason: Some(SessionEndReason::BrowserTerminated),
            });
        }
        for session in sessions.into_values() {
            if let Some(allocation) = self.disposable_profiles.get_mut(&session.profile_id) {
                allocation.cleanup_eligible_at_ms =
                    Some(ended_at_ms.saturating_add(allocation.cleanup_delay_ms));
            }
            self.session_history.push(TerminalBrowserSession {
                id: session.id,
                name: session.name,
                profile_id: session.profile_id,
                browser_id: session.browser_id,
                created_at_ms: session.created_at_ms,
                last_activity_at_ms: session.last_activity_at_ms,
                ended_at_ms,
                reason: SessionEndReason::BrowserTerminated,
            });
        }
        self.browsers.clear();
    }
}

pub struct BrowserSessionManager<'a, E> {
    state: &'a mut BrowserSessionState,
    catalog: &'a BrowserProfileCatalog,
    effects: &'a mut E,
    config: BrowserSessionManagerConfig,
    desktop_capacity: Option<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserOpenReservation {
    pub session_id: String,
    pub browser_id: String,
    pub desktop: BrowserDesktopAssignment,
}

impl<'a, E: BrowserSessionEffects> BrowserSessionManager<'a, E> {
    pub fn new(
        state: &'a mut BrowserSessionState,
        catalog: &'a BrowserProfileCatalog,
        effects: &'a mut E,
        config: BrowserSessionManagerConfig,
    ) -> Self {
        Self {
            state,
            catalog,
            effects,
            config,
            desktop_capacity: None,
        }
    }

    /// Apply current allocation limits without moving or closing existing browsers.
    pub fn with_desktop_capacity(
        mut self,
        maximum_displays: u32,
        maximum_browsers_per_display: u32,
    ) -> Self {
        self.desktop_capacity = Some((maximum_displays, maximum_browsers_per_display));
        self
    }

    pub fn open(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        self.open_internal(request, None, None, true)
    }

    /// Select or create a session before a browser command without refreshing
    /// an existing session's heartbeat until that command succeeds.
    pub fn open_for_command(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        self.open_internal(request, None, None, false)
    }

    pub fn open_reserved(
        &mut self,
        request: OpenBrowserSession,
        reservation: BrowserOpenReservation,
    ) -> Result<OpenBrowserSessionResult, String> {
        self.open_internal(request, Some(reservation), None, true)
    }

    /// Publishes an exact reserved browser launch that was already observed by
    /// an operation-recovery path without launching a second browser.
    ///
    /// The candidate state is applied only after every reserved identity and
    /// normal open invariant passes. This keeps a mismatched recovery
    /// observation from partially publishing manager state.
    pub fn open_reserved_observed(
        &mut self,
        request: OpenBrowserSession,
        reservation: BrowserOpenReservation,
        observed_launch: BrowserLaunch,
    ) -> Result<OpenBrowserSessionResult, String> {
        let mut candidate_state = self.state.clone();
        let result = {
            let mut candidate_manager = BrowserSessionManager::new(
                &mut candidate_state,
                self.catalog,
                self.effects,
                self.config.clone(),
            );
            candidate_manager.open_internal(
                request,
                Some(reservation),
                Some(observed_launch),
                true,
            )?
        };
        *self.state = candidate_state;
        Ok(result)
    }

    fn open_internal(
        &mut self,
        request: OpenBrowserSession,
        reservation: Option<BrowserOpenReservation>,
        mut observed_launch: Option<BrowserLaunch>,
        refresh_activity_on_reuse: bool,
    ) -> Result<OpenBrowserSessionResult, String> {
        let profile = self.resolve_profile_for_open(&request)?;
        let expired_matching_sessions = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.name == request.session_name
                    && session.profile_id == profile.id
                    && session.expires_at_ms <= request.activity_at_ms
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
                    && request.activity_at_ms < session.expires_at_ms
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
                if reservation.as_ref().is_some_and(|reservation| {
                    reservation.session_id != session.id
                        || reservation.browser_id != browser.id
                        || browser.desktop.as_ref() != Some(&reservation.desktop)
                }) {
                    return Err("browser_session_open_reservation_mismatch".to_string());
                }
                if observed_launch.is_some() {
                    return Err("browser_session_observed_launch_unexpected_on_reuse".to_string());
                }
                if refresh_activity_on_reuse {
                    let expires_at_ms = request
                        .activity_at_ms
                        .checked_add(self.config.session_idle_timeout_ms)
                        .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
                    let stored = self
                        .state
                        .sessions
                        .get_mut(&session.id)
                        .ok_or_else(|| "browser_session_missing_during_refresh".to_string())?;
                    stored.last_activity_at_ms = request.activity_at_ms;
                    stored.expires_at_ms = expires_at_ms;
                }
                return Ok(OpenBrowserSessionResult {
                    session_id: session.id,
                    session_name: session.name,
                    profile_id: session.profile_id,
                    browser_id: session.browser_id,
                    disposition: SessionBrowserDisposition::Reused,
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
        let generated_session_id = format!(
            "session:{}:{}:{next_session_sequence}",
            request.session_name, profile.id
        );
        if reservation
            .as_ref()
            .is_some_and(|reservation| reservation.session_id != generated_session_id)
        {
            return Err("browser_session_reserved_session_identity_mismatch".to_string());
        }
        let reusable_browser = self
            .state
            .browsers
            .values()
            .find(|browser| browser.profile_id == profile.id)
            .cloned();
        let (browser_id, disposition, launched) = if let Some(browser) = reusable_browser {
            if self.effects.browser_is_live(&browser)? {
                if reservation.as_ref().is_some_and(|reservation| {
                    reservation.browser_id != browser.id
                        || browser.desktop.as_ref() != Some(&reservation.desktop)
                }) {
                    return Err("browser_session_open_reservation_mismatch".to_string());
                }
                if observed_launch.is_some() {
                    return Err("browser_session_observed_launch_unexpected_on_reuse".to_string());
                }
                (browser.id, SessionBrowserDisposition::Reused, None)
            } else {
                self.retire_browser(
                    &browser,
                    SessionEndReason::BrowserUnresponsive,
                    request.activity_at_ms,
                )?;
                let launch = self.launch_or_adopt_browser(
                    &profile,
                    reservation.as_ref(),
                    &mut observed_launch,
                )?;
                (
                    launch.browser_id.clone(),
                    SessionBrowserDisposition::Launched,
                    Some(launch),
                )
            }
        } else {
            let launch =
                self.launch_or_adopt_browser(&profile, reservation.as_ref(), &mut observed_launch)?;
            (
                launch.browser_id.clone(),
                SessionBrowserDisposition::Launched,
                Some(launch),
            )
        };

        self.state.next_session_sequence = next_session_sequence;
        let session_id = reservation
            .as_ref()
            .map(|reservation| reservation.session_id.clone())
            .unwrap_or_else(|| generated_session_id.clone());
        if session_id != generated_session_id {
            return Err("browser_session_reserved_session_identity_mismatch".to_string());
        }
        if reservation
            .as_ref()
            .is_some_and(|reservation| reservation.browser_id != browser_id)
        {
            return Err("browser_session_reserved_browser_identity_mismatch".to_string());
        }
        let expires_at_ms = request
            .activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;

        if let Some(launch) = launched {
            self.state.browsers.insert(
                browser_id.clone(),
                ManagedBrowserInstance {
                    id: browser_id.clone(),
                    profile_id: profile.id.clone(),
                    pid: launch.pid,
                    cdp_endpoint: launch.cdp_endpoint,
                    process_identity: launch.process_identity,
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
                handoff_ids: Vec::new(),
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

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        reservation: Option<&BrowserOpenReservation>,
    ) -> Result<BrowserLaunch, String> {
        let selected_desktop =
            if self.config.remote_desktop_routes.is_empty() && self.desktop_capacity.is_none() {
                None
            } else {
                let live_display_names = self
                    .state
                    .browsers
                    .values()
                    .filter_map(|browser| {
                        browser
                            .desktop
                            .as_ref()
                            .map(|desktop| desktop.display_name.clone())
                    })
                    .collect::<Vec<_>>();
                Some(match self.desktop_capacity {
                    Some((maximum, density)) => crate::select_browser_desktop_with_capacity(
                        &self.config.remote_desktop_routes,
                        &live_display_names,
                        maximum,
                        density,
                    )?,
                    None => select_least_crowded_browser_desktop(
                        &self.config.remote_desktop_routes,
                        &live_display_names,
                    )?,
                })
            };
        let desktop = reservation
            .map(|reservation| Some(reservation.desktop.clone()))
            .unwrap_or_else(|| selected_desktop.clone());
        if reservation.is_some() && desktop != selected_desktop {
            return Err("browser_session_reserved_desktop_identity_mismatch".to_string());
        }
        let mut launch = if let Some(reservation) = reservation {
            self.effects.launch_browser_reserved(
                profile,
                desktop.as_ref(),
                &reservation.browser_id,
            )?
        } else {
            self.effects.launch_browser(profile, desktop.as_ref())?
        };
        if launch.desktop != desktop {
            return Err("browser_session_launch_desktop_mismatch".to_string());
        }
        launch.desktop = desktop;
        Ok(launch)
    }

    fn launch_or_adopt_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        reservation: Option<&BrowserOpenReservation>,
        observed_launch: &mut Option<BrowserLaunch>,
    ) -> Result<BrowserLaunch, String> {
        let Some(launch) = observed_launch.take() else {
            return self.launch_browser(profile, reservation);
        };
        let reservation = reservation
            .ok_or_else(|| "browser_session_observed_launch_requires_reservation".to_string())?;
        if launch.browser_id != reservation.browser_id {
            return Err("browser_session_reserved_browser_identity_mismatch".to_string());
        }
        let reserved_route_is_healthy = self.config.remote_desktop_routes.iter().any(|route| {
            route.healthy
                && route.id == reservation.desktop.route_id
                && route.display_name == reservation.desktop.display_name
        });
        if !reserved_route_is_healthy {
            return Err("browser_session_reserved_desktop_unavailable".to_string());
        }
        if launch.desktop.as_ref() != Some(&reservation.desktop) {
            return Err("browser_session_reserved_desktop_identity_mismatch".to_string());
        }
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
        self.ensure_session_tab(session_id, activity_at_ms, true)
    }

    /// Select or attribute the exact session tab before a command effect.
    /// A command attempt does not refresh its session heartbeat.
    pub fn tab_for_command(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<BrowserTabAcquisition, String> {
        self.ensure_session_tab(session_id, activity_at_ms, false)
    }

    /// Publish activity only after the exact session's command succeeds.
    pub fn record_successful_command(
        &mut self,
        session_id: &str,
        tab_id: &str,
        activity_at_ms: u64,
    ) -> Result<(), String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        if session.current_tab_id.as_deref() != Some(tab_id) {
            return Err("browser_session_current_tab_changed".to_string());
        }
        let tab = self
            .state
            .tabs
            .get(tab_id)
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        if tab.session_id != session_id || tab.browser_id != session.browser_id {
            return Err("browser_session_tab_attribution_mismatch".to_string());
        }
        let expires_at_ms = activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        let tab = self
            .state
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?;
        tab.last_activity_at_ms = tab.last_activity_at_ms.max(activity_at_ms);
        let session = self
            .state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
        session.last_activity_at_ms = session.last_activity_at_ms.max(activity_at_ms);
        session.expires_at_ms = session.expires_at_ms.max(expires_at_ms);
        Ok(())
    }

    fn ensure_session_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
        refresh_activity: bool,
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
            let target_id = tab.target_id.clone();
            if refresh_activity {
                tab.last_activity_at_ms = activity_at_ms;
                let stored_session = self
                    .state
                    .sessions
                    .get_mut(session_id)
                    .ok_or_else(|| "browser_session_missing_during_tab_refresh".to_string())?;
                stored_session.last_activity_at_ms = activity_at_ms;
                stored_session.expires_at_ms = expires_at_ms;
            }
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
        if refresh_activity {
            stored_session.last_activity_at_ms = activity_at_ms;
            stored_session.expires_at_ms = expires_at_ms;
        }
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
        let acquisition = self.tab_for_command(session_id, activity_at_ms)?;
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
        let expired_session_ids = self
            .state
            .sessions
            .values()
            .filter(|session| session.expires_at_ms <= now_ms)
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
        Ok(result)
    }

    /// Reconcile durable browser records against current process and CDP
    /// liveness before projecting active inventory.
    pub fn reconcile_liveness(&mut self, now_ms: u64) -> Result<Vec<String>, String> {
        let browsers = self.state.browsers.values().cloned().collect::<Vec<_>>();
        let mut retired = Vec::new();
        for browser in browsers {
            if !self.effects.browser_is_live(&browser)? {
                self.retire_browser(&browser, SessionEndReason::BrowserUnresponsive, now_ms)?;
                retired.push(browser.id);
            }
        }
        Ok(retired)
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
