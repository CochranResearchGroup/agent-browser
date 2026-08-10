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
    use crate::native::remote_view::route_pool_repair::ServiceRoutePoolRepairOptions;
    use crate::native::service_diagnostics::truncate_utf8;
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct ServiceRetentionPruneOptions {
        pub(crate) apply: bool,
        pub(crate) closed_tabs: bool,
        pub(crate) not_started_browsers: bool,
        pub(crate) process_exited_browsers: bool,
        pub(crate) released_sessions: bool,
        pub(crate) abandoned_sessions: bool,
        pub(crate) orphaned_profiles: bool,
        pub(crate) display_allocations: bool,
        pub(crate) abandoned_session_min_age_minutes: u64,
    }
    impl ServiceRetentionPruneOptions {
        pub(crate) fn from_command(cmd: &Value) -> Self {
            Self {
                apply: cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
                closed_tabs: cmd
                    .get("closedTabs")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                not_started_browsers: cmd
                    .get("notStartedBrowsers")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                process_exited_browsers: cmd
                    .get("processExitedBrowsers")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                released_sessions: cmd
                    .get("releasedSessions")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                abandoned_sessions: cmd
                    .get("abandonedSessions")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                orphaned_profiles: cmd
                    .get("orphanedProfiles")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                display_allocations: cmd
                    .get("displayAllocations")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                abandoned_session_min_age_minutes: cmd
                    .get("abandonedSessionMinAgeMinutes")
                    .and_then(Value::as_u64)
                    .unwrap_or(1440),
            }
        }
    }
    pub(crate) async fn handle_service_prune_retained(cmd: &Value) -> Result<Value, String> {
        let options = ServiceRetentionPruneOptions::from_command(cmd);
        if options.apply {
            let repository = LockedServiceStateRepository::default_json()?;
            repository.mutate(|state| Ok(prune_retained_service_state(state, options)))
        } else {
            let mut service_state = cmd
                .get("serviceState")
                .cloned()
                .map(serde_json::from_value::<ServiceState>)
                .transpose()
                .map_err(|err| format!("Invalid serviceState: {}", err))?
                .unwrap_or_default();
            Ok(prune_retained_service_state(&mut service_state, options))
        }
    }
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct ServiceRetentionRepairOptions {
        pub(crate) apply: bool,
        pub(crate) missing_lease_observed_at: bool,
    }
    impl ServiceRetentionRepairOptions {
        pub(crate) fn from_command(cmd: &Value) -> Self {
            Self {
                apply: cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
                missing_lease_observed_at: cmd
                    .get("missingLeaseObservedAt")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            }
        }
    }
    pub(crate) async fn handle_service_repair_retained(cmd: &Value) -> Result<Value, String> {
        let options = ServiceRetentionRepairOptions::from_command(cmd);
        let observed_at = chrono::Utc::now().to_rfc3339();
        if options.apply {
            let repository = LockedServiceStateRepository::default_json()?;
            repository.mutate(|state| {
                Ok(repair_retained_service_state(
                    state,
                    options,
                    observed_at.as_str(),
                ))
            })
        } else {
            let mut service_state = cmd
                .get("serviceState")
                .cloned()
                .map(serde_json::from_value::<ServiceState>)
                .transpose()
                .map_err(|err| format!("Invalid serviceState: {}", err))?
                .unwrap_or_default();
            Ok(repair_retained_service_state(
                &mut service_state,
                options,
                observed_at.as_str(),
            ))
        }
    }
    pub(crate) fn repair_retained_service_state(
        state: &mut ServiceState,
        options: ServiceRetentionRepairOptions,
        observed_at: &str,
    ) -> Value {
        let before_session_count = state.sessions.len();
        let mut missing_lease_observed_at = Vec::new();
        let mut skipped = Vec::new();
        if options.missing_lease_observed_at {
            for session in state.sessions.values() {
                if session_has_parseable_age_evidence(session) {
                    continue;
                }
                if matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                    || !legacy_inert_session_placeholder(state, session)
                {
                    skipped.push(session.id.clone());
                    continue;
                }
                missing_lease_observed_at.push(session.id.clone());
            }
        }
        missing_lease_observed_at.sort();
        skipped.sort();
        let repaired_count = missing_lease_observed_at.len();
        if options.apply {
            for session_id in &missing_lease_observed_at {
                if let Some(session) = state.sessions.get_mut(session_id) {
                    session.last_lease_observed_at = Some(observed_at.to_string());
                }
            }
        }
        json!(
            { "repaired" : options.apply, "dryRun" : ! options.apply, "observedAt" :
            observed_at, "policy" : { "missingLeaseObservedAt" : options
            .missing_lease_observed_at, "requiresInertSessionPlaceholder" : true,
            "excludesReleasedOrExpiredSessions" : true, "stampSource" :
            "currentObservationTime", }, "before" : { "sessionCount" :
            before_session_count, }, "candidates" : { "missingLeaseObservedAt" :
            missing_lease_observed_at, }, "candidateCounts" : { "missingLeaseObservedAt"
            : repaired_count, "total" : repaired_count, }, "skipped" : {
            "missingLeaseObservedAt" : skipped, }, "skippedCounts" : {
            "missingLeaseObservedAt" : skipped.len(), }, "repairedCounts" : {
            "missingLeaseObservedAt" : if options.apply { repaired_count } else { 0 },
            "total" : if options.apply { repaired_count } else { 0 }, }, "after" : {
            "sessionCount" : state.sessions.len(), }, "recommendedNextStep" : if options
            .apply {
            "Run agent-browser service prune-retained --abandoned-sessions as a dry-run; repaired sessions should now be too fresh until the minimum age guard elapses."
            } else {
            "Review candidates, then rerun with --apply to stamp current observation time onto safe legacy placeholders."
            }, }
        )
    }
    pub(crate) async fn handle_service_route_pool_repair(cmd: &Value) -> Result<Value, String> {
        let options = ServiceRoutePoolRepairOptions::from_command(cmd);
        let observed_at = chrono::Utc::now().to_rfc3339();
        if options.apply {
            let repository = LockedServiceStateRepository::default_json()?;
            repository.mutate(|state| {
                Ok(repair_route_pool_service_state(
                    state,
                    options,
                    observed_at.as_str(),
                ))
            })
        } else {
            let mut service_state = if let Some(service_state) = cmd.get("serviceState") {
                serde_json::from_value::<ServiceState>(service_state.clone())
                    .map_err(|err| format!("Invalid serviceState: {}", err))?
            } else {
                LockedServiceStateRepository::default_json()?.load_snapshot()?
            };
            Ok(repair_route_pool_service_state(
                &mut service_state,
                options,
                observed_at.as_str(),
            ))
        }
    }
    pub(crate) fn repair_route_pool_service_state(
        state: &mut ServiceState,
        options: ServiceRoutePoolRepairOptions,
        observed_at: &str,
    ) -> Value {
        let before_route_pool_count = state.route_pool.len();
        let mut stale_checkouts = Vec::new();
        let mut stale_checkout_reasons = serde_json::Map::new();
        let mut skipped_active_checkouts = Vec::new();
        let mut stale_pending_acquisitions = Vec::new();
        let mut stale_pending_acquisition_reasons = serde_json::Map::new();
        let mut stale_route_ids = BTreeSet::new();
        let mut stale_display_allocation_ids = BTreeSet::new();
        if options.stale_pending_acquisitions {
            for lease in state.remote_view_acquisition_leases.values() {
                if lease.state != "pending" {
                    continue;
                }
                match pending_acquisition_stale_reason(state, lease) {
                    Some(reason) => {
                        stale_pending_acquisitions.push(lease.id.clone());
                        stale_pending_acquisition_reasons.insert(
                            lease.id.clone(),
                            json!(
                                { "reason" : reason, "routeId" : lease.route_id,
                                "displayAllocationId" : lease.display_allocation_id,
                                "routePoolEntryId" : lease.route_pool_entry_id, }
                            ),
                        );
                        stale_route_ids.insert(lease.route_id.clone());
                        stale_display_allocation_ids.insert(lease.display_allocation_id.clone());
                    }
                    None => skipped_active_checkouts.push(lease.id.clone()),
                }
            }
        }
        if options.stale_checkouts {
            for entry in state.route_pool.values() {
                if entry.state != "checked_out" {
                    continue;
                }
                let Some(route_id) = entry.current_route_allocation_id.as_deref() else {
                    stale_checkouts.push(entry.id.clone());
                    stale_checkout_reasons.insert(
                        entry.id.clone(),
                        json!({ "reason" : "missing_current_route_allocation_id", }),
                    );
                    continue;
                };
                match route_pool_checkout_stale_reason(state, route_id) {
                    Some(reason) => {
                        stale_checkouts.push(entry.id.clone());
                        if state.remote_view_routes.contains_key(route_id) {
                            stale_route_ids.insert(route_id.to_string());
                        }
                        stale_checkout_reasons.insert(
                            entry.id.clone(),
                            json!({ "reason" : reason, "routeId" : route_id, }),
                        );
                    }
                    None => skipped_active_checkouts.push(entry.id.clone()),
                }
            }
        }
        let stale_route_id_set = stale_route_ids.clone();
        for route_id in &stale_route_id_set {
            if let Some(display_allocation_id) = state
                .remote_view_routes
                .get(route_id)
                .and_then(|route| route.display_allocation_id.clone())
            {
                let referenced_by_active_route =
                    state.remote_view_routes.iter().any(|(id, route)| {
                        !stale_route_id_set.contains(id)
                            && route.display_allocation_id.as_deref()
                                == Some(display_allocation_id.as_str())
                            && matches!(
                                route.state.as_str(),
                                "ready" | "allocating" | "reconnecting"
                            )
                    });
                if !referenced_by_active_route {
                    stale_display_allocation_ids.insert(display_allocation_id);
                }
            }
        }
        stale_checkouts.sort();
        stale_pending_acquisitions.sort();
        skipped_active_checkouts.sort();
        let stale_routes = stale_route_ids.into_iter().collect::<Vec<_>>();
        let stale_display_allocations =
            stale_display_allocation_ids.into_iter().collect::<Vec<_>>();
        let repaired_count = stale_checkouts.len();
        let repaired_pending_count = stale_pending_acquisitions.len();
        let released_route_count = stale_routes.len();
        let released_display_allocation_count = stale_display_allocations.len();
        if options.apply {
            for lease_id in &stale_pending_acquisitions {
                if let Some(lease_snapshot) =
                    state.remote_view_acquisition_leases.get(lease_id).cloned()
                {
                    match lease_snapshot.previous_route_pool_entry.clone() {
                        Some(entry) => {
                            state.route_pool.insert(entry.id.clone(), entry);
                        }
                        None => {
                            if let Some(id) = lease_snapshot.route_pool_entry_id.as_ref() {
                                state.route_pool.remove(id);
                            }
                        }
                    }
                    match lease_snapshot.previous_display_allocation.clone() {
                        Some(allocation) => {
                            state
                                .display_allocations
                                .insert(allocation.id.clone(), allocation);
                        }
                        None => {
                            state
                                .display_allocations
                                .remove(&lease_snapshot.display_allocation_id);
                        }
                    }
                    match lease_snapshot.previous_remote_view_route.clone() {
                        Some(route) => {
                            state.remote_view_routes.insert(route.id.clone(), route);
                        }
                        None => {
                            state.remote_view_routes.remove(&lease_snapshot.route_id);
                        }
                    }
                    if let Some(browser) = state.browsers.get_mut(&lease_snapshot.browser_id) {
                        browser.display_allocation_id = lease_snapshot
                            .previous_browser_display_allocation_id
                            .clone();
                    }
                    let reason = stale_pending_acquisition_reasons
                        .get(lease_id)
                        .and_then(|value| value.get("reason"))
                        .and_then(Value::as_str)
                        .unwrap_or("stale_pending_acquisition");
                    let rollback = json!(
                        { "state" : "rolled_back", "leaseId" : lease_id, "phase" :
                        "stale_pending_acquisition_repair", "routeId" : lease_snapshot
                        .route_id, "displayAllocationId" : lease_snapshot
                        .display_allocation_id, "routePoolEntryId" : lease_snapshot
                        .route_pool_entry_id, "restoredRoutePoolEntry" : lease_snapshot
                        .previous_route_pool_entry.is_some(), "restoredDisplayAllocation"
                        : lease_snapshot.previous_display_allocation.is_some(),
                        "restoredRemoteViewRoute" : lease_snapshot
                        .previous_remote_view_route.is_some(),
                        "restoredBrowserDisplayAllocation" : lease_snapshot
                        .previous_browser_display_allocation_id, "cleanup" : { "state" :
                        "stale_pending_acquisition_repaired", "reason" : reason, },
                        "updatedAt" : observed_at, }
                    );
                    if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
                        lease.state = "failed".to_string();
                        lease.phase = "rollback_complete".to_string();
                        lease.updated_at = Some(observed_at.to_string());
                        lease.failed_at = Some(observed_at.to_string());
                        lease.failure_reason =
                            Some(format!("stale_pending_acquisition_repair: {reason}"));
                        lease.cleanup = Some(rollback);
                    }
                }
            }
            for entry_id in &stale_checkouts {
                if let Some(entry) = state.route_pool.get_mut(entry_id) {
                    let previous_route_allocation_id = entry.current_route_allocation_id.clone();
                    let reason = stale_checkout_reasons
                        .get(entry_id)
                        .and_then(|value| value.get("reason"))
                        .and_then(Value::as_str)
                        .unwrap_or("stale_route_pool_checkout");
                    entry.state = "available".to_string();
                    entry.current_route_allocation_id = None;
                    entry.readiness = Some(json!(
                        { "state" : "ready", "reason" :
                        "stale_route_pool_checkout_repaired", "staleReason" : reason,
                        "previousRouteAllocationId" : previous_route_allocation_id,
                        "updatedAt" : observed_at, }
                    ));
                }
            }
            for route_id in &stale_routes {
                if let Some(route) = state.remote_view_routes.get_mut(route_id) {
                    let previous_state = route.state.clone();
                    route.state = "released".to_string();
                    route.readiness = Some(json!(
                        { "state" : "released", "reason" :
                        "stale_route_pool_checkout_repaired", "previousState" :
                        previous_state, "updatedAt" : observed_at, }
                    ));
                }
            }
            for display_allocation_id in &stale_display_allocations {
                if let Some(allocation) = state.display_allocations.get_mut(display_allocation_id) {
                    let previous_state = allocation.state.clone();
                    allocation.state = "released".to_string();
                    allocation.updated_at = Some(observed_at.to_string());
                    allocation.readiness = Some(json!(
                        { "state" : "released", "reason" :
                        "stale_route_pool_checkout_repaired", "previousState" :
                        previous_state, "updatedAt" : observed_at, }
                    ));
                }
            }
        }
        let repaired_total = if options.apply {
            repaired_pending_count
                + repaired_count
                + released_route_count
                + released_display_allocation_count
        } else {
            0
        };
        let repaired_pending_total = if options.apply {
            repaired_pending_count
        } else {
            0
        };
        json!(
            { "repaired" : options.apply, "dryRun" : ! options.apply, "observedAt" :
            observed_at, "policy" : { "staleCheckouts" : options.stale_checkouts,
            "stalePendingAcquisitions" : options.stale_pending_acquisitions,
            "repairsCheckedOutEntriesOnly" : false, "preservesActiveReadyRoutes" : true,
            }, "before" : { "routePoolEntryCount" : before_route_pool_count, },
            "candidates" : { "stalePendingAcquisitions" : stale_pending_acquisitions,
            "staleCheckouts" : stale_checkouts, "staleRoutes" : stale_routes,
            "staleDisplayAllocations" : stale_display_allocations, }, "candidateReasons"
            : { "staleCheckouts" : stale_checkout_reasons, "stalePendingAcquisitions" :
            stale_pending_acquisition_reasons, }, "candidateCounts" : {
            "stalePendingAcquisitions" : repaired_pending_count, "staleCheckouts" :
            repaired_count, "staleRoutes" : released_route_count,
            "staleDisplayAllocations" : released_display_allocation_count, "total" :
            repaired_pending_count + repaired_count + released_route_count +
            released_display_allocation_count, }, "skipped" : { "activeCheckouts" :
            skipped_active_checkouts, }, "skippedCounts" : { "activeCheckouts" :
            skipped_active_checkouts.len(), }, "repairedCounts" : {
            "stalePendingAcquisitions" : repaired_pending_total, "staleCheckouts" : if
            options.apply { repaired_count } else { 0 },
            "staleRoutes" : if options.apply { released_route_count } else { 0 },
            "staleDisplayAllocations" : if options.apply {
            released_display_allocation_count } else { 0 }, "total" : repaired_total, },
            "after" : { "routePoolEntryCount" : state.route_pool.len(), },
            "recommendedNextStep" : if options.apply {
            "Run service_remote_view_route_checkout for the intended display allocations, then run service_reconcile to refresh derived remote-view incidents."
            } else {
            "Review stale checkout candidates, then rerun with apply=true to return those route-pool entries to available state."
            }, }
        )
    }
    pub(crate) fn pending_acquisition_stale_reason(
        state: &ServiceState,
        lease: &RemoteViewAcquisitionLease,
    ) -> Option<&'static str> {
        let route_pending = state
            .remote_view_routes
            .get(&lease.route_id)
            .map(|route| route.state == "pending")
            .unwrap_or(false);
        let display_pending = state
            .display_allocations
            .get(&lease.display_allocation_id)
            .map(|allocation| allocation.state == "pending")
            .unwrap_or(false);
        let pool_pending = lease.route_pool_entry_id.as_ref().is_some_and(|entry_id| {
            state
                .route_pool
                .get(entry_id)
                .map(|entry| {
                    entry.state == "pending"
                        && entry.current_route_allocation_id.as_deref()
                            == Some(lease.route_id.as_str())
                })
                .unwrap_or(false)
        });
        let browser_ready = state
            .browsers
            .get(&lease.browser_id)
            .map(|browser| browser.health == ServiceBrowserHealth::Ready)
            .unwrap_or(false);
        if !browser_ready && (route_pending || display_pending || pool_pending) {
            return Some("pending_acquisition_without_ready_browser");
        }
        None
    }
    pub(crate) fn route_pool_checkout_stale_reason(
        state: &ServiceState,
        route_id: &str,
    ) -> Option<&'static str> {
        let Some(route) = state.remote_view_routes.get(route_id) else {
            return Some("route_missing");
        };
        if matches!(
            route.state.as_str(),
            "released" | "orphaned" | "failed" | "unavailable"
        ) {
            return Some("route_not_active");
        }
        if let Some(display_allocation_id) = route.display_allocation_id.as_deref() {
            match state.display_allocations.get(display_allocation_id) {
                Some(allocation) if matches!(allocation.state.as_str(), "ready" | "allocating") => {
                }
                Some(_) => return Some("display_allocation_not_active"),
                None => return Some("display_allocation_missing"),
            }
        }
        if let Some(browser_id) = route.browser_id.as_deref() {
            match state.browsers.get(browser_id) {
                Some(browser) if browser.health == ServiceBrowserHealth::Ready => {}
                Some(_) => return Some("browser_not_ready"),
                None => return Some("browser_missing"),
            }
        }
        None
    }
    pub(crate) fn prune_retained_service_state(
        state: &mut ServiceState,
        options: ServiceRetentionPruneOptions,
    ) -> Value {
        let before_profile_count = state.profiles.len();
        let before_browser_count = state.browsers.len();
        let before_tab_count = state.tabs.len();
        let before_session_count = state.sessions.len();
        let before_display_allocation_count = state.display_allocations.len();
        let closed_tab_ids = if options.closed_tabs {
            state
                .tabs
                .iter()
                .filter(|(_, tab)| tab.lifecycle == TabLifecycle::Closed)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut skipped_abandoned_sessions_missing_age_timestamp = Vec::new();
        let mut skipped_abandoned_sessions_too_fresh = Vec::new();
        let session_ids = state
            .sessions
            .iter()
            .filter(|(_, session)| {
                let released_lease_matches = options.released_sessions
                    && matches!(session.lease, LeaseState::Released | LeaseState::Expired);
                let abandoned_age_status = abandoned_session_age_status(
                    session
                        .last_lease_observed_at
                        .as_deref()
                        .or(session.created_at.as_deref()),
                    options.abandoned_session_min_age_minutes,
                );
                let abandoned_lease_matches = options.abandoned_sessions
                    && !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                    && matches!(abandoned_age_status, SessionAgeStatus::OldEnough);
                let lease_matches = released_lease_matches || abandoned_lease_matches;
                let session_shape_matches = session.tab_ids.is_empty()
                    && !session.browser_ids.is_empty()
                    && session.browser_ids.iter().all(|browser_id| {
                        prunable_session_browser_placeholder(
                            state,
                            browser_id,
                            session.id.as_str(),
                            options.process_exited_browsers,
                        )
                    });
                if options.abandoned_sessions
                    && !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                    && session_shape_matches
                    && !abandoned_lease_matches
                {
                    match abandoned_age_status {
                        SessionAgeStatus::MissingOrInvalid => {
                            skipped_abandoned_sessions_missing_age_timestamp
                                .push(session.id.clone())
                        }
                        SessionAgeStatus::TooFresh => {
                            skipped_abandoned_sessions_too_fresh.push(session.id.clone())
                        }
                        SessionAgeStatus::OldEnough => {}
                    }
                }
                lease_matches && session_shape_matches
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let pruned_session_ids = session_ids.iter().cloned().collect::<HashSet<_>>();
        let browser_ids = state
            .browsers
            .iter()
            .filter(|(id, browser)| {
                let not_started_matches = options.not_started_browsers
                    && retained_not_started_browser_placeholder(state, id, browser);
                let process_exited_matches = options.process_exited_browsers
                    && retained_failed_browser_placeholder(state, id, browser);
                (not_started_matches || process_exited_matches)
                    && browser
                        .active_session_ids
                        .iter()
                        .all(|session_id| !state.sessions.contains_key(session_id))
            })
            .map(|(id, _)| id.clone())
            .chain(session_ids.iter().flat_map(|session_id| {
                state
                    .sessions
                    .get(session_id)
                    .map(|session| session.browser_ids.clone())
                    .unwrap_or_default()
            }))
            .collect::<Vec<_>>();
        let browser_ids = browser_ids.into_iter().collect::<HashSet<_>>();
        let mut browser_ids = browser_ids.into_iter().collect::<Vec<_>>();
        browser_ids.sort();
        let referenced_profile_ids = referenced_service_profile_ids(state);
        let mut orphaned_profile_reasons = serde_json::Map::new();
        let mut profile_ids = if options.orphaned_profiles {
            state
                .profiles
                .iter()
                .filter_map(|(profile_id, profile)| {
                    orphaned_profile_prune_reason(profile_id, profile, &referenced_profile_ids).map(
                        |reason| {
                            orphaned_profile_reasons.insert(
                                profile_id.clone(),
                                json!(
                                    { "reason" : reason, "userDataDir" : profile.user_data_dir,
                                    }
                                ),
                            );
                            profile_id.clone()
                        },
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        profile_ids.sort();
        let display_allocation_candidates = if options.display_allocations {
            retained_display_allocation_candidates(state)
        } else {
            Vec::new()
        };
        let display_allocation_ids = display_allocation_candidates
            .iter()
            .filter(|candidate| candidate.apply_safe)
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>();
        let mut display_allocation_reasons = serde_json::Map::new();
        let mut display_allocation_class_counts = BTreeMap::new();
        for candidate in &display_allocation_candidates {
            *display_allocation_class_counts
                .entry(candidate.class_name)
                .or_insert(0usize) += 1;
            display_allocation_reasons.insert(candidate.id.clone(), candidate.to_json());
        }
        let skipped_abandoned_sessions_missing_age_timestamp_count =
            skipped_abandoned_sessions_missing_age_timestamp.len();
        let skipped_abandoned_sessions_too_fresh_count = skipped_abandoned_sessions_too_fresh.len();
        let skipped_abandoned_sessions_missing_age_timestamp_summary =
            summarize_skipped_session_groups(&skipped_abandoned_sessions_missing_age_timestamp);
        let skipped_abandoned_sessions_too_fresh_summary =
            summarize_skipped_session_groups(&skipped_abandoned_sessions_too_fresh);
        let mut session_tab_refs_removed = 0usize;
        let mut session_browser_refs_removed = 0usize;
        if options.apply {
            for session_id in &session_ids {
                state.sessions.remove(session_id);
            }
            for tab_id in &closed_tab_ids {
                state.tabs.remove(tab_id);
            }
            for browser_id in &browser_ids {
                state.browsers.remove(browser_id);
            }
            for profile_id in &profile_ids {
                state.profiles.remove(profile_id);
                state.entity_sources.profiles.remove(profile_id);
            }
            for display_allocation_id in &display_allocation_ids {
                state.display_allocations.remove(display_allocation_id);
            }
            for session in state.sessions.values_mut() {
                let before = session.tab_ids.len();
                session
                    .tab_ids
                    .retain(|tab_id| !closed_tab_ids.contains(tab_id));
                session_tab_refs_removed += before.saturating_sub(session.tab_ids.len());
                let before = session.browser_ids.len();
                session
                    .browser_ids
                    .retain(|browser_id| !browser_ids.contains(browser_id));
                session_browser_refs_removed += before.saturating_sub(session.browser_ids.len());
            }
            for browser in state.browsers.values_mut() {
                let before = browser.active_session_ids.len();
                browser
                    .active_session_ids
                    .retain(|session_id| !pruned_session_ids.contains(session_id));
                session_browser_refs_removed +=
                    before.saturating_sub(browser.active_session_ids.len());
            }
            state.refresh_derived_views();
        }
        let removed_display_allocation_count = if options.apply {
            display_allocation_ids.len()
        } else {
            0
        };
        json!(
            { "pruned" : options.apply, "dryRun" : ! options.apply, "policy" : {
            "closedTabs" : options.closed_tabs, "notStartedBrowsers" : options
            .not_started_browsers, "processExitedBrowsers" : options
            .process_exited_browsers, "processExitedBrowsersIncludesUnreachable" : true,
            "processExitedBrowsersIncludesFaultedPlaceholders" : true,
            "releasedSessionPruneRemovesRetainedViewStreams" : true, "releasedSessions" :
            options.released_sessions, "abandonedSessions" : options.abandoned_sessions,
            "orphanedProfiles" : options.orphaned_profiles, "displayAllocations" :
            options.display_allocations, "abandonedSessionMinAgeMinutes" : options
            .abandoned_session_min_age_minutes, "processExitedRequiresExplicitFlag" :
            true, "abandonedSessionsRequiresExplicitFlag" : true,
            "abandonedSessionsRequireAgeTimestamp" : true, "abandonedSessionAgeSource" :
            "lastLeaseObservedAtOrCreatedAt", "orphanedProfilesRequiresExplicitFlag" :
            true, "orphanedProfilesScope" :
            "customProfilesWithMissingEphemeralUserDataDirOrManagedOneTimeWithoutRetainedReferences",
            "displayAllocationsRequiresExplicitFlag" : true,
            "displayAllocationsApplyRequiresApplySafeClassification" : true, }, "before"
            : { "profileCount" : before_profile_count, "browserCount" :
            before_browser_count, "tabCount" : before_tab_count, "sessionCount" :
            before_session_count, "displayAllocationCount" :
            before_display_allocation_count, }, "candidates" : { "closedTabs" :
            closed_tab_ids, "browsers" : browser_ids, "sessions" : session_ids,
            "orphanedProfiles" : profile_ids, "displayAllocations" :
            display_allocation_ids, }, "candidateReasons" : { "orphanedProfiles" :
            orphaned_profile_reasons, "displayAllocations" : display_allocation_reasons,
            }, "candidateClassCounts" : { "displayAllocations" :
            display_allocation_class_counts, }, "candidateCounts" : { "closedTabs" :
            closed_tab_ids.len(), "browsers" : browser_ids.len(), "sessions" :
            session_ids.len(), "orphanedProfiles" : profile_ids.len(),
            "displayAllocations" : display_allocation_ids.len(), "total" : closed_tab_ids
            .len() + browser_ids.len() + session_ids.len() + profile_ids.len() +
            display_allocation_ids.len(), }, "skipped" : {
            "abandonedSessionsMissingAgeTimestamp" :
            skipped_abandoned_sessions_missing_age_timestamp, "abandonedSessionsTooFresh"
            : skipped_abandoned_sessions_too_fresh, }, "skippedCounts" : {
            "abandonedSessionsMissingAgeTimestamp" :
            skipped_abandoned_sessions_missing_age_timestamp_count,
            "abandonedSessionsTooFresh" : skipped_abandoned_sessions_too_fresh_count, },
            "skippedSummary" : { "abandonedSessionsMissingAgeTimestamp" :
            skipped_abandoned_sessions_missing_age_timestamp_summary,
            "abandonedSessionsTooFresh" : skipped_abandoned_sessions_too_fresh_summary,
            }, "removed" : { "closedTabs" : if options.apply { closed_tab_ids.len() }
            else { 0 }, "browsers" : if options.apply { browser_ids.len() } else { 0 },
            "sessions" : if options.apply { session_ids.len() } else { 0 },
            "orphanedProfiles" : if options.apply { profile_ids.len() } else { 0 },
            "displayAllocations" : removed_display_allocation_count, "sessionTabRefs" :
            session_tab_refs_removed, "sessionBrowserRefs" :
            session_browser_refs_removed, }, "after" : { "profileCount" : state.profiles
            .len(), "browserCount" : state.browsers.len(), "tabCount" : state.tabs.len(),
            "sessionCount" : state.sessions.len(), "displayAllocationCount" : state
            .display_allocations.len(), }, "recommendedNextStep" : if options.apply {
            "Run agent-browser service reconcile and inspect agent-browser service status."
            } else {
            "Review the candidates, then rerun with --apply when the retained records are safe to remove."
            }, }
        )
    }
    pub(crate) fn referenced_service_profile_ids(state: &ServiceState) -> HashSet<String> {
        let mut profile_ids = HashSet::new();
        for browser in state.browsers.values() {
            if let Some(profile_id) = browser.profile_id.as_deref().filter(|id| !id.is_empty()) {
                profile_ids.insert(profile_id.to_string());
            }
        }
        for session in state.sessions.values() {
            if let Some(profile_id) = session.profile_id.as_deref().filter(|id| !id.is_empty()) {
                profile_ids.insert(profile_id.to_string());
            }
            if let Some(value) = session.browser_capability_launch.as_ref() {
                collect_profile_ids_from_json(value, &mut profile_ids);
            }
        }
        for allocation in state.display_allocations.values() {
            if let Some(profile_id) = allocation.profile_id.as_deref().filter(|id| !id.is_empty()) {
                profile_ids.insert(profile_id.to_string());
            }
            if let Some(value) = allocation.readiness.as_ref() {
                collect_profile_ids_from_json(value, &mut profile_ids);
            }
        }
        for event in &state.events {
            if let Some(profile_id) = event.profile_id.as_deref().filter(|id| !id.is_empty()) {
                profile_ids.insert(profile_id.to_string());
            }
            if let Some(value) = event.details.as_ref() {
                collect_profile_ids_from_json(value, &mut profile_ids);
            }
        }
        for job in state.jobs.values() {
            if let Some(value) = job.result.as_ref() {
                collect_profile_ids_from_json(value, &mut profile_ids);
            }
        }
        for handoff in state.profile_seeding_handoffs.values() {
            if !handoff.profile_id.is_empty() {
                profile_ids.insert(handoff.profile_id.clone());
            }
        }
        profile_ids
    }
    pub(crate) fn collect_profile_ids_from_json(value: &Value, profile_ids: &mut HashSet<String>) {
        match value {
            Value::Object(map) => {
                for key in ["profileId", "profile_id"] {
                    if let Some(profile_id) = map.get(key).and_then(Value::as_str) {
                        if !profile_id.is_empty() {
                            profile_ids.insert(profile_id.to_string());
                        }
                    }
                }
                for value in map.values() {
                    collect_profile_ids_from_json(value, profile_ids);
                }
            }
            Value::Array(values) => {
                for value in values {
                    collect_profile_ids_from_json(value, profile_ids);
                }
            }
            _ => {}
        }
    }
    pub(crate) fn orphaned_profile_prune_reason(
        profile_id: &str,
        profile: &BrowserProfile,
        referenced_profile_ids: &HashSet<String>,
    ) -> Option<&'static str> {
        if matches!(
            profile.profile_origin,
            ProfileOrigin::ExternalByop | ProfileOrigin::ExternalObserved
        ) {
            return None;
        }
        if referenced_profile_ids.contains(profile_id) {
            return None;
        }
        if profile.profile_class == ProfileClass::ManagedOneTime && !profile.persistent {
            return Some("managed_one_time_unreferenced");
        }
        if !profile_id.starts_with("custom:") {
            return None;
        }
        if !profile.site_policy_ids.is_empty()
            || !profile.target_service_ids.is_empty()
            || !profile.authenticated_service_ids.is_empty()
            || !profile.account_ids.is_empty()
            || !profile.shared_service_ids.is_empty()
            || !profile.credential_provider_ids.is_empty()
            || !profile.target_readiness.is_empty()
        {
            return None;
        }
        let user_data_dir = profile.user_data_dir.as_deref()?;
        let path = Path::new(user_data_dir);
        if !is_ephemeral_profile_path(path) || path.exists() {
            return None;
        }
        Some("missing_ephemeral_user_data_dir")
    }
    pub(crate) fn is_ephemeral_profile_path(path: &Path) -> bool {
        if path.starts_with("/tmp") || path.starts_with("/var/tmp") {
            return true;
        }
        let path_text = path.to_string_lossy();
        path_text.contains("/AppData/Local/Temp/")
            || path_text.contains("\\AppData\\Local\\Temp\\")
            || (path_text.contains("/workspace.local/") && path_text.contains("/tmp/"))
            || (path_text.contains("/.local/state/")
                && (path_text.contains("/browser-smokes/") || path_text.contains("/ui-audits/")))
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum SessionAgeStatus {
        OldEnough,
        TooFresh,
        MissingOrInvalid,
    }
    pub(crate) fn abandoned_session_age_status(
        created_at: Option<&str>,
        min_age_minutes: u64,
    ) -> SessionAgeStatus {
        let Some(created_at) = created_at else {
            return SessionAgeStatus::MissingOrInvalid;
        };
        let Ok(created_at) = DateTime::parse_from_rfc3339(created_at) else {
            return SessionAgeStatus::MissingOrInvalid;
        };
        let Ok(min_age_minutes) = i64::try_from(min_age_minutes) else {
            return SessionAgeStatus::MissingOrInvalid;
        };
        let threshold = chrono::Utc::now() - chrono::Duration::minutes(min_age_minutes);
        if created_at.with_timezone(&chrono::Utc) <= threshold {
            SessionAgeStatus::OldEnough
        } else {
            SessionAgeStatus::TooFresh
        }
    }
    pub(crate) fn summarize_skipped_session_groups(session_ids: &[String]) -> Value {
        let mut groups = BTreeMap::<String, Vec<String>>::new();
        for session_id in session_ids {
            groups
                .entry(skipped_session_group(session_id))
                .or_default()
                .push(session_id.clone());
        }
        let group_count = groups.len();
        let mut groups = groups
            .into_iter()
            .map(|(group, mut ids)| {
                ids.sort();
                json!(
                    { "group" : group, "count" : ids.len(), "examples" : ids.into_iter()
                    .take(3).collect::< Vec < _ >> (), }
                )
            })
            .collect::<Vec<_>>();
        groups.sort_by(|left, right| {
            let left_count = left
                .get("count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let right_count = right
                .get("count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            right_count.cmp(&left_count).then_with(|| {
                left.get("group")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .cmp(
                        right
                            .get("group")
                            .and_then(|value| value.as_str())
                            .unwrap_or(""),
                    )
            })
        });
        let omitted_group_count = group_count.saturating_sub(10);
        groups.truncate(10);
        json!(
            { "groupCount" : group_count, "omittedGroupCount" : omitted_group_count,
            "groups" : groups, }
        )
    }
    pub(crate) fn skipped_session_group(session_id: &str) -> String {
        let Some((prefix, suffix)) = session_id.rsplit_once('-') else {
            return session_id.to_string();
        };
        if !prefix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
            prefix.to_string()
        } else {
            session_id.to_string()
        }
    }
    pub(crate) fn session_has_parseable_age_evidence(session: &BrowserSession) -> bool {
        session
            .last_lease_observed_at
            .as_deref()
            .or(session.created_at.as_deref())
            .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
            .is_some()
    }
    pub(crate) fn legacy_inert_session_placeholder(
        state: &ServiceState,
        session: &BrowserSession,
    ) -> bool {
        session.tab_ids.is_empty()
            && !session.browser_ids.is_empty()
            && session.browser_ids.iter().all(|browser_id| {
                inert_session_browser_placeholder(state, browser_id, session.id.as_str())
            })
    }
    pub(crate) fn prunable_session_browser_placeholder(
        state: &ServiceState,
        browser_id: &str,
        session_id: &str,
        allow_failed_retained: bool,
    ) -> bool {
        inert_session_browser_placeholder(state, browser_id, session_id)
            || (allow_failed_retained
                && failed_retained_session_browser_placeholder(state, browser_id, session_id))
    }
    pub(crate) fn inert_session_browser_placeholder(
        state: &ServiceState,
        browser_id: &str,
        session_id: &str,
    ) -> bool {
        let Some(browser) = state.browsers.get(browser_id) else {
            return false;
        };
        retained_not_started_browser_placeholder(state, browser_id, browser)
            && (browser.active_session_ids.is_empty()
                || browser.active_session_ids == vec![session_id.to_string()])
    }
    pub(crate) fn failed_retained_session_browser_placeholder(
        state: &ServiceState,
        browser_id: &str,
        session_id: &str,
    ) -> bool {
        let Some(browser) = state.browsers.get(browser_id) else {
            return false;
        };
        retained_failed_browser_placeholder(state, browser_id, browser)
            && (browser.active_session_ids.is_empty()
                || browser.active_session_ids == vec![session_id.to_string()])
    }
    pub(crate) fn failed_retained_browser_health(health: ServiceBrowserHealth) -> bool {
        matches!(
            health,
            ServiceBrowserHealth::ProcessExited
                | ServiceBrowserHealth::Unreachable
                | ServiceBrowserHealth::Faulted
        )
    }
    pub(crate) fn retained_not_started_browser_placeholder(
        state: &ServiceState,
        browser_id: &str,
        browser: &BrowserProcess,
    ) -> bool {
        browser.health == ServiceBrowserHealth::NotStarted
            && browser.pid.is_none()
            && browser.cdp_endpoint.is_none()
            && !browser_has_live_tabs(state, browser_id)
    }
    pub(crate) fn retained_failed_browser_placeholder(
        state: &ServiceState,
        browser_id: &str,
        browser: &BrowserProcess,
    ) -> bool {
        failed_retained_browser_health(browser.health)
            && (matches!(
                browser.health,
                ServiceBrowserHealth::ProcessExited | ServiceBrowserHealth::Unreachable
            ) || (browser.pid.is_none() && browser.cdp_endpoint.is_none()))
            && !browser_has_live_tabs(state, browser_id)
    }
    pub(crate) fn browser_has_live_tabs(state: &ServiceState, browser_id: &str) -> bool {
        state
            .tabs
            .values()
            .any(|tab| tab.browser_id == browser_id && tab.lifecycle != TabLifecycle::Closed)
    }
}
pub(crate) use service_commands::*;
