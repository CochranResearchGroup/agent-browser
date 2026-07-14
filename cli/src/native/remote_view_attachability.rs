use serde_json::{json, Value};

use super::service_model::{
    BrowserHealth, BrowserHost, BrowserProcess, DisplayAllocation, RemoteViewRoute, RoutePoolEntry,
    ServiceState, ViewStream, ViewStreamProvider,
};

const ACTIVE_VIEWER_LEASE_STATES: [&str; 2] = ["observing", "controlling"];

pub fn refresh_remote_view_attachability(state: &mut ServiceState) {
    let snapshot = state.clone();
    for browser in state.browsers.values_mut() {
        let browser_snapshot = browser.clone();
        let mut browser_attachability = browser_base_attachability(&browser_snapshot, &snapshot);
        for stream in &mut browser.view_streams {
            if stream.provider != ViewStreamProvider::RdpGateway {
                continue;
            }
            let attachability = derive_stream_attachability(&browser_snapshot, stream, &snapshot);
            if browser_attachability.as_ref().is_none_or(|current| {
                attachability_rank(&attachability) > attachability_rank(current)
            }) {
                browser_attachability = Some(attachability.clone());
            }
            stream.attachability = Some(attachability);
        }
        browser.attachability = browser_attachability;
    }
}

pub fn derive_stream_attachability(
    browser: &BrowserProcess,
    stream: &ViewStream,
    state: &ServiceState,
) -> Value {
    let route_id = stream.route_id.as_deref();
    let route = route_id.and_then(|id| state.remote_view_routes.get(id));
    let display_allocation_id = stream
        .display_allocation_id
        .as_deref()
        .or(browser.display_allocation_id.as_deref())
        .map(str::to_string);
    let display_allocation = display_allocation_id
        .as_deref()
        .and_then(|id| state.display_allocations.get(id));
    let route_pool_entry = route_pool_entry_for_stream(stream, route, state);
    let display_content_state = display_content_state(stream, route, display_allocation);
    let proof_state = normalized_proof_state(stream, route, display_allocation);
    let route_state = route.map(|route| route.state.as_str()).unwrap_or("missing");
    let route_pool_state = route_pool_entry
        .map(|entry| entry.state.as_str())
        .unwrap_or("missing");
    let route_display_allocation_id =
        route.and_then(|route| route.display_allocation_id.as_deref());
    let display_agrees = match (
        display_allocation_id.as_deref(),
        route_display_allocation_id,
    ) {
        (Some(stream_display), Some(route_display)) => stream_display == route_display,
        (Some(_), None) => true,
        _ => false,
    };
    let browser_agrees = route
        .and_then(|route| route.browser_id.as_deref())
        .map(|route_browser| route_browser == browser.id)
        .unwrap_or(true);
    let has_active_viewer = stream
        .viewer_lease_ids
        .iter()
        .chain(stream.controller_lease_id.iter())
        .any(|id| {
            state
                .viewer_leases
                .get(id)
                .is_some_and(|lease| ACTIVE_VIEWER_LEASE_STATES.contains(&lease.state.as_str()))
        });
    let (attachability_state, recommended_action, reason) =
        if is_terminal_browser_health(browser.health) {
            (
                "not_reattachable_closed",
                "open_new_remote_view_browser",
                "browser health is terminal",
            )
        } else if route_id.is_none() {
            (
                "reattachable_no_route",
                "service_remote_view_browser_reattach",
                "browser is live but no remote-view route is selected",
            )
        } else if !display_agrees || !browser_agrees {
            (
                "reattachable_stale_route",
                "service_remote_view_browser_reattach",
                "route ownership does not match browser display ownership",
            )
        } else if matches!(
            route_state,
            "orphaned" | "pending" | "released" | "failed" | "missing"
        ) || matches!(route_pool_state, "pending")
        {
            (
                "reattachable_stale_route",
                "service_remote_view_browser_reattach",
                "route or route-pool state is not attached-ready",
            )
        } else if route_state == "ready" && proof_state == "ready" {
            if has_active_viewer || stream.viewer_lease_ids.is_empty() {
                (
                    "attached_ready",
                    "open_existing_remote_view_route",
                    "route, browser, display, and proof agree",
                )
            } else {
                (
                    "reattachable_viewer_disconnected",
                    "service_viewer_lease_request",
                    "route is ready but retained viewer leases are disconnected",
                )
            }
        } else {
            (
                "reattachable_stale_route",
                "service_remote_view_browser_reattach",
                "operator-visible proof is not ready",
            )
        };

    json!({
        "state": attachability_state,
        "recommendedAction": recommended_action,
        "reason": reason,
        "browserId": browser.id.clone(),
        "profileId": browser.profile_id.clone(),
        "sessionName": browser.active_session_ids.first().cloned(),
        "browserHealth": browser.health,
        "displayAllocationId": display_allocation_id,
        "displayName": display_allocation.and_then(|allocation| allocation.display_name.clone()).or_else(|| browser.display_name.clone()),
        "routeId": route_id,
        "routePoolEntryId": route_pool_entry.map(|entry| entry.id.clone()),
        "connectionId": stream.connection_id.clone(),
        "frameUrl": stream.frame_url.clone(),
        "externalUrl": stream.external_url.clone(),
        "routeState": route_state,
        "routePoolState": route_pool_state,
        "proofState": proof_state,
        "displayContentState": display_content_state,
        "displayAgrees": display_agrees,
        "browserAgrees": browser_agrees,
        "hasActiveViewer": has_active_viewer,
    })
}

