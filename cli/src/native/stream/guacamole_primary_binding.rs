//! Exact Service authority for a backend-owned provider connection.

use crate::native::runtime_lifecycle::{digest_json, RuntimeLifecycleAuthority};
use crate::native::service_model::{RemoteViewRoute, ServiceState, ViewStreamProvider};
use crate::native::service_store::ServiceStateRepository;
use crate::runtime_owner_transfer::OwnerAuthorityClaim;
use sha2::{Digest, Sha256};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct PrimaryBinding {
    pub route_id: String,
    pub connection_id: String,
    pub provider_base: reqwest::Url,
    owner: OwnerAuthorityClaim,
    display_id: String,
    display_name: String,
    session_id: String,
    process_id: u32,
    endpoint_digest: String,
}

impl PrimaryBinding {
    #[cfg(test)]
    pub fn synthetic_fixture() -> Self {
        Self::resolve(&tests::repository(), "route", "1").unwrap()
    }
    pub fn resolve(
        repository: &impl ServiceStateRepository,
        route_id: &str,
        connection_id: &str,
    ) -> Result<Self, &'static str> {
        Self::resolve_inner(repository, route_id, connection_id, None)
    }

    fn resolve_inner(
        repository: &impl ServiceStateRepository,
        route_id: &str,
        connection_id: &str,
        retained: Option<&Self>,
    ) -> Result<Self, &'static str> {
        let snapshot = repository
            .load_snapshot()
            .map_err(|_| "guacamole_primary_state_unavailable")?;
        let route = snapshot
            .remote_view_routes
            .get(route_id)
            .filter(|route| {
                route.id == route_id
                    && route.connection_id.as_deref() == Some(connection_id)
                    && route.provider == ViewStreamProvider::RdpGateway
                    && route.provider_mode == "simultaneous_view"
                    && (route.state == "ready"
                        || retained.is_some_and(|binding| {
                            binding.pending_revalidation_matches(&snapshot, route)
                        }))
            })
            .ok_or("guacamole_primary_route_unavailable")?;
        let browser = route
            .browser_id
            .as_ref()
            .and_then(|id| snapshot.browsers.get(id))
            .filter(|browser| Some(&browser.id) == route.browser_id.as_ref())
            .ok_or("guacamole_primary_browser_unavailable")?;
        let mut owners = snapshot
            .runtime_owner_registry
            .owners
            .values()
            .filter(|owner| owner.browser_id == browser.id);
        let owner = owners.next().ok_or("guacamole_primary_owner_unavailable")?;
        if owners.next().is_some() {
            return Err("guacamole_primary_owner_ambiguous");
        }
        let mut binding = snapshot
            .runtime_owner_registry
            .binding_for_session(&owner.daemon_session_route)
            .map_err(|_| "guacamole_primary_owner_unavailable")?
            .ok_or("guacamole_primary_owner_unavailable")?;
        let expected_claim = OwnerAuthorityClaim::from_owner(owner);
        RuntimeLifecycleAuthority::new(repository)
            .authorize_effect(&mut binding)
            .map_err(|_| "guacamole_primary_owner_stale")?;
        if binding.claim != expected_claim {
            return Err("guacamole_primary_owner_stale");
        }
        let process_id = browser.pid.ok_or("guacamole_primary_process_unproven")?;
        let process = crate::process_identity::capture_process_identity(process_id, None, None)
            .ok_or("guacamole_primary_process_unproven")?;
        if digest_json(&process).map_err(|_| "guacamole_primary_process_unproven")?
            != owner.process_instance_digest
        {
            return Err("guacamole_primary_process_changed");
        }
        let endpoint = browser
            .cdp_endpoint
            .as_deref()
            .ok_or("guacamole_primary_endpoint_unproven")?;
        let endpoint_digest = format!("{:x}", Sha256::digest(endpoint.as_bytes()));
        if endpoint_digest != owner.cdp_endpoint_identity_digest {
            return Err("guacamole_primary_endpoint_changed");
        }
        let display_id = route
            .display_allocation_id
            .as_ref()
            .ok_or("guacamole_primary_display_unavailable")?;
        let session_id = route
            .session_id
            .as_ref()
            .ok_or("guacamole_primary_display_unavailable")?;
        let display = snapshot
            .display_allocations
            .get(display_id)
            .filter(|display| {
                display.id == *display_id
                    && display.owner_browser_id.as_ref() == Some(&browser.id)
                    && display.owner_session_id.as_ref() == Some(session_id)
                    && display.route_ids.iter().any(|id| id == route_id)
                    && browser.display_allocation_id.as_ref() == Some(display_id)
            })
            .ok_or("guacamole_primary_display_changed")?;
        let display_name = display
            .display_name
            .as_ref()
            .filter(|name| !name.is_empty() && browser.display_name.as_ref() == Some(name))
            .ok_or("guacamole_primary_display_changed")?;
        let local_url = route
            .route_descriptor
            .as_ref()
            .and_then(|descriptor| descriptor.get("localEmbedUrl"))
            .and_then(serde_json::Value::as_str)
            .ok_or("guacamole_primary_provider_unavailable")?;
        Ok(Self {
            route_id: route_id.to_owned(),
            connection_id: connection_id.to_owned(),
            provider_base: local_provider_base(local_url)?,
            owner: expected_claim,
            display_id: display_id.clone(),
            display_name: display_name.clone(),
            session_id: session_id.clone(),
            process_id,
            endpoint_digest,
        })
    }

    /// Record terminal provider custody at the backend owner, independently of
    /// whether a viewer is still present to report the failed connection.
    pub fn record_terminal(&self, occurrence_id: &str, code: &'static str, elapsed_ms: u64) {
        use crate::native::service_failure_journal::{
            append_service_failure_best_effort, ServiceFailureCategory, ServiceFailureRecord,
            ServiceFailureReferences,
        };
        let mut record = ServiceFailureRecord::new(
            ServiceFailureCategory::GuacamoleLoad,
            "guacamole_primary_owner",
            "terminal",
            code,
            "The backend-owned Guacamole primary connection ended.",
        )
        .with_action("guacamole_primary_ensure")
        .with_references(ServiceFailureReferences {
            route_id: Some(self.route_id.clone()),
            session_id: Some(self.session_id.clone()),
            display_id: Some(self.display_id.clone()),
            ..ServiceFailureReferences::default()
        })
        .with_details(serde_json::json!({
            "elapsedMs": elapsed_ms,
            "retrySafe": false,
            "recourse": "inspect_remote_view_provider",
        }));
        record.occurrence_id = occurrence_id.to_owned();
        append_service_failure_best_effort(&record);
    }

    /// Continuity, never admission: a current acquisition may temporarily mark
    /// the same ready route pending. Require its exact prior route and display
    /// custody while the usual live owner/process/endpoint guards still apply.
    fn pending_revalidation_matches(
        &self,
        snapshot: &ServiceState,
        route: &RemoteViewRoute,
    ) -> bool {
        if route.state != "pending"
            || route.last_provider_event.as_deref() != Some("remote_view_open_acquisition_pending")
        {
            return false;
        }
        let Some(lease_id) = route
            .readiness
            .as_ref()
            .filter(|value| value["component"] == "remote_view_open_acquisition")
            .and_then(|value| value["leaseId"].as_str())
        else {
            return false;
        };
        let Some(lease) = snapshot.remote_view_acquisition_leases.get(lease_id) else {
            return false;
        };
        if lease.id != lease_id
            || lease.state != "pending"
            || !matches!(
                lease.phase.as_str(),
                "reserved" | "display_ready" | "browser_attached" | "tab_acquired" | "proof_ready"
            )
            || lease.boot_epoch.is_none()
            || lease.boot_epoch != crate::process_identity::current_boot_epoch()
            || lease.browser_id != self.owner.logical_browser_id
            || lease.session_id != self.session_id
            || lease.route_id != self.route_id
            || lease.display_allocation_id != self.display_id
            || lease.previous_browser_display_allocation_id.as_ref() != Some(&self.display_id)
            || lease.completed_at.is_some()
            || lease.failed_at.is_some()
        {
            return false;
        }
        let Some(previous) = lease.previous_remote_view_route.as_ref() else {
            return false;
        };
        let Some(display) = lease.previous_display_allocation.as_ref() else {
            return false;
        };
        previous.state == "ready"
            && previous.id == self.route_id
            && previous.browser_id.as_ref() == Some(&self.owner.logical_browser_id)
            && previous.session_id.as_ref() == Some(&self.session_id)
            && previous.display_allocation_id.as_ref() == Some(&self.display_id)
            && previous.connection_id.as_ref() == Some(&self.connection_id)
            && previous.provider == ViewStreamProvider::RdpGateway
            && previous.provider_mode == "simultaneous_view"
            && previous
                .route_descriptor
                .as_ref()
                .and_then(|value| value["localEmbedUrl"].as_str())
                .and_then(|value| local_provider_base(value).ok())
                .is_some_and(|base| base == self.provider_base)
            && display.id == self.display_id
            && display.display_name.as_ref() == Some(&self.display_name)
            && display.owner_browser_id.as_ref() == Some(&self.owner.logical_browser_id)
            && display.owner_session_id.as_ref() == Some(&self.session_id)
            && display.route_ids.contains(&self.route_id)
    }

    pub fn is_current(&self, repository: &impl ServiceStateRepository) -> bool {
        Self::resolve_inner(repository, &self.route_id, &self.connection_id, Some(self))
            .is_ok_and(|current| current == *self)
    }
}

