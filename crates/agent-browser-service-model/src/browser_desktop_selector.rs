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
