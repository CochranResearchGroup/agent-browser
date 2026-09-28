//! Historical JSON route-bound acquisition fixtures. Test-only.

use super::*;
use crate::native::remote_view_finalization::finalize_route_bound_acquisition;
use crate::native::remote_view_lease::{RouteBoundLeaseLifecycle, RouteBoundLeaseState};
use crate::native::service_model::{
    ControlInputProvider, DisplayAllocation, DurableHandoffPresentationReceipt,
    RemoteViewAcquisitionLease, RemoteViewRoute, RoutePoolEntry,
};
use crate::native::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};
use sha2::{Digest, Sha256};

pub type LegacyJsonRepository = LockedServiceStateRepository<JsonServiceStateStore>;

pub struct CompleteRouteBoundHandoffOpenInput<'a> {
    pub handoff_id: Option<&'a str>,
    pub intent: &'a RemoteViewOpenIntent,
    pub planned_route_binding: &'a RemoteViewRouteBinding,
    pub acquisition_plan: &'a RemoteViewAcquisitionPlan,
    pub repository: &'a LockedServiceStateRepository<JsonServiceStateStore>,
    pub lease: &'a RemoteViewAcquisitionLease,
    pub observed_at: &'a str,
    pub browser_id: &'a str,
    pub session_name: &'a str,
    pub managed_one_time_profile: &'a Value,
    pub one_time_profile_warning: &'a Value,
    pub final_operator_visible: &'a Value,
    pub pre_checkout_operator_visible: &'a Value,
    pub launch_command: &'a Value,
    pub launch: &'a Value,
    pub tab: &'a Value,
    pub focus: &'a Value,
    pub checkout: &'a Value,
    pub display_access_grant: &'a Value,
    pub reused_current_browser: bool,
    pub visible_window_proof: &'a Value,
    pub dashboard_deployment_generation: Option<&'a str>,
}

pub struct BeginRouteBoundHandoffAcquisitionInput<'a> {
    pub inline_route_pool_entry: Option<&'a RoutePoolEntry>,
    pub acquisition_plan: &'a RemoteViewAcquisitionPlan,
    pub browser_id: &'a str,
    pub session_id: &'a str,
    pub observed_at: &'a str,
    pub default_control_input: Option<ControlInputProvider>,
}

pub struct RouteBoundHandoffFailureRollbackInput<'a> {
    pub lease: &'a RemoteViewAcquisitionLease,
    pub phase: &'a str,
    pub error: &'a str,
    pub cleanup: &'a Value,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffFailureCleanupInput<'a> {
    pub lease_id: &'a str,
    pub rollback: &'a Value,
    pub cleanup: &'a Value,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffFailureRecoveryInput<'a> {
    pub lease: &'a RemoteViewAcquisitionLease,
    pub phase: &'a str,
    pub error: &'a str,
    pub rollback_cleanup: &'a Value,
    pub launch: &'a Value,
    pub tab: Option<&'a Value>,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffFailureRecovery {
    pub rollback: Value,
    pub cleanup_plan: RouteBoundHandoffFailureCleanupPlan,
    pub cleanup_task: RouteBoundHandoffFailureCleanupTask,
    pub skipped_cleanup: Option<Value>,
}

pub struct RouteBoundHandoffImmediateFailureInput<'a> {
    pub lease: &'a RemoteViewAcquisitionLease,
    pub phase: &'a str,
    pub error: &'a str,
    pub cleanup: &'a Value,
    pub observed_at: &'a str,
}

pub struct RouteBoundHandoffImmediateFailure {
    pub rollback: Value,
    pub summary: String,
}

pub struct RouteBoundHandoffFailureCleanupSummary {
    pub rollback: Value,
    pub summary: String,
}

pub fn begin_route_bound_handoff_plan_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    inline_route_pool_entry: Option<&RoutePoolEntry>,
    acquisition_plan: &RemoteViewAcquisitionPlan,
    browser_id: &str,
    session_id: &str,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    begin_route_bound_handoff_acquisition(
        repository,
        BeginRouteBoundHandoffAcquisitionInput {
            inline_route_pool_entry,
            acquisition_plan,
            browser_id,
            session_id,
            observed_at,
            default_control_input: route_bound_handoff_default_control_input_provider(
                acquisition_plan.requested_view_stream_provider,
            ),
        },
    )
}

