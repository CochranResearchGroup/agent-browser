//! Canonical list-shaped route inventory contract fixtures.

use serde::Deserialize;
use std::collections::BTreeSet;

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
}
