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
    /// Resolve a profile identity query without launching or mutating a browser.
    pub(crate) async fn handle_service_profile_lookup(cmd: &Value) -> Result<Value, String> {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        service_state.refresh_profile_readiness();
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        for (field, parameter) in [
            ("serviceName", "serviceName"),
            ("targetServiceId", "targetServiceId"),
            ("siteId", "siteId"),
            ("loginId", "loginId"),
            ("accountId", "accountId"),
            ("profileId", "profileId"),
            ("profileName", "profileName"),
            ("hostname", "hostname"),
            ("authenticationState", "authenticationState"),
            ("freshnessState", "freshnessState"),
            ("tag", "tag"),
            ("query", "query"),
            ("url", "url"),
            ("readinessProfileId", "readinessProfileId"),
            ("browserBuild", "browserBuild"),
        ] {
            if let Some(value) = cmd.get(field).and_then(Value::as_str) {
                query.append_pair(parameter, value);
            }
        }
        let query = query.finish();
        stream::service_profile_lookup_response_for_state(
            (!query.is_empty()).then_some(query.as_str()),
            &service_state,
        )
    }
    pub(crate) async fn handle_service_profile_seeding_handoff(
        cmd: &Value,
    ) -> Result<Value, String> {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        service_state.refresh_profile_readiness();
        let profile_id = cmd
            .get("profileId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "service_profile_seeding_handoff requires profileId".to_string())?;
        let target_service_id = cmd
            .get("targetServiceId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty());
        service_profile_seeding_handoff(&service_state, profile_id, target_service_id)
    }
    /// Return the service-owned session collection without the full status payload.
    pub(crate) async fn handle_service_sessions(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let mut sessions = service_state.sessions.into_values().collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.id.cmp(&right.id));
        let count = sessions.len();
        Ok(json!({ "sessions" : sessions, "count" : count, }))
    }
    /// Return the service-owned browser collection without the full status payload.
    pub(crate) async fn handle_service_browsers(cmd: &Value) -> Result<Value, String> {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        service_state.refresh_service_tab_handles();
        let mut browsers = service_state.browsers.into_values().collect::<Vec<_>>();
        browsers.sort_by(|left, right| left.id.cmp(&right.id));
        let count = browsers.len();
        Ok(json!({ "browsers" : browsers, "count" : count, }))
    }
    /// Return the service-owned tab collection without the full status payload.
    pub(crate) async fn handle_service_tabs(cmd: &Value) -> Result<Value, String> {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        service_state.refresh_service_tab_handles();
        let mut tabs = service_state.tabs.into_values().collect::<Vec<_>>();
        tabs.sort_by(|left, right| left.id.cmp(&right.id));
        let count = tabs.len();
        Ok(json!({ "tabs" : tabs, "count" : count, }))
    }
    /// Return the service-owned monitor collection without the full status payload.
    pub(crate) async fn handle_service_monitors(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        let state = optional_command_string(cmd, "monitorState")
            .map(|state| {
                parse_monitor_state(&state).ok_or_else(|| format!("Invalid monitor state: {state}"))
            })
            .transpose()?;
        let filters = MonitorCollectionFilters {
            state,
            failed_only: cmd
                .get("failedOnly")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            summary: cmd
                .get("summary")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        };
        Ok(service_monitors_response(&service_state, filters))
    }

    pub(crate) async fn handle_service_site_policies(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|error| format!("Invalid serviceState: {error}"))?
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
        Ok(json!({
            "sitePolicies": site_policies,
            "sitePolicySources": site_policy_sources,
            "count": count,
        }))
    }

    pub(crate) async fn handle_service_providers(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|error| format!("Invalid serviceState: {error}"))?
            .unwrap_or_default();
        let mut providers = service_state.providers.into_values().collect::<Vec<_>>();
        providers.sort_by(|left, right| left.id.cmp(&right.id));
        let count = providers.len();
        Ok(json!({ "providers": providers, "count": count }))
    }

    pub(crate) async fn handle_service_challenges(cmd: &Value) -> Result<Value, String> {
        let service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|error| format!("Invalid serviceState: {error}"))?
            .unwrap_or_default();
        let mut challenges = service_state.challenges.into_values().collect::<Vec<_>>();
        challenges.sort_by(|left, right| left.id.cmp(&right.id));
        let count = challenges.len();
        Ok(json!({ "challenges": challenges, "count": count }))
    }
}
pub(crate) use service_commands::*;
