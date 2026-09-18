use serde::{Deserialize, Serialize};

pub const SERVICE_PROFILE_SEEDING_MODE_VALUES: [&str; 3] =
    ["not_required", "detached_headed_no_cdp", "attachable_ok"];

pub const SERVICE_PROFILE_SEEDING_HANDOFF_STATE_VALUES: [&str; 10] = [
    "not_required",
    "needs_manual_seeding",
    "seeding_launched_detached",
    "seeding_waiting_for_close",
    "completion_declared_waiting_for_close",
    "seeding_closed_unverified",
    "verification_pending",
    "fresh",
    "failed",
    "abandoned",
];

/// Persisted lifecycle for a CDP-free profile seeding handoff.
///
/// The record is deliberately provider-free. Its caller supplies process and
/// browser observations, while this Module preserves the durable wire contract
/// and determines the operator-facing lifecycle posture.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileSeedingHandoffRecord {
    pub id: String,
    pub profile_id: String,
    pub target_service_id: String,
    pub state: ProfileSeedingHandoffState,
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub expires_at: Option<String>,
    pub last_prompted_at: Option<String>,
    pub declared_complete_at: Option<String>,
    pub closed_at: Option<String>,
    pub updated_at: Option<String>,
    pub actor: Option<String>,
    pub note: Option<String>,
}

impl ProfileSeedingHandoffRecord {
    /// Decode one durable record using the established camel-case, defaulting
    /// wire contract.
    pub fn decode_json(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    /// Encode one durable record using the established camel-case wire contract.
    pub fn encode_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// The durable key for the profile and target pair represented by this record.
    pub fn canonical_id(&self) -> String {
        profile_seeding_handoff_id(&self.profile_id, &self.target_service_id)
    }
}

/// Lifecycle state for CDP-free profile seeding handoffs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSeedingHandoffState {
    NotRequired,
    #[default]
    NeedsManualSeeding,
    SeedingLaunchedDetached,
    SeedingWaitingForClose,
    CompletionDeclaredWaitingForClose,
    SeedingClosedUnverified,
    VerificationPending,
    Fresh,
    Failed,
    Abandoned,
}

impl ProfileSeedingHandoffState {
    pub fn intervention_severity(self) -> &'static str {
        match self {
            Self::NotRequired | Self::Fresh => "info",
            Self::SeedingLaunchedDetached
            | Self::SeedingClosedUnverified
            | Self::VerificationPending => "attention",
            Self::NeedsManualSeeding
            | Self::SeedingWaitingForClose
            | Self::CompletionDeclaredWaitingForClose => "action_required",
            Self::Failed | Self::Abandoned => "danger",
        }
    }

    pub fn intervention_message(self) -> &'static str {
        match self {
            Self::NotRequired => "No CDP-free profile seeding action is required for this target.",
            Self::NeedsManualSeeding => {
                "Launch the detached headed browser, complete setup, close Chrome, then let agent-browser verify freshness after CDP is allowed again."
            }
            Self::SeedingLaunchedDetached => {
                "The detached seeding browser has been launched. Complete setup in Chrome, then close that browser."
            }
            Self::SeedingWaitingForClose => {
                "The seeding browser is still open. Finish setup, close Chrome, extend the handoff, or abandon it."
            }
            Self::CompletionDeclaredWaitingForClose => {
                "Completion was declared, but Chrome still appears open. Close the seeding browser before attachable automation resumes."
            }
            Self::SeedingClosedUnverified => {
                "The seeding browser is closed, but authentication freshness has not been verified."
            }
            Self::VerificationPending => {
                "The profile is ready for a bounded post-seeding auth probe."
            }
            Self::Fresh => "The profile has fresh authenticated evidence for this target.",
            Self::Failed => "The seeding handoff failed. Review the operator note and retry or abandon.",
            Self::Abandoned => "The seeding handoff was abandoned. Start a new handoff before authenticated work.",
        }
    }

    pub fn blocks_profile_lease(self) -> bool {
        matches!(
            self,
            Self::NeedsManualSeeding
                | Self::SeedingLaunchedDetached
                | Self::SeedingWaitingForClose
                | Self::CompletionDeclaredWaitingForClose
        )
    }
}

/// The canonical durable key for one profile seeding target.
pub fn profile_seeding_handoff_id(profile_id: &str, target_service_id: &str) -> String {
    format!("{profile_id}:{target_service_id}")
}

/// Browser launch posture required while a target profile is being seeded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSeedingMode {
    #[default]
    NotRequired,
    DetachedHeadedNoCdp,
    AttachableOk,
}
