#![allow(unused_imports)]
use super::compensation::{
    remote_view_open_rollback_failure_after_cleanup, RemoteViewOpenFailureCleanupInput,
};
use super::deadline::{
    route_bound_execution_error_with_cleanup, route_bound_message_error_with_cleanup,
    RouteBoundOpenExecutionError, RouteBoundOpenSupervisor, RouteBoundRuntimeIssue,
};
use super::operator_route::{remote_view_open_dry_run, route_binding_with_operator_access};
use super::planner::{
    inline_route_pool_entries_from_command, inline_route_pool_entry_from_command,
    remote_view_open_command_with_effective_intent,
    remote_view_open_ensure_managed_one_time_profile, remote_view_open_one_time_profile_warning,
    remote_view_open_persist_request_route_pool, remote_view_open_runtime_attach_launch_command,
    remote_view_open_should_reuse_current_browser, service_remote_view_acquisition_plan_from_state,
};
use super::route_lifecycle::service_remote_view_timestamp;
use super::runtime::{
    CheckoutRouteRequest, DaemonRouteBoundOpenRepository, DaemonRouteBoundOpenRuntime,
    DisplayAccessRequest, FocusTargetRequest, LaunchBrowserRequest, OperatorAccessRequest,
    OperatorAccessResult, RouteBoundOpenRepository, RouteBoundOpenRuntime, VisibleWindowRequest,
};
use super::shared::*;
use super::target::route_bound_open_acquire_target;
use crate::native::remote_view::RemoteViewOpenIntent;
/// Transport-neutral attribution supplied after the ingress has authorized a
/// route-bound open. Cookies, headers, and transport sessions never cross this
/// seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteBoundOpenAuthorization {
    AuthenticatedDaemonCommand,
    Rejected,
}