fn local_provider_base(value: &str) -> Result<reqwest::Url, &'static str> {
    let mut url = reqwest::Url::parse(value).map_err(|_| "guacamole_primary_provider_invalid")?;
    let loopback = url.host_str().is_some_and(|host| {
        host.trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    });
    if !loopback
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.path() != "/guacamole/"
    {
        return Err("guacamole_primary_provider_invalid");
    }
    url.set_fragment(None);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(super) struct Repository(ServiceState);

    impl ServiceStateRepository for Repository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.0.clone())
        }

        fn mutate<R>(
            &self,
            _: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            panic!("primary binding admission must not mutate Service authority")
        }
    }

    pub(super) fn repository() -> Repository {
        let process =
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .unwrap();
        let process_digest = digest_json(&process).unwrap();
        let profile_digest = "a".repeat(64);
        let endpoint = "http://127.0.0.1:9222";
        let endpoint_digest = format!("{:x}", Sha256::digest(endpoint.as_bytes()));
        Repository(serde_json::from_value(json!({
            "browsers": {"browser": {
                "id": "browser", "pid": std::process::id(), "cdpEndpoint": endpoint,
                "displayAllocationId": "display", "displayName": ":10"
            }},
            "displayAllocations": {"display": {
                "id": "display", "ownerBrowserId": "browser", "ownerSessionId": "scene",
                "displayName": ":10", "routeIds": ["route"]
            }},
            "remoteViewRoutes": {"route": {
                "id": "route", "provider": "rdp_gateway", "providerMode": "simultaneous_view",
                "state": "ready", "connectionId": "1", "browserId": "browser",
                "sessionId": "scene", "displayAllocationId": "display",
                "routeDescriptor": {"localEmbedUrl": "http://127.0.0.1:8193/guacamole/#/client/1"}
            }},
            "runtimeOwnerRegistry": {
                "owners": {profile_digest.clone(): {
                    "ownerId": "owner", "profileIdentityDigest": profile_digest,
                    "state": "ready", "ownerGeneration": 7, "browserId": "browser",
                    "daemonSessionRoute": "daemon", "processInstanceDigest": process_digest,
                    "browserFamily": "chrome", "cdpEndpointIdentityDigest": endpoint_digest,
                    "targetSetDigest": "b".repeat(64)
                }},
                "lifecycleRecords": {"browser": {
                    "logicalBrowserId": "browser", "profileIdentityDigest": profile_digest,
                    "ownerGeneration": 7, "lifecycleState": "ready", "cleanupObligationState": "owned"
                }}
            }
        })).unwrap())
    }

    #[test]
    fn retained_primary_survives_real_revalidation_reservation_without_admitting_new_owner() {
        use crate::native::remote_view::RemoteViewAcquisitionPlan;
        use crate::native::remote_view_handoff::begin_route_bound_handoff_plan_acquisition;
        use crate::native::service_store::{
            JsonServiceStateStore, LockedServiceStateRepository, ServiceStateStore,
        };
        let root =
            std::env::temp_dir().join(format!("primary-reservation-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let store = JsonServiceStateStore::new(root.join("state.json"));
        let mut initial = repository().0;
        let mut capacity_slot =
            crate::native::presentation_capacity::PresentationSlot::warm_idle("slot:slot")
                .with_binding("route", "display");
        capacity_slot.state = crate::native::presentation_capacity::PresentationSlotState::Active;
        capacity_slot.browser_id = Some("browser".into());
        capacity_slot.scene_generation = 13;
        capacity_slot.lease_request_id = Some("retained-recovery".into());
        initial.presentation_capacity = Some(
            crate::native::presentation_capacity::PresentationCapacityAuthority {
                slots: vec![capacity_slot],
                ..Default::default()
            },
        );
        initial.route_pool.insert(
            "slot".into(),
            serde_json::from_value(json!({
                "id":"slot", "routeId":"route", "state":"checked_out",
                "provider":"rdp_gateway", "providerMode":"simultaneous_view",
                "currentRouteAllocationId":"route"
            }))
            .unwrap(),
        );
        store.save(&initial).unwrap();
        let live = LockedServiceStateRepository::new(store);
        let binding = PrimaryBinding::resolve(&live, "route", "1").unwrap();
        let plan: RemoteViewAcquisitionPlan = serde_json::from_value(json!({
            "mode":"reuse", "reusePolicy":"reuse_existing", "tabPolicy":"reuse_existing",
            "requestedBrowserHost":"remote_headed", "requestedViewStreamProvider":"rdp_gateway",
            "requestedControlInput":"manual_attached_desktop", "selectedRouteId":"route",
            "selectedRoutePoolEntryId":"slot",
            "displayAllocationId":"display", "displayName":":10",
            "routeBinding": {
                "routeId":"route", "displayAllocationId":"display", "displayName":":10",
                "launchDisplayName":":10", "displayIsolation":"private_virtual_display",
                "provider":"rdp_gateway", "providerMode":"simultaneous_view", "connectionId":"1",
                "routeDescriptor":{"localEmbedUrl":"http://127.0.0.1:8193/guacamole/#/client/1"}
            },
            "decisions":[], "blockers":[], "proofRequired":[], "cleanupOnFailure":[], "suggestedCommands":[]
        })).unwrap();
        let lease = begin_route_bound_handoff_plan_acquisition(
            &live,
            None,
            &plan,
            "browser",
            "scene",
            "2026-09-06T06:00:00Z",
        )
        .unwrap();
        assert!(PrimaryBinding::resolve(&live, "route", "1").is_err());
        assert!(
            binding.is_current(&live),
            "same-binding reservation invalidated retained primary"
        );
        let pending = live.load_snapshot().unwrap();
        for case in [
            "missing_lease",
            "failed_lease",
            "foreign_previous_route",
            "foreign_display",
            "changed_owner",
            "released_route",
        ] {
            let mut changed = Repository(pending.clone());
            match case {
                "missing_lease" => changed.0.remote_view_acquisition_leases.clear(),
                "failed_lease" => {
                    changed
                        .0
                        .remote_view_acquisition_leases
                        .get_mut(&lease.id)
                        .unwrap()
                        .state = "failed".into()
                }
                "foreign_previous_route" => {
                    changed
                        .0
                        .remote_view_acquisition_leases
                        .get_mut(&lease.id)
                        .unwrap()
                        .previous_remote_view_route
                        .as_mut()
                        .unwrap()
                        .browser_id = Some("peer".into())
                }
                "foreign_display" => {
                    changed
                        .0
                        .display_allocations
                        .get_mut("display")
                        .unwrap()
                        .owner_browser_id = Some("peer".into())
                }
                "changed_owner" => {
                    changed
                        .0
                        .runtime_owner_registry
                        .owners
                        .values_mut()
                        .next()
                        .unwrap()
                        .owner_generation += 1
                }
                "released_route" => {
                    changed.0.remote_view_routes.get_mut("route").unwrap().state = "released".into()
                }
                _ => unreachable!(),
            }
            assert!(!binding.is_current(&changed), "{case}");
        }
        // Installed repository reads overlay provider inventory after loading
        // durable reservations. Exercise that boundary, not only the raw store.
        let inventory = crate::native::presentation_inventory::PresentationProviderInventory::from_json(
            &json!({
                "schemaVersion":"agent-browser.development-presentation-inventory.v1",
                "environment":"development",
                "routes":[{
                    "routeId":"route", "slotId":"slot", "connectionId":"1",
                    "displayReservationId":"display", "displayName":":10", "state":"ready",
                    "routeDescriptor":{"localEmbedUrl":"http://127.0.0.1:8193/guacamole/#/client/1"}
                }]
            }).to_string()
        ).unwrap();
        let config = crate::native::presentation_capacity::PresentationCapacityConfig {
            warm_minimum: 1,
            hard_maximum: 2,
            human_priority_reserve: 1,
            recovery_reserve: 1,
            max_queue_depth: 64,
        };
        let mut overlaid = pending.clone();
        inventory
            .overlay_service_state(&mut overlaid, config.clone())
            .unwrap();
        let mut overlaid = Repository(overlaid);
        assert!(
            binding.is_current(&overlaid),
            "provider inventory erased retained reservation custody"
        );
        assert!(PrimaryBinding::resolve(&overlaid, "route", "1").is_err());
        assert_eq!(overlaid.0.remote_view_routes["route"].state, "pending");
        assert_eq!(overlaid.0.display_allocations["display"].state, "pending");
        assert_eq!(overlaid.0.route_pool["slot"].state, "pending");
        // Checkout calls this on the inventory-overlaid repository. Retaining
        // the primary alone is insufficient if refresh loses its capacity slot.
        overlaid
            .0
            .presentation_capacity
            .as_mut()
            .unwrap()
            .activate_bound_browser("route", "display", "browser")
            .expect("inventory refresh lost the slot needed by route checkout");
        let retained_slot = &overlaid.0.presentation_capacity.as_ref().unwrap().slots[0];
        assert_eq!(retained_slot.scene_generation, 13);
        assert_eq!(
            retained_slot.lease_request_id.as_deref(),
            Some("retained-recovery")
        );
        for case in [
            "missing_capacity",
            "foreign_browser",
            "foreign_display",
            "foreign_route",
            "foreign_slot",
        ] {
            let mut changed = pending.clone();
            let capacity = changed.presentation_capacity.as_mut().unwrap();
            match case {
                "missing_capacity" => capacity.slots.clear(),
                "foreign_browser" => capacity.slots[0].browser_id = Some("peer".into()),
                "foreign_display" => capacity.slots[0].display_allocation_id = Some("peer".into()),
                "foreign_route" => capacity.slots[0].route_id = Some("peer".into()),
                "foreign_slot" => capacity.slots[0].id = "peer".into(),
                _ => unreachable!(),
            }
            inventory
                .overlay_service_state(&mut changed, config.clone())
                .unwrap();
            assert!(
                changed.presentation_capacity.unwrap().slots.is_empty(),
                "overlay manufactured or borrowed {case}"
            );
        }
        for case in [
            "missing",
            "failed",
            "stale_boot",
            "foreign_session",
            "terminal_phase",
        ] {
            let mut changed = pending.clone();
            let current = changed
                .remote_view_acquisition_leases
                .get_mut(&lease.id)
                .unwrap();
            match case {
                "missing" => changed.remote_view_acquisition_leases.clear(),
                "failed" => current.failed_at = Some("2026-09-06T06:00:01Z".into()),
                "stale_boot" => current.boot_epoch = Some("foreign-boot".into()),
                "foreign_session" => current.session_id = "foreign".into(),
                "terminal_phase" => current.phase = "rollback_incomplete".into(),
                _ => unreachable!(),
            }
            inventory
                .overlay_service_state(&mut changed, config.clone())
                .unwrap();
            assert!(
                !binding.is_current(&Repository(changed)),
                "overlay admitted {case} acquisition"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_owner_survives_viewer_changes_but_rejects_authority_and_display_drift() {
        let mut repository = repository();
        let binding = PrimaryBinding::resolve(&repository, "route", "1").unwrap();
        assert!(binding.is_current(&repository));
        let route = repository.0.remote_view_routes.get_mut("route").unwrap();
        route.viewer_lease_ids.push("new-viewer".into());
        route.controller_lease_id = Some("new-controller".into());
        route.controller_epoch += 1;
        assert!(binding.is_current(&repository));

        let original = repository.0.clone();
        repository.0.browsers.get_mut("browser").unwrap().pid = None;
        assert_eq!(
            PrimaryBinding::resolve(&repository, "route", "1").err(),
            Some("guacamole_primary_process_unproven")
        );
        repository.0 = original.clone();
        repository
            .0
            .browsers
            .get_mut("browser")
            .unwrap()
            .cdp_endpoint = Some("http://127.0.0.1:9223".into());
        assert_eq!(
            PrimaryBinding::resolve(&repository, "route", "1").err(),
            Some("guacamole_primary_endpoint_changed")
        );
        repository.0 = original.clone();
        repository
            .0
            .display_allocations
            .get_mut("display")
            .unwrap()
            .owner_browser_id = Some("peer".into());
        assert!(!binding.is_current(&repository));
        repository.0 = original.clone();
        repository
            .0
            .runtime_owner_registry
            .lifecycle_records
            .clear();
        assert_eq!(
            PrimaryBinding::resolve(&repository, "route", "1").err(),
            Some("guacamole_primary_owner_stale")
        );
        repository.0 = original.clone();
        repository.0.runtime_owner_registry.owners.clear();
        assert_eq!(
            PrimaryBinding::resolve(&repository, "route", "1").err(),
            Some("guacamole_primary_owner_unavailable")
        );
        repository.0 = original;
        assert!(PrimaryBinding::resolve(&repository, "route", "foreign-connection").is_err());
        assert!(PrimaryBinding::resolve(&repository, "foreign-route", "1").is_err());
    }

    #[test]
    fn provider_origin_requires_literal_loopback_without_credentials_or_query() {
        assert_eq!(
            local_provider_base("http://127.0.0.1:8193/guacamole/#/client/1")
                .unwrap()
                .as_str(),
            "http://127.0.0.1:8193/guacamole/"
        );
        assert!(local_provider_base("http://[::1]:8193/guacamole/").is_ok());
        for value in [
            "https://provider.example/guacamole/",
            "http://localhost/guacamole/",
            "http://user:secret@127.0.0.1/guacamole/",
            "http://127.0.0.1/guacamole/?token=secret",
            "http://127.0.0.1/other/",
            "file:///guacamole/",
        ] {
            assert_eq!(
                local_provider_base(value).err(),
                Some("guacamole_primary_provider_invalid")
            );
        }
    }
}
