use super::open::{
    push_remote_view_service_event, remote_view_lease_is_active, required_remote_view_route_id,
    service_remote_view_timestamp,
};
use crate::native::action_runtime::runtime::{
    optional_command_string, service_browser_id, DaemonState,
};
use crate::native::desktop_control_coordinator::begin_service_controller_mutation;
use crate::native::remote_view_attachability::refresh_remote_view_attachability;
use crate::native::service_model::{
    advance_route_controller_authority, ServiceEventKind, ViewerLease,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use serde_json::{json, Value};
pub(crate) async fn handle_service_viewer_lease_request(
    cmd: &Value,
    daemon_state: &DaemonState,
) -> Result<Value, String> {
    mutate_service_viewer_lease(cmd, daemon_state, false)
}
pub(crate) async fn handle_service_controller_lease_takeover(
    cmd: &Value,
    daemon_state: &DaemonState,
) -> Result<Value, String> {
    mutate_service_viewer_lease(cmd, daemon_state, true)
}
pub(crate) fn mutate_service_viewer_lease(
    cmd: &Value,
    daemon_state: &DaemonState,
    controller_takeover: bool,
) -> Result<Value, String> {
    let route_id = required_remote_view_route_id(cmd)?;
    let requested_role =
        optional_command_string(cmd, "viewerRole").unwrap_or_else(|| "observer".to_string());
    let wants_controller = controller_takeover || requested_role == "controller";
    let now = service_remote_view_timestamp();
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let profile_policy_before = controller_takeover
        .then(|| route_profile_policy_snapshot(&snapshot, &route_id))
        .flatten();
    let _controller_mutation = wants_controller
        .then(|| begin_service_controller_mutation(&snapshot, &route_id))
        .transpose()?;
    repository.mutate(|state| {
        let route_snapshot = state
            .remote_view_routes
            .get(&route_id)
            .cloned()
            .ok_or_else(|| format!("remote view route '{}' not found", route_id))?;
        if route_snapshot.state == "released" {
            return Err(format!("remote view route '{}' is released", route_id));
        }
        let browser_id = optional_command_string(cmd, "browserId")
            .or_else(|| route_snapshot.browser_id.clone())
            .unwrap_or_else(|| service_browser_id(&daemon_state.session_id));
        let viewer_id =
            optional_command_string(cmd, "viewerId").unwrap_or_else(|| "operator".to_string());
        let viewer_lease_id = optional_command_string(cmd, "viewerLeaseId").unwrap_or_else(|| {
            format!(
                "viewer:{}:{}:{}",
                route_id,
                viewer_id,
                now.replace([':', '.'], "-")
            )
        });
        let active_viewer_count = route_snapshot
            .viewer_lease_ids
            .iter()
            .filter(|lease_id| lease_id.as_str() != viewer_lease_id.as_str())
            .filter_map(|lease_id| state.viewer_leases.get(lease_id))
            .filter(|lease| remote_view_lease_is_active(lease))
            .count();
        let previous_controller_lease_id = route_snapshot.controller_lease_id.clone();
        let previous_controller_is_other = previous_controller_lease_id
            .as_deref()
            .is_some_and(|id| id != viewer_lease_id.as_str());
        let provider_mode = route_snapshot.provider_mode.as_str();
        let viewer_role = if controller_takeover {
            "controller".to_string()
        } else {
            requested_role.clone()
        };
        if provider_mode == "single_viewer" && active_viewer_count > 0 {
            let event_id = push_remote_view_service_event(
                state,
                ServiceEventKind::ControllerDenied,
                &now,
                route_snapshot.browser_id.clone(),
                route_snapshot.session_id.clone(),
                format!(
                    "Remote view route '{}' rejected additional viewer",
                    route_id
                ),
                json!(
                    { "routeId" : route_id, "viewerLeaseId" : viewer_lease_id,
                    "viewerId" : viewer_id, "providerMode" : route_snapshot
                    .provider_mode, "reason" : "single_viewer_active", }
                ),
            );
            return Ok(json!(
                { "status" : "viewer_denied", "reason" : "single_viewer_active",
                "routeId" : route_id, "remoteViewRouteId" : route_id,
                "viewerLeaseId" : viewer_lease_id, "serviceEventId" : event_id,
                "updatedAt" : now, }
            ));
        }
        if wants_controller && previous_controller_is_other && !controller_takeover {
            let event_id = push_remote_view_service_event(
                state,
                ServiceEventKind::ControllerDenied,
                &now,
                route_snapshot.browser_id.clone(),
                route_snapshot.session_id.clone(),
                format!(
                    "Remote view route '{}' rejected controller request",
                    route_id
                ),
                json!(
                    { "routeId" : route_id, "viewerLeaseId" : viewer_lease_id,
                    "viewerId" : viewer_id, "providerMode" : route_snapshot
                    .provider_mode, "previousControllerLeaseId" :
                    previous_controller_lease_id, "reason" : "controller_active", }
                ),
            );
            return Ok(json!(
                { "status" : "controller_denied", "reason" : "controller_active",
                "routeId" : route_id, "remoteViewRouteId" : route_id,
                "viewerLeaseId" : viewer_lease_id, "controllerLeaseId" :
                previous_controller_lease_id, "serviceEventId" : event_id,
                "updatedAt" : now, }
            ));
        }
        if wants_controller {
            push_remote_view_service_event(
                state,
                ServiceEventKind::ControllerRequested,
                &now,
                route_snapshot.browser_id.clone(),
                route_snapshot.session_id.clone(),
                format!("Remote view route '{}' controller requested", route_id),
                json!(
                    { "routeId" : route_id, "viewerLeaseId" : viewer_lease_id,
                    "viewerId" : viewer_id, "providerMode" : route_snapshot
                    .provider_mode, "takeover" : controller_takeover,
                    "previousControllerLeaseId" : previous_controller_lease_id, }
                ),
            );
        }
        let route = state
            .remote_view_routes
            .get_mut(&route_id)
            .ok_or_else(|| format!("remote view route '{}' not found", route_id))?;
        if route.state == "released" {
            return Err(format!("remote view route '{}' is released", route_id));
        }
        let viewer_name = optional_command_string(cmd, "viewerName")
            .or_else(|| optional_command_string(cmd, "agentName"))
            .or_else(|| Some(viewer_id.clone()));
        let open_mode =
            optional_command_string(cmd, "openMode").unwrap_or_else(|| "embedded".to_string());
        let state_value = if viewer_role == "controller" {
            "controlling"
        } else {
            "observing"
        };
        let last_viewer_event = if controller_takeover {
            "taken_over"
        } else {
            "connected"
        };
        let lease = ViewerLease {
            id: viewer_lease_id.clone(),
            boot_epoch: crate::process_identity::current_boot_epoch(),
            route_id: Some(route_id.clone()),
            browser_id: Some(browser_id.clone()),
            viewer_id: Some(viewer_id),
            viewer_name,
            viewer_role: viewer_role.clone(),
            open_mode,
            state: state_value.to_string(),
            last_viewer_event: Some(last_viewer_event.to_string()),
            created_at: state
                .viewer_leases
                .get(&viewer_lease_id)
                .and_then(|lease| lease.created_at.clone())
                .or_else(|| Some(now.clone())),
            updated_at: Some(now.clone()),
            last_heartbeat_at: Some(now.clone()),
            expires_at: optional_command_string(cmd, "expiresAt"),
            service_event_id: optional_command_string(cmd, "serviceEventId"),
        };
        state
            .viewer_leases
            .insert(viewer_lease_id.clone(), lease.clone());
        if !route.viewer_lease_ids.contains(&viewer_lease_id) {
            route.viewer_lease_ids.push(viewer_lease_id.clone());
        }
        route.last_provider_event = Some(last_viewer_event.to_string());
        if viewer_role == "controller" {
            advance_route_controller_authority(state, &route_id, Some(viewer_lease_id.clone()))?;
        }
        let remote_view_route = state
            .remote_view_routes
            .get(&route_id)
            .cloned()
            .ok_or_else(|| format!("remote view route '{}' not found", route_id))?;
        let controller_lease_id = remote_view_route.controller_lease_id.clone();
        if let Some(browser) = state.browsers.get_mut(&browser_id) {
            for stream in &mut browser.view_streams {
                if stream.route_id.as_deref() == Some(route_id.as_str())
                    && !stream.viewer_lease_ids.contains(&viewer_lease_id)
                {
                    stream.viewer_lease_ids.push(viewer_lease_id.clone());
                }
            }
        }
        let event_kind = if viewer_role == "controller" {
            ServiceEventKind::ControllerGranted
        } else {
            ServiceEventKind::ViewerConnected
        };
        let service_event_id = push_remote_view_service_event(
            state,
            event_kind,
            &now,
            remote_view_route.browser_id.clone(),
            remote_view_route.session_id.clone(),
            format!(
                "Remote view route '{}' {}",
                route_id,
                if viewer_role == "controller" {
                    "controller granted"
                } else {
                    "viewer connected"
                }
            ),
            json!(
                { "routeId" : route_id, "viewerLeaseId" : viewer_lease_id,
                "viewerRole" : viewer_role, "providerMode" : remote_view_route
                .provider_mode, "previousControllerLeaseId" :
                previous_controller_lease_id, "controllerLeaseId" :
                controller_lease_id, }
            ),
        );
        refresh_remote_view_attachability(state);
        let profile_policy_after = controller_takeover
            .then(|| route_profile_policy_snapshot(state, &route_id))
            .flatten();
        if controller_takeover && profile_policy_after != profile_policy_before {
            return Err("controller_takeover_profile_policy_changed".to_string());
        }
        let browser_attachability = state
            .browsers
            .get(&browser_id)
            .and_then(|browser| browser.attachability.clone());
        Ok(json!(
            { "status" : if controller_takeover { "controller_taken" } else {
            "viewer_connected" }, "routeId" : route_id, "remoteViewRouteId" :
            route_id, "viewerLeaseId" : viewer_lease_id, "controllerLeaseId" :
            controller_lease_id, "previousControllerLeaseId" :
            previous_controller_lease_id, "controllerEpoch" : remote_view_route
            .controller_epoch, "serviceEventId" : service_event_id,
            "profilePolicyUnchanged" : controller_takeover.then_some(true),
            "viewerLease" : lease, "remoteViewRoute" : remote_view_route,
            "attachability" : browser_attachability, "updatedAt" : now, }
        ))
    })
}

fn route_profile_policy_snapshot(
    state: &crate::native::service_model::ServiceState,
    route_id: &str,
) -> Option<(
    String,
    super::super::service_profile_access_policy::ServiceProfileAccessPolicy,
)> {
    let route = state.remote_view_routes.get(route_id)?;
    let profile_id = route
        .session_id
        .as_deref()
        .and_then(|session_id| state.sessions.get(session_id))
        .and_then(|session| session.profile_id.as_deref())
        .or_else(|| {
            route
                .browser_id
                .as_deref()
                .and_then(|browser_id| state.browsers.get(browser_id))
                .and_then(|browser| browser.profile_id.as_deref())
        })?;
    let policy = state.profiles.get(profile_id)?.access_policy.clone()?;
    Some((profile_id.to_string(), policy))
}
pub(crate) async fn handle_service_viewer_lease_heartbeat(
    cmd: &Value,
    _daemon_state: &DaemonState,
) -> Result<Value, String> {
    let viewer_lease_id = optional_command_string(cmd, "viewerLeaseId")
        .ok_or_else(|| "service_viewer_lease_heartbeat requires viewerLeaseId".to_string())?;
    let now = service_remote_view_timestamp();
    let repository = LockedServiceStateRepository::default_json()?;
    repository.mutate(|state| {
        let lease = state
            .viewer_leases
            .get_mut(&viewer_lease_id)
            .ok_or_else(|| format!("viewer lease '{}' not found", viewer_lease_id))?;
        if matches!(lease.state.as_str(), "disconnected" | "expired" | "failed") {
            return Err(format!(
                "viewer_lease_inactive: viewer lease '{}' is {}",
                viewer_lease_id, lease.state
            ));
        }
        lease.last_viewer_event = Some("heartbeat".to_string());
        lease.updated_at = Some(now.clone());
        lease.last_heartbeat_at = Some(now.clone());
        if let Some(expires_at) = optional_command_string(cmd, "expiresAt") {
            lease.expires_at = Some(expires_at);
        }
        let route_id = lease.route_id.clone();
        refresh_remote_view_attachability(state);
        Ok(json!(
            { "status" : "viewer_heartbeat", "viewerLeaseId" : viewer_lease_id,
            "routeId" : route_id, "viewerLease" : state.viewer_leases.get(&
            viewer_lease_id), "updatedAt" : now, }
        ))
    })
}
pub(crate) async fn handle_service_viewer_lease_release(
    cmd: &Value,
    _daemon_state: &DaemonState,
) -> Result<Value, String> {
    let viewer_lease_id = optional_command_string(cmd, "viewerLeaseId")
        .ok_or_else(|| "service_viewer_lease_release requires viewerLeaseId".to_string())?;
    let now = service_remote_view_timestamp();
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let controlled_route_id = snapshot
        .viewer_leases
        .get(&viewer_lease_id)
        .and_then(|lease| lease.route_id.as_ref())
        .filter(|route_id| {
            snapshot
                .remote_view_routes
                .get(*route_id)
                .is_some_and(|route| {
                    route.controller_lease_id.as_deref() == Some(viewer_lease_id.as_str())
                })
        })
        .cloned();
    let _controller_mutation = controlled_route_id
        .as_deref()
        .map(|route_id| begin_service_controller_mutation(&snapshot, route_id))
        .transpose()?;
    repository.mutate(|state| {
        let lease = state
            .viewer_leases
            .get_mut(&viewer_lease_id)
            .ok_or_else(|| format!("viewer lease '{}' not found", viewer_lease_id))?;
        lease.state = "disconnected".to_string();
        lease.last_viewer_event = Some("disconnected".to_string());
        lease.updated_at = Some(now.clone());
        lease.last_heartbeat_at = Some(now.clone());
        let route_id = lease.route_id.clone();
        let browser_id = lease.browser_id.clone();
        if let Some(route_id) = route_id.as_ref() {
            let is_primary = state.remote_view_routes.get(route_id).is_some_and(|route| {
                route.controller_lease_id.as_deref() == Some(viewer_lease_id.as_str())
            });
            if is_primary {
                if controlled_route_id.as_deref() != Some(route_id.as_str()) {
                    return Err("desktop_control_coordinator_fence_required".to_string());
                }
                advance_route_controller_authority(state, route_id, None)?;
            }
            if let Some(route) = state.remote_view_routes.get_mut(route_id) {
                route.viewer_lease_ids.retain(|id| id != &viewer_lease_id);
                route.last_provider_event = Some("viewer_released".to_string());
            }
        }
        if let Some(route_id) = route_id.as_ref() {
            let route = state.remote_view_routes.get(route_id).cloned();
            for browser in state.browsers.values_mut() {
                for stream in &mut browser.view_streams {
                    if stream.route_id.as_deref() != Some(route_id.as_str()) {
                        continue;
                    }
                    stream.viewer_lease_ids.retain(|id| id != &viewer_lease_id);
                    if let Some(route) = route.as_ref() {
                        stream.project_controller(route);
                    }
                }
            }
        }
        push_remote_view_service_event(
            state,
            ServiceEventKind::ViewerDisconnected,
            &now,
            browser_id,
            None,
            format!("Remote view viewer lease '{}' released", viewer_lease_id),
            json!({ "routeId" : route_id, "viewerLeaseId" : viewer_lease_id, }),
        );
        refresh_remote_view_attachability(state);
        Ok(json!(
            { "status" : "released", "viewerLeaseId" : viewer_lease_id, "routeId"
            : route_id, "viewerLease" : state.viewer_leases.get(&
            viewer_lease_id), "updatedAt" : now, }
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, RemoteViewRoute, ServiceState, ViewStream,
    };
    use crate::native::service_profile_access_policy::ServiceProfileAccessPolicy;
    use std::collections::BTreeMap;

    #[test]
    fn human_takeover_changes_only_controller_authority_and_fences_the_old_controller() {
        let policy = ServiceProfileAccessPolicy::shared_local_default("research-gov");
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "research-gov".to_string(),
                BrowserProfile {
                    id: "research-gov".to_string(),
                    access_policy: Some(policy.clone()),
                    ..BrowserProfile::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "research-runtime".to_string(),
                BrowserSession {
                    id: "research-runtime".to_string(),
                    profile_id: Some("research-gov".to_string()),
                    ..BrowserSession::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route:research".to_string(),
                RemoteViewRoute {
                    id: "route:research".to_string(),
                    browser_id: Some("browser:research".to_string()),
                    session_id: Some("research-runtime".to_string()),
                    controller_lease_id: Some("lease:agent".to_string()),
                    controller_epoch: 7,
                    ..RemoteViewRoute::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser:research".to_string(),
                BrowserProcess {
                    id: "browser:research".to_string(),
                    profile_id: Some("research-gov".to_string()),
                    view_streams: vec![ViewStream {
                        id: "stream:research".to_string(),
                        route_id: Some("route:research".to_string()),
                        controller_lease_id: Some("lease:agent".to_string()),
                        controller_epoch: 7,
                        ..ViewStream::default()
                    }],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let before = route_profile_policy_snapshot(&state, "route:research");

        let epoch = advance_route_controller_authority(
            &mut state,
            "route:research",
            Some("lease:human".to_string()),
        )
        .unwrap();

        assert_eq!(epoch, 8);
        assert_eq!(
            route_profile_policy_snapshot(&state, "route:research"),
            before
        );
        assert_eq!(state.profiles["research-gov"].access_policy, Some(policy));
        assert!(
            !crate::native::service_model::controller_authority_fence_matches(
                &state,
                "route:research",
                "stream:research",
                "lease:agent",
                7,
            )
        );
        assert!(
            crate::native::service_model::controller_authority_fence_matches(
                &state,
                "route:research",
                "stream:research",
                "lease:human",
                8,
            )
        );
    }
}
