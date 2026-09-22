//! Ordinary browser allocation from live keeper routes and SQLite manager state.

use std::collections::BTreeSet;

use agent_browser_service_model::{BrowserDesktopRoute, BrowserSessionState};
use serde::Serialize;
use serde_json::Value;

use super::browser_session_store::{BrowserRuntimeConfig, BrowserRuntimeSqliteStore};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationAllocationStatus {
    pub state: &'static str,
    pub browser_count: usize,
    pub occupied_display_count: usize,
    pub maximum_displays: u32,
    pub maximum_browsers_per_display: u32,
}

fn occupied_displays(state: &BrowserSessionState) -> Vec<String> {
    state
        .browsers
        .values()
        .filter_map(|browser| {
            browser
                .desktop
                .as_ref()
                .map(|desktop| desktop.display_name.clone())
        })
        .collect()
}

pub(crate) fn allocation_status(
    state: &BrowserSessionState,
    config: &BrowserRuntimeConfig,
    routes: &[BrowserDesktopRoute],
) -> Result<PresentationAllocationStatus, String> {
    let displays = occupied_displays(state);
    let disposition = match agent_browser_service_model::select_browser_desktop_with_capacity(
        routes,
        &displays,
        config.maximum_displays,
        config.maximum_browsers_per_display,
    ) {
        Ok(_) => "available",
        Err(error) if error == "presentation_capacity_pending" => "pending",
        Err(error) if error == "presentation_capacity_full" => "full",
        Err(error) if error == "presentation_capacity_over_target" => "over_target",
        Err(error) if error == "browser_desktop_route_unavailable" => "unavailable",
        Err(error) => return Err(error),
    };
    Ok(PresentationAllocationStatus {
        state: disposition,
        browser_count: displays.len(),
        occupied_display_count: displays.iter().collect::<BTreeSet<_>>().len(),
        maximum_displays: config.maximum_displays,
        maximum_browsers_per_display: config.maximum_browsers_per_display,
    })
}

/// Existing browser reuse does not consume another display or density unit.
/// The host validates profile/session identity again under its effect lock.
pub(crate) fn command_reuses_browser(command: &Value, state: &BrowserSessionState) -> bool {
    let field = |key| {
        command
            .get(key)
            .or_else(|| command.get("params").and_then(|params| params.get(key)))
            .and_then(Value::as_str)
    };
    if let Some(session_id) = field("sessionId") {
        if state
            .sessions
            .get(session_id)
            .is_some_and(|session| state.browsers.contains_key(&session.browser_id))
        {
            return true;
        }
    }
    field("profileId").is_some_and(|profile_id| {
        state
            .browsers
            .values()
            .any(|browser| browser.profile_id == profile_id)
    })
}

/// Publish only additional protocol-route demand. The supervisor remains the
/// sole owner of provider effects; waiting clients never launch a keeper directly.
pub(crate) fn request_additional_route(
    store: &mut BrowserRuntimeSqliteStore,
    occupied_display_count: usize,
    maximum_displays: u32,
) -> Result<(), String> {
    for _ in 0..3 {
        let current = store.load_route_keeper_authority()?;
        let desired = u32::try_from(occupied_display_count)
            .unwrap_or(u32::MAX)
            .saturating_add(1)
            .min(maximum_displays)
            .min(current.policy.maximum_slots);
        if desired <= current.requested_ready_slots {
            return Ok(());
        }
        let mut next = current.clone();
        next.request_ready_slots(desired)?;
        match store.compare_and_swap_route_keeper_authority(&current, &next) {
            Ok(()) => return Ok(()),
            Err(error) if error == "route_keeper_authority_compare_and_swap_conflict" => {}
            Err(error) => return Err(error),
        }
    }
    Err("presentation_capacity_demand_conflict".to_string())
}
