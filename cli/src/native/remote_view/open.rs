//! Route-bound browser acquisition and durable handoff resolution.

#![allow(unused_imports)]
use crate::native::action_runtime::browser_operations::{
    close_compatible_duplicate_targets, handle_tab_close, handle_tab_new, handle_view_focus,
    is_blank_url, no_duplicate_target_cleanup, origin_for_url, persist_service_owned_tab_new,
    tab_new_shared_acquisition_evidence,
};
use crate::native::action_runtime::common::*;
use crate::native::action_runtime::runtime::{
    command_or_params_value, default_control_input_provider, handle_close, handle_launch,
    managed_runtime_attach_target, optional_command_or_params_bool,
    optional_command_or_params_string, optional_command_string, parse_control_input_provider,
    service_browser_id, DaemonState, REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS,
};
use crate::native::action_runtime::service_commands::service_event_kind_name;

/// Transport-neutral attribution supplied after the ingress has authorized a
/// route-bound open. Cookies, headers, and transport sessions never cross this
/// seam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RouteBoundOpenAttribution {
    pub(crate) caller_id: Option<String>,
    pub(crate) service_job_id: Option<String>,
}

/// The only two ways a caller can ask the route-bound coordinator to work.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RouteBoundOpenInvocation {
    DirectOpen {
        request: Value,
        handoff_id: Option<String>,
        attribution: RouteBoundOpenAttribution,
    },
    DurableResolution {
        handoff_id: String,
        allow_reopen_closed: bool,
        attribution: RouteBoundOpenAttribution,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundOpenBlocker {
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundOpenCompensation {
    pub(crate) state: String,
    pub(crate) evidence: Value,
}

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
    fn compatibility_message(&self) -> &str {
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

struct SystemRouteBoundOpenClock {
    started_at: Instant,
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
        // The compensation reserve is carved out of the caller's existing
        // total deadline. It never extends the public job timeout.
        let compensation_reserve_ms = total_ms.saturating_div(5).clamp(250, 15_000);
        Self {
            total_ms,
            compensation_reserve_ms,
            forward_deadline_ms: total_ms.saturating_sub(compensation_reserve_ms),
        }
    }
}

pub(crate) struct RouteBoundOpenSupervisor {
    deadline: Option<RouteBoundOpenDeadline>,
    cancellation: Option<CancellationToken>,
    clock: Arc<dyn RouteBoundOpenClock>,
}

impl RouteBoundOpenSupervisor {
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
    fn with_clock(
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

    fn ensure_forward(&self, operation: &'static str) -> Result<(), RouteBoundRuntimeIssue> {
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

    fn remaining_forward_ms(&self) -> Option<u64> {
        self.deadline.map(|deadline| {
            deadline
                .forward_deadline_ms
                .saturating_sub(self.clock.now_ms())
        })
    }

    fn remaining_total_ms(&self) -> Option<u64> {
        self.deadline
            .map(|deadline| deadline.total_ms.saturating_sub(self.clock.now_ms()))
    }

    async fn forward<T>(
        &self,
        operation: &'static str,
        effect: RouteBoundOpenFuture<'_, T>,
    ) -> Result<T, RouteBoundRuntimeIssue> {
        self.ensure_forward(operation)?;
        let outcome = match (self.remaining_forward_ms(), self.cancellation.clone()) {
            (Some(remaining), Some(cancellation)) => tokio::select! {
                biased;
                _ = cancellation.cancelled() => {
                    Err(RouteBoundRuntimeIssue::Cancelled { operation })
                }
                result = tokio::time::timeout(Duration::from_millis(remaining.max(1)), effect) => {
                    result.unwrap_or_else(|_| Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed {
                        operation,
                        total_ms: self.deadline.map(|deadline| deadline.total_ms).unwrap_or_default(),
                    }))
                }
            },
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
            (None, Some(cancellation)) => tokio::select! {
                biased;
                _ = cancellation.cancelled() => {
                    Err(RouteBoundRuntimeIssue::Cancelled { operation })
                }
                result = effect => result,
            },
            (None, None) => effect.await,
        }?;
        self.ensure_forward(operation)?;
        Ok(outcome)
    }

    async fn compensate<T>(
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
struct RouteBoundOpenExecutionError {
    message: String,
    runtime_issue: Option<RouteBoundRuntimeIssue>,
}

impl From<String> for RouteBoundOpenExecutionError {
    fn from(message: String) -> Self {
        Self {
            message,
            runtime_issue: None,
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
        }
    }
}

fn route_bound_execution_error_with_cleanup(
    issue: RouteBoundRuntimeIssue,
    cleanup: &str,
) -> RouteBoundOpenExecutionError {
    RouteBoundOpenExecutionError {
        message: format!("{}; cleanup={}", issue.compatibility_message(), cleanup),
        runtime_issue: Some(issue),
    }
}

/// Raw browser facts observed by the coordinator. The runtime adapter does
/// not decide whether a browser or target is reusable.
#[derive(Debug, Clone)]
pub(crate) struct RouteBoundBrowserObservation {
    pub(crate) browser_present: bool,
    pub(crate) browser_pid: Option<u32>,
    pub(crate) browser_id: String,
    pub(crate) session_id: String,
    pub(crate) runtime_profile: Option<String>,
    pub(crate) active_target_id: Option<String>,
    pub(crate) active_url: Option<String>,
    pub(crate) active_title: Option<String>,
    pub(crate) pages: Vec<PageInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct LaunchBrowserRequest {
    pub(crate) command: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct SwitchTargetRequest {
    pub(crate) target_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct NavigateTargetRequest {
    pub(crate) url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct OpenTargetRequest {
    pub(crate) command: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct FocusTargetRequest {
    pub(crate) command: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct CloseCreatedTargetRequest {
    pub(crate) target_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CloseCreatedBrowserRequest {
    pub(crate) browser_identity: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct CheckoutRouteRequest {
    pub(crate) command: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct DisplayAccessRequest {
    pub(crate) binding: RemoteViewRouteBinding,
}

#[derive(Debug, Clone)]
pub(crate) struct VisibleWindowRequest {
    pub(crate) binding: RemoteViewRouteBinding,
}

#[derive(Debug, Clone)]
pub(crate) struct OperatorAccessRequest {
    pub(crate) binding: RemoteViewRouteBinding,
}

/// The frozen route-bound effect ledger. It deliberately has no generic
/// execute command or daemon-state escape hatch.
pub(crate) trait RouteBoundOpenRuntime {
    fn observe_browser(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation>;
    fn launch_browser(&mut self, request: LaunchBrowserRequest) -> RouteBoundOpenFuture<'_, Value>;
    fn refresh_targets(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation>;
    fn switch_target(&mut self, request: SwitchTargetRequest) -> RouteBoundOpenFuture<'_, Value>;
    fn navigate_target(
        &mut self,
        request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, Value>;
    fn open_target(&mut self, request: OpenTargetRequest) -> RouteBoundOpenFuture<'_, Value>;
    fn focus_target(&mut self, request: FocusTargetRequest) -> RouteBoundOpenFuture<'_, Value>;
    fn close_created_target(
        &mut self,
        request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, Value>;
    fn close_created_browser(
        &mut self,
        request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, Value>;
    fn checkout_route(&mut self, request: CheckoutRouteRequest) -> RouteBoundOpenFuture<'_, Value>;
    fn ensure_display_access(
        &mut self,
        request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Value>;
    fn observe_visible_window(
        &mut self,
        request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, Value>;
    fn observe_operator_access(
        &mut self,
        request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<Value>>;
}

/// Transitional daemon adapter. It performs effects only; route selection,
/// reuse, proof classification, and compensation ownership stay in the
/// coordinator.
pub(crate) struct ActionsRouteBoundOpenRuntime<'a> {
    state: &'a mut DaemonState,
}

impl<'a> ActionsRouteBoundOpenRuntime<'a> {
    pub(crate) fn new(state: &'a mut DaemonState) -> Self {
        Self { state }
    }
}

async fn observe_daemon_browser(
    state: &mut DaemonState,
) -> Result<RouteBoundBrowserObservation, RouteBoundRuntimeIssue> {
    let session_id = state.session_id.clone();
    let browser_id = service_browser_id(&session_id);
    let Some(manager) = state.browser.as_mut() else {
        return Ok(RouteBoundBrowserObservation {
            browser_present: false,
            browser_pid: None,
            browser_id,
            session_id,
            runtime_profile: None,
            active_target_id: None,
            active_url: None,
            active_title: None,
            pages: Vec::new(),
        });
    };
    Ok(RouteBoundBrowserObservation {
        browser_present: true,
        browser_pid: manager.browser_pid(),
        browser_id,
        session_id,
        runtime_profile: manager.runtime_profile_name().map(str::to_string),
        active_target_id: manager.active_target_id().ok().map(str::to_string),
        active_url: manager.get_url().await.ok(),
        active_title: manager.get_title().await.ok(),
        pages: manager.pages_list(),
    })
}

fn route_bound_runtime_issue(
    operation: &'static str,
    message: String,
    command: Option<&Value>,
) -> RouteBoundRuntimeIssue {
    let marker = "already in use by PID";
    if let Some(marker_index) = message.find(marker) {
        let pid = message[marker_index + marker.len()..]
            .trim_start_matches(|character: char| {
                character.is_ascii_whitespace() || matches!(character, ':' | '=')
            })
            .split(|character: char| !character.is_ascii_digit())
            .next()
            .and_then(|value| value.parse::<u32>().ok());
        if let Some(pid) = pid {
            let profile_id = command
                .and_then(|command| {
                    optional_command_string(command, "runtimeProfile")
                        .or_else(|| optional_command_string(command, "profileId"))
                        .or_else(|| optional_command_string(command, "profile"))
                })
                .unwrap_or_default();
            return RouteBoundRuntimeIssue::RequestedProfileInUseByPid {
                profile_id,
                pid,
                owner_browser_id: command
                    .and_then(|command| optional_command_string(command, "browserId")),
                owner_session_id: command
                    .and_then(|command| optional_command_string(command, "sessionName")),
                compatibility_message: message,
            };
        }
    }
    RouteBoundRuntimeIssue::EffectFailed { operation, message }
}

impl RouteBoundOpenRuntime for ActionsRouteBoundOpenRuntime<'_> {
    fn observe_browser(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(observe_daemon_browser(self.state))
    }

    fn launch_browser(&mut self, request: LaunchBrowserRequest) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            handle_launch(&request.command, self.state)
                .await
                .map_err(|message| {
                    route_bound_runtime_issue("launch_browser", message, Some(&request.command))
                })
        })
    }

    fn refresh_targets(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(async move {
            self.state.drain_cdp_events_background().await;
            observe_daemon_browser(self.state).await
        })
    }

    fn switch_target(&mut self, request: SwitchTargetRequest) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            let manager = self.state.browser.as_mut().ok_or_else(|| {
                route_bound_runtime_issue("switch_target", "Browser not launched".to_string(), None)
            })?;
            manager
                .tab_switch_target_id(&request.target_id)
                .await
                .map_err(|message| route_bound_runtime_issue("switch_target", message, None))
        })
    }

    fn navigate_target(
        &mut self,
        request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            let manager = self.state.browser.as_mut().ok_or_else(|| {
                route_bound_runtime_issue(
                    "navigate_target",
                    "Browser not launched".to_string(),
                    None,
                )
            })?;
            manager
                .navigate(&request.url, WaitUntil::None)
                .await
                .map_err(|message| route_bound_runtime_issue("navigate_target", message, None))
        })
    }

    fn open_target(&mut self, request: OpenTargetRequest) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            handle_tab_new(&request.command, self.state)
                .await
                .map_err(|message| {
                    route_bound_runtime_issue("open_target", message, Some(&request.command))
                })
        })
    }

    fn focus_target(&mut self, request: FocusTargetRequest) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            handle_view_focus(&request.command, self.state)
                .await
                .map_err(|message| {
                    route_bound_runtime_issue("focus_target", message, Some(&request.command))
                })
        })
    }

    fn close_created_target(
        &mut self,
        request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            let manager = self.state.browser.as_mut().ok_or_else(|| {
                route_bound_runtime_issue(
                    "close_created_target",
                    "Browser not launched".to_string(),
                    None,
                )
            })?;
            manager
                .tab_close_target_id(&request.target_id)
                .await
                .map_err(|message| route_bound_runtime_issue("close_created_target", message, None))
        })
    }

    fn close_created_browser(
        &mut self,
        request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            let _ = request.browser_identity;
            handle_close(self.state).await.map_err(|message| {
                route_bound_runtime_issue("close_created_browser", message, None)
            })
        })
    }

    fn checkout_route(&mut self, request: CheckoutRouteRequest) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            handle_service_remote_view_route_checkout(&request.command, self.state)
                .await
                .map_err(|message| {
                    route_bound_runtime_issue("checkout_route", message, Some(&request.command))
                })
        })
    }

    fn ensure_display_access(
        &mut self,
        request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            remote_view_open_ensure_display_access(&request.binding).map_err(|message| {
                route_bound_runtime_issue("ensure_display_access", message, None)
            })
        })
    }

    fn observe_visible_window(
        &mut self,
        request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            remote_view_open_visible_window_proof(&request.binding).map_err(|message| {
                route_bound_runtime_issue("observe_visible_window", message, None)
            })
        })
    }

    fn observe_operator_access(
        &mut self,
        request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<Value>> {
        Box::pin(
            async move { Ok(remote_view_open_operator_access_readiness(&request.binding).await) },
        )
    }
}

/// Complete typed result set for direct opens and durable resolution.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RouteBoundOpenOutcome {
    Planned {
        plan: Value,
    },
    NotFound {
        result: Value,
    },
    ExplicitlyClosed {
        result: Value,
    },
    Reopened {
        opened: Value,
    },
    Opened {
        opened: Value,
    },
    RolledBack {
        blocker: RouteBoundOpenBlocker,
        compensation: RouteBoundOpenCompensation,
        compatibility_error: String,
    },
    ProviderFallback {
        fallback: Value,
    },
}

impl RouteBoundOpenOutcome {
    fn into_compatibility_result(self) -> Result<Value, String> {
        match self {
            Self::Planned { plan }
            | Self::NotFound { result: plan }
            | Self::ExplicitlyClosed { result: plan }
            | Self::Reopened { opened: plan }
            | Self::Opened { opened: plan }
            | Self::ProviderFallback { fallback: plan } => Ok(plan),
            Self::RolledBack {
                compatibility_error,
                ..
            } => Err(compatibility_error),
        }
    }
}

/// One deep interface owns planning, acquisition, proof, compensation, and
/// durable resolution. Callers cannot invoke those phases independently.
pub(crate) struct RouteBoundOpenCoordinator;

impl RouteBoundOpenCoordinator {
    pub(crate) async fn open<R: RouteBoundOpenRuntime>(
        invocation: RouteBoundOpenInvocation,
        runtime: &mut R,
        supervisor: &RouteBoundOpenSupervisor,
    ) -> Result<RouteBoundOpenOutcome, String> {
        match invocation {
            RouteBoundOpenInvocation::DirectOpen {
                mut request,
                handoff_id,
                attribution,
            } => {
                if let Some(handoff_id) = handoff_id {
                    request["remoteViewHandoffId"] = Value::String(handoff_id);
                }
                if request.get("serviceJobId").is_none() {
                    if let Some(service_job_id) = attribution.service_job_id {
                        request["serviceJobId"] = Value::String(service_job_id);
                    }
                }
                let planned = remote_view_open_dry_run(&request);
                match execute_direct_open(&request, runtime, supervisor).await {
                    Ok(result) if planned => Ok(RouteBoundOpenOutcome::Planned { plan: result }),
                    Ok(result) => Ok(RouteBoundOpenOutcome::Opened { opened: result }),
                    Err(error) => rolled_back_outcome(error.message),
                }
            }
            RouteBoundOpenInvocation::DurableResolution {
                handoff_id,
                allow_reopen_closed,
                attribution,
            } => {
                let mut request = json!({
                    "handoffId": handoff_id,
                    "allowReopenClosed": allow_reopen_closed,
                });
                if let Some(service_job_id) = attribution.service_job_id {
                    request["serviceJobId"] = Value::String(service_job_id);
                }
                let result = execute_durable_resolution(&request, runtime, supervisor)
                    .await
                    .map_err(|error| error.message)?;
                match result.get("status").and_then(Value::as_str) {
                    Some("not_found") => Ok(RouteBoundOpenOutcome::NotFound { result }),
                    Some("closed") => Ok(RouteBoundOpenOutcome::ExplicitlyClosed { result }),
                    Some("best_effort") => {
                        Ok(RouteBoundOpenOutcome::ProviderFallback { fallback: result })
                    }
                    _ if allow_reopen_closed => {
                        Ok(RouteBoundOpenOutcome::Reopened { opened: result })
                    }
                    _ => Ok(RouteBoundOpenOutcome::Opened { opened: result }),
                }
            }
        }
    }
}

fn rolled_back_outcome(error: String) -> Result<RouteBoundOpenOutcome, String> {
    let Some((message, cleanup)) = error.split_once("; cleanup=") else {
        return Err(error);
    };
    let evidence =
        serde_json::from_str(cleanup).unwrap_or_else(|_| Value::String(cleanup.to_string()));
    let state = if cleanup.contains("rollback_incomplete") {
        "rollback_incomplete"
    } else {
        "rolled_back"
    };
    let code = message
        .split_once(':')
        .map(|(code, _)| code)
        .unwrap_or("route_bound_open_failed")
        .trim()
        .to_string();
    Ok(RouteBoundOpenOutcome::RolledBack {
        blocker: RouteBoundOpenBlocker {
            code,
            message: message.to_string(),
        },
        compensation: RouteBoundOpenCompensation {
            state: state.to_string(),
            evidence,
        },
        compatibility_error: error,
    })
}

