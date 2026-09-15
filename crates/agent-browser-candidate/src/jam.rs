use serde::{Deserialize, Serialize};

use crate::{validate_nonempty, validate_sha256, CandidateError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgressEvidence {
    pub sequence: u64,
    pub phase: String,
    pub observed_at_unix_seconds: i64,
    pub process_evidence_sha256: String,
}

impl ProgressEvidence {
    fn validate(&self) -> Result<(), CandidateError> {
        validate_nonempty("progress_phase", &self.phase)?;
        validate_sha256("process_evidence_sha256", &self.process_evidence_sha256)?;
        if self.observed_at_unix_seconds < 0 {
            return Err(CandidateError::new(
                "invalid_progress_time",
                "progress time must not be negative",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryChoice {
    Wait,
    Cancel,
    Recover,
    Supersede,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LivenessStatus {
    Healthy {
        deadline_unix_seconds: i64,
        last_phase: String,
    },
    Stalled {
        deadline_unix_seconds: i64,
        last_phase: String,
        last_progress_at_unix_seconds: i64,
        process_evidence_sha256: String,
        recovery_choices: Vec<RecoveryChoice>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LivenessTracker {
    pub operation_id: String,
    pub last_progress: ProgressEvidence,
    pub deadline_unix_seconds: i64,
}

impl LivenessTracker {
    pub fn new(
        operation_id: impl Into<String>,
        initial: ProgressEvidence,
        renewal_seconds: i64,
    ) -> Result<Self, CandidateError> {
        let operation_id = operation_id.into();
        validate_nonempty("operation_id", &operation_id)?;
        initial.validate()?;
        let deadline_unix_seconds = renewal_deadline(&initial, renewal_seconds)?;
        Ok(Self {
            operation_id,
            last_progress: initial,
            deadline_unix_seconds,
        })
    }

    pub fn record_progress(
        &mut self,
        progress: ProgressEvidence,
        renewal_seconds: i64,
    ) -> Result<(), CandidateError> {
        progress.validate()?;
        let expected_sequence = self.last_progress.sequence.checked_add(1).ok_or_else(|| {
            CandidateError::new("progress_sequence_exhausted", "progress sequence overflow")
        })?;
        if progress.sequence != expected_sequence {
            return Err(CandidateError::new(
                "progress_sequence_mismatch",
                "progress sequence must advance by exactly one",
            ));
        }
        if progress.observed_at_unix_seconds <= self.last_progress.observed_at_unix_seconds {
            return Err(CandidateError::new(
                "progress_time_not_monotonic",
                "progress observation time must advance",
            ));
        }
        if progress.phase == self.last_progress.phase
            && progress.process_evidence_sha256 == self.last_progress.process_evidence_sha256
        {
            return Err(CandidateError::new(
                "durable_progress_missing",
                "phase or durable process evidence must change before renewal",
            ));
        }
        let deadline = renewal_deadline(&progress, renewal_seconds)?;
        self.last_progress = progress;
        self.deadline_unix_seconds = deadline;
        Ok(())
    }

    pub fn status(&self, now_unix_seconds: i64) -> LivenessStatus {
        if now_unix_seconds <= self.deadline_unix_seconds {
            return LivenessStatus::Healthy {
                deadline_unix_seconds: self.deadline_unix_seconds,
                last_phase: self.last_progress.phase.clone(),
            };
        }
        LivenessStatus::Stalled {
            deadline_unix_seconds: self.deadline_unix_seconds,
            last_phase: self.last_progress.phase.clone(),
            last_progress_at_unix_seconds: self.last_progress.observed_at_unix_seconds,
            process_evidence_sha256: self.last_progress.process_evidence_sha256.clone(),
            recovery_choices: vec![
                RecoveryChoice::Wait,
                RecoveryChoice::Cancel,
                RecoveryChoice::Recover,
                RecoveryChoice::Supersede,
            ],
        }
    }
}

fn renewal_deadline(
    progress: &ProgressEvidence,
    renewal_seconds: i64,
) -> Result<i64, CandidateError> {
    if renewal_seconds <= 0 {
        return Err(CandidateError::new(
            "invalid_renewal_duration",
            "renewal duration must be positive",
        ));
    }
    progress
        .observed_at_unix_seconds
        .checked_add(renewal_seconds)
        .ok_or_else(|| CandidateError::new("renewal_deadline_exhausted", "deadline overflow"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockClass {
    Coordination,
    InstallTransaction,
    RuntimeMutation,
    ServiceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockEventKind {
    Acquire(LockClass),
    Release(LockClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockEvent {
    pub monotonic_millis: u64,
    pub kind: LockEventKind,
}

impl LockEvent {
    pub fn acquire(monotonic_millis: u64, lock: LockClass) -> Self {
        Self {
            monotonic_millis,
            kind: LockEventKind::Acquire(lock),
        }
    }

    pub fn release(monotonic_millis: u64, lock: LockClass) -> Self {
        Self {
            monotonic_millis,
            kind: LockEventKind::Release(lock),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockTraceSummary {
    pub acquisition_count: usize,
    pub maximum_hold_millis: u64,
}

pub fn validate_lock_trace(
    events: &[LockEvent],
    maximum_allowed_hold_millis: u64,
) -> Result<LockTraceSummary, CandidateError> {
    let mut held = Vec::<(LockClass, u64)>::new();
    let mut previous_time = None;
    let mut acquisition_count = 0;
    let mut maximum_hold_millis = 0;
    for event in events {
        if previous_time.is_some_and(|previous| event.monotonic_millis < previous) {
            return Err(CandidateError::new(
                "lock_trace_time_not_monotonic",
                "lock trace times must be monotonic",
            ));
        }
        previous_time = Some(event.monotonic_millis);
        match event.kind {
            LockEventKind::Acquire(lock) => {
                if held.iter().any(|(held_lock, _)| *held_lock == lock)
                    || held.last().is_some_and(|(held_lock, _)| lock <= *held_lock)
                {
                    return Err(CandidateError::new(
                        "lock_order_violation",
                        "locks must be acquired once in coordination, install, runtime, Service State order",
                    ));
                }
                held.push((lock, event.monotonic_millis));
                acquisition_count += 1;
            }
            LockEventKind::Release(lock) => {
                let Some((held_lock, acquired_at)) = held.pop() else {
                    return Err(CandidateError::new(
                        "lock_release_without_acquire",
                        "lock release has no matching acquisition",
                    ));
                };
                if held_lock != lock {
                    return Err(CandidateError::new(
                        "lock_release_order_violation",
                        "locks must be released in reverse acquisition order",
                    ));
                }
                let held_for = event.monotonic_millis - acquired_at;
                if held_for > maximum_allowed_hold_millis {
                    return Err(CandidateError::new(
                        "physical_lock_hold_exceeded",
                        format!(
                            "{lock:?} was held for {held_for} ms, above the {maximum_allowed_hold_millis} ms budget"
                        ),
                    ));
                }
                maximum_hold_millis = maximum_hold_millis.max(held_for);
            }
        }
    }
    if !held.is_empty() {
        return Err(CandidateError::new(
            "lock_trace_incomplete",
            "all acquired locks must be released",
        ));
    }
    Ok(LockTraceSummary {
        acquisition_count,
        maximum_hold_millis,
    })
}
