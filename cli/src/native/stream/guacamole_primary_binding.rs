//! Exact Service authority for a backend-owned provider connection.

use crate::native::runtime_lifecycle::{digest_json, RuntimeLifecycleAuthority};
use crate::native::service_model::ViewStreamProvider;
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
                    && route.state == "ready"
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

    pub fn is_current(&self, repository: &impl ServiceStateRepository) -> bool {
        Self::resolve(repository, &self.route_id, &self.connection_id)
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
    use crate::native::service_model::ServiceState;
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
