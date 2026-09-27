//! Pure recovery admission for retained logical browsers.
//!
//! Adapters own process observation, persistence, and replacement effects. This
//! module decides whether an exact observation may admit one replacement.

use serde::{Deserialize, Serialize};

pub const BROWSER_RECOVERY_STATE_SCHEMA_V1: &str = "agent-browser.browser-recovery-state.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRecoveryDemand {
    BaselineCapacity,
    AuthenticatedActiveViewer,
    Dormant,
    ExactClientResume,
}

impl BrowserRecoveryDemand {
    fn is_eager(self) -> bool {
        !matches!(self, Self::Dormant)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OldBrowserUsability {
    Usable,
    ProvenUnusable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRecoveryPhase {
    Admitted,
    ObservedLive,
    RetryWait,
    Terminal,
    Recovered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserRecoveryState {
    pub schema_version: String,
    pub browser_id: String,
    pub generation: u64,
    pub attempts: u32,
    pub started_at_ms: u64,
    pub deadline_at_ms: u64,
    pub next_eligible_at_ms: u64,
    pub phase: BrowserRecoveryPhase,
}

/// Values are supplied by the runtime's existing recovery configuration. This
/// model intentionally defines no competing defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserRecoveryAdmissionPolicy {
    pub maximum_attempts: u32,
    pub base_backoff_ms: u64,
    pub maximum_backoff_ms: u64,
    pub deadline_ms: u64,
}

impl BrowserRecoveryAdmissionPolicy {
    pub fn validate(self) -> Result<Self, String> {
        if self.maximum_attempts == 0
            || self.base_backoff_ms == 0
            || self.maximum_backoff_ms < self.base_backoff_ms
            || self.deadline_ms == 0
        {
            return Err("browser_recovery_policy_invalid".to_string());
        }
        Ok(self)
    }

    fn backoff_after_attempt(self, attempt: u32) -> u64 {
        let multiplier = 1_u64
            .checked_shl(attempt.saturating_sub(1))
            .unwrap_or(u64::MAX);
        self.base_backoff_ms
            .saturating_mul(multiplier)
            .min(self.maximum_backoff_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserRecoveryDecision {
    BrowserUsable,
    AwaitOldBrowserProof,
    WaitForExactClientResume,
    WaitForRetry { retry_at_ms: u64 },
    AlreadyAdmitted { generation: u64 },
    AdmitReplacement { state: BrowserRecoveryState },
    Exhausted { state: BrowserRecoveryState },
}

pub fn decide_browser_recovery(
    current: Option<&BrowserRecoveryState>,
    browser_id: &str,
    demand: BrowserRecoveryDemand,
    old_browser: OldBrowserUsability,
    now_ms: u64,
    policy: BrowserRecoveryAdmissionPolicy,
) -> Result<BrowserRecoveryDecision, String> {
    let policy = policy.validate()?;
    if browser_id.is_empty() {
        return Err("browser_recovery_browser_id_invalid".to_string());
    }
    if let Some(current) = current {
        validate_state(current, browser_id)?;
    }
    match old_browser {
        OldBrowserUsability::Usable => return Ok(BrowserRecoveryDecision::BrowserUsable),
        OldBrowserUsability::Unknown => return Ok(BrowserRecoveryDecision::AwaitOldBrowserProof),
        OldBrowserUsability::ProvenUnusable => {}
    }
    if !demand.is_eager() {
        return Ok(BrowserRecoveryDecision::WaitForExactClientResume);
    }

    if let Some(state) = current {
        match state.phase {
            BrowserRecoveryPhase::Admitted | BrowserRecoveryPhase::ObservedLive => {
                return Ok(BrowserRecoveryDecision::AlreadyAdmitted {
                    generation: state.generation,
                })
            }
            BrowserRecoveryPhase::Terminal => {
                return Ok(BrowserRecoveryDecision::Exhausted {
                    state: state.clone(),
                })
            }
            BrowserRecoveryPhase::RetryWait if now_ms < state.next_eligible_at_ms => {
                return Ok(BrowserRecoveryDecision::WaitForRetry {
                    retry_at_ms: state.next_eligible_at_ms,
                })
            }
            BrowserRecoveryPhase::RetryWait => {}
            BrowserRecoveryPhase::Recovered => {}
        }
        if state.phase != BrowserRecoveryPhase::Recovered
            && (now_ms >= state.deadline_at_ms || state.attempts >= policy.maximum_attempts)
        {
            let mut exhausted = state.clone();
            exhausted.phase = BrowserRecoveryPhase::Terminal;
            return Ok(BrowserRecoveryDecision::Exhausted { state: exhausted });
        }
    }

    let (generation, attempts, started_at_ms, deadline_at_ms) = match current {
        Some(state) if state.phase == BrowserRecoveryPhase::Recovered => (
            state
                .generation
                .checked_add(1)
                .ok_or_else(|| "browser_recovery_generation_exhausted".to_string())?,
            1,
            now_ms,
            now_ms.saturating_add(policy.deadline_ms),
        ),
        Some(state) => (
            state
                .generation
                .checked_add(1)
                .ok_or_else(|| "browser_recovery_generation_exhausted".to_string())?,
            state
                .attempts
                .checked_add(1)
                .ok_or_else(|| "browser_recovery_attempts_exhausted".to_string())?,
            state.started_at_ms,
            state.deadline_at_ms,
        ),
        None => (1, 1, now_ms, now_ms.saturating_add(policy.deadline_ms)),
    };
    Ok(BrowserRecoveryDecision::AdmitReplacement {
        state: BrowserRecoveryState {
            schema_version: BROWSER_RECOVERY_STATE_SCHEMA_V1.to_string(),
            browser_id: browser_id.to_string(),
            generation,
            attempts,
            started_at_ms,
            deadline_at_ms,
            next_eligible_at_ms: now_ms,
            phase: BrowserRecoveryPhase::Admitted,
        },
    })
}

pub fn record_browser_recovery_success(
    current: &BrowserRecoveryState,
    generation: u64,
    recovered_at_ms: u64,
) -> Result<BrowserRecoveryState, String> {
    validate_state(current, &current.browser_id)?;
    if current.phase != BrowserRecoveryPhase::ObservedLive || current.generation != generation {
        return Err("browser_recovery_success_fence_mismatch".to_string());
    }
    let mut next = current.clone();
    next.attempts = 0;
    next.started_at_ms = recovered_at_ms;
    next.deadline_at_ms = recovered_at_ms;
    next.next_eligible_at_ms = recovered_at_ms;
    next.phase = BrowserRecoveryPhase::Recovered;
    Ok(next)
}

pub fn record_browser_recovery_observed_live(
    current: &BrowserRecoveryState,
    generation: u64,
) -> Result<BrowserRecoveryState, String> {
    validate_state(current, &current.browser_id)?;
    if current.phase != BrowserRecoveryPhase::Admitted || current.generation != generation {
        return Err("browser_recovery_observed_live_fence_mismatch".to_string());
    }
    let mut next = current.clone();
    next.phase = BrowserRecoveryPhase::ObservedLive;
    Ok(next)
}

pub fn record_browser_recovery_failure(
    current: &BrowserRecoveryState,
    generation: u64,
    failed_at_ms: u64,
    policy: BrowserRecoveryAdmissionPolicy,
) -> Result<BrowserRecoveryState, String> {
    let policy = policy.validate()?;
    validate_state(current, &current.browser_id)?;
    if current.phase != BrowserRecoveryPhase::Admitted || current.generation != generation {
        return Err("browser_recovery_failure_fence_mismatch".to_string());
    }
    let mut next = current.clone();
    if current.attempts >= policy.maximum_attempts || failed_at_ms >= current.deadline_at_ms {
        next.phase = BrowserRecoveryPhase::Terminal;
        next.next_eligible_at_ms = current.deadline_at_ms;
    } else {
        next.phase = BrowserRecoveryPhase::RetryWait;
        next.next_eligible_at_ms = failed_at_ms
            .saturating_add(policy.backoff_after_attempt(current.attempts))
            .min(current.deadline_at_ms);
    }
    Ok(next)
}

fn validate_state(state: &BrowserRecoveryState, browser_id: &str) -> Result<(), String> {
    if state.schema_version != BROWSER_RECOVERY_STATE_SCHEMA_V1
        || state.browser_id != browser_id
        || state.browser_id.is_empty()
        || state.generation == 0
        || (state.attempts == 0 && state.phase != BrowserRecoveryPhase::Recovered)
        || (state.attempts > 0 && state.phase == BrowserRecoveryPhase::Recovered)
        || state.deadline_at_ms < state.started_at_ms
        || state.next_eligible_at_ms > state.deadline_at_ms
    {
        return Err("browser_recovery_state_invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> BrowserRecoveryAdmissionPolicy {
        BrowserRecoveryAdmissionPolicy {
            maximum_attempts: 3,
            base_backoff_ms: 1_000,
            maximum_backoff_ms: 30_000,
            deadline_ms: 90_000,
        }
    }

    fn admitted(decision: BrowserRecoveryDecision) -> BrowserRecoveryState {
        match decision {
            BrowserRecoveryDecision::AdmitReplacement { state } => state,
            other => panic!("expected admission, got {other:?}"),
        }
    }

    #[test]
    fn demand_and_old_browser_proof_gate_admission() {
        let policy = policy();
        for demand in [
            BrowserRecoveryDemand::BaselineCapacity,
            BrowserRecoveryDemand::AuthenticatedActiveViewer,
            BrowserRecoveryDemand::ExactClientResume,
        ] {
            assert!(matches!(
                decide_browser_recovery(
                    None,
                    "browser",
                    demand,
                    OldBrowserUsability::ProvenUnusable,
                    10,
                    policy
                )
                .unwrap(),
                BrowserRecoveryDecision::AdmitReplacement { .. }
            ));
        }
        assert_eq!(
            decide_browser_recovery(
                None,
                "browser",
                BrowserRecoveryDemand::Dormant,
                OldBrowserUsability::ProvenUnusable,
                10,
                policy
            )
            .unwrap(),
            BrowserRecoveryDecision::WaitForExactClientResume
        );
        assert_eq!(
            decide_browser_recovery(
                None,
                "browser",
                BrowserRecoveryDemand::ExactClientResume,
                OldBrowserUsability::Unknown,
                10,
                policy
            )
            .unwrap(),
            BrowserRecoveryDecision::AwaitOldBrowserProof
        );
    }

    #[test]
    fn retry_backoff_is_bounded_and_deadline_is_terminal() {
        let policy = BrowserRecoveryAdmissionPolicy {
            maximum_attempts: 3,
            base_backoff_ms: 10,
            maximum_backoff_ms: 15,
            deadline_ms: 100,
        };
        let first = admitted(
            decide_browser_recovery(
                None,
                "browser",
                BrowserRecoveryDemand::BaselineCapacity,
                OldBrowserUsability::ProvenUnusable,
                1_000,
                policy,
            )
            .unwrap(),
        );
        let wait = record_browser_recovery_failure(&first, 1, 1_005, policy).unwrap();
        assert_eq!(wait.next_eligible_at_ms, 1_015);
        assert_eq!(
            decide_browser_recovery(
                Some(&wait),
                "browser",
                BrowserRecoveryDemand::BaselineCapacity,
                OldBrowserUsability::ProvenUnusable,
                1_014,
                policy
            )
            .unwrap(),
            BrowserRecoveryDecision::WaitForRetry { retry_at_ms: 1_015 }
        );
        let second = admitted(
            decide_browser_recovery(
                Some(&wait),
                "browser",
                BrowserRecoveryDemand::BaselineCapacity,
                OldBrowserUsability::ProvenUnusable,
                1_015,
                policy,
            )
            .unwrap(),
        );
        assert_eq!(second.generation, 2);
        let wait = record_browser_recovery_failure(&second, 2, 1_020, policy).unwrap();
        assert_eq!(wait.next_eligible_at_ms, 1_035);
        let at_deadline = decide_browser_recovery(
            Some(&wait),
            "browser",
            BrowserRecoveryDemand::BaselineCapacity,
            OldBrowserUsability::ProvenUnusable,
            1_100,
            policy,
        )
        .unwrap();
        assert!(matches!(
            at_deadline,
            BrowserRecoveryDecision::Exhausted { .. }
        ));
    }

    #[test]
    fn state_wire_is_strict() {
        let state = admitted(
            decide_browser_recovery(
                None,
                "browser",
                BrowserRecoveryDemand::ExactClientResume,
                OldBrowserUsability::ProvenUnusable,
                10,
                policy(),
            )
            .unwrap(),
        );
        let mut value = serde_json::to_value(&state).unwrap();
        assert_eq!(
            serde_json::from_value::<BrowserRecoveryState>(value.clone()).unwrap(),
            state
        );
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<BrowserRecoveryState>(value).is_err());
    }

    #[test]
    fn success_is_generation_fenced_and_starts_a_fresh_attempt_window() {
        let policy = policy();
        let first = admitted(
            decide_browser_recovery(
                None,
                "browser",
                BrowserRecoveryDemand::ExactClientResume,
                OldBrowserUsability::ProvenUnusable,
                10,
                policy,
            )
            .unwrap(),
        );
        assert_eq!(
            record_browser_recovery_success(&first, 2, 20),
            Err("browser_recovery_success_fence_mismatch".to_string())
        );
        assert_eq!(
            record_browser_recovery_success(&first, 1, 20),
            Err("browser_recovery_success_fence_mismatch".to_string())
        );
        let observed = record_browser_recovery_observed_live(&first, 1).unwrap();
        assert_eq!(observed.phase, BrowserRecoveryPhase::ObservedLive);
        let recovered = record_browser_recovery_success(&observed, 1, 20).unwrap();
        assert_eq!(recovered.phase, BrowserRecoveryPhase::Recovered);
        assert_eq!(recovered.attempts, 0);
        let next = admitted(
            decide_browser_recovery(
                Some(&recovered),
                "browser",
                BrowserRecoveryDemand::BaselineCapacity,
                OldBrowserUsability::ProvenUnusable,
                100,
                policy,
            )
            .unwrap(),
        );
        assert_eq!(next.generation, 2);
        assert_eq!(next.attempts, 1);
        assert_eq!(next.started_at_ms, 100);
        assert_eq!(next.deadline_at_ms, 90_100);
    }
}
