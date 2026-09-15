use crate::{
    evaluate_provider_free_scenario, ChallengeCompositeReceipt, ChallengeProfile,
    InterventionReason, ProviderFreeScenarioOutcome,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskFixture {
    AmbiguousObservation,
    ChallengeNotPresent,
    PassAfterAcknowledgedResolution,
    RejectedResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskPhase {
    Ready,
    Observing,
    Deciding,
    Resolving,
    Verifying,
    Admitted,
    CoolingDown,
    InterventionRequired,
    NotAdmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskOutcome {
    Denied,
    InterventionRequired,
    NotPresent,
    Passed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskDelivery {
    Acknowledged,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskVerification {
    Passed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskAdmission {
    Admitted,
    Withheld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskIntervention {
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeTaskCooldown {
    Active,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeTaskError {
    InvalidRequest,
    TransitionBudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeTaskRequest {
    pub task_id: String,
    pub profile: ChallengeProfile,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
    pub fixture: ChallengeTaskFixture,
    pub max_transitions: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeTaskReceipt {
    pub schema_version: &'static str,
    pub task_id: String,
    pub profile: ChallengeProfile,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
    pub challenge: ChallengeCompositeReceipt,
    pub phases: Vec<ChallengeTaskPhase>,
    pub outcome: ChallengeTaskOutcome,
    pub delivery: Option<ChallengeTaskDelivery>,
    pub verification: Option<ChallengeTaskVerification>,
    pub admission: ChallengeTaskAdmission,
    pub intervention: Option<ChallengeTaskIntervention>,
    pub cooldown: ChallengeTaskCooldown,
    pub attempts_started: u8,
    pub emitted_effects: bool,
}

/// Execute one repository-owned provider-free challenge task end to end.
///
/// The task owns phase ordering. Callers receive one receipt and cannot emit
/// browser, provider, network, or desktop-input effects through this seam.
pub fn execute_provider_free_task(
    request: ChallengeTaskRequest,
) -> Result<ChallengeTaskReceipt, ChallengeTaskError> {
    if request.site_policy_digest.len() != 64
        || !request
            .site_policy_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ChallengeTaskError::InvalidRequest);
    }
    let scenario = match request.fixture {
        ChallengeTaskFixture::AmbiguousObservation => {
            ProviderFreeScenarioOutcome::InterventionRequired
        }
        ChallengeTaskFixture::ChallengeNotPresent => ProviderFreeScenarioOutcome::NotPresent,
        ChallengeTaskFixture::PassAfterAcknowledgedResolution => {
            ProviderFreeScenarioOutcome::Passed
        }
        ChallengeTaskFixture::RejectedResolution => ProviderFreeScenarioOutcome::Denied,
    };
    let challenge = evaluate_provider_free_scenario(request.profile, scenario)
        .map_err(|_| ChallengeTaskError::InvalidRequest)?;
    let (phases, admission, cooldown) = match request.fixture {
        ChallengeTaskFixture::AmbiguousObservation => (
            vec![
                ChallengeTaskPhase::Ready,
                ChallengeTaskPhase::Observing,
                ChallengeTaskPhase::Deciding,
                ChallengeTaskPhase::InterventionRequired,
            ],
            ChallengeTaskAdmission::Withheld,
            ChallengeTaskCooldown::NotRequired,
        ),
        ChallengeTaskFixture::ChallengeNotPresent => (
            vec![
                ChallengeTaskPhase::Ready,
                ChallengeTaskPhase::Observing,
                ChallengeTaskPhase::Deciding,
                ChallengeTaskPhase::Admitted,
            ],
            ChallengeTaskAdmission::Admitted,
            ChallengeTaskCooldown::NotRequired,
        ),
        ChallengeTaskFixture::PassAfterAcknowledgedResolution => (
            vec![
                ChallengeTaskPhase::Ready,
                ChallengeTaskPhase::Observing,
                ChallengeTaskPhase::Deciding,
                ChallengeTaskPhase::Resolving,
                ChallengeTaskPhase::Verifying,
                ChallengeTaskPhase::Admitted,
            ],
            ChallengeTaskAdmission::Admitted,
            ChallengeTaskCooldown::Active,
        ),
        ChallengeTaskFixture::RejectedResolution => (
            vec![
                ChallengeTaskPhase::Ready,
                ChallengeTaskPhase::Observing,
                ChallengeTaskPhase::Deciding,
                ChallengeTaskPhase::Resolving,
                ChallengeTaskPhase::CoolingDown,
                ChallengeTaskPhase::NotAdmitted,
            ],
            ChallengeTaskAdmission::Withheld,
            ChallengeTaskCooldown::Active,
        ),
    };
    let transitions = phases.len().saturating_sub(1);
    if transitions > usize::from(request.max_transitions) {
        return Err(ChallengeTaskError::TransitionBudgetExceeded);
    }
    Ok(ChallengeTaskReceipt {
        schema_version: "challenge-task-receipt.v1",
        task_id: request.task_id,
        profile: request.profile,
        site_policy_digest: request.site_policy_digest,
        downstream_intent_id: request.downstream_intent_id,
        outcome: match challenge.outcome {
            ProviderFreeScenarioOutcome::NotPresent => ChallengeTaskOutcome::NotPresent,
            ProviderFreeScenarioOutcome::Passed => ChallengeTaskOutcome::Passed,
            ProviderFreeScenarioOutcome::Denied => ChallengeTaskOutcome::Denied,
            ProviderFreeScenarioOutcome::InterventionRequired => {
                ChallengeTaskOutcome::InterventionRequired
            }
            ProviderFreeScenarioOutcome::Eligible => {
                return Err(ChallengeTaskError::InvalidRequest)
            }
        },
        delivery: challenge.delivery.map(|delivery| match delivery {
            "acknowledged" => ChallengeTaskDelivery::Acknowledged,
            "rejected" => ChallengeTaskDelivery::Rejected,
            _ => unreachable!("registered provider-free delivery"),
        }),
        verification: challenge
            .verification
            .map(|verification| match verification {
                "passed" => ChallengeTaskVerification::Passed,
                _ => unreachable!("registered provider-free verification"),
            }),
        intervention: challenge
            .intervention
            .map(|intervention| match intervention {
                InterventionReason::Ambiguous => ChallengeTaskIntervention::Ambiguous,
                _ => unreachable!("registered provider-free intervention"),
            }),
        attempts_started: challenge.attempts_started,
        challenge,
        phases,
        admission,
        cooldown,
        emitted_effects: false,
    })
}