pub fn complete_route_bound_handoff_plan_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease: &RemoteViewAcquisitionLease,
    checkout: &Value,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    restore_route_bound_handoff_lease_if_missing(repository, lease)?;
    complete_route_bound_handoff_acquisition(repository, &lease.id, checkout, observed_at)
}

fn route_bound_handoff_default_control_input_provider(
    provider: crate::native::service_model::ViewStreamProvider,
) -> Option<ControlInputProvider> {
    let input = match provider {
        crate::native::service_model::ViewStreamProvider::CdpScreencast => {
            ControlInputProvider::CdpInput
        }
        crate::native::service_model::ViewStreamProvider::ChromeTabWebrtc
        | crate::native::service_model::ViewStreamProvider::VirtualDisplayWebrtc => {
            ControlInputProvider::WebrtcInput
        }
        crate::native::service_model::ViewStreamProvider::Novnc => ControlInputProvider::VncInput,
        crate::native::service_model::ViewStreamProvider::RdpGateway
        | crate::native::service_model::ViewStreamProvider::ExternalUrl => {
            ControlInputProvider::ManualAttachedDesktop
        }
    };
    Some(input)
}

pub fn begin_route_bound_handoff_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: BeginRouteBoundHandoffAcquisitionInput<'_>,
) -> Result<RemoteViewAcquisitionLease, String> {
    let lease_id = route_bound_handoff_lease_id(
        &input.acquisition_plan.selected_route_id,
        input.session_id,
        input.observed_at,
    );
    let mut lifecycle = RouteBoundLeaseLifecycle::new();
    lifecycle
        .transition_to(RouteBoundLeaseState::Planned)
        .map_err(|error| error.to_string())?;
    lifecycle
        .transition_to(RouteBoundLeaseState::Reserved)
        .map_err(|error| error.to_string())?;
    let (lease_state, lease_phase) = lifecycle.state_phase();
    let boot_epoch = crate::process_identity::current_boot_epoch();
    repository.mutate(|state| {
        if let Some(quarantined) = state.remote_view_acquisition_leases.values().find(|lease| {
            lease.state == "failed"
                && lease.phase == "rollback_incomplete"
                && (lease.browser_id == input.browser_id
                    || lease.session_id == input.session_id
                    || lease.route_id == input.acquisition_plan.selected_route_id
                    || lease.display_allocation_id == input.acquisition_plan.display_allocation_id)
        }) {
            return Err(format!(
                "route_bound_acquisition_quarantined: lease={} browser={} route={} display={}",
                quarantined.id,
                quarantined.browser_id,
                quarantined.route_id,
                quarantined.display_allocation_id
            ));
        }
        if let Some(entry) = input.inline_route_pool_entry {
            state.route_pool.insert(entry.id.clone(), entry.clone());
        }
        let previous_route_pool_entry = input
            .acquisition_plan
            .selected_route_pool_entry_id
            .as_ref()
            .and_then(|id| state.route_pool.get(id).cloned());
        let previous_display_allocation = state
            .display_allocations
            .get(&input.acquisition_plan.display_allocation_id)
            .cloned();
        let previous_remote_view_route = state
            .remote_view_routes
            .get(&input.acquisition_plan.selected_route_id)
            .cloned();
        let previous_browser_display_allocation_id = state
            .browsers
            .get(input.browser_id)
            .and_then(|browser| browser.display_allocation_id.clone());

        if let Some(route_pool_entry_id) =
            input.acquisition_plan.selected_route_pool_entry_id.as_ref()
        {
            if let Some(entry) = state.route_pool.get_mut(route_pool_entry_id) {
                entry.state = "pending".to_string();
                entry.current_route_allocation_id =
                    Some(input.acquisition_plan.selected_route_id.clone());
                entry.readiness = Some(json!({
                    "state": "pending",
                    "component": "remote_view_open_acquisition",
                    "leaseId": lease_id,
                    "updatedAt": input.observed_at,
                }));
            }
        }

        let display_allocation = state
            .display_allocations
            .entry(input.acquisition_plan.display_allocation_id.clone())
            .or_insert_with(|| DisplayAllocation {
                id: input.acquisition_plan.display_allocation_id.clone(),
                boot_epoch: boot_epoch.clone(),
                display_name: input.acquisition_plan.display_name.clone(),
                display_isolation: input
                    .acquisition_plan
                    .route_binding
                    .display_isolation
                    .clone(),
                owner_browser_id: Some(input.browser_id.to_string()),
                owner_session_id: Some(input.session_id.to_string()),
                state: "pending".to_string(),
                created_at: Some(input.observed_at.to_string()),
                updated_at: Some(input.observed_at.to_string()),
                ..DisplayAllocation::default()
            });
        display_allocation.display_name = input.acquisition_plan.display_name.clone();
        display_allocation.boot_epoch = boot_epoch.clone();
        display_allocation.display_isolation = input
            .acquisition_plan
            .route_binding
            .display_isolation
            .clone();
        display_allocation.owner_browser_id = Some(input.browser_id.to_string());
        display_allocation.owner_session_id = Some(input.session_id.to_string());
        display_allocation.state = "pending".to_string();
        display_allocation.updated_at = Some(input.observed_at.to_string());
        display_allocation.readiness = Some(json!({
            "state": "pending",
            "component": "remote_view_open_acquisition",
            "leaseId": lease_id,
            "updatedAt": input.observed_at,
        }));
        if !display_allocation
            .route_ids
            .contains(&input.acquisition_plan.selected_route_id)
        {
            display_allocation
                .route_ids
                .push(input.acquisition_plan.selected_route_id.clone());
        }

        state.remote_view_routes.insert(
            input.acquisition_plan.selected_route_id.clone(),
            RemoteViewRoute {
                id: input.acquisition_plan.selected_route_id.clone(),
                provider: input.acquisition_plan.requested_view_stream_provider,
                display_allocation_id: Some(input.acquisition_plan.display_allocation_id.clone()),
                browser_id: Some(input.browser_id.to_string()),
                session_id: Some(input.session_id.to_string()),
                route_source: if input
                    .acquisition_plan
                    .selected_route_pool_entry_id
                    .is_some()
                {
                    "pool".to_string()
                } else {
                    "retained_state".to_string()
                },
                connection_id: input.acquisition_plan.route_binding.connection_id.clone(),
                connection_name: input.acquisition_plan.route_binding.connection_name.clone(),
                frame_url: input.acquisition_plan.route_binding.frame_url.clone(),
                external_url: input.acquisition_plan.route_binding.external_url.clone(),
                route_descriptor: input
                    .acquisition_plan
                    .route_binding
                    .route_descriptor
                    .clone(),
                control_input: input.default_control_input,
                provider_mode: input.acquisition_plan.route_binding.provider_mode.clone(),
                state: "pending".to_string(),
                last_provider_event: Some("remote_view_open_acquisition_pending".to_string()),
                readiness: Some(json!({
                    "state": "pending",
                    "component": "remote_view_open_acquisition",
                    "leaseId": lease_id,
                    "updatedAt": input.observed_at,
                })),
                ..previous_remote_view_route.clone().unwrap_or_default()
            },
        );

        let lease = RemoteViewAcquisitionLease {
            id: lease_id.clone(),
            boot_epoch: boot_epoch.clone(),
            browser_id: input.browser_id.to_string(),
            session_id: input.session_id.to_string(),
            route_id: input.acquisition_plan.selected_route_id.clone(),
            display_allocation_id: input.acquisition_plan.display_allocation_id.clone(),
            route_pool_entry_id: input.acquisition_plan.selected_route_pool_entry_id.clone(),
            state: lease_state.to_string(),
            phase: lease_phase.to_string(),
            created_at: Some(input.observed_at.to_string()),
            updated_at: Some(input.observed_at.to_string()),
            previous_route_pool_entry,
            previous_display_allocation,
            previous_remote_view_route,
            previous_browser_display_allocation_id,
            ..RemoteViewAcquisitionLease::default()
        };
        state
            .remote_view_acquisition_leases
            .insert(lease_id.clone(), lease.clone());
        Ok(lease)
    })
}

