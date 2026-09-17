//! Service State adapter for provider-free presentation-capacity authority.

use std::collections::BTreeSet;

// Some compatibility exports are consumed only by CLI test modules.
#[allow(unused_imports)]
pub(crate) use agent_browser_service_model::{
    CapacityDecision, CapacityLimitingResource, CapacityNextSafeAction,
    PresentationCapacityAuthority, PresentationCapacityConfig, PresentationCapacityObservations,
    PresentationCapacityProjection, PresentationPriority, PresentationRequest, PresentationSlot,
    PresentationSlotObservation, PresentationSlotState, PressureAdmission, SlotTransitionReceipt,
};

use super::service_model::{BrowserHealth, ServiceState};

/// Builds durable slot candidates only from service-authoritative ready route,
/// display, and pool records. Missing capacity remains missing.
pub(crate) fn from_service_state(
    config: PresentationCapacityConfig,
    state: &ServiceState,
) -> Result<PresentationCapacityAuthority, String> {
    let mut slots = state
        .route_pool
        .values()
        .filter_map(|entry| {
            let route_id = entry.current_route_allocation_id.as_deref()?;
            let route = state.remote_view_routes.get(route_id)?;
            let display_id = route.display_allocation_id.as_deref()?;
            let display = state.display_allocations.get(display_id)?;
            let pool_ready = matches!(entry.state.as_str(), "available" | "checked_out")
                && entry
                    .readiness
                    .as_ref()
                    .and_then(|value| value.get("state"))
                    .and_then(serde_json::Value::as_str)
                    == Some("ready");
            let route_ready = route.state == "ready";
            let display_ready = matches!(display.state.as_str(), "ready" | "active");
            (pool_ready && route_ready && display_ready).then(|| {
                let mut slot = PresentationSlot::warm_idle(format!("slot:{}", entry.id))
                    .with_binding(route.id.clone(), display.id.clone());
                slot.browser_id = route.browser_id.clone();
                if slot.browser_id.is_some() {
                    slot.state = PresentationSlotState::Active;
                }
                slot
            })
        })
        .collect::<Vec<_>>();
    slots.sort_by(|left, right| left.id.cmp(&right.id));
    PresentationCapacityAuthority::new(config, slots)
}

pub(crate) fn observe_presentation_capacity(
    state: &ServiceState,
    capacity: &PresentationCapacityAuthority,
) -> PresentationCapacityObservations {
    PresentationCapacityObservations {
        slots: capacity
            .slots
            .iter()
            .map(|slot| observe_slot(state, slot))
            .collect(),
        binding_warnings: binding_warning_facts(state, capacity),
    }
}

fn observe_slot(state: &ServiceState, slot: &PresentationSlot) -> PresentationSlotObservation {
    let route_id = slot.route_id.as_deref();
    let display_id = slot.display_allocation_id.as_deref();
    let acquisition_lease_active = state.remote_view_acquisition_leases.values().any(|lease| {
        matches!(lease.state.as_str(), "pending" | "active" | "rolling_back")
            && (route_id == Some(lease.route_id.as_str())
                || display_id == Some(lease.display_allocation_id.as_str()))
    });
    let route = route_id.and_then(|id| state.remote_view_routes.get(id));
    let human_controller_active = route.is_some_and(|route| route.controller_lease_id.is_some());
    let non_controller_staging_viewer_active = route.is_some_and(|route| {
        route.viewer_lease_ids.iter().any(|lease_id| {
            state.viewer_leases.get(lease_id).is_some_and(|lease| {
                lease.viewer_role != "controller"
                    && matches!(lease.state.as_str(), "requested" | "active" | "ready")
            })
        })
    });
    let live_handoff_browser_ids = state
        .remote_view_handoffs
        .values()
        .filter(|handoff| {
            route_id == handoff.last_route_id.as_deref()
                && matches!(handoff.state.as_str(), "ready" | "resolving" | "active")
        })
        .filter_map(|handoff| handoff.browser_id.as_deref())
        .filter(|browser_id| {
            state.browsers.get(*browser_id).is_some_and(|browser| {
                browser
                    .view_streams
                    .iter()
                    .any(|stream| stream.route_id.as_deref() == route_id)
            })
        })
        .map(str::to_string)
        .collect::<BTreeSet<_>>();

    PresentationSlotObservation {
        slot_id: slot.id.clone(),
        route_id: slot.route_id.clone(),
        display_allocation_id: slot.display_allocation_id.clone(),
        acquisition_lease_active,
        human_controller_active,
        non_controller_staging_viewer_active,
        live_handoff_browser_ids,
        authoritative_browser_id: authoritative_browser_id(state, slot),
    }
}

