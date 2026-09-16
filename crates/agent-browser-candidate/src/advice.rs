use serde::{Deserialize, Serialize};

use crate::OperationState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedState {
    Idle,
    BuildActive,
    ArtifactSealed,
    InstallActive,
    RecoveryRequired,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateRelation {
    NoActiveCandidate,
    SameCandidate,
    EquivalentInput,
    CompetingCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryOutcome {
    BuildRecommended,
    InstallRecommended,
    AlreadyApplied,
    JoinedExisting,
    Queued,
    RecoveryRequired,
    IntegrityPreconditionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorAction {
    Inspect,
    Build,
    Install,
    Observe,
    Wait,
    Queue,
    Cancel,
    Discard,
    Supersede,
    Recover,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrityPreconditions {
    pub manifest_valid: bool,
    pub artifact_digest_valid: bool,
    pub revision_current: bool,
    pub fencing_generation_current: bool,
}

impl IntegrityPreconditions {
    pub fn satisfied() -> Self {
        Self {
            manifest_valid: true,
            artifact_digest_valid: true,
            revision_current: true,
            fencing_generation_current: true,
        }
    }

    fn failed_names(&self) -> Vec<String> {
        [
            ("manifest_valid", self.manifest_valid),
            ("artifact_digest_valid", self.artifact_digest_valid),
            ("revision_current", self.revision_current),
            (
                "fencing_generation_current",
                self.fencing_generation_current,
            ),
        ]
        .into_iter()
        .filter(|(_, valid)| !valid)
        .map(|(name, _)| name.to_string())
        .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveOperation {
    pub operation_id: String,
    pub candidate_id: String,
    pub state: OperationState,
    pub revision: u64,
    pub fencing_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdvisoryInput {
    pub observed_state: ObservedState,
    pub requested_candidate_id: String,
    pub candidate_relation: CandidateRelation,
    pub active_operation: Option<ActiveOperation>,
    pub integrity: IntegrityPreconditions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdvisoryResult {
    pub observed_state: ObservedState,
    pub outcome: AdvisoryOutcome,
    pub recommendation: OperatorAction,
    pub alternatives: Vec<OperatorAction>,
    pub consequences: Vec<String>,
    pub integrity_preconditions: Vec<String>,
    pub reason_codes: Vec<String>,
    pub active_operation: Option<ActiveOperation>,
}

pub fn advise(input: AdvisoryInput) -> AdvisoryResult {
    let failed = input.integrity.failed_names();
    if !failed.is_empty() {
        return result(
            &input,
            AdvisoryOutcome::IntegrityPreconditionFailed,
            OperatorAction::Recover,
            vec![OperatorAction::Inspect],
            vec!["no_effect_performed"],
            failed,
            vec!["integrity_precondition_failed"],
        );
    }

    match (input.observed_state, input.candidate_relation) {
        (ObservedState::BuildActive, CandidateRelation::SameCandidate)
        | (ObservedState::BuildActive, CandidateRelation::EquivalentInput) => result(
            &input,
            AdvisoryOutcome::JoinedExisting,
            OperatorAction::Observe,
            vec![OperatorAction::Cancel],
            vec!["no_new_build"],
            Vec::new(),
            vec!["equivalent_build_active"],
        ),
        (ObservedState::InstallActive, CandidateRelation::CompetingCandidate) => result(
            &input,
            AdvisoryOutcome::Queued,
            OperatorAction::Wait,
            vec![
                OperatorAction::Queue,
                OperatorAction::Cancel,
                OperatorAction::Discard,
                OperatorAction::Supersede,
            ],
            vec!["active_candidate_preserved", "no_effect_performed"],
            Vec::new(),
            vec!["competing_candidate_active"],
        ),
        (ObservedState::InstallActive, CandidateRelation::SameCandidate)
        | (ObservedState::InstallActive, CandidateRelation::EquivalentInput) => result(
            &input,
            AdvisoryOutcome::JoinedExisting,
            OperatorAction::Observe,
            vec![OperatorAction::Cancel, OperatorAction::Recover],
            vec!["no_second_install_transaction"],
            Vec::new(),
            vec!["equivalent_install_active"],
        ),
        (ObservedState::RecoveryRequired, _) => result(
            &input,
            AdvisoryOutcome::RecoveryRequired,
            OperatorAction::Recover,
            vec![OperatorAction::Inspect, OperatorAction::Rollback],
            vec!["new_effects_blocked_until_recovery"],
            Vec::new(),
            vec!["active_operation_requires_recovery"],
        ),
        (ObservedState::Accepted, CandidateRelation::SameCandidate)
        | (ObservedState::Accepted, CandidateRelation::EquivalentInput) => result(
            &input,
            AdvisoryOutcome::AlreadyApplied,
            OperatorAction::Inspect,
            Vec::new(),
            vec!["no_effect_performed"],
            Vec::new(),
            vec!["candidate_already_accepted"],
        ),
        (ObservedState::ArtifactSealed, _) => result(
            &input,
            AdvisoryOutcome::InstallRecommended,
            OperatorAction::Install,
            vec![OperatorAction::Inspect, OperatorAction::Discard],
            vec!["install_requires_explicit_apply"],
            Vec::new(),
            vec!["sealed_artifact_available"],
        ),
        _ => result(
            &input,
            AdvisoryOutcome::BuildRecommended,
            OperatorAction::Build,
            vec![OperatorAction::Inspect],
            vec!["build_required"],
            Vec::new(),
            vec!["no_reusable_candidate"],
        ),
    }
}

fn result(
    input: &AdvisoryInput,
    outcome: AdvisoryOutcome,
    recommendation: OperatorAction,
    alternatives: Vec<OperatorAction>,
    consequences: Vec<&str>,
    integrity_preconditions: Vec<String>,
    reason_codes: Vec<&str>,
) -> AdvisoryResult {
    AdvisoryResult {
        observed_state: input.observed_state,
        outcome,
        recommendation,
        alternatives,
        consequences: consequences.into_iter().map(str::to_string).collect(),
        integrity_preconditions,
        reason_codes: reason_codes.into_iter().map(str::to_string).collect(),
        active_operation: input.active_operation.clone(),
    }
}