pub(crate) async fn handle_remote_view_open(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let invocation = RouteBoundOpenInvocation::DirectOpen {
        request: cmd.clone(),
        handoff_id: optional_command_string(cmd, "remoteViewHandoffId")
            .or_else(|| optional_command_string(cmd, "serviceJobId")),
        attribution: RouteBoundOpenAttribution {
            caller_id: optional_command_string(cmd, "callerId"),
            service_job_id: optional_command_string(cmd, "serviceJobId"),
        },
    };
    let supervisor = RouteBoundOpenSupervisor::system(
        cmd.get("jobTimeoutMs").and_then(Value::as_u64),
        state.current_cancellation.clone(),
    );
    let mut runtime = ActionsRouteBoundOpenRuntime::new(state);
    RouteBoundOpenCoordinator::open(invocation, &mut runtime, &supervisor)
        .await?
        .into_compatibility_result()
}

pub(crate) async fn handle_service_remote_view_handoff_resolve(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handoff_id = optional_command_or_params_string(cmd, "handoffId")
        .or_else(|| optional_command_or_params_string(cmd, "remoteViewHandoffId"))
        .ok_or_else(|| "service_remote_view_handoff_resolve requires handoffId".to_string())?;
    let allow_reopen_closed =
        optional_command_or_params_bool(cmd, "allowReopenClosed").unwrap_or(false);
    let invocation = RouteBoundOpenInvocation::DurableResolution {
        handoff_id,
        allow_reopen_closed,
        attribution: RouteBoundOpenAttribution {
            caller_id: optional_command_string(cmd, "callerId"),
            service_job_id: optional_command_string(cmd, "serviceJobId"),
        },
    };
    let supervisor = RouteBoundOpenSupervisor::system(
        cmd.get("jobTimeoutMs").and_then(Value::as_u64),
        state.current_cancellation.clone(),
    );
    let mut runtime = ActionsRouteBoundOpenRuntime::new(state);
    RouteBoundOpenCoordinator::open(invocation, &mut runtime, &supervisor)
        .await?
        .into_compatibility_result()
}

pub(crate) fn ensure_remote_view_route_available_for_display(
    state: &ServiceState,
    route_id: &str,
    display_allocation_id: &str,
    browser_id: &str,
    allocation: Option<&DisplayAllocation>,
) -> Result<(), String> {
    let Some(route) = state.remote_view_routes.get(route_id) else {
        return Ok(());
    };
    if route.state == "released"
        || route.display_allocation_id.as_deref() == Some(display_allocation_id)
        || route.browser_id.as_deref() == Some(browser_id)
    {
        return Ok(());
    }
    let current_allocation_is_private = route
        .display_allocation_id
        .as_ref()
        .and_then(|id| state.display_allocations.get(id))
        .is_some_and(|allocation| allocation.display_isolation == "private_virtual_display");
    let requested_allocation_is_private = allocation
        .is_some_and(|allocation| allocation.display_isolation == "private_virtual_display");
    if current_allocation_is_private || requested_allocation_is_private {
        return Err(
            format!(
                "route_pool_contention: remote view route '{}' is already checked out to another private display allocation",
                route_id
            ),
        );
    }
    Ok(())
}

pub(crate) fn remote_view_lease_is_active(lease: &ViewerLease) -> bool {
    !matches!(
        lease.state.as_str(),
        "disconnected" | "expired" | "failed" | "released"
    )
}

pub(crate) fn push_remote_view_service_event(
    state: &mut ServiceState,
    kind: ServiceEventKind,
    timestamp: &str,
    browser_id: Option<String>,
    session_id: Option<String>,
    message: String,
    details: Value,
) -> String {
    let event_id = format!(
        "remote-view-event:{}:{}",
        service_event_kind_name(kind),
        timestamp.replace([':', '.'], "-")
    );
    state.events.push(ServiceEvent {
        id: event_id.clone(),
        timestamp: timestamp.to_string(),
        kind,
        message,
        browser_id,
        session_id,
        details: Some(details),
        ..ServiceEvent::default()
    });
    if state.events.len() > 100 {
        let excess = state.events.len() - 100;
        state.events.drain(0..excess);
    }
    event_id
}

pub(crate) fn service_remote_view_acquisition_plan_from_state(
    _cmd: &Value,
    state: &ServiceState,
    intent: &super::super::remote_view::RemoteViewOpenIntent,
    inline_route_pool_entry: Option<&RoutePoolEntry>,
    browser_id: &str,
    session_id: &str,
) -> Result<RemoteViewAcquisitionPlan, String> {
    plan_remote_view_acquisition(
        state,
        intent,
        inline_route_pool_entry,
        browser_id,
        session_id,
    )
}

pub(crate) fn remote_view_open_should_reuse_current_browser(
    acquisition_plan: &RemoteViewAcquisitionPlan,
    observation: &RouteBoundBrowserObservation,
    browser_id: &str,
    session_id: &str,
) -> bool {
    if browser_id != observation.browser_id || session_id != observation.session_id {
        return false;
    }
    if !observation.browser_present {
        return false;
    }
    acquisition_plan.decisions.iter().any(|decision| {
        decision.step == "route_pool_entry" && decision.reason == "same_owner_checked_out_route"
    })
}

pub(crate) fn remote_view_open_runtime_attach_launch_command(
    launch_command: &Value,
    observation: &RouteBoundBrowserObservation,
    intent: &super::super::remote_view::RemoteViewOpenIntent,
) -> Value {
    if observation.browser_present {
        return launch_command.clone();
    }
    let Some(target) = managed_runtime_attach_target(intent.runtime_profile.as_deref()) else {
        return launch_command.clone();
    };
    let mut command = launch_command.clone();
    if let Some(object) = command.as_object_mut() {
        object.insert("cdpPort".to_string(), json!(target.cdp_port));
        object.insert("runtimeAttachManaged".to_string(), Value::Bool(true));
    }
    command
}

pub(crate) fn inline_route_pool_entry_from_command(
    cmd: &Value,
) -> Result<Option<RoutePoolEntry>, String> {
    if let Some(entry) = command_or_params_value(cmd, "routePoolEntry") {
        return serde_json::from_value::<RoutePoolEntry>(entry.clone())
            .map(normalize_inline_route_pool_entry)
            .map(Some)
            .map_err(|err| format!("invalid routePoolEntry: {}", err));
    }
    if let Some(entries) = command_or_params_value(cmd, "routePool").and_then(Value::as_array) {
        let route_pool_entry_id = optional_command_or_params_string(cmd, "routePoolEntryId")
            .or_else(|| optional_command_or_params_string(cmd, "poolEntryId"));
        let requested_route_id = optional_command_or_params_string(cmd, "remoteViewRouteId")
            .or_else(|| optional_command_or_params_string(cmd, "routeId"))
            .or_else(|| optional_command_or_params_string(cmd, "viewStreamRouteId"));
        for entry in entries {
            let parsed = serde_json::from_value::<RoutePoolEntry>(entry.clone())
                .map(normalize_inline_route_pool_entry)
                .map_err(|err| format!("invalid routePool entry: {}", err))?;
            if route_pool_entry_id.as_deref() == Some(parsed.id.as_str())
                || requested_route_id.as_deref() == Some(parsed.route_id.as_str())
                || (route_pool_entry_id.is_none() && requested_route_id.is_none())
            {
                return Ok(Some(parsed));
            }
        }
    }
    Ok(None)
}

pub(crate) fn inline_route_pool_entries_from_command(
    cmd: &Value,
) -> Result<Vec<RoutePoolEntry>, String> {
    let mut parsed_entries = Vec::new();
    if let Some(entry) = command_or_params_value(cmd, "routePoolEntry") {
        parsed_entries.push(
            serde_json::from_value::<RoutePoolEntry>(entry.clone())
                .map(normalize_inline_route_pool_entry)
                .map_err(|err| format!("invalid routePoolEntry: {}", err))?,
        );
    }
    if let Some(entries) = command_or_params_value(cmd, "routePool").and_then(Value::as_array) {
        for entry in entries {
            parsed_entries.push(
                serde_json::from_value::<RoutePoolEntry>(entry.clone())
                    .map(normalize_inline_route_pool_entry)
                    .map_err(|err| format!("invalid routePool entry: {}", err))?,
            );
        }
    }
    let mut deduped = BTreeMap::new();
    for entry in parsed_entries {
        deduped.insert(entry.id.clone(), entry);
    }
    Ok(deduped.into_values().collect())
}

pub(crate) fn normalize_inline_route_pool_entry(mut entry: RoutePoolEntry) -> RoutePoolEntry {
    if matches!(entry.state.trim(), "" | "unknown")
        && entry.readiness.as_ref().is_some_and(|readiness| {
            readiness
                .get("state")
                .and_then(Value::as_str)
                .is_some_and(|state| state.trim() == "ready")
                || readiness_state(readiness).as_deref() == Some("ready")
        })
    {
        entry.state = "available".to_string();
    }
    entry
}

pub(crate) fn remote_view_open_persist_request_route_pool(
    repository: &LockedServiceStateRepository<super::super::service_store::JsonServiceStateStore>,
    entries: &[RoutePoolEntry],
) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    repository.mutate(|state| {
        for entry in entries {
            let mut next = entry.clone();
            if let Some(existing) = state.route_pool.get(&entry.id) {
                let existing_active = existing.current_route_allocation_id.is_some()
                    && !matches!(existing.state.as_str(), "" | "available" | "released");
                let incoming_inactive = entry.current_route_allocation_id.is_none()
                    && matches!(entry.state.as_str(), "" | "available" | "released");
                if existing_active && incoming_inactive {
                    next.state = existing.state.clone();
                    next.current_route_allocation_id = existing.current_route_allocation_id.clone();
                    next.readiness = existing.readiness.clone();
                }
            }
            state.route_pool.insert(entry.id.clone(), next);
        }
        Ok(())
    })
}

async fn execute_direct_open<R: RouteBoundOpenRuntime>(
    cmd: &Value,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
) -> Result<Value, RouteBoundOpenExecutionError> {
    let mut intent = normalize_remote_view_open_intent(cmd)?;
    let handoff_id = optional_command_string(cmd, "remoteViewHandoffId")
        .or_else(|| optional_command_string(cmd, "serviceJobId"));
    let initial_browser = supervisor
        .forward("observe_browser", runtime.observe_browser())
        .await?;
    let browser_id = intent
        .browser_id
        .clone()
        .unwrap_or_else(|| initial_browser.browser_id.clone());
    let session_id = intent
        .session_name
        .clone()
        .unwrap_or_else(|| initial_browser.session_id.clone());
    let repository = LockedServiceStateRepository::default_json()?;
    let mut service_state = repository.load_snapshot()?;
    let dry_run = remote_view_open_dry_run(cmd);
    let managed_one_time_profile = remote_view_open_ensure_managed_one_time_profile(
        &repository,
        &mut service_state,
        &mut intent,
        dry_run,
    )?;
    let effective_cmd = remote_view_open_command_with_effective_intent(cmd, &intent);
    let inline_route_pool_entries = inline_route_pool_entries_from_command(&effective_cmd)?;
    let inline_route_pool_entry = inline_route_pool_entry_from_command(&effective_cmd)?;
    for entry in &inline_route_pool_entries {
        service_state
            .route_pool
            .insert(entry.id.clone(), entry.clone());
    }
    if let Some(entry) = inline_route_pool_entry.as_ref() {
        service_state
            .route_pool
            .insert(entry.id.clone(), entry.clone());
    }
    let acquisition_plan = service_remote_view_acquisition_plan_from_state(
        &effective_cmd,
        &service_state,
        &intent,
        inline_route_pool_entry.as_ref(),
        &browser_id,
        &session_id,
    )?;
    let RouteBoundHandoffPlan {
        mut route_binding,
        launch_command,
        tab_command,
        checkout_command,
    } = route_bound_handoff_plan(&effective_cmd, &acquisition_plan, &browser_id, &session_id);
    let one_time_profile_warning =
        remote_view_open_one_time_profile_warning(&intent, &service_state);
    if dry_run {
        let operator_visible = route_bound_handoff_operator_visible(
            &route_binding,
            &browser_id,
            &session_id,
            None,
            None,
            tab_command.get("url").and_then(Value::as_str),
        );
        return Ok(planned_route_bound_handoff_response(
            RouteBoundHandoffPlannedResponseInput {
                intent: &intent,
                route_binding: &route_binding,
                acquisition_plan: &acquisition_plan,
                browser_id: &browser_id,
                session_name: &session_id,
                managed_one_time_profile: &managed_one_time_profile,
                one_time_profile_warning: &one_time_profile_warning,
                operator_visible: &operator_visible,
                launch_command: &launch_command,
                tab_command: &tab_command,
                checkout_command: &checkout_command,
            },
        ));
    }
    supervisor.ensure_forward("persist_route_pool")?;
    remote_view_open_persist_request_route_pool(&repository, &inline_route_pool_entries)?;
    let observed_at = service_remote_view_timestamp();
    supervisor.ensure_forward("reserve_acquisition")?;
    let acquisition_lease = begin_route_bound_handoff_plan_acquisition(
        &repository,
        inline_route_pool_entry.as_ref(),
        &acquisition_plan,
        &browser_id,
        &session_id,
        &observed_at,
    )?;
    let display_access_grant = match supervisor
        .forward(
            "ensure_display_access",
            runtime.ensure_display_access(DisplayAccessRequest {
                binding: route_binding.clone(),
            }),
        )
        .await
    {
        Ok(grant) => grant,
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let cleanup = route_bound_handoff_pre_launch_failure_cleanup("display_access_failed");
            let observed_at = service_remote_view_timestamp();
            let failure = route_bound_handoff_immediate_failure(
                &repository,
                RouteBoundHandoffImmediateFailureInput {
                    lease: &acquisition_lease,
                    phase: "display_access_failed",
                    error: &error_message,
                    cleanup: &cleanup,
                    observed_at: &observed_at,
                },
            )?;
            return Err(route_bound_execution_error_with_cleanup(
                error,
                &failure.summary,
            ));
        }
    };
    let reused_current_browser = remote_view_open_should_reuse_current_browser(
        &acquisition_plan,
        &initial_browser,
        &browser_id,
        &session_id,
    );
    let launch = if reused_current_browser {
        route_bound_handoff_reused_browser_launch_result(&route_binding, &browser_id, &session_id)
    } else {
        let effective_launch_command = remote_view_open_runtime_attach_launch_command(
            &launch_command,
            &initial_browser,
            &intent,
        );
        match supervisor
            .forward(
                "launch_browser",
                runtime.launch_browser(LaunchBrowserRequest {
                    command: effective_launch_command,
                }),
            )
            .await
        {
            Ok(launch) => launch,
            Err(error) => {
                let error_message = error.compatibility_message().to_string();
                let cleanup = route_bound_handoff_launch_failure_cleanup("browser_launch_failed");
                let observed_at = service_remote_view_timestamp();
                let failure = route_bound_handoff_immediate_failure(
                    &repository,
                    RouteBoundHandoffImmediateFailureInput {
                        lease: &acquisition_lease,
                        phase: "browser_launch_failed",
                        error: &error_message,
                        cleanup: &cleanup,
                        observed_at: &observed_at,
                    },
                )?;
                return Err(route_bound_execution_error_with_cleanup(
                    error,
                    &failure.summary,
                ));
            }
        }
    };
    let tab = match route_bound_open_acquire_target(
        &tab_command,
        runtime,
        supervisor,
        &service_state,
        &browser_id,
        &session_id,
        reused_current_browser,
    )
    .await
    {
        Ok(tab) => tab,
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_tab_open_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository: &repository,
                    lease: &acquisition_lease,
                    phase: failure_context.phase,
                    error: &error_message,
                    rollback_cleanup: &failure_context.cleanup,
                    launch: &launch,
                    tab: None,
                },
            )
            .await?;
            return Err(route_bound_execution_error_with_cleanup(
                error,
                &failure.summary,
            ));
        }
    };
    let focus_command = route_bound_handoff_focus_command(cmd, &tab, &session_id);
    let focus = match supervisor
        .forward(
            "focus_target",
            runtime.focus_target(FocusTargetRequest {
                command: focus_command,
            }),
        )
        .await
    {
        Ok(focus) => focus,
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_focus_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository: &repository,
                    lease: &acquisition_lease,
                    phase: failure_context.phase,
                    error: &error_message,
                    rollback_cleanup: &failure_context.cleanup,
                    launch: &launch,
                    tab: Some(&tab),
                },
            )
            .await?;
            return Err(route_bound_execution_error_with_cleanup(
                error,
                &failure.summary,
            ));
        }
    };
    let visible_window_proof = match supervisor
        .forward(
            "observe_visible_window",
            runtime.observe_visible_window(VisibleWindowRequest {
                binding: route_binding.clone(),
            }),
        )
        .await
    {
        Ok(proof) => proof,
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_visible_window_proof_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository: &repository,
                    lease: &acquisition_lease,
                    phase: failure_context.phase,
                    error: &error_message,
                    rollback_cleanup: &failure_context.cleanup,
                    launch: &launch,
                    tab: Some(&tab),
                },
            )
            .await?;
            return Err(route_bound_execution_error_with_cleanup(
                error,
                &failure.summary,
            ));
        }
    };
    let operator_access = supervisor
        .forward(
            "observe_operator_access",
            runtime.observe_operator_access(OperatorAccessRequest {
                binding: route_binding.clone(),
            }),
        )
        .await?;
    route_binding = route_binding_with_operator_access(route_binding, operator_access);
    let operator_visible = route_bound_handoff_operator_visible(
        &route_binding,
        &browser_id,
        &session_id,
        Some(&visible_window_proof),
        Some(&tab),
        tab_command.get("url").and_then(Value::as_str),
    );
    if let Some(handoff_failure) = route_bound_handoff_operator_visible_failure_if_not_ready(
        &route_binding,
        &browser_id,
        &session_id,
        &operator_visible,
        Some(&tab),
        tab_command.get("url").and_then(Value::as_str),
    ) {
        let failure = remote_view_open_rollback_failure_after_cleanup(
            runtime,
            supervisor,
            RemoteViewOpenFailureCleanupInput {
                repository: &repository,
                lease: &acquisition_lease,
                phase: "proof_failed",
                error: &handoff_failure.error,
                rollback_cleanup: &handoff_failure.cleanup,
                launch: &launch,
                tab: Some(&tab),
            },
        )
        .await?;
        return Err(format!("{}; cleanup={}", handoff_failure.error, failure.summary).into());
    }
    let checkout_command = route_bound_handoff_checkout_command_with_visible_window_proof(
        &checkout_command,
        &visible_window_proof,
    );
    let checkout = match supervisor
        .forward(
            "checkout_route",
            runtime.checkout_route(CheckoutRouteRequest {
                command: checkout_command.clone(),
            }),
        )
        .await
    {
        Ok(checkout) => checkout,
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_checkout_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository: &repository,
                    lease: &acquisition_lease,
                    phase: failure_context.phase,
                    error: &error_message,
                    rollback_cleanup: &failure_context.cleanup,
                    launch: &launch,
                    tab: Some(&tab),
                },
            )
            .await?;
            return Err(route_bound_execution_error_with_cleanup(
                error,
                &failure.summary,
            ));
        }
    };
    let post_checkout = route_bound_handoff_post_checkout_proof(
        RouteBoundHandoffPostCheckoutProofInput {
            planned_route_binding: &route_binding,
            checkout: &checkout,
            browser_id: &browser_id,
            session_name: &session_id,
            pre_checkout_operator_visible: &operator_visible,
            tab: Some(&tab),
            expected_url: tab_command.get("url").and_then(Value::as_str),
        },
        |final_route_binding| {
            let final_route_binding = route_binding_with_operator_access(
                final_route_binding.clone(),
                route_binding
                    .readiness
                    .as_ref()
                    .and_then(|readiness| readiness.get("operatorAccess"))
                    .cloned(),
            );
            route_bound_handoff_operator_visible(
                &final_route_binding,
                &browser_id,
                &session_id,
                Some(&visible_window_proof),
                Some(&tab),
                tab_command.get("url").and_then(Value::as_str),
            )
        },
    );
    if let Some(handoff_failure) = post_checkout.failure.as_ref() {
        let failure = remote_view_open_rollback_failure_after_cleanup(
            runtime,
            supervisor,
            RemoteViewOpenFailureCleanupInput {
                repository: &repository,
                lease: &acquisition_lease,
                phase: "final_proof_failed",
                error: &handoff_failure.error,
                rollback_cleanup: &handoff_failure.cleanup,
                launch: &launch,
                tab: Some(&tab),
            },
        )
        .await?;
        return Err(format!("{}; cleanup={}", handoff_failure.error, failure.summary).into());
    }
    let observed_at = service_remote_view_timestamp();
    supervisor.ensure_forward("finalize_open")?;
    Ok(complete_route_bound_handoff_open(
        CompleteRouteBoundHandoffOpenInput {
            handoff_id: handoff_id.as_deref(),
            intent: &intent,
            planned_route_binding: &route_binding,
            acquisition_plan: &acquisition_plan,
            repository: &repository,
            lease: &acquisition_lease,
            observed_at: &observed_at,
            browser_id: &browser_id,
            session_name: &session_id,
            managed_one_time_profile: &managed_one_time_profile,
            one_time_profile_warning: &one_time_profile_warning,
            final_operator_visible: &post_checkout.final_operator_visible,
            pre_checkout_operator_visible: &operator_visible,
            launch_command: &launch_command,
            launch: &launch,
            tab: &tab,
            focus: &focus,
            checkout: &checkout,
            display_access_grant: &display_access_grant,
            reused_current_browser,
            visible_window_proof: &visible_window_proof,
        },
    )?)
}

