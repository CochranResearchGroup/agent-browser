//! Canonical list-shaped route inventory contract fixtures.

use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::presentation_capacity::{PresentationCapacityAuthority, PresentationCapacityConfig};
use super::service_model::{
    ControlInputProvider, DisplayAllocation, RemoteViewRoute, RoutePoolEntry, ServiceState,
    ViewStreamProvider,
};

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteCandidate {
    pub(crate) id: String,
}

#[cfg(test)]
impl RouteCandidate {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteInventory {
    candidates: Vec<RouteCandidate>,
}

#[cfg(test)]
impl RouteInventory {
    pub(crate) fn new(candidates: Vec<RouteCandidate>) -> Result<Self, String> {
        let identities = candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<BTreeSet<_>>();
        if identities.len() != candidates.len() || identities.contains("") {
            return Err("route_inventory_identity_invalid".to_string());
        }
        Ok(Self { candidates })
    }

    pub(crate) fn candidates(&self) -> &[RouteCandidate] {
        &self.candidates
    }
}

/// The sole Slice A contract seam for migrating legacy alphabetic route inputs.
pub(crate) struct LegacyTwoRouteAdapter;

impl LegacyTwoRouteAdapter {
    #[cfg(test)]
    pub(crate) fn adapt(
        route_a: Option<&str>,
        route_b: Option<&str>,
    ) -> Result<RouteInventory, String> {
        RouteInventory::new(
            [route_a, route_b]
                .into_iter()
                .flatten()
                .map(RouteCandidate::new)
                .collect(),
        )
    }

