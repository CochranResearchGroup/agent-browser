//! Deterministic desktop selection for the ordinary remote-view path.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDesktopRoute {
    pub id: String,
    pub display_name: String,
    pub healthy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDesktopAssignment {
    pub route_id: String,
    pub display_name: String,
    pub live_browser_count: usize,
}

/// Select the least-crowded healthy virtual desktop in configured route order.
///
/// The caller supplies only current live-browser display assignments. Retained
/// leases, historical allocations, and prior route ownership are deliberately
/// outside this calculation. Display `:0` is local-screen-only and is never an
/// automatic remote-view candidate.
pub fn select_least_crowded_browser_desktop(
    routes: &[BrowserDesktopRoute],
    live_browser_display_names: &[String],
) -> Result<BrowserDesktopAssignment, String> {
    validate_healthy_routes(routes)?;

    let mut counts = HashMap::<&str, usize>::new();
    for display_name in live_browser_display_names {
        *counts.entry(display_name.as_str()).or_default() += 1;
    }

    routes
        .iter()
        .filter(|route| route.healthy && route.display_name != ":0")
        .map(|route| BrowserDesktopAssignment {
            route_id: route.id.clone(),
            display_name: route.display_name.clone(),
            live_browser_count: counts
                .get(route.display_name.as_str())
                .copied()
                .unwrap_or_default(),
        })
        .min_by_key(|assignment| assignment.live_browser_count)
        .ok_or_else(|| "browser_desktop_route_unavailable".to_string())
}

/// Select a healthy desktop while enforcing display and per-display capacity.
///
/// Every observed browser display counts toward the configured limits, even
/// when its route is absent or unhealthy. Existing placements are never
/// changed: this function either selects a destination for one new browser or
/// returns a typed capacity state for the caller to handle.
pub fn select_browser_desktop_with_capacity(
    routes: &[BrowserDesktopRoute],
    live_display_names: &[String],
    maximum_displays: u32,
    maximum_browsers_per_display: u32,
) -> Result<BrowserDesktopAssignment, String> {
    if maximum_displays == 0 {
        return Err("browser_runtime_config_maximum_displays_invalid".to_string());
    }
    if maximum_browsers_per_display == 0 {
        return Err("browser_runtime_config_display_density_invalid".to_string());
    }
    validate_healthy_routes(routes)?;

    let mut counts = HashMap::<&str, usize>::new();
    for display_name in live_display_names {
        *counts.entry(display_name.as_str()).or_default() += 1;
    }

    let maximum_displays = maximum_displays as usize;
    let maximum_browsers_per_display = maximum_browsers_per_display as usize;
    if counts.len() > maximum_displays
        || counts
            .values()
            .any(|count| *count > maximum_browsers_per_display)
    {
        return Err("presentation_capacity_over_target".to_string());
    }

    if counts.len() < maximum_displays {
        if let Some(route) = routes.iter().find(|route| {
            route.healthy
                && route.display_name != ":0"
                && !counts.contains_key(route.display_name.as_str())
        }) {
            return Ok(BrowserDesktopAssignment {
                route_id: route.id.clone(),
                display_name: route.display_name.clone(),
                live_browser_count: 0,
            });
        }
        return Err("presentation_capacity_pending".to_string());
    }

    routes
        .iter()
        .filter(|route| route.healthy && route.display_name != ":0")
        .filter_map(|route| {
            let live_browser_count = counts.get(route.display_name.as_str()).copied()?;
            (live_browser_count < maximum_browsers_per_display).then(|| BrowserDesktopAssignment {
                route_id: route.id.clone(),
                display_name: route.display_name.clone(),
                live_browser_count,
            })
        })
        .min_by_key(|assignment| assignment.live_browser_count)
        .ok_or_else(|| "presentation_capacity_full".to_string())
}

