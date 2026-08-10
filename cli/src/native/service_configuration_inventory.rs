#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::common::*;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    /// Return the service-owned site-policy collection without the full status payload.
    pub(crate) async fn handle_service_site_policies(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let site_policy_sources = cmd.get("sitePolicySources").cloned().unwrap_or_else(|| {
            json!(crate::native::service_model::service_site_policy_sources(
                &service_state
            ))
        });
        let mut site_policies = service_state
            .site_policies
            .into_values()
            .collect::<Vec<_>>();
        site_policies.sort_by(|left, right| left.id.cmp(&right.id));
        let count = site_policies.len();
        Ok(json!(
            { "sitePolicies" : site_policies, "sitePolicySources" :
            site_policy_sources, "count" : count, }
        ))
    }
    /// Return the service-owned provider collection without the full status payload.
    pub(crate) async fn handle_service_providers(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let mut providers = service_state.providers.into_values().collect::<Vec<_>>();
        providers.sort_by(|left, right| left.id.cmp(&right.id));
        let count = providers.len();
        Ok(json!({ "providers" : providers, "count" : count, }))
    }
    /// Return the service-owned challenge collection without the full status payload.
    pub(crate) async fn handle_service_challenges(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let mut challenges = service_state.challenges.into_values().collect::<Vec<_>>();
        challenges.sort_by(|left, right| left.id.cmp(&right.id));
        let count = challenges.len();
        Ok(json!({ "challenges" : challenges, "count" : count, }))
    }
}
pub(crate) use service_commands::*;