impl RouteBoundOpenAuthorization {
    pub(crate) fn is_authorized(self) -> bool {
        matches!(self, Self::AuthenticatedDaemonCommand)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteBoundOpenAttribution {
    pub(crate) caller_id: Option<String>,
    pub(crate) service_job_id: Option<String>,
    pub(crate) authorization: RouteBoundOpenAuthorization,
}

#[derive(Debug, Clone)]
pub(crate) struct RouteBoundResolutionSnapshot {
    pub(crate) state: ServiceState,
    pub(crate) handoff: RemoteViewHandoff,
    pub(crate) loaded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundFallbackEligibility {
    pub(crate) prior_provider: bool,
    pub(crate) snapshot_identity: bool,
    pub(crate) snapshot_timing: bool,
    pub(crate) exact_ownership_cause: bool,
    pub(crate) retained_route: bool,
    pub(crate) authorized_ingress: bool,
    pub(crate) operator_evidence: bool,
    pub(crate) browser_preserved: bool,
    pub(crate) duplicate_lane_prohibited: bool,
}

impl RouteBoundFallbackEligibility {
    pub(crate) fn is_eligible(&self) -> bool {
        self.prior_provider
            && self.snapshot_identity
            && self.snapshot_timing
            && self.exact_ownership_cause
            && self.retained_route
            && self.authorized_ingress
            && self.operator_evidence
            && self.browser_preserved
            && self.duplicate_lane_prohibited
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteBoundOpenDocument(Value);

impl RouteBoundOpenDocument {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub(crate) fn into_value(self) -> Value {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RouteBoundOpenCommand(Value);

impl RouteBoundOpenCommand {
    fn as_value(&self) -> &Value {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteBoundDirectOpenRequest {
    command: RouteBoundOpenCommand,
    intent: RemoteViewOpenIntent,
    dry_run: bool,
    handoff_id: Option<String>,
}

impl RouteBoundDirectOpenRequest {
    pub(crate) fn from_compatibility_command(
        mut command: Value,
        handoff_id: Option<String>,
        attribution: RouteBoundOpenAttribution,
    ) -> Result<Self, String> {
        if !attribution.authorization.is_authorized() {
            return Err("Unauthorized route-bound open invocation".to_string());
        }
        if let Some(handoff_id) = handoff_id.as_ref() {
            command["remoteViewHandoffId"] = Value::String(handoff_id.clone());
        }
        if command.get("serviceJobId").is_none() {
            if let Some(service_job_id) = attribution.service_job_id {
                command["serviceJobId"] = Value::String(service_job_id);
            }
        }
        let intent = normalize_remote_view_open_intent(&command)?;
        let dry_run = remote_view_open_dry_run(&command);
        Ok(Self {
            command: RouteBoundOpenCommand(command),
            intent,
            dry_run,
            handoff_id,
        })
    }

    fn command(&self) -> &Value {
        self.command.as_value()
    }
}

/// The only two ways a caller can ask the route-bound coordinator to work.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RouteBoundOpenInvocation {
    DirectOpen(Box<RouteBoundDirectOpenRequest>),
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
/// Complete typed result set for direct opens and durable resolution.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RouteBoundOpenOutcome {
    Planned {
        plan: RouteBoundOpenDocument,
    },
    NotFound {
        result: RouteBoundOpenDocument,
    },
    ExplicitlyClosed {
        result: RouteBoundOpenDocument,
    },
    Reopened {
        opened: RouteBoundOpenDocument,
    },
    Opened {
        opened: RouteBoundOpenDocument,
    },
    RolledBack {
        blocker: RouteBoundOpenBlocker,
        compensation: RouteBoundOpenCompensation,
        compatibility_error: String,
    },
    ProviderFallback {
        fallback: RouteBoundOpenDocument,
    },
}
impl RouteBoundOpenOutcome {
    pub(crate) fn into_compatibility_result(self) -> Result<Value, String> {
        match self {
            Self::Planned { plan }
            | Self::NotFound { result: plan }
            | Self::ExplicitlyClosed { result: plan }
            | Self::Reopened { opened: plan }
            | Self::Opened { opened: plan }
            | Self::ProviderFallback { fallback: plan } => Ok(plan.into_value()),
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
    pub(crate) async fn open<R: RouteBoundOpenRuntime, P: RouteBoundOpenRepository>(
        invocation: RouteBoundOpenInvocation,
        runtime: &mut R,
        repository: &P,
        supervisor: &RouteBoundOpenSupervisor,
    ) -> Result<RouteBoundOpenOutcome, String> {
        match invocation {
            RouteBoundOpenInvocation::DirectOpen(request) => {
                match execute_direct_open(*request, runtime, repository, supervisor).await {
                    Ok(RouteBoundDirectOpenResult::Planned(plan)) => {
                        Ok(RouteBoundOpenOutcome::Planned { plan })
                    }
                    Ok(RouteBoundDirectOpenResult::Opened(opened)) => {
                        Ok(RouteBoundOpenOutcome::Opened { opened })
                    }
                    Err(error) => rolled_back_outcome(error),
                }
            }
            RouteBoundOpenInvocation::DurableResolution {
                handoff_id,
                allow_reopen_closed,
                attribution,
            } => {
                if !attribution.authorization.is_authorized() {
                    return Err("Unauthorized route-bound open invocation".to_string());
                }
                match execute_durable_resolution(
                    handoff_id,
                    allow_reopen_closed,
                    attribution,
                    runtime,
                    repository,
                    supervisor,
                )
                .await
                {
                    Ok(outcome) => Ok(outcome),
                    Err(error) => rolled_back_outcome(error),
                }
            }
        }
    }
}
pub(crate) fn rolled_back_outcome(
    error: RouteBoundOpenExecutionError,
) -> Result<RouteBoundOpenOutcome, String> {
    let Some(failure) = error.terminal_failure else {
        return Err(error.message);
    };
    Ok(RouteBoundOpenOutcome::RolledBack {
        blocker: RouteBoundOpenBlocker {
            code: failure.blocker_code,
            message: failure.blocker_message,
        },
        compensation: RouteBoundOpenCompensation {
            state: failure.compensation_state.as_str().to_string(),
            evidence: failure.evidence,
        },
        compatibility_error: failure.compatibility_error,
    })
}
pub(crate) async fn handle_remote_view_open(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let invocation = RouteBoundOpenInvocation::DirectOpen(Box::new(
        RouteBoundDirectOpenRequest::from_compatibility_command(
            cmd.clone(),
            optional_command_string(cmd, "remoteViewHandoffId")
                .or_else(|| optional_command_string(cmd, "serviceJobId")),
            RouteBoundOpenAttribution {
                caller_id: optional_command_string(cmd, "callerId"),
                service_job_id: optional_command_string(cmd, "serviceJobId"),
                authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
            },
        )?,
    ));
    let supervisor = RouteBoundOpenSupervisor::system(
        cmd.get("jobTimeoutMs").and_then(Value::as_u64),
        state.current_cancellation.clone(),
    );
    let repository = DaemonRouteBoundOpenRepository::new()?;
    let mut runtime = DaemonRouteBoundOpenRuntime::new(state);
    RouteBoundOpenCoordinator::open(invocation, &mut runtime, &repository, &supervisor)
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
            authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
        },
    };
    let supervisor = RouteBoundOpenSupervisor::system(
        cmd.get("jobTimeoutMs").and_then(Value::as_u64),
        state.current_cancellation.clone(),
    );
    let repository = DaemonRouteBoundOpenRepository::new()?;
    let mut runtime = DaemonRouteBoundOpenRuntime::new(state);
    RouteBoundOpenCoordinator::open(invocation, &mut runtime, &repository, &supervisor)
        .await?
        .into_compatibility_result()
}
pub(crate) enum RouteBoundDirectOpenResult {
    Planned(RouteBoundOpenDocument),
    Opened(RouteBoundOpenDocument),
}

pub(crate) async fn execute_direct_open<R: RouteBoundOpenRuntime, P: RouteBoundOpenRepository>(
    request: RouteBoundDirectOpenRequest,
    runtime: &mut R,
    repository: &P,
    supervisor: &RouteBoundOpenSupervisor,
) -> Result<RouteBoundDirectOpenResult, RouteBoundOpenExecutionError> {
    let cmd = request.command().clone();
    let mut intent = request.intent;
    let handoff_id = request.handoff_id;
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
    let mut service_state = supervisor
        .forward(
            "repository_load_snapshot",
            repository.snapshot(supervisor.forward_repository_lock_timeout()),
        )
        .await?;
    let dry_run = request.dry_run;
    let managed_one_time_profile = supervisor
        .forward(
            "repository_managed_one_time_profile",
            repository.execute(
                "repository_managed_one_time_profile",
                supervisor.forward_repository_lock_timeout(),
                |repository| {
                    remote_view_open_ensure_managed_one_time_profile(
                        repository,
                        &mut service_state,
                        &mut intent,
                        dry_run,
                    )
                },
            ),
        )
        .await?;
    let effective_cmd = remote_view_open_command_with_effective_intent(&cmd, &intent);
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
        return Ok(RouteBoundDirectOpenResult::Planned(
            RouteBoundOpenDocument::new(planned_route_bound_handoff_response(
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
            )),
        ));
    }
    supervisor
        .forward(
            "repository_persist_route_pool",
            repository.execute(
                "repository_persist_route_pool",
                supervisor.forward_repository_lock_timeout(),
                |repository| {
                    remote_view_open_persist_request_route_pool(
                        repository,
                        &inline_route_pool_entries,
                    )
                },
            ),
        )
        .await?;
    let observed_at = service_remote_view_timestamp();
    let acquisition_lease = supervisor
        .forward(
            "repository_reserve_acquisition",
            repository.execute(
                "repository_reserve_acquisition",
                supervisor.forward_repository_lock_timeout(),
                |repository| {
                    begin_route_bound_handoff_plan_acquisition(
                        repository,
                        inline_route_pool_entry.as_ref(),
                        &acquisition_plan,
                        &browser_id,
                        &session_id,
                        &observed_at,
                    )
                },
            ),
        )
        .await?;
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
            let failure = supervisor
                .compensate(
                    "repository_display_access_failure",
                    repository.execute(
                        "repository_display_access_failure",
                        supervisor.compensation_repository_lock_timeout(),
                        |repository| {
                            route_bound_handoff_immediate_failure(
                                repository,
                                RouteBoundHandoffImmediateFailureInput {
                                    lease: &acquisition_lease,
                                    phase: "display_access_failed",
                                    error: &error_message,
                                    cleanup: &cleanup,
                                    observed_at: &observed_at,
                                },
                            )
                        },
                    ),
                )
                .await?;
            return Err(route_bound_execution_error_with_cleanup(
                error,
                "display_access_failed",
                failure.rollback,
                &failure.summary,
            ));
        }
    }
    .into_value();
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
                    command: effective_launch_command.into(),
                }),
            )
            .await
        {
            Ok(launch) => launch.into_value(),
            Err(error) => {
                let error_message = error.compatibility_message().to_string();
                let cleanup = route_bound_handoff_launch_failure_cleanup("browser_launch_failed");
                let observed_at = service_remote_view_timestamp();
                let failure = supervisor
                    .compensate(
                        "repository_browser_launch_failure",
                        repository.execute(
                            "repository_browser_launch_failure",
                            supervisor.compensation_repository_lock_timeout(),
                            |repository| {
                                route_bound_handoff_immediate_failure(
                                    repository,
                                    RouteBoundHandoffImmediateFailureInput {
                                        lease: &acquisition_lease,
                                        phase: "browser_launch_failed",
                                        error: &error_message,
                                        cleanup: &cleanup,
                                        observed_at: &observed_at,
                                    },
                                )
                            },
                        ),
                    )
                    .await?;
                return Err(route_bound_execution_error_with_cleanup(
                    error,
                    "browser_launch_failed",
                    failure.rollback,
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
                    repository,
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
                failure_context.phase,
                failure.rollback,
                &failure.summary,
            ));
        }
    };
    let focus_command = route_bound_handoff_focus_command(&cmd, &tab, &session_id);
    let focus = match supervisor
        .forward(
            "focus_target",
            runtime.focus_target(FocusTargetRequest {
                command: focus_command.into(),
            }),
        )
        .await
    {
        Ok(focus) => focus.into_value(),
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_focus_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository,
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
                failure_context.phase,
                failure.rollback,
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
        Ok(proof) => proof.into_value(),
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_visible_window_proof_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository,
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
                failure_context.phase,
                failure.rollback,
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
    route_binding = route_binding_with_operator_access(
        route_binding,
        operator_access.map(OperatorAccessResult::into_value),
    );
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
                repository,
                lease: &acquisition_lease,
                phase: "proof_failed",
                error: &handoff_failure.error,
                rollback_cleanup: &handoff_failure.cleanup,
                launch: &launch,
                tab: Some(&tab),
            },
        )
        .await?;
        return Err(route_bound_message_error_with_cleanup(
            "proof_failed",
            handoff_failure.error,
            failure.rollback,
            &failure.summary,
        ));
    }
    let checkout_command = route_bound_handoff_checkout_command_with_visible_window_proof(
        &checkout_command,
        &visible_window_proof,
    );
    let checkout = match supervisor
        .forward(
            "checkout_route",
            runtime.checkout_route(CheckoutRouteRequest {
                command: checkout_command.clone().into(),
            }),
        )
        .await
    {
        Ok(checkout) => checkout.into_value(),
        Err(error) => {
            let error_message = error.compatibility_message().to_string();
            let failure_context = route_bound_handoff_checkout_failure();
            let failure = remote_view_open_rollback_failure_after_cleanup(
                runtime,
                supervisor,
                RemoteViewOpenFailureCleanupInput {
                    repository,
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
                failure_context.phase,
                failure.rollback,
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
                repository,
                lease: &acquisition_lease,
                phase: "final_proof_failed",
                error: &handoff_failure.error,
                rollback_cleanup: &handoff_failure.cleanup,
                launch: &launch,
                tab: Some(&tab),
            },
        )
        .await?;
        return Err(route_bound_message_error_with_cleanup(
            "final_proof_failed",
            handoff_failure.error.clone(),
            failure.rollback,
            &failure.summary,
        ));
    }
    let observed_at = service_remote_view_timestamp();
    let opened = supervisor
        .forward(
            "repository_finalize_open",
            repository.execute(
                "repository_finalize_open",
                supervisor.forward_repository_lock_timeout(),
                |repository| {
                    complete_route_bound_handoff_open(CompleteRouteBoundHandoffOpenInput {
                        handoff_id: handoff_id.as_deref(),
                        intent: &intent,
                        planned_route_binding: &route_binding,
                        acquisition_plan: &acquisition_plan,
                        repository,
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
                    })
                },
            ),
        )
        .await?;
    Ok(RouteBoundDirectOpenResult::Opened(
        RouteBoundOpenDocument::new(opened),
    ))
}
/// Resolve an opaque remote-view handoff by reacquiring ephemeral route state
/// and preferring the originally retained browser target when it still exists.
pub(crate) async fn execute_durable_resolution<
    R: RouteBoundOpenRuntime,
    P: RouteBoundOpenRepository,
>(
    handoff_id: String,
    allow_reopen_closed: bool,
    attribution: RouteBoundOpenAttribution,
    runtime: &mut R,
    repository: &P,
    supervisor: &RouteBoundOpenSupervisor,
) -> Result<RouteBoundOpenOutcome, RouteBoundOpenExecutionError> {
    let service_state = supervisor
        .forward(
            "repository_load_handoff_snapshot",
            repository.snapshot(supervisor.forward_repository_lock_timeout()),
        )
        .await?;
    let Some(handoff) = service_state.remote_view_handoffs.get(&handoff_id).cloned() else {
        return Ok(RouteBoundOpenOutcome::NotFound {
            result: RouteBoundOpenDocument::new(json!(
            { "status" : "not_found", "resolved" : false, "handoffId" : handoff_id,
            "message" : "Remote-view handoff was not found", }
            )),
        });
    };
    let resolution_snapshot = RouteBoundResolutionSnapshot {
        state: service_state,
        handoff: handoff.clone(),
        loaded_at: service_remote_view_timestamp(),
    };
    let service_state = &resolution_snapshot.state;
    let handoff = &resolution_snapshot.handoff;
    if !allow_reopen_closed && remote_view_handoff_was_explicitly_closed(service_state, handoff) {
        return Ok(RouteBoundOpenOutcome::ExplicitlyClosed {
            result: RouteBoundOpenDocument::new(json!(
            { "status" : "closed", "resolved" : false, "reopenRequired" : true,
            "handoffId" : handoff.id, "handoffUrl" : handoff.handoff_url, "browserId"
            : handoff.browser_id, "sessionName" : handoff.session_name, "tabId" :
            handoff.tab_id, "targetId" : handoff.target_id, "viewStreamProvider" :
            handoff.view_stream_provider, "controlInput" : handoff.control_input,
            "message" :
            "The retained tab was deliberately closed. Reopen requires an explicit operator action.",
            }
            )),
        });
    }
    let service_job_id = attribution
        .service_job_id
        .clone()
        .unwrap_or_else(|| format!("resolve:{}", handoff.id));
    let mut resolution_command =
        remote_view_handoff_resolution_command(handoff, &service_job_id, allow_reopen_closed)?;
    apply_retained_remote_view_route(service_state, handoff, &mut resolution_command);
    let authorization = attribution.authorization;
    let direct_request = RouteBoundDirectOpenRequest::from_compatibility_command(
        resolution_command,
        Some(handoff.id.clone()),
        attribution,
    )?;
    let opened = match execute_direct_open(direct_request, runtime, repository, supervisor).await {
        Ok(RouteBoundDirectOpenResult::Planned(plan)) => {
            return Ok(RouteBoundOpenOutcome::Planned { plan });
        }
        Ok(RouteBoundDirectOpenResult::Opened(opened)) => opened.into_value(),
        Err(error) => {
            if let Some(fallback) = remote_view_handoff_provider_fallback_if_eligible(
                &resolution_snapshot,
                error.runtime_issue.as_ref(),
                authorization,
                runtime,
                supervisor,
            )
            .await
            {
                return Ok(RouteBoundOpenOutcome::ProviderFallback { fallback });
            }
            return Err(error);
        }
    };
    let opened = RouteBoundOpenDocument::new(json!(
        { "status" : "ready", "resolved" : true, "reopenedClosedTab" :
        allow_reopen_closed, "handoffId" : handoff.id, "handoffUrl" : opened
        .get("handoffUrl"), "externalUrl" : opened.get("externalUrl"),
        "providerExternalUrl" : opened.get("providerExternalUrl"), "browserId" :
        opened.get("browserId"), "sessionName" : opened.get("sessionName"), "tab" :
        opened.get("tab"), "viewStreamProvider" : handoff.view_stream_provider,
        "controlInput" : handoff.control_input, "open" : opened, }
    ));
    if allow_reopen_closed {
        Ok(RouteBoundOpenOutcome::Reopened { opened })
    } else {
        Ok(RouteBoundOpenOutcome::Opened { opened })
    }
}
pub(crate) async fn remote_view_handoff_provider_fallback_if_eligible<R: RouteBoundOpenRuntime>(
    snapshot: &RouteBoundResolutionSnapshot,
    issue: Option<&RouteBoundRuntimeIssue>,
    authorization: RouteBoundOpenAuthorization,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
) -> Option<RouteBoundOpenDocument> {
    let service_state = &snapshot.state;
    let handoff = &snapshot.handoff;
    let RouteBoundRuntimeIssue::RequestedProfileInUseByPid {
        profile_id,
        owner_browser_id,
        owner_session_id,
        ..
    } = issue?
    else {
        return None;
    };
    let route = handoff
        .last_route_id
        .as_ref()
        .and_then(|route_id| service_state.remote_view_routes.get(route_id));
    let exact_owner = handoff.profile_id.as_deref() == Some(profile_id.as_str())
        && owner_browser_id.as_deref() == handoff.browser_id.as_deref()
        && owner_session_id.as_deref() == handoff.session_name.as_deref();
    let retained_route = route.is_some_and(|route| {
        route.provider == ViewStreamProvider::RdpGateway
            && route.browser_id.as_deref() == handoff.browser_id.as_deref()
            && route.session_id.as_deref() == handoff.session_name.as_deref()
            && route.state != "released"
            && (route
                .external_url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
                || route
                    .route_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.get("publicOperatorUrl"))
                    .and_then(Value::as_str)
                    .is_some_and(|url| !url.trim().is_empty()))
    });
    let mut eligibility = RouteBoundFallbackEligibility {
        prior_provider: handoff.view_stream_provider == Some(ViewStreamProvider::RdpGateway),
        snapshot_identity: handoff.last_route_id.is_some()
            && handoff.browser_id.is_some()
            && handoff.session_name.is_some()
            && handoff.profile_id.is_some(),
        snapshot_timing: !snapshot.loaded_at.is_empty(),
        exact_ownership_cause: exact_owner,
        retained_route,
        authorized_ingress: authorization.is_authorized(),
        operator_evidence: false,
        browser_preserved: exact_owner,
        duplicate_lane_prohibited: exact_owner,
    };
    if !eligibility.prior_provider
        || !eligibility.snapshot_identity
        || !eligibility.snapshot_timing
        || !eligibility.exact_ownership_cause
        || !eligibility.retained_route
        || !eligibility.authorized_ingress
        || !eligibility.browser_preserved
        || !eligibility.duplicate_lane_prohibited
    {
        return None;
    }
    let route = route?;
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
        .flatten()?
        .into_value();
    if readiness_state(&operator_access).as_deref() != Some("ready") {
        return None;
    }
    eligibility.operator_evidence = true;
    if !eligibility.is_eligible() {
        return None;
    }
    let mut fallback = remote_view_handoff_provider_fallback_response(service_state, handoff)?;
    fallback["resolutionSnapshotLoadedAt"] = Value::String(snapshot.loaded_at.clone());
    fallback["fallbackEligibility"] = serde_json::to_value(eligibility).ok()?;
    Some(RouteBoundOpenDocument::new(fallback))
}
pub(crate) fn remote_view_handoff_provider_fallback_response(
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
