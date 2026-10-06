//! Deterministic selection of Remote View-owned desktops for Agent Browser.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::RemoteViewFixedDesktop;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewDesktopCandidate {
    pub desktop: RemoteViewFixedDesktop,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewDesktopSelection {
    pub desktop: RemoteViewFixedDesktop,
    pub live_browser_count: usize,
}

/// Select the least-crowded ready desktop in Remote View observation order.
///
/// Remote View owns readiness and desktop identity. Agent Browser supplies only
/// its current browser-to-desktop associations and owns the resulting choice.
pub fn select_least_crowded_remote_view_desktop(
    candidates: &[RemoteViewDesktopCandidate],
    live_browser_desktop_ids: &[String],
) -> Result<RemoteViewDesktopSelection, String> {
    let mut desktop_ids = HashSet::new();
    let mut route_labels = HashSet::new();
    for candidate in candidates.iter().filter(|candidate| candidate.ready) {
        let desktop = &candidate.desktop;
        if !desktop_ids.insert(desktop.desktop_id.as_str()) {
            return Err(format!(
                "remote_view_desktop_id_duplicate:{}",
                desktop.desktop_id
            ));
        }
        if !route_labels.insert(desktop.friendly_route_label.as_str()) {
            return Err(format!(
                "remote_view_route_label_duplicate:{}",
                desktop.friendly_route_label
            ));
        }
    }

    let mut counts = HashMap::<&str, usize>::new();
    for desktop_id in live_browser_desktop_ids {
        *counts.entry(desktop_id.as_str()).or_default() += 1;
    }

    candidates
        .iter()
        .filter(|candidate| candidate.ready)
        .map(|candidate| RemoteViewDesktopSelection {
            desktop: candidate.desktop.clone(),
            live_browser_count: counts
                .get(candidate.desktop.desktop_id.as_str())
                .copied()
                .unwrap_or_default(),
        })
        .min_by_key(|selection| selection.live_browser_count)
        .ok_or_else(|| "remote_view_desktop_unavailable".to_string())
}
