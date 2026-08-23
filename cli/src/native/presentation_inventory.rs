//! Canonical list-shaped route inventory contract fixtures.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteCandidate {
    pub(crate) id: String,
}

impl RouteCandidate {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteInventory {
    candidates: Vec<RouteCandidate>,
}

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
}