/// Resolve an opaque remote-view handoff by reacquiring ephemeral route state
/// and preferring the originally retained browser target when it still exists.
async fn execute_durable_resolution<R: RouteBoundOpenRuntime>(
    cmd: &Value,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
) -> Result<Value, RouteBoundOpenExecutionError> {
    let handoff_id = optional_command_or_params_string(cmd, "handoffId")
        .or_else(|| optional_command_or_params_string(cmd, "remoteViewHandoffId"))
        .ok_or_else(|| "service_remote_view_handoff_resolve requires handoffId".to_string())?;
    let allow_reopen_closed =
        optional_command_or_params_bool(cmd, "allowReopenClosed").unwrap_or(false);
    let repository = LockedServiceStateRepository::default_json()?;
    let service_state = repository.load_snapshot()?;
    let Some(handoff) = service_state.remote_view_handoffs.get(&handoff_id).cloned() else {
        return Ok(json!(
            { "status" : "not_found", "resolved" : false, "handoffId" : handoff_id,
            "message" : "Remote-view handoff was not found", }
        ));
    };
    if !allow_reopen_closed && remote_view_handoff_was_explicitly_closed(&service_state, &handoff) {
        return Ok(json!(
            { "status" : "closed", "resolved" : false, "reopenRequired" : true,
            "handoffId" : handoff.id, "handoffUrl" : handoff.handoff_url, "browserId"
            : handoff.browser_id, "sessionName" : handoff.session_name, "tabId" :
            handoff.tab_id, "targetId" : handoff.target_id, "viewStreamProvider" :
            handoff.view_stream_provider, "controlInput" : handoff.control_input,
            "message" :
            "The retained tab was deliberately closed. Reopen requires an explicit operator action.",
            }
        ));
    }
    let service_job_id = optional_command_string(cmd, "serviceJobId")
        .unwrap_or_else(|| format!("resolve:{}", handoff.id));
    let mut resolution_command =
        remote_view_handoff_resolution_command(&handoff, &service_job_id, allow_reopen_closed)?;
    apply_retained_remote_view_route(&service_state, &handoff, &mut resolution_command);
    let opened = match execute_direct_open(&resolution_command, runtime, supervisor).await {
        Ok(opened) => opened,
        Err(error) => {
            if let Some(fallback) = typed_remote_view_handoff_provider_fallback(
                &service_state,
                &handoff,
                error.runtime_issue.as_ref(),
                runtime,
                supervisor,
            )
            .await
            {
                return Ok(fallback);
            }
            return Err(error);
        }
    };
    Ok(json!(
        { "status" : "ready", "resolved" : true, "reopenedClosedTab" :
        allow_reopen_closed, "handoffId" : handoff.id, "handoffUrl" : opened
        .get("handoffUrl"), "externalUrl" : opened.get("externalUrl"),
        "providerExternalUrl" : opened.get("providerExternalUrl"), "browserId" :
        opened.get("browserId"), "sessionName" : opened.get("sessionName"), "tab" :
        opened.get("tab"), "viewStreamProvider" : handoff.view_stream_provider,
        "controlInput" : handoff.control_input, "open" : opened, }
    ))
}

async fn typed_remote_view_handoff_provider_fallback<R: RouteBoundOpenRuntime>(
    service_state: &ServiceState,
    handoff: &RemoteViewHandoff,
    issue: Option<&RouteBoundRuntimeIssue>,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
) -> Option<Value> {
    let RouteBoundRuntimeIssue::RequestedProfileInUseByPid {
        profile_id,
        owner_browser_id,
        owner_session_id,
        ..
    } = issue?
    else {
        return None;
    };
    if handoff.view_stream_provider != Some(ViewStreamProvider::RdpGateway)
        || handoff.profile_id.as_deref() != Some(profile_id.as_str())
        || owner_browser_id.as_deref() != handoff.browser_id.as_deref()
        || owner_session_id.as_deref() != handoff.session_name.as_deref()
    {
        return None;
    }
    let route = handoff
        .last_route_id
        .as_ref()
        .and_then(|route_id| service_state.remote_view_routes.get(route_id))?;
    if route.browser_id.as_deref() != handoff.browser_id.as_deref()
        || route.session_id.as_deref() != handoff.session_name.as_deref()
        || route.state == "released"
    {
        return None;
    }
    let display_allocation_id = route
        .display_allocation_id
        .clone()
        .or_else(|| handoff.last_display_allocation_id.clone())
        .unwrap_or_default();
    let allocation = service_state
        .display_allocations
        .get(&display_allocation_id);
    let binding = RemoteViewRouteBinding {
        route_id: route.id.clone(),
        route_pool_entry_id: handoff.last_route_pool_entry_id.clone(),
        display_allocation_id,
        route_pool_entry_state: None,
        current_route_allocation_id: None,
        display_name: allocation.and_then(|allocation| allocation.display_name.clone()),
        launch_display_name: allocation.and_then(|allocation| allocation.display_name.clone()),
        display_isolation: allocation
            .map(|allocation| allocation.display_isolation.clone())
            .unwrap_or_default(),
        route_user: None,
        display_access: None,
        provider: route.provider,
        provider_mode: route.provider_mode.clone(),
        connection_id: route.connection_id.clone(),
        connection_name: route.connection_name.clone(),
        frame_url: route.frame_url.clone(),
        external_url: route.external_url.clone(),
        route_descriptor: route.route_descriptor.clone(),
        readiness: route.readiness.clone(),
    };
    let operator_access = supervisor
        .forward(
            "observe_operator_access",
            runtime.observe_operator_access(OperatorAccessRequest { binding }),
        )
        .await
        .ok()
        .flatten()?;
    if readiness_state(&operator_access).as_deref() != Some("ready") {
        return None;
    }
    remote_view_handoff_provider_fallback_response(service_state, handoff)
}

pub(crate) fn remote_view_handoff_provider_fallback(
    service_state: &ServiceState,
    handoff: &RemoteViewHandoff,
    error: &str,
) -> Option<Value> {
    if handoff.view_stream_provider != Some(ViewStreamProvider::RdpGateway)
        || !error.contains("already in use by PID")
    {
        return None;
    }
    remote_view_handoff_provider_fallback_response(service_state, handoff)
}

fn remote_view_handoff_provider_fallback_response(
    service_state: &ServiceState,
    handoff: &RemoteViewHandoff,
) -> Option<Value> {
    let route = handoff
        .last_route_id
        .as_ref()
        .and_then(|route_id| service_state.remote_view_routes.get(route_id))?;
    let provider_url = route
        .external_url
        .as_deref()
        .or_else(|| {
            route
                .route_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.get("publicOperatorUrl"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|url| !url.is_empty())?;
    let tab = json!(
        { "id" : handoff.tab_id, "tabId" : handoff.tab_id, "targetId" : handoff
        .target_id, "browserId" : handoff.browser_id, "sessionId" : handoff.session_name,
        "profileId" : handoff.profile_id, }
    );
    Some(json!(
        { "status" : "ready", "resolved" : true, "bestEffort" : true,
        "providerFallback" : true, "providerFallbackUrl" : provider_url, "handoffId"
        : handoff.id, "handoffUrl" : handoff.handoff_url, "browserId" : handoff
        .browser_id, "sessionName" : handoff.session_name, "tabId" : handoff.tab_id,
        "targetId" : handoff.target_id, "tab" : tab, "viewStreamProvider" : handoff
        .view_stream_provider, "controlInput" : handoff.control_input, "externalUrl"
        : provider_url, "providerExternalUrl" : provider_url, "message" :
        "The original browser daemon is unavailable, but its retained RDP provider route is still available for a best-effort reconnect.",
        "open" : { "browserId" : handoff.browser_id, "sessionName" : handoff
        .session_name, "tab" : tab, "externalUrl" : provider_url,
        "providerExternalUrl" : provider_url, "route" : route, "intent" : handoff
        .intent, "operatorVisible" : { "state" : "best_effort", "reason" :
        "live_profile_owned_outside_original_daemon", }, }, }
    ))
}

pub(crate) fn remote_view_open_ensure_managed_one_time_profile(
    repository: &LockedServiceStateRepository<super::super::service_store::JsonServiceStateStore>,
    service_state: &mut ServiceState,
    intent: &mut super::super::remote_view::RemoteViewOpenIntent,
    dry_run: bool,
) -> Result<Value, String> {
    if intent.runtime_profile.is_some() || intent.profile.is_some() {
        return Ok(Value::Null);
    }
    if !remote_view_open_looks_like_one_time_operator_handoff(intent) {
        return Ok(Value::Null);
    }
    let profile_id = remote_view_open_managed_one_time_profile_id(intent);
    intent.runtime_profile = Some(profile_id.clone());
    if let Some(profile) = service_state.profiles.get(&profile_id) {
        return Ok(json!(
            { "state" : "reused", "profileId" : profile_id, "runtimeProfile" :
            profile_id, "profileClass" : profile.profile_class, "profileOrigin" :
            profile.profile_origin, "userDataDir" : profile.user_data_dir, "dryRun" :
            dry_run, }
        ));
    }
    let profile = remote_view_open_managed_one_time_profile(intent, &profile_id);
    service_state
        .entity_sources
        .profiles
        .insert(profile_id.clone(), ServiceEntitySource::PersistedState);
    service_state
        .profiles
        .insert(profile_id.clone(), profile.clone());
    if !dry_run {
        repository.mutate(|state| {
            state
                .entity_sources
                .profiles
                .insert(profile_id.clone(), ServiceEntitySource::PersistedState);
            state.profiles.insert(profile_id.clone(), profile.clone());
            Ok(())
        })?;
    }
    Ok(json!(
        { "state" : if dry_run { "planned" } else { "created" }, "profileId" :
        profile_id, "runtimeProfile" : profile_id, "profileClass" :
        ProfileClass::ManagedOneTime, "profileOrigin" :
        ProfileOrigin::AgentBrowserOwned, "userDataDir" : profile.user_data_dir,
        "persistent" : profile.persistent, "dryRun" : dry_run, }
    ))
}

pub(crate) fn remote_view_open_managed_one_time_profile(
    intent: &super::super::remote_view::RemoteViewOpenIntent,
    profile_id: &str,
) -> BrowserProfile {
    let service_ids = intent
        .service_name
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| vec![value.clone()])
        .unwrap_or_default();
    let browser_build = intent
        .browser_build
        .as_deref()
        .and_then(BrowserBuild::parse_label);
    let user_data_dir = runtime_profile_user_data_dir(profile_id)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let task_label = intent
        .task_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("operator handoff");
    BrowserProfile {
        id: profile_id.to_string(),
        name: format!("Managed one-time {task_label}"),
        profile_origin: ProfileOrigin::AgentBrowserOwned,
        profile_class: ProfileClass::ManagedOneTime,
        user_data_dir,
        default_browser_host: Some(ServiceBrowserHost::RemoteHeaded),
        browser_build,
        allocation: ProfileAllocationPolicy::PerService,
        keyring: ProfileKeyringPolicy::BasicPasswordStore,
        shared_service_ids: service_ids,
        manual_login_preferred: true,
        persistent: false,
        tags: vec!["managed_one_time".to_string()],
        ..BrowserProfile::default()
    }
}

pub(crate) fn remote_view_open_command_with_effective_intent(
    cmd: &Value,
    intent: &super::super::remote_view::RemoteViewOpenIntent,
) -> Value {
    let mut command = cmd.clone();
    if !command.is_object() {
        command = json!({});
    }
    if let Some(map) = command.as_object_mut() {
        if let Some(runtime_profile) = intent.runtime_profile.as_deref() {
            map.insert(
                "runtimeProfile".to_string(),
                Value::String(runtime_profile.to_string()),
            );
        }
        if let Some(profile) = intent.profile.as_deref() {
            map.insert("profile".to_string(), Value::String(profile.to_string()));
        }
    }
    command
}

pub(crate) fn remote_view_open_one_time_profile_warning(
    intent: &super::super::remote_view::RemoteViewOpenIntent,
    service_state: &ServiceState,
) -> Value {
    let Some(runtime_profile) = intent.runtime_profile.as_deref() else {
        return Value::Null;
    };
    if service_state.profiles.contains_key(runtime_profile) {
        return Value::Null;
    }
    if !remote_view_open_looks_like_one_time_operator_handoff(intent) {
        return Value::Null;
    }
    let recommended_profile_id = remote_view_open_managed_one_time_profile_id(intent);
    json!(
        { "state" : "warning", "code" : "arbitrary_runtime_profile_for_one_time_handoff",
        "requestedRuntimeProfile" : runtime_profile, "profileClass" :
        "operator_supplied", "recommendedProfileClass" : "managed_one_time",
        "recommendedProfileId" : recommended_profile_id, "message" :
        "This looks like a one-time operator handoff but it supplied a new arbitrary runtime profile. Prefer the managed one-time task profile so retries reuse one lane and cleanup can remove abandoned task state safely.",
        }
    )
}

pub(crate) fn remote_view_open_looks_like_one_time_operator_handoff(
    intent: &super::super::remote_view::RemoteViewOpenIntent,
) -> bool {
    if intent.view_stream_provider != ViewStreamProvider::RdpGateway {
        return false;
    }
    let manual_control = intent.control_input == "manual_attached_desktop";
    let remote_headed = intent.browser_host == "remote_headed";
    let text = [
        intent.service_name.as_deref(),
        intent.agent_name.as_deref(),
        intent.task_name.as_deref(),
        intent.url.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    let one_time_hint = [
        "temporary",
        "temp",
        "one-time",
        "one_time",
        "login",
        "payment",
        "challenge",
        "sosdirect",
        "templogin",
    ]
    .iter()
    .any(|needle| text.contains(needle));
    manual_control && remote_headed && one_time_hint
}

pub(crate) fn remote_view_open_managed_one_time_profile_id(
    intent: &super::super::remote_view::RemoteViewOpenIntent,
) -> String {
    let seed = [
        intent.service_name.as_deref().unwrap_or("service"),
        intent.agent_name.as_deref().unwrap_or("agent"),
        intent.task_name.as_deref().unwrap_or("task"),
        intent.url.as_deref().unwrap_or("url"),
    ]
    .join("|")
    .to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("managed-one-time-{suffix}")
}

pub(crate) async fn remote_view_open_cleanup_after_failure<R: RouteBoundOpenRuntime>(
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    task: &RouteBoundHandoffFailureCleanupTask,
    created_target_id: Option<&str>,
) -> Value {
    let result = match task {
        RouteBoundHandoffFailureCleanupTask::CloseOpenedTab { .. } => match created_target_id {
            Some(target_id) => {
                supervisor
                    .compensate(
                        "close_created_target",
                        runtime.close_created_target(CloseCreatedTargetRequest {
                            target_id: target_id.to_string(),
                        }),
                    )
                    .await
            }
            None => Err(route_bound_runtime_issue(
                "close_created_target",
                "rollback_incomplete: created target identity was unavailable".to_string(),
                None,
            )),
        },
        RouteBoundHandoffFailureCleanupTask::CloseNewBrowser { command } => {
            supervisor
                .compensate(
                    "close_created_browser",
                    runtime.close_created_browser(CloseCreatedBrowserRequest {
                        browser_identity: command.clone(),
                    }),
                )
                .await
        }
        RouteBoundHandoffFailureCleanupTask::Skipped { cleanup } => {
            return cleanup.clone();
        }
    };
    route_bound_handoff_failure_cleanup_task_result(
        task,
        result.map_err(|issue| issue.compatibility_message().to_string()),
    )
}

pub(crate) struct RemoteViewOpenFailureCleanupInput<'a> {
    pub(crate) repository:
        &'a LockedServiceStateRepository<super::super::service_store::JsonServiceStateStore>,
    pub(crate) lease: &'a RemoteViewAcquisitionLease,
    pub(crate) phase: &'a str,
    pub(crate) error: &'a str,
    pub(crate) rollback_cleanup: &'a Value,
    pub(crate) launch: &'a Value,
    pub(crate) tab: Option<&'a Value>,
}

pub(crate) async fn remote_view_open_rollback_failure_after_cleanup<R: RouteBoundOpenRuntime>(
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    input: RemoteViewOpenFailureCleanupInput<'_>,
) -> Result<RouteBoundHandoffFailureCleanupSummary, String> {
    let now = service_remote_view_timestamp();
    let recovery = begin_route_bound_handoff_failure_recovery(
        input.repository,
        RouteBoundHandoffFailureRecoveryInput {
            lease: input.lease,
            phase: input.phase,
            error: input.error,
            rollback_cleanup: input.rollback_cleanup,
            launch: input.launch,
            tab: input.tab,
            observed_at: &now,
        },
    )?;
    let created_target_id = input
        .tab
        .and_then(|tab| tab.get("targetId"))
        .and_then(Value::as_str);
    let cleanup = remote_view_open_cleanup_after_failure(
        runtime,
        supervisor,
        &recovery.cleanup_task,
        created_target_id,
    )
    .await;
    remote_view_open_complete_handoff_failure_cleanup(
        input.repository,
        &input.lease.id,
        &recovery.rollback,
        &cleanup,
    )
}

pub(crate) fn remote_view_open_complete_handoff_failure_cleanup(
    repository: &LockedServiceStateRepository<super::super::service_store::JsonServiceStateStore>,
    lease_id: &str,
    rollback: &Value,
    cleanup: &Value,
) -> Result<RouteBoundHandoffFailureCleanupSummary, String> {
    let now = service_remote_view_timestamp();
    complete_route_bound_handoff_failure_cleanup(
        repository,
        RouteBoundHandoffFailureCleanupInput {
            lease_id,
            rollback,
            cleanup,
            observed_at: &now,
        },
    )
}

pub(crate) fn remote_view_open_ensure_display_access(
    route_binding: &super::super::remote_view::RemoteViewRouteBinding,
) -> Result<Value, String> {
    let Some(display_name) = route_binding
        .launch_display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(format!(
            "route_display_missing: route '{}' has no launch display",
            route_binding.route_id
        ));
    };
    let initial_probe = remote_view_open_display_access_probe(display_name);
    if initial_probe
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(json!(
            { "state" : "already_ready", "displayName" : display_name, "probe" :
            initial_probe, }
        ));
    }
    let route_user = route_binding
        .route_user
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "x11_auth_denied: route '{}' display '{}' is not accessible and no route user was reported",
                route_binding.route_id, display_name
            )
        })?;
    let operator_user = env::var("AGENT_BROWSER_RDP_DISPLAY_ACCESS_USER")
        .or_else(|_| env::var("USER"))
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty() && value != "root")
        .ok_or_else(|| {
            format!(
                "display_access_grant_failed: route '{}' display '{}' cannot infer non-root operator user",
                route_binding.route_id, display_name
            )
        })?;
    let helper_path = env::var("AGENT_BROWSER_PRIVILEGED_HELPER").unwrap_or_else(|_| {
        "/usr/local/libexec/agent-browser/agent-browser-privileged-helper".to_string()
    });
    let status = Command::new("timeout")
        .args([
            "--kill-after=1",
            REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS,
            "sudo",
            "-n",
            &helper_path,
            "grant-display-access",
            "--operator-user",
            &operator_user,
            "--route-user",
            route_user,
            "--display",
            display_name,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| {
            format!(
                "display_access_grant_failed: route '{}' display '{}' bounded helper could not start: {}",
                route_binding.route_id, display_name, err
            )
        })?;
    if !status.success() {
        return Err(remote_view_display_access_grant_error(
            &route_binding.route_id,
            display_name,
            status.code().unwrap_or(-1),
            "",
        ));
    }
    let final_probe = remote_view_open_display_access_probe(display_name);
    if final_probe
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(json!(
            { "state" : "granted", "displayName" : display_name, "operatorUser" :
            operator_user, "routeUser" : route_user, "helperPath" : helper_path,
            "helperTimeout" : REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS,
            "probe" : final_probe, }
        ));
    }
    Err(format!(
        "x11_auth_denied: route '{}' display '{}' remained inaccessible after display access grant",
        route_binding.route_id, display_name
    ))
}

