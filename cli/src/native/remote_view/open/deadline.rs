#![allow(unused_imports)]
use super::shared::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteBoundRuntimeIssue {
    RequestedProfileInUseByPid {
        profile_id: String,
        pid: u32,
        owner_browser_id: Option<String>,
        owner_session_id: Option<String>,
        compatibility_message: String,
    },
    EffectFailed {
        operation: &'static str,
        message: String,
    },
    ForwardDeadlineElapsed {
        operation: &'static str,
        total_ms: u64,
    },
    Cancelled {
        operation: &'static str,
    },
}
impl RouteBoundRuntimeIssue {
    pub(crate) fn compatibility_message(&self) -> &str {
        match self {
            Self::RequestedProfileInUseByPid {
                compatibility_message,
                ..
            } => compatibility_message,
            Self::EffectFailed { message, .. } => message,
            Self::ForwardDeadlineElapsed { .. } => "Service job timed out during route-bound open",
            Self::Cancelled { .. } => "Service job was cancelled while running",
        }
    }
}
pub(crate) trait RouteBoundOpenClock: Send + Sync {
    fn now_ms(&self) -> u64;
}
pub(crate) struct SystemRouteBoundOpenClock {
    pub(crate) started_at: Instant,
}
impl RouteBoundOpenClock for SystemRouteBoundOpenClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RouteBoundOpenDeadline {
    pub(crate) total_ms: u64,
    pub(crate) compensation_reserve_ms: u64,
    pub(crate) forward_deadline_ms: u64,
}
impl RouteBoundOpenDeadline {
    pub(crate) fn from_total_ms(total_ms: u64) -> Self {
        let compensation_reserve_ms = total_ms.saturating_div(5).clamp(250, 15_000);
        Self {
            total_ms,
            compensation_reserve_ms,
            forward_deadline_ms: total_ms.saturating_sub(compensation_reserve_ms),
        }
    }
}
pub(crate) struct RouteBoundOpenSupervisor {
    pub(crate) deadline: Option<RouteBoundOpenDeadline>,
    pub(crate) cancellation: Option<CancellationToken>,
    pub(crate) clock: Arc<dyn RouteBoundOpenClock>,
}
impl RouteBoundOpenSupervisor {
    const MAX_REPOSITORY_LOCK_MS: u64 = 1_000;
    pub(crate) fn system(total_ms: Option<u64>, cancellation: Option<CancellationToken>) -> Self {
        Self {
            deadline: total_ms
                .filter(|value| *value > 0)
                .map(RouteBoundOpenDeadline::from_total_ms),
            cancellation,
            clock: Arc::new(SystemRouteBoundOpenClock {
                started_at: Instant::now(),
            }),
        }
    }
    #[cfg(test)]
    pub(crate) fn with_clock(
        total_ms: Option<u64>,
        cancellation: Option<CancellationToken>,
        clock: Arc<dyn RouteBoundOpenClock>,
    ) -> Self {
        Self {
            deadline: total_ms
                .filter(|value| *value > 0)
                .map(RouteBoundOpenDeadline::from_total_ms),
            cancellation,
            clock,
        }
    }
    pub(crate) fn ensure_forward(
        &self,
        operation: &'static str,
    ) -> Result<(), RouteBoundRuntimeIssue> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(RouteBoundRuntimeIssue::Cancelled { operation });
        }
        if let Some(deadline) = self.deadline {
            if self.clock.now_ms() >= deadline.forward_deadline_ms {
                return Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed {
                    operation,
                    total_ms: deadline.total_ms,
                });
            }
        }
        Ok(())
    }
    pub(crate) fn remaining_forward_ms(&self) -> Option<u64> {
        self.deadline.map(|deadline| {
            deadline
                .forward_deadline_ms
                .saturating_sub(self.clock.now_ms())
        })
    }
    pub(crate) fn remaining_total_ms(&self) -> Option<u64> {
        self.deadline
            .map(|deadline| deadline.total_ms.saturating_sub(self.clock.now_ms()))
    }
    pub(crate) fn forward_repository_lock_timeout(&self) -> Duration {
        Duration::from_millis(
            self.remaining_forward_ms()
                .unwrap_or(Self::MAX_REPOSITORY_LOCK_MS)
                .clamp(1, Self::MAX_REPOSITORY_LOCK_MS),
        )
    }
    pub(crate) fn compensation_repository_lock_timeout(&self) -> Duration {
        Duration::from_millis(
            self.remaining_total_ms()
                .unwrap_or(Self::MAX_REPOSITORY_LOCK_MS)
                .clamp(1, Self::MAX_REPOSITORY_LOCK_MS),
        )
    }
    pub(crate) async fn forward<T>(
        &self,
        operation: &'static str,
        effect: RouteBoundOpenFuture<'_, T>,
    ) -> Result<T, RouteBoundRuntimeIssue> {
        self.ensure_forward(operation)?;
        let outcome = match (self.remaining_forward_ms(), self.cancellation.clone()) {
            (Some(remaining), Some(cancellation)) => {
                tokio::select! {
                    biased; _ = cancellation.cancelled() => {
                    Err(RouteBoundRuntimeIssue::Cancelled { operation }) } result =
                    tokio::time::timeout(Duration::from_millis(remaining.max(1)), effect)
                    => { result.unwrap_or_else(| _ |
                    Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed { operation,
                    total_ms : self.deadline.map(| deadline | deadline.total_ms)
                    .unwrap_or_default(), })) }
                }
            }
            (Some(remaining), None) => {
                tokio::time::timeout(Duration::from_millis(remaining.max(1)), effect)
                    .await
                    .unwrap_or_else(|_| {
                        Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed {
                            operation,
                            total_ms: self
                                .deadline
                                .map(|deadline| deadline.total_ms)
                                .unwrap_or_default(),
                        })
                    })
            }
            (None, Some(cancellation)) => {
                tokio::select! {
                    biased; _ = cancellation.cancelled() => {
                    Err(RouteBoundRuntimeIssue::Cancelled { operation }) } result =
                    effect => result,
                }
            }
            (None, None) => effect.await,
        }?;
        self.ensure_forward(operation)?;
        Ok(outcome)
    }
    pub(crate) async fn compensate<T>(
        &self,
        operation: &'static str,
        effect: RouteBoundOpenFuture<'_, T>,
    ) -> Result<T, RouteBoundRuntimeIssue> {
        let Some(remaining) = self.remaining_total_ms() else {
            return effect.await;
        };
        if remaining == 0 {
            return Err(RouteBoundRuntimeIssue::EffectFailed {
                operation,
                message: "rollback_incomplete: total route-bound deadline elapsed".to_string(),
            });
        }
        tokio::time::timeout(Duration::from_millis(remaining), effect)
            .await
            .unwrap_or_else(|_| {
                Err(RouteBoundRuntimeIssue::EffectFailed {
                    operation,
                    message:
                        "rollback_incomplete: compensation did not finish by the total deadline"
                            .to_string(),
                })
            })
    }
}
impl std::fmt::Display for RouteBoundRuntimeIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.compatibility_message())
    }
}
pub(crate) type RouteBoundOpenFuture<'a, T> =
    std::pin::Pin<Box<dyn Future<Output = Result<T, RouteBoundRuntimeIssue>> + Send + 'a>>;
