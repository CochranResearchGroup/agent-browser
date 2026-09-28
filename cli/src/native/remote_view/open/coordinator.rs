//! Authenticated manual-seeding dispatch into the SQLite browser runtime.

use serde_json::Value;

use crate::native::action_runtime::runtime::DaemonState;

/// Transport-neutral attribution supplied after the daemon has authenticated
/// a route-bound request. Handoff intent never supplies these authority facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteBoundOpenAttribution {
    pub(crate) caller_id: Option<String>,
    pub(crate) service_job_id: Option<String>,
    pub(crate) dashboard_deployment_generation: Option<String>,
    pub(crate) service_principal_id: Option<String>,
    pub(crate) service_principal_provenance: Option<String>,
    pub(crate) authorization: RouteBoundOpenAuthorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteBoundOpenAuthorization {
    AuthenticatedDaemonCommand,
}

impl RouteBoundOpenAuthorization {
    pub(crate) fn is_authorized(self) -> bool {
        matches!(self, Self::AuthenticatedDaemonCommand)
    }
}

/// Construct attribution only at the authenticated command boundary.
pub(crate) fn route_bound_open_attribution_from_authenticated_dispatch(
    cmd: &Value,
) -> RouteBoundOpenAttribution {
    fn optional_string(cmd: &Value, key: &str) -> Option<String> {
        cmd.get(key)
            .or_else(|| cmd.get("params").and_then(|params| params.get(key)))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }
    RouteBoundOpenAttribution {
        caller_id: optional_string(cmd, "callerId"),
        service_job_id: optional_string(cmd, "serviceJobId"),
        dashboard_deployment_generation: optional_string(cmd, "dashboardDeploymentGeneration"),
        service_principal_id: optional_string(cmd, "servicePrincipalId"),
        service_principal_provenance: optional_string(cmd, "servicePrincipalProvenance"),
        authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
    }
}

/// Acquire one SQLite-owned, CDP-free manual-seeding browser. Publication
/// returns only the durable opaque handoff after fresh presentation proof.
pub(crate) async fn handle_service_profile_manual_seeding_acquire(
    cmd: &Value,
    state: &mut DaemonState,
    attribution: RouteBoundOpenAttribution,
) -> Result<Value, String> {
    super::manual_seeding_sqlite::handle_sqlite_manual_seeding_acquire(cmd, state, attribution)
        .await
}

/// Close the exact SQLite-owned detached process and handoff while retaining
/// the provider-owned keeper route.
pub(crate) async fn handle_service_profile_manual_seeding_close(
    cmd: &Value,
    _state: &DaemonState,
) -> Result<Value, String> {
    super::manual_seeding_sqlite::handle_sqlite_manual_seeding_close(cmd).await
}
