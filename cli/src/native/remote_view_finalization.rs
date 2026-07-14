use serde_json::{json, Value};

use super::remote_view_lease::{RouteBoundLeaseLifecycle, RouteBoundLeaseState};
use super::service_model::{
    BrowserProcess, DisplayAllocation, RemoteViewAcquisitionLease, RemoteViewRoute, RoutePoolEntry,
    ServiceState,
};

struct FinalizedRouteFacts {
    route_id: String,
    display_allocation_id: String,
    route_pool_entry_id: Option<String>,
    browser_id: String,
    session_id: String,
}

/// Finalizes a successful route-bound open in one service-state mutation.
///
/// The caller may have already written checkout records through a separate
/// repository instance. This function must still write the same ownership facts
/// because completing an older acquisition snapshot can otherwise persist stale
/// pending records over the checkout.
pub fn finalize_route_bound_acquisition(
    state: &mut ServiceState,
    lease_id: &str,
    checkout: &Value,
    observed_at: &str,
) -> Result<RemoteViewAcquisitionLease, String> {
    let lease_snapshot = state
        .remote_view_acquisition_leases
        .get(lease_id)
        .cloned()
        .ok_or_else(|| format!("remote_view_acquisition_lease_missing: {lease_id}"))?;
    let mut lifecycle =
        RouteBoundLeaseLifecycle::from_state_phase(&lease_snapshot.state, &lease_snapshot.phase)
            .ok_or_else(|| {
                format!(
                    "invalid_route_bound_lease_state: lease '{}' has state='{}' phase='{}'",
                    lease_snapshot.id, lease_snapshot.state, lease_snapshot.phase
                )
            })?;
    lifecycle
        .transition_to(RouteBoundLeaseState::DisplayReady)
        .map_err(|error| error.to_string())?;
    lifecycle
        .transition_to(RouteBoundLeaseState::BrowserAttached)
        .map_err(|error| error.to_string())?;
    lifecycle
        .transition_to(RouteBoundLeaseState::TabAcquired)
        .map_err(|error| error.to_string())?;
    lifecycle
        .transition_to(RouteBoundLeaseState::ProofReady)
        .map_err(|error| error.to_string())?;
    lifecycle
        .transition_to(RouteBoundLeaseState::Finalized)
        .map_err(|error| error.to_string())?;
    let (lease_state, lease_phase) = lifecycle.state_phase();

    let facts = FinalizedRouteFacts {
        route_id: checkout
            .get("remoteViewRouteId")
            .or_else(|| checkout.get("routeId"))
            .and_then(Value::as_str)
            .unwrap_or(lease_snapshot.route_id.as_str())
            .to_string(),
        display_allocation_id: checkout
            .get("displayAllocationId")
            .and_then(Value::as_str)
            .unwrap_or(lease_snapshot.display_allocation_id.as_str())
            .to_string(),
        route_pool_entry_id: checkout
            .get("routePoolEntryId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| lease_snapshot.route_pool_entry_id.clone()),
        browser_id: checkout
            .get("browserId")
            .and_then(Value::as_str)
            .unwrap_or(lease_snapshot.browser_id.as_str())
            .to_string(),
        session_id: checkout
            .get("sessionName")
            .and_then(Value::as_str)
            .unwrap_or(lease_snapshot.session_id.as_str())
            .to_string(),
    };
    let readiness = checkout
        .get("remoteViewRoute")
        .and_then(|route| route.get("readiness"))
        .cloned()
        .or_else(|| checkout.get("readiness").cloned())
        .unwrap_or_else(|| {
            json!({
                "state": "ready",
                "component": "route_bound_finalization",
                "updatedAt": observed_at,
            })
        });

    let route = finalized_route(
        state,
        checkout,
        &facts.route_id,
        &facts.display_allocation_id,
        &facts.browser_id,
        &facts.session_id,
        &readiness,
    );
    state
        .remote_view_routes
        .insert(facts.route_id.clone(), route);

    finalize_display_allocation(state, checkout, &facts, &readiness, observed_at);
    if let Some(entry_id) = facts.route_pool_entry_id.as_ref() {
        finalize_route_pool_entry(
            state,
            checkout,
            entry_id,
            &facts.route_id,
            &readiness,
            observed_at,
        );
    }
    finalize_browser(
        state.browsers.get_mut(&facts.browser_id),
        &facts.display_allocation_id,
        &facts.session_id,
    );

    let lease = state
        .remote_view_acquisition_leases
        .get_mut(lease_id)
        .ok_or_else(|| format!("remote_view_acquisition_lease_missing: {lease_id}"))?;
    lease.route_id = facts.route_id;
    lease.display_allocation_id = facts.display_allocation_id;
    lease.route_pool_entry_id = facts.route_pool_entry_id;
    lease.browser_id = facts.browser_id;
    lease.session_id = facts.session_id;
    lease.state = lease_state.to_string();
    lease.phase = lease_phase.to_string();
    lease.updated_at = Some(observed_at.to_string());
    lease.completed_at = Some(observed_at.to_string());
    Ok(lease.clone())
}

fn finalized_route(
    state: &ServiceState,
    checkout: &Value,
    route_id: &str,
    display_allocation_id: &str,
    browser_id: &str,
    session_id: &str,
    readiness: &Value,
) -> RemoteViewRoute {
    let mut route = checkout
        .get("remoteViewRoute")
        .cloned()
        .and_then(|value| serde_json::from_value::<RemoteViewRoute>(value).ok())
        .or_else(|| state.remote_view_routes.get(route_id).cloned())
        .unwrap_or_else(|| RemoteViewRoute {
            id: route_id.to_string(),
            ..RemoteViewRoute::default()
        });
    route.id = route_id.to_string();
    route.display_allocation_id = Some(display_allocation_id.to_string());
    route.browser_id = Some(browser_id.to_string());
    route.session_id = Some(session_id.to_string());
    route.state = "ready".to_string();
    route.last_provider_event = Some("route_bound_finalized".to_string());
    route.readiness = Some(readiness.clone());
    route
}

fn finalize_display_allocation(
    state: &mut ServiceState,
    checkout: &Value,
    facts: &FinalizedRouteFacts,
    readiness: &Value,
    observed_at: &str,
) {
    let display_name = checkout
        .get("routeBinding")
        .and_then(|binding| {
            binding
                .get("launchDisplayName")
                .or_else(|| binding.get("displayName"))
        })
        .and_then(Value::as_str)
        .map(str::to_string);
    let display_isolation = checkout
        .get("routeBinding")
        .and_then(|binding| binding.get("displayIsolation"))
        .and_then(Value::as_str)
        .unwrap_or("shared_display")
        .to_string();
    let allocation = state
        .display_allocations
        .entry(facts.display_allocation_id.clone())
        .or_insert_with(|| DisplayAllocation {
            id: facts.display_allocation_id.clone(),
            created_at: Some(observed_at.to_string()),
            ..DisplayAllocation::default()
        });
    allocation.display_name = display_name;
    allocation.display_isolation = display_isolation;
    allocation.owner_browser_id = Some(facts.browser_id.clone());
    allocation.owner_session_id = Some(facts.session_id.clone());
    allocation.state = "ready".to_string();
    allocation.updated_at = Some(observed_at.to_string());
    allocation.readiness = Some(readiness.clone());
    if !allocation.route_ids.iter().any(|id| id == &facts.route_id) {
        allocation.route_ids.push(facts.route_id.clone());
    }
}

fn finalize_route_pool_entry(
    state: &mut ServiceState,
    checkout: &Value,
    entry_id: &str,
    route_id: &str,
    readiness: &Value,
    observed_at: &str,
) {
    if let Some(mut entry) = checkout
        .get("routePoolEntry")
        .cloned()
        .and_then(|value| serde_json::from_value::<RoutePoolEntry>(value).ok())
    {
        entry.state = "checked_out".to_string();
        entry.current_route_allocation_id = Some(route_id.to_string());
        entry.readiness = entry.readiness.or_else(|| Some(readiness.clone()));
        state.route_pool.insert(entry_id.to_string(), entry);
        return;
    }
    if let Some(entry) = state.route_pool.get_mut(entry_id) {
        entry.state = "checked_out".to_string();
        entry.current_route_allocation_id = Some(route_id.to_string());
        entry.readiness = Some(json!({
            "state": "ready",
            "component": "route_bound_finalization",
            "routeId": route_id,
            "updatedAt": observed_at,
            "proof": readiness,
        }));
    }
}

fn finalize_browser(
    browser: Option<&mut BrowserProcess>,
    display_allocation_id: &str,
    session_id: &str,
) {
    if let Some(browser) = browser {
        browser.display_allocation_id = Some(display_allocation_id.to_string());
        if !browser
            .active_session_ids
            .iter()
            .any(|active_session_id| active_session_id == session_id)
        {
            browser.active_session_ids.push(session_id.to_string());
            browser.active_session_ids.sort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::native::service_model::{BrowserHealth, BrowserProcess};

    #[test]
    fn finalizes_completed_checkout_records_together() {
        let mut state = ServiceState {
            display_allocations: BTreeMap::from([(
                "remote-view-display:13".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:13".to_string(),
                    state: "pending".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    health: BrowserHealth::Ready,
                    ..BrowserProcess::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "guacamole:3".to_string(),
                RemoteViewRoute {
                    id: "guacamole:3".to_string(),
                    state: "pending".to_string(),
                    display_allocation_id: Some("remote-view-display:13".to_string()),
                    browser_id: Some("session:default".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "guacamole-rdp-a".to_string(),
                RoutePoolEntry {
                    id: "guacamole-rdp-a".to_string(),
                    route_id: "guacamole:3".to_string(),
                    state: "pending".to_string(),
                    current_route_allocation_id: Some("guacamole:3".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_acquisition_leases: BTreeMap::from([(
                "lease".to_string(),
                RemoteViewAcquisitionLease {
                    id: "lease".to_string(),
                    browser_id: "session:default".to_string(),
                    session_id: "default".to_string(),
                    route_id: "guacamole:3".to_string(),
                    display_allocation_id: "remote-view-display:13".to_string(),
                    route_pool_entry_id: Some("guacamole-rdp-a".to_string()),
                    state: "pending".to_string(),
                    phase: "reserved".to_string(),
                    ..RemoteViewAcquisitionLease::default()
                },
            )]),
            ..ServiceState::default()
        };

        let checkout = json!({
            "browserId": "session:default",
            "sessionName": "default",
            "routeId": "guacamole:3",
            "remoteViewRouteId": "guacamole:3",
            "displayAllocationId": "remote-view-display:13",
            "routePoolEntryId": "guacamole-rdp-a",
            "readiness": {
                "state": "ready",
                "component": "remote_view_open_visible_window"
            },
            "routeBinding": {
                "launchDisplayName": ":13",
                "displayIsolation": "shared_display"
            }
        });

        let lease = finalize_route_bound_acquisition(
            &mut state,
            "lease",
            &checkout,
            "2026-06-25T00:00:00Z",
        )
        .unwrap();

        assert_eq!(lease.state, "completed");
        assert_eq!(lease.phase, "checked_out");
        assert_eq!(state.route_pool["guacamole-rdp-a"].state, "checked_out");
        assert_eq!(
            state.display_allocations["remote-view-display:13"].state,
            "ready"
        );
        assert_eq!(state.remote_view_routes["guacamole:3"].state, "ready");
        assert_eq!(
            state.browsers["session:default"]
                .display_allocation_id
                .as_deref(),
            Some("remote-view-display:13")
        );
    }

    #[test]
    fn rejects_invalid_lease_lifecycle_without_partial_finalization() {
        let mut state = ServiceState {
            display_allocations: BTreeMap::from([(
                "display".to_string(),
                DisplayAllocation {
                    id: "display".to_string(),
                    state: "pending".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_acquisition_leases: BTreeMap::from([(
                "lease".to_string(),
                RemoteViewAcquisitionLease {
                    id: "lease".to_string(),
                    route_id: "route".to_string(),
                    display_allocation_id: "display".to_string(),
                    state: "completed".to_string(),
                    phase: "reserved".to_string(),
                    ..RemoteViewAcquisitionLease::default()
                },
            )]),
            ..ServiceState::default()
        };

        let error = finalize_route_bound_acquisition(
            &mut state,
            "lease",
            &json!({}),
            "2026-06-25T00:00:00Z",
        )
        .unwrap_err();

        assert!(error.contains("invalid_route_bound_lease_state"));
        assert_eq!(state.display_allocations["display"].state, "pending");
        assert_eq!(
            state.remote_view_acquisition_leases["lease"].state,
            "completed"
        );
    }
}