#[derive(Debug, Clone)]
pub(crate) struct RouteBoundOpenExecutionError {
    pub(crate) message: String,
    pub(crate) runtime_issue: Option<RouteBoundRuntimeIssue>,
    pub(crate) terminal_failure: Option<RouteBoundTerminalFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteBoundCompensationState {
    RolledBack,
    RollbackIncomplete,
}

impl RouteBoundCompensationState {
    pub(crate) fn from_evidence(evidence: &Value) -> Self {
        if evidence.get("state").and_then(Value::as_str) == Some("rollback_incomplete") {
            Self::RollbackIncomplete
        } else {
            Self::RolledBack
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RolledBack => "rolled_back",
            Self::RollbackIncomplete => "rollback_incomplete",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RouteBoundTerminalFailure {
    pub(crate) blocker_code: String,
    pub(crate) blocker_message: String,
    pub(crate) compensation_state: RouteBoundCompensationState,
    pub(crate) evidence: Value,
    pub(crate) compatibility_error: String,
}
impl From<String> for RouteBoundOpenExecutionError {
    fn from(message: String) -> Self {
        Self {
            message,
            runtime_issue: None,
            terminal_failure: None,
        }
    }
}
impl From<&str> for RouteBoundOpenExecutionError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}
impl From<RouteBoundRuntimeIssue> for RouteBoundOpenExecutionError {
    fn from(issue: RouteBoundRuntimeIssue) -> Self {
        Self {
            message: issue.compatibility_message().to_string(),
            runtime_issue: Some(issue),
            terminal_failure: None,
        }
    }
}
pub(crate) fn route_bound_execution_error_with_cleanup(
    issue: RouteBoundRuntimeIssue,
    blocker_code: &'static str,
    evidence: Value,
    cleanup_summary: &str,
) -> RouteBoundOpenExecutionError {
    let blocker_message = issue.compatibility_message().to_string();
    let compatibility_error = format!("{blocker_message}; cleanup={cleanup_summary}");
    RouteBoundOpenExecutionError {
        message: compatibility_error.clone(),
        runtime_issue: Some(issue),
        terminal_failure: Some(RouteBoundTerminalFailure {
            blocker_code: blocker_code.to_string(),
            blocker_message,
            compensation_state: RouteBoundCompensationState::from_evidence(&evidence),
            evidence,
            compatibility_error,
        }),
    }
}

pub(crate) fn route_bound_message_error_with_cleanup(
    blocker_code: &'static str,
    blocker_message: String,
    evidence: Value,
    cleanup_summary: &str,
) -> RouteBoundOpenExecutionError {
    let compatibility_error = format!("{blocker_message}; cleanup={cleanup_summary}");
    RouteBoundOpenExecutionError {
        message: compatibility_error.clone(),
        runtime_issue: None,
        terminal_failure: Some(RouteBoundTerminalFailure {
            blocker_code: blocker_code.to_string(),
            blocker_message,
            compensation_state: RouteBoundCompensationState::from_evidence(&evidence),
            evidence,
            compatibility_error,
        }),
    }
}
