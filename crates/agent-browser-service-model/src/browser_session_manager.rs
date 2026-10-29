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
    /// Kernel identity captured while the exact launched or verified attached
    /// process is still held. Absence supplies no original-process evidence.
    pub process_identity: Option<crate::RecordedProcessIdentity>,
    pub browser_id: String,
    pub pid: u32,
    /// Empty for a managed CDP-free native window. Native observation and input
    /// use Remote View; DOM actions and debugging attachment are unavailable.
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

/// Adapter-owned protection held through one automatic retirement effect group.
/// Dropping releases local activity exclusion. A separately persisted native
/// fence remains until exact settled state publication and explicit completion.
pub trait BrowserIdleRetirementGuard {}
impl BrowserIdleRetirementGuard for () {}

pub trait BrowserSessionEffects {
    /// Borrow an exact app browser using current local and Native observations.
    /// Unsupported adapters refuse before any launch or target effect.
    fn attach_app_browser(
        &mut self,
        _request: &crate::BrowserAttachmentRequest,
        _profile: &BrowserProfileCatalogEntry,
        _desktop: &RemoteViewFixedDesktop,
    ) -> Result<crate::BrowserAttachmentObservation, String> {
        Err("browser_attachment_capability_unavailable: exact process/CDP/desktop attachment is not supported by this adapter".into())
    }

    /// Read the current exact provider epoch before retaining a registration
    /// intent. This read confers no access and produces no use evidence.
    fn native_handoff_assignment_target(
        &mut self,
        _target: &crate::RemoteViewTabHandoffTarget,
    ) -> Result<crate::RemoteViewApplicationTarget, String> {
        Err("remote_view_native_observation_unsupported".into())
    }
    /// Register continuity with the native provider owner without a viewer
    /// grant or use assertion, after publication of the registration intent.
    fn register_native_handoff_observation(
        &mut self,
        _target: &crate::RemoteViewTabHandoffTarget,
        _provider_target: &crate::RemoteViewApplicationTarget,
        _target_digest: &str,
        _idempotency_key: &str,
    ) -> Result<crate::RemoteViewNativeHandoffObservation, String> {
        Err("remote_view_native_observation_unsupported".into())
    }

    /// Read private provider evidence joined to published launch custody.
    /// Public command evidence must never substitute this owner read.
    fn observe_native_handoff_use(
        &mut self,
        _target: &crate::RemoteViewTabHandoffTarget,
        _registered: &crate::RemoteViewNativeHandoffObservation,
    ) -> Result<crate::RemoteViewNativeHandoffUseRead, String> {
        Err("remote_view_native_observation_unsupported".into())
    }
    /// Reconcile a currently live exact-profile browser before placement or replacement.
    /// Return only verified attachment and task-target identities; do not launch or close peers.
    fn reconcile_profile_browser(
        &mut self,
        _profile: &BrowserProfileCatalogEntry,
        _browser: Option<&ManagedBrowserInstance>,
        _tabs: &[ManagedBrowserTab],
        _navigation: &[BrowserNavigationRecord],
    ) -> Result<Option<BrowserRecovery>, String> {
        Ok(None)
    }

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
    /// Explicit recovery onto an app-selected current Native assignment. The
    /// original absent browser remains the custody baseline until publication.
    /// This is process replacement, not relocation, and must never fall back.
    fn recover_browser_at(
        &mut self,
        _browser: &ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
        _tabs: &[ManagedBrowserTab],
        _navigation: &[BrowserNavigationRecord],
        _desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserRecovery, String> {
        Err("browser_reconnect_explicit_recovery_placement_unsupported".into())
    }

    /// Select an executable for subsequent launches; existing browsers retain identity.
    /// Pin one profile's next launches without changing another profile's build.
    fn select_profile_browser_executable(
        &mut self,
        _profile_id: String,
        _path: String,
    ) -> Result<(), String> {
        Err("browser_profile_build_selection_unsupported".into())
    }

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

    /// Observe explicit host placement without allocating ordinary spare capacity.
    fn remote_view_placement_candidates(
        &mut self,
        _placement: &BrowserPlacement,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        self.remote_view_desktop_candidates()
    }

    /// Receive the host's durable baseline before any operation effects. A
    /// custody adapter must retain unresolved launch intent across failures.
    fn begin_operation(&mut self, _expected: &BrowserSessionState) -> Result<(), String> {
        Ok(())
    }

    /// Initialize the borrowed task-expiry phase without allowing unrelated
    /// Native preparation recovery to precede its durable publication.
    fn begin_borrowed_retirement_operation(
        &mut self,
        expected: &BrowserSessionState,
    ) -> Result<(), String> {
        self.begin_operation(expected)
    }

    /// Exact automatic-retirement operations awaiting atomic state publication.
    fn pending_browser_activity_ids(&self) -> Vec<String> {
        Vec::new()
    }
    fn pending_idle_retirement_ids(&self) -> Vec<String> {
        Vec::new()
    }

    /// Called after atomic publication, never from guard destruction. Unknown
    /// provider completion retains original-operation readback obligations.
    fn finish_idle_retirement_publication(&mut self, _ids: &[String]) -> Result<(), String> {
        Ok(())
    }

    /// An observed launch awaiting atomic session-and-custody publication.
    fn pending_launch_intent(&self) -> Option<crate::BrowserLaunchIntent> {
        None
    }

    /// Called only after durable session-and-custody publication succeeds.
    fn acknowledge_launch_publication(&mut self) {}

    /// Apply one freshly admitted automatic effect against frozen original
    /// process/target custody. This must never fall back to current PID adoption.
    fn apply_idle_retirement_effect(
        &mut self,
        _scope: &crate::BrowserIdleRetirementScope,
        _tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        Err("browser_session_idle_original_effect_unavailable".into())
    }

    /// Read only the original close outcome. True requires conclusive physical
    /// absence; unknown observations never authorize another close attempt.
    fn observe_idle_retirement_effect(
        &mut self,
        _scope: &crate::BrowserIdleRetirementScope,
        _tab: Option<&ManagedBrowserTab>,
    ) -> Result<bool, String> {
        Err("browser_session_idle_effect_readback_unavailable".into())
    }

    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String>;

    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String>;

    /// Automatic expiry must preserve a browser while its desktop is being
    /// viewed. Explicit session close follows its separate user intent.
    fn permits_idle_browser_close(
        &mut self,
        _browser: &ManagedBrowserInstance,
    ) -> Result<bool, String> {
        Ok(true)
    }

    /// Revalidate current activity and retain adapter admission exclusion until
    /// the caller finishes automatic close effects. None defers without effects.
    /// Provider-free adapters have no concurrent authority and use the fallback;
    /// production adapters must supply the guard for their authority boundary.
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        for browser in browsers {
            if !self.permits_idle_browser_close(browser)? {
                return Ok(None);
            }
        }
        Ok(Some(Box::new(())))
    }

    /// Qualify this exact orphan allocation and retain activity admission through
    /// cleanup. Provider-free adapters have no external concurrent authority.
    /// Production adapters must check profile claims, jobs and physical custody.
    fn acquire_profile_cleanup_guard(
        &mut self,
        _allocation: &ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        Ok(Some(Box::new(())))
    }

    /// Admit every selected allocation and browser before any quota effect.
    /// Concurrent adapters must override this provider-free composition to check
    /// the complete group under one authority lock rather than nest locks.
    fn acquire_disposable_retirement_guard(
        &mut self,
        allocations: &[ManagedDisposableProfile],
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        struct GroupGuard {
            _guards: Vec<Box<dyn BrowserIdleRetirementGuard>>,
        }
        impl BrowserIdleRetirementGuard for GroupGuard {}
        let mut guards = Vec::new();
        for allocation in allocations {
            let Some(guard) = self.acquire_profile_cleanup_guard(allocation)? else {
                return Ok(None);
            };
            guards.push(guard);
        }
        let Some(guard) = self.acquire_idle_retirement_guard(browsers)? else {
            return Ok(None);
        };
        guards.push(guard);
        Ok(Some(Box::new(GroupGuard { _guards: guards })))
    }

    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String>;

    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String>;

    /// Acquire this named task's first tab without replaying an observed effect
    /// when session publication fails. Profile and task context are authoritative
    /// request inputs, rather than inferred bootstrap-tab ownership.
    fn acquire_session_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
        _session: &ManagedBrowserSession,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.acquire_initial_tab(browser, attributed_target_ids)
    }

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
    ExistingDisposable { profile_id: String },
    Disposable { policy_id: String },
}