fn authoritative_browser_id(state: &ServiceState, slot: &PresentationSlot) -> Option<String> {
    slot.route_id
        .as_deref()
        .zip(slot.display_allocation_id.as_deref())
        .and_then(|(route_id, display_allocation_id)| {
            let route = state.remote_view_routes.get(route_id)?;
            let pending_acquisition =
                super::presentation_inventory::has_current_acquisition_binding(state, route);
            if (route.state == "orphaned" || pending_acquisition)
                && slot.state == PresentationSlotState::Active
                && slot.browser_id.is_some()
                && slot.browser_id == route.browser_id
                && route.display_allocation_id.as_deref() == Some(display_allocation_id)
            {
                let browser_id = route.browser_id.as_deref()?;
                let browser = state.browsers.get(browser_id)?;
                let display = state.display_allocations.get(display_allocation_id)?;
                return (browser.display_allocation_id.as_deref() == Some(display_allocation_id)
                    && display.owner_browser_id == route.browser_id
                    && display.owner_session_id == route.session_id
                    && route
                        .session_id
                        .as_ref()
                        .is_some_and(|session| browser.active_session_ids.contains(session))
                    && display.route_ids.contains(&route.id)
                    && (matches!(display.state.as_str(), "ready" | "active" | "orphaned")
                        || pending_acquisition))
                    .then(|| browser_id.to_string());
            }
            if !matches!(
                route.state.as_str(),
                "ready" | "reconnecting" | "allocating"
            ) || route.display_allocation_id.as_deref() != Some(display_allocation_id)
            {
                return None;
            }
            let browser_id = route.browser_id.as_deref()?;
            let browser = state.browsers.get(browser_id)?;
            (browser.health == BrowserHealth::Ready
                && browser.display_allocation_id.as_deref() == Some(display_allocation_id))
            .then(|| browser_id.to_string())
        })
}

fn binding_warning_facts(
    state: &ServiceState,
    capacity: &PresentationCapacityAuthority,
) -> Vec<String> {
    let mut warnings = BTreeSet::new();
    for slot in &capacity.slots {
        if let Some(route_id) = slot.route_id.as_deref() {
            if !state.remote_view_routes.contains_key(route_id)
                && !state
                    .route_pool
                    .values()
                    .any(|entry| entry.route_id == route_id)
            {
                warnings.insert(format!("slot_route_missing:{}:{}", slot.id, route_id));
            }
        }
        if let Some(display_id) = slot.display_allocation_id.as_deref() {
            if !state.display_allocations.contains_key(display_id) {
                warnings.insert(format!("slot_display_missing:{}:{}", slot.id, display_id));
            }
        }
        if let Some(browser_id) = slot.browser_id.as_deref() {
            if !state.browsers.contains_key(browser_id) {
                warnings.insert(format!("slot_browser_missing:{}:{}", slot.id, browser_id));
            }
        }
    }
    warnings.into_iter().collect()
}

pub(crate) fn projection_with_service_state(
    capacity: &PresentationCapacityAuthority,
    pressure: PressureAdmission,
    service_state: Option<&ServiceState>,
) -> PresentationCapacityProjection {
    match service_state {
        Some(state) => {
            let observations = observe_presentation_capacity(state, capacity);
            capacity.projection_with_observations(pressure, Some(&observations))
        }
        None => capacity.projection(pressure),
    }
}

pub(crate) fn request_with_service_state(
    capacity: &mut PresentationCapacityAuthority,
    request: PresentationRequest,
    pressure: PressureAdmission,
    service_state: Option<&ServiceState>,
) -> CapacityDecision {
    match service_state {
        Some(state) => {
            let observations = observe_presentation_capacity(state, capacity);
            capacity.request_with_observations(request, pressure, Some(&observations))
        }
        None => capacity.request(request, pressure),
    }
}

pub(crate) fn request_bound_observation(
    capacity: &mut PresentationCapacityAuthority,
    request: PresentationRequest,
    pressure: PressureAdmission,
    service_state: &ServiceState,
    route_id: &str,
    display_allocation_id: &str,
) -> CapacityDecision {
    let observations = observe_presentation_capacity(service_state, capacity);
    capacity.request_bound_observation(
        request,
        pressure,
        &observations,
        route_id,
        display_allocation_id,
    )
}

pub(crate) fn request_bound_recovery(
    capacity: &mut PresentationCapacityAuthority,
    request: PresentationRequest,
    pressure: PressureAdmission,
    service_state: &ServiceState,
    route_id: &str,
    display_allocation_id: &str,
) -> CapacityDecision {
    let observations = observe_presentation_capacity(service_state, capacity);
    capacity.request_bound_recovery(
        request,
        pressure,
        &observations,
        route_id,
        display_allocation_id,
    )
}

