//! Pure recovery admission for retained logical browsers.
//!
//! Adapters own process observation, persistence, and replacement effects. This
//! module decides whether an exact observation may admit one replacement.

use serde::{Deserialize, Serialize};

pub const BROWSER_RECOVERY_STATE_SCHEMA_V1: &str = "agent-browser.browser-recovery-state.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRecoveryDemand {
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

/// Values come from the runtime's existing configuration. The pure model
/// intentionally defines no competing defaults.
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
                });
            }
            BrowserRecoveryPhase::Terminal => {
                return Ok(BrowserRecoveryDecision::Exhausted {
                    state: state.clone(),
                });
            }
            BrowserRecoveryPhase::RetryWait if now_ms < state.next_eligible_at_ms => {
                return Ok(BrowserRecoveryDecision::WaitForRetry {
                    retry_at_ms: state.next_eligible_at_ms,
                });
            }
            BrowserRecoveryPhase::RetryWait | BrowserRecoveryPhase::Recovered => {}
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
