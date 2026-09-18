//! Durable remote-view handoffs for ordinary Browser Session Manager browsers.

use agent_browser_service_model::{BrowserSessionState, RemoteViewHandoff, ServiceState};
use serde_json::{json, Value};

use super::presentation_inventory::StaticRouteInventory;
use super::remote_view::RemoteViewRouteBinding;
use super::remote_view_handoff::durable_remote_view_handoff_url;
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

pub(crate) fn attach_manager_handoff(
    response: &mut Value,
    sessions: &BrowserSessionState,
) -> Result<(), String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let inventory = StaticRouteInventory::from_environment()?;
    attach_manager_handoff_in_repository(response, sessions, &repository, &inventory)
}

fn attach_manager_handoff_in_repository(
    response: &mut Value,
    sessions: &BrowserSessionState,
    repository: &impl ServiceStateRepository,
    inventory: &StaticRouteInventory,
) -> Result<(), String> {
    if let Err(error) =
        try_attach_manager_handoff_in_repository(response, sessions, repository, inventory)
    {
        response["success"] = Value::Bool(false);
        response["error"] = Value::String(error.clone());
        response["data"]["operatorVisible"] = json!({
            "state": "unavailable",
            "reason": error,
        });
        return Err(error);
    }
    Ok(())
}

fn try_attach_manager_handoff_in_repository(
    response: &mut Value,
    sessions: &BrowserSessionState,
    repository: &impl ServiceStateRepository,
    inventory: &StaticRouteInventory,
) -> Result<(), String> {
    if response.get("success").and_then(Value::as_bool) != Some(true) {
        return Ok(());
    }
    let data = response
        .get("data")
        .ok_or_else(|| "browser_session_handoff_navigation_data_missing".to_string())?;
    let session_id = data
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "browser_session_handoff_session_id_missing".to_string())?;
    let session = sessions
        .sessions
        .get(session_id)
        .ok_or_else(|| "browser_session_handoff_session_missing".to_string())?;
    let browser = sessions
        .browsers
        .get(&session.browser_id)
        .ok_or_else(|| "browser_session_handoff_browser_missing".to_string())?;
    let desktop = browser
        .desktop
        .as_ref()
        .ok_or_else(|| "browser_session_handoff_desktop_missing".to_string())?;
    let tab_id = data
        .get("tabId")
        .and_then(Value::as_str)
        .ok_or_else(|| "browser_session_handoff_tab_id_missing".to_string())?;
    let tab = sessions
        .tabs
        .get(tab_id)
        .ok_or_else(|| "browser_session_handoff_tab_missing".to_string())?;
    if tab.session_id != session.id || tab.browser_id != browser.id {
        return Err("browser_session_handoff_attribution_mismatch".to_string());
    }
    let desired_url = data.get("url").and_then(Value::as_str).map(str::to_string);
    let candidate_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let projection = repository.mutate(|service| {
        let binding = manager_route_binding(
            service,
            inventory,
            &browser.id,
            &desktop.route_id,
            &desktop.display_name,
        )?;
        let handoff_id = service
            .remote_view_handoffs
            .values()
            .find(|handoff| {
                is_manager_handoff(handoff)
                    && handoff.browser_id.as_deref() == Some(browser.id.as_str())
                    && handoff.session_name.as_deref() == Some(session.name.as_str())
            })
            .map(|handoff| handoff.id.clone())
            .unwrap_or_else(|| candidate_id.clone());
        let handoff_url = durable_remote_view_handoff_url(&binding, &handoff_id)
            .ok_or_else(|| "browser_session_handoff_public_operator_url_missing".to_string())?;
        let created_at = service
            .remote_view_handoffs
            .get(&handoff_id)
            .and_then(|handoff| handoff.created_at.clone())
            .or_else(|| Some(now.clone()));
        let handoff = RemoteViewHandoff {
            id: handoff_id.clone(),
            state: "ready".to_string(),
            intent: json!({
                "browserSessionManager": true,
                "profileId": session.profile_id,
                "sessionId": session.id,
                "sessionName": session.name,
                "browserId": browser.id,
                "tabId": tab.id,
                "targetId": tab.target_id,
                "routePoolEntryId": desktop.route_id,
                "displayName": desktop.display_name,
            }),
            handoff_url: Some(handoff_url.clone()),
            desired_url: desired_url.clone(),
            profile_id: Some(session.profile_id.clone()),
            browser_id: Some(browser.id.clone()),
            session_name: Some(session.name.clone()),
            tab_id: Some(tab.id.clone()),
            target_id: Some(tab.target_id.clone()),
            view_stream_provider: Some(binding.provider),
            control_input: service
                .remote_view_routes
                .get(&binding.route_id)
                .and_then(|route| route.control_input),
            last_route_id: Some(binding.route_id.clone()),
            last_route_pool_entry_id: binding.route_pool_entry_id.clone(),
            last_display_allocation_id: Some(binding.display_allocation_id.clone()),
            created_at,
            updated_at: Some(now.clone()),
            last_resolved_at: Some(now.clone()),
            last_resolution: Some(json!({
                "status": "ready",
                "resolved": true,
                "browserSessionManager": true,
                "browserId": browser.id,
                "sessionName": session.name,
                "tabId": tab.id,
                "targetId": tab.target_id,
                "routeId": binding.route_id,
                "operatorVisible": { "state": "ready" },
            })),
            presentation_receipt: None,
        };
        service
            .remote_view_handoffs
            .insert(handoff_id.clone(), handoff);
        Ok(json!({
            "handoffId": handoff_id,
            "handoffUrl": handoff_url,
            "operatorVisible": { "state": "ready" },
            "viewStreamProvider": binding.provider,
        }))
    })?;
    if let (Some(data), Some(projection)) = (
        response.get_mut("data").and_then(Value::as_object_mut),
        projection.as_object(),
    ) {
        for (key, value) in projection {
            data.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

pub(crate) fn is_manager_handoff(handoff: &RemoteViewHandoff) -> bool {
    handoff
        .intent
        .get("browserSessionManager")
        .and_then(Value::as_bool)
        == Some(true)
}

pub(crate) fn manager_route_binding(
    service: &ServiceState,
    inventory: &StaticRouteInventory,
    browser_id: &str,
    configured_route_id: &str,
    display_name: &str,
) -> Result<RemoteViewRouteBinding, String> {
    let configured = inventory
        .routes()
        .iter()
        .find(|route| {
            route.id == configured_route_id && route.display_name.as_deref() == Some(display_name)
        })
        .ok_or_else(|| "browser_session_handoff_static_viewer_not_configured".to_string())?;
    let mut displays = service.display_allocations.values().filter(|display| {
        display.display_name.as_deref() == Some(display_name)
            && display.state == "ready"
            && display
                .readiness
                .as_ref()
                .and_then(|value| value.get("state"))
                .and_then(Value::as_str)
                == Some("ready")
            && display
                .owner_browser_id
                .as_deref()
                .is_none_or(|owner| owner == browser_id)
    });
    let display = displays
        .next()
        .ok_or_else(|| "browser_session_handoff_display_not_ready".to_string())?;
    if displays.next().is_some() {
        return Err("browser_session_handoff_display_ambiguous".to_string());
    }
    let mut routes = display.route_ids.iter().filter_map(|route_id| {
        service.remote_view_routes.get(route_id).filter(|route| {
            route.display_allocation_id.as_deref() == Some(display.id.as_str())
                && route.state == "ready"
                && route
                    .readiness
                    .as_ref()
                    .and_then(|value| value.get("state"))
                    .and_then(Value::as_str)
                    == Some("ready")
                && route
                    .browser_id
                    .as_deref()
                    .is_none_or(|owner| owner == browser_id)
                && route.control_input.is_some()
        })
    });
    let route = routes
        .next()
        .ok_or_else(|| "browser_session_handoff_route_not_ready".to_string())?;
    if routes.next().is_some() {
        return Err("browser_session_handoff_route_ambiguous".to_string());
    }
    Ok(RemoteViewRouteBinding {
        route_id: route.id.clone(),
        route_pool_entry_id: Some(configured.id.clone()),
        display_allocation_id: display.id.clone(),
        route_pool_entry_state: None,
        current_route_allocation_id: None,
        display_name: Some(display_name.to_string()),
        launch_display_name: Some(display_name.to_string()),
        display_isolation: display.display_isolation.clone(),
        route_user: configured.route_user.clone(),
        display_access: None,
        provider: route.provider,
        provider_mode: route.provider_mode.clone(),
        connection_id: route.connection_id.clone(),
        connection_name: route.connection_name.clone(),
        frame_url: route.frame_url.clone(),
        external_url: route.external_url.clone(),
        route_descriptor: route.route_descriptor.clone(),
        readiness: route.readiness.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_store::JsonServiceStateStore;
    use agent_browser_service_model::{
        BrowserDesktopAssignment, ControlInputProvider, DisplayAllocation, ManagedBrowserInstance,
        ManagedBrowserSession, ManagedBrowserTab, RemoteViewRoute, RoutePoolEntry,
    };
    use std::collections::BTreeMap;

    fn managed_sessions() -> BrowserSessionState {
        BrowserSessionState {
            sessions: BTreeMap::from([(
                "session-a".to_string(),
                ManagedBrowserSession {
                    id: "session-a".to_string(),
                    name: "alice".to_string(),
                    profile_id: "work".to_string(),
                    browser_id: "browser-a".to_string(),
                    created_at_ms: 1,
                    last_activity_at_ms: 2,
                    expires_at_ms: 10,
                    current_tab_id: Some("tab-a".to_string()),
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-a".to_string(),
                ManagedBrowserInstance {
                    id: "browser-a".to_string(),
                    profile_id: "work".to_string(),
                    pid: 42,
                    cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/a".to_string(),
                    process_identity: None,
                    desktop: Some(BrowserDesktopAssignment {
                        route_id: "slot-a".to_string(),
                        display_name: ":10".to_string(),
                        live_browser_count: 0,
                    }),
                    active_session_ids: vec!["session-a".to_string()],
                },
            )]),
            tabs: BTreeMap::from([(
                "tab-a".to_string(),
                ManagedBrowserTab {
                    id: "tab-a".to_string(),
                    target_id: "target-a".to_string(),
                    browser_id: "browser-a".to_string(),
                    session_id: "session-a".to_string(),
                    created_at_ms: 1,
                    last_activity_at_ms: 2,
                },
            )]),
            ..BrowserSessionState::default()
        }
    }

    #[test]
    fn manager_route_requires_exact_ready_static_display_and_public_origin() {
        let mut service = ServiceState::default();
        service.route_pool.insert(
            "slot-a".to_string(),
            RoutePoolEntry {
                id: "slot-a".to_string(),
                route_id: "route-a".to_string(),
                target: json!({"displayName": ":10", "displayAllocationId": "display-a"}),
                state: "available".to_string(),
                readiness: Some(json!({"state":"ready"})),
                ..RoutePoolEntry::default()
            },
        );
        service.display_allocations.insert(
            "display-a".to_string(),
            DisplayAllocation {
                id: "display-a".to_string(),
                display_name: Some(":10".to_string()),
                owner_browser_id: Some("browser-a".to_string()),
                state: "ready".to_string(),
                route_ids: vec!["route-a".to_string()],
                readiness: Some(json!({"state":"ready"})),
                ..DisplayAllocation::default()
            },
        );
        service.remote_view_routes.insert(
            "route-a".to_string(),
            RemoteViewRoute {
                id: "route-a".to_string(),
                display_allocation_id: Some("display-a".to_string()),
                route_descriptor: Some(
                    json!({"publicOperatorUrl":"https://dashboard.example/operator"}),
                ),
                control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                state: "ready".to_string(),
                readiness: Some(json!({"state":"ready"})),
                ..RemoteViewRoute::default()
            },
        );
        let inventory =
            StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap();
        let binding =
            manager_route_binding(&service, &inventory, "browser-a", "slot-a", ":10").unwrap();
        assert_eq!(
            durable_remote_view_handoff_url(&binding, "opaque-a").as_deref(),
            Some("https://dashboard.example/remote-view/opaque-a")
        );
        service.route_pool.get_mut("slot-a").unwrap().state = "stale".to_string();
        let binding =
            manager_route_binding(&service, &inventory, "browser-a", "slot-a", ":10").unwrap();
        assert_eq!(binding.route_pool_entry_state, None);
        assert_eq!(binding.current_route_allocation_id, None);
    }

    #[test]
    fn managed_navigation_persists_one_reusable_opaque_handoff() {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-manager-handoff-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("state.json");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|service| {
                service.route_pool.insert(
                    "slot-a".to_string(),
                    RoutePoolEntry {
                        id: "slot-a".to_string(),
                        route_id: "route-a".to_string(),
                        target: json!({"displayName": ":10", "displayAllocationId": "display-a"}),
                        state: "available".to_string(),
                        readiness: Some(json!({"state":"ready"})),
                        ..RoutePoolEntry::default()
                    },
                );
                service.remote_view_routes.insert(
                    "route-a".to_string(),
                    RemoteViewRoute {
                        id: "route-a".to_string(),
                        display_allocation_id: Some("display-a".to_string()),
                        route_descriptor: Some(
                            json!({"publicOperatorUrl":"https://dashboard.example/operator"}),
                        ),
                        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                        state: "ready".to_string(),
                        readiness: Some(json!({"state":"ready"})),
                        ..RemoteViewRoute::default()
                    },
                );
                service.display_allocations.insert(
                    "display-a".to_string(),
                    DisplayAllocation {
                        id: "display-a".to_string(),
                        display_name: Some(":10".to_string()),
                        owner_browser_id: Some("browser-a".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-a".to_string()],
                        readiness: Some(json!({"state":"ready"})),
                        ..DisplayAllocation::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        let sessions = managed_sessions();
        let mut response = json!({"success":true,"data":{
            "sessionId":"session-a","browserId":"browser-a","tabId":"tab-a",
            "targetId":"target-a","url":"https://example.test/login"
        }});
        let inventory =
            StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap();
        attach_manager_handoff_in_repository(&mut response, &sessions, &repository, &inventory)
            .unwrap();
        let first_id = response["data"]["handoffId"].as_str().unwrap().to_string();
        assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
        assert_eq!(
            response["data"]["handoffUrl"],
            format!("https://dashboard.example/remote-view/{first_id}")
        );
        response["data"]["url"] = json!("https://example.test/account");
        attach_manager_handoff_in_repository(&mut response, &sessions, &repository, &inventory)
            .unwrap();
        assert_eq!(response["data"]["handoffId"], first_id);
        let snapshot = repository.load_snapshot().unwrap();
        assert_eq!(snapshot.remote_view_handoffs.len(), 1);
        let handoff = snapshot.remote_view_handoffs.values().next().unwrap();
        assert_eq!(
            handoff.desired_url.as_deref(),
            Some("https://example.test/account")
        );
        assert_eq!(handoff.intent["browserSessionManager"], true);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn managed_navigation_fails_when_required_opaque_handoff_cannot_be_published() {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-manager-handoff-failure-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(
            directory.join("state.json"),
        ));
        repository
            .mutate(|service| {
                service.route_pool.insert(
                    "slot-a".to_string(),
                    RoutePoolEntry {
                        id: "slot-a".to_string(),
                        route_id: "route-a".to_string(),
                        target: json!({"displayName": ":10", "displayAllocationId": "display-a"}),
                        state: "available".to_string(),
                        readiness: Some(json!({"state":"ready"})),
                        ..RoutePoolEntry::default()
                    },
                );
                service.remote_view_routes.insert(
                    "route-a".to_string(),
                    RemoteViewRoute {
                        id: "route-a".to_string(),
                        display_allocation_id: Some("display-a".to_string()),
                        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                        state: "ready".to_string(),
                        readiness: Some(json!({"state":"ready"})),
                        ..RemoteViewRoute::default()
                    },
                );
                service.display_allocations.insert(
                    "display-a".to_string(),
                    DisplayAllocation {
                        id: "display-a".to_string(),
                        display_name: Some(":10".to_string()),
                        owner_browser_id: Some("browser-a".to_string()),
                        state: "ready".to_string(),
                        route_ids: vec!["route-a".to_string()],
                        readiness: Some(json!({"state":"ready"})),
                        ..DisplayAllocation::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        let mut response = json!({"id":"navigate-a","success":true,"data":{
            "sessionId":"session-a","browserId":"browser-a","tabId":"tab-a",
            "targetId":"target-a","url":"https://example.test/login"
        }});

        let error = attach_manager_handoff_in_repository(
            &mut response,
            &managed_sessions(),
            &repository,
            &StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap(),
        )
        .unwrap_err();

        assert_eq!(error, "browser_session_handoff_public_operator_url_missing");
        assert_eq!(response["success"], false);
        assert_eq!(
            response["error"],
            "browser_session_handoff_public_operator_url_missing"
        );
        assert_eq!(response["data"]["operatorVisible"]["state"], "unavailable");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn managed_navigation_fails_when_no_desktop_is_assigned() {
        let directory = std::env::temp_dir().join(format!(
            "agent-browser-manager-handoff-no-desktop-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(
            directory.join("state.json"),
        ));
        let mut sessions = managed_sessions();
        sessions.browsers.get_mut("browser-a").unwrap().desktop = None;
        let mut response = json!({"id":"navigate-a","success":true,"data":{
            "sessionId":"session-a","browserId":"browser-a","tabId":"tab-a",
            "targetId":"target-a","url":"https://example.test/login"
        }});

        let error = attach_manager_handoff_in_repository(
            &mut response,
            &sessions,
            &repository,
            &StaticRouteInventory::from_json(r#"[{"id":"slot-a","target":{"displayName":":10"}}]"#)
                .unwrap(),
        )
        .unwrap_err();

        assert_eq!(error, "browser_session_handoff_desktop_missing");
        assert_eq!(response["success"], false);
        assert_eq!(response["data"]["operatorVisible"]["state"], "unavailable");
        let _ = std::fs::remove_dir_all(directory);
    }
}
