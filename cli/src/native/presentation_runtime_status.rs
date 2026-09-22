//! Effective presentation readiness joins durable receipts with the live keeper owner.

use agent_browser_service_model::{BrowserDesktopRoute, RouteKeeperAuthority, RouteKeeperPhase};
use serde::Serialize;

use super::stream::RouteKeeperSupervisorHealth;

/// Diagnostic projection only; callers refresh the authority and supervisor probe
/// immediately before an effect. Persisted Ready records alone cannot admit work.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationKeeperStatus {
    pub schema_version: &'static str,
    pub supervisor: RouteKeeperSupervisorHealth,
    pub host_generation: Option<u64>,
    pub state: &'static str,
    pub ready_route_count: u32,
    pub configured_slot_count: u32,
    pub minimum_ready: u32,
    pub warm_target: u32,
    pub minimum_satisfied: bool,
    pub warm_target_satisfied: bool,
    pub unavailable_reason: Option<String>,
}

impl PresentationKeeperStatus {
    pub(crate) fn require_ready(&self) -> Result<(), String> {
        if self.state == "ready" && self.minimum_satisfied {
            Ok(())
        } else {
            Err(self
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "presentation_keeper_not_ready".to_string()))
        }
    }

    pub(crate) fn usable_routes(
        &self,
        authority: &RouteKeeperAuthority,
    ) -> Result<Vec<BrowserDesktopRoute>, String> {
        self.require_ready()?;
        current_routes(authority, self.host_generation)
    }
}

fn current_routes(
    authority: &RouteKeeperAuthority,
    host_generation: Option<u64>,
) -> Result<Vec<BrowserDesktopRoute>, String> {
    Ok(
        super::browser_session_host::route_keeper_desktop_routes(authority)?
            .into_iter()
            .filter(|route| {
                authority
                    .records
                    .get(&route.id)
                    .is_some_and(|record| Some(record.fence.host_generation) == host_generation)
            })
            .collect(),
    )
}

pub(crate) fn keeper_status(
    authority: &RouteKeeperAuthority,
    health: RouteKeeperSupervisorHealth,
    host_generation: Option<u64>,
) -> Result<PresentationKeeperStatus, String> {
    authority.projection()?;
    let quarantined = authority
        .records
        .values()
        .any(|record| record.phase == RouteKeeperPhase::Quarantined);
    let supervising = health == RouteKeeperSupervisorHealth::Supervising;
    let ready_route_count = if supervising && host_generation.is_some() && !quarantined {
        current_routes(authority, host_generation)?.len() as u32
    } else {
        0
    };
    let minimum_satisfied = ready_route_count >= authority.policy.minimum_ready;
    let (state, reason) = match &health {
        RouteKeeperSupervisorHealth::Failed { code }
        | RouteKeeperSupervisorHealth::Unavailable { code } => ("unavailable", Some(code.clone())),
        RouteKeeperSupervisorHealth::Stopped | RouteKeeperSupervisorHealth::Stopping => (
            "unavailable",
            Some("presentation_keeper_stopped".to_string()),
        ),
        _ if quarantined => (
            "quarantined",
            Some("presentation_keeper_quarantined".to_string()),
        ),
        RouteKeeperSupervisorHealth::Recovering => (
            "recovering",
            Some("presentation_keeper_recovering".to_string()),
        ),
        _ if host_generation.is_none() => (
            "unavailable",
            Some("presentation_keeper_owner_missing".to_string()),
        ),
        _ if minimum_satisfied => ("ready", None),
        _ if authority.records.values().any(|record| {
            matches!(
                record.phase,
                RouteKeeperPhase::Degraded | RouteKeeperPhase::RecoveryFailed
            )
        }) =>
        {
            ("degraded", Some("presentation_keeper_degraded".to_string()))
        }
        _ => (
            "recovering",
            Some("presentation_keeper_minimum_pending".to_string()),
        ),
    };
    Ok(PresentationKeeperStatus {
        schema_version: "agent-browser.presentation-keeper-status.v1",
        supervisor: health,
        host_generation,
        state,
        ready_route_count,
        configured_slot_count: authority.connection_catalog.bindings.len() as u32,
        minimum_ready: authority.policy.minimum_ready,
        warm_target: authority.policy.warm_target,
        minimum_satisfied,
        warm_target_satisfied: ready_route_count >= authority.policy.warm_target,
        unavailable_reason: reason,
    })
}

pub(crate) fn unavailable_status(code: &str) -> PresentationKeeperStatus {
    PresentationKeeperStatus {
        schema_version: "agent-browser.presentation-keeper-status.v1",
        supervisor: RouteKeeperSupervisorHealth::Unavailable {
            code: code.to_string(),
        },
        host_generation: None,
        state: "unavailable",
        ready_route_count: 0,
        configured_slot_count: 0,
        minimum_ready: 0,
        warm_target: 0,
        minimum_satisfied: false,
        warm_target_satisfied: false,
        unavailable_reason: Some(code.to_string()),
    }
}
