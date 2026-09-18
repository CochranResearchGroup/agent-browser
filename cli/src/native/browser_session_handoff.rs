//! Durable remote-view handoffs for ordinary Browser Session Manager browsers.

use agent_browser_service_model::{BrowserSessionState, RemoteViewHandoff, ServiceState};
use serde_json::{json, Value};

use super::browser::BrowserManager;
use super::remote_view::RemoteViewRouteBinding;
use super::remote_view_handoff::durable_remote_view_handoff_url;
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

pub(crate) fn attach_manager_handoff(
    response: &mut Value,
    sessions: &BrowserSessionState,
) -> Result<(), String> {
    let repository = LockedServiceStateRepository::default_json()?;
    attach_manager_handoff_in_repository(response, sessions, &repository)
}

fn attach_manager_handoff_in_repository(
    response: &mut Value,
    sessions: &BrowserSessionState,
    repository: &impl ServiceStateRepository,
) -> Result<(), String> {
    if response.get("success").and_then(Value::as_bool) != Some(true) {
        return Ok(());
    }
    let Some(data) = response.get("data") else {
        return Ok(());
    };
    let Some(session_id) = data.get("sessionId").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(session) = sessions.sessions.get(session_id) else {
        return Ok(());
    };
    let Some(browser) = sessions.browsers.get(&session.browser_id) else {
        return Ok(());
    };
    let Some(desktop) = browser.desktop.as_ref() else {
        return Ok(());
    };
    let Some(tab_id) = data.get("tabId").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(tab) = sessions.tabs.get(tab_id) else {
        return Ok(());
    };
    if tab.session_id != session.id || tab.browser_id != browser.id {
        return Err("browser_session_handoff_attribution_mismatch".to_string());
    }
    let desired_url = data.get("url").and_then(Value::as_str).map(str::to_string);
    let candidate_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let projection = repository.mutate(|service| {
        let binding = manager_route_binding(service, &desktop.route_id, &desktop.display_name)?;
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

pub(crate) async fn resolve_manager_handoff(
    handoff: &RemoteViewHandoff,
    service: &ServiceState,
) -> Result<Option<Value>, String> {
    if !is_manager_handoff(handoff) {
        return Ok(None);
    }
    let sessions = super::browser_session_store::BrowserSessionJsonStore::default_json()?
        .load_session_state()?;
    let session_id = handoff
        .intent
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "browser_session_handoff_session_id_missing".to_string())?;
    let session = sessions
        .sessions
        .get(session_id)
        .ok_or_else(|| "browser_session_handoff_session_ended".to_string())?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default();
    if session.expires_at_ms < now_ms {
        return Err("browser_session_handoff_session_expired".to_string());
    }
    let browser = sessions
        .browsers
        .get(&session.browser_id)
        .ok_or_else(|| "browser_session_handoff_browser_missing".to_string())?;
    let tab_id = handoff
        .tab_id
        .as_deref()
        .ok_or_else(|| "browser_session_handoff_tab_missing".to_string())?;
    let tab = sessions
        .tabs
        .get(tab_id)
        .ok_or_else(|| "browser_session_handoff_tab_closed".to_string())?;
    if handoff.browser_id.as_deref() != Some(browser.id.as_str())
        || handoff.session_name.as_deref() != Some(session.name.as_str())
        || handoff.target_id.as_deref() != Some(tab.target_id.as_str())
        || tab.browser_id != browser.id
        || tab.session_id != session.id
    {
        return Err("browser_session_handoff_identity_changed".to_string());
    }
    if !matches!(
        crate::process_identity::observe_process(browser.pid),
        crate::process_identity::ProcessObservation::Observed(_)
    ) {
        return Err("browser_session_handoff_process_missing".to_string());
    }
    let desktop = browser
        .desktop
        .as_ref()
        .ok_or_else(|| "browser_session_handoff_desktop_missing".to_string())?;
    let binding = manager_route_binding(service, &desktop.route_id, &desktop.display_name)?;
    let mut manager = BrowserManager::connect_cdp(&browser.cdp_endpoint)
        .await
        .map_err(|error| format!("browser_session_handoff_attach_failed:{error}"))?;
    if !manager.is_connection_alive().await {
        return Err("browser_session_handoff_cdp_unresponsive".to_string());
    }
    manager.tab_switch_target_id(&tab.target_id).await?;
    manager.focus_for_view(true).await?;
    manager.relinquish_browser_for_handoff();
    Ok(Some(json!({
        "status": "ready",
        "resolved": true,
        "browserSessionManager": true,
        "handoffId": handoff.id,
        "handoffUrl": handoff.handoff_url,
        "browserId": browser.id,
        "sessionName": session.name,
        "tabId": tab.id,
        "targetId": tab.target_id,
        "viewStreamProvider": binding.provider,
        "requiredViewStreamProvider": binding.provider,
        "presentationGeneration": 1,
        "presentationReceipt": {
            "generation": 1,
            "logicalBrowserId": browser.id,
            "targetId": tab.target_id,
            "requiredStreamProvider": binding.provider,
            "observedStreamProvider": binding.provider,
            "state": "ready",
            "browserSessionManager": true,
        },
        "operatorVisible": { "state": "ready" },
    })))
}

fn is_manager_handoff(handoff: &RemoteViewHandoff) -> bool {
    handoff
        .intent
        .get("browserSessionManager")
        .and_then(Value::as_bool)
        == Some(true)
}

fn manager_route_binding(
    service: &ServiceState,
    route_pool_entry_id: &str,
    display_name: &str,
) -> Result<RemoteViewRouteBinding, String> {
    let entry = service
        .route_pool
        .get(route_pool_entry_id)
        .ok_or_else(|| "browser_session_handoff_route_pool_entry_missing".to_string())?;
    if !matches!(entry.state.as_str(), "available" | "checked_out")
        || entry
            .readiness
            .as_ref()
            .and_then(|value| value.get("state"))
            .and_then(Value::as_str)
            != Some("ready")
        || entry.target.get("displayName").and_then(Value::as_str) != Some(display_name)
    {
        return Err("browser_session_handoff_route_pool_entry_not_ready".to_string());
    }
    let route = service
        .remote_view_routes
        .get(&entry.route_id)
        .ok_or_else(|| "browser_session_handoff_route_missing".to_string())?;
    if route.state != "ready"
        || route
            .readiness
            .as_ref()
            .and_then(|value| value.get("state"))
            .and_then(Value::as_str)
            != Some("ready")
    {
        return Err("browser_session_handoff_route_not_ready".to_string());
    }
    let display_allocation_id = route
        .display_allocation_id
        .clone()
        .or_else(|| {
            entry
                .target
                .get("displayAllocationId")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .ok_or_else(|| "browser_session_handoff_display_allocation_missing".to_string())?;
    Ok(RemoteViewRouteBinding {
        route_id: route.id.clone(),
        route_pool_entry_id: Some(entry.id.clone()),
        display_allocation_id,
        route_pool_entry_state: Some(entry.state.clone()),
        current_route_allocation_id: entry.current_route_allocation_id.clone(),
        display_name: Some(display_name.to_string()),
        launch_display_name: Some(display_name.to_string()),
        display_isolation: "private_virtual_display".to_string(),
        route_user: None,
        display_access: None,
        provider: route.provider,
        provider_mode: route.provider_mode.clone(),
        connection_id: route
            .connection_id
            .clone()
            .or_else(|| entry.connection_id.clone()),
        connection_name: route
            .connection_name
            .clone()
            .or_else(|| entry.connection_name.clone()),
        frame_url: route.frame_url.clone().or_else(|| entry.frame_url.clone()),
        external_url: route
            .external_url
            .clone()
            .or_else(|| entry.external_url.clone()),
        route_descriptor: route
            .route_descriptor
            .clone()
            .or_else(|| entry.route_descriptor.clone()),
        readiness: route.readiness.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_store::JsonServiceStateStore;
    use agent_browser_service_model::{
        BrowserDesktopAssignment, ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab,
        RemoteViewRoute, RoutePoolEntry,
    };
    use std::collections::BTreeMap;

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
        service.remote_view_routes.insert(
            "route-a".to_string(),
            RemoteViewRoute {
                id: "route-a".to_string(),
                display_allocation_id: Some("display-a".to_string()),
                route_descriptor: Some(
                    json!({"publicOperatorUrl":"https://dashboard.example/operator"}),
                ),
                state: "ready".to_string(),
                readiness: Some(json!({"state":"ready"})),
                ..RemoteViewRoute::default()
            },
        );
        let binding = manager_route_binding(&service, "slot-a", ":10").unwrap();
        assert_eq!(
            durable_remote_view_handoff_url(&binding, "opaque-a").as_deref(),
            Some("https://dashboard.example/remote-view/opaque-a")
        );
        service.route_pool.get_mut("slot-a").unwrap().state = "stale".to_string();
        assert_eq!(
            manager_route_binding(&service, "slot-a", ":10").unwrap_err(),
            "browser_session_handoff_route_pool_entry_not_ready"
        );
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
                        state: "ready".to_string(),
                        readiness: Some(json!({"state":"ready"})),
                        ..RemoteViewRoute::default()
                    },
                );
                Ok(())
            })
            .unwrap();
        let sessions = BrowserSessionState {
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
        };
        let mut response = json!({"success":true,"data":{
            "sessionId":"session-a","browserId":"browser-a","tabId":"tab-a",
            "targetId":"target-a","url":"https://example.test/login"
        }});
        attach_manager_handoff_in_repository(&mut response, &sessions, &repository).unwrap();
        let first_id = response["data"]["handoffId"].as_str().unwrap().to_string();
        assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
        assert_eq!(
            response["data"]["handoffUrl"],
            format!("https://dashboard.example/remote-view/{first_id}")
        );
        response["data"]["url"] = json!("https://example.test/account");
        attach_manager_handoff_in_repository(&mut response, &sessions, &repository).unwrap();
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
}
