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
    remote_view_open_should_reuse_current_browser,
    remote_view_open_should_reuse_current_browser_for_durable_resolution,
    service_remote_view_acquisition_plan_from_state,
};
use super::route_lifecycle::service_remote_view_timestamp;
use super::runtime::{
    AdoptRetainedBrowserRequest, CheckoutRouteRequest, DaemonRouteBoundOpenRepository,
    DaemonRouteBoundOpenRuntime, DisplayAccessRequest, FocusTargetRequest, LaunchBrowserRequest,
    OperatorAccessRequest, OperatorAccessResult, RouteBoundBrowserObservation,
    RouteBoundOpenRepository, RouteBoundOpenRuntime, VisibleWindowRequest,
};
use super::runtime_model::*;
use super::shared::*;
use super::target::route_bound_open_acquire_target;
use crate::native::remote_view::RemoteViewOpenIntent;
use crate::runtime_owner_transfer::ProfileOwnerState;
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
    pub(crate) dashboard_deployment_generation: Option<String>,
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
        dashboard_deployment_generation: optional_command_string(
            cmd,
            "dashboardDeploymentGeneration",
        ),
        authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
    }
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
    prefer_existing_browser: bool,
    retained_handoff: Option<RemoteViewHandoff>,
    require_retained_browser: bool,
    dashboard_deployment_generation: Option<String>,
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
            prefer_existing_browser: false,
            retained_handoff: None,
            require_retained_browser: false,
            dashboard_deployment_generation: attribution.dashboard_deployment_generation,
        })
    }

    fn prefer_existing_browser(mut self, handoff: RemoteViewHandoff) -> Self {
        self.prefer_existing_browser = true;
        self.retained_handoff = Some(handoff);
        self
    }

    fn require_retained_browser(mut self) -> Self {
        self.require_retained_browser = true;
        self
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
    Converging {
        result: RouteBoundOpenDocument,
    },
    RolledBack {
        blocker: RouteBoundOpenBlocker,
        compensation: RouteBoundOpenCompensation,
        compatibility_error: String,
    },
}
impl RouteBoundOpenOutcome {
    pub(crate) fn into_compatibility_result(self) -> Result<Value, String> {
        match self {
            Self::Planned { plan }
            | Self::NotFound { result: plan }
            | Self::ExplicitlyClosed { result: plan }
            | Self::Converging { result: plan }
            | Self::Reopened { opened: plan }
            | Self::Opened { opened: plan } => Ok(plan.into_value()),
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
    mut attribution: RouteBoundOpenAttribution,
) -> Result<Value, String> {
    let handoff_id = optional_command_or_params_string(cmd, "handoffId")
        .or_else(|| optional_command_or_params_string(cmd, "remoteViewHandoffId"))
        .ok_or_else(|| "service_remote_view_handoff_resolve requires handoffId".to_string())?;
    let allow_reopen_closed =
        optional_command_or_params_bool(cmd, "allowReopenClosed").unwrap_or(false);
    if attribution.dashboard_deployment_generation.is_none() {
        attribution.dashboard_deployment_generation =
            crate::dashboard_ingress::selected_dashboard_generation().ok();
    }
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
    let mut cmd = request.command();
    let mut intent = request.intent;
    let handoff_id = request.handoff_id;
    let prefer_existing_browser = request.prefer_existing_browser;
    let retained_handoff = request.retained_handoff;
    let require_retained_browser = request.require_retained_browser;
    let dashboard_deployment_generation = request.dashboard_deployment_generation;
    let mut initial_browser = supervisor
        .forward("observe_browser", runtime.observe_browser())
        .await?;
    if require_retained_browser && !initial_browser.browser_present {
        let source_session = retained_handoff
            .as_ref()
            .and_then(|handoff| handoff.session_name.clone())
            .ok_or_else(|| {
                RouteBoundRuntimeIssue::EffectFailed {
                    operation: "adopt_retained_browser",
                    message: "durable_handoff_session_missing: retained browser adoption requires the original daemon lane".to_string(),
                }
            })?;
        initial_browser = supervisor
            .forward(
                "adopt_retained_browser",
                runtime.adopt_retained_browser(AdoptRetainedBrowserRequest { source_session }),
            )
            .await?;
    }
    if require_retained_browser {
        let handoff =
            retained_handoff
                .as_ref()
                .ok_or_else(|| RouteBoundRuntimeIssue::EffectFailed {
                    operation: "validate_retained_browser",
                    message:
                        "durable_handoff_identity_missing: retained browser identity is required"
                            .to_string(),
                })?;
        if !durable_handoff_observation_matches(&initial_browser, handoff) {
            return Err(RouteBoundRuntimeIssue::EffectFailed {
                operation: "validate_retained_browser",
                message: "durable_handoff_target_unavailable: the exact retained browser target is not attached"
                    .to_string(),
            }
            .into());
        }
    }
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
    let reuse_durable_browser = prefer_existing_browser
        && remote_view_open_should_reuse_current_browser_for_durable_resolution(
            &initial_browser,
            &intent,
            &browser_id,
            &session_id,
            &service_state,
        );
    if reuse_durable_browser {
        if let Some(retained_handoff) = retained_handoff.as_ref() {
            apply_available_retained_remote_view_route(&service_state, retained_handoff, &mut cmd);
        }
    }
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
    ) || reuse_durable_browser;
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
                        dashboard_deployment_generation: dashboard_deployment_generation.as_deref(),
                    })
                },
            ),
        )
        .await?;
    Ok(RouteBoundDirectOpenResult::Opened(
        RouteBoundOpenDocument::from_compatibility(opened)?,
    ))
}
/// Resolve an opaque remote-view handoff by adopting the exact retained browser
/// and reacquiring presentation without navigation or provider substitution.
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
    let service_state = &service_state;
    if !allow_reopen_closed && remote_view_handoff_was_explicitly_closed(service_state, &handoff) {
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
    let required_dashboard_generation = attribution.dashboard_deployment_generation.clone();
    let previous_presentation_generation = handoff
        .presentation_receipt
        .as_ref()
        .map(|receipt| receipt.generation)
        .unwrap_or(0);
    let mut resolution_command =
        remote_view_handoff_resolution_command(&handoff, &service_job_id, allow_reopen_closed)?;
    apply_retained_remote_view_route(service_state, &handoff, &mut resolution_command);
    let mut direct_request = RouteBoundDirectOpenRequest::from_compatibility_command(
        resolution_command,
        Some(handoff.id.clone()),
        attribution,
    )?
    .prefer_existing_browser(handoff.clone());
    if !allow_reopen_closed {
        direct_request = direct_request.require_retained_browser();
    }
    let opened = match execute_direct_open(direct_request, runtime, repository, supervisor).await {
        Ok(RouteBoundDirectOpenResult::Planned(plan)) => {
            return Ok(RouteBoundOpenOutcome::Planned { plan });
        }
        Ok(RouteBoundDirectOpenResult::Opened(opened)) => opened.into_value(),
        Err(error)
            if error.runtime_issue.as_ref().is_some_and(|issue| {
                matches!(
                    issue,
                    RouteBoundRuntimeIssue::EffectFailed {
                        operation: "adopt_retained_browser" | "validate_retained_browser",
                        ..
                    }
                )
            }) =>
        {
            return Ok(RouteBoundOpenOutcome::Converging {
                result: RouteBoundOpenDocument::from_compatibility(json!({
                    "status": "converging",
                    "resolved": false,
                    "handoffId": handoff.id,
                    "handoffUrl": handoff.handoff_url,
                    "browserId": handoff.browser_id,
                    "sessionName": handoff.session_name,
                    "message": error.message,
                    "retryable": true,
                    "requiredViewStreamProvider": handoff.view_stream_provider,
                }))?,
            });
        }
        Err(error) => return Err(error),
    };
    let presentation_state = supervisor
        .forward(
            "repository_load_presentation_receipt",
            repository.snapshot(supervisor.forward_repository_lock_timeout()),
        )
        .await?;
    let presentation = presentation_state
        .remote_view_handoffs
        .get(&handoff.id)
        .and_then(|handoff| handoff.presentation_receipt.clone());
    let presentation_owner_matches = presentation.as_ref().is_some_and(|receipt| {
        presentation_state
            .runtime_owner_registry
            .owners
            .values()
            .any(|owner| {
                owner.state == ProfileOwnerState::Ready
                    && owner.browser_id == receipt.logical_browser_id
                    && owner.daemon_session_route
                        == handoff.session_name.as_deref().unwrap_or_default()
                    && Some(owner.owner_generation) == receipt.daemon_owner_generation
                    && receipt.process_instance_digest.as_deref()
                        == Some(owner.process_instance_digest.as_str())
            })
    });
    let presentation_matches = presentation_owner_matches
        && presentation.as_ref().is_some_and(|receipt| {
            receipt.state == "ready"
                && receipt.generation > previous_presentation_generation
                && required_dashboard_generation.as_deref()
                    == Some(receipt.dashboard_deployment_generation.as_str())
                && Some(receipt.logical_browser_id.as_str()) == handoff.browser_id.as_deref()
                && (allow_reopen_closed
                    || Some(receipt.target_id.as_str()) == handoff.target_id.as_deref())
                && Some(receipt.required_stream_provider) == handoff.view_stream_provider
                && receipt.observed_stream_provider == receipt.required_stream_provider
        });
    if !presentation_matches {
        return Ok(RouteBoundOpenOutcome::Converging {
            result: RouteBoundOpenDocument::from_compatibility(json!({
                "status": "converging",
                "resolved": false,
                "handoffId": handoff.id,
                "handoffUrl": handoff.handoff_url,
                "browserId": handoff.browser_id,
                "sessionName": handoff.session_name,
                "message": "The retained browser is attached, but its authenticated presentation generation is still converging.",
                "retryable": true,
                "requiredViewStreamProvider": handoff.view_stream_provider,
                "presentationReceipt": presentation,
            }))?,
        });
    }
    let presentation_generation = presentation
        .as_ref()
        .map(|receipt| receipt.generation)
        .unwrap_or(0);
    let presentation_target_id = presentation
        .as_ref()
        .map(|receipt| receipt.target_id.clone())
        .or_else(|| handoff.target_id.clone());
    let opened = RouteBoundOpenDocument::from_compatibility(json!(
        { "status" : "ready", "resolved" : true, "reopenedClosedTab" :
        allow_reopen_closed, "handoffId" : handoff.id, "handoffUrl" : opened
        .get("handoffUrl"), "externalUrl" : opened.get("externalUrl"),
        "providerExternalUrl" : opened.get("providerExternalUrl"), "browserId" :
        opened.get("browserId"), "sessionName" : opened.get("sessionName"), "tabId":
        handoff.tab_id, "targetId": presentation_target_id, "tab" : opened.get("tab"),
        "viewStreamProvider" : handoff.view_stream_provider,
        "requiredViewStreamProvider" : handoff.view_stream_provider,
        "controlInput" : handoff.control_input, "presentationGeneration":
        presentation_generation, "presentationReceipt": presentation, "open" : opened, }
    ))?;
    if allow_reopen_closed {
        Ok(RouteBoundOpenOutcome::Reopened { opened })
    } else {
        Ok(RouteBoundOpenOutcome::Opened { opened })
    }
}

fn durable_handoff_observation_matches(
    observation: &RouteBoundBrowserObservation,
    handoff: &RemoteViewHandoff,
) -> bool {
    let target_matches = handoff.target_id.as_deref().is_some_and(|target_id| {
        observation.active_target_id.as_deref() == Some(target_id)
            || observation
                .pages
                .iter()
                .any(|page| page.target_id == target_id)
    });
    observation.browser_present
        && handoff.browser_id.as_deref() == Some(observation.browser_id.as_str())
        && handoff.session_name.as_deref() == Some(observation.session_id.as_str())
        && handoff
            .profile_id
            .as_deref()
            .is_none_or(|profile_id| observation.runtime_profile.as_deref() == Some(profile_id))
        && target_matches
}
