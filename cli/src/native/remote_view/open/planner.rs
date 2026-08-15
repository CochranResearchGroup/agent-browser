#![allow(unused_imports)]
use super::runtime::RouteBoundBrowserObservation;
use super::shared::*;
pub(crate) fn ensure_remote_view_route_available_for_display(
    state: &ServiceState,
    route_id: &str,
    display_allocation_id: &str,
    browser_id: &str,
    allocation: Option<&DisplayAllocation>,
) -> Result<(), String> {
    let Some(route) = state.remote_view_routes.get(route_id) else {
        return Ok(());
    };
    if route.state == "released"
        || route.display_allocation_id.as_deref() == Some(display_allocation_id)
        || route.browser_id.as_deref() == Some(browser_id)
    {
        return Ok(());
    }
    let current_allocation_is_private = route
        .display_allocation_id
        .as_ref()
        .and_then(|id| state.display_allocations.get(id))
        .is_some_and(|allocation| allocation.display_isolation == "private_virtual_display");
    let requested_allocation_is_private = allocation
        .is_some_and(|allocation| allocation.display_isolation == "private_virtual_display");
    if current_allocation_is_private || requested_allocation_is_private {
        return Err(
            format!(
                "route_pool_contention: remote view route '{}' is already checked out to another private display allocation",
                route_id
            ),
        );
    }
    Ok(())
}
pub(crate) fn remote_view_lease_is_active(lease: &ViewerLease) -> bool {
    !matches!(
        lease.state.as_str(),
        "disconnected" | "expired" | "failed" | "released"
    )
}
pub(crate) fn push_remote_view_service_event(
    state: &mut ServiceState,
    kind: ServiceEventKind,
    timestamp: &str,
    browser_id: Option<String>,
    session_id: Option<String>,
    message: String,
    details: Value,
) -> String {
    let event_id = format!(
        "remote-view-event:{}:{}",
        service_event_kind_name(kind),
        timestamp.replace([':', '.'], "-")
    );
    state.events.push(ServiceEvent {
        id: event_id.clone(),
        timestamp: timestamp.to_string(),
        kind,
        message,
        browser_id,
        session_id,
        details: Some(details),
        ..ServiceEvent::default()
    });
    if state.events.len() > 100 {
        let excess = state.events.len() - 100;
        state.events.drain(0..excess);
    }
    event_id
}
pub(crate) fn service_remote_view_acquisition_plan_from_state(
    _cmd: &Value,
    state: &ServiceState,
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
    inline_route_pool_entry: Option<&RoutePoolEntry>,
    browser_id: &str,
    session_id: &str,
) -> Result<RemoteViewAcquisitionPlan, String> {
    plan_remote_view_acquisition(
        state,
        intent,
        inline_route_pool_entry,
        browser_id,
        session_id,
    )
}
pub(crate) fn remote_view_open_should_reuse_current_browser(
    acquisition_plan: &RemoteViewAcquisitionPlan,
    observation: &RouteBoundBrowserObservation,
    browser_id: &str,
    session_id: &str,
) -> bool {
    if browser_id != observation.browser_id || session_id != observation.session_id {
        return false;
    }
    if !observation.browser_present {
        return false;
    }
    acquisition_plan.decisions.iter().any(|decision| {
        decision.step == "route_pool_entry" && decision.reason == "same_owner_checked_out_route"
    })
}

