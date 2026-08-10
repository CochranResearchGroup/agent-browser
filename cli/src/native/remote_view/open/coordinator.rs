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
use super::runtime_model::*;
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

/// Construct attribution at the authenticated `execute_command` boundary.
/// Socket clients reach that boundary only after daemon-token validation;
/// in-process CLI and service adapters are already inside the trusted process.
pub(crate) fn route_bound_open_attribution_from_authenticated_dispatch(
    cmd: &Value,
) -> RouteBoundOpenAttribution {
    RouteBoundOpenAttribution {
        caller_id: optional_command_string(cmd, "callerId"),
        service_job_id: optional_command_string(cmd, "serviceJobId"),
        authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
    }
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
    pub(crate) immutable_snapshot_exists: bool,
    pub(crate) explicit_close_allows_resolution: bool,
    pub(crate) exact_opaque_rdp_identity: bool,
    pub(crate) typed_retained_owner_conflict: bool,
    pub(crate) current_bounded_route: bool,
    pub(crate) operator_access_succeeded: bool,
    pub(crate) best_effort_result: bool,
    pub(crate) no_new_ownership: bool,
    pub(crate) retained_browser_and_unrelated_tabs_unchanged: bool,
}

impl RouteBoundFallbackEligibility {
    pub(crate) fn is_eligible(&self) -> bool {
        self.immutable_snapshot_exists
            && self.explicit_close_allows_resolution
            && self.exact_opaque_rdp_identity
            && self.typed_retained_owner_conflict
            && self.current_bounded_route
            && self.operator_access_succeeded
            && self.best_effort_result
            && self.no_new_ownership
            && self.retained_browser_and_unrelated_tabs_unchanged
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RouteBoundFallbackMode {
    BestEffort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundOpenDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) resolved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reopen_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reopened_closed_tab: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) handoff_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) handoff_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) external_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provider_external_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) browser_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) session_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) message: Option<String>,
    #[serde(default, flatten)]
    pub(crate) extensions: Map<String, Value>,
}

impl RouteBoundOpenDocument {
    pub(crate) fn from_compatibility(value: Value) -> Result<Self, String> {
        if !value.is_object() {
            return Err("route-bound open outcome must be a JSON object".to_string());
        }
        serde_json::from_value(value)
            .map_err(|error| format!("route-bound open outcome has invalid typed fields: {error}"))
    }

    pub(crate) fn into_value(self) -> Value {
        serde_json::to_value(self).expect("route-bound open outcome must serialize")
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RouteBoundOpenCommand {
    record: RouteBoundCommandRecord,
}

impl RouteBoundOpenCommand {
    fn into_value(self) -> Value {
        self.record.into_value()
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
            command: RouteBoundOpenCommand {
                record: RouteBoundCommandRecord::from_compatibility(command, "route_bound_open")?,
            },
            intent,
            dry_run,
            handoff_id,
        })
    }

    fn command(&self) -> Value {
        self.command.clone().into_value()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RouteBoundOpenInvocationKind {
    DirectOpen(Box<RouteBoundDirectOpenRequest>),
    DurableResolution {
        handoff_id: String,
        allow_reopen_closed: bool,
        attribution: RouteBoundOpenAttribution,
    },
}

/// An invocation exists only after its ingress supplied an authenticated
/// daemon-command fact. The private inner enum prevents unauthorized callers
/// from assembling coordinator work and relying on a later rejection.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteBoundOpenInvocation {
    kind: RouteBoundOpenInvocationKind,
}

impl RouteBoundOpenInvocation {
    pub(crate) fn direct(request: RouteBoundDirectOpenRequest) -> Self {
        Self {
            kind: RouteBoundOpenInvocationKind::DirectOpen(Box::new(request)),
        }
    }

