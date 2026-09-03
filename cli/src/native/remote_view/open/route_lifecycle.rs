#![allow(unused_imports)]
use super::planner::{
    ensure_remote_view_route_available_for_display, inline_route_pool_entry_from_command,
    push_remote_view_service_event, remote_view_lease_is_active,
    service_remote_view_acquisition_plan_from_state,
};
use super::proof::command_object_with_action;
use super::route_pool::{
    merge_route_into_checkout, merge_route_pool_entry_into_checkout, merge_stream_into_checkout,
    select_browser_reattach_route_pool_entry,
};
use super::runtime::observe_daemon_browser;
use super::shared::*;
use crate::native::desktop_control_coordinator::begin_service_controller_mutation;
use crate::native::presentation_capacity::{
    CapacityDecision, PresentationRequest, PressureAdmission,
};
use crate::native::remote_view::operator_visible_browser_window_proof_for_process;
use crate::native::service_model::advance_route_controller_authority;

#[derive(Debug, Clone)]
struct BoundRecoveryReservation {
    request_id: String,
    slot_id: String,
    pressure: PressureAdmission,
}

fn reserve_bound_recovery<R>(
    repository: &R,
    browser_id: &str,
    route_id: &str,
    display_allocation_id: &str,
    route_switch: bool,
) -> Result<(Option<BoundRecoveryReservation>, Value), String>
where
    R: ServiceStateRepository,
{
    let request_id = format!(
        "remote-view-recovery:{}:{}",
        browser_id,
        service_remote_view_timestamp()
    );
    let mut unavailable = None;
    let reservation = repository.mutate(|state| {
        let Some(mut capacity) = state.presentation_capacity.take() else {
            return Ok(None);
        };
        let pressure = PressureAdmission::admit(capacity.config.hard_maximum);
        let request = PresentationRequest::recovery(request_id.clone()).for_browser(browser_id);
        let decision = if route_switch {
            capacity.request_bound_route_switch_recovery(
                request,
                pressure,
                state,
                route_id,
                display_allocation_id,
            )
        } else {
            capacity.request_bound_recovery(
                request,
                pressure,
                state,
                route_id,
                display_allocation_id,
            )
        };
        state.presentation_capacity = Some(capacity);
        match decision {
            CapacityDecision::Granted { slot_id, .. } => Ok(Some(BoundRecoveryReservation {
                request_id: request_id.clone(),
                slot_id,
                pressure,
            })),
            decision => {
                unavailable = Some(decision);
                Err("presentation_recovery_not_admitted".to_string())
            }
        }
    });
    match reservation {
        Ok(Some(reservation)) => Ok((
            Some(reservation.clone()),
            json!({
                "status": "granted",
                "priority": "recovery",
                "requestId": reservation.request_id,
                "slotId": reservation.slot_id,
            }),
        )),
        Ok(None) => Ok((
            None,
            json!({
                "status": "not_configured",
                "priority": "recovery",
                "reason": "presentation_capacity_unavailable",
            }),
        )),
        Err(_) if unavailable.is_some() => Err(format!(
            "presentation_recovery_not_admitted: {:?}",
            unavailable.expect("checked above")
        )),
        Err(error) => Err(format!("presentation_recovery_persistence_failed: {error}")),
    }
}

fn release_bound_recovery<R>(
    repository: &R,
    reservation: &BoundRecoveryReservation,
) -> Result<Value, String>
where
    R: ServiceStateRepository,
{
    repository.mutate(|state| {
        let Some(mut capacity) = state.presentation_capacity.take() else {
            return Err("presentation_capacity_unavailable".to_string());
        };
        let result = capacity.release_bound_presentation(
            &reservation.slot_id,
            &reservation.request_id,
            reservation.pressure,
            state,
        );
        state.presentation_capacity = Some(capacity);
        result?;
        Ok(json!({
            "status": "released",
            "priority": "recovery",
            "requestId": reservation.request_id,
            "slotId": reservation.slot_id,
        }))
    })
}