/// Durable handoff resolution is browser-first: an exact live daemon/profile
/// owner may acquire a new route without launching a second browser process.
/// Route ownership is deliberately not required here because restoring that
/// missing route binding is the purpose of durable resolution.
pub(crate) fn remote_view_open_should_reuse_current_browser_for_durable_resolution(
    observation: &RouteBoundBrowserObservation,
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
    browser_id: &str,
    session_id: &str,
    service_state: &ServiceState,
) -> bool {
    if !observation.browser_present
        || browser_id != observation.browser_id
        || session_id != observation.session_id
    {
        return false;
    }

    let requested_profile = intent
        .runtime_profile
        .as_deref()
        .or(intent.profile.as_deref());
    if requested_profile.is_none() || observation.runtime_profile.as_deref() == requested_profile {
        return true;
    }

    let Some(browser) = service_state.browsers.get(browser_id) else {
        return false;
    };
    browser.pid == observation.browser_pid
        && browser.profile_id.as_deref() == requested_profile
        && browser
            .active_session_ids
            .iter()
            .any(|active_session_id| active_session_id == session_id)
        && !matches!(
            browser.health,
            ServiceBrowserHealth::NotStarted
                | ServiceBrowserHealth::ProcessExited
                | ServiceBrowserHealth::Closing
                | ServiceBrowserHealth::Faulted
        )
}