pub(crate) fn request_bound_route_switch_recovery(
    capacity: &mut PresentationCapacityAuthority,
    request: PresentationRequest,
    pressure: PressureAdmission,
    service_state: &ServiceState,
    route_id: &str,
    display_allocation_id: &str,
) -> CapacityDecision {
    let observations = observe_presentation_capacity(service_state, capacity);
    capacity.request_bound_route_switch_recovery(
        request,
        pressure,
        &observations,
        route_id,
        display_allocation_id,
    )
}

pub(crate) fn release_bound_presentation(
    capacity: &mut PresentationCapacityAuthority,
    slot_id: &str,
    request_id: &str,
    pressure: PressureAdmission,
    service_state: &ServiceState,
) -> Result<Option<CapacityDecision>, String> {
    let observations = observe_presentation_capacity(service_state, capacity);
    capacity.release_bound_presentation(slot_id, request_id, pressure, &observations)
}

pub(crate) fn binding_warnings(
    capacity: &PresentationCapacityAuthority,
    state: &ServiceState,
) -> Vec<String> {
    let observations = observe_presentation_capacity(state, capacity);
    capacity.binding_warnings(&observations)
}

pub(crate) fn reconcile_authoritative_bindings(
    capacity: &mut PresentationCapacityAuthority,
    state: &ServiceState,
) -> usize {
    let observations = observe_presentation_capacity(state, capacity);
    capacity.reconcile_authoritative_bindings(&observations)
}

pub(crate) fn transition_slot(
    capacity: &mut PresentationCapacityAuthority,
    slot_id: &str,
    request_id: &str,
    next: PresentationSlotState,
    service_state: Option<&ServiceState>,
) -> Result<SlotTransitionReceipt, String> {
    match service_state {
        Some(state) => {
            let observations = observe_presentation_capacity(state, capacity);
            capacity.transition_slot(slot_id, request_id, next, Some(&observations))
        }
        None => capacity.transition_slot(slot_id, request_id, next, None),
    }
}