    fn subjects_from_environment() -> StaticRouteInventory {
        let route = |label: &str, default_user: &str| StaticRouteSubject {
            id: format!("legacy-route-{}", label.to_ascii_lowercase()),
            display_name: std::env::var(format!("AGENT_BROWSER_RDP_ROUTE_{label}_DISPLAY_NAME"))
                .ok()
                .filter(|value| !value.trim().is_empty()),
            route_user: std::env::var(format!("AGENT_BROWSER_RDP_ROUTE_{label}_USERNAME"))
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| Some(default_user.to_string())),
        };
        StaticRouteInventory {
            routes: vec![
                route("A", "agent-browser-rdp-a"),
                route("B", "agent-browser-rdp-b"),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticRouteSubject {
    pub(crate) id: String,
    pub(crate) display_name: Option<String>,
    pub(crate) route_user: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StaticRouteInventory {
    routes: Vec<StaticRouteSubject>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStaticRouteSubject {
    id: String,
    #[serde(default)]
    target: RawStaticRouteTarget,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStaticRouteTarget {
    display_name: Option<String>,
    route_user: Option<String>,
}

impl StaticRouteInventory {
    pub(crate) fn from_json(raw: &str) -> Result<Self, String> {
        let parsed = serde_json::from_str::<Vec<RawStaticRouteSubject>>(raw)
            .map_err(|error| format!("route_inventory_json_invalid:{error}"))?;
        let routes = parsed
            .into_iter()
            .map(|route| StaticRouteSubject {
                id: route.id.trim().to_string(),
                display_name: route
                    .target
                    .display_name
                    .filter(|value| !value.trim().is_empty()),
                route_user: route
                    .target
                    .route_user
                    .filter(|value| !value.trim().is_empty()),
            })
            .collect::<Vec<_>>();
        let identities = routes
            .iter()
            .map(|route| route.id.as_str())
            .collect::<BTreeSet<_>>();
        if identities.len() != routes.len() || identities.contains("") {
            return Err("route_inventory_identity_invalid".to_string());
        }
        Ok(Self { routes })
    }

    pub(crate) fn from_environment() -> Result<Self, String> {
        match std::env::var("AGENT_BROWSER_RDP_ROUTE_POOL_JSON") {
            Ok(raw) => Self::from_json(&raw),
            Err(std::env::VarError::NotPresent) => {
                Ok(LegacyTwoRouteAdapter::subjects_from_environment())
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                Err("route_inventory_environment_not_unicode".to_string())
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn routes(&self) -> &[StaticRouteSubject] {
        &self.routes
    }

    pub(crate) fn display_names(&self) -> impl Iterator<Item = &str> {
        self.routes
            .iter()
            .filter_map(|route| route.display_name.as_deref())
    }

    pub(crate) fn route_users(&self) -> impl Iterator<Item = &str> {
        self.routes
            .iter()
            .filter_map(|route| route.route_user.as_deref())
    }
}

const PRESENTATION_PROVIDER_INVENTORY_SCHEMA: &str =
    "agent-browser.development-presentation-inventory.v1";

/// A provider-owned readiness inventory projected into Service authority.
/// The adapter is opt-in through an exact file path and accepts development
/// inventories only, keeping production state isolated by default.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationProviderInventory {
    schema_version: String,
    environment: String,
    routes: Vec<PresentationProviderRoute>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresentationProviderRoute {
    route_id: String,
    slot_id: String,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    connection_id: Option<String>,
    #[serde(default)]
    connection_name: Option<String>,
    display_reservation_id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    frame_url: Option<String>,
    #[serde(default)]
    lifecycle: Option<String>,
    state: String,
}

impl PresentationProviderInventory {
    pub(crate) fn from_json(raw: &str) -> Result<Self, String> {
        let inventory = serde_json::from_str::<Self>(raw)
            .map_err(|error| format!("presentation_provider_inventory_json_invalid:{error}"))?;
        if inventory.schema_version != PRESENTATION_PROVIDER_INVENTORY_SCHEMA {
            return Err("presentation_provider_inventory_schema_invalid".to_string());
        }
        if inventory.environment != "development" {
            return Err("presentation_provider_inventory_environment_invalid".to_string());
        }
        validate_provider_identities(
            inventory.routes.iter().map(|route| route.route_id.as_str()),
            "route",
        )?;
        validate_provider_identities(
            inventory.routes.iter().map(|route| route.slot_id.as_str()),
            "slot",
        )?;
        validate_provider_identities(
            inventory
                .routes
                .iter()
                .map(|route| route.display_reservation_id.as_str()),
            "display",
        )?;
        Ok(inventory)
    }

    pub(crate) fn from_path(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path).map_err(|error| {
            format!(
                "presentation_provider_inventory_read_failed:{}:{error}",
                path.display()
            )
        })?;
        Self::from_json(&raw)
    }

    pub(crate) fn overlay_service_state(
        &self,
        state: &mut ServiceState,
        config: PresentationCapacityConfig,
    ) -> Result<(), String> {
        let live_browser_ids = state.browsers.keys().cloned().collect::<BTreeSet<_>>();
        let checked_out_routes = state
            .remote_view_routes
            .iter()
            .filter(|(_, route)| {
                route.state == "ready"
                    && route
                        .browser_id
                        .as_ref()
                        .is_some_and(|browser_id| live_browser_ids.contains(browser_id))
            })
            .map(|(id, route)| (id.clone(), route.clone()))
            .collect::<BTreeMap<_, _>>();
        let checked_out_displays = state
            .display_allocations
            .iter()
            .filter(|(_, display)| {
                display
                    .owner_browser_id
                    .as_ref()
                    .is_some_and(|browser_id| live_browser_ids.contains(browser_id))
            })
            .map(|(id, display)| (id.clone(), display.clone()))
            .collect::<BTreeMap<_, _>>();
        let checked_out_slots = state
            .route_pool
            .iter()
            .filter(|(_, entry)| {
                entry.state == "checked_out" && checked_out_routes.contains_key(&entry.route_id)
            })
            .map(|(id, entry)| (id.clone(), entry.clone()))
            .collect::<BTreeMap<_, _>>();
        state
            .remote_view_routes
            .retain(|_, route| route.route_source != "provider_inventory");
        state
            .display_allocations
            .retain(|_, display| !has_provider_inventory_readiness(display.readiness.as_ref()));
        state
            .route_pool
            .retain(|_, entry| !has_provider_inventory_readiness(entry.readiness.as_ref()));
        for provider_route in self.routes.iter().filter(|route| route.state == "ready") {
            let display_name = provider_route.display_name.clone().ok_or_else(|| {
                format!(
                    "presentation_provider_ready_display_missing:{}",
                    provider_route.route_id
                )
            })?;
            let route_id = provider_route.route_id.clone();
            let display_id = provider_route.display_reservation_id.clone();
            let mut display = DisplayAllocation {
                id: display_id.clone(),
                boot_epoch: crate::process_identity::current_boot_epoch(),
                display_name: Some(display_name),
                display_isolation: "private_virtual_display".to_string(),
                state: "ready".to_string(),
                route_ids: vec![route_id.clone()],
                readiness: Some(json!({"state":"ready","source":"provider_inventory"})),
                ..DisplayAllocation::default()
            };
            if let Some(existing) = checked_out_displays.get(&display_id) {
                display.owner_browser_id = existing.owner_browser_id.clone();
                display.owner_session_id = existing.owner_session_id.clone();
                display.profile_id = existing.profile_id.clone();
                display.browser_build = existing.browser_build.clone();
                display.pid_hints = existing.pid_hints.clone();
                display.created_at = existing.created_at.clone();
                display.updated_at = existing.updated_at.clone();
                display.last_health_check_at = existing.last_health_check_at.clone();
                display.readiness = existing.readiness.clone();
                for existing_route_id in &existing.route_ids {
                    if !display.route_ids.contains(existing_route_id) {
                        display.route_ids.push(existing_route_id.clone());
                    }
                }
            }
            state
                .display_allocations
                .insert(display_id.clone(), display);
            let mut route = RemoteViewRoute {
                id: route_id.clone(),
                provider: ViewStreamProvider::RdpGateway,
                display_allocation_id: Some(display_id),
                route_source: "provider_inventory".to_string(),
                connection_id: provider_route.connection_id.clone(),
                connection_name: provider_route.connection_name.clone(),
                frame_url: provider_route.frame_url.clone(),
                control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                provider_mode: "simultaneous_view".to_string(),
                state: "ready".to_string(),
                readiness: Some(json!({"state":"ready","source":"provider_inventory"})),
                ..RemoteViewRoute::default()
            };
            if let Some(existing) = checked_out_routes.get(&route_id) {
                route.browser_id = existing.browser_id.clone();
                route.session_id = existing.session_id.clone();
                route.route_source = existing.route_source.clone();
                route.viewer_lease_ids = existing.viewer_lease_ids.clone();
                route.controller_lease_id = existing.controller_lease_id.clone();
                route.controller_epoch = existing.controller_epoch;
                route.last_provider_event = existing.last_provider_event.clone();
                route.readiness = existing.readiness.clone();
            }
            state.remote_view_routes.insert(route_id.clone(), route);
            let mut slot = RoutePoolEntry {
                id: provider_route.slot_id.clone(),
                provider: ViewStreamProvider::RdpGateway,
                route_id: route_id.clone(),
                connection_id: provider_route.connection_id.clone(),
                connection_name: provider_route.connection_name.clone(),
                frame_url: provider_route.frame_url.clone(),
                target: json!({
                    "displayAllocationId": provider_route.display_reservation_id.clone(),
                    "displayName": state.display_allocations
                        .get(&provider_route.display_reservation_id)
                        .and_then(|display| display.display_name.clone()),
                    "displayIsolation": "private_virtual_display",
                    "routeUser": provider_route.user,
                    "lifecycle": provider_route.lifecycle,
                }),
                provider_mode: "simultaneous_view".to_string(),
                state: "available".to_string(),
                current_route_allocation_id: Some(route_id),
                readiness: Some(json!({"state":"ready","source":"provider_inventory"})),
                ..RoutePoolEntry::default()
            };
            if let Some(existing) = checked_out_slots.get(&provider_route.slot_id) {
                slot.state = existing.state.clone();
                slot.current_route_allocation_id = existing.current_route_allocation_id.clone();
                slot.readiness = existing.readiness.clone();
            }
            state
                .route_pool
                .insert(provider_route.slot_id.clone(), slot);
        }
        let previous = state.presentation_capacity.take();
        let mut capacity = PresentationCapacityAuthority::from_service_state(config, state)?;
        if let Some(previous) = previous {
            capacity.queued_requests = previous.queued_requests;
            capacity.queue_clock = previous.queue_clock;
            for slot in &mut capacity.slots {
                if let Some(previous_slot) = previous.slots.iter().find(|candidate| {
                    candidate.id == slot.id
                        && candidate.route_id == slot.route_id
                        && candidate.display_allocation_id == slot.display_allocation_id
                }) {
                    *slot = previous_slot.clone();
                }
            }
        }
        state.presentation_capacity = Some(capacity);
        Ok(())
    }
}

fn has_provider_inventory_readiness(readiness: Option<&serde_json::Value>) -> bool {
    readiness
        .and_then(|value| value.get("source"))
        .and_then(serde_json::Value::as_str)
        == Some("provider_inventory")
}

fn validate_provider_identities<'a>(
    values: impl Iterator<Item = &'a str>,
    field: &str,
) -> Result<(), String> {
    let values = values.collect::<Vec<_>>();
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(format!(
            "presentation_provider_inventory_{field}_identity_invalid"
        ));
    }
    if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(format!(
            "presentation_provider_inventory_{field}_identity_duplicate"
        ));
    }
    Ok(())
}

pub(crate) fn overlay_provider_inventory_from_environment(
    state: &mut ServiceState,
) -> Result<(), String> {
    let path = match std::env::var("AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH") {
        Ok(path) if !path.trim().is_empty() => path,
        Ok(_) | Err(std::env::VarError::NotPresent) => return Ok(()),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err("presentation_provider_inventory_path_not_unicode".to_string())
        }
    };
    let usize_env = |name: &str, fallback: usize| -> Result<usize, String> {
        match std::env::var(name) {
            Ok(value) => value
                .parse::<usize>()
                .map_err(|_| format!("{name}_invalid")),
            Err(std::env::VarError::NotPresent) => Ok(fallback),
            Err(std::env::VarError::NotUnicode(_)) => Err(format!("{name}_not_unicode")),
        }
    };
    let config = PresentationCapacityConfig {
        warm_minimum: usize_env("AGENT_BROWSER_PRESENTATION_WARM_MINIMUM", 4)?,
        hard_maximum: usize_env("AGENT_BROWSER_PRESENTATION_HARD_MAXIMUM", 6)?,
        human_priority_reserve: usize_env("AGENT_BROWSER_PRESENTATION_HUMAN_RESERVE", 1)?,
        recovery_reserve: usize_env("AGENT_BROWSER_PRESENTATION_RECOVERY_RESERVE", 1)?,
        max_queue_depth: usize_env("AGENT_BROWSER_PRESENTATION_MAX_QUEUE_DEPTH", 64)?,
    };
    PresentationProviderInventory::from_path(Path::new(&path))?.overlay_service_state(state, config)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeRouteEnvironment {
    pub(crate) route_pool_json: String,
    pub(crate) remote_view_url: String,
    pub(crate) legacy_display_a: Option<String>,
    pub(crate) legacy_display_b: Option<String>,
}

impl RuntimeRouteEnvironment {
    /// Projects the first two displays for older installations while the
    /// canonical route inventory remains list-shaped and preserves every route.
    pub(crate) fn legacy_display_env_values(&self) -> Result<Vec<(&'static str, &str)>, String> {
        let display_a = self
            .legacy_display_a
            .as_deref()
            .ok_or_else(|| "Canonical first route is missing a display".to_string())?;
        let display_b = self
            .legacy_display_b
            .as_deref()
            .ok_or_else(|| "Canonical second route is missing a display".to_string())?;
        Ok(vec![
            ("AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME", display_a),
            ("AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME", display_b),
        ])
    }
}

pub(crate) fn runtime_route_environment(
    route_pool: &[serde_json::Value],
) -> Result<RuntimeRouteEnvironment, String> {
    if route_pool.len() < 2 {
        return Err("Canonical route inventory requires at least two entries".to_string());
    }
    let route_pool_json = serde_json::to_string(route_pool)
        .map_err(|error| format!("Canonical route inventory serialization failed: {error}"))?;
    let inventory = StaticRouteInventory::from_json(&route_pool_json)?;
    if let Some(route) = inventory
        .routes
        .iter()
        .find(|route| route.display_name.is_none())
    {
        return Err(format!("Canonical route {} is missing a display", route.id));
    }
    let remote_view_url = route_pool[0]
        .pointer("/routeDescriptor/localEmbedUrl")
        .or_else(|| route_pool[0].get("frameUrl"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Canonical first route is missing a local operator URL".to_string())?
        .to_string();
    let display = |index: usize| {
        route_pool[index]
            .pointer("/target/displayName")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    };
    Ok(RuntimeRouteEnvironment {
        route_pool_json,
        remote_view_url,
        legacy_display_a: display(0),
        legacy_display_b: display(1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_route_inventory_preserves_arbitrary_n() {
        for count in [0, 1, 2, 4, 6, 8] {
            let inventory = RouteInventory::new(
                (0..count)
                    .map(|index| RouteCandidate::new(format!("route-{index}")))
                    .collect(),
            )
            .expect("fixture route identities should be valid");

            assert_eq!(inventory.candidates().len(), count);
            assert_eq!(
                inventory.candidates().last().map(|route| route.id.as_str()),
                (count > 0)
                    .then(|| format!("route-{}", count - 1))
                    .as_deref()
            );
        }
    }

    #[test]
    fn legacy_a_b_configuration_enters_through_one_compatibility_adapter() {
        let inventory = LegacyTwoRouteAdapter::adapt(Some("route-a"), Some("route-b"))
            .expect("legacy two-route configuration should remain compatible");

        assert_eq!(
            inventory
                .candidates()
                .iter()
                .map(|route| route.id.as_str())
                .collect::<Vec<_>>(),
            vec!["route-a", "route-b"]
        );
    }

    #[test]
    fn configured_subjects_preserve_six_route_users_and_displays() {
        let raw = serde_json::json!([
            {"id":"route-1","target":{"displayName":":21","routeUser":"rdp-1"}},
            {"id":"route-2","target":{"displayName":":22","routeUser":"rdp-2"}},
            {"id":"route-3","target":{"displayName":":23","routeUser":"rdp-3"}},
            {"id":"route-4","target":{"displayName":":24","routeUser":"rdp-4"}},
            {"id":"route-5","target":{"displayName":":25","routeUser":"rdp-5"}},
            {"id":"route-6","target":{"displayName":":26","routeUser":"rdp-6"}}
        ])
        .to_string();

        let subjects = StaticRouteInventory::from_json(&raw).unwrap();

        assert_eq!(subjects.routes().len(), 6);
        assert_eq!(subjects.routes()[5].display_name.as_deref(), Some(":26"));
        assert_eq!(subjects.routes()[5].route_user.as_deref(), Some("rdp-6"));
    }

    #[test]
    fn runtime_environment_keeps_full_inventory_and_projects_legacy_pair() {
        let routes = serde_json::json!([
            {"id":"route-1","frameUrl":"http://local/1","target":{"displayName":":21"}},
            {"id":"route-2","frameUrl":"http://local/2","target":{"displayName":":22"}},
            {"id":"route-3","frameUrl":"http://local/3","target":{"displayName":":23"}},
            {"id":"route-4","frameUrl":"http://local/4","target":{"displayName":":24"}}
        ]);

        let environment = runtime_route_environment(routes.as_array().unwrap()).unwrap();

        assert_eq!(
            serde_json::from_str::<Vec<serde_json::Value>>(&environment.route_pool_json)
                .unwrap()
                .len(),
            4
        );
        assert_eq!(environment.remote_view_url, "http://local/1");
        assert_eq!(environment.legacy_display_a.as_deref(), Some(":21"));
        assert_eq!(environment.legacy_display_b.as_deref(), Some(":22"));
        assert_eq!(environment.legacy_display_env_values().unwrap().len(), 2);
    }

    #[test]
    fn provider_inventory_projects_only_ready_slots_into_service_authority() {
        let raw = serde_json::json!({
            "schemaVersion": "agent-browser.development-presentation-inventory.v1",
            "environment": "development",
            "routes": [
                {
                    "routeId": "development-route-1",
                    "slotId": "development-slot-1",
                    "user": "agent-browser-rdp-dev-1",
                    "connectionId": "41",
                    "connectionName": "Agent Browser Dev RDP Route 1",
                    "displayReservationId": "development-display-1",
                    "displayName": ":21",
                    "lifecycle": "warm",
                    "state": "ready"
                },
                {
                    "routeId": "development-route-2",
                    "slotId": "development-slot-2",
                    "user": "agent-browser-rdp-dev-2",
                    "connectionId": "42",
                    "connectionName": "Agent Browser Dev RDP Route 2",
                    "displayReservationId": "development-display-2",
                    "displayName": null,
                    "lifecycle": "elastic",
                    "state": "absent"
                }
            ]
        })
        .to_string();
        let inventory = PresentationProviderInventory::from_json(&raw).unwrap();
        let mut state = crate::native::service_model::ServiceState::default();

        inventory
            .overlay_service_state(
                &mut state,
                crate::native::presentation_capacity::PresentationCapacityConfig {
                    warm_minimum: 1,
                    hard_maximum: 2,
                    human_priority_reserve: 1,
                    recovery_reserve: 1,
                    max_queue_depth: 64,
                },
            )
            .unwrap();

        assert_eq!(state.display_allocations.len(), 1);
        assert_eq!(state.remote_view_routes.len(), 1);
        assert_eq!(state.route_pool.len(), 1);
        let route_pool_entry = state.route_pool.get("development-slot-1").unwrap();
        assert_eq!(
            route_pool_entry.target["displayAllocationId"],
            "development-display-1"
        );
        assert_eq!(
            route_pool_entry.target["displayIsolation"],
            "private_virtual_display"
        );
        assert_eq!(
            state.display_allocations["development-display-1"].display_isolation,
            "private_virtual_display"
        );
        let capacity = state.presentation_capacity.as_ref().unwrap();
        assert_eq!(capacity.slots.len(), 1);
        assert_eq!(capacity.config.hard_maximum, 2);
        assert_eq!(capacity.slots[0].id, "slot:development-slot-1");
    }

    #[test]
    fn provider_inventory_refresh_preserves_checked_out_runtime_ownership() {
        let inventory = PresentationProviderInventory::from_json(
            &serde_json::json!({
                "schemaVersion": "agent-browser.development-presentation-inventory.v1",
                "environment": "development",
                "routes": [{
                    "routeId": "development-route-1",
                    "slotId": "development-slot-1",
                    "user": "agent-browser-rdp-dev-1",
                    "connectionId": "41",
                    "connectionName": "Agent Browser Dev RDP Route 1",
                    "displayReservationId": "development-display-1",
                    "displayName": ":21",
                    "lifecycle": "warm",
                    "state": "ready"
                }]
            })
            .to_string(),
        )
        .unwrap();
        let config = PresentationCapacityConfig {
            warm_minimum: 1,
            hard_maximum: 2,
            human_priority_reserve: 1,
            recovery_reserve: 1,
            max_queue_depth: 64,
        };
        let mut state = ServiceState::default();
        inventory
            .overlay_service_state(&mut state, config.clone())
            .unwrap();
        state.browsers.insert(
            "browser-1".to_string(),
            crate::native::service_model::BrowserProcess {
                id: "browser-1".to_string(),
                ..crate::native::service_model::BrowserProcess::default()
            },
        );
        let route = state
            .remote_view_routes
            .get_mut("development-route-1")
            .unwrap();
        route.browser_id = Some("browser-1".to_string());
        route.session_id = Some("scene-1".to_string());
        route.route_source = "pool".to_string();
        route.last_provider_event = Some("route_checked_out".to_string());
        let display = state
            .display_allocations
            .get_mut("development-display-1")
            .unwrap();
        display.owner_browser_id = Some("browser-1".to_string());
        display.owner_session_id = Some("scene-1".to_string());
        state
            .route_pool
            .get_mut("development-slot-1")
            .unwrap()
            .state = "checked_out".to_string();

        inventory
            .overlay_service_state(&mut state, config.clone())
            .unwrap();

        assert_eq!(
            state.remote_view_routes["development-route-1"]
                .browser_id
                .as_deref(),
            Some("browser-1")
        );
        assert_eq!(
            state.remote_view_routes["development-route-1"]
                .session_id
                .as_deref(),
            Some("scene-1")
        );
        assert_eq!(
            state.display_allocations["development-display-1"]
                .owner_session_id
                .as_deref(),
            Some("scene-1")
        );
        assert_eq!(state.route_pool["development-slot-1"].state, "checked_out");

        state.browsers.remove("browser-1");
        state
            .remote_view_routes
            .get_mut("development-route-1")
            .unwrap()
            .state = "orphaned".to_string();
        inventory.overlay_service_state(&mut state, config).unwrap();

        assert_eq!(
            state.remote_view_routes["development-route-1"].browser_id,
            None
        );
        assert_eq!(
            state.display_allocations["development-display-1"].owner_browser_id,
            None
        );
        assert_eq!(state.route_pool["development-slot-1"].state, "available");
    }

    #[test]
    fn provider_inventory_rejects_environment_and_identity_drift() {
        let wrong_environment = serde_json::json!({
            "schemaVersion": "agent-browser.development-presentation-inventory.v1",
            "environment": "production",
            "routes": []
        })
        .to_string();
        assert_eq!(
            PresentationProviderInventory::from_json(&wrong_environment).unwrap_err(),
            "presentation_provider_inventory_environment_invalid"
        );

        let duplicate = serde_json::json!({
            "schemaVersion": "agent-browser.development-presentation-inventory.v1",
            "environment": "development",
            "routes": [
                {"routeId":"route-1","slotId":"slot-1","displayReservationId":"display-1","displayName":":21","state":"ready"},
                {"routeId":"route-1","slotId":"slot-2","displayReservationId":"display-2","displayName":":22","state":"ready"}
            ]
        })
        .to_string();
        assert_eq!(
            PresentationProviderInventory::from_json(&duplicate).unwrap_err(),
            "presentation_provider_inventory_route_identity_duplicate"
        );
    }
}
