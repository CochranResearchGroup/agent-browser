//! Provider-free session, tab, and service-owned handle records.

use agent_browser_lease_authority::ServicePrincipalProvenance;
use serde::{Deserialize, Serialize};

use crate::{ProfileChildAccess, ProfileOrigin};

pub const SERVICE_LEASE_STATE_VALUES: [&str; 5] = [
    "shared",
    "exclusive",
    "human_takeover",
    "released",
    "expired",
];
pub const SERVICE_SESSION_CLEANUP_VALUES: [&str; 4] =
    ["detach", "close_tabs", "close_browser", "release_only"];
pub const SERVICE_PROFILE_SELECTION_REASON_VALUES: [&str; 7] = [
    "explicit_profile",
    "existing_owner",
    "authenticated_target",
    "account_match",
    "target_match",
    "service_allow_list",
    "browser_build_default",
];
pub const SERVICE_PROFILE_LEASE_DISPOSITION_VALUES: [&str; 3] =
    ["new_browser", "reused_browser", "active_lease_conflict"];
pub const SERVICE_TAB_LIFECYCLE_VALUES: [&str; 7] = [
    "unknown", "opening", "loading", "ready", "closing", "closed", "crashed",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceActor {
    Agent(String),
    Human(String),
    ApiClient(String),
    System,
}

impl ServiceActor {
    pub fn from_caller_context(service_name: Option<&str>, agent_name: Option<&str>) -> Self {
        if let Some(agent_name) = non_empty_label(agent_name) {
            return Self::Agent(agent_name.to_string());
        }
        if let Some(service_name) = non_empty_label(service_name) {
            return Self::ApiClient(service_name.to_string());
        }
        Self::System
    }

    pub fn is_system(&self) -> bool {
        matches!(self, Self::System)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Shared,
    Exclusive,
    HumanTakeover,
    Released,
    Expired,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCleanupPolicy {
    #[default]
    Detach,
    CloseTabs,
    CloseBrowser,
    ReleaseOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabLifecycle {
    Unknown,
    Opening,
    Loading,
    Ready,
    Closing,
    Closed,
    Crashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSelectionReason {
    ExplicitProfile,
    ExistingOwner,
    AuthenticatedTarget,
    AccountMatch,
    TargetMatch,
    ServiceAllowList,
    BrowserBuildDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileLeaseDisposition {
    NewBrowser,
    ReusedBrowser,
    ActiveLeaseConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserSession {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_provenance: Option<ServicePrincipalProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub work_lease_revision: u64,
    pub owner: ServiceActor,
    pub lease: LeaseState,
    pub profile_id: Option<String>,
    pub profile_selection_reason: Option<ProfileSelectionReason>,
    pub profile_lease_disposition: Option<ProfileLeaseDisposition>,
    pub profile_lease_conflict_session_ids: Vec<String>,
    pub browser_capability_launch: Option<serde_json::Value>,
    pub cleanup: SessionCleanupPolicy,
    pub browser_ids: Vec<String>,
    pub tab_ids: Vec<String>,
    pub created_at: Option<String>,
    pub last_lease_observed_at: Option<String>,
    pub expires_at: Option<String>,
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self {
            id: String::new(),
            boot_epoch: None,
            service_name: None,
            agent_name: None,
            task_name: None,
            principal_id: None,
            principal_provenance: None,
            work_lease_id: None,
            work_lease_revision: 0,
            owner: ServiceActor::System,
            lease: LeaseState::Shared,
            profile_id: None,
            profile_selection_reason: None,
            profile_lease_disposition: None,
            profile_lease_conflict_session_ids: Vec::new(),
            browser_capability_launch: None,
            cleanup: SessionCleanupPolicy::Detach,
            browser_ids: Vec::new(),
            tab_ids: Vec::new(),
            created_at: None,
            last_lease_observed_at: None,
            expires_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserTab {
    pub id: String,
    pub browser_id: String,
    pub target_id: Option<String>,
    pub session_id: Option<String>,
    pub lifecycle: TabLifecycle,
    pub url: Option<String>,
    pub title: Option<String>,
    pub owner_session_id: Option<String>,
    pub profile_access: Option<ProfileChildAccess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_provenance: Option<ServicePrincipalProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub work_lease_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_lease_expires_at: Option<String>,
    pub service_tab_handle: Option<ServiceTabHandle>,
    pub latest_snapshot_id: Option<String>,
    pub latest_screenshot_id: Option<String>,
    pub challenge_id: Option<String>,
}

impl Default for BrowserTab {
    fn default() -> Self {
        Self {
            id: String::new(),
            browser_id: String::new(),
            target_id: None,
            session_id: None,
            lifecycle: TabLifecycle::Unknown,
            url: None,
            title: None,
            owner_session_id: None,
            profile_access: None,
            principal_id: None,
            principal_provenance: None,
            work_lease_id: None,
            work_lease_revision: 0,
            work_lease_expires_at: None,
            service_tab_handle: None,
            latest_snapshot_id: None,
            latest_screenshot_id: None,
            challenge_id: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceTabHandle {
    pub browser_id: String,
    pub session_name: Option<String>,
    pub tab_id: String,
    pub target_id: Option<String>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub profile_id: Option<String>,
    pub profile_origin: ProfileOrigin,
    pub lease_id: Option<String>,
    pub lease_state: Option<LeaseState>,
    pub cleanup_policy: Option<SessionCleanupPolicy>,
    pub lease_heartbeat_expected: bool,
    pub owner_session_id: Option<String>,
    pub profile_access: Option<ProfileChildAccess>,
    pub job_id: Option<String>,
    pub trace_filter: ServiceTabHandleTraceFilter,
    pub valid: bool,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceTabHandleTraceFilter {
    pub browser_id: Option<String>,
    pub profile_id: Option<String>,
    pub session_id: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
}

fn non_empty_label(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_cli_state_records() {
        let session = BrowserSession::default();
        assert_eq!(session.lease, LeaseState::Shared);
        assert_eq!(session.owner, ServiceActor::System);
        assert_eq!(session.cleanup, SessionCleanupPolicy::Detach);
        assert_eq!(BrowserTab::default().lifecycle, TabLifecycle::Unknown);
        assert!(!ServiceTabHandle::default().valid);
    }

    #[test]
    fn serde_preserves_wire_names_and_omits_zero_optional_fields() {
        let mut session = BrowserSession::default();
        session.profile_selection_reason = Some(ProfileSelectionReason::AuthenticatedTarget);
        session.profile_lease_disposition = Some(ProfileLeaseDisposition::NewBrowser);
        let value = serde_json::to_value(&session).unwrap();
        assert_eq!(value["profileSelectionReason"], "authenticated_target");
        assert_eq!(value["profileLeaseDisposition"], "new_browser");
        assert!(value.get("bootEpoch").is_none());
        assert!(value.get("workLeaseRevision").is_none());
        assert_eq!(value["owner"], "system");
    }

    #[test]
    fn actor_inference_is_trimmed_and_deterministic() {
        assert_eq!(
            ServiceActor::from_caller_context(Some(" service "), Some(" agent ")),
            ServiceActor::Agent("agent".to_string())
        );
        assert_eq!(
            ServiceActor::from_caller_context(Some(" service "), None),
            ServiceActor::ApiClient("service".to_string())
        );
        assert!(ServiceActor::from_caller_context(Some("  "), Some("  ")).is_system());
    }

    #[test]
    fn value_constants_match_serialized_variants() {
        assert_eq!(SERVICE_LEASE_STATE_VALUES[0], "shared");
        assert_eq!(SERVICE_SESSION_CLEANUP_VALUES[1], "close_tabs");
        assert_eq!(
            SERVICE_PROFILE_SELECTION_REASON_VALUES[2],
            "authenticated_target"
        );
        assert_eq!(
            SERVICE_PROFILE_LEASE_DISPOSITION_VALUES[2],
            "active_lease_conflict"
        );
        assert_eq!(SERVICE_TAB_LIFECYCLE_VALUES[6], "crashed");
    }
}
