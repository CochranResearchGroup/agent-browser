#![allow(unused_imports)]
use super::planner::remote_view_lease_is_active;
use super::shared::*;
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