pub fn complete_route_bound_handoff_open(
    input: CompleteRouteBoundHandoffOpenInput<'_>,
) -> Result<Value, String> {
    let final_route_binding =
        final_route_bound_handoff_route_binding(input.planned_route_binding, input.checkout);
    let browser_build_proof =
        route_bound_handoff_browser_build_proof(input.intent, input.launch_command, input.launch);
    let handoff_url = input
        .handoff_id
        .and_then(|handoff_id| durable_remote_view_handoff_url(&final_route_binding, handoff_id));
    let acquisition_lease = finalize_route_bound_handoff_atomic(
        input.repository,
        input.lease,
        input.checkout,
        input.observed_at,
        input
            .handoff_id
            .map(|handoff_id| PersistRemoteViewHandoffInput {
                handoff_id,
                handoff_url: handoff_url.as_deref(),
                intent: input.intent,
                route_binding: &final_route_binding,
                browser_id: input.browser_id,
                session_name: input.session_name,
                tab: input.tab,
                observed_at: input.observed_at,
                dashboard_deployment_generation: input.dashboard_deployment_generation,
            }),
    )?;
    let acquisition_lease = serde_json::to_value(acquisition_lease)
        .map_err(|err| format!("route_bound_handoff_lease_serialize_failed: {err}"))?;

    Ok(opened_route_bound_handoff_response(
        RouteBoundHandoffOpenedResponseInput {
            handoff_id: input.handoff_id,
            handoff_url: handoff_url.as_deref(),
            intent: input.intent,
            planned_route_binding: input.planned_route_binding,
            final_route_binding: &final_route_binding,
            acquisition_plan: input.acquisition_plan,
            browser_id: input.browser_id,
            session_name: input.session_name,
            managed_one_time_profile: input.managed_one_time_profile,
            one_time_profile_warning: input.one_time_profile_warning,
            final_operator_visible: input.final_operator_visible,
            pre_checkout_operator_visible: input.pre_checkout_operator_visible,
            browser_build_proof: &browser_build_proof,
            launch: input.launch,
            tab: input.tab,
            focus: input.focus,
            checkout: input.checkout,
            acquisition_lease: &acquisition_lease,
            display_access_grant: input.display_access_grant,
            reused_current_browser: input.reused_current_browser,
            visible_window_proof: input.visible_window_proof,
        },
    ))
}