fn validate_healthy_routes(routes: &[BrowserDesktopRoute]) -> Result<(), String> {
    let mut route_ids = HashSet::new();
    let mut display_names = HashSet::new();
    for route in routes.iter().filter(|route| route.healthy) {
        if route.id.trim().is_empty() {
            return Err("browser_desktop_route_id_missing".to_string());
        }
        if route.display_name.trim().is_empty() {
            return Err(format!("browser_desktop_display_missing:{}", route.id));
        }
        if !route_ids.insert(route.id.as_str()) {
            return Err(format!("browser_desktop_route_id_duplicate:{}", route.id));
        }
        if !display_names.insert(route.display_name.as_str()) {
            return Err(format!(
                "browser_desktop_display_duplicate:{}",
                route.display_name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(id: &str, display_name: &str, healthy: bool) -> BrowserDesktopRoute {
        BrowserDesktopRoute {
            id: id.to_string(),
            display_name: display_name.to_string(),
            healthy,
        }
    }

    fn displays(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn grows_onto_unused_healthy_displays_before_sharing() {
        let routes = vec![route("route-a", ":10", true), route("route-b", ":11", true)];

        let selected =
            select_browser_desktop_with_capacity(&routes, &displays(&[":10"]), 2, 4).unwrap();

        assert_eq!(selected.route_id, "route-b");
        assert_eq!(selected.display_name, ":11");
        assert_eq!(selected.live_browser_count, 0);
    }

    #[test]
    fn waits_for_another_ready_display_instead_of_sharing_early() {
        let routes = vec![route("route-a", ":10", true)];

        assert_eq!(
            select_browser_desktop_with_capacity(&routes, &displays(&[":10"]), 2, 4),
            Err("presentation_capacity_pending".to_string())
        );
    }

    #[test]
    fn shares_the_least_loaded_display_at_the_display_maximum() {
        let routes = vec![route("route-a", ":10", true), route("route-b", ":11", true)];

        let selected =
            select_browser_desktop_with_capacity(&routes, &displays(&[":10", ":10", ":11"]), 2, 3)
                .unwrap();

        assert_eq!(selected.route_id, "route-b");
        assert_eq!(selected.live_browser_count, 1);
        assert_eq!(
            select_browser_desktop_with_capacity(
                &routes,
                &displays(&[":10", ":10", ":10", ":11", ":11", ":11"]),
                2,
                3,
            ),
            Err("presentation_capacity_full".to_string())
        );
        assert_eq!(
            select_browser_desktop_with_capacity(
                &routes,
                &displays(&[":10", ":10", ":10", ":10", ":11"]),
                2,
                3,
            ),
            Err("presentation_capacity_over_target".to_string())
        );
    }

    #[test]
    fn lowering_limits_reports_over_target_without_changing_placements() {
        let routes = vec![route("route-a", ":10", true), route("route-b", ":11", true)];
        let live_displays = displays(&[":10", ":10", ":11"]);
        let before = live_displays.clone();

        assert_eq!(
            select_browser_desktop_with_capacity(&routes, &live_displays, 1, 1),
            Err("presentation_capacity_over_target".to_string())
        );
        assert_eq!(live_displays, before);
    }

    #[test]
    fn unknown_and_unhealthy_occupancy_counts_toward_capacity() {
        let routes = vec![
            route("unhealthy", ":10", false),
            route("healthy", ":11", true),
        ];

        assert_eq!(
            select_browser_desktop_with_capacity(&routes, &displays(&[":10", ":unknown"]), 2, 4,),
            Err("presentation_capacity_full".to_string())
        );
        assert_eq!(
            select_browser_desktop_with_capacity(
                &routes,
                &displays(&[":10", ":unknown", ":other"]),
                2,
                4,
            ),
            Err("presentation_capacity_over_target".to_string())
        );
    }

    #[test]
    fn configured_route_order_breaks_capacity_selection_ties() {
        let routes = vec![route("route-b", ":11", true), route("route-a", ":10", true)];

        let unused = select_browser_desktop_with_capacity(&routes, &[], 2, 2).unwrap();
        assert_eq!(unused.route_id, "route-b");

        let shared =
            select_browser_desktop_with_capacity(&routes, &displays(&[":10", ":11"]), 2, 2)
                .unwrap();
        assert_eq!(shared.route_id, "route-b");
    }

    #[test]
    fn zero_capacity_limits_are_rejected() {
        let routes = vec![route("route-a", ":10", true)];

        assert_eq!(
            select_browser_desktop_with_capacity(&routes, &[], 0, 1),
            Err("browser_runtime_config_maximum_displays_invalid".to_string())
        );
        assert_eq!(
            select_browser_desktop_with_capacity(&routes, &[], 1, 0),
            Err("browser_runtime_config_display_density_invalid".to_string())
        );
    }
}
