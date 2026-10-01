//! Construct release acknowledgement from complete Agent Browser evidence.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    BrowserSessionState, RemoteViewApplicationCleanup, RemoteViewApplicationReleaseTarget,
    RemoteViewPresentationRetentionState, BROWSER_SESSION_STATE_SCHEMA_V1,
};

/// Absence requires a complete scoped inventory; an unavailable owner is not
/// an empty list. Runtime adapters collect these inventories under release
/// admission fencing, rather than reusing a cached snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "obligationIds",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RemoteViewApplicationObligationInventory {
    Unknown,
    Complete(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationCleanupSnapshot {
    pub schema_version: u32,
    pub assignment_id: String,
    pub desktop_id: String,
    pub lifecycle_generation: u64,
    pub pending_recovery: RemoteViewApplicationObligationInventory,
    pub foreground_leases: RemoteViewApplicationObligationInventory,
    pub cleanup_tasks: RemoteViewApplicationObligationInventory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteViewApplicationCleanupError {
    InvalidEvidence,
    IdentityConflict,
    EvidenceIncomplete,
    ApplicationReferencesRemain,
    PendingRecoveryRemains,
    ForegroundLeasesRemain,
    CleanupTasksRemain,
}

/// Constructible only by the evidence gate. Neither clonable nor deserializable;
/// raw wire booleans are not release admission.
///
/// Persisted wire acknowledgements cannot restore release admission:
/// ```compile_fail
/// use agent_browser_service_model::RemoteViewApplicationCleanupPermit;
/// let _: RemoteViewApplicationCleanupPermit = serde_json::from_str("{}").unwrap();
/// ```
/// A consumed permit cannot be duplicated for another release:
/// ```compile_fail
/// use agent_browser_service_model::RemoteViewApplicationCleanupPermit;
/// fn duplicate(permit: RemoteViewApplicationCleanupPermit) {
///     let _ = permit.clone();
/// }
/// ```
#[derive(Debug)]
pub struct RemoteViewApplicationCleanupPermit {
    target: RemoteViewApplicationReleaseTarget,
    acknowledgement: RemoteViewApplicationCleanup,
}

impl RemoteViewApplicationCleanupPermit {
    pub fn acknowledgement(&self) -> &RemoteViewApplicationCleanup {
        &self.acknowledgement
    }
    pub(crate) fn into_acknowledgement(
        self,
        target: &RemoteViewApplicationReleaseTarget,
    ) -> Result<RemoteViewApplicationCleanup, RemoteViewApplicationCleanupError> {
        if self.target != *target {
            return Err(RemoteViewApplicationCleanupError::IdentityConflict);
        }
        Ok(self.acknowledgement)
    }
}

fn inventory_clear(
    inventory: &RemoteViewApplicationObligationInventory,
    blocked: RemoteViewApplicationCleanupError,
) -> Result<(), RemoteViewApplicationCleanupError> {
    match inventory {
        RemoteViewApplicationObligationInventory::Unknown => {
            Err(RemoteViewApplicationCleanupError::EvidenceIncomplete)
        }
        RemoteViewApplicationObligationInventory::Complete(ids) if ids.is_empty() => Ok(()),
        RemoteViewApplicationObligationInventory::Complete(ids) => {
            let distinct: BTreeSet<_> = ids.iter().collect();
            if ids.iter().any(String::is_empty) || distinct.len() != ids.len() {
                Err(RemoteViewApplicationCleanupError::InvalidEvidence)
            } else {
                Err(blocked)
            }
        }
    }
}

/// Current browser affinities, retained joins, sessions and tabs all count.
/// Detaching presentation cannot clear live browser references. Independent
/// recovery, foreground and cleanup inventories must also be complete and clear.
pub fn prepare_remote_view_application_cleanup(
    state: &BrowserSessionState,
    target: &RemoteViewApplicationReleaseTarget,
    snapshot: &RemoteViewApplicationCleanupSnapshot,
) -> Result<RemoteViewApplicationCleanupPermit, RemoteViewApplicationCleanupError> {
    use RemoteViewApplicationCleanupError as Error;
    target.validate().map_err(|_| Error::InvalidEvidence)?;
    if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 || snapshot.schema_version != 1 {
        return Err(Error::InvalidEvidence);
    }
    let assignment = &target.assignment;
    if snapshot.assignment_id != assignment.assignment_id
        || snapshot.desktop_id != assignment.desktop_id
        || snapshot.lifecycle_generation != assignment.generation
    {
        return Err(Error::IdentityConflict);
    }
    let mut associated = BTreeSet::new();
    for (browser_id, retained) in &state.remote_view_presentations {
        if retained.assignment_id != assignment.assignment_id {
            if retained.desktop_id == assignment.desktop_id
                && retained.state != RemoteViewPresentationRetentionState::Released
            {
                return Err(Error::IdentityConflict);
            }
            continue;
        }
        if browser_id != &retained.browser_id
            || retained.registration_id != assignment.registration_id
            || retained.pool_id != assignment.pool_id
            || retained.desktop_id != assignment.desktop_id
            || retained.generation != assignment.generation
        {
            return Err(Error::IdentityConflict);
        }
        associated.insert(browser_id.as_str());
        if retained.state == RemoteViewPresentationRetentionState::Active {
            return Err(Error::ApplicationReferencesRemain);
        }
    }
    for (browser_id, browser) in &state.browsers {
        if browser.id != *browser_id {
            return Err(Error::InvalidEvidence);
        }
        if let Some(desktop) = &browser.desktop {
            if desktop.desktop_id == assignment.desktop_id {
                if desktop.generation != assignment.generation {
                    return Err(Error::IdentityConflict);
                }
                return Err(Error::ApplicationReferencesRemain);
            }
        }
        if associated.contains(browser_id.as_str()) {
            return Err(Error::ApplicationReferencesRemain);
        }
    }
    for (session_id, session) in &state.sessions {
        if session.id != *session_id {
            return Err(Error::InvalidEvidence);
        }
        if associated.contains(session.browser_id.as_str()) {
            return Err(Error::ApplicationReferencesRemain);
        }
        if !state.browsers.contains_key(&session.browser_id) {
            return Err(Error::EvidenceIncomplete);
        }
    }
    for (tab_id, tab) in &state.tabs {
        if tab.id != *tab_id {
            return Err(Error::InvalidEvidence);
        }
        if associated.contains(tab.browser_id.as_str()) {
            return Err(Error::ApplicationReferencesRemain);
        }
        if !state.browsers.contains_key(&tab.browser_id) {
            return Err(Error::EvidenceIncomplete);
        }
    }
    inventory_clear(&snapshot.pending_recovery, Error::PendingRecoveryRemains)?;
    inventory_clear(&snapshot.foreground_leases, Error::ForegroundLeasesRemain)?;
    inventory_clear(&snapshot.cleanup_tasks, Error::CleanupTasksRemain)?;
    Ok(RemoteViewApplicationCleanupPermit {
        target: target.clone(),
        acknowledgement: RemoteViewApplicationCleanup {
            schema_version: 1,
            assignment_id: assignment.assignment_id.clone(),
            lifecycle_generation: assignment.generation,
            application_references_clear: true,
            pending_recovery_clear: true,
            foreground_leases_clear: true,
            cleanup_tasks_clear: true,
        },
    })
}

/// Release admission must check current durable session and launch custody under
/// one transaction and retain its assignment fence across ambiguous outcomes.
/// Independent inventories still require scoped owner evidence from the caller.
pub trait BrowserReleaseCustodyStore {
    fn admit_assignment_release(
        &mut self,
        target: &RemoteViewApplicationReleaseTarget,
        snapshot: &RemoteViewApplicationCleanupSnapshot,
        release_id: &str,
    ) -> Result<RemoteViewApplicationCleanupPermit, crate::LaunchCustodyStoreError>;
}
