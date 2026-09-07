//! Production capacity admission from existing provider and Service identities.
//! This adapter never rewrites routes or promotes presentation readiness.

use super::*;
use crate::native::presentation_capacity::{PresentationSlot, PresentationSlotState};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProductionInventory {
    schema_version: String,
    environment: String,
    boot_epoch: String,
    routes: Vec<ProductionRoute>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductionRoute {
    route_pool_entry_id: String,
    route_id: String,
    display_allocation_id: String,
    display_name: String,
    route_user: String,
    connection_id: String,
}

impl ProductionInventory {
    pub(super) fn apply(
        path: &Path,
        state: &mut ServiceState,
        config: PresentationCapacityConfig,
    ) -> Result<(), String> {
        if !path.is_absolute() {
            return Err("production_presentation_inventory_absolute_path_required".into());
        }
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let inventory: Self = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let environment = match std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => "production".into(),
            Err(_) => return Err("production_presentation_inventory_environment_invalid".into()),
        };
        let boot = crate::process_identity::current_boot_epoch().unwrap_or_default();
        let capacity =
            inventory.qualify(state, config, &environment, &boot, |route, browser| {
                let display = crate::native::remote_view::display_owner::route_display_owner(
                    Some(&route.display_name),
                    Some(&route.route_user),
                );
                if display["verified"] != true {
                    return false;
                }
                browser.is_none_or(|browser| {
                    state
                        .browser_process_identities
                        .get(&browser.id)
                        .is_some_and(|record| {
                            browser.pid == Some(record.process_identity.pid)
                                && crate::process_identity::recorded_process_is_running(
                                    &record.process_identity,
                                ) == Ok(true)
                        })
                })
            })?;
        state.presentation_capacity = Some(capacity);
        Ok(())
    }

    fn qualify(
        &self,
        state: &ServiceState,
        config: PresentationCapacityConfig,
        environment: &str,
        boot: &str,
        observe: impl Fn(
            &ProductionRoute,
            Option<&crate::native::service_model::BrowserProcess>,
        ) -> bool,
    ) -> Result<PresentationCapacityAuthority, String> {
        if self.schema_version != "agent-browser.production-presentation-inventory.v1"
            || self.environment != "production"
            || environment != "production"
            || boot.is_empty()
            || self.boot_epoch != boot
        {
            return Err("production_presentation_inventory_scope_mismatch".into());
        }
        if self.routes.is_empty() {
            return Err("production_presentation_inventory_empty".into());
        }
        validate_provider_identities(self.routes.iter().map(|r| r.route_id.as_str()), "route")?;
        validate_provider_identities(
            self.routes.iter().map(|r| r.route_pool_entry_id.as_str()),
            "slot",
        )?;
        validate_provider_identities(
            self.routes.iter().map(|r| r.display_allocation_id.as_str()),
            "display",
        )?;
        let mut slots = Vec::new();
        for expected in &self.routes {
            let invalid = || {
                format!(
                    "production_presentation_inventory_binding_mismatch:{}",
                    expected.route_id
                )
            };
            let entry = state
                .route_pool
                .get(&expected.route_pool_entry_id)
                .ok_or_else(invalid)?;
            let route = state
                .remote_view_routes
                .get(&expected.route_id)
                .ok_or_else(invalid)?;
            let display = state
                .display_allocations
                .get(&expected.display_allocation_id)
                .ok_or_else(invalid)?;
            if entry.provider != ViewStreamProvider::RdpGateway
                || route.provider != entry.provider
                || entry.id != expected.route_pool_entry_id
                || display.id != expected.display_allocation_id
                || entry.route_id != route.id
                || entry.connection_id.as_deref() != Some(&expected.connection_id)
                || route.connection_id != entry.connection_id
                || route.display_allocation_id.as_deref() != Some(&expected.display_allocation_id)
                || display.display_name.as_deref() != Some(&expected.display_name)
                || display.boot_epoch.as_deref() != Some(boot)
                || entry.target["displayName"] != expected.display_name
                || entry.target["routeUser"] != expected.route_user
                || expected.route_user.trim().is_empty()
                || !display.route_ids.contains(&route.id)
                || route.browser_id != display.owner_browser_id
                || route.session_id != display.owner_session_id
                || !matches!(route.state.as_str(), "ready" | "orphaned" | "checked_out")
                || !matches!(display.state.as_str(), "ready" | "active" | "orphaned")
                || !matches!(entry.state.as_str(), "available" | "checked_out")
                || entry
                    .current_route_allocation_id
                    .as_ref()
                    .is_some_and(|id| id != &route.id)
            {
                return Err(invalid());
            }
            let browser = match route.browser_id.as_ref() {
                Some(id) => {
                    let browser = state.browsers.get(id).ok_or_else(invalid)?;
                    if browser.display_allocation_id != route.display_allocation_id
                        || browser.display_name != display.display_name
                        || browser.boot_epoch.as_deref() != Some(boot)
                        || route
                            .session_id
                            .as_ref()
                            .is_none_or(|session| !browser.active_session_ids.contains(session))
                    {
                        return Err(invalid());
                    }
                    Some(browser)
                }
                None if route.session_id.is_none()
                    && route.state == "ready"
                    && display.state == "ready" =>
                {
                    None
                }
                None => return Err(invalid()),
            };
            if !observe(expected, browser) {
                return Err(format!(
                    "production_presentation_inventory_owner_unproven:{}",
                    route.id
                ));
            }
            let mut slot = PresentationSlot::warm_idle(format!("slot:{}", entry.id))
                .with_binding(route.id.clone(), display.id.clone());
            slot.browser_id = route.browser_id.clone();
            if browser.is_some() {
                slot.state = PresentationSlotState::Active;
            }
            slots.push(slot);
        }
        slots.sort_by(|a, b| a.id.cmp(&b.id));
        if let Some(previous) = state.presentation_capacity.as_ref() {
            for old in &previous.slots {
                let Some(slot) = slots.iter_mut().find(|s| {
                    s.id == old.id
                        && s.route_id == old.route_id
                        && s.display_allocation_id == old.display_allocation_id
                        && s.browser_id == old.browser_id
                }) else {
                    return Err("production_presentation_inventory_capacity_custody_changed".into());
                };
                *slot = old.clone();
            }
        }
        let mut capacity = PresentationCapacityAuthority::new(config, slots)?;
        if let Some(previous) = state.presentation_capacity.as_ref() {
            capacity.queued_requests = previous.queued_requests.clone();
            capacity.queue_clock = previous.queue_clock;
        }
        Ok(capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::presentation_capacity::{PresentationRequest, PressureAdmission};
    use crate::native::service_model::BrowserProcess;

    fn fixture() -> (
        ProductionInventory,
        ServiceState,
        PresentationCapacityConfig,
    ) {
        let inventory = serde_json::from_value(json!({
            "schemaVersion":"agent-browser.production-presentation-inventory.v1",
            "environment":"production", "bootEpoch":"boot-test",
            "routes":[{"routePoolEntryId":"pool", "routeId":"route",
                "displayAllocationId":"display", "displayName":":14",
                "routeUser":"route-user", "connectionId":"3"}]
        }))
        .unwrap();
        let mut state = ServiceState::default();
        state.browsers.insert(
            "browser".into(),
            BrowserProcess {
                id: "browser".into(),
                boot_epoch: Some("boot-test".into()),
                display_name: Some(":14".into()),
                display_allocation_id: Some("display".into()),
                active_session_ids: vec!["session".into()],
                ..Default::default()
            },
        );
        state.display_allocations.insert(
            "display".into(),
            DisplayAllocation {
                id: "display".into(),
                boot_epoch: Some("boot-test".into()),
                display_name: Some(":14".into()),
                owner_browser_id: Some("browser".into()),
                owner_session_id: Some("session".into()),
                state: "orphaned".into(),
                route_ids: vec!["route".into()],
                ..Default::default()
            },
        );
        state.remote_view_routes.insert(
            "route".into(),
            RemoteViewRoute {
                id: "route".into(),
                provider: ViewStreamProvider::RdpGateway,
                display_allocation_id: Some("display".into()),
                browser_id: Some("browser".into()),
                session_id: Some("session".into()),
                connection_id: Some("3".into()),
                state: "orphaned".into(),
                ..Default::default()
            },
        );
        state.route_pool.insert(
            "pool".into(),
            RoutePoolEntry {
                id: "pool".into(),
                provider: ViewStreamProvider::RdpGateway,
                route_id: "route".into(),
                connection_id: Some("3".into()),
                target: json!({"displayName":":14","routeUser":"route-user"}),
                state: "available".into(),
                ..Default::default()
            },
        );
        let config = PresentationCapacityConfig {
            warm_minimum: 1,
            hard_maximum: 2,
            human_priority_reserve: 1,
            recovery_reserve: 1,
            max_queue_depth: 8,
        };
        (inventory, state, config)
    }

    #[test]
    fn production_recovery_preserves_orphaned_custody_and_active_reservations() {
        let (inventory, mut state, config) = fixture();
        let before = serde_json::to_value(&state).unwrap();
        let mut capacity = inventory
            .qualify(&state, config, "production", "boot-test", |_, _| true)
            .unwrap();
        assert_eq!(serde_json::to_value(&state).unwrap(), before);
        assert_eq!(capacity.reconcile_authoritative_bindings(&state), 0);
        assert_eq!(capacity.slots[0].browser_id.as_deref(), Some("browser"));
        assert!(!capacity
            .clone()
            .request_bound_recovery(
                PresentationRequest::recovery("foreign").for_browser("other-browser"),
                PressureAdmission::admit(2),
                &state,
                "route",
                "display"
            )
            .is_granted());
        let mut released_state = state.clone();
        released_state
            .remote_view_routes
            .get_mut("route")
            .unwrap()
            .state = "released".into();
        let mut released_capacity = capacity.clone();
        assert_eq!(
            released_capacity.reconcile_authoritative_bindings(&released_state),
            1
        );
        assert_eq!(
            released_capacity.slots[0].state,
            PresentationSlotState::WarmIdle
        );
        assert!(released_capacity.slots[0].browser_id.is_none());
        assert!(capacity
            .request_bound_recovery(
                PresentationRequest::recovery("recover").for_browser("browser"),
                PressureAdmission::admit(2),
                &state,
                "route",
                "display"
            )
            .is_granted());
        state.presentation_capacity = Some(capacity.clone());
        let refreshed = inventory
            .qualify(&state, config, "production", "boot-test", |_, _| true)
            .unwrap();
        assert_eq!(refreshed.slots, capacity.slots);
        assert_eq!(state.remote_view_routes["route"].state, "orphaned");
        assert_eq!(state.display_allocations["display"].state, "orphaned");
    }

    #[test]
    fn production_inventory_denies_scope_owner_drift_and_human_controller() {
        let (inventory, mut state, config) = fixture();
        assert!(inventory
            .qualify(&state, config, "development", "boot-test", |_, _| true)
            .is_err());
        assert!(inventory
            .qualify(&state, config, "production", "other-boot", |_, _| true)
            .is_err());
        assert!(inventory
            .qualify(&state, config, "production", "boot-test", |_, _| { false })
            .is_err());
        state
            .remote_view_routes
            .get_mut("route")
            .unwrap()
            .controller_lease_id = Some("human".into());
        let mut capacity = inventory
            .qualify(&state, config, "production", "boot-test", |_, _| true)
            .unwrap();
        assert!(!capacity
            .request_bound_recovery(
                PresentationRequest::recovery("recover").for_browser("browser"),
                PressureAdmission::admit(2),
                &state,
                "route",
                "display"
            )
            .is_granted());
        state
            .display_allocations
            .get_mut("display")
            .unwrap()
            .owner_browser_id = Some("foreign".into());
        assert!(inventory
            .qualify(&state, config, "production", "boot-test", |_, _| true)
            .is_err());
    }
}