/// Exact provider-owned desktop identity; generation fences stale assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserDesktopBinding {
    pub desktop_id: String,
    pub generation: u64,
}

/// Caller-authorized placement. Preferred alternatives are ordered and exhaustive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowserPlacement {
    Required {
        desktop: BrowserDesktopBinding,
    },
    Preferred {
        desktop: BrowserDesktopBinding,
        alternatives: Vec<BrowserDesktopBinding>,
    },
}

impl BrowserPlacement {
    fn bindings(&self) -> Vec<&BrowserDesktopBinding> {
        match self {
            Self::Required { desktop } => vec![desktop],
            Self::Preferred {
                desktop,
                alternatives,
            } => std::iter::once(desktop)
                .chain(alternatives.iter())
                .collect(),
        }
    }

    pub(crate) fn accepts(&self, desktop: Option<&RemoteViewFixedDesktop>) -> bool {
        desktop.is_some_and(|desktop| {
            self.bindings().iter().any(|binding| {
                binding.desktop_id == desktop.desktop_id && binding.generation == desktop.generation
            })
        })
    }

    pub(crate) fn select(
        &self,
        candidates: &[RemoteViewDesktopCandidate],
    ) -> Result<RemoteViewFixedDesktop, String> {
        for binding in self.bindings() {
            if let Some(candidate) = candidates.iter().find(|candidate| {
                candidate.ready
                    && candidate.desktop.desktop_id == binding.desktop_id
                    && candidate.desktop.generation == binding.generation
            }) {
                return Ok(candidate.desktop.clone());
            }
        }
        Err("browser_placement_unavailable:requested desktop generation is not ready; refresh placement or authorize an alternative".into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenBrowserSession {
    pub placement: Option<BrowserPlacement>,
    pub session_name: String,
    pub profile_intent: BrowserProfileIntent,
    pub activity_at_ms: u64,
}

impl OpenBrowserSession {
    /// Constrain this open without changing ordinary default placement.
    pub fn with_placement(mut self, placement: BrowserPlacement) -> Self {
        self.placement = Some(placement);
        self
    }

    /// Select one recorded disposable allocation without allocating or guessing
    /// another task's profile. Unknown allocations fail before browser effects.
    pub fn existing_disposable(
        session_name: impl Into<String>,
        profile_id: impl Into<String>,
        activity_at_ms: u64,
    ) -> Self {
        Self {
            placement: None,
            session_name: session_name.into(),
            profile_intent: BrowserProfileIntent::ExistingDisposable {
                profile_id: profile_id.into(),
            },
            activity_at_ms,
        }
    }
    pub fn exact_profile(
        session_name: impl Into<String>,
        profile_id: impl Into<String>,
        activity_at_ms: u64,
    ) -> Self {
        Self {
            placement: None,
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
            placement: None,
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
    pub expired_tab_ids: Vec<String>,
    pub expired_session_ids: Vec<String>,
    pub quota_evicted_session_ids: Vec<String>,
    pub closed_browser_ids: Vec<String>,
    pub deleted_disposable_profile_ids: Vec<String>,
    pub deferred_browser_errors: BTreeMap<String, String>,
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
    /// Empty for a managed CDP-free native window. Native observation and input
    /// use Remote View; DOM actions and debugging attachment are unavailable.
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

/// Explicit tab retention, independent of any link record. None is perpetual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserTabLifetime {
    pub inactivity_ms: Option<u64>,
    /// Optional inactivity threshold for measured quota pressure, never a link TTL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure_inactivity_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserTabEndReason {
    ExplicitClose,
    InactivityExpired,
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

/// Descriptive task context supplied at tab creation, never authenticated identity.
/// Missing attribution remains unknown rather than inferred from session labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserTaskMetadata {
    pub attribution: String,
    pub created_at_ms: Option<u64>,
    /// First exact-target human-use acknowledgement; never agent preparation.
    pub first_human_use_at_ms: Option<u64>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub human_name: Option<String>,
    pub origin_kind: Option<String>,
    pub task_name: Option<String>,
    pub purpose: Option<String>,
    pub request_id: Option<String>,
    pub job_id: Option<String>,
}

impl Default for BrowserTaskMetadata {
    fn default() -> Self {
        Self {
            attribution: "unknown".into(),
            created_at_ms: None,
            first_human_use_at_ms: None,
            service_name: None,
            agent_name: None,
            human_name: None,
            origin_kind: None,
            task_name: None,
            purpose: None,
            request_id: None,
            job_id: None,
        }
    }
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

/// A human handoff protects its actual tab until use or its configured deadline.
/// This hold is independent of the issuing link's lifetime and deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingHumanTabRetention {
    pub handoff_id: String,
    pub requested_at_ms: u64,
    pub expires_at_ms: Option<u64>,
}

/// Immutable provenance returned by qualified existing-profile adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserAdoptedProcessIdentity {
    pub profile_id: String,
    pub cdp_endpoint: String,
    pub process: crate::RecordedProcessIdentity,
}

/// Durable original request identity for a task effect. Fingerprints remain private;
/// public projections expose only exact identity, state and bounded references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserTaskOperation {
    pub request_id: String,
    pub task_id: String,
    pub profile_id: String,
    pub browser_id: String,
    pub tab_id: String,
    pub target_id: String,
    pub process_pid: u32,
    /// Immutable original kernel identity, private to owner fencing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_identity: Option<crate::RecordedProcessIdentity>,
    pub control_epoch: u64,
    pub action: String,
    pub fingerprint: String,
    pub state: BrowserTaskOperationState,
    pub created_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub evidence_references: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserTaskOperationState {
    Pending,
    OwnerAcknowledged,
    Uncertain,
}

/// A completed local artifact operation, without its path or private contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserTaskArtifactReference {
    pub artifact_id: String,
    pub kind: String,
    pub handle: String,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserSessionState {
    /// Original task effects are never evicted to manufacture retry permission.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub task_operations: BTreeMap<String, BTreeMap<String, BrowserTaskOperation>>,

    /// References recorded only after successful task artifact capture/download.
    /// Absence in older state means not recorded, never no artifacts existed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub task_artifact_references: BTreeMap<String, Vec<BrowserTaskArtifactReference>>,
    /// Profile-wide owner operations retain an actionable journal identity.
    /// A competing client must resume or cancel that operation before opening.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profile_operation_ids: BTreeMap<String, String>,
    /// App continuity is separate from browser control and account readiness.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hosted_continuity: BTreeMap<String, crate::BrowserHostedContinuity>,
    #[serde(default)]
    pub host_policies: crate::BrowserHostPolicies,
    /// App-provided processes and initial targets never transfer lifecycle custody.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub app_browser_attachments: BTreeMap<String, crate::BrowserAppAttachment>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attached_task_targets: BTreeMap<String, String>,
    /// App context retained under the logical task session identifier.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub host_bindings: BTreeMap<String, crate::BrowserHostTaskBinding>,
    pub schema_version: String,
    pub retention_policy: crate::BrowserRetentionPolicy,
    /// Last native placement, advanced only by a successful independent launch.
    pub last_remote_view_desktop_id: Option<String>,
    pub next_session_sequence: u64,
    pub next_disposable_sequence: u64,
    pub browsers: BTreeMap<String, ManagedBrowserInstance>,
    /// Attached shared browsers retain their external lifecycle owner across restart.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub shared_browser_ids: BTreeSet<String>,
    /// Only identities returned by qualified existing-profile adoption. No lifecycle custody transfer.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub adopted_browser_process_identities: BTreeMap<String, BrowserAdoptedProcessIdentity>,
    /// Explicit profile build choices survive other clients and owner restarts.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub profile_executables: BTreeMap<String, String>,
    /// Idle-closed physical browsers retain their last owner identity only for
    /// fenced recovery. Logical sessions, tabs and handoffs remain resolvable.
    pub idle_closed_browsers: BTreeMap<String, u64>,
    pub remote_view_presentations: BTreeMap<String, RemoteViewPresentationRetention>,
    /// Logical tab bindings survive provider route and desktop changes.
    pub remote_view_tab_handoffs: BTreeMap<String, crate::RemoteViewTabHandoff>,
    pub managed_tab_requests: BTreeMap<String, ManagedBrowserTabRequest>,
    pub sessions: BTreeMap<String, ManagedBrowserSession>,
    /// Explicit leave-open keeps the task and its physical browser occupied
    /// until an explicit close clears the preference, including across restart.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub leave_open_session_ids: BTreeSet<String>,
    pub disposable_profiles: BTreeMap<String, ManagedDisposableProfile>,
    pub tabs: BTreeMap<String, ManagedBrowserTab>,
    #[serde(default)]
    pub tab_lifetimes: BTreeMap<String, BrowserTabLifetime>,
    /// Original supplied context belongs to the tab, independently of its links.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tab_metadata: BTreeMap<String, BrowserTaskMetadata>,
    #[serde(default)]
    pub pending_human_tabs: BTreeMap<String, PendingHumanTabRetention>,
    pub tab_history: Vec<TerminalBrowserTab>,
    pub navigation_history: Vec<BrowserNavigationRecord>,
    pub navigation_daily_summaries: Vec<BrowserNavigationDailySummary>,
    pub history_compaction_events: Vec<BrowserHistoryCompactionEvent>,
    pub session_history: Vec<TerminalBrowserSession>,
}

impl Default for BrowserSessionState {
    fn default() -> Self {
        Self {
            task_artifact_references: BTreeMap::new(),
            task_operations: BTreeMap::new(),
            profile_operation_ids: BTreeMap::new(),
            hosted_continuity: BTreeMap::new(),
            host_policies: crate::BrowserHostPolicies::default(),
            schema_version: BROWSER_SESSION_STATE_SCHEMA_V1.to_string(),
            retention_policy: crate::BrowserRetentionPolicy::default(),
            last_remote_view_desktop_id: None,
            next_session_sequence: 0,
            next_disposable_sequence: 0,
            browsers: BTreeMap::new(),
            shared_browser_ids: BTreeSet::new(),
            adopted_browser_process_identities: BTreeMap::new(),
            profile_executables: BTreeMap::new(),
            host_bindings: BTreeMap::new(),
            app_browser_attachments: BTreeMap::new(),
            attached_task_targets: BTreeMap::new(),
            idle_closed_browsers: BTreeMap::new(),
            remote_view_presentations: BTreeMap::new(),
            remote_view_tab_handoffs: BTreeMap::new(),
            managed_tab_requests: BTreeMap::new(),
            sessions: BTreeMap::new(),
            leave_open_session_ids: BTreeSet::new(),
            disposable_profiles: BTreeMap::new(),
            tabs: BTreeMap::new(),
            tab_lifetimes: BTreeMap::new(),
            tab_metadata: BTreeMap::new(),
            pending_human_tabs: BTreeMap::new(),
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
    externally_protected_session_ids: BTreeSet<String>,
}

impl<'a, E: BrowserSessionEffects> BrowserSessionManager<'a, E> {
    pub fn new(
        state: &'a mut BrowserSessionState,
        catalog: &'a BrowserProfileCatalog,
        effects: &'a mut E,
        config: BrowserSessionManagerConfig,
    ) -> Self {
        // A valid handoff retains logical identity, not a permanent process.
        // External operation custody and explicit leave-open separately prevent physical idle closure.
        // Historical, closed and conflicting bindings confer no retention.
        let mut protected_session_ids: BTreeSet<String> = state
            .remote_view_tab_handoffs
            .keys()
            .filter_map(|id| crate::resolve_remote_view_tab_handoff(state, id).ok())
            .map(|target| target.session.id)
            .collect();
        let mut leave_open_session_ids = state
            .leave_open_session_ids
            .iter()
            .filter(|id| state.sessions.contains_key(*id))
            .cloned()
            .collect::<BTreeSet<_>>();
        leave_open_session_ids.extend(
            state
                .sessions
                .values()
                .filter(|task| state.profile_operation_ids.contains_key(&task.profile_id))
                .map(|task| task.id.clone()),
        );
        protected_session_ids.extend(leave_open_session_ids.iter().cloned());
        Self {
            state,
            catalog,
            effects,
            config,
            protected_session_ids,
            externally_protected_session_ids: leave_open_session_ids,
        }
    }

    /// Add sessions referenced by current external authority or a pending
    /// operation to the retained-handoff protection for this manager action.
    pub fn with_protected_sessions(mut self, session_ids: BTreeSet<String>) -> Self {
        self.protected_session_ids.extend(session_ids.clone());
        self.externally_protected_session_ids.extend(session_ids);
        self
    }

    /// Intersect all live task constraints sharing a profile. Caller omission
    /// cannot erase retained app intent; ended task metadata confers no veto.
    fn effective_placement(
        &self,
        profile_id: &str,
        requested: Option<&BrowserPlacement>,
    ) -> Result<Option<BrowserPlacement>, String> {
        let mut constraints = requested.into_iter().collect::<Vec<_>>();
        constraints.extend(
            self.state
                .sessions
                .values()
                .filter(|session| session.profile_id == profile_id)
                .filter_map(|session| self.state.host_bindings.get(&session.id))
                .map(|binding| &binding.request.placement),
        );
        let Some(first) = constraints.first() else {
            return Ok(None);
        };
        let mut allowed = first
            .bindings()
            .into_iter()
            .filter(|binding| {
                constraints
                    .iter()
                    .all(|constraint| constraint.bindings().contains(binding))
            })
            .cloned()
            .collect::<Vec<_>>();
        if allowed.is_empty() {
            return Err("browser_placement_conflict:shared tasks require incompatible desktops; explicitly rebind before changing placement".into());
        }
        let desktop = allowed.remove(0);
        Ok(Some(if allowed.is_empty() {
            BrowserPlacement::Required { desktop }
        } else {
            BrowserPlacement::Preferred {
                desktop,
                alternatives: allowed,
            }
        }))
    }

    pub fn open(
        &mut self,
        mut request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        let expires_at_ms = request
            .activity_at_ms
            .checked_add(self.config.session_idle_timeout_ms)
            .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?;
        let profile = self.resolve_profile_for_open(&request)?;
        if let Some(operation) = self.state.profile_operation_ids.get(&profile.id) {
            return Err(format!("browser_profile_operation_pending:{operation}:inspect or resume the owner operation"));
        }
        if self
            .state
            .app_browser_attachments
            .values()
            .any(|row| row.profile_id == profile.id)
        {
            let session = self.state.sessions.values().find(|row| row.name == request.session_name && row.profile_id == profile.id)
                .cloned().ok_or("browser_attachment_sharing_required: use explicit app attachment for another task")?;
            let browser = self
                .state
                .browsers
                .get(&session.browser_id)
                .ok_or("browser_session_browser_missing")?;
            if request
                .placement
                .as_ref()
                .is_some_and(|placement| !placement.accepts(browser.desktop.as_ref()))
            {
                return Err("browser_attachment_placement_conflict: retain current placement or explicitly relocate".into());
            }
            if !self.effects.browser_is_live(browser)? {
                return Err("browser_attachment_lost: app-owned browsers are report-only; no automatic replacement".into());
            }
            let stored = self
                .state
                .sessions
                .get_mut(&session.id)
                .expect("selected task");
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
        request.placement = self.effective_placement(&profile.id, request.placement.as_ref())?;
        if let Some(placement) = &request.placement {
            // Check retained physical identity before expiry, recovery, or task mutation.
            for browser in self
                .state
                .browsers
                .values()
                .filter(|browser| browser.profile_id == profile.id)
            {
                if !placement.accepts(browser.desktop.as_ref()) {
                    return Err("browser_placement_conflict:existing profile browser uses another desktop; retain it or explicitly relocate/rebind".into());
                }
            }
            let refreshed = self.effects.remote_view_placement_candidates(placement)?;
            placement.select(
                refreshed
                    .as_deref()
                    .unwrap_or(&self.config.remote_view_desktops),
            )?;
        }

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
            let browser = self
                .state
                .sessions
                .get(&session_id)
                .and_then(|session| self.state.browsers.get(&session.browser_id))
                .cloned()
                .ok_or("browser_session_browser_missing")?;
            let _retirement_guard = self
                .effects
                .acquire_idle_retirement_guard(&[browser])?
                .ok_or("browser_session_expired_epoch_activity_protected")?;
            self.close_session(
                &session_id,
                SessionEndReason::HeartbeatExpired,
                request.activity_at_ms,
            )?;
        }
        // A replacement must not reuse logical identity while native exclusion
        // still belongs to the old operation. The host publishes once and resumes
        // this already-selected profile, including a newly allocated disposable.
        if !self.effects.pending_idle_retirement_ids().is_empty() {
            return Err(format!(
                "browser_session_retirement_publication_required:{}",
                profile.id
            ));
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
            if self.effects.browser_is_live(&browser)?
                || self.reconcile_existing_browser_with_placement(
                    &browser,
                    &profile,
                    request.placement.as_ref(),
                )?
            {
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
                self.recover_retained_browser_with_placement(
                    &browser.id,
                    request.placement.as_ref(),
                )?;
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
            if self.effects.browser_is_live(&browser)?
                || self.reconcile_existing_browser_with_placement(
                    &browser,
                    &profile,
                    request.placement.as_ref(),
                )?
            {
                (browser.id, SessionBrowserDisposition::Reused, None)
            } else if browser
                .active_session_ids
                .iter()
                .any(|id| self.protected_session_ids.contains(id))
            {
                self.recover_retained_browser_with_placement(
                    &browser.id,
                    request.placement.as_ref(),
                )?;
                (browser.id, SessionBrowserDisposition::Launched, None)
            } else {
                self.retire_browser(
                    &browser,
                    SessionEndReason::BrowserUnresponsive,
                    request.activity_at_ms,
                )?;
                let launch = self.launch_browser(&profile, request.placement.as_ref())?;
                (
                    launch.browser_id.clone(),
                    SessionBrowserDisposition::Launched,
                    Some(launch),
                )
            }
        } else {
            let launch = self.launch_browser(&profile, request.placement.as_ref())?;
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
            if self.state.shared_browser_ids.contains(&browser_id) {
                if let Some(identity) = launch.process_identity.as_ref().filter(|identity| {
                    identity.pid == launch.pid
                        && !identity.start_token.is_empty()
                        && identity
                            .executable_path
                            .as_deref()
                            .is_some_and(|path| !path.is_empty())
                }) {
                    self.state.adopted_browser_process_identities.insert(
                        browser_id.clone(),
                        BrowserAdoptedProcessIdentity {
                            profile_id: profile.id.clone(),
                            cdp_endpoint: launch.cdp_endpoint.clone(),
                            process: identity.clone(),
                        },
                    );
                }
            }
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
            .ok_or("browser_session_recovery_browser_missing")?;
        let placement = self.effective_placement(&browser.profile_id, None)?;
        if placement
            .as_ref()
            .is_some_and(|placement| !placement.accepts(browser.desktop.as_ref()))
        {
            return Err("browser_placement_conflict:retained desktop differs from app placement; explicitly rebind before recovery".into());
        }
        self.recover_retained_browser_with_placement(browser_id, placement.as_ref())
    }

    fn recover_retained_browser_with_placement(
        &mut self,
        browser_id: &str,
        placement: Option<&BrowserPlacement>,
    ) -> Result<(), String> {
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .cloned()
            .ok_or("browser_session_recovery_browser_missing")?;
        if self.effects.browser_is_live(&browser)? {
            return Ok(());
        }
        if self.state.app_browser_attachments.contains_key(&browser.id) {
            return Err("browser_attachment_lost: preserve the app browser and request explicit rebind; no automatic recovery".into());
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
        if self.reconcile_existing_browser_with_placement(&browser, &profile, placement)? {
            return Ok(());
        }
        if !browser
            .active_session_ids
            .iter()
            .any(|id| self.protected_session_ids.contains(id))
        {
            return Err("browser_session_recovery_not_retained".into());
        }
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser.id)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(path) = self.state.profile_executables.get(&browser.profile_id) {
            self.effects
                .select_profile_browser_executable(profile.id.clone(), path.clone())?;
        }
        let recovered = self.effects.recover_browser(
            &browser,
            &profile,
            &tabs,
            &self.state.navigation_history,
        )?;
        if placement.is_some_and(|placement| !placement.accepts(recovered.launch.desktop.as_ref()))
        {
            return Err("browser_placement_conflict:recovery selected another desktop; explicitly relocate/rebind".into());
        }
        let browser_id = browser.id.clone();
        self.publish_recovery(browser, tabs, recovered, false, false)?;
        // A positively absent replacement has fresh launch custody; prior shared attachment does not transfer.
        self.state.shared_browser_ids.remove(&browser_id);
        self.state
            .adopted_browser_process_identities
            .remove(&browser_id);
        Ok(())
    }

    fn reconcile_existing_browser_with_placement(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        placement: Option<&BrowserPlacement>,
    ) -> Result<bool, String> {
        if self.state.app_browser_attachments.contains_key(&browser.id) {
            return Err(
                "browser_attachment_lost: no implicit recovery or borrowed target recreation"
                    .into(),
            );
        }
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser.id)
            .cloned()
            .collect::<Vec<_>>();
        let Some(recovered) = self.effects.reconcile_profile_browser(
            profile,
            Some(browser),
            &tabs,
            &self.state.navigation_history,
        )?
        else {
            return Ok(false);
        };
        if placement.is_some_and(|placement| !placement.accepts(recovered.launch.desktop.as_ref()))
        {
            return Err("browser_placement_conflict:reconciled browser uses another desktop; explicitly relocate/rebind".into());
        }
        // Reattaching our same process or completing its observed launch keeps
        // its managed lifecycle. A different externally discovered process is
        // borrowed and must survive final task closure.
        let managed = !self.state.shared_browser_ids.contains(&browser.id)
            && (browser.pid == recovered.launch.pid
                || self
                    .effects
                    .pending_launch_intent()
                    .is_some_and(|intent| intent.profile_id == profile.id));
        let adopted_launch = recovered.launch.clone();
        self.publish_recovery(browser.clone(), tabs, recovered, true, false)?;
        if !managed {
            self.state.shared_browser_ids.insert(browser.id.clone());
            self.state
                .adopted_browser_process_identities
                .remove(&browser.id);
            if let Some(identity) = adopted_launch.process_identity.filter(|identity| {
                identity.pid == adopted_launch.pid
                    && !identity.start_token.is_empty()
                    && identity
                        .executable_path
                        .as_deref()
                        .is_some_and(|path| !path.is_empty())
            }) {
                self.state.adopted_browser_process_identities.insert(
                    browser.id.clone(),
                    BrowserAdoptedProcessIdentity {
                        profile_id: browser.profile_id.clone(),
                        cdp_endpoint: adopted_launch.cdp_endpoint,
                        process: identity,
                    },
                );
            }
        }
        Ok(true)
    }

    /// Publish a qualified owner-directed physical replacement. The lifecycle
    /// adapter owns exact exit and launch proof; this seam retains logical IDs.
    /// The operation fence remains until its durable journal is terminal.
    pub fn publish_owner_replacement(
        &mut self,
        browser_id: &str,
        operation_id: &str,
        recovered: BrowserRecovery,
    ) -> Result<(), String> {
        let browser = self
            .state
            .browsers
            .get(browser_id)
            .cloned()
            .ok_or("browser_owner_replacement_browser_missing")?;
        if self
            .state
            .profile_operation_ids
            .get(&browser.profile_id)
            .map(String::as_str)
            != Some(operation_id)
            || recovered
                .launch
                .process_identity
                .as_ref()
                .is_none_or(|identity| {
                    identity.pid != recovered.launch.pid
                        || identity.start_token.is_empty()
                        || identity
                            .executable_path
                            .as_deref()
                            .is_none_or(str::is_empty)
                })
        {
            return Err("browser_owner_replacement_identity_unverified".into());
        }
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.browser_id == browser.id)
            .cloned()
            .collect();
        self.publish_recovery(browser, tabs, recovered, false, true)?;
        self.state.app_browser_attachments.remove(browser_id);
        self.state.shared_browser_ids.remove(browser_id);
        self.state
            .adopted_browser_process_identities
            .remove(browser_id);
        Ok(())
    }

    fn publish_recovery(
        &mut self,
        browser: ManagedBrowserInstance,
        tabs: Vec<ManagedBrowserTab>,
        recovered: BrowserRecovery,
        current_desktop_rebind: bool,
        owner_replacement: bool,
    ) -> Result<(), String> {
        let targets = recovered.target_ids.values().collect::<BTreeSet<_>>();
        if recovered.launch.browser_id != browser.id
            || recovered.launch.pid == 0
            || (recovered.launch.cdp_endpoint.is_empty() && !owner_replacement)
            || (!current_desktop_rebind
                && !self.state.shared_browser_ids.contains(&browser.id)
                && browser.desktop.is_some()
                && recovered.launch.desktop != browser.desktop
                && !self.state.idle_closed_browsers.contains_key(&browser.id))
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
        if self.state.shared_browser_ids.contains(&browser.id) && !owner_replacement {
            self.state
                .adopted_browser_process_identities
                .remove(&browser.id);
            if let Some(identity) = recovered
                .launch
                .process_identity
                .as_ref()
                .filter(|identity| {
                    identity.pid == recovered.launch.pid
                        && !identity.start_token.is_empty()
                        && identity
                            .executable_path
                            .as_deref()
                            .is_some_and(|path| !path.is_empty())
                })
            {
                self.state.adopted_browser_process_identities.insert(
                    browser.id.clone(),
                    BrowserAdoptedProcessIdentity {
                        profile_id: browser.profile_id.clone(),
                        cdp_endpoint: recovered.launch.cdp_endpoint.clone(),
                        process: identity.clone(),
                    },
                );
            }
        } else {
            self.state
                .adopted_browser_process_identities
                .remove(&browser.id);
        }
        let mut current = browser;
        let relocated = current.desktop != recovered.launch.desktop;
        current.desktop = recovered.launch.desktop;
        current.pid = recovered.launch.pid;
        current.cdp_endpoint = recovered.launch.cdp_endpoint;
        if let Some(desktop) = &current.desktop {
            for session in self
                .state
                .sessions
                .values()
                .filter(|session| session.browser_id == current.id)
            {
                if let Some(binding) = self.state.host_bindings.get_mut(&session.id) {
                    binding.desktop = desktop.clone();
                }
            }
        }

        self.state.idle_closed_browsers.remove(&current.id);
        self.state.browsers.insert(current.id.clone(), current);
        if relocated {
            // The logical URL survives placement changes. Its previous provider
            // grant was retired with the old assignment and cannot be reused.
            for handoff in self.state.remote_view_tab_handoffs.values_mut() {
                if tabs.iter().any(|tab| tab.id == handoff.tab_id) {
                    handoff.view = None;
                }
            }
        }
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
        placement: Option<&BrowserPlacement>,
    ) -> Result<BrowserLaunch, String> {
        if let Some(path) = self.state.profile_executables.get(&profile.id) {
            self.effects
                .select_profile_browser_executable(profile.id.clone(), path.clone())?;
        }
        if let Some(reconciled) = self
            .effects
            .reconcile_profile_browser(profile, None, &[], &[])?
        {
            let pending = self.effects.pending_launch_intent();
            let valid_posture = if profile.manual_login_url.is_some() {
                reconciled.launch.cdp_endpoint.is_empty()
                    && pending
                        .as_ref()
                        .is_some_and(|intent| intent.profile_id == profile.id)
                    && reconciled
                        .launch
                        .process_identity
                        .as_ref()
                        .is_some_and(|identity| {
                            identity.pid == reconciled.launch.pid
                                && !identity.start_token.is_empty()
                                && identity
                                    .executable_path
                                    .as_deref()
                                    .is_some_and(|path| !path.is_empty())
                        })
            } else {
                !reconciled.launch.cdp_endpoint.is_empty()
            };
            if reconciled.launch.pid == 0 || !valid_posture || !reconciled.target_ids.is_empty() {
                return Err("browser_session_recovery_identity_conflict".into());
            }
            if placement
                .is_some_and(|placement| !placement.accepts(reconciled.launch.desktop.as_ref()))
            {
                return Err("browser_placement_conflict:reconciled browser uses another desktop; explicitly relocate/rebind".into());
            }
            // An interrupted observed owner launch retains its durable custody.
            // Only a current external attachment is borrowed/shared lifecycle.
            if pending.is_none() {
                self.state
                    .shared_browser_ids
                    .insert(reconciled.launch.browser_id.clone());
            }
            return Ok(reconciled.launch);
        }
        let refreshed = match placement {
            Some(placement) => self.effects.remote_view_placement_candidates(placement)?,
            None => self.effects.remote_view_desktop_candidates()?,
        };
        let candidates = refreshed
            .as_deref()
            .unwrap_or(&self.config.remote_view_desktops);
        let desktop = if let Some(placement) = placement {
            Some(placement.select(candidates)?)
        } else if candidates.is_empty() {
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
            if refreshed.is_some() {
                let ready = candidates
                    .iter()
                    .filter(|candidate| candidate.ready)
                    .collect::<Vec<_>>();
                if ready.is_empty() {
                    return Err("remote_view_desktop_unavailable".into());
                }
                // Provider order is stable. A missing prior desktop starts at
                // the first eligible member; retained profile reuse skips this.
                let index = self
                    .state
                    .last_remote_view_desktop_id
                    .as_ref()
                    .and_then(|last| {
                        ready
                            .iter()
                            .position(|candidate| &candidate.desktop.desktop_id == last)
                    })
                    .map_or(0, |index| (index + 1) % ready.len());
                Some(ready[index].desktop.clone())
            } else {
                Some(
                    select_least_crowded_remote_view_desktop(candidates, &live_desktop_ids)?
                        .desktop,
                )
            }
        };
        let mut launch = self.effects.launch_browser(profile, desktop.as_ref())?;
        if launch.desktop != desktop {
            return Err("browser_session_launch_desktop_mismatch".to_string());
        }
        if refreshed.is_some() {
            self.state.last_remote_view_desktop_id =
                desktop.as_ref().map(|desktop| desktop.desktop_id.clone());
        }
        launch.desktop = desktop;
        Ok(launch)
    }

    fn resolve_profile_for_open(
        &mut self,
        request: &OpenBrowserSession,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        match &request.profile_intent {
            BrowserProfileIntent::ExistingDisposable { profile_id } => self
                .state
                .disposable_profiles
                .get(profile_id)
                .map(|allocation| allocation.profile.clone())
                .ok_or_else(|| format!("browser_disposable_profile_not_found:{profile_id}")),
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

    /// Preserve the exact live browser and logical task without releasing
    /// occupied profile custody or closing any peer's tab. This is not an
    /// external ownership release. Explicit close remains the terminal action.
    pub fn preserve_session(
        &mut self,
        session_id: &str,
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
            .ok_or("browser_session_browser_missing")?;
        if self.state.idle_closed_browsers.contains_key(&browser.id)
            || !self.effects.browser_is_live(browser)?
        {
            return Err("browser_session_preserve_requires_live_browser".into());
        }
        self.state.leave_open_session_ids.insert(session.id.clone());
        Ok(CloseBrowserSessionResult {
            session_id: session.id,
            browser_id: session.browser_id,
            disposition: SessionCloseDisposition::BrowserPreserved,
        })
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
        if let Some(operation) = self.state.profile_operation_ids.get(&session.profile_id) {
            return Err(format!("browser_profile_operation_pending:{operation}:inspect or cancel the owner operation before closing"));
        }
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or_else(|| "browser_session_browser_missing".to_string())?;
        if let Some(allocation) = self.state.disposable_profiles.get(&session.profile_id) {
            ended_at_ms
                .checked_add(allocation.cleanup_delay_ms)
                .ok_or("browser_disposable_cleanup_expiry_exhausted")?;
        }
        let final_session =
            browser.active_session_ids.len() == 1 && browser.active_session_ids[0] == session.id;
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.session_id == session.id)
            .cloned()
            .collect::<Vec<_>>();
        let idle_closed = self.state.idle_closed_browsers.contains_key(&browser.id);
        if !idle_closed && !final_session && !tabs.is_empty() {
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
            // Automatic retirement must not manufacture replacement work for
            // an empty peer. Explicit session transfer retains its bootstrap.
            if !remaining_has_tab && reason == SessionEndReason::ExplicitClose {
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
        if !idle_closed && final_session && !self.state.shared_browser_ids.contains(&browser.id) {
            self.effects.close_browser(&browser)?;
        } else if !idle_closed {
            for tab in &tabs {
                if !self
                    .state
                    .app_browser_attachments
                    .get(&browser.id)
                    .is_some_and(|row| row.borrowed_tab_ids.contains(&tab.target_id))
                {
                    self.effects.close_tab(&browser, tab)?;
                }
            }
        }

        for link in self
            .state
            .remote_view_tab_handoffs
            .values_mut()
            .filter(|link| link.session_id == session_id)
        {
            link.target_expired = reason != SessionEndReason::ExplicitClose;
        }
        self.state.sessions.remove(session_id);
        self.state.leave_open_session_ids.remove(session_id);
        for tab in tabs {
            self.state.tab_lifetimes.remove(&tab.id);
            self.state.tab_metadata.remove(&tab.id);
            self.state.pending_human_tabs.remove(&tab.id);
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

        let disposition = if final_session && !self.state.shared_browser_ids.contains(&browser.id) {
            self.state.browsers.remove(&session.browser_id);
            self.state.idle_closed_browsers.remove(&session.browser_id);
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

    /// Admit live provider-observed child targets only through an already-owned
    /// opener in this same browser. Unknown targets and peer claims remain
    /// untouched. The CLI adapter supplies fresh, sanitized CDP observations;
    /// caller-provided target IDs are not an attribution proof.
    pub fn observe_child_tabs(
        &mut self,
        session_id: &str,
        observations: &[(String, String)],
        activity_at_ms: u64,
    ) -> Result<Vec<String>, String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or("browser_session_not_found")?;
        let mut owned = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.session_id == session_id && tab.browser_id == session.browser_id)
            .map(|tab| tab.target_id.clone())
            .collect::<BTreeSet<_>>();
        let mut admitted = Vec::new();
        // Bounded transitive opener chains allow a login popup to open its
        // receiving destination. An unattached extension tab needs a separate
        // producer attribution contract and is never guessed from its URL.
        for _ in 0..64 {
            let mut changed = false;
            for (target, opener) in observations {
                if target.is_empty()
                    || target == opener
                    || !owned.contains(opener)
                    || owned.contains(target)
                {
                    continue;
                }
                if self
                    .state
                    .app_browser_attachments
                    .get(&session.browser_id)
                    .is_some_and(|row| row.borrowed_tab_ids.contains(target))
                {
                    continue;
                }
                if self.state.tabs.contains_key(target)
                    || self.state.tabs.values().any(|tab| tab.target_id == *target)
                {
                    continue;
                }
                if admitted.len() >= 64 {
                    return Err("browser_session_popup_limit_exceeded".into());
                }
                owned.insert(target.clone());
                admitted.push(target.clone());
                changed = true;
            }
            if !changed {
                break;
            }
        }
        for target in &admitted {
            self.state.tabs.insert(
                target.clone(),
                ManagedBrowserTab {
                    id: target.clone(),
                    target_id: target.clone(),
                    browser_id: session.browser_id.clone(),
                    session_id: session.id.clone(),
                    created_at_ms: activity_at_ms,
                    last_activity_at_ms: activity_at_ms,
                },
            );
        }
        Ok(admitted)
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
        let profile = self.catalog.profiles.get(&session.profile_id).or_else(|| {
            self.state
                .disposable_profiles
                .get(&session.profile_id)
                .map(|p| &p.profile)
        });
        let acquisition = if let Some(profile) = profile {
            self.effects.acquire_session_initial_tab(
                &browser,
                profile,
                &session,
                &attributed_target_ids,
            )?
        } else {
            self.effects
                .acquire_initial_tab(&browser, &attributed_target_ids)?
        };
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

    /// Close the addressed current tab, preserving peers and handoff records.
    /// An owned browser with no live tabs ends its empty sessions and closes.
    pub fn close_current_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<CloseBrowserTabResult, String> {
        let tab_id = self
            .state
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?
            .current_tab_id
            .clone()
            .ok_or_else(|| "browser_session_current_tab_absent".to_string())?;
        self.close_exact_tab_with(
            session_id,
            &tab_id,
            activity_at_ms,
            |effects, browser, tab| effects.close_tab(browser, tab),
        )
    }

    /// Close the addressed task even when its session's current tab changed.
    /// The caller supplies its admitted effect at this seam; the manager owns
    /// target attribution, peer state and last-owned-browser lifecycle rules.
    pub fn close_exact_tab_with(
        &mut self,
        session_id: &str,
        tab_id: &str,
        activity_at_ms: u64,
        close_target: impl FnOnce(
            &mut E,
            &ManagedBrowserInstance,
            &ManagedBrowserTab,
        ) -> Result<(), String>,
    ) -> Result<CloseBrowserTabResult, String> {
        if self.state.tabs.get(tab_id).is_some_and(|tab| {
            self.state
                .app_browser_attachments
                .get(&tab.browser_id)
                .is_some_and(|row| row.borrowed_tab_ids.contains(&tab.target_id))
        }) {
            return Err(
                "browser_attachment_borrowed_tab_close_forbidden: detach the task instead".into(),
            );
        }
        let browser_id = self
            .state
            .tabs
            .get(tab_id)
            .ok_or_else(|| "browser_session_current_tab_missing".to_string())?
            .browser_id
            .clone();
        let closed = self.close_named_tab_with(
            session_id,
            tab_id,
            BrowserTabEndReason::ExplicitClose,
            activity_at_ms,
            close_target,
        )?;
        if !self
            .state
            .tabs
            .values()
            .any(|tab| tab.browser_id == browser_id)
            && !self.state.shared_browser_ids.contains(&browser_id)
        {
            let empty_sessions = self
                .state
                .browsers
                .get(&browser_id)
                .ok_or_else(|| "browser_session_browser_missing".to_string())?
                .active_session_ids
                .clone();
            for id in empty_sessions {
                self.close_session(&id, SessionEndReason::ExplicitClose, activity_at_ms)?;
            }
        }
        Ok(closed)
    }

    fn close_named_tab(
        &mut self,
        session_id: &str,
        tab_id: &str,
        reason: BrowserTabEndReason,
        activity_at_ms: u64,
    ) -> Result<CloseBrowserTabResult, String> {
        self.close_named_tab_with(
            session_id,
            tab_id,
            reason,
            activity_at_ms,
            |effects, browser, tab| effects.close_tab(browser, tab),
        )
    }

    fn close_named_tab_with(
        &mut self,
        session_id: &str,
        tab_id: &str,
        reason: BrowserTabEndReason,
        activity_at_ms: u64,
        close_target: impl FnOnce(
            &mut E,
            &ManagedBrowserInstance,
            &ManagedBrowserTab,
        ) -> Result<(), String>,
    ) -> Result<CloseBrowserTabResult, String> {
        let session = self
            .state
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("browser_session_not_found:{session_id}"))?;
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
        let expires_at_ms = if reason == BrowserTabEndReason::ExplicitClose {
            activity_at_ms
                .checked_add(self.config.session_idle_timeout_ms)
                .ok_or_else(|| "browser_session_expiry_exhausted".to_string())?
        } else {
            session.expires_at_ms
        };
        let borrowed = self
            .state
            .app_browser_attachments
            .get(&browser.id)
            .is_some_and(|row| row.borrowed_tab_ids.contains(&tab.target_id));
        if borrowed && reason == BrowserTabEndReason::ExplicitClose {
            return Err(
                "browser_attachment_borrowed_tab_close_forbidden: detach the task instead".into(),
            );
        }
        if !borrowed {
            close_target(self.effects, &browser, &tab)?;
        }
        self.state.tabs.remove(&tab.id);
        self.state.tab_lifetimes.remove(&tab.id);
        self.state.tab_metadata.remove(&tab.id);
        self.state.pending_human_tabs.remove(&tab.id);
        for link in self
            .state
            .remote_view_tab_handoffs
            .values_mut()
            .filter(|link| link.tab_id == tab.id)
        {
            link.target_expired = reason == BrowserTabEndReason::InactivityExpired;
        }
        self.state.tab_history.push(TerminalBrowserTab {
            id: tab.id.clone(),
            target_id: tab.target_id.clone(),
            browser_id: tab.browser_id.clone(),
            session_id: tab.session_id.clone(),
            profile_id: session.profile_id.clone(),
            created_at_ms: tab.created_at_ms,
            last_activity_at_ms: tab.last_activity_at_ms,
            closed_at_ms: activity_at_ms,
            reason,
            session_end_reason: None,
        });
        let remaining_tab_id = self
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
        let current_tab_id = session
            .current_tab_id
            .clone()
            .filter(|id| self.state.tabs.contains_key(id))
            .or(remaining_tab_id);
        let stored_session = self
            .state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "browser_session_missing_during_tab_close".to_string())?;
        stored_session.current_tab_id = current_tab_id.clone();
        if reason == BrowserTabEndReason::ExplicitClose {
            stored_session.last_activity_at_ms = activity_at_ms;
            stored_session.expires_at_ms = expires_at_ms;
        }
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

    fn tab_awaits_human_use(&self, tab_id: &str, now_ms: u64) -> bool {
        self.state
            .pending_human_tabs
            .get(tab_id)
            .is_some_and(|hold| hold.expires_at_ms.is_none_or(|deadline| now_ms < deadline))
    }

    fn tab_lifetime_expired(&self, tab: &ManagedBrowserTab, now_ms: u64) -> bool {
        if self.tab_awaits_human_use(&tab.id, now_ms) {
            return false;
        }
        self.state
            .tab_lifetimes
            .get(&tab.id)
            .and_then(|lifetime| lifetime.inactivity_ms)
            .is_some_and(|ttl| now_ms.saturating_sub(tab.last_activity_at_ms) >= ttl)
    }

    fn session_has_retained_tab_lifetime(&self, session_id: &str, now_ms: u64) -> bool {
        self.state.tabs.values().any(|tab| {
            tab.session_id == session_id
                && (self.tab_awaits_human_use(&tab.id, now_ms)
                    || (self.state.tab_lifetimes.contains_key(&tab.id)
                        && !self.tab_lifetime_expired(tab, now_ms)))
        })
    }

    fn defer_borrowed_retirement_error(
        &mut self,
        browser: &ManagedBrowserInstance,
        failed_session_id: &str,
        error: String,
        result: &mut ReapBrowserSessionsResult,
        protected: &mut BTreeSet<String>,
        externally_protected: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if !self.state.shared_browser_ids.contains(&browser.id) || browser.desktop.is_some() {
            return Err(error);
        }
        result
            .deferred_browser_errors
            .entry(browser.id.clone())
            .or_insert(error);
        protected.insert(failed_session_id.to_owned());
        externally_protected.insert(failed_session_id.to_owned());
        self.externally_protected_session_ids
            .insert(failed_session_id.to_owned());
        Ok(())
    }

    fn reaper_retirement_guard(
        &mut self,
        browser: &ManagedBrowserInstance,
        result: &mut ReapBrowserSessionsResult,
        protected: &mut BTreeSet<String>,
        externally_protected: &mut BTreeSet<String>,
    ) -> Option<Box<dyn BrowserIdleRetirementGuard>> {
        match self
            .effects
            .acquire_idle_retirement_guard(std::slice::from_ref(browser))
        {
            Ok(Some(guard)) => Some(guard),
            outcome => {
                if let Err(error) = outcome {
                    result
                        .deferred_browser_errors
                        .insert(browser.id.clone(), error);
                }
                protected.extend(browser.active_session_ids.iter().cloned());
                externally_protected.extend(browser.active_session_ids.iter().cloned());
                self.externally_protected_session_ids
                    .extend(browser.active_session_ids.iter().cloned());
                None
            }
        }
    }

    /// A host may publish this phase before unrelated Native or disposable
    /// maintenance. Protected scopes grant no effect or observation authority.
    pub fn reap_borrowed_task_tabs(
        &mut self,
        now_ms: u64,
    ) -> Result<ReapBrowserSessionsResult, String> {
        let protected = self
            .state
            .sessions
            .values()
            .filter(|session| {
                self.state
                    .browsers
                    .get(&session.browser_id)
                    .is_none_or(|browser| {
                        !self.state.shared_browser_ids.contains(&browser.id)
                            || browser.desktop.is_some()
                    })
            })
            .map(|session| session.id.clone())
            .collect();
        self.reap_task_scopes(now_ms, &protected, false)
    }

    pub fn reap_with_protected_sessions(
        &mut self,
        now_ms: u64,
        protected_session_ids: &BTreeSet<String>,
    ) -> Result<ReapBrowserSessionsResult, String> {
        self.reap_task_scopes(now_ms, protected_session_ids, true)
    }

    fn reap_task_scopes(
        &mut self,
        now_ms: u64,
        protected_session_ids: &BTreeSet<String>,
        disposable_maintenance: bool,
    ) -> Result<ReapBrowserSessionsResult, String> {
        let mut result = ReapBrowserSessionsResult::default();
        let mut externally_protected_session_ids = self
            .externally_protected_session_ids
            .union(protected_session_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut protected_session_ids = self
            .protected_session_ids
            .union(protected_session_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        // Qualify viewer protection before either logical expiry or physical
        // idle close. It also protects disposable sessions from quota eviction.
        let candidates = self
            .state
            .browsers
            .values()
            .filter(|browser| {
                !self.state.idle_closed_browsers.contains_key(&browser.id)
                    && browser
                        .active_session_ids
                        .iter()
                        .any(|id| !externally_protected_session_ids.contains(id))
                    && browser.active_session_ids.iter().any(|id| {
                        self.state.sessions.get(id).is_some_and(|session| {
                            (!self.state.shared_browser_ids.contains(&browser.id)
                                && (session.expires_at_ms <= now_ms
                                    || (self
                                        .state
                                        .disposable_profiles
                                        .contains_key(&session.profile_id)
                                        && now_ms.saturating_sub(session.last_activity_at_ms)
                                            >= self
                                                .state
                                                .retention_policy
                                                .disposable_inactivity_ms)))
                                || self.state.tabs.values().any(|tab| {
                                    tab.session_id == session.id
                                        && self.tab_lifetime_expired(tab, now_ms)
                                })
                        })
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        for browser in candidates {
            let permitted = match self.effects.permits_idle_browser_close(&browser) {
                Ok(permitted) => permitted,
                Err(error) => {
                    result
                        .deferred_browser_errors
                        .insert(browser.id.clone(), error);
                    false
                }
            };
            if !permitted {
                externally_protected_session_ids.extend(browser.active_session_ids.iter().cloned());
                protected_session_ids.extend(browser.active_session_ids.iter().cloned());
            }
        }
        for session in self.state.sessions.values() {
            if self.session_has_retained_tab_lifetime(&session.id, now_ms) {
                protected_session_ids.insert(session.id.clone());
                continue;
            }
            if self
                .state
                .disposable_profiles
                .contains_key(&session.profile_id)
            {
                let expired = now_ms.saturating_sub(session.last_activity_at_ms)
                    >= self.state.retention_policy.disposable_inactivity_ms;
                if expired && !externally_protected_session_ids.contains(&session.id) {
                    protected_session_ids.remove(&session.id);
                }
            }
        }
        let expired_tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| {
                self.tab_lifetime_expired(tab, now_ms)
                    && !externally_protected_session_ids.contains(&tab.session_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        let expired_tab_browser_ids = expired_tabs
            .iter()
            .map(|tab| tab.browser_id.clone())
            .collect::<BTreeSet<_>>();
        for tab in expired_tabs {
            if externally_protected_session_ids.contains(&tab.session_id) {
                continue;
            }
            let browser = self
                .state
                .browsers
                .get(&tab.browser_id)
                .cloned()
                .ok_or("browser_session_browser_missing")?;
            let Some(_retirement_guard) = self.reaper_retirement_guard(
                &browser,
                &mut result,
                &mut protected_session_ids,
                &mut externally_protected_session_ids,
            ) else {
                continue;
            };
            let last_session_tab = self
                .state
                .tabs
                .values()
                .filter(|candidate| candidate.session_id == tab.session_id)
                .count()
                == 1;
            if last_session_tab {
                let closed = match self.close_session(
                    &tab.session_id,
                    SessionEndReason::HeartbeatExpired,
                    now_ms,
                ) {
                    Ok(closed) => closed,
                    Err(error) => {
                        self.defer_borrowed_retirement_error(
                            &browser,
                            &tab.session_id,
                            error,
                            &mut result,
                            &mut protected_session_ids,
                            &mut externally_protected_session_ids,
                        )?;
                        continue;
                    }
                };
                result.expired_session_ids.push(tab.session_id.clone());
                if closed.disposition == SessionCloseDisposition::BrowserClosed {
                    result.closed_browser_ids.push(closed.browser_id);
                }
            } else {
                if let Err(error) = self.close_named_tab(
                    &tab.session_id,
                    &tab.id,
                    BrowserTabEndReason::InactivityExpired,
                    now_ms,
                ) {
                    self.defer_borrowed_retirement_error(
                        &browser,
                        &tab.session_id,
                        error,
                        &mut result,
                        &mut protected_session_ids,
                        &mut externally_protected_session_ids,
                    )?;
                    continue;
                }
            }
            result.expired_tab_ids.push(tab.id);
        }
        // A browser whose last tab expired has no remaining work merely
        // because an empty peer session has a later heartbeat deadline.
        let empty_peer_session_ids = self
            .state
            .sessions
            .values()
            .filter(|session| {
                expired_tab_browser_ids.contains(&session.browser_id)
                    && !self.state.shared_browser_ids.contains(&session.browser_id)
                    && !self
                        .state
                        .tabs
                        .values()
                        .any(|tab| tab.browser_id == session.browser_id)
                    && self
                        .state
                        .browsers
                        .get(&session.browser_id)
                        .is_some_and(|browser| {
                            browser
                                .active_session_ids
                                .iter()
                                .all(|id| !externally_protected_session_ids.contains(id))
                        })
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        for session_id in empty_peer_session_ids {
            if externally_protected_session_ids.contains(&session_id) {
                continue;
            }
            let browser = self
                .state
                .sessions
                .get(&session_id)
                .and_then(|session| self.state.browsers.get(&session.browser_id))
                .cloned()
                .ok_or("browser_session_browser_missing")?;
            let Some(_retirement_guard) = self.reaper_retirement_guard(
                &browser,
                &mut result,
                &mut protected_session_ids,
                &mut externally_protected_session_ids,
            ) else {
                continue;
            };
            let closed =
                self.close_session(&session_id, SessionEndReason::HeartbeatExpired, now_ms)?;
            result.expired_session_ids.push(session_id);
            if closed.disposition == SessionCloseDisposition::BrowserClosed {
                result.closed_browser_ids.push(closed.browser_id);
            }
        }
        let expired_session_ids = self
            .state
            .sessions
            .values()
            .filter(|session| {
                let expired = if self
                    .state
                    .disposable_profiles
                    .contains_key(&session.profile_id)
                {
                    now_ms.saturating_sub(session.last_activity_at_ms)
                        >= self.state.retention_policy.disposable_inactivity_ms
                } else {
                    session.expires_at_ms <= now_ms
                };
                expired && !protected_session_ids.contains(&session.id)
            })
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        for session_id in expired_session_ids {
            if protected_session_ids.contains(&session_id) {
                continue;
            }
            let browser = self
                .state
                .sessions
                .get(&session_id)
                .and_then(|session| self.state.browsers.get(&session.browser_id))
                .cloned()
                .ok_or("browser_session_browser_missing")?;
            let Some(_retirement_guard) = self.reaper_retirement_guard(
                &browser,
                &mut result,
                &mut protected_session_ids,
                &mut externally_protected_session_ids,
            ) else {
                continue;
            };
            let closed =
                match self.close_session(&session_id, SessionEndReason::HeartbeatExpired, now_ms) {
                    Ok(closed) => closed,
                    Err(error) => {
                        self.defer_borrowed_retirement_error(
                            &browser,
                            &session_id,
                            error,
                            &mut result,
                            &mut protected_session_ids,
                            &mut externally_protected_session_ids,
                        )?;
                        continue;
                    }
                };
            result.expired_session_ids.push(session_id);
            if closed.disposition == SessionCloseDisposition::BrowserClosed {
                result.closed_browser_ids.push(closed.browser_id);
            }
        }
        // Retained logical sessions can outlive the physical browser. Close
        // once when every peer is idle and no current external operation pins
        // the browser. Preserve the last PID for exact fenced recovery, not as
        // proof of current occupancy.
        let idle_browsers = self
            .state
            .browsers
            .values()
            .filter(|browser| {
                !self.state.idle_closed_browsers.contains_key(&browser.id)
                    && !self.state.shared_browser_ids.contains(&browser.id)
                    && !browser.active_session_ids.is_empty()
                    && browser.active_session_ids.iter().all(|id| {
                        self.state
                            .sessions
                            .get(id)
                            .is_some_and(|session| session.expires_at_ms <= now_ms)
                            && !self.session_has_retained_tab_lifetime(id, now_ms)
                            && !externally_protected_session_ids.contains(id)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        for browser in idle_browsers {
            let Some(_retirement_guard) = self.reaper_retirement_guard(
                &browser,
                &mut result,
                &mut protected_session_ids,
                &mut externally_protected_session_ids,
            ) else {
                continue;
            };
            self.effects.close_browser(&browser)?;
            self.state
                .idle_closed_browsers
                .insert(browser.id.clone(), now_ms);
            result.closed_browser_ids.push(browser.id);
        }
        if !disposable_maintenance {
            return Ok(result);
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
            let Some(_cleanup_guard) = self.effects.acquire_profile_cleanup_guard(&allocation)?
            else {
                continue;
            };
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

    /// A pressure policy never overrides pending human use or peer eligibility.
    fn session_pressure_retirement_eligible(&self, session_id: &str, now_ms: u64) -> bool {
        let tabs = self
            .state
            .tabs
            .values()
            .filter(|tab| tab.session_id == session_id)
            .collect::<Vec<_>>();
        !tabs.is_empty()
            && tabs.iter().all(|tab| {
                let held = self
                    .state
                    .pending_human_tabs
                    .get(&tab.id)
                    .is_some_and(|hold| {
                        hold.expires_at_ms.is_none_or(|deadline| now_ms < deadline)
                    });
                !held
                    && self
                        .state
                        .tab_lifetimes
                        .get(&tab.id)
                        .and_then(|lifetime| lifetime.pressure_inactivity_ms)
                        .is_some_and(|duration| {
                            duration > 0
                                && now_ms.saturating_sub(tab.last_activity_at_ms) >= duration
                        })
            })
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
            let pressure_eligible = !sessions.is_empty()
                && sessions
                    .iter()
                    .all(|session| self.session_pressure_retirement_eligible(&session.id, now_ms));
            let externally_protected = sessions
                .iter()
                .any(|session| self.externally_protected_session_ids.contains(&session.id));
            let has_live_tabs = self
                .state
                .tabs
                .values()
                .any(|tab| sessions.iter().any(|session| session.id == tab.session_id));
            let protected_session = externally_protected
                || (has_live_tabs && !pressure_eligible)
                || (!pressure_eligible
                    && sessions
                        .iter()
                        .any(|session| protected_session_ids.contains(&session.id)));
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
            // Qualify current viewers/custody only when measured configured
            // pressure needs this candidate. Shared browsers are never retired.
            let browsers = self
                .state
                .browsers
                .values()
                .filter(|browser| browser.profile_id == profile_id)
                .cloned()
                .collect::<Vec<_>>();
            let mut permitted = true;
            for browser in browsers {
                if self.state.shared_browser_ids.contains(&browser.id)
                    || !self.effects.permits_idle_browser_close(&browser)?
                {
                    permitted = false;
                    break;
                }
            }
            if !permitted {
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

        // Qualify the complete pressure effect group before retiring any candidate.
        // One guard also avoids re-entering an adapter's retained authority lock.
        let browsers = self
            .state
            .browsers
            .values()
            .filter(|browser| selected.contains(&browser.profile_id))
            .cloned()
            .collect::<Vec<_>>();
        let allocations = selected
            .iter()
            .map(|profile_id| {
                self.state
                    .disposable_profiles
                    .get(profile_id)
                    .cloned()
                    .ok_or("browser_disposable_profile_missing_during_quota")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let retirement_guard = self
            .effects
            .acquire_disposable_retirement_guard(&allocations, &browsers)?
            .ok_or("browser_disposable_profile_activity_changed")?;
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
        drop(retirement_guard);
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
        if !self.state.idle_closed_browsers.contains_key(&browser.id) {
            if self.state.shared_browser_ids.contains(&browser.id) {
                for tab in self
                    .state
                    .tabs
                    .values()
                    .filter(|tab| tab.browser_id == browser.id)
                {
                    self.effects.close_tab(browser, tab)?;
                }
            } else {
                self.effects.close_browser(browser)?;
            }
        }
        self.state.shared_browser_ids.remove(&browser.id);
        self.state
            .adopted_browser_process_identities
            .remove(&browser.id);
        let session_ids = browser.active_session_ids.clone();
        for session_id in session_ids {
            self.state.leave_open_session_ids.remove(&session_id);
            if let Some(session) = self.state.sessions.remove(&session_id) {
                let tabs = self
                    .state
                    .tabs
                    .values()
                    .filter(|tab| tab.session_id == session.id)
                    .cloned()
                    .collect::<Vec<_>>();
                for tab in tabs {
                    self.state.tab_lifetimes.remove(&tab.id);
                    self.state.tab_metadata.remove(&tab.id);
                    self.state.pending_human_tabs.remove(&tab.id);
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
        self.state.idle_closed_browsers.remove(&browser.id);
        Ok(())
    }
}