struct PersistRemoteViewHandoffInput<'a> {
    handoff_id: &'a str,
    handoff_url: Option<&'a str>,
    intent: &'a RemoteViewOpenIntent,
    route_binding: &'a RemoteViewRouteBinding,
    browser_id: &'a str,
    session_name: &'a str,
    tab: &'a Value,
    observed_at: &'a str,
    dashboard_deployment_generation: Option<&'a str>,
}

fn finalize_route_bound_handoff_atomic(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease: &RemoteViewAcquisitionLease,
    checkout: &Value,
    observed_at: &str,
    handoff: Option<PersistRemoteViewHandoffInput<'_>>,
) -> Result<RemoteViewAcquisitionLease, String> {
    let handoff = handoff
        .map(remote_view_handoff_for_persistence)
        .transpose()?;
    repository.mutate(|state| {
        state
            .remote_view_acquisition_leases
            .entry(lease.id.clone())
            .or_insert_with(|| lease.clone());
        let acquisition_lease =
            finalize_route_bound_acquisition(state, &lease.id, checkout, observed_at)?;
        if let Some((handoff_id, mut handoff)) = handoff.clone() {
            let existing = state.remote_view_handoffs.get(&handoff_id);
            handoff.created_at = existing
                .and_then(|existing| existing.created_at.clone())
                .or(handoff.created_at);
            if let Some(existing) = existing {
                handoff.intent = existing.intent.clone();
                handoff.desired_url = existing
                    .desired_url
                    .clone()
                    .or_else(|| {
                        existing
                            .intent
                            .get("url")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .or_else(|| handoff.desired_url.clone());
            }
            if let Some(receipt) = handoff.presentation_receipt.as_mut() {
                receipt.generation = existing
                    .and_then(|existing| existing.presentation_receipt.as_ref())
                    .map(|receipt| receipt.generation.saturating_add(1))
                    .unwrap_or(1);
                receipt.daemon_owner_generation = Some(receipt.generation.max(1));
                receipt.process_instance_digest = state
                    .browser_process_identities
                    .get(&receipt.logical_browser_id)
                    .and_then(|identity| serde_json::to_vec(&identity.process_identity).ok())
                    .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
                handoff.last_resolution = Some(json!({
                    "status": "ready",
                    "browserId": handoff.browser_id,
                    "sessionName": handoff.session_name,
                    "tabId": handoff.tab_id,
                    "targetId": handoff.target_id,
                    "routeId": handoff.last_route_id,
                    "presentationGeneration": receipt.generation,
                    "presentationReceipt": receipt,
                }));
            }
            state.remote_view_handoffs.insert(handoff_id, handoff);
        }
        Ok(acquisition_lease)
    })
}

fn remote_view_handoff_for_persistence(
    input: PersistRemoteViewHandoffInput<'_>,
) -> Result<(String, RemoteViewHandoff), String> {
    let intent = serde_json::to_value(input.intent)
        .map_err(|error| format!("remote_view_handoff_intent_serialize_failed: {error}"))?;
    let tab_id = input
        .tab
        .get("tabId")
        .or_else(|| input.tab.get("id"))
        .or_else(|| {
            input
                .tab
                .get("serviceTabHandle")
                .and_then(|handle| handle.get("tabId"))
        })
        .and_then(Value::as_str)
        .map(str::to_string);
    let target_id = input
        .tab
        .get("targetId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let profile_id = input
        .tab
        .get("profileId")
        .or_else(|| input.tab.get("runtimeProfile"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| input.intent.runtime_profile.clone())
        .or_else(|| input.intent.profile.clone());
    let control_input =
        route_bound_handoff_default_control_input_provider(input.intent.view_stream_provider);
    let presentation_receipt = input
        .dashboard_deployment_generation
        .zip(target_id.as_deref())
        .map(
            |(dashboard_deployment_generation, target_id)| DurableHandoffPresentationReceipt {
                schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                generation: 0,
                dashboard_deployment_generation: dashboard_deployment_generation.to_string(),
                logical_browser_id: input.browser_id.to_string(),
                daemon_owner_generation: None,
                process_instance_digest: None,
                target_id: target_id.to_string(),
                required_stream_provider: input.intent.view_stream_provider,
                observed_stream_provider: input.route_binding.provider,
                route_id: input.route_binding.route_id.clone(),
                display_allocation_id: input.route_binding.display_allocation_id.clone(),
                observed_at: input.observed_at.to_string(),
                state: "ready".to_string(),
            },
        );

    Ok((
        input.handoff_id.to_string(),
        RemoteViewHandoff {
            id: input.handoff_id.to_string(),
            state: "ready".to_string(),
            intent,
            handoff_url: input.handoff_url.map(str::to_string),
            desired_url: input.intent.url.clone(),
            profile_id,
            browser_id: Some(input.browser_id.to_string()),
            session_name: Some(input.session_name.to_string()),
            tab_id,
            target_id,
            view_stream_provider: Some(input.intent.view_stream_provider),
            control_input,
            last_route_id: Some(input.route_binding.route_id.clone()),
            last_route_pool_entry_id: input.route_binding.route_pool_entry_id.clone(),
            last_display_allocation_id: Some(input.route_binding.display_allocation_id.clone()),
            created_at: Some(input.observed_at.to_string()),
            updated_at: Some(input.observed_at.to_string()),
            last_resolved_at: Some(input.observed_at.to_string()),
            last_resolution: Some(json!({
                "status": "ready",
                "browserId": input.browser_id,
                "sessionName": input.session_name,
                "tabId": input.tab.get("tabId").or_else(|| input.tab.get("id")),
                "targetId": input.tab.get("targetId"),
                "routeId": input.route_binding.route_id,
            })),
            presentation_receipt,
        },
    ))
}

pub fn complete_route_bound_handoff_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease_id: &str,
    checkout: &Value,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    repository
        .mutate(|state| finalize_route_bound_acquisition(state, lease_id, checkout, observed_at))
}

pub fn restore_route_bound_handoff_lease_if_missing(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease: &RemoteViewAcquisitionLease,
) -> Result<(), String> {
    repository.mutate(|state| {
        state
            .remote_view_acquisition_leases
            .entry(lease.id.clone())
            .or_insert_with(|| lease.clone());
        Ok(())
    })
}

pub fn rollback_route_bound_handoff_acquisition(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease_id: &str,
    phase: &str,
    error: &str,
    cleanup: &Value,
    observed_at: &str,
) -> Result<Value, String> {
    repository.mutate(|state| {
        let lease_snapshot = state
            .remote_view_acquisition_leases
            .get(lease_id)
            .cloned()
            .ok_or_else(|| format!("remote_view_acquisition_lease_missing: {lease_id}"))?;

        match lease_snapshot.previous_route_pool_entry.clone() {
            Some(entry) => {
                state.route_pool.insert(entry.id.clone(), entry);
            }
            None => {
                if let Some(id) = lease_snapshot.route_pool_entry_id.as_ref() {
                    state.route_pool.remove(id);
                }
            }
        }
        match lease_snapshot.previous_display_allocation.clone() {
            Some(allocation) => {
                state
                    .display_allocations
                    .insert(allocation.id.clone(), allocation);
            }
            None => {
                state
                    .display_allocations
                    .remove(&lease_snapshot.display_allocation_id);
            }
        }
        match lease_snapshot.previous_remote_view_route.clone() {
            Some(route) => {
                state.remote_view_routes.insert(route.id.clone(), route);
            }
            None => {
                state.remote_view_routes.remove(&lease_snapshot.route_id);
            }
        }
        if let Some(browser) = state.browsers.get_mut(&lease_snapshot.browser_id) {
            browser.display_allocation_id =
                lease_snapshot.previous_browser_display_allocation_id.clone();
        }

        let quarantine = json!({
            "state": "active",
            "reason": "rollback_incomplete",
            "leaseId": lease_id,
            "browserId": lease_snapshot.browser_id,
            "sessionId": lease_snapshot.session_id,
            "routeId": lease_snapshot.route_id,
            "displayAllocationId": lease_snapshot.display_allocation_id,
            "routePoolEntryId": lease_snapshot.route_pool_entry_id,
            "unconfirmedExternalEffects": cleanup,
            "updatedAt": observed_at,
        });
        let rollback = json!({
            "state": "rollback_incomplete",
            "leaseId": lease_id,
            "phase": phase,
            "routeId": lease_snapshot.route_id,
            "displayAllocationId": lease_snapshot.display_allocation_id,
            "routePoolEntryId": lease_snapshot.route_pool_entry_id,
            "restoredRoutePoolEntry": lease_snapshot.previous_route_pool_entry.is_some(),
            "restoredDisplayAllocation": lease_snapshot.previous_display_allocation.is_some(),
            "restoredRemoteViewRoute": lease_snapshot.previous_remote_view_route.is_some(),
            "restoredBrowserDisplayAllocation": lease_snapshot.previous_browser_display_allocation_id,
            "cleanup": cleanup,
            "quarantine": quarantine,
            "updatedAt": observed_at,
        });
        if let Some(route_pool_entry_id) = lease_snapshot.route_pool_entry_id.as_ref() {
            if let Some(entry) = state.route_pool.get_mut(route_pool_entry_id) {
                entry.state = "quarantined".to_string();
                entry.current_route_allocation_id = None;
                entry.readiness = Some(json!({
                    "state": "blocked",
                    "reason": "rollback_incomplete",
                    "leaseId": lease_id,
                    "updatedAt": observed_at,
                }));
            }
        }
        if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
            let mut lifecycle =
                RouteBoundLeaseLifecycle::from_state_phase(&lease.state, &lease.phase)
                    .unwrap_or_default();
            lifecycle
                .transition_to(RouteBoundLeaseState::RollbackIncomplete)
                .map_err(|error| error.to_string())?;
            let (lease_state, lease_phase) = lifecycle.state_phase();
            lease.state = lease_state.to_string();
            lease.phase = lease_phase.to_string();
            lease.updated_at = Some(observed_at.to_string());
            lease.failed_at = Some(observed_at.to_string());
            lease.failure_reason = Some(format!(
                "rollback_incomplete: cleanup not yet confirmed after {phase}: {error}"
            ));
            lease.cleanup = Some(rollback.clone());
        }
        Ok(rollback)
    })
}

pub fn update_route_bound_handoff_acquisition_cleanup(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    lease_id: &str,
    rollback: &Value,
    cleanup: &Value,
    observed_at: &str,
) -> Result<Value, String> {
    let mut updated_rollback = rollback.clone();
    let cleanup_state = cleanup
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let rollback_incomplete = cleanup_state.starts_with("failed_")
        || cleanup
            .get("error")
            .and_then(Value::as_str)
            .is_some_and(|error| error.contains("rollback_incomplete"));
    if let Some(object) = updated_rollback.as_object_mut() {
        object.insert("cleanup".to_string(), cleanup.clone());
        object.insert(
            "state".to_string(),
            Value::String(
                if rollback_incomplete {
                    "rollback_incomplete"
                } else {
                    "rolled_back"
                }
                .to_string(),
            ),
        );
        object.insert(
            "updatedAt".to_string(),
            Value::String(observed_at.to_string()),
        );
    }
    repository.mutate(|state| {
        let lease_snapshot = state
            .remote_view_acquisition_leases
            .get(lease_id)
            .cloned()
            .ok_or_else(|| format!("remote_view_acquisition_lease_missing: {lease_id}"))?;
        if rollback_incomplete {
            let quarantine = json!({
                "state": "active",
                "reason": "rollback_incomplete",
                "leaseId": lease_id,
                "browserId": lease_snapshot.browser_id,
                "sessionId": lease_snapshot.session_id,
                "routeId": lease_snapshot.route_id,
                "displayAllocationId": lease_snapshot.display_allocation_id,
                "routePoolEntryId": lease_snapshot.route_pool_entry_id,
                "unconfirmedExternalEffects": cleanup,
                "updatedAt": observed_at,
            });
            if let Some(object) = updated_rollback.as_object_mut() {
                object.insert("quarantine".to_string(), quarantine.clone());
            }
            if let Some(route_pool_entry_id) = lease_snapshot.route_pool_entry_id.as_ref() {
                if let Some(entry) = state.route_pool.get_mut(route_pool_entry_id) {
                    entry.state = "quarantined".to_string();
                    entry.current_route_allocation_id = None;
                    entry.readiness = Some(json!({
                        "state": "blocked",
                        "reason": "rollback_incomplete",
                        "leaseId": lease_id,
                        "updatedAt": observed_at,
                    }));
                }
            }
            if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
                let mut lifecycle =
                    RouteBoundLeaseLifecycle::from_state_phase(&lease.state, &lease.phase)
                        .unwrap_or_default();
                lifecycle
                    .transition_to(RouteBoundLeaseState::RollbackIncomplete)
                    .map_err(|error| error.to_string())?;
                let (lease_state, lease_phase) = lifecycle.state_phase();
                lease.state = lease_state.to_string();
                lease.phase = lease_phase.to_string();
                lease.failure_reason = Some(format!(
                    "rollback_incomplete: cleanup state {cleanup_state}"
                ));
                lease.updated_at = Some(observed_at.to_string());
                lease.failed_at = Some(observed_at.to_string());
                lease.cleanup = Some(updated_rollback.clone());
            }
        } else {
            if let Some(previous) = lease_snapshot.previous_route_pool_entry.as_ref() {
                state
                    .route_pool
                    .insert(previous.id.clone(), previous.clone());
            } else if let Some(route_pool_entry_id) = lease_snapshot.route_pool_entry_id.as_ref() {
                state.route_pool.remove(route_pool_entry_id);
            }
            if let Some(object) = updated_rollback.as_object_mut() {
                object.remove("quarantine");
            }
            if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
                let mut lifecycle =
                    RouteBoundLeaseLifecycle::from_state_phase(&lease.state, &lease.phase)
                        .unwrap_or_default();
                lifecycle
                    .transition_to(RouteBoundLeaseState::RolledBack)
                    .map_err(|error| error.to_string())?;
                let (lease_state, lease_phase) = lifecycle.state_phase();
                lease.state = lease_state.to_string();
                lease.phase = lease_phase.to_string();
                lease.failure_reason = lease_snapshot.failure_reason.clone();
                lease.updated_at = Some(observed_at.to_string());
                lease.cleanup = Some(updated_rollback.clone());
            }
        }
        Ok(updated_rollback.clone())
    })
}