pub(crate) fn remote_view_display_access_grant_error(
    route_id: &str,
    display_name: &str,
    exit_code: i32,
    stderr: &str,
) -> String {
    let stderr_suffix = if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    };
    if matches!(exit_code, 124 | 137) {
        return format!(
            "display_access_grant_timeout: route '{}' display '{}' helper exceeded {}{}",
            route_id, display_name, REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS, stderr_suffix
        );
    }
    format!(
        "display_access_grant_failed: route '{}' display '{}' helper exited with {}{}",
        route_id, display_name, exit_code, stderr_suffix
    )
}

pub(crate) fn remote_view_open_display_access_probe(display_name: &str) -> Value {
    match Command::new("timeout")
        .args(["--kill-after=1", "2", "xdpyinfo"])
        .env("DISPLAY", display_name)
        .output()
    {
        Ok(output) => {
            json!(
                { "available" : true, "success" : output.status.success(), "exitCode" :
                output.status.code(), "stdout" : String::from_utf8_lossy(& output.stdout)
                .lines().find(| line | line.trim_start().starts_with("name of display:"))
                .unwrap_or("").trim(), "stderr" : String::from_utf8_lossy(& output
                .stderr).trim().chars().take(240).collect::< String > (), }
            )
        }
        Err(error) => {
            json!(
                { "available" : false, "success" : false, "exitCode" : null, "stdout" :
                "", "stderr" : error.to_string(), }
            )
        }
    }
}