    pub(crate) fn durable_resolution(
        handoff_id: String,
        allow_reopen_closed: bool,
        attribution: RouteBoundOpenAttribution,
    ) -> Result<Self, String> {
        if !attribution.authorization.is_authorized() {
            return Err("Unauthorized route-bound open invocation".to_string());
        }
        Ok(Self {
            kind: RouteBoundOpenInvocationKind::DurableResolution {
                handoff_id,
                allow_reopen_closed,
                attribution,
            },
        })
    }
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
        match invocation.kind {
            RouteBoundOpenInvocationKind::DirectOpen(request) => {
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
            RouteBoundOpenInvocationKind::DurableResolution {
                handoff_id,
                allow_reopen_closed,
                attribution,
            } => {
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
    attribution: RouteBoundOpenAttribution,
) -> Result<Value, String> {
    let invocation =
        RouteBoundOpenInvocation::direct(RouteBoundDirectOpenRequest::from_compatibility_command(
            cmd.clone(),
            optional_command_string(cmd, "remoteViewHandoffId")
                .or_else(|| optional_command_string(cmd, "serviceJobId")),
            attribution,
        )?);
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
    attribution: RouteBoundOpenAttribution,
) -> Result<Value, String> {
    let handoff_id = optional_command_or_params_string(cmd, "handoffId")
        .or_else(|| optional_command_or_params_string(cmd, "remoteViewHandoffId"))
        .ok_or_else(|| "service_remote_view_handoff_resolve requires handoffId".to_string())?;
    let allow_reopen_closed =
        optional_command_or_params_bool(cmd, "allowReopenClosed").unwrap_or(false);
    let invocation =
        RouteBoundOpenInvocation::durable_resolution(handoff_id, allow_reopen_closed, attribution)?;
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
    let cmd = request.command();
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
            RouteBoundOpenDocument::from_compatibility(planned_route_bound_handoff_response(
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
            ))?,
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
                    command: LaunchBrowserCommand::from_compatibility(effective_launch_command)?,
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
                command: FocusTargetCommand::from_compatibility(focus_command)?,
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
                command: CheckoutRouteCommand::from_compatibility(checkout_command.clone())?,
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
        RouteBoundOpenDocument::from_compatibility(opened)?,
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
            result: RouteBoundOpenDocument::from_compatibility(json!(
            { "status" : "not_found", "resolved" : false, "handoffId" : handoff_id,
            "message" : "Remote-view handoff was not found", }
            ))?,
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
            result: RouteBoundOpenDocument::from_compatibility(json!(
            { "status" : "closed", "resolved" : false, "reopenRequired" : true,
            "handoffId" : handoff.id, "handoffUrl" : handoff.handoff_url, "browserId"
            : handoff.browser_id, "sessionName" : handoff.session_name, "tabId" :
            handoff.tab_id, "targetId" : handoff.target_id, "viewStreamProvider" :
            handoff.view_stream_provider, "controlInput" : handoff.control_input,
            "message" :
            "The retained tab was deliberately closed. Reopen requires an explicit operator action.",
            }
            ))?,
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
            if let Ok(post_attempt_state) = supervisor
                .forward(
                    "repository_load_fallback_post_attempt_snapshot",
                    repository.snapshot(supervisor.forward_repository_lock_timeout()),
                )
                .await
            {
                if let Some(fallback) = remote_view_handoff_provider_fallback_if_eligible(
                    &resolution_snapshot,
                    &post_attempt_state,
                    allow_reopen_closed,
                    error.runtime_issue.as_ref(),
                    authorization,
                    runtime,
                    supervisor,
                )
                .await
                {
                    return Ok(RouteBoundOpenOutcome::ProviderFallback { fallback });
                }
            }
            return Err(error);
        }
    };
    let opened = RouteBoundOpenDocument::from_compatibility(json!(
        { "status" : "ready", "resolved" : true, "reopenedClosedTab" :
        allow_reopen_closed, "handoffId" : handoff.id, "handoffUrl" : opened
        .get("handoffUrl"), "externalUrl" : opened.get("externalUrl"),
        "providerExternalUrl" : opened.get("providerExternalUrl"), "browserId" :
        opened.get("browserId"), "sessionName" : opened.get("sessionName"), "tab" :
        opened.get("tab"), "viewStreamProvider" : handoff.view_stream_provider,
        "controlInput" : handoff.control_input, "open" : opened, }
    ))?;
    if allow_reopen_closed {
        Ok(RouteBoundOpenOutcome::Reopened { opened })
    } else {
        Ok(RouteBoundOpenOutcome::Opened { opened })
    }
}
pub(crate) async fn remote_view_handoff_provider_fallback_if_eligible<R: RouteBoundOpenRuntime>(
    snapshot: &RouteBoundResolutionSnapshot,
    post_attempt_state: &ServiceState,
    allow_reopen_closed: bool,
    issue: Option<&RouteBoundRuntimeIssue>,
    authorization: RouteBoundOpenAuthorization,
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
) -> Option<RouteBoundOpenDocument> {
    if !authorization.is_authorized() {
        return None;
    }
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
    let current_bounded_route = route.is_some_and(|route| {
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
            && post_attempt_state.remote_view_routes.get(&route.id) == Some(route)
    });
    let fallback_mode = RouteBoundFallbackMode::BestEffort;
    let mut eligibility = RouteBoundFallbackEligibility {
        immutable_snapshot_exists: !snapshot.loaded_at.is_empty()
            && service_state.remote_view_handoffs.get(&handoff.id) == Some(handoff),
        explicit_close_allows_resolution: !remote_view_handoff_was_explicitly_closed(
            service_state,
            handoff,
        ) || allow_reopen_closed,
        exact_opaque_rdp_identity: handoff.view_stream_provider
            == Some(ViewStreamProvider::RdpGateway)
            && !handoff.id.trim().is_empty()
            && handoff
                .handoff_url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
            && handoff.last_route_id.is_some()
            && handoff.browser_id.is_some()
            && handoff.session_name.is_some()
            && handoff.profile_id.is_some(),
        typed_retained_owner_conflict: exact_owner,
        current_bounded_route,
        operator_access_succeeded: false,
        best_effort_result: fallback_mode == RouteBoundFallbackMode::BestEffort,
        no_new_ownership: service_state.profiles == post_attempt_state.profiles
            && service_state.browsers == post_attempt_state.browsers
            && service_state.sessions == post_attempt_state.sessions
            && service_state.viewer_leases == post_attempt_state.viewer_leases,
        retained_browser_and_unrelated_tabs_unchanged: service_state.browsers
            == post_attempt_state.browsers
            && service_state.tabs == post_attempt_state.tabs,
    };
    if !eligibility.immutable_snapshot_exists
        || !eligibility.explicit_close_allows_resolution
        || !eligibility.exact_opaque_rdp_identity
        || !eligibility.typed_retained_owner_conflict
        || !eligibility.current_bounded_route
        || !eligibility.best_effort_result
        || !eligibility.no_new_ownership
        || !eligibility.retained_browser_and_unrelated_tabs_unchanged
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
    eligibility.operator_access_succeeded = true;
    if !eligibility.is_eligible() {
        return None;
    }
    let mut fallback = remote_view_handoff_provider_fallback_response(service_state, handoff)?;
    fallback["fallbackMode"] = serde_json::to_value(fallback_mode).ok()?;
    fallback["resolutionSnapshotLoadedAt"] = Value::String(snapshot.loaded_at.clone());
    fallback["fallbackEligibility"] = serde_json::to_value(eligibility).ok()?;
    RouteBoundOpenDocument::from_compatibility(fallback).ok()
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