fn browser_base_attachability(browser: &BrowserProcess, state: &ServiceState) -> Option<Value> {
    if browser.host != BrowserHost::RemoteHeaded
        && !browser
            .view_streams
            .iter()
            .any(|stream| stream.provider == ViewStreamProvider::RdpGateway)
    {
        return None;
    }
    let state_value = if is_terminal_browser_health(browser.health) {
        "not_reattachable_closed"
    } else if route_pool_capacity_occupied(browser, state) {
        "reattachable_route_occupied"
    } else {
        "reattachable_no_route"
    };
    Some(json!({
        "state": state_value,
        "recommendedAction": match state_value {
            "reattachable_no_route" => "service_remote_view_browser_reattach",
            "reattachable_route_occupied" => "service_remote_view_route_switch",
            _ => "repair_browser_before_reattach",
        },
        "reason": match state_value {
            "reattachable_no_route" => "remote-headed browser is live without an attached route",
            "reattachable_route_occupied" => "remote-headed browser is live but all RDP gateway routes are occupied",
            _ => "browser is closed or terminal",
        },
        "browserId": browser.id.clone(),
        "profileId": browser.profile_id.clone(),
        "sessionName": browser.active_session_ids.first().cloned(),
        "browserHealth": browser.health,
        "displayAllocationId": browser.display_allocation_id.clone(),
        "routeId": Value::Null,
        "routeState": "missing",
        "routePoolState": "missing",
        "proofState": "not_checked",
        "displayContentState": "not_checked",
        "availableRoutePoolEntries": state
            .route_pool
            .values()
            .filter(|entry| entry.provider == ViewStreamProvider::RdpGateway && entry.state == "available")
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>(),
    }))
}

fn route_pool_capacity_occupied(browser: &BrowserProcess, state: &ServiceState) -> bool {
    let mut has_rdp_route_pool = false;
    for entry in state
        .route_pool
        .values()
        .filter(|entry| entry.provider == ViewStreamProvider::RdpGateway)
    {
        has_rdp_route_pool = true;
        if entry.state == "available" {
            return false;
        }
        if entry
            .current_route_allocation_id
            .as_deref()
            .is_some_and(|route_id| {
                state
                    .remote_view_routes
                    .get(route_id)
                    .and_then(|route| route.browser_id.as_deref())
                    == Some(browser.id.as_str())
            })
        {
            return false;
        }
    }
    has_rdp_route_pool
}

