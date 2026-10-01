//! Browser process intent retained across launch and session publication.
use serde::{Deserialize, Serialize};

use crate::{
    BrowserLaunch, BrowserSessionState, RemoteViewAssignmentRecord, RemoteViewAssignmentState,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserLaunchIntent {
    pub intent_id: String,
    pub profile_id: String,
    pub assignment: RemoteViewAssignmentRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserLaunchCustodyError {
    InvalidRecord,
    IdentityConflict,
    PublicationMissing,
}

impl BrowserLaunchIntent {
    pub fn validate(&self) -> Result<(), BrowserLaunchCustodyError> {
        let assignment = &self.assignment;
        if !crate::remote_view_application_response::uuid(&self.intent_id)
            || self.profile_id.trim().is_empty()
            || assignment.assignment_id.is_empty()
            || assignment.registration_id.is_empty()
            || assignment.pool_id.is_empty()
            || !crate::remote_view_application_response::uuid(&assignment.desktop_id)
            || assignment.generation == 0
            || assignment.state != RemoteViewAssignmentState::Active
        {
            return Err(BrowserLaunchCustodyError::InvalidRecord);
        }
        Ok(())
    }
}

/// Contains process identity, never launch environment or provider-private data.
/// Missing observation retains an unknown process outcome, not process absence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserLaunchCustodyRecord {
    pub schema_version: u32,
    pub intent: BrowserLaunchIntent,
    pub observed_browser_id: Option<String>,
    pub observed_pid: Option<u32>,
    pub published: bool,
}

impl BrowserLaunchCustodyRecord {
    pub fn pending(intent: BrowserLaunchIntent) -> Result<Self, BrowserLaunchCustodyError> {
        intent.validate()?;
        Ok(Self {
            schema_version: 1,
            intent,
            observed_browser_id: None,
            observed_pid: None,
            published: false,
        })
    }
    pub fn validate(&self) -> Result<(), BrowserLaunchCustodyError> {
        self.intent.validate()?;
        if self.schema_version != 1
            || !matches!(
                (&self.observed_browser_id, self.observed_pid),
                (None, None) | (Some(_), Some(1..))
            )
            || self
                .observed_browser_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
            || (self.published && self.observed_browser_id.is_none())
        {
            return Err(BrowserLaunchCustodyError::InvalidRecord);
        }
        Ok(())
    }
    pub fn observe(&mut self, launch: &BrowserLaunch) -> Result<(), BrowserLaunchCustodyError> {
        self.validate()?;
        let assignment = &self.intent.assignment;
        let desktop = launch
            .desktop
            .as_ref()
            .ok_or(BrowserLaunchCustodyError::IdentityConflict)?;
        if launch.browser_id.trim().is_empty()
            || launch.pid == 0
            || desktop.desktop_id != assignment.desktop_id
            || desktop.generation != assignment.generation
            || self
                .observed_browser_id
                .as_ref()
                .is_some_and(|id| id != &launch.browser_id)
            || self.observed_pid.is_some_and(|pid| pid != launch.pid)
        {
            return Err(BrowserLaunchCustodyError::IdentityConflict);
        }
        self.observed_browser_id = Some(launch.browser_id.clone());
        self.observed_pid = Some(launch.pid);
        Ok(())
    }
    /// Adapters must call this against the state published in the same durable
    /// transaction. Pure validation alone does not persist either record.
    pub fn confirm_publication(
        &mut self,
        state: &BrowserSessionState,
    ) -> Result<(), BrowserLaunchCustodyError> {
        self.validate()?;
        if state.schema_version != crate::BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(BrowserLaunchCustodyError::InvalidRecord);
        }
        let browser = self
            .observed_browser_id
            .as_ref()
            .and_then(|id| state.browsers.get(id))
            .ok_or(BrowserLaunchCustodyError::PublicationMissing)?;
        let assignment = &self.intent.assignment;
        if Some(browser.pid) != self.observed_pid
            || browser.profile_id != self.intent.profile_id
            || browser.desktop.as_ref().is_none_or(|desktop| {
                desktop.desktop_id != assignment.desktop_id
                    || desktop.generation != assignment.generation
            })
            || Some(&browser.id) != self.observed_browser_id.as_ref()
        {
            return Err(BrowserLaunchCustodyError::IdentityConflict);
        }
        self.published = true;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchCustodyStoreError {
    Unavailable,
    InvalidRecord,
    Conflict,
    ProfileOccupied,
    ReleaseFenced,
    LaunchPending,
    CleanupBlocked,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LaunchCustodyAdmission {
    New,
    Existing(BrowserLaunchCustodyRecord),
}

/// Injectable durable launch boundary. Admission commits the exact pending
/// intent before returning New. Existing never authorizes another process launch.
/// Observation is immutable by intent and process identity. Publication compares
/// current session state and commits session state plus custody together.
/// Implementations must retain unknown outcomes for explicit reconciliation.
pub trait BrowserLaunchCustodyStore {
    /// Read the exact published launch assignment for a currently owned browser.
    fn published_launch_assignment(
        &mut self,
        _browser: &crate::ManagedBrowserInstance,
    ) -> Result<crate::RemoteViewAssignmentRecord, LaunchCustodyStoreError> {
        Err(LaunchCustodyStoreError::Unavailable)
    }

    /// Read unresolved process claims before accepting effects after restart.
    /// A store without this readback cannot construct a consumer session adapter.
    fn unpublished_launch_records(
        &mut self,
    ) -> Result<Vec<BrowserLaunchCustodyRecord>, LaunchCustodyStoreError> {
        Err(LaunchCustodyStoreError::Unavailable)
    }
    fn admit_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
    ) -> Result<LaunchCustodyAdmission, LaunchCustodyStoreError>;
    fn observe_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        launch: &BrowserLaunch,
    ) -> Result<(), LaunchCustodyStoreError>;
    fn publish_launch_intent(
        &mut self,
        intent: &BrowserLaunchIntent,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), LaunchCustodyStoreError>;
}