pub fn rollback_route_bound_handoff_failure(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffFailureRollbackInput<'_>,
) -> Result<Value, String> {
    restore_route_bound_handoff_lease_if_missing(repository, input.lease)?;
    rollback_route_bound_handoff_acquisition(
        repository,
        &input.lease.id,
        input.phase,
        input.error,
        input.cleanup,
        input.observed_at,
    )
}

pub fn begin_route_bound_handoff_failure_recovery(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffFailureRecoveryInput<'_>,
) -> Result<RouteBoundHandoffFailureRecovery, String> {
    let rollback = rollback_route_bound_handoff_failure(
        repository,
        RouteBoundHandoffFailureRollbackInput {
            lease: input.lease,
            phase: input.phase,
            error: input.error,
            cleanup: input.rollback_cleanup,
            observed_at: input.observed_at,
        },
    )?;
    let cleanup_plan = route_bound_handoff_failure_cleanup_plan(input.launch, input.tab);
    let cleanup_task = route_bound_handoff_failure_cleanup_task(&cleanup_plan, input.launch);
    let skipped_cleanup = route_bound_handoff_skipped_failure_cleanup(&cleanup_plan);
    Ok(RouteBoundHandoffFailureRecovery {
        rollback,
        cleanup_plan,
        cleanup_task,
        skipped_cleanup,
    })
}