pub(crate) fn remote_view_open_runtime_attach_launch_command(
    launch_command: &Value,
    observation: &RouteBoundBrowserObservation,
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
) -> Value {
    if observation.browser_present {
        return launch_command.clone();
    }
    let Some(target) = managed_runtime_attach_target(intent.runtime_profile.as_deref()) else {
        return launch_command.clone();
    };
    let mut command = launch_command.clone();
    if let Some(object) = command.as_object_mut() {
        object.insert("cdpPort".to_string(), json!(target.cdp_port));
        object.insert("runtimeAttachManaged".to_string(), Value::Bool(true));
    }
    command
}
pub(crate) fn inline_route_pool_entry_from_command(
    cmd: &Value,
) -> Result<Option<RoutePoolEntry>, String> {
    if let Some(entry) = command_or_params_value(cmd, "routePoolEntry") {
        return serde_json::from_value::<RoutePoolEntry>(entry.clone())
            .map(normalize_inline_route_pool_entry)
            .map(Some)
            .map_err(|err| format!("invalid routePoolEntry: {}", err));
    }
    if let Some(entries) = command_or_params_value(cmd, "routePool").and_then(Value::as_array) {
        let route_pool_entry_id = optional_command_or_params_string(cmd, "routePoolEntryId")
            .or_else(|| optional_command_or_params_string(cmd, "poolEntryId"));
        let requested_route_id = optional_command_or_params_string(cmd, "remoteViewRouteId")
            .or_else(|| optional_command_or_params_string(cmd, "routeId"))
            .or_else(|| optional_command_or_params_string(cmd, "viewStreamRouteId"));
        for entry in entries {
            let parsed = serde_json::from_value::<RoutePoolEntry>(entry.clone())
                .map(normalize_inline_route_pool_entry)
                .map_err(|err| format!("invalid routePool entry: {}", err))?;
            if route_pool_entry_id.as_deref() == Some(parsed.id.as_str())
                || requested_route_id.as_deref() == Some(parsed.route_id.as_str())
                || (route_pool_entry_id.is_none() && requested_route_id.is_none())
            {
                return Ok(Some(parsed));
            }
        }
    }
    Ok(None)
}
pub(crate) fn inline_route_pool_entries_from_command(
    cmd: &Value,
) -> Result<Vec<RoutePoolEntry>, String> {
    let mut parsed_entries = Vec::new();
    if let Some(entry) = command_or_params_value(cmd, "routePoolEntry") {
        parsed_entries.push(
            serde_json::from_value::<RoutePoolEntry>(entry.clone())
                .map(normalize_inline_route_pool_entry)
                .map_err(|err| format!("invalid routePoolEntry: {}", err))?,
        );
    }
    if let Some(entries) = command_or_params_value(cmd, "routePool").and_then(Value::as_array) {
        for entry in entries {
            parsed_entries.push(
                serde_json::from_value::<RoutePoolEntry>(entry.clone())
                    .map(normalize_inline_route_pool_entry)
                    .map_err(|err| format!("invalid routePool entry: {}", err))?,
            );
        }
    }
    let mut deduped = BTreeMap::new();
    for entry in parsed_entries {
        deduped.insert(entry.id.clone(), entry);
    }
    Ok(deduped.into_values().collect())
}
pub(crate) fn normalize_inline_route_pool_entry(mut entry: RoutePoolEntry) -> RoutePoolEntry {
    if matches!(entry.state.trim(), "" | "unknown")
        && entry.readiness.as_ref().is_some_and(|readiness| {
            readiness
                .get("state")
                .and_then(Value::as_str)
                .is_some_and(|state| state.trim() == "ready")
                || readiness_state(readiness).as_deref() == Some("ready")
        })
    {
        entry.state = "available".to_string();
    }
    entry
}
pub(crate) fn remote_view_open_persist_request_route_pool(
    repository: &LockedServiceStateRepository<
        super::super::super::service_store::JsonServiceStateStore,
    >,
    entries: &[RoutePoolEntry],
) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    repository.mutate(|state| {
        for entry in entries {
            let mut next = entry.clone();
            if let Some(existing) = state.route_pool.get(&entry.id) {
                let existing_active = existing.current_route_allocation_id.is_some()
                    && !matches!(existing.state.as_str(), "" | "available" | "released");
                let incoming_inactive = entry.current_route_allocation_id.is_none()
                    && matches!(entry.state.as_str(), "" | "available" | "released");
                if existing_active && incoming_inactive {
                    next.state = existing.state.clone();
                    next.current_route_allocation_id = existing.current_route_allocation_id.clone();
                    next.readiness = existing.readiness.clone();
                }
            }
            state.route_pool.insert(entry.id.clone(), next);
        }
        Ok(())
    })
}
pub(crate) fn remote_view_open_ensure_managed_one_time_profile(
    repository: &LockedServiceStateRepository<
        super::super::super::service_store::JsonServiceStateStore,
    >,
    service_state: &mut ServiceState,
    intent: &mut super::super::super::remote_view::RemoteViewOpenIntent,
    dry_run: bool,
) -> Result<Value, String> {
    if intent.runtime_profile.is_some() || intent.profile.is_some() {
        return Ok(Value::Null);
    }
    if !remote_view_open_looks_like_one_time_operator_handoff(intent) {
        return Ok(Value::Null);
    }
    let profile_id = remote_view_open_managed_one_time_profile_id(intent);
    intent.runtime_profile = Some(profile_id.clone());
    if let Some(profile) = service_state.profiles.get(&profile_id) {
        return Ok(json!(
            { "state" : "reused", "profileId" : profile_id, "runtimeProfile" :
            profile_id, "profileClass" : profile.profile_class, "profileOrigin" :
            profile.profile_origin, "userDataDir" : profile.user_data_dir, "dryRun" :
            dry_run, }
        ));
    }
    let profile = remote_view_open_managed_one_time_profile(intent, &profile_id);
    service_state
        .entity_sources
        .profiles
        .insert(profile_id.clone(), ServiceEntitySource::PersistedState);
    service_state
        .profiles
        .insert(profile_id.clone(), profile.clone());
    if !dry_run {
        repository.mutate(|state| {
            state
                .entity_sources
                .profiles
                .insert(profile_id.clone(), ServiceEntitySource::PersistedState);
            state.profiles.insert(profile_id.clone(), profile.clone());
            Ok(())
        })?;
    }
    Ok(json!(
        { "state" : if dry_run { "planned" } else { "created" }, "profileId" :
        profile_id, "runtimeProfile" : profile_id, "profileClass" :
        ProfileClass::ManagedOneTime, "profileOrigin" :
        ProfileOrigin::AgentBrowserOwned, "userDataDir" : profile.user_data_dir,
        "persistent" : profile.persistent, "dryRun" : dry_run, }
    ))
}
pub(crate) fn remote_view_open_managed_one_time_profile(
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
    profile_id: &str,
) -> BrowserProfile {
    let service_ids = intent
        .service_name
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| vec![value.clone()])
        .unwrap_or_default();
    let browser_build = intent
        .browser_build
        .as_deref()
        .and_then(BrowserBuild::parse_label);
    let user_data_dir = runtime_profile_user_data_dir(profile_id)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let task_label = intent
        .task_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("operator handoff");
    BrowserProfile {
        id: profile_id.to_string(),
        name: format!("Managed one-time {task_label}"),
        profile_origin: ProfileOrigin::AgentBrowserOwned,
        profile_class: ProfileClass::ManagedOneTime,
        user_data_dir,
        default_browser_host: Some(ServiceBrowserHost::RemoteHeaded),
        browser_build,
        allocation: ProfileAllocationPolicy::PerService,
        keyring: ProfileKeyringPolicy::BasicPasswordStore,
        shared_service_ids: service_ids,
        manual_login_preferred: true,
        persistent: false,
        tags: vec!["managed_one_time".to_string()],
        ..BrowserProfile::default()
    }
}
pub(crate) fn remote_view_open_command_with_effective_intent(
    cmd: &Value,
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
) -> Value {
    let mut command = cmd.clone();
    if !command.is_object() {
        command = json!({});
    }
    if let Some(map) = command.as_object_mut() {
        if let Some(runtime_profile) = intent.runtime_profile.as_deref() {
            map.insert(
                "runtimeProfile".to_string(),
                Value::String(runtime_profile.to_string()),
            );
        }
        if let Some(profile) = intent.profile.as_deref() {
            map.insert("profile".to_string(), Value::String(profile.to_string()));
        }
    }
    command
}
pub(crate) fn remote_view_open_one_time_profile_warning(
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
    service_state: &ServiceState,
) -> Value {
    let Some(runtime_profile) = intent.runtime_profile.as_deref() else {
        return Value::Null;
    };
    if service_state.profiles.contains_key(runtime_profile) {
        return Value::Null;
    }
    if !remote_view_open_looks_like_one_time_operator_handoff(intent) {
        return Value::Null;
    }
    let recommended_profile_id = remote_view_open_managed_one_time_profile_id(intent);
    json!(
        { "state" : "warning", "code" : "arbitrary_runtime_profile_for_one_time_handoff",
        "requestedRuntimeProfile" : runtime_profile, "profileClass" :
        "operator_supplied", "recommendedProfileClass" : "managed_one_time",
        "recommendedProfileId" : recommended_profile_id, "message" :
        "This looks like a one-time operator handoff but it supplied a new arbitrary runtime profile. Prefer the managed one-time task profile so retries reuse one lane and cleanup can remove abandoned task state safely.",
        }
    )
}
pub(crate) fn remote_view_open_looks_like_one_time_operator_handoff(
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
) -> bool {
    if intent.view_stream_provider != ViewStreamProvider::RdpGateway {
        return false;
    }
    let manual_control = intent.control_input == "manual_attached_desktop";
    let remote_headed = intent.browser_host == "remote_headed";
    let text = [
        intent.service_name.as_deref(),
        intent.agent_name.as_deref(),
        intent.task_name.as_deref(),
        intent.url.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    let one_time_hint = [
        "temporary",
        "temp",
        "one-time",
        "one_time",
        "login",
        "payment",
        "challenge",
        "sosdirect",
        "templogin",
    ]
    .iter()
    .any(|needle| text.contains(needle));
    manual_control && remote_headed && one_time_hint
}
pub(crate) fn remote_view_open_managed_one_time_profile_id(
    intent: &super::super::super::remote_view::RemoteViewOpenIntent,
) -> String {
    let seed = [
        intent.service_name.as_deref().unwrap_or("service"),
        intent.agent_name.as_deref().unwrap_or("agent"),
        intent.task_name.as_deref().unwrap_or("task"),
        intent.url.as_deref().unwrap_or("url"),
    ]
    .join("|")
    .to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("managed-one-time-{suffix}")
}
