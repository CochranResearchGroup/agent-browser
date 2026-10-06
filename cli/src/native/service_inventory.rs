#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::providers;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::service_model::{
        retained_display_allocation_candidates, service_profile_allocations,
        service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
        BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
        BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession,
        BrowserTab, ControlInputProvider, DisplayAllocation, JobState as ServiceJobState,
        LeaseState, MonitorState, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
        ProfileLeaseDisposition, ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease,
        RemoteViewHandoff, RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent,
        ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle,
        ViewStream, ViewStreamProvider, ViewerLease,
    };
    use crate::native::service_monitors::{
        parse_monitor_state, run_due_persisted_monitors, service_monitors_response,
        MonitorCollectionFilters,
    };
    use crate::native::state;
    use crate::native::stream;
    use serde_json::{json, Map, Value};
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
        for (browser_id, observation) in &service_state.protected_browser_owner_observations {
            let browser = service_state
                .browsers
                .get(browser_id)
                .ok_or_else(|| "protected_browser_owner_observation_invalid".to_string())?;
            crate::native::service_health::validate_protected_browser_owner_observation_for_inventory(
                observation,
                browser,
            )?;
        }
        if let Some(managed) = crate::native::browser_session_store::BrowserSessionSqliteStore::default_session_state_read_only()? {
            for browser in managed.browsers.values() {
                let record = managed_browser_inventory_record(browser);
                if let Some(existing) = service_state.browsers.get(&record.id) {
                    if existing.pid != record.pid || existing.cdp_endpoint != record.cdp_endpoint
                        || existing.profile_id != record.profile_id {
                        return Err("browser_inventory_identity_conflict".into());
                    }
                } else {
                    service_state.browsers.insert(record.id.clone(), record);
                }
            }
        }
        let protected_browser_owner_observations =
            service_state.protected_browser_owner_observations.clone();
        let mut browsers = service_state.browsers.into_values().collect::<Vec<_>>();
        browsers.sort_by(|left, right| left.id.cmp(&right.id));
        let count = browsers.len();
        Ok(json!({
            "browsers": browsers,
            "count": count,
            "protectedBrowserOwnerObservations": protected_browser_owner_observations,
        }))
    }
    /// Project the existing manager identity and endpoint, without assigning new
    /// launch authority or claiming current CDP or operator-presentation health.
    fn managed_browser_inventory_record(
        browser: &agent_browser_service_model::ManagedBrowserInstance,
    ) -> BrowserProcess {
        BrowserProcess {
            id: browser.id.clone(),
            profile_id: Some(browser.profile_id.clone()),
            pid: Some(browser.pid),
            cdp_endpoint: Some(browser.cdp_endpoint.clone()),
            active_session_ids: browser.active_session_ids.clone(),
            host: if browser.desktop.is_some() {
                ServiceBrowserHost::RemoteHeaded
            } else {
                ServiceBrowserHost::LocalHeaded
            },
            health: ServiceBrowserHealth::Reconnecting,
            ..BrowserProcess::default()
        }
    }

    #[cfg(test)]
    mod managed_inventory_tests {
        use super::*;
        #[test]
        fn managed_inventory_preserves_exact_owner_endpoint_and_does_not_claim_fresh_health() {
            let browser = agent_browser_service_model::ManagedBrowserInstance {
                id: "browser:reviewed:fixture".into(),
                profile_id: "reviewed".into(),
                pid: 4242,
                cdp_endpoint: "http://127.0.0.1:9222".into(),
                desktop: None,
                active_session_ids: vec!["session:reviewed:1".into()],
            };
            let record = managed_browser_inventory_record(&browser);
            assert_eq!(record.id, browser.id);
            assert_eq!(record.pid, Some(browser.pid));
            assert_eq!(
                record.cdp_endpoint.as_deref(),
                Some(browser.cdp_endpoint.as_str())
            );
            assert_eq!(record.active_session_ids, browser.active_session_ids);
            assert_eq!(record.health, ServiceBrowserHealth::Reconnecting);
            assert!(record.last_health_observation.is_none());
            assert!(record.record_provenance.is_none());
        }
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
