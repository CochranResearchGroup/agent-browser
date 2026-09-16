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

/// Registered downstream consumer of a completed challenge task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeConsumerKind {
    Authentication,
    Navigation,
}

/// Stable, deserializable projection of the challenge receipt fields required
/// for a downstream admission decision.
///
/// The projection intentionally ignores provider and desktop details. Service
/// adapters may deserialize it from a durable full task receipt without
/// granting this pure crate persistence or runtime authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeConsumerEvidence {
    pub schema_version: String,
    pub task_id: String,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
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

impl From<&ChallengeTaskReceipt> for ChallengeConsumerEvidence {
    fn from(receipt: &ChallengeTaskReceipt) -> Self {
        Self {
            schema_version: receipt.schema_version.to_string(),
            task_id: receipt.task_id.clone(),
            site_policy_digest: receipt.site_policy_digest.clone(),
            downstream_intent_id: receipt.downstream_intent_id.clone(),
            phases: receipt.phases.clone(),
            outcome: receipt.outcome,
            delivery: receipt.delivery,
            verification: receipt.verification,
            admission: receipt.admission,
            intervention: receipt.intervention,
            cooldown: receipt.cooldown,
            attempts_started: receipt.attempts_started,
            emitted_effects: receipt.emitted_effects,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeConsumerAdmissionRequest {
    pub consumer: ChallengeConsumerKind,
    pub consumer_operation_id: String,
    pub expected_site_policy_digest: String,
    pub expected_downstream_intent_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeConsumerAdmission {
    Admitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeConsumerAdmissionReceipt {
    pub schema_version: &'static str,
    pub challenge_task_id: String,
    pub consumer: ChallengeConsumerKind,
    pub consumer_operation_id: String,
    pub site_policy_digest: String,
    pub downstream_intent_id: String,
    pub challenge_outcome: ChallengeTaskOutcome,
    pub challenge_admission: ChallengeTaskAdmission,
    pub challenge_cooldown: ChallengeTaskCooldown,
    pub challenge_emitted_effects: bool,
    pub consumer_admission: ChallengeConsumerAdmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeConsumerAdmissionError {
    InvalidRequest,
    InvalidReceipt,
    SitePolicyMismatch,
    DownstreamIntentMismatch,
    ChallengeWithheld,
}

/// Decide whether one registered consumer may continue from a completed
/// challenge task receipt.
///
/// This function is deterministic and emits no consumer effect. A successful
/// decision does not imply that authentication or navigation later succeeded.
pub fn admit_challenge_consumer(
    evidence: &ChallengeConsumerEvidence,
    request: ChallengeConsumerAdmissionRequest,
) -> Result<ChallengeConsumerAdmissionReceipt, ChallengeConsumerAdmissionError> {
    let valid_digest =
        |value: &str| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if request.consumer_operation_id.trim().is_empty()
        || request.expected_downstream_intent_id.trim().is_empty()
        || !valid_digest(&request.expected_site_policy_digest)
    {
        return Err(ChallengeConsumerAdmissionError::InvalidRequest);
    }
    if evidence.schema_version != "challenge-task-receipt.v1"
        || evidence.task_id.trim().is_empty()
        || evidence.downstream_intent_id.trim().is_empty()
        || !valid_digest(&evidence.site_policy_digest)
    {
        return Err(ChallengeConsumerAdmissionError::InvalidReceipt);
    }
    let terminal_phase_matches = match evidence.admission {
        ChallengeTaskAdmission::Admitted => {
            evidence.phases.last() == Some(&ChallengeTaskPhase::Admitted)
        }
        ChallengeTaskAdmission::Withheld => matches!(
            evidence.phases.last(),
            Some(ChallengeTaskPhase::NotAdmitted | ChallengeTaskPhase::InterventionRequired)
        ),
    };
    if !terminal_phase_matches {
        return Err(ChallengeConsumerAdmissionError::InvalidReceipt);
    }
    if !evidence
        .site_policy_digest
        .eq_ignore_ascii_case(&request.expected_site_policy_digest)
    {
        return Err(ChallengeConsumerAdmissionError::SitePolicyMismatch);
    }
    if evidence.downstream_intent_id != request.expected_downstream_intent_id {
        return Err(ChallengeConsumerAdmissionError::DownstreamIntentMismatch);
    }
    if evidence.admission != ChallengeTaskAdmission::Admitted
        || evidence.intervention.is_some()
        || !matches!(
            evidence.outcome,
            ChallengeTaskOutcome::NotPresent | ChallengeTaskOutcome::Passed
        )
    {
        return Err(ChallengeConsumerAdmissionError::ChallengeWithheld);
    }
    let outcome_is_valid = match evidence.outcome {
        ChallengeTaskOutcome::NotPresent => {
            evidence.attempts_started == 0
                && evidence.delivery.is_none()
                && evidence.verification.is_none()
        }
        ChallengeTaskOutcome::Passed => {
            evidence.attempts_started == 1
                && evidence.delivery == Some(ChallengeTaskDelivery::Acknowledged)
                && evidence.verification == Some(ChallengeTaskVerification::Passed)
        }
        ChallengeTaskOutcome::Denied | ChallengeTaskOutcome::InterventionRequired => false,
    };
    if !outcome_is_valid {
        return Err(ChallengeConsumerAdmissionError::InvalidReceipt);
    }
    Ok(ChallengeConsumerAdmissionReceipt {
        schema_version: "challenge-consumer-admission-receipt.v1",
        challenge_task_id: evidence.task_id.clone(),
        consumer: request.consumer,
        consumer_operation_id: request.consumer_operation_id,
        site_policy_digest: evidence.site_policy_digest.to_ascii_lowercase(),
        downstream_intent_id: evidence.downstream_intent_id.clone(),
        challenge_outcome: evidence.outcome,
        challenge_admission: evidence.admission,
        challenge_cooldown: evidence.cooldown,
        challenge_emitted_effects: evidence.emitted_effects,
        consumer_admission: ChallengeConsumerAdmission::Admitted,
    })
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