fn is_terminal_browser_health(health: BrowserHealth) -> bool {
    matches!(
        health,
        BrowserHealth::NotStarted
            | BrowserHealth::ProcessExited
            | BrowserHealth::Closing
            | BrowserHealth::Faulted
    )
}

fn attachability_rank(value: &Value) -> u8 {
    match value.get("state").and_then(Value::as_str).unwrap_or("") {
        "attached_ready" => 100,
        "reattachable_viewer_disconnected" => 80,
        "reattachable_stale_route" => 70,
        "reattachable_no_route" => 60,
        "reattachable_route_occupied" => 50,
        "not_reattachable_faulted" => 20,
        "not_reattachable_closed" => 10,
        _ => 0,
    }
}

fn route_pool_entry_for_stream<'a>(
    stream: &ViewStream,
    route: Option<&RemoteViewRoute>,
    state: &'a ServiceState,
) -> Option<&'a RoutePoolEntry> {
    state.route_pool.values().find(|entry| {
        stream.route_id.as_deref().is_some_and(|route_id| {
            entry.current_route_allocation_id.as_deref() == Some(route_id)
                || entry.route_id == route_id
        }) || route
            .and_then(|route| route.connection_id.as_deref())
            .is_some_and(|connection_id| entry.connection_id.as_deref() == Some(connection_id))
    })
}

fn display_content_state(
    stream: &ViewStream,
    route: Option<&RemoteViewRoute>,
    display_allocation: Option<&DisplayAllocation>,
) -> String {
    [
        stream
            .remote_readiness
            .as_ref()
            .and_then(|value| value.get("displayContent")),
        stream
            .readiness
            .as_ref()
            .and_then(|value| value.get("displayContent")),
        route
            .and_then(|route| route.readiness.as_ref())
            .and_then(|value| value.get("displayContent")),
        display_allocation
            .and_then(|allocation| allocation.readiness.as_ref())
            .and_then(|value| value.get("displayContent")),
    ]
    .into_iter()
    .flatten()
    .find_map(|value| value.get("state").and_then(Value::as_str))
    .unwrap_or("not_checked")
    .to_string()
}

