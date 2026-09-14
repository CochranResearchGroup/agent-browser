//! Pure, deterministic challenge lifecycle decisions.
//!
//! This crate returns policy decisions only. It has no browser, desktop-input,
//! platform, persistence, network, or runtime authority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeState {
    NotPresent,
    CheckboxPresent,
    Solving,
    VerificationInProgress,
    Passed,
    Failed,
    Expired,
    ChallengeOpen,
    Inconclusive,
    HumanInterventionRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationPosture {
    NotPresent,
    Eligible,
    Ambiguous,
    UnsupportedChallengeOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    Acknowledged,
    Partial,
    Uncertain,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeCompletion {
    Passed,
    Failed,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengePolicy {
    pub policy_digest: String,
    pub allowed_provider_ids: Vec<String>,
    pub max_attempts: u8,
    pub max_steps: u8,
    pub max_pointer_events: u8,
    pub max_key_events: u8,
    pub cooldown_ms: u64,
    pub deadline_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeSnapshot {
    pub state: ChallengeState,
    pub provider_id: Option<String>,
    pub attempts_started: u8,
    pub steps_executed: u8,
    pub pointer_events: u8,
    pub key_events: u8,
    pub cooldown_until_ms: u64,
    pub terminal_receipt_ref: Option<String>,
}

impl Default for ChallengeSnapshot {
    fn default() -> Self {
        Self {
            state: ChallengeState::NotPresent,
            provider_id: None,
            attempts_started: 0,
            steps_executed: 0,
            pointer_events: 0,
            key_events: 0,
            cooldown_until_ms: 0,
            terminal_receipt_ref: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeEvent {
    Observed {
        posture: ObservationPosture,
        provider_id: String,
        now_ms: u64,
    },
    StartAttempt {
        now_ms: u64,
    },
    EffectFinished {
        delivery: DeliveryState,
        steps: u8,
        pointer_events: u8,
        key_events: u8,
    },
    Verified {
        completion: ChallengeCompletion,
    },
    AuthorityLost,
    DeadlineExpired,
    Replay {
        original_receipt_ref: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionReason {
    Unsupported,
    Ambiguous,
    AuthorityLost,
    PartialEffect,
    UncertainEffect,
    AttemptExhausted,
    VerificationInconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeDecision {
    NoEffect(ChallengeSnapshot),
    PermitAttempt(ChallengeSnapshot),
    AwaitVerification(ChallengeSnapshot),
    Terminal {
        snapshot: ChallengeSnapshot,
        intervention: Option<InterventionReason>,
    },
    Replay {
        snapshot: ChallengeSnapshot,
        original_receipt_ref: String,
        emitted_new_effects: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeControlError {
    InvalidPolicy,
    InvalidTransition,
    ProviderUnavailable,
    BudgetExceeded,
    CooldownActive,
}

pub fn decide(
    policy: &ChallengePolicy,
    current: &ChallengeSnapshot,
    event: ChallengeEvent,
) -> Result<ChallengeDecision, ChallengeControlError> {
    validate_policy(policy)?;
    if let ChallengeEvent::Replay {
        original_receipt_ref,
    } = event
    {
        if current.terminal_receipt_ref.as_deref() != Some(original_receipt_ref.as_str()) {
            return Err(ChallengeControlError::InvalidTransition);
        }
        return Ok(ChallengeDecision::Replay {
            snapshot: current.clone(),
            original_receipt_ref,
            emitted_new_effects: false,
        });
    }
    if matches!(event, ChallengeEvent::DeadlineExpired) {
        let mut next = current.clone();
        next.state = ChallengeState::Expired;
        return Ok(ChallengeDecision::Terminal {
            snapshot: next,
            intervention: None,
        });
    }
    if matches!(event, ChallengeEvent::AuthorityLost) {
        return Ok(intervention(
            current,
            ChallengeState::HumanInterventionRequired,
            InterventionReason::AuthorityLost,
        ));
    }
    match (&current.state, event) {
        (
            _,
            ChallengeEvent::Observed {
                posture,
                provider_id,
                now_ms,
            },
        ) => {
            if now_ms >= policy.deadline_at_ms {
                return Err(ChallengeControlError::InvalidTransition);
            }
            if !policy.allowed_provider_ids.contains(&provider_id) {
                return Err(ChallengeControlError::ProviderUnavailable);
            }
            let mut next = current.clone();
            next.provider_id = Some(provider_id);
            match posture {
                ObservationPosture::NotPresent => {
                    next.state = ChallengeState::NotPresent;
                    Ok(ChallengeDecision::NoEffect(next))
                }
                ObservationPosture::Eligible => {
                    next.state = ChallengeState::CheckboxPresent;
                    Ok(ChallengeDecision::NoEffect(next))
                }
                ObservationPosture::Ambiguous => Ok(intervention(
                    &next,
                    ChallengeState::HumanInterventionRequired,
                    InterventionReason::Ambiguous,
                )),
                ObservationPosture::UnsupportedChallengeOpen => Ok(intervention(
                    &next,
                    ChallengeState::ChallengeOpen,
                    InterventionReason::Unsupported,
                )),
            }
        }
        (ChallengeState::CheckboxPresent, ChallengeEvent::StartAttempt { now_ms }) => {
            if now_ms < current.cooldown_until_ms {
                return Err(ChallengeControlError::CooldownActive);
            }
            if current.attempts_started >= policy.max_attempts {
                return Ok(intervention(
                    current,
                    ChallengeState::HumanInterventionRequired,
                    InterventionReason::AttemptExhausted,
                ));
            }
            let mut next = current.clone();
            next.state = ChallengeState::Solving;
            next.attempts_started += 1;
            next.cooldown_until_ms = now_ms.saturating_add(policy.cooldown_ms);
            Ok(ChallengeDecision::PermitAttempt(next))
        }
        (
            ChallengeState::Solving,
            ChallengeEvent::EffectFinished {
                delivery,
                steps,
                pointer_events,
                key_events,
            },
        ) => {
            if steps > policy.max_steps
                || pointer_events > policy.max_pointer_events
                || key_events > policy.max_key_events
            {
                return Err(ChallengeControlError::BudgetExceeded);
            }
            let mut next = current.clone();
            next.steps_executed = steps;
            next.pointer_events = pointer_events;
            next.key_events = key_events;
            match delivery {
                DeliveryState::Acknowledged => {
                    next.state = ChallengeState::VerificationInProgress;
                    Ok(ChallengeDecision::AwaitVerification(next))
                }
                DeliveryState::Partial => Ok(intervention(
                    &next,
                    ChallengeState::HumanInterventionRequired,
                    InterventionReason::PartialEffect,
                )),
                DeliveryState::Uncertain => Ok(intervention(
                    &next,
                    ChallengeState::HumanInterventionRequired,
                    InterventionReason::UncertainEffect,
                )),
                DeliveryState::Rejected => {
                    next.state = ChallengeState::Failed;
                    Ok(ChallengeDecision::Terminal {
                        snapshot: next,
                        intervention: None,
                    })
                }
            }
        }
        (ChallengeState::VerificationInProgress, ChallengeEvent::Verified { completion }) => {
            let mut next = current.clone();
            match completion {
                ChallengeCompletion::Passed => {
                    next.state = ChallengeState::Passed;
                    Ok(ChallengeDecision::Terminal {
                        snapshot: next,
                        intervention: None,
                    })
                }
                ChallengeCompletion::Failed => {
                    next.state = ChallengeState::Failed;
                    Ok(ChallengeDecision::Terminal {
                        snapshot: next,
                        intervention: None,
                    })
                }
                ChallengeCompletion::Inconclusive => Ok(intervention(
                    &next,
                    ChallengeState::Inconclusive,
                    InterventionReason::VerificationInconclusive,
                )),
            }
        }
        _ => Err(ChallengeControlError::InvalidTransition),
    }
}

fn intervention(
    current: &ChallengeSnapshot,
    state: ChallengeState,
    reason: InterventionReason,
) -> ChallengeDecision {
    let mut next = current.clone();
    next.state = state;
    ChallengeDecision::Terminal {
        snapshot: next,
        intervention: Some(reason),
    }
}

fn validate_policy(policy: &ChallengePolicy) -> Result<(), ChallengeControlError> {
    if policy.policy_digest.len() != 64
        || policy.allowed_provider_ids.is_empty()
        || policy.max_attempts > 1
        || policy.max_steps == 0
        || policy.deadline_at_ms == 0
    {
        return Err(ChallengeControlError::InvalidPolicy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ChallengePolicy {
        ChallengePolicy {
            policy_digest: "a".repeat(64),
            allowed_provider_ids: vec!["controlled-x11-v1".to_string()],
            max_attempts: 1,
            max_steps: 8,
            max_pointer_events: 16,
            max_key_events: 0,
            cooldown_ms: 5_000,
            deadline_at_ms: 50_000,
        }
    }

    #[test]
    fn table_driven_lifecycle_replay_is_deterministic_and_effect_free() {
        let cases = [
            (ObservationPosture::NotPresent, ChallengeState::NotPresent),
            (
                ObservationPosture::Eligible,
                ChallengeState::CheckboxPresent,
            ),
            (
                ObservationPosture::Ambiguous,
                ChallengeState::HumanInterventionRequired,
            ),
            (
                ObservationPosture::UnsupportedChallengeOpen,
                ChallengeState::ChallengeOpen,
            ),
        ];
        for (posture, expected) in cases {
            let event = ChallengeEvent::Observed {
                posture,
                provider_id: "controlled-x11-v1".to_string(),
                now_ms: 1_000,
            };
            let first = decide(&policy(), &ChallengeSnapshot::default(), event.clone()).unwrap();
            let second = decide(&policy(), &ChallengeSnapshot::default(), event).unwrap();
            assert_eq!(first, second);
            let state = match first {
                ChallengeDecision::NoEffect(snapshot)
                | ChallengeDecision::Terminal { snapshot, .. } => snapshot.state,
                other => panic!("observation emitted an effect-capable decision: {other:?}"),
            };
            assert_eq!(state, expected);
        }
    }

    #[test]
    fn one_attempt_reaches_verification_and_terminal_replay_emits_nothing() {
        let observed = decide(
            &policy(),
            &ChallengeSnapshot::default(),
            ChallengeEvent::Observed {
                posture: ObservationPosture::Eligible,
                provider_id: "controlled-x11-v1".to_string(),
                now_ms: 1_000,
            },
        )
        .unwrap();
        let ChallengeDecision::NoEffect(eligible) = observed else {
            panic!()
        };
        let ChallengeDecision::PermitAttempt(solving) = decide(
            &policy(),
            &eligible,
            ChallengeEvent::StartAttempt { now_ms: 1_001 },
        )
        .unwrap() else {
            panic!()
        };
        let ChallengeDecision::AwaitVerification(verifying) = decide(
            &policy(),
            &solving,
            ChallengeEvent::EffectFinished {
                delivery: DeliveryState::Acknowledged,
                steps: 1,
                pointer_events: 2,
                key_events: 0,
            },
        )
        .unwrap() else {
            panic!()
        };
        let ChallengeDecision::Terminal {
            mut snapshot,
            intervention: None,
        } = decide(
            &policy(),
            &verifying,
            ChallengeEvent::Verified {
                completion: ChallengeCompletion::Passed,
            },
        )
        .unwrap()
        else {
            panic!()
        };
        snapshot.terminal_receipt_ref = Some("receipt:1".to_string());
        let replay = decide(
            &policy(),
            &snapshot,
            ChallengeEvent::Replay {
                original_receipt_ref: "receipt:1".to_string(),
            },
        )
        .unwrap();
        assert!(matches!(
            replay,
            ChallengeDecision::Replay {
                emitted_new_effects: false,
                ..
            }
        ));
    }

    #[test]
    fn uncertainty_and_budget_exhaustion_never_authorize_another_attempt() {
        let mut solving = ChallengeSnapshot {
            state: ChallengeState::Solving,
            provider_id: Some("controlled-x11-v1".to_string()),
            attempts_started: 1,
            ..ChallengeSnapshot::default()
        };
        let uncertain = decide(
            &policy(),
            &solving,
            ChallengeEvent::EffectFinished {
                delivery: DeliveryState::Uncertain,
                steps: 1,
                pointer_events: 1,
                key_events: 0,
            },
        )
        .unwrap();
        assert!(matches!(
            uncertain,
            ChallengeDecision::Terminal {
                intervention: Some(InterventionReason::UncertainEffect),
                ..
            }
        ));
        solving.state = ChallengeState::CheckboxPresent;
        let exhausted = decide(
            &policy(),
            &solving,
            ChallengeEvent::StartAttempt { now_ms: 10_000 },
        )
        .unwrap();
        assert!(matches!(
            exhausted,
            ChallengeDecision::Terminal {
                intervention: Some(InterventionReason::AttemptExhausted),
                ..
            }
        ));
    }
}