pub(crate) fn release_and_dispatch_with_service_state(
    capacity: &mut PresentationCapacityAuthority,
    slot_id: &str,
    pressure: PressureAdmission,
    service_state: Option<&ServiceState>,
) -> Option<CapacityDecision> {
    match service_state {
        Some(state) => {
            let observations = observe_presentation_capacity(state, capacity);
            capacity.release_and_dispatch_with_observations(slot_id, pressure, Some(&observations))
        }
        None => capacity.release_and_dispatch(slot_id, pressure),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProcess, DisplayAllocation, RemoteViewHandoff, RemoteViewRoute, RoutePoolEntry,
        ViewStream, ViewerLease,
    };
    use std::collections::BTreeMap;

    fn config() -> PresentationCapacityConfig {
        PresentationCapacityConfig {
            warm_minimum: 1,
            hard_maximum: 1,
            human_priority_reserve: 0,
            recovery_reserve: 0,
            max_queue_depth: 8,
        }
    }

    fn one_slot_authority() -> PresentationCapacityAuthority {
        PresentationCapacityAuthority::new(
            config(),
            vec![PresentationSlot::warm_idle("slot-1").with_binding("route-1", "display-1")],
        )
        .unwrap()
    }

    #[test]
    fn reconciliation_activates_authoritatively_bound_warm_slot() {
        let state = ServiceState {
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::Ready,
                    display_allocation_id: Some("display-1".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    display_allocation_id: Some("display-1".to_string()),
                    browser_id: Some("browser-1".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        };
        let mut authority = one_slot_authority();

        assert_eq!(reconcile_authoritative_bindings(&mut authority, &state), 1);
        assert_eq!(authority.slots[0].state, PresentationSlotState::Active);
        assert_eq!(authority.slots[0].browser_id.as_deref(), Some("browser-1"));
    }

    #[test]
    fn stale_handoff_is_not_observed_without_a_live_route_stream() {
        let mut state = ServiceState {
            remote_view_handoffs: BTreeMap::from([(
                "handoff-old".to_string(),
                RemoteViewHandoff {
                    id: "handoff-old".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-old".to_string()),
                    last_route_id: Some("route-1".to_string()),
                    ..RemoteViewHandoff::default()
                },
            )]),
            ..ServiceState::default()
        };
        let authority = one_slot_authority();
        assert!(observe_presentation_capacity(&state, &authority).slots[0]
            .live_handoff_browser_ids
            .is_empty());

        state.browsers.insert(
            "browser-old".to_string(),
            BrowserProcess {
                id: "browser-old".to_string(),
                view_streams: vec![ViewStream {
                    route_id: Some("route-1".to_string()),
                    ..ViewStream::default()
                }],
                ..BrowserProcess::default()
            },
        );
        assert!(observe_presentation_capacity(&state, &authority).slots[0]
            .live_handoff_browser_ids
            .contains("browser-old"));
    }

    #[test]
    fn controller_and_viewer_facts_preserve_request_dependent_policy() {
        let mut state = ServiceState::default();
        state.viewer_leases.insert(
            "viewer-1".to_string(),
            ViewerLease {
                id: "viewer-1".to_string(),
                route_id: Some("route-1".to_string()),
                state: "active".to_string(),
                ..Default::default()
            },
        );
        state.remote_view_routes.insert(
            "route-1".to_string(),
            RemoteViewRoute {
                id: "route-1".to_string(),
                controller_lease_id: Some("controller-1".to_string()),
                viewer_lease_ids: vec!["viewer-1".to_string()],
                ..Default::default()
            },
        );

        let mut automated = one_slot_authority();
        let blocked = request_with_service_state(
            &mut automated,
            PresentationRequest::observation("observe").for_browser("browser-1"),
            PressureAdmission::admit(1),
            Some(&state),
        );
        assert_eq!(
            blocked.limiting_resource(),
            Some(CapacityLimitingResource::HumanController)
        );

        state
            .remote_view_routes
            .get_mut("route-1")
            .unwrap()
            .controller_lease_id = None;
        let mut staging = one_slot_authority();
        let blocked = request_with_service_state(
            &mut staging,
            PresentationRequest::observation("stage")
                .for_browser("browser-1")
                .requiring_staging(),
            PressureAdmission::admit(1),
            Some(&state),
        );
        assert_eq!(
            blocked.limiting_resource(),
            Some(CapacityLimitingResource::ViewerStagingConflict)
        );

        let mut capture = one_slot_authority();
        assert!(request_with_service_state(
            &mut capture,
            PresentationRequest::observation("capture").for_browser("browser-1"),
            PressureAdmission::admit(1),
            Some(&state),
        )
        .is_granted());
    }

    #[test]
    fn service_inventory_derivation_never_manufactures_unready_capacity() {
        let mut state = ServiceState::default();
        for (id, ready) in [("one", true), ("two", false)] {
            let route_id = format!("route-{id}");
            let display_id = format!("display-{id}");
            state.display_allocations.insert(
                display_id.clone(),
                DisplayAllocation {
                    id: display_id.clone(),
                    state: if ready { "ready" } else { "allocating" }.to_string(),
                    ..Default::default()
                },
            );
            state.remote_view_routes.insert(
                route_id.clone(),
                RemoteViewRoute {
                    id: route_id.clone(),
                    display_allocation_id: Some(display_id),
                    state: if ready { "ready" } else { "allocating" }.to_string(),
                    ..Default::default()
                },
            );
            state.route_pool.insert(
                id.to_string(),
                RoutePoolEntry {
                    id: id.to_string(),
                    route_id: format!("provider-{id}"),
                    state: "available".to_string(),
                    current_route_allocation_id: Some(route_id),
                    readiness: Some(serde_json::json!({
                        "state": if ready { "ready" } else { "blocked" }
                    })),
                    ..Default::default()
                },
            );
        }

        let authority = from_service_state(
            PresentationCapacityConfig {
                warm_minimum: 1,
                hard_maximum: 2,
                human_priority_reserve: 0,
                recovery_reserve: 0,
                max_queue_depth: 8,
            },
            &state,
        )
        .unwrap();
        assert_eq!(authority.slots.len(), 1);
        assert_eq!(authority.slots[0].id, "slot:one");
    }

    #[test]
    fn controller_arriving_after_reservation_fences_staging() {
        let mut authority = one_slot_authority();
        authority.request(
            PresentationRequest::observation("episode-1").for_browser("browser-1"),
            PressureAdmission::admit(1),
        );
        let mut state = ServiceState::default();
        state.remote_view_routes.insert(
            "route-1".to_string(),
            RemoteViewRoute {
                id: "route-1".to_string(),
                controller_lease_id: Some("human-controller".to_string()),
                ..Default::default()
            },
        );

        assert_eq!(
            transition_slot(
                &mut authority,
                "slot-1",
                "episode-1",
                PresentationSlotState::Staging,
                Some(&state),
            )
            .unwrap_err(),
            "presentation_slot_staging_blocked:human_controller"
        );
        assert_eq!(authority.slots[0].state, PresentationSlotState::Reserved);
        assert_eq!(authority.slots[0].scene_generation, 0);
    }
}
