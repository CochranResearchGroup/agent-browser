//! Provider-free duplication intent. This module is not a public operation yet:
//! adapters must persist it, hold source custody, copy data and publish/launch.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateIntent {
    pub operation_id: String,
    pub source_profile_id: String,
    pub destination_profile_id: String,
    pub browser_build: String,
    pub source_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuplicatePhase {
    AwaitingOwnerChoice,
    AwaitingGracefulClose,
    AwaitingStoppedCustody,
    CopyAdmitted,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuplicateChoice {
    FocusSource,
    CloseAndDuplicate,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDuplicateOperation {
    pub intent: DuplicateIntent,
    pub phase: DuplicatePhase,
    pub affected_task_ids: Vec<String>,
}

impl ProfileDuplicateOperation {
    /// Captures reviewable intent only. Peer occupancy is a warning, not a veto.
    pub fn prepare(
        intent: DuplicateIntent,
        affected_task_ids: Vec<String>,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            intent,
            phase: DuplicatePhase::AwaitingOwnerChoice,
            affected_task_ids,
        })
    }
}