fn normalized_proof_state(
    stream: &ViewStream,
    route: Option<&RemoteViewRoute>,
    display_allocation: Option<&DisplayAllocation>,
) -> String {
    let display_state = display_content_state(stream, route, display_allocation);
    if display_state == "browser_window_visible" {
        return "ready".to_string();
    }
    [
        stream.remote_readiness.as_ref(),
        stream.readiness.as_ref(),
        route.and_then(|route| route.readiness.as_ref()),
        display_allocation.and_then(|allocation| allocation.readiness.as_ref()),
    ]
    .into_iter()
    .flatten()
    .find_map(|value| value.get("state").and_then(Value::as_str))
    .unwrap_or(display_state.as_str())
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::native::service_model::{ControlInputProvider, ViewerLease};

    #[test]
    fn attachability_reports_attached_ready_from_stream_display_proof() {
        let mut state = ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), ready_browser("browser-1"))]),
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    state: "ready".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    display_allocation_id: Some("display-1".to_string()),
                    provider_mode: "simultaneous_view".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-1".to_string(),
                RoutePoolEntry {
                    id: "pool-1".to_string(),
                    route_id: "route-1".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-1".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        let attachability = state.browsers["browser-1"].attachability.as_ref().unwrap();
        assert_eq!(attachability["state"], "attached_ready");
        assert_eq!(attachability["proofState"], "ready");
        assert_eq!(
            state.browsers["browser-1"].view_streams[0]
                .attachability
                .as_ref()
                .unwrap()["state"],
            "attached_ready"
        );
    }

    #[test]
    fn attachability_reports_stale_route_when_route_display_disagrees() {
        let mut browser = ready_browser("browser-1");
        browser.view_streams[0].display_allocation_id = Some("display-2".to_string());
        let mut state = ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), browser)]),
            display_allocations: BTreeMap::from([(
                "display-2".to_string(),
                DisplayAllocation {
                    id: "display-2".to_string(),
                    state: "ready".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    state: "orphaned".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    display_allocation_id: Some("display-1".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        let attachability = state.browsers["browser-1"].attachability.as_ref().unwrap();
        assert_eq!(attachability["state"], "reattachable_stale_route");
        assert_eq!(attachability["displayAgrees"], false);
    }

    #[test]
    fn attachability_reports_viewer_disconnected_for_ready_route_without_active_viewer() {
        let mut browser = ready_browser("browser-1");
        browser.view_streams[0].viewer_lease_ids = vec!["viewer-1".to_string()];
        let mut state = ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), browser)]),
            display_allocations: BTreeMap::from([(
                "display-1".to_string(),
                DisplayAllocation {
                    id: "display-1".to_string(),
                    state: "ready".to_string(),
                    owner_browser_id: Some("browser-1".to_string()),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-1".to_string(),
                RemoteViewRoute {
                    id: "route-1".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-1".to_string()),
                    display_allocation_id: Some("display-1".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            viewer_leases: BTreeMap::from([(
                "viewer-1".to_string(),
                ViewerLease {
                    id: "viewer-1".to_string(),
                    state: "disconnected".to_string(),
                    route_id: Some("route-1".to_string()),
                    browser_id: Some("browser-1".to_string()),
                    ..ViewerLease::default()
                },
            )]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        assert_eq!(
            state.browsers["browser-1"].attachability.as_ref().unwrap()["state"],
            "reattachable_viewer_disconnected"
        );
    }

    #[test]
    fn attachability_reports_route_occupied_when_live_browser_has_no_available_rdp_route() {
        let mut browser = ready_browser("browser-1");
        browser.view_streams.clear();
        let mut state = ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), browser)]),
            remote_view_routes: BTreeMap::from([(
                "route-2".to_string(),
                RemoteViewRoute {
                    id: "route-2".to_string(),
                    state: "ready".to_string(),
                    browser_id: Some("browser-2".to_string()),
                    ..RemoteViewRoute::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-2".to_string(),
                RoutePoolEntry {
                    id: "pool-2".to_string(),
                    route_id: "route-2".to_string(),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-2".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        let attachability = state.browsers["browser-1"].attachability.as_ref().unwrap();
        assert_eq!(attachability["state"], "reattachable_route_occupied");
        assert_eq!(
            attachability["recommendedAction"],
            "service_remote_view_route_switch"
        );
        assert_eq!(attachability["availableRoutePoolEntries"], json!([]));
    }

    #[test]
    fn remote_view_reattach_stress_keeps_four_browsers_visible_with_two_routes() {
        let mut browser_c = ready_browser_with_route("browser-c", None, "display-c");
        browser_c.profile_id = Some("profile-c".to_string());
        let mut browser_d = ready_browser_with_route("browser-d", None, "display-d");
        browser_d.profile_id = Some("profile-d".to_string());

        let mut state = ServiceState {
            browsers: BTreeMap::from([
                (
                    "browser-a".to_string(),
                    ready_browser_with_route("browser-a", Some("route-a"), "display-a"),
                ),
                (
                    "browser-b".to_string(),
                    ready_browser_with_route("browser-b", Some("route-b"), "display-b"),
                ),
                ("browser-c".to_string(), browser_c),
                ("browser-d".to_string(), browser_d),
            ]),
            display_allocations: BTreeMap::from([
                (
                    "display-a".to_string(),
                    ready_display("display-a", "browser-a"),
                ),
                (
                    "display-b".to_string(),
                    ready_display("display-b", "browser-b"),
                ),
                (
                    "display-c".to_string(),
                    ready_display("display-c", "browser-c"),
                ),
                (
                    "display-d".to_string(),
                    ready_display("display-d", "browser-d"),
                ),
            ]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-a".to_string(),
                    ready_route("route-a", "browser-a", "display-a"),
                ),
                (
                    "route-b".to_string(),
                    ready_route("route-b", "browser-b", "display-b"),
                ),
            ]),
            route_pool: BTreeMap::from([
                ("pool-a".to_string(), checked_out_pool("pool-a", "route-a")),
                ("pool-b".to_string(), checked_out_pool("pool-b", "route-b")),
            ]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        assert_eq!(state.browsers.len(), 4);
        assert_eq!(
            state.browsers["browser-a"].attachability.as_ref().unwrap()["state"],
            "attached_ready"
        );
        assert_eq!(
            state.browsers["browser-b"].attachability.as_ref().unwrap()["state"],
            "attached_ready"
        );
        assert_eq!(
            state.browsers["browser-c"].attachability.as_ref().unwrap()["state"],
            "reattachable_route_occupied"
        );
        assert_eq!(
            state.browsers["browser-d"].attachability.as_ref().unwrap()["state"],
            "reattachable_route_occupied"
        );
    }

    #[test]
    fn service_reconcile_reattach_stress_classifies_stale_and_closed_records() {
        let mut browser_a = ready_browser_with_route("browser-a", Some("route-a"), "display-a");
        browser_a.view_streams[0].display_allocation_id = Some("display-stale".to_string());
        let mut browser_b = ready_browser_with_route("browser-b", Some("route-b"), "display-b");
        browser_b.health = BrowserHealth::ProcessExited;

        let mut state = ServiceState {
            browsers: BTreeMap::from([
                ("browser-a".to_string(), browser_a),
                ("browser-b".to_string(), browser_b),
            ]),
            display_allocations: BTreeMap::from([
                (
                    "display-a".to_string(),
                    ready_display("display-a", "browser-a"),
                ),
                (
                    "display-stale".to_string(),
                    ready_display("display-stale", "browser-a"),
                ),
                (
                    "display-b".to_string(),
                    ready_display("display-b", "browser-b"),
                ),
            ]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-a".to_string(),
                    ready_route("route-a", "browser-a", "display-a"),
                ),
                (
                    "route-b".to_string(),
                    ready_route("route-b", "browser-b", "display-b"),
                ),
            ]),
            route_pool: BTreeMap::from([
                ("pool-a".to_string(), checked_out_pool("pool-a", "route-a")),
                ("pool-b".to_string(), checked_out_pool("pool-b", "route-b")),
            ]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        assert_eq!(
            state.browsers["browser-a"].attachability.as_ref().unwrap()["state"],
            "reattachable_stale_route"
        );
        assert_eq!(
            state.browsers["browser-a"].attachability.as_ref().unwrap()["recommendedAction"],
            "service_remote_view_browser_reattach"
        );
        assert_eq!(
            state.browsers["browser-b"].attachability.as_ref().unwrap()["state"],
            "not_reattachable_closed"
        );
    }

    #[test]
    fn service_profile_identity_stress_preserves_explicit_profile_attachability() {
        let mut browser_a = ready_browser_with_route("browser-a", Some("route-a"), "display-a");
        browser_a.profile_id = Some("explicit-runtime-profile-a".to_string());
        browser_a.active_session_ids = vec!["p67-session-a".to_string()];
        let mut browser_b = ready_browser_with_route("browser-b", Some("route-b"), "display-b");
        browser_b.profile_id = Some("explicit-runtime-profile-b".to_string());
        browser_b.active_session_ids = vec!["p67-session-b".to_string()];

        let mut state = ServiceState {
            browsers: BTreeMap::from([
                ("browser-a".to_string(), browser_a),
                ("browser-b".to_string(), browser_b),
            ]),
            display_allocations: BTreeMap::from([
                (
                    "display-a".to_string(),
                    ready_display("display-a", "browser-a"),
                ),
                (
                    "display-b".to_string(),
                    ready_display("display-b", "browser-b"),
                ),
            ]),
            remote_view_routes: BTreeMap::from([
                (
                    "route-a".to_string(),
                    ready_route("route-a", "browser-a", "display-a"),
                ),
                (
                    "route-b".to_string(),
                    ready_route("route-b", "browser-b", "display-b"),
                ),
            ]),
            route_pool: BTreeMap::from([
                ("pool-a".to_string(), checked_out_pool("pool-a", "route-a")),
                ("pool-b".to_string(), checked_out_pool("pool-b", "route-b")),
            ]),
            ..ServiceState::default()
        };

        refresh_remote_view_attachability(&mut state);

        assert_eq!(
            state.browsers["browser-a"].attachability.as_ref().unwrap()["profileId"],
            "explicit-runtime-profile-a"
        );
        assert_eq!(
            state.browsers["browser-b"].attachability.as_ref().unwrap()["profileId"],
            "explicit-runtime-profile-b"
        );
        assert_eq!(
            state.browsers["browser-a"].attachability.as_ref().unwrap()["sessionName"],
            "p67-session-a"
        );
        assert_eq!(
            state.browsers["browser-b"].attachability.as_ref().unwrap()["sessionName"],
            "p67-session-b"
        );
    }

    fn ready_browser(id: &str) -> BrowserProcess {
        ready_browser_with_route(id, Some("route-1"), "display-1")
    }

    fn ready_browser_with_route(
        id: &str,
        route_id: Option<&str>,
        display_id: &str,
    ) -> BrowserProcess {
        BrowserProcess {
            id: id.to_string(),
            profile_id: Some(format!("{id}-profile")),
            host: BrowserHost::RemoteHeaded,
            health: BrowserHealth::Ready,
            display_allocation_id: Some(display_id.to_string()),
            active_session_ids: vec![format!("{id}-session")],
            view_streams: route_id
                .map(|route_id| {
                    vec![ViewStream {
                        id: format!("{id}-remote-headed-view"),
                        provider: ViewStreamProvider::RdpGateway,
                        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                        route_id: Some(route_id.to_string()),
                        display_allocation_id: Some(display_id.to_string()),
                        read_only: false,
                        readiness: Some(json!({
                            "state": "ready",
                            "displayContent": {
                                "state": "browser_window_visible",
                            },
                        })),
                        ..ViewStream::default()
                    }]
                })
                .unwrap_or_default(),
            ..BrowserProcess::default()
        }
    }

    fn ready_display(id: &str, browser_id: &str) -> DisplayAllocation {
        DisplayAllocation {
            id: id.to_string(),
            state: "ready".to_string(),
            owner_browser_id: Some(browser_id.to_string()),
            readiness: Some(json!({
                "state": "ready",
                "displayContent": {
                    "state": "browser_window_visible",
                },
            })),
            ..DisplayAllocation::default()
        }
    }

    fn ready_route(id: &str, browser_id: &str, display_id: &str) -> RemoteViewRoute {
        RemoteViewRoute {
            id: id.to_string(),
            state: "ready".to_string(),
            browser_id: Some(browser_id.to_string()),
            display_allocation_id: Some(display_id.to_string()),
            provider_mode: "simultaneous_view".to_string(),
            readiness: Some(json!({
                "state": "ready",
                "displayContent": {
                    "state": "browser_window_visible",
                },
            })),
            ..RemoteViewRoute::default()
        }
    }

    fn checked_out_pool(id: &str, route_id: &str) -> RoutePoolEntry {
        RoutePoolEntry {
            id: id.to_string(),
            route_id: route_id.to_string(),
            state: "checked_out".to_string(),
            current_route_allocation_id: Some(route_id.to_string()),
            ..RoutePoolEntry::default()
        }
    }
}
