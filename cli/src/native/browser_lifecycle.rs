#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::browser::{
        should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo,
        ProcessExitObservation, WaitUntil,
    };
    use crate::native::remote_view_handoff::{
        apply_retained_remote_view_route, begin_route_bound_handoff_failure_recovery,
        begin_route_bound_handoff_plan_acquisition, complete_route_bound_handoff_failure_cleanup,
        complete_route_bound_handoff_open, planned_route_bound_handoff_response,
        remote_view_handoff_resolution_command, remote_view_handoff_was_explicitly_closed,
        route_bound_handoff_checkout_command_with_visible_window_proof,
        route_bound_handoff_checkout_failure, route_bound_handoff_failure_cleanup_task_result,
        route_bound_handoff_focus_command, route_bound_handoff_focus_failure,
        route_bound_handoff_immediate_failure, route_bound_handoff_launch_failure_cleanup,
        route_bound_handoff_operator_visible,
        route_bound_handoff_operator_visible_failure_if_not_ready, route_bound_handoff_plan,
        route_bound_handoff_post_checkout_proof, route_bound_handoff_pre_launch_failure_cleanup,
        route_bound_handoff_reused_browser_launch_result, route_bound_handoff_tab_open_failure,
        route_bound_handoff_target_url_readiness, route_bound_handoff_visible_window_proof_failure,
        shared_profile_acquisition_result, CompleteRouteBoundHandoffOpenInput,
        RouteBoundHandoffFailureCleanupInput, RouteBoundHandoffFailureCleanupSummary,
        RouteBoundHandoffFailureCleanupTask, RouteBoundHandoffFailureRecoveryInput,
        RouteBoundHandoffImmediateFailureInput, RouteBoundHandoffPlan,
        RouteBoundHandoffPlannedResponseInput, RouteBoundHandoffPostCheckoutProofInput,
        SharedProfileAcquisitionResultInput,
    };
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
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    use crate::native::state;
    use serde_json::{json, Map, Value};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};
    pub(crate) fn persist_service_owned_tab_new(
        cmd: &Value,
        session_id: &str,
        target_id: Option<&str>,
        url: Option<&str>,
        title: Option<&str>,
        service_tab_handle: &Value,
    ) -> Result<(), String> {
        let Some(target_id) = target_id else {
            return Ok(());
        };
        let handle: ServiceTabHandle = serde_json::from_value(service_tab_handle.clone())
            .map_err(|err| format!("Invalid service tab handle: {}", err))?;
        let repository = LockedServiceStateRepository::default_json()?;
        let browser_id = service_browser_id(session_id);
        let tab_id = format!("target:{target_id}");
        let observed_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        let service_name = optional_command_string(cmd, "serviceName");
        let agent_name = optional_command_string(cmd, "agentName");
        let task_name = optional_command_string(cmd, "taskName");
        repository.mutate(|state| {
            state.tabs.insert(
                tab_id.clone(),
                BrowserTab {
                    id: tab_id.clone(),
                    browser_id: browser_id.clone(),
                    target_id: Some(target_id.to_string()),
                    session_id: Some(session_id.to_string()),
                    lifecycle: TabLifecycle::Ready,
                    url: url.map(str::to_string),
                    title: title.filter(|value| !value.is_empty()).map(str::to_string),
                    owner_session_id: Some(session_id.to_string()),
                    service_tab_handle: Some(handle.clone()),
                    ..BrowserTab::default()
                },
            );
            if let Some(session) = state.sessions.get_mut(session_id) {
                if !session.tab_ids.contains(&tab_id) {
                    session.tab_ids.push(tab_id.clone());
                }
            }
            if let Some(browser) = state.browsers.get_mut(&browser_id) {
                if !browser.active_session_ids.iter().any(|id| id == session_id) {
                    browser.active_session_ids.push(session_id.to_string());
                }
            }
            state.events.push(ServiceEvent {
                id: format!(
                    "service-tab-new-{}-{}",
                    tab_id.replace(':', "-"),
                    observed_at
                ),
                timestamp: observed_at.clone(),
                kind: ServiceEventKind::TabLifecycleChanged,
                message: format!("Service tab '{}' opened.", tab_id),
                browser_id: Some(browser_id),
                profile_id: handle.profile_id.clone(),
                session_id: Some(session_id.to_string()),
                service_name,
                agent_name,
                task_name,
                details: Some(json!(
                    { "action" : "tab_new", "targetId" : target_id, "tabId" :
                    tab_id, "url" : url, }
                )),
                ..ServiceEvent::default()
            });
            if state.events.len() > 100 {
                let excess = state.events.len() - 100;
                state.events.drain(0..excess);
            }
            Ok(())
        })
    }
    pub(crate) fn tab_new_shared_acquisition_evidence(
        cmd: &Value,
        session_id: &str,
        profile_id: Value,
    ) -> Value {
        let requested_browser_id = optional_command_string(cmd, "browserId");
        let requested_session_name = optional_command_string(cmd, "sessionName");
        let routed_browser_id = service_browser_id(session_id);
        let reused_browser = requested_browser_id
            .as_deref()
            .map(|browser_id| browser_id == routed_browser_id)
            .unwrap_or(false)
            || requested_session_name
                .as_deref()
                .map(|session_name| session_name == session_id)
                .unwrap_or(false);
        let route_hint_source = match (
            requested_browser_id.as_ref(),
            requested_session_name.as_ref(),
        ) {
            (Some(_), Some(_)) => "request.browserId_sessionName",
            (Some(_), None) => "request.browserId",
            (None, Some(_)) => "request.sessionName",
            (None, None) => "none",
        };
        let route_hint_fields: &[&str] = if route_hint_source == "none" {
            &[]
        } else {
            &["browserId", "sessionName"]
        };
        shared_profile_acquisition_result(SharedProfileAcquisitionResultInput {
            state: None,
            mode: "tab_new",
            action: "opened_new_tab",
            recommended_action: Some(if reused_browser {
                "reuse_existing_browser"
            } else {
                "open_shared_profile_tab"
            }),
            browser_reused: reused_browser,
            tab_opened: true,
            browser_id: &routed_browser_id,
            session_name: session_id,
            profile_id: Some(&profile_id),
            requested_profile: profile_id.as_str(),
            planned_profile: profile_id.as_str(),
            requested_browser_id: requested_browser_id.as_deref(),
            requested_session_name: requested_session_name.as_deref(),
            route_hint_source,
            route_hint_fields,
            route_bound: false,
            route_id: None,
            display_allocation_id: None,
            route_pool_entry_id: None,
            provider: None,
            provider_mode: None,
            tab_acquisition_decision: None,
        })
    }
    pub(crate) async fn handle_tab_switch(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        let index = cmd
            .get("index")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'index' parameter")? as usize;
        state.ref_map.clear();
        state.iframe_sessions.clear();
        state.active_frame_id = None;
        let result = mgr.tab_switch(index).await?;
        if let Some(ref server) = state.stream_server {
            if let Ok(dims) = mgr
                .evaluate(
                    "JSON.stringify([window.innerWidth,window.innerHeight])",
                    None,
                )
                .await
            {
                if let Some(s) = dims.get("result").and_then(|v| v.as_str()) {
                    if let Ok(arr) = serde_json::from_str::<Vec<u32>>(s) {
                        if arr.len() == 2 && arr[0] > 0 && arr[1] > 0 {
                            server.set_viewport(arr[0], arr[1]).await;
                        }
                    }
                }
            }
        }
        Ok(result)
    }
    pub(crate) async fn handle_tab_close(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        let index = cmd
            .get("index")
            .and_then(|v| v.as_u64())
            .map(|i| i as usize);
        state.ref_map.clear();
        state.iframe_sessions.clear();
        state.active_frame_id = None;
        mgr.tab_close(index).await
    }
    pub(crate) async fn handle_tab_handle_refresh(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let handle = cmd
            .get("serviceTabHandle")
            .and_then(Value::as_object)
            .ok_or_else(|| "tab_handle_refresh requires serviceTabHandle".to_string())?;
        let repair_policy = optional_command_string(cmd, "repairPolicy")
            .unwrap_or_else(|| "reject_only".to_string());
        if !matches!(
            repair_policy.as_str(),
            "reject_only" | "reuse_compatible" | "open_if_missing" | "replace_duplicates"
        ) {
            return Err(
                "tab_handle_refresh repairPolicy must be reject_only, reuse_compatible, open_if_missing, or replace_duplicates"
                    .to_string(),
            );
        }
        let observed_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        let browser_id = handle
            .get("browserId")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| service_browser_id(&state.session_id));
        let target_id = handle.get("targetId").and_then(Value::as_str);
        let requested_url = optional_command_string(cmd, "url")
            .or_else(|| optional_command_string(cmd, "desiredUrl"))
            .or_else(|| {
                handle
                    .get("url")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            });
        let desired_origin = requested_url.as_deref().and_then(origin_for_url);
        let mut candidates = retained_tab_handle_candidates(handle, requested_url.as_deref());
        let old_handle_valid =
            validate_service_tab_handle_for_current_session(handle, &state.session_id)
                .map(|_| true)
                .unwrap_or(false);
        let mgr = state.browser.as_mut().ok_or_else(|| {
            "Cannot refresh service tab handle: routed browser session is not running".to_string()
        })?;
        for page in mgr.pages_list() {
            let classification = classify_live_page_candidate(
                &page.target_id,
                page.url.as_str(),
                target_id,
                desired_origin.as_deref(),
            );
            candidates.push(json!(
                { "source" : "live_browser", "classification" : classification,
                "targetId" : page.target_id, "url" : page.url, "title" : page
                .title, }
            ));
        }
        if let Some(target_id) = target_id {
            if old_handle_valid || repair_policy != "reject_only" {
                if let Ok(mut switched) = mgr.tab_switch_target_id(target_id).await {
                    let url = mgr.get_url().await.unwrap_or_default();
                    let title = mgr.get_title().await.unwrap_or_default();
                    switched["refreshDecision"] = json!("exact_handle_still_valid");
                    let refreshed_handle = refreshed_service_tab_handle(
                        handle,
                        &state.session_id,
                        target_id,
                        url.as_str(),
                        title.as_str(),
                    );
                    persist_tab_handle_refresh_event(
                        cmd,
                        &browser_id,
                        refreshed_handle.get("profileId").and_then(Value::as_str),
                        "exact_handle_still_valid",
                        &observed_at,
                        &candidates,
                    )?;
                    let duplicate_target_cleanup = if repair_policy == "replace_duplicates" {
                        close_compatible_duplicate_targets(
                            mgr,
                            target_id,
                            Some(target_id),
                            desired_origin.as_deref(),
                        )
                        .await
                    } else {
                        no_duplicate_target_cleanup()
                    };
                    return Ok(json!(
                        { "ok" : true, "action" : "tab_handle_refresh", "refreshed" :
                        true, "decision" : "exact_handle_still_valid", "repairPolicy"
                        : repair_policy, "observedAt" : observed_at, "browserId" :
                        browser_id, "targetId" : target_id, "url" : url, "title" :
                        title, "tabSwitch" : switched, "serviceTabHandle" :
                        refreshed_handle, "duplicateTargetCleanup" :
                        duplicate_target_cleanup, "candidates" : candidates, }
                    ));
                }
            }
        }
        if repair_policy == "reject_only" {
            persist_tab_handle_refresh_event(
                cmd,
                &browser_id,
                handle.get("profileId").and_then(Value::as_str),
                "rejected_stale_or_missing_target",
                &observed_at,
                &candidates,
            )?;
            return Ok(json!(
                { "ok" : false, "action" : "tab_handle_refresh", "refreshed" : false,
                "decision" : "rejected_stale_or_missing_target", "repairPolicy" :
                repair_policy, "observedAt" : observed_at, "browserId" : browser_id,
                "staleReason" : handle.get("staleReason").cloned()
                .unwrap_or(Value::Null), "serviceTabHandle" : cmd
                .get("serviceTabHandle").cloned().unwrap_or(Value::Null),
                "candidates" : candidates, }
            ));
        }
        if repair_policy == "reuse_compatible"
            || repair_policy == "open_if_missing"
            || repair_policy == "replace_duplicates"
        {
            if let Some(page) = mgr.pages_list().into_iter().find(|page| {
                classify_live_page_candidate(
                    &page.target_id,
                    page.url.as_str(),
                    target_id,
                    desired_origin.as_deref(),
                )
                .starts_with("compatible_")
            }) {
                let mut switched = mgr.tab_switch_target_id(&page.target_id).await?;
                let url = mgr.get_url().await.unwrap_or_default();
                let title = mgr.get_title().await.unwrap_or_default();
                switched["refreshDecision"] = json!("reused_compatible_target");
                let refreshed_handle = service_tab_handle_from_parts(
                    handle,
                    &state.session_id,
                    &page.target_id,
                    url.as_str(),
                    title.as_str(),
                );
                persist_tab_handle_refresh_event(
                    cmd,
                    &browser_id,
                    refreshed_handle.get("profileId").and_then(Value::as_str),
                    "reused_compatible_target",
                    &observed_at,
                    &candidates,
                )?;
                let duplicate_target_cleanup = if repair_policy == "replace_duplicates" {
                    close_compatible_duplicate_targets(
                        mgr,
                        &page.target_id,
                        target_id,
                        desired_origin.as_deref(),
                    )
                    .await
                } else {
                    no_duplicate_target_cleanup()
                };
                return Ok(json!(
                    { "ok" : true, "action" : "tab_handle_refresh", "refreshed" :
                    true, "decision" : "reused_compatible_target", "repairPolicy" :
                    repair_policy, "observedAt" : observed_at, "browserId" :
                    browser_id, "targetId" : page.target_id, "url" : url, "title" :
                    title, "tabSwitch" : switched, "serviceTabHandle" :
                    refreshed_handle, "duplicateTargetCleanup" :
                    duplicate_target_cleanup, "candidates" : candidates, }
                ));
            }
        }
        if repair_policy == "open_if_missing" || repair_policy == "replace_duplicates" {
            let open_url = requested_url.as_deref().unwrap_or("about:blank");
            let mut opened = mgr.tab_new(Some(open_url)).await?;
            let new_target_id = opened
                .get("targetId")
                .and_then(Value::as_str)
                .ok_or_else(|| "tab_handle_refresh opened a tab without targetId".to_string())?
                .to_string();
            let url = mgr.get_url().await.unwrap_or_else(|_| open_url.to_string());
            let title = mgr.get_title().await.unwrap_or_default();
            opened["refreshDecision"] = json!("opened_replacement_target");
            let refreshed_handle = service_tab_handle_from_parts(
                handle,
                &state.session_id,
                &new_target_id,
                &url,
                &title,
            );
            persist_tab_handle_refresh_event(
                cmd,
                &browser_id,
                refreshed_handle.get("profileId").and_then(Value::as_str),
                "opened_replacement_target",
                &observed_at,
                &candidates,
            )?;
            let duplicate_target_cleanup = if repair_policy == "replace_duplicates" {
                close_compatible_duplicate_targets(
                    mgr,
                    &new_target_id,
                    target_id,
                    desired_origin.as_deref(),
                )
                .await
            } else {
                no_duplicate_target_cleanup()
            };
            return Ok(json!(
                { "ok" : true, "action" : "tab_handle_refresh", "refreshed" : true,
                "decision" : "opened_replacement_target", "repairPolicy" :
                repair_policy, "observedAt" : observed_at, "browserId" : browser_id,
                "targetId" : new_target_id, "url" : url, "title" : title, "tabNew" :
                opened, "serviceTabHandle" : refreshed_handle,
                "duplicateTargetCleanup" : duplicate_target_cleanup, "candidates" :
                candidates, }
            ));
        }
        persist_tab_handle_refresh_event(
            cmd,
            &browser_id,
            handle.get("profileId").and_then(Value::as_str),
            "no_compatible_target",
            &observed_at,
            &candidates,
        )?;
        Ok(json!(
            { "ok" : false, "action" : "tab_handle_refresh", "refreshed" : false,
            "decision" : "no_compatible_target", "repairPolicy" : repair_policy,
            "observedAt" : observed_at, "browserId" : browser_id, "serviceTabHandle"
            : cmd.get("serviceTabHandle").cloned().unwrap_or(Value::Null),
            "candidates" : candidates, }
        ))
    }
    pub(crate) async fn handle_tab_handle_release(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let handle = cmd
            .get("serviceTabHandle")
            .and_then(Value::as_object)
            .ok_or_else(|| "tab_handle_release requires serviceTabHandle".to_string())?;
        validate_service_tab_handle_route_for_current_session(handle, &state.session_id)?;
        let physical_tab_close =
            release_physical_tab_for_handle(handle, state, cmd.get("closePhysicalTab")).await;
        let released_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        let repository = LockedServiceStateRepository::default_json()?;
        repository.mutate(|service_state| {
            release_service_tab_handle_record(
                service_state,
                handle,
                &state.session_id,
                &released_at,
                &physical_tab_close,
            )
        })
    }
    pub(crate) async fn release_physical_tab_for_handle(
        handle: &Map<String, Value>,
        state: &mut DaemonState,
        close_physical_tab: Option<&Value>,
    ) -> Value {
        let close_requested = close_physical_tab.and_then(Value::as_bool).unwrap_or(true);
        if !close_requested {
            return json!(
                { "attempted" : false, "closed" : false, "skippedReason" :
                "request_disabled_physical_close", "error" : Value::Null, "result" :
                Value::Null, }
            );
        }
        if handle.get("cleanupPolicy").and_then(Value::as_str) == Some("release_only") {
            return json!(
                { "attempted" : false, "closed" : false, "skippedReason" :
                "cleanup_policy_release_only", "error" : Value::Null, "result" :
                Value::Null, }
            );
        }
        let Some(target_id) = handle.get("targetId").and_then(Value::as_str) else {
            return json!(
                { "attempted" : false, "closed" : false, "skippedReason" :
                "missing_target_id", "error" : Value::Null, "result" : Value::Null, }
            );
        };
        let Some(mgr) = state.browser.as_mut() else {
            return json!(
                { "attempted" : false, "closed" : false, "skippedReason" :
                "no_live_browser", "error" : Value::Null, "result" : Value::Null, }
            );
        };
        match mgr.tab_close_target_id_for_release(target_id).await {
            Ok(result) => {
                let closed = result
                    .get("closeCommandAcknowledged")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                json!(
                    { "attempted" : true, "closed" : closed, "skippedReason" :
                    (!closed).then_some("physical_close_failed"), "error" : result
                    .get("closeCommandError").cloned().unwrap_or(Value::Null), "result" :
                    result, }
                )
            }
            Err(error) => {
                let skipped_reason = if error.contains("Cannot close the last tab") {
                    "last_tab_preserved"
                } else if error.contains("was not found in the attached tab list") {
                    "target_not_attached"
                } else {
                    "physical_close_failed"
                };
                json!(
                    { "attempted" : true, "closed" : false, "skippedReason" :
                    skipped_reason, "error" : error, "result" : Value::Null, }
                )
            }
        }
    }
    pub(crate) fn release_service_tab_handle_record(
        service_state: &mut ServiceState,
        handle: &Map<String, Value>,
        routed_session_id: &str,
        released_at: &str,
        physical_tab_close: &Value,
    ) -> Result<Value, String> {
        let tab_id = handle
            .get("tabId")
            .and_then(Value::as_str)
            .ok_or_else(|| "serviceTabHandle.tabId is required".to_string())?;
        let browser_id = handle
            .get("browserId")
            .and_then(Value::as_str)
            .ok_or_else(|| "serviceTabHandle.browserId is required".to_string())?;
        let session_name = handle
            .get("sessionName")
            .and_then(Value::as_str)
            .or_else(|| handle.get("ownerSessionId").and_then(Value::as_str))
            .unwrap_or(routed_session_id);
        let target_id = handle.get("targetId").cloned().unwrap_or(Value::Null);
        let cleanup_policy = handle.get("cleanupPolicy").cloned().unwrap_or(Value::Null);
        let before_lifecycle = service_state
            .tabs
            .get(tab_id)
            .map(|tab| serde_json::to_value(tab.lifecycle).unwrap_or_else(|_| json!("unknown")));
        let mut tab_released = false;
        let mut tab_missing = false;
        match service_state.tabs.get_mut(tab_id) {
            Some(tab) => {
                if tab.browser_id != browser_id {
                    return Err(
                        format!(
                            "service tab handle browserId {browser_id} does not match retained tab {} browserId {}",
                            tab.id, tab.browser_id
                        ),
                    );
                }
                tab.lifecycle = TabLifecycle::Closed;
                tab.service_tab_handle = None;
                tab_released = true;
            }
            None => {
                tab_missing = true;
            }
        }
        if let Some(session) = service_state.sessions.get_mut(session_name) {
            session.last_lease_observed_at = Some(released_at.to_string());
        }
        service_state.events.push(ServiceEvent {
            id: format!(
                "tab-handle-release-{}-{}",
                tab_id.replace(':', "-"),
                released_at
            ),
            timestamp: released_at.to_string(),
            kind: ServiceEventKind::TabLifecycleChanged,
            message: format!("Service tab handle '{}' released.", tab_id),
            browser_id: Some(browser_id.to_string()),
            profile_id: handle
                .get("profileId")
                .and_then(Value::as_str)
                .map(str::to_string),
            session_id: Some(session_name.to_string()),
            service_name: optional_command_string_from_handle_or_trace(handle, "serviceName"),
            agent_name: optional_command_string_from_handle_or_trace(handle, "agentName"),
            task_name: optional_command_string_from_handle_or_trace(handle, "taskName"),
            details: Some(json!(
                { "action" : "tab_handle_release", "tabId" : tab_id, "targetId" :
                target_id, "cleanupPolicy" : cleanup_policy, "physicalTabClose" :
                physical_tab_close, "browserProcessPreserved" : true,
                "sessionRoutePreserved" : true, "tabMissing" : tab_missing, }
            )),
            ..ServiceEvent::default()
        });
        if service_state.events.len() > 100 {
            let excess = service_state.events.len() - 100;
            service_state.events.drain(0..excess);
        }
        service_state.refresh_service_tab_handles();
        let released_handle = service_state
            .tabs
            .get(tab_id)
            .and_then(|tab| tab.service_tab_handle.clone());
        Ok(json!(
            { "ok" : true, "action" : "tab_handle_release", "released" : true,
            "tabReleased" : tab_released, "tabMissing" : tab_missing,
            "browserProcessPreserved" : true, "sessionRoutePreserved" : true,
            "closeBrowserOnRelease" : false, "physicalTabClose" : physical_tab_close,
            "physicalTabCloseAttempted" : physical_tab_close.get("attempted")
            .cloned().unwrap_or(Value::Bool(false)), "physicalTabClosed" :
            physical_tab_close.get("closed").cloned().unwrap_or(Value::Bool(false)),
            "physicalTabCloseSkippedReason" : physical_tab_close.get("skippedReason")
            .cloned().unwrap_or(Value::Null), "browserId" : browser_id, "sessionName"
            : session_name, "tabId" : tab_id, "targetId" : target_id, "cleanupPolicy"
            : cleanup_policy, "beforeLifecycle" : before_lifecycle
            .unwrap_or(Value::Null), "afterLifecycle" : if tab_released {
            json!("closed") } else { Value::Null }, "serviceTabHandle" :
            released_handle, "releasedAt" : released_at, }
        ))
    }
    pub(crate) fn optional_command_string_from_handle_or_trace(
        handle: &Map<String, Value>,
        key: &str,
    ) -> Option<String> {
        handle
            .get(key)
            .and_then(Value::as_str)
            .or_else(|| {
                handle
                    .get("traceFilter")
                    .and_then(Value::as_object)
                    .and_then(|trace_filter| trace_filter.get(key))
                    .and_then(Value::as_str)
            })
            .map(str::to_string)
    }
    pub(crate) fn service_tab_handle_from_parts(
        previous: &Map<String, Value>,
        session_id: &str,
        target_id: &str,
        url: &str,
        title: &str,
    ) -> Value {
        let tab_id = format!("target:{target_id}");
        let profile_id = previous.get("profileId").cloned().unwrap_or(Value::Null);
        json!(
            { "browserId" : service_browser_id(session_id), "sessionName" : session_id,
            "tabId" : tab_id, "targetId" : target_id, "url" : url, "title" : title,
            "profileId" : profile_id.clone(), "profileOrigin" : previous
            .get("profileOrigin").cloned().unwrap_or_else(||
            json!("agent_browser_owned")), "leaseId" : previous.get("leaseId").cloned()
            .unwrap_or_else(|| json!(session_id)), "leaseState" : previous
            .get("leaseState").cloned().unwrap_or_else(|| json!("shared")),
            "cleanupPolicy" : previous.get("cleanupPolicy").cloned().unwrap_or_else(||
            json!("detach")), "leaseHeartbeatExpected" : previous
            .get("leaseHeartbeatExpected").and_then(Value::as_bool).unwrap_or(true),
            "ownerSessionId" : previous.get("ownerSessionId").cloned().unwrap_or_else(||
            json!(session_id)), "jobId" : previous.get("jobId").cloned()
            .unwrap_or(Value::Null), "traceFilter" : { "browserId" :
            service_browser_id(session_id), "profileId" : profile_id, "sessionId" :
            session_id, }, "valid" : true, "staleReason" : Value::Null, }
        )
    }
    pub(crate) fn refreshed_service_tab_handle(
        previous: &Map<String, Value>,
        session_id: &str,
        target_id: &str,
        url: &str,
        title: &str,
    ) -> Value {
        let mut refreshed =
            service_tab_handle_from_parts(previous, session_id, target_id, url, title);
        if let Some(tab_id) = previous.get("tabId") {
            refreshed["tabId"] = tab_id.clone();
        }
        refreshed
    }
    pub(crate) fn retained_tab_handle_candidates(
        handle: &Map<String, Value>,
        desired_url: Option<&str>,
    ) -> Vec<Value> {
        let mut service_state = LockedServiceStateRepository::default_json()
            .and_then(|repository| repository.load_snapshot())
            .unwrap_or_default();
        service_state.refresh_service_tab_handles();
        let handle_tab_id = handle.get("tabId").and_then(Value::as_str);
        let handle_target_id = handle.get("targetId").and_then(Value::as_str);
        let desired_origin = desired_url
            .or_else(|| handle.get("url").and_then(Value::as_str))
            .and_then(origin_for_url);
        service_state
            .tabs
            .values()
            .map(|tab| {
                let browser = service_state.browsers.get(&tab.browser_id);
                json!(
                    { "source" : "service_state", "classification" :
                    classify_retained_tab_candidate(tab, browser, handle_tab_id,
                    handle_target_id, desired_origin.as_deref()), "tabId" : tab.id,
                    "browserId" : tab.browser_id, "targetId" : tab.target_id, "url" : tab
                    .url, "title" : tab.title, "lifecycle" : tab.lifecycle,
                    "browserHealth" : browser.map(| browser | browser.health),
                    "serviceTabHandle" : tab.service_tab_handle, }
                )
            })
            .collect()
    }
    pub(crate) fn classify_retained_tab_candidate(
        tab: &BrowserTab,
        browser: Option<&BrowserProcess>,
        handle_tab_id: Option<&str>,
        handle_target_id: Option<&str>,
        desired_origin: Option<&str>,
    ) -> &'static str {
        if Some(tab.id.as_str()) == handle_tab_id {
            if tab.lifecycle == TabLifecycle::Closed {
                return "closed_tab";
            }
            if browser.is_none_or(|browser| browser.health != ServiceBrowserHealth::Ready) {
                return "dead_browser";
            }
            return "exact_handle";
        }
        if tab.target_id.as_deref().is_some() && tab.target_id.as_deref() == handle_target_id {
            return "matching_target";
        }
        if tab.lifecycle == TabLifecycle::Closed {
            return "closed_tab";
        }
        if browser.is_none_or(|browser| browser.health != ServiceBrowserHealth::Ready) {
            return "dead_browser";
        }
        if let Some(url) = tab.url.as_deref() {
            if is_blank_url(url) {
                return "compatible_blank_tab";
            }
            if desired_origin.is_some() && origin_for_url(url).as_deref() == desired_origin {
                return "compatible_same_origin_tab";
            }
        }
        "incompatible_tab"
    }
    pub(crate) fn classify_live_page_candidate(
        page_target_id: &str,
        page_url: &str,
        handle_target_id: Option<&str>,
        desired_origin: Option<&str>,
    ) -> &'static str {
        if Some(page_target_id) == handle_target_id {
            return "matching_target";
        }
        if is_blank_url(page_url) {
            return "compatible_blank_tab";
        }
        if desired_origin.is_some() && origin_for_url(page_url).as_deref() == desired_origin {
            return "compatible_same_origin_tab";
        }
        "incompatible_tab"
    }
    pub(crate) fn compatible_duplicate_live_pages(
        pages: &[PageInfo],
        selected_target_id: &str,
        handle_target_id: Option<&str>,
        desired_origin: Option<&str>,
    ) -> Vec<Value> {
        pages
            .iter()
            .filter_map(|page| {
                if page.target_id == selected_target_id {
                    return None;
                }
                let classification = classify_live_page_candidate(
                    &page.target_id,
                    page.url.as_str(),
                    handle_target_id,
                    desired_origin,
                );
                if !classification.starts_with("compatible_") {
                    return None;
                }
                Some(json!(
                    { "targetId" : page.target_id, "url" : page.url, "title" : page
                    .title, "classification" : classification, }
                ))
            })
            .collect()
    }
    pub(crate) fn no_duplicate_target_cleanup() -> Value {
        json!(
            { "policy" : "preserve", "attempted" : false, "closedCount" : 0,
            "closedTargets" : [], "failedTargets" : [], }
        )
    }
    pub(crate) async fn close_compatible_duplicate_targets(
        mgr: &mut BrowserManager,
        selected_target_id: &str,
        handle_target_id: Option<&str>,
        desired_origin: Option<&str>,
    ) -> Value {
        let duplicates = compatible_duplicate_live_pages(
            &mgr.pages_list(),
            selected_target_id,
            handle_target_id,
            desired_origin,
        );
        if duplicates.is_empty() {
            return json!(
                { "policy" : "replace_duplicates", "attempted" : true, "closedCount" : 0,
                "closedTargets" : [], "failedTargets" : [], }
            );
        }
        let mut closed_targets = Vec::new();
        let mut failed_targets = Vec::new();
        for duplicate in duplicates {
            let target_id = duplicate
                .get("targetId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if target_id.is_empty() {
                continue;
            }
            match mgr.tab_close_target_id(&target_id).await {
                Ok(result) => closed_targets.push(json!(
                    { "targetId" : target_id, "url" : duplicate.get("url")
                    .cloned().unwrap_or(Value::Null), "title" : duplicate
                    .get("title").cloned().unwrap_or(Value::Null),
                    "classification" : duplicate.get("classification").cloned()
                    .unwrap_or(Value::Null), "result" : result, }
                )),
                Err(error) => failed_targets.push(json!(
                    { "targetId" : target_id, "url" : duplicate.get("url")
                    .cloned().unwrap_or(Value::Null), "title" : duplicate
                    .get("title").cloned().unwrap_or(Value::Null),
                    "classification" : duplicate.get("classification").cloned()
                    .unwrap_or(Value::Null), "error" : error, }
                )),
            }
        }
        let _ = mgr.tab_switch_target_id(selected_target_id).await;
        json!(
            { "policy" : "replace_duplicates", "attempted" : true, "closedCount" :
            closed_targets.len(), "closedTargets" : closed_targets, "failedTargets" :
            failed_targets, }
        )
    }
    pub(crate) fn is_blank_url(url: &str) -> bool {
        let trimmed = url.trim();
        trimmed.is_empty() || trimmed == "about:blank"
    }
    pub(crate) fn origin_for_url(url: &str) -> Option<String> {
        let trimmed = url.trim();
        if let Some(rest) = trimmed.strip_prefix("https://") {
            return rest
                .split('/')
                .next()
                .filter(|host| !host.is_empty())
                .map(|host| format!("https://{}", host.to_ascii_lowercase()));
        }
        if let Some(rest) = trimmed.strip_prefix("http://") {
            return rest
                .split('/')
                .next()
                .filter(|host| !host.is_empty())
                .map(|host| format!("http://{}", host.to_ascii_lowercase()));
        }
        None
    }
    pub(crate) fn persist_tab_handle_refresh_event(
        cmd: &Value,
        browser_id: &str,
        profile_id: Option<&str>,
        decision: &str,
        observed_at: &str,
        candidates: &[Value],
    ) -> Result<(), String> {
        let repository = LockedServiceStateRepository::default_json()?;
        let event_id = format!("tab-handle-refresh-{}-{}", browser_id, observed_at);
        let service_name = optional_command_string(cmd, "serviceName");
        let agent_name = optional_command_string(cmd, "agentName");
        let task_name = optional_command_string(cmd, "taskName");
        repository.mutate(|state| {
            state.events.push(ServiceEvent {
                id: event_id.clone(),
                timestamp: observed_at.to_string(),
                kind: ServiceEventKind::TabLifecycleChanged,
                message: format!("Service tab handle refresh {decision}."),
                browser_id: Some(browser_id.to_string()),
                profile_id: profile_id.map(ToString::to_string),
                session_id: optional_command_string(cmd, "sessionName"),
                service_name,
                agent_name,
                task_name,
                details: Some(json!(
                    { "action" : "tab_handle_refresh", "decision" : decision,
                    "repairPolicy" : cmd.get("repairPolicy").cloned()
                    .unwrap_or_else(|| json!("reject_only")), "targetId" : cmd
                    .get("targetId").cloned().unwrap_or(Value::Null),
                    "candidateCount" : candidates.len(), "candidates" :
                    candidates, }
                )),
                ..ServiceEvent::default()
            });
            if state.events.len() > 100 {
                let excess = state.events.len() - 100;
                state.events.drain(0..excess);
            }
            Ok(())
        })
    }
    pub(crate) async fn handle_view_focus(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
        state.ref_map.clear();
        state.iframe_sessions.clear();
        state.active_frame_id = None;
        let mut tab_switched = None;
        let fallback_index = cmd
            .get("index")
            .and_then(|v| v.as_u64())
            .map(|i| i as usize);
        if let Some(target_id) = cmd
            .get("targetId")
            .or_else(|| cmd.get("target_id"))
            .and_then(|v| v.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            let target_id = target_id.trim();
            if mgr.active_target_id().ok() == Some(target_id) {
                tab_switched = Some(json!({ "targetId" : target_id, "state" : "already_active", }));
            } else {
                match mgr.tab_switch_target_id(target_id).await {
                    Ok(value) => tab_switched = Some(value),
                    Err(target_err) => {
                        if let Some(index) = fallback_index {
                            let mut fallback = mgr.tab_switch(index).await?;
                            fallback["fallbackFromTargetId"] = json!(target_id);
                            fallback["fallbackReason"] = json!(target_err);
                            tab_switched = Some(fallback);
                        } else {
                            return Err(target_err);
                        }
                    }
                }
            }
        } else if let Some(index) = fallback_index {
            tab_switched = Some(mgr.tab_switch(index).await?);
        }
        let maximize = cmd
            .get("maximize")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mut result = if cmd
            .get("nativeFocusOnly")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            mgr.focus_native_window_for_view_only(maximize)
        } else if cmd
            .get("allowBringToFrontFailure")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            mgr.focus_for_view_allowing_bring_to_front_failure(maximize)
                .await?
        } else {
            mgr.focus_for_view(maximize).await?
        };
        if let Some(tab_switched) = tab_switched {
            result["tabSwitch"] = tab_switched;
        }
        Ok(result)
    }
    pub(crate) async fn handle_view_takeover(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let browser_id = optional_command_string(cmd, "browserId")
            .unwrap_or_else(|| service_browser_id(&state.session_id));
        let session_name =
            optional_command_string(cmd, "sessionName").unwrap_or_else(|| state.session_id.clone());
        let stream_id = optional_command_string(cmd, "streamId");
        let provider = optional_command_string(cmd, "provider");
        let open_mode =
            optional_command_string(cmd, "openMode").unwrap_or_else(|| "iframe".to_string());
        let reason = optional_command_string(cmd, "reason")
            .unwrap_or_else(|| "operator_request".to_string());
        let target_id = optional_command_string(cmd, "targetId");
        let tab_index = cmd.get("index").and_then(Value::as_u64);
        let provider_mode = match provider.as_deref() {
            Some("rdp_gateway" | "rdp-gateway") => "provider_single_view",
            Some(_) => "provider_multi_view",
            None => "provider_unknown",
        };
        let requested_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        let viewer_lease_id = format!(
            "viewer:{}:{}:{}",
            browser_id,
            stream_id.as_deref().unwrap_or("default"),
            requested_at
        );
        let service_event_id = persist_view_takeover_requested_event(
            &browser_id,
            &session_name,
            stream_id.as_deref(),
            provider.as_deref(),
            &open_mode,
            &reason,
            target_id.as_deref(),
            tab_index,
            &viewer_lease_id,
            &requested_at,
            cmd,
        )?;
        Ok(json!(
            { "status" : "accepted", "takeoverStatus" : "accepted",
            "takeoverRequested" : true, "reconnectRequested" : true, "browserId" :
            browser_id, "sessionName" : session_name, "streamId" : stream_id,
            "provider" : provider, "openMode" : open_mode, "reason" : reason,
            "targetId" : target_id, "index" : tab_index, "providerMode" :
            provider_mode, "viewerLeaseId" : viewer_lease_id, "lastViewerEvent" :
            "takeover_requested", "serviceEventId" : service_event_id,
            "browserProcessPreserved" : true, "requestedAt" : requested_at, }
        ))
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn persist_view_takeover_requested_event(
        browser_id: &str,
        session_name: &str,
        stream_id: Option<&str>,
        provider: Option<&str>,
        open_mode: &str,
        reason: &str,
        target_id: Option<&str>,
        tab_index: Option<u64>,
        viewer_lease_id: &str,
        requested_at: &str,
        cmd: &Value,
    ) -> Result<String, String> {
        let repository = LockedServiceStateRepository::default_json()?;
        let event_id = format!("viewer-takeover-{}-{}", browser_id, requested_at);
        let service_name = optional_command_string(cmd, "serviceName");
        let agent_name = optional_command_string(cmd, "agentName");
        let task_name = optional_command_string(cmd, "taskName");
        repository.mutate(|state| {
            let profile_id = state
                .browsers
                .get(browser_id)
                .and_then(|browser| browser.profile_id.clone());
            let event = ServiceEvent {
                id: event_id.clone(),
                timestamp: requested_at.to_string(),
                kind: ServiceEventKind::ViewerTakeoverRequested,
                message: format!(
                    "Viewer takeover requested for {} via {}.",
                    browser_id,
                    provider.unwrap_or("unknown_provider")
                ),
                browser_id: Some(browser_id.to_string()),
                profile_id,
                session_id: Some(session_name.to_string()),
                service_name,
                agent_name,
                task_name,
                details: Some(json!(
                    { "streamId" : stream_id, "provider" : provider, "openMode" :
                    open_mode, "reason" : reason, "targetId" : target_id, "index"
                    : tab_index, "viewerLeaseId" : viewer_lease_id,
                    "lastViewerEvent" : "takeover_requested", "takeoverStatus" :
                    "accepted", }
                )),
                ..ServiceEvent::default()
            };
            state.events.push(event);
            if state.events.len() > 100 {
                let excess = state.events.len() - 100;
                state.events.drain(0..excess);
            }
            Ok(())
        })?;
        Ok(event_id)
    }
}
pub(crate) use action_commands::*;
#[cfg(test)]
mod action_tests;