pub(crate) fn remote_view_open_dry_run(cmd: &Value) -> bool {
    cmd.get("dryRun")
        .and_then(Value::as_bool)
        .or_else(|| {
            cmd.get("params")
                .and_then(|params| params.get("dryRun"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

pub(crate) async fn remote_view_open_operator_access_readiness(
    route_binding: &RemoteViewRouteBinding,
) -> Option<Value> {
    let probe_url = remote_view_operator_access_probe_url(route_binding)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .ok()?;
    let started_at = Instant::now();
    let response = client.get(&probe_url).send().await;
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    Some(match response {
        Ok(response) => {
            let status = response.status().as_u16();
            let state = if response.status().is_success() || response.status().is_redirection() {
                "ready"
            } else if matches!(status, 401 | 403) {
                "auth_expired"
            } else if matches!(status, 500 | 502 | 503 | 504) {
                "proxy_failed"
            } else {
                "public_operator_unavailable"
            };
            json!(
                { "state" : state, "url" : probe_url, "httpStatus" : status,
                "elapsedMs" : elapsed_ms, "reason" : if state == "ready" {
                "public operator URL responded" } else {
                "public operator URL did not return a usable response" }, }
            )
        }
        Err(error) => {
            let state = if error.is_timeout() {
                "timed_out"
            } else if error.is_connect() {
                "proxy_failed"
            } else {
                "public_operator_unavailable"
            };
            json!(
                { "state" : state, "url" : probe_url, "httpStatus" : null,
                "elapsedMs" : elapsed_ms, "reason" : error.to_string(), }
            )
        }
    })
}

pub(crate) fn route_binding_with_operator_access(
    mut route_binding: RemoteViewRouteBinding,
    operator_access: Option<Value>,
) -> RemoteViewRouteBinding {
    let Some(operator_access) = operator_access else {
        return route_binding;
    };
    let mut readiness = route_binding
        .readiness
        .take()
        .unwrap_or_else(|| route_binding_readiness(&route_binding));
    if !readiness.is_object() {
        readiness = json!(
            { "state" : readiness_state(& readiness).unwrap_or_else(|| "ready"
            .to_string()), "previous" : readiness, }
        );
    }
    if let Some(record) = readiness.as_object_mut() {
        record.insert("operatorAccess".to_string(), operator_access);
    }
    route_binding.readiness = Some(readiness);
    route_binding
}

pub(crate) fn remote_view_operator_access_probe_url(
    route_binding: &RemoteViewRouteBinding,
) -> Option<String> {
    for key in [
        "dashboardEmbedUrl",
        "publicOperatorUrl",
        "externalUrl",
        "healthUrl",
    ] {
        if let Some(url) = route_descriptor_string(route_binding, key)
            .and_then(|value| remote_view_http_probe_url(&value))
        {
            return Some(url);
        }
    }
    None
}

pub(crate) fn route_descriptor_string(
    route_binding: &RemoteViewRouteBinding,
    key: &str,
) -> Option<String> {
    route_binding
        .route_descriptor
        .as_ref()
        .and_then(|descriptor| descriptor.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn remote_view_http_probe_url(value: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_fragment(None);
    Some(url.to_string())
}

pub(crate) fn remote_view_open_visible_window_proof(
    route_binding: &super::super::remote_view::RemoteViewRouteBinding,
) -> Result<Value, String> {
    let display_name = route_binding
        .launch_display_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "route_display_missing: route '{}' has no launch display",
                route_binding.route_id
            )
        })?;
    if env::var("AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE")
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return Err(
            format!(
                "forced_visible_window_proof_failure: route '{}' display '{}' proof failure requested by AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE",
                route_binding.route_id, display_name
            ),
        );
    }
    let timeout = Duration::from_secs(10);
    let interval = Duration::from_millis(500);
    let started_at = Instant::now();
    let mut attempts = 0_u32;
    loop {
        attempts += 1;
        let display_content = route_bound_display_content(display_name).unwrap_or_else(|| {
            json!(
                { "state" : "display_probe_unavailable", "displayName" :
                display_name, "windows" : [], "error" :
                "route display probe returned no content", }
            )
        });
        match visible_browser_window_proof(
            &route_binding.route_id,
            display_name,
            display_content.clone(),
        ) {
            Ok(proof) => return Ok(proof),
            Err(error) => {
                let state = remote_view_visible_window_proof_state(&display_content);
                if !remote_view_visible_window_proof_retryable_state(state)
                    || started_at.elapsed() >= timeout
                {
                    return Err(format!(
                        "{error}; visible_window_proof_attempts={attempts}; timeoutMs={}",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(interval);
            }
        }
    }
}

pub(crate) fn remote_view_visible_window_proof_state(display_content: &Value) -> &str {
    display_content
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}

pub(crate) fn remote_view_visible_window_proof_retryable_state(state: &str) -> bool {
    matches!(
        state,
        "display_probe_unavailable" | "empty_display" | "non_browser_windows" | "unknown"
    )
}

pub(crate) fn command_object_with_action(cmd: &Value, action: &str) -> Map<String, Value> {
    let mut command = cmd.as_object().cloned().unwrap_or_default();
    command.insert("action".to_string(), Value::String(action.to_string()));
    command.remove("dryRun");
    command
}

pub(crate) async fn handle_service_remote_view_route_preflight(
    cmd: &Value,
    daemon_state: &DaemonState,
) -> Result<Value, String> {
    let observed_at = service_remote_view_timestamp();
    let browser_id = optional_command_string(cmd, "browserId")
        .unwrap_or_else(|| service_browser_id(&daemon_state.session_id));
    let session_id = optional_command_string(cmd, "sessionName")
        .unwrap_or_else(|| daemon_state.session_id.clone());
    let repository = LockedServiceStateRepository::default_json()?;
    let mut state = repository.load_snapshot()?;
    let intent = normalize_remote_view_open_intent(cmd)?;
    let inline_route_pool_entry = inline_route_pool_entry_from_command(cmd)?;
    if let Some(entry) = inline_route_pool_entry.as_ref() {
        state.route_pool.insert(entry.id.clone(), entry.clone());
    }
    let acquisition_plan = service_remote_view_acquisition_plan_from_state(
        cmd,
        &state,
        &intent,
        inline_route_pool_entry.as_ref(),
        &browser_id,
        &session_id,
    )?;
    let route_binding = acquisition_plan.route_binding.clone();
    let fast_preflight =
        remote_view_route_fast_preflight(&route_binding, &acquisition_plan, &observed_at);
    Ok(json!(
        { "status" : "preflight_ready", "preflightStatus" : fast_preflight
        .get("status").cloned().unwrap_or(Value::Null), "observedAt" : observed_at,
        "routeId" : route_binding.route_id, "displayAllocationId" : route_binding
        .display_allocation_id, "routePoolEntryId" : route_binding
        .route_pool_entry_id, "browserId" : browser_id, "sessionName" : session_id,
        "frameUrl" : route_binding.frame_url, "externalUrl" : route_binding
        .external_url, "routeDescriptor" : route_binding.route_descriptor,
        "providerMode" : route_binding.provider_mode, "routeBinding" : route_binding,
        "acquisitionPlan" : acquisition_plan, "fastPreflight" : fast_preflight, }
    ))
}

pub(crate) fn remote_view_route_fast_preflight(
    route_binding: &super::super::remote_view::RemoteViewRouteBinding,
    acquisition_plan: &RemoteViewAcquisitionPlan,
    observed_at: &str,
) -> Value {
    let route_readiness = route_binding.readiness.as_ref();
    let mut components = vec![
        remote_view_preflight_component(
            "acquisition_plan",
            if acquisition_plan.blockers.is_empty() {
                "ready"
            } else {
                "blocked"
            },
            if acquisition_plan.blockers.is_empty() {
                "acquisition planner selected a route without blockers".to_string()
            } else {
                format!(
                    "acquisition planner reported {} blocker(s)",
                    acquisition_plan.blockers.len()
                )
            },
            Some(observed_at),
            json!({ "mode" : acquisition_plan.mode,
        "selectedRoutePoolEntryId" : acquisition_plan.selected_route_pool_entry_id,
        "displayAllocationId" : acquisition_plan.display_allocation_id, "blockers" :
        acquisition_plan.blockers, }),
            None,
        ),
        remote_view_route_url_preflight_component(route_binding, observed_at),
        retained_remote_view_preflight_component(
            route_readiness,
            "guacamole_web",
            &["guacamole_web", "guacamole_web_app"],
            observed_at,
            "run_rdp_gateway_readiness",
        ),
        retained_remote_view_preflight_component(
            route_readiness,
            "guacamole_login",
            &["guacamole_login"],
            observed_at,
            "repair_guacamole_admin_credentials",
        ),
        retained_remote_view_preflight_component(
            route_readiness,
            "guacamole_connection_permissions",
            &["guacamole_connection_permissions"],
            observed_at,
            "repair_guacamole_connection_permissions",
        ),
        retained_remote_view_preflight_component(
            route_readiness,
            "rdp_backend_tcp",
            &["rdp_backend_tcp", "backend_tcp"],
            observed_at,
            "repair_rdp_backend_reachability",
        ),
        remote_view_helper_status_preflight_component(observed_at),
        remote_view_display_access_preflight_component(route_binding, observed_at),
        remote_view_route_desktop_preflight_component(route_binding, observed_at),
    ];
    let blockers = components
        .iter()
        .filter(|component| {
            component
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| matches!(status, "blocked" | "failed"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let stale = components
        .iter()
        .filter(|component| {
            component
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "stale")
        })
        .cloned()
        .collect::<Vec<_>>();
    let not_checked = components
        .iter()
        .filter(|component| {
            component
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "not_checked")
        })
        .cloned()
        .collect::<Vec<_>>();
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !stale.is_empty() {
        "stale"
    } else if !not_checked.is_empty() {
        "partial"
    } else {
        "ready"
    };
    let next_action = blockers
        .first()
        .or_else(|| stale.first())
        .or_else(|| not_checked.first())
        .and_then(|component| component.get("nextAction"))
        .and_then(Value::as_str)
        .unwrap_or("remote_view_open");
    json!(
        { "status" : status, "observedAt" : observed_at, "noLaunch" : true, "source" :
        "service_remote_view_route_preflight", "nextAction" : next_action, "components" :
        std::mem::take(& mut components), "blockers" : blockers, "stale" : stale,
        "notChecked" : not_checked, }
    )
}

pub(crate) fn remote_view_preflight_component(
    component: &str,
    status: &str,
    evidence: String,
    observed_at: Option<&str>,
    detail: Value,
    next_action: Option<&str>,
) -> Value {
    json!(
        { "component" : component, "status" : status, "evidence" : evidence, "observedAt"
        : observed_at, "freshness" : { "state" : if observed_at.is_some() {
        "observed_now" } else { "not_timestamped" }, "observedAt" : observed_at, },
        "nextAction" : next_action.unwrap_or(if status == "ready" { "none" } else {
        "inspect_remote_view_preflight" }), "detail" : detail, }
    )
}

pub(crate) fn remote_view_route_url_preflight_component(
    route_binding: &super::super::remote_view::RemoteViewRouteBinding,
    observed_at: &str,
) -> Value {
    let has_route_url = route_binding
        .frame_url
        .as_deref()
        .is_some_and(|url| url.contains("#/client/"))
        || route_binding
            .external_url
            .as_deref()
            .is_some_and(|url| url.contains("#/client/"))
        || route_binding
            .route_descriptor
            .as_ref()
            .and_then(Value::as_object)
            .is_some_and(|record| {
                [
                    "localEmbedUrl",
                    "dashboardEmbedUrl",
                    "publicOperatorUrl",
                    "externalUrl",
                    "healthUrl",
                ]
                .iter()
                .any(|key| {
                    record
                        .get(*key)
                        .and_then(Value::as_str)
                        .is_some_and(|url| url.contains("#/client/"))
                })
            });
    remote_view_preflight_component(
        "guacamole_route_url",
        if has_route_url { "ready" } else { "blocked" },
        if has_route_url {
            "selected route binding has a concrete Guacamole client URL".to_string()
        } else {
            "selected route binding has no concrete Guacamole client URL".to_string()
        },
        Some(observed_at),
        json!(
            { "frameUrl" : route_binding.frame_url, "externalUrl" : route_binding
            .external_url, "routeDescriptor" : route_binding.route_descriptor, }
        ),
        Some(if has_route_url {
            "none"
        } else {
            "repair_guacamole_route_url"
        }),
    )
}

pub(crate) fn retained_remote_view_preflight_component(
    readiness: Option<&Value>,
    output_component: &str,
    component_names: &[&str],
    observed_at: &str,
    default_next_action: &str,
) -> Value {
    let Some(component) = retained_readiness_component(readiness, component_names) else {
        return remote_view_preflight_component(
            output_component,
            "not_checked",
            format!("{output_component} has no retained readiness component"),
            Some(observed_at),
            json!(
                { "source" : "route_pool_entry.readiness", "componentNames" :
                component_names, }
            ),
            Some(default_next_action),
        );
    };
    let raw_status = component
        .get("status")
        .or_else(|| component.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = match raw_status {
        "ready" => "ready",
        "stale" | "expired" => "stale",
        "blocked" | "failed" | "missing" | "unavailable" => "blocked",
        _ => "not_checked",
    };
    let source_observed_at = component
        .get("observedAt")
        .or_else(|| component.get("checkedAt"))
        .or_else(|| component.get("lastCheckedAt"))
        .or_else(|| component.get("lastSucceededAt"))
        .and_then(Value::as_str);
    let next_action = component
        .get("nextAction")
        .and_then(Value::as_str)
        .unwrap_or(default_next_action);
    remote_view_preflight_component(
        output_component,
        status,
        component
            .get("evidence")
            .or_else(|| component.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("retained readiness component found")
            .to_string(),
        source_observed_at,
        json!(
            { "source" : "route_pool_entry.readiness", "observedByPreflightAt" :
            observed_at, "retainedComponent" : component, }
        ),
        Some(next_action),
    )
}

pub(crate) fn remote_view_helper_status_preflight_component(observed_at: &str) -> Value {
    let helper_path = env::var("AGENT_BROWSER_PRIVILEGED_HELPER").unwrap_or_else(|_| {
        "/usr/local/libexec/agent-browser/agent-browser-privileged-helper".to_string()
    });
    let report = remote_view_helper_status_probe(&helper_path);
    let ready = remote_view_helper_status_contract_ready(&report);
    remote_view_preflight_component(
        "privileged_helper_status",
        if ready { "ready" } else { "blocked" },
        if ready {
            "installed remote-view helper reports the current route desktop and display-access capability contract"
                .to_string()
        } else {
            "installed remote-view helper does not report the current route desktop and display-access capability contract"
                .to_string()
        },
        Some(observed_at),
        json!({ "helperPath" : helper_path, "statusProbe" : report, }),
        Some(if ready {
            "none"
        } else {
            "install_privileged_helper"
        }),
    )
}

pub(crate) fn remote_view_helper_status_probe(helper_path: &str) -> Value {
    let output = Command::new("timeout")
        .args([
            "--kill-after=1",
            "2s",
            "sudo",
            "-n",
            helper_path,
            "status-json",
        ])
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let mut report = json!(
                { "available" : true, "success" : output.status.success(), "timedOut" :
                matches!(output.status.code(), Some(124 | 137)), "exitCode" : output
                .status.code(), "stdout" : stdout, "stderr" : stderr, }
            );
            if !stdout.is_empty() {
                match serde_json::from_str::<Value>(&stdout) {
                    Ok(parsed) => {
                        if let Some(object) = report.as_object_mut() {
                            object.insert("parsed".to_string(), parsed);
                        }
                    }
                    Err(error) => {
                        if let Some(object) = report.as_object_mut() {
                            object.insert("parseError".to_string(), json!(error.to_string()));
                        }
                    }
                }
            }
            report
        }
        Err(error) => {
            json!(
                { "available" : false, "success" : false, "timedOut" : false, "exitCode"
                : null, "stdout" : "", "stderr" : error.to_string(), }
            )
        }
    }
}

pub(crate) fn remote_view_helper_status_contract_ready(report: &Value) -> bool {
    report.get("success").and_then(Value::as_bool) == Some(true)
        && report
            .pointer("/parsed/schemaVersion")
            .and_then(Value::as_i64)
            == Some(1)
        && report
            .pointer("/parsed/helperVersion")
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with("2026-06-23.p44-route-desktop-v"))
        && report
            .pointer("/parsed/routeDesktopSession/ready")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/routeDesktopSession/terminalStartupDetected")
            .and_then(Value::as_bool)
            == Some(false)
        && report
            .pointer("/parsed/displayAccess/supportsFilesystemX11Socket")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/displayAccess/supportsAbstractX11Socket")
            .and_then(Value::as_bool)
            == Some(true)
        && report
            .pointer("/parsed/displayAccess/boundedXhostTimeoutSeconds")
            .and_then(Value::as_i64)
            .is_some_and(|value| value > 0 && value <= 2)
}

pub(crate) fn remote_view_display_access_preflight_component(
    route_binding: &super::super::remote_view::RemoteViewRouteBinding,
    observed_at: &str,
) -> Value {
    let Some(display_name) = route_binding.launch_display_name.as_deref() else {
        return remote_view_preflight_component(
            "display_access",
            "blocked",
            "selected route has no launch display".to_string(),
            Some(observed_at),
            json!({ "routeId" : route_binding.route_id }),
            Some("repair_route_display_binding"),
        );
    };
    let probe = remote_view_open_display_access_probe(display_name);
    let status = if probe
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "ready"
    } else {
        "blocked"
    };
    remote_view_preflight_component(
        "display_access",
        status,
        if status == "ready" {
            format!("display {display_name} is accessible to agent-browser")
        } else {
            format!("display {display_name} is not accessible to agent-browser")
        },
        Some(observed_at),
        json!(
            { "displayName" : display_name, "routeUser" : route_binding.route_user,
            "retainedDisplayAccess" : route_binding.display_access, "probe" : probe, }
        ),
        Some(if status == "ready" {
            "none"
        } else {
            "grant_route_display_access"
        }),
    )
}

pub(crate) fn remote_view_route_desktop_preflight_component(
    route_binding: &super::super::remote_view::RemoteViewRouteBinding,
    observed_at: &str,
) -> Value {
    let Some(display_name) = route_binding.launch_display_name.as_deref() else {
        return remote_view_preflight_component(
            "route_desktop",
            "blocked",
            "selected route has no launch display".to_string(),
            Some(observed_at),
            json!({ "routeId" : route_binding.route_id }),
            Some("repair_route_display_binding"),
        );
    };
    let display_content = route_display_content(display_name).unwrap_or_else(|| {
        json!(
            { "state" : "display_probe_unavailable", "displayName" : display_name,
            "windows" : [], "error" : "route display probe returned no content", }
        )
    });
    let display_state = display_content
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = match display_state {
        "terminal_only" | "terminal_topmost" => "blocked",
        "display_probe_unavailable" => "not_checked",
        _ => "ready",
    };
    remote_view_preflight_component(
        "route_desktop",
        status,
        format!("route display {display_name} currently reports {display_state}"),
        Some(observed_at),
        json!(
            { "displayName" : display_name, "displayState" : display_state,
            "displayContent" : display_content, }
        ),
        Some(match status {
            "ready" => "none",
            "blocked" => "clear_route_terminal_or_restart_route_desktop",
            _ => "open_or_select_single_rdp_route_display",
        }),
    )
}

pub(crate) async fn handle_service_remote_view_browser_reattach(
    cmd: &Value,
    daemon_state: &DaemonState,
    route_switch: bool,
) -> Result<Value, String> {
    let browser_id = optional_command_or_params_string(cmd, "browserId")
        .unwrap_or_else(|| service_browser_id(&daemon_state.session_id));
    let repository = LockedServiceStateRepository::default_json()?;
    let mut snapshot = repository.load_snapshot()?;
    let inline_route_pool_entry = inline_route_pool_entry_from_command(cmd)?;
    if let Some(entry) = inline_route_pool_entry.as_ref() {
        snapshot.route_pool.insert(entry.id.clone(), entry.clone());
    }
    refresh_remote_view_attachability(&mut snapshot);
    let browser = snapshot.browsers.get(&browser_id).cloned().ok_or_else(|| {
        format!(
            "remote_view_browser_not_found: browser '{}' not found",
            browser_id
        )
    })?;
    if matches!(
        browser.health,
        ServiceBrowserHealth::NotStarted
            | ServiceBrowserHealth::ProcessExited
            | ServiceBrowserHealth::Closing
            | ServiceBrowserHealth::Faulted
    ) {
        return Err(format!(
            "remote_view_browser_not_reattachable: browser '{}' health is {:?}",
            browser_id, browser.health
        ));
    }
    let requested_stream_id = optional_command_or_params_string(cmd, "streamId");
    let stream = browser
        .view_streams
        .iter()
        .find(|stream| {
            stream.provider == ViewStreamProvider::RdpGateway
                && requested_stream_id
                    .as_deref()
                    .is_none_or(|id| stream.id == id)
        })
        .cloned();
    let requested_route_id = optional_command_or_params_string(cmd, "remoteViewRouteId")
        .or_else(|| optional_command_or_params_string(cmd, "routeId"))
        .or_else(|| optional_command_or_params_string(cmd, "viewStreamRouteId"));
    let requested_route_pool_entry_id = optional_command_or_params_string(cmd, "routePoolEntryId")
        .or_else(|| optional_command_or_params_string(cmd, "poolEntryId"));
    let controller_takeover = optional_command_or_params_bool(cmd, "controllerTakeover")
        .or_else(|| optional_command_or_params_bool(cmd, "allowControllerTakeover"))
        .unwrap_or(false);
    let selected_pool = select_browser_reattach_route_pool_entry(
        &snapshot,
        stream.as_ref(),
        requested_route_pool_entry_id.as_deref(),
        requested_route_id.as_deref(),
        route_switch,
        &browser_id,
        controller_takeover,
    );
    let selected_pool_entry = selected_pool
        .as_ref()
        .map(|selection| selection.entry.clone());
    let parked_route = selected_pool.and_then(|selection| selection.parked_route);
    let selected_route_id = requested_route_id
        .or_else(|| selected_pool_entry.as_ref().map(|entry| entry.route_id.clone()))
        .or_else(|| stream.as_ref().and_then(|stream| stream.route_id.clone()))
        .ok_or_else(|| {
            format!(
                "remote_view_route_unresolved: browser '{}' has no retained RDP route and no routePoolEntryId was provided",
                browser_id
            )
        })?;
    let route = snapshot.remote_view_routes.get(&selected_route_id).cloned();
    let previous_route_id = stream
        .as_ref()
        .and_then(|stream| stream.route_id.clone())
        .filter(|route_id| route_id != &selected_route_id);
    let previous_owned_route_id = previous_route_id
        .as_deref()
        .filter(|route_id| {
            snapshot
                .remote_view_routes
                .get(*route_id)
                .is_some_and(|route| route.browser_id.as_deref() == Some(browser_id.as_str()))
        })
        .map(str::to_string);
    let previous_route_pool_entry = previous_route_id.as_deref().and_then(|route_id| {
        snapshot
            .route_pool
            .values()
            .find(|entry| {
                entry.route_id == route_id
                    || entry.current_route_allocation_id.as_deref() == Some(route_id)
            })
            .cloned()
    });
    if route_switch {
        if let Some(previous_route_id) = previous_owned_route_id.as_deref() {
            if let Some(previous_route) = snapshot.remote_view_routes.get(previous_route_id) {
                let active_controller = previous_route
                    .controller_lease_id
                    .as_ref()
                    .and_then(|lease_id| snapshot.viewer_leases.get(lease_id))
                    .is_some_and(remote_view_lease_is_active);
                if active_controller && !controller_takeover {
                    return Err(
                        format!(
                            "remote_view_route_switch_controller_active: route '{}' has active controller lease '{}'",
                            previous_route_id, previous_route.controller_lease_id
                            .as_deref().unwrap_or("unknown")
                        ),
                    );
                }
            }
        }
    }
    let display_allocation_id = optional_command_or_params_string(cmd, "displayAllocationId")
        .or_else(|| {
            route_switch
                .then(|| {
                    route
                        .as_ref()
                        .and_then(|route| route.display_allocation_id.clone())
                        .or_else(|| {
                            selected_pool_entry
                                .as_ref()
                                .map(display_allocation_id_for_route_pool_entry)
                        })
                })
                .flatten()
        })
        .or_else(|| {
            stream
                .as_ref()
                .and_then(|stream| stream.display_allocation_id.clone())
        })
        .or_else(|| browser.display_allocation_id.clone())
        .or_else(|| {
            route
                .as_ref()
                .and_then(|route| route.display_allocation_id.clone())
        })
        .ok_or_else(|| {
            format!(
                "remote_view_display_unresolved: browser '{}' has no retained display allocation",
                browser_id
            )
        })?;
    let session_name = optional_command_or_params_string(cmd, "sessionName")
        .or_else(|| browser.active_session_ids.first().cloned())
        .unwrap_or_else(|| daemon_state.session_id.clone());
    let stream_id = requested_stream_id
        .or_else(|| stream.as_ref().map(|stream| stream.id.clone()))
        .unwrap_or_else(|| "remote-headed-view".to_string());
    let mut checkout = command_object_with_action(cmd, "service_remote_view_route_checkout");
    checkout.insert("browserId".to_string(), Value::String(browser_id.clone()));
    checkout.insert(
        "sessionName".to_string(),
        Value::String(session_name.clone()),
    );
    checkout.insert("streamId".to_string(), Value::String(stream_id.clone()));
    checkout.insert(
        "displayAllocationId".to_string(),
        Value::String(display_allocation_id.clone()),
    );
    checkout.insert(
        "routeId".to_string(),
        Value::String(selected_route_id.clone()),
    );
    checkout.insert(
        "provider".to_string(),
        json!(ViewStreamProvider::RdpGateway),
    );
    if let Some(entry) = selected_pool_entry.as_ref() {
        checkout.insert(
            "routePoolEntryId".to_string(),
            Value::String(entry.id.clone()),
        );
        if inline_route_pool_entry
            .as_ref()
            .is_some_and(|inline| inline.id == entry.id)
        {
            checkout.insert("routePoolEntry".to_string(), json!(entry));
        }
        merge_route_pool_entry_into_checkout(&mut checkout, entry);
    }
    if let Some(route) = route.as_ref() {
        merge_route_into_checkout(&mut checkout, route);
    }
    if let Some(stream) = stream.as_ref() {
        merge_stream_into_checkout(&mut checkout, stream);
    }
    let reattach_repair = if !route_switch {
        selected_pool_entry
            .as_ref()
            .filter(|entry| {
                entry.state == "pending"
                    && entry.current_route_allocation_id.as_deref()
                        == Some(selected_route_id.as_str())
            })
            .and_then(|entry| {
                route
                    .as_ref()
                    .filter(|route| {
                        route.browser_id.as_deref() == Some(browser_id.as_str())
                            && route.session_id.as_deref() == Some(session_name.as_str())
                    })
                    .map(|_| entry.id.clone())
            })
            .map(|entry_id| {
                let now = service_remote_view_timestamp();
                repository.mutate(|state| {
                    if let Some(entry) = state.route_pool.get_mut(&entry_id) {
                        entry.state = "available".to_string();
                        entry.current_route_allocation_id = None;
                        entry.readiness = Some(json!(
                            { "state" : "ready", "reason" :
                            "browser_reattach_reclaimed_stale_pending_route",
                            "previousRouteAllocationId" : selected_route_id, "browserId"
                            : browser_id, "sessionName" : session_name, "updatedAt" :
                            now, }
                        ));
                    }
                    Ok(json!(
                        { "status" : "repaired", "routePoolEntryId" : entry_id,
                        "routeId" : selected_route_id, "reason" :
                        "browser_reattach_reclaimed_stale_pending_route",
                        "updatedAt" : now, }
                    ))
                })
            })
            .transpose()?
    } else {
        None
    };
    let release_result = if route_switch {
        if let Some(previous_route_id) = previous_owned_route_id.as_ref() {
            Some(
                handle_service_remote_view_route_release(
                    &Value::Object(remote_view_route_release_command(
                        cmd,
                        previous_route_id,
                        true,
                    )),
                    daemon_state,
                )
                .await?,
            )
        } else {
            None
        }
    } else {
        None
    };
    let parked_release_result = if route_switch {
        if let Some(parked_route) = parked_route.as_ref() {
            Some(
                handle_service_remote_view_route_release(
                    &Value::Object(remote_view_route_release_command(
                        cmd,
                        &parked_route.route_id,
                        true,
                    )),
                    daemon_state,
                )
                .await?,
            )
        } else {
            None
        }
    } else {
        None
    };
    let checkout_command = Value::Object(checkout);
    let checkout_result =
        handle_service_remote_view_route_checkout(&checkout_command, daemon_state).await?;
    Ok(json!(
        { "status" : if route_switch { "route_switched" } else { "reattached" },
        "browserId" : browser_id, "sessionName" : session_name, "streamId" :
        stream_id, "routeId" : selected_route_id, "displayAllocationId" :
        display_allocation_id, "routePoolEntryId" : selected_pool_entry.as_ref()
        .map(| entry | entry.id.clone()), "previousRouteId" : previous_route_id,
        "previousRoutePoolEntryId" : previous_route_pool_entry.as_ref().map(| entry |
        entry.id.clone()), "newRouteId" : selected_route_id, "newRoutePoolEntryId" :
        selected_pool_entry.map(| entry | entry.id), "routeSwitchParking" :
        parked_route.map(| parking | json!({ "status" : "parked", "routeId" : parking
        .route_id, "routePoolEntryId" : parking.route_pool_entry_id, "browserId" :
        parking.browser_id, "sessionName" : parking.session_id, "controllerLeaseId" :
        parking.controller_lease_id, "release" : parked_release_result, })),
        "reattachRepair" : reattach_repair, "routeSwitchRelease" : release_result,
        "checkout" : checkout_result, }
    ))
}

pub(crate) fn remote_view_route_release_command(
    cmd: &Value,
    route_id: &str,
    park_for_route_switch: bool,
) -> Map<String, Value> {
    let mut release = Map::new();
    release.insert(
        "action".to_string(),
        Value::String("service_remote_view_route_release".to_string()),
    );
    release.insert("routeId".to_string(), Value::String(route_id.to_string()));
    if park_for_route_switch {
        release.insert("parkForRouteSwitch".to_string(), Value::Bool(true));
    }
    if let Some(service_name) = optional_command_string(cmd, "serviceName") {
        release.insert("serviceName".to_string(), Value::String(service_name));
    }
    if let Some(agent_name) = optional_command_string(cmd, "agentName") {
        release.insert("agentName".to_string(), Value::String(agent_name));
    }
    if let Some(task_name) = optional_command_string(cmd, "taskName") {
        release.insert("taskName".to_string(), Value::String(task_name));
    }
    release
}

pub(crate) async fn handle_service_remote_view_route_checkout(
    cmd: &Value,
    daemon_state: &DaemonState,
) -> Result<Value, String> {
    let browser_id = optional_command_string(cmd, "browserId")
        .unwrap_or_else(|| service_browser_id(&daemon_state.session_id));
    let session_id = optional_command_string(cmd, "sessionName")
        .unwrap_or_else(|| daemon_state.session_id.clone());
    let now = service_remote_view_timestamp();
    let repository = LockedServiceStateRepository::default_json()?;
    repository.mutate(|state| {
        let inline_route_pool_entry = inline_route_pool_entry_from_command(cmd)?;
        if let Some(entry) = inline_route_pool_entry.as_ref() {
            state.route_pool.insert(entry.id.clone(), entry.clone());
        }
        let intent = normalize_remote_view_open_intent(cmd)?;
        let acquisition_plan = service_remote_view_acquisition_plan_from_state(
            cmd,
            state,
            &intent,
            inline_route_pool_entry.as_ref(),
            &browser_id,
            &session_id,
        )?;
        let route_binding = acquisition_plan.route_binding.clone();
        let display_allocation_id = route_binding.display_allocation_id.clone();
        let existing_display_allocation = state
            .display_allocations
            .get(&display_allocation_id)
            .cloned();
        let provider = route_binding.provider;
        let control_input = optional_command_string(cmd, "controlInput")
            .or_else(|| optional_command_string(cmd, "controlInputProvider"))
            .and_then(|value| parse_control_input_provider(&value))
            .or_else(|| default_control_input_provider(provider));
        let route_id = route_binding.route_id.clone();
        ensure_remote_view_route_available_for_display(
            state,
            &route_id,
            &display_allocation_id,
            &browser_id,
            existing_display_allocation.as_ref(),
        )?;
        let connection_id = optional_command_string(cmd, "connectionId")
            .or_else(|| optional_command_string(cmd, "guacamoleConnectionId"))
            .or_else(|| route_binding.connection_id.clone());
        let connection_name = optional_command_string(cmd, "connectionName")
            .or_else(|| optional_command_string(cmd, "guacamoleConnectionName"))
            .or_else(|| route_binding.connection_name.clone());
        let frame_url = optional_command_string(cmd, "frameUrl")
            .or_else(|| optional_command_string(cmd, "remoteViewFrameUrl"))
            .or_else(|| route_binding.frame_url.clone());
        let external_url = optional_command_string(cmd, "externalUrl")
            .or_else(|| optional_command_string(cmd, "remoteViewExternalUrl"))
            .or_else(|| route_binding.external_url.clone())
            .or_else(|| frame_url.clone());
        let route_descriptor = cmd
            .get("routeDescriptor")
            .cloned()
            .or_else(|| cmd.get("route_descriptor").cloned())
            .or_else(|| route_binding.route_descriptor.clone());
        let provider_mode = optional_command_string(cmd, "providerMode")
            .unwrap_or_else(|| route_binding.provider_mode.clone());
        let route_source = if route_binding.route_pool_entry_id.is_some() {
            "pool"
        } else {
            "retained_state"
        };
        let readiness = cmd
            .get("readiness")
            .cloned()
            .or_else(|| {
                route_binding.readiness.as_ref().and_then(|readiness| {
                    readiness
                        .get("state")
                        .and_then(Value::as_str)
                        .is_some_and(|state| state == "ready")
                        .then(|| readiness.clone())
                })
            })
            .or_else(|| Some(route_binding_readiness(&route_binding)));
        let display_allocation = state
            .display_allocations
            .entry(display_allocation_id.clone())
            .or_insert_with(|| DisplayAllocation {
                id: display_allocation_id.clone(),
                owner_browser_id: Some(browser_id.clone()),
                owner_session_id: Some(session_id.clone()),
                state: "ready".to_string(),
                created_at: Some(now.clone()),
                updated_at: Some(now.clone()),
                ..DisplayAllocation::default()
            });
        display_allocation.owner_browser_id = Some(browser_id.clone());
        display_allocation.owner_session_id = Some(session_id.clone());
        display_allocation.display_name = route_binding.launch_display_name.clone();
        display_allocation.display_isolation = route_binding.display_isolation.clone();
        display_allocation.state = "ready".to_string();
        display_allocation.updated_at = Some(now.clone());
        if !display_allocation.route_ids.contains(&route_id) {
            display_allocation.route_ids.push(route_id.clone());
        }
        let route = RemoteViewRoute {
            id: route_id.clone(),
            provider,
            display_allocation_id: Some(display_allocation_id.clone()),
            browser_id: Some(browser_id.clone()),
            session_id: Some(session_id.clone()),
            route_source: route_source.to_string(),
            connection_id: connection_id.clone(),
            connection_name: connection_name.clone(),
            route_template: optional_command_string(cmd, "routeTemplate"),
            frame_url: frame_url.clone(),
            external_url: external_url.clone(),
            route_descriptor: route_descriptor.clone(),
            read_only: cmd
                .get("readOnly")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            control_input,
            provider_mode: provider_mode.clone(),
            state: "ready".to_string(),
            last_provider_event: Some("route_checked_out".to_string()),
            readiness: readiness.clone(),
            ..state
                .remote_view_routes
                .get(&route_id)
                .cloned()
                .unwrap_or_default()
        };
        state
            .remote_view_routes
            .insert(route_id.clone(), route.clone());
        if let Some(route_pool_entry_id) = route_binding.route_pool_entry_id.as_ref() {
            if let Some(entry) = inline_route_pool_entry
                .as_ref()
                .filter(|entry| entry.id == *route_pool_entry_id)
            {
                state
                    .route_pool
                    .insert(route_pool_entry_id.clone(), entry.clone());
            }
            if let Some(entry) = state.route_pool.get_mut(route_pool_entry_id) {
                entry.state = "checked_out".to_string();
                entry.current_route_allocation_id = Some(route_id.clone());
                entry.readiness = readiness.clone();
            }
        }
        if let Some(browser) = state.browsers.get_mut(&browser_id) {
            browser.display_allocation_id = Some(display_allocation_id.clone());
            browser.active_session_ids.push(session_id.clone());
            browser.active_session_ids.sort();
            browser.active_session_ids.dedup();
            upsert_remote_view_stream_for_route(
                browser,
                cmd,
                &route,
                &display_allocation_id,
                &frame_url,
                &external_url,
            );
        }
        let route_pool_entry = route_binding
            .route_pool_entry_id
            .as_ref()
            .and_then(|id| state.route_pool.get(id).cloned())
            .or_else(|| {
                optional_command_string(cmd, "routePoolEntryId")
                    .or_else(|| optional_command_string(cmd, "poolEntryId"))
                    .and_then(|id| state.route_pool.get(&id).cloned())
            });
        refresh_remote_view_attachability(state);
        let browser_attachability = state
            .browsers
            .get(&browser_id)
            .and_then(|browser| browser.attachability.clone());
        let stream_attachability = state.browsers.get(&browser_id).and_then(|browser| {
            browser
                .view_streams
                .iter()
                .find(|stream| stream.route_id.as_deref() == Some(route_id.as_str()))
                .and_then(|stream| stream.attachability.clone())
        });
        Ok(json!(
            { "status" : "checked_out", "routeId" : route_id, "remoteViewRouteId"
            : route.id, "displayAllocationId" : display_allocation_id,
            "routePoolEntryId" : route_binding.route_pool_entry_id, "browserId" :
            browser_id, "sessionName" : session_id, "frameUrl" : route.frame_url,
            "externalUrl" : route.external_url, "routeDescriptor" : route
            .route_descriptor, "routeBinding" : route_binding, "acquisitionPlan"
            : acquisition_plan, "providerMode" : route.provider_mode,
            "remoteViewRoute" : route, "routePoolEntry" : route_pool_entry,
            "attachability" : browser_attachability, "viewStreamAttachability" :
            stream_attachability, "updatedAt" : now, }
        ))
    })
}

pub(crate) async fn handle_service_remote_view_route_release(
    cmd: &Value,
    _daemon_state: &DaemonState,
) -> Result<Value, String> {
    let route_id = required_remote_view_route_id(cmd)?;
    let now = service_remote_view_timestamp();
    let repository = LockedServiceStateRepository::default_json()?;
    repository.mutate(|state| {
        let park_for_route_switch = optional_command_or_params_bool(cmd, "parkForRouteSwitch")
            .or_else(|| optional_command_or_params_bool(cmd, "releaseDisplayAllocation"))
            .unwrap_or(false);
        let route = state
            .remote_view_routes
            .get_mut(&route_id)
            .ok_or_else(|| format!("remote view route '{}' not found", route_id))?;
        route.state = "released".to_string();
        route.last_provider_event = Some("route_released".to_string());
        let display_allocation_id = route.display_allocation_id.clone();
        let browser_id = route.browser_id.clone();
        let session_id = route.session_id.clone();
        let viewer_lease_ids = route.viewer_lease_ids.clone();
        route.controller_lease_id = None;
        for lease_id in &viewer_lease_ids {
            if let Some(lease) = state.viewer_leases.get_mut(lease_id) {
                lease.state = "disconnected".to_string();
                lease.last_viewer_event = Some("disconnected".to_string());
                lease.updated_at = Some(now.clone());
                lease.last_heartbeat_at = Some(now.clone());
            }
        }
        for entry in state.route_pool.values_mut() {
            if entry.current_route_allocation_id.as_deref() == Some(route_id.as_str()) {
                entry.state = "available".to_string();
                entry.current_route_allocation_id = None;
                if entry
                    .readiness
                    .as_ref()
                    .and_then(|readiness| readiness.get("state"))
                    .and_then(Value::as_str)
                    .is_some_and(|state| state == "pending")
                {
                    entry.readiness = Some(json!(
                        { "state" : "ready", "reason" : "route_released",
                        "previousRouteAllocationId" : route_id, "updatedAt" : now, }
                    ));
                }
            }
        }
        if let Some(display_allocation_id) = display_allocation_id.as_ref() {
            if let Some(allocation) = state.display_allocations.get_mut(display_allocation_id) {
                allocation.route_ids.retain(|id| id != &route_id);
                allocation.updated_at = Some(now.clone());
                if park_for_route_switch {
                    allocation.state = "released".to_string();
                    allocation.readiness = Some(json!(
                        { "state" : "released", "reason" : "route_switch_parking",
                        "previousRouteAllocationId" : route_id,
                        "previousOwnerBrowserId" : browser_id.clone(),
                        "previousOwnerSessionId" : session_id.clone(), "updatedAt" :
                        now, }
                    ));
                }
            }
        }
        if let Some(browser_id) = browser_id.as_ref() {
            if let Some(browser) = state.browsers.get_mut(browser_id) {
                for stream in &mut browser.view_streams {
                    if stream.route_id.as_deref() == Some(route_id.as_str()) {
                        stream.viewer_lease_ids.clear();
                        stream.controller_lease_id = None;
                        stream.remote_readiness =
                            Some(json!({ "state" : "released", "updatedAt" : now, }));
                    }
                }
            }
        }
        push_remote_view_service_event(
            state,
            ServiceEventKind::RouteReleased,
            &now,
            browser_id.clone(),
            session_id,
            format!("Remote view route '{}' released", route_id),
            json!(
                { "routeId" : route_id, "displayAllocationId" :
                display_allocation_id, "releasedViewerLeaseIds" : viewer_lease_ids,
                "parkForRouteSwitch" : park_for_route_switch, }
            ),
        );
        refresh_remote_view_attachability(state);
        Ok(json!(
            { "status" : "released", "routeId" : route_id, "remoteViewRoute" :
            state.remote_view_routes.get(& route_id), "releasedViewerLeaseIds" :
            viewer_lease_ids, "parkForRouteSwitch" : park_for_route_switch,
            "updatedAt" : now, }
        ))
    })
}

pub(crate) fn required_remote_view_route_id(cmd: &Value) -> Result<String, String> {
    optional_command_string(cmd, "remoteViewRouteId")
        .or_else(|| optional_command_string(cmd, "routeId"))
        .or_else(|| optional_command_string(cmd, "viewStreamRouteId"))
        .ok_or_else(|| "remote-view route action requires routeId or remoteViewRouteId".to_string())
}

pub(crate) fn service_remote_view_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub(crate) fn upsert_remote_view_stream_for_route(
    browser: &mut super::super::service_model::BrowserProcess,
    cmd: &Value,
    route: &RemoteViewRoute,
    display_allocation_id: &str,
    frame_url: &Option<String>,
    external_url: &Option<String>,
) {
    let stream_id = optional_command_string(cmd, "streamId")
        .unwrap_or_else(|| "remote-headed-view".to_string());
    let url = optional_command_string(cmd, "remoteViewUrl")
        .or_else(|| frame_url.clone())
        .or_else(|| external_url.clone());
    let stream = browser.view_streams.iter_mut().find(|stream| {
        stream.id == stream_id || stream.route_id.as_deref() == Some(route.id.as_str())
    });
    let update_stream = |stream: &mut ViewStream| {
        stream.id = stream_id.clone();
        stream.provider = route.provider;
        stream.control_input = route.control_input;
        stream.url = url.clone();
        stream.frame_url = frame_url.clone();
        stream.external_url = external_url.clone();
        stream.route_descriptor = route.route_descriptor.clone();
        stream.route_id = Some(route.id.clone());
        stream.display_allocation_id = Some(display_allocation_id.to_string());
        stream.connection_id = route.connection_id.clone();
        stream.connection_name = route.connection_name.clone();
        stream.route_source = Some(route.route_source.clone());
        stream.provider_mode = Some(route.provider_mode.clone());
        stream.viewer_lease_ids = route.viewer_lease_ids.clone();
        stream.controller_lease_id = route.controller_lease_id.clone();
        stream.read_only = route.read_only;
        stream.readiness = route.readiness.clone();
        let mut remote_readiness = json!(
            { "state" : route.state, "lastProviderEvent" : route.last_provider_event, }
        );
        if let Some(display_content) = cmd.get("displayContent").cloned() {
            remote_readiness["displayContent"] = display_content;
        }
        stream.remote_readiness = Some(remote_readiness);
    };
    if let Some(stream) = stream {
        update_stream(stream);
    } else {
        let mut stream = ViewStream::default();
        update_stream(&mut stream);
        browser.view_streams.push(stream);
    }
}

pub(crate) fn remote_view_open_reusable_live_target(
    pages: &[PageInfo],
    preferred_target_id: Option<&str>,
    desired_origin: Option<&str>,
) -> Option<PageInfo> {
    if let Some(preferred_target_id) = preferred_target_id {
        if let Some(page) = pages
            .iter()
            .find(|page| page.target_id == preferred_target_id && !is_blank_url(&page.url))
        {
            return Some(page.clone());
        }
    }
    let desired_origin = desired_origin?;
    pages
        .iter()
        .find(|page| {
            !is_blank_url(page.url.as_str())
                && origin_for_url(page.url.as_str()).as_deref() == Some(desired_origin)
        })
        .cloned()
}

pub(crate) fn remote_view_open_retained_tab_candidate(
    service_state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    desired_origin: Option<&str>,
) -> Option<BrowserTab> {
    let desired_origin = desired_origin?;
    service_state
        .tabs
        .values()
        .filter(|tab| tab.browser_id == browser_id)
        .filter(|tab| tab.owner_session_id.as_deref() == Some(session_id))
        .filter(|tab| tab.lifecycle == TabLifecycle::Ready)
        .filter(|tab| {
            tab.target_id
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        })
        .find(|tab| {
            tab.url
                .as_deref()
                .filter(|url| !is_blank_url(url))
                .and_then(origin_for_url)
                .as_deref()
                == Some(desired_origin)
        })
        .cloned()
}

pub(crate) fn remote_view_open_tab_creation_command(cmd: &Value) -> Value {
    let mut initial = cmd.clone();
    initial["url"] = json!("about:blank");
    initial
}

pub(crate) fn remote_view_open_active_target_readback(
    active_target_id: Option<&str>,
    pages: &[PageInfo],
    target_id: &str,
) -> Option<Value> {
    if active_target_id != Some(target_id) {
        return None;
    }
    let page = pages.iter().find(|page| page.target_id == target_id)?;
    Some(json!(
        { "targetId" : page.target_id, "state" : "already_active", "url" : page.url,
        "title" : page.title, }
    ))
}

async fn route_bound_open_acquire_target<R: RouteBoundOpenRuntime>(
    cmd: &Value,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    service_state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    prefer_active_existing_target: bool,
) -> Result<Value, RouteBoundRuntimeIssue> {
    let expected_url = cmd.get("url").and_then(Value::as_str);
    let desired_origin = expected_url.and_then(origin_for_url);
    let observation = supervisor
        .forward("refresh_targets", runtime.refresh_targets())
        .await?;
    let retained_target_id = remote_view_open_retained_tab_candidate(
        service_state,
        browser_id,
        session_id,
        desired_origin.as_deref(),
    )
    .and_then(|tab| tab.target_id.clone());
    let selected = remote_view_open_reusable_live_target(
        &observation.pages,
        cmd.get("preferredTargetId").and_then(Value::as_str),
        desired_origin.as_deref(),
    )
    .or_else(|| {
        retained_target_id.as_deref().and_then(|target_id| {
            observation
                .pages
                .iter()
                .find(|page| page.target_id == target_id)
                .cloned()
        })
    })
    .or_else(|| {
        prefer_active_existing_target.then(|| {
            observation
                .active_target_id
                .as_deref()
                .and_then(|target_id| {
                    observation
                        .pages
                        .iter()
                        .find(|page| page.target_id == target_id)
                        .cloned()
                })
        })?
    });

    let mut tab = if let Some(page) = selected {
        let switch = if observation.active_target_id.as_deref() == Some(page.target_id.as_str()) {
            json!({
                "targetId": page.target_id,
                "state": "already_active",
                "url": page.url,
                "title": page.title,
            })
        } else {
            supervisor
                .forward(
                    "switch_target",
                    runtime.switch_target(SwitchTargetRequest {
                        target_id: page.target_id.clone(),
                    }),
                )
                .await?
        };
        let selected_target_id = switch
            .get("targetId")
            .and_then(Value::as_str)
            .unwrap_or(&page.target_id)
            .to_string();
        let decision = if retained_target_id.as_deref() == Some(selected_target_id.as_str()) {
            "reused_retained_service_tab"
        } else if prefer_active_existing_target
            && observation.active_target_id.as_deref() == Some(selected_target_id.as_str())
        {
            "reused_active_target_for_route_reattach"
        } else {
            "reused_compatible_target"
        };
        route_bound_open_reused_target_result(
            cmd,
            &observation,
            browser_id,
            session_id,
            &selected_target_id,
            switch,
            decision,
        )
        .map_err(|message| route_bound_runtime_issue("observe_browser", message, Some(cmd)))?
    } else {
        let mut opened = supervisor
            .forward(
                "open_target",
                runtime.open_target(OpenTargetRequest {
                    command: remote_view_open_tab_creation_command(cmd),
                }),
            )
            .await?;
        opened["tabAcquisitionDecision"] = json!("opened_new_target");
        opened["reusedExistingTarget"] = Value::Bool(false);
        opened
    };

    route_bound_open_wait_for_target(cmd, runtime, supervisor, &mut tab).await;
    tab["duplicateTargetCleanup"] = no_duplicate_target_cleanup();
    if let Some(service_tab_handle) = tab.get("serviceTabHandle").cloned() {
        persist_service_owned_tab_new(
            cmd,
            session_id,
            tab.get("targetId").and_then(Value::as_str),
            tab.get("url").and_then(Value::as_str),
            tab.get("title").and_then(Value::as_str),
            &service_tab_handle,
        )
        .map_err(|message| route_bound_runtime_issue("open_target", message, Some(cmd)))?;
    }
    Ok(tab)
}

fn route_bound_open_reused_target_result(
    cmd: &Value,
    observation: &RouteBoundBrowserObservation,
    browser_id: &str,
    session_id: &str,
    target_id: &str,
    switch: Value,
    decision: &str,
) -> Result<Value, String> {
    let page = observation
        .pages
        .iter()
        .find(|page| page.target_id == target_id);
    let url = switch
        .get("url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| page.map(|page| page.url.clone()))
        .or_else(|| observation.active_url.clone())
        .unwrap_or_default();
    let title = switch
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| page.map(|page| page.title.clone()))
        .or_else(|| observation.active_title.clone())
        .unwrap_or_default();
    let profile_id = observation.runtime_profile.clone().unwrap_or_default();
    let service_tab_handle = json!({
        "browserId": browser_id,
        "sessionName": session_id,
        "tabId": format!("target:{target_id}"),
        "targetId": target_id,
        "url": url,
        "title": title,
        "profileId": profile_id,
        "profileOrigin": "agent_browser_owned",
        "leaseId": session_id,
        "leaseState": "shared",
        "cleanupPolicy": "detach",
        "leaseHeartbeatExpected": true,
        "ownerSessionId": session_id,
        "jobId": Value::Null,
        "traceFilter": {
            "browserId": browser_id,
            "profileId": profile_id,
            "sessionId": session_id,
            "serviceName": optional_command_string(cmd, "serviceName"),
            "agentName": optional_command_string(cmd, "agentName"),
            "taskName": optional_command_string(cmd, "taskName"),
        },
        "valid": true,
        "staleReason": Value::Null,
    });
    persist_service_owned_tab_new(
        cmd,
        session_id,
        Some(target_id),
        Some(&url),
        Some(&title),
        &service_tab_handle,
    )?;
    Ok(json!({
        "targetId": target_id,
        "url": url,
        "title": title,
        "browserId": browser_id,
        "sessionId": session_id,
        "profileId": profile_id,
        "serviceTabHandle": service_tab_handle,
        "reusedExistingTarget": true,
        "tabAcquisitionDecision": decision,
        "targetReadiness": route_bound_handoff_target_url_readiness(
            cmd.get("url").and_then(Value::as_str),
            Some(&url),
        ),
        "tabSwitch": switch,
    }))
}

async fn route_bound_open_wait_for_target<R: RouteBoundOpenRuntime>(
    cmd: &Value,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    tab: &mut Value,
) {
    let Some(expected_url) = cmd.get("url").and_then(Value::as_str) else {
        return;
    };
    let Some(target_id) = tab
        .get("targetId")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    if let Ok(switched) = supervisor
        .forward(
            "switch_target",
            runtime.switch_target(SwitchTargetRequest {
                target_id: target_id.clone(),
            }),
        )
        .await
    {
        tab["targetSwitch"] = switched;
    }
    let observed_url = tab
        .pointer("/targetSwitch/url")
        .and_then(Value::as_str)
        .or_else(|| tab.get("url").and_then(Value::as_str));
    if route_bound_handoff_target_url_readiness(Some(expected_url), observed_url) != "ready" {
        match supervisor
            .forward(
                "navigate_target",
                runtime.navigate_target(NavigateTargetRequest {
                    url: expected_url.to_string(),
                }),
            )
            .await
        {
            Ok(result) => {
                tab["targetNavigation"] = json!({
                    "state": "requested",
                    "requestedUrl": expected_url,
                    "result": result,
                });
            }
            Err(error) => {
                tab["targetNavigation"] = json!({
                    "state": "failed",
                    "requestedUrl": expected_url,
                    "error": error.compatibility_message(),
                });
            }
        }
    }
    for attempt in 0..20 {
        let Ok(observation) = supervisor
            .forward("refresh_targets", runtime.refresh_targets())
            .await
        else {
            return;
        };
        let page = observation
            .pages
            .iter()
            .find(|page| page.target_id == target_id);
        let url = page
            .map(|page| page.url.as_str())
            .or(observation.active_url.as_deref());
        let title = page
            .map(|page| page.title.as_str())
            .or(observation.active_title.as_deref());
        if let Some(url) = url {
            tab["url"] = json!(url);
        }
        if let Some(title) = title {
            tab["title"] = json!(title);
        }
        tab["urlReadbackAttempts"] = json!(attempt + 1);
        let readiness = route_bound_handoff_target_url_readiness(Some(expected_url), url);
        tab["targetReadiness"] = json!(readiness);
        if readiness == "ready" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

pub(crate) async fn remote_view_open_acquire_tab(
    cmd: &Value,
    state: &mut DaemonState,
    service_state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    prefer_active_existing_target: bool,
) -> Result<Value, String> {
    let requested_url = cmd.get("url").and_then(Value::as_str);
    let desired_origin = requested_url.and_then(origin_for_url);
    if desired_origin.is_some() {
        if let Some(mgr) = state.browser.as_mut() {
            let active_url = mgr.get_url().await.ok();
            let active_title = mgr.get_title().await.ok();
            if active_url.as_deref().and_then(origin_for_url).as_deref()
                == desired_origin.as_deref()
            {
                mgr.set_active_page_metadata(active_url.as_deref(), active_title.as_deref());
            }
        }
    }
    let reusable_target = {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        remote_view_open_reusable_live_target(
            &mgr.pages_list(),
            cmd.get("preferredTargetId").and_then(Value::as_str),
            desired_origin.as_deref(),
        )
    };
    if let Some(page) = reusable_target {
        state.ref_map.clear();
        state.iframe_sessions.clear();
        state.active_frame_id = None;
        let session_id = state.session_id.clone();
        let browser_id = service_browser_id(&session_id);
        let mut result = {
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            let already_active = mgr.active_target_id().ok() == Some(page.target_id.as_str());
            let mut switched = if already_active {
                json!(
                    { "targetId" : page.target_id.clone(), "state" : "already_active",
                    "url" : page.url.clone(), "title" : page.title.clone(), }
                )
            } else {
                mgr.tab_switch_target_id(&page.target_id).await?
            };
            let url = if already_active {
                page.url
            } else {
                mgr.get_url().await.unwrap_or(page.url)
            };
            let title = if already_active {
                page.title
            } else {
                mgr.get_title().await.unwrap_or(page.title)
            };
            switched["refreshDecision"] = json!("reused_compatible_target");
            let mut result = json!(
                { "targetId" : switched.get("targetId").and_then(Value::as_str)
                .unwrap_or_default(), "url" : url, "title" : title, "browserId" :
                browser_id, "sessionId" : session_id, "reusedExistingTarget" : true,
                "tabAcquisitionDecision" : "reused_compatible_target", "tabSwitch" :
                switched, }
            );
            if let Some(object) = result.as_object_mut() {
                if let Some(runtime_profile) = mgr.runtime_profile_name() {
                    object.insert("runtimeProfile".to_string(), json!(runtime_profile));
                    object.insert("profileId".to_string(), json!(runtime_profile));
                }
                let profile_id = object.get("profileId").cloned().unwrap_or(Value::Null);
                object.insert(
                    "sharedAcquisition".to_string(),
                    tab_new_shared_acquisition_evidence(cmd, &state.session_id, profile_id.clone()),
                );
                let tab_id = object
                    .get("targetId")
                    .and_then(Value::as_str)
                    .map(|target_id| format!("target:{target_id}"))
                    .unwrap_or_else(|| format!("session:{}:active-tab", state.session_id));
                let service_tab_handle = json!(
                    { "browserId" : service_browser_id(& state.session_id), "sessionName"
                    : state.session_id.clone(), "tabId" : tab_id, "targetId" : object
                    .get("targetId").cloned().unwrap_or(Value::Null), "url" : object
                    .get("url").cloned().unwrap_or(Value::Null), "title" : object
                    .get("title").cloned().unwrap_or(Value::Null), "profileId" :
                    profile_id.clone(), "profileOrigin" : "agent_browser_owned",
                    "leaseId" : state.session_id.clone(), "leaseState" : "shared",
                    "cleanupPolicy" : "detach", "leaseHeartbeatExpected" : true,
                    "ownerSessionId" : state.session_id.clone(), "jobId" : Value::Null,
                    "traceFilter" : { "browserId" : service_browser_id(& state
                    .session_id), "profileId" : profile_id.clone(), "sessionId" : state
                    .session_id.clone(), "serviceName" : optional_command_string(cmd,
                    "serviceName"), "agentName" : optional_command_string(cmd,
                    "agentName"), "taskName" : optional_command_string(cmd, "taskName"),
                    }, "valid" : true, "staleReason" : Value::Null, }
                );
                persist_service_owned_tab_new(
                    cmd,
                    &state.session_id,
                    object.get("targetId").and_then(Value::as_str),
                    object.get("url").and_then(Value::as_str),
                    object.get("title").and_then(Value::as_str),
                    &service_tab_handle,
                )?;
                object.insert("serviceTabHandle".to_string(), service_tab_handle);
            }
            result
        };
        remote_view_open_wait_for_target_url(cmd, state, &mut result).await;
        if let Some(service_tab_handle) = result.get("serviceTabHandle").cloned() {
            persist_service_owned_tab_new(
                cmd,
                &state.session_id,
                result.get("targetId").and_then(Value::as_str),
                result.get("url").and_then(Value::as_str),
                result.get("title").and_then(Value::as_str),
                &service_tab_handle,
            )?;
        }
        let selected_target_id = result
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or_else(|| "remote_view_open reused a tab without targetId".to_string())?
            .to_string();
        if let Some(mgr) = state.browser.as_mut() {
            let duplicate_target_cleanup = close_compatible_duplicate_targets(
                mgr,
                &selected_target_id,
                None,
                desired_origin.as_deref(),
            )
            .await;
            result["duplicateTargetCleanup"] = duplicate_target_cleanup;
        }
        return Ok(result);
    }
    if let Some(tab) = remote_view_open_retained_tab_candidate(
        service_state,
        browser_id,
        session_id,
        desired_origin.as_deref(),
    ) {
        if let Some(target_id) = tab.target_id.as_deref() {
            if state
                .browser
                .as_ref()
                .and_then(|mgr| mgr.active_target_id().ok())
                == Some(target_id)
            {
                if let Some(mgr) = state.browser.as_mut() {
                    mgr.set_active_page_metadata(tab.url.as_deref(), tab.title.as_deref());
                }
                let profile_id = tab
                    .service_tab_handle
                    .as_ref()
                    .and_then(|handle| handle.profile_id.clone())
                    .unwrap_or_default();
                let mut result = json!(
                    { "targetId" : target_id, "url" : tab.url.clone()
                    .unwrap_or_default(), "title" : tab.title.clone()
                    .unwrap_or_default(), "browserId" : browser_id, "sessionId" :
                    session_id, "profileId" : profile_id, "reusedExistingTarget" : true,
                    "tabAcquisitionDecision" :
                    "reused_retained_service_tab_active_target", "targetReadiness" :
                    route_bound_handoff_target_url_readiness(cmd.get("url")
                    .and_then(Value::as_str), tab.url.as_deref()),
                    "duplicateTargetCleanup" : no_duplicate_target_cleanup(), }
                );
                if let Some(handle) = tab.service_tab_handle.as_ref() {
                    result["serviceTabHandle"] =
                        serde_json::to_value(handle).unwrap_or(Value::Null);
                }
                return Ok(result);
            }
        }
    }
    if prefer_active_existing_target {
        if let Some(mgr) = state.browser.as_mut() {
            if let Ok(target_id) = mgr.active_target_id().map(str::to_string) {
                let requested_url = requested_url.unwrap_or("about:blank");
                let title = mgr.get_title().await.unwrap_or_default();
                mgr.set_page_metadata_for_target(&target_id, Some(requested_url), Some(&title));
                let profile_id = mgr.runtime_profile_name().unwrap_or_default().to_string();
                let service_tab_handle = json!(
                    { "browserId" : browser_id, "sessionName" : session_id, "tabId" :
                    format!("target:{target_id}"), "targetId" : target_id, "url" :
                    requested_url, "title" : title, "profileId" : profile_id,
                    "profileOrigin" : "agent_browser_owned", "leaseId" : session_id,
                    "leaseState" : "shared", "cleanupPolicy" : "detach",
                    "leaseHeartbeatExpected" : true, "ownerSessionId" : session_id,
                    "jobId" : Value::Null, "traceFilter" : { "browserId" : browser_id,
                    "profileId" : profile_id.clone(), "sessionId" : session_id,
                    "serviceName" : optional_command_string(cmd, "serviceName"),
                    "agentName" : optional_command_string(cmd, "agentName"), "taskName" :
                    optional_command_string(cmd, "taskName"), }, "valid" : true,
                    "staleReason" : Value::Null, }
                );
                persist_service_owned_tab_new(
                    cmd,
                    session_id,
                    Some(&target_id),
                    Some(requested_url),
                    Some(&title),
                    &service_tab_handle,
                )?;
                return Ok(json!(
                    { "targetId" : target_id, "url" : requested_url, "title" : title,
                    "browserId" : browser_id, "sessionId" : session_id, "profileId" :
                    profile_id.clone(), "serviceTabHandle" : service_tab_handle,
                    "reusedExistingTarget" : true, "tabAcquisitionDecision" :
                    "reused_active_target_for_route_reattach", "targetReadiness" :
                    route_bound_handoff_target_url_readiness(cmd.get("url")
                    .and_then(Value::as_str), Some(requested_url)),
                    "duplicateTargetCleanup" : no_duplicate_target_cleanup(), }
                ));
            }
        }
    }
    let initial_tab_command = remote_view_open_tab_creation_command(cmd);
    let mut opened = handle_tab_new(&initial_tab_command, state).await?;
    remote_view_open_wait_for_target_url(cmd, state, &mut opened).await;
    if let Some(service_tab_handle) = opened.get("serviceTabHandle").cloned() {
        persist_service_owned_tab_new(
            cmd,
            &state.session_id,
            opened.get("targetId").and_then(Value::as_str),
            opened.get("url").and_then(Value::as_str),
            opened.get("title").and_then(Value::as_str),
            &service_tab_handle,
        )?;
    }
    if let Some(target_id) = opened
        .get("targetId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
    {
        if let Some(mgr) = state.browser.as_mut() {
            let duplicate_target_cleanup = close_compatible_duplicate_targets(
                mgr,
                &target_id,
                None,
                desired_origin.as_deref(),
            )
            .await;
            opened["duplicateTargetCleanup"] = duplicate_target_cleanup;
        }
    }
    opened["tabAcquisitionDecision"] = json!("opened_new_target");
    Ok(opened)
}

pub(crate) async fn remote_view_open_wait_for_target_url(
    cmd: &Value,
    state: &mut DaemonState,
    tab: &mut Value,
) {
    let Some(expected_url) = cmd.get("url").and_then(Value::as_str).map(str::to_string) else {
        return;
    };
    let mut target_id = tab
        .get("targetId")
        .and_then(Value::as_str)
        .map(str::to_string);
    {
        let Some(mgr) = state.browser.as_mut() else {
            return;
        };
        let mut switched_once = false;
        if let Some(target_id) = target_id.as_deref() {
            let active_readback = remote_view_open_active_target_readback(
                mgr.active_target_id().ok(),
                &mgr.pages_list(),
                target_id,
            );
            match if let Some(readback) = active_readback {
                Ok(readback)
            } else {
                mgr.tab_switch_target_id(target_id).await
            } {
                Ok(switched) => {
                    switched_once = true;
                    tab["targetSwitch"] = switched;
                }
                Err(err) => {
                    tab["targetSwitch"] = json!({ "state" : "failed", "error" : err, });
                }
            }
        }
        let switched_url = tab
            .pointer("/targetSwitch/url")
            .and_then(Value::as_str)
            .map(str::to_string);
        if switched_once
            && route_bound_handoff_target_url_readiness(
                Some(&expected_url),
                switched_url.as_deref(),
            ) != "ready"
        {
            match mgr.navigate(&expected_url, WaitUntil::None).await {
                Ok(navigation) => {
                    tab["targetNavigation"] = json!(
                        { "state" : "requested", "requestedUrl" : expected_url.clone(),
                        "result" : navigation, }
                    );
                }
                Err(err) => {
                    tab["targetNavigation"] = json!(
                        { "state" : "failed", "requestedUrl" : expected_url.clone(),
                        "error" : err, }
                    );
                    if let Some(target_id) = target_id.as_deref() {
                        mgr.set_page_metadata_for_target(target_id, Some(&expected_url), None);
                    }
                }
            }
        }
    }
    let desired_origin = origin_for_url(&expected_url);
    for attempt in 0..20 {
        state.drain_cdp_events_background().await;
        let Some(mgr) = state.browser.as_mut() else {
            return;
        };
        let selected_switched = if let Some(target_id) = target_id.as_deref() {
            let active_readback = remote_view_open_active_target_readback(
                mgr.active_target_id().ok(),
                &mgr.pages_list(),
                target_id,
            );
            if active_readback.is_some() {
                active_readback
            } else {
                mgr.tab_switch_target_id(target_id).await.ok()
            }
        } else {
            None
        };
        let pages = mgr.pages_list();
        let mut selected_target_id = target_id.clone();
        let mut switched = selected_switched;
        let mut target_page = target_id.as_deref().and_then(|target_id| {
            pages
                .iter()
                .find(|page| page.target_id == target_id)
                .cloned()
        });
        let selected_url = switched
            .as_ref()
            .and_then(|value| value.get("url"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| target_page.as_ref().map(|page| page.url.clone()));
        if route_bound_handoff_target_url_readiness(Some(&expected_url), selected_url.as_deref())
            != "ready"
        {
            if let Some(compatible_page) =
                remote_view_open_reusable_live_target(&pages, None, desired_origin.as_deref())
            {
                if target_id.as_deref() != Some(compatible_page.target_id.as_str()) {
                    if let Ok(compatible_switched) =
                        mgr.tab_switch_target_id(&compatible_page.target_id).await
                    {
                        tab["targetReselection"] = json!(
                            { "state" : "reselected_compatible_target",
                            "previousTargetId" : target_id, "targetId" : compatible_page
                            .target_id, "url" : compatible_page.url, "title" :
                            compatible_page.title, }
                        );
                        target_id = compatible_switched
                            .get("targetId")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .or_else(|| {
                                tab.pointer("/targetReselection/targetId")
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                            });
                        selected_target_id = target_id.clone();
                        switched = Some(compatible_switched);
                    }
                }
            }
        }
        target_page = selected_target_id.as_deref().and_then(|target_id| {
            mgr.pages_list()
                .into_iter()
                .find(|page| page.target_id == target_id)
        });
        let mut url = switched
            .as_ref()
            .and_then(|value| value.get("url"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if url.is_none() {
            url = mgr.get_url().await.ok();
        }
        if url.is_none() {
            url = target_page.as_ref().map(|page| page.url.clone());
        }
        let mut title = switched
            .as_ref()
            .and_then(|value| value.get("title"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if title.is_none() {
            title = mgr.get_title().await.ok();
        }
        if title.is_none() {
            title = target_page.as_ref().map(|page| page.title.clone());
        }
        if let Some(url) = url.as_deref() {
            tab["url"] = json!(url);
            if let Some(service_tab_handle) = tab
                .get_mut("serviceTabHandle")
                .and_then(Value::as_object_mut)
            {
                service_tab_handle.insert("url".to_string(), json!(url));
            }
        }
        if let Some(title) = title.as_deref() {
            tab["title"] = json!(title);
            if let Some(service_tab_handle) = tab
                .get_mut("serviceTabHandle")
                .and_then(Value::as_object_mut)
            {
                service_tab_handle.insert("title".to_string(), json!(title));
            }
        }
        if let Some(target_id) = selected_target_id.as_deref() {
            tab["targetId"] = json!(target_id);
            if let Some(service_tab_handle) = tab
                .get_mut("serviceTabHandle")
                .and_then(Value::as_object_mut)
            {
                service_tab_handle.insert("targetId".to_string(), json!(target_id));
                service_tab_handle
                    .insert("tabId".to_string(), json!(format!("target:{target_id}")));
            }
            mgr.set_page_metadata_for_target(target_id, url.as_deref(), title.as_deref());
        }
        mgr.set_active_page_metadata(url.as_deref(), title.as_deref());
        tab["urlReadbackAttempts"] = json!(attempt + 1);
        tab["targetReadiness"] = json!(route_bound_handoff_target_url_readiness(
            Some(&expected_url),
            url.as_deref()
        ));
        if route_bound_handoff_target_url_readiness(Some(&expected_url), url.as_deref()) == "ready"
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

pub(crate) fn retained_readiness_component<'a>(
    readiness: Option<&'a Value>,
    component_names: &[&str],
) -> Option<&'a Value> {
    readiness
        .and_then(|readiness| {
            readiness
                .get("components")
                .or_else(|| readiness.pointer("/readiness/components"))
        })
        .and_then(Value::as_array)
        .and_then(|components| {
            components.iter().find(|component| {
                component
                    .get("component")
                    .and_then(Value::as_str)
                    .is_some_and(|name| {
                        component_names.iter().any(|expected| {
                            name == *expected
                                || name
                                    .strip_prefix(*expected)
                                    .is_some_and(|rest| rest.starts_with(':'))
                        })
                    })
            })
        })
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct RoutePoolSelection {
    pub(crate) entry: RoutePoolEntry,
    pub(crate) parked_route: Option<RouteParkingPlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct RouteParkingPlan {
    pub(crate) route_id: String,
    pub(crate) route_pool_entry_id: String,
    pub(crate) browser_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) controller_lease_id: Option<String>,
}

pub(crate) fn select_browser_reattach_route_pool_entry(
    state: &ServiceState,
    stream: Option<&ViewStream>,
    requested_route_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
    route_switch: bool,
    browser_id: &str,
    controller_takeover: bool,
) -> Option<RoutePoolSelection> {
    let current_route_id = stream.and_then(|stream| stream.route_id.as_deref());
    if let Some(id) = requested_route_pool_entry_id {
        return state.route_pool.get(id).cloned().map(|entry| {
            route_pool_selection_for_entry(
                state,
                entry,
                route_switch,
                browser_id,
                current_route_id,
                controller_takeover,
            )
        });
    }
    if let Some(route_id) = requested_route_id {
        return state
            .route_pool
            .values()
            .find(|entry| {
                entry.route_id == route_id
                    || entry.current_route_allocation_id.as_deref() == Some(route_id)
            })
            .cloned()
            .map(|entry| {
                route_pool_selection_for_entry(
                    state,
                    entry,
                    route_switch,
                    browser_id,
                    current_route_id,
                    controller_takeover,
                )
            });
    }
    if route_switch {
        if let Some(entry) = state.route_pool.values().find(|entry| {
            entry.provider == ViewStreamProvider::RdpGateway
                && matches!(entry.state.as_str(), "available" | "ready" | "unknown")
                && Some(entry.route_id.as_str()) != current_route_id
        }) {
            return Some(RoutePoolSelection {
                entry: entry.clone(),
                parked_route: None,
            });
        }
        if let Some(selection) = select_parkable_route_pool_entry(
            state,
            browser_id,
            current_route_id,
            controller_takeover,
        ) {
            return Some(selection);
        }
    }
    if let Some(route_id) = current_route_id {
        if let Some(entry) = state.route_pool.values().find(|entry| {
            entry.route_id == route_id
                || entry.current_route_allocation_id.as_deref() == Some(route_id)
        }) {
            return Some(RoutePoolSelection {
                entry: entry.clone(),
                parked_route: None,
            });
        }
    }
    state
        .route_pool
        .values()
        .find(|entry| {
            entry.provider == ViewStreamProvider::RdpGateway
                && matches!(entry.state.as_str(), "available" | "ready" | "unknown")
        })
        .cloned()
        .map(|entry| RoutePoolSelection {
            entry,
            parked_route: None,
        })
}

pub(crate) fn route_pool_selection_for_entry(
    state: &ServiceState,
    entry: RoutePoolEntry,
    route_switch: bool,
    browser_id: &str,
    current_route_id: Option<&str>,
    controller_takeover: bool,
) -> RoutePoolSelection {
    let parked_route = route_switch
        .then(|| {
            parkable_route_for_entry(
                state,
                &entry,
                browser_id,
                current_route_id,
                controller_takeover,
            )
        })
        .flatten();
    RoutePoolSelection {
        entry,
        parked_route,
    }
}

pub(crate) fn select_parkable_route_pool_entry(
    state: &ServiceState,
    browser_id: &str,
    current_route_id: Option<&str>,
    controller_takeover: bool,
) -> Option<RoutePoolSelection> {
    let mut candidates = state
        .route_pool
        .values()
        .filter(|entry| entry.provider == ViewStreamProvider::RdpGateway)
        .filter_map(|entry| {
            let parking = parkable_route_for_entry(
                state,
                entry,
                browser_id,
                current_route_id,
                controller_takeover,
            )?;
            Some((
                route_parking_sort_key(state, &parking.route_id),
                entry.id.clone(),
                entry.clone(),
                parking,
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .next()
        .map(
            |(_sort_key, _entry_id, entry, parked_route)| RoutePoolSelection {
                entry,
                parked_route: Some(parked_route),
            },
        )
}

pub(crate) fn parkable_route_for_entry(
    state: &ServiceState,
    entry: &RoutePoolEntry,
    browser_id: &str,
    current_route_id: Option<&str>,
    controller_takeover: bool,
) -> Option<RouteParkingPlan> {
    if !matches!(entry.state.as_str(), "checked_out" | "occupied") {
        return None;
    }
    let route_id = entry
        .current_route_allocation_id
        .as_deref()
        .filter(|route_id| Some(*route_id) != current_route_id)
        .or_else(|| {
            (!entry.route_id.is_empty() && Some(entry.route_id.as_str()) != current_route_id)
                .then_some(entry.route_id.as_str())
        })?;
    let route = state.remote_view_routes.get(route_id)?;
    if route.browser_id.as_deref() == Some(browser_id) {
        return None;
    }
    let owner_browser_is_live = route
        .browser_id
        .as_deref()
        .and_then(|id| state.browsers.get(id))
        .is_some_and(|browser| {
            matches!(
                browser.health,
                ServiceBrowserHealth::Ready
                    | ServiceBrowserHealth::Launching
                    | ServiceBrowserHealth::Reconnecting
                    | ServiceBrowserHealth::Degraded
                    | ServiceBrowserHealth::CdpDisconnected
            )
        });
    if !owner_browser_is_live {
        return None;
    }
    let active_controller = route
        .controller_lease_id
        .as_ref()
        .and_then(|lease_id| state.viewer_leases.get(lease_id))
        .is_some_and(remote_view_lease_is_active);
    if active_controller && !controller_takeover {
        return None;
    }
    Some(RouteParkingPlan {
        route_id: route_id.to_string(),
        route_pool_entry_id: entry.id.clone(),
        browser_id: route.browser_id.clone(),
        session_id: route.session_id.clone(),
        controller_lease_id: route.controller_lease_id.clone(),
    })
}

pub(crate) fn route_parking_sort_key(state: &ServiceState, route_id: &str) -> (usize, String) {
    let Some(route) = state.remote_view_routes.get(route_id) else {
        return (usize::MAX, String::new());
    };
    let active_viewer_count = route
        .viewer_lease_ids
        .iter()
        .filter(|lease_id| {
            state
                .viewer_leases
                .get(*lease_id)
                .is_some_and(remote_view_lease_is_active)
        })
        .count();
    let newest_activity = route
        .viewer_lease_ids
        .iter()
        .filter_map(|lease_id| state.viewer_leases.get(lease_id))
        .filter_map(|lease| {
            lease
                .last_heartbeat_at
                .as_deref()
                .or(lease.updated_at.as_deref())
                .or(lease.created_at.as_deref())
        })
        .max()
        .unwrap_or("");
    (active_viewer_count, newest_activity.to_string())
}

pub(crate) fn merge_route_pool_entry_into_checkout(
    command: &mut Map<String, Value>,
    entry: &RoutePoolEntry,
) {
    insert_checkout_string(command, "frameUrl", entry.frame_url.clone());
    insert_checkout_string(command, "externalUrl", entry.external_url.clone());
    insert_checkout_string(command, "connectionId", entry.connection_id.clone());
    insert_checkout_string(command, "connectionName", entry.connection_name.clone());
    insert_checkout_value(command, "routeDescriptor", entry.route_descriptor.clone());
    insert_checkout_string(command, "providerMode", Some(entry.provider_mode.clone()));
}

pub(crate) fn merge_route_into_checkout(command: &mut Map<String, Value>, route: &RemoteViewRoute) {
    insert_checkout_string(command, "frameUrl", route.frame_url.clone());
    insert_checkout_string(command, "externalUrl", route.external_url.clone());
    insert_checkout_string(command, "connectionId", route.connection_id.clone());
    insert_checkout_string(command, "connectionName", route.connection_name.clone());
    insert_checkout_value(command, "routeDescriptor", route.route_descriptor.clone());
    insert_checkout_string(command, "providerMode", Some(route.provider_mode.clone()));
}

pub(crate) fn merge_stream_into_checkout(command: &mut Map<String, Value>, stream: &ViewStream) {
    insert_checkout_string(command, "frameUrl", stream.frame_url.clone());
    insert_checkout_string(command, "externalUrl", stream.external_url.clone());
    insert_checkout_string(command, "connectionId", stream.connection_id.clone());
    insert_checkout_string(command, "connectionName", stream.connection_name.clone());
    insert_checkout_value(command, "routeDescriptor", stream.route_descriptor.clone());
    insert_checkout_string(command, "providerMode", stream.provider_mode.clone());
    let display_content = stream
        .remote_readiness
        .as_ref()
        .and_then(|value| value.get("displayContent").cloned())
        .or_else(|| {
            stream
                .readiness
                .as_ref()
                .and_then(|value| value.get("displayContent").cloned())
        });
    insert_checkout_value(command, "displayContent", display_content);
}

pub(crate) fn insert_checkout_string(
    command: &mut Map<String, Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        command
            .entry(key.to_string())
            .or_insert(Value::String(value));
    }
}

pub(crate) fn insert_checkout_value(
    command: &mut Map<String, Value>,
    key: &str,
    value: Option<Value>,
) {
    if let Some(value) = value {
        command.entry(key.to_string()).or_insert(value);
    }
}