/// Reattach or move one retained remote-headed browser. Configured
/// presentation capacity is reserved at recovery priority before any route
/// mutation and released after success or failure.
pub(crate) async fn handle_service_remote_view_browser_reattach(
    cmd: &Value,
    daemon_state: &mut DaemonState,
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
    let (recovery_reservation, recovery_admission) = reserve_bound_recovery(
        &repository,
        &browser_id,
        &selected_route_id,
        &display_allocation_id,
        route_switch,
    )?;
    if !route_switch && recovery_reservation.is_none() {
        return Err(
            "operator_presentation_authority_unavailable: presentation recovery capacity is not configured"
                .to_string(),
        );
    }
    let operation_result: Result<Value, String> = async {
        let (focus, visible_window_proof) = if route_switch {
            (
                json!({
                    "state": "not_requested",
                    "reason": "route_switch_uses_existing_transition_contract",
                }),
                Value::Null,
            )
        } else {
            let display_name = checkout
            .get("launchDisplayName")
            .or_else(|| checkout.get("displayName"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                snapshot
                    .display_allocations
                    .get(&display_allocation_id)
                    .and_then(|allocation| allocation.display_name.clone())
            })
            .ok_or_else(|| {
                format!(
                    "operator_presentation_display_missing: route '{}' has no launch display",
                    selected_route_id
                )
            })?;
            let before_focus = observe_daemon_browser(daemon_state)
            .await
            .map_err(|error| error.compatibility_message().to_string())?;
        if !before_focus.browser_present
            || before_focus.browser_id != browser_id
            || before_focus.session_id != session_name
        {
            return Err(format!(
                "operator_presentation_identity_mismatch: expected browser '{}' session '{}', observed browser '{}' session '{}' present={}",
                browser_id,
                session_name,
                before_focus.browser_id,
                before_focus.session_id,
                before_focus.browser_present
            ));
        }
            let focus = handle_view_focus(
            &json!({
                "action": "view_focus",
                "maximize": true,
            }),
            daemon_state,
        )
        .await
        .map_err(|error| format!("operator_presentation_focus_failed: {error}"))?;
            let after_focus = observe_daemon_browser(daemon_state)
            .await
            .map_err(|error| error.compatibility_message().to_string())?;
        if !after_focus.browser_present
            || after_focus.browser_id != before_focus.browser_id
            || after_focus.session_id != before_focus.session_id
            || after_focus.browser_pid != before_focus.browser_pid
        {
            return Err(
                "operator_presentation_identity_changed: retained browser identity changed during focus"
                    .to_string(),
            );
        }
            let browser_pid = after_focus.browser_pid.ok_or_else(|| {
                "operator_presentation_process_missing: retained browser has no process identity"
                    .to_string()
            })?;
            let visible_window_proof = operator_visible_browser_window_proof_for_process(
                &selected_route_id,
                &display_name,
                browser_pid,
            )?;
            (focus, visible_window_proof)
        };
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
        if !visible_window_proof.is_null() {
            checkout.insert(
                "readiness".to_string(),
                json!({
                    "state": "ready",
                    "component": "operator_visible_window",
                    "displayContent": visible_window_proof["displayContent"].clone(),
                }),
            );
        }
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
            "checkout" : checkout_result, "focus" : focus,
            "operatorVisible" : visible_window_proof, }
        ))
    }
    .await;
    let recovery_release = recovery_reservation
        .as_ref()
        .map(|reservation| release_bound_recovery(&repository, reservation))
        .transpose();
    match (operation_result, recovery_release) {
        (Ok(mut response), Ok(recovery_release)) => {
            response["recoveryAdmission"] = recovery_admission;
            response["recoveryRelease"] = recovery_release.unwrap_or_else(|| {
                json!({
                    "status": "not_configured",
                    "priority": "recovery",
                    "reason": "presentation_capacity_unavailable",
                })
            });
            Ok(response)
        }
        (Err(operation_error), Ok(_)) => Err(operation_error),
        (Ok(_), Err(release_error)) => Err(format!(
            "presentation_recovery_release_failed: {release_error}"
        )),
        (Err(operation_error), Err(release_error)) => Err(format!(
            "{operation_error}; presentation_recovery_release_failed: {release_error}"
        )),
    }
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
            &session_id,
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
        let browser_snapshot = state.browsers.get(&browser_id).cloned();
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
        if let Some(browser) = browser_snapshot.as_ref() {
            display_allocation.profile_id = browser.profile_id.clone();
            // A replacement browser can be adopted as `attached_existing` while it
            // still owns a service-managed remote-headed display.  The allocation
            // describes that display workspace, not the browser adoption path, so
            // keep its remote-headed identity for desktop capture validation.
            display_allocation.host = Some(ServiceBrowserHost::RemoteHeaded);
            display_allocation.pid_hints = browser
                .pid
                .map(|browser_pid| json!({ "browserPid": browser_pid }));
        }
        if intent.browser_build.is_some() {
            display_allocation.browser_build = intent.browser_build.clone();
        }
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
        if let Some(capacity) = state.presentation_capacity.as_mut() {
            capacity.activate_bound_browser(&route_id, &display_allocation_id, &browser_id)?;
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
    let snapshot = repository.load_snapshot()?;
    let _controller_mutation = begin_service_controller_mutation(&snapshot, &route_id)?;
    repository.mutate(|state| {
        let park_for_route_switch = optional_command_or_params_bool(cmd, "parkForRouteSwitch")
            .or_else(|| optional_command_or_params_bool(cmd, "releaseDisplayAllocation"))
            .unwrap_or(false);
        let route = state
            .remote_view_routes
            .get(&route_id)
            .cloned()
            .ok_or_else(|| format!("remote view route '{}' not found", route_id))?;
        let display_allocation_id = route.display_allocation_id.clone();
        let browser_id = route.browser_id.clone();
        let session_id = route.session_id.clone();
        let viewer_lease_ids = route.viewer_lease_ids.clone();
        if let (Some(capacity), Some(display_allocation_id), Some(browser_id)) = (
            state.presentation_capacity.as_mut(),
            display_allocation_id.as_deref(),
            browser_id.as_deref(),
        ) {
            if park_for_route_switch {
                capacity.release_bound_browser_for_route_switch(
                    &route_id,
                    display_allocation_id,
                    browser_id,
                )?;
            } else {
                capacity.release_bound_browser(&route_id, display_allocation_id, browser_id)?;
            }
        }
        if route.controller_lease_id.is_some() {
            advance_route_controller_authority(state, &route_id, None)?;
        }
        let released_route = state
            .remote_view_routes
            .get_mut(&route_id)
            .ok_or_else(|| format!("remote view route '{}' not found", route_id))?;
        released_route.state = "released".to_string();
        released_route.last_provider_event = Some("route_released".to_string());
        let released_route = released_route.clone();
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
                        stream.project_controller(&released_route);
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
    browser: &mut super::super::super::service_model::BrowserProcess,
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
        stream.project_controller(route);
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
