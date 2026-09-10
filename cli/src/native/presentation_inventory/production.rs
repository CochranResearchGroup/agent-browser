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
        let raw = fs::read_to_string(path).map_err(|e| {
            format!(
                "production_presentation_inventory_read_failed:{}:{e}",
                path.display()
            )
        })?;
        let inventory: Self = serde_json::from_str(&raw)
            .map_err(|e| format!("production_presentation_inventory_json_invalid:{e}"))?;
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
            let invalid = |reason: &str| {
                format!(
                    "production_presentation_inventory_binding_mismatch:{}:{}",
                    expected.route_id, reason
                )
            };
            let entry = state
                .route_pool
                .get(&expected.route_pool_entry_id)
                .ok_or_else(|| invalid("route_pool_entry_missing"))?;
            let route = state
                .remote_view_routes
                .get(&expected.route_id)
                .ok_or_else(|| invalid("route_missing"))?;
            let display = state
                .display_allocations
                .get(&expected.display_allocation_id)
                .ok_or_else(|| invalid("display_allocation_missing"))?;
            let pending_acquisition = has_current_acquisition_binding(state, route)
                || has_current_acquisition_custody(state, route);
            let binding_mismatch = [
                (
                    "route_pool_provider",
                    entry.provider != ViewStreamProvider::RdpGateway,
                ),
                ("route_provider", route.provider != entry.provider),
                (
                    "route_pool_entry_id",
                    entry.id != expected.route_pool_entry_id,
                ),
                (
                    "display_allocation_id",
                    display.id != expected.display_allocation_id,
                ),
                ("route_pool_route_id", entry.route_id != route.id),
                (
                    "route_pool_connection_id",
                    entry.connection_id.as_deref() != Some(&expected.connection_id),
                ),
                (
                    "route_connection_id",
                    route.connection_id != entry.connection_id,
                ),
                (
                    "route_display_allocation_id",
                    route.display_allocation_id.as_deref() != Some(&expected.display_allocation_id),
                ),
                (
                    "display_name",
                    display.display_name.as_deref() != Some(&expected.display_name),
                ),
                (
                    "display_boot_epoch",
                    display.boot_epoch.as_deref() != Some(boot),
                ),
                (
                    "route_pool_display_name",
                    entry.target["displayName"] != expected.display_name,
                ),
                (
                    "route_pool_route_user",
                    entry.target["routeUser"] != expected.route_user,
                ),
                (
                    "inventory_route_user_empty",
                    expected.route_user.trim().is_empty(),
                ),
                ("display_route_ids", !display.route_ids.contains(&route.id)),
                (
                    "route_display_browser_owner",
                    route.browser_id != display.owner_browser_id,
                ),
                (
                    "route_display_session_owner",
                    route.session_id != display.owner_session_id,
                ),
                (
                    "route_state",
                    !matches!(route.state.as_str(), "ready" | "orphaned" | "checked_out")
                        && !pending_acquisition,
                ),
                (
                    "display_state",
                    !matches!(display.state.as_str(), "ready" | "active" | "orphaned")
                        && !pending_acquisition,
                ),
                (
                    "route_pool_state",
                    !matches!(entry.state.as_str(), "available" | "checked_out")
                        && !pending_acquisition,
                ),
                (
                    "route_pool_current_allocation",
                    entry
                        .current_route_allocation_id
                        .as_ref()
                        .is_some_and(|id| id != &route.id),
                ),
            ]
            .into_iter()
            .find_map(|(reason, mismatched)| mismatched.then_some(reason));
            if let Some(reason) = binding_mismatch {
                return Err(invalid(reason));
            }
            // Reconciliation deliberately retains the former browser and
            // session identifiers on orphaned route and display rows as
            // diagnostic history. Once the pool entry is independently
            // available with no current allocation, those identifiers no
            // longer describe live occupancy and must not consume capacity.
            let reconciled_orphan_owner = !pending_acquisition
                && route.state == "orphaned"
                && display.state == "orphaned"
                && entry.state == "available"
                && entry.current_route_allocation_id.is_none();
            let browser = match route.browser_id.as_ref() {
                Some(id) if state.browsers.contains_key(id) => {
                    let browser = &state.browsers[id];
                    let browser_mismatch = [
                        (
                            "browser_display_allocation_id",
                            browser.display_allocation_id != route.display_allocation_id,
                        ),
                        (
                            "browser_display_name",
                            browser.display_name != display.display_name,
                        ),
                        (
                            "browser_boot_epoch",
                            browser.boot_epoch.as_deref() != Some(boot),
                        ),
                        (
                            "browser_active_session",
                            route.session_id.as_ref().is_none_or(|session| {
                                !browser.active_session_ids.contains(session)
                            }),
                        ),
                    ]
                    .into_iter()
                    .find_map(|(reason, mismatched)| mismatched.then_some(reason));
                    if let Some(reason) = browser_mismatch {
                        return Err(invalid(reason));
                    }
                    Some(browser)
                }
                Some(_) if pending_acquisition => None,
                Some(_) if reconciled_orphan_owner => None,
                Some(_) => return Err(invalid("browser_missing_outside_pending_acquisition")),
                None if route.session_id.is_none()
                    && route.state == "ready"
                    && display.state == "ready" =>
                {
                    None
                }
                None => return Err(invalid("route_browser_missing_for_bound_state")),
            };
            if !observe(expected, browser) {
                return Err(format!(
                    "production_presentation_inventory_owner_unproven:{}",
                    route.id
                ));
            }
            let mut slot = PresentationSlot::warm_idle(format!("slot:{}", entry.id))
                .with_binding(route.id.clone(), display.id.clone());
            slot.browser_id = browser.and_then(|_| route.browser_id.clone());
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
                }) else {
                    return Err(format!(
                        "production_presentation_inventory_capacity_custody_changed:{}",
                        json!({"reason":"binding_changed","previousSlot":old})
                    ));
                };
                let exact_pending_binding = slot.route_id.as_deref().is_some_and(|route_id| {
                    state.remote_view_routes.get(route_id).is_some_and(|route| {
                        (has_current_acquisition_binding(state, route)
                            || has_current_acquisition_custody(state, route))
                            && route.browser_id == slot.browser_id
                    })
                });
                let exact_pending_browser_acquisition = old.state
                    == PresentationSlotState::WarmIdle
                    && old.browser_id.is_none()
                    && old.lease_request_id.is_none()
                    && old.cleanup_obligation_ids.is_empty()
                    && slot.browser_id.is_some()
                    && exact_pending_binding;
                if exact_pending_browser_acquisition {
                    continue;
                }
                if slot.browser_id != old.browser_id {
                    return Err(format!(
                        "production_presentation_inventory_capacity_custody_changed:{}",
                        json!({
                            "reason":"browser_changed",
                            "exactPendingBinding":exact_pending_binding,
                            "previousSlot":old,
                            "qualifiedSlot":slot,
                        })
                    ));
                }
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
        let mut unavailable = capacity.clone();
        unavailable.admission_error =
            Some("production_presentation_inventory_owner_unproven:route".into());
        state.presentation_capacity = Some(unavailable);
        let refreshed = inventory
            .qualify(&state, config, "production", "boot-test", |_, _| true)
            .unwrap();
        assert_eq!(refreshed.slots, capacity.slots);
        assert!(refreshed.admission_error.is_none());
        assert_eq!(state.remote_view_routes["route"].state, "orphaned");
        assert_eq!(state.display_allocations["display"].state, "orphaned");
    }

    #[test]
    fn production_inventory_reports_the_exact_binding_axis() {
        let (inventory, mut state, config) = fixture();
        state.browsers.get_mut("browser").unwrap().display_name = Some(":99".into());

        assert_eq!(
            inventory
                .qualify(&state, config, "production", "boot-test", |_, _| true)
                .unwrap_err(),
            "production_presentation_inventory_binding_mismatch:route:browser_display_name"
        );
    }

    #[test]
    fn production_inventory_treats_reconciled_orphan_owner_as_warm_idle() {
        let (inventory, mut state, config) = fixture();
        state.browsers.clear();
        let repair = crate::native::service_health::reconcile_remote_view_state_with_display_probe(
            &mut state,
            |_| true,
            false,
        );
        assert_eq!(repair.orphaned_routes, 1);
        assert_eq!(repair.orphaned_display_allocations, 1);
        assert_eq!(state.remote_view_routes["route"].state, "orphaned");
        assert_eq!(state.display_allocations["display"].state, "orphaned");
        assert_eq!(state.route_pool["pool"].state, "available");
        assert_eq!(state.route_pool["pool"].current_route_allocation_id, None);
        let before = serde_json::to_value(&state).unwrap();

        let capacity = inventory
            .qualify(&state, config, "production", "boot-test", |_, _| true)
            .unwrap();

        assert_eq!(capacity.slots[0].state, PresentationSlotState::WarmIdle);
        assert!(capacity.slots[0].browser_id.is_none());
        assert_eq!(serde_json::to_value(&state).unwrap(), before);
    }

    #[test]
    fn production_inventory_rejects_missing_browser_for_checked_out_route() {
        let (inventory, mut state, config) = fixture();
        state.browsers.clear();
        state.remote_view_routes.get_mut("route").unwrap().state = "checked_out".into();
        let entry = state.route_pool.get_mut("pool").unwrap();
        entry.state = "checked_out".into();
        entry.current_route_allocation_id = Some("route".into());

        assert_eq!(
            inventory
                .qualify(&state, config, "production", "boot-test", |_, _| true)
                .unwrap_err(),
            "production_presentation_inventory_binding_mismatch:route:browser_missing_outside_pending_acquisition"
        );
    }

    #[test]
    fn production_inventory_preserves_pending_new_browser_acquisition() {
        let (mut inventory, mut state, config) = fixture();
        let boot = crate::process_identity::current_boot_epoch().unwrap();
        inventory.boot_epoch = boot.clone();
        state.browsers.clear();
        let readiness = json!({
            "state": "pending",
            "component": "remote_view_open_acquisition",
            "leaseId": "lease",
        });
        let route = state.remote_view_routes.get_mut("route").unwrap();
        route.browser_id = Some("new-browser".into());
        route.session_id = Some("new-session".into());
        route.state = "pending".into();
        route.last_provider_event = Some("remote_view_open_acquisition_pending".into());
        route.readiness = Some(readiness.clone());
        let display = state.display_allocations.get_mut("display").unwrap();
        display.boot_epoch = Some(boot.clone());
        display.owner_browser_id = Some("new-browser".into());
        display.owner_session_id = Some("new-session".into());
        display.state = "pending".into();
        display.readiness = Some(readiness.clone());
        let entry = state.route_pool.get_mut("pool").unwrap();
        entry.state = "pending".into();
        entry.current_route_allocation_id = Some("route".into());
        entry.readiness = Some(readiness);
        state.remote_view_acquisition_leases.insert(
            "lease".into(),
            crate::native::service_model::RemoteViewAcquisitionLease {
                id: "lease".into(),
                boot_epoch: Some(boot.clone()),
                browser_id: "new-browser".into(),
                session_id: "new-session".into(),
                route_id: "route".into(),
                display_allocation_id: "display".into(),
                route_pool_entry_id: Some("pool".into()),
                state: "pending".into(),
                phase: "reserved".into(),
                ..Default::default()
            },
        );

        let capacity = inventory
            .qualify(&state, config, "production", &boot, |_, _| true)
            .unwrap();
        assert_eq!(capacity.slots[0].state, PresentationSlotState::WarmIdle);
        assert!(capacity.slots[0].browser_id.is_none());

        state.presentation_capacity = Some(capacity);
        state.browsers.insert(
            "new-browser".into(),
            BrowserProcess {
                id: "new-browser".into(),
                boot_epoch: Some(boot.clone()),
                display_name: Some(":14".into()),
                display_allocation_id: Some("display".into()),
                active_session_ids: vec!["new-session".into()],
                ..Default::default()
            },
        );
        state.remote_view_routes.get_mut("route").unwrap().readiness =
            Some(json!({"state":"ready","source":"browser_observation"}));
        assert!(!has_current_acquisition_binding(
            &state,
            &state.remote_view_routes["route"]
        ));
        assert!(has_current_acquisition_custody(
            &state,
            &state.remote_view_routes["route"]
        ));
        let capacity = inventory
            .qualify(&state, config, "production", &boot, |_, _| true)
            .unwrap();
        assert_eq!(capacity.slots[0].state, PresentationSlotState::Active);
        assert_eq!(capacity.slots[0].browser_id.as_deref(), Some("new-browser"));
    }

    #[test]
    fn production_inventory_preserves_real_pending_handoff_acquisition() {
        use crate::native::remote_view::RemoteViewAcquisitionPlan;
        use crate::native::remote_view_handoff::{
            begin_route_bound_handoff_acquisition, BeginRouteBoundHandoffAcquisitionInput,
        };
        use crate::native::service_store::{
            JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
        };
        let (mut inventory, mut state, config) = fixture();
        state.browsers.get_mut("browser").unwrap().health =
            crate::native::service_model::BrowserHealth::Ready;
        state.viewer_leases.insert(
            "viewer".into(),
            crate::native::service_model::ViewerLease {
                id: "viewer".into(),
                browser_id: Some("browser".into()),
                route_id: Some("route".into()),
                state: "active".into(),
                ..Default::default()
            },
        );
        let boot = crate::process_identity::current_boot_epoch().unwrap();
        inventory.boot_epoch = boot.clone();
        state.browsers.get_mut("browser").unwrap().boot_epoch = Some(boot.clone());
        state
            .display_allocations
            .get_mut("display")
            .unwrap()
            .boot_epoch = Some(boot.clone());
        state.presentation_capacity = Some(
            inventory
                .qualify(&state, config, "production", &boot, |_, _| true)
                .unwrap(),
        );
        let directory = std::env::temp_dir().join(format!(
            "production-pending-handoff-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("state.json");
        fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        let plan: RemoteViewAcquisitionPlan = serde_json::from_value(json!({
            "mode":"strict_operator_open", "reusePolicy":"retained", "tabPolicy":"reuse",
            "requestedBrowserHost":"remote_headed", "requestedViewStreamProvider":"rdp_gateway",
            "requestedControlInput":"manual_attached_desktop", "selectedRoutePoolEntryId":"pool",
            "selectedRouteId":"route", "displayAllocationId":"display", "displayName":":14",
            "routeBinding":{"routeId":"route", "routePoolEntryId":"pool",
                "displayAllocationId":"display", "displayName":":14", "launchDisplayName":":14",
                "displayIsolation":"shared_display", "routeUser":"route-user",
                "provider":"rdp_gateway", "providerMode":"simultaneous_view", "connectionId":"3"},
            "decisions":[], "blockers":[], "proofRequired":[], "cleanupOnFailure":[], "suggestedCommands":[]
        })).unwrap();
        let lease = begin_route_bound_handoff_acquisition(
            &repository,
            BeginRouteBoundHandoffAcquisitionInput {
                inline_route_pool_entry: None,
                acquisition_plan: &plan,
                browser_id: "browser",
                session_id: "session",
                observed_at: "2026-09-07T23:00:00Z",
                default_control_input: None,
            },
        )
        .unwrap();
        let mut pending = repository.load_snapshot().unwrap();
        crate::native::service_health::reconcile_remote_view_state_with_display_probe(
            &mut pending,
            |_| true,
            false,
        );
        assert_eq!(pending.remote_view_routes["route"].state, "pending");
        assert_eq!(pending.viewer_leases["viewer"].state, "active");
        let mut capacity = inventory
            .qualify(&pending, config, "production", &boot, |_, _| true)
            .expect("the native acquisition reservation must survive inventory reload");
        assert_eq!(capacity.reconcile_authoritative_bindings(&pending), 0);
        assert_eq!(capacity.slots[0].browser_id.as_deref(), Some("browser"));
        capacity
            .activate_bound_browser("route", "display", "browser")
            .unwrap();
        assert_eq!(pending.remote_view_routes["route"].state, "pending");
        assert_eq!(pending.display_allocations["display"].state, "pending");
        let mut missing_display = pending.clone();
        crate::native::service_health::reconcile_remote_view_state_with_display_probe(
            &mut missing_display,
            |_| false,
            false,
        );
        assert_eq!(
            missing_display.remote_view_routes["route"].state,
            "orphaned"
        );
        assert_eq!(
            missing_display.viewer_leases["viewer"].state,
            "disconnected"
        );
        for drift in ["browser", "boot", "completed", "pool"] {
            let mut conflicting = pending.clone();
            let record = conflicting
                .remote_view_acquisition_leases
                .get_mut(&lease.id)
                .unwrap();
            match drift {
                "browser" => record.browser_id = "foreign".into(),
                "boot" => record.boot_epoch = Some("prior-boot".into()),
                "completed" => record.completed_at = Some("2026-09-07T23:00:01Z".into()),
                "pool" => record.route_pool_entry_id = Some("foreign-pool".into()),
                _ => unreachable!(),
            }
            assert!(
                inventory
                    .qualify(&conflicting, config, "production", &boot, |_, _| true)
                    .is_err(),
                "{drift}"
            );
        }
        fs::remove_dir_all(directory).unwrap();
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