pub fn route_bound_handoff_immediate_failure(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffImmediateFailureInput<'_>,
) -> Result<RouteBoundHandoffImmediateFailure, String> {
    let rollback = rollback_route_bound_handoff_failure(
        repository,
        RouteBoundHandoffFailureRollbackInput {
            lease: input.lease,
            phase: input.phase,
            error: input.error,
            cleanup: input.cleanup,
            observed_at: input.observed_at,
        },
    )?;
    let rollback = update_route_bound_handoff_acquisition_cleanup(
        repository,
        &input.lease.id,
        &rollback,
        input.cleanup,
        input.observed_at,
    )?;
    let summary = route_bound_handoff_cleanup_summary(input.cleanup, Some(&rollback));
    Ok(RouteBoundHandoffImmediateFailure { rollback, summary })
}

pub fn complete_route_bound_handoff_failure_cleanup(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    input: RouteBoundHandoffFailureCleanupInput<'_>,
) -> Result<RouteBoundHandoffFailureCleanupSummary, String> {
    let rollback = update_route_bound_handoff_acquisition_cleanup(
        repository,
        input.lease_id,
        input.rollback,
        input.cleanup,
        input.observed_at,
    )?;
    let summary = route_bound_handoff_cleanup_summary(input.cleanup, Some(&rollback));
    Ok(RouteBoundHandoffFailureCleanupSummary { rollback, summary })
}
