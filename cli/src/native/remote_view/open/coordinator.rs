#![allow(unused_imports)]
use super::compensation::{
    remote_view_open_rollback_failure_after_cleanup, RemoteViewOpenFailureCleanupInput,
};
use super::deadline::{
    route_bound_execution_error_with_cleanup, RouteBoundOpenExecutionError,
    RouteBoundOpenSupervisor, RouteBoundRuntimeIssue,
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
    ActionsRouteBoundOpenRuntime, CheckoutRouteRequest, DisplayAccessRequest, FocusTargetRequest,
    LaunchBrowserRequest, OperatorAccessRequest, RouteBoundOpenRuntime, VisibleWindowRequest,
};
use super::shared::*;
use super::target::route_bound_open_acquire_target;
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
    pub(crate) fn into_compatibility_result(self) -> Result<Value, String> {
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
                let mut request = json!(
                    { "handoffId" : handoff_id, "allowReopenClosed" :
                    allow_reopen_closed, }
                );
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
pub(crate) fn rolled_back_outcome(error: String) -> Result<RouteBoundOpenOutcome, String> {
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
pub(crate) async fn execute_direct_open<R: RouteBoundOpenRuntime>(
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
pub(crate) async fn execute_durable_resolution<R: RouteBoundOpenRuntime>(
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
pub(crate) async fn typed_remote_view_handoff_provider_fallback<R: RouteBoundOpenRuntime>(
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
